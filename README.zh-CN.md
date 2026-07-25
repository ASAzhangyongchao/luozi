# 落字（Luozi）

跨平台快捷语音转文字桌面应用（Tauri 2 + Rust）。

**现状：** Mac-first **M2 会话骨架**（假文本交付）。**尚无可下载 Release。**

## 当前可用

- 按住 `Control+Alt+Space`：真录音 → 松手后交付假文本「落字测试」（AX 插入 → 剪贴板 + ⌘V → 仅剪贴板）
- Esc（仅录音中注册）：取消当前会话
- 托盘：开始语音输入 / 取消 / 撤销上次落字（剪贴板路径）/ 练习窗 / 关于 / 退出
- `luozi-core` 会话状态机单测 + `AppConfig.schemaVersion = 1`
- 「落光 · Falling Cursor」品牌资产：`assets/brand/`
- 手工验收：`docs/spikes/m2-manual.md`
- M0 Spike 证据仍在 `docs/spikes/`、`tests/manual/`（Overall = **Partial**）

**权限：** 麦克风 / 辅助功能必须授给**实际运行的那个进程**。`npm run tauri dev` 用的是 `target/debug/luozi`；打包的 **`Luozi.app`** 是另一套签名，开关不能共用。

## 品牌

视觉语言：**落光 · Falling Cursor**（夜青 `#0B3034` + 极光青 `#42D9D3` + 冰白 `#F4FBFA`）。

## 快捷键（临时）

| 动作 | 临时绑定（macOS） |
|---|---|
| 继续说（按住） | `Control+Alt+Space` |
| 取消 | `Esc`（会话进行中） |
| 语音修改 | `Control+Alt+M`（尚未启用） |

## 开发

```bash
npm ci
npm run tauri dev          # 热迭代（辅助功能授给 debug/luozi）
cargo test --workspace
npm run run:macos-app      # 构建并打开 Luozi.app（稳定权限验收）
```

若已有 `src-tauri/target/release/bundle/macos/Luozi.app`，可直接 `open`，不必每次重编。

## 明确还没有

ASR / Whisper、工作台、语音修改、公开仓、公证安装包。

## 许可证

MIT — 见 [LICENSE](./LICENSE)。
