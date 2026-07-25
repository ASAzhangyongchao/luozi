use std::process::Command;
use std::thread;
use std::time::Duration;

use accessibility_sys::{
    kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXTrustedCheckOptionPrompt, kAXValueAttribute,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
    AXUIElementRef,
};
use core_foundation::base::{CFRelease, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::NSWorkspace;
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusProbeReport {
    pub ok: bool,
    pub accessibility_trusted: bool,
    pub text_edit_activated: bool,
    pub typed_value: String,
    pub expected_value: String,
    pub overlay_show_stole_frontmost: bool,
    pub frontmost_before_overlay: String,
    pub frontmost_during_overlay: String,
    pub frontmost_after_overlay: String,
    pub typing_ok: bool,
    pub focus_ok: bool,
    pub message: String,
}

fn ensure_accessibility_prompt() -> bool {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let pairs = [(key.as_CFType(), CFBoolean::true_value().as_CFType())];
        let dict = CFDictionary::from_CFType_pairs(&pairs);
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef())
    }
}

fn frontmost_name() -> String {
    let workspace = NSWorkspace::sharedWorkspace();
    match workspace.frontmostApplication() {
        Some(app) => app
            .localizedName()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(unknown)".into()),
        None => "(none)".into(),
    }
}

fn is_text_edit(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("textedit") || name.contains("文本编辑")
}

fn post_key(key: CGKeyCode, flags: CGEventFlags, down: bool) {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("event source");
    let event = CGEvent::new_keyboard_event(source, key, down).expect("key event");
    event.set_flags(flags);
    event.post(CGEventTapLocation::HID);
}

fn tap_key(key: CGKeyCode, flags: CGEventFlags) {
    post_key(key, flags, true);
    thread::sleep(Duration::from_millis(40));
    post_key(key, flags, false);
    thread::sleep(Duration::from_millis(80));
}

fn release_modifiers() {
    for key in [
        KeyCode::CONTROL,
        KeyCode::OPTION,
        KeyCode::COMMAND,
        KeyCode::RIGHT_CONTROL,
        KeyCode::RIGHT_OPTION,
        KeyCode::RIGHT_COMMAND,
    ] {
        post_key(key, CGEventFlags::CGEventFlagNull, false);
    }
    thread::sleep(Duration::from_millis(60));
}

fn type_unicode(text: &str) {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("event source");
    let down = CGEvent::new_keyboard_event(source.clone(), 0, true).expect("key down");
    down.set_flags(CGEventFlags::CGEventFlagNull);
    down.set_string(text);
    down.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(40));
    let up = CGEvent::new_keyboard_event(source, 0, false).expect("key up");
    up.set_flags(CGEventFlags::CGEventFlagNull);
    up.set_string(text);
    up.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(80));
}

fn hold_control_option_space(hold_ms: u64) {
    let flags = CGEventFlags::CGEventFlagControl | CGEventFlags::CGEventFlagAlternate;
    post_key(KeyCode::SPACE, flags, true);
    thread::sleep(Duration::from_millis(hold_ms));
    post_key(KeyCode::SPACE, flags, false);
    release_modifiers();
    thread::sleep(Duration::from_millis(120));
}

fn short_control_option_space() {
    hold_control_option_space(80);
}

fn focused_ax_value() -> Result<String, String> {
    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return Err("AXUIElementCreateSystemWide returned null".into());
        }

        let mut focused: AXUIElementRef = std::ptr::null_mut();
        let focused_attr = CFString::new(kAXFocusedUIElementAttribute);
        let err = AXUIElementCopyAttributeValue(
            system,
            focused_attr.as_concrete_TypeRef(),
            &mut focused as *mut _ as *mut _,
        );
        CFRelease(system as _);
        if err != kAXErrorSuccess || focused.is_null() {
            return Err(format!("no focused UI element (ax error {err})"));
        }

        let mut value_ref = std::ptr::null();
        let value_attr = CFString::new(kAXValueAttribute);
        let err = AXUIElementCopyAttributeValue(
            focused,
            value_attr.as_concrete_TypeRef(),
            &mut value_ref,
        );
        CFRelease(focused as _);
        if err != kAXErrorSuccess || value_ref.is_null() {
            return Err(format!("focused element has no AXValue (ax error {err})"));
        }

        let cf_str = CFString::wrap_under_create_rule(value_ref as _);
        Ok(cf_str.to_string())
    }
}

fn activate_text_edit() -> Result<(), String> {
    let status = Command::new("open")
        .args(["-a", "TextEdit"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("failed to open TextEdit".into());
    }
    thread::sleep(Duration::from_millis(900));

    let cmd = CGEventFlags::CGEventFlagCommand;
    // New document, force plain text, select all, clear.
    tap_key(KeyCode::ANSI_N, cmd);
    thread::sleep(Duration::from_millis(400));
    tap_key(KeyCode::ANSI_T, cmd | CGEventFlags::CGEventFlagShift);
    thread::sleep(Duration::from_millis(200));
    tap_key(KeyCode::ANSI_A, cmd);
    thread::sleep(Duration::from_millis(80));
    tap_key(KeyCode::DELETE, CGEventFlags::CGEventFlagNull);
    release_modifiers();
    thread::sleep(Duration::from_millis(200));
    Ok(())
}

#[tauri::command]
pub async fn run_focus_abc_probe(app: AppHandle) -> Result<FocusProbeReport, String> {
    tauri::async_runtime::spawn_blocking(move || run_focus_abc_probe_blocking(app))
        .await
        .map_err(|e| e.to_string())?
}

pub fn run_focus_abc_probe_blocking(app: AppHandle) -> Result<FocusProbeReport, String> {
    let trusted = ensure_accessibility_prompt();
    if !trusted {
        return Ok(FocusProbeReport {
            ok: false,
            accessibility_trusted: false,
            text_edit_activated: false,
            typed_value: String::new(),
            expected_value: "ABC".into(),
            overlay_show_stole_frontmost: false,
            frontmost_before_overlay: String::new(),
            frontmost_during_overlay: String::new(),
            frontmost_after_overlay: String::new(),
            typing_ok: false,
            focus_ok: false,
            message:
                "请在系统设置 → 隐私与安全性 → 辅助功能 中允许 Luozi，然后点一次「自动测焦点」"
                    .into(),
        });
    }

    let overlay = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;

    activate_text_edit()?;
    let front = frontmost_name();
    let text_edit_activated = is_text_edit(&front);

    type_unicode("A");
    thread::sleep(Duration::from_millis(150));

    let before = frontmost_name();
    // Drive overlay via window API (focus risk) and via synthesized hotkey (real path).
    hold_control_option_space(1000);
    let _ = overlay.show();
    thread::sleep(Duration::from_millis(250));
    let during = frontmost_name();
    let _ = overlay.hide();
    thread::sleep(Duration::from_millis(200));
    let after = frontmost_name();

    let stole = {
        let d = during.to_lowercase();
        d.contains("luozi") && !before.to_lowercase().contains("luozi")
    };

    type_unicode("B");
    thread::sleep(Duration::from_millis(150));

    short_control_option_space();
    let _ = overlay.show();
    thread::sleep(Duration::from_millis(120));
    let _ = overlay.hide();
    thread::sleep(Duration::from_millis(150));

    type_unicode("C");
    thread::sleep(Duration::from_millis(250));

    let typed_value = focused_ax_value().unwrap_or_else(|e| format!("<read_error:{e}>"));
    let typing_ok = typed_value.replace('\u{fffc}', "").contains("ABC");
    let focus_ok = !stole && is_text_edit(&before) && is_text_edit(&during) && is_text_edit(&after);
    let ok = typing_ok && focus_ok && text_edit_activated;

    let message = if ok {
        "pass: TextEdit kept focus; value contains ABC".into()
    } else if stole {
        "fail: showing overlay stole frontmost focus to Luozi".into()
    } else if focus_ok && !typing_ok {
        format!(
            "partial: focus kept on TextEdit, but typed value was `{typed_value}` (expected ABC)"
        )
    } else {
        format!("fail: focus_ok={focus_ok}, typed=`{typed_value}`")
    };

    Ok(FocusProbeReport {
        ok,
        accessibility_trusted: true,
        text_edit_activated,
        typed_value,
        expected_value: "ABC".into(),
        overlay_show_stole_frontmost: stole,
        frontmost_before_overlay: before,
        frontmost_during_overlay: during,
        frontmost_after_overlay: after,
        typing_ok,
        focus_ok,
        message,
    })
}
