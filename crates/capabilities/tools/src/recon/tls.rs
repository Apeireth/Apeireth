//! `net.tls`：TLS 握手明文字段解析（ClientHello / ServerHello 指纹）。
//!
//! 输入是从连接开头读到的字节：TLS record header（content type 0x16、version、
//! length）→ handshake（type 1/2、3 字节长度）→ 握手体。握手消息可以跨多个
//! handshake record 分片；同一 record 内多余的消息或字节在解析第一条目标消息后
//! 忽略。
//!
//! 已解析扩展：
//! * `server_name`（0x0000）—— 收集全部 `host_name`；
//! * `application_layer_protocol_negotiation`（0x0010）—— ClientHello 列表 /
//!   ServerHello 单选；
//! * `supported_versions`（0x002b）—— ClientHello 列表 / ServerHello 单选。
//!
//! 其余扩展（signature_algorithms、key_share、session_ticket、GREASE 等）只做
//! 边界校验后跳过。本模块不做 TLS 解密、证书校验，也不解析 TLS 1.3
//! EncryptedExtensions；输入假定为记录层明文（首个 flight）。

use thiserror::Error;

/// TLS record 的 content type：Handshake。
const CONTENT_TYPE_HANDSHAKE: u8 = 0x16;
/// TLS record 主版本号（SSL 3.0 / TLS 1.x 的 major 均为 3）。
const TLS_MAJOR_VERSION: u8 = 0x03;
/// Handshake type：ClientHello。
const HANDSHAKE_CLIENT_HELLO: u8 = 1;
/// Handshake type：ServerHello。
const HANDSHAKE_SERVER_HELLO: u8 = 2;

/// Extension type：server_name（SNI）。
const EXT_SERVER_NAME: u16 = 0x0000;
/// Extension type：application_layer_protocol_negotiation（ALPN）。
const EXT_ALPN: u16 = 0x0010;
/// Extension type：supported_versions。
const EXT_SUPPORTED_VERSIONS: u16 = 0x002b;

/// 单个 handshake 消息的固定头长度（type 1 + length 3）。
const HANDSHAKE_HEADER_LEN: usize = 4;
/// TLS record 头长度（content type 1 + version 2 + length 2）。
const RECORD_HEADER_LEN: usize = 5;
/// TLS 1.3 及以前允许的 session id 上限。
const MAX_SESSION_ID_LEN: usize = 32;

/// TLS 握手解析错误。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TlsError {
    /// 输入短于完成当前读取所必需的长度。
    #[error("TLS input is too short")]
    TooShort,
    /// record 层不合法（content type / version / record 长度）。
    #[error("bad TLS record: {0}")]
    BadRecord(&'static str),
    /// 握手或扩展结构不合法。
    #[error("malformed TLS handshake: {0}")]
    Malformed(&'static str),
}

/// 从 ClientHello 中提取的指纹字段。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientHello {
    /// legacy_version 字段（TLS 1.3 的 ClientHello 通常仍为 0x0303）。
    pub version: u16,
    /// cipher_suites 列表，按线上顺序保留（含 GREASE 值）。
    pub cipher_suites: Vec<u16>,
    /// SNI 中的全部 host_name；无 SNI 扩展时为空。
    pub sni: Vec<String>,
    /// ALPN 协议列表；无 ALPN 扩展时为空。
    pub alpn: Vec<String>,
    /// supported_versions 列表；无该扩展时为空。
    pub supported_versions: Vec<u16>,
}

/// 从 ServerHello 中提取的指纹字段。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerHello {
    /// legacy_version 字段。
    pub version: u16,
    /// 服务端选中的 cipher suite。
    pub cipher_suite: u16,
    /// 服务端选中的 ALPN 协议；无 ALPN 扩展时为 `None`。
    pub alpn: Option<String>,
    /// 服务端选中的 supported_versions 版本；无该扩展时为 `None`。
    pub supported_version: Option<u16>,
}

/// 解析一条 ClientHello 记录（可跨多个 handshake record 分片）。
///
/// 输入必须从 TLS record header 开始；仅解析第一条 ClientHello 握手消息。
pub fn parse_client_hello(bytes: &[u8]) -> Result<ClientHello, TlsError> {
    let message = collect_handshake(bytes)?;
    let handshake_type = *message.first().ok_or(TlsError::TooShort)?;
    if handshake_type != HANDSHAKE_CLIENT_HELLO {
        return Err(TlsError::Malformed("expected a ClientHello handshake"));
    }
    let body = message
        .get(HANDSHAKE_HEADER_LEN..)
        .ok_or(TlsError::TooShort)?;
    parse_client_hello_body(body)
}

/// 解析一条 ServerHello 记录（可跨多个 handshake record 分片）。
///
/// 输入必须从 TLS record header 开始；仅解析第一条 ServerHello 握手消息。
pub fn parse_server_hello(bytes: &[u8]) -> Result<ServerHello, TlsError> {
    let message = collect_handshake(bytes)?;
    let handshake_type = *message.first().ok_or(TlsError::TooShort)?;
    if handshake_type != HANDSHAKE_SERVER_HELLO {
        return Err(TlsError::Malformed("expected a ServerHello handshake"));
    }
    let body = message
        .get(HANDSHAKE_HEADER_LEN..)
        .ok_or(TlsError::TooShort)?;
    parse_server_hello_body(body)
}

/// 顺序读取 record 层，拼出第一条完整的 handshake 消息。
///
/// TLS 允许一条握手消息跨多个 record（每个 record 仍可携带其它握手消息的
/// 片段），因此这里先按 record 长度切分并拼接，再按 handshake 长度截断。
fn collect_handshake(bytes: &[u8]) -> Result<Vec<u8>, TlsError> {
    let mut message = Vec::new();
    let mut offset = 0usize;
    loop {
        let header_end = offset
            .checked_add(RECORD_HEADER_LEN)
            .ok_or(TlsError::TooShort)?;
        let header = bytes.get(offset..header_end).ok_or(TlsError::TooShort)?;
        if header[0] != CONTENT_TYPE_HANDSHAKE {
            return Err(TlsError::BadRecord("record content type is not handshake"));
        }
        if header[1] != TLS_MAJOR_VERSION {
            return Err(TlsError::BadRecord("unsupported record version"));
        }
        let record_len = usize::from(u16::from_be_bytes([header[3], header[4]]));
        if record_len == 0 {
            return Err(TlsError::BadRecord("zero-length handshake record"));
        }
        let payload_end = header_end
            .checked_add(record_len)
            .ok_or(TlsError::TooShort)?;
        let payload = bytes
            .get(header_end..payload_end)
            .ok_or(TlsError::TooShort)?;
        message.extend_from_slice(payload);
        offset = payload_end;

        if let Some(hs_header) = message.get(..HANDSHAKE_HEADER_LEN) {
            let hs_len = (usize::from(hs_header[1]) << 16)
                | (usize::from(hs_header[2]) << 8)
                | usize::from(hs_header[3]);
            let message_end = hs_len
                .checked_add(HANDSHAKE_HEADER_LEN)
                .ok_or(TlsError::TooShort)?;
            if message.len() >= message_end {
                message.truncate(message_end);
                return Ok(message);
            }
        }

        if offset >= bytes.len() {
            return Err(TlsError::TooShort);
        }
    }
}

/// 解析 ClientHello 握手体（不含 4 字节 handshake 头）。
fn parse_client_hello_body(body: &[u8]) -> Result<ClientHello, TlsError> {
    let mut reader = Reader::new(body);
    let version = reader.u16()?;
    reader.take(32)?; // random
    skip_session_id(&mut reader)?;
    let cipher_suites = parse_cipher_suites(&mut reader)?;
    skip_compression_methods(&mut reader)?;

    let mut sni = Vec::new();
    let mut alpn = Vec::new();
    let mut supported_versions = Vec::new();
    for (extension_type, data) in parse_extensions(&mut reader)? {
        match extension_type {
            EXT_SERVER_NAME => sni.extend(parse_sni(data)?),
            EXT_ALPN => alpn.extend(parse_alpn_protocols(data)?),
            EXT_SUPPORTED_VERSIONS => {
                supported_versions.extend(parse_supported_versions_client(data)?);
            }
            _ => {}
        }
    }

    Ok(ClientHello {
        version,
        cipher_suites,
        sni,
        alpn,
        supported_versions,
    })
}

/// 解析 ServerHello 握手体（不含 4 字节 handshake 头）。
fn parse_server_hello_body(body: &[u8]) -> Result<ServerHello, TlsError> {
    let mut reader = Reader::new(body);
    let version = reader.u16()?;
    reader.take(32)?; // random
    skip_session_id(&mut reader)?;
    let cipher_suite = reader.u16()?;
    reader.u8()?; // compression method

    let mut alpn = None;
    let mut supported_version = None;
    for (extension_type, data) in parse_extensions(&mut reader)? {
        match extension_type {
            EXT_ALPN => alpn = Some(parse_alpn_single(data)?),
            EXT_SUPPORTED_VERSIONS => {
                supported_version = Some(parse_supported_versions_server(data)?);
            }
            _ => {}
        }
    }

    Ok(ServerHello {
        version,
        cipher_suite,
        alpn,
        supported_version,
    })
}

/// 解析扩展块：u16 总长 + 若干 (type, u16 len, data)。
fn parse_extensions<'a>(reader: &mut Reader<'a>) -> Result<Vec<(u16, &'a [u8])>, TlsError> {
    if reader.is_empty() {
        // TLS 1.0/1.1 的 Hello 可以完全没有扩展块。
        return Ok(Vec::new());
    }
    let extensions_len = usize::from(reader.u16()?);
    let block = reader.take(extensions_len)?;
    if !reader.is_empty() {
        return Err(TlsError::Malformed("bytes after the extension block"));
    }

    let mut block_reader = Reader::new(block);
    let mut extensions = Vec::new();
    while !block_reader.is_empty() {
        let extension_type = block_reader.u16()?;
        let extension_len = usize::from(block_reader.u16()?);
        let data = block_reader.take(extension_len)?;
        extensions.push((extension_type, data));
    }
    Ok(extensions)
}

/// 解析 SNI 扩展：server_name_list + 若干 (name_type, u16 len, name)。
fn parse_sni(data: &[u8]) -> Result<Vec<String>, TlsError> {
    let mut reader = Reader::new(data);
    let list_len = usize::from(reader.u16()?);
    let list = reader.take(list_len)?;
    if !reader.is_empty() {
        return Err(TlsError::Malformed("bytes after the SNI name list"));
    }

    let mut list_reader = Reader::new(list);
    let mut names = Vec::new();
    while !list_reader.is_empty() {
        let name_type = list_reader.u8()?;
        let name_len = usize::from(list_reader.u16()?);
        let name = list_reader.take(name_len)?;
        if name_type == 0 {
            // name_type 0 = host_name；其它类型按 RFC 8734 等未来扩展忽略。
            if name.is_empty() {
                return Err(TlsError::Malformed("zero-length SNI host name"));
            }
            names.push(utf8(name, "SNI host name is not valid UTF-8")?.to_owned());
        }
    }
    if names.is_empty() {
        return Err(TlsError::Malformed("SNI contains no host_name"));
    }
    Ok(names)
}

/// 解析 ALPN 协议列表（ClientHello 的列表，或 ServerHello 列表中的唯一项）。
fn parse_alpn_protocols(data: &[u8]) -> Result<Vec<String>, TlsError> {
    let mut reader = Reader::new(data);
    let list_len = usize::from(reader.u16()?);
    let list = reader.take(list_len)?;
    if !reader.is_empty() {
        return Err(TlsError::Malformed("bytes after the ALPN protocol list"));
    }

    let mut list_reader = Reader::new(list);
    let mut protocols = Vec::new();
    while !list_reader.is_empty() {
        let protocol_len = usize::from(list_reader.u8()?);
        if protocol_len == 0 {
            return Err(TlsError::Malformed("zero-length ALPN protocol"));
        }
        let protocol = list_reader.take(protocol_len)?;
        protocols.push(utf8(protocol, "ALPN protocol is not valid UTF-8")?.to_owned());
    }
    if protocols.is_empty() {
        return Err(TlsError::Malformed("empty ALPN protocol list"));
    }
    Ok(protocols)
}

/// ServerHello 的 ALPN 必须且只能包含一个协议。
fn parse_alpn_single(data: &[u8]) -> Result<String, TlsError> {
    let mut protocols = parse_alpn_protocols(data)?;
    if protocols.len() != 1 {
        return Err(TlsError::Malformed(
            "ServerHello ALPN must select exactly one protocol",
        ));
    }
    protocols
        .pop()
        .ok_or(TlsError::Malformed("empty ALPN protocol list"))
}

/// ClientHello 的 supported_versions：u8 列表长度 + u16 版本列表。
fn parse_supported_versions_client(data: &[u8]) -> Result<Vec<u16>, TlsError> {
    let mut reader = Reader::new(data);
    let versions_len = usize::from(reader.u8()?);
    let raw = reader.take(versions_len)?;
    if !reader.is_empty() {
        return Err(TlsError::Malformed(
            "bytes after the supported_versions list",
        ));
    }
    if raw.is_empty() || raw.len() % 2 != 0 {
        return Err(TlsError::Malformed(
            "supported_versions list must be non-empty and even",
        ));
    }

    let mut versions = Vec::with_capacity(raw.len() / 2);
    let mut raw_reader = Reader::new(raw);
    while !raw_reader.is_empty() {
        versions.push(raw_reader.u16()?);
    }
    Ok(versions)
}

/// ServerHello 的 supported_versions：恰好一个 u16 版本。
fn parse_supported_versions_server(data: &[u8]) -> Result<u16, TlsError> {
    let mut reader = Reader::new(data);
    let version = reader.u16()?;
    if !reader.is_empty() {
        return Err(TlsError::Malformed(
            "ServerHello supported_versions must contain one version",
        ));
    }
    Ok(version)
}

/// 读取 cipher_suites：u16 总长 + u16 列表；长度必须非零且为偶数。
fn parse_cipher_suites(reader: &mut Reader<'_>) -> Result<Vec<u16>, TlsError> {
    let suites_len = usize::from(reader.u16()?);
    if suites_len == 0 || suites_len % 2 != 0 {
        return Err(TlsError::Malformed(
            "cipher suite list must be non-empty and even",
        ));
    }
    let raw = reader.take(suites_len)?;
    let mut suites = Vec::with_capacity(suites_len / 2);
    for pair in raw.chunks_exact(2) {
        suites.push(u16::from_be_bytes([pair[0], pair[1]]));
    }
    Ok(suites)
}

/// 读取并跳过 session id（u8 长度 + 最多 32 字节）。
fn skip_session_id(reader: &mut Reader<'_>) -> Result<(), TlsError> {
    let session_id_len = usize::from(reader.u8()?);
    if session_id_len > MAX_SESSION_ID_LEN {
        return Err(TlsError::Malformed("session id exceeds 32 bytes"));
    }
    reader.take(session_id_len)?;
    Ok(())
}

/// 读取并跳过 compression_methods（u8 长度 + 至少一个方法）。
fn skip_compression_methods(reader: &mut Reader<'_>) -> Result<(), TlsError> {
    let methods_len = usize::from(reader.u8()?);
    if methods_len == 0 {
        return Err(TlsError::Malformed("empty compression method list"));
    }
    reader.take(methods_len)?;
    Ok(())
}

/// 把字节转成 UTF-8；失败时返回带上下文的 [`TlsError::Malformed`]。
fn utf8<'a>(bytes: &'a [u8], context: &'static str) -> Result<&'a str, TlsError> {
    std::str::from_utf8(bytes).map_err(|_| TlsError::Malformed(context))
}

/// 越界安全的只读游标：所有读取都经 [`Reader::take`] 做边界检查。
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    /// 读取 `len` 字节；不足时返回 [`TlsError::TooShort`]，绝不越界。
    fn take(&mut self, len: usize) -> Result<&'a [u8], TlsError> {
        let end = self.offset.checked_add(len).ok_or(TlsError::TooShort)?;
        let bytes = self.bytes.get(self.offset..end).ok_or(TlsError::TooShort)?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, TlsError> {
        let bytes = self.take(1)?;
        Ok(bytes[0])
    }

    fn u16(&mut self) -> Result<u16, TlsError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u24(value: usize) -> [u8; 3] {
        [(value >> 16) as u8, (value >> 8) as u8, value as u8]
    }

    fn handshake(handshake_type: u8, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(HANDSHAKE_HEADER_LEN + body.len());
        out.push(handshake_type);
        out.extend_from_slice(&u24(body.len()));
        out.extend_from_slice(body);
        out
    }

    fn tls_record(content_type: u8, version: u16, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(RECORD_HEADER_LEN + payload.len());
        out.push(content_type);
        out.extend_from_slice(&version.to_be_bytes());
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn handshake_record(handshake_type: u8, body: &[u8]) -> Vec<u8> {
        tls_record(
            CONTENT_TYPE_HANDSHAKE,
            0x0301,
            &handshake(handshake_type, body),
        )
    }

    fn extension(extension_type: u16, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&extension_type.to_be_bytes());
        out.extend_from_slice(&(data.len() as u16).to_be_bytes());
        out.extend_from_slice(data);
        out
    }

    fn extension_block(extensions: &[Vec<u8>]) -> Vec<u8> {
        let body = extensions.concat();
        let mut out = Vec::with_capacity(2 + body.len());
        out.extend_from_slice(&(body.len() as u16).to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn sni_extension() -> Vec<u8> {
        let name = b"example.com";
        let mut list = Vec::new();
        list.push(0); // host_name
        list.extend_from_slice(&(name.len() as u16).to_be_bytes());
        list.extend_from_slice(name);

        let mut data = Vec::new();
        data.extend_from_slice(&(list.len() as u16).to_be_bytes());
        data.extend_from_slice(&list);
        extension(EXT_SERVER_NAME, &data)
    }

    fn alpn_extension(protocols: &[&str]) -> Vec<u8> {
        let mut list = Vec::new();
        for protocol in protocols {
            list.push(protocol.len() as u8);
            list.extend_from_slice(protocol.as_bytes());
        }
        let mut data = Vec::new();
        data.extend_from_slice(&(list.len() as u16).to_be_bytes());
        data.extend_from_slice(&list);
        extension(EXT_ALPN, &data)
    }

    fn supported_versions_client_extension(versions: &[u16]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push((versions.len() * 2) as u8);
        for version in versions {
            data.extend_from_slice(&version.to_be_bytes());
        }
        extension(EXT_SUPPORTED_VERSIONS, &data)
    }

    fn client_hello_body(with_extensions: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0x0303u16.to_be_bytes()); // legacy_version
        body.extend_from_slice(&[0x11; 32]); // random
        body.push(0); // session id 长度
        body.extend_from_slice(&4u16.to_be_bytes()); // cipher suites 长度
        body.extend_from_slice(&0x1301u16.to_be_bytes());
        body.extend_from_slice(&0x1302u16.to_be_bytes());
        body.extend_from_slice(&[1, 0]); // compression methods: null
        if with_extensions {
            body.extend_from_slice(&extension_block(&[
                sni_extension(),
                alpn_extension(&["h2", "http/1.1"]),
                supported_versions_client_extension(&[0x0304, 0x0303]),
            ]));
        }
        body
    }

    fn client_hello_record(with_extensions: bool) -> Vec<u8> {
        handshake_record(HANDSHAKE_CLIENT_HELLO, &client_hello_body(with_extensions))
    }

    fn server_hello_body(with_extensions: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&0x0303u16.to_be_bytes()); // legacy_version
        body.extend_from_slice(&[0x22; 32]); // random
        body.push(32); // session id 长度
        body.extend_from_slice(&[0x33; 32]);
        body.extend_from_slice(&0x1301u16.to_be_bytes()); // cipher suite
        body.push(0); // compression method
        if with_extensions {
            body.extend_from_slice(&extension_block(&[
                alpn_extension(&["h2"]),
                extension(EXT_SUPPORTED_VERSIONS, &0x0304u16.to_be_bytes()),
            ]));
        }
        body
    }

    fn server_hello_record(with_extensions: bool) -> Vec<u8> {
        handshake_record(HANDSHAKE_SERVER_HELLO, &server_hello_body(with_extensions))
    }

    #[test]
    fn parses_client_hello_with_extensions() {
        let hello = parse_client_hello(&client_hello_record(true)).expect("parse ClientHello");
        assert_eq!(hello.version, 0x0303);
        assert_eq!(hello.cipher_suites, vec![0x1301, 0x1302]);
        assert_eq!(hello.sni, vec!["example.com".to_owned()]);
        assert_eq!(hello.alpn, vec!["h2".to_owned(), "http/1.1".to_owned()]);
        assert_eq!(hello.supported_versions, vec![0x0304, 0x0303]);
    }

    #[test]
    fn parses_server_hello_with_extensions() {
        let hello = parse_server_hello(&server_hello_record(true)).expect("parse ServerHello");
        assert_eq!(hello.version, 0x0303);
        assert_eq!(hello.cipher_suite, 0x1301);
        assert_eq!(hello.alpn.as_deref(), Some("h2"));
        assert_eq!(hello.supported_version, Some(0x0304));
    }

    #[test]
    fn parses_hello_without_extensions() {
        let client =
            parse_client_hello(&client_hello_record(false)).expect("parse bare ClientHello");
        assert_eq!(client.version, 0x0303);
        assert!(client.sni.is_empty());
        assert!(client.alpn.is_empty());
        assert!(client.supported_versions.is_empty());

        let server =
            parse_server_hello(&server_hello_record(false)).expect("parse bare ServerHello");
        assert_eq!(server.version, 0x0303);
        assert_eq!(server.cipher_suite, 0x1301);
        assert_eq!(server.alpn, None);
        assert_eq!(server.supported_version, None);
    }

    #[test]
    fn rejects_truncated_input() {
        assert!(matches!(parse_client_hello(&[]), Err(TlsError::TooShort)));

        let record = client_hello_record(true);
        for cut in 0..record.len() {
            assert!(
                parse_client_hello(&record[..cut]).is_err(),
                "a {cut}-byte prefix must not parse as a complete ClientHello"
            );
        }

        let server = server_hello_record(true);
        assert!(matches!(
            parse_server_hello(&server[..server.len() - 1]),
            Err(TlsError::TooShort)
        ));
    }

    #[test]
    fn rejects_illegal_record_type() {
        let mut wrong_content_type = client_hello_record(true);
        wrong_content_type[0] = 0x17; // application_data
        assert!(matches!(
            parse_client_hello(&wrong_content_type),
            Err(TlsError::BadRecord(_))
        ));

        let mut wrong_record_version = client_hello_record(true);
        wrong_record_version[1] = 0x02; // 非 TLS 主版本
        assert!(matches!(
            parse_client_hello(&wrong_record_version),
            Err(TlsError::BadRecord(_))
        ));
    }

    #[test]
    fn reassembles_fragmented_client_hello() {
        let record = client_hello_record(true);
        let payload = &record[RECORD_HEADER_LEN..];
        // 在 handshake 头中间切分，覆盖 record 分片与部分头部两种情况。
        let split = 2;
        let mut fragmented = tls_record(CONTENT_TYPE_HANDSHAKE, 0x0303, &payload[..split]);
        fragmented.extend_from_slice(&tls_record(
            CONTENT_TYPE_HANDSHAKE,
            0x0303,
            &payload[split..],
        ));

        let hello = parse_client_hello(&fragmented).expect("parse fragmented ClientHello");
        assert_eq!(hello.sni, vec!["example.com".to_owned()]);
        assert_eq!(hello.supported_versions, vec![0x0304, 0x0303]);
    }

    #[test]
    fn rejects_malformed_extension_block() {
        // 扩展块声明的长度大于实际剩余字节。
        let mut body = client_hello_body(false);
        body.extend_from_slice(&[0xff, 0xff]);
        let record = handshake_record(HANDSHAKE_CLIENT_HELLO, &body);
        assert!(matches!(
            parse_client_hello(&record),
            Err(TlsError::TooShort)
        ));

        // 扩展块之后还有多余的握手体字节。
        let mut body = client_hello_body(false);
        body.extend_from_slice(&[0x00, 0x00, 0xde, 0xad]);
        let record = handshake_record(HANDSHAKE_CLIENT_HELLO, &body);
        assert!(matches!(
            parse_client_hello(&record),
            Err(TlsError::Malformed(_))
        ));
    }
}
