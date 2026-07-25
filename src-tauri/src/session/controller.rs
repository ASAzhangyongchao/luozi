//! App-owned session controller: machine + recorder + delivery.

use std::sync::Mutex;

use luozi_core::{
    route_delivery, AppConfig, DeliveryAction, SessionCommand, SessionEffect, SessionMachine,
    SessionPhase, TargetValidation,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::spike::{self, TargetToken, ValidationState};

use super::clipboard::ClipboardGate;
use super::recorder::{SessionRecorder, MAX_RECORDING_MS};

/// M2 placeholder transcript until ASR (M3).
pub const FAKE_TRANSCRIPT: &str = "落字测试";

fn arm_escape(app: &AppHandle) {
    if let Ok(sc) = "Escape".parse::<Shortcut>() {
        let _ = app.global_shortcut().register(sc);
    }
}

fn disarm_escape(app: &AppHandle) {
    if let Ok(sc) = "Escape".parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(sc);
    }
}

pub struct AppSessionState {
    pub machine: Mutex<SessionMachine>,
    pub recorder: Mutex<SessionRecorder>,
    pub clipboard: Mutex<ClipboardGate>,
    pub source_target: Mutex<Option<TargetToken>>,
    /// Actually registered continue-speaking binding (may differ from config provisional).
    pub registered_continue: Mutex<Option<String>>,
    #[allow(dead_code)]
    pub config: AppConfig,
}

impl Default for AppSessionState {
    fn default() -> Self {
        Self {
            machine: Mutex::new(SessionMachine::new()),
            recorder: Mutex::new(SessionRecorder::new()),
            clipboard: Mutex::new(ClipboardGate::default()),
            source_target: Mutex::new(None),
            registered_continue: Mutex::new(None),
            config: AppConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub phase: String,
    pub session_id: Option<u64>,
    pub can_undo: bool,
    pub fake_transcript: bool,
    pub registered_continue: Option<String>,
    pub message: String,
}

fn phase_name(phase: &SessionPhase) -> &'static str {
    match phase {
        SessionPhase::Idle => "idle",
        SessionPhase::Recording { .. } => "recording",
        SessionPhase::Transcribing { .. } => "transcribing",
        SessionPhase::Delivering { .. } => "delivering",
    }
}

fn emit_phase(app: &AppHandle, phase: &str, message: &str) {
    let _ = app.emit(
        "session://phase",
        serde_json::json!({ "phase": phase, "message": message }),
    );
}

fn map_validation(state: ValidationState) -> TargetValidation {
    match state {
        ValidationState::SameTarget => TargetValidation::SameTarget,
        ValidationState::Changed => TargetValidation::Changed,
        ValidationState::Unsupported => TargetValidation::Unsupported,
        ValidationState::Secure => TargetValidation::Secure,
    }
}

fn show_overlay(app: &AppHandle, visible: bool) {
    if let Some(window) = app.get_webview_window("overlay") {
        if visible {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }
}

fn is_main_thread() -> bool {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn pthread_main_np() -> i32;
        }
        unsafe { pthread_main_np() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Run Accessibility / UI work on the AppKit main thread (required on macOS).
fn on_main_thread<R, F>(app: &AppHandle, f: F) -> Result<R, String>
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    if is_main_thread() {
        return Ok(f());
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| format!("main_thread_dispatch_failed: {e}"))?;
    rx.recv()
        .map_err(|_| "main_thread_result_dropped".into())
}

fn dummy_token() -> TargetToken {
    TargetToken {
        platform: std::env::consts::OS.into(),
        process_id: 0,
        window_id: "none".into(),
        element_id: "none".into(),
        role: "unknown".into(),
        is_secure: false,
        captured_at_ms: 0,
    }
}

/// Capture focused target. Prefer calling from the AppKit main thread.
pub fn capture_source_token(app: &AppHandle) -> TargetToken {
    match spike::capture_target() {
        Ok(t) if t.is_secure => {
            emit_phase(app, "rejected", "安全输入区域：未开始录音");
            t
        }
        Ok(t) => {
            eprintln!(
                "luozi: capture ok pid={} role={}",
                t.process_id, t.role
            );
            t
        }
        Err(err) => {
            eprintln!("luozi: capture_target soft-fail: {err}");
            emit_phase(app, "warn", "未捕获输入框，将落到剪贴板");
            dummy_token()
        }
    }
}

#[derive(Debug)]
enum DeliverOutcome {
    Inserted,
    Clipboard,
    Discard,
    Error(String),
}

fn apply_delivery(app: &AppHandle, state: &AppSessionState, text: &str) -> bool {
    let token = state
        .source_target
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(dummy_token);

    let token_for_ax = token.clone();
    let text_owned = text.to_string();
    let outcome = on_main_thread(app, move || {
        let validation = spike::validate_target(token_for_ax.clone())
            .unwrap_or(ValidationState::Unsupported);
        match route_delivery(map_validation(validation)) {
            DeliveryAction::Discard => DeliverOutcome::Discard,
            DeliveryAction::Clipboard => DeliverOutcome::Clipboard,
            DeliveryAction::Insert => match spike::deliver_text(token_for_ax, text_owned) {
                Ok(ValidationState::SameTarget) => DeliverOutcome::Inserted,
                Ok(other) => {
                    if route_delivery(map_validation(other)) == DeliveryAction::Clipboard {
                        DeliverOutcome::Clipboard
                    } else {
                        DeliverOutcome::Discard
                    }
                }
                Err(err) => DeliverOutcome::Error(err),
            },
        }
    });

    let ok = match outcome {
        Ok(DeliverOutcome::Inserted) => {
            eprintln!("luozi: delivered insert → {text}");
            emit_phase(app, "inserted", "已落字");
            true
        }
        Ok(DeliverOutcome::Clipboard) => {
            eprintln!("luozi: delivered clipboard → {text}");
            write_clipboard(app, state, text)
        }
        Ok(DeliverOutcome::Discard) => {
            eprintln!("luozi: delivered discard");
            emit_phase(app, "discarded", "安全输入：已丢弃，未写入");
            true
        }
        Ok(DeliverOutcome::Error(err)) => {
            eprintln!("luozi: deliver_text error: {err}; clipboard fallback");
            write_clipboard(app, state, text)
        }
        Err(err) => {
            eprintln!("luozi: deliver dispatch failed: {err}; clipboard fallback");
            write_clipboard(app, state, text)
        }
    };

    let _ = state.source_target.lock().map(|mut g| *g = None);
    ok
}

fn write_clipboard(app: &AppHandle, state: &AppSessionState, text: &str) -> bool {
    match state.clipboard.lock() {
        Ok(mut gate) => match gate.write_with_undo(text) {
            Ok(()) => {
                // Prefer caret insert via ⌘V when AX direct set is unavailable.
                match spike::paste_via_cmd_v() {
                    Ok(()) => {
                        eprintln!("luozi: paste ⌘V ok → {text}");
                        emit_phase(app, "inserted", "已落字");
                        true
                    }
                    Err(err) => {
                        eprintln!("luozi: paste ⌘V failed: {err}; left on clipboard");
                        emit_phase(app, "clipboard", "已落到剪贴板，可 ⌘V 粘贴");
                        true
                    }
                }
            }
            Err(err) => {
                emit_phase(app, "error", &err);
                false
            }
        },
        Err(_) => {
            emit_phase(app, "error", "clipboard_lock_failed");
            false
        }
    }
}

pub fn wait_until_recording(state: &AppSessionState, timeout_ms: u64) -> bool {
    let steps = (timeout_ms / 50).max(1);
    for _ in 0..steps {
        let recording = state
            .machine
            .lock()
            .ok()
            .map(|m| matches!(m.phase(), SessionPhase::Recording { .. }))
            .unwrap_or(false);
        if recording {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

pub fn start_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    let token = if is_main_thread() {
        capture_source_token(app)
    } else {
        let app_c = app.clone();
        on_main_thread(app, move || capture_source_token(&app_c))?
    };
    start_session_with_token(app, state, token)
}

pub fn start_session_with_token(
    app: &AppHandle,
    state: &AppSessionState,
    token: TargetToken,
) -> Result<SessionStatus, String> {
    if token.is_secure {
        emit_phase(app, "rejected", "安全输入区域：未开始录音");
        return Err("secure_input_rejected".into());
    }

    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Start)
    };

    match effect {
        SessionEffect::BeganRecording { session_id } => {
            if let Err(err) = state
                .recorder
                .lock()
                .map_err(|_| "recorder_lock_failed".to_string())?
                .start()
            {
                let _ = state
                    .machine
                    .lock()
                    .map(|mut m| m.handle(SessionCommand::Cancel));
                emit_phase(app, "error", &err);
                return Err(err);
            }
            let _ = state.source_target.lock().map(|mut g| *g = Some(token));
            show_overlay(app, true);
            arm_escape(app);
            emit_phase(app, "recording", "听写中 · Esc 取消");

            // Auto-stop at max duration.
            let app_handle = app.clone();
            let sid = session_id;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(MAX_RECORDING_MS));
                let Some(state) = app_handle.try_state::<AppSessionState>() else {
                    return;
                };
                let active = state
                    .machine
                    .lock()
                    .ok()
                    .and_then(|m| m.active_session_id());
                if active == Some(sid) {
                    let _ = stop_session(&app_handle, &state);
                }
            });

            status_from(state, format!("recording session {session_id}"))
        }
        SessionEffect::RejectedBusy => {
            emit_phase(app, "busy", "上一段正在落字，可按 Esc 取消");
            Err("session_busy".into())
        }
        other => Err(format!("unexpected_start_effect: {other:?}")),
    }
}

pub fn stop_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    let duration_ms = state
        .recorder
        .lock()
        .map_err(|_| "recorder_lock_failed".to_string())?
        .elapsed_ms();

    let audio = match state
        .recorder
        .lock()
        .map_err(|_| "recorder_lock_failed".to_string())?
        .stop()
    {
        Ok(a) => Some(a),
        Err(err) => {
            let _ = cancel_session(app, state);
            return Err(err);
        }
    };

    let duration_ms = audio.as_ref().map(|a| a.duration_ms).unwrap_or(duration_ms);
    // Drop samples immediately — M2 does not run ASR.
    drop(audio);

    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Stop { duration_ms })
    };

    match effect {
        SessionEffect::RejectedTooShort { .. } => {
            eprintln!("luozi: session too short (<300ms)");
            disarm_escape(app);
            show_overlay(app, false);
            emit_phase(app, "too_short", "时间太短，请按住再松手");
            status_from(state, "too_short".into())
        }
        SessionEffect::BeginTranscribe { session_id } => {
            emit_phase(app, "transcribing", "落字中");
            let deliver_effect = {
                let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
                machine.handle(SessionCommand::TranscriptionReady {
                    session_id,
                    text: FAKE_TRANSCRIPT.into(),
                })
            };
            match deliver_effect {
                SessionEffect::Deliver { session_id, text } => {
                    eprintln!("luozi: delivering fake transcript ({duration_ms}ms hold)");
                    // Hide overlay first so the source app keeps / regains key focus for paste.
                    disarm_escape(app);
                    show_overlay(app, false);
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    let ok = apply_delivery(app, state, &text);
                    let _ = state.machine.lock().map(|mut m| {
                        m.handle(SessionCommand::DeliveryFinished { session_id, ok })
                    });
                    if ok {
                        status_from(state, "completed".into())
                    } else {
                        Err("delivery_failed".into())
                    }
                }
                SessionEffect::StaleIgnored { .. } => {
                    disarm_escape(app);
                    show_overlay(app, false);
                    status_from(state, "stale".into())
                }
                other => Err(format!("unexpected_transcribe_effect: {other:?}")),
            }
        }
        SessionEffect::RejectedBusy => Err("session_busy".into()),
        other => Err(format!("unexpected_stop_effect: {other:?}")),
    }
}

pub fn cancel_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    if let Ok(mut rec) = state.recorder.lock() {
        rec.cancel();
    }
    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Cancel)
    };
    let _ = state.source_target.lock().map(|mut g| *g = None);
    disarm_escape(app);
    show_overlay(app, false);
    match effect {
        SessionEffect::Canceled { .. } => {
            emit_phase(app, "canceled", "已取消");
            status_from(state, "canceled".into())
        }
        SessionEffect::RejectedBusy => status_from(state, "idle".into()),
        other => Err(format!("unexpected_cancel_effect: {other:?}")),
    }
}

pub fn undo_last(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    let mut gate = state.clipboard.lock().map_err(|_| "clipboard_lock_failed")?;
    gate.undo_last()?;
    emit_phase(app, "undone", "已撤销剪贴板落字");
    drop(gate);
    status_from(state, "undone".into())
}

fn status_from(state: &AppSessionState, message: String) -> Result<SessionStatus, String> {
    let machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
    let can_undo = state
        .clipboard
        .lock()
        .map(|mut g| g.can_undo())
        .unwrap_or(false);
    let registered_continue = state
        .registered_continue
        .lock()
        .ok()
        .and_then(|g| g.clone());
    Ok(SessionStatus {
        phase: phase_name(machine.phase()).into(),
        session_id: machine.active_session_id(),
        can_undo,
        fake_transcript: true,
        registered_continue,
        message,
    })
}

#[tauri::command]
pub fn session_start(app: AppHandle, state: State<'_, AppSessionState>) -> Result<SessionStatus, String> {
    start_session(&app, &state)
}

#[tauri::command]
pub fn session_stop(app: AppHandle, state: State<'_, AppSessionState>) -> Result<SessionStatus, String> {
    stop_session(&app, &state)
}

#[tauri::command]
pub fn session_cancel(app: AppHandle, state: State<'_, AppSessionState>) -> Result<SessionStatus, String> {
    cancel_session(&app, &state)
}

#[tauri::command]
pub fn session_undo_last(
    app: AppHandle,
    state: State<'_, AppSessionState>,
) -> Result<SessionStatus, String> {
    undo_last(&app, &state)
}

#[tauri::command]
pub fn session_status(state: State<'_, AppSessionState>) -> Result<SessionStatus, String> {
    status_from(&state, "ok".into())
}
