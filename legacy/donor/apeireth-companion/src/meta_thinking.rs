//! `apeireth-companion::meta_thinking` — 元思考递归链 (§5.1 记忆域深化包机制③).
//!
//! **VCP MetaThinkingManager 吸收** (调研依据:
//! research/source/vcptoolbox/Plugin/RAGDiaryPlugin/MetaThinkingManager.js):
//! VCP 的元思考链 = 多阶段簇召回, 上一阶段召回结果的向量均值与原始查询按
//! [0.8, 0.2] 加权融合作为下一阶段查询向量 — 即「思考 → 再思考」:
//! 一段思考的产出成为下一段思考的输入。本模块把该机制抬到文本级 (0 embedding 依赖):
//!
//! - [`MetaThinker`] trait — 一步思考注入点 (真 LLM 实现留部署层)
//! - [`MetaThinkingChain`] — 阶段化执行: 每阶段把上一阶段思考产出拼入下一段输入;
//!   **深度上限** (max_depth, 默认 [`DEFAULT_MAX_DEPTH`]) 防无限递归;
//!   **循环防护** — 思考产出与既往阶段完全重复 → 熔断 (VCP 无此防护, 文本级必需);
//!   **空思考降级** — 产出空白 → 标 degraded 不融合继续 (VCP degraded 同款);
//!   **思考器失败熔断** — 单阶段 Err → 记录后停链 (VCP error → break 同款)。
//! - [`ReflectionMetaThinker`] — 反思挂接点 trait 口; [`ChainReflectionThinker`]
//!   是链式适配器。**实接线 reflection.rs 延后到 N14 修复后** (0 装 PASS)。
//! - 产物 = markdown 报告 ([`MetaChainResult::to_markdown`]), 格式对齐 N4
//!   [`crate::thought_cluster`] 思维簇 (VCP 报告格式同款), [`save_to_cluster`]
//!   一键落簇可存可回读。
//!
//! **0 假装**:
//! - reflection.rs 接线未做 (待 N14 编译阻塞解除, 见 backlog N14); 本模块自包含可测。
//! - 真 LLM MetaThinker 实现留部署层; 本 crate 只定义契约 + mock 确定性测试。
//! - 同步 trait (文本级机制无 IO); LLM 实现需要 async 时在部署层包 async→sync 边界。
//! - VCP 的 auto 主题切换/语义组增强/向量缓存不属本机制件 (归语义路由/记忆检索包)。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;

use crate::thought_cluster::{ThoughtClusterError, ThoughtClusterManager, ThoughtClusterReader};

/// 默认深度上限 (防无限递归; 可配但必须 ≥1)。
pub const DEFAULT_MAX_DEPTH: usize = 10;

/// 元思考错误 (诚实失败, 每个变体可行动)。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetaThinkError {
    /// 链定义为空 (无阶段可执行)。
    #[error("meta-thinking: chain has no stages")]
    EmptyChain,
    /// 初始查询为空。
    #[error("meta-thinking: query is empty")]
    EmptyQuery,
    /// 深度上限非法 (0)。
    #[error("meta-thinking: invalid max_depth {0} (must be >= 1)")]
    InvalidDepth(usize),
    /// 思考器失败 (附原因)。
    #[error("meta-thinking: thinker failed: {0}")]
    Thinker(String),
}

/// 一步思考的输入 (阶段上下文全集)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaThinkInput {
    /// 阶段序号 (1 起)。
    pub stage: usize,
    /// 本阶段簇名。
    pub cluster: String,
    /// 初始查询 (每阶段都在, 对应 VCP 权重 0.8 的原始查询分量)。
    pub query: String,
    /// 本阶段簇上下文 (由注入 reader 读簇文件拼接; 无 reader → 空)。
    pub cluster_context: String,
    /// 上一阶段思考产出 (首阶段为 None) — 「思考 → 再思考」的递归载体。
    pub previous_thought: Option<String>,
}

/// 一步思考的产出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaThinkOutput {
    /// 思考文本 (空白 = 空思考, 阶段降级)。
    pub thought: String,
}

/// 一步思考注入点 (真实现 = LLM 调用, 留部署层; mock 先行可测)。
pub trait MetaThinker: Send + Sync {
    /// 基于阶段上下文产出一段思考。
    fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError>;
}

/// 链阶段 (簇名 = 阶段名, 与 VCP clusters 数组 1:1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainStage {
    /// 簇名。
    pub cluster: String,
}

/// 停链原因 (审计留痕)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// 全链正常完成。
    Completed,
    /// 达到深度上限被截断 (还有未执行阶段)。
    DepthLimitReached,
    /// 思考产出与既往阶段重复 (循环防护熔断)。
    CycleDetected,
    /// 思考器报错熔断。
    ThinkerHalted,
}

impl StopReason {
    /// 机器可读字符串 (日志/审计)。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::DepthLimitReached => "depth_limit_reached",
            Self::CycleDetected => "cycle_detected",
            Self::ThinkerHalted => "thinker_halted",
        }
    }
}

/// 单阶段执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageResult {
    /// 阶段序号 (1 起)。
    pub stage: usize,
    /// 簇名。
    pub cluster: String,
    /// 思考产出 (空思考降级时为空串)。
    pub thought: String,
    /// 是否降级 (空思考, VCP degraded 同款)。
    pub degraded: bool,
    /// 思考器错误 (熔断时记录)。
    pub error: Option<String>,
}

/// 链执行结果 (完整审计信息)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaChainResult {
    /// 各阶段结果。
    pub stages: Vec<StageResult>,
    /// 最后一段有效思考 (全降级 → None)。
    pub final_thought: Option<String>,
    /// 停链原因。
    pub stop_reason: StopReason,
    /// 是否因深度上限截断。
    pub truncated: bool,
}

impl MetaChainResult {
    /// markdown 报告 (格式对齐 VCP _formatMetaThinkingResults + N4 思维簇可存)。
    pub fn to_markdown(&self) -> String {
        let path: Vec<&str> = self.stages.iter().map(|s| s.cluster.as_str()).collect();
        let mut out = String::new();
        out.push_str("[--- 元思考链 ---]\n");
        out.push_str(&format!(
            "[推理链路径: {} | 停止: {}]\n\n",
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

/// 元思考递归链机制件。
pub struct MetaThinkingChain {
    stages: Vec<ChainStage>,
    max_depth: usize,
    reader: Option<Arc<dyn ThoughtClusterReader>>,
}

impl MetaThinkingChain {
    /// 构造 (簇名列表 = 阶段序列; max_depth=0 → 运行期报 InvalidDepth)。
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

    /// 注入思维簇读取器 (阶段上下文 = 簇文件内容拼接; N4 机制复用, 不另立)。
    pub fn with_reader(mut self, reader: Arc<dyn ThoughtClusterReader>) -> Self {
        self.reader = Some(reader);
        self
    }

    /// 执行递归链: 思考 → 再思考, 上一段产出喂下一段输入。
    ///
    /// 防护: 深度上限截断 / 循环熔断 (产出重复) / 空思考降级 / 思考器错误熔断。
    /// 除输入校验错误外, 阶段级失败都记录在结果里返回 (0 静默吞错)。
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
                        // 空思考降级: 不融合, 继续 (VCP degraded 同款)
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
                        // 循环防护: 产出与既往阶段完全重复 → 熔断
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

/// 把链报告落进 N4 思维簇 (产物可存可回读; 簇名须符合簇规则, 以「簇」结尾)。
pub fn save_to_cluster(
    manager: &ThoughtClusterManager,
    cluster: &str,
    result: &MetaChainResult,
) -> Result<PathBuf, ThoughtClusterError> {
    manager.create_file(cluster, &result.to_markdown())
}

/// 反思挂接点 trait 口 (任务方向②): 对一段反思上下文做元思考 (思考的再思考)。
///
/// **0 装 PASS**: reflection.rs 实接线延后到 N14 编译阻塞解除;
/// 本 trait + [`ChainReflectionThinker`] 适配器即插即用。
pub trait ReflectionMetaThinker: Send + Sync {
    /// 反思上下文 → 元思考报告 (markdown)。
    fn meta_reflect(&self, reflection_context: &str) -> Result<String, MetaThinkError>;
}

/// 链式元思考适配器: 任意 MetaThinkingChain + MetaThinker → ReflectionMetaThinker。
pub struct ChainReflectionThinker {
    chain: MetaThinkingChain,
    thinker: Arc<dyn MetaThinker>,
}

impl ChainReflectionThinker {
    /// 构造。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thought_cluster::ThoughtFile;
    use chrono::TimeZone; // 补 import 解除 lib-test 编译阻塞 (编译器建议位, agent_orchestrator2 代加, 供主人知悉)
    use std::sync::Mutex;

    // ---------- mock 思考器 ----------

    /// 脚本思考器: 按阶段序号弹脚本产出; 记录收到的输入 (断言递归喂入)。
    struct ScriptedThinker {
        outputs: Mutex<std::collections::VecDeque<Result<String, String>>>,
        inputs: Mutex<Vec<MetaThinkInput>>,
    }

    impl ScriptedThinker {
        fn new(outputs: Vec<Result<String, String>>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into()),
                inputs: Mutex::new(Vec::new()),
            }
        }
        fn recorded(&self) -> Vec<MetaThinkInput> {
            self.inputs.lock().unwrap().clone()
        }
    }

    impl MetaThinker for ScriptedThinker {
        fn think(&self, input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            self.inputs.lock().unwrap().push(input.clone());
            let next = self
                .outputs
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err("script exhausted".to_string()));
            match next {
                Ok(t) => Ok(MetaThinkOutput { thought: t }),
                Err(e) => Err(MetaThinkError::Thinker(e)),
            }
        }
    }

    /// 常量思考器: 永远返回同一句 (循环防护测试)。
    struct ConstThinker(String);

    impl MetaThinker for ConstThinker {
        fn think(&self, _input: &MetaThinkInput) -> Result<MetaThinkOutput, MetaThinkError> {
            Ok(MetaThinkOutput {
                thought: self.0.clone(),
            })
        }
    }

    /// 桩读取器: 固定簇内容。
    struct StubReader {
        content: String,
    }

    impl ThoughtClusterReader for StubReader {
        fn clusters(&self) -> Vec<String> {
            vec!["测试簇".into()]
        }
        fn read_cluster(&self, _name: &str) -> Vec<ThoughtFile> {
            vec![ThoughtFile {
                name: "2026-08-16-001.md".into(),
                content: self.content.clone(),
            }]
        }
        fn read_chain(&self, _name: &str) -> Vec<ThoughtFile> {
            Vec::new()
        }
    }

    // ---------- 正常路径: 思考 → 再思考 ----------

    #[test]
    fn chain_feeds_previous_thought_into_next_stage() {
        let chain = MetaThinkingChain::new(&["分析簇", "综合簇", "结论簇"], DEFAULT_MAX_DEPTH);
        let thinker = ScriptedThinker::new(vec![
            Ok("第一段思考".into()),
            Ok("第二段思考".into()),
            Ok("第三段思考".into()),
        ]);
        let result = chain.run("今天学到了什么?", &thinker).unwrap();
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert!(!result.truncated);
        assert_eq!(result.final_thought.as_deref(), Some("第三段思考"));

        let inputs = thinker.recorded();
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0].previous_thought, None); // 首段无前驱
        assert_eq!(inputs[1].previous_thought.as_deref(), Some("第一段思考"));
        assert_eq!(inputs[2].previous_thought.as_deref(), Some("第二段思考"));
        // 每阶段都携带原始查询 (VCP 0.8 权重的原始分量)
        assert!(inputs.iter().all(|i| i.query == "今天学到了什么?"));
        assert_eq!(inputs[1].stage, 2);
        assert_eq!(inputs[1].cluster, "综合簇");
    }

    // ---------- 深度上限 ----------

    #[test]
    fn depth_limit_truncates_chain() {
        let chain = MetaThinkingChain::new(&["a簇", "b簇", "c簇", "d簇", "e簇"], 2);
        let thinker = ScriptedThinker::new(vec![Ok("t1".into()), Ok("t2".into())]);
        let result = chain.run("q", &thinker).unwrap();
        assert!(result.truncated);
        assert_eq!(result.stop_reason, StopReason::DepthLimitReached);
        assert_eq!(result.stages.len(), 2); // 只跑 2 段, 后 3 段未执行
        assert_eq!(result.final_thought.as_deref(), Some("t2"));
    }

    #[test]
    fn zero_depth_rejected() {
        let chain = MetaThinkingChain::new(&["a簇"], 0);
        assert_eq!(
            chain.run("q", &ConstThinker("t".into())),
            Err(MetaThinkError::InvalidDepth(0))
        );
    }

    // ---------- 循环防护 ----------

    #[test]
    fn cycle_detection_breaks_on_repeated_thought() {
        let chain = MetaThinkingChain::new(&["a簇", "b簇", "c簇"], DEFAULT_MAX_DEPTH);
        let result = chain.run("q", &ConstThinker("同一句".into())).unwrap();
        assert_eq!(result.stop_reason, StopReason::CycleDetected);
        assert_eq!(result.stages.len(), 2); // 第 2 段重复即熔断, 第 3 段不执行
        assert!(result.stages[1].error.as_deref().unwrap().contains("cycle"));
    }

    // ---------- 空思考降级 ----------

    #[test]
    fn empty_thought_degrades_stage_and_continues_unfused() {
        let chain = MetaThinkingChain::new(&["a簇", "b簇"], DEFAULT_MAX_DEPTH);
        let thinker = ScriptedThinker::new(vec![Ok("   ".into()), Ok("第二段".into())]);
        let result = chain.run("q", &thinker).unwrap();
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert!(result.stages[0].degraded);
        assert_eq!(result.stages[0].thought, "");
        assert!(!result.stages[1].degraded);
        // 空思考不融合: 第二段输入 previous_thought 仍为 None
        assert_eq!(thinker.recorded()[1].previous_thought, None);
        assert_eq!(result.final_thought.as_deref(), Some("第二段"));
    }

    // ---------- 思考器失败熔断 ----------

    #[test]
    fn thinker_error_halts_chain_with_recorded_error() {
        let chain = MetaThinkingChain::new(&["a簇", "b簇", "c簇"], DEFAULT_MAX_DEPTH);
        let thinker = ScriptedThinker::new(vec![Ok("t1".into()), Err("上游超时".into())]);
        let result = chain.run("q", &thinker).unwrap();
        assert_eq!(result.stop_reason, StopReason::ThinkerHalted);
        assert_eq!(result.stages.len(), 2); // 第 3 段不执行
        assert!(result.stages[1]
            .error
            .as_deref()
            .unwrap()
            .contains("上游超时"));
        assert_eq!(result.final_thought.as_deref(), Some("t1")); // 熔断前有效产出保留
    }

    // ---------- 输入校验失败路径 ----------

    #[test]
    fn empty_chain_rejected() {
        let chain = MetaThinkingChain::new(&[], DEFAULT_MAX_DEPTH);
        assert_eq!(
            chain.run("q", &ConstThinker("t".into())),
            Err(MetaThinkError::EmptyChain)
        );
    }

    #[test]
    fn empty_query_rejected() {
        let chain = MetaThinkingChain::new(&["a簇"], DEFAULT_MAX_DEPTH);
        assert_eq!(
            chain.run("  ", &ConstThinker("t".into())),
            Err(MetaThinkError::EmptyQuery)
        );
    }

    // ---------- 簇上下文注入 (N4 reader 复用) ----------

    #[test]
    fn reader_supplies_cluster_context() {
        let chain = MetaThinkingChain::new(&["分析簇"], DEFAULT_MAX_DEPTH).with_reader(Arc::new(
            StubReader {
                content: "昨天的思考记录".into(),
            },
        ));
        let thinker = ScriptedThinker::new(vec![Ok("t1".into())]);
        chain.run("q", &thinker).unwrap();
        assert_eq!(thinker.recorded()[0].cluster_context, "昨天的思考记录");
    }

    // ---------- 产物格式 + N4 思维簇可存 ----------

    #[test]
    fn markdown_report_has_path_and_stage_markers() {
        let chain = MetaThinkingChain::new(&["分析簇", "结论簇"], DEFAULT_MAX_DEPTH);
        let thinker = ScriptedThinker::new(vec![Ok("t1".into()), Ok("t2".into())]);
        let result = chain.run("q", &thinker).unwrap();
        let md = result.to_markdown();
        assert!(md.contains("[--- 元思考链 ---]"));
        assert!(md.contains("[推理链路径: 分析簇 → 结论簇 | 停止: completed]"));
        assert!(md.contains("【阶段1: 分析簇】"));
        assert!(md.contains("【阶段2: 结论簇】"));
        assert!(md.contains("t1") && md.contains("t2"));
        assert!(md.contains("[--- 元思考链结束 ---]"));
    }

    #[test]
    fn save_to_cluster_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("apeireth_meta_thinking_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clock = Arc::new(apeireth_core::clock::VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
                .single()
                .unwrap(),
        ));
        let manager = ThoughtClusterManager::new(&dir, clock);
        let chain = MetaThinkingChain::new(&["分析簇"], DEFAULT_MAX_DEPTH);
        let result = chain
            .run("q", &ScriptedThinker::new(vec![Ok("t1".into())]))
            .unwrap();

        let path = save_to_cluster(&manager, "元思考簇", &result).unwrap();
        assert!(path.exists());
        let files = manager.read_cluster("元思考簇").unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("t1"));
        assert!(files[0].content.contains("[--- 元思考链结束 ---]"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_to_cluster_rejects_bad_cluster_name() {
        let dir =
            std::env::temp_dir().join(format!("apeireth_meta_thinking_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let clock = Arc::new(apeireth_core::clock::VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 8, 16, 6, 0, 0)
                .single()
                .unwrap(),
        ));
        let manager = ThoughtClusterManager::new(&dir, clock);
        let result = MetaChainResult {
            stages: Vec::new(),
            final_thought: None,
            stop_reason: StopReason::Completed,
            truncated: false,
        };
        // 非「簇」结尾 → N4 规则拒绝 (诚实失败)
        assert!(save_to_cluster(&manager, "not-a-cluster", &result).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- ReflectionMetaThinker 适配器 ----------

    #[test]
    fn chain_reflection_thinker_produces_report() {
        let chain = MetaThinkingChain::new(&["反思簇"], DEFAULT_MAX_DEPTH);
        let rt =
            ChainReflectionThinker::new(chain, Arc::new(ConstThinker("对反思的再思考".into())));
        let report = rt.meta_reflect("本周反思: 陪伴质量").unwrap();
        assert!(report.contains("对反思的再思考"));
        assert!(report.contains("[--- 元思考链 ---]"));
    }

    #[test]
    fn chain_reflection_thinker_propagates_validation_errors() {
        let chain = MetaThinkingChain::new(&[], DEFAULT_MAX_DEPTH);
        let rt = ChainReflectionThinker::new(chain, Arc::new(ConstThinker("t".into())));
        assert_eq!(rt.meta_reflect("ctx"), Err(MetaThinkError::EmptyChain));
    }
}
