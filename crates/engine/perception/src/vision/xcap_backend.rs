//! XcapVisionBackend — real, backend-only monitor capture for the Vision modality.
//!
//! `xcap` performs the operating-system capture; this module owns only the
//! bounded adapter to the canonical [`VisionBackend`] trait. It deliberately
//! does not register or enable the backend in Runtime/Module configuration.
//!
//! ## Selection and failure semantics
//!
//! - Monitor indexes are deterministic for each capture: primary displays come
//!   first, then `(x, y, name, id)` ascending.
//! - A monitor is enumerated again immediately before capture. If its identity
//!   no longer exists (for example after a display disconnect), capture fails
//!   closed with `BackendUnavailable`; it never panics or captures another
//!   display by index.
//! - A crash/headless/permission failure from `xcap` is mapped to the typed
//!   canonical `PerceptionBackendError::BackendUnavailable` channel. No blank
//!   or synthetic image is returned.
//! - Captures are bounded before encoding by maximum width, height, and raw
//!   RGBA byte count.
//!
//! The backend has mock-only test seams below. They validate selection, limits,
//! disappearance, and format handling but are not hardware validation. The
//! ignored `real_xcap_hardware_capture_smoke` test is the explicit interactive
//! desktop check.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use std::io::Cursor;

use async_trait::async_trait;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use image::{DynamicImage, ImageFormat, RgbaImage};

use apeireth_plugin::perception_backend::{PerceptionBackendError, ScreenshotBytes, VisionBackend};

/// Hard capture bounds. They limit raw RGBA input before image encoding can
/// amplify allocations. Common 8K UHD displays remain within the byte bound.
const MAX_CAPTURE_WIDTH: u32 = 8_192;
const MAX_CAPTURE_HEIGHT: u32 = 8_192;
const MAX_CAPTURE_RGBA_BYTES: usize = 128 * 1024 * 1024;

/// XcapVisionBackend configuration.
#[derive(Debug, Clone)]
pub struct XcapVisionConfig {
    /// Target monitor index in the deterministic order documented above.
    /// Index zero is the primary monitor when one is reported by the OS.
    pub monitor_index: usize,
    /// Encoded screenshot format: `png`, `jpeg`, or `jpg` (normalized to
    /// `jpeg` in [`ScreenshotBytes::format`]).
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenshotFormat {
    Png,
    Jpeg,
}

impl ScreenshotFormat {
    fn parse(value: &str) -> Result<Self, PerceptionBackendError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpeg" | "jpg" => Ok(Self::Jpeg),
            "" => Err(PerceptionBackendError::Provider(
                "screenshot format cannot be empty; expected png or jpeg".to_string(),
            )),
            other => Err(PerceptionBackendError::Provider(format!(
                "unsupported screenshot format {other:?}; expected png or jpeg"
            ))),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

/// Stable monitor identity retained between enumeration and capture. It is
/// intentionally value-only so the backend itself has no platform handle and
/// therefore requires no manual `Send`/`Sync` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MonitorDescriptor {
    id: u32,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    is_primary: bool,
}

/// Raw frame contract at the backend boundary. `rgba` is explicitly RGBA8,
/// never an assumed BGRA buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedRgbaFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Internal structured failures preserve enough state for tests and accurate
/// mapping into the frozen canonical perception error taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CaptureFailure {
    Unavailable(String),
    MonitorIndexOutOfRange {
        requested: usize,
        available: usize,
    },
    MonitorDisappeared {
        id: u32,
        name: String,
    },
    ImageBoundsExceeded {
        width: u32,
        height: u32,
        rgba_bytes: usize,
    },
    InvalidRgbaLayout {
        expected: usize,
        actual: usize,
    },
}

impl CaptureFailure {
    fn into_backend_error(self) -> PerceptionBackendError {
        let message = match self {
            Self::Unavailable(message) => message,
            Self::MonitorIndexOutOfRange {
                requested,
                available,
            } => format!(
                "xcap monitor index {requested} is unavailable; deterministic monitor count is {available}"
            ),
            Self::MonitorDisappeared { id, name } => format!(
                "xcap monitor disappeared or changed before capture (id={id}, name={name:?})"
            ),
            Self::ImageBoundsExceeded {
                width,
                height,
                rgba_bytes,
            } => format!(
                "xcap capture exceeds bounds: {width}x{height}, {rgba_bytes} RGBA bytes; \
                 limits are {MAX_CAPTURE_WIDTH}x{MAX_CAPTURE_HEIGHT} and {MAX_CAPTURE_RGBA_BYTES} bytes"
            ),
            Self::InvalidRgbaLayout { expected, actual } => format!(
                "xcap returned invalid RGBA layout: expected {expected} bytes, got {actual}"
            ),
        };
        PerceptionBackendError::BackendUnavailable(message)
    }
}

/// Testable, synchronous platform boundary. Implementations must be safe to
/// call from `spawn_blocking`; no object is ever marked Send/Sync manually.
trait CaptureDriver: Send + Sync {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorDescriptor>, CaptureFailure>;

    fn capture_monitor(
        &self,
        monitor: &MonitorDescriptor,
    ) -> Result<CapturedRgbaFrame, CaptureFailure>;
}

/// Production driver backed by the maintained `xcap` crate.
struct SystemXcapDriver;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn descriptor_from_monitor(monitor: &xcap::Monitor) -> Result<MonitorDescriptor, CaptureFailure> {
    let property = |name: &str, error: xcap::XCapError| {
        CaptureFailure::Unavailable(format!("xcap could not read monitor {name}: {error}"))
    };
    Ok(MonitorDescriptor {
        id: monitor.id().map_err(|error| property("id", error))?,
        name: monitor.name().map_err(|error| property("name", error))?,
        x: monitor
            .x()
            .map_err(|error| property("x coordinate", error))?,
        y: monitor
            .y()
            .map_err(|error| property("y coordinate", error))?,
        width: monitor.width().map_err(|error| property("width", error))?,
        height: monitor
            .height()
            .map_err(|error| property("height", error))?,
        is_primary: monitor
            .is_primary()
            .map_err(|error| property("primary flag", error))?,
    })
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn monitor_matches_descriptor(
    monitor: &xcap::Monitor,
    descriptor: &MonitorDescriptor,
) -> Result<bool, CaptureFailure> {
    Ok(descriptor_from_monitor(monitor)? == *descriptor)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
impl CaptureDriver for SystemXcapDriver {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorDescriptor>, CaptureFailure> {
        xcap::Monitor::all()
            .map_err(|error| {
                CaptureFailure::Unavailable(format!(
                    "xcap could not enumerate monitors (headless, permission denied, or unsupported desktop): {error}"
                ))
            })
            .and_then(|monitors| {
                monitors
                    .iter()
                    .map(descriptor_from_monitor)
                    .collect::<Result<Vec<_>, _>>()
            })
    }

    fn capture_monitor(
        &self,
        descriptor: &MonitorDescriptor,
    ) -> Result<CapturedRgbaFrame, CaptureFailure> {
        // Re-enumerate just before capture. A display index may otherwise point
        // at a different monitor after hot-plug/reconfiguration.
        let monitors = xcap::Monitor::all().map_err(|error| {
            CaptureFailure::Unavailable(format!(
                "xcap could not re-enumerate monitors before capture: {error}"
            ))
        })?;
        let mut matching_monitor = None;
        for monitor in monitors {
            if monitor_matches_descriptor(&monitor, descriptor)? {
                matching_monitor = Some(monitor);
                break;
            }
        }
        let monitor = matching_monitor.ok_or_else(|| CaptureFailure::MonitorDisappeared {
            id: descriptor.id,
            name: descriptor.name.clone(),
        })?;

        let image = monitor.capture_image().map_err(|error| {
            CaptureFailure::Unavailable(format!(
                "xcap failed to capture monitor {:?}: {error}",
                descriptor.name
            ))
        })?;
        let width = image.width();
        let height = image.height();
        Ok(CapturedRgbaFrame {
            width,
            height,
            // xcap exposes `image::RgbaImage`; `into_raw` is therefore an
            // explicit RGBA8 byte sequence rather than a platform BGRA guess.
            rgba: image.into_raw(),
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl CaptureDriver for SystemXcapDriver {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorDescriptor>, CaptureFailure> {
        Err(CaptureFailure::Unavailable(
            "xcap vision capture is unsupported on this target platform".to_string(),
        ))
    }

    fn capture_monitor(
        &self,
        _monitor: &MonitorDescriptor,
    ) -> Result<CapturedRgbaFrame, CaptureFailure> {
        Err(CaptureFailure::Unavailable(
            "xcap vision capture is unsupported on this target platform".to_string(),
        ))
    }
}

fn stable_sort_monitors(monitors: &mut [MonitorDescriptor]) {
    monitors.sort_by(|left, right| {
        right
            .is_primary
            .cmp(&left.is_primary)
            .then_with(|| left.x.cmp(&right.x))
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn select_monitor(
    mut monitors: Vec<MonitorDescriptor>,
    monitor_index: usize,
) -> Result<MonitorDescriptor, CaptureFailure> {
    stable_sort_monitors(&mut monitors);
    let available = monitors.len();
    monitors
        .get(monitor_index)
        .cloned()
        .ok_or(CaptureFailure::MonitorIndexOutOfRange {
            requested: monitor_index,
            available,
        })
}

fn expected_rgba_bytes(width: u32, height: u32) -> Result<usize, CaptureFailure> {
    let pixels = usize::try_from(width).ok().and_then(|width| {
        usize::try_from(height)
            .ok()
            .and_then(|height| width.checked_mul(height))
    });
    let rgba_bytes = pixels.and_then(|pixels| pixels.checked_mul(4)).ok_or(
        CaptureFailure::ImageBoundsExceeded {
            width,
            height,
            rgba_bytes: usize::MAX,
        },
    )?;

    if width == 0
        || height == 0
        || width > MAX_CAPTURE_WIDTH
        || height > MAX_CAPTURE_HEIGHT
        || rgba_bytes > MAX_CAPTURE_RGBA_BYTES
    {
        return Err(CaptureFailure::ImageBoundsExceeded {
            width,
            height,
            rgba_bytes,
        });
    }
    Ok(rgba_bytes)
}

fn validate_monitor_bounds(monitor: &MonitorDescriptor) -> Result<(), CaptureFailure> {
    expected_rgba_bytes(monitor.width, monitor.height).map(|_| ())
}

fn validate_frame(frame: &CapturedRgbaFrame) -> Result<(), CaptureFailure> {
    let expected = expected_rgba_bytes(frame.width, frame.height)?;
    if frame.rgba.len() != expected {
        return Err(CaptureFailure::InvalidRgbaLayout {
            expected,
            actual: frame.rgba.len(),
        });
    }
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn encode_frame(
    frame: CapturedRgbaFrame,
    output_format: ScreenshotFormat,
) -> Result<Vec<u8>, PerceptionBackendError> {
    validate_frame(&frame).map_err(CaptureFailure::into_backend_error)?;
    let image = RgbaImage::from_raw(frame.width, frame.height, frame.rgba).ok_or_else(|| {
        PerceptionBackendError::BackendUnavailable(
            "xcap returned an RGBA frame that could not be reconstructed".to_string(),
        )
    })?;
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut encoded, output_format.image_format())
        .map_err(|error| {
            PerceptionBackendError::BackendUnavailable(format!(
                "failed to encode xcap RGBA capture as {}: {error}",
                output_format.label()
            ))
        })?;
    let bytes = encoded.into_inner();
    if bytes.is_empty() {
        return Err(PerceptionBackendError::BackendUnavailable(
            "xcap encoder returned an empty screenshot".to_string(),
        ));
    }
    Ok(bytes)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn encode_frame(
    _frame: CapturedRgbaFrame,
    _output_format: ScreenshotFormat,
) -> Result<Vec<u8>, PerceptionBackendError> {
    Err(PerceptionBackendError::BackendUnavailable(
        "xcap image encoding is unsupported on this target platform".to_string(),
    ))
}

fn captured_at_epoch_ms() -> Result<i64, PerceptionBackendError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            PerceptionBackendError::BackendUnavailable(format!(
                "system clock is before the Unix epoch: {error}"
            ))
        })?
        .as_millis();
    i64::try_from(millis).map_err(|_| {
        PerceptionBackendError::BackendUnavailable(
            "system timestamp cannot be represented as epoch milliseconds".to_string(),
        )
    })
}

fn capture_with_driver(
    driver: &dyn CaptureDriver,
    monitor_index: usize,
    output_format: ScreenshotFormat,
) -> Result<ScreenshotBytes, PerceptionBackendError> {
    let monitors = driver
        .enumerate_monitors()
        .map_err(CaptureFailure::into_backend_error)?;
    let monitor =
        select_monitor(monitors, monitor_index).map_err(CaptureFailure::into_backend_error)?;
    validate_monitor_bounds(&monitor).map_err(CaptureFailure::into_backend_error)?;

    let frame = driver
        .capture_monitor(&monitor)
        .map_err(CaptureFailure::into_backend_error)?;
    if frame.width != monitor.width || frame.height != monitor.height {
        return Err(CaptureFailure::MonitorDisappeared {
            id: monitor.id,
            name: monitor.name,
        }
        .into_backend_error());
    }
    let bytes = encode_frame(frame, output_format)?;
    Ok(ScreenshotBytes {
        bytes,
        format: output_format.label().to_string(),
        captured_at_ms: captured_at_epoch_ms()?,
    })
}

/// Real XCap-backed screen-capture implementation. It remains a selectable
/// backend object only; production registration is intentionally out of scope.
pub struct XcapVisionBackend {
    config: XcapVisionConfig,
    driver: Arc<dyn CaptureDriver>,
}

impl XcapVisionBackend {
    /// Construct a backend using the platform `xcap` driver.
    pub fn new(config: XcapVisionConfig) -> Self {
        Self {
            config,
            driver: Arc::new(SystemXcapDriver),
        }
    }

    /// Construct a backend targeting the deterministic primary-monitor slot.
    pub fn default_monitor() -> Self {
        Self::new(XcapVisionConfig::default())
    }

    /// Return the immutable capture configuration.
    pub fn config(&self) -> &XcapVisionConfig {
        &self.config
    }

    #[cfg(test)]
    fn with_driver(config: XcapVisionConfig, driver: Arc<dyn CaptureDriver>) -> Self {
        Self { config, driver }
    }
}

#[async_trait]
impl VisionBackend for XcapVisionBackend {
    async fn capture(&self) -> Result<ScreenshotBytes, PerceptionBackendError> {
        let output_format = ScreenshotFormat::parse(&self.config.format)?;
        let monitor_index = self.config.monitor_index;
        let driver = Arc::clone(&self.driver);
        tokio::task::spawn_blocking(move || {
            capture_with_driver(driver.as_ref(), monitor_index, output_format)
        })
        .await
        .map_err(|error| {
            PerceptionBackendError::Stream(format!("xcap screenshot worker task failed: {error}"))
        })?
    }

    fn name(&self) -> &'static str {
        "xcap_vision"
    }

    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        // Validate format before touching desktop state, then prove the
        // configured stable monitor slot is currently usable.
        let _output_format = ScreenshotFormat::parse(&self.config.format)?;
        let monitor_index = self.config.monitor_index;
        let driver = Arc::clone(&self.driver);
        tokio::task::spawn_blocking(move || {
            let monitors = driver
                .enumerate_monitors()
                .map_err(CaptureFailure::into_backend_error)?;
            let monitor = select_monitor(monitors, monitor_index)
                .map_err(CaptureFailure::into_backend_error)?;
            validate_monitor_bounds(&monitor).map_err(CaptureFailure::into_backend_error)
        })
        .await
        .map_err(|error| {
            PerceptionBackendError::Stream(format!("xcap ping worker task failed: {error}"))
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FakeCaptureDriver {
        monitors: Vec<MonitorDescriptor>,
        capture_result: Result<CapturedRgbaFrame, CaptureFailure>,
    }

    impl CaptureDriver for FakeCaptureDriver {
        fn enumerate_monitors(&self) -> Result<Vec<MonitorDescriptor>, CaptureFailure> {
            Ok(self.monitors.clone())
        }

        fn capture_monitor(
            &self,
            _monitor: &MonitorDescriptor,
        ) -> Result<CapturedRgbaFrame, CaptureFailure> {
            self.capture_result.clone()
        }
    }

    fn monitor(id: u32, name: &str, x: i32, y: i32, is_primary: bool) -> MonitorDescriptor {
        MonitorDescriptor {
            id,
            name: name.to_string(),
            x,
            y,
            width: 2,
            height: 2,
            is_primary,
        }
    }

    fn rgba_frame(width: u32, height: u32) -> CapturedRgbaFrame {
        CapturedRgbaFrame {
            width,
            height,
            rgba: vec![0x42; usize::try_from(width * height * 4).unwrap()],
        }
    }

    fn fake_backend(
        config: XcapVisionConfig,
        monitors: Vec<MonitorDescriptor>,
        capture_result: Result<CapturedRgbaFrame, CaptureFailure>,
    ) -> XcapVisionBackend {
        XcapVisionBackend::with_driver(
            config,
            Arc::new(FakeCaptureDriver {
                monitors,
                capture_result,
            }),
        )
    }

    #[test]
    fn default_config_has_primary_monitor_slot_and_png_format() {
        let backend = XcapVisionBackend::default_monitor();
        assert_eq!(backend.config().monitor_index, 0);
        assert_eq!(backend.config().format, "png");
        assert_eq!(backend.name(), "xcap_vision");
    }

    #[test]
    fn monitor_order_is_primary_then_geometry_name_and_id() {
        let selected = select_monitor(
            vec![
                monitor(3, "right", 100, 0, false),
                monitor(1, "primary", 0, 0, true),
                monitor(2, "left", -100, 0, false),
            ],
            0,
        )
        .unwrap();
        assert_eq!(
            selected.id, 1,
            "primary monitor is deterministic index zero"
        );

        let second = select_monitor(
            vec![
                monitor(3, "right", 100, 0, false),
                monitor(1, "primary", 0, 0, true),
                monitor(2, "left", -100, 0, false),
            ],
            1,
        )
        .unwrap();
        assert_eq!(second.id, 2, "non-primary monitors sort by geometry");
    }

    #[test]
    fn out_of_range_monitor_selection_is_structured() {
        assert_eq!(
            select_monitor(vec![monitor(1, "primary", 0, 0, true)], 1),
            Err(CaptureFailure::MonitorIndexOutOfRange {
                requested: 1,
                available: 1,
            })
        );
    }

    #[test]
    fn raw_rgba_bounds_reject_oversized_and_invalid_images() {
        assert!(matches!(
            expected_rgba_bytes(MAX_CAPTURE_WIDTH + 1, 1),
            Err(CaptureFailure::ImageBoundsExceeded { .. })
        ));
        assert!(matches!(
            expected_rgba_bytes(MAX_CAPTURE_WIDTH, MAX_CAPTURE_HEIGHT),
            Err(CaptureFailure::ImageBoundsExceeded { .. })
        ));
        assert!(expected_rgba_bytes(7_680, 4_320).is_ok());
        assert!(matches!(
            expected_rgba_bytes(0, 1),
            Err(CaptureFailure::ImageBoundsExceeded { .. })
        ));
        let invalid = CapturedRgbaFrame {
            width: 2,
            height: 2,
            rgba: vec![0; 3],
        };
        assert_eq!(
            validate_frame(&invalid),
            Err(CaptureFailure::InvalidRgbaLayout {
                expected: 16,
                actual: 3,
            })
        );
    }

    #[tokio::test]
    async fn mock_capture_returns_real_png_bytes_and_matching_format() {
        let backend = fake_backend(
            XcapVisionConfig::default(),
            vec![monitor(1, "primary", 0, 0, true)],
            Ok(rgba_frame(2, 2)),
        );
        let screenshot = backend.capture().await.expect("mock capture must encode");
        assert_eq!(screenshot.format, "png");
        assert!(screenshot.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(screenshot.captured_at_ms > 0);
    }

    #[tokio::test]
    async fn mock_capture_normalizes_jpg_to_actual_jpeg_format() {
        let backend = fake_backend(
            XcapVisionConfig {
                monitor_index: 0,
                format: "jpg".to_string(),
            },
            vec![monitor(1, "primary", 0, 0, true)],
            Ok(rgba_frame(2, 2)),
        );
        let screenshot = backend.capture().await.expect("mock capture must encode");
        assert_eq!(screenshot.format, "jpeg");
        assert!(screenshot.bytes.starts_with(&[0xff, 0xd8, 0xff]));
    }

    #[tokio::test]
    async fn display_disappearance_fails_closed_with_backend_unavailable() {
        let backend = fake_backend(
            XcapVisionConfig::default(),
            vec![monitor(1, "primary", 0, 0, true)],
            Err(CaptureFailure::MonitorDisappeared {
                id: 1,
                name: "primary".to_string(),
            }),
        );
        let error = backend
            .capture()
            .await
            .expect_err("disappeared display must not capture");
        match error {
            PerceptionBackendError::BackendUnavailable(message) => {
                assert!(message.contains("disappeared"));
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ping_checks_format_and_selected_monitor_without_capturing() {
        let backend = fake_backend(
            XcapVisionConfig::default(),
            vec![monitor(1, "primary", 0, 0, true)],
            Err(CaptureFailure::Unavailable(
                "capture should not run during ping".to_string(),
            )),
        );
        assert!(backend.ping().await.is_ok());

        let invalid_format = fake_backend(
            XcapVisionConfig {
                monitor_index: 0,
                format: "webp".to_string(),
            },
            vec![monitor(1, "primary", 0, 0, true)],
            Ok(rgba_frame(2, 2)),
        );
        assert!(matches!(
            invalid_format.ping().await,
            Err(PerceptionBackendError::Provider(_))
        ));
    }

    #[test]
    fn xcap_vision_backend_is_send_sync_without_manual_unsafe_impls() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<XcapVisionBackend>();
        assert_send_sync::<dyn VisionBackend>();
    }

    /// REAL HARDWARE TEST: run only in a known interactive desktop session.
    /// It is intentionally ignored in normal unit runs so a headless CI result
    /// cannot be mislabeled as a successful physical capture.
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    #[ignore = "requires an interactive desktop and an accessible physical monitor"]
    async fn real_xcap_hardware_capture_smoke() {
        let screenshot = XcapVisionBackend::default_monitor()
            .capture()
            .await
            .expect("interactive desktop capture must succeed");
        assert_eq!(screenshot.format, "png");
        assert!(screenshot.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(screenshot.captured_at_ms > 0);
    }
}
