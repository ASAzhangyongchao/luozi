//! Shared Luozi core types. Desktop shell and future services depend on this crate.
//! No I/O, no API keys, no ASR.

mod config;
mod session;

pub use config::{AppConfig, DEFAULT_SCHEMA_VERSION};
pub use session::{
    route_delivery, DeliveryAction, SessionCommand, SessionEffect, SessionId, SessionMachine,
    SessionPhase, TargetValidation, MIN_RECORDING_MS,
};

