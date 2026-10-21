# 架构守卫集成计划（草案）

> `Status: 📝 v0.2（2026-10-20；Wave 3 收口更新）`
> 原则：**先 report-only，后 blocking；不改现有脚本的默认行为**。本文件是接入补丁说明，不自动改任何现有文件。

## 1. 本地接入（两处，各一行）

### 1.0 已完成（2026-10-20 契约落地批）

- `.gitignore` 已加 `!scripts/arch_guard.py`（跨批常驻工具入库，与 `docs_truth.py`/`verify_changed.py` 同族）；
- 基线已生成：`scripts/arch-baseline.txt`（Wave 3 收口实测 **340 条**：env 153 / edge 111 / size 65 / ext 6 / concept 4 / devcycle 1；初版 2026-10-20 为 356 条）；
- **三处接入已完成（灰度 report-only）**：
  - `scripts/verify_changed.py`：新增 arch 阶段（`--arch-strict` 才拦截；默认仅报告，且对文档批也生效）；
  - `scripts/commit.ps1`：新增 `-ArchStrict` 开关，默认 report-only + 警告；
  - `.github/workflows/arch-guard.yml`：新增 workflow（`continue-on-error: true`，含 baseline 只减检查）；
- 灰度截止：**2026-11-03**（契约 `[meta].rolling_report_only_until`）；之后 CI/commit 转 blocking。

### 1.1 `scripts/verify_changed.py`（fast gate）

在 check 阶段之后、test 阶段之前插入一个 arch 阶段（**默认 report-only，`--strict` 可升 blocking**）：

```python
# 伪代码：在 run_fast_gate 的 check 之后
code = _run([sys.executable, "scripts/arch_guard.py", "--report"], log_path)
# report-only: 不改变 fast gate 退出码；WARN 写日志
```

口径：fast gate 只跑 `--report`（秒级，不动构建锁）；blocking 放在 CI 与 wave 收口。

### 1.2 `scripts/commit.ps1`（提交收口）

在 secret/neutral 扫描之后加：

```powershell
$archExit = & python scripts/arch_guard.py --strict 2>&1 | Tee-Object -FilePath logs/arch-guard.log
if ($LASTEXITCODE -ne 0) { Write-Error "arch_guard 拦截：新增架构违规/例外超期（见 logs/arch-guard.log）"; exit 1 }
```

灰度：前 2 周只 WARN（不 exit），与 DTS `docs_truth.py` 的 P0 report-only → P1 blocking 同节奏。

## 2. CI 接入（已生效，灰度 report-only）

正式 workflow：[`.github/workflows/arch-guard.yml`](../../.github/workflows/arch-guard.yml)（2026-10-20 生效）。
灰度策略：`continue-on-error: true`（report-only）至 2026-11-03；到期后删掉该行并把 `--report` 改 `--strict`。
workflow 内含 **baseline 只减检查**（PR 时对比 base 的 `scripts/arch-baseline.txt`，新增行只警告不阻断）。

## 3. 波次协议

1. 每波开工：`current_wave += 1`（改 `contract.toml`，同批提交）。
2. 每波收口：`python scripts/arch_guard.py --report` → 架构复盘（记分牌）→ 缩基线 → 写一条 wave 记录（wave_note）。
3. 发版冻结：`[exceptions].release_freeze = true`（或独立 freeze 文件），预算归零。
4. 到期例外：`current_wave > expires_wave` → 守卫红 → 要么清除，要么转 ADR 改契约。

## 4. 记分牌（每波对比，只允许一项以上变好、任何一项变差需例外）

| 指标 | 采集 | 当前值（Wave 4 收口，2026-10-21 实测） | 方向 |
|---|---|---|---|
| workspace members | `cargo metadata` | **30**（图像家族仍在主仓；目标 20） | 图像家族出主仓后降至 20 |
| 生产依赖边 | arch_guard | **111** | 只减不增（新增需 ADR） |
| 裸 env 名 / 出现次数 / 文件 | arch_guard + `wiring_report --stats` | guard src 口径 **154**；registry entries **154** | 只减 |
| env 登记 `TODO-OWNER` / `UNCLASSIFIED` | `wiring_report.py --stats`（效率线） | entries **154**；TODO-OWNER **0**；UNCLASSIFIED **146**；kind: product 133 / test_only 11 / derived 10 | 每波只减 |
| 最大 src 文件行数 | arch_guard | **6,150**（doc_ops.rs） | 只减 |
| 同名概念定义数 | arch_guard | ContextAssembler **1**（R7-2.1 收敛）/ ContextBlock **1** / CapabilityRegistry **1** | 只减 |
| 例外活跃数/权重 | exceptions.toml | **2 / 2**（EXC-0002 cognitive 4763、EXC-0003 tool_modules 1216；weight=1、expires_wave=6，清偿任务在收口后执行） | 有上限、到期清 |
| 未接线字样 | `git grep -c '未接线' -- crates/` 求和 | **299**（2026-10-21 口径；旧记分牌 253 为不同采集法，不可直接比较） | 只减 |
| claims 覆盖 | docs_truth | **20 条**（PRODUCTION WIRED 10 / IMPLEMENTED 9 / NOT IMPLEMENTED 1，2026-10-21 实测） | 只增 |
| 生物机制观测格 | self_status_source 台账 | 14 条机制中 **9/14** 观测格注册（R1b-2b 迟绑定 5 + W3.7 生态 1 + 承载力 + 练习等；R1b-2c 未新增观测格） | 只增 |
| 全量门禁耗时 | 日志 | **26.8 分钟**（2026-10-21 `gate_full`，main 264 + image 77 套件；R6-A 唯执行器/锁/失败分类生效；日志 `artifacts/wave4-close-full-2026-10-21.log`） | 只减 |

## 5. 后续（v0.2 路线）

1. `ARCHITECTURE.md` 改为 `contract.toml` 生成物 + `--check` 漂移检测——**R0 已完成（2026-10-20）**：`scripts/arch_doc.py` → [ARCHITECTURE.generated.md](ARCHITECTURE.generated.md)（Wave 3 收口后 30 members / 111 边，CI `--check`）；根 `ARCHITECTURE.md` 的手写依赖表已退役，改为指针 + 人工边界规则；
2. `claims.yaml` 与能力矩阵逐行映射（接线账）；
3. env 读取位置收敛到 config 模块后，按文件卡（`allowed_read_locations`）；
4. 依赖边的"代码级使用"核验（可选：`cargo udeps` 或自研 grep）；
5. 测试/构建成本指标纳入记分牌（fast/full 分层门禁）。
