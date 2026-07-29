# Luozi Core Experience Plan Index

> **For Implementation Worker:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement the selected P0-P5 plan step-by-step.

**Goal:** 将已确认的核心体验总设计拆成六个可独立交付、顺序执行的实施计划。

**Architecture:** 每个计划只建立一个清晰能力边界，并以前一计划的已验证提交作为基线。共享类型优先进入 `luozi-core`，系统 I/O 留在 `src-tauri`，设置和草稿只消费稳定 DTO；禁止继续把供应商、快捷键、识别整理和窗口生命周期堆进 `session/controller.rs`。

**Tech Stack:** Rust 2021、Tauri 2、TypeScript、Vite 6、Vitest、原生 HTML/CSS、macOS AppKit / Core Graphics 窄桥接。

**Spec:** `docs/superpowers/specs/2026-07-29-core-experience-ai-services-shortcuts-design.md`

---

## 执行前置条件

当前工作树存在未提交的提供方、设置、权限和 controller 改动。任何计划开始前必须执行：

```bash
cd /Users/zhangyongchao/knowledge-system/apps/luozi
git status --short
git diff --check
```

进入条件：

- 现有改动已经由所有者提交为可恢复基线；
- `git diff --check` 无输出；
- 不允许自动 `stash`、覆盖、还原或删除现有改动；
- 每份子计划使用独立 `codex/` 工作树；
- 前一计划通过自动检查和对应 Mac 实机验收后，下一计划才能开始。

如果工作树仍然脏，停止执行并请用户确认现有改动归属。

## 计划顺序

| 顺序 | 计划 | 独立交付结果 | 依赖 |
|---:|---|---|---|
| P0 | [基线与质量测量](2026-07-29-p0-baseline-quality-metrics.md) | 可运行的前端测试、阶段计时、质量指标和基准语料 | 已确认代码基线 |
| P1 | [AI 服务中心](2026-07-29-p1-ai-service-center.md) | 页面内 Key、连接测试、教程、能力路由和旧配置迁移 | P0 |
| P2 | [识别质量链路](2026-07-29-p2-recognition-quality-pipeline.md) | 三种输入模式、保护层、个人词典、AI 整理和超时降级 | P1 |
| P3 | [设置与草稿工作台](2026-07-29-p3-settings-draft-workbench.md) | 六分区设置、响应式布局、草稿问题标记和语音修改入口 | P2 |
| P4 | [可配置单键快捷键](2026-07-29-p4-configurable-single-key-shortcuts.md) | 录制、冲突检测、原子切换、单键兼容 Spike 和引导 | P3 |
| P5 | [轻量性能收口](2026-07-29-p5-lightweight-performance-hardening.md) | 懒加载窗口、模型卸载、隐藏态停机和性能验收报告 | P4 |

## 旧计划关系

`2026-07-28-r1-experience-redesign.md` 保留为历史方案，不再直接执行。其视觉 token、HUD 状态和菜单栏心智由 P3/P5 按最新规格吸收；AI 服务、识别策略和快捷键边界以 P1/P2/P4 为准。

## 全局提交原则

- 每个 Task 只提交该 Task 文件。
- 不使用 `git add .`。
- 每次提交前运行该 Task 的聚焦测试。
- 每份计划结束时运行：

```bash
npm run test:frontend
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected：全部成功，`git diff --check` 无输出。

## 完成定义

只有六份计划全部完成，并且最终 Mac 实机覆盖连接服务、短语音、AI 超时、焦点切换、单键冲突和五分钟空闲资源采样，才能宣称总设计完成。任何单份计划完成时只声明对应切片已交付。
