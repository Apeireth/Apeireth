//! 生物机制族 W3.6 · 生命周期账本**文件持久化**装配测试（SG-18）。
//!
//! 覆盖装配级契约（不是 store 单测的重复）：
//! ① 缺失文件 = 合法空态；绑定 / 只读 / 播种都不创建文件；
//! ② 写读往返 + 重启恢复：同 name 同 phase / 同 use_count / 同墓碑 / 同计数；
//! ③ 坏档 fail-closed：空账 + 诊断真因 + 原档逐字节不动（本进程拒写）；
//! ④ 串档 / 未来版本同样拒开且不覆盖；
//! ⑤ 原子写无 `.tmp-*` / `.bak` 残留；信封身份 / 版本 / 逐资产字段为真值；
//! ⑥ 默认关证据：不传目录 = 纯内存、零文件、零时钟读取、观测与纯内存逐字节一致；
//! ⑦ `evaluate` 是唯一自动落盘点（工具结果 / 播种只记内存），且同输入同字节；
//! ⑧ 容量淘汰与退役墓碑 round-trip（`evicted_total` / `tombstone` 不丢）；
//! ⑨ 落盘失败不打断回合（诊断计数），障碍解除后下一次收口重试成功；
//! ⑩ 装配根接线入口（R1b-1b）：开关关 = 目录一次都不碰；开关开 + 目录 = 绑定落盘。
//!
//! **0 装诚实**：生产装配根已传目录（`bio_production_wiring::install_bio_modules`，
//! R1b-1b 起开关值由 `NervousSwitchboard` 解析后显式传入）；无 live E2E；
//! 跨进程锁未做（单写者）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_orchestration::lifecycle::{LifecyclePolicy, LifecycleStage, RetireReason};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_lifecycle::{
    bio_lifecycle_wiring_from_enabled_with_store_dir, bio_lifecycle_wiring_from_env_value,
    bio_lifecycle_wiring_from_env_value_with_store_dir, BioLifecycleState, LifecycleClock,
    LIFECYCLE_STATE_DOC_NAME, LIFECYCLE_STATE_DOC_VERSION, LIFECYCLE_STATE_FILE_NAME,
};

const NOW_MS: u64 = 1_760_000_000_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1_000;

fn fixed_clock(at_ms: u64) -> LifecycleClock {
    Arc::new(move || at_ms)
}

/// 可计数时钟：证明「关 = 一次时钟都不读」。
fn counting_clock(at_ms: u64) -> (LifecycleClock, Arc<AtomicU64>) {
    let reads = Arc::new(AtomicU64::new(0));
    let reads_for_clock = Arc::clone(&reads);
    let clock: LifecycleClock = Arc::new(move || {
        reads_for_clock.fetch_add(1, Ordering::SeqCst);
        at_ms
    });
    (clock, reads)
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn store_path(dir: &Path) -> PathBuf {
    dir.join(LIFECYCLE_STATE_FILE_NAME)
}

fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("列目录")
        .map(|entry| entry.expect("目录项").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn read_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("读文件")
}

// ============================================
// ① 缺失 = 合法空态；绑定 / 播种 / 记账不创建文件
// ============================================

#[test]
fn a_missing_file_is_a_legal_empty_state_and_binding_creates_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(dir.path());
    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());

    assert!(state.persistence_enabled(), "给了目录 = 绑定文件账");
    assert_eq!(state.persistence_path(), Some(path.as_path()));
    assert_eq!(state.persistence_load_diagnostic(), None, "缺失不是坏档");
    assert!(state.ledger().is_empty(), "缺失 = 合法空账");
    assert!(!path.exists(), "装载 / 绑定不创建文件");

    state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    state
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
        .expect("record use");
    assert!(!path.exists(), "播种 / 工具结果只记内存, 落盘在回合收口");

    state.evaluate_at(NOW_MS + 2);
    assert!(path.exists(), "evaluate 后落盘");
    assert_eq!(state.persistence_save_failures(), 0, "正常落盘无失败计数");
}

// ============================================
// ② 写读往返 + 重启恢复：同 name 同 phase
// ============================================

#[test]
fn a_restart_restores_the_same_asset_names_phases_and_counters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    first.seed_tool_names_at(&names(&["filesystem", "search"]), NOW_MS);
    for at in [NOW_MS + 1, NOW_MS + 2, NOW_MS + 3] {
        first
            .observe_tool_outcome_at("filesystem", true, at)
            .expect("use");
    }
    first
        .observe_tool_outcome_at("search", false, NOW_MS + 1)
        .expect("failure");
    first.evaluate_at(NOW_MS + 4);
    let before = first.observation();
    assert_eq!(before.stage_counts.get("mature"), Some(&1));
    assert_eq!(before.stage_counts.get("newborn"), Some(&1));
    drop(first);

    // 「重启」= 新状态 / 新时钟 / 同一目录，路径即真源。
    let second =
        BioLifecycleState::new(fixed_clock(NOW_MS + DAY_MS)).with_store_from_dir(dir.path());
    assert_eq!(second.persistence_load_diagnostic(), None);
    let ledger = second.ledger();
    let filesystem = ledger.get("filesystem").expect("恢复的资产");
    assert_eq!(filesystem.stage, LifecycleStage::Mature, "同 name 同 phase");
    assert_eq!(filesystem.use_count, 3);
    assert_eq!(filesystem.created_ms, NOW_MS, "创建时刻不被重启改写");
    assert_eq!(filesystem.last_used_ms, NOW_MS + 3);
    let search = ledger.get("search").expect("恢复的资产");
    assert_eq!(search.stage, LifecycleStage::Newborn);
    assert_eq!(search.trial_failures, 1);

    let after = second.observation();
    assert_eq!(after.assets_total, before.assets_total);
    assert_eq!(after.stage_counts, before.stage_counts);
    assert_eq!(
        after.transitions_total, before.transitions_total,
        "累计迁移"
    );
    assert_eq!(after.evicted_total, before.evicted_total);
    assert_eq!(
        after.rejected_after_retire_total,
        before.rejected_after_retire_total
    );
    assert_eq!(after.policy, before.policy);
    assert_eq!(after.capacity, before.capacity);

    // 重启后按真实工具声明名再播种 = 幂等（不覆盖已恢复的历史）。
    assert_eq!(
        second.seed_tool_names_at(&names(&["filesystem", "search"]), NOW_MS + DAY_MS),
        0,
        "重复声明不新增, 已退役墓碑也不会被覆盖"
    );
    assert_eq!(second.ledger().len(), 2);
}

// ============================================
// ③ 坏档 fail-closed：空账 + 诊断真因 + 原档不动
// ============================================

#[test]
fn a_corrupt_file_yields_an_empty_ledger_a_diagnostic_and_refuses_to_overwrite() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = store_path(dir.path());
    let corrupt = b"{ not json".to_vec();
    std::fs::write(&path, &corrupt).expect("写坏文件");

    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    let diagnostic = state
        .persistence_load_diagnostic()
        .expect("坏档必须返回诊断, 不 panic");
    assert!(
        diagnostic.contains("refused to open") && diagnostic.contains("JSON"),
        "诊断必须携带真因: {diagnostic}"
    );
    assert!(state.ledger().is_empty(), "坏档 = 空账");

    // 之后所有真实信号只改内存；落盘路径拒写，原档逐字节不动。
    state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    state
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
        .expect("record use");
    state.evaluate_at(NOW_MS + 2);
    assert_eq!(read_bytes(&path), corrupt, "坏档拒绝被空账 / 新账覆盖");
    assert!(state.persist().is_err(), "显式刷盘同样拒绝覆盖坏档");
    assert!(state.persistence_save_failures() >= 1, "拒写必须计入诊断");
    assert!(
        state
            .persistence_last_save_error()
            .is_some_and(|error| error.contains("refusing to overwrite")),
        "诊断真因: {:?}",
        state.persistence_last_save_error()
    );
    // 内存账仍是真值（坏档不阻断观测面）。
    assert_eq!(state.ledger().len(), 1);
}

// ============================================
// ④ 串档 / 未来版本拒开且不覆盖
// ============================================

#[test]
fn a_wrong_document_name_or_future_version_is_refused_and_preserved() {
    let dir = tempfile::tempdir().expect("tempdir");

    // 先写一份合法账，再篡改信封字段（body 仍合法）——测的是身份 / 版本门。
    let writer = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    writer.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    writer.evaluate_at(NOW_MS + 1);
    drop(writer);
    let path = store_path(dir.path());
    let original: serde_json::Value =
        serde_json::from_slice(&read_bytes(&path)).expect("信封 JSON");
    assert_eq!(original["name"], LIFECYCLE_STATE_DOC_NAME);
    assert_eq!(original["version"], LIFECYCLE_STATE_DOC_VERSION);

    let mut wrong_name = original.clone();
    wrong_name["name"] = serde_json::json!("apeireth.other");
    let bytes = serde_json::to_vec_pretty(&wrong_name).expect("序列化");
    std::fs::write(&path, &bytes).expect("写串档");
    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    let diagnostic = state.persistence_load_diagnostic().expect("串档拒开");
    assert!(diagnostic.contains("身份不符"), "{diagnostic}");
    assert_eq!(read_bytes(&path), bytes, "串档不覆盖");

    let mut future = original;
    future["version"] = serde_json::json!(LIFECYCLE_STATE_DOC_VERSION + 1);
    future["compatible_versions"] = serde_json::json!([LIFECYCLE_STATE_DOC_VERSION + 1]);
    let bytes = serde_json::to_vec_pretty(&future).expect("序列化");
    std::fs::write(&path, &bytes).expect("写未来版本");
    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    let diagnostic = state.persistence_load_diagnostic().expect("未来版本拒开");
    assert!(diagnostic.contains("未知未来版本"), "{diagnostic}");
    assert_eq!(read_bytes(&path), bytes, "未来版本不覆盖");
}

// ============================================
// ⑤ 原子写 + 落盘形态（信封 + 逐资产字段）
// ============================================

#[test]
fn writes_are_atomic_and_leave_no_temporary_residue() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir.path());
    state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    state
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
        .expect("use");
    state.evaluate_at(NOW_MS + 2);
    state
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 3)
        .expect("use");
    state.evaluate_at(NOW_MS + 4);

    let entries = dir_entries(dir.path());
    assert_eq!(
        entries,
        vec![LIFECYCLE_STATE_FILE_NAME.to_string()],
        "原子写不留 .tmp-* / .bak 残留: {entries:?}"
    );

    let value: serde_json::Value =
        serde_json::from_slice(&read_bytes(&store_path(dir.path()))).expect("信封 JSON");
    assert_eq!(value["name"], LIFECYCLE_STATE_DOC_NAME);
    assert_eq!(value["version"], LIFECYCLE_STATE_DOC_VERSION);
    assert_eq!(
        value["compatible_versions"],
        serde_json::json!([LIFECYCLE_STATE_DOC_VERSION])
    );
    assert_eq!(
        value["body"]["records"]["filesystem"]["asset_id"],
        "filesystem"
    );
    assert_eq!(value["body"]["records"]["filesystem"]["kind"], "tool");
    assert_eq!(value["body"]["records"]["filesystem"]["stage"], "trial");
    assert_eq!(
        value["body"]["records"]["filesystem"]["use_count"], 2,
        "逐资产证据字段为内核真值"
    );
    assert_eq!(
        value["body"]["records"]["filesystem"]["tombstone"],
        serde_json::Value::Null
    );
}

// ============================================
// ⑥ 默认关证据：纯内存、零文件、零时钟读取、观测逐字节一致
// ============================================

#[test]
fn the_default_off_path_is_pure_memory_and_byte_identical() {
    let dir = tempfile::tempdir().expect("tempdir");

    // 开关关 + 给了目录：不构造、不读时钟、目录一次都不碰。
    let (clock, reads) = counting_clock(NOW_MS);
    let disabled = bio_lifecycle_wiring_from_env_value_with_store_dir(
        Some("0"),
        clock,
        &names(&["filesystem"]),
        Some(dir.path()),
    );
    assert!(disabled.is_none(), "关 = 不构造");
    assert_eq!(reads.load(Ordering::SeqCst), 0, "关 = 一次时钟都不读");
    assert!(dir_entries(dir.path()).is_empty(), "关 = 零文件");

    // 开关开 + 不传目录：纯内存句柄，零文件。
    let wiring = bio_lifecycle_wiring_from_env_value(
        Some("1"),
        fixed_clock(NOW_MS),
        &names(&["filesystem"]),
    )
    .expect("开关开");
    assert!(!wiring.state().persistence_enabled());
    assert_eq!(wiring.state().persistence_path(), None);
    assert!(wiring.persistence_load_diagnostic().is_none());
    assert_eq!(
        wiring.state().persist(),
        Ok(false),
        "无目录 = 显式刷盘也是零 I/O 的成功 no-op"
    );

    // 同一序列在纯内存状态上重放 = 观测逐字节一致（golden）。
    let plain = BioLifecycleState::new(fixed_clock(NOW_MS));
    plain.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    for state in [wiring.state().as_ref(), &plain] {
        state
            .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
            .expect("use");
        state.evaluate_at(NOW_MS + 2);
    }
    assert_eq!(
        serde_json::to_value(wiring.state().observation()).expect("json"),
        serde_json::to_value(plain.observation()).expect("json"),
        "开关开但不传目录 = 与纯内存逐字节一致"
    );

    // 显式 `None` 目录的新入口同样纯内存。
    let explicit_none = bio_lifecycle_wiring_from_env_value_with_store_dir(
        Some("1"),
        fixed_clock(NOW_MS),
        &names(&["filesystem"]),
        None,
    )
    .expect("开关开");
    assert!(!explicit_none.state().persistence_enabled());
    assert!(dir_entries(dir.path()).is_empty(), "全程零文件");
}

// ============================================
// ⑦ evaluate 是唯一自动落盘点；同输入同字节
// ============================================

#[test]
fn evaluate_is_the_deterministic_persist_point() {
    fn replay(dir: &Path) -> Vec<u8> {
        let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(dir);
        state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
        state
            .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
            .expect("use");
        assert!(
            !store_path(dir).exists(),
            "工具结果 / 播种后、回合收口前不落盘"
        );
        state.evaluate_at(NOW_MS + 2);
        let bytes = read_bytes(&store_path(dir));
        // 同 now 的第二次 evaluate 幂等且无变化 = 不产生新内容（字节仍一致）。
        state.evaluate_at(NOW_MS + 2);
        assert_eq!(
            read_bytes(&store_path(dir)),
            bytes,
            "幂等 evaluate 不改文件"
        );
        bytes
    }

    let first = tempfile::tempdir().expect("tempdir");
    let second = tempfile::tempdir().expect("tempdir");
    let left = replay(first.path());
    let right = replay(second.path());
    assert_eq!(left, right, "同输入序列 → 同字节（确定性落盘）");

    let value: serde_json::Value = serde_json::from_slice(&left).expect("信封 JSON");
    assert_eq!(value["body"]["records"]["filesystem"]["stage"], "trial");
    assert_eq!(value["body"]["counters"]["transitions_total"], 1);
}

// ============================================
// ⑧ 容量淘汰 + 退役墓碑 round-trip
// ============================================

#[test]
fn capacity_eviction_and_tombstones_roundtrip_across_a_restart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = LifecyclePolicy {
        max_trial_failures: 1,
        ..LifecyclePolicy::default()
    };
    let first = BioLifecycleState::with_policy(policy, 2, fixed_clock(NOW_MS))
        .with_store_from_dir(dir.path());
    assert_eq!(
        first.seed_tool_names_at(&names(&["tool.a", "tool.b", "tool.c"]), NOW_MS),
        3
    );
    assert_eq!(first.observation().evicted_total, 1, "容量淘汰非静默");
    assert!(first.ledger().get("tool.a").is_none(), "同层级字典序淘汰");
    for at in [NOW_MS + 1, NOW_MS + 2] {
        first
            .observe_tool_outcome_at("tool.b", false, at)
            .expect("failure");
    }
    let transitions = first.evaluate_at(NOW_MS + 3);
    assert!(
        transitions.iter().any(|transition| matches!(
            transition,
            apeireth_orchestration::lifecycle::LifecycleTransition::Retired {
                reason: RetireReason::Failed,
                ..
            }
        )),
        "失败超限 = 留墓碑: {transitions:?}"
    );
    let before = first.observation();
    drop(first);

    let second =
        BioLifecycleState::new(fixed_clock(NOW_MS + DAY_MS)).with_store_from_dir(dir.path());
    assert_eq!(second.persistence_load_diagnostic(), None);
    let ledger = second.ledger();
    let retired = ledger.get("tool.b").expect("墓碑记录");
    assert_eq!(retired.stage, LifecycleStage::Retired);
    let tombstone = retired.tombstone.expect("退役必带墓碑");
    assert_eq!(tombstone.reason, RetireReason::Failed);
    assert_eq!(tombstone.retired_ms, NOW_MS + 3);
    assert!(tombstone.recoverable);
    assert_eq!(retired.trial_failures, 2);
    assert_eq!(ledger.capacity(), 2, "容量随账本恢复");
    assert_eq!(ledger.policy().max_trial_failures, 1, "策略随账本恢复");
    assert_eq!(second.observation().evicted_total, before.evicted_total);
    assert_eq!(second.observation().retired_total, 1);
}

// ============================================
// ⑨ 落盘失败不打断回合；障碍解除后重试成功
// ============================================

#[test]
fn a_write_failure_is_diagnosed_and_retried_without_breaking_the_ledger() {
    let base = tempfile::tempdir().expect("tempdir");
    let blocker = base.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("写障碍文件");
    let care_dir = blocker.join("care");

    let state = BioLifecycleState::new(fixed_clock(NOW_MS)).with_store_from_dir(&care_dir);
    assert_eq!(
        state.persistence_load_diagnostic(),
        None,
        "父路径不可写不是装载错误（文件缺失 = 空态）"
    );
    state.seed_tool_names_at(&names(&["filesystem"]), NOW_MS);
    state
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
        .expect("use");
    let transitions = state.evaluate_at(NOW_MS + 2);
    assert_eq!(transitions.len(), 1, "真实回合收口产生一次晋升");
    assert_eq!(
        state.persistence_save_failures(),
        1,
        "落盘失败必须计数, 不 panic"
    );
    assert!(state.persistence_last_save_error().is_some());
    assert_eq!(state.ledger().len(), 1, "落盘失败不打断内存账 / 回合");

    // 障碍解除 → 下一次回合收口重试（dirty 未清）成功；失败历史保留。
    std::fs::remove_file(&blocker).expect("移除障碍");
    std::fs::create_dir_all(&care_dir).expect("建目录");
    state.evaluate_at(NOW_MS + 3);
    assert!(store_path(&care_dir).exists(), "重试成功落盘");
    assert_eq!(state.persistence_save_failures(), 1, "成功不清零失败历史");
}

// ============================================
// ⑩ 装配根接线入口（R1b-1b：开关值由装配根解析后显式传入）：
//    开关关不碰目录；开关开 + 目录 = 绑定落盘
// ============================================

#[test]
fn the_enabled_flag_binds_a_store_dir_only_when_on() {
    let disabled_dir = tempfile::tempdir().expect("tempdir");
    let (clock, reads) = counting_clock(NOW_MS);
    let wiring = bio_lifecycle_wiring_from_enabled_with_store_dir(
        false,
        clock,
        &names(&["filesystem"]),
        Some(disabled_dir.path()),
    );
    assert!(wiring.is_none(), "开关关 = 不构造");
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert!(
        dir_entries(disabled_dir.path()).is_empty(),
        "开关关 = 目录一次都不碰"
    );

    let enabled_dir = tempfile::tempdir().expect("tempdir");
    let wiring = bio_lifecycle_wiring_from_enabled_with_store_dir(
        true,
        fixed_clock(NOW_MS),
        &names(&["filesystem"]),
        Some(enabled_dir.path()),
    )
    .expect("开关开");
    assert!(wiring.state().persistence_enabled());
    assert_eq!(
        wiring.state().persistence_path(),
        Some(store_path(enabled_dir.path()).as_path())
    );
    assert_eq!(wiring.persistence_load_diagnostic(), None);
    wiring
        .state()
        .observe_tool_outcome_at("filesystem", true, NOW_MS + 1)
        .expect("use");
    wiring.state().evaluate_at(NOW_MS + 2);
    assert!(store_path(enabled_dir.path()).exists(), "开 + 目录 = 落盘");
}
