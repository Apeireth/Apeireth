//! 多 AI 一致 — ≥3 不同 LLM trait + Rust mock provider
//!
//! **设计** (阶段 1 §18.6 + 阶段 2 D2):
//! - 关键操作需 ≥3 个不同 LLM 一致通过 (多 AI 一致)
//! - trait 抽象 — 不依赖外部 SDK (OpenAI / Anthropic / Ollama)
//! - Rust mock provider: 用 `AiProvider` trait + 内置 mock
//!
//! **硬约束**: 不模拟 HTTP 调用; 测试用 `MockAiProvider` 即可

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// AI 立场
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiStance {
    /// 同意 (-1.0..+1.0 中的 +1)
    Approve,
    /// 反对
    Reject,
    /// 弃权 (不计入一致)
    Abstain,
}

/// AI 提供商标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AiProviderId {
    /// provider 名称 (如 "gpt-4-mock" / "claude-mock" / "local-llm-mock")
    pub name: String,
    /// 模型版本
    pub version: String,
}

impl AiProviderId {
    /// 新建 provider ID
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// AI 表决
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiVerdict {
    /// provider ID
    pub provider: AiProviderId,
    /// 立场
    pub stance: AiStance,
    /// 置信度 (0.0 .. 1.0)
    pub confidence: f64,
    /// 推理理由
    pub rationale: String,
    /// 时间戳
    pub timestamp: i64,
}

/// 多 AI 表决结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiConsensus {
    /// 一致通过 (≥3 provider 都 Approve, 置信度平均 ≥ 0.7)
    Unanimous {
        providers: Vec<AiProviderId>,
        avg_confidence: f64,
    },
    /// 部分通过 (≥2 approve 但未达一致)
    Partial {
        approve: Vec<AiProviderId>,
        reject: Vec<AiProviderId>,
    },
    /// 拒绝 (≥2 reject)
    Rejected {
        reject: Vec<AiProviderId>,
        reason: String,
    },
    /// 票数不足 (<3 verdict)
    Insufficient { verdict_count: usize },
}

/// 多 AI 错误
#[derive(Debug, Error)]
pub enum MultiAiError {
    #[error("provider `{0}` already registered")]
    DuplicateProvider(String),
    #[error("need at least 3 distinct AI providers; have {0}")]
    NotEnoughProviders(usize),
}

/// AI Provider trait — 抽象 LLM 接口
///
/// **设计**: provider 在决策提交时被 `evaluate` 调用; 返回该 provider 对该决策的
/// `AiVerdict`。**不**模拟外部 HTTP — 调用方应通过 mock 实现测试, 或自行实现本地
/// inference provider (例: llama.cpp binding)。
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    /// Provider 标识
    fn id(&self) -> &AiProviderId;

    /// 对决策给出表决
    async fn evaluate(&self, decision_summary: &str) -> AiVerdict;
}

/// 多 AI 表决聚合器
pub struct MultiAiConsensus {
    providers: Vec<Box<dyn AiProvider>>,
}

impl std::fmt::Debug for MultiAiConsensus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiAiConsensus")
            .field("provider_count", &self.providers.len())
            .field(
                "providers",
                &self
                    .providers
                    .iter()
                    .map(|p| p.id().clone())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl MultiAiConsensus {
    /// 新建空聚合器
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// 注册 provider (重复 ID 报错)
    pub fn register(&mut self, provider: Box<dyn AiProvider>) -> Result<(), MultiAiError> {
        let id = provider.id().clone();
        if self.providers.iter().any(|p| p.id() == &id) {
            return Err(MultiAiError::DuplicateProvider(id.name));
        }
        self.providers.push(provider);
        Ok(())
    }

    /// 异步投票: 每个 provider 给一个 verdict
    pub async fn poll(&self, decision_summary: &str) -> Vec<AiVerdict> {
        let mut verdicts = Vec::with_capacity(self.providers.len());
        for p in &self.providers {
            verdicts.push(p.evaluate(decision_summary).await);
        }
        verdicts
    }

    /// 聚合 verdict 列表 → 共识
    pub fn aggregate(verdicts: &[AiVerdict]) -> AiConsensus {
        if verdicts.len() < 3 {
            return AiConsensus::Insufficient {
                verdict_count: verdicts.len(),
            };
        }
        let mut approve = Vec::new();
        let mut reject = Vec::new();
        let mut sum_conf = 0.0_f64;
        let mut count = 0;
        for v in verdicts {
            if matches!(v.stance, AiStance::Abstain) {
                continue;
            }
            sum_conf += v.confidence;
            count += 1;
            match v.stance {
                AiStance::Approve => approve.push(v.provider.clone()),
                AiStance::Reject => reject.push(v.provider.clone()),
                AiStance::Abstain => {}
            }
        }
        if reject.len() >= 2 {
            let n = reject.len();
            return AiConsensus::Rejected {
                reject,
                reason: format!("{n} 个 AI provider 反对"),
            };
        }
        if approve.len() >= 3 {
            let avg_confidence = if count > 0 {
                sum_conf / f64::from(count)
            } else {
                0.0
            };
            if avg_confidence >= 0.7 {
                return AiConsensus::Unanimous {
                    providers: approve,
                    avg_confidence,
                };
            }
        }
        AiConsensus::Partial { approve, reject }
    }
}

impl Default for MultiAiConsensus {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock AI Provider — 用于测试, 内置固定 verdict
pub struct MockAiProvider {
    id: AiProviderId,
    fixed_stance: AiStance,
    fixed_confidence: f64,
    rationale: String,
}

impl MockAiProvider {
    /// 新建 mock provider
    pub fn new(name: &str, stance: AiStance) -> Self {
        Self {
            id: AiProviderId::new(name, "mock-v1"),
            fixed_stance: stance,
            fixed_confidence: 0.85,
            rationale: format!("mock {name} 默认立场"),
        }
    }

    /// 自定义置信度
    pub fn with_confidence(mut self, c: f64) -> Self {
        self.fixed_confidence = c.clamp(0.0, 1.0);
        self
    }

    /// 自定义理由
    pub fn with_rationale(mut self, r: impl Into<String>) -> Self {
        self.rationale = r.into();
        self
    }
}

#[async_trait::async_trait]
impl AiProvider for MockAiProvider {
    fn id(&self) -> &AiProviderId {
        &self.id
    }

    async fn evaluate(&self, _decision_summary: &str) -> AiVerdict {
        AiVerdict {
            provider: self.id.clone(),
            stance: self.fixed_stance,
            confidence: self.fixed_confidence,
            rationale: self.rationale.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision_summary() -> String {
        "Modify the E-layer philosophy key PHL-04 to allow weak observability".to_string()
    }

    #[tokio::test]
    async fn three_ai_unanimous_approve() {
        let mut consensus = MultiAiConsensus::new();
        consensus
            .register(Box::new(MockAiProvider::new(
                "gpt4-mock",
                AiStance::Approve,
            )))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new(
                "claude-mock",
                AiStance::Approve,
            )))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new(
                "local-mock",
                AiStance::Approve,
            )))
            .unwrap();
        let verdicts = consensus.poll(&decision_summary()).await;
        assert_eq!(verdicts.len(), 3);
        match MultiAiConsensus::aggregate(&verdicts) {
            AiConsensus::Unanimous {
                providers,
                avg_confidence,
            } => {
                assert_eq!(providers.len(), 3);
                assert!(avg_confidence >= 0.7);
            }
            other => panic!("expected Unanimous, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multi_ai_rejected_when_two_reject() {
        let mut consensus = MultiAiConsensus::new();
        consensus
            .register(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new("b", AiStance::Reject)))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new("c", AiStance::Reject)))
            .unwrap();
        let verdicts = consensus.poll(&decision_summary()).await;
        assert!(matches!(
            MultiAiConsensus::aggregate(&verdicts),
            AiConsensus::Rejected { .. }
        ));
    }

    #[tokio::test]
    async fn multi_ai_partial_when_mixed() {
        let mut consensus = MultiAiConsensus::new();
        consensus
            .register(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new("b", AiStance::Approve)))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new("c", AiStance::Reject)))
            .unwrap();
        let verdicts = consensus.poll(&decision_summary()).await;
        // 2 approve + 1 reject → Partial (不是 Unanimous, 也不是 Rejected)
        match MultiAiConsensus::aggregate(&verdicts) {
            AiConsensus::Partial { approve, reject } => {
                assert_eq!(approve.len(), 2);
                assert_eq!(reject.len(), 1);
            }
            other => panic!("expected Partial, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multi_ai_insufficient_with_two_providers() {
        let mut consensus = MultiAiConsensus::new();
        consensus
            .register(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .unwrap();
        consensus
            .register(Box::new(MockAiProvider::new("b", AiStance::Approve)))
            .unwrap();
        let verdicts = consensus.poll(&decision_summary()).await;
        assert!(matches!(
            MultiAiConsensus::aggregate(&verdicts),
            AiConsensus::Insufficient { verdict_count: 2 }
        ));
    }

    #[test]
    fn multi_ai_rejects_duplicate_provider() {
        let mut consensus = MultiAiConsensus::new();
        consensus
            .register(Box::new(MockAiProvider::new("a", AiStance::Approve)))
            .unwrap();
        assert!(matches!(
            consensus.register(Box::new(MockAiProvider::new("a", AiStance::Reject))),
            Err(MultiAiError::DuplicateProvider(_))
        ));
    }
}
