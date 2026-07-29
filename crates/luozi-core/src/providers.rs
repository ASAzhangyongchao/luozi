//! Built-in cloud provider presets (BYOK). Secrets stay in Keychain.

use serde::{Deserialize, Serialize};

use crate::config::{
    CloudAsrConfig, TextAiConfig, GROQ_DEFAULT_BASE_URL, GROQ_DEFAULT_MODEL, GROQ_PROVIDER_ID,
    GROQ_TEXT_AI_DEFAULT_MODEL, GROQ_TEXT_AI_PROVIDER_ID,
};

/// How cloud ASR talks to the vendor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AsrProtocol {
    /// OpenAI-compatible `POST …/audio/transcriptions` (Groq Whisper, etc.).
    #[default]
    OpenaiTranscriptions,
    /// OpenAI-compatible chat + `input_audio` (Qwen3-ASR / 小米 MiMo ASR).
    QwenAsrChat,
    /// 豆包 / 火山引擎录音文件识别 AUC（APP ID + Access Token）。
    DoubaoAuc,
    /// Listed for UI honesty; no public ASR API yet (e.g. 小米 MiMo).
    Unsupported,
}

impl AsrProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiTranscriptions => "openaiTranscriptions",
            Self::QwenAsrChat => "qwenAsrChat",
            Self::DoubaoAuc => "doubaoAuc",
            Self::Unsupported => "unsupported",
        }
    }

    pub fn supports_cloud(self) -> bool {
        !matches!(self, Self::Unsupported)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsrProviderPreset {
    pub id: &'static str,
    pub label_zh: &'static str,
    pub help_zh: &'static str,
    pub protocol: AsrProtocol,
    pub base_url: &'static str,
    pub model: &'static str,
    pub credential_ref: &'static str,
    pub key_prompt_zh: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextAiProviderPreset {
    pub id: &'static str,
    pub label_zh: &'static str,
    pub help_zh: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub credential_ref: &'static str,
    pub key_prompt_zh: &'static str,
}

pub const ASR_PROVIDERS: &[AsrProviderPreset] = &[
    AsrProviderPreset {
        id: GROQ_PROVIDER_ID,
        label_zh: "Groq Whisper",
        help_zh: "海外 Groq 的 Whisper 转写。API Key 在 console.groq.com 创建；音频经 HTTPS 上传到 api.groq.com。",
        protocol: AsrProtocol::OpenaiTranscriptions,
        base_url: GROQ_DEFAULT_BASE_URL,
        model: GROQ_DEFAULT_MODEL,
        credential_ref: "asr.groq",
        key_prompt_zh: "粘贴 Groq API Key（仅存本机钥匙串）",
    },
    AsrProviderPreset {
        id: "qwen",
        label_zh: "千问 ASR",
        help_zh: "阿里云百炼 / DashScope 的 Qwen3-ASR。使用中国区兼容模式；Key 在百炼控制台创建。",
        protocol: AsrProtocol::QwenAsrChat,
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        model: "qwen3-asr-flash",
        credential_ref: "asr.qwen",
        key_prompt_zh: "粘贴千问 / DashScope API Key（仅存本机钥匙串）",
    },
    AsrProviderPreset {
        id: "doubao",
        label_zh: "豆包 ASR",
        help_zh: "火山引擎豆包语音（录音文件识别）。凭证是 APP ID + Access Token，粘贴格式：APPID:TOKEN。",
        protocol: AsrProtocol::DoubaoAuc,
        base_url: "https://openspeech.bytedance.com",
        model: "bigmodel",
        credential_ref: "asr.doubao",
        key_prompt_zh: "粘贴豆包语音凭证，格式 APPID:AccessToken（仅存本机钥匙串）",
    },
    AsrProviderPreset {
        id: "xiaomi",
        label_zh: "小米 MiMo ASR",
        help_zh: "小米 MiMo‑V2.5‑ASR（OpenAI 兼容 chat + input_audio）。Key 在 mimo.mi.com 控制台创建。",
        protocol: AsrProtocol::QwenAsrChat,
        base_url: "https://api.xiaomimimo.com/v1",
        model: "mimo-v2.5-asr",
        credential_ref: "asr.xiaomi",
        key_prompt_zh: "粘贴小米 MiMo API Key（ASR，仅存本机钥匙串）",
    },
];

pub const TEXT_AI_PROVIDERS: &[TextAiProviderPreset] = &[
    TextAiProviderPreset {
        id: GROQ_TEXT_AI_PROVIDER_ID,
        label_zh: "Groq",
        help_zh: "Groq Chat Completions（OpenAI 兼容）。与 ASR Key 分开存放、分开同意。",
        base_url: GROQ_DEFAULT_BASE_URL,
        model: GROQ_TEXT_AI_DEFAULT_MODEL,
        credential_ref: "textai.groq",
        key_prompt_zh: "粘贴 Groq API Key（文本 AI，仅存本机钥匙串）",
    },
    TextAiProviderPreset {
        id: "textai.qwen",
        label_zh: "千问",
        help_zh: "阿里云百炼兼容模式 Chat。默认模型 qwen-plus；Key 与 ASR 分开授权。",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        model: "qwen-plus",
        credential_ref: "textai.qwen",
        key_prompt_zh: "粘贴千问 / DashScope API Key（文本 AI，仅存本机钥匙串）",
    },
    TextAiProviderPreset {
        id: "textai.doubao",
        label_zh: "豆包",
        help_zh: "火山方舟 OpenAI 兼容接口。模型字段需填控制台创建的 Endpoint ID（形如 ep-…）。",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        model: "ep-xxxxxxxx",
        credential_ref: "textai.doubao",
        key_prompt_zh: "粘贴火山方舟 / 豆包 API Key（文本 AI，仅存本机钥匙串）",
    },
    TextAiProviderPreset {
        id: "textai.xiaomi",
        label_zh: "小米 MiMo",
        help_zh: "小米 MiMo OpenAI 兼容 Chat。按量付费 Base：api.xiaomimimo.com；Token Plan 请之后在高级设置改 Base。",
        base_url: "https://api.xiaomimimo.com/v1",
        model: "mimo-v2.5-pro",
        credential_ref: "textai.xiaomi",
        key_prompt_zh: "粘贴小米 MiMo API Key（文本 AI，仅存本机钥匙串）",
    },
];

pub fn asr_preset(id: &str) -> Option<&'static AsrProviderPreset> {
    ASR_PROVIDERS.iter().find(|p| p.id == id)
}

pub fn text_ai_preset(id: &str) -> Option<&'static TextAiProviderPreset> {
    TEXT_AI_PROVIDERS.iter().find(|p| p.id == id)
}

pub fn cloud_asr_from_preset(p: &AsrProviderPreset) -> CloudAsrConfig {
    CloudAsrConfig {
        provider_id: p.id.into(),
        base_url: p.base_url.into(),
        model: p.model.into(),
        credential_ref: p.credential_ref.into(),
        protocol: p.protocol,
    }
}

pub fn text_ai_from_preset(p: &TextAiProviderPreset) -> TextAiConfig {
    TextAiConfig {
        provider_id: p.id.into(),
        base_url: p.base_url.into(),
        model: p.model.into(),
        credential_ref: p.credential_ref.into(),
    }
}

pub fn asr_mode_from_str(s: &str) -> Option<crate::config::AsrMode> {
    match s {
        "auto" | "Auto" => Some(crate::config::AsrMode::Auto),
        "localOnly" | "LocalOnly" => Some(crate::config::AsrMode::LocalOnly),
        "cloudOnly" | "CloudOnly" => Some(crate::config::AsrMode::CloudOnly),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_presets_cover_requested_vendors() {
        let ids: Vec<_> = ASR_PROVIDERS.iter().map(|p| p.id).collect();
        assert!(ids.contains(&"groq"));
        assert!(ids.contains(&"qwen"));
        assert!(ids.contains(&"doubao"));
        assert!(ids.contains(&"xiaomi"));
        assert!(asr_preset("xiaomi").unwrap().protocol.supports_cloud());
        assert_eq!(asr_preset("xiaomi").unwrap().model, "mimo-v2.5-asr");
    }

    #[test]
    fn text_presets_cover_requested_vendors() {
        assert!(text_ai_preset("textai.qwen").is_some());
        assert!(text_ai_preset("textai.doubao").is_some());
        assert!(text_ai_preset("textai.xiaomi").is_some());
    }
}
