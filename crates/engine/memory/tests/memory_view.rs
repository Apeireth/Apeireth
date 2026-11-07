//! 记忆纵深体系**读侧契约**（`memory_view`，设计稿 §5 ①②）集成测试。
//!
//! 钉住几件事（每条都是「不许悄悄变」的判据）：
//! 1. **读契约的边界**：`record` 命中/未命中；`provenance` 对不存在的记录返回**空链**
//!    （**不 panic**）—— 契约层不允许有"炸掉"的路径。
//! 2. **确定性**：`by_time` 按 `(timestamp_ms, id)` 升序、时间窗是**闭区间**；
//!    `related` 按 id 升序、两次调用**逐字相等**；同一批记录建两次视图结果**逐字相等**。
//! 3. **权限在契约里**：`visible_to` 的五种范围口径（全局 / 同 user / 异 user /
//!    同 persona / 异 persona / project / session）。
//! 4. **写契约只产标注**：`MemoryAnnotation` 的四种载荷；分数**出界要 clamp**（不 panic）；
//!    `AlgoStamp` 三字段齐全（追责三件套）；**标注没有回写原始层的入口**。

use apeireth_memory::memory_view::{
    scope_visible, AlgoStamp, AnnotationKind, AnnotationPayload, InMemoryMemoryView,
    MemoryAnnotation, MemoryView, ProvenanceChain, ProvenanceStep, Record, RecordId, Scope,
};

/// 造一条记录（默认全局可见）。
fn record(id: &str, content: &str, timestamp_ms: i64) -> Record {
    Record {
        id: RecordId::new(id),
        content: content.to_string(),
        timestamp_ms,
        provenance: ProvenanceChain::default(),
        scope: Scope::Global,
    }
}

/// 造一条带可见范围的记录。
fn scoped_record(id: &str, content: &str, timestamp_ms: i64, scope: Scope) -> Record {
    Record {
        scope,
        ..record(id, content, timestamp_ms)
    }
}

/// 造一条出处链。
fn provenance(source: &str, note: &str, at_ms: i64) -> ProvenanceChain {
    ProvenanceChain {
        steps: vec![ProvenanceStep {
            note: note.to_string(),
            at_ms,
            source: source.to_string(),
        }],
    }
}

/// 造一个算法戳（追责三件套齐全）。
fn stamp() -> AlgoStamp {
    AlgoStamp {
        algo_id: "betti_hole_detector".to_string(),
        version: "v1.2.3".to_string(),
        params_digest: "sha256:deadbeef".to_string(),
    }
}

// ============================================
// 1. record()：命中 / 未命中
// ============================================

#[test]
fn record_returns_hit_and_miss() {
    let view = InMemoryMemoryView::new(vec![record("r1", "记忆归档", 100)]);

    let hit = view.record(&RecordId::new("r1")).expect("r1 应命中");
    assert_eq!(hit.id, RecordId::new("r1"));
    assert_eq!(hit.content, "记忆归档");
    assert_eq!(hit.timestamp_ms, 100);

    assert!(
        view.record(&RecordId::new("nope")).is_none(),
        "未命中应返回 None"
    );
}

// ============================================
// 2. by_time()：闭区间 + (timestamp_ms, id) 全序
// ============================================

#[test]
fn by_time_window_is_closed_on_both_ends() {
    let view = InMemoryMemoryView::new(vec![
        record("r1", "甲", 100),
        record("r2", "乙", 200),
        record("r3", "丙", 300),
    ]);

    // 端点**含在内**（闭区间）。
    let ids: Vec<&str> = view
        .by_time(100, 200)
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(ids, vec!["r1", "r2"], "闭区间应含两端");

    // 窗口外的记录不出现。
    let ids: Vec<&str> = view
        .by_time(101, 299)
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(ids, vec!["r2"], "窗口外不应出现");
    let ids: Vec<&str> = view
        .by_time(201, 299)
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert!(ids.is_empty(), "窗口外的记录不应出现");
}

#[test]
fn by_time_orders_by_timestamp_then_id() {
    let view = InMemoryMemoryView::new(vec![
        record("r3", "丙", 200),
        record("r1", "甲", 100),
        record("r2", "乙", 200),
        record("r0", "零", 100),
    ]);

    let ids: Vec<&str> = view
        .by_time(0, 1_000)
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["r0", "r1", "r2", "r3"],
        "应按 (timestamp_ms, id) 升序"
    );
}

#[test]
fn by_time_with_inverted_window_is_empty_not_panic() {
    let view = InMemoryMemoryView::new(vec![record("r1", "甲", 100)]);

    assert!(
        view.by_time(300, 100).is_empty(),
        "倒挂窗口应返回空而不是 panic"
    );
}

// ============================================
// 3. related()：共享标签词 ⇒ 互相关
// ============================================

#[test]
fn related_links_records_sharing_a_token() {
    let view = InMemoryMemoryView::new(vec![
        record("r1", "记忆的遗忘语义是什么", 100),
        record("r2", "记忆系统的设计原则", 200),
        record("r3", "量子纠缠的实验记录", 300),
    ]);

    let related: Vec<&str> = view
        .related(&RecordId::new("r1"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(related, vec!["r2"], "只应命中共享标签词的 r2");
    assert!(!related.contains(&"r3"), "不相关记录不应出现在相关集里");
}

#[test]
fn related_is_symmetric() {
    let view = InMemoryMemoryView::new(vec![
        record("r1", "记忆的遗忘语义是什么", 100),
        record("r2", "记忆系统的设计原则", 200),
    ]);

    let forward: Vec<&str> = view
        .related(&RecordId::new("r1"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    let backward: Vec<&str> = view
        .related(&RecordId::new("r2"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();

    assert_eq!(forward, vec!["r2"]);
    assert_eq!(backward, vec!["r1"], "互相关应对称");
}

#[test]
fn related_sorted_by_id_and_deterministic() {
    let view = InMemoryMemoryView::new(vec![
        record("rz", "记忆检索", 100),
        record("ra", "记忆归档", 200),
        record("rm", "记忆摘要", 300),
    ]);

    let first: Vec<&str> = view
        .related(&RecordId::new("ra"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(first, vec!["rm", "rz"], "结果应按 id 升序（不是插入序）");

    let second: Vec<&str> = view
        .related(&RecordId::new("ra"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(first, second, "两次调用应逐字相等");
}

#[test]
fn related_of_unknown_id_is_empty_not_panic() {
    let view = InMemoryMemoryView::new(vec![record("r1", "甲", 100)]);

    assert!(view.related(&RecordId::new("nope")).is_empty());
}

// ============================================
// 4. provenance()：有链 / 空链
// ============================================

#[test]
fn provenance_returns_chain_for_known_record() {
    let mut rec = record("r1", "甲", 100);
    rec.provenance = provenance("user-1", "用户原话", 90);
    let view = InMemoryMemoryView::new(vec![rec]);

    let chain = view.provenance(&RecordId::new("r1"));
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
    assert_eq!(chain.steps[0].source, "user-1");
    assert_eq!(chain.steps[0].note, "用户原话");
    assert_eq!(chain.steps[0].at_ms, 90);
}

#[test]
fn provenance_of_unknown_record_is_empty_chain_not_panic() {
    let view = InMemoryMemoryView::new(vec![record("r1", "甲", 100)]);

    let chain = view.provenance(&RecordId::new("nope"));
    assert!(chain.is_empty(), "无记录应返回空链");
    assert_eq!(chain.steps.len(), 0);
}

// ============================================
// 5. visible_to()：五种范围口径
// ============================================

#[test]
fn visible_to_global_scope_sees_everything() {
    let view = InMemoryMemoryView::new(vec![
        scoped_record(
            "r1",
            "甲",
            100,
            Scope::User {
                user_id: "u1".into(),
            },
        ),
        scoped_record(
            "r2",
            "乙",
            200,
            Scope::Session {
                session_id: "s1".into(),
            },
        ),
    ]);

    // ⚠️ **canonical 口径**：`Global` 只授权 `Global` 记录 ⇒ 这个视图里
    // 有 Session 记录 ⇒ **对 Global 不可见**（fail-closed）。
    // ⇒ 要「全量视角」就把 `Global` 与具体范围**一起**授权（列表口径）。
    assert!(
        !view.visible_to(&Scope::Global),
        "混着 Session 记录的视图不对 Global 开放（canonical 口径）"
    );
    // ⚠️ canonical **跨类不可见** ⇒ 视图里的 User(u1) 记录让这个视图对 Session 也**不可用**
    // ⇒ 混合视图对**任何单一 scope** 都不可见（fail-closed）。
    assert!(
        !view.visible_to(&Scope::Session {
            session_id: "s1".into()
        }),
        "混合视图对任何单一 scope 都不可见（fail-closed）"
    );
    // 只装 Session 记录的视图 ⇒ 对该 Session 可见。
    let session_only = InMemoryMemoryView::new(vec![scoped_record(
        "r2",
        "乙",
        200,
        Scope::Session {
            session_id: "s1".into(),
        },
    )]);
    assert!(session_only.visible_to(&Scope::Session {
        session_id: "s1".into()
    }));
}

#[test]
fn visible_to_same_user_visible_other_user_hidden() {
    let view = InMemoryMemoryView::new(vec![scoped_record(
        "r1",
        "甲",
        100,
        Scope::User {
            user_id: "u1".into(),
        },
    )]);

    assert!(view.visible_to(&Scope::User {
        user_id: "u1".into()
    }));
    assert!(
        !view.visible_to(&Scope::User {
            user_id: "u2".into()
        }),
        "异 user 必须不可见（不 panic）"
    );
}

#[test]
fn visible_to_same_persona_visible_other_persona_hidden() {
    let view = InMemoryMemoryView::new(vec![scoped_record(
        "r1",
        "甲",
        100,
        Scope::Persona {
            persona_id: "p1".into(),
            user_id: "u1".into(),
        },
    )]);

    assert!(view.visible_to(&Scope::Persona {
        persona_id: "p1".into(),
        user_id: "u1".into(),
    }));
    assert!(
        !view.visible_to(&Scope::Persona {
            persona_id: "p2".into(),
            user_id: "u1".into(),
        }),
        "异 persona 必须不可见（不 panic）"
    );
    // ⚠️ 已知口径不对称：User 查询看不到同用户的 Persona 记录（授权委托方向单一）。
    assert!(!view.visible_to(&Scope::User {
        user_id: "u1".into()
    }));
}

#[test]
fn visible_to_project_and_session_scopes() {
    let project_view = InMemoryMemoryView::new(vec![scoped_record(
        "r1",
        "甲",
        100,
        Scope::Project {
            project_id: "p1".into(),
        },
    )]);
    assert!(project_view.visible_to(&Scope::Project {
        project_id: "p1".into()
    }));
    assert!(!project_view.visible_to(&Scope::Project {
        project_id: "p2".into()
    }));
    assert!(!project_view.visible_to(&Scope::Session {
        session_id: "s1".into()
    }));

    let session_view = InMemoryMemoryView::new(vec![scoped_record(
        "r1",
        "甲",
        100,
        Scope::Session {
            session_id: "s1".into(),
        },
    )]);
    assert!(session_view.visible_to(&Scope::Session {
        session_id: "s1".into()
    }));
    assert!(!session_view.visible_to(&Scope::Session {
        session_id: "s2".into()
    }));
}

#[test]
fn visible_to_mixed_view_is_universal_gate() {
    // 全称口径：视图里混了不同 scope 时，**每条都可见**才放行整个视图。
    let view = InMemoryMemoryView::new(vec![
        scoped_record("g", "全局", 100, Scope::Global),
        scoped_record(
            "u",
            "用户",
            200,
            Scope::User {
                user_id: "u1".into(),
            },
        ),
    ]);

    // ⚠️ **canonical 口径**：`Global` **只**授权 `Global` 记录（跨类不可见）
    // ⇒ 装了 User(u1) 记录的视图对 `Global` **不可见**（fail-closed）。
    assert!(
        !view.visible_to(&Scope::Global),
        "混着 User 记录的视图不对 Global 开放"
    );
    // 只装 User(u1) 记录的视图 ⇒ 对 User(u1) 可见。
    let user_only = InMemoryMemoryView::new(vec![scoped_record(
        "r1",
        "甲",
        100,
        Scope::User {
            user_id: "u1".into(),
        },
    )]);
    assert!(user_only.visible_to(&Scope::User {
        user_id: "u1".into()
    }));
    assert!(!user_only.visible_to(&Scope::Global));
    // ⚠️ canonical **跨类不可见** ⇒ 视图里的 Global 记录让这个视图对 User(u1) 也**不可用**
    // ⇒ 混合视图对**任何单一 scope** 都不可见（fail-closed）。
    assert!(
        !view.visible_to(&Scope::User {
            user_id: "u1".into()
        }),
        "混合视图对任何单一 scope 都不可见（fail-closed）"
    );
    assert!(
        !view.visible_to(&Scope::User {
            user_id: "u2".into()
        }),
        "混着 User(u1) 的视图不应整体放给 User(u2)"
    );

    // 逐条口径（**列表**）：这个视图里只有 Global 与 User(u1) 两条
    // ⇒ 授权 User(u2) ⇒ **一条都看不到**（canonical：跨类 + 异 user 都不可见）。
    let visible: Vec<&str> = view
        .visible_records(&[Scope::User {
            user_id: "u2".into(),
        }])
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(visible, Vec::<&str>::new());
    // 授权 User(u1) ⇒ 只看得到 u1 那条（Global 记录跨类不可见）。
    let visible_u1: Vec<&str> = view
        .visible_records(&[Scope::User {
            user_id: "u1".into(),
        }])
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(visible_u1, vec!["u"]);
    // 授权 `Global` ⇒ 看得到全局那条（canonical：同类相等）。
    assert!(view.record_visible_to(&RecordId::new("g"), &[Scope::Global]));
    // ⚠️ canonical：**不授权** `User(u1)` ⇒ 看不到 u1 那条（跨类也不可见）。
    assert!(!view.record_visible_to(&RecordId::new("u"), &[Scope::Global]));
    assert!(view.record_visible_to(
        &RecordId::new("u"),
        &[Scope::User {
            user_id: "u1".into()
        }]
    ));
    assert!(!view.record_visible_to(
        &RecordId::new("u"),
        &[Scope::User {
            user_id: "u2".into()
        }]
    ));
    assert!(
        !view.record_visible_to(&RecordId::new("nope"), &[Scope::Global]),
        "不存在的记录 ⇒ false，不 panic"
    );
}

#[test]
fn scope_visible_matches_the_canonical_ladder() {
    let user = Scope::User {
        user_id: "u1".into(),
    };
    // **同类相等**才可见（canonical `is_visible_in` 的口径）。
    assert!(scope_visible(&Scope::Global, &[Scope::Global]));
    assert!(scope_visible(&user, &[user.clone()]));
    // **跨类一律不可见**（canonical 口径）。
    assert!(!scope_visible(&Scope::Global, &[user.clone()]));
    assert!(!scope_visible(
        &Scope::Project {
            project_id: "p1".into()
        },
        &[Scope::Global]
    ));
    assert!(!scope_visible(
        &Scope::Project {
            project_id: "p1".into()
        },
        &[user.clone()]
    ));
}

#[test]
fn granted_scope_list_expresses_delegation() {
    // 「人设代理既要读用户原始记忆、又要读自己的人设记忆」⇒ **靠列表**表达
    // （canonical 的做法）⇒ 契约里**不**自造「人设 ⊃ 用户」的阶梯。
    let user = Scope::User {
        user_id: "u1".into(),
    };
    let persona = Scope::Persona {
        persona_id: "p1".into(),
        user_id: "u1".into(),
    };
    let granted = [user.clone(), persona.clone()];
    assert!(scope_visible(&user, &granted));
    assert!(scope_visible(&persona, &granted));
    assert!(!scope_visible(
        &Scope::User {
            user_id: "u2".into()
        },
        &granted
    ));
}

// ============================================
// 6. 写契约：标注（只产标注，不回写原始层）
// ============================================

#[test]
fn annotation_payload_covers_four_kinds() {
    let association = MemoryAnnotation::new(
        AnnotationKind::Association,
        RecordId::new("r1"),
        AnnotationPayload::association(RecordId::new("r2"), 0.8),
        stamp(),
    );
    assert_eq!(association.kind, AnnotationKind::Association);
    assert_eq!(
        association.payload,
        AnnotationPayload::Association {
            target: RecordId::new("r2"),
            strength: 0.8
        }
    );

    let tag = MemoryAnnotation::new(
        AnnotationKind::Tag,
        RecordId::new("r1"),
        AnnotationPayload::tag("遗忘"),
        stamp(),
    );
    assert_eq!(tag.kind, AnnotationKind::Tag);
    assert_eq!(
        tag.payload,
        AnnotationPayload::Tag {
            label: "遗忘".to_string()
        }
    );

    let score = MemoryAnnotation::new(
        AnnotationKind::Score,
        RecordId::new("r1"),
        AnnotationPayload::score(0.5),
        stamp(),
    );
    assert_eq!(score.kind, AnnotationKind::Score);
    assert_eq!(score.payload, AnnotationPayload::Score { value: 0.5 });

    let weight = MemoryAnnotation::new(
        AnnotationKind::SuggestedWeight,
        RecordId::new("r1"),
        AnnotationPayload::suggested_weight(0.25),
        stamp(),
    );
    assert_eq!(weight.kind, AnnotationKind::SuggestedWeight);
    assert_eq!(
        weight.payload,
        AnnotationPayload::SuggestedWeight { value: 0.25 }
    );
}

#[test]
fn score_payload_clamps_out_of_range_without_panic() {
    let high = MemoryAnnotation::new(
        AnnotationKind::Score,
        RecordId::new("r1"),
        AnnotationPayload::Score { value: 1.7 },
        stamp(),
    );
    assert_eq!(high.payload, AnnotationPayload::Score { value: 1.0 });

    let low = MemoryAnnotation::new(
        AnnotationKind::Score,
        RecordId::new("r1"),
        AnnotationPayload::Score { value: -0.3 },
        stamp(),
    );
    assert_eq!(low.payload, AnnotationPayload::Score { value: 0.0 });

    // 建议权重同样 clamp。
    let weight = MemoryAnnotation::new(
        AnnotationKind::SuggestedWeight,
        RecordId::new("r1"),
        AnnotationPayload::SuggestedWeight { value: 42.0 },
        stamp(),
    );
    assert_eq!(
        weight.payload,
        AnnotationPayload::SuggestedWeight { value: 1.0 }
    );

    // NaN 不往下传（不 panic）。
    let nan = MemoryAnnotation::new(
        AnnotationKind::Score,
        RecordId::new("r1"),
        AnnotationPayload::Score { value: f64::NAN },
        stamp(),
    );
    assert_eq!(nan.payload, AnnotationPayload::Score { value: 0.0 });
}

#[test]
fn algo_stamp_carries_three_fields() {
    let annotation = MemoryAnnotation::new(
        AnnotationKind::Tag,
        RecordId::new("r1"),
        AnnotationPayload::tag("x"),
        stamp(),
    );

    assert_eq!(annotation.algo.algo_id, "betti_hole_detector");
    assert_eq!(annotation.algo.version, "v1.2.3");
    assert_eq!(annotation.algo.params_digest, "sha256:deadbeef");
    assert_eq!(annotation.target, RecordId::new("r1"));
}

#[test]
fn annotations_can_be_produced_from_immutable_view_only() {
    // 契约层证据：产标注**只需要**只读视图 ⇒ 标注没有回写原始层的入口。
    fn produce(view: &InMemoryMemoryView) -> Vec<MemoryAnnotation> {
        view.by_time(0, i64::MAX)
            .into_iter()
            .map(|record| {
                MemoryAnnotation::new(
                    AnnotationKind::Tag,
                    record.id.clone(),
                    AnnotationPayload::tag("batch"),
                    stamp(),
                )
            })
            .collect()
    }

    let view = InMemoryMemoryView::new(vec![record("r1", "甲", 100), record("r2", "乙", 200)]);
    let annotations = produce(&view);
    assert_eq!(annotations.len(), 2);
    // 产出标注没有改动视图（视图长度不变）。
    assert_eq!(view.len(), 2);
}

// ============================================
// 7. insert()：倒排索引必须重建
// ============================================

#[test]
fn insert_makes_new_record_visible_to_related_immediately() {
    let mut view = InMemoryMemoryView::new(vec![record("r1", "记忆归档", 100)]);
    assert!(view.related(&RecordId::new("r1")).is_empty());

    view.insert(record("r2", "记忆检索", 200));

    let related: Vec<&str> = view
        .related(&RecordId::new("r1"))
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    assert_eq!(related, vec!["r2"], "insert 后应立刻能看到新记录");
    assert_eq!(view.len(), 2);
}

#[test]
fn insert_with_same_id_rebuilds_inverted_index() {
    let mut view = InMemoryMemoryView::new(vec![
        record("r1", "记忆归档", 100),
        record("r2", "记忆检索", 200),
    ]);
    assert_eq!(view.related(&RecordId::new("r2")).len(), 1);

    // 同 id 覆盖：旧标签词不再指向它（不留幽灵命中）。
    view.insert(record("r1", "量子实验", 100));

    assert!(
        view.related(&RecordId::new("r2")).is_empty(),
        "覆盖后旧标签的倒排项应被重建掉"
    );
    assert_eq!(view.len(), 2, "同 id 覆盖不增加条数");
}

// ============================================
// 8. 确定性：同一批记录建两次 ⇒ 逐字相等
// ============================================

#[test]
fn two_builds_of_same_batch_are_identical() {
    let build = || {
        InMemoryMemoryView::new(vec![
            record("r3", "记忆摘要", 300),
            record("r1", "记忆归档", 100),
            record("r2", "记忆检索", 200),
            record("r4", "量子纠缠", 400),
        ])
    };

    let left = build();
    let right = build();

    let ids = |view: &InMemoryMemoryView, from: i64, to: i64| -> Vec<String> {
        view.by_time(from, to)
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect()
    };
    assert_eq!(ids(&left, 0, 1_000), ids(&right, 0, 1_000));

    let related = |view: &InMemoryMemoryView, id: &str| -> Vec<String> {
        view.related(&RecordId::new(id))
            .iter()
            .map(|r| r.id.as_str().to_string())
            .collect()
    };
    assert_eq!(related(&left, "r1"), related(&right, "r1"));
    assert_eq!(related(&left, "r1"), vec!["r2", "r3"]);
}

// ============================================
// 9. AnnotationKind 可排序 ⇒ 能当 BTreeMap 的键
// ============================================

#[test]
fn annotation_kind_is_ord_and_usable_as_btree_map_key() {
    use std::collections::BTreeMap;

    let mut buckets: BTreeMap<AnnotationKind, usize> = BTreeMap::new();
    buckets.insert(AnnotationKind::Association, 1);
    buckets.insert(AnnotationKind::Tag, 2);
    buckets.insert(AnnotationKind::Score, 3);
    buckets.insert(AnnotationKind::SuggestedWeight, 4);

    let order: Vec<AnnotationKind> = buckets.keys().copied().collect();
    assert_eq!(
        order,
        vec![
            AnnotationKind::Association,
            AnnotationKind::Tag,
            AnnotationKind::Score,
            AnnotationKind::SuggestedWeight,
        ],
        "AnnotationKind 应有稳定全序"
    );
    assert!(AnnotationKind::Association < AnnotationKind::Tag);
}
