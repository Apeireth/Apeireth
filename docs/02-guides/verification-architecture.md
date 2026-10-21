# 验证架构（三层门禁语义）

> `Status: ⚠️ 部分交付（2026-10-21，R6-A/B；R6-C/D 未交付）`
> 依据：[ADR-0011](../adr/0011-r6-verification-architecture.md) D1（三层语义）、D2（full 唯一执行器）、D3（数字单一真源）。
> 目的：一个改动该跑哪一层门禁、谁阻断、数字从哪来，只有一种解释。

## 1. 三层语义（何时跑什么）

| 层 | 执行器 | 范围 | 何时 | 阻断语义 | 产出的证据 |
|---|---|---|---|---|---|
| **fast** | `python scripts/verify_changed.py` | 改动 crate + 传递反向依赖（check / clippy `-D warnings` / nextest） | 每批提交前（AI 高频迭代） | 阻断本地提交（退出码非 0） | `logs/verify-changed.log`（不入库）；**不产生对外数字** |
| **arch** | `python scripts/arch_guard.py`、`arch_doc.py`、`docs_truth.py`、`wiring_report.py` | 依赖边 / 层 / env / 概念 / 尺寸 / 例外、文档真源、wiring 生成物 | 提交前与 PR | 灰度期 report-only，**2026-11-03 后转 blocking**（ADR-0001/0004） | 守卫输出、`scripts/arch-baseline.txt`、`docs/status/wiring.yaml` |
| **full** | `pwsh scripts/gate_full.ps1`（唯一执行器） | 主 workspace 三件套（test/clippy/fmt，`-j 2`）+ 图像家族独立门禁 | wave 收口 / 夜间 / 发版前；**不随每个 PR 全跑** | 阻断收口/发版（退出码非 0） | `logs/gate-full.log`（不入库）；运行时通过数写入 [test-baseline.md](../03-reference/test-baseline.md) |

口径：

- **full 的唯一性是排他的**：不接受第三条命令自跑全量。`gate_full.ps1` 用进程级锁 `logs/gate-full.lock` 保证同一时刻只有一个全量执行；并发调用按 `-FailIfBusy` 直接退出码 2，或按 `-LockTimeoutMinutes`（默认 90）等待后退出码 2。并行 AI 线共享 `target/` 时，先拿锁再跑，杜绝「两线同时全量 → 互等文件锁 / OOM」。
- **数字只有一种来源**：对外引用测试通过数 = full 层 `cargo test --workspace --all-targets --locked --no-fail-fast` 的实际绿数，且以 test-baseline 为准（单一真源）。fast/arch 层不发布数字。
- **失败分类（归因，不改变结论）**：full 失败时输出 `FAILURE_CLASS=oom|env|flaky|regression`；分类只用于记录与复盘，**失败仍然是失败（exit 1）**。分类证据规则见 `gate_full.ps1` 的 `Get-FailureClass`；对既有日志可 `pwsh scripts/gate_full.ps1 -ClassifyLog <log>` 单独判类。
- **arch 灰度说明（如实）**：`arch-guard.yml`、`wiring-ledger.yml` 当前 `continue-on-error: true`，report-only 至 2026-11-03；到期由架构线复核后转 blocking。R6-A/B 只落地语义与 CI 口径，不改灰度到期日。

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
| `rust.yml` → `rust-tests`（3 OS matrix，nextest） | full（PR 触发，非 nightly 专属） | 现行；R6-C 将评估夜间化与成本分层 |
| `rust.yml` → `hard-walls`（守门 1–6） | arch（契约口径） | 守门 1/3/4 已于 R6-B 改为读契约真源（LOCKED / R11 / 14 键）；旧「24 LOCKED / 13 键」字面量 0 残留 |
| `arch-guard.yml`、`wiring-ledger.yml`、`docs-truth.yml`、`doc-caliber.yml` | arch | 灰度 report-only 至 2026-11-03（arch-guard / wiring-ledger） |
| `kani.yml`（22 harness 矩阵 + 汇总门）、`miri.yml` | 形式化面 | 保持 blocking（无 `continue-on-error`）；覆盖边界待 R6-D `formal-coverage.md` 登记 |
| `image-family.yml`、`gate_full.ps1 -ImageOnly` | full（图像家族） | 已交付（R4a-1） |

## 4. 未交付（如实）

| 项 | 状态 |
|---|---|
| R6-C：夜间 full 调度 + 成本分层（PR 只跑 fast/affected） | NOT IMPLEMENTED（效率线实施，需 owner 拍板夜间频率/载体） |
| R6-D：四层验收载体 `docs/status/verification-ledger.yaml`、形式化边界 `docs/status/formal-coverage.md` | NOT IMPLEMENTED（效率线实施） |
| arch-guard / wiring-ledger 转 blocking | 计划 2026-11-03 后由架构线复核执行 |

四层验收制（形式验证 / 基准复现 / 场景实测 / 主人签核）的定义以 [founding-design-v2 §7](../01-architecture/founding-design-v2.md) 为准；R6-D 交付前，「已验收」只能按四级诚实口径标注。
