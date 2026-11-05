//! W2-② 远程访问面的进程级证据 (真 `apeireth` 二进制): 非回环绑定的
//! fail-closed 闸 (无令牌拒启动)、带令牌启动、逃生阀 `none` 明示可裸奔留审计、
//! `APEIRETH_LISTEN` 绑定旋钮、坏令牌配置拒开 —— 这些只有在真进程边界上才成立,
//! 所以钉在这里而不是单测里 (纯闸语义另有 `apeireth-gateway` 单测面)。

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
async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
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
async fn wait_ready(endpoint: &str) -> bool {
    for _ in 0..120 {
        if reqwest::get(format!("{endpoint}/health"))
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
    child: &mut std::process::Child,
    endpoint: &str,
    token: &str,
) -> Option<i32> {
    let response = reqwest::Client::new()
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

/// 停机走**信号面** (F-2 · 2026-11-05): 逃生阀下管理面 (含停机端点) 对未认证
/// 身份 fail-closed ⇒ 「绑 `anonymous` 就能停机」不成立 ⇒ ⇒ 逃生阀下停机走
/// `SIGTERM` (CLI 的 `graceful_shutdown_signal` 收 ⇒ 同走收尾账)。
///
/// 非 Unix 平台没有 `SIGTERM` (Windows 走 Ctrl+C) ⇒ 那里直接杀进程。
async fn terminate_by_signal(child: &mut std::process::Child) -> Option<i32> {
    #[cfg(unix)]
    {
        let pid = child.id();
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
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
    let endpoint = format!("http://127.0.0.1:{port}");

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
            .env_remove("APEIRETH_GATEWAY_TOKENS")
            .env_remove("APEIRETH_LISTEN"),
    );

    assert!(
        wait_ready(&endpoint).await,
        "带令牌的非回环绑定必须能起: {}",
        err_buf.lock().unwrap()
    );

    // 远程面上全 API 面令牌门照常执行: 无令牌调 chat = 401。
    let denied = reqwest::Client::new()
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
    let health = reqwest::get(format!("{endpoint}/health"))
        .await
        .expect("health");
    assert!(health.status().is_success(), "探活无认证可用");

    let exit_code = graceful_stop(&mut child, &endpoint, TOKEN).await;
    assert_eq!(exit_code, Some(0), "优雅停机必须退出成功");

    // 远程绑定的清场事实落审计档 (非回环 + 强制令牌)。
    let audit = fs_err::read_to_string(data_dir.join("daemon-audit.jsonl")).unwrap_or_default();
    assert!(
        audit.contains("gateway.remote_bind") && audit.contains("mode=enforced"),
        "远程绑定事实必须落档: {audit}"
    );

    let _ = fs_err::remove_dir_all(&dir);
}

/// 逃生阀 `none` = 明示可裸奔: 非回环绑定放行启动, 但**留审计** (`mode=
/// declared_none` + 装配期声明痕); 无令牌调用也放行 (门体不设防)。
///
/// # 前置条件 (P0-1 · 2026-11-04 管理面 fail-closed · F-2 · 2026-11-05)
///
/// 停机端点属**管理面** (要求 `UserCapability::AdminControl`) ⇒ 角色面未激活
/// 时一律 403 ⇒ 本用例要测的是**逃生阀的放行与留痕**, 角色门的前置条件由
/// 这里显式装上。
///
/// ⚠️ **F-2 后「绑 `anonymous` 就能停机」这条捷径不成立**: 逃生阀下过门署名名
/// 恒为 `anonymous` (门体不设防 ⇒ 每次放行落 `ClientIdentity::anonymous()`),
/// 而该身份**未认证** ⇒ 角色门**只认已认证身份** ⇒ 它**不是**任何用户 ⇒
/// 管理面 (含停机端点) 对它**一律 403** ⇒ ⇒ 逃生阀下停机走**信号面**
/// (SIGTERM ⇒ CLI 的 `graceful_shutdown_signal` ⇒ 同走收尾账)。另注: 逃生阀与
/// **客户端令牌表互斥** (配置矛盾拒开) ⇒ 逃生阀口径下没有别的过门署名名可绑。
///
/// ⭐ **本 fixture 故意保留 `anonymous` 绑定** (不是残留): 「用户表把 owner
/// 绑给 `anonymous` + 调用方零凭据」= **F-2 那个洞的攻击形状** ⇒ ⇒ 这里断言
/// 管理面**仍拒** ⇒ ⇒ 就是钉住「这条绑定不再让未认证身份当上 owner」。
#[tokio::test(flavor = "multi_thread")]
async fn non_loopback_escape_valve_runs_but_leaves_the_audit_trace() {
    let bin = bin_path();
    if !bin.exists() {
        eprintln!("SKIP: no canonical CLI binary; run cargo build -p apeireth-cli");
        return;
    }
    let dir = scratch_dir("escape");
    let slot = dir.join("gateway.pid");
    let data_dir = dir.join("data");
    fs_err::create_dir_all(&data_dir).unwrap();
    // 角色面激活 (管理面前置条件): owner 绑逃生阀口径的过门署名名 `anonymous`
    // (⚠️ 故意: 这是 F-2 的攻击形状, 断言它**不再**让未认证身份当 owner)。
    activate_role_surface(&data_dir, "owner-1", &[ANONYMOUS_CLIENT]);
    let port = free_port().await;
    let endpoint = format!("http://127.0.0.1:{port}");

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
            .env("APEIRETH_GATEWAY_TOKEN", "none")
            .env_remove("APEIRETH_GATEWAY_TOKENS")
            .env_remove("APEIRETH_LISTEN"),
    );

    assert!(
        wait_ready(&endpoint).await,
        "逃生阀下非回环绑定必须能起: {}",
        err_buf.lock().unwrap()
    );

    // 逃生阀 = 门体不设防 (明示自担): 无令牌调用**过令牌门**放行 (不是 401)。
    //
    // ⚠️ F-2 之后**管理面** (含停机端点) 对未认证身份一律 fail-closed ⇒ ⇒
    // 这里的写面会被**角色门**拒 (403) —— 令牌门那一层是放行的 (≠401)。
    let probe = reqwest::Client::new()
        .post(format!("{endpoint}/v1/chat"))
        .body("{}")
        .send()
        .await
        .expect("chat probe");
    assert_ne!(
        probe.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "逃生阀下放行 (风险由操作者明示自担)"
    );

    // ⇒ ⇒ 管理面 (停机端点) 对未认证身份**不可达** ⇒ 「绑 `anonymous` 就能
    // 停机」这条捷径**不成立** ⇒ ⇒ 逃生阀下停机走**信号面** (SIGTERM)。
    //
    // ⚠️ 逃生阀下**带 Bearer 也一样**: 门体不设防 ⇒ 每次放行落
    // `ClientIdentity::anonymous()` (**未认证**) ⇒ 角色门**只认已认证身份**
    // ⇒ 角色门裁 deny (口径 A) ⇒ 停机**不触发**。
    let refused = reqwest::Client::new()
        .post(format!("{endpoint}/v1/admin/shutdown"))
        .header("Authorization", "Bearer any")
        .send()
        .await
        .expect("shutdown request");
    assert_eq!(
        refused.status(),
        reqwest::StatusCode::FORBIDDEN,
        "逃生阀下管理面必须 fail-closed (未认证身份不是 owner)"
    );
    let refused_body = refused.text().await.unwrap_or_default();
    assert!(
        refused_body.contains("permission_denied"),
        "拒因必须点名角色门裁决: {refused_body}"
    );

    let exit_code = terminate_by_signal(&mut child).await;
    #[cfg(unix)]
    assert_eq!(exit_code, Some(0), "SIGTERM 停机必须干净退出 (同走收尾账)");
    #[cfg(not(unix))]
    let _ = exit_code;

    // 留痕三断言: 装配期声明 + 远程绑定事实 + 每次放行痕。
    let audit = fs_err::read_to_string(data_dir.join("daemon-audit.jsonl")).unwrap_or_default();
    assert!(
        audit.contains("gateway.auth_declared_unprotected"),
        "逃生阀声明必须留痕: {audit}"
    );
    assert!(
        audit.contains("gateway.remote_bind") && audit.contains("mode=declared_none"),
        "非回环逃生阀必须留痕: {audit}"
    );
    assert!(
        audit.contains("gateway.auth_unprotected"),
        "每次放行留痕: {audit}"
    );
    // ⇒ ⇒ 信号面停机**同走收尾账** (收尾审计落盘)。
    #[cfg(unix)]
    assert!(
        audit.contains("gateway.shutdown"),
        "SIGTERM 停机必须走收尾账 (写收尾审计): {audit}"
    );

    let _ = fs_err::remove_dir_all(&dir);
}

/// 同步测试也要端口: 绑一下拿空闲端口 (tokio 外的调用面)。
fn free_port_blocking() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind free port");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}
