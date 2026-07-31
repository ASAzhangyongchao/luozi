# 落字（Luozi）M4 Mac-first 模型获取 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用版本化模型清单 + SHA256 校验，让新机器能一键（脚本或托盘）下载并校验 `ggml-small`，缺模型/损坏时有明确修复路径；为按需加载与空闲释放打好钩子。

**Architecture:** 清单 JSON 嵌入应用；`model_store` 负责路径、校验、断点友好下载（`.partial` + rename）；托盘「下载推荐模型」后台执行并 `emit` 进度；ASR 引擎继续懒加载，空闲 5 分钟可卸（本切片先做可调用的 `unload` API）。

**Tech Stack:** Tauri 2、`ureq`（或 curl 回退）、`sha2`、既有 `asr::default_model_path`

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md` §8.1.2、§14 M4
**Prior:** M3 Mac Whisper 闭环已可用；安装入口 `npm run install:macos-app` → `/Applications/Luozi.app`

---

## 文件

```text
src-tauri/resources/models-manifest.json   # id/url/bytes/sha256
src-tauri/src/session/model_store.rs       # verify + download
src-tauri/src/session/asr.rs               # unload / last_used 钩子
scripts/fetch-ggml-small.sh                # SHA256 校验
docs/spikes/m4-manual.md
```

---

### Task 1: Manifest + SHA verify

- [x] 嵌入 `ggml-small` 条目（本机已校验 sha256）
- [x] `verify_file(path, sha256) -> Result`
- [x] 单测：错 hash 失败；临时小文件过测

### Task 2: Download

- [x] `download_model(entry) -> Result<PathBuf>`：`.partial`、curl/ureq、完成后校验再 rename
- [x] 空间不足 / 校验失败 → 明确错误码文案
- [x] 更新 `fetch-ggml-small.sh` 同样校验

### Task 3: 产品入口

- [x] 托盘「下载推荐模型…」；进行中禁用重复点击
- [x] `emit model://progress`；完成刷新托盘状态
- [x] README / CHANGELOG / m4-manual

### Task 4: 空闲释放（最小）

- [x] `AsrEngine` 可 `take` 卸下；记录 `last_used`
- [x] 后台定时：空闲 ≥5min 且非录音/转写 → drop engine

## 明确不做（本切片）

- base/small 硬件分档 UI、暂停按钮精致进度条、Windows 实机宣称、云端 ASR
