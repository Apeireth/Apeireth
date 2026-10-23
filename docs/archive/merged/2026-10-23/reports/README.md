# reports/ — 报告库（F-02 出库后口径）

> **F-02 大资产出库执行批（2026-10-18）· 批量日志迁出说明**
> v1.0 时代批量工作日志 + UUID/短哈希工作日志共 **878 件 / 34.7MB** 已打包迁 release 资产暂存区
> `release-assets/reports-archive-2026-07-08.tar`（随 Release 资产分发，`release-assets/` 不入 git 跟踪面）；
> **git 历史不动**，历史 blob 可按文件名找回。
> **711 处 docs 引用不改**——docs/ 与代码注释中出现的报告名属历史卷宗口径，**历史引用名保留**，
> 迁出不改写引用；需要取回时按名从 release 资产包或 git 历史获取。
> 逐件迁移清单（路径 + 字节数 + sha256）：
> [../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv](../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv)。

## 本目录现状（2026-10-18 起）

- **已迁出（878 件）**：文件名带 2026-07/08 日期的 v1.0 时代批量日志（`agent-*`、`decision-*`、
  `rNN-*`、`round*`、`1.0-release-*`、`achievement-*`、`dispatch-*` 等）+ UUID/短哈希命名工作日志。
- **留库（195 件 tracked）**：近期证据/验收报告（2026-09~10）、`benchmark-baseline.md` /
  `benchmark-reproduction.md`、`diagnostics-review/`、`pet-real-smoke-2026-10-11/`、`audit-2026-08-19/`、
  `tp30/`、`r18-snapshots/`、`evidence-V10-qa2-A11-A20/` 等证据件，及无日期的评审/验收/设计裁定文档。
- 目录名与报告名在历史文档中的引用一律保留原值（历史不追溯改写纪律）。

---

## reports/ — R12 收尾报告（历史口径，R12 收尾时点登记）

> **范围**: R12 接手期间的所有报告 (12 份)
> **创建时间**: R12 收尾任务 T16 (technical_writer) + R12 评审
> **当前**: 12 个报告 (~283KB)

---

## 📚 报告清单

### 收尾总结 (2)

| 报告 | 大小 | 任务 | 内容 |
|------|------|------|------|
| `r12-finalize-2026-07-30.md` | 21.6KB | T16 | R12 收尾总结 |
| `r12-future-changes-2026-07-30.md` | 26.2KB | T14 | R12 重大变更预备 |

### MVP 启动 (1)

| 报告 | 大小 | 任务 | 内容 |
|------|------|------|------|
| `r13-mvp-kickoff-2026-07-30.md` | 9.5KB | T9 | R13 MVP Phase 0 + 1.1 启动 |

### 附录 N 评审 (5)

| 报告 | 大小 | 任务 | 内容 |
|------|------|------|------|
| `apeireth-omnibus-appendix-n-r12-handoff-arch-check.md` | 32.0KB | T4-M3 | 架构评审 |
| `apeireth-omnibus-appendix-n-r12-handoff-sec-check.md` | 10.7KB | T4-M2.5-SEC | 安全评审 |
| `apeireth-omnibus-appendix-n-r12-handoff-perf-check.md` | 15.2KB | T4-M2.5-PERF | 性能评审 |
| `apeireth-omnibus-appendix-n-r12-handoff-fe-check.md` | 24.6KB | T4-M2.5-FE | 全栈评审 |
| `apeireth-omnibus-appendix-n-r12-handoff-peer-review.md` | 30.1KB | T4-M2 | 同行评审 |

### 收尾接续 (4)

| 报告 | 大小 | 任务 | 内容 |
|------|------|------|------|
| `r12-baseline-verification-2026-07-30.md` | 25.8KB | T1 | §5.B 6/6 PASS |
| `r12-working-changes-audit-2026-07-30.md` | 24.5KB | T2 | working changes 35 files +1759/-316 |
| `r12-t6-commit-audit-v2-2026-07-30.md` | 32.6KB | T7 | T6-A/B/C 接续审计 v2 |
| `r12-sec-cross-validation-2026-07-30.md` | 30.6KB | T5 | SEC P0 交叉验证 |

---

## 用途

1. **接手者** (R13 / R14 团队) — 历史回溯
2. **R14 Rust 重写** — `r12-future-changes-2026-07-30.md` 是 8 类大变动预备
3. **R13 MVP 接力** — `r13-mvp-kickoff-2026-07-30.md` 继续

## 维护原则

- ❌ 不修改 (R12 已归档)
- ✅ 可以 git commit 新增报告

---

_主哲学 anchor: 主 00:56 任何人都能接手 + 主 17:43 实事求是._