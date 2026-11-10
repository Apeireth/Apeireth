//! Exercise the executable boundary: a process restart must not become an
//! in-process reopen, and resume must not silently create a missing session.
use std::collections::BTreeSet;
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// 基准 harness 可执行文件路径。
///
/// **为什么不用 `env!("CARGO_BIN_EXE_…")`**: 本机 cargo 1.97.1 在
/// `cargo build --workspace --tests` / `cargo test --no-run` 下**不注入**该变量
/// （最小复现：一个只有 `src/main.rs` + 一个集成测试的包同样报
/// “not defined at compile time”）⇒ `env!` 直接编译失败 ⇒ 整条门禁红。
/// 改为**运行期定位**：集成测试可执行文件在 `target/<profile>/deps/<test>-<hash>`，
/// 向上一层（`<profile>`）取同名二进制即 harness；Windows 按 `EXE_SUFFIX` 补 `.exe`。
fn exe() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_apeireth-bench-harness") {
        return std::path::PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join(format!(
        "apeireth-bench-harness{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        candidate.exists(),
        "先构建 harness 二进制（`cargo build -p apeireth-bench-harness`），期望路径: {}",
        candidate.display()
    );
    candidate
}

#[test]
fn independent_modes_report_samples_and_reap_their_children() {
    for mode in ["first_run", "normal_start", "warm_resume"] {
        let output = Command::new(exe())
            .arg(mode)
            .env("BENCH_QUICK", "1")
            .env("BENCH_RAW", "1")
            .env_remove("BENCH_SCALE")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let samples: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with(&format!("STARTUP_SAMPLE|{mode}|")))
            .collect();
        assert_eq!(samples.len(), 3, "{text}");
        let pids: BTreeSet<_> = samples
            .iter()
            .map(|line| line.split('|').nth(3).unwrap())
            .collect();
        assert_eq!(
            pids.len(),
            if mode == "warm_resume" { 1 } else { 3 },
            "{text}"
        );
        for row in text
            .lines()
            .filter(|line| line.starts_with(&format!("STARTUP|{mode}|")))
        {
            let fields: Vec<_> = row.split('|').collect();
            assert_eq!(fields[4], "3", "{row}");
            let p50: f64 = fields[5].parse().unwrap();
            let p95: f64 = fields[6].parse().unwrap();
            assert!(p50.is_finite() && p95 >= p50, "{row}");
        }
        assert!(text.contains("formal_gate=legacy-cold-start-unchanged"));
        assert!(text.contains("targets=PROPOSED_UNAPPROVED"));
        #[cfg(target_os = "linux")]
        for pid in pids {
            assert!(
                !std::path::Path::new(&format!("/proc/{}", pid.strip_prefix("pid=").unwrap()))
                    .exists(),
                "child still alive: {pid}"
            );
        }
    }
}

fn probe(mode: &str, dir: &std::path::Path) -> Output {
    let mut child = Command::new(exe())
        .args(["__startup-probe", mode])
        .arg(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The pipe is buffered by the OS even if the child has not reached READY.
    let _ = child.stdin.take().unwrap().write_all(b"STOP\n");
    child.wait_with_output().unwrap()
}

#[test]
fn existing_mode_refuses_missing_or_incomplete_databases_and_first_refuses_reuse() {
    let dir = std::env::temp_dir().join(format!("a1b-contract-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let missing = probe("existing", &dir);
    assert!(!missing.status.success());
    assert!(!String::from_utf8_lossy(&missing.stdout).contains("READY|"));
    let first = probe("first", &dir);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(probe("existing", &dir).status.success());
    assert!(!probe("first", &dir).status.success());
    // An intact migrated schema without the active session is not ready, and
    // the restart path must not recreate the fixture to manufacture success.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        use apeireth_runtime::SessionStore;
        let store =
            apeireth_runtime_assembly::SqliteSessionStore::open(dir.join("session.sqlite3"))
                .await
                .unwrap();
        let id = "a1b00000-0000-4000-8000-000000000001".parse().unwrap();
        assert!(store.delete(&id).await.unwrap());
    });
    drop(rt);
    let missing_session = probe("existing", &dir);
    assert!(!missing_session.status.success());
    assert!(String::from_utf8_lossy(&missing_session.stderr).contains("fixture session missing"));
    assert!(!String::from_utf8_lossy(&missing_session.stdout).contains("READY|"));
    std::fs::remove_file(dir.join("session.sqlite3")).unwrap();
    assert!(!probe("existing", &dir).status.success());
    std::fs::remove_dir_all(dir).unwrap();
}
