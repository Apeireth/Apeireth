//! 矿脉 · 首个外部数据库驱动（PostgreSQL 线协议，批次推进）。
//!
//! 契约（I-starport-builtins.md「矿脉」外部驱动点亮）：
//!   1. 真协议 —— 启动握手（协议 3.0）→ 认证（Ok/明文口令/MD5/SCRAM-SHA-256
//!      认证面全亮）→ 简单查询（'Q'）→ RowDescription/DataRow/CommandComplete
//!      逐帧解析；扩展查询协议为**驱动扩展缝**（如实报，不假通）；
//!   2. 传输纪律 —— TLS 经 SSLRequest 协商（'S' 放行/'N' 拒绝如实报）；
//!      明文通道沿用信使同款双确认（`plain-insecure` + allow 标志）；
//!   3. 只读口径 —— 复用 sqlite_workbench::ensure_readonly_sql（纵深与
//!      SQLite 面同一道闸），行上限同一 [`sqlite_workbench::SQL_ROW_LIMIT`]；
//!   4. 0 装 —— 文本格式值如实按字符串/NULL 回传（不猜类型）；协议漂移
//!      /错误帧（ErrorResponse）带服务端原话如实报。
//!
//! 测试：`cargo test`（纯函数 + 本地假服务端端到端）。

use std::time::{Duration, Instant};

use base64::Engine as _;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rand::RngCore;
use sha2::Sha256;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::sqlite_workbench::{ensure_readonly_sql, SqlQueryResult, SQL_ROW_LIMIT};

/// 全程超时（连接/握手/每帧读取共用）。
pub const PG_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, serde::Deserialize)]
pub struct PgConfig {
    pub host: String,
    pub port: u16,
    /// 'require'（SSLRequest+TLS，拒绝即失败）| 'plain-insecure'（明文，需 allow）。
    pub tls: String,
    #[serde(default)]
    pub allow_insecure_plain: bool,
    pub user: String,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

/// 配置校验（纯函数，0 装；与信使同纪律）。
pub fn validate_config(config: &PgConfig) -> Result<(), String> {
    if config.host.trim().is_empty() {
        return Err("数据库主机不能为空".to_string());
    }
    if config.port == 0 {
        return Err("数据库端口非法".to_string());
    }
    if config.user.trim().is_empty() {
        return Err("数据库用户不能为空".to_string());
    }
    match config.tls.as_str() {
        "require" => {}
        "plain-insecure" => {
            if !config.allow_insecure_plain {
                return Err(
                    "明文通道需显式 allow_insecure_plain（口令将走明线，确认再连）".to_string(),
                );
            }
        }
        other => {
            return Err(format!(
                "未知 TLS 模式（仅 require / plain-insecure）：{other}"
            ))
        }
    }
    Ok(())
}

/// SSLRequest 帧（8 字节定长：长度 + 魔数 80877103）。
pub fn build_ssl_request() -> [u8; 8] {
    let mut frame = [0u8; 8];
    frame[..4].copy_from_slice(&8i32.to_be_bytes());
    frame[4..].copy_from_slice(&80_877_103i32.to_be_bytes());
    frame
}

/// 启动帧（协议 3.0：长度 + 196608 + user/database 键值 + 尾 0）。
pub fn build_startup(user: &str, database: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&196_608i32.to_be_bytes());
    body.extend_from_slice(b"user\0");
    body.extend_from_slice(user.as_bytes());
    body.push(0);
    if let Some(db) = database {
        body.extend_from_slice(b"database\0");
        body.extend_from_slice(db.as_bytes());
        body.push(0);
    }
    body.push(0);
    let mut frame = Vec::with_capacity(body.len() + 4);
    frame.extend_from_slice(&((body.len() + 4) as i32).to_be_bytes());
    frame.extend_from_slice(&body);
    frame
}

/// 口令帧（'p' + 长度 + 口令 + 尾 0）。
pub fn build_password(password: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(password.len() + 5);
    payload.push(b'p');
    payload.extend_from_slice(&((password.len() + 5) as i32).to_be_bytes());
    payload.extend_from_slice(password.as_bytes());
    payload.push(0);
    payload
}

/// 查询帧（'Q' + SQL + 尾 0）。
pub fn build_query(sql: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(sql.len() + 6);
    payload.push(b'Q');
    payload.extend_from_slice(&((sql.len() + 5) as i32).to_be_bytes());
    payload.extend_from_slice(sql.as_bytes());
    payload.push(0);
    payload
}

/// 认证帧载荷首 4 字节 = 认证种类（0 Ok / 3 明文口令 / 5 MD5 / 10+ SCRAM）。
pub fn parse_auth_kind(payload: &[u8]) -> Result<u32, String> {
    if payload.len() < 4 {
        return Err("认证帧畸形".to_string());
    }
    Ok(u32::from_be_bytes([
        payload[0], payload[1], payload[2], payload[3],
    ]))
}

/// 十六进制小写编码（认证哈希口径用）。
fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// PG MD5 认证响应（标准口径）：`md5` + md5_hex( md5_hex(password‖user) 的
/// 十六进制文本 ‖ salt 四字节 )。内层对「口令在前、用户在后」的拼接取 MD5
/// 十六进制文本；外层对该文本的 ASCII 字节 ‖ 服务端 4 字节 salt 再取 MD5。
pub fn build_auth_md5(user: &str, password: &str, salt: &[u8]) -> Result<String, String> {
    if salt.len() != 4 {
        return Err(format!("MD5 salt 必须 4 字节（实得 {}）", salt.len()));
    }
    let mut inner = Md5::new();
    inner.update(password.as_bytes());
    inner.update(user.as_bytes());
    let inner_hex = hex_lower(&inner.finalize());

    let mut outer = Md5::new();
    outer.update(inner_hex.as_bytes());
    outer.update(salt);
    Ok(format!("md5{}", hex_lower(&outer.finalize())))
}

/// SASL 响应帧（'p' + 长度 + 原始字节，无尾零；SCRAM 各步用）。
pub fn build_sasl_response(response: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(response.len() + 5);
    payload.push(b'p');
    payload.extend_from_slice(&((response.len() + 4) as i32).to_be_bytes());
    payload.extend_from_slice(response);
    payload
}

/// SCRAM-SHA-256 客户端首报文（GS2 头 "n,," + client-first-bare）。
pub fn build_scram_client_first(client_nonce: &str) -> String {
    format!("n,,n=,r={client_nonce}")
}

/// 随机客户端 nonce（18 字节 → base64；每次认证一把，只走内存）。
fn random_scram_nonce() -> String {
    let mut bytes = [0u8; 18];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// SCRAM 服务端首报文字段（组合随机数 / salt / 迭代数）。
pub struct ScramFirst {
    pub combined_nonce: String,
    pub salt: Vec<u8>,
    pub iterations: u32,
}

/// 解析服务端首报文（r=…,s=…,i=…）并核验组合随机数以客户端随机数为前缀
/// （RFC 5802 §5.1：不匹配即拒，防随机数调包）。
pub fn parse_scram_server_first(
    client_nonce: &str,
    server_first: &str,
) -> Result<ScramFirst, String> {
    let fields = parse_scram_first_fields(server_first)?;
    if !fields.combined_nonce.starts_with(client_nonce) {
        return Err("SCRAM 组合随机数未以客户端随机数为前缀（疑似调包）".to_string());
    }
    Ok(fields)
}

fn parse_scram_first_fields(server_first: &str) -> Result<ScramFirst, String> {
    let mut nonce = None;
    let mut salt = None;
    let mut iterations = None;
    for part in server_first.split(',') {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("SCRAM 首报文字段畸形：{part}"))?;
        match key {
            "r" => nonce = Some(value.to_string()),
            "s" => {
                salt = Some(
                    base64::engine::general_purpose::STANDARD
                        .decode(value)
                        .map_err(|error| format!("SCRAM salt 解码失败：{error}"))?,
                );
            }
            "i" => {
                iterations = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("SCRAM 迭代数非法：{value}"))?,
                );
            }
            _ => {}
        }
    }
    Ok(ScramFirst {
        combined_nonce: nonce.ok_or("SCRAM 首报文缺 r")?,
        salt: salt.ok_or("SCRAM 首报文缺 s")?,
        iterations: iterations.ok_or("SCRAM 首报文缺 i")?,
    })
}

/// SCRAM 核心派生（RFC 5802 §3 + RFC 7677 SHA-256）：
/// SaltedPassword=PBKDF2-HMAC-SHA256(password, salt, i, 32)；
/// ClientKey=HMAC(SaltedPassword,"Client Key")；StoredKey=SHA256(ClientKey)；
/// ClientSignature=HMAC(StoredKey, AuthMessage)；Proof=ClientKey⊕ClientSignature；
/// ServerKey=HMAC(SaltedPassword,"Server Key")；ServerSignature=HMAC(ServerKey, AuthMessage)；
/// AuthMessage = client-first-bare ‖ "," ‖ server-first ‖ "," ‖ client-final-without-proof。
/// 返回 (client_proof_b64, server_signature_b64)。
pub fn scram_derive(
    password: &str,
    client_first_bare: &str,
    server_first: &str,
    client_final_without_proof: &str,
) -> Result<(String, String), String> {
    let fields = parse_scram_first_fields(server_first)?;
    let mut salted = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        &fields.salt,
        fields.iterations,
        &mut salted,
    );
    let client_key = hmac_sha256(&salted, b"Client Key");
    let stored_key = Sha256::digest(&client_key);
    let auth_message = format!("{client_first_bare},{server_first},{client_final_without_proof}");
    let client_signature = hmac_sha256(&stored_key, auth_message.as_bytes());
    let proof: Vec<u8> = client_key
        .iter()
        .zip(client_signature.iter())
        .map(|(left, right)| left ^ right)
        .collect();
    let server_key = hmac_sha256(&salted, b"Server Key");
    let server_signature = hmac_sha256(&server_key, auth_message.as_bytes());
    Ok((
        base64::engine::general_purpose::STANDARD.encode(proof),
        base64::engine::general_purpose::STANDARD.encode(server_signature),
    ))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

/// 校验服务端终报文（v=<b64 签名>）：不符抛错（0 装：不放行）。
pub fn verify_scram_server_final(
    expected_signature_b64: &str,
    server_final: &str,
) -> Result<(), String> {
    let value = server_final
        .strip_prefix("v=")
        .ok_or_else(|| format!("SCRAM 终报文缺 v= 签名：{server_final}"))?;
    if value.trim() != expected_signature_b64 {
        return Err("SCRAM 服务端签名不符（会话可能被劫持）".to_string());
    }
    Ok(())
}

/// RowDescription → 列名序（文本格式；每列名后 18 字节元数据）。
pub fn parse_row_description(payload: &[u8]) -> Result<Vec<String>, String> {
    if payload.len() < 2 {
        return Err("RowDescription 帧畸形".to_string());
    }
    let count = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    let mut names = Vec::with_capacity(count);
    let mut cursor = 2usize;
    for _ in 0..count {
        let end = payload[cursor..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|offset| cursor + offset)
            .ok_or("列名未终结")?;
        names.push(String::from_utf8_lossy(&payload[cursor..end]).into_owned());
        cursor = end + 1 + 18;
        if cursor > payload.len() {
            return Err("RowDescription 越界".to_string());
        }
    }
    Ok(names)
}

/// DataRow → 文本格式值列（-1 长度 = NULL；值如实按字符串，不猜类型）。
pub fn parse_data_row(payload: &[u8]) -> Result<Vec<Option<String>>, String> {
    if payload.len() < 2 {
        return Err("DataRow 帧畸形".to_string());
    }
    let count = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    let mut values = Vec::with_capacity(count);
    let mut cursor = 2usize;
    for _ in 0..count {
        if cursor + 4 > payload.len() {
            return Err("DataRow 越界".to_string());
        }
        let len = i32::from_be_bytes([
            payload[cursor],
            payload[cursor + 1],
            payload[cursor + 2],
            payload[cursor + 3],
        ]);
        cursor += 4;
        if len < 0 {
            values.push(None);
            continue;
        }
        let end = cursor + len as usize;
        if end > payload.len() {
            return Err("DataRow 值越界".to_string());
        }
        values.push(Some(
            String::from_utf8_lossy(&payload[cursor..end]).into_owned(),
        ));
        cursor = end;
    }
    Ok(values)
}

/// ErrorResponse → 服务端原话（'M' 字段；缺则如实报帧长）。
pub fn parse_error_message(payload: &[u8]) -> String {
    let mut cursor = 0usize;
    while cursor < payload.len() {
        let code = payload[cursor];
        cursor += 1;
        let end = payload[cursor..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|offset| cursor + offset)
            .unwrap_or(payload.len());
        if code == b'M' {
            return String::from_utf8_lossy(&payload[cursor..end]).into_owned();
        }
        cursor = end + 1;
    }
    format!("未识别错误帧（{} 字节）", payload.len())
}

/// 读一帧（类型字节 + Int32 长度 + 载荷；长度含自身 4 字节）。
async fn read_message<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<(u8, Vec<u8>), String> {
    let mut header = [0u8; 5];
    tokio::time::timeout(
        Duration::from_millis(PG_TIMEOUT_MS),
        reader.read_exact(&mut header),
    )
    .await
    .map_err(|_| "读帧超时".to_string())?
    .map_err(|error| format!("读帧失败（如实报）：{error}"))?;
    let len = i32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    if !(4..=64 * 1024 * 1024).contains(&len) {
        return Err(format!("帧长度非法：{len}"));
    }
    let mut payload = vec![0u8; (len - 4) as usize];
    if !payload.is_empty() {
        tokio::time::timeout(
            Duration::from_millis(PG_TIMEOUT_MS),
            reader.read_exact(&mut payload),
        )
        .await
        .map_err(|_| "读帧超时".to_string())?
        .map_err(|error| format!("读帧失败（如实报）：{error}"))?;
    }
    Ok((header[0], payload))
}

fn tls_provider() -> std::sync::Arc<rustls::crypto::CryptoProvider> {
    if let Some(provider) = rustls::crypto::CryptoProvider::get_default() {
        return provider.clone();
    }
    let _ =
        rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider());
    rustls::crypto::CryptoProvider::get_default()
        .map(std::sync::Arc::clone)
        .expect("ring 提供者刚安装必在")
}

/// 只读查询一个真实 PostgreSQL 服务（文本格式值、有界结果、截断如实标注）。
#[tauri::command]
pub async fn query_postgres(config: PgConfig, sql: String) -> Result<SqlQueryResult, String> {
    validate_config(&config)?;
    let checked = ensure_readonly_sql(&sql)?;
    let started = Instant::now();
    let addr = format!("{}:{}", config.host.trim(), config.port);
    let mut stream = tokio::time::timeout(
        Duration::from_millis(PG_TIMEOUT_MS),
        TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| format!("连接超时：{addr}"))?
    .map_err(|error| format!("连接失败（如实报）：{error}"))?;

    // 传输分流（时序纪律：SSLRequest 协商必须在包 TLS 之前、流未拆之时）。
    let (mut reader, mut writer): (
        BufReader<Box<dyn AsyncRead + Send + Unpin>>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) = if config.tls == "require" {
        stream
            .write_all(&build_ssl_request())
            .await
            .map_err(|error| format!("SSLRequest 写失败：{error}"))?;
        stream
            .flush()
            .await
            .map_err(|error| format!("SSLRequest 刷写失败：{error}"))?;
        let mut verdict = [0u8; 1];
        tokio::time::timeout(
            Duration::from_millis(PG_TIMEOUT_MS),
            stream.read_exact(&mut verdict),
        )
        .await
        .map_err(|_| "SSL 协商超时".to_string())?
        .map_err(|error| format!("SSL 协商读失败：{error}"))?;
        if verdict[0] != b'S' {
            return Err("服务端拒绝 TLS（SSLRequest 被拒）——改明文需双确认".to_string());
        }
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls_config = rustls::ClientConfig::builder_with_provider(tls_provider())
            .with_safe_default_protocol_versions()
            .map_err(|error| format!("TLS 版本协商失败：{error}"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(tls_config));
        let server_name = rustls::pki_types::ServerName::try_from(config.host.trim().to_string())
            .map_err(|error| format!("主机名非法：{error}"))?;
        let tls_stream = tokio::time::timeout(
            Duration::from_millis(PG_TIMEOUT_MS),
            connector.connect(server_name, stream),
        )
        .await
        .map_err(|_| "TLS 握手超时".to_string())?
        .map_err(|error| format!("TLS 握手失败（如实报）：{error}"))?;
        let (read_half, write_half) = tokio::io::split(tls_stream);
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    } else {
        let (read_half, write_half) = stream.into_split();
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    };

    // 启动握手 + 认证。
    writer
        .write_all(&build_startup(
            config.user.trim(),
            config.database.as_deref().map(str::trim),
        ))
        .await
        .map_err(|error| format!("启动帧写失败：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败：{error}"))?;
    loop {
        let (kind, payload) = read_message(&mut reader).await?;
        match kind {
            b'R' => match parse_auth_kind(&payload)? {
                0 => {}
                3 => {
                    let password = config
                        .password
                        .as_deref()
                        .ok_or("服务端要求口令但未提供（0 装：不发空口令）")?;
                    writer
                        .write_all(&build_password(password))
                        .await
                        .map_err(|error| format!("口令帧写失败：{error}"))?;
                    writer
                        .flush()
                        .await
                        .map_err(|error| format!("刷写失败：{error}"))?;
                }
                5 => {
                    let password = config
                        .password
                        .as_deref()
                        .ok_or("服务端要求口令但未提供（0 装：不发空口令）")?;
                    if payload.len() < 8 {
                        return Err("MD5 认证帧缺 salt".to_string());
                    }
                    let response = build_auth_md5(config.user.trim(), password, &payload[4..8])?;
                    writer
                        .write_all(&build_password(&response))
                        .await
                        .map_err(|error| format!("MD5 响应写失败：{error}"))?;
                    writer
                        .flush()
                        .await
                        .map_err(|error| format!("刷写失败：{error}"))?;
                }
                10 => {
                    let password = config
                        .password
                        .as_deref()
                        .ok_or("服务端要求口令但未提供（0 装：不发空口令）")?;
                    let mechanisms = String::from_utf8_lossy(&payload[4..]);
                    if !mechanisms
                        .split('\0')
                        .any(|mechanism| mechanism == "SCRAM-SHA-256")
                    {
                        return Err("服务端 SASL 机制不含 SCRAM-SHA-256（驱动扩展缝）".to_string());
                    }
                    let client_nonce = random_scram_nonce();
                    let client_first = build_scram_client_first(&client_nonce);
                    let client_first_bare =
                        client_first.strip_prefix("n,,").expect("构造即含 GS2 头");
                    // SASLInitialResponse：机制\0 + 响应长度 + 首报文。
                    let mut initial = Vec::new();
                    initial.extend_from_slice(b"SCRAM-SHA-256\0");
                    initial.extend_from_slice(&(client_first.len() as i32).to_be_bytes());
                    initial.extend_from_slice(client_first.as_bytes());
                    writer
                        .write_all(&build_sasl_response(&initial))
                        .await
                        .map_err(|error| format!("SCRAM 首报文写失败：{error}"))?;
                    writer
                        .flush()
                        .await
                        .map_err(|error| format!("刷写失败：{error}"))?;

                    // SASLContinue（种类 11）→ 派生 → client-final。
                    let (step_kind, step_payload) = read_message(&mut reader).await?;
                    if step_kind != b'R' || parse_auth_kind(&step_payload)? != 11 {
                        return Err("SCRAM 期待 SASLContinue（认证种类 11），协议漂移".to_string());
                    }
                    let server_first = String::from_utf8_lossy(&step_payload[4..]).into_owned();
                    let first = parse_scram_server_first(&client_nonce, &server_first)?;
                    let client_final_without_proof = format!("c=biws,r={}", first.combined_nonce);
                    let (proof, server_signature) = scram_derive(
                        password,
                        client_first_bare,
                        &server_first,
                        &client_final_without_proof,
                    )?;
                    let client_final = format!("{client_final_without_proof},p={proof}");
                    writer
                        .write_all(&build_sasl_response(client_final.as_bytes()))
                        .await
                        .map_err(|error| format!("SCRAM 终报文写失败：{error}"))?;
                    writer
                        .flush()
                        .await
                        .map_err(|error| format!("刷写失败：{error}"))?;

                    // SASLFinal（种类 12）→ 验服务端签名。
                    let (step_kind, step_payload) = read_message(&mut reader).await?;
                    if step_kind != b'R' || parse_auth_kind(&step_payload)? != 12 {
                        return Err("SCRAM 期待 SASLFinal（认证种类 12），协议漂移".to_string());
                    }
                    let server_final = String::from_utf8_lossy(&step_payload[4..]).into_owned();
                    verify_scram_server_final(&server_signature, &server_final)?;
                }
                other => return Err(format!("未知认证方式：{other}")),
            },
            b'S' | b'K' => {}
            b'Z' => break,
            b'E' => return Err(format!("启动失败：{}", parse_error_message(&payload))),
            other => return Err(format!("启动阶段协议漂移（帧类型 {}）", other as char)),
        }
    }

    // 简单查询。
    writer
        .write_all(&build_query(checked))
        .await
        .map_err(|error| format!("查询帧写失败：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败：{error}"))?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
    let mut truncated = false;
    loop {
        let (kind, payload) = read_message(&mut reader).await?;
        match kind {
            b'T' => columns = parse_row_description(&payload)?,
            b'D' => {
                if rows.len() >= SQL_ROW_LIMIT {
                    truncated = true;
                    continue;
                }
                let values = parse_data_row(&payload)?;
                rows.push(
                    values
                        .into_iter()
                        .map(|value| match value {
                            Some(text) => serde_json::Value::String(text),
                            None => serde_json::Value::Null,
                        })
                        .collect(),
                );
            }
            b'C' => {}
            b'E' => return Err(format!("查询失败：{}", parse_error_message(&payload))),
            b'Z' => break,
            other => return Err(format!("查询阶段协议漂移（帧类型 {}）", other as char)),
        }
    }

    Ok(SqlQueryResult {
        columns,
        row_count: rows.len(),
        truncated,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> PgConfig {
        PgConfig {
            host: "db.example.test".to_string(),
            port: 5432,
            tls: "require".to_string(),
            allow_insecure_plain: false,
            user: "app".to_string(),
            database: Some("main".to_string()),
            password: Some("secret".to_string()),
        }
    }

    #[test]
    fn config_validation_matrix() {
        assert!(validate_config(&base_config()).is_ok());
        let mut bad = base_config();
        bad.host = " ".to_string();
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.port = 0;
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.user = "".to_string();
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.tls = "prefer".to_string();
        assert!(validate_config(&bad).is_err(), "未知模式拒绝");
        let mut bad = base_config();
        bad.tls = "plain-insecure".to_string();
        assert!(validate_config(&bad).is_err(), "明文未确认必须拒绝");
        bad.allow_insecure_plain = true;
        assert!(validate_config(&bad).is_ok(), "明文双确认后放行");
        println!("  [1] config matrix OK");
    }

    #[test]
    fn frames_and_parsers() {
        assert_eq!(
            build_ssl_request(),
            [0, 0, 0, 8, 4, 210, 22, 47],
            "SSLRequest 定长魔数核对"
        );
        let startup = build_startup("app", Some("main"));
        let len = i32::from_be_bytes([startup[0], startup[1], startup[2], startup[3]]) as usize;
        assert_eq!(len, startup.len(), "启动帧长度前缀自洽");
        assert_eq!(&startup[4..8], &196_608i32.to_be_bytes(), "协议 3.0 版本号");
        assert!(
            startup.windows(6).any(|w| w == b"user\0a"),
            "user 键值在帧内"
        );

        let password = build_password("pw");
        assert_eq!(password[0], b'p');
        assert_eq!(
            i32::from_be_bytes([password[1], password[2], password[3], password[4]]) as usize,
            password.len() - 1,
            "长度字段含自身 4 字节但不含类型字节"
        );

        assert_eq!(parse_auth_kind(&[0, 0, 0, 0]).unwrap(), 0);
        assert_eq!(parse_auth_kind(&[0, 0, 0, 3]).unwrap(), 3);
        assert!(parse_auth_kind(&[0, 0]).is_err());

        // RowDescription：2 列（name\0 + 18 字节元数据 ×2）。
        let mut row_desc = vec![0, 2];
        for name in [b"id\0".as_slice(), b"title\0".as_slice()] {
            row_desc.extend_from_slice(name);
            row_desc.extend_from_slice(&[0u8; 18]);
        }
        assert_eq!(
            parse_row_description(&row_desc).unwrap(),
            vec!["id".to_string(), "title".to_string()]
        );

        // DataRow：'abc' / NULL / '' 三列。
        let mut data_row = vec![0, 3];
        data_row.extend_from_slice(&3i32.to_be_bytes());
        data_row.extend_from_slice(b"abc");
        data_row.extend_from_slice(&(-1i32).to_be_bytes());
        data_row.extend_from_slice(&0i32.to_be_bytes());
        assert_eq!(
            parse_data_row(&data_row).unwrap(),
            vec![Some("abc".to_string()), None, Some(String::new())]
        );
        assert!(
            parse_data_row(&[0, 1, 0, 0, 0, 5, b'x']).is_err(),
            "值越界拒绝"
        );

        // ErrorResponse：字段=码字节+值+\0（S 严重度 + M 原话），帧尾双 0。
        let error_frame = b"SERROR\0Mrelation does not exist\0\0";
        assert_eq!(parse_error_message(error_frame), "relation does not exist");
        println!("  [2] frames & parsers OK");
    }

    /// MD5 认证常量向量（独立密码学工具预算定，非实现自证）：
    /// user=app/password=secret/salt=[1,2,3,4]；
    /// inner=md5_hex("secretapp")=6a422f785c9e20873908ce25d1736ae2；
    /// response=md5+md5_hex(inner_hex_ascii‖salt)=md5911f527656472583a006e7727877b33e。
    #[test]
    fn md5_auth_vector() {
        assert_eq!(
            build_auth_md5("app", "secret", &[1, 2, 3, 4]).unwrap(),
            "md5911f527656472583a006e7727877b33e"
        );
        assert!(
            build_auth_md5("app", "secret", &[1, 2, 3]).is_err(),
            "salt 长度钉死 4 字节"
        );
        println!("  [4] md5 auth vector OK");
    }

    /// SCRAM-SHA-256 测试向量（RFC 7677 附录 B 输入：user=user / password=pencil /
    /// nonce / salt=QSXCR+Q6sek8bf92 / i=4096）。输出常量经独立密码学工具
    /// （.NET 全链 PBKDF2-HMAC-SHA256 + HMAC-SHA256）复算一致。防混植注记：
    /// RFC 5802（SHA-1）同输入的证明值 `v0X8v3Bz2T0CJGbJQyF0X+HI4Ts=` 不适用
    /// 本口径——同输入按 SHA-1 链独立复算恰得该值（与已公布 5802 向量吻合），
    /// 佐证协议结构解读无误，两链分歧只在哈希族。
    #[test]
    fn scram_rfc7677_vector() {
        let client_first_bare = "n=user,r=fyko+d2lbbFgONRv9qkxdawL";
        let server_first = "r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,s=QSXCR+Q6sek8bf92,i=4096";
        let client_final_without_proof = "c=biws,r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j";
        let (proof, server_signature) = scram_derive(
            "pencil",
            client_first_bare,
            server_first,
            client_final_without_proof,
        )
        .unwrap();
        assert_eq!(
            proof, "qQRLRHGPDGjB+7iVAE7NNi5xEoHKHuLCHPNQ8BTmvds=",
            "SCRAM-SHA-256 客户端证明（独立工具复算一致）"
        );
        assert_eq!(
            server_signature, "XKW6VuW1FANROQabnJBz1KaeCnQL/HZByQtX/iU+o30=",
            "SCRAM-SHA-256 服务端签名（独立工具复算一致）"
        );
        assert_eq!(build_scram_client_first("abc"), "n,,n=,r=abc");
        println!("  [5] scram sha-256 vector OK");
    }

    #[test]
    fn scram_parse_and_verify() {
        let first = parse_scram_server_first("abc", "r=abcEXT,s=QSXCR+Q6sek8bf92,i=4096").unwrap();
        assert_eq!(first.combined_nonce, "abcEXT");
        assert_eq!(first.iterations, 4096);
        assert!(
            parse_scram_server_first("abc", "r=XYZ,s=QSXCR+Q6sek8bf92,i=4096").is_err(),
            "随机数前缀核验（防调包）"
        );
        assert!(
            parse_scram_server_first("abc", "r=abc,s=QSXCR+Q6sek8bf92").is_err(),
            "缺 i 拒"
        );
        assert!(
            parse_scram_server_first("abc", "r=abc,s=!!!,i=1").is_err(),
            "坏 salt 拒"
        );

        assert!(verify_scram_server_final("sig", "v=sig").is_ok());
        assert!(
            verify_scram_server_final("sig", "v=other").is_err(),
            "签名不符拒"
        );
        assert!(
            verify_scram_server_final("sig", "e=fail").is_err(),
            "缺 v= 拒"
        );
        println!("  [6] scram parse & verify OK");
    }

    /// 本地假服务端端到端：真 TCP + 真协议对账（明文双确认路径 + 明文口令认证）。
    #[tokio::test]
    async fn end_to_end_against_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);

            // 读启动帧（无类型字节：4 字节长度 + 主体）。
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf).await.unwrap();
            let len = i32::from_be_bytes(len_buf) as usize;
            let mut body = vec![0u8; len - 4];
            reader.read_exact(&mut body).await.unwrap();
            let startup = String::from_utf8_lossy(&body).into_owned();
            assert!(
                startup.contains("user\0app"),
                "启动帧携带用户（版本号打头，用包含判定）"
            );

            // 明文口令认证：334 → 口令 → Ok。
            let mut auth = vec![b'R'];
            auth.extend_from_slice(&8i32.to_be_bytes());
            auth.extend_from_slice(&3i32.to_be_bytes());
            write_half.write_all(&auth).await.unwrap();
            // 口令帧：类型字节 'p' + 长度 + 口令（读帧必须先吃类型字节）。
            let mut pass_header = [0u8; 5];
            reader.read_exact(&mut pass_header).await.unwrap();
            assert_eq!(pass_header[0], b'p', "口令帧类型字节");
            let plen = i32::from_be_bytes([
                pass_header[1],
                pass_header[2],
                pass_header[3],
                pass_header[4],
            ]) as usize;
            let mut pass = vec![0u8; plen - 4];
            reader.read_exact(&mut pass).await.unwrap();
            assert_eq!(pass, b"secret\0".to_vec(), "口令帧内容");

            let mut ok = vec![b'R'];
            ok.extend_from_slice(&8i32.to_be_bytes());
            ok.extend_from_slice(&0i32.to_be_bytes());
            write_half.write_all(&ok).await.unwrap();
            let mut ready = vec![b'Z'];
            ready.extend_from_slice(&5i32.to_be_bytes());
            ready.push(b'I');
            write_half.write_all(&ready).await.unwrap();

            // 读查询帧（类型字节 'Q' + 长度 + SQL）。
            let mut q_header = [0u8; 5];
            reader.read_exact(&mut q_header).await.unwrap();
            assert_eq!(q_header[0], b'Q', "查询帧类型字节");
            let qlen =
                i32::from_be_bytes([q_header[1], q_header[2], q_header[3], q_header[4]]) as usize;
            let mut qbody = vec![0u8; qlen - 4];
            reader.read_exact(&mut qbody).await.unwrap();
            assert!(
                String::from_utf8_lossy(&qbody).contains("SELECT"),
                "查询帧携带只读 SQL"
            );

            // 回 RowDescription（2 列）。
            let mut desc_payload = vec![0u8, 2];
            for name in [b"name\0".as_slice(), b"note\0".as_slice()] {
                desc_payload.extend_from_slice(name);
                desc_payload.extend_from_slice(&[0u8; 18]);
            }
            let mut desc = vec![b'T'];
            desc.extend_from_slice(&((desc_payload.len() + 4) as i32).to_be_bytes());
            desc.extend_from_slice(&desc_payload);
            write_half.write_all(&desc).await.unwrap();

            // 回 DataRow（'alice' / NULL）。
            let mut data_payload = vec![0u8, 2];
            data_payload.extend_from_slice(&5i32.to_be_bytes());
            data_payload.extend_from_slice(b"alice");
            data_payload.extend_from_slice(&(-1i32).to_be_bytes());
            let mut data = vec![b'D'];
            data.extend_from_slice(&((data_payload.len() + 4) as i32).to_be_bytes());
            data.extend_from_slice(&data_payload);
            write_half.write_all(&data).await.unwrap();

            let mut complete = vec![b'C'];
            complete.extend_from_slice(&13i32.to_be_bytes());
            complete.extend_from_slice(b"SELECT 1\0");
            write_half.write_all(&complete).await.unwrap();
            write_half.write_all(&ready).await.unwrap();
        });

        let config = PgConfig {
            host: "127.0.0.1".to_string(),
            port,
            tls: "plain-insecure".to_string(),
            allow_insecure_plain: true,
            user: "app".to_string(),
            database: Some("main".to_string()),
            password: Some("secret".to_string()),
        };
        let result = query_postgres(config, "SELECT name, note FROM t".to_string())
            .await
            .unwrap();
        assert_eq!(result.columns, vec!["name", "note"]);
        assert_eq!(result.row_count, 1);
        assert!(!result.truncated);
        assert_eq!(result.rows[0][0], serde_json::json!("alice"));
        assert_eq!(result.rows[0][1], serde_json::Value::Null);

        // 只读口径在连接前即拒（纵深第一道）。
        let mut write_config = base_config();
        write_config.host = "127.0.0.1".to_string();
        write_config.port = port;
        write_config.tls = "plain-insecure".to_string();
        write_config.allow_insecure_plain = true;
        assert!(
            query_postgres(write_config, "DELETE FROM t".to_string())
                .await
                .is_err(),
            "写类必须在连接前拒绝"
        );

        server.await.unwrap();
        println!("  [3] end-to-end fake server OK");
    }
}
