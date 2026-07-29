# Luozi P1 AI Service Center Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 交付页面内 Key、内置与自定义服务、连接测试、官方教程、能力路由和旧配置迁移组成的一站式 AI 服务中心。

**Architecture:** `luozi-core` 定义无秘密的连接和能力模型；`ConnectionStore` 将连接配置与钥匙串引用分离；`ConnectionTester` 先用内存中的 Key 验证，再原子写入钥匙串和配置；设置前端只通过窄 Tauri 命令读取 DTO、测试并保存连接。

**Tech Stack:** Rust 2021、Tauri 2、Serde、ureq、macOS Keychain、TypeScript、Vitest、原生 HTML/CSS。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §5

**Prerequisite:** P0 已完成并通过全量验证。

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `crates/luozi-core/src/connections.rs` | Create | 无秘密连接、能力、状态和路由类型 |
| `crates/luozi-core/src/providers.rs` | Modify | 统一内置服务能力、默认值和帮助链接 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出连接模型 |
| `src-tauri/src/session/connection_store.rs` | Create | 连接 JSON、旧配置迁移和原子保存 |
| `src-tauri/src/session/connection_test.rs` | Create | 认证、模型、协议和延迟测试 |
| `src-tauri/src/session/credentials.rs` | Modify | 原子替换、删除和存在性检查 |
| `src-tauri/src/session/cloud.rs` | Modify | 支持显式 Key 的无持久化 ASR 探测 |
| `src-tauri/src/session/text_ai.rs` | Modify | 支持显式 Key 的最小文字探测 |
| `src-tauri/src/session/settings_api.rs` | Modify | AI 服务 DTO |
| `src-tauri/src/session/mod.rs` | Modify | 注册连接模块 |
| `src-tauri/src/lib.rs` | Modify | 暴露连接测试、保存、切换、删除命令 |
| `src-tauri/resources/provider-guides.zh-CN.json` | Create | 官方控制台和文档元数据 |
| `settings.html` | Modify | AI 服务一级页面与连接对话框 |
| `src/settings/types.ts` | Create | 设置与连接 DTO |
| `src/settings/ai-services.ts` | Create | 连接页渲染和交互 |
| `src/settings.ts` | Modify | 入口和刷新编排 |
| `src/styles-settings.css` | Modify | 连接卡、状态和响应式表单 |
| `tests/frontend/ai-services-model.test.ts` | Create | 连接状态和能力文案测试 |
| `docs/spikes/p1-ai-services-manual.md` | Create | Key、迁移和错误映射实机验收 |

---

### Task 1: 定义统一连接与能力模型

**Files:**

- Create: `crates/luozi-core/src/connections.rs`
- Modify: `crates/luozi-core/src/providers.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写失败测试**

创建 `crates/luozi-core/src/connections.rs`：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiCapability {
    SpeechRecognition,
    TextCleanup,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityBinding {
    pub model: String,
    pub protocol: String,
    pub credential_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnection {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub base_url: String,
    pub asr: Option<CapabilityBinding>,
    pub text_cleanup: Option<CapabilityBinding>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiServiceConfig {
    pub connections: Vec<AiConnection>,
    pub active_asr_connection_id: Option<String>,
    pub active_text_connection_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_connection_must_support_requested_capability() {
        let config = AiServiceConfig {
            connections: vec![AiConnection {
                id: "deepseek-main".into(),
                provider_id: "deepseek".into(),
                label: "DeepSeek".into(),
                base_url: "https://api.deepseek.com".into(),
                asr: None,
                text_cleanup: Some(CapabilityBinding {
                    model: "deepseek-chat".into(),
                    protocol: "openaiChat".into(),
                    credential_ref: "connection.deepseek-main.text".into(),
                }),
            }],
            active_asr_connection_id: Some("deepseek-main".into()),
            active_text_connection_id: Some("deepseek-main".into()),
        };
        assert_eq!(
            config.resolve(AiCapability::SpeechRecognition),
            Err("connection_capability_mismatch")
        );
        assert!(config.resolve(AiCapability::TextCleanup).is_ok());
    }
}
```

- [ ] **Step 2: 确认测试红灯**

Run:

```bash
cargo test -p luozi-core connections
```

Expected：FAIL，缺少 `resolve`。

- [ ] **Step 3: 写最小解析实现**

在 `AiServiceConfig` 后加入：

```rust
impl AiServiceConfig {
    pub fn resolve(&self, capability: AiCapability) -> Result<&AiConnection, &'static str> {
        let id = match capability {
            AiCapability::SpeechRecognition => self.active_asr_connection_id.as_deref(),
            AiCapability::TextCleanup => self.active_text_connection_id.as_deref(),
        }
        .ok_or("connection_not_selected")?;
        let connection = self
            .connections
            .iter()
            .find(|item| item.id == id)
            .ok_or("connection_not_found")?;
        let supported = match capability {
            AiCapability::SpeechRecognition => connection.asr.is_some(),
            AiCapability::TextCleanup => connection.text_cleanup.is_some(),
        };
        supported
            .then_some(connection)
            .ok_or("connection_capability_mismatch")
    }
}
```

从 `lib.rs` 导出：

```rust
mod connections;
pub use connections::{
    AiCapability, AiConnection, AiServiceConfig, CapabilityBinding,
};
```

在 `providers.rs` 中用统一定义替换分散服务默认值；如果旧 UI 仍需要 ASR 或文本 AI 的单能力列表，必须由 `provider_definitions()` 派生，不能再各自维护一份默认值：

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefinition {
    pub id: String,
    pub label_zh: String,
    pub asr: Option<ProviderCapabilityDefinition>,
    pub text_cleanup: Option<ProviderCapabilityDefinition>,
    pub console_url: String,
    pub docs_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilityDefinition {
    pub default_base_url: String,
    pub default_model: String,
    pub protocol: String,
}

pub fn provider_definitions() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "deepseek".into(),
            label_zh: "DeepSeek".into(),
            asr: None,
            text_cleanup: Some(ProviderCapabilityDefinition {
                default_base_url: "https://api.deepseek.com".into(),
                default_model: "deepseek-chat".into(),
                protocol: "openaiChat".into(),
            }),
            console_url: "https://platform.deepseek.com/api_keys".into(),
            docs_url: "https://api-docs.deepseek.com/".into(),
        },
    ]
}
```

Rust 实现必须逐项写出 `groq`、`qwen`、`doubao`、`xiaomi`、`openai`、`deepseek` 和 `custom-openai`，并用测试断言这些 id 与 `provider-guides.zh-CN.json` 完全一致。DeepSeek 的 `asr` 必须为 `None`；自定义 OpenAI 兼容服务的 `console_url` 和 `docs_url` 允许为空字符串，但保存前必须要求用户自行填写 base URL 和模型。

- [ ] **Step 4: 运行测试**

Run:

```bash
cargo test -p luozi-core connections
cargo test -p luozi-core providers
```

Expected：全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/connections.rs crates/luozi-core/src/providers.rs crates/luozi-core/src/lib.rs
git commit -m "feat: define unified AI service connections"
```

---

### Task 2: 建立连接存储和旧配置迁移

**Files:**

- Create: `src-tauri/src/session/connection_store.rs`
- Modify: `src-tauri/src/session/mod.rs`
- Modify: `src-tauri/src/session/config_store.rs`

- [ ] **Step 1: 写迁移测试**

在 `connection_store.rs` 先写：

```rust
use luozi_core::{AiConnection, AiServiceConfig, AppConfig, CapabilityBinding};

pub fn migrate_legacy(cfg: &AppConfig) -> AiServiceConfig {
    let asr_id = format!("{}-asr", cfg.cloud_asr.provider_id);
    let text_id = format!("{}-text", cfg.text_ai.provider_id);
    let asr = AiConnection {
        id: asr_id.clone(),
        provider_id: cfg.cloud_asr.provider_id.clone(),
        label: cfg.cloud_asr.provider_id.clone(),
        base_url: cfg.cloud_asr.base_url.clone(),
        asr: Some(CapabilityBinding {
            model: cfg.cloud_asr.model.clone(),
            protocol: cfg.cloud_asr.protocol.as_str().into(),
            credential_ref: cfg.cloud_asr.credential_ref.clone(),
        }),
        text_cleanup: None,
    };
    let text = AiConnection {
        id: text_id.clone(),
        provider_id: cfg.text_ai.provider_id.clone(),
        label: cfg.text_ai.provider_id.clone(),
        base_url: cfg.text_ai.base_url.clone(),
        asr: None,
        text_cleanup: Some(CapabilityBinding {
            model: cfg.text_ai.model.clone(),
            protocol: "openaiChat".into(),
            credential_ref: cfg.text_ai.credential_ref.clone(),
        }),
    };
    AiServiceConfig {
        connections: vec![asr, text],
        active_asr_connection_id: Some(asr_id),
        active_text_connection_id: Some(text_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_keeps_existing_credential_references() {
        let legacy = AppConfig::default();
        let migrated = migrate_legacy(&legacy);
        assert_eq!(
            migrated.connections[0].asr.as_ref().unwrap().credential_ref,
            legacy.cloud_asr.credential_ref
        );
        assert_eq!(
            migrated.connections[1]
                .text_cleanup
                .as_ref()
                .unwrap()
                .credential_ref,
            legacy.text_ai.credential_ref
        );
    }
}
```

- [ ] **Step 2: 运行迁移测试**

Run:

```bash
cargo test -p luozi migration_keeps_existing_credential_references
```

Expected：PASS；该测试先固定迁移语义，再增加 I/O。

- [ ] **Step 3: 增加原子存储**

在同文件加入 `connections_path`、`load` 和 `save`。`save` 必须先写同目录临时文件，再 rename：

```rust
pub fn save(config: &AiServiceConfig) -> Result<(), String> {
    let path = connections_path();
    let parent = path.parent().ok_or_else(|| "connection_dir_missing".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| format!("connection_dir_failed: {e}"))?;
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&temp, bytes).map_err(|e| format!("connection_write_failed: {e}"))?;
    std::fs::rename(&temp, &path).map_err(|e| format!("connection_replace_failed: {e}"))
}
```

`load` 行为固定：

1. `connections.json` 有效时直接读取。
2. 文件不存在时调用 `migrate_legacy(&config_store::load())`，保存后返回。
3. 文件存在但损坏时返回错误，不静默覆盖为默认。

迁移只搬运 provider、base URL、model 和旧 `credential_ref`，不创建新的隐私授权，也不把授权写入连接 JSON。`consent.rs` 继续作为唯一授权来源；只有 `settings_test_and_save_connection` 在连接测试成功并保存后，才调用 `consent::grant` 写入用户已确认的 capability。

在 `session/mod.rs` 增加：

```rust
pub mod connection_store;
```

- [ ] **Step 4: 增加临时目录往返测试**

把路径逻辑拆为 `load_from(path, legacy)` / `save_to(path, config)`，测试损坏 JSON 返回 `connection_parse_failed`，有效 JSON 往返相等。

Run:

```bash
cargo test -p luozi connection_store
```

Expected：迁移、往返、损坏文件三个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/session/connection_store.rs src-tauri/src/session/mod.rs src-tauri/src/session/config_store.rs
git commit -m "feat: migrate and persist AI service connections"
```

---

### Task 3: 实现不落盘的连接测试和原子保存

**Files:**

- Create: `src-tauri/src/session/connection_test.rs`
- Modify: `src-tauri/src/session/cloud.rs`
- Modify: `src-tauri/src/session/text_ai.rs`
- Modify: `src-tauri/src/session/credentials.rs`
- Modify: `src-tauri/src/session/mod.rs`

- [ ] **Step 1: 写错误映射测试**

在 `connection_test.rs` 定义：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionTestCode {
    Connected,
    Unauthorized,
    ModelNotFound,
    QuotaExceeded,
    RateLimited,
    NetworkUnavailable,
    ProtocolMismatch,
}

pub fn map_probe_error(error: &str) -> ConnectionTestCode {
    match error {
        "cloud_unauthorized" | "text_ai_unauthorized" => ConnectionTestCode::Unauthorized,
        "model_not_found" => ConnectionTestCode::ModelNotFound,
        "quota_exceeded" => ConnectionTestCode::QuotaExceeded,
        "cloud_rate_limited" | "text_ai_rate_limited" => ConnectionTestCode::RateLimited,
        "cloud_timeout" | "text_ai_timeout" | "network_unavailable" => {
            ConnectionTestCode::NetworkUnavailable
        }
        _ => ConnectionTestCode::ProtocolMismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_user_actionable_errors() {
        assert_eq!(map_probe_error("cloud_unauthorized"), ConnectionTestCode::Unauthorized);
        assert_eq!(map_probe_error("model_not_found"), ConnectionTestCode::ModelNotFound);
        assert_eq!(map_probe_error("cloud_rate_limited"), ConnectionTestCode::RateLimited);
    }
}
```

- [ ] **Step 2: 运行测试**

Run:

```bash
cargo test -p luozi maps_user_actionable_errors
```

Expected：PASS。

- [ ] **Step 3: 提取显式 Key 探测入口**

在 `cloud.rs` 把当前请求实现拆为：

```rust
pub fn transcribe_pcm_with_key(
    cfg: &CloudAsrConfig,
    pcm_16k_mono: &[f32],
    language: &str,
    api_key: &str,
) -> Result<String, String>
```

原 `transcribe_pcm` 只负责从钥匙串读取后调用该函数。

在 `text_ai.rs` 增加：

```rust
pub fn probe_text_connection(cfg: &TextAiConfig, api_key: &str) -> Result<(), String>
```

该函数发送固定测试句 `只回复 OK`，`max_tokens` 固定为 `4`，不读取用户草稿。ASR 探测使用代码生成的 250 ms 16 kHz 静音 WAV；认证通过且返回成功或明确 `no_speech` 均算协议可达，认证、模型和限流错误继续失败。

- [ ] **Step 4: 实现 test-then-save**

在 `connection_test.rs` 增加 `test_and_save(request)`：

1. 校验 provider、HTTPS、自定义地址和模型。
2. 使用请求内 Key 调用对应 probe。
3. 记录 `Instant` 得到本次延迟。
4. probe 成功后写入最终 `credential_ref`。
5. 保存连接 JSON。
6. 任一步失败时删除刚写入的 credential，并保持旧连接不变。

`credentials.rs` 增加：

```rust
pub fn replace_secret(account: &str, secret: &str) -> Result<(), String> {
    set_secret(account, secret)
}
```

不得在 `Debug`、日志或 DTO 中返回 `secret`。

- [ ] **Step 5: 运行测试**

为 `ConnectionProbe` 定义 trait，并用 fake probe 测试“失败不写 Key、成功才保存、延迟存在、错误码正确”。

Run:

```bash
cargo test -p luozi connection_test
```

Expected：全部 PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/session/connection_test.rs src-tauri/src/session/cloud.rs src-tauri/src/session/text_ai.rs src-tauri/src/session/credentials.rs src-tauri/src/session/mod.rs
git commit -m "feat: test AI connections before saving keys"
```

---

### Task 4: 暴露窄设置 API 和服务教程

**Files:**

- Create: `src-tauri/resources/provider-guides.zh-CN.json`
- Modify: `src-tauri/src/session/settings_api.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建教程资源**

创建 JSON，包含 `groq`、`qwen`、`doubao`、`xiaomi`、`openai`、`deepseek` 和 `custom-openai`。字段固定为 `id`、`consoleUrl`、`docsUrl`、`steps`：

```json
[
  {
    "id": "groq",
    "consoleUrl": "https://console.groq.com/keys",
    "docsUrl": "https://console.groq.com/docs/quickstart",
    "steps": ["登录 Groq Console", "打开 API Keys", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "qwen",
    "consoleUrl": "https://bailian.console.aliyun.com/?apiKey=1",
    "docsUrl": "https://help.aliyun.com/zh/model-studio/get-api-key",
    "steps": ["登录阿里云百炼控制台", "进入 API Key 管理", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "doubao",
    "consoleUrl": "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
    "docsUrl": "https://www.volcengine.com/docs/82379/1263271",
    "steps": ["登录火山方舟控制台", "进入 API Key 管理", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "xiaomi",
    "consoleUrl": "https://platform.xiaomimimo.com/",
    "docsUrl": "https://platform.xiaomimimo.com/docs",
    "steps": ["登录小米 MiMo 平台", "进入开发者控制台", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "openai",
    "consoleUrl": "https://platform.openai.com/api-keys",
    "docsUrl": "https://platform.openai.com/docs/quickstart",
    "steps": ["登录 OpenAI Platform", "打开 API keys", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "deepseek",
    "consoleUrl": "https://platform.deepseek.com/api_keys",
    "docsUrl": "https://api-docs.deepseek.com/",
    "steps": ["登录 DeepSeek Platform", "打开 API keys", "创建并复制 Key", "回到 Luozi 点击测试并连接"]
  },
  {
    "id": "custom-openai",
    "consoleUrl": "",
    "docsUrl": "",
    "steps": ["打开目标兼容服务的官方控制台和 API 文档", "确认 base URL、模型名和 OpenAI 兼容协议", "创建并复制 Key", "回到 Luozi 填写 base URL、模型和 Key 后点击测试并连接"]
  }
]
```

禁止链接第三方教程；设置页只打开上述官方链接。`custom-openai` 没有默认链接时隐藏“打开控制台”和“查看文档”按钮，只显示手动填写说明。

- [ ] **Step 2: 定义 DTO**

在 `settings_api.rs` 增加：

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionDto {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub supports_asr: bool,
    pub supports_text_cleanup: bool,
    pub active_for_asr: bool,
    pub active_for_text_cleanup: bool,
    pub connected: bool,
    pub masked_key: String,
}
```

`masked_key` 只能是空字符串或 `••••••••`，不得读取 Key 后四位。

- [ ] **Step 3: 增加 Tauri 命令**

在 `src-tauri/src/lib.rs` 注册：

```rust
settings_ai_services_snapshot
settings_test_and_save_connection
settings_set_active_connection
settings_delete_connection
settings_provider_guide
```

命令参数使用单独 request DTO；带 Key 的 request 不派生 `Debug`。

- [ ] **Step 4: 运行 Rust 检查**

Run:

```bash
cargo test -p luozi settings_api
cargo check -p luozi
```

Expected：成功。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/resources/provider-guides.zh-CN.json src-tauri/src/session/settings_api.rs src-tauri/src/lib.rs
git commit -m "feat: expose AI service settings API"
```

---

### Task 5: 实现 AI 服务设置页

**Files:**

- Modify: `settings.html`
- Create: `src/settings/types.ts`
- Create: `src/settings/ai-services.ts`
- Modify: `src/settings.ts`
- Modify: `src/styles-settings.css`
- Create: `tests/frontend/ai-services-model.test.ts`

- [ ] **Step 1: 写前端状态测试**

在 `ai-services.ts` 导出纯函数：

```ts
import type { AiConnectionDto } from "./types";

export function connectionSummary(item: AiConnectionDto): string {
  const roles = [
    item.activeForAsr ? "语音识别" : "",
    item.activeForTextCleanup ? "文字整理" : "",
  ].filter(Boolean);
  return roles.length > 0 ? roles.join(" · ") : "已连接，尚未使用";
}

export function connectionStatus(item: AiConnectionDto): "connected" | "needs-key" {
  return item.connected ? "connected" : "needs-key";
}
```

创建测试，覆盖同时承担两种能力、仅连接未启用和未连接状态。

- [ ] **Step 2: 运行测试**

Run:

```bash
npm run test:frontend -- tests/frontend/ai-services-model.test.ts
```

Expected：PASS。

- [ ] **Step 3: 创建 AI 服务页面**

`settings.html` 新增 `data-section="ai-services"`，页面结构固定为：

```html
<section id="section-ai-services" class="settings-section" hidden>
  <header class="section-header">
    <div>
      <h1>AI 服务</h1>
      <p>连接语音识别和文字整理服务。Key 只保存在本机钥匙串。</p>
    </div>
    <button id="btnAddAiService" class="primary">添加服务</button>
  </header>
  <div id="aiConnectionList" class="connection-list"></div>
  <dialog id="aiConnectionDialog" class="connection-dialog"></dialog>
</section>
```

连接对话框依次显示服务商、Key、推荐模型、“怎么获取 Key”、折叠高级字段、隐私说明和唯一主按钮“测试并连接”。

- [ ] **Step 4: 接入命令和错误文案**

`ai-services.ts` 只调用 Task 4 的五个命令。错误码映射必须分别显示：

```ts
const ERROR_TEXT = {
  unauthorized: "Key 无效，请重新检查。",
  modelNotFound: "模型不存在或尚未开通。",
  quotaExceeded: "当前服务额度不足。",
  rateLimited: "请求过于频繁，请稍后重试。",
  networkUnavailable: "网络不可用，未保存本次连接。",
  protocolMismatch: "接口与当前服务不兼容。",
} as const;
```

保存成功后清空 Key input 的 `.value`，关闭对话框并刷新列表。

- [ ] **Step 5: 响应式和无障碍**

CSS 要求：

- 宽度 ≥ 720 px 时连接卡为两列信息；
- 宽度 < 720 px 时操作区换行；
- Key 输入使用 `type="password"` 和明确 label；
- 测试中禁用提交并显示非循环高耗能的进度状态；
- 危险删除与主要连接按钮不使用同一颜色。

- [ ] **Step 6: 运行前端检查**

```bash
npm run test:frontend -- tests/frontend/ai-services-model.test.ts
npm run build
```

Expected：全部成功。

- [ ] **Step 7: 提交**

```bash
git add settings.html src/settings/types.ts src/settings/ai-services.ts src/settings.ts src/styles-settings.css tests/frontend/ai-services-model.test.ts
git commit -m "feat: add in-page AI service setup"
```

---

### Task 6: 迁移与实机验收

**Files:**

- Create: `docs/spikes/p1-ai-services-manual.md`

- [ ] **Step 1: 写实机清单**

清单必须包含：

- 从当前 Groq 配置迁移后 Key 仍可用。
- 新增一个文字整理连接。
- 新增一个语音识别连接。
- 同一连接承担两种能力。
- 自定义 OpenAI 兼容地址。
- Key 无效、模型错误、限流、断网和删除连接。
- 设置快照和日志不出现 Key。

- [ ] **Step 2: 执行至少一个真实服务验收**

Run:

```bash
npm run tauri dev
```

使用用户自备测试 Key 完成连接、切换和删除；Key 不写入清单。清单只记录 PASS / FAIL、错误码和本次延迟。

- [ ] **Step 3: 扫描敏感信息**

Run:

```bash
rg -n "sk-[A-Za-z0-9_-]{12,}|api[_-]?key[\"'= :]+[A-Za-z0-9_-]{12,}" . --glob '!target/**' --glob '!node_modules/**'
```

Expected：无真实 Key 命中。

- [ ] **Step 4: 全量验证**

```bash
npm run test:frontend
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected：全部成功。

- [ ] **Step 5: 提交**

```bash
git add docs/spikes/p1-ai-services-manual.md
git commit -m "docs: record AI service acceptance"
```
