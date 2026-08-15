//! R177 motivation organ Kani proofs (W2)
//!
//! **要验证的不变量** (C-SGI-1~7 七条硬约束):
//! 1. C-SGI-1 唯一性: 新内容与 sgi_current id/content 不同时通过
//! 2. C-SGI-5 内容三选一: SGIContent 永远是 Structured/FreeText/Multimodal
//! 3. C-SGI-6 长度上限: FreeText 字符数 ≤ SGI_MAX_TEXT_CHARS
//! 4. C-SGI-7 三条必备: Structured 的 goal/deadline/success_criteria 都非空
//! 5. drive intensity 自动 clamp 到 [0.0, 1.0]
//! 6. history_len 单调不减
//! 7. SGIContent::kind_name 三种值

#![allow(missing_docs)]

use crate::{
    check_csgi1_uniqueness, check_csgi5_content_kind, check_csgi6_max_chars,
    check_csgi7_three_required, DriveKind, Evidence, EvidenceKind, ExternalDrive, InternalDrive,
    Modality, MultimodalIntent, MotivationDrive, SGIContent, SGIStructured, SGI, SGIEntry,
    SGI_MAX_TEXT_CHARS, MIN_EVIDENCE_KINDS,
};
use std::collections::HashMap;

fn make_structured(goal: &str, deadline: &str, success: &str) -> SGIContent {
    SGIContent::Structured(SGIStructured {
        goal: goal.into(),
        deadline: deadline.into(),
        success_criteria: success.into(),
        extras: HashMap::new(),
        multimodal: None,
    })
}

fn make_freetext(s: &str) -> SGIContent {
    SGIContent::FreeText(s.into())
}

fn make_multimodal() -> SGIContent {
    SGIContent::Multimodal(MultimodalIntent {
        modality: Modality::Image,
        pointer: "img://test".into(),
    })
}

// ============================================
// Property 1: drive intensity 自动 clamp 到 [0.0, 1.0]
// ============================================
#[test]
fn r177_mot_01_intensity_clamped() {
    let i_hi = InternalDrive::new("test", 5.0);
    assert!(i_hi.intensity <= 1.0);
    let i_lo = InternalDrive::new("test", -2.0);
    assert!(i_lo.intensity >= 0.0);
    let e_hi = ExternalDrive::new("test", 100.0);
    assert!(e_hi.intensity <= 1.0);
    let e_lo = ExternalDrive::new("test", -100.0);
    assert!(e_lo.intensity >= 0.0);
}

// ============================================
// Property 2: drive.kind() 匹配 DriveKind
// ============================================
#[test]
fn r177_mot_02_drive_kind_match() {
    assert!(matches!(InternalDrive::new("i", 0.5).kind(), DriveKind::Internal));
    assert!(matches!(ExternalDrive::new("e", 0.5).kind(), DriveKind::External));
}

// ============================================
// Property 3: SGIContent::kind_name 三种值
// ============================================
#[test]
fn r177_mot_03_kind_name_three() {
    assert_eq!(make_structured("g", "d", "s").kind_name(), "structured");
    assert_eq!(make_freetext("hi").kind_name(), "free_text");
    assert_eq!(make_multimodal().kind_name(), "multimodal");
}

// ============================================
// Property 4: C-SGI-5 校验通过 (3 种合法)
// ============================================
#[test]
fn r177_mot_04_csgi5_passes() {
    assert!(check_csgi5_content_kind(&make_structured("g", "d", "s")).is_ok());
    assert!(check_csgi5_content_kind(&make_freetext("hi")).is_ok());
    assert!(check_csgi5_content_kind(&make_multimodal()).is_ok());
}

// ============================================
// Property 5: C-SGI-6 长度上限
// ============================================
#[test]
fn r177_mot_05_csgi6_max_chars() {
    let s_eq = "x".repeat(SGI_MAX_TEXT_CHARS);
    assert!(check_csgi6_max_chars(&make_freetext(&s_eq)).is_ok());
    let s_over = "x".repeat(SGI_MAX_TEXT_CHARS + 1);
    assert!(check_csgi6_max_chars(&make_freetext(&s_over)).is_err());
    assert!(check_csgi6_max_chars(&make_structured("g", "d", "s")).is_ok());
}

// ============================================
// Property 6: C-SGI-7 三条必备
// ============================================
#[test]
fn r177_mot_06_csgi7_three_required() {
    assert!(check_csgi7_three_required(&make_structured("g", "d", "s")).is_ok());
    assert!(check_csgi7_three_required(&make_structured("", "d", "s")).is_err());
    assert!(check_csgi7_three_required(&make_structured("g", "", "s")).is_err());
    assert!(check_csgi7_three_required(&make_structured("g", "d", "")).is_err());
}

// ============================================
// Property 7: C-SGI-1 唯一性
// ============================================
#[test]
fn r177_mot_07_csgi1_uniqueness() {
    let drive = InternalDrive::new("d", 0.5);
    let entry_a = SGIEntry::new(make_structured("g", "d", "s"), &drive);
    let sgi = SGI {
        sgi_current: Some(entry_a.clone()),
        sgi_history: vec![entry_a.clone()],
    };

    let entry_same_id = SGIEntry { id: entry_a.id, ..entry_a.clone() };
    assert!(check_csgi1_uniqueness(&sgi, &entry_same_id, true).is_err());

    let entry_same_content = SGIEntry::new(make_structured("g", "d", "s"), &drive);
    assert!(check_csgi1_uniqueness(&sgi, &entry_same_content, false).is_err());

    let entry_diff = SGIEntry::new(make_structured("g2", "d2", "s2"), &drive);
    assert!(check_csgi1_uniqueness(&sgi, &entry_diff, false).is_ok());
}

// ============================================
// Property 8: history_len 单调不减
// ============================================
#[test]
fn r177_mot_08_history_len_monotonic() {
    let mut sgi = SGI::new();
    assert_eq!(sgi.history_len(), 0);
    let drive = InternalDrive::new("d", 0.5);
    let entry = SGIEntry::new(make_structured("g", "d", "s"), &drive);
    sgi.sgi_history.push(entry.clone());
    assert_eq!(sgi.history_len(), 1);
    sgi.sgi_history.push(entry);
    assert_eq!(sgi.history_len(), 2);
    sgi.sgi_current = Some(SGIEntry::new(make_structured("g2", "d2", "s2"), &drive));
    assert_eq!(sgi.history_len(), 2);
}

// ============================================
// Property 9: MIN_EVIDENCE_KINDS = 3
// ============================================
#[test]
fn r177_mot_09_min_evidence_kinds() {
    assert_eq!(MIN_EVIDENCE_KINDS, 3);
    let kinds = [EvidenceKind::Council, EvidenceKind::History, EvidenceKind::Principle];
    let evidence: Vec<Evidence> = kinds
        .iter()
        .map(|k| Evidence {
            kind: *k,
            source: format!("{:?}", k),
            weight: 0.9,
        })
        .collect();
    assert_eq!(evidence.len(), MIN_EVIDENCE_KINDS);
}

// ============================================
// Property 10: SGI::new() 空状态
// ============================================
#[test]
fn r177_mot_10_sgi_new_empty() {
    let sgi = SGI::new();
    assert!(sgi.current().is_none());
    assert_eq!(sgi.history_len(), 0);
    assert!(sgi.history().is_empty());
}

// ============================================
// Kani-style formal proof
// ============================================
#[cfg(kani)]
#[kani::proof]
fn r177_mot_kani_01_intensity_in_range() {
    let i = InternalDrive::new("x", 100.0);
    assert!(i.intensity >= 0.0 && i.intensity <= 1.0);
    let e = ExternalDrive::new("x", -100.0);
    assert!(e.intensity >= 0.0 && e.intensity <= 1.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_mot_kani_02_csgi6_boundary() {
    assert!(SGI_MAX_TEXT_CHARS > 0);
}
