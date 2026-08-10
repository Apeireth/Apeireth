//! # apeireth-image-prompt 端到端 demo
//!
//! 演示 5 步 (R20 阶段 4 skeleton 阶段):
//! 1. 加载 5 个 example prompt (从 fixture 估 JSON)
//! 2. PromptLibrary::add 5 个 (5 星优先返回)
//! 3. 模板变量渲染 `{{subject}} in {{style}}, {{mood}} lighting`
//! 4. dedup 验证 (同 sha256 二次 add 返回 Duplicate)
//! 5. export 5 个 prompt 为 JSON 字符串
//!
//! 运行: `cargo run --example image_prompt_demo`

use apeireth_image_prompt::{
    compute_sha256, ExportFormat, ImagePromptError, ImagePromptLibraryTrait, ImagePromptResult,
    PromptCategory, PromptEntry, PromptLibrary, PromptSearchQuery, PromptTemplate,
    TemplateRenderer,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ImagePromptResult<()> {
    println!("=== apeireth-image-prompt demo (R20 阶段 4 skeleton) ===\n");

    // --- 步骤 0: 准备 5 个 example prompt (in-memory, 不读 fixture 文件) ---
    let example_prompts = build_5_example_prompts();
    println!("[0] 准备 5 个 example prompt OK:");
    for p in &example_prompts {
        println!(
            "    - {} | category={:?} | rating={} | sha256={}...",
            p.name,
            p.category,
            p.rating,
            &p.sha256[..16]
        );
    }

    // --- 步骤 1: 创建 PromptLibrary (自定义 temp dir, 不写真 IO) ---
    let tmp = tempfile::TempDir::new().map_err(ImagePromptError::Io)?;
    let lib = PromptLibrary::with_storage_dir(PathBuf::from(tmp.path()));
    println!("\n[1] PromptLibrary OK: storage_dir = {}", lib.storage_dir().display());

    // --- 步骤 2: add 5 个 (骨架 — 仅入 in-memory index, 不写文件) ---
    let mut added_ids = Vec::new();
    for p in &example_prompts {
        match lib.add(p.clone()).await {
            Ok(id) => {
                added_ids.push(id);
            }
            Err(ImagePromptError::Duplicate(_)) => {
                println!("    [skip duplicate] {}", p.name);
            }
            Err(e) => return Err(e),
        }
    }
    println!("[2] add 5 prompt OK: added_ids.len = {}", added_ids.len());
    assert_eq!(added_ids.len(), 5);

    // --- 步骤 3: dedup 验证 — 同 sha256 二次 add 返回 Duplicate ---
    let dup_prompt = example_prompts[0].clone();
    let dup_result = lib.add(dup_prompt).await;
    assert!(matches!(dup_result, Err(ImagePromptError::Duplicate(_))));
    println!("[3] Dedup 验证 OK: 同 sha256 二次 add 返回 Duplicate");

    // --- 步骤 4: 模板变量渲染 `{{subject}} in {{style}}, {{mood}} lighting` ---
    let tpl = TemplateRenderer::new("{{subject}} in {{style}}, {{mood}} lighting")
        .with_defaults(&HashMap::from([("mood".to_string(), "warm".to_string())]));
    let mut vars = HashMap::new();
    vars.insert("subject".to_string(), "a cat".to_string());
    vars.insert("style".to_string(), "ink wash painting".to_string());
    let rendered = tpl.render(&vars)?;
    assert_eq!(rendered, "a cat in ink wash painting, warm lighting");
    println!("[4] 模板渲染 OK: rendered = \"{}\"", rendered);

    // --- 步骤 5: search (骨架 — 估返回空 vec, 不接 FTS5) ---
    let query = PromptSearchQuery {
        subject: Some("cat".to_string()),
        style: None,
        quality: None,
        composition: None,
        lighting: None,
        fulltext: None,
        limit: Some(10),
    };
    let search_result = lib.search(&query).await?;
    assert!(search_result.is_empty(), "search skeleton 应返回空 vec (FTS5 TODO)");
    println!("[5] search OK: 返回 0 项 (skeleton, FTS5 TODO)");

    // --- 步骤 6: export JSON (骨架 — list 返回空, export 也空) ---
    let json = lib.export(ExportFormat::Json).await?;
    assert!(json.contains("[]") || json.is_empty(), "export skeleton 应输出空 list");
    println!("[6] export JSON OK: len = {} chars", json.len());

    // --- 步骤 7: SHA256 稳定性验证 ---
    let h1 = compute_sha256("a cat in ink wash painting, warm lighting");
    let h2 = compute_sha256("a cat in ink wash painting, warm lighting");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
    println!("[7] SHA256 稳定 OK: {}...", &h1[..16]);

    // --- 步骤 8: 评分 5 星优先 (骨架 — update storage TODO) ---
    let high_id = &added_ids[0];
    lib.rate(high_id, 5).await?;
    println!("[8] rate({}) = 5 OK (skeleton — storage TODO)", high_id);

    println!("\n=== 全部 8 步 OK ===");
    Ok(())
}

/// 5 个 example prompt (mirror tests/fixtures/example_prompts.json 6 项中 5 项)
fn build_5_example_prompts() -> Vec<PromptEntry> {
    let mut p1 = PromptEntry::new(
        "ink_wash_cat",
        "a cat in ink wash painting, warm lighting",
        PromptCategory::Illustration,
    );
    p1.subject = "cat".to_string();
    p1.style = "ink wash".to_string();
    p1.quality = "high".to_string();
    p1.composition = "portrait".to_string();
    p1.lighting = "warm".to_string();
    p1.tags = vec!["cat".to_string(), "ink".to_string(), "painting".to_string()];
    p1.rating = 5;
    p1.template = Some(
        PromptTemplate::new("{{subject}} in {{style}}, {{mood}} lighting")
            .with_default("mood", "warm"),
    );

    let mut p2 = PromptEntry::new(
        "anime_girl_sunset",
        "anime girl at sunset, golden hour",
        PromptCategory::Anime,
    );
    p2.subject = "girl".to_string();
    p2.style = "anime".to_string();
    p2.quality = "high".to_string();
    p2.lighting = "golden hour".to_string();
    p2.tags = vec!["anime".to_string(), "girl".to_string(), "sunset".to_string()];
    p2.rating = 4;

    let mut p3 = PromptEntry::new(
        "photorealistic_mountain",
        "photorealistic mountain landscape, dramatic sky",
        PromptCategory::Photorealistic,
    );
    p3.subject = "mountain".to_string();
    p3.style = "photorealistic".to_string();
    p3.quality = "8k".to_string();
    p3.composition = "wide shot".to_string();
    p3.lighting = "dramatic".to_string();
    p3.tags = vec!["mountain".to_string(), "landscape".to_string(), "8k".to_string()];
    p3.rating = 5;

    let mut p4 = PromptEntry::new(
        "abstract_color_flow",
        "abstract color flow, vibrant gradient",
        PromptCategory::Abstract,
    );
    p4.subject = "color flow".to_string();
    p4.style = "abstract".to_string();
    p4.quality = "high".to_string();
    p4.tags = vec!["abstract".to_string(), "color".to_string(), "gradient".to_string()];
    p4.rating = 3;

    let mut p5 = PromptEntry::new(
        "sketch_pencil_portrait",
        "pencil sketch portrait, minimalist",
        PromptCategory::Sketch,
    );
    p5.subject = "portrait".to_string();
    p5.style = "pencil sketch".to_string();
    p5.quality = "medium".to_string();
    p5.tags = vec!["sketch".to_string(), "pencil".to_string(), "portrait".to_string()];
    p5.rating = 4;

    vec![p1, p2, p3, p4, p5]
}
