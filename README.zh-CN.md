# 落字（Luozi）

跨平台快捷语音转文字桌面应用（Tauri 2 + Rust）。

**现状：** Mac-first **M8 设置壳（最短）**：独立设置 / 关于窗，可配 ASR 与文本 AI、查看快捷键与权限。M7 语音修改 / M6 草稿 / M5 Groq 仍可用。**尚无 Release 安装包。**

## 当前可用

- 按住 `Control+Alt+Space`：转写 → 工作台焦点则写入草稿，否则 AX / 剪贴板
- 按住 `Control+Alt+Shift+Space`：说修改要求 → 文本 AI 改草稿
- 托盘：**设置…**、**如何使用…**、关于、打开语音草稿、引擎模式、Key/同意、下载模型
- 手工验收：`docs/spikes/m5-manual.md` … `m8-manual.md`、`guide-manual.md`

**云端：** ASR 与文本 AI 分开授权；Key 只进钥匙串。

**权限：** `npm run install:macos-app` → `/Applications/Luozi.app`。

## 明确还没有

首次自动引导（菜单内教程已提供）、液态玻璃精修、快捷键录制改键、Release 安装包、公证。

## 许可证

MIT — 见 [LICENSE](./LICENSE)。
