//! App-owned session controller: machine + recorder + delivery.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
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
use super::recorder::{EnergySink, SessionRecorder, MAX_RECORDING_MS};

/// Kept for docs/tests that mention the M2 placeholder string.
#[allow(dead_code)]
pub const FAKE_TRANSCRIPT: &str = "落字测试";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SessionIntent {
    #[default]
    Continue,
    VoiceEdit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum SessionInputSource {
    #[default]
    Direct,
    Menu(SessionIntent),
    Shortcut(SessionIntent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShortcutStartPlan {
    intent: SessionIntent,
    source: SessionInputSource,
}

fn shortcut_start_plan(intent: SessionIntent) -> ShortcutStartPlan {
    ShortcutStartPlan {
        intent,
        source: SessionInputSource::Shortcut(intent),
    }
}

#[derive(Default)]
struct SessionIntentSlot {
    owned: Option<(u64, SessionIntent, SessionInputSource)>,
}

impl SessionIntentSlot {
    fn bind(&mut self, session_id: u64, intent: SessionIntent, source: SessionInputSource) {
        self.owned = Some((session_id, intent, source));
    }

    fn get(&self, session_id: u64) -> Option<SessionIntent> {
        self.owned
            .filter(|(owner_session_id, _, _)| *owner_session_id == session_id)
            .map(|(_, intent, _)| intent)
    }

    fn source(&self, session_id: u64) -> Option<SessionInputSource> {
        self.owned
            .filter(|(owner_session_id, _, _)| *owner_session_id == session_id)
            .map(|(_, _, source)| source)
    }

    fn consume(&mut self, session_id: u64) -> Option<SessionIntent> {
        if self
            .owned
            .is_none_or(|(owner_session_id, _, _)| owner_session_id != session_id)
        {
            return None;
        }
        self.owned.take().map(|(_, intent, _)| intent)
    }

    fn clear(&mut self, session_id: u64) -> bool {
        self.consume(session_id).is_some()
    }
}

struct OwnedSessionResource<T> {
    resource: T,
    owner_session_id: Option<u64>,
}

impl<T> OwnedSessionResource<T> {
    fn new(resource: T) -> Self {
        Self {
            resource,
            owner_session_id: None,
        }
    }

    fn bind_owner(&mut self, session_id: u64) {
        self.owner_session_id = Some(session_id);
    }

    fn owner_session_id(&self) -> Option<u64> {
        self.owner_session_id
    }

    fn resource_mut(&mut self) -> &mut T {
        &mut self.resource
    }

    #[cfg(test)]
    fn resource(&self) -> &T {
        &self.resource
    }

    fn cleanup_if_owned(&mut self, session_id: u64, cleanup: impl FnOnce(&mut T)) -> bool {
        if self.owner_session_id != Some(session_id) {
            return false;
        }
        cleanup(&mut self.resource);
        self.owner_session_id = None;
        true
    }
}

#[derive(Default)]
struct ShortcutHoldSlot {
    continue_active: AtomicBool,
    voice_edit_active: AtomicBool,
}

impl ShortcutHoldSlot {
    fn slot(&self, intent: SessionIntent) -> &AtomicBool {
        match intent {
            SessionIntent::Continue => &self.continue_active,
            SessionIntent::VoiceEdit => &self.voice_edit_active,
        }
    }

    fn set(&self, intent: SessionIntent, active: bool) {
        self.slot(intent).store(active, Ordering::SeqCst);
    }

    fn is_active(&self, intent: SessionIntent) -> bool {
        self.slot(intent).load(Ordering::SeqCst)
    }

    fn any_active(&self) -> bool {
        self.continue_active.load(Ordering::SeqCst) || self.voice_edit_active.load(Ordering::SeqCst)
    }
}

/// Whisper + deliver must not hang the session forever (stuck busy overlay).
const TRANSCRIBE_WATCHDOG_MS: u64 = 45_000;

#[derive(Clone, Default)]
struct EnergySessionGate {
    state: Arc<Mutex<Option<(u64, bool)>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HudCommand {
    generation: u64,
    visible: bool,
}

#[derive(Default)]
struct HudLifecycleState {
    generation: u64,
    desired_visible: bool,
}

#[derive(Clone, Default)]
struct HudLifecycle {
    state: Arc<Mutex<HudLifecycleState>>,
}

impl HudLifecycle {
    fn request_visibility(&self, visible: bool) -> HudCommand {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.generation = state
            .generation
            .checked_add(1)
            .expect("HUD generation overflow");
        state.desired_visible = visible;
        HudCommand {
            generation: state.generation,
            visible,
        }
    }

    fn expire_transient(&self, generation: u64) -> Option<HudCommand> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.generation != generation || !state.desired_visible {
            return None;
        }
        state.desired_visible = false;
        Some(HudCommand {
            generation,
            visible: false,
        })
    }

    fn command_is_current(&self, command: HudCommand) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.generation == command.generation && state.desired_visible == command.visible
    }
}

impl EnergySessionGate {
    fn activate(&self, session_id: u64) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *state == Some((session_id, false)) {
            return false;
        }
        *state = Some((session_id, true));
        true
    }

    fn deactivate(&self, session_id: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.is_none_or(|(current_session_id, _)| current_session_id == session_id) {
            *state = Some((session_id, false));
        }
    }

    #[cfg(test)]
    fn deactivate_current(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((session_id, _)) = *state {
            *state = Some((session_id, false));
        }
    }
}

#[cfg(test)]
fn deactivate_energy_for_cancel(gate: &EnergySessionGate, recording_session_id: Option<u64>) {
    if let Some(session_id) = recording_session_id {
        gate.deactivate(session_id);
    } else {
        gate.deactivate_current();
    }
}

fn cleanup_session_resources_locked(
    state: &AppSessionState,
    _machine: &mut SessionMachine,
    session_id: u64,
) {
    prepare_cancel_state(state);
    state.energy_session.deactivate(session_id);
    if let Ok(mut recorder) = state.recorder.lock() {
        recorder.cleanup_if_owned(session_id, SessionRecorder::cancel);
    }
    clear_session_intent(state, session_id);
    let _ = state.source_target.lock().map(|mut target| *target = None);
    let _ = state.source_pid.lock().map(|mut pid| *pid = None);
}

fn cleanup_stale_recorder_after_start(state: &AppSessionState, session_id: u64) -> bool {
    let Ok(machine) = state.machine.lock() else {
        return false;
    };
    if machine.recording_session_id() == Some(session_id) {
        return false;
    }
    let Ok(mut recorder) = state.recorder.lock() else {
        return false;
    };
    recorder.cleanup_if_owned(session_id, SessionRecorder::cancel)
}

fn recording_session_is_current(state: &AppSessionState, session_id: u64) -> bool {
    state
        .machine
        .lock()
        .ok()
        .and_then(|machine| machine.recording_session_id())
        == Some(session_id)
}

fn session_intent_for(state: &AppSessionState, session_id: u64) -> Option<SessionIntent> {
    state.intent.lock().ok()?.get(session_id)
}

fn session_input_source_for(
    state: &AppSessionState,
    session_id: u64,
) -> Option<SessionInputSource> {
    state.intent.lock().ok()?.source(session_id)
}

fn consume_session_intent(state: &AppSessionState, session_id: u64) -> Option<SessionIntent> {
    state.intent.lock().ok()?.consume(session_id)
}

fn clear_session_intent(state: &AppSessionState, session_id: u64) -> bool {
    state
        .intent
        .lock()
        .is_ok_and(|mut intent| intent.clear(session_id))
}

fn cancel_session_if_current(state: &AppSessionState, session_id: u64) -> bool {
    let Ok(mut machine) = state.machine.lock() else {
        return false;
    };
    if machine.active_session_id() != Some(session_id) {
        return false;
    }
    if !matches!(
        machine.handle(SessionCommand::Cancel),
        SessionEffect::Canceled {
            session_id: canceled_session_id
        } if canceled_session_id == session_id
    ) {
        return false;
    }
    cleanup_session_resources_locked(state, &mut machine, session_id);
    true
}

fn commit_capture_if_current(state: &AppSessionState, session_id: u64, token: TargetToken) -> bool {
    let Ok(machine) = state.machine.lock() else {
        return false;
    };
    if machine.recording_session_id() != Some(session_id) {
        return false;
    }
    let Ok(mut target) = state.source_target.lock() else {
        return false;
    };
    let Ok(mut pid) = state.source_pid.lock() else {
        return false;
    };
    let process_id = token.process_id;
    *target = Some(token);
    if process_id != 0 {
        *pid = Some(process_id as i32);
    }
    true
}

fn commit_source_pid_if_current(
    state: &AppSessionState,
    session_id: u64,
    source_pid: Option<i32>,
) -> bool {
    let Ok(machine) = state.machine.lock() else {
        return false;
    };
    if machine.recording_session_id() != Some(session_id) {
        return false;
    }
    let Ok(mut pid) = state.source_pid.lock() else {
        return false;
    };
    *pid = source_pid;
    true
}

fn with_recording_session_if_current(
    state: &AppSessionState,
    session_id: u64,
    callback: impl FnOnce(),
) -> bool {
    let Ok(machine) = state.machine.lock() else {
        return false;
    };
    if machine.recording_session_id() != Some(session_id) {
        return false;
    }
    callback();
    drop(machine);
    true
}

fn forward_energy_if_active(
    gate: &EnergySessionGate,
    session_id: u64,
    level: f32,
    emit: impl FnOnce(f32),
) -> bool {
    let state = gate
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *state != Some((session_id, true)) {
        return false;
    }

    // Keep the gate locked across emit so deactivate returning means no event is in flight.
    emit(level);
    drop(state);
    true
}

fn try_queue_energy(sender: &SyncSender<f32>, level: f32) {
    let _ = sender.try_send(level);
}

fn energy_event_channel() -> (EnergySink, Receiver<f32>) {
    let (sender, receiver) = sync_channel(1);
    let sink: EnergySink = Arc::new(move |level| try_queue_energy(&sender, level));
    (sink, receiver)
}

fn spawn_energy_event_worker(
    session_id: u64,
    gate: EnergySessionGate,
    receiver: Receiver<f32>,
    emit: impl Fn(u64, f32) + Send + 'static,
) {
    std::thread::spawn(move || {
        while let Ok(level) = receiver.recv() {
            if !forward_energy_if_active(&gate, session_id, level, |level| {
                emit(session_id, level);
            }) {
                break;
            }
        }
    });
}

fn arm_escape(_app: &AppHandle) {
    // Escape is registered once at startup; keep it for the whole process life.
}

fn disarm_escape(_app: &AppHandle) {
    // no-op — see arm_escape
}

pub struct AppSessionState {
    pub machine: Mutex<SessionMachine>,
    recorder: Mutex<OwnedSessionResource<SessionRecorder>>,
    pub clipboard: Mutex<ClipboardGate>,
    pub source_target: Mutex<Option<TargetToken>>,
    /// Lazy-loaded Whisper context (M3).
    pub asr: Mutex<Option<AsrEngine>>,
    /// Last successful ASR use — idle unload after 5 minutes.
    pub asr_last_used: Mutex<Option<Instant>>,
    /// Actually registered continue-speaking binding (may differ from config provisional).
    pub registered_continue: Mutex<Option<String>>,
    /// Aggregate input-active marker retained for status/tests.
    /// Session decisions use the source-specific menu/shortcut ownership below.
    pub hold_active: AtomicBool,
    shortcut_holds: ShortcutHoldSlot,
    /// Menu toggle is active while a menu-started recording awaits its second click.
    menu_active: AtomicBool,
    /// Recording ownership marker retained until stop/cancel completes.
    menu_session_id: Mutex<Option<u64>>,
    /// Frontmost app pid when recording started (for restore + paste/type).
    pub source_pid: Mutex<Option<i32>>,
    /// Model download in flight (tray debounce).
    pub model_fetching: AtomicBool,
    /// Continue dictation vs voice-edit instruction (M7).
    intent: Mutex<SessionIntentSlot>,
    energy_session: EnergySessionGate,
    hud_lifecycle: HudLifecycle,
    #[allow(dead_code)]
    pub config: AppConfig,
}

impl Default for AppSessionState {
    fn default() -> Self {
        Self {
            machine: Mutex::new(SessionMachine::new()),
            recorder: Mutex::new(OwnedSessionResource::new(SessionRecorder::new())),
            clipboard: Mutex::new(ClipboardGate::default()),
            source_target: Mutex::new(None),
            asr: Mutex::new(None),
            asr_last_used: Mutex::new(None),
            registered_continue: Mutex::new(None),
            hold_active: AtomicBool::new(false),
            shortcut_holds: ShortcutHoldSlot::default(),
            menu_active: AtomicBool::new(false),
            menu_session_id: Mutex::new(None),
            source_pid: Mutex::new(None),
            model_fetching: AtomicBool::new(false),
            intent: Mutex::new(SessionIntentSlot::default()),
            energy_session: EnergySessionGate::default(),
            hud_lifecycle: HudLifecycle::default(),
            config: super::config_store::load(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuToggleDecision {
    Start { session_id: u64 },
    Finish { session_id: u64 },
    RejectedBusy,
}

fn refresh_hold_active(state: &AppSessionState) {
    state.hold_active.store(
        state.menu_active.load(Ordering::SeqCst) || state.shortcut_holds.any_active(),
        Ordering::SeqCst,
    );
}

fn claim_session_with_intent_locked(
    state: &AppSessionState,
    machine: &mut SessionMachine,
    start_intent: SessionIntent,
    input_source: SessionInputSource,
) -> Result<SessionEffect, String> {
    let effect = machine.handle(SessionCommand::Start);
    if let SessionEffect::BeganRecording { session_id } = &effect {
        let Ok(mut intent) = state.intent.lock() else {
            let _ = machine.handle(SessionCommand::Cancel);
            return Err("intent_lock_failed".into());
        };
        intent.bind(*session_id, start_intent, input_source);
    }
    Ok(effect)
}

#[cfg(test)]
fn claim_session_with_intent(
    state: &AppSessionState,
    start_intent: SessionIntent,
) -> Result<SessionEffect, String> {
    let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
    claim_session_with_intent_locked(
        state,
        &mut machine,
        start_intent,
        SessionInputSource::Direct,
    )
}

#[cfg(test)]
fn claim_shortcut_session_with_intent(
    state: &AppSessionState,
    start_intent: SessionIntent,
) -> Result<SessionEffect, String> {
    let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
    claim_session_with_intent_locked(
        state,
        &mut machine,
        start_intent,
        SessionInputSource::Shortcut(start_intent),
    )
}

fn decide_menu_toggle(state: &AppSessionState, start_intent: SessionIntent) -> MenuToggleDecision {
    let Ok(mut machine) = state.machine.lock() else {
        return MenuToggleDecision::RejectedBusy;
    };

    match machine.phase() {
        SessionPhase::Idle => {
            let Ok(SessionEffect::BeganRecording { session_id }) = claim_session_with_intent_locked(
                state,
                &mut machine,
                start_intent,
                SessionInputSource::Menu(start_intent),
            ) else {
                return MenuToggleDecision::RejectedBusy;
            };
            let Ok(mut menu_session_id) = state.menu_session_id.lock() else {
                let _ = machine.handle(SessionCommand::Cancel);
                clear_session_intent(state, session_id);
                return MenuToggleDecision::RejectedBusy;
            };
            *menu_session_id = Some(session_id);
            state.menu_active.store(true, Ordering::SeqCst);
            refresh_hold_active(state);
            MenuToggleDecision::Start { session_id }
        }
        SessionPhase::Recording { session_id } => {
            let session_id = *session_id;
            if session_input_source_for(state, session_id)
                != Some(SessionInputSource::Menu(start_intent))
                || !state.menu_active.load(Ordering::SeqCst)
                || state
                    .menu_session_id
                    .lock()
                    .map_or(true, |owner| *owner != Some(session_id))
            {
                return MenuToggleDecision::RejectedBusy;
            }
            state.menu_active.store(false, Ordering::SeqCst);
            refresh_hold_active(state);
            MenuToggleDecision::Finish { session_id }
        }
        SessionPhase::Transcribing { .. } | SessionPhase::Delivering { .. } => {
            MenuToggleDecision::RejectedBusy
        }
    }
}

fn prepare_cancel_state(state: &AppSessionState) {
    state.shortcut_holds.set(SessionIntent::Continue, false);
    state.shortcut_holds.set(SessionIntent::VoiceEdit, false);
    state.menu_active.store(false, Ordering::SeqCst);
    refresh_hold_active(state);
    let _ = state
        .menu_session_id
        .lock()
        .map(|mut session_id| *session_id = None);
}

fn is_menu_session(state: &AppSessionState, session_id: u64) -> bool {
    state
        .menu_session_id
        .lock()
        .ok()
        .is_some_and(|current| *current == Some(session_id))
}

fn session_input_is_active(state: &AppSessionState, session_id: u64) -> bool {
    match session_input_source_for(state, session_id) {
        Some(SessionInputSource::Menu(_)) => {
            state.menu_active.load(Ordering::SeqCst) && is_menu_session(state, session_id)
        }
        Some(SessionInputSource::Shortcut(intent)) => state.shortcut_holds.is_active(intent),
        Some(SessionInputSource::Direct) => state.hold_active.load(Ordering::SeqCst),
        None => false,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HudPhasePayload<'a> {
    phase: &'a str,
    message: &'a str,
    session_id: Option<u64>,
}

fn hud_phase_payload<'a>(
    phase: &'a str,
    message: &'a str,
    session_id: Option<u64>,
) -> HudPhasePayload<'a> {
    HudPhasePayload {
        phase,
        message,
        session_id,
    }
}

fn emit_phase(
    app: &AppHandle,
    session_id: Option<u64>,
    phase: &str,
    message: &str,
) -> Option<HudCommand> {
    let command = show_overlay(app, true);
    let _ = app.emit(
        "session://phase",
        hud_phase_payload(phase, message, session_id),
    );
    command
}

fn transient_duration_ms(phase: &str) -> u64 {
    match phase {
        "inserted" | "undone" => 600,
        "canceled" | "too_short" => 1_600,
        "clipboard" => 3_500,
        "error" | "rejected" | "discarded" | "warn" => 5_000,
        _ => 2_200,
    }
}

fn undo_feedback() -> (&'static str, &'static str) {
    ("undone", "已撤销剪贴板落字")
}

/// Show overlay with a short-lived status, then hide (does not block).
fn emit_transient(app: &AppHandle, session_id: Option<u64>, phase: &str, message: &str) {
    let Some(command) = emit_phase(app, session_id, phase, message) else {
        return;
    };
    let duration_ms = transient_duration_ms(phase);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms));
        let Some(state) = app.try_state::<AppSessionState>() else {
            return;
        };
        let lifecycle = state.hud_lifecycle.clone();
        let hide = {
            let Ok(machine) = state.machine.lock() else {
                return;
            };
            if !machine.is_idle() {
                return;
            }
            lifecycle.expire_transient(command.generation)
        };
        if let Some(hide) = hide {
            apply_overlay_command(&app, lifecycle, hide);
        }
    });
}

fn registered_shortcut_label(state: &AppSessionState) -> String {
    state
        .registered_continue
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .unwrap_or_else(|| super::config_store::load().continue_speaking_shortcut)
}

fn recording_phase_copy(
    state: &AppSessionState,
    session_id: u64,
    editing: bool,
) -> (&'static str, String) {
    let phase = if editing {
        "recording_edit"
    } else {
        "recording"
    };
    if is_menu_session(state, session_id) {
        return (phase, "再次点击菜单完成 · Esc 取消".into());
    }
    if editing {
        return (phase, "说修改要求… · Esc 取消".into());
    }
    let shortcut = registered_shortcut_label(state);
    (phase, format!("松开 {shortcut} 开始整理 · Esc 取消"))
}

fn emit_recording_phase_if_current(
    app: &AppHandle,
    state: &AppSessionState,
    session_id: u64,
    editing: bool,
) -> bool {
    with_recording_session_if_current(state, session_id, || {
        let (phase, message) = recording_phase_copy(state, session_id, editing);
        emit_phase(app, Some(session_id), phase, &message);
    })
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
        emit_transient(app, None, "warn", "模型正在下载中…");
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let Some(state) = app.try_state::<AppSessionState>() else {
            return;
        };
        emit_transient(&app, None, "transcribing", "正在下载推荐模型…");
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
                emit_transient(&app, None, "inserted", "模型已就绪，可按住说话");
            }
            Err(err) => {
                eprintln!("luozi: model fetch failed: {err}");
                emit_transient(&app, None, "error", &err);
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
        if cancel_session_if_current(&state, session_id) {
            eprintln!("luozi: watchdog force-cancel stuck transcribe/deliver {session_id}");
            disarm_escape(&app);
            emit_transient(&app, Some(session_id), "error", "落字超时，已取消，请重试");
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
        let pcm =
            asr::resample_to_16k_mono(&capture.samples, capture.sample_rate, capture.channels);
        if pcm.is_empty() {
            let _ = fail_transcribe(&app, &state, session_id, "no_speech".into());
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
                let _ = fail_transcribe(&app, &state, session_id, err);
                return;
            }
        };

        let Some(intent) = consume_session_intent(&state, session_id) else {
            eprintln!("luozi: transcript intent stale for session {session_id}");
            hide_overlay_if_idle(&app, &state);
            return;
        };

        if intent == SessionIntent::VoiceEdit {
            // Consume session machine so we leave busy state, then apply text AI to draft.
            let edit_effect = {
                let Ok(mut machine) = state.machine.lock() else {
                    return;
                };
                machine.handle(SessionCommand::TranscriptionReady {
                    session_id,
                    text: transcript.clone(),
                })
            };
            match edit_effect {
                SessionEffect::Deliver {
                    session_id: accepted_session_id,
                    ..
                } if accepted_session_id == session_id => {}
                SessionEffect::StaleIgnored { .. } => {
                    eprintln!("luozi: voice edit transcript stale");
                    hide_overlay_if_idle(&app, &state);
                    return;
                }
                other => {
                    let _ = fail_transcribe(
                        &app,
                        &state,
                        session_id,
                        format!("unexpected_voice_edit_effect: {other:?}"),
                    );
                    return;
                }
            }
            // Force idle regardless of delivery effect.
            let _ = state.machine.lock().map(|mut m| {
                m.handle(SessionCommand::DeliveryFinished {
                    session_id,
                    ok: true,
                })
            });
            disarm_escape(&app);
            apply_voice_edit(&app, &state, &cfg, session_id, &transcript);
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
                let ok = apply_delivery(&app, &state, session_id, &text);
                let _ = state.machine.lock().map(|mut m| {
                    m.handle(SessionCommand::DeliveryFinished {
                        session_id,
                        ok: ok.machine_ok(),
                    })
                });
            }
            SessionEffect::StaleIgnored { .. } => {
                eprintln!("luozi: transcript stale (canceled during ASR)");
                disarm_escape(&app);
                hide_overlay_if_idle(&app, &state);
            }
            other => {
                eprintln!("luozi: unexpected_transcribe_effect: {other:?}");
                let _ = fail_transcribe(
                    &app,
                    &state,
                    session_id,
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
    match on_main_thread_timeout(
        app,
        MAIN_THREAD_AX_TIMEOUT,
        move || match spike::capture_target() {
            Ok(t) => Ok(t),
            Err(err) => Err(err),
        },
    ) {
        Ok(Ok(t)) if t.is_secure => {
            // Caller decides whether to cancel / reject; do not flash overlay here.
            t
        }
        Ok(Ok(t)) => {
            eprintln!("luozi: capture ok pid={} role={}", t.process_id, t.role);
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

fn apply_overlay_command(app: &AppHandle, lifecycle: HudLifecycle, command: HudCommand) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if !lifecycle.command_is_current(command) {
            return;
        }
        if let Some(window) = app.get_webview_window("overlay") {
            let _ = window.set_ignore_cursor_events(true);
            let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
            if command.visible {
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    });
}

fn show_overlay(app: &AppHandle, visible: bool) -> Option<HudCommand> {
    let state = app.try_state::<AppSessionState>()?;
    let lifecycle = state.hud_lifecycle.clone();
    let command = lifecycle.request_visibility(visible);
    apply_overlay_command(app, lifecycle, command);
    Some(command)
}

fn hide_overlay_if_idle(app: &AppHandle, state: &AppSessionState) {
    let Ok(machine) = state.machine.lock() else {
        return;
    };
    if machine.is_idle() {
        show_overlay(app, false);
    }
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

fn apply_voice_edit(
    app: &AppHandle,
    _state: &AppSessionState,
    cfg: &AppConfig,
    session_id: u64,
    instruction: &str,
) {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        emit_transient(app, Some(session_id), "error", "没听清修改要求");
        return;
    }

    let Some(draft) = app.try_state::<super::DraftStore>() else {
        emit_transient(app, Some(session_id), "error", "草稿不可用");
        return;
    };

    if draft.is_empty() {
        emit_transient(app, Some(session_id), "error", "先说一段或粘贴文字");
        return;
    }

    if !super::text_ai::text_ai_ready(&cfg.text_ai) {
        emit_transient(app, Some(session_id), "error", "请先配置并同意文本 AI");
        return;
    }

    let text = draft.text_snapshot();
    let (sel_start, sel_end) = draft.selection();
    let scope = match resolve_edit_scope(&text, sel_start, sel_end, instruction) {
        Ok(s) => s,
        Err(err) => {
            emit_transient(app, Some(session_id), "error", map_text_ai_err(&err));
            return;
        }
    };
    let original = text[scope.start..scope.end].to_string();
    if original.trim().is_empty() {
        emit_transient(app, Some(session_id), "error", "没有可修改的内容");
        return;
    }

    emit_phase(app, Some(session_id), "transcribing", "正在修改…");

    let proposed = match super::text_ai::rewrite_scope(&cfg.text_ai, instruction, &original) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("luozi: text_ai failed: {err}");
            emit_transient(app, Some(session_id), "error", map_text_ai_err(&err));
            return;
        }
    };

    if proposed == original {
        emit_transient(app, Some(session_id), "inserted", "无需修改");
        return;
    }

    match assess_edit_risk(&scope, &original, &proposed, instruction) {
        EditRisk::Low => match draft.insert_at(scope.start, scope.end, &proposed) {
            Ok(_) => {
                let _ = app.emit(
                    "draft://updated",
                    serde_json::json!({ "reason": "voice_edit" }),
                );
                emit_transient(app, Some(session_id), "inserted", "已修改");
            }
            Err(err) => {
                eprintln!("luozi: apply edit failed: {err}");
                emit_transient(app, Some(session_id), "error", "写入草稿失败");
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
            emit_transient(
                app,
                Some(session_id),
                "confirm",
                "高风险修改：请在草稿窗确认",
            );
        }
    }
}

fn prepare_voice_edit(app: &AppHandle) -> Result<(), String> {
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
            emit_transient(app, None, "error", "先说一段或粘贴文字");
            return Err("draft_empty".into());
        }
    }
    Ok(())
}

/// Open draft, ensure content, then start hold-to-talk as a voice-edit session.
pub fn start_voice_edit_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    start_shortcut_session(app, state, SessionIntent::VoiceEdit)
}

pub fn start_continue_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    start_shortcut_session(app, state, SessionIntent::Continue)
}

fn run_menu_toggle(
    app: &AppHandle,
    state: &AppSessionState,
    decision: MenuToggleDecision,
) -> Result<SessionStatus, String> {
    match decision {
        MenuToggleDecision::Start { session_id } => {
            start_claimed_session(app, state, dummy_token(), session_id)
        }
        MenuToggleDecision::Finish { session_id } => {
            stop_session_if_current(app, state, Some(session_id))
        }
        MenuToggleDecision::RejectedBusy => {
            eprintln!("luozi: menu toggle ignored (session busy)");
            Err("session_busy".into())
        }
    }
}

pub fn toggle_continue_menu_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    run_menu_toggle(
        app,
        state,
        decide_menu_toggle(state, SessionIntent::Continue),
    )
}

pub fn toggle_voice_edit_menu_session(
    app: &AppHandle,
    state: &AppSessionState,
) -> Result<SessionStatus, String> {
    let phase = state
        .machine
        .lock()
        .map_err(|_| "session_lock_failed")?
        .phase()
        .clone();
    if matches!(phase, SessionPhase::Idle) {
        prepare_voice_edit(app)?;
    }
    run_menu_toggle(
        app,
        state,
        decide_menu_toggle(state, SessionIntent::VoiceEdit),
    )
}

fn apply_delivery(
    app: &AppHandle,
    state: &AppSessionState,
    session_id: u64,
    text: &str,
) -> DeliveryResult {
    // M6: when the draft workbench is open, write into the draft instead of external apps.
    if draft_window_is_front(app) {
        if let Some(draft) = app.try_state::<super::DraftStore>() {
            match draft.append_transcript(text) {
                Ok(()) => {
                    let _ = app.emit(
                        "draft://updated",
                        serde_json::json!({ "reason": "dictation" }),
                    );
                    emit_transient(app, Some(session_id), "inserted", "已写入草稿");
                    eprintln!("luozi: delivered into draft workbench");
                    return DeliveryResult::Inserted;
                }
                Err(err) => {
                    eprintln!("luozi: draft append failed: {err}");
                    emit_transient(app, Some(session_id), "error", "草稿写入失败");
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
            emit_transient(app, Some(session_id), "inserted", "已落字");
            DeliveryResult::Inserted
        }
        Ok(DeliverOutcome::Discard) => {
            eprintln!("luozi: delivered discard");
            emit_transient(
                app,
                Some(session_id),
                "discarded",
                "安全输入：已丢弃，未写入",
            );
            DeliveryResult::Inserted
        }
        Ok(DeliverOutcome::Clipboard) | Ok(DeliverOutcome::Error(_)) | Err(_) => {
            // Electron / unverified AX: clipboard + ⌘V, then VERIFY before saying 已落字.
            // CGEvent "Ok" only means events were posted — never treat as success alone.
            restore_source_app(state);
            let _ = spike::type_text_via_cg_events(text);
            std::thread::sleep(std::time::Duration::from_millis(80));
            if spike::focused_field_contains(text) {
                eprintln!("luozi: verified after CG type → {text}");
                emit_transient(app, Some(session_id), "inserted", "已落字");
                DeliveryResult::Inserted
            } else {
                write_clipboard_with_paste(app, state, session_id, text, trusted)
            }
        }
    };

    let _ = state.source_target.lock().map(|mut g| *g = None);
    result
}

fn write_clipboard_with_paste(
    app: &AppHandle,
    state: &AppSessionState,
    session_id: u64,
    text: &str,
    trusted: bool,
) -> DeliveryResult {
    let write_result = match state.clipboard.lock() {
        Ok(mut gate) => gate.write_with_undo(text),
        Err(_) => Err("clipboard_lock_failed".into()),
    };
    if let Err(err) = write_result {
        emit_transient(app, Some(session_id), "error", &err);
        return DeliveryResult::Failed;
    }

    restore_source_app(state);
    let pasted = spike::paste_via_cmd_v().is_ok();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let verified = spike::focused_field_contains(text);
    eprintln!("luozi: clipboard written pasted={pasted} trusted={trusted} verified={verified}");
    if verified {
        emit_transient(app, Some(session_id), "inserted", "已落字");
        DeliveryResult::Inserted
    } else if !trusted {
        emit_transient(
            app,
            Some(session_id),
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
            Some(session_id),
            "clipboard",
            "未进输入框，已到剪贴板 · 请 ⌘V",
        );
        DeliveryResult::Clipboard
    }
}

pub fn wait_until_recording_session(
    state: &AppSessionState,
    session_id: u64,
    timeout_ms: u64,
) -> bool {
    let steps = (timeout_ms / 50).max(1);
    for _ in 0..steps {
        if recording_session_is_current(state, session_id) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

fn fail_transcribe(
    app: &AppHandle,
    state: &AppSessionState,
    session_id: u64,
    err: String,
) -> Result<SessionStatus, String> {
    let canceled = cancel_session_if_current(state, session_id);
    if !canceled {
        eprintln!("luozi: transcribe failure stale for session {session_id}: {err}");
        return Err(err);
    }
    disarm_escape(app);
    emit_transient(app, Some(session_id), "error", &user_facing_asr_error(&err));
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
    if err.contains("cloud_provider_unsupported") {
        return "当前厂商无云端 ASR · 请用本地或换厂商".into();
    }
    if err.contains("cloud_not_ready") || err.contains("no_engine") {
        return "无可用引擎 · 配置本地或云端 ASR".into();
    }
    if err.contains("no_speech") {
        return "未检测到语音".into();
    }
    err.to_string()
}

/// Set ASR mode by id (`auto` / `localOnly` / `cloudOnly`) and persist.
pub fn set_asr_mode(app: &AppHandle, mode: &str) -> Result<AsrMode, String> {
    let next = luozi_core::asr_mode_from_str(mode).ok_or_else(|| "invalid_asr_mode".to_string())?;
    let cfg = super::config_store::update(|c| {
        c.asr_mode = next;
    })?;
    emit_transient(
        app,
        None,
        "inserted",
        &format!("引擎模式：{}", cfg.asr_mode.label_zh()),
    );
    Ok(cfg.asr_mode)
}

/// Cycle ASR mode Auto → LocalOnly → CloudOnly and persist.
pub fn cycle_asr_mode(app: &AppHandle) -> Result<AsrMode, String> {
    let cfg = super::config_store::update(|c| {
        c.asr_mode = c.asr_mode.cycle();
    })?;
    emit_transient(
        app,
        None,
        "inserted",
        &format!("引擎模式：{}", cfg.asr_mode.label_zh()),
    );
    Ok(cfg.asr_mode)
}

/// Switch cloud ASR preset (Key/consent stay per credential_ref / provider_id).
pub fn set_cloud_asr_provider(app: &AppHandle, provider_id: &str) -> Result<(), String> {
    let preset =
        luozi_core::asr_preset(provider_id).ok_or_else(|| "unknown_asr_provider".to_string())?;
    let next = luozi_core::cloud_asr_from_preset(preset);
    let _ = super::config_store::update(|c| {
        c.cloud_asr = next;
    })?;
    emit_transient(
        app,
        None,
        "inserted",
        &format!("云端 ASR：{}", preset.label_zh),
    );
    Ok(())
}

/// Switch text AI preset.
pub fn set_text_ai_provider(app: &AppHandle, provider_id: &str) -> Result<(), String> {
    let preset = luozi_core::text_ai_preset(provider_id)
        .ok_or_else(|| "unknown_text_ai_provider".to_string())?;
    let next = luozi_core::text_ai_from_preset(preset);
    let _ = super::config_store::update(|c| {
        c.text_ai = next;
    })?;
    // Doubao needs a real Endpoint ID — prompt once if still placeholder.
    if provider_id == "textai.doubao" {
        if let Err(e) = prompt_text_ai_model(app) {
            if e != "canceled" {
                return Err(e);
            }
        }
    }
    emit_transient(
        app,
        None,
        "inserted",
        &format!("文本 AI：{}", preset.label_zh),
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn osascript_prompt(message: &str) -> Result<String, String> {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"set answer to display dialog "{escaped}" default answer "" with hidden answer buttons {{"取消", "保存"}} default button "保存"
        if button returned of answer is "取消" then return ""
        return text returned of answer"#
    );
    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("osascript_failed: {e}"))?;
    if !out.status.success() {
        return Err("canceled".into());
    }
    let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if key.is_empty() {
        return Err("canceled".into());
    }
    Ok(key)
}

#[cfg(target_os = "macos")]
fn osascript_prompt_visible(message: &str, default: &str) -> Result<String, String> {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    let def = default.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"set answer to display dialog "{escaped}" default answer "{def}" buttons {{"取消", "保存"}} default button "保存"
        if button returned of answer is "取消" then return ""
        return text returned of answer"#
    );
    let out = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("osascript_failed: {e}"))?;
    if !out.status.success() {
        return Err("canceled".into());
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() {
        return Err("canceled".into());
    }
    Ok(value)
}

pub fn prompt_text_ai_model(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cfg = super::config_store::load();
        let current = cfg.text_ai.model.clone();
        let value = osascript_prompt_visible(
            "文本 AI 模型名（豆包请填方舟 Endpoint ID，形如 ep-…）",
            &current,
        )?;
        let _ = super::config_store::update(|c| {
            c.text_ai.model = value;
        })?;
        emit_transient(app, None, "inserted", "文本 AI 模型已更新");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("unsupported".into())
    }
}

/// Record consent for current cloud preset host.
pub fn consent_current_cloud(app: &AppHandle) -> Result<(), String> {
    let cfg = super::config_store::load();
    if !cfg.cloud_asr.protocol.supports_cloud() {
        return Err("cloud_provider_unsupported".into());
    }
    let host = cfg
        .cloud_asr
        .host()
        .ok_or_else(|| "cloud_protocol_error".to_string())?;
    super::consent::grant(&cfg.cloud_asr.provider_id, &host)?;
    emit_transient(app, None, "inserted", &format!("已同意上传到 {host}"));
    Ok(())
}

/// Prompt for current ASR provider API key (macOS) and store in Keychain.
pub fn prompt_and_store_asr_key(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cfg = super::config_store::load();
        if !cfg.cloud_asr.protocol.supports_cloud() {
            return Err("cloud_provider_unsupported".into());
        }
        let prompt = luozi_core::asr_preset(&cfg.cloud_asr.provider_id)
            .map(|p| p.key_prompt_zh)
            .unwrap_or("粘贴云端 ASR API Key（仅存本机钥匙串）");
        let key = osascript_prompt(prompt)?;
        super::credentials::set_secret(&cfg.cloud_asr.credential_ref, &key)?;
        emit_transient(app, None, "inserted", "ASR Key 已写入钥匙串");
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
        let cfg = super::config_store::load();
        let prompt = luozi_core::text_ai_preset(&cfg.text_ai.provider_id)
            .map(|p| p.key_prompt_zh)
            .unwrap_or("粘贴文本 AI API Key（仅存本机钥匙串）");
        let key = osascript_prompt(prompt)?;
        super::credentials::set_secret(&cfg.text_ai.credential_ref, &key)?;
        emit_transient(app, None, "inserted", "文本 AI Key 已写入钥匙串");
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
    emit_transient(
        app,
        None,
        "inserted",
        &format!("已同意文本 AI 上传到 {host}"),
    );
    Ok(())
}

pub fn note_hold_pressed(state: &AppSessionState, intent: SessionIntent) {
    state.shortcut_holds.set(intent, true);
    refresh_hold_active(state);
}

pub fn note_hold_released(state: &AppSessionState, intent: SessionIntent) -> Option<u64> {
    state.shortcut_holds.set(intent, false);
    refresh_hold_active(state);
    let session_id = state
        .machine
        .lock()
        .ok()
        .and_then(|machine| machine.recording_session_id())?;
    if session_input_source_for(state, session_id) != Some(SessionInputSource::Shortcut(intent)) {
        return None;
    }
    Some(session_id)
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
        let energy_sink: EnergySink = Arc::new(|_| {});
        match rec.start(energy_sink) {
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
    start_session_with_token(app, state, dummy_token())
}

pub fn start_shortcut_session(
    app: &AppHandle,
    state: &AppSessionState,
    intent: SessionIntent,
) -> Result<SessionStatus, String> {
    if intent == SessionIntent::VoiceEdit {
        prepare_voice_edit(app)?;
    }
    let plan = shortcut_start_plan(intent);
    // HARD RULE: never wait on Accessibility before opening the mic.
    // Capture runs in the background; deliver path falls back to clipboard+⌘V.
    start_session_with_token_claim(app, state, dummy_token(), None, plan.intent, plan.source)
}

pub fn start_session_with_token(
    app: &AppHandle,
    state: &AppSessionState,
    token: TargetToken,
) -> Result<SessionStatus, String> {
    start_session_with_token_claim(
        app,
        state,
        token,
        None,
        SessionIntent::Continue,
        SessionInputSource::Direct,
    )
}

fn start_claimed_session(
    app: &AppHandle,
    state: &AppSessionState,
    token: TargetToken,
    session_id: u64,
) -> Result<SessionStatus, String> {
    start_session_with_token_claim(
        app,
        state,
        token,
        Some(session_id),
        SessionIntent::Continue,
        SessionInputSource::Menu(SessionIntent::Continue),
    )
}

fn start_session_with_token_claim(
    app: &AppHandle,
    state: &AppSessionState,
    token: TargetToken,
    claimed_session_id: Option<u64>,
    start_intent: SessionIntent,
    input_source: SessionInputSource,
) -> Result<SessionStatus, String> {
    if token.is_secure {
        if let Some(session_id) = claimed_session_id {
            let _ = cancel_session_if_current(state, session_id);
        }
        emit_transient(app, None, "rejected", "安全输入区域：未开始录音");
        return Err("secure_input_rejected".into());
    }

    let effect = if let Some(session_id) = claimed_session_id {
        let machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        if machine.recording_session_id() != Some(session_id) {
            drop(machine);
            return status_from(state, "menu_session_stale".into());
        }
        SessionEffect::BeganRecording { session_id }
    } else {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        claim_session_with_intent_locked(state, &mut machine, start_intent, input_source)?
    };

    match effect {
        SessionEffect::BeganRecording { session_id } => {
            let started_from_menu = is_menu_session(state, session_id);
            // Remember where the user was typing BEFORE mic TCC / overlay.
            if !commit_source_pid_if_current(state, session_id, spike::current_frontmost_pid()) {
                let _ = cancel_session_if_current(state, session_id);
                return status_from(state, "canceled_before_source_pid".into());
            }

            let (energy_sink, energy_receiver) = energy_event_channel();

            // Mic open can block on first-run TCC. Source-specific input state remembers
            // a shortcut release or second menu click that happened during that dialog.
            let start_result = match state.recorder.lock() {
                Ok(mut recorder) => {
                    let result = recorder.resource_mut().start(energy_sink);
                    if result.is_ok() {
                        recorder.bind_owner(session_id);
                    }
                    result
                }
                Err(_) => Err("recorder_lock_failed".to_string()),
            };
            if let Err(err) = start_result {
                if cancel_session_if_current(state, session_id) {
                    emit_transient(app, Some(session_id), "error", &err);
                }
                return Err(err);
            }

            if cleanup_stale_recorder_after_start(state, session_id)
                || !recording_session_is_current(state, session_id)
            {
                return status_from(state, "canceled_during_microphone_open".into());
            }

            if !session_input_is_active(state, session_id) {
                if started_from_menu {
                    eprintln!(
                        "luozi: menu finish requested during mic open — stop session {session_id}"
                    );
                    return stop_session_if_current(app, state, Some(session_id));
                }
                eprintln!(
                    "luozi: hold released during mic open (likely TCC) — cancel session {session_id}"
                );
                if cancel_session_if_current(state, session_id) {
                    disarm_escape(app);
                    emit_transient(
                        app,
                        Some(session_id),
                        "canceled",
                        "已授权麦克风。请再按住说话，松手落字",
                    );
                }
                return status_from(state, "canceled_after_permission".into());
            }

            if !state.energy_session.activate(session_id) {
                return status_from(state, "canceled_before_energy_worker".into());
            }
            let app_for_energy = app.clone();
            spawn_energy_event_worker(
                session_id,
                state.energy_session.clone(),
                energy_receiver,
                move |session_id, level| {
                    let _ = app_for_energy.emit(
                        "session://energy",
                        serde_json::json!({
                            "sessionId": session_id,
                            "level": level,
                        }),
                    );
                },
            );

            if !commit_capture_if_current(state, session_id, token) {
                let _ = cancel_session_if_current(state, session_id);
                return status_from(state, "canceled_before_target_commit".into());
            }
            arm_escape(app);
            let Some(intent) = session_intent_for(state, session_id) else {
                let _ = cancel_session_if_current(state, session_id);
                return status_from(state, "canceled_before_intent_read".into());
            };
            let editing = intent == SessionIntent::VoiceEdit;
            if !emit_recording_phase_if_current(app, state, session_id, editing) {
                return status_from(state, "canceled_before_recording_phase".into());
            }

            // Best-effort focus capture in background (never blocks start).
            let app_cap = app.clone();
            std::thread::spawn(move || {
                let t = capture_source_token(&app_cap);
                let Some(state) = app_cap.try_state::<AppSessionState>() else {
                    return;
                };
                if t.is_secure {
                    // Too late to reject cleanly mid-record; cancel instead.
                    if !cancel_session_if_current(&state, session_id) {
                        return;
                    }
                    disarm_escape(&app_cap);
                    emit_transient(
                        &app_cap,
                        Some(session_id),
                        "rejected",
                        "安全输入区域：已取消",
                    );
                    return;
                }
                if t.process_id != 0 && !commit_capture_if_current(&state, session_id, t.clone()) {
                    return;
                }
                // Keep HUD on recording copy even if capture was soft-fail.
                let _ = emit_recording_phase_if_current(&app_cap, &state, session_id, editing);
            });

            // Auto-stop at max duration.
            let app_handle = app.clone();
            let sid = session_id;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(MAX_RECORDING_MS));
                let Some(state) = app_handle.try_state::<AppSessionState>() else {
                    return;
                };
                let _ = stop_session_if_current(&app_handle, &state, Some(sid));
            });

            // Watchdog: if still Recording well past max, force-cancel (Esc-dead scenarios).
            let app_wd = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(MAX_RECORDING_MS + 5_000));
                let Some(state) = app_wd.try_state::<AppSessionState>() else {
                    return;
                };
                if cancel_session_if_current(&state, sid) {
                    eprintln!("luozi: watchdog force-cancel stuck recording {sid}");
                    disarm_escape(&app_wd);
                    emit_transient(&app_wd, Some(sid), "canceled", "已取消");
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
                    if !session_input_is_active(&state, sid) {
                        eprintln!("luozi: hold inactive while recording — auto stop {sid}");
                        let _ = stop_session_if_current(&app_hold, &state, Some(sid));
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

fn stop_session_if_current(
    app: &AppHandle,
    state: &AppSessionState,
    expected_session_id: Option<u64>,
) -> Result<SessionStatus, String> {
    let (audio, effect) = {
        let mut machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        let Some(session_id) = machine.recording_session_id() else {
            drop(machine);
            eprintln!("luozi: stop ignored (not recording)");
            return status_from(state, "stop_ignored".into());
        };
        if expected_session_id.is_some_and(|expected| expected != session_id) {
            drop(machine);
            eprintln!(
                "luozi: stop ignored (expected {:?}, active {session_id})",
                expected_session_id
            );
            return status_from(state, "stop_ignored".into());
        }

        state.energy_session.deactivate(session_id);
        let stop_result = match state.recorder.lock() {
            Ok(mut recorder) => {
                if recorder.owner_session_id() != Some(session_id) {
                    Err("not_recording".to_string())
                } else {
                    let _elapsed_ms = recorder.resource_mut().elapsed_ms();
                    let result = recorder.resource_mut().stop();
                    recorder.cleanup_if_owned(session_id, SessionRecorder::cancel);
                    result
                }
            }
            Err(_) => Err("recorder_lock_failed".to_string()),
        };
        let audio = match stop_result {
            Ok(audio) => audio,
            Err(err) => {
                let _ = machine.handle(SessionCommand::Cancel);
                cleanup_session_resources_locked(state, &mut machine, session_id);
                drop(machine);
                disarm_escape(app);
                if err == "not_recording" {
                    eprintln!("luozi: stop not_recording (session canceled)");
                    return status_from(state, "stop_ignored".into());
                }
                emit_transient(app, Some(session_id), "canceled", "已取消");
                return Err(err);
            }
        };

        prepare_cancel_state(state);
        let effect = machine.handle(SessionCommand::Stop {
            duration_ms: audio.duration_ms,
        });
        if matches!(effect, SessionEffect::RejectedTooShort { .. }) {
            cleanup_session_resources_locked(state, &mut machine, session_id);
        }
        (audio, effect)
    };

    match effect {
        SessionEffect::RejectedTooShort { session_id } => {
            eprintln!("luozi: session too short (<300ms)");
            disarm_escape(app);
            emit_transient(app, Some(session_id), "too_short", "时间太短，请按住再松手");
            status_from(state, "too_short".into())
        }
        SessionEffect::BeginTranscribe { session_id } => {
            emit_phase(app, Some(session_id), "transcribing", "落字中");
            let language = super::config_store::load().language;

            spawn_transcribe_watchdog(app.clone(), session_id);
            finish_transcribe_async(app.clone(), session_id, audio.duration_ms, audio, language);
            status_from(state, "transcribing".into())
        }
        SessionEffect::RejectedBusy => {
            eprintln!("luozi: stop ignored (session busy)");
            Err("session_busy".into())
        }
        other => Err(format!("unexpected_stop_effect: {other:?}")),
    }
}

pub fn stop_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    stop_session_if_current(app, state, None)
}

pub fn finish_shortcut_session(
    app: &AppHandle,
    state: &AppSessionState,
    intent: SessionIntent,
    session_id: u64,
) -> Result<SessionStatus, String> {
    if session_input_source_for(state, session_id) != Some(SessionInputSource::Shortcut(intent)) {
        return status_from(state, "stop_ignored".into());
    }
    stop_session_if_current(app, state, Some(session_id))
}

pub fn cancel_session(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    prepare_cancel_state(state);
    let session_id = state
        .machine
        .lock()
        .ok()
        .and_then(|machine| machine.active_session_id());
    let Some(session_id) = session_id else {
        hide_overlay_if_idle(app, state);
        return status_from(state, "idle".into());
    };
    if !cancel_session_if_current(state, session_id) {
        return status_from(state, "cancel_ignored".into());
    }
    disarm_escape(app);
    emit_transient(app, Some(session_id), "canceled", "已取消");
    status_from(state, "canceled".into())
}

pub fn undo_last(app: &AppHandle, state: &AppSessionState) -> Result<SessionStatus, String> {
    let mut gate = state
        .clipboard
        .lock()
        .map_err(|_| "clipboard_lock_failed")?;
    gate.undo_last()?;
    drop(gate);
    let (phase, message) = undo_feedback();
    emit_transient(app, None, phase, message);
    status_from(state, "undone".into())
}

fn status_from(state: &AppSessionState, message: String) -> Result<SessionStatus, String> {
    let (phase, session_id) = {
        let machine = state.machine.lock().map_err(|_| "session_lock_failed")?;
        (
            phase_name(machine.phase()).to_string(),
            machine.active_session_id(),
        )
    };
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
        phase,
        session_id,
        can_undo,
        fake_transcript: false,
        registered_continue,
        message,
    })
}

#[tauri::command]
pub fn session_start(
    app: AppHandle,
    state: State<'_, AppSessionState>,
) -> Result<SessionStatus, String> {
    start_session(&app, &state)
}

#[tauri::command]
pub fn session_stop(
    app: AppHandle,
    state: State<'_, AppSessionState>,
) -> Result<SessionStatus, String> {
    stop_session(&app, &state)
}

#[tauri::command]
pub fn session_cancel(
    app: AppHandle,
    state: State<'_, AppSessionState>,
) -> Result<SessionStatus, String> {
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

#[cfg(test)]
mod hud_tests {
    use super::{
        hud_phase_payload, transient_duration_ms, undo_feedback, HudCommand, HudLifecycle,
    };

    #[test]
    fn old_generation_cannot_hide_new_transient() {
        let lifecycle = HudLifecycle::default();
        let old_show = lifecycle.request_visibility(true);
        let new_show = lifecycle.request_visibility(true);

        assert!(lifecycle.expire_transient(old_show.generation).is_none());
        assert!(lifecycle.command_is_current(new_show));
    }

    #[test]
    fn stale_overlay_commands_do_not_match_current_desired_state() {
        let lifecycle = HudLifecycle::default();
        let show = lifecycle.request_visibility(true);
        let stale_hide = HudCommand {
            generation: show.generation,
            visible: false,
        };

        assert!(lifecycle.command_is_current(show));
        assert!(!lifecycle.command_is_current(stale_hide));

        let hide = lifecycle
            .expire_transient(show.generation)
            .expect("current transient should expire");
        assert!(!lifecycle.command_is_current(show));
        assert!(lifecycle.command_is_current(hide));

        let replacement_show = lifecycle.request_visibility(true);
        assert!(!lifecycle.command_is_current(hide));
        assert!(lifecycle.command_is_current(replacement_show));
    }

    #[test]
    fn explicit_hide_invalidates_pending_show_and_timer() {
        let lifecycle = HudLifecycle::default();
        let show = lifecycle.request_visibility(true);
        let hide = lifecycle.request_visibility(false);

        assert!(!lifecycle.command_is_current(show));
        assert!(lifecycle.command_is_current(hide));
        assert!(lifecycle.expire_transient(show.generation).is_none());
    }

    #[test]
    fn inserted_feedback_is_brief() {
        assert_eq!(transient_duration_ms("inserted"), 600);
    }

    #[test]
    fn undone_feedback_is_brief() {
        assert_eq!(transient_duration_ms("undone"), 600);
    }

    #[test]
    fn undo_uses_transient_feedback_phase() {
        assert_eq!(undo_feedback(), ("undone", "已撤销剪贴板落字"));
    }

    #[test]
    fn clipboard_feedback_remains_readable() {
        assert_eq!(transient_duration_ms("clipboard"), 3_500);
    }

    #[test]
    fn errors_remain_readable() {
        assert_eq!(transient_duration_ms("error"), 5_000);
        assert_eq!(transient_duration_ms("rejected"), 5_000);
    }

    #[test]
    fn phase_payload_serializes_explicit_session_id_in_camel_case() {
        let payload = hud_phase_payload("recording", "listening", Some(42));

        assert_eq!(
            serde_json::to_value(payload).expect("serialize HUD phase payload"),
            serde_json::json!({
                "phase": "recording",
                "message": "listening",
                "sessionId": 42,
            }),
        );
    }

    #[test]
    fn phase_payload_serializes_explicit_null_session_id() {
        let payload = hud_phase_payload("warn", "model", None);

        assert_eq!(
            serde_json::to_value(payload).expect("serialize HUD phase payload"),
            serde_json::json!({
                "phase": "warn",
                "message": "model",
                "sessionId": null,
            }),
        );
    }
}

#[cfg(test)]
mod session_resource_tests {
    use super::{
        cancel_session_if_current, claim_session_with_intent, cleanup_stale_recorder_after_start,
        clear_session_intent, commit_capture_if_current, consume_session_intent,
        decide_menu_toggle, dummy_token, session_intent_for, with_recording_session_if_current,
        AppSessionState, MenuToggleDecision, OwnedSessionResource, SessionCommand, SessionEffect,
        SessionIntent,
    };
    use std::sync::atomic::Ordering;

    #[derive(Default)]
    struct FakeRecorder {
        cancel_count: usize,
    }

    fn begin_session(state: &AppSessionState) -> u64 {
        let effect = state
            .machine
            .lock()
            .expect("machine lock")
            .handle(SessionCommand::Start);
        match effect {
            SessionEffect::BeganRecording { session_id } => session_id,
            other => panic!("expected recording, got {other:?}"),
        }
    }

    fn target(process_id: u32) -> super::TargetToken {
        let mut token = dummy_token();
        token.process_id = process_id;
        token
    }

    #[test]
    fn stale_session_cannot_cancel_new_session_resources() {
        let state = AppSessionState::default();
        let old_session_id = begin_session(&state);
        assert!(cancel_session_if_current(&state, old_session_id));

        let new_session_id = begin_session(&state);
        state.hold_active.store(true, Ordering::SeqCst);
        *state.source_target.lock().expect("target lock") = Some(target(202));
        *state.source_pid.lock().expect("pid lock") = Some(202);

        assert!(!cancel_session_if_current(&state, old_session_id));
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .recording_session_id(),
            Some(new_session_id),
        );
        assert!(state.hold_active.load(Ordering::SeqCst));
        assert_eq!(
            state
                .source_target
                .lock()
                .expect("target lock")
                .as_ref()
                .map(|token| token.process_id),
            Some(202),
        );
        assert_eq!(*state.source_pid.lock().expect("pid lock"), Some(202));
    }

    #[test]
    fn stale_capture_cannot_overwrite_new_session_target() {
        let state = AppSessionState::default();
        let old_session_id = begin_session(&state);
        assert!(cancel_session_if_current(&state, old_session_id));
        let new_session_id = begin_session(&state);
        assert!(commit_capture_if_current(
            &state,
            new_session_id,
            target(202),
        ));

        assert!(!commit_capture_if_current(
            &state,
            old_session_id,
            target(101),
        ));
        assert_eq!(
            state
                .source_target
                .lock()
                .expect("target lock")
                .as_ref()
                .map(|token| token.process_id),
            Some(202),
        );
        assert_eq!(*state.source_pid.lock().expect("pid lock"), Some(202));
    }

    #[test]
    fn current_capture_commits_target_and_pid() {
        let state = AppSessionState::default();
        let session_id = begin_session(&state);

        assert!(commit_capture_if_current(&state, session_id, target(303)));
        assert_eq!(
            state
                .source_target
                .lock()
                .expect("target lock")
                .as_ref()
                .map(|token| token.process_id),
            Some(303),
        );
        assert_eq!(*state.source_pid.lock().expect("pid lock"), Some(303));
    }

    #[test]
    fn stale_session_does_not_run_guarded_recording_callback() {
        let state = AppSessionState::default();
        let old_session_id = begin_session(&state);
        assert!(cancel_session_if_current(&state, old_session_id));
        let _new_session_id = begin_session(&state);
        let mut called = false;

        assert!(!with_recording_session_if_current(
            &state,
            old_session_id,
            || called = true,
        ));
        assert!(!called);
    }

    #[test]
    fn current_session_runs_guarded_recording_callback() {
        let state = AppSessionState::default();
        let session_id = begin_session(&state);
        let mut called = false;

        assert!(with_recording_session_if_current(
            &state,
            session_id,
            || called = true,
        ));
        assert!(called);
    }

    #[test]
    fn stale_old_owner_is_canceled_and_cleared() {
        let mut recorder = OwnedSessionResource::new(FakeRecorder::default());
        recorder.bind_owner(7);

        assert!(recorder.cleanup_if_owned(7, |recorder| {
            recorder.cancel_count += 1;
        }));
        assert_eq!(recorder.owner_session_id(), None);
        assert_eq!(recorder.resource().cancel_count, 1);
    }

    #[test]
    fn old_cleanup_cannot_cancel_new_owner() {
        let mut recorder = OwnedSessionResource::new(FakeRecorder::default());
        recorder.bind_owner(8);

        assert!(!recorder.cleanup_if_owned(7, |recorder| {
            recorder.cancel_count += 1;
        }));
        assert_eq!(recorder.owner_session_id(), Some(8));
        assert_eq!(recorder.resource().cancel_count, 0);
    }

    #[test]
    fn cancel_before_microphone_open_cleans_late_recorder_owner() {
        let state = AppSessionState::default();
        let session_id =
            match claim_session_with_intent(&state, SessionIntent::Continue).expect("claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert!(cancel_session_if_current(&state, session_id));

        state
            .recorder
            .lock()
            .expect("recorder lock")
            .bind_owner(session_id);

        assert!(cleanup_stale_recorder_after_start(&state, session_id));
        assert_eq!(
            state
                .recorder
                .lock()
                .expect("recorder lock")
                .owner_session_id(),
            None
        );
    }

    #[test]
    fn menu_finish_before_microphone_open_cleans_late_recorder_owner() {
        let state = AppSessionState::default();
        let MenuToggleDecision::Start { session_id } =
            decide_menu_toggle(&state, SessionIntent::Continue)
        else {
            panic!("menu should claim recording");
        };
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Finish { session_id }
        );

        assert!(cancel_session_if_current(&state, session_id));
        state
            .recorder
            .lock()
            .expect("recorder lock")
            .bind_owner(session_id);

        assert!(cleanup_stale_recorder_after_start(&state, session_id));
        assert_eq!(
            state
                .recorder
                .lock()
                .expect("recorder lock")
                .owner_session_id(),
            None
        );
    }

    #[test]
    fn busy_continue_does_not_overwrite_active_voice_edit_intent() {
        let state = AppSessionState::default();

        let session_id =
            match claim_session_with_intent(&state, SessionIntent::VoiceEdit).expect("claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert_eq!(
            claim_session_with_intent(&state, SessionIntent::Continue).expect("busy claim"),
            SessionEffect::RejectedBusy
        );
        assert_eq!(
            session_intent_for(&state, session_id),
            Some(SessionIntent::VoiceEdit)
        );
    }

    #[test]
    fn busy_voice_edit_does_not_overwrite_active_continue_intent() {
        let state = AppSessionState::default();

        let session_id =
            match claim_session_with_intent(&state, SessionIntent::Continue).expect("claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert_eq!(
            claim_session_with_intent(&state, SessionIntent::VoiceEdit).expect("busy claim"),
            SessionEffect::RejectedBusy
        );
        assert_eq!(
            session_intent_for(&state, session_id),
            Some(SessionIntent::Continue)
        );
    }

    #[test]
    fn canceled_session_late_consume_cannot_take_new_session_intent() {
        let state = AppSessionState::default();
        let old_session_id =
            match claim_session_with_intent(&state, SessionIntent::VoiceEdit).expect("old claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert!(cancel_session_if_current(&state, old_session_id));
        let new_session_id =
            match claim_session_with_intent(&state, SessionIntent::Continue).expect("new claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };

        assert_eq!(consume_session_intent(&state, old_session_id), None);
        assert_eq!(
            session_intent_for(&state, new_session_id),
            Some(SessionIntent::Continue)
        );
    }

    #[test]
    fn matching_session_consumes_intent_once() {
        let state = AppSessionState::default();
        let session_id =
            match claim_session_with_intent(&state, SessionIntent::VoiceEdit).expect("claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };

        assert_eq!(
            consume_session_intent(&state, session_id),
            Some(SessionIntent::VoiceEdit)
        );
        assert_eq!(consume_session_intent(&state, session_id), None);
    }

    #[test]
    fn cancel_clears_only_matching_session_intent() {
        let state = AppSessionState::default();
        let old_session_id =
            match claim_session_with_intent(&state, SessionIntent::VoiceEdit).expect("old claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert!(cancel_session_if_current(&state, old_session_id));
        assert_eq!(session_intent_for(&state, old_session_id), None);

        let new_session_id =
            match claim_session_with_intent(&state, SessionIntent::Continue).expect("new claim") {
                SessionEffect::BeganRecording { session_id } => session_id,
                other => panic!("expected recording, got {other:?}"),
            };
        assert!(!clear_session_intent(&state, old_session_id));
        assert_eq!(
            session_intent_for(&state, new_session_id),
            Some(SessionIntent::Continue)
        );
    }
}

#[cfg(test)]
mod menu_toggle_tests {
    use super::{
        claim_shortcut_session_with_intent, decide_menu_toggle, note_hold_pressed,
        note_hold_released, prepare_cancel_state, recording_phase_copy, session_input_is_active,
        session_intent_for, shortcut_start_plan, AppSessionState, MenuToggleDecision,
        SessionCommand, SessionEffect, SessionInputSource, SessionIntent,
    };
    use std::sync::atomic::Ordering;

    #[test]
    fn idle_menu_action_claims_recording_and_sets_menu_hold() {
        let state = AppSessionState::default();

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );
        assert!(state.hold_active.load(Ordering::SeqCst));
        assert!(state.menu_active.load(Ordering::SeqCst));
        assert_eq!(
            *state.menu_session_id.lock().expect("menu session lock"),
            Some(0)
        );
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .recording_session_id(),
            Some(0)
        );
    }

    #[test]
    fn recording_menu_action_finishes_current_session_and_clears_menu_hold() {
        let state = AppSessionState::default();
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Finish { session_id: 0 }
        );
        assert!(!state.hold_active.load(Ordering::SeqCst));
        assert!(!state.menu_active.load(Ordering::SeqCst));
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .recording_session_id(),
            Some(0),
            "finish decision leaves the owned recording for stop_session"
        );
    }

    #[test]
    fn cancel_clears_menu_hold_and_session_marker() {
        let state = AppSessionState::default();
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );

        prepare_cancel_state(&state);

        assert!(!state.hold_active.load(Ordering::SeqCst));
        assert!(!state.menu_active.load(Ordering::SeqCst));
        assert_eq!(
            *state.menu_session_id.lock().expect("menu session lock"),
            None
        );
    }

    #[test]
    fn busy_menu_action_does_not_start_a_second_session() {
        let state = AppSessionState::default();
        let session_id = {
            let mut machine = state.machine.lock().expect("machine lock");
            let SessionEffect::BeganRecording { session_id } =
                machine.handle(SessionCommand::Start)
            else {
                panic!("session should start");
            };
            assert!(matches!(
                machine.handle(SessionCommand::Stop { duration_ms: 500 }),
                SessionEffect::BeginTranscribe { .. }
            ));
            session_id
        };

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::RejectedBusy
        );
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .active_session_id(),
            Some(session_id)
        );
        assert!(!state.menu_active.load(Ordering::SeqCst));
    }

    #[test]
    fn menu_recording_copy_explains_toggle_completion() {
        let state = AppSessionState::default();
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );

        assert_eq!(
            recording_phase_copy(&state, 0, false),
            ("recording", "再次点击菜单完成 · Esc 取消".to_string())
        );
        assert_eq!(
            recording_phase_copy(&state, 0, true),
            ("recording_edit", "再次点击菜单完成 · Esc 取消".to_string())
        );
    }

    #[test]
    fn idle_voice_edit_claim_commits_intent_before_recording_is_visible() {
        let state = AppSessionState::default();

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::VoiceEdit),
            MenuToggleDecision::Start { session_id: 0 }
        );
        assert_eq!(
            session_intent_for(&state, 0),
            Some(SessionIntent::VoiceEdit)
        );
    }

    #[test]
    fn shortcut_release_cannot_clear_a_menu_owned_session() {
        let state = AppSessionState::default();
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );

        assert_eq!(note_hold_released(&state, SessionIntent::Continue), None);

        assert!(state.hold_active.load(Ordering::SeqCst));
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .recording_session_id(),
            Some(0)
        );
    }

    #[test]
    fn menu_toggle_cannot_finish_a_shortcut_owned_session() {
        let state = AppSessionState::default();
        note_hold_pressed(&state, SessionIntent::Continue);
        let session_id = match claim_shortcut_session_with_intent(&state, SessionIntent::Continue)
            .expect("claim")
        {
            SessionEffect::BeganRecording { session_id } => session_id,
            other => panic!("expected recording, got {other:?}"),
        };

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::RejectedBusy
        );
        assert!(state.hold_active.load(Ordering::SeqCst));
        assert_eq!(
            state
                .machine
                .lock()
                .expect("machine lock")
                .recording_session_id(),
            Some(session_id)
        );
    }

    #[test]
    fn continue_shortcut_release_cannot_clear_voice_edit_shortcut_hold() {
        let state = AppSessionState::default();
        note_hold_pressed(&state, SessionIntent::VoiceEdit);
        let session_id = match claim_shortcut_session_with_intent(&state, SessionIntent::VoiceEdit)
            .expect("claim")
        {
            SessionEffect::BeganRecording { session_id } => session_id,
            other => panic!("expected recording, got {other:?}"),
        };

        assert_eq!(note_hold_released(&state, SessionIntent::Continue), None);

        assert!(state.hold_active.load(Ordering::SeqCst));
        assert_eq!(
            session_intent_for(&state, session_id),
            Some(SessionIntent::VoiceEdit)
        );
        assert_eq!(
            note_hold_released(&state, SessionIntent::VoiceEdit),
            Some(session_id)
        );
        assert!(!state.hold_active.load(Ordering::SeqCst));
    }

    #[test]
    fn different_menu_intent_cannot_finish_the_active_menu_session() {
        let state = AppSessionState::default();
        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::Continue),
            MenuToggleDecision::Start { session_id: 0 }
        );

        assert_eq!(
            decide_menu_toggle(&state, SessionIntent::VoiceEdit),
            MenuToggleDecision::RejectedBusy
        );
        assert!(state.menu_active.load(Ordering::SeqCst));
        assert!(state.hold_active.load(Ordering::SeqCst));
    }

    #[test]
    fn release_before_shortcut_claim_is_not_masked_by_another_shortcut_hold() {
        let state = AppSessionState::default();
        note_hold_pressed(&state, SessionIntent::VoiceEdit);
        note_hold_pressed(&state, SessionIntent::Continue);

        assert_eq!(note_hold_released(&state, SessionIntent::Continue), None);
        assert!(
            state.hold_active.load(Ordering::SeqCst),
            "voice edit remains held"
        );

        let session_id = match claim_shortcut_session_with_intent(&state, SessionIntent::Continue)
            .expect("claim")
        {
            SessionEffect::BeganRecording { session_id } => session_id,
            other => panic!("expected recording, got {other:?}"),
        };
        assert!(!session_input_is_active(&state, session_id));
        assert_eq!(note_hold_released(&state, SessionIntent::VoiceEdit), None);
        assert!(!state.hold_active.load(Ordering::SeqCst));
    }

    #[test]
    fn continue_production_start_plan_uses_continue_shortcut_ownership() {
        assert_eq!(
            shortcut_start_plan(SessionIntent::Continue).source,
            SessionInputSource::Shortcut(SessionIntent::Continue)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        deactivate_energy_for_cancel, energy_event_channel, forward_energy_if_active,
        EnergySessionGate,
    };
    use std::sync::mpsc::TryRecvError;
    use std::sync::{Arc, Mutex};

    #[test]
    fn energy_event_channel_queues_until_worker_starts_and_drops_when_full() {
        let (sink, receiver) = energy_event_channel();

        sink(0.25);
        sink(0.75);

        assert_eq!(receiver.try_recv(), Ok(0.25));
        assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn energy_event_gate_drops_queued_and_new_levels_after_deactivation() {
        let gate = EnergySessionGate::default();
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let session_id = 7;
        gate.activate(session_id);

        let emitted_while_active = emitted.clone();
        assert!(forward_energy_if_active(
            &gate,
            session_id,
            0.25,
            move |level| emitted_while_active
                .lock()
                .expect("emitted lock")
                .push(level),
        ));

        gate.deactivate(session_id);
        for level in [0.5, 0.75] {
            let emitted_after_stop = emitted.clone();
            assert!(!forward_energy_if_active(
                &gate,
                session_id,
                level,
                move |level| emitted_after_stop.lock().expect("emitted lock").push(level),
            ));
        }

        assert_eq!(*emitted.lock().expect("emitted lock"), vec![0.25]);
    }

    #[test]
    fn energy_event_gate_does_not_reactivate_a_stopped_session() {
        let gate = EnergySessionGate::default();
        let session_id = 8;

        gate.deactivate(session_id);

        assert!(!gate.activate(session_id));
        assert!(!forward_energy_if_active(
            &gate,
            session_id,
            0.5,
            |_| panic!("stopped session must not emit"),
        ));
    }

    #[test]
    fn energy_event_cancel_uses_recording_session_id_to_tombstone_an_empty_gate() {
        let gate = EnergySessionGate::default();
        let session_id = 9;

        deactivate_energy_for_cancel(&gate, Some(session_id));

        assert!(!gate.activate(session_id));
    }
}
