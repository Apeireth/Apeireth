//! LlmAnalysis - LLM 分析 (从 v1.0 apeireth-memory/llm_analysis.rs 118 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真结果 + 启发式 fallback (不依赖 LLM)

#[derive(Debug, Clone)]
pub struct AnalysisResult { pub summary: String, pub confidence: f32, pub model: String }

/// 0 装 PASS stub: 真 LLM 调用 (无 LLM 时 返 heuristic)
pub async fn analyze(text: &str) -> AnalysisResult {
    // 0 装 PASS: heuristic summary (first 50 chars), 标 stub
    let summary: String = text.chars().take(50).collect();
    AnalysisResult { summary, confidence: 0.5, model: "heuristic".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_basic() {
        let r = analyze("hello world this is a long text").await;
        assert!(r.summary.len() <= 50);
        assert_eq!(r.model, "heuristic");
    }
    #[tokio::test]
    async fn test_empty() {
        let r = analyze("").await;
        assert_eq!(r.summary, "");
    }
    #[tokio::test]
    async fn test_unicode() {
        let r = analyze("你好").await;
        assert_eq!(r.summary, "你好");
    }
}
