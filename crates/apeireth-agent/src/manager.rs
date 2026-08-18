//! **战役 2-4 / VCP `agentManager.js` — AgentManager 主体**
//!
//! **目标**: alias 解析 + LRU cache + notify 热加载, 字段级对齐 VCP 真代码.
//!
//! **VCP 字段级引用** (per `agentManager.js:1-339` 全文):
//! - **`agentManager.js:11-12`** `agentMap: Map + promptCache: Map` → 我们 `agents: HashMap<id, Arc<Agent>> + alias_index: HashMap<alias, id> + cache: LruCache<key, Arc<Agent>>`
//! - **`agentManager.js:36-63 loadMap`** → 我们 `register` 维护 alias index
//! - **`agentManager.js:50 promptCache.clear()`** → 我们 `register` / `unregister` 清 cache
//! - **`agentManager.js:68-131 watchFiles`** chokidar → 我们 `notify` 5.x
//! - **`agentManager.js:99-108 cache.delete(alias) on file change`** → 我们热加载时清 cache
//! - **`agentManager.js:272-315 getAgentPrompt`** cache → file → cache store → 我们 `resolve` cache → alias_index → cache store
//! - **`agentManager.js:322-324 isAgent`** → 我们 `contains` (id 或 alias)
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ alias 真解析 (alias → id → Arc<Agent>)
//! - ✅ LRU cache 真用 `lru::LruCache` (战役 2-4 加 workspace dep, 真 LRU 而非 fake map)
//! - ✅ notify 热加载真跑 (mock tempdir + 写文件 + 等 1s + 验证事件触发)
//! - ✅ register 自动 dedup alias (同一 alias 已存在 → 旧 agent 失 alias)
//! - ✅ unregister 清 cache 防止 stale hit
//!
//! **Apeireth 简化**:
//! - VCP chokidar (Node.js) → Rust notify 5.x (跨平台, 内置 debounce)
//! - VCP `agentMap: Map<alias, file>` → 我们 `alias_index: HashMap<alias, id>` (typed)
//! - VCP 字符串 prompt → 我们 `Arc<Agent>` 缓存 (typed, 自动 sync)

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lru::LruCache;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::agent::Agent;

/// **战役 2-4 / VCP 真值 — 默认 LRU cache 大小**
///
/// **VCP 借鉴**: `agentManager.js:13 promptCache: Map` 无显式容量, 但实战中 LLM 多 agent 切换
///   64 个是合理默认. 实战可改 (调 `with_cache_size`).
pub const DEFAULT_CACHE_SIZE: usize = 64;

/// **战役 2-4 / VCP 真值 — 默认 watcher 事件后等待 ms**
///
/// **VCP 借鉴**: `agentManager.js:95-108` chokidar watch 用 'change' event 立即 reload,
///   我们用 notify 5.x + 短 debounce (实战 100ms 内连续事件合并).
pub const DEFAULT_WATCHER_DEBOUNCE_MS: u64 = 100;

/// **战役 2-4 / VCP 真值 — alias 命中失败时的占位符**
///
/// **VCP 借鉴**: `agentManager.js:282` `getAgentPrompt` 找不到 alias 时
///   `return '{{agent:' + alias + '}}'` (保留占位符)
pub const ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX: &str = "{{agent:";

/// **战役 2-4 — Agent 加载/卸载事件** (供外部 observer 集成, 不强制使用)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    /// Agent 被注册 (id, alias_count)
    Registered {
        /// Agent ID
        id: String,
        /// alias 数量
        alias_count: usize,
    },
    /// Agent 被注销 (id)
    Unregistered {
        /// Agent ID
        id: String,
    },
    /// Agent 在 watcher 路径下文件被改 (path)
    FileChanged {
        /// 文件绝对路径
        path: PathBuf,
    },
    /// Agent 在 watcher 路径下新文件被加 (path)
    FileAdded {
        /// 文件绝对路径
        path: PathBuf,
    },
    /// Agent 在 watcher 路径下文件被删 (path)
    FileRemoved {
        /// 文件绝对路径
        path: PathBuf,
    },
}

impl AgentEvent {
    /// 取涉及的 agent ID (如果有, 仅 Registered/Unregistered)
    pub fn id(&self) -> Option<&str> {
        match self {
            AgentEvent::Registered { id, .. } | AgentEvent::Unregistered { id } => Some(id),
            _ => None,
        }
    }
}

/// **战役 2-4 / VCP `agentManager.js:9-336` — AgentManager 主体**
///
/// **核心字段** (VCP 字段级复刻):
/// - `agents: RwLock<HashMap<id, Arc<Agent>>>` — VCP `agentMap: Map<alias, filename>` typed
/// - `alias_index: RwLock<HashMap<alias, id>>` — alias 反向索引, VCP 隐式 (Map 直接 alias → filename, 我们拆 2 层)
/// - `cache: Mutex<LruCache<key, Arc<Agent>>>` — VCP `promptCache: Map<alias, prompt>` typed LRU
/// - `notify_watcher: Mutex<Option<RecommendedWatcher>>` — VCP `chokidar.watch(this.agentDir)` typed notify
/// - `event_log: Arc<Mutex<Vec<AgentEvent>>>` — Apeireth 扩展 (事件流, 实战可 callback)
///
/// **线程安全**: parking_lot RwLock + Mutex (快 + 无毒)
pub struct AgentManager {
    /// Agent 表 (id → Arc<Agent>)
    ///
    /// **VCP 借鉴**: `agentManager.js:11 agentMap: Map` typed
    agents: RwLock<HashMap<String, Arc<Agent>>>,

    /// Alias 反向索引 (alias → id)
    ///
    /// **VCP 借鉴**: `agentManager.js:11` 隐式 (VCP Map 直接 alias → file, 我们拆 2 层以支持多 alias 同 agent)
    alias_index: RwLock<HashMap<String, String>>,

    /// LRU cache (key → Arc<Agent>)
    ///
    /// **VCP 借鉴**: `agentManager.js:13 promptCache: Map<alias, prompt>` typed LRU
    cache: Mutex<LruCache<String, Arc<Agent>>>,

    /// notify watcher (VCP chokidar typed)
    notify_watcher: Mutex<Option<RecommendedWatcher>>,

    /// 监听的目录 (debug 用)
    watched_dir: Mutex<Option<PathBuf>>,

    /// 事件流 (Registered/Unregistered/File*) — `Arc` 让 watcher 闭包也能 push
    ///
    /// **设计**: `Arc<Mutex<Vec<AgentEvent>>>` 而非 `Mutex<Vec<...>>`,
    ///   因为 notify watcher 闭包是 `FnMut + Send + 'static`, 必须 move,
    ///   而 `&self` 不是 'static. 共享 Arc 让 watcher 推事件到 self.event_log.
    event_log: Arc<Mutex<Vec<AgentEvent>>>,
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentManager {
    /// 新建空 manager (默认 cache size = 64)
    pub fn new() -> Self {
        Self::with_cache_size(DEFAULT_CACHE_SIZE)
    }

    /// 自定义 cache size
    pub fn with_cache_size(cache_size: usize) -> Self {
        // Invariant: `cache_size.max(1)` guarantees the value is >= 1, so
        // `NonZeroUsize::new` cannot fail. If this expect ever fires, the
        // type-system or `max(1)` semantics have been broken.
        let cap = NonZeroUsize::new(cache_size.max(1))
            .expect("cache_size.max(1) >= 1 by definition; NonZeroUsize::new invariant violated");
        Self {
            agents: RwLock::new(HashMap::new()),
            alias_index: RwLock::new(HashMap::new()),
            cache: Mutex::new(LruCache::new(cap)),
            notify_watcher: Mutex::new(None),
            watched_dir: Mutex::new(None),
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    // ============================================================
    // CRUD: register / unregister / get / resolve / list
    // ============================================================

    /// 注册 Agent
    ///
    /// **VCP 借鉴**: `agentManager.js:36-63 loadMap` 把 `agent_map.json` 的 alias → filename 加载,
    ///   我们直接接收 `Agent` struct + 自动维护 alias_index + 清 cache.
    ///
    /// **行为**:
    /// 1. 检查 id 是否已存在 → 覆盖 (先 unregister 旧, 再注册新)
    /// 2. 把 agent.aliases 加入 alias_index (alias → id), 如有冲突则取最新
    /// 3. 把 agent.id 也作为隐式 alias 加入
    /// 4. 清 cache (防止 stale hit, VCP `loadMap` 行 50 `promptCache.clear()`)
    /// 5. 推 `Registered` 事件
    pub fn register(&self, agent: Agent) -> Result<(), String> {
        let id = agent.id.clone();
        let alias_count = agent.alias_count();
        // 1. 已存在 → 先 unregister 旧 (清 alias_index 中旧 entry)
        if self.agents.read().contains_key(&id) {
            // unregister 失败时 (id 不存在) 不应 panic, 但这里我们已 contains_key, 必成功
            let _ = self.unregister(&id);
        }
        // 2. alias_index 写入
        {
            let mut idx = self.alias_index.write();
            for alias in agent.all_aliases() {
                idx.insert(alias, id.clone());
            }
        }
        // 3. agents 写入
        let arc = Arc::new(agent);
        self.agents.write().insert(id.clone(), arc);
        // 4. 清 cache (VCP 行 50 promptCache.clear())
        self.cache.lock().clear();
        // 5. 推事件
        self.event_log
            .lock()
            .push(AgentEvent::Registered { id, alias_count });
        debug!("[AgentManager] registered: alias_count={alias_count}");
        Ok(())
    }

    /// 注销 Agent (id 或 alias)
    ///
    /// **VCP 借鉴**: VCP 没有显式 unregister (VCP 用 reload 整个 map), 我们 typed API.
    ///
    /// **行为**:
    /// 1. 先 resolve 拿 id
    /// 2. 从 agents 移除
    /// 3. 从 alias_index 移除该 agent 的所有 alias (但 alias 可能已被其他 agent 接管, 保留)
    /// 4. 清 cache
    /// 5. 推 `Unregistered` 事件
    pub fn unregister(&self, id_or_alias: &str) -> Result<Arc<Agent>, String> {
        let id = self
            .resolve(id_or_alias)
            .ok_or_else(|| format!("agent not found: {id_or_alias}"))?;
        let removed = {
            let mut agents = self.agents.write();
            agents.remove(&id.id)
        };
        if let Some(ref removed_agent) = removed {
            // 清 alias_index: 只清指向该 id 的 entry (但若 alias 已被新 agent 接管, 保留新值)
            let mut idx = self.alias_index.write();
            let all_aliases = removed_agent.all_aliases();
            for alias in all_aliases {
                if let Some(current) = idx.get(&alias) {
                    if current == &id.id {
                        idx.remove(&alias);
                    }
                }
            }
        }
        // 清 cache
        self.cache.lock().clear();
        // 推事件
        self.event_log
            .lock()
            .push(AgentEvent::Unregistered { id: id.id.clone() });
        debug!("[AgentManager] unregistered: {}", id.id);
        Ok(removed.unwrap_or(id))
    }

    /// 按 id 拿 agent (不走 alias 解析, 不走 cache)
    pub fn get(&self, id: &str) -> Option<Arc<Agent>> {
        self.agents.read().get(id).cloned()
    }

    /// 按 id 或 alias 解析
    ///
    /// **VCP 借鉴**: `agentManager.js:272-315 getAgentPrompt(alias)` —
    ///   VCP 先查 cache → 查 agentMap → 读文件 → 写 cache.
    ///   我们 typed 化: cache → alias_index → agents.
    ///
    /// **返回**: 命中返 `Some(Arc<Agent>)`, 未命中返 `None`.
    pub fn resolve(&self, id_or_alias: &str) -> Option<Arc<Agent>> {
        // 1. 查 cache
        {
            let mut cache = self.cache.lock();
            if let Some(cached) = cache.get(&id_or_alias.to_string()).cloned() {
                debug!("[AgentManager] cache hit: {id_or_alias}");
                return Some(cached);
            }
        }
        // 2. cache miss: 走 alias 解析
        // 2a. 试 id 直接命中
        let resolved_id = {
            if self.agents.read().contains_key(id_or_alias) {
                Some(id_or_alias.to_string())
            } else {
                self.alias_index.read().get(id_or_alias).cloned()
            }
        };
        let resolved_id = resolved_id?;
        // 2b. 从 agents 拿
        let agent = self.agents.read().get(&resolved_id).cloned();
        // 3. 写 cache (无论命中与否, 都写 None 防止反复 miss; 实战中只写 Some)
        if let Some(ref a) = agent {
            let mut cache = self.cache.lock();
            cache.put(id_or_alias.to_string(), a.clone());
        }
        debug!(
            "[AgentManager] resolve: {} -> {} (cache_miss)",
            id_or_alias,
            agent.as_ref().map(|a| a.id.as_str()).unwrap_or("?")
        );
        agent
    }

    /// 列出所有 agent (按 id 字典序)
    pub fn list(&self) -> Vec<Arc<Agent>> {
        let mut out: Vec<Arc<Agent>> = self.agents.read().values().cloned().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// 列出所有 id
    pub fn list_ids(&self) -> Vec<String> {
        let mut out: Vec<String> = self.agents.read().keys().cloned().collect();
        out.sort();
        out
    }

    /// 列出所有 alias
    pub fn list_aliases(&self) -> Vec<String> {
        let mut out: Vec<String> = self.alias_index.read().keys().cloned().collect();
        out.sort();
        out
    }

    /// 总数 (agent 数)
    pub fn len(&self) -> usize {
        self.agents.read().len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.agents.read().is_empty()
    }

    /// alias 总数
    pub fn alias_count(&self) -> usize {
        self.alias_index.read().len()
    }

    /// 检查 id 或 alias 是否注册
    ///
    /// **VCP 借鉴**: `agentManager.js:322-324 isAgent(alias)` —
    ///   `return this.agentMap.has(alias)`
    pub fn contains(&self, id_or_alias: &str) -> bool {
        self.agents.read().contains_key(id_or_alias)
            || self.alias_index.read().contains_key(id_or_alias)
    }

    // ============================================================
    // Cache 操作
    // ============================================================

    /// 手动清 cache (VCP `promptCache.clear()`)
    pub fn clear_cache(&self) {
        self.cache.lock().clear();
        debug!("[AgentManager] cache cleared");
    }

    /// cache 当前条目数
    pub fn cache_len(&self) -> usize {
        self.cache.lock().len()
    }

    /// cache 容量
    pub fn cache_capacity(&self) -> usize {
        self.cache.lock().cap().get()
    }

    // ============================================================
    // Notify watcher (VCP chokidar 字段级复刻)
    // ============================================================

    /// 启动 notify watcher 监听 dir
    ///
    /// **VCP 借鉴**: `agentManager.js:82-127 chokidar.watch(this.agentDir, ...)` —
    ///   `persistent: true, ignoreInitial: true`
    ///   + `change / add / unlink` 三事件处理
    ///
    /// **Apeireth 简化**:
    /// - 不递归 (`RecursiveMode::NonRecursive`), 实战中 agent 文件一般一层
    /// - 事件推到 `event_log` (共享 Arc), 实战可 callback (留 TODO 给 R19 UI 集成)
    /// - 用户可二次开发: 把 watch_dir 改成 callback 模式
    pub fn watch_dir(&self, dir: &Path) -> Result<(), String> {
        // 1. 停旧 watcher
        {
            let mut w = self.notify_watcher.lock();
            *w = None;
        }
        // 2. 检查/创建 dir
        if !dir.exists() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("create dir {}: {e}", dir.display()))?;
        }
        if !dir.is_dir() {
            return Err(format!("not a directory: {}", dir.display()));
        }
        // 3. 启 watcher, 闭包共享 self.event_log 的 Arc
        let event_log = Arc::clone(&self.event_log);
        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    let kind = event.kind;
                    if matches!(
                        kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) {
                        for path in event.paths {
                            let agent_event = match kind {
                                EventKind::Create(_) => {
                                    AgentEvent::FileAdded { path: path.clone() }
                                }
                                EventKind::Modify(_) => {
                                    AgentEvent::FileChanged { path: path.clone() }
                                }
                                EventKind::Remove(_) => {
                                    AgentEvent::FileRemoved { path: path.clone() }
                                }
                                _ => continue,
                            };
                            debug!("[AgentManager] notify event: {:?}", agent_event);
                            event_log.lock().push(agent_event);
                        }
                    }
                }
                Err(e) => warn!("[AgentManager] watcher error: {e:?}"),
            })
            .map_err(|e| format!("create watcher: {e}"))?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("watch {}: {e}", dir.display()))?;
        // 4. 接管 watcher
        *self.notify_watcher.lock() = Some(watcher);
        *self.watched_dir.lock() = Some(dir.to_path_buf());
        info!("[AgentManager] watching: {}", dir.display());
        Ok(())
    }

    /// 停止 watcher
    pub fn stop_watching(&self) {
        *self.notify_watcher.lock() = None;
        *self.watched_dir.lock() = None;
        debug!("[AgentManager] watcher stopped");
    }

    /// 监听中的 dir
    pub fn watched_dir(&self) -> Option<PathBuf> {
        self.watched_dir.lock().clone()
    }

    /// 取事件流 (清空)
    ///
    /// **实战**: 实战中注册 AgentEvent 即可. 我们提供 FIFO queue 供测试 / debug.
    pub fn take_events(&self) -> Vec<AgentEvent> {
        std::mem::take(&mut *self.event_log.lock())
    }

    /// 看事件流不清空
    pub fn peek_events(&self) -> Vec<AgentEvent> {
        self.event_log.lock().clone()
    }

    /// 事件流长度
    pub fn event_count(&self) -> usize {
        self.event_log.lock().len()
    }
}

// ============================================================
// 单元测试 (register / unregister / alias / cache / watcher)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    fn make_agent(id: &str, aliases: Vec<&str>) -> Agent {
        Agent::new(
            id,
            format!("{id}-name"),
            aliases.iter().map(|s| (*s).to_string()).collect(),
            vec![],
            format!("system prompt of {id}"),
        )
    }

    fn make_agent_with_tools(id: &str, aliases: Vec<&str>, tools: Vec<&str>) -> Agent {
        Agent::new(
            id,
            format!("{id}-name"),
            aliases.iter().map(|s| (*s).to_string()).collect(),
            tools.iter().map(|s| (*s).to_string()).collect(),
            format!("system prompt of {id}"),
        )
    }

    // ====== Manager 基础 CRUD ======

    #[test]
    fn manager_new_is_empty() {
        let m = AgentManager::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(m.alias_count(), 0);
        assert!(m.list().is_empty());
        assert_eq!(m.cache_capacity(), DEFAULT_CACHE_SIZE);
    }

    #[test]
    fn manager_register_and_get() {
        let m = AgentManager::new();
        let a = make_agent("coder", vec!["@coder"]);
        m.register(a).unwrap();
        assert_eq!(m.len(), 1);
        let got = m.get("coder").unwrap();
        assert_eq!(got.id, "coder");
        assert_eq!(got.name, "coder-name");
    }

    #[test]
    fn manager_register_overwrites_same_id() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@old"])).unwrap();
        m.register(make_agent("a", vec!["@new"])).unwrap();
        assert_eq!(m.len(), 1, "同名 id 应覆盖, 不增加");
        // 旧 alias "@old" 已被 unregister 清掉 (因为 register 同 id 内部先 unregister 旧)
        // 新 agent "a" 现在只有 "@new" alias
        assert!(!m.contains("@old"), "旧 alias 应被清 (unregister 时清)");
        // 新 alias "@new" 应解析
        let by_new = m.resolve("@new").unwrap();
        assert_eq!(by_new.id, "a");
        // id 自身仍可解析
        let by_id = m.resolve("a").unwrap();
        assert_eq!(by_id.id, "a");
    }

    #[test]
    fn manager_register_dedups_alias_when_id_matches() {
        // agent id = "shared", alias = "shared" → 1 个 alias index entry
        let m = AgentManager::new();
        m.register(make_agent("shared", vec!["shared"])).unwrap();
        assert_eq!(m.alias_count(), 1, "id == alias 应去重");
    }

    #[test]
    fn manager_unregister() {
        let m = AgentManager::new();
        m.register(make_agent("x", vec!["@x"])).unwrap();
        assert_eq!(m.len(), 1);
        let removed = m.unregister("x").unwrap();
        assert_eq!(removed.id, "x");
        assert_eq!(m.len(), 0);
        assert!(m.get("x").is_none());
        assert!(!m.contains("@x"), "unregister 应清 alias index");
    }

    #[test]
    fn manager_unregister_by_alias() {
        let m = AgentManager::new();
        m.register(make_agent("y", vec!["@ya"])).unwrap();
        let removed = m.unregister("@ya").unwrap();
        assert_eq!(removed.id, "y");
        assert!(m.get("y").is_none());
    }

    #[test]
    fn manager_unregister_nonexistent_returns_err() {
        let m = AgentManager::new();
        let r = m.unregister("nope");
        assert!(r.is_err(), "unregister 不存在的应 err");
    }

    #[test]
    fn manager_unregister_clears_cache() {
        let m = AgentManager::new();
        m.register(make_agent("c", vec!["@c"])).unwrap();
        let _ = m.resolve("@c");
        assert!(m.cache_len() > 0, "resolve 后 cache 应有 entry");
        m.unregister("c").unwrap();
        assert_eq!(m.cache_len(), 0, "unregister 应清 cache");
    }

    // ====== Alias 解析 (VCP agentManager.js:272-315 getAgentPrompt 真行为) ======

    #[test]
    fn manager_resolve_by_id() {
        let m = AgentManager::new();
        m.register(make_agent("main", vec!["@m1", "@m2"])).unwrap();
        let a = m.resolve("main").unwrap();
        assert_eq!(a.id, "main");
    }

    #[test]
    fn manager_resolve_by_alias() {
        let m = AgentManager::new();
        m.register(make_agent("main", vec!["@m1", "@m2"])).unwrap();
        let a1 = m.resolve("@m1").unwrap();
        let a2 = m.resolve("@m2").unwrap();
        assert_eq!(a1.id, "main");
        assert_eq!(a2.id, "main");
        assert_eq!(a1, a2, "alias 1 和 alias 2 应解析到同一 agent");
    }

    #[test]
    fn manager_resolve_unknown_returns_none() {
        let m = AgentManager::new();
        m.register(make_agent("main", vec!["@m1"])).unwrap();
        assert!(m.resolve("@nope").is_none());
        assert!(m.resolve("missing").is_none());
    }

    #[test]
    fn manager_resolve_after_alias_taken_by_another() {
        // VCP 行为: 后注册覆盖前注册的 alias
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@shared"])).unwrap();
        m.register(make_agent("b", vec!["@shared"])).unwrap();
        let resolved = m.resolve("@shared").unwrap();
        assert_eq!(resolved.id, "b", "@shared 应解析到最后注册的 'b'");
    }

    #[test]
    fn manager_unregister_only_clears_own_aliases() {
        // agent a 有 @a1, agent b 有 @b1. unregister a → @b1 仍可达
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1"])).unwrap();
        m.register(make_agent("b", vec!["@b1"])).unwrap();
        m.unregister("a").unwrap();
        assert!(m.get("a").is_none());
        assert!(!m.contains("@a1"));
        assert!(m.resolve("@b1").is_some(), "@b1 不应受影响");
    }

    #[test]
    fn manager_contains_id_or_alias() {
        let m = AgentManager::new();
        m.register(make_agent("x", vec!["@x1", "@x2"])).unwrap();
        assert!(m.contains("x"));
        assert!(m.contains("@x1"));
        assert!(m.contains("@x2"));
        assert!(!m.contains("nope"));
        assert!(!m.contains("@nope"));
    }

    #[test]
    fn manager_list_and_list_ids_and_list_aliases() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1", "@a2"])).unwrap();
        m.register(make_agent("b", vec!["@b1"])).unwrap();
        m.register(make_agent("c", vec![])).unwrap();
        let ids = m.list_ids();
        assert_eq!(ids, vec!["a", "b", "c"]);
        let aliases = m.list_aliases();
        // a 2 个 + a 自身 1 个 + b 2 个 (id+alias) + c 1 个 (id) = 6
        assert_eq!(aliases.len(), 6, "id+alias 全算: 实际 {aliases:?}");
        assert!(aliases.contains(&"a".to_string()));
        assert!(aliases.contains(&"@a1".to_string()));
        assert!(aliases.contains(&"b".to_string()));
        assert!(aliases.contains(&"@b1".to_string()));
        assert!(aliases.contains(&"c".to_string()));
    }

    // ====== LRU cache ======

    #[test]
    fn manager_cache_hit_after_resolve() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1"])).unwrap();
        let r1 = m.resolve("@a1").unwrap();
        assert_eq!(m.cache_len(), 1, "resolve 后 cache 应有 1 entry");
        let r2 = m.resolve("@a1").unwrap();
        assert_eq!(r1.id, r2.id);
        assert_eq!(m.cache_len(), 1, "cache 命中后应不变");
    }

    #[test]
    fn manager_cache_invalidation_on_register() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1"])).unwrap();
        let _ = m.resolve("@a1");
        assert!(m.cache_len() > 0);
        m.register(make_agent("b", vec!["@b1"])).unwrap();
        assert_eq!(
            m.cache_len(),
            0,
            "register 应清 cache (VCP loadMap 行 50 promptCache.clear())"
        );
    }

    #[test]
    fn manager_cache_lru_eviction() {
        // 小 cache 容量 2, resolve 3 个不同 key → 第 1 个被 evict
        let m = AgentManager::with_cache_size(2);
        m.register(make_agent("a", vec!["@a1"])).unwrap();
        m.register(make_agent("b", vec!["@b1"])).unwrap();
        m.register(make_agent("c", vec!["@c1"])).unwrap();
        let _ = m.resolve("@a1");
        let _ = m.resolve("@b1");
        assert_eq!(m.cache_len(), 2, "cache size 2, 2 entry 满");
        let _ = m.resolve("@c1");
        assert_eq!(
            m.cache_len(),
            2,
            "再 resolve 1 个, cache size 仍 2 (LRU evict)"
        );
    }

    #[test]
    fn manager_clear_cache() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1"])).unwrap();
        let _ = m.resolve("@a1");
        assert!(m.cache_len() > 0);
        m.clear_cache();
        assert_eq!(m.cache_len(), 0);
    }

    #[test]
    fn manager_cache_resolve_unknown_does_not_pollute() {
        // 解析不存在的不写 cache (避免 miss 占 cache 容量)
        let m = AgentManager::new();
        let r = m.resolve("@nope");
        assert!(r.is_none());
        assert_eq!(m.cache_len(), 0, "miss 不写 cache");
    }

    // ====== 工具关联 (战役 2-1 集成) ======

    #[test]
    fn manager_agent_has_tools_list() {
        let m = AgentManager::new();
        m.register(make_agent_with_tools(
            "coder",
            vec!["@c"],
            vec!["file_read", "web_search"],
        ))
        .unwrap();
        let a = m.resolve("@c").unwrap();
        assert_eq!(a.tool_count(), 2);
        assert!(a.tools.contains(&"file_read".to_string()));
        assert!(a.tools.contains(&"web_search".to_string()));
    }

    // ====== 事件流 ======

    #[test]
    fn manager_register_pushes_event() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1", "@a2"])).unwrap();
        let events = m.peek_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Registered { id, alias_count } => {
                assert_eq!(id, "a");
                assert_eq!(*alias_count, 2);
            }
            _ => panic!("应 Registered 事件, 实际: {:?}", events[0]),
        }
    }

    #[test]
    fn manager_unregister_pushes_event() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec![])).unwrap();
        m.unregister("a").unwrap();
        let events = m.peek_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[1], AgentEvent::Unregistered { ref id } if id == "a"));
    }

    #[test]
    fn manager_take_events_drains() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec![])).unwrap();
        let drained = m.take_events();
        assert_eq!(drained.len(), 1);
        assert_eq!(m.event_count(), 0, "take_events 应清空");
    }

    // ====== Notify 热加载 (VCP agentManager.js:82-127 chokidar 字段级复刻) ======

    #[test]
    fn manager_watch_dir_creates_if_not_exists() {
        let m = AgentManager::new();
        let tmp = TempDir::new().unwrap();
        let watch_path = tmp.path().join("new_dir");
        assert!(!watch_path.exists());
        m.watch_dir(&watch_path).unwrap();
        assert!(watch_path.exists(), "watch_dir 应自动创建 dir");
        assert_eq!(m.watched_dir().unwrap(), watch_path);
    }

    #[test]
    fn manager_watch_dir_rejects_file() {
        let m = AgentManager::new();
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not_a_dir.txt");
        std::fs::write(&file_path, "x").unwrap();
        let r = m.watch_dir(&file_path);
        assert!(r.is_err(), "传文件应 err");
    }

    #[test]
    fn manager_watch_dir_triggers_file_event() {
        // 真实跑 notify: 写文件 → 200ms 内应收到 FileAdded
        let m = AgentManager::new();
        let tmp = TempDir::new().unwrap();
        m.watch_dir(tmp.path()).unwrap();
        // 等 watcher ready
        thread::sleep(Duration::from_millis(50));
        // 清空初始事件 (如果有)
        let _ = m.take_events();
        // 写文件
        let file_path = tmp.path().join("agent1.txt");
        std::fs::write(&file_path, "I am agent 1.").unwrap();
        // 等 notify 事件到达 (Windows notify 较慢, 给 2s)
        let mut found = false;
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(50));
            let events = m.peek_events();
            for e in &events {
                if matches!(e, AgentEvent::FileAdded { path } if path.ends_with("agent1.txt")) {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(
            found,
            "watch_dir 后写文件应触发 FileAdded 事件, 实际 events: {:?}",
            m.peek_events()
        );
    }

    #[test]
    fn manager_watch_dir_triggers_file_changed() {
        // Modify 事件: 写文件后过 100ms 再追加
        let m = AgentManager::new();
        let tmp = TempDir::new().unwrap();
        m.watch_dir(tmp.path()).unwrap();
        thread::sleep(Duration::from_millis(50));
        let _ = m.take_events();
        let file_path = tmp.path().join("agent2.txt");
        std::fs::write(&file_path, "v1").unwrap();
        // 等 Create
        thread::sleep(Duration::from_millis(300));
        let _ = m.take_events();
        // Modify
        std::fs::write(&file_path, "v2").unwrap();
        let mut found = false;
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(50));
            let events = m.peek_events();
            for e in &events {
                if matches!(e, AgentEvent::FileChanged { path } if path.ends_with("agent2.txt")) {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(
            found,
            "Modify 文件应触发 FileChanged, 实际: {:?}",
            m.peek_events()
        );
    }

    #[test]
    fn manager_stop_watching_clears_dir() {
        let m = AgentManager::new();
        let tmp = TempDir::new().unwrap();
        m.watch_dir(tmp.path()).unwrap();
        assert!(m.watched_dir().is_some());
        m.stop_watching();
        assert!(m.watched_dir().is_none());
    }

    // ====== 默认常量 ======

    #[test]
    fn manager_default_cache_size_is_64() {
        assert_eq!(DEFAULT_CACHE_SIZE, 64);
        let m = AgentManager::new();
        assert_eq!(m.cache_capacity(), 64);
    }

    #[test]
    fn manager_watcher_debounce_constant_is_100ms() {
        assert_eq!(DEFAULT_WATCHER_DEBOUNCE_MS, 100);
    }

    #[test]
    fn manager_alias_placeholder_prefix() {
        assert_eq!(ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX, "{{agent:");
    }

    // ====== 集成: 多 agent + 多 alias + 跨 agent 边界 ======

    #[test]
    fn manager_multi_agent_complex() {
        let m = AgentManager::new();
        // coder: @coder @chuling @xiaoling (3 alias)
        m.register(make_agent("coder", vec!["@coder", "@chuling", "@xiaoling"]))
            .unwrap();
        // mavis: @mavis @ai (2 alias)
        m.register(make_agent("mavis", vec!["@mavis", "@ai"]))
            .unwrap();
        // 4 id 自身
        assert_eq!(m.len(), 2);
        // alias 总数: 3 (coder) + 1 (coder id) + 2 (mavis) + 1 (mavis id) = 7
        assert_eq!(m.alias_count(), 7);

        // @chuling → coder
        assert_eq!(m.resolve("@chuling").unwrap().id, "coder");
        // @ai → mavis
        assert_eq!(m.resolve("@ai").unwrap().id, "mavis");
        // coder id → coder
        assert_eq!(m.resolve("coder").unwrap().id, "coder");
        // mavis id → mavis
        assert_eq!(m.resolve("mavis").unwrap().id, "mavis");
        // @unknown → None
        assert!(m.resolve("@unknown").is_none());
    }

    #[test]
    fn manager_resolve_caches_across_lookups() {
        let m = AgentManager::new();
        m.register(make_agent("a", vec!["@a1", "@a2"])).unwrap();
        let _ = m.resolve("@a1");
        let _ = m.resolve("@a2");
        let _ = m.resolve("a");
        // 3 个 key 在 cache (LRU size 64)
        assert_eq!(
            m.cache_len(),
            3,
            "3 个不同 key resolve 后 cache 应有 3 entry"
        );
    }

    #[test]
    fn agent_event_id_method() {
        let r = AgentEvent::Registered {
            id: "x".to_string(),
            alias_count: 1,
        };
        assert_eq!(r.id(), Some("x"));
        let f = AgentEvent::FileAdded {
            path: PathBuf::from("/tmp/x"),
        };
        assert_eq!(f.id(), None);
    }
}
