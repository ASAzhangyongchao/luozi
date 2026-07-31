# 落字（Luozi）M1 Mac-first 公开仓骨架 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 M0 Overall=Partial 且用户已批准 Mac-first 例外的前提下，把 `apps/luozi` 从 Spike 壳整理成可本地构建的公开仓骨架：workspace、`luozi-core` 配置 schema、最小托盘壳、双端 CI 定义与开源样板文件；不推远程、不做 ASR。

**Architecture:** Cargo workspace 根 + `crates/luozi-core`（纯 Rust 配置/类型）+ 现有 `src-tauri` 桌面壳依赖 core。M0 `spike/` 探针保留在 `src-tauri` 内并以 `m0_spike` feature 或 `docs/spikes` 旁路隔离，默认产品路径不暴露 Spike UI。托盘菜单只提供：练习窗、关于、退出。GitHub Actions 矩阵覆盖 macOS/Windows 的前端 build、fmt、clippy、test、Tauri check；CI 通过 ≠ Windows 实机验证。

**Tech Stack:** Tauri 2、Rust workspace、Vite/TS、serde AppConfig、tauri tray、GitHub Actions

**Gate:** 知识库路线图 §3.6（2026-07-25 用户批准）。公开仓 / submodule 仍需另一次确认。

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md`
**Roadmap:** `docs/superpowers/plans/2026-07-25-落字Luozi-M0-M1-Spike与公开仓骨架.md`
**M0 report:** `apps/luozi/docs/spikes/m0-report.md`

---

## 执行边界

- 产品根：`/Users/zhangyongchao/knowledge-system/apps/luozi`（独立 `.git`）。
- 知识库计划在 `docs/superpowers/plans/`；禁止对 knowledge-system 根 `git add -A`。
- 默认快捷键写入 schema 时标记 **provisional**，文案不得写成「已正式定稿」。
- Windows CI 只证明可编译；不把 CI 结果填进 M0 热键/焦点矩阵。
- 不创建 GitHub remote，除非用户本轮再次确认。

---

## M1 目标文件结构

```text
apps/luozi/
  Cargo.toml                         # workspace
  crates/luozi-core/
    Cargo.toml
    src/lib.rs
    src/config.rs                    # AppConfig schemaVersion=1
    src/config.test.rs / tests/
  src-tauri/                         # desktop member
    Cargo.toml                       # depends on luozi-core
    src/lib.rs                       # tray + practice window
    src/spike/                       # M0 probes (kept, not default UX)
  src/                               # practice / about UI
  .github/workflows/ci.yml
  README.md / README.zh-CN.md
  LICENSE
  CHANGELOG.md
  SECURITY.md
  CONTRIBUTING.md
  AGENTS.md
  docs/spikes/                       # retain M0 evidence
  tests/manual/                      # retain M0 matrices
```

---

### Task 1: Workspace + luozi-core + AppConfig

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/luozi-core/Cargo.toml`
- Create: `crates/luozi-core/src/lib.rs`
- Create: `crates/luozi-core/src/config.rs`
- Create: `crates/luozi-core/tests/config_roundtrip.rs`
- Modify: `src-tauri/Cargo.toml` (workspace member + dependency)

- [ ] **Step 1: 建立 workspace，加入 `luozi-core` 与 `src-tauri`**

根 `Cargo.toml`：

```toml
[workspace]
resolver = "2"
members = ["crates/luozi-core", "src-tauri"]
```

`luozi-core` 仅依赖 `serde` / `serde_json`。

- [ ] **Step 2: 定义 `AppConfig`（TDD）**

先写失败测试：默认配置 `schema_version == 1`，JSON roundtrip 稳定。

字段（仅 M0 已摸到的 + 设计已定通用项）：

```rust
pub struct AppConfig {
    pub schema_version: u32, // must be 1
    pub continue_speaking_shortcut: String, // provisional macOS: Control+Alt+Space
    pub voice_edit_shortcut: String,        // provisional macOS: Control+Alt+M
    pub hold_to_talk: bool,                 // default true
    pub shortcuts_provisional: bool,        // true until dual-platform Go
    pub language: String,                   // "auto"
}
```

禁止：API Key 明文、云端凭据、模型路径。

- [ ] **Step 3: `src-tauri` 依赖 `luozi-core`，暴露 `get_app_config` 命令**

- [ ] **Step 4: 提交**

```bash
git add -- Cargo.toml crates/luozi-core src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add luozi-core AppConfig schemaVersion 1"
```

---

### Task 2: 最小托盘壳与练习窗

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `capabilities/`
- Modify: `src-tauri/src/lib.rs`
- Modify: `index.html`, `src/main.ts`, `src/styles.css`
- Create: `src/about.ts` or about section in practice UI

- [ ] **Step 1: 托盘菜单：练习窗 / 关于 / 退出**

使用 Tauri 2 tray；点击「练习窗」显示 main；「关于」显示版本与 provisional 快捷键说明；「退出」退出进程。

- [ ] **Step 2: 默认启动可隐藏主窗到托盘（Mac）**；开发模式仍可直接开窗便于调试。

- [ ] **Step 3: Spike 入口降级**

主产品 UI 不再默认展示 M0 Spike 控制台；Spike 仅在 `LUOZI_M0_SPIKE=1` 或「开发者 / Spike」隐藏入口启用，避免误触写入「落字测试」。

- [ ] **Step 4: Mac 本地 `npm run tauri build` 冒烟**

- [ ] **Step 5: 提交**

```bash
git commit -m "feat: add minimal Luozi tray shell and practice window"
```

---

### Task 3: 开源样板与 CI

**Files:**
- Create: `.github/workflows/ci.yml`
- Replace/expand: `README.md`, `README.zh-CN.md`
- Create: `LICENSE` (MIT 或用户指定；未指定前用 MIT 草案并在 README 标明可改)
- Create: `CHANGELOG.md`, `SECURITY.md`, `CONTRIBUTING.md`, `AGENTS.md`

- [ ] **Step 1: README 中英写明尚无可下载 Release、Windows 实机未验证、快捷键 provisional**

- [ ] **Step 2: CI 矩阵 `macos-latest` + `windows-latest`**

Jobs：`npm ci && npm run build`、`cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、`cargo tauri build --debug` 或等价 check（若 Windows 签名问题则 `tauri build --no-bundle` / compile only，并在注释写明）。

- [ ] **Step 3: 提交**

```bash
git commit -m "docs: add M1 open-source skeleton and CI matrix"
```

---

### Task 4: 收口验证

- [ ] **Step 1: 本地验证**

```bash
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 2: 更新 `docs/spikes/m0-report.md` 仅交叉链接 M1 计划；不改 Overall=Partial**

- [ ] **Step 3: 向用户汇报路径与「待你确认才公开仓」**

---

## 明确不做（本计划）

- `gh repo create` / push / submodule
- 会话状态机、录音会话、ASR、工作台、文本 AI
- 把 provisional 快捷键写成正式产品承诺
- 用 CI 结果回填 Windows M0 矩阵为 pass
