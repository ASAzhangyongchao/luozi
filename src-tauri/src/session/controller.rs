//! App-owned session controller: machine + recorder + delivery.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use luozi_core::{
    assess_edit_risk, resolve_edit_scope, route_asr, route_asr_after_local_failure, route_delivery,
    AppConfig, AsrBackendChoice, AsrMode, DeliveryAction, EditRisk, SessionCommand, SessionEffect,
    SessionMachine, SessionPhase, TargetValidation,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::spike::{self, TargetToken, ValidationState};

use super::asr::{self, AsrEngine};
use super::clipboard::ClipboardGate;
use super::recorder::{SessionRecorder, MAX_RECORDING_MS};

/// Kept for docs/tests that mention the M2 placeholder string.
#[allow(dead_code)]
pub const FAKE_TRANSCRIPT: &str = "落字测试";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SessionIntent {
    #[default]
    Continue,
    VoiceEdit,
}

/// Whisper + deliver must not hang the session forever (stuck busy overlay).
const TRANSCRIBE_WATCHDOG_MS: u64 = 45_000;
/// Brief status on overlay before auto-dismiss (inserted / canceled / errors).
const OVERLAY_AUTO_HIDE_MS: u64 = 2_200;

fn arm_escape(_app: &AppHandle) {
    // Escape is registered once at startup; keep it for the whole process life.
}

fn disarm_escape(_app: &AppHandle) {
    // no-op — see arm_escape
}

pub struct AppSessionState {
    pub machine: Mutex<SessionMachine>,
    pub recorder: Mutex<SessionRecorder>,
    pub clipboard: Mutex<ClipboardGate>,
    pub source_target: Mutex<Option<TargetToken>>,
    /// Lazy-loaded Whisper context (M3).
    pub asr: Mutex<Option<AsrEngine>>,
    /// Last successful ASR use — idle unload after 5 minutes.
    pub asr_last_used: Mutex<Option<Instant>>,
    /// Actually registered continue-speaking binding (may differ from config provisional).
    pub registered_continue: Mutex<Option<String>>,
    /// Hold-to-talk: true while continue-speaking key is down.
    /// Cleared on Released / Esc. Start checks this after mic opens (TCC can block).
    pub hold_active: AtomicBool,
    /// Frontmost app pid when recording started (for restore + paste/type).
    pub source_pid: Mutex<Option<i32>>,
    /// Model download in flight (tray debounce).
    pub model_fetching: AtomicBool,
    /// Continue dictation vs voice-edit instruction (M7).
    pub intent: Mutex<SessionIntent>,
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
            asr: Mutex::new(None),
            asr_last_used: Mutex::new(None),
            registered_continue: Mutex::new(None),
            hold_active: AtomicBool::new(false),
            source_pid: Mutex::new(None),
            model_fetching: AtomicBool::new(false),
            intent: Mutex::new(SessionIntent::Continue),
            config: super::config_store::load(),
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

/// Show overlay with a short-lived status, then hide (does not block).
fn emit_transient(app: &AppHandle, phase: &str, message: &str) {
    show_overlay(app, true);
    emit_phase(app, phase, message);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(OVERLAY_AUTO_HIDE_MS));
        // Only hide if we are idle — avoid racing a new recording that started.
        if let Some(state) = app.try_state::<AppSessionState>() {
            let idle = state
                .machine
                .lock()
                .ok()
                .map(|m| m.is_idle())
                .unwrap_or(true);
            if idle {
                show_overlay(&app, false);
            }
        } else {
            show_overlay(&app, false);
        }
    });
}

fn touch_asr_used(state: &AppSessionState) {
    let _ = state
        .asr_last_used
        .lock()
        .map(|mut g| *g = Some(Instant::now()));
}

/// Unload Whisper if idle ≥5 minutes and session is idle.
pub fn maybe_unload_idle_asr(state: &AppSessionState) {
    let idle_ok = state
        .machine
        .lock()
        .ok()
        .map(|m| m.is_idle())
        .unwrap_or(false);
    if !idle_ok {
        return;
    }
    let stale = state
        .asr_last_used
        .lock()
        .ok()
        .and_then(|g| *g)
        .map(|t| t.elapsed().as_secs() >= 300)
        .unwrap_or(false);
    if !stale {
        return;
    }
    if let Ok(mut slot) = state.asr.lock() {
        if slot.take().is_some() {
            eprintln!("luozi: unloaded Whisper after 5m idle");
            let _ = state.asr_last_used.lock().map(|mut g| *g = None);
        }
    }
}

/// Tray / command: download + verify recommended model.
pub fn fetch_recommended_model(app: &AppHandle, state: &AppSessionState) {
    if state
        .model_fetching
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        emit_transient(app, "warn", "模型正在下载中…");
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppSessionState>() else {
            return;
        };
        emit_transient(&app, "transcribing", "正在下载推荐模型…");
        let result = super::model_store::ensure_recommended_model(|pct, phase| {
            let _ = app.emit(
                "model://progress",
                serde_json::json!({ "pct": pct, "phase": phase }),
            );
            if pct == 1 || pct == 50 || pct == 95 || pct == 100 {
                eprintln!("luozi: model fetch {pct}% ({phase})");
            }
        });
        state.model_fetching.store(false, Ordering::SeqCst);
        match result {
            Ok(path) => {
                eprintln!("luozi: model fetch ok → {}", path.display());
                emit_transient(&app, "inserted", "模型已就绪，可按住说话");
            }
            Err(err) => {
                eprintln!("luozi: model fetch failed: {err}");
                emit_transient(&app, "error", &err);
            }
        }
    });
}

fn spawn_transcribe_watchdog(app: AppHandle, session_id: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(TRANSCRIBE_WATCHDOG_MS));
        let Some(state) = app.try_state::<AppSessionState>() else {
            return;
        };
        let stuck = state
            .machine
            .lock()
            .ok()
            .and_then(|m| match m.phase() {
                SessionPhase::Transcribing { session_id: sid }
                | SessionPhase::Delivering { session_id: sid }
                    if *sid == session_id =>
                {
                    Some(*sid)
                }
                _ => None,
            })
            .is_some();
        if stuck {
            eprintln!("luozi: watchdog force-cancel stuck transcribe/deliver {session_id}");
            if let Ok(mut rec) = state.recorder.lock() {
                rec.cancel();
            }
            let _ = state
                .machine
                .lock()
                .map(|mut m| m.handle(SessionCommand::Cancel));
            let _ = state.source_target.lock().map(|mut g| *g = None);
            disarm_escape(&app);
            emit_transient(&app, "error", "落字超时，已取消，请重试");
        }
    });
}

/// ASR + delivery on a worker so hotkey/Esc stay responsive.
fn finish_transcribe_async(
    app: AppHandle,
    session_id: u64,
    duration_ms: u64,
    capture: super::recorder::AudioCapture,
    language: String,
) {
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppSessionState>() else {
            return;
        };

        let cfg = super::config_store::load();
        let pcm = asr::resample_to_16k_mono(
            &capture.samples,
            capture.sample_rate,
            capture.channels,
        );
        if pcm.is_empty() {
            let _ = fail_transcribe(&app, &state, "no_speech".into());
            return;
        }

        let local_ready = asr::default_model_path().is_file();
        let cloud_ok = super::cloud::cloud_ready(&cfg.cloud_asr);
        let choice = route_asr(cfg.asr_mode, local_ready, cloud_ok);

        let transcript = match run_asr_with_route(&state, &cfg, &pcm, &language, choice) {
            Ok(text) => {
                eprintln!("luozi: asr ok → {text}");
                text
            }
            Err(err) => {
                eprintln!("luozi: asr failed: {err}");
                let _ = fail_transcribe(&app, &state, err);
                return;
            }
        };

        let intent = state
            .intent
            .lock()
            .ok()
            .map(|g| *g)
            .unwrap_or(SessionIntent::Continue);
        let _ = state
            .intent
            .lock()
            .map(|mut g| *g = SessionIntent::Continue);

        if intent == SessionIntent::VoiceEdit {
            // Consume session machine so we leave busy state, then apply text AI to draft.
            let _ = {
                let Ok(mut machine) = state.machine.lock() else {
                    return;
                };
                machine.handle(SessionCommand::TranscriptionReady {
                    session_id,
                    text: transcript.clone(),
                })
            };
            // Force idle regardless of delivery effect.
            let _ = state.machine.lock().map(|mut m| {
                m.handle(SessionCommand::DeliveryFinished {
                    session_id,
                    ok: true,
                })
            });
            disarm_escape(&app);
            apply_voice_edit(&app, &state, &cfg, &transcript);
            return;
        }

        let deliver_effect = {
            let Ok(mut machine) = state.machine.lock() else {
                return;
            };
            machine.handle(SessionCommand::TranscriptionReady {
                session_id,
                text: transcript,
            })
        };

        match deliver_effect {
            SessionEffect::Deliver { session_id, text } => {
                eprintln!("luozi: delivering transcript ({duration_ms}ms hold)");
                disarm_escape(&app);
                show_overlay(&app, false);
                std::thread::sleep(std::time::Duration::from_millis(80));
                let ok = apply_delivery(&app, &state, &text);
                let _ = state.machine.lock().map(|mut m| {
                    m.handle(SessionCommand::DeliveryFinished {
                        session_id,
                        ok: ok.machine_ok(),
                    })
                });
                let app2 = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(OVERLAY_AUTO_HIDE_MS));
                    if let Some(state) = app2.try_state::<AppSessionState>() {
                        if state.machine.lock().ok().map(|m| m.is_idle()).unwrap_or(true) {
                            show_overlay(&app2, false);
                        }
                    }
                });
            }
            SessionEffect::StaleIgnored { .. } => {
                eprintln!("luozi: transcript stale (canceled during ASR)");
                disarm_escape(&app);
                show_overlay(&app, false);
            }
            other => {
                eprintln!("luozi: unexpected_transcribe_effect: {other:?}");
                let _ = fail_transcribe(
                    &app,
                    &state,
                    format!("unexpected_transcribe_effect: {other:?}"),
                );
            }
        }
    });
}

fn run_asr_with_route(
    state: &AppSessionState,
    cfg: &AppConfig,
    pcm: &[f32],
    language: &str,
    choice: AsrBackendChoice,
) -> Result<String, String> {
    match choice {
        AsrBackendChoice::NeedConfig { reason } => Err(reason.to_string()),
        AsrBackendChoice::Cloud => {
            eprintln!("luozi: asr route → cloud ({})", cfg.cloud_asr.provider_id);
            super::cloud::transcribe_pcm(&cfg.cloud_asr, pcm, language)
        }
        AsrBackendChoice::Local => match run_local_asr(state, pcm, language) {
            Ok(text) => Ok(text),
            Err(local_err) => {
                eprintln!("luozi: local asr failed ({local_err}); checking cloud fallback");
                match route_asr_after_local_failure(
                    cfg.asr_mode,
                    super::cloud::cloud_ready(&cfg.cloud_asr),
                ) {
                    AsrBackendChoice::Cloud => {
                        eprintln!("luozi: asr fallback → cloud");
                        super::cloud::transcribe_pcm(&cfg.cloud_asr, pcm, language)
                    }
                    AsrBackendChoice::NeedConfig { reason } => {
                        if cfg.asr_mode == AsrMode::LocalOnly {
                            Err(local_err)
                        } else if local_err.contains("model_") {
                            Err(format!("{local_err}; {reason}"))
                        } else {
                            Err(local_err)
                        }
                    }
                    AsrBackendChoice::Local => Err(local_err),
                }
            }
        },
    }
}

fn run_local_asr(state: &AppSessionState, pcm: &[f32], language: &str) -> Result<String, String> {
    let engine = {
        let mut slot = state
            .asr
            .lock()
            .map_err(|_| "asr_lock_failed".to_string())?;
        asr::take_ready_engine(&mut slot)?
    };
    let result = engine.transcribe(pcm, language);
    let _ = state.asr.lock().map(|mut s| *s = Some(engine));
    touch_asr_used(state);
    result
}

fn map_validation(state: ValidationState) -> TargetValidation {
    match state {
        ValidationState::SameTarget => TargetValidation::SameTarget,
        ValidationState::Changed => TargetValidation::Changed,
        ValidationState::Unsupported => TargetValidation::Unsupported,
        ValidationState::Secure => TargetValidation::Secure,
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

const MAIN_THREAD_AX_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

/// Dispatch to AppKit main with a hard timeout.
/// NEVER call blocking AX work while already on the main thread — that beachballs Esc.
fn on_main_thread_timeout<R, F>(
    app: &AppHandle,
    timeout: std::time::Duration,
    f: F,
) -> Result<R, String>
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    if is_main_thread() {
        return Err("refused_ax_on_main_thread".into());
    }
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| format!("main_thread_dispatch_failed: {e}"))?;
    rx.recv_timeout(timeout)
        .map_err(|_| "main_thread_timeout".into())
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

/// Best-effort focus capture with timeout. Never blocks the AppKit main run loop from itself.
pub fn capture_source_token(app: &AppHandle) -> TargetToken {
    let app_c = app.clone();
    match on_main_thread_timeout(app, MAIN_THREAD_AX_TIMEOUT, move || {
        match spike::capture_target() {
            Ok(t) => Ok(t),
            Err(err) => Err(err),
        }
    }) {
        Ok(Ok(t)) if t.is_secure => {
            // Caller decides whether to cancel / reject; do not flash overlay here.
            t
        }
        Ok(Ok(t)) => {
            eprintln!(
                "luozi: capture ok pid={} role={}",
                t.process_id, t.role
            );
            t
        }
        Ok(Err(err)) => {
            // Soft-fail is normal (no AX focus / practice window). Stay silent —
            // flashing「未捕获输入框」on the dictation HUD made it look stuck.
            eprintln!("luozi: capture_target soft-fail: {err}");
            let _ = app_c;
            dummy_token()
        }
        Err(err) => {
            eprintln!("luozi: capture skipped: {err}");
            dummy_token()
        }
    }
}

fn show_overlay(app: &AppHandle, visible: bool) {
    show_overlay_ex(app, visible, false);
}

fn show_overlay_ex(app: &AppHandle, visible: bool, interactive: bool) {
    // Never block session control; never use the short AX timeout (hide was failing
    // silently and leaving the HUD stuck on the last message).
    let app = app.clone();
    std::thread::spawn(move || {
        let app2 = app.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if app
            .run_on_main_thread(move || {
                if let Some(window) = app2.get_webview_window("overlay") {
                    // Typeless-style cancel/confirm need hits during recording;
                    // keep pass-through the rest of the time so HUD never steals clicks.
                    let _ = window.set_ignore_cursor_events(!interactive);
                    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                    if visible {
                        let _ = window.show();
                    } else {
                        let _ = window.hide();
                    }
                }
                let _ = tx.send(());
            })
            .is_err()
        {
            return;
        }
        if rx.recv_timeout(std::time::Duration::from_secs(2)).is_err() {
            eprintln!(
                "luozi: overlay {} timed out on main thread",
                if visible { "show" } else { "hide" }
            );
        }
    });
}

#[derive(Debug)]
enum DeliverOutcome {
    Inserted,
    Clipboard,
    Discard,
    Error(String),
}

#[derive(Debug, Clone, Copy)]
enum DeliveryResult {
    /// Text reached the caret (AX or typed).
    Inserted,
    /// Left on clipboard / paste attempted — still a usable outcome.
    Clipboard,
    /// Nothing useful happened.
    Failed,
}

impl DeliveryResult {
    fn machine_ok(self) -> bool {
        !matches!(self, DeliveryResult::Failed)
    }
}

fn restore_source_app(state: &AppSessionState) {
    let pid = state
        .source_pid
        .lock()
        .ok()
        .and_then(|g| *g)
        .filter(|p| *p > 0);
    let Some(pid) = pid else {
        eprintln!("luozi: no source_pid to restore");
        return;
    };
    match spike::activate_pid(pid) {
        Ok(()) => eprintln!("luozi: restored frontmost pid={pid}"),
        Err(err) => eprintln!("luozi: restore pid={pid} failed: {err}"),
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
}

fn draft_window_is_front(app: &AppHandle) -> bool {
    let Some(main) = app.get_webview_window("main") else {
        return false;
    };
    main.is_visible().unwrap_or(false) && main.is_focused().unwrap_or(false)
}

fn map_text_ai_err(err: &str) -> &'static str {
    match err {
        "text_ai_not_consented" | "cloud_not_consented" => "文本 AI 未授权",
        "text_ai_unauthorized" | "cloud_unauthorized" => "文本 AI Key 无效",
        "text_ai_timeout" => "文本 AI 超时",
        "text_ai_not_configured" | "text_ai_not_ready" => "请先配置文本 AI",
        "edit_scope_invalid" => "修改范围无效",
        "text_ai_invalid_patch" => "AI 返回无效",
        _ => "文本 AI 失败",
    }
}

fn apply_voice_edit(app: &AppHandle, _state: &AppSessionState, cfg: &AppConfig, instruction: &str) {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        emit_transient(app, "error", "没听清修改要求");
        return;
    }

    let Some(draft) = app.try_state::<super::DraftStore>() else {
        emit_transient(app, "error", "草稿不可用");
        return;
    };

    if draft.is_empty() {
        emit_transient(app, "error", "先说一段或粘贴文字");
        return;
    }

    if !super::text_ai::text_ai_ready(&cfg.text_ai) {
        emit_transient(app, "error", "请先配置并同意文本 AI");
        return;
    }

    let text = draft.text_snapshot();
    let (sel_start, sel_end) = draft.selection();
    let scope = match resolve_edit_scope(&text, sel_start, sel_end, instruction) {
        Ok(s) => s,
        Err(err) => {
            emit_transient(app, "error", map_text_ai_err(&err));
            return;
        }
    };
    let original = text[scope.start..scope.end].to_string();
    if original.trim().is_empty() {
        emit_transient(app, "error", "没有可修改的内容");
        return;
    }

    emit_phase(app, "transcribing", "正在修改…");
    show_overlay(app, true);

    let proposed = match super::text_ai::rewrite_scope(&cfg.text_ai, instruction, &original) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("luozi: text_ai failed: {err}");
            emit_transient(app, "error", map_text_ai_err(&err));
            return;
        }
    };

    if proposed == original {
        emit_transient(app, "inserted", "无需修改");
        return;
    }

    match assess_edit_risk(&scope, &original, &proposed, instruction) {
        EditRisk::Low => match draft.insert_at(scope.start, scope.end, &proposed) {
            Ok(_) => {
                let _ = app.emit("draft://updated", serde_json::json!({ "reason": "voice_edit" }));
                emit_transient(app, "inserted", "已修改");
            }
            Err(err) => {
                eprintln!("luozi: apply edit failed: {err}");
                emit_transient(app, "error", "写入草稿失败");
            }
        },
        EditRisk::High { reasons } => {
            let reason_labels: Vec<String> = reasons.iter().map(|r| (*r).to_string()).collect();
            draft.set_pending(super::draft::PendingEdit {
                start: scope.start,
                end: scope.end,
                proposed: proposed.clone(),
                reasons: reason_labels.clone(),
                original: original.clone(),
            });
            let preview: String = proposed.chars().take(80).collect();
            let _ = app.emit(
                "draft://edit-preview",
                serde_json::json!({
                    "reasons": reason_labels,
                    "originalPreview": original.chars().take(80).collect::<String>(),
                    "proposedPreview": preview,
                }),
            );
            emit_transient(app, "confirm", "高风险修改：请在草稿窗确认");
        }
    }
}

/// Open draft, ensure content, then start hold-to-talk as a voice-edit session.
pub fn start_voice_edit_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    // Show workbench first.
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }

    if let Some(draft) = app.try_state::<super::DraftStore>() {
        if draft.is_empty() {
            let _ = draft.load_last_into_draft();
            let _ = app.emit(
                "draft://updated",
                serde_json::json!({ "reason": "load_last" }),
            );
        }
        if draft.is_empty() {
            emit_transient(app, "error", "先说一段或粘贴文字");
            return Err("draft_empty".into());
        }
    }

    let _ = state
        .intent
        .lock()
        .map(|mut g| *g = SessionIntent::VoiceEdit);
    start_session(app, state)
}

pub fn start_continue_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    let _ = state
        .intent
        .lock()
        .map(|mut g| *g = SessionIntent::Continue);
    start_session(app, state)
}

fn apply_delivery(app: &AppHandle, state: &AppSessionState, text: &str) -> DeliveryResult {
    // M6: when the draft workbench is open, write into the draft instead of external apps.
    if draft_window_is_front(app) {
        if let Some(draft) = app.try_state::<super::DraftStore>() {
            match draft.append_transcript(text) {
                Ok(()) => {
                    let _ = app.emit(
                        "draft://updated",
                        serde_json::json!({ "reason": "dictation" }),
                    );
                    emit_transient(app, "inserted", "已写入草稿");
                    eprintln!("luozi: delivered into draft workbench");
                    return DeliveryResult::Inserted;
                }
                Err(err) => {
                    eprintln!("luozi: draft append failed: {err}");
                    emit_transient(app, "error", "草稿写入失败");
                    return DeliveryResult::Failed;
                }
            }
        }
    }

    // Remember successful external deliveries for 「载入最近落字」.
    if let Some(draft) = app.try_state::<super::DraftStore>() {
        draft.remember_transcript(text);
    }

    let trusted = spike::accessibility_is_trusted();
    eprintln!(
        "luozi: deliver begin trusted={} text_chars={}",
        trusted,
        text.chars().count()
    );
    if !trusted {
        // Settings UI can show ON while AXIsProcessTrusted is still false after adhoc re-sign.
        eprintln!(
            "luozi: AXIsProcessTrusted=false — toggle Luozi OFF/ON in Accessibility, or remove+re-add"
        );
    }

    // Always hide is done by caller; wait a beat then restore the app user was in.
    std::thread::sleep(std::time::Duration::from_millis(60));
    restore_source_app(state);

    let token = state
        .source_target
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(dummy_token);

    let token = {
        let fresh = capture_source_token(app);
        if fresh.process_id != 0 {
            fresh
        } else if token.process_id != 0 {
            token
        } else {
            fresh
        }
    };
    eprintln!(
        "luozi: deliver target pid={} role={}",
        token.process_id, token.role
    );

    let token_for_ax = token.clone();
    let text_owned = text.to_string();
    let outcome = if trusted {
        on_main_thread_timeout(app, MAIN_THREAD_AX_TIMEOUT, move || {
            let validation = spike::validate_target(token_for_ax.clone())
                .unwrap_or(ValidationState::Unsupported);
            match route_delivery(map_validation(validation)) {
                DeliveryAction::Discard => DeliverOutcome::Discard,
                DeliveryAction::Clipboard => DeliverOutcome::Clipboard,
                DeliveryAction::Insert => {
                    match spike::deliver_text(token_for_ax, text_owned.clone()) {
                        Ok(ValidationState::SameTarget) => DeliverOutcome::Inserted,
                        Ok(other) => {
                            if route_delivery(map_validation(other)) == DeliveryAction::Clipboard {
                                DeliverOutcome::Clipboard
                            } else {
                                DeliverOutcome::Discard
                            }
                        }
                        Err(err) => DeliverOutcome::Error(err),
                    }
                }
            }
        })
    } else {
        Err("ax_not_trusted".into())
    };

    let result = match outcome {
        Ok(DeliverOutcome::Inserted) => {
            eprintln!("luozi: delivered insert → {text}");
            emit_transient(app, "inserted", "已落字");
            DeliveryResult::Inserted
        }
        Ok(DeliverOutcome::Discard) => {
            eprintln!("luozi: delivered discard");
            emit_transient(app, "discarded", "安全输入：已丢弃，未写入");
            DeliveryResult::Inserted
        }
        Ok(DeliverOutcome::Clipboard)
        | Ok(DeliverOutcome::Error(_))
        | Err(_) => {
            // Electron / unverified AX: clipboard + ⌘V, then VERIFY before saying 已落字.
            // CGEvent "Ok" only means events were posted — never treat as success alone.
            restore_source_app(state);
            let _ = spike::type_text_via_cg_events(text);
            std::thread::sleep(std::time::Duration::from_millis(80));
            if spike::focused_field_contains(text) {
                eprintln!("luozi: verified after CG type → {text}");
                emit_transient(app, "inserted", "已落字");
                DeliveryResult::Inserted
            } else {
                write_clipboard_with_paste(app, state, text, trusted)
            }
        }
    };

    let _ = state.source_target.lock().map(|mut g| *g = None);
    result
}

fn write_clipboard_with_paste(
    app: &AppHandle,
    state: &AppSessionState,
    text: &str,
    trusted: bool,
) -> DeliveryResult {
    match state.clipboard.lock() {
        Ok(mut gate) => match gate.write_with_undo(text) {
            Ok(()) => {
                restore_source_app(state);
                let pasted = spike::paste_via_cmd_v().is_ok();
                std::thread::sleep(std::time::Duration::from_millis(100));
                let verified = spike::focused_field_contains(text);
                eprintln!(
                    "luozi: clipboard written pasted={pasted} trusted={trusted} verified={verified}"
                );
                if verified {
                    emit_transient(app, "inserted", "已落字");
                    DeliveryResult::Inserted
                } else if !trusted {
                    emit_transient(
                        app,
                        "error",
                        "辅助功能未生效：关掉再打开 Luozi 开关",
                    );
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                        .spawn();
                    DeliveryResult::Clipboard
                } else {
                    emit_transient(
                        app,
                        "clipboard",
                        "未进输入框，已到剪贴板 · 请 ⌘V",
                    );
                    DeliveryResult::Clipboard
                }
            }
            Err(err) => {
                emit_transient(app, "error", &err);
                DeliveryResult::Failed
            }
        },
        Err(_) => {
            emit_transient(app, "error", "clipboard_lock_failed");
            DeliveryResult::Failed
        }
    }
}

pub fn wait_until_recording(state: &AppSessionState, timeout_ms: u64) -> bool {
    let steps = (timeout_ms / 50).max(1);
    for _ in 0..steps {
        if is_recording_phase(state) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

pub fn is_recording_phase(state: &AppSessionState) -> bool {
    state
        .machine
        .lock()
        .ok()
        .map(|m| m.is_recording())
        .unwrap_or(false)
}

fn fail_transcribe(app: &AppHandle, state: &AppSessionState, err: String) -> Result<SessionStatus, String> {
    let _ = state
        .machine
        .lock()
        .map(|mut m| m.handle(SessionCommand::Cancel));
    disarm_escape(app);
    emit_transient(app, "error", &user_facing_asr_error(&err));
    Err(err)
}

fn user_facing_asr_error(err: &str) -> String {
    if err.contains("cloud_not_consented") {
        return "云端未授权 · 托盘可同意上传".into();
    }
    if err.contains("cloud_unauthorized") {
        return "云端 Key 无效 · 请重新配置".into();
    }
    if err.contains("cloud_rate_limited") {
        return "云端限流 · 请稍后或改本地".into();
    }
    if err.contains("cloud_timeout") {
        return "云端超时 · 未自动重试".into();
    }
    if err.contains("cloud_protocol_error") {
        return "云端协议错误".into();
    }
    if err.contains("local_not_ready") || err.contains("model_missing") {
        return "本地模型未就绪 · 可下载或改云端".into();
    }
    if err.contains("cloud_not_ready") || err.contains("no_engine") {
        return "无可用引擎 · 配置本地或 Groq".into();
    }
    if err.contains("no_speech") {
        return "未检测到语音".into();
    }
    err.to_string()
}

/// Cycle ASR mode Auto → LocalOnly → CloudOnly and persist.
pub fn cycle_asr_mode(app: &AppHandle) -> Result<AsrMode, String> {
    let cfg = super::config_store::update(|c| {
        c.asr_mode = c.asr_mode.cycle();
    })?;
    emit_transient(
        app,
        "inserted",
        &format!("引擎模式：{}", cfg.asr_mode.label_zh()),
    );
    Ok(cfg.asr_mode)
}

/// Record consent for current cloud preset host.
pub fn consent_current_cloud(app: &AppHandle) -> Result<(), String> {
    let cfg = super::config_store::load();
    let host = cfg
        .cloud_asr
        .host()
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    super::consent::grant(&cfg.cloud_asr.provider_id, &host)?;
    emit_transient(
        app,
        "inserted",
        &format!("已同意上传到 {host}"),
    );
    Ok(())
}

/// Prompt for Groq API key (macOS) and store in Keychain.
pub fn prompt_and_store_groq_key(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"
        set answer to display dialog "粘贴 Groq API Key（仅存本机钥匙串，不会进配置文件）" default answer "" with hidden answer buttons {"取消", "保存"} default button "保存"
        if button returned of answer is "取消" then return ""
        return text returned of answer
        "#;
        let out = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| format!("osascript_failed: {e}"))?;
        if !out.status.success() {
            return Err("canceled".into());
        }
        let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if key.is_empty() {
            return Err("canceled".into());
        }
        let cfg = super::config_store::load();
        super::credentials::set_secret(&cfg.cloud_asr.credential_ref, &key)?;
        // Ensure Groq preset fields are present.
        let _ = super::config_store::update(|c| {
            if c.cloud_asr.provider_id.is_empty() {
                c.cloud_asr = luozi_core::CloudAsrConfig::default();
            }
        });
        emit_transient(app, "inserted", "Groq Key 已写入钥匙串");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("keychain_unsupported_platform".into())
    }
}

/// Prompt for text AI API key (macOS) and store in Keychain (separate from ASR).
pub fn prompt_and_store_text_ai_key(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"
        set answer to display dialog "粘贴文本 AI API Key（Groq/OpenAI 兼容；仅存本机钥匙串）" default answer "" with hidden answer buttons {"取消", "保存"} default button "保存"
        if button returned of answer is "取消" then return ""
        return text returned of answer
        "#;
        let out = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| format!("osascript_failed: {e}"))?;
        if !out.status.success() {
            return Err("canceled".into());
        }
        let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if key.is_empty() {
            return Err("canceled".into());
        }
        let cfg = super::config_store::load();
        super::credentials::set_secret(&cfg.text_ai.credential_ref, &key)?;
        let _ = super::config_store::update(|c| {
            if c.text_ai.provider_id.is_empty() {
                c.text_ai = luozi_core::TextAiConfig::default();
            }
        });
        emit_transient(app, "inserted", "文本 AI Key 已写入钥匙串");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("keychain_unsupported_platform".into())
    }
}

pub fn consent_current_text_ai(app: &AppHandle) -> Result<(), String> {
    let cfg = super::config_store::load();
    let host = cfg
        .text_ai
        .host()
        .ok_or_else(|| "text_ai_protocol_error".to_string())?;
    super::consent::grant(&cfg.text_ai.provider_id, &host)?;
    emit_transient(app, "inserted", &format!("已同意文本 AI 上传到 {host}"));
    Ok(())
}

pub fn note_hold_pressed(state: &AppSessionState) {
    state.hold_active.store(true, Ordering::SeqCst);
}

pub fn note_hold_released(state: &AppSessionState) {
    state.hold_active.store(false, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn is_hold_active(state: &AppSessionState) -> bool {
    state.hold_active.load(Ordering::SeqCst)
}

/// Tray status helper — mirrors AXIsProcessTrusted for the running binary.
pub fn accessibility_trusted_for_tray() -> bool {
    crate::spike::accessibility_is_trusted()
}

/// Trigger the macOS mic TCC prompt at launch so the first hotkey hold is not blocked.
pub fn warmup_microphone_async() {
    std::thread::spawn(|| {
        let mut rec = SessionRecorder::new();
        match rec.start() {
            Ok(()) => {
                std::thread::sleep(std::time::Duration::from_millis(120));
                match rec.stop() {
                    Ok(c) => eprintln!(
                        "luozi: mic warmup ok ({} frames, {} ms)",
                        c.frames, c.duration_ms
                    ),
                    Err(err) => eprintln!("luozi: mic warmup stop: {err}"),
                }
            }
            Err(err) => eprintln!("luozi: mic warmup start: {err}"),
        }
    });
}

pub fn start_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    // HARD RULE: never wait on Accessibility before opening the mic.
    // Capture runs in the background; deliver path falls back to clipboard+⌘V.
    start_session_with_token(app, state, dummy_token())
}

pub fn start_session_with_token(
    app: &AppHandle,
    state: &AppSessionState,
    token: TargetToken,
) -> Result<SessionStatus, String> {
    if token.is_secure {
        emit_transient(app, "rejected", "安全输入区域：未开始录音");
        return Err("secure_input_rejected".into());
    }

    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Start)
    };

    match effect {
        SessionEffect::BeganRecording { session_id } => {
            // Remember where the user was typing BEFORE mic TCC / overlay.
            let _ = state
                .source_pid
                .lock()
                .map(|mut g| *g = spike::current_frontmost_pid());

            // Mic open can block on first-run TCC. Do not leave「听写中」if the user
            // already released during that dialog (hold_active cleared on Released).
            let start_result = state
                .recorder
                .lock()
                .map_err(|_| "recorder_lock_failed".to_string())?
                .start();
            if let Err(err) = start_result {
                let _ = state
                    .machine
                    .lock()
                    .map(|mut m| m.handle(SessionCommand::Cancel));
                state.hold_active.store(false, Ordering::SeqCst);
                emit_transient(app, "error", &err);
                return Err(err);
            }

            if !state.hold_active.load(Ordering::SeqCst) {
                eprintln!(
                    "luozi: hold released during mic open (likely TCC) — cancel session {session_id}"
                );
                if let Ok(mut rec) = state.recorder.lock() {
                    rec.cancel();
                }
                let _ = state
                    .machine
                    .lock()
                    .map(|mut m| m.handle(SessionCommand::Cancel));
                let _ = state.source_target.lock().map(|mut g| *g = None);
                disarm_escape(app);
                emit_transient(
                    app,
                    "canceled",
                    "已授权麦克风。请再按住说话，松手落字",
                );
                return status_from(state, "canceled_after_permission".into());
            }

            let _ = state.source_target.lock().map(|mut g| *g = Some(token));
            show_overlay_ex(app, true, true);
            arm_escape(app);
            let editing = state
                .intent
                .lock()
                .ok()
                .map(|g| *g == SessionIntent::VoiceEdit)
                .unwrap_or(false);
            if editing {
                emit_phase(app, "recording_edit", "说修改要求…");
            } else {
                emit_phase(app, "recording", "听写中");
            }

            // Best-effort focus capture in background (never blocks start).
            let app_cap = app.clone();
            std::thread::spawn(move || {
                let t = capture_source_token(&app_cap);
                if t.is_secure {
                    // Too late to reject cleanly mid-record; cancel instead.
                    if let Some(state) = app_cap.try_state::<AppSessionState>() {
                        let _ = cancel_session(&app_cap, &state);
                        emit_transient(&app_cap, "rejected", "安全输入区域：已取消");
                    }
                    return;
                }
                if t.process_id != 0 {
                    if let Some(state) = app_cap.try_state::<AppSessionState>() {
                        let _ = state.source_target.lock().map(|mut g| *g = Some(t.clone()));
                        let _ = state
                            .source_pid
                            .lock()
                            .map(|mut g| *g = Some(t.process_id as i32));
                    }
                }
                // Keep HUD on recording copy even if capture was soft-fail.
                if let Some(state) = app_cap.try_state::<AppSessionState>() {
                    if is_recording_phase(&state) {
                        emit_phase(&app_cap, "recording", "听写中");
                    }
                }
            });

            // Auto-stop at max duration.
            let app_handle = app.clone();
            let sid = session_id;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(MAX_RECORDING_MS));
                let Some(state) = app_handle.try_state::<AppSessionState>() else {
                    return;
                };
                let should_stop = state
                    .machine
                    .lock()
                    .ok()
                    .and_then(|m| m.recording_session_id())
                    == Some(sid);
                if should_stop {
                    let _ = stop_session(&app_handle, &state);
                }
            });

            // Watchdog: if still Recording well past max, force-cancel (Esc-dead scenarios).
            let app_wd = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(MAX_RECORDING_MS + 5_000));
                let Some(state) = app_wd.try_state::<AppSessionState>() else {
                    return;
                };
                let stuck = state
                    .machine
                    .lock()
                    .ok()
                    .and_then(|m| m.recording_session_id())
                    == Some(sid);
                if stuck {
                    eprintln!("luozi: watchdog force-cancel stuck recording {sid}");
                    let _ = cancel_session(&app_wd, &state);
                }
            });

            // Short watchdog: hold released while UI still says recording (lost Released).
            let app_hold = app.clone();
            std::thread::spawn(move || {
                for _ in 0..40 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let Some(state) = app_hold.try_state::<AppSessionState>() else {
                        return;
                    };
                    let still = state
                        .machine
                        .lock()
                        .ok()
                        .and_then(|m| m.recording_session_id())
                        == Some(sid);
                    if !still {
                        return;
                    }
                    if !state.hold_active.load(Ordering::SeqCst) {
                        eprintln!("luozi: hold inactive while recording — auto stop {sid}");
                        let _ = stop_session(&app_hold, &state);
                        return;
                    }
                }
            });

            status_from(state, format!("recording session {session_id}"))
        }
        SessionEffect::RejectedBusy => {
            // Silent: do not reflash overlay — that felt like a stuck popup.
            // Overlay (if any) keeps showing「落字中」until ASR finishes or Esc/watchdog.
            eprintln!("luozi: start ignored (session busy)");
            Err("session_busy".into())
        }
        other => Err(format!("unexpected_start_effect: {other:?}")),
    }
}

pub fn stop_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    // Duplicate Released / race after ASR started: ignore without Cancel.
    if !is_recording_phase(state) {
        eprintln!("luozi: stop ignored (not recording)");
        return status_from(state, "stop_ignored".into());
    }

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
        Err(err) if err == "not_recording" => {
            eprintln!("luozi: stop not_recording (no cancel)");
            return status_from(state, "stop_ignored".into());
        }
        Err(err) => {
            let _ = cancel_session(app, state);
            return Err(err);
        }
    };

    let duration_ms = audio.as_ref().map(|a| a.duration_ms).unwrap_or(duration_ms);

    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Stop { duration_ms })
    };

    match effect {
        SessionEffect::RejectedTooShort { .. } => {
            eprintln!("luozi: session too short (<300ms)");
            disarm_escape(app);
            emit_transient(app, "too_short", "时间太短，请按住再松手");
            status_from(state, "too_short".into())
        }
        SessionEffect::BeginTranscribe { session_id } => {
            emit_phase(app, "transcribing", "落字中");
            show_overlay_ex(app, true, false);
            let language = super::config_store::load().language;
            let Some(capture) = audio else {
                return fail_transcribe(app, state, "asr_no_audio".into());
            };

            spawn_transcribe_watchdog(app.clone(), session_id);
            finish_transcribe_async(app.clone(), session_id, duration_ms, capture, language);
            status_from(state, "transcribing".into())
        }
        SessionEffect::RejectedBusy => {
            eprintln!("luozi: stop ignored (session busy)");
            Err("session_busy".into())
        }
        other => Err(format!("unexpected_stop_effect: {other:?}")),
    }
}

pub fn cancel_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    state.hold_active.store(false, Ordering::SeqCst);
    let _ = state.source_pid.lock().map(|mut g| *g = None);
    if let Ok(mut rec) = state.recorder.lock() {
        rec.cancel();
    }
    let effect = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        machine.handle(SessionCommand::Cancel)
    };
    let _ = state.source_target.lock().map(|mut g| *g = None);
    disarm_escape(app);
    match effect {
        SessionEffect::Canceled { .. } => {
            emit_transient(app, "canceled", "已取消");
            status_from(state, "canceled".into())
        }
        SessionEffect::RejectedBusy => {
            show_overlay(app, false);
            status_from(state, "idle".into())
        }
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
        fake_transcript: false,
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
