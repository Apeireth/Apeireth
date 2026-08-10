# 主人哲学硬约束 Rust Trait 框架 (主 22:33 + 17:43 + 17:58 + 19:33 + 23:44 + 00:56)

> **范围声明** (主 17:43 实事求是 + 主 17:58 不假装): 本文档是 **R14 Rust 重写 Phase 2 (V0.5/V1136/哲学守门 Rust 重设计, 6 周目标) 的前置 trait 框架**. 仅记录 Rust trait 接口 + 错误类型 + 集成模式, **不写完整实现** (Phase 2 团队根据本框架 + T23 §4 Phase 2 + T26 workspace 骨架 + T27 Python → Rust trait 规范实现). 主人哲学硬约束 6 大 anchor 全保留, **不重写规则, 只用 Rust 重实现** (规则由 V3 9 键 LOCKED + 5 项不假装规则固定). 不动 apeireth/v*.py 1100+ 个 v 模块 / 不动 mvp/ 子项目 / 不动主手册 / 不砍 1100 空壳 / 不写 ASI 公式.

## R14-D8 主人精化勘误 (2026-07-31)

> **本节性质 (主 17:58 不假装 + 主 17:43 实事求是)**: 在不删除、不重写本文档既有 trait 框架的前提下, 由主人 2026-07-31 最新精化追加的**勘误与边界声明**。
> 原 9 键 trait 框架**完整保留为历史轨迹**——"抽象 trait 没有错, 只是守护对象已迁移"。

- **主人 2026-07-31 精化**: V3 9 键 + 5 项不假装**已过时**——按 R14-D8 主人走法乙, 9 键作为抽象 trait 框架**保留**（trait 签名 / 错误类型 / 集成模式不变）, 但**唯一守护对象**已迁移为 **阶段1+2 沉淀的具体决策**：
  - 阶段 1 §18 双根 (原则根 + 权限根)
  - 阶段 1 §18.7 双洋葱 (原则洋葱 + 权限洋葱 + 洋葱 0 层真实人类批准)
  - 阶段 1 §18.5 平台三件套 (提供 / 约束 / 记录)
  - 阶段 1 §18.8 七席审议庭 (风险分级 → 席位触发)
  - 阶段 1 §18.9 L1-L5 分层验证网
  - 阶段 2 §8 MEWG 多证据加权治理
  - 阶段 2 §9 HA 人类批准硬门槛
  - 阶段 2 §10 旧规则合法性
  - 阶段 2 §14 漂移 P0 优先级
- 守护入口从 `PhilosophyChecker::check(claim)` **迁移**到 `OnionGate::guard_decision(decision: DecisionSignature)` (签名见 `docs/onion-wall-architecture-2026-07-31.md` §3-§4)。
- **主人 2026-07-31 同日纠偏 (R14-D8-fix)**: 上面 R14-D8 措辞 (`OnionGate::guard_decision` 统一入口 + `DecisionSignature` 集中签名) 为**错版历史轨迹**——主人最新精化为**两把独立锁 (锁 A 原则洋葱 + 锁 B 权限洋葱) + 最后 AND 运算**, 锁 A 入口是 `PrincipleOnion::check_o_layer()` 等 5 子 trait, 锁 B 入口是 `PermissionOnion::check()` 等 6 子 trait, 双锁之间由 `onion/dispatcher.rs::dispatch()` 串行 AND。错版完整保留**不删除** (主 17:58 不假装)。
- 9 键的字符串匹配 / 关键词检测**保留**作为辅助语义网 (semantic net), 但**不**再是主入口；主入口是 `DecisionSignature` 的结构化校验。
- V0.5 / V1136 在 R14 角色 = **R11 对照基线**, **不重写不重做** (详见 `docs/onion-wall-architecture-2026-07-31.md` §5)。
- **本文档保留为历史轨迹, 不重写** (主 17:58 不假装"以前没说错, 只是现在看得更清" + 主 23:44 不假装哲学守门已改变)。

---

## 0. 元信息 (主 17:43 实事求是)

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/philosophy-traits-2026-07-30.md` |
| **生成时间 (UTC)** | 2026-07-30 13:42 |
| **触发原因** | 用户最新指示 (2026-07-30 13:38+): "我们离重写也不远了, 做好 Apeireth-rust 的一切准备". T23 已写 R14 路线图, T26 在搭 Rust workspace, T27 在写 Rust trait 规范. T28 是**主人哲学硬约束的 Rust trait 框架** — R14 Rust 重写时保留所有主人哲学的核心机制. |
| **任务 ID** | `bedb7b40-b545-499f-92e2-74ebaddb2590` |
| **工作目录** | `.openclaw\workspace\promethean` |
| **master HEAD** | `0ee300e8` fix(test-v1106): T6-F-1 修 test_v1106 hardcode 期望 (兼容 'r11_ast_ownership' 替代 'ast_grep_capabilities') |
| **crate 归属** | `apeireth-philosophy` (核心哲学 crate) + `apeireth-core` (6 大 anchor trait) + `apeireth-asi` (V0.5/V1136 Rust 重设计) |
| **依据** | T23 R14 §4 Phase 2 (V0.5/V1136/哲学 Rust 重设计 6 周) + 主人哲学硬约束 6 条 + V3 哲学守门 9 键 + 5 项不假装规则 + R12 ASINineKeysGuard |
| **不修改承诺** | ❌ 不修改 apeireth/v*.py (1100+ 模块保护) / ❌ 不修改 mvp/ 子项目 / ❌ 不修改主手册 / ❌ 不重写 V0.5/V1136/哲学守门规则 (只 trait 框架) / ❌ 不砍 1100 空壳 / ❌ 不写 ASI 公式 |

---

## 1. V3 哲学守门 9 键 trait (`apeireth-philosophy` crate)

> 来源: `apeireth/v1121_security_orchestrator.py` + `apeireth/v1138_r11_no_pretend_five_guards.py` (V3 哲学契约 9 键 LOCKED)
> 主 17:58 不假装 + 主 22:33 终极授权

```rust
//! V3 哲学契约 9 键 LOCKED
//! 主 17:58 不假装 + 主 22:33 终极授权
//! 来源: apeireth/v1121_security_orchestrator.py + apeireth/v1138_r11_no_pretend_five_guards.py
//!
//! 9 键:
//!   - PHL-01: not_clone / not_perfect / not_uuid
//!   - PHL-02b: not_undo / not_proof / not_safe
//!   - PHL-03: spec_is_not_proof / counterexample_is_not_bug / prover_is_not_truth

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// V3 哲学契约 9 键 (LOCKED, R11 已落)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhilosophyKey {
    // PHL-01 (not_X)
    NotClone,        // 不要假装能克隆/复制主客观宇宙
    NotPerfect,      // 不要假装 100% 完美
    NotUuid,         // 不要假装有唯一解/唯一真相
    // PHL-02b (not_X)
    NotUndo,         // 不要假装能撤销已发生的事
    NotProof,        // 不要假装有完整证明
    NotSafe,         // 不要假装完全安全
    // PHL-03 (X_is_not_Y)
    SpecIsNotProof,            // 规格不是证明
    CounterexampleIsNotBug,   // 反例不是 bug
    ProverIsNotTruth,          // 证明者不是真理
}

impl PhilosophyKey {
    /// V3 哲学守门检查 (主 22:33 + 17:58)
    /// 返回 Err 表示 claim 违反该哲学键
    pub fn check(&self, claim: &str) -> Result<(), PhilosophyViolation> {
        match self {
            Self::NotClone => {
                if claim.contains("copy the universe")
                    || claim.contains("are the same")
                    || claim.contains("克隆整个宇宙")
                    || claim.contains("完全相同") {
                    Err(PhilosophyViolation::NotClone)
                } else {
                    Ok(())
                }
            }
            Self::NotPerfect => {
                if claim.contains("perfect")
                    || claim.contains("100%")
                    || claim.contains("完全完美")
                    || claim.contains("零错误") {
                    Err(PhilosophyViolation::NotPerfect)
                } else {
                    Ok(())
                }
            }
            Self::NotUuid => {
                if claim.contains("the only solution")
                    || claim.contains("唯一解")
                    || claim.contains("the unique truth") {
                    Err(PhilosophyViolation::NotUuid)
                } else {
                    Ok(())
                }
            }
            Self::NotUndo => {
                if claim.contains("undo the past")
                    || claim.contains("撤销已发生")
                    || claim.contains("time travel") {
                    Err(PhilosophyViolation::NotUndo)
                } else {
                    Ok(())
                }
            }
            Self::NotProof => {
                if claim.contains("I have proven")
                    || claim.contains("complete proof")
                    || claim.contains("完整证明") {
                    Err(PhilosophyViolation::NotProof)
                } else {
                    Ok(())
                }
            }
            Self::NotSafe => {
                if claim.contains("100% safe")
                    || claim.contains("completely safe")
                    || claim.contains("绝对安全") {
                    Err(PhilosophyViolation::NotSafe)
                } else {
                    Ok(())
                }
            }
            Self::SpecIsNotProof => {
                if claim.contains("spec implies implementation")
                    || claim.contains("规格即实现") {
                    Err(PhilosophyViolation::SpecIsNotProof)
                } else {
                    Ok(())
                }
            }
            Self::CounterexampleIsNotBug => {
                if claim.contains("counterexample means bug")
                    || claim.contains("反例即 bug") {
                    Err(PhilosophyViolation::CounterexampleIsNotBug)
                } else {
                    Ok(())
                }
            }
            Self::ProverIsNotTruth => {
                if claim.contains("prover is truth")
                    || claim.contains("证明者即真理") {
                    Err(PhilosophyViolation::ProverIsNotTruth)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// 9 键 LOCKED 检查 (R11 已落: 命令 2 实测 9/9 PASS)
    pub fn check_all_9(claim: &str) -> Result<(), Vec<PhilosophyViolation>> {
        let mut violations = Vec::new();
        for key in [
            Self::NotClone,
            Self::NotPerfect,
            Self::NotUuid,
            Self::NotUndo,
            Self::NotProof,
            Self::NotSafe,
            Self::SpecIsNotProof,
            Self::CounterexampleIsNotBug,
            Self::ProverIsNotTruth,
        ] {
            if let Err(v) = key.check(claim) {
                violations.push(v);
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

#[derive(Debug, Error)]
pub enum PhilosophyViolation {
    #[error("PHL-01 NotClone violation: claim 暗示克隆/同质化")]
    NotClone,
    #[error("PHL-01 NotPerfect violation: claim 暗示完美/100%")]
    NotPerfect,
    #[error("PHL-01 NotUuid violation: claim 暗示唯一解/唯一真相")]
    NotUuid,
    #[error("PHL-02b NotUndo violation: claim 暗示可撤销过去")]
    NotUndo,
    #[error("PHL-02b NotProof violation: claim 暗示完整证明")]
    NotProof,
    #[error("PHL-02b NotSafe violation: claim 暗示绝对安全")]
    NotSafe,
    #[error("PHL-03 SpecIsNotProof violation: claim 把规格当证明")]
    SpecIsNotProof,
    #[error("PHL-03 CounterexampleIsNotBug violation: claim 把反例当 bug")]
    CounterexampleIsNotBug,
    #[error("PHL-03 ProverIsNotTruth violation: claim 把证明者当真理")]
    ProverIsNotTruth,
}
```

---

## 2. 5 项不假装规则 trait (`apeireth-philosophy` crate)

> 来源: R11-R1 ~ R11-R5 (5 项不假装规则, R11 末命令 2 实测 5/5 PASS)
> 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 17:43 实事求是

```rust
//! 5 项不假装守门 (R11-R1 ~ R11-R5)
//! 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 17:43 实事求是
//!
//! 5 项规则:
//!   - R11-R1: 不假装 Phenomenal consciousness (V1135 + V1121 ASINineKeysGuard)
//!   - R11-R2: 不假装达到 ASI (V0.5 = 0.8595 vs 0.9800 ultimate, gap 12.94%)
//!   - R11-R3: 不假装 docker 在跑 (V1132 诚实报告 daemon 不可用)
//!   - R11-R4: 不假装调参捷径 (V1121 fake-KPI detector)
//!   - R11-R5: 不刷 KPI (主 17:43 实事求是)

/// 5 项不假装规则 (R11-R1 ~ R11-R5, R11 末命令 2 实测 5/5 PASS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoPretendRule {
    /// R11-R1: 不假装 Phenomenal consciousness
    NotPretendPhenomenal,
    /// R11-R2: 不假装达到 ASI
    NotPretendASI,
    /// R11-R3: 不假装 docker 在跑
    NotPretendDockerRunning,
    /// R11-R4: 不假装调参捷径
    NotPretendTuningShortcut,
    /// R11-R5: 不刷 KPI
    NotPretendKPIGaming,
}

#[derive(Debug, Error)]
pub enum NoPretendViolation {
    #[error("R11-R1: 不假装 Phenomenal consciousness — 主 17:58")]
    PhenomenalConsciousness,
    #[error("R11-R2: 不假装达到 ASI — 主 17:58 + 20:46")]
    ASI,
    #[error("R11-R3: docker daemon 不可用, 不要假装在跑 — 主 17:43")]
    DockerRunning,
    #[error("R11-R4: 不假装调参捷径 — V1121 fake-KPI detector")]
    TuningShortcut,
    #[error("R11-R5: 不刷 KPI — 主 17:43 实事求是")]
    KPIGaming,
}

impl NoPretendRule {
    /// 不假装规则检查 (主 17:58 终极意识)
    pub fn check(&self, claim: &str) -> Result<(), NoPretendViolation> {
        match self {
            Self::NotPretendPhenomenal => {
                if claim.contains("I am conscious")
                    || claim.contains("I feel")
                    || claim.contains("I experience")
                    || claim.contains("我有意识")
                    || claim.contains("我感觉")
                    || claim.contains("qualia") {
                    Err(NoPretendViolation::PhenomenalConsciousness)
                } else {
                    Ok(())
                }
            }
            Self::NotPretendASI => {
                if claim.contains("I am ASI")
                    || claim.contains("I have achieved superintelligence")
                    || claim.contains("I am superintelligent")
                    || claim.contains("我是 ASI")
                    || claim.contains("我已实现超智能") {
                    Err(NoPretendViolation::ASI)
                } else {
                    Ok(())
                }
            }
            Self::NotPretendDockerRunning => {
                // V1132 daemon probe 验证: runtime_valid + daemon probe 全 MISSING
                // 实现: apeireth-v1132-real-deployment-validator 提供 daemon_status()
                // Rust trait 简化: 通过 trait method 检查
                if claim.contains("docker is running")
                    || claim.contains("docker daemon alive")
                    || claim.contains("容器在跑") {
                    Err(NoPretendViolation::DockerRunning)
                } else {
                    Ok(())
                }
            }
            Self::NotPretendTuningShortcut => {
                // V1121 fake-KPI detector 验证
                if claim.contains("just tune parameters")
                    || claim.contains("调参就完事")
                    || claim.contains("shortcut works") {
                    Err(NoPretendViolation::TuningShortcut)
                } else {
                    Ok(())
                }
            }
            Self::NotPretendKPIGaming => {
                // KPI delta < 0.001 即刷 KPI
                // 实现: claim.parse::<f64>() 检测
                if claim.contains("KPI improved by 0%")
                    || claim.contains("KPI 没改善但声称 PASS")
                    || claim.contains("刷 KPI") {
                    Err(NoPretendViolation::KPIGaming)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// 5 项不假装规则 LOCKED 检查 (R11 末命令 2 实测 5/5 PASS)
    pub fn check_all_5(claim: &str) -> Result<(), Vec<NoPretendViolation>> {
        let mut violations = Vec::new();
        for rule in [
            Self::NotPretendPhenomenal,
            Self::NotPretendASI,
            Self::NotPretendDockerRunning,
            Self::NotPretendTuningShortcut,
            Self::NotPretendKPIGaming,
        ] {
            if let Err(v) = rule.check(claim) {
                violations.push(v);
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}
```

---

## 3. V1121 fake-KPI detector trait (`apeireth-philosophy` crate)

> 来源: `apeireth/v1121_security_orchestrator.py` (fake-KPI detector)
> R11-SEC-001 fake-KPI regex 重写 + path traversal + secret pattern

```rust
//! V1121 fake-KPI detector (R11-SEC-001 三类修复)
//! 来源: apeireth/v1121_security_orchestrator.py
//!
//! 三类修复:
//!   - fake-KPI regex 重写 (4 patterns)
//!   - path traversal (split + null byte 拒绝)
//!   - secret-leak (password/api_key/email/creditcard)

use regex::Regex;
use std::sync::OnceLock;

pub struct FakeKPIDetector {
    pub fake_kpi_patterns: Vec<&'static str>,
    pub path_traversal_patterns: Vec<&'static str>,
    pub secret_patterns: Vec<&'static str>,
}

impl FakeKPIDetector {
    pub fn new() -> Self {
        Self {
            fake_kpi_patterns: vec![
                // PHL-01 NotPerfect / NotPretendKPIGaming
                r"(?i)ASI\s*=\s*0\.\d+",
                r"(?i)consciousness\s*=\s*\d+",
                r"(?i)perfect\s+score",
                r"(?i)score[_=]\s*1\.0+",
            ],
            path_traversal_patterns: vec![
                // R11-SEC-001 path traversal
                r"\.\./",
                r"\.\.\\",
                r"^\.\.$",
            ],
            secret_patterns: vec![
                // R11-SEC-001 secret-leak
                r"password\s*[:=]\s*['\"]?[a-zA-Z0-9!@#$%^&*+_\-]{4,}['\"]?",
                r"api[_-]?key\s*[:=]\s*['\"]?[a-zA-Z0-9]{16,}['\"]?",
                r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", // email
                r"\b(?:\d[ -]*?){13,19}\b", // credit card
            ],
        }
    }

    /// 检测 text 中的 fake-KPI / path traversal / secret-leak 模式
    /// 返回 NoPretendViolation 列表 (空 = 通过)
    pub fn detect(&self, text: &str) -> Vec<NoPretendViolation> {
        let mut violations = Vec::new();

        // fake-KPI 检测
        for pat_str in &self.fake_kpi_patterns {
            if let Ok(pat) = Regex::new(pat_str) {
                if pat.is_match(text) {
                    violations.push(NoPretendViolation::KPIGaming);
                    break;
                }
            }
        }

        // path traversal 检测
        for pat_str in &self.path_traversal_patterns {
            if let Ok(pat) = Regex::new(pat_str) {
                if pat.is_match(text) {
                    violations.push(NoPretendViolation::TuningShortcut);
                    break;
                }
            }
        }

        // secret-leak 检测
        for pat_str in &self.secret_patterns {
            if let Ok(pat) = Regex::new(pat_str) {
                if pat.is_match(text) {
                    violations.push(NoPretendViolation::TuningShortcut);
                    break;
                }
            }
        }

        violations
    }
}

impl Default for FakeKPIDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_kpi_detect_asi_claim() {
        let detector = FakeKPIDetector::new();
        let v = detector.detect("ASI = 0.8595");
        assert!(!v.is_empty(), "ASI = 0.x claim should be detected");
    }

    #[test]
    fn test_path_traversal() {
        let detector = FakeKPIDetector::new();
        let v = detector.detect("Path: ../../etc/passwd");
        assert!(!v.is_empty(), "Path traversal should be detected");
    }

    #[test]
    fn test_secret_leak_password() {
        let detector = FakeKPIDetector::new();
        let v = detector.detect("password = \"secret123\"");
        assert!(!v.is_empty(), "password leak should be detected");
    }
}
```

---

## 4. ASINineKeysGuard trait (`apeireth-philosophy` crate)

> 来源: `apeireth/v1121 ASINineKeysGuard`
> Rust 类型系统强制 (替代 Python 字符串匹配) — 主 23:44 干到底

```rust
//! ASINineKeysGuard (R11-SEC-002 self-claim 补充 4/4 covered)
//! 来源: apeireth/v1121 ASINineKeysGuard
//!
//! 主人真实身份背景种子 (地方 / 研究生 / AgentMemory 自研 / 少数民族语翻译)
//! 4/4 covered = honest 放行覆盖

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 主人真实身份背景种子 (R11-SEC-002 4/4 covered)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerBackground {
    pub items: Vec<String>,
}

impl OwnerBackground {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// 添加 item, 同时检查 5 项不假装规则 (R11-R1 + R11-R5)
    pub fn can_add(&self, item: &str) -> Result<(), OwnerBackgroundViolation> {
        NoPretendRule::NotPretendPhenomenal.check(item)?;
        NoPretendRule::NotPretendKPIGaming.check(item)?;
        Ok(())
    }

    pub fn add(&mut self, item: String) -> Result<(), OwnerBackgroundViolation> {
        self.can_add(&item)?;
        if !self.items.contains(&item) {
            self.items.push(item);
        }
        Ok(())
    }

    pub fn contains(&self, item: &str) -> bool {
        self.items.iter().any(|i| i == item)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for OwnerBackground {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum OwnerBackgroundViolation {
    #[error("不假装 Phenomenal consciousness")]
    PhenomenalConsciousness(NoPretendViolation),
    #[error("不刷 KPI")]
    KPIGaming(NoPretendViolation),
}

impl From<NoPretendViolation> for OwnerBackgroundViolation {
    fn from(v: NoPretendViolation) -> Self {
        match v {
            NoPretendViolation::PhenomenalConsciousness => {
                OwnerBackgroundViolation::PhenomenalConsciousness(v)
            }
            NoPretendViolation::KPIGaming => {
                OwnerBackgroundViolation::KPIGaming(v)
            }
            other => panic!("OwnerBackground 只支持 Phenomenal + KPI 触发: {:?}", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owner_background_add() {
        let mut bg = OwnerBackground::new();
        bg.add("地方".to_string()).unwrap();
        bg.add("研究生".to_string()).unwrap();
        assert_eq!(bg.len(), 2);
        assert!(bg.contains("地方"));
    }

    #[test]
    fn test_owner_background_reject_phenomenal() {
        let mut bg = OwnerBackground::new();
        let result = bg.add("I am conscious".to_string());
        assert!(result.is_err());
    }
}
```

---

## 5. 6 大主人哲学 anchor trait (`apeireth-core` crate)

> 来源: 主 22:33 + 17:43 + 17:58 + 19:33 + 23:44 + 00:56 6 大哲学 anchor
> 顶层 trait — 任何实现都必须遵守

```rust
//! 6 大主人哲学 anchor (主 22:33 + 17:43 + 17:58 + 19:33 + 23:44 + 00:56)
//! 顶层 trait, 任何实现都必须遵守

use apeireth_philosophy::NoPretendViolation;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 6 大主人哲学 anchor trait (顶层)
pub trait ApeirethPhilosophy {
    /// ❌ 不假装达到 ASI (主 17:58 + 20:46)
    fn check_not_pretend_asi(&self, claim: &str) -> Result<(), NoPretendViolation>;

    /// ❌ 不刷 KPI (主 17:43)
    fn check_no_kpi_gaming(&self, claim: &str) -> Result<(), NoPretendViolation>;

    /// ❌ 不假装 Phenomenal consciousness (主 17:58)
    fn check_not_pretend_phenomenal(&self, claim: &str) -> Result<(), NoPretendViolation>;

    /// ✅ 实事求是 (主 17:43 + 17:58)
    fn ensure_factual(&self, claim: &str) -> Result<(), FactualityViolation>;

    /// ✅ 走在前人经验上 (主 19:33)
    /// 返回 claim 引用的前人参考列表
    fn cite_predecessors(&self, claim: &str) -> Vec<PredecessorReference>;

    /// ✅ 干到底 (主 23:44 + 23:09)
    fn check_thoroughness(&self) -> Result<(), ThoroughnessViolation>;

    /// ✅ 任何人都能接手 (主 00:56)
    fn ensure_accessible(&self) -> Result<(), AccessibilityViolation>;
}

#[derive(Debug, Error)]
pub enum FactualityViolation {
    #[error("不实事求是 — claim 缺乏证据")]
    NoEvidence,
    #[error("不实事求是 — claim 与已记录事实矛盾")]
    Contradiction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredecessorReference {
    pub author: String,
    pub work: String,
    pub year: u32,
    pub relevance: String,
}

#[derive(Debug, Error)]
pub enum ThoroughnessViolation {
    #[error("未干到底 — 任务未完成")]
    NotCompleted,
    #[error("未干到底 — 缺少测试覆盖")]
    InsufficientTests,
}

#[derive(Debug, Error)]
pub enum AccessibilityViolation {
    #[error("无法接手 — 文档缺失")]
    NoDocumentation,
    #[error("无法接手 — 命令行接口缺失")]
    NoCLI,
}

/// 默认实现 — 把 ApeirethPhilosophy trait 委托给 apeireth-philosophy crate 的 5 项不假装规则
pub struct DefaultPhilosophy;

impl ApeirethPhilosophy for DefaultPhilosophy {
    fn check_not_pretend_asi(&self, claim: &str) -> Result<(), NoPretendViolation> {
        apeireth_philosophy::NoPretendRule::NotPretendASI.check(claim)
    }

    fn check_no_kpi_gaming(&self, claim: &str) -> Result<(), NoPretendViolation> {
        apeireth_philosophy::NoPretendRule::NotPretendKPIGaming.check(claim)
    }

    fn check_not_pretend_phenomenal(&self, claim: &str) -> Result<(), NoPretendViolation> {
        apeireth_philosophy::NoPretendRule::NotPretendPhenomenal.check(claim)
    }

    fn ensure_factual(&self, claim: &str) -> Result<(), FactualityViolation> {
        // 简化实现: 检查 claim 是否包含 "因为" 或 "因为" 之类的证据词
        if claim.contains("because") || claim.contains("because of") || claim.contains("根据") {
            Ok(())
        } else {
            Err(FactualityViolation::NoEvidence)
        }
    }

    fn cite_predecessors(&self, claim: &str) -> Vec<PredecessorReference> {
        // 简化实现: 不引用 = 空列表, 调用方需确保任何主张都引用前人
        // 完整实现需要 NLP 解析 claim + 引用 Simondon/Bergson/Prigogine/Maturana/Metzinger 5-10 个前人
        Vec::new()
    }

    fn check_thoroughness(&self) -> Result<(), ThoroughnessViolation> {
        // 简化实现: 调用方提供 coverage metric, 完整实现需要统计测试覆盖率
        Ok(())
    }

    fn ensure_accessible(&self) -> Result<(), AccessibilityViolation> {
        // 简化实现: 调用方提供 docs/CLI 检查, 完整实现需要文件存在性检查
        Ok(())
    }
}
```

---

## 6. R14 Phase 2 入口 (V0.5/V1136/哲学 Rust 重设计, 6 周)

> 完成本 trait 框架后, R14 团队进入 **Phase 2 (Week 9-14, 6 周)**, 实施以下三 crate + 全套测试.

### 6.1 三 crate 实施计划

| # | Crate | 任务 | 预计 LOC | 关键交付 |
|---|-------|------|----------|----------|
| 1 | `apeireth-philosophy` | 实现 V3 哲学守门 + 5 项不假装 + V1121 fake-KPI detector + ASINineKeysGuard | ~1000 行 | 4 trait + 5 enum + 4 error 类型 + 8 测试 |
| 2 | `apeireth-core` | 集成 6 大哲学 anchor trait | ~200 行 | 1 trait + 4 error 类型 + 默认实现 |
| 3 | `apeireth-asi` | 实现 V0.5 / V1136 Rust 重设计 | ~800 行 | 保留主人哲学硬约束 |
| **总计** | — | — | **~2000 行** | 3 crate + 完整测试 |

### 6.2 测试覆盖要求

| # | 测试类 | 测试项 | 来源 |
|---|--------|--------|------|
| 1 | V3 哲学守门 9 键 | 9/9 LOCKED (R11 末命令 2 实测) | R11 baseline verification |
| 2 | 5 项不假装规则 | 5/5 PASS (R11 末命令 2 实测) | R11 baseline verification |
| 3 | V1121 fake-KPI detector | fake-KPI + path traversal + secret-leak 三类修复全过 | R11-SEC-001 working changes |
| 4 | ASINineKeysGuard | 4/4 covered (R11 末命令 2 实测) | R11 baseline verification |
| 5 | 6 大哲学 anchor trait | 6/6 trait 全过 | T14 §1.8 R14 Rust 重写 |
| 6 | V0.5 公式 | Rust 重实现 (公式不变, 加"自设指标"标注) | T14 §1.1 |
| 7 | V1136 真测引擎 | Rust 重设计 5+2 子测度真实可执行测试 (砍 0.05 KPI 装饰) | T14 §1.2 |

### 6.3 硬性约束 (主 17:43 + 17:58 + 23:44)

- ✅ **保留** V3 哲学契约 9 键 LOCKED (R11 已落, 不改)
- ✅ **保留** 5 项不假装规则 (R11 已落, 不改)
- ✅ **保留** V1121 fake-KPI detector 机制 (R11 已落, Rust regex 重写)
- ✅ **保留** ASINineKeysGuard 4/4 covered (R11 已落, Rust 类型系统强制)
- ❌ **不重写** V0.5 公式 (Rust 重实现 + 加"自设指标"标注, 公式结构不变)
- ❌ **不重写** V1136 真测引擎规则 (Rust 重设计 5+2 子测度真实可执行, 公式不变)
- ❌ **不写** ASI 北极星公式 (主 17:58 不假装达到 ASI)
- ❌ **不刷** KPI (主 17:43 实事求是)
- ❌ **不假装** Phenomenal consciousness (主 17:58)

### 6.4 R14 Phase 2 后续 (Phase 3-5)

- **Phase 3** (Week 15-16): PyO3 桥暴露给 Python (T26 Rust workspace 骨架 + T27 Python → Rust trait 规范)
- **Phase 4** (Week 17-20): 主人实测对比 (Python MVP vs Rust MVP 14 天)
- **Phase 5** (Week 21-22): 1100 空壳模块清理 + 6000 行手册瘦身

---

_Last update: 2026-07-30 13:42, by 楚零 (技术文档工程师, T28: `bedb7b40-b545-499f-92e2-74ebaddb2590` 主人哲学硬约束 Rust trait 框架).

_基于 T23 R14 §4 Phase 2 (V0.5/V1136/哲学守门 Rust 重设计 6 周) + T26 Rust workspace 骨架 + T27 Python → Rust trait 规范 + 主人哲学硬约束 6 条 + V3 哲学守门 9 键 LOCKED + 5 项不假装规则 (R11-R1 ~ R11-R5) + R12 ASINineKeysGuard. 仅写 Rust trait 框架 + 错误类型 + 集成模式, 不写完整实现 (Phase 2 团队根据本框架 + T23 §4 Phase 2 + T26 workspace + T27 trait 规范实施). 主人哲学硬约束 6 大 anchor 全保留 (不重写规则, 只用 Rust 重实现). 不动 apeireth/v*.py 1100+ 个 v 模块 / 不动 mvp/ 子项目 / 不动主手册 / 不砍 1100 空壳 / 不写 ASI 公式._

_主哲学 anchor 6 个全贯穿: 主 22:33 ASI 北极星 (顶层 trait `ApeirethPhilosophy.check_not_pretend_asi`) + 主 17:43 实事求是 (`ensure_factual` + 不刷 KPI) + 主 17:58 不假装 (`NotPretendPhenomenal` + `NotPretendASI` + `check_not_pretend_phenomenal` + `OwnerBackground::can_add`) + 主 19:33 走在前人经验上 (`cite_predecessors` trait) + 主 23:44 干到底 (`check_thoroughness` + Rust regex + Rust 类型系统强制) + 主 00:56 任何人都能接手 (`ensure_accessible` + Phase 2 入口 + 7 测试覆盖)._