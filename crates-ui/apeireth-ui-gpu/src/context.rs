//! GPU 后端抽象：状态 → 像素的唯一通道。
//!
//! 职责边界（设计文档 §3.3.4）：只回答「**状态 → 像素**」，与 `ui-dom`
//! **平级**（两条正交投影，不是父子）。**不改着色器数学**。

use crate::scene::SceneUniforms;

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
    /// 设备丢失（需重建上下文）。
    DeviceLost,
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
    /// 拿不到表面纹理（表面已被配置为别的设备等）。
    SurfaceTextureUnavailable(String),
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
            GpuError::DeviceLost => write!(f, "DeviceLost: GPU 设备丢失"),
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
            GpuError::SurfaceTextureUnavailable(detail) => {
                write!(f, "SurfaceTextureUnavailable: {detail}")
            }
            GpuError::BindingFailed(detail) => write!(f, "BindingFailed: {detail}"),
            GpuError::DeviceError(detail) => write!(f, "DeviceError: {detail}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// GPU 后端抽象：引擎唯一允许触碰「真实 GPU」的地方。
pub trait GpuContext {
    /// 本上下文的能力档位。
    fn tier(&self) -> Tier;

    /// 提交一帧（uniform → 绘制）。
    fn submit_frame(&self, uniforms: &SceneUniforms) -> Result<(), GpuError>;
}

/// **未接线**的 GPU 上下文（P0 桩）。
///
/// 诚实口径：所有绘制请求显式返回 [`GpuError::NotImplemented`]，**不假装**
/// 「场景已经在跑」。P0 探测结论回来后由真实后端替换。
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableGpuContext;

impl GpuContext for UnavailableGpuContext {
    fn tier(&self) -> Tier {
        Tier::Software
    }

    fn submit_frame(&self, _uniforms: &SceneUniforms) -> Result<(), GpuError> {
        Err(GpuError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_context_reports_not_implemented() {
        let context = UnavailableGpuContext;
        assert_eq!(context.tier(), Tier::Software);
        let err = context
            .submit_frame(&SceneUniforms::default())
            .expect_err("未接线的后端必须显式报错");
        assert_eq!(err, GpuError::NotImplemented);
    }

    #[test]
    fn tier_capability_is_explicit() {
        assert!(!Tier::Software.can_render());
        assert!(!Tier::Compute.can_render());
        assert!(Tier::Full.can_render());
        assert!(Tier::Software < Tier::Full, "档位可排序 ⇒ 可做降级梯队");
    }
}
