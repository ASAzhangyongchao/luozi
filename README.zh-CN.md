# 落字（Luozi）

跨平台快捷语音转文字桌面应用（Tauri 2 + Rust）。

**现状：** Mac-first 受限 M1 骨架。**尚无可下载 Release。**

## 当前可用

- 托盘菜单：练习窗 / 关于 / 退出
- 共享 crate `luozi-core`，`AppConfig.schemaVersion = 1`
- macOS M0 Spike 证据见 `docs/spikes/`、`tests/manual/`（Overall = **Partial**；Windows 无实机）

## 快捷键（临时）

在双端 M0 Go 之前，下列组合仅为 **provisional**，不是正式产品承诺：

| 动作 | 临时绑定（macOS） |
|---|---|
| 继续说 | `Control+Alt+Space` |
| 语音修改 | `Control+Alt+M` |

## 开发

```bash
npm ci
npm run tauri dev
```

M0 Spike 控制台：练习窗 URL 加 `?spike`。

## M1 不做

会话状态机、ASR、工作台、文本 AI、公开仓推送、公证安装包。

## 许可证

MIT — 见 [LICENSE](./LICENSE)。
