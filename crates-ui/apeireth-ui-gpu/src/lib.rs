//! # apeireth-ui-gpu — GPU 桥（状态 → 像素）
//!
//! 与 `ui-dom` **平级**的第二个投影（不是父子）：DOM 显影与 GPU 显影是同一份
//! 状态的**两条正交出路**（ADR-0015 §3.3；与 `00-PHILOSOPHY.md` §2「四个投影
//! 显影同一份状态」一致）。
//!
//! 职责边界（设计文档 §3.3.4）：只回答「**状态 → 像素**」，**不改着色器数学**
//! —— 电影感载体（GLSL / WGSL / CSS 合成）全在引擎之外，引擎一行都不碰。
//!
//! ## 本批（P0 骨架）交付口径
//! - ✅ **已实现**：GPU 后端抽象（[`GpuContext`]）、能力档位与降级梯队
//!   （[`Tier`]）、uniform 合成（状态 → 缓冲布局）、黑洞场景的纯计算
//!   （史瓦西半径 / 光子球 / 吸积盘内外半径 / 圆轨道速度）、显式未接线桩
//!   （[`UnavailableGpuContext`] 恒返回 [`GpuError::NotImplemented`]）。
//! - 🔵 **未接线**：wgpu 后端 —— 等 P0 探测结论（探测子线实测：wgpu 30.0.1，
//!   feature 必须成组开 `["webgl", "glsl", "wgsl", "naga-ir"]`；`glsl` 单独开
//!   会因上游 naga `front/mod.rs:5` 的 gate 条件导致编译失败）。
//!
//! ## 纪律
//! 本 crate 不写着色器、不写 CSS、不碰排版与主题（设计文档 §1.3）。

#![forbid(unsafe_code)]

pub mod context;
pub mod scene;

pub use context::{GpuContext, GpuError, Tier, UnavailableGpuContext};
pub use scene::{
    BlackHoleParams, SceneError, SceneUniforms, GRAVITATIONAL_CONSTANT, SOLAR_MASS, SPEED_OF_LIGHT,
};

#[cfg(test)]
mod tests {
    //! crate 级断言：跨模块组合行为（场景参数 → uniform → 后端提交）。

    use super::*;

    #[test]
    fn scene_params_flow_into_uniforms_and_submission() {
        let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
        let uniforms = params.to_uniforms(2.0e7, 1.25);

        let context = UnavailableGpuContext;
        assert_eq!(context.tier(), Tier::Software);
        assert_eq!(
            context.submit_frame(&uniforms),
            Err(GpuError::NotImplemented),
            "未接线的后端必须显式报未接线，不得静默成功"
        );
        assert!(uniforms.disk_outer_radius_km > uniforms.disk_inner_radius_km);
    }
}
