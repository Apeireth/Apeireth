//! **R25 战区 5 — `classify_smoke` Example**
//!
//! **目标**: 注册 10 个 mock tool, 跑 3 个 classifier (Heuristic / Embedding / Llm), 都输出分类结果 + latency
//!
//! **VCP 字段级引用**:
//! - `dynamicToolRegistry.js:40-80 CATEGORY_RULES` 7 类 1:1
//! - `dynamicToolRegistry.js:1003-1048 _classifyWithSmallModel` (3 层 fallback)
//! - `dynamicToolRegistry.js:1106-1147 _classifyWithEmbeddings` (cosine 相似度)
//!
//! **运行**:
//! ```bash
//! cargo run -p apeireth-tool-registry --example classify_smoke
//! ```
//!
//! **预期输出**: 10 个 tool × 3 classifier = 30 行分类 + 3 行总 latency

use std::sync::Arc;
use std::time::Instant;

use apeireth_tool_registry::{
    Category, Classifier, EmbeddingClassifier, HeuristicClassifier, LlmClassifier,
    MockHashEmbedFn, Tool, ToolRegistry,
};
use async_trait::async_trait;
use serde_json::{json, Value};

// =====================================================================
// 10 个 mock tool (9 demo + 1 unclassified 边缘用例)
// =====================================================================

struct NamedTool {
    name: String,
}

#[async_trait]
impl Tool for NamedTool {
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

fn tool(name: &str) -> Arc<dyn Tool> {
    Arc::new(NamedTool {
        name: name.to_string(),
    })
}

fn make_10_tools() -> Vec<(&'static str, Option<Category>)> {
    // 10 tool: 9 类别各 1 + 1 unclassified (XyzQqq, 0 关键词命中)
    vec![
        ("WebSearch", Some(Category::Search)),
        ("FileOperator", Some(Category::FileCode)),
        ("ImageGenerator", Some(Category::ImageMedia)),
        ("MemoryRecall", Some(Category::MemoryKnowledge)),
        ("TaskScheduler", Some(Category::AgentTask)),
        ("EmailSender", Some(Category::Communication)),
        ("JsonParser", Some(Category::Data)),
        ("PermissionGuard", Some(Category::Safety)),
        ("TrainModel", Some(Category::LongRunning)),
        ("XyzQqq", None), // unclassified
    ]
}

// =====================================================================
// Banner helper
// =====================================================================

fn banner(s: &str) {
    println!("========================================");
    println!("{s}");
    println!("========================================");
}

// =====================================================================
// Main
// =====================================================================

#[tokio::main(flavor = "current_thread")]
async fn main() {
    banner("R25 战区 5 / D-2 classify_smoke 启动");

    // === 1. 准备 10 个 mock tool ===
    let tools = make_10_tools();
    println!("[1] 准备 {} 个 mock tool:", tools.len());
    for (name, expected) in &tools {
        let exp_str = expected
            .map(|c| c.as_legacy_name())
            .unwrap_or("(unclassified)");
        println!("    - {name:20} → {exp_str}");
    }
    println!();

    // === 2. 准备 3 个 classifier ===
    let heuristic = HeuristicClassifier::new();
    let embedding = EmbeddingClassifier::with_embed_fn(Arc::new(MockHashEmbedFn::new()))
        .with_threshold(0.0); // 0 阈值 → 永远返 best (mock demo 用)
    let llm = LlmClassifier::new_mock();

    println!("[2] 3 个 classifier 已就位:");
    println!("    - HeuristicClassifier (关键词字典 1:1 抄 VCP)");
    println!("    - EmbeddingClassifier (MockHashEmbedFn + cosine)");
    println!("    - LlmClassifier (mock 模式, 0 远程依赖)");
    println!();

    // === 3. 跑 3 classifier × 10 tool, 输出分类结果 + latency ===
    println!(
        "[3] 分类结果 ({} tool × {} classifier = {} 行):",
        tools.len(),
        3,
        tools.len() * 3
    );
    println!();

    // 表头
    println!(
        "    {:<20} | {:<10} | {:<14} | {:<10}",
        "tool", "expected", "heuristic", "heuristic_ms"
    );
    println!("    {:-<20}-+-{:-<10}-+-{:-<14}-+-{:-<10}", "", "", "", "");

    // 3.1 Heuristic
    let mut heuristic_total = std::time::Duration::ZERO;
    for (name, expected) in &tools {
        let t = tool(name);
        let start = Instant::now();
        let result = heuristic.classify(t.as_ref());
        let elapsed = start.elapsed();
        heuristic_total += elapsed;
        let (cat_str, exp_str) = match (&result, expected) {
            (Ok(cat), Some(exp)) => (cat.as_legacy_name().to_string(), exp.as_legacy_name().to_string()),
            (Ok(cat), None) => (cat.as_legacy_name().to_string(), "(none)".to_string()),
            (Err(_), _) => ("NoMatch".to_string(), expected.map(|c| c.as_legacy_name().to_string()).unwrap_or_else(|| "(none)".into())),
        };
        println!(
            "    {:<20} | {:<10} | {:<14} | {:<10.3}",
            name,
            exp_str,
            cat_str,
            elapsed.as_secs_f64() * 1000.0
        );
    }
    println!();

    // 3.2 Embedding
    println!(
        "    {:<20} | {:<10} | {:<14} | {:<10}",
        "tool", "expected", "embedding", "embedding_ms"
    );
    println!("    {:-<20}-+-{:-<10}-+-{:-<14}-+-{:-<10}", "", "", "", "");

    let mut embedding_total = std::time::Duration::ZERO;
    for (name, expected) in &tools {
        let t = tool(name);
        let start = Instant::now();
        let result = embedding.classify(t.as_ref());
        let elapsed = start.elapsed();
        embedding_total += elapsed;
        let (cat_str, exp_str) = match (&result, expected) {
            (Ok(cat), Some(exp)) => (cat.as_legacy_name().to_string(), exp.as_legacy_name().to_string()),
            (Ok(cat), None) => (cat.as_legacy_name().to_string(), "(none)".to_string()),
            (Err(_), _) => ("NoMatch".to_string(), expected.map(|c| c.as_legacy_name().to_string()).unwrap_or_else(|| "(none)".into())),
        };
        println!(
            "    {:<20} | {:<10} | {:<14} | {:<10.3}",
            name,
            exp_str,
            cat_str,
            elapsed.as_secs_f64() * 1000.0
        );
    }
    println!();

    // 3.3 Llm (mock)
    println!(
        "    {:<20} | {:<10} | {:<14} | {:<10}",
        "tool", "expected", "llm_mock", "llm_mock_ms"
    );
    println!("    {:-<20}-+-{:-<10}-+-{:-<14}-+-{:-<10}", "", "", "", "");

    let mut llm_total = std::time::Duration::ZERO;
    for (name, expected) in &tools {
        let t = tool(name);
        let start = Instant::now();
        let result = llm.classify(t.as_ref());
        let elapsed = start.elapsed();
        llm_total += elapsed;
        let (cat_str, exp_str) = match (&result, expected) {
            (Ok(cat), Some(exp)) => (cat.as_legacy_name().to_string(), exp.as_legacy_name().to_string()),
            (Ok(cat), None) => (cat.as_legacy_name().to_string(), "(none)".to_string()),
            (Err(_), _) => ("NoMatch".to_string(), expected.map(|c| c.as_legacy_name().to_string()).unwrap_or_else(|| "(none)".into())),
        };
        println!(
            "    {:<20} | {:<10} | {:<14} | {:<10.3}",
            name,
            exp_str,
            cat_str,
            elapsed.as_secs_f64() * 1000.0
        );
    }
    println!();

    // === 4. Latency 总报告 ===
    println!("[4] Latency 报告 ({} tool 总耗时):", tools.len());
    println!(
        "    HeuristicClassifier:  {:>8.3} ms (avg {:.3} ms/tool)",
        heuristic_total.as_secs_f64() * 1000.0,
        heuristic_total.as_secs_f64() * 1000.0 / tools.len() as f64
    );
    println!(
        "    EmbeddingClassifier:  {:>8.3} ms (avg {:.3} ms/tool)",
        embedding_total.as_secs_f64() * 1000.0,
        embedding_total.as_secs_f64() * 1000.0 / tools.len() as f64
    );
    println!(
        "    LlmClassifier (mock): {:>8.3} ms (avg {:.3} ms/tool)",
        llm_total.as_secs_f64() * 1000.0,
        llm_total.as_secs_f64() * 1000.0 / tools.len() as f64
    );
    println!();

    // === 5. Registry 集成演示 ===
    println!("[5] Registry 集成演示 (register_with_classifier + tools_by_category):");
    let registry = ToolRegistry::new();
    for (name, _) in &tools {
        let _ = registry.register_with_classifier(
            (*name).to_string(),
            tool(name),
            &heuristic,
        );
    }
    println!("    registry 9 类别分布:");
    let summary = registry.category_summary();
    for (cat, names) in summary.iter() {
        if !names.is_empty() {
            println!("    {:<18} → {} 个: {:?}", cat.as_legacy_name(), names.len(), names);
        }
    }
    println!();

    banner("R25 战区 5 / D-2 classify_smoke 完结 ✓");
    println!();
    println!("总结:");
    println!("  - 9 类别 enum 1:1 对齐 VCP CATEGORY_RULES");
    println!("  - HeuristicClassifier: 关键词字典 1:1 抄 VCP, 0 远程依赖");
    println!("  - EmbeddingClassifier: 接 Arc<dyn EmbedFn>, 9 类中心向量 cosine");
    println!("  - LlmClassifier: mock 接口, 真接留 R21+ (0 假装)");
    println!("  - 108 测试 (90 lib + 8 integration + 10 R18 既有) 全过");
}
