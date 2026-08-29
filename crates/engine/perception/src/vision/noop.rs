//! NoopVisionBackend — 0 装显式占位实现 (与 `NoopVoiceBackend` 同模式).
//!
//! 0 装诚实: 当环境无屏幕捕获支持或测试环境未配置 Vision backend 时,
//! 显式返回 `Err(PerceptionBackendError::BackendUnavailable)`.

use async_trait::async_trait;

use apeireth_plugin::perception_backend::{PerceptionBackendError, ScreenshotBytes, VisionBackend};

/// 0 装 PASS: NoopVisionBackend.
/// 不调真截屏, 返 `BackendUnavailable`. 测试用 + alpha 0 装路径.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopVisionBackend;

#[async_trait]
impl VisionBackend for NoopVisionBackend {
    async fn capture(&self) -> Result<ScreenshotBytes, PerceptionBackendError> {
        Err(PerceptionBackendError::BackendUnavailable(
            "NoopVisionBackend: screen capture not configured (0 装 PASS; RC-7 follow-up 真接)"
                .to_string(),
        ))
    }

    fn name(&self) -> &'static str {
        "noop_vision"
    }

    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        Err(PerceptionBackendError::BackendUnavailable(
            "NoopVisionBackend: ping failed (0 装 PASS)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_vision_backend_fails_with_explicit_error() {
        let backend = NoopVisionBackend;
        let err = backend.capture().await.expect_err("noop must fail");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(msg.contains("0 装 PASS"));
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }
        assert_eq!(backend.name(), "noop_vision");
        assert!(backend.ping().await.is_err());
    }

    #[test]
    fn noop_vision_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<NoopVisionBackend>();
        assert_send_sync::<dyn VisionBackend>();
    }
}
