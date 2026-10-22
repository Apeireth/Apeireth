//! Evidence-first 结果信封。
//!
//! 每个 recon 工具都返回 [`Evidence`]：结论（claim）、被观察对象（target）、
//! 授权范围（scope）、可复核证据（evidence）与显式局限（limitations）。
//! 这是 Apeireth「0 装 PASS」在 recon 工具面上的落地——没有证据不下结论，
//! 没覆盖到的面写进 limitations，而不是假装已覆盖。

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 一条可复核证据：字节、记录、文件或观察。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// 证据种类：`bytes` / `file` / `record` / `observation`。
    pub kind: String,
    /// 来源：路径、URL 或文件偏移（如 `0x1f0`）。
    pub source: String,
    /// 内容摘要（可用时为 SHA-256）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// 补充说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl EvidenceItem {
    /// 一条最小证据。
    pub fn new(kind: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            source: source.into(),
            digest: None,
            note: None,
        }
    }

    /// 附带摘要（builder 风格）。
    #[must_use]
    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }

    /// 附带说明（builder 风格）。
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// 一次分析结论及其证据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// 结论（一句话，可复核）。
    pub claim: String,
    /// 工具名（如 `re.binfmt`）。
    pub tool: String,
    /// 工具版本。
    pub tool_version: String,
    /// 被观察对象（路径、URL、地址）。
    pub target: String,
    /// 授权范围名（来自 [`crate::recon::Scope`]）。
    pub scope: String,
    /// 观察时间（RFC3339 UTC）。
    pub observed_at: String,
    /// 结构化细节。
    pub details: serde_json::Value,
    /// 证据列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceItem>,
    /// 显式局限与未测面。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

/// 信封校验失败。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvidenceError {
    /// claim 为空。
    #[error("evidence claim must not be empty")]
    EmptyClaim,
    /// tool 为空。
    #[error("evidence tool identity must not be empty")]
    EmptyTool,
    /// target 为空。
    #[error("evidence target must not be empty")]
    EmptyTarget,
    /// scope 为空。
    #[error("evidence scope must not be empty")]
    EmptyScope,
    /// observed_at 为空。
    #[error("evidence observed_at must not be empty")]
    EmptyObservedAt,
}

impl Evidence {
    /// 构造一条空证据信封（details 为 null，时间取当前 UTC）。
    pub fn new(
        tool: impl Into<String>,
        tool_version: impl Into<String>,
        target: impl Into<String>,
        scope: impl Into<String>,
        claim: impl Into<String>,
    ) -> Self {
        Self {
            claim: claim.into(),
            tool: tool.into(),
            tool_version: tool_version.into(),
            target: target.into(),
            scope: scope.into(),
            observed_at: rfc3339_utc(SystemTime::now()),
            details: serde_json::Value::Null,
            evidence: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// 设置结构化细节（builder 风格）。
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// 追加一条证据（builder 风格）。
    #[must_use]
    pub fn with_evidence(mut self, item: EvidenceItem) -> Self {
        self.evidence.push(item);
        self
    }

    /// 追加一条显式局限（builder 风格）。
    #[must_use]
    pub fn with_limitation(mut self, limitation: impl Into<String>) -> Self {
        self.limitations.push(limitation.into());
        self
    }

    /// 信封最低完整性校验：身份与目标不能为空。
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.claim.trim().is_empty() {
            return Err(EvidenceError::EmptyClaim);
        }
        if self.tool.trim().is_empty() {
            return Err(EvidenceError::EmptyTool);
        }
        if self.target.trim().is_empty() {
            return Err(EvidenceError::EmptyTarget);
        }
        if self.scope.trim().is_empty() {
            return Err(EvidenceError::EmptyScope);
        }
        if self.observed_at.trim().is_empty() {
            return Err(EvidenceError::EmptyObservedAt);
        }
        Ok(())
    }
}

/// 把 `SystemTime` 格式化为 RFC3339 UTC（秒精度，无外部时间依赖）。
///
/// 早于 Unix Epoch 的时间按 Epoch 处理（分析场景不产生历史时间戳）。
pub fn rfc3339_utc(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant 的 civil-from-days 算法（1970-01-01 起的天数 → 年月日）。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn envelope_round_trips_and_validates() {
        let evidence = Evidence::new("re.binfmt", "0.1.0", "/tmp/a.bin", "demo", "ELF64 PIE")
            .with_details(serde_json::json!({"format": "ELF64"}))
            .with_evidence(EvidenceItem::new("bytes", "0x0").with_digest("sha256:00"))
            .with_limitation("未执行动态分析");
        evidence.validate().expect("valid envelope");
        let json = serde_json::to_string(&evidence).expect("serialize");
        let back: Evidence = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, evidence);
    }

    #[test]
    fn empty_identity_is_rejected() {
        let mut evidence = Evidence::new("t", "v", "target", "scope", "claim");
        evidence.claim.clear();
        assert_eq!(evidence.validate(), Err(EvidenceError::EmptyClaim));

        let mut evidence = Evidence::new("t", "v", "target", "scope", "claim");
        evidence.target.clear();
        assert_eq!(evidence.validate(), Err(EvidenceError::EmptyTarget));
    }

    #[test]
    fn rfc3339_formats_known_instants() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let day = UNIX_EPOCH + Duration::from_secs(86_400);
        assert_eq!(rfc3339_utc(day), "1970-01-02T00:00:00Z");
    }
}
