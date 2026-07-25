//! Local Whisper (whisper.cpp via whisper-rs) for M3 Mac-first ASR.

use std::path::{Path, PathBuf};
use std::sync::Once;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

static WHISPER_LOG_ONCE: Once = Once::new();

pub const MODEL_FILE_NAME: &str = "ggml-small.bin";
/// Matches Tauri `identifier` in tauri.conf.json for Application Support layout.
pub const APP_SUPPORT_DIR_NAME: &str = "app.luozi.desktop";

pub fn default_model_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("LUOZI_WHISPER_MODEL") {
        let p = PathBuf::from(override_path);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
            .join(APP_SUPPORT_DIR_NAME)
            .join("models")
            .join(MODEL_FILE_NAME)
    }

    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("models")
            .join(MODEL_FILE_NAME)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("models")
            .join(MODEL_FILE_NAME)
    }
}

/// Interleaved f32 → mono 16 kHz (Whisper input).
pub fn resample_to_16k_mono(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let sr = sample_rate.max(1) as f64;
    let frames = samples.len() / ch;
    if frames == 0 {
        return Vec::new();
    }

    let mut mono = Vec::with_capacity(frames);
    for i in 0..frames {
        let mut sum = 0.0f32;
        for c in 0..ch {
            sum += samples[i * ch + c];
        }
        mono.push(sum / ch as f32);
    }

    if (sample_rate as i32 - 16_000).abs() < 1 {
        return mono;
    }

    let out_len = ((frames as f64) * 16_000.0 / sr).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    let last = (frames - 1) as f64;
    for i in 0..out_len {
        let src = (i as f64) * sr / 16_000.0;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(frames - 1);
        let t = (src - i0 as f64) as f32;
        let a = mono[i0.min(frames - 1)];
        let b = mono[i1];
        out.push(a + (b - a) * t);
        let _ = last;
    }
    out
}

pub struct AsrEngine {
    ctx: WhisperContext,
    model_path: PathBuf,
}

impl AsrEngine {
    pub fn load(path: &Path) -> Result<Self, String> {
        WHISPER_LOG_ONCE.call_once(|| {
            // Best-effort; ignore if already installed by another crate version.
            let _ = std::panic::catch_unwind(whisper_rs::install_logging_hooks);
        });

        if !path.is_file() {
            return Err(format!(
                "model_missing: place {MODEL_FILE_NAME} at {} (or set LUOZI_WHISPER_MODEL)",
                path.display()
            ));
        }

        let ctx = WhisperContext::new_with_params(
            path.to_string_lossy().as_ref(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("whisper_load_failed: {e}"))?;

        eprintln!("luozi: whisper model loaded from {}", path.display());
        Ok(Self {
            ctx,
            model_path: path.to_path_buf(),
        })
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// `language`: `"auto"` → Whisper auto-detect; otherwise ISO-like code (`zh`, `en`, …).
    pub fn transcribe(&self, pcm_16k_mono: &[f32], language: &str) -> Result<String, String> {
        if pcm_16k_mono.is_empty() {
            return Err("asr_empty_audio".into());
        }

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| format!("whisper_state_failed: {e}"))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

        let lang = language.trim();
        if lang.is_empty() || lang.eq_ignore_ascii_case("auto") {
            params.set_language(None);
        } else {
            params.set_language(Some(lang));
        }

        state
            .full(params, pcm_16k_mono)
            .map_err(|e| format!("whisper_infer_failed: {e}"))?;

        let n = state
            .full_n_segments()
            .map_err(|e| format!("whisper_segments_failed: {e}"))?;
        let mut text = String::new();
        for i in 0..n {
            let seg = state
                .full_get_segment_text_lossy(i)
                .map_err(|e| format!("whisper_segment_text_failed: {e}"))?;
            text.push_str(seg.trim());
        }

        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("asr_empty_transcript".into());
        }
        Ok(text)
    }
}

/// Load (or reuse) engine and transcribe a capture buffer.
pub fn transcribe_capture(
    engine: &mut Option<AsrEngine>,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    language: &str,
) -> Result<String, String> {
    let path = default_model_path();
    if engine
        .as_ref()
        .map(|e| e.model_path() != path.as_path())
        .unwrap_or(true)
    {
        *engine = Some(AsrEngine::load(&path)?);
    }
    let pcm = resample_to_16k_mono(samples, sample_rate, channels);
    engine
        .as_ref()
        .expect("engine just loaded")
        .transcribe(&pcm, language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_same_rate_mono_passthrough() {
        let samples = vec![0.0, 0.5, -0.5, 1.0];
        let out = resample_to_16k_mono(&samples, 16_000, 1);
        assert_eq!(out, samples);
    }

    #[test]
    fn resample_stereo_averages_channels() {
        // L=1, R=3 → mono 2; two frames
        let samples = vec![1.0, 3.0, 0.0, 2.0];
        let out = resample_to_16k_mono(&samples, 16_000, 2);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 2.0).abs() < 1e-5);
        assert!((out[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn resample_48k_to_16k_length() {
        let frames = 4800; // 0.1s at 48k
        let samples: Vec<f32> = (0..frames).map(|i| (i as f32) * 0.0001).collect();
        let out = resample_to_16k_mono(&samples, 48_000, 1);
        assert!((out.len() as i32 - 1600).abs() <= 2);
    }

    #[test]
    fn default_model_path_ends_with_ggml_small() {
        let p = default_model_path();
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some(MODEL_FILE_NAME));
    }
}
