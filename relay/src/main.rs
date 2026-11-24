#![forbid(unsafe_code)]
use re_with::waveform::{Account, WaveformConfig};
use re_with::{Config, app};
use std::time::Duration;

/// From the environment; Err names what is missing. The password is a systemd credential file, never a variable.
fn waveforms() -> Result<WaveformConfig, String> {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty()).ok_or(format!("no {k}"));
    let dir = var("WAVEFORM_DIR")?.into();
    let user = var("NAVIDROME_USER")?;
    let file = std::path::Path::new(&var("CREDENTIALS_DIRECTORY")?).join("navidrome-password");
    let password = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let password = password.trim_end_matches(['\r', '\n']).to_string();
    let ffmpeg = var("FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    Ok(WaveformConfig { dir, account: Account { user, password }, ffmpeg, poll: Duration::from_secs(60) })
}

fn main() -> std::process::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("status") {
        let path = std::env::var_os("STATUS_SOCKET").unwrap_or_else(|| "/run/re-with/status.sock".into());
        return re_with::status::client(std::path::Path::new(&path));
    }
    serve();
    std::process::ExitCode::SUCCESS
}

#[tokio::main]
async fn serve() {
    let navidrome = std::env::var("NAVIDROME_URL").unwrap_or_else(|_| "http://127.0.0.1:4533".into());
    let listen = std::env::var("LISTEN").unwrap_or_else(|_| "127.0.0.1:4534".into());
    let listener = tokio::net::TcpListener::bind(&listen).await.unwrap_or_else(|e| panic!("bind {listen}: {e}"));
    eprintln!("re-with listening on {listen}, navidrome at {navidrome}");
    let waveforms = waveforms().inspect_err(|e| eprintln!("waveforms off: {e}")).ok();
    // systemd's RuntimeDirectory=re-with: /run/re-with
    let status_socket = std::env::var_os("RUNTIME_DIRECTORY").map(|d| std::path::PathBuf::from(d).join("status.sock"));
    axum::serve(listener, app(Config { navidrome, waveforms, status_socket })).await.expect("server");
}
