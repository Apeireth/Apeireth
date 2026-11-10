//! `doc_ops` 的**测试夹具**（从 `doc_ops.rs` 搬移过来，**纯搬移，不改逻辑**）。
//!
//! 存在的理由：`doc_ops.rs` 曾是 6150 行的单文件，且在 size 棘轮基线上
//! （`arch-baseline.txt`，0 容差）⇒ 任何净增都会让 `arch_guard` 变红。
//! 把**纯测试夹具**外置 ⇒ 本文件净减 ⇒ 新修复能落进去。
//!
//! 用 `#[path = "doc_ops/fixtures.rs"]` 挂载 ⇒ **不需要**改 `lib.rs`（别人的文件）。

use super::*;

/// 手写 PDF: N 页, 每页若干行文字 (Tj + Td 定位)。返回真实 PDF 字节。
pub(super) fn fixture_pdf(pages: &[&[&str]]) -> Vec<u8> {
    fixture_pdf_with_trailer(pages, None)
}

/// 手写 PDF + 可选 trailer 附加项 (加密标记检测用真形状)。
pub(super) fn fixture_pdf_with_trailer(pages: &[&[&str]], trailer_extra: Option<&str>) -> Vec<u8> {
    let n = pages.len();
    let font_num = 3 + 2 * n;
    let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();

    let kids: Vec<String> = (0..n)
        .map(|index| format!("{} 0 R", 3 + 2 * index))
        .collect();
    objects.push((1, format!("<< /Type /Catalog /Pages 2 0 R >>").into_bytes()));
    objects.push((
        2,
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes(),
    ));
    for (index, lines) in pages.iter().enumerate() {
        let page_num = 3 + 2 * index;
        let content_num = page_num + 1;
        objects.push((
            page_num as u32,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                     /Resources << /Font << /F1 {font_num} 0 R >> >> \
                     /Contents {content_num} 0 R >>"
            )
            .into_bytes(),
        ));
        let mut body = String::from("BT\n/F1 12 Tf\n72 720 Td\n");
        for (line_index, line) in lines.iter().enumerate() {
            if line_index > 0 {
                body.push_str("0 -14 Td\n");
            }
            body.push('(');
            body.push_str(line);
            body.push_str(") Tj\n");
        }
        body.push_str("ET\n");
        objects.push((
            content_num as u32,
            format!("<< /Length {} >>\nstream\n{body}endstream", body.len()).into_bytes(),
        ));
    }
    objects.push((
        font_num as u32,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ));
    assemble_pdf(&objects, trailer_extra)
}

/// 手写 PDF, 单页内容流用 FlateDecode (zlib) 压缩 —— 真压缩 fixture。
pub(super) fn fixture_pdf_flate(lines: &[&str]) -> Vec<u8> {
    use std::io::Write;
    let mut body = String::from("BT\n/F1 12 Tf\n72 720 Td\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            body.push_str("0 -14 Td\n");
        }
        body.push('(');
        body.push_str(line);
        body.push_str(") Tj\n");
    }
    body.push_str("ET\n");
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(body.as_bytes()).expect("压缩内容流");
    let compressed = encoder.finish().expect("收尾压缩器");
    let objects: Vec<(u32, Vec<u8>)> = vec![
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                   /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                .to_vec(),
        ),
        (
            4,
            format!(
                "<< /Length {} /Filter /FlateDecode >>\nstream\n",
                compressed.len()
            )
            .into_bytes(),
        ),
        (
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        ),
    ];
    // 压缩流要拼进对象 4: 手工组装 (数据在流关键字之后)。
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = vec![0usize; objects.len() + 1];
    for (num, body_bytes) in &objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body_bytes);
        if *num == 4 {
            out.extend_from_slice(&compressed);
            out.extend_from_slice(b"\nendstream");
        }
        out.extend_from_slice(b"\nendobj\n");
    }
    append_xref(&mut out, &offsets, objects.len(), None);
    out
}

/// 按对象表拼 PDF (算 xref 偏移)。
pub(super) fn assemble_pdf(objects: &[(u32, Vec<u8>)], trailer_extra: Option<&str>) -> Vec<u8> {
    let count = objects.iter().map(|(num, _)| *num).max().unwrap_or(0);
    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize; count as usize + 1];
    for (num, body) in objects {
        offsets[*num as usize] = out.len();
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    append_xref(&mut out, &offsets, count as usize, trailer_extra);
    out
}

/// 追加 xref 表 + trailer + startxref。
pub(super) fn append_xref(
    out: &mut Vec<u8>,
    offsets: &[usize],
    count: usize,
    trailer_extra: Option<&str>,
) {
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", count + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets[1..] {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let extra = trailer_extra.unwrap_or_default();
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {extra}>>\nstartxref\n{xref_at}\n%%EOF\n",
            count + 1
        )
        .as_bytes(),
    );
}

/// 写 fixture 进工作区, 返回 (配置, 文件名)。
///
/// 测试装配显式开写总闸 (生产由组装根注入 `file_write` 同一开关值)。
pub(super) fn with_input(tag: &str, name: &str, bytes: &[u8]) -> (DocOpsConfig, String) {
    let root = workspace(tag);
    std::fs::write(root.join(name), bytes).expect("落 fixture");
    (
        DocOpsConfig::new(root).with_write_gate(true),
        name.to_string(),
    )
}

/// 每测试独立临时工作区。
pub(super) fn workspace(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("apeireth-doc-ops-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    let dir = p.join("ws");
    std::fs::create_dir_all(&dir).expect("创建测试工作区");
    dir
}

/// 参数快捷构造。
pub(super) fn params(action: &str, input: &str) -> DocOpsParams {
    DocOpsParams {
        action: Some(action.to_string()),
        input: Some(input.to_string()),
        ..DocOpsParams::default()
    }
}
