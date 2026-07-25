# Luozi M0 Spike Report

## Scope

本报告只覆盖热键、不抢焦点悬浮窗、安全目标复核、麦克风 / 系统材质和双端构建。

不覆盖：会话状态机、whisper.cpp / ASR、工作台、文本 AI、公开仓、公开分发。

## Commit

- Tested implementation commit: `cc1038b` (`codex/m0-mac-closure`)
- Prior M0 implementation commit: `47dcf6b6b7291b84ecdeb8bac4b95f1f3e3c88e2` (`spike/m0`)
- Product root: `/Users/zhangyongchao/knowledge-system/apps/luozi`
- Report date: 2026-07-25
- macOS machine: arm64, macOS 26.5.2 (Build 25F84)

## Gate Results

| Gate | macOS | Windows | Overall | Evidence |
|---|---|---|---|---|
| G1 Hotkey Pressed / Released | partial | not available | partial | `Control+Alt+Space` 已完成短按 / 1s / 5s 各 10 次、WeType、Safari 与重启复注册；`Control+Alt+M` 仍缺实体按键全矩阵；Word / VS Code 本机无 |
| G2 Non-activating overlay | partial | not available | partial | TextEdit / Safari 焦点不变、Safari 连续输入与点击悬浮条均通过；Word / VS Code 本机无；一次固定延时初始化丢首字母但未发生焦点切换 |
| G3 Target validation and safe delivery | partial | not available | partial | same target、切 App、同 App 切输入框、安全输入、无输入焦点均通过且零误投；系统认证框未覆盖 |
| G4 Permission and material fallback | partial | not available | partial | 录音允许与清理、不透明降级、5 分钟资源基线通过；拒绝权限 / 无设备 / 录音中断开、GPU、原生系统材质未覆盖 |
| G5 Build and launch | pass (internal build / launch) | not available | partial | `Luozi.app` + `Luozi_0.0.0_aarch64.dmg` 构建、启动、退出、重启通过；仅 adhoc 签名，不作分发结论 |

## Default Shortcut Decision

| Platform | Continue speaking | Voice edit | Evidence |
|---|---|---|---|
| macOS | `Control+Alt+Space` | `Control+Alt+M` | 两者保持 provisional，不回写为正式默认键。Space 的短按 / 长按 / IME / Safari / 重启证据已齐；M 仅确认可注册并有既往事件，仍缺实体按键完整矩阵；`Fn` 不可用 |
| Windows | none | none | 无 Windows 实机，不选定 |

## Security Assertions

- New-focus misdelivery count: `0`（已测：等待中切 App、同 App 切输入框 → `changed`，零插入）
- Non-input insertion count: `0`（按钮焦点 → `unsupported`，两个文本框均为空）
- Secure-input insertion count: `0`（密码框 → `secure`，未写入）
- Secure-input clipboard write count: `0`（M0 探针不写剪贴板）
- Secure-input full-text feedback count: `0`（不读取/日志密码正文）

未覆盖用例（不计入上述计数，但阻止 Overall Go）：macOS 系统认证框。

## Resource Notes

- macOS Release Spike（主窗口打开，非最终托盘态）5 分钟、300 次采样：
  - 主进程 CPU P95 `0.3%`，max `1.1%`
  - 相关 4 进程总 CPU P95 `0.5%`，max `1.9%`
  - `top` 相关进程内存首值 `74.53 MiB`、末值 `74.47 MiB`、max `87.52 MiB`，未见持续增长
  - RSS 快照相关进程总量 `164.16 MiB → 107.78 MiB`；主进程末值 `59.70 MiB`
  - 3 个 WebKit 进程按相邻启动时间与角色归入本次 Spike
  - 未采集单进程 GPU；当前 Spike 没有托盘生命周期，不能据此声明最终托盘态 `≤ 120 MiB`
- Windows idle CPU / memory: not available
- 30 overlay cycles: pass（`m0-overlay-cycle-result.json`，2592 ms，`ok: true`，`stoleFocus: false`）

## Platform Fallbacks

- macOS:
  - `Fn`：官方插件无法识别 → 首版放弃 `Fn`，不引入低级钩子
  - 悬浮条：关闭 Tauri `macOSPrivateApi` 与窗口透明，使用标准不透明 Canvas 作为可验收降级
  - 原生 Liquid Glass / vibrancy：M0 尚未完成公开 AppKit 材质桥接，不宣称已实现系统液态玻璃
  - 降低透明度：同样落到不透明高对比 Canvas，不依赖透明效果也可读
  - AX 无法安全写入 → 返回 `unsupported`，预期剪贴板回退（M0 不实际覆盖剪贴板）
  - 安全输入 / 目标变化 → `secure` / `changed`，零插入
- Windows:
  - 代码骨架已在 `src-tauri/src/spike/windows.rs`，本机未编译验证
  - 无实机前不声明 Mica / UIA / SmartScreen 行为

## Decision

- Result: **Partial**
- Blocking failures:
  1. Windows 实机不可用（路线图要求双端才可 Go）
  2. macOS 语音修改键 `Control+Alt+M` 缺实体按键短按 / 1s / 5s 各 10 次，以及应用冲突复核；Word / VS Code 本机缺失
  3. 系统认证框安全分支未验证
  4. 麦克风拒绝 / 无设备 / 录音中断开路径未手工验证
  5. 原生公开系统材质未验证；当前仅确认无私有 API 的不透明降级可用
  6. 最终托盘生命周期与 GPU 尚不能由当前主窗口 Spike 度量
- Required spec changes:
  - 暂不把候选键写成正式默认：继续说 `Control+Alt+Space`、语音修改 `Control+Alt+M` 继续保留 provisional
  - 记录首版不依赖 `Fn`，也不为它引入低级钩子
  - 记录无私有 API 的不透明 Canvas 为 M0 可用降级；原生系统材质另行验证
  - Windows 默认键与平台回退保持 “待实机后写入”，不在本报告假装已定

## Next Actions (M0 boundary)

1. ~~不创建公开 GitHub 仓，不添加 knowledge-system submodule，不编写 / 执行 M1。~~
2. **2026-07-25 用户批准**：进入 Mac-first 受限 M1（见知识库路线图 §3.6）。仍不创建公开仓、不挂 submodule，直至 M1 完成且用户再次确认。
3. 携带缺口：实体按键验证 `Control+Alt+M`、麦克风拒绝 / 恢复、系统认证框、无设备 / 断开、Word / VS Code、Windows 实机。
4. 执行计划：`docs/superpowers/plans/2026-07-25-落字Luozi-M1-Mac-first公开仓骨架.md`（knowledge-system）。
