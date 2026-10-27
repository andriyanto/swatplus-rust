//! SWAT+ output: the text tables SWAT+ writes ([`text`]), the same tables as Parquet
//! ([`parquet`]), the places they go ([`store`]), tar archives for the browser ([`tar`]), and
//! progress reports as JSON lines ([`progress`]).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]

pub mod parquet;
pub mod progress;
pub mod store;
pub mod tar;
pub mod text;

pub use store::{DirStore, MemStore, Store};
pub use text::TextTables;
