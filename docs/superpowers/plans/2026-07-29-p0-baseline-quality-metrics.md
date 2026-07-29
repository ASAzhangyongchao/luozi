# Luozi P0 Baseline and Quality Metrics Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 固化可恢复代码基线，并建立后续所有体验改造共用的前端测试、阶段计时、质量指标和安全基准语料。

**Architecture:** 纯计算指标进入 `luozi-core`，不接触录音内容和 Key；Tauri 只记录会话阶段耗时并发送无正文 DTO；基准语料只保存公开中性文本和分类，真实录音保存在 Git 忽略目录。

**Tech Stack:** Rust 2021、Tauri 2、TypeScript、Vite 6、Vitest、Serde JSON。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §6、§12、§13

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `package.json` / `package-lock.json` | Modify | 增加统一前端测试命令 |
| `tests/frontend/test-harness.test.ts` | Create | 验证 Vitest 与仓库路径 |
| `crates/luozi-core/src/metrics.rs` | Create | 阶段耗时、百分位和质量分数纯函数 |
| `crates/luozi-core/src/lib.rs` | Modify | 导出指标类型 |
| `src-tauri/src/session/telemetry.rs` | Create | 无正文的单会话阶段计时 |
| `src-tauri/src/session/mod.rs` | Modify | 注册 telemetry 模块 |
| `src-tauri/src/session/controller.rs` | Modify | 在现有会话边界写入阶段时间 |
| `tests/benchmarks/corpus.json` | Create | 20 条公开提示词 × 5 次录制条件 |
| `tests/benchmarks/README.md` | Create | 私有录音与报告运行方法 |
| `.gitignore` | Modify | 忽略真实基准音频和结果 JSONL |

---

### Task 1: 固化干净基线和前端测试入口

**Files:**

- Modify: `package.json`
- Modify: `package-lock.json`
- Create: `tests/frontend/test-harness.test.ts`

- [ ] **Step 1: 确认执行基线**

Run:

```bash
git status --short
git diff --check
```

Expected：工作树为空；如果不为空，停止，不执行后续步骤。

- [ ] **Step 2: 安装并登记 Vitest**

Run:

```bash
npm install --save-dev vitest@3.2.4
```

将 `package.json` scripts 增加：

```json
{
  "test:frontend": "vitest run"
}
```

- [ ] **Step 3: 写测试入口**

创建 `tests/frontend/test-harness.test.ts`：

```ts
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("Luozi frontend test harness", () => {
  it("runs from the repository root", () => {
    const pkg = JSON.parse(readFileSync(resolve("package.json"), "utf8")) as {
      name: string;
    };
    expect(pkg.name).toBe("luozi");
  });
});
```

- [ ] **Step 4: 运行测试**

Run:

```bash
npm run test:frontend -- tests/frontend/test-harness.test.ts
```

Expected：1 test PASS。

- [ ] **Step 5: 提交**

```bash
git add package.json package-lock.json tests/frontend/test-harness.test.ts
git commit -m "test: add frontend test harness"
```

---

### Task 2: 建立纯 Rust 阶段计时与百分位模型

**Files:**

- Create: `crates/luozi-core/src/metrics.rs`
- Modify: `crates/luozi-core/src/lib.rs`

- [ ] **Step 1: 写失败测试**

创建 `crates/luozi-core/src/metrics.rs`：

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricPhase {
    HudVisible,
    RecordingReady,
    AsrFinished,
    CleanupFinished,
    DeliveryFinished,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_each_phase_once_and_keeps_monotonic_values() {
        let mut timings = PhaseTimings::default();
        assert!(timings.record(MetricPhase::HudVisible, 80).is_ok());
        assert!(timings.record(MetricPhase::AsrFinished, 900).is_ok());
        assert_eq!(timings.get(MetricPhase::HudVisible), Some(80));
        assert!(timings.record(MetricPhase::HudVisible, 90).is_err());
        assert!(timings.record(MetricPhase::CleanupFinished, 700).is_err());
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        assert_eq!(nearest_rank(&[100, 200, 300, 400], 50), Some(200));
        assert_eq!(nearest_rank(&[100, 200, 300, 400], 95), Some(400));
        assert_eq!(nearest_rank(&[], 50), None);
    }
}
```

在 `crates/luozi-core/src/lib.rs` 加入：

```rust
mod metrics;
pub use metrics::{nearest_rank, MetricPhase, PhaseTimings};
```

- [ ] **Step 2: 确认测试红灯**

Run:

```bash
cargo test -p luozi-core metrics
```

Expected：FAIL，缺少 `PhaseTimings` 和 `nearest_rank`。

- [ ] **Step 3: 写最小实现**

在测试前加入：

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTimings {
    values: Vec<(MetricPhase, u64)>,
}

impl PhaseTimings {
    pub fn record(&mut self, phase: MetricPhase, elapsed_ms: u64) -> Result<(), &'static str> {
        if self.values.iter().any(|(p, _)| *p == phase) {
            return Err("phase_already_recorded");
        }
        if self.values.last().is_some_and(|(_, last)| elapsed_ms < *last) {
            return Err("phase_not_monotonic");
        }
        self.values.push((phase, elapsed_ms));
        Ok(())
    }

    pub fn get(&self, phase: MetricPhase) -> Option<u64> {
        self.values
            .iter()
            .find_map(|(p, value)| (*p == phase).then_some(*value))
    }
}

pub fn nearest_rank(values: &[u64], percentile: u8) -> Option<u64> {
    if values.is_empty() || percentile == 0 || percentile > 100 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((percentile as usize * sorted.len()) + 99) / 100;
    sorted.get(rank.saturating_sub(1)).copied()
}
```

- [ ] **Step 4: 确认测试绿灯**

Run:

```bash
cargo test -p luozi-core metrics
```

Expected：2 tests PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/metrics.rs crates/luozi-core/src/lib.rs
git commit -m "feat: add session timing metrics"
```

---

### Task 3: 增加文字质量指标

**Files:**

- Modify: `crates/luozi-core/src/metrics.rs`

- [ ] **Step 1: 写字符错误率测试**

在 `metrics.rs` tests 中加入：

```rust
#[test]
fn character_error_rate_counts_insert_delete_replace() {
    assert_eq!(edit_distance("落字", "落字"), 0);
    assert_eq!(edit_distance("落字", "落子"), 1);
    assert!((character_error_rate("今天三点", "今天四点") - 0.25).abs() < f32::EPSILON);
}

#[test]
fn protected_token_score_requires_exact_tokens() {
    let expected = ["2026-07-29", "ASFF", "12800"];
    assert_eq!(protected_token_errors(&expected, "日期 2026-07-29，ASFF 金额 12800"), 0);
    assert_eq!(protected_token_errors(&expected, "日期 2026-07-28，金额 12800"), 2);
}
```

- [ ] **Step 2: 确认测试红灯**

Run:

```bash
cargo test -p luozi-core metrics
```

Expected：FAIL，缺少三个质量函数。

- [ ] **Step 3: 写最小质量函数**

在 `metrics.rs` 加入：

```rust
pub fn edit_distance(expected: &str, actual: &str) -> usize {
    let a: Vec<char> = expected.chars().collect();
    let b: Vec<char> = actual.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut next = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let replace = prev[j] + usize::from(ca != cb);
            next[j + 1] = (prev[j + 1] + 1).min(next[j] + 1).min(replace);
        }
        prev = next;
    }
    prev[b.len()]
}

pub fn character_error_rate(expected: &str, actual: &str) -> f32 {
    let count = expected.chars().count().max(1);
    edit_distance(expected, actual) as f32 / count as f32
}

pub fn protected_token_errors(expected: &[&str], actual: &str) -> usize {
    expected.iter().filter(|token| !actual.contains(**token)).count()
}
```

并从 `lib.rs` 导出：

```rust
pub use metrics::{
    character_error_rate, edit_distance, nearest_rank, protected_token_errors, MetricPhase,
    PhaseTimings,
};
```

- [ ] **Step 4: 运行测试**

Run:

```bash
cargo test -p luozi-core metrics
```

Expected：全部 metrics tests PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/luozi-core/src/metrics.rs crates/luozi-core/src/lib.rs
git commit -m "feat: add transcription quality metrics"
```

---

### Task 4: 把无正文计时接入会话链路

**Files:**

- Create: `src-tauri/src/session/telemetry.rs`
- Modify: `src-tauri/src/session/mod.rs`
- Modify: `src-tauri/src/session/controller.rs`

- [ ] **Step 1: 写 telemetry 状态测试**

创建 `src-tauri/src/session/telemetry.rs`：

```rust
use std::collections::HashMap;
use std::time::Instant;

use luozi_core::{MetricPhase, PhaseTimings};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetricsDto {
    pub session_id: u64,
    pub timings: PhaseTimings,
}

#[derive(Default)]
pub struct SessionTelemetry {
    active: HashMap<u64, (Instant, PhaseTimings)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_removes_session_and_never_stores_text() {
        let mut state = SessionTelemetry::default();
        state.begin(7);
        state.mark(7, MetricPhase::HudVisible);
        let dto = state.finish(7).expect("metrics");
        assert_eq!(dto.session_id, 7);
        assert!(state.finish(7).is_none());
    }
}
```

- [ ] **Step 2: 确认测试红灯**

Run:

```bash
cargo test -p luozi finish_removes_session_and_never_stores_text
```

Expected：FAIL，缺少 `begin`、`mark`、`finish`。

- [ ] **Step 3: 写最小实现**

在 `SessionTelemetry` 后加入：

```rust
impl SessionTelemetry {
    pub fn begin(&mut self, session_id: u64) {
        self.active
            .insert(session_id, (Instant::now(), PhaseTimings::default()));
    }

    pub fn mark(&mut self, session_id: u64, phase: MetricPhase) {
        if let Some((started, timings)) = self.active.get_mut(&session_id) {
            let _ = timings.record(phase, started.elapsed().as_millis() as u64);
        }
    }

    pub fn finish(&mut self, session_id: u64) -> Option<SessionMetricsDto> {
        let (_, timings) = self.active.remove(&session_id)?;
        Some(SessionMetricsDto {
            session_id,
            timings,
        })
    }
}
```

在 `session/mod.rs` 增加：

```rust
pub mod telemetry;
```

在 `AppSessionState` 增加：

```rust
pub telemetry: Mutex<super::telemetry::SessionTelemetry>,
```

并在默认构造中初始化：

```rust
telemetry: Mutex::new(super::telemetry::SessionTelemetry::default()),
```

在开始会话、显示 HUD、ASR 完成、整理完成和交付完成的现有边界分别调用 `begin` / `mark`；交付结束后：

```rust
if let Some(metrics) = state
    .telemetry
    .lock()
    .ok()
    .and_then(|mut value| value.finish(session_id))
{
    let _ = app.emit("session://metrics", metrics);
}
```

- [ ] **Step 4: 运行聚焦测试与会话测试**

Run:

```bash
cargo test -p luozi finish_removes_session_and_never_stores_text
cargo test -p luozi-core session
```

Expected：全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/session/telemetry.rs src-tauri/src/session/mod.rs src-tauri/src/session/controller.rs
git commit -m "feat: emit content-free session timings"
```

---

### Task 5: 建立公开基准语料和私有录音边界

**Files:**

- Create: `tests/benchmarks/corpus.json`
- Create: `tests/benchmarks/README.md`
- Modify: `.gitignore`

- [ ] **Step 1: 创建语料清单**

创建 `tests/benchmarks/corpus.json`：

```json
{
  "schemaVersion": 1,
  "recordingsPerPrompt": 5,
  "prompts": [
    {"id":"zh-message-01","category":"zh-message","expected":"今天下午三点开会，请提前十分钟提醒我。","protected":["三点","十分钟"]},
    {"id":"zh-message-02","category":"zh-message","expected":"这个问题我明天确认以后再回复你。","protected":[]},
    {"id":"self-correct-01","category":"self-correction","expected":"我们下午三点出发。","spoken":"我们下午两点，不对，三点出发。","protected":["三点"]},
    {"id":"self-correct-02","category":"self-correction","expected":"请发给王晓明。","spoken":"请发给王小明，不对，是晓得的晓。","protected":["王晓明"]},
    {"id":"mixed-01","category":"mixed-language","expected":"请检查 ASFF 的 API 返回结果。","protected":["ASFF","API"]},
    {"id":"mixed-02","category":"mixed-language","expected":"把 feature branch 合并到 main。","protected":["feature branch","main"]},
    {"id":"number-01","category":"number","expected":"订单金额是 12800 元。","protected":["12800"]},
    {"id":"number-02","category":"number","expected":"发布日期是 2026-07-29。","protected":["2026-07-29"]},
    {"id":"number-03","category":"number","expected":"成功率从 98.5% 提高到 99.2%。","protected":["98.5%","99.2%"]},
    {"id":"url-01","category":"url","expected":"网址是 github.com/ASAzhangyongchao/luozi。","protected":["github.com/ASAzhangyongchao/luozi"]},
    {"id":"email-01","category":"email","expected":"请发送到 luozi@example.com。","protected":["luozi@example.com"]},
    {"id":"list-01","category":"list","expected":"今天做三件事：第一，检查接口；第二，修复问题；第三，发布版本。","protected":[]},
    {"id":"paragraph-01","category":"paragraph","expected":"先完成语音识别。然后检查文字质量。最后再写入输入框。","protected":[]},
    {"id":"product-01","category":"proper-noun","expected":"Luozi 使用 Tauri 和 Whisper。","protected":["Luozi","Tauri","Whisper"]},
    {"id":"product-02","category":"proper-noun","expected":"请更新 IAM-Center 的配置。","protected":["IAM-Center"]},
    {"id":"quiet-01","category":"quiet-speech","expected":"这是一段较小音量的测试。","protected":[]},
    {"id":"noise-01","category":"light-noise","expected":"背景有轻微噪音，但内容应该保持完整。","protected":[]},
    {"id":"punctuation-01","category":"punctuation","expected":"你确认了吗？如果确认，请直接回复。","protected":[]},
    {"id":"draft-01","category":"draft","expected":"把这一段改得更简洁，但不要改变数字 42。","protected":["42"]},
    {"id":"code-01","category":"developer","expected":"变量名使用 camelCase，文件名是 settings.ts。","protected":["camelCase","settings.ts"]}
  ]
}
```

- [ ] **Step 2: 写录音与隐私说明**

创建 `tests/benchmarks/README.md`，明确：

```markdown
# Luozi Benchmark Corpus

`corpus.json` 包含 20 条公开中性提示词，每条录制 5 次，共 100 个样本。

真实 WAV 保存到 `tests/benchmarks/private-audio/<prompt-id>/<take>.wav`，不得提交。
五次条件固定为：正常音量、较小音量、较快语速、较慢语速、轻度环境噪音。

每次运行保存无音频、无 Key、无全文日志的结果到
`tests/benchmarks/private-results/<date>-<backend>.jsonl`。
公开报告只允许包含聚合 CER、受保护词错误数、P50/P95 和资源指标。
```

- [ ] **Step 3: 忽略私有数据**

在 `.gitignore` 加入：

```gitignore
tests/benchmarks/private-audio/
tests/benchmarks/private-results/
```

- [ ] **Step 4: 验证语料规模**

Run:

```bash
node -e "const c=require('./tests/benchmarks/corpus.json'); if(c.prompts.length*c.recordingsPerPrompt!==100) process.exit(1); console.log('samples=100')"
git check-ignore tests/benchmarks/private-audio/sample.wav
```

Expected：输出 `samples=100`，第二条输出被忽略的路径。

- [ ] **Step 5: 提交**

```bash
git add .gitignore tests/benchmarks/corpus.json tests/benchmarks/README.md
git commit -m "test: add private-safe dictation benchmark corpus"
```

---

### Task 6: P0 全量验证

**Files:**

- No file changes.

- [ ] **Step 1: 运行全部自动检查**

```bash
npm run test:frontend
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected：全部成功，`git diff --check` 无输出。

- [ ] **Step 2: 验证隐私边界**

Run:

```bash
rg -n "transcript|instruction|api.?key|secret" src-tauri/src/session/telemetry.rs
```

Expected：不出现保存正文、修改指令或 Key 的字段；只允许类型名或说明性注释。

- [ ] **Step 3: 记录计划完成提交**

如果验证未产生文件变化，不创建空提交。记录最后一个有效提交 SHA，作为 P1 工作树起点。
