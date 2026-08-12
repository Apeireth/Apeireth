//! VCP image-process compatibility.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpImageProcessCommand {
    ImageProcessor,
    ImageHasher,
    ImageOcrTool,
    Unknown,
}

pub const VCP_IMAGEPROC_COMMAND_COUNT: usize = 3;

impl VcpImageProcessCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "ImageProcessor" => Self::ImageProcessor,
            "ImageHasher" => Self::ImageHasher,
            "ImageOcrTool" => Self::ImageOcrTool,
            _ => Self::Unknown,
        }
    }
}

pub struct VcpImageProcessRouter;

impl VcpImageProcessRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_IMAGEPROC_COMMAND_COUNT }
}

impl Default for VcpImageProcessRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_3_commands() {
        for s in ["ImageProcessor", "ImageHasher", "ImageOcrTool"] {
            assert_ne!(VcpImageProcessCommand::from_str(s), VcpImageProcessCommand::Unknown);
        }
        assert_eq!(VCP_IMAGEPROC_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(VcpImageProcessCommand::from_str("xyz"), VcpImageProcessCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(VcpImageProcessRouter::command_count(), 3);
    }
}