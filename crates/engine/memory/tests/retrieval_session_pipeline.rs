//! 检索管线的**显式会话**入口（**跨 crate 的公开 API 视角**）。
//!
//! 目的: 证明**外部调用方**可以直接把 session id 交给检索管线
//! (`retrieve_for_session` / `retrieve_with_status_for_session`),
//! 而不必依赖**线程局部的会话作用域**（跨线程 / 跨 `await` 会失效 ⇒ ⇒
//! 静默回落到默认桶 ⇒ ⇒ 又变多会话串扰）。
//!
//! ⚠️ 生效条目显著性是**进程级槽** ⇒ ⇒ 本文件**只放一个测试函数**
//! (同一文件里的多个测试会并行跑 ⇒ ⇒ 会互相踩槽)。

use apeireth_memory::retrieval_tuning::{
    install_effective_for_session, RetrievalTuningEngine, TuningEvidence, ENTRY_STEP,
};
use apeireth_memory::{
    BasicLexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidate, MemoryProvenance,
    MemoryScope, ScoreComponents,
};

fn candidate(id: &str, content: &str, scope: MemoryScope) -> MemoryCandidate {
    MemoryCandidate {
        id: id.into(),
        layer: "episodic".into(),
        scope,
        content: content.into(),
        score: 0.0,
        score_components: ScoreComponents::default(),
        provenance: MemoryProvenance::default(),
    }
}

fn score_of(items: &[MemoryCandidate], id: &str) -> f64 {
    items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.score)
        .expect("候选条目在场")
}

/// **外部调用方**用**显式 session id** 检索 ⇒ ⇒ 生效条目显著性按会话隔离。
///
/// ① A 会话显式查 ⇒ ⇒ 只吃到 A 的显著性（**吃不到** B 的）；
/// ② B 会话显式查 ⇒ ⇒ 只吃到 B 的；
/// ③ 作用域内的旧入口照旧工作（**兼容兜底**）。
#[test]
fn external_callers_can_pass_the_session_id_explicitly() {
    let scope = MemoryScope::Global;
    let source = BasicLexicalCandidateSource::new(vec![
        candidate("a", "rust memory retrieval design", scope.clone()),
        candidate("b", "rust memory retrieval notes", scope.clone()),
    ]);
    let pipeline = HybridRetrievalPipeline::default();

    // 基线 (未装任何生效值 => 零显著性加成)。
    let baseline = pipeline
        .retrieve("rust memory", &[scope.clone()], &[&source], 2, 1_000)
        .unwrap();
    let base_a = score_of(&baseline, "a");
    let base_b = score_of(&baseline, "b");

    // A 会话提 b; B 会话提 a (各自只动自己的条目)。
    let mut engine_a = RetrievalTuningEngine::new();
    engine_a.observe(&TuningEvidence::positive("b", None, 1_000));
    let mut engine_b = RetrievalTuningEngine::new();
    engine_b.observe(&TuningEvidence::positive("a", None, 1_000));
    install_effective_for_session("session-a", engine_a.effective());
    install_effective_for_session("session-b", engine_b.effective());

    // ① A 显式查 => 吃到 A 的 b 提权, 吃不到 B 的 a 提权。
    let a_items = pipeline
        .retrieve_for_session(
            "session-a",
            "rust memory",
            &[scope.clone()],
            &[&source],
            2,
            1_000,
        )
        .unwrap();
    assert!(
        (score_of(&a_items, "b") - base_b - ENTRY_STEP).abs() < 1e-12,
        "A 必须读到自己的 b 提权"
    );
    assert!(
        (score_of(&a_items, "a") - base_a).abs() < 1e-12,
        "A 不得吃到 B 的 a 提权 (不许串扰)"
    );

    // ② B 显式查 => 吃到 B 的 a 提权, **吃不到** A 的 b 提权。
    let b_items = pipeline
        .retrieve_for_session(
            "session-b",
            "rust memory",
            &[scope.clone()],
            &[&source],
            2,
            1_000,
        )
        .unwrap();
    assert!(
        (score_of(&b_items, "a") - base_a - ENTRY_STEP).abs() < 1e-12,
        "B 必须读到自己的 a 提权"
    );
    assert!(
        (score_of(&b_items, "b") - base_b).abs() < 1e-12,
        "B 不得吃到 A 的 b 提权 (不许串扰)"
    );

    // ③ 作用域内的旧入口照旧工作 (**兼容兜底**)。
    let ambient = {
        let _scope = apeireth_memory::retrieval_tuning::enter_session_scope("session-a");
        pipeline
            .retrieve("rust memory", &[scope.clone()], &[&source], 2, 1_000)
            .unwrap()
    };
    assert_eq!(
        ambient
            .iter()
            .map(|item| (item.id.as_str(), item.score))
            .collect::<Vec<_>>(),
        a_items
            .iter()
            .map(|item| (item.id.as_str(), item.score))
            .collect::<Vec<_>>(),
        "显式入口与作用域入口必须读到同一份显著性加成 (两条通道等价)"
    );

    // 收尾: 清空进程级槽 (不留下跨用例的残留)。
    apeireth_memory::retrieval_tuning::clear_effective();
}
