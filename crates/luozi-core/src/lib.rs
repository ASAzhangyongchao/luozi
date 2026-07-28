//! Shared Luozi core types. Desktop shell and future services depend on this crate.
//! No I/O, no API keys, no ASR.

mod config;
mod draft;
mod engine;
mod providers;
mod session;
mod text_edit;
mod voice_energy;

pub use config::{
    url_host, AppConfig, AsrMode, CloudAsrConfig, TextAiConfig, DEFAULT_SCHEMA_VERSION,
    GROQ_DEFAULT_BASE_URL, GROQ_DEFAULT_MODEL, GROQ_PROVIDER_ID, GROQ_TEXT_AI_DEFAULT_MODEL,
    GROQ_TEXT_AI_PROVIDER_ID,
};
pub use draft::DraftDocument;
pub use engine::{route_asr, route_asr_after_local_failure, AsrBackendChoice};
pub use providers::{
    asr_mode_from_str, asr_preset, cloud_asr_from_preset, text_ai_from_preset, text_ai_preset,
    AsrProtocol, AsrProviderPreset, TextAiProviderPreset, ASR_PROVIDERS, TEXT_AI_PROVIDERS,
};
pub use session::{
    route_delivery, DeliveryAction, SessionCommand, SessionEffect, SessionId, SessionMachine,
    SessionPhase, TargetValidation, MIN_RECORDING_MS,
};
pub use text_edit::{
    assess_edit_risk, extract_protected_spans, resolve_edit_scope, EditRisk, EditScope,
    EditScopeKind,
};
pub use voice_energy::{measure_levels, EnergyThrottle, VoiceEnergyMeter};
