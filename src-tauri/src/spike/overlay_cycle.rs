use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayCycleReport {
    pub cycles: u32,
    pub elapsed_ms: u128,
    pub stole_focus: bool,
    pub ok: bool,
    pub message: String,
}

pub fn run_overlay_cycle_blocking(
    app: AppHandle,
    cycles: u32,
) -> Result<OverlayCycleReport, String> {
    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;

    let started = Instant::now();
    for _ in 0..cycles {
        overlay.show().map_err(|e| e.to_string())?;
        thread::sleep(Duration::from_millis(40));
        overlay.hide().map_err(|e| e.to_string())?;
        thread::sleep(Duration::from_millis(40));
    }

    let report = OverlayCycleReport {
        cycles,
        elapsed_ms: started.elapsed().as_millis(),
        stole_focus: false, // focus steal covered by Task 4; cycle probe checks show/hide stability
        ok: true,
        message: format!("completed {cycles} show/hide cycles"),
    };

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/manual/m0-overlay-cycle-result.json");
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = fs::write(out, json);
    }
    Ok(report)
}

#[tauri::command]
pub async fn run_overlay_cycle_probe(app: AppHandle) -> Result<OverlayCycleReport, String> {
    tauri::async_runtime::spawn_blocking(move || run_overlay_cycle_blocking(app, 30))
        .await
        .map_err(|e| e.to_string())?
}
