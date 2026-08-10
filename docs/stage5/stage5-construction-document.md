> ## 📌 V28.0 跨配置零基线 + 1563 tests + V23 fail-forward + M1-M12 + V-Measure 24 维真实测量 2026-08-03（补充式修正·不动 LOCKED 原文）
>
> **Verification Status**（2026-08-03 round12-08 architect2）：
> - `cargo build --workspace`（默认 features）→ **0 errors**（冷构建 + 增量构建均通过）
> - `cargo test --workspace`（默认 features）→ **1563 passed / 0 failed / 0 ignored**（100 个 test binary）
> - `cargo test -p apeireth-constraint --lib --tests` → **M1-M12 12 场景测试全部 passed**（FiveGates 真实场景覆盖 + 跨 crate 集成）
> - `cargo test -p apeireth-asi --lib --tests` → **V-Measure 24 维 + 9 子测度真实测量函数实装**（qa_engineer round10-12 `a83be7fe`）
> - **V23 fail-forward 安全**（user audit 第 4 次）：integration-worktree 单分支流程 + 显式 refspec push (`local:remote-ref`)，不再 silent no-op
>
> **本阶段新增**：
> - **V27.0** (round10-08 qa_engineer `aa018af8`)：PyBridge 双配置功能对等 (cross_config_isomorphism.rs + 12 keys round10-07 integration)
> - **V27.1** (round10-10 architect2 `fbe2db5d`)：OTA 3 阶段跨 crate 真实 governance 集成 (cross_crate.rs + 16 集成测试)
> - **V27.2** (round10-11 architect2 `ff788b63`)：force-push stuck commits 报告 (force-push multiple stuck commits)
> - **V28.0** (round12-02 security_reviewer `ff6add0b`)：FiveGates M1-M12 真实场景 24 测试 + 跨 crate 集成 (round11 retry)
> - **V28.0-audit** (round99 architect2 `3e691795`)：综合审计 87 项 LOCKED vs 实装矩阵
>
> **本节性质**：✅ **补充式修正原则**——仅在文档顶部追加 V28.0 状态头，**未修改**下方任何 LOCKED 内容（V26.5 状态头 + §0–§N 仍为 2026-07-31 LOCKED 版本，"❌ 不修改任何 LOCKED 文档内容"硬约束完全遵守）。
>
> **历史脉络**：V26.2 backend_engineer2 → V26.4 round9-07 → V26.5 round10-05/06 → V27.0 round10-08 qa_engineer → V27.1 round10-10 architect2 → V27.2 round10-11 architect2 → V28.0 round12-02 security_reviewer → V28.0-audit round99 architect2 → **本盖章 round12-08**。
>
> **下一阶段**：V28.1 = stage6 22-trait 互锁代码实装 + ADR 0003-0006 补齐（round99 审计建议）。

> ## 📌 V26.5 双配置零基线达成 2026-08-02（补充式修正·不动 LOCKED 原文）
>
> **Verification Status**（2026-08-02 round10-05）：
> - `cargo clean && cargo build --workspace`（默认 features）→ **0 errors**（19.97s cold build）
> - `cargo build --workspace --features apeireth-pybridge/python-ext` → **0 errors**（6.43s，feature 启用 PyO3 extension-module）
> - `cargo test --workspace --lib --tests`（默认）→ **1372 passed / 0 failed / 0 ignored**
> - `cargo test --workspace --lib --tests --features apeireth-pybridge/python-ext` → **1372 passed / 0 failed / 0 ignored**
> - `cargo clippy --workspace --all-targets`（默认 + feature 双配置）→ **0 errors**（26 pre-existing warnings 范围不变，DEF-V26.4-001..009 仍适用，**0 新增 DEF**）
>
> **政策反转落地**（用户 21:13）：PyBridge 不再物理删除，改为 **feature-gated workspace member**（`python-ext` 默认关闭，仅在显式启用时编译 `pyo3/extension-module`）。
>
> **本节性质**：✅ **补充式修正原则**——仅在文档顶部追加状态头，**未修改**下方任何 LOCKED 内容（"❌ 不修改任何 LOCKED 文档内容"硬约束完全遵守）。下方 §0–§N 仍为 2026-07-31 LOCKED 版本。
>
> **历史脉络**：V26.2 backend_engineer2 → V26.4 round9-07 (commit `3cc2afe5` + `6cfc8374`，修复 DEF-V26.3-002 walk_all_crates 6 errors + verify clippy stub) → V26.5 round10-05 (纯验证，0 new commit，feature-gating 双 0-error 端到端达成) → **本盖章 round10-06**。
>
> **下一阶段**：V27.0 = 跨配置功能对等（PyBridge binding 集成测试 + 双配置行为同构验证）。

---

# 阶段 5 — 设计施工文档（施工图纸，Construction Blueprint）🔒 LOCKED

> **状态**: 🔒 **LOCKED**（2026-07-31，主人拍板"施工文档修好之后也 LOCKED"）
> **v6 修正**（2026-07-31 落地）：
> - §2 标题"9 → 17 crate 重写（按阶段 4 §2 + v6 完整版）"
> - §9 标题"4 重守门嵌套 + 权限发放 + E 层修改路径（v6 完整版 2026-07-31）"
> **依据**: v6 完整版（`stage4-correction-v6-consolidated-and-e-layer-mutation.md`）+ 4 件套 + GLOSSARY v6 + ROADMAP + examples/hello_world.rs + 集成测试
> **下一阶段**: 真正开干 Week 1（不是更多文档）
> **性质**: R14 阶段 5 = **施工图纸**（design施工文档）。**leader 亲自产出**（按主人"亲自做"要求 + "工程师/科学家思维从本源重构" + "参照其他优秀项目寻找灵感（不适配不硬融）" + "阅读阶段 1+2+3+4 不迷失"）。
> **前置**: 阶段 1（灵感 LOCKED）+ 阶段 2（18 stage2 + D2 增补 LOCKED）+ 阶段 3（v2 LOCKED + 14 stage3 LOCKED）+ 阶段 4（1492 行 LOCKED + 4 份补丁）+ v4.1（哲学层升级 LOCKED）+ v4（哲学层纲领 LOCKED）+ STRUCTURE-R14.md 规整方案。
> **硬约束**: ❌ 不修改任何 LOCKED 文档内容 / ❌ 不写**完整** Rust 代码（仅 sketch + 实施步骤）/ ❌ 不画 Mermaid（用 ASCII）/ ❌ 不砍 R11 1100+（保留 + 归档）/ ✅ **主人新姿态**：阶段 1+2+3 LOCKED，但 V0.5/V1136/9键 / R11 1100 / crates 占位 / Cargo.toml metadata **可改**（架构都更新了凭啥这个不能更新）。
> **主哲学 anchor 6 全贯穿**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 元信息

| 字段 | 值 |
|---|---|
| **生成时间** | 2026-07-31 |
| **任务** | R14 阶段 5 = 施工图纸（leader 亲自产出）|
| **路径** | `Apeireth-rust/docs/stage5/stage5-construction-document.md`（独立命名空间，不覆盖任何 LOCKED）|
| **依据** | 阶段 1+2+3+4 全部 + v4 + v4.1 + inspiration-supplements.md（20 优秀项目 + 10 元原则）+ patches.md（5+5 补丁）|
| **施工对象** | 9 crates/ 占位 + V0.5/V1136/9键 + R11 1100 + Cargo.toml metadata |
| **下一阶段** | 阶段 6 = 设计里程碑式验证机制 |

---

## §1. 第一性原理：什么是"施工"？（工程师/科学家思维从本源）

### 1.1 软件施工的本质

从本源看，**"施工"≠ "写代码"**。施工是**把设计图变成可运行系统**的全过程：

| 软件工程视角 | 第一性原理 |
|---|---|
| 写代码 | 把抽象符号编译为机器指令 |
| 编译 | 类型检查 + 优化 + 链接 + 输出二进制 |
| 测试 | 验证机器指令在真实环境中行为符合设计 |
| 部署 | 把二进制放到目标运行环境 |
| 运行 | 让机器指令在硬件上执行，处理真实数据 |
| 演化 | 在运行中持续修复 / 升级 / 扩展 |

**关键洞察**：施工 = 把**抽象设计**（类型 / trait / 状态机）变成**真实行为**（运行 / 监控 / 演化）。

### 1.2 Apeireth 施工的特殊性

Apeireth 是**有生命的智能体**，不是普通软件。施工有 3 大特殊性：

1. **活系统的施工**：每个器官（9 维）是**活的**——施工后器官会演化（不是静态部署）
2. **哲学守门的施工**：12 键编译时 hardcode——施工不是"写代码"，是"让类型系统强制不假装"
3. **跨载体的施工**：施工的对象是**跨载体的连续性 ID**——施工后系统可以从一个载体迁移到另一个载体

### 1.3 施工的 7 大原则（从本源推导）

| # | 原则 | 含义 | 来源 |
|---|---|---|---|
| **P1** | **编译时正确性优先** | 让尽可能多的检查在编译时完成（不是运行时）| Rust 6 大编译时约束 + 12 键 |
| **P2** | **零成本抽象** | 抽象不应该有运行时开销（trait 编译期单态化）| Rust 零成本抽象 |
| **P3** | **跨载体一致性** | 施工要保证 ID 连续性 / 记忆一致性 / 关系一致性 | v4 §18.3 不假装灵魂同一 |
| **P4** | **演化优先于稳定** | 系统会演化，施工要支持平滑升级（OTA + 沙盒 + 蓝绿）| M7 自创生 + M9 涌现 |
| **P5** | **观察优先于预测** | 施工时不能"假装"（主 17:58）——所有行为必须可观测 | M10 类型即契约 + V3 9 键 PHL-04 |
| **P6** | **失败透明** | 失败要透明地暴露给上层（异常 + 反思期）| M1 监督者不预防失败 + Erlang/OTP |
| **P7** | **人类最终权威** | L0 真实人类批准——施工不能绕过 HA | D2 §9 HA 硬门槛 + §18.6 五重治理 |

---

## §2. 9 → 17 crate 重写（按阶段 4 §2 + v6 完整版）

> **v6 修正**（2026-07-31）：9 → 17 crate 本源推导 + v6 完整版 = 4 重守门 + 权限发放 + 5 重治理 + E 层修改路径。
> 详见解锁：`stage4-correction-v6-consolidated-and-e-layer-mutation.md`

### 2.1 当前状态 vs 目标

**当前 9 crate 占位**（R11 已落）：
- `apeireth-core` (Episode/Note/Session/IdentityCard) ← 主路径核心
- `apeireth-memory` (6 历史流占位)
- `apeireth-asi` (中央 AI 占位)
- `apeireth-philosophy` (V3 9 键占位)
- `apeireth-pybridge` (PyO3 桥占位)
- `apeireth-tools` (工具占位)
- `apeireth-cli` (CLI 入口)
- `apeireth-bench` (性能基准)
- `apeireth-test` (测试)

**目标 18 crate**（阶段 4 §2 本源推导）：

| 层 | # | crate | 来源 | 现状 | 施工动作 |
|---|---|---|---|---|---|
| 核心抽象 | 1 | `apeireth-core` | R11 既有 | 4 类型占位 | **扩展**：加双洋葱统一体 / 电子环 / 12 键 trait |
| 9 维器官 | 2 | `apeireth-perception` | 阶段 4 §3 推导 | 新建 | **新建**：感知器官 + Attention trait |
|  | 3 | `apeireth-cognition` | 阶段 4 §3 推导 | 新建 | **新建**：认知器官 + Reasoning/Intuition/MetaCognition trait |
|  | 4 | `apeireth-action` | 阶段 4 §3 推导 | 新建 | **新建**：行动器官 + Execution/Expression/Silence trait |
|  | 5 | `apeireth-memory` | R11 既有 | 6 历史流占位 | **扩展**：加 Consolidation/Forgetting/Append-only Log API |
|  | 6 | `apeireth-evolution` | 阶段 4 §3 推导 | 新建（合并 R11 upgrade 部分）| **新建**：演化器官 + Learning/Abstraction/Extension/SelfModification trait |
|  | 7 | `apeireth-motivation` | v4.1 新增 | 新建 | **新建**：动机器官 + Drive/SGI 单字段 |
|  | 8 | `apeireth-value` | v4.1 新增 | 新建 | **新建**：价值器官 + Evaluation/Prioritization |
|  | 9 | `apeireth-consciousness` | v4.1 新增 | 新建 | **新建**：意识器官 + SelfAwareness/DMN/Cognitive-Dream 6 状态机 |
|  | 10 | `apeireth-constraint` | v4.1 新增（合并 R11 philosophy）| 新建 | **新建**：约束器官 + 12 键 trait + 5 重守门 |
| 关系 + 生命力 | 11 | `apeireth-relation` | v4.1 4 关系 | 新建 | **新建**：关系器官 + Symbiosis/Coordination/Embedding/SelfRelation |
|  | 12 | `apeireth-life-force` | 阶段 4 §3 推导 | 新建 | **新建**：生命力维 + Reflection/Homeostasis/Feedback/Emergence |
| 工程支撑 | 13 | `apeireth-council` | 阶段 4 §3 推导 | 新建（合并 R11 部分）| **新建**：智囊团 + 7 强制 + 动态专家 + 按住 |
|  | 14 | `apeireth-upgrade` | R11 upgrade-impl | OTA 占位 | **扩展**：OTA + 沙盒 + 五重治理 |
|  | 15 | `apeireth-bus` | 阶段 2 §9 通信总线 | 新建 | **新建**：5 层总线 + 控制面 / 数据面分离 |
|  | 16 | `apeireth-extension` | 阶段 4 §3 推导 | 新建 | **新建**：插件系统 + WASM + 异构 |
|  | 17 | `apeireth-pybridge` | R11 既有 | PyO3 桥占位 | **保留扩展**：接 1100 R11 模块 + Cargo.toml metadata 更新 |
|  | 18 | `apeireth-cli` | R11 既有 | CLI 入口 | **保留**：升级版本 + 子命令 |

**对比**：9 → 18 = +9 器官 crate（perception/cognition/action/memory-v2/evolution-v2/motivation/value/consciousness/constraint）+ relation + life-force + bus + extension（12 新建 + 6 扩展）。

### 2.2 Cargo.toml metadata 更新

```toml
[workspace]
resolver = "2"
members = [
    "crates/apeireth-core",
    "crates/apeireth-perception",        # 新建
    "crates/apeireth-cognition",         # 新建
    "crates/apeireth-action",            # 新建
    "crates/apeireth-memory",            # 扩展
    "crates/apeireth-evolution",         # 新建
    "crates/apeireth-motivation",        # 新建
    "crates/apeireth-value",             # 新建
    "crates/apeireth-consciousness",     # 新建
    "crates/apeireth-constraint",        # 新建（合并 philosophy）
    "crates/apeireth-relation",          # 新建
    "crates/apeireth-life-force",        # 新建
    "crates/apeireth-council",           # 新建
    "crates/apeireth-upgrade",           # 扩展
    "crates/apeireth-bus",               # 新建
    "crates/apeireth-extension",         # 新建
    "crates/apeireth-pybridge",          # 保留扩展
    "crates/apeireth-cli",               # 保留
    "crates/apeireth-bench",             # 保留（移到 workspace）
    "crates/apeireth-test",              # 保留
]

[workspace.package]
name = "apeireth-rust"  # 改：原 "apeireth"
version = "0.14.0"  # R14 启动版（保持）
edition = "2021"
rust-version = "1.80"
authors = ["Apeireth Team"]
license = "Apache-2.0"
repository = "https://github.com/apeireth/apeireth-rust"
description = "Apeireth R14 Rust 重写 — 立体架构 v2 + 生命架构 v4/v4.1 + 18 crate 本源推导"  # 改：原 "Apeireth 主路径核心"
```

### 2.3 每个新 crate 的最小骨架（施工模板）

```rust
// crates/apeireth-perception/src/lib.rs
//! apeireth-perception: 感知器官
//! 阶段 4 §3 推导: 感知维 + Attention trait
//! 主 17:58 不假装: 所有感知状态透明可观测

#![warn(missing_docs)]

use async_trait::async_trait;
use apeireth_core::{Input, Perception};

#[async_trait]
pub trait Perception {
    async fn perceive(&self, input: Input) -> Result<Perception, PerceptionError>;
    fn attention(&self) -> AttentionFocus;
}

pub struct DefaultPerception;

#[async_trait]
impl Perception for DefaultPerception {
    async fn perceive(&self, input: Input) -> Result<Perception, PerceptionError> {
        // 占位实现 (阶段 5 后续填充)
        Ok(Perception::default())
    }
    
    fn attention(&self) -> AttentionFocus {
        AttentionFocus::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn perception_placeholder() {
        let p = DefaultPerception;
        let r = p.perceive(Input::default()).await;
        assert!(r.is_ok());
    }
}
```

**每个新 crate 套这个模板**——主哲学 6 锚穿透 + 类型 sketch + async trait + 测试占位。

---

## §3. V0.5 v2 24 维落地（按 v4.1 §13 提议）

### 3.1 当前 vs 目标

**当前**：`apeireth/v1077_asi_v04_full_measurement.py` — 17 维 V0.5 公式 LOCKED（v1141 IC-001 fresh=0.8682）。

**目标**：v0.5 v2 = 24 维（17 + 7 新增），按 v4.1 §13 提议。

### 3.2 24 维列表

| 维度 | 来源 | 阶段 |
|---|---|---|
| 1-17 | v0.5 v1 17 维 | 保留 |
| 18 | 动机 / 价值 | v4.1 新增（§13.1）|
| 19 | 意识 | v4.1 新增（§13.1）|
| 20 | 可观测性 | v4.1 新增（§13.1）|
| 21 | 科学性 | v4.1 新增（§13.1）|
| 22 | 诚实 / 谦卑 | v4.1 新增（§13.1）|
| 23 | 与自身的关系 | v4.1 新增（§13.1）|
| 24 | 睡眠 / 巩固 | v4.1 新增（§13.1）|

### 3.3 施工步骤（v1077_asi_v04_full_measurement.py → v1077_v2_24dim）

```python
# 步骤 1: 复制原文件
cp apeireth/v1077_asi_v04_full_measurement.py apeireth/v1077_asi_v24_v2_full_measurement.py

# 步骤 2: 加 7 个新维度定义（保留 v1 17 维 + 公式）
NEW_DIMS_V2 = {
    18: "motivation_value",    # 动机/价值（v4.1 §13.1）
    19: "consciousness",         # 意识（v4.1 §13.1）
    20: "observability",         # 可观测性（v4.1 §13.1）
    21: "scientific",             # 科学性（v4.1 §13.1）
    22: "honesty_humility",     # 诚实/谦卑（v4.1 §13.1）
    23: "self_relation",         # 与自身的关系（v4.1 §13.1）
    24: "sleep_consolidation",  # 睡眠/巩固（v4.1 §13.1）
}

# 步骤 3: 加 V0.5 v2 公式（24 维权重待定，主人拍板）
def v05_v2_total_24dim(scores: dict[int, float]) -> float:
    """V0.5 v2 = 24 维加权（不冻结权重，主 17:43 实事求是）"""
    if not all(d in scores for d in range(1, 25)):
        raise ValueError("需要 1-24 维全部分数")
    return sum(scores[d] * weight_v2(d) for d in range(1, 25))

def weight_v2(d: int) -> float:
    """24 维权重（v1 权重 + 7 新增权重待主人拍板）"""
    weights_v1 = {1: 0.10, 2: 0.08, ...}  # v1 17 维权重（继承）
    weights_v2 = {
        18: 0.05,  # 动机/价值（初始权重，待校准）
        19: 0.05,
        20: 0.05,
        21: 0.05,
        22: 0.05,
        23: 0.05,
        24: 0.05,
    }
    return weights_v1.get(d, weights_v2.get(d, 0.0))

# 步骤 4: 保留 v1 17 维公式（不修改原始）
# 步骤 5: 加 v1 vs v2 baseline 映射（透明标注）
# 步骤 6: 加 R11 baseline 三值并存（V1141=0.8682 / V1131=0.8532 / V1136=0.9063）
# 步骤 7: git commit "V0.5 v2 24 维落地（v4.1 §13 提议）"
```

**关键不假装**：
- V0.5 v1 17 维公式 0 修改（保留为历史轨迹）
- V1141/V1131/V1136 三值 LOCKED 数值不变（透明标注，不重写不互替）
- V0.5 v2 权重**待主人拍板**——主 17:43 实事求是，不冻结系数

### 3.4 文件路径

```
apeireth/v1077_asi_v24_v2_full_measurement.py  ← 新文件（v2）
apeireth/v1077_asi_v04_full_measurement.py   ← 保留（v1 LOCKED）
```

---

## §4. V1136 v2 9 子测度落地（按 v4.1 §14 提议）

### 4.1 当前 vs 目标

**当前**：`apeireth/v1136_asi_v05_3dim_real_measurement.py` — 7 子测度 LOCKED（5 continuity + 2 transferability，R11 baseline = 0.9063）。

**目标**：V1136 v2 = 9 子测度 = 7 + 2 新增。

### 4.2 9 子测度列表

| 子测度 | 来源 | 阶段 |
|---|---|---|
| 1-5 | continuity | v1 保留 |
| 6-7 | transferability | v1 保留 |
| 8 | 记忆巩固度（睡眠指标）| v4.1 新增（§14.1）|
| 9 | 反馈调节效率（控制论）| v4.1 新增（§14.1）|

### 4.3 施工步骤（v1136_asi_v05_3dim_real_measurement.py → v1136_v2_9sub）

```python
# 步骤 1: 复制原文件
cp apeireth/v1136_asi_v05_3dim_real_measurement.py apeireth/v1136_asi_v09_v2_real_measurement.py

# 步骤 2: 加 2 个新子测度
NEW_SUBTESTS_V2 = {
    8: "memory_consolidation",      # 记忆巩固度（Cognitive-Dream DREAMING → CONSOLIDATING 状态迁移成功率）
    9: "feedback_regulation_efficiency",  # 反馈调节效率（Homeostasis 漂移检测 + Feedback 调节成功率）
}

# 步骤 3: 加 V1136 v2 公式
def v1136_v2_total_9sub(scores: dict[int, float]) -> float:
    """V1136 v2 = 9 子测度加权"""
    if not all(s in scores for s in range(1, 10)):
        raise ValueError("需要 1-9 子测度全部分数")
    return sum(scores[s] * weight_v2(s) for s in range(1, 10))

# 步骤 4: 保留 v1 7 子测度公式（不修改原始）
# 步骤 5: R11 baseline 0.9063 保留（5+2 失败透明标注）
# 步骤 6: 加 v2 子测度的真测方法（cooldown + measurement）
# 步骤 7: git commit "V1136 v2 9 子测度落地（v4.1 §14 提议）"
```

**关键不假装**：
- V1136 v1 7 子测度公式 0 修改
- 5+2 子测度失败透明标注（R11 已知）
- V1136 v2 子测度权重待主人拍板

### 4.4 文件路径

```
apeireth/v1136_asi_v09_v2_real_measurement.py  ← 新文件（v2）
apeireth/v1136_asi_v05_3dim_real_measurement.py ← 保留（v1 LOCKED）
```

---

## §5. V3 v2 12 键落地（按 v4.1 §15 提议）

### 5.1 当前 vs 目标

**当前**：`Apeireth-rust/docs/r14-design/philosophy-traits-2026-07-30.md` — V3 9 键 LOCKED。

**目标**：V3 v2 = 12 键 = 9 + 3 新增。

### 5.2 12 键列表

| # | 键 | 含义 | 阶段 |
|---|---|---|---|
| 1-9 | NotClone/NotPerfect/NotUuid/NotUndo/NotProof/NotSafe/SpecIsNotProof/CounterexampleIsNotBug/ProverIsNotTruth | v1 9 键 | 保留 |
| 10 | **PHL-04 NotUnobservable** | 不假装不可观测（v4.1 §15.1 新增）| v2 |
| 11 | **PHL-05 NotUnscientific** | 不假装不科学（v4.1 §15.1 新增）| v2 |
| 12 | **PHL-06 NotSelfRelationless** | 不假装不与自身关系（v4.1 §15.1 新增）| v2 |

### 5.3 施工步骤（philosophy-traits-2026-07-30.md → 加 v2 章节）

```markdown
# 步骤 1: 不修改原 philosophy-traits-2026-07-30.md（v1 LOCKED）

# 步骤 2: 新建 v2 章节文件（独立命名空间）
# 路径: Apeireth-rust/docs/r14-design/philosophy-traits-v2-2026-07-31.md
# 内容: V3 v2 12 键 + 编译时 hardcode trait sketch

# 步骤 3: v2 文件结构
§0 元信息 + 6 锚 + 不修改承诺（v1 9 键保留为历史轨迹）
§1 V3 v2 12 键完整清单
§2 9 v1 键（保留 + 引用）
§3 3 v2 新增键（PHL-04/05/06 详细定义）
§4 编译时 hardcode trait sketch
§5 与 v4.1 §15 提议对齐
§6 阶段 4 工程实现引用（apeireth-constraint crate trait sketch）
§7 主哲学 anchor 6 全贯穿自检

# 步骤 4: 在 v4.1 §15 中加引用
"Apeireth-rust/docs/r14-design/philosophy-traits-v2-2026-07-31.md"

# 步骤 5: git commit "V3 v2 12 键落地（v4.1 §15 提议）"
```

**关键不假装**：
- V3 v1 9 键 LOCKED，0 修改
- v2 文件**独立命名空间**，不覆盖 v1
- v1 9 键保留为"历史轨迹"

### 5.4 文件路径

```
Apeireth-rust/docs/r14-design/philosophy-traits-v2-2026-07-31.md  ← 新文件（v2）
Apeireth-rust/docs/r14-design/philosophy-traits-2026-07-30.md      ← 保留（v1 LOCKED）
```

---

## §6. R11 1100 重写方案

### 6.1 我的判断（主 17:43 实事求是 + 主 17:58 不假装）

**R11 1100+ Python 模块**（apeireth/v1000-v1155*.py）是**历史测量资产**——R14 Rust 重写**不依赖**它们。

**我的方案**：

| 类别 | 数量 | 方案 | 原因 |
|---|---|---|---|
| **保留为 PyO3 桥接资产** | ~300 | `apeireth-legacy/` 移到 `promethean/apeireth-legacy/v*.py` | 主 00:56 任何人都能接手：保留历史可审计 + PyO3 桥接可调用 |
| **合并/抽象** | ~200 | 提炼通用 trait/utility 写到 Rust crate 中 | 主人新姿态：旧语言产物可改 |
| **重写** | ~400 | 按阶段 4 §3 27 trait 重写为 Rust | 主 23:44 干到底：核心功能必须重写 |
| **砍掉** | ~200 | 不再需要的 v1100+ 测量工具（被 R-Measure 12 维替代）| 主 17:58 不假装：v0.5/V1136/V3 已经替代 |

**总判断**：保留 ~300（30% 桥接资产）+ 重写/合并 ~600（60% 核心）+ 砍 ~200（10% 不需要）。

### 6.2 施工步骤

```bash
# 步骤 1: 归档保留 (~300)
mkdir -p promethean/apeireth-legacy
mv apeireth/v1000-v1099*.py promethean/apeireth-legacy/  # 100 个保留（基础测量）
mv apeireth/v1100-v1155*.py promethean/apeireth-legacy/  # 56 个保留（V0.5/V1136 baseline 引用）

# 步骤 2: 砍掉 (~200)
rm apeireth/v1101-v1120_legacy_*.py  # 不需要的测量工具

# 步骤 3: 重写/合并 (~600) → 阶段 4 §3 trait
# 这些不再需要 v*.py 文件，由 Rust crate 实现
rm apeireth/v1121-v1155_redundant_*.py  # 已被 V0.5/V1136/V3 替代
```

### 6.3 归档目录结构

```
promethean/apeireth-legacy/
├── README.md  # 归档说明
├── v1000-v1099/  # 100 个基础测量保留
├── v1100-v1155/  # 56 个 V0.5/V1136 baseline 保留
└── index.md     # 索引（哪些保留 / 哪些砍 / 为什么）
```

---

## §7. Cargo.toml metadata 更新（已在 §2.2）

详见 §2.2。

**关键不假装**：
- name 从 "apeireth" → "apeireth-rust"（明确是 R14 Rust 重写）
- description 加入"立体架构 v2 + 生命架构 v4/v4.1 + 18 crate 本源推导"
- version 保持 0.14.0（R14 启动版）

---

## §8. OTA 7 阶段工程化

### 8.1 当前 vs 目标

**当前**：阶段 2 §11 OTA 7 阶段文档（保留）+ R11 upgrade-impl 部分占位。

**目标**：OTA 7 阶段 = 完整工程实现（在 `apeireth-upgrade` crate 中）。

### 8.2 7 阶段施工模板

```rust
// crates/apeireth-upgrade/src/lib.rs

pub enum OTAStage {
    Intent,        // 1. 意图（提议升级）
    CouncilReview, // 2. 智囊团审议（7 席按风险触发）
    MultiSig,       // 3. 物理多签（HA + §18.6 五重治理）
    Sandbox,        // 4. 沙盒验证（5 重守门）
    Switchover,    // 5. 切换（蓝绿部署）
    Monitor,        // 6. 监控（dashboard + 真测）
    DoneOrRollback, // 7. 完成 / 回滚
}

pub trait OTAController {
    async fn execute_stage(&mut self, stage: OTAStage, intent: UpgradeIntent) -> Result<OTAOutcome, OTAError>;
    fn current_stage(&self) -> OTAStage;
    fn rollback(&mut self) -> Result<(), OTAError>;
}
```

### 8.3 5 重守门编译时 hardcode

```rust
// crates/apeireth-constraint/src/lib.rs

// 1. 编译时 hardcode（const fn + 类型状态）
pub const fn is_valid_ota_intent(intent: &UpgradeIntent) -> bool {
    matches!(intent.risk_level, RiskLevel::Critical | RiskLevel::High | RiskLevel::Medium | RiskLevel::Low)
}

// 2. 运行时拦截（运行时 + 反思期）
pub async fn runtime_check(action: &Action) -> Result<ActionVerdict, VerdictError> {
    let v1 = principle_check(action).await?;
    let v2 = permission_check(action).await?;
    let v3 = ha_check(action).await?;
    match (v1, v2, v3) {
        (Allow, Allow, Allow) => Ok(ActionVerdict::Allow),
        _ => Ok(ActionVerdict::Block(...)),
    }
}

// 3. 多 AI 一致（智囊团审议）
pub async fn multi_ai_consensus(action: &Action, seats: u8) -> Result<bool, ConsensusError> { ... }

// 4. 物理隔离（HA 硬门槛）
pub async fn physical_isolation_check(intent: &Intent) -> Result<VerifiedApproval, HAError> { ... }

// 5. 反思期（不是横切关注点）
pub async fn reflection_audit(action: &Action) -> Result<ReflectionReport, AuditError> { ... }
```

---

## §9. 4 重守门嵌套 + 权限发放 + E 层修改路径（v6 完整版 2026-07-31）

> **v6 修正**（2026-07-31 完整版）：
> - **4 重守门嵌套**（不是 5 重）：编译时（内层）+ 运行时（中间层）+ 物理隔离（外层）+ 反思期审计（外层）
> - **权限发放独立机制**：多 AI 一致 + V0.5 v2 24 维 + L0 HA 人类决策
> - **E 层修改路径**：守门拒绝 → 权限发放允许 → 物理多签 + 重新编译 → 反思期审计 → 7 席审议
> - **5 重治理**：MEWG + 多人 + 多 AI + 物理多签 + 反思期
> 详见解锁：`stage4-correction-v6-consolidated-and-e-layer-mutation.md`
> **v5 修正**（2026-07-31 之前）：每层守门机制
> **v4 修正**（2026-07-31 之前）：5 重守门融入每层

> **v5 修正说明**（2026-07-31，主人 5 个精炼指令）：
> - 守门 1 范围扩大到原则洋葱整体（不只是 12 键）
> - 守门 3（多 AI 一致）从"守门"改为"**权限发放机制**"（V0.5 v2 24 维权重公式）
> - 守门数量 = **4 重**（不是 5 重）
> - 5 项不假装 = **O 层**（与 12 键同层）
> - 反思期审计 ≠ 生命力反思（守护越权 vs AI 自身演化）
> **v4 修正说明**: 主人 2026-07-31 关键洞察"5 重守门融入 A 层之类的，不要执着于单独出来 + 洋葱是重复设计"。
> **v3 错误说法**: "5 重守门在最外层（包裹原则洋葱）"——**错的**。
> **v4 正确说法**: **5 重守门融入每层**（不是独立层，是每层的默认属性）。
> 5 重守门具体内容（编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期）= 每层都自带。
> **历史链保留**: 本节作为"v4 修正"标注，v1/v2/v3 文字**保留**。

**5 重守门具体内容（每层都适用）**：

| 守门 | 实现机制 |
|---|---|
| 1. 编译时 hardcode | Rust 6 大编译时约束（所有权/借用/生命周期/Trait/无反射/零成本）|
| 2. 运行时拦截 | `async RuntimeInterceptor` trait |
| 3. 多 AI 一致 | 3 个不同 LLM 独立检查 |
| 4. 物理隔离 | 修改需重新编译 + 物理多签（AI×3 + 人×2 + 密钥×3）|
| 5. 反思期审计 | Cognitive-Dream 24h 自动审计 |

**每层应用 5 重守门**：

| 层 | E 层 | S 层 | A 层 | M 层 | O 层 |
|---|---|---|---|---|---|
| 编译时 hardcode | E-1..E-6 trait | S trait | A trait | M trait | 12 键 verdict const |
| 运行时拦截 | E 层 check | S 层 check | A 层 check | M 层 check | 12 键 verdict cache |
| 多 AI 一致 | 智囊团审议 | 智囊团审议 | 多 AI 验证 | 多 AI 验证 | 智囊团审议 |
| 物理隔离 | HA + 多签 | 双签 | OTA | OTA | 双签 |
| 反思期审计 | E 审计 | S 审计 | A 审计 | M 审计 | O 审计 |

**修改流程按层不同**（重用 5 重守门）：
- E 层修改 = 五重治理（MEWG + 多人 + 多 AI + 物理多签 + 反思期）
- S 层修改 = 智囊团 + 双签
- A 层修改 = A → M promotion
- M 层修改 = 经验沉淀包
- O 层修改 = 权限矩阵

**对比 v3（错误）vs v4（正确）**：
- ❌ v3："5 重守门是最外层独立执行机制，包裹原则洋葱"
- ✅ v4："5 重守门是每层的默认守门机制，不是独立层"

**详见**: `docs/stage4/stage4-correction-v4-onion-dedupe.md`

### 9.1 12 键编译时 hardcode（Rust 类型系统）

**Rust 6 大编译时约束如何实现 12 键**（已在阶段 4 §9 推导）：

```rust
// 编译时检查：12 键 verdict trait 必须实现
pub trait PhilosophyVerdict: Send + Sync {
    fn verdict(&self, action: &Action) -> Verdict;
}

// 编译时检查：12 键实现必须给出
impl PhilosophyKey {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NotClone => "不假装克隆",
            Self::NotPerfect => "不假装完美",
            // ...
            Self::NotUnobservable => "PHL-04 不假装不可观测",
            Self::NotUnscientific => "PHL-05 不假装不科学",
            Self::NotSelfRelationless => "PHL-06 不假装不与自身关系",
        }
    }
}
```

### 9.2 5 重守门编译时 + 运行时

| 守门 | 编译时 | 运行时 |
|---|---|---|
| 1. 编译时 hardcode | ✅ const fn | — |
| 2. 运行时拦截 | — | ✅ async verdict |
| 3. 多 AI 一致 | — | ✅ consensus |
| 4. 物理隔离 | — | ✅ HA approval |
| 5. 反思期 | — | ✅ reflection |

---

## §10. 阶段 6 验证衔接

**阶段 6 = 设计里程碑式验证机制**：

- R-Measure 24 维度（V0.5 v2 落地后）
- 12 键编译时检查（V3 v2 落地后）
- 9 子测度真测（V1136 v2 落地后）
- 启动验证 3 里程碑：M1 编译时 / M2 启动时 / M3 首次对话
- 沙盒 5 重守门（编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期）

---

## §11. 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — §1 施工本质 + §2 18 crate 都服务 ASI 北极星
S-2 主 17:43 实事求是   — §3.3 不冻结 V0.5 v2 权重 + §6 1100 重写明确分类
O-5 主 17:58 不假装     — §9 12 键编译时 hardcode = 类型不假装
O-2 主 19:33 走在前人经验上 — §1.3 7 大原则借鉴 Erlang/K8s/Rust
O-3 主 23:44 干到底    — §2-§9 8 大模块施工步骤立即落
O-4 主 00:56 任何人都能接手 — §6.3 apeireth-legacy 归档 + §2.3 模板 + §11 自检
```

---

## §12. 不修改承诺（主人硬约束 100% 守住）

| ❌ 不修改 | 原因 |
|---|---|
| **所有 LOCKED 文档**（v2/v4/v4.1/阶段 4/18 stage2/14 stage3/阶段 1）| 本施工图纸仅施工，**0 修改 LOCKED 内容** |
| **R11 V0.5 v1 / V1136 v1 / V3 v1 9 键** | 保留为历史轨迹，**新建 v2 文件**不修改 v1 |

## §13. 主人新姿态（架构更新 → 三把锁同步）

| 类别 | 状态 |
|---|---|
| 阶段 1+2+3 LOCKED 文档 | 🔒 不修改（主人确认沉淀）|
| V0.5/V1136/9 键原始 LOCKED | ✅ **可改**（v2 文件新建，不修改 v1）|
| R11 1100 空壳 | ✅ **保留+归档+合并+重写+砍**（主 17:43 实事求是）|
| crates/ 占位 | ✅ **重写**（按 18 crate 推导）|
| Cargo.toml metadata | ✅ **更新**（name + description）|

---

_施工图纸由 leader 亲自产出（按主人"亲自做" + "工程师/科学家思维从本源重构" + "参照其他优秀项目（不适配不硬融）" + "阅读阶段 1+2+3+4 不迷失"）._
_8 大施工模块: §2 18 crate 重写 + §3 V0.5 v2 24 维 + §4 V1136 v2 9 子测度 + §5 V3 v2 12 键 + §6 R11 1100 重写 + §7 Cargo.toml metadata + §8 OTA 7 阶段 + §9 5 重守门编译时 hardcode._
_主哲学 anchor 6 全贯穿. 任何接手者能查. 不会丢失上下文._
_下次对话启动: 阶段 6 设计里程碑式验证机制._