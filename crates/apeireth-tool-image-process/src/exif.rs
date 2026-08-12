//! EXIF data extraction (honest stub).

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub fields: HashMap<String, String>,
}

impl ExifData {
    pub fn new() -> Self { Self::default() }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }
}

/// Extract EXIF from raw bytes. Honest stub — returns empty.
pub fn extract_exif(_data: &[u8]) -> ExifData {
    // Real impl: kamadak-exif crate, JPEG/TIFF marker parsing
    ExifData::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_exif_for_arbitrary_data() {
        let e = extract_exif(b"random bytes");
        assert!(e.fields.is_empty());
    }

    #[test]
    fn exif_get_returns_none_for_missing() {
        let e = ExifData::new();
        assert!(e.get("Make").is_none());
    }
}