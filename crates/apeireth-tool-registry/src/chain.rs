//! **VCP 新版 `dynamicToolRegistry.js:986-1000 _classifyRecord` — 分类四级降级链**
//!
//! **目标**: 工具分类按"先准后廉"顺序降级:
//! 1. **自定义** — 用户显式映射 (0 远程, 确定性最高)
//! 2. **小模型** — LLM 分类 (`LlmClassifier` 等, trait 口注入)
//! 3. **RAG** — 嵌入相似度 (`EmbeddingClassifier` 等, trait 口注入)
//! 4. **关键词** — 启发式字典兜底 (`HeuristicClassifier`, 已实装, 0 远程)
//!
//! 任一级返 `Ok` 即定案并记录 `ClassifyStage`; 返 `Err` (NoMatch/模型错/嵌入错)
//! 降下一级. 全部失败 → `Err(ClassifyError::NoMatch)` (0 假装"分到了某类").
//!
//! **VCP 字段级引用** `dynamicToolRegistry.js:986-1000 _classifyRecord`:
//! - `descriptionOverride` (用户显式分类) → 我们 Custom 级
//! - `_classifyWithSmallModel` → 我们 SmallModel 级
//! - `_classifyWithEmbeddings` → 我们 Rag 级
//! - `_fallbackClassify` (关键词兜底) → 我们 Keyword 级
//!
//! **不假装** (0 装 PASS):
//! - ✅ Keyword 级真实装 (`HeuristicClassifier`, 关键词字典 1:1 抄 VCP)
//! - ✅ Custom 级真实装 (`CustomMapClassifier`, name → Category 映射表)
//! - ❌ SmallModel / Rag 级**未接真模型** — 仅 trait 注入口 (`Option<Arc<dyn Classifier>>`),
//!   `has_small_model()` / `has_rag()` 如实报告接入状态, 不假装已接
//!
//! **挂接**: `ClassifyChain` 自身实现 `Classifier` trait, 可直接传给
//! `ToolRegistry::register_with_classifier`.

use std::sync::Arc;

use crate::classifier::{Category, ClassifyError, Classifier, HeuristicClassifier};
use crate::trait_def::Tool;

/// 分类四级降级链级数 (编译期 hardcode)
pub const CHAIN_LEVELS: usize = 4;

/// **四级降级链的决定级** (哪个级别定案的)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifyStage {
    /// 自定义映射定案
    Custom,
    /// 小模型定案
    SmallModel,
    /// RAG (嵌入相似度) 定案
    Rag,
    /// 关键词兜底定案
    Keyword,
}

impl ClassifyStage {
    /// 级别名 (日志/遥测用)
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::SmallModel => "small_model",
            Self::Rag => "rag",
            Self::Keyword => "keyword",
        }
    }
}

/// **带决定级的分类结果**
#[derive(Debug, Clone)]
pub struct ClassifyOutcome {
    /// 分类结果
    pub category: Category,
    /// 哪一级定案
    pub stage: ClassifyStage,
    /// 定案级的置信度 (0.0..1.0)
    pub confidence: f32,
}

/// **分类四级降级链**: 自定义 → 小模型 → RAG → 关键词
///
/// 后三级为 trait 注入口 (`Option<Arc<dyn Classifier>>`), 关键词级实装兜底.
/// 自身实现 `Classifier`, 可直接给 `register_with_classifier` 用.
pub struct ClassifyChain {
    /// 自定义级 (如 `CustomMapClassifier`)
    custom: Option<Arc<dyn Classifier>>,
    /// 小模型级 (如 `LlmClassifier`, 真接留后续)
    small_model: Option<Arc<dyn Classifier>>,
    /// RAG 级 (如 `EmbeddingClassifier`, 真接留后续)
    rag: Option<Arc<dyn Classifier>>,
    /// 关键词级 (实装兜底, 0 远程)
    keyword: HeuristicClassifier,
}

impl Default for ClassifyChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassifyChain {
    /// 新建四级链 — **仅关键词级接入** (0 装: 小模型/RAG 未接, 如实标 None)
    pub fn new() -> Self {
        Self {
            custom: None,
            small_model: None,
            rag: None,
            keyword: HeuristicClassifier::new(),
        }
    }

    /// 注入自定义级分类器
    pub fn with_custom(mut self, classifier: Arc<dyn Classifier>) -> Self {
        self.custom = Some(classifier);
        self
    }

    /// 注入小模型级分类器
    pub fn with_small_model(mut self, classifier: Arc<dyn Classifier>) -> Self {
        self.small_model = Some(classifier);
        self
    }

    /// 注入 RAG 级分类器
    pub fn with_rag(mut self, classifier: Arc<dyn Classifier>) -> Self {
        self.rag = Some(classifier);
        self
    }

    /// 自定义级是否已接 (如实报告)
    pub fn has_custom(&self) -> bool {
        self.custom.is_some()
    }

    /// 小模型级是否已接 (如实报告, 未接真模型时 false)
    pub fn has_small_model(&self) -> bool {
        self.small_model.is_some()
    }

    /// RAG 级是否已接 (如实报告, 未接真模型时 false)
    pub fn has_rag(&self) -> bool {
        self.rag.is_some()
    }

    /// **四级降级分类** — 自定义 → 小模型 → RAG → 关键词
    ///
    /// 任一级 `Ok` 即定案 (记录级); `Err` 降级. 全失败返 `NoMatch`.
    pub fn classify_staged(&self, tool: &dyn Tool) -> Result<ClassifyOutcome, ClassifyError> {
        let stages: [(ClassifyStage, &Option<Arc<dyn Classifier>>); 3] = [
            (ClassifyStage::Custom, &self.custom),
            (ClassifyStage::SmallModel, &self.small_model),
            (ClassifyStage::Rag, &self.rag),
        ];
        for (stage, slot) in stages {
            if let Some(classifier) = slot {
                if let Ok(category) = classifier.classify(tool) {
                    let confidence = classifier.confidence(tool).unwrap_or(1.0);
                    return Ok(ClassifyOutcome {
                        category,
                        stage,
                        confidence,
                    });
                }
                // Err → 降下一级 (VCP fallback 行为)
            }
        }
        // 关键词兜底 (NoMatch 透传 = 0 假装)
        let category = self.keyword.classify(tool)?;
        let confidence = self.keyword.confidence(tool).unwrap_or(0.0);
        Ok(ClassifyOutcome {
            category,
            stage: ClassifyStage::Keyword,
            confidence,
        })
    }
}

impl Classifier for ClassifyChain {
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError> {
        self.classify_staged(tool).map(|o| o.category)
    }

    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError> {
        self.classify_staged(tool).map(|o| o.confidence)
    }
}

/// **自定义级实装** — name → Category 映射表 (0 远程, 确定性)
///
/// 用户显式指定分类 (对应 VCP `descriptionOverride`), 未命中返 NoMatch 供链降级.
#[derive(Debug, Default)]
pub struct CustomMapClassifier {
    map: std::collections::HashMap<String, Category>,
}

impl CustomMapClassifier {
    /// 新建空映射表
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入一条 name → Category 映射 (覆盖同名)
    pub fn insert(&mut self, name: impl Into<String>, category: Category) {
        self.map.insert(name.into(), category);
    }

    /// 映射条数
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl Classifier for CustomMapClassifier {
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError> {
        self.map.get(tool.name()).copied().ok_or_else(|| {
            ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: 0,
            }
        })
    }

    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError> {
        if self.map.contains_key(tool.name()) {
            Ok(1.0) // 用户显式指定 = 满置信
        } else {
            Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: 0,
            })
        }
    }
}

// ============================================================
// 测试 (四级降级各路径)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AwaitingAxis, OutputAxis, ResidentAxis, ToolAxes, ToolKind, TransportAxis, TriggerAxis};
    use serde_json::Value;

    /// 测试用工具 (仅 name 有意义, classifier 只用 name)
    struct TestTool(String);

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.0
        }
        fn kind(&self) -> ToolKind {
            ToolKind::Sync
        }
        fn axes(&self) -> ToolAxes {
            ToolAxes {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Ephemeral,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            }
        }
        async fn call(&self, _args: Value) -> Result<Value, String> {
            Ok(Value::Null)
        }
    }

    /// 测试用注入级: 固定 Ok / 固定 Err
    struct FixedClassifier(Option<Category>);

    impl Classifier for FixedClassifier {
        fn classify(&self, _tool: &dyn Tool) -> Result<Category, ClassifyError> {
            self.0.ok_or(ClassifyError::NoMatch {
                name: "fixed-mock".to_string(),
                tried_keywords: 0,
            })
        }
        fn confidence(&self, _tool: &dyn Tool) -> Result<f32, ClassifyError> {
            self.0.map(|_| 0.9).ok_or(ClassifyError::NoMatch {
                name: "fixed-mock".to_string(),
                tried_keywords: 0,
            })
        }
    }

    #[test]
    fn keyword_only_chain_classifies_by_heuristic() {
        // 默认链 (0 装): 仅关键词级接入
        let chain = ClassifyChain::new();
        let out = chain.classify_staged(&TestTool("WebSearch".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::Keyword);
        assert_eq!(out.category, Category::Search);
        assert!(out.confidence > 0.0);
    }

    #[test]
    fn new_chain_honestly_reports_unconnected_levels() {
        // 0 装 PASS: 未接的级如实标 false
        let chain = ClassifyChain::new();
        assert!(!chain.has_custom());
        assert!(!chain.has_small_model());
        assert!(!chain.has_rag());
    }

    #[test]
    fn custom_map_decides_at_custom_stage() {
        let mut custom = CustomMapClassifier::new();
        custom.insert("MySpecialTool", Category::Data);
        let chain = ClassifyChain::new().with_custom(Arc::new(custom));
        let out = chain.classify_staged(&TestTool("MySpecialTool".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::Custom);
        assert_eq!(out.category, Category::Data);
        assert_eq!(out.confidence, 1.0);
    }

    #[test]
    fn custom_miss_falls_through_to_keyword() {
        // 自定义级未命中 → 降到关键词级
        let custom = CustomMapClassifier::new(); // 空表
        let chain = ClassifyChain::new().with_custom(Arc::new(custom));
        let out = chain.classify_staged(&TestTool("FileOperator".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::Keyword);
        assert_eq!(out.category, Category::FileCode);
    }

    #[test]
    fn small_model_slot_decides_when_custom_misses() {
        // 小模型级注入口: 固定返 Ok(ImageMedia)
        let chain = ClassifyChain::new()
            .with_custom(Arc::new(CustomMapClassifier::new())) // miss
            .with_small_model(Arc::new(FixedClassifier(Some(Category::ImageMedia))));
        let out = chain.classify_staged(&TestTool("Whatever".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::SmallModel);
        assert_eq!(out.category, Category::ImageMedia);
    }

    #[test]
    fn small_model_error_falls_through_to_rag() {
        // 小模型级 Err → 降到 RAG 级
        let chain = ClassifyChain::new()
            .with_small_model(Arc::new(FixedClassifier(None))) // Err
            .with_rag(Arc::new(FixedClassifier(Some(Category::Communication))));
        let out = chain.classify_staged(&TestTool("Whatever".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::Rag);
        assert_eq!(out.category, Category::Communication);
    }

    #[test]
    fn all_upper_levels_miss_keyword_decides() {
        // 三级注入全 Err → 关键词兜底
        let chain = ClassifyChain::new()
            .with_custom(Arc::new(FixedClassifier(None)))
            .with_small_model(Arc::new(FixedClassifier(None)))
            .with_rag(Arc::new(FixedClassifier(None)));
        let out = chain.classify_staged(&TestTool("NoteRecall".to_string())).unwrap();
        assert_eq!(out.stage, ClassifyStage::Keyword);
        // "noterecall" 含 "note"/"recall" → MemoryKnowledge
        assert_eq!(out.category, Category::MemoryKnowledge);
    }

    #[test]
    fn all_levels_miss_returns_no_match() {
        // 四级全 miss → NoMatch (0 假装分到某类)
        let chain = ClassifyChain::new()
            .with_custom(Arc::new(FixedClassifier(None)))
            .with_small_model(Arc::new(FixedClassifier(None)))
            .with_rag(Arc::new(FixedClassifier(None)));
        let err = chain
            .classify_staged(&TestTool("Xyzzy".to_string()))
            .unwrap_err();
        assert!(matches!(err, ClassifyError::NoMatch { .. }));
    }

    #[test]
    fn chain_implements_classifier_trait_for_registry() {
        // ClassifyChain 实现 Classifier → 可给 register_with_classifier 用
        fn assert_classifier<T: Classifier>() {}
        assert_classifier::<ClassifyChain>();
        assert_classifier::<CustomMapClassifier>();

        let chain = ClassifyChain::new();
        let tool = TestTool("WebSearch".to_string());
        let as_dyn: &dyn Classifier = &chain;
        assert_eq!(as_dyn.classify(&tool).unwrap(), Category::Search);
        assert!(as_dyn.confidence(&tool).unwrap() > 0.0);
    }

    #[test]
    fn chain_level_count_is_four() {
        assert_eq!(CHAIN_LEVELS, 4);
    }
}
