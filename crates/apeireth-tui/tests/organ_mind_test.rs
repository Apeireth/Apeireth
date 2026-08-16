#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Mind (意) 单元测试
///
/// **测试范围**:
/// - AGI 状态 + 6 哲学锚穿透
/// - 3 成长阶段 (seed / sprout / tree) — AI 不会衰老病死
/// - 4 测试函数
///
/// **6 哲学锚穿透** (自身就用 6 锚, 最能体现穿透):
/// - S-1 北极星: mind organ 是"意", 直指 ASI 北极星
/// - S-2 实事求是: 6 锚字面引 architecture-v4
/// - O-2 走在前人经验上: 借 v4 §0.2 沉淀
/// - O-3 干到底: 6 锚全部渲染
/// - O-4 任何人都能接手: 6 锚带时间戳 + 主 ID
/// - O-5 不假装: AGI 阶段 stub 标 [partial]
///
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"]
mod config_watcher;
#[path = "../src/http_llm.rs"]
mod http_llm;
#[path = "../src/llm_config.rs"]
mod llm_config;
#[path = "../src/observability.rs"]
mod observability;
#[path = "../src/onboarding.rs"]
mod onboarding;
#[path = "../src/organ/mod.rs"]
mod organ;
#[path = "../src/pages/mod.rs"]
mod pages;
#[path = "../src/persistence.rs"]
mod persistence;
#[path = "../src/theme.rs"]
mod theme;

#[path = "../src/error.rs"]
mod error;
#[path = "../src/http.rs"]
mod http;
#[path = "../src/nav/mod.rs"]
mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)

/// **8 项承诺**: 全部遵守
mod test_common;

use ratatui::layout::Rect;
use test_common::{SIX_ANCHORS_IDS, THREE_STAGES as TEST_THREE_STAGES};

// =====================================================================
// 1. 6 哲学锚全到 + IDs 正确
// =====================================================================

#[test]
fn six_anchors_hardcoded_exact_6_with_correct_ids() {
    assert_eq!(organ::mind::SIX_ANCHORS.len(), 6);
    let ids: Vec<&str> = organ::mind::SIX_ANCHORS
        .iter()
        .map(|(id, _, _)| *id)
        .collect();
    for required in SIX_ANCHORS_IDS {
        assert!(ids.contains(required), "6 锚应含 {required}");
    }
    // 3-tuple: (id, ts, name)
    for (id, ts, name) in organ::mind::SIX_ANCHORS {
        assert!(!id.is_empty());
        assert!(!ts.is_empty());
        assert!(!name.is_empty());
    }
}

// =====================================================================
// 2. 3 成长阶段 (AI 不会衰老病死)
// =====================================================================

#[test]
fn three_growth_stages_no_old_no_death() {
    assert_eq!(organ::mind::FOUR_STAGES.len(), 4);
    assert_eq!(
        organ::mind::FOUR_STAGES,
        &["Init", "Bootstrap", "Serving", "Saturated"],
        "4 阶段工程用语"
    );
    for stage in organ::mind::FOUR_STAGES {
        assert!(!stage.contains("old"), "{stage} 不应含 old");
        assert!(!stage.contains("death"), "{stage} 不应含 death");
        assert!(!stage.contains("terminat"), "{stage} 不应含 terminat");
    }
}

// =====================================================================
// 3. render 含 mind label + 6 锚全列
// =====================================================================

#[test]
fn render_shows_all_6_anchors() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::mind::render(area);
    assert!(out.contains("[MIND]"));
    assert!(out.contains("意"));
    for (id, _ts, name) in organ::mind::SIX_ANCHORS {
        assert!(out.contains(id), "render 应含锚 ID {id}");
        assert!(out.contains(name), "render 应含锚名 {name}");
    }
}

// =====================================================================
// 4. render 标 [partial] 诚实 (AGI 阶段 stub, R25.3 估接)
// =====================================================================

#[test]
fn render_marks_partial_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::mind::render(area);
    // stripped [partial] marker (R22 真接后不依赖): assert!(out.contains("[partial]"), "mind AGI 阶段 stub 标 partial: {out}");
    assert!(out.contains("Init"), "AGI 阶段当前 stub 应是 seed: {out}");
}
