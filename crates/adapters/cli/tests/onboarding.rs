//! `apeireth onboard` 引导项目的集成测试（真文件回路 + 进程 env 补位）。
//!
//! 纪律：
//! - 测试**绝不**碰真实钥匙串：密钥相关测试强制 `APEIRETH_KEYRING_BACKEND=in-memory`；
//! - 进程级 env 走 [`EnvGuard`] 作用域：**每个用例都开**（读侧与写侧同持一把锁；
//!   作用域内改 env、析构时恢复原值 ⇒ 用例之间不可能互相看到对方的 env 覆盖）；
//! - 档案路径全部注入临时目录，不读写 `~/.apeireth`。
//!
//! **为什么每个用例都得开作用域**：env 是**进程级**可变状态 —— 同一测试二进制里的
//! 并行用例共享一张 env 表。实测 flake（本文件
//! `tuning_state_evolution_reshapes_budgets_during_use`）：并行用例把
//! `APEIRETH_DISABLE_ONBOARDING=1` 写进进程 env ⇒ 本用例的用量回写（内部读这个
//! 逃生门）被**静默跳过** ⇒ 回合数断言假红。
//!
//! **防回归**：两条源码级机械判据把纪律钉死 ——
//! `every_case_opens_an_env_scope`（每个测试用例都得恰好开一个作用域）与
//! `env_mutation_is_confined_to_the_env_guard_entry`（裸改 env 只准出现在
//! `EnvGuard` 入口体内）。

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Mutex, MutexGuard};

use apeireth_cli::onboarding::{
    self, ActivationSummary, CasualPreset, Channel, OnboardingPaths, OnboardingProfile,
    TuningState, UserTier, WizardOptions, RECOMMENDED_PRESET_ENVS,
};

/// 进程级 env 串行锁 —— **读侧与写侧同持这一把**。
///
/// env 是进程级可变状态：并行用例互相看到对方的覆盖就是竞态（实测 flake：
/// 调参用例读到并行用例设的 `APEIRETH_DISABLE_ONBOARDING=1` ⇒ 用量回写被静默
/// 跳过 ⇒ 回合数断言假红）。锁只经 [`EnvGuard`] 入口获取 ⇒ 改 env 与读 env
/// 天然互斥。
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// env 作用域守卫：**持进程级 env 锁** + 用后恢复 env 原值。
///
/// 两条纪律：
/// 1. 每个用例**必须**先开作用域（判据：`every_case_opens_an_env_scope`）；
/// 2. 改 env **只准**走 [`EnvGuard::apply`]（唯一入口：作用域内改、析构时恢复）。
///
/// 作用域存活期间本进程内**无人能改 env** ⇒ 读 env 的用例（哪怕它自己不改 env）
/// 也必须开作用域，否则会读到并行用例写进去的值。
struct EnvGuard {
    /// 进程级 env 锁的守卫：**字段析构发生在 env 恢复之后** ⇒ 恢复完成才放锁
    /// （否则下一个持锁者会看到一个「半恢复」的 env）。
    _lock: MutexGuard<'static, ()>,
    /// 各 key 的原值（`None` = 原先未设）；作用域析构时按此恢复。
    saved: HashMap<&'static str, Option<String>>,
}

impl EnvGuard {
    /// 开作用域：取进程级 env 串行锁（作用域存活期间本进程内无人能改 env）。
    fn enter() -> Self {
        Self {
            _lock: ENV_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            saved: HashMap::new(),
        }
    }

    /// 作用域内改 env（**唯一入口**）：覆盖给定 key；`None` = 清除。
    ///
    /// 同一 key 可多次调用（先清后设）：只在首次记录原值，析构恢复才准。
    fn apply(&mut self, overrides: &[(&'static str, Option<&str>)]) {
        for (key, value) in overrides {
            // 同一 key 可能出现多次（先清后设）: 只在首次记录原值, Drop 恢复才准。
            self.saved
                .entry(*key)
                .or_insert_with(|| std::env::var(key).ok());
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    /// 恢复原值（[`Drop`] 调用）。
    fn restore(saved: &HashMap<&'static str, Option<String>>) {
        for (key, value) in saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // 先恢复 env，再由字段析构释放锁 ⇒ 下一个持锁者看到的 env 已是原值。
        Self::restore(&self.saved);
    }
}

fn temp_paths(name: &str) -> (tempfile::TempDir, OnboardingPaths) {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = OnboardingPaths::from_data_dir(dir.path().join(name).as_path());
    (dir, paths)
}

fn casual_profile() -> OnboardingProfile {
    let mut profile = OnboardingProfile::casual(
        "deepseek",
        Channel::Generic,
        "https://api.deepseek.com/v1",
        vec!["deepseek-chat".to_string()],
        "deepseek-chat",
    );
    profile.self_description = "我是学生，回答简洁".to_string();
    profile
}

/// 激活会触碰的全部 env（补位测试前后都清干净）。
fn activation_env_overrides<'a>(
    cleared: &'a [(&'static str, Option<&'static str>)],
) -> Vec<(&'static str, Option<&'static str>)> {
    let mut all: Vec<(&'static str, Option<&'static str>)> = vec![
        ("APEIRETH_API_URL", None),
        ("APEIRETH_API_MODELS", None),
        ("APEIRETH_MODEL", None),
        ("APEIRETH_CONTEXT_BUDGET_CHARS", None),
        ("APEIRETH_MAX_TURN_ROUNDS", None),
        ("APEIRETH_MAX_TOOL_CALLS", None),
        ("APEIRETH_DISABLE_ONBOARDING", None),
    ];
    for knob in RECOMMENDED_PRESET_ENVS {
        all.push((knob, None));
    }
    for entry in cleared {
        all.push(*entry);
    }
    all
}

// ---------------------------------------------------------------------------
// 源码级机械判据的锚点（判据扫的是本文件源码，故锚点用 `concat!` 拆开写：
// 免得判据把自己写下的字面量也当成「真实调用」）。
// ---------------------------------------------------------------------------

/// 测试用例标记（用 `concat!` 拆开写：免得判据把自己写下的字面量也当成真实标记）。
const TEST_MARKER: &str = concat!("#[", "test]");

/// 作用域入口**语句**（带赋值号 ⇒ 注释/提示语里提到入口名不会被误计）。
const SCOPE_ENTRY: &str = concat!("= EnvGuard::", "enter()");

/// 裸改进程 env 的两种写法（源码里只准出现在 `EnvGuard` 入口体内）。
const ENV_MUTATION_CALLS: [&str; 2] = [
    concat!("std::env::", "set_var"),
    concat!("std::env::", "remove_var"),
];

/// `EnvGuard` 实现体的锚点。
const ENV_GUARD_IMPL_MARKER: &str = concat!("impl EnvGuard ", "{");

/// 本文件源码里所有测试用例标记的位置（升序）。
fn test_marker_positions(source: &str) -> Vec<usize> {
    source
        .match_indices(TEST_MARKER)
        .map(|(index, _)| index)
        .collect()
}

/// 去掉行注释（`//` 及其后内容）—— 判据只扫**代码**：注释里提到 API 名不算数。
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从 `from` 处的第一个左花括号起做括号配对，返回闭合右花括号之后的位置。
fn matching_brace_end(source: &str, from: usize) -> Option<usize> {
    let open = source[from..].find('{')? + from;
    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[test]
fn activate_fills_only_unset_env_and_never_overrides_explicit_env() {
    let mut guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("activate");
    onboarding::save_profile_at(&paths, &casual_profile()).expect("save");

    {
        guard.apply(&activation_env_overrides(&[]));
        let summary = onboarding::activate_at(&paths);
        assert!(summary.applied);
        // 通道配置 + 推荐配置六旋钮 + 自动调参预算族都补位了。
        assert!(summary.filled_env.contains(&"APEIRETH_API_URL".to_string()));
        assert!(summary
            .filled_env
            .contains(&"APEIRETH_API_MODELS".to_string()));
        assert!(summary.filled_env.contains(&"APEIRETH_MODEL".to_string()));
        for knob in RECOMMENDED_PRESET_ENVS {
            assert!(summary.filled_env.contains(&knob.to_string()), "{knob}");
            assert_eq!(std::env::var(knob).unwrap(), "1", "{knob}");
        }
        for knob in [
            "APEIRETH_CONTEXT_BUDGET_CHARS",
            "APEIRETH_MAX_TURN_ROUNDS",
            "APEIRETH_MAX_TOOL_CALLS",
        ] {
            assert!(summary.filled_env.contains(&knob.to_string()), "{knob}");
        }
        assert_eq!(
            std::env::var("APEIRETH_API_URL").unwrap(),
            "https://api.deepseek.com/v1"
        );
        assert_eq!(std::env::var("APEIRETH_MODEL").unwrap(), "deepseek-chat");
        assert!(!summary.key_seeded, "未存钥匙 = 不补密钥");
    }

    // 显式环境变量永远最高优先：预设已设的值绝不覆盖。
    {
        guard.apply(&activation_env_overrides(&[
            ("APEIRETH_API_URL", Some("https://explicit.example/v1")),
            ("APEIRETH_MAX_TURN_ROUNDS", Some("3")),
        ]));
        let summary = onboarding::activate_at(&paths);
        assert!(!summary.filled_env.contains(&"APEIRETH_API_URL".to_string()));
        assert!(!summary
            .filled_env
            .contains(&"APEIRETH_MAX_TURN_ROUNDS".to_string()));
        assert_eq!(
            std::env::var("APEIRETH_API_URL").unwrap(),
            "https://explicit.example/v1",
            "显式 env 不被档案覆盖"
        );
        assert_eq!(std::env::var("APEIRETH_MAX_TURN_ROUNDS").unwrap(), "3");
    }
}

#[test]
fn activate_escape_hatch_disables_the_whole_profile() {
    let mut guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("escape");
    onboarding::save_profile_at(&paths, &casual_profile()).expect("save");

    guard.apply(&activation_env_overrides(&[(
        "APEIRETH_DISABLE_ONBOARDING",
        Some("1"),
    )]));
    assert!(
        onboarding::active_profile_at(&paths).is_none(),
        "逃生门开启 = 档案整体不生效"
    );
    let summary = onboarding::activate_at(&paths);
    assert_eq!(summary, ActivationSummary::default(), "0 补位");
    assert!(std::env::var("APEIRETH_API_URL").is_err());
    assert!(onboarding::identity_system_block_at(&paths).is_none());
}

#[test]
fn activate_without_profile_changes_nothing() {
    let mut guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("no-profile");
    guard.apply(&activation_env_overrides(&[]));
    let summary = onboarding::activate_at(&paths);
    assert!(!summary.applied);
    assert!(summary.filled_env.is_empty());
    assert!(std::env::var("APEIRETH_MODEL").is_err());
}

#[test]
fn auto_tune_off_keeps_presets_but_skips_budget_knobs() {
    let mut guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("no-tune");
    let mut profile = casual_profile();
    profile.auto_tune = false;
    onboarding::save_profile_at(&paths, &profile).expect("save");

    guard.apply(&activation_env_overrides(&[]));
    let summary = onboarding::activate_at(&paths);
    for knob in RECOMMENDED_PRESET_ENVS {
        assert!(summary.filled_env.contains(&knob.to_string()), "{knob}");
    }
    for knob in [
        "APEIRETH_CONTEXT_BUDGET_CHARS",
        "APEIRETH_MAX_TURN_ROUNDS",
        "APEIRETH_MAX_TOOL_CALLS",
    ] {
        assert!(
            !summary.filled_env.contains(&knob.to_string()),
            "自动调参关 = 预算族不动: {knob}"
        );
    }
}

#[test]
fn tuning_state_evolution_reshapes_budgets_during_use() {
    // ⚠️ **持锁 + 清 env**（2026-10-26 修并行 flake）：本测试读档案/调参状态，
    // 而别的测试会**临时**设 `APEIRETH_DISABLE_ONBOARDING=1`（逃生门）。
    // ⇒ 与「设 env」的测试**串行**，并把自己的 env 清干净。
    // 本仓写法: `EnvGuard::enter()` 取进程级 env 串行锁 + 作用域, 再 `apply`
    // 清整组激活 env（隔离更彻底）。
    let mut _guard = EnvGuard::enter();
    _guard.apply(&activation_env_overrides(&[]));
    let (_dir, paths) = temp_paths("tuning");
    onboarding::save_profile_at(&paths, &casual_profile()).expect("save");

    // 轻回合 → 快档。
    onboarding::record_usage_at(&paths, 60, 1, 80, 50);
    let state = onboarding::load_tuning_state_at(&paths).expect("state");
    assert_eq!(state.turns, 1);
    assert_eq!(state.last_rounds, 1);
    let preset = CasualPreset::default();
    let light = onboarding::tuned_knobs(&preset, Some(&state));
    assert_eq!(light.max_turn_rounds, 4);
    assert_eq!(light.max_tool_calls, 8);

    // 重回合 → 预算扩、轮数放。
    onboarding::record_usage_at(&paths, 9_000, 6, 3_000, 4_000);
    let state = onboarding::load_tuning_state_at(&paths).expect("state");
    assert_eq!(state.turns, 2);
    let heavy = onboarding::tuned_knobs(&preset, Some(&state));
    assert_eq!(heavy.context_budget_chars, 36_000);
    assert_eq!(heavy.max_turn_rounds, 8);
    assert!(heavy.max_tool_calls >= 16);

    // 回合数持续累计（使用过程 = 持续调整的依据）。
    onboarding::record_usage_at(&paths, 10, 1, 10, 5);
    let state = onboarding::load_tuning_state_at(&paths).expect("state");
    assert_eq!(state.turns, 3);
}

#[test]
fn tuning_recording_is_a_noop_without_casual_auto_tune_profile() {
    // ⚠️ 同上：持锁 + 清 env（并行 flake 修复）。
    // 本仓写法: `EnvGuard::enter()` 取串行锁 + 作用域, 再 `apply` 清整组激活 env。
    let mut _guard = EnvGuard::enter();
    _guard.apply(&activation_env_overrides(&[]));
    let (_dir, paths) = temp_paths("tuning-noop");
    onboarding::record_usage_at(&paths, 100, 2, 10, 5);
    assert!(
        onboarding::load_tuning_state_at(&paths).is_none(),
        "无档案 = 不写调参状态"
    );

    let mut profile = casual_profile();
    profile.auto_tune = false;
    onboarding::save_profile_at(&paths, &profile).expect("save");
    onboarding::record_usage_at(&paths, 100, 2, 10, 5);
    assert!(
        onboarding::load_tuning_state_at(&paths).is_none(),
        "自动调参关 = 不写调参状态"
    );
}

#[test]
fn wizard_casual_flow_end_to_end_feeds_identity_and_activation() {
    let mut guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("wizard-e2e");
    // 服务商 2（MiniMax）/ 端点回车 / 模型回车 / 默认模型回车 / key 空 /
    // 自我描述词 / 称呼 / 人设回车 / 应用 1。
    let script = "2\n\n\n\n\n我是工程师，结论先行\n小明\n\n1\n";
    let mut input = Cursor::new(script.as_bytes().to_vec());
    let mut output: Vec<u8> = Vec::new();
    let summary = onboarding::run_wizard(
        &mut input,
        &mut output,
        &WizardOptions {
            paths: paths.clone(),
            tier_override: Some(UserTier::Casual),
            store_keys: false,
        },
    )
    .expect("wizard");
    assert!(summary.contains("自动调参：开"), "{summary}");

    // 自我描述词 → 身份段。
    let block = onboarding::identity_system_block_at(&paths).expect("identity");
    assert!(block.contains("我是工程师，结论先行"));
    assert!(block.contains("小明"));

    // 档案 → 激活补位（MiniMax 预设值原样进 env）。
    guard.apply(&activation_env_overrides(&[]));
    let activation = onboarding::activate_at(&paths);
    assert!(activation.applied);
    assert_eq!(
        std::env::var("APEIRETH_API_URL").unwrap(),
        "https://api.minimaxi.com/v1"
    );
    assert_eq!(std::env::var("APEIRETH_API_MODELS").unwrap(), "MiniMax-M3");
    assert_eq!(std::env::var("APEIRETH_MODEL").unwrap(), "MiniMax-M3");
}

#[test]
fn wizard_pro_flow_writes_nothing() {
    // ⚠️ 同上：持锁 + 清 env（并行 flake 修复）—— 向导流程读档案与 env。
    // 本仓写法: `EnvGuard::enter()` 取串行锁 + 作用域, 再 `apply` 清整组激活 env。
    let mut _guard = EnvGuard::enter();
    _guard.apply(&activation_env_overrides(&[]));
    let (_dir, paths) = temp_paths("wizard-pro-e2e");
    let mut input = Cursor::new(b"2\n".to_vec());
    let mut output: Vec<u8> = Vec::new();
    let summary = onboarding::run_wizard(
        &mut input,
        &mut output,
        &WizardOptions {
            paths: paths.clone(),
            tier_override: None,
            store_keys: false,
        },
    )
    .expect("wizard");
    assert!(summary.contains("只做介绍"));
    assert!(!paths.profile.exists(), "专业用户不写档案");
    assert!(!paths.tuning.exists(), "专业用户不写调参状态");
    assert!(onboarding::identity_system_block_at(&paths).is_none());
}

#[test]
fn in_memory_keyring_reports_not_durable_and_never_roundtrips() {
    let mut guard = EnvGuard::enter();
    let dir = tempfile::tempdir().expect("temp dir");
    let keyring_dir = dir.path().join("keyring").to_string_lossy().into_owned();
    // 测试只准用 in-memory 后端: 绝不写真实平台钥匙串 / 加密文件后端。
    guard.apply(&[
        ("APEIRETH_KEYRING_BACKEND", Some("in-memory")),
        ("APEIRETH_KEYRING_DIR", Some(keyring_dir.as_str())),
    ]);
    let stored = onboarding::store_api_key("provider.minimax.api_key", "sk-test-never-persisted")
        .expect("in-memory write");
    assert!(!stored.durable, "内存后端必须如实上报不持久");
    assert!(
        onboarding::read_stored_api_key("provider.minimax.api_key").is_none(),
        "内存后端不跨进程，读侧视为无"
    );
    assert!(
        onboarding::store_api_key("provider.minimax.api_key", "   ").is_err(),
        "空 key 拒绝写入"
    );
}

#[test]
fn tuning_state_serde_roundtrip() {
    let _guard = EnvGuard::enter();
    let (_dir, paths) = temp_paths("tuning-serde");
    let state = TuningState {
        updated_at_ms: 1_700_000_000_000,
        turns: 7,
        last_prompt_chars: 120,
        last_rounds: 3,
        last_output_chars: 450,
        last_output_tokens: 260,
    };
    onboarding::save_tuning_state_at(&paths, &state).expect("save");
    assert_eq!(onboarding::load_tuning_state_at(&paths), Some(state));
}

/// 防回归判据 ① —— **每个用例都必须恰好开一个 env 作用域**。
///
/// 判据读本文件源码（`include_str!` 静态扫描）：每个测试用例的函数体都得出现
/// 一次 `EnvGuard::enter()`。
///
/// 机械判据怎么挡回归：新加用例若忘了开作用域 —— 判据立刻失败；忘了的用例在并行
/// 下会读到别的用例写进进程 env 的值（实测 flake 的根因）。
/// 判据本身也守同一条纪律（它自己也开作用域）。
#[test]
fn every_case_opens_an_env_scope() {
    let _guard = EnvGuard::enter();
    let source = strip_line_comments(include_str!("onboarding.rs"));
    let markers = test_marker_positions(&source);
    assert!(!markers.is_empty(), "本文件应有测试用例");
    for (index, start) in markers.iter().enumerate() {
        let body_start = start + TEST_MARKER.len();
        let body_end = markers.get(index + 1).copied().unwrap_or(source.len());
        let body = &source[body_start..body_end];
        assert_eq!(
            body.matches(SCOPE_ENTRY).count(),
            1,
            "每个用例必须恰好开一个 env 作用域（作用域入口调用）：缺它 = 与并行用例共享进程级 env"
        );
    }
}

/// 防回归判据 ② —— **裸改进程 env 只准出现在 `EnvGuard` 入口体内**。
///
/// 判据读本文件源码（`include_str!` 静态扫描）：裸改进程 env 的两种调用
/// （置值 / 删值）不得出现在 `EnvGuard` 实现体之外。
///
/// 机械判据怎么挡回归：入口自带进程级锁，所以「绕过入口裸改 env」就是「无锁改
/// 进程 env」—— 判据把它钉死，新用例想裸改就必须先改判据（改判据时会被 review 看见）。
#[test]
fn env_mutation_is_confined_to_the_env_guard_entry() {
    let _guard = EnvGuard::enter();
    let source = strip_line_comments(include_str!("onboarding.rs"));
    let impl_start = source
        .find(ENV_GUARD_IMPL_MARKER)
        .expect("EnvGuard 实现体存在（入口收口判据的锚点）");
    let impl_end =
        matching_brace_end(&source, impl_start).expect("EnvGuard 实现体闭合（入口收口判据的锚点）");
    let inside = &source[impl_start..impl_end];
    let outside = format!("{}{}", &source[..impl_start], &source[impl_end..]);
    for call in ENV_MUTATION_CALLS {
        assert!(inside.contains(call), "入口必须真的负责改 env（{call}）");
        assert!(
            !outside.contains(call),
            "裸改进程 env 只准出现在 EnvGuard 入口体内（{call} 出现在入口外）"
        );
    }
}

/// 防回归判据 ③ —— **作用域真能串行**（运行时判据，非源码扫描）。
///
/// 作用域存活期间，别的线程拿不到进程级 env 锁 ⇒ 用例之间不可能同时改 env；
/// 作用域结束后锁必须释放（否则后续用例会互相阻塞）。
#[test]
fn env_scope_holds_the_process_env_lock() {
    let scope = EnvGuard::enter();
    // 锁是非重入的：本线程再取一次必然拿不到 ⇒ 证明作用域确实持着锁。
    assert!(
        ENV_LOCK.try_lock().is_err(),
        "env 作用域必须真的持着进程级 env 锁"
    );
    // 别的线程同样拿不到 ⇒ 跨线程串行。
    let other_thread_got_lock = std::thread::spawn(|| ENV_LOCK.try_lock().is_ok())
        .join()
        .expect("探测线程结束");
    assert!(
        !other_thread_got_lock,
        "env 作用域存活期间别的线程不得拿到 env 锁"
    );
    // 作用域结束后锁必须释放：另起线程取锁，取到即回信（中毒也不怕：取锁侧按
    // 中毒取内层，与作用域入口同款）。
    drop(scope);
    let (tx, rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let _held = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = tx.send(());
    });
    assert!(
        rx.recv_timeout(std::time::Duration::from_secs(10)).is_ok(),
        "作用域结束后进程级 env 锁必须释放"
    );
    waiter.join().expect("取锁线程结束");
}
