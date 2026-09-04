# JSONL 研究日志 Schema（logs/）

> 依据 `00-master-plan.md` §5 B1：`seed + config hash + 事件流`。

## 文件命名

`logs/<experiment>-<confighash>.jsonl`，每行一条事件，UTF-8，append-only。

## 每行 Schema（必填 + 可选）

```json
{
  "ts": "2026-09-04T10:00:00.000+08:00",   // 必填 ISO-8601 事件时间
  "experiment": "p1-forget-closure",        // 必填 实验名（kebab-case）
  "seed": 42,                               // 必填 随机种子
  "config_hash": "a1b2c3d4",                // 必填 配置哈希（见 configs/README.md 约定）
  "event": "probe.recall_direct",           // 必填 事件名（点分层命名）
  "payload": { },                           // 必填 事件载荷（见下）
  "version": 1                              // 可选 schema 版本，缺省 1
}
```

## 事件名规范（点分层）

| 前缀 | 含义 |
|---|---|
| `retrieval.*` | 检索实验（`retrieval.query` / `retrieval.hit` / `retrieval.metric`） |
| `retention.*` | 记忆保留实验 |
| `forget.*` | 遗忘传播实验（`forget.root` / `forget.closure` / `forget.audit`） |
| `compress.*` | 上下文压缩实验 |
| `calibration.*` | 校准实验（`calibration.prediction` / `calibration.outcome`） |
| `approval.*` | 审批状态机实验 |
| `tool.*` | 工具执行事件（`tool.invoke` / `tool.result`） |
| `meta.*` | 元事件（`meta.run_start` / `meta.run_end` / `meta.crash`） |

## 载荷最小约定

- `meta.run_start`：`{ "commit": "<full sha>", "machine": "...", "cargo_version": "..." }`
- `meta.run_end`：`{ "exit": 0|1, "elapsed_ms": 1234 }`
- 其他事件：载荷自描述，字段名 snake_case，禁止把实验结论写进日志（日志是证据不是结论）。

## 消费

- 读取端用 seed + config_hash 复现环境；事件流可重建实验过程。
- 同一 config_hash 的两次运行必须可对比（差异只允许出现在 seed 与时间戳）。
