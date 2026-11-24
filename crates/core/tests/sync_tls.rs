//! wss:// to a relay whose certificate comes from a CA in the system store (a homelab CA), like Qt trusted before.
//! rustls-native-certs reads SSL_CERT_FILE; the test runs itself again as a child process with it set, so no test in
//! this process changes the environment.
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
fn trusts_a_ca_from_the_system_store() {
    let dir = std::env::temp_dir().join(format!("re-la-tls-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ok = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "trusts_a_ca_child", "--nocapture"])
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

/// The test proper, run by the one above with SSL_CERT_FILE naming where to put the CA.
#[test]
#[ignore]
fn trusts_a_ca_child() {
    use futures_util::{SinkExt, StreamExt};
    let Some(ca_file) = std::env::var_os(CHILD) else {
        return; // not started by trusts_a_ca_from_the_system_store
    };
    let ca = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    std::fs::write(&ca_file, ca.cert.pem()).unwrap();

    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let port = rt.block_on(async {
        let key = tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(ca.signing_key.serialize_der()).unwrap();
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![ca.cert.der().clone()], key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
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
