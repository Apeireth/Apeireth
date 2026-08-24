//! OCR (honest stub — would use tesseract-rs in real impl).

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
}

/// OCR extract from image bytes. Honest stub returns empty result.
pub fn ocr_extract(_data: &[u8], language: &str) -> OcrResult {
    OcrResult {
        text: String::new(),
        confidence: 0.0,
        language: language.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocr_stub_returns_empty() {
        let r = ocr_extract(b"image bytes", "eng");
        assert_eq!(r.text, "");
        assert_eq!(r.confidence, 0.0);
        assert_eq!(r.language, "eng");
    }
}
