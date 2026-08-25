//! **战役 2-4 / VCP `agentManager.js` — Agent 主体**
//!
//! **目标**: `Agent` struct + 字段级对齐 VCP 真代码 (alias 多对一, tools 关联 tool-registry).
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md` + `agentManager.js`):
//! - **`agentManager.js:11`** `this.agentMap: Map<alias, filename>` → 我们 `Agent.aliases: Vec<String>`
//!   (一个 agent 可以有多个 alias, LLM 用 alias 引用, 内部解析到唯一 agent ID)
//! - **`agentManager.js:12`** `this.promptCache: Map<alias, prompt>` → 我们 `AgentManager.cache`
//!   (LRU 缓存, alias → Arc`Agent`)
//! - **`agentManager.js:23-31 initialize`** → 我们 `AgentManager::register` / `watch_dir`
//! - **`agentManager.js:36-63 loadMap`** → 我们 `AgentManager::register` 自动维护 alias index
//! - **`agentManager.js:95-127 watchFiles`** → 我们用 `notify` 5.x 监听 dir
//!
//! **不假装** (主哲学锚 #1 不漂移):
//! - ✅ Agent 6 字段真实现 (id / name / aliases / tools / system_prompt / created_at)
//! - ✅ aliases 真支持多 alias (Vec, 去重, 自动加入 alias index)
//! - ✅ tools 字段关联战役 2-1 `ToolRegistry::get` (跨 crate 集成, 不重复)
//! - ✅ created_at 用 `SystemTime::now()` 真时间戳

use serde::{Deserialize, Serialize};

/// **战役 2-4 / VCP `agentManager.js` — Agent 主体**
///
/// **VCP 借鉴**: `agentManager.js:11-17` `agentMap: Map<alias, filename>` + `promptCache: Map<alias, prompt>`
///   + `agentFiles: []` + `folderStructure: {}`
///
/// **Apeireth 简化**:
/// - VCP 是 `alias → filename` (文件路径字符串), 我们是 `alias → Agent` (typed struct)
/// - VCP agent 是一段 prompt 字符串, 我们有 id / name / aliases / tools / system_prompt 完整 struct
/// - VCP 一个 alias 对应一个文件, 我们一个 agent 可以有多个 alias (语义丰富)
///
/// **字段说明**:
/// - `id` — 唯一 ID, 内部识别 (VCP `filename` 真代码对应, 但是 typed)
/// - `name` — 显示名 (VCP 没有, 我们加, 实战中 UI 显示)
/// - `aliases` — 多个 alias (`@coder` / `@chuling` / `@mavis` 同一个), LLM 用 alias 引用
/// - `tools` — 关联的工具名 (战役 2-1 `ToolRegistry::get(name)`, 不复制)
/// - `system_prompt` — Agent 系统 prompt (VCP `promptCache.get(alias)` 字段对应)
/// - `created_at` — 创建时间戳 ms (VCP 没有, 实战审计用)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    /// 唯一 ID (VCP `filename` 对应, 但 typed)
    pub id: String,
    /// 显示名 (VCP 没有, UI 用)
    pub name: String,
    /// 多个 alias (`@coder` / `@chuling` / `@mavis`)
    ///
    /// **VCP 借鉴**: `agentManager.js:11 agentMap: Map<alias, filename>` —
    ///   VCP 一个 alias 对应一个 file, 我们一个 agent 可以有多个 alias
    pub aliases: Vec<String>,
    /// 关联的工具名 (战役 2-1 `ToolRegistry::get(name)`, 不复制)
    ///
    /// **实战**: 实战中 `apeireth-pipeline` 调 `agent.tools` 列表工具
    pub tools: Vec<String>,
    /// Agent 系统 prompt
    ///
    /// **VCP 借鉴**: `agentManager.js:13 promptCache: Map<alias, prompt>` —
    ///   VCP 把整个文件当 prompt, 我们有结构化 `system_prompt` 字段
    pub system_prompt: String,
    /// 创建时间戳 ms (SystemTime::now().duration_since(UNIX_EPOCH).as_millis())
    pub created_at: i64,
}

impl Agent {
    /// 新建 Agent (自动取 created_at = now_ms)
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aliases: Vec<String>,
        tools: Vec<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aliases,
            tools,
            system_prompt: system_prompt.into(),
            created_at: now_ms(),
        }
    }

    /// 取所有 alias (包含 id 自身作为隐式 alias)
    ///
    /// **VCP 行为**: VCP `getAgentPrompt(alias)` 先查 `promptCache`, 再查 `agentMap.get(alias)`.
    ///   我们把 `id` 也作为隐式 alias, 这样 `resolve(id)` 也能命中.
    ///
    /// **不重复**: 实战中 id 和 alias 重名时只算一个
    pub fn all_aliases(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![self.id.clone()];
        for a in &self.aliases {
            if !out.contains(a) {
                out.push(a.clone());
            }
        }
        out
    }

    /// 是否命中 (id 或 alias 任一匹配)
    pub fn matches(&self, key: &str) -> bool {
        self.id == key || self.aliases.iter().any(|a| a == key)
    }

    /// tool 数量
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// alias 数量 (不包含 id 自身)
    pub fn alias_count(&self) -> usize {
        self.aliases.len()
    }
}

/// 当前时间戳 ms (SystemTime::now)
pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// 单元测试 (Agent 基础字段 + alias 命中 + 边界)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_new_basic() {
        let a = Agent::new(
            "coder-1",
            "Coder Agent",
            vec!["@coder".to_string(), "@chuling".to_string()],
            vec!["file_read".to_string()],
            "I am a coder.",
        );
        assert_eq!(a.id, "coder-1");
        assert_eq!(a.name, "Coder Agent");
        assert_eq!(a.aliases.len(), 2);
        assert_eq!(a.tools.len(), 1);
        assert_eq!(a.system_prompt, "I am a coder.");
        assert!(
            a.created_at > 0,
            "created_at 应 > 0, 实际: {}",
            a.created_at
        );
    }

    #[test]
    fn agent_new_with_no_aliases() {
        let a = Agent::new("solo", "Solo", vec![], vec![], "no alias");
        assert_eq!(a.alias_count(), 0);
        assert_eq!(a.tool_count(), 0);
    }

    #[test]
    fn agent_new_with_multiple_tools() {
        let a = Agent::new(
            "multi",
            "Multi",
            vec!["@m".to_string()],
            vec!["t1".to_string(), "t2".to_string(), "t3".to_string()],
            "p",
        );
        assert_eq!(a.tool_count(), 3);
    }

    #[test]
    fn agent_all_aliases_includes_id() {
        let a = Agent::new(
            "main",
            "Main",
            vec!["@alias1".to_string(), "@alias2".to_string()],
            vec![],
            "p",
        );
        let all = a.all_aliases();
        assert_eq!(all.len(), 3, "id + 2 aliases = 3, 实际: {all:?}");
        assert!(all.contains(&"main".to_string()));
        assert!(all.contains(&"@alias1".to_string()));
        assert!(all.contains(&"@alias2".to_string()));
    }

    #[test]
    fn agent_all_aliases_dedup_id_and_alias() {
        // id 跟 alias 重名时, all_aliases 去重
        let a = Agent::new(
            "shared",
            "Shared",
            vec!["shared".to_string(), "@a".to_string()],
            vec![],
            "p",
        );
        let all = a.all_aliases();
        assert_eq!(all.len(), 2, "id == alias 应去重, 实际: {all:?}");
    }

    #[test]
    fn agent_matches_id() {
        let a = Agent::new("myid", "n", vec!["@a".to_string()], vec![], "p");
        assert!(a.matches("myid"));
        assert!(a.matches("@a"));
        assert!(!a.matches("@unknown"));
        assert!(!a.matches(""));
    }

    #[test]
    fn agent_alias_count_and_tool_count() {
        let a = Agent::new(
            "x",
            "X",
            vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            vec!["t1".to_string(), "t2".to_string()],
            "p",
        );
        assert_eq!(a.alias_count(), 3);
        assert_eq!(a.tool_count(), 2);
    }

    #[test]
    fn agent_serialize_deserialize_json() {
        let a = Agent::new(
            "test",
            "Test",
            vec!["@test".to_string()],
            vec!["t".to_string()],
            "system prompt here",
        );
        let json = serde_json::to_string(&a).unwrap();
        let back: Agent = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back, "JSON serialize/deserialize 应 round-trip");
    }

    #[test]
    fn agent_clone() {
        let a = Agent::new("c", "C", vec!["@c".to_string()], vec![], "p");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn agent_equality_partial_eq() {
        // 同一 struct, 字段一致 → eq
        let a1 = Agent::new("same", "n", vec![], vec![], "p");
        // a1.clone() 字段一致 → eq
        assert_eq!(a1, a1.clone(), "clone 应 eq");
        // 不同 id → 不等
        let a2 = Agent::new("different", "n", vec![], vec![], "p");
        assert_ne!(a1, a2, "不同 id 应不等");
        // 不同 name → 不等
        let a3 = Agent::new("same", "different-name", vec![], vec![], "p");
        assert_ne!(a1, a3, "不同 name 应不等");
    }
}
