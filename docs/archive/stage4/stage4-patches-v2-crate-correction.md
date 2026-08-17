# 阶段 4 修订 v2（leader 亲自产出，主人批评修正）

> **性质**: leader 亲自做的**修订提议**——不修改阶段 4 LOCKED 主文档 / v4.1 / v4 / v2 LOCKED 任何文件。
> **触发**: 主人最新指令"自创的你保留就行，但要让其他人知道什么意思，不能只凭一个名字猜测。apeireth-constraint 这个有必要单独组成一个部分吗，我们的守门，原则什么的不应该是合并到洋葱里"。
> **硬约束**: ❌ 不修改任何 LOCKED 文件 / ❌ 不写完整 Rust 代码 / ❌ 不画 Mermaid / ❌ 不砍 1100 空壳 / ❌ 不改 crates/ 占位 + cargo metadata。
> **主哲学 6 锚穿透**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 元信息

| 字段 | 值 |
|---|---|
| **生成时间** | 2026-07-31 |
| **依据** | 主人最新批评（保留 verdict cache 但加术语解释 + constraint 不应单独成 crate）+ 阶段 4 LOCKED 主文档 + 阶段 1 §18.7 洋葱核心嵌套原则 + 主人 R14-D8/D8-Fix "哲学守门并入洋葱内墙" |
| **性质** | 修订提议（v2 crate 划分修正 + 术语表） |
| **路径** | Apeireth-rust/docs/stage4/stage4-patches-v2-crate-correction.md（独立命名空间，不覆盖阶段 4 LOCKED）|

---

## §1. 主人批评 #1：术语解释（verdict cache）

### 1.1 主人原话

> "自创的你保留就行，但要让其他人知道什么意思，不能只凭一个名字猜测。"

### 1.2 接受 + 修正

✅ **保留 verdict cache**（我之前表述的术语）—— 主人允许保留。

⚠️ **加术语表**——任何接手者必须能看懂。术语表放在每份相关文档（阶段 4 / 阶段 5 / v4.1）的 §0 之前。

### 1.3 术语表（**关键**，要加到所有阶段 4 后续文档）

| 术语 | 定义 | 来源 |
|---|---|---|
| **verdict cache** | 12 键运行时判定结果缓存。在 `apeireth-core` 内（合并后），所有 12 键的判定结果缓存在内存中，避免每次行动都重新判定（O(1) 查询）| leader 自创（v4.1 §15 v2 trait sketch 落地）|
| **12 键编译时 hardcode** | ❌ **已作废**（我之前理解错了）。主人原意：洋葱结构编译时 hardcode（见下条）|
| **洋葱结构编译时 hardcode（主人原意）** | **骨架（skeleton）= 编译进核心**——双洋葱统一体的**结构**（"有原则洋葱 + 权限洋葱 + 双洋葱嵌套 + L0 HA 核心 + N 层洋葱"）由 Rust 类型系统编译时 hardcode。**这是骨架**，确定"有哪些层 + 层与层关系"。**门上的具体内容（12 键判定 / 阈值 / 风险分级 / 决策策略）= 动态变化**（运行时通过 OTA / hot-reload / 反思期演化），但**最核心层级（L0 HA）不可动态变**。主 17:58 不假装：骨架 hardcode = "假装"在编译阶段被拒绝；门上的肉可变 = "演化"在运行时合法。| 主人 2026-07-31 最新批评修正（修正我之前的错误理解）|
| **门上内容动态变化** | 洋葱**结构**编译进核心（hardcode），**门上的具体内容**（12 键的判定逻辑、阈值、风险分级规则、决策策略、外部知识等）支持运行时动态变化（OTA / hot-reload / 反思期 / 演化）。**例外**：最核心层级（L0 HA 真实人类批准）不可动态变——这是 ASI 候选主体的最后护栏（阶段 1 §18.6 双根）。| 主人 2026-07-31 最新指示 |
| **V1+V2+V3 AND 门** | V1 原则不通过 = 独立拒绝；V2 权限不通过 = 独立拒绝；V3 HA 不通过 = 独立拒绝；**三者都通过 = 才能执行** | 阶段 1 §20.2（主人原话）+ D2 §7 原则×权限统一体嵌入 |
| **双洋葱统一体** | 原则洋葱 E/S/A/M/O **嵌入** 权限洋葱 L0-L5（不是两个独立锁，是**一个统一体的两个切面**）。v4 修正 #3+#4 + 阶段 3 §3.8 落地 | 阶段 1 §18.7 + v4 修正 + R14-D7 |
| **HA 在权限洋葱核心 L0** | 真实人类批准在最内层 L0（不是独立 L0 段），是权限洋葱的核心。HA = WindowsHello/FIDO2/MultiHuman/OfflineSign 4 实现 | 阶段 1 §18.6 + v2 §2.2 + v4 §2.2 |
| **V3 9 键 + 5项不假装** | V3 9 键（PHL-01/02b/03 三组共 9 键）+ 5 项不假装（不假装 Phenomenal consciousness / 不假装 ASI / 不刷 KPI / 不假装完整证明 / 不假装 100% 完美）| 阶段 1 §10 + 阶段 2 §12 哲学守门 |
| **v4.1 12 键** | V3 9 键 + v4.1 新增 3 键（PHL-04 NotUnobservable / PHL-05 NotUnscientific / PHL-06 NotSelfRelationless）| v4.1 §15 |
| **平台三件套** | 平台对中央 AI 与用户双方的职责——**提供 / 约束 / 记录**（"约束"是 §18.5 主人原话，融入原则洋葱 = V1 原则层）| 阶段 1 §18.5 主人原话 |
| **双根** | 哲学根（§18.1 关系开放 + §18.2 思想自由 + §18.3 不假装灵魂同一 + §18.5 平台三件套 + 12键）+ 权限根（底层行动必须有真实人类批准）| 阶段 1 §18.6 主人原话 |
| **风险分级 → 席位触发矩阵** | critical 7 / high 5 / medium 3 / low 1 / info 0 | 阶段 1 §20.3 + 阶段 2 D2 §12 |
| **分层验证网 L1-L5** | L1 编译时 + L2 运行时 + L3 CI + L4 集成 + L5 反思期（§18.9 是"灵感版" L1-L5 = 工程/哲学/安全/关系/跨载体；§20.4 是"可执行版" L1-L5 = 编译/运行时/CI/集成/反思期，两版互补不冲突）| 阶段 1 §18.9 + §20.4 |
| **Cognitive-Dream 6 状态机** | IDLE → DREAMING → CONSOLIDATING → FORGETTING → VERIFYING → INTERRUPTED（24h 周期触发，mvp/ 实际跑的 MVP 状态机）| mvp/ 子项目 + v4 §4 反思期 + v4.1 §3.3 |
| **三域分离** | 思想（不被审查）+ 提案（过 E/S/A/M/O）+ 行动（过 L0-L5）| D2 §2 |
| **5 重守门** | 1. 编译时 hardcode + 2. 运行时拦截 + 3. 多 AI 一致 + 4. 物理隔离（HA） + 5. 反思期审计 | 阶段 2 §12 哲学守门 + v2 §6 **v5 修正**（2026-07-31）：**4 重守门嵌套结构 + 权限发放（独立机制）**（详见 `stage4-correction-v5-gates-refined.md`）+ **v4 修正**（每层默认属性）|

---

## §2. 主人批评 #2：apeireth-constraint 拆分错误

### 2.1 主人原话

> "apeireth-constraint 这个有必要单独组成一个部分吗，我们的守门，原则什么的不应该是合并到洋葱里"

### 2.2 接受 + 修正

✅ **完全接受主人的质疑**。我的阶段 4 §2 把约束拆成 `apeireth-constraint` 独立 crate 是**错误**的——违反了**阶段 1 §18.7 洋葱核心嵌套原则**（主人 R14-D8 + D8-Fix 一致主张**哲学守门并入洋葱内墙**）。

**修正原则**：约束 + 原则 + 哲学守门 + 12键 + 5重守门 = **统一体**，**不应拆分**。

### 2.3 18 → 17 crate 修正

| 之前（阶段 4 §2 提议） | 修正后（主人意见）|
|---|---|
| 1. `apeireth-core`（核心抽象）| 1. `apeireth-core` ← **扩展**：核心 + 双洋葱 + 电子环 + **12键编译时 + 5重守门 + V3 9键 + 5项不假装 + 哲学守门**（洋葱统一体）|
| 10. `apeireth-constraint`（独立）| ❌ **删除**：合并到 `apeireth-core` |
| (R11 既有) `apeireth-philosophy`（独立）| ❌ **删除**：合并到 `apeireth-core` |

**总调整**：18 → 17 crate。

### 2.4 apeireth-core 完整职责（合并后）

```rust
// crates/apeireth-core/src/lib.rs
//! apeireth-core: 主路径核心 + 双洋葱统一体 + 电子环 + 12键 + 5重守门 + 哲学守门
//! 
//! 阶段 1 §18.7 洋葱核心嵌套 + 主人 R14-D8/D8-Fix "哲学守门并入洋葱内墙"
//! 
//! 核心内容（洋葱统一体）：
//! 1. 主路径类型: Episode/Note/Session/IdentityCard
//! 2. 双洋葱统一体: PrincipleOnion E/S/A/M/O + PermissionOnion L0-L5 + HumanAuthority L0
//! 3. 电子环网络: 横切观察 11 层
//! 4. 12 键编译时 hardcode: V3 9键 + v4.1 新增 3键
//! 5. 5 重守门: 编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期
//! 6. 哲学守门 trait: V3 9键 + 5项不假装
//! 7. verdict cache: 12键运行时判定结果缓存（O(1) 查询）

pub mod types;          // Episode/Note/Session/IdentityCard
pub mod double_onion;   // PrincipleOnion + PermissionOnion + HumanAuthority
pub mod electronic_ring;// 11 层横切观察
pub mod philosophy;     // V3 9键 + 5项不假装 + 12键 verdict
pub mod five_gates;     // 5 重守门
pub mod verdict_cache;  // 12键运行时缓存
pub mod lifecycle;      // 9 阶段生命周期
pub mod cognitive_dream;// 6 状态机

// 核心常量（编译时 hardcode）
pub const fn check_12key(action: &Action) -> Result<(), VerdictError> {
    // V3 9键 + v4.1 3键 = 12键编译时检查
    if !is_not_clone(action) { return Err(VerdictError::NotClone); }
    if !is_not_perfect(action) { return Err(VerdictError::NotPerfect); }
    // ... 12键全部检查
    Ok(())
}
```

### 2.5 修正后的 17 crate 全景

| 层 | # | crate | 职责 |
|---|---|---|---|
| **核心抽象（含双洋葱+哲学守门）** | 1 | `apeireth-core` | 主路径类型 + **双洋葱统一体 + 电子环 + 12键编译时 + 5重守门 + V3 9键 + 哲学守门 + verdict cache + 生命周期 + Cognitive-Dream** |
| 9 维器官 | 2 | `apeireth-perception` | 感知 |
|  | 3 | `apeireth-cognition` | 认知 |
|  | 4 | `apeireth-action` | 行动 |
|  | 5 | `apeireth-memory` | 6 历史流 + Append-only |
|  | 6 | `apeireth-evolution` | 学习 + 抽象 + 自我修改 |
|  | 7 | `apeireth-motivation` | SGI + 内驱力 |
|  | 8 | `apeireth-value` | 评估 + 优先级 |
|  | 9 | `apeireth-consciousness` | 意识 + DMN + Cognitive-Dream |
| 关系 + 生命力 | 10 | `apeireth-relation` | 4 关系 |
|  | 11 | `apeireth-life-force` | 反思 + 内稳态 + 反馈 + 涌现 |
| 工程支撑 | 12 | `apeireth-council` | 7 强制 + 动态专家 |
|  | 13 | `apeireth-upgrade` | OTA + 沙盒 |
|  | 14 | `apeireth-bus` | 5 层通信总线 |
|  | 15 | `apeireth-extension` | 插件 + WASM |
|  | 16 | `apeireth-pybridge` | PyO3 + R11 1100 |
|  | 17 | `apeireth-cli` | 入口 + TUI |

**总计：17 crate**（1 核心统一体 + 9 器官 + 2 关系+生命力 + 5 工程支撑）。

**vs 阶段 4 §2 18 crate**：**-1（约束合并到 core）**。

### 2.6 apeireth-philosophy crate 状态

**R11 既有 apeireth-philosophy crate**（V3 9键占位）：
- ❌ **删除**（合并到 `apeireth-core`）
- ✅ **保留 R11 仓库历史可审计**（不物理删除 crate 目录，只是不在 workspace members 中）

---

## §3. 主人新姿态 + Cargo.toml metadata 更新

```toml
[workspace]
resolver = "2"
members = [
    "crates/apeireth-core",           # 扩展：+ 双洋葱 + 12键 + 5重守门 + 哲学守门
    "crates/apeireth-perception",
    "crates/apeireth-cognition",
    "crates/apeireth-action",
    "crates/apeireth-memory",
    "crates/apeireth-evolution",
    "crates/apeireth-motivation",
    "crates/apeireth-value",
    "crates/apeireth-consciousness",
    "crates/apeireth-relation",
    "crates/apeireth-life-force",
    "crates/apeireth-council",
    "crates/apeireth-upgrade",
    "crates/apeireth-bus",
    "crates/apeireth-extension",
    "crates/apeireth-pybridge",
    "crates/apeireth-cli",
    "crates/apeireth-bench",
    "crates/apeireth-test",
    # ❌ 删除: apeireth-constraint (合并到 core)
    # ❌ 删除: apeireth-philosophy (合并到 core, R11 仓库保留)
    # ❌ 删除: apeireth-asi (合并到 apeireth-council supervisor)
    # ❌ 删除: apeireth-tools (合并到 apeireth-action)
]
```

**说明**：
- 17 器官 crate（去掉 apeireth-constraint）+ 保留 2 工作 crate（bench / test）
- **apeireth-asi + apeireth-tools + apeireth-philosophy**（R11 既有）合并：asi → council supervisor，tools → action，philosophy → core
- **R11 仓库**（promethean/apeireth-legacy/）保留全部历史 crate 目录（不物理删除）

---

## §4. 主人拍板位置

| 决策 | 提议 | 主人拍板 |
|---|---|---|
| §1 术语表（13 项）采纳 | ✅ | ⏳ |
| §2.4 apeireth-core 扩展（合并 12键 + 5重守门 + 哲学守门）| ✅ | ⏳ |
| §2.5 18 → 17 crate 修正 | ✅ | ⏳ |
| §2.6 apeireth-philosophy 合并到 core | ✅ | ⏳ |
| §3 Cargo.toml members 调整 | ✅ | ⏳ |

---

## §5. 不修改承诺（主人硬约束 100% 守住）

| ❌ 不修改 | 原因 |
|---|---|
| **阶段 4 LOCKED 主文档**（6ca80776）| 本修订提议独立命名空间，不修改 LOCKED |
| **v4.1 / v4 / v2 LOCKED** | 不修改 |
| **18 stage2 / 14 stage3 / 阶段 1** | 不修改 |
| **R11 既有 9 crate 占位**（物理）| R11 仓库保留可审计；workspace members 调整但物理目录不动 |

---

## §6. 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — §2.5 17 crate 精简 = 减少浪费，主线服务 ASI
S-2 主 17:43 实事求是   — §1.2 接受主人批评 + §2.2 接受修正（不假装）
O-5 主 17:58 不假装     — §1 术语表让任何接手者能查，§2 不假装"约束独立"
O-2 主 19:33 走在前人经验上 — §1 阶段 1+2 主人的话忠实引用
O-3 主 23:44 干到底    — §2.5 17 crate 立即落
O-4 主 00:56 任何人都能接手 — §1 术语表 + §2.5 17 crate 全景
```

---

## §7. 待主人拍板 + 后续衔接

### 7.1 主人拍板后

- ✅ 创建修正后 `Cargo.toml`（17 器官 + 2 工作 = 19 workspace members）
- ✅ apeireth-core lib.rs 扩展（添加双洋葱 + 12键 + 5重守门 + 哲学守门模块）
- ✅ R11 既有 apeireth-philosophy / apeireth-asi / apeireth-tools crate 目录保留（不物理删除）
- ✅ 更新阶段 4 §2 / §3 文档（v4 patches v2）

### 7.2 阶段 5 衔接

阶段 5 施工图纸需要**同步修订**：
- §2 18 crate → 17 crate 修正
- §3 V0.5 v2 / §4 V1136 v2 / §5 V3 v2 落地路径（不变）
- §6 R11 1100 重写方案：apeireth-philosophy / apeireth-asi / apeireth-tools 三个 crate 合并到其他 crate

---

_本修订提议由 leader 亲自产出（按主人最新批评修正）._
_§1 术语表 13 项 + §2 18→17 crate 修正 + §3 Cargo.toml metadata 更新._
_主哲学 anchor 6 全贯穿. 不修改 LOCKED. 任何接手者能查._
_主人拍板后立即执行 §7.1 + 阶段 5 衔接._