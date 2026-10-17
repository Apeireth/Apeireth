//! 生产自述来源 (自省通道) —— [`SelfStatusSource`] 的生产实现。
//!
//! 装配时把**真实生效值** (能力开关实际取值 / 工作区 / 自学习接线) 与两个
//! data 系探测口 (记忆账本计数 / 凭据存在性) 收进一个只读来源; `self_status`
//! 工具每次调用现采集快照。data 系失败**不**让整帧失败: 对应字段显式
//! `null` + 原因 (0 装诚实: 明说没测到, 不编造)。
//!
//! 预算一节读回合轮数/单轮工具上限: 组装根把**生效中的**可配置预算旋钮
//! 取值注入时按 `configured` 口径报告 (与运行时装配同一条解析路径); 未注入
//! 的组合回退编译期常量并注明 `constant` 口径 (见 [`BudgetStatus::source`])。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_core::model_profile::ModelProfile;
use apeireth_orchestration::self_tuning::TunableParam;
use apeireth_tools_canonical::{
    derive_tuning_preset, BackupStatus, BudgetStatus, CapabilitySwitch, CareStatus,
    ContentFeedStatus, DomainCounts, DomainStatus, MemoryLedgerStats, QuotaDimensionStatus,
    QuotaMeterReading, QuotaStatus, SelfStatusIdentity, SelfStatusSnapshot, SelfStatusSource,
    SessionQuotaProbe, StatusProbe, TaskBoardStatus, TraitStatus, TuningStatus, VoiceStatus,
    WorkspaceStatus, BUDGET_SOURCE_CONSTANT_NOTE, DATA_PROBE_NOT_WIRED, QUOTA_DEPTH_NOT_WIRED,
    QUOTA_METER_SCOPE_REQUIRED,
};

use super::production::ProductionModulesConfig;
use super::session::Session;
use super::{execute::MAX_TOOL_CALLS_PER_ROUND, runtime::DEFAULT_MAX_ROUNDS};

/// 预算取值口径: 读编译期常量 (可配置口径未接线的回退)。
pub const BUDGET_SOURCE_CONSTANT: &str = "constant";
/// 预算取值口径: 读**生效中的**可配置预算旋钮 (与运行时装配同一条解析路径)。
pub const BUDGET_SOURCE_CONFIGURED: &str = "configured";

/// 口径注记: 预算经可配置旋钮解析 (未设 env 时取默认值)。
pub const BUDGET_SOURCE_CONFIGURED_NOTE: &str =
    "read through the live budget knobs (env-configurable; defaults apply when unset)";

/// 单回合轮数上限的编译期常量。
pub fn max_rounds_per_turn_constant() -> u64 {
    u64::from(DEFAULT_MAX_ROUNDS)
}

/// 单轮工具调用上限的编译期常量。
pub fn max_tool_calls_per_round_constant() -> u64 {
    MAX_TOOL_CALLS_PER_ROUND as u64
}

/// 从编译期常量构造预算一节 (含口径注记)。
///
/// 配额三维此口径下无旋钮 = 不设限 (上限真值 `None`), 已耗一列无探测口 =
/// 显式 `null` + 原因; 深度维如实"暂无接口"。
pub fn budget_status_from_constants() -> BudgetStatus {
    BudgetStatus {
        max_rounds_per_turn: max_rounds_per_turn_constant(),
        max_tool_calls_per_round: max_tool_calls_per_round_constant(),
        quota: QuotaStatus {
            tokens: QuotaDimensionStatus::without_reading(None, DATA_PROBE_NOT_WIRED),
            tool_steps: QuotaDimensionStatus::without_reading(None, DATA_PROBE_NOT_WIRED),
            cost_micros: QuotaDimensionStatus::without_reading(None, DATA_PROBE_NOT_WIRED),
            depth: QuotaDimensionStatus::not_wired(QUOTA_DEPTH_NOT_WIRED),
        },
        source: BUDGET_SOURCE_CONSTANT.to_string(),
        note: BUDGET_SOURCE_CONSTANT_NOTE.to_string(),
    }
}

/// 从**生效中的**可配置预算旋钮构造预算一节 (组装根把实际装配进运行时的
/// 取值原样传入, 自述与运行时同源)。
///
/// `quota_max_tokens` / `quota_max_tool_steps` / `quota_max_cost_micros` 是
/// 会话配额三维旋钮的生效上限 (`None` = 不设限, 显示 "—"); 已耗一列在自述
/// 采集时按会话作用域现探测 (见
/// [`ProductionSelfStatusSource::snapshot_in_session`]), 花费维含无价轮时另
/// 如实标"部分计量"。
pub fn budget_status_from_configured(
    max_rounds_per_turn: u64,
    max_tool_calls_per_round: u64,
    quota_max_tokens: Option<u64>,
    quota_max_tool_steps: Option<u64>,
    quota_max_cost_micros: Option<u64>,
) -> BudgetStatus {
    BudgetStatus {
        max_rounds_per_turn,
        max_tool_calls_per_round,
        quota: QuotaStatus {
            tokens: QuotaDimensionStatus::without_reading(quota_max_tokens, DATA_PROBE_NOT_WIRED),
            tool_steps: QuotaDimensionStatus::without_reading(
                quota_max_tool_steps,
                DATA_PROBE_NOT_WIRED,
            ),
            cost_micros: QuotaDimensionStatus::without_reading(
                quota_max_cost_micros,
                DATA_PROBE_NOT_WIRED,
            ),
            depth: QuotaDimensionStatus::not_wired(QUOTA_DEPTH_NOT_WIRED),
        },
        source: BUDGET_SOURCE_CONFIGURED.to_string(),
        note: BUDGET_SOURCE_CONFIGURED_NOTE.to_string(),
    }
}

/// 自省通道: 会话配额计量探测口 —— 同步读会话存储里该会话的配额账本
/// (只回数字, 不回内容)。
///
/// 生产会话行就是 [`super::sqlite_session::SqliteSessionStore`] 落的 JSON 行;
/// 计量点即落库点 (见 runtime 执行面的 `meter_*`), 读数因此精确到最近一笔
/// 记账。未落库的会话回 `Err(原因)`, 自述字段显式 `null` + 原因。
pub fn session_quota_probe(db_path: PathBuf) -> SessionQuotaProbe {
    Arc::new(move |session: SessionId| {
        use rusqlite::OptionalExtension;
        let connection = rusqlite::Connection::open(&db_path)
            .map_err(|error| format!("session store open failed: {error}"))?;
        let data: Option<String> = connection
            .query_row(
                "SELECT data FROM sessions WHERE id = ?1",
                rusqlite::params![session.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("session row read failed: {error}"))?;
        let json = data.ok_or_else(|| "session has no persisted row yet".to_string())?;
        let loaded: Session = serde_json::from_str(&json)
            .map_err(|error| format!("session row parse failed: {error}"))?;
        Ok(QuotaMeterReading {
            consumed_tokens: u64::try_from(loaded.quota.consumed_tokens).unwrap_or(u64::MAX),
            consumed_tool_steps: u64::try_from(loaded.quota.consumed_tool_steps)
                .unwrap_or(u64::MAX),
            consumed_cost_micros: loaded.quota.consumed_cost_micros,
            unpriced_rounds: u64::try_from(loaded.quota.unpriced_rounds).unwrap_or(u64::MAX),
        })
    })
}

/// 自省通道: 用户域节探测口 (W3-①) —— 现探测当前域的域文档真值与域内计数。
///
/// - 域身份 (id/显示名/角色/创建时刻) 每次现读域文档 (坏域配置拒开, 如实);
/// - 域内计数: 会话数读本域会话库 (与配额账本同库), 记忆/受保护数读本域记忆库
///   账本; 单维失败 = 该维显式 null + 原因, 不整节失败 (0 装诚实)。
pub fn domain_status_probe(
    registry: Arc<apeireth_core::user_domain::DomainRegistry>,
    user_id: apeireth_core::user_domain::DomainUserId,
    sessions_db: PathBuf,
    memory: Arc<apeireth_memory::SqliteMemoryStore>,
) -> StatusProbe<DomainStatus> {
    Arc::new(move || {
        let domain = registry.load(&user_id).map_err(|error| error.to_string())?;
        let mut count_reason: Option<String> = None;
        let sessions = match crate::domain_plane::session_row_count(&sessions_db) {
            Ok(count) => Some(count),
            Err(error) => {
                count_reason = Some(error);
                None
            }
        };
        let ledger = match memory.ledger_counts() {
            Ok(counts) => Some(counts),
            Err(error) => {
                count_reason.get_or_insert_with(|| format!("memory ledger probe failed: {error}"));
                None
            }
        };
        Ok(DomainStatus {
            user_id: domain.user_id.to_string(),
            display_name: domain.display_name.clone(),
            role: domain.role_str().to_string(),
            created_at: domain.created_at,
            counts: DomainCounts {
                sessions,
                memories: ledger.as_ref().map(|counts| counts.memories),
                protected: ledger.as_ref().map(|counts| counts.protected),
                reason: count_reason,
            },
        })
    })
}

/// 自省通道: 双引擎关怀节探测口 (V3-a) —— 现探测 `care_rules.json` 真值
/// (规则数 / 下次触发 / 最近关怀), 映射到自述关怀节。
///
/// 时钟与交互锚点由调用方注入: `now_ms()` 取现时刻 (与关怀引擎同一时钟合同),
/// `last_interaction_ms()` 取进程内交互观察 (缺位时久未联系类规则以规则创建
/// 时刻为锚, 如实)。拒开 (StoredDoc 语义) = `Err(真因)`, 自述节显式 null +
/// 原因, 不编数。
pub fn care_status_probe(
    data_dir: PathBuf,
    now_ms: Arc<dyn Fn() -> i64 + Send + Sync>,
    last_interaction_ms: Arc<dyn Fn() -> Option<i64> + Send + Sync>,
) -> StatusProbe<CareStatus> {
    Arc::new(move || {
        let store = apeireth_organ::care::CareStore::new(data_dir.clone());
        let status = store
            .status(now_ms(), last_interaction_ms())
            .map_err(|error| error.to_string())?;
        let last_care_note = status
            .last_care
            .as_ref()
            .map(|record| {
                let action = if record.spoke { "spoke" } else { "declined" };
                format!(
                    "{}: {} ({}): {}",
                    record.rule_id, action, record.judge, record.note
                )
            })
            .or_else(|| Some("尚无关怀记录".to_string()));
        Ok(CareStatus {
            rules: Some(status.rules),
            next_trigger_at_ms: status.next_trigger_at_ms,
            last_care_at_ms: status.last_care.as_ref().map(|record| record.at_ms),
            last_care_note,
            reason: None,
        })
    })
}

/// 自省通道: 任务节探测口 (V6-a) —— 读持久任务队列的分态计数真值
/// (pending/assigned/running/done/failed/blocked), 只回计数不回任务正文。
///
/// 探测口绑同一个 [`apeireth_orchestration::task_queue::TaskQueue`] 实例
/// (与 `tool.task_board` 工具面同队同账): 自述与执行面同一真值源。
pub fn task_board_status_probe(
    queue: Arc<apeireth_orchestration::task_queue::TaskQueue>,
) -> StatusProbe<TaskBoardStatus> {
    Arc::new(move || {
        let counts = queue
            .counts()
            .map_err(|error| format!("task queue read failed: {error}"))?;
        Ok(TaskBoardStatus {
            pending: counts.pending,
            assigned: counts.assigned,
            running: counts.running,
            done: counts.done,
            failed: counts.failed,
            blocked: counts.blocked,
        })
    })
}

/// 自省通道: 人格节探测口 (`self_status_persona`, V4-a) —— 读人格层落库面
/// 自述节真值 (信念/边界/偏好/张力计数 + 最后整编时刻, 只计数不回内容)。
///
/// 探测口绑同一个 [`apeireth_memory::PersonaMemoryStore`] 实例 (与人格提取步
/// 的 `persona_memory_*` 分表同库同账): 自述与落库面同一真值源。读失败 =
/// `Err(真因)`, 自述节显式 null + 原因, 不编数 (0 装诚实)。
pub fn persona_status_probe(
    store: Arc<dyn apeireth_memory::PersonaMemoryStore>,
) -> StatusProbe<apeireth_tools_canonical::PersonaStatusSection> {
    use apeireth_memory::PersonaMemoryStore as _;
    Arc::new(move || {
        let status = store
            .status()
            .map_err(|error| format!("persona store read failed: {error}"))?;
        Ok(apeireth_tools_canonical::PersonaStatusSection {
            core_beliefs: status.core_beliefs,
            boundaries: status.boundaries,
            preferences: status.preferences,
            tensions: status.tensions,
            last_consolidation_at: status.last_consolidation_at,
            reason: None,
        })
    })
}

/// 生产自述来源: 装配期收集生效值, 调用期现探测 data 系。
pub struct ProductionSelfStatusSource {
    identity: SelfStatusIdentity,
    capabilities: Vec<CapabilitySwitch>,
    ledger: Option<StatusProbe<MemoryLedgerStats>>,
    credentials: Option<StatusProbe<bool>>,
    workspace_root: Option<PathBuf>,
    self_learning: bool,
    budget: BudgetStatus,
    quota: Option<SessionQuotaProbe>,
    model_profile: Option<ModelProfile>,
    domain: Option<StatusProbe<DomainStatus>>,
    content_feed: Option<StatusProbe<ContentFeedStatus>>,
    care: Option<StatusProbe<CareStatus>>,
    backup: Option<StatusProbe<BackupStatus>>,
    task_board: Option<StatusProbe<TaskBoardStatus>>,
    voice: Option<StatusProbe<VoiceStatus>>,
    persona: Option<StatusProbe<apeireth_tools_canonical::PersonaStatusSection>>,
    /// 性格节真值探测口 (裁定 ④⑤): 四杆锁定态 + 最近移动 (成长计既有账)。
    trait_growth: Option<StatusProbe<TraitStatus>>,
}

impl ProductionSelfStatusSource {
    /// 只有生效值部分的来源 (data 系探测口缺位时对应字段显式 null + 原因)。
    pub fn new(
        capabilities: Vec<CapabilitySwitch>,
        self_learning: bool,
        workspace_root: Option<PathBuf>,
    ) -> Self {
        Self {
            identity: SelfStatusIdentity::default(),
            capabilities,
            ledger: None,
            credentials: None,
            workspace_root,
            self_learning,
            budget: budget_status_from_constants(),
            quota: None,
            model_profile: None,
            domain: None,
            content_feed: None,
            care: None,
            backup: None,
            task_board: None,
            voice: None,
            persona: None,
            trait_growth: None,
        }
    }

    /// 接入记忆账本计数探测口 (会话/记忆/保护/教训计数)。
    #[must_use]
    pub fn with_ledger_probe(mut self, probe: StatusProbe<MemoryLedgerStats>) -> Self {
        self.ledger = Some(probe);
        self
    }

    /// 接入凭据存在性探测口 (只回布尔, 不回显凭据本体)。
    #[must_use]
    pub fn with_credentials_probe(mut self, probe: StatusProbe<bool>) -> Self {
        self.credentials = Some(probe);
        self
    }

    /// 接入会话配额计量探测口 (按会话 id 读已耗真值, 只回数字)。
    #[must_use]
    pub fn with_quota_probe(mut self, probe: SessionQuotaProbe) -> Self {
        self.quota = Some(probe);
        self
    }

    /// 覆写预算一节 (可配置口径落地时由组装根传入实际值)。
    #[must_use]
    pub fn with_budget(mut self, budget: BudgetStatus) -> Self {
        self.budget = budget;
        self
    }

    /// 注入生效模型的画像 (模型面轴 H-P1): 组装根把画像表解析出的生效模型行
    /// 传入 (未知模型传显式 unknown 行, 不是 None); 缺位时模型节显式 null +
    /// 原因。
    #[must_use]
    pub fn with_model_profile(mut self, profile: ModelProfile) -> Self {
        self.model_profile = Some(profile);
        self
    }

    /// 接入用户域节探测口 (W3-①): 当前域 id/角色/域内计数, 每次自述现探测。
    #[must_use]
    pub fn with_domain_probe(mut self, probe: StatusProbe<DomainStatus>) -> Self {
        self.domain = Some(probe);
        self
    }

    /// 接入内容流面探测口 (J1): 订阅数/最近拉取, 每次自述现探测 (真值如实)。
    #[must_use]
    pub fn with_content_feed_probe(mut self, probe: StatusProbe<ContentFeedStatus>) -> Self {
        self.content_feed = Some(probe);
        self
    }

    /// 接入双引擎关怀节探测口 (V3-a): 规则数/下次触发/最近关怀, 每次自述现
    /// 探测 (真值如实)。
    #[must_use]
    pub fn with_care_probe(mut self, probe: StatusProbe<CareStatus>) -> Self {
        self.care = Some(probe);
        self
    }

    /// 接入备份节探测口 (G5): 最近备份时间/清单件数真值, 每次自述现探测
    /// (尚无备份显式 null + 原因, 不编时间)。
    #[must_use]
    pub fn with_backup_probe(mut self, probe: StatusProbe<BackupStatus>) -> Self {
        self.backup = Some(probe);
        self
    }

    /// 接入任务节探测口 (V6-a): 任务队列分态计数真值, 每次自述现探测
    /// (只回计数不回任务正文)。
    #[must_use]
    pub fn with_task_board_probe(mut self, probe: StatusProbe<TaskBoardStatus>) -> Self {
        self.task_board = Some(probe);
        self
    }

    /// 接入语音节探测口 (V3-b): 账本数/最近说话/嗓名, 每次自述现探测
    /// (与发声工具同一本语句账本; 摘录口径不回全文)。
    #[must_use]
    pub fn with_voice_probe(mut self, probe: StatusProbe<VoiceStatus>) -> Self {
        self.voice = Some(probe);
        self
    }

    /// 接入人格节探测口 (`self_status_persona`, V4-a): 信念/边界/偏好/张力
    /// 计数 + 最后整编真值, 每次自述现探测 (只计数不回内容; 与人格层落库面
    /// 同一真值源)。
    #[must_use]
    pub fn with_persona_probe(
        mut self,
        probe: StatusProbe<apeireth_tools_canonical::PersonaStatusSection>,
    ) -> Self {
        self.persona = Some(probe);
        self
    }

    /// 接入性格节真值探测口 (裁定 ④⑤): 四杆锁定态 + 最近移动, 每次自述现
    /// 探测 (与 `tool.trait_report` 同一成长计既有账; 缺源/读失败显式 null +
    /// 原因, 不编锁态/移动史)。
    #[must_use]
    pub fn with_trait_growth_probe(mut self, probe: StatusProbe<TraitStatus>) -> Self {
        self.trait_growth = Some(probe);
        self
    }

    /// 自述身份 (产品名 / workspace 版本 / 运行时角色)。
    pub fn identity(&self) -> &SelfStatusIdentity {
        &self.identity
    }

    /// 配额各维的已耗列: 会话作用域 + 探测口都在场才有真值, 否则显式
    /// `null` + 原因 (0 装诚实: 无作用域不编造数字)。`apply` 决定读数怎么
    /// 落进行里 (花费维要额外带"部分计量"标注, 其余维直接换真值)。
    fn quota_row_with(
        &self,
        row: QuotaDimensionStatus,
        session: Option<SessionId>,
        apply: impl FnOnce(QuotaDimensionStatus, &QuotaMeterReading) -> QuotaDimensionStatus,
    ) -> QuotaDimensionStatus {
        let Some(session) = session else {
            return row.with_missing_reading(QUOTA_METER_SCOPE_REQUIRED);
        };
        let Some(probe) = &self.quota else {
            return row.with_missing_reading(DATA_PROBE_NOT_WIRED);
        };
        match probe(session) {
            Ok(reading) => apply(row, &reading),
            Err(error) => row.with_missing_reading(format!("quota meter probe failed: {error}")),
        }
    }

    fn quota_row(
        &self,
        row: QuotaDimensionStatus,
        session: Option<SessionId>,
        pick: fn(&QuotaMeterReading) -> u64,
    ) -> QuotaDimensionStatus {
        self.quota_row_with(row, session, |row, reading| row.with_reading(pick(reading)))
    }

    /// 预算一节按当前作用域落真值: 上限来自装配注入的生效旋钮 (与运行时
    /// 同源), 已耗/余量来自会话账本; 花费维含无价轮时另如实标"部分计量"。
    fn budget_with_quota_reading(&self, session: Option<SessionId>) -> BudgetStatus {
        let mut budget = self.budget.clone();
        budget.quota.tokens = self.quota_row(budget.quota.tokens, session, |reading| {
            reading.consumed_tokens
        });
        budget.quota.tool_steps = self.quota_row(budget.quota.tool_steps, session, |reading| {
            reading.consumed_tool_steps
        });
        budget.quota.cost_micros =
            self.quota_row_with(budget.quota.cost_micros, session, |row, reading| {
                row.with_cost_reading(reading.consumed_cost_micros, reading.unpriced_rounds)
            });
        budget
    }

    fn snapshot_with_scope(&self, session: Option<SessionId>) -> SelfStatusSnapshot {
        // 调参状态: 四滑杆当前**生效值** (进程内自动调整 > env 旋钮 > 基线),
        // 预设名由取值集推导, 自学习开关取装配时的接线事实。性格节真值
        // (锁定态 + 最近移动, 裁定 ④⑤) 现读成长账面: 缺源/读失败 = 显式
        // null + 原因 (0 装诚实, 不编锁态/移动史)。
        let values = [
            TunableParam::MemoryFade.effective(),
            TunableParam::CuriosityStrength.effective(),
            TunableParam::ToneSaturation.effective(),
            TunableParam::ConsolidationCadence.effective(),
        ];
        let (trait_locked, trait_recent_moves, trait_reason) = match &self.trait_growth {
            Some(probe) => match probe() {
                Ok(status) => (Some(status.locked), Some(status.recent_moves), None),
                Err(error) => (
                    None,
                    None,
                    Some(format!("trait growth probe failed: {error}")),
                ),
            },
            None => (None, None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };
        let tuning = TuningStatus {
            memory_fade: values[0],
            curiosity_strength: values[1],
            tone_saturation: values[2],
            consolidation_cadence: values[3],
            preset: derive_tuning_preset(values),
            self_learning: self.self_learning,
            // 性格节真值真接线 (裁定 ④⑤ 落地批): 锁定态 + 最近移动来自
            // `trait_growth` 探测口 (成长计既有账, 与 `tool.trait_report` 同账)。
            locked: trait_locked,
            recent_moves: trait_recent_moves,
        };

        // 记忆账本: 现探测; 失败/缺位 → 计数显式 null + 原因。
        let mut reasons: Vec<String> = Vec::new();
        let memory_ledger = match &self.ledger {
            Some(probe) => match probe() {
                Ok(stats) => {
                    if let Some(reason) = &stats.reason {
                        reasons.push(reason.clone());
                    }
                    Some(stats)
                }
                Err(error) => {
                    reasons.push(format!("memory ledger probe failed: {error}"));
                    None
                }
            },
            None => {
                reasons.push(DATA_PROBE_NOT_WIRED.to_string());
                None
            }
        };
        let memory_ledger_reason = reasons.first().cloned();

        // 工作区: 根路径 + 凭据存在性布尔 (凭据本体绝不回显)。
        let root = self
            .workspace_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());
        let mut workspace_reason: Option<String> = None;
        let credentials_present = match &self.credentials {
            Some(probe) => match probe() {
                Ok(present) => Some(present),
                Err(error) => {
                    workspace_reason = Some(format!("credentials probe failed: {error}"));
                    None
                }
            },
            None => {
                workspace_reason = Some(DATA_PROBE_NOT_WIRED.to_string());
                None
            }
        };

        // 模型画像: 装配期解析的生效模型行 (H-P1); 缺位显式 null + 原因。
        let (model, model_reason) = match &self.model_profile {
            Some(profile) => (Some(profile.clone()), None),
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 用户域一节 (W3-① 域节): 当前域 id/角色/域内计数, 现探测如实;
        // 探测口缺位 = 显式 null + 原因 (W3-2 报备的域面接线在本批落地)。
        let (domain, domain_reason) = match &self.domain {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("user domain probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 内容流面一节 (J1): 订阅数/最近拉取, 现探测如实; 探测口缺位 = 显式
        // null + 原因 (0 装诚实)。
        let (content_feed, content_feed_reason) = match &self.content_feed {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("content feed probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 双引擎关怀一节 (V3-a): 规则数/下次触发/最近关怀真值, 现探测如实;
        // 探测口缺位 = 显式 null + 原因 (0 装诚实)。
        let (care, care_reason) = match &self.care {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("care probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 备份一节 (G5): 最近备份时间/清单件数真值, 现探测如实; 尚无备份 =
        // 显式 null + 「尚无备份记录」(不编时间), 探测口缺位 = 显式 null + 原因。
        let (backup, backup_reason) = match &self.backup {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("backup probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 任务节 (V6-a): 任务队列分态计数真值, 现探测如实 (只回计数);
        // 探测口缺位 = 显式 null + 原因 (0 装诚实)。
        let (task_board, task_board_reason) = match &self.task_board {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("task board probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 语音节 (V3-b): 账本数/最近说话/嗓名, 现探测如实 (与发声工具同一本
        // 语句账本); 探测口缺位 = 显式 null + 原因 (0 装诚实)。
        let (voice, voice_reason) = match &self.voice {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("voice probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        // 人格节 (V4-a): 信念/边界/偏好/张力计数 + 最后整编真值, 现探测如实
        // (与人格层落库面同账, 只计数不回内容); 探测口缺位 = 显式 null + 原因
        // (0 装诚实)。
        let (persona, persona_reason) = match &self.persona {
            Some(probe) => match probe() {
                Ok(status) => (Some(status), None),
                Err(error) => (None, Some(format!("persona probe failed: {error}"))),
            },
            None => (None, Some(DATA_PROBE_NOT_WIRED.to_string())),
        };

        SelfStatusSnapshot {
            identity: self.identity.clone(),
            capabilities: self.capabilities.clone(),
            memory_ledger,
            memory_ledger_reason,
            tuning: Some(tuning),
            tuning_reason: trait_reason,
            budget: Some(self.budget_with_quota_reading(session)),
            budget_reason: None,
            workspace: Some(WorkspaceStatus {
                root,
                credentials_present,
            }),
            workspace_reason,
            model,
            model_reason,
            domain,
            domain_reason,
            content_feed,
            content_feed_reason,
            care,
            care_reason,
            backup,
            backup_reason,
            task_board,
            task_board_reason,
            voice,
            voice_reason,
            persona,
            persona_reason,
        }
    }
}

impl SelfStatusSource for ProductionSelfStatusSource {
    fn snapshot(&self) -> SelfStatusSnapshot {
        self.snapshot_with_scope(None)
    }

    fn snapshot_in_session(&self, session: Option<SessionId>) -> SelfStatusSnapshot {
        self.snapshot_with_scope(session)
    }
}

/// 能力名册投影: 把**实际用于装配的** [`ProductionModulesConfig`] 与注入槽
/// 存在性投影成开关全表 (真实生效值, 非配置文本), 外加组装根补充行
/// (例如授权层的 `local_read_tools` 旋钮)。
///
/// 本地文件工具行按**实际注册条件**取值 (配置开启且工作区根已注入才算生效),
/// 名册不复述"配置说开"而注册没成的事实。同一口径延伸到挂在别的槽上的
/// 开关 (开关生效链审计续): 记忆固化挂在记忆写入模块、检索深度自适应与
/// 图社区分诊挂在记忆召回模块、前瞻召回挂在记忆召回模块、记忆注入挂在
/// 记忆协调器 —— 上游槽缺席时下游开关静默失效, 名册一律照实报 false。
/// `self_tuning` / `typed_recall` 两个注入槽参数为接线事实, 本函数再 AND
/// 上游槽 (自学习挂记忆召回, 类型化召回挂记忆协调器), 与配置行同口径。
///
/// MCP 外部工具桥一行的自报语义 (如实): 桥已装配**且**有可用服务器配置
/// 才算外部工具面生效 —— 已启用但无服务器配置 = 无外部工具 (照实 false);
/// 服务器配置的折叠由组装根在 [`super::production::ProductionModules::build`]
/// 完成 (配置坏 = 拒开, 语义不变)。
pub fn roster_from_config(
    config: &ProductionModulesConfig,
    workspace_root_present: bool,
    self_tuning: bool,
    typed_recall: bool,
    extras: &[CapabilitySwitch],
) -> Vec<CapabilitySwitch> {
    let mut roster = vec![
        CapabilitySwitch::new("filesystem", config.filesystem && workspace_root_present),
        CapabilitySwitch::new("search", config.search && workspace_root_present),
        CapabilitySwitch::new("repo", config.repo && workspace_root_present),
        CapabilitySwitch::new("shell", config.shell.is_some()),
        CapabilitySwitch::new("fetch", config.fetch.is_some()),
        // 内容流接入 (`tool.content_feed`, J1): 按实际注册条件取值 (配置注入
        // 才算生效)。治理: RequireApproval 级 (拉取与入库都经审批档)。
        CapabilitySwitch::new("content_feed", config.content_feed.is_some()),
        // V3-a 双引擎主动关怀的规则面 (注册条件 = 数据目录已注入, 注册即闸),
        // 名册按实际注册条件取值。
        CapabilitySwitch::new("care_schedule", config.care_schedule.is_some()),
        // G5 备份导出 (`tool.backup_export`, 注册条件 = 数据源配置已注入,
        // 注册即闸), 名册按实际注册条件取值。
        CapabilitySwitch::new("backup_export", config.backup.is_some()),
        // V3-b 通用嗓发声 (`tool.voice_speak`, 注册条件 = 发声配置已注入,
        // 注册即闸), 名册按实际注册条件取值。
        CapabilitySwitch::new("voice_speak", config.voice_speak.is_some()),
        // V6-a 任务簿 (`tool.task_board`, 注册条件 = 任务队列已注入, 注册即闸),
        // 名册按实际注册条件取值 (自述任务节与工具面同队同账)。
        CapabilitySwitch::new("task_board", config.task_board.is_some()),
        // 性格周报 (`tool.trait_report`, 裁定 ⑤, 注册条件 = 成长账落位已注入,
        // 注册即闸), 名册按实际注册条件取值 (自述性格节与工具面同账)。
        CapabilitySwitch::new("trait_report", config.trait_report.is_some()),
        // 受控文件写入 (`tool.apply_patch`): 主开关按实际注册条件取值 (配置开启
        // 且工作区根已注入才算生效); 自动放行子档依赖主开关 —— 主开关没注册成,
        // 子档无处生效, 名册照实说 false。
        CapabilitySwitch::new("file_write", config.file_write && workspace_root_present),
        CapabilitySwitch::new(
            "file_write_auto_pass",
            config.file_write_auto_pass && config.file_write && workspace_root_present,
        ),
        // git 写分级 (`tool.git_write`): 主开关按实际注册条件取值 (配置开启
        // 且工作区根已注入才算生效); 「自动放行 L1」子档依赖主开关, 且只放
        // L1 可逆本地写 —— L2 出网永远人批不入子档。
        CapabilitySwitch::new("git_write", config.git_write && workspace_root_present),
        CapabilitySwitch::new(
            "git_write_auto_pass_l1",
            config.git_write_auto_pass_l1 && config.git_write && workspace_root_present,
        ),
        // 干活能力轴 P2 (worktree 计划/落盘两级): 按实际注册条件取值 —— 开关
        // + `file_write` 唯一写总闸 + 工作区根三者齐备才算生效 (子依赖写总闸,
        // fail-closed); 写总闸没开, 两级照实 false。
        CapabilitySwitch::new(
            "worktree_plan",
            config.worktree_plan && config.file_write && workspace_root_present,
        ),
        CapabilitySwitch::new(
            "worktree_apply",
            config.worktree_apply && config.file_write && workspace_root_present,
        ),
        CapabilitySwitch::new("mcp", config.mcp),
        // 书→技能沉淀 (I1): 写面按实际注册条件取值 (开关 + `file_write` 唯一写
        // 总闸 + 工作区根 + 技能库目录齐备才算生效, fail-closed); 只读检索按
        // (开关 + 技能库目录) 取值, 不依赖写总闸。
        CapabilitySwitch::new(
            "skill_distill",
            config.skill_distill
                && config.file_write
                && workspace_root_present
                && config.skills_dir.is_some(),
        ),
        CapabilitySwitch::new(
            "skill_search",
            config.skill_search && config.skills_dir.is_some(),
        ),
        CapabilitySwitch::new("education", config.education),
        CapabilitySwitch::new("memory_recall", config.memory_recall),
        CapabilitySwitch::new("memory_writeback", config.memory_writeback),
        CapabilitySwitch::new(
            "memory_injection",
            config.memory_injection && (config.memory_recall || config.memory_writeback),
        ),
        CapabilitySwitch::new(
            "proactive_recall",
            config.proactive_recall.is_some() && config.memory_recall,
        ),
        // 类型化召回候选只走记忆协调器 (记忆召回或记忆写入在场), 同口径。
        CapabilitySwitch::new(
            "typed_recall",
            typed_recall && (config.memory_recall || config.memory_writeback),
        ),
        CapabilitySwitch::new("preference_recall", config.preference_recall),
        CapabilitySwitch::new("preference_learning", config.preference_learning),
        // 记忆固化挂在记忆写入模块 (`with_consolidation`): 写入模块缺席时
        // 静默失效, 名册照实 false (按实际注册条件取值)。
        CapabilitySwitch::new(
            "consolidation",
            config.consolidation && config.memory_writeback,
        ),
        // V4-a 人格提取步挂在记忆写入模块的整编触发点 (`with_persona_layer`):
        // 人格层落库面缺席由组装根折进生效视图, 这里再 AND 上游槽 (写入模块 +
        // 整编触发点) —— 上游槽缺席时静默失效, 名册照实 false。
        CapabilitySwitch::new(
            "persona_consolidation",
            config.persona_consolidation && config.consolidation && config.memory_writeback,
        ),
        // V4-b 散列增量门挂在记忆写入模块的整编触发点 (`with_ingest_gate`):
        // 散列门落库面缺席由组装根折进生效视图, 这里再 AND 上游槽 (写入模块 +
        // 整编触发点) —— 上游槽缺席时静默失效, 名册照实 false。
        CapabilitySwitch::new(
            "consolidation_hash_gate",
            config.consolidation_hash_gate && config.consolidation && config.memory_writeback,
        ),
        CapabilitySwitch::new("reflexion", config.reflexion),
        CapabilitySwitch::new("self_assessment", config.self_assessment),
        CapabilitySwitch::new("judge", config.judge.enabled),
        CapabilitySwitch::new("council", config.council),
        CapabilitySwitch::new("organs", config.organs),
        CapabilitySwitch::new("partner_bond", config.partner_bond),
        // 检索深度自适应挂在记忆召回模块 (`with_morphology_recall`), 同口径。
        CapabilitySwitch::new(
            "morphology_recall",
            config.morphology_recall && config.memory_recall,
        ),
        // 图社区分诊挂在记忆召回模块 (`with_community_triage`), 同口径
        // (图谱槽在否由组装根折进生效视图)。
        CapabilitySwitch::new(
            "community_triage",
            config.community_triage && config.memory_recall,
        ),
        CapabilitySwitch::new("absorption_insight", config.absorption_insight),
        // 自学习信号只走记忆召回模块 (`with_self_tuning`): 召回缺席时接线
        // 无处生效, 名册照实 false (接线事实 AND 上游槽)。
        CapabilitySwitch::new("self_tuning", self_tuning && config.memory_recall),
    ];
    roster.extend(extras.iter().cloned());
    roster.sort_by(|a, b| a.name.cmp(&b.name));
    roster
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_status_reads_the_constants_and_says_so() {
        let budget = budget_status_from_constants();
        assert_eq!(budget.max_rounds_per_turn, 8);
        assert_eq!(budget.max_tool_calls_per_round, 16);
        assert_eq!(budget.source, BUDGET_SOURCE_CONSTANT);
        assert!(budget.note.contains("constants"), "{}", budget.note);
        // 配额三维: 无旋钮 = 不设限 (上限真值 None), 已耗列无探测口显式 null。
        assert_eq!(budget.quota.tokens.limit, None);
        assert_eq!(budget.quota.tool_steps.limit, None);
        assert_eq!(budget.quota.cost_micros.limit, None);
        assert_eq!(budget.quota.tokens.consumed, None);
        assert_eq!(
            budget.quota.cost_micros.reason.as_deref(),
            Some(DATA_PROBE_NOT_WIRED)
        );
        // 深度维如实"暂无接口"。
        assert_eq!(
            budget.quota.depth.reason.as_deref(),
            Some(QUOTA_DEPTH_NOT_WIRED)
        );
    }

    #[test]
    fn configured_budget_status_carries_the_live_knob_values() {
        let budget = budget_status_from_configured(2, 4, Some(5_000), None, Some(7_000));
        assert_eq!(budget.max_rounds_per_turn, 2);
        assert_eq!(budget.max_tool_calls_per_round, 4);
        assert_eq!(budget.source, BUDGET_SOURCE_CONFIGURED);
        assert!(budget.note.contains("knobs"), "{}", budget.note);
        // 配额上限与运行时同源: Some 直通, None = 不设限 (显示 "—")。
        assert_eq!(budget.quota.tokens.limit, Some(5_000));
        assert_eq!(budget.quota.tool_steps.limit, None);
        assert_eq!(budget.quota.cost_micros.limit, Some(7_000));
        assert_eq!(budget.quota.tool_steps.display, "—/—");
        assert_eq!(budget.quota.tokens.display, "—/5000");
    }

    #[test]
    fn roster_projects_the_effective_config_switches() {
        let mut config = ProductionModulesConfig::default();
        config.shell = None;
        config.fetch = None;
        config.organs = true;
        let extras = vec![CapabilitySwitch::new("local_read_tools", true)];
        let roster = roster_from_config(&config, true, true, true, &extras);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["shell"], false);
        assert_eq!(by_name["fetch"], false);
        assert_eq!(by_name["organs"], true);
        assert_eq!(by_name["self_tuning"], true);
        assert_eq!(by_name["typed_recall"], true);
        assert_eq!(by_name["local_read_tools"], true);
        assert_eq!(by_name["memory_recall"], true);
        assert_eq!(by_name["filesystem"], true);

        // 无工作区根 = 本地文件工具没注册成, 名册照实说 false。
        let roster = roster_from_config(&config, false, true, true, &extras);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["filesystem"], false);
        assert_eq!(by_name["search"], false);
        assert_eq!(by_name["repo"], false);
    }

    #[test]
    fn roster_reports_silent_dependency_switches_by_registration_condition() {
        // 挂在别的槽上的开关: 上游槽缺席 = 静默失效, 名册照实 false;
        // 上游槽在场且开关开 = 真生效, 照实 true。
        let mut config = ProductionModulesConfig::default();
        config.memory_recall = false;
        config.memory_writeback = false;
        config.consolidation = true;
        config.morphology_recall = true;
        config.community_triage = true;
        config.memory_injection = true;
        config.proactive_recall = Some(apeireth_memory::ProactiveRecallPolicy::default());
        let roster = roster_from_config(&config, true, true, true, &[]);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["consolidation"], false, "记忆写入缺席");
        assert_eq!(by_name["morphology_recall"], false, "记忆召回缺席");
        assert_eq!(by_name["community_triage"], false, "记忆召回缺席");
        assert_eq!(by_name["self_tuning"], false, "记忆召回缺席");
        assert_eq!(by_name["proactive_recall"], false, "记忆召回缺席");
        assert_eq!(by_name["memory_injection"], false, "记忆协调器缺席");

        config.memory_recall = true;
        config.memory_writeback = true;
        let roster = roster_from_config(&config, true, true, true, &[]);
        let by_name: std::collections::HashMap<&str, bool> = roster
            .iter()
            .map(|row| (row.name.as_str(), row.enabled))
            .collect();
        assert_eq!(by_name["consolidation"], true);
        assert_eq!(by_name["morphology_recall"], true);
        assert_eq!(by_name["community_triage"], true);
        assert_eq!(by_name["self_tuning"], true);
        assert_eq!(by_name["proactive_recall"], true);
        assert_eq!(by_name["memory_injection"], true);
    }

    #[test]
    fn missing_data_probes_become_explicit_nulls_with_reasons() {
        let source = ProductionSelfStatusSource::new(
            vec![CapabilitySwitch::new("shell", false)],
            false,
            Some(PathBuf::from("C:\\work")),
        );
        let snapshot = source.snapshot();
        assert!(snapshot.memory_ledger.is_none());
        assert!(snapshot
            .memory_ledger_reason
            .as_deref()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED));
        let workspace = snapshot.workspace.expect("workspace section");
        assert_eq!(workspace.root.as_deref(), Some("C:\\work"));
        assert!(workspace.credentials_present.is_none());
        assert!(snapshot
            .workspace_reason
            .as_deref()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED));
        // 生效值部分照常在场。
        assert!(snapshot.tuning.is_some());
        assert!(snapshot.budget.is_some());
    }

    #[test]
    fn ledger_probe_failure_is_a_reason_not_a_failed_frame() {
        let probe: StatusProbe<MemoryLedgerStats> = Arc::new(|| Err("db locked".to_string()));
        let source =
            ProductionSelfStatusSource::new(Vec::new(), false, None).with_ledger_probe(probe);
        let snapshot = source.snapshot();
        assert!(snapshot.memory_ledger.is_none());
        assert!(
            snapshot
                .memory_ledger_reason
                .as_deref()
                .unwrap()
                .contains("db locked"),
            "{:?}",
            snapshot.memory_ledger_reason
        );
    }

    #[test]
    fn probes_feed_real_counts_and_credentials_presence() {
        let ledger_probe: StatusProbe<MemoryLedgerStats> = Arc::new(|| {
            Ok(MemoryLedgerStats {
                sessions: Some(2),
                memories: Some(5),
                protected: Some(1),
                lessons: Some(3),
                // V4-b 保鲜回路三真值随探测口透传 (缺源显式 null 由字段级 Option 承载)。
                pending_ingest: Some(1),
                last_fold_at_ms: Some(1_728_000_900_000),
                dedup_skipped: Some(7),
                reason: None,
            })
        });
        let credentials_probe: StatusProbe<bool> = Arc::new(|| Ok(true));
        let source = ProductionSelfStatusSource::new(Vec::new(), true, Some(PathBuf::from("root")))
            .with_ledger_probe(ledger_probe)
            .with_credentials_probe(credentials_probe);
        let snapshot = source.snapshot();
        let stats = snapshot.memory_ledger.expect("ledger");
        assert_eq!(stats.sessions, Some(2));
        assert_eq!(stats.memories, Some(5));
        assert_eq!(stats.protected, Some(1));
        assert_eq!(stats.lessons, Some(3));
        assert_eq!(stats.pending_ingest, Some(1), "待摄取数真值透传");
        assert_eq!(
            stats.last_fold_at_ms,
            Some(1_728_000_900_000),
            "最近折入真值透传"
        );
        assert_eq!(stats.dedup_skipped, Some(7), "去重计数真值透传");
        assert!(snapshot.memory_ledger_reason.is_none());
        let workspace = snapshot.workspace.expect("workspace");
        assert_eq!(workspace.credentials_present, Some(true));
        assert!(snapshot.workspace_reason.is_none());
        assert!(snapshot.tuning.expect("tuning").self_learning);
    }

    #[test]
    fn quota_rows_pick_up_the_live_reading_only_inside_a_session_scope() {
        let probe: SessionQuotaProbe = Arc::new(|_session| {
            Ok(QuotaMeterReading {
                consumed_tokens: 120,
                consumed_tool_steps: 3,
                consumed_cost_micros: 2_000,
                unpriced_rounds: 2,
            })
        });
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_budget(budget_status_from_configured(
                8,
                16,
                Some(5_000),
                Some(30),
                Some(10_000),
            ))
            .with_quota_probe(probe);

        // 会话作用域内: 三维真值 (上限/已耗/余量), 不设限侧显 "—"。
        let session = SessionId::new();
        let budget = source
            .snapshot_in_session(Some(session))
            .budget
            .expect("budget");
        assert_eq!(budget.quota.tokens.limit, Some(5_000));
        assert_eq!(budget.quota.tokens.consumed, Some(120));
        assert_eq!(budget.quota.tokens.remaining, Some(4_880));
        assert_eq!(budget.quota.tokens.display, "120/5000");
        assert_eq!(budget.quota.tool_steps.consumed, Some(3));
        assert_eq!(budget.quota.tool_steps.remaining, Some(27));
        assert_eq!(budget.quota.tool_steps.display, "3/30");
        // 花费维真值 + 无价轮如实标"部分计量" (2 轮无单价未计入)。
        let cost = &budget.quota.cost_micros;
        assert_eq!(cost.limit, Some(10_000));
        assert_eq!(cost.consumed, Some(2_000));
        assert_eq!(cost.remaining, Some(8_000));
        assert_eq!(cost.display, "2000/10000");
        assert_eq!(
            cost.reason,
            Some(apeireth_tools_canonical::quota_cost_partial_note(2)),
            "2 轮无单价未计入: 如实标部分计量"
        );

        // 无会话作用域: 已耗列显式 null + 原因, 上限真值不变。
        let budget = source.snapshot().budget.expect("budget");
        assert_eq!(budget.quota.tokens.limit, Some(5_000));
        assert_eq!(budget.quota.tokens.consumed, None);
        assert_eq!(
            budget.quota.tokens.reason.as_deref(),
            Some(QUOTA_METER_SCOPE_REQUIRED)
        );
        assert_eq!(budget.quota.tool_steps.consumed, None);
        assert_eq!(budget.quota.cost_micros.consumed, None);
    }

    #[test]
    fn a_fully_priced_cost_reading_carries_no_partial_note() {
        // 全量口径 (0 无价轮) 不标"部分计量": 标注只在缺口真存在时出现。
        let probe: SessionQuotaProbe = Arc::new(|_session| {
            Ok(QuotaMeterReading {
                consumed_tokens: 1,
                consumed_tool_steps: 1,
                consumed_cost_micros: 7_000,
                unpriced_rounds: 0,
            })
        });
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_budget(budget_status_from_configured(8, 16, None, None, None))
            .with_quota_probe(probe);
        let budget = source
            .snapshot_in_session(Some(SessionId::new()))
            .budget
            .expect("budget");
        let cost = &budget.quota.cost_micros;
        assert_eq!(cost.consumed, Some(7_000));
        assert_eq!(cost.limit, None);
        assert_eq!(cost.display, "7000/—");
        assert!(cost.reason.is_none(), "全量口径无标注: {cost:?}");
    }

    #[test]
    fn quota_probe_failure_is_a_reason_not_a_failed_frame() {
        let probe: SessionQuotaProbe = Arc::new(|_session| Err("row missing".to_string()));
        let source =
            ProductionSelfStatusSource::new(Vec::new(), false, None).with_quota_probe(probe);
        let budget = source
            .snapshot_in_session(Some(SessionId::new()))
            .budget
            .expect("budget");
        assert_eq!(budget.quota.tokens.consumed, None);
        assert!(budget
            .quota
            .tokens
            .reason
            .as_deref()
            .unwrap()
            .contains("row missing"));
    }

    #[test]
    fn a_missing_quota_probe_reports_the_missing_interface_explicitly() {
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_budget(budget_status_from_configured(8, 16, Some(10), None, None));
        let budget = source
            .snapshot_in_session(Some(SessionId::new()))
            .budget
            .expect("budget");
        assert_eq!(budget.quota.tokens.limit, Some(10));
        assert_eq!(budget.quota.tokens.consumed, None);
        assert_eq!(
            budget.quota.tokens.reason.as_deref(),
            Some(DATA_PROBE_NOT_WIRED)
        );
        assert_eq!(budget.quota.cost_micros.consumed, None);
    }

    #[test]
    fn the_model_profile_reaches_the_self_report_snapshot() {
        use apeireth_core::model_profile::{ModelProfileTable, BASELINE_CONTEXT_WINDOW_FLOOR};
        let table = ModelProfileTable::baseline();
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_model_profile(table.resolve("MiniMax-M3").into_owned());
        let snapshot = source.snapshot();
        let profile = snapshot.model.expect("model profile must arrive");
        assert_eq!(profile.model_id, "MiniMax-M3");
        assert_eq!(profile.context_window, Some(BASELINE_CONTEXT_WINDOW_FLOOR));
        assert!(
            profile.input_price_micros.is_none(),
            "基线单价未提供 (绝不编价)"
        );
        assert!(snapshot.model_reason.is_none());
    }

    #[test]
    fn an_unknown_model_profile_stays_an_explicit_unknown_row() {
        use apeireth_core::model_profile::ModelProfileTable;
        // 未知模型注入显式 unknown 行 (不是 None): 事实如实未提供, 不编造。
        let table = ModelProfileTable::baseline();
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_model_profile(table.resolve("never-heard-of-it").into_owned());
        let snapshot = source.snapshot();
        let profile = snapshot.model.expect("unknown row still arrives");
        assert!(profile.is_unknown());
        assert_eq!(profile.model_id, "never-heard-of-it");
    }

    #[test]
    fn a_missing_model_profile_reports_explicit_null_with_a_reason() {
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None);
        let snapshot = source.snapshot();
        assert!(snapshot.model.is_none());
        assert!(snapshot
            .model_reason
            .as_deref()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED));
    }

    /// V4-a 人格节探测口 (`self_status_persona`): 读人格层落库面自述节真值
    /// (计数 + 最后整编, 只计数不回内容)。
    #[test]
    fn persona_probe_reports_the_persona_memory_store_status_truth() {
        use apeireth_memory::{PersonaExtract, PersonaMemoryStore as _, PersonaStatement};
        let store = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let mut memory = apeireth_memory::PersonaMemory::default();
        let statement = |text: &str| PersonaStatement {
            text: text.to_string(),
            source_episode_id: "e1".to_string(),
        };
        memory.apply_extract(
            &PersonaExtract {
                core_beliefs: vec![statement("我相信先验证再叙事。")],
                preferences: vec![statement("我喜欢清晨整理记忆。")],
                boundaries: vec![statement("我绝不静默改用户数据。")],
                narrative_lines: Vec::new(),
            },
            1_728_300_000,
        );
        store.save(&memory).expect("persona save");

        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_persona_probe(persona_status_probe(store));
        let snapshot = source.snapshot();
        let persona = snapshot.persona.expect("persona section must arrive");
        assert_eq!(persona.core_beliefs, 1);
        assert_eq!(persona.boundaries, 1);
        assert_eq!(persona.preferences, 1);
        assert_eq!(persona.tensions, 0, "无同核反极 = 无张力 (不编数)");
        assert_eq!(persona.last_consolidation_at, Some(1_728_300_000));
        assert!(persona.reason.is_none());
        assert!(snapshot.persona_reason.is_none());

        // 尚未整编的落库面: 计数如实为 0, 最后整编显式 null (不编时间)。
        let empty = Arc::new(apeireth_memory::InMemoryPersonaMemoryStore::new());
        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_persona_probe(persona_status_probe(empty));
        let snapshot = source.snapshot();
        let persona = snapshot.persona.expect("persona section must arrive");
        assert_eq!(persona.core_beliefs, 0);
        assert_eq!(persona.tensions, 0);
        assert_eq!(persona.last_consolidation_at, None);
    }

    /// 人格探测口读失败 = 原因不是整帧失败 (0 装诚实: 显式 null + 真因)。
    #[test]
    fn persona_probe_failure_is_a_reason_not_a_failed_frame() {
        struct BrokenPersonaStore;

        impl apeireth_memory::PersonaMemoryStore for BrokenPersonaStore {
            fn load(&self) -> apeireth_memory::MemoryResult<apeireth_memory::PersonaMemory> {
                Err(apeireth_memory::MemoryError::Invalid(
                    "persona table missing".to_string(),
                ))
            }

            fn save(
                &self,
                _memory: &apeireth_memory::PersonaMemory,
            ) -> apeireth_memory::MemoryResult<()> {
                Err(apeireth_memory::MemoryError::Invalid(
                    "persona table missing".to_string(),
                ))
            }

            fn status(
                &self,
            ) -> apeireth_memory::MemoryResult<apeireth_memory::PersonaStatusSection> {
                Err(apeireth_memory::MemoryError::Invalid(
                    "persona table missing".to_string(),
                ))
            }
        }

        let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_persona_probe(persona_status_probe(Arc::new(BrokenPersonaStore)));
        let snapshot = source.snapshot();
        assert!(snapshot.persona.is_none());
        assert!(
            snapshot
                .persona_reason
                .as_deref()
                .unwrap()
                .contains("persona table missing"),
            "{:?}",
            snapshot.persona_reason
        );
    }

    /// 人格探测口缺位 = 显式「来源未接线」, 不是编造的 0。
    #[test]
    fn a_missing_persona_probe_reports_the_missing_interface_explicitly() {
        let snapshot = ProductionSelfStatusSource::new(Vec::new(), false, None).snapshot();
        assert!(snapshot.persona.is_none());
        assert!(
            snapshot
                .persona_reason
                .as_deref()
                .unwrap()
                .contains(DATA_PROBE_NOT_WIRED),
            "{:?}",
            snapshot.persona_reason
        );
    }
}
