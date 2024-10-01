mod client;

pub use client::Client;
pub use nostr_sdk;
pub use prediction_market_event;

#[cfg(feature = "cli")]
pub mod cli;
