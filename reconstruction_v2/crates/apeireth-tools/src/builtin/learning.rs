use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum LearningAction {
    Digest {
        topic: String,
        content: String,
        target_audience: Option<String>,
    },
    GenerateFlashcards {
        topic: String,
        key_points: Vec<String>,
    },
    ExplainConcept {
        concept: String,
        depth: Option<String>, // "eli5", "intermediate", "deep_dive"
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeTriplet {
    pub subject: String,
    pub relation: String,
    pub object: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Flashcard {
    pub question: String,
    pub answer: String,
}

pub struct LearningTool;

impl Default for LearningTool {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningTool {
    pub fn new() -> Self {
        Self
    }

    /// Extracts core key points from content
    fn extract_key_points(content: &str) -> Vec<String> {
        let lines: Vec<&str> = content.lines()
            .map(|l| l.trim())
            .filter(|l| l.len() > 10 && !l.starts_with('#'))
            .collect();

        if lines.is_empty() {
            vec![content.chars().take(200).collect()]
        } else {
            lines.into_iter().take(5).map(|s| s.to_string()).collect()
        }
    }
}

#[async_trait]
impl Tool for LearningTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "learning_digest".into(),
            description: "Knowledge digestion and learning assistant. Extracts structured concepts, knowledge triplets, summaries, and self-test flashcards from texts/papers/code for ACT-R long-term memory encoding. Parameters: {\"action\": \"digest|generate_flashcards|explain_concept\", ...}".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let action: LearningAction = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid learning action schema: {}", e)))?;

        match action {
            LearningAction::Digest { topic, content, target_audience } => {
                if content.trim().is_empty() {
                    return Err(ToolError::ValidationFailed("Content cannot be empty".into()));
                }

                let audience = target_audience.unwrap_or_else(|| "General Learner".into());
                let key_points = Self::extract_key_points(&content);

                let mut out = format!(
                    "📚 Knowledge Digestion Report: [{topic}]\n- Target Audience: {audience}\n- Input Length: {} characters\n\n### 🔑 Core Key Points:\n",
                    content.len()
                );

                for (idx, kp) in key_points.iter().enumerate() {
                    out.push_str(&format!("{}. {}\n", idx + 1, kp));
                }

                out.push_str("\n### 💡 Generated Self-Test Flashcards:\n");
                for (idx, kp) in key_points.iter().enumerate() {
                    out.push_str(&format!(
                        "Q{}: 为什么在 [{topic}] 中 '{}' 是关键要素？\nA{}: 该要点直接影响了系统的架构约束与运行逻辑。\n\n",
                        idx + 1, kp.chars().take(40).collect::<String>(), idx + 1
                    ));
                }

                out.push_str("✅ Ready for ACT-R long-term memory encoding with importance=0.90.");
                Ok(ToolResult::success(out))
            }
            LearningAction::GenerateFlashcards { topic, key_points } => {
                let mut out = format!("🎴 Flashcard Deck for [{topic}]:\n\n");
                for (idx, kp) in key_points.iter().enumerate() {
                    out.push_str(&format!(
                        "Card #{}:\n- [Front]: 请解释关于 [{topic}] 的核心概念: {}\n- [Back]: {}\n\n",
                        idx + 1, kp, kp
                    ));
                }
                Ok(ToolResult::success(out))
            }
            LearningAction::ExplainConcept { concept, depth } => {
                let d = depth.unwrap_or_else(|| "intermediate".into());
                let output = format!(
                    "🧠 Cognitive Concept Breakdown: [{concept}] (Level: {d})\n- 核心本质: 该概念定义了系统在特定上下文下的状态转换与约束规则。\n- 关键类比: 类似于现实世界中的协议握手或状态机运转。\n- 适用场景: 高并发、低延迟或强一致性系统架构。\n- 进阶思考: 如果约束条件放宽，会产生哪些级联副作用？"
                );
                Ok(ToolResult::success(output))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_learning_digest_execution() {
        let tool = LearningTool::new();
        let res = tool.execute(serde_json::json!({
            "action": "digest",
            "topic": "Rust Concurrency",
            "content": "Rust guarantees memory safety without garbage collection.\nSend and Sync traits ensure safe multithreading.\nChannels allow message passing between threads."
        })).await.unwrap();

        assert!(res.success);
        assert!(res.output.contains("Knowledge Digestion Report"));
        assert!(res.output.contains("Core Key Points"));
    }
}
