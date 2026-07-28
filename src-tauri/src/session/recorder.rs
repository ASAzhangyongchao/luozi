//! Continuous microphone capture for one dictation session.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use luozi_core::{EnergyThrottle, VoiceEnergyMeter};

/// Spec ceiling for a single utterance (seconds → ms).
pub const MAX_RECORDING_MS: u64 = 30_000;

pub type EnergySink = Arc<dyn Fn(f32) + Send + Sync + 'static>;

#[derive(Clone)]
struct EnergyReporter {
    state: Arc<Mutex<EnergyReporterState>>,
    started: Instant,
    sink: EnergySink,
}

struct EnergyReporterState {
    meter: VoiceEnergyMeter,
    throttle: EnergyThrottle,
}

impl EnergyReporter {
    fn new(sink: EnergySink) -> Self {
        Self {
            state: Arc::new(Mutex::new(EnergyReporterState {
                meter: VoiceEnergyMeter::default(),
                throttle: EnergyThrottle::new(40),
            })),
            started: Instant::now(),
            sink,
        }
    }

    fn observe_levels(&self, rms: f32, peak: f32) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let level = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let level = state.meter.observe_levels(rms, peak);
            state.throttle.should_emit(now_ms).then_some(level)
        };

        if let Some(level) = level {
            (self.sink)(level);
        }
    }
}

fn append_normalized<I>(buffer: &Arc<Mutex<Vec<f32>>>, reporter: &EnergyReporter, samples: I)
where
    I: IntoIterator<Item = f32>,
{
    let Ok(mut output) = buffer.lock() else {
        return;
    };
    let mut count = 0_usize;
    let mut sum_squares = 0.0_f32;
    let mut peak = 0.0_f32;

    for sample in samples {
        let value = if sample.is_nan() {
            0.0
        } else {
            sample.clamp(-1.0, 1.0)
        };
        output.push(value);
        count += 1;
        sum_squares += value * value;
        peak = peak.max(value.abs());
    }
    drop(output);

    let rms = if count == 0 {
        0.0
    } else {
        (sum_squares / count as f32).sqrt()
    };
    reporter.observe_levels(rms, peak);
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // retained for M3 ASR handoff
pub struct AudioCapture {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub duration_ms: u64,
    /// Interleaved f32 samples; M2 keeps them only until fake transcript is chosen.
    pub samples: Vec<f32>,
}

struct Active {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    err: Arc<Mutex<Option<String>>>,
    sample_rate: u32,
    channels: u16,
    started: Instant,
}

#[derive(Default)]
pub struct SessionRecorder {
    active: Option<Active>,
}

impl SessionRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.active
            .as_ref()
            .map(|a| a.started.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn start(&mut self, energy_sink: EnergySink) -> Result<(), String> {
        if self.active.is_some() {
            return Err("recorder_busy".into());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no_input_device".to_string())?;
        let supported = device.default_input_config().map_err(map_config_err)?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let sample_rate = config.sample_rate;
        let channels = config.channels;

        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let reporter = EnergyReporter::new(energy_sink);

        let stream = build_stream(
            &device,
            &config,
            sample_format,
            samples.clone(),
            err.clone(),
            reporter,
        )?;
        stream.play().map_err(|e| map_err_string(e.to_string()))?;

        self.active = Some(Active {
            stream,
            samples,
            err,
            sample_rate,
            channels,
            started: Instant::now(),
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<AudioCapture, String> {
        let active = self
            .active
            .take()
            .ok_or_else(|| "not_recording".to_string())?;
        let duration_ms = active.started.elapsed().as_millis() as u64;
        drop(active.stream);

        if let Ok(slot) = active.err.lock() {
            if let Some(e) = slot.as_ref() {
                return Err(e.clone());
            }
        }

        let data = active
            .samples
            .lock()
            .map_err(|_| "sample_buffer_lock_failed".to_string())?
            .clone();
        // M3: empty capture must fail early (mic permission / silent), not enter ASR.
        if data.is_empty() {
            return Err("permission_or_silent_capture".into());
        }

        let frames = data.len() / active.channels.max(1) as usize;
        Ok(AudioCapture {
            sample_rate: active.sample_rate,
            channels: active.channels,
            frames,
            duration_ms,
            samples: data,
        })
    }

    pub fn cancel(&mut self) {
        if let Some(active) = self.active.take() {
            drop(active.stream);
        }
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    samples: Arc<Mutex<Vec<f32>>>,
    err: Arc<Mutex<Option<String>>>,
    reporter: EnergyReporter,
) -> Result<Stream, String> {
    let err_flag = err.clone();
    let err_cb = move |e| {
        if let Ok(mut slot) = err_flag.lock() {
            *slot = Some(format!("device_disconnected: {e}"));
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => {
            let samples_cb = samples;
            let reporter_cb = reporter;
            device.build_input_stream(
                config.clone(),
                move |data: &[f32], _| {
                    append_normalized(&samples_cb, &reporter_cb, data.iter().copied());
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I16 => {
            let samples_cb = samples;
            let reporter_cb = reporter;
            device.build_input_stream(
                config.clone(),
                move |data: &[i16], _| {
                    append_normalized(
                        &samples_cb,
                        &reporter_cb,
                        data.iter().map(|sample| *sample as f32 / i16::MAX as f32),
                    );
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I32 => {
            let samples_cb = samples;
            let reporter_cb = reporter;
            device.build_input_stream(
                config.clone(),
                move |data: &[i32], _| {
                    append_normalized(
                        &samples_cb,
                        &reporter_cb,
                        data.iter().map(|sample| *sample as f32 / i32::MAX as f32),
                    );
                },
                err_cb,
                None,
            )
        }
        SampleFormat::U16 => {
            let samples_cb = samples;
            let reporter_cb = reporter;
            device.build_input_stream(
                config.clone(),
                move |data: &[u16], _| {
                    append_normalized(
                        &samples_cb,
                        &reporter_cb,
                        data.iter()
                            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0),
                    );
                },
                err_cb,
                None,
            )
        }
        other => return Err(format!("unsupported_sample_format: {other:?}")),
    };

    stream.map_err(|e| map_err_string(e.to_string()))
}

fn map_config_err(e: impl std::fmt::Display) -> String {
    map_err_string(e.to_string())
}

fn map_err_string(msg_raw: String) -> String {
    let msg = msg_raw.to_lowercase();
    if msg.contains("permission") || msg.contains("denied") {
        format!("permission_denied: {msg_raw}")
    } else if msg.contains("disconnect") || msg.contains("not available") {
        format!("device_disconnected: {msg_raw}")
    } else {
        format!("stream_failed: {msg_raw}")
    }
}

#[cfg(test)]
mod tests {
    use super::{append_normalized, EnergyReporter, EnergySink};
    use std::sync::{Arc, Mutex};

    fn collecting_reporter() -> (EnergyReporter, Arc<Mutex<Vec<f32>>>) {
        let levels = Arc::new(Mutex::new(Vec::new()));
        let levels_for_sink = levels.clone();
        let sink: EnergySink = Arc::new(move |level| {
            levels_for_sink
                .lock()
                .expect("level collector lock")
                .push(level);
        });
        (EnergyReporter::new(sink), levels)
    }

    #[test]
    fn append_normalized_writes_clamped_samples_and_reports_finite_level() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let (reporter, levels) = collecting_reporter();

        append_normalized(&buffer, &reporter, [-2.0, -0.5, 0.5, 2.0]);

        assert_eq!(
            *buffer.lock().expect("sample buffer lock"),
            vec![-1.0, -0.5, 0.5, 1.0]
        );
        let levels = levels.lock().expect("level collector lock");
        assert_eq!(levels.len(), 1);
        assert!(levels[0].is_finite());
        assert!((0.0..=1.0).contains(&levels[0]));
    }

    #[test]
    fn energy_reporter_throttles_consecutive_observations_within_40ms() {
        let (reporter, levels) = collecting_reporter();

        reporter.observe_levels(0.25, 0.5);
        reporter.observe_levels(0.5, 0.75);

        assert_eq!(levels.lock().expect("level collector lock").len(), 1);
    }

    #[test]
    fn append_normalized_treats_nan_as_silence_without_poisoning_output() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let (reporter, levels) = collecting_reporter();

        append_normalized(
            &buffer,
            &reporter,
            [f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
        );

        assert_eq!(
            *buffer.lock().expect("sample buffer lock"),
            vec![0.0, 1.0, -1.0]
        );
        let levels = levels.lock().expect("level collector lock");
        assert_eq!(levels.len(), 1);
        assert!(levels[0].is_finite());
        assert!((0.0..=1.0).contains(&levels[0]));
    }
}
