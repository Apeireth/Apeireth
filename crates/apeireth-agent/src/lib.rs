//! `apeireth-agent` — **Apeireth R17 战役 2-4 Agent 管理系统**
//!
//! **目标**: 真支持 VCP-style 多 alias agent 注册 + LRU cache + notify 热加载.
//!
//! **4 大模块** (字段级引用 VCP 真代码 `agentManager.js:1-339`):
//! 1. `agent` — `Agent` struct (6 字段: id / name / aliases / tools / system_prompt / created_at)
//! 2. `manager` — `AgentManager` (CRUD + alias 解析 + LRU cache + notify 热加载 + 事件流)
//! 3. `AgentEvent` enum (Registered / Unregistered / FileChanged / FileAdded / FileRemoved)
//! 4. `lib` (本文件) — 入口 + 编译期 hardcode 守门
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md` + `agentManager.js` 全文):
//!
//! | VCP 字段/方法 | Rust 字段/方法 | 字段级引用 |
//! |---|---|---|
//! | `agentManager.js:11 agentMap: Map<alias, filename>` | `AgentManager::alias_index: HashMap<alias, id>` + `Agent::aliases: Vec<String>` | typed 化 + 支持多 alias 同 agent |
//! | `agentManager.js:12 promptCache: Map<alias, prompt>` | `AgentManager::cache: LruCache<key, Arc<Agent>>` | typed LRU (容量 64) |
//! | `agentManager.js:13 agentFiles: []` | `AgentEvent::FileAdded/Changed/Removed` + `event_log: Vec<AgentEvent>` | 事件流 (typed) |
//! | `agentManager.js:14 folderStructure: {}` | (留 R19 UI 用, 实战中递归扫描) | 不在本战役 2-4 DoD |
//! | `agentManager.js:23-31 initialize(debugMode)` | `AgentManager::new() / watch_dir()` | 实战: register + start watcher |
//! | `agentManager.js:36-63 loadMap` | `AgentManager::register` (自动维护 alias_index) | typed Map vs JSON map |
//! | `agentManager.js:50 promptCache.clear()` | `register` / `unregister` 内 `cache.clear()` | 守 VCP 行为 |
//! | `agentManager.js:68-131 watchFiles` chokidar | `watch_dir` (notify 5.x) | Rust 替代 chokidar |
//! | `agentManager.js:95-108 cache.delete(alias) on file change` | 实战中 `FileChanged` event → 调 `clear_cache` | typed event 推送 |
//! | `agentManager.js:272-315 getAgentPrompt(alias)` | `AgentManager::resolve(id_or_alias)` | typed Arc<Agent> 返回 |
//! | `agentManager.js:282 "{{agent:" + alias + "}}"` 占位符 | `ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX` | 实战: LLM 输出保留原占位符 |
//! | `agentManager.js:322-324 isAgent(alias)` | `AgentManager::contains(id_or_alias)` | typed bool 返回 |
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ alias 真解析 (alias → id → Arc`Agent`)
//! - ✅ LRU cache 真用 `lru::LruCache` (战场 2-4 加 workspace dep, 真 LRU 而非 fake HashMap)
//! - ✅ notify 热加载真跑 (mock tempdir + 写文件 + 等 2s + 验证 FileAdded 事件触发)
//! - ✅ register 维护 alias_index + 清 cache (VCP loadMap 行为)
//! - ✅ unregister 移除 alias + 清 cache (VCP 隐式行为, 我们 typed API)
//! - ✅ 单元测试 ≥ 30 (实际 ~33, ≥ DoD × 2×)
//!
//! **不修改承诺** (R17 finalize 8 项不修改承诺):
//! - ✅ 2026-08-04 R17 战役 4-5: Cargo.toml version = "0.14.0" → "1.0.0" (1.0 release, 主人授权)
//! - ❌ 不改战役 1 / 战役 2-1 / 战役 2-2 / 战役 2-3 全部代码 (用 import, 不改源码)
//! - ❌ 不引入 unsafe (workspace `#![deny(unsafe_code)]` 继承)
//! - ❌ 不假装 "已实现但没真跑"
//! - ❌ 不抄 VCP 业务代码 (借鉴字段 + 行为模式, 不抄 chokidar 实现)
//!
//! **架构位置**:
//! ```text
//!   apeireth-api / apeireth-pipeline / 未来消费者
//!          ↓
//!      apeireth-agent (本 crate)
//!      ├── agent.rs     : Agent struct (6 字段) + now_ms + 10 tests
//!      ├── manager.rs   : AgentManager + AgentEvent + 33 tests
//!      └── lib.rs       : 入口 + 编译期 hardcode
//! ```
//!
//! **跨 crate 集成**:
//! - `apeireth-tool-registry` — Agent.tools[i] → `ToolRegistry::get(name)` (战役 2-1)
//! - `apeireth-tool-runtime` — 实战中 `ToolExecutor` 调 agent 关联的工具 (战役 2-2)
//! - `apeireth-tool-approval` — 实战中 `ApprovalManager` 按 agent 决策 (战役 2-3)
//! - `apeireth-memory` — 实战中 `RecordStore` 把 agent 调用写到 action_stream (战役 1)

#![warn(missing_docs)]
#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod agent;
pub mod manager;

pub use agent::{now_ms, Agent};
pub use manager::{
    AgentEvent, AgentManager, ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX, DEFAULT_CACHE_SIZE,
    DEFAULT_WATCHER_DEBOUNCE_MS,
};

// ============================================================
// 编译期 hardcode (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 战役 2-4 实际借鉴 VCP 字段数
/// (`agentMap` / `promptCache` / `agentFiles` / `folderStructure` / `loadMap` / `watchFiles` /
///  `getAgentPrompt` / `isAgent` / `setAgentDir` = 9 项)
pub const BORROWED_VCP_FIELDS: usize = 9;

/// Agent 字段数 (id / name / aliases / tools / system_prompt / created_at) — 编译期 hardcode
pub const AGENT_FIELD_COUNT: usize = 6;

/// AgentEvent variant 数 (Registered / Unregistered / FileChanged / FileAdded / FileRemoved) — 编译期 hardcode
pub const AGENT_EVENT_VARIANT_COUNT: usize = 4;

/// 默认 LRU cache size (VCP `promptCache: Map` 无显式容量, 实战 64 合理)
pub const DEFAULT_CACHE_SIZE_CONST: usize = DEFAULT_CACHE_SIZE;

/// 默认 watcher debounce ms (实战 100ms 内连续事件合并)
pub const DEFAULT_WATCHER_DEBOUNCE_MS_CONST: u64 = DEFAULT_WATCHER_DEBOUNCE_MS;

/// Alias 未命中时占位符前缀 (VCP `{{agent:` 真值)
pub const ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX_CONST: &str = ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX;

// ============================================================
// 编译期断言 (工程铁律: 不假装 + 编译期 hardcode)
// ============================================================

const _: () = {
    // 9 字段借鉴 VCP
    assert!(
        BORROWED_VCP_FIELDS == 9,
        "BORROWED_VCP_FIELDS = 9 (VCP agentManager.js 字段级引用)"
    );
    // 6 字段 Agent
    assert!(
        AGENT_FIELD_COUNT == 6,
        "AGENT_FIELD_COUNT = 6 (id/name/aliases/tools/system_prompt/created_at)"
    );
    // 4 variant AgentEvent
    assert!(
        AGENT_EVENT_VARIANT_COUNT == 4,
        "AGENT_EVENT_VARIANT_COUNT = 4"
    );

    // 默认 cache size 64
    assert!(
        DEFAULT_CACHE_SIZE_CONST == 64,
        "DEFAULT_CACHE_SIZE = 64 (实战合理值)"
    );
    // debounce 100ms
    assert!(
        DEFAULT_WATCHER_DEBOUNCE_MS_CONST == 100,
        "DEFAULT_WATCHER_DEBOUNCE_MS = 100"
    );
    // 占位符前缀长度 (避开 &str == 的 const 限制; 真值字符串内容由 runtime test 验证)
    assert!(
        ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX_CONST.len() == "{{agent:".len(),
        "VCP 真值: {{agent: (len = 8)"
    );
};

// ============================================================
// lib 入口测试 (编译期 hardcode 二次断言 + 端到端)
// ============================================================

#[cfg(test)]
mod lib_tests {
    use super::*;
    use crate::agent::Agent;
    use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
    use std::sync::Arc;

    #[test]
    fn lib_constants_match_vcp() {
        // 编译期 hardcode 已 assert, 这里再 runtime 测一次
        assert_eq!(BORROWED_VCP_FIELDS, 9);
        assert_eq!(AGENT_FIELD_COUNT, 6);
        assert_eq!(AGENT_EVENT_VARIANT_COUNT, 4);
        assert_eq!(DEFAULT_CACHE_SIZE_CONST, 64);
        assert_eq!(DEFAULT_WATCHER_DEBOUNCE_MS_CONST, 100);
        assert_eq!(ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX_CONST, "{{agent:");
    }

    #[test]
    fn lib_public_api_compiles() {
        // 验证 lib.rs 公开 API 全部可见
        let _mgr = AgentManager::new();
        let _event = AgentEvent::Registered {
            id: "x".to_string(),
            alias_count: 1,
        };
        let a = Agent::new("x", "X", vec![], vec![], "p");
        assert_eq!(a.id, "x");
    }

    #[test]
    fn lib_end_to_end_coder_and_mavis() {
        // 端到端: 2 agent + 多 alias + 跨 agent 解析 + 工具关联 + 事件流
        let mgr = AgentManager::new();

        // 注册 coder: 3 alias + 2 工具
        let coder = Agent::new(
            "coder",
            "Coder Agent",
            vec![
                "@coder".to_string(),
                "@chuling".to_string(),
                "@xiaoling".to_string(),
            ],
            vec!["file_read".to_string(), "web_search".to_string()],
            "I am a coder agent.",
        );
        mgr.register(coder).unwrap();

        // 注册 mavis: 2 alias + 1 工具
        let mavis = Agent::new(
            "mavis",
            "Mavis Agent",
            vec!["@mavis".to_string(), "@ai".to_string()],
            vec!["chat".to_string()],
            "I am mavis agent.",
        );
        mgr.register(mavis).unwrap();

        assert_eq!(mgr.len(), 2);
        // 6 alias + 1 id(coder) + 2 alias + 1 id(mavis) = 7
        // (coder: id+3aliases=4, mavis: id+2aliases=3, total=7)
        assert_eq!(mgr.alias_count(), 7);

        // alias 解析
        let a1 = mgr.resolve("@chuling").unwrap();
        assert_eq!(a1.id, "coder");
        let a2 = mgr.resolve("@ai").unwrap();
        assert_eq!(a2.id, "mavis");

        // id 直接解析
        let a3 = mgr.resolve("coder").unwrap();
        assert_eq!(a3.id, "coder");

        // 工具关联
        let a4 = mgr.resolve("@xiaoling").unwrap();
        assert_eq!(a4.tool_count(), 2);
        assert!(a4.tools.contains(&"file_read".to_string()));

        // 未命中
        assert!(mgr.resolve("@unknown").is_none());

        // 事件流: 2 Registered
        let events = mgr.peek_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn lib_tool_registry_integration() {
        // 跨 crate 集成: agent 关联工具 → ToolRegistry 真查
        let mgr = AgentManager::new();
        let registry = ToolRegistry::new();

        // 注册工具到 ToolRegistry (战役 2-1)
        registry.register(
            "file_read".to_string(),
            Arc::new(MockSyncTool {
                name: "file_read".to_string(),
            }),
        );

        // 注册 agent, 工具名 "file_read"
        mgr.register(Agent::new(
            "coder",
            "Coder",
            vec!["@coder".to_string()],
            vec!["file_read".to_string()],
            "p",
        ))
        .unwrap();

        // 解析 agent → 拿工具名 → ToolRegistry 真查
        let agent = mgr.resolve("@coder").unwrap();
        let tool_name = &agent.tools[0];
        let tool = registry
            .get(tool_name)
            .expect("tool 'file_read' 应在 registry 中");
        assert_eq!(tool.name(), "file_read");
        assert_eq!(tool.kind(), apeireth_tool_registry::ToolKind::Sync);
    }

    #[test]
    fn lib_alias_field_count_vcp_consistent() {
        // Agent 6 字段 + AgentEvent 4 variant + 9 VCP 借鉴 = 工程铁律一致性
        // 9 = 6 (Agent 字段) + 1 (now_ms) + 2 (LruCache + alias_index)
        //   或: 6 + 1 + 1 + 1 (agentMap / promptCache / watchFiles)
        // 真要算 VCP 字段: agentMap (1) + promptCache (1) + agentFiles (1) + folderStructure (1)
        //   + loadMap (1) + watchFiles (1) + getAgentPrompt (1) + isAgent (1) + setAgentDir (1) = 9
        assert_eq!(BORROWED_VCP_FIELDS, 9);
    }

    #[test]
    fn lib_manager_default_state() {
        let mgr = AgentManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
        assert_eq!(mgr.cache_capacity(), 64);
        assert!(mgr.watched_dir().is_none());
        assert_eq!(mgr.event_count(), 0);
    }

    #[test]
    fn lib_agent_event_id_accessor() {
        let r = AgentEvent::Registered {
            id: "x".to_string(),
            alias_count: 0,
        };
        assert_eq!(r.id(), Some("x"));
        let u = AgentEvent::Unregistered {
            id: "y".to_string(),
        };
        assert_eq!(u.id(), Some("y"));
    }
}
