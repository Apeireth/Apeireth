//! 双后端引擎入口：把「状态 → uniform」接到真实的画布上。
//!
//! ## 后端形态（ADR-0015 §2.6.2 已定案，勿改）
//!
//! `webgl` 与 `webgpu` 两个后端在**同一个 wasm 产物里编译期共存**，
//! **运行期二选一** —— 选在哪一刻发生，由宿主能力决定。
//!
//! ## 引擎创建的 5 步（顺序不可换）
//!
//! ```text
//! 1. wgpu::util::is_browser_webgpu_supported().await
//!      ← 判据是「requestAdapter() 解析出非 null adapter」，**不是**
//!        `navigator.gpu` 的存在性（实测：宿主有 navigator.gpu 但
//!        requestAdapter() 返 null 时，navigator.gpu 判据是错的）
//!      ← ⚠ **只探这一次**：早期实现还走
//!        `new_instance_with_webgpu_detection` ⇒ 它内部**再探一次**
//!        （`wgpu-30.0.1/src/util/init.rs:122`）⇒ 启动时两趟往返。
//! 2. 建 Instance
//!      ← 走 `wgpu::Instance::new` + **受限**的 backends 集合；
//!        **禁** Instance::new(Backends::all())
//!        （实测：宿主有 navigator.gpu 但拿不到 adapter 时，
//!          Instance::new(Backends::all()) 会走进 NotFound 死路，
//!          且 wgpu 在 request_adapter 阶段**没有任何回落**）
//!      ← ⚠ **三个后端偏好都必须先守卫「本产物编入了哪些后端」**：
//!        否则 `Instance::new` 会 **panic**
//!        （`wgpu-30.0.1/src/api/instance.rs:73-79`）⇒ 浏览器里 = 页面崩溃。
//! 3. instance.create_surface(SurfaceTarget::Canvas(canvas))
//!      ← **必须先于 adapter**：WebGL 后端没有 compatible_surface
//!        就产不出 adapter（wgpu-hal-30.0.1/src/gles/web.rs:123-144
//!        在 surface_hint == None 时返回空）
//! 4. instance.request_adapter(&RequestAdapterOptions {
//!        compatible_surface: Some(&surface), .. })
//! 5. adapter.request_device(&DeviceDescriptor {
//!        required_limits: adapter.limits(), .. })
//!      ← **必须取 adapter.limits()**：WebGL2 无 compute，
//!        Limits::default() 要 max_compute_workgroups_per_dimension
//!        = 65535，实测被拒（LimitsExceeded）
//! ```
//!
//! ## ⭐ 画布尺寸会变（resize 通路）
//!
//! 画布尺寸会变（窗口 resize / DPR 变化 / 宿主改 `canvas.width/height`）。
//! 早期实现**只在 `create` 里**配置一次表面 ⇒ 尺寸变了以后：场景层按**新**
//! 尺寸算几何、GPU 按**旧** config 出图（拉伸 / 裁切）、`uRes` 与真实
//! framebuffer 不符；`gl_FragCoord` 的翻转高度陈旧（竖直错位）；WebGPU 后端
//! 改 `canvas.width/height` 还会**作废交换链** ⇒ `get_current_texture()`
//! 永久 `Lost` ⇒ 没有重建入口就**永远画不出来**。
//!
//! ⇒ 现在有 [`Engine::resize`]（重新配置表面 + 更新配置）与
//! [`Engine::reconfigure_surface`]（交换链失效后的恢复路径）。
//!
//! ## ⭐ 设备事件是**异步**的
//!
//! 设备错误与设备丢失由后端**异步**投递 ⇒ 引擎每帧**先 poll** 再取事件
//! （wgpu 30 的 `Device::poll` 在 **GLES 后端不是 no-op** ⇒ 不 poll ⇒
//! 后端错误**永远不派发** ⇒ 失败完全静默）。事件带**帧号**；宿主即使
//! **不画帧**，也能用 [`Engine::poll_device`] 取走积压的事件。
//!
//! ## 画布只能绑一种 context
//!
//! 一个 canvas 只能绑一种 context 类型（先用 WebGPU 后端 `create_surface`
//! 再用 WebGL 后端会报 `canvas already in use`）⇒ **后端必须在碰画布之前
//! 定死**。本引擎因此在第 2 步就把后端钉死，第 3 步才碰画布。
//!
//! ## 色彩管线（显式契约）
//!
//! wgpu 的表面格式列表会把 **sRGB 格式排在前面**
//! （`wgpu-core-30.0.1/src/device/global.rs:56` 的
//! `hal_caps.formats.sort_by_key(|fc| !fc.format.is_srgb())`）。
//! 若直接取 `formats[0]`：
//!
//! | 后端 | formats[0] | 后端在写出时 | 同一片元值 0.35 的画布字节 |
//! |---|---|---|---|
//! | WebGL（GLES） | `Rgba8UnormSrgb` | 做 linear→sRGB 编码 | **160** |
//! | WebGPU（浏览器） | `Rgba8Unorm` | 不做编码 | **89** |
//!
//! ⇒ 直接取 `formats[0]` 会让两个后端**画出不同的像素**。本引擎因此
//! **显式改选非 sRGB 格式**（[`select_surface_format`]），并把实际口径
//! 记进 [`ColorProfile`]。详见该函数的文档。

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use wgpu::util::DeviceExt;

use crate::bindings::{BindingError, UniformBufferLayout, UniformValues};
use crate::context::{GpuContext, GpuError, Tier};
use crate::glsl::{self, GlslError};

/// **设备轮询的等待上限**（单次 poll）。
///
/// ## 为什么要有上限
///
/// `PollType::wait_indefinitely()` 在 **GLES 后端**会**阻塞到最近一次提交完成**
/// ⇒ ⇒ 设备忙时**整页卡住**；而 wgpu 的 `Wait` 若超时则返回 `PollError::Timeout`
/// ⇒ ⇒ **超时不是设备故障**（只是「还没做完」）。
///
/// ⇒ ⇒ 实测：场景面（GLES 后端）用无限等待 ⇒ 直接 `Timeout` 报错 ⇒ 画面全废。
const DEVICE_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

/// 有界地轮询设备 ⇒ 让后端异步错误在**这一帧内**派发。
///
/// ⚠️ **超时不算失败**（`PollError::Timeout` ⇒ 设备尚未完成 ⇒ 下一帧继续）。
/// ⇒ ⇒ 只有「真错误」才上报（设备丢失等由 `on_uncaptured_error` / 丢失回调收集）。
fn poll_device_bounded(device: &wgpu::Device) -> Result<(), GpuError> {
    match device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(DEVICE_POLL_TIMEOUT),
    }) {
        // `PollStatus` 只表明「提交是否已完成」⇒ 不是错误。
        Ok(_status) => Ok(()),
        // ⚠️ **超时是正常状态**（设备还没做完）⇒ 不当失败。
        Err(wgpu::PollError::Timeout) => Ok(()),
        Err(error) => Err(GpuError::DeviceError(format!("设备轮询失败：{error:?}"))),
    }
}

/// 画布目标。
///
/// 浏览器宿主用 [`SurfaceTargetInput::HtmlCanvas`]；**原生窗口路径属
/// ADR-0015 §2.5 的第二阶段，本批未接线** ⇒ 非 wasm32 宿主只有一个
/// 「不支持」的取值。
#[derive(Clone)]
pub enum SurfaceTargetInput {
    /// 浏览器 HTML 画布（wasm32 宿主）。
    #[cfg(target_arch = "wasm32")]
    HtmlCanvas(web_sys::HtmlCanvasElement),
    /// 本宿主没有画布目标（原生窗口路径未接线）。
    #[cfg(not(target_arch = "wasm32"))]
    Unsupported,
}

/// 后端偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    /// 运行期按宿主能力选（默认）：宿主能给出可用的 WebGPU adapter 就走
    /// WebGPU，否则走 WebGL。
    Auto,
    /// 强制走 WebGL（wgpu-core / GLES 后端）。
    WebGl,
    /// 强制走浏览器 WebGPU 后端；宿主不支持时**显式报错**（不回落）。
    WebGpu,
}

impl BackendPreference {
    /// 该偏好是否要求宿主具备 WebGPU。
    pub fn requires_webgpu(&self) -> bool {
        matches!(self, BackendPreference::WebGpu)
    }
}

/// 画布字节的编码方式（引擎的**显式色彩契约**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputEncoding {
    /// 画布**直接接收**片元输出的字节（后端不做 OETF）。
    ///
    /// 这与生产路径同义：生产侧的 WebGL2 上下文用默认 RGBA8 绘制缓冲，
    /// 写入即字节。
    ///
    /// ⚠ **只对 8-bit unorm 成立**：fp16 表面是**线性**的，浏览器呈现时
    /// 会补 sRGB OETF ⇒ 它的字节**不是**显示编码 ⇒ 不得归到本档。
    RawBytes,
    /// 画布是 sRGB 目标：后端在写出时做 linear→sRGB 编码。
    ///
    /// 此时片元输出被**二次编码**，与生产路径差一次 OETF。
    SrgbEncoded,
}

/// 引擎的色彩档案（实测口径，不是猜测）。
#[derive(Debug, Clone, PartialEq)]
pub struct ColorProfile {
    /// 实际选定的表面格式。
    pub surface_format: wgpu::TextureFormat,
    /// 该格式下的字节编码方式。
    pub encoding: OutputEncoding,
    /// 后端提供的全部候选格式（按 wgpu 给出的顺序）。
    pub available_formats: Vec<wgpu::TextureFormat>,
}

/// 表面格式选择策略（**纯函数**，可在 native 上单测）。
///
/// 显式约定：**优先选非 sRGB 格式**（raw write），理由见模块文档的
/// 色彩管线小节。回退顺序：
/// `Rgba8Unorm` → `Bgra8Unorm` → `Rgba8UnormSrgb` → `Bgra8UnormSrgb`。
///
/// ## ⭐ 为什么**没有** fp16（`Rgba16Float`）
///
/// fp16 表面是**线性**的：着色器输出的数值是线性的，**浏览器呈现时**才补
/// sRGB OETF。而 8-bit unorm 的字节**已经是**显示编码 ⇒ 两者**不是**同一
/// 编码口径。
///
/// ⇒ 早期实现把 `Rgba16Float` 归进 `RawBytes`（「画布直接接收片元输出的字节，
/// 后端不做 OETF」）⇒ **口径错**：宿主会以为画布字节就是显示编码 ⇒ fp16
/// 分支会**明显偏亮**（线性 0.5 显示成 sRGB 0.5 ⇒ 实际更亮）。
///
/// ⇒ 现在：**fp16 不在候选里**。若宿主只提供 fp16（两个浏览器后端都不会：
/// 它们必定提供 `rgba8unorm` / `bgra8unorm`）⇒ 本函数返回 `None` ⇒ 引擎
/// **显式报错**（[`GpuError::Unsupported`]），而不是**静默**按 RawBytes 处理。
pub fn select_surface_format(
    available: &[wgpu::TextureFormat],
) -> Option<(wgpu::TextureFormat, OutputEncoding)> {
    const RAW_PREFERENCE: [wgpu::TextureFormat; 2] = [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ];
    const SRGB_PREFERENCE: [wgpu::TextureFormat; 2] = [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    ];
    for format in RAW_PREFERENCE {
        if available.contains(&format) {
            return Some((format, OutputEncoding::RawBytes));
        }
    }
    for format in SRGB_PREFERENCE {
        if available.contains(&format) {
            return Some((format, OutputEncoding::SrgbEncoded));
        }
    }
    // 宿主只给了本引擎**没有显式编码口径**的格式（典型：只有 fp16）
    // ⇒ **显式拒绝**（`None`）⇒ 调用方报错。
    //
    // ⚠ 这里**不能**回落成「取首个 + 按 `is_srgb()` 归类」：那会把 fp16
    //   静默归成 `RawBytes` ⇒ 口径错 ⇒ 画面偏亮（且宿主无从察觉）。
    None
}

/// 引擎创建参数。
#[derive(Debug, Clone, PartialEq)]
pub struct EngineOptions {
    /// 后端偏好。
    pub backend: BackendPreference,
    /// 画布宽度（像素）。
    pub width: u32,
    /// 画布高度（像素）。
    pub height: u32,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            backend: BackendPreference::Auto,
            width: 320,
            height: 240,
        }
    }
}

/// 引擎的对外报告（宿主可读的实测事实）。
#[derive(Debug, Clone, PartialEq)]
pub struct EngineReport {
    /// 请求的后端偏好。
    pub requested_backend: BackendPreference,
    /// 宿主是否具备可用的 WebGPU adapter（`requestAdapter()` 解析出非 null）。
    pub webgpu_supported: bool,
    /// 实际用上的后端。
    pub backend: wgpu::Backend,
    /// adapter 名称。
    pub adapter_name: String,
    /// adapter 厂商 ID（后端相关）。
    pub adapter_vendor: u32,
    /// adapter 设备 ID（后端相关）。
    pub adapter_device: u32,
    /// 驱动名。
    pub driver: String,
    /// 驱动信息。
    pub driver_info: String,
    /// 设备类型。
    pub device_type: wgpu::DeviceType,
    /// 色彩档案。
    pub color: ColorProfile,
    /// 画布宽度。
    pub width: u32,
    /// 画布高度。
    pub height: u32,
}

/// 一次绘制后的报告。
#[derive(Debug, Clone, PartialEq)]
pub struct FrameReport {
    /// 实际用上的后端。
    pub backend: wgpu::Backend,
    /// 实际选定的表面格式。
    pub surface_format: wgpu::TextureFormat,
    /// 字节编码方式。
    pub encoding: OutputEncoding,
    /// 本帧画的顶点数。
    pub vertex_count: u32,
    /// 本帧写入的 uniform **成员**数（= uniform 块里的成员个数）。
    pub uniform_members: u32,
    /// 本帧的**帧号**（从 1 起）。
    ///
    /// ⚠ 用途：设备事件带帧号 ⇒ 宿主能把「哪一帧出的问题」对上来。
    pub frame_number: u64,
}

/// 一次绘制请求的描述（与业务无关：着色器源 + 顶点数据 + uniform 值）。
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDescriptor {
    /// 顶点着色器（GLSL ES 3.0 源）。
    pub vertex_glsl: String,
    /// 片元着色器（GLSL ES 3.0 源）。
    pub fragment_glsl: String,
    /// 顶点数据（`vec2` 一对一个顶点；位置语义由着色器自己定义）。
    pub vertex_data: Vec<f32>,
    /// 要画的顶点数（`None` = 全部）。
    pub vertex_count: Option<u32>,
}

impl SceneDescriptor {
    /// 用两段 GLSL 源建一个绘制请求（无顶点数据）。
    pub fn new(vertex_glsl: impl Into<String>, fragment_glsl: impl Into<String>) -> Self {
        Self {
            vertex_glsl: vertex_glsl.into(),
            fragment_glsl: fragment_glsl.into(),
            vertex_data: Vec::new(),
            vertex_count: None,
        }
    }

    /// 附上顶点数据（`vec2` 一对一个顶点）。
    pub fn with_vertex_data(mut self, data: Vec<f32>) -> Self {
        self.vertex_data = data;
        self
    }

    /// 指定要画的顶点数。
    pub fn with_vertex_count(mut self, count: u32) -> Self {
        self.vertex_count = Some(count);
        self
    }

    /// 实际要画的顶点数。
    pub fn resolved_vertex_count(&self) -> u32 {
        self.vertex_count
            .unwrap_or((self.vertex_data.len() / 2) as u32)
    }
}

/// 已建好的绘制管线（含 uniform 缓冲、顶点缓冲与绑定组）。
pub struct ScenePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: Option<wgpu::Buffer>,
    vertex_buffer: Option<wgpu::Buffer>,
    layout: UniformBufferLayout,
    vertex_count: u32,
    scratch: Vec<u8>,
    /// 建管线时的表面格式（color target 格式）。
    ///
    /// ⚠ 用途：表面格式**变了** ⇒ 旧的 color target 格式不再匹配 ⇒ 管线
    /// 必须重建（[`Engine::recreate_pipeline`]）⇒ 引擎据此**显式报错**，
    /// 而不是画出一张错格式的图。
    surface_format: wgpu::TextureFormat,
    /// 建管线时的表面**代号**。
    ///
    /// ⚠ 用途：只是**可观测**（宿主能看出表面被重新配置过）。
    /// **尺寸**变化**不**作废管线（管线依赖的是格式，不是尺寸）⇒ 代号不同
    /// **不是**错误。
    surface_generation: u64,
}

impl ScenePipeline {
    /// 本管线的 uniform 布局。
    pub fn uniform_layout(&self) -> &UniformBufferLayout {
        &self.layout
    }

    /// 本管线要画的顶点数。
    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// 建管线时的表面格式。
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// 建管线时的表面**代号**。
    pub fn surface_generation(&self) -> u64 {
        self.surface_generation
    }
}

/// 表面状态（配置 + **代号**）。
///
/// ⚠ 用内部可变性（[`RwLock`]）承载：宿主的 resize 回调通常是 `&self`
/// （借来的引擎引用）⇒ 表面状态必须能在 `&self` 下更新。
#[derive(Debug, Clone)]
struct SurfaceState {
    /// 当前的表面配置。
    config: wgpu::SurfaceConfiguration,
    /// **代号**：每次重新配置表面 **+1**。
    ///
    /// 用途：宿主能看出「表面被重新配置过」⇒ 交换链重建 ⇒ 旧的表面纹理
    /// 全部作废（WebGPU 后端改 `canvas.width/height` 就是这个效果）。
    generation: u64,
}

/// 设备事件（**异步**到达 ⇒ 必须带**帧号**）。
///
/// ## 为什么必须带帧号
///
/// 设备错误 / 设备丢失都是**后端异步**投递的：它们在**某一次** `poll`
/// 之后才到达。若队列不带帧号，宿主无法判断「这是哪一帧出的问题」。
///
/// 更要命的是**时序**：早期实现是在帧**末尾**取走错误，而错误可能**之后**
/// 才到 ⇒ 被归到**下一帧** ⇒ 宿主若停画，就**永远看不到**。
///
/// ⇒ 现在：① 每帧**先 poll** 再取事件（见 [`Engine::poll_device`]）；
/// ② 事件带**到达时的帧号**；③ 宿主即使不画，也能显式调
/// [`Engine::poll_device`] 把积压的事件取走。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// 未捕获的设备错误（校验 / 内部错误）。
    Error {
        /// 事件到达时的帧号（`0` = 还没画过任何一帧）。
        frame: u64,
        /// 错误文本。
        message: String,
    },
    /// **设备丢失**（需重建上下文）。
    Lost {
        /// 事件到达时的帧号（`0` = 还没画过任何一帧）。
        frame: u64,
        /// 丢失原因（wgpu 的 `DeviceLostReason`）。
        reason: String,
        /// 丢失说明（后端给的文本）。
        message: String,
    },
}

/// 双后端引擎：实例 / 适配器 / 设备 / 表面 / 队列。
pub struct Engine {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// 表面状态（配置 + 代号）。
    surface_state: RwLock<SurfaceState>,
    color: ColorProfile,
    backend: wgpu::Backend,
    adapter_name: String,
    adapter_vendor: u32,
    adapter_device: u32,
    driver: String,
    driver_info: String,
    device_type: wgpu::DeviceType,
    requested_backend: BackendPreference,
    webgpu_supported: bool,
    /// 设备事件队列（异步到达 ⇒ 带帧号）。
    device_events: Arc<Mutex<Vec<DeviceEvent>>>,
    /// 当前帧号（设备事件回调据此给事件打帧号）。
    frame_counter: Arc<AtomicU64>,
    /// 管线创建次数（**可观测**：重复创建是重资源操作）。
    pipeline_creations: AtomicU64,
}

impl Engine {
    /// 按 5 步流程建引擎（顺序不可换，见模块文档）。
    ///
    /// # Errors
    ///
    /// - 宿主没有画布目标 → [`GpuError::Unsupported`]；
    /// - 请求的后端在**本编译产物里没编入** → [`GpuError::Unsupported`]；
    /// - 宿主不支持 WebGPU（强制 WebGpu 时）→ [`GpuError::Unsupported`]；
    /// - 建表面失败 → [`GpuError::SurfaceCreationFailed`]；
    /// - 拿不到适配器 → [`GpuError::AdapterUnavailable`]；
    /// - 拿不到设备 → [`GpuError::DeviceRequestFailed`]。
    pub async fn create(
        target: SurfaceTargetInput,
        options: EngineOptions,
    ) -> Result<Self, GpuError> {
        // ── 步骤 1：宿主能力探测（判据 = 可用的 WebGPU adapter）────────────
        //
        // ⚠ **只探一次**：`wgpu::util::new_instance_with_webgpu_detection` 内部
        //   **还会再探一次**（`util/init.rs:122`）⇒ 早期实现一次启动要走
        //   **两趟** `requestAdapter()` 往返 ⇒ 这里改成「自己探一次 + 直接
        //   `Instance::new`」。
        let webgpu_supported = wgpu::util::is_browser_webgpu_supported().await;

        // ── 步骤 2：定死后端 → 建 Instance（禁 Instance::new(Backends::all())）──
        let instance = build_instance(options.backend, webgpu_supported)?;

        // ── 步骤 3：建表面（必须先于 adapter：WebGL 后端无 compatible_surface
        //            就产不出 adapter）────────────────────────────────────────
        let surface = create_surface(&instance, &target)?;

        // ── 步骤 4：带 compatible_surface 申请 adapter ──────────────────────
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::AdapterUnavailable(format!("{e:?}")))?;

        // ── 步骤 5：required_limits 必须取 adapter.limits() ─────────────────
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("apeireth-ui-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| GpuError::DeviceRequestFailed(format!("{e:?}")))?;

        // ⚠ 设备事件队列与帧号必须在**装处理器之前**就绪（处理器要往里写）。
        let device_events: Arc<Mutex<Vec<DeviceEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let frame_counter = Arc::new(AtomicU64::new(0));

        // ⚠ 设备错误不 panic：收进队列，由调用方以 `GpuError` 拿回。
        //
        // ⚠⚠ **装处理器的时机**：wgpu 的 `Device::on_uncaptured_error` 与
        // `Device::set_device_lost_callback` **只能在设备建好之后**装
        // （API 限制）⇒ 「建设备那一刻」到「装上处理器」之间有一个**窗口**。
        // ⇒ 把这个窗口压到**最小**：装完处理器**之前不做任何设备操作**
        // （不 configure 表面、不建着色器模块、不建管线）⇒ 窗口内的错误
        // 不会落到 wgpu 的默认处理器（在浏览器里 = **panic**）。
        {
            let sink = Arc::clone(&device_events);
            let counter = Arc::clone(&frame_counter);
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                if let Ok(mut events) = sink.lock() {
                    events.push(DeviceEvent::Error {
                        frame: counter.load(Ordering::Relaxed),
                        message: error.to_string(),
                    });
                }
            }));
        }
        {
            // 设备丢失：wgpu 30 的入口是 `Device::set_device_lost_callback`
            // （`wgpu-30.0.1/src/api/device.rs:681`）⇒ 引擎**必须**注册它，
            // 否则设备丢失后 `draw_frame` 仍返回 `Ok(FrameReport)`（「画了 N 帧」
            // 但屏上什么都没有），宿主无从判断该重建还是偶发错误。
            let sink = Arc::clone(&device_events);
            let counter = Arc::clone(&frame_counter);
            device.set_device_lost_callback(Box::new(move |reason, message| {
                if let Ok(mut events) = sink.lock() {
                    events.push(DeviceEvent::Lost {
                        frame: counter.load(Ordering::Relaxed),
                        reason: format!("{reason:?}"),
                        message,
                    });
                }
            }));
        }

        let info = adapter.get_info();
        let caps = surface.get_capabilities(&adapter);
        let available_formats = caps.formats.clone();
        let (surface_format, encoding) =
            select_surface_format(&available_formats).ok_or_else(|| {
                GpuError::Unsupported(
                "宿主没有提供本引擎有**显式编码口径**的表面格式（已知：Rgba8Unorm / Bgra8Unorm \
                 及其 sRGB 变体；fp16 是线性目标，不得按 RawBytes 归类）"
                    .to_string(),
            )
            })?;

        let width = options.width.max(1);
        let height = options.height.max(1);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            instance,
            surface,
            device,
            queue,
            surface_state: RwLock::new(SurfaceState {
                config,
                generation: 0,
            }),
            color: ColorProfile {
                surface_format,
                encoding,
                available_formats,
            },
            backend: info.backend,
            adapter_name: info.name.clone(),
            adapter_vendor: info.vendor,
            adapter_device: info.device,
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
            device_type: info.device_type,
            requested_backend: options.backend,
            webgpu_supported,
            device_events,
            frame_counter,
            pipeline_creations: AtomicU64::new(0),
        })
    }

    /// 引擎的对外报告（宿主可读的实测事实）。
    pub fn report(&self) -> EngineReport {
        let (width, height) = self.surface_size();
        EngineReport {
            requested_backend: self.requested_backend,
            webgpu_supported: self.webgpu_supported,
            backend: self.backend,
            adapter_name: self.adapter_name.clone(),
            adapter_vendor: self.adapter_vendor,
            adapter_device: self.adapter_device,
            driver: self.driver.clone(),
            driver_info: self.driver_info.clone(),
            device_type: self.device_type,
            color: self.color.clone(),
            width,
            height,
        }
    }

    /// 色彩档案。
    pub fn color_profile(&self) -> &ColorProfile {
        &self.color
    }

    /// 当前表面尺寸（设备像素）。
    pub fn surface_size(&self) -> (u32, u32) {
        match self.surface_state.read() {
            Ok(state) => (state.config.width, state.config.height),
            Err(_) => (0, 0),
        }
    }

    /// 当前表面**格式**（= color target 格式）。
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        match self.surface_state.read() {
            Ok(state) => state.config.format,
            Err(_) => self.color.surface_format,
        }
    }

    /// 表面**代号**：每次重新配置表面 +1。
    ///
    /// 宿主可用它判断「交换链被重建过」（⇒ 之前拿到的表面纹理全部作废）。
    pub fn surface_generation(&self) -> u64 {
        match self.surface_state.read() {
            Ok(state) => state.generation,
            Err(_) => 0,
        }
    }

    /// ⭐ **改尺寸**：重新配置表面（并更新配置）。
    ///
    /// ## 为什么必须有这条通路
    ///
    /// 画布尺寸会变（窗口 resize / DPR 变化 / 宿主改 `canvas.width/height`）。
    /// 早期实现**只在 `create` 里**配置一次表面 ⇒ 尺寸变了以后：
    ///
    /// - 场景层按**新**尺寸算几何，GPU 按**旧** config 出图 ⇒ **拉伸 / 裁切**，
    ///   且 `uRes` 与真实 framebuffer 不符；
    /// - `gl_FragCoord` 的翻转高度陈旧 ⇒ **竖直错位 / 局部镜像**；
    /// - WebGPU 后端改 `canvas.width/height` 会**作废交换链** ⇒
    ///   `get_current_texture()` 永久 `Lost` ⇒ 没有重建入口就**永远画不出来**。
    ///
    /// ## 管线要不要重建？
    ///
    /// **不要**（除非表面**格式**变了）：渲染管线依赖的是 `config.format`
    /// （color target 格式），**不是** `config.width/height`。而
    /// `gl_FragCoord` 的翻转高度已经改成 **uniform**（见
    /// [`glsl::FRAMEBUFFER_HEIGHT_MEMBER`]）⇒ 改尺寸**不必重编译着色器**。
    ///
    /// ⇒ 重建管线是**极重**的操作（实测 `Page.captureScreenshot` **超时**，
    /// 见 `crates-ui/evidence/p5-b/harness/src/lib.rs`）⇒ 改尺寸**不**触发它。
    /// 需要重建时宿主显式调 [`Engine::recreate_pipeline`]。
    ///
    /// ## 幂等
    ///
    /// 尺寸没变 ⇒ **不动**表面（不重新配置 ⇒ 不作废交换链 ⇒ 不闪一帧）。
    ///
    /// # Errors
    ///
    /// 内部状态锁被毒化 ⇒ [`GpuError::Unsupported`]（极罕见）。
    pub fn resize(&self, width: u32, height: u32) -> Result<(), GpuError> {
        let width = width.max(1);
        let height = height.max(1);
        let mut state = self
            .surface_state
            .write()
            .map_err(|_| GpuError::Unsupported("引擎的表面状态锁被毒化（内部错误）".to_string()))?;
        if state.config.width == width && state.config.height == height {
            return Ok(());
        }
        let mut config = state.config.clone();
        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
        state.config = config;
        state.generation += 1;
        Ok(())
    }

    /// ⭐ **重新配置表面**（尺寸不变）—— **交换链失效后的恢复路径**。
    ///
    /// ## 为什么需要
    ///
    /// WebGPU 后端在 `canvas.width/height` 变化时会让交换链**作废** ⇒
    /// `get_current_texture()` 返回 `Lost`。此时**重新配置表面**（用同一份
    /// config）就能换一条新的交换链 ⇒ 渲染恢复。
    ///
    /// ⇒ 本方法同时是 [`Engine::draw_frame`] 检出 `Lost` 时的**自动恢复**
    /// 与宿主的**显式恢复**入口。
    ///
    /// # Errors
    ///
    /// 内部状态锁被毒化 ⇒ [`GpuError::Unsupported`]（极罕见）。
    pub fn reconfigure_surface(&self) -> Result<(), GpuError> {
        let mut state = self
            .surface_state
            .write()
            .map_err(|_| GpuError::Unsupported("引擎的表面状态锁被毒化（内部错误）".to_string()))?;
        let config = state.config.clone();
        self.surface.configure(&self.device, &config);
        state.generation += 1;
        Ok(())
    }

    /// ⭐ **轮询设备**：把后端**异步**投递的事件取走。
    ///
    /// ## 为什么每帧都要 poll
    ///
    /// wgpu 30 的 `Device::poll`（`wgpu-30.0.1/src/api/device.rs:95`）在
    /// **GLES 后端不是 no-op** ⇒ 不 poll ⇒ 后端错误**永远不派发**到
    /// `on_uncaptured_error` ⇒ 队列恒空 ⇒ 失败完全静默。
    ///
    /// ## 为什么宿主可以单独调它
    ///
    /// 事件是**异步**到达的：若宿主停画（不调 `draw_frame`），积压的事件
    /// 就**永远看不到** ⇒ 宿主显式调本方法即可把积压事件取走。
    ///
    /// # Errors
    ///
    /// 后端 poll 失败（设备已丢等）→ [`GpuError::DeviceError`]。
    pub fn poll_device(&self) -> Result<Vec<DeviceEvent>, GpuError> {
        poll_device_bounded(&self.device)?;
        Ok(self.take_device_events())
    }

    /// 取走（并清空）已收集的设备事件。
    ///
    /// wgpu 的默认处理器会 panic；引擎改成收集 ⇒ 宿主能拿到**具体**的
    /// 校验错误文本，而不是一句 JS 异常。
    pub fn take_device_events(&self) -> Vec<DeviceEvent> {
        match self.device_events.lock() {
            Ok(mut events) => std::mem::take(&mut *events),
            Err(_) => Vec::new(),
        }
    }

    /// 取走（并清空）已收集的**设备错误**文本（不含设备丢失）。
    ///
    /// ⚠ 兼容入口：新的（带帧号的）事件流用 [`Engine::take_device_events`]。
    pub fn take_device_errors(&self) -> Vec<String> {
        self.take_device_events()
            .into_iter()
            .filter_map(|event| match event {
                DeviceEvent::Error { message, .. } => Some(message),
                DeviceEvent::Lost { .. } => None,
            })
            .collect()
    }

    /// 本引擎**已经创建过**多少条管线（**可观测**的重资源计数）。
    ///
    /// 用途：管线创建**极重**（实测 `Page.captureScreenshot` 超时）⇒ 宿主
    /// （与评审者）能一眼看出「是不是在每帧重建管线」。
    pub fn pipeline_creation_count(&self) -> u64 {
        self.pipeline_creations.load(Ordering::Relaxed)
    }

    /// 建一条绘制管线（着色器 → naga IR → pipeline + bind group）。
    ///
    /// 顶点阶段与片元阶段共用 group 0 ⇒ 两个阶段的 uniform 块**各占一个
    /// binding**（片元 = [`glsl::FRAGMENT_UNIFORM_BLOCK_BINDING`]，
    /// 顶点 = [`glsl::VERTEX_UNIFORM_BLOCK_BINDING`]），否则两侧的
    /// `binding=0` 会互相撞车。
    ///
    /// 片元阶段还会做 `gl_FragCoord` 的**坐标语义修正**
    /// （[`glsl::fix_frag_coord_semantics`]）：GLSL ES 规定片元坐标的 y 轴
    /// **向上**，而本引擎的两个后端都把它变成**向下** ⇒ 不修正的话画面是
    /// 上下颠倒的（实测证据见 `crates-ui/evidence/p5-b/`）。
    ///
    /// ## ⭐ 管线**只建一次**
    ///
    /// 一次 `create_scene_pipeline` 会新建 **7 个 GPU 资源**（bind group
    /// layout / pipeline layout / 顶点模块 / 片元模块 / 渲染管线 / uniform
    /// 缓冲 / 顶点缓冲）+ **1 个绑定组**。这是**重资源**操作：实测
    /// 「重建极重 ⇒ `Page.captureScreenshot` **超时**」
    /// （`crates-ui/evidence/p5-b/harness/src/lib.rs` 的注释原文）。
    ///
    /// ⇒ **规范**：管线建一次、长期复用；尺寸变化走 [`Engine::resize`]
    /// （**不**重建管线）；确需重建时显式调 [`Engine::recreate_pipeline`]。
    /// ⇒ 引擎用 [`Engine::pipeline_creation_count`] 暴露创建次数 ⇒
    /// 「是不是在每帧重建」**可观测**。
    ///
    /// # Errors
    ///
    /// - GLSL 解析 / 校验失败 → [`GpuError::ShaderCompilationFailed`]；
    /// - uniform 类型不受支持 → [`GpuError::ShaderCompilationFailed`]；
    /// - 管线 / 绑定组创建失败 → [`GpuError::PipelineCreationFailed`]。
    pub fn create_scene_pipeline(
        &self,
        scene: &SceneDescriptor,
    ) -> Result<ScenePipeline, GpuError> {
        let pipeline = self.build_pipeline(scene)?;
        self.pipeline_creations.fetch_add(1, Ordering::Relaxed);
        Ok(pipeline)
    }

    /// ⭐ **显式重建**一条管线（原地替换 `pipeline` 的内容）。
    ///
    /// ## 什么时候需要
    ///
    /// - 表面**格式**变了（⇒ color target 格式不再匹配）；
    /// - 着色器源变了（⇒ 该换一条管线）。
    ///
    /// ## 什么时候**不**需要
    ///
    /// **尺寸变了不需要**（见 [`Engine::resize`]）：管线依赖的是表面
    /// **格式**，不是尺寸；且 `gl_FragCoord` 的翻转高度已是 uniform ⇒
    /// 改尺寸**不必**重编译着色器 ⇒ 也就**不必**重建这条重资源。
    ///
    /// # Errors
    ///
    /// 同 [`Engine::create_scene_pipeline`]。
    pub fn recreate_pipeline(
        &self,
        pipeline: &mut ScenePipeline,
        scene: &SceneDescriptor,
    ) -> Result<(), GpuError> {
        let rebuilt = self.build_pipeline(scene)?;
        self.pipeline_creations.fetch_add(1, Ordering::Relaxed);
        *pipeline = rebuilt;
        Ok(())
    }

    /// 真正建管线的实现（[`Engine::create_scene_pipeline`] 与
    /// [`Engine::recreate_pipeline`] 共用）。
    fn build_pipeline(&self, scene: &SceneDescriptor) -> Result<ScenePipeline, GpuError> {
        // 顶点阶段：变换 + 解析。
        let vertex_source = glsl::transform_with_block_binding(
            &scene.vertex_glsl,
            glsl::VERTEX_UNIFORM_BLOCK_BINDING,
        )
        .map_err(|e| GpuError::ShaderCompilationFailed(format!("vertex: {e}")))?;
        let vertex_module_ir = glsl::parse(&vertex_source, naga::ShaderStage::Vertex)
            .map_err(|e| GpuError::ShaderCompilationFailed(format!("vertex: {e}")))?;

        // 片元阶段：变换 + 解析。
        //
        // ⚠️ 片元阶段额外做 `gl_FragCoord` 的**坐标语义修正**
        // （glsl::fix_frag_coord_semantics）：GLSL ES 规定片元坐标的 y 轴
        // **向上**，而本引擎的两个后端都把它变成**向下** ⇒ 不修正的话同一份
        // GLSL 源画出来的画面是上下颠倒的（实测证据见 crates-ui/evidence/p5-b/）。
        let fragment_source = glsl::transform_fragment_with_block_binding(
            &scene.fragment_glsl,
            glsl::FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .map_err(|e| GpuError::ShaderCompilationFailed(format!("fragment: {e}")))?;
        let fragment_module_ir = glsl::parse(&fragment_source, naga::ShaderStage::Fragment)
            .map_err(|e| GpuError::ShaderCompilationFailed(format!("fragment: {e}")))?;

        // 顶点阶段的 uniform 尚未接线 ⇒ **显式拒绝**（不静默忽略：静默忽略
        // 会让顶点阶段的 uniform 永远拿到 0 值，画出错位的画面）。
        if !glsl::uniform_declarations(&scene.vertex_glsl).is_empty() {
            return Err(GpuError::Unsupported(
                "顶点阶段的 uniform 尚未接线（本批只支持片元阶段的 uniform 块）".to_string(),
            ));
        }

        // 片元块的成员清单 = 生产 uniform + **引擎注入的帧缓冲高度成员**
        // （⇒ 生产 uniform 的偏移逐字节不变 ⇒ 既有逐像素证据不受影响）。
        // ⚠ 成员清单与片元变换器**共用同一个函数**（`glsl::fragment_uniform_block_members`）
        //   ⇒ 布局层与着色器里的块**必然一致**（否则绑定组会声明着色器没有的
        //   binding ⇒ wgpu 校验报错）。
        let fragment_members = glsl::fragment_uniform_block_members(&scene.fragment_glsl);
        let layout = UniformBufferLayout::from_members(
            &fragment_members,
            glsl::FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .map_err(|e: GlslError| GpuError::ShaderCompilationFailed(e.to_string()))?;

        // 与 naga IR 里的成员偏移对照 —— 不一致就**显式报错**
        // （绝不「静默写错字节」）。
        verify_naga_layout(&fragment_module_ir, &layout)?;

        let surface_format = self.surface_format();
        let surface_generation = self.surface_generation();

        // 显式绑定组布局：整个 uniform 块 = **一个** binding。
        // （WebGL2 后端每个着色阶段能用的 uniform buffer 只有 12–14 个 ⇒
        //  16 个 uniform 必须合并成一个块，见 glsl 模块的 X3 说明。）
        let entries = if layout.is_empty() {
            Vec::new()
        } else {
            vec![wgpu::BindGroupLayoutEntry {
                binding: layout.binding(),
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(layout.total_size()),
                },
                count: None,
            }]
        };
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("apeireth-ui-gpu.uniforms"),
                    entries: &entries,
                });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("apeireth-ui-gpu.scene"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        // 着色器模块直接吃**引擎自己解析出来的 naga IR**（不二次解析）：
        // 引擎的解析结果与 naga IR 里的成员偏移因此是同一份事实。
        let vertex_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("apeireth-ui-gpu.vertex"),
                source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(vertex_module_ir)),
            });
        let fragment_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("apeireth-ui-gpu.fragment"),
                source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(fragment_module_ir)),
            });

        let vertex_buffer_layout = if scene.vertex_data.is_empty() {
            Vec::new()
        } else {
            vec![Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            })]
        };

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("apeireth-ui-gpu.scene"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffer_layout,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        // uniform 缓冲：一个缓冲、装下整个 std140 块。
        let total_size = layout.total_size();
        let uniform_buffer = if total_size == 0 {
            None
        } else {
            Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("apeireth-ui-gpu.uniforms"),
                size: total_size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        };

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apeireth-ui-gpu.uniforms"),
            layout: &bind_group_layout,
            entries: &match &uniform_buffer {
                Some(buffer) => vec![wgpu::BindGroupEntry {
                    binding: layout.binding(),
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset: 0,
                        size: NonZeroU64::new(total_size),
                    }),
                }],
                None => Vec::new(),
            },
        });

        let vertex_buffer = if scene.vertex_data.is_empty() {
            None
        } else {
            let bytes: Vec<u8> = scene
                .vertex_data
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            Some(
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("apeireth-ui-gpu.vertices"),
                        contents: &bytes,
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            )
        };

        let errors = self.take_device_errors();
        if !errors.is_empty() {
            return Err(GpuError::PipelineCreationFailed(errors.join("; ")));
        }

        Ok(ScenePipeline {
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            layout,
            vertex_count: scene.resolved_vertex_count(),
            scratch: vec![0u8; total_size as usize],
            surface_format,
            surface_generation,
        })
    }

    /// 画一帧：uniform → 缓冲 → 渲染 → 呈现。
    ///
    /// ## 每帧的动作顺序（不可换）
    ///
    /// 1. **帧号 +1**（设备事件据此打帧号）；
    /// 2. 写 uniform（宿主的取值 + **引擎自填的帧缓冲高度**）；
    /// 3. 取表面纹理（`Lost` ⇒ **自动重新配置表面**并重试一次）；
    /// 4. 编码 + 提交 + 呈现；
    /// 5. **poll 设备** ⇒ 后端异步错误在这一帧内**派发**（GLES 后端不 poll
    ///    就**永远**不派发 ⇒ 失败完全静默）；
    /// 6. 取设备事件：设备丢失 ⇒ [`GpuError::DeviceLost`]；设备错误 ⇒
    ///    [`GpuError::DeviceError`]。
    ///
    /// # Errors
    ///
    /// - uniform 值与布局不符 → [`GpuError::BindingFailed`]；
    /// - 表面纹理**丢失** → [`GpuError::SurfaceTextureLost`]（需重建表面）；
    /// - 表面纹理**过期** → [`GpuError::SurfaceTextureOutdated`]（尺寸变了）；
    /// - 表面纹理**超时 / 被遮挡** → [`GpuError::SurfaceTextureTimeout`] /
    ///   [`GpuError::SurfaceTextureOccluded`]（**瞬时**，可跳过本帧）；
    /// - 设备丢失 → [`GpuError::DeviceLost`]；
    /// - 设备错误 → [`GpuError::DeviceError`]。
    pub fn draw_frame(
        &self,
        pipeline: &mut ScenePipeline,
        uniforms: &UniformValues,
    ) -> Result<FrameReport, GpuError> {
        // ① 帧号 +1（设备事件回调据此打帧号）。
        let frame_number = self.frame_counter.fetch_add(1, Ordering::Relaxed) + 1;

        // ①′ 管线与当前表面**格式**必须一致（格式变了 ⇒ color target 不匹配
        //     ⇒ 画出来是错格式的图）⇒ **显式报错**，让宿主去重建管线。
        //
        // ⚠ **尺寸**变化**不**触发本检查（见 `Engine::resize` 的文档）。
        let current_format = self.surface_format();
        if pipeline.surface_format != current_format {
            return Err(GpuError::PipelineStale(format!(
                "管线的表面格式是 {:?}，当前表面格式是 {:?} ⇒ 宿主需调用 Engine::recreate_pipeline",
                pipeline.surface_format, current_format
            )));
        }

        // ② 写 uniform：宿主取值 + 引擎自填的帧缓冲高度。
        uniforms
            .write_into(&pipeline.layout, &mut pipeline.scratch)
            .map_err(|e: BindingError| GpuError::BindingFailed(e.to_string()))?;
        self.write_framebuffer_height(pipeline);
        if let Some(buffer) = &pipeline.uniform_buffer {
            if !pipeline.scratch.is_empty() {
                self.queue.write_buffer(buffer, 0, &pipeline.scratch);
            }
        }

        // ③ 取表面纹理（`Lost` ⇒ 自动重新配置表面 ⇒ 重试一次）。
        let frame = self.acquire_surface_texture()?;

        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("apeireth-ui-gpu.frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("apeireth-ui-gpu.frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &pipeline.bind_group, &[]);
            if let Some(vertex_buffer) = &pipeline.vertex_buffer {
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            }
            pass.draw(0..pipeline.vertex_count, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);

        // ⑤ poll 设备 ⇒ 后端异步错误在**这一帧内**派发
        // （GLES 后端不 poll ⇒ 错误永远不派发 ⇒ 失败完全静默）。
        poll_device_bounded(&self.device)?;

        // ⑥ 取设备事件：设备丢失 / 设备错误。
        let events = self.take_device_events();
        let mut lost: Vec<&DeviceEvent> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        for event in &events {
            match event {
                DeviceEvent::Error { message, .. } => errors.push(message.clone()),
                DeviceEvent::Lost { .. } => lost.push(event),
            }
        }
        if !lost.is_empty() {
            let detail = lost
                .iter()
                .map(|event| match event {
                    DeviceEvent::Lost {
                        frame,
                        reason,
                        message,
                    } => format!("第 {frame} 帧：{reason}（{message}）"),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(GpuError::DeviceLost(detail));
        }
        if !errors.is_empty() {
            return Err(GpuError::DeviceError(errors.join("; ")));
        }

        Ok(FrameReport {
            backend: self.backend,
            surface_format: self.color.surface_format,
            encoding: self.color.encoding,
            vertex_count: pipeline.vertex_count,
            uniform_members: pipeline.layout.len() as u32,
            frame_number,
        })
    }

    /// 写**引擎自填**的帧缓冲高度（`gl_FragCoord` 修正用）。
    ///
    /// ⚠ 每帧都写（不是建管线时写一次）：尺寸会变 ⇒ 高度会变 ⇒ uniform
    /// 缓冲里的那个 f32 必须跟着变 ⇒ **改尺寸不必重编译着色器**。
    fn write_framebuffer_height(&self, pipeline: &mut ScenePipeline) {
        let Some(slot) = pipeline
            .layout
            .slot_by_name(glsl::FRAMEBUFFER_HEIGHT_MEMBER)
        else {
            return;
        };
        let height = self.surface_size().1 as f32;
        let at = slot.offset as usize;
        if at + 4 > pipeline.scratch.len() {
            return;
        }
        pipeline.scratch[at..at + 4].copy_from_slice(&height.to_le_bytes());
    }

    /// 取表面纹理（`Lost` ⇒ **自动重新配置表面**并重试一次）。
    ///
    /// ## 为什么 `Lost` 要特殊处理
    ///
    /// `get_current_texture()` 的几种失败**语义完全不同**：
    ///
    /// | 取值 | 语义 | 该做什么 |
    ///|---|---|---|
    /// | `Lost` | 交换链**作废**（WebGPU 后端改 `canvas.width/height` 就会这样） | **重建表面**（重新 configure） |
    /// | `Outdated` | 表面配置**过期**（尺寸变了） | 调 [`Engine::resize`] |
    /// | `Timeout` | 本帧没拿到纹理（**瞬时**） | **跳过本帧**，下一帧再来 |
    /// | `Occluded` | 画布**不可见**（**瞬时**） | 跳过本帧 |
    /// | `Validation` | 配置非法 | 修配置 |
    ///
    /// ⇒ 早期实现把它们**全部**塌缩成 `SurfaceTextureUnavailable` ⇒ 宿主
    /// 不知道该重建、该 resize、还是该跳过 ⇒ 只能瞎试。
    ///
    /// ⇒ 现在：**逐个区分**；并且 `Lost` 会**自动**重新配置表面并重试一次
    /// （这就是「交换链失效后的恢复路径」）。
    fn acquire_surface_texture(&self) -> Result<wgpu::SurfaceTexture, GpuError> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => Ok(texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(texture),
            wgpu::CurrentSurfaceTexture::Lost => {
                // 交换链作废 ⇒ 重新配置表面 ⇒ 再试一次。
                self.reconfigure_surface()?;
                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(texture),
                    other => Err(GpuError::SurfaceTextureLost(format!(
                        "表面纹理丢失（重新配置表面后仍拿不到）：{other:?}"
                    ))),
                }
            }
            wgpu::CurrentSurfaceTexture::Outdated => Err(GpuError::SurfaceTextureOutdated(
                "表面配置已过期（画布尺寸变了）⇒ 宿主需调用 Engine::resize".to_string(),
            )),
            wgpu::CurrentSurfaceTexture::Timeout => Err(GpuError::SurfaceTextureTimeout(
                "本帧没拿到表面纹理（瞬时）⇒ 跳过本帧即可".to_string(),
            )),
            wgpu::CurrentSurfaceTexture::Occluded => Err(GpuError::SurfaceTextureOccluded(
                "画布当前不可见（瞬时）⇒ 跳过本帧即可".to_string(),
            )),
            wgpu::CurrentSurfaceTexture::Validation => Err(GpuError::SurfaceTextureUnavailable(
                "表面配置非法（wgpu 校验失败）".to_string(),
            )),
        }
    }
}

/// 按画布目标建表面（步骤 3）。
fn create_surface(
    instance: &wgpu::Instance,
    target: &SurfaceTargetInput,
) -> Result<wgpu::Surface<'static>, GpuError> {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = instance;
    match target {
        #[cfg(target_arch = "wasm32")]
        SurfaceTargetInput::HtmlCanvas(canvas) => instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| GpuError::SurfaceCreationFailed(format!("{e:?}"))),
        #[cfg(not(target_arch = "wasm32"))]
        SurfaceTargetInput::Unsupported => Err(GpuError::Unsupported(
            "本宿主没有画布目标（原生窗口路径属 ADR-0015 §2.5 的第二阶段，本批未接线）".to_string(),
        )),
    }
}

/// 把 naga IR 里的 uniform 块成员偏移与引擎自己的 std140 计算对照。
///
/// 不一致 ⇒ **显式报错**（绝不「静默写错字节」—— 那会画出错位的画面，
/// 而且从画面上很难看出是布局问题）。
///
/// ## ⭐ 成员**个数**也要对
///
/// 早期实现用 `members.iter().zip(layout.slots())` ⇒ **按较短者截断** ⇒
/// 尾部成员**根本不校验**（naga IR 多一个成员 / 少一个成员都发现不了）。
///
/// ⇒ 现在先比**个数**，个数不同直接报错；个数相同再逐个比名字与偏移。
fn verify_naga_layout(module: &naga::Module, layout: &UniformBufferLayout) -> Result<(), GpuError> {
    if layout.is_empty() {
        return Ok(());
    }
    for (_, var) in module.global_variables.iter() {
        let Some(binding) = var.binding else {
            continue;
        };
        if binding.binding != layout.binding() {
            continue;
        }
        let ty = &module.types[var.ty];
        let naga::TypeInner::Struct { members, .. } = &ty.inner else {
            continue;
        };
        if members.len() != layout.slots().len() {
            return Err(GpuError::ShaderCompilationFailed(format!(
                "uniform 块成员**个数**不一致：naga IR 里是 {} 个，引擎按 std140 算出的是 {} 个",
                members.len(),
                layout.slots().len()
            )));
        }
        for (member, slot) in members.iter().zip(layout.slots()) {
            let name = member.name.clone().unwrap_or_default();
            if name != slot.name || u64::from(member.offset) != slot.offset {
                return Err(GpuError::ShaderCompilationFailed(format!(
                    "uniform 块成员布局不一致：`{name}` 在 naga IR 里的偏移是 {} 字节，引擎按 std140 算出的是 {} 字节",
                    member.offset, slot.offset
                )));
            }
        }
    }
    Ok(())
}

/// 按后端偏好决定 **Instance 的描述**（**纯函数**，可在 native 上单测）。
///
/// - [`BackendPreference::Auto`]：`Backends::all()`；宿主不支持 WebGPU 时把
///   `BROWSER_WEBGPU` **摘掉**（⇒ 回落 WebGL）；
/// - 强制后端：**受限**的 backends 集合（绝不是 `Backends::all()`），
///   并且**先检查该后端是否编入了本产物**（否则 `Instance::new` 会 **panic**）。
///
/// ## ⭐ `Auto` 也必须守卫「无后端 feature」
///
/// wgpu 30 的 `Instance::new`（`wgpu-30.0.1/src/api/instance.rs:73-79`）在
/// 「本产物一个后端 feature 都没编入」时**直接 panic**
/// （`panic!("No wgpu backend feature … was enabled")`）⇒ 在浏览器里就是
/// **页面崩溃**。
///
/// ⚠ 早期实现只在 `WebGl` / `WebGpu` 分支检查 ⇒ **`Auto` 分支会 panic**。
///
/// ⇒ 现在三个分支**都**检查 ⇒ 返回 [`GpuError::Unsupported`]（**显式失败**，
/// 不 panic）。
fn instance_descriptor(
    preference: BackendPreference,
    webgpu_supported: bool,
) -> Result<wgpu::InstanceDescriptor, GpuError> {
    instance_descriptor_with(
        preference,
        webgpu_supported,
        wgpu::Instance::enabled_backend_features(),
    )
}

/// [`instance_descriptor`] 的**可测**版本（`enabled` 显式传入）。
///
/// ⚠ 为什么要显式传入「本产物编入了哪些后端」：
/// `wgpu::Instance::enabled_backend_features()` 是**编译期**事实（native 上
/// 本 crate 的 `webgl` / `webgpu` feature 都是 wasm-only ⇒ 集合为空）⇒ 若把
/// 它藏在函数里，**native 侧的单测就永远走不到「有后端」的分支** ⇒
/// 「Auto 摘掉 BROWSER_WEBGPU」这条逻辑**测不到**。
fn instance_descriptor_with(
    preference: BackendPreference,
    webgpu_supported: bool,
    enabled: wgpu::Backends,
) -> Result<wgpu::InstanceDescriptor, GpuError> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    match preference {
        BackendPreference::Auto => {
            if enabled.is_empty() {
                return Err(GpuError::Unsupported(
                    "本编译产物未编入任何可用的 wgpu 后端 feature（feature `webgl` / `webgpu` 未开）"
                        .to_string(),
                ));
            }
            desc.backends = wgpu::Backends::all();
            if !webgpu_supported {
                desc.backends.remove(wgpu::Backends::BROWSER_WEBGPU);
            }
            Ok(desc)
        }
        BackendPreference::WebGl => {
            if !enabled.contains(wgpu::Backends::GL) {
                return Err(GpuError::Unsupported(
                    "本编译产物未编入 WebGL 后端（feature `webgl` 未开）".to_string(),
                ));
            }
            desc.backends = wgpu::Backends::GL;
            Ok(desc)
        }
        BackendPreference::WebGpu => {
            if !enabled.contains(wgpu::Backends::BROWSER_WEBGPU) {
                return Err(GpuError::Unsupported(
                    "本编译产物未编入 WebGPU 后端（feature `webgpu` 未开）".to_string(),
                ));
            }
            if !webgpu_supported {
                // ⚠ 区分「宿主不支持」与「拿不到适配器」：后者报
                //   `AdapterUnavailable`（「拿不到适配器」）会**误导**宿主去查
                //   适配器，而真因是宿主根本没有 `navigator.gpu`（或
                //   `requestAdapter()` 解析出 null）。
                return Err(GpuError::Unsupported(
                    "宿主不支持 WebGPU（`navigator.gpu` 缺失，或 `requestAdapter()` 解析出 null）"
                        .to_string(),
                ));
            }
            desc.backends = wgpu::Backends::BROWSER_WEBGPU;
            Ok(desc)
        }
    }
}

/// 按后端偏好建 Instance（步骤 2）。
///
/// - [`BackendPreference::Auto`]：走**受限**的 backends 集合
///   （宿主不支持 WebGPU 时把 `BROWSER_WEBGPU` 摘掉）—— **禁**直接
///   `Instance::new(Backends::all())`；
/// - 强制后端：同样**受限**，并且**先检查该后端是否编入了本产物**
///   （否则 `Instance::new` 会 panic）。
///
/// ## ⚠ 为什么**不用** `wgpu::util::new_instance_with_webgpu_detection`
///
/// 它内部**会再探一次** WebGPU（`wgpu-30.0.1/src/util/init.rs:122`）⇒ 启动时
/// **两趟** `requestAdapter()` 往返 ⇒ 慢且没必要。
///
/// ⇒ 引擎自己探**一次**（`is_browser_webgpu_supported`）⇒ 把结果直接喂给
/// [`instance_descriptor`] ⇒ 只探一次。
fn build_instance(
    preference: BackendPreference,
    webgpu_supported: bool,
) -> Result<wgpu::Instance, GpuError> {
    let desc = instance_descriptor(preference, webgpu_supported)?;
    Ok(wgpu::Instance::new(desc))
}

impl GpuContext for Engine {
    fn tier(&self) -> Tier {
        Tier::Full
    }

    fn submit_frame(
        &self,
        pipeline: &mut ScenePipeline,
        uniforms: &UniformValues,
    ) -> Result<FrameReport, GpuError> {
        // **真实现**（2026-10-24 接线）：P0 的 trait 签名表达不了真实操作（见
        // `context.rs` 的「签名修订」表）⇒ 实现曾是恒 `Err(NotImplemented)` 的死路，
        // 而真实通路是**另一条并行 API**（`Engine::draw_frame`）。
        // ⇒ 现在 trait 签名与现实一致 ⇒ **声明的抽象就是实际在用的那条**：
        // 本方法就是 `draw_frame` 的委托，**不留两套 API**。
        self.draw_frame(pipeline, uniforms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 色彩策略（纯函数）
    // -----------------------------------------------------------------------

    #[test]
    fn raw_format_is_preferred_over_srgb() {
        // 反例钉住上游事实：wgpu 把 sRGB 格式排在 formats[0]
        // （wgpu-core-30.0.1/src/device/global.rs:56 的
        //  sort_by_key(|fc| !fc.format.is_srgb())）。
        let available = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm,
        ];
        let (format, encoding) = select_surface_format(&available).expect("有可选格式");
        assert_eq!(format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(encoding, OutputEncoding::RawBytes);
    }

    #[test]
    fn srgb_only_hosts_are_classified_as_srgb_encoded() {
        let available = [wgpu::TextureFormat::Rgba8UnormSrgb];
        let (format, encoding) = select_surface_format(&available).expect("有可选格式");
        assert_eq!(format, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(
            encoding,
            OutputEncoding::SrgbEncoded,
            "只有 sRGB 格式时必须显式标出「后端会做 OETF」"
        );
    }

    #[test]
    fn bgra_is_used_when_rgba_is_absent() {
        let available = [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
        ];
        let (format, encoding) = select_surface_format(&available).expect("有可选格式");
        assert_eq!(format, wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(encoding, OutputEncoding::RawBytes);
    }

    #[test]
    fn empty_format_list_has_no_selection() {
        assert_eq!(select_surface_format(&[]), None);
    }

    #[test]
    fn float_formats_are_explicitly_rejected() {
        // ⭐ fp16 是**线性**目标（浏览器呈现时补 sRGB OETF）⇒ 与 8-bit unorm
        // **不是**同一编码口径 ⇒ 不得按 RawBytes 归类。
        assert_eq!(
            select_surface_format(&[wgpu::TextureFormat::Rgba16Float]),
            None,
            "只有 fp16 时必须**显式拒绝**（而不是静默按 RawBytes 处理）"
        );
        // 但只要有 8-bit unorm，就选它（fp16 只是候选之一）。
        let mixed = [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba8Unorm,
        ];
        let (format, encoding) = select_surface_format(&mixed).expect("有可选格式");
        assert_eq!(format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(encoding, OutputEncoding::RawBytes);
    }

    #[test]
    fn a_format_without_an_encoding_contract_is_never_selected() {
        for exotic in [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureFormat::Rgba8Unorm,
        ] {
            if exotic == wgpu::TextureFormat::Rgba8Unorm {
                continue;
            }
            assert_eq!(
                select_surface_format(&[exotic]),
                None,
                "{exotic:?} 没有显式编码口径 ⇒ 不得被选中"
            );
        }
    }

    #[test]
    fn selection_is_stable_regardless_of_input_order() {
        let ordered = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm,
        ];
        let mut reversed = ordered.to_vec();
        reversed.reverse();
        assert_eq!(
            select_surface_format(&reversed),
            select_surface_format(&ordered),
            "策略不得依赖后端给出的顺序"
        );
    }

    // -----------------------------------------------------------------------
    // 后端偏好
    // -----------------------------------------------------------------------

    #[test]
    fn backend_preference_flags_are_explicit() {
        assert!(BackendPreference::WebGpu.requires_webgpu());
        assert!(!BackendPreference::Auto.requires_webgpu());
        assert!(!BackendPreference::WebGl.requires_webgpu());
    }

    #[test]
    fn default_options_are_neutral() {
        let options = EngineOptions::default();
        assert_eq!(options.backend, BackendPreference::Auto);
        assert!(options.width > 0 && options.height > 0);
    }

    // -----------------------------------------------------------------------
    // 绘制请求描述
    // -----------------------------------------------------------------------

    #[test]
    fn vertex_count_defaults_to_the_vertex_data_length() {
        let scene = SceneDescriptor::new("vs", "fs").with_vertex_data(vec![0.0; 12]);
        assert_eq!(scene.resolved_vertex_count(), 6);
    }

    #[test]
    fn explicit_vertex_count_wins() {
        let scene = SceneDescriptor::new("vs", "fs")
            .with_vertex_data(vec![0.0; 12])
            .with_vertex_count(3);
        assert_eq!(scene.resolved_vertex_count(), 3);
    }

    #[test]
    fn empty_scene_has_zero_vertices() {
        let scene = SceneDescriptor::new("vs", "fs");
        assert_eq!(scene.resolved_vertex_count(), 0);
    }

    // -----------------------------------------------------------------------
    // 后端选择（守卫：不得 panic）
    // -----------------------------------------------------------------------

    #[test]
    fn auto_backend_refuses_to_build_an_instance_without_any_backend_feature() {
        // ⭐ wgpu 30 的 `Instance::new`（`api/instance.rs:73-79`）在「本产物
        // 一个后端 feature 都没编入」时 **panic**
        // （`panic!("No wgpu backend feature … was enabled")`）⇒ 在浏览器里
        // 就是**页面崩溃**。
        //
        // ⇒ `Auto` 分支必须**先**守卫 ⇒ 返回 `Err(Unsupported)`。
        let err = instance_descriptor_with(BackendPreference::Auto, false, wgpu::Backends::empty())
            .expect_err("没有后端 feature 时 Auto 必须显式报错（不得 panic）");
        assert!(
            matches!(err, GpuError::Unsupported(_)),
            "必须是 Unsupported（显式失败）：{err:?}"
        );
    }

    #[test]
    fn auto_backend_drops_webgpu_when_the_host_lacks_it() {
        // 宿主不支持 WebGPU ⇒ Auto 必须把 BROWSER_WEBGPU 摘掉（回落 WebGL）。
        let desc = instance_descriptor_with(BackendPreference::Auto, false, wgpu::Backends::all())
            .expect("有后端 feature 时 Auto 必须给出描述");
        assert!(
            !desc.backends.contains(wgpu::Backends::BROWSER_WEBGPU),
            "宿主不支持 WebGPU 时必须摘掉 BROWSER_WEBGPU"
        );
        assert!(
            desc.backends.contains(wgpu::Backends::GL),
            "宿主不支持 WebGPU 时必须仍保留 WebGL（回落）"
        );

        let desc = instance_descriptor_with(BackendPreference::Auto, true, wgpu::Backends::all())
            .expect("有后端 feature 时 Auto 必须给出描述");
        assert!(
            desc.backends.contains(wgpu::Backends::BROWSER_WEBGPU),
            "宿主支持 WebGPU 时必须保留 BROWSER_WEBGPU"
        );
    }

    #[test]
    fn forced_webgpu_reports_the_missing_host_capability_distinctly() {
        // ⚠ 区分「宿主不支持 WebGPU」与「拿不到适配器」：后者报
        //   `AdapterUnavailable`（「拿不到适配器」）会**误导**宿主去查适配器。
        let err = instance_descriptor_with(
            BackendPreference::WebGpu,
            false,
            wgpu::Backends::BROWSER_WEBGPU,
        )
        .expect_err("宿主不支持 WebGPU 时必须显式报错");
        match &err {
            GpuError::Unsupported(detail) => {
                assert!(
                    detail.contains("宿主不支持 WebGPU"),
                    "错误文本必须点明「宿主不支持」：{detail}"
                );
            }
            other => panic!("必须是 Unsupported：{other:?}"),
        }
    }

    #[test]
    fn forced_backends_report_a_missing_compile_time_feature() {
        let err =
            instance_descriptor_with(BackendPreference::WebGl, false, wgpu::Backends::empty())
                .expect_err("没编入 WebGL 后端时必须显式报错");
        assert!(matches!(err, GpuError::Unsupported(_)), "{err:?}");

        let err =
            instance_descriptor_with(BackendPreference::WebGpu, true, wgpu::Backends::empty())
                .expect_err("没编入 WebGPU 后端时必须显式报错");
        assert!(matches!(err, GpuError::Unsupported(_)), "{err:?}");
    }

    #[test]
    fn the_real_instance_descriptor_never_panics_on_this_build() {
        // ⭐ 真实入口（用**本产物**编译期事实）：不得 panic。
        let result = instance_descriptor(BackendPreference::Auto, false);
        if wgpu::Instance::enabled_backend_features().is_empty() {
            assert!(
                result.is_err(),
                "本产物没有后端 feature ⇒ Auto 必须报错而不是 panic"
            );
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }

    // -----------------------------------------------------------------------
    // naga IR ↔ 引擎布局的**双向**交叉校验
    // -----------------------------------------------------------------------

    #[test]
    fn naga_layout_check_rejects_a_member_count_mismatch() {
        // ⭐ 早期实现用 `zip` ⇒ **按较短者截断** ⇒ 尾部成员**根本不校验**。
        let members = glsl::fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);
        let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");

        // 少一个成员 ⇒ 必须报错。
        let mut truncated = members.clone();
        truncated.pop();
        let short_layout = UniformBufferLayout::from_members(&truncated, 0).expect("布局可建");
        let module = glsl::parse(
            &glsl::transform_fragment_with_block_binding(
                crate::shaders::SCENE_FRAGMENT_GLSL,
                glsl::FRAGMENT_UNIFORM_BLOCK_BINDING,
            )
            .expect("变换成功"),
            naga::ShaderStage::Fragment,
        )
        .expect("解析成功");
        let err =
            verify_naga_layout(&module, &short_layout).expect_err("成员个数不一致必须显式报错");
        assert!(
            matches!(err, GpuError::ShaderCompilationFailed(_)),
            "{err:?}"
        );
        // 完整布局必须通过。
        assert!(verify_naga_layout(&module, &layout).is_ok());
    }

    // -----------------------------------------------------------------------
    // 设备事件（设备丢失可检测）
    // -----------------------------------------------------------------------

    #[test]
    fn device_events_carry_a_frame_number() {
        let lost = DeviceEvent::Lost {
            frame: 42,
            reason: "Destroyed".to_string(),
            message: "设备被销毁".to_string(),
        };
        assert!(matches!(lost, DeviceEvent::Lost { frame: 42, .. }));
        let error = DeviceEvent::Error {
            frame: 7,
            message: "校验失败".to_string(),
        };
        assert!(matches!(error, DeviceEvent::Error { frame: 7, .. }));
    }

    #[test]
    fn device_lost_carries_its_reason() {
        let err = GpuError::DeviceLost("第 12 帧：Destroyed（设备被销毁）".to_string());
        let text = err.to_string();
        assert!(text.contains("DeviceLost"), "{text}");
        assert!(
            text.contains("Destroyed"),
            "reason 必须能拿到宿主手里：{text}"
        );
    }

    #[test]
    fn surface_texture_failures_have_distinct_variants() {
        // ⚠ `Lost`（需重建表面）/ `Outdated`（需 resize）/ `Timeout`（瞬时）
        //   的**处置方式完全不同** ⇒ 不得塌缩成同一个错误。
        let lost = GpuError::SurfaceTextureLost("交换链作废".to_string());
        let outdated = GpuError::SurfaceTextureOutdated("尺寸变了".to_string());
        let timeout = GpuError::SurfaceTextureTimeout("瞬时".to_string());
        assert_ne!(lost, outdated);
        assert_ne!(outdated, timeout);
        assert!(lost.to_string().contains("需重建表面"), "{lost}");
        assert!(outdated.to_string().contains("需 resize"), "{outdated}");
        assert!(timeout.to_string().contains("跳过本帧"), "{timeout}");
    }
}
