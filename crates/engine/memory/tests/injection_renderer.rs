//! 记忆纵深体系**注入契约**（`injection_renderer`，设计稿 §5 ③）集成测试。
//!
//! 钉住几件事（每条都是「不许悄悄变」的判据）：
//! 1. **三个形态都能渲染**：`Plain` / `Bulleted` / `Conclusions` 各归各的变体；
//! 2. **预算纪律**：产物的 `char_len()` **永远**不超过 `budget.max_chars`
//!    （逐个预算值测：小预算 / 恰好够 / 不够）；
//! 3. **装不下就说装不下**：预算为 0 / 第一条就超预算 ⇒ `Injection::Empty`
//!    （**不硬塞超预算**、不 panic）；
//! 4. **如实标注**：被截断的内容带「已截断」（不静默截断）；
//! 5. **确定性**：按 `relevance` 降序装入（同分按 `id` 升序）⇒ 同输入两次**逐字相等**；
//! 6. **形态可换**：同一个契约后面挂着三个**可换**的实现（`&dyn InjectionRenderer`）。

use apeireth_memory::injection_renderer::{
    Budget, BulletedRenderer, ConclusionsRenderer, Evidence, Injection, InjectionRenderer,
    PlainRenderer, ANTI_HALLUCINATION_NOTE, TRUNCATION_MARKER,
};

/// 造一条证据（便捷函数）。
fn ev(id: &str, text: &str, attribution: &str, relevance: f64) -> Evidence {
    Evidence::new(id, text, attribution, relevance)
}

/// 三条样例证据（相关度递减、id 递增 ⇒ 排序口径可预期）。
fn sample() -> Vec<Evidence> {
    vec![
        ev("e1", "用户喜欢用中文回答问题。", "用户原话", 0.9),
        ev("e2", "项目代号是 Orion。", "2026-10-25 的提交", 0.7),
        ev("e3", "交付时间是下周一。", "检索命中", 0.5),
    ]
}

/// 三个渲染器（统一成 trait 引用 ⇒ 证明契约真能换实现）。
fn renderers() -> Vec<(&'static str, Box<dyn InjectionRenderer>)> {
    vec![
        ("plain", Box::new(PlainRenderer::new())),
        ("bulleted", Box::new(BulletedRenderer)),
        ("conclusions", Box::new(ConclusionsRenderer)),
    ]
}

/// 取纯文本形态的文本（其它形态 ⇒ panic，测试里只在该用的时候用）。
fn plain_text(injection: &Injection) -> &str {
    match injection {
        Injection::Plain { text } => text.as_str(),
        other => panic!("期望 Plain 形态，实得 {other:?}"),
    }
}

/// 取结构化条目的各行。
fn bulleted_lines(injection: &Injection) -> &[String] {
    match injection {
        Injection::Bulleted { lines } => lines.as_slice(),
        other => panic!("期望 Bulleted 形态，实得 {other:?}"),
    }
}

/// 取结论形态的各条。
fn conclusion_items(injection: &Injection) -> &[String] {
    match injection {
        Injection::Conclusions { items } => items.as_slice(),
        other => panic!("期望 Conclusions 形态，实得 {other:?}"),
    }
}

// ============================================
// 1. 三个渲染器都能渲染（各自的变体正确）
// ============================================

#[test]
fn plain_renderer_renders_attribution_and_content_per_line() {
    let injection = PlainRenderer::new().render(&sample(), Budget::new(1_000, 0));

    let text = plain_text(&injection);
    assert!(
        text.contains("[用户原话] 用户喜欢用中文回答问题。"),
        "{text}"
    );
    assert!(
        text.contains("[2026-10-25 的提交] 项目代号是 Orion。"),
        "{text}"
    );
    assert!(text.contains("[检索命中] 交付时间是下周一。"), "{text}");
    assert_eq!(injection.item_count(), 3, "三条证据 ⇒ 三个条目");
}

#[test]
fn plain_renderer_default_carries_the_anti_hallucination_rule() {
    let injection = PlainRenderer::new().render(&sample(), Budget::new(1_000, 0));

    let text = plain_text(&injection);
    assert!(text.contains("禁止说「我记得我们以前聊过」"), "{text}");
    assert_eq!(
        injection.item_count(),
        3,
        "反幻觉提示是规则，不是一条证据 ⇒ 不计入条目数"
    );
}

#[test]
fn plain_renderer_without_note_drops_the_rule() {
    let injection = PlainRenderer::without_note().render(&sample(), Budget::new(1_000, 0));

    let text = plain_text(&injection);
    assert!(!text.contains("禁止说"), "关掉开关就不该有规则：{text}");
    assert!(!text.contains(ANTI_HALLUCINATION_NOTE), "{text}");
    assert_eq!(injection.item_count(), 3);
}

#[test]
fn plain_renderer_omits_the_rule_entirely_when_it_does_not_fit() {
    // 预算比整条规则还短 ⇒ **整体省略**规则（不截断规则本身：半条规则更危险）。
    let budget = Budget::new(ANTI_HALLUCINATION_NOTE.chars().count() - 1, 0);
    let injection = PlainRenderer::new().render(&sample(), budget);

    let text = plain_text(&injection);
    assert!(!text.contains("规则:"), "装不下就不该留半条规则：{text}");
    assert!(
        !text.contains(ANTI_HALLUCINATION_NOTE),
        "装不下就不该留半条规则：{text}"
    );
    assert!(injection.char_len() <= budget.max_chars);
}

#[test]
fn bulleted_renderer_renders_content_and_attribution_per_line() {
    let injection = BulletedRenderer.render(&sample(), Budget::new(1_000, 0));

    let lines = bulleted_lines(&injection);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "用户喜欢用中文回答问题。 —— 用户原话");
    assert_eq!(lines[1], "项目代号是 Orion。 —— 2026-10-25 的提交");
    assert_eq!(lines[2], "交付时间是下周一。 —— 检索命中");
}

#[test]
fn conclusions_renderer_takes_the_first_sentence_only() {
    let evidence = vec![
        ev("e1", "第一句结论。第二句补充说明。", "用户原话", 0.9),
        ev("e2", "另一条结论！还有后半段。", "检索命中", 0.5),
    ];
    let injection = ConclusionsRenderer.render(&evidence, Budget::new(1_000, 0));

    let items = conclusion_items(&injection);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], "第一句结论。");
    assert_eq!(items[1], "另一条结论！");
}

// ============================================
// 2. 预算纪律：char_len() 永远不超过 max_chars
// ============================================

#[test]
fn budget_discipline_never_exceeds_max_chars() {
    let evidence = sample();
    // 逐个预算值：太小 / 勉强 / 恰好 / 宽裕。
    for max_chars in [0usize, 1, 5, 12, 20, 40, 61, 100, 1_000] {
        for (name, renderer) in renderers() {
            let budget = Budget::new(max_chars, 0);
            let injection = renderer.render(&evidence, budget);
            assert!(
                injection.char_len() <= max_chars,
                "{name} 在预算 {max_chars} 下超预算：{}",
                injection.char_len()
            );
        }
    }
}

#[test]
fn exact_budget_fits_every_item_without_truncation() {
    let evidence = sample();
    let full = BulletedRenderer.render(&evidence, Budget::new(10_000, 0));
    let joined = bulleted_lines(&full).join("\n");
    let exact = joined.chars().count();

    // 恰好够 ⇒ 三条全装下，且**没有**截断标注。
    let injection = BulletedRenderer.render(&evidence, Budget::new(exact, 0));
    assert_eq!(injection.item_count(), 3, "恰好够 ⇒ 三条都装下");
    assert_eq!(injection.char_len(), exact, "恰好够 ⇒ 用满预算");
    assert!(
        !bulleted_lines(&injection)
            .iter()
            .any(|line| line.contains("已截断")),
        "恰好够就不该有截断标注"
    );
}

#[test]
fn budget_too_small_returns_empty_instead_of_overflowing() {
    // 预算装不下第一条 ⇒ 如实说装不下（不硬塞超预算）。
    let injection = PlainRenderer::new().render(&sample(), Budget::new(5, 0));

    match &injection {
        Injection::Empty { reason } => {
            assert!(reason.contains("预算"), "原因要说清是预算问题：{reason}");
        }
        other => panic!("装不下应返回 Empty，实得 {other:?}"),
    }
    assert_eq!(injection.char_len(), 0, "Empty 没有注入任何内容");
    assert_eq!(injection.item_count(), 0);
}

// ============================================
// 3. max_items 生效
// ============================================

#[test]
fn max_items_caps_the_number_of_injected_items() {
    let injection = BulletedRenderer.render(&sample(), Budget::new(10_000, 1));
    assert_eq!(injection.item_count(), 1, "max_items = 1 ⇒ 只装一条");

    let injection = BulletedRenderer.render(&sample(), Budget::new(10_000, 2));
    assert_eq!(injection.item_count(), 2, "max_items = 2 ⇒ 只装两条");
}

#[test]
fn max_items_zero_means_unlimited() {
    let injection = BulletedRenderer.render(&sample(), Budget::new(10_000, 0));
    assert_eq!(injection.item_count(), 3, "max_items = 0 ⇒ 不限条数");
}

// ============================================
// 4. 排序口径：relevance 降序（同分按 id 升序）
// ============================================

#[test]
fn items_are_loaded_by_relevance_descending() {
    let evidence = vec![
        ev("id-b", "内容B", "出处B", 0.3),
        ev("id-a", "内容A", "出处A", 0.9),
        ev("id-c", "内容C", "出处C", 0.6),
    ];
    let injection = BulletedRenderer.render(&evidence, Budget::new(10_000, 0));

    let lines = bulleted_lines(&injection);
    assert_eq!(
        lines,
        vec!["内容A —— 出处A", "内容C —— 出处C", "内容B —— 出处B"],
        "高相关的先装"
    );
}

#[test]
fn equal_relevance_is_ordered_by_id_ascending() {
    // ⚠️ 输入顺序刻意**反过来** ⇒ 结果只可能由 id 决定（确定性）。
    let evidence = vec![
        ev("id-b", "内容B", "出处B", 0.5),
        ev("id-a", "内容A", "出处A", 0.5),
    ];
    let injection = BulletedRenderer.render(&evidence, Budget::new(10_000, 0));

    let lines = bulleted_lines(&injection);
    assert_eq!(
        lines,
        vec!["内容A —— 出处A", "内容B —— 出处B"],
        "同分按 id 升序"
    );
}

// ============================================
// 5. 空证据 / 零预算 / 第一条就超预算 ⇒ Empty
// ============================================

#[test]
fn empty_evidence_returns_empty_injection() {
    for (name, renderer) in renderers() {
        let injection = renderer.render(&[], Budget::new(1_000, 0));
        match &injection {
            Injection::Empty { reason } => {
                assert!(!reason.is_empty(), "{name}：原因不能是空串");
            }
            other => panic!("{name}：空证据应返回 Empty，实得 {other:?}"),
        }
        assert_eq!(injection.item_count(), 0);
        assert_eq!(injection.char_len(), 0);
    }
}

#[test]
fn zero_budget_returns_empty_with_reason_without_panic() {
    for (name, renderer) in renderers() {
        let injection = renderer.render(&sample(), Budget::new(0, 0));
        match &injection {
            Injection::Empty { reason } => {
                assert!(
                    reason.contains("预算"),
                    "{name}：原因要说清是预算问题：{reason}"
                );
            }
            other => panic!("{name}：零预算应返回 Empty，实得 {other:?}"),
        }
    }
}

#[test]
fn first_evidence_over_budget_returns_empty_instead_of_hard_fitting() {
    // 第一条就放不下 ⇒ 明确说「第一条证据就超出预算」（不硬塞超预算）。
    let injection = BulletedRenderer.render(&sample(), Budget::new(3, 0));

    match &injection {
        Injection::Empty { reason } => {
            assert_eq!(reason, "第一条证据就超出预算");
        }
        other => panic!("第一条就超预算应返回 Empty，实得 {other:?}"),
    }
}

// ============================================
// 6. 截断：如实标注 + 刚好用完预算
// ============================================

#[test]
fn truncated_output_is_marked_as_truncated() {
    let evidence = sample();
    let full = BulletedRenderer.render(&evidence, Budget::new(10_000, 0));
    let first = bulleted_lines(&full)[0].clone();
    let second = bulleted_lines(&full)[1].clone();
    let first_len = first.chars().count();

    // 预算 = 第一条 + 换行 + 10 个字符 ⇒ 第二条放不下 ⇒ **截断并标注**。
    let budget = first_len + 1 + 10;
    let injection = BulletedRenderer.render(&evidence, Budget::new(budget, 0));

    let lines = bulleted_lines(&injection);
    assert_eq!(lines.len(), 2, "第一条装下，第二条被截断");
    assert!(
        lines[1].contains("已截断"),
        "被截断必须如实标注：{:?}",
        lines[1]
    );
    assert!(lines[1].ends_with(TRUNCATION_MARKER), "{:?}", lines[1]);
    // 截断内容确实是**第二条的前缀**（不是别的东西），且刚好用完预算。
    let prefix: String = second.chars().take(4).collect();
    assert_eq!(lines[1], format!("{prefix}{TRUNCATION_MARKER}"));
    assert_eq!(injection.char_len(), budget, "截断应**刚好**用完预算");
}

#[test]
fn truncated_output_fills_the_budget_exactly() {
    let evidence = sample();
    let exact = bulleted_lines(&BulletedRenderer.render(&evidence, Budget::new(10_000, 0)))
        .join("\n")
        .chars()
        .count();

    for max_chars in [exact - 1, exact - 5, exact - 20] {
        let injection = BulletedRenderer.render(&evidence, Budget::new(max_chars, 0));
        assert_eq!(
            injection.char_len(),
            max_chars,
            "截断应**刚好**用完预算，不多不少"
        );
    }
}

// ============================================
// 7. 结论形态：更小、不贴原文
// ============================================

#[test]
fn conclusions_never_contain_the_full_text() {
    let evidence = vec![ev(
        "e1",
        "第一句结论。第二句补充说明。第三句更多细节。",
        "用户原话",
        0.9,
    )];
    let injection = ConclusionsRenderer.render(&evidence, Budget::new(1_000, 0));

    let items = conclusion_items(&injection);
    assert_eq!(items, vec!["第一句结论。".to_string()]);
    assert!(
        !items[0].contains("第二句"),
        "结论形态不贴原文：{:?}",
        items[0]
    );
}

#[test]
fn conclusions_fit_more_items_under_the_same_budget() {
    let long_tail = "后面还有很长的一段补充说明，占用预算空间，需要被截断才装得下更多内容。";
    let evidence = vec![
        ev("e1", &format!("短结论。{long_tail}"), "用户原话", 0.9),
        ev("e2", &format!("短结论。{long_tail}"), "用户原话", 0.7),
        ev("e3", &format!("短结论。{long_tail}"), "用户原话", 0.5),
    ];
    let budget = Budget::new(120, 0);

    let plain = PlainRenderer::new().render(&evidence, budget);
    let conclusions = ConclusionsRenderer.render(&evidence, budget);

    assert!(
        conclusions.item_count() > plain.item_count(),
        "同预算下结论形态应装更多条：{} vs {}",
        conclusions.item_count(),
        plain.item_count()
    );
    assert!(conclusions.char_len() <= budget.max_chars);
    assert!(plain.char_len() <= budget.max_chars);
}

// ============================================
// 8. Evidence::new 的 clamp
// ============================================

#[test]
fn evidence_new_clamps_relevance_out_of_range() {
    assert_eq!(ev("a", "甲", "出处", 1.7).relevance, 1.0);
    assert_eq!(ev("a", "甲", "出处", -0.3).relevance, 0.0);
    assert_eq!(ev("a", "甲", "出处", f64::NAN).relevance, 0.0);
    // 区间内的值**原样**保留。
    assert_eq!(ev("a", "甲", "出处", 0.42).relevance, 0.42);
}

// ============================================
// 9. char_len / item_count 对四个变体都正确
// ============================================

#[test]
fn char_len_and_item_count_are_correct_for_all_four_variants() {
    let plain = Injection::Plain {
        text: "abc".to_string(),
    };
    assert_eq!(plain.char_len(), 3);
    assert_eq!(plain.item_count(), 1);

    let plain_with_note = Injection::Plain {
        text: format!("abc\n{ANTI_HALLUCINATION_NOTE}"),
    };
    assert_eq!(
        plain_with_note.char_len(),
        3 + 1 + ANTI_HALLUCINATION_NOTE.chars().count()
    );
    assert_eq!(plain_with_note.item_count(), 1, "提示行不算证据条目");

    let bulleted = Injection::Bulleted {
        lines: vec!["a".to_string(), "bb".to_string()],
    };
    assert_eq!(bulleted.char_len(), 4, "2 + 1（换行）+ 1");
    assert_eq!(bulleted.item_count(), 2);

    let conclusions = Injection::Conclusions {
        items: vec!["x".to_string(), "yy".to_string()],
    };
    assert_eq!(conclusions.char_len(), 4, "1 + 1（换行）+ 2");
    assert_eq!(conclusions.item_count(), 2);

    let empty = Injection::Empty {
        reason: "预算为 0".to_string(),
    };
    assert_eq!(empty.char_len(), 0, "Empty 没有注入任何内容");
    assert_eq!(empty.item_count(), 0);
}

// ============================================
// 10. 确定性 + 形态可换
// ============================================

#[test]
fn rendering_is_deterministic_across_calls() {
    let evidence = sample();
    for (name, renderer) in renderers() {
        let budget = Budget::new(120, 0);
        let first = renderer.render(&evidence, budget);
        let second = renderer.render(&evidence, budget);
        assert_eq!(first, second, "{name}：同一输入两次渲染应逐字相等");
    }
}

#[test]
fn renderers_are_interchangeable_behind_the_contract() {
    // 同一个契约后面挂着三个**可换**的实现 ⇒ 换形态不改调用方。
    let evidence = sample();
    let budget = Budget::new(1_000, 0);

    let plain = PlainRenderer::new().render(&evidence, budget);
    let bulleted = BulletedRenderer.render(&evidence, budget);
    let conclusions = ConclusionsRenderer.render(&evidence, budget);

    assert!(matches!(plain, Injection::Plain { .. }));
    assert!(matches!(bulleted, Injection::Bulleted { .. }));
    assert!(matches!(conclusions, Injection::Conclusions { .. }));
    // 三者都装下了同样的三条（预算宽裕 ⇒ 形态差异只影响体量，不影响条目数）。
    assert_eq!(plain.item_count(), 3);
    assert_eq!(bulleted.item_count(), 3);
    assert_eq!(conclusions.item_count(), 3);
}
