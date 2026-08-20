# 17 重守门 (历史 v7 嵌套视角, **实际 v9 lineage 2026-08-19 验证**)

> **2026-08-19 修正 (主人 2026-08-19 拍板: 末尾加 §v7→v9 漂移, 0 重写历史 §7)**: 本文件 §7 守门列表按 v7 写 (R125-5 + R126-guard-7), 但实际代码已升到 **v9** (R127-2 P6-3 加 action_rail/flow_executor → R131 加 evidence_guard). 详见末尾 §v7→v9 视角漂移 + 实际 9-fold lineage. **0 假装 §7 是当前真相**.

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"4 重守门(v5 修正)" 拆出。
> **R125 B4 升 v6 (2026-08-10 16:55, Mavis 自主, 主人 16:31 最高权限授权)**: 5 重 → 6 重, 加 Colang DSL 守门 (R125-5 NVIDIA Guardrails 借鉴触发). v5 (4 重 + 权限发放) 实质保留.
> **R126 P1-3 retry 升 v7 (2026-08-10 21:11, bg_b4c7a22f)**: 6 重 → 7 重, 加 Superpowers Skill Guard (R125-14/15/16/18/19/126-guard-7, superpowers 借鉴触发). v6 (5 嵌套 + Colang DSL) 实质保留.
> **R127-2 P6-3 升 v8 (2026-08-10 22:xx)**: 7 重 → 8 重, 加 action_rail + flow_executor (Guardrails ActionDispatcher 借鉴触发).
> **R131 升 v9 (2026-08-xx)**: 8 重 → 9 重, 加 evidence_guard. 当前 master HEAD `9bf36b1e` 实际是 9 重 v9.

```
[Document-Meta]
Document: docs/glossary/17-4-gates-permission.md
Version: Manual-Rev-L + Fix-17 + R125-B4 + R126-P1-3-retry + R127-2-P6-3 + R131 (历史 §7 仅至 v7, 实际 v9 lineage 见末尾 §)
R-Cycle: R125-B4 (升 v6) → R126-P1-3-retry (升 v7) → R127-2-P6-3 (升 v8) → R131 (升 v9, 主人已知 §7 已过时)
Last-Modified: 2026-08-19 (v7→v9 漂移诚实标注 + 末尾 §v7→v9 lineage 段, 0 重写历史 §7)
Status: 🟡 历史 v7 视角 (§7 内容) + 🟢 v9 lineage 实际 (末尾 §v7→v9). 历史保留 per R119 "形式撤销原意保留" 纪律
```

## 定义 (v7 嵌套视角, **历史 §7**; 实际 v9 见末尾)

主人 8/10 16:31 最高权限授权 + 8/10 16:55 R125-5 NVIDIA Guardrails 借鉴 + 8/10 21:11 R126 P1-3 retry Superpowers Skill 借鉴, **历史 v7 嵌套视角 7 重守门** (v5 4 重 + 权限发放 → v6 5 重 + 权限发放 + Colang DSL → v7 6 重 + 权限发放 + Colang DSL + Superpowers Skill Guard; **后续 v8 (action_rail + flow_executor) + v9 (evidence_guard) 在 2026-08-19 验证时未合入本节**):

> ⚠️ **2026-08-19 主人已知**: 本 § 是历史 v7 嵌套视角, 实际 v9 lineage 见末尾 §v7→v9 视角漂移 + 实际 9-fold lineage. 重写 v9 版本是后续路线 (R128+), 当前按 R119 "形式撤销原意保留" 纪律保留历史.

## 7 重守门 (嵌套结构 v7 视角, 从内到外 — **历史 §, 实际 v9 见末尾**)

1. **编译时 hardcode** (内层, **原则洋葱整体**编译时拒绝, 不只是 13 键)
2. **运行时拦截** (中间层, 所有决策前 async trait check)
3. **物理隔离** (外层, 重大修改需物理访问 + 物理多签)
4. **反思期审计** (外层, **守护越权检查**, 不与生命力反思混淆)
5. **多 AI 一致** (外层, 3 个不同 LLM 独立检查, R125-14 superpowers Skill 触发)
6. **Colang DSL 守门** (R125-5 NVIDIA Guardrails 借鉴, DSL 表达"什么操作允许/禁止")
7. **Superpowers Skill Guard** (R126-guard-7 NEW, superpowers 借鉴 234 file, 14 Skill + Recommender + Executor 状态机表达"什么 Skill 被允许/禁止/有条件放行")

## 权限发放 (独立机制, 不是守门)

- **多 AI 一致** = apeireth-council 智囊团审议(7 强制 + 动态专家, 按风险触发)
- **公式** = V0.5 24 维权重公式 (实际 `V05_DIM_COUNT: usize = 24` @ `apeireth-asi/src/lib.rs:56`); 30 维升级 R126 P1-4 在 master HEAD `9bf36b1e` **未合入** (历史/路线图误传, 是路线图 v2.0 长期目标)
- **人类决策** = L0 HA 真实人类批准
- **风险分级输出** = critical 7 / high 5 / medium 3 / low 1 / info 0
- **守门 1-7 联合** = 守住"没有相应权限而运行的代码"

## v4 → v5 → v6 → v7 → v8 → v9 变化 (**历史; 实际 v9 已落地, 见末尾 §**)

- v4 说"5 重守门融入每层"
- v5 改"4 重守门嵌套 + 权限发放(独立机制)" (2026-07-31)
- **v6 改"6 重守门嵌套 + 权限发放(独立机制) + Colang DSL 守门(新加第 6 重)"** (2026-08-10 R125 B4, 触发 R125-5 NVIDIA Guardrails)
- **v7 改"7 重守门嵌套 + 权限发放(独立机制) + Colang DSL 守门 + Superpowers Skill Guard(新加第 7 重)"** (2026-08-10 21:11 R126 P1-3 retry done, 触发 R125-14/15/16/18/19/126-guard-7 superpowers 借鉴 234 file)
- **v8 改"8 重守门嵌套 + action_rail (守门 8) + flow_executor"** (2026-08-10 R127-2 P6-3 done, 触发 Guardrails ActionDispatcher 借鉴)
- **v9 改"9 重守门嵌套 + evidence_guard (守门 9)"** (2026-08-xx R131 done)

- 守门 1 范围扩大到**原则洋葱整体** (v6: 不只是 12 键 → v7: 不只是 13 键, R125-12 后 PHL-07 接受 → v9: 不只是 13 键, 同)
- 多 AI 一致**不再算守门**——是权限发放机制 (v5)
- **v6 多 AI 一致**算守门 (5 重) (R125-14 superpowers Skill 触发)
- **Colang DSL 守门** (6 重) (R125-5 NVIDIA Guardrails 借鉴)
- **Superpowers Skill Guard** (7 重, NEW) (R126-guard-7, 借鉴 superpowers 234 file 14 Skill + SkillRegistry + SkillRecommender + SkillExecutor + 5 phase state machine)
- **Action Rail** (8 重, v8 NEW) (R127-2 P6-3, 借鉴 Guardrails ActionDispatcher)
- **Evidence Guard** (9 重, v9 NEW) (R131)

## 6 项不假装 = O 层(与 13 键同层, v7 修正)

## 出处

阶段 2 §6.1 + 阶段 1 §18.6 + 阶段 4 correction-v5 + R125 B4 (R125-5 NVIDIA Guardrails) + R126 P1-3 retry (R126-guard-7 superpowers).

详见 [`docs/stage4/stage4-correction-v5-gates-refined.md`](../../stage4/stage4-correction-v5-gates-refined.md)

## 6 个内容 (升级为 7 个, v7 修正)

1. 编译时 hardcode — Rust 6 大编译时约束
2. 运行时拦截 — `async RuntimeInterceptor` trait
3. 物理隔离 HA — 修改需重新编译 + 物理多签(AI×3 + 人×2 + 密钥×3)
4. 反思期审计 — Cognitive-Dream 24h 自动审计
5. **多 AI 一致** (v6 升) — 3 个不同 LLM 独立检查
6. **Colang DSL 守门** (v6 新加) — DSL 表达"什么操作允许/禁止"
7. **Superpowers Skill Guard** (v7 新加) — 14 Skill 状态机表达"什么 Skill 被允许/禁止/有条件放行" (R126-guard-7)

## 每层都自带(不是独立层)

- E 层修改流程 = 五重治理(MEWG + 多人 + 多 AI + 物理多签 + 反思期)
- S 层修改流程 = 智囊团 + 双签
- A 层修改流程 = A → M promotion
- M 层修改流程 = 经验沉淀包
- O 层修改流程 = 权限矩阵

## 出处(7 个内容, v7 历史视角; 实际 v9 见末尾)

阶段 2 §6.1 + 阶段 1 §18.6 + 阶段 4 correction-v4 + R125 B4 (v6 升 6 重) + R126 P1-3 retry (v7 升 7 重) + R127-2 P6-3 (v8 升 8 重) + R131 (v9 升 9 重).

详见 [`docs/stage4/stage4-correction-v4-onion-dedupe.md`](../../stage4/stage4-correction-v4-onion-dedupe.md)

## 8 哲学锚穿透 (R125 B5 升 8 锚, R126 P1-2 实施)

- **S-1** 北极星: 9 重守门 + 权限发放 = ASI 完整性工程化 (实际 v9, 历史 §7 写"7 重 v7"是过时, 见末尾 §)
- **S-2** 实事求是: v5 → v6 → v7 → v8 → v9 修正 (4 → 5 → 6 → 7 → 8 → 9 重 + Colang DSL + Superpowers Skill Guard + Action Rail + Evidence Guard)
- **S-3** 质量工程化: Colang DSL 编译期 hardcode (跟 R123-1 clippy+doc 清关联)
- **O-1** 安全优先: 9 重守门 (实际 v9) = 最高安全标准
- **O-2** 走在前人经验上: 借鉴 NVIDIA Guardrails Colang DSL (R125-5) + superpowers Skill Guard (R126-guard-7, 借鉴 234 file) + Guardrails ActionDispatcher (R127-2 P6-3) + R131 Evidence
- **O-5** 不假装: 编译期 hardcode 原则洋葱整体

## 不漂移 (R126 P1-3 retry 升 v7, **历史 §7 嵌套视角**; 实际 v9 lineage 见末尾)
- 🔒 权限发放独立机制严守
- 🔒 Colang DSL 守门 (v6 升, R125-5 实施时落地)
- 🔒 Superpowers Skill Guard (v7 新加, R126-guard-7 实施时落地, 14 Skill + 5 phase state machine)
- 0 改 workspace.version (R125 末 B2 升 1.2.0, 1.0 release **未归** 仍 1.2.0 — **双轴制**: 产品轴 tag = v1.0.0 (8/18 发布), workspace 轴 = 1.2.0 (`Cargo.toml:228`); 顶层 README 明确)
- 0 改 R11 baseline 3 值
- 0 改 13 键原 13 (R125-12 后 PHL-07 = 13 键, A3 严守, 但 `apeireth-core/src/lib.rs` philosophy.rs 仍 hardcode `[PhilosophyKey; 12]` — PHL-07 待合并 core)
- 0 改 V0.5 24 维 (代码 `V05_DIM_COUNT: usize = 24` @ `apeireth-asi/src/lib.rs:56`; 30 维升级 R126 P1-4 在 master HEAD `9bf36b1e` **未合入** — git log 搜 "P1-4 / 30 维 / v05.*30" 全 0 命中, 是历史/路线图误传)

## v7 → v9 视角漂移 + 实际 9-fold lineage (2026-08-19, 修正 8/19)

> **诚实标注 (0 装 PASS 严守)**: 本节描述 §7 重守门列表与实际代码 lineage 不 1:1 对齐的事实. 0 假装"已统一".

### 实际 lineage (master HEAD `9bf36b1e`, 2026-08-19 subagent 验证)

| 版本 | 触发 | 加 mod | 累计 | 编译期 hardcode |
|---|---|---|---|---|
| v6 | R125-5 (NVIDIA Guardrails 借鉴) | `colang_dsl` | **6 重** | — |
| v7 | R126-guard-7 (superpowers 借鉴) | `skill_guard` + `seven_fold_guard` | **7 重** | `SkillId::COUNT = 7`, `SEVEN_FOLD_GUARDS_HARDCODE = 7` |
| v8 | R127-2 P6-3 (action rail 借鉴 Guardrails ActionDispatcher) | `action_rail` + `flow_executor` | **8 重** | `EIGHT_FOLD_GUARDS_HARDCODE = 8` |
| v9 | R131 (evidence guard 升级) | `evidence_guard` | **9 重** | 编译期 `assert!` 验证 |

实际代码 lineage (`crates/apeireth-sovereignty/src/lib.rs:65-83`):
```
R126-guard-7 升级 (B4 6 重守门 v6 → v7): 加 skill_guard + seven_fold_guard 2 个新 mod
R127-2 P6-3 升级 (B4 7 重守门 v7 → 8 重守门 v8): 加 action_rail + flow_executor 2 个新 mod
R131 升级 (B4 8 重守门 v8 → 9 重守门 v9): 加 evidence_guard 1 个新 mod
```

**实际是 9 重 v9** (本文件原 §7 仍写"7 重 v7"是**历史过时** — 2026-08-19 subagent 验证: 子代理 grep `crates/apeireth-sovereignty/src/` 抓到完整 lineage)。

### 两套视角并存 (更新版)

实际写代码用了 **"Skill 编排"视角** (`SkillId::ALL` 7 个 1:1 映射 7 重, 见 `skill_guard.rs:99`), 但本文件 §7 重守门 (本节之前) 沿用 v6 时期产物 **"嵌套结构"视角** (编译时→运行时→物理隔离→反思期审计→多 AI 一致→Colang DSL + 守门 7). 两套视角**并存于代码 + 文档**, 没整合成一套. **2026-08-19 加注**: §7 还漏写 v8 (action_rail) + v9 (evidence_guard), 不仅视角漂移, 连数量都过时.

### 实际代码的 7 Skill (`SkillId::ALL`, 编译期 hardcode `[SkillId; 7]`) — 仅 v7 部分

```rust
pub enum SkillId {
    MultiAiGuard,           // gate 1 (Skill 视角)
    MultiHumanGuard,        // gate 2 (Skill 视角)
    PhysicalMultisigGuard,  // gate 3
    ReflectionGuard,        // gate 4
    MewgGuard,              // gate 5 (Skill 视角)
    ColangDslGuard,         // gate 6 (R125-5)
    SuperpowersSkillGuard,  // gate 7 (R126-guard-7 NEW)
}
pub const ALL: [SkillId; 7] = [...];  // 编译期 hardcode 7 entries
pub const COUNT: usize = 7;
```

> 注: `SkillId::ALL` 是 v7 时的 Skill 视角定义, **v8 (action_rail) + v9 (evidence_guard) 不在 Skill 视角** — 它们是独立 mod (gate 8/9), 不走 Skill 编排. 嵌套视角把"7 重 Skill 编排 + 2 个独立 mod" 算作 "9 重 v9" 总数.

### 两套视角的 1-to-1 对齐表 (0 完全对齐, 历史视角)

| 嵌套视角 (本文件 §7) | Skill 视角 (`SkillId::ALL`) | 1-to-1? |
|---|---|:---:|
| 守门 1 编译时 hardcode | (不在 7 Skill — orthogonal 编译期机制) | ❌ 不在 |
| 守门 2 运行时拦截 | (不在 7 Skill — orthogonal 运行时机制) | ❌ 不在 |
| 守门 3 物理隔离 | gate 3 `PhysicalMultisigGuard` | ✅ 名字 + 编号一致 |
| 守门 4 反思期审计 | gate 4 `ReflectionGuard` | ✅ 一致 |
| 守门 5 多 AI 一致 | gate 1 `MultiAiGuard` | ❌ 编号重号 (Skill 视角 1, 嵌套视角 5) |
| 守门 6 Colang DSL | gate 6 `ColangDslGuard` | ✅ 一致 |
| (嵌套视角无7) | gate 7 `SuperpowersSkillGuard` (R126-guard-7 NEW) | ❌ 嵌套视角补 7 |
| (嵌套视角无) | gate 2 `MultiHumanGuard` (多人投票) | ❌ 嵌套视角**漏列** |
| (嵌套视角无) | gate 5 `MewgGuard` (MEWG 汇总) | ❌ 嵌套视角**漏列** |

### 3 处诚实标注 (更新: 4 处)

1. **编译时 hardcode + 运行时拦截** 在嵌套视角是 gate 1 + gate 2, 在 Skill 视角**不在 7 Skill** (它们是 orthogonal 机制: 编译期 `assert!(SkillId::COUNT == 7)` 硬约束 + 运行时 `async trait check`, 跟 Skill 编排的"运行顺序"是两条线). 嵌套视角把它们算 gate, Skill 视角不算.

2. **MultiHumanGuard + MewgGuard** Skill 视角有, 嵌套视角**没列**. 可能旧 v6 文件漏了, 可能嵌套视角根本不包含它们 (它们是 Skill 编排特有: `MultiHumanGuardSkill` 借鉴 superpowers 模式, `MewgGuardSkill` 借鉴 MEWG 模式). **0 假装对齐**.

3. **Superpowers Skill Guard** 在 Skill 视角是 gate 7 (R126-guard-7 NEW), 嵌套视角**没列** v7 之前的7 (嵌套视角本来只有 6 重). 嵌套视角的"v7 加 守门 7"是后补的, 严格说不是真嵌套, 是把 Skill 视角的 gate 7 **硬塞**进嵌套视角的位置. 嵌套视角 gate 7 跟 gate 3-6 同质 (都是某种"外层"), 但 Superpowers Skill Guard 本质是"中心调度编排器", 跟外层守门是**不同维度**.

4. **(2026-08-19 新增)** **本文件 §7 重守门列表过时至 v7, 实际是 v9**: v8 (action_rail + flow_executor, R127-2 P6-3) + v9 (evidence_guard, R131) 完全没在本文件 §7 提到. 嵌套视角按 v7 写, 跟实际 v9 差 2 个 gate. 主人 8/19 已知.

### 当前选择 (主人 2026-08-19 拍板: 末尾加一段, 0 重写)

- 保留嵌套视角 (本文件 §7 重守门列表, 含 v7 后补的守门 7) — **但 §7 已过时, 实际是 v9**
- 保留 Skill 视角 (代码 `SkillId::ALL` 实际实施) — **v7 部分, v8/v9 不在 Skill 视角**
- **0 假装两套视角 1-to-1 对齐**
- **0 重写本文件为 Skill 视角 + v9** (那是另一项工作, 需要重新画 v4→v5→v6→v7→v8→v9 的演变图 + 同步所有引用本文件的章节)
- 后续如果要重写, 应是 R128+ 路线 (per 旧 ROADMAP §4 v2.0 长期 + 决策 #21 Phase 4)

### 引用本节的下游

- `crates/apeireth-sovereignty/src/seven_fold_guard.rs` (7 重守门 v7 衔接器) — 引用 Skill 视角 v7 部分
- `crates/apeireth-sovereignty/src/skill_guard.rs` (守门 7 实施) — 引用 Skill 视角
- `crates/apeireth-sovereignty/src/action_rail.rs` (v8 行动轨守门) — **§7 没列**
- `crates/apeireth-sovereignty/src/flow_executor.rs` (v8 流程执行器) — **§7 没列**
- `crates/apeireth-sovereignty/src/evidence_guard.rs` (v9 证据守门) — **§7 没列**
- `crates/apeireth-pybridge/src/stage7_i6_permission_security.rs` (K3 集成) — 引用"6 重 v7 + G7 跨语言 = 7 重", 跟本文件嵌套视角错位
- `crates/_archived/apeireth-formal/src/stage5_2/verdict_cache_13keys_formal.rs` (形式化) — 引用 13 键 (本文件 §不漂移 已对齐)
- `docs/archive/pages-source/{api,architecture,index,getting-started}.md` — 全部写"6 重守门 v7" (gate 数错, 应是 7 重 v7; v9 实际更高)
