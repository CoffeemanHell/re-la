//! Windows: media keys and the media flyout through SystemMediaTransportControls. They come from a MediaPlayer whose
//! own commands are off, so no window handle is needed. What shows and when is `mpris.rs`'s, as on Linux.
use crate::hub::player;
use crate::mpris::{Msg, Out, Status, new_shared, serve};
use std::path::PathBuf;
use tokio::sync::mpsc;
use windows::Foundation::TypedEventHandler;
use windows::Media::Playback::MediaPlayer;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls, SystemMediaTransportControlsButton as Button,
    SystemMediaTransportControlsButtonPressedEventArgs as Pressed,
};
use windows::Storage::StorageFile;
use windows::Storage::Streams::RandomAccessStreamReference;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
use windows::core::HSTRING;

pub(crate) async fn run(rx: mpsc::UnboundedReceiver<Msg>, dir: PathBuf) -> windows::core::Result<()> {
    // SAFETY: plain WinRT start-up for this thread, before any WinRT object is made on it
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    let shared = new_shared();
    let mp = MediaPlayer::new()?;
    mp.CommandManager()?.SetIsEnabled(false)?; // the controls are ours, not the (unused) MediaPlayer's
    let c = mp.SystemMediaTransportControls()?;
    let sh = shared.clone();
    c.ButtonPressed(&TypedEventHandler::<SystemMediaTransportControls, Pressed>::new(move |_, args| {
        let b = args.ok()?.Button()?;
        let s = sh.lock().unwrap().status();
        press(&s, b);
        Ok(())
    }))?;
    serve(rx, dir, shared, async |o| {
        if let Out::Changed(names, s) = o
            && let Err(e) = show(&c, &names, &s)
        {
            eprintln!("smtc: {e}");
        }
    })
    .await;
    Ok(())
}

/// The keys as MPRIS's Play, Pause, Stop, Next and Previous.
fn press(s: &Status, b: Button) {
    match b {
        Button::Play if s.can_play && s.playback != "Playing" => player().call(|c| c.toggle()),
        Button::Pause | Button::Stop if s.can_play && s.playback == "Playing" => player().call(|c| c.toggle()),
        Button::Next if s.can_skip => player().call(|c| c.next()),
        Button::Previous if s.can_skip => player().call(|c| c.prev()),
        _ => {}
    }
}

fn show(c: &SystemMediaTransportControls, names: &[&str], s: &Status) -> windows::core::Result<()> {
    if names.contains(&"PlaybackStatus") {
        c.SetIsEnabled(s.playback != "Stopped")?;
        c.SetPlaybackStatus(match s.playback {
            "Playing" => MediaPlaybackStatus::Playing,
            "Paused" => MediaPlaybackStatus::Paused,
            _ => MediaPlaybackStatus::Stopped,
        })?;
    }
    if names.contains(&"CanPlay") {
        c.SetIsPlayEnabled(s.can_play)?;
        c.SetIsPauseEnabled(s.can_play)?;
        c.SetIsStopEnabled(s.can_play)?;
    }
    if names.contains(&"CanGoNext") {
        c.SetIsNextEnabled(s.can_skip)?;
        c.SetIsPreviousEnabled(s.can_skip)?;
    }
    if names.contains(&"Metadata") {
        let d = c.DisplayUpdater()?;
        d.ClearAll()?; // a song without a cover must not keep the last one's
        d.SetType(MediaPlaybackType::Music)?;
        let m = d.MusicProperties()?;
        m.SetTitle(&HSTRING::from(s.meta.title.as_str()))?;
        m.SetArtist(&HSTRING::from(s.meta.artist.as_deref().unwrap_or_default()))?;
        m.SetAlbumTitle(&HSTRING::from(s.meta.album.as_deref().unwrap_or_default()))?;
        let file = s.meta.art.as_deref().and_then(|u| url::Url::parse(u).ok()?.to_file_path().ok());
        if let Some(f) = file.and_then(|p| StorageFile::GetFileFromPathAsync(&HSTRING::from(p.as_os_str())).ok()?.join().ok()) {
            d.SetThumbnail(&RandomAccessStreamReference::CreateFromFile(&f)?)?;
        }
        d.Update()?;
    }
    Ok(())
}
