//! Capture metadata helpers recovered around the existing `ScreenshotBytes` contract.
//!
//! `XcapVisionBackend` already captures PNG/JPEG bytes. Donor `VisionInput` also
//! carried `width` / `height` / optional OCR. Those fields never made it onto
//! `ScreenshotBytes` (frozen in `apeireth-plugin`). This module extracts them
//! from encoded image bytes without adding a new capture backend or a second
//! screenshot type.
//!
//! PNG IHDR and JPEG SOF parsers are pure-std so they compile on every target,
//! including those that do not enable the `image` crate.

use apeireth_plugin::perception_backend::ScreenshotBytes;
use serde::{Deserialize, Serialize};

use crate::normalize::{vision_priority, SignalSource};

/// PNG signature.
const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Structured capture metadata attached to a screenshot observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureMetadata {
    /// Encoded image format (`png` / `jpeg`).
    pub format: String,
    /// Pixel width when parseable from the encoded header.
    pub width: Option<u32>,
    /// Pixel height when parseable from the encoded header.
    pub height: Option<u32>,
    /// Encoded byte length.
    pub byte_len: usize,
    /// Capture timestamp copied from [`ScreenshotBytes::captured_at_ms`].
    pub captured_at_ms: i64,
    /// Provenance of the capture (defaults to internal for OS screenshots).
    pub source: SignalSource,
}

/// Parse PNG IHDR width/height. Returns `None` for truncated or non-PNG bytes.
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    // signature (8) + length (4) + type (4) + width (4) + height (4)
    const IHDR_MIN: usize = 24;
    if bytes.len() < IHDR_MIN {
        return None;
    }
    if !bytes.starts_with(PNG_MAGIC) {
        return None;
    }
    // IHDR chunk type sits at bytes 12..16.
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

/// Parse the first JPEG SOF marker for width/height. Returns `None` for
/// truncated, progressive-without-SOF, or non-JPEG bytes.
pub fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }
    let mut index = 2usize;
    while index + 3 < bytes.len() {
        if bytes[index] != 0xff {
            return None;
        }
        while index < bytes.len() && bytes[index] == 0xff {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let marker = bytes[index];
        index += 1;
        // Standalone markers (no length): RST0-7 (D0-D7), SOI (D8), EOI (D9), TEM (01).
        if matches!(marker, 0xd0..=0xd9 | 0x01) {
            continue;
        }
        if index + 1 >= bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes([bytes[index], bytes[index + 1]]) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }
        // SOF0..SOF3, SOF5..SOF7, SOF9..SOF11, SOF13..SOF15 (not DHT/DAC/JPG).
        let is_sof = matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        );
        if is_sof {
            // SOF layout after length: precision (1) + height (2) + width (2)
            if length < 7 || index + 6 >= bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes([bytes[index + 3], bytes[index + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[index + 5], bytes[index + 6]]) as u32;
            if width == 0 || height == 0 {
                return None;
            }
            return Some((width, height));
        }
        index += length;
    }
    None
}

/// Read width/height from PNG or JPEG encoded bytes.
pub fn encoded_image_dimensions(bytes: &[u8], format: &str) -> Option<(u32, u32)> {
    match format.trim().to_ascii_lowercase().as_str() {
        "png" => png_dimensions(bytes),
        "jpeg" | "jpg" => jpeg_dimensions(bytes).or_else(|| {
            // Format label may be wrong; try PNG as a fallback.
            png_dimensions(bytes)
        }),
        _ => png_dimensions(bytes).or_else(|| jpeg_dimensions(bytes)),
    }
}

/// Build capture metadata from an existing screenshot.
pub fn capture_metadata(screenshot: &ScreenshotBytes) -> CaptureMetadata {
    let dims = encoded_image_dimensions(&screenshot.bytes, &screenshot.format);
    CaptureMetadata {
        format: screenshot.format.clone(),
        width: dims.map(|(width, _)| width),
        height: dims.map(|(_, height)| height),
        byte_len: screenshot.bytes.len(),
        captured_at_ms: screenshot.captured_at_ms,
        source: SignalSource::Internal,
    }
}

/// Vision attention score derived from screenshot metadata when dimensions
/// are known; `None` when the header could not be parsed (honest, not 1.0).
pub fn capture_attention_score(metadata: &CaptureMetadata) -> Option<f64> {
    match (metadata.width, metadata.height) {
        (Some(width), Some(height)) => Some(vision_priority(width, height)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::from(PNG_MAGIC);
        bytes.extend_from_slice(&13u32.to_be_bytes()); // IHDR length
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth / color / compression / filter / interlace
        bytes.extend_from_slice(&[0, 0, 0, 0]); // dummy CRC
        bytes
    }

    fn minimal_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8]; // SOI
                                          // APP0 (JFIF) skipped; go straight to SOF0.
        bytes.extend_from_slice(&[0xff, 0xc0]);
        // SOF length includes the 2 length bytes: precision(1)+height(2)+width(2) = 7.
        bytes.extend_from_slice(&7u16.to_be_bytes());
        bytes.push(8); // precision
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes
    }

    #[test]
    fn png_dimensions_reads_ihdr() {
        assert_eq!(png_dimensions(&minimal_png(1920, 1080)), Some((1920, 1080)));
        assert_eq!(png_dimensions(&minimal_png(640, 480)), Some((640, 480)));
        assert!(png_dimensions(&[0, 1, 2, 3]).is_none());
        assert!(png_dimensions(&minimal_png(0, 10)).is_none());
    }

    #[test]
    fn jpeg_dimensions_reads_sof0() {
        assert_eq!(jpeg_dimensions(&minimal_jpeg(800, 600)), Some((800, 600)));
        assert_eq!(jpeg_dimensions(&minimal_jpeg(1, 1)), Some((1, 1)));
        assert!(jpeg_dimensions(&[0xff, 0xd8, 0x00]).is_none());
        assert!(jpeg_dimensions(&minimal_jpeg(0, 10)).is_none());
    }

    #[test]
    fn jpeg_skips_app_markers_before_sof() {
        let mut bytes = vec![0xff, 0xd8];
        // APP0 with 4 extra payload bytes.
        bytes.extend_from_slice(&[0xff, 0xe0, 0x00, 0x06, b'J', b'F', b'I', b'F']);
        bytes.extend_from_slice(&[0xff, 0xc0, 0x00, 0x07, 8, 0x00, 0x20, 0x00, 0x10]);
        assert_eq!(jpeg_dimensions(&bytes), Some((16, 32)));
    }

    #[test]
    fn capture_metadata_from_screenshot_png() {
        let screenshot = ScreenshotBytes {
            bytes: minimal_png(1280, 720),
            format: "png".into(),
            captured_at_ms: 42,
        };
        let metadata = capture_metadata(&screenshot);
        assert_eq!(metadata.format, "png");
        assert_eq!(metadata.width, Some(1280));
        assert_eq!(metadata.height, Some(720));
        assert_eq!(metadata.captured_at_ms, 42);
        assert_eq!(metadata.source, SignalSource::Internal);
        let score = capture_attention_score(&metadata).expect("dims known");
        assert!((score - vision_priority(1280, 720)).abs() < 1e-9);
    }

    #[test]
    fn capture_metadata_unknown_bytes_has_no_dims() {
        let screenshot = ScreenshotBytes {
            bytes: vec![1, 2, 3, 4],
            format: "png".into(),
            captured_at_ms: 1,
        };
        let metadata = capture_metadata(&screenshot);
        assert!(metadata.width.is_none());
        assert!(metadata.height.is_none());
        assert!(capture_attention_score(&metadata).is_none());
    }

    #[test]
    fn encoded_image_dimensions_falls_back_across_labels() {
        let png = minimal_png(10, 20);
        assert_eq!(encoded_image_dimensions(&png, "jpeg"), Some((10, 20)));
        let jpeg = minimal_jpeg(11, 22);
        assert_eq!(encoded_image_dimensions(&jpeg, "unknown"), Some((11, 22)));
    }
}
