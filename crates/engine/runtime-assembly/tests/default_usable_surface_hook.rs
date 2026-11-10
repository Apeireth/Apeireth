//! **固定检索 Hook 在默认组合里兑现不了** ⇒ ⇒ 默认组合**装配期拒开**。
//!
//! # 这份文件钉什么
//!
//! 固定检索 Hook 的开关在数据根的 `memory-hook.json`（`automatic_retrieval`）。
//! Hook 每回合发出**记忆工具面自己**的 `memory_retrieve` 调用 ⇒ ⇒ 能不能兑现
//! 由**分发器的范围闸**判 ⇒ ⇒ 需要**两个**前提同时成立：
//!
//! ① 该组合**绑了 Host 身份**（授权来自执行上下文的 `AgentPrincipal`）；
//! ② 该组合**发布了**记忆工具面（否则那个调用**不在会话范围**里 ⇒ 范围闸拒）。
//!
//! ⚠️ **默认桌面组合两个都不满足**（不绑身份 + 默认组合不发布记忆工具面）
//! ⇒ ⇒ 开关一开 ⇒ ⇒ Hook 每回合发一条 **`scope_denied` 失败帧**
//! ⇒ ⇒ 「**开了但兑现不了**」= 与「没接」同一类缺陷。
//!
//! ⇒ ⇒ **处置 = ①不发布 + ③说清楚**：默认组合在**装配期拒开**，
//! 并给出**可执行**的提示（不是让 Host 每回合看一条失败帧）。
//!
//! # 反证
//!
//! 去掉 `canonical::desktop_host.rs` 里的拒开闸 ⇒ 本文件第一例红
//! （组合能起来 ⇒ 「开了但兑现不了」又回来了）。
//!
//! # 为什么本文件是独立的测试二进制
//!
//! 它动**进程级** `APEIRETH_DATA_DIR`（数据根的唯一来源）⇒ ⇒ 独占一个二进制。

#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{SessionId, SystemClock};
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_plugin::self_assessment::{SelfAssessment, SelfAssessmentStore};
use apeireth_runtime_assembly::{
    canonical::desktop_host::build_desktop_main_runtime_at, CognitiveBackends,
    CognitiveModuleConfig, ProductionModules,
};
use std::sync::Arc;
use support::*;

type CapResult<T> = apeireth_plugin::memory_backend::CapabilityResult<T>;

/// 开关文件（与 `memory_assembly.rs` 的常量同形；信封格式照 `StoredDoc`）。
const SWITCH: &[u8] = br#"{"name":"apeireth.memory.hook","version":1,"compatible_versions":[1],"body":{"automatic_retrieval":true}}"#;

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

/// 把数据根指到临时目录并写好开关文件（进程级 env ⇒ 本文件独占一个测试二进制）。
///
/// ⚠️ 返回的目录**同时**是 env 指向的根**和**注入给组合的根 ⇒ ⇒ 生产上两者
/// 同一个（`build_desktop_main_runtime` 注入的就是默认数据根）⇒ ⇒ 本文件
/// 复刻的就是生产形态。
fn arm_switch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-hook-surface-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("memory-hook.json"), SWITCH).expect("write");
    std::env::set_var("APEIRETH_DATA_DIR", &dir);
    dir
}

/// **干净**的数据根（没有开关文件）。
fn clean_root() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-hook-clean-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

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

/// ① **反证**：开关打开 + **默认桌面组合** ⇒ ⇒ **装配期拒开**（可执行提示）。
///
/// 反证口径：去掉 `desktop_host.rs` 的拒开闸 ⇒ 本例红
/// ⇒ ⇒ 「开了但兑现不了」又回来了（每回合一条 `scope_denied` 失败帧）。
#[tokio::test]
async fn the_default_desktop_refuses_an_arm_hook_it_cannot_honor() {
    let data_root = arm_switch();
    let workspace = tempfile::tempdir().expect("temp workspace");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let modules = production(db, Some(workspace.path().to_path_buf()));
    let policy = modules.desktop_agent_policy().unwrap();
    let outcome = build_desktop_main_runtime_at(
        modules.register_into(builder(Provider::new(vec![]))),
        policy,
        &data_root,
    )
    .await;
    let error = outcome.expect_err(
        "开关打开 + 默认桌面组合（不绑身份、不发布记忆工具面）⇒ ⇒ 必须**拒开**，\
         否则 Hook 每回合发一条 scope_denied 失败帧",
    );
    let message = format!("{error}");
    assert!(
        message.contains("memory-hook.json") && message.contains("scope_denied"),
        "拒开信息必须**说清楚**（点名开关文件 + 点名失败形态）⇒ ⇒ 实际：{message}"
    );
}

/// ② **默认态**：开关**没**开 ⇒ ⇒ 默认桌面组合照常起来（零行为变化）。
#[tokio::test]
async fn the_default_desktop_still_starts_without_the_switch() {
    let data_root = clean_root();
    let workspace = tempfile::tempdir().expect("temp workspace");
    let db = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let modules = production(db, Some(workspace.path().to_path_buf()));
    let policy = modules.desktop_agent_policy().unwrap();
    let runtime = build_desktop_main_runtime_at(
        modules.register_into(builder(Provider::new(vec![]))),
        policy,
        &data_root,
    )
    .await
    .expect("开关没开 ⇒ 默认组合必须照常起来（零行为变化）");
    assert!(
        runtime.managed_main().is_some(),
        "默认桌面组合必须有托管身份"
    );
}
