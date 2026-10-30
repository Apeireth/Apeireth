//! **`NumberedEvidenceRenderer` 与生产注入的逐字等价**（B2 接线的安全网）。
//!
//! # 为什么要有这个测试
//!
//! 设计稿 §5 ③ 把注入上提为契约 [`InjectionRenderer`]，但生产路径（`context_compiler`）
//! 一直直接调 `memory_injection::build_memory_injection_with_budget`
//! ⇒ 契约是**库级**的。
//!
//! ⇒ 本测试**先证明**新渲染器能**逐字复现**今天的输出
//! ⇒ ⇒ 接线当天**行为零变化**（否则内部卷 / 小说语料基线会动 ⇒ 那是行为变更，不是重构）。
//!
//! # 覆盖的三处差异（实测出来的，不是猜的）
//!
//! | | 生产 | [`PlainRenderer`] |
//! |---|---|---|
//! | 标题行 | **有** | 无 |
//! | 条目行 | `1. 内容` | `[出处] 内容` |
//! | 规则措辞 | 「以上**编号**条目」 | 「以上条目」 |
//!
//! # 用法
//!
//! ```sh
//! cargo test -p apeireth-memory --test injection_renderer_production --locked
//! ```

use apeireth_memory::injection_renderer::{
    Budget, Evidence, Injection, InjectionRenderer, NumberedEvidenceRenderer,
};
use apeireth_memory::memory_injection::{build_memory_injection_with_budget, EVIDENCE_MAX_CHARS};

/// 把证据组装成生产函数要的「纯文本行」形态。
fn entries_of(evidence: &[Evidence]) -> Vec<String> {
    evidence.iter().map(|item| item.text.clone()).collect()
}

/// 取渲染结果里的纯文本（`Bulleted` / `Conclusions` 形态不参与本测试的逐字比对）。
fn plain_text(injection: &Injection) -> String {
    match injection {
        Injection::Plain { text } => text.clone(),
        Injection::Empty { .. } => String::new(),
        other => panic!("本测试只比对 Plain 形态，得到 {other:?}"),
    }
}

/// 核心等价断言：渲染器输出 **==** 生产函数输出（**逐字节**）。
fn assert_byte_identical(evidence: &[Evidence], per_entry_chars: usize) {
    let renderer = NumberedEvidenceRenderer::new().with_per_entry_chars(per_entry_chars);
    let rendered = plain_text(&renderer.render(evidence, Budget::new(100_000, 0)));
    let production = build_memory_injection_with_budget(&entries_of(evidence), per_entry_chars);
    assert_eq!(
        rendered, production,
        "渲染器输出与生产**不等价** ⇒ 接线会改变行为（这不是重构）"
    );
}

#[test]
fn 生产形态_与今天逐字一致_单条() {
    let evidence = vec![Evidence::new("r1", "用户说他喜欢 Rust", "记忆命中", 1.0)];
    assert_byte_identical(&evidence, EVIDENCE_MAX_CHARS);
}

#[test]
fn 生产形态_与今天逐字一致_多条() {
    let evidence = vec![
        Evidence::new("r1", "第一条：项目用 Rust 重写", "记忆命中", 0.9),
        Evidence::new("r2", "第二条：性能比 Python 好", "记忆命中", 0.8),
        Evidence::new("r3", "第三条：团队决定先做记忆层", "检索命中", 0.7),
    ];
    assert_byte_identical(&evidence, EVIDENCE_MAX_CHARS);
}

#[test]
fn 生产形态_与今天逐字一致_超长条目被截断() {
    let long = "长".repeat(EVIDENCE_MAX_CHARS * 3 + 17);
    let evidence = vec![
        Evidence::new("r1", &long, "记忆命中", 1.0),
        Evidence::new("r2", "短条目", "记忆命中", 0.5),
    ];
    assert_byte_identical(&evidence, EVIDENCE_MAX_CHARS);
}

#[test]
fn 生产形态_与今天逐字一致_多字节与换行() {
    let evidence = vec![
        Evidence::new("r1", "带换行的\n正文\t与制表", "记忆命中", 1.0),
        Evidence::new("r2", "emoji 🦀 与中文标点，。；：", "记忆命中", 0.5),
    ];
    assert_byte_identical(&evidence, EVIDENCE_MAX_CHARS);
}

#[test]
fn 生产形态_单条上限可配_且与生产一致() {
    let evidence = vec![Evidence::new("r1", &"x".repeat(500), "记忆命中", 1.0)];
    for cap in [0usize, 1, 40, 120, 400] {
        assert_byte_identical(&evidence, cap);
    }
}

#[test]
fn 生产形态_保持输入顺序_不按相关度重排() {
    // ⚠️ 相关度**故意倒序** ⇒ 若渲染器按 relevance 重排，顺序会变 ⇒ 测试会红。
    let evidence = vec![
        Evidence::new("r1", "第一条（相关度最低）", "记忆命中", 0.1),
        Evidence::new("r2", "第二条（相关度最高）", "记忆命中", 0.9),
        Evidence::new("r3", "第三条（相关度居中）", "记忆命中", 0.5),
    ];
    let renderer = NumberedEvidenceRenderer::new();
    let text = plain_text(&renderer.render(&evidence, Budget::new(100_000, 0)));
    let first = text.find("第一条").expect("第一条应在");
    let second = text.find("第二条").expect("第二条应在");
    let third = text.find("第三条").expect("第三条应在");
    assert!(
        first < second && second < third,
        "顺序被改动了 ⇒ 生产顺序 = 检索顺序，渲染层不得再排一次：\n{text}"
    );
}

#[test]
fn 生产形态_标题与规则措辞_不许被统一() {
    // ⚠️ 这条是**回归钉子**：生产规则说的是「以上**编号**条目」，
    // 而通用常量 `ANTI_HALLUCINATION_NOTE` 说的是「以上条目」
    // ⇒ ⇒ 两个措辞**必须不同**；谁把生产改成通用措辞 ⇒ 本测试变红。
    let renderer = NumberedEvidenceRenderer::new();
    let text = plain_text(&renderer.render(
        &[Evidence::new("r1", "内容", "记忆命中", 1.0)],
        Budget::new(100_000, 0),
    ));
    assert!(
        text.contains("以上编号条目"),
        "生产规则措辞被改动了 ⇒ 今天的注入文本会变：\n{text}"
    );
    assert!(
        text.starts_with("[记忆证据 — 你只知道以下条目, 不要声称记得列表之外的任何对话]"),
        "标题行不见了 ⇒ 今天的注入文本会变：\n{text}"
    );
}

#[test]
fn 生产形态_空证据_不产出文本() {
    let renderer = NumberedEvidenceRenderer::new();
    let injection = renderer.render(&[], Budget::new(100_000, 0));
    assert!(
        matches!(injection, Injection::Empty { .. }),
        "空证据应产出 Empty（而不是空文本）⇒ 得到了 {injection:?}"
    );
}

#[test]
fn 生产形态_预算为零_不产出文本() {
    let renderer = NumberedEvidenceRenderer::new();
    let injection = renderer.render(
        &[Evidence::new("r1", "内容", "记忆命中", 1.0)],
        Budget::new(0, 0),
    );
    assert!(
        matches!(injection, Injection::Empty { .. }),
        "预算为 0 应产出 Empty ⇒ 得到了 {injection:?}"
    );
}
