# re-with

A small server next to Navidrome. It lets re-la users listen together (WebSocket), and it works out every song's
waveform once, so the apps draw the seek bar's waveform without decoding anything.

- It does not change Navidrome.
- Listening together has no accounts of its own: it checks each user with a Navidrome `ping`. Tokens stay in memory
  only for that check and are never stored or logged. Its state is RAM only.
- Waveforms (optional, below) are files on disk, one per song.

## Waveforms

re-with follows Navidrome's library scans: after each scan (and at start) it downloads every song without a current
waveform from Navidrome, decodes it with ffmpeg at its own sample rate (one song at a time, `nice 19`) and keeps 4096
peaks and RMS values (about 8 KB per song: 50,000 songs take about 400 MB). Songs gone from the library lose theirs.
Apps fetch them from `<navidrome-url>/re-with/waveform/<song id>` with their Subsonic login in headers; each request
asks Navidrome whether that user can see the song.

To turn it on:

1. Install ffmpeg (`apt install ffmpeg`; Navidrome usually has it already).
2. In Navidrome, make a user for re-with (not an admin) that can see every library. Downloads must be allowed
   (`EnableDownloads`, on by default).
3. Put its password in a file only root can read:
   ```bash
   sudo install -d -m 700 /etc/re-with
   sudo install -m 600 /dev/stdin /etc/re-with/navidrome-password   # type the password, then Ctrl-D
   ```
4. The unit in `deploy/` reads it as a systemd credential and keeps the waveforms in `/var/lib/re-with/waveforms`.
   Without the folder, user or password re-with runs without waveforms and logs `waveforms off: <what is missing>`.

The first pass over a big library takes hours; listening together works meanwhile.

## Build

Requires Rust 1.88+ and a C compiler; the tests need ffmpeg. The relay is part of the Cargo workspace at the repo root, so build it from
there (it shares its message types with the app through `crates/proto`):

```bash
cargo build -p re-with --release
sudo install -m 755 target/release/re-with /usr/local/bin/
```

For an ARM64 server (Debian 13 or any glibc 2.39+), CI builds it: on GitHub, Actions → the latest `relay` run on `main` → Artifacts → `re-with-aarch64`.

## Install (systemd + Caddy)

```bash
sudo cp deploy/re-with.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now re-with
```

Add `deploy/Caddyfile.snippet` to your Navidrome site block in the Caddyfile, then restart Caddy. re-la connects to `<navidrome-url>/re-with` by default.

## Status

On the server, `sudo re-with status` shows what re-with is doing, redrawn every second (Ctrl-C quits): the waveform
pass (done / total, the song now, rate and time left, errors), Navidrome's scan, and how many people are listening
together (counts only). It reads the unix socket `/run/re-with/status.sock` (the unit's `RuntimeDirectory`), never
a TCP port, so nothing of it reaches the internet. Piped, it prints once. `STATUS_SOCKET` points it elsewhere.

## Settings

| Variable | Default | Meaning |
|---|---|---|
| `NAVIDROME_URL` | `http://127.0.0.1:4533` | Navidrome address used to check logins |
| `LISTEN` | `127.0.0.1:4534` | Address the relay listens on |
| `WAVEFORM_DIR` | (none) | Waveform folder; unset: no waveforms |
| `NAVIDROME_USER` | (none) | re-with's own Navidrome user, for waveforms |
| `FFMPEG` | `ffmpeg` | ffmpeg to decode with |
| `RUNTIME_DIRECTORY` | (set by systemd) | The status socket goes here; unset: no status |

The password of `NAVIDROME_USER` is the systemd credential `navidrome-password` (`$CREDENTIALS_DIRECTORY`), never a
variable.

## Test

```bash
cargo test -p re-with -p rela-proto
```
