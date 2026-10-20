//! TLS 服务面钉测 (安全加固批 B5): 真加载 → 真启用 → 真握手; 坏材料拒启动。
//!
//! 0 装口径: live TLS 握手 E2E **已实测** (自签测试链 + 以测试 CA 为根锚的
//! HTTPS 客户端真握手, 2026-10-20, 本文件即是脚本); 覆盖面 = 树内默认协议族
//! (TLS 1.2/1.3) × RSA-2048 测试证书链。测试夹具为一次性生成的自签材料,
//! 见 `tests/fixtures/tls/README.md`, 私钥仅测试用途、无任何真实信任价值。
//!
//! 每条钉住的行为: 合法证书链真启用并真握手 / 坏文件拒启动 / 半套配置拒启动 /
//! 未设旋钮明文回环不变 / 非回环逃生阀与回环语义不变。

use std::sync::Arc;

use apeireth_gateway::{
    bind_scope, build_gateway_state_with_services, enforce_remote_bind, load_tls_material,
    resolve_tls_material_from, serve_state_with_shutdown, serve_state_with_tls,
    tls_surface_status_from, BindScope, GatewayServices, GatewayState, GatewayTokenGate,
    TlsConfigError, TlsSurface, TLS_CERT_ENV, TLS_KEY_ENV,
};
use apeireth_runtime::canonical::Runtime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/tls/{name}", env!("CARGO_MANIFEST_DIR"))
}

async fn gateway_state() -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    build_gateway_state_with_services(runtime, GatewayServices::default())
}

/// live TLS 握手 E2E: 合法证书链 = 真启用 + 真握手 + 真校验。
#[tokio::test]
async fn a_valid_chain_really_serves_tls_and_the_handshake_completes() {
    let material = load_tls_material(&fixture("server-chain.pem"), &fixture("server-key.pem"))
        .expect("合法材料必须真加载成功");
    assert_eq!(material.chain_len(), 2);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_state_with_tls(
        listener,
        gateway_state().await,
        Some(material),
        async move {
            let _ = shutdown_rx.await;
        },
    ));

    // 以测试 CA 为根锚的 HTTPS 客户端: 走真 TLS 握手, 零跳过校验。
    let ca = fs_err::read(fixture("ca-cert.pem")).unwrap();
    let client = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(&ca).expect("测试 CA 解析"))
        .build()
        .unwrap();
    let response = client
        .get(format!("https://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("真 TLS 握手 + 请求必须成功");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("\"ok\""), "{body}");

    // 明文客户端撞 TLS 口: 拿不到任何明文响应字节 (无降级口子)。
    let mut plain = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    plain
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap();
    let mut buffer = Vec::new();
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        plain.read_to_end(&mut buffer),
    )
    .await;
    match read {
        // 超时/连接被掐都算通过: 关键钉的是明文请求换不来明文 HTTP 响应。
        Err(_) => {}
        Ok(Err(_)) => {}
        Ok(Ok(bytes)) => {
            let plaintext_http = buffer.windows(4).any(|window| window == b"HTTP");
            assert!(
                !plaintext_http,
                "明文请求不得拿到明文 HTTP 响应: {buffer:?}"
            );
            if bytes > 0 {
                // 有回字节也只允许是 TLS 记录帧 (握手失败告警), 不许是明文。
                assert!(
                    (0x15..=0x17).contains(&buffer[0]),
                    "非明文响应字节只允许是 TLS 记录帧: {buffer:?}"
                );
            }
        }
    }

    // 不信测试 CA 的客户端: 握手必须失败 (证书校验真开着, 不是摆设)。
    let stranger = reqwest::Client::builder().build().unwrap();
    assert!(
        stranger
            .get(format!("https://127.0.0.1:{port}/health"))
            .send()
            .await
            .is_err(),
        "无根锚的信任链必须握手失败"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap().expect("TLS 服务面优雅收尾");
}

/// 未设旋钮 = 明文回环不变; 旋钮指向坏材料 = 服务入口 fail-closed 拒启动。
///
/// (两段顺序钉在同一个测试里: 环境旋钮是进程级状态, 并行测试各自改环境
/// 会互踩, 故按顺序在同一测试内走完两面。)
#[tokio::test]
async fn unset_knobs_keep_plaintext_loopback_while_bad_material_refuses_to_start() {
    // 未设旋钮: 明文回环照常服务。
    std::env::remove_var(TLS_CERT_ENV);
    std::env::remove_var(TLS_KEY_ENV);
    assert_eq!(
        tls_surface_status_from(None, None),
        TlsSurface::Unconfigured,
        "未设旋钮 = 未配置 (明文缺省)"
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_state_with_shutdown(
        listener,
        gateway_state().await,
        async move {
            let _ = shutdown_rx.await;
        },
    ));
    let response = reqwest::Client::builder()
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("明文回环必须照常服务");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let _ = shutdown_tx.send(());
    server.await.unwrap().expect("明文面优雅收尾");

    // 旋钮指向坏材料: 服务入口拒启动 (不是警告、不是降级明文)。
    std::env::set_var(TLS_CERT_ENV, fixture("no-such-cert.pem"));
    std::env::set_var(TLS_KEY_ENV, fixture("no-such-key.pem"));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let err = serve_state_with_shutdown(
        listener,
        gateway_state().await,
        std::future::pending::<()>(),
    )
    .await
    .expect_err("坏材料必须拒启动");
    let text = err.to_string();
    assert!(text.contains("TLS configuration refused"), "{text}");
    assert!(text.contains("fail-closed"), "{text}");
    assert!(text.contains("no-such-cert.pem"), "{text}");

    std::env::remove_var(TLS_CERT_ENV);
    std::env::remove_var(TLS_KEY_ENV);
}

/// 半套配置 = 配置矛盾, 拒启动 (双向)。
#[tokio::test]
async fn half_configured_knobs_refuse_to_start() {
    let cert = fixture("server-chain.pem");
    let key = fixture("server-key.pem");

    let err = resolve_tls_material_from(Some(&cert), None).expect_err("只给证书拒启动");
    assert!(
        matches!(
            err,
            TlsConfigError::IncompleteConfig {
                cert_env_set: true,
                key_env_set: false
            }
        ),
        "{err:?}"
    );
    let err = resolve_tls_material_from(None, Some(&key)).expect_err("只给私钥拒启动");
    assert!(
        matches!(
            err,
            TlsConfigError::IncompleteConfig {
                cert_env_set: false,
                key_env_set: true
            }
        ),
        "{err:?}"
    );

    // 状态面如实 = Failed, 且拒启动文本在场。
    let surface = tls_surface_status_from(Some(&cert), None);
    assert!(matches!(surface, TlsSurface::Failed { .. }), "{surface:?}");
    let refusal = surface.refusal().expect("半套配置必须给拒启动文本");
    assert!(refusal.contains("refusing to start"), "{refusal}");
}

/// 回环语义与非回环逃生阀语义不变 (TLS 接线零回归)。
#[tokio::test]
async fn loopback_and_escape_valve_semantics_are_unchanged() {
    assert_eq!(bind_scope("127.0.0.1:8080"), BindScope::Loopback);

    // 回环绑定: 恒放行, 无需令牌。
    let cleared = enforce_remote_bind(
        "127.0.0.1",
        &GatewayTokenGate::UnprotectedDefault,
        &Default::default(),
    )
    .expect("回环恒放行");
    assert_eq!(cleared.scope, BindScope::Loopback);
    assert!(cleared.audit_facts.is_empty(), "回环不额外留痕");

    // 非回环逃生阀 (`APEIRETH_GATEWAY_TOKEN=none`): 放行但留审计。
    let cleared = enforce_remote_bind(
        "0.0.0.0",
        &GatewayTokenGate::UnprotectedDeclared,
        &Default::default(),
    )
    .expect("逃生阀放行 (操作者明示自担)");
    assert_eq!(cleared.scope, BindScope::NonLoopback);
    let detail = cleared.audit_facts[0].1.clone().unwrap_or_default();
    assert!(detail.contains("mode=declared_none"), "{detail}");
    assert!(detail.contains("escape valve"), "{detail}");
}
