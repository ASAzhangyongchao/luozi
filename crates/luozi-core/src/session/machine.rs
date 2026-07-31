//! Dictation session state machine (spec §5.7). Single active session only.

/// Monotonic session identity. Late results with a mismatched id are discarded.
pub type SessionId = u64;

/// Minimum hold duration before stop is treated as a real utterance (spec: 0.3s).
pub const MIN_RECORDING_MS: u64 = 300;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SessionPhase {
    #[default]
    Idle,
    Recording {
        session_id: SessionId,
    },
    Transcribing {
        session_id: SessionId,
    },
    Delivering {
        session_id: SessionId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionCommand {
    Start,
    /// Stop recording. `duration_ms` is wall time since start.
    Stop {
        duration_ms: u64,
    },
    Cancel,
    /// ASR (or M2 fake text) finished for a session.
    TranscriptionReady {
        session_id: SessionId,
        text: String,
    },
    /// Platform insert/clipboard finished.
    DeliveryFinished {
        session_id: SessionId,
        ok: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEffect {
    BeganRecording { session_id: SessionId },
    RejectedBusy,
    RejectedTooShort { session_id: SessionId },
    Canceled { session_id: SessionId },
    BeginTranscribe { session_id: SessionId },
    Deliver { session_id: SessionId, text: String },
    Completed { session_id: SessionId },
    Failed { session_id: SessionId },
    StaleIgnored { session_id: SessionId },
}

#[derive(Debug, Default)]
pub struct SessionMachine {
    phase: SessionPhase,
    next_id: SessionId,
}

impl SessionMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(&self) -> &SessionPhase {
        &self.phase
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.phase, SessionPhase::Idle)
    }

    pub fn active_session_id(&self) -> Option<SessionId> {
        match self.phase {
            SessionPhase::Idle => None,
            SessionPhase::Recording { session_id }
            | SessionPhase::Transcribing { session_id }
            | SessionPhase::Delivering { session_id } => Some(session_id),
        }
    }

    /// Only while the mic is open — not during ASR/delivery.
    pub fn recording_session_id(&self) -> Option<SessionId> {
        match self.phase {
            SessionPhase::Recording { session_id } => Some(session_id),
            _ => None,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording_session_id().is_some()
    }

    pub fn handle(&mut self, cmd: SessionCommand) -> SessionEffect {
        match cmd {
            SessionCommand::Start => self.on_start(),
            SessionCommand::Stop { duration_ms } => self.on_stop(duration_ms),
            SessionCommand::Cancel => self.on_cancel(),
            SessionCommand::TranscriptionReady { session_id, text } => {
                self.on_transcription_ready(session_id, text)
            }
            SessionCommand::DeliveryFinished { session_id, ok } => {
                self.on_delivery_finished(session_id, ok)
            }
        }
    }

    fn on_start(&mut self) -> SessionEffect {
        if !self.is_idle() {
            return SessionEffect::RejectedBusy;
        }
        let session_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.phase = SessionPhase::Recording { session_id };
        SessionEffect::BeganRecording { session_id }
    }

    fn on_stop(&mut self, duration_ms: u64) -> SessionEffect {
        let SessionPhase::Recording { session_id } = self.phase else {
            return SessionEffect::RejectedBusy;
        };
        if duration_ms < MIN_RECORDING_MS {
            self.phase = SessionPhase::Idle;
            return SessionEffect::RejectedTooShort { session_id };
        }
        self.phase = SessionPhase::Transcribing { session_id };
        SessionEffect::BeginTranscribe { session_id }
    }

    fn on_cancel(&mut self) -> SessionEffect {
        let Some(session_id) = self.active_session_id() else {
            return SessionEffect::RejectedBusy;
        };
        self.phase = SessionPhase::Idle;
        SessionEffect::Canceled { session_id }
    }

    fn on_transcription_ready(&mut self, session_id: SessionId, text: String) -> SessionEffect {
        match self.phase {
            SessionPhase::Transcribing { session_id: active } if active == session_id => {
                self.phase = SessionPhase::Delivering { session_id };
                SessionEffect::Deliver { session_id, text }
            }
            _ => SessionEffect::StaleIgnored { session_id },
        }
    }

    fn on_delivery_finished(&mut self, session_id: SessionId, ok: bool) -> SessionEffect {
        match self.phase {
            SessionPhase::Delivering { session_id: active } if active == session_id => {
                self.phase = SessionPhase::Idle;
                if ok {
                    SessionEffect::Completed { session_id }
                } else {
                    SessionEffect::Failed { session_id }
                }
            }
            _ => SessionEffect::StaleIgnored { session_id },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_assigns_monotonic_ids() {
        let mut m = SessionMachine::new();
        assert_eq!(
            m.handle(SessionCommand::Start),
            SessionEffect::BeganRecording { session_id: 0 }
        );
        let _ = m.handle(SessionCommand::Cancel);
        assert_eq!(
            m.handle(SessionCommand::Start),
            SessionEffect::BeganRecording { session_id: 1 }
        );
    }

    #[test]
    fn busy_start_rejected() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        assert_eq!(m.handle(SessionCommand::Start), SessionEffect::RejectedBusy);
        assert!(matches!(
            m.phase(),
            SessionPhase::Recording { session_id: 0 }
        ));
    }

    #[test]
    fn cancel_recording_returns_idle() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        assert_eq!(
            m.handle(SessionCommand::Cancel),
            SessionEffect::Canceled { session_id: 0 }
        );
        assert!(m.is_idle());
    }

    #[test]
    fn too_short_stop_cancels_without_transcribe() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        assert_eq!(
            m.handle(SessionCommand::Stop { duration_ms: 100 }),
            SessionEffect::RejectedTooShort { session_id: 0 }
        );
        assert!(m.is_idle());
    }

    #[test]
    fn happy_path_fake_transcription_and_delivery() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        assert_eq!(
            m.handle(SessionCommand::Stop { duration_ms: 500 }),
            SessionEffect::BeginTranscribe { session_id: 0 }
        );
        assert_eq!(
            m.handle(SessionCommand::TranscriptionReady {
                session_id: 0,
                text: "落字测试".into(),
            }),
            SessionEffect::Deliver {
                session_id: 0,
                text: "落字测试".into(),
            }
        );
        assert_eq!(
            m.handle(SessionCommand::DeliveryFinished {
                session_id: 0,
                ok: true,
            }),
            SessionEffect::Completed { session_id: 0 }
        );
        assert!(m.is_idle());
    }

    #[test]
    fn late_result_for_wrong_session_is_discarded() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        let _ = m.handle(SessionCommand::Stop { duration_ms: 500 });
        assert_eq!(
            m.handle(SessionCommand::TranscriptionReady {
                session_id: 99,
                text: "late".into(),
            }),
            SessionEffect::StaleIgnored { session_id: 99 }
        );
        assert!(matches!(
            m.phase(),
            SessionPhase::Transcribing { session_id: 0 }
        ));
    }

    #[test]
    fn cancel_during_transcribe_then_late_deliver_ignored() {
        let mut m = SessionMachine::new();
        let _ = m.handle(SessionCommand::Start);
        let _ = m.handle(SessionCommand::Stop { duration_ms: 500 });
        assert_eq!(
            m.handle(SessionCommand::Cancel),
            SessionEffect::Canceled { session_id: 0 }
        );
        assert_eq!(
            m.handle(SessionCommand::TranscriptionReady {
                session_id: 0,
                text: "落字测试".into(),
            }),
            SessionEffect::StaleIgnored { session_id: 0 }
        );
        assert!(m.is_idle());
    }

    #[test]
    fn recording_session_id_only_while_recording() {
        let mut m = SessionMachine::new();
        assert!(m.recording_session_id().is_none());
        let _ = m.handle(SessionCommand::Start);
        assert_eq!(m.recording_session_id(), Some(0));
        let _ = m.handle(SessionCommand::Stop { duration_ms: 500 });
        assert!(!m.is_recording());
        assert_eq!(m.active_session_id(), Some(0));
        assert!(m.recording_session_id().is_none());
    }
}
