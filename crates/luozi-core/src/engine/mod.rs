//! Pure ASR routing decisions. No I/O, no network, no keys.

use crate::config::AsrMode;

/// Which backend should attempt transcription.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AsrBackendChoice {
    Local,
    Cloud,
    NeedConfig { reason: &'static str },
}

/// Initial route before attempting local/cloud.
///
/// Spec §8.3: Auto + local ready → Local only (no upload).
pub fn route_asr(mode: AsrMode, local_ready: bool, cloud_ready: bool) -> AsrBackendChoice {
    match mode {
        AsrMode::LocalOnly => {
            if local_ready {
                AsrBackendChoice::Local
            } else {
                AsrBackendChoice::NeedConfig {
                    reason: "local_not_ready",
                }
            }
        }
        AsrMode::CloudOnly => {
            if cloud_ready {
                AsrBackendChoice::Cloud
            } else {
                AsrBackendChoice::NeedConfig {
                    reason: "cloud_not_ready",
                }
            }
        }
        AsrMode::Auto => {
            if local_ready {
                AsrBackendChoice::Local
            } else if cloud_ready {
                AsrBackendChoice::Cloud
            } else {
                AsrBackendChoice::NeedConfig {
                    reason: "no_engine",
                }
            }
        }
    }
}

/// Fallback after local failure / missing model (Auto only uploads if cloud ready).
pub fn route_asr_after_local_failure(
    mode: AsrMode,
    cloud_ready: bool,
) -> AsrBackendChoice {
    match mode {
        AsrMode::Auto if cloud_ready => AsrBackendChoice::Cloud,
        AsrMode::CloudOnly if cloud_ready => AsrBackendChoice::Cloud,
        AsrMode::LocalOnly => AsrBackendChoice::NeedConfig {
            reason: "local_failed",
        },
        AsrMode::Auto => AsrBackendChoice::NeedConfig {
            reason: "cloud_not_ready",
        },
        AsrMode::CloudOnly => AsrBackendChoice::NeedConfig {
            reason: "cloud_not_ready",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_prefers_local_when_ready() {
        assert_eq!(
            route_asr(AsrMode::Auto, true, true),
            AsrBackendChoice::Local
        );
    }

    #[test]
    fn auto_uses_cloud_when_local_missing() {
        assert_eq!(
            route_asr(AsrMode::Auto, false, true),
            AsrBackendChoice::Cloud
        );
    }

    #[test]
    fn auto_needs_config_when_neither() {
        assert_eq!(
            route_asr(AsrMode::Auto, false, false),
            AsrBackendChoice::NeedConfig {
                reason: "no_engine"
            }
        );
    }

    #[test]
    fn local_only_ignores_cloud() {
        assert_eq!(
            route_asr(AsrMode::LocalOnly, false, true),
            AsrBackendChoice::NeedConfig {
                reason: "local_not_ready"
            }
        );
    }

    #[test]
    fn auto_fallback_after_local_failure() {
        assert_eq!(
            route_asr_after_local_failure(AsrMode::Auto, true),
            AsrBackendChoice::Cloud
        );
        assert_eq!(
            route_asr_after_local_failure(AsrMode::LocalOnly, true),
            AsrBackendChoice::NeedConfig {
                reason: "local_failed"
            }
        );
    }

    #[test]
    fn cloud_only_requires_cloud() {
        assert_eq!(
            route_asr(AsrMode::CloudOnly, true, false),
            AsrBackendChoice::NeedConfig {
                reason: "cloud_not_ready"
            }
        );
        assert_eq!(
            route_asr(AsrMode::CloudOnly, false, true),
            AsrBackendChoice::Cloud
        );
    }
}
