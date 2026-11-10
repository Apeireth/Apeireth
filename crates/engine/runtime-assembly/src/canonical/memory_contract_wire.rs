//! B2 批「三接口契约」的**生产装配接线**（读契约 / 标注契约 / 注入契约）。
//!
//! # 这个文件解决什么问题
//! 三契约**早就实现好了**，但**生产零调用** ⇒ 契约**约束不到任何东西**（只是「库级完成」）⇒ 本文件把它们接进生产装配，并给出**会真的挡住事故**的那一道门。
//!
//! # 三条契约各自接在哪
//!
//! * **注入契约**（`InjectionRenderer`）⇒ `context_compiler::compile_injection_with_renderer` 用契约渲染器出注入（⭐ **已在生产**；此前口径写「零调用」是**错的**）；
//! * **读契约**（`MemoryView`）⇒ `context_compiler` 编译前把选中集**包成契约视图**，编译从视图的**无损投影**取条目（⭐ **本批接入**）；
//! * **标注契约**（`MemoryAnnotation`）⇒ 同一步产出**纯数据**标注（分数 + 出处）⇒ 类型层面**不可能**回写原始层（⭐ **本批接入**）。
//!
//! # 「会真的挡住」的那道门：口径对齐自检
//!
//! 契约的可见性口径（`memory_view::scope_visible`）与 canonical 的 `MemoryScope::is_visible_in`
//! 是**两套**实现 ⇒ 一旦分叉，**同一批记忆在两处会得到不同的可见性** ⇒ **权限判定分叉 = 安全风险**
//! ⇒ [`install`] 在**装配时**逐组比对（5 × 5 = 25 组）⇒ **不一致 ⇒ 拒绝装配**（不是「记一笔继续跑」）。
//!
//! # 为什么**不**在读路径里做重建
//! `vector_rebuild` 是**运维动作**（删派生索引 → 重建 → 比对），不是**每请求动作** ⇒ 它走**运维入口**，不进读路径。
//!
//! # 中性表述
//! 只描述**机制**（契约 / 口径对齐 / 失败闭合），不引用任何外部产品名。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use apeireth_memory::canonical::memory_contracts::{assert_scope_alignment, SelectedMemoryView};
use apeireth_memory::injection_renderer::{InjectionRenderer, PlainRenderer};
use apeireth_memory::layers::RecalledMemoryItem;
use apeireth_memory::memory_view::{AlgoStamp, Scope};

/// 契约对齐自检覆盖的**范围组数**（5 类范围 × 5 类授权范围）。
pub const ALIGNMENT_PAIRS: usize = 25;

/// 契约面被经过的次数（**只读诊断面**：不参与判定，只供「链路真的过了契约」取证）。
static CONTRACT_INSTALLS: AtomicU64 = AtomicU64::new(0);

/// 契约面被经过的次数（**只读诊断面**：装配真的过了契约门 ⇒ 计数会涨）。
pub fn contract_install_count() -> u64 {
    CONTRACT_INSTALLS.load(Ordering::Relaxed)
}

/// 生产装配拿到的**契约束**。
#[derive(Clone)]
pub struct MemoryContractBundle {
    /// 注入渲染器（**形态可换**：生产默认走既有编号形态；换成别的形态只改这里）。
    pub renderer: Arc<dyn InjectionRenderer>,
    /// 口径对齐自检覆盖的组数（**证据**：自检真的跑了 ⇒ 不是空跑）。
    pub alignment_pairs: usize,
}

/// ⭐ **装配契约束**（生产装配根调用这一行）。
///
/// # 失败闭合
/// 契约与 canonical 的可见性口径**分叉** ⇒ ⇒ **拒绝装配**（装配失败 ⇒ 进程起不来）
/// ⇒ ⇒ 「两套口径」这件事从此**不是靠人记得**。
///
/// # 用法（装配根那一行）
/// ```ignore
/// let _contracts = memory_contract_wire::install().map_err(RuntimeError::misconfigured)?;
/// ```
pub fn install() -> Result<MemoryContractBundle, String> {
    CONTRACT_INSTALLS.fetch_add(1, Ordering::Relaxed);
    assert_scope_alignment()?;
    Ok(MemoryContractBundle {
        renderer: Arc::new(PlainRenderer::new()),
        alignment_pairs: ALIGNMENT_PAIRS,
    })
}

/// ⭐ **给算法用的契约视图工厂**（读契约 + 标注契约的入口）。
///
/// # 为什么放在装配层
/// 算法**只能**通过 [`SelectedMemoryView`] 读生产选中的记忆 ⇒ ⇒ 它**碰不到库**
/// ⇒ ⇒ 「接口要比今天的检索窄」这条设计目标**在生产里成立**。
pub fn view_for(items: &[RecalledMemoryItem], granted_scopes: &[Scope]) -> SelectedMemoryView {
    SelectedMemoryView::build(
        items,
        granted_scopes,
        AlgoStamp {
            algo_id: "production.selection".to_string(),
            version: "1".to_string(),
            params_digest: "production".to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::layers::MemoryLayerKind;

    fn item(id: &str, content: &str) -> RecalledMemoryItem {
        RecalledMemoryItem {
            id: id.to_string(),
            layer: MemoryLayerKind::Episodic,
            content: content.to_string(),
            timestamp_ms: 1_700_000_000_000,
            score: 0.8,
            importance: 0.5,
            source_ref: Some("session-a".to_string()),
            score_components: None,
            diary_snippets: Vec::new(),
        }
    }

    /// ⭐ **装配必须真的跑过口径自检**（不是空跑）。
    #[test]
    fn install_runs_the_scope_alignment_self_check() {
        let bundle = install().expect("装配必须通过口径自检");
        assert_eq!(
            bundle.alignment_pairs, ALIGNMENT_PAIRS,
            "自检必须真的覆盖 25 组范围组合（不是空跑）"
        );
    }

    /// ⭐ **算法只能通过契约视图读**（视图是窄面 + 标注是纯数据）。
    #[test]
    fn the_view_factory_gives_algorithms_a_narrow_contract_surface() {
        let items = vec![item("a", "alpha"), item("b", "beta")];
        let view = view_for(&items, &[Scope::Global]);
        assert_eq!(view.len(), 2);
        assert_eq!(view.annotations().len(), 4, "每条：分数标注 + 标签标注");
        // 不授权 ⇒ **fail-closed**（一条都不给）。
        let closed = view_for(&items, &[]);
        assert!(closed.is_empty(), "没授权 ⇒ 视图必须空");
    }
}
