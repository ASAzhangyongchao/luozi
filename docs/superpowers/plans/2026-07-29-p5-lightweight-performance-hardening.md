# Luozi P5 Lightweight Performance Hardening Implementation Plan

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan step-by-step.

**Goal:** 让 Luozi 在非录音状态只保留必要菜单栏与快捷键能力，并用实测证明隐藏窗口、模型、动画、轮询和网络不会持续消耗资源。

**Architecture:** 除 HUD 外的 WebView 改为按需创建和关闭销毁；本地 ASR 使用可测试的 120 秒闲置策略；前端控制器拥有明确 dispose 生命周期；开发脚本按进程树采样 CPU / RSS / 网络，最终报告记录可控与系统基线。

**Tech Stack:** Rust 2021、Tauri 2、TypeScript、Vite 6、macOS `ps` / `nettop` / unified log。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md` §12、§15

**Prerequisite:** P4 已完成并通过快捷键实机矩阵。

---

## 文件结构

| 文件 | 动作 | 单一职责 |
|---|---|---|
| `src-tauri/src/windows.rs` | Create | 草稿、设置和帮助窗口的按需创建 |
| `src-tauri/src/lib.rs` | Modify | 使用 window factory，删除常驻隐藏 WebView |
| `src-tauri/tauri.conf.json` | Modify | 只预创建必要 HUD |
| `src-tauri/src/session/model_lifecycle.rs` | Create | 120 秒闲置判断和卸载动作 |
| `src-tauri/src/session/controller.rs` | Modify | 触碰模型使用时间和执行卸载 |
| `src-tauri/src/session/mod.rs` | Modify | 注册 model lifecycle |
| `src/main.ts` | Modify | dispose 监听、计时器和 mirror |
| `src/settings.ts` | Modify | dispose 权限轮询和分区控制器 |
| `src/overlay.ts` | Modify | HUD 隐藏后停止计时器和能量更新 |
| `src/styles.css` / `src/styles-settings.css` / `src/overlay.css` | Modify | hidden 与 reduced-motion 停止动画 |
| `tests/frontend/lifecycle.test.ts` | Create | disposable registry 和隐藏态模型 |
| `scripts/measure-idle-macos.sh` | Create | 五分钟进程树 CPU / RSS / 网络采样 |
| `docs/spikes/p5-performance-report.md` | Create | 目标、结果、差异和结论 |

---

### Task 1: 按需创建草稿和设置窗口

**Files:**

- Create: `src-tauri/src/windows.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 写窗口描述测试**

`windows.rs` 定义纯描述：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuxWindow {
    Draft,
    Settings,
}

impl AuxWindow {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "main",
            Self::Settings => "settings",
        }
    }

    pub fn url(self) -> &'static str {
        match self {
            Self::Draft => "index.html",
            Self::Settings => "settings.html",
        }
    }

    pub fn size(self) -> (f64, f64) {
        match self {
            Self::Draft => (760.0, 600.0),
            Self::Settings => (760.0, 560.0),
        }
    }
}
```

测试两个 label、URL 和尺寸。

- [ ] **Step 2: 实现 open_or_create**

```rust
pub fn open_or_create(
    app: &tauri::AppHandle,
    kind: AuxWindow,
) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    if let Some(window) = app.get_webview_window(kind.label()) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let (width, height) = kind.size();
    WebviewWindowBuilder::new(app, kind.label(), WebviewUrl::App(kind.url().into()))
        .title(match kind {
            AuxWindow::Draft => "落字 · 语音草稿",
            AuxWindow::Settings => "落字 · 设置",
        })
        .inner_size(width, height)
        .skip_taskbar(true)
        .build()
        .map_err(|e| format!("window_create_failed: {e}"))?;
    Ok(())
}
```

- [ ] **Step 3: 删除预创建窗口**

从 `tauri.conf.json` 的 `windows` 移除 `main` 和 `settings`，保留 `overlay`。菜单和命令打开窗口时调用 `open_or_create`。

关闭草稿前先 `DraftStore::flush()`，随后允许 window destroy；设置直接 destroy。不得继续 `prevent_close + hide`。

- [ ] **Step 4: 运行测试**

```bash
cargo test -p luozi windows
cargo test -p luozi overlay_transparency_uses_macos_private_api
cargo check -p luozi
```

Expected：成功，config 中只剩 overlay 预创建。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/windows.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "perf: create auxiliary windows on demand"
```

---

### Task 2: 把模型闲置卸载改为可测试策略

**Files:**

- Create: `src-tauri/src/session/model_lifecycle.rs`
- Modify: `src-tauri/src/session/controller.rs`
- Modify: `src-tauri/src/session/mod.rs`

- [ ] **Step 1: 写闲置策略测试**

```rust
pub const DEFAULT_MODEL_IDLE_SECS: u64 = 120;

pub fn should_unload_model(
    session_idle: bool,
    model_loaded: bool,
    idle_secs: Option<u64>,
) -> bool {
    session_idle
        && model_loaded
        && idle_secs.is_some_and(|value| value >= DEFAULT_MODEL_IDLE_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unloads_only_after_two_idle_minutes() {
        assert!(!should_unload_model(true, true, Some(119)));
        assert!(should_unload_model(true, true, Some(120)));
        assert!(!should_unload_model(false, true, Some(300)));
        assert!(!should_unload_model(true, false, Some(300)));
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p luozi model_lifecycle
```

Expected：PASS。

- [ ] **Step 3: 替换 300 秒硬编码**

`controller::maybe_unload_idle_asr` 调用 `should_unload_model`。轮询间隔从 60 秒降到 30 秒，但线程只读取原子 / Mutex 状态，不唤醒 WebView、不请求网络。

日志改为：

```text
luozi: unloaded local ASR after 120s idle
```

- [ ] **Step 4: 运行测试并提交**

```bash
cargo test -p luozi model_lifecycle
cargo test -p luozi controller
git add src-tauri/src/session/model_lifecycle.rs src-tauri/src/session/controller.rs src-tauri/src/session/mod.rs src-tauri/src/lib.rs
git commit -m "perf: unload local ASR after two idle minutes"
```

---

### Task 3: 为前端建立统一 dispose 生命周期

**Files:**

- Create: `src/lifecycle.ts`
- Create: `tests/frontend/lifecycle.test.ts`
- Modify: `src/main.ts`
- Modify: `src/settings.ts`
- Modify: `src/overlay.ts`

- [ ] **Step 1: 写 disposable registry 测试**

```ts
export type Disposer = () => void | Promise<void>;

export class DisposerRegistry {
  private items: Disposer[] = [];

  add(disposer: Disposer): void {
    this.items.push(disposer);
  }

  async dispose(): Promise<void> {
    const items = this.items.splice(0).reverse();
    for (const item of items) await item();
  }

  size(): number {
    return this.items.length;
  }
}
```

测试逆序调用、只调用一次和 dispose 后 size 为 0。

- [ ] **Step 2: 接入三个入口**

- 所有 `listen` 返回的 unlisten 加入 registry。
- 所有 interval / timeout 的 clear 加入 registry。
- mirror scroll / resize listener 加入 registry。
- settings section controller cleanup 加入 registry。
- window close / pagehide 时调用 `dispose()`。

不得把 `setInterval` 保存在模块外而无 cleanup。

- [ ] **Step 3: HUD 隐藏停机**

overlay 收到非 sticky 状态后：

1. 清除能量值。
2. 清除 hide timer。
3. window hidden 后设置 `data-active="false"`。
4. 后续晚到 energy 事件在非 active 时丢弃。

- [ ] **Step 4: 运行测试并提交**

```bash
npm run test:frontend -- tests/frontend/lifecycle.test.ts
npm run build
git add src/lifecycle.ts tests/frontend/lifecycle.test.ts src/main.ts src/settings.ts src/overlay.ts
git commit -m "perf: dispose hidden frontend activity"
```

---

### Task 4: 停止隐藏态和无障碍场景动画

**Files:**

- Modify: `src/styles.css`
- Modify: `src/styles-settings.css`
- Modify: `src/overlay.css`

- [ ] **Step 1: 加入统一停机规则**

三个 CSS 均包含：

```css
[hidden],
[data-active="false"] {
  animation: none !important;
  transition: none !important;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    scroll-behavior: auto !important;
  }
}
```

- [ ] **Step 2: 删除空闲循环动画**

搜索并删除空闲状态的 `infinite` animation。只允许 HUD 可见且处于 recording / processing 时播放有限状态反馈。

- [ ] **Step 3: 验证**

```bash
rg -n "infinite|requestAnimationFrame|setInterval" src
npm run build
```

Expected：`infinite` 只允许出现在测试说明或明确的 loading 且元素隐藏后销毁；所有 `setInterval` 均有 registry cleanup。

- [ ] **Step 4: 提交**

```bash
git add src/styles.css src/styles-settings.css src/overlay.css
git commit -m "perf: stop hidden and idle animations"
```

---

### Task 5: 创建可重复的 Mac 资源采样脚本

**Files:**

- Create: `scripts/measure-idle-macos.sh`

- [ ] **Step 1: 写脚本**

脚本参数固定为 App 可执行文件名和采样秒数，默认 `Luozi`、`300`。每秒执行：

```bash
#!/usr/bin/env bash
set -euo pipefail

app_name="${1:-Luozi}"
duration="${2:-300}"
interval=1

root_pid="$(pgrep -x "$app_name" | head -n 1 || true)"
if [[ -z "$root_pid" ]]; then
  echo "error=app_not_running app=${app_name}" >&2
  exit 2
fi

csv="$(mktemp -t luozi-resource.XXXXXX.csv)"
cpu_sorted="${csv}.cpu"

collect_pids() {
  local parent="$1"
  local child
  printf '%s\n' "$parent"
  pgrep -P "$parent" 2>/dev/null | while read -r child; do
    collect_pids "$child"
  done
}

count_tcp_connections() {
  local pids_csv="$1"
  local total=0
  local pid
  IFS=',' read -r -a pid_list <<< "$pids_csv"
  for pid in "${pid_list[@]}"; do
    if [[ -n "$pid" ]]; then
      count="$(lsof -nP -a -p "$pid" -iTCP -sTCP:ESTABLISHED 2>/dev/null | tail -n +2 | wc -l | tr -d ' ')"
      total=$((total + count))
    fi
  done
  printf '%s' "$total"
}

median_file() {
  awk '{ values[NR] = $1 } END {
    if (NR == 0) {
      print "0.00"
    } else if (NR % 2 == 1) {
      printf "%.2f", values[(NR + 1) / 2]
    } else {
      printf "%.2f", (values[NR / 2] + values[NR / 2 + 1]) / 2
    }
  }' "$1"
}

echo "sample,cpu_percent,rss_mb,network_connections" > "$csv"

sample=1
while [[ "$sample" -le "$duration" ]]; do
  pids_csv="$(collect_pids "$root_pid" | sort -u | paste -sd, -)"
  metrics="$(ps -p "$pids_csv" -o %cpu=,rss= 2>/dev/null | awk '
    { cpu += $1; rss += $2 }
    END {
      if (NR == 0) {
        printf "0.00,0.00"
      } else {
        printf "%.2f,%.2f", cpu, rss / 1024
      }
    }')"
  network_connections="$(count_tcp_connections "$pids_csv")"
  echo "${sample},${metrics},${network_connections}" >> "$csv"
  sample=$((sample + interval))
  sleep "$interval"
done

tail -n +2 "$csv" | cut -d, -f2 | sort -n > "$cpu_sorted"
samples="$(tail -n +2 "$csv" | wc -l | tr -d ' ')"
cpu_median="$(median_file "$cpu_sorted")"
cpu_max="$(tail -n 1 "$cpu_sorted")"
rss_max_mb="$(tail -n +2 "$csv" | cut -d, -f3 | sort -n | tail -n 1)"
network_connections="$(tail -n +2 "$csv" | cut -d, -f4 | sort -n | tail -n 1)"

echo "samples=${samples}"
echo "cpu_median=${cpu_median}"
echo "cpu_max=${cpu_max}"
echo "rss_max_mb=${rss_max_mb}"
echo "network_connections=${network_connections}"
echo "csv=${csv}"
```

脚本筛选主进程和递归子进程，累加 `%CPU` 和 RSS，写入临时 CSV。结束时输出：

```text
samples=<n>
cpu_median=<value>
cpu_max=<value>
rss_max_mb=<value>
network_connections=<value>
```

网络连接按样本统计主进程和递归子进程的已建立 TCP 连接。脚本不得输出命令行环境变量。

- [ ] **Step 2: Shell 语法检查**

```bash
bash -n scripts/measure-idle-macos.sh
```

Expected：无输出，exit 0。

- [ ] **Step 3: 运行 30 秒冒烟**

```bash
bash scripts/measure-idle-macos.sh Luozi 30
```

Expected：30 个样本，输出 CPU median、CPU max、RSS max 和连接数。

- [ ] **Step 4: 提交**

```bash
git add scripts/measure-idle-macos.sh
git commit -m "test: add macOS idle resource sampler"
```

---

### Task 6: 最终轻量性能验收

**Files:**

- Create: `docs/spikes/p5-performance-report.md`

- [ ] **Step 1: 采集四个状态**

每个状态采样五分钟：

1. 只显示菜单栏，未打开任何窗口。
2. 打开再关闭设置。
3. 打开再关闭草稿。
4. 使用一次本地识别，等待 120 秒模型卸载后继续采样。

- [ ] **Step 2: 验证目标**

- idle CPU median `< 0.5%`。
- 无本地模型时进程树 RSS 目标 `≤ 150 MB`。
- hidden 后无持续 WebView 进程或可观察轮询。
- idle 无已建立 AI 服务网络连接。
- 麦克风指示在录音结束后关闭。

- [ ] **Step 3: 写性能报告**

报告必须包含 Mac 型号、macOS、构建类型、四组原始摘要、是否达标和进程拆分。若 WebKit 系统基线导致 RSS 超标，列出主进程与 WebView 子进程各自 RSS 和最小实测值，不修改验收数字掩盖结果。

- [ ] **Step 4: 最终全量验证**

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
git add docs/spikes/p5-performance-report.md
git commit -m "docs: record lightweight performance acceptance"
```

---

### Task 7: 总设计回归

**Files:**

- Modify: `CHANGELOG.md`
- Modify: `README.md`
- Modify: `README.zh-CN.md`

- [ ] **Step 1: 更新用户文档**

只描述已实际通过验收的能力：

- 菜单栏语音输入。
- AI 服务连接。
- 三种输入模式。
- 可配置快捷键。
- 草稿问题标记。
- 本地 / 云端与隐私边界。

不得把 Spike No-Go 的特殊键写成支持。

- [ ] **Step 2: 运行文档链接和敏感信息扫描**

```bash
rg -n "TODO|TBD|FIXME" README.md README.zh-CN.md CHANGELOG.md
rg -n "sk-[A-Za-z0-9_-]{12,}" . --glob '!target/**' --glob '!node_modules/**'
```

Expected：无占位符，无真实 Key。

- [ ] **Step 3: 提交**

```bash
git add README.md README.zh-CN.md CHANGELOG.md
git commit -m "docs: document Luozi core experience"
```
