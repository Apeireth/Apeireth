//! TLS 服务面钉测 (安全加固批 B5 + B5-TEST 补测): 真加载 → 真启用 → 真握手;
//! 坏材料拒启动。
//!
//! 0 装口径 (实测矩阵, 2026-10-21 复测; 每格给出可查的测试名):
//!
//! | 维度 | 口径 | 钉住的测试 |
//! | --- | --- | --- |
//! | 合法链真启用 + live 握手 E2E | 已实测 | `a_valid_chain_really_serves_tls_and_the_handshake_completes` |
//! | 协议版本 1.2 / 1.3 分别钉版本握手 | 已实测 | `the_protocol_version_matrix_negotiates_each_pinned_version` |
//! | RSA-2048 测试链 (加载 + 握手) | 已实测 | 同第一行 (该行材料即 RSA 链) |
//! | ECDSA P-256 测试链 (加载 + 握手) | 已实测 | `ecdsa_p256_material_loads_and_really_handshakes` |
//! | 冷启动换证书轮换 + 旧信任面拒 | 已实测 | `certificate_rotation_is_a_cold_start_swap_and_stale_trust_is_refused` |
//! | 坏文件 / 半套配置拒启动 | 已实测 | `unset_knobs_keep_plaintext_loopback_while_bad_material_refuses_to_start`、`half_configured_knobs_refuse_to_start` |
//! | 未设旋钮明文回环 / 逃生阀不变 | 已实测 | `unset_knobs_keep_plaintext_loopback_while_bad_material_refuses_to_start`、`loopback_and_escape_valve_semantics_are_unchanged` |
//! | 生产 (非自签) 证书面 | **未实测** | 无真实生产证书可用 (自签材料零真实信任价值); 验证清单见下 |
//! | 运行中热重载证书 | **未接线** | 如实标注: 材料只在启动加载一次, 换证书 = 重启 (见轮换测试注释) |
//!
//! 生产证书面 (未实测) 的可复跑验证清单 —— 操作者持真实材料执行:
//!
//! 1. 备真实链 `fullchain.pem` (叶 + 中间, 叶在前) 与私钥 `privkey.pem`
//!    (PKCS#8 / PKCS#1 / SEC1 任一);
//! 2. `APEIRETH_TLS_CERT=fullchain.pem` `APEIRETH_TLS_KEY=privkey.pem`
//!    启动网关 (非回环绑定照旧过令牌闸): 期望出现 `TLS enabled: ...`
//!    接线确认行, 对外 URL 为 `https://`, 且无任何明文降级提示;
//! 3. 任一 TLS 客户端对 `https://<域名>:<端口>/health` 发 GET: 期望 200、
//!    证书链校验通过 (叶 + 中间完整)、协商版本 ∈ {1.2, 1.3};
//! 4. 明文客户端对同端口发 HTTP 请求: 期望拿不到明文响应 (至多 TLS 告警);
//! 5. 换一把不配对私钥重启: 期望启动被拒, 错误文本点名配对失败
//!    (`PairingRefused` 口径)。
//!
//! 测试夹具全部为一次性生成的自签材料, 生成口径见
//! `tests/fixtures/tls/README.md`; 私钥仅测试用途、无任何真实信任价值。
//!
//! 每条钉住的行为: 合法证书链真启用并真握手 / 坏文件拒启动 / 半套配置拒启动 /
//! 未设旋钮明文回环不变 / 非回环逃生阀与回环语义不变 / 分协议版本握手 /
//! ECC 材料加载与握手 / 冷启动轮换语义 (旧信任面拒)。

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

// ── B5-TEST 补测面: 协议矩阵 / ECC 材料 / 冷启动轮换 ────────────────────────

/// 测试用客户端 TLS 面: 以指定 CA 为根锚; `pinned_versions` 非空即钉协议版本。
fn test_tls_client(
    ca_path: &str,
    pinned_versions: &[&'static rustls::SupportedProtocolVersion],
) -> tokio_rustls::TlsConnector {
    use rustls_pki_types::pem::PemObject;
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_pki_types::CertificateDer::pem_file_iter(ca_path).expect("CA PEM 可读") {
        roots
            .add(cert.expect("CA 段可解析"))
            .expect("CA 可入根锚库");
    }
    let builder = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ));
    let builder = if pinned_versions.is_empty() {
        builder
            .with_safe_default_protocol_versions()
            .expect("默认版本集合法")
    } else {
        builder
            .with_protocol_versions(pinned_versions)
            .expect("钉住的版本集合法")
    };
    let config = builder.with_root_certificates(roots).with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// 对服务口完成一次真握手, 回协商出的协议版本 (失败回可断言的病因字符串)。
async fn handshake_version(
    connector: &tokio_rustls::TlsConnector,
    port: u16,
) -> Result<rustls::ProtocolVersion, String> {
    let tcp = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|error| error.to_string())?;
    let server_name = rustls_pki_types::ServerName::try_from("localhost")
        .map_err(|error| error.to_string())?
        .to_owned();
    let stream = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        connector.connect(server_name, tcp),
    )
    .await
    .map_err(|_| "handshake timeout".to_string())?
    .map_err(|error| error.to_string())?;
    stream
        .get_ref()
        .1
        .protocol_version()
        .ok_or_else(|| "handshake produced no negotiated version".to_string())
}

/// 起一个 TLS 服务面 (显式材料), 回 (端口, 服务任务, 停机闸)。
async fn serve_tls_on(
    material: apeireth_gateway::TlsMaterial,
) -> (
    u16,
    tokio::task::JoinHandle<std::io::Result<()>>,
    tokio::sync::oneshot::Sender<()>,
) {
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
    (port, server, shutdown_tx)
}

/// 协议版本矩阵 (B5-TEST ①): 客户端钉 1.2、钉 1.3 各真握手一次, 断言协商出
/// 预期版本。服务端配置含 TLS 1.2 面 (rustls tls12 feature B5 已开)。
#[tokio::test]
async fn the_protocol_version_matrix_negotiates_each_pinned_version() {
    let material = load_tls_material(&fixture("server-chain.pem"), &fixture("server-key.pem"))
        .expect("服务端材料真加载");
    let (port, server, shutdown_tx) = serve_tls_on(material).await;

    let cases: [(
        &[&'static rustls::SupportedProtocolVersion],
        rustls::ProtocolVersion,
    ); 2] = [
        (&[&rustls::version::TLS12], rustls::ProtocolVersion::TLSv1_2),
        (&[&rustls::version::TLS13], rustls::ProtocolVersion::TLSv1_3),
    ];
    for (pinned, expected) in cases {
        let connector = test_tls_client(&fixture("ca-cert.pem"), pinned);
        let negotiated = handshake_version(&connector, port)
            .await
            .expect("钉版本握手必须成功");
        assert_eq!(negotiated, expected, "客户端钉版本握手须协商出预期版本");
    }

    let _ = shutdown_tx.send(());
    server.await.unwrap().expect("TLS 服务面优雅收尾");
}

/// ECC 面 (B5-TEST ②): ECDSA P-256 自签链真加载 + 真握手 (第二算法族材料面)。
#[tokio::test]
async fn ecdsa_p256_material_loads_and_really_handshakes() {
    let material = load_tls_material(
        &fixture("server-ecdsa-chain.pem"),
        &fixture("server-ecdsa-key.pem"),
    )
    .expect("ECDSA P-256 材料必须真加载成功");
    assert_eq!(material.chain_len(), 2, "ECDSA 链两段都要吃进");

    let (port, server, shutdown_tx) = serve_tls_on(material).await;
    let connector = test_tls_client(&fixture("ca-ecdsa-cert.pem"), &[]);
    let negotiated = handshake_version(&connector, port)
        .await
        .expect("ECDSA P-256 链真握手必须成功");
    assert!(
        matches!(
            negotiated,
            rustls::ProtocolVersion::TLSv1_2 | rustls::ProtocolVersion::TLSv1_3
        ),
        "{negotiated:?}"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap().expect("TLS 服务面优雅收尾");
}

/// 证书轮换 (B5-TEST ③) = **冷启动换材料**: 重启后新证书真生效, 只信旧证书面
/// 的客户端被拒 (不许默默沿用旧证书)。
///
/// 如实标注: **热重载未接线** —— 材料只在启动加载一次 (serve 入口
/// `resolve_tls_material()`), 运行中替换 PEM 文件不会生效, 换证书 = 重启。
#[tokio::test]
async fn certificate_rotation_is_a_cold_start_swap_and_stale_trust_is_refused() {
    // 第一轮启动: 证书面 A;信 A 的客户端握手成功 (旧证书面生效)。
    let material_a = load_tls_material(&fixture("server-chain.pem"), &fixture("server-key.pem"))
        .expect("轮换前材料加载");
    let (port_a, server_a, shutdown_a) = serve_tls_on(material_a).await;
    let client_a = test_tls_client(&fixture("ca-cert.pem"), &[]);
    handshake_version(&client_a, port_a)
        .await
        .expect("旧证书面生效时握手必须成功");
    let _ = shutdown_a.send(());
    server_a.await.unwrap().expect("第一轮收尾");

    // 第二轮启动 (冷启动边界): 证书面 B —— 换证书 = 重启, 无热重载。
    let material_b =
        load_tls_material(&fixture("server-b-chain.pem"), &fixture("server-b-key.pem"))
            .expect("轮换后材料加载");
    let (port_b, server_b, shutdown_b) = serve_tls_on(material_b).await;

    // 新证书真生效: 信 B 的客户端握手成功。
    let client_b = test_tls_client(&fixture("ca-b-cert.pem"), &[]);
    handshake_version(&client_b, port_b)
        .await
        .expect("重启后新证书面必须生效");

    // 旧证书面已下线: 只信 A 的客户端必须被拒。
    let err = handshake_version(&client_a, port_b)
        .await
        .expect_err("只信旧证书面的客户端必须被拒");
    assert!(!err.is_empty(), "拒绝要有因");

    let _ = shutdown_b.send(());
    server_b.await.unwrap().expect("第二轮收尾");
}
