//! `apeireth-ui-gpu` 集成冒烟：场景参数 → uniform → 后端提交的**跨模块**行为。

use apeireth_ui_gpu::{
    BlackHoleParams, GpuContext, GpuError, SceneUniforms, Tier, UnavailableGpuContext,
};

#[test]
fn scene_parameters_flow_into_uniforms() {
    let params = BlackHoleParams::new(1.0, 0.0).expect("参数合法");
    let uniforms = params.to_uniforms(1.0e7, 0.0);

    assert_eq!(uniforms.camera_distance_km, 1.0e7);
    assert!(uniforms.disk_outer_radius_km > uniforms.disk_inner_radius_km);
    assert_eq!(uniforms.disk_inner_radius_km, params.disk_inner_radius_km());
}

#[test]
fn unavailable_backend_reports_not_implemented_honestly() {
    let context = UnavailableGpuContext;
    assert_eq!(context.tier(), Tier::Software);

    let err = context
        .submit_frame(&SceneUniforms::default())
        .expect_err("未接线不得静默成功");
    assert_eq!(err, GpuError::NotImplemented);
    assert!(err.to_string().contains("未接线"));
}
