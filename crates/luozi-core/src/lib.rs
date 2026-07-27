//! Shared Luozi core types. Desktop shell and future services depend on this crate.
//! No I/O, no API keys, no ASR.

mod config;
mod draft;
mod engine;
mod session;

pub use config::{
    url_host, AppConfig, AsrMode, CloudAsrConfig, DEFAULT_SCHEMA_VERSION, GROQ_DEFAULT_BASE_URL,
    GROQ_DEFAULT_MODEL, GROQ_PROVIDER_ID,
};
pub use draft::DraftDocument;
pub use engine::{route_asr, route_asr_after_local_failure, AsrBackendChoice};
pub use session::{
    route_delivery, DeliveryAction, SessionCommand, SessionEffect, SessionId, SessionMachine,
    SessionPhase, TargetValidation, MIN_RECORDING_MS,
};
