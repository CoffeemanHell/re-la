//! The player. `PlayerCore` decides everything (queue, two decks, crossfade,
//! gapless, listening together, reports, DJ, stations); `Player` runs it on its own thread for a front end.
mod actor;
mod deck;
mod offline;
mod playback;
mod server;
mod state;

pub use actor::{Player, PlayerListener};
pub use deck::{Deck, DeckEvent, DeckSink, EndReason};
pub use state::{ApiThen, Changes, Job, PlayerCore, PlayerSnapshot, StationPort, SyncPort};
#[cfg(test)]
pub(crate) use state::default_options;

#[cfg(test)]
mod tests;
