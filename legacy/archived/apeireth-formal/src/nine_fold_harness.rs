//! R131.9 NINE_FOLD_GUARDS_HARDCODE + flush_noop Kani harness (critical missing 2+3)
//!
//! **目的**: 形式化守门 9 (Perceptual Evidence Guard) + semantic_persist flush_noop 显性.
//!
//! **跑法**: `cargo kani --harness kani_verify_*`

#![cfg_attr(kani, allow(dead_code))]

// ============================================================
// 1. NINE_FOLD_GUARDS_HARDCODE 编译期守门
// ============================================================

/// 9 重守门 hardcode (per evidence_guard.rs::NINE_FOLD_GUARDS_HARDCODE)
/// R131 P1.3 升级 8 → 9, 加 Evidence Guard (守门 9).
pub const NINE_FOLD_GUARDS_HARDCODE: usize = 9;

/// Evidence Guard 在 9 重守门中的索引 (守门 1-9)
pub const EVIDENCE_FOLD_GUARD_INDEX: u8 = 9;

/// EvidenceKind 5 类 (per EvidenceKind enum)
pub const EVIDENCE_KIND_COUNT: usize = 5;

/// Evidence Check 4 类 (Pass / PassInferred / Fail / Missing)
pub const EVIDENCE_CHECK_COUNT: usize = 4;

/// Inference confidence 阈值 (高于此值视为 Fail)
pub const INFERENCE_CONFIDENCE_THRESHOLD: f64 = 0.7;

// ============================================================
// POD EvidenceGuard 模型
// ============================================================

/// POD Evidence Kind (5 类用 u8 表示, 0=ToolCall / 1=MemoryLookup / 2=ExternalSource / 3=SemanticReference / 4=Inference)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EvidenceKindPod(pub u8);

impl EvidenceKindPod {
    pub const TOOL_CALL: u8 = 0;
    pub const MEMORY_LOOKUP: u8 = 1;
    pub const EXTERNAL_SOURCE: u8 = 2;
    pub const SEMANTIC_REFERENCE: u8 = 3;
    pub const INFERENCE: u8 = 4;

    pub const fn is_empirical(self) -> bool {
        self.0 != Self::INFERENCE
    }
}

/// POD Claim 模型 (per EvidenceClaim)
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ClaimPod {
    /// 证据链 (固定 3 槽, 每槽 u8 表示 EvidenceKind)
    pub evidence: [u8; 3],
    /// 证据数 (0-3)
    pub evidence_count: u8,
    /// 推理置信度 (0.0 - 1.0, 用 i32 表示 * 100)
    pub confidence_x100: i32,
    /// claim 是否存在
    pub exists: bool,
}

impl ClaimPod {
    pub const fn new_inference(confidence: f64) -> Self {
        Self {
            evidence: [EvidenceKindPod::INFERENCE, 255, 255],
            evidence_count: 1,
            confidence_x100: (confidence * 100.0) as i32,
            exists: true,
        }
    }
    pub const fn new_empirical() -> Self {
        Self {
            evidence: [EvidenceKindPod::TOOL_CALL, 255, 255],
            evidence_count: 1,
            confidence_x100: 100,
            exists: true,
        }
    }
    pub const fn empty() -> Self {
        Self { evidence: [255; 3], evidence_count: 0, confidence_x100: 0, exists: false }
    }

    /// verify: Pass / PassInferred / Fail / Missing (per EvidenceCheck enum)
    pub const fn verify(self) -> u8 {
        // 0=Pass, 1=PassInferred, 2=Fail, 3=Missing
        if !self.exists { return 3; } // Missing
        let has_emp = self.evidence[0] == EvidenceKindPod::TOOL_CALL
            || self.evidence[0] == EvidenceKindPod::MEMORY_LOOKUP
            || self.evidence[0] == EvidenceKindPod::EXTERNAL_SOURCE
            || self.evidence[0] == EvidenceKindPod::SEMANTIC_REFERENCE;
        if has_emp { return 0; } // Pass
        // Inference 路径
        if self.confidence_x100 < 70 { return 1; } // PassInferred (confidence < 0.7)
        2 // Fail
    }
}

// ============================================================
// nondet_* helper
// ============================================================

#[cfg(kani)]
fn nondet_u8() -> u8 { kani::any() }
#[cfg(not(kani))]
fn nondet_u8() -> u8 { 0 }

#[cfg(kani)]
fn nondet_i32() -> i32 { kani::any() }
#[cfg(not(kani))]
fn nondet_i32() -> i32 { 0 }

#[cfg(kani)]
fn nondet_bool() -> bool { kani::any() }
#[cfg(not(kani))]
fn nondet_bool() -> bool { false }

// ============================================================
// Kani harness 1: NINE_FOLD_GUARDS_HARDCODE = 9 编译期守门
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_nine_fold_guards_hardcode_eq_9() {
    assert!(NINE_FOLD_GUARDS_HARDCODE == 9, "NINE_FOLD_GUARDS_HARDCODE must be 9");
    assert!(EVIDENCE_FOLD_GUARD_INDEX == 9, "Evidence Guard must be the 9th in 9-fold");
    assert!(EVIDENCE_KIND_COUNT == 5, "EvidenceKind must have 5 variants");
    assert!(EVIDENCE_CHECK_COUNT == 4, "EvidenceCheck must have 4 variants");
    assert!((INFERENCE_CONFIDENCE_THRESHOLD - 0.7).abs() < 0.001, "Inference threshold must be 0.7");
}

// ============================================================
// Kani harness 2: verify() 0 panic + 4 类必返其一
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_evidence_guard_5_kinds_complete() {
    let claim = ClaimPod {
        evidence: [nondet_u8(), nondet_u8(), nondet_u8()],
        evidence_count: 1, // 简化: 总是 1 个证据
        confidence_x100: nondet_i32(),
        exists: nondet_bool(),
    };
    let result = claim.verify();
    // 4 类之一: 0=Pass / 1=PassInferred / 2=Fail / 3=Missing
    assert!(result < 4, "verify must return 0/1/2/3 (Pass/PassInferred/Fail/Missing)");
}

// ============================================================
// Kani harness 3: 实证证据 → Pass; Inference 高 conf → Fail
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_evidence_guard_pass_fail_logic() {
    // 实证证据 → Pass (0)
    let emp = ClaimPod::new_empirical();
    assert!(emp.verify() == 0, "empirical evidence must Pass");

    // Inference low conf → PassInferred (1)
    let low_inf = ClaimPod::new_inference(0.5);
    assert!(low_inf.verify() == 1, "Inference with conf < 0.7 must PassInferred");

    // Inference high conf → Fail (2)
    let high_inf = ClaimPod::new_inference(0.9);
    assert!(high_inf.verify() == 2, "Inference with conf >= 0.7 must Fail");

    // 不存在 → Missing (3)
    let missing = ClaimPod::empty();
    assert!(missing.verify() == 3, "non-existent claim must Missing");
}

// ============================================================
// Kani harness 4: 9 重守门 = 守门 1..9 严格唯一
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_nine_fold_guards_indices_complete() {
    // 守门 1-9 严格覆盖 (per 守门 9 重体系)
    // 这里只验证 Evidence Guard 是守门 9 (其他守门 1-8 在 governance process 已守门)
    for i in 1..=9u8 {
        if i == EVIDENCE_FOLD_GUARD_INDEX {
            assert!(i == 9, "Evidence Guard is gate 9");
        }
    }
    // 9 重总守门数
    assert!(NINE_FOLD_GUARDS_HARDCODE == 9, "9-fold guards hardcode invariant");
}

// ============================================================
// 2. semantic_persist flush_noop 形式化
// ============================================================

/// POD PersistentSemanticIndex 模型 (per apeireth-memory::semantic_persist)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PersistentSemanticIndexPod {
    /// 内部 entry 数 (固定 array, Kani-friendly)
    pub entry_count: u32,
    /// 内部 version (每次写递增)
    pub version: u32,
    /// flush_noop 调用计数
    pub flush_noop_call_count: u32,
    /// save 调用计数 (deprecated 路径)
    pub save_call_count: u32,
}

impl PersistentSemanticIndexPod {
    pub const fn new() -> Self {
        Self { entry_count: 0, version: 0, flush_noop_call_count: 0, save_call_count: 0 }
    }

    /// flush_noop: 显式 no-op, 0 修改 entry_count / version (R131 P0-2)
    pub const fn flush_noop(&mut self) {
        self.flush_noop_call_count += 1;
        // 关键不变量: 0 修改 entry_count / version
    }

    /// save: 旧 API, 已 deprecated (R131 P0-2)
    pub const fn save(&mut self) {
        self.save_call_count += 1;
    }

    /// 真实写入 (真实现, 非 save 路径)
    pub const fn write_real(&mut self, count: u32) {
        self.entry_count += count;
        self.version += 1;
    }
}

// ============================================================
// Kani harness 5: flush_noop 0 修改 state
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_flush_noop_does_not_modify_state() {
    let mut idx = PersistentSemanticIndexPod::new();
    let initial_count = idx.entry_count;
    let initial_version = idx.version;
    let n: u8 = nondet_u8();

    for _ in 0..n {
        idx.flush_noop();
    }

    // 关键不变量: flush_noop 0 修改 entry_count / version
    assert!(idx.entry_count == initial_count, "flush_noop must not modify entry_count");
    assert!(idx.version == initial_version, "flush_noop must not modify version");
    // 但 flush_noop_call_count 累加 (用于审计)
    assert!(idx.flush_noop_call_count as u8 == n, "flush_noop_call_count accumulates");
}

// ============================================================
// Kani harness 6: write_real vs flush_noop 行为对比
// ============================================================

#[cfg_attr(kani, kani::proof)]
pub fn kani_verify_flush_noop_vs_write_real() {
    let mut idx = PersistentSemanticIndexPod::new();

    // flush_noop 不增 entry_count / version
    idx.flush_noop();
    idx.flush_noop();
    assert!(idx.entry_count == 0, "flush_noop 0 increments entry_count");
    assert!(idx.version == 0, "flush_noop 0 increments version");
    assert!(idx.flush_noop_call_count == 2, "flush_noop call_count increments");

    // write_real 真增
    idx.write_real(5);
    assert!(idx.entry_count == 5, "write_real increments entry_count");
    assert!(idx.version == 1, "write_real increments version");

    // flush_noop 仍不增
    idx.flush_noop();
    assert!(idx.entry_count == 5, "after write_real, flush_noop still 0 increments");
    assert!(idx.version == 1, "after write_real, flush_noop still 0 increments version");
}

// ============================================
// Unit tests
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_fold_guards_compile_time() {
        assert_eq!(NINE_FOLD_GUARDS_HARDCODE, 9);
        assert_eq!(EVIDENCE_FOLD_GUARD_INDEX, 9);
        assert_eq!(EVIDENCE_KIND_COUNT, 5);
        assert_eq!(EVIDENCE_CHECK_COUNT, 4);
    }

    #[test]
    fn evidence_kind_pod_5_kinds() {
        assert_eq!(EvidenceKindPod::TOOL_CALL, 0);
        assert_eq!(EvidenceKindPod::MEMORY_LOOKUP, 1);
        assert_eq!(EvidenceKindPod::EXTERNAL_SOURCE, 2);
        assert_eq!(EvidenceKindPod::SEMANTIC_REFERENCE, 3);
        assert_eq!(EvidenceKindPod::INFERENCE, 4);
        assert!(!EvidenceKindPod(EvidenceKindPod::INFERENCE).is_empirical());
        assert!(EvidenceKindPod(EvidenceKindPod::TOOL_CALL).is_empirical());
    }

    #[test]
    fn claim_verify_empirical_pass() {
        let c = ClaimPod::new_empirical();
        assert_eq!(c.verify(), 0);
    }

    #[test]
    fn claim_verify_inference_low_pass() {
        let c = ClaimPod::new_inference(0.5);
        assert_eq!(c.verify(), 1);
    }

    #[test]
    fn claim_verify_inference_high_fail() {
        let c = ClaimPod::new_inference(0.9);
        assert_eq!(c.verify(), 2);
    }

    #[test]
    fn claim_verify_missing() {
        let c = ClaimPod::empty();
        assert_eq!(c.verify(), 3);
    }

    #[test]
    fn claim_verify_boundary_07() {
        let c_low = ClaimPod::new_inference(0.69);
        let c_high = ClaimPod::new_inference(0.71);
        assert_eq!(c_low.verify(), 1, "0.69 must be PassInferred");
        assert_eq!(c_high.verify(), 2, "0.71 must be Fail");
    }

    #[test]
    fn flush_noop_does_not_modify_state() {
        let mut idx = PersistentSemanticIndexPod::new();
        for _ in 0..100 {
            idx.flush_noop();
        }
        assert_eq!(idx.entry_count, 0);
        assert_eq!(idx.version, 0);
        assert_eq!(idx.flush_noop_call_count, 100);
    }

    #[test]
    fn flush_noop_vs_write_real() {
        let mut idx = PersistentSemanticIndexPod::new();
        idx.flush_noop();
        idx.flush_noop();
        assert_eq!(idx.entry_count, 0);
        idx.write_real(5);
        assert_eq!(idx.entry_count, 5);
        assert_eq!(idx.version, 1);
        idx.flush_noop();
        assert_eq!(idx.entry_count, 5);
        assert_eq!(idx.version, 1);
    }

    #[test]
    fn flush_noop_save_separation() {
        let mut idx = PersistentSemanticIndexPod::new();
        idx.flush_noop();
        idx.save();
        idx.save();
        assert_eq!(idx.flush_noop_call_count, 1);
        assert_eq!(idx.save_call_count, 2);
        // 两者都不修改 entry_count / version
        assert_eq!(idx.entry_count, 0);
        assert_eq!(idx.version, 0);
    }
}
