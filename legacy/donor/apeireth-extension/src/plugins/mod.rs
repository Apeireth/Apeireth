//! plugins — 6 类插件实现 (sync / async / static / service / messagePreprocessor / hybrid)

pub mod async_plug;
pub mod hybrid;
pub mod preprocessor;
pub mod service;
pub mod static_plug;
pub mod sync;

pub use async_plug::AsyncPlugin;
pub use hybrid::HybridPlugin;
pub use preprocessor::MessagePreprocessorPlugin;
pub use service::ServicePlugin;
pub use static_plug::StaticPlugin;
pub use sync::SyncPlugin;
