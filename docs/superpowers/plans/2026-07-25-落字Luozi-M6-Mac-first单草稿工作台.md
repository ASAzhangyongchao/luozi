# 落字（Luozi）M6 Mac-first 单草稿工作台 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用一份本机草稿工作台替换练习窗：编辑 / 粘贴 / 复制、自动保存与恢复、最多 20 步撤销重做、最近一次落字可载入；工作台在前台时语音结果写入光标处而非外部 App。

**Architecture:** `luozi-core` 提供纯逻辑 `DraftDocument`（正文 + 环形差异栈）；`src-tauri/session/draft.rs` 负责落盘与「最近一次落字」内存 TTL；前端 `main` 窗口改为工作台 UI；交付路径检测工作台焦点后改为 `draft_insert`。语音修改 / 文本 AI 仍属 M7。

**Tech Stack:** Tauri 2、既有 session 热键、纯文本 textarea（首版不嵌富文本编辑器）

**Spec:** §7.1–7.2、§14 M6
**Prior:** M5 Groq 路由可用；设置壳仍属 M8。

---

## 文件

```text
crates/luozi-core/src/draft.rs
src-tauri/src/session/draft.rs
src/main.ts / index.html / styles.css   # 工作台
src-tauri/tauri.conf.json               # 标题「落字 · 语音草稿」
src-tauri/src/lib.rs                    # 启用「打开语音草稿」
docs/spikes/m6-manual.md
```

---

### Task 1: DraftDocument（core）

- [x] `text` + `undo`/`redo` 栈（最多 20）；`apply(new_text)`、`undo`、`redo`、`clear`
- [x] 单测：连续编辑、撤销回退、超过 20 丢最旧

### Task 2: 落盘 + 最近落字

- [x] `Application Support/.../draft.json` 原子写
- [x] 内存 `last_transcript`：成功落字后写入，10 分钟 TTL，载入后清空
- [x] commands：`draft_state` / `draft_save` / `draft_undo` / `draft_redo` / `draft_clear` / `draft_load_last`

### Task 3: 工作台 UI

- [x] 单栏 textarea + 顶部复制 / 清空 / 载入最近；底部「继续说」说明用全局热键
- [x] 防抖 1s 自动保存；关闭前强制 save
- [x] 托盘「打开语音草稿」可用；窗口标题改名；练习/Spike 仅 `?spike`

### Task 4: 交付进草稿

- [x] 工作台可见且为前台时，转写结果插入草稿（选区替换或文末追加）并 HUD「已写入草稿」
- [x] 否则保持原 AX / 剪贴板路径

## 明确不做（本切片）

- 说出修改（M7）、疑似词标注 UI、液态玻璃精修（M8）、多文档
