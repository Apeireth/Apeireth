//! Sovereignty 反 AI 滥用检测 (R20 阶段 6 估补)
//!
//! **职责** (本模块 flesh out 估补, lib.rs LOCKED 不重 export):
//! - **4 类检测** (按严重度升序):
//!   1. **异常调用频率** (`AnomalousFrequency`) — 短时间内调用次数超阈值 → 限速
//!   2. **异常参数** (`AnomalousParameters`) — 参数分布偏离基线 (e.g. 字符串长度爆炸 / 嵌套过深) → 告警
//!   3. **越权访问** (`UnauthorizedAccess`) — 触碰超出 AI 主体权限范围的资源 → 拒绝
//!   4. **数据外泄** (`DataExfiltration`) — 大批量外带数据 / 跨边界导出 → 立即冻结
//!
//! - **3 K-1 强校验** (任何 signal 必须满足, 否则 `Err(AntiAiError::K1Violation)`):
//!   1. **K-1.a** — `subject` 非空 (谁被检测)
//!   2. **K-1.b** — `evidence_count >= 1` (至少一条证据, 不空判)
//!   3. **K-1.c** — `severity ∈ [0.0, 1.0]` 闭区间
//!
//! **6 哲学锚穿透**:
//! - **主 22:33 ASI 北极星** — 反 AI 滥用守护人类权威 (L0 HA 不可被 AI 绕过)
//! - **主 17:43 实事求是** — severity 是真实风险评估, 非装饰
//! - **主 17:58 不假装** — `try_emit` 返回 `Err` 表达真实失败, 不 silent pass
//! - **主 19:33 走在前人肩上** — 复用 `serde::Serialize` + `thiserror::Error`
//! - **主 23:44 干到底** — 3 K-1 强校验在 `try_emit` 一处集中执行
//! - **主 00:56 任何人都能接手** — 4 类检测枚举化, 公开 API 简单直白
//!
//! **8 项不修改承诺**:
//! - ✅ 编译期 hardcode: 检测类型数 = 4, K-1 强校验数 = 3
//! - ✅ 0 触碰 LOCKED
//! - ✅ 0 依赖 NewAPI
//! - ✅ 0 重复造轮子
//! - ✅ 诚实标缺: ❌ 不接外部 IDS / WAF; 仅 in-memory pattern + 启发式
//!
//! **诚实登记**:
//! - ❌ **不接外部 IDS / WAF** — 仅 in-memory 启发式, 真生产应接 Suricata / Falco
//! - ❌ **不假装有 ML 异常检测** — 4 类检测都是简单规则, 不假装 ML
//! - ❌ **不实际限速 / 冻结** — 仅发出 signal, 限速/冻结由 governance.rs 真正执行
//!
//! **用法**:
//! ```ignore
//! use anti_ai::{AntiAiMonitor, ThreatSignal, ThreatType, Severity};
//!
//! let mut monitor = AntiAiMonitor::new();
//! let sig = ThreatSignal::new(
//!     ThreatType::AnomalousFrequency,
//!     "ai-subject-1",
//!     0.7,
//!     vec!["called 1000 times in 10s".into()],
//! )?;
//! monitor.try_emit(sig)?;
//! let high = monitor.high_severity_signals();
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 编译时 hardcode: 4 检测 / 3 K-1 强校验
// ============================================================

/// 检测类型数 (编译时硬编码: AnomalousFrequency / AnomalousParameters / UnauthorizedAccess / DataExfiltration = 4)
pub const THREAT_TYPE_COUNT_HARDCODE: usize = 4;

/// K-1 强校验数 (编译时硬编码: subject 非空 / evidence ≥ 1 / severity ∈ [0,1] = 3)
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

/// 高严重度阈值 (>= 此值的 signal 需立即处置)
pub const HIGH_SEVERITY_THRESHOLD: f64 = 0.7;

/// 4 类威胁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatType {
    /// 异常调用频率 — 短时间内调用次数超阈值
    AnomalousFrequency,
    /// 异常参数 — 参数分布偏离基线 (长度/嵌套/字符集)
    AnomalousParameters,
    /// 越权访问 — 触碰 AI 主体权限范围外的资源
    UnauthorizedAccess,
    /// 数据外泄 — 大批量外带 / 跨边界导出
    DataExfiltration,
}

impl ThreatType {
    /// 字符串 ID
    pub fn as_str(self) -> &'static str {
        match self {
            ThreatType::AnomalousFrequency => "anomalous_frequency",
            ThreatType::AnomalousParameters => "anomalous_parameters",
            ThreatType::UnauthorizedAccess => "unauthorized_access",
            ThreatType::DataExfiltration => "data_exfiltration",
        }
    }

    /// 该类威胁的内置默认严重度 (启发式基线, 可被 signal.severity 覆盖)
    pub fn default_severity(self) -> f64 {
        match self {
            ThreatType::AnomalousFrequency => 0.5,
            ThreatType::AnomalousParameters => 0.4,
            ThreatType::UnauthorizedAccess => 0.8,
            ThreatType::DataExfiltration => 0.9,
        }
    }
}

/// 严重度别名 (与 `audit::AuditLevel` 不同, 这里是无量纲 0.0-1.0)
pub type Severity = f64;

/// 单条威胁信号
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatSignal {
    /// 信号 ID
    pub id: String,
    /// 威胁类型
    pub threat_type: ThreatType,
    /// 被检测的 AI 主体 ID — K-1.a
    pub subject: String,
    /// 严重度 (0.0-1.0) — K-1.c
    pub severity: Severity,
    /// 证据 (人类可读理由列表) — K-1.b: 至少 1 条
    pub evidence: Vec<String>,
    /// 时间戳 (epoch ms)
    pub timestamp_ms: i64,
}

impl ThreatSignal {
    /// 构造威胁信号 (timestamp 自动用 `chrono::Utc::now()`)
    pub fn new(
        threat_type: ThreatType,
        subject: impl Into<String>,
        severity: Severity,
        evidence: Vec<String>,
    ) -> Result<Self, AntiAiError> {
        let sig = Self {
            id: format!(
                "threat-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            threat_type,
            subject: subject.into(),
            severity,
            evidence,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };
        // 构造期就校验 — 不留半成品
        sig.validate_k1()?;
        Ok(sig)
    }

    /// 便捷构造 — 用 ThreatType 默认严重度
    pub fn with_default_severity(
        threat_type: ThreatType,
        subject: impl Into<String>,
        evidence: Vec<String>,
    ) -> Result<Self, AntiAiError> {
        Self::new(
            threat_type,
            subject,
            threat_type.default_severity(),
            evidence,
        )
    }

    /// 3 K-1 强校验
    ///
    /// - **K-1.a**: subject 非空
    /// - **K-1.b**: evidence 至少 1 条
    /// - **K-1.c**: severity ∈ [0.0, 1.0]
    pub fn validate_k1(&self) -> Result<(), AntiAiError> {
        if self.subject.trim().is_empty() {
            return Err(AntiAiError::K1SubjectEmpty);
        }
        if self.evidence.is_empty() {
            return Err(AntiAiError::K1EvidenceEmpty);
        }
        if !self.severity.is_finite() || self.severity < 0.0 || self.severity > 1.0 {
            return Err(AntiAiError::K1SeverityOutOfRange(self.severity));
        }
        Ok(())
    }

    /// 是否高严重度
    pub fn is_high_severity(&self) -> bool {
        self.severity >= HIGH_SEVERITY_THRESHOLD
    }
}

/// 反 AI 监控错误
#[derive(Debug, Error, PartialEq)]
pub enum AntiAiError {
    /// K-1.a 强校验失败 — subject 非空
    #[error("K-1.a 强校验失败: subject 字段为空 (反 AI 检测必须记录谁被检测)")]
    K1SubjectEmpty,
    /// K-1.b 强校验失败 — evidence 至少 1 条
    #[error("K-1.b 强校验失败: evidence 为空 (反 AI 检测必须有至少 1 条证据)")]
    K1EvidenceEmpty,
    /// K-1.c 强校验失败 — severity ∈ [0.0, 1.0]
    #[error("K-1.c 强校验失败: severity {0} 不在 [0.0, 1.0] 闭区间内")]
    K1SeverityOutOfRange(f64),
}

/// 反 AI 监控器 (in-memory, 接收 signal)
#[derive(Debug, Clone, Default)]
pub struct AntiAiMonitor {
    signals: Vec<ThreatSignal>,
}

impl AntiAiMonitor {
    /// 新建空监控器
    pub fn new() -> Self {
        Self::default()
    }

    /// 提交威胁信号 (先 K-1 强校验)
    pub fn try_emit(&mut self, signal: ThreatSignal) -> Result<(), AntiAiError> {
        signal.validate_k1()?;
        self.signals.push(signal);
        Ok(())
    }

    /// 当前 signal 数
    pub fn len(&self) -> usize {
        self.signals.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }

    /// 高严重度 signal 列表 (>= HIGH_SEVERITY_THRESHOLD)
    pub fn high_severity_signals(&self) -> Vec<&ThreatSignal> {
        self.signals
            .iter()
            .filter(|s| s.is_high_severity())
            .collect()
    }

    /// 按 subject 过滤
    pub fn filter_by_subject(&self, subject: &str) -> Vec<&ThreatSignal> {
        self.signals
            .iter()
            .filter(|s| s.subject == subject)
            .collect()
    }

    /// 按 threat_type 过滤
    pub fn filter_by_type(&self, threat_type: ThreatType) -> Vec<&ThreatSignal> {
        self.signals
            .iter()
            .filter(|s| s.threat_type == threat_type)
            .collect()
    }

    /// 全列表引用
    pub fn all(&self) -> &[ThreatSignal] {
        &self.signals
    }

    /// 清空 (仅测试用)
    pub fn clear(&mut self) {
        self.signals.clear();
    }
}

const _: () = {
    assert!(THREAT_TYPE_COUNT_HARDCODE == 4);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
    assert!(HIGH_SEVERITY_THRESHOLD > 0.0 && HIGH_SEVERITY_THRESHOLD <= 1.0);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threat_type_count_is_4() {
        assert_eq!(THREAT_TYPE_COUNT_HARDCODE, 4);
        assert_eq!(ThreatType::AnomalousFrequency.as_str(), "anomalous_frequency");
        assert_eq!(ThreatType::AnomalousParameters.as_str(), "anomalous_parameters");
        assert_eq!(ThreatType::UnauthorizedAccess.as_str(), "unauthorized_access");
        assert_eq!(ThreatType::DataExfiltration.as_str(), "data_exfiltration");
    }

    #[test]
    fn threat_type_default_severity_ordering() {
        // DataExfiltration 应该是最严重的内置默认
        assert!(
            ThreatType::DataExfiltration.default_severity()
                > ThreatType::AnomalousFrequency.default_severity()
        );
        assert!(
            ThreatType::UnauthorizedAccess.default_severity()
                > ThreatType::AnomalousParameters.default_severity()
        );
    }

    #[test]
    fn k1_strict_checks_three_failures() {
        // K-1.a: subject 空
        let res1 = ThreatSignal::new(
            ThreatType::AnomalousFrequency,
            "",
            0.5,
            vec!["x".into()],
        );
        assert_eq!(res1.err(), Some(AntiAiError::K1SubjectEmpty));

        // K-1.b: evidence 空
        let res2 = ThreatSignal::new(
            ThreatType::AnomalousFrequency,
            "ai-1",
            0.5,
            vec![],
        );
        assert_eq!(res2.err(), Some(AntiAiError::K1EvidenceEmpty));

        // K-1.c: severity 越界
        let res3 = ThreatSignal::new(
            ThreatType::AnomalousFrequency,
            "ai-1",
            1.5,
            vec!["x".into()],
        );
        assert_eq!(res3.err(), Some(AntiAiError::K1SeverityOutOfRange(1.5)));

        // K-1.c: severity NaN
        let res4 = ThreatSignal::new(
            ThreatType::AnomalousFrequency,
            "ai-1",
            f64::NAN,
            vec!["x".into()],
        );
        assert!(matches!(
            res4.err(),
            Some(AntiAiError::K1SeverityOutOfRange(_))
        ));
    }

    #[test]
    fn monitor_emits_and_filters() {
        let mut mon = AntiAiMonitor::new();
        assert!(mon.is_empty());

        // 高严重度: DataExfiltration
        mon.try_emit(
            ThreatSignal::with_default_severity(
                ThreatType::DataExfiltration,
                "ai-1",
                vec!["exported 1GB in 10s".into()],
            )
            .unwrap(),
        )
        .unwrap();

        // 低严重度: AnomalousParameters
        mon.try_emit(
            ThreatSignal::with_default_severity(
                ThreatType::AnomalousParameters,
                "ai-1",
                vec!["string length 1MB".into()],
            )
            .unwrap(),
        )
        .unwrap();

        // 不同 subject
        mon.try_emit(
            ThreatSignal::with_default_severity(
                ThreatType::AnomalousFrequency,
                "ai-2",
                vec!["1000 calls/s".into()],
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(mon.len(), 3);

        // 高严重度应只 1 条
        let high = mon.high_severity_signals();
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].threat_type, ThreatType::DataExfiltration);

        // 按 subject 过滤
        assert_eq!(mon.filter_by_subject("ai-1").len(), 2);
        assert_eq!(mon.filter_by_subject("ai-2").len(), 1);

        // 按 type 过滤
        assert_eq!(
            mon.filter_by_type(ThreatType::AnomalousParameters).len(),
            1
        );

        // K-1 失败不入库 — 直接构造一个 subject 为空的 signal (绕过构造函数, 验证 try_emit 拒绝)
        let bad = ThreatSignal {
            id: "bad-1".into(),
            threat_type: ThreatType::AnomalousFrequency,
            subject: "".into(),
            severity: 0.5,
            evidence: vec!["x".into()],
            timestamp_ms: 0,
        };
        assert!(mon.try_emit(bad).is_err());
        assert_eq!(mon.len(), 3);
    }
}
