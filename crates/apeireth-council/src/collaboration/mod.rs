//! R25 D-3: 4 协作模式 — `apeireth-council` 战区 3 (Multi-Agent)
//!
//! **职责** (per v2.0 strategy §2B):
//! - 实现 4 种协作模式: Planner + Executor / Debate / Voting / Hierarchical
//! - 提供统一的 [`CollaborationContext`] + [`CollaborationVerdict`] API
//! - 跟现有 `Council` (R10) + `CouncilMemberDeliberator` (R33-4-1) + `synthesize()` 集成
//!
//! **AutoGen / LangGraph / VCP 借鉴** (per 主哲学锚 S-1):
//! - `Planner+Executor` ↔ LangGraph `PlanAndExecute` (plan_node + execute_node) + AutoGen planner/executor 角色分离
//! - `Debate` ↔ AutoGen `GroupChat.max_round` + LangGraph `MessagesState` (跨轮 state)
//! - `Voting` ↔ AutoGen `GroupChatManager` 投票聚合 + LangGraph `add_conditional_edges`
//! - `Hierarchical` ↔ OpenAI `swarm` handoff + LangGraph `subgraph`
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 R10 `Council::deliberate` / `Council::deliberate_persona` (R10 LOCKED)
//! - 0 改 R33-4 `council_member.rs` (R33-4 LOCKED)
//! - 0 改 R33-4-1 `council_member_deliberation.rs` (R33-4-1 LOCKED — Debate 模式仅 `pub use` + 包装)
//! - 0 改 R33-4-2 `council_member_persona_combo.rs` (R33-4-2 LOCKED)
//! - 0 改 R15 7 强制 advisor (R15 LOCKED)
//! - 0 改 R16-09 `LlmAdvisorBackend` (R16-09 LOCKED, 仅复用)
//! - 0 改 `synthesis.rs` / `hold.rs` / `deliberation.rs` / `sovereignty.rs` / `lifecycle.rs` (R10/R11/R19 LOCKED)
//! - 0 引入 I/O / 网络 / 外部 LLM HTTP
//!
//! **设计原则**:
//! - 4 模式产出统一 `CollaborationVerdict` (per `SynthesisReport` + `opinions` + `steps` + `termination_reason`)
//! - 4 模式独立 sub-module, 互不耦合
//! - Debate 模式复用 R33-4-1 `CouncilMemberDeliberator::deliberate()` (1:1 transparent)
//! - 其他 3 模式自己 `construct opinions + call synthesize()` (跟 R10 `synthesize()` 0 漂移)

#![deny(unsafe_code)]

pub mod types;
pub mod planner_executor;
pub mod debate;
pub mod voting;
pub mod hierarchical;

pub use types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
