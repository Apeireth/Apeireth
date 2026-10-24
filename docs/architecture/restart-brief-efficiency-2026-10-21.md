# 效率线（第三线）重启简报 — 2026-10-21

> 用途：owner 重启第三线后，本文件是**唯一开工入口**。读它与
> [line-coordination.md](line-coordination.md) 即可接续；仓库真源优先于本简报。
> 签发：架构线（lead），2026-10-21。状态：Wave 4 已收口（`5f6ee7da`，`current_wave=5`）。

## 0. 当前仓库基线（开工前先复核，命令附后）

| 项 | 现值 | 复核命令 |
|---|---|---|
| HEAD | `5f6ee7da`（Wave 4 收口批） | `git log --oneline -3` |
| 全量基线 | **341 套件 / 8570 passed / 0 failed / 51 ignored**（2026-10-21；main 264/6263 + image 77/2307） | `docs/03-reference/test-baseline.md` §1 |
| 全量日志 | `artifacts/wave4-close-full-2026-10-21.log`、`logs/gate-full.log`（26.8 min，exit 0，HEAD `f2dad300`） | `Select-String logs/gate-full.log 'GATE FULL'` |
| 契约守卫 | `wave=5 / crates=30 / edges=111 / env=154 / exceptions=2 / VIOLATIONS 0 / STALE 0` | `python scripts/arch_guard.py --report` |
| 真源门禁 | `docs_truth --verify --check` / `arch_doc --check` / `wiring_report --verify` 均绿 | 三条命令 |
| 灰度 | arch/wiring 守卫 report-only **至 2026-11-03**；R3c 转 blocking 由架构线 review | `contract.toml rolling_report_only_until` |

## 1. 你的资产（勿被他人代改，改动走你线提交）

`scripts/verify_changed.py`、`scripts/commit.ps1`、`scripts/docs_truth.py`、`scripts/wiring_report.py`、
`docs/status/{claims.yaml,STATUS.md,wiring.yaml,CLOSEOUT-2026-10-20.md}`、`.github/workflows/{docs-truth,wiring-ledger}.yml`、
`docs/02-guides/fast-feedback-gate.md`、`docs/03-reference/test-baseline.md`（维护权本轮起正式回归你线）。

## 2. 本轮任务（按序，1 commit = 1 任务）

### E1. 正式接管 test-baseline 维护权 + 全量执行权（第一件）
- 背景：Wave 4 收口时全量数字由架构线用 R6-A `gate_full.ps1` **代跑**并广播，`test-baseline.md`
  维护权仍归你线（§1 来源栏已写明代跑事实）。重启后由你线**接管执行与刷新**。
- 动作：
  1. 复核 §1 数字与 `artifacts/wave4-close-full-2026-10-21.log` 一致（现 341/8570/0/51）；
  2. 以后每波收口用 `pwsh scripts/gate_full.ps1 -FailIfBusy`（R6-A 唯一执行器：进程锁 + 失败分类），
     **不再自拼 cargo test 命令**；数字变化同时刷新 §1 与 README 徽章（`python scripts/docs_truth.py --gen-badges`）；
  3. 在 `test-baseline.md` 来源栏把「代跑广播」改为「由全量门禁执行线执行（接管日期 2026-10-21）」。
- 验收：`docs_truth --verify --check` exit 0；徽章与 §1 同日同口径；`logs/gate-full.lock` 无残留。

### E2. R8-D 余项：治理面进 wiring 台账（只登记已交付面，不冒充）
- 现状：R8-A `fd74a669`（GovernanceChain 六段骨架，**生产零调用**）、R8-B1 `b6d902ce`
  （`ChainGovernanceHook` 适配器，**零生产接线**）、R8-C `da7e224f`（延迟反思队列，默认零变化）、
  R8-D 部分 `60627d8e`（演练测试入 CI）。
- 动作：用 `wiring_report.py --gen` 在 `docs/status/wiring.yaml` 登记治理面条目，level 按实况：
  骨架/适配器 = **IMPLEMENTED**（不是 PRODUCTION WIRED）；演练入 CI = 证据字段；同步任何受影响的
  claims `production_refs`；不得为提高覆盖率把未接线写成 PRODUCTION WIRED。
- 验收：`wiring_report --verify` exit 0；变更 diff 仅生成物 + 必要 claims；提交正文披露「R8-B2/B3 未落地，
  故治理面仍未进生产调用链」。

### E3. R6-D 夜间 full 首跑（成本分层真正生效）
- 载体已存在：`.github/workflows/nightly-full.yml`（cron UTC 18:00 / 北京时间 02:00 + `workflow_dispatch`
  `main_only`；当前**从未运行过 NOT RUN**）。
- 动作：以 `workflow_dispatch`（`main_only=true`）触发首跑；下载 `logs/gate-full.log` artifact；结果回填：
  `docs/02-guides/verification-architecture.md` §4 的「R6-D NOT IMPLEMENTED」改实况；
  ADR-0011 §6 R6-D 勾选仅在**首跑绿**时；失败按 `FAILURE_CLASS` 归因并如实登记，不静默重试。
- 验收：CI run URL/结论入文档；ADR-0011 §6/§9 追加记录；若失败，写明失败类与归因（不得写成通过）。

### E4. R6-C 校验器接门禁（签核仍归 owner）
- 载体：`scripts/verify_ledger.py`（架构线 `391cc8ec` 交付，只读校验 `docs/status/verification-ledger.yaml`）；
  当前未接入 docs_truth/CI，属你线资产范围。
- 动作：把校验器接入 `docs_truth --verify` 或 CI（二选一，最小实现）；**不代签** signoff（`signoff` 字段
  只能 owner 填真实签名/日期）。
- 验收：故意改坏一行 ledger 能被门禁捕获（负向测试记录）；正常时 exit 0。

### E5. 边界（硬约束）
- 不改 `crates/**`（生物线/安全线在途文件一律不碰；当前生物线有 `runtime-assembly/src/canonical/nervous_snapshot.rs` 在途）。
- arch/wiring 守卫灰度期不得提前转 blocking；R3c 转 blocking 等架构线 2026-11-03 后 review。
- 显式路径 stage，禁止 `git add -A`；提交走 `commit.ps1`；数字来源只认 test-baseline。

## 3. 立即复核命令

```powershell
cd C:\Users\USER_A\Apeireth-rust
git log --oneline -3
python scripts/arch_guard.py --report
python scripts/docs_truth.py --verify --check
python scripts/wiring_report.py --verify
Get-Content ../03-reference/test-baseline.md -TotalCount 20
```

## 4. 与架构线的接口
- 契约/守卫/基线/ADR：架构线维护；你线需要改 `arch_guard.py`/`arch-baseline.txt` 时先找架构线。
- 全量数字：你线执行、你线刷新（E1）；架构线只引用。
- 灰度转 blocking 的 review 由架构线在 2026-11-03 后做，结果会写进 handbook 与 contract。
