# M3 手工验收收口（Mac）

**日期：** 2026-07-27  
**结论：** Mac 上「录音 → 本地 Whisper → 交付」主路径可用；正式安装身份为 `/Applications/Luozi.app`。

## 环境

| 项 | 值 |
|---|---|
| 安装 | `npm run install:macos-app` → `/Applications/Luozi.app` |
| 模型 | `~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin`（~465MB） |
| 热键 | 按住 `Control+Option+空格`（Control+Alt+Space） |
| 权限 | 辅助功能 + 麦克风均授给 **Applications 里的 Luozi** |

## 验收清单

- [x] 有模型时短句转写为真实文字（非「落字测试」）
- [x] 备忘录等系统文本框可落字或明确剪贴板降级
- [x] Esc 可取消听写中
- [x] 无假成功「已落字」（须读回校验；否则提示剪贴板）
- [x] `npm run install:macos-app` 后可在「应用程序」里找到 Luozi
- [ ] CER / 延迟基线表格（后续补测，不挡进 M4）

## 已知限制

- Cursor / Electron 编辑器常只能剪贴板 + ⌘V
- adhoc 重签后辅助功能需关开一次
- 尚无应用内模型下载器（→ M4）
- 无文本 AI 纠错润色（→ M7）

## 相关

- 详细步骤：`m3-manual.md`
- 安装脚本：`scripts/install-macos-app.sh`
