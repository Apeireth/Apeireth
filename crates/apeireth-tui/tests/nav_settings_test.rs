/// 5 nav × Settings 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围**:
/// - 5 鉴权 (auth_token / api_key_secondary / session_secret / signing_key / refresh_token)
/// - 5 Provider (primary / fallback / embedding / rerank / vision)
/// - 4 SDK (sandbox / keyring / observability / protocol)
/// - 5 测试函数
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: Settings 屏服务 ASI 北极星 (配置可见 → 信任)
/// - S-2 实事求是: 14 配置项全列, 不假装接持久化
/// - O-2 走在前人肩上: 复用 persistence::Settings (R19 已有, 不重复造轮子)
/// - O-3 干到底: 5+5+4 = 14 配置项不漏
/// - O-4 任何人都能接手: 字段名清楚
/// - O-5 不假装: 编辑/持久化待 R25.3 接
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

/// **8 项承诺**: 全部遵守

mod test_common;

use ratatui::layout::Rect;
use test_common::{FIVE_AUTH, FIVE_PROVIDER, FOUR_SDK};

// =====================================================================
// 1. 5 鉴权正确计数 + 跟 test_common 同步
// =====================================================================

#[test]
fn five_auth_correct_count_and_synced() {
    assert_eq!(nav::settings::FIVE_AUTH.len(), 5);
    assert_eq!(nav::settings::FIVE_AUTH, FIVE_AUTH, "FIVE_AUTH 跟 test_common 同步");
    assert!(nav::settings::FIVE_AUTH.contains(&"auth_token"));
    assert!(nav::settings::FIVE_AUTH.contains(&"refresh_token"));
}

// =====================================================================
// 2. 5 Provider 正确计数 + 同步
// =====================================================================

#[test]
fn five_provider_correct_count_and_synced() {
    assert_eq!(nav::settings::FIVE_PROVIDER.len(), 5);
    assert_eq!(
        nav::settings::FIVE_PROVIDER, FIVE_PROVIDER,
        "FIVE_PROVIDER 跟 test_common 同步"
    );
    assert!(nav::settings::FIVE_PROVIDER.contains(&"provider_primary"));
    assert!(nav::settings::FIVE_PROVIDER.contains(&"provider_vision"));
}

// =====================================================================
// 3. 4 SDK 正确计数 + 同步
// =====================================================================

#[test]
fn four_sdk_correct_count_and_synced() {
    assert_eq!(nav::settings::FOUR_SDK.len(), 4);
    assert_eq!(nav::settings::FOUR_SDK, FOUR_SDK, "FOUR_SDK 跟 test_common 同步");
    assert!(nav::settings::FOUR_SDK.contains(&"sdk_sandbox"));
    assert!(nav::settings::FOUR_SDK.contains(&"sdk_protocol"));
}

// =====================================================================
// 4. 5+5+4 = 14 总和
// =====================================================================

#[test]
fn total_14_config_keys() {
    let total = nav::settings::FIVE_AUTH.len()
        + nav::settings::FIVE_PROVIDER.len()
        + nav::settings::FOUR_SDK.len();
    assert_eq!(total, 14, "5 鉴权 + 5 Provider + 4 SDK = 14 配置项");
}

// =====================================================================
// 5. render 列出全部 14 字段 + 标 [partial]
// =====================================================================

#[test]
fn render_lists_all_14_keys_and_marks_partial() {
    let area = Rect::new(0, 0, 80, 30);
    let out = nav::settings::render(area);
    for k in nav::settings::FIVE_AUTH
        .iter()
        .chain(nav::settings::FIVE_PROVIDER.iter())
        .chain(nav::settings::FOUR_SDK.iter())
    {
        assert!(out.contains(k), "render 应含配置 {k}");
    }
    assert!(out.contains("5 鉴权"), "render 应有 5 鉴权段");
    assert!(out.contains("5 Provider"), "render 应有 5 Provider 段");
    assert!(out.contains("4 SDK"), "render 应有 4 SDK 段");
    assert!(
        out.contains("[partial]") || out.contains("partial"),
        "Settings render 应标 partial, 不假装接 persistence: {out}"
    );
}

