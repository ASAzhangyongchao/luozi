# Luozi P4 Configurable Single-Key Shortcuts Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 交付可录制、可更换、可回滚的语音输入、语音修改和取消快捷键，并在 Mac 能力实测通过后开放安全单键。

**Architecture:** `luozi-core` 定义平台无关 binding、gesture 和安全策略；`ShortcutManager` 通过 backend trait 原子注册；标准功能键与组合键使用 Tauri 插件，`Fn` / `Caps Lock` 先通过隔离 Spike，再决定是否启用 Mac 原生 adapter；前端只负责录制候选值和展示注册结果。

**Tech Stack:** Rust 2021、Tauri 2 global-shortcut、TypeScript、Vitest、macOS AppKit `NSEvent` 兼容性 Spike。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §7

**Prerequisite:** P3 已完成；实际菜单栏、HUD 和设置均读取统一设置 DTO。

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `crates/luozi-core/src/shortcuts.rs` | Create | 动作、手势、binding、禁止键和冲突规则 |
| `crates/luozi-core/src/config.rs` | Modify | 新快捷键配置与旧字符串迁移 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出快捷键类型 |
| `src-tauri/src/session/shortcut_manager.rs` | Create | backend trait、原子注册、回滚和运行态 |
| `src-tauri/src/session/shortcut_standard.rs` | Create | Tauri 标准 backend |
| `src-tauri/src/spike/macos_single_key.rs` | Create | Fn / Caps Lock 事件完整性 Spike |
| `src-tauri/src/session/shortcut_macos.rs` | Create | Mac 特殊单键生产 adapter；No-Go 时只暴露 false capability |
| `src-tauri/src/session/controller.rs` | Modify | 通过动作事件启动、停止、修改和取消 |
| `src-tauri/src/session/settings_api.rs` | Modify | 快捷键录制、候选验证和保存 DTO |
| `src-tauri/src/session/mod.rs` | Modify | 注册 manager 与 backend |
| `src-tauri/src/lib.rs` | Modify | 删除分散注册函数并暴露窄命令 |
| `settings.html` | Modify | 三个录制行和手势选择 |
| `src/settings/shortcuts.ts` | Create | 候选录制、冲突和保存 UI |
| `src/settings.ts` | Modify | 初始化快捷键分区 |
| `src/styles-settings.css` | Modify | 录制态、错误态和键帽 |
| `tests/frontend/shortcut-recorder.test.ts` | Create | 候选规范化和安全提示 |
| `docs/spikes/p4-macos-single-key-report.md` | Create | Fn / Caps Lock Go-No-Go 证据 |
| `docs/spikes/p4-shortcuts-manual.md` | Create | 最终快捷键实机矩阵 |

---

### Task 1: 定义快捷键类型和安全策略

**Files:**

- Create: `crates/luozi-core/src/shortcuts.rs`
- Modify: `crates/luozi-core/src/config.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写策略测试**

创建 `shortcuts.rs`：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutAction {
    Dictate,
    VoiceEdit,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutGesture {
    Hold,
    Toggle,
    DoubleTapHold,
    Press,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutBackendKind {
    Standard,
    MacSpecial,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub key: String,
    pub modifiers: Vec<String>,
    pub gesture: ShortcutGesture,
    pub backend: ShortcutBackendKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(key: &str, modifiers: &[&str]) -> ShortcutBinding {
        ShortcutBinding {
            key: key.into(),
            modifiers: modifiers.iter().map(|v| (*v).into()).collect(),
            gesture: ShortcutGesture::Hold,
            backend: ShortcutBackendKind::Standard,
        }
    }

    #[test]
    fn rejects_text_producing_global_single_keys() {
        assert_eq!(validate_binding(&candidate("A", &[])), Err("unsafe_text_key"));
        assert_eq!(validate_binding(&candidate("Space", &[])), Err("unsafe_text_key"));
        assert!(validate_binding(&candidate("F8", &[])).is_ok());
        assert!(validate_binding(&candidate("A", &["Control"])).is_ok());
    }

    #[test]
    fn rejects_identical_action_gestures() {
        let binding = candidate("F8", &[]);
        assert_eq!(
            validate_pair(&binding, &binding),
            Err("shortcut_conflict")
        );
    }
}
```

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi-core shortcuts
```

Expected：FAIL，缺少 `validate_binding` 和 `validate_pair`。

- [ ] **Step 3: 写最小策略**

禁止无 modifier 的 `A-Z`、`0-9`、`Space`、`Enter`、`Backspace`、`Tab` 和方向键；允许 `F1-F20`、`Escape` 和带至少一个 modifier 的标准键。`Fn` / `CapsLock` 必须使用 `MacSpecial`。

`validate_pair` 比较规范化 key、排序后的 modifiers、gesture 和 backend。

`AppConfig` 增加：

```rust
#[serde(default)]
pub shortcuts: ShortcutConfig,

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConfig {
    pub dictate: ShortcutBinding,
    pub voice_edit: ShortcutBinding,
    pub cancel: ShortcutBinding,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            dictate: ShortcutBinding {
                key: "F8".into(),
                modifiers: vec![],
                gesture: ShortcutGesture::Hold,
                backend: ShortcutBackendKind::Standard,
            },
            voice_edit: ShortcutBinding {
                key: "F9".into(),
                modifiers: vec![],
                gesture: ShortcutGesture::Hold,
                backend: ShortcutBackendKind::Standard,
            },
            cancel: ShortcutBinding {
                key: "Escape".into(),
                modifiers: vec![],
                gesture: ShortcutGesture::Press,
                backend: ShortcutBackendKind::Standard,
            },
        }
    }
}
```

`ShortcutConfig::default` 使用推荐单键；旧配置迁移时从现有 `continue_speaking_shortcut`、`voice_edit_shortcut` 和 `Escape` 生成同结构配置。旧字段保留一个迁移版本后再删除。

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi-core shortcuts
cargo test -p luozi-core config
git add crates/luozi-core/src/shortcuts.rs crates/luozi-core/src/config.rs crates/luozi-core/src/lib.rs
git commit -m "feat: define configurable shortcut policy"
```

---

### Task 2: 实现原子 ShortcutManager 和标准 backend

**Files:**

- Create: `src-tauri/src/session/shortcut_manager.rs`
- Create: `src-tauri/src/session/shortcut_standard.rs`
- Modify: `src-tauri/src/session/mod.rs`

- [ ] **Step 1: 写 fake backend 测试**

在 `shortcut_manager.rs` 定义：

```rust
use std::collections::HashMap;
use luozi_core::{ShortcutAction, ShortcutBinding};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShortcutCapabilities {
    pub standard_single_keys: bool,
    pub function_key: bool,
    pub caps_lock: bool,
}

pub trait ShortcutBackend {
    fn register(&mut self, action: ShortcutAction, binding: &ShortcutBinding) -> Result<(), String>;
    fn unregister(&mut self, action: ShortcutAction) -> Result<(), String>;
    fn capabilities(&self) -> ShortcutCapabilities;
}

pub struct ShortcutManager<B: ShortcutBackend> {
    backend: B,
    active: HashMap<ShortcutAction, ShortcutBinding>,
}
```

fake backend 测试：

1. 旧 binding 注销成功且新 binding 注册成功后更新 active。
2. 新 binding 注册失败时自动恢复旧 binding，active 仍是旧值。
3. 注销旧 binding 失败时不注册新值，并返回 `shortcut_replace_failed`。

- [ ] **Step 2: 运行红灯**

```bash
cargo test -p luozi shortcut_manager
```

Expected：FAIL，缺少 `replace`。

- [ ] **Step 3: 写原子替换**

```rust
pub fn replace(
    &mut self,
    action: ShortcutAction,
    next: ShortcutBinding,
) -> Result<(), String> {
    luozi_core::validate_binding(&next).map_err(str::to_string)?;
    let previous = self.active.get(&action).cloned();

    if previous.is_some() {
        self.backend
            .unregister(action)
            .map_err(|error| format!("shortcut_replace_failed:{error}"))?;
    }

    if let Err(error) = self.backend.register(action, &next) {
        if let Some(previous) = previous {
            let _ = self.backend.register(action, &previous);
        }
        return Err(format!("shortcut_replace_failed:{error}"));
    }

    self.active.insert(action, next);
    Ok(())
}
```

- [ ] **Step 4: 实现标准 backend**

`shortcut_standard.rs` 把 `ShortcutBinding` 转成 Tauri `Shortcut` 字符串。无 modifier 的功能键保持 `F8` 形式；modifier 使用固定顺序 `Control+Alt+Shift+Super+Key`。Tauri 回调只根据注册时传入的 `ShortcutAction` 发送 `{action, pressed}` 事件，不直接操作会话。

- [ ] **Step 5: 运行测试并提交**

```bash
cargo test -p luozi shortcut_manager
cargo check -p luozi
git add src-tauri/src/session/shortcut_manager.rs src-tauri/src/session/shortcut_standard.rs src-tauri/src/session/mod.rs
git commit -m "feat: add atomic shortcut manager"
```

---

### Task 3: 执行 Mac 特殊单键兼容性 Spike

**Files:**

- Create: `src-tauri/src/spike/macos_single_key.rs`
- Modify: `src-tauri/src/spike/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `docs/spikes/p4-macos-single-key-report.md`

- [ ] **Step 1: 建立事件状态机测试**

定义纯状态机：

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpecialKeyState {
    pub pressed: bool,
    pub presses: u32,
    pub releases: u32,
}

impl SpecialKeyState {
    pub fn observe(&mut self, down: bool) {
        if down && !self.pressed {
            self.pressed = true;
            self.presses += 1;
        } else if !down && self.pressed {
            self.pressed = false;
            self.releases += 1;
        }
    }
}
```

测试重复 flagsChanged 不重复计数，按下 / 松开各一次。

- [ ] **Step 2: 增加开发态 AppKit monitor**

Mac-only 使用 `NSEvent` 全局 monitor 监听 `.flagsChanged`，只识别：

- keyCode `63` 且 `.function` flag 变化：Fn。
- keyCode `57` 且 `.capsLock` flag 变化：Caps Lock。

非 Mac 返回 `unsupported_platform`。Spike 只在显式命令 `spike_start_single_key_probe` 后运行 15 秒；结束后移除 monitor。

AppKit 接入只保留最小骨架：

```rust
#[cfg(target_os = "macos")]
pub fn start_single_key_probe(app: tauri::AppHandle) -> Result<(), String> {
    // 生产代码放在 macOS cfg 模块中，测试只覆盖 SpecialKeyState。
    // unsafe / objc 调用必须封装在本函数内，外部只接收 ShortcutEvent。
    install_global_flags_changed_monitor(move |key_code, flags| {
        if key_code == 63 {
            emit_probe_event(&app, "Fn", flags.contains_function());
        } else if key_code == 57 {
            emit_probe_event(&app, "CapsLock", flags.contains_caps_lock());
        }
    })
}
```

- [ ] **Step 3: 实机采集**

每个按键执行：

- 按住 / 松开 20 次。
- 快速短按 20 次。
- 与系统输入法切换、听写、媒体键设置组合验证。
- App 前台、后台、锁屏恢复后各验证。

Go 条件：

- 40 次测试全部产生一对 press / release。
- 无重复触发和卡住 pressed。
- 五分钟 idle CPU 增量低于 `0.1%`。
- 拒绝权限后 App 仍可使用标准快捷键。

- [ ] **Step 4: 写报告**

`p4-macos-single-key-report.md` 必须分别给 Fn 和 Caps Lock 写 `Go` 或 `No-Go`、macOS 版本、键盘类型、事件计数、权限和系统冲突。No-Go 的按键不得进入 Task 4。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/spike/macos_single_key.rs src-tauri/src/spike/mod.rs src-tauri/src/lib.rs docs/spikes/p4-macos-single-key-report.md
git commit -m "spike: verify macOS special single keys"
```

---

### Task 4: 把 Spike Go 的单键接入生产 adapter

**Files:**

- Create: `src-tauri/src/session/shortcut_macos.rs`
- Modify: `src-tauri/src/session/shortcut_manager.rs`
- Modify: `src-tauri/src/session/mod.rs`

- [ ] **Step 1: 写 capability 测试**

`shortcut_macos.rs` 导出：

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MacSpecialCapabilities {
    pub function_key: bool,
    pub caps_lock: bool,
}
```

能力值来自 Task 3 报告对应的编译常量；No-Go 必须为 `false`。测试确保 `supports("Fn")` / `supports("CapsLock")` 与常量一致。

- [ ] **Step 2: 实现生产 listener**

复用 Spike 已验证的 AppKit monitor，但生产版只监听当前 active binding。回调发送 press / release 到 ShortcutManager，不记录其他 keyCode。切换 binding 时先启动新 monitor，成功后移除旧 monitor。No-Go 的按键在 capability 中为 `false`，`register` 直接返回 `special_key_not_supported`。

- [ ] **Step 3: 权限失败降级**

启动返回权限错误时：

- 不修改 active config。
- 返回 `special_key_permission_required`。
- 设置页展示打开系统设置和“选择其他按键”。
- 标准 backend 继续工作。

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi shortcut_macos
cargo test -p luozi shortcut_manager
git add src-tauri/src/session/shortcut_macos.rs src-tauri/src/session/shortcut_manager.rs src-tauri/src/session/mod.rs
git commit -m "feat: add verified macOS single-key adapter"
```

如果两个特殊键均为 No-Go，本 Task 只提交 capability 为 false 的模块和测试，不创建事件 monitor；标准功能键仍满足可配置单键交付。

---

### Task 5: 统一运行时动作分发

**Files:**

- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/session/controller.rs`
- Modify: `src-tauri/src/session/settings_api.rs`

- [ ] **Step 1: 写动作映射测试**

定义：

```rust
pub enum ShortcutEvent {
    Pressed(luozi_core::ShortcutAction),
    Released(luozi_core::ShortcutAction),
}
```

测试 Hold：Dictate pressed 开始、released 停止；Toggle：两次 pressed 切换；Cancel press 调用 cancel；VoiceEdit 使用独立 intent。

- [ ] **Step 2: 删除分散候选回退**

删除 `register_session_shortcuts` 中硬编码的候选数组。启动时由 ShortcutManager 注册配置；单项失败只标记该项失败，不静默换成另一个用户未选择的键。

- [ ] **Step 3: 暴露设置命令**

注册：

```rust
settings_validate_shortcut_candidate
settings_begin_shortcut_recording
settings_cancel_shortcut_recording
settings_save_shortcut
settings_reset_shortcut
settings_shortcut_capabilities
```

`settings_begin_shortcut_recording(action)` 进入 8 秒录制窗口；标准键由前端 `keydown` 生成候选，`Fn` / `Caps Lock` 只能由 native adapter 发送 `shortcut://candidate` 事件生成候选。`settings_cancel_shortcut_recording` 必须移除临时监听。

`settings_save_shortcut` 顺序固定为：policy 校验 → 冲突校验 → manager 原子替换 → config 原子保存。配置保存失败时 manager 恢复旧 binding。

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi shortcut
cargo test -p luozi controller
git add src-tauri/src/lib.rs src-tauri/src/session/controller.rs src-tauri/src/session/settings_api.rs
git commit -m "feat: dispatch configurable shortcut actions"
```

---

### Task 6: 实现录制式快捷键设置

**Files:**

- Modify: `settings.html`
- Create: `src/settings/shortcuts.ts`
- Modify: `src/settings.ts`
- Modify: `src/styles-settings.css`
- Create: `tests/frontend/shortcut-recorder.test.ts`

- [ ] **Step 1: 写候选规范化测试**

`shortcuts.ts` 导出：

```ts
export function normalizeRecordedKey(event: KeyboardEvent): {
  key: string;
  modifiers: string[];
} {
  const modifiers = [
    event.ctrlKey ? "Control" : "",
    event.altKey ? "Alt" : "",
    event.shiftKey ? "Shift" : "",
    event.metaKey ? "Super" : "",
  ].filter(Boolean);
  const key = event.code.startsWith("Key")
    ? event.code.slice(3)
    : event.code.startsWith("Digit")
      ? event.code.slice(5)
      : event.key;
  return { key, modifiers };
}
```

测试 F8 单键、Control+A、Space 单键和 Escape。

- [ ] **Step 2: 创建三行录制 UI**

每行动作包含 label、当前键帽、`更换`、`恢复推荐值`。点击更换后进入录制态：

```text
请按下要使用的按键 · Esc 退出录制
```

录制只形成候选值；后端验证成功后才显示保存成功。

录制流程固定：

1. 点击“更换”调用 `settings_begin_shortcut_recording(action)`。
2. 普通键由 `keydown` 调用 `normalizeRecordedKey` 形成候选。
3. 特殊键由 `shortcut://candidate` 事件返回 `{ action, key, modifiers, backend }`。
4. 前端调用 `settings_validate_shortcut_candidate` 显示可用、冲突或权限提示。
5. 用户确认后调用 `settings_save_shortcut`；Esc 或超时调用 `settings_cancel_shortcut_recording`。

- [ ] **Step 3: 处理安全和冲突**

- `unsafe_text_key`：说明该按键会阻断正常输入。
- `shortcut_conflict`：指出与哪个 Luozi 动作冲突。
- `system_reserved`：提示系统占用。
- `special_key_permission_required`：提供权限入口和其他单键选择。
- 注册失败：保持原键帽。

- [ ] **Step 4: 增加手势选择**

语音输入支持“按住说话”和“按一下开始，再按一下结束”；语音修改支持独立 binding 或 `DoubleTapHold`。默认不显示高级手势，展开后可选。

- [ ] **Step 5: 运行检查并提交**

```bash
npm run test:frontend -- tests/frontend/shortcut-recorder.test.ts
npm run build
git add settings.html src/settings/shortcuts.ts src/settings.ts src/styles-settings.css tests/frontend/shortcut-recorder.test.ts
git commit -m "feat: add shortcut recording settings"
```

---

### Task 7: 快捷键实机矩阵

**Files:**

- Create: `docs/spikes/p4-shortcuts-manual.md`

- [ ] **Step 1: 验证标准单键**

至少测试两个 F-key，各执行短按、按住 1 秒、按住 5 秒、快速重复 20 次。

- [ ] **Step 2: 验证三种动作**

- Dictate Hold / Toggle。
- VoiceEdit 独立键 / DoubleTapHold。
- Cancel 自定义和恢复 Escape。

- [ ] **Step 3: 验证失败恢复**

冲突键、字母单键、系统保留键、权限拒绝、配置写入失败都必须保持旧 binding 可用。

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
git add docs/spikes/p4-shortcuts-manual.md
git commit -m "docs: record configurable shortcut acceptance"
```
