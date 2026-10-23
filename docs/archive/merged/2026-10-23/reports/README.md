# reports/ — 交付记录（12 册）

> **性质**：L5 卷宗 —— 某个时间点做完过什么的**留痕**。
> **不是**：现行口径。册里的数字是**当批实测**（历史口径），**现行数字只认**：
> 测试 → [`../docs/03-reference/test-baseline.md`](../docs/03-reference/test-baseline.md)；
> 能力状态 → [`../docs/status/claims.yaml`](../docs/status/claims.yaml)；
> 默认开关 → [`../docs/03-reference/capabilities-matrix.md`](../docs/03-reference/capabilities-matrix.md)；
> 债 → [`../docs/04-internal/backend-enhancement/00-backlog.md`](../docs/04-internal/backend-enhancement/00-backlog.md)。

---

## 1. 12 册索引

| 册 | 主题 | 篇数 | 一句话定位 |
|---|---|---|---|
| [01](01-role-and-achievement-reports.md) | 角色成就与验收 | 24 | R14/R19 之后各角色（架构 / 全栈 / QA / DevOps / 后端…）逐批交付自述 |
| [02](02-omnibus-and-batch-handoff.md) | 综合批次与交接 | 7 | omnibus 附录与批次交接单 |
| [03](03-audit-and-verification.md) | 审计与验证 | 14 | 代码质量、实现审计、文档活跃度、仓库体检 |
| [04](04-evaluation-and-guard.md) | 评测与守望 | 7 | guard v3 系列评测、trait 验收、真实测量 |
| [05](05-benchmark-and-performance.md) | 基准与性能 | 7 | 基准基线与复现、冷启动剖析、形象性能 |
| [06](06-security-and-sandbox.md) | 安全与沙箱 | 11 | 安全门禁验收、越权缺口修复、威胁模型评审 |
| [07](07-research-assessments.md) | 调研与评估 | 12 | 外部项目逐个评估（是否借鉴、借鉴什么） |
| [08](08-capability-and-architecture.md) | 能力与架构 | 10 | 认知基础设施收口、Memory 2.x 生命周期、能力缺口矩阵、微内核化 |
| [09](09-round-dispatch-decision-ledger.md) | 轮次·派发·决策 | 16 | 派发单与决策链留痕（**过程证据**） |
| [10](10-desktop-and-ui.md) | 桌面与界面 | 3 | 桌面壳骨架、设置实时生效契约、内置能力合成 |
| [11](11-milestone-and-decision-records.md) | 里程碑与决策 | 8 | workspace 真实集成验证与独立复核、SDK 砍填决策、默认落地裁定 |
| [12](12-misc-delivery-notes.md) | 杂项交付记事 | 2 | 许可声明产出、模型实测对比等 |

**合计 121 篇**（2026-10-23 文档治理批：121 篇散篇 → 12 册）。

## 2. 两种合订规格（读之前要知道）

| 册 | 规格 | 怎么读 |
|---|---|---|
| 01–08 | **人工合订**：逐篇写「结论/要点」+ **现状复核（2026-10-23）**（仍成立 / 已过时 / 未核实）+ 归属链接 | 可直接当结论读；「已过时」项已标出现在的口径出处 |
| 09–12 | **机械合订**：脚本摘取标题 / 元信息块 / 各节要点 / 归属路径；**未做逐条现状复核** | 当**索引**读；要判断是否仍成立请自查上方真源 |

> 两种规格都**不替代原件**：121 篇原件**逐字归档**在 `docs/archive/merged/2026-10-23/reports/`，**原路径留 4 行重定向桩**（登记于 [`../docs/_redirects.tsv`](../docs/_redirects.tsv)）。

## 3. 怎么用

| 我要做的事 | 去哪 |
|---|---|
| 查"某个能力当初是怎么做出来的" | 册 08（能力与架构）、册 03（审计） |
| 查"某批交付了什么、当时判据是什么" | 册 01 / 02 |
| 查"性能/基准当时是多少" | 册 05 |
| 查"安全上出过什么问题、怎么收的" | 册 06 |
| 查"我们参考过哪些外部项目、借鉴了什么" | 册 07 |
| 查"这个决策当时为什么这么拍" | 册 11（里程碑与决策）、册 09（决策链） |
| 查**现在**系统是什么 | **不要查这里** → [`../docs/status/`](../docs/status/) 与 [`../docs/03-reference/`](../docs/03-reference/) |

## 4. 目录里的非文档文件（74 个）

`reports/` 下还有 **74 个非 .md 文件**（21 `.ps1` / 14 `.txt` / 13 `.png` / 13 `.log` / 9 `.html` / 8 `.py` / 2 `.err` / 1 `.yml` / 1 `.csv` 等）——它们是**历史交付物**（跑批脚本、截图、日志、审计输出）。

**处置：保留原位，不删除、不迁移。** 依据仓内《证据留存策略》§2：
> 仓库内仍被跟踪的 `artifacts/`、`reports/`、`previews/` 文件是**历史交付物**，不因本策略回溯删除。

因此这 74 个非文档文件**不参与文档索引**（[`docs_index.py`](../scripts/docs_index.py) 只索引 `.md`），也不计入上面的 12 册。

## 5. 两次出库的历史（不要重复做）

| 批次 | 时间 | 做了什么 |
|---|---|---|
| **F-02 大资产出库** | 2026-10-18 | v1.0 时代批量工作日志 + UUID/短哈希工作日志共 **878 件 / 34.7MB** 打包迁 release 资产暂存区 `release-assets/reports-archive-2026-07-08.tar`（`release-assets/` 不入 git 跟踪面）；**git 历史不动**，历史 blob 可按文件名找回。逐件迁移清单（路径 + 字节数 + sha256）：[`f-02-migration-manifest-2026-10-18.tsv`](../docs/04-internal/backend-enhancement/f-02-migration-manifest-2026-10-18.tsv) |
| **文档治理批合订** | 2026-10-23 | 留库的 121 篇 `.md` 散篇合订为 12 册；原件逐字归档 + 原路径留桩 |

> **711 处历史引用不改**——docs/ 与代码注释中出现的报告名属历史卷宗口径，**历史引用名保留**；需要取回时按名从 release 资产包或 git 历史获取。

> 本文件 2026-08 前身的「R12 收尾报告清单」所列 12 份报告，已在 F-02 出库中迁出，故不再列于本索引。

## 6. 纪律

1. **册内数字是历史口径**，引用时标注原篇日期。
2. **原文不改写**——原件逐字归档；册是阅读层，不是原文的替代。
3. **要写"现在是什么"**，写到 `../docs/status/` 或 `../docs/03-reference/`，不要往册里加"当前"结论。
4. 新增交付记录请**先判断题材**归入哪一册；确需新册，先在本表加行并说明与既有册的边界。
