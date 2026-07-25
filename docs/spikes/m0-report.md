# Luozi M0 Spike Report

## Scope

本报告只覆盖热键、不抢焦点悬浮窗、安全目标复核、麦克风 / 系统材质和双端构建。

不覆盖：会话状态机、whisper.cpp / ASR、工作台、文本 AI、公开仓、Release。

## Commit

- Tested commit: `47dcf6b6b7291b84ecdeb8bac4b95f1f3e3c88e2` (`spike/m0`)
- Product root: `/Users/zhangyongchao/knowledge-system/apps/luozi`
- Report date: 2026-07-25
- macOS machine: arm64, macOS 26.5.2 (Build 25F84)

## Gate Results

| Gate | macOS | Windows | Overall | Evidence |
|---|---|---|---|---|
| G1 Hotkey Pressed / Released | pass (with limits) | not available | partial | `tests/manual/m0-hotkey-macos.md`；`Fn` fail；四组非 Fn 候选有 Pressed/Released |
| G2 Non-activating overlay | pass (TextEdit) | not available | partial | `tests/manual/m0-focus-delivery-macos.md`；`m0-focus-auto-result.json`；Word / VS Code 本机无；浏览器 overlay ABC 未跑全 |
| G3 Target validation and safe delivery | pass (auto core cases) | not available | partial | `m0-delivery-auto-result.json`：same_target / changed / secure 均 pass；无焦点、同 App 切框、认证框未覆盖 |
| G4 Permission and material fallback | pass (core) | not available | partial | `m0-permission-material-macos.md`；`m0-audio-auto-result.json`；拒绝麦克风 / 无设备 / 断开未手工模拟；60s idle CPU 未采 |
| G5 Build and launch | pass | not available | partial | `Luozi.app` + `Luozi_0.0.0_aarch64.dmg`；adhoc 签名 |

## Default Shortcut Decision

| Platform | Continue speaking | Voice edit | Evidence |
|---|---|---|---|
| macOS | `Control+Alt+Space` | `Control+Alt+M` | 官方插件可注册；有 Pressed/Released；`Fn` 不可用。冲突矩阵（IME / Word / 浏览器 / VS Code / 重启）未完整跑完，故为 provisional default |
| Windows | none | none | 无 Windows 实机，不选定 |

## Security Assertions

- New-focus misdelivery count: `0`（已测：等待中切 App → `changed`，零插入）
- Secure-input insertion count: `0`（密码框 → `secure`，未写入）
- Secure-input clipboard write count: `0`（M0 探针不写剪贴板）
- Secure-input full-text feedback count: `0`（不读取/日志密码正文）

未覆盖用例（不计入上述计数，但阻止 Overall Go）：无输入焦点、同 App 另一输入框、系统认证框。

## Resource Notes

- macOS idle CPU / memory: not instrumented
- Windows idle CPU / memory: not available
- 30 overlay cycles: pass（`m0-overlay-cycle-result.json`，约 2590 ms，`ok: true`）

## Platform Fallbacks

- macOS:
  - `Fn`：官方插件无法识别 → 首版放弃 `Fn`，不引入低级钩子
  - 透明悬浮条：Tauri `transparent` + 现有 `macOSPrivateApi`；材质用 CSS `backdrop-filter`，非私有 vibrancy API
  - `prefers-reduced-transparency: reduce` → 不透明 Canvas 高对比背景
  - AX 无法安全写入 → 返回 `unsupported`，预期剪贴板回退（M0 不实际覆盖剪贴板）
  - 安全输入 / 目标变化 → `secure` / `changed`，零插入
- Windows:
  - 代码骨架已在 `src-tauri/src/spike/windows.rs`，本机未编译验证
  - 无实机前不声明 Mica / UIA / SmartScreen 行为

## Decision

- Result: **Partial**
- Blocking failures:
  1. Windows 实机不可用（路线图要求双端才可 Go）
  2. macOS 热键冲突矩阵与重启复注册未完整
  3. Word / VS Code 本机缺失；浏览器 overlay ABC 未完整
  4. 交付安全矩阵若干分支未覆盖（无焦点、同 App 切框、认证框）
  5. 麦克风拒绝 / 无设备 / 断开路径未手工验证
- Required spec changes:
  - 记录 macOS provisional 默认键：继续说 `Control+Alt+Space`，语音修改 `Control+Alt+M`
  - 明确首版不依赖 `Fn`
  - Windows 默认键与平台回退保持 “待实机后写入”，不在本报告假装已定

## Next Actions (M0 boundary)

1. 不创建公开 GitHub 仓，不添加 knowledge-system submodule，不编写 / 执行 M1。
2. 用户仅有 Mac 时：可将本 Partial 作为 “Mac 风险已摸清” 的内部结论；若要正式 Go，需补 Windows 实机或修订路线图门禁。
3. 若继续产品化，下一步应是用户明确批准后：要么补 Windows Spike，要么批准 “Mac-first Partial → 受限 M1 计划”。
