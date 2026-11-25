//! What the player needs from an audio output: the desktop gives it two libmpv decks (crates/mpv), Android two
//! ExoPlayers. A deck only plays what it is told; the core decides everything.
use std::sync::Arc;

pub trait Deck: Send {
    /// Replace whatever the deck holds with url (loadfile replace), paused or not.
    fn load(&mut self, url: &str, paused: bool);
    /// Gapless: queue url after the current file in the same deck (loadfile append).
    fn append(&mut self, url: &str);
    /// Forget the appended file (playlist-clear).
    fn clear_appended(&mut self);
    /// After a gapless move: drop the finished file from the deck's playlist (playlist-remove 0).
    fn drop_first(&mut self);
    fn set_paused(&mut self, paused: bool);
    /// Absolute and exact.
    fn seek(&mut self, s: f64);
    /// 0..100, mpv's cubic scale.
    fn set_volume(&mut self, v: f64);
    fn set_speed(&mut self, s: f64);
    fn stop(&mut self);
    /// Exclusive device access (audio-exclusive + ao-reload).
    fn set_exclusive(&mut self, on: bool);
    /// The output device by the name audio_devices gives; "auto": the system's choice. Desktop only.
    fn set_audio_device(&mut self, _name: &str) {}
    /// (name, description) of each output device.
    fn audio_devices(&self) -> Vec<(String, String)> {
        vec![]
    }
    /// One line for RELA_AUDIO_LOG: what the output says about itself.
    fn audio_log(&self) -> String {
        String::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Eof,
    Stop,
    Error,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeckEvent {
    /// time-pos; None when the deck has none.
    Position(Option<f64>),
    Duration(f64),
    /// audio-bitrate in bit/s; None when unknown.
    Bitrate(Option<f64>),
    /// A string property (audio-params/format, current-ao, …); None when it went away.
    Prop(String, Option<String>),
    StartFile,
    FileLoaded,
    EndFile { reason: EndReason, error: String },
}

/// Where a deck reports: its index (0 or 1) and the event. Called on any thread.
pub type DeckSink = Arc<dyn Fn(usize, DeckEvent) + Send + Sync>;
