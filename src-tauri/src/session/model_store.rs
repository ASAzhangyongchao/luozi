//! Model manifest, SHA256 verify, and download (M4).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::asr::{self, APP_SUPPORT_DIR_NAME, MODEL_FILE_NAME};

const MANIFEST_JSON: &str = include_str!("../../resources/models-manifest.json");
const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelManifest {
    pub schema_version: u32,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub file_name: String,
    pub bytes: u64,
    pub sha256: String,
    pub url: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub recommended: bool,
}

pub fn load_manifest() -> Result<ModelManifest, String> {
    let manifest: ModelManifest =
        serde_json::from_str(MANIFEST_JSON).map_err(|e| format!("model_manifest_invalid: {e}"))?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "model_manifest_schema_unsupported: got {} expected {MANIFEST_SCHEMA_VERSION}",
            manifest.schema_version
        ));
    }
    Ok(manifest)
}

pub fn recommended_model() -> Result<ModelEntry, String> {
    let m = load_manifest()?;
    m.models
        .into_iter()
        .find(|e| e.recommended)
        .or_else(|| {
            load_manifest()
                .ok()
                .and_then(|x| x.models.into_iter().next())
        })
        .ok_or_else(|| "model_manifest_empty".into())
}

pub fn models_dir() -> PathBuf {
    asr::default_model_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Library/Application Support")
                .join(APP_SUPPORT_DIR_NAME)
                .join("models")
        })
}

pub fn path_for_entry(entry: &ModelEntry) -> PathBuf {
    models_dir().join(&entry.file_name)
}

/// Hex SHA256 of file contents.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("model_open_failed: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 256];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("model_read_failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_entry(path: &Path, entry: &ModelEntry) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("model_missing: {}", path.display()));
    }
    let meta = fs::metadata(path).map_err(|e| format!("model_stat_failed: {e}"))?;
    let len = meta.len();
    // Allow ±1% drift for mirror quirks, but still require hash match.
    let min = entry.bytes.saturating_mul(99) / 100;
    let max = entry.bytes.saturating_mul(101) / 100 + 1024;
    if len < min || len > max {
        return Err(format!(
            "model_size_mismatch: got {len} expected ~{}",
            entry.bytes
        ));
    }
    let got = sha256_file(path)?;
    if !got.eq_ignore_ascii_case(entry.sha256.trim()) {
        return Err(format!(
            "model_checksum_failed: got {got} expected {}",
            entry.sha256
        ));
    }
    Ok(())
}

pub fn model_status() -> String {
    let Ok(entry) = recommended_model() else {
        return "模型清单无效".into();
    };
    let path = path_for_entry(&entry);
    match verify_entry(&path, &entry) {
        Ok(()) => format!("模型就绪 · {}", entry.display_name),
        Err(err) if err.starts_with("model_missing") => "模型未安装 · 托盘可下载".into(),
        Err(_) => "模型校验失败 · 请重新下载".into(),
    }
}

fn disk_free_bytes(dir: &Path) -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("df")
            .args(["-k", &dir.to_string_lossy()])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        // Filesystem 1K-blocks Used Available ...
        let line = text.lines().nth(1)?;
        let avail_k: u64 = line.split_whitespace().nth(3)?.parse().ok()?;
        Some(avail_k * 1024)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = dir;
        None
    }
}

/// Download recommended model if missing or checksum fails. Reports progress 0..=100 via callback.
pub fn ensure_recommended_model<F>(mut on_progress: F) -> Result<PathBuf, String>
where
    F: FnMut(u8, &str),
{
    let entry = recommended_model()?;
    let dest = path_for_entry(&entry);
    if verify_entry(&dest, &entry).is_ok() {
        on_progress(100, "already_ok");
        return Ok(dest);
    }

    let dir = models_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("model_dir_create_failed: {e}"))?;

    if let Some(free) = disk_free_bytes(&dir) {
        let need = entry.bytes + 64 * 1024 * 1024;
        if free < need {
            return Err(format!(
                "model_disk_full: need ~{} MB free, have ~{} MB",
                need / (1024 * 1024),
                free / (1024 * 1024)
            ));
        }
    }

    let partial = dest.with_extension("bin.partial");
    let _ = fs::remove_file(&partial);

    on_progress(1, "downloading");
    download_to(&entry.url, &partial, entry.bytes, &mut on_progress)?;

    on_progress(95, "verifying");
    verify_entry(&partial, &entry).inspect_err(|_| {
        let _ = fs::remove_file(&partial);
    })?;

    // Replace destination atomically-ish.
    let _ = fs::remove_file(&dest);
    fs::rename(&partial, &dest).map_err(|e| format!("model_rename_failed: {e}"))?;
    on_progress(100, "done");
    eprintln!("luozi: model ready at {}", dest.display());
    Ok(dest)
}

fn download_to<F>(
    url: &str,
    dest: &Path,
    expected_bytes: u64,
    on_progress: &mut F,
) -> Result<(), String>
where
    F: FnMut(u8, &str),
{
    // Prefer curl for resume-friendly large files on Mac.
    if let Ok(status) = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--retry",
            "3",
            "--retry-delay",
            "2",
            "-o",
            &dest.to_string_lossy(),
            url,
        ])
        .status()
    {
        if status.success() {
            on_progress(90, "downloaded");
            return Ok(());
        }
        return Err(format!("model_download_curl_failed: status={status}"));
    }

    // Fallback: ureq stream
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("model_download_failed: {e}"))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| format!("model_create_failed: {e}"))?;
    let mut buf = [0u8; 1024 * 64];
    let mut written: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("model_download_read_failed: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("model_download_write_failed: {e}"))?;
        written += n as u64;
        if let Some(progress) = written
            .checked_mul(90)
            .and_then(|value| value.checked_div(expected_bytes))
        {
            let pct = progress.min(90) as u8;
            on_progress(pct.max(1), "downloading");
        }
    }
    file.flush()
        .map_err(|e| format!("model_download_flush_failed: {e}"))?;
    let _ = MODEL_FILE_NAME;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn manifest_parses_and_has_recommended() {
        let m = load_manifest().expect("manifest");
        assert_eq!(m.schema_version, 1);
        assert!(m.models.iter().any(|e| e.recommended));
        let rec = recommended_model().unwrap();
        assert_eq!(rec.file_name, "ggml-small.bin");
        assert_eq!(rec.sha256.len(), 64);
    }

    #[test]
    fn verify_rejects_wrong_hash() {
        let dir = std::env::temp_dir().join("luozi-model-test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("tiny.bin");
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"hello-luozi").unwrap();
        }
        let entry = ModelEntry {
            id: "t".into(),
            display_name: "t".into(),
            file_name: "tiny.bin".into(),
            bytes: 11,
            sha256: "0".repeat(64),
            url: "http://example.invalid".into(),
            license: String::new(),
            recommended: false,
        };
        let err = verify_entry(&path, &entry).unwrap_err();
        assert!(err.contains("model_checksum_failed"), "{err}");
        let _ = fs::remove_file(&path);
    }
}
