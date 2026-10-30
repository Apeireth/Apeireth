# Harness 证据摘要

- 任务：`selftest`
- 代理：`audit-bot`
- 提交：`e7059882e9e895a66e8bafcce4e6d0721b524d72`
- 树：`d0000bade96475479cb94437e364ce9878dd45a1`
- 基线：`4a42d9daba89669b002d47cd99119f8e14a3f0eb`
- 起：`2026-10-30T02:47:06+00:00`　止：`2026-10-30T02:47:59+00:00`
- diff 标签：`rust`
- 变更文件：3 个
- **裁决：FAIL**

## 门禁结果

| 门禁 | 状态 | 必需 | 退出码 | 耗时(s) | 说明 |
|---|---|---|---|---|---|
| `docs-truth` | **PASS** | 是 | 0 | 0.49 | — |
| `doc-class` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-caliber` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-numbers` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `doc-paths` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `docs-links` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `batch-status` | **SKIPPED** | 是 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `claims-verify` | **SKIPPED** | 是 | - | 0.00 | 显式跳过（--skip-gate）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `arch-strict` | **SKIPPED** | 是 | - | 0.00 | 显式跳过（--skip-gate）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `arch-doc` | **PASS** | 是 | 0 | 0.07 | — |
| `crate-budget` | **FAIL** | 是 | 1 | 0.08 | 退出码 1 ≠ 0 |
| `dead-code` | **PASS** | 是 | 0 | 0.20 | — |
| `unwired` | **PASS** | 是 | 0 | 0.53 | — |
| `unwired-groups` | **PASS** | 是 | 0 | 3.92 | — |
| `stale-binaries` | **PASS** | 是 | 0 | 0.61 | — |
| `mod-targets` | **PASS** | 是 | 0 | 0.13 | — |
| `guard-wiring` | **PASS** | 是 | 0 | 9.69 | — |
| `quality-metrics` | **PASS** | 是 | 0 | 0.03 | — |
| `golden-baseline` | **SKIPPED** | 是 | - | 0.00 | 显式跳过（--skip-gate）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `golden-no-leak` | **PASS** | 是 | 0 | 0.09 | — |
| `internal-golden` | **PASS** | 是 | 0 | 0.10 | — |
| `d4-ledger` | **PASS** | 是 | 0 | 36.51 | — |
| `wiring-report` | **SKIPPED** | 是 | - | 0.00 | 显式跳过（--skip-gate）　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `verify-ledger` | **SKIPPED** | 是 | - | 0.00 | scope ['docs', 'claims'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `gate-manifest` | **SKIPPED** | 是 | - | 0.00 | scope ['docs', 'ci', 'contract'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `self-status-vocab` | **SKIPPED** | 否 | - | 0.00 | scope ['docs'] 与 diff 标签 ['rust'] 不相交　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-rust` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=cargo）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-nextest` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=cargo）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-clippy` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=cargo）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-fmt` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=cargo）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-doc-test` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=cargo）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-cargo-deny` | **SKIPPED** | 是 | - | 0.00 | CI 专属门禁（env=ci）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |
| `ci-promise-audit` | **SKIPPED** | 否 | - | 0.00 | CI 专属门禁（env=ci）：本地不执行，须由 GitHub Actions 跑　⚠ SKIPPED ≠ PASS（未跑过，不算通过） |

## 裁决依据

- 门禁 crate-budget FAIL（退出码 1）
- required gate not applicable/skipped without justification: doc-class
- required gate not applicable/skipped without justification: doc-caliber
- required gate not applicable/skipped without justification: doc-numbers
- required gate not applicable/skipped without justification: doc-paths
- required gate not applicable/skipped without justification: docs-links
- required gate not applicable/skipped without justification: batch-status
- required gate not applicable/skipped without justification: claims-verify
- required gate not applicable/skipped without justification: arch-strict
- required gate not applicable/skipped without justification: golden-baseline
- required gate not applicable/skipped without justification: wiring-report
- required gate not applicable/skipped without justification: verify-ledger
- required gate not applicable/skipped without justification: gate-manifest
- required gate not applicable/skipped without justification: ci-rust
- required gate not applicable/skipped without justification: ci-nextest
- required gate not applicable/skipped without justification: ci-clippy
- required gate not applicable/skipped without justification: ci-fmt
- required gate not applicable/skipped without justification: ci-doc-test
- required gate not applicable/skipped without justification: ci-cargo-deny

## 失败门禁输出尾部

### crate-budget（退出码 1）

```
FAIL: crate 体量预算（1 处）
  · crates/engine/memory: 77874→78066 行 · 174→174 文件（**超基线**）

修法：① 把增长**说清并接受** ⇒ 跑 `--gen` 重写基线（**要在提交信息里说明为什么**）；② 或者把代码减回去。
⚠️ 注意：本守卫**故意**比 `arch_guard` 粗（按 crate 总量）⇒ 它防的是「拆文件规避尺寸棘轮」，不是精确定位。
```

---

> 口径：**SKIPPED 不等于 PASS**；**UNVERIFIED 不能进入合并**；
> 只有 `PASS`（或已批准的 `WAIVED`）才允许合并。
