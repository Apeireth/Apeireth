//! # Full Pipeline Example — 1 个完整 5 阶段 pipeline 例子
//!
//! per 任务 spec: "公开 API 100% 文档化 + 例子 (一个完整 pipeline 例子)"
//!
//! ## 演示
//!
//! 1. 创建 1 个 `ChatPipeline` marker (用户可换 `TaskPipeline` / `MemoryPipeline` / ...)
//! 2. 构造 5 阶段 pipeline (Dispatch → Normalize → Policy → Reliability → Throttle)
//! 3. 跑 3 个示例输入:
//!    - 正常 chat message (成功, 全 5 阶段过)
//!    - spam message (Policy 拒绝)
//!    - 超大 payload (Policy 拒绝 by size)
//! 4. 用 `run_with_trace` 收集诊断
//!
//! ## 跑
//!
//! ```bash
//! cargo run -p apeireth-pipeline-g5 --example full_pipeline
//! ```

use apeireth_pipeline_g5::*;

// ChatPipeline marker (per `Pipeline<T, I, O>` 用法, 编译期区分 pipeline 种类)
#[derive(Debug, Clone, Copy)]
struct ChatPipeline;

fn main() {
    println!("===========================================");
    println!("apeireth-pipeline-g5: Full Pipeline Example");
    println!("===========================================\n");

    // ------------------------------------------------------------------
    // Step 1: 构造 5 阶段 Pipeline
    // (Dispatch whitelist_disabled 让任意 kind 通过, 给 example 演示通用)
    // ------------------------------------------------------------------
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> = Pipeline::new(
        PipelineConfig::new("chat-default", "ChatPipeline").with_diagnostics(), // 启用 diagnostics
    )
    .with_stage(DefaultDispatch::new().with_whitelist_disabled())
    .with_stage(DefaultNormalize::new())
    .with_stage(DefaultPolicy::new())
    .with_stage(DefaultReliability::new())
    .with_stage(DefaultThrottle::new());

    println!(
        "Pipeline '{}' (type={}, stages={})\n",
        pipeline.config().name,
        pipeline.config().type_marker,
        pipeline.len(),
    );

    // ------------------------------------------------------------------
    // Step 2: 跑正常 message (成功)
    // ------------------------------------------------------------------
    println!("--- Example 1: Normal chat message ---");
    let input = PipelineMessage::new("chat", "  Hello World  ").with_trace_id("trace-001");
    println!(
        "Input: kind={:?}, payload={:?}, attempt={}, trace_id={:?}",
        input.kind, input.payload, input.attempt, input.trace_id
    );

    let (result, trace) = pipeline.run_with_trace(input);
    match result {
        Ok(output) => {
            println!("✓ Success");
            println!(
                "  Output: kind={:?}, payload={:?}, attempt={}, trace_id={:?}",
                output.kind, output.payload, output.attempt, output.trace_id
            );
        }
        Err(e) => println!("✗ Failed: {}", e),
    }
    println!(
        "  Trace: stages_run={:?}, failed_at={:?}\n",
        trace.stages_run, trace.failed_at
    );

    // ------------------------------------------------------------------
    // Step 3: 跑 spam message (Policy 拒绝)
    // ------------------------------------------------------------------
    println!("--- Example 2: Spam message (Policy deny-list) ---");
    let input = PipelineMessage::new("spam", "buy cheap stuff now").with_trace_id("trace-002");
    println!("Input: kind={:?}, payload={:?}", input.kind, input.payload);

    let (result, trace) = pipeline.run_with_trace(input);
    match result {
        Ok(output) => println!("✓ Success: {:?}", output.payload),
        Err(e) => {
            println!(
                "✗ Failed: {} (kind={:?}, stage={:?})",
                e,
                e.kind(),
                e.stage_kind()
            );
        }
    }
    println!(
        "  Trace: stages_run={:?}, failed_at={:?}\n",
        trace.stages_run, trace.failed_at
    );

    // ------------------------------------------------------------------
    // Step 4: 跑超大 payload (Policy deny by size)
    // ------------------------------------------------------------------
    println!("--- Example 3: Oversized payload (Policy size limit) ---");
    let big_payload = "a".repeat(MAX_POLICY_PAYLOAD_SIZE + 1); // 16 KiB + 1
    let input = PipelineMessage::new("chat", big_payload).with_trace_id("trace-003");
    println!(
        "Input: kind={:?}, payload.len()={}",
        input.kind,
        input.payload.len()
    );

    let (result, trace) = pipeline.run_with_trace(input);
    match result {
        Ok(output) => println!("✓ Success: payload.len()={}", output.payload.len()),
        Err(e) => {
            println!("✗ Failed: {} (kind={:?})", e, e.kind());
        }
    }
    println!(
        "  Trace: stages_run={:?}, failed_at={:?}\n",
        trace.stages_run, trace.failed_at
    );

    // ------------------------------------------------------------------
    // Step 5: 演示 Normalize 5 步归一化
    // ------------------------------------------------------------------
    println!("--- Example 4: Normalize 5 步归一化 ---");
    let n = DefaultNormalize::new();
    let samples = vec![
        ("  Hello World  ", "trim + fold + lowercase"),
        ("HELLO 你好", "lowercase ASCII + Unicode 保留"),
        ("multi   spaces", "fold whitespace"),
        ("with\0null", "strip null"),
        ("\u{FEFF}with bom", "strip UTF-8 BOM"),
    ];
    for (input, desc) in samples {
        let output = n.normalize(input);
        println!("  {:?} -> {:?} ({})", input, output, desc);
    }
    println!();

    // ------------------------------------------------------------------
    // Step 6: 演示 Reliability backoff
    // ------------------------------------------------------------------
    println!("--- Example 5: Reliability backoff (4 步) ---");
    for attempt in 0..=6 {
        let backoff = DefaultReliability::backoff_ms(attempt);
        println!("  attempt={} -> backoff_ms={}", attempt, backoff);
    }
    println!();

    // ------------------------------------------------------------------
    // Step 7: 演示 5 阶段编译期守门
    // ------------------------------------------------------------------
    println!("--- Example 6: 编译期 hardcode 守门 (runtime assert 验证) ---");
    println!("  STAGE_KIND_COUNT = {} (期望 5)", STAGE_KIND_COUNT);
    println!("  STAGE_ORDER[0] = {:?} (期望 Dispatch)", STAGE_ORDER[0]);
    println!("  STAGE_ORDER[4] = {:?} (期望 Throttle)", STAGE_ORDER[4]);
    println!(
        "  PIPELINE_G5_STAGE_COUNT = {} (期望 5)",
        PIPELINE_G5_STAGE_COUNT
    );
    println!(
        "  PIPELINE_ERROR_VARIANT_COUNT = {} (期望 6)",
        PIPELINE_ERROR_VARIANT_COUNT
    );
    println!("  PLATFORM_NAME = {:?} (期望 \"apeireth\")", PLATFORM_NAME);
    println!(
        "  PIPELINE_G5_SCHEMA_VERSION = {:?} (期望 \"1\")",
        PIPELINE_G5_SCHEMA_VERSION
    );
    println!();

    println!("===========================================");
    println!("Example complete. 5 阶段 pipeline 通用框架演示完毕.");
    println!("===========================================");
}
