# Luozi Benchmark Corpus

`corpus.json` 包含 20 条公开中性提示词，每条录制 5 次，共 100 个样本。

真实 WAV 保存到 `tests/benchmarks/private-audio/<prompt-id>/<take>.wav`，不得提交。
五次条件固定为：正常音量、较小音量、较快语速、较慢语速、轻度环境噪音。

每次运行保存无音频、无 Key、无全文日志的结果到
`tests/benchmarks/private-results/<date>-<backend>.jsonl`。
公开报告只允许包含聚合 CER、受保护词错误数、P50/P95 和资源指标。
