//! Aggregated settings snapshot + helpers for the settings window (M8).

use std::sync::Mutex;

use luozi_core::{AppConfig, ASR_PROVIDERS, TEXT_AI_PROVIDERS};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::cloud;
use super::controller;
use super::model_store;
use super::text_ai;

pub const GITHUB_REPO_URL: &str = "https://github.com/ASAzhangyongchao/luozi";
pub const GITHUB_RELEASES_URL: &str = "https://github.com/ASAzhangyongchao/luozi/releases";

static PENDING_SECTION: Mutex<Option<String>> = Mutex::new(None);

pub fn set_pending_section(section: &str) {
    if let Ok(mut g) = PENDING_SECTION.lock() {
        *g = Some(section.to_string());
    }
}

pub fn take_pending_section() -> Option<String> {
    PENDING_SECTION.lock().ok().and_then(|mut g| g.take())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOptionDto {
    pub id: String,
    pub label: String,
    pub help: String,
    pub supports_cloud: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub version: String,
    pub asr_mode_label: String,
    pub asr_mode: String,
    pub model_status: String,
    pub model_ready: bool,
    pub cloud_asr_ready: bool,
    pub cloud_asr_host: String,
    pub cloud_asr_provider: String,
    pub cloud_asr_provider_label: String,
    pub cloud_asr_model: String,
    pub cloud_asr_supports: bool,
    pub cloud_asr_providers: Vec<ProviderOptionDto>,
    pub text_ai_ready: bool,
    pub text_ai_host: String,
    pub text_ai_provider: String,
    pub text_ai_provider_label: String,
    pub text_ai_model: String,
    pub text_ai_providers: Vec<ProviderOptionDto>,
    pub continue_speaking_shortcut: String,
    pub voice_edit_shortcut: String,
    pub registered_continue: Option<String>,
    pub accessibility_trusted: bool,
    pub microphone_authorized: bool,
    pub microphone_status: String,
    pub hold_to_talk: bool,
    pub repo_url: String,
    pub releases_url: String,
}

pub fn snapshot(app: &AppHandle) -> SettingsSnapshot {
    let cfg = super::config_store::load();
    let registered = app
        .try_state::<controller::AppSessionState>()
        .and_then(|s| s.registered_continue.lock().ok().and_then(|g| g.clone()));
    let mic = super::permissions::microphone_authorization();
    let asr_label = luozi_core::asr_preset(&cfg.cloud_asr.provider_id)
        .map(|p| p.label_zh.to_string())
        .unwrap_or_else(|| cfg.cloud_asr.provider_id.clone());
    let text_label = luozi_core::text_ai_preset(&cfg.text_ai.provider_id)
        .map(|p| p.label_zh.to_string())
        .unwrap_or_else(|| cfg.text_ai.provider_id.clone());
    SettingsSnapshot {
        version: env!("CARGO_PKG_VERSION").to_string(),
        asr_mode_label: cfg.asr_mode.label_zh().to_string(),
        asr_mode: cfg.asr_mode.as_str().to_string(),
        model_status: model_store::model_status(),
        model_ready: super::asr::default_model_path().is_file(),
        cloud_asr_ready: cloud::cloud_ready(&cfg.cloud_asr),
        cloud_asr_host: cfg.cloud_asr.host().unwrap_or_default(),
        cloud_asr_provider: cfg.cloud_asr.provider_id.clone(),
        cloud_asr_provider_label: asr_label,
        cloud_asr_model: cfg.cloud_asr.model.clone(),
        cloud_asr_supports: cfg.cloud_asr.protocol.supports_cloud(),
        cloud_asr_providers: ASR_PROVIDERS
            .iter()
            .map(|p| ProviderOptionDto {
                id: p.id.into(),
                label: p.label_zh.into(),
                help: p.help_zh.into(),
                supports_cloud: p.protocol.supports_cloud(),
            })
            .collect(),
        text_ai_ready: text_ai::text_ai_ready(&cfg.text_ai),
        text_ai_host: cfg.text_ai.host().unwrap_or_default(),
        text_ai_provider: cfg.text_ai.provider_id.clone(),
        text_ai_provider_label: text_label,
        text_ai_model: cfg.text_ai.model.clone(),
        text_ai_providers: TEXT_AI_PROVIDERS
            .iter()
            .map(|p| ProviderOptionDto {
                id: p.id.into(),
                label: p.label_zh.into(),
                help: p.help_zh.into(),
                supports_cloud: true,
            })
            .collect(),
        continue_speaking_shortcut: cfg.continue_speaking_shortcut.clone(),
        voice_edit_shortcut: cfg.voice_edit_shortcut.clone(),
        registered_continue: registered,
        accessibility_trusted: controller::accessibility_trusted_for_tray(),
        microphone_authorized: mic.is_ready(),
        microphone_status: mic.as_str().to_string(),
        hold_to_talk: cfg.hold_to_talk,
        repo_url: GITHUB_REPO_URL.into(),
        releases_url: GITHUB_RELEASES_URL.into(),
    }
}

pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("open_url_failed: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err("open_url_unsupported".into())
    }
}

pub fn open_privacy_microphone() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("unsupported".into())
    }
}

pub fn open_privacy_accessibility() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("unsupported".into())
    }
}

#[allow(dead_code)]
pub fn config() -> AppConfig {
    super::config_store::load()
}
