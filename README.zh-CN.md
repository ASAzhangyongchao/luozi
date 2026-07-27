# 落字（Luozi）

跨平台快捷语音转文字桌面应用（Tauri 2 + Rust）。

**现状：** Mac-first **M6 语音草稿工作台**（编辑/粘贴/复制/自动保存/撤销；工作台焦点时语音写入草稿）。M5 Groq 云端仍可用。**尚无可下载 Release。**

## 当前可用

- 按住 `Control+Alt+Space`：转写 → 工作台焦点则写入草稿，否则 AX / 剪贴板
- 托盘 **打开语音草稿**、引擎模式、Groq Key/同意、下载模型
- 手工验收：`docs/spikes/m5-manual.md`、`docs/spikes/m6-manual.md`

**云端（M5）：** Key 只进钥匙串；默认不上传；须同意 `api.groq.com`。

**权限：** `npm run install:macos-app` → `/Applications/Luozi.app`。

## 明确还没有

说出修改 / 文本 AI（M7）、完整设置页（M8）、Release 安装包、公证。

## 许可证

MIT — 见 [LICENSE](./LICENSE)。
