//! 信使 · IMAP 收信面（批次推进）。
//!
//! 契约（I-starport-builtins.md「信使」收信缝点亮）：
//!   1. 真协议 —— IMAP4rev1：问候 → LOGIN（引号转义）→ SELECT（UIDVALIDITY/
//!      EXISTS 取真值）→ UID FETCH（带字面量 {N} 的帧解析）→ LOGOUT；
//!      AUTHENTICATE PLAIN / IDLE / 移动删除为**扩展缝**（不设假旋钮）；
//!   2. 传输纪律 —— 三值：'implicit'（993 典型）/ 'starttls'（能力协商后
//!      裸流升级，时序与 PG SSLRequest 同法：升级在拆流之前）/
//!      'plain-insecure'（双确认）；口令只走内存；
//!   3. 0 装 —— 控制字符/未转义标识符拒绝；字面量越界/协议漂移带服务端原话
//!      如实报；头字段缺谁置空不猜；
//!   4. 有界 —— 信封列表上限 [`IMAP_FETCH_LIMIT`]，超出如实标注 truncated。
//!
//! 测试：`cargo test`（纯函数 + 本地假服务端端到端）。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::net::TcpStream;

/// 信封列表上限（超出截断如实标注）。
pub const IMAP_FETCH_LIMIT: usize = 50;
/// 全程超时。
pub const IMAP_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, serde::Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    /// 'implicit' | 'starttls' | 'plain-insecure'（明文需 allow_insecure_plain）。
    pub tls: String,
    #[serde(default)]
    pub allow_insecure_plain: bool,
    pub user: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ImapEnvelope {
    pub uid: u32,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: String,
    pub size_bytes: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct ImapFetchResult {
    pub folder: String,
    pub uid_validity: u32,
    pub exists: u32,
    pub envelopes: Vec<ImapEnvelope>,
    pub truncated: bool,
    pub duration_ms: u64,
}

/// 配置校验（纯函数，0 装；与发送面同纪律）。
pub fn validate_config(config: &ImapConfig) -> Result<(), String> {
    if config.host.trim().is_empty() {
        return Err("收信主机不能为空".to_string());
    }
    if config.port == 0 {
        return Err("收信端口非法".to_string());
    }
    if config.user.trim().is_empty() {
        return Err("收信用户不能为空".to_string());
    }
    if config.password.is_empty() {
        return Err("收信口令不能为空（只走内存）".to_string());
    }
    match config.tls.as_str() {
        "implicit" | "starttls" => {}
        "plain-insecure" => {
            if !config.allow_insecure_plain {
                return Err(
                    "明文通道需显式 allow_insecure_plain（口令将走明线，确认再连）".to_string(),
                );
            }
        }
        other => {
            return Err(format!(
                "未知 TLS 模式（仅 implicit / starttls / plain-insecure）：{other}"
            ))
        }
    }
    Ok(())
}

/// IMAP 引号串（\ 与 " 反斜杠转义；控制字符拒绝）。
pub fn quote_string(value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) {
        return Err("引号串含控制字符，拒绝".to_string());
    }
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("\"{escaped}\""))
}

/// LOGIN 命令载荷（两段引号串）。
pub fn build_login(user: &str, password: &str) -> Result<String, String> {
    Ok(format!(
        "LOGIN {} {}",
        quote_string(user)?,
        quote_string(password)?
    ))
}

/// 能力解析：`* CAPABILITY IMAP4rev1 STARTTLS ...` → 大写词表。
pub fn parse_capabilities(untagged: &str) -> Vec<String> {
    untagged
        .lines()
        .filter_map(|line| line.strip_prefix("* CAPABILITY "))
        .flat_map(|rest| {
            rest.split_whitespace()
                .map(|word| word.to_ascii_uppercase())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// SELECT 回执取真值：UIDVALIDITY 与 EXISTS（缺哪项如实缺，不猜）。
pub fn parse_select_status(untagged: &str) -> (Option<u32>, Option<u32>) {
    let mut uid_validity = None;
    let mut exists = None;
    for line in untagged.lines() {
        if let Some(rest) = line.strip_prefix("* OK [UIDVALIDITY ") {
            uid_validity = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.trim_end_matches(']').parse().ok());
        }
        if let Some(rest) = line.strip_prefix("* ") {
            if rest.to_ascii_uppercase().contains(" EXISTS") {
                exists = rest
                    .split_whitespace()
                    .next()
                    .and_then(|value| value.parse().ok());
            }
        }
    }
    (uid_validity, exists)
}

/// FETCH 响应里提取字面量体（`{N}\\r\\n` 之后恰 N 字节）。
/// 返回 (节名, 体) 序列；长度越界/形状不符抛错（0 装）。
pub fn parse_fetch_literals(response: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bytes = response.as_bytes();
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let open = match bytes[cursor..].iter().position(|byte| *byte == b'{') {
            Some(offset) => cursor + offset,
            None => break,
        };
        let close = bytes[open..]
            .iter()
            .position(|byte| *byte == b'}')
            .map(|offset| open + offset)
            .ok_or_else(|| "字面量长度未终结".to_string())?;
        let len: usize = std::str::from_utf8(&bytes[open + 1..close])
            .map_err(|_| "字面量长度非法".to_string())?
            .trim()
            .parse()
            .map_err(|_| "字面量长度非法".to_string())?;
        // 字面量前恒有 \r\n 两字节。
        let body_start = close + 1 + 2;
        let body_end = body_start + len;
        if body_end > bytes.len() {
            return Err(format!("字面量越界：需 {len} 字节"));
        }
        let head_start = bytes[..open]
            .iter()
            .rposition(|byte| *byte == b'[')
            .map(|offset| offset + 1)
            .unwrap_or(0);
        let section = String::from_utf8_lossy(&bytes[head_start..open])
            .trim()
            .to_string();
        out.push((section, bytes[body_start..body_end].to_vec()));
        cursor = body_end;
    }
    Ok(out)
}

/// 信封头解析（缺谁置空不猜；同名首见优先）。
pub fn parse_envelope_headers(header_text: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in header_text.lines() {
        let colon = match line.find(':') {
            Some(index) => index,
            None => continue,
        };
        let key = line[..colon].trim().to_ascii_lowercase();
        if !matches!(key.as_str(), "from" | "to" | "subject" | "date") {
            continue;
        }
        let value = line[colon + 1..].trim().to_string();
        fields.entry(key).or_insert(value);
    }
    fields
}

/// 逐行读（裸流阶段用：TLS 升级前不能拆流，BufReader 占流后难回收）。
async fn read_line_raw(stream: &mut TcpStream) -> Result<String, String> {
    let mut line = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        tokio::time::timeout(
            Duration::from_millis(IMAP_TIMEOUT_MS),
            stream.read_exact(&mut byte),
        )
        .await
        .map_err(|_| "读行超时".to_string())?
        .map_err(|error| format!("读行失败（如实报）：{error}"))?;
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        if line.len() > 64 * 1024 {
            return Err("单行过长（疑似协议漂移）".to_string());
        }
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

/// 裸流阶段：写一行命令 → 读到带标签回执（untagged 归拢返回）。
async fn run_command_raw(
    stream: &mut TcpStream,
    tag: &str,
    command: &str,
) -> Result<String, String> {
    stream
        .write_all(format!("{tag} {command}\r\n").as_bytes())
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    stream
        .flush()
        .await
        .map_err(|error| format!("刷写失败（如实报）：{error}"))?;
    let mut untagged = String::new();
    loop {
        let line = read_line_raw(stream).await?;
        // 整词匹配（头词相等）：`a2` 不许误配 `a20`（标签碰撞防线）。
        let mut halves = line.splitn(2, char::is_whitespace);
        let head = halves.next().unwrap_or("");
        if head == tag {
            let status = halves
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();
            if status == "OK" {
                return Ok(untagged);
            }
            return Err(format!("{status}：{line}"));
        }
        untagged.push_str(&line);
        untagged.push('\n');
    }
}

/// 读到带标签回执（主阶段：BufReader 拆流后），返回 untagged 全文。
/// 字节级保真：FETCH 字面量含任意行界，逐行 read_until 归拢不改一字节。
async fn read_tagged<R: AsyncBufRead + Unpin>(reader: &mut R, tag: &str) -> Result<String, String> {
    let mut raw: Vec<u8> = Vec::new();
    loop {
        let mut line: Vec<u8> = Vec::new();
        let read = tokio::time::timeout(
            Duration::from_millis(IMAP_TIMEOUT_MS),
            reader.read_until(b'\n', &mut line),
        )
        .await
        .map_err(|_| "读回执超时".to_string())?
        .map_err(|error| format!("读回执失败（如实报）：{error}"))?;
        if read == 0 {
            return Err("连接被服务端关闭（协议漂移）".to_string());
        }
        let as_text = String::from_utf8_lossy(&line);
        let trimmed = as_text.trim_end_matches(['\r', '\n']);
        // 整词匹配（头词相等）：`a2` 不许误配 `a20`（标签碰撞防线）。
        let mut halves = trimmed.splitn(2, char::is_whitespace);
        let head = halves.next().unwrap_or("");
        if head == "+" {
            return Err("收到继续请求（本面不发字面量命令，判协议漂移）".to_string());
        }
        if head == tag {
            let status = halves
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();
            if status == "OK" {
                return Ok(String::from_utf8_lossy(&raw).into_owned());
            }
            return Err(format!("{status}：{trimmed}"));
        }
        raw.extend_from_slice(&line);
    }
}

/// 发命令并等带标签回执（主阶段）。
async fn run_command<W: AsyncWrite + Unpin, R: AsyncBufRead + Unpin>(
    writer: &mut W,
    reader: &mut R,
    tag: &str,
    command: &str,
) -> Result<String, String> {
    writer
        .write_all(format!("{tag} {command}\r\n").as_bytes())
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败（如实报）：{error}"))?;
    read_tagged(reader, tag).await
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

async fn wrap_tls(
    stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls_config = rustls::ClientConfig::builder_with_provider(tls_provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("TLS 版本协商失败：{error}"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(tls_config));
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|error| format!("主机名非法：{error}"))?;
    tokio::time::timeout(
        Duration::from_millis(IMAP_TIMEOUT_MS),
        connector.connect(server_name, stream),
    )
    .await
    .map_err(|_| "TLS 握手超时".to_string())?
    .map_err(|error| format!("TLS 握手失败（如实报）：{error}"))
}

/// 收信封列表（真协议；UID FETCH 最近信件头字段）。
#[tauri::command]
pub async fn imap_fetch(
    config: ImapConfig,
    folder: String,
    limit: Option<usize>,
) -> Result<ImapFetchResult, String> {
    validate_config(&config)?;
    let folder = folder.trim();
    if folder.is_empty() || folder.contains(char::is_control) {
        return Err("文件夹名非法".to_string());
    }
    let limit = limit.unwrap_or(IMAP_FETCH_LIMIT).clamp(1, IMAP_FETCH_LIMIT);
    let started = Instant::now();
    let addr = format!("{}:{}", config.host.trim(), config.port);
    let mut stream = tokio::time::timeout(
        Duration::from_millis(IMAP_TIMEOUT_MS),
        TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| format!("连接超时：{addr}"))?
    .map_err(|error| format!("连接失败（如实报）：{error}"))?;

    // —— 裸流阶段：问候（STARTTLS 协商与升级在下面分流分支内、拆流之前）——
    let greeting = read_line_raw(&mut stream).await?;
    if !greeting.starts_with("* OK") && !greeting.starts_with("* PREAUTH") {
        return Err(format!("问候异常：{greeting}"));
    }

    // —— 传输分流（时序纪律：starttls 的能力协商与升级在包 TLS 前完成）——
    let (mut reader, mut writer): (
        BufReader<Box<dyn AsyncRead + Send + Unpin>>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) = if config.tls == "implicit" {
        let tls_stream = wrap_tls(stream, config.host.trim()).await?;
        let (read_half, write_half) = tokio::io::split(tls_stream);
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    } else if config.tls == "starttls" {
        let capabilities = run_command_raw(&mut stream, "a0", "CAPABILITY").await?;
        if !parse_capabilities(&capabilities)
            .iter()
            .any(|cap| cap == "STARTTLS")
        {
            return Err("服务端未宣告 STARTTLS 能力（如实报，不硬升）".to_string());
        }
        run_command_raw(&mut stream, "a1", "STARTTLS").await?;
        let tls_stream = wrap_tls(stream, config.host.trim()).await?;
        let (read_half, write_half) = tokio::io::split(tls_stream);
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    } else {
        let (read_half, write_half) = stream.into_split();
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    };

    let login = build_login(&config.user, &config.password)?;
    run_command(&mut writer, &mut reader, "a2", &login).await?;
    let select_reply = run_command(
        &mut writer,
        &mut reader,
        "a3",
        &format!("SELECT {}", quote_string(folder)?),
    )
    .await?;
    let (uid_validity, exists) = parse_select_status(&select_reply);

    // UID FETCH 最近信件头字段（有界）。
    let fetch_command = format!(
        "UID FETCH *:{} (UID BODY.PEEK[HEADER.FIELDS (FROM TO SUBJECT DATE)])",
        limit
    );
    let fetch_reply = run_command(&mut writer, &mut reader, "a4", &fetch_command).await?;
    let literals = parse_fetch_literals(&fetch_reply)?;
    let mut envelopes = Vec::with_capacity(literals.len());
    for (section, body) in &literals {
        let header_text = String::from_utf8_lossy(body).into_owned();
        let fields = parse_envelope_headers(&header_text);
        // UID 从含本节名的 FETCH 行取（`* n FETCH (UID 123 ... BODY[...]`）。
        let uid = fetch_reply
            .lines()
            .find(|line| line.contains(section.as_str()))
            .and_then(|line| {
                let idx = line.find("UID ")?;
                line[idx + 4..].split_whitespace().next()?.parse().ok()
            })
            .unwrap_or(0);
        envelopes.push(ImapEnvelope {
            uid,
            from: fields.get("from").cloned().unwrap_or_default(),
            to: fields.get("to").cloned().unwrap_or_default(),
            subject: fields.get("subject").cloned().unwrap_or_default(),
            date: fields.get("date").cloned().unwrap_or_default(),
            size_bytes: body.len(),
        });
    }
    envelopes.truncate(limit);
    let _ = run_command(&mut writer, &mut reader, "a5", "LOGOUT").await;

    Ok(ImapFetchResult {
        folder: folder.to_string(),
        uid_validity: uid_validity.unwrap_or(0),
        exists: exists.unwrap_or(0),
        truncated: exists.unwrap_or(0) as usize > envelopes.len(),
        envelopes,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> ImapConfig {
        ImapConfig {
            host: "imap.example.test".to_string(),
            port: 993,
            tls: "implicit".to_string(),
            allow_insecure_plain: false,
            user: "app".to_string(),
            password: "secret".to_string(),
        }
    }

    #[test]
    fn config_and_quote_validation() {
        assert!(validate_config(&base_config()).is_ok());
        let mut bad = base_config();
        bad.host = " ".to_string();
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.password = "".to_string();
        assert!(validate_config(&bad).is_err(), "空口令拒绝");
        let mut bad = base_config();
        bad.tls = "plain-insecure".to_string();
        assert!(validate_config(&bad).is_err(), "明文未确认必须拒绝");
        bad.allow_insecure_plain = true;
        assert!(validate_config(&bad).is_ok());

        assert_eq!(quote_string("a\"b\\c").unwrap(), "\"a\\\"b\\\\c\"");
        assert!(quote_string("x\ny").is_err(), "控制字符拒绝");
        let login = build_login("u", "p").unwrap();
        assert_eq!(login, "LOGIN \"u\" \"p\"");
        println!("  [1] config & quoting OK");
    }

    #[test]
    fn protocol_parsers() {
        assert_eq!(
            parse_capabilities("* CAPABILITY IMAP4rev1 STARTTLS AUTH=PLAIN\r\n"),
            vec!["IMAP4REV1", "STARTTLS", "AUTH=PLAIN"]
        );
        let (uid_validity, exists) =
            parse_select_status("* OK [UIDVALIDITY 4242] UIDs valid\r\n* 3 EXISTS\r\n");
        assert_eq!(uid_validity, Some(4242));
        assert_eq!(exists, Some(3));
        let (uid_validity, exists) = parse_select_status("* 0 EXISTS\r\n");
        assert_eq!(uid_validity, None, "缺项如实缺");
        assert_eq!(exists, Some(0));

        // 字面量长度程序化计算（手数必错，教训）。
        let literal = b"Subject: hello world\r\n\r\n";
        let response = format!(
            "* 1 FETCH (UID 7 BODY[HEADER.FIELDS (SUBJECT)] {{{}}}\r\n",
            literal.len()
        );
        let mut response_bytes = response.into_bytes();
        response_bytes.extend_from_slice(literal);
        response_bytes.extend_from_slice(b")\r\n");
        let response_text = String::from_utf8_lossy(&response_bytes).into_owned();
        let literals = parse_fetch_literals(&response_text).unwrap();
        assert_eq!(literals.len(), 1);
        assert!(literals[0].0.contains("HEADER.FIELDS"));
        assert_eq!(
            literals[0].1,
            b"Subject: hello world\r\n\r\n".to_vec(),
            "字面量恰 N 字节（含尾部空行）"
        );
        assert!(
            parse_fetch_literals("x {99}\r\nshort").is_err(),
            "字面量越界拒绝"
        );

        let fields =
            parse_envelope_headers("Subject: hi\r\nFrom: a@b.c\r\nDate: now\r\nGarbage line\r\n");
        assert_eq!(fields.get("subject").map(String::as_str), Some("hi"));
        assert_eq!(fields.get("from").map(String::as_str), Some("a@b.c"));
        assert_eq!(fields.get("to"), None, "缺谁置空不猜");
        println!("  [2] protocol parsers OK");
    }

    /// 本地假服务端端到端：明文双确认路径（问候→LOGIN→SELECT→UID FETCH→LOGOUT）。
    #[tokio::test]
    async fn end_to_end_against_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            write_half
                .write_all(b"* OK fake imap ready\r\n")
                .await
                .unwrap();
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                if line.starts_with("a2 ") {
                    assert!(line.contains("LOGIN \"app\" \"secret\""), "引号转义登录");
                    write_half.write_all(b"a2 OK logged in\r\n").await.unwrap();
                } else if line.starts_with("a3 ") {
                    assert!(line.contains("SELECT \"INBOX\""), "文件夹引号");
                    write_half
                        .write_all(
                            b"* OK [UIDVALIDITY 4242] ok\r\n* 1 EXISTS\r\na3 OK selected\r\n",
                        )
                        .await
                        .unwrap();
                } else if line.starts_with("a4 ") {
                    assert!(line.contains("UID FETCH"), "UID FETCH 真发");
                    let literal = b"Subject: hello\r\nFrom: a@b.c\r\n\r\n";
                    let mut response = format!(
                        "* 1 FETCH (UID 7 BODY[HEADER.FIELDS (FROM TO SUBJECT DATE)] {{{}}}\r\n",
                        literal.len()
                    )
                    .into_bytes();
                    response.extend_from_slice(literal);
                    response.extend_from_slice(b")\r\na4 OK fetched\r\n");
                    write_half.write_all(&response).await.unwrap();
                } else if line.starts_with("a5 ") {
                    write_half
                        .write_all(b"* BYE\r\na5 OK bye\r\n")
                        .await
                        .unwrap();
                    break;
                }
            }
        });

        let config = ImapConfig {
            host: "127.0.0.1".to_string(),
            port,
            tls: "plain-insecure".to_string(),
            allow_insecure_plain: true,
            user: "app".to_string(),
            password: "secret".to_string(),
        };
        let result = imap_fetch(config, "INBOX".to_string(), Some(10))
            .await
            .unwrap();
        assert_eq!(result.folder, "INBOX");
        assert_eq!(result.uid_validity, 4242);
        assert_eq!(result.exists, 1);
        assert_eq!(result.envelopes.len(), 1);
        assert_eq!(result.envelopes[0].uid, 7);
        assert_eq!(result.envelopes[0].subject, "hello");
        assert_eq!(result.envelopes[0].from, "a@b.c");
        assert!(!result.truncated);
        server.await.unwrap();
        println!("  [3] end-to-end fake server OK");
    }
}
