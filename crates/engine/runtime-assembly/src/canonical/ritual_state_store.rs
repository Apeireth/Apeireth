//! 仪式状态存储 (`ritual_state.json`) —— SG-22 补半程 (生物机制族 W3.5 批)。
//!
//! organ [`apeireth_organ::ritual`] 是纯内核 (状态由调用方持有, 无 I/O、无存储);
//! 本面补上调用方侧的**状态存储**, 让「仪式候选」在装配面有真实数据源:
//!
//! ```text
//! <care_dir>/ritual_state.json   (StoredDoc 信封: name / version / compatible_versions / body)
//!   body = { config?: RitualConfig, states: [RitualState] }
//!      │
//!      ├─ cli 生物供给 (CareBioSupply::ritual_inputs) → plan_rituals → 候选
//!      └─ cli 显式注入口 (CareHeartbeatSource::record_ritual_done) → 写回
//! ```
//!
//! - **复用既有文件账范式**: 单文档 [`stored_doc::open_single`] /
//!   [`stored_doc::save_single`] (StoredDoc 拒开语义 + 持久档原子写), 与
//!   `care_rules.json` 同姿势; 不新增 crate、不新增外部依赖、不新增开关。
//! - **fail-closed**: 文件畸形 / 串档 / 未来版本 / 重复 kind 一律 `Err` 拒开
//!   (不静默回退空表、不修复、不覆盖); 上层读失败时不产仪式候选并留真因。
//! - **文件缺失 = 合法空态** (`Ok(None)`): 从未有过仪式记录, 不是损坏。
//! - **原子写**: [`stored_doc::save_single`] 走
//!   [`storage_atomic::write_atomic_durable`](apeireth_core::storage_atomic::write_atomic_durable)
//!   (临时文件 + 替换): 读者只会读到旧的完整内容或新的完整内容, 不会读到半写。
//! - **开关不在本层**: 读写由 cli 生物供给在
//!   `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` 时调用; 开关关 = 本面零读取零写入
//!   (存储文件不存在时也不会被创建)。
//! - **显式排程**: `body.config` 存在 = 仪式的显式排程来源; 缺失 = organ
//!   [`RitualConfig::default`] (空排程 = 不主动)。本层不推断默认排程, 不把
//!   「从未完成」自动变成「应该提醒」。
//! - **格式严格**: `config` / `states` 按内核 serde 形状解析, 缺字段 = 拒开
//!   (不做缺省补全); 显式排程需写全 [`RitualConfig`] 字段 (含 `dup_window_ms`
//!   / `default_mute_ms` / `max_catchup`)。
//!
//! # 能力级别 (四级口径)
//!
//! **IMPLEMENTED** (存储面已实现) —— 接线在 cli 装配面 (`CareHeartbeatSource`)
//! 完成; 生产默认关 (`APEIRETH_ENABLE_BIO_CARE_CANDIDATES` 缺省), **不是
//! DEFAULT ENABLED**, 不是 HARDWARE VALIDATED。
//!
//! # 不做 (诚实边界)
//!
//! - **不做跨进程文件锁**: 同进程内用一把 [`Mutex`] 串行化「读 → 改 → 写」防
//!   丢失更新; 两个常驻实例同数据目录并发写不在本批范围 (部署保证单写者)。
//! - **不做静默写回**: 只接「完成 / 跳过」两个显式事件; `muted_until_ms` 的
//!   关系方动作写回 (内核 [`RitualConfig::mute_until`]) 未接, 如实标注。
//! - **不做 schema 迁移**: 未来版本拒开; 迁移必须显式触发 (与 `stored_doc` 同
//!   纪律), 本批不提供迁移入口。
//! - **不建模 streak**: `consecutive_done` / `skipped_count` 只是存储字段, 内核
//!   [`plan_rituals`](apeireth_organ::ritual::plan_rituals) 不读它们; 本层也不
//!   据此加码、不产生亏欠文案。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_organ::ritual::{RitualConfig, RitualKind, RitualState};
use serde::{Deserialize, Serialize};

/// `ritual_state.json` 的文档身份 (StoredDoc 信封 `name` 字段)。
pub const RITUAL_STATE_DOC_NAME: &str = "apeireth.ritual_state";
/// `ritual_state.json` 的格式版本戳 (读端当前版本; 历史/未来版本拒开)。
pub const RITUAL_STATE_DOC_VERSION: u32 = 1;
/// 数据目录里的文件名。
pub const RITUAL_STATE_FILE_NAME: &str = "ritual_state.json";

/// 读端版本契约: 只认当前版本 (历史/未来版本拒开)。
#[must_use]
pub fn ritual_state_doc_compat() -> DocCompat {
    DocCompat::exact(RITUAL_STATE_DOC_NAME, RITUAL_STATE_DOC_VERSION)
}

/// `ritual_state.json` 的主体: 显式排程 (可选) + 每 kind 至多一条状态。
///
/// 状态字段原样使用内核 [`RitualState`] (last_done_ms / consecutive_done /
/// skipped_count / muted_until_ms), 不另立一套投影, 避免两套字段漂移。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RitualStateBody {
    /// 显式排程与全局旋钮; `None` = 取 organ [`RitualConfig::default`]
    /// (空排程 = 不主动, 仪式可以永不做)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<RitualConfig>,
    /// 每 kind 至多一条状态 (重复 kind 拒开/拒写)。
    #[serde(default)]
    pub states: Vec<RitualState>,
}

impl RitualStateBody {
    /// 解析本表体的排程配置: 显式 `config` 优先, 缺失回落 organ 默认 (空排程)。
    #[must_use]
    pub fn resolved_config(&self) -> RitualConfig {
        self.config.clone().unwrap_or_default()
    }
}

/// 仪式状态存储错误 (闭合词表: 拒开 / 拒收)。
#[derive(Debug, thiserror::Error)]
pub enum RitualStateStoreError {
    /// `ritual_state.json` 存在但拒开 (StoredDoc 语义: 不静默修复)。
    #[error("ritual_state.json refused to open: {0}")]
    Stored(#[from] StoredDocError),
    /// 表体被拒收 (重复 kind / 重复排程 / 自相矛盾)。
    #[error("ritual state body refused: {0}")]
    Refused(String),
}

/// 仪式状态存储 (数据目录 `ritual_state.json`, StoredDoc 拒开 + 原子写)。
///
/// `Clone` 共享同一把进程内串行锁 (同数据目录的读写面必须共享同一实例语义;
/// 不同实例间不做跨进程互斥, 见模块注「不做」)。
#[derive(Debug, Clone)]
pub struct RitualStateStore {
    path: PathBuf,
    gate: Arc<Mutex<()>>,
}

impl RitualStateStore {
    /// 绑定数据目录 (`<data_dir>/ritual_state.json`)。
    pub fn under_data_dir(data_dir: impl Into<PathBuf>) -> Self {
        Self::at(data_dir.into().join(RITUAL_STATE_FILE_NAME))
    }

    /// 绑定显式文件路径。
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            gate: Arc::new(Mutex::new(())),
        }
    }

    /// `ritual_state.json` 路径。
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 装载表体 (StoredDoc 拒开语义):
    ///
    /// - 文件缺失 = `Ok(None)` (合法空态: 从未有过仪式记录);
    /// - 存在但坏 (畸形 / 串档 / 未来版本 / 重复 kind) = `Err` (拒开, 不回退)。
    pub fn load(&self) -> Result<Option<RitualStateBody>, RitualStateStoreError> {
        let _guard = self.lock();
        self.load_inner()
    }

    /// 落盘表体 (持久档原子写, 盖当前版本戳; 坏表拒写)。
    ///
    /// **写入不先读旧表**: 显式保存 = 覆盖语义 (运维/测试注入用); 但表体本身
    /// 先校验, 坏表拒绝落盘。事件写回走 [`Self::record_done`] / [`Self::record_skip`]
    /// (读 → 改 → 写, 坏文件拒开即失败, 不覆盖损坏文件)。
    pub fn save(&self, body: &RitualStateBody) -> Result<(), RitualStateStoreError> {
        let _guard = self.lock();
        self.save_inner(body)
    }

    /// 一次装载 → (排程配置, 状态列表): cli 生物供给的只读入口。
    ///
    /// 配置取「显式 `body.config` 优先, 缺失回落 organ 默认」; 状态原样透传
    /// (空 = 无状态, 不编 `fresh`)。坏文件 = `Err`, 由调用方 fail-closed
    /// (不产候选 + 留真因), 不静默回退空表。
    pub fn inputs(&self) -> Result<(RitualConfig, Vec<RitualState>), RitualStateStoreError> {
        let _guard = self.lock();
        let body = self.load_inner()?.unwrap_or_default();
        Ok((body.resolved_config(), body.states))
    }

    /// 记一次**完成** (显式注入口): `last_done_ms` 前移 + `consecutive_done` 计数。
    ///
    /// - 时刻单调: `last_done_ms = max(旧值, now_ms)` —— 时钟回拨不回退完成时刻
    ///   (与内核「未来时间戳防御性降级」同向, 不制造假到点);
    /// - `consecutive_done` 只增不减 (统计字段, 内核判定不读);
    /// - `muted_until_ms` / `skipped_count` 原样保留 (完成不解除静默、不清统计);
    /// - 坏文件 = `Err` 且**不覆盖** (fail-closed: 不让一次写入把损坏账洗白)。
    pub fn record_done(
        &self,
        kind: RitualKind,
        now_ms: u64,
    ) -> Result<RitualState, RitualStateStoreError> {
        let _guard = self.lock();
        let mut body = self.load_inner()?.unwrap_or_default();
        let state = match body.states.iter_mut().find(|state| state.kind == kind) {
            Some(state) => {
                state.last_done_ms = state.last_done_ms.max(now_ms);
                state.consecutive_done = state.consecutive_done.saturating_add(1);
                state.clone()
            }
            None => {
                let state = RitualState {
                    kind,
                    last_done_ms: now_ms,
                    consecutive_done: 1,
                    skipped_count: 0,
                    muted_until_ms: None,
                };
                body.states.push(state.clone());
                state
            }
        };
        self.save_inner(&body)?;
        Ok(state)
    }

    /// 记一次**跳过** (显式注入口): 只加 `skipped_count`, **不动** `last_done_ms`
    /// / 静默 / 连续完成 —— 跳过不是完成, 也不触发任何补课或惩罚 (内核无 streak
    /// 语义, 该计数只作统计)。
    pub fn record_skip(&self, kind: RitualKind) -> Result<RitualState, RitualStateStoreError> {
        let _guard = self.lock();
        let mut body = self.load_inner()?.unwrap_or_default();
        let state = match body.states.iter_mut().find(|state| state.kind == kind) {
            Some(state) => {
                state.skipped_count = state.skipped_count.saturating_add(1);
                state.clone()
            }
            None => {
                let state = RitualState {
                    skipped_count: 1,
                    ..RitualState::fresh(kind)
                };
                body.states.push(state.clone());
                state
            }
        };
        self.save_inner(&body)?;
        Ok(state)
    }

    /// 串行锁 (中毒 = 继续用: 数据面没有半写状态, 不因一次 panic 砖化存储)。
    fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 无锁装载 (调用方已持 [`Self::lock`])。
    fn load_inner(&self) -> Result<Option<RitualStateBody>, RitualStateStoreError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let doc =
            stored_doc::open_single::<RitualStateBody>(&self.path, &ritual_state_doc_compat())?;
        validate_body(&doc.body)?;
        Ok(Some(doc.body))
    }

    /// 无锁落盘 (调用方已持 [`Self::lock`])。
    fn save_inner(&self, body: &RitualStateBody) -> Result<(), RitualStateStoreError> {
        validate_body(body)?;
        stored_doc::save_single(
            &self.path,
            &ritual_state_doc_compat(),
            body.clone(),
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }
}

/// 表体校验 (装载与写出同口径, 坏表拒开也拒写)。
///
/// 规则:
/// - `states` 每 kind 至多一条 (重复 = 表自相矛盾, 拒收);
/// - `config.schedules` 每 kind 至多一条 (内核同 kind 取第一条, 重复是配置
///   错误而非歧义, 拒收);
/// - 其它字段是内核类型 (枚举闭合 / `u64` / `u32`), 反序列化阶段已闭合。
fn validate_body(body: &RitualStateBody) -> Result<(), RitualStateStoreError> {
    let mut seen_kinds: Vec<RitualKind> = Vec::with_capacity(body.states.len());
    for state in &body.states {
        if seen_kinds.contains(&state.kind) {
            return Err(RitualStateStoreError::Refused(format!(
                "duplicate state for ritual kind {:?}",
                state.kind
            )));
        }
        seen_kinds.push(state.kind);
    }
    if let Some(config) = &body.config {
        let mut seen_schedules: Vec<RitualKind> = Vec::with_capacity(config.schedules.len());
        for schedule in &config.schedules {
            if seen_schedules.contains(&schedule.kind) {
                return Err(RitualStateStoreError::Refused(format!(
                    "duplicate schedule for ritual kind {:?}",
                    schedule.kind
                )));
            }
            seen_schedules.push(schedule.kind);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apeireth_core::stored_doc::StoredDoc;
    use apeireth_organ::ritual::{Cadence, RitualSchedule, Window};

    use super::*;

    const NOW: u64 = 1_760_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 24 * HOUR_MS;

    fn schedule(kind: RitualKind) -> RitualSchedule {
        RitualSchedule::new(kind, Cadence::Daily, Window::Flexible, 6 * HOUR_MS)
    }

    fn body_with(kind: RitualKind, last_done_ms: u64) -> RitualStateBody {
        RitualStateBody {
            config: Some(RitualConfig::new(vec![schedule(kind)])),
            states: vec![RitualState {
                last_done_ms,
                ..RitualState::fresh(kind)
            }],
        }
    }

    #[test]
    fn store_roundtrips_states_and_explicit_schedule_in_a_stored_doc_envelope() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        let body = body_with(RitualKind::MorningCheckIn, NOW - DAY_MS);

        store.save(&body).expect("落盘");
        assert_eq!(store.load().expect("装载"), Some(body.clone()));

        // 落盘形态 = StoredDoc 信封 (身份 / 版本 / 自洽), 与 care_rules.json 同姿势。
        let raw = std::fs::read_to_string(store.path()).expect("读文件");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("信封 JSON");
        assert_eq!(value["name"], RITUAL_STATE_DOC_NAME);
        assert_eq!(value["version"], RITUAL_STATE_DOC_VERSION);
        assert_eq!(
            value["compatible_versions"],
            serde_json::json!([RITUAL_STATE_DOC_VERSION])
        );
        assert_eq!(value["body"]["states"][0]["kind"], "morning_check_in");
        assert_eq!(value["body"]["config"]["schedules"][0]["cadence"], "daily");

        // 新实例 (新锁) 读同一文件: 路径即真源, 不靠进程内缓存。
        let reopened = RitualStateStore::at(store.path());
        assert_eq!(reopened.load().expect("再装载"), Some(body));
    }

    #[test]
    fn a_missing_file_is_a_legal_empty_state_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        assert_eq!(store.load().expect("缺失 = 空态"), None);
        let (config, states) = store.inputs().expect("缺失 = 默认输入");
        assert_eq!(config, RitualConfig::default(), "缺省 = organ 默认空排程");
        assert!(states.is_empty(), "不编状态");
        assert!(!store.path().exists(), "只读不创建文件");
    }

    #[test]
    fn a_corrupt_file_fails_closed_and_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        std::fs::write(store.path(), "{ not json").expect("写坏文件");

        assert!(store.load().is_err(), "坏文件必须 Err, 不静默回退空表");
        assert!(store.inputs().is_err(), "供给口拿不到假空表");
        assert!(
            store.record_done(RitualKind::MorningCheckIn, NOW).is_err(),
            "事件写回在坏文件上拒绝 (不把损坏账洗白)"
        );
        assert_eq!(
            std::fs::read_to_string(store.path()).expect("读文件"),
            "{ not json",
            "拒绝写入 = 原文件逐字节不变"
        );
    }

    #[test]
    fn wrong_doc_name_and_future_version_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());

        let wrong_name = serde_json::to_string(&StoredDoc::new(
            "apeireth.other",
            RITUAL_STATE_DOC_VERSION,
            vec![RITUAL_STATE_DOC_VERSION],
            RitualStateBody::default(),
        ))
        .expect("序列化");
        std::fs::write(store.path(), wrong_name).expect("写串档");
        let error = store.load().expect_err("串档拒开");
        assert!(error.to_string().contains("身份不符"), "{error}");

        let future = serde_json::to_string(&StoredDoc::new(
            RITUAL_STATE_DOC_NAME,
            RITUAL_STATE_DOC_VERSION + 1,
            vec![RITUAL_STATE_DOC_VERSION + 1],
            RitualStateBody::default(),
        ))
        .expect("序列化");
        std::fs::write(store.path(), future).expect("写未来版本");
        let error = store.load().expect_err("未来版本拒开");
        assert!(error.to_string().contains("未知未来版本"), "{error}");
    }

    #[test]
    fn duplicate_kinds_are_refused_on_save_and_on_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        let mut body = body_with(RitualKind::EveningWrap, 0);
        body.states
            .push(RitualState::fresh(RitualKind::EveningWrap));
        let error = store.save(&body).expect_err("重复状态拒写");
        assert!(error.to_string().contains("duplicate state"), "{error}");

        let mut duplicate_schedule = body_with(RitualKind::EveningWrap, 0);
        let config = duplicate_schedule.config.as_mut().expect("显式配置");
        config.schedules.push(schedule(RitualKind::EveningWrap));
        let error = store.save(&duplicate_schedule).expect_err("重复排程拒写");
        assert!(error.to_string().contains("duplicate schedule"), "{error}");

        // 磁盘上的重复状态同样拒开 (不是只挡写端)。
        let mut stored = body_with(RitualKind::EveningWrap, 0);
        stored.states.clear();
        stored
            .states
            .push(RitualState::fresh(RitualKind::EveningWrap));
        stored
            .states
            .push(RitualState::fresh(RitualKind::EveningWrap));
        let raw = serde_json::to_string(&StoredDoc::new(
            RITUAL_STATE_DOC_NAME,
            RITUAL_STATE_DOC_VERSION,
            vec![RITUAL_STATE_DOC_VERSION],
            stored,
        ))
        .expect("序列化");
        std::fs::write(store.path(), raw).expect("写重复状态");
        assert!(store.load().is_err(), "重复状态文件拒开");
    }

    #[test]
    fn writes_are_atomic_and_leave_no_temporary_residue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());

        store
            .save(&body_with(RitualKind::MorningCheckIn, NOW - HOUR_MS))
            .expect("首次落盘");
        store
            .save(&body_with(RitualKind::WeeklyReview, NOW - DAY_MS))
            .expect("覆盖落盘");

        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .expect("列目录")
            .map(|entry| entry.expect("目录项").file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![RITUAL_STATE_FILE_NAME.to_string()],
            "原子写不留临时文件残留: {names:?}"
        );

        // 覆盖后整份新内容 (旧 kind 不被拼接残留)。
        let body = store.load().expect("装载").expect("文件存在");
        assert_eq!(body.states.len(), 1);
        assert_eq!(body.states[0].kind, RitualKind::WeeklyReview);
    }

    #[test]
    fn record_done_persists_and_the_next_read_agrees() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        store
            .save(&RitualStateBody {
                config: Some(RitualConfig::new(vec![schedule(
                    RitualKind::MorningCheckIn,
                )])),
                states: Vec::new(),
            })
            .expect("显式排程落盘");

        let state = store
            .record_done(RitualKind::MorningCheckIn, NOW - HOUR_MS)
            .expect("记完成");
        assert_eq!(state.last_done_ms, NOW - HOUR_MS);
        assert_eq!(state.consecutive_done, 1);

        // 新实例读同一文件 = 一致 (不靠内存缓存)。
        let reopened = RitualStateStore::at(store.path());
        let (config, states) = reopened.inputs().expect("装载");
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].last_done_ms, NOW - HOUR_MS);
        assert_eq!(states[0].consecutive_done, 1);
        assert_eq!(
            config,
            RitualConfig::new(vec![schedule(RitualKind::MorningCheckIn)]),
            "事件写回保留显式排程 (配置不被状态覆盖)"
        );

        // 第二次完成: 前移 + 计数 +1。
        reopened
            .record_done(RitualKind::MorningCheckIn, NOW)
            .expect("第二次完成");
        let states = reopened.inputs().expect("装载").1;
        assert_eq!(states[0].last_done_ms, NOW);
        assert_eq!(states[0].consecutive_done, 2);
    }

    #[test]
    fn clock_rollback_never_moves_last_done_backwards() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        store
            .record_done(RitualKind::EveningWrap, NOW)
            .expect("记完成");
        let state = store
            .record_done(RitualKind::EveningWrap, NOW - DAY_MS)
            .expect("回拨时刻的完成事件");
        assert_eq!(
            state.last_done_ms, NOW,
            "完成时刻单调: 回拨不回退 (不制造假到点)"
        );
        assert_eq!(state.consecutive_done, 2);
    }

    #[test]
    fn record_skip_only_counts_and_leaves_the_due_decision_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        store
            .record_done(RitualKind::SharedProject, NOW)
            .expect("完成一次");
        let skipped = store
            .record_skip(RitualKind::SharedProject)
            .expect("跳过一次");
        assert_eq!(skipped.last_done_ms, NOW, "跳过不动完成时刻");
        assert_eq!(skipped.skipped_count, 1);
        assert_eq!(skipped.consecutive_done, 1, "跳过不清连续完成统计");

        let (_, states) = store.inputs().expect("装载");
        assert_eq!(states[0].skipped_count, 1, "跳过计数落盘");
    }

    #[test]
    fn inputs_resolve_explicit_schedule_or_fall_back_to_the_organ_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());

        // 显式排程: 原样解析。
        store
            .save(&body_with(RitualKind::WeeklyReview, 0))
            .expect("落盘");
        let (config, _) = store.inputs().expect("装载");
        assert_eq!(config.schedules.len(), 1);
        assert_eq!(config.schedules[0].kind, RitualKind::WeeklyReview);
        assert_eq!(
            config.dup_window_ms,
            apeireth_organ::ritual::DEFAULT_DUP_WINDOW_MS
        );

        // 只写状态、不写排程: 回落 organ 默认 (空排程) —— 不推断默认节律。
        store
            .save(&RitualStateBody {
                config: None,
                states: vec![RitualState::fresh(RitualKind::WeeklyReview)],
            })
            .expect("只写状态");
        let (config, states) = store.inputs().expect("装载");
        assert_eq!(config, RitualConfig::default());
        assert!(config.schedules.is_empty(), "不擅自启用仪式");
        assert_eq!(states.len(), 1);
    }

    #[test]
    fn the_path_points_at_the_documented_file_name() {
        let store = RitualStateStore::under_data_dir("care-dir");
        assert_eq!(
            store.path(),
            Path::new("care-dir").join(RITUAL_STATE_FILE_NAME)
        );
        assert!(store.path().ends_with("ritual_state.json"));
        assert_eq!(RITUAL_STATE_DOC_NAME, "apeireth.ritual_state");
        assert_eq!(RITUAL_STATE_DOC_VERSION, 1);
    }

    #[test]
    fn cloned_handles_share_the_same_file_and_serialized_writes_do_not_lose_updates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = RitualStateStore::under_data_dir(dir.path());
        let clone = store.clone();
        let threads: Vec<_> = RitualKind::ALL
            .iter()
            .map(|kind| {
                let store = clone.clone();
                let kind = *kind;
                std::thread::spawn(move || {
                    store.record_done(kind, NOW).expect("并发完成");
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("线程收束");
        }
        let mut kinds: Vec<RitualKind> = store
            .inputs()
            .expect("装载")
            .1
            .into_iter()
            .map(|state| state.kind)
            .collect();
        kinds.sort_by_key(|kind| {
            RitualKind::ALL
                .iter()
                .position(|candidate| candidate == kind)
                .expect("闭合词表")
        });
        assert_eq!(kinds, RitualKind::ALL.to_vec(), "同进程串行化: 无丢失更新");
        // 句柄是同一份锁 (路径独立, 锁共享)。
        assert_eq!(store.path(), clone.path());
        assert!(Arc::ptr_eq(&store.gate, &clone.gate));
    }
}
