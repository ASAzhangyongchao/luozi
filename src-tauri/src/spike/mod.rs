mod target;

#[cfg(target_os = "macos")]
mod delivery_probe;
#[cfg(target_os = "macos")]
mod focus_probe;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

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
    #[cfg(target_os = "macos")]
    {
        macos::deliver_probe(&token)
    }
    #[cfg(target_os = "windows")]
    {
        windows::deliver_probe(&token)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = token;
        Err("deliver_probe unsupported on this platform".into())
    }
}
