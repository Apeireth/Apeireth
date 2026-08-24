pub mod social;
pub mod game;
pub mod dispatcher;

pub use social::discord::DiscordBridge;
pub use social::telegram::TelegramBridge;
pub use social::onebot::OneBotBridge;
pub use game::vision_loop::{GameLoopConfig, GameVisionLoop, GameActionPolicy};
pub use dispatcher::{BridgeDispatcher, InboundMessage, OutboundMessage};
