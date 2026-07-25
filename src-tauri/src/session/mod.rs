//! Product session path (M2+): recorder, clipboard undo, controller, local ASR.

pub mod asr;
mod clipboard;
pub mod controller;
mod recorder;

pub use controller::{
    session_cancel, session_start, session_status, session_stop, session_undo_last, AppSessionState,
};
