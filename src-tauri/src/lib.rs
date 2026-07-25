mod session;
mod spike;

use luozi_core::AppConfig;
use session::{
    session_cancel, session_start, session_status, session_stop, session_undo_last, AppSessionState,
};
use spike::{
    capture_target, deliver_probe, record_one_second_probe, run_delivery_matrix_probe,
    run_focus_abc_probe, run_overlay_cycle_probe, validate_target,
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[tauri::command]
fn get_app_config() -> AppConfig {
    AppConfig::default()
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_about(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("luozi://show-about", ());
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn with_session<F>(app: &tauri::AppHandle, f: F)
where
    F: FnOnce(&AppSessionState),
{
    if let Some(state) = app.try_state::<AppSessionState>() {
        f(&state);
    }
}

/// Tray IA follows design §6.3. M2 enables start / cancel / undo.
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let cfg = AppConfig::default();

    let model_ready = session::asr::default_model_path().is_file();
    let status_label = if model_ready {
        "Whisper 就绪 · 快捷键临时"
    } else {
        "模型未就绪 · 运行 npm run fetch:model"
    };
    let title = MenuItem::with_id(app, "title", "落字", false, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", status_label, false, None::<&str>)?;
    let sep_status = PredefinedMenuItem::separator(app)?;

    let start = MenuItem::with_id(
        app,
        "start",
        "开始语音输入",
        true,
        Some(cfg.continue_speaking_shortcut.as_str()),
    )?;
    let draft = MenuItem::with_id(app, "draft", "打开语音草稿", false, None::<&str>)?;
    let voice_edit = MenuItem::with_id(
        app,
        "voice_edit",
        "说出修改要求",
        false,
        Some(cfg.voice_edit_shortcut.as_str()),
    )?;
    let cancel = MenuItem::with_id(app, "cancel", "取消当前录音", true, Some("Escape"))?;
    let undo = MenuItem::with_id(app, "undo", "撤销上次落字", true, None::<&str>)?;
    let sep_actions = PredefinedMenuItem::separator(app)?;

    let engine_label = if model_ready {
        "引擎：本地 Whisper（M3）"
    } else {
        "引擎：模型未安装"
    };
    let engine = MenuItem::with_id(app, "engine", engine_label, false, None::<&str>)?;
    let mode_label = if cfg.hold_to_talk {
        "录音模式：按住说话"
    } else {
        "录音模式：按一下开关"
    };
    let mode = MenuItem::with_id(app, "mode", mode_label, false, None::<&str>)?;
    let sep_prefs = PredefinedMenuItem::separator(app)?;

    let practice = MenuItem::with_id(app, "practice", "练习窗…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置…", false, None::<&str>)?;
    let perms = MenuItem::with_id(app, "perms", "检查权限…", true, None::<&str>)?;
    let sep_footer = PredefinedMenuItem::separator(app)?;

    let about = MenuItem::with_id(app, "about", "关于落字", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出落字", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &title,
            &status,
            &sep_status,
            &start,
            &draft,
            &voice_edit,
            &cancel,
            &undo,
            &sep_actions,
            &engine,
            &mode,
            &sep_prefs,
            &practice,
            &settings,
            &perms,
            &sep_footer,
            &about,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("落字 Luozi")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "practice" => show_main(app),
            "about" => show_about(app),
            "start" => with_session(app, |s| {
                let _ = session::controller::start_session(app, s);
            }),
            "cancel" => with_session(app, |s| {
                let _ = session::controller::cancel_session(app, s);
            }),
            "undo" => with_session(app, |s| {
                let _ = session::controller::undo_last(app, s);
            }),
            "perms" => {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
                        .spawn();
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                        .spawn();
                }
                show_about(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    #[cfg(target_os = "macos")]
    {
        let icon = Image::from_bytes(include_bytes!("../icons/tray-template@2x.png"))?;
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = app.default_window_icon() {
            tray = tray.icon(icon.clone());
        }
    }

    let _tray = tray.build(app)?;
    Ok(())
}

/// Register continue-speaking with a dedicated handler (do not also call `register`).
/// Escape is registered only while a session is active; handled by Builder::with_handler.
fn register_session_shortcuts(app: &tauri::AppHandle) -> Result<String, String> {
    let cfg = AppConfig::default();
    let primary = cfg.continue_speaking_shortcut.clone();
    let candidates = [
        primary.as_str(),
        "Control+Alt+Period",
        "Control+Shift+Space",
        "Control+Alt+Z",
    ];

    let _ = app.global_shortcut().unregister_all();

    for raw in candidates {
        let sc: Shortcut = match raw.parse() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("luozi: skip bad shortcut {raw}: {e}");
                continue;
            }
        };

        let hold = cfg.hold_to_talk;
        // Carbon hotkeys already arrive on the AppKit main thread.
        // NEVER call run_on_main_thread here — it deadlocks (beachball).
        // Capture AX on this main-thread callback; open the mic on a worker.
        match app.global_shortcut().on_shortcut(sc, move |app, shortcut, event| {
            eprintln!(
                "luozi: hotkey {} {:?}",
                shortcut.into_string(),
                event.state
            );
            let app = app.clone();
            let state = event.state;
            match state {
                ShortcutState::Pressed => {
                    // Do NOT capture AX on this callback thread — prompt/AX work
                    // on the main thread beachballs the app (Esc dies, overlay stuck).
                    std::thread::spawn(move || {
                        with_session(&app, |s| {
                            if let Err(err) = session::controller::start_session(&app, s) {
                                eprintln!("luozi: session_start failed: {err}");
                            }
                        });
                    });
                }
                ShortcutState::Released if hold => {
                    std::thread::spawn(move || {
                        with_session(&app, |s| {
                            if !session::controller::wait_until_recording(s, 2000) {
                                eprintln!("luozi: release before recording ready");
                                return;
                            }
                            if let Err(err) = session::controller::stop_session(&app, s) {
                                eprintln!("luozi: session_stop failed: {err}");
                            }
                        });
                    });
                }
                _ => {}
            }
        }) {
            Ok(()) => {
                eprintln!("luozi: continue-speaking shortcut registered: {raw}");
                if let Some(state) = app.try_state::<AppSessionState>() {
                    if let Ok(mut slot) = state.registered_continue.lock() {
                        *slot = Some(raw.to_string());
                    }
                }
                // Escape must stay registered for the whole process — session-scoped
                // register from a worker was racy and failed while the UI was stuck.
                if let Ok(esc) = "Escape".parse::<Shortcut>() {
                    match app.global_shortcut().register(esc) {
                        Ok(()) => eprintln!("luozi: Escape armed (always-on cancel)"),
                        Err(e) => eprintln!("luozi: Escape register failed: {e}"),
                    }
                }
                return Ok(raw.to_string());
            }
            Err(e) => eprintln!("luozi: on_shortcut({raw}) failed: {e}"),
        }
    }

    Err(
        "Unable to register continue-speaking hotkey; use tray 「开始语音输入」".into(),
    )
}

fn shortcut_escape_handler(
    app: &tauri::AppHandle,
    shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if !shortcut.into_string().eq_ignore_ascii_case("Escape") {
        return;
    }
    if event.state != ShortcutState::Pressed {
        return;
    }
    eprintln!("luozi: Escape → cancel");
    let app = app.clone();
    std::thread::spawn(move || {
        with_session(&app, |s| {
            if let Err(err) = session::controller::cancel_session(&app, s) {
                eprintln!("luozi: session_cancel failed: {err}");
            }
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    shortcut_escape_handler(app, shortcut, event);
                })
                .build(),
        )
        .manage(AppSessionState::default())
        .setup(|app| {
            build_tray(app.handle())?;
            match register_session_shortcuts(app.handle()) {
                Ok(name) => eprintln!("luozi: hotkey ready → {name}"),
                Err(err) => eprintln!("luozi: shortcut registration failed: {err}"),
            }

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
            session_start,
            session_stop,
            session_cancel,
            session_undo_last,
            session_status,
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
