//! 关怀面板只读面的面契约: `GET /v1/panel/care` 的 `care_list` 帧同形形状 +
//! 心跳两列 + 显式空态 + W2-② 令牌门 + 未接线 501 诚实降级 + `care.read` 入能力清单。
//!
//! 这里钉六件事:
//! 1. **帧同形字段集合** —— `rules` 行 = `care_list` 帧 `rule_json` 同形六字段
//!    + 真值补列 `last_fired_at_ms` / `next_trigger_at_ms`; `discipline` / `last_care`
//!    逐字段同名同义 (帧同形对工具帧的逐值对照钉在组合根侧真数据链
//!    `crates/adapters/cli/tests/panel_care_traits_real_chain.rs`);
//! 2. **心跳两列 + 关怀节真值** —— `status` 节 (规则数/下次触发/最近关怀/心跳两列),
//!    心跳 `null` = 未接线/未运行 (不冒充在跑);
//! 3. **空表显式空** —— `rules: []` / `last_care: null` / `status.rules: 0`, 不缺字段、不 501;
//! 4. **令牌门同面** —— 缺 Bearer 401 `auth_missing_key`, 伪造 401 `auth_invalid_key`,
//!    真令牌 200 (与面板族同一把门);
//! 5. **未接线如实** —— 端口未注入 = 501 `unsupported` (不假装有数据源);
//! 6. **能力清单入册** —— `care.read` 只读面, `supported/available` 随接线状态。

use std::sync::Arc;

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, CareDisciplineDto,
    CarePanelDto, CarePanelQuery, CarePanelStatusDto, CareRecordDto, CareRuleDto, GatewayServices,
    GatewayState, GatewayTokenGate,
};
use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-care-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

/// 端口桩: 回一份固定关怀快照 (面形状与门语义的测试不需要真文件面)。
struct StubCare {
    frame: CarePanelDto,
    error: Option<String>,
}

#[async_trait]
impl CarePanelQuery for StubCare {
    async fn care_frame(&self) -> Result<CarePanelDto, String> {
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(self.frame.clone()),
        }
    }
}

fn sample_frame() -> CarePanelDto {
    CarePanelDto {
        status: CarePanelStatusDto {
            rules: 1,
            next_trigger_at_ms: Some(1_728_000_007_200),
            last_care_at_ms: Some(1_728_000_000_000),
            last_care_note: Some("morning: spoke (rule_fallback): 早安问候".to_string()),
            heartbeat_next_poll_at_ms: Some(1_728_000_000_500),
            heartbeat_last_hit_at_ms: Some(1_728_000_000_000),
        },
        rules: vec![CareRuleDto {
            id: "morning".to_string(),
            trigger: serde_json::json!({ "kind": "daily_moment", "minutes_of_day": 450 }),
            trigger_kind: "daily_moment".to_string(),
            material: "早安问候".to_string(),
            enabled: true,
            created_at_ms: 1_727_000_000_000,
            last_fired_at_ms: Some(1_728_000_000_000),
            next_trigger_at_ms: 1_728_007_800_000,
        }],
        discipline: CareDisciplineDto {
            quiet_start_minutes: Some(23 * 60),
            quiet_end_minutes: Some(8 * 60),
            frequency_max: 1,
            frequency_window_hours: 4,
            daily_max: 5,
            focus_gap_minutes: 3,
            catchup_minutes: 120,
            // 合同演进随行 (K 轴 L2/L5 批): 守护例外档三列入 fixture。
            guard_alert_breaks_quiet: true,
            guard_frequency_max: 3,
            guard_frequency_window_hours: 1,
        },
        last_care: Some(CareRecordDto {
            rule_id: "morning".to_string(),
            at_ms: 1_728_000_000_000,
            judge: "rule_fallback".to_string(),
            spoke: true,
            note: "早安问候".to_string(),
            // 合同演进随行 (K 轴 L2/L5 批): 台账来源档列。
            kind: "rule".to_string(),
        }),
    }
}

async fn harness(services: GatewayServices, gate: GatewayTokenGate) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.expect("runtime"));
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = gate;
    state
}

fn request(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    if let Some(token) = token {
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {token}").parse().expect("bearer"),
        );
    }
    request
}

async fn status_and_body(response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn stub_services() -> GatewayServices {
    GatewayServices {
        care: Some(Arc::new(StubCare {
            frame: sample_frame(),
            error: None,
        })),
        ..GatewayServices::default()
    }
}

/// 1. 帧同形字段集合 + 心跳两列: 逐字段钉死, 触发器内联 `kind` 标签同形。
#[tokio::test]
async fn care_endpoint_projects_the_care_list_frame_shape_with_heartbeat_columns() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/care", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let mut top: Vec<&str> = body
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    top.sort_unstable();
    assert_eq!(
        top,
        vec!["discipline", "last_care", "rules", "status"],
        "帧顶层四节 = care_list 帧同形三节 + 关怀节真值"
    );

    // 规则行: care_list 帧 `rule_json` 六字段 + 两列真值补列。
    let rule = &body["rules"][0];
    let mut keys: Vec<&str> = rule
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "created_at_ms",
            "enabled",
            "id",
            "last_fired_at_ms",
            "material",
            "next_trigger_at_ms",
            "trigger",
            "trigger_kind"
        ]
    );
    assert_eq!(rule["id"], "morning");
    assert_eq!(
        rule["trigger"]["kind"], "daily_moment",
        "触发器内联 kind 标签同形"
    );
    assert_eq!(rule["trigger"]["minutes_of_day"], 450);
    assert_eq!(rule["trigger_kind"], "daily_moment");
    assert_eq!(rule["material"], "早安问候");
    assert_eq!(rule["enabled"], true);
    assert_eq!(rule["created_at_ms"], 1_727_000_000_000i64);
    assert_eq!(rule["last_fired_at_ms"], 1_728_000_000_000i64);
    assert_eq!(rule["next_trigger_at_ms"], 1_728_007_800_000i64);

    // discipline: care_list 帧 discipline 节逐字段同名 (合同演进 K 轴 L2/L5 批:
    // 随 care_list 帧加守护例外档三列, 同形镜像照跟)。
    let discipline = &body["discipline"];
    let mut keys: Vec<&str> = discipline
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "catchup_minutes",
            "daily_max",
            "focus_gap_minutes",
            "frequency_max",
            "frequency_window_hours",
            "guard_alert_breaks_quiet",
            "guard_frequency_max",
            "guard_frequency_window_hours",
            "quiet_end_minutes",
            "quiet_start_minutes"
        ]
    );
    assert_eq!(discipline["quiet_start_minutes"], 23 * 60);

    // last_care: care_list 帧 `record_json` 逐字段同名 (null = 尚无关怀; 合同
    // 演进 K 轴 L2/L5 批: 随 care_list 帧加台账来源档列 `kind`)。
    let record = &body["last_care"];
    let mut keys: Vec<&str> = record
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["at_ms", "judge", "kind", "note", "rule_id", "spoke"]
    );
    assert_eq!(record["spoke"], true);

    // status: 关怀节真值 + 心跳两列 (null 语义在空态例钉)。
    let status_node = &body["status"];
    let mut keys: Vec<&str> = status_node
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "heartbeat_last_hit_at_ms",
            "heartbeat_next_poll_at_ms",
            "last_care_at_ms",
            "last_care_note",
            "next_trigger_at_ms",
            "rules"
        ],
        "关怀节真值六列 (含心跳两列), 不多不少"
    );
    assert_eq!(status_node["rules"], 1);
    assert_eq!(
        status_node["last_care_note"], "morning: spoke (rule_fallback): 早安问候",
        "最近关怀判定文案与自述关怀节同一口径"
    );
    assert_eq!(
        status_node["heartbeat_next_poll_at_ms"],
        1_728_000_000_500i64
    );
    assert_eq!(
        status_node["heartbeat_last_hit_at_ms"],
        1_728_000_000_000i64
    );
}

/// 2. 空表 = 显式空态 (字段一个不少, 不缺字段也不 501; 心跳 null 不冒充在跑)。
#[tokio::test]
async fn empty_care_table_is_an_explicit_empty_state() {
    let frame = CarePanelDto {
        status: CarePanelStatusDto {
            rules: 0,
            next_trigger_at_ms: None,
            last_care_at_ms: None,
            last_care_note: Some("尚无关怀记录".to_string()),
            heartbeat_next_poll_at_ms: None,
            heartbeat_last_hit_at_ms: None,
        },
        rules: vec![],
        discipline: CareDisciplineDto {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            frequency_max: 1,
            frequency_window_hours: 4,
            daily_max: 5,
            focus_gap_minutes: 3,
            catchup_minutes: 120,
            // 合同演进随行 (K 轴 L2/L5 批): 守护例外档三列入 fixture。
            guard_alert_breaks_quiet: true,
            guard_frequency_max: 3,
            guard_frequency_window_hours: 1,
        },
        last_care: None,
    };
    let services = GatewayServices {
        care: Some(Arc::new(StubCare { frame, error: None })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/care", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["rules"], serde_json::json!([]));
    assert_eq!(body["last_care"], serde_json::Value::Null);
    assert_eq!(body["status"]["rules"], 0);
    assert_eq!(
        body["status"]["next_trigger_at_ms"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["status"]["heartbeat_next_poll_at_ms"],
        serde_json::Value::Null,
        "心跳两列 null = 未接线/未运行, 不冒充在跑"
    );
    assert_eq!(
        body["status"]["heartbeat_last_hit_at_ms"],
        serde_json::Value::Null
    );
    assert_eq!(body["status"]["last_care_note"], "尚无关怀记录");
}

/// 3. 令牌门同面: 强制态下与面板族同一门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn care_endpoint_is_behind_the_gateway_token_gate() {
    let state = harness(
        stub_services(),
        GatewayTokenGate::Enforced(TOKEN.to_string()),
    )
    .await;
    let router = canonical_router_with_state(state);

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/care", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_missing_key"), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/care", Some(FORGED_TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_invalid_key"), "{text}");
    assert!(!text.contains(TOKEN), "401 帧不得回显令牌: {text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/care", Some(TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"rules\""), "{text}");
}

/// 4. 未接线 = 501 `unsupported` 诚实降级 (不假装有数据源)。
#[tokio::test]
async fn care_endpoint_degrades_to_501_without_the_query_port() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/care", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("\"unsupported\""), "{text}");
    assert!(text.contains("care.read"), "{text}");
}

/// 5. 读失败 = 500 错误帧报真因 (错误即帧, 不静默降级成空表)。
#[tokio::test]
async fn care_endpoint_reports_read_failures_as_error_frames() {
    let services = GatewayServices {
        care: Some(Arc::new(StubCare {
            frame: sample_frame(),
            error: Some("care_rules.json refused to open: 版本不符".to_string()),
        })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/care", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{text}");
    assert!(text.contains("runtime_error"), "{text}");
    assert!(text.contains("版本不符"), "真因如实进错误帧: {text}");
}

/// 6. 能力清单入册: `care.read` 只读面, `supported/available` 随接线状态。
#[tokio::test]
async fn care_read_is_registered_in_the_capability_manifest() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/apeireth/capabilities", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let groups = body["capabilities"].as_array().expect("groups");
    let group = groups
        .iter()
        .find(|group| group["name"] == "care")
        .expect("care 分组入册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["id"], "care.read");
    assert_eq!(cap["read"], true);
    assert_eq!(
        cap["write"], false,
        "care.read 恒只读 (写面归工具, 停人工审批)"
    );
    assert_eq!(cap["operations"], serde_json::json!(["list"]));
    assert_eq!(cap["supported"], true);
    assert_eq!(cap["available"], true);

    // 未接线: 清单如实报不支持 (不虚报可用)。
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (_, text) = status_and_body(
        router
            .oneshot(request("/v1/apeireth/capabilities", None))
            .await
            .unwrap(),
    )
    .await;
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    let groups = body["capabilities"].as_array().expect("groups");
    let group = groups
        .iter()
        .find(|group| group["name"] == "care")
        .expect("care 分组在册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["supported"], false);
    assert_eq!(cap["available"], false);
    assert!(cap["reason"].as_str().is_some(), "不可用须给机器可读真因");
}
