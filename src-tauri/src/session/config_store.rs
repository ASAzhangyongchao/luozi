//! Load / save AppConfig JSON (no secrets).

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use luozi_core::AppConfig;

use super::asr::APP_SUPPORT_DIR_NAME;

static CACHED: Mutex<Option<AppConfig>> = Mutex::new(None);

fn config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
            .join(APP_SUPPORT_DIR_NAME)
            .join("config.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_SUPPORT_DIR_NAME)
            .join("config.json")
    }
}

pub fn load() -> AppConfig {
    if let Ok(guard) = CACHED.lock() {
        if let Some(cfg) = guard.as_ref() {
            return cfg.clone();
        }
    }
    let path = config_path();
    let cfg = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<AppConfig>(&text).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    };
    let cfg = if cfg.validate().is_ok() {
        cfg
    } else {
        AppConfig::default()
    };
    if let Ok(mut guard) = CACHED.lock() {
        *guard = Some(cfg.clone());
    }
    cfg
}

pub fn save(cfg: &AppConfig) -> Result<(), String> {
    cfg.validate()?;
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("config_dir_failed: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("config_write_failed: {e}"))?;
    if let Ok(mut guard) = CACHED.lock() {
        *guard = Some(cfg.clone());
    }
    Ok(())
}

pub fn update<F>(f: F) -> Result<AppConfig, String>
where
    F: FnOnce(&mut AppConfig),
{
    let mut cfg = load();
    f(&mut cfg);
    save(&cfg)?;
    Ok(cfg)
}
