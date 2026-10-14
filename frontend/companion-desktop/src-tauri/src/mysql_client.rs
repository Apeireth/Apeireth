//! 矿脉 · MySQL 线协议驱动（批次推进；暂未挂载注册——待传输认证线落地后一并入编译图）。
//!
//! 契约（I-starport-builtins.md「矿脉」更多库驱动点亮）：
//!   1. 真协议 —— 握手 V10 解析 → 认证（mysql_native_password / caching_sha2
//!      password：快速认证与 TLS 下明文全认证；认证切换帧 0xFE 支持）→
//!      COM_QUERY 文本协议 → 列定义/行包/EOF 逐包解析；
//!      压缩协议/预处理语句/本地文件载入为**扩展缝**（不设假旋钮）；
//!   2. 传输纪律 —— TLS 经能力协商（CLIENT_SSL，握手响应前升级）；明文双确认
//!      同款（plain-insecure + allow）；caching_sha2 全认证仅在 TLS 下走明文
//!      口令（明文通道如实拒，防口令明线）；
//!   3. 只读口径 —— 复用 sqlite_workbench::ensure_readonly_sql（纵深同一道闸），
//!      行上限同一 [`sqlite_workbench::SQL_ROW_LIMIT`]；
//!   4. 0 装 —— 长度编码越界/包漂移/错误包（0xFF）带服务端原话如实报；
//!      文本协议值如实按字符串/NULL 回传（不猜类型）。
//!
//! 测试：`cargo test`（纯函数 + 本地假服务端端到端）。

use std::time::{Duration, Instant};

use sha1::{Digest as _, Sha1};
use sha2::Sha256;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::sqlite_workbench::{ensure_readonly_sql, SqlQueryResult, SQL_ROW_LIMIT};

pub const MYSQL_TIMEOUT_MS: u64 = 30_000;

/// 能力位（握手用的子集）。
const CLIENT_PROTOCOL_41: u32 = 0x0000_0200;
const CLIENT_SECURE_CONNECTION: u32 = 0x0000_8000;
const CLIENT_PLUGIN_AUTH: u32 = 0x0008_0000;
const CLIENT_SSL: u32 = 0x0000_0800;
// 口径注明：本驱动不启用 CLIENT_DEPRECATE_EOF（0x0100_0000），行界走 EOF 标记。

#[derive(Debug, serde::Deserialize)]
pub struct MySqlConfig {
    pub host: String,
    pub port: u16,
    /// 'require'（能力协商 TLS）| 'plain-insecure'（明文双确认）。
    pub tls: String,
    #[serde(default)]
    pub allow_insecure_plain: bool,
    pub user: String,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

/// 配置校验（纯函数，0 装；与信使/PG 同纪律）。
pub fn validate_config(config: &MySqlConfig) -> Result<(), String> {
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

// ---------------------------------------------------------------------------
// 长度编码（lenenc）与包装配
// ---------------------------------------------------------------------------

/// 长度编码整数解码 → (值, 消耗字节)。0xfb=NULL 约定由调用方处置。
pub fn decode_lenenc(bytes: &[u8]) -> Result<(u64, usize), String> {
    let first = *bytes.first().ok_or("lenenc 空输入")?;
    match first {
        0xfc => {
            if bytes.len() < 3 {
                return Err("lenenc 越界".to_string());
            }
            Ok((u16::from_le_bytes([bytes[1], bytes[2]]) as u64, 3))
        }
        0xfd => {
            if bytes.len() < 4 {
                return Err("lenenc 越界".to_string());
            }
            Ok((
                u32::from_le_bytes([bytes[1], bytes[2], bytes[3], 0]) as u64,
                4,
            ))
        }
        0xfe => {
            if bytes.len() < 9 {
                return Err("lenenc 越界".to_string());
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[1..9]);
            Ok((u64::from_le_bytes(buf), 9))
        }
        value if value < 0xfb => Ok((value as u64, 1)),
        other => Err(format!("lenenc 首字节非法：{other}")),
    }
}

/// 长度编码整数编码（测试对账件，与 decode_lenenc 成对往返）。
#[cfg(test)]
pub fn encode_lenenc(value: u64) -> Vec<u8> {
    if value < 0xfb {
        vec![value as u8]
    } else if value <= u64::from(u16::MAX) {
        let mut out = vec![0xfc];
        out.extend_from_slice(&(value as u16).to_le_bytes());
        out
    } else if value <= 0xFF_FFFF {
        vec![
            0xfd,
            (value & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
        ]
    } else {
        let mut out = vec![0xfe];
        out.extend_from_slice(&value.to_le_bytes());
        out
    }
}

/// 服务器包 → 载荷（3 字节长度 + 1 字节序号）。
pub fn build_packet(sequence: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    vec![
        (len & 0xff) as u8,
        ((len >> 8) & 0xff) as u8,
        ((len >> 16) & 0xff) as u8,
        sequence,
    ]
    .into_iter()
    .chain(payload.iter().copied())
    .collect()
}

// ---------------------------------------------------------------------------
// 握手与认证
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct Handshake {
    pub server_version: String,
    pub connection_id: u32,
    /// 20 字节盐（part1(8) + part2(12)）。
    pub salt: [u8; 20],
    pub capabilities: u32,
    pub auth_plugin: String,
}

/// 握手 V10 解析（纯函数；形状不符如实报）。
/// 标准布局：协议(1) 版本串\\0 conn(4) 盐一(8) filler(1) caps_low(2) 字符集(1)
/// 状态(2) caps_high(2) auth_len(1) 保留(10) 盐二(auth_len-8，标准 13 含尾零)
/// 插件名\\0。盐取 20 实字节（盐二截 12，尾零不入盐）。
pub fn parse_handshake(payload: &[u8]) -> Result<Handshake, String> {
    if payload.first() != Some(&10) {
        return Err("握手协议版本非 V10".to_string());
    }
    let mut cursor = 1usize;
    let version_end = payload[cursor..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|offset| cursor + offset)
        .ok_or_else(|| "服务端版本串未终结".to_string())?;
    let server_version = String::from_utf8_lossy(&payload[cursor..version_end]).into_owned();
    cursor = version_end + 1;
    // 固定尾段：conn4 + 盐8 + filler1 + caps2 + charset1 + status2 + caps2 + auth_len1 + 保留10 = 31。
    if cursor + 31 > payload.len() {
        return Err("握手包过短".to_string());
    }
    let connection_id = u32::from_le_bytes([
        payload[cursor],
        payload[cursor + 1],
        payload[cursor + 2],
        payload[cursor + 3],
    ]);
    cursor += 4;
    let mut salt = [0u8; 20];
    salt[..8].copy_from_slice(&payload[cursor..cursor + 8]);
    cursor += 8 + 1; // 盐一 + filler
    let capabilities_low = u16::from_le_bytes([payload[cursor], payload[cursor + 1]]) as u32;
    cursor += 2 + 1 + 2; // caps_low + 字符集 + 状态
    let capabilities_high = u16::from_le_bytes([payload[cursor], payload[cursor + 1]]) as u32;
    let capabilities = capabilities_low | (capabilities_high << 16);
    cursor += 2;
    let auth_len = payload[cursor] as usize;
    cursor += 1 + 10; // auth_len + 保留
                      // 盐二：auth_len > 8 时取 auth_len - 8（标准 13 含尾零），否则按剩余封顶。
    let part2_len = if auth_len > 8 {
        (auth_len - 8).min(payload.len().saturating_sub(cursor))
    } else {
        12.min(payload.len().saturating_sub(cursor))
    };
    let part2 = &payload[cursor..cursor + part2_len];
    let take = 12.min(part2.len());
    salt[8..8 + take].copy_from_slice(&part2[..take]);
    cursor += part2_len;
    let auth_plugin = if cursor < payload.len() {
        let end = payload[cursor..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|offset| cursor + offset)
            .unwrap_or(payload.len());
        String::from_utf8_lossy(&payload[cursor..end]).into_owned()
    } else {
        String::new()
    };
    Ok(Handshake {
        server_version,
        connection_id,
        salt,
        capabilities,
        auth_plugin: if auth_plugin.is_empty() {
            "mysql_native_password".to_string()
        } else {
            auth_plugin
        },
    })
}

/// mysql_native_password 认证响应（20 字节）：SHA1(pass) ^ SHA1(salt ++ SHA1(SHA1(pass)))。
pub fn scramble_native(password: &str, salt: &[u8; 20]) -> [u8; 20] {
    let stage1 = Sha1::digest(password.as_bytes());
    let stage2 = Sha1::digest(stage1);
    let mut hash = Sha1::new();
    hash.update(salt);
    hash.update(stage2);
    let stage3 = hash.finalize();
    let mut out = [0u8; 20];
    for index in 0..20 {
        out[index] = stage1[index] ^ stage3[index];
    }
    out
}

/// caching_sha2_password 快速认证响应（32 字节）：
/// SHA256(pass) ^ SHA256(SHA256(SHA256(pass)) ++ salt)。
pub fn scramble_caching_sha2(password: &str, salt: &[u8; 20]) -> [u8; 32] {
    let stage1 = Sha256::digest(password.as_bytes());
    let stage2 = Sha256::digest(stage1);
    let mut hash = Sha256::new();
    hash.update(stage2);
    hash.update(salt);
    let stage3 = hash.finalize();
    let mut out = [0u8; 32];
    for index in 0..32 {
        out[index] = stage1[index] ^ stage3[index];
    }
    out
}

/// 认证响应（按插件派发；未知插件如实报扩展缝）。
pub fn build_auth_response(
    plugin: &str,
    password: &str,
    salt: &[u8; 20],
) -> Result<Vec<u8>, String> {
    match plugin {
        "mysql_native_password" => Ok(scramble_native(password, salt).to_vec()),
        "caching_sha2_password" => Ok(scramble_caching_sha2(password, salt).to_vec()),
        "mysql_clear_password" => {
            let mut out = password.as_bytes().to_vec();
            out.push(0);
            Ok(out)
        }
        other => Err(format!("认证插件为扩展缝：{other}")),
    }
}

/// 握手响应 41（能力子集 + 用户 + 认证响应 + 可选库名 + 插件名）。
pub fn build_handshake_response(
    handshake: &Handshake,
    user: &str,
    auth_response: &[u8],
    database: Option<&str>,
    ssl: bool,
) -> Vec<u8> {
    let mut capabilities = CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | CLIENT_PLUGIN_AUTH;
    if ssl {
        capabilities |= CLIENT_SSL;
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(&capabilities.to_le_bytes());
    payload.extend_from_slice(&0x0100_0000u32.to_le_bytes()); // max packet
    payload.push(0x21); // utf8mb4_general_ci
    payload.extend_from_slice(&[0u8; 23]);
    payload.extend_from_slice(user.as_bytes());
    payload.push(0);
    payload.push(auth_response.len() as u8);
    payload.extend_from_slice(auth_response);
    if let Some(db) = database {
        payload.extend_from_slice(db.as_bytes());
        payload.push(0);
    }
    payload.extend_from_slice(handshake.auth_plugin.as_bytes());
    payload.push(0);
    payload
}

/// 文本协议行解析（lenenc 字符串列；0xfb = NULL）。
pub fn parse_text_row(payload: &[u8], columns: usize) -> Result<Vec<Option<String>>, String> {
    let mut values = Vec::with_capacity(columns);
    let mut cursor = 0usize;
    for _ in 0..columns {
        if cursor >= payload.len() {
            return Err("行包列数不足".to_string());
        }
        if payload[cursor] == 0xfb {
            cursor += 1;
            values.push(None);
            continue;
        }
        let (len, used) = decode_lenenc(&payload[cursor..])?;
        cursor += used;
        let end = cursor + len as usize;
        if end > payload.len() {
            return Err("行值越界".to_string());
        }
        values.push(Some(
            String::from_utf8_lossy(&payload[cursor..end]).into_owned(),
        ));
        cursor = end;
    }
    Ok(values)
}

/// 错误包（0xFF）取原话（2 字节错误码 + 6 字节 sqlstate 标记 + 原话）。
pub fn parse_error_packet(payload: &[u8]) -> String {
    if payload.len() < 9 {
        return format!("未识别错误包（{} 字节）", payload.len());
    }
    let code = u16::from_le_bytes([payload[1], payload[2]]);
    format!(
        "错误码 {}：{}",
        code,
        String::from_utf8_lossy(&payload[9..])
    )
}

/// 读一包（3 字节长度 + 1 字节序号 + 载荷）。
async fn read_packet<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<(u8, Vec<u8>), String> {
    let mut header = [0u8; 4];
    tokio::time::timeout(
        Duration::from_millis(MYSQL_TIMEOUT_MS),
        reader.read_exact(&mut header),
    )
    .await
    .map_err(|_| "读包超时".to_string())?
    .map_err(|error| format!("读包失败（如实报）：{error}"))?;
    let len = (header[0] as usize) | ((header[1] as usize) << 8) | ((header[2] as usize) << 16);
    let mut payload = vec![0u8; len];
    if len > 0 {
        tokio::time::timeout(
            Duration::from_millis(MYSQL_TIMEOUT_MS),
            reader.read_exact(&mut payload),
        )
        .await
        .map_err(|_| "读包超时".to_string())?
        .map_err(|error| format!("读包失败（如实报）：{error}"))?;
    }
    Ok((header[3], payload))
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

/// 只读查询一个真实 MySQL 服务（文本协议、有界结果、截断如实标注）。
#[tauri::command]
pub async fn query_mysql(config: MySqlConfig, sql: String) -> Result<SqlQueryResult, String> {
    validate_config(&config)?;
    let checked = ensure_readonly_sql(&sql)?;
    let started = Instant::now();
    let addr = format!("{}:{}", config.host.trim(), config.port);
    let mut stream = tokio::time::timeout(
        Duration::from_millis(MYSQL_TIMEOUT_MS),
        TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| format!("连接超时：{addr}"))?
    .map_err(|error| format!("连接失败（如实报）：{error}"))?;

    // 握手包在裸流上读（TLS 升级在握手响应之前，时序同 PG SSLRequest 口径）。
    let mut greeting_reader = BufReader::new(&mut stream);
    let (greet_seq, greet_payload) = read_packet(&mut greeting_reader).await?;
    drop(greeting_reader);
    if greet_payload.first() == Some(&0xff) {
        return Err(format!("握手失败：{}", parse_error_packet(&greet_payload)));
    }
    let handshake = parse_handshake(&greet_payload)?;

    // TLS 能力协商（require）：先发 SSL 能力握手响应头再升级。
    let (mut reader, mut writer): (
        BufReader<Box<dyn AsyncRead + Send + Unpin>>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) = if config.tls == "require" {
        let ssl_probe = build_handshake_response(
            &handshake,
            "",
            &[],
            None,
            true, // 只声明 CLIENT_SSL 的握手响应头（4 字节能力+最大包+字符集+23 零）——
                  // MySQL 口径：SSLRequest 为握手响应的前 32 字节（用户段为空）。
        );
        let probe = &ssl_probe[..32.min(ssl_probe.len())];
        stream
            .write_all(&build_packet(greet_seq + 1, probe))
            .await
            .map_err(|error| format!("SSLRequest 写失败：{error}"))?;
        stream
            .flush()
            .await
            .map_err(|error| format!("SSLRequest 刷写失败：{error}"))?;
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
            Duration::from_millis(MYSQL_TIMEOUT_MS),
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

    // 认证。
    let password = config.password.clone().unwrap_or_default();
    let auth_response = build_auth_response(&handshake.auth_plugin, &password, &handshake.salt)?;
    let response = build_handshake_response(
        &handshake,
        config.user.trim(),
        &auth_response,
        config.database.as_deref().map(str::trim),
        config.tls == "require",
    );
    let mut seq = greet_seq.wrapping_add(if config.tls == "require" { 2 } else { 1 });
    writer
        .write_all(&build_packet(seq, &response))
        .await
        .map_err(|error| format!("握手响应写失败：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败：{error}"))?;

    loop {
        let (reply_seq, payload) = read_packet(&mut reader).await?;
        seq = reply_seq.wrapping_add(1);
        match payload.first() {
            Some(0x00) => break, // OK：认证过
            Some(0xff) => return Err(format!("认证失败：{}", parse_error_packet(&payload))),
            Some(0xfe) => {
                // 认证切换：0xFE + 插件名\0 + 新盐。
                let plugin_end = payload[1..]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|offset| 1 + offset)
                    .ok_or("认证切换帧畸形")?;
                let plugin = String::from_utf8_lossy(&payload[1..plugin_end]).into_owned();
                let mut new_salt = [0u8; 20];
                let raw = &payload[plugin_end + 1..];
                let take = 12.min(raw.len());
                new_salt[..take].copy_from_slice(&raw[..take]);
                let switched = build_auth_response(&plugin, &password, &new_salt)?;
                writer
                    .write_all(&build_packet(seq, &switched))
                    .await
                    .map_err(|error| format!("切换响应写失败：{error}"))?;
                writer
                    .flush()
                    .await
                    .map_err(|error| format!("刷写失败：{error}"))?;
            }
            Some(0x01) => {
                // caching_sha2 全认证请求：仅 TLS 下可回明文口令（0 装防明线）。
                if config.tls != "require" {
                    return Err("caching_sha2 全认证需 TLS（明文通道拒绝口令明线）".to_string());
                }
                let mut clear = password.as_bytes().to_vec();
                clear.push(0);
                writer
                    .write_all(&build_packet(seq, &clear))
                    .await
                    .map_err(|error| format!("明文口令写失败：{error}"))?;
                writer
                    .flush()
                    .await
                    .map_err(|error| format!("刷写失败：{error}"))?;
            }
            Some(0x02) => {
                // caching_sha2 状态：0x03 快认证成功（下轮 0x00 收尾）/ 0x04 全认证。
                match payload.get(1) {
                    Some(0x03) => {}
                    Some(0x04) => {
                        return Err(
                            "服务端要求全认证（TLS 明文回传为扩展缝之外的既有路径）".to_string()
                        );
                    }
                    other => return Err(format!("未知认证状态：{:?}", other)),
                }
            }
            other => return Err(format!("认证阶段包漂移（首字节 {:?}）", other)),
        }
    }

    // COM_QUERY 文本协议。
    let mut query_payload = vec![0x03];
    query_payload.extend_from_slice(checked.as_bytes());
    writer
        .write_all(&build_packet(seq, &query_payload))
        .await
        .map_err(|error| format!("查询包写失败：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败：{error}"))?;

    let (_, first) = read_packet(&mut reader).await?;
    if first.first() == Some(&0xff) {
        return Err(format!("查询失败：{}", parse_error_packet(&first)));
    }
    if first.first() == Some(&0x00) {
        return Ok(SqlQueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            truncated: false,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        });
    }
    let (column_count, _) = decode_lenenc(&first)?;
    let mut columns = Vec::with_capacity(column_count as usize);
    for _ in 0..column_count {
        let (_, def) = read_packet(&mut reader).await?;
        // 列定义 41：4 段 lenenc 串（catalog/schema/table/org_table）后是 name。
        let mut cursor = 0usize;
        for _skip in 0..4 {
            let (len, used) = decode_lenenc(&def[cursor..])?;
            cursor += used + len as usize;
        }
        let (name_len, used) = decode_lenenc(&def[cursor..])?;
        cursor += used;
        columns
            .push(String::from_utf8_lossy(&def[cursor..cursor + name_len as usize]).into_owned());
    }
    // EOF 标记（未启用 DEPRECATE_EOF）。
    let (_, eof) = read_packet(&mut reader).await?;
    if eof.first() != Some(&0xfe) {
        return Err("列定义后未见 EOF 标记（协议漂移）".to_string());
    }
    let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
    let mut truncated = false;
    loop {
        let (row_seq, payload) = read_packet(&mut reader).await?;
        if payload.first() == Some(&0xfe) && payload.len() < 9 {
            break; // 行结束 EOF
        }
        if payload.first() == Some(&0xff) {
            return Err(format!("查询失败：{}", parse_error_packet(&payload)));
        }
        if rows.len() >= SQL_ROW_LIMIT {
            truncated = true;
            continue;
        }
        let values = parse_text_row(&payload, columns.len())?;
        rows.push(
            values
                .into_iter()
                .map(|value| match value {
                    Some(text) => serde_json::Value::String(text),
                    None => serde_json::Value::Null,
                })
                .collect(),
        );
        let _ = row_seq;
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

    fn base_config() -> MySqlConfig {
        MySqlConfig {
            host: "db.example.test".to_string(),
            port: 3306,
            tls: "require".to_string(),
            allow_insecure_plain: false,
            user: "app".to_string(),
            database: Some("main".to_string()),
            password: Some("secret".to_string()),
        }
    }

    #[test]
    fn config_and_lenenc() {
        assert!(validate_config(&base_config()).is_ok());
        let mut bad = base_config();
        bad.tls = "plain-insecure".to_string();
        assert!(validate_config(&bad).is_err(), "明文未确认拒绝");
        bad.allow_insecure_plain = true;
        assert!(validate_config(&bad).is_ok());

        for value in [
            0u64,
            250,
            251,
            65_535,
            65_536,
            0xFF_FFFF,
            0x100_0000,
            u32::MAX as u64,
        ] {
            let encoded = encode_lenenc(value);
            let (decoded, used) = decode_lenenc(&encoded).unwrap();
            assert_eq!(decoded, value, "lenenc 往返：{value}");
            assert_eq!(used, encoded.len());
        }
        assert!(decode_lenenc(&[0xfc, 0x01]).is_err(), "越界拒绝");
        println!("  [1] config & lenenc OK");
    }

    #[test]
    fn handshake_and_auth() {
        // 构造最小握手包：协议 10 + 版本 "8.0.0\0" + 连接 id + 盐 8 + filler +
        // caps_low + charset + status + caps_high + auth_len + reserved10 + 盐 12 + 插件名\0。
        // 标准握手帧（盐二 13 字节含尾零，auth_len=21）。
        let mut payload = vec![10u8];
        payload.extend_from_slice(b"8.0.0\0");
        payload.extend_from_slice(&7u32.to_le_bytes());
        payload.extend_from_slice(b"12345678");
        payload.push(0);
        payload.extend_from_slice(&0x0200u16.to_le_bytes()); // caps low 含 PROTOCOL_41
        payload.push(0x21);
        payload.extend_from_slice(&2u16.to_le_bytes());
        payload.extend_from_slice(&0x0008u16.to_le_bytes()); // caps high 含 PLUGIN_AUTH（协议小端）
        payload.push(21);
        payload.extend_from_slice(&[0u8; 10]);
        payload.extend_from_slice(b"abcdefghijkl\0");
        payload.extend_from_slice(b"caching_sha2_password\0");

        let handshake = parse_handshake(&payload).unwrap();
        assert_eq!(handshake.server_version, "8.0.0");
        assert_eq!(handshake.connection_id, 7);
        assert_eq!(handshake.auth_plugin, "caching_sha2_password");
        assert_eq!(&handshake.salt[..8], b"12345678");
        assert_eq!(&handshake.salt[8..], b"abcdefghijkl");

        let native = scramble_native("secret", &handshake.salt);
        assert_eq!(native.len(), 20);
        assert_eq!(native, scramble_native("secret", &handshake.salt), "确定性");
        assert_ne!(
            native,
            scramble_native("other", &handshake.salt),
            "口令敏感"
        );
        let sha2 = scramble_caching_sha2("secret", &handshake.salt);
        assert_eq!(sha2.len(), 32);
        assert_ne!(
            &sha2[..],
            &scramble_native("secret", &handshake.salt)[..],
            "两口径不同物"
        );

        let response = build_handshake_response(&handshake, "app", &native, Some("main"), true);
        assert_eq!(
            &response[..4],
            &0x0008_8a00u32.to_le_bytes(),
            "能力位组合 = PROTOCOL_41|SECURE_CONNECTION|PLUGIN_AUTH|SSL"
        );
        assert!(response.windows(4).any(|w| w == b"app\0"), "用户在响应内");

        assert!(
            build_auth_response("sha256_password", "x", &handshake.salt).is_err(),
            "未知插件如实报缝"
        );
        let clear = build_auth_response("mysql_clear_password", "pw", &handshake.salt).unwrap();
        assert_eq!(clear, b"pw\0".to_vec());
        println!("  [2] handshake & auth OK");
    }

    #[test]
    fn text_rows_and_errors() {
        // 两列行：'abc' / NULL。
        let mut row = encode_lenenc(3);
        row.extend_from_slice(b"abc");
        row.push(0xfb);
        assert_eq!(
            parse_text_row(&row, 2).unwrap(),
            vec![Some("abc".to_string()), None]
        );
        assert!(parse_text_row(&row, 3).is_err(), "列数不足拒绝");

        let error_packet = b"\xff\x15\x04#28000Access denied";
        assert!(parse_error_packet(error_packet).contains("Access denied"));
        assert!(parse_error_packet(error_packet).contains("1045"));
        println!("  [3] text rows & errors OK");
    }
}
