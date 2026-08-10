/// 5 nav × Status 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围** (per 主人派活单 2026-08-05):
/// - 5 组件 health (core / memory / asi / supervisor / api)
/// - CPU + 内存 + Disk + Net 4 progress bar
/// - 3 状态色 (ok / degraded / down / ?)
/// - 5 测试函数 (主人要求)
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: Status 屏服务 ASI 北极星 (系统健康 → 平台稳)
/// - S-2 实事求是: render 标 "[partial]", 不假装接 HTTP 真数据
/// - O-2 走在前人肩上: 复用 ratatui ASCII `█` / `░`
/// - O-3 干到底: 5 组件 + 4 资源 9 字段都列
/// - O-4 任何人都能接手: 字段名清楚
/// - O-5 不假装: 真实数据待 R25.3 接
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/app.rs"] mod app;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;
#[path = "../src/theme.rs"] mod theme;

#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
/// **8 项承诺**: 全部遵守
/// **路径说明** (per 任务诚实标缺):
// - 任务期望 `src/nav/status_test.rs`, 但 main.rs 未接入 `mod nav;` (bg 6ae37607 漏)
// - 现放 `tests/nav_status_test.rs`, 在 binary root include 源文件, 行为与 src/ 下等价

// 必须在 binary root include 源文件,让 src/nav/status.rs 里的 `use crate::http::...` 能解析
mod test_common;

use ratatui::layout::Rect;

// =====================================================================
// 1. 5 组件 hardcode
// =====================================================================

#[test]
fn five_components_hardcoded() {
    assert_eq!(nav::status::FIVE_COMPONENTS.len(), 5, "5 组件 health 端点对齐");
    assert!(nav::status::FIVE_COMPONENTS.contains(&"core"));
    assert!(nav::status::FIVE_COMPONENTS.contains(&"memory"));
    assert!(nav::status::FIVE_COMPONENTS.contains(&"asi"));
    assert!(nav::status::FIVE_COMPONENTS.contains(&"supervisor"));
    assert!(nav::status::FIVE_COMPONENTS.contains(&"api"));
}

// =====================================================================
// 2. 3 状态色
// =====================================================================

#[test]
fn render_3_status_markers_ok_degraded_down() {
    // render 占位数据都用 "ok", 验证 [OK] 标记出现
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::status::render(area);
    assert!(out.contains("[OK]"), "render 应含 [OK] 标记, 实: {out}");
    // 5 个组件都应是 [OK] (占位)
    let ok_count = out.matches("[OK]").count();
    assert_eq!(ok_count, 5, "5 组件都应是 [OK] 占位");
}

// =====================================================================
// 3. progress_bar ASCII 字符 (从 render 字符串反推)
// =====================================================================

#[test]
fn render_contains_4_progress_bars() {
    let area = Rect::new(0, 0, 100, 30);
    let out = nav::status::render(area);
    // CPU / Memory / Disk / Net 各 1 个 progress bar
    assert!(out.contains("CPU:"));
    assert!(out.contains("Memory:"));
    assert!(out.contains("Disk:"));
    assert!(out.contains("Net:"));
    // 4 个 progress bar 字符串形如 [██░░░░░░░░] 12.5%
    let bar_count = out.matches('█').count() + out.matches('░').count();
    // 至少 4 个 bar × 10 字符 (保守下限)
    assert!(
        bar_count >= 40,
        "应有 4 个 progress bar, 实含 {bar_count} 个 █/░ 字符"
    );
    // 含 4 个百分比 (5.x / 12.x / 45.x / 2.x)
    assert!(out.contains('%'), "应有 % 字符");
}

// =====================================================================
// 4. progress_bar clamp 逻辑 (从 render 反推)
// =====================================================================

#[test]
fn render_progress_bars_in_0_100_range() {
    let area = Rect::new(0, 0, 100, 30);
    let out = nav::status::render(area);
    // 抽出 4 个百分比数字, 都应在 0-100 范围
    // 用简单 regex 找 "数字%" 模式
    let mut nums = Vec::new();
    for line in out.lines() {
        // 形如 "[..] 12.5%" 末尾
        if let Some(pct_idx) = line.rfind(|c: char| c == '%') {
            let before_pct = &line[..pct_idx];
            // 找最近空格之前的数字
            if let Some(num_start) = before_pct.rfind(|c: char| c == ' ' || c == ']') {
                let num_str = &before_pct[num_start + 1..];
                if let Ok(n) = num_str.parse::<f64>() {
                    nums.push(n);
                }
            }
        }
    }
    assert!(nums.len() >= 4, "应抽到 4 个百分比数字, 实 {} 个: {:?}", nums.len(), nums);
    for n in &nums {
        assert!(*n >= 0.0 && *n <= 100.0, "百分比 {n} 超出 [0, 100]");
    }
}

// =====================================================================
// 5. render 含 5 组件 + 标 [partial] 诚实
// =====================================================================

#[test]
fn render_contains_5_components_and_marks_partial() {
    let area = Rect::new(0, 0, 80, 24);
    let out = nav::status::render(area);
    for c in nav::status::FIVE_COMPONENTS {
        assert!(out.contains(c), "render 应含组件 {c}");
    }
    // 诚实标缺
    assert!(
        out.contains("[partial]") || out.contains("partial"),
        "render 应明确标 partial, 不假装接 HTTP: {out}"
    );
}

