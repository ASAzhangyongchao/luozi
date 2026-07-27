# M4 手工验收 — 模型获取与校验

**目标：** 新机器无需手搓路径，能下载并 SHA256 校验 `ggml-small`；缺模型时有明确修复入口。

## 前置

- 已能跑 `/Applications/Luozi.app`（`npm run install:macos-app`）
- 磁盘空闲 ≥ ~550MB
- 可选代理：`export https_proxy=…`（Hugging Face 慢时）

## 路径 A：CLI

```bash
cd apps/luozi
npm run fetch:model
```

期望：

1. 下载到 `~/Library/Application Support/app.luozi.desktop/models/ggml-small.bin`
2. 打印 `done`；重复执行显示 `already present and verified`
3. 故意损坏文件后再跑 → 重新下载并通过校验

## 路径 B：托盘

1. 临时移走模型文件（或新机器未下载）
2. 启动落字 → 托盘状态含「模型未就绪」
3. 菜单 **下载推荐模型…**
4. HUD / 日志出现下载进度；完成后提示「模型已就绪」
5. 按住 `Control+Alt+Space` 能正常转写

## 空闲卸载（可选）

1. 成功转写一次后，空闲 ≥5 分钟
2. 日志应出现 `unloaded Whisper after 5m idle`
3. 再次按住说话仍可懒加载并转写

## 失败场景

| 场景 | 期望 |
|---|---|
| 磁盘不足 | 明确 `model_disk_full` / 中文提示，不留下坏 `.bin` |
| 校验失败 | 删除 partial，可重试 |
| 重复点下载 | 提示「模型正在下载中…」，不并行双开 |

## 明确不测（本切片）

- base/small 分档 UI、精致进度条、Windows 实机下载宣称
