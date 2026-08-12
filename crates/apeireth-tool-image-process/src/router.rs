//! Multimodal router: dispatch image ops to right impl.

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
use thiserror::Error;

use crate::hash::perceptual_hash;
use crate::exif::extract_exif;
use crate::ocr::{ocr_extract, OcrResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessOp {
    Hash,
    Exif,
    Ocr,
    Thumbnail,
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("invalid op: `{0}`")]
    InvalidOp(String),
}

pub struct ImageRouter;

impl ImageRouter {
    pub fn new() -> Self { Self }
    /// Dispatch an op on raw bytes; returns JSON-shaped string for display.
    pub fn dispatch(&self, op: ProcessOp, data: &[u8], lang: Option<&str>) -> Result<String, ProcessError> {
        match op {
            ProcessOp::Hash => {
                let h = perceptual_hash(data);
                Ok(format!("hash=0x{:016x} ({}x{})", h.bits, h.width, h.height))
            }
            ProcessOp::Exif => {
                let e = extract_exif(data);
                Ok(format!("exif fields={}", e.fields.len()))
            }
            ProcessOp::Ocr => {
                let lang_str = lang.unwrap_or("eng");
                let r: OcrResult = ocr_extract(data, lang_str);
                Ok(format!("ocr lang={} text=\"{}\" confidence={}", lang_str, r.text, r.confidence))
            }
            ProcessOp::Thumbnail => {
                // Stub: just returns size info
                Ok(format!("thumbnail stub: {} bytes input", data.len()))
            }
        }
    }
}

impl Default for ImageRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_op() {
        let r = ImageRouter::new();
        let s = r.dispatch(ProcessOp::Hash, b"data", None).unwrap();
        assert!(s.starts_with("hash=0x"));
    }

    #[test]
    fn exif_op() {
        let r = ImageRouter::new();
        let s = r.dispatch(ProcessOp::Exif, b"data", None).unwrap();
        assert!(s.contains("exif"));
    }

    #[test]
    fn ocr_op() {
        let r = ImageRouter::new();
        let s = r.dispatch(ProcessOp::Ocr, b"data", Some("chi_sim")).unwrap();
        assert!(s.contains("ocr"));
        assert!(s.contains("chi_sim"));
    }

    #[test]
    fn thumbnail_op() {
        let r = ImageRouter::new();
        let s = r.dispatch(ProcessOp::Thumbnail, &[0u8; 100], None).unwrap();
        assert!(s.contains("100"));
    }
}