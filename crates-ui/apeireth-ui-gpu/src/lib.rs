//! # apeireth-ui-gpu — GPU 桥（状态 → 像素）
//!
//! 与 `ui-dom` **平级**的第二个投影（不是父子）：DOM 显影与 GPU 显影是同一份
//! 状态的**两条正交出路**（ADR-0015 §3.3；与 `00-PHILOSOPHY.md` §2「四个投影
//! 显影同一份状态」一致）。
//!
//! 职责边界（设计文档 §3.3.4）：只回答「**状态 → 像素**」，**不改着色器数学**
//! —— 电影感载体（GLSL / WGSL / CSS 合成）全在引擎之外，引擎一行都不碰。
//!
//! ## 本批（P5 · GPU 桥能力层）交付口径
//!
//! ### ✅ 已实现
//!
//! - **GLSL ES 3.0 → naga IR 前置变换器**（[`glsl`]）：三步机械变换
//!   （删 `#version 300 es` 行 + 把 N 个 plain uniform **合并进一个 std140
//!   uniform 块** + 必要时补一个 16 字节对齐的 padding 成员），纯文本变换
//!   ⇒ **native 可单测**，不需 GPU；**不触碰着色器数学**。
//!   - 为什么必须合并成一块：naga 给每个 uniform 一个独立 binding ⇒ 16 个
//!     uniform = 16 个 uniform buffer，而 **WebGL2 后端每个着色阶段只有
//!     12–14 个**（实测本机 14，GLES3 规范下限 12）⇒「逐个注入 binding」
//!     在 WebGL 后端**必然失败**（实测报错原文见 `crates-ui/evidence/p5-gpu/`）。
//! - **uniform 名 → 索引映射**（[`bindings`]）：naga 会抹掉 uniform 名 ⇒
//!   引擎自维护映射表，按 **索引**（= 声明序 = 块内成员序）绑定；表由单测
//!   钉住「顺序 ⇒ 索引」与「表长度 = 着色器 uniform 声明数」。
//! - **双后端引擎入口**（[`engine`]）：`Engine::create(canvas)` 严格走 5 步
//!   （探测 → 建 Instance → 建表面 → 带 `compatible_surface` 申请 adapter →
//!   `required_limits = adapter.limits()` 申请设备），后端在碰画布之前定死。
//! - **显式色彩契约**（[`engine::select_surface_format`]）：wgpu 默认把 sRGB
//!   表面格式排在 `formats[0]` ⇒ 直接取会让 WebGL 与 WebGPU 两个后端
//!   **画出不同像素**（实测同一片元值 0.35 → 字节 160 vs 89）；引擎**显式
//!   改选非 sRGB 格式**并把实际口径记进 [`engine::ColorProfile`]。
//! - **设备错误不 panic**（[`engine::Engine::take_device_errors`]）：wgpu 的
//!   默认未捕获错误处理器在浏览器里会 panic ⇒ 引擎改装自己的处理器，把
//!   校验错误**收进队列**并以 [`GpuError`] 返回。
//! - **着色器资产入库**（[`shaders`]）：生产版 GLSL 的去注释副本（光学数值
//!   逐字未改），供上面的变换器与管线消费。
//! - **片元坐标语义修正**（[`glsl::fix_frag_coord_semantics`]）：GLSL ES 规定
//!   `gl_FragCoord` 的 y 轴**向上**，而本引擎的两个后端都把它变成**向下**
//!   ⇒ 不修正的话同一份 GLSL 源画出来的画面是**上下颠倒**的。变换器在**源级**
//!   把 `gl_FragCoord` 换成 `vec4(x, 帧缓冲高 - y, z, w)`。
//!   - ⭐ 翻转高度来自 **uniform**（[`glsl::FRAMEBUFFER_HEIGHT_MEMBER`]），不是
//!     **烘焙字面量** ⇒ **改尺寸不必重编译着色器**（生产 uniform 的偏移与取值
//!     逐字节不变 ⇒ 逐像素证据不受影响；等价性论证见该函数的文档）。
//!
//! ### ✅ 生命周期与可观测性（P5-c 重构批，2026-10-25）
//!
//! - ⭐ **resize 通路**（[`engine::Engine::resize`]）：改尺寸 ⇒ **重新配置
//!   表面** + 更新配置。早期实现**只在 `create` 里**配置一次表面 ⇒ 尺寸变了
//!   以后场景层按新尺寸算几何、GPU 按旧 config 出图（拉伸/裁切），翻转高度
//!   陈旧（竖直错位），WebGPU 后端改 `canvas.width/height` 还会**作废交换链**
//!   ⇒ `get_current_texture()` 永久 `Lost` ⇒ **没有重建入口就永远画不出来**。
//! - ⭐⭐ **尺寸自侦测**（[`engine::CanvasSizeSource`] +
//!   [`engine::EngineOptions::canvas_size`]）：引擎**不持有画布句柄** ⇒ 它
//!   自己**看不到** `canvas.width/height` 变没变 ⇒ ⇒ 若只有「宿主显式调
//!   [`engine::Engine::resize`]」一条路 ⇒ **与生产行为不一致**（生产是
//!   Svelte + `<canvas>`：改 `canvas.width/height` 就完事，**什么都不调**）。
//!   ⇒ 宿主把「读画布尺寸」的闭包交给引擎 ⇒ 引擎**每帧**比对 ⇒ 不一致就
//!   **自动** resize ⇒ ⇒ **行为与生产一致**。
//!   - ⚠ 上游事实：wgpu 的两个后端在 `configure()` 里都会**改写
//!     `canvas.width/height`**（`wgpu-30.0.1/src/backend/webgpu.rs:4213-4215`、
//!     `wgpu-hal-30.0.1/src/gles/web.rs:357-358`）⇒ ⇒ 引擎的 config 尺寸
//!     **必须**与画布一致 ⇒ ⇒ 自侦测也是「**不让引擎与宿主抢画布尺寸**」。
//! - ⭐ **表面重建路径**（[`engine::Engine::reconfigure_surface`]）：交换链
//!   失效时重新配置表面 ⇒ 恢复；[`engine::Engine::draw_frame`] 检出 `Lost` 时
//!   **自动**重建并重试一次。
//! - ⭐ **设备丢失可检测**（[`engine::DeviceEvent::Lost`]）：注册 wgpu 30 的
//!   `Device::set_device_lost_callback` ⇒ 丢失原因带**帧号**进队列 ⇒
//!   `draw_frame` 返回 [`GpuError::DeviceLost`]（宿主据此决定重建）。
//! - ⭐ **每帧 poll 设备**：wgpu 30 的 `Device::poll` 在 **GLES 后端不是 no-op**
//!   ⇒ 不 poll ⇒ 后端错误**永远不派发** ⇒ 失败完全静默。事件带**帧号**，
//!   且宿主可用 [`engine::Engine::poll_device`] 在**不画帧**时取走积压事件。
//! - ⭐ **表面纹理错误逐个区分**：`Lost`（重建表面）/ `Outdated`（resize）/
//!   `Timeout`、`Occluded`（瞬时，跳过本帧）各有**专属**错误变体。
//! - ⭐ **后端选择不 panic**：`Auto` 分支也守卫「本产物没编入任何后端
//!   feature」⇒ 返回 [`GpuError::Unsupported`]（wgpu 的 `Instance::new` 在
//!   那种情况下会 **panic** ⇒ 浏览器里就是**页面崩溃**）。
//! - ⭐ **启动只探一次 WebGPU**（早期实现走
//!   `new_instance_with_webgpu_detection` ⇒ 内部**再探一次** ⇒ 两趟
//!   `requestAdapter()` 往返）。
//! - ⭐ **管线只建一次**（[`engine::Engine::create_scene_pipeline`] 的文档 +
//!   [`engine::Engine::pipeline_creation_count`] 计数 + 显式
//!   [`engine::Engine::recreate_pipeline`]）：管线创建是**重资源**操作（实测
//!   `Page.captureScreenshot` **超时**）。
//!
//! ### ✅ 与生产画面一致（P5-b 实测）
//!
//! - 生产版的场景类（**宿主侧**）在真实浏览器里跑出来的画面，与本引擎用
//!   **同一批 uniform 取值**画出来的画面**逐像素完全一致**：两个后端各
//!   **0 / 576,000** 像素不同（差分阈值：任一通道绝对差 > 2/255）。
//!   证据见 `crates-ui/evidence/p5-b/`（含抓到的 16 个 uniform 原值、
//!   三张画布、逐像素判据与控制台日志）。
//! - ⚠ **本批（2026-10-25 重构）改了「翻转高度」的来源**（字面量 → uniform）
//!   ⇒ 该证据**是在改动前**抓的 ⇒ **本批未重新抓取**（浏览器 harness 不在本批
//!   范围内）。
//!   - **等价性论证**：`600.0`（GLSL float 字面量）与 uniform 里的 f32
//!     `600.0` 是**同一个 f32 值** ⇒ `highp - highp` 的 IEEE 减法**逐位相同**
//!     ⇒ 像素不变；且引擎成员**追加在 uniform 块末尾** ⇒ 生产 uniform 的
//!     **偏移与取值逐字节不变**（块大小仍是 96 字节）。
//!   - ⇒ 结论：**该证据按构造仍然成立**，但**未在本批重新实测**（诚实标注：
//!     未经重新抓取的证据不能算「本批实测」）。
//!
//! ### ✅ resize 自侦测**已在浏览器实测**（2026-10-25）
//!
//! - 探针把引擎页的画布从 **960×600 改成 720×600**（**比例不同** ⇒ 构图
//!   变化可测）⇒ 宿主**没有**调任何 resize 接口（引擎页里根本没有这种接口）
//!   ⇒ 引擎**自己**侦测到并 resize：
//!   - 两个后端的表面尺寸都从 `960×600` 变成 `720×600`；
//!   - 表面**代号** `0 → 1`（⇒ 表面确实被重新配置过 ⇒ 交换链重建 ⇒
//!     WebGPU 后端「改画布尺寸 ⇒ 交换链作废」的恢复路径**实测**走通）；
//!   - **自动 resize 计数** `0 → 1`（⇒ 走的是**自动**路径，不是宿主显式调用）。
//! - 逐像素判据：生产画面**裁剪**左侧 720 列 vs 引擎 resize 后的实拍 ⇒
//!   **0 / 432,000** 像素不同（WebGL 与 WebGPU **都是 0**）⇒ ⇒ 新尺寸**真的
//!   生效**，且画面是**裁剪**（不是**拉伸**）。
//!   - **反事实**（判据灵敏性）：把生产画面**缩放**到 720×600 再比 ⇒
//!     **208,508 / 432,000 = 48.27%** 像素不同 ⇒ ⇒ 0 差异不是因为「怎么比
//!     都一样」。
//! - ⚠ **本批未把重新生成的证据入库**（批次纪律：证据目录只改 harness 源码）
//!   ⇒ ⇒ 上面的数字来自**本批实测**（Chromium 154 + SwiftShader），但
//!   **对应的 PNG / report.json 不在版本库里**。
//!
//! ### 🔵 未接线 / 未做
//!
//! - **uniform 取值来源仍未接线（按设计如此）**：引擎**不知道**场景语义
//!   （构图比例、机位、presence 档位、时间线照明 …）⇒ 取值必须由**宿主**喂。
//!   本批证明的是「**同一批取值 ⇒ 同一张画面**」这条管道，而不是「引擎自己
//!   算得出取值」。
//! - **未在 Tauri WebView 内验证**：本批只在 Linux 无头 Chromium 154 上实测；
//!   macOS WKWebView / Windows WebView2 / Linux WebKitGTK **均未实测**。
//! - **顶点阶段的 uniform 未接线**：[`engine::Engine::create_scene_pipeline`]
//!   对「顶点阶段有 uniform」的着色器**显式报错**（不静默忽略：静默忽略会让
//!   顶点阶段的 uniform 永远拿到 0 值，画出错位的画面）。
//! - **P0 桩仍在**（[`UnavailableGpuContext`]）：显式返回
//!   [`GpuError::NotImplemented`]，不假装「场景已经在跑」。
//! - ✅ **[`GpuContext::submit_frame`] 已接线**（2026-10-24）：P0 的 trait 签名
//!   （`&SceneUniforms` / `&self` / `Result<(), _>`）**表达不了真实操作** ⇒ 实现曾是
//!   恒 `Err(NotImplemented)` 的死路，而真实通路是**另一条并行 API**
//!   （[`engine::Engine::draw_frame`]）⇒ 现已把签名改成与现实一致
//!   （`&mut ScenePipeline` + [`bindings::UniformValues`] + 返回
//!   [`engine::FrameReport`]），**声明的抽象就是实际在用的那条**。
//! - 🔵 **「物理量 → 16 个着色器 uniform」的映射属宿主侧**（引擎按设计不碰：
//!   那一步依赖相机 / 主题 / 存在态等语义，引擎里不出现业务名词）⇒ 引擎只提供
//!   **管道**（[`bindings::UniformValues`] 按名字写），取值由宿主喂。
//! - **native 侧不是画面验证**：native 单测只覆盖**纯逻辑**（变换 / 映射 /
//!   布局 / 策略 / 坐标语义）；画面级验证只在真实浏览器里做（证据见
//!   `crates-ui/evidence/p5-gpu/` 与 `crates-ui/evidence/p5-b/`）。

#![forbid(unsafe_code)]

pub mod bindings;
pub mod context;
pub mod engine;
pub mod glsl;
pub mod scene;
pub mod shaders;

pub use bindings::{
    binding_of, binding_table, name_of, BindingError, UniformBufferLayout, UniformSlot,
    UniformValues, SHADER_UNIFORM_NAMES,
};
pub use context::{GpuContext, GpuError, Tier, UnavailableGpuContext};
pub use engine::{
    select_surface_format, BackendPreference, BlendMode, CanvasSizeSource, ColorProfile,
    DeviceEvent, Engine, EngineOptions, EngineReport, FrameReport, OutputEncoding, SceneDescriptor,
    ScenePass, ScenePipeline, SurfaceTargetInput, Topology, VertexFormat,
};
pub use glsl::{GlslError, UniformDecl};
pub use scene::{
    BlackHoleParams, SceneError, SceneUniforms, GRAVITATIONAL_CONSTANT, SOLAR_MASS, SPEED_OF_LIGHT,
};
pub use shaders::{
    INTRO_BACKDROP_FRAGMENT_GLSL, INTRO_BACKDROP_UNIFORM_NAMES, INTRO_BACKDROP_VERTEX_GLSL,
    INTRO_SPRITE_COORD_FRAGMENT_GLSL, INTRO_SPRITE_FLAT_FRAGMENT_GLSL, INTRO_SPRITE_FRAGMENT_GLSL,
    INTRO_SPRITE_PROBE_FRAGMENT_GLSL, INTRO_SPRITE_UNIFORM_NAMES, INTRO_SPRITE_VERTEX_GLSL,
    SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL,
};

#[cfg(test)]
mod tests {
    //! crate 级断言：跨模块组合行为（着色器源 → 变换 → 映射表 → uniform 布局）。

    use super::*;

    #[test]
    fn scene_params_flow_into_uniforms_and_submission() {
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let uniforms = params.to_uniforms(2.0e7, 1.25);

        // 桩的档位（其**运行时**报错断言在浏览器侧，见 `context.rs` 的说明）。
        let context = UnavailableGpuContext;
        assert_eq!(context.tier(), Tier::Software);
        assert!(uniforms.disk_outer_radius_km > uniforms.disk_inner_radius_km);
    }

    #[test]
    fn shader_assets_drive_the_binding_table_end_to_end() {
        // 着色器源 → uniform 声明 → 映射表 → 缓冲布局：一条链打通。
        let decls = glsl::uniform_declarations(SCENE_FRAGMENT_GLSL);
        assert_eq!(decls.len(), SHADER_UNIFORM_NAMES.len());

        let members = glsl::uniform_block_members(&decls);
        let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");
        assert_eq!(layout.len(), 17, "16 个 uniform + 1 个 padding 成员");
        for (index, slot) in layout
            .slots()
            .iter()
            .take(SHADER_UNIFORM_NAMES.len())
            .enumerate()
        {
            assert_eq!(slot.index, index);
            assert_eq!(slot.name, SHADER_UNIFORM_NAMES[index]);
            assert_eq!(binding_of(&slot.name), Some(index as u32));
        }
    }

    #[test]
    fn uniform_values_round_trip_through_the_layout() {
        let members = glsl::uniform_block_members(&glsl::uniform_declarations(SCENE_FRAGMENT_GLSL));
        let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");

        let mut values = UniformValues::new();
        values.set(0, &[320.0, 240.0]); // uRes
        values.set(1, &[0.0, 0.0]); // uCenter
        for binding in 2..16 {
            values.set(binding, &[0.5]);
        }

        let mut bytes = vec![0u8; layout.total_size() as usize];
        values.write_into(&layout, &mut bytes).expect("写入成功");
        assert!(bytes.iter().any(|byte| *byte != 0), "缓冲里必须真有数据");
    }

    #[test]
    fn color_policy_prefers_raw_formats_for_backend_parity() {
        // 两个后端要画出同一张图，表面格式的编码方式必须一致 ⇒ 策略固定。
        let webgl_like = [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm,
        ];
        let webgpu_like = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ];
        let (_, left) = select_surface_format(&webgl_like).expect("可选");
        let (_, right) = select_surface_format(&webgpu_like).expect("可选");
        assert_eq!(
            left, right,
            "两个后端必须落到同一种编码方式，否则画面不一致"
        );
        assert_eq!(left, OutputEncoding::RawBytes);
    }
}
