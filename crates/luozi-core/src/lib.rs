//! Shared Luozi core types. Desktop shell and future services depend on this crate.
//! No I/O, no API keys, no ASR.

mod config;

pub use config::{AppConfig, DEFAULT_SCHEMA_VERSION};
