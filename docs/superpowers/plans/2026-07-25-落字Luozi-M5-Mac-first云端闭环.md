# 落字（Luozi）M5 Mac-first 云端闭环 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mac 上打通「本地优先 / 仅云端 / 自动」路由：Keychain 存 Groq Key、OpenAI 兼容 `/audio/transcriptions`、逐提供方同意状态；本地不可用或失败时，仅在已配置且已授权时可走云端。

**Architecture:** `luozi-core` 扩展 `AppConfig`（引擎模式 + 云端预设引用，**不含 Key 明文**）与纯逻辑 `EngineRouter`；`src-tauri` 实现 Keychain、同意落盘、WAV 编码、multipart 上传、错误归一化；会话路径先试本地再按路由决定是否上传。本切片只落地 **Groq**；硅基/智谱/自定义留接口与预设表，不宣称已验收。

**Tech Stack:** Tauri 2、`ureq` multipart、`security`/`security-framework`（Mac Keychain）、既有 Whisper 本地路径

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md` §8.2、§8.3、§8.5、§14 M5
**Prior:** M4 模型下载/校验可用；正式设置 UI 仍属 M8 —— 本切片用配置文件 + 托盘/环境变量完成最小配置闭环。

**本切片范围（用户确认）：** Mac-first 最短：Keychain + OpenAI 兼容客户端 + Groq + 本地优先路由。

---

## 文件

```text
crates/luozi-core/src/config.rs          # AsrMode / CloudPresetRef / schema 仍可 v1 增量字段
crates/luozi-core/src/engine/mod.rs      # EngineRouter 纯逻辑
src-tauri/src/session/cloud.rs           # multipart + 错误码
src-tauri/src/session/credentials.rs     # Mac Keychain
src-tauri/src/session/consent.rs         # providerId+host 同意
src-tauri/src/session/router.rs          # 编排本地/云端
src-tauri/src/session/controller.rs      # finish_transcribe 走 router
src-tauri/src/lib.rs                     # 托盘：模式切换 / 配置 Groq
docs/spikes/m5-manual.md
```

---

### Task 1: Config + EngineRouter（core）

- [x] `AsrMode`: `LocalOnly` | `CloudOnly` | `Auto`（默认 `Auto`，行为：本地就绪只走本地）
- [x] `CloudConfig`: `provider_id`、`base_url`、`model`、`credential_ref`、`consented_host`（可空）
- [x] `route_asr(mode, local_ready, cloud_ready) -> Local | Cloud | NeedConfig`
- [x] 单测：自动+本地就绪→Local；自动+本地失败/缺失+云端就绪→Cloud；未同意→NeedConfig

### Task 2: Keychain + 同意

- [x] `credentials::set/get/delete("asr.groq")` → Keychain service `app.luozi.desktop`
- [x] 同意文件：`Application Support/.../cloud-consent.json`；主机变化清同意
- [x] 环境变量 `LUOZI_GROQ_API_KEY` 仅开发种子（写入 Keychain 后可不留明文）；日志永不打印 Key

### Task 3: CloudEngine（Groq）

- [x] PCM → 临时 16k mono WAV → multipart POST `{base}/audio/transcriptions`
- [x] 超时 15s；不自动重试计费 POST
- [x] 映射：401/403→`cloud_unauthorized`，429→`cloud_rate_limited`，超时→`cloud_timeout`，其余→`cloud_protocol_error`
- [x] 未同意→`cloud_not_consented`；无 Key→`cloud_unauthorized`
- [x] HTTPS only（localhost HTTP 例外留给自定义，本切片 Groq 只用 HTTPS）

### Task 4: 接会话 + 托盘

- [x] `finish_transcribe_async`：按 router 选引擎；Auto 本地失败且云端就绪则 fallback
- [x] 托盘：显示当前模式；「引擎：自动/仅本地/仅云端」循环切换；「配置 Groq Key…」（osascript 输入或说明用 env）；「同意上传到 Groq…」
- [x] 配置持久化到 `config.json`（无 Key）
- [x] README / CHANGELOG / `m5-manual.md`

### Task 5: 验收

- [x] 未同意时抓包/日志确认无外发（单元：router 拒绝 + consent gate）
- [ ] 有 Key + 同意：固定短音频可走 Groq（手工）
- [ ] 本地模型在时 Auto 不上传

## 明确不做（本切片）

- 硅基流动 / 智谱实机验收、完整设置页、竞速、文本 AI、Windows Credential Manager 宣称、内置 Key
