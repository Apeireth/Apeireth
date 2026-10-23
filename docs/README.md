# Apeireth 文档体系

> **文档体系 2.0（2026-10-23 文档治理批）** — 取代 1.1（2026-08-26 封板）。
> **一句话**：本体系按 **5 层寿命模型** 组织，按 **12 类题材** 标注；**每个问题有唯一入口**，**每个数字有唯一真源**。

---

## 0. 我要什么 → 去哪

| 我要做的事 | 唯一入口 | 别去哪 |
|---|---|---|
| **接手干活** | [03-reference/team-handover-reference.md](03-reference/team-handover-reference.md)《接手一站式手册》 | 不要逐篇读 `04-internal/` |
| **搞懂系统是什么** | [01-architecture/](01-architecture/README.md) | 不要从 `reports/` 倒推 |
| **查测试数字** | [03-reference/test-baseline.md](03-reference/test-baseline.md) | 任何别的文档都只**引用**它，不抄写 |
| **查能力/默认开关** | [status/claims.yaml](status/claims.yaml) + [03-reference/capabilities-matrix.md](03-reference/capabilities-matrix.md) | 别拿散文当能力声明（口径只认 claims.yaml 的四级标注） |
| **查现在的债/待办** | [04-internal/backend-enhancement/00-backlog.md](04-internal/backend-enhancement/00-backlog.md)（D 编号唯一真源） | 不要翻历史 backlog |
| **查某次决策为什么** | [adr/](adr/README.md) | ADR 正文**只增不改** |
| **查机器可执行的契约** | [architecture/contract.toml](architecture/contract.toml) | 散文里的"约束"不是契约 |
| **查某批交付了什么** | [reports](../reports/README.md))（12 册） | 不要在 `reports/` 里翻 121 篇散篇 |
| **查我们审过什么** | [04-internal/audits/](04-internal/audits/README.md) | — |
| **查历史考古** | [04-internal/history/](04-internal/history/) + [archive/](archive/README.md) | — |
| **查未接线能力** | [status/unwired-ledger.md](status/unwired-ledger.md)（W1–W7） | — |

**全量机器可读清单**：[`INDEX.csv`](INDEX.csv)（1000+ 行，含层/题材/日期/被引用数/处置）。
**重定向表**：[`_redirects.tsv`](_redirects.tsv)（旧路径 → 新路径）。

---

## 1. 5 层寿命模型

文档的**层**决定它能漂移多久、能不能改、要不要归档。这是本体系的骨架。

| 层 | 含义 | 改写规则 | 位置 |
|---|---|---|---|
| **L1 真源** | 描述"现在是什么"，随代码变，**须可机校** | 由生成器/校验器维护，改了就跑 `--verify` | `03-reference/`、`status/` |
| **L2 决策** | 一次拍板，永久 | **只增不改**（被取代就写新 ADR） | `adr/`、`architecture/` |
| **L3 常青** | 长期有效的"是什么/怎么做" | 可改，但引用要标来源 | `01-architecture/`、`02-guides/`、`design/`、`development/` |
| **L4 在飞** | 有明确终点的施工面 | 施工线自己维护 | `04-internal/` |
| **L5 卷宗** | 过去是什么样 | **只引用，不改写** | `archive/`、`04-internal/history/`、`reports/` |

**12 类题材**（正交标签，记在 `INDEX.csv` 的 `topic` 列）：1 决策 / 2 契约与纪律 / 3 现状真源 / 4 设计与规格 / 5 手册与操作 / 6 研究与调研 / 7 审计与核对 / 8 交接与接手 / 9 交付与验收 / **10 经验（正向可复用）** / **11 教训（反向不可重复）** / 12 历史与卷宗。

> 经验与教训是**唯一跨层的一族** —— 它们不是目录，而是一条**提炼流水线**：
> `L4 在飞 → L5 卷宗 → 提炼成可复用条目（engineering-experience/）→ 门禁检查同类坑二次出现时是否被引用`。
> 容器：[04-internal/engineering-experience/](04-internal/engineering-experience/)（`01-collaboration-patterns` = 经验，`02-failure-anatomy` = 教训）。

---

## 2. 目录地图

```
docs/
├── 03-reference/    L1+L3  真源与参考（测试基线/能力矩阵/接手手册/术语）
├── status/          L1     文档事实体系的机器真源（claims/wiring/verification-ledger/未接线台账）
├── adr/             L2     决策记录（只增不改，README 索引完整覆盖 16 篇）
├── architecture/    L2     可执行契约（contract.toml / env-registry / 异常表 / 线路协调）
├── 01-architecture/ L3     架构说明（哲学/愿景/架构/安全/能力规范）
├── 02-guides/       L3     使用与操作指南（按场景分组）
├── design/          L3     设计面（设计系统/形象/关系/交接）
├── development/     L3     开发流程（目录布局/加模块/RFC/社区运营）
├── packaging/  security/  integration/  beta/  rc/  vision/   L3  专项
├── 04-internal/     L4     在飞工作面
│   ├── backend-enhancement/   施工线台账（轴 A–P / 债栏 / 缺口账 / 交付册）
│   ├── audits/                审计与核对册（4 册）
│   ├── history/               历史卷宗（阶段施工记录 / 考古卷宗）
│   ├── engineering-experience/ 经验与教训
│   ├── philosophy-study/      哲学研究
│   └── license-port/          许可搬运四关
├── archive/         L5     历史归档（只读）
└── ../reports/      L5     交付记录 12 册（仓根，非 docs/ 下）
```

**每个目录都有 `README.md` 索引**，写清三件事：**这是什么 / 先读哪篇 / 什么不是**。

---

## 3. 常见入口索引

| 文档 | 说明 |
|---|---|
| [01-architecture/philosophy.md](01-architecture/philosophy.md) | **9 哲学锚 / 14 键 / 三洋葱**（LOCKED 本体，0 触碰） |
| [01-architecture/vision.md](01-architecture/vision.md) | 愿景与产品北极星 |
| [01-architecture/architecture.md](01-architecture/architecture.md) | 架构总览 |
| [01-architecture/system-capabilities.md](01-architecture/system-capabilities.md) | 能力规范 + **四级口径定义** |
| [03-reference/test-baseline.md](03-reference/test-baseline.md) | **测试数字唯一真源** |
| [03-reference/capabilities-matrix.md](03-reference/capabilities-matrix.md) | 能力矩阵 + 默认开关现值表 |
| [03-reference/api.md](03-reference/api.md) | API 参考（真实端点/工具协议/认证） |
| [03-reference/team-handover-reference.md](03-reference/team-handover-reference.md) | **接手一站式手册** |
| [04-internal/ENGINEER-MANIFESTO.md](04-internal/ENGINEER-MANIFESTO.md) | **工程总纲**（0 装诚实 / O-6 三阶审查） |
| [04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md](04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) | **并发施工宪章**（多 AI 同仓纪律） |
| [04-internal/backend-enhancement/00-backlog.md](04-internal/backend-enhancement/00-backlog.md) | **债栏唯一真源**（D 编号） |
| [04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md](04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md) | 系统级缺口台账（SG/NT/NF） |
| [04-internal/handover-ledger.md](04-internal/handover-ledger.md) | 批次交接台账（10 篇按批次交接稿的索引） |
| [02-guides/quick-start.md](02-guides/quick-start.md) | 快速开始 |
| [02-guides/user-manual.md](02-guides/user-manual.md) | 用户手册 |
| [02-guides/permission-surfaces.md](02-guides/permission-surfaces.md) | 权限面边界 |
| [02-guides/deployment.md](02-guides/deployment.md) | 部署 |
| [design/frontend-handoff.md](design/frontend-handoff.md) | 前端交接（接手前端先读） |
| [development/repository-layout.md](development/repository-layout.md) | 目录布局与 ownership |
| [packaging/release-asset-verification.md](packaging/release-asset-verification.md) | 发布资产校验与签名 |
| [security/process-executor-threat-model.md](security/process-executor-threat-model.md) | 进程执行器威胁模型 |

---

## 4. 文档纪律（写文档前必读）

1. **唯一真源**：数字只写在真源里，其它地方**引用**。测试数字 → `test-baseline.md`；能力状态 → `claims.yaml`；默认开关 → `capabilities-matrix.md`；债 → `00-backlog.md`。
2. **历史不追溯改写**：带日期的历史记录保留原值，需要时加「历史口径（日期）」标注。**归档只移动，不改写正文**。
3. **0 装诚实**：文档里说"已实现/已接线"，必须按**四级口径**（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED / HARDWARE VALIDATED）标注落在哪一级。默认关闭的能力不得写成系统现状。
4. **合并留桩**：合并旧文档时，原件进 `archive/`，**原路径留 4 行重定向桩**并登记 `_redirects.tsv` —— 保证既有链接（含仓外引用）不 404。
5. **中性表述**：代码/注释/commit 零第三方产品名、零"移植/借鉴"措辞（真实路径与研究引注走豁免表）。
6. **数字必须可复核**：写数字必带命令 + 日期 + 工具链；**无脚本不写数字**。

## 5. 文档门禁

| 门禁 | 命令 | 管什么 |
|---|---|---|
| 链接完整性 | `python scripts/docs_links.py --check` | 断链 = 0（archive 内部历史链接不算） |
| 索引分层 | `python scripts/docs_index.py --check` | 每目录有 README、INDEX 全覆盖、层/题材合法、重定向表自洽 |
| 文档真源 | `python scripts/docs_truth.py --verify --check` | claims 证据 / staleness / 生成物漂移 |
| 架构契约 | `python scripts/arch_guard.py --strict` | 层/边/预算/例外 |
| 口径体检 | `python scripts/check_doc_caliber.py --changed` | 四级口径标注 |

## 6. 变更历史

| 版本 | 日期 | 变更 |
|---|---|---|
| 2.0 | 2026-10-23 | 文档治理批：5 层寿命模型 + 12 类题材；补 15 个目录索引；引入 `INDEX.csv` / `_redirects.tsv` / 两条门禁；合并 4 篇接手稿为 1 册；reports/ 121 散篇合为 12 册；审计册 4 册；历史卷宗 2 册 |
| 1.1 | 2026-08-26 | 结构分层，历史归档，与当前根工作区对齐 |
