# 落字（Luozi）M0–M1 交付路线图

> 本文件只负责阶段边界、前置条件和 Go / No-Go 门禁，不直接作为逐条代码执行清单。
> M0 的可执行计划见：`docs/superpowers/plans/2026-07-25-落字Luozi-M0-风险Spike.md`。
> M1 的详细实现计划必须在 M0 报告确认快捷键、窗口和平台 API 后再生成。

**目标：** 先用最小双端实验消除落字最危险的技术不确定性，再建立公开仓骨架；不提前实现状态机、ASR、工作台或文本 AI。

**设计依据：** `docs/superpowers/specs/2026-07-25-落字Luozi-快捷录音转文字设计.md`

---

## 1. 已确定的执行原则

1. 产品代码放在 `/Users/zhangyongchao/knowledge-system/apps/luozi`（与 QuotaPet 同级），保持独立 `.git`；对 knowledge-system 根仓库禁止 `git add -A`。
2. M0 期间不创建公开远程仓库；只保留本地 Git 历史，并通过 `git bundle` 把同一提交带到 Windows 实机验证。
3. M0 只回答可行性问题，不把实验代码包装成正式产品模块。
4. M0 通过后才写 M1 详细实现计划；M0 结论可能改变快捷键、窗口桥接和目录结构，禁止提前假定。
5. M1 只交付公开仓骨架、配置 schema、最小托盘壳和双端 CI；会话状态机属于 M2。
6. 任何 Git 提交先确认仓库根目录，再按明确路径暂存；禁止在 knowledge-system 中执行 `git add -A`。
7. 创建公开 GitHub 仓、推送源码、在 knowledge-system 添加 submodule 都是独立门禁，必须在执行当时取得用户确认。
8. CI 只证明双端可编译；全局快捷键、焦点、麦克风、Word、输入法、UAC 和 SmartScreen 必须用实际系统验证。

---

## 2. 阶段关系

```text
设计规格已批准
      │
      ▼
M0 双端风险 Spike
      ├─ No-Go ─► 更新设计 / 单独写平台桥接 Spike ─► 重新执行 M0
      │
      └─ Go
          │
          ▼
      编写 M1 详细计划
          │
          ▼
      M1 公开仓骨架
          │
          ▼
      用户确认创建公开仓并推送
          │
          ▼
      用户确认 knowledge-system 挂 submodule
          │
          ▼
      M2 会话与交付
```

M0 和 M1 不能并行：M1 的默认快捷键、平台模块边界、窗口方案和权限说明依赖 M0 结论。

---

## 3. M0：双端风险 Spike

### 3.1 范围

M0 只验证五件事：

| 门 | 必须回答的问题 | 证据 |
|---|---|---|
| G1 全局快捷键 | Mac / Windows 能否稳定收到 `Pressed` / `Released`；哪些组合与系统、输入法和常用软件不冲突 | 双端热键矩阵、事件日志、默认键候选顺序 |
| G2 不抢焦点悬浮窗 | 录音条显示、更新、隐藏时是否保持原输入焦点 | TextEdit / Notepad、Word、浏览器、VS Code 手工记录 |
| G3 安全交付 | 能否捕获并复核同一 AX / UIA 输入元素；目标变化和安全输入是否零误投 | 交付矩阵、零误投记录 |
| G4 权限与材质 | 麦克风、Accessibility / UIA、锁屏 / 安全桌面、系统材质与不透明回退是否可控 | 权限截图、拒绝路径、资源记录 |
| G5 双端构建 | 同一提交能否在 Mac / Windows 构建并启动未签名包 | 构建日志、产物路径、Gatekeeper / SmartScreen 现象 |

### 3.2 明确不做

- 不实现会话状态机、`sessionId`、取消和晚到结果；这些属于 M2。
- 不接入 whisper.cpp、模型下载、云端 ASR 或文本 AI。
- 不做正式设置页、工作台、草稿、修订、标注和完整视觉。
- 不实现低级键盘钩子。官方插件候选全部失败时，M0 判定 No-Go，另写窄平台桥接计划。
- 不创建 Keychain / Credential Manager、更新器、安装器脚本或 Release。
- 不创建空的 `model-manifest/`、`docs/tutorials/` 和 `scripts/` 目录。

### 3.3 前置条件

- Mac：macOS 13+ 实机，当前用户可授予麦克风和 Accessibility 权限。
- Windows：Windows 10 22H2 或 Windows 11 x64 实机 / 可交互虚拟机；必须能安装 MSVC 构建工具、切换微软拼音并测试 UAC。
- 两端都安装 Rust stable、Node 当前 LTS、npm、Git。
- Tauri 全局快捷键插件要求 Rust 至少 `1.77.2`。
- Windows 环境不可用时，只能标记 `partial`，不得宣布 M0 通过。

### 3.4 快捷键候选

M0 按以下固定候选测试，不使用“其它组合”“备选 1”等占位：

| 平台 | 继续说候选顺序 | 语音修改候选顺序 |
|---|---|---|
| macOS | `Fn` → `Control+Option+Space` → `Control+Shift+Space` | `Control+Option+M` → `Control+Shift+M` |
| Windows | `Ctrl+Alt+Space` → `Ctrl+Shift+Space` | `Ctrl+Alt+M` → `Ctrl+Shift+M` |

规则：

1. `⌥⌘Space` 已被 macOS Finder 搜索占用，不进入候选。
2. `Fn` 只走官方插件的实测；如果硬件不产生独立事件，首版直接放弃 `Fn`，不为它引入低级监听。
3. 每个候选分别测试短按、按住 1 秒、按住 5 秒各 10 次，必须每次只有一对 `Pressed` / `Released`，无缺失、无重复、无粘连。
4. 官方插件的注册状态不能证明未被其它 App 占用；练习框真实触发结果是最终依据。
5. 默认键取第一个在系统、中文输入法、Word、浏览器和 VS Code 中均通过的组合；如果没有一组完整通过，M0 为 No-Go。

### 3.5 M0 Go 条件

必须同时满足：

- Mac / Windows 五门均有实机结果。
- 两个动作各找到一组稳定快捷键，或报告明确给出经过验证的同成本回退。
- 悬浮窗不激活、不夺走原输入焦点。
- `TargetToken` 至少绑定进程、窗口身份和 AX / UIA 元素身份；窗口标题不能单独作为身份。
- 转写等待期间切换 App、窗口或输入框，结果只能进入剪贴板，不能落到新焦点。
- 密码框、安全输入、锁屏和 UAC 场景为：零插入、零剪贴板、零正文反馈。
- 两端都能从同一 Git 提交构建并启动。
- `docs/spikes/m0-report.md` 对每一门给出 `pass`、`fail` 或 `partial`，并附证据路径。

任一安全项失败均为 No-Go；不能用“以后修”进入 M1。

### 3.6 已批准例外：Mac-first 受限 M1（2026-07-25）

用户于 2026-07-25 明确批准：在 M0 Overall 仍为 **Partial**（无 Windows 实机）的前提下，进入 **Mac-first 受限 M1**。

批准含义：

1. 不宣称双端 M0 Go；Windows 热键 / 焦点 / UIA / SmartScreen 仍标 `not available`。
2. 允许编写并执行 M1 详细计划：本地仓骨架、`luozi-core`、`AppConfig schemaVersion = 1`、最小托盘壳、双端 **CI 编译矩阵**（CI ≠ 实机验证）。
3. 仍禁止：创建公开 GitHub 仓、knowledge-system submodule、ASR / 状态机 / 工作台（M2+）。
4. 携带进 M1 的已知缺口：`Control+Alt+M` 实体全矩阵、Word / VS Code、系统认证框、拒麦 / 无设备 / 断开、原生系统材质；默认快捷键保持 **provisional**，不得写成已定稿产品承诺。

可执行计划：`docs/superpowers/plans/2026-07-25-落字Luozi-M1-Mac-first公开仓骨架.md`。

---

## 4. M1：公开仓骨架

M1 详细任务原规则为「M0 Go 后生成」。已批准例外见 §3.6；详细执行清单见 `2026-07-25-落字Luozi-M1-Mac-first公开仓骨架.md`。
本路线图仍只锁边界。

### 4.1 M1 必须交付

- 延续 `apps/luozi` 独立仓（与 QuotaPet 同级）；公开前保持独立 `.git`，确认后再挂 submodule。
- 清理或隔离 M0 实验代码，保留 `docs/spikes/m0-report.md` 和手工矩阵。
- Rust workspace 与 `luozi-core` 空 crate。
- `AppConfig schemaVersion = 1`，字段只包含已在设计中确定且 M0 已验证的默认键、通用设置和凭据引用；不含 Key 明文。
- 最小托盘壳：打开练习窗、关于、退出；不提前堆满不可用菜单。
- macOS / Windows GitHub Actions 矩阵，至少运行前端构建、Rust fmt、clippy、test 和 Tauri 编译检查。
- README 中英、LICENSE、CHANGELOG、SECURITY、CONTRIBUTING、AGENTS。
- README 在没有 Release 时明确写“尚无可下载版本”，不放失效下载链接。
- 仓库无模型、无密钥、无构建产物、无空占位目录。

### 4.2 M1 明确不做

- 会话状态机、录音会话、Esc 取消、目标交付、剪贴板撤销——M2。
- whisper.cpp 和本地模型——M3 / M4。
- 云端 ASR——M5。
- 单草稿工作台——M6。
- 语音修改和文本 AI——M7。
- 完整设置、引导、性能收口——M8。
- Release、签名、SHA256 和公开 `v0.1.0`——M9。

### 4.3 公开仓门禁

顺序固定：

1. 在独立本地仓完成 M1 所有提交。
2. 本地运行测试，并在 Mac / Windows CI 定义上完成静态复核。
3. 用户确认 GitHub owner、仓库名和公开可见性。
4. 在独立仓目录执行 `gh repo create ... --source=. --remote=origin --push`。
5. 等远程 CI 通过。
6. 用户再次确认后，在 knowledge-system 添加 `apps/落字` submodule。
7. knowledge-system 只暂存 `.gitmodules` 与 `apps/落字` gitlink，单独提交；不得混入其它改动。

---

## 5. 后续阶段边界

| 阶段 | 核心交付 | 不得提前 |
|---|---|---|
| M2 | 会话状态机、真录音、Esc、假文本交付、目标变化降级、剪贴板有限撤销 | ASR 模型、工作台 |
| M3 | 固定 GGML 模型手动放置、whisper.cpp、本地最短闭环 | 下载器、AI |
| M4 | 模型 manifest、下载、SHA256、按需加载和释放 | 云端预设 |
| M5 | 云端 ASR、凭据库、逐提供方授权 | 文本 AI |
| M6 | 单草稿、粘贴、续写、恢复、撤销 / 重做 | 深度 AI |
| M7 | 第二快捷键、BYOK、范围补丁、风险预览、标注 | 自动更新 |
| M8 | 设置、首次引导、系统材质、日志、安全和性能预算 | 正式发布 |
| M9 | 双端安装包、SHA256、Release、实机矩阵签字 | 二期能力 |

并行限制：

- M2 的交付路由未通过前，不大面积做工作台 UI。
- M3 的本地转写闭环未通过前，不启动 M7。
- 任一里程碑只做一轮与风险匹配的验证；没有新证据不重复全量检查。

---

## 6. 当前下一步

- M0 可执行计划已完成（Mac Partial）：`docs/superpowers/plans/2026-07-25-落字Luozi-M0-风险Spike.md`
- 产品仓报告：`apps/luozi/docs/spikes/m0-report.md`（Overall = Partial）
- Mac-first 受限 M1 骨架已落地；Logo / 托盘视觉收口后，用户于 2026-07-25 批准进入 M2
- **进行中**：`docs/superpowers/plans/2026-07-25-落字Luozi-M2-Mac-first会话与交付.md`
- 公开仓 / submodule 仍需另次确认；不宣称双端 M0 Go
