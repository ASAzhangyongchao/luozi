# 落字（Luozi）M2 Mac-first 会话与交付 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Mac-first 前提下实现可单测的会话状态机、真连续录音、Esc 取消、假文本交付（插入 / 剪贴板降级）、目标变化安全路由与剪贴板有限撤销；不上 ASR、不做工作台。

**Architecture:** `luozi-core` 放纯逻辑会话机与交付路由；`src-tauri` 负责 cpal 连续录音、macOS AX 插入 / 剪贴板、全局热键与 Esc、托盘接线、overlay 状态事件。M0 `spike/` 探针保留但不进默认产品路径。Windows 只保证可编译，实机交付矩阵仍标 Partial。

**Tech Stack:** Tauri 2、Rust workspace、cpal、macOS Accessibility / Pasteboard、`tauri-plugin-global-shortcut`、Vite overlay

**Gate:** Logo / 托盘视觉收口后用户批准进入 M2（2026-07-25）。公开仓 / submodule 仍需另次确认。M0 Overall 仍为 Partial。

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md` §5.3–5.7、§6、§14 M2
**Roadmap:** `docs/superpowers/plans/2026-07-25-落字Luozi-M0-M1-Spike与公开仓骨架.md`
**Reuse:** `apps/luozi/src-tauri/src/spike/{audio,target,macos,windows}.rs`

---

## 执行边界

- 产品根：`/Users/zhangyongchao/knowledge-system/apps/luozi`（独立 `.git`）。知识库计划在 `docs/superpowers/plans/`；禁止对 knowledge-system 根 `git add -A`。
- **假文本**：停止录音后不调用 ASR，固定交付文案（如 `落字测试`），证明插入 / 剪贴板 / 撤销链路。
- 录音时长：最短 0.3s（过短 → Canceled）、最长 30s（到时自动 stop）。
- 安全输入：开录时 `isSecure` → 拒绝启动；交付时才发现 → Discard（不插、不写剪贴板）。
- Esc：全局取消当前会话；托盘「取消当前录音」同步。
- 剪贴板撤销：仅剪贴板路径留下可撤销快照，默认约 60s；插入路径不保证跨进程撤销。
- 不做：whisper、工作台、语音修改、设置页、公开推送、Windows 实机验收宣称。

---

## 目标文件结构

```text
crates/luozi-core/src/
  config.rs
  session/
    mod.rs                 # SessionId, Phase, Event, Outcome
    machine.rs             # Idle→Recording→Transcribing→Delivering→…
    delivery.rs            # ValidationState → Insert | Clipboard | Discard
  lib.rs

src-tauri/src/
  session/
    mod.rs                 # App-owned SessionController
    recorder.rs            # cpal start/stop/cancel, duration limits
    clipboard.rs           # macOS pasteboard write + undo snapshot
  platform/                # thin wrappers over spike capture/validate/insert
  lib.rs                   # tray enable start/cancel/undo; register Esc + hotkeys
  spike/                   # keep probes

src/overlay.ts             # listen session://* → 文案切换
```

---

### Task 1: luozi-core 会话状态机（TDD）

**Files:**
- Create: `crates/luozi-core/src/session/mod.rs`
- Create: `crates/luozi-core/src/session/machine.rs`
- Create: `crates/luozi-core/src/session/delivery.rs`
- Modify: `crates/luozi-core/src/lib.rs`
- Test: unit tests in `machine.rs` / `delivery.rs`

- [x] **Step 1: 写失败测试** — `start` 从 Idle→Recording 分配递增 `sessionId`；忙时第二次 `start` 被拒绝；`cancel` 从 Recording→Canceled→Idle；`stop`→Transcribing（M2 立刻假成功）→Delivering→Success；过期 `sessionId` 的 late deliver 丢弃。

- [x] **Step 2: 实现 `SessionMachine` 与 `DeliveryRoute::{Insert,Clipboard,Discard}`**

- [x] **Step 3: `cargo test -p luozi-core` 全绿**

- [ ] **Step 4: 在 `apps/luozi` 提交** `feat: add M2 session state machine`

---

### Task 2: 连续录音 SessionRecorder

**Files:**
- Create: `src-tauri/src/session/recorder.rs`
- Create: `src-tauri/src/session/mod.rs`
- Modify: `src-tauri/src/lib.rs`（注册模块）
- Reuse patterns from: `src-tauri/src/spike/audio.rs`

- [ ] **Step 1: `start()` 开 cpal 流写入内存/临时 WAV；`stop()` 返回 `AudioBuffer` meta；`cancel()` 丢弃**

- [ ] **Step 2: 强制最短 0.3s / 最长 30s；设备错误映射为可展示 `errorCode`**

- [ ] **Step 3: 手工：托盘开始→停→临时文件不残留**

---

### Task 3: 假文本交付 + 剪贴板撤销

**Files:**
- Create: `src-tauri/src/session/clipboard.rs`（macOS；Windows stub 可编译）
- Modify: spike insert 路径抽出可参数化 `deliver_text(token, text)`
- Modify: `src-tauri/src/session/mod.rs` controller

- [ ] **Step 1: `same_target` → 插入假文本；失败降 Clipboard**

- [ ] **Step 2: `changed` / `unsupported` → 写剪贴板并保存可撤销快照**

- [ ] **Step 3: `secure` → Discard，零副作用**

- [ ] **Step 4: `undo_last_clipboard` 条件恢复；超时失效**

---

### Task 4: 热键 / Esc / 托盘 / Overlay 接线

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/overlay.ts` / `overlay.html`
- Modify: tray menu enable `start` / `cancel` / `undo`

- [ ] **Step 1: 全局注册 provisional「继续说」：Pressed=start（或 hold 模式 start），Released=stop**

- [ ] **Step 2: 全局 Esc → cancel（仅会话非 Idle 时消费）**

- [ ] **Step 3: 托盘 `start`/`cancel`/`undo` 启用并调用同一 controller**

- [ ] **Step 4: emit `session://phase`；overlay 显示 听写中 / 落字中 / 已落字 / 已取消**

- [ ] **Step 5: 状态行「引擎未就绪」改为「假文本就绪（M2）」类文案**

---

### Task 5: 手工验收 + 文档

**Files:**
- Create: `apps/luozi/docs/spikes/m2-manual.md`
- Modify: `apps/luozi/README.zh-CN.md`、路线图 §6
- Modify: `CHANGELOG.md`

- [ ] **Step 1: TextEdit 焦点插入假文本；切 App 后走剪贴板；密码页拒绝或 Discard**

- [ ] **Step 2: Esc 取消不落字；剪贴板路径可撤销一次**

- [ ] **Step 3: 更新 README「当前可用」；路线图下一步指向 M3**

---

## 明确不做（本计划）

- whisper.cpp / 模型下载 / 云端 ASR
- 语音草稿工作台、语音修改、文本 AI
- 左键迷你面板 WebView（仍用练习窗占位）
- 公开 GitHub / submodule / 公证安装包
- 宣称 Windows 实机 M2 Go

---

## 验收标准（Mac）

1. `cargo test --workspace` 通过，状态机单测覆盖忙拒绝、取消、late discard。
2. 按住 provisional 热键：overlay 出现 → 松手 → TextEdit 出现「落字测试」（或剪贴板路径提示）。
3. 录音中 Esc：无插入、无剪贴板变更。
4. 切焦点后交付只写剪贴板；`撤销上次落字` 可恢复上一份普通文本（60s 内）。
5. 安全输入：不插入、不写剪贴板。
