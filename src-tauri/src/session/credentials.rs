//! Mac Keychain (and env seed) for ASR API keys. Never log secrets.

use std::process::Command;

use super::asr::APP_SUPPORT_DIR_NAME;

const SERVICE: &str = APP_SUPPORT_DIR_NAME;

/// Store API key in Keychain (macOS) or reject on other platforms (M5 Mac-first).
pub fn set_secret(account: &str, secret: &str) -> Result<(), String> {
    if account.trim().is_empty() || secret.is_empty() {
        return Err("credential_empty".into());
    }
    #[cfg(target_os = "macos")]
    {
        // Delete then add — `-U` update is flaky across macOS versions.
        let _ = Command::new("security")
            .args(["delete-generic-password", "-s", SERVICE, "-a", account])
            .output();
        let status = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                SERVICE,
                "-a",
                account,
                "-w",
                secret,
                "-T",
                "",
            ])
            .status()
            .map_err(|e| format!("keychain_write_failed: {e}"))?;
        if !status.success() {
            return Err("keychain_write_failed".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (account, secret);
        Err("keychain_unsupported_platform".into())
    }
}

pub fn get_secret(account: &str) -> Result<String, String> {
    if account.trim().is_empty() {
        return Err("credential_empty".into());
    }
    // Dev seed: never log the value.
    if account == "asr.groq" {
        if let Ok(v) = std::env::var("LUOZI_GROQ_API_KEY") {
            let t = v.trim().to_string();
            if !t.is_empty() {
                return Ok(t);
            }
        }
    }
    if account == "textai.groq" {
        if let Ok(v) = std::env::var("LUOZI_TEXT_AI_API_KEY") {
            let t = v.trim().to_string();
            if !t.is_empty() {
                return Ok(t);
            }
        }
        // Allow reusing Groq ASR key for text AI in local dev when text key unset.
        if let Ok(v) = std::env::var("LUOZI_GROQ_API_KEY") {
            let t = v.trim().to_string();
            if !t.is_empty() {
                return Ok(t);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("security")
            .args(["find-generic-password", "-s", SERVICE, "-a", account, "-w"])
            .output()
            .map_err(|e| format!("keychain_read_failed: {e}"))?;
        if !out.status.success() {
            return Err("cloud_unauthorized".into());
        }
        let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if key.is_empty() {
            return Err("cloud_unauthorized".into());
        }
        Ok(key)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("cloud_unauthorized".into())
    }
}

#[allow(dead_code)]
pub fn delete_secret(account: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("security")
            .args(["delete-generic-password", "-s", SERVICE, "-a", account])
            .status();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = account;
        Ok(())
    }
}

pub fn has_secret(account: &str) -> bool {
    get_secret(account).is_ok()
}
