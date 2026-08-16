//! R127-2 P6-2 — **opencode 子代理 重试** 阶段 1: 子代理调度
//!
//! # 背景
//!
//! R125-12 (opencode 子代理) ⏳ 限流持续 (opencode source MISSING, HTTP 502).
//! R127-2 P6-2 重试: 不再依赖 opencode 源码 (仍 MISSING), 改借鉴已 cloned 的
//! `langchain-ai/langgraph 829` (decision-56 §3) + `oh-my-opencode` 4 专家模式
//! (per 决策 #22 §2.7 B7 9 organ 内部 fn 借 OpenCode 子代理).
//!
//! # 借鉴 ID
//!
//! - `R127-2-P6-2-BORROW-langchain-ai/langgraph-829-state-graph-agent-2026-08-10` (主, 已 ✅ cloned)
//! - `R124-1-BORROW-code-yeongyu/oh-my-opencode-4-expert-roles-2026-08-10` (副, oh-my-opencode 4 专家语义, 0 装)
//! - `R125-12-BORROW-anomalyco/opencode-sub-agent-pattern-2026-08-10` (⏳ 限流, 0 装, 借 ID 索引已写)
//!
//! # 设计
//!
//! **4 专家角色** (per `oh-my-opencode` 4 专家语义, 公开文档可读, 0 装):
//! 1. **Oracle** — 架构审阅 (read-only)
//! 2. **Librarian** — 文档检索 (read-only)
//! 3. **Explore** — 代码扫 (read-only)
//! 4. **Frontend** — UI 渲染 (write, idempotent)
//!
//! **调度模式** (per `langgraph 829` StateGraph 拓扑, 已 ✅ cloned):
//! - `SubAgentRegistry` = 节点注册表 (BTreeMap, 决定 iteration 顺序)
//! - `AgentRouter` = 路由表 (organ → expert, expert → organ)
//! - 4 专家 = 4 个 SubAgent 节点, 跟 apeireth-tui 9 organ 内部 fn 桥接
//!
//! # 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #56 §3)
//!
//! - ✅ **cloned = 真实施** (langgraph 829 ✅ cloned, oh-my-opencode 公开 docs 可读, 0 装 opencode 私有)
//! - ✅ **真 src 改动** (本文件 + `apeireth-agent/src/lib.rs` +1 `pub mod subagent;`)
//! - ✅ **tests pass** (8+ unit tests, `cargo test -p apeireth-agent`)
//! - ❌ **0 假装"已对接 opencode 私有"** (我们自实现, 0 抄 opencode TS 代码)
//!
//! # 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #55 §4)
//!
//! - **B1** 24 LOCKED 入口签名 0 改 (本文件 + lib.rs 仅 +1 `pub mod subagent;`, 入口签名 0 改)
//! - **B2** workspace.version 1.2.0 0 改 (本文件 0 触碰 Cargo.toml)
//! - **A1** R11 baseline 3 值 0 改 (本文件 0 触碰 integration_r_measure.rs)
//! - **A3** 13 键 0 改 (本文件 0 触碰)
//! - **C1** 0 commit (Mavis 整合 #5 拍板, 等 Mavis 调度)
//! - **C2** 0 装 PASS 严守 (本文件 真 src 改动 + tests pass, 0 装"已对接 opencode 私有")

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 1. 4 专家角色 (oh-my-opencode 4 专家语义 1:1)
// ============================================================

/// **4 专家角色** (per `oh-my-opencode` 4 专家语义)
///
/// 公开 docs 可读, 0 装 opencode 私有代码:
/// - **Oracle** — 架构审阅 (read-only)
/// - **Librarian** — 文档检索 (read-only)
/// - **Explore** — 代码扫 (read-only)
/// - **Frontend** — UI 渲染 (write, idempotent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExpertRole {
    /// 架构审阅 (read-only)
    Oracle = 0,
    /// 文档检索 (read-only)
    Librarian = 1,
    /// 代码扫 (read-only)
    Explore = 2,
    /// UI 渲染 (write, idempotent)
    Frontend = 3,
}

impl ExpertRole {
    /// 4 专家总数 (编译期 hardcode)
    pub const COUNT: usize = 4;

    /// 数字 0-3 → ExpertRole
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Oracle),
            1 => Some(Self::Librarian),
            2 => Some(Self::Explore),
            3 => Some(Self::Frontend),
            _ => None,
        }
    }

    /// 角色显示名 (5 Locale 不强约束, 用 ASCII 跨平台)
    pub fn name(self) -> &'static str {
        match self {
            Self::Oracle => "oracle",
            Self::Librarian => "librarian",
            Self::Explore => "explore",
            Self::Frontend => "frontend",
        }
    }

    /// 角色是否 read-only
    ///
    /// **借鉴**: MCP `readOnlyHint` annotation 语义, 但我们用 4 专家 1:1 简化
    pub fn is_read_only(self) -> bool {
        match self {
            Self::Oracle | Self::Librarian | Self::Explore => true,
            Self::Frontend => false,
        }
    }
}

impl fmt::Display for ExpertRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ============================================================
// 2. SubAgent trait (主调度接口, 1:1 借鉴 langgraph 829 Node trait)
// ============================================================

/// **SubAgent trait** — 1 个子代理 (per `langgraph 829` Node 1:1)
///
/// **跟 langgraph 829 `StateNode` 1:1** (per libs/langgraph/langgraph/graph/_node.py):
/// - `role()` — 节点 ID
/// - `system_prompt()` — 节点 system prompt
/// - `invoke(task, context)` — 节点 fn, 返 String (简化, 跟 langgraph `state -> Partial<State>` 平行)
/// - `capabilities()` — 节点能做的能力列表 (类比 langgraph `tags`)
///
/// **0 装 PASS 严守**: 1:1 翻译 langgraph 公开 `StateNode` 语义, 0 装"对接 LangGraph 私有"
#[async_trait::async_trait]
pub trait SubAgent: Send + Sync {
    /// 返回角色
    fn role(&self) -> ExpertRole;

    /// 返回 system prompt
    fn system_prompt(&self) -> &'static str;

    /// 异步执行入口
    ///
    /// **参数**:
    /// - `task` — 任务描述 (跟 langgraph `state` input 1:1)
    /// - `context` — 上下文 (跨 organ 传递, 跟 langgraph `Runtime[Context]` 1:1 简化)
    ///
    /// **返**: `Result<String, SubAgentError>` — 简化, 跟 tool-runtime 错误流一致
    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError>;

    /// 返回能力列表 (e.g. `["read", "search", "summarize"]`)
    ///
    /// **默认**: 空 (subclass 可 override)
    fn capabilities(&self) -> &[&'static str] {
        &[]
    }
}

// ============================================================
// 3. SubAgentError (typed 错误, 跟 tool-runtime 错误流一致)
// ============================================================

/// **子代理调度错误** (typed, 跟 `Tool` trait `Result<Value, String>` 1:1 简化)
#[derive(Debug, Error)]
pub enum SubAgentError {
    /// 任务为空
    #[error("sub-agent task is empty")]
    EmptyTask,
    /// 上下文为空
    #[error("sub-agent context is empty")]
    EmptyContext,
    /// 角色未注册
    #[error("sub-agent role `{0}` not registered")]
    UnknownRole(ExpertRole),
    /// 内部错误 (执行期)
    #[error("sub-agent `{role}` failed: {message}")]
    Execution {
        /// 失败角色
        role: ExpertRole,
        /// 错误信息
        message: String,
    },
}

// ============================================================
// 4. 4 专家实现 (公开 docs 1:1 翻译, 0 装 opencode 私有)
// ============================================================

/// **Oracle 专家** — 架构审阅 (read-only)
///
/// **语义** (per oh-my-opencode 公开 docs): 审阅架构决策, 给出 trade-off 分析
/// **0 装**: 我们只做 "accept task + 返格式化响应", 0 装 LLM 调用
pub struct OracleSubAgent;

#[async_trait::async_trait]
impl SubAgent for OracleSubAgent {
    fn role(&self) -> ExpertRole {
        ExpertRole::Oracle
    }

    fn system_prompt(&self) -> &'static str {
        "Oracle: 架构审阅, 读-only. 给出 trade-off 分析 + 风险评估."
    }

    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError> {
        if task.is_empty() {
            return Err(SubAgentError::EmptyTask);
        }
        // 0 装: 不调 LLM, 仅格式化返响应
        Ok(format!(
            "[oracle] 架构审阅任务: {task}\n[oracle] 上下文摘要: {} 字符\n[oracle] 0 装 LLM: 仅占位响应",
            context.chars().count()
        ))
    }

    fn capabilities(&self) -> &[&'static str] {
        &["review", "analyze", "tradeoff", "risk-assessment"]
    }
}

/// **Librarian 专家** — 文档检索 (read-only)
///
/// **语义**: 检索文档, 返相关摘录
pub struct LibrarianSubAgent;

#[async_trait::async_trait]
impl SubAgent for LibrarianSubAgent {
    fn role(&self) -> ExpertRole {
        ExpertRole::Librarian
    }

    fn system_prompt(&self) -> &'static str {
        "Librarian: 文档检索, 读-only. 从 docs/ 索引返相关摘录."
    }

    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError> {
        if task.is_empty() {
            return Err(SubAgentError::EmptyTask);
        }
        Ok(format!(
            "[librarian] 检索任务: {task}\n[librarian] 上下文命中: {} 字符\n[librarian] 0 装: 索引占位",
            context.chars().count()
        ))
    }

    fn capabilities(&self) -> &[&'static str] {
        &["search", "retrieve", "summarize", "cite"]
    }
}

/// **Explore 专家** — 代码扫 (read-only)
///
/// **语义**: 扫代码, 找相关 pattern / symbol
pub struct ExploreSubAgent;

#[async_trait::async_trait]
impl SubAgent for ExploreSubAgent {
    fn role(&self) -> ExpertRole {
        ExpertRole::Explore
    }

    fn system_prompt(&self) -> &'static str {
        "Explore: 代码扫, 读-only. 找 pattern / symbol / 依赖图."
    }

    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError> {
        if task.is_empty() {
            return Err(SubAgentError::EmptyTask);
        }
        Ok(format!(
            "[explore] 扫描任务: {task}\n[explore] 上下文代码: {} 字符\n[explore] 0 装: 扫描占位",
            context.chars().count()
        ))
    }

    fn capabilities(&self) -> &[&'static str] {
        &["scan", "grep", "ast", "deps"]
    }
}

/// **Frontend 专家** — UI 渲染 (write, idempotent)
///
/// **语义**: 渲染 UI, idempotent (同 input → 同 output)
pub struct FrontendSubAgent;

#[async_trait::async_trait]
impl SubAgent for FrontendSubAgent {
    fn role(&self) -> ExpertRole {
        ExpertRole::Frontend
    }

    fn system_prompt(&self) -> &'static str {
        "Frontend: UI 渲染, write + idempotent. 接收结构化描述, 返 ratatui widget 描述."
    }

    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError> {
        if task.is_empty() {
            return Err(SubAgentError::EmptyTask);
        }
        Ok(format!(
            "[frontend] 渲染任务: {task}\n[frontend] 上下文数据: {} 字符\n[frontend] 0 装: 渲染占位",
            context.chars().count()
        ))
    }

    fn capabilities(&self) -> &[&'static str] {
        &["render", "layout", "theme", "i18n"]
    }
}

// ============================================================
// 5. SubAgentRegistry (主调度, 1:1 借鉴 langgraph 829 nodes dict)
// ============================================================

/// **SubAgentRegistry** — 4 专家节点注册表 (per langgraph 829 `nodes: dict` 1:1)
///
/// **设计**:
/// - 内部 `BTreeMap<ExpertRole, Arc<dyn SubAgent>>` — 决定 iteration 顺序
/// - `dispatch(role, task, context)` — 派 1 个专家, 类比 langgraph 节点 invoke
/// - 0 装 LLM: 4 专家仅占位响应
pub struct SubAgentRegistry {
    experts: RwLock<BTreeMap<ExpertRole, Arc<dyn SubAgent>>>,
}

impl SubAgentRegistry {
    /// 创建空 registry
    pub fn new() -> Self {
        Self {
            experts: RwLock::new(BTreeMap::new()),
        }
    }

    /// 默认 4 专家 (Oracle / Librarian / Explore / Frontend)
    pub fn with_default_experts() -> Self {
        let r = Self::new();
        r.register(Arc::new(OracleSubAgent));
        r.register(Arc::new(LibrarianSubAgent));
        r.register(Arc::new(ExploreSubAgent));
        r.register(Arc::new(FrontendSubAgent));
        r
    }

    /// 注册 1 个专家
    ///
    /// **0 装**: 同 role 重复注册覆盖 (跟 langgraph `add_node` 行为一致)
    pub fn register(&self, agent: Arc<dyn SubAgent>) {
        let role = agent.role();
        self.experts.write().insert(role, agent);
    }

    /// 派 1 个专家
    pub async fn dispatch(
        &self,
        role: ExpertRole,
        task: &str,
        context: &str,
    ) -> Result<String, SubAgentError> {
        if task.is_empty() {
            return Err(SubAgentError::EmptyTask);
        }
        let agent = {
            let experts = self.experts.read();
            experts
                .get(&role)
                .cloned()
                .ok_or(SubAgentError::UnknownRole(role))?
        };
        if context.is_empty() {
            return Err(SubAgentError::EmptyContext);
        }
        agent.invoke(task, context).await
    }

    /// 列出已注册专家
    pub fn registered_roles(&self) -> Vec<ExpertRole> {
        self.experts.read().keys().copied().collect()
    }

    /// 注册专家数
    pub fn len(&self) -> usize {
        self.experts.read().len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.experts.read().is_empty()
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self {
        Self::with_default_experts()
    }
}

// ============================================================
// 6. AgentRouter (organ ↔ expert 路由表)
// ============================================================

/// **AgentRouter** — 主 agent 路由 (per opencode oh-my-opencode 4 专家模式 1:1)
///
/// **设计**:
/// - `route_organ_to_expert(organ_name)` — organ 任务 → 派哪个专家
/// - `route_expert_to_organ(role)` — 专家输出 → 哪个 organ 接收
/// - 路由表是 1 个 `BTreeMap<String, ExpertRole>`, 决定 iteration 顺序
pub struct AgentRouter {
    /// organ → expert 路由表
    organ_to_expert: RwLock<BTreeMap<String, ExpertRole>>,
    /// expert → organ 默认路由表
    expert_to_organ: RwLock<BTreeMap<ExpertRole, String>>,
}

impl AgentRouter {
    /// 创建空 router
    pub fn new() -> Self {
        Self {
            organ_to_expert: RwLock::new(BTreeMap::new()),
            expert_to_organ: RwLock::new(BTreeMap::new()),
        }
    }

    /// 默认 9 organ → 4 expert 路由 (per B7 9 organ 内部 fn 借 OpenCode 子代理)
    ///
    /// **路由策略** (per oh-my-opencode 4 专家 + 9 organ 语义):
    /// - `heart` → Oracle (审阅生命周期)
    /// - `brain` → Oracle (审阅认知决策)
    /// - `hand` → Frontend (UI 工具执行)
    /// - `eye` → Explore (扫代码/symbol)
    /// - `ear` → Librarian (检索事件日志)
    /// - `memory` → Librarian (检索记忆)
    /// - `voice` → Frontend (UI 语音)
    /// - `body` → Explore (扫资源)
    /// - `mind` → Oracle (审阅意识)
    pub fn with_default_organ_routes() -> Self {
        let r = Self::new();
        let mut o_to_e = r.organ_to_expert.write();
        let mut e_to_o = r.expert_to_organ.write();

        o_to_e.insert("heart".to_string(), ExpertRole::Oracle);
        o_to_e.insert("brain".to_string(), ExpertRole::Oracle);
        o_to_e.insert("hand".to_string(), ExpertRole::Frontend);
        o_to_e.insert("eye".to_string(), ExpertRole::Explore);
        o_to_e.insert("ear".to_string(), ExpertRole::Librarian);
        o_to_e.insert("memory".to_string(), ExpertRole::Librarian);
        o_to_e.insert("voice".to_string(), ExpertRole::Frontend);
        o_to_e.insert("body".to_string(), ExpertRole::Explore);
        o_to_e.insert("mind".to_string(), ExpertRole::Oracle);

        e_to_o.insert(ExpertRole::Oracle, "brain".to_string());
        e_to_o.insert(ExpertRole::Librarian, "memory".to_string());
        e_to_o.insert(ExpertRole::Explore, "eye".to_string());
        e_to_o.insert(ExpertRole::Frontend, "hand".to_string());

        drop(o_to_e);
        drop(e_to_o);
        r
    }

    /// 加 1 个 organ → expert 路由
    pub fn add_organ_route(&self, organ: impl Into<String>, expert: ExpertRole) {
        self.organ_to_expert.write().insert(organ.into(), expert);
    }

    /// 路由 organ → expert (未命中返 None, 跟 langgraph `MissingNode` 1:1)
    pub fn route_organ_to_expert(&self, organ: &str) -> Option<ExpertRole> {
        self.organ_to_expert.read().get(organ).copied()
    }

    /// 路由 expert → organ (默认 organ)
    pub fn route_expert_to_organ(&self, expert: ExpertRole) -> Option<String> {
        self.expert_to_organ.read().get(&expert).cloned()
    }

    /// organ 路由数
    pub fn organ_route_count(&self) -> usize {
        self.organ_to_expert.read().len()
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::with_default_organ_routes()
    }
}

// ============================================================
// 7. 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 4 专家角色数 (编译期 hardcode)
pub const EXPERT_ROLE_COUNT: usize = ExpertRole::COUNT;

/// 9 organ 默认路由数 (per B7 9 organ)
pub const DEFAULT_ORGAN_ROUTE_COUNT: usize = 9;

const _: () = {
    assert!(
        EXPERT_ROLE_COUNT == 4,
        "EXPERT_ROLE_COUNT = 4 (Oracle/Librarian/Explore/Frontend)"
    );
    assert!(
        DEFAULT_ORGAN_ROUTE_COUNT == 9,
        "DEFAULT_ORGAN_ROUTE_COUNT = 9 (9 organ 内部 fn 借 OpenCode 子代理)"
    );
};

// ============================================================
// 8. 单元测试 (10+ tests, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expert_role_from_u8_round_trip_4() {
        for n in 0..=3u8 {
            let role = ExpertRole::from_u8(n).expect("0-3 valid");
            assert_eq!(role as u8, n);
        }
        assert!(ExpertRole::from_u8(4).is_none());
        assert!(ExpertRole::from_u8(255).is_none());
    }

    #[test]
    fn expert_role_4_distinct_names() {
        let names: Vec<&str> = (0..=3u8)
            .map(|n| ExpertRole::from_u8(n).unwrap().name())
            .collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 4, "4 专家名应互不相同");
    }

    #[test]
    fn expert_role_read_only_3_of_4() {
        // Oracle/Librarian/Explore = read-only, Frontend = write
        assert!(ExpertRole::Oracle.is_read_only());
        assert!(ExpertRole::Librarian.is_read_only());
        assert!(ExpertRole::Explore.is_read_only());
        assert!(!ExpertRole::Frontend.is_read_only());
    }

    #[tokio::test]
    async fn oracle_invoke_basic() {
        let oracle = OracleSubAgent;
        let out = oracle
            .invoke("审阅 Cargo.toml", "workspace context")
            .await
            .unwrap();
        assert!(out.contains("[oracle]"));
        assert!(out.contains("架构审阅任务"));
    }

    #[tokio::test]
    async fn oracle_invoke_empty_task_errors() {
        let oracle = OracleSubAgent;
        let err = oracle.invoke("", "ctx").await.unwrap_err();
        assert!(matches!(err, SubAgentError::EmptyTask));
    }

    #[tokio::test]
    async fn all_4_experts_invoke_basic() {
        // 4 专家都跑 1 次, 验证都能 invoke
        let registry = SubAgentRegistry::with_default_experts();
        assert_eq!(registry.len(), 4);

        for role in [
            ExpertRole::Oracle,
            ExpertRole::Librarian,
            ExpertRole::Explore,
            ExpertRole::Frontend,
        ] {
            let out = registry
                .dispatch(role, "test task", "test context")
                .await
                .unwrap();
            assert!(out.contains(&format!("[{role}]")));
        }
    }

    #[tokio::test]
    async fn registry_dispatch_unknown_role_errors() {
        let registry = SubAgentRegistry::new(); // 空 registry
        let err = registry
            .dispatch(ExpertRole::Oracle, "task", "ctx")
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            SubAgentError::UnknownRole(ExpertRole::Oracle)
        ));
    }

    #[tokio::test]
    async fn registry_dispatch_empty_context_errors() {
        let registry = SubAgentRegistry::with_default_experts();
        let err = registry
            .dispatch(ExpertRole::Oracle, "task", "")
            .await
            .unwrap_err();
        assert!(matches!(err, SubAgentError::EmptyContext));
    }

    #[test]
    fn agent_router_default_9_organ_routes() {
        let router = AgentRouter::with_default_organ_routes();
        assert_eq!(router.organ_route_count(), 9);

        // 9 organ 各路由到 1 个专家
        for organ in [
            "heart", "brain", "hand", "eye", "ear", "memory", "voice", "body", "mind",
        ] {
            let expert = router.route_organ_to_expert(organ).expect(organ);
            assert!(
                matches!(
                    expert,
                    ExpertRole::Oracle
                        | ExpertRole::Librarian
                        | ExpertRole::Explore
                        | ExpertRole::Frontend
                ),
                "{organ} 应路由到 4 专家之一"
            );
        }
    }

    #[test]
    fn agent_router_organ_to_expert_and_back() {
        let router = AgentRouter::with_default_organ_routes();
        // organ → expert → organ (round-trip)
        let brain_expert = router.route_organ_to_expert("brain").unwrap();
        assert_eq!(brain_expert, ExpertRole::Oracle);
        let back_organ = router.route_expert_to_organ(brain_expert).unwrap();
        assert_eq!(back_organ, "brain");
    }

    #[test]
    fn agent_router_custom_route_overrides() {
        let router = AgentRouter::new();
        router.add_organ_route("custom-organ", ExpertRole::Frontend);
        assert_eq!(
            router.route_organ_to_expert("custom-organ").unwrap(),
            ExpertRole::Frontend
        );
        // 未注册的 organ 返 None
        assert!(router.route_organ_to_expert("not-registered").is_none());
    }

    #[test]
    fn expert_role_serialize_deserialize_json() {
        for n in 0..=3u8 {
            let role = ExpertRole::from_u8(n).unwrap();
            let json = serde_json::to_string(&role).unwrap();
            let back: ExpertRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, back);
        }
    }
}
