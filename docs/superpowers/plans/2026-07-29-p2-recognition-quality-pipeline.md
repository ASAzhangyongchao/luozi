# Luozi P2 Recognition Quality Pipeline Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 交付快速落字、智能落字、原文模式，以及词典保护、受约束 AI 整理、1.2 秒预算和可靠降级组成的统一识别链路。

**Architecture:** `luozi-core` 负责模式、路由、基础规范化、受保护词和 AI 结果校验；Tauri 的 `RecognitionPipeline` 通过 trait 编排 ASR 与 Cleanup，便于 fake 测试；controller 只启动管线并交付最终 `RecognitionOutcome`。

**Tech Stack:** Rust 2021、Tauri 2、Serde JSON、ureq、现有 Whisper / 云端 ASR、Vitest。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §4、§6、§11、§13

**Prerequisite:** P1 已完成；至少一个 ASR 连接或本地模型可用于实机验收。

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `crates/luozi-core/src/recognition.rs` | Create | 输入模式、路由输入、结果和降级原因 |
| `crates/luozi-core/src/cleanup.rs` | Create | 基础规范化、受保护词和 AI 结果校验 |
| `crates/luozi-core/src/dictionary.rs` | Create | 个人词典类型和替换规则 |
| `crates/luozi-core/src/engine/mod.rs` | Modify | 使用新连接可用性路由 |
| `crates/luozi-core/src/config.rs` | Modify | 保存输入模式和回退偏好 |
| `crates/luozi-core/src/text_edit.rs` | Modify | 复用统一受保护词逻辑 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出识别类型 |
| `src-tauri/src/session/dictionary_store.rs` | Create | 本地词典 JSON 和原子保存 |
| `src-tauri/src/session/recognition_pipeline.rs` | Create | ASR、规范化、AI 整理和降级编排 |
| `src-tauri/src/session/text_ai.rs` | Modify | 严格 Cleanup JSON 请求与 1.2 秒预算 |
| `src-tauri/src/session/controller.rs` | Modify | 用统一管线替换直接 ASR 后交付 |
| `src-tauri/src/session/settings_api.rs` | Modify | 模式、词典和路由设置 DTO |
| `src-tauri/src/session/mod.rs` | Modify | 注册新模块 |
| `src-tauri/src/lib.rs` | Modify | 模式与词典命令 |
| `src/settings/recognition-mode.ts` | Create | 前端模式纯函数 |
| `tests/frontend/recognition-mode.test.ts` | Create | 用户模式文案和回退提示 |
| `docs/spikes/p2-recognition-quality-manual.md` | Create | 质量、延迟和失败降级验收 |

---

### Task 1: 定义三种模式和可测试路由

**Files:**

- Create: `crates/luozi-core/src/recognition.rs`
- Modify: `crates/luozi-core/src/config.rs`
- Modify: `crates/luozi-core/src/engine/mod.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写模式与路由测试**

创建 `recognition.rs`：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputMode {
    Fast,
    #[default]
    Smart,
    Verbatim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecognitionBackend {
    Local,
    Cloud,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecognitionAvailability {
    pub local_ready: bool,
    pub cloud_ready: bool,
    pub network_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecognitionPreference {
    Automatic,
    LocalOnly,
    CloudOnly { allow_local_fallback: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_prefers_valid_cloud_and_falls_back_offline() {
        let online = RecognitionAvailability {
            local_ready: true,
            cloud_ready: true,
            network_available: true,
        };
        assert_eq!(
            choose_backend(RecognitionPreference::Automatic, online),
            Ok(RecognitionBackend::Cloud)
        );
        let offline = RecognitionAvailability {
            network_available: false,
            ..online
        };
        assert_eq!(
            choose_backend(RecognitionPreference::Automatic, offline),
            Ok(RecognitionBackend::Local)
        );
    }

    #[test]
    fn cloud_only_does_not_silently_upload_or_fallback() {
        let availability = RecognitionAvailability {
            local_ready: true,
            cloud_ready: false,
            network_available: false,
        };
        assert_eq!(
            choose_backend(
                RecognitionPreference::CloudOnly {
                    allow_local_fallback: false
                },
                availability
            ),
            Err("cloud_not_ready")
        );
    }
}
```

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi-core recognition
```

Expected：FAIL，缺少 `choose_backend`。

- [ ] **Step 3: 写最小路由实现**

```rust
pub fn choose_backend(
    preference: RecognitionPreference,
    state: RecognitionAvailability,
) -> Result<RecognitionBackend, &'static str> {
    match preference {
        RecognitionPreference::LocalOnly => state
            .local_ready
            .then_some(RecognitionBackend::Local)
            .ok_or("local_not_ready"),
        RecognitionPreference::CloudOnly {
            allow_local_fallback,
        } => {
            if state.cloud_ready && state.network_available {
                Ok(RecognitionBackend::Cloud)
            } else if allow_local_fallback && state.local_ready {
                Ok(RecognitionBackend::Local)
            } else {
                Err("cloud_not_ready")
            }
        }
        RecognitionPreference::Automatic => {
            if state.cloud_ready && state.network_available {
                Ok(RecognitionBackend::Cloud)
            } else if state.local_ready {
                Ok(RecognitionBackend::Local)
            } else {
                Err("no_engine")
            }
        }
    }
}
```

`AppConfig` 增加带 `#[serde(default)]` 的 `input_mode` 和 `allow_local_fallback`；旧 JSON 反序列化必须得到 `Smart` 和 `true`。现有 `AsrMode` 映射为新 `RecognitionPreference`，删除 `engine/mod.rs` 中“Auto 固定本地优先”的测试和实现。

- [ ] **Step 4: 运行配置与路由测试**

```bash
cargo test -p luozi-core recognition
cargo test -p luozi-core config
cargo test -p luozi-core engine
```

Expected：全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/recognition.rs crates/luozi-core/src/config.rs crates/luozi-core/src/engine/mod.rs crates/luozi-core/src/lib.rs
git commit -m "feat: add dictation modes and adaptive routing"
```

---

### Task 2: 建立基础规范化和受保护词校验

**Files:**

- Create: `crates/luozi-core/src/cleanup.rs`
- Modify: `crates/luozi-core/src/text_edit.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写保护层测试**

创建 `cleanup.rs`：

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedToken {
    pub start: usize,
    pub end: usize,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_spacing_without_rewriting_meaning() {
        assert_eq!(normalize_basic("  今天  三点。。 "), "今天 三点。");
    }

    #[test]
    fn extracts_numbers_urls_email_and_acronyms() {
        let text = "ASFF 金额 12800，日期 2026-07-29，发到 luozi@example.com";
        let values: Vec<String> = protected_tokens(text)
            .into_iter()
            .map(|item| item.value)
            .collect();
        assert!(values.contains(&"ASFF".into()));
        assert!(values.contains(&"12800".into()));
        assert!(values.contains(&"2026-07-29".into()));
        assert!(values.contains(&"luozi@example.com".into()));
    }

    #[test]
    fn rejects_cleanup_that_drops_protected_data() {
        let original = "金额 12800 元，日期 2026-07-29";
        let proposed = "金额若干元。";
        assert_eq!(
            validate_cleanup(original, proposed, &protected_tokens(original)),
            Err("protected_token_changed")
        );
    }
}
```

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi-core cleanup
```

Expected：FAIL，缺少三个函数。

- [ ] **Step 3: 写最小实现**

实现必须是确定性规则，不调用模型，不读取配置：

```rust
pub fn normalize_basic(input: &str) -> String {
    let mut out = String::new();
    let mut last_ascii_space = false;
    let mut last_cjk_punct: Option<char> = None;

    for ch in input.trim().chars() {
        if ch.is_ascii_whitespace() {
            if !last_ascii_space {
                out.push(' ');
            }
            last_ascii_space = true;
            last_cjk_punct = None;
            continue;
        }

        last_ascii_space = false;
        if is_repeatable_cjk_punct(ch) {
            if last_cjk_punct == Some(ch) {
                continue;
            }
            last_cjk_punct = Some(ch);
        } else {
            last_cjk_punct = None;
        }
        out.push(ch);
    }

    out
}

fn is_repeatable_cjk_punct(ch: char) -> bool {
    matches!(ch, '，' | '。' | '！' | '？' | '；' | '：' | '、')
}

pub fn protected_tokens(text: &str) -> Vec<ProtectedToken> {
    let mut tokens = Vec::new();
    collect_email_or_url_tokens(text, &mut tokens);
    collect_numeric_tokens(text, &mut tokens);
    collect_uppercase_acronyms(text, &mut tokens);
    tokens.sort_by_key(|item| (item.start, item.end));
    tokens.dedup_by(|a, b| a.start == b.start && a.end == b.end && a.value == b.value);
    tokens
}

fn push_token(tokens: &mut Vec<ProtectedToken>, start: usize, end: usize, value: &str) {
    if start < end && !value.trim_matches(|ch: char| ch.is_ascii_punctuation()).is_empty() {
        tokens.push(ProtectedToken {
            start,
            end,
            value: value.to_string(),
        });
    }
}

fn collect_email_or_url_tokens(text: &str, tokens: &mut Vec<ProtectedToken>) {
    let mut offset = 0;
    for raw in text.split_whitespace() {
        if let Some(start) = text[offset..].find(raw).map(|index| offset + index) {
            let end = start + raw.len();
            offset = end;
            let value = raw.trim_matches(|ch: char| "，。；：、,.!?()[]".contains(ch));
            if value.contains('@') || value.starts_with("http://") || value.starts_with("https://") {
                push_token(tokens, start, end, value);
            }
        }
    }
}

fn collect_numeric_tokens(text: &str, tokens: &mut Vec<ProtectedToken>) {
    let mut start = None;
    for (index, ch) in text.char_indices() {
        let keep = ch.is_ascii_digit() || matches!(ch, '-' | '/' | '.' | ':' | '+' | '%');
        match (start, keep) {
            (None, true) => start = Some(index),
            (Some(begin), false) => {
                push_token(tokens, begin, index, &text[begin..index]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        push_token(tokens, begin, text.len(), &text[begin..]);
    }
}

fn collect_uppercase_acronyms(text: &str, tokens: &mut Vec<ProtectedToken>) {
    let mut start = None;
    for (index, ch) in text.char_indices() {
        match (start, ch.is_ascii_uppercase()) {
            (None, true) => start = Some(index),
            (Some(begin), false) => {
                if index - begin >= 2 {
                    push_token(tokens, begin, index, &text[begin..index]);
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        if text.len() - begin >= 2 {
            push_token(tokens, begin, text.len(), &text[begin..]);
        }
    }
}

pub fn validate_cleanup(
    original: &str,
    proposed: &str,
    protected: &[ProtectedToken],
) -> Result<(), &'static str> {
    let original_len = original.chars().count().max(1);
    let proposed_len = proposed.chars().count();
    if proposed.trim().is_empty() {
        return Err("cleanup_empty");
    }
    if proposed_len * 10 > original_len * 25 {
        return Err("cleanup_too_long");
    }
    if proposed_len * 10 < original_len * 4 {
        return Err("cleanup_too_short");
    }
    if protected.iter().any(|item| !proposed.contains(&item.value)) {
        return Err("protected_token_changed");
    }
    Ok(())
}
```

从 `lib.rs` 导出：

```rust
pub use cleanup::{
    normalize_basic, protected_tokens, validate_cleanup, ProtectedToken,
};
```

`text_edit.rs` 删除重复数字 / 缩写扫描，改为调用 `protected_tokens`。

- [ ] **Step 4: 运行测试**

```bash
cargo test -p luozi-core cleanup
cargo test -p luozi-core text_edit
```

Expected：全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/cleanup.rs crates/luozi-core/src/text_edit.rs crates/luozi-core/src/lib.rs
git commit -m "feat: protect transcript facts during cleanup"
```

---

### Task 3: 增加本地个人词典

**Files:**

- Create: `crates/luozi-core/src/dictionary.rs`
- Create: `src-tauri/src/session/dictionary_store.rs`
- Modify: `crates/luozi-core/src/lib.rs`
- Modify: `src-tauri/src/session/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写词典应用测试**

在 `dictionary.rs` 定义：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub spoken: String,
    pub written: String,
}

pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
    entries.iter().fold(text.to_string(), |value, entry| {
        if entry.spoken.trim().is_empty() || entry.written.trim().is_empty() {
            value
        } else {
            value.replace(&entry.spoken, &entry.written)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_explicit_user_spelling() {
        let entries = vec![DictionaryEntry {
            spoken: "落子".into(),
            written: "落字".into(),
        }];
        assert_eq!(apply_dictionary("打开落子设置", &entries), "打开落字设置");
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p luozi-core dictionary
```

Expected：PASS。

- [ ] **Step 3: 建立原子 JSON 存储**

`dictionary_store.rs` 使用 `dictionary.json.tmp` + rename 保存：

```rust
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryStore {
    pub entries: Vec<luozi_core::DictionaryEntry>,
}
```

校验规则：

- `spoken` / `written` trim 后非空；
- 单项各不超过 128 字符；
- `spoken` 大小写不敏感唯一；
- 最多 500 项。

为 add、update、delete、duplicate 和损坏 JSON 写临时目录测试。

- [ ] **Step 4: 暴露窄命令**

在 `lib.rs` 注册：

```rust
dictionary_list
dictionary_upsert
dictionary_delete
```

命令只返回词典项，不返回草稿或历史。

- [ ] **Step 5: 运行测试并提交**

```bash
cargo test -p luozi-core dictionary
cargo test -p luozi dictionary_store
git add crates/luozi-core/src/dictionary.rs crates/luozi-core/src/lib.rs src-tauri/src/session/dictionary_store.rs src-tauri/src/session/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add local personal dictionary"
```

---

### Task 4: 实现严格 AI Cleanup 响应

**Files:**

- Modify: `src-tauri/src/session/text_ai.rs`

- [ ] **Step 1: 写响应解析测试**

增加：

```rust
#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResponse {
    pub text: String,
    #[serde(default)]
    pub suspicions: Vec<CleanupSuspicion>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSuspicion {
    pub fragment: String,
    pub reason: String,
}

#[test]
fn parses_strict_cleanup_json_and_rejects_markdown() {
    let ok = parse_cleanup_response(
        r#"{"text":"下午三点开会。","suspicions":[]}"#,
    )
    .unwrap();
    assert_eq!(ok.text, "下午三点开会。");
    assert!(parse_cleanup_response("```json\n{}\n```").is_err());
}
```

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi parses_strict_cleanup_json_and_rejects_markdown
```

Expected：FAIL，缺少 parser。

- [ ] **Step 3: 实现解析和请求**

`parse_cleanup_response` 直接 `serde_json::from_str`，并拒绝空 text、超过原文 2.5 倍的结果和超过 20 个 suspicion。

新增 `cleanup_transcript`：

- connect timeout `800 ms`；
- read timeout `1200 ms`；
- temperature `0.1`；
- system prompt 明确只返回 JSON；
- user payload 包含原文、模式、词典保护后的 token 列表；
- 不发送外部输入框上下文。

请求失败统一返回现有可映射错误，不在该函数内重试。

- [ ] **Step 4: 运行测试**

```bash
cargo test -p luozi text_ai
```

Expected：全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/session/text_ai.rs
git commit -m "feat: add bounded structured transcript cleanup"
```

---

### Task 5: 编排统一 RecognitionPipeline

**Files:**

- Create: `src-tauri/src/session/recognition_pipeline.rs`
- Modify: `src-tauri/src/session/mod.rs`
- Modify: `src-tauri/src/session/controller.rs`

- [ ] **Step 1: 写 fake runner 测试**

在新文件定义：

```rust
pub trait AsrRunner {
    fn transcribe(&self, pcm: &[f32], language: &str) -> Result<String, String>;
}

pub trait CleanupRunner {
    fn cleanup(
        &self,
        text: &str,
        protected: &[luozi_core::ProtectedToken],
    ) -> Result<super::text_ai::CleanupResponse, String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecognitionOutcome {
    pub text: String,
    pub raw_text: String,
    pub used_cleanup: bool,
    pub fallback_reason: Option<String>,
    pub suspicions: Vec<super::text_ai::CleanupSuspicion>,
}
```

测试必须覆盖：

1. Fast 不调用 Cleanup。
2. Smart 成功使用 AI 结果。
3. Smart 超时回退规范化 ASR。
4. Verbatim 不删除口头语。
5. AI 丢失 protected token 时回退。

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi recognition_pipeline
```

Expected：FAIL，缺少 pipeline 实现。

- [ ] **Step 3: 写最小管线**

执行顺序固定：

```rust
let raw = asr.transcribe(pcm, language)?;
let normalized = luozi_core::normalize_basic(&raw);
let dictionary_text = luozi_core::apply_dictionary(&normalized, dictionary);
let protected = luozi_core::protected_tokens(&dictionary_text);
```

- Fast 返回 `dictionary_text`。
- Verbatim 返回只做 trim 和重复标点压缩的结果。
- Smart 调用 Cleanup，随后 `validate_cleanup`；任何 Cleanup 错误写入 `fallback_reason`，返回 `dictionary_text`。

在 `session/mod.rs` 注册模块。

- [ ] **Step 4: 替换 controller 直接调用**

`finish_transcribe_async` 不再自行调用 `run_asr_with_route` 后立即交付；它构造 runner，调用 `RecognitionPipeline::run`，把 `outcome.text` 交给现有 SessionMachine。

保持不变：

- `sessionId` 晚到结果保护；
- VoiceEdit 分支；
- 焦点复核；
- 剪贴板降级；
- Escape 取消。

Telemetry 分别在 ASR 和 Cleanup 结束时 mark。

- [ ] **Step 5: 运行测试**

```bash
cargo test -p luozi recognition_pipeline
cargo test -p luozi-core session
cargo test -p luozi controller
```

Expected：全部 PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/session/recognition_pipeline.rs src-tauri/src/session/mod.rs src-tauri/src/session/controller.rs
git commit -m "feat: route dictation through quality pipeline"
```

---

### Task 6: 暴露模式、词典和回退状态

**Files:**

- Modify: `src-tauri/src/session/settings_api.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/settings/recognition-mode.ts`
- Create: `tests/frontend/recognition-mode.test.ts`

- [ ] **Step 1: 写前端模式模型测试**

创建纯函数：

```ts
export type RecognitionMode = "fast" | "smart" | "verbatim";

export function modeDescription(mode: "fast" | "smart" | "verbatim"): string {
  return {
    fast: "速度优先，不等待 AI 整理",
    smart: "识别后进行受约束的智能整理",
    verbatim: "尽量保留原话，只做最小格式处理",
  }[mode];
}

export function recognitionModeLabel(mode: RecognitionMode): string {
  return {
    fast: "快速落字",
    smart: "智能落字",
    verbatim: "原文模式",
  }[mode];
}

export function nextRecognitionMode(mode: RecognitionMode): RecognitionMode {
  return {
    fast: "smart",
    smart: "verbatim",
    verbatim: "fast",
  }[mode] as RecognitionMode;
}
```

测试三个 key 的 label / description 均返回非空且互不相同，`nextRecognitionMode` 必须形成 `fast -> smart -> verbatim -> fast` 闭环。

- [ ] **Step 2: 增加设置命令**

注册：

```rust
settings_set_input_mode
settings_set_local_fallback
dictionary_list
dictionary_upsert
dictionary_delete
```

`SettingsSnapshot` 增加 `input_mode`、`allow_local_fallback` 和 `last_fallback_reason`，不增加草稿全文。

- [ ] **Step 3: 运行检查并提交**

```bash
npm run test:frontend -- tests/frontend/recognition-mode.test.ts
cargo test -p luozi settings_api
npm run build
git add src-tauri/src/session/settings_api.rs src-tauri/src/lib.rs src/settings/recognition-mode.ts tests/frontend/recognition-mode.test.ts
git commit -m "feat: expose dictation quality settings"
```

---

### Task 7: 质量与降级实机验收

**Files:**

- Create: `docs/spikes/p2-recognition-quality-manual.md`

- [ ] **Step 1: 执行固定语料**

从 `tests/benchmarks/corpus.json` 每类至少录制一次，Smart 模式对 self-correction、number、mixed-language、list 和 proper-noun 必须有结果。

- [ ] **Step 2: 验证失败路径**

依次验证：

- 断网自动回退本地。
- 固定云端且关闭本地回退时不静默切换。
- AI Key 失效时仍插入基础转写。
- AI 超过 1.2 秒时 HUD 提示使用原始结果。
- protected token 被 AI 删除时回退。

- [ ] **Step 3: 写实测报告**

清单记录每个场景的 ASR、Cleanup、Delivery 和总耗时，以及是否人工修改。不得写入 Key、完整真实工作内容或原始音频路径。

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
git add docs/spikes/p2-recognition-quality-manual.md
git commit -m "docs: record recognition quality acceptance"
```
