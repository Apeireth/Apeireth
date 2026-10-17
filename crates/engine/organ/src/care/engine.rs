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
//! - 常驻心跳生产者 (定时 `poll`) **未接线**: 本模块是它调用的工具性内核
//!   (poll → 候选 → 判定 → 标准回合输入), 接线归常驻进程线。

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::emergence::{minutes_of_day_from_epoch_ms, Boundaries};

use super::rules::{CareRecord, CareRule, CareRulesBody, CareStore, CareStoreError, CareTrigger};

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

/// 关怀候选 (每条到点规则 → 素材 + 时机)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareCandidate {
    /// 触发的规则 id。
    pub rule_id: String,
    /// 触发器语义描述 (时机的可读形态)。
    pub trigger: String,
    /// 时机: 到点时刻 (epoch 毫秒)。
    pub due_ms: i64,
    /// 素材: 关怀内容素材 (规则自带)。
    pub material: String,
}

impl CareCandidate {
    fn from_rule(rule: &CareRule, due_ms: i64) -> Self {
        Self {
            rule_id: rule.id.clone(),
            trigger: rule.trigger.describe(),
            due_ms,
            material: rule.material.clone(),
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
        let mut prompt = format!(
            "请判定这次主动关怀该不该发起, 只输出结构化判定 JSON ({{\"speak\": bool, \"focus\": \
             string|null, \"reason\": string}}), 不要生成消息正文。\n关怀候选: 规则 {} ({})\n素材: \
             {}",
            self.candidate.rule_id, self.candidate.trigger, self.candidate.material
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
    /// 频率上限 (每 N 小时最多 M 次, 已满)。
    FrequencyCap,
    /// 关怀预算已耗尽 (同账本接缝: 烧不起就不发)。
    Budget,
}

impl CareHold {
    /// 稳定标签 (留痕 / 测试用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::QuietWindow => "quiet_window",
            Self::Focus => "focus",
            Self::FrequencyCap => "frequency_cap",
            Self::Budget => "budget",
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
    /// 打扰纪律拦下 (候选拦下不丢, 下次心跳重试)。
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
    pub fn to_turn_input(&self) -> String {
        let focus = self
            .verdict
            .focus
            .as_deref()
            .unwrap_or(self.candidate.material.as_str());
        format!(
            "[care:{rule}] 时机: {trigger} (到点 {due}); 素材: {material}; 发起意愿判定: \
             {mode} 该说 (理由: {reason}); 说什么方向: {focus}. 请以一句关怀消息回应这次主动关怀。",
            rule = self.candidate.rule_id,
            trigger = self.candidate.trigger,
            due = self.candidate.due_ms,
            material = self.candidate.material,
            mode = self.verdict.mode.label(),
            reason = self.verdict.reason,
            focus = focus,
        )
    }
}

/// 双引擎关怀内核 (规则引擎 + 判定面 + 打扰纪律)。
pub struct CareEngine {
    store: CareStore,
    clock: Arc<dyn CareClock>,
    judge: Arc<dyn CareJudge>,
    material: Arc<dyn CareMaterialSource>,
    budget: Arc<dyn CareBudgetGate>,
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

    /// 每次心跳调用一次: 取第一个到点规则 → 打扰纪律 → 发起意愿判定。
    ///
    /// 顺序固定 (先立规矩再谈温度): 勿扰窗口 → 专注抑制 → 频率上限 →
    /// 关怀预算 → 判定。纪律拦下 = [`CarePoll::Held`] (候选不丢, 重试);
    /// 判定说不 = [`CarePoll::Declined`] (当次到点消费掉, 台账留痕);
    /// 判定该说 = [`CarePoll::Initiating`] (发起留痕 + 台账)。
    pub fn poll(&mut self) -> Result<CarePoll, CareStoreError> {
        let now_ms = self.clock.now_ms();
        let body = self.store.list()?;
        let Some(rule) = self.first_due_rule(&body, now_ms) else {
            return Ok(CarePoll::Idle);
        };

        // 打扰纪律 (四件, 顺序固定)。
        if let Some(hold) = self.discipline_hold(&body, now_ms) {
            return Ok(CarePoll::Held {
                rule_id: rule.id.clone(),
                hold,
            });
        }

        // 发起意愿判定 (显式判定面)。
        let candidate = CareCandidate::from_rule(rule, now_ms);
        let input = CareJudgmentInput {
            recent_material: self.material.recent_material(&candidate),
            candidate,
        };
        let verdict = self.judge.judge(&input);
        let record = CareRecord {
            rule_id: input.candidate.rule_id.clone(),
            at_ms: now_ms,
            judge: verdict.mode.label().to_string(),
            spoke: verdict.speak,
            note: verdict.reason.clone(),
        };
        self.store.record_care(record, now_ms)?;
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

    fn discipline_hold(&self, body: &CareRulesBody, now_ms: i64) -> Option<CareHold> {
        let discipline = &body.discipline;
        // ① 勿扰窗口 (与 initiative 语义的安静窗口同口径)。
        let boundaries = Boundaries {
            quiet_start_minutes: discipline.quiet_start_minutes,
            quiet_end_minutes: discipline.quiet_end_minutes,
            ..Boundaries::default()
        };
        if boundaries.in_quiet_window(minutes_of_day_from_epoch_ms(now_ms)) {
            return Some(CareHold::QuietWindow);
        }
        // ② 专注抑制: 距上次交互不足窗口 = 连续交互期, 不插话。
        if let Some(last) = self.last_interaction_ms {
            if now_ms.saturating_sub(last) < discipline.focus_gap_ms() {
                return Some(CareHold::Focus);
            }
        }
        // ③ 频率上限: 窗口内已发 M 次 → 拦下 (不丢, 窗口滑过再发)。
        let window_start = now_ms - discipline.frequency_window_ms();
        let sent = body
            .history
            .iter()
            .filter(|record| record.spoke && record.at_ms >= window_start)
            .count();
        if sent >= usize::try_from(discipline.frequency_max).unwrap_or(usize::MAX) {
            return Some(CareHold::FrequencyCap);
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
}
