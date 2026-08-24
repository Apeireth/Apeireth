//! R22 ST-A2.3 — 涌现能力识别 (Emergence Detection)
//!
//! **深化层级** (per R22 路线 A, ST-A2.3):
//! - 现有 `observe_emergence` 是 placeholder, 仅 lib.rs 注释提及, 无实装
//! - 本模块提供 **EmergenceDetector** 实装, 跟踪 5 类涌现信号 + 阈值 + 报告
//!
//! **5 类涌现信号** (per architecture-v3-aircraft-carrier §2.1 涌现子组件):
//! 1. `CrossDomainInsight`  — 跨域洞察 (从 A 域抽出类比映射到 B 域)
//! 2. `AbstractionLevelShift` — 抽象层跃迁 (从实例升级到一般规律)
//! 3. `NoveltyConvergence`  — 新颖性收敛 (多个新想法汇聚成 1 个)
//! 4. `SelfOrganizingPattern` — 自组织模式 (无外部控制下产生结构)
//! 5. `RecursiveImprovement` — 递归改进 (在改过程中改自己的过程)
//!
//! **不修改承诺 (LOCKED)**:
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 5 类信号以工程近似实现 (per signal 有 heuristic confidence), 不假装真涌现测量
//! - 阈值 DEFAULT_EMERGENCE_THRESHOLD = 0.7, 低于阈值的信号记录但不报告
//! - in-memory 滑窗, 持久化留给 ST-A2.4 (emergence_stream)
//! - readiness: Ok (跟 apeireth-life-force 其他模块同水平)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// 默认涌现阈值 (signal.confidence >= threshold 才报告)
pub const DEFAULT_EMERGENCE_THRESHOLD: f64 = 0.7;
/// 默认 history 上限 (LRU 6 弹出)
pub const DEFAULT_MAX_SIGNAL_HISTORY: usize = 32;
/// 至少多少 evidence 才视为有效信号
pub const MIN_EVIDENCE_COUNT: usize = 2;

/// 5 类涌现信号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmergenceSignalType {
    /// 跨域洞察 (类比映射)
    CrossDomainInsight,
    /// 抽象层跃迁
    AbstractionLevelShift,
    /// 新颖性收敛
    NoveltyConvergence,
    /// 自组织模式
    SelfOrganizingPattern,
    /// 递归改进
    RecursiveImprovement,
}

impl EmergenceSignalType {
    /// 全部 5 类 (供完整性测试)
    pub const ALL: [EmergenceSignalType; 5] = [
        Self::CrossDomainInsight,
        Self::AbstractionLevelShift,
        Self::NoveltyConvergence,
        Self::SelfOrganizingPattern,
        Self::RecursiveImprovement,
    ];

    /// 标签 (供日志 / 审计)
    pub fn label(self) -> &'static str {
        match self {
            Self::CrossDomainInsight => "cross_domain_insight",
            Self::AbstractionLevelShift => "abstraction_level_shift",
            Self::NoveltyConvergence => "novelty_convergence",
            Self::SelfOrganizingPattern => "self_organizing_pattern",
            Self::RecursiveImprovement => "recursive_improvement",
        }
    }
}

impl fmt::Display for EmergenceSignalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 单条涌现信号
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmergenceSignal {
    pub signal_type: EmergenceSignalType,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub ts: i64,
    pub continuity_id: String,
}

impl EmergenceSignal {
    /// 是否应该报告 (confidence 满足阈值 + evidence 数 >= MIN_EVIDENCE_COUNT)
    pub fn should_report(&self, threshold: f64) -> bool {
        self.confidence >= threshold && self.evidence.len() >= MIN_EVIDENCE_COUNT
    }
}

/// 涌现报告 (从 detector snapshot 出来后供 L0 HA / supervision 消费)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmergenceReport {
    pub signals_above_threshold: Vec<EmergenceSignal>,
    pub total_signals_recorded: u64,
    pub threshold: f64,
    pub snapshot_at: i64,
}

/// 涌现检测器
///
/// **不假装**: 纯 in-memory 滑窗, 持久化留给 emergence_stream (ST-A2.4).
pub struct EmergenceDetector {
    pub threshold: f64,
    pub max_history: usize,
    pub continuity_id: String,
    signals: VecDeque<EmergenceSignal>,
    total_recorded: u64,
}

impl EmergenceDetector {
    /// 构造新 detector (默认阈值)
    pub fn new(continuity_id: impl Into<String>) -> Self {
        Self {
            threshold: DEFAULT_EMERGENCE_THRESHOLD,
            max_history: DEFAULT_MAX_SIGNAL_HISTORY,
            continuity_id: continuity_id.into(),
            signals: VecDeque::new(),
            total_recorded: 0,
        }
    }

    /// 自定义阈值构造
    pub fn with_threshold(continuity_id: impl Into<String>, threshold: f64) -> Self {
        let mut s = Self::new(continuity_id);
        s.threshold = threshold.clamp(0.0, 1.0);
        s
    }

    /// 校验 signal 合法
    fn validate(&self, signal: &EmergenceSignal) -> Result<(), EmergenceError> {
        if signal.continuity_id != self.continuity_id {
            return Err(EmergenceError::ContinuityMismatch {
                expected: self.continuity_id.clone(),
                actual: signal.continuity_id.clone(),
            });
        }
        if !(0.0..=1.0).contains(&signal.confidence) {
            return Err(EmergenceError::ConfidenceOutOfRange(signal.confidence));
        }
        Ok(())
    }

    /// 记录一条信号 (不管是否达阈值, 都进 history)
    pub fn record(&mut self, signal: EmergenceSignal) -> Result<(), EmergenceError> {
        self.validate(&signal)?;
        self.signals.push_back(signal);
        while self.signals.len() > self.max_history {
            self.signals.pop_front();
        }
        self.total_recorded += 1;
        Ok(())
    }

    /// 生成当前 snapshot (达阈值的信号列表)
    pub fn snapshot(&self, now: i64) -> EmergenceReport {
        let above: Vec<EmergenceSignal> = self
            .signals
            .iter()
            .filter(|s| s.should_report(self.threshold))
            .cloned()
            .collect();
        EmergenceReport {
            signals_above_threshold: above,
            total_signals_recorded: self.total_recorded,
            threshold: self.threshold,
            snapshot_at: now,
        }
    }

    /// 最近 N 条信号 (LIFO)
    pub fn recent(&self, n: usize) -> Vec<EmergenceSignal> {
        self.signals.iter().rev().take(n).cloned().collect()
    }

    /// 当前 history 长度
    pub fn len(&self) -> usize {
        self.signals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }
}

/// 涌现检测错误
#[derive(Debug, Clone, PartialEq)]
pub enum EmergenceError {
    /// 主体连续性 ID 不匹配
    ContinuityMismatch { expected: String, actual: String },
    /// confidence 越界
    ConfidenceOutOfRange(f64),
}

impl fmt::Display for EmergenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContinuityMismatch { expected, actual } => write!(
                f,
                "emergence continuity mismatch: expected={expected}, actual={actual}"
            ),
            Self::ConfidenceOutOfRange(v) => {
                write!(f, "emergence confidence out of [0.0, 1.0]: {v}")
            }
        }
    }
}

impl std::error::Error for EmergenceError {}

// ============================================
// 单元测试 (10+ tests)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(
        kind: EmergenceSignalType,
        confidence: f64,
        evidence: Vec<&str>,
        continuity_id: &str,
        ts: i64,
    ) -> EmergenceSignal {
        EmergenceSignal {
            signal_type: kind,
            confidence,
            evidence: evidence.into_iter().map(String::from).collect(),
            ts,
            continuity_id: continuity_id.to_string(),
        }
    }

    #[test]
    fn five_types_hardcoded() {
        assert_eq!(EmergenceSignalType::ALL.len(), 5);
    }

    #[test]
    fn five_labels_unique() {
        let labels: Vec<&str> = EmergenceSignalType::ALL.iter().map(|s| s.label()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 5);
    }

    #[test]
    fn signal_should_report_above_threshold_with_evidence() {
        let s = sig(
            EmergenceSignalType::CrossDomainInsight,
            0.8,
            vec!["a", "b"],
            "did:test",
            1000,
        );
        assert!(s.should_report(0.7));
    }

    #[test]
    fn signal_should_not_report_below_threshold() {
        let s = sig(
            EmergenceSignalType::CrossDomainInsight,
            0.5,
            vec!["a", "b"],
            "did:test",
            1000,
        );
        assert!(!s.should_report(0.7));
    }

    #[test]
    fn signal_should_not_report_with_insufficient_evidence() {
        let s = sig(
            EmergenceSignalType::CrossDomainInsight,
            0.9,
            vec!["only_one"],
            "did:test",
            1000,
        );
        assert!(!s.should_report(0.7), "evidence count < 2 不应报告");
    }

    #[test]
    fn detector_new_defaults() {
        let d = EmergenceDetector::new("did:test-001");
        assert_eq!(d.threshold, DEFAULT_EMERGENCE_THRESHOLD);
        assert_eq!(d.continuity_id, "did:test-001");
        assert!(d.is_empty());
        assert_eq!(d.total_recorded, 0);
    }

    #[test]
    fn detector_record_and_recent() {
        let mut d = EmergenceDetector::new("did:test-001");
        d.record(sig(
            EmergenceSignalType::CrossDomainInsight,
            0.8,
            vec!["a", "b"],
            "did:test-001",
            1000,
        ))
        .unwrap();
        d.record(sig(
            EmergenceSignalType::AbstractionLevelShift,
            0.9,
            vec!["c", "d", "e"],
            "did:test-001",
            1100,
        ))
        .unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d.total_recorded, 2);
        let recent = d.recent(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent[0].signal_type,
            EmergenceSignalType::AbstractionLevelShift
        );
    }

    #[test]
    fn detector_rejects_continuity_mismatch() {
        let mut d = EmergenceDetector::new("did:test-001");
        let bad = sig(
            EmergenceSignalType::CrossDomainInsight,
            0.8,
            vec!["a", "b"],
            "did:other-002",
            1000,
        );
        let res = d.record(bad);
        assert!(matches!(
            res,
            Err(EmergenceError::ContinuityMismatch { .. })
        ));
    }

    #[test]
    fn detector_rejects_confidence_out_of_range() {
        let mut d = EmergenceDetector::new("did:test-001");
        let bad = sig(
            EmergenceSignalType::CrossDomainInsight,
            1.5,
            vec!["a", "b"],
            "did:test-001",
            1000,
        );
        let res = d.record(bad);
        assert!(matches!(res, Err(EmergenceError::ConfidenceOutOfRange(_))));
    }

    #[test]
    fn detector_snapshot_filters_by_threshold() {
        let mut d = EmergenceDetector::with_threshold("did:test-001", 0.75);
        d.record(sig(
            EmergenceSignalType::CrossDomainInsight,
            0.5,
            vec!["a", "b"],
            "did:test-001",
            1000,
        ))
        .unwrap(); // below
        d.record(sig(
            EmergenceSignalType::AbstractionLevelShift,
            0.8,
            vec!["c", "d"],
            "did:test-001",
            1100,
        ))
        .unwrap(); // above
        d.record(sig(
            EmergenceSignalType::NoveltyConvergence,
            0.6,
            vec!["e", "f"],
            "did:test-001",
            1200,
        ))
        .unwrap(); // below threshold
        let report = d.snapshot(1300);
        assert_eq!(
            report.signals_above_threshold.len(),
            1,
            "0.5 和 0.7 < 0.75 不报, 只有 0.8 ≥ 0.75 报"
        );
        assert_eq!(report.total_signals_recorded, 3);
        assert_eq!(report.threshold, 0.75);
    }

    #[test]
    fn detector_history_lru_eviction() {
        let mut d = EmergenceDetector::new("did:test-001");
        d.max_history = 3;
        for i in 0..5 {
            d.record(sig(
                EmergenceSignalType::CrossDomainInsight,
                0.8,
                vec!["a", "b"],
                "did:test-001",
                1000 + i,
            ))
            .unwrap();
        }
        assert_eq!(d.len(), 3);
        assert_eq!(d.total_recorded, 5);
    }

    #[test]
    fn with_threshold_clamps_to_range() {
        let d = EmergenceDetector::with_threshold("did:test", 1.5);
        assert_eq!(d.threshold, 1.0);
        let d2 = EmergenceDetector::with_threshold("did:test", -0.5);
        assert_eq!(d2.threshold, 0.0);
    }
}
