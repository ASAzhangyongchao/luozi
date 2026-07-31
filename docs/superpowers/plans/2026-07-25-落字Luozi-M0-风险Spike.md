# 落字（Luozi）M0 风险 Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在独立本地仓中，用同一提交验证落字在 macOS 与 Windows 上的全局快捷键、不抢焦点悬浮窗、安全目标复核、麦克风 / 系统材质和未签名构建，并产出明确的 Go / No-Go 报告。

**Architecture:** 使用最小 Tauri 2 + Vanilla TypeScript 壳，只保留实验入口和手工证据。平台能力放在 `src-tauri/src/spike/`，实验 UI 放在 `src/`；M0 不建立正式业务架构，不创建公开远程。Mac 完成代码提交后通过 `git bundle` 把同一 Git 历史带到 Windows 实机，Windows 结果再以 bundle 返回。

**Tech Stack:** Rust stable（≥1.77.2）、Tauri 2、Vanilla TypeScript/Vite、npm、官方 `global-shortcut` 插件、macOS Accessibility、Windows UI Automation、Git

**Spec:** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md`

**Roadmap:** `docs/superpowers/plans/2026-07-25-落字Luozi-M0-M1-Spike与公开仓骨架.md`

---

## 执行边界

- 产品代码根目录固定为 `/Users/zhangyongchao/knowledge-system/apps/luozi`。
- 知识库计划在 docs/；产品源码在 `apps/luozi` 独立 Git。对 knowledge-system 根仓库禁止 `git add -A`，产品提交只在 `apps/luozi` 内执行。
- M0 分支固定为 `spike/m0`；`main` 只保存初始脚手架提交。
- M0 不创建 GitHub 仓库，不添加 knowledge-system submodule。
- Windows 必须是可交互环境；GitHub Actions 不能替代快捷键、焦点、麦克风、Word、UAC 和 SmartScreen 验证。
- 本计划只验证风险，不实现会话状态机、ASR、模型、工作台、AI、设置和正式视觉。
- 官方快捷键插件候选全部失败时停止；不在本计划内顺手扩展低级键盘钩子。

---

## M0 文件结构

```text
/Users/zhangyongchao/knowledge-system/apps/luozi/
  src/
    main.ts                    # Spike 控制台、快捷键事件与窗口控制
    styles.css                 # 最小可读样式
    overlay.ts                 # 悬浮条页面逻辑
  src-tauri/
    capabilities/default.json # M0 所需最小权限
    src/
      lib.rs                  # 注册 Spike 命令
      spike/
        mod.rs
        target.rs             # TargetToken 与平台分发
        macos.rs              # AX 探针
        windows.rs            # UIA 探针
        audio.rs              # 1 秒录音与清理探针
  overlay.html                # 不抢焦点悬浮条页面
  tests/manual/
    m0-hotkey-macos.md
    m0-hotkey-windows.md
    m0-focus-delivery-macos.md
    m0-focus-delivery-windows.md
    m0-permission-material-macos.md
    m0-permission-material-windows.md
  docs/spikes/
    m0-environment.md
    m0-report.md
  rust-toolchain.toml
  .nvmrc
```

只创建上述文件。`model-manifest/`、`docs/tutorials/`、`scripts/`、`luozi-core` 和正式设置页均不属于 M0。

---

### Task 1: 双端环境与仓库安全预检

**Files:**
- Create later: `/Users/zhangyongchao/knowledge-system/apps/luozi/docs/spikes/m0-environment.md`

- [ ] **Step 1: 确认产品目录尚未存在**

Run on Mac:

```bash
test -d /Users/zhangyongchao/knowledge-system/apps/luozi
```

Expected: exit `0`（仓已按决策落在 `apps/luozi`）。若路径不对或不是独立 Git 仓，停止排查，不覆盖。

- [ ] **Step 2: 记录 knowledge-system 当前状态但不改变它**

Run:

```bash
git -C /Users/zhangyongchao/knowledge-system status --short
```

Expected: 只读输出。后续所有产品 Git 命令必须显式使用 `/Users/zhangyongchao/knowledge-system/apps/luozi`。

- [ ] **Step 3: 检查 Mac 工具链**

Run:

```bash
sw_vers
uname -m
rustc --version
cargo --version
node --version
npm --version
git --version
```

Expected:

- macOS 13 或更高；
- `arm64` 优先，Intel 仅记录不阻塞内部 Spike；
- Rust ≥1.77.2；
- Node 为仍受支持的 LTS；
- npm、Git 可用。

- [ ] **Step 4: 在 Windows 实机检查工具链**

Run in PowerShell:

```powershell
[Environment]::OSVersion.Version
$env:PROCESSOR_ARCHITECTURE
rustc --version
cargo --version
rustup show active-toolchain
node --version
npm --version
git --version
where.exe cl
```

Expected:

- Windows 10 22H2 或 Windows 11 x64；
- Rust 使用 `x86_64-pc-windows-msvc`；
- Node 为仍受支持的 LTS；
- `cl.exe` 可用。

任何一项不满足时停止。按 Tauri 官方 Windows prerequisites 补齐 MSVC C++ Build Tools 与 WebView2 后，再重新执行本步骤。

- [ ] **Step 5: 确认 Windows 手工测试能力**

在 Windows 上确认以下项目可打开：

- Notepad；
- Microsoft Word；
- Chrome 或 Edge；
- VS Code；
- 微软拼音；
- Windows 安全设置与 UAC 提示。

Expected: 六项均可实际操作。Word 不可用时，M0 可继续其它门，但最终报告必须标记 `partial`，不能宣称完整 Go。

---

### Task 2: 创建独立 Tauri 2 Spike 仓

**Files:**
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/rust-toolchain.toml`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/.nvmrc`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/docs/spikes/m0-environment.md`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/tauri.conf.json`

- [ ] **Step 1: 用官方脚手架创建 ASCII 工程目录**

Run:

```bash
mkdir -p /Users/zhangyongchao/project
cd /Users/zhangyongchao/project
npm create tauri-app@latest
```

依次选择：

```text
Project name: luozi
Identifier: app.luozi.desktop
Frontend language: TypeScript / JavaScript
Package manager: npm
UI template: Vanilla
UI flavor: TypeScript
```

Expected: 创建 `/Users/zhangyongchao/knowledge-system/apps/luozi`，包含 `src-tauri`、`src`、`package.json`。

- [ ] **Step 2: 安装锁定依赖**

Run:

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
npm install
```

Expected: 生成 `package-lock.json`，安装结束无 error。

- [ ] **Step 3: 固定 Rust 与 Node 环境**

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Run:

```bash
node --version > .nvmrc
```

Expected: `.nvmrc` 包含当前 Node 完整版本；执行期不得手写一个未安装版本。

- [ ] **Step 4: 固定应用身份**

Modify `src-tauri/tauri.conf.json`，确认以下字段存在且取值完全一致：

```json
{
  "productName": "Luozi",
  "version": "0.0.0",
  "identifier": "app.luozi.desktop"
}
```

同时确认：

```json
{
  "bundle": {
    "macOS": {
      "minimumSystemVersion": "13.0"
    }
  }
}
```

保留脚手架生成的 `build`、`app`、`bundle` 其它字段，不删除图标配置。

- [ ] **Step 5: 创建证据目录**

Run:

```bash
mkdir -p docs/spikes tests/manual
```

Create `docs/spikes/m0-environment.md`，写入 Task 1 的实际输出，至少包含：

```markdown
# Luozi M0 Environment

## Git

- Baseline commit: 在首次提交后回填实际 SHA

## macOS

- OS:
- Architecture:
- Rust:
- Cargo:
- Node:
- npm:
- Tauri:

## Windows

- OS:
- Architecture:
- Rust toolchain:
- Node:
- npm:
- MSVC:

## Evidence rule

所有矩阵必须记录日期、机器、应用版本、结果和证据文件名；没有实测不得填写 pass。
```

除 `Baseline commit` 外先填写全部实际值；`Baseline commit` 在 Step 10 取得首个提交 SHA 后填写。此文件不进入 Step 9 的脚手架提交。

- [ ] **Step 6: 验证最小工程**

Run:

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 两条命令均 exit `0`。

- [ ] **Step 7: 启动 Mac 开发壳**

Run:

```bash
npm run tauri dev
```

Expected: 出现标题为 Luozi 的窗口，无 panic。确认后正常退出。

- [ ] **Step 8: 初始化独立 Git**

Run:

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git rev-parse --show-toplevel
```

Expected: 输出 `/Users/zhangyongchao/knowledge-system/apps/luozi`。如果命令失败，再执行：

```bash
git init -b main
```

再次运行 `git rev-parse --show-toplevel`，必须得到独立产品目录。

- [ ] **Step 9: 提交脚手架**

Run:

```bash
git add -- .gitignore .nvmrc rust-toolchain.toml README.md index.html package.json package-lock.json public src src-tauri tsconfig.json vite.config.ts
git diff --cached --check
git commit -m "chore: scaffold Luozi M0 spike"
git branch spike/m0
git switch spike/m0
```

Expected: 提交成功，当前分支为 `spike/m0`，knowledge-system 无新增暂存。

- [ ] **Step 10: 回填基线 SHA**

Run:

```bash
git rev-parse HEAD
```

把输出写进 `docs/spikes/m0-environment.md` 的 `Baseline commit`，然后：

```bash
git add -- docs/spikes/m0-environment.md
git diff --cached --check
git commit -m "docs: record Luozi M0 environments"
```

---

### Task 3: 全局快捷键 Pressed / Released 探针

**Files:**
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/Cargo.toml`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/lib.rs`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/capabilities/default.json`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/index.html`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/main.ts`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/styles.css`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-hotkey-macos.md`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-hotkey-windows.md`

- [ ] **Step 1: 安装官方插件**

Run:

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
npm run tauri add global-shortcut
```

Expected: Rust 与 npm 依赖加入 lockfile，`src-tauri/src/lib.rs` 初始化插件。

- [ ] **Step 2: 收紧前端调用权限**

确认 `src-tauri/capabilities/default.json` 至少包含：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Luozi M0 main window capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "global-shortcut:allow-is-registered",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-unregister-all"
  ]
}
```

不得加入 shell、filesystem、HTTP 或剪贴板权限。

- [ ] **Step 3: 写最小热键控制台**

Replace `index.html` body with:

```html
<main class="container">
  <h1>Luozi M0 · Hotkey Probe</h1>
  <p>一次只注册一个候选键。真实事件比注册状态更可信。</p>
  <label for="shortcut">Candidate</label>
  <select id="shortcut">
    <option>Fn</option>
    <option>Control+Alt+Space</option>
    <option>Control+Shift+Space</option>
    <option>Control+Alt+M</option>
    <option>Control+Shift+M</option>
  </select>
  <div class="actions">
    <button id="register">Register</button>
    <button id="clear">Unregister all</button>
  </div>
  <p id="status">Not registered</p>
  <pre id="events" aria-live="polite"></pre>
</main>
<script type="module" src="/src/main.ts"></script>
```

Replace `src/main.ts` with:

```ts
import {
  isRegistered,
  register,
  unregisterAll,
  type ShortcutEvent,
} from "@tauri-apps/plugin-global-shortcut";
import "./styles.css";

const shortcut = document.querySelector<HTMLSelectElement>("#shortcut")!;
const registerButton = document.querySelector<HTMLButtonElement>("#register")!;
const clearButton = document.querySelector<HTMLButtonElement>("#clear")!;
const status = document.querySelector<HTMLParagraphElement>("#status")!;
const events = document.querySelector<HTMLPreElement>("#events")!;

let sequence = 0;

function appendEvent(event: ShortcutEvent) {
  sequence += 1;
  const line = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}`;
  events.textContent = `${line}\n${events.textContent ?? ""}`;
}

registerButton.addEventListener("click", async () => {
  await unregisterAll();
  events.textContent = "";
  sequence = 0;
  const candidate = shortcut.value;

  try {
    await register(candidate, appendEvent);
    const ownedByThisApp = await isRegistered(candidate);
    status.textContent =
      `Registered by Luozi: ${ownedByThisApp}. ` +
      "Now perform the guided press/release test.";
  } catch (error) {
    status.textContent = `Registration failed: ${String(error)}`;
  }
});

clearButton.addEventListener("click", async () => {
  await unregisterAll();
  status.textContent = "Not registered";
  events.textContent = "";
  sequence = 0;
});

window.addEventListener("beforeunload", () => {
  void unregisterAll();
});
```

- [ ] **Step 4: 构建检查**

Run:

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS。

- [ ] **Step 5: 创建固定矩阵**

两个矩阵都必须包含以下列：

```markdown
| Action | Candidate | Short 10/10 | Hold 1s 10/10 | Hold 5s 10/10 | IME | Word | Browser | VS Code | Restart | Result | Evidence |
|---|---|---:|---:|---:|---|---|---|---|---|---|---|
```

Mac 固定测试：

- 继续说：`Fn`、`Control+Alt+Space`、`Control+Shift+Space`
- 语音修改：`Control+Alt+M`、`Control+Shift+M`

Windows 固定测试：

- 继续说：`Control+Alt+Space`、`Control+Shift+Space`
- 语音修改：`Control+Alt+M`、`Control+Shift+M`

每次通过必须恰好出现一条 `Pressed` 和一条 `Released`。缺失、重复、键盘自动重复、占用系统功能都记 fail。

- [ ] **Step 6: 执行 Mac 矩阵**

Run:

```bash
npm run tauri dev
```

按矩阵完成全部测试并粘贴事件日志摘要。特别验证：

- 中文输入法切换和候选框；
- Word 中输入与常用格式快捷键；
- Chrome / Safari；
- VS Code；
- 退出重开后重新注册。

`Fn` 注册失败或无事件时直接标 fail；不增加低级监听。

- [ ] **Step 7: 提交热键探针**

Run:

```bash
git add -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json index.html src/main.ts src/styles.css tests/manual/m0-hotkey-macos.md tests/manual/m0-hotkey-windows.md
git diff --cached --check
git commit -m "test: add Luozi global hotkey spike"
```

Expected: 只提交热键探针与矩阵。

---

### Task 4: 不抢焦点悬浮窗探针

**Files:**
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/tauri.conf.json`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/vite.config.ts`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/index.html`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/main.ts`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/overlay.html`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/overlay.ts`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/overlay.css`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-focus-delivery-macos.md`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-focus-delivery-windows.md`

- [ ] **Step 1: 增加静态 overlay 窗口**

在 `src-tauri/tauri.conf.json` 的 `app.windows` 中保留 `main`，再增加：

```json
{
  "label": "overlay",
  "title": "Luozi Overlay",
  "url": "overlay.html",
  "width": 360,
  "height": 72,
  "visible": false,
  "alwaysOnTop": true,
  "decorations": false,
  "resizable": false,
  "skipTaskbar": true,
  "focus": false,
  "focusable": false,
  "transparent": true
}
```

- [ ] **Step 2: 配置 Vite 多入口**

Replace `vite.config.ts` with:

```ts
import { defineConfig } from "vite";
import { resolve } from "node:path";

export default defineConfig({
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_"],
  server: {
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
      },
    },
  },
});
```

- [ ] **Step 3: 创建悬浮条页面**

Create `overlay.html`:

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Luozi Overlay</title>
  </head>
  <body>
    <div class="overlay" role="status">● 听写中 · M0</div>
    <script type="module" src="/src/overlay.ts"></script>
  </body>
</html>
```

Create `src/overlay.ts`:

```ts
import "./overlay.css";
```

Create `src/overlay.css`:

```css
:root {
  color: CanvasText;
  background: transparent;
  font-family: system-ui, sans-serif;
}

html,
body {
  margin: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}

.overlay {
  box-sizing: border-box;
  display: grid;
  place-items: center;
  width: 100%;
  height: 100%;
  border: 1px solid color-mix(in srgb, CanvasText 16%, transparent);
  border-radius: 24px;
  background: color-mix(in srgb, Canvas 82%, transparent);
  font-weight: 600;
}
```

- [ ] **Step 4: 让已注册快捷键驱动 overlay**

在 `src/main.ts` 顶部增加：

```ts
import { Window } from "@tauri-apps/api/window";
```

在 DOM 查询之后、`appendEvent` 之前增加：

```ts
const overlay = await Window.getByLabel("overlay");
```

把 `appendEvent` 改为：

```ts
function appendEvent(event: ShortcutEvent) {
  sequence += 1;
  const line = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}`;
  events.textContent = `${line}\n${events.textContent ?? ""}`;

  if (event.state === "Pressed") {
    void overlay?.show();
  } else {
    void overlay?.hide();
  }
}
```

主窗口按钮不能作为焦点验证入口，因为点击它本身会先夺走外部 App 焦点。

- [ ] **Step 5: 构建检查**

Run:

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS。

- [ ] **Step 6: 执行焦点矩阵**

Mac 测试 TextEdit、Word、Chrome / Safari、VS Code；Windows 测 Notepad、Word、Chrome / Edge、VS Code。

先在主窗口注册已通过的候选键，再把焦点放到目标 App。每个目标执行：

1. 把光标放进普通文本框并输入 `A`。
2. 按住快捷键 1 秒；overlay 应出现，松开后应隐藏。
3. 直接键入 `B`，不重新点击原 App。
4. 再短按一次快捷键，结束后键入 `C`。
5. 结果必须为连续 `ABC`，且活动进程、窗口和输入元素未变化。

点击 overlay 本体如果会激活落字窗口，记录 fail；M0 不在本任务内追加原生窗口桥。

- [ ] **Step 7: 提交悬浮窗探针**

Run:

```bash
git add -- src-tauri/tauri.conf.json vite.config.ts overlay.html src/main.ts src/overlay.ts src/overlay.css tests/manual/m0-focus-delivery-macos.md tests/manual/m0-focus-delivery-windows.md
git diff --cached --check
git commit -m "test: add Luozi non-activating overlay spike"
```

---

### Task 5: AX / UIA 目标捕获与安全交付探针

**Files:**
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/Cargo.toml`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/lib.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/mod.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/target.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/macos.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/windows.rs`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/index.html`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src/main.ts`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-focus-delivery-macos.md`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-focus-delivery-windows.md`

- [ ] **Step 1: 增加分平台原生依赖**

Run:

```bash
cargo add accessibility-sys core-foundation core-foundation-sys --target 'cfg(target_os = "macos")' --manifest-path src-tauri/Cargo.toml
cargo add windows --target 'cfg(target_os = "windows")' --features Win32_Foundation,Win32_System_Com,Win32_UI_Accessibility --manifest-path src-tauri/Cargo.toml
```

Expected: 依赖只进入对应 target section，不让 Mac 编译 Windows API 或反向编译。

- [ ] **Step 2: 定义平台无关 TargetToken**

Create `src-tauri/src/spike/target.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetToken {
    pub platform: &'static str,
    pub process_id: u32,
    pub window_id: String,
    pub element_id: String,
    pub role: String,
    pub is_secure: bool,
    pub captured_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    SameTarget,
    Changed,
    Unsupported,
    Secure,
}
```

窗口标题只允许进入调试描述，不得作为 `window_id` 或 `element_id`。

- [ ] **Step 3: 实现 macOS AX 探针**

`macos.rs` 只调用系统 Accessibility API：

1. `AXUIElementCreateSystemWide`；
2. 读取 `kAXFocusedApplicationAttribute`；
3. 读取 `kAXFocusedWindowAttribute` 与 `kAXFocusedUIElementAttribute`；
4. 通过 `AXUIElementGetPid` 取得进程；
5. 读取 role、subrole、identifier；
6. `AXSecureTextField` 或等价安全角色设置 `is_secure = true`；
7. 复核时重新获取焦点元素，并比较 pid、窗口 AX 身份和元素 AX 身份；
8. 仅在 `SameTarget && !is_secure` 时尝试设置 `kAXSelectedTextAttribute`；
9. 不支持写入时返回 `Unsupported`，由 UI 记录“应走剪贴板”，不模拟粘贴快捷键。

所有 `CFTypeRef` 必须按 Create / Copy 规则释放；不得读取输入框完整正文。

- [ ] **Step 4: 实现 Windows UIA 探针**

`windows.rs` 使用 `windows` crate的 UI Automation API：

1. 初始化 COM；
2. 创建 `CUIAutomation`；
3. `GetFocusedElement`；
4. 读取 `CurrentProcessId`、`CurrentNativeWindowHandle`、`GetRuntimeId`、`CurrentControlType`、`CurrentIsPassword`；
5. `window_id` 使用 native window handle，`element_id` 使用完整 runtime id；
6. 复核时重新获取焦点元素并逐项比较；
7. `CurrentIsPassword = true` 时返回 `Secure`；
8. 只在空测试框支持 `ValuePattern::SetValue` 时写入 `落字测试`；
9. Word、Notepad 或其它目标不支持安全插入时返回 `Unsupported`，不得使用 `SendInput` 或模拟粘贴。

不得把窗口标题当作身份，不得读取密码值。

- [ ] **Step 5: 暴露三个 Spike 命令**

在 `src-tauri/src/lib.rs` 注册：

```rust
#[tauri::command]
fn capture_target() -> Result<TargetToken, String>;

#[tauri::command]
fn validate_target(token: TargetToken) -> Result<ValidationState, String>;

#[tauri::command]
fn deliver_probe(token: TargetToken) -> Result<ValidationState, String>;
```

实际实现按 `cfg(target_os = "macos")` / `cfg(target_os = "windows")` 分发；其它平台返回明确 unsupported。

- [ ] **Step 6: 让快捷键触发 2 秒安全交付**

在 `src/main.ts` 顶部增加：

```ts
import { invoke } from "@tauri-apps/api/core";
```

在 DOM 查询之后增加：

```ts
type TargetToken = {
  platform: string;
  processId: number;
  windowId: string;
  elementId: string;
  role: string;
  isSecure: boolean;
  capturedAtMs: number;
};

type ValidationState =
  | "same_target"
  | "changed"
  | "unsupported"
  | "secure";

let deliveryRun = 0;

async function runDeliveryProbe() {
  const run = ++deliveryRun;

  try {
    const token = await invoke<TargetToken>("capture_target");
    events.textContent =
      `capture\t${JSON.stringify(token)}\n${events.textContent ?? ""}`;

    window.setTimeout(async () => {
      if (run !== deliveryRun) return;

      const validation = await invoke<ValidationState>("validate_target", {
        token,
      });
      events.textContent =
        `validate\t${validation}\n${events.textContent ?? ""}`;

      if (validation !== "same_target") return;

      const delivered = await invoke<ValidationState>("deliver_probe", {
        token,
      });
      events.textContent =
        `deliver\t${delivered}\n${events.textContent ?? ""}`;
    }, 2_000);
  } catch (error) {
    events.textContent =
      `target_error\t${String(error)}\n${events.textContent ?? ""}`;
  }
}
```

把当前 `appendEvent` 改为：

```ts
function appendEvent(event: ShortcutEvent) {
  sequence += 1;
  const line = `${sequence}\t${Date.now()}\t${event.shortcut}\t${event.state}`;
  events.textContent = `${line}\n${events.textContent ?? ""}`;

  if (event.state === "Pressed") {
    void overlay?.show();
    void runDeliveryProbe();
  } else {
    void overlay?.hide();
  }
}
```

`Pressed` 必须在外部 App 仍有焦点时捕获 token；`Released` 只负责隐藏 overlay，不改变已捕获 token。

UI 必须展示：

- 捕获的 pid、window id、element id、role、secure；
- 复核结果；
- 是否直接写入；
- 不支持或目标变化时显示 `clipboard_fallback_expected`，但 M0 不实际覆盖剪贴板。

- [ ] **Step 7: 构建检查**

Run on Mac:

```bash
cargo fmt --all --check
cargo check --manifest-path src-tauri/Cargo.toml
npm run build
```

Expected: PASS。

- [ ] **Step 8: 执行安全矩阵**

每个平台至少覆盖：

| Case | Expected |
|---|---|
| 同一普通文本框等待 2 秒 | `SameTarget`；支持则写入，不支持则明确 fallback |
| 等待中切 App | `Changed`；零插入 |
| 等待中切同 App 的另一输入框 | `Changed`；零插入 |
| 无输入焦点 | `Unsupported`；零插入 |
| 密码框 | `Secure`；零插入、零剪贴板、零正文日志 |
| macOS 认证 / Windows UAC | 不启动或立即取消；零正文反馈 |

安全项出现一次误投即 fail，不允许用成功率抵消。

- [ ] **Step 9: 提交目标探针**

Run:

```bash
git add -- src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/spike index.html src/main.ts tests/manual/m0-focus-delivery-macos.md tests/manual/m0-focus-delivery-windows.md
git diff --cached --check
git commit -m "test: add Luozi target validation spike"
```

---

### Task 6: 麦克风、系统材质与双端构建

**Files:**
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/Cargo.toml`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/mod.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/src/spike/audio.rs`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/Info.plist`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri/tauri.conf.json`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-permission-material-macos.md`
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/tests/manual/m0-permission-material-windows.md`

- [ ] **Step 1: 增加最小录音依赖**

Run:

```bash
cargo add cpal hound --manifest-path src-tauri/Cargo.toml
```

Expected: 依赖与锁文件更新。

- [ ] **Step 2: 实现 1 秒录音探针**

`audio.rs` 必须：

1. 使用系统默认输入设备；
2. 录制 1 秒单声道 PCM；
3. 输出到应用私有临时目录的随机 WAV；
4. 返回采样率、声道、帧数与字节数；
5. 命令结束前删除 WAV；
6. 权限拒绝、无设备、设备断开分别返回不同错误字符串；
7. 不保留音频、不打印音频内容。

暴露：

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioProbeResult {
    sample_rate: u32,
    channels: u16,
    frames: usize,
    bytes: u64,
    deleted: bool,
}

#[tauri::command]
async fn record_one_second_probe() -> Result<AudioProbeResult, String>;
```

- [ ] **Step 3: 配置麦克风用途说明**

macOS 构建必须包含中文和英文均能读懂的用途说明：

Create `src-tauri/Info.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>NSMicrophoneUsageDescription</key>
  <string>落字需要使用麦克风，把你主动录制的语音转换为文字。 Luozi uses the microphone only when you start a recording.</string>
</dict>
</plist>
```

Windows 不声明额外后台录音能力；只使用用户主动触发的桌面麦克风权限。

- [ ] **Step 4: 测试权限分支**

两端分别测试：

- 首次允许；
- 系统设置中拒绝后重试；
- 无输入设备；
- 录音中断开设备。

Expected: 每种情况有可区分结果；临时目录中没有遗留 WAV。

- [ ] **Step 5: 测试系统材质与不透明回退**

只对 overlay 测试：

- macOS：系统 semantic material / vibrancy；
- Windows 11：Mica；
- Windows 10 或效果失败：标准不透明背景；
- 降低透明度开启时：标准高对比背景。

记录：

- 材质名称；
- 是否抢焦点；
- 静止 60 秒 CPU；
- 显示 / 隐藏 30 次后内存是否持续增长；
- 降级结果截图。

不使用 macOS 私有 API；Windows 不采用拖动性能已知较差的 Acrylic 作为默认候选。

- [ ] **Step 6: Mac 构建冒烟**

Run:

```bash
npm run build
cargo fmt --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build
```

Expected: 全部 PASS；记录 `.app` / `.dmg` 实际路径以及 Gatekeeper 现象。

- [ ] **Step 7: 提交 Mac 侧完整 Spike**

Run:

```bash
git add -- src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/Info.plist src-tauri/src src-tauri/tauri.conf.json index.html src tests/manual docs/spikes
git diff --cached --check
git commit -m "test: complete Luozi M0 probes on macOS"
```

- [ ] **Step 8: 创建 Windows 传输 bundle**

Run on Mac:

```bash
git status --short
git bundle create /private/tmp/luozi-m0.bundle --all
git bundle verify /private/tmp/luozi-m0.bundle
```

Expected: 工作树 clean，bundle verify 成功。把 `/private/tmp/luozi-m0.bundle` 复制到 Windows 的 `$HOME\Downloads\luozi-m0.bundle`。

- [ ] **Step 9: 在 Windows 克隆同一提交**

Run in PowerShell:

```powershell
$Repo = Join-Path $HOME "project\luozi"
$Bundle = Join-Path $HOME "Downloads\luozi-m0.bundle"
if (Test-Path $Repo) { throw "Target exists: $Repo" }
git clone $Bundle $Repo
Set-Location $Repo
git switch spike/m0
git rev-parse HEAD
npm ci
```

Expected: HEAD 与 Mac bundle 中 `spike/m0` 一致，`npm ci` 成功。

- [ ] **Step 10: 执行 Windows 全矩阵**

依次完成：

- Task 3 Windows 热键矩阵；
- Task 4 Windows 焦点矩阵；
- Task 5 Windows UIA 安全矩阵；
- Task 6 麦克风、Mica / 不透明回退；
- Notepad、Word、Edge / Chrome、VS Code、微软拼音、UAC。

不得把 CI 编译结果填成手工通过。

- [ ] **Step 11: Windows 构建冒烟**

Run in PowerShell:

```powershell
npm run build
cargo fmt --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build
```

Expected: PASS；记录 MSI / EXE / portable 产物实际路径和 SmartScreen 现象。

- [ ] **Step 12: 提交 Windows 证据并回传**

Run in PowerShell:

```powershell
git add -- tests/manual/m0-hotkey-windows.md tests/manual/m0-focus-delivery-windows.md tests/manual/m0-permission-material-windows.md docs/spikes/m0-environment.md
git diff --cached --check
git commit -m "docs: record Luozi M0 Windows evidence"
$Out = Join-Path $HOME "Downloads\luozi-m0-windows.bundle"
git bundle create $Out --all
git bundle verify $Out
```

Expected: commit 和 bundle 成功。把 `luozi-m0-windows.bundle` 复制回 Mac `/private/tmp/luozi-m0-windows.bundle`。

- [ ] **Step 13: Mac 快进到 Windows 证据提交**

Run on Mac:

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git fetch /private/tmp/luozi-m0-windows.bundle spike/m0
git merge --ff-only FETCH_HEAD
```

Expected: fast-forward 成功，无冲突。

---

### Task 7: 汇总 M0 报告并执行 Go / No-Go

**Files:**
- Create: `/Users/zhangyongchao/knowledge-system/apps/luozi/docs/spikes/m0-report.md`
- Modify: `/Users/zhangyongchao/knowledge-system/apps/luozi/docs/spikes/m0-environment.md`

- [ ] **Step 1: 写报告固定结构**

Create `docs/spikes/m0-report.md`:

```markdown
# Luozi M0 Spike Report

## Scope

本报告只覆盖热键、不抢焦点悬浮窗、安全目标复核、麦克风 / 系统材质和双端构建。

## Commit

- Tested commit:

## Gate Results

| Gate | macOS | Windows | Overall | Evidence |
|---|---|---|---|---|
| G1 Hotkey Pressed / Released | | | | |
| G2 Non-activating overlay | | | | |
| G3 Target validation and safe delivery | | | | |
| G4 Permission and material fallback | | | | |
| G5 Build and launch | | | | |

## Default Shortcut Decision

| Platform | Continue speaking | Voice edit | Evidence |
|---|---|---|---|
| macOS | | | |
| Windows | | | |

## Security Assertions

- New-focus misdelivery count:
- Secure-input insertion count:
- Secure-input clipboard write count:
- Secure-input full-text feedback count:

## Resource Notes

- macOS idle CPU / memory:
- Windows idle CPU / memory:
- 30 overlay cycles:

## Platform Fallbacks

- macOS:
- Windows:

## Decision

- Result: Go / No-Go / Partial
- Blocking failures:
- Required spec changes:
```

所有空项必须用实际结果补齐；不存在的问题写 `none`，不能留空。

- [ ] **Step 2: 应用硬门禁**

只有同时满足以下条件才能写 `Go`：

- 两端 G1–G5 全部 `pass`；
- 每个平台两个动作都有稳定默认键；
- 焦点变化误投为 0；
- 安全输入插入、剪贴板、全文反馈均为 0；
- 同一提交在双端构建并启动；
- fallback 有具体平台行为，不是“以后处理”。

Word 不可测试、Windows 非实机 / 非交互环境、任一安全计数不为 0，结果只能是 `Partial` 或 `No-Go`。

- [ ] **Step 3: 最终验证**

Run:

```bash
npm run build
cargo fmt --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
rg -n "TBD|TODO|FIXME|以后处理|备选 1|其它组合" docs/spikes tests/manual
git status --short
```

Expected:

- build、fmt、clippy、test、diff check 全部通过；
- `rg` 无命中；
- 只有 `docs/spikes/m0-report.md` 和实际回填文件处于未提交状态。

- [ ] **Step 4: 提交 M0 报告**

Run:

```bash
git add -- docs/spikes/m0-report.md docs/spikes/m0-environment.md tests/manual
git diff --cached --check
git diff --cached --name-status
git commit -m "docs: finalize Luozi M0 spike decision"
```

Expected: 只提交报告和手工证据。

- [ ] **Step 5: 停止在 M0 边界**

如果结果是 `Go`：

1. 把报告交给用户审阅；
2. 回写设计文档中已验证的默认快捷键和平台回退；
3. 再生成独立 M1 公开仓骨架计划。

如果结果是 `Partial` / `No-Go`：

1. 不创建公开仓；
2. 不添加 submodule；
3. 为具体失败门单独写窄 Spike 计划；
4. 不顺手进入 M1、M2 或 UI 美化。

---

## 执行交接

Plan complete and saved to:

`docs/superpowers/plans/2026-07-25-落字Luozi-M0-风险Spike.md`

执行时二选一：

1. **Subagent-Driven（推荐）**：按 Task 逐个执行，每个任务后做规格与安全复核。
2. **Inline Execution**：使用 `superpowers:executing-plans`，按 Task 1–3、Task 4–6、Task 7 三批执行，每批结束停下来复核。

无论选择哪种方式，Windows 实机证据和用户对外部 GitHub 操作的授权都不能由 Agent 代替。
