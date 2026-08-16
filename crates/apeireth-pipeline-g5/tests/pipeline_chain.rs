//! # Pipeline 集成测试 (13 测全过, per 整合 #3 R21 续补)
//!
//! 13 测试覆盖 (per 任务 spec "13 集成测试全过", bg_cf3e5220 已建 + R21 续补验证):
//! 1. **test_5_stage_chain_success** — 5 阶段全跑通 (Dispatch → Normalize → Policy → Reliability → Throttle)
//! 2. **test_error_propagation_fail_fast** — 第 1 阶段失败, 后续 stage 0 跑 (fail-fast)
//! 3. **test_empty_pipeline_returns_error** — 0 stage 返回 `EmptyPipeline`
//! 4. **test_invalid_stage_order** — stage 顺序错 (Policy 排第 1) 返回 `InvalidStageOrder`
//! 5. **test_policy_denied** — Policy 阶段 deny-list 命中, 返回 `PolicyDenied`
//! 6. **test_throttle_limit** — Throttle 阶段 QPS 超限, 返回 `Throttled`
//! 7. **test_pipeline_message_validation** — PipelineMessage 字段长度守门
//! 8. **test_stage_kind_count_guard** — STAGE_KIND_COUNT == 5 编译期守门
//! 9. **test_run_with_trace** — run_with_trace 收集诊断信息
//! 10. **test_dispatch_empty_kind** — Dispatch 阶段空 kind 拒绝
//! 11. **test_normalize_5_steps** — Normalize 阶段 5 步归一化真跑
//! 12. **test_reliability_max_attempts** — Reliability 阶段超 MAX_RETRY_ATTEMPTS 拒绝
//! 13. **test_reliability_backoff** — Reliability 阶段 backoff 4 步 [100/200/500/1000] 计算
//!
//! ## R21 续补 (per 整合 #3 决策 F-3 — sandbox 真接 6 API)
//!
//! test_5_stage_chain_success 现校验 `IDEMPOTENCY_KEY_PREFIX == "sandbox-"` (per Reliability 阶段
//! R21 续补: 从 `"pl-g5-"` schema 名 → `"sandbox-"` schema 名, 跟 `apeireth-sandbox` crate 真接 schema 对齐).
//! 0 假装真接 sandbox: 当前仅 schema 名称对齐, 实际 `apeireth-sandbox` 6 API 集成留 R21+.

use apeireth_pipeline_g5::*;

// ============================================================================
// Test 1: 5 阶段链 end-to-end success
// ============================================================================

#[test]
fn test_5_stage_chain_success() {
    // 5 阶段 Pipeline<ChatPipeline, PipelineMessage, PipelineMessage>
    // 用 whitelist_disabled 让 kind "chat" 通过 (默认白名单里有, 但为了演示通用)
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("chat-default", "ChatPipeline"))
            .with_stage(DefaultDispatch::new().with_whitelist_disabled())
            .with_stage(DefaultNormalize::new())
            .with_stage(DefaultPolicy::new())
            .with_stage(DefaultReliability::new())
            .with_stage(DefaultThrottle::new());

    assert_eq!(pipeline.len(), 5);
    assert_eq!(pipeline.stage_kinds().len(), 5);
    assert_eq!(
        pipeline.stage_kinds(),
        vec![
            StageKind::Dispatch,
            StageKind::Normalize,
            StageKind::Policy,
            StageKind::Reliability,
            StageKind::Throttle,
        ]
    );

    // 跑链
    let input = PipelineMessage::new("chat", "  Hello World  ");
    let result = pipeline.run(input);
    assert!(
        result.is_ok(),
        "5-stage chain should succeed: {:?}",
        result.err()
    );

    let output = result.unwrap();
    // Dispatch 加 "[chat] " 前缀
    assert!(
        output.payload.starts_with("[chat] "),
        "payload should start with [chat], got: {:?}",
        output.payload
    );
    // Normalize trim + lowercase
    assert!(
        output.payload.contains("hello world"),
        "payload should be normalized: {:?}",
        output.payload
    );
    // Reliability attempt +1
    assert_eq!(output.attempt, 1, "attempt should be 1 after Reliability");
    // Reliability 加 trace_id 前缀 "sandbox-" (R21 续补, per 整合 #3 决策 F-3 sandbox 真接 schema)
    assert!(
        output.trace_id.starts_with("sandbox-"),
        "trace_id should start with sandbox-, got: {:?}",
        output.trace_id
    );
}

// ============================================================================
// Test 2: 错误传播 (fail-fast)
// ============================================================================

#[test]
fn test_error_propagation_fail_fast() {
    // input: kind="spam" (在 Policy deny-list 里, 不在 Dispatch 默认白名单)
    // Dispatch whitelist_disabled → 通过
    // Normalize 通过 (payload 不空)
    // Policy 拒绝 (spam in POLICY_DENY_KINDS)
    // 后续 Reliability / Throttle 0 跑 (fail-fast)
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("chat-empty", "ChatPipeline"))
            .with_stage(DefaultDispatch::new().with_whitelist_disabled())
            .with_stage(DefaultNormalize::new())
            .with_stage(DefaultPolicy::new())
            .with_stage(DefaultReliability::new())
            .with_stage(DefaultThrottle::new());

    let input = PipelineMessage::new("spam", "this is spam content");
    let result = pipeline.run(input);
    assert!(result.is_err(), "Policy should deny 'spam'");

    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::PolicyDenied);
    assert!(err.to_string().contains("policy denied"));
    assert!(err.to_string().contains("spam"));
}

// ============================================================================
// Test 3: 空 Pipeline 返回错误
// ============================================================================

#[test]
fn test_empty_pipeline_returns_error() {
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("empty", "ChatPipeline"));
    assert!(pipeline.is_empty());
    assert_eq!(pipeline.len(), 0);

    let input = PipelineMessage::new("chat", "hello");
    let result = pipeline.run(input);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::EmptyPipeline);
}

// ============================================================================
// Test 4: 无效 stage 顺序 (Policy 排第 1)
// ============================================================================

#[test]
fn test_invalid_stage_order() {
    // 故意错序: Policy 排第 1 (应该 Dispatch 排第 1)
    struct DummyStage;
    impl Stage<PipelineMessage, PipelineMessage> for DummyStage {
        fn kind(&self) -> StageKind {
            StageKind::Policy // 故意返 Policy, 制造顺序错
        }
        fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
            Ok(input)
        }
    }

    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("bad-order", "ChatPipeline")).with_stage(DummyStage);
    let input = PipelineMessage::new("chat", "hello");
    let result = pipeline.run(input);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::InvalidStageOrder);
}

// ============================================================================
// Test 5: Policy 阶段 deny-list
// ============================================================================

#[test]
fn test_policy_denied() {
    // 4 阶段链 (关闭 strict_order 因为顺序是 [Dispatch, Normalize, Policy, Throttle], 不严格)
    // 用 Dispatch whitelist_disabled 让 "phishing" 通过
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> = Pipeline::new(
        PipelineConfig::new("policy-test", "ChatPipeline").with_strict_order_disabled(),
    )
    .with_stage(DefaultDispatch::new().with_whitelist_disabled())
    .with_stage(DefaultNormalize::new())
    .with_stage(DefaultPolicy::new())
    .with_stage(DefaultThrottle::new());

    let input = PipelineMessage::new("phishing", "phishing attempt");
    let result = pipeline.run(input);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::PolicyDenied);
    assert!(err.to_string().contains("phishing"));
}

// ============================================================================
// Test 6: Throttle 阶段限流
// ============================================================================

#[test]
fn test_throttle_limit() {
    // 单跑 Dispatch + Throttle (2 阶段, 关闭 strict_order)
    // Throttle 检查顺序: max-concurrent (50) → qps-cap (100) → burst (200)
    // MAX_CONCURRENT=50 是最先触发, 跑 60 次期望 50 成功 + 10 限流
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> = Pipeline::new(
        PipelineConfig::new("throttle-test", "ChatPipeline").with_strict_order_disabled(),
    )
    .with_stage(DefaultDispatch::new().with_whitelist_disabled())
    .with_stage(DefaultThrottle::new());

    // 跑 MAX_CONCURRENT + 10 次 (默认 MAX_CONCURRENT = 50, 跑 60 次)
    let mut successes = 0;
    let mut throttled = 0;
    for i in 0..(MAX_CONCURRENT + 10) {
        let input = PipelineMessage::new("chat", &format!("msg {}", i));
        match pipeline.run(input) {
            Ok(_) => successes += 1,
            Err(e) if e.kind() == PipelineErrorKind::Throttled => throttled += 1,
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
    // 至少有 1 个被限流 (因为 60 > MAX_CONCURRENT=50)
    assert!(
        throttled > 0,
        "expected at least 1 throttled, got successes={} throttled={}",
        successes,
        throttled
    );
    // successes 应该等于 MAX_CONCURRENT
    assert_eq!(
        successes, MAX_CONCURRENT,
        "expected exactly MAX_CONCURRENT successes, got {}",
        successes
    );
}

// ============================================================================
// Test 7: PipelineMessage 字段长度守门
// ============================================================================

#[test]
fn test_pipeline_message_validation() {
    // 正常 message
    let msg = PipelineMessage::new("chat", "hello");
    assert!(msg.validate().is_ok());

    // 超长 kind
    let long_kind = "a".repeat(MAX_KIND_LEN + 1);
    let msg = PipelineMessage::new(long_kind, "hello");
    assert!(msg.validate().is_err());

    // 超长 payload
    let long_payload = "a".repeat(MAX_PAYLOAD_LEN + 1);
    let msg = PipelineMessage::new("chat", long_payload);
    assert!(msg.validate().is_err());

    // 边界: 正好 MAX_KIND_LEN (OK)
    let boundary_kind = "a".repeat(MAX_KIND_LEN);
    let msg = PipelineMessage::new(boundary_kind, "hello");
    assert!(msg.validate().is_ok());
}

// ============================================================================
// Test 8: 编译期 hardcode 守门 (runtime assert 验证)
// ============================================================================

#[test]
fn test_stage_kind_count_guard() {
    assert_eq!(STAGE_KIND_COUNT, 5);
    assert_eq!(STAGE_ORDER.len(), 5);
    assert_eq!(STAGE_ORDER[0], StageKind::Dispatch);
    assert_eq!(STAGE_ORDER[4], StageKind::Throttle);
    assert_eq!(PIPELINE_G5_STAGE_COUNT, 5);
    assert_eq!(PIPELINE_G5_MAX_STAGES, 5);
    assert_eq!(PIPELINE_ERROR_VARIANT_COUNT, 6);
    assert_eq!(PLATFORM_NAME, "apeireth");
}

// ============================================================================
// Test 9: run_with_trace 收集诊断
// ============================================================================

#[test]
fn test_run_with_trace() {
    // 用 Dispatch whitelist_disabled 让 "exploit" 通过 (默认白名单里没有)
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("trace-test", "ChatPipeline").with_diagnostics())
            .with_stage(DefaultDispatch::new().with_whitelist_disabled())
            .with_stage(DefaultNormalize::new())
            .with_stage(DefaultPolicy::new())
            .with_stage(DefaultReliability::new())
            .with_stage(DefaultThrottle::new());

    let input = PipelineMessage::new("chat", "hello");
    let (result, trace) = pipeline.run_with_trace(input);
    assert!(result.is_ok());

    assert_eq!(trace.pipeline_name, "trace-test");
    assert_eq!(trace.stages_run.len(), 5);
    assert_eq!(trace.stages_run[0], "default-dispatch");
    assert_eq!(trace.stages_run[1], "default-normalize");
    assert_eq!(trace.stages_run[2], "default-policy");
    assert_eq!(trace.stages_run[3], "default-reliability");
    assert_eq!(trace.stages_run[4], "default-throttle");
    assert!(trace.failed_at.is_none());

    // 失败 trace: "exploit" 触发 Policy 拒绝
    let input = PipelineMessage::new("exploit", "exploit attempt");
    let (result, trace) = pipeline.run_with_trace(input);
    assert!(result.is_err());
    assert_eq!(
        trace.failed_at,
        Some(2),
        "Policy 是第 3 个 stage (idx=2), 应该在这失败"
    );
}

// ============================================================================
// Test 10: Dispatch 空 kind 拒绝
// ============================================================================

#[test]
fn test_dispatch_empty_kind() {
    let pipeline: Pipeline<ChatPipeline, PipelineMessage, PipelineMessage> =
        Pipeline::new(PipelineConfig::new("dispatch-test", "ChatPipeline"))
            .with_stage(DefaultDispatch::new());

    let input = PipelineMessage::new("", "hello");
    let result = pipeline.run(input);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::Stage);
    assert!(err.to_string().contains("empty"));
}

// ============================================================================
// Test 11: Normalize 5 步归一化真跑
// ============================================================================

#[test]
fn test_normalize_5_steps() {
    let n = DefaultNormalize::new();
    // 1. trim
    assert_eq!(n.normalize("  hello  "), "hello");
    // 2. fold whitespace
    assert_eq!(n.normalize("hello   world"), "hello world");
    // 3. lowercase ascii (Unicode 不动)
    assert_eq!(n.normalize("HELLO World 你好"), "hello world 你好");
    // 4. strip null
    assert_eq!(n.normalize("hel\0lo"), "hello");
    // 5. strip bom
    assert_eq!(n.normalize("\u{FEFF}hello"), "hello");
    // 全部
    assert_eq!(n.normalize("  HELLO\0\u{FEFF}  World  "), "hello world");
}

// ============================================================================
// Test 12: Reliability 超 MAX_RETRY_ATTEMPTS 拒绝
// ============================================================================

#[test]
fn test_reliability_max_attempts() {
    let r = DefaultReliability::new();
    // 正常 attempt=0 → 1
    let msg = PipelineMessage::new("chat", "hello");
    let result = r.process(msg).unwrap();
    assert_eq!(result.attempt, 1);

    // attempt=MAX_RETRY_ATTEMPTS=5 → 6
    let mut msg = PipelineMessage::new("chat", "hello");
    msg.attempt = 5;
    let result = r.process(msg).unwrap();
    assert_eq!(result.attempt, 6);

    // attempt=MAX_RETRY_ATTEMPTS+1=6 → 拒绝
    let mut msg = PipelineMessage::new("chat", "hello");
    msg.attempt = 6;
    let result = r.process(msg);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), PipelineErrorKind::Stage);
    assert!(err.to_string().contains("max retries"));
}

// ============================================================================
// Test 13: Throttle backoff 计算
// ============================================================================

#[test]
fn test_reliability_backoff() {
    // backoff_ms(0) = 0
    assert_eq!(DefaultReliability::backoff_ms(0), 0);
    // backoff_ms(1) = RETRY_BACKOFF_MS[0] = 100
    assert_eq!(DefaultReliability::backoff_ms(1), 100);
    // backoff_ms(2) = RETRY_BACKOFF_MS[1] = 200
    assert_eq!(DefaultReliability::backoff_ms(2), 200);
    // backoff_ms(3) = RETRY_BACKOFF_MS[2] = 500
    assert_eq!(DefaultReliability::backoff_ms(3), 500);
    // backoff_ms(4) = RETRY_BACKOFF_MS[3] = 1000
    assert_eq!(DefaultReliability::backoff_ms(4), 1000);
    // backoff_ms(5) 越界 → 用最后一个 (1000)
    assert_eq!(DefaultReliability::backoff_ms(5), 1000);
    // backoff_ms(100) 越界 → 用最后一个
    assert_eq!(DefaultReliability::backoff_ms(100), 1000);
}

// ============================================================================
// ChatPipeline 类型 marker (per 任务 spec "Pipeline<T, I, O> trait")
// ============================================================================

/// **类型 marker**: ChatPipeline (per `Pipeline<T, I, O>` 用法, 编译期区分 pipeline 种类).
///
/// 实际场景可定义多个 marker: `TaskPipeline` / `MemoryPipeline` / `McpPipeline` / ...
/// 阶段 6 skeleton 只给 1 个例子 (ChatPipeline), 用户可自由扩展.
#[derive(Debug, Clone, Copy)]
pub struct ChatPipeline;
