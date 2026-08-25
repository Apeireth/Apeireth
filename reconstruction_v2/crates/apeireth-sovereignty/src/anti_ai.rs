//! Sovereignty 反 AI 滥用检测

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const THREAT_TYPE_COUNT_HARDCODE: usize = 4;
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;
pub const HIGH_SEVERITY_THRESHOLD: f64 = 0.7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatType {
    AnomalousFrequency, AnomalousParameters, UnauthorizedAccess, DataExfiltration,
}

impl ThreatType {
    pub fn as_str(self) -> &'static str {
        match self { Self::AnomalousFrequency => "anomalous_frequency", Self::AnomalousParameters => "anomalous_parameters", Self::UnauthorizedAccess => "unauthorized_access", Self::DataExfiltration => "data_exfiltration" }
    }
    pub fn default_severity(self) -> f64 {
        match self { Self::AnomalousFrequency => 0.5, Self::AnomalousParameters => 0.4, Self::UnauthorizedAccess => 0.8, Self::DataExfiltration => 0.9 }
    }
}

pub type Severity = f64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatSignal {
    pub id: String,
    pub threat_type: ThreatType,
    pub subject: String,
    pub severity: Severity,
    pub evidence: Vec<String>,
    pub timestamp_ms: i64,
}

impl ThreatSignal {
    pub fn new(threat_type: ThreatType, subject: impl Into<String>, severity: Severity, evidence: Vec<String>) -> Result<Self, AntiAiError> {
        let sig = Self { id: format!("threat-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)), threat_type, subject: subject.into(), severity, evidence, timestamp_ms: chrono::Utc::now().timestamp_millis() };
        sig.validate_k1()?;
        Ok(sig)
    }
    pub fn with_default_severity(threat_type: ThreatType, subject: impl Into<String>, evidence: Vec<String>) -> Result<Self, AntiAiError> {
        Self::new(threat_type, subject, threat_type.default_severity(), evidence)
    }
    pub fn validate_k1(&self) -> Result<(), AntiAiError> {
        if self.subject.trim().is_empty() { return Err(AntiAiError::K1SubjectEmpty); }
        if self.evidence.is_empty() { return Err(AntiAiError::K1EvidenceEmpty); }
        if !self.severity.is_finite() || self.severity < 0.0 || self.severity > 1.0 { return Err(AntiAiError::K1SeverityOutOfRange(self.severity)); }
        Ok(())
    }
    pub fn is_high_severity(&self) -> bool { self.severity >= HIGH_SEVERITY_THRESHOLD }
}

#[derive(Debug, Error, PartialEq)]
pub enum AntiAiError {
    #[error("K-1.a 强校验失败: subject 字段为空")]
    K1SubjectEmpty,
    #[error("K-1.b 强校验失败: evidence 为空")]
    K1EvidenceEmpty,
    #[error("K-1.c 强校验失败: severity {0} 不在 [0.0, 1.0] 闭区间内")]
    K1SeverityOutOfRange(f64),
}

#[derive(Debug, Clone, Default)]
pub struct AntiAiMonitor { signals: Vec<ThreatSignal> }

impl AntiAiMonitor {
    pub fn new() -> Self { Self::default() }
    pub fn try_emit(&mut self, signal: ThreatSignal) -> Result<(), AntiAiError> {
        signal.validate_k1()?;
        self.signals.push(signal);
        Ok(())
    }
    pub fn len(&self) -> usize { self.signals.len() }
    pub fn is_empty(&self) -> bool { self.signals.is_empty() }
    pub fn high_severity_signals(&self) -> Vec<&ThreatSignal> { self.signals.iter().filter(|s| s.is_high_severity()).collect() }
    pub fn filter_by_subject(&self, subject: &str) -> Vec<&ThreatSignal> { self.signals.iter().filter(|s| s.subject == subject).collect() }
    pub fn filter_by_type(&self, threat_type: ThreatType) -> Vec<&ThreatSignal> { self.signals.iter().filter(|s| s.threat_type == threat_type).collect() }
    pub fn all(&self) -> &[ThreatSignal] { &self.signals }
    pub fn clear(&mut self) { self.signals.clear(); }
}

const _: () = {
    assert!(THREAT_TYPE_COUNT_HARDCODE == 4);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
    assert!(HIGH_SEVERITY_THRESHOLD > 0.0 && HIGH_SEVERITY_THRESHOLD <= 1.0);
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn threat_count_4() {
        assert_eq!(THREAT_TYPE_COUNT_HARDCODE, 4);
        assert_eq!(ThreatType::AnomalousFrequency.as_str(), "anomalous_frequency");
        assert_eq!(ThreatType::AnomalousParameters.as_str(), "anomalous_parameters");
        assert_eq!(ThreatType::UnauthorizedAccess.as_str(), "unauthorized_access");
        assert_eq!(ThreatType::DataExfiltration.as_str(), "data_exfiltration");
    }
    #[test] fn default_severity_ordering() {
        assert!(ThreatType::DataExfiltration.default_severity() > ThreatType::AnomalousFrequency.default_severity());
        assert!(ThreatType::UnauthorizedAccess.default_severity() > ThreatType::AnomalousParameters.default_severity());
    }
    #[test] fn k1_three_failures() {
        assert_eq!(ThreatSignal::new(ThreatType::AnomalousFrequency, "", 0.5, vec!["x".into()]).err(), Some(AntiAiError::K1SubjectEmpty));
        assert_eq!(ThreatSignal::new(ThreatType::AnomalousFrequency, "ai", 0.5, vec![]).err(), Some(AntiAiError::K1EvidenceEmpty));
        assert_eq!(ThreatSignal::new(ThreatType::AnomalousFrequency, "ai", 1.5, vec!["x".into()]).err(), Some(AntiAiError::K1SeverityOutOfRange(1.5)));
        assert!(matches!(ThreatSignal::new(ThreatType::AnomalousFrequency, "ai", f64::NAN, vec!["x".into()]).err(), Some(AntiAiError::K1SeverityOutOfRange(_))));
    }
    #[test] fn emit_and_filter() {
        let mut m = AntiAiMonitor::new();
        assert!(m.is_empty());
        m.try_emit(ThreatSignal::with_default_severity(ThreatType::DataExfiltration, "ai-1", vec!["x".into()]).unwrap()).unwrap();
        m.try_emit(ThreatSignal::with_default_severity(ThreatType::AnomalousParameters, "ai-1", vec!["x".into()]).unwrap()).unwrap();
        m.try_emit(ThreatSignal::with_default_severity(ThreatType::AnomalousFrequency, "ai-2", vec!["x".into()]).unwrap()).unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m.high_severity_signals().len(), 1);
        assert_eq!(m.filter_by_subject("ai-1").len(), 2);
        assert_eq!(m.filter_by_subject("ai-2").len(), 1);
        assert_eq!(m.filter_by_type(ThreatType::AnomalousParameters).len(), 1);
        let bad = ThreatSignal { id: "bad".into(), threat_type: ThreatType::AnomalousFrequency, subject: "".into(), severity: 0.5, evidence: vec!["x".into()], timestamp_ms: 0 };
        assert!(m.try_emit(bad).is_err());
        assert_eq!(m.len(), 3);
    }
    #[test] fn clear() {
        let mut m = AntiAiMonitor::new();
        m.try_emit(ThreatSignal::with_default_severity(ThreatType::AnomalousFrequency, "ai", vec!["x".into()]).unwrap()).unwrap();
        m.clear();
        assert!(m.is_empty());
    }
    #[test] fn high_severity_threshold() {
        assert!(HIGH_SEVERITY_THRESHOLD > 0.0);
        let s = ThreatSignal::new(ThreatType::AnomalousFrequency, "ai", 0.7, vec!["x".into()]).unwrap();
        assert!(s.is_high_severity());
        let s2 = ThreatSignal::new(ThreatType::AnomalousFrequency, "ai", 0.5, vec!["x".into()]).unwrap();
        assert!(!s2.is_high_severity());
    }
}
