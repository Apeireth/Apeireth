//! apeireth-motivation 集成测试 (≥ 1 测试, 跨函数 + 跨模块).
//!
//! 覆盖:
//! - 完整 write_flow 端到端 (C-SGI-1~7 串联)
//! - V0.5 v2 §13 motivation_score 端到端
//! - ReflectionAuditor 触发链路
//! - 失败回滚 / 静默告警

use apeireth_motivation::{
    motivation_score, write_flow, AuditEvent, AutonomyConsistency, Evidence, EvidenceKind,
    ExternalDrive, InternalDrive, IntrinsicIntensity, MotivationError, ReflectionAuditor,
    SGIContent, SGIEntry, SGIStructured, ValueStability, MIN_EVIDENCE_SCORE, SGI,
    SGI_MAX_TEXT_CHARS,
};
use std::collections::HashMap;

/// 测试用 helper: 有效 E 层证据 (3 类, 0.9 权重).
fn good_evidences() -> Vec<Evidence> {
    vec![
        Evidence {
            kind: EvidenceKind::Council,
            source: "int_council".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::History,
            source: "int_history".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Principle,
            source: "int_principle".into(),
            weight: 0.9,
        },
    ]
}

/// 测试用 helper: 构造有效 SGIEntry (Structured + 三件套).
fn make_entry(goal: &str, drive_label: &str, intensity: f64, drive_kind: &str) -> SGIEntry {
    let content = SGIContent::Structured(SGIStructured {
        goal: goal.to_string(),
        deadline: "2026-12-31".to_string(),
        success_criteria: "int_test_passes".to_string(),
        extras: HashMap::new(),
        multimodal: None,
    });
    let mut entry = match drive_kind {
        "external" => {
            let drive = ExternalDrive::new(drive_label, intensity);
            SGIEntry::new(content, &drive)
        }
        _ => {
            let drive = InternalDrive::new(drive_label, intensity);
            SGIEntry::new(content, &drive)
        }
    };
    entry.evidence_refs = good_evidences();
    entry
}

/// 集成测试 1: 完整 SGI 生命周期 — 多次写入 + 验证 predecessor 链 + 审计器联动.
#[test]
fn integration_sgi_full_lifecycle() {
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();

    // 写入 1: 初始目标
    let e1 = make_entry("goal_init", "drive_init", 0.7, "internal");
    let r1 = write_flow(&mut sgi, e1, &good_evidences(), &mut auditor, false)
        .expect("first write must succeed");

    // 写入 2: 不同目标
    let e2 = make_entry("goal_reflect", "drive_reflect", 0.85, "internal");
    let r2 = write_flow(&mut sgi, e2, &good_evidences(), &mut auditor, false)
        .expect("second write must succeed");

    // 写入 3: 外驱目标
    let e3 = make_entry("goal_user_cmd", "drive_user", 0.6, "external");
    let r3 = write_flow(&mut sgi, e3, &good_evidences(), &mut auditor, false)
        .expect("third write must succeed (external drive still valid)");

    // 历史应为 3 条, 当前为 r3
    assert_eq!(sgi.history_len(), 3);
    assert_eq!(sgi.current().unwrap().id, r3.entry_id);

    // predecessor 链: e2.predecessor = e1.id, e3.predecessor = e2.id
    let hist = sgi.history();
    assert_eq!(hist[0].id, r1.entry_id);
    assert_eq!(hist[0].predecessor, None);
    assert_eq!(hist[1].id, r2.entry_id);
    assert_eq!(hist[1].predecessor, Some(r1.entry_id));
    assert_eq!(hist[2].id, r3.entry_id);
    assert_eq!(hist[2].predecessor, Some(r2.entry_id));

    // 审计器: 应有 3 条 HistoryAppended 事件
    let appended = auditor
        .events()
        .iter()
        .filter(|e| matches!(e, AuditEvent::HistoryAppended { .. }))
        .count();
    assert_eq!(appended, 3);
    assert_eq!(auditor.silent_alert_count(), 0);
    assert_eq!(auditor.history_failed_count(), 0);

    // evidence_score 应 ≥ MIN_EVIDENCE_SCORE
    assert!(r1.evidence_score >= MIN_EVIDENCE_SCORE);
    assert!(r2.evidence_score >= MIN_EVIDENCE_SCORE);
    assert!(r3.evidence_score >= MIN_EVIDENCE_SCORE);
}

/// 集成测试 2: C-SGI-7 三条必备完整链路 — FreeText 解析 + Structured 字段校验.
#[test]
fn integration_csgi7_three_required_across_kinds() {
    // Structured 缺 deadline → 失败
    let drive = InternalDrive::new("d", 0.5);
    let bad_structured = SGIContent::Structured(SGIStructured {
        goal: "ok".to_string(),
        deadline: "".to_string(), // 缺失
        success_criteria: "ok".to_string(),
        extras: HashMap::new(),
        multimodal: None,
    });
    let entry = SGIEntry::new(bad_structured, &drive);
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();
    let err = write_flow(&mut sgi, entry, &good_evidences(), &mut auditor, false);
    assert!(matches!(err, Err(MotivationError::MissingRequired(_))));

    // FreeText 缺 # DEADLINE → 失败
    let bad_freetext = SGIContent::FreeText("# goal: ok\n# success: ok\n".to_string());
    let entry2 = SGIEntry::new(bad_freetext, &drive);
    let err2 = write_flow(&mut sgi, entry2, &good_evidences(), &mut auditor, false);
    assert!(matches!(err2, Err(MotivationError::MissingRequired(_))));

    // FreeText 三段齐全 → 通过 (先 reset)
    let good_freetext = SGIContent::FreeText(
        "# goal: learn rust\n# deadline: 2026-12-31\n# success: all tests pass\n".to_string(),
    );
    let mut entry3 = SGIEntry::new(good_freetext, &drive);
    entry3.evidence_refs = good_evidences();
    let ok = write_flow(&mut sgi, entry3, &good_evidences(), &mut auditor, false);
    assert!(ok.is_ok(), "freetext with all 3 markers must pass: {ok:?}");
}

/// 集成测试 3: C-SGI-6 边界 — 恰好 SGI_MAX_TEXT_CHARS 通过, SGI_MAX_TEXT_CHARS+1 拒绝.
#[test]
fn integration_csgi6_max_chars_boundary() {
    let drive = InternalDrive::new("d", 0.5);
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();

    // 边界 — 恰好 N 字符 (前置 # goal / # deadline / # success 段头, 否则 C-SGI-7 先报)
    let prefix = "# goal: ok\n# deadline: 2026-12-31\n# success: ok\n";
    let pad_ok_len = SGI_MAX_TEXT_CHARS.saturating_sub(prefix.len());
    let text_ok = format!("{prefix}{}", "x".repeat(pad_ok_len));
    assert_eq!(text_ok.chars().count(), SGI_MAX_TEXT_CHARS);
    let mut e_ok = SGIEntry::new(SGIContent::FreeText(text_ok), &drive);
    e_ok.evidence_refs = good_evidences();
    let r_ok = write_flow(&mut sgi, e_ok, &good_evidences(), &mut auditor, false);
    assert!(r_ok.is_ok(), "exact max chars must pass: {r_ok:?}");

    // 越界 — N+1 字符 (新 SGI, 重置)
    let mut sgi2 = SGI::new();
    let text_over = format!("{prefix}{}", "x".repeat(pad_ok_len + 1));
    assert_eq!(text_over.chars().count(), SGI_MAX_TEXT_CHARS + 1);
    let mut e_over = SGIEntry::new(SGIContent::FreeText(text_over), &drive);
    e_over.evidence_refs = good_evidences();
    let r_over = write_flow(&mut sgi2, e_over, &good_evidences(), &mut auditor, false);
    assert!(matches!(r_over, Err(MotivationError::TextTooLong { .. })));
    // 失败应不写入 sgi_history
    assert_eq!(sgi2.history_len(), 0);
}

/// 集成测试 4: C-SGI-3 E 层 — 校验 distinct EvidenceKind 数量约束.
#[test]
fn integration_csgi3_evidence_kinds_count() {
    // MIN_EVIDENCE_KINDS = 3, 但允许更多 (Council/History/Principle/Permission/Human/Audit 共 6 类)
    let drive = InternalDrive::new("d", 0.5);
    let entry = SGIEntry::new(
        SGIContent::Structured(SGIStructured {
            goal: "ok".to_string(),
            deadline: "2026-12-31".to_string(),
            success_criteria: "ok".to_string(),
            extras: HashMap::new(),
            multimodal: None,
        }),
        &drive,
    );

    // 6 类全有 → 最高覆盖
    let six_kinds = vec![
        Evidence {
            kind: EvidenceKind::Council,
            source: "c".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::History,
            source: "h".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Principle,
            source: "p".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Permission,
            source: "pm".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Human,
            source: "hu".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Audit,
            source: "a".into(),
            weight: 0.9,
        },
    ];
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();
    let r = write_flow(&mut sgi, entry.clone(), &six_kinds, &mut auditor, false);
    assert!(r.is_ok(), "6 distinct kinds must pass: {r:?}");

    // 同种类重复 — 不增加 distinct count, 2 类 → 失败
    let mut sgi2 = SGI::new();
    let dup_kinds = vec![
        Evidence {
            kind: EvidenceKind::Council,
            source: "c1".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Council, // 重复
            source: "c2".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::History,
            source: "h".into(),
            weight: 0.9,
        },
    ];
    let r2 = write_flow(&mut sgi2, entry, &dup_kinds, &mut auditor, false);
    assert!(matches!(r2, Err(MotivationError::EvidenceInsufficient(_))));
}

/// 集成测试 5: V0.5 v2 §13 motivation_score + 跨权重组合.
#[test]
fn integration_v05_motivation_score_weighted() {
    // 健康代理 + 高权重三维
    let healthy = motivation_score(
        AutonomyConsistency {
            internal_intensity: 1.0,
            internal_history_ratio: 1.0,
        },
        ValueStability {
            goal_turnover: 0.0,
            deadline_variance: 0.0,
        },
        IntrinsicIntensity {
            current_internal: 1.0,
            historical_peak: 1.0,
        },
    );
    assert!(healthy.passes_threshold);
    assert!((healthy.total - 1.0).abs() < 1e-9);

    // 全 0 → 0
    let zero = motivation_score(
        AutonomyConsistency {
            internal_intensity: 0.0,
            internal_history_ratio: 0.0,
        },
        ValueStability {
            goal_turnover: 1.0,
            deadline_variance: 1.0,
        },
        IntrinsicIntensity {
            current_internal: 0.0,
            historical_peak: 0.0,
        },
    );
    assert!((zero.total - 0.0).abs() < 1e-9);
    assert!(!zero.passes_threshold);

    // 中等: 三维各 0.5 → 总分 ≈ 0.5
    let mid = motivation_score(
        AutonomyConsistency {
            internal_intensity: 0.5,
            internal_history_ratio: 0.5,
        },
        ValueStability {
            goal_turnover: 0.5,
            deadline_variance: 0.5,
        },
        IntrinsicIntensity {
            current_internal: 0.5,
            historical_peak: 0.5,
        },
    );
    let (w1, w2, w3) = apeireth_motivation::MOTIVATION_WEIGHTS;
    // autonomy = sqrt(0.25) = 0.5
    // value = ((1-0.5) + (1-0.5)) / 2 = 0.5
    // intrinsic = (0.5+0.5)/2 = 0.5
    // total = 0.35*0.5 + 0.35*0.5 + 0.30*0.5 = 0.5
    let expected = w1 * 0.5 + w2 * 0.5 + w3 * 0.5;
    assert!(
        (mid.total - expected).abs() < 1e-9,
        "got {} expected {}",
        mid.total,
        expected
    );
}

/// 集成测试 6: 失败不污染 sgi_history (原子性).
#[test]
fn integration_failure_does_not_corrupt_history() {
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();

    // 成功 1
    let e1 = make_entry("goal_ok_1", "d", 0.5, "internal");
    write_flow(&mut sgi, e1, &good_evidences(), &mut auditor, false).expect("first must succeed");

    // 失败 1: C-SGI-7 缺 deadline
    let drive = InternalDrive::new("d", 0.5);
    let bad = SGIContent::Structured(SGIStructured {
        goal: "ok".to_string(),
        deadline: "".to_string(),
        success_criteria: "ok".to_string(),
        extras: HashMap::new(),
        multimodal: None,
    });
    let mut entry_bad = SGIEntry::new(bad, &drive);
    entry_bad.evidence_refs = good_evidences();
    let r = write_flow(&mut sgi, entry_bad, &good_evidences(), &mut auditor, false);
    assert!(r.is_err());

    // 失败 2: C-SGI-3 证据不足
    let e2 = make_entry("goal_2", "d", 0.5, "internal");
    let weak = vec![Evidence {
        kind: EvidenceKind::Council,
        source: "c".into(),
        weight: 0.9,
    }];
    let r2 = write_flow(&mut sgi, e2, &weak, &mut auditor, false);
    assert!(r2.is_err());

    // 历史应仍为 1 条 (仅成功 1)
    assert_eq!(sgi.history_len(), 1);
    let cur = sgi.current().expect("current must still be set");
    assert_eq!(cur.goal_in_structured(), "goal_ok_1");
}

/// 内部 helper extension — 仅测试中读取 Structured.goal.
trait SGIEntryGoalExt {
    fn goal_in_structured(&self) -> String;
}

impl SGIEntryGoalExt for SGIEntry {
    fn goal_in_structured(&self) -> String {
        match &self.content {
            SGIContent::Structured(s) => s.goal.clone(),
            _ => panic!("expected Structured content"),
        }
    }
}
