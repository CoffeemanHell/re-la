//! re-la's core, shared by the desktop app (cxx-qt) and Android (UniFFI): no Qt, no Android in here.
#![forbid(unsafe_code)]
pub mod covers;
pub mod player;
pub mod queue;
pub mod radio;
pub mod settings;
pub mod subsonic;
pub mod subwave;
pub mod sync;
pub mod waveform;
