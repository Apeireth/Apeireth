/// 9 器官 × Growth (生长) 单元测试 (R26 升级)
///
/// **测试范围** (R26 升级):
/// - 4 阶段工程用语 badge (Init/Bootstrap/Serving/Saturated) 校验
/// - 砍了 Birth/Reproduction/Migration/Rebirth/Decline/Death, 不应在 UI 出现
/// - `compute_life_stages_info()` 仅返 4 项
/// - 反思环 R26 真接 backend (旧实现 identity.birth_time 写死导致永远空圆, 修)
///
/// **R11 LOCKED 边界**: 0 触, 仅 TUI 层测试.

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


mod test_common;

#[test]
fn stage_badge_returns_4_engineering_terms() {
    assert_eq!(backend::stage_badge(1), "Init");
    assert_eq!(backend::stage_badge(2), "Bootstrap");
    assert_eq!(backend::stage_badge(3), "Serving");
    assert_eq!(backend::stage_badge(4), "Saturated");
}

#[test]
fn stage_badge_rejects_out_of_range() {
    assert_eq!(backend::stage_badge(0), "?");
    assert_eq!(backend::stage_badge(5), "?");
    assert_eq!(backend::stage_badge(255), "?");
}

#[test]
fn life_stages_info_only_returns_4_stages() {
    let stages = backend::compute_life_stages_info().unwrap();
    // R26: 砍了 6 阶段, 仅 4 阶段进入 UI
    assert_eq!(stages.len(), 4);
    let names: Vec<&str> = stages.iter().map(|s| s.zh.as_str()).collect();
    assert!(names.contains(&"Init"));
    assert!(names.contains(&"Bootstrap"));
    assert!(names.contains(&"Serving"));
    assert!(names.contains(&"Saturated"));
    // 砍掉的阶段不应出现
    assert!(!names.contains(&"Birth"));
    assert!(!names.contains(&"Reproduction"));
    assert!(!names.contains(&"Migration"));
    assert!(!names.contains(&"Rebirth"));
    assert!(!names.contains(&"Decline"));
    assert!(!names.contains(&"Death"));
}

#[test]
fn life_stages_info_idx_are_1_through_4() {
    let stages = backend::compute_life_stages_info().unwrap();
    let mut idxs: Vec<u8> = stages.iter().map(|s| s.idx).collect();
    idxs.sort();
    assert_eq!(idxs, vec![1, 2, 3, 4]);
}

#[test]
fn reflection_progress_is_in_unit_range() {
    // R26 真接 backend: progress = recent_72h_episode / 1000 阈值, 钳位 [0, 1]
    let p = backend::compute_reflection_progress();
    assert!((0.0..=1.0).contains(&p), "progress 应在 [0, 1] 范围: {p}");
}

