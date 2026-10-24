//! 随 crate 入库的 GLSL 着色器资产。
//!
//! ## 来源与纪律
//!
//! - 资产内容 = 生产版 GLSL 的**去注释**副本（注释不参与任何语义）；
//! - **光学数值与表达式逐字未改** —— 引擎不改着色器数学
//!   （设计文档 §3.3.4：只回答「状态 → 像素」）；
//! - 去注释的**唯一**理由：不让与引擎无关的词汇进入引擎树
//!   （仓库纪律：引擎代码不出现业务名词）；
//! - 资产仍是 `#version 300 es` 的 **GLSL ES 3.0** 源 ⇒ 进 naga 之前
//!   **必须**先过 [`crate::glsl::transform`]（删 `#version` 行 +
//!   注入 `layout(binding=N)`）。
//!
//! ## 边界
//!
//! 本模块只提供**着色器文本**。uniform 的取值来源、场景参数语义与
//! 画面逐帧一致性属 **P5-b**，本批不做。

/// 顶点着色器（GLSL ES 3.0）。
pub const SCENE_VERTEX_GLSL: &str = include_str!("../assets/scene.vert");

/// 片元着色器（GLSL ES 3.0）。
pub const SCENE_FRAGMENT_GLSL: &str = include_str!("../assets/scene.frag");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glsl::{parse, uniform_declarations, ShaderStage};

    #[test]
    fn assets_are_glsl_es_3_0_sources() {
        assert!(SCENE_VERTEX_GLSL.contains("#version 300 es"));
        assert!(SCENE_FRAGMENT_GLSL.contains("#version 300 es"));
    }

    #[test]
    fn assets_carry_no_comments() {
        // 去注释是入库纪律（引擎树不出现业务词汇）⇒ 资产里不得再有注释。
        assert!(
            !SCENE_FRAGMENT_GLSL.contains("//"),
            "片元着色器资产里仍有行注释"
        );
        assert!(
            !SCENE_FRAGMENT_GLSL.contains("/*"),
            "片元着色器资产里仍有块注释"
        );
    }

    #[test]
    fn vertex_asset_parses_into_naga_ir() {
        let module =
            parse(SCENE_VERTEX_GLSL, ShaderStage::Vertex).expect("顶点着色器必须能过 naga");
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "main");
    }

    #[test]
    fn fragment_asset_parses_into_naga_ir() {
        let module =
            parse(SCENE_FRAGMENT_GLSL, ShaderStage::Fragment).expect("片元着色器必须能过 naga");
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "main");
    }

    #[test]
    fn fragment_asset_declares_sixteen_uniforms() {
        let decls = uniform_declarations(SCENE_FRAGMENT_GLSL);
        assert_eq!(decls.len(), 16);
        assert_eq!(decls[0].name, "uRes");
        assert_eq!(decls[0].type_name, "vec2");
        assert_eq!(decls[1].name, "uCenter");
        assert_eq!(decls[1].type_name, "vec2");
        assert!(decls[2..].iter().all(|d| d.type_name == "float"));
    }

    #[test]
    fn fragment_asset_keeps_its_helper_functions() {
        // 光学主体（helper 函数）必须逐字保住 —— 往返后一个都不能少。
        let module = parse(SCENE_FRAGMENT_GLSL, ShaderStage::Fragment).expect("解析成功");
        let mut names: Vec<&str> = module
            .functions
            .iter()
            .filter_map(|(_, f)| f.name.as_deref())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "diskRadiance",
                "fbm",
                "hash21",
                "main",
                "starLayer",
                "vnoise"
            ],
            "helper 函数集合必须与源一致"
        );
    }

    #[test]
    fn fragment_asset_keeps_its_output_variable() {
        let module = parse(SCENE_FRAGMENT_GLSL, ShaderStage::Fragment).expect("解析成功");
        assert!(
            module
                .global_variables
                .iter()
                .any(|(_, var)| { var.name.as_deref() == Some("fragColor") }),
            "片元输出变量必须保住"
        );
    }
}
