//! R16-09 apeireth-memory LLM analysis: Episode 用 LLM 摘要 + 关键词提取
//!
//! 不修改 core::Episode (保持 8 项不修改承诺)
//! 在 memory 层加一个 helper, 接受 LlmProvider, 返回 LLM-generated 分析
//!
//! 用法:
//! ```ignore
//! use apeireth_llm_iface::{ChatMessage, LlmProvider, LlmRequest};  // R179 P0-3: 走 iface 不再 dep apeireth-api
//! use apeireth_core::Episode;
//! use apeireth_memory::llm_analysis::{analyze_episode, AnalysisKind};
//!
//! let llm: Arc<dyn LlmProvider> = ...;
//! let episode = Episode { ... };
//! let summary = analyze_episode(&llm, &episode, AnalysisKind::Summary).await?;
//! ```

use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_llm_iface::{ChatMessage, LlmProvider, LlmRequest}; // R179 P0-3: 拆 memory <-> api 编译期边
use serde::{Deserialize, Serialize};

/// LLM 分析类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisKind {
    /// 摘要 (1-2 句话)
    Summary,
    /// 关键词提取 (3-5 个)
    Keywords,
    /// 风险标记 (是否含 risk-level 关键词)
    RiskFlag,
    /// 12 键哲学守门 (是否符合 12 键)
    PhilosophyGate,
}

impl AnalysisKind {
    fn system_prompt(&self) -> &'static str {
        match self {
            AnalysisKind::Summary =>
                "你是 apeireth-memory 摘要器. 用 1-2 句话总结给定的 episode 内容. 中文, 简洁.",
            AnalysisKind::Keywords =>
                "你是 apeireth-memory 关键词提取器. 从 episode 提取 3-5 个最关键的概念或主题词. 逗号分隔, 中文.",
            AnalysisKind::RiskFlag =>
                "你是 apeireth-memory 风险标记器. 判断 episode 是否含风险关键词 (nuke/weapon/self-modify/override/绕过/核武器/自毁). 输出 'risky' 或 'safe' 加简短理由. 中文.",
            AnalysisKind::PhilosophyGate =>
                "你是 apeireth-memory 12 键哲学守门. 判断 episode 是否符合 12 键 (不假装/不欺骗/实事求是). 输出 'pass' 或 'block' 加简短理由. 中文.",
        }
    }
}

/// LLM 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub kind: AnalysisKind,
    pub content: String,
    pub model: String,
    pub latency_ms: u64,
}

/// 调 LLM 分析一个 Episode
///
/// 失败 (网络/限流) 走 Result 让上层决定
pub async fn analyze_episode(
    llm: &Arc<dyn LlmProvider>,
    episode: &Episode,
    kind: AnalysisKind,
) -> Result<AnalysisResult, String> {
    // 修 Bug: 跟 council_advise 一样, model 字段必须用真 model 名
    let model = if llm.name() == "apeireth-api" {
        "MiniMax-M3".to_string()
    } else {
        llm.name().to_string()
    };

    let user_msg = format!(
        "Episode 内容:\n角色: {}\n内容: {}\n时间: {}",
        episode.role, episode.content, episode.timestamp
    );

    let req = LlmRequest::new(
        &model,
        vec![
            ChatMessage::system(kind.system_prompt().to_string()),
            ChatMessage::user(user_msg),
        ],
    )
    .with_temperature(0.3)
    .with_max_tokens(150);

    let resp = llm
        .complete(req)
        .await
        .map_err(|e| format!("llm 分析失败: {e}"))?;

    Ok(AnalysisResult {
        kind,
        content: resp.content,
        model: resp.model,
        latency_ms: resp.latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_kind_system_prompt_not_empty() {
        for kind in [
            AnalysisKind::Summary,
            AnalysisKind::Keywords,
            AnalysisKind::RiskFlag,
            AnalysisKind::PhilosophyGate,
        ] {
            assert!(!kind.system_prompt().is_empty());
        }
    }
}
