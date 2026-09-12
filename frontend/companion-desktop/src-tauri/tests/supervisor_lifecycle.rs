//! Lifecycle integration tests that spawn the REAL canonical backend.
//!
//! The unit tests in `backend_supervisor.rs` cover state and argument shape
//! without a child process, which leaves the parts that only exist at runtime
//! unexercised: the Ready transition after a live health probe, unexpected-exit
//! detection, owned shutdown, and restart. Those are the paths a release
//! actually depends on, so they are covered here against a real process.
//!
//! Requires `target/release/apeireth` (or debug) to exist — build it with
//! `cargo build --release -p apeireth-cli`. Each test resolves the binary the
//! same way production does.
//!
//! The gateway builds a canonical runtime that opens a SQLite session store, so
//! concurrent instances would contend on the same database file. `GATEWAY_LOCK`
//! serializes these tests; cargo still runs them on separate threads, they just
//! never hold a live gateway at the same time.
//!
//! These tests use `new_for_test()` (no logger). That used to drop piped
//! stdout/stderr immediately, which killed the CLI on its first `eprintln!`
//! (Windows exit 101) and made `wait_for_ready` time out. Reaching Ready here
//! is the regression proof that the supervisor drains those pipes.

use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use companion_desktop_lib::backend_supervisor::{
    BackendCapabilityEnv, BackendOwnership, BackendProviderEnv, BackendState, BackendSupervisor,
};

fn gateway_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Scoped process-env mutation: restores previous values on drop, so tests
/// that set/clear `APEIRETH_*_DB` cannot leak state into sibling tests.
struct EnvGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvGuard {
    fn set(vars: &[(&'static str, &str)]) -> Self {
        let mut saved = Vec::new();
        for (key, value) in vars {
            saved.push((*key, std::env::var_os(key)));
            std::env::set_var(key, value);
        }
        Self(saved)
    }

    fn clear(keys: &[&'static str]) -> Self {
        let mut saved = Vec::new();
        for key in keys {
            saved.push((*key, std::env::var_os(key)));
            std::env::remove_var(key);
        }
        Self(saved)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Point the child at throwaway sqlite files so a leaked process cannot lock
/// the developer's checkout `.apeireth/` directory, and so tests do not share
/// the default relative `sessions.sqlite3` path.
fn isolate_backend_data() {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-supervisor-test-{}",
        std::process::id()
    ));
    fs_err::create_dir_all(&dir).expect("temp backend data dir");
    std::env::set_var("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"));
    std::env::set_var("APEIRETH_COGNITIVE_DB", dir.join("cognitive.sqlite3"));
}

/// Whether a canonical backend build is available to spawn.
///
/// Reported rather than silently skipped: a lifecycle test that quietly passes
/// with no binary present would be worse than one that fails.
fn backend_available() -> bool {
    BackendSupervisor::resolve_dev_backend_for_test().is_some()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

/// Kill a process outside the supervisor's knowledge, simulating a crash.
fn kill_externally(pid: u32) {
    #[cfg(windows)]
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output();
    #[cfg(not(windows))]
    let status = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output();
    assert!(status.is_ok(), "failed to invoke the kill command for pid {pid}");
}

#[test]
fn spawns_real_backend_and_reaches_ready() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found; run cargo build --release -p apeireth-cli");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();

        let message = supervisor.start().await.expect("backend should start");
        assert!(message.contains("Backend started"), "unexpected: {message}");

        let info = supervisor.info().await;
        assert_eq!(info.state, BackendState::Ready);
        assert_eq!(info.ownership, BackendOwnership::OwnedByDesktop);
        let pid = info.pid.expect("a Ready backend must report a PID");
        let endpoint = info.endpoint.clone().expect("a Ready backend must publish an endpoint");
        let port = info.port.expect("a Ready backend must report its port");

        // Endpoint must be loopback on the allocated port, never the legacy one.
        assert_eq!(endpoint, format!("http://127.0.0.1:{port}"));
        assert_ne!(port, 8090, "must never bind the retired companion port");

        // The endpoint the supervisor published actually answers.
        let health = reqwest::get(format!("{endpoint}/health"))
            .await
            .expect("health request should reach the published endpoint");
        assert!(health.status().is_success(), "health status: {}", health.status());
        let body = health.text().await.unwrap_or_default();
        assert!(body.contains("\"status\""), "unexpected health body: {body}");

        // Owned shutdown returns the process to Stopped and clears the endpoint.
        supervisor.stop().await.expect("owned backend should stop");
        let after = supervisor.info().await;
        assert_eq!(after.state, BackendState::Stopped);
        assert!(after.pid.is_none(), "PID must be cleared after stop");
        assert!(after.endpoint.is_none(), "endpoint must be cleared after stop");

        // The port is genuinely released.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(
            reqwest::get(format!("http://127.0.0.1:{port}/health"))
                .await
                .is_err(),
            "stopped backend should no longer answer on {port}"
        );
        eprintln!("verified lifecycle: pid={pid} port={port}");
    });
}

/// Settings-UI provider configuration must reach the real child's environment:
/// applying an OpenAI-compatible env restarts the backend, and the restarted
/// gateway then lists the injected model (model registration reads
/// `APEIRETH_OPENAI_MODELS`, no auth needed at list time). Re-applying the same
/// env is a no-op that must not restart again.
#[test]
fn provider_env_reaches_the_backend() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();
        supervisor.start().await.expect("backend should start");
        let first_port = supervisor.info().await.port.expect("initial port");

        let env = BackendProviderEnv {
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("supervisor-test-model-x9".into()),
            ..Default::default()
        };
        let info = supervisor
            .apply_provider_env(env.clone())
            .await
            .expect("apply should restart the Ready backend");
        assert_eq!(info.state, BackendState::Ready);
        assert_eq!(info.restart_count, 1, "a changed env restarts exactly once");
        let endpoint = info.endpoint.clone().expect("endpoint after restart");
        assert_ne!(
            info.port.expect("port after restart"),
            first_port,
            "restart picks a fresh port; old {first_port} new {:?}",
            info.port
        );

        let models = reqwest::get(format!("{endpoint}/v1/models"))
            .await
            .expect("models request")
            .text()
            .await
            .unwrap_or_default();
        assert!(
            models.contains("supervisor-test-model-x9"),
            "injected APEIRETH_OPENAI_MODELS must appear in /v1/models: {models}"
        );

        // Same env again: no restart.
        let before = supervisor.info().await.restart_count;
        supervisor
            .apply_provider_env(env)
            .await
            .expect("no-op apply");
        assert_eq!(
            supervisor.info().await.restart_count,
            before,
            "unchanged env must not restart"
        );

        supervisor.stop().await.expect("cleanup stop");
    });
}

/// Advanced-capability toggles reach the real child the same way: enabling
/// shell via `apply_backend_config` restarts the backend and the restarted
/// gateway then serves `tool.shell` in `/v1/tools/list` (registration reads
/// `APEIRETH_ENABLE_SHELL`, no auth needed at list time). The combined call
/// with an unchanged provider must still restart exactly once, proving the
/// single-restart contract.
#[test]
fn capability_env_reaches_the_backend() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();
        supervisor.start().await.expect("backend should start");

        let caps = BackendCapabilityEnv {
            enable_shell: true,
            ..Default::default()
        };
        let info = supervisor
            .apply_backend_config(None, Some(caps.clone()))
            .await
            .expect("capability apply should restart the Ready backend");
        assert_eq!(info.state, BackendState::Ready);
        assert_eq!(info.restart_count, 1, "changed capabilities restart exactly once");
        let endpoint = info.endpoint.clone().expect("endpoint after restart");

        let tools = reqwest::get(format!("{endpoint}/v1/tools/list"))
            .await
            .expect("tools request")
            .text()
            .await
            .unwrap_or_default();
        assert!(
            tools.contains("\"name\":\"shell\"") && tools.contains("\"permission\":\"granted\""),
            "injected APEIRETH_ENABLE_SHELL must register the shell tool with the governance grant: {tools}"
        );

        // Same capabilities again (provider unchanged/omitted): no restart.
        let before = supervisor.info().await.restart_count;
        supervisor
            .apply_backend_config(None, Some(caps))
            .await
            .expect("no-op apply");
        assert_eq!(
            supervisor.info().await.restart_count,
            before,
            "unchanged capabilities must not restart"
        );

        supervisor.stop().await.expect("cleanup stop");
    });
}

/// Regression for the 2026-09-28 real-world boot failure: the sidecar must
/// never resolve its default relative `.apeireth/...` store paths against the
/// launch CWD (Start-menu launches run with CWD = System32, where creating
/// `.apeireth` fails with Access Denied). With a logger attached and no
/// explicit `APEIRETH_*_DB` env, the supervisor injects absolute app-data store
/// paths and pins the child's CWD; this test proves the stores land there.
#[test]
fn sidecar_stores_land_in_app_data_regardless_of_cwd() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    // Sibling tests set absolute APEIRETH_*_DB env vars via isolate_backend_data;
    // this test must observe the pure app-data anchor path, so clear them for
    // its duration (restored on drop).
    let _env = EnvGuard::clear(&["APEIRETH_SESSION_DB", "APEIRETH_COGNITIVE_DB"]);

    let app_data = std::env::temp_dir().join(format!(
        "apeireth-desktop-data-{}-stores",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&app_data);

    runtime().block_on(async {
        let supervisor = BackendSupervisor::with_logger_in_dir(app_data.clone())
            .expect("production-shaped supervisor in temp dir");
        supervisor
            .start()
            .await
            .expect("backend should start with app-data anchors");
        let info = supervisor.info().await;
        assert_eq!(info.state, BackendState::Ready);

        let sessions = app_data.join("data").join("sessions.sqlite3");
        let cognitive = app_data.join("data").join("cognitive.sqlite3");
        assert!(
            sessions.is_file(),
            "sessions store must land in app data: {sessions:?}"
        );
        assert!(
            cognitive.is_file(),
            "cognitive store must land in app data: {cognitive:?}"
        );

        supervisor.stop().await.expect("cleanup stop");
    });
}

/// Explicit `APEIRETH_SESSION_DB` / `APEIRETH_COGNITIVE_DB` env vars are the
/// highest-priority store paths: even with a logger attached (which would
/// otherwise anchor app-data), the child must use the user-provided paths.
#[test]
fn explicit_env_store_paths_take_priority() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();

    let app_data = std::env::temp_dir().join(format!(
        "apeireth-desktop-data-{}-envprio",
        std::process::id()
    ));
    let custom = std::env::temp_dir().join(format!(
        "apeireth-desktop-custom-{}-envprio",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&app_data);
    let _ = std::fs::remove_dir_all(&custom);

    let custom_session = custom.join("sessions.sqlite3").to_string_lossy().to_string();
    let custom_cognitive = custom
        .join("cognitive.sqlite3")
        .to_string_lossy()
        .to_string();
    let _env = EnvGuard::set(&[
        ("APEIRETH_SESSION_DB", &custom_session),
        ("APEIRETH_COGNITIVE_DB", &custom_cognitive),
    ]);

    runtime().block_on(async {
        let supervisor = BackendSupervisor::with_logger_in_dir(app_data.clone())
            .expect("production-shaped supervisor in temp dir");
        supervisor
            .start()
            .await
            .expect("backend should start with explicit env store paths");
        assert_eq!(supervisor.info().await.state, BackendState::Ready);

        assert!(
            custom.join("sessions.sqlite3").is_file(),
            "explicit APEIRETH_SESSION_DB must win: {:?}",
            custom.join("sessions.sqlite3")
        );
        assert!(
            custom.join("cognitive.sqlite3").is_file(),
            "explicit APEIRETH_COGNITIVE_DB must win"
        );
        assert!(
            !app_data.join("data").exists(),
            "app-data anchors must not be used when explicit env is set: {:?}",
            app_data.join("data")
        );

        supervisor.stop().await.expect("cleanup stop");
    });
}

/// The path `watch_for_exit` exists for: a backend that dies after reaching
/// Ready must stop being reported as healthy.
#[test]
fn detects_unexpected_exit_and_records_it() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();
        supervisor.start().await.expect("backend should start");

        let pid = supervisor.info().await.pid.expect("PID");
        assert_eq!(supervisor.info().await.state, BackendState::Ready);

        // Crash it behind the supervisor's back.
        kill_externally(pid);

        // The watcher polls at 750ms; allow several intervals.
        let mut observed = None;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(400)).await;
            let info = supervisor.info().await;
            if info.state == BackendState::Failed {
                observed = Some(info);
                break;
            }
        }

        let info = observed.expect("an unexpected exit must move the state to Failed");
        assert!(
            info.last_error.is_some(),
            "a crash must record a reason, got none"
        );
        assert!(
            info.pid.is_none() && info.endpoint.is_none(),
            "a dead backend must not keep publishing a PID or endpoint"
        );
        eprintln!(
            "recorded crash: exit_code={:?} error={:?}",
            info.last_exit_code, info.last_error
        );
    });
}

#[test]
fn restart_replaces_the_process_and_counts_it() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();
        supervisor.start().await.expect("initial start");

        let first = supervisor.info().await;
        let first_pid = first.pid.expect("first PID");
        assert_eq!(first.restart_count, 0);

        supervisor.restart().await.expect("restart should succeed");

        let second = supervisor.info().await;
        assert_eq!(second.state, BackendState::Ready, "restart must end Ready");
        assert_eq!(second.restart_count, 1, "restart must be counted");
        let second_pid = second.pid.expect("second PID");
        assert_ne!(
            first_pid, second_pid,
            "restart must replace the process, not reuse it"
        );

        // The new endpoint answers, so the restart produced a working gateway.
        let endpoint = second.endpoint.clone().expect("endpoint after restart");
        let health = reqwest::get(format!("{endpoint}/health"))
            .await
            .expect("restarted backend should answer");
        assert!(health.status().is_success());

        supervisor.stop().await.expect("cleanup stop");
        eprintln!("restart verified: {first_pid} -> {second_pid}");
    });
}

/// Starting twice must not spawn a second backend: the desktop owns exactly one.
#[test]
fn concurrent_start_does_not_spawn_two_backends() {
    if !backend_available() {
        eprintln!("SKIP: no canonical backend build found");
        return;
    }
    let _guard = gateway_lock();
    isolate_backend_data();

    runtime().block_on(async {
        let supervisor = BackendSupervisor::new_for_test();
        supervisor.start().await.expect("first start");
        let pid = supervisor.info().await.pid.expect("PID");

        // A second start must be refused while one is already running.
        let second = supervisor.start().await;
        assert!(second.is_err(), "second start should be refused, got {second:?}");

        let info = supervisor.info().await;
        assert_eq!(info.pid, Some(pid), "the original process must be untouched");
        assert_eq!(info.state, BackendState::Ready);

        supervisor.stop().await.expect("cleanup stop");
    });
}
