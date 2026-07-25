use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{SampleFormat as WavSampleFormat, WavSpec, WavWriter};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProbeResult {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub bytes: u64,
    pub deleted: bool,
}

fn temp_wav_path() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("luozi-m0-audio");
    fs::create_dir_all(&dir).map_err(|e| format!("temp_dir_create_failed: {e}"))?;
    let name = format!(
        "probe-{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    Ok(dir.join(name))
}

fn record_blocking() -> Result<AudioProbeResult, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no_input_device: system has no default microphone".to_string())?;

    let supported = device.default_input_config().map_err(|e| {
        let msg = e.to_string().to_lowercase();
        if msg.contains("permission") || msg.contains("denied") {
            format!("permission_denied: {e}")
        } else {
            format!("device_config_failed: {e}")
        }
    })?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    // Keep device channel layout; report actual channels in the probe result.

    let path = temp_wav_path()?;
    let sample_rate = config.sample_rate;
    let channels = config.channels;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let err_flag = Arc::new(Mutex::new(None::<String>));

    let stream = match sample_format {
        SampleFormat::F32 => {
            let samples_cb = samples.clone();
            let err_cb = err_flag.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[f32], _| {
                        if let Ok(mut buf) = samples_cb.lock() {
                            buf.extend_from_slice(data);
                        }
                    },
                    move |err| {
                        if let Ok(mut slot) = err_cb.lock() {
                            *slot = Some(format!("device_disconnected: {err}"));
                        }
                    },
                    None,
                )
                .map_err(map_stream_err)?
        }
        SampleFormat::I16 => {
            let samples_cb = samples.clone();
            let err_cb = err_flag.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[i16], _| {
                        if let Ok(mut buf) = samples_cb.lock() {
                            buf.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                        }
                    },
                    move |err| {
                        if let Ok(mut slot) = err_cb.lock() {
                            *slot = Some(format!("device_disconnected: {err}"));
                        }
                    },
                    None,
                )
                .map_err(map_stream_err)?
        }
        SampleFormat::U16 => {
            let samples_cb = samples.clone();
            let err_cb = err_flag.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[u16], _| {
                        if let Ok(mut buf) = samples_cb.lock() {
                            buf.extend(
                                data.iter()
                                    .map(|s| (*s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                            );
                        }
                    },
                    move |err| {
                        if let Ok(mut slot) = err_cb.lock() {
                            *slot = Some(format!("device_disconnected: {err}"));
                        }
                    },
                    None,
                )
                .map_err(map_stream_err)?
        }
        other => return Err(format!("unsupported_sample_format: {other:?}")),
    };

    stream.play().map_err(map_stream_err)?;
    std::thread::sleep(Duration::from_secs(1));
    drop(stream);

    if let Ok(slot) = err_flag.lock() {
        if let Some(err) = slot.as_ref() {
            let _ = fs::remove_file(&path);
            return Err(err.clone());
        }
    }

    let data = samples
        .lock()
        .map_err(|_| "sample_buffer_lock_failed".to_string())?
        .clone();
    if data.is_empty() {
        let _ = fs::remove_file(&path);
        return Err(
            "permission_or_silent_capture: recorded zero frames (mic denied or muted)".into(),
        );
    }

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: WavSampleFormat::Float,
    };
    {
        let mut writer =
            WavWriter::create(&path, spec).map_err(|e| format!("wav_write_failed: {e}"))?;
        for sample in &data {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("wav_write_failed: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("wav_finalize_failed: {e}"))?;
    }

    let bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let frames = data.len() / channels.max(1) as usize;
    let deleted = fs::remove_file(&path).is_ok();
    let _ = fs::remove_dir(std::env::temp_dir().join("luozi-m0-audio"));

    Ok(AudioProbeResult {
        sample_rate,
        channels,
        frames,
        bytes,
        deleted,
    })
}

fn map_stream_err(e: cpal::Error) -> String {
    let msg = e.to_string().to_lowercase();
    if msg.contains("permission") || msg.contains("denied") {
        format!("permission_denied: {e}")
    } else if msg.contains("disconnect") || msg.contains("not available") {
        format!("device_disconnected: {e}")
    } else {
        format!("stream_failed: {e}")
    }
}

#[tauri::command]
pub async fn record_one_second_probe() -> Result<AudioProbeResult, String> {
    tauri::async_runtime::spawn_blocking(record_blocking)
        .await
        .map_err(|e| e.to_string())?
}

pub fn run_audio_probe_blocking() -> Result<AudioProbeResult, String> {
    record_blocking()
}
