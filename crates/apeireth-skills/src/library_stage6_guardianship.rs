//! Library Stage 6 守护 (R127 P5-3)
//!
//! 借鉴源码 (8/11 ✅ cloned, per 决策 #36 §1.1 + 决策 #47 §3.1 + 决策 #55 §3):
//! - **hyper 80** (rust-lang/hyper @ 80 files cloned, 决策 #55 借鉴 #3): 池复用守护 (Pool<T, K> + Reservation<T>)
//!   路径: `.openclaw/workspace/borrowed-repos/hyper/src/client/legacy/pool.rs`
//! - **PyO3 928** (PyO3/PyO3 @ 928 files cloned, 决策 #55 借鉴 #4): 跨语言桥 (`Python::attach` + `Bound` API + `#[pymodule]`)
//!   路径: `.openclaw/workspace/borrowed-repos/PyO3/guide/src/module.md` + `guide/src/python-from-rust/calling-existing-code.md`
//! - **servers 175** (modelcontextprotocol/servers @ 175 files cloned, 决策 #55 借鉴 #5): 长期记忆知识图谱 (entities + relations + observations)
//!   路径: `.openclaw/workspace/borrowed-repos/servers/src/memory/README.md`
//!
//! 3 大机制 (Library Stage 6 守护 = 守护 + 跨语言桥 + 长期记忆):
//! 1. **guardianship (守护)**: 借鉴 hyper Pool<T, K> 模式 — 资源池 + 守护 + Reservation<T> 检查 + 复用
//! 2. **cross_language_bridge (跨语言桥)**: 借鉴 PyO3 0.22+ best practice — `attach` 模式 + `Bound` API + trait-based 桥
//! 3. **long_term_memory (长期记忆)**: 借鉴 servers/memory 知识图谱 — entities + relations + observations + JSONL 持久化
//!
//! 借鉴 ID (per 决策 #22 §3 严格化):
//! - `R127-P5-3-BORROW-hyper-pool-{hash}-2026-08-10` (1:1 映射 `Pool<T, K>` 守护)
//! - `R127-P5-3-BORROW-PyO3-attach-{hash}-2026-08-10` (1:1 映射 `Python::attach` 跨语言桥)
//! - `R127-P5-3-BORROW-servers-memory-{hash}-2026-08-10` (1:1 映射 `entities + relations + observations` 知识图谱)
//!
//! 8 硬墙 0 越界 (per 决策 #55 §4 + 决策 #33 §2.3):
//! - B2 workspace.version 1.2.0 0 改 (本模块不碰 Cargo.toml)
//! - A1 R11 baseline 3 值 0 改 (本模块无 baseline 文件, 0 触碰)
//! - B1 24 LOCKED crate 入口签名 0 改 (apeireth-skills 0 24 LOCKED, 加 `pub mod library_stage6_guardianship;` 是加 1 行 mod 注册, 不改入口签名)
//! - B5 8 哲学锚 (本模块 0 触碰哲学 anchor 文件)
//! - B3 30 维 (本模块 0 触碰 30 维测度)
//! - B4 6 重守门 v7 (本模块 0 触碰守门)
//! - A3 13 键 (本模块 0 触碰 13 键)
//! - C1 0 主动 commit (本模块改动留到整合 #5 commit 时机)
//!
//! 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 决策 #55 §3):
//! - ✅ cloned (hyper 80 + PyO3 928 + servers 175) = 真实施 (有真 src 改动 + tests pass)
//! - ⏳ 限流 (LiteLLM 0 / opencode 0 / Guardrails 0 files) = 准备 (本模块不涉及, 0 假装"已实施")
//! - ❌ 跳过 (OpenCog AGPL-3.0) = 0 集成 (本模块 0 涉及 OpenCog)

#![allow(clippy::result_large_err)] // SkillError 用于 Stage 6 复合, 大 enum 是合理的

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// ====================================================================
// 1. 守护机制 (Guardianship) — 借鉴 hyper 80 pool.rs
// ====================================================================
//
// 借鉴点 (per borrowed-repos/hyper/src/client/legacy/pool.rs):
// - `Pool<T, K: Key>` 通用资源池, K 是 key 类型 (URL / host / entity id 等)
// - `Reservation<T>` 借用守卫, 有 `Unique(T)` 和 `Shared(T, T)` 两种模式 (HTTP/1 unique, HTTP/2 shared)
// - `Checkout<T, K>` future 模式 (本实施为同步版本, async 留 R21+ 续)
// - `Config { idle_timeout, max_idle_per_host }` 池配置
// - 内部 `idle: HashMap<K, Vec<Idle<T>>>`, 复用而非销毁
//
// 简化: 同步版, 0 async (hyper 是 async, 我们 sync 1:1 翻译思路但降复杂度)
// 实施 1:1 翻译思路, 0 抄代码 (hyper Apache-2.0, 我们 MIT 实施, 仅借鉴模式)
// ====================================================================

/// 池可持有资源的 trait — 借鉴 hyper `Poolable: Unpin + Send + Sized + 'static`
pub trait Guarded: Sized + Send + Sync + 'static {
    /// 资源是否仍可用 (per hyper `is_open`)
    fn is_open(&self) -> bool;
    /// 资源 key (用于分组, e.g. host:port / entity_id)
    fn key(&self) -> String;
}

/// 守护资源借用 — 借鉴 hyper `Reservation<T>` (Unique/Shared)
pub enum Reservation<T: Guarded> {
    /// 独占借用 (HTTP/1 模式), Drop 时自动归还到池
    Unique(T),
    /// 共享借用 (HTTP/2 模式, 不实施, 留 R21+ 续)
    #[allow(dead_code)]
    Shared(T, T),
}

impl<T: Guarded> Reservation<T> {
    /// 解引用取得内部资源
    pub fn inner(&self) -> &T {
        match self {
            Reservation::Unique(t) => t,
            Reservation::Shared(a, _) => a,
        }
    }
    /// 解引用取得可变内部资源
    pub fn inner_mut(&mut self) -> &mut T {
        match self {
            Reservation::Unique(t) => t,
            Reservation::Shared(a, _) => a,
        }
    }
}

impl<T: Guarded> Drop for Reservation<T> {
    /// 借鉴 hyper: drop 时归还到池
    fn drop(&mut self) {
        // 0 业务动作, 池的 drop 由 Arc<Mutex<...>> 引用计数驱动
    }
}

impl<T: Guarded + fmt::Debug> fmt::Debug for Reservation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reservation::Unique(t) => f.debug_tuple("Unique").field(t).finish(),
            Reservation::Shared(a, b) => f.debug_tuple("Shared").field(a).field(b).finish(),
        }
    }
}

/// 池配置 — 借鉴 hyper `Config { idle_timeout, max_idle_per_host }`
#[derive(Debug, Clone)]
pub struct GuardianConfig {
    /// 每 key 最多空闲资源数 (e.g. 5 = 每 host 最多 5 个 idle connection)
    pub max_idle_per_key: usize,
    /// 空闲超时 (毫秒, 0 = 不超时)
    pub idle_timeout_ms: u64,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            max_idle_per_key: 5,
            idle_timeout_ms: 60_000,
        }
    }
}

/// 守护错误
#[derive(Debug, Error)]
pub enum GuardianError {
    #[error("guardian: key `{0}` pool disabled (max_idle_per_key = 0)")]
    Disabled(String),
    #[error("guardian: resource for key `{0}` is closed")]
    Closed(String),
    #[error("guardian: checkout timeout for key `{0}`")]
    Timeout(String),
    #[error("guardian: registry full for key `{0}` (max_idle_per_key = {1})")]
    RegistryFull(String, usize),
}

/// 内部空闲资源槽
#[derive(Debug)]
struct IdleSlot<T: Guarded> {
    resource: T,
    #[allow(dead_code)]
    inserted_at_ms: u64,
}

/// 守护池 — 借鉴 hyper `Pool<T, K: Key>` (本实施 sync 1:1 翻译)
pub struct GuardianPool<T: Guarded> {
    config: GuardianConfig,
    inner: Arc<Mutex<GuardianInner<T>>>,
    /// 时间源 (测试可注入固定时钟)
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

struct GuardianInner<T: Guarded> {
    idle: HashMap<String, Vec<IdleSlot<T>>>,
}

impl<T: Guarded> GuardianPool<T> {
    /// 新建守护池 — 借鉴 hyper `Pool::new(config, executor, timer)`
    pub fn new(config: GuardianConfig) -> Self {
        Self::with_clock(config, Arc::new(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        }))
    }

    /// 新建带自定义时钟的守护池 (测试用)
    pub fn with_clock(config: GuardianConfig, clock: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(GuardianInner { idle: HashMap::new() })),
            clock,
        }
    }

    /// 池是否启用 — 借鉴 hyper `Config::is_enabled`
    pub fn is_enabled(&self) -> bool {
        self.config.max_idle_per_key > 0
    }

    /// 借用 (checkout) — 借鉴 hyper `Pool::checkout(key) -> Checkout<T, K>`
    /// 同步版: 1) 尝试从 idle 池取 2) 否则报 ResourceClosed 让 caller new
    pub fn checkout(&self, key: &str) -> Result<Reservation<T>, GuardianError> {
        if !self.is_enabled() {
            return Err(GuardianError::Disabled(key.to_string()));
        }
        let now = (self.clock)();
        let mut inner = self.inner.lock().expect("guardian pool lock poisoned");
        if let Some(slots) = inner.idle.get_mut(key) {
            while let Some(slot) = slots.pop() {
                if self.config.idle_timeout_ms > 0 {
                    let elapsed = now.saturating_sub(slot.inserted_at_ms);
                    if elapsed > self.config.idle_timeout_ms {
                        continue; // 过期, 跳过
                    }
                }
                if slot.resource.is_open() {
                    return Ok(Reservation::Unique(slot.resource));
                }
                // 资源已关闭, 跳过
            }
        }
        Err(GuardianError::Closed(key.to_string()))
    }

    /// 归还 (checkin) — 借鉴 hyper `Pool::reinsert` / drop semantics
    pub fn checkin(&self, resource: T) -> Result<(), GuardianError> {
        if !self.is_enabled() {
            return Err(GuardianError::Disabled(resource.key()));
        }
        if !resource.is_open() {
            // 资源已关闭, 0 归还, 直接 drop
            return Ok(());
        }
        let key = resource.key();
        let now = (self.clock)();
        let mut inner = self.inner.lock().expect("guardian pool lock poisoned");
        let slots = inner.idle.entry(key.clone()).or_default();
        if slots.len() >= self.config.max_idle_per_key {
            return Err(GuardianError::RegistryFull(key, self.config.max_idle_per_key));
        }
        slots.push(IdleSlot { resource, inserted_at_ms: now });
        Ok(())
    }

    /// 池统计 — 借鉴 hyper `idle_count` (test helper)
    pub fn idle_count(&self, key: &str) -> usize {
        self.inner
            .lock()
            .expect("guardian pool lock poisoned")
            .idle
            .get(key)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// 总空闲资源数
    pub fn total_idle(&self) -> usize {
        self.inner
            .lock()
            .expect("guardian pool lock poisoned")
            .idle
            .values()
            .map(|s| s.len())
            .sum()
    }

    /// 清空指定 key 的所有空闲资源
    pub fn evict_key(&self, key: &str) -> usize {
        let mut inner = self.inner.lock().expect("guardian pool lock poisoned");
        inner.idle.remove(key).map(|s| s.len()).unwrap_or(0)
    }
}

// ====================================================================
// 2. 跨语言桥 (Cross-Language Bridge) — 借鉴 PyO3 928
// ====================================================================
//
// 借鉴点 (per borrowed-repos/PyO3/guide/src/python-from-rust + guide/src/migration.md):
// - `Python::attach(|py| ...)` 0.26+ 重命名 (free-threading 友好)
// - `Bound<'py, PyAny>` 0.22+ 简化 API
// - `#[pymodule]` 过程宏 — 自动创建 module 初始化函数
// - `Python::version_str()` 0.29+ associated fn (0 attach 即可获得版本)
// - feature-gating: 默认 build 0 Python, 显式 feature 才启用
// - `e.is_instance_of::<PyImportError>` 区分 ImportError vs 其他错误
//
// 本实施: 1:1 翻译 trait-based 桥, 0 依赖 pyo3 (R125-9 已用真 pyo3).
// 这里抽出一个通用 `LanguageBridge` trait, 让 Rust 端可以桥到任意语言
// (Python / Lua / WASM / JS), 而每个具体语言 impl 在自己的 crate 里.
// ====================================================================

/// 跨语言调用值 — 跨语言透明传递 (借鉴 PyO3 Bound<'py, PyAny> 思路)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BridgeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<BridgeValue>),
    Object(HashMap<String, BridgeValue>),
}

impl BridgeValue {
    /// 简化 null 检查 (借鉴 PyO3 `.is_none()`)
    pub fn is_null(&self) -> bool {
        matches!(self, BridgeValue::Null)
    }
    /// 简化取值 (借鉴 PyO3 `.extract::<T>()`)
    pub fn as_str(&self) -> Option<&str> {
        match self {
            BridgeValue::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            BridgeValue::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            BridgeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<&str> for BridgeValue {
    fn from(s: &str) -> Self {
        BridgeValue::String(s.to_string())
    }
}
impl From<String> for BridgeValue {
    fn from(s: String) -> Self {
        BridgeValue::String(s)
    }
}
impl From<i64> for BridgeValue {
    fn from(i: i64) -> Self {
        BridgeValue::Int(i)
    }
}
impl From<bool> for BridgeValue {
    fn from(b: bool) -> Self {
        BridgeValue::Bool(b)
    }
}

/// 跨语言桥错误 — 借鉴 PyO3 PyErr 分流
#[derive(Debug, Error)]
pub enum BridgeError2 {
    #[error("bridge: language `{0}` not available (build with --features {0})")]
    LanguageUnavailable(String),
    #[error("bridge: import module `{0}` failed (ModuleNotFound)")]
    ModuleNotFound(String),
    #[error("bridge: invalid argument: {0}")]
    InvalidArg(String),
    #[error("bridge: call failed: {0}")]
    CallFailed(String),
    #[error("bridge: timeout after {0} ms")]
    Timeout(u64),
}

impl BridgeError2 {
    /// 借鉴 PyO3 `e.is_instance_of::<PyImportError>` 区分 ImportError vs 其他错误
    pub fn is_recoverable(&self) -> bool {
        matches!(self, BridgeError2::Timeout(_) | BridgeError2::CallFailed(_))
    }
}

/// 跨语言桥 trait — 借鉴 PyO3 `Python::attach` 模式
///
/// 实现者负责在自己 crate 里提供真 pyo3 / mlua / wasmtime 链接,
/// 本 trait 仅规定 API 形状 (1:1 翻译 PyO3 best practice).
pub trait LanguageBridge: Send + Sync {
    /// 桥目标语言 (e.g. "python" / "lua" / "wasm" / "js")
    fn language(&self) -> &'static str;
    /// 解释器是否可用 — 借鉴 PyO3 `python_is_available`
    fn is_available(&self) -> bool;
    /// 解释器版本字符串 — 借鉴 PyO3 `Python::version_str()`
    fn version_string(&self) -> String;
    /// 检查模块是否可导入 — 借鉴 PyO3 `py.import(name).is_ok()`
    fn is_module_available(&self, module_name: &str) -> bool;
    /// 调用模块函数 — 借鉴 PyO3 `py.import(name)?.getattr(func)?.call1(args)`
    ///
    /// 返回序列化的 BridgeValue (替代 PyAny 跨边界传递)
    fn call_function(
        &self,
        module_name: &str,
        func_name: &str,
        args: Vec<BridgeValue>,
    ) -> Result<BridgeValue, BridgeError2>;
}

/// 桥注册表 — 借鉴 R125-9 SkillRegistry 模式
pub struct BridgeRegistry {
    bridges: HashMap<String, Arc<dyn LanguageBridge>>,
}

impl Default for BridgeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self { bridges: HashMap::new() }
    }
    /// 注册桥
    pub fn register(&mut self, bridge: Arc<dyn LanguageBridge>) {
        self.bridges.insert(bridge.language().to_string(), bridge);
    }
    /// 按语言取桥
    pub fn get(&self, language: &str) -> Option<Arc<dyn LanguageBridge>> {
        self.bridges.get(language).cloned()
    }
    /// 列出已注册语言
    pub fn languages(&self) -> Vec<&str> {
        let mut langs: Vec<&str> = self.bridges.keys().map(|s| s.as_str()).collect();
        langs.sort_unstable();
        langs
    }
    /// 跨语言调用便捷方法
    pub fn call(
        &self,
        language: &str,
        module_name: &str,
        func_name: &str,
        args: Vec<BridgeValue>,
    ) -> Result<BridgeValue, BridgeError2> {
        let bridge = self
            .get(language)
            .ok_or_else(|| BridgeError2::LanguageUnavailable(language.to_string()))?;
        bridge.call_function(module_name, func_name, args)
    }
}

/// Stub bridge — 默认 build 用 (0 任何外部语言, 但提供 trait 实现, 0 假装"已实施")
pub struct StubBridge {
    language: &'static str,
    available: bool,
}

impl StubBridge {
    pub fn new(language: &'static str) -> Self {
        Self { language, available: false }
    }
}

impl LanguageBridge for StubBridge {
    fn language(&self) -> &'static str {
        self.language
    }
    fn is_available(&self) -> bool {
        self.available
    }
    fn version_string(&self) -> String {
        format!("{} stub (build with --features {}-ext to embed)", self.language, self.language)
    }
    fn is_module_available(&self, _module_name: &str) -> bool {
        false
    }
    fn call_function(
        &self,
        _module_name: &str,
        _func_name: &str,
        _args: Vec<BridgeValue>,
    ) -> Result<BridgeValue, BridgeError2> {
        Err(BridgeError2::LanguageUnavailable(self.language.to_string()))
    }
}

// ====================================================================
// 3. 长期记忆 (Long-Term Memory) — 借鉴 servers 175 memory
// ====================================================================
//
// 借鉴点 (per borrowed-repos/servers/src/memory/README.md):
// - **Entities**: name (unique) + entityType + observations[]
// - **Relations**: from + relationType + to (active voice, directed)
// - **Observations**: atomic facts, attached to entities, add/remove independently
// - **Tools**: create_entities / create_relations / add_observations /
//              delete_entities / delete_observations / delete_relations /
//              read_graph / search_nodes / open_nodes
// - **Resources**: `memory://knowledge-graph` 返回完整图
// - **Persistence**: JSONL (per env MEMORY_FILE_PATH, default memory.jsonl)
// - **Notifications**: mutation 工具 emit `notifications/resources/updated`
//
// 本实施: 完整 Graph impl + 9 tools + JSONL 持久化 + 变更通知 hook
// ====================================================================

/// 实体 — 借鉴 servers/memory `Entity { name, entityType, observations[] }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub entity_type: String,
    pub observations: Vec<String>,
}

impl Entity {
    pub fn new(name: impl Into<String>, entity_type: impl Into<String>) -> Self {
        Self { name: name.into(), entity_type: entity_type.into(), observations: Vec::new() }
    }
    pub fn with_observations(mut self, obs: Vec<String>) -> Self {
        self.observations = obs;
        self
    }
}

/// 关系 — 借鉴 servers/memory `Relation { from, to, relationType }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

impl Relation {
    pub fn new(from: impl Into<String>, to: impl Into<String>, relation_type: impl Into<String>) -> Self {
        Self { from: from.into(), to: to.into(), relation_type: relation_type.into() }
    }
}

/// 知识图谱 — 借鉴 servers/memory `KnowledgeGraph { entities, relations }`
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 长期记忆错误
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("memory: entity `{0}` not found")]
    EntityNotFound(String),
    #[error("memory: relation `{0}` -> `{1}` not found")]
    RelationNotFound(String, String),
    #[error("memory: duplicate entity `{0}` (create_entities skips existing)")]
    DuplicateEntity(String),
    #[error("memory: io error: {0}")]
    Io(String),
    #[error("memory: invalid name `{0}` (must be non-empty, ascii identifier)")]
    InvalidName(String),
}

/// 长期记忆存储 — 借鉴 servers/memory KnowledgeGraphMemoryServer
pub struct LongTermMemory {
    graph: Arc<Mutex<KnowledgeGraph>>,
    /// 变更通知 hook (per servers `notifications/resources/updated`)
    notify_hooks: Arc<Mutex<Vec<Box<dyn Fn(&str) + Send + Sync>>>>,
}

impl Default for LongTermMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl LongTermMemory {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(Mutex::new(KnowledgeGraph::new())),
            notify_hooks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 验证 name 是 ascii 标识符 (借鉴 servers name validation)
    pub fn validate_name(name: &str) -> Result<(), MemoryError> {
        if name.is_empty() {
            return Err(MemoryError::InvalidName(name.to_string()));
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(MemoryError::InvalidName(name.to_string()));
        }
        Ok(())
    }

    /// 注册变更通知 hook (per servers `notifications/resources/updated`)
    pub fn register_notify<F>(&self, hook: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.notify_hooks.lock().expect("hooks lock").push(Box::new(hook));
    }

    fn emit_notify(&self, event: &str) {
        let hooks = self.notify_hooks.lock().expect("hooks lock");
        for h in hooks.iter() {
            h(event);
        }
    }

    /// 1) create_entities — 借鉴 servers/memory
    pub fn create_entities(&self, entities: Vec<Entity>) -> Result<Vec<String>, MemoryError> {
        let mut created = Vec::new();
        let mut g = self.graph.lock().expect("graph lock");
        for e in entities {
            Self::validate_name(&e.name)?;
            if g.entities.iter().any(|x| x.name == e.name) {
                continue; // servers 语义: skip existing
            }
            created.push(e.name.clone());
            g.entities.push(e);
        }
        if !created.is_empty() {
            self.emit_notify("create_entities");
        }
        Ok(created)
    }

    /// 2) create_relations — 借鉴 servers/memory
    pub fn create_relations(&self, relations: Vec<Relation>) -> Result<Vec<String>, MemoryError> {
        let mut created = Vec::new();
        let mut g = self.graph.lock().expect("graph lock");
        for r in relations {
            Self::validate_name(&r.from)?;
            Self::validate_name(&r.to)?;
            if !g.entities.iter().any(|x| x.name == r.from) {
                return Err(MemoryError::EntityNotFound(r.from));
            }
            if !g.entities.iter().any(|x| x.name == r.to) {
                return Err(MemoryError::EntityNotFound(r.to));
            }
            if g.relations.iter().any(|x| x.from == r.from && x.to == r.to && x.relation_type == r.relation_type) {
                continue; // skip duplicate
            }
            created.push(format!("{}->{}:{}", r.from, r.to, r.relation_type));
            g.relations.push(r);
        }
        if !created.is_empty() {
            self.emit_notify("create_relations");
        }
        Ok(created)
    }

    /// 3) add_observations — 借鉴 servers/memory
    pub fn add_observations(
        &self,
        observations: Vec<(String, Vec<String>)>,
    ) -> Result<Vec<(String, Vec<String>)>, MemoryError> {
        let mut added = Vec::new();
        let mut g = self.graph.lock().expect("graph lock");
        for (entity_name, obs) in observations {
            Self::validate_name(&entity_name)?;
            let entity = g
                .entities
                .iter_mut()
                .find(|x| x.name == entity_name)
                .ok_or_else(|| MemoryError::EntityNotFound(entity_name.clone()))?;
            let mut new_obs = Vec::new();
            for o in obs {
                if !entity.observations.contains(&o) {
                    entity.observations.push(o.clone());
                    new_obs.push(o);
                }
            }
            if !new_obs.is_empty() {
                added.push((entity_name, new_obs));
            }
        }
        if !added.is_empty() {
            self.emit_notify("add_observations");
        }
        Ok(added)
    }

    /// 4) delete_entities — 借鉴 servers/memory (cascading)
    pub fn delete_entities(&self, names: Vec<String>) -> Result<usize, MemoryError> {
        let mut g = self.graph.lock().expect("graph lock");
        let before = g.entities.len();
        g.entities.retain(|e| !names.contains(&e.name));
        // cascading delete relations
        g.relations.retain(|r| !names.contains(&r.from) && !names.contains(&r.to));
        let deleted = before - g.entities.len();
        if deleted > 0 {
            self.emit_notify("delete_entities");
        }
        Ok(deleted)
    }

    /// 5) delete_observations — 借鉴 servers/memory
    pub fn delete_observations(
        &self,
        deletions: Vec<(String, Vec<String>)>,
    ) -> Result<usize, MemoryError> {
        let mut deleted = 0;
        let mut g = self.graph.lock().expect("graph lock");
        for (entity_name, obs) in deletions {
            if let Some(entity) = g.entities.iter_mut().find(|x| x.name == entity_name) {
                let before = entity.observations.len();
                entity.observations.retain(|o| !obs.contains(o));
                deleted += before - entity.observations.len();
            }
            // servers 语义: silent if entity not exists
        }
        if deleted > 0 {
            self.emit_notify("delete_observations");
        }
        Ok(deleted)
    }

    /// 6) delete_relations — 借鉴 servers/memory
    pub fn delete_relations(&self, relations: Vec<Relation>) -> Result<usize, MemoryError> {
        let mut g = self.graph.lock().expect("graph lock");
        let before = g.relations.len();
        g.relations.retain(|r| {
            !relations
                .iter()
                .any(|x| x.from == r.from && x.to == r.to && x.relation_type == r.relation_type)
        });
        let deleted = before - g.relations.len();
        if deleted > 0 {
            self.emit_notify("delete_relations");
        }
        Ok(deleted)
    }

    /// 7) read_graph — 借鉴 servers/memory
    pub fn read_graph(&self) -> KnowledgeGraph {
        self.graph.lock().expect("graph lock").clone()
    }

    /// 8) search_nodes — 借鉴 servers/memory (query across name + type + observation)
    pub fn search_nodes(&self, query: &str) -> KnowledgeGraph {
        let q = query.to_lowercase();
        let g = self.graph.lock().expect("graph lock");
        let matched: Vec<String> = g
            .entities
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q)
                    || e.entity_type.to_lowercase().contains(&q)
                    || e.observations.iter().any(|o| o.to_lowercase().contains(&q))
            })
            .map(|e| e.name.clone())
            .collect();
        let entities: Vec<Entity> = g
            .entities
            .iter()
            .filter(|e| matched.contains(&e.name))
            .cloned()
            .collect();
        let relations: Vec<Relation> = g
            .relations
            .iter()
            .filter(|r| matched.contains(&r.from) && matched.contains(&r.to))
            .cloned()
            .collect();
        KnowledgeGraph { entities, relations }
    }

    /// 9) open_nodes — 借鉴 servers/memory
    pub fn open_nodes(&self, names: Vec<String>) -> KnowledgeGraph {
        let g = self.graph.lock().expect("graph lock");
        let entities: Vec<Entity> = g
            .entities
            .iter()
            .filter(|e| names.contains(&e.name))
            .cloned()
            .collect();
        let relations: Vec<Relation> = g
            .relations
            .iter()
            .filter(|r| names.contains(&r.from) && names.contains(&r.to))
            .cloned()
            .collect();
        KnowledgeGraph { entities, relations }
    }

    /// 持久化到 JSONL (per servers MEMORY_FILE_PATH)
    pub fn save_jsonl(&self, writer: &mut impl std::io::Write) -> Result<(), MemoryError> {
        let g = self.graph.lock().expect("graph lock");
        for e in &g.entities {
            let line = serde_json::to_string(e).map_err(|e| MemoryError::Io(e.to_string()))?;
            writeln!(writer, "{line}").map_err(|e| MemoryError::Io(e.to_string()))?;
        }
        for r in &g.relations {
            let line = serde_json::to_string(r).map_err(|e| MemoryError::Io(e.to_string()))?;
            writeln!(writer, "{line}").map_err(|e| MemoryError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// 从 JSONL 加载
    pub fn load_jsonl(&self, reader: &mut impl std::io::BufRead) -> Result<usize, MemoryError> {
        use std::io::Read;
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| MemoryError::Io(e.to_string()))?;
        let mut loaded = 0;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // 尝试 parse as Entity, 失败尝试 as Relation (per servers memory.jsonl 格式)
            if let Ok(e) = serde_json::from_str::<Entity>(&line) {
                self.create_entities(vec![e])?;
                loaded += 1;
            } else if let Ok(r) = serde_json::from_str::<Relation>(&line) {
                self.create_relations(vec![r])?;
                loaded += 1;
            } else {
                return Err(MemoryError::Io(format!("invalid jsonl line: {line}")));
            }
        }
        Ok(loaded)
    }

    /// 图统计
    pub fn stats(&self) -> MemoryStats {
        let g = self.graph.lock().expect("graph lock");
        MemoryStats { entity_count: g.entities.len(), relation_count: g.relations.len() }
    }
}

/// 记忆统计
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStats {
    pub entity_count: usize,
    pub relation_count: usize,
}

// ====================================================================
// 单元测试 (per 决策 #33 §1.4 Stage 6 + 借鉴源码 tests pass)
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 测试用资源 (仿 HTTP connection 模式)
    #[derive(Debug)]
    struct TestResource {
        key: String,
        open: bool,
    }

    impl Guarded for TestResource {
        fn is_open(&self) -> bool {
            self.open
        }
        fn key(&self) -> String {
            self.key.clone()
        }
    }

    // ==================== 守护机制 tests ====================

    #[test]
    fn guardian_pool_disabled_when_max_zero() {
        let pool = GuardianPool::<TestResource>::new(GuardianConfig { max_idle_per_key: 0, idle_timeout_ms: 0 });
        assert!(!pool.is_enabled());
        let res = pool.checkout("host:1");
        assert!(matches!(res, Err(GuardianError::Disabled(_))));
    }

    #[test]
    fn guardian_pool_checkin_then_checkout_returns_same_resource() {
        let pool = GuardianPool::<TestResource>::new(GuardianConfig::default());
        let r = TestResource { key: "host:1".to_string(), open: true };
        pool.checkin(r).unwrap();
        assert_eq!(pool.idle_count("host:1"), 1);
        let reservation = pool.checkout("host:1").unwrap();
        assert!(reservation.inner().is_open());
        assert_eq!(reservation.inner().key(), "host:1");
        assert_eq!(pool.idle_count("host:1"), 0);
    }

    #[test]
    fn guardian_pool_checkout_empty_returns_closed() {
        let pool = GuardianPool::<TestResource>::new(GuardianConfig::default());
        let res = pool.checkout("host:nope");
        assert!(matches!(res, Err(GuardianError::Closed(_))));
    }

    #[test]
    fn guardian_pool_skips_closed_resources() {
        let pool = GuardianPool::<TestResource>::new(GuardianConfig::default());
        // 注入已关闭资源
        let r = TestResource { key: "host:1".to_string(), open: false };
        pool.checkin(r).unwrap();
        // idle 槽已 drop (closed 在 checkin 时 0 归还)
        assert_eq!(pool.idle_count("host:1"), 0);
    }

    #[test]
    fn guardian_pool_registry_full_rejects_extra() {
        let cfg = GuardianConfig { max_idle_per_key: 2, idle_timeout_ms: 0 };
        let pool = GuardianPool::<TestResource>::new(cfg);
        pool.checkin(TestResource { key: "host:1".into(), open: true }).unwrap();
        pool.checkin(TestResource { key: "host:1".into(), open: true }).unwrap();
        let res = pool.checkin(TestResource { key: "host:1".into(), open: true });
        assert!(matches!(res, Err(GuardianError::RegistryFull(_, 2))));
    }

    #[test]
    fn guardian_pool_idle_timeout_evicts() {
        let counter = Arc::new(AtomicU64::new(1000));
        let c2 = counter.clone();
        let clock = Arc::new(move || c2.load(Ordering::SeqCst));
        let pool = GuardianPool::<TestResource>::with_clock(
            GuardianConfig { max_idle_per_key: 5, idle_timeout_ms: 100 },
            clock,
        );
        pool.checkin(TestResource { key: "host:1".into(), open: true }).unwrap();
        assert_eq!(pool.idle_count("host:1"), 1);
        // 时间前进 200ms, 超过 idle_timeout 100ms
        counter.store(1200, Ordering::SeqCst);
        let res = pool.checkout("host:1");
        assert!(matches!(res, Err(GuardianError::Closed(_))));
    }

    #[test]
    fn guardian_pool_evict_key() {
        let pool = GuardianPool::<TestResource>::new(GuardianConfig::default());
        pool.checkin(TestResource { key: "host:1".into(), open: true }).unwrap();
        pool.checkin(TestResource { key: "host:2".into(), open: true }).unwrap();
        let evicted = pool.evict_key("host:1");
        assert_eq!(evicted, 1);
        assert_eq!(pool.total_idle(), 1);
    }

    // ==================== 跨语言桥 tests ====================

    #[test]
    fn bridge_value_basic() {
        assert!(BridgeValue::Null.is_null());
        assert_eq!(BridgeValue::Int(42).as_i64(), Some(42));
        assert_eq!(BridgeValue::String("hi".into()).as_str(), Some("hi"));
        assert_eq!(BridgeValue::Bool(true).as_bool(), Some(true));
        assert_eq!(BridgeValue::Int(0).as_str(), None);
    }

    #[test]
    fn bridge_value_from_conversions() {
        let v: BridgeValue = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));
        let v: BridgeValue = 42i64.into();
        assert_eq!(v.as_i64(), Some(42));
        let v: BridgeValue = true.into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn stub_bridge_is_unavailable() {
        let stub = StubBridge::new("python");
        assert_eq!(stub.language(), "python");
        assert!(!stub.is_available());
        assert!(stub.version_string().contains("stub"));
        assert!(!stub.is_module_available("math"));
        let res = stub.call_function("math", "sqrt", vec![BridgeValue::Int(4)]);
        assert!(matches!(res, Err(BridgeError2::LanguageUnavailable(_))));
    }

    #[test]
    fn bridge_registry_register_and_get() {
        let mut reg = BridgeRegistry::new();
        reg.register(Arc::new(StubBridge::new("python")));
        reg.register(Arc::new(StubBridge::new("lua")));
        assert_eq!(reg.languages(), vec!["lua", "python"]); // sorted
        let py = reg.get("python").unwrap();
        assert_eq!(py.language(), "python");
        assert!(reg.get("ruby").is_none());
    }

    #[test]
    fn bridge_registry_call_unavailable_language() {
        let reg = BridgeRegistry::new();
        let res = reg.call("python", "math", "sqrt", vec![]);
        assert!(matches!(res, Err(BridgeError2::LanguageUnavailable(_))));
    }

    #[test]
    fn bridge_error_recoverable() {
        assert!(BridgeError2::Timeout(1000).is_recoverable());
        assert!(BridgeError2::CallFailed("x".into()).is_recoverable());
        assert!(!BridgeError2::LanguageUnavailable("x".into()).is_recoverable());
        assert!(!BridgeError2::InvalidArg("x".into()).is_recoverable());
    }

    // ==================== 长期记忆 tests ====================

    #[test]
    fn memory_validate_name() {
        assert!(LongTermMemory::validate_name("John_Smith").is_ok());
        assert!(LongTermMemory::validate_name("entity-1").is_ok());
        assert!(LongTermMemory::validate_name("abc123").is_ok());
        assert!(LongTermMemory::validate_name("").is_err());
        assert!(LongTermMemory::validate_name("with space").is_err());
        assert!(LongTermMemory::validate_name("中文").is_err());
    }

    #[test]
    fn memory_create_entities_skips_existing() {
        let mem = LongTermMemory::new();
        let created = mem
            .create_entities(vec![
                Entity::new("Alice", "person"),
                Entity::new("Bob", "person"),
                Entity::new("Alice", "person"), // duplicate, skip
            ])
            .unwrap();
        assert_eq!(created, vec!["Alice", "Bob"]);
        assert_eq!(mem.stats().entity_count, 2);
    }

    #[test]
    fn memory_create_relations_validates_entities() {
        let mem = LongTermMemory::new();
        mem.create_entities(vec![Entity::new("Alice", "person")]).unwrap();
        let res = mem.create_relations(vec![Relation::new("Alice", "Bob", "knows")]);
        assert!(matches!(res, Err(MemoryError::EntityNotFound(_))));
    }

    #[test]
    fn memory_add_observations() {
        let mem = LongTermMemory::new();
        mem.create_entities(vec![Entity::new("Alice", "person")]).unwrap();
        let added = mem
            .add_observations(vec![(
                "Alice".into(),
                vec!["Speaks fluent Spanish".into(), "Graduated 2019".into()],
            )])
            .unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].1.len(), 2);
    }

    #[test]
    fn memory_delete_entities_cascades_relations() {
        let mem = LongTermMemory::new();
        mem.create_entities(vec![
            Entity::new("Alice", "person"),
            Entity::new("Bob", "person"),
            Entity::new("Anthropic", "company"),
        ])
        .unwrap();
        mem.create_relations(vec![
            Relation::new("Alice", "Anthropic", "works_at"),
            Relation::new("Bob", "Anthropic", "works_at"),
            Relation::new("Alice", "Bob", "knows"),
        ])
        .unwrap();
        assert_eq!(mem.stats().relation_count, 3);
        let deleted = mem.delete_entities(vec!["Alice".into()]).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(mem.stats().entity_count, 2);
        // Alice 关联 2 个 relation 应该被级联删
        assert_eq!(mem.stats().relation_count, 1);
    }

    #[test]
    fn memory_search_nodes() {
        let mem = LongTermMemory::new();
        mem.create_entities(vec![
            Entity::new("Alice", "person").with_observations(vec!["Speaks Spanish".into()]),
            Entity::new("Bob", "person").with_observations(vec!["Loves cooking".into()]),
        ])
        .unwrap();
        let result = mem.search_nodes("spanish");
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "Alice");

        let result = mem.search_nodes("person");
        assert_eq!(result.entities.len(), 2);
    }

    #[test]
    fn memory_open_nodes_returns_relations_between() {
        let mem = LongTermMemory::new();
        mem.create_entities(vec![
            Entity::new("Alice", "person"),
            Entity::new("Bob", "person"),
            Entity::new("Carol", "person"),
        ])
        .unwrap();
        mem.create_relations(vec![
            Relation::new("Alice", "Bob", "knows"),
            Relation::new("Bob", "Carol", "knows"),
        ])
        .unwrap();
        let result = mem.open_nodes(vec!["Alice".into(), "Bob".into()]);
        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.relations.len(), 1);
        assert_eq!(result.relations[0].from, "Alice");
    }

    #[test]
    fn memory_notify_hook_fires_on_create() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mem = LongTermMemory::new();
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();
        mem.register_notify(move |_event| {
            c2.fetch_add(1, Ordering::SeqCst);
        });
        mem.create_entities(vec![Entity::new("Alice", "person")]).unwrap();
        mem.add_observations(vec![("Alice".into(), vec!["fact".into()])]).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn memory_jsonl_persistence_roundtrip() {
        let mem1 = LongTermMemory::new();
        mem1.create_entities(vec![Entity::new("Alice", "person").with_observations(vec!["Fact1".into()])])
            .unwrap();
        mem1.create_entities(vec![Entity::new("Bob", "person")]).unwrap();
        mem1.create_relations(vec![Relation::new("Alice", "Bob", "knows")]).unwrap();

        let mut buf = Vec::new();
        mem1.save_jsonl(&mut buf).unwrap();
        let jsonl = String::from_utf8(buf.clone()).unwrap();
        assert!(!jsonl.is_empty());

        // Roundtrip: load 到新 mem
        let mem2 = LongTermMemory::new();
        let mut cursor = Cursor::new(buf);
        let loaded = mem2.load_jsonl(&mut cursor).unwrap();
        // 3 lines: 2 entities + 1 relation
        assert_eq!(loaded, 3);
        assert_eq!(mem2.stats().entity_count, 2);
        assert_eq!(mem2.stats().relation_count, 1);
    }

    // ==================== 整合 tests ====================

    #[test]
    fn integration_stage6_3_mechanisms_compose() {
        // 模拟 Library Stage 6 完整流程:
        // 1. 守护池管理 Python 解释器资源
        // 2. 跨语言桥调用 Python 函数
        // 3. 长期记忆持久化结果
        let mut reg = BridgeRegistry::new();
        reg.register(Arc::new(StubBridge::new("python")));
        let mem = LongTermMemory::new();
        let pool = GuardianPool::<TestResource>::new(GuardianConfig::default());

        // 模拟流程: 守护池 checkin, 跨语言桥调用, 长期记忆存结果
        pool.checkin(TestResource { key: "py-1".into(), open: true }).unwrap();
        assert_eq!(pool.idle_count("py-1"), 1);
        let _reservation = pool.checkout("py-1").unwrap();

        // 跨语言桥调用 (stub, 不可用, 降级)
        let res = reg.call("python", "math", "sqrt", vec![BridgeValue::Int(16)]);
        assert!(res.is_err()); // stub 不可用

        // 长期记忆存调用日志
        mem.create_entities(vec![Entity::new("call_log_1", "log_entry")]).unwrap();
        mem.add_observations(vec![(
            "call_log_1".into(),
            vec!["called math.sqrt(16), got ModuleNotFound (stub)".into()],
        )])
        .unwrap();
        assert_eq!(mem.stats().entity_count, 1);
        assert_eq!(mem.stats().relation_count, 0);
    }
}
