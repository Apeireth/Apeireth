# Agent R123-1 Readmap — clippy 150 + doc 1077 L1 速赢 (2026-08-10)

**时间**: 2026-08-10 15:45 启动
**角色**: 团队成员 R123-1 (Mavis 派, 维护战术, 接手 R122-6 真实标数)
**任务**: 0 业务影响 warning 清零 L1 速赢
**预算**: 1h40m (15:45 → 17:25, 留 5 min 给最终报告)

---

## §0. TL;DR

承接 R122-6 (15:00 commit) 真实标数: clippy 150 + doc 1077 + 1 doc error (serde_yaml, R122-5 战区不动)。本任务只清 L1 速赢:

| # | 类别 | baseline | L1 目标 | 范围 |
|---|---|---|---|---|
| 1 | clippy | 150 | < 30 | unused/cast/dead_code/conversion/let_else |
| 2 | doc | 1077 | < 200 | unclosed HTML / broken link / URL not hyperlink |
| 3 | missing_docs (clippy 1280+ + doc 498) | 1778+ | **0 必修** | 标 L2 文档债, R124 续 |
| 4 | deprecation (proc-macro-error2) | 1 | 0 触碰 | 第三方 dep, 等上游 |
| 5 | doc error (serde_yaml) | 1 | 0 触碰 | R122-5 战区, R122 续 TODO L0 |

**0 改 workspace.version (1.1.0), 0 触碰 24 LOCKED, 0 改 11 agent 公共 API 签名, 0 装 (O-5), 0 主动 commit**

---

## §1. 项目状态 (15:45 当前)

**git 状态**: clean (除 2 个未追踪 worktree 引用, 跟 R122-6 一样)

**项目结构**: 92 crate, workspace.version 1.1.0 (Cargo.toml:246, R122-6 严守 0 改)

**R122-6 baseline (15:00 commit 后真实标数)**:
- clippy 150 warnings (`reports/agent-r122-6-clippy.log`, 147KB, 1220 行)
- doc 1077 warnings + 1 error (`reports/agent-r122-6-doc.log`, 178KB, 4732 行)
- 0 dbg! 跟 0 todo!() (`reports/agent-r122-6-debug-scan.log`, 185 行)

---

## §2. 任务范围

### §2.1 clippy L1 (90% of 150)
**优先级**:
1. `unused_import` / `unused_variables` — 加 `_` 前缀 / 删 1 行
2. `dead_code` — 加 `#[allow(dead_code)]` 或真删
3. `useless_conversion` — 删 1 行
4. `cast_*_can_be_expressed_infallibly` — 改 `From` trait, 0 行为改
5. `clippy::needless_*` / `let...else` / `wildcard_enum` / `stripping prefix manually` — 删冗余

**0 改**:
- 公共 API 签名
- 业务逻辑
- 0 触碰 24 LOCKED
- 0 触碰 9 器官 logic

### §2.2 doc L1 (60% of 1077)
**优先级**:
1. `unclosed HTML tag` 37 — `dyn` / `String` / `T` / `Agent` — 在 `#[derive]` / `///` 上下文加 `` ` ``
2. `broken link` 5 — 删 `[]` 空链
3. `this URL is not a hyperlink` 5 — 加 `<...>` 包裹
4. `missing_docs` 498 — **0 必修**, 标 L2 文档债

**0 改**:
- 公共 API 签名
- 业务逻辑

### §2.3 0 假装 (O-5)
- missing_docs 498: 0 假装"已修", 标 L2 R124 续
- future_incompat 1 (proc-macro-error2 v2.0.1): 0 假装"已修", 标 L3 等上游
- doc error 1 (apeireth-pipeline serde_yaml): 0 触碰, 标 L0 R122-5 修

---

## §3. 8 墙硬约束 (严守)

| # | 约束 | 0 触碰验证 |
|---|---|---|
| 1 | 0 改 workspace.version (Cargo.toml:246 = 1.1.0) | 0 改 Cargo.toml |
| 2 | 0 改 R11 baseline (0.8682/0.8532/0.9063) | 0 改 tests/integration_r_measure.rs |
| 3 | 0 触碰 24 LOCKED crate mtime | 0 改 cognition/core/sovereignty/formal/asi 等 LOCKED |
| 4 | 0 触碰 9 器官 logic | 0 改 hand.rs 等 |
| 5 | 0 触碰 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 | 0 触碰 |
| 6 | 0 改 11 agent 公共 API 签名 | 0 改 public surface |
| 7 | 0 主动 commit | 0 commit |
| 8 | 0 装 (O-5) | 真实标数, 不假装 |

---

## §4. 报告路径

- readmap: `reports/agent-r123-1-readmap-2026-08-10.md` (本文件)
- final: `reports/agent-r123-1-final-2026-08-10.md`
- decision log: `reports/agent-r123-1-decision-log-2026-08-10.md`
- L1 速赢 log (before/after): `reports/agent-r123-1-cleanup-log-2026-08-10.md`

---

**R123-1 readmap 完. 开始 baseline + L1 速赢.**
