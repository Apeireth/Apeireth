use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HybridRoutingDecision {
    LocalFastPath {
        intent: String,
        confidence: f64,
        response_template: Option<String>,
    },
    CloudDeepPath {
        reason: String,
        requires_mcts: bool,
    },
}

pub struct HybridCognitiveRouter;

impl Default for HybridCognitiveRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridCognitiveRouter {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates user input and routes between local sub-5ms fast path and cloud frontier reasoning
    pub fn route(&self, input: &str) -> HybridRoutingDecision {
        let trimmed = input.trim().to_lowercase();

        // 1. Instant local fast paths - Health & Ping
        if trimmed == "ping" || trimmed == "health" || trimmed == "状态" || trimmed == "运行状态" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "system_health".into(),
                confidence: 0.99,
                response_template: Some("pong - Apeireth 2.0 Living Host Online (All systems operational)".into()),
            };
        }

        // 2. Instant local fast paths - Clock & Date
        if trimmed == "time" || trimmed == "几点了" || trimmed == "现在时间" || trimmed == "date" || trimmed == "今天几号" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "clock_query".into(),
                confidence: 0.98,
                response_template: Some(format!("当前时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))),
            };
        }

        // 3. Instant local fast paths - Self Identity
        if trimmed == "who are you" || trimmed == "你是谁" || trimmed == "介绍一下你自己" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "identity_self".into(),
                confidence: 0.95,
                response_template: Some("我是 Apeireth（阿派瑞斯），一个运行在你本地 Rust 微内核之上的自主认知伴侣操作系统。我具备自我升级、ACT-R 记忆反思、未来预测、辅助投资、辅助学习与物理屏幕感知 6 大原生核心能力。".into()),
            };
        }

        // 4. Instant local fast paths - Simple math expressions (e.g. "1+1", "100*20")
        if let Some(math_result) = Self::try_eval_simple_math(&trimmed) {
            return HybridRoutingDecision::LocalFastPath {
                intent: "math_eval".into(),
                confidence: 0.99,
                response_template: Some(format!("计算结果: {}", math_result)),
            };
        }

        // 5. Complex queries needing Deep Cloud LLM & MCTS Causal reasoning
        let requires_mcts = trimmed.contains("why") || trimmed.contains("how to") || trimmed.contains("design")
            || trimmed.contains("refactor") || trimmed.contains("为什么") || trimmed.contains("设计") || trimmed.contains("架构")
            || trimmed.contains("推演") || trimmed.contains("分析");

        HybridRoutingDecision::CloudDeepPath {
            reason: "Conversational reasoning / tool invocation required".into(),
            requires_mcts,
        }
    }

    fn try_eval_simple_math(expr: &str) -> Option<f64> {
        let clean: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
        if clean.is_empty() {
            return None;
        }

        for (op_idx, op) in ['+', '-', '*', '/'].iter().enumerate() {
            let parts: Vec<&str> = clean.split(*op).collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    let res = match op_idx {
                        0 => a + b,
                        1 => a - b,
                        2 => a * b,
                        3 => if b != 0.0 { a / b } else { return None },
                        _ => return None,
                    };
                    return Some(res);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_cognitive_routing() {
        let router = HybridCognitiveRouter::new();

        let r1 = router.route("ping");
        match r1 {
            HybridRoutingDecision::LocalFastPath { intent, .. } => assert_eq!(intent, "system_health"),
            _ => panic!("Expected local fast path for ping"),
        }

        let r2 = router.route("12 + 34");
        match r2 {
            HybridRoutingDecision::LocalFastPath { intent, response_template, .. } => {
                assert_eq!(intent, "math_eval");
                assert!(response_template.unwrap().contains("46"));
            }
            _ => panic!("Expected local math fast path"),
        }

        let r3 = router.route("How to design a distributed consensus protocol in Rust?");
        match r3 {
            HybridRoutingDecision::CloudDeepPath { requires_mcts, .. } => assert!(requires_mcts),
            _ => panic!("Expected cloud deep path for complex design question"),
        }
    }
}
