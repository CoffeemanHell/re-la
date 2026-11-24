//! The desktop front end: QObjects that QML sees, on top of rela-core.
pub mod app;
pub mod covers;
pub mod hub;
pub mod mpris;
pub mod player;
pub mod qt;
pub mod qtgui;
#[cfg(windows)]
mod smtc;
pub mod subwave;
pub mod sync;
pub mod wheel;
