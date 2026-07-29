use std::ffi::c_void;
use std::time::{SystemTime, UNIX_EPOCH};

use accessibility_sys::{
    kAXChildrenAttribute, kAXErrorSuccess, kAXFocusedApplicationAttribute, kAXFocusedAttribute,
    kAXFocusedUIElementAttribute, kAXFocusedWindowAttribute, kAXIdentifierAttribute,
    kAXPositionAttribute, kAXRoleAttribute, kAXSecureTextFieldSubrole, kAXSelectedTextAttribute,
    kAXSizeAttribute, kAXSubroleAttribute, kAXTextAreaRole, kAXTextFieldRole,
    kAXTrustedCheckOptionPrompt, kAXValueAttribute, kAXValueTypeCGPoint, kAXValueTypeCGSize,
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXUIElementCreateSystemWide, AXUIElementGetPid, AXUIElementRef, AXUIElementSetAttributeValue,
    AXValueGetType, AXValueGetValue, AXValueRef,
};
use core_foundation::array::CFArray;
use core_foundation::base::{CFRelease, CFRetain, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::NSWorkspace;
use std::thread;
use std::time::Duration;

use super::target::{TargetToken, ValidationState};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

fn ensure_accessibility() -> Result<(), String> {
    // NEVER prompt on the hot path — a modal on the AppKit main thread beachballs
    // the app (overlay stuck, Esc dead). Tray 「权限」can open System Settings.
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let pairs = [(key.as_CFType(), CFBoolean::false_value().as_CFType())];
        let dict = CFDictionary::from_CFType_pairs(&pairs);
        if AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) {
            Ok(())
        } else {
            Err("Accessibility permission required for Luozi".into())
        }
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

unsafe fn copy_attr(element: AXUIElementRef, attr: &str) -> Result<*const c_void, String> {
    let mut value = std::ptr::null();
    let name = CFString::new(attr);
    let err = AXUIElementCopyAttributeValue(element, name.as_concrete_TypeRef(), &mut value);
    if err != kAXErrorSuccess || value.is_null() {
        Err(format!("AX attr `{attr}` unavailable (code {err})"))
    } else {
        Ok(value)
    }
}

unsafe fn copy_string_attr(element: AXUIElementRef, attr: &str) -> Option<String> {
    let value = copy_attr(element, attr).ok()?;
    let cf = CFString::wrap_under_create_rule(value as _);
    Some(cf.to_string())
}

unsafe fn geometry_id(element: AXUIElementRef) -> String {
    let mut parts = Vec::new();

    if let Ok(pos_ref) = copy_attr(element, kAXPositionAttribute) {
        let ax_value = pos_ref as AXValueRef;
        if AXValueGetType(ax_value) == kAXValueTypeCGPoint {
            let mut point = CGPoint { x: 0.0, y: 0.0 };
            if AXValueGetValue(
                ax_value,
                kAXValueTypeCGPoint,
                &mut point as *mut _ as *mut c_void,
            ) {
                parts.push(format!("p={:.1},{:.1}", point.x, point.y));
            }
        }
        CFRelease(pos_ref);
    }

    if let Ok(size_ref) = copy_attr(element, kAXSizeAttribute) {
        let ax_value = size_ref as AXValueRef;
        if AXValueGetType(ax_value) == kAXValueTypeCGSize {
            let mut size = CGSize {
                width: 0.0,
                height: 0.0,
            };
            if AXValueGetValue(
                ax_value,
                kAXValueTypeCGSize,
                &mut size as *mut _ as *mut c_void,
            ) {
                parts.push(format!("s={:.1}x{:.1}", size.width, size.height));
            }
        }
        CFRelease(size_ref);
    }

    if parts.is_empty() {
        "geom=unknown".into()
    } else {
        parts.join(";")
    }
}

unsafe fn element_identity(element: AXUIElementRef, role: &str, subrole: &str) -> String {
    let identifier = copy_string_attr(element, kAXIdentifierAttribute).unwrap_or_default();
    let geom = geometry_id(element);
    format!("role={role};subrole={subrole};id={identifier};{geom}")
}

struct FocusedAx {
    pid: i32,
    window_id: String,
    element_id: String,
    role: String,
    is_secure: bool,
    focused: AXUIElementRef,
    app: AXUIElementRef,
    window: Option<AXUIElementRef>,
}

impl Drop for FocusedAx {
    fn drop(&mut self) {
        unsafe {
            if !self.focused.is_null() {
                CFRelease(self.focused as _);
            }
            if let Some(window) = self.window {
                if !window.is_null() {
                    CFRelease(window as _);
                }
            }
            if !self.app.is_null() {
                CFRelease(self.app as _);
            }
        }
    }
}

fn frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid > 0 {
        Some(pid)
    } else {
        None
    }
}

pub(crate) fn current_frontmost_pid() -> Option<i32> {
    frontmost_pid()
}

pub(crate) fn accessibility_is_trusted() -> bool {
    ensure_accessibility().is_ok()
}

/// Bring another app to front so caret paste / typing lands there (not in Luozi).
pub(crate) fn activate_pid(pid: i32) -> Result<(), String> {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Err(format!("no_running_app_for_pid:{pid}"));
    };
    let ok = app.activateWithOptions(
        NSApplicationActivationOptions::ActivateIgnoringOtherApps
            | NSApplicationActivationOptions::ActivateAllWindows,
    );
    if ok {
        Ok(())
    } else {
        Err(format!("activate_pid_failed:{pid}"))
    }
}

/// Type Unicode into the focused field via CGEvent (works in many Electron editors).
pub(crate) fn type_text_via_cg_events(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "cg_event_source_failed".to_string())?;

    // CGEventKeyboardSetUnicodeString accepts limited UTF-16 per event — chunk it.
    let utf16: Vec<u16> = text.encode_utf16().collect();
    for chunk in utf16.chunks(20) {
        let down = CGEvent::new_keyboard_event(source.clone(), 0, true)
            .map_err(|_| "cg_unicode_down_failed".to_string())?;
        down.set_string_from_utf16_unchecked(chunk);
        down.post(CGEventTapLocation::HID);

        let up = CGEvent::new_keyboard_event(source.clone(), 0, false)
            .map_err(|_| "cg_unicode_up_failed".to_string())?;
        up.post(CGEventTapLocation::HID);
        thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}

fn trusted_status_label() -> &'static str {
    match ensure_accessibility() {
        Ok(()) => "trusted",
        Err(_) => "NOT_trusted",
    }
}

unsafe fn capture_focused_from_app(app: AXUIElementRef) -> Result<FocusedAx, String> {
    let focused_value = copy_attr(app, kAXFocusedUIElementAttribute).or_else(|_| {
        // Some apps expose focus only via system-wide after app resolve.
        let system = AXUIElementCreateSystemWide();
        let result = copy_attr(system, kAXFocusedUIElementAttribute);
        CFRelease(system as _);
        result
    })?;
    let focused = focused_value as AXUIElementRef;

    let window = copy_attr(app, kAXFocusedWindowAttribute)
        .ok()
        .map(|v| v as AXUIElementRef);

    let mut pid: i32 = 0;
    let pid_err = AXUIElementGetPid(focused, &mut pid);
    if pid_err != kAXErrorSuccess || pid <= 0 {
        let app_pid_err = AXUIElementGetPid(app, &mut pid);
        if app_pid_err != kAXErrorSuccess || pid <= 0 {
            CFRelease(focused as _);
            if let Some(window) = window {
                CFRelease(window as _);
            }
            return Err(format!(
                "AXUIElementGetPid failed ({pid_err}/{app_pid_err})"
            ));
        }
    }

    // Retain app for FocusedAx drop ownership.
    CFRetain(app as _);

    let role = copy_string_attr(focused, kAXRoleAttribute).unwrap_or_else(|| "unknown".into());
    let subrole = copy_string_attr(focused, kAXSubroleAttribute).unwrap_or_default();
    let is_secure = subrole == kAXSecureTextFieldSubrole
        || role.to_lowercase().contains("secure")
        || subrole.to_lowercase().contains("secure");

    let window_id = if let Some(window) = window {
        let w_role =
            copy_string_attr(window, kAXRoleAttribute).unwrap_or_else(|| "AXWindow".into());
        let w_sub = copy_string_attr(window, kAXSubroleAttribute).unwrap_or_default();
        format!("pid={pid};{}", element_identity(window, &w_role, &w_sub))
    } else {
        format!("pid={pid};window=none")
    };
    let element_id = format!("pid={pid};{}", element_identity(focused, &role, &subrole));

    Ok(FocusedAx {
        pid,
        window_id,
        element_id,
        role,
        is_secure,
        focused,
        app,
        window,
    })
}

unsafe fn capture_focused() -> Result<FocusedAx, String> {
    eprintln!("luozi: AX status={}", trusted_status_label());

    let system = AXUIElementCreateSystemWide();
    if !system.is_null() {
        match copy_attr(system, kAXFocusedApplicationAttribute) {
            Ok(app_value) => {
                let app = app_value as AXUIElementRef;
                CFRelease(system as _);
                match capture_focused_from_app(app) {
                    Ok(v) => {
                        CFRelease(app as _);
                        return Ok(v);
                    }
                    Err(e) => {
                        eprintln!("luozi: AX focused-app path failed: {e}");
                        CFRelease(app as _);
                    }
                }
            }
            Err(e) => {
                eprintln!("luozi: AXFocusedApplication: {e}");
                CFRelease(system as _);
            }
        }
    }

    // Path B: NSWorkspace frontmost app → AX application element.
    let pid = frontmost_pid().ok_or_else(|| {
        format!(
            "no focused AX app and no frontmost app (AX={})",
            trusted_status_label()
        )
    })?;
    eprintln!("luozi: AX fallback frontmost pid={pid}");
    let app = AXUIElementCreateApplication(pid);
    if app.is_null() {
        return Err(format!("AXUIElementCreateApplication({pid}) returned null"));
    }
    match capture_focused_from_app(app) {
        Ok(v) => {
            CFRelease(app as _);
            Ok(v)
        }
        Err(e) => {
            CFRelease(app as _);
            Err(e)
        }
    }
}

/// Write already happened; synthesize ⌘V into the frontmost app so text lands at caret.
pub(crate) fn paste_via_cmd_v() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "cg_event_source_failed".to_string())?;
    let key = KeyCode::ANSI_V;
    let flags = CGEventFlags::CGEventFlagCommand;

    let down = CGEvent::new_keyboard_event(source.clone(), key, true)
        .map_err(|_| "cg_key_down_failed".to_string())?;
    down.set_flags(flags);
    down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(20));

    let up = CGEvent::new_keyboard_event(source, key, false)
        .map_err(|_| "cg_key_up_failed".to_string())?;
    up.set_flags(flags);
    up.post(CGEventTapLocation::HID);
    Ok(())
}

fn role_supports_text_insert(role: &str) -> bool {
    role == kAXTextFieldRole
        || role == kAXTextAreaRole
        || role == "AXComboBox"
        || role == "AXWebArea"
        || role == "AXGroup"
        || role == "AXScrollArea"
        || role == "AXDocument"
        || role.to_lowercase().contains("text")
}

unsafe fn element_is_secure(element: AXUIElementRef) -> bool {
    let role = copy_string_attr(element, kAXRoleAttribute).unwrap_or_default();
    let subrole = copy_string_attr(element, kAXSubroleAttribute).unwrap_or_default();
    subrole == kAXSecureTextFieldSubrole
        || role.to_lowercase().contains("secure")
        || subrole.to_lowercase().contains("secure")
}

unsafe fn find_secure_in_tree(root: AXUIElementRef, depth: usize) -> Option<AXUIElementRef> {
    if depth > 14 {
        return None;
    }
    if element_is_secure(root) {
        CFRetain(root as _);
        return Some(root);
    }
    let children_value = copy_attr(root, kAXChildrenAttribute).ok()?;
    let children: CFArray<*const c_void> = CFArray::wrap_under_create_rule(children_value as _);
    for i in 0..children.len() {
        if let Some(child_ref) = children.get(i) {
            let child = (*child_ref) as AXUIElementRef;
            if let Some(found) = find_secure_in_tree(child, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}

unsafe fn element_frame_center(element: AXUIElementRef) -> Option<(f64, f64)> {
    let pos_ref = copy_attr(element, kAXPositionAttribute).ok()?;
    let size_ref = match copy_attr(element, kAXSizeAttribute) {
        Ok(v) => v,
        Err(_) => {
            CFRelease(pos_ref);
            return None;
        }
    };
    let mut point = CGPoint::default();
    let mut size = CGSize::default();
    let ok_pos = AXValueGetType(pos_ref as AXValueRef) == kAXValueTypeCGPoint
        && AXValueGetValue(
            pos_ref as AXValueRef,
            kAXValueTypeCGPoint,
            &mut point as *mut _ as *mut c_void,
        );
    let ok_size = AXValueGetType(size_ref as AXValueRef) == kAXValueTypeCGSize
        && AXValueGetValue(
            size_ref as AXValueRef,
            kAXValueTypeCGSize,
            &mut size as *mut _ as *mut c_void,
        );
    CFRelease(pos_ref);
    CFRelease(size_ref);
    if !ok_pos || !ok_size {
        return None;
    }
    Some((point.x + size.width / 2.0, point.y + size.height / 2.0))
}

fn click_screen_point(x: f64, y: f64) {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("event source");
    if let Ok(move_event) = CGEvent::new_mouse_event(
        source.clone(),
        core_graphics::event::CGEventType::MouseMoved,
        core_graphics::geometry::CGPoint::new(x, y),
        core_graphics::event::CGMouseButton::Left,
    ) {
        move_event.post(CGEventTapLocation::HID);
    }
    thread::sleep(Duration::from_millis(40));
    if let Ok(down) = CGEvent::new_mouse_event(
        source.clone(),
        core_graphics::event::CGEventType::LeftMouseDown,
        core_graphics::geometry::CGPoint::new(x, y),
        core_graphics::event::CGMouseButton::Left,
    ) {
        down.post(CGEventTapLocation::HID);
    }
    thread::sleep(Duration::from_millis(40));
    if let Ok(up) = CGEvent::new_mouse_event(
        source,
        core_graphics::event::CGEventType::LeftMouseUp,
        core_graphics::geometry::CGPoint::new(x, y),
        core_graphics::event::CGMouseButton::Left,
    ) {
        up.post(CGEventTapLocation::HID);
    }
}

/// Find an AXSecureTextField under pid and click its center to take focus.
pub(crate) fn focus_first_secure_field_in_pid(pid: i32) -> Result<bool, String> {
    ensure_accessibility()?;
    unsafe {
        let app_from_pid = AXUIElementCreateApplication(pid);
        if app_from_pid.is_null() {
            return Err("AXUIElementCreateApplication failed".into());
        }

        let found = find_secure_in_tree(app_from_pid, 0);
        CFRelease(app_from_pid as _);
        let Some(secure) = found else {
            return Ok(false);
        };

        let center = element_frame_center(secure);
        let attr = CFString::new(kAXFocusedAttribute);
        let _ = AXUIElementSetAttributeValue(
            secure,
            attr.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
        CFRelease(secure as _);

        if let Some((x, y)) = center {
            click_screen_point(x, y);
            thread::sleep(Duration::from_millis(200));
            return Ok(true);
        }
        Ok(false)
    }
}

pub(crate) fn capture_target() -> Result<TargetToken, String> {
    ensure_accessibility()?;
    unsafe {
        let focused = capture_focused()?;
        Ok(TargetToken {
            platform: "macos".into(),
            process_id: focused.pid as u32,
            window_id: focused.window_id.clone(),
            element_id: focused.element_id.clone(),
            role: focused.role.clone(),
            is_secure: focused.is_secure,
            captured_at_ms: now_ms(),
        })
    }
}

pub(crate) fn validate_target(token: &TargetToken) -> Result<ValidationState, String> {
    ensure_accessibility()?;
    if token.is_secure {
        return Ok(ValidationState::Secure);
    }

    unsafe {
        let current = match capture_focused() {
            Ok(v) => v,
            Err(_) => return Ok(ValidationState::Unsupported),
        };

        if current.is_secure {
            return Ok(ValidationState::Secure);
        }

        if current.pid as u32 == token.process_id
            && current.window_id == token.window_id
            && current.element_id == token.element_id
        {
            Ok(ValidationState::SameTarget)
        } else {
            Ok(ValidationState::Changed)
        }
    }
}

/// Read AXValue / AXSelectedText from the current focused element (best-effort).
pub(crate) fn read_focused_field_text() -> Option<String> {
    ensure_accessibility().ok()?;
    unsafe {
        let current = capture_focused().ok()?;
        if let Some(v) = copy_string_attr(current.focused, kAXValueAttribute) {
            if !v.is_empty() {
                return Some(v);
            }
        }
        copy_string_attr(current.focused, kAXSelectedTextAttribute)
    }
}

/// True when focused field value contains `needle` (post-insert verification).
pub(crate) fn focused_field_contains(needle: &str) -> bool {
    let needle = needle.trim();
    if needle.is_empty() {
        return false;
    }
    match read_focused_field_text() {
        Some(v) => v.contains(needle),
        None => false,
    }
}

pub(crate) fn deliver_probe(token: &TargetToken) -> Result<ValidationState, String> {
    deliver_text(token, "落字测试")
}

/// Insert `text` into the focused field when validation still matches `token`.
pub(crate) fn deliver_text(token: &TargetToken, text: &str) -> Result<ValidationState, String> {
    let state = validate_target(token)?;
    match state {
        ValidationState::SameTarget => {}
        other => return Ok(other),
    }

    if token.is_secure {
        return Ok(ValidationState::Secure);
    }

    unsafe {
        let current = capture_focused()?;
        if current.is_secure {
            return Ok(ValidationState::Secure);
        }
        if !role_supports_text_insert(&current.role) {
            return Ok(ValidationState::Unsupported);
        }

        let cf_text = CFString::new(text);
        let attr = CFString::new(kAXSelectedTextAttribute);
        let err = AXUIElementSetAttributeValue(
            current.focused,
            attr.as_concrete_TypeRef(),
            cf_text.as_CFTypeRef(),
        );
        if err == kAXErrorSuccess {
            // SetAttribute success ≠ visible insert in Electron; verify when possible.
            thread::sleep(Duration::from_millis(40));
            if focused_field_contains(text) {
                return Ok(ValidationState::SameTarget);
            }
            eprintln!("luozi: AXSelectedText set ok but value verify failed");
        }
        // Fallback: replace entire value (some fields reject selected-text sets).
        let value_attr = CFString::new(kAXValueAttribute);
        let err2 = AXUIElementSetAttributeValue(
            current.focused,
            value_attr.as_concrete_TypeRef(),
            cf_text.as_CFTypeRef(),
        );
        if err2 == kAXErrorSuccess {
            thread::sleep(Duration::from_millis(40));
            if focused_field_contains(text) {
                return Ok(ValidationState::SameTarget);
            }
            eprintln!("luozi: AXValue set ok but value verify failed");
        }
        eprintln!("luozi: AX insert failed selected={err} value={err2}; caller may type/paste");
        Ok(ValidationState::Unsupported)
    }
}
