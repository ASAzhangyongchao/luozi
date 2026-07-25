//! Continuous microphone capture for one dictation session.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

/// Spec ceiling for a single utterance (seconds → ms).
pub const MAX_RECORDING_MS: u64 = 30_000;

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

    pub fn start(&mut self) -> Result<(), String> {
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

        let stream = build_stream(&device, &config, sample_format, samples.clone(), err.clone())?;
        stream
            .play()
            .map_err(|e| map_err_string(e.to_string()))?;

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
        let active = self.active.take().ok_or_else(|| "not_recording".to_string())?;
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
            device.build_input_stream(
                config.clone(),
                move |data: &[f32], _| {
                    if let Ok(mut buf) = samples_cb.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I16 => {
            let samples_cb = samples;
            device.build_input_stream(
                config.clone(),
                move |data: &[i16], _| {
                    if let Ok(mut buf) = samples_cb.lock() {
                        buf.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::I32 => {
            let samples_cb = samples;
            device.build_input_stream(
                config.clone(),
                move |data: &[i32], _| {
                    if let Ok(mut buf) = samples_cb.lock() {
                        buf.extend(data.iter().map(|s| *s as f32 / i32::MAX as f32));
                    }
                },
                err_cb,
                None,
            )
        }
        SampleFormat::U16 => {
            let samples_cb = samples;
            device.build_input_stream(
                config.clone(),
                move |data: &[u16], _| {
                    if let Ok(mut buf) = samples_cb.lock() {
                        buf.extend(
                            data.iter()
                                .map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                        );
                    }
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

