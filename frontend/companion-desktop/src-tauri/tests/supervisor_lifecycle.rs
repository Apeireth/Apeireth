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
    BackendOwnership, BackendState, BackendSupervisor,
};

fn gateway_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
