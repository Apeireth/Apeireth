//! GPU 后端抽象：状态 → 像素的唯一通道。
//!
//! 职责边界（设计文档 §3.3.4）：只回答「**状态 → 像素**」，与 `ui-dom`
//! **平级**（两条正交投影，不是父子）。**不改着色器数学**。

use crate::bindings::UniformValues;
use crate::engine::{FrameReport, ScenePipeline};

/// GPU 能力档位（降级梯队）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// 无 GPU 能力（场景整体退化为静态背景）。
    Software,
    /// 仅计算着色器档（能跑物理、不能出画面）。
    Compute,
    /// 完整渲染档。
    Full,
}

impl Tier {
    /// 档位是否支持出画面。
    pub fn can_render(&self) -> bool {
        matches!(self, Tier::Full)
    }
}

/// GPU 桥错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuError {
    /// **未接线**：该通路在本批尚未接线（例如把 P0 的 `SceneUniforms`
    /// 映射到着色器 uniform 属 P5-b 的场景语义）。
    NotImplemented,
    /// **设备丢失**（需重建上下文）。
    ///
    /// ⚠ 带**原因文本**（wgpu 的 `DeviceLostReason` + 后端说明 + 帧号）：
    /// 宿主据此决定「重建整个上下文」还是「偶发错误」。
    ///
    /// 来源：引擎注册的 `Device::set_device_lost_callback`
    /// （`wgpu-30.0.1/src/api/device.rs:681`）⇒ 设备丢失**可检测**。
    DeviceLost(String),
    /// 不支持的能力。
    Unsupported(String),
    /// 建表面失败（典型原因：画布已被另一种 context 占用）。
    SurfaceCreationFailed(String),
    /// 拿不到适配器（宿主没有满足条件的 GPU 适配器）。
    AdapterUnavailable(String),
    /// 拿不到设备（限额不满足等）。
    DeviceRequestFailed(String),
    /// 着色器编译 / 校验失败。
    ShaderCompilationFailed(String),
    /// 管线 / 绑定组创建失败。
    PipelineCreationFailed(String),
    /// ⭐ **管线已过期**（表面格式变了 ⇒ color target 格式不再匹配）。
    ///
    /// 处置：宿主调 [`crate::engine::Engine::recreate_pipeline`] 重建。
    ///
    /// ⚠ **尺寸**变化**不会**触发本错误（管线依赖的是表面**格式**，不是尺寸；
    /// 且 `gl_FragCoord` 的翻转高度已是 uniform ⇒ 改尺寸不必重编译着色器）。
    PipelineStale(String),
    /// 拿不到表面纹理（**配置非法**等）。
    ///
    /// ⚠ `Lost` / `Outdated` / `Timeout` / `Occluded` **各有专属变体**
    /// （语义完全不同 ⇒ 处置方式也完全不同）⇒ 宿主不必猜。
    SurfaceTextureUnavailable(String),
    /// 表面纹理**丢失**（交换链作废）—— **需重建表面**。
    ///
    /// 典型触发：WebGPU 后端在 `canvas.width/height` 变化时作废交换链。
    /// 引擎会**自动**重新配置表面并重试一次；仍失败 ⇒ 报本错误 ⇒ 宿主
    /// 应调 [`crate::engine::Engine::resize`] 或重建引擎。
    SurfaceTextureLost(String),
    /// 表面纹理**过期**（画布尺寸变了）—— 宿主需调
    /// [`crate::engine::Engine::resize`]。
    SurfaceTextureOutdated(String),
    /// 本帧没拿到表面纹理（**瞬时**）—— **跳过本帧**即可。
    SurfaceTextureTimeout(String),
    /// 画布当前不可见（**瞬时**）—— **跳过本帧**即可。
    SurfaceTextureOccluded(String),
    /// uniform 绑定失败（名字未知 / 分量数不符 / 缺值）。
    BindingFailed(String),
    /// 设备侧错误（校验 / 内部错误）—— 由引擎自己的错误处理器收集，
    /// **不 panic**（wgpu 的默认处理器在浏览器里会 panic）。
    DeviceError(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::NotImplemented => {
                write!(
                    f,
                    "NotImplemented: 该通路未接线（P0 桩按探测结论接入，见 crate 文档的 🔵 段）"
                )
            }
            GpuError::DeviceLost(detail) => write!(f, "DeviceLost: GPU 设备丢失（{detail}）"),
            GpuError::Unsupported(detail) => write!(f, "Unsupported: {detail}"),
            GpuError::SurfaceCreationFailed(detail) => {
                write!(f, "SurfaceCreationFailed: {detail}")
            }
            GpuError::AdapterUnavailable(detail) => {
                write!(f, "AdapterUnavailable: {detail}")
            }
            GpuError::DeviceRequestFailed(detail) => {
                write!(f, "DeviceRequestFailed: {detail}")
            }
            GpuError::ShaderCompilationFailed(detail) => {
                write!(f, "ShaderCompilationFailed: {detail}")
            }
            GpuError::PipelineCreationFailed(detail) => {
                write!(f, "PipelineCreationFailed: {detail}")
            }
            GpuError::PipelineStale(detail) => {
                write!(f, "PipelineStale: {detail}（需重建管线）")
            }
            GpuError::SurfaceTextureUnavailable(detail) => {
                write!(f, "SurfaceTextureUnavailable: {detail}")
            }
            GpuError::SurfaceTextureLost(detail) => {
                write!(f, "SurfaceTextureLost: {detail}（需重建表面）")
            }
            GpuError::SurfaceTextureOutdated(detail) => {
                write!(f, "SurfaceTextureOutdated: {detail}（需 resize）")
            }
            GpuError::SurfaceTextureTimeout(detail) => {
                write!(f, "SurfaceTextureTimeout: {detail}（瞬时，可跳过本帧）")
            }
            GpuError::SurfaceTextureOccluded(detail) => {
                write!(f, "SurfaceTextureOccluded: {detail}（瞬时，可跳过本帧）")
            }
            GpuError::BindingFailed(detail) => write!(f, "BindingFailed: {detail}"),
            GpuError::DeviceError(detail) => write!(f, "DeviceError: {detail}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// GPU 后端抽象：引擎唯一允许触碰「真实 GPU」的地方。
///
/// **签名修订（2026-10-24，P5 之后）**：P0 的签名是
/// `fn submit_frame(&self, uniforms: &SceneUniforms) -> Result<(), GpuError>`，
/// 而那**表达不了真实操作**，实测导致实现只能恒返 `Err(NotImplemented)`：
///
/// | P0 签名 | 现实 |
/// |---|---|
/// | `&SceneUniforms`（4 个几何量） | 着色器要 **16 个 uniform**（[`UniformValues`]） |
/// | `&self` | 绘制需要 `&mut ScenePipeline`（bind group / uniform 缓冲 / 顶点缓冲都在里面） |
/// | `Result<(), _>` | 真实通路返回 [`FrameReport`]（画了几帧、用的什么格式） |
///
/// ⇒ 抽象与现实脱节 ⇒ **P0 声明的这条抽象曾是条死路**，而真实通路是另一条并行 API。
/// ⇒ 本次把签名改成**与现实一致**，让**声明的抽象就是实际在用的那条**（不留两套 API）。
pub trait GpuContext {
    /// 本上下文的能力档位。
    fn tier(&self) -> Tier;

    /// 提交一帧（uniform → 绘制）。
    ///
    /// - `pipeline`：已建好的场景管线（持有 bind group / uniform 缓冲 / 顶点缓冲）。
    /// - `uniforms`：该帧的着色器 uniform 取值（**按名字写**，见 [`UniformValues::set_by_name`]）。
    ///
    /// # Errors
    ///
    /// 绑定失败 / 表面取不到纹理 / 设备错误 ⇒ [`GpuError`]。
    fn submit_frame(
        &self,
        pipeline: &mut ScenePipeline,
        uniforms: &UniformValues,
    ) -> Result<FrameReport, GpuError>;
}

/// **未接线**的 GPU 上下文（P0 桩）。
///
/// 诚实口径：所有绘制请求显式返回 [`GpuError::NotImplemented`]，**不假装**
/// 「场景已经在跑」。真实后端见 [`crate::engine::Engine`]。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableGpuContext;

impl GpuContext for UnavailableGpuContext {
    fn tier(&self) -> Tier {
        Tier::Software
    }

    fn submit_frame(
        &self,
        _pipeline: &mut ScenePipeline,
        _uniforms: &UniformValues,
    ) -> Result<FrameReport, GpuError> {
        Err(GpuError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trait_signature_can_express_a_real_frame() {
        // **钉住「抽象不再表达不了现实」**：P0 的签名（`&SceneUniforms` / `&self` /
        // `Result<(), _>`）表达不了真实操作 ⇒ 实现只能是死路，而真实通路是另一条
        // 并行 API。现在签名是
        // `(&mut ScenePipeline, &UniformValues) -> Result<FrameReport, _>`。
        //
        // ⇒ 用**泛型约束**做**类型层**的钉子：下面这个 helper 只在 trait 方法
        // **恰好是** `(&mut ScenePipeline, &UniformValues) -> Result<FrameReport, _>`
        // 时才编译得过 ⇒ 签名一旦退回 P0 形态，**本文件编译不过**
        // （不是运行时才发现、也不是靠注释提醒）。
        fn call_via_trait<C: GpuContext + ?Sized>(
            ctx: &C,
            pipeline: &mut ScenePipeline,
            uniforms: &UniformValues,
        ) -> Result<FrameReport, GpuError> {
            ctx.submit_frame(pipeline, uniforms)
        }
        // 引用它（不调用 —— native 造不出 ScenePipeline）。
        let _ = call_via_trait::<UnavailableGpuContext>;
    }

    #[test]
    fn the_trait_is_object_safe_and_carries_a_tier() {
        // 抽象要能被宿主当 `&dyn GpuContext` 持有（那才是这个 trait 的用途：
        // 「宿主不关心背后是真引擎还是桩」）。
        let ctx: &dyn GpuContext = &UnavailableGpuContext;
        assert_eq!(ctx.tier(), Tier::Software);
    }

    // ⚠️ 桩的**运行时**报错（`Err(NotImplemented)`）在 native 侧**测不了**：
    // `ScenePipeline` 内含 `wgpu::RenderPipeline` / `wgpu::BindGroup` 等**非 Option**
    // 类型 ⇒ 没有设备就造不出管线 ⇒ native 单测拿不到 `&mut ScenePipeline`。
    // ⇒ 该断言移到**浏览器侧**（P5 harness：那里有真管线）—— 覆盖更强（在真 GPU
    // 上下文里），且不与「native 不参与浏览器专属类型」这条纪律冲突。

    #[test]
    fn tier_capability_is_explicit() {
        assert!(!Tier::Software.can_render());
        assert!(!Tier::Compute.can_render());
        assert!(Tier::Full.can_render());
        assert!(Tier::Software < Tier::Full, "档位可排序 ⇒ 可做降级梯队");
    }
}
