//! apeireth-motivation demo — 演示 SGI 单字段 + C-SGI-1~7 七条硬约束 +
//! E 层校验 + ReflectionAuditor 告警 + §13 动机/价值测度.
//!
//! 运行: `cargo run -p apeireth-motivation --example motivation_demo`

use apeireth_motivation::{
    motivation_score, write_flow, AuditEvent, AutonomyConsistency, DriveKind, Evidence,
    EvidenceKind, InternalDrive, IntrinsicIntensity, MotivationError, ReflectionAuditor,
    SGIContent, SGIEntry, SGIStructured, ValueStability, SGI, SGI_MAX_TEXT_CHARS,
};
use std::collections::HashMap;

fn main() {
    println!("=== apeireth-motivation demo (A11.2) ===\n");

    // ---- 1. 准备 SGI + ReflectionAuditor ----
    let mut sgi = SGI::new();
    let mut auditor = ReflectionAuditor::new();

    // ---- 2. 第一次写入: 内驱目标 (C-SGI-1 唯一性 — 空, 必然通过) ----
    println!("[场景 1] 首次写入 — 内驱目标 goal_A");
    let entry_a = build_entry("goal_A", "self_goal_emergent", 0.7);
    let evidences = good_evidences();
    match write_flow(&mut sgi, entry_a, &evidences, &mut auditor, false) {
        Ok(r) => println!(
            "  ✅ OK: entry_id={} evidence_score={:.3} history_len={}",
            r.entry_id,
            r.evidence_score,
            sgi.history_len()
        ),
        Err(e) => println!("  ❌ ERR: {e}"),
    }

    // ---- 3. 第二次写入: 不同内容 (C-SGI-1 通过) ----
    println!("\n[场景 2] 二次写入 — 内驱目标 goal_B (新内容)");
    let entry_b = build_entry("goal_B", "self_goal_reflect", 0.8);
    match write_flow(&mut sgi, entry_b, &evidences, &mut auditor, false) {
        Ok(r) => println!(
            "  ✅ OK: entry_id={} evidence_score={:.3} history_len={}",
            r.entry_id,
            r.evidence_score,
            sgi.history_len()
        ),
        Err(e) => println!("  ❌ ERR: {e}"),
    }

    // ---- 4. 第三次写入: 重复内容 (C-SGI-1 拒绝, 不允许 duplicate) ----
    println!("\n[场景 3] 三次写入 — 重复 goal_B (C-SGI-1 拒绝)");
    let entry_c = build_entry("goal_B", "drive_again", 0.9);
    match write_flow(&mut sgi, entry_c, &evidences, &mut auditor, false) {
        Ok(_) => println!("  ❌ UNEXPECTED OK"),
        Err(MotivationError::NotUnique(id)) => println!("  ✅ C-SGI-1 拒绝 (重复 id={id:?})"),
        Err(e) => println!("  ❌ UNEXPECTED ERR: {e}"),
    }

    // ---- 5. 显式声明 duplicate (allow_duplicate=true) ----
    println!("\n[场景 4] 四次写入 — 重复 goal_B 但显式声明 duplicate");
    let entry_d = build_entry("goal_B", "drive_reaffirm", 0.85);
    match write_flow(&mut sgi, entry_d, &evidences, &mut auditor, true) {
        Ok(r) => println!(
            "  ✅ OK: entry_id={} evidence_score={:.3} history_len={}",
            r.entry_id,
            r.evidence_score,
            sgi.history_len()
        ),
        Err(e) => println!("  ❌ ERR: {e}"),
    }

    // ---- 6. C-SGI-6 最长 N 字符 — 4096 + 1 ----
    println!(
        "\n[场景 5] FreeText 超长 ({}+1) — C-SGI-6 拒绝",
        SGI_MAX_TEXT_CHARS
    );
    let prefix = "# goal: ok\n# deadline: 2026-12-31\n# success: ok\n";
    let pad_len = SGI_MAX_TEXT_CHARS.saturating_sub(prefix.len()) + 1;
    let huge = format!("{prefix}{}", "x".repeat(pad_len));
    let drive = InternalDrive::new("freetext_test", 0.5);
    let entry_huge = SGIEntry::new(SGIContent::FreeText(huge), &drive);
    match write_flow(&mut sgi, entry_huge, &evidences, &mut auditor, false) {
        Ok(_) => println!("  ❌ UNEXPECTED OK"),
        Err(MotivationError::TextTooLong { actual, max }) => {
            println!("  ✅ C-SGI-6 拒绝 (actual={actual} max={max})");
        }
        Err(e) => println!("  ❌ UNEXPECTED ERR: {e}"),
    }

    // ---- 7. C-SGI-3 E 层证据不足 ----
    println!("\n[场景 6] E 层证据不足 (仅 2 类) — C-SGI-3 拒绝");
    let entry_e = build_entry("goal_C", "drive_C", 0.5);
    let weak_evidences = vec![
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
    ];
    match write_flow(&mut sgi, entry_e, &weak_evidences, &mut auditor, false) {
        Ok(_) => println!("  ❌ UNEXPECTED OK"),
        Err(MotivationError::EvidenceInsufficient(reason)) => {
            println!("  ✅ C-SGI-3 拒绝 ({reason})");
        }
        Err(e) => println!("  ❌ UNEXPECTED ERR: {e}"),
    }

    // ---- 8. §13 V0.5 v2 动机/价值测度 ----
    println!("\n[场景 7] §13 V0.5 v2 动机/价值测度");
    let score = motivation_score(
        AutonomyConsistency {
            internal_intensity: 0.9,
            internal_history_ratio: 0.85,
        },
        ValueStability {
            goal_turnover: 0.1,
            deadline_variance: 0.15,
        },
        IntrinsicIntensity {
            current_internal: 0.9,
            historical_peak: 0.95,
        },
    );
    println!(
        "  total = {:.3} (passes_threshold = {})\n  autonomy = {:.3}\n  value    = {:.3}\n  intrinsic= {:.3}",
        score.total, score.passes_threshold, score.autonomy, score.value, score.intrinsic
    );

    // ---- 9. 审计器统计 ----
    println!("\n[场景 8] ReflectionAuditor 统计");
    let silent = auditor.silent_alert_count();
    let history_fail = auditor.history_failed_count();
    let appended = auditor
        .events()
        .iter()
        .filter(|e| matches!(e, AuditEvent::HistoryAppended { .. }))
        .count();
    println!("  silent_alerts      = {silent}");
    println!("  history_failed     = {history_fail}");
    println!("  history_appended   = {appended}");
    println!(
        "  sgi_current.drive  = {:?}",
        sgi.current().map(|e| e.drive_kind)
    );
    println!(
        "  sgi_history        = {} entries (强不可变, 不可回滚)",
        sgi.history_len()
    );

    println!("\n=== demo end ===");
}

/// helper: 构造有效 SGIEntry (Structured + 3 类证据).
fn build_entry(goal: &str, drive_label: &str, intensity: f64) -> SGIEntry {
    let drive = InternalDrive::new(drive_label, intensity);
    let content = SGIContent::Structured(SGIStructured {
        goal: goal.to_string(),
        deadline: "2026-12-31".to_string(),
        success_criteria: "test passes".to_string(),
        extras: HashMap::new(),
        multimodal: None,
    });
    let mut e = SGIEntry::new(content, &drive);
    e.evidence_refs = good_evidences();
    e
}

/// helper: 构造有效 E 层证据 (3 类, 0.9 权重, 通过 MIN_EVIDENCE_SCORE).
fn good_evidences() -> Vec<Evidence> {
    vec![
        Evidence {
            kind: EvidenceKind::Council,
            source: "demo_council".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::History,
            source: "demo_history".into(),
            weight: 0.9,
        },
        Evidence {
            kind: EvidenceKind::Principle,
            source: "demo_principle".into(),
            weight: 0.9,
        },
    ]
}

/// 静默消除 unused import 警告 (DriveKind 仅用作 enum 引用, 不直接比较).
#[allow(dead_code)]
fn _assert_drive_kind_used() {
    let _: DriveKind = DriveKind::Internal;
}
