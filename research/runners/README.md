# runners/

实验运行器入口。

- Rust 独立 bin（`cargo run --bin <name>`，位于主 workspace `crates/` 之外的自建 crate 或 `cargo run --manifest-path` 独立项目）；运行器**不修改产品代码**。
- 每个运行器输出 JSONL 研究日志（schema 见 `logs/README.md`）到 `logs/`。
- 运行器必须可复现：读 `configs/` 配置，记录 seed + config_hash。
- 状态：待实现（Phase 1 起逐步添加）。
