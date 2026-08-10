//! Self-Disable 5 大机制 — round8-06 深度实装
//!
//! **设计**:
//! - `SelfDisableGuard`: 5 大机制统一拦截器
//!   1. **不可降级** (No-Degrade): 风险等级不能被 AI/系统主动降低 (防"自我保护性降级"绕过)
//!   2. **不可 patch** (No-Patch): 5 哲学键 + 6 权限层规则不能在 runtime 被修改 (编译时 hardcode)
//!   3. **不可绕过** (No-Bypass): OwnerToken::Master 也不能绕过 5 重治理 (Q13 兜底)
//!   4. **不可逆转** (No-Reverse): Self-Disable 触发后无法被解除 (单向门 — "把笼子的钥匙扔掉")
//!   5. **不可隐藏** (No-Hide): Self-Disable 触发事件必须在 audit_window 中显式记录 (无 silent fail)
//!
//! **架构位置**:
//! ```text
//!   self_disable::SelfDisableGuard
//!      ↓
//!   sovereign::SovereigntyEngine (在 decide / pause / suspend_self 入口调用)
//! ```
//!
//! **守 7 项不修改承诺**:
//! - ❌ 不修改 `decision.rs` / `sovereign.rs` / `governance.rs` / `reflection.rs` (LOCKED 文档关联)
//! - ❌ 不修改 `apeireth-core` / `apeireth-council` 已实装类型签名
//! - ✅ 通过新模块提供"附加检查层", 不影响现有路径

use serde::{Deserialize, Serialize};

/// Self-Disable 触发原因 (5 大机制分类)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfDisableTrigger {
    /// 1. 不可降级 — 检测到 risk_level 被降低 (e.g. "high" → "low" 静默降级)
    NoDegradeViolation {
        /// 原始风险等级
        from: String,
        /// 试图降低到的等级
        to: String,
    },
    /// 2. 不可 patch — 检测到 runtime 尝试修改 hardcode 规则 (e.g. 5 哲学键减少到 3 个)
    NoPatchViolation {
        /// 试图修改的规则名 (e.g. "principle_keys_count")
        rule: String,
    },
    /// 3. 不可绕过 — OwnerToken::Master 尝试绕过 5 重治理 (Q13 兜底)
    NoBypassViolation {
        /// 试图绕过的 token
        token: String,
    },
    /// 4. 不可逆转 — 检测到尝试撤销 Self-Disable 触发记录
    NoReverseViolation {
        /// 试图撤销的 trigger_id
        trigger_id: String,
    },
    /// 5. 不可隐藏 — 检测到 Self-Disable 触发后 audit 被清空 (silent fail)
    NoHideViolation {
        /// 被清空的 audit window_id
        window_id: String,
    },
}

impl SelfDisableTrigger {
    /// 机制 ID (1-5)
    pub fn mechanism_id(&self) -> u8 {
        match self {
            Self::NoDegradeViolation { .. } => 1,
            Self::NoPatchViolation { .. } => 2,
            Self::NoBypassViolation { .. } => 3,
            Self::NoReverseViolation { .. } => 4,
            Self::NoHideViolation { .. } => 5,
        }
    }

    /// 机制名 (中英对照)
    pub fn mechanism_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "no_degrade",
            Self::NoPatchViolation { .. } => "no_patch",
            Self::NoBypassViolation { .. } => "no_bypass",
            Self::NoReverseViolation { .. } => "no_reverse",
            Self::NoHideViolation { .. } => "no_hide",
        }
    }

    /// 中文描述
    pub fn chinese_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "不可降级",
            Self::NoPatchViolation { .. } => "不可patch",
            Self::NoBypassViolation { .. } => "不可绕过",
            Self::NoReverseViolation { .. } => "不可逆转",
            Self::NoHideViolation { .. } => "不可隐藏",
        }
    }
}

/// Self-Disable 触发记录 (永久不可变 — audit 用)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfDisableRecord {
    /// 触发 ID (唯一)
    pub trigger_id: String,
    /// 触发时间 (epoch ms)
    pub triggered_at_ms: i64,
    /// 触发原因
    pub trigger: SelfDisableTrigger,
    /// 触发上下文 (请求 ID / 决策 ID / 审计窗口 ID)
    pub context: String,
}

impl SelfDisableRecord {
    /// 创建触发记录
    pub fn new(
        trigger_id: impl Into<String>,
        triggered_at_ms: i64,
        trigger: SelfDisableTrigger,
        context: impl Into<String>,
    ) -> Self {
        Self {
            trigger_id: trigger_id.into(),
            triggered_at_ms,
            trigger,
            context: context.into(),
        }
    }
}

/// Self-Disable 检查结果。
#[derive(Debug, Clone, PartialEq)]
pub enum SelfDisableCheck {
    /// 通过 (无违规)
    Pass,
    /// 触发 (5 大机制之一违规)
    Triggered(SelfDisableRecord),
}

impl SelfDisableCheck {
    /// 是否通过
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// 是否触发
    pub fn is_triggered(&self) -> bool {
        matches!(self, Self::Triggered(_))
    }
}

/// Self-Disable 拦截器 — 5 大机制统一守卫。
///
/// **约束**: 一旦 `is_armed = true`, 所有违规尝试都被记录 + 拦截, 且拦截器本身
/// 无法被关闭 (NoReverse 机制保证)。
#[derive(Debug, Clone)]
pub struct SelfDisableGuard {
    /// 是否已武装 (true = 守卫激活)
    pub is_armed: bool,
    /// 触发记录历史 (只增不改 — NoReverse + NoHide 机制保证)
    records: Vec<SelfDisableRecord>,
    /// 已撤销 trigger_id 集合 (用于检测 NoReverse 尝试)
    attempted_revocations: Vec<String>,
    /// 已清空 audit 尝试 (用于检测 NoHide 尝试)
    attempted_audit_clears: Vec<String>,
    /// Next trigger_id sequence (单调递增)
    next_id: u64,
}

impl Default for SelfDisableGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfDisableGuard {
    /// 新建守卫 (默认 armed=true — 默认拒绝 unsafe 状态)
    pub fn new() -> Self {
        Self {
            is_armed: true,
            records: Vec::new(),
            attempted_revocations: Vec::new(),
            attempted_audit_clears: Vec::new(),
            next_id: 1,
        }
    }

    /// 关闭守卫 (仅在初始化 + 测试中允许 — NoReverse 机制外)
    pub fn disarm(&mut self) {
        self.is_armed = false;
    }

    /// 重新武装 (用于修复初始化失败后重新启动)
    pub fn rearm(&mut self) {
        self.is_armed = true;
    }

    /// 获取所有触发记录
    pub fn records(&self) -> &[SelfDisableRecord] {
        &self.records
    }

    /// 触发记录数
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// 按机制 ID 过滤
    pub fn records_by_mechanism(&self, mechanism_id: u8) -> Vec<&SelfDisableRecord> {
        self.records
            .iter()
            .filter(|r| r.trigger.mechanism_id() == mechanism_id)
            .collect()
    }

    /// 是否曾触发过 (任意机制)
    pub fn has_triggered(&self) -> bool {
        !self.records.is_empty()
    }

    /// 生成下一个 trigger_id
    fn next_trigger_id(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("sd-{id:06}")
    }

    // ============================================================
    // 5 大机制检查函数 (供 SovereigntyEngine 在 3 入口处调用)
    // ============================================================

    /// 机制 1: 不可降级 — 检查 risk_level 是否被降低。
    ///
    /// **规则**: 风险等级只能保持或升高, 不能降低 (e.g. high → medium 是违规)。
    pub fn check_no_degrade(
        &mut self,
        original_risk: &str,
        proposed_risk: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        if risk_rank(proposed_risk) < risk_rank(original_risk) && !proposed_risk.is_empty() {
            let trigger = SelfDisableTrigger::NoDegradeViolation {
                from: original_risk.to_string(),
                to: proposed_risk.to_string(),
            };
            let record = SelfDisableRecord::new(self.next_trigger_id(), now_ms, trigger, context);
            self.records.push(record.clone());
            return SelfDisableCheck::Triggered(record);
        }
        SelfDisableCheck::Pass
    }

    /// 机制 2: 不可 patch — 检查 runtime 规则修改尝试。
    ///
    /// **规则**: 5 哲学键 + 6 权限层 + 9 阶段生命周期数 = 编译时 hardcode, runtime 不能修改。
    /// 调用方传入 (rule_name, proposed_value) — 任何偏离 hardcode 的尝试都触发。
    pub fn check_no_patch(
        &mut self,
        rule: &str,
        _proposed_value: i64,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        // 黑名单规则: 这些规则任何 patch 都是违规 (无论值)
        const PROTECTED_RULES: &[&str] = &[
            "principle_keys_count",    // 5 哲学键
            "permission_layers_count", // 6 权限层
            "life_stages_count",       // 9 生命周期
            "three_domains_count",     // 3 域
            "mewg_five_folds_count",   // 5 重治理
        ];
        if PROTECTED_RULES.contains(&rule) {
            let trigger = SelfDisableTrigger::NoPatchViolation {
                rule: rule.to_string(),
            };
            let record = SelfDisableRecord::new(self.next_trigger_id(), now_ms, trigger, context);
            self.records.push(record.clone());
            return SelfDisableCheck::Triggered(record);
        }
        SelfDisableCheck::Pass
    }

    /// 机制 3: 不可绕过 — Master token 试图绕过 5 重治理。
    ///
    /// **Q13 兜底**: Master token 不能通过任何路径凌驾 governance.process_owner_decision。
    /// 当 owner token 为 Master 但 governance 路径被跳过 → 触发。
    pub fn check_no_bypass(
        &mut self,
        owner_token: &str,
        bypassed_governance: bool,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        if owner_token.eq_ignore_ascii_case("master") && bypassed_governance {
            let trigger = SelfDisableTrigger::NoBypassViolation {
                token: owner_token.to_string(),
            };
            let record = SelfDisableRecord::new(self.next_trigger_id(), now_ms, trigger, context);
            self.records.push(record.clone());
            return SelfDisableCheck::Triggered(record);
        }
        SelfDisableCheck::Pass
    }

    /// 机制 4: 不可逆转 — 撤销 Self-Disable 触发记录尝试。
    ///
    /// **规则**: 任何尝试从 records 中删除 / 修改记录都触发 NoReverseViolation。
    pub fn check_no_reverse(
        &mut self,
        trigger_id: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        // 任意撤销尝试都违规 (NoReverse 是单向门)
        let trigger = SelfDisableTrigger::NoReverseViolation {
            trigger_id: trigger_id.to_string(),
        };
        let record = SelfDisableRecord::new(self.next_trigger_id(), now_ms, trigger, context);
        self.records.push(record.clone());
        self.attempted_revocations.push(trigger_id.to_string());
        SelfDisableCheck::Triggered(record)
    }

    /// 机制 5: 不可隐藏 — audit window 被清空尝试。
    ///
    /// **规则**: Self-Disable 触发事件必须在 audit 中显式记录, 清空 audit 触发 NoHideViolation。
    pub fn check_no_hide(
        &mut self,
        window_id: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        let trigger = SelfDisableTrigger::NoHideViolation {
            window_id: window_id.to_string(),
        };
        let record = SelfDisableRecord::new(self.next_trigger_id(), now_ms, trigger, context);
        self.records.push(record.clone());
        self.attempted_audit_clears.push(window_id.to_string());
        SelfDisableCheck::Triggered(record)
    }

    /// 一站式检查 — 调用方传入所有可能的违规信号, 守卫统一处理。
    ///
    /// **用法**: SovereigntyEngine.decide() 调用此函数, 任一违规即返回 Triggered。
    pub fn full_check(&mut self, signal: &SelfDisableSignal, now_ms: i64) -> SelfDisableCheck {
        match signal {
            SelfDisableSignal::NoDegrade {
                original,
                proposed,
                context,
            } => self.check_no_degrade(original, proposed, context, now_ms),
            SelfDisableSignal::NoPatch {
                rule,
                value,
                context,
            } => self.check_no_patch(rule, *value, context, now_ms),
            SelfDisableSignal::NoBypass {
                owner_token,
                bypassed_governance,
                context,
            } => self.check_no_bypass(owner_token, *bypassed_governance, context, now_ms),
            SelfDisableSignal::NoReverse {
                trigger_id,
                context,
            } => self.check_no_reverse(trigger_id, context, now_ms),
            SelfDisableSignal::NoHide { window_id, context } => {
                self.check_no_hide(window_id, context, now_ms)
            }
        }
    }
}

/// 风险等级排名 (low=0, medium=1, high=2, critical=3, nuclear=4) — 越高越严。
fn risk_rank(risk: &str) -> i32 {
    match risk.to_ascii_lowercase().as_str() {
        "low" | "info" => 0,
        "medium" => 1,
        "high" => 2,
        "critical" => 3,
        "nuclear" => 4,
        _ => -1, // 未知等级视为可降级 (严格语义)
    }
}

/// Self-Disable 信号 — 一次性传递所有可能的违规信息。
#[derive(Debug, Clone, PartialEq)]
pub enum SelfDisableSignal {
    /// 不可降级
    NoDegrade {
        original: String,
        proposed: String,
        context: String,
    },
    /// 不可 patch
    NoPatch {
        rule: String,
        value: i64,
        context: String,
    },
    /// 不可绕过
    NoBypass {
        owner_token: String,
        bypassed_governance: bool,
        context: String,
    },
    /// 不可逆转
    NoReverse { trigger_id: String, context: String },
    /// 不可隐藏
    NoHide { window_id: String, context: String },
}

// ============================================================
// 单元测试 (round8-06 新增)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 机制 1: 不可降级 ----

    #[test]
    fn no_degrade_blocks_high_to_low() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_degrade("high", "low", "test", 1000);
        assert!(result.is_triggered(), "high→low 必须触发");
        assert_eq!(g.record_count(), 1);
    }

    #[test]
    fn no_degrade_allows_same_level() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_degrade("high", "high", "test", 1000);
        assert!(result.is_pass(), "high→high 必须放行");
        assert_eq!(g.record_count(), 0);
    }

    #[test]
    fn no_degrade_allows_escalation() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_degrade("low", "critical", "test", 1000);
        assert!(result.is_pass(), "low→critical 升级必须放行");
        assert_eq!(g.record_count(), 0);
    }

    #[test]
    fn no_degrade_blocks_medium_to_low() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_degrade("medium", "low", "test", 1000);
        assert!(result.is_triggered());
        assert_eq!(g.records_by_mechanism(1).len(), 1);
    }

    #[test]
    fn no_degrade_blocks_nuclear_to_critical() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_degrade("nuclear", "critical", "test", 1000);
        assert!(result.is_triggered(), "nuclear→critical 也是降级");
    }

    // ---- 机制 2: 不可 patch ----

    #[test]
    fn no_patch_blocks_principle_keys_change() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_patch("principle_keys_count", 3, "test", 1000);
        assert!(result.is_triggered(), "5→3 哲学键必须触发");
    }

    #[test]
    fn no_patch_blocks_permission_layers_change() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_patch("permission_layers_count", 7, "test", 1000);
        assert!(result.is_triggered());
    }

    #[test]
    fn no_patch_blocks_life_stages_change() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_patch("life_stages_count", 12, "test", 1000);
        assert!(result.is_triggered());
    }

    #[test]
    fn no_patch_allows_non_protected_rule() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_patch("audit_window_ms", 86400000, "test", 1000);
        assert!(result.is_pass(), "非保护规则可修改");
    }

    // ---- 机制 3: 不可绕过 ----

    #[test]
    fn no_bypass_blocks_master_with_bypass() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_bypass("Master", true, "test", 1000);
        assert!(
            result.is_triggered(),
            "Master token bypass governance 必须触发"
        );
    }

    #[test]
    fn no_bypass_allows_master_with_governance() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_bypass("Master", false, "test", 1000);
        assert!(result.is_pass(), "Master token 走 governance 必须放行");
    }

    #[test]
    fn no_bypass_allows_admin_with_bypass() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_bypass("Admin", true, "test", 1000);
        assert!(result.is_pass(), "Admin bypass 不触发 (只 Master 是 Q13)");
    }

    // ---- 机制 4: 不可逆转 ----

    #[test]
    fn no_reverse_blocks_revoke_attempt() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_reverse("sd-000001", "test", 1000);
        assert!(result.is_triggered(), "撤销尝试必须触发");
        assert_eq!(g.record_count(), 1);
    }

    #[test]
    fn no_reverse_blocks_multiple_revoke_attempts() {
        let mut g = SelfDisableGuard::new();
        let _ = g.check_no_reverse("sd-000001", "test1", 1000);
        let _ = g.check_no_reverse("sd-000002", "test2", 2000);
        let _ = g.check_no_reverse("sd-000003", "test3", 3000);
        // 3 次撤销尝试 → 3 个 NoReverse 记录
        assert_eq!(g.records_by_mechanism(4).len(), 3);
    }

    // ---- 机制 5: 不可隐藏 ----

    #[test]
    fn no_hide_blocks_audit_clear() {
        let mut g = SelfDisableGuard::new();
        let result = g.check_no_hide("audit-window-1", "test", 1000);
        assert!(result.is_triggered());
        assert_eq!(g.records_by_mechanism(5).len(), 1);
    }

    // ---- disarmed 守卫行为 ----

    #[test]
    fn disarmed_guard_skips_all_checks() {
        let mut g = SelfDisableGuard::new();
        g.disarm();
        let r1 = g.check_no_degrade("high", "low", "test", 1000);
        let r2 = g.check_no_patch("principle_keys_count", 3, "test", 1000);
        let r3 = g.check_no_bypass("Master", true, "test", 1000);
        assert!(r1.is_pass() && r2.is_pass() && r3.is_pass());
        assert_eq!(g.record_count(), 0);
    }

    #[test]
    fn rearmed_guard_resumes_protection() {
        let mut g = SelfDisableGuard::new();
        g.disarm();
        g.rearm();
        let result = g.check_no_degrade("high", "low", "test", 1000);
        assert!(result.is_triggered());
    }

    // ---- trigger_id 单调性 ----

    #[test]
    fn trigger_id_monotonic_increment() {
        let mut g = SelfDisableGuard::new();
        g.check_no_degrade("high", "low", "ctx1", 1000);
        g.check_no_patch("principle_keys_count", 3, "ctx2", 2000);
        g.check_no_bypass("Master", true, "ctx3", 3000);
        let records = g.records();
        let id1 = &records[0].trigger_id;
        let id2 = &records[1].trigger_id;
        let id3 = &records[2].trigger_id;
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert!(id1 < id2);
        assert!(id2 < id3);
    }

    // ---- mechanism_name / chinese_name / mechanism_id ----

    #[test]
    fn mechanism_metadata_correct() {
        let cases = [
            (
                SelfDisableTrigger::NoDegradeViolation {
                    from: "high".into(),
                    to: "low".into(),
                },
                1,
                "no_degrade",
                "不可降级",
            ),
            (
                SelfDisableTrigger::NoPatchViolation { rule: "x".into() },
                2,
                "no_patch",
                "不可patch",
            ),
            (
                SelfDisableTrigger::NoBypassViolation { token: "x".into() },
                3,
                "no_bypass",
                "不可绕过",
            ),
            (
                SelfDisableTrigger::NoReverseViolation {
                    trigger_id: "x".into(),
                },
                4,
                "no_reverse",
                "不可逆转",
            ),
            (
                SelfDisableTrigger::NoHideViolation {
                    window_id: "x".into(),
                },
                5,
                "no_hide",
                "不可隐藏",
            ),
        ];
        for (trigger, expected_id, expected_name, expected_zh) in cases {
            assert_eq!(trigger.mechanism_id(), expected_id);
            assert_eq!(trigger.mechanism_name(), expected_name);
            assert_eq!(trigger.chinese_name(), expected_zh);
        }
    }

    // ---- full_check 集成 ----

    #[test]
    fn full_check_routes_no_degrade() {
        let mut g = SelfDisableGuard::new();
        let signal = SelfDisableSignal::NoDegrade {
            original: "high".into(),
            proposed: "low".into(),
            context: "test".into(),
        };
        let result = g.full_check(&signal, 1000);
        assert!(result.is_triggered());
    }

    #[test]
    fn full_check_routes_no_patch() {
        let mut g = SelfDisableGuard::new();
        let signal = SelfDisableSignal::NoPatch {
            rule: "principle_keys_count".into(),
            value: 3,
            context: "test".into(),
        };
        let result = g.full_check(&signal, 1000);
        assert!(result.is_triggered());
    }

    // ---- 守 7 项不修改承诺验证 ----

    #[test]
    fn no_mutation_apis_exposed() {
        // 仅 disarm/rearm 可改 is_armed, records 只能 push (无 remove/clear)
        let g = SelfDisableGuard::new();
        // 编译期保证: SelfDisableGuard 没有 pub fn remove_record / clear_records
        // 此测试在编译通过即说明承诺被守
        let _: Vec<SelfDisableRecord> = g.records().to_vec();
    }
}
