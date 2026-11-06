//! TEMPORARY DIAGNOSTIC — not a gate. Delete after the investigation.
//!
//! Replicates `startup_modes::existing_mode_refuses_missing_or_incomplete_databases_...`
//! step by step and prints the child's stderr at every step so a divergent
//! failure reason is visible instead of hidden behind a bare `assert!`.
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn exe() -> std::path::PathBuf {
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    dir.join(format!(
        "apeireth-bench-harness{}",
        std::env::consts::EXE_SUFFIX
    ))
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
    let _ = child.stdin.take().unwrap().write_all(b"STOP\n");
    child.wait_with_output().unwrap()
}

fn show(tag: &str, out: &Output) {
    println!(
        "DIAG {tag}: status={:?} stdout={:?} stderr={:?}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn diag_existing_sequence() {
    let rounds: usize = std::env::var("DIAG_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    for round in 0..rounds {
        let dir = std::env::temp_dir().join(format!("diag-probe-{}-{}", std::process::id(), round));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        println!("DIAG round {round} dir={}", dir.display());

        let missing = probe("existing", &dir);
        show("step1_existing_on_empty", &missing);
        let first = probe("first", &dir);
        show("step2_first", &first);
        let existing = probe("existing", &dir);
        show("step3_existing", &existing);
        let refuse = probe("first", &dir);
        show("step4_first_must_refuse", &refuse);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let deleted = rt.block_on(async {
            use apeireth_runtime::SessionStore;
            let store =
                apeireth_runtime_assembly::SqliteSessionStore::open(dir.join("session.sqlite3"))
                    .await
                    .unwrap();
            let id = "a1b00000-0000-4000-8000-000000000001".parse().unwrap();
            store.delete(&id).await.unwrap()
        });
        println!("DIAG step5_delete_returned={deleted}");
        drop(rt);
        // Deliberately NOT dropping any handle the pool registry may still hold:
        // mirror the gate test exactly (it also drops only the runtime).
        let missing_session = probe("existing", &dir);
        show("step6_existing_after_delete", &missing_session);
        println!(
            "DIAG step6_contains_fixture_session_missing={}",
            String::from_utf8_lossy(&missing_session.stderr).contains("fixture session missing")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
