//! Process-level verification of remote listener admission and TLS.

use std::process::{Command, Stdio};
use std::time::Duration;

mod common;

use common::{activate_role_surface, ANONYMOUS_CLIENT, ENV_CLIENT};

const TOKEN: &str = "remote-face-token-0123456789abcdef";

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
        "apeireth-remote-e2e-{}-{label}",
        std::process::id()
    ));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A free loopback port (bind, read, release).

/// ⭐ **端口分配：治「夹具的 TOCTOU」**（不是治产品）
///
/// **原写法（TOCTOU 竞态）**：`bind :0` ⇒ ⇒ 取端口 ⇒ ⇒ **立刻释放** ⇒ ⇒ 稍后再 bind
/// ⇒ ⇒ **并行测试之间会抢同一个端口** ⇒ ⇒ 自己的网关 bind 失败（子进程立刻退出）
/// ⇒ ⇒ 而 `wait_ready` 会连上**抢占者**的网关 ⇒ ⇒ **断言打在别人的服务上**
/// ⇒ ⇒ 表现为「安全断言偶发失败（期望 403 实得 200）」⇒ ⇒ **看起来像安全回归，其实是夹具竞态**。
///
/// **新写法**：按 **pid 分段 + 进程内原子计数器** 取候选端口 ⇒ ⇒ **向前探测**到 bind 成功
/// ⇒ ⇒ 同进程内**并发测试不会撞**（计数器各不相同）· 跨进程也**基本不会撞**（pid 分段不同）
/// ⇒ ⇒ 探测本身仍是「取后释放」，但候选端口**互不重叠** ⇒ ⇒ 撞的概率 ≈ 0。
fn next_free_port() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static COUNTER: AtomicU16 = AtomicU16::new(0);
    let pid = std::process::id();
    // 段基址：pid 分段 ⇒ ⇒ 不同进程不重叠；段内 64 个槽：进程内并发测试不重叠。
    let base = 21_000u16 + ((pid % 400) as u16) * 64;
    for step in 0..64u16 {
        let candidate = base + ((COUNTER.fetch_add(1, Ordering::SeqCst) + step) % 64);
        if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", candidate)) {
            let port = listener.local_addr().map(|a| a.port()).unwrap_or(candidate);
            drop(listener);
            return port;
        }
    }
    // 段内全占 ⇒ ⇒ 回落到内核选端口（极罕见 ⇒ ⇒ 仍比原写法好）
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind fallback port");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

async fn free_port() -> u16 {
    next_free_port()
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
        .expect("spawn gateway");
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

/// 等到 `/health` 应答 (就绪探针) 或超时。
async fn wait_ready(client: &reqwest::Client, endpoint: &str) -> bool {
    for _ in 0..120 {
        if client
            .get(format!("{endpoint}/health"))
            .send()
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

/// 令牌门下的优雅停机 (服务脚本 stop 的同一条路径), 返回进程退出码。
async fn graceful_stop(
    client: &reqwest::Client,
    child: &mut std::process::Child,
    endpoint: &str,
    token: &str,
) -> Option<i32> {
    let response = client
        .post(format!("{endpoint}/v1/admin/shutdown"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("shutdown request");
    assert!(
        response.status().is_success(),
        "authorized shutdown must be accepted: {}",
        response.status()
    );
    for _ in 0..60 {
        if let Some(status) = child.try_wait().expect("child status") {
            return status.code();
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    None
}

/// fail-closed 即帧: 非回环绑定 + 令牌族两面皆空 = **拒绝启动** (退出码非零,
/// 显式点名缺什么), 不裸奔、不警告后照跑。
#[test]
fn non_loopback_bind_without_any_token_refuses_to_start() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let dir = scratch_dir("refuse");
    let output = Command::new(&bin)
        .args([
            "gateway",
            "serve",
            "--bind",
            "0.0.0.0",
            "--port",
            &free_port_blocking().to_string(),
            "--pid-file",
            &dir.join("gateway.pid").to_string_lossy(),
        ])
        .env("APEIRETH_DATA_DIR", &dir)
        .env("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"))
        .env("APEIRETH_COGNITIVE_DB", dir.join("cognitive.sqlite3"))
        .env_remove("APEIRETH_GATEWAY_TOKEN")
        .env_remove("APEIRETH_GATEWAY_TOKENS")
        .env_remove("APEIRETH_LISTEN")
        .output()
        .expect("spawn gateway");

    assert_ne!(
        output.status.code(),
        Some(0),
        "无令牌的非回环绑定必须拒启动: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("refusing to bind non-loopback"),
        "fail-closed 即帧: {stderr}"
    );
    assert!(
        stderr.contains("APEIRETH_GATEWAY_TOKEN"),
        "拒帧必须点名缺什么: {stderr}"
    );
    assert!(!dir.join("gateway.pid").exists(), "拒启动不得留下常驻占位");

    let _ = fs_err::remove_dir_all(&dir);
}

/// `APEIRETH_LISTEN` 是绑定旋钮的第一公民: 不给 `--bind`, env 指到非回环
/// 同样过远程绑定闸 (无令牌照样拒启动)。
#[test]
fn the_listen_env_knob_goes_through_the_same_remote_bind_gate() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let dir = scratch_dir("listen-env");
    let output = Command::new(&bin)
        .args([
            "gateway",
            "serve",
            "--port",
            &free_port_blocking().to_string(),
            "--pid-file",
            &dir.join("gateway.pid").to_string_lossy(),
        ])
        .env("APEIRETH_LISTEN", "0.0.0.0")
        .env("APEIRETH_DATA_DIR", &dir)
        .env("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"))
        .env("APEIRETH_COGNITIVE_DB", dir.join("cognitive.sqlite3"))
        .env_remove("APEIRETH_GATEWAY_TOKEN")
        .env_remove("APEIRETH_GATEWAY_TOKENS")
        .output()
        .expect("spawn gateway");

    assert_ne!(
        output.status.code(),
        Some(0),
        "APEIRETH_LISTEN=0.0.0.0 无令牌必须拒启动: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("refusing to bind non-loopback"), "{stderr}");

    let _ = fs_err::remove_dir_all(&dir);
}

/// 坏令牌配置拒开 (StoredDoc/JSON 面 fail-closed): 进程在绑定之前就显式失败。
#[test]
fn broken_client_token_configuration_refuses_to_open() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let dir = scratch_dir("bad-config");
    let output = Command::new(&bin)
        .args([
            "gateway",
            "serve",
            "--bind",
            "127.0.0.1",
            "--port",
            &free_port_blocking().to_string(),
        ])
        .env("APEIRETH_GATEWAY_TOKENS", "not json")
        .env("APEIRETH_DATA_DIR", &dir)
        .env("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"))
        .env("APEIRETH_COGNITIVE_DB", dir.join("cognitive.sqlite3"))
        .env_remove("APEIRETH_GATEWAY_TOKEN")
        .env_remove("APEIRETH_LISTEN")
        .output()
        .expect("spawn gateway");

    assert_ne!(
        output.status.code(),
        Some(0),
        "坏令牌配置必须拒开: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("client token configuration refused to open"),
        "{stderr}"
    );

    let _ = fs_err::remove_dir_all(&dir);
}

/// 带令牌的非回环绑定可启动; 全 API 面令牌门在远程面上照常执行 (无令牌调
/// chat = 401); 停机后审计档案留 `gateway.remote_bind mode=enforced` 事实。
///
/// # 前置条件 (P0-1 · 2026-11-04 管理面 fail-closed)
///
/// 停机端点属**管理面** (要求 `UserCapability::AdminControl`) ⇒ 角色面未激活
/// (数据目录无 `users.json`) 时一律 403 ⇒ 本用例要测的是**远程绑定面**,
/// 角色门的前置条件由这里显式装上: owner 绑本用例的过门署名名 (env 单令牌
/// 面 = `env`; 未设防口径 = `anonymous`)。
#[tokio::test(flavor = "multi_thread")]
async fn non_loopback_bind_with_a_token_serves_behind_the_gate_and_keeps_the_audit() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let dir = scratch_dir("enforced");
    let slot = dir.join("gateway.pid");
    let data_dir = dir.join("data");
    fs_err::create_dir_all(&data_dir).unwrap();
    // 角色面激活 (管理面前置条件): owner 在册, 绑两个过门署名名。
    activate_role_surface(&data_dir, "owner-1", &[ENV_CLIENT, ANONYMOUS_CLIENT]);
    let port = free_port().await;
    let endpoint = format!("https://127.0.0.1:{port}");
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gateway/tests/fixtures/tls");
    let ca = fs_err::read(fixtures.join("ca-cert.pem")).unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .add_root_certificate(reqwest::Certificate::from_pem(&ca).unwrap())
        .build()
        .unwrap();

    let (mut child, _out_buf, err_buf) = spawn_with_drained_pipes(
        Command::new(&bin)
            .args([
                "gateway",
                "serve",
                "--bind",
                "0.0.0.0",
                "--port",
                &port.to_string(),
                "--pid-file",
                &slot.to_string_lossy(),
            ])
            .env("APEIRETH_DATA_DIR", &data_dir)
            .env("APEIRETH_SESSION_DB", data_dir.join("sessions.sqlite3"))
            .env("APEIRETH_COGNITIVE_DB", data_dir.join("cognitive.sqlite3"))
            .env("APEIRETH_GATEWAY_TOKEN", TOKEN)
            .env("APEIRETH_TLS_CERT", fixtures.join("server-chain.pem"))
            .env("APEIRETH_TLS_KEY", fixtures.join("server-key.pem"))
            .env_remove("APEIRETH_GATEWAY_TOKENS")
            .env_remove("APEIRETH_LISTEN"),
    );

    assert!(
        wait_ready(&client, &endpoint).await,
        "带令牌的非回环绑定必须能起: {}",
        err_buf.lock().unwrap()
    );

    // 远程面上全 API 面令牌门照常执行: 无令牌调 chat = 401。
    let denied = client
        .post(format!("{endpoint}/v1/chat"))
        .body("{}")
        .send()
        .await
        .expect("chat probe");
    assert_eq!(
        denied.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "远程面无令牌调 chat 必须 401"
    );
    // 健康探活豁免照旧 (探测刚需, 无认证)。
    let health = client
        .get(format!("{endpoint}/health"))
        .send()
        .await
        .expect("health");
    assert!(health.status().is_success(), "探活无认证可用");

    let exit_code = graceful_stop(&client, &mut child, &endpoint, TOKEN).await;
    assert_eq!(exit_code, Some(0), "优雅停机必须退出成功");

    // 远程绑定的清场事实落审计档 (非回环 + 强制令牌)。
    let audit = fs_err::read_to_string(data_dir.join("daemon-audit.jsonl")).unwrap_or_default();
    assert!(
        audit.contains("gateway.remote_bind") && audit.contains("mode=enforced"),
        "远程绑定事实必须落档: {audit}"
    );

    let _ = fs_err::remove_dir_all(&dir);
}

#[tokio::test]
async fn remote_start_refuses_insecure_configuration_before_persisting_state() {
    for (mode, token, reason) in [
        ("remote-none", "none", "without an enforceable token"),
        ("remote-cleartext", TOKEN, "without TLS"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            tokio::process::Command::new(bin_path())
                .args([
                    "gateway",
                    "serve",
                    "--bind",
                    "0.0.0.0",
                    "--port",
                    "0",
                    "--pid-file",
                ])
                .arg(dir.path().join("gateway.pid"))
                .env("APEIRETH_DATA_DIR", dir.path())
                .env(
                    "APEIRETH_COGNITIVE_DB",
                    dir.path().join("cognitive.sqlite3"),
                )
                .env("APEIRETH_GATEWAY_TOKEN", token)
                .env_remove("APEIRETH_GATEWAY_TOKENS")
                .env_remove("APEIRETH_TLS_CERT")
                .env_remove("APEIRETH_TLS_KEY")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success() && stderr.contains(reason),
            "{mode}: {stderr}"
        );
        assert!(!dir.path().join("gateway.pid").exists());
        assert!(!dir.path().join("cognitive.sqlite3").exists());
    }
}

/// 同步测试也要端口: 绑一下拿空闲端口 (tokio 外的调用面)。
fn free_port_blocking() -> u16 {
    next_free_port()
}
