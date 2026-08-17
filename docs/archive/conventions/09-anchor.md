# 09 主哲学 8 锚穿透系统 (R125 B5 升 8 锚)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §9 拆出,核验后写。
> **R125 B5 升 8 锚 (2026-08-10 16:55, Mavis 自主, 主人 16:31 最高权限授权)**: 加 S-3 质量工程化 (跟 R123-1 clippy+doc 清关联) + O-1 安全优先 (跟 5 重守门关联). 6→8 锚.

```
[Document-Meta]
Document: docs/conventions/09-anchor.md
Version: Manual-Rev-L + Fix-17 + R125-B5
R-Cycle: R125-B5
Last-Modified: 2026-08-10
Status: 🟢 活跃 (8 锚, R125 末 B5 升)
```

## 8 锚(核验后,严守) (R125 B5 升)

| 锚 | 来源 (主 时间) | 含义 |
|---|---|---|
| **S-1** | 主 22:33 北极星导向 | 服务 ASI 北极星 |
| **S-2** | 主 17:43 实事求是 | 基于现状不重写,核验后写(per R119 主人 8/10 01:14 拍板) |
| **S-3** | 主 16:55 (R123-1) 质量工程化 | 代码质量 = 工程信誉, clippy 150 + doc 1077 清 (per R123-1) + clippy-final FAIL 诚实标 |
| **O-1** | 主 16:55 (R125-5) 安全优先 | 安全 > 功能 > 性能, 5 重守门 v5 + 6 重 v6 (per R125-5 NVIDIA Guardrails) |
| **O-2** | 主 19:33 走在前人经验上 | 借鉴 Hermes / OpenClaw / VCP / claude-mem + LangGraph / AutoGen / MCP / LSP / semver |
| **O-3** | 主 23:44 干到底 | 决策立刻沉淀,1 commit 总(per 主人 8/9 拍板) |
| **O-4** | 主 00:56 任何人都能接手 | 4 件套齐全,顶层瘦(per R119 主人 8/10 拍板) |
| **O-5** | 主 17:58 不假装 | 12 键编译期 hardcode, 8 项不修改承诺形式撤销后原意保留(per R119) |

## 穿透检查清单

每 5 个成就 = 强制穿透检查:

```
每个 PR 必须遵守 6 锚穿透。
```

## 6 锚在 R119 文档重建中的落实

| 锚 | 落实 |
|---|---|
| S-1 | 12 子规范 + 7 子系统 + 21 词条 + 顶层瘦 = 全部服务 ASI 北极星 |
| S-2 | 核验每份子文件 + 实际 (workspace 90+ crate, R 周期 R11-R118, Fix-3..Fix-17, 12+ ADR) |
| O-2 | 借鉴 semver / Linux kernel / Rust crate / VCP / LangGraph / AutoGen / MCP / LSP |
| O-3 | 3-3a 1 commit 总(per 主人 8/9 拍板"1 commit 也行") |
| O-4 | 顶层 README 5 屏 + 6 跳, docs/conventions/README.md 14 文件目录索引 |
| O-5 | 8 项不修改承诺形式撤销,原意保留,实际核验 |

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值


---

## R145 7 新模块锚点映射 (2026-08-12)

> **R145 落实**: 7 新模块按 8 锚分配. 详见 [`docs/architecture-v4-2-r145-modules/README.md`](../architecture-v4-2-r145-modules/README.md).

| 锚 | 承担模块 |
|---|---|
| S-1 北极星 | heartbeat 自演化目的 |
| S-2 实事求是 | arbitration 唯一事实 + search 查得到 |
| S-3 质量工程化 | async_task 状态机 |
| O-1 安全优先 | group_chat 治理 + arbitration append-only |
| O-2 走在前人 | search BM25-lite + emotion PAD |
| O-3 干到底 | heartbeat 主动循环 |
| O-4 接手 | emotion 调温 + 全部模块编译期守门 |
| O-5 不假装 | channel 透明 + arbitration 不假装分布式 |

## 锚点穿透检查 (R145+)

每个新模块必须满足:
- 至少承担 1 锚 (主承担)
- 编译期硬编码守门 (O-4 接手)
- 不假装分布式 / 不假装一致 / 不假装未实现 (O-5 不假装)
- 命名/路径符合 conventions/01-namespace.md + 02-path.md
