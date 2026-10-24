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
//! 2. 建 Instance
//!      ← 走 wgpu::util::new_instance_with_webgpu_detection；
//!        **禁** Instance::new(Backends::all())
//!        （实测：宿主有 navigator.gpu 但拿不到 adapter 时，
//!          Instance::new(Backends::all()) 会走进 NotFound 死路，
//!          且 wgpu 在 request_adapter 阶段**没有任何回落**）
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
use std::sync::{Arc, Mutex};

use wgpu::util::DeviceExt;

use crate::bindings::{BindingError, UniformBufferLayout, UniformValues};
use crate::context::{GpuContext, GpuError, Tier};
use crate::glsl::{self, GlslError};

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
/// `Rgba8Unorm` → `Bgra8Unorm` → `Rgba16Float`，
/// 之后才轮到 sRGB 格式；全都没有时退回列表首个并按其是否 sRGB 归类。
pub fn select_surface_format(
    available: &[wgpu::TextureFormat],
) -> Option<(wgpu::TextureFormat, OutputEncoding)> {
    const RAW_PREFERENCE: [wgpu::TextureFormat; 3] = [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba16Float,
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
    available.first().copied().map(|format| {
        let encoding = if format.is_srgb() {
            OutputEncoding::SrgbEncoded
        } else {
            OutputEncoding::RawBytes
        };
        (format, encoding)
    })
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
}

/// 双后端引擎：实例 / 适配器 / 设备 / 表面 / 队列。
pub struct Engine {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
    width: u32,
    height: u32,
    device_errors: Arc<Mutex<Vec<String>>>,
}

impl Engine {
    /// 按 5 步流程建引擎（顺序不可换，见模块文档）。
    ///
    /// # Errors
    ///
    /// - 宿主没有画布目标 → [`GpuError::Unsupported`]；
    /// - 请求的后端在**本编译产物里没编入** → [`GpuError::Unsupported`]；
    /// - 建表面失败 → [`GpuError::SurfaceCreationFailed`]；
    /// - 拿不到适配器 → [`GpuError::AdapterUnavailable`]；
    /// - 拿不到设备 → [`GpuError::DeviceRequestFailed`]。
    pub async fn create(
        target: SurfaceTargetInput,
        options: EngineOptions,
    ) -> Result<Self, GpuError> {
        // ── 步骤 1：宿主能力探测（判据 = 可用的 WebGPU adapter）────────────
        let webgpu_supported = wgpu::util::is_browser_webgpu_supported().await;

        // ── 步骤 2：定死后端 → 建 Instance（禁 Instance::new(Backends::all())）──
        let instance = build_instance(options.backend, webgpu_supported).await?;

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

        // 设备错误不 panic：收进队列，由调用方以 `GpuError` 拿回。
        let device_errors = Arc::new(Mutex::new(Vec::new()));
        {
            let sink = Arc::clone(&device_errors);
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                if let Ok(mut messages) = sink.lock() {
                    messages.push(error.to_string());
                }
            }));
        }

        let info = adapter.get_info();
        let caps = surface.get_capabilities(&adapter);
        let available_formats = caps.formats.clone();
        let (surface_format, encoding) = select_surface_format(&available_formats)
            .ok_or_else(|| GpuError::Unsupported("宿主没有提供任何可用的表面格式".to_string()))?;

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
            config,
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
            width,
            height,
            device_errors,
        })
    }

    /// 引擎的对外报告（宿主可读的实测事实）。
    pub fn report(&self) -> EngineReport {
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
            width: self.width,
            height: self.height,
        }
    }

    /// 色彩档案。
    pub fn color_profile(&self) -> &ColorProfile {
        &self.color
    }

    /// 取走（并清空）已收集的设备错误。
    ///
    /// wgpu 的默认处理器会 panic；引擎改成收集 ⇒ 宿主能拿到**具体**的
    /// 校验错误文本，而不是一句 JS 异常。
    pub fn take_device_errors(&self) -> Vec<String> {
        match self.device_errors.lock() {
            Ok(mut messages) => std::mem::take(&mut *messages),
            Err(_) => Vec::new(),
        }
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
    /// # Errors
    ///
    /// - GLSL 解析 / 校验失败 → [`GpuError::ShaderCompilationFailed`]；
    /// - uniform 类型不受支持 → [`GpuError::ShaderCompilationFailed`]；
    /// - 管线 / 绑定组创建失败 → [`GpuError::PipelineCreationFailed`]。
    pub fn create_scene_pipeline(
        &self,
        scene: &SceneDescriptor,
    ) -> Result<ScenePipeline, GpuError> {
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
            self.config.height,
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

        let fragment_members =
            glsl::uniform_block_members(&glsl::uniform_declarations(&scene.fragment_glsl));
        let layout = UniformBufferLayout::from_members(
            &fragment_members,
            glsl::FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .map_err(|e: GlslError| GpuError::ShaderCompilationFailed(e.to_string()))?;

        // 与 naga IR 里的成员偏移对照 —— 不一致就**显式报错**
        // （绝不「静默写错字节」）。
        verify_naga_layout(&fragment_module_ir, &layout)?;

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
                        format: self.config.format,
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
        })
    }

    /// 画一帧：uniform → 缓冲 → 渲染 → 呈现。
    ///
    /// # Errors
    ///
    /// - uniform 值与布局不符 → [`GpuError::BindingFailed`]；
    /// - 拿不到表面纹理 → [`GpuError::SurfaceTextureUnavailable`]。
    pub fn draw_frame(
        &self,
        pipeline: &mut ScenePipeline,
        uniforms: &UniformValues,
    ) -> Result<FrameReport, GpuError> {
        uniforms
            .write_into(&pipeline.layout, &mut pipeline.scratch)
            .map_err(|e: BindingError| GpuError::BindingFailed(e.to_string()))?;
        if let Some(buffer) = &pipeline.uniform_buffer {
            if !pipeline.scratch.is_empty() {
                self.queue.write_buffer(buffer, 0, &pipeline.scratch);
            }
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => {
                return Err(GpuError::SurfaceTextureUnavailable(format!("{other:?}")));
            }
        };
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

        let errors = self.take_device_errors();
        if !errors.is_empty() {
            return Err(GpuError::DeviceError(errors.join("; ")));
        }

        Ok(FrameReport {
            backend: self.backend,
            surface_format: self.color.surface_format,
            encoding: self.color.encoding,
            vertex_count: pipeline.vertex_count,
            uniform_members: pipeline.layout.len() as u32,
        })
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

/// 按后端偏好建 Instance（步骤 2）。
///
/// - [`BackendPreference::Auto`]：走
///   `wgpu::util::new_instance_with_webgpu_detection`（探测不通过时由它
///   把 `BROWSER_WEBGPU` 从描述里摘掉）—— **禁**直接
///   `Instance::new(Backends::all())`；
/// - 强制后端：`Instance::new` + **受限**的 backends 集合（绝不是
///   `Backends::all()`），并且**先检查该后端是否编入了本产物**（否则
///   `Instance::new` 会 panic）。
async fn build_instance(
    preference: BackendPreference,
    webgpu_supported: bool,
) -> Result<wgpu::Instance, GpuError> {
    let enabled = wgpu::Instance::enabled_backend_features();
    match preference {
        BackendPreference::Auto => {
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgpu::Backends::all();
            if !webgpu_supported {
                desc.backends.remove(wgpu::Backends::BROWSER_WEBGPU);
            }
            Ok(wgpu::util::new_instance_with_webgpu_detection(desc).await)
        }
        BackendPreference::WebGl => {
            if !enabled.contains(wgpu::Backends::GL) {
                return Err(GpuError::Unsupported(
                    "本编译产物未编入 WebGL 后端（feature `webgl` 未开）".to_string(),
                ));
            }
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgpu::Backends::GL;
            Ok(wgpu::Instance::new(desc))
        }
        BackendPreference::WebGpu => {
            if !enabled.contains(wgpu::Backends::BROWSER_WEBGPU) {
                return Err(GpuError::Unsupported(
                    "本编译产物未编入 WebGPU 后端（feature `webgpu` 未开）".to_string(),
                ));
            }
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgpu::Backends::BROWSER_WEBGPU;
            Ok(wgpu::Instance::new(desc))
        }
    }
}

impl GpuContext for Engine {
    fn tier(&self) -> Tier {
        Tier::Full
    }

    fn submit_frame(&self, _uniforms: &crate::scene::SceneUniforms) -> Result<(), GpuError> {
        // **未接线**：把 P0 的 `SceneUniforms`（几何量）映射到 16 个着色器
        // uniform 属场景语义（P5-b）。本批的真实通路是
        // `Engine::draw_frame(pipeline, uniforms)`。
        Err(GpuError::NotImplemented)
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
    fn unknown_formats_fall_back_to_the_first_entry() {
        let available = [wgpu::TextureFormat::Rgba16Float];
        let (format, encoding) = select_surface_format(&available).expect("有可选格式");
        assert_eq!(format, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(encoding, OutputEncoding::RawBytes);
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
}
