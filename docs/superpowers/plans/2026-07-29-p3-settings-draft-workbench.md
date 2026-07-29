# Luozi P3 Settings and Draft Workbench Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 交付紧凑、响应式的六分区设置，以及支持疑似错误、AI 明显修改、语音修改范围的轻量草稿工作台。

**Architecture:** 设置保留单窗口但把纯类型、导航和各分区控制器拆开；统一 token 只提供语义，不在每页重复颜色；草稿继续使用 textarea，通过同步 mirror 渲染标记，避免引入富文本编辑器和常驻大型依赖。

**Tech Stack:** TypeScript、Vite 6、Vitest、原生 HTML/CSS、Tauri 2、Rust 2021。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §3、§6.6、§8、§9

**Prerequisite:** P2 已完成；设置 DTO 和 RecognitionOutcome 已稳定。

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `src/design-tokens.css` | Create | 珍珠玻璃语义 token 与无障碍回退 |
| `tests/frontend/design-tokens.test.ts` | Create | token 和媒体查询约束 |
| `settings.html` | Modify | 六分区语义结构 |
| `src/settings/navigation.ts` | Create | 纯导航状态和响应式 section 切换 |
| `src/settings/general.ts` | Create | 通用与语音输入页 |
| `src/settings/permissions.ts` | Create | 权限页和可见时轮询 |
| `src/settings.ts` | Modify | 设置入口编排 |
| `src/styles-settings.css` | Modify | 桌面密度、液态玻璃边界、窄窗口布局 |
| `tests/frontend/settings-navigation.test.ts` | Create | 顺序、唯一主动作和窄窗口模型 |
| `crates/luozi-core/src/annotations.rs` | Create | 标记类型、范围校验和纯文字降级 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出标记模型 |
| `src-tauri/src/session/draft.rs` | Modify | 草稿标记状态与交叉编辑清理 |
| `src-tauri/src/session/controller.rs` | Modify | 把 RecognitionOutcome 标记送入草稿 |
| `index.html` | Modify | editor mirror 与底部语音动作 |
| `src/draft/annotation-model.ts` | Create | 标记排序、切片和 CSS class 映射 |
| `src/draft/editor-mirror.ts` | Create | textarea 滚动、尺寸和内容同步 |
| `src/main.ts` | Modify | 草稿编排和事件处理 |
| `src/styles.css` | Modify | 三段布局与轻量标记 |
| `tests/frontend/draft-annotations.test.ts` | Create | 标记切片和 HTML 转义 |
| `docs/spikes/p3-settings-draft-manual.md` | Create | 设置与草稿视觉验收 |

---

### Task 1: 建立统一视觉 token

**Files:**

- Create: `src/design-tokens.css`
- Create: `tests/frontend/design-tokens.test.ts`
- Modify: `src/styles-settings.css`
- Modify: `src/styles.css`
- Modify: `src/overlay.css`

- [ ] **Step 1: 写 token 测试**

测试读取 CSS 并锁定：

```ts
expect(css).toContain("--surface-pearl: #fbfdf9");
expect(css).toContain("--surface-mist: #eef7f5");
expect(css).toContain("--text-ink: #173a3a");
expect(css).toContain("--brand-mint: #62d5c7");
expect(css).toContain("--brand-violet: #8b92ff");
expect(css).toContain("--state-error: #f1786d");
expect(css).toContain("@media (prefers-reduced-motion: reduce)");
expect(css).toContain("@media (prefers-reduced-transparency: reduce)");
```

- [ ] **Step 2: 运行红灯**

```bash
npm run test:frontend -- tests/frontend/design-tokens.test.ts
```

Expected：FAIL，文件不存在。

- [ ] **Step 3: 创建 token**

`src/design-tokens.css` 定义上述颜色、`8/16/24 px` 间距、`8/14/999 px` 圆角、系统字体和两类无障碍媒体查询。深色模式只替换 surface / text，不改变状态色语义。

三个现有 CSS 文件第一行统一：

```css
@import "./design-tokens.css";
```

删除重复品牌色，旧变量只能映射到新 token。

- [ ] **Step 4: 运行测试与构建**

```bash
npm run test:frontend -- tests/frontend/design-tokens.test.ts
npm run build
```

Expected：成功。

- [ ] **Step 5: 提交**

```bash
git add src/design-tokens.css src/styles-settings.css src/styles.css src/overlay.css tests/frontend/design-tokens.test.ts
git commit -m "style: unify Luozi visual tokens"
```

---

### Task 2: 重建六分区设置壳

**Files:**

- Modify: `settings.html`
- Create: `src/settings/navigation.ts`
- Create: `src/settings/general.ts`
- Create: `src/settings/permissions.ts`
- Modify: `src/settings.ts`
- Modify: `src/styles-settings.css`
- Create: `tests/frontend/settings-navigation.test.ts`

- [ ] **Step 1: 写导航模型测试**

`navigation.ts`：

```ts
export const SETTINGS_SECTIONS = [
  "general",
  "voice",
  "ai-services",
  "shortcuts",
  "permissions",
  "advanced",
] as const;

export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

export function isSettingsSection(value: string): value is SettingsSection {
  return SETTINGS_SECTIONS.includes(value as SettingsSection);
}

export function settingsLayout(width: number): "sidebar" | "topbar" {
  return width < 720 ? "topbar" : "sidebar";
}
```

测试顺序严格等于“通用、语音输入、AI 服务、快捷键、隐私与权限、高级与关于”，719 为 topbar，720 为 sidebar。

- [ ] **Step 2: 运行测试**

```bash
npm run test:frontend -- tests/frontend/settings-navigation.test.ts
```

Expected：PASS。

- [ ] **Step 3: 调整 HTML**

`settings.html` 保留六个 `.nav-item` 和六个 `.settings-section`。删除每行问号按钮；需要说明的行使用：

```html
<div class="setting-copy">
  <strong>智能落字</strong>
  <p>识别后进行受约束的文字整理；超时自动使用基础转写。</p>
</div>
```

每个 section 只能有一个 `.primary` 按钮；危险操作使用 `.danger`，已连接后的次级操作使用 `.quiet`。

- [ ] **Step 4: 拆分前端控制器**

- `general.ts`：模式、回退和通用设置。
- `permissions.ts`：权限 DTO、打开系统设置、仅 section 可见时的 1600 ms 轮询。
- `ai-services.ts`：P1 连接页面。
- `settings.ts`：初始化、导航、窗口 focus refresh 和 close cleanup。

`settings.ts` 关闭前必须调用每个分区控制器返回的 cleanup。

- [ ] **Step 5: 实现响应式 CSS**

固定规则：

```css
.settings-shell {
  display: grid;
  grid-template-columns: 176px minmax(0, 1fr);
  min-width: 0;
}

@media (max-width: 719px) {
  .settings-shell {
    display: block;
  }
  .settings-nav {
    display: flex;
    overflow-x: auto;
    border-right: 0;
    border-bottom: 1px solid var(--line-soft);
  }
  .setting-row {
    align-items: stretch;
    flex-direction: column;
  }
}
```

不得设置导致横向裁切的固定内容宽度。

- [ ] **Step 6: 验证并提交**

```bash
npm run test:frontend -- tests/frontend/settings-navigation.test.ts
npm run build
git add settings.html src/settings/navigation.ts src/settings/general.ts src/settings/permissions.ts src/settings.ts src/styles-settings.css tests/frontend/settings-navigation.test.ts
git commit -m "feat: rebuild responsive settings shell"
```

---

### Task 3: 定义草稿标记模型

**Files:**

- Create: `crates/luozi-core/src/annotations.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写范围测试**

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationKind {
    SuspectedError,
    AiChanged,
    VoiceEditScope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAnnotation {
    pub start: usize,
    pub end: usize,
    pub kind: AnnotationKind,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_utf8_ranges_and_drops_intersections() {
        let text = "落字测试";
        let mark = DraftAnnotation {
            start: 0,
            end: "落字".len(),
            kind: AnnotationKind::SuspectedError,
            reason: "low_confidence".into(),
        };
        assert!(validate_annotation(text, &mark).is_ok());
        assert!(mark_intersects_edit(&mark, 0, "落".len()));
    }
}
```

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi-core annotations
```

Expected：FAIL，缺少两个函数。

- [ ] **Step 3: 写实现**

`DraftAnnotation.start` / `end` 固定为 Rust 字节偏移；`validate_annotation` 校验 `start < end <= text.len()` 且两端为 UTF-8 边界；`mark_intersects_edit` 使用半开区间相交。增加 `plain_text(text)`，始终原样返回正文，证明标记不会污染复制结果。

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi-core annotations
git add crates/luozi-core/src/annotations.rs crates/luozi-core/src/lib.rs
git commit -m "feat: add draft annotation model"
```

---

### Task 4: 把识别标记接入草稿状态

**Files:**

- Modify: `src-tauri/src/session/draft.rs`
- Modify: `src-tauri/src/session/controller.rs`

- [ ] **Step 1: 写草稿标记状态测试**

扩展 `DraftStateDto`：

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAnnotationDto {
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub kind: luozi_core::AnnotationKind,
    pub reason: String,
}

pub annotations: Vec<DraftAnnotationDto>,
```

测试：

- 新 transcript 可携带标记。
- 手动编辑与标记相交时删除该标记。
- 不相交标记按字节偏移平移。
- `draft_save` 和 `draft_copy` 只保存 / 复制正文。
- 中文范围转换正确：`byte_to_utf16("落字A", 3) == 1`，`byte_to_utf16("落字A", 6) == 2`。

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi draft
```

Expected：FAIL，现有 DraftStore 没有 annotations。

- [ ] **Step 3: 写最小状态更新**

`DraftStore` 内部继续保存字节偏移：`annotations: Mutex<Vec<DraftAnnotation>>`。对外 snapshot 时转换为 UTF-16 DTO：

```rust
pub fn replace_annotations(&self, marks: Vec<DraftAnnotation>);
pub fn apply_text_edit(&self, start: usize, end: usize, replacement: &str);

pub fn byte_to_utf16(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].encode_utf16().count()
}

fn annotation_to_dto(text: &str, mark: &DraftAnnotation) -> DraftAnnotationDto {
    DraftAnnotationDto {
        start_utf16: byte_to_utf16(text, mark.start),
        end_utf16: byte_to_utf16(text, mark.end),
        kind: mark.kind,
        reason: mark.reason.clone(),
    }
}
```

`controller.rs` 在 RecognitionOutcome 写入草稿时把 suspicion 转换为 `SuspectedError`，AI diff 转换为 `AiChanged`，语音编辑范围转换为 `VoiceEditScope`。

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi draft
cargo test -p luozi controller
git add src-tauri/src/session/draft.rs src-tauri/src/session/controller.rs
git commit -m "feat: carry recognition annotations into drafts"
```

---

### Task 5: 用轻量 mirror 渲染草稿标记

**Files:**

- Modify: `index.html`
- Create: `src/draft/annotation-model.ts`
- Create: `src/draft/editor-mirror.ts`
- Modify: `src/main.ts`
- Modify: `src/styles.css`
- Create: `tests/frontend/draft-annotations.test.ts`

- [ ] **Step 1: 写切片与转义测试**

`annotation-model.ts` 导出：

```ts
export type DraftAnnotation = {
  startUtf16: number;
  endUtf16: number;
  kind: "suspectedError" | "aiChanged" | "voiceEditScope";
  reason: string;
};

export function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
```

`renderAnnotatedText` 使用 `startUtf16` / `endUtf16` 按 UTF-16 安全边界排序非重叠标记，输出 `<mark class="annotation annotation--suspected-error">` 等标签；非法或重叠标记降级为纯文本。

测试 `<script>` 被转义、三种 class 正确、非法范围不丢文字。

- [ ] **Step 2: 运行测试**

```bash
npm run test:frontend -- tests/frontend/draft-annotations.test.ts
```

Expected：PASS。

- [ ] **Step 3: 增加 mirror DOM**

编辑区结构：

```html
<div class="editor-stack">
  <div id="editorHighlights" class="editor-highlights" aria-hidden="true"></div>
  <textarea id="editor" aria-label="语音草稿正文"></textarea>
</div>
```

`editor-mirror.ts` 同步正文、scrollTop、scrollLeft、font、line-height、padding 和尺寸。mirror 使用 `pointer-events: none`，textarea 保持真实输入和选择。

- [ ] **Step 4: 接入状态**

`main.ts` 的 `DraftState` 增加 annotations。每次 `applyState` 先设置 textarea，再调用 mirror render；input 和 scroll 时同步。复制仍读取 `textarea.value`。

- [ ] **Step 5: 添加颜色规则**

- 红色波浪线：`text-decoration: underline wavy var(--state-error)`。
- 琥珀底纹：低透明度 warning surface。
- 蓝色边界：`outline`，不填充正文。
- 减少动态效果下无动画。

- [ ] **Step 6: 验证并提交**

```bash
npm run test:frontend -- tests/frontend/draft-annotations.test.ts
npm run build
git add index.html src/draft/annotation-model.ts src/draft/editor-mirror.ts src/main.ts src/styles.css tests/frontend/draft-annotations.test.ts
git commit -m "feat: render lightweight draft annotations"
```

---

### Task 6: 收口工作台主交互

**Files:**

- Modify: `index.html`
- Modify: `src/main.ts`
- Modify: `src/styles.css`

- [ ] **Step 1: 改成三段结构**

- 顶部：标题、模式、撤销。
- 中间：编辑器与标记图例。
- 底部：`继续说` 主动作、`润色`、`说修改要求`。

页面删除概念海报、长段解释和等权大卡片。按钮文案读取实际模式和快捷键。

- [ ] **Step 2: 接入语音动作**

主动作调用现有 start / stop 会话；润色调用 P2 Cleanup；说修改要求调用 VoiceEdit。若用户使用全局快捷键，按钮只作为发现和兜底。

- [ ] **Step 3: 加入问题摘要**

编辑器下方只显示：

```text
2 处疑似问题 · 1 处 AI 修改
```

点击摘要滚动到第一个标记；不弹出多层设置。

- [ ] **Step 4: 构建并提交**

```bash
npm run build
git add index.html src/main.ts src/styles.css
git commit -m "feat: simplify the voice draft workbench"
```

---

### Task 7: 设置与工作台验收

**Files:**

- Create: `docs/spikes/p3-settings-draft-manual.md`

- [ ] **Step 1: 视觉尺寸检查**

检查设置宽度 `760`、`719`、`600`，不得横向滚动或裁切。检查浅色、深色、降低透明度、增强对比度和减少动态效果。

- [ ] **Step 2: 草稿检查**

分别生成红、琥珀、蓝三类标记；确认复制得到纯文字，手动编辑只清理相交标记，长文滚动时 mirror 与 textarea 对齐。

- [ ] **Step 3: 资源检查**

设置停留在非权限页 30 秒，确认无权限轮询；草稿窗口隐藏后确认无 mirror 更新和动画。

- [ ] **Step 4: 全量验证**

```bash
npm run test:frontend
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected：全部成功。

- [ ] **Step 5: 提交**

```bash
git add docs/spikes/p3-settings-draft-manual.md
git commit -m "docs: record settings and draft acceptance"
```
