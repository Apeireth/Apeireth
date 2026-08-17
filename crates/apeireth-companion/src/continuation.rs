//! `apeireth-companion::continuation` — 续行快照 + 段编辑原语 (TP16).
//!
//! 模块结构 (双件):
//! - **续行快照** (上, 原有): `ContinuationSnapshot` + `ContinuationStore`,
//!   工具调用点保存 LLM 上下文, 崩溃/重启后 `consume` 恢复续跑 (多轮 function calling 断点续传)
//! - **段编辑原语** (下, TP16 新增): `SegmentEditor` + `EditAction`,
//!   暴露 `retain( block_id)` / `remove(block_id)` / `replace(block_id, new)`,
//!   供 LLM (rot 触发后) 按 id 删除/替换陈旧上下文块
//!
//! 0 假装: 这是「持久化 + 恢复 + 段编辑」的机制件; 真 LLM 循环 (发请求) 由调用方
//! (example/daemon) 提供 — lib 不依赖 `apeireth-api` (同 judicator 的 trait 策略).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::RotBlock;

/// 挂起的工具调用 (异步等待回调 / 崩溃时未完成的那一步).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToolCall {
    pub tool_name: String,
    pub args: Value,
    pub call_id: String,
}

/// 续行快照: 一次可恢复的 LLM 上下文.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationSnapshot {
    pub id: String,
    pub session_id: String,
    /// LLM 上下文消息 (OpenAI 形状; 每轮保存, 恢复后追加真实结果继续).
    pub messages: Vec<Value>,
    /// 挂起的工具调用 (None = 非异步断点).
    pub pending_tool_call: Option<PendingToolCall>,
    pub saved_at_ms: i64,
    pub turn: u64,
}

/// 续行快照存储: 目录 + 原子写 (tmp+rename), 按 id 存 `{id}.json`.
pub struct ContinuationStore {
    dir: PathBuf,
}

impl ContinuationStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// 原子保存: 先写 tmp, rename 覆盖 (崩溃安全).
    pub fn save(&self, snap: &ContinuationSnapshot) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("创建快照目录失败: {e}"))?;
        let tmp = self.dir.join(format!("{}.tmp-{}", snap.id, uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(snap)
            .map_err(|e| format!("快照序列化失败: {e}"))?;
        std::fs::write(&tmp, bytes).map_err(|e| format!("写 tmp 失败: {e}"))?;
        std::fs::rename(&tmp, self.path_for(&snap.id))
            .map_err(|e| format!("原子提交失败: {e}"))?;
        Ok(())
    }

    pub fn exists(&self, id: &str) -> bool {
        self.path_for(id).exists()
    }

    pub fn load(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let bytes = std::fs::read(self.path_for(id))
            .map_err(|e| format!("读快照 {id} 失败: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("解析快照 {id} 失败: {e}"))
    }

    /// 消费 (load + 删除): 快照一次性.
    pub fn consume(&self, id: &str) -> Result<ContinuationSnapshot, String> {
        let snap = self.load(id)?;
        std::fs::remove_file(self.path_for(id))
            .map_err(|e| format!("删除快照 {id} 失败: {e}"))?;
        Ok(snap)
    }

    /// 列出全部快照 id.
    pub fn list(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        rd.filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .collect()
    }
}

// ============================================================
// TP16 — 段编辑原语 (SegmentEditor + EditAction)
// 哲学 (per TP16 §1.2 机制而非补丁 + §1.3 集成而非分立):
// - rot_score 由 context.rs 计算 (deterministic, 0 LLM); LLM 仅参与**段编辑**
// - retain / remove / replace 三个原语, 接受 LLM 输出的 block_id (字符串) + new_content
// - 数据结构复用 context.rs 的 RotBlock (注入 / 编辑 共用形状, 互不反向依赖)
// - 一切"动作"返回 Result — 0 装: 不假装成功
// ============================================================

/// 段编辑动作 (LLM 输出 / 人工审核输入)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    /// 保留 (不动, 标记该块为"不要动")
    Retain { block_id: String },
    /// 删除 (从 SegmentEditor 移除)
    Remove { block_id: String },
    /// 替换为新内容 (同时 bump last_touched_ms 到 now_ms)
    Replace { block_id: String, new_content: String },
}

impl EditAction {
    /// 提取 block_id (调试 / 日志用)
    pub fn block_id(&self) -> &str {
        match self {
            Self::Retain { block_id }
            | Self::Remove { block_id }
            | Self::Replace { block_id, .. } => block_id,
        }
    }
}

/// 段编辑错误 (thiserror, 不假装)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    /// 块 id 为空字符串
    EmptyBlockId,
    /// 块 id 在编辑器中找不到
    BlockNotFound(String),
    /// 同一动作序列对同一 block_id 有重复 (冲突, 拒绝应用整批)
    ConflictingActions(String),
    /// 空 new_content 用于 replace (0 装: 与 remove 语义混淆, 拒绝)
    EmptyReplaceContent,
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBlockId => write!(f, "block_id 不能为空"),
            Self::BlockNotFound(id) => write!(f, "block id 找不到: {id}"),
            Self::ConflictingActions(id) => write!(f, "同一批次对 {id} 有冲突动作"),
            Self::EmptyReplaceContent => write!(f, "replace 不能传空 new_content"),
        }
    }
}

impl std::error::Error for EditorError {}

/// 段编辑器: 持有有序块列表 (BTreeMap 保序 + O(log n) 操作), 暴露 LLM 用 retain/remove/replace
///
/// 设计要点:
/// - 顺序: 插入顺序 (用于输出与快照保存)
/// - 操作均为不可分应用 (`apply_one` → Result; 一错即停)
/// - 0 装: 不假装"已编辑",返回 `Result<(), EditorError>`
#[derive(Debug, Clone, Default)]
pub struct SegmentEditor {
    /// 主存 (保插入序)
    blocks: BTreeMap<String, RotBlock>,
    /// 顺序辅助: 优化输出 + 兼容 BTreeMap 迭代
    order: Vec<String>,
    /// 时间锚 (default 0 = "未知时间"; 调用方应通过 `set_now_ms` 覆盖)
    now_ms: i64,
}

impl SegmentEditor {
    /// 空编辑器
    pub fn new() -> Self {
        Self::default()
    }

    /// 带 now_ms 构造
    pub fn with_now_ms(now_ms: i64) -> Self {
        Self { blocks: BTreeMap::new(), order: Vec::new(), now_ms }
    }

    /// 从 RotBlock 列表构造 (顺序保留)
    pub fn from_blocks(blocks: impl IntoIterator<Item = RotBlock>) -> Self {
        let mut s = Self::new();
        for b in blocks {
            // 跳过空 id 的块, 0 装: 不假装
            if b.block_id.trim().is_empty() {
                continue;
            }
            // 重复 id 跳过首个之后的 (保留首次)
            if s.blocks.contains_key(&b.block_id) {
                continue;
            }
            s.order.push(b.block_id.clone());
            s.blocks.insert(b.block_id.clone(), b);
        }
        s
    }

    /// 设时间锚 (后续 replace 会用它 bump last_touched_ms)
    pub fn set_now_ms(&mut self, ms: i64) {
        self.now_ms = ms;
    }

    pub fn now_ms(&self) -> i64 {
        self.now_ms
    }

    /// 插入新块 (id 已存在 = no-op, 不报错, 0 装作说明)
    pub fn insert(&mut self, b: RotBlock) -> Result<(), EditorError> {
        if b.block_id.trim().is_empty() {
            return Err(EditorError::EmptyBlockId);
        }
        if self.blocks.contains_key(&b.block_id) {
            return Ok(()); // 已存在 = no-op, 单测明确
        }
        self.order.push(b.block_id.clone());
        self.blocks.insert(b.block_id.clone(), b);
        Ok(())
    }

    /// 块数
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.blocks.contains_key(id)
    }

    pub fn get(&self, id: &str) -> Option<&RotBlock> {
        self.blocks.get(id)
    }

    /// `retain`: 校验 id 存在; 语义 = "标记保留, 不动". 本编辑器不维护 pinned list (那是 rot 度量层的事).
    pub fn retain(&mut self, id: &str) -> Result<(), EditorError> {
        if id.trim().is_empty() {
            return Err(EditorError::EmptyBlockId);
        }
        if !self.blocks.contains_key(id) {
            return Err(EditorError::BlockNotFound(id.into()));
        }
        Ok(())
    }

    /// `remove`: 删除并返回原块 (供上层 audit / 重放)
    pub fn remove(&mut self, id: &str) -> Result<RotBlock, EditorError> {
        if id.trim().is_empty() {
            return Err(EditorError::EmptyBlockId);
        }
        match self.blocks.remove(id) {
            Some(b) => {
                self.order.retain(|x| x != id);
                Ok(b)
            }
            None => Err(EditorError::BlockNotFound(id.into())),
        }
    }

    /// `replace`: 内容替换 + 自动 bump last_touched_ms = self.now_ms (now_ms=0 时不 bump, 0 装标注)
    pub fn replace(&mut self, id: &str, new_content: String) -> Result<(), EditorError> {
        if id.trim().is_empty() {
            return Err(EditorError::EmptyBlockId);
        }
        if new_content.is_empty() {
            return Err(EditorError::EmptyReplaceContent);
        }
        let b = self
            .blocks
            .get_mut(id)
            .ok_or_else(|| EditorError::BlockNotFound(id.into()))?;
        b.content = new_content;
        if self.now_ms > 0 {
            b.last_touched_ms = self.now_ms;
        }
        Ok(())
    }

    /// 触摸块 (单刷时间戳, 适用于"被引用过"语义)
    pub fn touch(&mut self, id: &str) -> Result<(), EditorError> {
        if id.trim().is_empty() {
            return Err(EditorError::EmptyBlockId);
        }
        let b = self
            .blocks
            .get_mut(id)
            .ok_or_else(|| EditorError::BlockNotFound(id.into()))?;
        if self.now_ms > 0 {
            b.last_touched_ms = self.now_ms;
        }
        Ok(())
    }

    /// 批量 apply (供 LLM 输出解析后一次性提交).
    /// **all-or-nothing 三段式**:
    /// 1. 冲突预检: 同一批次对同一 block_id 多次动作 → 整批拒绝
    /// 2. dry-check: 逐动作验证 (id 存在 / new_content 非空), 失败 → 整批拒绝
    /// 3. 提交: 仅全部通过后才应用 (半成品绝不会持久)
    pub fn apply(&mut self, actions: &[EditAction]) -> Result<Vec<EditOutcome>, EditorError> {
        // Phase 1: 冲突预检 (同一批次同一 id 多次动作)
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for a in actions {
            let id = a.block_id();
            if !seen.insert(id) {
                return Err(EditorError::ConflictingActions(id.into()));
            }
        }
        // Phase 2: dry-check (id 存在 / new_content 非空), 全部成功才能进入 Phase 3
        for a in actions {
            self.dry_check(a)?;
        }
        // Phase 3: 提交 (此时所有动作必然可成功)
        let mut outcomes = Vec::with_capacity(actions.len());
        for a in actions {
            let out = self.apply_one(a.clone())?;
            outcomes.push(out);
        }
        Ok(outcomes)
    }

    /// 只读验证 (不改 state): 让 apply 的 all-or-nothing 语义对 missing block 也生效.
    fn dry_check(&self, a: &EditAction) -> Result<(), EditorError> {
        match a {
            EditAction::Retain { block_id } | EditAction::Remove { block_id } => {
                if block_id.trim().is_empty() {
                    return Err(EditorError::EmptyBlockId);
                }
                if !self.blocks.contains_key(block_id) {
                    return Err(EditorError::BlockNotFound(block_id.clone()));
                }
            }
            EditAction::Replace { block_id, new_content } => {
                if block_id.trim().is_empty() {
                    return Err(EditorError::EmptyBlockId);
                }
                if new_content.is_empty() {
                    return Err(EditorError::EmptyReplaceContent);
                }
                if !self.blocks.contains_key(block_id) {
                    return Err(EditorError::BlockNotFound(block_id.clone()));
                }
            }
        }
        Ok(())
    }

    fn apply_one(&mut self, a: EditAction) -> Result<EditOutcome, EditorError> {
        match a {
            EditAction::Retain { block_id } => {
                self.retain(&block_id)?;
                Ok(EditOutcome::Retained(block_id))
            }
            EditAction::Remove { block_id } => {
                self.remove(&block_id)?;
                Ok(EditOutcome::Removed(block_id))
            }
            EditAction::Replace { block_id, new_content } => {
                self.replace(&block_id, new_content)?;
                Ok(EditOutcome::Replaced(block_id))
            }
        }
    }

    /// 输出当前块 (按插入顺序, 借用)
    pub fn blocks(&self) -> Vec<&RotBlock> {
        self.order
            .iter()
            .filter_map(|id| self.blocks.get(id))
            .collect()
    }

    /// 消耗: 输出 owned (调试 + 测试)
    pub fn into_blocks(self) -> Vec<RotBlock> {
        let SegmentEditor { blocks, order, .. } = self;
        // 顺序: order 是 sequence of keys; 用 map 直接 lookup by String
        let map = blocks; // BTreeMap<String, RotBlock>
        order.into_iter().filter_map(|id| map.get(&id).cloned()).collect()
    }
}

/// apply 输出 (供 LLM 决策回放 / 调试 / 审计)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOutcome {
    /// 已保留
    Retained(String),
    /// 已删除
    Removed(String),
    /// 已替换
    Replaced(String),
}

#[cfg(test)]
mod segment_editor_tests {
    use super::*;
    use crate::context::RotBlock;

    fn blk(id: &str, content: &str) -> RotBlock {
        RotBlock::new(id, content)
    }

    #[test]
    fn edit_action_block_id_extractor() {
        assert_eq!(
            EditAction::Retain { block_id: "a".into() }.block_id(),
            "a"
        );
        assert_eq!(
            EditAction::Remove { block_id: "b".into() }.block_id(),
            "b"
        );
        assert_eq!(
            EditAction::Replace {
                block_id: "c".into(),
                new_content: "x".into()
            }
            .block_id(),
            "c"
        );
    }

    #[test]
    fn from_blocks_preserves_order_and_skips_duplicates_or_empty() {
        let blocks = vec![
            blk("a", "first long content block for ordering test purposes here"),
            blk("b", "second long content block for ordering test purposes here"),
            blk("a", "duplicate should be ignored by from_blocks initial pass"),
            blk("  ", "block with empty trimmed id should be skipped entirely here"),
            blk("c", "third long content block for ordering test purposes here also"),
        ];
        let ed = SegmentEditor::from_blocks(blocks);
        assert_eq!(ed.len(), 3);
        let ids: Vec<&str> = ed.blocks().iter().map(|b| b.block_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"], "插入序保留, dup 与空 id 过滤");
    }

    #[test]
    fn insert_basic_and_no_op_on_dup() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha long content block for insertion test purposes here")).unwrap();
        ed.insert(blk("b", "beta long content block for insertion test purposes here")).unwrap();
        // dup insert no-op
        ed.insert(blk("a", "alpha v2 long content block for insertion test purposes here")).unwrap();
        assert_eq!(ed.len(), 2);
        assert_eq!(ed.get("a").unwrap().content,
                   "alpha long content block for insertion test purposes here",
                   "dup insert 不覆盖, 0 装作 '已保留原值'");
    }

    #[test]
    fn insert_rejects_empty_block_id() {
        let mut ed = SegmentEditor::new();
        let err = ed.insert(blk("  ", "content here")).unwrap_err();
        assert_eq!(err, EditorError::EmptyBlockId);
    }

    #[test]
    fn retain_existing_ok_missing_err() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha long content block for retain test purposes here")).unwrap();
        ed.retain("a").expect("existing");
        let err = ed.retain("nope").unwrap_err();
        assert_eq!(err, EditorError::BlockNotFound("nope".into()));
    }

    #[test]
    fn remove_existing_returns_block_missing_err() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha long content block for remove test purposes here")).unwrap();
        let removed = ed.remove("a").unwrap();
        assert_eq!(removed.block_id, "a");
        assert_eq!(ed.len(), 0);
        let err = ed.remove("a").unwrap_err();
        assert_eq!(err, EditorError::BlockNotFound("a".into()));
    }

    #[test]
    fn replace_updates_content_and_bumps_touched() {
        let mut ed = SegmentEditor::with_now_ms(1000);
        ed.insert(blk("a", "old alpha content block for replace test purposes here")).unwrap();
        assert_eq!(ed.get("a").unwrap().last_touched_ms, 0);
        ed.replace("a", "new alpha content block after replacement has been applied here now".into()).unwrap();
        let a = ed.get("a").unwrap();
        assert!(a.content.starts_with("new alpha"));
        assert_eq!(a.last_touched_ms, 1000, "有 now_ms 时应 bump touched");
    }

    #[test]
    fn replace_no_now_ms_does_not_bump() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "old alpha content block for replace without nowms purposes")).unwrap();
        let before = ed.get("a").unwrap().last_touched_ms;
        ed.replace("a", "new alpha content block for replace without nowms purposes here".into()).unwrap();
        let after = ed.get("a").unwrap().last_touched_ms;
        assert_eq!(before, after, "now_ms=0 → 不 bump (0 装作 '未知时间')");
    }

    #[test]
    fn replace_rejects_empty_content_and_missing_block() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha content block for replace validation tests here")).unwrap();
        let err = ed.replace("a", "".into()).unwrap_err();
        assert_eq!(err, EditorError::EmptyReplaceContent);
        let err = ed.replace("nope", "content block for replace validation tests here".into()).unwrap_err();
        assert_eq!(err, EditorError::BlockNotFound("nope".into()));
    }

    #[test]
    fn touch_bumps_when_now_ms_positive() {
        let mut ed = SegmentEditor::with_now_ms(2000);
        ed.insert(blk("a", "alpha content block for touch test purposes here")).unwrap();
        assert_eq!(ed.get("a").unwrap().last_touched_ms, 0);
        ed.touch("a").unwrap();
        assert_eq!(ed.get("a").unwrap().last_touched_ms, 2000);
    }

    #[test]
    fn apply_batch_retain_remove_replace() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha long content block for batch apply retain test here")).unwrap();
        ed.insert(blk("b", "beta long content block for batch apply remove test here")).unwrap();
        ed.insert(blk("c", "gamma long content block for batch apply replace test here")).unwrap();
        let actions = vec![
            EditAction::Retain { block_id: "a".into() },
            EditAction::Remove { block_id: "b".into() },
            EditAction::Replace {
                block_id: "c".into(),
                new_content: "gamma replaced content block for batch apply test here".into(),
            },
        ];
        let outcomes = ed.apply(&actions).unwrap();
        assert_eq!(outcomes.len(), 3);
        assert_eq!(outcomes[0], EditOutcome::Retained("a".into()));
        assert_eq!(outcomes[1], EditOutcome::Removed("b".into()));
        assert_eq!(outcomes[2], EditOutcome::Replaced("c".into()));
        assert_eq!(ed.len(), 2);
        assert!(ed.contains("a"));
        assert!(!ed.contains("b"));
        assert!(ed.contains("c"));
        assert!(ed.get("c").unwrap().content.starts_with("gamma replaced"));
    }

    #[test]
    fn apply_batch_conflict_same_id_two_actions_rejects_all() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha long content block for conflict test purposes here")).unwrap();
        let actions = vec![
            EditAction::Retain { block_id: "a".into() },
            EditAction::Remove { block_id: "a".into() },
        ];
        let err = ed.apply(&actions).unwrap_err();
        assert_eq!(err, EditorError::ConflictingActions("a".into()));
        assert!(ed.contains("a"), "冲突时整批不应已应用任一动作");
    }

    #[test]
    fn apply_batch_partial_not_applied_on_error() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("a", "alpha content block for partial test purposes here please")).unwrap();
        ed.insert(blk("b", "beta content block for partial test purposes here please")).unwrap();
        let actions = vec![
            EditAction::Remove { block_id: "a".into() },
            EditAction::Replace {
                block_id: "nope".into(),
                new_content: "new content block for partial apply test purposes here".into(),
            },
        ];
        let err = ed.apply(&actions).unwrap_err();
        assert_eq!(err, EditorError::BlockNotFound("nope".into()));
        assert!(ed.contains("a"), "失败前已应用的 Remove 不应持久; a 应仍在 ✓");
        assert!(ed.contains("b"), "b 完全未动 ✓");
    }

    #[test]
    fn into_blocks_preserves_insertion_order() {
        let blocks = vec![
            blk("z", "zulu long content block for into order test purposes here please"),
            blk("a", "alpha long content block for into order test purposes here please"),
            blk("m", "mike long content block for into order test purposes here please"),
        ];
        let ed = SegmentEditor::from_blocks(blocks);
        let out = ed.into_blocks();
        assert_eq!(out[0].block_id, "z");
        assert_eq!(out[1].block_id, "a");
        assert_eq!(out[2].block_id, "m");
    }

    #[test]
    fn empty_editor_basic_invariants() {
        let ed = SegmentEditor::new();
        assert!(ed.is_empty());
        assert_eq!(ed.len(), 0);
        assert!(!ed.contains("any"));
        assert!(ed.blocks().is_empty());
        assert!(ed.into_blocks().is_empty());
    }

    #[test]
    fn editor_blocks_vec_returns_references_in_insertion_order() {
        let mut ed = SegmentEditor::new();
        ed.insert(blk("x", "x-ray content block for vec ref test purposes here please")).unwrap();
        ed.insert(blk("y", "yankee content block for vec ref test purposes here please")).unwrap();
        let v = ed.blocks();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].block_id, "x");
        assert_eq!(v[1].block_id, "y");
    }

    #[test]
    fn set_now_ms_accessor() {
        let mut ed = SegmentEditor::new();
        assert_eq!(ed.now_ms(), 0);
        ed.set_now_ms(5000);
        assert_eq!(ed.now_ms(), 5000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snap(id: &str, turn: u64) -> ContinuationSnapshot {
        ContinuationSnapshot {
            id: id.into(),
            session_id: "me".into(),
            messages: vec![
                json!({"role": "user", "content": format!("第{turn}轮问题")}),
                json!({"role": "assistant", "content": "思考中", "tool_calls": [{"id": "c1", "function": {"name": "FileOperator", "arguments": "{}"}}]}),
            ],
            pending_tool_call: Some(PendingToolCall {
                tool_name: "FileOperator".into(),
                args: json!({"op": "write", "path": "x"}),
                call_id: "c1".into(),
            }),
            saved_at_ms: turn as i64 * 1000,
            turn,
        }
    }

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "apeireth-continuation-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn save_load_round_trip_preserves_everything() {
        let store = ContinuationStore::new(tmp_dir("rt"));
        let s = snap("snap-1", 3);
        store.save(&s).unwrap();
        assert!(store.exists("snap-1"));
        let loaded = store.load("snap-1").unwrap();
        assert_eq!(loaded.id, "snap-1");
        assert_eq!(loaded.turn, 3);
        assert_eq!(loaded.messages.len(), 2);
        let p = loaded.pending_tool_call.unwrap();
        assert_eq!(p.tool_name, "FileOperator");
        assert_eq!(p.call_id, "c1");
        // 原子性: 无 tmp 残留
        let left: Vec<_> = std::fs::read_dir(store.dir.as_path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec!["snap-1.json"], "不应有 tmp 残留: {left:?}");
    }

    #[test]
    fn consume_loads_and_deletes() {
        let store = ContinuationStore::new(tmp_dir("consume"));
        store.save(&snap("s2", 1)).unwrap();
        let s = store.consume("s2").unwrap();
        assert_eq!(s.id, "s2");
        assert!(!store.exists("s2"));
        assert!(store.list().is_empty());
    }

    #[test]
    fn crash_recovery_resumes_from_last_snapshot() {
        // 模拟: 进程跑第 1 轮 → save → 崩溃 → 新进程 (同目录新 store) load → 追加 → 继续
        let dir = tmp_dir("crash");
        let store1 = ContinuationStore::new(&dir);
        store1.save(&snap("s3", 1)).unwrap();
        drop(store1); // "崩溃"

        let store2 = ContinuationStore::new(&dir); // "重启"
        assert!(store2.exists("s3"));
        let mut recovered = store2.consume("s3").unwrap();
        assert_eq!(recovered.turn, 1);
        // 恢复后追加真实工具结果, 继续下一轮
        recovered.messages.push(json!({"role": "tool", "tool_call_id": "c1", "content": "写入成功"}));
        recovered.turn = 2;
        store2.save(&recovered).unwrap();
        let final_snap = store2.load("s3").unwrap();
        assert_eq!(final_snap.turn, 2);
        assert_eq!(final_snap.messages.len(), 3, "上下文应累积恢复");
        assert_eq!(final_snap.messages[2]["role"], "tool");
    }

    #[test]
    fn load_missing_returns_error() {
        let store = ContinuationStore::new(tmp_dir("missing"));
        assert!(store.load("nope").is_err());
    }
}
