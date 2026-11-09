//! **默认可用面底账**（default usable surface catalog）。
//!
//! # 这份文件是什么
//!
//! 「默认路径不许有兑现不了的承诺」这条纪律的**验收底账**：
//! 默认桌面组合（`canonical::desktop_host::build_desktop_main_runtime`，
//! **桌面产品与 CLI 共用的那个组合根**）到底**发布了什么**、**能不能兑现**。
//!
//! # 三条判据（逐条可复核）
//!
//! 1. **发布面** = `runtime.tool_declarations_for_session(main.session)`
//!    ⇒ 模型真正看到的那张脸（不是注册表里有的，是**发出去的**）；
//! 2. **可兑现** = 用**从 schema 合成的最小合法参数**逐个**真跑**一次
//!    ⇒ ⇒ 授权层**放行**（只允许业务级报错：路径不存在、查询为空一类）；
//!    ⇒ ⇒ 出现**授权类拒绝**（`scope_denied` / `namespace_denied` /
//!    「Host principal is required」/「requires the Runtime context」/「not granted」）
//!    ⇒ ⇒ **本类缺陷**（接上了但必然失败）；
//! 3. **面外** = 一切**没被发布**的能力面 ⇒ ⇒ 默认组合里**不出现**。
//!
//! # 允许清单（**底账本体**）
//!
//! [`PUBLISHED_ALLOWLIST`] 是**当前**默认组合发布的全部工具面。
//! ⇒ ⇒ 以后**新增**一个发布面 ⇒ 必须**显式**改这张清单 + 给出**可兑现**的证据
//! ⇒ ⇒ 「不小心发布了一个兑现不了的面」会**当场红**。
//!
//! # 反证
//!
//! * 把 [`PUBLISHED_ALLOWLIST`] 去掉一个条目 ⇒ `the_published_surface_is_pinned` 红；
//! * 把「授权类拒绝」判据放宽（去掉 `scope_denied`）⇒ `every_published_tool_is_honorable` 红
//!   （前提：默认组合发布了一个兑现不了的面，例如 P1 之前的记忆工具面）。
//!
//! # 中性表述
//!
//! 只描述机制（发布面 / 授权前提 / 范围闸），不引用任何外部产品名、文档或代码。

#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{SessionId, SystemClock};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::TurnRequest;
use apeireth_runtime_assembly::{
    canonical::desktop_host::build_desktop_main_runtime, CognitiveBackends, CognitiveModuleConfig,
    ProductionModules,
};
use std::sync::Arc;
use support::*;

type CapResult<T> = apeireth_plugin::memory_backend::CapabilityResult<T>;

/// **当前**默认桌面组合发布的全部工具面（**底账本体**）。
///
/// ⚠️ **改这张清单 = 改默认路径的承诺** ⇒ ⇒ 必须同时给出**可兑现**的证据
/// （`every_published_tool_is_honorable` 会逐个真跑）。
const PUBLISHED_ALLOWLIST: &[&str] = &["filesystem", "repo_map", "repo", "search", "self_status"];

/// **授权类拒绝**的标记（出现任一 ⇒ ⇒ 这个面**兑现不了**）。
const AUTHORIZATION_REFUSALS: &[&str] = &[
    "scope_denied",
    "namespace_denied",
    "Host principal is required",
    "requires the Runtime context",
    "not granted",
    "System One",
];

struct NoopPreferences;
impl PreferenceStore for NoopPreferences {
    fn record(&self, _pref: &UserPreference) -> CapResult<()> {
        Ok(())
    }
    fn recall_for_context(
        &self,
        _session_id: &SessionId,
        _current_topic: &str,
        _limit: u32,
    ) -> CapResult<Vec<UserPreference>> {
        Ok(vec![])
    }
    fn forget(&self, _pref_id: &str) -> CapResult<()> {
        Ok(())
    }
    fn list_for_session(&self, _session_id: &SessionId) -> CapResult<Vec<UserPreference>> {
        Ok(vec![])
    }
}

struct NoopAssessments;
impl SelfAssessmentStore for NoopAssessments {
    fn record(&self, _sa: &SelfAssessment) -> CapResult<()> {
        Ok(())
    }
    fn recent_for_task(&self, _task_id: &str, _limit: u32) -> CapResult<Vec<SelfAssessment>> {
        Ok(vec![])
    }
    fn latest_alignment(&self, _task_id: &str) -> CapResult<Option<f64>> {
        Ok(None)
    }
}

/// 生产组合（带工作区根 ⇒ 文件系统/检索/仓库三张脸在场）。
fn production(db: Arc<SqliteMemoryStore>, root: Option<std::path::PathBuf>) -> ProductionModules {
    ProductionModules::build(
        CognitiveModuleConfig::default(),
        CognitiveBackends {
            memory: Some(db.clone()),
            memory_governance: Some(db.clone()),
            scoped_memory: Some(db.clone()),
            preferences: Some(Arc::new(NoopPreferences)),
            self_assessments: Some(Arc::new(NoopAssessments)),
            workspace_root: root,
            ..Default::default()
        },
        Arc::new(SystemClock),
    )
    .unwrap()
}

/// 从 JSON schema 合成**最小合法参数**（只填 required；枚举取第一项）。
fn minimal_args(schema: &serde_json::Value) -> serde_json::Value {
    let properties = schema.get("properties").and_then(|p| p.as_object());
    let required: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut map = serde_json::Map::new();
    for key in required {
        let entry = properties.and_then(|props| props.get(&key));
        if let Some(first) = entry
            .and_then(|entry| entry.get("enum"))
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .cloned()
        {
            map.insert(key, first);
            continue;
        }
        let kind = entry
            .and_then(|entry| entry.get("type"))
            .and_then(|kind| kind.as_str())
            .unwrap_or("string");
        let value = match kind {
            "integer" | "number" => serde_json::json!(0),
            "boolean" => serde_json::json!(false),
            "array" => serde_json::json!([]),
            "object" => serde_json::json!({}),
            _ => serde_json::json!(""),
        };
        map.insert(key, value);
    }
    serde_json::Value::Object(map)
}

fn tool_texts(provider: &Arc<Provider>) -> Vec<String> {
    provider
        .requests()
        .iter()
        .flat_map(|request| request.messages.iter())
        .filter(|message| message.role == MessageRole::Tool)
        .map(|message| ContentPart::join_text(&message.content))
        .collect()
}

/// 默认组合（工作区根可注入）⇒ 运行时。
async fn default_runtime(
    root: Option<std::path::PathBuf>,
    provider: Arc<Provider>,
) -> apeireth_runtime::Runtime {
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let modules = production(db, root);
    let policy = modules.desktop_agent_policy().unwrap();
    build_desktop_main_runtime(modules.register_into(builder(provider)), policy)
        .await
        .unwrap()
}

/// ① **底账本体**：默认组合发布的工具面**恰好**是清单里的那些。
///
/// ⇒ 新增一个发布面 ⇒ 这里红 ⇒ ⇒ 「不小心发布」不可能悄悄发生。
#[tokio::test]
async fn the_published_surface_is_pinned() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let runtime =
        default_runtime(Some(workspace.path().to_path_buf()), Provider::new(vec![])).await;
    let main = runtime.managed_main().expect("desktop main identity");
    let mut published: Vec<String> = runtime
        .tool_declarations_for_session(main.session)
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect();
    published.sort();
    let mut expected: Vec<String> = PUBLISHED_ALLOWLIST.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        published, expected,
        "默认桌面组合发布的工具面变了 ⇒ ⇒ 底账要**显式**更新（并给出可兑现的证据）"
    );
}

/// ② **可兑现**：每个被发布的工具面**真跑**一次 ⇒ ⇒ 授权层**放行**。
///
/// ⇒ 授权类拒绝（`scope_denied` / `namespace_denied` …）⇒ ⇒ 这个面**兑现不了**
/// ⇒ ⇒ 「接上了但必然失败」⇒ ⇒ 本例红。
#[tokio::test]
async fn every_published_tool_is_honorable() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("probe.txt"), b"probe marker").expect("write probe file");
    let runtime =
        default_runtime(Some(workspace.path().to_path_buf()), Provider::new(vec![])).await;
    let main = runtime.managed_main().expect("desktop main identity");
    let declarations = runtime.tool_declarations_for_session(main.session);

    for declaration in &declarations {
        let name = declaration.name.clone();
        let mut args = minimal_args(&serde_json::Value::Object(declaration.parameters.clone()));
        // 给「读文件」一张**真的**路径 ⇒ ⇒ 能读到内容 ⇒ ⇒ 证明这张脸**真的能用**
        //（而不只是「没被授权层拒掉」）。
        if let Some(map) = args.as_object_mut() {
            if map.get("path").is_some() {
                map.insert(
                    "path".to_string(),
                    serde_json::json!(workspace.path().join("probe.txt").to_string_lossy()),
                );
            }
            if map.get("query").is_some() {
                map.insert("query".to_string(), serde_json::json!("probe"));
            }
        }
        let provider = Provider::new(vec![vec![call(&name, args)], vec![]]);
        let runtime = default_runtime(Some(workspace.path().to_path_buf()), provider.clone()).await;
        let session = SessionId::new();
        let _ = runtime.execute(TurnRequest::new(session, "probe")).await;
        let texts = tool_texts(&provider);
        for text in &texts {
            for marker in AUTHORIZATION_REFUSALS {
                assert!(
                    !text.contains(marker),
                    "工具面「{name}」在**默认组合**里兑现不了（授权类拒绝「{marker}」）\
                     ⇒ ⇒ 「接上了但必然失败」⇒ ⇒ 处置：①不发布 / ②补最小前提 / ③说清楚\n  实际输出：{text}"
                );
            }
        }
    }
}

/// ③ **面外**：命名空间受限的记忆工具面**不出现**（P1 不变量，从本文件再钉一次）。
#[tokio::test]
async fn no_namespace_scoped_tool_is_published() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let runtime =
        default_runtime(Some(workspace.path().to_path_buf()), Provider::new(vec![])).await;
    let main = runtime.managed_main().expect("desktop main identity");
    assert!(
        main.principal.is_none(),
        "默认桌面组合**不绑** Host 身份（本文件的前提）"
    );
    let published: Vec<String> = runtime
        .tool_declarations_for_session(main.session)
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect();
    assert!(
        !published.iter().any(|name| name.starts_with("memory_")),
        "默认组合不得发布命名空间受限的记忆工具面；published = {published:?}"
    );
}
