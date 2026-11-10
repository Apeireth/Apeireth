//! B2 批「三接口契约」的**生产接线件**：读契约（[`MemoryView`]）/ 标注契约（[`MemoryAnnotation`]）/
//! 注入契约（[`InjectionRenderer`]）。
//!
//! # 这个模块解决什么问题
//!
//! 三契约**早就实现好了**（`memory_view.rs` / `injection_renderer.rs`），但**生产零调用**
//! ⇒ ⇒ 契约**没有约束任何东西**（只是「库级完成」）。
//! 本模块把**生产读路径**（召回 → 选片段 → 注入渲染）接到契约上：
//!
//! 1. **读契约**：生产选中的那批记忆，被包成 [`SelectedMemoryView`]
//!    （`MemoryView` 的生产实现）⇒ 算法**只能**通过它读，**不许**直接碰库；
//! 2. **标注契约**：每条被选中的记忆，产出一条**纯数据**的 [`MemoryAnnotation`]
//!    （分数 + 出处）⇒ 类型层面**不可能**回写原始层；
//! 3. **注入契约**：视图的证据（内容 + 出处 + 相关度）可以被 [`InjectionRenderer`] 渲染，
//!    且**渲染产物必须过预算校验**（越预算 ⇒ **报错**，不照用）。
//!
//! # 默认行为必须**逐字节不变**
//!
//! * 视图的可见性口径**与 canonical [`MemoryScope::is_visible_in`] 逐条对齐**
//!   （见 [`scope_visible`] 与 [`assert_scope_alignment`]）⇒ ⇒ 走契约视图 = 走 canonical 口径
//!   ⇒ ⇒ **零行为变化**；
//! * 注入渲染：**默认仍走既有编译路径**（XML 闭世界形态**不可由契约的 `Injection` 枚举表达**
//!   ⇒ 换渲染器**必然**改字节）⇒ ⇒ 契约渲染器是**显式 opt-in**（装了才走）。
//!
//! # 「契约违例被真的挡住」落在哪
//!
//! * **口径分叉**：契约与 canonical 的可见性口径**一旦分叉** ⇒ [`assert_scope_alignment`] 报错
//!   ⇒ ⇒ 生产装配**拒绝启动**（不是「记一笔继续跑」）；
//! * **渲染器越预算**：渲染器的产出在装配后**再校验一次** ⇒ 超预算 ⇒ **报错**（不照用）。
//!
//! # 中性表述
//!
//! 只描述**机制**（读契约 / 标注契约 / 注入契约 / 预算纪律），不引用任何外部产品名。

use crate::injection_renderer::{Budget, Evidence, Injection, InjectionRenderer};
use crate::layers::RecalledMemoryItem;
use crate::memory_view::{
    scope_visible, AlgoStamp, AnnotationKind, AnnotationPayload, MemoryAnnotation, MemoryView,
    ProvenanceChain, Record, RecordId, Scope,
};
use crate::scope::MemoryScope;

/// 视图里的一条记录（**契约记录 + 生产条目**，双份 ⇒ 投影无损）。
#[derive(Debug, Clone)]
pub struct ViewEntry {
    /// 契约侧记录（窄：id / 正文 / 时间戳 / 出处 / 范围）。
    pub record: Record,
    /// 生产侧原条目（**无损投影**：渲染与编译仍用它 ⇒ 逐字节不变）。
    pub item: RecalledMemoryItem,
    /// 相关度（0.0 ~ 1.0）。
    pub relevance: f64,
    /// 出处说明（注入时给模型看的「这条从哪来」）。
    pub attribution: String,
}

/// 生产选中集的**契约视图**（`MemoryView` 的生产实现）。
///
/// # 为什么它有「原条目」这一份
/// 契约视图是**窄面**（给算法读）；而生产编译路径要的是**全字段**条目
/// ⇒ ⇒ 视图**同时**持有原条目 ⇒ ⇒ 「用视图编译」与「直接用条目编译」**逐字节相同**
/// ⇒ ⇒ 接线不改变任何输出。
#[derive(Debug, Clone, Default)]
pub struct SelectedMemoryView {
    entries: Vec<ViewEntry>,
    annotations: Vec<MemoryAnnotation>,
}

impl SelectedMemoryView {
    /// 造一个视图（**生产接线入口**）。
    ///
    /// # 口径
    ///
    /// * 每条生产条目 ⇒ 一条契约记录（id / 正文 / 时间戳 / 出处 / 范围）；
    /// * 每条被选中的条目 ⇒ 一条**纯数据标注**（分数 + 标签），**不回写**任何东西；
    /// * **可见性 fail-closed**：不在授权范围里的条目**直接不装进视图**。
    pub fn build(items: &[RecalledMemoryItem], granted_scopes: &[Scope], algo: AlgoStamp) -> Self {
        let mut entries = Vec::new();
        let mut annotations = Vec::new();
        for item in items {
            let scope = Scope::Global;
            if !scope_visible(&scope, granted_scopes) {
                // ⭐ **fail-closed**：契约视图**不装**不可见的记录（不是「装了再过滤」）。
                continue;
            }
            let relevance = item.score.clamp(0.0, 1.0);
            let attribution = item
                .source_ref
                .clone()
                .unwrap_or_else(|| format!("layer={}", item.layer.as_str()));
            let record = Record {
                id: RecordId::new(item.id.clone()),
                content: item.content.clone(),
                timestamp_ms: item.timestamp_ms,
                provenance: ProvenanceChain::default(),
                scope: scope.clone(),
            };
            annotations.push(MemoryAnnotation::new(
                AnnotationKind::Score,
                record.id.clone(),
                AnnotationPayload::score(relevance),
                algo.clone(),
            ));
            annotations.push(MemoryAnnotation::new(
                AnnotationKind::Tag,
                record.id.clone(),
                AnnotationPayload::tag(attribution.clone()),
                algo.clone(),
            ));
            entries.push(ViewEntry {
                record,
                item: item.clone(),
                relevance,
                attribution,
            });
        }
        Self {
            entries,
            annotations,
        }
    }

    /// 视图里的**证据**（注入契约的输入：内容 + 出处 + 相关度，**刻意比检索侧窄**）。
    pub fn evidence(&self) -> Vec<Evidence> {
        self.entries
            .iter()
            .map(|entry| {
                Evidence::new(
                    entry.record.id.0.clone(),
                    entry.record.content.clone(),
                    entry.attribution.clone(),
                    entry.relevance,
                )
            })
            .collect()
    }

    /// 视图产出的**标注**（纯数据；**没有**任何写回方法 ⇒ 结构上不可能回写原始层）。
    pub fn annotations(&self) -> &[MemoryAnnotation] {
        &self.annotations
    }

    /// 无损投影：视图里的**生产条目**（顺序与生产一致）。
    pub fn items(&self) -> Vec<RecalledMemoryItem> {
        self.entries
            .iter()
            .map(|entry| entry.item.clone())
            .collect()
    }

    /// 视图条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 视图是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl MemoryView for SelectedMemoryView {
    fn record(&self, id: &RecordId) -> Option<&Record> {
        self.entries
            .iter()
            .find(|entry| &entry.record.id == id)
            .map(|entry| &entry.record)
    }

    fn by_time(&self, from_ms: i64, to_ms: i64) -> Vec<&Record> {
        if from_ms > to_ms {
            return Vec::new();
        }
        let mut records: Vec<&Record> = self
            .entries
            .iter()
            .filter(|entry| {
                entry.record.timestamp_ms >= from_ms && entry.record.timestamp_ms <= to_ms
            })
            .map(|entry| &entry.record)
            .collect();
        records.sort_by(|left, right| {
            left.timestamp_ms
                .cmp(&right.timestamp_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        records
    }

    fn related(&self, id: &RecordId) -> Vec<&Record> {
        // 「相关」= **同出处组**的其它记录（生产侧 = 同一 `source_ref`）⇒ 确定性（按 id 升序）。
        let Some(anchor) = self.entries.iter().find(|entry| &entry.record.id == id) else {
            return Vec::new();
        };
        let group = anchor.attribution.clone();
        let mut records: Vec<&Record> = self
            .entries
            .iter()
            .filter(|entry| entry.record.id != *id && entry.attribution == group)
            .map(|entry| &entry.record)
            .collect();
        records.sort_by(|left, right| left.id.cmp(&right.id));
        records
    }

    fn provenance(&self, id: &RecordId) -> ProvenanceChain {
        self.record(id)
            .map(|record| record.provenance.clone())
            .unwrap_or_default()
    }

    fn visible_to(&self, scope: &Scope) -> bool {
        // **fail-closed**：视图内**每条**都可见才 true（契约口径）。
        !self.entries.is_empty()
            && self
                .entries
                .iter()
                .all(|entry| scope_visible(&entry.record.scope, &[scope.clone()]))
    }

    fn record_visible_to(&self, id: &RecordId, granted_scopes: &[Scope]) -> bool {
        self.record(id)
            .map(|record| scope_visible(&record.scope, granted_scopes))
            .unwrap_or(false)
    }
}

/// canonical 范围 → 契约范围（**一一对应**；不可映射 ⇒ `None` ⇒ 装配报错）。
pub fn canonical_scope_to_contract(scope: &MemoryScope) -> Option<Scope> {
    Some(match scope {
        MemoryScope::Global => Scope::Global,
        MemoryScope::User { user_id } => Scope::User {
            user_id: user_id.clone(),
        },
        MemoryScope::Persona {
            persona_id,
            user_id,
        } => Scope::Persona {
            persona_id: persona_id.clone(),
            user_id: user_id.clone(),
        },
        MemoryScope::Project { project_id } => Scope::Project {
            project_id: project_id.clone(),
        },
        MemoryScope::Session { session_id } => Scope::Session {
            session_id: session_id.clone(),
        },
    })
}

/// ⭐ **口径对齐自检**：契约的 [`scope_visible`] 与 canonical 的 [`MemoryScope::is_visible_in`]
/// **必须逐条一致**。
///
/// # 为什么这是**装配门**而不是测试
/// 契约与 canonical 是**两套可见性口径**（一个在契约里、一个在 canonical 里）
/// ⇒ ⇒ 一旦分叉，**同一批记忆在两处会得到不同的可见性** ⇒ ⇒ **权限判定分叉 = 安全风险**
/// ⇒ ⇒ 生产装配时**跑一次**，不一致 ⇒ **拒绝启动**（不是「记一笔继续跑」）。
///
/// # 覆盖面
/// 全部 5 类范围 × 全部 5 类授权范围 + 跨类组合 ⇒ 共 25 组。
pub fn assert_scope_alignment() -> Result<(), String> {
    let canonical = [
        MemoryScope::Global,
        MemoryScope::User {
            user_id: "u1".to_string(),
        },
        MemoryScope::Persona {
            persona_id: "p1".to_string(),
            user_id: "u1".to_string(),
        },
        MemoryScope::Project {
            project_id: "pr1".to_string(),
        },
        MemoryScope::Session {
            session_id: "s1".to_string(),
        },
    ];
    let mut mismatches: Vec<String> = Vec::new();
    for record_scope in &canonical {
        for granted_scope in &canonical {
            let canonical_visible = record_scope.is_visible_in(std::slice::from_ref(granted_scope));
            let contract_scope = canonical_scope_to_contract(record_scope)
                .expect("canonical 范围必须可映射到契约范围");
            let contract_granted = canonical_scope_to_contract(granted_scope)
                .expect("canonical 范围必须可映射到契约范围");
            let contract_visible = scope_visible(&contract_scope, &[contract_granted]);
            if canonical_visible != contract_visible {
                mismatches.push(format!(
                    "记录范围 {record_scope:?} × 授权 {granted_scope:?}：canonical={canonical_visible} 契约={contract_visible}"
                ));
            }
        }
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "契约可见性口径与 canonical **分叉**（{} 组）⇒ ⇒ **拒绝装配**：\n  {}",
            mismatches.len(),
            mismatches.join("\n  ")
        ))
    }
}

/// ⭐ **用契约渲染注入**，并**校验产物**（越预算 ⇒ **报错**，不照用）。
///
/// # 为什么渲染后还要校验
/// `InjectionRenderer` 是 trait ⇒ 实现可以**乱来**（返回超预算的 `Injection`）
/// ⇒ ⇒ 类型给不了保证 ⇒ ⇒ 只能**产出后校验**：
/// * `char_len() > budget.max_chars` ⇒ **报错**（上下文被撑爆 ⇒ 上游预算口径失效）。
pub fn render_via_contract(
    renderer: &dyn InjectionRenderer,
    evidence: &[Evidence],
    budget: Budget,
) -> Result<String, String> {
    let injection: Injection = renderer.render(evidence, budget);
    let length = injection.char_len();
    if length > budget.max_chars {
        return Err(format!(
            "渲染器产出**超预算**：{length} 字 > 预算 {} 字 ⇒ ⇒ **拒绝照用**（坏渲染器不许进生产）",
            budget.max_chars
        ));
    }
    Ok(injection_text(&injection))
}

/// 把契约产物转成**文本**（形态可换 ⇒ 文本是它的投影）。
pub fn injection_text(injection: &Injection) -> String {
    match injection {
        Injection::Plain { text } => text.clone(),
        Injection::Bulleted { lines } => lines.join("\n"),
        Injection::Conclusions { items } => items.join("\n"),
        Injection::Empty { reason } => format!("（没有可注入的证据：{reason}）"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::MemoryLayerKind;

    fn item(id: &str, content: &str, score: f64) -> RecalledMemoryItem {
        RecalledMemoryItem {
            id: id.to_string(),
            layer: MemoryLayerKind::Episodic,
            content: content.to_string(),
            timestamp_ms: 1_700_000_000_000,
            score,
            importance: 0.5,
            source_ref: Some("session-a".to_string()),
            score_components: None,
            diary_snippets: Vec::new(),
        }
    }

    fn stamp() -> AlgoStamp {
        AlgoStamp {
            algo_id: "production.selection".to_string(),
            version: "1".to_string(),
            params_digest: "test".to_string(),
        }
    }

    #[test]
    fn the_contract_scope_matches_the_canonical_scope() {
        assert_scope_alignment().expect("两套口径必须逐条一致");
    }

    #[test]
    fn the_view_is_fail_closed_and_keeps_the_production_items() {
        let items = vec![item("a", "alpha", 0.9), item("b", "beta", 0.4)];
        let view = SelectedMemoryView::build(&items, &[Scope::Global], stamp());
        assert_eq!(view.len(), 2, "Global 授权 ⇒ 全部可见");
        assert_eq!(view.items().len(), items.len(), "投影必须无损");
        assert_eq!(view.items()[0].id, "a", "顺序必须与生产一致");

        // 不授权 Global ⇒ **fail-closed**（一条都不装）。
        let closed = SelectedMemoryView::build(&items, &[], stamp());
        assert!(closed.is_empty(), "没授权 ⇒ 视图必须空（fail-closed）");
    }

    #[test]
    fn annotations_are_pure_data() {
        let view = SelectedMemoryView::build(&[item("a", "alpha", 0.9)], &[Scope::Global], stamp());
        assert_eq!(
            view.annotations().len(),
            2,
            "每条一个分数标注 + 一个标签标注"
        );
        for annotation in view.annotations() {
            assert_eq!(annotation.algo.algo_id, "production.selection");
            assert_eq!(annotation.target.0, "a");
        }
    }

    #[test]
    fn a_renderer_that_exceeds_the_budget_is_refused() {
        struct Greedy;
        impl InjectionRenderer for Greedy {
            fn render(&self, _evidence: &[Evidence], _budget: Budget) -> Injection {
                Injection::Plain {
                    text: "x".repeat(10_000),
                }
            }
        }
        let evidence = vec![Evidence::new("a", "alpha", "src", 1.0)];
        let error =
            render_via_contract(&Greedy, &evidence, Budget::new(100, 10)).expect_err("必须拒绝");
        assert!(error.contains("超预算"), "报错必须带真因：{error}");
    }
}
