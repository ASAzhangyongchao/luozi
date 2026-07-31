# 落字（Luozi）M3 Mac-first 本地 Whisper 最短闭环 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用本地 `whisper.cpp`（via `whisper-rs`）+ 手动放置的多语言 **`ggml-small`** 模型，把 M2 假文本替换为真转写，完成 Mac 上「录音 → 转写 → 交付」最短闭环。

**Architecture:** `src-tauri/src/session/asr.rs` 负责模型路径解析、16 kHz 单声道重采样、Whisper 推理；`controller::stop_session` 在 `BeginTranscribe` 时调用 ASR，再走既有 `apply_delivery`。模型不进 Git；默认路径 `~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin`（与 Tauri identifier 对齐）。Metal 优先，失败回退 CPU。不做下载器（M4）、不做云端、不宣称 Windows 实机 ASR。

**Tech Stack:** Tauri 2、`whisper-rs`（macOS `metal`）、cpal 缓冲、既有 session 状态机与交付路径

**Decision (user 2026-07-25):** 默认模型档 = **`ggml-small`**（中文优先稳妥）。

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md` §8.1、§14 M3
**Prior:** M2 已提交 `apps/luozi` `d9cb3d6`

---

## 执行边界

- 产品根：`apps/luozi` 独立 `.git`；计划落在 knowledge-system `docs/superpowers/plans/`。
- 无模型文件 → 明确错误（overlay + eprintln），**不再**静默交付「落字测试」。
- 空音频 / ASR 空结果 → 可展示错误，不交付空串。
- 语言：Whisper `auto` / 中文优先可设 `language = "zh"` 若实测更好；默认跟 `AppConfig.language`（`auto` 则 Whisper auto）。
- 不做：M4 一键下载 UI、云端、工作台、公证。

---

## 目标文件结构

```text
src-tauri/src/session/
  asr.rs              # model path, resample, WhisperContext, transcribe
  controller.rs       # stop_session → asr instead of FAKE_TRANSCRIPT
  recorder.rs         # keep samples until ASR (already in AudioCapture)

scripts/
  fetch-ggml-small.sh # optional helper: curl model into Application Support

docs/spikes/m3-manual.md
```

---

### Task 1: ASR 模块 + 模型路径

**Files:**
- Create: `apps/luozi/src-tauri/src/session/asr.rs`
- Modify: `apps/luozi/src-tauri/src/session/mod.rs`
- Modify: `apps/luozi/src-tauri/Cargo.toml`（`whisper-rs`；macos features `metal`）

- [ ] **Step 1:** `default_model_path()` → `~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin`（非 macOS 用相应 config dir stub）
- [ ] **Step 2:** `resample_to_16k_mono(samples, sr, ch) -> Vec<f32>`
- [ ] **Step 3:** `transcribe(pcm16k, language) -> Result<String, String>` 用 `whisper-rs`；缺模型返回 `model_missing: <path>`
- [ ] **Step 4:** 单元测重采样（已知输入长度断言）；ASR 本体不做 CI 强依赖模型

### Task 2: 接线会话

**Files:**
- Modify: `apps/luozi/src-tauri/src/session/controller.rs`

- [ ] **Step 1:** `stop` 保留 `AudioCapture`，`BeginTranscribe` 时 `asr::transcribe`
- [ ] **Step 2:** 成功 → 既有 deliver；失败 → emit error、cancel/finish machine 干净回 Idle
- [ ] **Step 3:** 托盘/overlay 文案「落字中」保持；引擎菜单去掉「假文本」暗示或改为「本地 Whisper」

### Task 3: 取模脚本与手册

**Files:**
- Create: `apps/luozi/scripts/fetch-ggml-small.sh`
- Create: `apps/luozi/docs/spikes/m3-manual.md`
- Modify: README / README.zh-CN / CHANGELOG / package.json（可选 `fetch:model`）

- [ ] **Step 1:** 脚本下载 HuggingFace `ggerganov/whisper.cpp` 的 `ggml-small.bin` 到默认路径
- [ ] **Step 2:** m3-manual：装模型 → 说 5 秒中文短句 → 光标见转写；无模型见错误
- [ ] **Step 3:** `cargo test --workspace`；在 `apps/luozi` 提交（不 push）

---

## 验收

- Mac：模型就位后，按住热键说短句，松手后光标/剪贴板为**真实转写**（非「落字测试」）。
- 无模型：明确提示路径，不误投假文本。
- `cargo test --workspace` 绿。

## 明确不做

- M4 应用内下载器 / SHA UI
- Windows 实机 ASR 宣称
- 公开 GitHub / 公证
