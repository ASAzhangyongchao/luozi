# M5 手工验收 — 云端 Groq + 本地优先路由

**目标：** 本地优先；仅在本地不可用/失败且已配置+授权时上传到 Groq；未同意不外发。

## 前置

- `npm run install:macos-app` 安装最新构建
- 可选：本地模型已就绪（`npm run fetch:model`）
- Groq API Key（[console.groq.com](https://console.groq.com)）

## A. 未同意不得外发

1. 托盘确认尚未点「同意上传到 Groq…」
2. 切到「仅云端」或去掉本地模型后按住说话
3. 期望 HUD：`云端未授权`；日志无 `cloud asr → host=`

## B. 配置 Groq

1. 托盘 **配置 Groq Key…** → 粘贴 Key → 保存（进钥匙串，不进 `config.json`）
2. 托盘 **同意上传到 Groq…** → 提示已同意 `api.groq.com`
3. 开发替代：`export LUOZI_GROQ_API_KEY=...`（仅本机进程，勿提交）

## C. 自动模式不上传（本地就绪）

1. 引擎模式：**自动（本地优先）**
2. 本地模型存在时按住说话
3. 日志应走本地 Whisper，**不应**出现 `asr route → cloud`

## D. 仅云端 / 本地缺失走云端

1. 切到 **仅云端**，或暂时移走 `ggml-small.bin`
2. Key + 同意已就绪
3. 按住说短句 → 日志 `asr route → cloud` → 落字成功

## E. 错误映射

| 情况 | 期望文案 |
|---|---|
| 错 Key | 云端 Key 无效 |
| 未同意 | 云端未授权 |
| 无引擎 | 无可用引擎 |

## 明确不测

- 硅基 / 智谱、完整设置页、竞速、Windows 凭据库
