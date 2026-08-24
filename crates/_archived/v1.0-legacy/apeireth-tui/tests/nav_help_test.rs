#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 5 nav × Help 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围**:
/// - 6 哲学锚 (S-1 / S-2 / O-2 / O-3 / O-4 / O-5)
/// - 8 项不修改承诺
/// - 1.0 release 文档索引
/// - 5 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: Help 屏服务 ASI 北极星 (哲学可见 → 原则锚定)
/// - S-2 实事求是: 6 锚 hardcode, 跟 v4 §0.2 字面一致
/// - O-2 走在前人肩上: 借 v4 §0.2 沉淀
/// - O-3 干到底: 6 锚 + 8 承诺 + 1.0 release 索引全列
/// - O-4 任何人都能接手: 锚带时间戳, 全部可追溯
/// - O-5 不假装: release 文档标 [stub], 不假装路径
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
/// **8 项承诺**: 全部遵守 (自身是 8 项承诺的载体)
mod test_common;

use ratatui::layout::Rect;
use test_common::{EIGHT_PROMISES_LITERAL, SIX_ANCHORS_IDS};

// =====================================================================
// 1. 6 哲学锚全到 + 跟 test_common 同步
// =====================================================================

#[test]
fn six_anchors_all_present_and_synced() {
    assert_eq!(nav::help::SIX_ANCHORS.len(), 6);
    let codes: Vec<&str> = nav::help::SIX_ANCHORS.iter().map(|(c, _)| *c).collect();
    for required in SIX_ANCHORS_IDS {
        assert!(codes.contains(required), "6 锚应含 {required}");
    }
}

// =====================================================================
// 2. 6 锚 text 互不相同
// =====================================================================

#[test]
fn six_anchors_text_distinct() {
    let texts: Vec<&str> = nav::help::SIX_ANCHORS.iter().map(|(_, t)| *t).collect();
    let unique: std::collections::HashSet<&str> = texts.iter().copied().collect();
    assert_eq!(unique.len(), 6, "6 锚 text 互不相同");
    // 关键哲学词必须出现
    let all: String = texts.join(" | ");
    assert!(all.contains("北极星"), "S-1 应含'北极星'");
    assert!(all.contains("实事求是"), "S-2 应含'实事求是'");
    assert!(all.contains("不假装"), "O-5 应含'不假装'");
}

// =====================================================================
// 3. 8 项不修改承诺 正确计数
// =====================================================================

#[test]
fn eight_promises_count_8() {
    assert_eq!(nav::help::EIGHT_PROMISES.len(), 8);
    // 跟 test_common 8 项字面 substring 同步
    for (i, expected_substr) in EIGHT_PROMISES_LITERAL.iter().enumerate() {
        assert!(
            nav::help::EIGHT_PROMISES[i].contains(expected_substr),
            "EIGHT_PROMISES[{}] 应含 '{}', 实际: '{}'",
            i,
            expected_substr,
            nav::help::EIGHT_PROMISES[i]
        );
    }
}

// =====================================================================
// 4. render 列出全部 6 锚 + 8 承诺
// =====================================================================

#[test]
fn render_lists_all_anchors_and_promises() {
    let area = Rect::new(0, 0, 80, 50);
    let out = nav::help::render(area);
    for (code, _) in nav::help::SIX_ANCHORS {
        assert!(out.contains(code), "render 应含锚 {code}");
    }
    for (i, _) in nav::help::EIGHT_PROMISES.iter().enumerate() {
        // 不在 assert! msg 里用 format 占位符 (没 args 报错), 改用字面字符串
        let needle = format!("{}.", i + 1);
        assert!(out.contains(&needle), "render 应含编号 (1.-8.) 开头");
    }
    assert!(out.contains("HELP"), "render 应有 HELP 标题");
    assert!(out.contains("1.0 Release"), "render 应有 1.0 Release 段");
}

// =====================================================================
// 5. release 文档标 [stub] 诚实
// =====================================================================

#[test]
fn render_marks_release_doc_stub() {
    let area = Rect::new(0, 0, 80, 50);
    let out = nav::help::render(area);
    assert!(
        out.contains("[stub]") || out.contains("stub"),
        "release 文档应标 stub, 不假装路径: {out}"
    );
}
