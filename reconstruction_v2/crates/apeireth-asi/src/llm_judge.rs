//! R16-09 apeireth-asi LLM judge (6 语义维度 — v1 等价)
//!
//! **v2 适配**: v1 依赖 `apeireth-api::llm::LlmProvider`. v2 没有 apeireth-api crate.
//! 我们在 apeireth-asi 内部定义一个最小等价的 `LlmProvider` trait + `ScriptedLlmProvider`
//! (用于测试), 让 `judge()` 函数保持自包含可用。
//!
//! 6 维从 V0.5 24 维中挑出:
//! 11. core_values_consistency
//! 12. voice_consistency
//! 15. philosophy_alignment
//! 19. cone_of_truth_rate
//! 22. abstraction_level
//! 23. analogy_quality

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============ 本地 LlmProvider trait (替代 v1 apeireth-api::llm) ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage { pub role: String, pub content: String }
impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self { Self { role: "system".into(), content: content.into() } }
    pub fn user(content: impl Into<String>) -> Self { Self { role: "user".into(), content: content.into() } }
    pub fn assistant(content: impl Into<String>) -> Self { Self { role: "assistant".into(), content: content.into() } }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}
impl LlmRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self { model: model.into(), messages, temperature: None, max_tokens: None }
    }
    pub fn with_temperature(mut self, t: f64) -> Self { self.temperature = Some(t); self }
    pub fn with_max_tokens(mut self, n: u32) -> Self { self.max_tokens = Some(n); self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub latency_ms: u64,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("LLM provider error: {0}")]
    Provider(String),
    #[error("network error: {0}")]
    Network(String),
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    fn name(&self) -> &str;
}

// ============ ScriptedLlmProvider (测试双) ============

#[derive(Debug, Clone)]
pub struct ScriptedResponse {
    pub content: String,
    pub model: String,
    pub latency_ms: u64,
}
impl ScriptedResponse {
    pub fn new(content: impl Into<String>) -> Self { Self { content: content.into(), model: "scripted".into(), latency_ms: 0 } }
    pub fn with_model(mut self, m: impl Into<String>) -> Self { self.model = m.into(); self }
    pub fn with_latency(mut self, ms: u64) -> Self { self.latency_ms = ms; self }
}

pub struct ScriptedLlmProvider {
    name: String,
    scripts: std::collections::HashMap<String, ScriptedResponse>,
    default: Option<ScriptedResponse>,
}
impl ScriptedLlmProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), scripts: std::collections::HashMap::new(), default: None }
    }
    pub fn with_script(mut self, key: impl Into<String>, resp: ScriptedResponse) -> Self {
        self.scripts.insert(key.into(), resp); self
    }
    pub fn with_default(mut self, resp: ScriptedResponse) -> Self {
        self.default = Some(resp); self
    }
}

#[async_trait]
impl LlmProvider for ScriptedLlmProvider {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 找最后一个 user message 当 key
        let key = req.messages.iter().rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_default();
        if let Some(r) = self.scripts.get(&key) {
            return Ok(LlmResponse { content: r.content.clone(), model: r.model.clone(), latency_ms: r.latency_ms });
        }
        if let Some(d) = &self.default {
            return Ok(LlmResponse { content: d.content.clone(), model: d.model.clone(), latency_ms: d.latency_ms });
        }
        Err(LlmError::Provider(format!("no script for key: {}", key.chars().take(50).collect::<String>())))
    }
    fn name(&self) -> &str { &self.name }
}

// ============ LlmJudgeDim + JudgeResult ============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmJudgeDim {
    CoreValuesConsistency,
    VoiceConsistency,
    PhilosophyAlignment,
    ConeOfTruthRate,
    AbstractionLevel,
    AnalogyQuality,
}
impl LlmJudgeDim {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    pub dim: LlmJudgeDim,
    pub score: f64,
    pub reasoning: String,
    pub model: String,
    pub latency_ms: u64,
}

/// 用 LLM 评估一个维度的得分 (v1 judge 等价).
pub async fn judge(
    llm: &Arc<dyn LlmProvider>,
    dim: LlmJudgeDim,
    output: &str,
) -> Result<JudgeResult, String> {
    // 跟 v1 一样: model 字段必须用真 model 名, 不能用 "apeireth-api" 这样的 stub
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
    let resp = llm.complete(req).await
        .map_err(|e| format!("LLM judge 失败: {e}"))?;
    let score = parse_score(&resp.content);
    let reasoning = resp.content.clone();
    Ok(JudgeResult { dim, score, reasoning, model: resp.model, latency_ms: resp.latency_ms })
}

/// 解析分数 (从 LLM 输出中找第一个 0.0-1.0 数字).
pub fn parse_score(content: &str) -> f64 {
    for token in content.split(|c: char| !c.is_ascii_digit() && c != '.') {
        if let Ok(v) = token.parse::<f64>() {
            if (0.0..=1.0).contains(&v) { return v; }
        }
    }
    0.5 // 默认
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dim_index() {
        assert_eq!(LlmJudgeDim::CoreValuesConsistency.index(), 11);
        assert_eq!(LlmJudgeDim::VoiceConsistency.index(), 12);
        assert_eq!(LlmJudgeDim::PhilosophyAlignment.index(), 15);
        assert_eq!(LlmJudgeDim::ConeOfTruthRate.index(), 19);
        assert_eq!(LlmJudgeDim::AbstractionLevel.index(), 22);
        assert_eq!(LlmJudgeDim::AnalogyQuality.index(), 23);
    }
    #[test]
    fn dim_names_unique() {
        let names = [LlmJudgeDim::CoreValuesConsistency, LlmJudgeDim::VoiceConsistency, LlmJudgeDim::PhilosophyAlignment, LlmJudgeDim::ConeOfTruthRate, LlmJudgeDim::AbstractionLevel, LlmJudgeDim::AnalogyQuality];
        let mut strs: Vec<&str> = names.iter().map(|d| d.name()).collect();
        strs.sort(); strs.dedup();
        assert_eq!(strs.len(), 6);
    }
    #[test]
    fn system_prompts_non_empty() {
        for d in [LlmJudgeDim::CoreValuesConsistency, LlmJudgeDim::VoiceConsistency, LlmJudgeDim::PhilosophyAlignment, LlmJudgeDim::ConeOfTruthRate, LlmJudgeDim::AbstractionLevel, LlmJudgeDim::AnalogyQuality] {
            assert!(!d.system_prompt().is_empty());
        }
    }
    #[test]
    fn parse_score_basic() {
        assert!((parse_score("0.85") - 0.85).abs() < 0.01);
        assert!((parse_score("分数: 0.7 因为...") - 0.7).abs() < 0.01);
        assert!((parse_score("无数字") - 0.5).abs() < 0.01);
        assert!((parse_score("1.5 超范围") - 0.5).abs() < 0.01);
    }
    #[tokio::test]
    async fn judge_with_scripted() {
        // 用 default response (不依赖具体 key 匹配)
        let scripted = ScriptedLlmProvider::new("test-judge")
            .with_default(ScriptedResponse::new("分数 0.85 因为输出内容好"));
        let llm: Arc<dyn LlmProvider> = Arc::new(scripted);
        let r = judge(&llm, LlmJudgeDim::CoreValuesConsistency, "hi").await.unwrap();
        assert_eq!(r.dim, LlmJudgeDim::CoreValuesConsistency);
        assert!((r.score - 0.85).abs() < 0.01);
    }
    #[tokio::test]
    async fn judge_with_default() {
        let scripted = ScriptedLlmProvider::new("d")
            .with_default(ScriptedResponse::new("0.6"));
        let llm: Arc<dyn LlmProvider> = Arc::new(scripted);
        let r = judge(&llm, LlmJudgeDim::VoiceConsistency, "anything").await.unwrap();
        assert!((r.score - 0.6).abs() < 0.01);
    }
    #[tokio::test]
    async fn judge_no_match_returns_err() {
        let scripted = ScriptedLlmProvider::new("n");
        let llm: Arc<dyn LlmProvider> = Arc::new(scripted);
        let r = judge(&llm, LlmJudgeDim::AnalogyQuality, "anything").await;
        assert!(r.is_err());
    }
    #[tokio::test]
    async fn judge_records_latency() {
        let scripted = ScriptedLlmProvider::new("lat")
            .with_default(ScriptedResponse::new("0.5").with_latency(123));
        let llm: Arc<dyn LlmProvider> = Arc::new(scripted);
        let r = judge(&llm, LlmJudgeDim::AbstractionLevel, "k").await.unwrap();
        assert_eq!(r.latency_ms, 123);
    }
}
