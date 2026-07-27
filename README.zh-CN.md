# 落字（Luozi）

跨平台快捷语音转文字桌面应用（Tauri 2 + Rust）。

**现状：** Mac-first **M7 语音修改（最短）**：第二快捷键 + 文本 AI（Groq chat）改选区/当前段；保护片段高风险需确认。M6 草稿 / M5 Groq ASR 仍可用。**尚无 Release 安装包。**

## 当前可用

- 按住 `Control+Alt+Space`：转写 → 工作台焦点则写入草稿，否则 AX / 剪贴板
- 按住 `Control+Alt+Shift+Space`：说修改要求 → 文本 AI 改草稿（需单独 Key + 同意）
- 托盘：打开语音草稿、引擎模式、ASR/文本 AI Key 与同意、下载模型
- 手工验收：`docs/spikes/m5-manual.md`、`docs/spikes/m6-manual.md`、`docs/spikes/m7-manual.md`

**云端：** ASR 与文本 AI 分开授权；Key 只进钥匙串。

**权限：** `npm run install:macos-app` → `/Applications/Luozi.app`。

## 明确还没有

完整疑似词 UI / diff sheet、设置页（M8）、Release 安装包、公证。

## 许可证

MIT — 见 [LICENSE](./LICENSE)。
