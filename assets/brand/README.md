# 落字品牌资产（落光标）

矢量真相源目录。公开展示与 App 图标必须以本目录 SVG 为准。

| 文件 | 用途 |
|---|---|
| `luozi-mark.svg` | 彩色「落光标」主标（圆角底 + 光标 + 字点 + 基线 + 不对称回声弧） |
| `luozi-tray-template.svg` | 菜单栏 / 托盘单色线稿矢量 |
| `luozi-wordmark-zh.svg` | 「落字 / LUOZI」字标组合 |
| `luozi-mark-1024.png` | RGBA App 图标源（圆角外透明，无白底）；生成 `src-tauri/icons/*` |

运行时菜单栏使用 `src-tauri/icons/tray-template@2x.png`（黑+透明 Template），经 `icon_as_template(true)` 交给系统着色。

颜色：夜青 `#0B3034`、极光青 `#42D9D3`、冰白 `#F4FBFA`。

禁止：多柱 EQ、麦克风剪影、聊天泡、Tauri 默认蓝黄环、衬线文学风字标。

前端练习窗通过 `public/assets/brand/` 镜像 SVG 引用；改真相源后请同步复制到 `public/`。
