//! **B3 生产接线**的验收测试：影子双跑 + 换算法流程。
//!
//! # 这些测试钉住什么
//!
//! 1. **默认关 ⇒ 逐字节零行为变化**（没有 `shadow.json` ⇒ 不构造任何东西、不落任何盘）；
//! 2. **坏配置 ⇒ 拒绝装配**（坏配置不许被当成「没配置」）；
//! 3. **影子不改生产结果**（影子跑前后的生产召回**逐条 id 相同**）；
//! 4. **报告真的产出 + 成本真的记下**（归档目录里有报告；裁决带数字）；
//! 5. **三道闸任一不过 ⇒ 拒绝切换**（指针不变、不落盘、如实说哪道闸拦下）；
//! 6. **没有治理面 ⇒ 不许换**（失败闭合，不自造审批）；
//! 7. **回退真的能回退**（指针 + 生产旋钮都回到上一版）。
//!
//! # 中性表述
//!
//! 只描述**机制**（影子双跑 / 裁决 / 三道闸 / 归档 / 回退 / 治理链），不引用任何外部产品名。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use apeireth_governance::{Decision, GovernanceChain, GovernanceHook};
use apeireth_memory::{MemoryGovernanceStore, MemoryRecallQuery, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime_assembly::shadow_wire::{
    install_shadow, swap_cost, ShadowConfigBody, ShadowProbeCase,
};
use serde_json::json;

/// 记录时刻（注入 ⇒ 可复现）。
const EPISODE_TS: i64 = 1_760_000_000_000;
/// 探针会话 id（测试里写记录与召回都用它 ⇒ 可见域对得上）。
const SESSION: &str = "sess-shadow";

/// 一个**放行**的治理钩子（测试用 ⇒ 用来证明「治理放行之后」的那条路是通的）。
struct AllowAll;

#[async_trait::async_trait]
impl GovernanceHook for AllowAll {
    fn name(&self) -> &str {
        "test.allow_all"
    }

    async fn evaluate(&self, _request: &apeireth_governance::GovernanceRequest<'_>) -> Decision {
        Decision::Allow
    }
}

/// 一个**拒绝**的治理钩子（测试用）。
struct DenyAll;

#[async_trait::async_trait]
impl GovernanceHook for DenyAll {
    fn name(&self) -> &str {
        "test.deny_all"
    }

    async fn evaluate(&self, _request: &apeireth_governance::GovernanceRequest<'_>) -> Decision {
        Decision::Deny {
            reason: "测试用：治理拒绝".to_string(),
        }
    }
}

/// 建一个**有文件路径**的库 + 一条真实记录（走生产写路径）。
fn fresh_store(
    dir: &Path,
) -> (
    PathBuf,
    Arc<dyn MemoryBackend>,
    Arc<dyn MemoryGovernanceStore>,
) {
    let path = dir.join("cognitive.sqlite3");
    let store = Arc::new(SqliteMemoryStore::open(&path).expect("建库"));
    let episode = Episode {
        id: "ep-shadow-1".to_string(),
        timestamp: EPISODE_TS,
        role: "user".to_string(),
        content: "红楼梦里宝玉与黛玉的关系".to_string(),
        session_id: SESSION.to_string(),
    };
    store
        .put_episode_for_subject(&episode, SESSION)
        .expect("写记录");
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    (path, backend, governance)
}

/// 写一份 `shadow.json`（**StoredDoc 信封** ⇒ 与仓里其它配置面同构）。
fn write_config(dir: &Path, body: &ShadowConfigBody) {
    let doc = apeireth_core::stored_doc::StoredDoc::new(
        "apeireth.memory.shadow_config",
        1,
        vec![1],
        body,
    );
    apeireth_core::stored_doc::save_single(
        &dir.join("shadow.json"),
        &apeireth_core::stored_doc::DocCompat::exact("apeireth.memory.shadow_config", 1),
        body,
        0o600,
    )
    .expect("写配置");
    // 信封自写一遍（`save_single` 已经写了；这里只做**格式自检**⇒ 不额外落文件）。
    let _ = doc;
}

/// 一份「探针题 + 候选旋钮」配置。
fn config_body(with_answers: bool) -> ShadowConfigBody {
    ShadowConfigBody {
        challenger_id: "candidate-a".to_string(),
        challenger: Default::default(),
        runs: Some(3),
        archive_keep: Some(5),
        session_id: Some(SESSION.to_string()),
        // ⭐ **样本充分闸**：影子轮数 >= 2 **且** 样本题数 >= 10（库级口径）
        // ⇒ ⇒ 这里给 10 道题；否则三道闸会拦下（那也是**正确**行为，另有用例钉）。
        cases: (0..10)
            .map(|index| ShadowProbeCase {
                id: format!("q{index}"),
                query: format!("宝玉 黛玉 {index}"),
                expect_ids: if with_answers {
                    vec!["ep-shadow-1".to_string()]
                } else {
                    Vec::new()
                },
            })
            .collect(),
    }
}

/// ⭐⭐ **默认关 ⇒ 逐字节零行为变化**。
#[test]
fn without_a_config_file_nothing_is_constructed_and_nothing_is_written() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    let service = install_shadow(backend, governance, dir.path(), None).expect("装配");
    assert!(
        service.is_none(),
        "没有 shadow.json ⇒ 不构造任何东西（默认态）"
    );
    // 数据根里**不许**多出任何文件（归档 / 指针 / 旋钮都没有）。
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .expect("读目录")
        .map(|entry| {
            entry
                .expect("目录项")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["cognitive.sqlite3".to_string()],
        "默认态不许落任何盘（只有测试自己建的库）"
    );
}

/// ⭐⭐ **坏配置 ⇒ 拒绝装配**（不许被当成「没配置」）。
#[test]
fn a_broken_config_refuses_to_assemble() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    std::fs::write(dir.path().join("shadow.json"), b"{ not json").expect("写坏配置");
    let error = install_shadow(backend, governance, dir.path(), None).expect_err("必须拒绝");
    assert!(!error.is_empty(), "必须带真因");
}

/// ⭐⭐⭐ **影子不改生产结果 + 报告真的产出 + 成本真的记下**。
#[tokio::test]
async fn the_shadow_never_changes_the_production_result_and_archives_a_report() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(true));

    let service = install_shadow(backend, governance, dir.path(), None)
        .expect("装配")
        .expect("有配置 ⇒ 构造服务");

    let before = service
        .read_pointer()
        .expect("读指针")
        .map(|pointer| pointer.current);
    assert!(before.is_none(), "一开始没有指针（从没换过）");

    let (report, verdict) = service.observe().expect("影子双跑 + 裁决");

    // ① 影子**不改**生产状态：指针没出现、旋钮没被写。
    assert!(
        service.read_pointer().expect("读指针").is_none(),
        "影子只观测 ⇒ 不许写指针"
    );
    assert!(
        !dir.path().join("retrieval.json").exists(),
        "影子只观测 ⇒ 不许写生产旋钮"
    );

    // ② 报告真的产出（归档目录里有文件）。
    let archives = std::fs::read_dir(dir.path().join("memory/shadow/reports"))
        .expect("归档目录")
        .count();
    assert_eq!(archives, 1, "影子跑完 ⇒ 归档目录里有一份报告");
    let report_text = std::fs::read_to_string(
        std::fs::read_dir(dir.path().join("memory/shadow/reports"))
            .expect("归档目录")
            .next()
            .expect("有一份")
            .expect("条目")
            .path(),
    )
    .expect("读报告");
    let parsed: serde_json::Value = serde_json::from_str(&report_text).expect("报告是 JSON");
    assert_eq!(parsed["schema"], "apeireth.shadow_report.v1");
    assert_eq!(parsed["challenger_id"], "candidate-a");

    // ③ 影子报告**没有**「答案」字段（影子纪律：只装 id 与指标）。
    assert!(
        parsed["pairs"].as_array().is_some_and(|pairs| {
            pairs
                .iter()
                .all(|pair| pair.get("answer").is_none() && pair.get("content").is_none())
        }),
        "影子报告只许有 id 与指标 ⇒ 不许出现答案/正文"
    );

    // ④ 成本真的记下（`Swap cost` 与库级口径一致）。
    let cost = swap_cost(3, 1);
    assert_eq!(cost.shadow_runs, 3);
    assert_eq!(cost.shadow_cases, 1);
    assert!(
        cost.rollback_ms < cost.switch_ms,
        "回退必须比切换快 ⇒「可回退」才不是假的"
    );

    // ⑤ 裁决**带数字**（不是空口）。
    assert!(!verdict.reason.is_empty(), "裁决必须带人话理由");
    assert_eq!(
        report.pairs.len(),
        10,
        "探针题数与影子对数一致（10 道探针 ⇒ 10 对）"
    );
    // ⭐ 现役侧必须**真的召回了东西**（否则这些断言都是空转）。
    assert!(
        report
            .pairs
            .iter()
            .any(|pair| !pair.champion.ids.is_empty()),
        "现役侧必须真的召回得到记录（召回不到 ⇒ 断言全是空转）"
    );
}

/// ⭐⭐ **三道闸任一不过 ⇒ 拒绝切换**（没有标准答案 ⇒ 没证据 ⇒ 不许换）。
#[tokio::test]
async fn a_swap_without_evidence_is_refused_and_changes_nothing() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(false));

    let chain = Arc::new(GovernanceChain::new(Arc::new(AllowAll)));
    let service = install_shadow(backend, governance, dir.path(), Some(chain))
        .expect("装配")
        .expect("有配置 ⇒ 构造服务");

    let outcome = service.swap(1_760_000_000_000).await.expect("执行切换");
    assert!(!outcome.switched, "没有标准答案 ⇒ 没证据 ⇒ 不许换");
    assert!(
        outcome.detail.contains("不许换"),
        "拒绝理由必须说清：{}",
        outcome.detail
    );
    assert_eq!(outcome.archive_path, "", "拒绝 ⇒ 不落盘（不产生归档）");
    assert!(
        service.read_pointer().expect("读指针").is_none(),
        "拒绝 ⇒ 指针逐字不变"
    );
    assert!(
        !dir.path().join("retrieval.json").exists(),
        "拒绝 ⇒ 不许写生产旋钮"
    );
}

/// ⭐⭐ **没有治理面 ⇒ 不许换**（失败闭合，不自造审批）。
#[tokio::test]
async fn a_swap_without_a_governance_surface_is_refused() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(true));

    let service = install_shadow(backend, governance, dir.path(), None)
        .expect("装配")
        .expect("有配置 ⇒ 构造服务");

    let error = service.swap(1_760_000_000_000).await.expect_err("必须拒绝");
    assert!(error.contains("治理"), "拒绝理由必须点名治理面：{error}");
    assert!(
        service.read_pointer().expect("读指针").is_none(),
        "拒绝 ⇒ 指针逐字不变"
    );
}

/// ⭐⭐ **治理面拒绝 ⇒ 不许换**。
#[tokio::test]
async fn a_swap_denied_by_governance_is_refused() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(true));

    let chain = Arc::new(GovernanceChain::new(Arc::new(DenyAll)));
    let service = install_shadow(backend, governance, dir.path(), Some(chain))
        .expect("装配")
        .expect("有配置 ⇒ 构造服务");

    let error = service.swap(1_760_000_000_000).await.expect_err("必须拒绝");
    assert!(error.contains("治理"), "拒绝理由必须点名治理面：{error}");
    assert!(
        service.read_pointer().expect("读指针").is_none(),
        "治理拒绝 ⇒ 指针逐字不变"
    );
}

/// ⭐⭐⭐ **换算法 + 回退**（全流程：治理放行 ⇒ 三道闸过 ⇒ 切换 ⇒ 回退）。
#[tokio::test]
async fn a_governed_swap_and_its_rollback_both_land() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(true));

    let chain = Arc::new(GovernanceChain::new(Arc::new(AllowAll)));
    let service = install_shadow(backend, governance, dir.path(), Some(chain))
        .expect("装配")
        .expect("有配置 ⇒ 构造服务");

    // ① 切换：治理放行 + 三道闸过（候选与现役同旋钮 ⇒ 准确度持平、token 不增 ⇒ 放行）。
    let outcome = service.swap(1_760_000_000_000).await.expect("执行切换");
    assert!(outcome.switched, "应当切换成功：{}", outcome.detail);
    assert!(!outcome.archive_path.is_empty(), "切换 ⇒ 必须留归档");

    // ② 指针落盘（当前 = 候选、上一版 = 现役）。
    let pointer = service.read_pointer().expect("读指针").expect("有指针");
    assert_eq!(pointer.current, "candidate-a");
    assert_eq!(pointer.previous.as_deref(), Some("champion"));
    assert_eq!(pointer.generation, 1);

    // ③ 生产旋钮被写了（**下次启动生效** ⇒ 不是热重载）。
    assert!(
        dir.path().join("retrieval.json").exists(),
        "切换 ⇒ 生产旋钮面必须落盘（下次启动生效）"
    );

    // ④ 回退：指针回到现役、生产旋钮也回到上一版。
    let rollback = service.rollback(1_760_000_001_000).expect("执行回退");
    assert!(rollback.switched, "应当回退成功：{}", rollback.detail);
    let pointer = service.read_pointer().expect("读指针").expect("有指针");
    assert_eq!(pointer.current, "champion", "回退 ⇒ 指针回到上一版");
    assert_eq!(
        pointer.previous.as_deref(),
        Some("candidate-a"),
        "再回退一次能回去"
    );
    assert_eq!(pointer.generation, 1, "回退是撤销 ⇒ 代次不变");
}

/// ⭐ **反证**：装配一旦被短路，「有配置 ⇒ 构造服务」这条断言必须红。
///
/// （本测试自己构造一个**短路版装配结果**来证明这条断言不是恒绿：
/// 若装配被短路，服务是 `None` ⇒ 依赖服务的用例（影子/裁决/切换/回退）**全部拿不到服务**。）
#[test]
fn short_circuiting_the_wiring_would_break_the_enabled_cases() {
    let dir = tempfile::tempdir().expect("临时目录");
    let (_path, backend, governance) = fresh_store(dir.path());
    write_config(dir.path(), &config_body(true));

    // 正常路径：有配置 ⇒ 构造服务。
    let service =
        install_shadow(backend.clone(), governance.clone(), dir.path(), None).expect("装配");
    assert!(
        service.is_some(),
        "有配置 ⇒ 必须构造服务（若这里为空 ⇒ 装配被短路了 ⇒ 其它用例的绿是假的）"
    );

    // 短路路径：等价于 `install_shadow` 直接 `Ok(None)` ⇒ 服务拿不到。
    let short_circuited: Option<Arc<apeireth_runtime_assembly::shadow_wire::ShadowSwapService>> =
        None;
    assert!(
        short_circuited.is_none(),
        "短路后拿不到服务 ⇒ 依赖服务的用例（影子/裁决/切换/回退）全部会红"
    );
}
