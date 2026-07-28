const NOISE_FLOOR: f32 = 0.015;
const ACTIVE_CEILING: f32 = 0.42;
const ATTACK: f32 = 0.58;
const RELEASE: f32 = 0.16;

#[derive(Clone, Debug, Default)]
pub struct VoiceEnergyMeter {
    smoothed: f32,
}

impl VoiceEnergyMeter {
    pub fn observe_levels(&mut self, rms: f32, peak: f32) -> f32 {
        let mixed = (rms.max(0.0) * 0.72 + peak.max(0.0) * 0.28).clamp(0.0, 1.0);
        let target = ((mixed - NOISE_FLOOR) / (ACTIVE_CEILING - NOISE_FLOOR)).clamp(0.0, 1.0);
        let coefficient = if target > self.smoothed {
            ATTACK
        } else {
            RELEASE
        };

        self.smoothed += (target - self.smoothed) * coefficient;
        if self.smoothed < 0.005 {
            self.smoothed = 0.0;
        }

        self.smoothed.clamp(0.0, 1.0)
    }
}

pub fn measure_levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }

    let (sum_of_squares, peak) = samples.iter().fold((0.0_f32, 0.0_f32), |acc, sample| {
        let sample = if sample.is_nan() {
            0.0
        } else {
            sample.clamp(-1.0, 1.0)
        };
        (acc.0 + sample * sample, acc.1.max(sample.abs()))
    });

    ((sum_of_squares / samples.len() as f32).sqrt(), peak)
}

#[derive(Clone, Debug)]
pub struct EnergyThrottle {
    interval_ms: u64,
    last_emitted_at_ms: Option<u64>,
}

impl EnergyThrottle {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms: interval_ms.max(1),
            last_emitted_at_ms: None,
        }
    }

    pub fn should_emit(&mut self, now_ms: u64) -> bool {
        let should_emit = self.last_emitted_at_ms.is_none_or(|last_emitted_at_ms| {
            now_ms.saturating_sub(last_emitted_at_ms) >= self.interval_ms
        });

        if should_emit {
            self.last_emitted_at_ms = Some(now_ms);
        }

        should_emit
    }
}

#[cfg(test)]
mod tests {
    use super::{measure_levels, EnergyThrottle, VoiceEnergyMeter};

    #[test]
    fn silence_stays_at_zero() {
        let mut meter = VoiceEnergyMeter::default();

        assert_eq!(meter.observe_levels(0.0, 0.0), 0.0);
    }

    #[test]
    fn louder_input_produces_more_energy_and_is_clamped() {
        let mut quiet_meter = VoiceEnergyMeter::default();
        let quiet = quiet_meter.observe_levels(0.04, 0.05);
        let mut loud_meter = VoiceEnergyMeter::default();
        let loud = loud_meter.observe_levels(1.0, 1.0);

        assert!(loud > quiet);
        assert!((0.0..=1.0).contains(&loud));
    }

    #[test]
    fn release_falls_instead_of_snapping_to_zero() {
        let mut meter = VoiceEnergyMeter::default();
        let loud = meter.observe_levels(1.0, 1.0);
        let released = meter.observe_levels(0.0, 0.0);

        assert!(released > 0.0);
        assert!(released < loud);
    }

    #[test]
    fn throttle_enforces_40ms_minimum_interval() {
        let mut throttle = EnergyThrottle::new(40);

        assert!(throttle.should_emit(0));
        assert!(!throttle.should_emit(20));
        assert!(throttle.should_emit(40));
        assert!(!throttle.should_emit(79));
        assert!(throttle.should_emit(80));
    }

    #[test]
    fn throttle_does_not_emit_when_clock_rolls_back() {
        let mut throttle = EnergyThrottle::new(40);

        assert!(throttle.should_emit(100));
        assert!(!throttle.should_emit(90));
        assert!(throttle.should_emit(140));
    }

    #[test]
    fn measure_levels_returns_zero_for_empty_samples() {
        assert_eq!(measure_levels(&[]), (0.0, 0.0));
    }

    #[test]
    fn measure_levels_clamps_samples_before_calculating_rms_and_peak() {
        let (rms, peak) = measure_levels(&[1.0, -1.0, 0.0, 2.0]);

        assert!((rms - (3.0_f32 / 4.0).sqrt()).abs() < 0.000_001);
        assert!((peak - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn measure_levels_treats_nan_as_silence_and_keeps_output_finite() {
        let (rms, peak) = measure_levels(&[f32::NAN, f32::INFINITY, f32::NEG_INFINITY]);

        assert!(rms.is_finite());
        assert!(peak.is_finite());
        assert!((0.0..=1.0).contains(&rms));
        assert!((0.0..=1.0).contains(&peak));
        assert!((rms - (2.0_f32 / 3.0).sqrt()).abs() < 0.000_001);
        assert_eq!(peak, 1.0);
    }
}
