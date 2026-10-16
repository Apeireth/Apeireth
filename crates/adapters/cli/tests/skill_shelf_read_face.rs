//! 技能库只读面的真数据链 (组合根侧): `CliSkillShelf` 读 `SkillLibrary::scan`
//! (`<data_dir>/skills/*.json`, StoredDoc 拒开语义) → `GET /v1/panel/skills` 的
//! 元数据投影。面形状 / 令牌门 / 未接线 501 的契约钉在
//! `crates/adapters/gateway/tests/panel_skills_face.rs` (同一端口)。
//!
//! 这里钉三件事 (全是真文件面, 零桩):
//! 1. **真值元数据** —— 真卡七字段逐字上屏, 卡体 (`body`) 零泄漏 (不回卡体),
//!    排序确定 (卡名键 + id);
//! 2. **坏卡列拒不砖库** —— 坏 JSON / 串档 / 版本不符三种坏卡各自如实列拒
//!    (文件名 + 结构化原因), 好卡照常上屏, 整库不砖;
//! 3. **空库显式空** —— `skills: []` / `count: 0` / `refused: []`, 不 501 不缺字段。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_cli::gateway_panels::CliSkillShelf;
use apeireth_core::stored_doc::StoredDoc;
use apeireth_gateway::{
    build_gateway_state_with_services, canonical_router_with_state, GatewayServices,
    GatewayTokenGate,
};
use apeireth_runtime::canonical::Runtime;
use apeireth_tools_canonical::{
    SkillCard, SkillCardDraft, SkillLibrary, SkillSource, SkillSourceKind,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

const BODY_SENTINEL: &str = "BODY-SENTINEL-DO-NOT-LEAK";

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-skill-shelf-read-face-{tag}-{}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

fn draft(name: &str, body: &str) -> SkillCardDraft {
    SkillCardDraft {
        name: name.to_string(),
        summary: "换元后微分元同步替换".to_string(),
        triggers: vec!["换元".to_string(), "dx".to_string()],
        body: body.to_string(),
        source: SkillSource {
            kind: SkillSourceKind::Book,
            title: "数学分析讲义".to_string(),
        },
    }
}

/// 网关 HTTP 入口装配 (技能库指向同一数据目录 = 与 skill_search 同库)。
async fn router_over(dir: &Path) -> axum::Router {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        skills: Some(Arc::new(CliSkillShelf::under_data_dir(dir))),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::UnprotectedDefault;
    canonical_router_with_state(state)
}

async fn skills_body(router: axum::Router) -> (StatusCode, serde_json::Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/panel/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let body: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if !body.is_null() {
        assert!(
            !text.contains(BODY_SENTINEL),
            "读面不回卡体 (L2 封套面独占): {text}"
        );
    }
    (status, body)
}

/// 1. 真值元数据: 两卡七字段逐字上屏 + 确定性排序 + 卡体零泄漏。
#[tokio::test]
async fn the_metadata_face_projects_real_cards_without_bodies() {
    let dir = temp_dir("real");
    let library = SkillLibrary::under_data_dir(&dir);
    let first = library
        .create(&draft("alpha-technique", BODY_SENTINEL), 1_000)
        .unwrap();
    let second = library
        .create(&draft("beta-technique", BODY_SENTINEL), 2_000)
        .unwrap();

    let (status, body) = skills_body(router_over(&dir).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["count"], 2);
    let skills = body["skills"].as_array().expect("skills array");
    assert_eq!(skills.len(), 2);
    assert_eq!(body["refused"], serde_json::json!([]));

    // 确定性排序: 卡名键 + id (alpha 在前)。
    assert_eq!(skills[0]["name"], "alpha-technique");
    assert_eq!(skills[1]["name"], "beta-technique");

    let alpha = &skills[0];
    let mut keys: Vec<&str> = alpha
        .as_object()
        .unwrap()
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
        "元数据行 = 合同七字段 (无 body)"
    );
    assert_eq!(alpha["id"], first.card.body.id);
    assert_eq!(alpha["summary"], "换元后微分元同步替换");
    assert_eq!(alpha["triggers"], serde_json::json!(["换元", "dx"]));
    assert_eq!(alpha["source"]["kind"], "book");
    assert_eq!(alpha["source"]["title"], "数学分析讲义");
    assert_eq!(alpha["created_at"], 1_000);
    assert_eq!(alpha["version"], 1);
    assert_eq!(skills[1]["id"], second.card.body.id);
    assert_eq!(skills[1]["created_at"], 2_000);

    let _ = fs_err::remove_dir_all(&dir);
}

/// 2. 坏卡列拒不砖库: 三种坏卡 (坏 JSON / 串档 / 版本不符) 各自如实列拒,
/// 好卡照常上屏。
#[tokio::test]
async fn bad_cards_are_listed_as_refusals_without_bricking_the_library() {
    let dir = temp_dir("refuse");
    let library = SkillLibrary::under_data_dir(&dir);
    library
        .create(&draft("alpha-technique", BODY_SENTINEL), 1_000)
        .unwrap();

    // 坏 JSON: 解析不过 → 拒开。
    fs_err::write(dir.join("skills/broken.json"), b"{ not json").unwrap();
    // 串档: 信封身份与文件名不符 → 拒开。
    let stranger = SkillCard {
        id: "skill-0000000000000001".to_string(),
        name: "stranger".to_string(),
        summary: "串档卡".to_string(),
        triggers: vec![],
        body: BODY_SENTINEL.to_string(),
        source: SkillSource {
            kind: SkillSourceKind::Document,
            title: "陌生来源".to_string(),
        },
        created_at: 9,
    };
    let doc = StoredDoc::new("skill-card:somebody-else", 1, vec![1], stranger);
    fs_err::write(
        dir.join("skills/mismatched.json"),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();
    // 版本不符: 信封版本不在读端契约 → 拒开 (不静默迁移)。
    let future = SkillCard {
        id: "skill-0000000000000002".to_string(),
        name: "future".to_string(),
        summary: "未来版本卡".to_string(),
        triggers: vec![],
        body: BODY_SENTINEL.to_string(),
        source: SkillSource {
            kind: SkillSourceKind::Article,
            title: "未来来源".to_string(),
        },
        created_at: 9,
    };
    let doc = StoredDoc::new("skill-card:future", 99, vec![99], future);
    fs_err::write(
        dir.join("skills/future.json"),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();

    let (status, body) = skills_body(router_over(&dir).await).await;
    assert_eq!(status, StatusCode::OK, "坏卡不砖库, 读面照常 200: {body}");
    let skills = body["skills"].as_array().expect("skills array");
    assert_eq!(skills.len(), 1, "好卡照常上屏");
    assert_eq!(skills[0]["name"], "alpha-technique");
    assert_eq!(body["count"], 1);

    let refused = body["refused"].as_array().expect("refused array");
    assert_eq!(refused.len(), 3, "三种坏卡各自如实列拒");
    let mut files: Vec<&str> = refused
        .iter()
        .map(|entry| entry["file"].as_str().expect("file"))
        .collect();
    files.sort_unstable();
    assert_eq!(files, vec!["broken.json", "future.json", "mismatched.json"]);
    for entry in refused {
        assert!(
            entry["reason"].as_str().map(str::len).unwrap_or(0) > 1,
            "拒开原因如实非空: {entry}"
        );
    }

    let _ = fs_err::remove_dir_all(&dir);
}

/// 3. 空库 = 显式空态 (目录不存在也一样, 不 501 不缺字段)。
#[tokio::test]
async fn an_empty_skill_library_is_an_explicit_empty_state() {
    let dir = temp_dir("empty");
    let (status, body) = skills_body(router_over(&dir).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["skills"], serde_json::json!([]));
    assert_eq!(body["count"], 0);
    assert_eq!(body["refused"], serde_json::json!([]));
    let _ = fs_err::remove_dir_all(&dir);
}
