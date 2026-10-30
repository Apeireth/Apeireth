# Harness 证据摘要

- 任务：`selftest`
- 代理：`audit-bot`
- 提交：`4a42d9daba89669b002d47cd99119f8e14a3f0eb`
- 树：`9c20cf2941f2ecc28d0ec7647e49c0b2ee4eef0b`
- 基线：`caf499b769cd1d3aa0a6281c77acc5e1ca5c6ae7`
- 起：`2026-10-30T02:43:49+00:00`　止：`2026-10-30T02:43:51+00:00`
- diff 标签：`ci, contract, docs`
- 变更文件：6 个
- **裁决：BLOCKED**

## 门禁结果

| 门禁 | 状态 | 必需 | 退出码 | 耗时(s) | 说明 |
|---|---|---|---|---|---|
| `docs-truth` | **PASS** | 是 | 0 | 0.54 | — |
| `doc-class` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-caliber` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-numbers` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-paths` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `docs-links` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `batch-status` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `claims-verify` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `arch-strict` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `arch-doc` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `crate-budget` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `dead-code` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `unwired` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `unwired-groups` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `stale-binaries` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `mod-targets` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `guard-wiring` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `quality-metrics` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `golden-baseline` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `golden-no-leak` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `internal-golden` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `d4-ledger` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `wiring-report` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `verify-ledger` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `gate-manifest` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `self-status-vocab` | **SKIPPED** | 否 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-rust` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-nextest` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-clippy` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-fmt` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-doc-test` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-cargo-deny` | **SKIPPED** | 是 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-promise-audit` | **SKIPPED** | 否 | - | 0.00 | not selected by --only（本次未选中）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |

## 裁决依据

- required gate not applicable/skipped without justification: doc-class
- required gate not applicable/skipped without justification: doc-caliber
- required gate not applicable/skipped without justification: doc-numbers
- required gate not applicable/skipped without justification: doc-paths
- required gate not applicable/skipped without justification: docs-links
- required gate not applicable/skipped without justification: batch-status
- required gate not applicable/skipped without justification: claims-verify
- required gate not applicable/skipped without justification: arch-strict
- required gate not applicable/skipped without justification: arch-doc
- required gate not applicable/skipped without justification: crate-budget
- required gate not applicable/skipped without justification: dead-code
- required gate not applicable/skipped without justification: unwired
- required gate not applicable/skipped without justification: unwired-groups
- required gate not applicable/skipped without justification: stale-binaries
- required gate not applicable/skipped without justification: mod-targets
- required gate not applicable/skipped without justification: guard-wiring
- required gate not applicable/skipped without justification: quality-metrics
- required gate not applicable/skipped without justification: golden-baseline
- required gate not applicable/skipped without justification: golden-no-leak
- required gate not applicable/skipped without justification: internal-golden
- required gate not applicable/skipped without justification: d4-ledger
- required gate not applicable/skipped without justification: wiring-report
- required gate not applicable/skipped without justification: verify-ledger
- required gate not applicable/skipped without justification: gate-manifest
- required gate not applicable/skipped without justification: ci-rust
- required gate not applicable/skipped without justification: ci-nextest
- required gate not applicable/skipped without justification: ci-clippy
- required gate not applicable/skipped without justification: ci-fmt
- required gate not applicable/skipped without justification: ci-doc-test
- required gate not applicable/skipped without justification: ci-cargo-deny

---

> 口径：**SKIPPED 不等于 PASS**；**UNVERIFIED 不能进入合并**；
> 只有 `PASS`（或已批准的 `WAIVED`）才允许合并。
