# 工作线登记表（工作线 ↔ 设计稿 ↔ **进度页** ↔ 守卫）

[Document-Meta]
Document: docs/status/workline-registry.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-10-28
Author: lead

> **用途**：`AGENTS.md` §0 要求「开工前先认领工作线」⇒ 本表就是那张**认领清单**。
> ⚠️ **卡点是哪一条**：一条工作线若**只登记了设计稿、没登记进度页**
> ⇒ ⇒ 下一个人（或压缩后的你）**查不到"做到哪了"** ⇒ 一定会重演 2026-10-28 的事故
> （误判已有模块不存在、误判已接生产的能力未接线）。
>
> **新工作线必须加一行**（`AGENTS.md` §0.3）。加行即视为"可接手"。

## 一、登记表

| 工作线 | 设计稿（目标形态） | **进度页（做到哪了）** | 守卫（机器判对错） |
|---|---|---|---|
| **记忆与知识纵深**（D0–D5 / B0–B4） | [memory-knowledge-depth-system-spec.md](../01-architecture/memory-knowledge-depth-system-spec.md) | [spec-batch-status.md](spec-batch-status.md) | `python3 scripts/d4_ledger.py --check`（B0-c 候选池）· `python3 scripts/check_quality_metrics.py`（§6.1 十八项）· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| **护栏接线 / 越权拦截** | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md)（审计即设计依据 + 缺口清单） | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md) §4（缺口清单，逐条标「已修/未修」） | `python3 scripts/check_guard_wiring.py`（台账↔代码互校） |
| **记忆质量路线**（已证/已否的路） | [memory-quality-route-map-2026-10-27.md](memory-quality-route-map-2026-10-27.md) | 同文件（§〇 2026-10-28 更新块） | `python3 scripts/check_golden_baseline.py`（准确度不许降 + 送达率观察项） |
| **状态声明（29 条能力口径）** | [claims.yaml](claims.yaml) | `claims.yaml` 的 `verified_at` / `docs_truth --verify` 输出 | `python3 scripts/docs_truth.py --verify --block-stale` |

## 二、登记规则（四条）

1. **进度页必填**：只写设计稿不写进度页 ⇒ **不算登记**（接手者查不到"做到哪"）；
2. **进度页必须写「还差什么」**：只有"已完成/未完成"两态不够 ⇒ 要写**差的那一项**；
   验收标准里的每一条，进度页要能逐条回答 ⇒ 否则无法判定"算不算完"；
3. **每条可判定的断言都要有守卫**：凡进度页里写了「**PRODUCTION WIRED** / 已落地 / 数字 = N」
   ⇒ 必须有一条**退出码可判**的命令能验它 ⇒ 否则那句话等同散文（见下节）；
4. **守卫失败时改代码或改文档，不许改基线**（棘轮"变好"时的缩表除外）。

## 三、为什么这一列「守卫」是必须的（今天实测的证据）

同一仓库里，**有守卫的文档从没"悄悄漂"过**：

| 文档 | 守卫 | 结果 |
|---|---|---|
| `claims.yaml`（29 条） | `docs_truth --verify --block-stale` | 今天**两次** stale=7 ⇒ **当场被抓、当场修** |
| §6.1 十八项指标 | `check_quality_metrics.py` | 一直 18/18（台账与代码不一致即失败） |
| 架构 / 未接线 / 死代码基线 | `arch_guard` / `audit_unwired` / `check_dead_code` | 一直一致 |
| **设计稿 §8 批次计划**（原状态） | **无** | **漂了整整一天没人知道** |

⇒ ⇒ **结论**：差别不在"会不会漂"，而在"**漂了会不会有人喊**"。
⇒ ⇒ 没有守卫的文档 = 散文；**散文不能当进度用**。

## 四、守卫是什么（给新接手的人）

**文档守卫 = 一个在 CI / 门禁里跑的脚本，把「文档里写的断言」和「另一半真源」比对，
不一致就非 0 退出。**

三类形态（本仓都有实例）：

| 形态 | 判什么 | 本仓实例 |
|---|---|---|
| **一致性** | 文档说的 == 代码/台账里的 | `docs_truth.py --verify`（源码动过而声明没更新 ⇒ STALE）· `check_quality_metrics.py`（台账↔代码互校）· `check_guard_wiring.py` |
| **完整性** | 该有的都得有 | `docs_links.py`（断链）· `docs_index.py`（索引覆盖）· `check_mod_targets.py`（`mod` 声明必须有文件）· `check_doc_paths.py` |
| **棘轮** | 只能变好，不能变差 | `arch_guard.py --strict`（尺寸/依赖边）· `audit_unwired.py --check`（未接线 API 143→142 只许减）· `check_dead_code.py` |

**四条硬要求**：

1. **机器可判定**：断言能算成 true/false（"文档写得好"不可判定 ⇒ 不能当守卫）；
2. **失败要响**：非 0 退出 + 说**哪一条**不一致 + 给**修法**；
3. **有真源**：守卫要有可比对的另一半（代码 / git 历史 / 台账 JSON），不能自说自话；
4. **不许自我豁免**：发现不一致 ⇒ 改代码或改文档，**不是**改基线。

**关键认知**：进度页里写「❌ 没做」反而是**守卫友好**的形态——因为它**可判定**。
守卫真正要防的是：**悄悄变成"做了"（或悄悄退回"没做"）而页面没更新。**
