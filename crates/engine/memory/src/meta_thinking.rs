//! `apeireth-memory::meta_thinking` — 元思考递归链 (元自学习 / 认知多阶段推演).
//!
//! ## 核心哲学
//! AI 的深度推演不应是一锤子买卖，而是阶段化的“思考 $\to$ 再思考”：
//! 上一阶段思考的产出作为下一阶段的上下文输入，形成递进深化的思维链。
//!
//! ## 机制与防御
//! - [`MetaThinker`] trait: 单步思考抽象注入点 (业务层/LLM 部署层实现)；
//! - [`MetaThinkingChain`]: 阶段化推演引擎；
//!   - **深度上限保护** (`max_depth`, 默认 10): 杜绝失控无限递归；
//!   - **循环思考防护** (`CycleDetected`): 发现思考产出与既往阶段完全一致时立即熔断；
//!   - **空思考降级** (`degraded`): 产出空白时标定降级并继续推进后续阶段；
//!   - **思考器异常熔断** (`ThinkerHalted`): 单阶段出错记录审计证据并停止链；
//! - [`save_to_cluster`]: 将结构化 Markdown 报告落盘至 N4 思维簇；
//! - [`ReflectionMetaThinker`] + [`ChainReflectionThinker`]: 反思周期挂接适配器；
//! - 纯 Safe Rust 零未定义行为，0 外部不可信 C-FFI 依赖。

#![deny(unsafe_code)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::thought_cluster::{ThoughtClusterError, ThoughtClusterManager, ThoughtClusterReader};

/// 默认最大思考深度 (防无限递归).
pub const DEFAULT_MAX_DEPTH: usize = 10;

/// 元思考错误枚举.
#[derive(Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaThinkError {
    #[error("元思考链阶段配置为空")]
    EmptyChain,
    #[error("初始查询为空")]
    EmptyQuery,
    #[error("非法最大深度: {0} (必须 >= 1)")]
    InvalidDepth(usize),
    #[error("思考器执行失败: {0}")]
    Thinker(String),
}

/// 一步思考的输入上下文.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaThinkInput {
    /// 阶段序号 (从 1 起始).
    pub stage: usize,
    /// 当前阶段所属簇名.
    pub cluster: String,
    /// 初始查询输入 (跨阶段保留).
    pub query: String,
    /// 当前阶段簇上下文.
    pub cluster_context: String,
    /// 上一阶段的思考产出 (首阶段为 None) — 递归递进载体.
    pub previous_thought: Option<String>,
}

/// 一步思考的输出.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaThinkOutput {
    /// 思考文本.
    pub thought: String,
}

impl MetaThinkOutput {
    pub fn new(thought: impl Into<String>) -> Self {
        Self {
            thought: thought.into(),
        }
    }
}

/// 单步思考器抽象 (LLM 或规则推理器实现).
pub trait MetaThinker: Send + Sync {
    /// 根据当前阶段上下文产出一段思考.
    fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError>;
}

/// 链阶段定义 (与簇名绑定).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainStage {
    pub cluster: String,
}

/// 停链原因留痕.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// 全链所有阶段正常完成.
    Completed,
    /// 达到深度上限截断.
    DepthLimitReached,
    /// 思考产出与既往阶段重复 (检测到思维死循环并熔断).
    CycleDetected,
    /// 思考器报错熔断.
    ThinkerHalted,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::DepthLimitReached => "depth_limit_reached",
            Self::CycleDetected => "cycle_detected",
            Self::ThinkerHalted => "thinker_halted",
        }
    }
}

/// 单阶段执行明细.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: usize,
    pub cluster: String,
    pub thought: String,
    pub degraded: bool,
    pub error: Option<String>,
}

/// 链执行总结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaChainResult {
    pub stages: Vec<StageResult>,
    pub final_thought: Option<String>,
    pub stop_reason: StopReason,
    pub truncated: bool,
}

impl MetaChainResult {
    /// 导出为 Markdown 格式报告.
    pub fn to_markdown(&self) -> String {
        let path: Vec<&str> = self.stages.iter().map(|s| s.cluster.as_str()).collect();
        let mut out = String::new();
        out.push_str("[--- 元思考链 ---]\n");
        out.push_str(&format!(
            "[推理链路径: {} | 停止原因: {}]\n\n",
            path.join(" → "),
            self.stop_reason.as_str()
        ));
        for s in &self.stages {
            out.push_str(&format!("【阶段{}: {}】", s.stage, s.cluster));
            if s.degraded {
                out.push_str(" [降级模式]\n");
            } else {
                out.push('\n');
            }
            if let Some(err) = &s.error {
                out.push_str(&format!("  [错误: {err}]\n"));
            } else if s.thought.trim().is_empty() {
                out.push_str("  [空思考]\n");
            } else {
                out.push_str(&format!("{}\n", s.thought.trim()));
            }
            out.push('\n');
        }
        out.push_str("[--- 元思考链结束 ---]\n");
        out
    }
}

/// 元思考递归链执行引擎.
pub struct MetaThinkingChain {
    stages: Vec<ChainStage>,
    max_depth: usize,
    reader: Option<Arc<dyn ThoughtClusterReader>>,
}

impl MetaThinkingChain {
    /// 构造元思考链.
    pub fn new(clusters: &[&str], max_depth: usize) -> Self {
        Self {
            stages: clusters
                .iter()
                .map(|c| ChainStage {
                    cluster: c.trim().to_string(),
                })
                .collect(),
            max_depth,
            reader: None,
        }
    }

    /// 注入思维簇读取器以获取历史簇上下文.
    pub fn with_reader(mut self, reader: Arc<dyn ThoughtClusterReader>) -> Self {
        self.reader = Some(reader);
        self
    }

    /// 执行递归思考链.
    pub fn run(
        &self,
        query: &str,
        thinker: &dyn MetaThinker,
    ) -> Result<MetaChainResult, MetaThinkError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(MetaThinkError::EmptyQuery);
        }
        if self.stages.is_empty() {
            return Err(MetaThinkError::EmptyChain);
        }
        if self.max_depth == 0 {
            return Err(MetaThinkError::InvalidDepth(self.max_depth));
        }

        let truncated = self.stages.len() > self.max_depth;
        let limit = self.stages.len().min(self.max_depth);

        let mut results: Vec<StageResult> = Vec::new();
        let mut previous_thought: Option<String> = None;
        let mut seen: HashSet<String> = HashSet::new();
        let mut stop_reason = if truncated {
            StopReason::DepthLimitReached
        } else {
            StopReason::Completed
        };

        for (i, stage) in self.stages.iter().take(limit).enumerate() {
            let stage_no = i + 1;
            let cluster_context = self
                .reader
                .as_ref()
                .map(|r| {
                    r.read_cluster(&stage.cluster)
                        .into_iter()
                        .map(|f| f.content)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();

            let input = MetaThinkInput {
                stage: stage_no,
                cluster: stage.cluster.clone(),
                query: query.to_string(),
                cluster_context,
                previous_thought: previous_thought.clone(),
            };

            match thinker.think(&input) {
                Err(e) => {
                    results.push(StageResult {
                        stage: stage_no,
                        cluster: stage.cluster.clone(),
                        thought: String::new(),
                        degraded: false,
                        error: Some(e.to_string()),
                    });
                    stop_reason = StopReason::ThinkerHalted;
                    break;
                }
                Ok(out) => {
                    let thought = out.thought.trim().to_string();
                    if thought.is_empty() {
                        results.push(StageResult {
                            stage: stage_no,
                            cluster: stage.cluster.clone(),
                            thought: String::new(),
                            degraded: true,
                            error: None,
                        });
                        continue;
                    }
                    if seen.contains(&thought) {
                        results.push(StageResult {
                            stage: stage_no,
                            cluster: stage.cluster.clone(),
                            thought: thought.clone(),
                            degraded: false,
                            error: Some("cycle detected: thought repeats a previous stage".into()),
                        });
                        stop_reason = StopReason::CycleDetected;
                        break;
                    }
                    seen.insert(thought.clone());
                    results.push(StageResult {
                        stage: stage_no,
                        cluster: stage.cluster.clone(),
                        thought: thought.clone(),
                        degraded: false,
                        error: None,
                    });
                    previous_thought = Some(thought);
                }
            }
        }

        let final_thought = results
            .iter()
            .rev()
            .find(|s| !s.thought.trim().is_empty())
            .map(|s| s.thought.clone());

        Ok(MetaChainResult {
            stages: results,
            final_thought,
            stop_reason,
            truncated,
        })
    }
}

/// 将元思考链报告落盘保存到指定思维簇.
pub fn save_to_cluster(
    manager: &ThoughtClusterManager,
    cluster: &str,
    result: &MetaChainResult,
) -> Result<PathBuf, ThoughtClusterError> {
    manager.create_file(cluster, &result.to_markdown())
}

/// 反思挂接点抽象.
pub trait ReflectionMetaThinker: Send + Sync {
    /// 反思上下文 $\to$ 元思考 Markdown 报告.
    fn meta_reflect(&self, reflection_context: &str) -> Result<String, MetaThinkError>;
}

/// 链式元思考适配器.
pub struct ChainReflectionThinker {
    chain: MetaThinkingChain,
    thinker: Arc<dyn MetaThinker>,
}

impl ChainReflectionThinker {
    pub fn new(chain: MetaThinkingChain, thinker: Arc<dyn MetaThinker>) -> Self {
        Self { chain, thinker }
    }
}

impl ReflectionMetaThinker for ChainReflectionThinker {
    fn meta_reflect(&self, reflection_context: &str) -> Result<String, MetaThinkError> {
        let result = self.chain.run(reflection_context, self.thinker.as_ref())?;
        Ok(result.to_markdown())
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleEchoThinker;
    impl MetaThinker for SimpleEchoThinker {
        fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            let prev = input.previous_thought.as_deref().unwrap_or("none");
            Ok(MetaThinkOutput::new(format!(
                "Stage {}: prev=[{}] query=[{}] cluster=[{}]",
                input.stage, prev, input.query, input.cluster
            )))
        }
    }

    struct FailingThinker;
    impl MetaThinker for FailingThinker {
        fn think(&self, _input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            Err(MetaThinkError::Thinker("LLM timeout".into()))
        }
    }

    struct RepeatThinker;
    impl MetaThinker for RepeatThinker {
        fn think(&self, _input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            Ok(MetaThinkOutput::new("constant thought"))
        }
    }

    struct EmptyDegradingThinker;
    impl MetaThinker for EmptyDegradingThinker {
        fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            if input.stage == 1 {
                Ok(MetaThinkOutput::new(""))
            } else {
                Ok(MetaThinkOutput::new("stage 2 valid thought"))
            }
        }
    }

    #[test]
    fn empty_query_and_empty_chain_rejected() {
        let chain = MetaThinkingChain::new(&["前思维簇"], 5);
        assert_eq!(
            chain.run("", &SimpleEchoThinker).unwrap_err(),
            MetaThinkError::EmptyQuery
        );

        let empty_chain = MetaThinkingChain::new(&[], 5);
        assert_eq!(
            empty_chain.run("query", &SimpleEchoThinker).unwrap_err(),
            MetaThinkError::EmptyChain
        );

        let zero_depth_chain = MetaThinkingChain::new(&["前思维簇"], 0);
        assert_eq!(
            zero_depth_chain.run("query", &SimpleEchoThinker).unwrap_err(),
            MetaThinkError::InvalidDepth(0)
        );
    }

    #[test]
    fn multistage_chain_passes_previous_thought_correctly() {
        let chain = MetaThinkingChain::new(&["前思维簇", "中间推演簇", "总结簇"], 5);
        let result = chain.run("优化算法", &SimpleEchoThinker).unwrap();

        assert_eq!(result.stages.len(), 3);
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert!(!result.truncated);

        assert!(result.stages[0].thought.contains("prev=[none]"));
        assert!(result.stages[1].thought.contains("prev=[Stage 1:"));
        assert!(result.stages[2].thought.contains("prev=[Stage 2:"));
        assert!(result.final_thought.is_some());
    }

    #[test]
    fn depth_limit_truncates_execution() {
        let chain = MetaThinkingChain::new(&["簇1", "簇2", "簇3", "簇4"], 2);
        let result = chain.run("测试", &SimpleEchoThinker).unwrap();

        assert_eq!(result.stages.len(), 2);
        assert_eq!(result.stop_reason, StopReason::DepthLimitReached);
        assert!(result.truncated);
    }

    #[test]
    fn cycle_detection_halts_chain() {
        let chain = MetaThinkingChain::new(&["簇1", "簇2", "簇3"], 5);
        let result = chain.run("循环测试", &RepeatThinker).unwrap();

        assert_eq!(result.stages.len(), 2);
        assert_eq!(result.stop_reason, StopReason::CycleDetected);
        assert!(result.stages[1].error.is_some());
    }

    #[test]
    fn thinker_failure_halts_chain() {
        let chain = MetaThinkingChain::new(&["簇1", "簇2"], 5);
        let result = chain.run("失败测试", &FailingThinker).unwrap();

        assert_eq!(result.stages.len(), 1);
        assert_eq!(result.stop_reason, StopReason::ThinkerHalted);
        assert!(result.stages[0].error.as_ref().unwrap().contains("LLM timeout"));
    }

    #[test]
    fn empty_thought_degrades_and_continues() {
        let chain = MetaThinkingChain::new(&["簇1", "簇2"], 5);
        let result = chain.run("降级测试", &EmptyDegradingThinker).unwrap();

        assert_eq!(result.stages.len(), 2);
        assert!(result.stages[0].degraded);
        assert_eq!(result.stages[1].thought, "stage 2 valid thought");
        assert_eq!(result.stop_reason, StopReason::Completed);
    }

    #[test]
    fn to_markdown_renders_clean_audit_trace() {
        let chain = MetaThinkingChain::new(&["阶段A簇", "阶段B簇"], 5);
        let result = chain.run("架构分析", &SimpleEchoThinker).unwrap();
        let md = result.to_markdown();

        assert!(md.contains("[--- 元思考链 ---]"));
        assert!(md.contains("推理链路径: 阶段A簇 → 阶段B簇"));
        assert!(md.contains("【阶段1: 阶段A簇】"));
        assert!(md.contains("【阶段2: 阶段B簇】"));
        assert!(md.contains("[--- 元思考链结束 ---]"));
    }

    #[test]
    fn chain_reflection_thinker_adapts_cleanly() {
        let chain = MetaThinkingChain::new(&["阶段A簇"], 5);
        let adapter = ChainReflectionThinker::new(chain, Arc::new(SimpleEchoThinker));
        let md = adapter.meta_reflect("反思输入").unwrap();
        assert!(md.contains("[--- 元思考链 ---]"));
    }
}
