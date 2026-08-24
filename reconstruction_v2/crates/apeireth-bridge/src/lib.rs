pub mod social;
pub mod game;
pub mod dispatcher;
pub mod web;
pub mod stock;
pub mod livekit;
pub mod lark;
pub mod acp;

pub use social::discord::DiscordBridge;
pub use social::telegram::TelegramBridge;
pub use social::onebot::OneBotBridge;
pub use game::vision_loop::{GameLoopConfig, GameVisionLoop, GameActionPolicy};
pub use dispatcher::{BridgeDispatcher, InboundMessage, OutboundMessage};
