# 城堡内墙洋葱架构 (Onion Wall Architecture) (2026-07-31)

> **范围 (主 17:43 实事求是)**: R14 阶段 4 落实架构前的**架构概念文档**——记录主人 2026-07-31 走法乙的 3 个核心细节, 作为后续 Rust trait + crate 边界 + DB schema 的**单一入口参考**。
>
> **触发**: 主人 2026-07-31 指示 "哲学守门应该并到原则洋葱外层, 不是独立 crate; V0.5/V1136 是 R11 对照基线, 不是 R14 重设计对象"。
>
> **依据**:
> - 阶段 1 §3 原则洋葱 v3.0 (E/S/A/M/O 5 层)
> - 阶段 1 §18.6 权限根 = 真实人类批准 (洋葱 0 层)
> - 阶段 1 §18.7 双洋葱正交 (已精化为"洋葱核心嵌套" — R14-D7)
> - 阶段 2 §5 权限包 (L0-L5)
> - 阶段 2 §10 智囊团 (7 席 + 按住 + Synthesis)
> - 阶段 2 §12 哲学守门 (V3 9 键 + 5 项不假装)
> - 阶段 1 §18.5 平台职责三件套 (提供 / 约束 / 记录)
> - R14-D7 洋葱核心嵌套精化
>
> **不修改承诺**:
> ❌ 不写新 Rust 代码 (本节只描述 trait 签名, 不写实现)
> ❌ 不画 Mermaid 图 (用 ASCII 简化示意, 阶段 3 蓝图才画正式架构图)
> ❌ 不重写 V0.5 / V1136 / 哲学守门 9 键 (保留为历史轨迹)
> ❌ 不修改其他 16 份 stage2 文档
> ❌ 不修改 crates/ 占位实现 (仅 crates/README 表)
> ❌ 不修改 cargo metadata `description` 字段
> ✅ 仅**新增**本概念文档 + 措辞精化原文位置 (见 §6 衔接锚点)

---

## §1. 比喻起源 (主人原话引用 + 走法乙)

### 1.1 主人原话 (2026-07-31)

> "哲学守门应该并到原则洋葱外层, 不是独立 crate。"
> "V0.5/V1136 是 R11 对照基线, 不是 R14 重设计对象。"

> **D8-fix 主人纠偏原话 (2026-07-31, 同日纠偏)**:
> "我们实际上要的就是双洋葱正交，两把锁。"
>
> **D8-fix 纠错段 (主 17:58 不假装 + 主 17:43 实事求是)**: 本节 §1.2 + §1.3 + §2 + §3 中按"原则洋葱+权限洋葱交叉咬合 / 城堡内墙 / onion_wall/ / OnionGate 联合守门 trait / DecisionSignature 14+ 守卫"措辞均为 **错版历史轨迹**, 由 Leader 提议但未对齐阶段 2 §10 decision-system Phase 1 已落的"两把独立锁"设计。错版完整保留**不删除** (主 17:58 不假装"以前没说错, 只是现在看得更准"), 主人最新纠偏后改为 **两把独立锁 (锁 A 原则洋葱 + 锁 B 权限洋葱)** 设计, 详见 §D8-fix 增量节 (位于各节末尾)。

### 1.2 走法乙 vs 走法甲

| 走法 | 哲学守门归属 | V0.5 / V1136 | 状态 |
|------|------------|-------------|------|
| **走法甲** (原方案, 阶段 2 §12 沉淀) | 独立 crate `apeireth-philosophy` | Rust 重写公式 + 重做真测引擎 | ❌ 已过时 (R14-D8 主人精化) |
| **走法乙** (R14-D8 主人精化) | 并入 `apeireth-core/src/onion_wall/` (与原则洋葱、权限洋葱交叉咬合) | **保留为 R11 对照基线** (v1077 / v1106) | ✅ ~~采用~~ **R14-D8-fix 改**: 并入 `apeireth-core/src/onion/` (锁 A 原则洋葱 + 锁 B 权限洋葱, 两把独立锁 + AND 运算) |

### 1.3 走法乙的 3 核心细节

1. **哲学守门 = 城堡内墙**: 原则洋葱 + 权限洋葱**交叉咬合**形成"城堡内墙", 任何 action 必须**同时通过**两层 (per-layer 双重过滤), 不允许"高权限绕过低原则"或"高原则阻止低权限" (与阶段 2 §7.2 正交运算一致)。

> **D8-fix 纠错 (主人 2026-07-31)**: 细节 1 措辞为错版历史轨迹——"交叉咬合 / per-layer 双重过滤"**不是**主人意图, 主人最新精化为 **"两把独立锁 (锁 A 原则洋葱 + 锁 B 权限洋葱) + 最后 AND 运算"**。详见 §2 D8-fix 增量节。
2. **守护对象 = 阶段1+2 沉淀的具体决策** (双根 / 双洋葱 / 三件套 / 七席 / L1-L5 / MEWG / HA / 旧规则合法性 / 漂移 P0), 不是抽象的 9 键字符串匹配。9 键作为辅助语义网**保留**。
3. **V0.5 / V1136 = R11 对照基线** (不重写不重做), R14 用法 = Phase 2+ 性能对照 + 重写后行为 1:1 验证 (不刷 KPI)。

---

## §2. 内墙咬合形态 (ASCII 简化示意)

> **D8-fix 主人纠正 (2026-07-31)**: §2 标题改为 **"双锁独立形态"**——"内墙咬合"为错版措辞, 主人明确"两把锁"各自独立运行, 最后 AND 运算。

### 2.1 整体洋葱结构 (含 0 层)

```
                     ┌─────────────────────────────────────────────┐
                     │              关系形态 / 双方共同生活 (外层)        │
                     │  ┌─────────────────────────────────────────┐ │
                     │  │          平台机制 (中间层)                  │ │
                     │  │  ┌───────────────────────────────────┐ │ │
                     │  │  │       城堡内墙 (onion_wall/)        │ │ │
                     │  │  │  ┌─────────────────────────────┐ │ │ │
                     │  │  │  │    洋葱 0 层 (权限根)           │ │ │ │
                     │  │  │  │   真实人类批准 (R14-D7)         │ │ │ │
                     │  │  │  └─────────────────────────────┘ │ │ │
                     │  │  │   原则洋葱 (E/S/A/M/O) ⨯ 权限洋葱 (L0-L5) │ │
                     │  │  │   per-layer 双重过滤                │ │ │
                     │  │  └───────────────────────────────────┘ │ │
                     │  └─────────────────────────────────────────┘ │
                     └─────────────────────────────────────────────┘
```

### 2.2 原则洋葱 ⨯ 权限洋葱 交叉咬合 (per-layer 双重过滤)

```
原则洋葱 (rank ∈ {5(E), 4(S), 3(A), 2(M), 1(O)})
   ↕ 交叉咬合 (per-layer)
权限洋葱 (level ∈ {0, 1, 2, 3, 4, 5})

∀ action A:
  decision = principle_check(A, rank)  AND  permission_check(A, level)
    ↓
  onion_wall::gate(action, rank, level) → Allow | Deny
    ↓
  OnionGate::guard_decision(signature) → Audit + Reflect
```

**正交不互替 (与阶段 2 §7 一致)**:
- 原则不通过 → 一律拒绝 (无论权限多高)
- 权限不通过 → 一律拒绝 (无论原则多正确)
- 两者都通过 → 进入执行 (SGI + 历史流记录 + 阶段 1 §18.5 三件套"记录")
- **真实人类批准在洋葱最核心 (洋葱 0 层, §18.6 权限根)**, 不是洋葱之外的第三守门 (R14-D7 精化)

> **D8-fix 主人纠正 (2026-07-31)**: §2.2 标题改为 **"两把独立锁各自跑, 最后 AND"**——错版措辞"per-layer 双重过滤 / 交叉咬合 / OnionGate 联合守门"**不**是主人意图。正确设计:
>
> ```
> 锁 A: 原则洋葱 (5 重守门, 独立 trait 各自跑)
>   ├── ELayerCheck    (E 层 - 原则层, hardcode + 多 AI 一致)
>   ├── SLayerAudit    (S 层 - 价值观, 智囊团审核 + 物理多签)
>   ├── ALayer         (A 层 - 经验, AI 自己可改 + 版本备份)
>   ├── MLayer         (M 层 - 方法论, AI 自己可改 + promotion 管道)
>   └── OLayerGuard    (O 层 - 操作, AI 自己可改 + 9 键守门)
>
> 锁 B: 权限洋葱 (Layer 0-5, 独立 trait 各自跑)
>   ├── L0Self         (AI 自决)
>   ├── L1Monitor      (可观察)
>   ├── L2Council      (智囊团决议)
>   ├── L3MultiSig     (多人多签)
>   ├── L4Hardware     (硬件多签)
>   └── L5Physical     (物理多签 + 多人)
>
> ∀ action A:
>   verdict_A = principle_onion::check(A)  // 锁 A: 5 个子 trait 各自跑, 全部 Allow 才 Allow
>   verdict_B = permission_onion::check(A) // 锁 B: 6 个子 trait 各自跑, 全部 Allow 才 Allow
>   final = verdict_A AND verdict_B          // D2 §7.2 硬规则, 串行 AND, 不是 per-layer 咬合
>     ↓
>   onion::dispatcher::dispatch(A, verdict_A, verdict_B) → Execute | Deny | RequireHumanApproval
> ```
>
> **关键区别**: 错版"per-layer 双重过滤"= 原则洋葱每层都要伴随对应权限洋葱层 (层级耦合); D8-fix "两把独立锁各自跑"= 锁 A 内部 5 子 trait 串行 + 锁 B 内部 6 子 trait 串行 + 两把锁之间 AND (顶层耦合, 不是层级耦合)。这与阶段 2 §10 decision-system Phase 1 已落的设计**精确对齐**。

### 2.3 与现有阶段 1+2 沉淀的衔接

| 阶段 1+2 沉淀 | 在内墙中的位置 |
|--------------|--------------|
| 阶段 1 §3 原则洋葱 (5 层 E/S/A/M/O) | 原则洋葱维度 |
| 阶段 1 §18.5 三件套 (提供/约束/记录) | 内墙"约束"维度 (提供 = 中间层, 记录 = onion_wall/audit) |
| 阶段 1 §18.6 双根 (原则根 + 权限根) | 权限根 = 洋葱 0 层, 原则根 = 内墙不变量 |
| 阶段 1 §18.7 双洋葱正交 (R14-D7: 洋葱核心嵌套) | 内墙 = 洋葱核心 + 中间层, 外层 = 关系形态 |
| 阶段 1 §18.8 七席审议庭 | onion_wall/council/ 子模块 (七席触发器) |
| 阶段 1 §18.9 L1-L5 分层验证网 | onion_wall/validation/ 子模块 (5 层校验) |
| 阶段 2 §5 权限包 (L0-L5) | 权限洋葱维度 |
| 阶段 2 §7 双洋葱正交决策 | 内墙正交运算 |
| 阶段 2 §8 MEWG | onion_wall/mewg/ 子模块 (多证据聚合) |
| 阶段 2 §9 HA 硬门槛 | 洋葱 0 层 (真实人类批准) — 阶段 2 §9.1 R14-D7 精化 |
| 阶段 2 §10 智囊团 | onion_wall/council/ 子模块 (7 席 + Synthesis) |
| 阶段 2 §12 哲学守门 9 键 | 保留为辅助语义网 (onion_wall/keys/ 子模块) |
| 阶段 2 §14 漂移 P0 优先级 | onion_wall/drift/ 子模块 (漂移检测 + 优先级) |

> **D8-fix 主人纠正 (2026-07-31)**: §2.3 表格中所有 `onion_wall/` 路径改为 `onion/principle/` + `onion/permission/` + `onion/dispatcher.rs` + `onion/human_gate.rs`, 详见 §3 D8-fix 增量节。
> - 三件套"记录" → `onion/dispatcher.rs` (审计追溯由 dispatcher 收口)
> - 七席审议庭 → `onion/dispatcher.rs::council()` (锁 B L2 子 trait 内部触发)
> - L1-L5 分层验证 → `onion/dispatcher.rs::validation()` (锁 B L1 子 trait 内部触发)
> - 智囊团 → `onion/dispatcher.rs::synthesis()` (锁 B L2 子 trait)
> - MEWG → `onion/permission/mewg.rs` (锁 B 内部多证据聚合)
> - HA → `onion/human_gate.rs` (硬门槛, 与 dispatcher 解耦)
> - 9 键 → `onion/principle/keys.rs` (锁 A OLayerGuard 内部辅助语义网)
> - 漂移 → `onion/principle/drift.rs` (锁 A MLayer 内部漂移检测)

---

## §3. 模块边界映射 (`apeireth-core/src/onion_wall/`)

> **D8-fix 主人纠正 (2026-07-31)**: §3 标题改为 **`apeireth-core/src/onion/`** (移除 `_wall`, 路径扁平化); 子模块结构由"1 个 onion_wall/ 大目录 12 文件"改为 **"onion/principle/ (锁 A) + onion/permission/ (锁 B) + onion/dispatcher.rs + onion/human_gate.rs"** 4 模块。下面 §3.1 子模块结构保留错版 `onion_wall/` 作为历史轨迹, 紧接其后追加 D8-fix 新版 `onion/` 目录布局。

### 3.1 子模块结构 (Rust crate 内目录布局)

> **本节 §3.1 旧措辞 (错版历史轨迹, R14-D8 错版 Leader 提议)**: 单一 `onion_wall/` 大目录 12 文件布局, **不删除**, 完整保留如下:
> ```
> apeireth-core/
> ├── src/
> │   ├── onion_wall/
> │   │   ├── mod.rs              # 顶层 mod 入口
> │   │   ├── gate.rs             # OnionGate trait (原则+权限联合守门)
> │   │   ├── decision.rs         # DecisionSignature 结构 (阶段1+2 沉淀的决策清单)
> │   │   ├── keys.rs             # V3 9 键 (从 philosophy crate 迁移, 保留为辅助语义网)
> │   │   ├── principle.rs        # 原则洋葱 (E/S/A/M/O 5 层) 守门
> │   │   ├── permission.rs       # 权限洋葱 (L0-L5) 守门
> │   │   ├── mewg.rs             # 多证据加权治理 (阶段 2 §8)
> │   │   ├── ha.rs               # 人类批准硬门槛 (阶段 2 §9)
> │   │   ├── council.rs          # 智囊团 7 席 (阶段 2 §10)
> │   │   ├── validation.rs       # L1-L5 分层验证网 (阶段 1 §18.9)
> │   │   ├── drift.rs            # 漂移检测 + 优先级 (阶段 2 §14)
> │   │   ├── audit.rs            # 审计 + 历史流 (阶段 1 §18.5 "记录")
> │   │   └── reflection.rs       # 反思期 (阶段 2 §12)
> │   └── ...                     # 其他 core 模块
> └── Cargo.toml
> ```

> **D8-fix 新版 (主人 2026-07-31 纠偏后采纳)**: `onion/` 目录布局 = 锁 A 原则洋葱子目录 + 锁 B 权限洋葱子目录 + 顶层 dispatcher.rs 与 human_gate.rs:
> ```
> apeireth-core/
> ├── src/
> │   ├── onion/
> │   │   ├── mod.rs                    # 顶层 mod 入口 (导出 PrincipleOnion / PermissionOnion / dispatcher)
> │   │   │
> │   │   ├── principle/                # 锁 A: 原则洋葱 (5 重守门, 独立 trait 各自跑)
> │   │   │   ├── mod.rs                # PrincipleOnion trait (锁 A 入口)
> │   │   │   ├── e_layer.rs            # ELayerCheck trait (E 层 - 原则, hardcode + 多 AI 一致)
> │   │   │   ├── s_layer.rs            # SLayerAudit trait (S 层 - 价值观, 智囊团审核 + 物理多签)
> │   │   │   ├── a_layer.rs            # ALayer trait (A 层 - 经验, AI 自己可改 + 版本备份)
> │   │   │   ├── m_layer.rs            # MLayer trait (M 层 - 方法论, AI 自己可改 + promotion 管道)
> │   │   │   ├── o_layer.rs            # OLayerGuard trait (O 层 - 操作, AI 自己可改 + 9 键守门)
> │   │   │   ├── keys.rs               # V3 9 键 (从 philosophy crate 迁移, 锁 A OLayerGuard 内部辅助语义网)
> │   │   │   └── drift.rs              # 漂移检测 + 优先级 (阶段 2 §14, 锁 A MLayer 内部)
> │   │   │
> │   │   ├── permission/               # 锁 B: 权限洋葱 (Layer 0-5, 独立 trait 各自跑)
> │   │   │   ├── mod.rs                # PermissionOnion trait (锁 B 入口)
> │   │   │   ├── l0_self.rs            # L0Self trait (AI 自决)
> │   │   │   ├── l1_monitor.rs         # L1Monitor trait (可观察)
> │   │   │   ├── l2_council.rs         # L2Council trait (智囊团决议)
> │   │   │   ├── l3_multisig.rs        # L3MultiSig trait (多人多签)
> │   │   │   ├── l4_hardware.rs        # L4Hardware trait (硬件多签)
> │   │   │   ├── l5_physical.rs        # L5Physical trait (物理多签 + 多人)
> │   │   │   └── mewg.rs               # MEWG 多证据聚合 (阶段 2 §8, 锁 B 内部多证据加权)
> │   │   │
> │   │   ├── dispatcher.rs             # 双锁调度器: principle_onion.check() AND permission_onion.check() → Execute | Deny
> │   │   └── human_gate.rs             # HA 硬门槛: D2 §9 10 项必 HA, 触及洋葱 0 层时强制要求真实人类批准
> │   │
> │   └── ...                           # 其他 core 模块
> └── Cargo.toml
> ```

### 3.2 守护 trait: `OnionGate` (原则+权限联合守门)

> **本节 §3.2 旧措辞 (错版历史轨迹, R14-D8 错版 Leader 提议)**: `OnionGate` 联合守门 trait + `DecisionSignature` 14+ 守卫**不删除**, 完整保留如下:
>
> ```rust
> //! OnionGate (主 17:58 不假装: trait 不绑定具体实现, 只承诺"联合守门")
> //! 阶段 1 §18.5 三件套"约束" + 阶段 2 §7.2 正交运算 + R14-D7 洋葱核心嵌套
>
> use serde::{Deserialize, Serialize};
>
> /// OnionGate 联合守门 trait
> /// 主入口: guard_decision(decision) → DecisionVerdict
> pub trait OnionGate {
>     /// 联合守门: 原则检查 AND 权限检查 (per-layer 双重过滤)
>     /// 返回 DecisionVerdict::Allow / Deny (含原因)
>     fn guard_decision(&self, decision: &DecisionSignature) -> DecisionVerdict;
>
>     /// 洋葱 0 层守门 (真实人类批准, R14-D7): 触及洋葱 0 层的决策必须 HA
>     fn guard_onion_zero(&self, decision: &DecisionSignature) -> DecisionVerdict;
>
>     /// 审计追溯: 给定 decision_id, 返回完整守门链路
>     fn trace_guard(&self, decision_id: &DecisionId) -> Result<GuardTrace, GateError>;
> }
>
> #[derive(Debug, Clone, Serialize, Deserialize)]
> pub enum DecisionVerdict {
>     Allow { rank: u8, level: u8, audit_ref: String },
>     Deny { rank: u8, level: u8, reason: String, audit_ref: String },
>     RequireHumanApproval { rank: u8, level: u8, intent: String, audit_ref: String },
> }
> ```
>
> **抽象层原则 (主 17:58 不假装, 错版)**:
> - trait **不绑定**具体守门策略 (L1-L5 / 七席触发器 / MEWG 系数)
> - trait **不绑定**具体人类批准机制 (阶段 2 §9.1 HA 硬门槛)
> - trait **必须**包含 `trace_guard` —— 主 17:58 不假装 + 主 17:43 实事求是: **不可静默**

> **D8-fix 新版 (主人 2026-07-31 纠偏后采纳)**: 删除 `OnionGate` 联合守门 trait, 拆为两把独立锁 trait, 每把锁的入口 trait 与 5+6 个子 trait 各自独立:
>
> ```rust
> //! 锁 A: PrincipleOnion (主 17:58 不假装: trait 不绑定具体实现, 只承诺"5 重守门各自跑")
> //! 阶段 1 §3 原则洋葱 + 阶段 2 §12 哲学守门 trait 框架
> //! 路径: apeireth-core/src/onion/principle/mod.rs
>
> use serde::{Deserialize, Serialize};
>
> /// 锁 A 入口: PrincipleOnion trait (5 个子 trait 各自跑, 全部 Allow 才 Allow)
> pub trait PrincipleOnion {
>     fn check_e_layer(&self, action: &Action) -> VerdictA;
>     fn check_s_layer(&self, action: &Action) -> VerdictA;
>     fn check_a_layer(&self, action: &Action) -> VerdictA;
>     fn check_m_layer(&self, action: &Action) -> VerdictA;
>     fn check_o_layer(&self, action: &Action) -> VerdictA;
>     /// 锁 A 总入口: 5 个子 verdict 串行 AND
>     fn check(&self, action: &Action) -> VerdictA;
> }
>
> /// 锁 B 入口: PermissionOnion trait (6 个子 trait 各自跑, 全部 Allow 才 Allow)
> /// 路径: apeireth-core/src/onion/permission/mod.rs
> pub trait PermissionOnion {
>     fn check_l0_self(&self, action: &Action) -> VerdictB;
>     fn check_l1_monitor(&self, action: &Action) -> VerdictB;
>     fn check_l2_council(&self, action: &Action) -> VerdictB;
>     fn check_l3_multisig(&self, action: &Action) -> VerdictB;
>     fn check_l4_hardware(&self, action: &Action) -> VerdictB;
>     fn check_l5_physical(&self, action: &Action) -> VerdictB;
>     /// 锁 B 总入口: 6 个子 verdict 串行 AND
>     fn check(&self, action: &Action) -> VerdictB;
> }
>
> /// 双锁 AND 运算 + HA 硬门槛统一调度
> /// 路径: apeireth-core/src/onion/dispatcher.rs
> pub fn dispatch(action: &Action, a: &impl PrincipleOnion, b: &impl PermissionOnion, ha: &impl HumanGate)
>     -> FinalVerdict
> {
>     let va = a.check(action);
>     let vb = b.check(action);
>     match (va, vb) {
>         (VerdictA::Allow, VerdictB::Allow) => {
>             // 锁 A + 锁 B 都通过 → 检查 HA 硬门槛 (D2 §9 10 项)
>             if ha.requires_approval(action) {
>                 FinalVerdict::RequireHumanApproval { intent: action.intent() }
>             } else {
>                 FinalVerdict::Execute { audit_ref: trace(a, b, ha) }
>             }
>         }
>         _ => FinalVerdict::Deny { reason: format!("lock_a={:?}, lock_b={:?}", va, vb) },
>     }
> }
> ```

**抽象层原则 (主 17:58 不假装, D8-fix 新版)**:
- **锁 A trait 不绑定**具体守门策略 (E/S/A/M/O 各自的子策略, 例如硬编码 / 物理多签 / 9 键守门由各子 trait 自行实现)
- **锁 B trait 不绑定**具体升级路径 (L0-L5 各自的层级检查, 例如 AI 自决 / 智囊团 / 物理多签由各子 trait 自行实现)
- **dispatcher trait 不绑定**具体 AND 失败原因 (任一锁 Deny 即整体 Deny, 失败原因分别记录在 va / vb)
- **HumanGate trait 必须**包含 `trace_approval` —— 主 17:58 不假装 + 主 17:43 实事求是: **不可静默**

### 3.3 决策签名: `DecisionSignature` (阶段1+2 沉淀的决策清单)

> **本节 §3.3 旧措辞 (错版历史轨迹, R14-D8 错版 Leader 提议)**: `DecisionSignature` 14+ 守卫**不删除**, 完整保留如下:
>
> ```rust
> //! DecisionSignature = 阶段1+2 沉淀的具体决策结构化表达
> //! 唯一守护对象: 双根 / 双洋葱 / 三件套 / 七席 / L1-L5 / MEWG / HA / 旧规则 / 漂移 P0
>
> use serde::{Deserialize, Serialize};
>
> #[derive(Debug, Clone, Serialize, Deserialize)]
> pub struct DecisionSignature {
>     /// 决策 ID (全局唯一, 强不可变)
>     pub id: DecisionId,
>
>     /// 决策类别 (从阶段1+2 沉淀的决策清单)
>     pub category: DecisionCategory,
>
>     /// 原则洋葱层 (rank ∈ {5(E), 4(S), 3(A), 2(M), 1(O)})
>     pub principle_rank: u8,
>
>     /// 权限洋葱层 (level ∈ {0, 1, 2, 3, 4, 5})
>     pub permission_level: u8,
>
>     /// 涉及双根? (principle_root / permission_root)
>     pub touches_double_root: bool,
>
>     /// 涉及七席审议庭? (critical/high/medium/low/info)
>     pub council_risk: CouncilRisk,
>
>     /// 验证层 (L1-L5)
>     pub validation_layer: ValidationLayer,
>
>     /// 是否需要真实人类批准 (HA)
>     pub requires_human_approval: bool,
>
>     /// 漂移优先级 (P0-P3)
>     pub drift_priority: DriftPriority,
>
>     /// 决策时间戳 + 触发者
>     pub timestamp: i64,
>     pub triggered_by: String,
> }
>
> #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
> pub enum DecisionCategory {
>     DoubleRoot,           // 阶段 1 §18.6 双根变更
>     DoubleOnion,          // 阶段 1 §18.7 双洋葱 (R14-D7 洋葱核心嵌套)
>     PlatformTriad,        // 阶段 1 §18.5 三件套
>     Council,              // 阶段 1 §18.8 七席审议庭
>     ValidationL1L5,       // 阶段 1 §18.9 L1-L5 分层验证网
>     MEWG,                 // 阶段 2 §8 多证据加权治理
>     HA,                   // 阶段 2 §9 人类批准硬门槛
>     LegacyLegality,       // 阶段 2 §10 旧规则合法性
>     DriftP0,              // 阶段 2 §14 漂移 P0 优先级
>     // ... 阶段 1+2 沉淀的其他决策类别
> }
> ```

> **D8-fix 新版 (主人 2026-07-31 纠偏后采纳)**: 删除 `DecisionSignature` 14+ 守卫 enum 集中清单, 改为**决策按原则洋葱层 or 权限洋葱层分布到对应子 trait**, 不再有单一集中签名:
>
> ```rust
> //! D8-fix: 决策按锁 A / 锁 B 分布到对应子 trait, 各自携带签名
> //! 锁 A 子 trait: 每个原则层都有各自对应的领域决策 (E/S/A/M/O 各自一个 LayerAction)
> //! 锁 B 子 trait: 每个权限层都有各自对应的领域决策 (L0-L5 各自一个 LayerAction)
>
> /// 锁 A: 原则洋葱每层的领域决策 (5 个 struct, 替代 DecisionSignature 集中清单)
> pub struct ELayerAction { /* E 层决策字段, 例如根原则变更提案 */ }
> pub struct SLayerAction { /* S 层决策字段, 例如价值观变更审核 */ }
> pub struct ALayerAction { /* A 层决策字段, 例如经验策略变更 */ }
> pub struct MLayerAction { /* M 层决策字段, 例如方法论 promotion */ }
> pub struct OLayerAction { /* O 层决策字段, 例如底层操作 */ }
>
> /// 锁 B: 权限洋葱每层的领域决策 (6 个 struct, 替代 DecisionSignature 集中清单)
> pub struct L0SelfAction      { /* AI 自决级别操作 */ }
> pub struct L1MonitorAction   { /* 可观察级别操作 */ }
> pub struct L2CouncilAction   { /* 智囊团决议级别操作 */ }
> pub struct L3MultiSigAction  { /* 多人多签级别操作 */ }
> pub struct L4HardwareAction  { /* 硬件多签级别操作 */ }
> pub struct L5PhysicalAction  { /* 物理多签级别操作 */ }
> ```
>
> **D8-fix 关键变化**:
> - 错版 `DecisionSignature` 用单一 struct + enum 集中表达 14+ 守卫 → 不符合"两把独立锁"语义
> - D8-fix 用 5 + 6 = 11 个领域 Action struct, 每个 Action 由对应层子 trait 直接接收, 不需要"集中签名清单"
> - 这与阶段 2 §10 decision-system Phase 1 已落的"按层分发"设计精确对齐
> - 主 17:58 不假装: Action struct 字段 schema 由阶段 4 ADR 决定, 本节**不冻结**

### 3.4 守门映射表 (守护决策 → onion_wall/ 下的 trait 方法)

> **本节 §3.4 旧措辞 (错版历史轨迹, R14-D8 错版 Leader 提议)**: 10 行决策集中映射到 `onion_wall/` 子模块 + `OnionGate::guard_decision()` 统一入口**不删除**, 完整保留如下:
>
> | 阶段 1+2 沉淀决策 | DecisionCategory | 守门 trait 方法 | onion_wall/ 子模块 |
> |------------------|-----------------|----------------|-------------------|
> | 阶段 1 §18.6 双根变更 | `DoubleRoot` | `gate.guard_decision()` + `gate.guard_onion_zero()` | `gate.rs` + `ha.rs` |
> | 阶段 1 §18.7 双洋葱 | `DoubleOnion` | `gate.guard_decision()` | `gate.rs` + `principle.rs` + `permission.rs` |
> | 阶段 1 §18.5 三件套 | `PlatformTriad` | `gate.guard_decision()` + `audit.log()` | `gate.rs` + `audit.rs` |
> | 阶段 1 §18.8 七席审议庭 | `Council` | `gate.guard_decision()` + `council.trigger()` | `gate.rs` + `council.rs` |
> | 阶段 1 §18.9 L1-L5 验证 | `ValidationL1L5` | `validation.check()` | `validation.rs` |
> | 阶段 2 §8 MEWG | `MEWG` | `mewg.aggregate()` + `gate.guard_decision()` | `mewg.rs` + `gate.rs` |
> | 阶段 2 §9 HA | `HA` | `gate.guard_onion_zero()` + `ha.require()` | `ha.rs` |
> | 阶段 2 §10 旧规则 | `LegacyLegality` | `gate.guard_decision()` + `drift.check()` | `gate.rs` + `drift.rs` |
> | 阶段 2 §14 漂移 P0 | `DriftP0` | `drift.check()` + `gate.guard_decision()` | `drift.rs` + `gate.rs` |
> | 阶段 2 §12 9 键 (辅助) | (辅助语义网) | `keys.check()` | `keys.rs` |

> **D8-fix 新版 (主人 2026-07-31 纠偏后采纳)**: 决策按"原则洋葱层 / 权限洋葱层"分布到对应子 trait, 每个阶段 1+2 沉淀的决策映射到锁 A 子 trait 或锁 B 子 trait, **不再有 `OnionGate` 统一入口**:
>
> | 阶段 1+2 沉淀决策 | 归属锁 | 守门 trait 方法 (D8-fix) | 路径 |
> |------------------|-------|------------------------|------|
> | 阶段 1 §18.6 原则根变更 | **锁 A** | `principle_onion.check_e_layer()` | `onion/principle/e_layer.rs` (ELayerCheck) |
> | 阶段 1 §18.6 权限根变更 | **锁 B** | `permission_onion.check_l5_physical()` | `onion/permission/l5_physical.rs` (L5Physical) |
> | 阶段 1 §18.7 双洋葱 (R14-D7) | **两锁耦合** | `dispatcher.dispatch()` (同时过锁 A 全部子 + 锁 B 全部子) | `onion/dispatcher.rs` |
> | 阶段 1 §18.5 三件套"约束" | **锁 A** | `principle_onion.check_e_layer()` | `onion/principle/e_layer.rs` |
> | 阶段 1 §18.5 三件套"记录" | **dispatcher 收口** | `dispatcher.dispatch()` 返回的 `audit_ref` | `onion/dispatcher.rs` |
> | 阶段 1 §18.8 七席审议庭 | **锁 B** | `permission_onion.check_l2_council()` | `onion/permission/l2_council.rs` (L2Council) |
> | 阶段 1 §18.9 L1-L5 验证 | **锁 B** | `permission_onion.check_l1_monitor()` (L1 = 可观察 = 验证层基础) | `onion/permission/l1_monitor.rs` (L1Monitor) |
> | 阶段 2 §8 MEWG | **锁 B** | `permission_onion.check_l4_hardware()` (MEWG 校准由 L4 触发) | `onion/permission/l4_hardware.rs` + `onion/permission/mewg.rs` |
> | 阶段 2 §9 HA 硬门槛 | **HA 调度器** | `human_gate.requires_approval()` + `human_gate.trace_approval()` | `onion/human_gate.rs` |
> | 阶段 2 §10 旧规则合法性 | **锁 A** | `principle_onion.check_m_layer()` (旧规则 → 方法论层变更) | `onion/principle/m_layer.rs` (MLayer) |
> | 阶段 2 §14 漂移 P0 优先级 | **锁 A** | `principle_onion.check_m_layer()` (漂移检测由 MLayer 内部触发) | `onion/principle/drift.rs` |
> | 阶段 2 §12 9 键 (辅助语义网) | **锁 A** | `principle_onion.check_o_layer()` (9 键是 O 层守门辅助) | `onion/principle/o_layer.rs` + `onion/principle/keys.rs` |
>
> **D8-fix 关键区别**:
> - 错版所有决策都映射到 `OnionGate::guard_decision()` 统一入口 → 不符合"两把独立锁"语义
> - D8-fix 每个决策明确归属锁 A / 锁 B / HA / dispatcher 之一, 锁内部按子层 trait 分发
> - 这与阶段 2 §10 decision-system Phase 1 已落的"按层分发"设计精确对齐

---

## §4. 9 键 → 决策签名迁移 (仅列映射, 不重写 9 键)

> **本节性质 (主 17:58 不假装 + 主 17:43 实事求是)**: 列出 9 键到 `DecisionSignature` 的**映射关系**, 9 键字符串匹配**保留**作为辅助语义网, 不删除不重写。

> **D8-fix 主人纠正 (2026-07-31)**: §4 表头中的"9 键 → DecisionSignature 迁移"措辞错版——D8-fix 后 9 键归 `onion/principle/keys.rs` (锁 A OLayerGuard 内部), 不再迁移到集中签名 `DecisionSignature` (该 enum 已删除)。下方表格保留作为历史轨迹, 紧接其后追加 D8-fix 新版。

| V3 9 键 (从 philosophy crate 迁移到 `onion_wall/keys/`) | 对应 DecisionCategory | 迁移后角色 |
|------------------------------------------------|---------------------|----------|
| PHL-01: NotClone | (不映射到具体 category) | 辅助语义网 (语义层守门) |
| PHL-01: NotPerfect | `MEWG` (校准原则) | 辅助语义网 (KPI 校准提醒) |
| PHL-01: NotUuid | `DoubleRoot` / `DoubleOnion` | 辅助语义网 (唯一性提醒) |
| PHL-02b: NotUndo | (不映射) | 辅助语义网 (不可逆提醒) |
| PHL-02b: NotProof | (不映射) | 辅助语义网 (证明边界提醒) |
| PHL-02b: NotSafe | `HA` / `ValidationL1L5` | 辅助语义网 (安全边界提醒) |
| PHL-03: SpecIsNotProof | (不映射) | 辅助语义网 (规格 vs 证明提醒) |
| PHL-03: CounterexampleIsNotBug | (不映射) | 辅助语义网 (反例处理提醒) |
| PHL-03: ProverIsNotTruth | `Council` (七席独立性) | 辅助语义网 (独立性提醒) |

**迁移原则 (主 17:43 实事求是)**:
- 9 键的字符串匹配**保留**在 `onion_wall/keys/`, 但**不**是主入口
- 主入口是 `OnionGate::guard_decision(decision: &DecisionSignature)` 的**结构化校验**
- 9 键作为**辅助语义网**, 在结构化校验后做二次语义扫描, 防止结构化校验漏掉的"哲学陷阱" (例如 "I have proven" 这种语义违规)
- 9 键 → DecisionCategory 映射**不要求 1:1 覆盖**——9 键覆盖面 > DecisionCategory, 正常

> **D8-fix 迁移原则 (主 17:43 实事求是, 主人 2026-07-31 纠偏后采纳)**:
> - 9 键字符串匹配**保留**在 `onion/principle/keys.rs` (锁 A OLayerGuard 内部辅助), **不**是主入口
> - 锁 A 主入口是 `PrincipleOnion::check_o_layer()` (O 层子 trait), 9 键作为该子 trait 的辅助语义网
> - 9 键 → ELayerCheck / SLayerAudit / ALayer / MLayer / OLayerGuard 五个子 trait 映射**不要求 1:1 覆盖**——9 键覆盖面 > 5 子 trait, 正常
> - 错版"主入口是 `OnionGate::guard_decision()`" 措辞不删除 (上面保留), 仅作历史轨迹
> - 错版 `DecisionSignature` enum 已删除 (§3.3 D8-fix), 9 键 → enum 的映射关系失效, 但**映射意图保留** (映射意图 = "9 键不是凭空存在, 是辅助锁 A 的工具")

---

## §5. V0.5 / V1136 在 R14 的角色定位 (对照基线)

> **本节性质 (主 17:43 实事求是 + 主 17:58 不假装)**: 明确 V0.5 / V1136 在 R14 的角色 = **R11 对照基线**, **不重写不重做**。

### 5.1 V0.5 角色

| 项 | R11 现状 | R14 角色 (R14-D8 主人精化) |
|----|---------|--------------------------|
| **V0.5** = ASI 真测公式 (v1077) 17 维 | R11 已落 (Python 实现) | **R11 对照基线**, **不重写** |
| 公式结构 | v1077 V0.4 17 维真测 | **保持不变** |
| R14 用法 | — | Phase 2+ 性能对照 (Rust 重写后行为 1:1 验证) |
| 不做的事 | — | ❌ 不重写公式; ❌ 不加"自设指标"标注; ❌ 不刷 KPI |

### 5.2 V1136 角色

| 项 | R11 现状 | R14 角色 (R14-D8 主人精化) |
|----|---------|--------------------------|
| **V1136** = 连续性 + 可迁移性测度 (v1103 / v1106) | R11 已落 (Python 真测引擎) | **R11 对照基线**, **不重做** |
| 真测规则 | v1106 真实工程韧性基准 | **保持不变** |
| R14 用法 | — | Phase 2+ 性能对照 + 重写后行为 1:1 验证 (不刷 KPI) |
| 不做的事 | — | ❌ 不重做真测引擎; ❌ 不砍 0.05 KPI 装饰; ❌ 不刷 KPI |

### 5.3 与 crates/README 的对应 (R14-D8 已落实)

| crate | R14-D8 措辞 |
|-------|------------|
| `apeireth-asi` | ASI 北极星导向 + **借 R11 真测 (v1077/v1101) 作 baseline (不重写 V0.5 公式)** |
| `apeireth-bench` | 性能基准 (V1130 wallclock) + **借 R11 真测 (v1012/v1106) 作 baseline (不重做 V1136 真测引擎)** |

### 5.4 与 V0.5 / V1136 既有文档的关系

- `apeireth/v1077_*.py` / `apeireth/v1101_*.py` / `apeireth/v1103_*.py` / `apeireth/v1106_*.py` **不动** (R11 1100+ v*.py 保护约束)
- `Apeireth-rust/docs/rust-traits-spec-2026-07-30.md` §6 (V0.5/V1136 trait 草案) 中"重写公式 / 重做真测"措辞**保留为历史轨迹**, 但在 §6 顶部追加 R14-D8 勘误段 (本次任务范围之外, 由后续任务处理)
- 哲学守门 trait 框架 (`philosophy-traits-2026-07-30.md`) 中"重写 V0.5 公式 / 重做 V1136 真测引擎"措辞**保留为历史轨迹**, 已在文档头加 R14-D8 勘误段

---

## §6. 阶段 4 衔接锚点 (给后续 SCHEMA.md 写作参考)

### 6.1 给阶段 4 落实架构文档 (SCHEMA.md / ADR.md) 的衔接清单

- **trait 接口落地**: `OnionGate` / `DecisionSignature` / `DecisionVerdict` / `GuardTrace` 的完整 Rust trait 签名 (本节 §3.2-§3.3 给出了 stub, 阶段 4 补充实现签名)
- **子模块清单**: `onion_wall/{mod, gate, decision, keys, principle, permission, mewg, ha, council, validation, drift, audit, reflection}.rs` (本节 §3.1)
- **DB schema**: `decision_signatures` / `guard_traces` / `ha_approvals` / `council_votes` / `validation_results` / `drift_events` 表结构
- **crate 边界**: `apeireth-core` 内 `onion_wall/` 模块, 不再需要独立 `apeireth-philosophy` crate (R14-D8)

> **D8-fix 衔接清单 (主人 2026-07-31 纠偏后采纳, 阶段 4 SCHEMA.md / ADR.md 写作参考)**:
> - **trait 接口落地 (D8-fix)**: `PrincipleOnion` (5 子 trait) / `PermissionOnion` (6 子 trait) / `dispatcher::dispatch()` / `HumanGate::requires_approval()` + `trace_approval()` 完整 Rust trait 签名 (本节 §3.2 D8-fix 已给出 stub, 阶段 4 补充实现签名)
> - **子模块清单 (D8-fix)**: `onion/{mod, dispatcher, human_gate}.rs` + `onion/principle/{mod, e_layer, s_layer, a_layer, m_layer, o_layer, keys, drift}.rs` + `onion/permission/{mod, l0_self, l1_monitor, l2_council, l3_multisig, l4_hardware, l5_physical, mewg}.rs` (本节 §3.1 D8-fix 已给出目录树)
> - **DB schema (D8-fix)**: 删除 `decision_signatures` 集中表; 新增 `principle_layer_actions` (5 子层) + `permission_layer_actions` (6 子层) + `dispatcher_traces` + `human_approvals` 分表
> - **crate 边界 (D8-fix)**: `apeireth-core` 内 `onion/` 模块 (扁平, 含 principle/ + permission/ 两个子目录), 不再需要独立 `apeireth-philosophy` crate (R14-D8 + R14-D8-fix 一致)
- **cargo metadata**: `apeireth-core` 的 `description` 字段**不动** (与 crates/README "职责" 列一致)
- **crates/README 表格**: 已完成 R14-D8 措辞精化 (`apeireth-asi` / `apeireth-bench` / `apeireth-core` / `apeireth-philosophy` 四行 + §3 / §4 块)

### 6.2 给阶段 5 施工文档的衔接清单

- **HA 硬门槛落地**: 5 类 `HumanAuthorityVerifier` 实现 (WinHello / FIDO2 / MultiSig / OfflineSig / Recovery) — 见 `inspiration-stage1-2026-07-30.md` §21.3 (R14-D6-A 沉淀)
- **MEWG 校准**: 系数初始值 (待生产校准, **不冻结**) — 见 `stage2-decisions-addendum-sovereignty-continuity-governance.md` §8.4
- **Council 7 席**: 风险分级 → 席位触发矩阵 (critical=7/high=5/medium=3/low=1/info=0) — 见 `inspiration-stage1-2026-07-30.md` §20.3 (R14-D5-C A3 沉淀)
- **L1-L5 验证网**: 编译 / 运行时 / CI / 集成 / 反思期 — 见 `inspiration-stage1-2026-07-30.md` §20.4 (R14-D5-C A4 沉淀)
- **V0.5 / V1136 baseline 验证**: Phase 2+ 性能对照, 不刷 KPI

### 6.3 边界声明锚点

- **主 17:58 不假装**: 哲学守门**不是**独立 crate; V0.5 / V1136 **不是** R14 重设计对象; 9 键 trait 框架**保留**为历史轨迹
- **主 17:43 实事求是**: 走法乙的 3 细节是基于 R11 现状 + 主人洞察的精化, 不假装"以前没说错, 只是现在看得更清"
- **主 19:33 走在前人经验上**: 嵌套洋葱是经典的分层架构思想; per-layer 双重过滤借鉴权限模型的"AND 门"思路
- **主 22:33 ASI 北极星**: 洋葱核心 0 层 = 真实人类批准, 保留最后护栏
- **主 23:44 干到底**: 哲学守门 trait 框架 (9 键 + 5 项不假装) **保留**为历史轨迹, 不假装"哲学守门已改变"
- **主 00:56 任何人都能接手**: 本节 §1-§5 + crates/README 措辞精化 + 两份哲学守门文档勘误段 = 任何接手者能看清 R14-D8 演化脉络

### 6.4 不做的事清单 (主 17:58 不假装)

- ❌ 不写新 Rust 代码 (本节只描述 trait 签名, 不写实现)
- ❌ 不画 Mermaid 图 (用 ASCII 简化示意, 阶段 3 蓝图才画正式架构图)
- ❌ 不重写 V0.5 / V1136 / 哲学守门 9 键
- ❌ 不修改其他 16 份 stage2 文档
- ❌ 不修改 crates/ 占位实现
- ❌ 不修改 cargo metadata `description` 字段
- ❌ 不重写 `rust-traits-spec-2026-07-30.md` (本次任务范围之外, 由后续任务处理)
- ❌ 不重写 `stage2-decisions-philosophy-guard.md` 主 21KB 内容 (本次任务仅头部加勘误段)
- ❌ 不重写 `philosophy-traits-2026-07-30.md` 主 trait 框架 (本次任务仅头部加勘误段)

---

## §7. 主哲学 anchor 6 个全贯穿 (本节)

| 主哲学 anchor | 在本节中的体现 |
|--------------|--------------|
| **主 22:33 ASI 北极星 (S-1)** | §1.3 细节 2: 守护对象 = 阶段1+2 沉淀的具体决策; §2.2 洋葱 0 层 = 真实人类批准 (最后护栏) |
| **主 17:43 实事求是 (S-2)** | §1.2 走法乙 vs 走法甲对比表; §5 V0.5 / V1136 角色定位 (不重写不重做); §6.3 "不假装以前没说错" |
| **主 17:58 不假装 (O-5)** | §1.1 主人原话引用; §3.2 抽象层原则; §4 9 键保留为辅助语义网; §6.4 不做事清单 |
| **主 19:33 走在前人经验上 (O-2)** | §2.1-§2.2 嵌套洋葱 (经典分层架构); §2.2 per-layer 双重过滤 (借鉴权限 AND 门); §2.3 现有沉淀衔接 |
| **主 23:44 干到底 (O-3)** | §1.3 走法乙 3 细节完整沉淀; §3.1 子模块结构 (12 个子模块全覆盖); §6.1-§6.2 阶段 4-5 衔接锚点 |
| **主 00:56 任何人都能接手 (O-4)** | §0 范围 + §1 比喻起源 + §2 咬合形态 + §3 模块映射 + §6 衔接锚点 = 任何接手者能看清演化脉络 |

> **D8-fix 主人纠偏补充 (2026-07-31)**: §7 上表"§2 咬合形态"措辞错版, 已于 R14-D8-fix 改为"§2 双锁独立形态", 但**保留旧措辞作为历史轨迹** (主 17:58 不假装"以前没说错")。任何接手者读本文档时, 应同时看到错版 (R14-D8) + 新版 (R14-D8-fix) 两套结构化叙述 + 关键区别段, 自选语义层级演化脉络。

---

_R14-D8 城堡内墙洋葱架构已沉淀, 6 节 + 边界声明锚点 + 6 主哲学 anchor 全贯穿. 仅新增本概念文档 + crates/README 措辞精化 + 两份哲学守门文档头部勘误段, 不修改任何 stage2 主体内容, 不写代码, 不冻结架构. 下一步: 阶段 4 落实架构 (SCHEMA.md + ADR.md + trait 完整签名 + DB schema), 阶段 5 施工 (HA 5 类实现 + MEWG 校准 + Council 7 席触发 + L1-L5 验证 + V0.5/V1136 baseline 验证)._
