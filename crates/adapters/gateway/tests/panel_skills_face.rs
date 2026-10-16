//! 技能库只读面的面契约: `GET /v1/panel/skills` 的元数据形状 (不回卡体) +
//! 显式空态 + W2-② 令牌门 + 未接线 501 诚实降级 + `skills.read` 入能力清单。
//!
//! 这里钉五件事:
//! 1. **元数据形状** —— `{skills:[{id,name,summary,triggers,source,created_at,version}],
//!    count, refused}`; 卡体 (`body`) **不回** (卡体是 L2 封套面, 读面只给元数据);
//! 2. **空库显式空** —— `skills: []` / `count: 0` / `refused: []`, 不缺字段、不 501;
//! 3. **令牌门同面** —— 缺 Bearer 401 `auth_missing_key`, 伪造 401 `auth_invalid_key`,
//!    真令牌 200 (与面板族同一把门);
//! 4. **未接线如实** —— 端口未注入 = 501 `unsupported` (不假装有数据源);
//! 5. **能力清单入册** —— `skills.read` 只读面, `supported/available` 随接线状态。
//!
//! 真数据面 (StoredDoc 拒开语义: 坏卡列拒不砖库) 的端到端钉在组合根侧
//! `crates/adapters/cli/tests/skill_shelf_read_face.rs` (同一端口, 真 `SkillLibrary`)。

use std::sync::Arc;

use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices, GatewayState,
    GatewayTokenGate, SkillMetaDto, SkillRefusalDto, SkillShelfQuery, SkillShelfScanDto,
    SkillSourceDto,
};
use apeireth_runtime::canonical::Runtime;
use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const TOKEN: &str = "panel-skills-token-0123456789abcdef";
const FORGED_TOKEN: &str = "forged-token-fedcba9876543210";

/// 端口桩: 回一份固定扫描结果 (面形状与门语义的测试不需要真文件面)。
struct StubSkills {
    scan: SkillShelfScanDto,
}

#[async_trait]
impl SkillShelfQuery for StubSkills {
    async fn list_skills(&self) -> Result<SkillShelfScanDto, String> {
        Ok(self.scan.clone())
    }
}

fn sample_scan() -> SkillShelfScanDto {
    SkillShelfScanDto {
        skills: vec![SkillMetaDto {
            id: "skill-1f2e3d4c5b6a7988".to_string(),
            name: "发布前检查单".to_string(),
            summary: "发布前的最小检查口".to_string(),
            triggers: vec!["发布".to_string(), "上线检查".to_string()],
            source: SkillSourceDto {
                kind: "book".to_string(),
                title: "工程手册".to_string(),
            },
            created_at: 1_728_000_000_000,
            version: 1,
        }],
        count: 1,
        refused: vec![SkillRefusalDto {
            file: "broken.json".to_string(),
            reason: "拒开: JSON 解析失败".to_string(),
        }],
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
        skills: Some(Arc::new(StubSkills {
            scan: sample_scan(),
        })),
        ..GatewayServices::default()
    }
}

/// 1. 元数据形状: 字段集合逐字钉死, `body` 零出现 (无卡体字段断言)。
#[tokio::test]
async fn skills_endpoint_returns_metadata_only_never_card_bodies() {
    let state = harness(stub_services(), GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/skills", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["count"], 1);
    let skills = body["skills"].as_array().expect("skills array");
    assert_eq!(skills.len(), 1);
    let skill = &skills[0];
    let mut keys: Vec<&str> = skill
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "created_at",
            "id",
            "name",
            "source",
            "summary",
            "triggers",
            "version"
        ],
        "元数据行字段集合 = 合同七字段, 不多不少"
    );
    assert!(
        !text.contains("\"body\""),
        "读面不回卡体 (卡体是 L2 封套面): {text}"
    );
    assert_eq!(skill["id"], "skill-1f2e3d4c5b6a7988");
    assert_eq!(skill["name"], "发布前检查单");
    assert_eq!(skill["summary"], "发布前的最小检查口");
    assert_eq!(skill["triggers"], serde_json::json!(["发布", "上线检查"]));
    assert_eq!(skill["source"]["kind"], "book");
    assert_eq!(skill["source"]["title"], "工程手册");
    assert_eq!(skill["created_at"], 1_728_000_000_000u64);
    assert_eq!(skill["version"], 1);
    let refused = body["refused"].as_array().expect("refused array");
    assert_eq!(refused.len(), 1, "拒开侧车如实列拒");
    assert_eq!(refused[0]["file"], "broken.json");
    assert!(refused[0]["reason"].as_str().unwrap().len() > 1);
}

/// 2. 空库 = 显式空态 (字段一个不少, 不缺字段也不 501)。
#[tokio::test]
async fn empty_skill_library_is_an_explicit_empty_state() {
    let services = GatewayServices {
        skills: Some(Arc::new(StubSkills {
            scan: SkillShelfScanDto {
                skills: vec![],
                count: 0,
                refused: vec![],
            },
        })),
        ..GatewayServices::default()
    };
    let state = harness(services, GatewayTokenGate::UnprotectedDefault).await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/skills", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["skills"], serde_json::json!([]));
    assert_eq!(body["count"], 0);
    assert_eq!(body["refused"], serde_json::json!([]));
}

/// 3. 令牌门同面: 强制态下与面板族同一门 (401 矩阵 + 真令牌 200)。
#[tokio::test]
async fn skills_endpoint_is_behind_the_gateway_token_gate() {
    let state = harness(
        stub_services(),
        GatewayTokenGate::Enforced(TOKEN.to_string()),
    )
    .await;
    let router = canonical_router_with_state(state);

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/skills", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    assert!(text.contains("auth_missing_key"), "{text}");

    let (status, text) = status_and_body(
        router
            .clone()
            .oneshot(request("/v1/panel/skills", Some(FORGED_TOKEN)))
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
            .oneshot(request("/v1/panel/skills", Some(TOKEN)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("\"skills\""), "{text}");
}

/// 4. 未接线 = 501 `unsupported` 诚实降级 (不假装有数据源)。
#[tokio::test]
async fn skills_endpoint_degrades_to_501_without_the_query_port() {
    let state = harness(
        GatewayServices::default(),
        GatewayTokenGate::UnprotectedDefault,
    )
    .await;
    let router = canonical_router_with_state(state);
    let (status, text) = status_and_body(
        router
            .oneshot(request("/v1/panel/skills", None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{text}");
    assert!(text.contains("\"unsupported\""), "{text}");
    assert!(text.contains("skills.read"), "{text}");
}

/// 5. 能力清单入册: `skills.read` 只读面, `supported/available` 随接线状态。
#[tokio::test]
async fn skills_read_is_registered_in_the_capability_manifest() {
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
        .find(|group| group["name"] == "skills")
        .expect("skills 分组入册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["id"], "skills.read");
    assert_eq!(cap["read"], true);
    assert_eq!(cap["write"], false);
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
        .find(|group| group["name"] == "skills")
        .expect("skills 分组在册");
    let cap = &group["capabilities"][0];
    assert_eq!(cap["supported"], false);
    assert_eq!(cap["available"], false);
    assert!(cap["reason"].as_str().is_some(), "不可用须给机器可读真因");
}
