//! Lightweight macOS permission probes for the settings UI.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicAuth {
    Authorized,
    Denied,
    Restricted,
    NotDetermined,
    Unknown,
}

impl MicAuth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authorized => "authorized",
            Self::Denied => "denied",
            Self::Restricted => "restricted",
            Self::NotDetermined => "notDetermined",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_ready(self) -> bool {
        matches!(self, Self::Authorized)
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::MicAuth;
    use objc2::runtime::AnyClass;
    use objc2_foundation::NSString;

    #[link(name = "AVFoundation", kind = "framework")]
    extern "C" {}

    /// AVMediaTypeAudio legacy four-char value used by AVFoundation.
    const AV_MEDIA_TYPE_AUDIO: &str = "soun";

    pub fn microphone_authorization() -> MicAuth {
        // AVAuthorizationStatus: NotDetermined=0 Restricted=1 Denied=2 Authorized=3
        unsafe {
            let Some(cls) = AnyClass::get(c"AVCaptureDevice") else {
                return MicAuth::Unknown;
            };
            let media = NSString::from_str(AV_MEDIA_TYPE_AUDIO);
            let status: isize =
                objc2::msg_send![cls, authorizationStatusForMediaType: &*media];
            match status {
                0 => MicAuth::NotDetermined,
                1 => MicAuth::Restricted,
                2 => MicAuth::Denied,
                3 => MicAuth::Authorized,
                _ => MicAuth::Unknown,
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn microphone_authorization() -> MicAuth {
    macos::microphone_authorization()
}

#[cfg(not(target_os = "macos"))]
pub fn microphone_authorization() -> MicAuth {
    MicAuth::Unknown
}
