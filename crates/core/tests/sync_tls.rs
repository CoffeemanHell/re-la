//! TLS to servers whose certificate comes from a CA in the system store (a homelab CA), like Qt trusted before: wss://
//! to a relay, and HTTP/2 to the music server.
//! On Linux the system's verifier reads SSL_CERT_FILE; the test runs itself again as a child process with it set, so no
//! test in this process changes the environment.
use rela_core::subsonic::{Login, Subsonic};
use rela_core::sync::{SyncClient, SyncEvent, SyncListener, SyncSnapshot};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Collect(Mutex<Sender<SyncEvent>>);
impl SyncListener for Collect {
    fn on_event(&self, event: SyncEvent, _: SyncSnapshot) {
        let _ = self.0.lock().unwrap().send(event);
    }
}

#[test]
#[cfg_attr(windows, ignore = "Windows judges by its own store, not by SSL_CERT_FILE")]
fn trusts_a_ca_from_the_system_store() {
    in_child("trusts_a_ca_child");
}

#[test]
#[cfg_attr(windows, ignore = "Windows judges by its own store, not by SSL_CERT_FILE")]
fn http2_survives_a_dead_connection() {
    in_child("http2_child");
}

fn in_child(test: &str) {
    let dir = std::env::temp_dir().join(format!("re-la-tls-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ok = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", test, "--nocapture"])
        .env("SSL_CERT_FILE", dir.join("ca.pem"))
        .env(CHILD, dir.join("ca.pem"))
        .status()
        .unwrap()
        .success();
    let _ = std::fs::remove_dir_all(dir);
    assert!(ok, "the child test failed (its output is above)");
}

/// `--include-ignored` runs the child test directly, in the developer's environment: a SSL_CERT_FILE of theirs stays
/// as it is.
#[test]
fn the_child_leaves_a_users_cert_file_alone() {
    let dir = std::env::temp_dir().join(format!("re-la-tls-keep-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mine = dir.join("bundle.pem");
    std::fs::write(&mine, "my CAs").unwrap();
    let ok = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "trusts_a_ca_child"])
        .env("SSL_CERT_FILE", &mine)
        .env_remove(CHILD)
        .status()
        .unwrap()
        .success();
    let kept = std::fs::read_to_string(&mine).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(kept, "my CAs");
    assert!(ok, "the child skips when the parent didn't start it");
}

/// Set by the parent test: where the child writes its CA (the same path as SSL_CERT_FILE).
const CHILD: &str = "RELA_TLS_CHILD_CA";

/// Writes a new CA for localhost to `ca_file`; a server's TLS with it, offering `alpn`.
fn tls_for(ca_file: &std::ffi::OsStr, alpn: &[&str]) -> tokio_rustls::TlsAcceptor {
    let ca = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    std::fs::write(ca_file, ca.cert.pem()).unwrap();
    let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(ca.signing_key.serialize_der()).unwrap();
    let mut config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![ca.cert.der().clone()], key)
        .unwrap();
    config.alpn_protocols = alpn.iter().map(|p| p.as_bytes().to_vec()).collect();
    tokio_rustls::TlsAcceptor::from(Arc::new(config))
}

/// The test proper, run by the one above with SSL_CERT_FILE naming where to put the CA.
#[test]
#[ignore]
fn trusts_a_ca_child() {
    use futures_util::{SinkExt, StreamExt};
    let Some(ca_file) = std::env::var_os(CHILD) else {
        return; // not started by trusts_a_ca_from_the_system_store
    };
    let acceptor = tls_for(&ca_file, &[]);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let port = rt.block_on(async {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (tcp, _) = l.accept().await.unwrap();
            let tls = acceptor.accept(tcp).await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tls).await.unwrap();
            let _hello = ws.next().await;
            let welcome = r#"{"type":"welcome","you":"a","now":0}"#;
            ws.send(tokio_tungstenite::tungstenite::Message::text(welcome)).await.unwrap();
            while ws.next().await.is_some() {}
        });
        port
    });

    let (tx, rx) = channel();
    let c = SyncClient::new(false, false, Arc::new(Collect(Mutex::new(tx))));
    c.start(format!("wss://localhost:{port}/"), "a".into(), "t".into(), "s".into());
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(SyncEvent::Connected) => break,
            Ok(_) => {}
            Err(_) => panic!("no TLS connection to a server signed by a CA from SSL_CERT_FILE"),
        }
    }
}

struct TlsListener(tokio::net::TcpListener, tokio_rustls::TlsAcceptor);
impl axum::serve::Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let (tcp, addr) = self.0.accept().await.unwrap();
        (self.1.accept(tcp).await.unwrap(), addr)
    }
    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.0.local_addr()
    }
}

/// All calls share one HTTP/2 connection. When it goes silent (another network, a proxy that forgot it), only the calls
/// on it may fail: the next ones get a new connection.
#[test]
#[ignore]
fn http2_child() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let Some(ca_file) = std::env::var_os(CHILD) else {
        return; // not started by http2_survives_a_dead_connection
    };
    let acceptor = tls_for(&ca_file, &["h2", "http/1.1"]);
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let versions: Arc<Mutex<Vec<axum::http::Version>>> = Default::default();
        let seen = versions.clone();
        let app = axum::Router::new().fallback(move |r: axum::extract::Request| {
            seen.lock().unwrap().push(r.version());
            async { r#"{"subsonic-response": {"status": "ok"}}"# }
        });
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = l.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(TlsListener(l, acceptor), app).await.unwrap() });

        // a proxy between the two that can stop passing on what is on a connection, without closing it
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        let silent = Arc::new(tokio::sync::Notify::new());
        let connections = Arc::new(AtomicUsize::new(0));
        let (s, n) = (silent.clone(), connections.clone());
        tokio::spawn(async move {
            loop {
                let (mut a, _) = l.accept().await.unwrap();
                n.fetch_add(1, Ordering::SeqCst);
                let s = s.clone();
                tokio::spawn(async move {
                    let mut b = tokio::net::TcpStream::connect(server).await.unwrap();
                    tokio::select! {
                        _ = tokio::io::copy_bidirectional(&mut a, &mut b) => {}
                        _ = s.notified() => std::future::pending().await,
                    }
                });
            }
        });

        let login = Login::from_password(&format!("https://localhost:{port}"), "a", "p").unwrap();
        let c = Subsonic::new(login);
        c.call("ping", &[]).await.unwrap();
        for r in futures_util::future::join_all((0..20).map(|_| c.call("ping", &[]))).await {
            r.unwrap();
        }
        assert!(versions.lock().unwrap().iter().all(|v| *v == axum::http::Version::HTTP_2), "{versions:?}");
        assert_eq!(connections.load(Ordering::SeqCst), 1);

        silent.notify_waiters();
        let start = std::time::Instant::now();
        assert!(c.call("ping", &[]).await.is_err());
        assert!(start.elapsed() < Duration::from_secs(25), "the call's own 30 s ended it: {:?}", start.elapsed());
        c.call("ping", &[]).await.expect("a new connection");
        assert_eq!(connections.load(Ordering::SeqCst), 2);
    });
}
