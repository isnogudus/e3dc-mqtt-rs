//! E3DC client wrapper module
//!
//! Provides a high-level interface to query E3DC data via RSCP protocol.

pub mod client;
pub mod types;

pub use client::E3dcClient;
pub use types::*;
