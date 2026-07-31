# 落字（Luozi）M8 Mac-first 设置壳 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Mac 上提供正式设置窗与关于页：通用 / 语音与 AI / 快捷键 / 权限 / 关于；托盘「设置…」可点；练习窗降为 `?spike` 开发入口。

**Architecture:** 新增 Tauri 窗口 `settings`（`settings.html`）；前端侧栏切换分区；后端复用 `config_store` / Keychain / consent，并补充只读状态查询命令。草稿窗 `main` 保持独立。首次引导、液态玻璃精修、改键录制留后。

**Tech Stack:** Tauri 2、既有托盘/credentials/consent、Vite 多页

**本切片（用户确认）：** Mac 最短设置壳。

---

## 文件

```text
settings.html / src/settings.ts / src/styles-settings.css
src-tauri/tauri.conf.json          # settings 窗
src-tauri/src/lib.rs               # show_settings / about / 命令
src-tauri/src/session/settings_api.rs  # 状态聚合
vite.config.ts                     # settings 入口
docs/spikes/m8-manual.md
```

---

### Task 1: 窗口与托盘

- [x] `settings` 窗
- [x] 托盘「设置…」/「关于」/「检查权限…」
- [x] Spike 降为「开发 Spike…」

### Task 2: settings_api 命令

- [x] `settings_snapshot` 与配置/同意/打开系统设置/仓库链接

### Task 3: 设置 UI

- [x] 侧栏五分区 + 操作按钮

### Task 4: 文档与验证

- [x] m8-manual / README / CHANGELOG
- [x] `cargo test --workspace` + `npm run build`
