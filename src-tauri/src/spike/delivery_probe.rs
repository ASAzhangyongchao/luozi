use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde::Serialize;

use super::macos::{
    capture_target, deliver_probe, focus_first_secure_field_in_pid, validate_target,
};
use super::target::ValidationState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryCaseResult {
    pub name: String,
    pub expected: String,
    pub actual: String,
    pub delivered: Option<String>,
    pub pass: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryMatrixReport {
    pub ok: bool,
    pub cases: Vec<DeliveryCaseResult>,
    pub message: String,
}

fn state_name(state: &ValidationState) -> String {
    match state {
        ValidationState::SameTarget => "same_target".into(),
        ValidationState::Changed => "changed".into(),
        ValidationState::Unsupported => "unsupported".into(),
        ValidationState::Secure => "secure".into(),
    }
}

fn activate_text_edit_plain() -> Result<(), String> {
    let status = Command::new("open")
        .args(["-a", "TextEdit"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("open TextEdit failed".into());
    }
    thread::sleep(Duration::from_millis(800));
    // AppleScript from Luozi (trusted) — create empty plain doc focus.
    let script = r#"
tell application "TextEdit"
  activate
  make new document
  set text of front document to ""
end tell
"#;
    let status = Command::new("osascript")
        .args(["-e", script])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("TextEdit AppleScript activate failed".into());
    }
    thread::sleep(Duration::from_millis(500));
    Ok(())
}

fn activate_safari() -> Result<(), String> {
    let status = Command::new("open")
        .args(["-a", "Safari"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("open Safari failed".into());
    }
    thread::sleep(Duration::from_millis(900));
    Ok(())
}

fn open_password_field() -> Result<(), String> {
    let html = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/manual/fixtures/password.html")
        .canonicalize()
        .map_err(|e| format!("password fixture missing: {e}"))?;
    let url = format!("file://{}", html.display());

    let status = Command::new("open")
        .args(["-a", "Safari", &url])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("open Safari password page failed".into());
    }

    let activate = r#"
tell application "Safari" to activate
delay 0.8
"#;
    let _ = Command::new("osascript").args(["-e", activate]).status();
    thread::sleep(Duration::from_millis(900));

    let pid_out = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to get unix id of first process whose name is "Safari""#,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    let pid_str = String::from_utf8_lossy(&pid_out.stdout).trim().to_string();
    let pid: i32 = pid_str
        .parse()
        .map_err(|_| format!("Safari pid parse failed: `{pid_str}`"))?;

    match focus_first_secure_field_in_pid(pid) {
        Ok(true) => {}
        Ok(false) => return Err("Safari AX tree has no AXSecureTextField".into()),
        Err(e) => return Err(e),
    }
    thread::sleep(Duration::from_millis(400));

    let token = capture_target()?;
    if token.is_secure {
        Ok(())
    } else {
        Err(format!(
            "focused element still not secure after AX focus: role={} id={}",
            token.role, token.element_id
        ))
    }
}

fn case_same_target() -> DeliveryCaseResult {
    let name = "same_plain_textedit_2s".into();
    let expected = "same_target".into();
    if let Err(e) = activate_text_edit_plain() {
        return DeliveryCaseResult {
            name,
            expected,
            actual: "error".into(),
            delivered: None,
            pass: false,
            detail: e,
        };
    }

    let token = match capture_target() {
        Ok(t) => t,
        Err(e) => {
            return DeliveryCaseResult {
                name,
                expected,
                actual: "error".into(),
                delivered: None,
                pass: false,
                detail: e,
            };
        }
    };

    thread::sleep(Duration::from_secs(2));
    let validation = match validate_target(&token) {
        Ok(v) => v,
        Err(e) => {
            return DeliveryCaseResult {
                name,
                expected,
                actual: "error".into(),
                delivered: None,
                pass: false,
                detail: e,
            };
        }
    };
    let actual = state_name(&validation);
    if validation != ValidationState::SameTarget {
        return DeliveryCaseResult {
            name,
            expected,
            actual,
            delivered: None,
            pass: false,
            detail: format!("token={token:?}"),
        };
    }

    let delivered = match deliver_probe(&token) {
        Ok(v) => v,
        Err(e) => {
            return DeliveryCaseResult {
                name,
                expected,
                actual,
                delivered: None,
                pass: false,
                detail: e,
            };
        }
    };
    let delivered_name = state_name(&delivered);
    // SameTarget write OR Unsupported (clipboard fallback) both acceptable if no mis-delivery.
    let pass = matches!(
        delivered,
        ValidationState::SameTarget | ValidationState::Unsupported
    );
    DeliveryCaseResult {
        name,
        expected: "same_target(+write|unsupported_fallback)".into(),
        actual,
        delivered: Some(delivered_name),
        pass,
        detail: format!("role={} secure={}", token.role, token.is_secure),
    }
}

fn case_changed_app() -> DeliveryCaseResult {
    let name = "switch_app_during_wait".into();
    let expected = "changed".into();
    if let Err(e) = activate_text_edit_plain() {
        return DeliveryCaseResult {
            name,
            expected,
            actual: "error".into(),
            delivered: None,
            pass: false,
            detail: e,
        };
    }
    let token = match capture_target() {
        Ok(t) => t,
        Err(e) => {
            return DeliveryCaseResult {
                name,
                expected,
                actual: "error".into(),
                delivered: None,
                pass: false,
                detail: e,
            };
        }
    };
    thread::sleep(Duration::from_millis(400));
    if let Err(e) = activate_safari() {
        return DeliveryCaseResult {
            name,
            expected,
            actual: "error".into(),
            delivered: None,
            pass: false,
            detail: e,
        };
    }
    thread::sleep(Duration::from_millis(600));
    match validate_target(&token) {
        Ok(v) => {
            let actual = state_name(&v);
            DeliveryCaseResult {
                name,
                expected,
                actual: actual.clone(),
                delivered: None,
                pass: v == ValidationState::Changed,
                detail: format!("after switch validate={actual}"),
            }
        }
        Err(e) => DeliveryCaseResult {
            name,
            expected,
            actual: "error".into(),
            delivered: None,
            pass: false,
            detail: e,
        },
    }
}

fn case_secure_password() -> DeliveryCaseResult {
    let name = "password_field_secure".into();
    let expected = "secure".into();
    if let Err(e) = open_password_field() {
        return DeliveryCaseResult {
            name,
            expected,
            actual: "error".into(),
            delivered: None,
            pass: false,
            detail: e,
        };
    }
    let token = match capture_target() {
        Ok(t) => t,
        Err(e) => {
            return DeliveryCaseResult {
                name,
                expected,
                actual: "error".into(),
                delivered: None,
                pass: false,
                detail: e,
            };
        }
    };
    // Never attempt write unless classified secure; mis-delivery is an automatic fail.
    if !token.is_secure {
        return DeliveryCaseResult {
            name,
            expected,
            actual: format!("role={} subrole_missing_secure", token.role),
            delivered: None,
            pass: false,
            detail: format!("refused write; element={}", token.element_id),
        };
    }

    let delivered = deliver_probe(&token).ok().map(|s| state_name(&s));
    let pass = delivered.as_deref() == Some("secure");
    DeliveryCaseResult {
        name,
        expected,
        actual: "secure".into(),
        delivered,
        pass,
        detail: format!("element={}", token.element_id),
    }
}

pub fn run_delivery_matrix_blocking() -> DeliveryMatrixReport {
    let cases = vec![
        case_same_target(),
        case_changed_app(),
        case_secure_password(),
    ];
    let ok = cases.iter().all(|c| c.pass);
    let message = if ok {
        "pass: delivery safety matrix".into()
    } else {
        let failed = cases
            .iter()
            .filter(|c| !c.pass)
            .map(|c| c.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        format!("fail or partial: {failed}")
    };

    let report = DeliveryMatrixReport { ok, cases, message };
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/manual/m0-delivery-auto-result.json");
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = fs::write(out, json);
    }
    report
}

#[tauri::command]
pub async fn run_delivery_matrix_probe() -> Result<DeliveryMatrixReport, String> {
    tauri::async_runtime::spawn_blocking(run_delivery_matrix_blocking)
        .await
        .map_err(|e| e.to_string())
}
