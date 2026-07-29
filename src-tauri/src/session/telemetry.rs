use std::collections::HashMap;
use std::time::Instant;

use luozi_core::{MetricPhase, PhaseTimings};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetricsDto {
    pub session_id: u64,
    pub timings: PhaseTimings,
}

#[derive(Default)]
pub struct SessionTelemetry {
    active: HashMap<u64, (Instant, PhaseTimings)>,
}

impl SessionTelemetry {
    pub fn begin(&mut self, session_id: u64) {
        self.active
            .insert(session_id, (Instant::now(), PhaseTimings::default()));
    }

    pub fn mark(&mut self, session_id: u64, phase: MetricPhase) {
        if let Some((started, timings)) = self.active.get_mut(&session_id) {
            let _ = timings.record(phase, started.elapsed().as_millis() as u64);
        }
    }

    pub fn finish(&mut self, session_id: u64) -> Option<SessionMetricsDto> {
        let (_, timings) = self.active.remove(&session_id)?;
        Some(SessionMetricsDto {
            session_id,
            timings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_removes_session_and_never_stores_text() {
        let mut state = SessionTelemetry::default();
        state.begin(7);
        state.mark(7, MetricPhase::HudVisible);
        let dto = state.finish(7).expect("metrics");
        assert_eq!(dto.session_id, 7);
        assert!(state.finish(7).is_none());
    }
}
