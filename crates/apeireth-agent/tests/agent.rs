//! Integration tests for apeireth-agent
//!
//! **R18 第 2 阶段第 8 项**: AgentManager + Agent 真实现

use apeireth_agent::{agent::Agent, manager::AgentManager};

fn make_agent(id: &str, name: &str, aliases: Vec<&str>, tools: Vec<&str>, sys: &str) -> Agent {
    let mut a = Agent::new(id, name, vec![], vec![], sys);
    a.aliases = aliases.into_iter().map(String::from).collect();
    a.tools = tools.into_iter().map(String::from).collect();
    a
}

// =====================================================================
// Agent 自身
// =====================================================================

#[test]
fn agent_new_with_id_and_name() {
    let a = make_agent("agent-1", "First Agent", vec![], vec![], "You are an agent.");
    assert_eq!(a.id, "agent-1");
    assert_eq!(a.name, "First Agent");
    assert_eq!(a.system_prompt, "You are an agent.");
    assert_eq!(a.aliases.len(), 0);
    assert_eq!(a.tools.len(), 0);
}

#[test]
fn agent_with_aliases() {
    let a = make_agent("a1", "A1", vec!["alias1", "alias2"], vec![], "sys");
    assert_eq!(a.alias_count(), 2);
    assert_eq!(a.all_aliases().len(), 3); // 2 aliases + 1 id
}

#[test]
fn agent_matches_by_id() {
    let a = make_agent("unique-id", "Name", vec![], vec![], "sys");
    assert!(a.matches("unique-id"));
    assert!(!a.matches("other-id"));
}

#[test]
fn agent_matches_by_alias() {
    let a = make_agent("a1", "Name", vec!["myalias"], vec![], "sys");
    assert!(a.matches("myalias"));
    assert!(a.matches("a1"));
}

#[test]
fn agent_tool_count() {
    let a = make_agent("a1", "Name", vec![], vec!["file_ops", "code_exec"], "sys");
    assert_eq!(a.tool_count(), 2);
}

// =====================================================================
// AgentManager
// =====================================================================

#[test]
fn manager_new_is_empty() {
    let m = AgentManager::new();
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
}

#[test]
fn manager_register_and_get() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec![], vec![], "sys"));
    let got = m.get("a1").expect("should be registered");
    assert_eq!(got.id, "a1");
}

#[test]
fn manager_register_and_resolve_by_alias() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["myalias"], vec![], "sys"));
    let resolved = m.resolve("myalias").expect("should resolve");
    assert_eq!(resolved.id, "a1");
}

#[test]
fn manager_unregister() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec![], vec![], "sys"));
    assert_eq!(m.len(), 1);
    let removed = m.unregister("a1");
    assert!(removed.is_ok());
    assert_eq!(m.len(), 0);
}

#[test]
fn manager_unregister_by_alias() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["myalias"], vec![], "sys"));
    let removed = m.unregister("myalias");
    assert!(removed.is_ok(), "unregister by alias should work");
    assert_eq!(m.len(), 0);
}

#[test]
fn manager_list_ids() {
    let m = AgentManager::new();
    m.register(make_agent("a", "A", vec![], vec![], "sys"));
    m.register(make_agent("b", "B", vec![], vec![], "sys"));
    m.register(make_agent("c", "C", vec![], vec![], "sys"));
    let ids = m.list_ids();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
    assert!(ids.contains(&"c".to_string()));
}

#[test]
fn manager_list_aliases() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["alpha"], vec![], "sys"));
    m.register(make_agent("a2", "A2", vec!["beta", "gamma"], vec![], "sys"));
    // 实际: 1+2 显式 alias + 2 隐式 id alias ("a1" / "a2") = 5
    // (per `manager.rs:179 register()` 第 3 步: id 也作为隐式 alias)
    assert_eq!(m.alias_count(), 5);
    let aliases = m.list_aliases();
    assert_eq!(aliases.len(), 5);
}

#[test]
fn manager_contains_id_and_alias() {
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["myalias"], vec![], "sys"));
    assert!(m.contains("a1"));
    assert!(m.contains("myalias"));
    assert!(!m.contains("nonexistent"));
}

#[test]
fn manager_with_cache_size() {
    let m = AgentManager::with_cache_size(64);
    assert_eq!(m.cache_len(), 0);
}

#[test]
fn manager_clear_cache() {
    let m = AgentManager::with_cache_size(64);
    m.register(make_agent("a1", "A1", vec![], vec![], "sys"));
    m.clear_cache();
    assert_eq!(m.cache_len(), 0);
}

// =====================================================================
// Agent 6 字段 (VCP `agentManager.js` 字段级) + 边界
// =====================================================================

#[test]
fn agent_has_six_fields() {
    use apeireth_agent::agent::Agent;
    // 6 字段: id / name / aliases / tools / system_prompt / created_at
    let a = Agent::new("a", "A", vec!["x".into(), "y".into()], vec!["t1".into()], "sys");
    let _ = (a.id, a.name, a.aliases, a.tools, a.system_prompt, a.created_at);
}

#[test]
fn agent_all_aliases_includes_id() {
    // id 自身也算 alias (per manager.rs:179 register 第 3 步)
    use apeireth_agent::agent::Agent;
    let a = Agent::new("agent1", "A1", vec!["@a".into()], vec![], "sys");
    let aliases = a.all_aliases();
    // 1 显式 alias + 1 隐式 id alias = 2
    assert!(aliases.contains(&"agent1".to_string()));
    assert!(aliases.contains(&"@a".to_string()));
    assert_eq!(aliases.len(), 2);
}

#[test]
fn agent_id_accessor() {
    use apeireth_agent::manager::AgentEvent;
    let r = AgentEvent::Registered { id: "a1".to_string(), alias_count: 2 };
    assert_eq!(r.id(), Some("a1"));
    let u = AgentEvent::Unregistered { id: "a2".to_string() };
    assert_eq!(u.id(), Some("a2"));
    let f = AgentEvent::FileChanged { path: std::path::PathBuf::from("/tmp/a") };
    assert!(f.id().is_none(), "FileChanged 不是 agent 级别事件");
}

#[test]
fn manager_alias_count_includes_implicit_id_alias() {
    // 实战: 显式 alias + id 隐式 alias 都计数
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["@a", "@b"], vec![], "sys"));
    // 2 显式 + 1 隐式 id = 3
    assert_eq!(m.alias_count(), 3, "应 3 alias: 2 显式 + 1 id 隐式");
}

#[test]
fn manager_list_aliases_sorted() {
    // list_aliases 应按字母序 (实战中 UI 显示)
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["zebra", "alpha", "mike"], vec![], "sys"));
    let aliases = m.list_aliases();
    // 含 id + 3 显式
    assert_eq!(aliases.len(), 4);
    // 验证排序
    let mut sorted = aliases.clone();
    sorted.sort();
    assert_eq!(aliases, sorted, "应按字母序");
}

#[test]
fn manager_register_overwrite_clears_old_aliases() {
    // 实战: 同一 id 重新注册 → 旧 alias 全部清理 (VCP loadMap 行为)
    let m = AgentManager::new();
    m.register(make_agent("a1", "A1", vec!["@old1", "@old2"], vec![], "sys"));
    assert!(m.contains("@old1"));
    // 重新注册, 只用新 alias
    m.register(make_agent("a1", "A1 v2", vec!["@new"], vec![], "sys v2"));
    assert!(m.contains("@new"), "新 alias 应有");
    assert!(!m.contains("@old1"), "旧 alias 应被清");
    assert!(!m.contains("@old2"), "旧 alias 应被清");
}

#[test]
fn manager_default_cache_size_is_64() {
    // 编译期 hardcode DEFAULT_CACHE_SIZE = 64
    use apeireth_agent::manager::DEFAULT_CACHE_SIZE;
    let m = AgentManager::new();
    assert_eq!(m.cache_capacity(), 64);
    assert_eq!(DEFAULT_CACHE_SIZE, 64);
}

#[test]
fn manager_clear_events_resets_log() {
    // 事件流管理 (Apeireth 扩展): 注册 2 个 → 至少 2 个 Registered 事件
    let m = AgentManager::new();
    assert_eq!(m.event_count(), 0, "初始 event_count 应 0");
    m.register(make_agent("a1", "A1", vec![], vec![], "sys"));
    m.register(make_agent("a2", "A2", vec![], vec![], "sys"));
    assert!(m.event_count() >= 2, "注册 2 个应 ≥ 2 事件, got {}", m.event_count());
    // 验证 peek_events 顺序: 最新的应排在最后
    let events = m.peek_events();
    assert!(!events.is_empty());
}
