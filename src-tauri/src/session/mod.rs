//! Product session path (M2+): recorder, clipboard undo, controller, local/cloud ASR.

pub mod asr;
pub mod cloud;
pub mod config_store;
pub mod consent;
pub mod credentials;
pub mod draft;
mod clipboard;
pub mod controller;
pub mod model_store;
mod recorder;
pub mod settings_api;
pub mod text_ai;

pub use controller::{
    session_cancel, session_start, session_status, session_stop, session_undo_last, AppSessionState,
};
pub use draft::{DraftStateDto, DraftStore};
