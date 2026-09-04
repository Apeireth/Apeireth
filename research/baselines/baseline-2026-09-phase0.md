# Phase 0 基线冻结（2026-09-04）

## 冻结信息

| 项 | 值 |
|---|---|
| commit | `ede73515cab5c4b2bc5dd4fc03ada7e97de35fc5` |
| commit 时间 | 2026-09-04 17:11 +0800 |
| 测试命令 | `cargo test --workspace`（含 doc-tests） |
| 日志 | `.harness-phase0-test.log` |

## 测试基线（重核结果）

| 项 | 值 |
|---|---|
| 套件数（test result 行） | 106 |
| **passed** | **3061** |
| **failed** | **0** |
| ignored | 13 |

## 旧口径矛盾的解释

- 口述"2012 项"：无对应日志，来源不明，作废。
- 旧日志 `.harness-final4-test.log` 的"1739 passed / 97 crate"：只统计了 lib 测试子集（未含 doc-tests 与部分集成目标）。
- **正确口径 = 3061 passed（含 doc-tests）**。后续所有实验引用本文件，不再引用 2012 / 1739。

## 默认行为闸门

Phase 0 只新增 `research/` 工作区（无 Cargo.toml，不进 workspace 构建），主产品代码零改动——等价性门与性能门天然满足（见 `research/README.md`）。
