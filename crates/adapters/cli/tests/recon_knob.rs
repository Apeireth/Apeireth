//! Recon / 外部适配器旋钮（2026-10-22 接线批）的治理接线回归。
//!
//! 钉四件事：
//! 1. **开关只认显式 `"1"`**（`0` / `true` / `on` / `11` 都不算开，前后空白容忍）；
//! 2. **授权范围缺省 = 空范围**：网络目标全拒绝，且坏 scope 文件 = 启动期报错
//!    （不静默降级为空范围或全放行）；
//! 3. **注册与授权同源**：同一开关值同时决定装配侧注册与治理侧 grant，
//!    杜绝「注册了但权力没到手」；
//! 4. **治理档位按爆炸半径分档**：只读本地分析 6 件 grant 即用；出网 3 件与
//!    外部适配器 10 件 grant + 逐次人工审批。

use std::sync::Mutex;

use apeireth_cli::build_production_governance_from_env;
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceRequest, GovernanceVerdict};
use apeireth_tools_canonical::recon::wiring::{
    external_config_from_env, recon_config_from_env, recon_enabled_from_env, scope_from_env,
    ENABLE_EXTERNAL_ENV, ENABLE_RECON_ENV, TOOL_SCOPE_ENV,
};
use serde_json::json;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    keys: &'static [&'static str],
    previous: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn guard(keys: &'static [&'static str]) -> Self {
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        Self { keys, previous }
    }

    fn clear(&self) {
        for key in self.keys {
            std::env::remove_var(key);
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

async fn verdict(capability_id: &str) -> GovernanceVerdict {
    let governance = build_production_governance_from_env();
    let capability = CapabilityId::new(capability_id).expect("capability id");
    let arguments = json!({ "target": "example.com" });
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    governance.evaluate_verbose(&request).await
}

fn scope_file(name: &str, body: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "apeireth-recon-knob-{name}-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, body).expect("write scope");
    path
}

/// 开关语义：只认显式 `"1"`；`0`/`true`/`on`/`11` 一律不算开。
#[test]
fn switches_accept_only_an_explicit_one() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_RECON_ENV, ENABLE_EXTERNAL_ENV, TOOL_SCOPE_ENV]);
    guard.clear();

    assert!(!recon_enabled_from_env(), "默认关 (fail-closed)");
    assert!(recon_config_from_env().expect("默认读值").is_none());
    assert!(external_config_from_env("/tmp")
        .expect("默认读值")
        .is_none());

    std::env::set_var(ENABLE_RECON_ENV, "1");
    std::env::set_var(ENABLE_EXTERNAL_ENV, "1");
    assert!(recon_enabled_from_env());
    assert!(recon_config_from_env().expect("开启读值").is_some());
    assert!(external_config_from_env("/tmp")
        .expect("开启读值")
        .is_some());

    std::env::set_var(ENABLE_RECON_ENV, " 1 ");
    assert!(recon_enabled_from_env(), "前后空白容忍");

    for value in ["0", "true", "on", "11", ""] {
        std::env::set_var(ENABLE_RECON_ENV, value);
        assert!(!recon_enabled_from_env(), "must refuse {value:?}");
    }
}

/// 授权范围：缺省 = 空范围（网络全拒）；合法文件 = 按文件授权；坏文件 = 启动期错误。
#[tokio::test]
async fn scope_defaults_to_deny_all_and_bad_files_fail_the_boot() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_RECON_ENV, ENABLE_EXTERNAL_ENV, TOOL_SCOPE_ENV]);
    guard.clear();

    let empty = scope_from_env().expect("缺省必须给空范围而不是报错");
    assert!(!empty.authorized, "缺省范围未授权");
    assert!(
        empty.check_domain("example.com").is_err(),
        "空范围必须拒绝一切网络目标"
    );
    assert!(
        empty.check_address("127.0.0.1".parse().unwrap()).is_err(),
        "空范围必须拒绝一切地址"
    );

    let good = scope_file(
        "good",
        r#"{"name":"lab","authorized":true,"paths":["/tmp"],"domains":["example.com"],"cidrs":["127.0.0.1/32"]}"#,
    );
    std::env::set_var(TOOL_SCOPE_ENV, good.to_str().unwrap());
    let scope = scope_from_env().expect("合法 scope 必须通过");
    assert!(scope.check_domain("example.com").is_ok());
    assert!(
        scope.check_domain("other.example.org").is_err(),
        "未授权域名必须拒绝"
    );

    // 坏 JSON：启动期报错，且错误信息可读（含文件名与失败类别）。
    let bad = scope_file("bad", "{ not json");
    std::env::set_var(TOOL_SCOPE_ENV, bad.to_str().unwrap());
    std::env::set_var(ENABLE_RECON_ENV, "1");
    let error = recon_config_from_env().expect_err("坏 scope 必须让启动失败");
    assert!(error.contains("scope JSON 解析失败"), "{error}");

    // 结构坏（未显式授权）：同样是启动失败。
    let unauthorized = scope_file("unauthorized", r#"{"name":"lab","paths":["/tmp"]}"#);
    std::env::set_var(TOOL_SCOPE_ENV, unauthorized.to_str().unwrap());
    let error = recon_config_from_env().expect_err("未授权 scope 必须让启动失败");
    assert!(error.contains("scope 校验失败"), "{error}");

    for path in [good, bad, unauthorized] {
        let _ = std::fs::remove_file(path);
    }
}

/// 治理接线：开关关 = 一律拒绝；开关开 = 本地分析放行、出网与外部适配器停人工审批。
#[tokio::test]
async fn governance_follows_the_switches_and_the_approval_tiers() {
    let _lock = lock_env();
    let guard = EnvGuard::guard(&[ENABLE_RECON_ENV, ENABLE_EXTERNAL_ENV, TOOL_SCOPE_ENV]);
    guard.clear();

    // 默认关（fail-closed）：注册与授权都没有，拒绝即帧。
    for id in ["tool.recon.pack", "tool.recon.net", "tool.external.nmap"] {
        let result = verdict(id).await;
        assert!(
            matches!(result.decision, Decision::Deny { .. }),
            "默认关不得授权 {id}: {}",
            result.decision
        );
    }

    // 只开 recon：本地分析 6 件放行，出网 3 件停人工审批。
    std::env::set_var(ENABLE_RECON_ENV, "1");
    let local = verdict("tool.recon.pack").await;
    assert!(
        matches!(local.decision, Decision::Allow),
        "只读本地分析应 grant 即用: {}",
        local.decision
    );
    let network = verdict("tool.recon.net").await;
    assert!(
        matches!(network.decision, Decision::RequireApproval { .. }),
        "出网工具必须每次人批: {}",
        network.decision
    );

    // recon 关、external 开：外部适配器逐次人批（执行第三方二进制属执行档）。
    std::env::remove_var(ENABLE_RECON_ENV);
    std::env::set_var(ENABLE_EXTERNAL_ENV, "1");
    let external = verdict("tool.external.nmap").await;
    assert!(
        matches!(external.decision, Decision::RequireApproval { .. }),
        "外部适配器必须每次人批: {}",
        external.decision
    );
    let recon_off = verdict("tool.recon.pack").await;
    assert!(
        matches!(recon_off.decision, Decision::Deny { .. }),
        "recon 开关关 = 不授权: {}",
        recon_off.decision
    );
}
