//! D-2：`rubric::Ballot` 的 council 接线口径（`from_advice`）。
//!
//! 这里只从**公开 API 外部**验证（`mod tests` 里的单测已经覆盖内部细节）：
//!
//! 1. `Ballot::from_advice` 对外可用，且显式/缺省置信度能被区分；
//! 2. `confidence_defaulted` 能序列化；
//! 3. **向后兼容**：旧 JSON（没有 `confidence_defaulted` 键）仍能读出来，
//!    且读成 `false`（旧数据里的置信度都是显式给的）。

use apeireth_governance::rubric::{AdvisorDomain, Ballot, StanceKind};

#[test]
fn from_advice_is_publicly_reachable_and_marks_defaulted_confidence() {
    let explicit = Ballot::from_advice(
        "advisor-safety",
        StanceKind::StrongApprove,
        Some(0.9),
        0.2,
        1.0,
        AdvisorDomain::Safety,
    );
    assert!(!explicit.confidence_defaulted);
    assert!((explicit.confidence - 0.9).abs() < 1e-9);

    let defaulted = Ballot::from_advice(
        "advisor-history",
        StanceKind::Approve,
        None,
        0.35,
        0.55,
        AdvisorDomain::History,
    );
    assert!(defaulted.confidence_defaulted);
    assert!((defaulted.confidence - 0.35).abs() < 1e-9);
    assert_eq!(defaulted.domain, Some(AdvisorDomain::History));
}

#[test]
fn confidence_defaulted_survives_serde_round_trip() {
    let defaulted = Ballot::from_advice(
        "advisor-legal",
        StanceKind::Neutral,
        None,
        0.5,
        0.85,
        AdvisorDomain::Legal,
    );
    let json = serde_json::to_string(&defaulted).expect("Ballot 应当可以序列化");
    assert!(
        json.contains("\"confidence_defaulted\":true"),
        "序列化结果里必须带上 confidence_defaulted: {json}"
    );
    let back: Ballot = serde_json::from_str(&json).expect("Ballot 应当可以反序列化");
    assert_eq!(back, defaulted);
    assert!(back.confidence_defaulted);
}

#[test]
fn legacy_json_without_confidence_defaulted_still_deserializes() {
    // 旧 JSON：没有 confidence_defaulted 键。
    let legacy = r#"{
        "advisor_id": "advisor-safety",
        "stance": "StrongApprove",
        "confidence": 0.75,
        "weight": 1.0
    }"#;
    let parsed: Ballot = serde_json::from_str(legacy).expect("旧 JSON 仍应可读");
    assert!(!parsed.confidence_defaulted, "旧数据里的置信度视为显式给出");
    assert!((parsed.confidence - 0.75).abs() < 1e-9);
    assert_eq!(parsed.advisor_id, "advisor-safety");
    assert_eq!(parsed.stance, StanceKind::StrongApprove);
}
