use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetToken {
    pub platform: String,
    pub process_id: u32,
    pub window_id: String,
    pub element_id: String,
    pub role: String,
    pub is_secure: bool,
    pub captured_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    SameTarget,
    Changed,
    Unsupported,
    Secure,
}
