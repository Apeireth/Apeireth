//! 双引擎关怀判定面: 规则引擎 (确定性兜底) + 模型情境引擎 (增色) + 打扰纪律。
//!
//! # 双引擎架构 (规则兜底 + 模型情境双轨)
//!
//! ```text
//! care_rules.json (StoredDoc 拒开)
//!   │  四类触发器: inactive / daily_moment / anniversary / interval
//!   ▼
//! 规则引擎 (确定性, 时钟可注入)
//!   │  到点规则 → 关怀候选 (素材 + 时机)  ←—— 兜底保证不漏
//!   ▼
//! 打扰纪律 (先立规矩再谈温度, 顺序固定)
//!   │  ① 勿扰窗口 ② 专注抑制 ③ 频率上限 ④ 关怀预算 (同账本接缝)
//!   ▼
//! 发起意愿判定 (显式判定面, 沿 initiative 语义: 判「该不该说 / 说什么」)
//!   │  ├─ RuleFallbackJudge   —— 无模型配置: 纯规则照发 (规则兜底)
//!   │  └─ ModelContextJudge   —— 候选 + 近期记忆素材 → 结构化判定
//!   │        (判定失败/解析失败 → 回退规则判定, 不漏发不乱发)
//!   ▼
//! 关怀回合 (标准回合输入 → 走标准回合, 生成的关怀消息进会话流 + IM 推送缝)
//! ```
//!
//! # 0 装诚实
//!
//! - 判定面**不做隐式生成**: 模型侧只输出结构化判定 (`speak` / `focus` /
//!   `reason`), 不在这里生成消息正文 —— 消息由标准回合生成 (零新消息通道)。
//! - 无模型配置 = [`RuleFallbackJudge`] 纯规则照发; 模型判定失败/解析失败 =
//!   回退规则判定并在判定理由里如实注明真因。
//! - 常驻心跳生产者已交付在 `canonical::heartbeat` 调度缝 + runtime-assembly
//!   装配 (`CareHeartbeatProducer`, 开关 `APEIRETH_ENABLE_CARE_HEARTBEAT`
//!   注册即闸默认关); 常驻宿主的启动注入点 (presence `try_take_initiative`
//!   同源闸 → [`CareInitiativeGate`]) 已在常驻宿主接线 (cli 组合根
//!   `care_heartbeat` 模块, 2026-10-17 随行); 未注入 = 显式不发起限量
//!   的单内核档 (见 [`AlwaysTakeInitiative`])。
//! - W3.1 生物机制候选 (仪式 / 重聚) 的纪律管道入口
//!   [`CareEngine::poll_bio_candidate`] 已实现并有测试; 产候选面由
//!   [`super::CareConfig::bio_candidates_enabled`] 总开关控制, **默认关**
//!   (关闭时零发起零记账); runtime-assembly 装配待 W3.2。

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::emergence::{day_key_from_epoch_ms, minutes_of_day_from_epoch_ms, Boundaries};

use super::reminder::CareTier;
use super::rules::{CareRecord, CareRule, CareRulesBody, CareStore, CareStoreError, CareTrigger};
use super::CareConfig;

/// 关怀时钟 (可注入: 生产用系统时钟, 测试用冻结时钟)。
pub trait CareClock: Send + Sync {
    /// 当前时刻 (epoch 毫秒)。
    fn now_ms(&self) -> i64;
}

/// 系统时钟 (生产默认)。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemCareClock;

impl CareClock for SystemCareClock {
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0)
    }
}

/// 冻结时钟 (测试可注入可拨动)。
#[derive(Debug)]
pub struct FrozenCareClock {
    at_ms: AtomicI64,
}

impl FrozenCareClock {
    /// 冻结在给定时刻。
    pub fn new(at_ms: i64) -> Self {
        Self {
            at_ms: AtomicI64::new(at_ms),
        }
    }

    /// 拨到给定时刻。
    pub fn set(&self, at_ms: i64) {
        self.at_ms.store(at_ms, Ordering::SeqCst);
    }

    /// 前进给定毫秒数。
    pub fn advance(&self, delta_ms: i64) {
        self.at_ms.fetch_add(delta_ms, Ordering::SeqCst);
    }
}

impl CareClock for FrozenCareClock {
    fn now_ms(&self) -> i64 {
        self.at_ms.load(Ordering::SeqCst)
    }
}

/// 关怀候选 (每条到点规则 / 提醒信号 → 素材 + 时机)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareCandidate {
    /// 触发的规则 id (提醒/告警信号 = 信号稳定身份)。
    pub rule_id: String,
    /// 触发器语义描述 (时机的可读形态)。
    pub trigger: String,
    /// 时机: 到点时刻 (epoch 毫秒)。
    pub due_ms: i64,
    /// 素材: 关怀内容素材 (规则自带 / 信号建议语)。
    pub material: String,
    /// 来源档 (规则 / 提醒信号 / 守护告警): 打扰纪律分档 + 守护告警标注同源。
    pub tier: CareTier,
}

impl CareCandidate {
    fn from_rule(rule: &CareRule, due_ms: i64) -> Self {
        Self {
            rule_id: rule.id.clone(),
            trigger: rule.trigger.describe(),
            due_ms,
            material: rule.material.clone(),
            tier: CareTier::Rule,
        }
    }
}

/// 发起意愿判定档 (显式留痕: 谁判的)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareJudgeMode {
    /// 规则兜底档: 无模型配置, 规则到点即发。
    RuleFallback,
    /// 模型情境档: 候选 + 近期记忆素材 → 结构化判定。
    ModelContext,
}

impl CareJudgeMode {
    /// 稳定标签 (台账 / 判定载荷共用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::RuleFallback => "rule_fallback",
            Self::ModelContext => "model_context",
        }
    }
}

/// 发起意愿判定 (显式判定面, 沿 initiative 语义: 判「该不该说 / 说什么」,
/// **不做隐式生成** —— 消息正文由标准回合生成)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareVerdict {
    /// 该不该说 (显式判定)。
    pub speak: bool,
    /// 说什么: 内容方向 (`None` = 用规则素材)。
    pub focus: Option<String>,
    /// 判定理由 (诚实留痕, 不编造)。
    pub reason: String,
    /// 判定档。
    pub mode: CareJudgeMode,
}

impl CareVerdict {
    /// 规则兜底判定: 到点即说, 内容方向 = 规则素材。
    pub fn rule_fallback(material: &str) -> Self {
        Self {
            speak: true,
            focus: Some(material.to_string()),
            reason: "规则兜底: 触发器到点即发, 不漏关怀".to_string(),
            mode: CareJudgeMode::RuleFallback,
        }
    }

    /// 从模型输出的结构化判定解析 (闭合合同: `speak` / `focus` / `reason`)。
    ///
    /// 解析失败即失败 (不猜测、不隐式生成), 由调用方回退规则判定。
    pub fn from_structured_json(raw: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|error| format!("verdict is not JSON: {error}"))?;
        let object = value
            .as_object()
            .ok_or_else(|| "verdict must be a JSON object".to_string())?;
        let speak = object
            .get("speak")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| "verdict.speak must be a boolean".to_string())?;
        let focus = match object.get("focus") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(text)) if !text.trim().is_empty() => Some(text.clone()),
            Some(_) => return Err("verdict.focus must be a non-empty string or null".to_string()),
        };
        let reason = object
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| "verdict.reason must be a non-empty string".to_string())?
            .to_string();
        Ok(Self {
            speak,
            focus,
            reason,
            mode: CareJudgeMode::ModelContext,
        })
    }
}

/// 判定输入: 候选 + 近期记忆素材 (情境引擎的增色输入; 空 = 纯规则)。
#[derive(Debug, Clone)]
pub struct CareJudgmentInput {
    /// 关怀候选 (素材 + 时机)。
    pub candidate: CareCandidate,
    /// 近期记忆素材 (组装根注入的只读摘录; 不是本模块编造的)。
    pub recent_material: Vec<String>,
}

impl CareJudgmentInput {
    /// 显式判定请求文本 (模型侧只回结构化判定, 不生成消息正文)。
    pub fn to_prompt(&self) -> String {
        let annotation = self
            .candidate
            .tier
            .annotation()
            .map(|mark| format!(", 标注: {mark}"))
            .unwrap_or_default();
        let mut prompt = format!(
            "请判定这次主动关怀该不该发起, 只输出结构化判定 JSON ({{\"speak\": bool, \"focus\": \
             string|null, \"reason\": string}}), 不要生成消息正文。\n关怀候选: 规则 {} ({}, 来源档 \
             {}{})\n素材: {}",
            self.candidate.rule_id,
            self.candidate.trigger,
            self.candidate.tier.label(),
            annotation,
            self.candidate.material
        );
        if !self.recent_material.is_empty() {
            prompt.push_str("\n近期记忆素材:");
            for item in &self.recent_material {
                prompt.push_str("\n- ");
                prompt.push_str(item);
            }
        }
        prompt
    }
}

/// 发起意愿判定面 (显式判定, 沿 initiative 语义)。
pub trait CareJudge: Send + Sync {
    /// 判定「该不该说 / 说什么」。
    fn judge(&self, input: &CareJudgmentInput) -> CareVerdict;
}

/// 规则兜底判定器 (无模型配置的默认档: 规则到点即发)。
#[derive(Debug, Clone, Copy, Default)]
pub struct RuleFallbackJudge;

impl CareJudge for RuleFallbackJudge {
    fn judge(&self, input: &CareJudgmentInput) -> CareVerdict {
        CareVerdict::rule_fallback(&input.candidate.material)
    }
}

/// 模型判定端口 (模型情境引擎的调用缝: 组装根接真模型, 测试接脚本端口)。
///
/// 合同: 返回**结构化判定 JSON 文本** (见
/// [`CareVerdict::from_structured_json`]); `Err` = 判定调用失败 (真因字符串)。
pub trait CareModelPort: Send + Sync {
    /// 发起意愿判定 (候选 + 近期记忆素材 → 结构化判定文本)。
    fn verdict(&self, input: &CareJudgmentInput) -> Result<String, String>;
}

/// 模型情境判定器 (双引擎的第二轨: 增色)。
///
/// 判定失败 / 解析失败 → **回退规则判定** (兜底保证不漏), 并在判定理由里
/// 如实注明回退真因。
pub struct ModelContextJudge {
    port: Arc<dyn CareModelPort>,
}

impl ModelContextJudge {
    /// 绑定模型判定端口。
    pub fn new(port: Arc<dyn CareModelPort>) -> Self {
        Self { port }
    }
}

impl CareJudge for ModelContextJudge {
    fn judge(&self, input: &CareJudgmentInput) -> CareVerdict {
        match self.port.verdict(input) {
            Ok(raw) => match CareVerdict::from_structured_json(&raw) {
                Ok(verdict) => verdict,
                Err(reason) => fallback_with_note(input, format!("判定解析失败: {reason}")),
            },
            Err(error) => fallback_with_note(input, format!("判定调用失败: {error}")),
        }
    }
}

fn fallback_with_note(input: &CareJudgmentInput, note: String) -> CareVerdict {
    let mut verdict = CareVerdict::rule_fallback(&input.candidate.material);
    verdict.reason = format!("{}; 回退规则判定 ({note})", verdict.reason);
    verdict
}

/// 关怀预算判定 (关怀回合照常计费的前置问询)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CareBudgetVerdict {
    /// 预算还烧得起 (计费随标准回合落同账本)。
    Admitted,
    /// 预算已耗尽 (与三维烧钱闸同账本: 关怀不给免费通道)。
    Exhausted {
        /// 耗尽真因 (维度 / 上限 / 已耗, 如实)。
        reason: String,
    },
}

/// 关怀预算闸 (与三维烧钱闸**同账本接缝**): 关怀回合照常计费 —— 本闸只做
/// 「还烧得起吗」的前置问询, 真正的记账随标准回合落会话配额账本 (三维
/// tokens / tool_steps / cost_micros 同一本账, 关怀回合无免费通道)。
pub trait CareBudgetGate: Send + Sync {
    /// 前置问询: 这次关怀回合烧得起吗。
    fn admit(&self, at_ms: i64) -> CareBudgetVerdict;
}

/// 恒放行预算闸 (默认档: 计费照常, 不做前置拦截)。
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysAdmitBudget;

impl CareBudgetGate for AlwaysAdmitBudget {
    fn admit(&self, _at_ms: i64) -> CareBudgetVerdict {
        CareBudgetVerdict::Admitted
    }
}

/// 闭包预算闸 (组装根把会话配额账本的「烧得起吗」问询接进本闸)。
pub struct FnBudgetGate<F> {
    admit: F,
}

impl<F> FnBudgetGate<F>
where
    F: Fn(i64) -> CareBudgetVerdict + Send + Sync,
{
    /// 绑定问询闭包。
    pub fn new(admit: F) -> Self {
        Self { admit }
    }
}

impl<F> CareBudgetGate for FnBudgetGate<F>
where
    F: Fn(i64) -> CareBudgetVerdict + Send + Sync,
{
    fn admit(&self, at_ms: i64) -> CareBudgetVerdict {
        (self.admit)(at_ms)
    }
}

/// 发起闸 (try_take_initiative 同源接缝)。
///
/// 与常驻进程 presence 面的发起限量 (`try_take_initiative`) **同一闸源**:
/// 关怀发起与其他 initiative 生产者共用同一份防唠叨预算 (宁少勿多)。闸只在
/// 「有到点候选且打扰纪律四件已放行」时被问询 (空转 poll零消耗); 闸拒绝 =
/// [`CareHold::InitiativeGate`] 拦下 —— **候选不丢**, 窗口滑过重试。
pub trait CareInitiativeGate: Send + Sync {
    /// 尝试占用一个发起额度: `true` = 放行发起; `false` = 本轮不发起。
    fn try_take_initiative(&self, now_ms: i64) -> bool;
}

/// 恒放行发起闸 (内核单用的缺省档)。
///
/// **0 装诚实**: 生产装配必须注入 presence 同源闸 (见 [`FnInitiativeGate`]);
/// 本缺省档只保证内核单独可用 (测试/工具面), 不代表生产没有发起限量。
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysTakeInitiative;

impl CareInitiativeGate for AlwaysTakeInitiative {
    fn try_take_initiative(&self, _now_ms: i64) -> bool {
        true
    }
}

/// 闭包发起闸 (组装根把 presence 的 `try_take_initiative` 接进本闸)。
pub struct FnInitiativeGate<F> {
    take: F,
}

impl<F> FnInitiativeGate<F>
where
    F: Fn(i64) -> bool + Send + Sync,
{
    /// 绑定问询闭包。
    pub fn new(take: F) -> Self {
        Self { take }
    }
}

impl<F> CareInitiativeGate for FnInitiativeGate<F>
where
    F: Fn(i64) -> bool + Send + Sync,
{
    fn try_take_initiative(&self, now_ms: i64) -> bool {
        (self.take)(now_ms)
    }
}

/// 近期记忆素材源 (模型情境引擎的增色输入缝; 缺省 = 纯规则)。
pub trait CareMaterialSource: Send + Sync {
    /// 取该候选的近期记忆素材 (只读摘录, 由组装根注入来源)。
    fn recent_material(&self, candidate: &CareCandidate) -> Vec<String>;
}

/// 空素材源 (纯规则档: 情境输入为空)。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRecentMaterial;

impl CareMaterialSource for NoRecentMaterial {
    fn recent_material(&self, _candidate: &CareCandidate) -> Vec<String> {
        Vec::new()
    }
}

/// 打扰纪律拦下的原因 (先立规矩再谈温度; 拦下不丢, 候选保持到点可重试)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CareHold {
    /// 勿扰窗口内 (安静时段不打扰)。
    QuietWindow,
    /// 连续交互期 (专注抑制: 用户正在对话, 不插话)。
    Focus,
    /// 频率上限 (滚动窗口档 / 日档任一已满, 默认 4h≤1 次 + 日≤5 次)。
    FrequencyCap,
    /// 关怀预算已耗尽 (同账本接缝: 烧不起就不发)。
    Budget,
    /// 发起闸拦下 (try_take_initiative 同源闸拒绝: 防唠叨发起限量已用尽;
    /// 候选不丢, 窗口滑过重试)。
    InitiativeGate,
}

impl CareHold {
    /// 稳定标签 (留痕 / 测试用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::QuietWindow => "quiet_window",
            Self::Focus => "focus",
            Self::FrequencyCap => "frequency_cap",
            Self::Budget => "budget",
            Self::InitiativeGate => "initiative_gate",
        }
    }
}

/// 一次心跳的关怀判定结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarePoll {
    /// 发起: 候选经判定该说, 标准回合输入已备好 (走标准回合)。
    Initiating(Box<CareInitiative>),
    /// 显式判定说不 (该不该说 = 不说; 台账留痕, 当次到点消费掉)。
    Declined {
        /// 规则 id。
        rule_id: String,
        /// 判定 (含理由)。
        verdict: CareVerdict,
    },
    /// 打扰纪律/发起闸拦下 (候选拦下不丢, 下次心跳重试)。
    Held {
        /// 规则 id。
        rule_id: String,
        /// 拦下原因。
        hold: CareHold,
    },
    /// 无到点候选。
    Idle,
}

/// 关怀发起 (候选 + 判定 + 时刻): 输出面 = 标准回合输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareInitiative {
    /// 关怀候选 (素材 + 时机)。
    pub candidate: CareCandidate,
    /// 发起意愿判定 (显式)。
    pub verdict: CareVerdict,
    /// 发起时刻 (epoch 毫秒)。
    pub at_ms: i64,
}

impl CareInitiative {
    /// 标准回合输入 (零新消息通道: 关怀发起 = 走标准回合, 生成的关怀消息
    /// 随标准回合进会话流, IM 推送走既有推送缝)。
    ///
    /// 守护告警档 (破勿扰发起) **必带「守护告警」标注** —— 人看得见它为什么
    /// 破窗 (打扰纪律守护例外档的诚实留痕)。
    pub fn to_turn_input(&self) -> String {
        let focus = self
            .verdict
            .focus
            .as_deref()
            .unwrap_or(self.candidate.material.as_str());
        let body = format!(
            "[care:{rule}] 时机: {trigger} (到点 {due}); 素材: {material}; 发起意愿判定: \
             {mode} 该说 (理由: {reason}); 说什么方向: {focus}. 请以一句关怀消息回应这次主动关怀。",
            rule = self.candidate.rule_id,
            trigger = self.candidate.trigger,
            due = self.candidate.due_ms,
            material = self.candidate.material,
            mode = self.verdict.mode.label(),
            reason = self.verdict.reason,
            focus = focus,
        );
        match self.candidate.tier.annotation() {
            Some(mark) => format!("[{mark}] {body}"),
            None => body,
        }
    }
}

/// 双引擎关怀内核 (规则引擎 + 判定面 + 打扰纪律)。
pub struct CareEngine {
    store: CareStore,
    clock: Arc<dyn CareClock>,
    judge: Arc<dyn CareJudge>,
    material: Arc<dyn CareMaterialSource>,
    budget: Arc<dyn CareBudgetGate>,
    initiative: Arc<dyn CareInitiativeGate>,
    care_config: CareConfig,
    last_interaction_ms: Option<i64>,
}

impl CareEngine {
    /// 组装内核: 规则表 + 时钟 + 判定器 (其余取纯规则缺省档)。
    pub fn new(store: CareStore, clock: Arc<dyn CareClock>, judge: Arc<dyn CareJudge>) -> Self {
        Self {
            store,
            clock,
            judge,
            material: Arc::new(NoRecentMaterial),
            budget: Arc::new(AlwaysAdmitBudget),
            initiative: Arc::new(AlwaysTakeInitiative),
            care_config: CareConfig::default(),
            last_interaction_ms: None,
        }
    }

    /// 注入近期记忆素材源 (模型情境引擎的增色输入)。
    #[must_use]
    pub fn with_material_source(mut self, source: Arc<dyn CareMaterialSource>) -> Self {
        self.material = source;
        self
    }

    /// 注入关怀预算闸 (与三维烧钱闸同账本接缝)。
    #[must_use]
    pub fn with_budget_gate(mut self, gate: Arc<dyn CareBudgetGate>) -> Self {
        self.budget = gate;
        self
    }

    /// 注入发起闸 (try_take_initiative 同源接缝: 与其他 initiative 生产者
    /// 共用同一份防唠叨预算)。
    #[must_use]
    pub fn with_initiative_gate(mut self, gate: Arc<dyn CareInitiativeGate>) -> Self {
        self.initiative = gate;
        self
    }

    /// 注入 care 接线配置 (生物机制候选开关; 默认关, 见 [`CareConfig`])。
    #[must_use]
    pub fn with_care_config(mut self, config: CareConfig) -> Self {
        self.care_config = config;
        self
    }

    /// 当前接线配置 (含生物机制候选开关)。
    pub fn care_config(&self) -> CareConfig {
        self.care_config
    }

    /// 生物机制候选 (仪式 / 重聚) 是否启用 (默认 `false`)。
    pub fn bio_candidates_enabled(&self) -> bool {
        self.care_config.bio_candidates_enabled()
    }

    /// 规则表存储 (自述探测口同源读它)。
    pub fn store(&self) -> &CareStore {
        &self.store
    }

    /// 进程内观察到的上次交互时刻 (久未联系锚 + 专注抑制输入)。
    pub fn last_interaction_ms(&self) -> Option<i64> {
        self.last_interaction_ms
    }

    /// 观察一次用户交互 (喂给久未联系锚点 + 专注抑制)。
    ///
    /// **0 装诚实**: 交互观察是进程内值; 重启后久未联系类规则以规则创建时刻
    /// 为锚 (如实在规则面文档注明)。
    pub fn observe_interaction(&mut self, at_ms: i64) {
        self.last_interaction_ms = Some(
            self.last_interaction_ms
                .map_or(at_ms, |last| last.max(at_ms)),
        );
    }

    /// 每次心跳调用一次: 取第一个到点规则 → 打扰纪律 → 发起闸 → 发起意愿判定。
    ///
    /// 顺序固定 (先立规矩再谈温度): 勿扰窗口 → 专注抑制 → 频率上限 →
    /// 关怀预算 → 发起闸 (try_take_initiative 同源) → 判定。纪律/闸拦下 =
    /// [`CarePoll::Held`] (候选不丢, 重试); 判定说不 = [`CarePoll::Declined`]
    /// (当次到点消费掉, 台账留痕); 判定该说 = [`CarePoll::Initiating`]
    /// (发起留痕 + 台账)。发起闸拦下时**零判定零记账** —— 纯规则轮询路径
    /// 不碰模型 (poll 本身零模型调用)。
    ///
    /// 提醒/告警信号走同一管道 (同源纪律): [`Self::poll_reminder`]。
    pub fn poll(&mut self) -> Result<CarePoll, CareStoreError> {
        let now_ms = self.now_ms();
        let body = self.store.list()?;
        let Some(rule) = self.first_due_rule(&body, now_ms) else {
            return Ok(CarePoll::Idle);
        };

        // 打扰纪律 (四件, 顺序固定; 规则档 = 无守护例外)。
        if let Some(hold) = self.discipline_hold_for(&body, now_ms, CareTier::Rule) {
            return Ok(CarePoll::Held {
                rule_id: rule.id.clone(),
                hold,
            });
        }
        let candidate = CareCandidate::from_rule(rule, now_ms);
        self.admit_candidate(now_ms, candidate)
    }

    /// 生物机制候选 (仪式 / 重聚) 的统一裁决入口: 与规则 / 提醒**同一条**纪律管道。
    ///
    /// - **开关关 (默认)**: 直接 [`CarePoll::Idle`] —— defense-in-depth, 即使调用方
    ///   绕过产候选面的开关, 内核也不发起 (零判定零记账); 既有 care 行为不变;
    /// - **开关开**: 勿扰窗口 → 专注抑制 → 频率上限 → 关怀预算 → 发起闸
    ///   (try_take_initiative 同源) → 发起意愿判定 → **信号档台账**
    ///   (`kind` = `ritual` / `reunion`; 不碰规则 `fired_at` 锚点, 不伪造规则);
    /// - 纪律 / 发起闸拦下 = [`CarePoll::Held`] (候选不丢, 重试); 判定说不 =
    ///   [`CarePoll::Declined`]; 判定该说 = [`CarePoll::Initiating`]。
    ///
    /// 非生物档候选 (规则 / 提醒 / 守护告警) 各有自己的入口, 串档显式拒绝
    /// ([`CareStoreError::Refused`]), 不静默改档。
    pub fn poll_bio_candidate(
        &mut self,
        candidate: &CareCandidate,
    ) -> Result<CarePoll, CareStoreError> {
        if !self.care_config.bio_candidates_enabled() {
            return Ok(CarePoll::Idle);
        }
        if !candidate.tier.is_bio() {
            return Err(CareStoreError::Refused(format!(
                "candidate tier {} is not a bio candidate (ritual/reunion expected)",
                candidate.tier.label()
            )));
        }
        let now_ms = self.now_ms();
        let body = self.store.list()?;
        if let Some(hold) = self.discipline_hold_for(&body, now_ms, candidate.tier) {
            return Ok(CarePoll::Held {
                rule_id: candidate.rule_id.clone(),
                hold,
            });
        }
        self.admit_candidate(now_ms, candidate.clone())
    }

    /// 现时刻 (epoch 毫秒; 时钟可注入)。
    pub(crate) fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    /// 纪律之后、判定之前的统一收口: 发起闸 (try_take_initiative 同源) →
    /// 发起意愿判定 → 台账留痕 (规则档随发起留痕锚点, 信号档只留台账)。
    ///
    /// 发起闸拒绝 = [`CarePoll::Held`] (候选不丢, 且零判定零记账); 判定说不 =
    /// [`CarePoll::Declined`] (当次消费掉, 台账留痕); 判定该说 =
    /// [`CarePoll::Initiating`]。守护告警档的发起**必带「守护告警」标注**
    /// (台账 note + 标准回合输入同源)。
    pub(crate) fn admit_candidate(
        &self,
        now_ms: i64,
        candidate: CareCandidate,
    ) -> Result<CarePoll, CareStoreError> {
        // 发起闸 (try_take_initiative 同源): 纪律放行后、判定前问询 ——
        // 拒绝 = 候选不丢 (Held 重试), 且本轮零判定零记账。
        if !self.initiative.try_take_initiative(now_ms) {
            return Ok(CarePoll::Held {
                rule_id: candidate.rule_id.clone(),
                hold: CareHold::InitiativeGate,
            });
        }

        // 发起意愿判定 (显式判定面)。
        let input = CareJudgmentInput {
            recent_material: self.material.recent_material(&candidate),
            candidate,
        };
        let verdict = self.judge.judge(&input);
        let mut note = verdict.reason.clone();
        if let Some(mark) = input.candidate.tier.annotation() {
            note = format!("{mark}: {note}");
        }
        let record = CareRecord {
            rule_id: input.candidate.rule_id.clone(),
            at_ms: now_ms,
            judge: verdict.mode.label().to_string(),
            spoke: verdict.speak,
            note,
            kind: input.candidate.tier.label().to_string(),
        };
        if input.candidate.tier == CareTier::Rule {
            // 规则档: 发起留痕 (fired_at 锚点) + 台账同写。
            self.store.record_care(record, now_ms)?;
        } else {
            // 信号档 (提醒/守护告警): 只留台账 (发起留痕锚点归规则;
            // 信号去重走消费位, 见 care::reminder 模块 0 装诚实注)。
            self.store.record_signal_care(record)?;
        }
        if !verdict.speak {
            return Ok(CarePoll::Declined {
                rule_id: input.candidate.rule_id,
                verdict,
            });
        }
        Ok(CarePoll::Initiating(Box::new(CareInitiative {
            candidate: input.candidate,
            verdict,
            at_ms: now_ms,
        })))
    }

    fn first_due_rule<'body>(
        &self,
        body: &'body CareRulesBody,
        now_ms: i64,
    ) -> Option<&'body CareRule> {
        body.rules.iter().find(|rule| {
            rule.enabled
                && rule.trigger.is_due(
                    now_ms,
                    rule.created_at_ms,
                    body.last_fired(&rule.id),
                    self.last_interaction_ms,
                    body.discipline.catchup_minutes,
                )
        })
    }

    /// 打扰纪律 (同源: 规则 / 提醒 / 守护告警共用同一本 [`super::rules::CareDiscipline`])。
    ///
    /// 分档语义 (守护例外档 = 唯一例外, 可配):
    ///
    /// - **勿扰窗口**: 守护告警档默认**可破勿扰** (设计裁定: 安全告警破窗必带
    ///   「守护告警」标注, 见 [`CareInitiative::to_turn_input`]);
    ///   [`super::rules::CareDiscipline::guard_alert_breaks_quiet`] 配成 `false`
    ///   = 例外档收起, 守护告警照常被勿扰拦。提醒档 / 规则档照常被拦 (同源纪律);
    /// - **专注抑制**: 三档照常生效 (例外档只破勿扰一项, 不是全豁免);
    /// - **频率上限**: 提醒档与规则档**同账** (同源: 都计普通双档 4h≤1 + 日≤5,
    ///   守护告警不占这份预算); 守护告警走**独立上限**
    ///   ([`super::rules::CareDiscipline::guard_frequency_max`] /
    ///   [`super::rules::CareDiscipline::guard_frequency_window_hours`], 默认 1
    ///   小时 ≤ 3 条) —— 普通频率不误伤安全告警, 守护频率自己有界;
    /// - **关怀预算**: 三档照常生效 (关怀回合照常计费)。
    pub(crate) fn discipline_hold_for(
        &self,
        body: &CareRulesBody,
        now_ms: i64,
        tier: CareTier,
    ) -> Option<CareHold> {
        let discipline = &body.discipline;
        // ① 勿扰窗口 (与 initiative 语义的安静窗口同口径; 守护例外档可破窗)。
        let guard_breaks_quiet = tier.is_guard_alert() && discipline.guard_alert_breaks_quiet;
        if !guard_breaks_quiet {
            let boundaries = Boundaries {
                quiet_start_minutes: discipline.quiet_start_minutes,
                quiet_end_minutes: discipline.quiet_end_minutes,
                ..Boundaries::default()
            };
            if boundaries.in_quiet_window(minutes_of_day_from_epoch_ms(now_ms)) {
                return Some(CareHold::QuietWindow);
            }
        }
        // ② 专注抑制: 距上次交互不足窗口 = 连续交互期, 不插话 (三档同源)。
        if let Some(last) = self.last_interaction_ms {
            if now_ms.saturating_sub(last) < discipline.focus_gap_ms() {
                return Some(CareHold::Focus);
            }
        }
        // ③ 频率上限 (分档): 守护告警独立上限; 规则 / 提醒同账双档。
        if tier.is_guard_alert() {
            let window_start = now_ms - discipline.guard_frequency_window_ms();
            let guard_sent = body
                .history
                .iter()
                .filter(|record| {
                    record.spoke
                        && record.kind == CareTier::GuardAlert.label()
                        && record.at_ms >= window_start
                })
                .count();
            if guard_sent >= usize::try_from(discipline.guard_frequency_max).unwrap_or(usize::MAX) {
                return Some(CareHold::FrequencyCap);
            }
        } else {
            // 滚动窗口档 + 日档 (设计裁定 R1⑬: 4h≤1 次 + 日≤5 次); 守护告警
            // 台账**不占**这份预算 (独立上限, 不误伤普通关怀频率)。
            let window_start = now_ms - discipline.frequency_window_ms();
            let sent = body
                .history
                .iter()
                .filter(|record| {
                    record.spoke
                        && record.kind != CareTier::GuardAlert.label()
                        && record.at_ms >= window_start
                })
                .count();
            if sent >= usize::try_from(discipline.frequency_max).unwrap_or(usize::MAX) {
                return Some(CareHold::FrequencyCap);
            }
            let today = day_key_from_epoch_ms(now_ms);
            let sent_today = body
                .history
                .iter()
                .filter(|record| {
                    record.spoke
                        && record.kind != CareTier::GuardAlert.label()
                        && day_key_from_epoch_ms(record.at_ms) == today
                })
                .count();
            if sent_today >= usize::try_from(discipline.daily_max).unwrap_or(usize::MAX) {
                return Some(CareHold::FrequencyCap);
            }
        }
        // ④ 关怀预算 (同账本接缝: 关怀回合照常计费, 烧不起就不发)。
        if let CareBudgetVerdict::Exhausted { .. } = self.budget.admit(now_ms) {
            return Some(CareHold::Budget);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::care::rules::{CareDiscipline, CareRule};
    use crate::care::CareConfig;

    fn store_with(rule: &CareRule, discipline: CareDiscipline) -> (tempfile::TempDir, CareStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = CareStore::new(dir.path());
        let mut body = store.list().expect("empty body");
        body.discipline = discipline;
        body.rules.push(rule.clone());
        store.save(&body).expect("save");
        (dir, store)
    }

    fn daily_rule(minutes_of_day: u32) -> CareRule {
        CareRule {
            id: "daily".to_string(),
            trigger: CareTrigger::DailyMoment { minutes_of_day },
            material: "问候早安".to_string(),
            enabled: true,
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn frozen_clock_is_injectable_and_settable() {
        let clock = FrozenCareClock::new(1_000);
        assert_eq!(clock.now_ms(), 1_000);
        clock.advance(500);
        assert_eq!(clock.now_ms(), 1_500);
        clock.set(42);
        assert_eq!(clock.now_ms(), 42);
    }

    #[test]
    fn structured_verdict_parsing_is_closed_and_strict() {
        let verdict = CareVerdict::from_structured_json(
            r#"{"speak": false, "focus": null, "reason": "他正在忙"}"#,
        )
        .expect("valid verdict");
        assert!(!verdict.speak);
        assert_eq!(verdict.focus, None);
        assert_eq!(verdict.mode, CareJudgeMode::ModelContext);
        assert!(
            CareVerdict::from_structured_json("{}").is_err(),
            "缺字段拒绝"
        );
        assert!(
            CareVerdict::from_structured_json(r#"{"speak": true, "reason": ""}"#).is_err(),
            "空理由拒绝"
        );
        assert!(
            CareVerdict::from_structured_json("不是 JSON").is_err(),
            "非 JSON 拒绝 (不猜测不隐式生成)"
        );
    }

    #[test]
    fn model_context_judge_falls_back_to_the_rule_verdict_on_failure() {
        struct BrokenPort;
        impl CareModelPort for BrokenPort {
            fn verdict(&self, _input: &CareJudgmentInput) -> Result<String, String> {
                Err("模型不可用".to_string())
            }
        }
        struct GarbledPort;
        impl CareModelPort for GarbledPort {
            fn verdict(&self, _input: &CareJudgmentInput) -> Result<String, String> {
                Ok("{ 没有判定合同 }".to_string())
            }
        }
        let input = CareJudgmentInput {
            candidate: CareCandidate {
                rule_id: "daily".to_string(),
                trigger: "每日作息时刻 08:30".to_string(),
                due_ms: 1_700_000_000_000,
                material: "问候早安".to_string(),
                tier: CareTier::Rule,
            },
            recent_material: Vec::new(),
        };
        for judge in [
            ModelContextJudge::new(Arc::new(BrokenPort)),
            ModelContextJudge::new(Arc::new(GarbledPort)),
        ] {
            let verdict = judge.judge(&input);
            assert!(verdict.speak, "回退规则判定: 兜底保证不漏");
            assert_eq!(verdict.mode, CareJudgeMode::RuleFallback);
            assert!(
                verdict.reason.contains("回退规则判定"),
                "{}",
                verdict.reason
            );
        }
    }

    fn open_discipline() -> CareDiscipline {
        CareDiscipline {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            ..CareDiscipline::default()
        }
    }

    fn due_interval_rule() -> CareRule {
        CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: 1_700_000_000_000,
        }
    }

    /// 发起闸 (try_take_initiative 同源) 拦下 = Held: 候选不丢 (不消耗发起锚点),
    /// 零判定零记账; 闸放行才发起。
    #[test]
    fn the_initiative_gate_holds_without_losing_the_candidate_or_recording_it() {
        let (_dir, store) = store_with(&due_interval_rule(), open_discipline());
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000));
        let consulted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let admitted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let gate = FnInitiativeGate::new({
            let consulted = Arc::clone(&consulted);
            let admitted = Arc::clone(&admitted);
            move |_at_ms| {
                consulted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                admitted.load(std::sync::atomic::Ordering::SeqCst)
            }
        });
        let mut engine = CareEngine::new(store, clock.clone(), Arc::new(RuleFallbackJudge))
            .with_initiative_gate(Arc::new(gate));
        clock.set(1_700_000_000_000 + 3_600_000);

        assert_eq!(
            engine.poll().expect("poll"),
            CarePoll::Held {
                rule_id: "check-in".to_string(),
                hold: CareHold::InitiativeGate,
            },
            "发起闸拒绝 = 拦下 (候选拦下不丢)"
        );
        let body = engine.store().list().expect("list");
        assert!(
            body.history.is_empty(),
            "发起闸拦下不写台账: {:?}",
            body.history
        );
        assert!(
            body.fired_at.is_empty(),
            "发起锚点不消耗 (候选不丢, 窗口滑过重试): {:?}",
            body.fired_at
        );

        admitted.store(true, std::sync::atomic::Ordering::SeqCst);
        match engine.poll().expect("poll") {
            CarePoll::Initiating(initiative) => {
                assert_eq!(initiative.candidate.rule_id, "check-in");
            }
            other => panic!("闸放行后必须发起: {other:?}"),
        }
        assert_eq!(
            consulted.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "发起闸只在有到点候选且纪律放行时被问询"
        );
    }

    /// 发起闸在打扰纪律之后才被问询: 纪律拦下/无到点候选 = 零问询零消耗
    /// (防唠叨预算不被空转 poll 烧掉)。
    #[test]
    fn the_initiative_gate_is_only_consulted_after_the_discipline_passes() {
        // ① 无到点候选: Idle, 闸零问询。
        let rule = CareRule {
            created_at_ms: 1_700_000_000_000,
            ..due_interval_rule()
        };
        let (_dir, store) = store_with(&rule, open_discipline());
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000));
        let consulted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut engine = CareEngine::new(store, clock.clone(), Arc::new(RuleFallbackJudge))
            .with_initiative_gate(Arc::new(FnInitiativeGate::new({
                let consulted = Arc::clone(&consulted);
                move |_at_ms| {
                    consulted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    true
                }
            })));
        assert_eq!(engine.poll().expect("idle"), CarePoll::Idle);

        // ② 到点但勿扰窗口内: Held(QuietWindow), 闸仍零问询。
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000 + 3_600_000));
        let (_dir, store) = store_with(&due_interval_rule(), CareDiscipline::default());
        let mut engine = CareEngine::new(store, clock.clone(), Arc::new(RuleFallbackJudge))
            .with_initiative_gate(Arc::new(FnInitiativeGate::new({
                let consulted = Arc::clone(&consulted);
                move |_at_ms| {
                    consulted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    true
                }
            })));
        assert_eq!(
            engine.poll().expect("hold"),
            CarePoll::Held {
                rule_id: "check-in".to_string(),
                hold: CareHold::QuietWindow,
            }
        );
        assert_eq!(
            consulted.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "纪律拦下时发起闸零问询 (预算不被空转烧掉)"
        );
    }

    fn bio_candidate(rule_id: &str, tier: CareTier) -> CareCandidate {
        CareCandidate {
            rule_id: rule_id.to_string(),
            trigger: "生物机制候选".to_string(),
            due_ms: 1_700_000_000_000,
            material: "ritual.morning_check_in.due".to_string(),
            tier,
        }
    }

    /// 默认关: 生物机制候选入口直接 Idle, 零判定零记账 (既有 care 行为不受影响)。
    #[test]
    fn bio_candidates_are_off_by_default_and_idle_without_ledger_writes() {
        let (_dir, store) = store_with(&due_interval_rule(), open_discipline());
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000));
        let mut engine = CareEngine::new(store, clock, Arc::new(RuleFallbackJudge));
        assert!(!engine.bio_candidates_enabled());
        assert_eq!(engine.care_config(), CareConfig::default());

        let candidate = bio_candidate("ritual.morning_check_in.due", CareTier::Ritual);
        assert_eq!(
            engine.poll_bio_candidate(&candidate).expect("off = idle"),
            CarePoll::Idle
        );
        let body = engine.store().list().expect("list");
        assert!(body.history.is_empty(), "开关关零记账: {:?}", body.history);
        assert!(body.fired_at.is_empty(), "开关关零发起锚点");
    }

    /// 开关开: 生物机制候选走既有纪律管道 (频率 / 发起闸 / 判定), 台账走信号档。
    #[test]
    fn bio_candidates_use_the_same_discipline_and_signal_ledger() {
        let (_dir, store) = store_with(&due_interval_rule(), open_discipline());
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000));
        let mut engine = CareEngine::new(store, clock, Arc::new(RuleFallbackJudge))
            .with_care_config(CareConfig {
                bio_candidates_enabled: true,
            });
        assert!(engine.bio_candidates_enabled());
        let candidate = bio_candidate("reunion.days", CareTier::Reunion);

        match engine.poll_bio_candidate(&candidate).expect("poll") {
            CarePoll::Initiating(initiative) => {
                assert_eq!(initiative.candidate, candidate);
                assert_eq!(initiative.verdict.mode, CareJudgeMode::RuleFallback);
                assert_eq!(initiative.at_ms, 1_700_000_000_000);
            }
            other => panic!("开关开且纪律放行必须发起: {other:?}"),
        }
        let body = engine.store().list().expect("list");
        assert_eq!(body.history.len(), 1);
        assert_eq!(body.history[0].kind, "reunion", "信号档台账来源档");
        assert!(body.fired_at.is_empty(), "信号档不碰规则 fired_at 锚点");

        // 同一条候选再次轮询: 普通频率上限 (默认 4h≤1) 拦下, 候选不丢。
        assert_eq!(
            engine.poll_bio_candidate(&candidate).expect("poll"),
            CarePoll::Held {
                rule_id: "reunion.days".to_string(),
                hold: CareHold::FrequencyCap,
            }
        );
        assert_eq!(
            engine.store().list().expect("list").history.len(),
            1,
            "拦下不追加台账"
        );
    }

    /// 生物机制候选同样先过勿扰窗口 (默认 23:00-8:00), 拦下零记账。
    #[test]
    fn bio_candidates_are_held_by_the_quiet_window_without_recording() {
        // 1_700_000_000_000 = 22:13:20 UTC; +1h = 23:13:20 UTC, 落在默认勿扰窗内。
        let clock = Arc::new(FrozenCareClock::new(1_700_000_003_600_000));
        let (_dir, store) = store_with(&due_interval_rule(), CareDiscipline::default());
        let mut engine = CareEngine::new(store, clock, Arc::new(RuleFallbackJudge))
            .with_care_config(CareConfig {
                bio_candidates_enabled: true,
            });
        let candidate = bio_candidate("ritual.morning_check_in.due", CareTier::Ritual);
        assert_eq!(
            engine.poll_bio_candidate(&candidate).expect("hold"),
            CarePoll::Held {
                rule_id: "ritual.morning_check_in.due".to_string(),
                hold: CareHold::QuietWindow,
            }
        );
        let body = engine.store().list().expect("list");
        assert!(body.history.is_empty(), "勿扰拦下零记账");
    }

    /// 串档显式拒绝: 规则 / 提醒 / 守护告警各走自己的入口。
    #[test]
    fn bio_entry_refuses_non_bio_tiers() {
        let (_dir, store) = store_with(&due_interval_rule(), open_discipline());
        let clock = Arc::new(FrozenCareClock::new(1_700_000_000_000));
        let mut engine = CareEngine::new(store, clock, Arc::new(RuleFallbackJudge))
            .with_care_config(CareConfig {
                bio_candidates_enabled: true,
            });
        let candidate = bio_candidate("check-in", CareTier::Rule);
        let error = engine.poll_bio_candidate(&candidate).unwrap_err();
        assert!(matches!(error, CareStoreError::Refused(_)), "{error}");
        assert!(engine.store().list().expect("list").history.is_empty());
    }

    /// 既有规则轮询在默认关开关下逐字段不变 (候选 / 判定 / 台账 / 发起锚点)。
    #[test]
    fn existing_rule_poll_is_unchanged_with_the_default_off_switch() {
        const AT: i64 = 1_700_000_003_600_000;
        let (_dir, store) = store_with(&due_interval_rule(), open_discipline());
        let clock = Arc::new(FrozenCareClock::new(AT));
        let mut engine = CareEngine::new(store, clock, Arc::new(RuleFallbackJudge));
        assert!(!engine.bio_candidates_enabled());

        match engine.poll().expect("poll") {
            CarePoll::Initiating(initiative) => {
                assert_eq!(
                    initiative.candidate,
                    CareCandidate {
                        rule_id: "check-in".to_string(),
                        trigger: "间隔循环 每 1 小时".to_string(),
                        due_ms: AT,
                        material: "问问他最近怎么样".to_string(),
                        tier: CareTier::Rule,
                    },
                    "既有规则候选逐字段不变"
                );
                assert_eq!(
                    initiative.verdict,
                    CareVerdict::rule_fallback("问问他最近怎么样"),
                    "既有判定逐字段不变"
                );
                assert_eq!(initiative.at_ms, AT);
            }
            other => panic!("既有规则到点必须照常发起: {other:?}"),
        }
        let body = engine.store().list().expect("list");
        assert_eq!(body.history.len(), 1);
        assert_eq!(body.history[0].kind, "rule");
        assert_eq!(body.last_fired("check-in"), Some(AT));
    }
}
