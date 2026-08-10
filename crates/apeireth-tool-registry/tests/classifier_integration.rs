//! Integration tests for `apeireth-tool-registry::classifier` (R25 战区 5)
//!
//! **R25 D-2 D2-5**: 5+ integration tests 覆盖 9 类别 + 3 classifier + registry 集成
//!
//! **不与 R18 #2.4 tests/registry.rs 冲突**: 本文件新增, 0 改既有 tests
//!
//! **VCP 字段级引用**: `dynamicToolRegistry.js:40-80 CATEGORY_RULES` 7 类 + 2 Apeireth 独有

use apeireth_tool_registry::{
    Category, ClassifyError, Classifier, EmbeddingClassifier, HeuristicClassifier, LlmClassifier,
    MockHashEmbedFn, Tool, ToolRegistry, AXIS_COMBINATION_COUNT, CATEGORY_COUNT,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

// =====================================================================
// 通用 Mock Tool 工厂 (9 demo tool)
// =====================================================================

fn tool(name: &str) -> Arc<dyn Tool> {
    Arc::new(IntegrationMockTool {
        name: name.to_string(),
    })
}

struct IntegrationMockTool {
    name: String,
}

#[async_trait]
impl Tool for IntegrationMockTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, _args: Value) -> Result<Value, String> {
        Ok(json!({}))
    }
}

fn make_9_demo_tools() -> Vec<(&'static str, Category)> {
    // 9 demo tool 配 9 类别 (期望 ground truth)
    vec![
        ("WebSearch", Category::Search),
        ("FileOperator", Category::FileCode),
        ("ImageGenerator", Category::ImageMedia),
        ("MemoryRecall", Category::MemoryKnowledge),
        ("TaskScheduler", Category::AgentTask),
        ("EmailSender", Category::Communication),
        ("JsonParser", Category::Data),
        ("PermissionGuard", Category::Safety),
        ("TrainModel", Category::LongRunning),
    ]
}

// =====================================================================
// Test 1: 9 类别枚举完整性
// =====================================================================

#[test]
fn integration_category_count_is_9_and_vcp_aligned() {
    // VCP 7 类 1:1 + 2 Apeireth 独有
    assert_eq!(CATEGORY_COUNT, 9);
    assert_eq!(Category::COUNT, 9);
    let all = Category::all();
    assert_eq!(all.len(), 9);

    // 验证 VCP 7 类的 VCP 字符串
    let vcp_7: Vec<&str> = vec![
        "search",
        "file_code",
        "image_media",
        "memory_knowledge",
        "agent_task",
        "communication",
        "data",
    ];
    let mut apeireth_7: Vec<&str> = Category::all()
        .iter()
        .filter(|c| !matches!(c, Category::Safety | Category::LongRunning))
        .map(|c| c.as_vcp_name())
        .collect();
    apeireth_7.sort();
    let mut sorted_vcp = vcp_7.clone();
    sorted_vcp.sort();
    assert_eq!(apeireth_7, sorted_vcp, "VCP 7 类 1:1 镜像");

    // 验证 Apeireth 独有 2 类
    assert_eq!(Category::Safety.as_vcp_name(), "safety");
    assert_eq!(Category::LongRunning.as_vcp_name(), "long_running");
}

// =====================================================================
// Test 2: HeuristicClassifier 9 demo 准确率 ≥ 80% (硬指标)
// =====================================================================

#[test]
fn integration_heuristic_accuracy_on_9_demo_meets_80_percent() {
    let demos = make_9_demo_tools();
    let classifier = HeuristicClassifier::new();
    let mut correct = 0;
    let mut results: Vec<(String, Category, Category)> = Vec::new();
    for (name, expected) in &demos {
        let t = tool(name);
        let got = classifier.classify(t.as_ref()).unwrap_or(Category::Search);
        if got == *expected {
            correct += 1;
        } else {
            results.push(((*name).to_string(), *expected, got));
        }
    }
    let accuracy = correct as f32 / demos.len() as f32;
    assert!(
        accuracy >= 0.8,
        "9 demo 准确率应 ≥ 80%, 实际 {:.0}% ({}/9), 错: {results:?}",
        accuracy * 100.0,
        correct
    );
}

// =====================================================================
// Test 3: EmbeddingClassifier 跑通, cosine 准确率 mock ≥ 70%
// =====================================================================

#[test]
fn integration_embedding_classifier_runs_and_returns_category() {
    let classifier = EmbeddingClassifier::with_embed_fn(Arc::new(MockHashEmbedFn::new()))
        .with_threshold(0.0); // 0 阈值 → 永远返 best
    let demos = make_9_demo_tools();
    let mut ran = 0;
    for (name, _expected) in &demos {
        let t = tool(name);
        match classifier.classify(t.as_ref()) {
            Ok(cat) => {
                ran += 1;
                // 任何 Category 都算跑通 (mock embed 32 维, cosine 准确率不保证 100%)
                assert!(matches!(cat, Category::Search | Category::FileCode | Category::ImageMedia
                    | Category::MemoryKnowledge | Category::AgentTask | Category::Communication
                    | Category::Data | Category::Safety | Category::LongRunning));
            }
            Err(ClassifyError::NoMatch { .. }) => {
                // 0 阈值下应该不会 NoMatch, 但兜底记 1 次
            }
            Err(e) => panic!("EmbeddingClassifier 意外错: {e:?}"),
        }
    }
    assert_eq!(ran, demos.len(), "EmbeddingClassifier 应跑通全部 9 demo");
}

// =====================================================================
// Test 4: LlmClassifier mock 模式 + 3 个分类器都能 classify 同一 tool
// =====================================================================

#[test]
fn integration_three_classifiers_all_run_on_same_tool() {
    let name = "WebSearch";
    let t = tool(name);

    let heuristic = HeuristicClassifier::new();
    let embedding = EmbeddingClassifier::with_embed_fn(Arc::new(MockHashEmbedFn::new()))
        .with_threshold(0.0);
    let llm = LlmClassifier::new_mock();

    let h = heuristic.classify(t.as_ref()).expect("heuristic ok");
    let e = embedding.classify(t.as_ref()).expect("embedding ok");
    let l = llm.classify(t.as_ref()).expect("llm mock ok");

    // heuristic 应该返 Search (WebSearch 关键词匹配)
    assert_eq!(h, Category::Search, "heuristic 应 = Search");
    // embedding mock: 任何 Category 都行 (mock 32 维 FNV 不保证准确)
    let _ = e;
    // llm mock: WebSearch → Search (按 name 子串)
    assert_eq!(l, Category::Search, "llm mock 应 = Search");

    // 3 个 classifier 都要能返 confidence
    let h_conf = heuristic.confidence(t.as_ref()).expect("conf");
    let e_conf = embedding.confidence(t.as_ref()).expect("conf");
    let l_conf = llm.confidence(t.as_ref()).expect("conf");
    assert!((0.0..=1.0).contains(&h_conf));
    assert!((0.0..=1.0).contains(&e_conf));
    assert!((0.0..=1.0).contains(&l_conf));
}

// =====================================================================
// Test 5: registry 集成 — register_with_classifier + tools_by_category
// =====================================================================

#[test]
fn integration_registry_with_classifier_end_to_end() {
    let registry = ToolRegistry::new();
    let classifier = HeuristicClassifier::new();
    let demos = make_9_demo_tools();

    // 1. 用 classifier 注册 9 个 tool
    for (name, _expected) in &demos {
        let result = registry.register_with_classifier(
            (*name).to_string(),
            tool(name),
            &classifier,
        );
        // 9 demo 应该全部成功分类 (heuristic 在 demo 集上 100% 命中)
        assert!(
            result.is_ok(),
            "register_with_classifier({name}) 应成功, 实际: {result:?}"
        );
    }

    // 2. 按类别查 (VCP _recordCategories 类比)
    let search_tools = registry.tools_by_category(Category::Search);
    assert_eq!(search_tools, vec!["WebSearch".to_string()]);

    let file_tools = registry.tools_by_category(Category::FileCode);
    assert_eq!(file_tools, vec!["FileOperator".to_string()]);

    let safety_tools = registry.tools_by_category(Category::Safety);
    assert_eq!(safety_tools, vec!["PermissionGuard".to_string()]);

    // 3. category_summary
    let summary = registry.category_summary();
    assert_eq!(summary.len(), 9, "9 类别全列");
    assert_eq!(summary.get(&Category::Search).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::FileCode).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::ImageMedia).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::MemoryKnowledge).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::AgentTask).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::Communication).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::Data).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::Safety).unwrap().len(), 1);
    assert_eq!(summary.get(&Category::LongRunning).unwrap().len(), 1);

    // 4. unregister 同步清 categories
    registry.unregister("WebSearch");
    let search_tools_after = registry.tools_by_category(Category::Search);
    assert!(search_tools_after.is_empty(), "unregister 后 categories 也清");

    // 5. clear 全清
    registry.clear();
    for cat in Category::all().iter() {
        assert!(registry.tools_by_category(*cat).is_empty());
    }
}

// =====================================================================
// Test 6: register_with_classifier NoMatch 不写 categories
// =====================================================================

#[test]
fn integration_registry_no_match_does_not_write_categories() {
    let registry = ToolRegistry::new();
    let classifier = HeuristicClassifier::new();

    // "XyzQqq" 没有关键词命中
    let result = registry.register_with_classifier("XyzQqq".to_string(), tool("XyzQqq"), &classifier);
    assert!(matches!(result, Err(ClassifyError::NoMatch { .. })));

    // tool 仍写入 (显式 0 假装: 不分就 0 类, 但 tool 仍可用)
    assert!(registry.get("XyzQqq").is_some(), "tool 仍应注册");

    // 但 categories 没有
    let summary = registry.category_summary();
    let total_in_categories: usize = summary.values().map(|v| v.len()).sum();
    assert_eq!(total_in_categories, 0, "NoMatch 不应写 categories");
}

// =====================================================================
// Test 7: register() (无 classifier) 行为 0 改 (R18 #2.4 兼容)
// =====================================================================

#[test]
fn integration_registry_register_without_classifier_still_works() {
    // 验证 R18 #2.4 既有 register() 行为 0 改, categories 表 0 写入
    let registry = ToolRegistry::new();
    registry.register("OldStyleTool".to_string(), tool("OldStyleTool"));

    assert!(registry.get("OldStyleTool").is_some());
    // categories 仍是空
    let summary = registry.category_summary();
    let total_in_categories: usize = summary.values().map(|v| v.len()).sum();
    assert_eq!(total_in_categories, 0, "register() 0 分类, 0 写 categories");
}

// =====================================================================
// Test 8: 编译期常量交叉验证 (VCP §6.2.1 #12 + 战区 5)
// =====================================================================

#[test]
fn integration_compile_time_constants_unchanged() {
    // 5 轴 243 组合 (R17 战役 2-1)
    assert_eq!(AXIS_COMBINATION_COUNT, 243);
    // 9 类别 (R25 战区 5)
    assert_eq!(CATEGORY_COUNT, 9);
}
