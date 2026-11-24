//! Waveforms: every song's levels worked out once on the server (ffmpeg), served to the clients.
pub mod decode;
pub mod levels;
pub mod navidrome;
pub mod route;
pub mod scan;
pub mod store;

pub use levels::{Accumulator, BINS, Levels};
pub use navidrome::{Account, Fetch, Navidrome, Song};
pub use scan::{WaveformConfig, start};
pub use store::{Key, Store, body, decode, encode, safe_id};
