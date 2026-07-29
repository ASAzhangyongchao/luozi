//! Product session path (M2+): recorder, clipboard undo, controller, local/cloud ASR.

pub mod asr;
mod clipboard;
pub mod cloud;
pub mod config_store;
pub mod consent;
pub mod controller;
pub mod credentials;
pub mod draft;
pub mod model_store;
pub mod permissions;
mod recorder;
pub mod settings_api;
pub mod telemetry;
pub mod text_ai;

pub use controller::{
    session_cancel, session_start, session_status, session_stop, session_undo_last, AppSessionState,
};
pub use draft::{DraftStateDto, DraftStore};
