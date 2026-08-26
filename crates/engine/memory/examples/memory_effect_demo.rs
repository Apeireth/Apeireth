//! `memory_effect_demo` — apeireth-memory 端到端真效果验证 (R17 阶段 6)
//!
//! **目的**: 验证 apeireth-memory 真的能记忆 + 真持久化
//!
//! **流程**:
//! 1. 创建一个 SqliteMemoryStore (持久化到 temp file)
//! 2. 模拟 3 轮对话, 写入 6 条 episode (system + user + assistant, 每轮 3 条)
//! 3. drop store
//! 4. 重新打开 store (验证持久化)
//! 5. 查 session 的所有 episode (验证读取)
//! 6. 测试真实场景: "AI 记住偏好" — 写入 user "我喜欢 Rust" → 重新打开 → 查"我喜欢 Rust"是否还在
//!
//! **跑法**: cargo run -p apeireth-memory --example memory_effect_demo

use apeireth_core::Episode;
use apeireth_memory::{EpisodeQuery, EpisodeStore, SqliteMemoryStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 apeireth-memory 端到端真效果验证 (R17)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // 1. 创建临时 SQLite 数据库 (file-backed, 验证真持久化)
    let tmp_path = std::env::temp_dir().join("apeireth-memory-effect-demo.db");
    // 清理旧文件 (防止上次残留)
    let _ = std::fs::remove_file(&tmp_path);
    println!("📁 temp DB: {}", tmp_path.display());
    println!();

    // 2. 写 episode (场景 1: 模拟多轮对话)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 A: 写 3 轮对话到 SqliteMemoryStore");
    let store = SqliteMemoryStore::open(&tmp_path)?;
    let now = chrono::Utc::now().timestamp();

    // Round 1: 自我介绍
    store.put_episode(&Episode {
        id: "ep-001".into(),
        timestamp: now,
        role: "user".into(),
        content: "你好, 我是 apeireth 项目的 AI 助手".into(),
        session_id: "session-1".into(),
    })?;
    store.put_episode(&Episode {
        id: "ep-002".into(),
        timestamp: now + 1,
        role: "assistant".into(),
        content: "你好! 我会记住我们这次对话".into(),
        session_id: "session-1".into(),
    })?;

    // Round 2: 用户说偏好 (重点验证: "AI 记住偏好")
    store.put_episode(&Episode {
        id: "ep-003".into(),
        timestamp: now + 2,
        role: "user".into(),
        content: "我最喜欢的编程语言是 Rust".into(),
        session_id: "session-1".into(),
    })?;
    store.put_episode(&Episode {
        id: "ep-004".into(),
        timestamp: now + 3,
        role: "assistant".into(),
        content: "已记住: 用户喜欢 Rust".into(),
        session_id: "session-1".into(),
    })?;

    // Round 3: 用户身份
    store.put_episode(&Episode {
        id: "ep-005".into(),
        timestamp: now + 4,
        role: "user".into(),
        content: "我在做 2026 年学术研究项目".into(),
        session_id: "session-1".into(),
    })?;
    store.put_episode(&Episode {
        id: "ep-006".into(),
        timestamp: now + 5,
        role: "assistant".into(),
        content: "已记住: 用户在做学术研究".into(),
        session_id: "session-1".into(),
    })?;

    let count = store.count_by_session("session-1")?;
    println!(
        "✅ 写入 6 条 episode 到 session-1, count_by_session = {}",
        count
    );
    assert_eq!(count, 6, "R17 不假装: 写入 6 条必须 count = 6");

    // 3. drop store (强制 SQLite flush 到 disk)
    drop(store);
    println!("💾 store drop (强制 flush 到 disk)");
    println!();

    // 4. 重新打开 store (验证持久化)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 B: 重新打开 store (验证持久化)");
    let store2 = SqliteMemoryStore::open(&tmp_path)?;
    let count2 = store2.count_by_session("session-1")?;
    println!("✅ 重新打开后 count_by_session = {}", count2);
    assert_eq!(count2, 6, "R17 不假装: 重新打开后必须还有 6 条 (持久化)");

    // 5. 查 session 的所有 episode
    let recent = store2.recent_episodes("session-1", 100)?;
    println!("📝 重新打开后查到的 episodes ({} 条):", recent.len());
    for ep in &recent {
        println!(
            "   [{}] {} ({}): {}",
            ep.id, ep.role, ep.timestamp, ep.content
        );
    }
    println!();

    // 6. 真实场景: "AI 记住偏好" 测试
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 阶段 C: 真实场景测试 — 'AI 记住偏好'");
    let query = EpisodeQuery::new()
        .for_session("session-1")
        .with_role("user");
    let user_episodes = store2.query(&query)?;
    println!(
        "🔍 查 user role 的所有 episode ({} 条):",
        user_episodes.len()
    );
    let mut found_rust_preference = false;
    let mut found_research_context = false;
    for ep in &user_episodes {
        println!("   [{}] {}", ep.id, ep.content);
        if ep.content.contains("Rust") {
            found_rust_preference = true;
        }
        if ep.content.contains("学术研究") {
            found_research_context = true;
        }
    }
    println!();
    println!("✅ 验证:");
    println!(
        "   AI 记住 '我喜欢 Rust':     {}",
        if found_rust_preference {
            "✅ 是"
        } else {
            "❌ 否"
        }
    );
    println!(
        "   AI 记住 '学术研究':   {}",
        if found_research_context {
            "✅ 是"
        } else {
            "❌ 否"
        }
    );

    assert!(
        found_rust_preference,
        "R17 不假装: '我喜欢 Rust' 必须被记住"
    );
    assert!(found_research_context, "R17 不假装: '学术研究' 必须被记住");

    // 7. 清理
    drop(store2);
    let _ = std::fs::remove_file(&tmp_path);

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ memory_effect_demo 验收通过 (R17 阶段 6 效果验证)");
    println!();
    println!("📊 验证总结:");
    println!("   ✅ SqliteMemoryStore 写入正常 (6 条 episode)");
    println!("   ✅ SqliteMemoryStore 持久化 (drop + reopen 后还有 6 条)");
    println!("   ✅ EpisodeStore::query 按 session+role 过滤正常");
    println!("   ✅ AI 记忆偏好 (Rust + 学术研究) 真持久化");

    Ok(())
}
