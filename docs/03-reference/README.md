# docs/03-reference — 参考与真源

> **性质**：L1 真源（机器可核验的唯一出处）+ L3 参考（查阅型文档）。
> **纪律**：本目录里的数字与状态是**唯一真源**；其它文档只**引用**不抄写。
> 改动本目录任一文件后必须跑 `python scripts/docs_truth.py --verify --check`（CI 用 `--block-stale`）。

## 真源（唯一出处，改这里）

| 文件 | 它是真源的东西 | 校验方式 |
|---|---|---|
| `test-baseline.md` | **测试数字唯一真源**（命令 / 口径 / 日期 / 工具链） | `docs_truth.py --verify` |
| `capabilities-matrix.md` | 能力矩阵与对外契约、默认开关现值表 | `docs_truth.py --verify` |
| `api.md` | API 参考（真实端点 / 工具协议 / 认证） | 人工 |
| `crates.md` | 当前工作区 crate 索引 | `cargo metadata` |
| `glossary.md` | 术语表（品牌 / 架构 / 记忆 / 安全） | 人工 |

## 参考与手册

| 文件 | 读它做什么 |
|---|---|
| `team-handover-reference.md` | **接手一站式手册**（项目全貌 / LOCKED / 能力 / 命令 / FAQ）——新接手的人先读这篇 |
| `vcp-line-level-absorption-guide.md` | 同类工程源码行级吸收指南（历史研究结论） |
| `absorption-2026-09.md` | 吸收结论归档 |
| `sota-benchmarks-absorption-report.md` | 同业基准对照 |
| `beyond-sota-future-paradigms-whitepaper.md` | 前沿范式白皮书 |
| `vision-alignment-whitepaper.md` | 愿景对齐白皮书 |

## 索引

全仓文档的机器可读清单在 [`../INDEX.csv`](../INDEX.csv)；重定向表在 [`../_redirects.tsv`](../_redirects.tsv)。
