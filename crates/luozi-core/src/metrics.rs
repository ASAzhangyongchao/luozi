use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricPhase {
    HudVisible,
    RecordingReady,
    AsrFinished,
    CleanupFinished,
    DeliveryFinished,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTimings {
    values: Vec<(MetricPhase, u64)>,
}

impl PhaseTimings {
    pub fn record(&mut self, phase: MetricPhase, elapsed_ms: u64) -> Result<(), &'static str> {
        if self.values.iter().any(|(p, _)| *p == phase) {
            return Err("phase_already_recorded");
        }
        if self
            .values
            .last()
            .is_some_and(|(_, last)| elapsed_ms < *last)
        {
            return Err("phase_not_monotonic");
        }
        self.values.push((phase, elapsed_ms));
        Ok(())
    }

    pub fn get(&self, phase: MetricPhase) -> Option<u64> {
        self.values
            .iter()
            .find_map(|(p, value)| (*p == phase).then_some(*value))
    }
}

pub fn nearest_rank(values: &[u64], percentile: u8) -> Option<u64> {
    if values.is_empty() || percentile == 0 || percentile > 100 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((percentile as usize * sorted.len()) + 99) / 100;
    sorted.get(rank.saturating_sub(1)).copied()
}

pub fn edit_distance(expected: &str, actual: &str) -> usize {
    let a: Vec<char> = expected.chars().collect();
    let b: Vec<char> = actual.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let replace = prev[j] + usize::from(ca != cb);
            next[j + 1] = (prev[j + 1] + 1).min(next[j] + 1).min(replace);
        }
        prev = next;
    }
    prev[b.len()]
}

pub fn character_error_rate(expected: &str, actual: &str) -> f32 {
    let count = expected.chars().count().max(1);
    edit_distance(expected, actual) as f32 / count as f32
}

pub fn protected_token_errors(expected: &[&str], actual: &str) -> usize {
    expected
        .iter()
        .filter(|token| !actual.contains(**token))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_each_phase_once_and_keeps_monotonic_values() {
        let mut timings = PhaseTimings::default();
        assert!(timings.record(MetricPhase::HudVisible, 80).is_ok());
        assert!(timings.record(MetricPhase::AsrFinished, 900).is_ok());
        assert_eq!(timings.get(MetricPhase::HudVisible), Some(80));
        assert!(timings.record(MetricPhase::HudVisible, 90).is_err());
        assert!(timings.record(MetricPhase::CleanupFinished, 700).is_err());
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        assert_eq!(nearest_rank(&[100, 200, 300, 400], 50), Some(200));
        assert_eq!(nearest_rank(&[100, 200, 300, 400], 95), Some(400));
        assert_eq!(nearest_rank(&[], 50), None);
    }

    #[test]
    fn character_error_rate_counts_insert_delete_replace() {
        assert_eq!(edit_distance("落字", "落字"), 0);
        assert_eq!(edit_distance("落字", "落子"), 1);
        assert!((character_error_rate("今天三点", "今天四点") - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn protected_token_score_requires_exact_tokens() {
        let expected = ["2026-07-29", "ASFF", "12800"];
        assert_eq!(
            protected_token_errors(&expected, "日期 2026-07-29，ASFF 金额 12800"),
            0
        );
        assert_eq!(
            protected_token_errors(&expected, "日期 2026-07-28，金额 12800"),
            2
        );
    }
}
