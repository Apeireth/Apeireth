//! `re.pack`：字符串提取、Shannon 熵、魔数识别与嵌入区域雕刻。
//!
//! 这是 recon 工具面的第一个原生实现：零外部依赖、纯 safe Rust。
//! 结论只描述**观察到的事实**；无法确认的壳/加密写进 limitations，
//! 由上层组装 [`Evidence`](crate::recon::Evidence) 时补充。

use serde::{Deserialize, Serialize};

/// 一条提取出的字符串。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedString {
    /// 文件内偏移。
    pub offset: u64,
    /// 文本内容。
    pub text: String,
    /// 编码：`ascii` 或 `utf16le`。
    pub encoding: String,
}

/// 一段可识别的嵌入区域。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedRegion {
    /// 识别出的类型（`elf` / `pe` / `zip` / `png` …）。
    pub kind: String,
    /// 文件内偏移。
    pub offset: u64,
    /// 已知长度（未知时为 `None`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u64>,
}

/// 一次 `re.pack` 分析的原始观察。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackReport {
    /// 输入大小（字节）。
    pub size: usize,
    /// 整体 Shannon 熵（bit/byte，0..=8）。
    pub entropy_bits: f64,
    /// 顶层魔数识别结果。
    pub magic: Option<String>,
    /// 提取到的字符串（受 `max_strings` 上限约束）。
    pub strings: Vec<ExtractedString>,
    /// 嵌入区域。
    pub embedded: Vec<EmbeddedRegion>,
    /// 加壳线索（仅在有明确证据时给出）。
    pub packed_hint: Option<String>,
}

/// 提取参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackOptions {
    /// 最短字符串长度。
    pub min_string_len: usize,
    /// 最多保留的字符串条数。
    pub max_strings: usize,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            min_string_len: 4,
            max_strings: 4096,
        }
    }
}

/// 分析一段字节。
pub fn analyze(bytes: &[u8], options: &PackOptions) -> PackReport {
    let magic = detect_magic(bytes).map(str::to_string);
    let entropy_bits = shannon_entropy(bytes);
    let strings = extract_ascii_strings(bytes, options);
    let embedded = carve(bytes);
    let packed_hint = packed_hint(bytes, entropy_bits);
    PackReport {
        size: bytes.len(),
        entropy_bits,
        magic,
        strings,
        embedded,
        packed_hint,
    }
}

/// Shannon 熵（bit/byte）。空输入返回 0。
pub fn shannon_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for &byte in bytes {
        counts[byte as usize] += 1;
    }
    let total = bytes.len() as f64;
    let mut entropy = 0.0;
    for count in counts {
        if count == 0 {
            continue;
        }
        let p = count as f64 / total;
        entropy -= p * p.log2();
    }
    entropy
}

/// 提取可打印 ASCII 连续串。
fn extract_ascii_strings(bytes: &[u8], options: &PackOptions) -> Vec<ExtractedString> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (index, &byte) in bytes.iter().enumerate() {
        let printable = matches!(byte, 0x20..=0x7e);
        match (printable, start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                push_string(&mut out, bytes, from, index, options);
                start = None;
            }
            _ => {}
        }
        if out.len() >= options.max_strings {
            return out;
        }
    }
    if let Some(from) = start {
        push_string(&mut out, bytes, from, bytes.len(), options);
    }
    out
}

fn push_string(
    out: &mut Vec<ExtractedString>,
    bytes: &[u8],
    from: usize,
    to: usize,
    options: &PackOptions,
) {
    if to - from < options.min_string_len {
        return;
    }
    let Ok(text) = std::str::from_utf8(&bytes[from..to]) else {
        return;
    };
    out.push(ExtractedString {
        offset: from as u64,
        text: text.to_string(),
        encoding: "ascii".to_string(),
    });
}

/// 顶层魔数识别。
fn detect_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x7fELF") {
        return Some("elf");
    }
    if bytes.starts_with(b"MZ") {
        return Some("pe");
    }
    if bytes.starts_with(b"\x00asm") {
        return Some("wasm");
    }
    if bytes.starts_with(b"PK\x03\x04") {
        // APK 是 ZIP，带 AndroidManifest.xml 条目名。
        if contains(bytes, b"AndroidManifest.xml") {
            return Some("apk");
        }
        return Some("zip");
    }
    if bytes.starts_with(b"\x1f\x8b") {
        return Some("gzip");
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some("jpeg");
    }
    if bytes.starts_with(b"%PDF-") {
        return Some("pdf");
    }
    if bytes.starts_with(b"7z\xbc\xaf\x27\x1c") {
        return Some("7z");
    }
    if bytes.starts_with(b"Rar!\x1a\x07") {
        return Some("rar");
    }
    None
}

/// 扫描常见嵌入格式的魔数位置。
fn carve(bytes: &[u8]) -> Vec<EmbeddedRegion> {
    const SIGNATURES: &[(&[u8], &str)] = &[
        (b"\x7fELF", "elf"),
        (b"MZ", "pe"),
        (b"PK\x03\x04", "zip"),
        (b"\x89PNG\r\n\x1a\n", "png"),
        (b"%PDF-", "pdf"),
        (b"\x1f\x8b", "gzip"),
        (b"UPX!", "upx"),
    ];
    let mut regions = Vec::new();
    for (signature, kind) in SIGNATURES {
        let mut offset = 0usize;
        while let Some(found) = find_from(bytes, signature, offset) {
            // 偏移 0 已由顶层魔数表达，不重复雕刻。
            if found > 0 {
                regions.push(EmbeddedRegion {
                    kind: (*kind).to_string(),
                    offset: found as u64,
                    length: None,
                });
            }
            offset = found + signature.len();
        }
    }
    regions.sort_by_key(|region| region.offset);
    regions
}

/// 仅在有明确证据时给出加壳线索。
fn packed_hint(bytes: &[u8], entropy_bits: f64) -> Option<String> {
    if contains(bytes, b"UPX!") {
        return Some("upx".to_string());
    }
    if bytes.len() >= 4096 && entropy_bits >= 7.2 {
        return Some(format!("high-entropy ({entropy_bits:.2} bit/byte)"));
    }
    None
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find_from(haystack, needle, 0).is_some()
}

fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_is_zero_for_constant_and_eight_for_uniform() {
        assert_eq!(shannon_entropy(&[0u8; 256]), 0.0);
        let uniform: Vec<u8> = (0..=255u8).cycle().take(256 * 4).collect();
        assert!((shannon_entropy(&uniform) - 8.0).abs() < 1e-9);
        assert_eq!(shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn strings_are_extracted_with_offsets() {
        let mut data = vec![0u8; 4];
        data.extend_from_slice(b"hello world");
        data.push(0);
        data.extend_from_slice(b"ab");
        let report = analyze(&data, &PackOptions::default());
        assert_eq!(report.strings.len(), 1);
        assert_eq!(report.strings[0].offset, 4);
        assert_eq!(report.strings[0].text, "hello world");
    }

    #[test]
    fn magic_and_embedded_regions_are_detected() {
        let mut elf = b"\x7fELF\x02\x01\x01".to_vec();
        elf.extend_from_slice(&[0u8; 64]);
        assert_eq!(
            analyze(&elf, &PackOptions::default()).magic.as_deref(),
            Some("elf")
        );

        let mut blob = vec![0u8; 16];
        blob.extend_from_slice(b"PK\x03\x04");
        blob.extend_from_slice(&[0u8; 32]);
        let report = analyze(&blob, &PackOptions::default());
        assert!(report
            .embedded
            .iter()
            .any(|region| region.kind == "zip" && region.offset == 16));
    }

    #[test]
    fn upx_signature_is_reported_as_packed_hint() {
        let mut data = b"MZ".to_vec();
        data.extend_from_slice(&[0u8; 128]);
        data.extend_from_slice(b"UPX!");
        let report = analyze(&data, &PackOptions::default());
        assert_eq!(report.packed_hint.as_deref(), Some("upx"));
        assert_eq!(report.magic.as_deref(), Some("pe"));
    }
}
