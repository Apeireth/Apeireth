//! Resident gateway end-to-end tests (W2-① daemon 化) against the real
//! `apeireth` binary: the single-instance refusal (二次启动显式报"已在运行",
//! 不双开) and the graceful shutdown closing contract (收尾审计 + 落盘 + PID
//! 文件释放) driven through the token-gated shutdown endpoint.
//!
//! These are the two behaviors that only exist at process level, so they are
//! pinned here rather than in unit tests.

use std::process::{Command, Stdio};
use std::time::Duration;

const TOKEN: &str = "loopback-token-0123456789abcdef";

fn bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("current test binary path");
    path.pop(); // exit test exe
    if path.ends_with("deps") {
        path.pop(); // exit deps dir to target/debug
    }
    path.push(if cfg!(windows) {
        "apeireth.exe"
    } else {
        "apeireth"
    });
    path
}

fn scratch_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-resident-e2e-{}-{label}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A free loopback port (bind, read, release).
async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn slot_text(pid: u32, port: u16) -> String {
    format!("pid={pid}\nport={port}\nbind=127.0.0.1\nstarted_at_ms=1\n")
}

/// Spawn a long-running gateway child with its stdout/stderr continuously
/// drained into buffers (a dropped pipe would kill the CLI's first `eprintln!`;
/// the buffers double as failure diagnostics).
fn spawn_with_drained_pipes(
    command: &mut Command,
) -> (
    std::process::Child,
    std::sync::Arc<std::sync::Mutex<String>>,
    std::sync::Arc<std::sync::Mutex<String>>,
) {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn resident gateway");
    let out = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let err = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    for (stream, sink) in [
        (
            Box::new(child.stdout.take().expect("stdout pipe")) as Box<dyn std::io::Read + Send>,
            out.clone(),
        ),
        (
            Box::new(child.stderr.take().expect("stderr pipe")) as Box<dyn std::io::Read + Send>,
            err.clone(),
        ),
    ] {
        std::thread::spawn(move || {
            use std::io::Read;
            let mut stream = stream;
            let mut buffer = String::new();
            let _ = stream.read_to_string(&mut buffer);
            sink.lock().unwrap().push_str(&buffer);
        });
    }
    (child, out, err)
}

/// 二次启动显式拒绝: 活占位 (记录端口应答) 在手, 第二个 `gateway serve`
/// 必须非零退出并显式报"已在运行", 且不碰既有占位内容。
#[test]
fn second_start_refuses_with_an_explicit_already_running_error() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async {
        // 活占位: 真实监听者占住记录端口。
        let live = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let live_port = live.local_addr().unwrap().port();
        let dir = scratch_dir("refuse");
        let slot = dir.join("gateway.pid");
        fs_err::write(&slot, slot_text(4242, live_port)).unwrap();
        let other_port = free_port().await;

        let output = Command::new(&bin)
            .args([
                "gateway",
                "serve",
                "--bind",
                "127.0.0.1",
                "--port",
                &other_port.to_string(),
                "--pid-file",
                &slot.to_string_lossy(),
            ])
            .output()
            .expect("spawn second gateway");

        assert_ne!(
            output.status.code(),
            Some(0),
            "second start must fail: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("already running"),
            "refusal must be explicit: {stderr}"
        );
        assert!(
            stderr.contains("refusing to start a second instance"),
            "{stderr}"
        );
        assert!(
            stderr.contains("4242"),
            "refusal must name the live holder: {stderr}"
        );
        // 拒绝不篡改、不删除既有占位。
        assert_eq!(
            fs_err::read_to_string(&slot).unwrap(),
            slot_text(4242, live_port),
            "refusal must leave the incumbent slot untouched"
        );

        let _ = fs_err::remove_dir_all(&dir);
    });
}

/// 优雅停机收尾 (真进程): 令牌门下的 `/v1/admin/shutdown` 触发后, 网关必须
/// 写下收尾审计 (`gateway.shutdown`) 并落盘到审计档案、释放 PID 文件, 然后
/// 以成功码退出。这也是服务脚本 stop 路径的执行契约。
#[test]
fn graceful_shutdown_endpoint_stops_the_process_and_frees_the_slot() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime.block_on(async {
        let dir = scratch_dir("graceful");
        let slot = dir.join("gateway.pid");
        let data_dir = dir.join("data");
        fs_err::create_dir_all(&data_dir).unwrap();
        let port = free_port().await;
        let endpoint = format!("http://127.0.0.1:{port}");

        // 真进程 stdout/stderr 由旁路线程持续抽干 (管道空悬会砸掉 CLI 的首个
        // eprintln!), 失败时缓冲即完整诊断。
        let (mut child, out_buf, err_buf) = spawn_with_drained_pipes(
            Command::new(&bin)
                .args([
                    "gateway",
                    "serve",
                    "--bind",
                    "127.0.0.1",
                    "--port",
                    &port.to_string(),
                    "--pid-file",
                    &slot.to_string_lossy(),
                ])
                .env("APEIRETH_DATA_DIR", &data_dir)
                .env("APEIRETH_SESSION_DB", data_dir.join("sessions.sqlite3"))
                .env("APEIRETH_COGNITIVE_DB", data_dir.join("cognitive.sqlite3"))
                .env("APEIRETH_GATEWAY_TOKEN", TOKEN),
        );

        // 就绪探针。
        let health = format!("{endpoint}/health");
        let mut ready = false;
        for _ in 0..120 {
            if reqwest::get(&health)
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false)
            {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if !ready {
            let _ = child.kill();
            panic!("gateway never became ready: {}", err_buf.lock().unwrap());
        }

        // 占位在手: PID 文件记录本进程与端口。
        let slot_text_now = fs_err::read_to_string(&slot).expect("slot file written");
        assert!(
            slot_text_now.contains(&format!("port={port}")),
            "{slot_text_now}"
        );
        assert!(
            slot_text_now.contains(&format!("pid={}", child.id())),
            "slot must record the serving process: {slot_text_now}"
        );

        // 优雅停机令 (服务脚本 stop 的同一条路径)。
        let response = reqwest::Client::new()
            .post(format!("{endpoint}/v1/admin/shutdown"))
            .header("Authorization", format!("Bearer {TOKEN}"))
            .send()
            .await
            .expect("shutdown request");
        assert!(
            response.status().is_success(),
            "authorized shutdown must be accepted: {}",
            response.status()
        );

        // 进程在收尾后自行退出 (成功码)。
        let mut exit_code = None;
        for _ in 0..60 {
            if let Some(status) = child.try_wait().expect("child status") {
                exit_code = status.code();
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        assert_eq!(
            exit_code,
            Some(0),
            "graceful shutdown must exit cleanly, stderr: {}",
            err_buf.lock().unwrap()
        );
        assert!(
            out_buf.lock().unwrap().contains("server stopped at"),
            "clean stop must be reported on stdout: {}",
            out_buf.lock().unwrap()
        );

        // 收尾三断言: PID 文件释放 + 审计档案落盘收尾事实。
        assert!(
            !slot.exists(),
            "graceful shutdown must release the resident slot file"
        );
        let audit = fs_err::read_to_string(data_dir.join("daemon-audit.jsonl")).unwrap_or_default();
        assert!(
            audit.contains("gateway.shutdown"),
            "closing audit fact must reach the archive on disk: {audit}"
        );
        assert!(
            audit.contains("graceful=1"),
            "closing audit fact must record the graceful path: {audit}"
        );

        let _ = fs_err::remove_dir_all(&dir);
    });
}
