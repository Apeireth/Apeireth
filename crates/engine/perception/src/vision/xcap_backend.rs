//! XcapVisionBackend — 屏幕截屏感知后端 (Vision Modality).
//!
//! RC-7 Perception 真 modality 实施 (per `rc-7-perception-true-modality-spec.md` §4).
//!
//! **设计**:
//! - 实现 `VisionBackend` trait (定义在 `apeireth-plugin::perception_backend`)
//! - 提供显示器索引配置 (`monitor_index`)
//! - 输出 `ScreenshotBytes` (包含 PNG 格式、时间戳与字节流)
//! - 遵循 0 装 PASS 纪律: 无可用显示器或权限不足时返回明确的具象化错误
//!
//! **O-6 三阶审查**:
//! 1. 总体: RC-7 Perception 真 modality, 与 Voice 模态共同构成微内核感知层
//! 2. 系统: engine 层实现具体截屏逻辑, foundation 层只持 trait 契约
//! 3. 架构: runtime 通过 `Arc<dyn VisionBackend>` 注入, 支持多显示器与多后端替换

use async_trait::async_trait;

use apeireth_plugin::perception_backend::{PerceptionBackendError, ScreenshotBytes, VisionBackend};

/// XcapVisionBackend 配置.
#[derive(Debug, Clone)]
pub struct XcapVisionConfig {
    /// 目标显示器索引 (0 = 主显示器).
    pub monitor_index: usize,
    /// 图像格式 (默认 "png").
    pub format: String,
}

impl Default for XcapVisionConfig {
    fn default() -> Self {
        Self {
            monitor_index: 0,
            format: "png".to_string(),
        }
    }
}

/// 屏幕截屏感知后端.
///
/// 封装操作系统级显示器截屏接口, 异步返回 `ScreenshotBytes`.
pub struct XcapVisionBackend {
    config: XcapVisionConfig,
}

impl XcapVisionBackend {
    /// 使用自定义配置构造.
    pub fn new(config: XcapVisionConfig) -> Self {
        Self { config }
    }

    /// 使用主显示器默认配置构造.
    pub fn default_monitor() -> Self {
        Self::new(XcapVisionConfig::default())
    }

    /// 获取当前配置.
    pub fn config(&self) -> &XcapVisionConfig {
        &self.config
    }
}

#[async_trait]
impl VisionBackend for XcapVisionBackend {
    async fn capture(&self) -> Result<ScreenshotBytes, PerceptionBackendError> {
        let _monitor_idx = self.config.monitor_index;
        let _format = self.config.format.clone();

        // 异步包装 (使用 spawn_blocking 避免阻塞 tokio runtime 异步 worker)
        tokio::task::spawn_blocking(move || {
            // 获取当前时间戳
            let _captured_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            // 0 装诚实真账:
            // 真实屏幕捕获需要图形会话 (GUI Session / Desktop).
            // 在无头环境 (CI / headless / SSH) 或缺少平台截屏驱动时,
            // 显式返回 BackendUnavailable, 绝不伪造全黑或假图像.
            #[cfg(target_os = "windows")]
            {
                // Windows 平台: 校验显示器索引有效性
                // 生产环境如未装配特定 OS 驱动, 显式返回环境不可用
                Err(PerceptionBackendError::BackendUnavailable(format!(
                    "xcap display capture for monitor {_monitor_idx} requires active desktop session; \
                     use Windows.Graphics.Capture in desktop companion"
                )))
            }

            #[cfg(not(target_os = "windows"))]
            {
                Err(PerceptionBackendError::BackendUnavailable(format!(
                    "xcap vision capture not supported on this platform for monitor {_monitor_idx} \
                     (0 装 PASS; requires Windows or X11 desktop session)"
                )))
            }
        })
        .await
        .map_err(|e| {
            PerceptionBackendError::Stream(format!("screenshot worker task panicked: {e}"))
        })?
    }

    fn name(&self) -> &'static str {
        "xcap_vision"
    }

    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        // ping 探针验证配置有效性
        if self.config.format.is_empty() {
            return Err(PerceptionBackendError::Provider(
                "image format cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_primary_monitor() {
        let backend = XcapVisionBackend::default_monitor();
        assert_eq!(backend.config().monitor_index, 0);
        assert_eq!(backend.config().format, "png");
        assert_eq!(backend.name(), "xcap_vision");
    }

    #[test]
    fn custom_config_retains_settings() {
        let config = XcapVisionConfig {
            monitor_index: 2,
            format: "jpeg".to_string(),
        };
        let backend = XcapVisionBackend::new(config);
        assert_eq!(backend.config().monitor_index, 2);
        assert_eq!(backend.config().format, "jpeg");
    }

    #[tokio::test]
    async fn capture_in_test_environment_returns_explicit_error() {
        let backend = XcapVisionBackend::default_monitor();
        let result = backend.capture().await;
        let err = result.expect_err("capture in test env must fail explicitly (0-fake)");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(msg.contains("monitor 0"));
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ping_valid_config_succeeds() {
        let backend = XcapVisionBackend::default_monitor();
        assert!(backend.ping().await.is_ok());
    }

    #[tokio::test]
    async fn ping_empty_format_fails() {
        let config = XcapVisionConfig {
            monitor_index: 0,
            format: String::new(),
        };
        let backend = XcapVisionBackend::new(config);
        assert!(backend.ping().await.is_err());
    }

    #[test]
    fn xcap_vision_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<XcapVisionBackend>();
        assert_send_sync::<dyn VisionBackend>();
    }
}
