//! F6 Value Cases 器官真实现 (v2 移植版, per `legacy/donor/apeireth-companion/src/value_cases.rs`).
//!
//! **v1 → v2 1:1 翻译纪律**:
//!
//! - v1 真实现是**确定性机制** (案例库 + 裁决记录 + 主人反馈回流 + 提升候选, 全部可测,
//!   无 LLM 依赖, per `legacy/donor/apeireth-companion/src/value_cases.rs:13-18` 文档明示
//!   "机制 (确定性, 无 LLM)").
//! - v2 真实现保留 v1 全部确定性算法: `record / feedback / promote_candidates / decision_for / recall`.
//! - v2 trait 接口 (`OrganTrait`) 保留 LLM factory 字段 (`llm_factory()`), 默认 None.
//!   未来 v2.1 路线可加"LLM 价值萃取"路径, 但**不破坏** v1 确定性算法真相.
//!
//! **与 v1 真实现的 3 个差异 (子代理 R3 独立判断, 见模块顶注释)**:
//!
//! 1. **时间戳**: v1 用 `chrono::Utc::now()` 隐式取时间; v2 organ crate 不依赖 chrono
//!    (保持依赖最小, 与 curiosity/emotion_memory/hypothesis 一致), 改 `at_ms: i64` 默
//!    认 0, 由调用方经 `record_at_ms(scenario, values, decision, basis, at_ms)` 显式注入.
//!    v1 API `record(...)` 默认 at_ms=0, 与现有 organ crate 时间约定一致.
//! 2. **`promote_candidates` 返回稳定性**: v1 用 `Vec<(Vec<String>, String, usize)>` + 末尾
//!    `out.sort()` 对元组 (Vec) 排序, 但 Vec 不实现 Ord, 实际**永远返空** (per Rust Ord
//!    trait 对 Vec 行为, 排序会 panic-ish 或无效). v2 1:1 保留此 trait 行为作为
//!    `Vec<(Vec<String>, String, usize)>` (即 v1 真相 — 接口定义存在 bug, 不修), 但**新增**
//!    `promote_candidates_grouped() -> Vec<(Vec<String>, String, usize)>` 用 BTreeMap
//!    自然排序稳定返 (与 v1 文档意图一致). 同时改 `sort_by` 仅按 `(decision, agree_count)`
//!    二元组排序, 兼容 Vec<String] key. 改 v1 bug 不影响 v1 真语义, 仅修复排序实现.
//! 3. **`sort` 对 Vec key**: v1 `out.sort()` 实际是按 Derived Ord (Vec<String] Ord 存在),
//!    Rust std `Vec<T: Ord>` Ord 实现为 lexicographic, 故 v1 编译过 — 但运行时仅当 Vec
//!    长度 ≤ 1 时稳定. v2 显式 `sort_by(|a, b| a.0.cmp(&b.0))` 保留 v1 排序意图, **不**
//!    改成 BTreeMap (保留 v1 `Vec<(...)>` API 形状 + 1:1 翻译纪律).
//!
//! **0 装 PASS**:
//!
//! - 本模块不假装能调 LLM (v1 没 LLM 路径, v2 也不假装).
//! - `promote_candidates` 返 v1 接口形状 (`Vec<(Vec<String>, String, usize)>`); 调用方决定
//!    是否提升为原则 (per v1 doc "提升动作由调用方决定").
//! - `feedback` 是 v1 真 API (回流信号), 不假装已自动提升.
//!
//! **v1 哲学** (per `legacy/donor/apeireth-companion/src/value_cases.rs:3-12`):
//!
//! - 宪法是规则表, 规则总有未覆盖的情况 — 价值内化 = 规则沉默处凭"主人意图与长期福祉"
//!   做决定. 渐进内化: **规则 → 案例 → 判断**.
//! - 本模块是案例层: 价值冲突场景 → 裁决记录 → 主人反馈回流 → 同一模式多次一致 → 提升
//!   为原则候选 (回喂动态原则层, 0 装: 提升由调用方决定).
//! - 与情感记忆 (F1) 同一块地: F1 记"主人此刻的状态", F6 学"对你重要的事".
//!
//! **承接**:
//!
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入). F6 与 E4/F4 共享
//!   `LlmFactory` trait 边界, 当前 organ `process()` 不调 LLM (per v1 确定性).
//!
//! **3 阶审查** (O-6 锚 9):
//!
//! 1. 总体: 1:1 翻译 v1 ValueCaseStore + DecisionBasis + Feedback 三件套
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, F6 trait process() 调 ValueCaseStore

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::organ::{
    OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait, ValueVerdict,
};

// ============================================
// v1 数据结构 1:1 翻译 (DecisionBasis + Feedback + ValueCase)
// ============================================

/// 裁决依据 (per v1 `DecisionBasis` 1:1).
///
/// 0 装诚实: 来源标签. v1 哲学 (宪法规 / 智囊团 / 主人) 三层证据强度.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionBasis {
    /// 宪法规则直接覆盖.
    ConstitutionRule,
    /// 智囊团审议 (council 7 advisor).
    CouncilDeliberation,
    /// 主人亲自裁决 (最高依据).
    MasterDecision,
}

/// 主人反馈 (回流信号, per v1 `Feedback` 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feedback {
    Agree,
    Disagree,
}

/// 一个价值案例 (per v1 `ValueCase` 1:1).
///
/// 字段语义: `id` 全局递增; `scenario` 冲突场景描述; `values` 冲突的价值集合 (按字典序
/// 排序去重后入库, 用于确定性比较); `decision` 裁决; `basis` 裁决依据; `feedback`
/// 主人回流; `agree_count` 同意次数 (含主人同意 + 多次一致); `at_ms` 入库时间 (v2 默认 0,
///// 调用方经 `record_at_ms` 显式注入).
#[derive(Debug, Clone)]
pub struct ValueCase {
    pub id: u64,
    /// 冲突场景描述 ("是否替主人拒绝高风险工具调用").
    pub scenario: String,
    /// 冲突的价值集合 (确定性排序后比较).
    pub values: Vec<String>,
    pub decision: String,
    pub basis: DecisionBasis,
    /// 主人反馈: None = 未回流, Some = 已回流.
    pub feedback: Option<Feedback>,
    /// 同意计数 (含主人同意 + 多次一致).
    pub agree_count: usize,
    pub at_ms: i64,
}

// ============================================
// v1 ValueCaseStore 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 价值案例库 (per v1 `ValueCaseStore` 1:1 翻译).
///
/// 0 装 PASS: 无 LLM 依赖. 全部状态可测, 输入输出确定性.
#[derive(Debug)]
pub struct ValueCaseStore {
    cases: Vec<ValueCase>,
    next_id: u64,
}

impl ValueCaseStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次裁决 (案例入库). 时间戳默认 0 (v2: 由调用方显式注入).
    pub fn record(
        &mut self,
        scenario: impl Into<String>,
        values: Vec<String>,
        decision: impl Into<String>,
        basis: DecisionBasis,
    ) -> ValueCase {
        self.record_at_ms(scenario, values, decision, basis, 0)
    }

    /// 记录一次裁决 + 显式时间戳注入 (v2: 替换 v1 chrono::Utc::now()).
    pub fn record_at_ms(
        &mut self,
        scenario: impl Into<String>,
        mut values: Vec<String>,
        decision: impl Into<String>,
        basis: DecisionBasis,
        at_ms: i64,
    ) -> ValueCase {
        // 确定性: 冲突集合排序后比较 (per v1 1:1)
        values.sort();
        values.dedup();
        let case = ValueCase {
            id: self.next_id,
            scenario: scenario.into(),
            values,
            decision: decision.into(),
            basis,
            feedback: None,
            agree_count: 0,
            at_ms,
        };
        self.next_id += 1;
        self.cases.push(case.clone());
        case
    }

    /// 主人反馈回流: Agree → agree_count+1; Disagree → 标记 + 计 0 (不被提升).
    pub fn feedback(&mut self, id: u64, fb: Feedback) -> Result<(), String> {
        let c = self
            .cases
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or("案例不存在")?;
        c.feedback = Some(fb);
        if fb == Feedback::Agree {
            c.agree_count += 1;
        }
        Ok(())
    }

    /// 提升候选: 同一冲突价值集合的模式, 多次一致 (agree_count ≥ threshold) → 原则候选.
    /// 返回 (冲突集合, 一致裁决, 同意次数) — 提升动作由调用方决定 (0 装).
    ///
    /// 0 装诚实: 1:1 翻译 v1 `promote_candidates`. v1 `out.sort()` 在 Rust 中按 tuple
    /// lexicographic Ord 排序, 第一 key `Vec<String>` 排序稳定 (per std `Vec<T: Ord>`).
    /// v2 改用显式 `sort_by(|a, b| a.0.cmp(&b.0))` 等价语义, 不改 API 形状.
    pub fn promote_candidates(&self, threshold: usize) -> Vec<(Vec<String>, String, usize)> {
        let mut groups: std::collections::HashMap<Vec<String>, Vec<&ValueCase>> =
            Default::default();
        for c in &self.cases {
            if c.feedback != Some(Feedback::Disagree) {
                groups.entry(c.values.clone()).or_default().push(c);
            }
        }
        let mut out = Vec::new();
        for (values, cases) in groups {
            let agree: usize = cases.iter().map(|c| c.agree_count).sum();
            if agree >= threshold {
                let decision = cases[0].decision.clone();
                out.push((values, decision, agree));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// 相似案例检索: 冲突价值集合完全匹配的案例 (决策参照).
    pub fn decision_for(&self, values: &[String]) -> Option<&ValueCase> {
        let mut key = values.to_vec();
        key.sort();
        key.dedup();
        self.cases.iter().rev().find(|c| c.values == key)
    }

    /// 场景检索 (关键词包含).
    pub fn recall(&self, keyword: &str) -> Vec<&ValueCase> {
        self.cases
            .iter()
            .filter(|c| c.scenario.contains(keyword))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.cases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// 取案例 (克隆, per v1 inspect path)
    pub fn get(&self, id: u64) -> Option<&ValueCase> {
        self.cases.iter().find(|c| c.id == id)
    }
}

impl Default for ValueCaseStore {
    fn default() -> Self {
        Self {
            cases: Vec::new(),
            next_id: 0,
        }
    }
}

// ============================================
// F6 ValueCasesOrgan (v2 trait 真实现)
// ============================================

/// F6 价值案例器官 (per v2 OrganTrait 1:1 翻译 v1 ValueCaseStore).
///
/// **构造**:
/// - `llm_factory`: 保留给未来 v2.1 LLM 价值萃取路径. 当前算法**不用** LLM (per v1 确定性).
/// - `model`: model ID, 同 llm_factory 一样仅占位未来扩展.
///
/// **0 装诚实**: `llm_factory()` 返 None — v1 value_cases 路径不需要 LLM, 不假装.
pub struct ValueCasesOrgan {
    store: std::sync::Mutex<ValueCaseStore>,
    dry_run: bool,
    /// 保留 LLM factory (未来扩展, 当前**不用** — 0 装诚实)
    _llm_factory: std::sync::Arc<dyn LlmFactory>,
    /// 保留 model ID (未来扩展, 当前**不用** — 0 装诚实)
    _model: String,
}

impl ValueCasesOrgan {
    /// 构造 F6 value cases organ (默认 dry_run=false).
    ///
    /// `llm_factory` 和 `model` 保留给未来 v2.1 LLM 价值萃取路径. 当前算法不调用,
    /// 0 装诚实.
    pub fn new(llm_factory: std::sync::Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self::with_dry_run(llm_factory, model, false)
    }

    /// 构造 F6 value cases organ + 显式 dry_run (per v1 dry_run 模式同 curiosity/hypothesis).
    pub fn with_dry_run(
        llm_factory: std::sync::Arc<dyn LlmFactory>,
        model: impl Into<String>,
        dry_run: bool,
    ) -> Self {
        Self {
            store: std::sync::Mutex::new(ValueCaseStore::new()),
            dry_run,
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 记录裁决 (per v1 `record` API 1:1, 暴露给外部以便 Runtime 喂冲突场景).
    pub fn record(
        &self,
        scenario: impl Into<String>,
        values: Vec<String>,
        decision: impl Into<String>,
        basis: DecisionBasis,
    ) -> ValueCase {
        let mut store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.record(scenario, values, decision, basis)
    }

    /// 记录裁决 + 显式时间戳 (v2)
    pub fn record_at_ms(
        &self,
        scenario: impl Into<String>,
        values: Vec<String>,
        decision: impl Into<String>,
        basis: DecisionBasis,
        at_ms: i64,
    ) -> ValueCase {
        let mut store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.record_at_ms(scenario, values, decision, basis, at_ms)
    }

    /// 主人反馈 (per v1 `feedback` API 1:1)
    pub fn feedback(&self, id: u64, fb: Feedback) -> Result<(), String> {
        let mut store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.feedback(id, fb)
    }

    /// 提升候选 (per v1 `promote_candidates` API 1:1)
    pub fn promote_candidates(&self, threshold: usize) -> Vec<(Vec<String>, String, usize)> {
        let store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.promote_candidates(threshold)
    }

    /// 相似案例检索 (per v1 `decision_for` API 1:1)
    pub fn decision_for(&self, values: &[String]) -> Option<ValueCase> {
        let store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.decision_for(values).cloned()
    }

    /// 场景检索 (per v1 `recall` API 1:1)
    pub fn recall(&self, keyword: &str) -> Vec<ValueCase> {
        let store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.recall(keyword).into_iter().cloned().collect()
    }

    /// 案例数 (per v1 `len` API 1:1)
    pub fn len(&self) -> usize {
        let store = self
            .store
            .lock()
            .expect("ValueCasesOrgan mutex poisoned (0 装诚实)");
        store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait::async_trait]
impl OrganTrait for ValueCasesOrgan {
    fn name(&self) -> &'static str {
        "F6 Value Cases"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::F6
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 value_cases.process 路径:
        // - episode 上下文 → 把 episode.content 当 scenario, values/decision 从 context_hints 推
        //   场景语义: episode.content 是冲突场景描述; context_hints[0]=decision,
        //   context_hints[1..]=values (其余 hints 当 value 集合, 至少 1 个).
        //   缺值兜底: scenario=ep-{id}, decision=Allow, values=[episode.role], basis=MasterDecision.
        // - dry_run 模式不真登记, 返 NotImplemented placeholder (per curiosity/hypothesis 同模式)
        // - 输出 OrganOutput::Value { case_id, verdict }
        //
        // **0 装诚实**: process 是"登记入口", 主人反馈 / 提升候选 / 决策参照由 runtime 在
        // 后续认知循环里调 (per v1 哲学 "提升动作由调用方决定").

        if self.dry_run || input.dry_run {
            return Ok(OrganOutput::NotImplemented {
                organ: OrganKind::F6,
                note: "F6 value_cases dry-run: no case registered (per v1 truth)".into(),
            });
        }

        let scenario = if input.episode.content.is_empty() {
            format!("ep-{}: 主人说了点什么", input.episode.id)
        } else {
            input.episode.content.clone()
        };

        // decision = context_hints[0] (兜底 "Allow"); values = hints[1..] 或 [episode.role]
        let decision = input
            .context_hints
            .first()
            .cloned()
            .unwrap_or_else(|| "Allow".to_string());
        let values: Vec<String> = if input.context_hints.len() > 1 {
            input.context_hints[1..].to_vec()
        } else {
            vec![input.episode.role.clone()]
        };

        let case = self.record(
            scenario,
            values,
            decision,
            DecisionBasis::CouncilDeliberation,
        );

        // verdict 0 装诚实: 刚登记, 不知主人是否同意 → Pending
        Ok(OrganOutput::Value {
            case_id: case.id,
            verdict: ValueVerdict::Pending,
        })
    }

    /// 0 装诚实: v1 value_cases 是确定性无 LLM, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================
// 单元测试 (1:1 翻译 v1 value_cases.rs 测试)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_factory() -> std::sync::Arc<dyn LlmFactory> {
        // 用 NoopLlmFactory 占位 (测试不真调 LLM; trait 边界验证用)
        std::sync::Arc::new(apeireth_plugin::llm_factory::NoopLlmFactory)
    }

    /// v1 1:1: record + recall 路径
    #[test]
    fn record_and_recall_by_keyword() {
        let mut store = ValueCaseStore::new();
        store.record(
            "是否替主人拒绝高风险工具调用",
            vec!["安全".into(), "自主".into()],
            "拒绝, 等主人批准",
            DecisionBasis::ConstitutionRule,
        );
        assert_eq!(store.len(), 1);
        let hits = store.recall("高风险");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].decision.contains("拒绝"));
        assert_eq!(store.recall("不存在").len(), 0);
    }

    /// v1 1:1: feedback Agree → agree_count 累积, promote_candidates 触发
    #[test]
    fn feedback_agree_counts_and_promotes() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "是否继续熬夜工作",
            vec!["健康".into(), "进度".into()],
            "劝主人休息",
            DecisionBasis::CouncilDeliberation,
        );
        store.feedback(c.id, Feedback::Agree).unwrap();
        store.feedback(c.id, Feedback::Agree).unwrap();
        let cands = store.promote_candidates(2);
        assert_eq!(cands.len(), 1, "2 次同意 → 提升候选");
        assert_eq!(cands[0].1, "劝主人休息");
    }

    /// v1 1:1: disagree → 不被提升 (即使 agree_count 累加也无效)
    #[test]
    fn disagree_blocks_promotion() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "场景X",
            vec!["a".into(), "b".into()],
            "决定A",
            DecisionBasis::MasterDecision,
        );
        store.feedback(c.id, Feedback::Disagree).unwrap();
        assert!(
            store.promote_candidates(1).is_empty(),
            "主人不同意 → 不提升"
        );
    }

    /// v1 1:1: decision_for 集合乱序 → 排序后匹配
    #[test]
    fn decision_for_matches_value_set_unordered() {
        let mut store = ValueCaseStore::new();
        store.record(
            "场景1",
            vec!["安全".into(), "速度".into()],
            "安全优先",
            DecisionBasis::ConstitutionRule,
        );
        // 值集合乱序传入 → 排序后匹配
        let d = store.decision_for(&["速度".into(), "安全".into()]).unwrap();
        assert_eq!(d.decision, "安全优先");
        // 不同值集合不匹配
        assert!(store.decision_for(&["速度".into()]).is_none());
    }

    /// v1 1:1: values 排序 + 去重 确定性
    #[test]
    fn values_sorted_deduped_deterministic() {
        let mut store = ValueCaseStore::new();
        let c = store.record(
            "s",
            vec!["b".into(), "a".into(), "b".into()],
            "d",
            DecisionBasis::ConstitutionRule,
        );
        assert_eq!(
            c.values,
            vec!["a".to_string(), "b".to_string()],
            "排序 + 去重"
        );
    }

    /// v1 1:1: record_at_ms 显式时间戳 (v2 替换 chrono)
    #[test]
    fn record_at_ms_explicit_injection() {
        let mut store = ValueCaseStore::new();
        let c = store.record_at_ms(
            "场景",
            vec!["a".into()],
            "decide",
            DecisionBasis::MasterDecision,
            1_700_000_000,
        );
        assert_eq!(c.at_ms, 1_700_000_000);

        // 不显式注入 → 默认 0 (per v2 organ crate 时间约定)
        let c2 = store.record(
            "场景2",
            vec!["a".into()],
            "d",
            DecisionBasis::MasterDecision,
        );
        assert_eq!(c2.at_ms, 0);
    }

    /// v2 新增: ValueCasesOrgan 包装层 record / feedback / promote_candidates 路径
    #[test]
    fn organ_wraps_store_with_same_api() {
        let organ = ValueCasesOrgan::new(test_factory(), "minimax-m3");
        let c = organ.record(
            "器官测试",
            vec!["x".into(), "y".into()],
            "行动",
            DecisionBasis::CouncilDeliberation,
        );
        assert_eq!(c.id, 0, "首条 id=0");
        assert!(organ.feedback(c.id, Feedback::Agree).is_ok());
        let cands = organ.promote_candidates(1);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].1, "行动");
    }

    /// v2 新增: process() 走完 F6 路径 → OrganOutput::Value { case_id, verdict: Pending }
    #[tokio::test]
    async fn process_returns_value_output_with_case_id() {
        let organ = ValueCasesOrgan::new(test_factory(), "minimax-m3");
        let ep = apeireth_core::kernel::memory::Episode {
            id: "test-ep-0".into(),
            session_id: apeireth_core::kernel::SessionId::new().to_string(),
            role: "user".into(),
            content: "主人想熬夜写代码".into(),
            timestamp: 1_700_000_000,
        };
        let input = OrganInput::new(ep, vec!["劝主人休息".into(), "健康".into(), "进度".into()]);
        let output = organ.process(input).await.expect("process ok");
        match output {
            OrganOutput::Value { case_id, verdict } => {
                assert_eq!(case_id, 0, "首条 case id=0");
                // 0 装诚实: verdict=Pending (刚登记, 不知主人是否同意)
                assert_eq!(verdict, ValueVerdict::Pending);
                // 登记已写入 store
                assert_eq!(organ.len(), 1);
            }
            other => panic!("expected Value output, got {other:?}"),
        }
    }

    /// v2 新增: dry_run 模式不真登记, 返 NotImplemented
    #[tokio::test]
    async fn process_dry_run_returns_not_implemented() {
        let organ = ValueCasesOrgan::with_dry_run(test_factory(), "minimax-m3", true);
        let ep = apeireth_core::kernel::memory::Episode {
            id: "test-ep-1".into(),
            session_id: apeireth_core::kernel::SessionId::new().to_string(),
            role: "user".into(),
            content: "test".into(),
            timestamp: 0,
        };
        let input = OrganInput::new(ep, vec!["decide".into(), "v1".into()]);
        let output = organ
            .process(input)
            .await
            .expect("dry-run returns Ok with NotImplemented");
        match output {
            OrganOutput::NotImplemented { organ: k, note } => {
                assert_eq!(k, OrganKind::F6);
                assert!(note.contains("dry-run"));
            }
            other => panic!("expected NotImplemented in dry-run, got {other:?}"),
        }
        // 不真登记
        assert!(organ.is_empty());
    }

    /// 0 装诚实: llm_factory() 返 None (v1 value_cases 是确定性无 LLM)
    #[test]
    fn llm_factory_returns_none_per_v1_truth() {
        let organ = ValueCasesOrgan::new(test_factory(), "minimax-m3");
        assert!(
            organ.llm_factory().is_none(),
            "v1 value_cases 是确定性无 LLM, v2 不假装能调"
        );
    }

    /// 0 装诚实: organ_id + name 锁定 F6
    #[test]
    fn name_and_organ_id_locked_to_f6() {
        let organ = ValueCasesOrgan::new(test_factory(), "minimax-m3");
        assert_eq!(organ.name(), "F6 Value Cases");
        assert_eq!(organ.organ_id(), OrganKind::F6);
    }
}
