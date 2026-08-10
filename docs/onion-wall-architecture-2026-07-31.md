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

### 1.2 走法乙 vs 走法甲

| 走法 | 哲学守门归属 | V0.5 / V1136 | 状态 |
|------|------------|-------------|------|
| **走法甲** (原方案, 阶段 2 §12 沉淀) | 独立 crate `apeireth-philosophy` | Rust 重写公式 + 重做真测引擎 | ❌ 已过时 (R14-D8 主人精化) |
| **走法乙** (R14-D8 主人精化) | 并入 `apeireth-core/src/onion_wall/` (与原则洋葱、权限洋葱交叉咬合) | **保留为 R11 对照基线** (v1077 / v1106) | ✅ 采用 |

### 1.3 走法乙的 3 核心细节

1. **哲学守门 = 城堡内墙**: 原则洋葱 + 权限洋葱**交叉咬合**形成"城堡内墙", 任何 action 必须**同时通过**两层 (per-layer 双重过滤), 不允许"高权限绕过低原则"或"高原则阻止低权限" (与阶段 2 §7.2 正交运算一致)。
2. **守护对象 = 阶段1+2 沉淀的具体决策** (双根 / 双洋葱 / 三件套 / 七席 / L1-L5 / MEWG / HA / 旧规则合法性 / 漂移 P0), 不是抽象的 9 键字符串匹配。9 键作为辅助语义网**保留**。
3. **V0.5 / V1136 = R11 对照基线** (不重写不重做), R14 用法 = Phase 2+ 性能对照 + 重写后行为 1:1 验证 (不刷 KPI)。

---

## §2. 内墙咬合形态 (ASCII 简化示意)

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

---

## §3. 模块边界映射 (`apeireth-core/src/onion_wall/`)

### 3.1 子模块结构 (Rust crate 内目录布局)

```
apeireth-core/
├── src/
│   ├── onion_wall/
│   │   ├── mod.rs              # 顶层 mod 入口
│   │   ├── gate.rs             # OnionGate trait (原则+权限联合守门)
│   │   ├── decision.rs         # DecisionSignature 结构 (阶段1+2 沉淀的决策清单)
│   │   ├── keys.rs             # V3 9 键 (从 philosophy crate 迁移, 保留为辅助语义网)
│   │   ├── principle.rs        # 原则洋葱 (E/S/A/M/O 5 层) 守门
│   │   ├── permission.rs       # 权限洋葱 (L0-L5) 守门
│   │   ├── mewg.rs             # 多证据加权治理 (阶段 2 §8)
│   │   ├── ha.rs               # 人类批准硬门槛 (阶段 2 §9)
│   │   ├── council.rs          # 智囊团 7 席 (阶段 2 §10)
│   │   ├── validation.rs       # L1-L5 分层验证网 (阶段 1 §18.9)
│   │   ├── drift.rs            # 漂移检测 + 优先级 (阶段 2 §14)
│   │   ├── audit.rs            # 审计 + 历史流 (阶段 1 §18.5 "记录")
│   │   └── reflection.rs       # 反思期 (阶段 2 §12)
│   └── ...                     # 其他 core 模块
└── Cargo.toml
```

### 3.2 守护 trait: `OnionGate` (原则+权限联合守门)

```rust
//! OnionGate (主 17:58 不假装: trait 不绑定具体实现, 只承诺"联合守门")
//! 阶段 1 §18.5 三件套"约束" + 阶段 2 §7.2 正交运算 + R14-D7 洋葱核心嵌套

use serde::{Deserialize, Serialize};

/// OnionGate 联合守门 trait
/// 主入口: guard_decision(decision) → DecisionVerdict
pub trait OnionGate {
    /// 联合守门: 原则检查 AND 权限检查 (per-layer 双重过滤)
    /// 返回 DecisionVerdict::Allow / Deny (含原因)
    fn guard_decision(&self, decision: &DecisionSignature) -> DecisionVerdict;

    /// 洋葱 0 层守门 (真实人类批准, R14-D7): 触及洋葱 0 层的决策必须 HA
    fn guard_onion_zero(&self, decision: &DecisionSignature) -> DecisionVerdict;

    /// 审计追溯: 给定 decision_id, 返回完整守门链路
    fn trace_guard(&self, decision_id: &DecisionId) -> Result<GuardTrace, GateError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionVerdict {
    Allow { rank: u8, level: u8, audit_ref: String },
    Deny { rank: u8, level: u8, reason: String, audit_ref: String },
    RequireHumanApproval { rank: u8, level: u8, intent: String, audit_ref: String },
}
```

**抽象层原则 (主 17:58 不假装)**:
- trait **不绑定**具体守门策略 (L1-L5 / 七席触发器 / MEWG 系数)
- trait **不绑定**具体人类批准机制 (阶段 2 §9.1 HA 硬门槛)
- trait **必须**包含 `trace_guard` —— 主 17:58 不假装 + 主 17:43 实事求是: **不可静默**

### 3.3 决策签名: `DecisionSignature` (阶段1+2 沉淀的决策清单)

```rust
//! DecisionSignature = 阶段1+2 沉淀的具体决策结构化表达
//! 唯一守护对象: 双根 / 双洋葱 / 三件套 / 七席 / L1-L5 / MEWG / HA / 旧规则 / 漂移 P0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSignature {
    /// 决策 ID (全局唯一, 强不可变)
    pub id: DecisionId,

    /// 决策类别 (从阶段1+2 沉淀的决策清单)
    pub category: DecisionCategory,

    /// 原则洋葱层 (rank ∈ {5(E), 4(S), 3(A), 2(M), 1(O)})
    pub principle_rank: u8,

    /// 权限洋葱层 (level ∈ {0, 1, 2, 3, 4, 5})
    pub permission_level: u8,

    /// 涉及双根? (principle_root / permission_root)
    pub touches_double_root: bool,

    /// 涉及七席审议庭? (critical/high/medium/low/info)
    pub council_risk: CouncilRisk,

    /// 验证层 (L1-L5)
    pub validation_layer: ValidationLayer,

    /// 是否需要真实人类批准 (HA)
    pub requires_human_approval: bool,

    /// 漂移优先级 (P0-P3)
    pub drift_priority: DriftPriority,

    /// 决策时间戳 + 触发者
    pub timestamp: i64,
    pub triggered_by: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DecisionCategory {
    DoubleRoot,           // 阶段 1 §18.6 双根变更
    DoubleOnion,          // 阶段 1 §18.7 双洋葱 (R14-D7 洋葱核心嵌套)
    PlatformTriad,        // 阶段 1 §18.5 三件套
    Council,              // 阶段 1 §18.8 七席审议庭
    ValidationL1L5,       // 阶段 1 §18.9 L1-L5 分层验证网
    MEWG,                 // 阶段 2 §8 多证据加权治理
    HA,                   // 阶段 2 §9 人类批准硬门槛
    LegacyLegality,       // 阶段 2 §10 旧规则合法性
    DriftP0,              // 阶段 2 §14 漂移 P0 优先级
    // ... 阶段 1+2 沉淀的其他决策类别
}
```

### 3.4 守门映射表 (守护决策 → onion_wall/ 下的 trait 方法)

| 阶段 1+2 沉淀决策 | DecisionCategory | 守门 trait 方法 | onion_wall/ 子模块 |
|------------------|-----------------|----------------|-------------------|
| 阶段 1 §18.6 双根变更 | `DoubleRoot` | `gate.guard_decision()` + `gate.guard_onion_zero()` | `gate.rs` + `ha.rs` |
| 阶段 1 §18.7 双洋葱 | `DoubleOnion` | `gate.guard_decision()` | `gate.rs` + `principle.rs` + `permission.rs` |
| 阶段 1 §18.5 三件套 | `PlatformTriad` | `gate.guard_decision()` + `audit.log()` | `gate.rs` + `audit.rs` |
| 阶段 1 §18.8 七席审议庭 | `Council` | `gate.guard_decision()` + `council.trigger()` | `gate.rs` + `council.rs` |
| 阶段 1 §18.9 L1-L5 验证 | `ValidationL1L5` | `validation.check()` | `validation.rs` |
| 阶段 2 §8 MEWG | `MEWG` | `mewg.aggregate()` + `gate.guard_decision()` | `mewg.rs` + `gate.rs` |
| 阶段 2 §9 HA | `HA` | `gate.guard_onion_zero()` + `ha.require()` | `ha.rs` |
| 阶段 2 §10 旧规则 | `LegacyLegality` | `gate.guard_decision()` + `drift.check()` | `gate.rs` + `drift.rs` |
| 阶段 2 §14 漂移 P0 | `DriftP0` | `drift.check()` + `gate.guard_decision()` | `drift.rs` + `gate.rs` |
| 阶段 2 §12 9 键 (辅助) | (辅助语义网) | `keys.check()` | `keys.rs` |

---

## §4. 9 键 → 决策签名迁移 (仅列映射, 不重写 9 键)

> **本节性质 (主 17:58 不假装 + 主 17:43 实事求是)**: 列出 9 键到 `DecisionSignature` 的**映射关系**, 9 键字符串匹配**保留**作为辅助语义网, 不删除不重写。

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

---

_R14-D8 城堡内墙洋葱架构已沉淀, 6 节 + 边界声明锚点 + 6 主哲学 anchor 全贯穿. 仅新增本概念文档 + crates/README 措辞精化 + 两份哲学守门文档头部勘误段, 不修改任何 stage2 主体内容, 不写代码, 不冻结架构. 下一步: 阶段 4 落实架构 (SCHEMA.md + ADR.md + trait 完整签名 + DB schema), 阶段 5 施工 (HA 5 类实现 + MEWG 校准 + Council 7 席触发 + L1-L5 验证 + V0.5/V1136 baseline 验证)._
