use apeireth_memory::{
    BoundedMemoryInput, ExtractedMemory, ExtractionClass, MemoryExtractionMessage,
    MemoryMaterializer, MemoryProvenance, MemoryReconciler, MemoryReconciliationOutcome,
    MemoryReconciliationRecord, MemoryScope,
};

fn candidate(scope: MemoryScope, content: &str) -> ExtractedMemory {
    ExtractedMemory {
        class: ExtractionClass::Fact,
        content: content.into(),
        confidence: 0.9,
        scope,
        provenance: MemoryProvenance::default(),
        source_trace: None,
    }
}

fn record(id: &str, candidate: &ExtractedMemory) -> MemoryReconciliationRecord {
    MemoryReconciliationRecord {
        id: id.into(),
        class: candidate.class.clone(),
        content: candidate.content.clone(),
        scope: candidate.scope.clone(),
        confidence: candidate.confidence,
        revision: 3,
        reinforcement: 0,
    }
}

fn ambiguous_scopes() -> [MemoryScope; 2] {
    [
        MemoryScope::Persona {
            persona_id: "operator@owner".into(),
            user_id: "alice".into(),
        },
        MemoryScope::Persona {
            persona_id: "operator".into(),
            user_id: "owner@alice".into(),
        },
    ]
}

#[test]
fn display_labels_do_not_merge_distinct_persona_identities() {
    let [a, b] = ambiguous_scopes();
    assert_eq!(a.to_string(), b.to_string());
    let old = candidate(a, "城市=武汉");
    for content in ["城市=武汉", "城市=上海"] {
        let report = MemoryReconciler::new()
            .reconcile(&[record("old", &old)], &[candidate(b.clone(), content)]);
        assert_eq!(
            report.decisions[0].outcome,
            MemoryReconciliationOutcome::New
        );
        assert_eq!(report.decisions[0].existing_id, None);
    }
}

#[test]
fn delimiter_in_scope_cannot_be_interpreted_as_assertion_content() {
    let old = candidate(MemoryScope::session("s|城市"), "武汉");
    let report = MemoryReconciler::new().reconcile(
        &[record("old", &old)],
        &[candidate(MemoryScope::session("s"), "城市|武汉")],
    );
    assert_eq!(
        report.decisions[0].outcome,
        MemoryReconciliationOutcome::New
    );
}

#[test]
fn structured_markers_do_not_claim_unique_predicate_cardinality() {
    for (old, new) in [
        ("fact:小林|城市|武汉", "fact:小周|城市|上海"),
        ("fact:小林|城市|武汉", "fact:小林|语言|中文"),
        ("relation:小林|同事|小周", "relation:小林|同事|小王"),
    ] {
        let mut first = candidate(MemoryScope::Global, old);
        if old.starts_with("relation:") {
            first.class = ExtractionClass::Relation;
        }
        let mut second = candidate(MemoryScope::Global, new);
        second.class = first.class.clone();
        let report = MemoryReconciler::new().reconcile(&[record("old", &first)], &[second]);
        assert_eq!(
            report.decisions[0].outcome,
            MemoryReconciliationOutcome::New
        );
    }
}

#[test]
fn provisional_ids_never_replace_a_durable_snapshot_row() {
    let old = candidate(MemoryScope::Global, "城市=武汉");
    let report = MemoryReconciler::new().reconcile(
        &[record("candidate-0", &old)],
        &[
            candidate(MemoryScope::Global, "a=先写另一条"),
            candidate(MemoryScope::Global, "城市=武汉"),
        ],
    );
    let matched = report
        .decisions
        .iter()
        .find(|d| d.candidate.content == old.content)
        .unwrap();
    assert_eq!(matched.outcome, MemoryReconciliationOutcome::Duplicate);
    assert_eq!(matched.existing_id.as_deref(), Some("candidate-0"));
}

#[tokio::test]
async fn materialized_ids_encode_structured_scope_boundaries() {
    let materializer = MemoryMaterializer::default();
    let mut ids = Vec::new();
    for scope in ambiguous_scopes() {
        let input = BoundedMemoryInput::new(
            scope,
            vec![MemoryExtractionMessage {
                role: "user".into(),
                content: "我住在武汉".into(),
            }],
        );
        let first = materializer
            .materialize_episodes(input.clone(), 100)
            .await
            .unwrap();
        let replay = materializer.materialize_episodes(input, 100).await.unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].episode.id, replay[0].episode.id);
        ids.push(first[0].episode.id.clone());
    }
    assert_ne!(ids[0], ids[1]);
}
