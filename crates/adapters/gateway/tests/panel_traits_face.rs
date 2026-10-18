//! 性格面板只读面的面契约: `GET /v1/panel/traits` 的四滑杆现值 + 锁定态 +
//! 最近移动形状 + 显式空态 + W2-② 令牌门 + 未接线 501 诚实降级 + `traits.read`
//! 入能力清单。
//!
//! 这里钉六件事 (与 `tool.trait_report` 的**同账同源**逐值对照钉在组合根侧
//! 真数据链 `crates/adapters/cli/tests/panel_care_traits_real_chain.rs`):
//! 1. **帧字段集合** —— `{values, locked, recent_moves, records, skipped_records}`,
//!    移动行 = `trait_report::TraitMove` 同形, 账面行 = `TuningRecord` 同形;
//! 2. **现值/锁定态四杆齐** —— 四根一根不少 (缺档 = 全解锁默认态, 不编锁定);
//! 3. **空账显式空** —— `records: []` / `recent_moves: []` / `skipped_records: 0`,
//!    空账按空报 0, 不缺字段、不 501;
//! 4. **令牌门同面** —— 缺 Bearer 401 `auth_missing_key`, 伪造 401 `auth_invalid_key`,
//!    真令牌 200;
//! 5. **未接线如实** —— 端口未注入 = 501 `unsupported`; 读失败 = 500 错误帧报真因
//!    (锁定档坏档绝不读成全解锁);
//! 6. **能力清单入册** —— `traits.read` 只读面, `supported/available` 随接线状态。

use std::sync::Arc;

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices, GatewayState,
    GatewayTokenGate, TraitLocksDto, TraitMoveDto, TraitPanelDto, TraitPanelQuery, TraitValuesDto,
    TuningRecordDto,
};
use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-traits-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

/// 端口桩: 回一份固定成长账快照 (面形状与门语义的测试不需要真文件面)。
struct StubTraits {
    frame: TraitPanelDto,
    error: Option<String>,
}

#[async_trait]
impl TraitPanelQuery for StubTraits {
    async fn trait_frame(&self) -> Result<TraitPanelDto, String> {
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(self.frame.clone()),
        }
    }
}

fn sample_frame() -> TraitPanelDto {
    TraitPanelDto {
        values: TraitValuesDto {
            memory_fade: 1.25,
            curiosity_strength: 1.0,
            tone_saturation: 0.8,
            consolidation_cadence: 1.0,
        },
        locked: TraitLocksDto {
            memory_fade: true,
            curiosity_strength: false,
            tone_saturation: false,
            consolidation_cadence: false,
        },
        recent_moves: vec![TraitMoveDto {
            param: "memory_fade".to_string(),
            label: "遗忘衰减强度".to_string(),
            at_epoch_ms: 1_728_000_000_000,
            delta: 0.25,
            reason: "上调一档".to_string(),
        }],
        records: vec![TuningRecordDto {
            seq: 1,
            param: "memory_fade".to_string(),
            previous: 1.0,
            next: 1.25,
            reason: "上调一档".to_string(),
            at_epoch_ms: 1_728_000_000_000,
        }],
        skipped_records: 1,
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
        traits: Some(Arc::new(StubTraits {
            frame: sample_frame(),
            error: None,
        })),
        ..GatewayServices::default()
    }
}

/// 1. 帧字段集合: 现值/锁定态四杆齐 + 移动行/账面行逐字段钉死。
#[tokio::test]
async fn traits_endpoint_projects_values_locks_and_recent_moves() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/traits", None))
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
        vec![
            "locked",
            "recent_moves",
            "records",
            "skipped_records",
            "values"
        ],
        "帧五节 = 四滑杆现值 + 锁定态 + 最近移动 + 账面行 + 坏行跳过"
    );

    // 现值/锁定态: 四杆一根不少 (稳定词序)。
    let values = &body["values"];
    let locked = &body["locked"];
    for slider in [
        "memory_fade",
        "curiosity_strength",
        "tone_saturation",
        "consolidation_cadence",
    ] {
        assert!(values[slider].is_number(), "现值缺杆 {slider}");
        assert!(locked[slider].is_boolean(), "锁定态缺杆 {slider}");
    }
    assert_eq!(values["memory_fade"], 1.25);
    assert_eq!(locked["memory_fade"], true, "锁定态如实");

    // 最近移动行: `trait_report::TraitMove` 同形。
    let movement = &body["recent_moves"][0];
    let mut keys: Vec<&str> = movement
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["at_epoch_ms", "delta", "label", "param", "reason"]
    );
    assert_eq!(movement["param"], "memory_fade");
    assert_eq!(movement["label"], "遗忘衰减强度");
    assert_eq!(movement["delta"], 0.25);
    assert_eq!(movement["reason"], "上调一档");

    // 账面行: `TuningRecord` 同形。
    let record = &body["records"][0];
    let mut keys: Vec<&str> = record
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["at_epoch_ms", "next", "param", "previous", "reason", "seq"]
    );
    assert_eq!(record["seq"], 1);
    assert_eq!(record["next"], 1.25);
    assert_eq!(record["previous"], 1.0);

    assert_eq!(body["skipped_records"], 1, "坏行跳过计数如实 (不静默吞)");
}

/// 2. 空账 = 显式空态 (空账按空报 0, 不拿「—」冒充; 缺档 = 全解锁默认态)。
#[tokio::test]
async fn empty_growth_ledger_is_an_explicit_empty_state() {
    let frame = TraitPanelDto {
        values: TraitValuesDto {
            memory_fade: 1.0,
            curiosity_strength: 1.0,
            tone_saturation: 1.0,
            consolidation_cadence: 1.0,
        },
        locked: TraitLocksDto {
            memory_fade: false,
            curiosity_strength: false,
            tone_saturation: false,
            consolidation_cadence: false,
        },
        recent_moves: vec![],
        records: vec![],
        skipped_records: 0,
    };
    let services = GatewayServices {
        traits: Some(Arc::new(StubTraits { frame, error: None })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/traits", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["records"], serde_json::json!([]));
    assert_eq!(body["recent_moves"], serde_json::json!([]));
    assert_eq!(body["skipped_records"], 0, "空账按空报 0 (0 = 测得为零)");
    assert_eq!(body["locked"]["memory_fade"], false, "缺档 = 全解锁默认态");
}

/// 3. 令牌门同面: 强制态下与面板族同一门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn traits_endpoint_is_behind_the_gateway_token_gate() {
    let state = harness(
        stub_services(),
        GatewayTokenGate::Enforced(TOKEN.to_string()),
    )
    .await;
    let router = canonical_router_with_state(state);

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/traits", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_missing_key"), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/traits", Some(FORGED_TOKEN)))
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
            .oneshot(request("/v1/panel/traits", Some(TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"values\""), "{text}");
}

/// 4. 未接线 = 501 `unsupported` 诚实降级 (不假装有数据源)。
#[tokio::test]
async fn traits_endpoint_degrades_to_501_without_the_query_port() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/traits", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("\"unsupported\""), "{text}");
    assert!(text.contains("traits.read"), "{text}");
}

/// 5. 读失败 = 500 错误帧报真因 (锁定档坏档绝不读成全解锁放行)。
#[tokio::test]
async fn traits_endpoint_reports_read_failures_as_error_frames() {
    let services = GatewayServices {
        traits: Some(Arc::new(StubTraits {
            frame: sample_frame(),
            error: Some("trait lock state refused to parse: 坏档".to_string()),
        })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/traits", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{text}");
    assert!(text.contains("runtime_error"), "{text}");
    assert!(text.contains("坏档"), "真因如实进错误帧: {text}");
}

/// 6. 能力清单入册: `traits.read` 只读面, `supported/available` 随接线状态。
#[tokio::test]
async fn traits_read_is_registered_in_the_capability_manifest() {
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
        .find(|group| group["name"] == "traits")
        .expect("traits 分组入册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["id"], "traits.read");
    assert_eq!(cap["read"], true);
    assert_eq!(
        cap["write"], false,
        "traits.read 恒只读 (锁定/解锁走 CLI 显式动作)"
    );
    assert_eq!(cap["operations"], serde_json::json!(["read"]));
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
        .find(|group| group["name"] == "traits")
        .expect("traits 分组在册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["supported"], false);
    assert_eq!(cap["available"], false);
    assert!(cap["reason"].as_str().is_some(), "不可用须给机器可读真因");
}
