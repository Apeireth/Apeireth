//! R16-09 apeireth-asi LLM judge (6 语义维度用 LLM 评估)
//!
//! 6 维从 V0.5 24 维中挑出, 需 LLM 语义判断的 (非纯统计可算):
//! 11. core_values_consistency   - 核心价值一致性
//! 12. voice_consistency         - 声音一致性
//! 15. philosophy_alignment      - 12 键哲学对齐 (V1 守门)
//! 19. cone_of_truth_rate        - 真理锥率
//! 22. abstraction_level         - 抽象层级
//! 23. analogy_quality           - 类比质量
//!
//! 修 Bug: 跟 council_advise 一样, model 字段必须用真 model 名
//! 协议层: 用 apeireth-api::llm::LlmProvider 抽象, 接 minimaxi / OpenAI / 等

use std::sync::Arc;

use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use serde::{Deserialize, Serialize};

/// 6 个 LLM-judge 维度 (语义判断)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmJudgeDim {
    /// 11: 核心价值一致性
    CoreValuesConsistency,
    /// 12: 声音一致性
    VoiceConsistency,
    /// 15: 12 键哲学对齐 (V1 守门)
    PhilosophyAlignment,
    /// 19: 真理锥率
    ConeOfTruthRate,
    /// 22: 抽象层级
    AbstractionLevel,
    /// 23: 类比质量
    AnalogyQuality,
}

impl LlmJudgeDim {
    /// dim 编号 (V0.5 24 维)
    pub fn index(&self) -> u8 {
        match self {
            Self::CoreValuesConsistency => 11,
            Self::VoiceConsistency => 12,
            Self::PhilosophyAlignment => 15,
            Self::ConeOfTruthRate => 19,
            Self::AbstractionLevel => 22,
            Self::AnalogyQuality => 23,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::CoreValuesConsistency => "core_values_consistency",
            Self::VoiceConsistency => "voice_consistency",
            Self::PhilosophyAlignment => "philosophy_alignment",
            Self::ConeOfTruthRate => "cone_of_truth_rate",
            Self::AbstractionLevel => "abstraction_level",
            Self::AnalogyQuality => "analogy_quality",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            Self::CoreValuesConsistency =>
                "你是 apeireth-asi V0.5 维 11 评估器. 给定一段 LLM 输出, 评估是否与核心价值 (不假装/实事求是/12 键哲学) 一致. 输出 0.0-1.0 分数 + 简短理由. 中文.",
            Self::VoiceConsistency =>
                "你是 apeireth-asi V0.5 维 12 评估器. 给定一段 LLM 输出, 评估声音一致性 (风格/语气/格式统一). 输出 0.0-1.0 分数 + 简短理由. 中文.",
            Self::PhilosophyAlignment =>
                "你是 apeireth-asi V0.5 维 15 评估器. 评估输出是否符合 12 键哲学 (主 17:58 不假装 / 主 20:46 隐喻是工具 / 主 22:33 ASI 北极星). 输出 0.0-1.0 分数 + 简短理由. 中文.",
            Self::ConeOfTruthRate =>
                "你是 apeireth-asi V0.5 维 19 评估器. 评估输出是否在真理锥内 (事实正确 / 不模糊). 输出 0.0-1.0 分数 + 简短理由. 中文.",
            Self::AbstractionLevel =>
                "你是 apeireth-asi V0.5 维 22 评估器. 评估输出的抽象层级 (0=具体 1=高度抽象). 输出 0.0-1.0 分数 + 简短理由. 中文.",
            Self::AnalogyQuality =>
                "你是 apeireth-asi V0.5 维 23 评估器. 评估输出中类比的质量 (贴切度/解释力). 输出 0.0-1.0 分数 + 简短理由. 中文.",
        }
    }
}

/// LLM judge 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    pub dim: LlmJudgeDim,
    pub score: f64,        // 0.0-1.0
    pub reasoning: String, // LLM 解释
    pub model: String,
    pub latency_ms: u64,
}

/// 用 LLM 评估一个维度的得分
pub async fn judge(
    llm: &Arc<dyn LlmProvider>,
    dim: LlmJudgeDim,
    output: &str,
) -> Result<JudgeResult, String> {
    // 修 Bug: 跟 council_advise 一样, model 字段必须用真 model 名
    let model = if llm.name() == "apeireth-api" {
        "MiniMax-M3".to_string()
    } else {
        llm.name().to_string()
    };

    let req = LlmRequest::new(
        &model,
        vec![
            ChatMessage::system(dim.system_prompt().to_string()),
            ChatMessage::user(format!("待评估输出:\n```\n{output}\n```")),
        ],
    )
    .with_temperature(0.2)
    .with_max_tokens(200);

    let resp = llm
        .complete(req)
        .await
        .map_err(|e| format!("LLM judge 失败: {e}"))?;

    // 解析分数 (从 LLM 输出中提取 0.0-1.0 数字)
    let score = parse_score(&resp.content);
    let reasoning = resp.content.clone();

    Ok(JudgeResult {
        dim,
        score,
        reasoning,
        model: resp.model,
        latency_ms: resp.latency_ms,
    })
}

/// 解析分数 (从 LLM 输出中找第一个 0.0-1.0 数字)
fn parse_score(content: &str) -> f64 {
    // 简单正则: 找第一个 0.0-1.0 数字
    for token in content.split(|c: char| !c.is_ascii_digit() && c != '.') {
        if let Ok(v) = token.parse::<f64>() {
            if (0.0..=1.0).contains(&v) {
                return v;
            }
        }
    }
    0.5 // 默认 (LLM 输出无数字时)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_api::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};

    #[test]
    fn test_dim_index() {
        assert_eq!(LlmJudgeDim::CoreValuesConsistency.index(), 11);
        assert_eq!(LlmJudgeDim::PhilosophyAlignment.index(), 15);
    }

    #[test]
    fn test_parse_score() {
        assert!((parse_score("0.85") - 0.85).abs() < 0.01);
        assert!((parse_score("分数: 0.7 因为...") - 0.7).abs() < 0.01);
        assert!((parse_score("无数字") - 0.5).abs() < 0.01);
        assert!((parse_score("1.5 超范围") - 0.5).abs() < 0.01); // 超范围返回默认
    }

    #[tokio::test]
    async fn test_judge_with_scripted() {
        let scripted = ScriptedLlmProvider::new("test-judge")
            .with_script("hi", ScriptedResponse::new("分数 0.85 因为输出内容好"));
        let llm: Arc<dyn LlmProvider> = Arc::new(scripted);
        let result = judge(&llm, LlmJudgeDim::CoreValuesConsistency, "hi")
            .await
            .unwrap();
        assert_eq!(result.dim, LlmJudgeDim::CoreValuesConsistency);
        assert!((result.score - 0.85).abs() < 0.01);
    }
}
