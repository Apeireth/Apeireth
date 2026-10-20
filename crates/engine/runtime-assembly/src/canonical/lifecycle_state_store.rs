//! 生命周期账本的文件存储 (`lifecycle_state.json`) —— 生物机制族 W3.6 收尾（SG-18）。
//!
//! 内核 [`apeireth_orchestration::lifecycle`] 是**纯内存**账本（0 I/O、注入时钟）；
//! 本面给装配面 [`BioLifecycleState`](super::BioLifecycleState) 提供**可选**文件账，
//! 让「重启不丢账」在显式提供目录时成立：
//!
//! ```text
//! <care_dir>/lifecycle_state.json   (StoredDoc 信封: name / version / compatible_versions / body)
//!   body = {
//!     policy:  LifecyclePolicy,       // 阈值快照（重启后同阈值继续判定）
//!     capacity: 64,                   // 容量上限（含墓碑）
//!     counters: { transitions_total, evicted_total, rejected_after_retire_total },
//!     records: { "<asset_id>": LifecycleRecord }   // 内核记录原样: asset_id / kind / stage /
//!   }                                              // created_ms / last_used_ms / use_count /
//! }                                                // trial_failures / tombstone
//! ```
//!
//! - **复用既有文件账范式**：[`stored_doc::open_single`] / [`stored_doc::save_single`]
//!   （拒开语义 + 持久档原子写），与 `ritual_state.json` 同姿势；不新增 crate、
//!   不新增外部依赖、不新增开关。
//! - **缺失 = 合法空态**（`load` 返回 `Ok(None)`）：从未有过账本，不是损坏。
//! - **坏档拒开且不覆盖**：畸形 JSON / 串档 / 未来版本 / 表体自相矛盾一律 `Err`；
//!   上层 [`BioLifecycleState`](super::BioLifecycleState) 据此保持空账并**拒绝写入**
//!   （本层 `save` 是显式覆盖语义，坏档保护由绑定方在装载失败后拒写实现）。
//! - **原子写**：临时文件 + 替换（读者只看到旧完整内容或新完整内容）；正常完成
//!   不留 `.tmp-*` 残留。
//! - **写入时机由调用方决定**（见 `BioLifecycleState`）：每回合收口 `evaluate`
//!   后、且自上次成功落盘以来账本有变化时写一次；`persist()` 可显式提前刷盘。
//!   本层没有后台线程、没有 debounce、不读系统时钟。
//!
//! # 能力级别（四级口径）
//!
//! **IMPLEMENTED**（存储面已实现并测试）；接线入口 =
//! [`bio_lifecycle_wiring_from_env_value_with_store_dir`](super::bio_lifecycle_wiring_from_env_value_with_store_dir)，
//! 生产装配根接线由 Lead 完成（`bio_production_wiring.rs` 本批禁改）。默认关
//! （不传目录 = 纯内存）—— **不是 DEFAULT ENABLED**，无 live E2E。
//!
//! # 不做（诚实边界）
//!
//! - **不做跨进程文件锁**：同进程内一把 [`Mutex`] 串行化读写；两个常驻实例同目录
//!   并发写不在本批范围（部署保证单写者）。
//! - **不做 schema 迁移**：只认 v1；未来版本拒开，迁移必须显式触发（与
//!   `stored_doc` 同纪律），本批不提供迁移入口。
//! - **不持久化进程内瞬时观测量**：`last_transitions` / `last_action` 是「最近一次
//!   回合」的瞬时面，重启后从空开始；三个累计计数随账本持久化（单调延续）。
//! - **不做 WAL / 逐事件 fsync**：持久化边界 = 回合收口（`evaluate`）；回合进行中
//!   的崩溃会丢该回合尚未收口的记账（内存真值不受影响）。
//! - **不做损坏修复**：坏档只诊断不覆盖；调用方修好文件（或换目录）后重启。
//! - **不创建/托管目录生命周期**：目录由调用方（装配根 care 目录）准备。

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_orchestration::lifecycle::{
    LifecycleLedger, LifecyclePolicy, LifecycleRecord, LifecycleStage,
};
use serde::{Deserialize, Serialize};

/// `lifecycle_state.json` 的文档身份（StoredDoc 信封 `name` 字段）。
pub const LIFECYCLE_STATE_DOC_NAME: &str = "apeireth.lifecycle_state";
/// 格式版本戳（读端当前版本；历史 / 未来版本拒开）。
pub const LIFECYCLE_STATE_DOC_VERSION: u32 = 1;
/// care 数据目录里的文件名。
pub const LIFECYCLE_STATE_FILE_NAME: &str = "lifecycle_state.json";

/// 读端版本契约：只认当前版本（历史 / 未来版本拒开）。
#[must_use]
pub fn lifecycle_state_doc_compat() -> DocCompat {
    DocCompat::exact(LIFECYCLE_STATE_DOC_NAME, LIFECYCLE_STATE_DOC_VERSION)
}

/// 账本累计计数快照（进程内观测量的**可持久化**子集）。
///
/// 这三个计数只增不减，且无法从记录表反推（被容量淘汰的记录已不在表内），
/// 因此随账本落盘，重启后单调延续。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleCounters {
    /// 累计迁移数（`evaluate` 返回值之和）。
    pub transitions_total: u64,
    /// 累计容量淘汰数（内核交还的记录，不静默丢）。
    pub evicted_total: u64,
    /// 累计「已退役资产再次被真实使用」次数（拒绝静默复活）。
    pub rejected_after_retire_total: u64,
}

/// `lifecycle_state.json` 的主体：内核账本快照 + 累计计数。
///
/// `records` 的键 = 记录 `asset_id`（写入 / 装载两侧都校验一致）；值 = 内核
/// [`LifecycleRecord`] 原样（不另立投影，避免两套字段漂移）。`policy` /
/// `capacity` 与内核账本同源，重启后继续同一策略判定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleStateBody {
    /// 生效策略快照（内核 [`LifecyclePolicy`]）。
    pub policy: LifecyclePolicy,
    /// 容量上限（含墓碑；必须 ≥ 1）。
    pub capacity: usize,
    /// 累计计数快照。
    pub counters: LifecycleCounters,
    /// 按 `asset_id` 排序的记录表（`BTreeMap` 顺序确定，同输入同字节）。
    pub records: BTreeMap<String, LifecycleRecord>,
}

impl LifecycleStateBody {
    /// 从内核账本导出表体（`active` + `retired` 覆盖全部阶段，不触碰私有字段）。
    #[must_use]
    pub fn from_ledger(ledger: &LifecycleLedger, counters: LifecycleCounters) -> Self {
        let records = ledger
            .active()
            .into_iter()
            .chain(ledger.retired())
            .map(|record| (record.asset_id.clone(), record.clone()))
            .collect();
        Self {
            policy: ledger.policy(),
            capacity: ledger.capacity(),
            counters,
            records,
        }
    }

    /// 还原内核账本（经内核 `LifecycleLedger` 的 serde 快照形状：`records` /
    /// `policy` / `capacity`；`counters` 是附加字段，内核反序列化时忽略）。
    ///
    /// 本方法是**装载侧**对内核私有字段的唯一耦合点：内核 serde 形状若漂移，
    /// 这里返回 `Err`（fail-closed），不会静默读出半本账。
    pub fn ledger(&self) -> Result<LifecycleLedger, LifecycleStateStoreError> {
        let snapshot = LedgerSnapshot {
            records: self.records.clone(),
            policy: self.policy,
            capacity: self.capacity,
        };
        let value = serde_json::to_value(snapshot).map_err(|error| {
            LifecycleStateStoreError::Refused(format!("ledger snapshot serialize failed: {error}"))
        })?;
        serde_json::from_value(value).map_err(|error| {
            LifecycleStateStoreError::Refused(format!("kernel ledger snapshot refused: {error}"))
        })
    }
}

/// 内核账本 serde 快照形状（与 `LifecycleLedger` 的派生序列化字段同名）。
///
/// 内核把该类型的序列化声明为「快照 / 回放」用途；本结构只用于在装配面
/// 读取 / 重建账本，不改内核、不反射、不 unsafe。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerSnapshot {
    records: BTreeMap<String, LifecycleRecord>,
    policy: LifecyclePolicy,
    capacity: usize,
}

/// 生命周期文件账错误（闭合词表：拒开 / 拒收）。
#[derive(Debug, thiserror::Error)]
pub enum LifecycleStateStoreError {
    /// `lifecycle_state.json` 存在但拒开（StoredDoc 语义：不静默修复）。
    #[error("lifecycle_state.json refused to open: {0}")]
    Stored(#[from] StoredDocError),
    /// 表体被拒收（自相矛盾 / 超出容量 / 键与记录不一致 / 内核形状不符）。
    #[error("lifecycle state body refused: {0}")]
    Refused(String),
}

/// 生命周期文件账（`<care_dir>/lifecycle_state.json`，拒开 + 原子写）。
///
/// `Clone` 共享同一把进程内串行锁（同一文件的多句柄必须共享同一实例语义）；
/// 不同进程之间不做互斥（见模块注「不做」）。
#[derive(Debug, Clone)]
pub struct LifecycleStateStore {
    path: PathBuf,
    gate: Arc<Mutex<()>>,
}

impl LifecycleStateStore {
    /// 绑定 care 数据目录（`<care_dir>/lifecycle_state.json`）。
    pub fn under_care_dir(care_dir: impl Into<PathBuf>) -> Self {
        Self::at(care_dir.into().join(LIFECYCLE_STATE_FILE_NAME))
    }

    /// 绑定显式文件路径（测试 / 回放）。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            gate: Arc::new(Mutex::new(())),
        }
    }

    /// `lifecycle_state.json` 路径。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 装载表体（StoredDoc 拒开语义）：
    ///
    /// - 文件缺失 = `Ok(None)`（合法空态：从未有过账本）；
    /// - 存在但坏（畸形 / 串档 / 未来版本 / 表体自相矛盾 / 内核形状不符）
    ///   = `Err`（拒开，不回退空表、不修复）。
    pub fn load(&self) -> Result<Option<LifecycleStateBody>, LifecycleStateStoreError> {
        let _guard = self.lock();
        self.load_inner()
    }

    /// 落盘表体（持久档原子写，盖当前版本戳；坏表拒写）。
    ///
    /// **写入不先读旧表**：显式保存 = 覆盖语义（测试 / 运维注入用）；表体先校验，
    /// 坏表拒绝落盘。事件写回走 [`BioLifecycleState`](super::BioLifecycleState) 的
    /// 「启动装载 → 内存改 → 整本落盘」路径；装载失败的实例**拒写**，因此不会覆盖
    /// 损坏文件。
    pub fn save(&self, body: &LifecycleStateBody) -> Result<(), LifecycleStateStoreError> {
        let _guard = self.lock();
        validate_body(body)?;
        stored_doc::save_single(
            &self.path,
            &lifecycle_state_doc_compat(),
            body.clone(),
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }

    /// 串行锁（中毒 = 继续用：数据面没有半写状态，不因一次 panic 砖化存储）。
    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 无锁装载（调用方已持 [`Self::lock`]）。
    fn load_inner(&self) -> Result<Option<LifecycleStateBody>, LifecycleStateStoreError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<LifecycleStateBody>(
            &self.path,
            &lifecycle_state_doc_compat(),
        )?;
        validate_body(&doc.body)?;
        // 内核形状自检：表体必须能还原为账本，防止内核 serde 形状漂移被静默接受。
        doc.body.ledger()?;
        Ok(Some(doc.body))
    }
}

/// 表体校验（装载与写出同口径，坏表拒开也拒写）。
///
/// 规则：
/// - `capacity >= 1` 且记录条数不超过容量（内核账本不变量：容量含墓碑）；
/// - 记录键 = 记录 `asset_id`（键与值不一致 = 表自相矛盾）；
/// - `asset_id` 非空白（内核 `register` 的同一约束）；
/// - `stage == Retired` ⇔ `tombstone.is_some()`（内核不变量：退役必留墓碑，
///   墓碑只属于退役记录）。
///
/// `policy` 沿用内核 serde 语义（`#[serde(default)]`：缺省阈值 = 内核默认），
/// 其余字段严格（缺字段 = 反序列化阶段即拒开）。
fn validate_body(body: &LifecycleStateBody) -> Result<(), LifecycleStateStoreError> {
    if body.capacity == 0 {
        return Err(LifecycleStateStoreError::Refused(
            "capacity must be at least 1".to_string(),
        ));
    }
    if body.records.len() > body.capacity {
        return Err(LifecycleStateStoreError::Refused(format!(
            "records ({}) exceed capacity ({})",
            body.records.len(),
            body.capacity
        )));
    }
    for (key, record) in &body.records {
        if record.asset_id != *key {
            return Err(LifecycleStateStoreError::Refused(format!(
                "record key {key:?} does not match asset_id {:?}",
                record.asset_id
            )));
        }
        if record.asset_id.trim().is_empty() {
            return Err(LifecycleStateStoreError::Refused(
                "asset_id must not be blank".to_string(),
            ));
        }
        let retired = record.stage == LifecycleStage::Retired;
        match (retired, record.tombstone.is_some()) {
            (true, false) => {
                return Err(LifecycleStateStoreError::Refused(format!(
                    "retired asset {:?} has no tombstone",
                    record.asset_id
                )));
            }
            (false, true) => {
                return Err(LifecycleStateStoreError::Refused(format!(
                    "live asset {:?} carries a tombstone",
                    record.asset_id
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use apeireth_core::stored_doc::StoredDoc;
    use apeireth_orchestration::lifecycle::{AssetKind, RetireReason, Tombstone};

    use super::*;

    const NOW: u64 = 1_760_000_000_000;

    fn sample_ledger() -> LifecycleLedger {
        let policy = LifecyclePolicy {
            max_trial_failures: 1,
            ..LifecyclePolicy::default()
        };
        let mut ledger = LifecycleLedger::new(policy, 4);
        ledger.register("tool.a", AssetKind::Tool, NOW).expect("a");
        ledger.record_use("tool.a", NOW + 1).expect("use a");
        ledger.record_use("tool.a", NOW + 2).expect("use a");
        ledger.evaluate(NOW + 3);
        ledger.register("tool.b", AssetKind::Tool, NOW).expect("b");
        ledger.record_failure("tool.b", NOW + 1).expect("fail b");
        ledger.record_failure("tool.b", NOW + 2).expect("fail b");
        ledger.evaluate(NOW + 3);
        ledger
    }

    fn sample_counters() -> LifecycleCounters {
        LifecycleCounters {
            transitions_total: 3,
            evicted_total: 7,
            rejected_after_retire_total: 2,
        }
    }

    fn sample_body() -> LifecycleStateBody {
        LifecycleStateBody::from_ledger(&sample_ledger(), sample_counters())
    }

    #[test]
    fn the_path_and_doc_identity_are_the_documented_ones() {
        let store = LifecycleStateStore::under_care_dir("care-dir");
        assert_eq!(
            store.path(),
            Path::new("care-dir").join(LIFECYCLE_STATE_FILE_NAME)
        );
        assert!(store.path().ends_with("lifecycle_state.json"));
        assert_eq!(LIFECYCLE_STATE_DOC_NAME, "apeireth.lifecycle_state");
        assert_eq!(LIFECYCLE_STATE_DOC_VERSION, 1);
        let compat = lifecycle_state_doc_compat();
        assert_eq!(compat.name, LIFECYCLE_STATE_DOC_NAME);
        assert_eq!(compat.current_version, LIFECYCLE_STATE_DOC_VERSION);
    }

    #[test]
    fn a_missing_file_is_a_legal_empty_state_and_a_read_creates_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        assert_eq!(store.load().expect("缺失 = 空态"), None);
        assert!(!store.path().exists(), "只读不创建文件");
    }

    #[test]
    fn save_then_load_roundtrips_the_ledger_counters_and_envelope() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        let body = sample_body();

        store.save(&body).expect("落盘");
        let loaded = store.load().expect("装载").expect("文件存在");
        assert_eq!(loaded, body, "表体逐字段往返");
        assert_eq!(
            loaded.ledger().expect("还原账本"),
            sample_ledger(),
            "内核账本无损往返（阶段 / 计数 / 墓碑 / 策略 / 容量）"
        );

        // 落盘形态 = StoredDoc 信封（身份 / 版本 / 自洽），body 为逐资产字段。
        let raw = std::fs::read_to_string(store.path()).expect("读文件");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("信封 JSON");
        assert_eq!(value["name"], LIFECYCLE_STATE_DOC_NAME);
        assert_eq!(value["version"], LIFECYCLE_STATE_DOC_VERSION);
        assert_eq!(
            value["compatible_versions"],
            serde_json::json!([LIFECYCLE_STATE_DOC_VERSION])
        );
        assert_eq!(value["body"]["capacity"], 4);
        assert_eq!(value["body"]["counters"]["evicted_total"], 7);
        assert_eq!(value["body"]["records"]["tool.a"]["stage"], "trial");
        assert_eq!(value["body"]["records"]["tool.a"]["asset_id"], "tool.a");
        assert_eq!(
            value["body"]["records"]["tool.b"]["tombstone"]["reason"],
            "failed"
        );

        // 新实例（新锁）读同一文件：路径即真源，不靠进程内缓存。
        let reopened = LifecycleStateStore::at(store.path());
        assert_eq!(reopened.load().expect("再装载"), Some(body));
    }

    #[test]
    fn a_corrupt_file_fails_closed_and_is_never_overwritten_by_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        std::fs::write(store.path(), "{ not json").expect("写坏文件");

        assert!(store.load().is_err(), "坏文件必须 Err, 不静默回退空表");
        assert_eq!(
            std::fs::read_to_string(store.path()).expect("读文件"),
            "{ not json",
            "装载失败 = 原文件逐字节不变"
        );
    }

    #[test]
    fn wrong_doc_name_and_future_version_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        let body = sample_body();

        let wrong_name = serde_json::to_string(&StoredDoc::new(
            "apeireth.other",
            LIFECYCLE_STATE_DOC_VERSION,
            vec![LIFECYCLE_STATE_DOC_VERSION],
            body.clone(),
        ))
        .expect("序列化");
        std::fs::write(store.path(), wrong_name).expect("写串档");
        let error = store.load().expect_err("串档拒开");
        assert!(error.to_string().contains("身份不符"), "{error}");

        let future = serde_json::to_string(&StoredDoc::new(
            LIFECYCLE_STATE_DOC_NAME,
            LIFECYCLE_STATE_DOC_VERSION + 1,
            vec![LIFECYCLE_STATE_DOC_VERSION + 1],
            body,
        ))
        .expect("序列化");
        std::fs::write(store.path(), future).expect("写未来版本");
        let error = store.load().expect_err("未来版本拒开");
        assert!(error.to_string().contains("未知未来版本"), "{error}");
    }

    #[test]
    fn contradictory_bodies_are_refused_on_save_and_on_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());

        // 键与记录 asset_id 不一致。
        let mut mismatched = sample_body();
        let record = mismatched.records.remove("tool.a").expect("sample record");
        mismatched.records.insert("tool.alias".to_string(), record);
        let error = store.save(&mismatched).expect_err("键不一致拒写");
        assert!(error.to_string().contains("does not match"), "{error}");

        // 容量 0 / 记录超额。
        let mut zero_capacity = sample_body();
        zero_capacity.capacity = 0;
        assert!(store.save(&zero_capacity).is_err(), "容量 0 拒写");
        let mut over_capacity = sample_body();
        over_capacity.capacity = 1;
        assert!(store.save(&over_capacity).is_err(), "记录超容量拒写");

        // 退役无墓碑 / 活记录带墓碑。
        let mut missing_tombstone = sample_body();
        missing_tombstone
            .records
            .get_mut("tool.b")
            .expect("retired record")
            .tombstone = None;
        assert!(store.save(&missing_tombstone).is_err(), "退役无墓碑拒写");
        let mut stray_tombstone = sample_body();
        stray_tombstone
            .records
            .get_mut("tool.a")
            .expect("live record")
            .tombstone = Some(Tombstone {
            retired_ms: NOW,
            reason: RetireReason::Manual,
            recoverable: true,
        });
        assert!(store.save(&stray_tombstone).is_err(), "活记录带墓碑拒写");
        assert!(!store.path().exists(), "坏表拒写 = 不创建文件");

        // 磁盘上的矛盾表体同样拒开（不是只挡写端）。
        let mut stored = sample_body();
        stored.records.get_mut("tool.b").expect("retired").tombstone = None;
        let raw = serde_json::to_string(&StoredDoc::new(
            LIFECYCLE_STATE_DOC_NAME,
            LIFECYCLE_STATE_DOC_VERSION,
            vec![LIFECYCLE_STATE_DOC_VERSION],
            stored,
        ))
        .expect("序列化");
        std::fs::write(store.path(), raw).expect("写矛盾表");
        assert!(store.load().is_err(), "矛盾表体文件拒开");
    }

    #[test]
    fn writes_are_atomic_and_leave_no_temporary_residue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());

        store.save(&sample_body()).expect("首次落盘");
        let mut updated = sample_body();
        updated.counters.transitions_total = 99;
        store.save(&updated).expect("覆盖落盘");

        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .expect("列目录")
            .map(|entry| entry.expect("目录项").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![LIFECYCLE_STATE_FILE_NAME.to_string()],
            "原子写不留临时文件残留: {names:?}"
        );
        assert_eq!(
            store.load().expect("装载").expect("文件存在").counters,
            updated.counters,
            "覆盖后整份新内容"
        );
    }

    #[test]
    fn cloned_handles_share_the_same_file_and_serialization_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        let clone = store.clone();
        assert_eq!(store.path(), clone.path());
        assert!(Arc::ptr_eq(&store.gate, &clone.gate));

        store.save(&sample_body()).expect("句柄写入");
        assert_eq!(clone.load().expect("克隆句柄装载"), Some(sample_body()));
    }

    #[test]
    fn an_empty_records_table_is_a_valid_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LifecycleStateStore::under_care_dir(dir.path());
        let body = LifecycleStateBody {
            policy: LifecyclePolicy::default(),
            capacity: 8,
            counters: LifecycleCounters::default(),
            records: BTreeMap::new(),
        };
        store.save(&body).expect("空表落盘");
        let loaded = store.load().expect("装载").expect("文件存在");
        assert!(loaded.records.is_empty());
        assert!(loaded.ledger().expect("还原空账").is_empty());
    }
}
