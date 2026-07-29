# 如何使用（Guide 窗）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 托盘右键菜单增加「如何使用…」，打开独立分章节新手教程窗。

**Architecture:** 镜像设置窗：新增 `guide` WebviewWindow + 静态 `guide.html`/`guide.ts`，复用 `styles-settings.css`；Rust 侧 `show_guide` + 关窗 hide + 页脚 invoke 打开设置。文案写死默认快捷键，并注明以设置页为准。

**Tech Stack:** Tauri 2、Vite 多页、TypeScript、现有 settings 视觉 token。

**Spec:** `docs/superpowers/specs/2026-07-27-guide-window-design.md`

---

## File map

| 文件 | 动作 | 职责 |
|---|---|---|
| `guide.html` | Create | 教程壳 + 6 章节正文 |
| `src/guide.ts` | Create | 章节切换；`open_settings_window` invoke |
| `src/styles-settings.css` | Modify | 增加 guide 阅读排版与页脚 |
| `vite.config.ts` | Modify | rollup input 增加 `guide` |
| `src-tauri/tauri.conf.json` | Modify | 登记 `guide` 窗口 |
| `src-tauri/capabilities/default.json` | Modify | windows 加 `guide` |
| `src-tauri/capabilities/desktop.json` | Modify | windows 加 `guide` |
| `src-tauri/src/lib.rs` | Modify | show_guide、托盘、关窗、背景色、open_settings_window |
| `docs/spikes/guide-manual.md` | Create | 手工验收 |
| `README.zh-CN.md` / `README.md` / `CHANGELOG.md` | Modify | 标明菜单教程可用 |

---

### Task 1: Vite 多页入口 + 空壳 guide 页

**Files:**
- Create: `guide.html`
- Create: `src/guide.ts`
- Modify: `vite.config.ts`
- Modify: `src/styles-settings.css`（页脚样式）

- [ ] **Step 1: 扩展 Vite 入口**

在 `vite.config.ts` 的 `build.rollupOptions.input` 增加：

```ts
guide: resolve(__dirname, "guide.html"),
```

完整 `input` 应为：

```ts
input: {
  main: resolve(__dirname, "index.html"),
  overlay: resolve(__dirname, "overlay.html"),
  settings: resolve(__dirname, "settings.html"),
  guide: resolve(__dirname, "guide.html"),
},
```

- [ ] **Step 2: 创建最小 `guide.html`（先放 1 个章节骨架，Task 3 再填全文）**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>落字 · 如何使用</title>
  </head>
  <body class="settings-body">
    <div class="settings-shell guide-shell">
      <nav class="settings-nav" aria-label="教程章节">
        <button type="button" class="nav-item active" data-section="start">快速开始</button>
        <button type="button" class="nav-item" data-section="hotkeys">快捷键</button>
        <button type="button" class="nav-item" data-section="draft">语音草稿</button>
        <button type="button" class="nav-item" data-section="voice-edit">说出修改</button>
        <button type="button" class="nav-item" data-section="settings">设置与权限</button>
        <button type="button" class="nav-item" data-section="faq">常见问题</button>
      </nav>
      <div class="guide-column">
        <main class="settings-main">
          <section id="section-start" class="settings-section">
            <h1>快速开始</h1>
            <p class="muted">（正文 Task 3 填写）</p>
          </section>
          <section id="section-hotkeys" class="settings-section" hidden>
            <h1>快捷键</h1>
          </section>
          <section id="section-draft" class="settings-section" hidden>
            <h1>语音草稿</h1>
          </section>
          <section id="section-voice-edit" class="settings-section" hidden>
            <h1>说出修改</h1>
          </section>
          <section id="section-settings" class="settings-section" hidden>
            <h1>设置与权限</h1>
          </section>
          <section id="section-faq" class="settings-section" hidden>
            <h1>常见问题</h1>
          </section>
        </main>
        <footer class="guide-footer">
          <button type="button" id="btnOpenSettings">打开设置…</button>
        </footer>
      </div>
    </div>
    <script type="module" src="/src/guide.ts"></script>
  </body>
</html>
```

- [ ] **Step 3: 创建 `src/guide.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles-settings.css";

function showSection(id: string) {
  document.querySelectorAll<HTMLElement>(".settings-section").forEach((el) => {
    el.hidden = el.id !== `section-${id}`;
  });
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.section === id);
  });
}

async function main() {
  document.querySelectorAll<HTMLButtonElement>(".nav-item").forEach((btn) => {
    btn.addEventListener("click", () => showSection(btn.dataset.section || "start"));
  });

  document.querySelector("#btnOpenSettings")?.addEventListener("click", async () => {
    try {
      await invoke("open_settings_window", { section: "general" });
    } catch (err) {
      console.error("open settings failed", err);
    }
  });

  // Ensure title if opened via deep link later.
  try {
    await getCurrentWindow().setTitle("落字 · 如何使用");
  } catch {
    /* ignore in plain browser preview */
  }

  showSection("start");
}

void main();
```

- [ ] **Step 4: 在 `src/styles-settings.css` 末尾追加**

```css
.guide-shell {
  min-height: 100vh;
}

.guide-column {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 100vh;
}

.guide-column .settings-main {
  flex: 1;
}

.guide-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 28px 16px;
  border-top: 1px solid var(--line);
  background: var(--panel);
}

.guide-footer button {
  appearance: none;
  border: 1px solid var(--line);
  background: #fff;
  color: var(--text);
  border-radius: 8px;
  padding: 8px 14px;
  cursor: pointer;
  font-size: 13px;
}

.guide-footer button:hover {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--line));
  background: color-mix(in srgb, var(--accent) 10%, #fff);
}

.settings-section ol.guide-steps,
.settings-section ul.guide-list {
  margin: 0 0 14px;
  padding-left: 1.25rem;
}

.settings-section ol.guide-steps li,
.settings-section ul.guide-list li {
  margin: 0.45rem 0;
}

.settings-section kbd {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
  padding: 1px 6px;
  border-radius: 4px;
  border: 1px solid var(--line);
  background: #fff;
}
```

- [ ] **Step 5: 验证 Vite 能编进 guide**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
npm run build
test -f dist/guide.html && echo OK_GUIDE_HTML
```

Expected: build 成功；打印 `OK_GUIDE_HTML`。

- [ ] **Step 6: Commit（在 `apps/luozi` 独立 git 仓库）**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git add guide.html src/guide.ts vite.config.ts src/styles-settings.css
git commit -m "$(cat <<'EOF'
feat: scaffold guide window frontend shell

EOF
)"
```

---

### Task 2: 登记 Tauri 窗口 + capabilities + Rust 入口

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/capabilities/desktop.json`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `tauri.conf.json` 的 `app.windows` 在 settings 后追加**

```json
{
  "label": "guide",
  "title": "落字 · 如何使用",
  "url": "guide.html",
  "width": 720,
  "height": 560,
  "visible": false,
  "skipTaskbar": true
}
```

- [ ] **Step 2: capabilities**

`default.json` 与 `desktop.json` 的 `"windows"` 数组都加入 `"guide"`：

```json
"windows": ["main", "overlay", "settings", "guide"]
```

（`desktop.json` 若原本没有 `overlay`，保持原有项并追加 `guide`，例如 `["main", "settings", "guide"]`。）

- [ ] **Step 3: 在 `lib.rs` 增加 `show_guide` 与 `open_settings_window`**

放在 `show_about` 附近：

```rust
fn show_guide(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("guide") {
        let _ = window.set_title("落字 · 如何使用");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle, section: Option<String>) {
    show_settings(&app, section.as_deref().or(Some("general")));
}
```

注意：`show_settings` 签名已是 `show_settings(app, section: Option<&str>)`，上面 `or(Some("general"))` 在 `None` 时传入 `Some("general")`。

- [ ] **Step 4: 托盘菜单**

在 `build_tray` 里，`settings` 项之前增加：

```rust
let guide = MenuItem::with_id(app, "guide", "如何使用…", true, None::<&str>)?;
```

`Menu::with_items` 中放在 `&settings` 之前：

```rust
&guide,
&settings,
&about,
&quit,
```

`on_menu_event` 增加：

```rust
"guide" => show_guide(app),
```

- [ ] **Step 5: setup 里隐藏 + 背景色；关窗 hide**

在 setup 的 settings 块后：

```rust
if let Some(guide) = app.get_webview_window("guide") {
    let _ = guide.set_background_color(Some(tauri::window::Color(
        0xf4, 0xfb, 0xfa, 0xff,
    )));
    let _ = guide.hide();
}
```

`on_window_event` 的 settings 分支旁增加：

```rust
} else if window.label() == "guide" {
    api.prevent_close();
    let _ = window.hide();
}
```

- [ ] **Step 6: 注册 command**

在 `invoke_handler` 的 `generate_handler!` 列表加入 `open_settings_window`（靠近其它 settings_* 即可）。

- [ ] **Step 7: 编译检查**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi/src-tauri
cargo check
```

Expected: 无 error。

- [ ] **Step 8: Commit**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json src-tauri/capabilities/desktop.json src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat: wire tray How-to menu to guide window

EOF
)"
```

---

### Task 3: 填写六章新手教程正文

**Files:**
- Modify: `guide.html`（替换各 section 占位）

- [ ] **Step 1: 用下列正文替换各 `settings-section`（保持 id / hidden 逻辑）**

**快速开始 (`section-start`)**

```html
<section id="section-start" class="settings-section">
  <h1>快速开始</h1>
  <ol class="guide-steps">
    <li>把落字装到 <code>/Applications</code>（开发可用 <code>npm run install:macos-app</code>）。</li>
    <li>菜单栏出现落字图标后，<strong>右键</strong>打开托盘菜单（左键暂不是产品入口）。</li>
    <li>打开「设置…」→「权限」，允许麦克风与辅助功能。</li>
    <li>按住 <kbd>Control+Alt+Space</kbd> 说话，松手后开始转写。</li>
    <li>若「语音草稿」窗在前台且有焦点 → 文字写入草稿；否则尝试插入到当前 App，失败则复制到剪贴板。</li>
  </ol>
  <p class="muted">云端能力需自备 Key，并在设置里单独同意上传。默认优先本地转写。</p>
</section>
```

**快捷键 (`section-hotkeys`)**

```html
<section id="section-hotkeys" class="settings-section" hidden>
  <h1>快捷键</h1>
  <ul class="guide-list">
    <li><kbd>Control+Alt+Space</kbd>（按住）：语音输入 → 草稿或落字</li>
    <li><kbd>Control+Alt+Shift+Space</kbd>（按住）：说出修改要求 → 改草稿</li>
    <li><kbd>Escape</kbd>：取消当前录音 / 会话</li>
  </ul>
  <p>交互是<strong>按住说话，松手处理</strong>，不是点一下开关。</p>
  <p class="muted">上表为默认组合。若你改过键，以「设置 → 快捷键」显示为准。</p>
</section>
```

**语音草稿 (`section-draft`)**

```html
<section id="section-draft" class="settings-section" hidden>
  <h1>语音草稿</h1>
  <ul class="guide-list">
    <li>托盘「打开语音草稿」进入工作台。</li>
    <li>草稿有焦点时，语音结果写入正文；约 1 秒自动保存；关闭窗口会隐藏到托盘，内容仍在。</li>
    <li>支持撤销 / 重做、复制为纯文本、清空（清空需确认）。</li>
    <li>空草稿时可载入「最近落字」（短时间内外部落字成功后可用）。</li>
  </ul>
</section>
```

**说出修改 (`section-voice-edit`)**

```html
<section id="section-voice-edit" class="settings-section" hidden>
  <h1>说出修改</h1>
  <ol class="guide-steps">
    <li>先打开语音草稿，并保证正文里已有内容。</li>
    <li>在设置里配置<strong>文本 AI</strong> Key，并同意上传正文（与 ASR 分开）。</li>
    <li>按住 <kbd>Control+Alt+Shift+Space</kbd>，口头说修改要求，松手后模型改写草稿范围。</li>
    <li>进度与结果看屏幕 HUD / 草稿状态；失败时检查 Key、同意与网络。</li>
  </ol>
</section>
```

**设置与权限 (`section-settings`)**

```html
<section id="section-settings" class="settings-section" hidden>
  <h1>设置与权限</h1>
  <ul class="guide-list">
    <li><strong>引擎模式</strong>：自动 / 仅本地 / 仅云端（在设置里弹窗选择）。</li>
    <li><strong>本地模型</strong>：托盘「下载推荐模型…」；就绪后可不依赖云端转写。</li>
    <li><strong>云端 ASR / 文本 AI</strong>：分项配置 Key 与同意；厂商以设置页可选列表为准（如 Groq、千问、豆包、小米等）。</li>
    <li><strong>麦克风</strong>：录音必需。</li>
    <li><strong>辅助功能</strong>：向其他 App 插入文字；未授权时降级为剪贴板。</li>
  </ul>
  <p class="muted">ad-hoc 重装后签名变化，辅助功能里可能需要关掉再打开一次「落字」。</p>
  <p>点窗口底部「打开设置…」可直接进入设置。</p>
</section>
```

**常见问题 (`section-faq`)**

```html
<section id="section-faq" class="settings-section" hidden>
  <h1>常见问题</h1>
  <ul class="guide-list">
    <li><strong>辅助功能未生效？</strong>到系统设置检查；重装后开关一次「落字」。</li>
    <li><strong>没有本地模型？</strong>可先配云端 ASR，或下载推荐模型。</li>
    <li><strong>文字没进别人的 App？</strong>多半是辅助功能未授权，请到剪贴板粘贴。</li>
    <li><strong>没有首次引导？</strong>随时从托盘「如何使用…」打开本教程。</li>
    <li><strong>正式安装包？</strong>当前以开发安装路径为准；Release / 公证后续再做。</li>
  </ul>
</section>
```

- [ ] **Step 2: 确认章节切换**

用浏览器或 `npm run build` 后目视；或 `tauri dev` 后点托盘「如何使用…」切 6 章。

- [ ] **Step 3: Commit**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git add guide.html
git commit -m "$(cat <<'EOF'
docs(ui): fill beginner guide chapter copy

EOF
)"
```

---

### Task 4: 手工验收清单 + README / CHANGELOG

**Files:**
- Create: `docs/spikes/guide-manual.md`
- Modify: `README.zh-CN.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: 创建 `docs/spikes/guide-manual.md`**

```markdown
# 手工验收 — 如何使用（Guide 窗）

**目标：** 托盘能打开分章节新手教程；页脚能进设置。

## A. 入口

1. 托盘右键 → **如何使用…** → 独立窗「落字 · 如何使用」
2. 再点一次 → 同一窗前置，不新建
3. 关闭窗口 → App 仍在托盘；菜单可再次打开

## B. 章节

1. 左侧 6 项可切换：快速开始 / 快捷键 / 语音草稿 / 说出修改 / 设置与权限 / 常见问题
2. 默认打开「快速开始」
3. 文案无「已支持改键 / 开机启动 / Release 安装包」等误导

## C. 打开设置

1. 点页脚 **打开设置…** → 打开「落字 · 设置」（通用）

## 明确不测

- 首次自动弹教程
- 快捷键与用户自定义实时同步显示
```

- [ ] **Step 2: 更新 README**

`README.zh-CN.md`：
- 「当前可用」托盘一行补上「如何使用…」
- 「明确还没有」把「首次引导」改为「首次自动引导（菜单内教程已提供）」或等价表述

`README.md` 同步英文：tray includes How to use；first-run auto onboarding still absent, in-app guide available from tray.

- [ ] **Step 3: CHANGELOG Unreleased Added**

```markdown
- In-app beginner guide window from tray 「如何使用…」 (chaptered: start / hotkeys / draft / voice edit / settings / FAQ)
```

- [ ] **Step 4: 真机过一遍 `docs/spikes/guide-manual.md`**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
npm run install:macos-app   # 或项目惯用的 run:macos-app
```

按清单勾选 A–C。

- [ ] **Step 5: Commit**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git add docs/spikes/guide-manual.md README.zh-CN.md README.md CHANGELOG.md
git commit -m "$(cat <<'EOF'
docs: add guide manual checklist and announce tray How-to

EOF
)"
```

---

### Task 5: 最终验证

- [ ] **Step 1: 全量检查（按 AGENTS.md 推 PR 前清单的可执行子集）**

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
npm run build
test -f dist/guide.html
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: 全部通过；`dist/guide.html` 存在。

- [ ] **Step 2: 对照 spec 扫尾**

确认 spec 中「非目标」未误做：无首次自动弹、无 Markdown 渲染、无动态快捷键绑定。

---

## Spec coverage (self-review)

| Spec 项 | Task |
|---|---|
| 独立 guide 窗 | Task 1–2 |
| 托盘「如何使用…」 | Task 2 |
| 6 章节 + 默认快速开始 | Task 1 骨架 + Task 3 正文 |
| 页脚打开设置 | Task 1 + Task 2 `open_settings_window` |
| 关窗 hide | Task 2 |
| 冰白背景防暗角 | Task 2 |
| 默认快捷键 + 以设置页为准 | Task 3 快捷键章 |
| 手工验收 / README | Task 4 |
| 不做首次自动弹等 | Task 5 扫尾 |

无 TBD 占位；`open_settings_window` / `show_guide` / section id 在各 Task 一致。
