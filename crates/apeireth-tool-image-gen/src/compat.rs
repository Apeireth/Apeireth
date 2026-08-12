//! VCP image-gen compatibility (1 router for 13 VCP image providers).
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageGenCommand {
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

pub const IMAGEGEN_COMMAND_COUNT: usize = 12;

impl ImageGenCommand {
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

pub struct ImageGenCompatRouter;

impl ImageGenCompatRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { IMAGEGEN_COMMAND_COUNT }
}

impl Default for ImageGenCompatRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_12_commands() {
        let names = ["OpenAiDallE","StabilityAi","Midjourney","MiniMaxImage","GoogleImagen","AdobeFirefly","LeonardoAi","Ideogram","PlaygroundAi","BingImageCreator","Craiyon","Nightcafe"];
        for s in names {
            assert_ne!(ImageGenCommand::from_str(s), ImageGenCommand::Unknown);
        }
        assert_eq!(IMAGEGEN_COMMAND_COUNT, 12);
    }

    #[test]
    fn unknown_maps() {
        assert_eq!(ImageGenCommand::from_str("xyz"), ImageGenCommand::Unknown);
    }

    #[test]
    fn router_count() {
        assert_eq!(ImageGenCompatRouter::command_count(), 12);
    }
}