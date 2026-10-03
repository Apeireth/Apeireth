//! `apeireth onboard` 引导项目的集成测试（真文件回路 + 进程 env 补位）。
//!
//! 纪律：
//! - 测试**绝不**碰真实钥匙串：密钥相关测试强制 `APEIRETH_KEYRING_BACKEND=in-memory`；
//! - env 补位测试走 `ENV_LOCK` + [`EnvGuard`]（用后恢复，进程内串行）；
//! - 档案路径全部注入临时目录，不读写 `~/.apeireth`。

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

use apeireth_cli::onboarding::{
    self, ActivationSummary, CasualPreset, Channel, OnboardingPaths, OnboardingProfile,
    TuningState, UserTier, WizardOptions, RECOMMENDED_PRESET_ENVS,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// env 守卫：set/remove 并在 Drop 恢复原值（测试隔离纪律，与既有测试同款）。
struct EnvGuard {
    saved: HashMap<&'static str, Option<String>>,
}

impl EnvGuard {
    fn apply(overrides: &[(&'static str, Option<&str>)]) -> Self {
        let mut saved = HashMap::new();
        for (key, value) in overrides {
            // 同一 key 可能出现多次（先清后设）: 只在首次记录原值, Drop 恢复才准。
            saved.entry(*key).or_insert_with(|| std::env::var(key).ok());
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
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

#[test]
fn activate_fills_only_unset_env_and_never_overrides_explicit_env() {
    let _lock = ENV_LOCK.lock().unwrap();
    let (_dir, paths) = temp_paths("activate");
    onboarding::save_profile_at(&paths, &casual_profile()).expect("save");

    {
        let _guard = EnvGuard::apply(&activation_env_overrides(&[]));
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
        let _guard = EnvGuard::apply(&activation_env_overrides(&[
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
    let _lock = ENV_LOCK.lock().unwrap();
    let (_dir, paths) = temp_paths("escape");
    onboarding::save_profile_at(&paths, &casual_profile()).expect("save");

    let _guard = EnvGuard::apply(&activation_env_overrides(&[(
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
    let _lock = ENV_LOCK.lock().unwrap();
    let (_dir, paths) = temp_paths("no-profile");
    let _guard = EnvGuard::apply(&activation_env_overrides(&[]));
    let summary = onboarding::activate_at(&paths);
    assert!(!summary.applied);
    assert!(summary.filled_env.is_empty());
    assert!(std::env::var("APEIRETH_MODEL").is_err());
}

#[test]
fn auto_tune_off_keeps_presets_but_skips_budget_knobs() {
    let _lock = ENV_LOCK.lock().unwrap();
    let (_dir, paths) = temp_paths("no-tune");
    let mut profile = casual_profile();
    profile.auto_tune = false;
    onboarding::save_profile_at(&paths, &profile).expect("save");

    let _guard = EnvGuard::apply(&activation_env_overrides(&[]));
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
    let _lock = ENV_LOCK.lock().unwrap();
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
    let _guard = EnvGuard::apply(&activation_env_overrides(&[]));
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
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().expect("temp dir");
    let keyring_dir = dir.path().join("keyring").to_string_lossy().into_owned();
    // 测试只准用 in-memory 后端: 绝不写真实平台钥匙串 / 加密文件后端。
    let _guard = EnvGuard::apply(&[
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
