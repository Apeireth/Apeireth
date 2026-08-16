//! M2 独立验证 harness (lib test target 被 tool_bridge WIP E0433 阻塞期间):
//! #[path] 原样挂载 community.rs 原文; memory_graph 模块用同形 GraphFact 副本替代.
mod memory_graph {
    #[derive(Debug, Clone)]
    pub struct GraphFact {
        pub id: String,
        pub chain: String,
        pub rev: u64,
        pub subject: String,
        pub predicate: String,
        pub object: String,
        pub valid_at: i64,
        pub invalid_at: Option<i64>,
        pub importance: u8,
    }
}
#[path = "crates/apeireth-companion/src/community.rs"]
mod community;
fn main() {}
