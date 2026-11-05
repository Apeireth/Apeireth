# 验证架构（三层门禁语义）

> `Status: ⚠️ 部分交付（2026-10-21，R6-A/B；R6-C/D 未交付）`
> 依据：[ADR-0011](../adr/0011-r6-verification-architecture.md) D1（三层语义）、D2（full 唯一执行器）、D3（数字单一真源）。
> 目的：一个改动该跑哪一层门禁、谁阻断、数字从哪来，只有一种解释。

## 1. 三层语义（何时跑什么）

| 层 | 执行器 | 范围 | 何时 | 阻断语义 | 产出的证据 |
|---|---|---|---|---|---|
| **fast** | `python scripts/verify_changed.py` | 改动 crate + 传递反向依赖（check / clippy `-D warnings` / nextest） | 每批提交前（AI 高频迭代） | 阻断本地提交（退出码非 0） | `logs/verify-changed.log`（不入库）；**不产生对外数字** |
| **arch** | `python scripts/arch_guard.py`、`arch_doc.py`、`docs_truth.py`、`wiring_report.py` | 依赖边 / 层 / env / 概念 / 尺寸 / 例外、文档真源、wiring 生成物 | 提交前与 PR | **blocking**（灰度期已于 2026-11-04 到期升格）：CI `arch-guard.yml` = `--strict`、CI `wiring-ledger.yml` = R3c `--crosscheck --strict`，均**无** `continue-on-error`；本地 `commit.ps1` 默认 `--strict` 阻断（显式 `-Report` 才降级为仅告警）。**fast gate 的 arch 段仍 report-only**（ADR-0001 §2.3 / ADR-0011 的设计，非漏改） | 守卫输出、`scripts/arch-baseline.txt`、`docs/status/wiring.yaml` |
| **full** | `pwsh scripts/gate_full.ps1`（唯一执行器） | 主 workspace 三件套（test/clippy/fmt，`-j 2`）+ 图像家族独立门禁 | wave 收口 / 夜间 / 发版前；**不随每个 PR 全跑** | 阻断收口/发版（退出码非 0） | `logs/gate-full.log`（不入库）；运行时通过数写入 [test-baseline.md](../03-reference/test-baseline.md) |

口径：

- **full 的唯一性是排他的**：不接受第三条命令自跑全量。`gate_full.ps1` 用进程级锁 `logs/gate-full.lock` 保证同一时刻只有一个全量执行；并发调用按 `-FailIfBusy` 直接退出码 2，或按 `-LockTimeoutMinutes`（默认 90）等待后退出码 2。并行 AI 线共享 `target/` 时，先拿锁再跑，杜绝「两线同时全量 → 互等文件锁 / OOM」。
- **数字只有一种来源**：对外引用测试通过数 = full 层 `cargo test --workspace --all-targets --locked --no-fail-fast` 的实际绿数，且以 test-baseline 为准（单一真源）。fast/arch 层不发布数字。
- **失败分类（归因，不改变结论）**：full 失败时输出 `FAILURE_CLASS=oom|env|flaky|regression`；分类只用于记录与复盘，**失败仍然是失败（exit 1）**。分类证据规则见 `gate_full.ps1` 的 `Get-FailureClass`；对既有日志可 `pwsh scripts/gate_full.ps1 -ClassifyLog <log>` 单独判类。
- **arch 门禁等级（2026-11-05 实测）**：灰度期（`continue-on-error: true` + report-only）已于 **2026-11-04** 到期结束 —— `arch-guard.yml` 跑 `arch_guard --strict`、`wiring-ledger.yml` 跑 `wiring_report --crosscheck --strict`，**均无** `continue-on-error` ⇒ blocking；本地 `commit.ps1` **默认 `--strict` 阻断**（显式 `-Report` 降级为仅告警，`-ArchStrict` 为历史兼容 no-op）。
  **仍为 report-only 的一层（不是漏改）**：`verify_changed.py` 的 arch 段（fast gate）**永远 report-only**，仅 `--arch-strict` 才升格拦截 —— ADR-0001 §2.3 / ADR-0011 的明确设计（快反馈、秒级、不动构建锁），blocking 落在 commit 与 CI 两处；`arch-guard.yml` 内「Baseline ratchet」步骤亦为 advisory（ADR-0001 §2.2 棘轮只减不增）。
  〔历史口径（2026-10-21，R6-A/B）：当时两条 workflow 为 `continue-on-error: true`，R6-A/B 只落地语义与 CI 口径、不改灰度到期日〕
  日历注：`2026-11-03` 是契约 `[meta].rolling_report_only_until` 的**约定到期日**（该键即日历真源、无脚本消费），实际升格发生在 **2026-11-04**。

## 2. full 层用法

```powershell
# 完整门禁（会先拿 logs/gate-full.lock）
pwsh scripts/gate_full.ps1

# 只看将执行的命令（不执行、不加锁）
pwsh scripts/gate_full.ps1 -DryRun

# 并行线：拿不到锁立即失败（适合 CI/自动化）
pwsh scripts/gate_full.ps1 -FailIfBusy

# 指定等锁上限（分钟，默认 90）
pwsh scripts/gate_full.ps1 -LockTimeoutMinutes 30

# 只跑主 workspace / 只跑图像家族
pwsh scripts/gate_full.ps1 -MainOnly
pwsh scripts/gate_full.ps1 -ImageOnly

# 对既有日志判失败类（不执行门禁）
pwsh scripts/gate_full.ps1 -ClassifyLog logs/gate-full.log
```

退出码：`0` 通过；`1` 门禁失败（日志尾部有 `FAILURE_CLASS=`）；`2` 锁被占用/等锁超时（未运行门禁，不代表红绿）。

图像家族清单唯一来源 = [契约](../architecture/contract.toml) `[vendored].image_family`，脚本通过 `python scripts/arch_guard.py --print-vendored` 读取；不在脚本里手抄清单。

## 3. CI 与本地门禁的对应

| CI workflow | 对应层 | 现状 |
|---|---|---|
| `rust.yml` → `rust-tests`（3 OS matrix，nextest） | full（PR 触发，非 nightly 专属） | 现行；R6-D 夜间化载体已落（`nightly-full.yml`），PR 层降级待排期 |
| `rust.yml` → `hard-walls`（守门 1–6） | arch（契约口径） | 守门 1/3/4 已于 R6-B 改为读契约真源（LOCKED / R11 / 14 键）；旧「24 LOCKED / 13 键」字面量 0 残留 |
| `arch-guard.yml`、`wiring-ledger.yml`、`docs-truth.yml`、`doc-caliber.yml` | arch | arch-guard / wiring-ledger 已 **blocking**（2026-11-04 升格：`--strict` / R3c `--crosscheck --strict`，无 `continue-on-error`）；docs-truth / doc-caliber 保持 blocking |
| `kani.yml`（22 harness 矩阵 + 汇总门）、`miri.yml` | 形式化面 | 保持 blocking（无 `continue-on-error`）；覆盖边界待 R6-C `formal-coverage.md` 登记 |
| `image-family.yml`、`gate_full.ps1 -ImageOnly` | full（图像家族） | 已交付（R4a-1） |

## 4. 未交付（如实）+ 门禁等级补记

| 项 | 状态 |
|---|---|
| R6-D：夜间 full 调度 + 成本分层（PR 只跑 fast/affected） | 载体已落（`.github/workflows/nightly-full.yml`，2026-10-21）；**首跑阻塞**：远端默认分支 `main=fb68f5d3` 落后本地 194 提交、默认分支无该 workflow，dispatch API 404 → 首跑 0 次、可追溯性 NOT MET；**2026-10-21 owner 裁决「不用推送，都本地就行」→ 按 NOT MET 收口**，本地全量实跑替代证据 `logs/gate-full.log`（341/8570/0/51，E1 复跑），不计夜间 CI 可追溯；PR 层降级未做 |
| R6-C：四层验收载体 `docs/status/verification-ledger.yaml`、形式化边界 `docs/status/formal-coverage.md` | 载体已落（5 条单层条目、四层齐全 0、signoff 0，2026-10-21）；只读校验器 `scripts/verify_ledger.py` 已落，`docs-truth.yml` 已接门禁步骤（2026-10-21 E4）；owner 签核仍待 → 验收 NOT MET |
| arch-guard / wiring-ledger 转 blocking（原口径：计划 2026-11-03 后由架构线复核执行） | **已交付（2026-11-04 到期执行）** —— CI 两处升格 blocking（`arch_guard --strict` / R3c `--crosscheck --strict`，均无 `continue-on-error`）；本地 `commit.ps1` 默认 `--strict` 阻断（显式 `-Report` 才降级为仅告警）。**fast gate 的 arch 段按 ADR-0001 §2.3 / ADR-0011 设计仍 report-only**（非漏改）。详见 §1 与 §3 |

四层验收制（形式验证 / 基准复现 / 场景实测 / 主人签核）的定义以 [founding-design-v2 §7](../01-architecture/founding-design-v2.md) 为准；R6-C 交付前，「已验收」只能按四级诚实口径标注。
