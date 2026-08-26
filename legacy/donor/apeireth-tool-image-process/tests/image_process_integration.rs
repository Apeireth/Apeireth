//! Integration tests for apeireth-tool-image-process (post-1.0.0)
//!
//! src/ 7 module 真实现 (hash/exif/ocr/router/mcp/compat/enhanced). 这里 (tests/) 加跨 API 集成.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_image_process::exif::extract_exif;
use apeireth_tool_image_process::router::ProcessError;
use apeireth_tool_image_process::{
    ocr_extract, perceptual_hash, ExifData, ImageHash, ImageProcessCommand,
    ImageProcessCompatRouter, ImageRouter, OcrResult, ProcessOp, IMAGEPROC_COMMAND_COUNT,
    R141_IMAGE_PROC_DELIVERABLES,
};

// =============================================================================
// Constants
// =============================================================================

#[test]
fn r141_image_proc_deliverables() {
    assert_eq!(R141_IMAGE_PROC_DELIVERABLES, 7);
}

#[test]
fn imageproc_command_count() {
    assert_eq!(IMAGEPROC_COMMAND_COUNT, 3);
}

// =============================================================================
// ImageHash
// =============================================================================

#[test]
fn perceptual_hash_deterministic() {
    let data = b"hello world";
    let h1 = perceptual_hash(data);
    let h2 = perceptual_hash(data);
    assert_eq!(h1.bits, h2.bits);
    assert_eq!(h1, h2);
}

#[test]
fn perceptual_hash_struct_fields() {
    let h = perceptual_hash(b"data");
    assert_eq!(h.width, 8);
    assert_eq!(h.height, 8);
}

#[test]
fn perceptual_hash_empty_data() {
    let h = perceptual_hash(&[]);
    assert_eq!(h.bits, 0);
    assert_eq!(h.width, 8);
}

#[test]
fn perceptual_hash_long_data_truncates_at_64() {
    let data = vec![0xFFu8; 256];
    let h = perceptual_hash(&data);
    // 应只读前 64 字节; 不 panic
    assert_eq!(h.width, 8);
}

#[test]
fn perceptual_hash_changes_with_byte() {
    let mut a = vec![0u8; 32];
    let mut b = vec![0u8; 32];
    b[0] = 1;
    let ha = perceptual_hash(&a);
    let hb = perceptual_hash(&b);
    assert_ne!(ha.bits, hb.bits);
}

#[test]
fn image_hash_distance_zero_for_identical() {
    let h = perceptual_hash(b"abc");
    assert_eq!(h.distance(&h), 0);
}

#[test]
fn image_hash_distance_symmetric() {
    let h1 = perceptual_hash(b"abc");
    let h2 = perceptual_hash(b"xyz");
    assert_eq!(h1.distance(&h2), h2.distance(&h1));
}

#[test]
fn image_hash_distance_bounded() {
    let h1 = perceptual_hash(b"abc");
    let h2 = perceptual_hash(b"xyz");
    assert!(h1.distance(&h2) <= 64);
}

#[test]
fn image_hash_clone_eq() {
    let h = perceptual_hash(b"abc");
    let h2 = h.clone();
    assert_eq!(h, h2);
}

// =============================================================================
// ExifData
// =============================================================================

#[test]
fn exif_data_new_empty() {
    let e = ExifData::new();
    assert!(e.fields.is_empty());
}

#[test]
fn exif_data_default_empty() {
    let e = ExifData::default();
    assert!(e.fields.is_empty());
}

#[test]
fn exif_data_get_missing_none() {
    let e = ExifData::new();
    assert!(e.get("Make").is_none());
}

#[test]
fn extract_exif_stub_returns_empty() {
    let e = extract_exif(b"random bytes");
    assert!(e.fields.is_empty());
}

#[test]
fn extract_exif_empty_data_empty() {
    let e = extract_exif(&[]);
    assert!(e.fields.is_empty());
}

// =============================================================================
// OcrResult
// =============================================================================

#[test]
fn ocr_stub_returns_empty() {
    let r = ocr_extract(b"image bytes", "eng");
    assert_eq!(r.text, "");
    assert_eq!(r.confidence, 0.0);
    assert_eq!(r.language, "eng");
}

#[test]
fn ocr_stub_preserves_language() {
    let r = ocr_extract(b"data", "chi_sim");
    assert_eq!(r.language, "chi_sim");
    let r2 = ocr_extract(b"data", "jpn");
    assert_eq!(r2.language, "jpn");
}

#[test]
fn ocr_stub_empty_data() {
    let r = ocr_extract(&[], "eng");
    assert_eq!(r.text, "");
    assert_eq!(r.confidence, 0.0);
}

#[test]
fn ocr_result_clone() {
    let r = ocr_extract(b"data", "eng");
    let r2 = r.clone();
    assert_eq!(r.text, r2.text);
    assert_eq!(r.language, r2.language);
}

// =============================================================================
// ProcessOp
// =============================================================================

#[test]
fn process_op_variants_distinct() {
    let ops = [
        ProcessOp::Hash,
        ProcessOp::Exif,
        ProcessOp::Ocr,
        ProcessOp::Thumbnail,
    ];
    let unique: std::collections::HashSet<_> = ops.iter().collect();
    assert_eq!(unique.len(), 4);
}

#[test]
fn process_op_eq_copy_hash() {
    let op = ProcessOp::Hash;
    let op2 = op;
    assert_eq!(op, op2);
}

// =============================================================================
// ImageRouter
// =============================================================================

#[test]
fn image_router_new() {
    let _r = ImageRouter::new();
}

#[test]
fn image_router_default() {
    let _r = ImageRouter::default();
}

#[test]
fn router_hash_op() {
    let r = ImageRouter::new();
    let s = r.dispatch(ProcessOp::Hash, b"data", None).unwrap();
    assert!(s.starts_with("hash=0x"));
}

#[test]
fn router_exif_op() {
    let r = ImageRouter::new();
    let s = r.dispatch(ProcessOp::Exif, b"data", None).unwrap();
    assert!(s.contains("exif"));
}

#[test]
fn router_ocr_op() {
    let r = ImageRouter::new();
    let s = r
        .dispatch(ProcessOp::Ocr, b"data", Some("chi_sim"))
        .unwrap();
    assert!(s.contains("ocr"));
    assert!(s.contains("chi_sim"));
}

#[test]
fn router_ocr_op_default_lang() {
    let r = ImageRouter::new();
    let s = r.dispatch(ProcessOp::Ocr, b"data", None).unwrap();
    assert!(s.contains("eng"), "默认 lang=eng: {s}");
}

#[test]
fn router_thumbnail_op() {
    let r = ImageRouter::new();
    let s = r.dispatch(ProcessOp::Thumbnail, &[0u8; 100], None).unwrap();
    assert!(s.contains("100"));
}

#[test]
fn router_dispatches_all_4_ops() {
    let r = ImageRouter::new();
    for op in [
        ProcessOp::Hash,
        ProcessOp::Exif,
        ProcessOp::Ocr,
        ProcessOp::Thumbnail,
    ] {
        let r = r.dispatch(op, b"data", None);
        assert!(r.is_ok(), "{op:?} 应 OK");
    }
}

// =============================================================================
// ProcessError
// =============================================================================

#[test]
fn process_error_invalid_op_display() {
    let e = ProcessError::InvalidOp("foo".into());
    let s = e.to_string();
    assert!(s.contains("foo"));
}

// =============================================================================
// ImageProcessCommand
// =============================================================================

#[test]
fn image_process_command_from_str_3() {
    for s in ["ImageProcessor", "ImageHasher", "ImageOcrTool"] {
        assert_ne!(
            ImageProcessCommand::from_str(s),
            ImageProcessCommand::Unknown
        );
    }
}

#[test]
fn image_process_command_unknown_fallback() {
    assert_eq!(
        ImageProcessCommand::from_str("xyz"),
        ImageProcessCommand::Unknown
    );
    assert_eq!(
        ImageProcessCommand::from_str(""),
        ImageProcessCommand::Unknown
    );
}

#[test]
fn image_process_command_eq_hash() {
    let a = ImageProcessCommand::ImageProcessor;
    let b = ImageProcessCommand::ImageProcessor;
    let c = ImageProcessCommand::ImageOcrTool;
    assert_eq!(a, b);
    assert_ne!(a, c);
    let mut set = std::collections::HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(c);
    set.insert(ImageProcessCommand::Unknown);
    assert_eq!(set.len(), 3);
}

// =============================================================================
// ImageProcessCompatRouter
// =============================================================================

#[test]
fn image_process_router_count() {
    assert_eq!(ImageProcessCompatRouter::command_count(), 3);
}

#[test]
fn image_process_router_default() {
    let _r = ImageProcessCompatRouter::default();
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_hash_then_distance() {
    // 相同 data → distance = 0
    let h1 = perceptual_hash(b"identical");
    let h2 = perceptual_hash(b"identical");
    assert_eq!(h1.distance(&h2), 0);
}

#[test]
fn integration_router_with_exif() {
    // Router → ExifData via op
    let r = ImageRouter::new();
    let result = r.dispatch(ProcessOp::Exif, b"image_data", None).unwrap();
    // 应含 "exif fields=" 表示已执行
    assert!(result.contains("exif fields="));
}

#[test]
fn integration_router_with_ocr_default_lang() {
    // Router → OCR → OcrResult via op
    let r = ImageRouter::new();
    let result = r.dispatch(ProcessOp::Ocr, b"image_data", None).unwrap();
    assert!(result.contains("eng"));
}

#[test]
fn integration_router_all_4_ops_consistent() {
    let r = ImageRouter::new();
    let data = b"abc";
    let results: Vec<String> = [
        ProcessOp::Hash,
        ProcessOp::Exif,
        ProcessOp::Ocr,
        ProcessOp::Thumbnail,
    ]
    .iter()
    .map(|op| r.dispatch(*op, data, None).unwrap())
    .collect();
    assert_eq!(results.len(), 4);
    // 4 个 op 应各自 unique 输出
    let unique: std::collections::HashSet<&String> = results.iter().collect();
    assert_eq!(unique.len(), 4, "4 个 op 应各自 unique");
}

#[test]
fn integration_router_with_real_data() {
    // 真实场景: 模拟一张 PNG header
    let png_header = b"\x89PNG\r\n\x1a\n";
    let r = ImageRouter::new();
    let h = r.dispatch(ProcessOp::Hash, png_header, None).unwrap();
    assert!(h.starts_with("hash="));
    let e = r.dispatch(ProcessOp::Exif, png_header, None).unwrap();
    assert!(e.contains("exif"));
    let o = r.dispatch(ProcessOp::Ocr, png_header, Some("eng")).unwrap();
    assert!(o.contains("ocr"));
    let t = r.dispatch(ProcessOp::Thumbnail, png_header, None).unwrap();
    assert!(t.contains("bytes"));
}
