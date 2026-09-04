# configs/

实验配置。

- 每实验一份配置（TOML/JSON），字段自描述；`config_hash = sha256(规范化配置文本)[0..8]`。
- 配置变更必须改 hash；同 hash 必须同语义（跨机器可复现）。
- seed 单独记录在日志行（不在 config hash 内，见 `logs/README.md`）。
