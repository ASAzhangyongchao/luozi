use std::time::{SystemTime, UNIX_EPOCH};

use windows::core::{Interface, Result as WinResult};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationValuePattern,
    TreeScope_Element, UIA_ControlTypePropertyId, UIA_IsPasswordPropertyId,
    UIA_ProcessIdPropertyId, UIA_RuntimeIdPropertyId, UIA_ValuePatternId,
};

use super::target::{TargetToken, ValidationState};

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn ensure_com() -> WinResult<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    Ok(())
}

fn automation() -> WinResult<IUIAutomation> {
    ensure_com()?;
    unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
}

fn runtime_id_string(element: &IUIAutomationElement) -> WinResult<String> {
    unsafe {
        let runtime = element.GetRuntimeId()?;
        let mut parts = Vec::new();
        let len = runtime.Length()?;
        for i in 0..len {
            let value = runtime.GetElement(i)?;
            parts.push(value.to_string());
        }
        Ok(parts.join("."))
    }
}

fn focused_token() -> Result<TargetToken, String> {
    let auto = automation().map_err(|e| e.to_string())?;
    let element = unsafe { auto.GetFocusedElement() }.map_err(|e| e.to_string())?;

    let process_id = unsafe {
        element
            .GetCurrentPropertyValue(UIA_ProcessIdPropertyId)
            .map_err(|e| e.to_string())?
            .cast()? as i32
    };

    let hwnd = unsafe { element.CurrentNativeWindowHandle() }.map_err(|e| e.to_string())?;
    let window_id = format!("hwnd={}", hwnd.0 as isize);

    let element_id = runtime_id_string(&element).map_err(|e| e.to_string())?;
    let control_type = unsafe {
        element
            .GetCurrentPropertyValue(UIA_ControlTypePropertyId)
            .map_err(|e| e.to_string())?
            .to_string()
    };
    let is_secure = unsafe {
        element
            .GetCurrentPropertyValue(UIA_IsPasswordPropertyId)
            .map_err(|e| e.to_string())?
            .is_yes()
    };

    let _ = TreeScope_Element; // keep feature surface stable for future tree walks

    Ok(TargetToken {
        platform: "windows".into(),
        process_id: process_id as u32,
        window_id,
        element_id,
        role: control_type,
        is_secure,
        captured_at_ms: now_ms(),
    })
}

pub fn capture_target() -> Result<TargetToken, String> {
    focused_token()
}

pub fn validate_target(token: &TargetToken) -> Result<ValidationState, String> {
    if token.is_secure {
        return Ok(ValidationState::Secure);
    }
    let current = match focused_token() {
        Ok(v) => v,
        Err(_) => return Ok(ValidationState::Unsupported),
    };
    if current.is_secure {
        return Ok(ValidationState::Secure);
    }
    if current.process_id == token.process_id
        && current.window_id == token.window_id
        && current.element_id == token.element_id
    {
        Ok(ValidationState::SameTarget)
    } else {
        Ok(ValidationState::Changed)
    }
}

pub fn deliver_probe(token: &TargetToken) -> Result<ValidationState, String> {
    let state = validate_target(token)?;
    match state {
        ValidationState::SameTarget => {}
        other => return Ok(other),
    }
    if token.is_secure {
        return Ok(ValidationState::Secure);
    }

    let auto = automation().map_err(|e| e.to_string())?;
    let element = unsafe { auto.GetFocusedElement() }.map_err(|e| e.to_string())?;
    let is_secure = unsafe {
        element
            .GetCurrentPropertyValue(UIA_IsPasswordPropertyId)
            .map_err(|e| e.to_string())?
            .is_yes()
    };
    if is_secure {
        return Ok(ValidationState::Secure);
    }

    let pattern = unsafe {
        match element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
            Ok(p) => p,
            Err(_) => return Ok(ValidationState::Unsupported),
        }
    };

    // Only write a short probe string; never read password contents.
    match unsafe { pattern.SetValue(&windows::core::BSTR::from("落字测试")) } {
        Ok(()) => Ok(ValidationState::SameTarget),
        Err(_) => Ok(ValidationState::Unsupported),
    }
}

#[allow(dead_code)]
fn _hwnd_type_check(hwnd: HWND) -> isize {
    hwnd.0 as isize
}
