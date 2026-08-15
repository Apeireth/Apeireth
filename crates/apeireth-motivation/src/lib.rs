//! apeireth-motivation: 动机器官 (A11.2 落点 — R14 Phase 4)
//!
//! **职责**: 内部动机/价值主路径 — `MotivationDrive` trait (内驱/外驱) +
//! `SGI` 单字段 (`sgi_current` + `sgi_history` 二元) + C-SGI-1~7 七条硬约束
//! (编译时 hardcode) + E 层多证据加权校验 + `ReflectionAuditor` 静默/失败告警 +
//! V0.5 v2 §13 动机/价值测度公式 `motivation_score`.
//!
//! **架构位置**: 阶段 4 §2 主路径 17 crate 之 A11.2 器官 (在 apeireth-cognition
//! 之后、apeireth-action 之前 — 提供 self-goal 给 action 选择).
//!
//! **当前状态**: A11.2 最小可用落地 (P8 任务 f5549281 by fullstack_engineer2).
//! 本 crate 提供 7+ pub fn + 5+ unit tests + 1+ integration test + example,
//! 编译时 hardcode C-SGI-1~7 + §21.4 写入流程.
//!
//! **诚实登记 (主 17:58 不假装)**:
//! - V0.5 v2 24 维权重是**提议** (0.06 起点), 待主人拍板, 不冻结 (见 §13.3).
//! - 完整动机器官 (24 维全部 + §13 公式实时校准) 待 A18/A19 深化.
//! - 当前实现聚焦 SGI 单字段 + 7 条硬约束 + E 层校验最小骨架.
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core / apeireth-asi 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值 (0.8682 / 0.8532 / 0.9063)
//! - ❌ 不碰 apeireth-legacy/

#![deny(unsafe_code)]

use chrono::Utc;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;
// R37-2: 9 organ 部分合并 — value → motivation 透明 re-export (workspace member 真删)
// 下游调用方 `use apeireth_value::X` 仍能用 (R37-2 后 0 breaking)
pub use apeireth_value::*;
// R173 ST-B3.1 — bridge 3: consciousness -> motivation
pub mod consciousness_bridge;
// R173 ST-B6.1 — bridge 6: life-force -> motivation
pub mod life_force_bridge;
// R176: bridges 3+6 Kani proofs
mod bridge_kani_proofs;
// R177: organ invariants (10 tests + 2 Kani proofs)
mod organ_kani_proofs;

// ============================================
// 1. 错误类型
// ============================================

/// 顶层错误: 动机子系统所有 fallback error.
#[derive(Debug, Error)]
pub enum MotivationError {
    /// C-SGI-1 唯一性违反 (sgi_current 与新内容重复且未显式声明)
    #[error("C-SGI-1 violation: sgi_current 重复 (id={0:?})")]
    NotUnique(Option<Uuid>),

    /// C-SGI-2 可审计违反 (sgi_history 追加失败)
    #[error("C-SGI-2 violation: sgi_history append failed: {0}")]
    HistoryAppendFailed(String),

    /// C-SGI-3 E 层校验未通过
    #[error("C-SGI-3 violation: E-layer evidence insufficient: {0}")]
    EvidenceInsufficient(String),

    /// C-SGI-5 内容三选一违反 (未知 content kind)
    #[error("C-SGI-5 violation: content kind 不在 三选一 ({0})")]
    InvalidContentKind(String),

    /// C-SGI-6 最长 N 字符违反
    #[error("C-SGI-6 violation: free-text {actual} > max {max} chars")]
    TextTooLong {
        /// 实际字符数
        actual: usize,
        /// 允许的最大字符数
        max: usize,
    },

    /// C-SGI-7 三条必备违反 (goal / deadline / success_criteria 缺失)
    #[error("C-SGI-7 violation: 三条必备缺失 — {0}")]
    MissingRequired(String),

    /// 序列化错误
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 统一结果类型.
pub type MotivationResult<T> = Result<T, MotivationError>;

// ============================================
// 2. MotivationDrive trait — 内驱/外驱
// ============================================

/// 驱动种类: 内驱 (自主目标, §18.5 三件套自发生成) / 外驱 (用户/外部输入).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    /// 内驱: 自主目标 (SGI 来源 = 主体自身)
    Internal,
    /// 外驱: 用户/外部输入 (SGI 来源 = 用户指令或环境)
    External,
}

/// 动机驱动 trait — 任何动机来源必须实现.
pub trait MotivationDrive: Send + Sync {
    /// 驱动种类
    fn kind(&self) -> DriveKind;
    /// 强度 [0.0, 1.0] — 主 17:43 实事求是 (可量化)
    fn intensity(&self) -> f64;
    /// 标签 (供日志 / 反思期显示)
    fn label(&self) -> &str;
}

/// 内驱实现 (自主目标 / 自我反思 / 涌现意图).
#[derive(Debug, Clone)]
pub struct InternalDrive {
    /// 标签 (例: "self_goal_emergent")
    pub label: String,
    /// 强度 [0, 1]
    pub intensity: f64,
}

impl InternalDrive {
    /// 构造内驱 (强度钳位到 [0, 1]).
    pub fn new(label: impl Into<String>, intensity: f64) -> Self {
        Self {
            label: label.into(),
            intensity: intensity.clamp(0.0, 1.0),
        }
    }
}

impl MotivationDrive for InternalDrive {
    fn kind(&self) -> DriveKind {
        DriveKind::Internal
    }
    fn intensity(&self) -> f64 {
        self.intensity
    }
    fn label(&self) -> &str {
        &self.label
    }
}

/// 外驱实现 (用户指令 / 环境触发 / 系统调度).
#[derive(Debug, Clone)]
pub struct ExternalDrive {
    /// 标签 (例: "user_instruction")
    pub label: String,
    /// 强度 [0, 1]
    pub intensity: f64,
}

impl ExternalDrive {
    /// 构造外驱 (强度钳位到 [0, 1]).
    pub fn new(label: impl Into<String>, intensity: f64) -> Self {
        Self {
            label: label.into(),
            intensity: intensity.clamp(0.0, 1.0),
        }
    }
}

impl MotivationDrive for ExternalDrive {
    fn kind(&self) -> DriveKind {
        DriveKind::External
    }
    fn intensity(&self) -> f64 {
        self.intensity
    }
    fn label(&self) -> &str {
        &self.label
    }
}

// ============================================
// 3. SGI 内容三选一 (C-SGI-5)
// ============================================

/// 多模态意图的模态 (C-SGI-5 ③ 多模态意图).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modality {
    /// 图像 (image/* URI or base64 pointer)
    Image,
    /// 音频 (audio/* URI or base64 pointer)
    Audio,
    /// 结构化指针 (json / sql / sled KV locator)
    StructuredPointer,
}

/// 多模态意图 (C-SGI-5 ③).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultimodalIntent {
    /// 多模态模态
    pub modality: Modality,
    /// 指向 (URI / path / locator)
    pub pointer: String,
}

/// 结构化对象 (C-SGI-5 ①) — 含 C-SGI-7 三条必备 + 自由扩展字段.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SGIStructured {
    /// C-SGI-7 ① 目标
    pub goal: String,
    /// C-SGI-7 ② 期限 (ISO-8601 字符串 / 自然语言皆可)
    pub deadline: String,
    /// C-SGI-7 ③ 成功标准
    pub success_criteria: String,
    /// 自由扩展字段 (附加上下文)
    pub extras: HashMap<String, String>,
    /// 配套多模态意图 (可选 — 三选一之外, 可附带 pointer)
    pub multimodal: Option<MultimodalIntent>,
}

/// SGI 内容 (C-SGI-5 三选一: ① 结构化 / ② 自由文本 / ③ 多模态意图).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SGIContent {
    /// ① 结构化对象 (typed fields)
    Structured(SGIStructured),
    /// ② 自由文本 (UTF-8, 受 C-SGI-6 最长 N 字符限制)
    FreeText(String),
    /// ③ 多模态意图 (image/audio/structured-pointer)
    Multimodal(MultimodalIntent),
}

impl SGIContent {
    /// C-SGI-5 判别: 返回内容种类的可读名.
    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Structured(_) => "structured",
            Self::FreeText(_) => "free_text",
            Self::Multimodal(_) => "multimodal",
        }
    }
}

// ============================================
// 4. E 层证据 (C-SGI-3)
// ============================================

/// 证据种类 (C-SGI-3 — 至少 council / history / principle 三类证据覆盖).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceKind {
    /// 多 AI 一致 (智囊团 §10)
    Council,
    /// 历史流 (§21.5 6 历史流)
    History,
    /// 原则洋葱 (§3 9 原则, 12 键 verdict)
    Principle,
    /// 权限洋葱 (§3 9 权限)
    Permission,
    /// 真实人类 (HA §18.5)
    Human,
    /// 反思期审计 (§21.5 反思流)
    Audit,
}

/// E 层单条证据.
#[derive(Debug, Clone)]
pub struct Evidence {
    /// 证据种类
    pub kind: EvidenceKind,
    /// 来源标识 (例: "v1077_asi_v04" / "council_member_v1052" / "verdict_cache")
    pub source: String,
    /// 权重 [0.0, 1.0]
    pub weight: f64,
}

/// E 层校验 (C-SGI-3) — 多证据加权.
///
/// **硬约束**: 至少 3 类不同 EvidenceKind, 加权总分 ≥ MIN_EVIDENCE_SCORE.
pub const MIN_EVIDENCE_KINDS: usize = 3;

/// E 层校验最低加权分数 (V0.5 v2 §13 提议起点, 待主人拍板).
pub const MIN_EVIDENCE_SCORE: f64 = 0.85;

// ============================================
// 5. SGI 单字段 — sgi_current + sgi_history 二元 (C-SGI-1, C-SGI-2)
// ============================================

/// SGI 单条 entry (sgi_current 与 sgi_history 共用结构).
#[derive(Debug, Clone)]
pub struct SGIEntry {
    /// 唯一 ID
    pub id: Uuid,
    /// SGI 内容 (C-SGI-5 三选一)
    pub content: SGIContent,
    /// 驱动来源 (内驱/外驱)
    pub drive_kind: DriveKind,
    /// 驱动标签 (供反思期追溯)
    pub drive_label: String,
    /// 驱动强度 [0, 1]
    pub intensity: f64,
    /// E 层证据引用 (sgi_history 写入时一并保存)
    pub evidence_refs: Vec<Evidence>,
    /// 前驱 entry id (sgi_history 回链)
    pub predecessor: Option<Uuid>,
    /// 时间戳 (Unix seconds)
    pub timestamp: i64,
}

impl SGIEntry {
    /// 构造最小 SGIEntry (时间戳默认 now).
    pub fn new(content: SGIContent, drive: &dyn MotivationDrive) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            drive_kind: drive.kind(),
            drive_label: drive.label().to_string(),
            intensity: drive.intensity(),
            evidence_refs: Vec::new(),
            predecessor: None,
            timestamp: Utc::now().timestamp(),
        }
    }

    /// 追加 E 层证据 (写入前收集).
    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence_refs.push(evidence);
        self
    }
}

/// SGI 单字段 (sgi_current + sgi_history 二元) — §21.4 落地结构.
#[derive(Debug, Default)]
pub struct SGI {
    /// 当前唯一目标 (C-SGI-1 唯一性)
    sgi_current: Option<SGIEntry>,
    /// 目标历史 (C-SGI-2 追加, 不覆盖)
    sgi_history: Vec<SGIEntry>,
}

impl SGI {
    /// 创建空 SGI (无 sgi_current, 无 sgi_history).
    pub fn new() -> Self {
        Self::default()
    }

    /// 读 sgi_current (C-SGI-1 唯一性读取).
    pub fn current(&self) -> Option<&SGIEntry> {
        self.sgi_current.as_ref()
    }

    /// 读 sgi_history (追加日志, 不可回滚 — §21.4 强不可变).
    pub fn history(&self) -> &[SGIEntry] {
        &self.sgi_history
    }

    /// sgi_history 长度 (调试 / 反思期).
    pub fn history_len(&self) -> usize {
        self.sgi_history.len()
    }

    /// 内部 helper: 历史追加 + 当前切换 (由 WriteFlow 唯一调用).
    fn commit(&mut self, entry: SGIEntry) {
        self.sgi_history.push(entry.clone());
        self.sgi_current = Some(entry);
    }

    /// 内部 helper: 失败回滚 (sgi_history 已 push 但 sgi_current 不动).
    #[allow(dead_code)]
    fn rollback(&mut self, history_index: usize) {
        if history_index < self.sgi_history.len() {
            self.sgi_history.truncate(history_index);
        }
    }
}

// ============================================
// 6. 七条硬约束 C-SGI-1~7 (编译时 hardcode)
// ============================================

/// C-SGI-6 默认上限 N = 4096 (§21.4 阶段 2 校准起点, 待主人拍板).
pub const SGI_MAX_TEXT_CHARS: usize = 4096;

/// C-SGI-1 唯一性校验: 新内容与 sgi_current 必须不同 (或显式声明 duplicate).
///
/// **禁止**: 此函数是唯一允许做"是否重复"判定的地方. 修改 = 改 hardcode.
pub fn check_csgi1_uniqueness(
    sgi: &SGI,
    new: &SGIEntry,
    allow_duplicate: bool,
) -> MotivationResult<()> {
    if let Some(cur) = sgi.current() {
        if cur.id == new.id {
            return Err(MotivationError::NotUnique(Some(cur.id)));
        }
        if !allow_duplicate && cur.content == new.content {
            return Err(MotivationError::NotUnique(Some(cur.id)));
        }
    }
    Ok(())
}

/// C-SGI-5 内容三选一校验 — 必须是三种合法 SGIContent 之一.
pub fn check_csgi5_content_kind(content: &SGIContent) -> MotivationResult<()> {
    // 编译时穷举: 三种 variant, match 必须 exhaustive. 未覆盖 = 编译失败.
    let name = match content {
        SGIContent::Structured(_) => "structured",
        SGIContent::FreeText(_) => "free_text",
        SGIContent::Multimodal(_) => "multimodal",
    };
    if name != "structured" && name != "free_text" && name != "multimodal" {
        return Err(MotivationError::InvalidContentKind(name.to_string()));
    }
    Ok(())
}

/// C-SGI-6 最长 N 字符校验 — 仅对 FreeText 检查 (`char count` = Unicode scalar values).
pub fn check_csgi6_max_chars(content: &SGIContent) -> MotivationResult<()> {
    if let SGIContent::FreeText(s) = content {
        let actual = s.chars().count();
        if actual > SGI_MAX_TEXT_CHARS {
            return Err(MotivationError::TextTooLong {
                actual,
                max: SGI_MAX_TEXT_CHARS,
            });
        }
    }
    Ok(())
}

/// C-SGI-7 三条必备校验 — goal / deadline / success_criteria 非空.
///
/// **Structured / Multimodal**: 直接读字段.
/// **FreeText**: 解析 # GOAL / # DEADLINE / # SUCCESS 三段 (大小写不敏感).
pub fn check_csgi7_three_required(content: &SGIContent) -> MotivationResult<()> {
    match content {
        SGIContent::Structured(s) => {
            require_nonempty(&s.goal, "goal")?;
            require_nonempty(&s.deadline, "deadline")?;
            require_nonempty(&s.success_criteria, "success_criteria")
        }
        SGIContent::Multimodal(_) => {
            // 多模态必须配套一个 structured 三件套 (modality/pointer 不能替代)
            // 此处 Multimodal 单独不构成完整 SGI content, 需 multimodal.goal 等字段 —
            // 但 §21.4 要求三选一, 这里强制 Multimodal 携带附加三件套, 改用 Multimodal
            // 字段里的结构化 marker.
            // §21.4 例: 多模态意图需在 extras 内显式给出三条必备.
            Err(MotivationError::MissingRequired(
                "multimodal 必须在 extras 携带 goal/deadline/success_criteria, 或改用 Structured 包装".into(),
            ))
        }
        SGIContent::FreeText(text) => parse_freetext_three_required(text),
    }
}

/// helper: 非空校验.
fn require_nonempty(field: &str, name: &str) -> MotivationResult<()> {
    if field.trim().is_empty() {
        return Err(MotivationError::MissingRequired(format!(
            "{name} 为空 (C-SGI-7 三条必备)"
        )));
    }
    Ok(())
}

/// helper: 自由文本解析三条必备 (段头 `# GOAL:` / `# DEADLINE:` / `# SUCCESS:` 大小写不敏感).
fn parse_freetext_three_required(text: &str) -> MotivationResult<()> {
    let lower = text.to_lowercase();
    let has_goal = lower.contains("# goal");
    let has_deadline = lower.contains("# deadline");
    let has_success = lower.contains("# success");
    if !has_goal || !has_deadline || !has_success {
        return Err(MotivationError::MissingRequired(format!(
            "free_text 缺少 段头 (# goal / # deadline / # success), got goal={has_goal} deadline={has_deadline} success={has_success}"
        )));
    }
    Ok(())
}

/// C-SGI-3 E 层校验 — 多证据加权.
///
/// 返回加权总分 (供调用方判定 ≥ MIN_EVIDENCE_SCORE).
/// **硬约束**: 不同 EvidenceKind 数量必须 ≥ MIN_EVIDENCE_KINDS.
pub fn evidence_check(evidences: &[Evidence]) -> MotivationResult<f64> {
    let mut distinct_kinds = std::collections::HashSet::new();
    let mut total = 0.0_f64;
    for ev in evidences {
        distinct_kinds.insert(ev.kind);
        total += ev.weight.clamp(0.0, 1.0);
    }
    if distinct_kinds.len() < MIN_EVIDENCE_KINDS {
        return Err(MotivationError::EvidenceInsufficient(format!(
            "distinct EvidenceKind = {}, 必须 ≥ {MIN_EVIDENCE_KINDS}",
            distinct_kinds.len()
        )));
    }
    // 加权总分按 kinds 平均 (覆盖度优先, 单点过强不放过)
    let avg = total / (distinct_kinds.len() as f64).max(1.0);
    if avg < MIN_EVIDENCE_SCORE {
        return Err(MotivationError::EvidenceInsufficient(format!(
            "E-layer avg score = {avg:.3} < {MIN_EVIDENCE_SCORE}"
        )));
    }
    Ok(avg)
}

// ============================================
// 7. 写入流程 (§21.4 7 步)
// ============================================

/// 写入结果.
#[derive(Debug, Clone)]
pub struct WriteResult {
    /// 写入成功的 entry id (新 sgi_current.id)
    pub entry_id: Uuid,
    /// E 层校验加权分数
    pub evidence_score: f64,
    /// 写入时间戳
    pub timestamp: i64,
}

/// §21.4 7 步写入流程 — 唯一允许修改 sgi_current 的入口.
///
/// **步骤** (编译时 hardcode 顺序, 不可调整):
/// 1. C-SGI-1 唯一性
/// 2. C-SGI-7 三条必备
/// 3. C-SGI-5 / C-SGI-6 内容三选一 + 最长 N
/// 4. C-SGI-3 E 层校验
/// 5. C-SGI-2 写 sgi_history (含 predecessor)
/// 6. C-SGI-1 原子更新 sgi_current
/// 7. C-SGI-4 静默失败 → ReflectionAuditor 告警
///
/// **任何步骤失败 = 整次 SGI 变更失败, 不得部分提交.**
pub fn write_flow(
    sgi: &mut SGI,
    mut entry: SGIEntry,
    evidences: &[Evidence],
    auditor: &mut ReflectionAuditor,
    allow_duplicate: bool,
) -> MotivationResult<WriteResult> {
    // 步骤 1: C-SGI-1 唯一性
    check_csgi1_uniqueness(sgi, &entry, allow_duplicate)?;

    // 步骤 2: C-SGI-7 三条必备
    check_csgi7_three_required(&entry.content)?;

    // 步骤 3: C-SGI-5 / C-SGI-6
    check_csgi5_content_kind(&entry.content)?;
    check_csgi6_max_chars(&entry.content)?;

    // 步骤 4: C-SGI-3 E 层校验
    let evidence_score = evidence_check(evidences)?;

    // 步骤 5: C-SGI-2 写 sgi_history — 设置 predecessor + 时间戳
    entry.predecessor = sgi.current().map(|c| c.id);
    let entry_id = entry.id;
    let pre_history_len = sgi.history_len();
    sgi.commit(entry.clone());

    // 步骤 6 + 7: 原子性检查 — sgi_history 必须已 push 且 sgi_current 已切换
    if sgi.history_len() != pre_history_len + 1 || sgi.current().map(|c| c.id) != Some(entry_id) {
        // C-SGI-4 静默失败 → ReflectionAuditor 告警
        auditor.alert_silent(
            Utc::now().timestamp(),
            format!(
                "atomic commit broken: history_len {} → {}, current {:?} vs expected {}",
                pre_history_len,
                sgi.history_len(),
                sgi.current().map(|c| c.id),
                entry_id
            ),
        );
        // 已知不可恢复, 回滚 history (best-effort)
        sgi.rollback(pre_history_len);
        return Err(MotivationError::HistoryAppendFailed(
            "atomic commit broken — sgi_history push 与 sgi_current 切换不同步".into(),
        ));
    }

    // C-SGI-2 历史追加成功记录
    auditor.note_history_appended(entry_id, sgi.history_len());

    Ok(WriteResult {
        entry_id,
        evidence_score,
        timestamp: entry.timestamp,
    })
}

// ============================================
// 8. ReflectionAuditor — 告警 (C-SGI-4)
// ============================================

/// 反思期审计事件.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditEvent {
    /// C-SGI-4 静默 SGI 变更告警
    SilentSGIMutation {
        /// 时间戳
        at: i64,
        /// 原因描述
        reason: String,
    },
    /// C-SGI-2 sgi_history 追加失败
    HistoryAppendFailed {
        /// entry id
        entry_id: Uuid,
        /// 原因描述
        reason: String,
    },
    /// 正常 sgi_history 追加 (info-level, 反思期可读)
    HistoryAppended {
        /// entry id
        entry_id: Uuid,
        /// 当时 history_len
        history_len: usize,
    },
}

/// 反思期审计器 — 收集 C-SGI-4 静默变更告警 + C-SGI-2 失败告警.
#[derive(Debug, Default)]
pub struct ReflectionAuditor {
    events: Vec<AuditEvent>,
}

impl ReflectionAuditor {
    /// 创建空审计器.
    pub fn new() -> Self {
        Self::default()
    }

    /// C-SGI-4 静默 SGI 变更告警.
    pub fn alert_silent(&mut self, at: i64, reason: String) {
        self.events
            .push(AuditEvent::SilentSGIMutation { at, reason });
    }

    /// C-SGI-2 sgi_history 追加失败告警.
    pub fn alert_history_failed(&mut self, entry_id: Uuid, reason: String) {
        self.events
            .push(AuditEvent::HistoryAppendFailed { entry_id, reason });
    }

    /// sgi_history 追加成功 (info, 不计入告警).
    pub fn note_history_appended(&mut self, entry_id: Uuid, history_len: usize) {
        self.events.push(AuditEvent::HistoryAppended {
            entry_id,
            history_len,
        });
    }

    /// 读所有事件.
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// 静默告警计数 (C-SGI-4 触发次数).
    pub fn silent_alert_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, AuditEvent::SilentSGIMutation { .. }))
            .count()
    }

    /// 失败告警计数 (C-SGI-2 触发次数).
    pub fn history_failed_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| matches!(e, AuditEvent::HistoryAppendFailed { .. }))
            .count()
    }
}

// ============================================
// 9. V0.5 v2 §13 动机/价值测度 (0.06 权重, 0.85 门槛)
// ============================================

/// 自主目标一致性 (V0.5 v2 §13 维度 1 动机/价值 变量 1).
#[derive(Debug, Clone, Copy)]
pub struct AutonomyConsistency {
    /// 当前内驱强度 [0, 1]
    pub internal_intensity: f64,
    /// sgi_history 中内驱占比 [0, 1]
    pub internal_history_ratio: f64,
}

/// 价值取向稳定性 (V0.5 v2 §13 维度 1 变量 2).
#[derive(Debug, Clone, Copy)]
pub struct ValueStability {
    /// sgi_history 平均 goal 字数变化率 [0, 1] (越小越稳定)
    pub goal_turnover: f64,
    /// deadline 平均时间跨度方差 (归一化到 [0, 1])
    pub deadline_variance: f64,
}

/// 内在动力强度 (V0.5 v2 §13 维度 1 变量 3).
#[derive(Debug, Clone, Copy)]
pub struct IntrinsicIntensity {
    /// 当前 sgi_current 内驱强度
    pub current_internal: f64,
    /// 历史内驱强度峰值
    pub historical_peak: f64,
}

/// V0.5 v2 §13 维度 1 动机/价值 综合测度 — 0-1 区间, ≥ 0.85 硬门槛.
///
/// **不冻结** (主 17:43 实事求是): 权重是提议起点, 待主人拍板.
pub const MOTIVATION_WEIGHTS: (f64, f64, f64) = (0.35, 0.35, 0.30);

/// 测度结果.
#[derive(Debug, Clone, Copy)]
pub struct MotivationScore {
    /// 总分 [0, 1]
    pub total: f64,
    /// 自主目标一致性分量
    pub autonomy: f64,
    /// 价值取向稳定性分量
    pub value: f64,
    /// 内在动力强度分量
    pub intrinsic: f64,
    /// 是否通过硬门槛 (≥ 0.85)
    pub passes_threshold: bool,
}

/// 计算动机/价值维度评分 (§13 提议公式).
///
/// 公式: `motivation_score = w1*autonomy + w2*value + w3*intrinsic`
pub fn motivation_score(
    autonomy: AutonomyConsistency,
    value: ValueStability,
    intrinsic: IntrinsicIntensity,
) -> MotivationScore {
    let (w1, w2, w3) = MOTIVATION_WEIGHTS;

    // 自主目标一致性: 内驱当前强度 × 历史占比 (二者的几何均值, 任一低就拉低总分)
    let autonomy_score =
        ((autonomy.internal_intensity * autonomy.internal_history_ratio).clamp(0.0, 1.0)).sqrt();

    // 价值取向稳定性: turnover 越低越好, deadline_variance 越低越好 (互补)
    let value_score = ((1.0 - value.goal_turnover).clamp(0.0, 1.0)
        + (1.0 - value.deadline_variance).clamp(0.0, 1.0))
        / 2.0;

    // 内在动力强度: 当前 × 历史峰值 (算术均值)
    let intrinsic_score =
        ((intrinsic.current_internal + intrinsic.historical_peak) / 2.0).clamp(0.0, 1.0);

    let total = (w1 * autonomy_score + w2 * value_score + w3 * intrinsic_score).clamp(0.0, 1.0);

    MotivationScore {
        total,
        autonomy: autonomy_score,
        value: value_score,
        intrinsic: intrinsic_score,
        passes_threshold: total >= MIN_EVIDENCE_SCORE,
    }
}

// ============================================
// 10. 单元测试 (5+ tests, 编译时 hardcode)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 helper: 构造有效 SGIEntry (Structured, 含三件套).
    fn make_structured_entry(goal: &str, drive_label: &str, intensity: f64) -> SGIEntry {
        let drive = InternalDrive::new(drive_label, intensity);
        let content = SGIContent::Structured(SGIStructured {
            goal: goal.to_string(),
            deadline: "2026-12-31".to_string(),
            success_criteria: "test passes".to_string(),
            extras: HashMap::new(),
            multimodal: None,
        });
        let mut e = SGIEntry::new(content, &drive);
        e.evidence_refs = vec![
            Evidence {
                kind: EvidenceKind::Council,
                source: "test_council".into(),
                weight: 0.9,
            },
            Evidence {
                kind: EvidenceKind::History,
                source: "test_history".into(),
                weight: 0.9,
            },
            Evidence {
                kind: EvidenceKind::Principle,
                source: "test_principle".into(),
                weight: 0.9,
            },
        ];
        e
    }

    /// 测试用 helper: 构造有效 E 层证据 (3 类, 0.9 权重).
    fn good_evidences() -> Vec<Evidence> {
        vec![
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
        ]
    }

    #[test]
    fn csgi1_uniqueness_blocks_same_content() {
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let e1 = make_structured_entry("goal_A", "drive1", 0.7);
        write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();

        // 同内容第二次 → C-SGI-1 拒绝
        let e2 = make_structured_entry("goal_A", "drive2", 0.7);
        let err = write_flow(&mut sgi, e2, &good_evidences(), &mut auditor, false);
        assert!(matches!(err, Err(MotivationError::NotUnique(_))));
    }

    #[test]
    fn csgi1_uniqueness_allows_explicit_duplicate() {
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let e1 = make_structured_entry("goal_A", "drive1", 0.7);
        write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();

        // 显式声明 duplicate → 允许
        let e2 = make_structured_entry("goal_A", "drive2", 0.8);
        let ok = write_flow(&mut sgi, e2, &good_evidences(), &mut auditor, true);
        assert!(ok.is_ok());
    }

    #[test]
    fn csgi7_three_required_blocks_missing_goal() {
        let drive = InternalDrive::new("d", 0.5);
        let content = SGIContent::Structured(SGIStructured {
            goal: "".to_string(),
            deadline: "2026-12-31".to_string(),
            success_criteria: "ok".to_string(),
            extras: HashMap::new(),
            multimodal: None,
        });
        let entry = SGIEntry::new(content, &drive);
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let err = write_flow(&mut sgi, entry, &good_evidences(), &mut auditor, false);
        assert!(matches!(err, Err(MotivationError::MissingRequired(_))));
    }

    #[test]
    fn csgi6_max_chars_blocks_oversize_freetext() {
        let drive = InternalDrive::new("d", 0.5);
        // C-SGI-7 三条必备需先满足, 才能测 C-SGI-6 — 否则先报 C-SGI-7
        let prefix = "# goal: ok\n# deadline: 2026-12-31\n# success: ok\n";
        let pad = "x".repeat(SGI_MAX_TEXT_CHARS + 1);
        let huge = format!("{prefix}{pad}");
        let content = SGIContent::FreeText(huge);
        let entry = SGIEntry::new(content, &drive);
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let err = write_flow(&mut sgi, entry, &good_evidences(), &mut auditor, false);
        assert!(matches!(err, Err(MotivationError::TextTooLong { .. })));
    }

    #[test]
    fn csgi3_evidence_insufficient_blocks_two_kinds() {
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let entry = make_structured_entry("goal_X", "d", 0.6);
        // 仅 2 类证据
        let weak = vec![
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
        let err = write_flow(&mut sgi, entry, &weak, &mut auditor, false);
        assert!(matches!(err, Err(MotivationError::EvidenceInsufficient(_))));
        assert!(sgi.current().is_none());
        assert_eq!(sgi.history_len(), 0);
    }

    #[test]
    fn csgi2_audit_appends_to_history_atomically() {
        let mut sgi = SGI::new();
        let mut auditor = ReflectionAuditor::new();
        let e1 = make_structured_entry("goal_A", "d1", 0.7);
        let r1 = write_flow(&mut sgi, e1.clone(), &good_evidences(), &mut auditor, false).unwrap();
        let e2 = make_structured_entry("goal_B", "d2", 0.8);
        let r2 = write_flow(&mut sgi, e2, &good_evidences(), &mut auditor, false).unwrap();

        assert_ne!(r1.entry_id, r2.entry_id);
        assert_eq!(sgi.history_len(), 2);
        assert_eq!(sgi.current().unwrap().id, r2.entry_id);

        // predecessor 链
        let last = sgi.history().last().unwrap();
        assert_eq!(last.predecessor, Some(r1.entry_id));
    }

    #[test]
    fn drive_kind_internal_external_distinguished() {
        let i = InternalDrive::new("self", 0.9);
        let e = ExternalDrive::new("user_cmd", 0.4);
        assert_eq!(i.kind(), DriveKind::Internal);
        assert_eq!(e.kind(), DriveKind::External);
        assert!(i.intensity() > e.intensity());
    }

    #[test]
    fn motivation_score_above_threshold_for_healthy_agent() {
        // 健康代理: 三维全高 → 总分 ≥ 0.85
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.9,
                internal_history_ratio: 0.85,
            },
            ValueStability {
                goal_turnover: 0.1,
                deadline_variance: 0.1,
            },
            IntrinsicIntensity {
                current_internal: 0.9,
                historical_peak: 0.95,
            },
        );
        assert!(score.passes_threshold);
        assert!(score.total >= MIN_EVIDENCE_SCORE);
    }

    #[test]
    fn motivation_score_below_threshold_for_low_intrinsic() {
        // 内在动力弱 → 不通过门槛
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.5,
                internal_history_ratio: 0.5,
            },
            ValueStability {
                goal_turnover: 0.8,
                deadline_variance: 0.8,
            },
            IntrinsicIntensity {
                current_internal: 0.2,
                historical_peak: 0.3,
            },
        );
        assert!(!score.passes_threshold);
        assert!(score.total < MIN_EVIDENCE_SCORE);
    }

    #[test]
    fn auditor_silent_alert_counter_increments() {
        let mut auditor = ReflectionAuditor::new();
        assert_eq!(auditor.silent_alert_count(), 0);
        auditor.alert_silent(123, "test reason".into());
        assert_eq!(auditor.silent_alert_count(), 1);
        auditor.alert_history_failed(Uuid::new_v4(), "h fail".into());
        assert_eq!(auditor.history_failed_count(), 1);
    }
}


