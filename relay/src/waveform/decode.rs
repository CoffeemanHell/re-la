//! A song's levels from ffmpeg: its own process (a broken file can't take re-with down), lowest priority, at the
//! file's own sample rate and channels (no `-ac`: ffmpeg's mono-to-stereo mix is 3 dB down; the accumulator takes
//! any number of channels, a peak is the loudest of them).
use super::{Accumulator, Levels};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};

fn spawn(ffmpeg: &str, file: &Path) -> std::io::Result<Child> {
    let run = |nice: bool| {
        let mut c = if nice {
            let mut c = Command::new("nice");
            c.args(["-n", "19", ffmpeg]);
            c
        } else {
            Command::new(ffmpeg)
        };
        c.args(["-nostdin", "-v", "error", "-i"])
            .arg(file)
            .args(["-map", "0:a:0", "-f", "f32le", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
    };
    match run(true) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => run(false), // no `nice` here
        r => r,
    }
}

async fn run(ffmpeg: &str, file: &Path) -> Result<Levels, String> {
    let mut child = spawn(ffmpeg, file).map_err(|e| format!("ffmpeg: {e}"))?;
    let mut err = child.stderr.take().expect("piped");
    // read alongside stdout: a chatty ffmpeg must not block on a full stderr pipe
    let errors = tokio::spawn(async move {
        let mut b = Vec::new();
        let mut chunk = [0u8; 4096];
        while let Ok(n) = err.read(&mut chunk).await {
            if n == 0 {
                break;
            }
            if b.len() < 4096 {
                b.extend_from_slice(&chunk[..n]);
            }
        }
        b
    });
    let mut out = child.stdout.take().expect("piped");
    let (mut acc, mut buf, mut have) = (Accumulator::new(), vec![0u8; 1 << 16], 0usize);
    let mut samples = Vec::with_capacity(buf.len() / 4);
    loop {
        let n = out.read(&mut buf[have..]).await.map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            break;
        }
        have += n;
        let whole = have / 4 * 4;
        samples.clear();
        samples.extend(buf[..whole].as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)));
        acc.push(&samples);
        buf.copy_within(whole..have, 0); // a sample split across reads waits for its other bytes
        have -= whole;
    }
    let status = child.wait().await.map_err(|e| format!("ffmpeg: {e}"))?;
    let errors = errors.await.unwrap_or_default();
    if !status.success() {
        let line = String::from_utf8_lossy(&errors).lines().next().unwrap_or_default().trim().to_string();
        return Err(if line.is_empty() { format!("ffmpeg {status}") } else { line });
    }
    acc.finish().ok_or_else(|| "no audio".to_string())
}

/// Err: a short reason for the log ("timeout", "no audio" or ffmpeg's first error line). Past `limit` the
/// ffmpeg process is killed.
pub async fn levels(ffmpeg: &str, file: &Path, limit: Duration) -> Result<Levels, String> {
    tokio::time::timeout(limit, run(ffmpeg, file)).await.unwrap_or_else(|_| Err("timeout".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rwf-decode-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A 3 s 440 Hz mono sine at half scale, 96 kHz, in the format the name's extension says.
    fn sine(d: &Path, name: &str) -> PathBuf {
        let out = d.join(name);
        let ok = Command::new("ffmpeg")
            .args(["-v", "error", "-f", "lavfi", "-i", "aevalsrc=0.5*sin(440*2*PI*t):s=96000:d=3"])
            .arg(&out)
            .status()
            .expect("ffmpeg needed for these tests")
            .success();
        assert!(ok);
        out
    }

    const LIMIT: Duration = Duration::from_secs(60);

    #[tokio::test]
    async fn decodes_flac() {
        let d = dir("flac");
        let l = levels("ffmpeg", &sine(&d, "x.flac"), LIMIT).await.unwrap();
        assert!(l.peaks[100..4000].iter().all(|&p| (125..=131).contains(&p)), "{:?}", &l.peaks[100..108]);
        let _ = std::fs::remove_dir_all(d);
    }

    #[tokio::test]
    async fn decodes_m4a_with_index_at_end() {
        let d = dir("m4a");
        let l = levels("ffmpeg", &sine(&d, "x.m4a"), LIMIT).await.unwrap();
        assert!(l.peaks[2000] > 100, "{}", l.peaks[2000]);
        let _ = std::fs::remove_dir_all(d);
    }

    #[tokio::test]
    async fn garbage_is_an_error() {
        let d = dir("garbage");
        let f = d.join("x.mp3");
        std::fs::write(&f, (0..1024u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect::<Vec<_>>()).unwrap();
        assert!(levels("ffmpeg", &f, LIMIT).await.is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    #[tokio::test]
    async fn timeout_kills() {
        let d = dir("timeout");
        let f = sine(&d, "x.flac");
        assert_eq!(levels("ffmpeg", &f, Duration::from_nanos(1)).await, Err("timeout".to_string()));
        let _ = std::fs::remove_dir_all(d);
    }
}
