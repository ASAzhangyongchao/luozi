mod spike;

use luozi_core::AppConfig;
use spike::{
    capture_target, deliver_probe, record_one_second_probe, run_delivery_matrix_probe,
    run_focus_abc_probe, run_overlay_cycle_probe, validate_target,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

#[tauri::command]
fn get_app_config() -> AppConfig {
    AppConfig::default()
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let practice = MenuItem::with_id(app, "practice", "练习窗", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&practice, &about, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("落字 Luozi")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "practice" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "about" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("luozi://show-about", ());
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    let _tray = tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            build_tray(app.handle())?;

            #[cfg(target_os = "macos")]
            {
                use std::fs;
                use std::path::PathBuf;
                use std::thread;
                use std::time::Duration;

                if std::env::var_os("LUOZI_M0_AUTO_FOCUS").is_some() {
                    let handle = app.handle().clone();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_secs(2));
                        match spike::run_focus_abc_probe_blocking(handle) {
                            Ok(report) => {
                                let json = serde_json::to_string_pretty(&report)
                                    .unwrap_or_else(|e| e.to_string());
                                let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                    .join("../tests/manual/m0-focus-auto-result.json");
                                let _ = fs::write(&out, &json);
                                eprintln!(
                                    "LUOZI_M0_AUTO_FOCUS result written to {}",
                                    out.display()
                                );
                                eprintln!("{json}");
                            }
                            Err(err) => eprintln!("LUOZI_M0_AUTO_FOCUS failed: {err}"),
                        }
                    });
                }

                if std::env::var_os("LUOZI_M0_AUTO_DELIVERY").is_some() {
                    thread::spawn(|| {
                        thread::sleep(Duration::from_secs(2));
                        let report = spike::run_delivery_matrix_blocking();
                        let json =
                            serde_json::to_string_pretty(&report).unwrap_or_else(|e| e.to_string());
                        eprintln!("LUOZI_M0_AUTO_DELIVERY result:\n{json}");
                    });
                }
            }

            if std::env::var_os("LUOZI_M0_AUTO_AUDIO").is_some() {
                use std::fs;
                use std::path::PathBuf;
                use std::thread;
                use std::time::Duration;

                thread::spawn(|| {
                    thread::sleep(Duration::from_secs(2));
                    match spike::run_audio_probe_blocking() {
                        Ok(report) => {
                            let json = serde_json::to_string_pretty(&report)
                                .unwrap_or_else(|e| e.to_string());
                            let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("../tests/manual/m0-audio-auto-result.json");
                            let _ = fs::write(&out, &json);
                            eprintln!("LUOZI_M0_AUTO_AUDIO result written to {}", out.display());
                            eprintln!("{json}");
                        }
                        Err(err) => {
                            let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("../tests/manual/m0-audio-auto-result.json");
                            let json = format!("{{\n  \"ok\": false,\n  \"error\": {err:?}\n}}");
                            let _ = fs::write(&out, &json);
                            eprintln!("LUOZI_M0_AUTO_AUDIO failed: {err}");
                        }
                    }
                });
            }

            if std::env::var_os("LUOZI_M0_AUTO_OVERLAY").is_some() {
                use std::thread;
                use std::time::Duration;

                let handle = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(2));
                    match spike::run_overlay_cycle_blocking(handle, 30) {
                        Ok(report) => {
                            let json = serde_json::to_string_pretty(&report)
                                .unwrap_or_else(|e| e.to_string());
                            eprintln!("LUOZI_M0_AUTO_OVERLAY result:\n{json}");
                        }
                        Err(err) => eprintln!("LUOZI_M0_AUTO_OVERLAY failed: {err}"),
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            run_focus_abc_probe,
            run_delivery_matrix_probe,
            run_overlay_cycle_probe,
            record_one_second_probe,
            capture_target,
            validate_target,
            deliver_probe
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn m0_macos_fallback_does_not_enable_private_api() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid Tauri config");
        let private_api = config
            .pointer("/app/macOSPrivateApi")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let cargo_manifest = include_str!("../Cargo.toml");

        assert!(!private_api, "M0 must not enable app.macOSPrivateApi");
        assert!(
            !cargo_manifest.contains("macos-private-api"),
            "M0 must not compile Tauri's macos-private-api feature"
        );
    }

    #[test]
    fn m0_overlay_uses_opaque_fallback_without_private_api() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid Tauri config");
        let overlay = config
            .pointer("/app/windows")
            .and_then(serde_json::Value::as_array)
            .and_then(|windows| {
                windows.iter().find(|window| {
                    window.get("label").and_then(serde_json::Value::as_str) == Some("overlay")
                })
            })
            .expect("overlay window");

        assert_eq!(
            overlay
                .get("transparent")
                .and_then(serde_json::Value::as_bool),
            Some(false),
            "M0 fallback must use an opaque window until a public native material bridge exists"
        );
    }
}
