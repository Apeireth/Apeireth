# ROADMAP — Apeireth

> **R119-2 Mavis 重写 (2026-08-10)**: 顶层 ROADMAP 从 13KB 缩到 ~3KB。完整路线图下沉到 `docs/roadmap/`。

```
[Document-Meta]
Document: ROADMAP.md
Version: 1.1.0-R114
R-Cycle: R119-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 时间线

| 日期 | 事件 | 详情 |
|---|---|---|
| 2026-07-30 | R11 baseline LOCKED | V0.5 24 维 / V1136 R-Measure / 6 哲学锚 |
| 2026-07-31 | R14 Rust 重写启动 | 9 LOCKED 主文档 / 17 crate 推演 |
| 2026-08-04 | R17 战役 0-4 收官 | 1.0 release / 11 子文档 / 4 LOCKED 哲学层 |
| 2026-08-05 | R20 阶段 1-6 1.0 release | 12 项 checklist 100% PASS, 14 new crate |
| 2026-08-09 | R38 1.1 RC | telemetry 4→1 + provider 5→1 真合并, 4148 tests |
| 2026-08-09 | R46-R62 1.1.1 + 1.1.2 patch | mini-redis / cognition_graph / cargo audit |
| 2026-08-09 | R63-R72 1.2 candidate + LIVE | LIVE MiniMax 7 model 100% pass, MCP subscribe push |
| 2026-08-10 | R78-R113 1.2 patch LIVE 续 | 11 R + 1 LIVE: skills / graph / MCP 真接 |
| 2026-08-10 | R114-R118 动态运营层 | Eval/Council MCP + CLI + TUI cognition live + Protocol bridges, 4921 tests |
| 2026-08-10 | R119 文档重建 | 顶层 README/CHANGELOG/ROADMAP 瘦身 + docs/ 子目录重组 |
| 2026-08-10 | R119-1 ~ R119-5 收尾 | 10 commit: hygiene + 顶层瘦身 + 3 规范下沉 + OMNIBUS 拆 + construction+final-check+release 索引 + 7 子目录 README + 根目录 100+ 临时文件 + src-tauri 6.8GB + target 277GB 清 |

## 未来路线(思想层)

| 主题 | 状态 | 详情 |
|---|---|---|
| Tauri 集成(终极前端) | 等设计团队 | 主人 2026-08-04 23:33 拍板, TUI 是"集成测试床" |
| 商业化 / 真用户 | 等 | 主人 2026-08-05 "现在根本没用户用" |
| vector store long_term 真接 | 1.3 路线 | 当前 total/5 heuristic, apeireth-vector 还在 skeleton |
| TUI 9 organ UI 完整 | 需 UI 放行 | R78 cognition summary 已接, 其他 8 organ 待办 |
| backend cognition_summary per-chat-cycle | 估补 | 当前仅 snapshot_organ_main 触发 |

## 详细路线图

跳 [`docs/roadmap/`](docs/roadmap/):
- [`docs/roadmap/README.md`](docs/roadmap/README.md) — 路线图总览
- [`docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md`](docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md) — 1.0 release 9-30 tag 计划
- [`docs/roadmap/v1.2-release-plan-2026-08-09.md`](docs/roadmap/v1.2-release-plan-2026-08-09.md) — R69 1.2 release plan

## 思想层保留(哲学 LOCKED)

| 主题 | 来源 | 状态 |
|---|---|---|
| 立体架构 v2 | R11 / R14 | 🔒 LOCKED |
| 生命架构 v4 | R11 / R14 | 🔒 LOCKED |
| 哲学层升级 v4.1 | R11 / R14 | 🔒 LOCKED |
| 6 哲学锚 | S-1 / S-2 / O-2 / O-3 / O-4 / O-5 | 🔒 LOCKED |
| 12 键编译期 hardcode | 哲学守门 | 🔒 LOCKED |
| 5 重守门 + 双洋葱 | 架构层 | 🔒 LOCKED |

详见 [`docs/v2-strategy/00-VISION.md`](docs/v2-strategy/00-VISION.md) + [`docs/conventions/09-anchor.md`](docs/conventions/09-anchor.md)。

---

_本 ROADMAP 由 Mavis R119-2 重写,原 13KB 详单下沉到 `docs/roadmap/`。思想层 (v2 / v4 / v4.1 / 6 锚) 严守,技术发展史按主人 2026-08-10 原则(思想历史 + 最新)筛选保留。_
