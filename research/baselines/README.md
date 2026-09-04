# baselines/

冻结基线记录。每个 baseline 文件 = 一个不可变的性能/测试口径快照。

- 命名：`baseline-<date>-<phase>.md`
- 内容必须含：commit、测试数、硬件/负载描述、指标快照（引用 `metrics/README.md` 定义）。
- 实验报告只允许与 baseline 文件做对比，不允许"凭记忆"对比。
