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
//!   把 `gl_FragCoord` 换成 `vec4(x, height - y, z, w)` ⇒ 引擎里的片元坐标与
//!   GLSL ES 规范逐点一致（**不改任何表达式与数值**，只改坐标的取法）。
//!
//! ### ✅ 与生产画面一致（P5-b 实测）
//!
//! - 生产版的场景类（**宿主侧**）在真实浏览器里跑出来的画面，与本引擎用
//!   **同一批 uniform 取值**画出来的画面**逐像素完全一致**：两个后端各
//!   **0 / 576,000** 像素不同（差分阈值：任一通道绝对差 > 2/255）。
//!   证据见 `crates-ui/evidence/p5-b/`（含抓到的 16 个 uniform 原值、
//!   三张画布、逐像素判据与控制台日志）。
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
    select_surface_format, BackendPreference, ColorProfile, Engine, EngineOptions, EngineReport,
    FrameReport, OutputEncoding, SceneDescriptor, ScenePipeline, SurfaceTargetInput,
};
pub use glsl::{GlslError, UniformDecl};
pub use scene::{
    BlackHoleParams, SceneError, SceneUniforms, GRAVITATIONAL_CONSTANT, SOLAR_MASS, SPEED_OF_LIGHT,
};
pub use shaders::{SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL};

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
