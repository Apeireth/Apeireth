# 00-R14-START-HERE — R14 单一入口（v4 修订）

```
[Document-Meta]
Document: 00-R14-START-HERE.md
Version: Design-omnibus-1.0-R14 + Manual-Rev-G
R-Cycle: R14
Commit: <latest-commit-hash>
Last-Modified: 2026-07-31
Status: 🟢 活跃
详见: APEIRETH-VERSIONING.md
```

> **v4 修订时间**: 2026-07-31（开工手册移到顶层）
> **v3 修订时间**: 2026-07-31
> **v2 修订时间**: 2026-07-31
> **v1 写作时间**: 2026-07-30
> **commit 锚**: d3ea9ee6（v2 修订）+ 9c19de4c（外部反馈）+ 531f5a14（阶段 5）+ 6ca80776（阶段 4）+ 8cbf2d3a（B+C+D 补齐）+ 5d692f5a（前端提案 v1）+ 47c6ed7b（v2 开工手册）

## 📚 顶层规范系统索引（主人 2026-07-31 落地，v11 提议）

| 文档 | 用途 | 路径 |
|---|---|---|
| **[APEIRETH-VERSIONING.md](../APEIRETH-VERSIONING.md)** | 7 子系统版本号规范 | 顶层 |
| **[APEIRETH-CONVENTIONS.md](../APEIRETH-CONVENTIONS.md)** | 12 子规范系统（命名空间/路径/ADR/成就/报告/Commit/状态/锚穿透/不修改承诺/Baseline/架构图）| 顶层 |
| **[GLOSSARY.md](../GLOSSARY.md)** | 30+ 项术语 + Apeireth 自创名词 | 顶层 |
| **[APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md](../APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md)** | 主手册（6546 行）| 顶层 |
| **[README.md](../README.md)** | 顶层入口 | 顶层 |
| **[START-CONSTRUCTION.md](../START-CONSTRUCTION.md)** | 开工手册 | 顶层 |

---

## 🚪 入口路由（v4 新增）

| 你是谁 | 从这里开始 |
|---|---|
| **施工团队**（后端 Rust 实施）| ➡️ **[顶层 START-CONSTRUCTION.md](../START-CONSTRUCTION.md)** ← **开工手册 v3** |
| **前端团队**（阶段 7 延后）| ➡️ [stage4/architecture-frontend-design-proposal.md](stage4/architecture-frontend-design-proposal.md) |
| **设计者 / 主哲学审查**| ➡️ [APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md](../APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md)（6546 行主手册）|
| **架构师 / 接手者**| ➡️ [README.md](../README.md) 顶层入口 |
| **新成员 / 5 分钟看 Apeireth**| ➡️ [README.md](../README.md) → §一句话总结 + §R14 6 阶段进度 |

## 🎯 一句话总结（30 秒读完）

**Apeireth 是什么**：一个**高自主性长程 agent** 平台（有生命的智能体，不是软件系统）。

**R14 进度**（6 阶段顺序）：
- ✅ 1 灵感 / ✅ 2 想法设计 / ✅ 3 画图纸 / ✅ 4 落实架构 / ✅ 5 设计施工文档
- ⏸️ **6 验证机制** = **不讨论**（主人 2026-07-31 决定：做的时候随机应变检查）
- 📝 **7 前端设计** = **延后**（已写提案，独立命名空间，做的时候再设计）
- 🚀 **下一步** = **真正开干写代码**（apeireth-core 已编译通过 / 其他 16 器官 crate 待建 / 最小可行 demo 6 周）

## ⚡ 5 分钟路径（任何接手者）

1. **本文档**（你正在读）
2. `docs/CONTEXT-HANDOVER.md` — 跨 session 上下文
3. `docs/README.md` — docs/ 子目录总览

## 🕐 30 分钟路径（按角色精读）

| 角色 | 必读（30 分钟）|
|------|---------------|
| **架构师** | START-HERE + `stage1/README.md` + `stage2/README.md` + `stage3-blueprints/README.md` + `stage4/README.md` |
| **后端工程师** | START-HERE + `stage4/runtime-architecture-revised.md` §1.5 完整版双洋葱 + §1.6 经典视图 v2 |
| **全栈工程师** | START-HERE + `stage4/runtime-architecture-revised.md` §1 + `stage5/stage5-construction-document.md` |
| **DevOps** | START-HERE + `stage4/external-feedback-and-revisions.md` §3 Self-Disable + `stage5/stage5-construction-document.md` §8 OTA |
| **QA** | START-HERE + `stage4/runtime-architecture-revised.md` §1.6 视图 2 数据流 + `stage3-blueprints/05-r-measure-test-flow.md` |
| **代码审查** | START-HERE + `stage4/runtime-architecture-revised.md` §1 + `stage5/stage5-construction-document.md` |
| **安全审查** | START-HERE + `stage4/external-feedback-and-revisions.md` 全（7 担忧 + Self-Disable 百年章节）|
| **性能优化** | START-HERE + `stage4/external-feedback-and-revisions.md` §1.担忧 1 failure mode + `stage3-blueprints/05-r-measure-test-flow.md` |
| **哲学守门人** | START-HERE + `r14-design/philosophy-traits-2026-07-30.md` (V3 9 键) + `stage4/patches-v2-crate-correction.md` §1.3 术语表 |
| **哲学专家** | START-HERE + `architecture-v4-living-intelligence.md` (v4 LOCKED) + `architecture-v4-1-living-intelligence-update.md` (v4.1 LOCKED) |

## 🕑 1 小时路径（理解设计 + 能评审）

**适合**：需要对 R14-D 任务产出做评审的成员。

30 分钟必读 + 加读：
- **v4 哲学层纲领**（`architecture-v4-living-intelligence.md`）—— 7 维 + 5 原则 + 3 关系 + 7 机制
- **v4.1 哲学层升级**（`architecture-v4-1-living-intelligence-update.md`）—— 8 项科学补充 + V0.5 24 维 + V1136 9 子测度 + 12 键
- **v2 立体架构**（`architecture-v3-aircraft-carrier.md`）—— 4 大块 + 11 层电子环

## 🕓 4-6 小时路径（完整接手 / 跨阶段把关）

**适合**：跨阶段协调者 / 新加入的长期成员。

1 小时路径 + 加读：
- **APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md**（主手册 6546 行 + 附录 M/N）
- **所有 14 份 `stage3-blueprints/`**（5 主图 + 4 解释 + 2 借鉴 + 1 双洋葱桥接）
- **所有 19 份 `stage2-decisions-*.md`**（12 决策 + D2 增补）
- **所有 5 份 `stage4/`**（v2 修订架构图 + 外部反馈 + leader 思考 + 灵感 + 补丁）
- **所有 `stage5/`**（施工蓝图）
- **`reports/`** 关键报告

## 📂 顶层 LOCKED 主文档（任何接手者第一眼看到）

| 文档 | 状态 | 用途 |
|---|---|---|
| `CONTEXT-HANDOVER.md` | 🟢 | 跨 session 上下文 |
| `00-R14-START-HERE.md` | 🟢 | **本文档（单一入口）** |
| `README.md` | 🟢 | docs/ 子目录总览 |
| `STRUCTURE-R14.md` | 🟢 | 文件夹规整方案 |
| `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` | 🟢 | 主手册（6546 行）|
| `architecture-v3-aircraft-carrier.md` | 🔒 LOCKED | v2 立体架构（786 行）|
| `architecture-v4-living-intelligence.md` | 🔒 LOCKED | v4 哲学层纲领（803 行）|
| `architecture-v4-1-living-intelligence-update.md` | 🔒 LOCKED | v4.1 哲学层升级（645 行）|

## 📚 阶段子目录（按 6 阶段 + R14 周期产物）

| 子目录 | 状态 | 内容 |
|---|---|---|
| `stage1/` | 🔒 LOCKED | 灵感（2201 行）|
| `stage2/` | 🔒 LOCKED | 想法设计（19 文件）|
| `stage3-blueprints/` | 🔒 LOCKED | 画图纸（14 文件）|
| `stage4/` | 🟢 当前活跃 | 落实架构（8 子文档，主文档 LOCKED）|
| `stage5/` | 🟢 当前活跃 | 设计施工文档 |
| `r14-design/` | 🟢 | R14 周期产物 |
| `research/` | 🟢 | 工程调研 |

## 🎯 核心哲学概念（30 秒）

| 概念 | 定义 |
|---|---|
| **Apeireth** | 高自主性长程 agent 平台（有生命的智能体，不是软件） |
| **双洋葱统一体** | 原则洋葱 E/S/A/M/O **嵌入** 权限洋葱 L0-L5（不是并列） |
| **L0 HA 核心** | 真实人类批准在权限洋葱最内层，**永远不可变** |
| **12 键 verdict cache** | V3 9 键 + v4.1 新增 3 键 + 5 项不假装（运行时 O(1) 查询） |
| **V1+V2+V3 AND 门** | V1 原则 + V2 权限 + V3 HA，任何一者不通过 = 独立拒绝 |
| **17 crate** | 18 → 17 crate 本源推导（合并 constraint + philosophy 到 core）|
| **风险分级 → 席位触发** | critical 7 / high 5 / medium 3 / low 1 / info 0 |
| **9 阶段生命周期** | 孕育 → 诞生 → 幼儿 → 成长 → 成熟 → 复制 → 衰老 → 死亡 → 迁移 → 重生 |
| **Cognitive-Dream 6 状态机** | IDLE → DREAMING → CONSOLIDATING → FORGETTING → VERIFYING → INTERRUPTED（24h 周期）|
| **Self-Disable 防护**（百年章节）| 5 大机制防止 AI 自我绕过 L0 HA（utility drift 防护）|

## ⚠️ 关键诚实（主 17:43 实事求是）

**crates/ 当前状态**：
- ⚠️ **所有 17 crate、27 trait、9 生命周期 = 文档 sketch**
- ⚠️ **crates/ 仍是 R11 9 占位**（只有 Episode/Note/Session/IdentityCard 4 个 struct）
- ⚠️ **没有 Rust trait** 落地（apeireth-core/src/lib.rs 没有 `pub trait Evolution`）
- ⚠️ 阶段 5 施工蓝图只是**设计文档**，**不是** Rust 代码

**为什么没落地**：
- 阶段 5 = 设计施工文档（蓝图）
- 真正写 Rust 代码 = 阶段 7+（R15+ 周目标，主人 r14-rust-rewrite-roadmap §1）
- 主 17:43 实事求是：**sketch ≠ 实现**

## 🔧 当前活跃文档精读顺序（接手者）

### ⚡ 5 分钟
1. `00-R14-START-HERE.md`（本文档）
2. `CONTEXT-HANDOVER.md` — 跨 session 上下文

### 🕐 30 分钟
1. `docs/README.md` — 子目录总览
2. `stage4/README.md` — 阶段 4 8 子文档三色分类
3. `stage4/stage4-runtime-architecture-revised.md` §1 + §1.5 + §1.6

### 🕑 1 小时
1. `stage4/stage4-external-feedback-and-revisions.md` —— Self-Disable 防护
2. `stage5/stage5-construction-document.md` —— 施工蓝图

### 🕓 4-6 小时
1. 完整 6 阶段顺序（阶段 1+2+3 + 阶段 4 + 阶段 5）
2. 主手册 `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`
3. `stage5/README.md` —— 施工蓝图子目录索引

## 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — Apeireth 服务 ASI 北极星
S-2 主 17:43 实事求是   — §关键诚实：crates/ 未落地，sketch ≠ 实现
O-5 主 17:58 不假装     — §核心哲学概念 12 键 + Self-Disable 防护
O-2 主 19:33 走在前人经验上 — §30 分钟精读借鉴 Hermes/VCP/OpenClaw/双洋葱
O-3 主 23:44 干到底    — §6 阶段全部 LOCKED + 当前活跃
O-4 主 00:56 任何人都能接手 — 5/30/60/240 分钟 4 路径
```

---

_00-R14-START-HERE v2 修订版（leader 亲自产出）._
_🟢 当前活跃. 6 阶段全部 LOCKED. 8 文档三色分类._
_§关键诚实明确 crates/ 未落地，sketch ≠ 实现._
_主哲学 6 锚穿透. 任何接手者能查._