#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// TUI unit 集成测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试目标** (per 主人派活单 2026-08-05):
/// - 跑 5 nav + 9 器官 dispatcher
/// - 验证 5 nav enum + 9 器官 enum 集成
/// - 验证 nav + organ 渲染产出非空 String
/// - 验证 nav + organ dispatcher 调对应 render 函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 集成测试服务 ASI 北极星 (TUI 端到端 → 平台完整)
/// - S-2 实事求是: 全部断言基于真实 render 输出
/// - O-2 走在前人肩上: 借 mod dispatcher 模式
/// - O-3 干到底: 5+9=14 屏全跑, 跨 nav/organ 集成
/// - O-4 任何人都能接手: 测试名清楚
/// - O-5 不假装: 集成测试不假装接 HTTP, 只调纯函数 dispatcher
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
/// **路径说明** (per 任务诚实标缺):
// - 任务期望 `tests/test_tui_unit_in_process.rs` — **是** 放 tests/ 下, ✓
// - bg 任务的 src/ nav/organ 漏了 main.rs 接入 (mod nav; mod organ; 缺失),
//   本文件用 #[path] include 绕过, 跑 dispatcher 跟单测等价
mod test_common;

use ratatui::layout::Rect;

// =====================================================================
// 1. 5 nav dispatcher 跑全 5 nav, 全部非空
// =====================================================================

#[test]
fn dispatch_5_nav_all_render_non_empty() {
    let area = Rect::new(0, 0, 80, 24);
    for n in 0..=4u8 {
        let nav_enum = nav::Nav::from_u8(n).expect("0-4 valid");
        let out = nav::dispatch_render(nav_enum, area);
        assert!(!out.is_empty(), "{nav_enum:?} render 应非空");
        assert!(
            out.len() > 30,
            "{nav_enum:?} render 字符数太少: {}",
            out.len()
        );
    }
}

// =====================================================================
// 2. 5 nav 渲染产出互不相同
// =====================================================================

#[test]
fn dispatch_5_nav_screens_distinct() {
    let area = Rect::new(0, 0, 80, 24);
    let mut outputs = Vec::new();
    for n in 0..=4u8 {
        let nav_enum = nav::Nav::from_u8(n).unwrap();
        outputs.push(nav::dispatch_render(nav_enum, area));
    }
    let unique: std::collections::HashSet<&String> = outputs.iter().collect();
    assert_eq!(unique.len(), 5, "5 nav 屏应互不相同");
}

// =====================================================================
// 3. 9 器官 dispatcher 跑全 9 organ, 全部非空
// =====================================================================

#[test]
fn dispatch_9_organ_all_render_non_empty() {
    let area = Rect::new(0, 0, 40, 10);
    for n in 0..=8u8 {
        let organ_enum = organ::Organ::from_u8(n).expect("0-8 valid");
        let out = organ::dispatch_render(organ_enum, area);
        assert!(!out.is_empty(), "{organ_enum:?} render 应非空");
        assert!(
            out.len() > 10,
            "{organ_enum:?} render 字符数太少: {}",
            out.len()
        );
    }
}

// =====================================================================
// 4. 9 器官渲染产出互不相同
// =====================================================================

#[test]
fn dispatch_9_organ_screens_distinct() {
    let area = Rect::new(0, 0, 40, 10);
    let mut outputs = Vec::new();
    for n in 0..=8u8 {
        let organ_enum = organ::Organ::from_u8(n).unwrap();
        outputs.push(organ::dispatch_render(organ_enum, area));
    }
    let unique: std::collections::HashSet<&String> = outputs.iter().collect();
    assert_eq!(unique.len(), 9, "9 器官屏应互不相同");
}

// =====================================================================
// 5. 跨 nav/organ 集成: 14 屏 label + 标识都出现
// =====================================================================

#[test]
fn cross_nav_organ_14_screens_labels() {
    let area = Rect::new(0, 0, 80, 24);
    // 5 nav 屏 — 各 nav 应有自己标识
    let nav_labels = ["STATUS", "SESSION", "TOOLS", "SETTINGS", "HELP"];
    for (i, expected) in nav_labels.iter().enumerate() {
        let nav_enum = nav::Nav::from_u8(i as u8).unwrap();
        let out = nav::dispatch_render(nav_enum, area);
        assert!(
            out.contains(expected),
            "nav[{i}] render 应含 '{expected}': {out}"
        );
    }
    // 9 器官屏 — 各 organ 应有自己 ASCII label
    let organ_ascii = [
        "[♥]", "[BRAIN]", "[HAND]", "[EYE]", "[EAR]", "[MEM]", "[VOICE]", "[BODY]", "[MIND]",
    ];
    let organ_area = Rect::new(0, 0, 40, 10);
    for (i, expected) in organ_ascii.iter().enumerate() {
        let organ_enum = organ::Organ::from_u8(i as u8).unwrap();
        let out = organ::dispatch_render(organ_enum, organ_area);
        assert!(
            out.contains(expected),
            "organ[{i}] render 应含 '{expected}': {out}"
        );
    }
}
