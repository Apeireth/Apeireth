//! 通用嗓 TTS 闭环 (`tool.voice_speak`) 集成回归: 合成调用 (假引擎面注入) /
//! 参数边界拒 / 播放队列语义 (打断·串行) / 真停清理 (句柄零泄漏) / 语句账本
//! append+读回 / 治理档与旋钮 fail-closed / 勿扰窗口抑制 / 脱敏口径。
//!
//! 真机「发音」验收位在本文件尾部 `#[ignore]` 组 (需 Windows 系统语音引擎 +
//! 扬声器, `cargo test -- --ignored` 执行)。

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Permission, PermissionPolicy, ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::voice::mock::{MockBehavior, MockSynthEngine};
use apeireth_tools_canonical::voice::{
    fixed_clock, system_synth_engine, voice_speak_capability, AbsentSynthEngine, PlaybackPolicy,
    QuietHours, SpeakGate, SpokenLedger, SpokenStatus, SynthEngine, VoiceSpeakBoundaryGuard,
    VoiceSpeakConfig, VoiceSpeakParams, VoiceSpeakRiskMappingHook, VoiceSpeakTool,
    VOICE_SPEAK_CAPABILITY_ID, VOICE_SPEAK_MAX_TEXT_CHARS, VOICE_SPEAK_TOOL_DESCRIPTION,
    VOICE_SPEAK_TOOL_NAME,
};
use apeireth_tools_canonical::{PreExecuteHook, PreExecuteRequest, PreVerdict};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_string(),
        name: VOICE_SPEAK_TOOL_NAME.to_string(),
        arguments,
    }
}

fn args(text: &str) -> Value {
    // 本文件的机械面用例是「关怀帧发声」: 申报 `scope: "care"` 过缺省发声
    // 范围档 (范围自身的钉见 `scope_field_gates_the_frame_kind_on_the_tool_face`)。
    json!({ "text": text, "scope": "care" })
}

fn expect_value(result: &ToolResult) -> Value {
    match &result.outcome {
        ToolOutcome::Ok { value } => value.clone(),
        ToolOutcome::Error { message, .. } => panic!("期望成功却得错误帧: {message}"),
    }
}

fn expect_error(result: &ToolResult) -> String {
    match &result.outcome {
        ToolOutcome::Error { message, .. } => message.clone(),
        ToolOutcome::Ok { value } => panic!("期望错误帧却得成功: {value}"),
    }
}

fn temp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("apeireth-voice-speak-{tag}-"))
        .tempdir()
        .expect("tempdir")
}

fn config(data_dir: &std::path::Path) -> VoiceSpeakConfig {
    VoiceSpeakConfig::new(data_dir, fixed_clock(12 * 60))
        .with_quiet(QuietHours::parse("22:00-06:00").expect("window"))
        .with_playback_policy(PlaybackPolicy::InterruptPrevious)
}

fn capability(
    config: VoiceSpeakConfig,
    engine: Arc<dyn SynthEngine>,
    policy: Arc<std::sync::Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    voice_speak_capability(config, engine, policy)
}

/// 轮询等待假引擎的计数面到某个状态 (有界, 不空转死等)。
async fn wait_until(
    state: &apeireth_tools_canonical::voice::mock::MockState,
    label: &str,
    hit: impl Fn(&apeireth_tools_canonical::voice::mock::MockState) -> bool,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !hit(state) {
        assert!(
            std::time::Instant::now() < deadline,
            "等待超时: {label} (state: created={}, dropped={}, history={})",
            state.created(),
            state.dropped(),
            state.history_len()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

// ---------------------------------------------------------------------------
// 合成调用 (假引擎面注入) + 参数校验
// ---------------------------------------------------------------------------

/// 合成调用走注入引擎面: 文本/语速/音量/嗓名逐字到引擎, 报告如实回时长与嗓。
#[tokio::test]
async fn speak_invokes_the_injected_engine_with_resolved_params() {
    let dir = temp_dir("invoke");
    let engine = Arc::new(
        MockSynthEngine::new(MockBehavior::AutoComplete).with_voices(vec!["测试嗓".to_string()]),
    );
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());
    let mut params = json!({ "text": "你好, 世界", "scope": "care", "rate": 2, "volume": 80, "voice": "测试嗓" });

    let frame = expect_value(&tool.invoke(&call(params.clone())).await);
    assert_eq!(frame["status"], "spoken");
    assert_eq!(frame["voice"], "测试嗓");
    assert_eq!(frame["rate"], 2);
    assert_eq!(frame["volume"], 80);
    assert_eq!(frame["text_chars"], 6);
    assert_eq!(frame["engine"], "mock");
    assert_eq!(frame["ledger"], "spoken.jsonl");

    let requests = state.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].text, "你好, 世界");
    assert_eq!(requests[0].rate, 2);
    assert_eq!(requests[0].volume, 80);
    assert_eq!(requests[0].voice.as_deref(), Some("测试嗓"));

    // 账本: 说了什么/何时/哪把嗓/时长, 全文如实。
    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records.len(), 1);
    let spoken = &scan.records[0].body;
    assert_eq!(spoken.text, "你好, 世界");
    assert_eq!(spoken.voice, "测试嗓");
    assert_eq!(spoken.status, SpokenStatus::Completed);
    assert!(spoken.spoken_at_ms > 0);
    assert_eq!(spoken.engine, "mock");

    // 缺省参数: 默认嗓 + 缺省语速/音量。
    params = args("第二句");
    let frame = expect_value(&tool.invoke(&call(params)).await);
    assert_eq!(frame["voice"], "system-default");
    assert_eq!(frame["rate"], 0);
    assert_eq!(frame["volume"], 100);
    assert_eq!(state.live(), 0, "句柄零泄漏");
    assert_eq!(state.max_running(), 1, "同时只播一条");
}

/// 参数边界**拒绝即帧** (不钳制): 语速/音量越界、空文本、超长文本全拒,
/// 且引擎零调用 (拒绝发生在发声之前)。
#[tokio::test]
async fn out_of_range_params_are_refused_before_the_engine() {
    let dir = temp_dir("bounds");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let cases = [
        json!({ "text": "一句", "rate": 11 }),
        json!({ "text": "一句", "rate": -11 }),
        json!({ "text": "一句", "volume": 101 }),
        json!({ "text": "   " }),
        json!({ "text": "字".repeat(VOICE_SPEAK_MAX_TEXT_CHARS + 1) }),
        json!({ "text": "一句", "voice": "  " }),
    ];
    for case in cases {
        let message = expect_error(&tool.invoke(&call(case)).await);
        assert!(
            message.contains("拒绝发声") || message.contains("invalid voice_speak parameters"),
            "坏参数必须拒绝即帧: {message}"
        );
    }
    assert_eq!(state.created(), 0, "拒绝发生在发声之前: 引擎零调用");
    assert_eq!(state.requests().len(), 0);
}

/// 无嗓 / 未知嗓 = 错误即帧 (未知嗓附已装嗓清单可自纠); 未知字段拒绝。
#[tokio::test]
async fn unknown_or_missing_voice_is_an_honest_error_frame() {
    let dir = temp_dir("voice");
    let silent = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete).with_voices(vec![]));
    let tool = capability(config(dir.path()), silent, authorized_policy());
    let message = expect_error(&tool.invoke(&call(args("没有嗓"))).await);
    assert!(message.contains("无嗓"), "{message}");

    let dir2 = temp_dir("voice2");
    let engine = Arc::new(
        MockSynthEngine::new(MockBehavior::AutoComplete).with_voices(vec!["嗓一".to_string()]),
    );
    let tool = capability(config(dir2.path()), engine, authorized_policy());
    let message = expect_error(
        &tool
            .invoke(&call(
                json!({ "text": "一句", "scope": "care", "voice": "没有这把" }),
            ))
            .await,
    );
    assert!(message.contains("未知嗓"), "{message}");
    assert!(message.contains("嗓一"), "拒绝附已装嗓清单: {message}");

    let message = expect_error(&tool.invoke(&call(json!({ "text": "一", "nope": 1 }))).await);
    assert!(
        message.contains("invalid voice_speak parameters"),
        "未知字段拒绝: {message}"
    );
}

// ---------------------------------------------------------------------------
// 播放队列语义 (打断 / 串行) + 真停清理
// ---------------------------------------------------------------------------

/// 新话打断旧话 (显式打断档 `APEIRETH_VOICE_INTERRUPT=1`): 旧话终态
/// `interrupted` 如实入账, 新话照播。
#[tokio::test]
async fn interrupt_policy_stops_the_previous_utterance() {
    let dir = temp_dir("interrupt");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let old = tokio::spawn({
        let tool = Arc::clone(&tool);
        async move { tool.invoke(&call(args("旧话"))).await }
    });
    wait_until(&state, "旧话开播", |s| s.history_len() == 1).await;

    let new = tokio::spawn({
        let tool = Arc::clone(&tool);
        async move { tool.invoke(&call(args("新话"))).await }
    });
    wait_until(&state, "新话开播", |s| s.history_len() == 2).await;
    state.playback(1).expect("新话句柄").complete();

    let old_frame = expect_value(&old.await.expect("join"));
    let new_frame = expect_value(&new.await.expect("join"));
    assert_eq!(old_frame["status"], "interrupted", "旧话被打断 = 说了半句");
    assert_eq!(new_frame["status"], "spoken");
    assert_eq!(state.max_running(), 1, "同时只播一条");
    assert_eq!(state.live(), 0, "句柄零泄漏");

    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    let statuses: Vec<_> = scan.records.iter().map(|doc| doc.body.status).collect();
    assert_eq!(statuses.len(), 2, "两句都入账");
    assert!(
        statuses.contains(&SpokenStatus::Interrupted)
            && statuses.contains(&SpokenStatus::Completed),
        "打断 = 说了半句如实入账: {statuses:?}"
    );
}

/// 排队串行 (APEIRETH_VOICE_INTERRUPT=0 档): 旧话播完新话才开播, 不重叠。
#[tokio::test]
async fn queue_policy_plays_serially_without_overlap() {
    let dir = temp_dir("queue");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
    let state = engine.state();
    let config = config(dir.path()).with_playback_policy(PlaybackPolicy::QueueSerial);
    let tool = capability(config, engine, authorized_policy());

    let first = tokio::spawn({
        let tool = Arc::clone(&tool);
        async move { tool.invoke(&call(args("第一句"))).await }
    });
    wait_until(&state, "第一句开播", |s| s.history_len() == 1).await;
    let second = tokio::spawn({
        let tool = Arc::clone(&tool);
        async move { tool.invoke(&call(args("第二句"))).await }
    });
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(state.history_len(), 1, "排队档: 第一句在播时第二句不许开播");
    state.playback(0).expect("第一句句柄").complete();
    wait_until(&state, "第二句开播", |s| s.history_len() == 2).await;
    state.playback(1).expect("第二句句柄").complete();

    let first_frame = expect_value(&first.await.expect("join"));
    let second_frame = expect_value(&second.await.expect("join"));
    assert_eq!(first_frame["status"], "spoken");
    assert_eq!(second_frame["status"], "spoken");
    assert_eq!(state.max_running(), 1, "串行: 同时只播一条");
    assert_eq!(state.live(), 0);
}

/// 真停清理: 调用被取消 (超时/回合中断) = 播放跟着停, 句柄零泄漏无僵尸播放。
#[tokio::test]
async fn cancelled_call_stops_playback_and_leaves_no_handles() {
    let dir = temp_dir("cancel");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::Hold));
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let cancelled = tokio::time::timeout(
        Duration::from_millis(150),
        tool.invoke(&call(args("说到一半被取消"))),
    )
    .await;
    assert!(cancelled.is_err(), "Hold 档不收尾, 调用被取消");
    wait_until(&state, "真停收尾", |s| s.live() == 0).await;
    assert!(state.stop_calls() >= 1, "取消 = 真停 (不留僵尸播放)");
    assert_eq!(state.max_running(), 1);
}

// ---------------------------------------------------------------------------
// 语句账本 + 脱敏口径
// ---------------------------------------------------------------------------

/// 账本 append + 读回 (append-only 保序); 坏行读作不存在并如实计数。
#[tokio::test]
async fn ledger_appends_and_reads_back() {
    let dir = temp_dir("ledger");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let tool = capability(config(dir.path()), engine, authorized_policy());
    tool.invoke(&call(args("第一句"))).await;
    tool.invoke(&call(args("第二句"))).await;

    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    let texts: Vec<&str> = scan
        .records
        .iter()
        .map(|doc| doc.body.text.as_str())
        .collect();
    assert_eq!(texts, vec!["第一句", "第二句"], "append-only 读回保序");
    assert_eq!(scan.skipped, 0);

    // 坏行读作不存在 (如实计入 skipped, 不砖化整本)。
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(ledger.path())
            .expect("open ledger");
        writeln!(file, "{{broken line").expect("append bad line");
    }
    tool.invoke(&call(args("第三句"))).await;
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records.len(), 3, "好行照读 (坏行不砖化整本)");
    assert_eq!(scan.skipped, 1, "坏行如实计数");
    assert_eq!(scan.records[2].body.text, "第三句");
}

/// 账本写失败 = 错误即帧 (已发声的真因照实报, 不静默丢责任记录)。
#[tokio::test]
async fn ledger_write_failure_is_an_honest_error_frame() {
    let dir = temp_dir("ledger-fail");
    let blocked = dir.path().join("blocked");
    std::fs::create_dir(&blocked).expect("dir stands in the way");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let config = config(dir.path()).with_ledger_path(&blocked);
    let tool = capability(config, engine, authorized_policy());

    let message = expect_error(&tool.invoke(&call(args("这句说得出但账写不下"))).await);
    assert!(message.contains("已发声"), "{message}");
    assert!(message.contains("语句账本写失败"), "{message}");
}

/// 脱敏口径 (如实定档): 合成文本全文只进语句账本; 输出帧/错误帧只带字符数,
/// 不带原文。
#[tokio::test]
async fn text_lands_only_in_the_ledger_never_in_frames() {
    let dir = temp_dir("redact");
    const SECRET: &str = "TOP-SECRET-SENTENCE-机密句";
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let frame = expect_value(&tool.invoke(&call(args(SECRET))).await);
    let frame_text = serde_json::to_string(&frame).expect("serialize");
    assert!(!frame_text.contains(SECRET), "输出帧不带原文: {frame_text}");
    assert_eq!(frame["text_chars"], SECRET.chars().count(), "只报字符数");

    let error = expect_error(
        &tool
            .invoke(&call(json!({ "text": SECRET, "rate": 99 })))
            .await,
    );
    assert!(!error.contains(SECRET), "错误帧不带原文: {error}");
    assert!(!error.contains("SECRET"), "错误帧不带原文: {error}");

    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records[0].body.text, SECRET, "全文唯一落点 = 语句账本");
}

// ---------------------------------------------------------------------------
// 勿扰窗口 + 治理档 (fail-closed) + 输出合同
// ---------------------------------------------------------------------------

/// 勿扰窗口抑制: 窗口内错误即帧 (引擎零调用), 窗口外照播。
#[tokio::test]
async fn quiet_window_suppresses_speaking_as_an_error_frame() {
    let dir = temp_dir("quiet");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let state = engine.state();
    let night = VoiceSpeakConfig::new(dir.path(), fixed_clock(23 * 60))
        .with_quiet(QuietHours::parse("22:00-06:00").expect("window"));
    let tool = capability(night, engine, authorized_policy());

    let message = expect_error(&tool.invoke(&call(args("夜里不该出声"))).await);
    assert!(message.contains("勿扰窗口"), "{message}");
    assert_eq!(state.created(), 0, "勿扰抑制: 引擎零调用 (不造播放)");
    assert_eq!(state.requests().len(), 0);

    let dir2 = temp_dir("quiet-day");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let state = engine.state();
    let day = VoiceSpeakConfig::new(dir2.path(), fixed_clock(12 * 60))
        .with_quiet(QuietHours::parse("22:00-06:00").expect("window"));
    let tool = capability(day, engine, authorized_policy());
    let frame = expect_value(&tool.invoke(&call(args("白天照播"))).await);
    assert_eq!(frame["status"], "spoken");
    assert_eq!(state.created(), 1);
}

fn authorized_policy() -> Arc<std::sync::Mutex<PermissionPolicy>> {
    apeireth_tools_canonical::voice::authorized_voice_speak_policy()
}

/// 治理档 (发声面档, 与 shell 的执行档分立) + fail-closed:
/// grant = 免逐次人批; 未 grant = 拒绝即帧; 显式审批档仍走人批链;
/// 坏参数 / 勿扰窗口在 pre 拒绝即帧 (单调, 不得翻回)。
#[test]
fn governance_tier_is_the_voice_tier_and_fails_closed() {
    let probe_call = call(args("一句"));
    let capability_id = CapabilityId::new(VOICE_SPEAK_CAPABILITY_ID).expect("capability id");

    // 发声面默认档: grant 即发声 (免逐次人批) —— 与 shell 的
    // 「grant + require_approval 每次人批」执行档分立 (不借壳)。
    let granted = authorized_policy();
    let hook = VoiceSpeakRiskMappingHook::new(
        Arc::clone(&granted),
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    );
    let request = PreExecuteRequest::new(&probe_call, &capability_id);
    assert_eq!(hook.pre_verdict(&request), PreVerdict::Allow);

    // fail-closed: 未授权 = 拒绝即帧 (不降级成审批)。
    let denied = VoiceSpeakRiskMappingHook::new(
        Arc::new(std::sync::Mutex::new(PermissionPolicy::new())),
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    );
    let verdict = denied.pre_verdict(&request);
    assert!(
        matches!(verdict, PreVerdict::Deny { .. }),
        "未授权必须拒绝即帧: {verdict:?}"
    );

    // 宿主显式挂审批档 = 同一条人工审批链 (Ask, 不是静默放行)。
    let mut review_policy = PermissionPolicy::new();
    review_policy.grant(Permission::ExecuteTool(
        VOICE_SPEAK_CAPABILITY_ID.to_string(),
    ));
    review_policy.require_approval_for(VOICE_SPEAK_CAPABILITY_ID);
    let review = VoiceSpeakRiskMappingHook::new(
        Arc::new(std::sync::Mutex::new(review_policy)),
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    );
    assert!(
        matches!(review.pre_verdict(&request), PreVerdict::Ask { .. }),
        "显式审批档走人批链"
    );

    // 坏参数 / 勿扰窗口在 pre 阶段就拒绝即帧 (单调边界)。
    let bad_call = call(json!({ "text": "一句", "rate": 99 }));
    let bad = VoiceSpeakRiskMappingHook::new(
        Arc::clone(&granted),
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    );
    assert!(matches!(
        bad.pre_verdict(&PreExecuteRequest::new(&bad_call, &capability_id)),
        PreVerdict::Deny { .. }
    ));
    let night = VoiceSpeakRiskMappingHook::new(
        granted,
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(23 * 60),
    );
    assert!(matches!(
        night.pre_verdict(&request),
        PreVerdict::Deny { .. }
    ));
}

/// 单调边界 Guard: 只拒不放 (坏参数/勿扰拒; 好请求与他人能力放行)。
#[test]
fn boundary_guard_only_denies() {
    let guard = VoiceSpeakBoundaryGuard::new(
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(23 * 60),
    );
    let capability_id = CapabilityId::new(VOICE_SPEAK_CAPABILITY_ID).expect("capability id");
    let night_call = call(args("夜里一句"));
    let request = ToolGuardRequest::new(&capability_id, &night_call.name, &night_call.arguments);
    let refusal = guard.deny(&request).expect("勿扰窗口必须拒");
    assert!(refusal.contains("勿扰窗口"), "{refusal}");

    let day = VoiceSpeakBoundaryGuard::new(
        QuietHours::parse("22:00-06:00").expect("window"),
        fixed_clock(12 * 60),
    );
    let day_call = call(args("白天一句"));
    let request = ToolGuardRequest::new(&capability_id, &day_call.name, &day_call.arguments);
    assert!(day.deny(&request).is_none(), "好请求不拒");

    let other = CapabilityId::new("tool.shell").expect("capability id");
    let foreign_call = call(json!({ "text": "一句", "rate": 99 }));
    let request = ToolGuardRequest::new(&other, &foreign_call.name, &foreign_call.arguments);
    assert!(day.deny(&request).is_none(), "他人能力不归本 Guard");
}

/// 输出归一合同 (冻结形状): 六字段在场; 少字段的帧违约即帧。
#[tokio::test]
async fn output_contract_freezes_the_report_shape() {
    let dir = temp_dir("contract");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let tool = capability(config(dir.path()), engine, authorized_policy());
    let frame = expect_value(&tool.invoke(&call(args("一句"))).await);
    let schema = VoiceSpeakTool::output_schema();
    schema.validate(&frame).expect("输出帧必须过合同");

    let truncated = json!({ "status": "spoken", "voice": "x" });
    assert!(
        schema.validate(&truncated).is_err(),
        "少字段必须违约 (形状冻结)"
    );

    // 工具描述与声明面同文 (命名即承诺)。
    let tool_only = VoiceSpeakTool::new(
        config(dir.path()),
        Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete)),
    );
    let declaration = tool_only.declaration();
    assert_eq!(declaration.name, VOICE_SPEAK_TOOL_NAME);
    assert_eq!(
        declaration.description.as_deref(),
        Some(VOICE_SPEAK_TOOL_DESCRIPTION)
    );
}

/// 发声范围走工具面 (设计裁定 R1⑰): `scope` 字段判定 —— care 帧出声,
/// chat 帧 (含未申报) 拒绝即帧; 显式 all 档全量出声 (旧行为兼容)。
#[tokio::test]
async fn scope_field_gates_the_frame_kind_on_the_tool_face() {
    let dir = temp_dir("scope");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let frame = expect_value(
        &tool
            .invoke(&call(json!({ "text": "关怀提醒一句", "scope": "care" })))
            .await,
    );
    assert_eq!(frame["status"], "spoken");

    let message = expect_error(
        &tool
            .invoke(&call(
                json!({ "text": "常规划算对话回复", "scope": "chat" }),
            ))
            .await,
    );
    assert!(message.contains("发声范围"), "{message}");
    let message = expect_error(&tool.invoke(&call(json!({ "text": "未申报一句" }))).await);
    assert!(
        message.contains("发声范围"),
        "未申报按常规划算对话回复计: {message}"
    );
    assert_eq!(
        state.created(),
        1,
        "范围外帧不开播 (引擎只造了 care 帧的播放)"
    );

    // 旧行为兼容: 显式 all 档 = 全量出声 (常规划算对话回复照常出声)。
    let dir2 = temp_dir("scope-all");
    let engine2 = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let tool2 = capability(
        config(dir2.path()).with_speak_gate(SpeakGate::All),
        engine2,
        authorized_policy(),
    );
    let frame = expect_value(
        &tool2
            .invoke(&call(
                json!({ "text": "常规划算对话回复", "scope": "chat" }),
            ))
            .await,
    );
    assert_eq!(frame["status"], "spoken", "显式 all 档: chat 帧照播");
}

/// 合成失败 / 引擎缺席 = 错误即帧 (不假报已说)。
#[tokio::test]
async fn synthesis_failure_and_absent_engine_are_error_frames() {
    let dir = temp_dir("fail");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::Fail {
        reason: "引擎报错(真因)".to_string(),
    }));
    let state = engine.state();
    let tool = capability(config(dir.path()), engine, authorized_policy());
    let message = expect_error(&tool.invoke(&call(args("合成会失败"))).await);
    assert!(message.contains("合成失败"), "{message}");
    assert!(
        message.contains("引擎报错(真因)"),
        "真因如实转述: {message}"
    );
    assert_eq!(state.live(), 0, "失败也要真停清理");

    let dir2 = temp_dir("absent");
    let absent: Arc<dyn SynthEngine> =
        Arc::new(AbsentSynthEngine::new("无系统语音合成引擎 (如实报)"));
    let tool = capability(config(dir2.path()), absent, authorized_policy());
    let message = expect_error(&tool.invoke(&call(args("没有引擎"))).await);
    assert!(message.contains("无系统语音合成引擎"), "{message}");
    let ledger = SpokenLedger::under_data_dir(dir2.path());
    assert!(
        ledger.scan().expect("scan").records.is_empty(),
        "没说话不入账 (不造发声)"
    );
}

/// self_status 语音节: 账本数 / 最近说话 / 嗓名 (账本真值现探测, 尚无语句 =
/// 如实注不编时间; 摘录口径不回全文)。
#[tokio::test]
async fn self_status_voice_section_reports_ledger_facts_honestly() {
    let dir = temp_dir("self-status");
    let engine = Arc::new(MockSynthEngine::new(MockBehavior::AutoComplete));
    let tool = VoiceSpeakTool::new(config(dir.path()), engine);
    let params: VoiceSpeakParams =
        serde_json::from_value(json!({ "text": "最近说的一句", "scope": "care" })).expect("params");

    // 尚无语句: 空账本如实报 0 + 「尚无语句记录」。
    let empty = tool.ledger().status().expect("empty status");
    assert_eq!(empty.utterances, Some(0));
    assert_eq!(empty.note.as_deref(), Some("尚无语句记录"));
    assert_eq!(empty.last_spoken_at_ms, None, "不编时间");

    tool.speak_once(&params).await.expect("speak");
    let status = tool.ledger().status().expect("status");
    assert_eq!(status.utterances, Some(1), "账本数真值");
    assert_eq!(status.voice.as_deref(), Some("system-default"), "嗓名");
    assert_eq!(
        status.last_text_excerpt.as_deref(),
        Some("最近说的一句"),
        "摘录口径 (全文只在账本)"
    );

    // 自述渲染: 语音节全维在场 (缺位显式 null + reason 的合同不变)。
    let mut snapshot =
        apeireth_tools_canonical::SelfStatusSnapshot::minimal(Default::default(), Vec::new());
    snapshot.voice = Some(status);
    snapshot.voice_reason = None;
    let value = apeireth_tools_canonical::render_snapshot(&snapshot);
    assert_eq!(value["voice"]["utterances"], 1);
    assert_eq!(value["voice"]["voice"], "system-default");
    assert_eq!(value["voice"]["reason"], Value::Null);

    // 探测口缺位 = 全维显式 null + 原因 (不整帧失败)。
    snapshot.voice = None;
    snapshot.voice_reason = Some("status source not wired in this composition".to_string());
    let value = apeireth_tools_canonical::render_snapshot(&snapshot);
    assert_eq!(value["voice"]["utterances"], Value::Null);
    assert_eq!(
        value["voice"]["reason"],
        "status source not wired in this composition"
    );
}

// ---------------------------------------------------------------------------
// 真机「发音」验收位 (写全不删; 需 Windows 系统语音引擎 + 扬声器)
// ---------------------------------------------------------------------------

/// 真机发音验收: 真系统引擎探测 → 选嗓 → 合成即播 → 账本落一行。
/// 0 装纪律: 写全不删, 真机验收用 `cargo test -- --ignored`。
#[cfg(windows)]
#[tokio::test]
#[ignore = "真机发音验收位: 需 Windows 系统语音引擎与扬声器 (cargo test -p apeireth-tools-canonical --test voice_speak -- --ignored)"]
async fn real_machine_sapi_pronunciation() {
    let engine = system_synth_engine();
    let status = engine.status();
    assert!(status.available, "真机应探测到合成引擎: {}", status.reason);
    let voices = engine.installed_voices().expect("已装嗓清单");
    assert!(!voices.is_empty(), "真机应有已装嗓");

    let dir = temp_dir("real");
    let tool = capability(config(dir.path()), engine, authorized_policy());
    let frame = expect_value(&tool.invoke(&call(args("你好, 这是通用嗓验收。"))).await);
    assert_eq!(frame["status"], "spoken", "真机应发声完毕");
    assert!(frame["duration_ms"].as_u64().unwrap_or(0) > 0, "真发音时长");

    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records.len(), 1, "真机发声也必须入账");
    assert_eq!(scan.records[0].body.text, "你好, 这是通用嗓验收。");
}

/// 真机打断验收: 新话真打断旧话 (旧话半句入账, 无僵尸播放)。
#[cfg(windows)]
#[tokio::test]
#[ignore = "真机发音验收位 (打断拍): 需 Windows 系统语音引擎与扬声器"]
async fn real_machine_interrupt_stops_the_previous_utterance() {
    let engine = system_synth_engine();
    let dir = temp_dir("real-interrupt");
    let tool = capability(config(dir.path()), engine, authorized_policy());

    let old = tokio::spawn({
        let tool = Arc::clone(&tool);
        async move {
            tool.invoke(&call(args(
                "这句话很长很长很长很长很长很长很长很长很长很长",
            )))
            .await
        }
    });
    tokio::time::sleep(Duration::from_millis(700)).await;
    let new_frame = expect_value(&tool.invoke(&call(args("打断!"))).await);
    assert_eq!(new_frame["status"], "spoken");
    let old_frame = expect_value(&old.await.expect("join"));
    assert_eq!(old_frame["status"], "interrupted", "真机新话打断旧话");

    let ledger = SpokenLedger::under_data_dir(dir.path());
    let scan = ledger.scan().expect("scan");
    assert_eq!(scan.records.len(), 2, "两句都如实入账");
}
