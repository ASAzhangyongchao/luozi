mod session;
mod spike;

use luozi_core::AppConfig;
use session::{
    session_cancel, session_start, session_status, session_stop, session_undo_last,
    AppSessionState, DraftStateDto, DraftStore,
};
use spike::{
    capture_target, deliver_probe, record_one_second_probe, run_delivery_matrix_probe,
    run_focus_abc_probe, run_overlay_cycle_probe, validate_target,
};
use std::time::Duration;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[tauri::command]
fn get_app_config() -> AppConfig {
    session::config_store::load()
}

#[tauri::command]
fn draft_state(draft: tauri::State<'_, DraftStore>) -> DraftStateDto {
    draft.snapshot()
}

#[tauri::command]
fn draft_save(text: String, draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.save_text(text)
}

#[tauri::command]
fn draft_undo(draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.undo()
}

#[tauri::command]
fn draft_redo(draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.redo()
}

#[tauri::command]
fn draft_clear(draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.clear()
}

#[tauri::command]
fn draft_load_last(draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.load_last_into_draft()
}

#[tauri::command]
fn draft_insert(
    start: usize,
    end: usize,
    text: String,
    draft: tauri::State<'_, DraftStore>,
) -> Result<DraftStateDto, String> {
    draft.insert_at(start, end, &text)
}

#[tauri::command]
fn draft_set_selection(start: usize, end: usize, draft: tauri::State<'_, DraftStore>) {
    draft.set_selection(start, end);
}

#[tauri::command]
fn draft_apply_pending(draft: tauri::State<'_, DraftStore>) -> Result<DraftStateDto, String> {
    draft.apply_pending()
}

#[tauri::command]
fn draft_reject_pending(draft: tauri::State<'_, DraftStore>) -> Result<(), String> {
    draft.clear_pending();
    Ok(())
}

#[tauri::command]
fn settings_snapshot(app: tauri::AppHandle) -> session::settings_api::SettingsSnapshot {
    session::settings_api::snapshot(&app)
}

#[tauri::command]
fn settings_cycle_asr_mode(app: tauri::AppHandle) -> Result<String, String> {
    session::controller::cycle_asr_mode(&app).map(|m| m.label_zh().to_string())
}

#[tauri::command]
fn settings_set_asr_mode(app: tauri::AppHandle, mode: String) -> Result<String, String> {
    session::controller::set_asr_mode(&app, &mode).map(|m| m.label_zh().to_string())
}

#[tauri::command]
fn settings_set_asr_provider(app: tauri::AppHandle, provider_id: String) -> Result<(), String> {
    session::controller::set_cloud_asr_provider(&app, &provider_id)
}

#[tauri::command]
fn settings_set_text_ai_provider(app: tauri::AppHandle, provider_id: String) -> Result<(), String> {
    session::controller::set_text_ai_provider(&app, &provider_id)
}

#[tauri::command]
fn settings_prompt_asr_key(app: tauri::AppHandle) -> Result<(), String> {
    session::controller::prompt_and_store_asr_key(&app)
}

#[tauri::command]
fn settings_consent_asr(app: tauri::AppHandle) -> Result<(), String> {
    session::controller::consent_current_cloud(&app)
}

#[tauri::command]
fn settings_prompt_text_ai_key(app: tauri::AppHandle) -> Result<(), String> {
    session::controller::prompt_and_store_text_ai_key(&app)
}

#[tauri::command]
fn settings_prompt_text_ai_model(app: tauri::AppHandle) -> Result<(), String> {
    session::controller::prompt_text_ai_model(&app)
}

#[tauri::command]
fn settings_consent_text_ai(app: tauri::AppHandle) -> Result<(), String> {
    session::controller::consent_current_text_ai(&app)
}

#[tauri::command]
fn settings_open_microphone() -> Result<(), String> {
    session::settings_api::open_privacy_microphone()
}

#[tauri::command]
fn settings_open_accessibility() -> Result<(), String> {
    session::settings_api::open_privacy_accessibility()
}

#[tauri::command]
fn settings_open_repo() -> Result<(), String> {
    session::settings_api::open_url(session::settings_api::GITHUB_REPO_URL)
}

#[tauri::command]
fn settings_open_releases() -> Result<(), String> {
    session::settings_api::open_url(session::settings_api::GITHUB_RELEASES_URL)
}

#[tauri::command]
fn settings_open_spike(app: tauri::AppHandle) {
    show_spike(&app);
}

#[tauri::command]
fn settings_take_nav() -> Option<String> {
    session::settings_api::take_pending_section()
}

fn show_draft(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title("落字 · 语音草稿");
        // Ensure product path is not left on ?spike from a prior Spike open.
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_settings(app: &tauri::AppHandle, section: Option<&str>) {
    if let Some(sec) = section {
        session::settings_api::set_pending_section(sec);
    }
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        if let Some(sec) = section {
            let app2 = app.clone();
            let sec = sec.to_string();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(120));
                if let Some(w) = app2.get_webview_window("settings") {
                    let _ = w.emit("settings://nav", serde_json::json!({ "section": sec }));
                }
            });
        }
    }
}

fn show_spike(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("luozi://show-spike", ());
        let _ = window.set_title("落字 · Spike");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_about(app: &tauri::AppHandle) {
    show_settings(app, Some("about"));
}

fn with_session<F>(app: &tauri::AppHandle, f: F)
where
    F: FnOnce(&AppSessionState),
{
    if let Some(state) = app.try_state::<AppSessionState>() {
        f(&state);
    }
}

#[derive(Default)]
struct RegisteredShortcuts {
    continue_speaking: Option<String>,
    voice_edit: Option<String>,
    cancel: Option<String>,
}

fn menu_action_label(action: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(shortcut) => format!("{action} · {shortcut}"),
        None => action.to_string(),
    }
}

/// Tray IA follows design §6.3. M2 enables start / cancel / undo.
fn build_tray(
    app: &tauri::AppHandle,
    registered_shortcuts: &RegisteredShortcuts,
) -> tauri::Result<()> {
    let cfg = session::config_store::load();

    let model_readiness = session::model_store::model_readiness();
    let cloud_ok = session::cloud::cloud_ready(&cfg.cloud_asr);
    let ax_ok = session::controller::accessibility_trusted_for_tray();
    let cloud_label = if cloud_ok {
        "云端已就绪"
    } else {
        "云端未配置"
    };
    let accessibility_label = if ax_ok {
        "辅助功能已开启"
    } else {
        "辅助功能未开启"
    };
    let status_label = format!(
        "{} · {cloud_label} · {accessibility_label}",
        model_readiness.status_label()
    );
    let title = MenuItem::with_id(app, "title", "Luozi 已就绪", false, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", status_label, false, None::<&str>)?;
    let sep_status = PredefinedMenuItem::separator(app)?;

    let start_label = menu_action_label(
        "开始语音输入",
        registered_shortcuts.continue_speaking.as_deref(),
    );
    let start = MenuItem::with_id(app, "start", &start_label, true, None::<&str>)?;
    let cancel_label = menu_action_label("取消当前会话", registered_shortcuts.cancel.as_deref());
    let cancel = MenuItem::with_id(app, "cancel", &cancel_label, true, None::<&str>)?;
    let undo = MenuItem::with_id(app, "undo", "撤销上次落字", true, None::<&str>)?;
    let sep_actions = PredefinedMenuItem::separator(app)?;

    let draft = MenuItem::with_id(app, "draft", "打开语音草稿…", true, None::<&str>)?;
    let voice_edit_label =
        menu_action_label("修改当前草稿", registered_shortcuts.voice_edit.as_deref());
    let voice_edit = MenuItem::with_id(app, "voice_edit", &voice_edit_label, true, None::<&str>)?;
    let sep_prefs = PredefinedMenuItem::separator(app)?;

    let engine = MenuItem::with_id(
        app,
        "engine",
        format!("转写：{}", cfg.asr_mode.label_zh()),
        false,
        None::<&str>,
    )?;
    let fetch_model = MenuItem::with_id(
        app,
        "fetch_model",
        "下载本地模型…",
        !model_readiness.is_ready(),
        None::<&str>,
    )?;

    let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "关于 Luozi", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 Luozi", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &title,
            &status,
            &sep_status,
            &start,
            &cancel,
            &undo,
            &sep_actions,
            &draft,
            &voice_edit,
            &sep_prefs,
            &engine,
            &fetch_model,
            &settings,
            &about,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("落字 Luozi")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "draft" => show_draft(app),
            "settings" => show_settings(app, Some("general")),
            "about" => show_about(app),
            "start" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    with_session(&app, |s| {
                        if let Err(err) = session::controller::toggle_continue_menu_session(&app, s)
                        {
                            if err != "session_busy" {
                                eprintln!("luozi: tray continue toggle failed: {err}");
                            }
                        }
                    });
                });
            }
            "voice_edit" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    with_session(&app, |s| {
                        if let Err(err) =
                            session::controller::toggle_voice_edit_menu_session(&app, s)
                        {
                            if err != "draft_empty" && err != "session_busy" {
                                eprintln!("luozi: tray voice-edit toggle failed: {err}");
                            }
                        }
                    });
                });
            }
            "cancel" => {
                let app = app.clone();
                std::thread::spawn(move || {
                    with_session(&app, |s| {
                        let _ = session::controller::cancel_session(&app, s);
                    });
                });
            }
            "undo" => with_session(app, |s| {
                let _ = session::controller::undo_last(app, s);
            }),
            "fetch_model" => with_session(app, |s| {
                session::controller::fetch_recommended_model(app, s);
            }),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
            // Formal product path is hotkey + menu. Left-click must NOT open the
            // unfinished practice window (that confused formal testing).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                eprintln!("luozi: tray left-click ignored (use menu / hotkey)");
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

/// Register continue-speaking + voice-edit hold hotkeys.
/// Escape is always-on via Builder::with_handler.
fn register_session_shortcuts(app: &tauri::AppHandle) -> RegisteredShortcuts {
    let cfg = session::config_store::load();
    let primary = cfg.continue_speaking_shortcut.clone();
    let voice_edit = cfg.voice_edit_shortcut.clone();
    let candidates = [
        primary.as_str(),
        "Control+Alt+Period",
        "Control+Shift+Space",
        "Control+Alt+Z",
    ];

    let _ = app.global_shortcut().unregister_all();

    let mut registered = RegisteredShortcuts::default();
    for raw in candidates {
        let sc: Shortcut = match raw.parse() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("luozi: skip bad shortcut {raw}: {e}");
                continue;
            }
        };

        let hold = cfg.hold_to_talk;
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
                    with_session(&app, |s| {
                        session::controller::note_hold_pressed(
                            s,
                            session::controller::SessionIntent::Continue,
                        );
                    });
                    std::thread::spawn(move || {
                        with_session(&app, |s| {
                            if let Err(err) = session::controller::start_continue_session(&app, s) {
                                eprintln!("luozi: session_start failed: {err}");
                            }
                        });
                    });
                }
                ShortcutState::Released if hold => {
                    let owned_session_id = app
                        .try_state::<AppSessionState>()
                        .and_then(|s| {
                            session::controller::note_hold_released(
                                &s,
                                session::controller::SessionIntent::Continue,
                            )
                        });
                    let Some(session_id) = owned_session_id else {
                        return;
                    };
                    std::thread::spawn(move || {
                        with_session(&app, |s| {
                            if !session::controller::wait_until_recording_session(
                                s, session_id, 800,
                            ) {
                                eprintln!(
                                    "luozi: release before recording ready (will cancel after mic opens)"
                                );
                                return;
                            }
                            if let Err(err) = session::controller::finish_shortcut_session(
                                &app,
                                s,
                                session::controller::SessionIntent::Continue,
                                session_id,
                            ) {
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
                registered.continue_speaking = Some(raw.to_string());
                if let Some(state) = app.try_state::<AppSessionState>() {
                    if let Ok(mut slot) = state.registered_continue.lock() {
                        *slot = Some(raw.to_string());
                    }
                }
                break;
            }
            Err(e) => eprintln!("luozi: on_shortcut({raw}) failed: {e}"),
        }
    }

    // Voice-edit second hotkey (M7).
    let voice_candidates = [voice_edit.as_str(), "Control+Alt+M", "Control+Alt+Shift+M"];
    for raw in voice_candidates {
        if registered.continue_speaking.as_deref() == Some(raw) {
            continue;
        }
        let sc: Shortcut = match raw.parse() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("luozi: skip bad voice-edit shortcut {raw}: {e}");
                continue;
            }
        };
        let hold = cfg.hold_to_talk;
        match app
            .global_shortcut()
            .on_shortcut(sc, move |app, shortcut, event| {
                eprintln!(
                    "luozi: voice-edit hotkey {} {:?}",
                    shortcut.into_string(),
                    event.state
                );
                let app = app.clone();
                match event.state {
                    ShortcutState::Pressed => {
                        with_session(&app, |s| {
                            session::controller::note_hold_pressed(
                                s,
                                session::controller::SessionIntent::VoiceEdit,
                            );
                        });
                        std::thread::spawn(move || {
                            with_session(&app, |s| {
                                if let Err(err) =
                                    session::controller::start_voice_edit_session(&app, s)
                                {
                                    if err != "draft_empty" {
                                        eprintln!("luozi: voice_edit start failed: {err}");
                                    }
                                }
                            });
                        });
                    }
                    ShortcutState::Released if hold => {
                        let owned_session_id = app.try_state::<AppSessionState>().and_then(|s| {
                            session::controller::note_hold_released(
                                &s,
                                session::controller::SessionIntent::VoiceEdit,
                            )
                        });
                        let Some(session_id) = owned_session_id else {
                            return;
                        };
                        std::thread::spawn(move || {
                            with_session(&app, |s| {
                                if !session::controller::wait_until_recording_session(
                                    s, session_id, 800,
                                ) {
                                    return;
                                }
                                if let Err(err) = session::controller::finish_shortcut_session(
                                    &app,
                                    s,
                                    session::controller::SessionIntent::VoiceEdit,
                                    session_id,
                                ) {
                                    eprintln!("luozi: voice_edit stop failed: {err}");
                                }
                            });
                        });
                    }
                    _ => {}
                }
            }) {
            Ok(()) => {
                eprintln!("luozi: voice-edit shortcut registered: {raw}");
                registered.voice_edit = Some(raw.to_string());
                break;
            }
            Err(e) => eprintln!("luozi: voice-edit on_shortcut({raw}) failed: {e}"),
        }
    }

    if let Ok(esc) = "Escape".parse::<Shortcut>() {
        match app.global_shortcut().register(esc) {
            Ok(()) => {
                eprintln!("luozi: Escape armed (always-on cancel)");
                registered.cancel = Some("Escape".into());
            }
            Err(e) => eprintln!("luozi: Escape register failed: {e}"),
        }
    }

    registered
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
        .manage(DraftStore::default())
        .setup(|app| {
            // Tray-first: never leave a blank main window on launch.
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_title("落字 · 语音草稿");
                let _ = main.hide();
            }
            if let Some(settings) = app.get_webview_window("settings") {
                // Match settings CSS ice-white so dark-mode OS chrome does not show in corners.
                let _ = settings
                    .set_background_color(Some(tauri::window::Color(0xf4, 0xfb, 0xfa, 0xff)));
                let _ = settings.hide();
            }
            if let Some(overlay) = app.get_webview_window("overlay") {
                // Clear plate so CSS border-radius does not sit on a white window.
                let _ = overlay.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                let _ = overlay.set_ignore_cursor_events(true);
                let _ = overlay.hide();
            }
            let registered_shortcuts = register_session_shortcuts(app.handle());
            match registered_shortcuts.continue_speaking.as_deref() {
                Some(name) => eprintln!("luozi: hotkey ready → {name}"),
                None => {
                    eprintln!("luozi: shortcut registration failed; tray start remains available")
                }
            }
            build_tray(app.handle(), &registered_shortcuts)?;
            // First-run mic TCC should not happen mid hold-to-talk.
            session::controller::warmup_microphone_async();

            // M4: unload Whisper after ≥5 minutes idle (poll once a minute).
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    with_session(&handle, |s| {
                        session::controller::maybe_unload_idle_asr(s);
                    });
                });
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
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    if let Some(draft) = window.app_handle().try_state::<DraftStore>() {
                        if let Err(err) = draft.flush() {
                            eprintln!("luozi: draft flush on hide failed: {err}");
                        }
                    }
                    let _ = window.hide();
                } else if window.label() == "settings" {
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
            draft_state,
            draft_save,
            draft_undo,
            draft_redo,
            draft_clear,
            draft_load_last,
            draft_insert,
            draft_set_selection,
            draft_apply_pending,
            draft_reject_pending,
            settings_snapshot,
            settings_cycle_asr_mode,
            settings_set_asr_mode,
            settings_set_asr_provider,
            settings_set_text_ai_provider,
            settings_prompt_asr_key,
            settings_consent_asr,
            settings_prompt_text_ai_key,
            settings_prompt_text_ai_model,
            settings_consent_text_ai,
            settings_open_microphone,
            settings_open_accessibility,
            settings_open_repo,
            settings_open_releases,
            settings_open_spike,
            settings_take_nav,
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
    fn overlay_transparency_uses_macos_private_api() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid Tauri config");
        let private_api = config
            .pointer("/app/macOSPrivateApi")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let cargo_manifest = include_str!("../Cargo.toml");
        let overlay = config
            .pointer("/app/windows")
            .and_then(serde_json::Value::as_array)
            .and_then(|windows| {
                windows.iter().find(|window| {
                    window.get("label").and_then(serde_json::Value::as_str) == Some("overlay")
                })
            })
            .expect("overlay window");

        assert!(
            private_api,
            "rounded HUD requires app.macOSPrivateApi for real window transparency"
        );
        assert!(
            cargo_manifest.contains("macos-private-api"),
            "rounded HUD requires Tauri macos-private-api feature"
        );
        assert_eq!(
            overlay
                .get("transparent")
                .and_then(serde_json::Value::as_bool),
            Some(true),
            "overlay must be transparent so the HUD pill can have rounded corners"
        );
    }
}
