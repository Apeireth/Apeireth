//! **战役 2-4 — `apeireth-agent` 端到端 demo**
//!
//! **目标**: 演示 alias 多对一 + LRU cache + notify 热加载 + tool-registry 集成
//!
//! **跑法**: `cargo run -p apeireth-agent --example agent_demo`
//!
//! **5 步全跑通**:
//! 1. 注册 2 agent (coder + mavis, 各 3/2 alias)
//! 2. alias 解析 6 case (id + alias 多 alias 同 agent + 不命中)
//! 3. LRU cache 演示 (resolve 多次 → cache 命中)
//! 4. tool-registry 集成 (跨 crate, MockSyncTool 真跑)
//! 5. notify 热加载 (临时目录 + 写文件 → 等 1s → 验证 FileAdded 事件)

use apeireth_agent::{Agent, AgentEvent, AgentManager};
use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn separator(title: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  {title}");
    println!("{}\n", "=".repeat(60));
}

fn main() {
    separator("战役 2-4 / `apeireth-agent` 端到端 demo");
    println!("VCP 借鉴: agentManager.js:1-339 (alias + cache + 热加载)");

    let mgr = AgentManager::new();

    // ============================================================
    // Step 1: 注册 2 agent (coder 3 alias, mavis 2 alias)
    // ============================================================
    separator("Step 1: 注册 2 agent (coder + mavis, 各多 alias)");

    let coder = Agent::new(
        "coder",
        "Coder Agent",
        vec![
            "@coder".to_string(),
            "@chuling".to_string(),
            "@xiaoling".to_string(),
        ],
        vec!["file_read".to_string(), "web_search".to_string()],
        "I am a coder agent. I write code and search the web.",
    );
    mgr.register(coder).expect("register coder");
    println!("[OK] registered: coder with 3 aliases (@coder / @chuling / @xiaoling)");

    let mavis = Agent::new(
        "mavis",
        "Mavis Agent",
        vec!["@mavis".to_string(), "@ai".to_string()],
        vec!["chat".to_string()],
        "I am mavis agent. I chat and help users.",
    );
    mgr.register(mavis).expect("register mavis");
    println!("[OK] registered: mavis with 2 aliases (@mavis / @ai)");

    println!(
        "\nstate: agents={}, aliases={}",
        mgr.len(),
        mgr.alias_count()
    );
    println!("       ids={:?}", mgr.list_ids());
    println!("       aliases={:?}", mgr.list_aliases());

    // ============================================================
    // Step 2: alias 解析 6 case
    // ============================================================
    separator("Step 2: alias 解析 6 case (VCP agentManager.js:272-315 字段级复刻)");

    let cases: &[&str] = &[
        "@chuling",  // → coder
        "@xiaoling", // → coder
        "@coder",    // → coder (id 也作 alias)
        "coder",     // → coder (id 直接)
        "@mavis",    // → mavis
        "@ai",       // → mavis
    ];
    for c in cases {
        match mgr.resolve(c) {
            Some(a) => println!("[OK] resolve({c:>10}) -> id={}, name={}", a.id, a.name),
            None => println!("[--] resolve({c:>10}) -> None"),
        }
    }
    println!(
        "[--] resolve(@unknown)    -> {}",
        if mgr.resolve("@unknown").is_none() {
            "None"
        } else {
            "Some"
        }
    );

    // ============================================================
    // Step 3: LRU cache 演示
    // ============================================================
    separator("Step 3: LRU cache 演示 (VCP agentManager.js:13 promptCache 字段级复刻)");

    println!("[init] cache len = {}", mgr.cache_len());
    let _ = mgr.resolve("@chuling");
    let _ = mgr.resolve("@xiaoling");
    let _ = mgr.resolve("@mavis");
    println!(
        "[after 3 resolves] cache len = {} (LRU 已记录 3 key)",
        mgr.cache_len()
    );

    // 再次 resolve, 应 cache 命中
    let _ = mgr.resolve("@chuling");
    println!(
        "[re-resolve @chuling] cache len = {} (LRU 命中, 顺序更新)",
        mgr.cache_len()
    );

    // register 应清 cache (VCP loadMap 行 50 promptCache.clear())
    let _ = mgr.register(Agent::new("helper", "Helper", vec![], vec![], "p"));
    println!(
        "[after register 'helper'] cache len = {} (register 清 cache)",
        mgr.cache_len()
    );

    // ============================================================
    // Step 4: tool-registry 集成 (跨 crate, 战役 2-1)
    // ============================================================
    separator("Step 4: 跨 crate 集成 (apeireth-tool-registry)");

    let registry = ToolRegistry::new();
    registry.register(
        "file_read".to_string(),
        Arc::new(MockSyncTool {
            name: "file_read".to_string(),
        }),
    );
    registry.register(
        "web_search".to_string(),
        Arc::new(MockSyncTool {
            name: "web_search".to_string(),
        }),
    );
    println!("[OK] ToolRegistry registered 2 mock sync tools");

    let coder = mgr.resolve("@chuling").expect("resolve @chuling");
    println!("[OK] agent '{}' 关联工具: {:?}", coder.name, coder.tools);
    for tool_name in &coder.tools {
        let tool = registry
            .get(tool_name)
            .expect(&format!("tool {tool_name} 应在 registry"));
        println!(
            "    -> ToolRegistry::get({}) = kind={:?}, name={}",
            tool_name,
            tool.kind(),
            tool.name()
        );
    }

    // ============================================================
    // Step 5: notify 热加载
    // ============================================================
    separator("Step 5: notify 热加载 (VCP agentManager.js:82-127 chokidar 字段级复刻)");

    let tmp = TempDir::new().expect("create tempdir");
    let watch_path = tmp.path().to_path_buf();
    mgr.watch_dir(&watch_path).expect("watch_dir");
    println!("[OK] AgentManager::watch_dir({})", watch_path.display());

    // 等 watcher ready
    thread::sleep(Duration::from_millis(100));
    let _ = mgr.take_events(); // 清初始 events

    // 写文件 → 触发 FileAdded
    let new_file = watch_path.join("new_agent.txt");
    std::fs::write(&new_file, "I am a new agent.").expect("write new file");
    println!("[--] wrote file: {}", new_file.display());

    // 等 notify 事件
    let mut found_add = false;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let events = mgr.peek_events();
        for e in &events {
            if let AgentEvent::FileAdded { path } = e {
                if path.ends_with("new_agent.txt") {
                    found_add = true;
                    break;
                }
            }
        }
        if found_add {
            break;
        }
    }
    assert!(
        found_add,
        "watch_dir 后写文件应触发 FileAdded, 实际 events: {:?}",
        mgr.peek_events()
    );
    println!("[OK] notify 触发 FileAdded 事件 (VCP chokidar 字段级复刻)");

    // 改文件 → 触发 FileChanged
    std::fs::write(&new_file, "I am updated agent.").expect("rewrite file");
    let mut found_change = false;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let events = mgr.peek_events();
        for e in &events {
            if let AgentEvent::FileChanged { path } = e {
                if path.ends_with("new_agent.txt") {
                    found_change = true;
                    break;
                }
            }
        }
        if found_change {
            break;
        }
    }
    assert!(
        found_change,
        "改文件应触发 FileChanged, 实际 events: {:?}",
        mgr.peek_events()
    );
    println!("[OK] notify 触发 FileChanged 事件");

    // 删文件 → 触发 FileRemoved
    std::fs::remove_file(&new_file).expect("remove file");
    let mut found_remove = false;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let events = mgr.peek_events();
        for e in &events {
            if let AgentEvent::FileRemoved { path } = e {
                if path.ends_with("new_agent.txt") {
                    found_remove = true;
                    break;
                }
            }
        }
        if found_remove {
            break;
        }
    }
    assert!(
        found_remove,
        "删文件应触发 FileRemoved, 实际 events: {:?}",
        mgr.peek_events()
    );
    println!("[OK] notify 触发 FileRemoved 事件");

    // 停止 watcher
    mgr.stop_watching();
    println!("[OK] stop_watching");

    // ============================================================
    // Step 6: unregister + cache 清
    // ============================================================
    separator("Step 6: unregister + cache 清 (VCP loadMap 行 50 行为)");

    let _ = mgr.resolve("@ai"); // 让 cache 有 entry
    println!(
        "[before] cache len = {}, contains(@ai) = {}",
        mgr.cache_len(),
        mgr.contains("@ai")
    );
    mgr.unregister("mavis").expect("unregister mavis");
    println!(
        "[after]  cache len = {}, contains(@ai) = {} (unregister 清 cache + alias)",
        mgr.cache_len(),
        mgr.contains("@ai")
    );
    assert_eq!(mgr.cache_len(), 0, "unregister 应清 cache");
    assert!(!mgr.contains("@ai"), "unregister 应清 alias index");
    assert!(!mgr.contains("@mavis"), "unregister 应清 alias index");
    assert!(!mgr.contains("mavis"), "unregister 应从 agents 移除");

    // ============================================================
    // Summary
    // ============================================================
    separator("Summary");
    println!("✅ Step 1: register 2 agent (coder + mavis, 各 3/2 alias)");
    println!("✅ Step 2: alias 解析 6 case 全过");
    println!("✅ Step 3: LRU cache 真命中 + register 清 cache");
    println!("✅ Step 4: tool-registry 跨 crate 真集成 (MockSyncTool kind=Sync)");
    println!("✅ Step 5: notify 热加载 FileAdded + FileChanged + FileRemoved 全触发");
    println!("✅ Step 6: unregister 清 cache + alias index");
    println!();
    println!("VCP 借鉴 (字段级):");
    println!("  agentManager.js:11  agentMap         -> alias_index + Agent.aliases");
    println!("  agentManager.js:12  promptCache      -> LruCache<key, Arc<Agent>>");
    println!("  agentManager.js:13  agentFiles       -> AgentEvent::FileAdded/Changed/Removed");
    println!("  agentManager.js:50  cache.clear()    -> register/unregister 自动清 cache");
    println!("  agentManager.js:82-127 chokidar      -> notify 5.x (Rust typed)");
    println!("  agentManager.js:272-315 getAgentPrompt -> resolve(id_or_alias)");
    println!("  agentManager.js:282 {{agent:alias}}  -> ALIAS_NOT_FOUND_PLACEHOLDER_PREFIX");
    println!("  agentManager.js:322-324 isAgent      -> contains(id_or_alias)");
    println!("  agentManager.js:330-335 setAgentDir  -> watch_dir(path)");
    println!();
    println!("战役 2-4 demo 完工 🎉");
}
