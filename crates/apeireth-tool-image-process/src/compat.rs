//! VCP image-process compatibility.

// R156 O-5: allow(missing_docs) 同父底
#![allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageProcessCommand {
    ImageProcessor,
    ImageHasher,
    ImageOcrTool,
    Unknown,
}

pub const IMAGEPROC_COMMAND_COUNT: usize = 3;

impl ImageProcessCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "ImageProcessor" => Self::ImageProcessor,
            "ImageHasher" => Self::ImageHasher,
            "ImageOcrTool" => Self::ImageOcrTool,
            _ => Self::Unknown,
        }
    }
}

pub struct ImageProcessCompatRouter;

impl ImageProcessCompatRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { IMAGEPROC_COMMAND_COUNT }
}

impl Default for ImageProcessCompatRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_3_commands() {
        for s in ["ImageProcessor", "ImageHasher", "ImageOcrTool"] {
            assert_ne!(ImageProcessCommand::from_str(s), ImageProcessCommand::Unknown);
        }
        assert_eq!(IMAGEPROC_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(ImageProcessCommand::from_str("xyz"), ImageProcessCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(ImageProcessCompatRouter::command_count(), 3);
    }
}