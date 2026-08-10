/// 9 器官 × Body (体) 单元测试
///
/// **测试范围**:
/// - 进程 + 4 资源 (CPU / memory / disk / net)
/// - 4 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: 身体资源服务 ASI 北极星 (载体稳定 → 思考连续)
/// - S-2 实事求是: sysinfo 未引, 用占位不假装
/// - O-2: 借 unix top / Windows tasklist 设计
/// - O-3: 4 资源字段都列
/// - O-4: 4 字段清楚
/// - O-5: 全部用占位, 标 [partial]
///
/// **8 项承诺**: 全部遵守
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
/// **保守原则**: 不加 sysinfo 依赖 (会动 Cargo.toml = LOCKED),
// 用占位数据 + ASCII progress bar (跟 nav/status.rs 复用 █ / ░).



mod test_common;

use ratatui::layout::Rect;

// =====================================================================
// 1. render 含 body label + 进程
// =====================================================================

#[test]
fn render_contains_body_label_and_pid() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::body::render(area);
    assert!(out.contains("[BODY]"));
    assert!(out.contains("体"));
    assert!(out.contains("pid"), "应有 pid 字段");
    assert!(out.contains("apeireth-tui"), "应有进程名 (编译期 hardcode)");
}

// =====================================================================
// 2. render 含 4 资源通道
// =====================================================================

#[test]
fn render_4_resource_channels() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::body::render(area);
    assert!(out.contains("cpu"), "应含 cpu 资源");
    assert!(out.contains("memory"), "应含 memory 资源");
    assert!(out.contains("disk"), "应含 disk 资源");
    assert!(out.contains("net"), "应含 net 资源");
}

// =====================================================================
// 3. render 用 ASCII progress bar (跟 nav/status 复用)
// =====================================================================

#[test]
fn render_uses_ascii_progress_bars() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::body::render(area);
    assert!(out.contains('█'), "应含满块");
    assert!(out.contains('░'), "应含空块");
    // 不能含 emoji — 允许 ASCII + 边框 (─│) + em dash (—) + progress bar (█░) + CJK
    // (跟 nav/status.rs 复用 ASCII 字符集, 跨平台)
    for c in out.chars() {
        let cu = c as u32;
        assert!(
            c.is_ascii()
                || c == '─' || c == '│' || c == '—'
                || c == '█' || c == '░'
                || (cu > 0x4e00 && cu < 0x9fff),
            "body 不应含非 ASCII 字符 {c:?}"
        );
    }
}

// =====================================================================
// 4. render 标 [partial] 诚实 (sysinfo 未引, LOCKED Cargo.toml)
// =====================================================================

#[test]
fn render_marks_partial_honestly() {
    let area = Rect::new(0, 0, 80, 24);
    let out = organ::body::render(area);
    assert!(
        out.contains("[partial]"),
        "body 必须标 partial, 不假装接 sysinfo (LOCKED Cargo.toml): {out}"
    );
}

