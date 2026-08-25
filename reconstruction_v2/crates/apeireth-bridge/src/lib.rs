pub const JSON_RPC_VERSION: &str = "2.0";

pub mod acp;
pub mod dispatcher;
pub mod game;
pub mod lark;
pub mod livekit;
pub mod social;
pub mod stock;
pub mod v1acp;
pub mod web;

pub use social::discord::DiscordBridge;
pub use social::telegram::TelegramBridge;
pub use social::onebot::OneBotBridge;
pub use game::vision_loop::{GameLoopConfig, GameVisionLoop, GameActionPolicy};
pub use dispatcher::{BridgeDispatcher, InboundMessage, OutboundMessage};
