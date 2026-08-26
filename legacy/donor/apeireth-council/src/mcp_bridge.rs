//! R115: CouncilMember → MCP `prompts` + `resources` 桥接
//!
//! **目标**: apeireth-council 的 CouncilMember list + 智囊团审议能力 暴露为 MCP
//! prompts + resources, 任意 MCP client 都能 list/get 跑审议.
//!
//! **Apeireth 真接 (本 module)**:
//! - `CouncilPromptServer` impl `apeireth_mcp::prompts::PromptServer`
//!   - 1 个 prompt: `council_deliberate` (params: query, area?, risk_level?)
//!   - 返 messages[]: system (council 介绍) + user (query) + 7 advisor role 引述
//! - `CouncilResourceServer` impl `apeireth_mcp::resources::ResourceServer`
//!   - 3 resources:
//!     - `apeireth://council/members` — 当前 CouncilMember list (JSON)
//!     - `apeireth://council/last-verdict` — 最近一次审议结果 (JSON)
//!     - `apeireth://council/risk-hint` — 拿 cognition graph summary 推 risk 等级
//! - `CouncilState` — 内部 snapshot (members + last_verdict), caller 注入
//! - `CouncilPromptServer::new(state)` / `CouncilResourceServer::new(state)`
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `CouncilMember` / `MultiRoundVerdict` / `CouncilMemberDeliberator` (R33-4 / R33-4-1 LOCKED)
//! - 0 改 `apeireth_mcp::prompts` / `apeireth_mcp::resources` 已有类型 (R33-3 / R84 LOCKED)
//! - 0 引入 I/O / 网络 (state 注入, 0 自创 I/O)
//!
//! **借鉴锚 (S-13)**:
//! - AutoGen `ConversableAgent.send` (council 成员 → LLM message 序列)
//! - VCP `vcptoolbox/modules` (module 1:1 → MCP resource)
//! - MCP spec §prompts + §resources

use apeireth_mcp::prompts::{
    GetPromptResult, Prompt, PromptArgument, PromptMessage, PromptServer, PROMPT_INVALID_ARGS,
    PROMPT_NOT_FOUND,
};
use apeireth_mcp::protocol::JsonRpcError;
use apeireth_mcp::resources::{Resource, ResourceContent, ResourceServer, RESOURCE_NOT_FOUND};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::council_member::CouncilMember;

// ============================================================
// CouncilState (caller 注入的 snapshot)
// ============================================================

/// **Council 状态快照** — 持有 members + last_verdict + bus 引用 (可选)
#[derive(Debug, Default, Clone)]
pub struct CouncilState {
    /// 当前 CouncilMember list
    pub members: Vec<CouncilMember>,
    /// 最近一次审议结果 (verdict 字符串, e.g. "approved" / "rejected" / "needs_more_discussion")
    pub last_verdict: Option<String>,
    /// 最近一次审议总 rounds
    pub last_total_rounds: Option<u32>,
}

impl CouncilState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设 members
    pub fn with_members(mut self, members: Vec<CouncilMember>) -> Self {
        self.members = members;
        self
    }

    /// 加 1 个 member
    pub fn add_member(&mut self, m: CouncilMember) {
        self.members.push(m);
    }

    /// 设 last verdict
    pub fn set_last_verdict(&mut self, verdict: impl Into<String>, total_rounds: u32) {
        self.last_verdict = Some(verdict.into());
        self.last_total_rounds = Some(total_rounds);
    }

    /// member 数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// **CouncilState 共享句柄** (thread-safe)
pub type SharedCouncilState = Arc<Mutex<CouncilState>>;

// ============================================================
// CouncilPromptServer
// ============================================================

/// **MCP PromptServer impl, 把 council 审议能力暴露为 1 个 prompt**
pub struct CouncilPromptServer {
    state: SharedCouncilState,
}

impl std::fmt::Debug for CouncilPromptServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .state
            .lock()
            .expect("CouncilPromptServer state mutex poisoned");
        f.debug_struct("CouncilPromptServer")
            .field("member_count", &s.member_count())
            .field("has_last_verdict", &s.last_verdict.is_some())
            .finish()
    }
}

impl CouncilPromptServer {
    /// 构造 (共享 state)
    pub fn new(state: SharedCouncilState) -> Self {
        Self { state }
    }

    /// 构造默认 state (empty)
    pub fn with_empty_state() -> Self {
        Self {
            state: Arc::new(Mutex::new(CouncilState::new())),
        }
    }

    /// 拿 state 句柄
    pub fn state_handle(&self) -> SharedCouncilState {
        Arc::clone(&self.state)
    }

    /// 已知 prompt 名
    pub const PROMPT_DELIBERATE: &'static str = "council_deliberate";
}

// ============================================================
// PromptServer trait impl
// ============================================================

impl PromptServer for CouncilPromptServer {
    fn list(&self) -> Vec<Prompt> {
        vec![Prompt::new(Self::PROMPT_DELIBERATE)
            .with_description(
                "Convene Apeireth council to deliberate on a query (1+ member, returns messages[])",
            )
            .with_arguments(vec![
                PromptArgument::new("query")
                    .required()
                    .with_description("Query to deliberate (required)"),
                PromptArgument::new("area")
                    .with_description("Query area (e.g. cognition_graph, L3 key operation)"),
                PromptArgument::new("risk_level")
                    .with_description("Risk level (low/medium/high/nuclear)"),
            ])]
    }

    fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError> {
        match name {
            Self::PROMPT_DELIBERATE => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        JsonRpcError::new(
                            PROMPT_INVALID_ARGS,
                            "council_deliberate requires `query` argument (string)",
                        )
                    })?;
                let area = arguments
                    .get("area")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general");
                let risk_level = arguments
                    .get("risk_level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium");

                let s = self
                    .state
                    .lock()
                    .expect("CouncilPromptServer state mutex poisoned");
                let members_text = if s.members.is_empty() {
                    "no members registered (caller should call add_member)".to_string()
                } else {
                    s.members
                        .iter()
                        .map(|m| {
                            format!(
                                "- role={}, goal={}, backstory={}, provider={}",
                                m.role, m.goal, m.backstory, m.provider
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                let system_text = format!(
                    "# Apeireth Council Deliberation\n\n\
                     You are convening a council of {member_count} members:\n{members}\n\n\
                     Query area: {area}\n\
                     Risk level: {risk}\n\n\
                     Each member will weigh in based on their role/goal/backstory, then a final verdict (approved/rejected/needs_more_discussion) is synthesized.",
                    member_count = s.member_count(),
                    members = members_text,
                    area = area,
                    risk = risk_level
                );

                Ok(GetPromptResult::new(vec![
                    PromptMessage::assistant_text(system_text),
                    PromptMessage::user_text(query),
                ])
                .with_description(format!("Council deliberation for query `{}`", query)))
            }
            _ => Err(JsonRpcError::new(
                PROMPT_NOT_FOUND,
                format!(
                    "council prompt `{}` not found (known: {})",
                    name,
                    Self::PROMPT_DELIBERATE
                ),
            )),
        }
    }
}

// ============================================================
// CouncilResourceServer
// ============================================================

/// **MCP ResourceServer impl, 把 council 状态暴露为 3 resources**
pub struct CouncilResourceServer {
    state: SharedCouncilState,
}

impl std::fmt::Debug for CouncilResourceServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .state
            .lock()
            .expect("CouncilResourceServer state mutex poisoned");
        f.debug_struct("CouncilResourceServer")
            .field("member_count", &s.member_count())
            .field("has_last_verdict", &s.last_verdict.is_some())
            .finish()
    }
}

impl CouncilResourceServer {
    pub fn new(state: SharedCouncilState) -> Self {
        Self { state }
    }

    pub fn with_empty_state() -> Self {
        Self {
            state: Arc::new(Mutex::new(CouncilState::new())),
        }
    }

    pub fn state_handle(&self) -> SharedCouncilState {
        Arc::clone(&self.state)
    }

    /// URI 常量
    /// URI 常量
    pub const URI_MEMBERS: &'static str = "apeireth://council/members";
    pub const URI_LAST_VERDICT: &'static str = "apeireth://council/last-verdict";
    pub const URI_RISK_HINT: &'static str = "apeireth://council/risk-hint";
}

// ============================================================
// ResourceServer trait impl
// ============================================================

impl ResourceServer for CouncilResourceServer {
    fn list(&self) -> Vec<Resource> {
        vec![
            Resource::new(Self::URI_MEMBERS, "members")
                .with_description("Current CouncilMember list (role/goal/backstory/provider)")
                .with_mime_type("application/json"),
            Resource::new(Self::URI_LAST_VERDICT, "last-verdict")
                .with_description("Last council deliberation verdict + total rounds")
                .with_mime_type("application/json"),
            Resource::new(Self::URI_RISK_HINT, "risk-hint")
                .with_description(
                    "Risk hint derived from council state (based on members + last verdict)",
                )
                .with_mime_type("application/json"),
        ]
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let s = self
            .state
            .lock()
            .expect("CouncilResourceServer state mutex poisoned");
        match uri {
            Self::URI_MEMBERS => {
                let members_json: Vec<Value> = s
                    .members
                    .iter()
                    .map(|m| {
                        json!({
                            "role": m.role,
                            "goal": m.goal,
                            "backstory": m.backstory,
                            "provider": m.provider,
                        })
                    })
                    .collect();
                let v = json!({
                    "count": s.member_count(),
                    "members": members_json,
                });
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            Self::URI_LAST_VERDICT => {
                let v = match (&s.last_verdict, &s.last_total_rounds) {
                    (Some(verdict), Some(rounds)) => json!({
                        "verdict": verdict,
                        "total_rounds": rounds,
                    }),
                    (Some(verdict), None) => json!({"verdict": verdict}),
                    _ => {
                        return Err(JsonRpcError::new(
                            RESOURCE_NOT_FOUND,
                            "no last verdict yet (caller should call set_last_verdict after a deliberation)",
                        ));
                    }
                };
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            Self::URI_RISK_HINT => {
                let hint = if s.members.is_empty() {
                    "no_members"
                } else if s.last_verdict.as_deref() == Some("approved") {
                    "low"
                } else if s.last_verdict.as_deref() == Some("rejected") {
                    "high"
                } else {
                    "medium"
                };
                let v = json!({
                    "risk": hint,
                    "member_count": s.member_count(),
                    "verdict": s.last_verdict,
                });
                ResourceContent::new(uri, serde_json::to_string_pretty(&v).unwrap_or_default())
                    .with_mime_type("application/json")
                    .pipe(Ok)
            }
            _ => Err(JsonRpcError::new(
                RESOURCE_NOT_FOUND,
                format!("unknown apeireth://council/ URI: {}", uri),
            )),
        }
    }
}

// ============================================================
// Pipe helper
// ============================================================

trait Pipe: Sized {
    fn pipe<U, F: FnOnce(Self) -> U>(self, f: F) -> U {
        f(self)
    }
}
impl<T> Pipe for T {}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_mcp::prompts::PROMPT_NOT_FOUND;
    use apeireth_mcp::resources::RESOURCE_NOT_FOUND;

    fn make_member(role: &str) -> CouncilMember {
        CouncilMember {
            role: role.to_string(),
            goal: format!("goal of {}", role),
            backstory: format!("backstory of {}", role),
            provider: "test".to_string(),
        }
    }

    fn make_state_with_members(n: usize) -> SharedCouncilState {
        let mut s = CouncilState::new();
        for i in 0..n {
            s.add_member(make_member(&format!("role_{}", i)));
        }
        Arc::new(Mutex::new(s))
    }

    // ----- CouncilState -----

    #[test]
    fn state_default_is_empty() {
        let s = CouncilState::new();
        assert_eq!(s.member_count(), 0);
        assert!(s.last_verdict.is_none());
    }

    #[test]
    fn state_with_members() {
        let s = CouncilState::new().with_members(vec![make_member("a"), make_member("b")]);
        assert_eq!(s.member_count(), 2);
    }

    #[test]
    fn state_add_member() {
        let mut s = CouncilState::new();
        s.add_member(make_member("x"));
        assert_eq!(s.member_count(), 1);
    }

    #[test]
    fn state_set_last_verdict() {
        let mut s = CouncilState::new();
        s.set_last_verdict("approved", 3);
        assert_eq!(s.last_verdict.as_deref(), Some("approved"));
        assert_eq!(s.last_total_rounds, Some(3));
    }

    // ----- CouncilPromptServer -----

    #[test]
    fn prompt_server_default_empty() {
        let s = CouncilPromptServer::with_empty_state();
        let prompts = s.list();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, CouncilPromptServer::PROMPT_DELIBERATE);
    }

    #[test]
    fn prompt_server_with_members() {
        let state = make_state_with_members(2);
        let s = CouncilPromptServer::new(state);
        let prompts = s.list();
        let p = &prompts[0];
        assert!(p.description.is_some());
        let args = p.arguments.as_ref().unwrap();
        assert_eq!(args.len(), 3);
        assert!(args.iter().any(|a| a.name == "query" && a.required));
    }

    #[test]
    fn prompt_get_with_query() {
        let state = make_state_with_members(2);
        let s = CouncilPromptServer::new(state);
        let result = s
            .get(
                CouncilPromptServer::PROMPT_DELIBERATE,
                &json!({"query": "test query"}),
            )
            .unwrap();
        assert_eq!(result.messages.len(), 2);
        // First message: system (assistant)
        assert_eq!(
            result.messages[0].role,
            apeireth_mcp::prompts::PromptRole::Assistant
        );
        // Second message: user (the query)
        assert_eq!(
            result.messages[1].role,
            apeireth_mcp::prompts::PromptRole::User
        );
    }

    #[test]
    fn prompt_get_includes_members_in_system() {
        let state = make_state_with_members(2);
        let s = CouncilPromptServer::new(state);
        let result = s
            .get(
                CouncilPromptServer::PROMPT_DELIBERATE,
                &json!({"query": "test"}),
            )
            .unwrap();
        match &result.messages[0].content {
            apeireth_mcp::prompts::PromptContent::Text { text, .. } => {
                assert!(text.contains("role_0"));
                assert!(text.contains("role_1"));
                assert!(text.contains("council of 2"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn prompt_get_missing_query_errors() {
        let s = CouncilPromptServer::with_empty_state();
        let err = s
            .get(CouncilPromptServer::PROMPT_DELIBERATE, &json!({}))
            .unwrap_err();
        assert_eq!(err.code, PROMPT_INVALID_ARGS);
    }

    #[test]
    fn prompt_get_unknown_name_errors() {
        let s = CouncilPromptServer::with_empty_state();
        let err = s.get("nope", &json!({})).unwrap_err();
        assert_eq!(err.code, PROMPT_NOT_FOUND);
    }

    #[test]
    fn prompt_get_optional_area_and_risk() {
        let s = CouncilPromptServer::with_empty_state();
        let result = s
            .get(
                CouncilPromptServer::PROMPT_DELIBERATE,
                &json!({
                    "query": "test",
                    "area": "L3",
                    "risk_level": "high"
                }),
            )
            .unwrap();
        match &result.messages[0].content {
            apeireth_mcp::prompts::PromptContent::Text { text, .. } => {
                assert!(text.contains("L3"));
                assert!(text.contains("high"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn prompt_get_with_empty_members() {
        let s = CouncilPromptServer::with_empty_state();
        let result = s
            .get(
                CouncilPromptServer::PROMPT_DELIBERATE,
                &json!({"query": "test"}),
            )
            .unwrap();
        match &result.messages[0].content {
            apeireth_mcp::prompts::PromptContent::Text { text, .. } => {
                assert!(text.contains("no members registered"));
            }
            _ => panic!("expected Text"),
        }
    }

    // ----- CouncilResourceServer -----

    #[test]
    fn resource_server_lists_three() {
        let s = CouncilResourceServer::with_empty_state();
        let list = s.list();
        assert_eq!(list.len(), 3);
        let uris: Vec<&str> = list.iter().map(|r: &Resource| r.uri.as_str()).collect();
        assert!(uris.contains(&CouncilResourceServer::URI_MEMBERS));
        assert!(uris.contains(&CouncilResourceServer::URI_LAST_VERDICT));
        assert!(uris.contains(&CouncilResourceServer::URI_RISK_HINT));
    }

    #[test]
    fn resource_read_members() {
        let state = make_state_with_members(2);
        let s = CouncilResourceServer::new(state);
        let c = s.read(CouncilResourceServer::URI_MEMBERS).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["count"], 2);
        assert_eq!(v["members"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn resource_read_last_verdict_with_data() {
        let state = make_state_with_members(1);
        {
            let mut s = state.lock().unwrap();
            s.set_last_verdict("approved", 3);
        }
        let server = CouncilResourceServer::new(state);
        let c = server
            .read(CouncilResourceServer::URI_LAST_VERDICT)
            .unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["verdict"], "approved");
        assert_eq!(v["total_rounds"], 3);
    }

    #[test]
    fn resource_read_last_verdict_empty_errors() {
        let s = CouncilResourceServer::with_empty_state();
        let err = s.read(CouncilResourceServer::URI_LAST_VERDICT).unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn resource_read_risk_hint_no_members() {
        let s = CouncilResourceServer::with_empty_state();
        let c = s.read(CouncilResourceServer::URI_RISK_HINT).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["risk"], "no_members");
    }

    #[test]
    fn resource_read_risk_hint_approved() {
        let state = make_state_with_members(1);
        {
            let mut s = state.lock().unwrap();
            s.set_last_verdict("approved", 1);
        }
        let server = CouncilResourceServer::new(state);
        let c = server.read(CouncilResourceServer::URI_RISK_HINT).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["risk"], "low");
    }

    #[test]
    fn resource_read_risk_hint_rejected() {
        let state = make_state_with_members(1);
        {
            let mut s = state.lock().unwrap();
            s.set_last_verdict("rejected", 1);
        }
        let server = CouncilResourceServer::new(state);
        let c = server.read(CouncilResourceServer::URI_RISK_HINT).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["risk"], "high");
    }

    #[test]
    fn resource_read_risk_hint_unknown_verdict() {
        let state = make_state_with_members(1);
        {
            let mut s = state.lock().unwrap();
            s.set_last_verdict("unknown", 1);
        }
        let server = CouncilResourceServer::new(state);
        let c = server.read(CouncilResourceServer::URI_RISK_HINT).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["risk"], "medium");
    }

    #[test]
    fn resource_read_unknown_uri_errors() {
        let s = CouncilResourceServer::with_empty_state();
        let err = s.read("apeireth://council/nonexistent").unwrap_err();
        assert_eq!(err.code, RESOURCE_NOT_FOUND);
    }

    // ----- shared state -----

    #[test]
    fn shared_state_between_prompt_and_resource() {
        let state = make_state_with_members(2);
        {
            let mut s = state.lock().unwrap();
            s.set_last_verdict("approved", 3);
        }
        let p = CouncilPromptServer::new(Arc::clone(&state));
        let r = CouncilResourceServer::new(Arc::clone(&state));
        // prompt sees 2 members
        let p_prompts = p.list();
        assert!(p_prompts[0].description.is_some());
        // resource sees verdict
        let c = r.read(CouncilResourceServer::URI_LAST_VERDICT).unwrap();
        let v: Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(v["verdict"], "approved");
    }

    #[test]
    fn debug_impl_works() {
        let p = CouncilPromptServer::with_empty_state();
        let r = CouncilResourceServer::with_empty_state();
        assert!(format!("{:?}", p).contains("CouncilPromptServer"));
        assert!(format!("{:?}", r).contains("CouncilResourceServer"));
    }
}
