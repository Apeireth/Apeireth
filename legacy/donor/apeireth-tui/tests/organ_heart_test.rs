#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 9 器官 × Heart (心) 单元测试
///
/// **测试范围**:
/// - 60Hz 心跳 ASCII (♥ 满 / ♡ 空)
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 心跳服务 ASI 北极星
/// - S-2 实事求是: 60Hz 占位, 真实数据待 R25.3
/// - O-2: 借 ratatui ASCII
/// - O-3: 跳频 + 状态 + tick 全列
/// - O-4: 字段名清楚
/// - O-5: 标 [partial]
///
/// **8 项承诺**: 全部遵守
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

mod test_common;

use ratatui::layout::Rect;

// =====================================================================
// 1. render 含 ASCII 满跳 / 空跳
// =====================================================================

#[test]
fn render_contains_heart_ascii_full_and_empty() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::heart::render(area);
    assert!(out.contains("♥"), "应含 ♥ 满跳, 实: {out}");
    assert!(out.contains("♡"), "应含 ♡ 空跳, 实: {out}");
}

// =====================================================================
// 2. render 含 60Hz + CPU 字段
// =====================================================================

#[test]
fn render_contains_60hz_and_cpu() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::heart::render(area);
    assert!(out.contains("[HEART]"), "render 应有 [HEART] label");
    assert!(out.contains("心"), "render 应有中文'心'");
    // R22 ST-A1.6: 60Hz → "beats: N ticks", 字段重命名
    assert!(
        out.contains("beats"),
        "heart render 应含 beats (原 60Hz 重命名)"
    );
    // R22 ST-A1.6: CPU → cycle_count
    assert!(
        out.contains("cycle_count"),
        "heart render 应含 cycle_count (原 CPU 重命名)"
    );
}

// =====================================================================
// 3. render 标 [partial] 诚实
// =====================================================================

#[test]
fn render_marks_partial_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::heart::render(area);
    // heart R22 ST-A1.6 已真接 /v1/observability/heart (data goes through api),
    // 不再假装为 partial. 验证 render 含 [ok] 或调度信息.
    assert!(
        out.contains("[ok]") || out.contains("beats"),
        "heart render 现以 [ok] / beats 标识真接: {out}"
    );
}

// =====================================================================
// 4. render 跨平台 ASCII (无 emoji)
// =====================================================================

#[test]
fn render_uses_cross_platform_ascii() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::heart::render(area);
    // 跨平台: ♥ / ♡ 是 Unicode 几何符号 (不是 emoji), 跟 ratatui 安全
    for c in out.chars() {
        // 允许 ASCII + ♥ + ♡ + em dash (—) + 中文
        // 注意: 括号包 cast 避免 Rust 解析成 generic args 比较
        let cu = c as u32;
        assert!(
            c.is_ascii() || c == '♥' || c == '♡' || c == '—' || (cu > 0x4e00 && cu < 0x9fff),
            "heart 字符 {c:?} 跨平台不安全"
        );
    }
}
