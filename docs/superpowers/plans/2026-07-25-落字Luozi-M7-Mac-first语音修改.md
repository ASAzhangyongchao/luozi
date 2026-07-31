# 落字（Luozi）M7 Mac-first 语音修改 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mac 上打通「第二快捷键录修改指令 → 打开/载入草稿 → OpenAI 兼容文本 AI → 选区/当前段替换 → 保护片段风险确认」。

**Architecture:** `luozi-core` 纯逻辑：`resolve_edit_scope`、`protected_tokens`、`assess_edit_risk`；`src-tauri` 增加独立 `TextAiConfig` + Keychain/`textai.*` 同意、chat/completions 客户端；会话增加 `SessionIntent::VoiceEdit`，录音复用 ASR，交付改为草稿补丁而非 AX。设置壳仍属 M8。

**Tech Stack:** Tauri 2、既有录音/ASR/DraftStore、`ureq` JSON chat、Keychain（复用 credentials）

**Spec:** §7.3–7.4、§8.6、§14 M7
**本切片（用户确认）：** Mac 最短闭环；不做疑似词 UI、完整 diff sheet、语音「应用/取消」、快捷落字 AI 成文。

---

## 文件

```text
crates/luozi-core/src/text_edit.rs       # scope / protect / risk
crates/luozi-core/src/config.rs          # TextAiConfig + 默认第二快捷键
src-tauri/src/session/text_ai.rs         # chat completions
src-tauri/src/session/draft.rs           # selection + pending edit
src-tauri/src/session/controller.rs      # VoiceEdit 意图与收尾
src-tauri/src/lib.rs                     # 注册第二热键 + 托盘 BYOK
src/main.ts / index.html / styles.css    # 选区上报、高风险确认
docs/spikes/m7-manual.md
```

---

### Task 1: text_edit 纯逻辑（core）

- [x] `EditScope { start, end, kind: Selection | Paragraph | FullDocument }`
- [x] `resolve_edit_scope(text, sel_start, sel_end, instruction) -> EditScope`
- [x] `extract_protected_spans(text) -> Vec<(start,end)>`
- [x] `assess_edit_risk(...) -> Low | High { reasons }`
- [x] 单测覆盖选区、段落、全文关键词、保护片段升级

### Task 2: TextAiConfig + credentials

- [x] `TextAiConfig` 默认 Groq chat；`credential_ref=textai.groq`
- [x] `AppConfig.text_ai`；旧 JSON 缺省可反序列化
- [x] `LUOZI_TEXT_AI_API_KEY`（可回退 `LUOZI_GROQ_API_KEY`）
- [x] 同意：`provider_id=textai.groq` + host
- [x] 默认 `voice_edit_shortcut = "Control+Alt+Shift+Space"`

### Task 3: text_ai 客户端

- [x] `text_ai_ready` / `rewrite_scope` chat completions
- [x] 错误码与脱敏日志

### Task 4: 会话 VoiceEdit + 热键

- [x] `SessionIntent::VoiceEdit`
- [x] 第二热键注册；空草稿载入最近落字
- [x] Overlay / 应用补丁 / 高风险 pending
- [x] 托盘文本 AI Key + 同意；「说出修改要求」启用

### Task 5: 前端选区与确认

- [x] UTF-8 选区上报；`draft://edit-preview` confirm
- [x] footer 第二快捷键提示

### Task 6: 文档与验收

- [x] `docs/spikes/m7-manual.md`
- [x] README / CHANGELOG
- [x] `cargo test --workspace`

## 明确不做（本切片）

- 疑似词波浪线、完整 diff sheet、语音说应用/取消、设置页、快捷落字自动 AI、多厂商验收、Windows
