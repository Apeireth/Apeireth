//! Council 7 advisor 数据结构 (跨 SSR/客户端 共享)
//!
//! R18 MVP 简化: 不用 Leptos server_fn (避免 hydration 复杂度),
//! 改用纯 axum form POST → server-side render ResultPage.

use serde::{Deserialize, Serialize};

/// Council 7 advisor 单一意见 (跨 SSR/客户端)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorOpinion {
    pub domain: String,
    pub stance: String,
    pub reasoning: String,
}

/// Council 7 advisor 完整响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilAdviseResponse {
    pub topic: String,
    pub status: String,
    pub advisors: Vec<AdvisorOpinion>,
    pub verdict: String,
    /// 使用的 LLM 协议 ("openai" / "anthropic")
    pub protocol: String,
    /// 辩论唯一 ID (用于 /council/save 关联; 若已自动存则可空)
    pub debate_id: Option<String>,
}
