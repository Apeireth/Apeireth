//! 信使 · SMTP 发送面（批次推进）。
//!
//! 契约（I-starport-builtins.md「信使」传输缝点亮）：
//!   1. 真传输 —— tokio TCP + TLS（tokio-rustls，证书锚 webpki-roots）：隐式
//!      TLS 与 STARTTLS 双路（STARTTLS：能力核验 → 220 升级 → 重新 EHLO）；
//!      明文通道必须显式 `plain-insecure` 模式 + `allow_insecure_plain` 双确认
//!      （凭证会走明线，如实命名不粉饰）；
//!   2. 协议纪律 —— EHLO → (AUTH PLAIN) → MAIL FROM → RCPT TO → DATA → QUIT
//!      逐级验回执码；DATA 逐行点填充（dot-stuffing）；30s 超时；
//!   3. 凭据即用即抛 —— 密码只随本次调用走内存，不落任何存储（keychain 语义
//!      不变：如需持久凭据走系统凭据管理器，另行批）；
//!   4. 0 装 —— 配置畸形/回执异常码/协议漂移一律如实报（带服务端原话）。
//!
//! 测试：`cargo test`（纯函数 + 本地假服务端端到端）。

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::net::TcpStream;

/// 全程超时（连接/握手/每级回执共用）。
pub const SMTP_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, serde::Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    /// 'implicit'（连接即 TLS）| 'starttls'（明文问候后升级）| 'plain-insecure'（明文，需 allow）。
    pub tls: String,
    #[serde(default)]
    pub allow_insecure_plain: bool,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    /// 信封发件人。
    pub from: String,
}

#[derive(Debug, serde::Serialize)]
pub struct SmtpSendReceipt {
    pub accepted: Vec<String>,
    /// 服务端对 DATA 的最终回执原话（如实带回）。
    pub server_response: String,
    pub duration_ms: u64,
}

/// 配置校验（纯函数，0 装）。
pub fn validate_config(config: &SmtpConfig) -> Result<(), String> {
    if config.host.trim().is_empty() {
        return Err("SMTP 主机不能为空".to_string());
    }
    if config.port == 0 {
        return Err("SMTP 端口非法".to_string());
    }
    if config.from.trim().is_empty() {
        return Err("信封发件人不能为空".to_string());
    }
    match config.tls.as_str() {
        "implicit" | "starttls" => {}
        "plain-insecure" => {
            if !config.allow_insecure_plain {
                return Err(
                    "明文通道需显式 allow_insecure_plain（凭证将走明线，确认再发）".to_string(),
                );
            }
        }
        other => {
            return Err(format!(
                "未知 TLS 模式（仅 implicit / starttls / plain-insecure）：{other}"
            ))
        }
    }
    if config.username.is_some() && config.password.is_none() {
        return Err("给了用户名必须给密码（不半认证）".to_string());
    }
    Ok(())
}

/// 逐行点填充（RFC 5321：行首 `.` 加倍；防正文吞信）。行界原样保真。
pub fn dot_stuff(eml: &str) -> String {
    let mut out = String::with_capacity(eml.len() + 16);
    for (index, line) in eml.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let (content, ending) = match line.strip_suffix('\r') {
            Some(stripped) => (stripped, "\r"),
            None => (line, ""),
        };
        if content.starts_with('.') {
            out.push('.');
        }
        out.push_str(content);
        out.push_str(ending);
    }
    if !out.ends_with('\n') {
        out.push_str("\r\n");
    }
    out
}

/// AUTH PLAIN 载荷（base64 of `\\0user\\0pass`）。
pub fn build_auth_plain(username: &str, password: &str) -> String {
    let mut raw = Vec::with_capacity(username.len() + password.len() + 2);
    raw.push(0u8);
    raw.extend_from_slice(username.as_bytes());
    raw.push(0u8);
    raw.extend_from_slice(password.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(raw)
}

/// 解析 SMTP 回执行族：`NNN-` 续行、`NNN ` 收尾。返回 (码, 是否收尾)。
pub fn parse_reply_line(line: &str) -> Result<(u16, bool), String> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return Err(format!("回执行畸形：{line}"));
    }
    let code: u16 = line[..3]
        .parse()
        .map_err(|_| format!("回执码非法：{line}"))?;
    match bytes[3] {
        b'-' => Ok((code, false)),
        b' ' => Ok((code, true)),
        other => Err(format!("回执分隔符非法：{}", other as char)),
    }
}

/// EHLO 回执族能力识别：按行取「码-词条」的首个空白分隔词精确比对
/// （大小写不敏感；不做子串匹配——XSTARTTLS 不算 STARTTLS）。
pub fn ehlo_has_capability(reply: &str, name: &str) -> bool {
    let want = name.trim().to_ascii_lowercase();
    reply.lines().any(|line| {
        if !matches!(line.as_bytes().get(3), Some(b'-') | Some(b' ')) {
            return false;
        }
        line[4..]
            .split_whitespace()
            .next()
            .map(|token| token.to_ascii_lowercase() == want)
            .unwrap_or(false)
    })
}

/// 明文阶段读一行（逐字节到 `\n` 止，不读过行界——STARTTLS 升级后的流
/// 不得残留缓冲字节）。
async fn read_line_raw(stream: &mut TcpStream) -> Result<String, String> {
    let mut line = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        tokio::time::timeout(
            Duration::from_millis(SMTP_TIMEOUT_MS),
            stream.read_exact(&mut byte),
        )
        .await
        .map_err(|_| "读回执超时".to_string())?
        .map_err(|error| format!("读回执失败（如实报）：{error}"))?;
        if byte[0] == b'\n' {
            break;
        }
        if line.len() > 64 * 1024 {
            return Err("回执行过长（疑似协议漂移）".to_string());
        }
        line.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&line)
        .trim_end_matches('\r')
        .to_string())
}

/// 明文阶段读一族回执（逐字节行级，复用同一回执码解析）。
async fn read_reply_raw(stream: &mut TcpStream) -> Result<(u16, String), String> {
    let mut buffer = String::new();
    let mut lines = 0usize;
    loop {
        let line = read_line_raw(stream).await?;
        lines += 1;
        if lines > 64 {
            return Err("回执族过长（疑似协议漂移）".to_string());
        }
        let (code, last) = parse_reply_line(&line)?;
        buffer.push_str(&line);
        buffer.push('\n');
        if last {
            return Ok((code, buffer));
        }
    }
}

/// 明文阶段发一行命令（CRLF 收尾）。
async fn write_line_raw(stream: &mut TcpStream, line: &str) -> Result<(), String> {
    stream
        .write_all(line.as_bytes())
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    stream
        .write_all(b"\r\n")
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    stream
        .flush()
        .await
        .map_err(|error| format!("刷写失败（如实报）：{error}"))?;
    Ok(())
}

/// 包 TLS（隐式与 STARTTLS 共用同一 TLS 装配；拆流时序纪律：协商已完成、
/// 流仍整装时才包）。
async fn wrap_tls(
    stream: TcpStream,
    host: &str,
) -> Result<
    (
        BufReader<Box<dyn AsyncRead + Send + Unpin>>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ),
    String,
> {
    let connector = tokio_rustls::TlsConnector::from(build_tls_config()?);
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|error| format!("主机名非法：{error}"))?;
    let tls = tokio::time::timeout(
        Duration::from_millis(SMTP_TIMEOUT_MS),
        connector.connect(server_name, stream),
    )
    .await
    .map_err(|_| "TLS 握手超时".to_string())?
    .map_err(|error| format!("TLS 握手失败（如实报）：{error}"))?;
    let (read_half, write_half) = tokio::io::split(tls);
    Ok((BufReader::new(Box::new(read_half)), Box::new(write_half)))
}

/// 读一族回执（直到收尾行），返回 (最终码, 全文)。
async fn read_reply<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<(u16, String), String> {
    let mut buffer = String::new();
    let mut lines = 0usize;
    loop {
        let mut line = String::new();
        let read = tokio::time::timeout(
            Duration::from_millis(SMTP_TIMEOUT_MS),
            reader.read_line(&mut line),
        )
        .await
        .map_err(|_| "读回执超时".to_string())?
        .map_err(|error| format!("读回执失败（如实报）：{error}"))?;
        if read == 0 {
            return Err("连接被服务端关闭（协议漂移）".to_string());
        }
        lines += 1;
        if lines > 64 {
            return Err("回执族过长（疑似协议漂移）".to_string());
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let (code, last) = parse_reply_line(trimmed)?;
        buffer.push_str(trimmed);
        buffer.push('\n');
        if last {
            return Ok((code, buffer));
        }
    }
}

/// 发一族命令并验回执码（期望码不符如实报，带服务端原话）。
async fn command<W: AsyncWrite + Unpin, R: AsyncBufRead + Unpin>(
    writer: &mut W,
    reader: &mut R,
    line: &str,
    expect: u16,
) -> Result<String, String> {
    writer
        .write_all(line.as_bytes())
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    writer
        .write_all(b"\r\n")
        .await
        .map_err(|error| format!("写命令失败（如实报）：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败（如实报）：{error}"))?;
    let (code, reply) = read_reply(reader).await?;
    if code != expect {
        return Err(format!(
            "服务端回执 {code}（期望 {expect}）：{}",
            reply.trim()
        ));
    }
    Ok(reply)
}

fn tls_provider() -> Arc<rustls::crypto::CryptoProvider> {
    if let Some(provider) = rustls::crypto::CryptoProvider::get_default() {
        return provider.clone();
    }
    let _ =
        rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider());
    rustls::crypto::CryptoProvider::get_default()
        .map(Arc::clone)
        .expect("ring 提供者刚安装必在")
}

fn build_tls_config() -> Result<Arc<rustls::ClientConfig>, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(tls_provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("TLS 版本协商失败：{error}"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// 真发送一封 .eml（信封收件人显式给出；头 To 与信封可不同，如实分立）。
#[tauri::command]
pub async fn send_smtp(
    config: SmtpConfig,
    eml: String,
    recipients: Vec<String>,
) -> Result<SmtpSendReceipt, String> {
    validate_config(&config)?;
    if recipients.is_empty() {
        return Err("信封收件人不能为空".to_string());
    }
    if eml.trim().is_empty() {
        return Err("邮件体不能为空".to_string());
    }
    let started = Instant::now();
    let addr = format!("{}:{}", config.host.trim(), config.port);

    let mut stream = tokio::time::timeout(
        Duration::from_millis(SMTP_TIMEOUT_MS),
        TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| format!("连接超时：{addr}"))?
    .map_err(|error| format!("连接失败（如实报）：{error}"))?;

    // 传输分流（时序纪律：STARTTLS 协商必须在包 TLS 之前、流未拆之时）：
    // starttls = 明文问候 → EHLO → 能力核验 → 220 升级 → 包 TLS；
    // implicit = 直接包 TLS；plain-insecure = 双确认后明文直用。
    let (mut reader, mut writer): (
        BufReader<Box<dyn AsyncRead + Send + Unpin>>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) = if config.tls == "starttls" {
        let (greeting_code, _) = read_reply_raw(&mut stream).await?;
        if greeting_code != 220 {
            return Err(format!("问候语异常：{greeting_code}"));
        }
        write_line_raw(&mut stream, "EHLO apeireth.local").await?;
        let (ehlo_code, ehlo_reply) = read_reply_raw(&mut stream).await?;
        if ehlo_code != 250 {
            return Err(format!("EHLO 被拒（{}）：{}", ehlo_code, ehlo_reply.trim()));
        }
        if !ehlo_has_capability(&ehlo_reply, "STARTTLS") {
            return Err("服务端不支持 STARTTLS（能力缺失）——改明文需双确认".to_string());
        }
        write_line_raw(&mut stream, "STARTTLS").await?;
        let (upgrade_code, upgrade_reply) = read_reply_raw(&mut stream).await?;
        if upgrade_code != 220 {
            return Err(format!(
                "STARTTLS 升级被拒（{}）：{}",
                upgrade_code,
                upgrade_reply.trim()
            ));
        }
        wrap_tls(stream, config.host.trim()).await?
    } else if config.tls == "implicit" {
        wrap_tls(stream, config.host.trim()).await?
    } else {
        let (read_half, write_half) = stream.into_split();
        (BufReader::new(Box::new(read_half)), Box::new(write_half))
    };

    // 问候与 EHLO：starttls 的问候已在升级前完成，此处只做升级后的重新 EHLO
    // （RFC 3207：明文阶段能力表作废）；其余模式此处为常规问候 + 首次 EHLO。
    if config.tls != "starttls" {
        let (greeting_code, _) = read_reply(&mut reader).await?;
        if greeting_code != 220 {
            return Err(format!("问候语异常：{greeting_code}"));
        }
    }
    command(&mut writer, &mut reader, "EHLO apeireth.local", 250).await?;

    if let Some(username) = config.username.as_deref() {
        let password = config.password.as_deref().unwrap_or_default();
        command(&mut writer, &mut reader, "AUTH PLAIN", 334).await?;
        command(
            &mut writer,
            &mut reader,
            &build_auth_plain(username, password),
            235,
        )
        .await?;
    }

    command(
        &mut writer,
        &mut reader,
        &format!("MAIL FROM:<{}>", config.from.trim()),
        250,
    )
    .await?;
    let mut accepted = Vec::new();
    for recipient in &recipients {
        let clean = recipient.trim();
        if clean.is_empty() {
            return Err("信封收件人含空项".to_string());
        }
        let reply = command(&mut writer, &mut reader, &format!("RCPT TO:<{clean}>"), 250).await?;
        let _ = reply;
        accepted.push(clean.to_string());
    }
    command(&mut writer, &mut reader, "DATA", 354).await?;
    writer
        .write_all(dot_stuff(&eml).as_bytes())
        .await
        .map_err(|error| format!("写信体失败（如实报）：{error}"))?;
    writer
        .write_all(b"\r\n.\r\n")
        .await
        .map_err(|error| format!("写信体失败（如实报）：{error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("刷写失败（如实报）：{error}"))?;
    let (data_code, data_reply) = read_reply(&mut reader).await?;
    if data_code != 250 {
        return Err(format!("投递被拒：{}", data_reply.trim()));
    }
    let _ = command(&mut writer, &mut reader, "QUIT", 221).await;

    Ok(SmtpSendReceipt {
        accepted,
        server_response: data_reply.trim().to_string(),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> SmtpConfig {
        SmtpConfig {
            host: "smtp.example.test".to_string(),
            port: 465,
            tls: "implicit".to_string(),
            allow_insecure_plain: false,
            username: Some("alice".to_string()),
            password: Some("secret".to_string()),
            from: "me@home.test".to_string(),
        }
    }

    #[test]
    fn config_validation_matrix() {
        assert!(validate_config(&base_config()).is_ok());

        let mut bad = base_config();
        bad.host = "  ".to_string();
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.port = 0;
        assert!(validate_config(&bad).is_err());
        let mut bad = base_config();
        bad.from = "".to_string();
        assert!(validate_config(&bad).is_err());
        // 断言演进（2026-10-11 传输缝清账）：STARTTLS 已点亮，从「未知模式」
        // 晋升合法模式；真正未知的模式仍拒。
        let mut bad = base_config();
        bad.tls = "starttls".to_string();
        assert!(validate_config(&bad).is_ok(), "STARTTLS 已点亮（缝已清）");
        let mut bad = base_config();
        bad.tls = "upgrade".to_string();
        assert!(validate_config(&bad).is_err(), "未知模式拒绝");
        let mut bad = base_config();
        bad.tls = "plain-insecure".to_string();
        assert!(validate_config(&bad).is_err(), "明文未确认必须拒绝");
        bad.allow_insecure_plain = true;
        assert!(validate_config(&bad).is_ok(), "明文双确认后放行");
        let mut bad = base_config();
        bad.password = None;
        assert!(validate_config(&bad).is_err(), "半认证拒绝");
        println!("  [1] config matrix OK");
    }

    #[test]
    fn dot_stuff_and_auth_plain() {
        assert_eq!(dot_stuff("a\n.b\n..\n"), "a\n..b\n...\n");
        assert_eq!(dot_stuff("plain"), "plain\r\n");
        assert_eq!(dot_stuff("keep\r\n.dotted"), "keep\r\n..dotted\r\n");
        let encoded = build_auth_plain("alice", "secret");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        assert_eq!(decoded, b"\0alice\0secret".to_vec());
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(b"hello"),
            "aGVsbG8=",
            "编码器口径核对"
        );
        println!("  [2] dot-stuff + auth plain OK");
    }

    #[test]
    fn reply_parsing() {
        assert_eq!(parse_reply_line("250 OK").unwrap(), (250, true));
        assert_eq!(parse_reply_line("250-PIPE").unwrap(), (250, false));
        assert!(parse_reply_line("25x bad").is_err());
        assert!(parse_reply_line("25!bad").is_err());
        assert!(parse_reply_line("25").is_err());
        println!("  [3] reply parsing OK");
    }

    #[test]
    fn ehlo_capability_parsing() {
        assert!(ehlo_has_capability(
            "250-fake\r\n250-STARTTLS\r\n",
            "STARTTLS"
        ));
        assert!(
            ehlo_has_capability("250-fake\r\n250-starttls\r\n", "STARTTLS"),
            "大小写不敏感"
        );
        assert!(
            !ehlo_has_capability("250-XSTARTTLS\r\n", "STARTTLS"),
            "子串不误报"
        );
        assert!(
            ehlo_has_capability("250-AUTH PLAIN LOGIN\r\n", "AUTH"),
            "词条首词匹配"
        );
        assert!(
            !ehlo_has_capability("250-AUTH PLAIN LOGIN\r\n", "AUTH LOGIN"),
            "按首词精确口径"
        );
        assert!(ehlo_has_capability("250 SIZE 1024\r\n", "SIZE"), "单行能力");
        assert!(
            !ehlo_has_capability("220 ready\r\n", "STARTTLS"),
            "非能力行不认"
        );
        assert!(!ehlo_has_capability("", "STARTTLS"), "空回执不认");
        println!("  [4] ehlo capability parsing OK");
    }

    /// STARTTLS 端到端（按契约验到「发 STARTTLS→读 220→握手发起」：
    /// TLS 握手本体与 implicit 共用同一 wrap_tls 装配，不重复造证书）。
    #[tokio::test]
    async fn end_to_end_starttls_reaches_handshake() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            write_half.write_all(b"220 fake ready\r\n").await.unwrap();
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert!(line.starts_with("EHLO"), "升级前明文 EHLO");
            // 回执族纪律：能力走续行（250-），族必须以空格码收尾行终结。
            write_half
                .write_all(b"250-fake\r\n250-STARTTLS\r\n250 OK\r\n")
                .await
                .unwrap();
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            assert!(line.starts_with("STARTTLS"), "STARTTLS 命令必须发出");
            write_half.write_all(b"220 go ahead\r\n").await.unwrap();
            // 窥探握手发起：TLS 记录头 0x16 0x03（ClientHello）。
            let mut peek = [0u8; 2];
            reader.read_exact(&mut peek).await.unwrap();
            assert_eq!(peek, [0x16, 0x03], "TLS 握手必须已发起");
        });

        let config = SmtpConfig {
            host: "127.0.0.1".to_string(),
            port,
            tls: "starttls".to_string(),
            allow_insecure_plain: false,
            username: None,
            password: None,
            from: "me@home.test".to_string(),
        };
        let error = send_smtp(
            config,
            "From: me@home.test\r\nTo: you@there.test\r\nSubject: hi\r\n\r\nbody\r\n".to_string(),
            vec!["you@there.test".to_string()],
        )
        .await
        .expect_err("假服务端无真证书，握手必然失败");
        assert!(
            error.contains("TLS 握手"),
            "必须走到 TLS 握手一步（实得：{error}）"
        );
        server.await.unwrap();
        println!("  [5] end-to-end starttls handshake OK");
    }

    /// 能力缺失负路径：服务端不广告 STARTTLS 时如实拒绝，且绝不发出升级命令。
    #[tokio::test]
    async fn end_to_end_starttls_capability_missing() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut log = Vec::new();
            write_half.write_all(b"220 fake ready\r\n").await.unwrap();
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                log.push(line.clone());
                if line.starts_with("EHLO") {
                    write_half
                        .write_all(b"250-fake\r\n250 OK\r\n")
                        .await
                        .unwrap();
                } else {
                    write_half.write_all(b"502 no\r\n").await.unwrap();
                }
            }
            log
        });

        let config = SmtpConfig {
            host: "127.0.0.1".to_string(),
            port,
            tls: "starttls".to_string(),
            allow_insecure_plain: false,
            username: None,
            password: None,
            from: "me@home.test".to_string(),
        };
        let error = send_smtp(
            config,
            "From: me@home.test\r\nTo: you@there.test\r\nSubject: hi\r\n\r\nbody\r\n".to_string(),
            vec!["you@there.test".to_string()],
        )
        .await
        .expect_err("无能力必须拒绝");
        assert!(
            error.contains("能力缺失") || error.contains("不支持 STARTTLS"),
            "如实报能力缺失（实得：{error}）"
        );
        let log = server.await.unwrap();
        assert!(
            log.iter().all(|line| line.starts_with("EHLO")),
            "STARTTLS 命令绝不发出（实录：{log:?}）"
        );
        println!("  [6] end-to-end starttls capability-missing OK");
    }

    /// 本地假服务端端到端：真 TCP + 真协议对账（明文双确认路径）。
    #[tokio::test]
    async fn end_to_end_against_fake_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut seen = String::new();
            write_half.write_all(b"220 fake ready\r\n").await.unwrap();
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                seen.push_str(&line);
                let upper = line.to_ascii_uppercase();
                if upper.starts_with("EHLO") {
                    write_half
                        .write_all(b"250-fake\r\n250 OK\r\n")
                        .await
                        .unwrap();
                } else if upper.starts_with("AUTH PLAIN") {
                    write_half.write_all(b"334 go\r\n").await.unwrap();
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    seen.push_str(&line);
                    write_half.write_all(b"235 authed\r\n").await.unwrap();
                } else if upper.starts_with("MAIL FROM") || upper.starts_with("RCPT TO") {
                    write_half.write_all(b"250 ok\r\n").await.unwrap();
                } else if upper.starts_with("DATA") {
                    write_half.write_all(b"354 go\r\n").await.unwrap();
                    // 读正文到独立行 "."
                    loop {
                        line.clear();
                        if reader.read_line(&mut line).await.unwrap() == 0 {
                            break;
                        }
                        seen.push_str(&line);
                        if line.trim_end_matches(['\r', '\n']) == "." {
                            break;
                        }
                    }
                    write_half.write_all(b"250 queued as 42\r\n").await.unwrap();
                } else if upper.starts_with("QUIT") {
                    write_half.write_all(b"221 bye\r\n").await.unwrap();
                    break;
                }
            }
            seen
        });

        let config = SmtpConfig {
            host: "127.0.0.1".to_string(),
            port,
            tls: "plain-insecure".to_string(),
            allow_insecure_plain: true,
            username: Some("alice".to_string()),
            password: Some("secret".to_string()),
            from: "me@home.test".to_string(),
        };
        let eml = "From: me@home.test\r\nTo: you@there.test\r\nSubject: hi\r\n\r\n.line starts with dot\r\n";
        let receipt = send_smtp(config, eml.to_string(), vec!["you@there.test".to_string()])
            .await
            .unwrap();

        assert_eq!(receipt.accepted, vec!["you@there.test"]);
        assert!(receipt.server_response.contains("42"), "服务端原话如实带回");

        let seen = server.await.unwrap();
        assert!(seen.contains("AUTH PLAIN"), "认证走真协议");
        assert!(seen.contains("MAIL FROM:<me@home.test>"));
        assert!(seen.contains("RCPT TO:<you@there.test>"));
        assert!(seen.contains("..line starts with dot"), "点填充必须落线");
        println!("  [4] end-to-end fake server OK");
    }
}
