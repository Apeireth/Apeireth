//! VCP image-gen compatibility (1 router for 13 VCP image providers).

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpImageGenCommand {
    OpenAiDallE,
    StabilityAi,
    Midjourney,
    MiniMaxImage,
    GoogleImagen,
    AdobeFirefly,
    LeonardoAi,
    Ideogram,
    PlaygroundAi,
    BingImageCreator,
    Craiyon,
    Nightcafe,
    Unknown,
}

pub const VCP_IMAGEGEN_COMMAND_COUNT: usize = 12;

impl VcpImageGenCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "OpenAiDallE" => Self::OpenAiDallE,
            "StabilityAi" => Self::StabilityAi,
            "Midjourney" => Self::Midjourney,
            "MiniMaxImage" => Self::MiniMaxImage,
            "GoogleImagen" => Self::GoogleImagen,
            "AdobeFirefly" => Self::AdobeFirefly,
            "LeonardoAi" => Self::LeonardoAi,
            "Ideogram" => Self::Ideogram,
            "PlaygroundAi" => Self::PlaygroundAi,
            "BingImageCreator" => Self::BingImageCreator,
            "Craiyon" => Self::Craiyon,
            "Nightcafe" => Self::Nightcafe,
            _ => Self::Unknown,
        }
    }
}

pub struct VcpImageGenRouter;

impl VcpImageGenRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_IMAGEGEN_COMMAND_COUNT }
}

impl Default for VcpImageGenRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_12_commands() {
        let names = ["OpenAiDallE","StabilityAi","Midjourney","MiniMaxImage","GoogleImagen","AdobeFirefly","LeonardoAi","Ideogram","PlaygroundAi","BingImageCreator","Craiyon","Nightcafe"];
        for s in names {
            assert_ne!(VcpImageGenCommand::from_str(s), VcpImageGenCommand::Unknown);
        }
        assert_eq!(VCP_IMAGEGEN_COMMAND_COUNT, 12);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(VcpImageGenCommand::from_str("xyz"), VcpImageGenCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(VcpImageGenRouter::command_count(), 12);
    }
}