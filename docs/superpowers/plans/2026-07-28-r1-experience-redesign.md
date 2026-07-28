# Luozi R1 Experience Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不改写 ASR、文本 AI 和安全交付逻辑的前提下，交付统一的珍珠玻璃视觉令牌、可响应真实声音的「呼吸泡 + 轨道声场」HUD、主动作优先的菜单栏入口，以及重新分层的设置外壳。

**Architecture:** 保留当前 Tauri 2 多页结构和 Rust 会话控制器。Rust 继续作为 HUD 生命周期和会话真相源，并新增带 `sessionId` 的状态事件与最高 25 Hz 的归一化声音能量事件；前端只把语义状态和能量渲染为 SVG/CSS。设置与托盘只重排信息架构，不改变现有配置、钥匙串、授权和交付 API。

**Tech Stack:** Rust 2021、Tauri 2、CPAL、TypeScript、Vite 6、原生 HTML/CSS、Vitest。

**Spec:** `docs/superpowers/specs/2026-07-28-experience-redesign-design.md`

---

## 实施边界

本计划只实现设计规格中的 R1：

- 统一视觉 token；
- 呼吸泡 + 轨道声场 SVG 真相源；
- `listening`、`processing`、`success`、`error` 四类 HUD 核心体验；
- 菜单栏主动作与二级入口重排；
- 设置导航、通用页与高级配置分层；
- 工作台继续按需打开，不删除任何草稿能力。

本计划不创建首次引导、左键菜单浮层、历史窗口、快捷键录制、Key 自定义对话框、外部选中文本修改，也不重写云端提供方和跨 App 落字。

## 执行前置条件

当前工作树已有未提交的 M8 提供方与设置改动，并且与本计划将修改的 `settings.html`、`src/settings.ts`、`src/styles-settings.css`、`src-tauri/src/lib.rs`、`src-tauri/src/session/controller.rs` 重叠。

执行者开始 Task 1 前必须：

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git status --short
git diff --check
```

Expected:

- `git diff --check` 无输出；
- 现有 M8 改动已经由所有者提交为可恢复基线；
- 不允许自动 `stash`、覆盖或丢弃这些改动；
- 在独立 `codex/` 工作树中执行本计划。

如果上述基线仍为脏状态，停止执行并请求用户先确认如何处理现有改动。

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `src/design-tokens.css` | Create | 珍珠玻璃语义色、形状、阴影和可访问性 token |
| `tests/frontend/design-tokens.test.ts` | Create | 锁定核心 token 与无障碍媒体查询 |
| `package.json` / `package-lock.json` | Modify | 增加 Vitest 与前端测试命令 |
| `crates/luozi-core/src/voice_energy.rs` | Create | 纯函数能量测量、平滑、限幅与 25 Hz 节流 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出声音能量模型 |
| `src-tauri/src/session/recorder.rs` | Modify | 从 CPAL 样本生成归一化能量回调 |
| `src-tauri/src/session/controller.rs` | Modify | 发送带 `sessionId` 的 phase / energy 事件并统一 HUD 隐藏时长 |
| `src/assets/luozi-orbit-avatar.svg` | Create | 呼吸泡与轨道声场唯一 SVG 真相源 |
| `assets/brand/luozi-tray-template.svg` | Modify | 16 px 呼吸泡 + 单轨道菜单栏真相源 |
| `public/assets/brand/luozi-tray-template.svg` | Modify | 前端可访问的菜单栏 SVG 镜像 |
| `src-tauri/icons/tray-template.png` / `tray-template@2x.png` | Modify | Tauri 运行时单色 Template PNG |
| `assets/brand/README.md` | Modify | 更新品牌资产语义和生成说明 |
| `src/vite-env.d.ts` | Create | 支持 Vite `?raw` SVG 导入 |
| `src/overlay-model.ts` | Create | 会话事件到 HUD 语义状态的纯映射 |
| `tests/frontend/overlay-model.test.ts` | Create | 状态映射、晚到事件和自动消失时长测试 |
| `tests/frontend/brand-assets.test.ts` | Create | 锁定 16 px 菜单栏图标语义与单色约束 |
| `overlay.html` | Modify | HUD 角色、主文案和副文案语义壳 |
| `src/overlay.ts` | Modify | 渲染状态、能量和 stale session 防护 |
| `src/overlay.css` | Modify | 珍珠 HUD、轨道状态和减少动态效果 |
| `src-tauri/src/lib.rs` | Modify | 菜单顺序、产品文案与设置入口 |
| `tests/frontend/tray-ia.test.ts` | Create | 锁定菜单主动作和入口顺序 |
| `settings.html` | Modify | 六组设置导航与普通 / 高级分层 |
| `src/settings.ts` | Modify | 新 section id 与权限轮询生命周期 |
| `src/styles-settings.css` | Modify | 桌面密度、浅色层级和状态控件 |
| `tests/frontend/settings-ia.test.ts` | Create | 导航顺序与技术配置归属测试 |
| `src-tauri/tauri.conf.json` | Modify | 将 HUD 尺寸调整到目标范围 |
| `docs/spikes/r1-experience-manual.md` | Create | R1 Mac 实机与视觉验收清单 |

---

### Task 1: 建立前端测试入口和统一视觉 token

**Files:**

- Create: `src/design-tokens.css`
- Create: `tests/frontend/design-tokens.test.ts`
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src/overlay.css`
- Modify: `src/styles-settings.css`

- [ ] **Step 1: 写 token 约束测试**

创建 `tests/frontend/design-tokens.test.ts`：

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "src/design-tokens.css"), "utf8").toLowerCase();

describe("luozi design tokens", () => {
  it("defines the approved semantic palette", () => {
    expect(css).toContain("--surface-pearl: #fbfdf9");
    expect(css).toContain("--surface-mist: #eef7f5");
    expect(css).toContain("--text-ink: #173a3a");
    expect(css).toContain("--brand-mint: #62d5c7");
    expect(css).toContain("--brand-violet: #8b92ff");
    expect(css).toContain("--state-success: #50c59f");
    expect(css).toContain("--state-error: #f1786d");
  });

  it("contains reduced motion and reduced transparency fallbacks", () => {
    expect(css).toContain("@media (prefers-reduced-motion: reduce)");
    expect(css).toContain("@media (prefers-reduced-transparency: reduce)");
  });
});
```

- [ ] **Step 2: 安装并登记 Vitest**

Run:

```bash
npm install --save-dev vitest
```

将 `package.json` 的 scripts 更新为：

```json
{
  "dev": "vite",
  "build": "tsc && vite build",
  "test:frontend": "vitest run",
  "preview": "vite preview",
  "tauri": "tauri",
  "run:macos-app": "bash scripts/run-macos-app.sh",
  "install:macos-app": "bash scripts/install-macos-app.sh",
  "fetch:model": "bash scripts/fetch-ggml-small.sh"
}
```

- [ ] **Step 3: 运行测试并确认红灯**

Run:

```bash
npm run test:frontend -- tests/frontend/design-tokens.test.ts
```

Expected: FAIL，错误指出 `src/design-tokens.css` 不存在。

- [ ] **Step 4: 创建统一 token 文件**

创建 `src/design-tokens.css`：

```css
:root {
  color-scheme: light dark;
  --surface-pearl: #fbfdf9;
  --surface-mist: #eef7f5;
  --surface-solid: #ffffff;
  --text-ink: #173a3a;
  --text-muted: #688181;
  --line-soft: color-mix(in srgb, var(--text-ink) 12%, transparent);
  --brand-mint: #62d5c7;
  --brand-violet: #8b92ff;
  --state-success: #50c59f;
  --state-error: #f1786d;
  --state-warning: #d99b4d;
  --shadow-float: 0 12px 36px color-mix(in srgb, #163434 16%, transparent);
  --radius-control: 8px;
  --radius-panel: 14px;
  --radius-hud: 999px;
  --space-1: 8px;
  --space-2: 16px;
  --space-3: 24px;
  --motion-fast: 160ms;
  --motion-state: 600ms;
  font-family:
    "PingFang SC",
    -apple-system,
    BlinkMacSystemFont,
    "SF Pro Text",
    "Segoe UI",
    system-ui,
    sans-serif;
}

@media (prefers-color-scheme: dark) {
  :root {
    --surface-pearl: #182323;
    --surface-mist: #202e2e;
    --surface-solid: #253333;
    --text-ink: #eef8f5;
    --text-muted: #9db2b0;
    --line-soft: color-mix(in srgb, #eef8f5 14%, transparent);
    --shadow-float: 0 16px 42px color-mix(in srgb, #000 42%, transparent);
  }
}

@media (prefers-reduced-motion: reduce) {
  :root {
    --motion-fast: 0ms;
    --motion-state: 0ms;
  }
}

@media (prefers-reduced-transparency: reduce) {
  :root {
    --surface-pearl: var(--surface-solid);
    --shadow-float: 0 8px 24px color-mix(in srgb, #163434 14%, transparent);
  }
}
```

在 `src/overlay.css` 和 `src/styles-settings.css` 第一行加入：

```css
@import "./design-tokens.css";
```

删除这两个文件各自重复的品牌色声明，并让旧组件变量映射到语义 token：

```css
:root {
  --bg: var(--surface-mist);
  --panel: var(--surface-pearl);
  --line: var(--line-soft);
  --text: var(--text-ink);
  --muted: var(--text-muted);
  --accent: var(--brand-mint);
  --ok: var(--state-success);
}
```

- [ ] **Step 5: 运行测试并确认绿灯**

Run:

```bash
npm run test:frontend -- tests/frontend/design-tokens.test.ts
npm run build
```

Expected: 2 tests PASS；TypeScript 与 Vite build 成功。

- [ ] **Step 6: 提交**

```bash
git add package.json package-lock.json src/design-tokens.css src/overlay.css src/styles-settings.css tests/frontend/design-tokens.test.ts
git commit -m "style: add Luozi semantic design tokens"
```

---

### Task 2: 实现可测试的声音能量模型

**Files:**

- Create: `crates/luozi-core/src/voice_energy.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 先写能量模型测试**

创建 `crates/luozi-core/src/voice_energy.rs`，先只放以下测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_at_zero() {
        let mut meter = VoiceEnergyMeter::default();
        assert_eq!(meter.observe_levels(0.0, 0.0), 0.0);
    }

    #[test]
    fn louder_input_produces_more_energy_and_is_clamped() {
        let mut quiet = VoiceEnergyMeter::default();
        let mut loud = VoiceEnergyMeter::default();
        let quiet_value = quiet.observe_levels(0.03, 0.05);
        let loud_value = loud.observe_levels(0.30, 0.80);
        assert!(loud_value > quiet_value);
        assert!((0.0..=1.0).contains(&loud_value));
    }

    #[test]
    fn release_falls_instead_of_snapping_to_zero() {
        let mut meter = VoiceEnergyMeter::default();
        let loud = meter.observe_levels(0.40, 0.90);
        let released = meter.observe_levels(0.0, 0.0);
        assert!(released > 0.0);
        assert!(released < loud);
    }

    #[test]
    fn throttle_emits_no_more_than_twenty_five_times_per_second() {
        let mut throttle = EnergyThrottle::new(40);
        assert!(throttle.should_emit(0));
        assert!(!throttle.should_emit(20));
        assert!(throttle.should_emit(40));
        assert!(!throttle.should_emit(79));
        assert!(throttle.should_emit(80));
    }
}
```

在 `crates/luozi-core/src/lib.rs` 增加：

```rust
mod voice_energy;

pub use voice_energy::{measure_levels, EnergyThrottle, VoiceEnergyMeter};
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```bash
cargo test -p luozi-core voice_energy
```

Expected: FAIL，缺少 `VoiceEnergyMeter`、`EnergyThrottle` 和 `measure_levels`。

- [ ] **Step 3: 写最小声音能量实现**

在测试代码之前加入：

```rust
const NOISE_FLOOR: f32 = 0.015;
const ACTIVE_CEILING: f32 = 0.42;
const ATTACK: f32 = 0.58;
const RELEASE: f32 = 0.16;

#[derive(Clone, Debug, Default)]
pub struct VoiceEnergyMeter {
    smoothed: f32,
}

impl VoiceEnergyMeter {
    pub fn observe_levels(&mut self, rms: f32, peak: f32) -> f32 {
        let mixed = (rms.max(0.0) * 0.72 + peak.max(0.0) * 0.28).clamp(0.0, 1.0);
        let target =
            ((mixed - NOISE_FLOOR) / (ACTIVE_CEILING - NOISE_FLOOR)).clamp(0.0, 1.0);
        let coefficient = if target > self.smoothed {
            ATTACK
        } else {
            RELEASE
        };
        self.smoothed += (target - self.smoothed) * coefficient;
        if self.smoothed < 0.005 {
            self.smoothed = 0.0;
        }
        self.smoothed.clamp(0.0, 1.0)
    }
}

pub fn measure_levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum_squares = 0.0_f32;
    let mut peak = 0.0_f32;
    for sample in samples {
        let value = sample.clamp(-1.0, 1.0);
        sum_squares += value * value;
        peak = peak.max(value.abs());
    }
    ((sum_squares / samples.len() as f32).sqrt(), peak)
}

#[derive(Clone, Debug)]
pub struct EnergyThrottle {
    interval_ms: u64,
    last_emit_ms: Option<u64>,
}

impl EnergyThrottle {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms: interval_ms.max(1),
            last_emit_ms: None,
        }
    }

    pub fn should_emit(&mut self, now_ms: u64) -> bool {
        let due = self
            .last_emit_ms
            .map(|last| now_ms.saturating_sub(last) >= self.interval_ms)
            .unwrap_or(true);
        if due {
            self.last_emit_ms = Some(now_ms);
        }
        due
    }
}
```

- [ ] **Step 4: 运行聚焦测试**

Run:

```bash
cargo test -p luozi-core voice_energy
```

Expected: 4 tests PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/lib.rs crates/luozi-core/src/voice_energy.rs
git commit -m "feat: add smoothed voice energy model"
```

---

### Task 3: 从 CPAL 录音链路发送限频能量

**Files:**

- Modify: `src-tauri/src/session/recorder.rs`
- Modify: `src-tauri/src/session/controller.rs`

- [ ] **Step 1: 定义 recorder 能量回调**

在 `src-tauri/src/session/recorder.rs` 的 import 和常量附近加入：

```rust
use luozi_core::{EnergyThrottle, VoiceEnergyMeter};

pub type EnergySink = Arc<dyn Fn(f32) + Send + Sync + 'static>;

#[derive(Clone)]
struct EnergyReporter {
    state: Arc<Mutex<EnergyReporterState>>,
    started: Instant,
    sink: EnergySink,
}

struct EnergyReporterState {
    meter: VoiceEnergyMeter,
    throttle: EnergyThrottle,
}

impl EnergyReporter {
    fn new(sink: EnergySink) -> Self {
        Self {
            state: Arc::new(Mutex::new(EnergyReporterState {
                meter: VoiceEnergyMeter::default(),
                throttle: EnergyThrottle::new(40),
            })),
            started: Instant::now(),
            sink,
        }
    }

    fn observe_levels(&self, rms: f32, peak: f32) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let level = state.meter.observe_levels(rms, peak);
        if state.throttle.should_emit(now_ms) {
            (self.sink)(level);
        }
    }
}

fn append_normalized<I>(
    buffer: &Arc<Mutex<Vec<f32>>>,
    reporter: &EnergyReporter,
    samples: I,
) where
    I: IntoIterator<Item = f32>,
{
    let Ok(mut output) = buffer.lock() else {
        return;
    };
    let mut count = 0_usize;
    let mut sum_squares = 0.0_f32;
    let mut peak = 0.0_f32;
    for sample in samples {
        let value = sample.clamp(-1.0, 1.0);
        output.push(value);
        count += 1;
        sum_squares += value * value;
        peak = peak.max(value.abs());
    }
    drop(output);
    let rms = if count == 0 {
        0.0
    } else {
        (sum_squares / count as f32).sqrt()
    };
    reporter.observe_levels(rms, peak);
}
```

- [ ] **Step 2: 把 sink 接入 `SessionRecorder::start` 和 `build_stream`**

将签名改为：

```rust
pub fn start(&mut self, energy_sink: EnergySink) -> Result<(), String>
```

创建 reporter 并传入 `build_stream`：

```rust
let reporter = EnergyReporter::new(energy_sink);
let stream = build_stream(
    &device,
    &config,
    sample_format,
    samples.clone(),
    err.clone(),
    reporter,
)?;
```

将 `build_stream` 签名增加 `reporter: EnergyReporter`，并用下面完整的 sample-format match 替换现有 match：

```rust
let stream = match sample_format {
    SampleFormat::F32 => {
        let samples_cb = samples;
        let reporter_cb = reporter;
        device.build_input_stream(
            config.clone(),
            move |data: &[f32], _| {
                append_normalized(&samples_cb, &reporter_cb, data.iter().copied());
            },
            err_cb,
            None,
        )
    }
    SampleFormat::I16 => {
        let samples_cb = samples;
        let reporter_cb = reporter;
        device.build_input_stream(
            config.clone(),
            move |data: &[i16], _| {
                append_normalized(
                    &samples_cb,
                    &reporter_cb,
                    data.iter().map(|sample| *sample as f32 / i16::MAX as f32),
                );
            },
            err_cb,
            None,
        )
    }
    SampleFormat::I32 => {
        let samples_cb = samples;
        let reporter_cb = reporter;
        device.build_input_stream(
            config.clone(),
            move |data: &[i32], _| {
                append_normalized(
                    &samples_cb,
                    &reporter_cb,
                    data.iter().map(|sample| *sample as f32 / i32::MAX as f32),
                );
            },
            err_cb,
            None,
        )
    }
    SampleFormat::U16 => {
        let samples_cb = samples;
        let reporter_cb = reporter;
        device.build_input_stream(
            config.clone(),
            move |data: &[u16], _| {
                append_normalized(
                    &samples_cb,
                    &reporter_cb,
                    data.iter()
                        .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0),
                );
            },
            err_cb,
            None,
        )
    }
    other => return Err(format!("unsupported_sample_format: {other:?}")),
};
```

- [ ] **Step 3: 从 controller 发送 session-scoped 能量事件**

在 `src-tauri/src/session/controller.rs` 增加 `Arc` 与 `EnergySink` import：

```rust
use std::sync::{Arc, Mutex};
use super::recorder::{EnergySink, SessionRecorder, MAX_RECORDING_MS};
```

在 `SessionEffect::BeganRecording { session_id }` 中、调用 recorder `start` 之前加入：

```rust
let app_for_energy = app.clone();
let energy_sink: EnergySink = Arc::new(move |level| {
    let _ = app_for_energy.emit(
        "session://energy",
        serde_json::json!({
            "sessionId": session_id,
            "level": level,
        }),
    );
});
```

将 recorder 调用改为：

```rust
.start(energy_sink);
```

- [ ] **Step 4: 更新麦克风预热调用**

`warmup_microphone_async` 不属于用户会话，不发送 HUD 能量。把其中 `rec.start()` 替换为：

```rust
let energy_sink: EnergySink = Arc::new(|_| {});
match rec.start(energy_sink) {
```

函数后续的 `Ok(())`、sleep、`rec.stop()` 和错误处理保持不变。

- [ ] **Step 5: 检查编译和能量单元测试**

Run:

```bash
cargo fmt --all --check
cargo test -p luozi-core voice_energy
cargo check -p luozi
```

Expected: 命令全部成功；没有更改 ASR 或交付函数。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/session/recorder.rs src-tauri/src/session/controller.rs
git commit -m "feat: emit throttled session voice energy"
```

---

### Task 4: 统一 HUD 事件协议和生命周期

**Files:**

- Modify: `src-tauri/src/session/controller.rs`

- [ ] **Step 1: 先写 transient 时长测试**

在 `src-tauri/src/session/controller.rs` 末尾加入：

```rust
#[cfg(test)]
mod hud_tests {
    use super::transient_duration_ms;

    #[test]
    fn inserted_feedback_is_brief() {
        assert_eq!(transient_duration_ms("inserted"), 600);
    }

    #[test]
    fn clipboard_and_errors_remain_readable() {
        assert_eq!(transient_duration_ms("clipboard"), 3_500);
        assert_eq!(transient_duration_ms("error"), 5_000);
        assert_eq!(transient_duration_ms("rejected"), 5_000);
    }
}
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```bash
cargo test -p luozi hud_tests
```

Expected: FAIL，缺少 `transient_duration_ms`。

- [ ] **Step 3: 增加带 sessionId 的 payload 和单一时长函数**

用以下代码替换现有 `OVERLAY_AUTO_HIDE_MS`、`emit_phase` 和 `emit_transient`：

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HudPhasePayload<'a> {
    phase: &'a str,
    message: &'a str,
    session_id: Option<u64>,
}

fn active_session_id(app: &AppHandle) -> Option<u64> {
    app.try_state::<AppSessionState>().and_then(|state| {
        state
            .machine
            .lock()
            .ok()
            .and_then(|machine| machine.active_session_id())
    })
}

fn emit_phase(app: &AppHandle, phase: &str, message: &str) {
    let _ = app.emit(
        "session://phase",
        HudPhasePayload {
            phase,
            message,
            session_id: active_session_id(app),
        },
    );
}

fn transient_duration_ms(phase: &str) -> u64 {
    match phase {
        "inserted" | "undone" => 600,
        "canceled" | "too_short" => 1_600,
        "clipboard" => 3_500,
        "error" | "rejected" | "discarded" | "warn" => 5_000,
        _ => 2_200,
    }
}

fn emit_transient(app: &AppHandle, phase: &str, message: &str) {
    show_overlay(app, true);
    emit_phase(app, phase, message);
    let duration_ms = transient_duration_ms(phase);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms));
        if let Some(state) = app.try_state::<AppSessionState>() {
            let idle = state
                .machine
                .lock()
                .ok()
                .map(|machine| machine.is_idle())
                .unwrap_or(true);
            if idle {
                show_overlay(&app, false);
            }
        }
    });
}
```

删除旧的统一 `OVERLAY_AUTO_HIDE_MS` 常量。前端不再自行启动第二套隐藏计时器。

- [ ] **Step 4: 让监听态显示实际快捷键**

在 controller 增加：

```rust
fn registered_shortcut_label(state: &AppSessionState) -> String {
    state
        .registered_continue
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .unwrap_or_else(|| super::config_store::load().continue_speaking_shortcut)
}
```

把普通录音的两处 `emit_phase(..., "recording", ...)` 文案替换为：

```rust
let shortcut = registered_shortcut_label(state);
emit_phase(
    app,
    "recording",
    &format!("松开 {shortcut} 开始整理 · Esc 取消"),
);
```

后台焦点捕获线程中的同类文案替换为：

```rust
if let Some(state) = app_cap.try_state::<AppSessionState>() {
    if is_recording_phase(&state) {
        let shortcut = registered_shortcut_label(&state);
        emit_phase(
            &app_cap,
            "recording",
            &format!("松开 {shortcut} 开始整理 · Esc 取消"),
        );
    }
}
```

- [ ] **Step 5: 运行聚焦测试**

Run:

```bash
cargo fmt --all --check
cargo test -p luozi hud_tests
cargo check -p luozi
```

Expected: HUD tests PASS；Rust check 成功。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/session/controller.rs
git commit -m "refactor: make Rust own HUD event lifecycle"
```

---

### Task 5: 实现呼吸泡 SVG 与四类 HUD 状态

**Files:**

- Create: `src/assets/luozi-orbit-avatar.svg`
- Create: `src/vite-env.d.ts`
- Create: `src/overlay-model.ts`
- Create: `tests/frontend/overlay-model.test.ts`
- Create: `tests/frontend/brand-assets.test.ts`
- Modify: `assets/brand/luozi-tray-template.svg`
- Modify: `public/assets/brand/luozi-tray-template.svg`
- Modify: `src-tauri/icons/tray-template.png`
- Modify: `src-tauri/icons/tray-template@2x.png`
- Modify: `assets/brand/README.md`
- Modify: `overlay.html`
- Modify: `src/overlay.ts`
- Modify: `src/overlay.css`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 写 HUD 状态映射测试**

创建 `tests/frontend/overlay-model.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { initialHudState, reduceHudState } from "../../src/overlay-model";

describe("HUD state reducer", () => {
  it("maps recording, processing, inserted and error phases", () => {
    const listening = reduceHudState(initialHudState, {
      phase: "recording",
      message: "松开 Control+Alt+Space 开始整理 · Esc 取消",
      sessionId: 7,
    });
    expect(listening.kind).toBe("listening");
    expect(listening.title).toBe("正在听你说");

    const processing = reduceHudState(listening, {
      phase: "transcribing",
      message: "落字中",
      sessionId: 7,
    });
    expect(processing.kind).toBe("processing");

    const success = reduceHudState(processing, {
      phase: "inserted",
      message: "已落字",
      sessionId: 7,
    });
    expect(success.kind).toBe("success");

    const error = reduceHudState(processing, {
      phase: "error",
      message: "未检测到语音",
      sessionId: 7,
    });
    expect(error.kind).toBe("error");
  });

  it("ignores events from an older session", () => {
    const current = reduceHudState(initialHudState, {
      phase: "recording",
      message: "new",
      sessionId: 9,
    });
    const stale = reduceHudState(current, {
      phase: "inserted",
      message: "old",
      sessionId: 8,
    });
    expect(stale).toEqual(current);
  });

  it("distinguishes clipboard and permission outcomes", () => {
    const clipboard = reduceHudState(initialHudState, {
      phase: "clipboard",
      message: "未进输入框，已到剪贴板 · 请 ⌘V",
      sessionId: 2,
    });
    expect(clipboard.kind).toBe("clipboard");
    expect(clipboard.title).toBe("已放入剪贴板");

    const permission = reduceHudState(initialHudState, {
      phase: "error",
      message: "辅助功能未生效",
      sessionId: 2,
    });
    expect(permission.kind).toBe("permission");
  });
});
```

- [ ] **Step 2: 写菜单栏图标约束测试**

创建 `tests/frontend/brand-assets.test.ts`：

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const tray = readFileSync(
  resolve(process.cwd(), "assets/brand/luozi-tray-template.svg"),
  "utf8",
);

describe("tray template asset", () => {
  it("is a monochrome 16 px breathing bubble with one orbit", () => {
    expect(tray).toContain('viewBox="0 0 16 16"');
    expect(tray).toContain('class="bubble"');
    expect(tray).toContain('class="orbit"');
    expect(tray).not.toContain("<linearGradient");
    expect(tray).not.toMatch(/#[0-9a-f]{3,8}/i);
  });
});
```

- [ ] **Step 3: 运行测试并确认红灯**

Run:

```bash
npm run test:frontend -- tests/frontend/overlay-model.test.ts tests/frontend/brand-assets.test.ts
```

Expected: FAIL，缺少 `src/overlay-model.ts`，且旧 tray SVG 不含 `bubble` / `orbit` class。

- [ ] **Step 4: 创建纯 HUD reducer**

创建 `src/overlay-model.ts`：

```ts
export type HudKind =
  | "idle"
  | "listening"
  | "processing"
  | "success"
  | "clipboard"
  | "permission"
  | "error";

export type HudPhaseEvent = {
  phase: string;
  message: string;
  sessionId: number | null;
};

export type HudState = {
  kind: HudKind;
  title: string;
  detail: string;
  sessionId: number | null;
};

export const initialHudState: HudState = {
  kind: "idle",
  title: "Luozi 已就绪",
  detail: "",
  sessionId: null,
};

export function reduceHudState(current: HudState, event: HudPhaseEvent): HudState {
  if (
    current.sessionId !== null &&
    event.sessionId !== null &&
    event.sessionId < current.sessionId
  ) {
    return current;
  }

  const sessionId = event.sessionId ?? current.sessionId;
  switch (event.phase) {
    case "recording":
    case "recording_edit":
      return {
        kind: "listening",
        title: event.phase === "recording_edit" ? "正在听修改要求" : "正在听你说",
        detail: event.message,
        sessionId,
      };
    case "transcribing":
    case "delivering":
      return {
        kind: "processing",
        title: event.message.includes("修改") ? "正在修改表达" : "正在整理表达",
        detail: "已经停止录音，正在准备写入",
        sessionId,
      };
    case "inserted":
    case "undone":
      return {
        kind: "success",
        title: event.phase === "undone" ? "已撤销" : "已写入当前输入框",
        detail: event.message,
        sessionId,
      };
    case "clipboard":
      return {
        kind: "clipboard",
        title: "已放入剪贴板",
        detail: event.message,
        sessionId,
      };
    case "error":
      return {
        kind: event.message.includes("辅助功能") || event.message.includes("permission")
          ? "permission"
          : "error",
        title:
          event.message.includes("辅助功能") || event.message.includes("permission")
            ? "需要检查系统权限"
            : "这段没有处理成功",
        detail: event.message,
        sessionId,
      };
    case "rejected":
    case "discarded":
    case "too_short":
    case "warn":
      return {
        kind: "error",
        title: event.phase === "too_short" ? "请再按久一点" : "本次没有写入",
        detail: event.message,
        sessionId,
      };
    case "canceled":
      return {
        kind: "idle",
        title: "已取消",
        detail: event.message,
        sessionId,
      };
    default:
      return {
        kind: "processing",
        title: event.message || "正在处理",
        detail: "",
        sessionId,
      };
  }
}
```

- [ ] **Step 5: 创建 HUD 的唯一 SVG 真相源**

创建 `src/assets/luozi-orbit-avatar.svg`：

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 56" role="img" aria-label="Luozi 呼吸泡">
  <defs>
    <linearGradient id="bubble-fill" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity=".96"/>
      <stop offset="1" stop-color="#eef7f5" stop-opacity=".82"/>
    </linearGradient>
  </defs>
  <path class="orbit orbit-back" pathLength="1" d="M7 30c3-15 17-24 32-22 14 2 24 12 26 24"/>
  <path class="orbit orbit-front" pathLength="1" d="M65 32c-3 14-16 22-31 21C20 52 9 44 7 30"/>
  <circle class="bubble" cx="36" cy="29" r="18" fill="url(#bubble-fill)"/>
  <circle class="eye" cx="30" cy="27" r="1.65"/>
  <circle class="eye" cx="42" cy="27" r="1.65"/>
  <path class="expression" d="M31.5 35c2.8 1.8 6.2 1.8 9 0"/>
  <circle class="orbit-node" cx="62.5" cy="19.5" r="2.4"/>
</svg>
```

创建 `src/vite-env.d.ts`：

```ts
/// <reference types="vite/client" />
```

- [ ] **Step 6: 更新 16 px 菜单栏图标真相源**

用以下内容替换 `assets/brand/luozi-tray-template.svg`：

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" role="img" aria-label="Luozi tray template">
  <ellipse class="orbit" cx="8" cy="8" rx="6.25" ry="4.55" fill="none" stroke="black" stroke-width="1.25" stroke-linecap="round" stroke-dasharray="8.2 4.8"/>
  <circle class="bubble" cx="8" cy="8" r="4.15" fill="none" stroke="black" stroke-width="1.2"/>
  <circle cx="6.65" cy="7.55" r=".55" fill="black"/>
  <circle cx="9.35" cy="7.55" r=".55" fill="black"/>
  <path d="M6.7 9.55c.8.55 1.8.55 2.6 0" fill="none" stroke="black" stroke-width=".7" stroke-linecap="round"/>
  <circle cx="13.5" cy="5.7" r=".75" fill="black"/>
</svg>
```

同步公开镜像：

```bash
cp assets/brand/luozi-tray-template.svg public/assets/brand/luozi-tray-template.svg
```

在 macOS 上生成 Tauri Template PNG：

```bash
mkdir -p /private/tmp/luozi-r1-tray
qlmanage -t -s 64 -o /private/tmp/luozi-r1-tray assets/brand/luozi-tray-template.svg
sips -z 16 16 /private/tmp/luozi-r1-tray/luozi-tray-template.svg.png --out src-tauri/icons/tray-template.png
sips -z 32 32 /private/tmp/luozi-r1-tray/luozi-tray-template.svg.png --out src-tauri/icons/tray-template@2x.png
sips -g pixelWidth -g pixelHeight -g hasAlpha src-tauri/icons/tray-template.png src-tauri/icons/tray-template@2x.png
```

Expected:

- `tray-template.png` 为 `16 × 16`；
- `tray-template@2x.png` 为 `32 × 32`；
- 两个文件 `hasAlpha: yes`；
- 图形是黑色 + 透明背景，继续由 `icon_as_template(true)` 交给 macOS 着色。

把 `assets/brand/README.md` 中托盘描述更新为：

```markdown
| `luozi-tray-template.svg` | 菜单栏 16 px 单色「呼吸泡 + 单轨道」矢量真相源 |

运行时菜单栏使用 `src-tauri/icons/tray-template.png` 与
`src-tauri/icons/tray-template@2x.png`。两者必须保持黑色 + 透明背景，
并由 `icon_as_template(true)` 交给 macOS 自动着色。
```

- [ ] **Step 7: 改造 HTML 和 TypeScript**

用以下 body 内容替换 `overlay.html` 当前 `.overlay`：

```html
<div class="overlay" data-state="idle" role="status" aria-live="polite">
  <div class="avatar" id="avatar" aria-hidden="true"></div>
  <div class="copy">
    <strong class="title">Luozi 已就绪</strong>
    <span class="detail"></span>
  </div>
</div>
```

用以下内容替换 `src/overlay.ts`：

```ts
import { listen } from "@tauri-apps/api/event";
import avatarSvg from "./assets/luozi-orbit-avatar.svg?raw";
import { initialHudState, reduceHudState, type HudPhaseEvent } from "./overlay-model";
import "./overlay.css";

const root = document.querySelector<HTMLElement>(".overlay")!;
const avatar = document.querySelector<HTMLElement>("#avatar")!;
const title = document.querySelector<HTMLElement>(".title")!;
const detail = document.querySelector<HTMLElement>(".detail")!;

let state = initialHudState;
avatar.innerHTML = avatarSvg;

function render() {
  root.dataset.state = state.kind;
  title.textContent = state.title;
  detail.textContent = state.detail;
  detail.hidden = state.detail.length === 0;
}

function applyEnergy(level: number) {
  const energy = Math.min(1, Math.max(0, level));
  const svg = avatar.querySelector<SVGElement>("svg");
  svg?.style.setProperty("--energy", energy.toFixed(3));
  svg?.style.setProperty("--orbit-dash", `${(0.18 + energy * 0.42).toFixed(3)} 1`);
  svg?.style.setProperty("--orbit-opacity", (0.28 + energy * 0.72).toFixed(3));
  svg?.style.setProperty("--orbit-back-opacity", (0.18 + energy * 0.46).toFixed(3));
}

void listen<HudPhaseEvent>("session://phase", (event) => {
  state = reduceHudState(state, event.payload);
  render();
});

void listen<{ sessionId: number; level: number }>("session://energy", (event) => {
  if (state.sessionId !== null && event.payload.sessionId !== state.sessionId) return;
  if (state.kind !== "listening") return;
  applyEnergy(event.payload.level);
});

render();
```

前端不再调用 `getCurrentWindow().hide()`，由 Rust controller 统一控制 HUD 生命周期。

- [ ] **Step 8: 用珍珠 HUD 和轨道状态替换旧 CSS**

`src/overlay.css` 保留 token import 和页面透明基础，其余组件样式替换为：

```css
html,
body {
  margin: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent !important;
  user-select: none;
  -webkit-user-select: none;
}

.overlay {
  display: flex;
  align-items: center;
  gap: 12px;
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  padding: 8px 18px 8px 12px;
  border: 1px solid color-mix(in srgb, var(--text-ink) 10%, transparent);
  border-radius: var(--radius-hud);
  background: color-mix(in srgb, var(--surface-pearl) 88%, transparent);
  color: var(--text-ink);
  box-shadow: var(--shadow-float);
  backdrop-filter: blur(22px) saturate(1.15);
  -webkit-backdrop-filter: blur(22px) saturate(1.15);
}

.avatar {
  width: 62px;
  height: 48px;
  flex: 0 0 auto;
}

.avatar svg {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
}

.bubble {
  stroke: color-mix(in srgb, var(--text-ink) 14%, transparent);
  stroke-width: 1;
  filter: drop-shadow(0 4px 8px color-mix(in srgb, #173a3a 14%, transparent));
}

.eye {
  fill: var(--text-ink);
}

.expression {
  fill: none;
  stroke: var(--text-muted);
  stroke-width: 1.5;
  stroke-linecap: round;
}

.orbit {
  fill: none;
  stroke: var(--brand-mint);
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-dasharray: var(--orbit-dash, 0.18 1);
  opacity: var(--orbit-opacity, 0.28);
  transition:
    opacity 80ms linear,
    stroke-dasharray 80ms linear,
    stroke var(--motion-fast) ease;
}

.orbit-back {
  opacity: var(--orbit-back-opacity, 0.18);
}

.orbit-node {
  fill: var(--brand-mint);
  transition: fill var(--motion-fast) ease;
}

.copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 2px;
}

.title {
  overflow: hidden;
  color: var(--text-ink);
  font-size: 14px;
  font-weight: 650;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail {
  overflow: hidden;
  color: var(--text-muted);
  font-size: 11.5px;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overlay[data-state="processing"] .orbit {
  stroke: var(--brand-violet);
  animation: phase-shift var(--motion-state) ease-out 1;
}

.overlay[data-state="processing"] .orbit-node {
  fill: var(--brand-violet);
}

.overlay[data-state="success"] .orbit,
.overlay[data-state="clipboard"] .orbit {
  stroke: var(--state-success);
  stroke-dasharray: 0.82 1;
}

.overlay[data-state="success"] .orbit-node,
.overlay[data-state="clipboard"] .orbit-node {
  fill: var(--state-success);
}

.overlay[data-state="error"] .orbit,
.overlay[data-state="permission"] .orbit {
  opacity: 0.18;
  stroke: var(--text-muted);
}

.overlay[data-state="error"] .orbit-node,
.overlay[data-state="permission"] .orbit-node {
  fill: var(--state-error);
}

@keyframes phase-shift {
  from {
    stroke-dashoffset: 0;
  }
  to {
    stroke-dashoffset: -0.12;
  }
}

@media (prefers-reduced-motion: reduce) {
  .orbit {
    animation: none !important;
    transition: none;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .overlay {
    background: var(--surface-solid);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}
```

- [ ] **Step 9: 调整 HUD 窗口尺寸**

在 `src-tauri/tauri.conf.json` 的 `overlay` 窗口中设置：

```json
"width": 420,
"height": 64
```

其他 overlay 属性保持不变。

- [ ] **Step 10: 运行前端测试和构建**

Run:

```bash
npm run test:frontend -- tests/frontend/overlay-model.test.ts tests/frontend/brand-assets.test.ts
npm run build
```

Expected: 4 tests PASS；Vite 能解析 `?raw` SVG；build 成功。

- [ ] **Step 11: 提交**

```bash
git add src/assets/luozi-orbit-avatar.svg src/vite-env.d.ts src/overlay-model.ts tests/frontend/overlay-model.test.ts tests/frontend/brand-assets.test.ts assets/brand/luozi-tray-template.svg public/assets/brand/luozi-tray-template.svg src-tauri/icons/tray-template.png src-tauri/icons/tray-template@2x.png assets/brand/README.md overlay.html src/overlay.ts src/overlay.css src-tauri/tauri.conf.json
git commit -m "feat: redesign HUD with orbit voice avatar"
```

---

### Task 6: 重排菜单栏入口并降低工作台权重

**Files:**

- Create: `tests/frontend/tray-ia.test.ts`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写菜单信息架构测试**

创建 `tests/frontend/tray-ia.test.ts`：

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = readFileSync(resolve(process.cwd(), "src-tauri/src/lib.rs"), "utf8");

describe("tray information architecture", () => {
  it("places dictation before secondary windows and settings", () => {
    const menuStart = source.indexOf("let menu = Menu::with_items");
    const menuEnd = source.indexOf("let mut tray = TrayIconBuilder", menuStart);
    const menu = source.slice(menuStart, menuEnd);
    expect(menu.indexOf("&start")).toBeLessThan(menu.indexOf("&draft"));
    expect(menu.indexOf("&draft")).toBeLessThan(menu.indexOf("&settings"));
  });

  it("uses product language instead of milestone language", () => {
    expect(source).toContain('"Luozi 已就绪"');
    expect(source).not.toContain("本地+云端（M5）");
  });
});
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```bash
npm run test:frontend -- tests/frontend/tray-ia.test.ts
```

Expected: 第二个测试 FAIL，因为当前菜单仍包含里程碑文案。

- [ ] **Step 3: 改写菜单文案和顺序**

在 `build_tray` 中使用以下文案：

```rust
let title = MenuItem::with_id(app, "title", "Luozi 已就绪", false, None::<&str>)?;
let status = MenuItem::with_id(app, "status", status_label, false, None::<&str>)?;
let start = MenuItem::with_id(
    app,
    "start",
    "开始语音输入",
    true,
    Some(cfg.continue_speaking_shortcut.as_str()),
)?;
let draft = MenuItem::with_id(app, "draft", "打开语音草稿…", true, None::<&str>)?;
let voice_edit = MenuItem::with_id(
    app,
    "voice_edit",
    "修改当前草稿",
    true,
    Some(cfg.voice_edit_shortcut.as_str()),
)?;
let undo = MenuItem::with_id(app, "undo", "撤销上次落字", true, None::<&str>)?;
let cancel = MenuItem::with_id(app, "cancel", "取消当前会话", true, Some("Escape"))?;
let engine = MenuItem::with_id(
    app,
    "engine",
    &format!("转写：{}", cfg.asr_mode.label_zh()),
    false,
    None::<&str>,
)?;
let fetch_model = MenuItem::with_id(
    app,
    "fetch_model",
    "下载本地模型…",
    !model_ready,
    None::<&str>,
)?;
let settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
let about = MenuItem::with_id(app, "about", "关于 Luozi", true, None::<&str>)?;
let quit = MenuItem::with_id(app, "quit", "退出 Luozi", true, None::<&str>)?;
```

删除顶层 `asr_mode` 菜单项，切换引擎统一进入设置。菜单按以下顺序构建：

```rust
let menu = Menu::with_items(
    app,
    &[
        &title,
        &status,
        &sep_status,
        &start,
        &cancel,
        &undo,
        &sep_actions,
        &draft,
        &voice_edit,
        &sep_prefs,
        &engine,
        &fetch_model,
        &settings,
        &about,
        &quit,
    ],
)?;
```

删除 `"asr_mode"` 对应的 `on_menu_event` 分支；底层 `cycle_asr_mode` API 保留，避免扩大改动。

- [ ] **Step 4: 运行测试和 Rust check**

Run:

```bash
npm run test:frontend -- tests/frontend/tray-ia.test.ts
cargo check -p luozi
```

Expected: 2 tests PASS；Rust check 成功。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/lib.rs tests/frontend/tray-ia.test.ts
git commit -m "refactor: make dictation primary in tray menu"
```

---

### Task 7: 重构设置导航和普通 / 高级分层

**Files:**

- Create: `tests/frontend/settings-ia.test.ts`
- Modify: `settings.html`
- Modify: `src/settings.ts`
- Modify: `src/styles-settings.css`

- [ ] **Step 1: 写设置 IA 测试**

创建 `tests/frontend/settings-ia.test.ts`：

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const html = readFileSync(resolve(process.cwd(), "settings.html"), "utf8");

describe("settings information architecture", () => {
  it("uses the approved six navigation groups", () => {
    const sections = [...html.matchAll(/class="nav-item[^"]*" data-section="([^"]+)"/g)].map(
      (match) => match[1],
    );
    expect(sections).toEqual(["general", "voice", "hotkeys", "privacy", "advanced", "about"]);
  });

  it("keeps provider and key controls inside advanced settings", () => {
    const advancedStart = html.indexOf('id="section-advanced"');
    const advancedEnd = html.indexOf('id="section-about"', advancedStart);
    const advanced = html.slice(advancedStart, advancedEnd);
    expect(advanced).toContain('id="btnPickAsrProvider"');
    expect(advanced).toContain('id="btnAsrKey"');
    expect(advanced).toContain('id="btnPickTextProvider"');
    expect(advanced).toContain('id="btnTextAiKey"');
  });
});
```

- [ ] **Step 2: 运行测试并确认红灯**

Run:

```bash
npm run test:frontend -- tests/frontend/settings-ia.test.ts
```

Expected: FAIL，当前只有 5 个导航分组，且 provider / Key 仍在语音页。

- [ ] **Step 3: 替换设置导航**

将 `settings.html` 的 nav 替换为：

```html
<nav class="settings-nav" aria-label="设置分区">
  <button type="button" class="nav-item active" data-section="general">通用</button>
  <button type="button" class="nav-item" data-section="voice">语音与写作</button>
  <button type="button" class="nav-item" data-section="hotkeys">快捷键</button>
  <button type="button" class="nav-item" data-section="privacy">隐私与权限</button>
  <button type="button" class="nav-item" data-section="advanced">高级</button>
  <button type="button" class="nav-item nav-about" data-section="about">关于</button>
</nav>
```

- [ ] **Step 4: 重写通用页和语音页**

通用页只保留已存在且真实的状态：

```html
<section id="section-general" class="settings-section">
  <div class="section-heading">
    <h1>通用</h1>
    <p>控制 Luozi 的常驻方式与本地草稿行为。</p>
  </div>
  <div class="settings-group">
    <div class="row">
      <div>
        <strong>关闭窗口后继续运行</strong>
        <p class="muted">草稿和设置窗口关闭后，Luozi 继续驻留菜单栏。</p>
      </div>
      <span class="badge ok">已开启</span>
    </div>
    <div class="row">
      <div>
        <strong>当前草稿自动恢复</strong>
        <p class="muted">关闭语音草稿时保存到本机，下次打开继续编辑。</p>
      </div>
      <span class="badge ok">已开启</span>
    </div>
    <div class="row">
      <div>
        <strong>界面语言</strong>
        <p class="muted">当前版本使用简体中文。</p>
      </div>
      <span class="badge">简体中文</span>
    </div>
  </div>
</section>
```

语音与写作页只放用户能理解的模式、本地模型和文本整理摘要：

```html
<section id="section-voice" class="settings-section" hidden>
  <div class="section-heading">
    <h1>语音与写作</h1>
    <p>选择日常转写方式；提供方和密钥统一放在「高级」。</p>
  </div>
  <div class="settings-group">
    <div class="row">
      <div>
        <strong>转写方式</strong>
        <p class="muted" id="asrModeHint">自动（本地优先）</p>
      </div>
      <button type="button" id="btnPickAsrMode">选择方式…</button>
    </div>
    <div class="row">
      <div>
        <strong>本地语音模型</strong>
        <p class="muted" id="modelStatus">—</p>
      </div>
      <span class="badge" id="modelBadge">—</span>
    </div>
    <div class="row">
      <div>
        <strong>文本整理</strong>
        <p class="muted" id="textAiSummary">基础转写可直接使用；AI 修改需在高级设置中授权。</p>
      </div>
      <span class="badge" id="textAiSummaryBadge">基础</span>
    </div>
  </div>
</section>
```

- [ ] **Step 5: 创建隐私与权限、高级 section**

用以下内容替换原 `section-permissions`：

```html
<section id="section-privacy" class="settings-section" hidden>
  <div class="section-heading">
    <h1>隐私与权限</h1>
    <p>权限只在对应功能发生时使用；辅助功能失败时仍按既有规则降级到剪贴板。</p>
  </div>
  <div class="settings-group">
    <div class="row">
      <div>
        <strong>麦克风</strong>
        <p class="muted" id="micStatus">录音需要麦克风权限</p>
      </div>
      <div class="btn-group">
        <span class="badge" id="micBadge">—</span>
        <button type="button" id="btnOpenMic">打开系统设置</button>
      </div>
    </div>
    <div class="row">
      <div>
        <strong>辅助功能</strong>
        <p class="muted" id="axStatus">用于向其他 App 插入文字；失败则降级剪贴板</p>
      </div>
      <div class="btn-group">
        <span class="badge" id="axBadge">—</span>
        <button type="button" id="btnOpenAx">打开系统设置</button>
      </div>
    </div>
  </div>
  <p class="note" id="axAdhocNote" hidden>
    ad-hoc 重装后签名变化，可能需要在辅助功能里关掉再打开「Luozi」。
  </p>
</section>
```

把现有云端 ASR 和文本 AI provider / Key 行原样移动到以下结构：

```html
<section id="section-advanced" class="settings-section" hidden>
  <div class="section-heading">
    <h1>高级</h1>
    <p>提供方、模型、密钥与分项上传授权。密钥仍只保存在系统钥匙串。</p>
  </div>
  <h2>云端语音转写</h2>
  <div class="settings-group">
    <div class="row">
      <div>
        <strong>提供方</strong>
        <p class="muted" id="cloudAsrStatus">—</p>
      </div>
      <button type="button" id="btnPickAsrProvider">选择提供方…</button>
    </div>
    <div class="row">
      <div>
        <strong>密钥与上传授权</strong>
        <p class="muted">Key 只进入本机钥匙串；上传音频前仍需单独同意。</p>
      </div>
      <div class="btn-group wrap">
        <button type="button" id="btnAsrKey">配置 Key…</button>
        <button type="button" id="btnAsrConsent">同意上传…</button>
      </div>
    </div>
  </div>
  <h2>文本 AI</h2>
  <div class="settings-group">
    <div class="row">
      <div>
        <strong>提供方与模型</strong>
        <p class="muted" id="textAiStatus">—</p>
      </div>
      <div class="btn-group wrap">
        <button type="button" id="btnPickTextProvider">选择提供方…</button>
        <button type="button" id="btnTextAiModel">模型…</button>
      </div>
    </div>
    <div class="row">
      <div>
        <strong>密钥与上传授权</strong>
        <p class="muted">文本 AI 与语音转写分开授权。</p>
      </div>
      <div class="btn-group wrap">
        <button type="button" id="btnTextAiKey">配置 Key…</button>
        <button type="button" id="btnTextAiConsent">同意上传…</button>
      </div>
    </div>
  </div>
</section>
```

- [ ] **Step 6: 更新 TypeScript section id 与摘要**

把 `showSection` 中的权限判断改为：

```ts
const isPrivacy = id === "privacy";
syncPermissionsPoll(isPrivacy);
if (isPrivacy) void refresh();
```

在 `applySnapshot` 的文本 AI 状态更新后增加：

```ts
const textAiSummary = document.querySelector("#textAiSummary");
if (textAiSummary) {
  textAiSummary.textContent = s.textAiReady
    ? `AI 修改已就绪 · ${s.textAiProviderLabel}`
    : "基础转写可直接使用；AI 修改需在高级设置中授权。";
}
const textAiSummaryBadge = document.querySelector("#textAiSummaryBadge");
if (textAiSummaryBadge) {
  textAiSummaryBadge.textContent = s.textAiReady ? "AI 已就绪" : "基础";
  textAiSummaryBadge.className = `badge ${s.textAiReady ? "ok" : ""}`;
}
```

其他 invoke、provider picker、Key 和 consent 事件处理保持现有实现。

- [ ] **Step 7: 收紧设置视觉密度**

在 `src/styles-settings.css` 中使用以下核心布局值：

```css
.settings-shell {
  display: grid;
  grid-template-columns: 156px minmax(0, 1fr);
  min-height: 100vh;
  background: var(--surface-mist);
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 14px 10px;
  border-right: 1px solid var(--line-soft);
  background: color-mix(in srgb, var(--surface-pearl) 92%, transparent);
}

.nav-item {
  min-height: 34px;
  padding: 7px 10px;
  border: 0;
  border-radius: var(--radius-control);
  background: transparent;
  color: var(--text-ink);
  text-align: left;
  font-size: 13px;
}

.nav-item.active {
  background: color-mix(in srgb, var(--brand-mint) 17%, var(--surface-pearl));
  font-weight: 650;
}

.nav-about {
  margin-top: auto;
}

.settings-main {
  padding: 28px 32px 40px;
  overflow: auto;
}

.section-heading {
  margin-bottom: 20px;
}

.section-heading h1 {
  margin: 0 0 4px;
  font-size: 22px;
}

.section-heading p {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
}

.settings-group {
  overflow: hidden;
  margin-bottom: 20px;
  border: 1px solid var(--line-soft);
  border-radius: var(--radius-panel);
  background: var(--surface-pearl);
}

.settings-group .row {
  min-height: 58px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--line-soft);
}

.settings-group .row:last-child {
  border-bottom: 0;
}
```

删除整窗薄荷径向渐变；保留 dialog、help 和 badge 现有功能样式，但把颜色来源改为统一 token。

- [ ] **Step 8: 运行设置 IA 测试和构建**

Run:

```bash
npm run test:frontend -- tests/frontend/settings-ia.test.ts
npm run build
```

Expected: 2 tests PASS；所有已有 DOM id 仍能通过 TypeScript build。

- [ ] **Step 9: 提交**

```bash
git add settings.html src/settings.ts src/styles-settings.css tests/frontend/settings-ia.test.ts
git commit -m "refactor: simplify settings information architecture"
```

---

### Task 8: 完成一轮聚焦回归和 Mac 实机验收

**Files:**

- Create: `docs/spikes/r1-experience-manual.md`

- [ ] **Step 1: 创建手工验收清单**

创建 `docs/spikes/r1-experience-manual.md`：

```markdown
# R1 体验改造手工验收

## 前置

- 安装位置：`/Applications/Luozi.app`
- 默认主快捷键：以设置页「实际注册」为准
- 测试背景：白色编辑器、深色 IDE、复杂网页

## HUD

- [ ] 按住主快捷键：64 px 高珍珠 HUD 出现，不抢焦点
- [ ] 正常说话：轨道亮度和可见长度随声音变化，角色和五官不缩放
- [ ] 静音：轨道回落，不持续整圈旋转
- [ ] 松开：进入紫色处理态，只播放一次相位动作
- [ ] 成功插入：绿色闭合反馈约 600 ms 后消失
- [ ] 剪贴板降级：明确显示剪贴板结果，停留时间可读
- [ ] 错误：只亮珊瑚节点，不闪烁、不抖动
- [ ] 开启减少动态效果：无位移和旋转动画
- [ ] 开启降低透明度：HUD 使用不透明面板，文字仍清楚

## 产品入口

- [ ] 正常启动不弹草稿或设置
- [ ] 菜单第一主动作是「开始语音输入」
- [ ] 草稿作为二级入口按需打开
- [ ] 托盘不再显示 M5 / M8 等开发阶段文案

## 设置

- [ ] 导航依次为通用、语音与写作、快捷键、隐私与权限、高级、关于
- [ ] 普通语音页不出现 provider、Key、上传同意
- [ ] 高级页仍能配置 ASR / 文本 AI provider、Key 和分项授权
- [ ] 离开隐私页后权限轮询停止

## 安全回归

- [ ] Safari、Chrome、备忘录、微信输入框和终端各完成 3 次落字
- [ ] 录音中切换输入目标：不得写入新焦点，按既有规则降级
- [ ] 密码框：拒绝录音或丢弃结果
- [ ] 辅助功能关闭：保留剪贴板结果并明确提示
- [ ] 连续两次会话：旧 session 的晚到事件不覆盖新 HUD
```

- [ ] **Step 2: 运行唯一一轮自动化全量检查**

Run:

```bash
npm run test:frontend
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: 全部成功；不重复运行无新证据的检查。

- [ ] **Step 3: 安装 Mac App 并完成实机清单**

Run:

```bash
npm run install:macos-app
open /Applications/Luozi.app
```

逐项执行 `docs/spikes/r1-experience-manual.md`。失败项记录可复现步骤、实际 HUD 状态和对应日志，不以视觉主观判断替代安全回归。

- [ ] **Step 4: 最终差异审查**

Run:

```bash
git diff --check
git status --short
git log --oneline --decorate -8
```

确认：

- 无模型、Key、构建产物或 `.superpowers/` 被跟踪；
- 没有修改 ASR 路由、文本 AI 请求、钥匙串或目标校验语义；
- R2 / R3 文件和功能没有提前进入本分支。

- [ ] **Step 5: 提交验收文档**

```bash
git add docs/spikes/r1-experience-manual.md
git commit -m "docs: add R1 experience acceptance checklist"
```

## 完成定义

R1 只有同时满足以下条件才算完成：

1. 四类 HUD 状态、真实声音能量和 stale session 防护通过自动测试；
2. 菜单与设置 IA 测试通过，普通设置不再暴露技术控制台结构；
3. Rust / TypeScript build、clippy 和 workspace tests 一轮通过；
4. Mac 实机完成原位落字、目标切换、权限降级和密码框回归；
5. 现有草稿、ASR、文本 AI、Keychain 和安全交付语义未改变；
6. 实施提交保持小而清晰，可逐个回退。
