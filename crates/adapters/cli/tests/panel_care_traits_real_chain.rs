//! 关怀/性格面板只读面的真数据链 (组合根侧): `CliCarePanel` / `CliTraitPanel`
//! 读真文件面 → `GET /v1/panel/care` / `GET /v1/panel/traits` 的**同形 / 同账**
//! 对照 (面板不另造口径)。
//!
//! 这里钉四件事:
//! 1. **care 帧同形断言** —— 面板 `rules` 行 / `discipline` / `last_care` 与
//!    `tool.care_schedule` `care_list` 帧**逐字段逐值同形** (面板只加两列真值
//!    补列: 每规则发起锚 / 下次触发), 同一 `care_rules.json` 同一拒开口径;
//! 2. **care 关怀节真值同源** —— `status` 节 (规则数 / 下次触发 / 最近关怀含
//!    判定文案) 与 `tool.self_status` 关怀节同一表体投影; 心跳两列 = 生产者真值
//!    透传 (未起 = 显式 null, 不冒充在跑);
//! 3. **traits 同账断言** —— 面板四杆现值 / 锁定态 / 最近移动 / 坏行计数与
//!    `tool.trait_report` 帧**同一本账同一判据** (同一 `tuning-log.jsonl` +
//!    同一 `trait-locks.json`), 账面行逐字段保真;
//! 4. **坏档即帧** —— 锁定档坏档 = 失败帧报真因 (绝不读成全解锁放行)。

use std::sync::Arc;

use apeireth_cli::gateway_panels::{CliCarePanel, CliTraitPanel};
use apeireth_cli::CareHeartbeatSource;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices,
    GatewayTokenGate,
};
use apeireth_orchestration::self_tuning::{
    TunableParam, TuningRecord, DAY_MS, TRAIT_LOCKED_SKIP_PREFIX,
};
use apeireth_organ::care::{
    CareRecord, CareRule, CareScheduleTool, CareStore, CareTrigger, CARE_SCHEDULE_TOOL_NAME,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime::canonical::Runtime;
use apeireth_runtime_assembly::{
    now_epoch_ms, set_trait_lock, trait_growth_probe, TraitGrowthPaths, TraitReportTool,
    TRAIT_REPORT_TOOL_NAME, TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-panel-care-traits-{tag}-{}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

async fn get_json(router: axum::routing::Router<()>, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let value = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, value)
}

async fn panel_router(
    care: Option<Arc<dyn apeireth_gateway::CarePanelQuery>>,
    traits: Option<Arc<dyn apeireth_gateway::TraitPanelQuery>>,
) -> axum::routing::Router<()> {
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let services = GatewayServices {
        care,
        traits,
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    canonical_router_with_state(state)
}

/// 工具面一帧 (care_list / trait_report 的真实工具投影)。
async fn tool_frame(tool: &dyn ToolCapability, call: ToolCall) -> Value {
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("tool frame json")
}

fn care_call(action: &str) -> ToolCall {
    ToolCall {
        id: "call-care".to_string(),
        name: CARE_SCHEDULE_TOOL_NAME.to_string(),
        arguments: serde_json::json!({ "action": action }),
    }
}

fn trait_call() -> ToolCall {
    ToolCall {
        id: "call-trait".to_string(),
        name: TRAIT_REPORT_TOOL_NAME.to_string(),
        arguments: serde_json::json!({}),
    }
}

/// 1. care 帧同形断言: 面板三节与 `care_list` 工具帧逐字段逐值同形。
#[tokio::test]
async fn care_frame_is_isomorphic_to_the_care_list_tool_frame() {
    let dir = temp_dir("care-shape");
    let store = CareStore::new(&dir);
    let now = now_epoch_ms();
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 12 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: now - DAY_MS,
        })
        .expect("add rule");
    store
        .add_rule(CareRule {
            id: "morning".to_string(),
            trigger: CareTrigger::DailyMoment {
                minutes_of_day: 450,
            },
            material: "早安问候".to_string(),
            enabled: true,
            created_at_ms: now - DAY_MS,
        })
        .expect("add rule");
    store
        .record_care(
            CareRecord {
                rule_id: "morning".to_string(),
                at_ms: now - 3_600_000,
                judge: "rule_fallback".to_string(),
                spoke: true,
                note: "早安问候".to_string(),
            },
            now - 3_600_000,
        )
        .expect("record care");

    // 工具帧 = `care_list` 同一表体的工具投影 (对照基准)。
    let tool = CareScheduleTool::new(&dir);
    let tool_body = tool_frame(&tool, care_call("care_list")).await;

    // 面板帧 = 同一表体的面板投影 (心跳两列: 生产者未起 = 显式 null)。
    let care = Arc::new(CliCarePanel::new(
        dir.clone(),
        CareHeartbeatSource::new().status_probe(),
    ));
    let router = panel_router(Some(care), None).await;
    let (status, panel) = get_json(router, "/v1/panel/care").await;
    assert_eq!(status, StatusCode::OK, "{panel}");

    // ① rules 行: 工具帧每个字段逐值同形 (面板只加两列真值补列)。
    let tool_rules = tool_body["rules"].as_array().expect("tool rules");
    let panel_rules = panel["rules"].as_array().expect("panel rules");
    assert_eq!(panel_rules.len(), tool_rules.len(), "规则行数同形");
    for (tool_rule, panel_rule) in tool_rules.iter().zip(panel_rules.iter()) {
        for (key, value) in tool_rule.as_object().expect("tool rule object") {
            assert_eq!(
                panel_rule[key], *value,
                "规则行字段 {key} 必须与 care_list 帧同形同值"
            );
        }
        let mut extra: Vec<&str> = panel_rule
            .as_object()
            .expect("panel rule object")
            .keys()
            .map(String::as_str)
            .filter(|key| !tool_rule.as_object().unwrap().contains_key(*key))
            .collect();
        extra.sort_unstable();
        assert_eq!(
            extra,
            vec!["last_fired_at_ms", "next_trigger_at_ms"],
            "同形之上只有两列真值补列"
        );
    }
    // 发起锚真值列: record_care 落的 fired_at 如实透出 (无发起记录 = null)。
    let by_id = |frame: &Value, id: &str| {
        frame["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|rule| rule["id"] == id)
            .expect("rule by id")
            .clone()
    };
    assert_eq!(
        by_id(&panel, "morning")["last_fired_at_ms"],
        now - 3_600_000
    );
    assert_eq!(by_id(&panel, "check-in")["last_fired_at_ms"], Value::Null);

    // ② discipline / last_care: 与 care_list 帧逐值同形。
    assert_eq!(panel["discipline"], tool_body["discipline"], "打扰纪律同形");
    assert_eq!(panel["last_care"], tool_body["last_care"], "关怀台账同形");

    // ③ 关怀节真值: 规则数 / 最近关怀 / 判定文案与自述关怀节同口径。
    assert_eq!(
        panel["status"]["rules"],
        tool_body["rules"].as_array().unwrap().len() as u64
    );
    assert_eq!(panel["status"]["last_care_at_ms"], now - 3_600_000);
    assert_eq!(
        panel["status"]["last_care_note"], "morning: spoke (rule_fallback): 早安问候",
        "最近关怀判定文案与自述关怀节同一投影"
    );
    assert!(panel["status"]["next_trigger_at_ms"].is_number());

    // ④ 心跳两列: 生产者未起 = 显式 null (不冒充在跑)。
    assert_eq!(
        panel["status"]["heartbeat_next_poll_at_ms"],
        Value::Null,
        "未起 = 显式 null"
    );
    assert_eq!(panel["status"]["heartbeat_last_hit_at_ms"], Value::Null);
    let _ = fs_err::remove_dir_all(&dir);
}

/// 2. 心跳两列 = 生产者真值透传 (同一 CareHeartbeatSource 绑定)。
#[tokio::test]
async fn care_heartbeat_columns_carry_producer_truth() {
    use apeireth_runtime_assembly::CareHeartbeatStatus;

    let dir = temp_dir("care-heartbeat");
    let heartbeat: Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync> =
        Arc::new(|| CareHeartbeatStatus {
            next_poll_at_ms: Some(1_728_000_000_500),
            last_hit_at_ms: Some(1_728_000_000_000),
            ..CareHeartbeatStatus::default()
        });
    let care = Arc::new(CliCarePanel::new(dir.clone(), heartbeat));
    let router = panel_router(Some(care), None).await;
    let (status, panel) = get_json(router, "/v1/panel/care").await;
    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(
        panel["status"]["heartbeat_next_poll_at_ms"],
        1_728_000_000_500i64
    );
    assert_eq!(
        panel["status"]["heartbeat_last_hit_at_ms"],
        1_728_000_000_000i64
    );
    let _ = fs_err::remove_dir_all(&dir);
}

/// 一行成长账 (逐记录信封, 与写端同格式)。
fn envelope(row: &TuningRecord) -> String {
    serde_json::to_string(&apeireth_core::stored_doc::StoredDoc::new(
        TUNING_LOG_DOC_NAME,
        TUNING_LOG_DOC_VERSION,
        vec![TUNING_LOG_DOC_VERSION],
        row,
    ))
    .expect("serialize")
}

/// 3. traits 同账断言: 面板四杆现值 / 锁定态 / 最近移动与 `tool.trait_report`
/// 同一本账同一判据; 账面行逐字段保真。
#[tokio::test]
async fn traits_frame_is_on_the_same_account_as_tool_trait_report() {
    let dir = temp_dir("traits-same-account");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    let now = now_epoch_ms();

    // 成长计既有账 fixture: 实际移动 / 零移动留痕 / 锁定跳过留痕 / 另一杆移动。
    let rows = vec![
        TuningRecord {
            seq: 1,
            param: TunableParam::ToneSaturation,
            previous: 1.0,
            next: 0.8,
            reason: "过饱和".to_string(),
            at_epoch_ms: now - 2 * DAY_MS,
        },
        TuningRecord {
            seq: 2,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.0,
            reason: "已达参数上限: x".to_string(),
            at_epoch_ms: now - DAY_MS,
        },
        TuningRecord {
            seq: 3,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.0,
            reason: format!("{TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 被锁"),
            at_epoch_ms: now - DAY_MS,
        },
        TuningRecord {
            seq: 4,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.25,
            reason: "上调一档".to_string(),
            at_epoch_ms: now - 3_600_000,
        },
    ];
    let docs: Vec<String> = rows.iter().map(envelope).collect();
    fs_err::write(paths.log_path.clone(), docs.join("\n")).expect("写账");
    // 锁定档 (显式动作面的持久档): 只锁一根。
    set_trait_lock(&paths.locks_path(), TunableParam::MemoryFade, true).expect("锁定落盘");

    // 工具帧 = `tool.trait_report` 同账投影 (对照基准)。
    let tool = TraitReportTool::new(trait_growth_probe(paths.clone()));
    let tool_body = tool_frame(&tool, trait_call()).await;

    // 面板帧 = 同一本账的面板投影。
    let traits = Arc::new(CliTraitPanel::new(paths.clone()));
    let router = panel_router(None, Some(traits)).await;
    let (status, panel) = get_json(router, "/v1/panel/traits").await;
    assert_eq!(status, StatusCode::OK, "{panel}");

    // ① 四杆现值 + 锁定态: 与工具帧逐杆同值 (同一 TraitGrowthFacts 读数)。
    for slider in tool_body["sliders"].as_array().expect("sliders") {
        let param = slider["param"].as_str().expect("param");
        assert_eq!(
            panel["values"][param], slider["value"],
            "现值必须与 tool.trait_report 同账同值: {param}"
        );
        assert_eq!(
            panel["locked"][param], slider["locked"],
            "锁定态必须与 tool.trait_report 同账同值: {param}"
        );
    }
    assert_eq!(panel["locked"]["memory_fade"], true, "锁定档真值如实");
    assert_eq!(panel["locked"]["tone_saturation"], false);

    // ② 最近移动: 同一判据 (零移动留痕与锁定跳过不计) —— 与工具帧的 last_move
    //    同一账本同一行。
    let moves = panel["recent_moves"].as_array().expect("recent moves");
    assert_eq!(moves.len(), 2, "零移动留痕与锁定跳过不计移动");
    assert_eq!(moves[0]["param"], "memory_fade", "新 → 旧");
    assert_eq!(moves[0]["reason"], "上调一档");
    assert_eq!(moves[0]["delta"], 0.25);
    assert_eq!(moves[1]["param"], "tone_saturation");
    let fade_last_move = &tool_body["sliders"][0]["last_move"];
    assert_eq!(moves[0]["param"], fade_last_move["param"]);
    assert_eq!(moves[0]["label"], fade_last_move["label"]);
    assert_eq!(moves[0]["at_epoch_ms"], fade_last_move["at_epoch_ms"]);
    assert_eq!(moves[0]["delta"], fade_last_move["delta"]);
    assert_eq!(moves[0]["reason"], fade_last_move["reason"], "同账同一行");

    // ③ 账面行保真 + 坏行计数同源。
    let records = panel["records"].as_array().expect("records");
    assert_eq!(records.len(), rows.len(), "账面行全量保真 (含留痕行)");
    for (raw, record) in rows.iter().zip(records.iter()) {
        assert_eq!(record["seq"], raw.seq);
        assert_eq!(record["param"], raw.param.id());
        assert_eq!(record["previous"], raw.previous);
        assert_eq!(record["next"], raw.next);
        assert_eq!(record["reason"], raw.reason);
        assert_eq!(record["at_epoch_ms"], raw.at_epoch_ms);
    }
    assert_eq!(
        panel["skipped_records"], tool_body["skipped_records"],
        "坏行跳过计数与工具帧同源"
    );
    assert_eq!(panel["skipped_records"], 0);
    let _ = fs_err::remove_dir_all(&dir);
}

/// 4. 坏档即帧: 锁定档坏档 = 失败帧报真因 (绝不读成全解锁放行)。
#[tokio::test]
async fn broken_trait_lock_state_is_refused_as_an_error_frame() {
    let dir = temp_dir("traits-broken-locks");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    fs_err::write(paths.locks_path(), "{ not json").expect("坏档");

    let traits = Arc::new(CliTraitPanel::new(paths));
    let router = panel_router(None, Some(traits)).await;
    let (status, panel) = get_json(router, "/v1/panel/traits").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{panel}");
    assert_eq!(panel["error"]["code"], "runtime_error");
    let message = panel["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("trait lock state refused"),
        "真因如实进错误帧: {message}"
    );
    let _ = fs_err::remove_dir_all(&dir);
}
