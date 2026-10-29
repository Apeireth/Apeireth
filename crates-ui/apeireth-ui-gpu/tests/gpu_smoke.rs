//! `apeireth-ui-gpu` 集成冒烟：**跨模块**行为。
//!
//! - 场景参数 → uniform（P0 的纯计算）；
//! - 未接线后端的诚实口径；
//! - P5 的端到端链：着色器源 → 前置变换 → naga IR → uniform 块布局 → 值写入；
//! - P5 的显式色彩契约：两个后端必须落到同一种字节编码。

use apeireth_ui_gpu::{
    binding_of, glsl, name_of, select_surface_format, BlackHoleParams, GpuContext, GpuError,
    OutputEncoding, Tier, UnavailableGpuContext, UniformBufferLayout, UniformValues,
    SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL, SHADER_UNIFORM_NAMES,
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

    // 桩的**运行时**报错在浏览器侧断言（这里 native 造不出 `ScenePipeline`
    // —— 它内含非 Option 的 wgpu 类型，见 `context.rs` 的说明）。
    assert!(GpuError::NotImplemented.to_string().contains("未接线"));
}

#[test]
fn production_shader_survives_the_whole_transform_chain() {
    // 着色器源 → 前置变换 → naga IR（两个阶段都要过）。
    let vertex =
        glsl::parse(SCENE_VERTEX_GLSL, glsl::ShaderStage::Vertex).expect("顶点着色器过 naga");
    let fragment =
        glsl::parse(SCENE_FRAGMENT_GLSL, glsl::ShaderStage::Fragment).expect("片元着色器过 naga");
    assert_eq!(vertex.entry_points.len(), 1);
    assert_eq!(fragment.entry_points.len(), 1);

    // naga 往返（GLSL ES 3.00 再吐一次 + 回环再解析）仍然成立。
    let emitted =
        glsl::emit_glsl_es_300(&fragment, glsl::ShaderStage::Fragment).expect("吐出 GLSL ES");
    assert!(emitted.starts_with("#version 300 es"));
    assert!(
        glsl::parse(&glsl::reparse_source(&emitted), glsl::ShaderStage::Fragment).is_ok(),
        "回环后必须仍能被 naga 解析"
    );
}

#[test]
fn uniform_block_layout_covers_every_mapped_uniform() {
    let decls = glsl::uniform_declarations(SCENE_FRAGMENT_GLSL);
    let members = glsl::uniform_block_members(&decls);
    let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");

    assert_eq!(decls.len(), SHADER_UNIFORM_NAMES.len());
    for (index, name) in SHADER_UNIFORM_NAMES.iter().enumerate() {
        let slot = layout.slot(index as u32).expect("成员存在");
        assert_eq!(&slot.name, name);
        assert_eq!(binding_of(name), Some(index as u32));
        assert_eq!(name_of(index as u32), Some(*name));
    }
    // std140 块大小必须是 16 的倍数（WebGL2 后端的硬要求）。
    assert_eq!(layout.total_size() % 16, 0);
}

#[test]
fn uniform_values_land_on_the_declared_offsets() {
    let decls = glsl::uniform_declarations(SCENE_FRAGMENT_GLSL);
    let members = glsl::uniform_block_members(&decls);
    let layout = UniformBufferLayout::from_members(&members, 0).expect("布局可建");

    let mut values = UniformValues::new();
    values.set(0, &[640.0, 480.0]); // uRes
    values.set(1, &[422.4, 264.0]); // uCenter
    values.set(2, &[46.08]); // uScale
    for index in 3..16 {
        values.set(index, &[0.25]);
    }

    let mut bytes = vec![0u8; layout.total_size() as usize];
    values.write_into(&layout, &mut bytes).expect("写入成功");

    let res = layout.slot(0).expect("uRes");
    let read = |offset: u64| {
        f32::from_le_bytes(
            bytes[offset as usize..offset as usize + 4]
                .try_into()
                .expect("4 字节"),
        )
    };
    assert_eq!(read(res.offset), 640.0);
    assert_eq!(read(res.offset + 4), 480.0);
    assert_eq!(read(layout.slot(2).expect("uScale").offset), 46.08);
}

#[test]
fn both_backends_land_on_the_same_byte_encoding() {
    // WebGL 后端给的格式列表里 sRGB 在前；WebGPU 后端只给 raw 格式。
    let webgl_like = [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba16Float,
    ];
    let webgpu_like = [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba16Float,
    ];
    let (webgl_format, webgl_encoding) = select_surface_format(&webgl_like).expect("可选");
    let (webgpu_format, webgpu_encoding) = select_surface_format(&webgpu_like).expect("可选");

    assert_eq!(webgl_encoding, webgpu_encoding, "两个后端必须同一种编码");
    assert_eq!(webgl_encoding, OutputEncoding::RawBytes);
    assert_eq!(webgl_format, webgpu_format);
}
