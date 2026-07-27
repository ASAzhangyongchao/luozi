mod audio;
mod overlay_cycle;
mod target;

#[cfg(target_os = "macos")]
mod delivery_probe;
#[cfg(target_os = "macos")]
mod focus_probe;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use audio::{record_one_second_probe, run_audio_probe_blocking};
pub use overlay_cycle::{run_overlay_cycle_blocking, run_overlay_cycle_probe};

#[cfg(target_os = "macos")]
pub use delivery_probe::{run_delivery_matrix_blocking, run_delivery_matrix_probe};
#[cfg(target_os = "macos")]
pub use focus_probe::{run_focus_abc_probe, run_focus_abc_probe_blocking};

pub use target::{TargetToken, ValidationState};

#[cfg(not(target_os = "macos"))]
use serde::Serialize;

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusProbeReport {
    pub ok: bool,
    pub message: String,
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn run_focus_abc_probe() -> Result<FocusProbeReport, String> {
    Ok(FocusProbeReport {
        ok: false,
        message: "Focus ABC probe is only implemented for macOS in M0".into(),
    })
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryMatrixReport {
    pub ok: bool,
    pub message: String,
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn run_delivery_matrix_probe() -> Result<DeliveryMatrixReport, String> {
    Ok(DeliveryMatrixReport {
        ok: false,
        message: "Delivery matrix probe is only implemented for macOS in M0".into(),
    })
}

#[tauri::command]
pub fn capture_target() -> Result<TargetToken, String> {
    #[cfg(target_os = "macos")]
    {
        macos::capture_target()
    }
    #[cfg(target_os = "windows")]
    {
        windows::capture_target()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("capture_target unsupported on this platform".into())
    }
}

#[tauri::command]
pub fn validate_target(token: TargetToken) -> Result<ValidationState, String> {
    #[cfg(target_os = "macos")]
    {
        macos::validate_target(&token)
    }
    #[cfg(target_os = "windows")]
    {
        windows::validate_target(&token)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = token;
        Err("validate_target unsupported on this platform".into())
    }
}

#[tauri::command]
pub fn deliver_probe(token: TargetToken) -> Result<ValidationState, String> {
    deliver_text(token, "落字测试".into())
}

pub fn deliver_text(token: TargetToken, text: String) -> Result<ValidationState, String> {
    #[cfg(target_os = "macos")]
    {
        macos::deliver_text(&token, &text)
    }
    #[cfg(target_os = "windows")]
    {
        windows::deliver_text(&token, &text)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (token, text);
        Err("deliver_text unsupported on this platform".into())
    }
}

/// After writing the clipboard, synthesize ⌘V so text lands at the caret.
pub fn paste_via_cmd_v() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::paste_via_cmd_v()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("paste_via_cmd_v unsupported on this platform".into())
    }
}

#[cfg(target_os = "macos")]
pub fn current_frontmost_pid() -> Option<i32> {
    macos::current_frontmost_pid()
}

#[cfg(not(target_os = "macos"))]
pub fn current_frontmost_pid() -> Option<i32> {
    None
}

#[cfg(target_os = "macos")]
pub fn activate_pid(pid: i32) -> Result<(), String> {
    macos::activate_pid(pid)
}

#[cfg(not(target_os = "macos"))]
pub fn activate_pid(_pid: i32) -> Result<(), String> {
    Err("activate_pid unsupported".into())
}

#[cfg(target_os = "macos")]
pub fn type_text_via_cg_events(text: &str) -> Result<(), String> {
    macos::type_text_via_cg_events(text)
}

#[cfg(not(target_os = "macos"))]
pub fn type_text_via_cg_events(_text: &str) -> Result<(), String> {
    Err("type_text_via_cg_events unsupported".into())
}

#[cfg(target_os = "macos")]
pub fn focused_field_contains(needle: &str) -> bool {
    macos::focused_field_contains(needle)
}

#[cfg(not(target_os = "macos"))]
pub fn focused_field_contains(_needle: &str) -> bool {
    false
}

#[cfg(target_os = "macos")]
pub fn accessibility_is_trusted() -> bool {
    macos::accessibility_is_trusted()
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_is_trusted() -> bool {
    false
}
