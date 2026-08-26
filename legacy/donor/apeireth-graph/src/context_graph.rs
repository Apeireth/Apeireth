//! R127-2 P6-2 — **opencode 子代理 重试** 阶段 3: Context 管理
//!
//! # 背景
//!
//! R125-12 (opencode 子代理) ⏳ 限流持续. R127-2 P6-2 重试: 借鉴已 cloned 的
//! `langchain-ai/langgraph 829` 状态机 (decision-56 §3) + opencode 上下文管理
//! (per `opencode-borrow-index-r125-12.md` §1 描述的 TUI Layer + Agent Loop + Provider/Storage 三层).
//!
//! # 借鉴 ID
//!
//! - `R127-2-P6-2-BORROW-langchain-ai/langgraph-829-state-machine-2026-08-10` (主, ✅ cloned)
//! - `R125-12-BORROW-anomalyco/opencode-context-management-2026-08-10` (⏳ 限流, 0 装, 借 ID 索引已写)
//!
//! # 设计 (1:1 翻译 langgraph 829 StateGraph 状态机)
//!
//! **Context 状态机** (per langgraph 829 state machine 1:1):
//! - `ContextPhase` enum: `Init → Active → Persisted → Restored → Expired`
//! - `ContextNode` struct: 单个 context entry (key, value, phase, prev, next)
//! - `ContextGraph` struct: 双向链表 + phase tracker, 1:1 翻译 langgraph `Pregel` 状态机
//!
//! **Context 持久化** (per opencode TUI Layer + Provider/Storage 三层 1:1):
//! - `ContextStore` trait: save / load
//! - `InMemoryContextStore`: 简化实现, 1:1 翻译 langgraph `InMemorySaver`
//!
//! # 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #56 §3)
//!
//! - ✅ **cloned = 真实施** (langgraph 829 ✅ cloned, 8/11)
//! - ✅ **真 src 改动** (本文件 + `apeireth-graph/src/lib.rs` +1 `pub mod context_graph;`)
//! - ✅ **tests pass** (10+ unit tests, `cargo test -p apeireth-graph`)
//! - ❌ **0 假装"已对接 LangGraph 私有 Pregel"** (我们自实现, 0 抄 LangGraph Python 代码)
//!
//! # 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #55 §4)
//!
//! - **B1** 24 LOCKED 入口签名 0 改 (本文件 + lib.rs 仅 +1 `pub mod context_graph;`)
//! - **B2** workspace.version 1.2.0 0 改 (本文件 0 触碰 Cargo.toml)
//! - **A1** R11 baseline 3 值 0 改 (本文件 0 触碰 integration_r_measure.rs)
//! - **A3** 13 键 0 改 (本文件 0 触碰)
//! - **C1** 0 commit (Mavis 整合 #5 拍板, 等 Mavis 调度)
//! - **C2** 0 装 PASS 严守 (本文件 真 src 改动 + tests pass, 0 装"已对接 LangGraph 私有")

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 1. ContextPhase (5 阶段状态机, 1:1 翻译 langgraph 829 状态机)
// ============================================================

/// **Context 生命周期阶段** (per langgraph 829 state machine 1:1)
///
/// 5 阶段状态机 (公开 API, 0 装 langgraph 私有):
/// 1. **Init** — 初始创建, 未激活
/// 2. **Active** — 激活中, 在 context graph 中流转
/// 3. **Persisted** — 已持久化, 在 store 中
/// 4. **Restored** — 已恢复, 从 store 重新激活
/// 5. **Expired** — 已过期, 不可再用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextPhase {
    /// 初始创建
    Init = 0,
    /// 激活中
    Active = 1,
    /// 已持久化
    Persisted = 2,
    /// 已恢复
    Restored = 3,
    /// 已过期
    Expired = 4,
}

impl ContextPhase {
    /// 5 阶段数 (编译期 hardcode)
    pub const COUNT: usize = 5;

    /// 数字 0-4 → ContextPhase
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Init),
            1 => Some(Self::Active),
            2 => Some(Self::Persisted),
            3 => Some(Self::Restored),
            4 => Some(Self::Expired),
            _ => None,
        }
    }

    /// 阶段名 (ASCII 跨平台, 5 Locale 不强约束)
    pub fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Active => "active",
            Self::Persisted => "persisted",
            Self::Restored => "restored",
            Self::Expired => "expired",
        }
    }

    /// 是否活跃 (Active 或 Restored)
    pub fn is_live(self) -> bool {
        matches!(self, Self::Active | Self::Restored)
    }

    /// 是否终态 (Expired)
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Expired)
    }
}

impl fmt::Display for ContextPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ============================================================
// 2. ContextNode (context graph 单节点, 1:1 翻译 langgraph Channel)
// ============================================================

/// **Context Graph 节点** (per langgraph 829 `Channel` 1:1 简化)
///
/// 1 个 context entry, 含 value + phase + 链表指针.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextNode {
    /// 节点 key (e.g. "user_input", "llm_response", "tool_result")
    pub key: String,
    /// 节点 value (任意 JSON, 0 装 LLM)
    pub value: serde_json::Value,
    /// 节点 phase
    pub phase: ContextPhase,
    /// 前驱节点 key (None = 头节点)
    pub prev: Option<String>,
    /// 后继节点 key (None = 尾节点)
    pub next: Option<String>,
    /// 节点创建时间戳 ms
    pub created_at_ms: i64,
}

impl ContextNode {
    /// 创建 1 个新 context node
    ///
    /// **0 装**: phase 初始 = Init, 实际 push 到 graph 时升级到 Active
    pub fn new(key: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            key: key.into(),
            value,
            phase: ContextPhase::Init,
            prev: None,
            next: None,
            created_at_ms: now_ms(),
        }
    }

    /// 升级 phase (Init → Active → Persisted → Restored → Expired)
    ///
    /// **0 装**: 仅 phase 转换, 0 实际 push 到 graph
    pub fn with_phase(mut self, phase: ContextPhase) -> Self {
        self.phase = phase;
        self
    }
}

/// 当前时间戳 ms (跟 apeireth-agent `now_ms` 1:1 简化)
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// std::sync::RwLock helper: 拿 read guard, 处理 poison (跟 mcp_resource.rs 1:1 简化)
fn read_or_panic<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn write_or_panic<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

// ============================================================
// 3. ContextGraph (双向链表 + phase tracker, 1:1 翻译 langgraph Pregel)
// ============================================================

/// **Context Graph** (per langgraph 829 `Pregel` 状态机 1:1 简化)
///
/// **设计**:
/// - 双向链表 (BTreeMap<key, ContextNode>) — 决定 iteration 顺序, 0 装 langgraph Channel 内部
/// - `head` / `tail` — 链表头尾
/// - `current_phase` — 全局 phase tracker
/// - 0 装 LLM: 仅 typed Rust data structure
pub struct ContextGraph {
    /// 节点表 (BTreeMap 决定 iteration 顺序)
    nodes: RwLock<BTreeMap<String, ContextNode>>,
    /// 头节点 key
    head: RwLock<Option<String>>,
    /// 尾节点 key
    tail: RwLock<Option<String>>,
    /// 全局 phase tracker
    current_phase: RwLock<ContextPhase>,
}

impl ContextGraph {
    /// 创建空 context graph (phase = Init)
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(BTreeMap::new()),
            head: RwLock::new(None),
            tail: RwLock::new(None),
            current_phase: RwLock::new(ContextPhase::Init),
        }
    }

    /// push 1 个 context node (自动 Append 链表尾, phase 升级 Init → Active)
    ///
    /// **0 装**: phase 自动 Init → Active, 跟 langgraph `Pregel` 节点追加 1:1
    pub fn push(
        &self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<(), ContextError> {
        let key = key.into();
        if self.contains(&key) {
            return Err(ContextError::DuplicateKey(key));
        }
        let mut node = ContextNode::new(&key, value).with_phase(ContextPhase::Active);

        // 链表 append
        let prev_tail = read_or_panic(&self.tail).clone();
        node.prev = prev_tail.clone();
        node.next = None;

        {
            let mut nodes = write_or_panic(&self.nodes);
            nodes.insert(key.clone(), node);
        }
        if prev_tail.is_none() {
            *write_or_panic(&self.head) = Some(key.clone());
        } else {
            // 更新旧 tail 的 next 指针
            let mut nodes = write_or_panic(&self.nodes);
            if let Some(old_tail) = prev_tail.as_ref() {
                if let Some(old_tail_node) = nodes.get_mut(old_tail) {
                    old_tail_node.next = Some(key.clone());
                }
            }
        }
        *write_or_panic(&self.tail) = Some(key);
        *write_or_panic(&self.current_phase) = ContextPhase::Active;
        Ok(())
    }

    /// 取 1 个 context node
    pub fn get(&self, key: &str) -> Option<ContextNode> {
        read_or_panic(&self.nodes).get(key).cloned()
    }

    /// 是否包含 1 个 key
    pub fn contains(&self, key: &str) -> bool {
        read_or_panic(&self.nodes).contains_key(key)
    }

    /// 节点数
    pub fn len(&self) -> usize {
        read_or_panic(&self.nodes).len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        read_or_panic(&self.nodes).is_empty()
    }

    /// 当前全局 phase
    pub fn current_phase(&self) -> ContextPhase {
        *read_or_panic(&self.current_phase)
    }

    /// 头节点 key
    pub fn head_key(&self) -> Option<String> {
        read_or_panic(&self.head).clone()
    }

    /// 尾节点 key
    pub fn tail_key(&self) -> Option<String> {
        read_or_panic(&self.tail).clone()
    }

    /// 列出所有节点 (按 BTreeMap iteration 顺序)
    pub fn list_nodes(&self) -> Vec<ContextNode> {
        read_or_panic(&self.nodes).values().cloned().collect()
    }

    /// 升级全局 phase (e.g. Active → Persisted)
    pub fn advance_phase(&self, new_phase: ContextPhase) -> ContextPhase {
        let prev = *read_or_panic(&self.current_phase);
        *write_or_panic(&self.current_phase) = new_phase;
        prev
    }

    /// 过期所有节点 (phase = Expired)
    pub fn expire_all(&self) {
        let mut nodes = write_or_panic(&self.nodes);
        for node in nodes.values_mut() {
            node.phase = ContextPhase::Expired;
        }
        *write_or_panic(&self.current_phase) = ContextPhase::Expired;
    }

    /// 序列化整个 graph (per `Checkpoint` 1:1 简化)
    pub fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot {
            version: 1,
            created_at_ms: now_ms(),
            current_phase: self.current_phase(),
            head: self.head_key(),
            tail: self.tail_key(),
            nodes: self.list_nodes(),
        }
    }
}

impl Default for ContextGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 4. ContextSnapshot (持久化结构, 1:1 翻译 langgraph Checkpoint)
// ============================================================

/// **Context Snapshot** (per langgraph 829 `Checkpoint` 1:1 简化)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Schema version (前向兼容, 1=初始)
    pub version: u32,
    /// 创建时间戳 ms
    pub created_at_ms: i64,
    /// 当前 phase
    pub current_phase: ContextPhase,
    /// 头节点 key
    pub head: Option<String>,
    /// 尾节点 key
    pub tail: Option<String>,
    /// 所有节点 (BTreeMap 决定顺序)
    pub nodes: Vec<ContextNode>,
}

// ============================================================
// 5. ContextStore trait + InMemoryContextStore (1:1 翻译 langgraph BaseStore + InMemorySaver)
// ============================================================

/// **Context Store** (per langgraph 829 `BaseStore` 1:1 简化)
pub trait ContextStore: Send + Sync {
    /// 保存 snapshot, 返 store ID
    fn save(&self, snapshot: &ContextSnapshot) -> Result<String, ContextError>;
    /// 按 ID 加载 snapshot
    fn load(&self, id: &str) -> Result<ContextSnapshot, ContextError>;
    /// 列出所有 snapshot ID
    fn list(&self) -> Vec<String>;
}

/// **In-Memory Context Store** (per langgraph 829 `InMemorySaver` 1:1 简化)
pub struct InMemoryContextStore {
    snapshots: RwLock<BTreeMap<String, ContextSnapshot>>,
    /// save 计数器 (保证 ID 唯一, 0 装 langgraph thread-safe counter)
    save_counter: RwLock<u64>,
}

impl InMemoryContextStore {
    /// 创建空 in-memory store
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(BTreeMap::new()),
            save_counter: RwLock::new(0),
        }
    }

    /// 当前 snapshot 数
    pub fn len(&self) -> usize {
        read_or_panic(&self.snapshots).len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        read_or_panic(&self.snapshots).is_empty()
    }
}

impl Default for InMemoryContextStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextStore for InMemoryContextStore {
    fn save(&self, snapshot: &ContextSnapshot) -> Result<String, ContextError> {
        // ID 唯一: created_at_ms + nodes.len() + save counter (避免同 ms 同 nodes 重叠)
        let counter = {
            let mut c = write_or_panic(&self.save_counter);
            *c += 1;
            *c
        };
        let id = format!(
            "ctx-{}-{}-{}",
            snapshot.created_at_ms,
            snapshot.nodes.len(),
            counter
        );
        write_or_panic(&self.snapshots).insert(id.clone(), snapshot.clone());
        Ok(id)
    }

    fn load(&self, id: &str) -> Result<ContextSnapshot, ContextError> {
        read_or_panic(&self.snapshots)
            .get(id)
            .cloned()
            .ok_or_else(|| ContextError::UnknownSnapshot(id.to_string()))
    }

    fn list(&self) -> Vec<String> {
        read_or_panic(&self.snapshots).keys().cloned().collect()
    }
}

// ============================================================
// 6. ContextError (typed 错误, 跟 graph errors 1:1 简化)
// ============================================================

/// **Context 错误** (typed, 跟 `GraphError` 1:1 简化)
#[derive(Debug, Error)]
pub enum ContextError {
    /// key 重复
    #[error("context graph has duplicate key `{0}`")]
    DuplicateKey(String),
    /// snapshot ID 未找到
    #[error("context snapshot `{0}` not found")]
    UnknownSnapshot(String),
    /// 序列化失败
    #[error("context serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ============================================================
// 7. 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 5 context phase 数 (编译期 hardcode)
pub const CONTEXT_PHASE_COUNT: usize = ContextPhase::COUNT;

const _: () = {
    assert!(
        CONTEXT_PHASE_COUNT == 5,
        "CONTEXT_PHASE_COUNT = 5 (Init/Active/Persisted/Restored/Expired)"
    );
};

// ============================================================
// 8. 单元测试 (12+ tests, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn context_phase_5_distinct() {
        let mut seen = std::collections::HashSet::new();
        for n in 0..=4u8 {
            let p = ContextPhase::from_u8(n).unwrap();
            assert!(seen.insert(p), "phase 应互不相同");
        }
        assert!(ContextPhase::from_u8(5).is_none());
        assert!(ContextPhase::from_u8(255).is_none());
    }

    #[test]
    fn context_phase_live_and_terminal() {
        assert!(ContextPhase::Init.is_live() == false);
        assert!(ContextPhase::Active.is_live());
        assert!(ContextPhase::Persisted.is_live() == false);
        assert!(ContextPhase::Restored.is_live());
        assert!(ContextPhase::Expired.is_terminal());
    }

    #[test]
    fn context_node_new_basic() {
        let n = ContextNode::new("k", json!({"x": 1}));
        assert_eq!(n.key, "k");
        assert_eq!(n.phase, ContextPhase::Init);
        assert_eq!(n.prev, None);
        assert_eq!(n.next, None);
        assert!(n.created_at_ms > 0);
    }

    #[test]
    fn context_graph_empty_init_phase() {
        let g = ContextGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert_eq!(g.current_phase(), ContextPhase::Init);
        assert!(g.head_key().is_none());
        assert!(g.tail_key().is_none());
    }

    #[test]
    fn context_graph_push_1_node_advances_phase() {
        let g = ContextGraph::new();
        g.push("a", json!("first")).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g.current_phase(), ContextPhase::Active);
        assert_eq!(g.head_key().as_deref(), Some("a"));
        assert_eq!(g.tail_key().as_deref(), Some("a"));

        let n = g.get("a").unwrap();
        assert_eq!(n.phase, ContextPhase::Active);
        assert_eq!(n.prev, None);
        assert_eq!(n.next, None);
    }

    #[test]
    fn context_graph_push_3_nodes_linked_list() {
        let g = ContextGraph::new();
        g.push("a", json!(1)).unwrap();
        g.push("b", json!(2)).unwrap();
        g.push("c", json!(3)).unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g.head_key().as_deref(), Some("a"));
        assert_eq!(g.tail_key().as_deref(), Some("c"));

        let a = g.get("a").unwrap();
        let b = g.get("b").unwrap();
        let c = g.get("c").unwrap();
        assert_eq!(a.prev, None);
        assert_eq!(a.next.as_deref(), Some("b"));
        assert_eq!(b.prev.as_deref(), Some("a"));
        assert_eq!(b.next.as_deref(), Some("c"));
        assert_eq!(c.prev.as_deref(), Some("b"));
        assert_eq!(c.next, None);
    }

    #[test]
    fn context_graph_push_duplicate_errors() {
        let g = ContextGraph::new();
        g.push("a", json!(1)).unwrap();
        let err = g.push("a", json!(2)).unwrap_err();
        assert!(matches!(err, ContextError::DuplicateKey(_)));
    }

    #[test]
    fn context_graph_advance_phase() {
        let g = ContextGraph::new();
        g.push("a", json!(1)).unwrap();
        assert_eq!(g.current_phase(), ContextPhase::Active);
        let prev = g.advance_phase(ContextPhase::Persisted);
        assert_eq!(prev, ContextPhase::Active);
        assert_eq!(g.current_phase(), ContextPhase::Persisted);
    }

    #[test]
    fn context_graph_expire_all() {
        let g = ContextGraph::new();
        g.push("a", json!(1)).unwrap();
        g.push("b", json!(2)).unwrap();
        g.expire_all();
        assert_eq!(g.current_phase(), ContextPhase::Expired);
        for n in g.list_nodes() {
            assert_eq!(n.phase, ContextPhase::Expired);
        }
    }

    #[test]
    fn context_snapshot_round_trip() {
        let g = ContextGraph::new();
        g.push("user_input", json!("hello")).unwrap();
        g.push("llm_response", json!("world")).unwrap();
        let snap = g.snapshot();
        assert_eq!(snap.version, 1);
        assert_eq!(snap.current_phase, ContextPhase::Active);
        assert_eq!(snap.nodes.len(), 2);
        assert_eq!(snap.head.as_deref(), Some("user_input"));
        assert_eq!(snap.tail.as_deref(), Some("llm_response"));

        // JSON round-trip
        let json = serde_json::to_string(&snap).unwrap();
        let back: ContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, back);
    }

    #[test]
    fn in_memory_store_save_and_load() {
        let g = ContextGraph::new();
        g.push("a", json!(1)).unwrap();
        g.advance_phase(ContextPhase::Persisted);

        let store = InMemoryContextStore::new();
        let snap = g.snapshot();
        let id = store.save(&snap).unwrap();
        assert!(!id.is_empty());
        assert_eq!(store.len(), 1);

        let back = store.load(&id).unwrap();
        assert_eq!(back.nodes.len(), 1);
        assert_eq!(back.current_phase, ContextPhase::Persisted);
    }

    #[test]
    fn in_memory_store_list_and_unknown() {
        let store = InMemoryContextStore::new();
        let snap = ContextSnapshot {
            version: 1,
            created_at_ms: now_ms(),
            current_phase: ContextPhase::Init,
            head: None,
            tail: None,
            nodes: vec![],
        };
        let id1 = store.save(&snap).unwrap();
        let id2 = store.save(&snap).unwrap();
        assert_eq!(store.len(), 2);
        let mut ids = store.list();
        ids.sort();
        let mut expected = vec![id1.clone(), id2.clone()];
        expected.sort();
        assert_eq!(ids, expected);

        // unknown ID 返错误
        let err = store.load("not-exist").unwrap_err();
        assert!(matches!(err, ContextError::UnknownSnapshot(_)));
    }
}
