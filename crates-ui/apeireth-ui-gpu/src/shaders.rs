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

/// 片头「背景 matte」顶点着色器（生产 `intro/fire.ts` 的 `BG_VERT` 去注释副本）。
///
/// ⚠ **只做「去注释 + 令牌插值内联」两步**（见 [`INTRO_ASSET_TRANSFORMS`]）
///   ⇒ 着色器**主体**与生产**逐字相同**。
pub const INTRO_BACKDROP_VERTEX_GLSL: &str = include_str!("../assets/intro_backdrop.vert");

/// 片头「背景 matte」片元着色器（生产 `intro/fire.ts` 的 `BG_FRAG` 去注释副本）。
pub const INTRO_BACKDROP_FRAGMENT_GLSL: &str = include_str!("../assets/intro_backdrop.frag");

/// 片头「火焰粒子」顶点着色器（生产 `intro/fire.ts` 的 `FIRE_VERT` 去注释副本）。
pub const INTRO_SPRITE_VERTEX_GLSL: &str = include_str!("../assets/intro_sprite.vert");

/// 片头「火焰粒子」片元着色器（生产 `intro/fire.ts` 的 `FIRE_FRAG` 去注释副本）。
pub const INTRO_SPRITE_FRAGMENT_GLSL: &str = include_str!("../assets/intro_sprite.frag");

/// ⭐⭐ **varying 编码探针**（2026-10-27 诊断用）。
///
/// ⚠️ **不是**生产着色器 ⇒ ⇒ **不**进「引擎资产 == 生产源」的逐字校验表。
/// 用途：把 varying 编码进 RGB ⇒ ⇒ 直接读出「哪个 varying 不同」。
pub const INTRO_SPRITE_PROBE_FRAGMENT_GLSL: &str =
    include_str!("../assets/intro_sprite_probe.frag");

/// ⭐⭐ **纯色探针**（2026-10-27 诊断用）⇒ ⇒ 画布上只剩**粒子并集** ⇒ ⇒ 专测几何。
pub const INTRO_SPRITE_FLAT_FRAGMENT_GLSL: &str = include_str!("../assets/intro_sprite_flat.frag");

/// 资产与生产源之间的**声明外壳变换**（资产 = 生产源 ∘ 这些变换）。
///
/// ## 为什么要有这张表
///
/// 资产是**编译期**入库的副本 ⇒ 生产源改了 ⇒ 资产会**悄悄过期** ⇒ ⇒ 必须
/// 有**机制**保证二者不漂移 ⇒ 本表列出「资产相对生产源做了哪些变换」⇒ 由
/// 单测 `intro_assets_match_the_production_source` 逐条**核对**。
///
/// | 变换 | 说明 |
/// |---|---|
/// | 去注释 | 生产源里的中文注释**不参与任何语义** ⇒ 资产里不留（引擎树不出现业务词汇） |
/// | 令牌插值内联 | 生产源的 `${COLOR_*.join(',')}` ⇒ 按 `scene/tokens.ts` 的**原值**内联（不是猜值） |
///
/// ⚠ **着色器主体（表达式与数值）一字不改**。
pub const INTRO_ASSET_TRANSFORMS: [&str; 2] = ["去注释", "令牌插值内联"];

/// 片头「背景 matte」的 uniform 名（生产 `fire.ts` 的 `BG_UNIFORMS`）——**声明序**。
///
/// ⚠ **索引 = 成员序 = 声明序** ⇒ 宿主（场景层）按**索引**写 ⇒ 与引擎的
///   `glsl::fragment_uniform_block_members` 的成员序**必然一致**。
pub const INTRO_BACKDROP_UNIFORM_NAMES: [&str; 27] = [
    "uRes",
    "uAspect",
    "uTime",
    "uStars",
    "uStarStretch",
    "uMilky",
    "uMoon",
    "uMoonPos",
    "uMoonR",
    "uFurnace",
    "uFurnaceR",
    "uEngine",
    "uShock",
    "uShockApex",
    "uBody",
    "uBodyPos",
    "uBodyLen",
    "uBodyMolten",
    "uBodyMorph",
    "uPlanetHint",
    "uGroundGlow",
    "uGroundPos",
    "uCoreGlow",
    "uCorePos",
    "uFlash",
    "uFlashPos",
    "uVig",
];

/// 片头「火焰粒子」的 uniform 名（生产 `fire.ts` 的 `FIRE_UNIFORMS`）——**声明序**。
///
/// ⚠ 同上：**索引 = 成员序 = 声明序**。
pub const INTRO_SPRITE_UNIFORM_NAMES: [&str; 40] = [
    "uAspect",
    "uDpr",
    "uTime",
    "uP0",
    "uVel",
    "uAcc",
    "uDir",
    "uSpread",
    "uSpawnR",
    "uSpeed",
    "uSpeedVar",
    "uInherit",
    "uLife",
    "uLifeVar",
    "uSize0",
    "uSize1",
    "uSizeVar",
    "uGrav",
    "uDrag",
    "uTurb",
    "uTurbFreq",
    "uTurbTime",
    "uCount01",
    "uBurst",
    "uBurstSpan",
    "uRadial01",
    "uRadialSpeed",
    "uRadialDrag",
    "uRadialLife",
    "uSoft01",
    "uSoftSize",
    "uSoftGain",
    "uStreak",
    "uTwinkle",
    "uGain",
    "uEmber01",
    "uAlpha",
    "uColHot",
    "uColMid",
    "uColTail",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glsl::{parse, uniform_declarations, ShaderStage};

    /// 生产 `fire.ts` 的**源文本**（编译期嵌入 ⇒ 零运行时 IO）。
    const PRODUCTION_FIRE_TS: &str =
        include_str!("../../../frontend/companion-desktop/src/lib/intro/fire.ts");

    /// 从 `fire.ts` 里按「反引号模板」取出一条程序。
    fn extract(source: &str, name: &str) -> Option<String> {
        let head = format!("const {name} = `");
        let start = source.find(&head)? + head.len();
        let rest = &source[start..];
        let end = rest.find("`;")?;
        Some(rest[..end].to_string())
    }

    /// **声明外壳变换 ①**：把生产模板字符串里的 `${…}` 插值按生产口径替换。
    fn interpolate(source: &str) -> String {
        let subs = [
            ("${COLOR_VOID.baseRgb.join(',')}", "0.027,0.027,0.047"),
            ("${COLOR_STARFIELD.tintWarmRgb.join(',')}", "0.95,0.85,0.62"),
            ("${COLOR_STARFIELD.tintBoneRgb.join(',')}", "0.98,0.93,0.8"),
            ("${COLOR_STARFIELD.tintBlueRgb.join(',')}", "0.72,0.8,1.0"),
            ("${COLOR_STARFIELD.milkyWayRgb.join(',')}", "0.9,0.8,0.58"),
        ];
        let mut out = source.to_owned();
        for (from, to) in subs {
            out = out.replace(from, to);
        }
        out
    }

    /// **声明外壳变换 ②**：去注释（保留换行 ⇒ 行号不变）。
    fn strip_comments(source: &str) -> String {
        let mut out = String::with_capacity(source.len());
        let chars: Vec<char> = source.chars().collect();
        let mut index = 0usize;
        while index < chars.len() {
            if chars[index] == '/' && index + 1 < chars.len() {
                if chars[index + 1] == '/' {
                    while index < chars.len() && chars[index] != '\n' {
                        index += 1;
                    }
                    continue;
                }
                if chars[index + 1] == '*' {
                    index += 2;
                    while index < chars.len() {
                        if chars[index] == '\n' {
                            out.push('\n');
                        }
                        if chars[index] == '*' && index + 1 < chars.len() && chars[index + 1] == '/'
                        {
                            index += 2;
                            break;
                        }
                        index += 1;
                    }
                    continue;
                }
            }
            out.push(chars[index]);
            index += 1;
        }
        out
    }

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

    // -----------------------------------------------------------------------
    // ⭐ 片头「火之文明史」的四条 GLSL 资产
    // -----------------------------------------------------------------------

    #[test]
    fn intro_assets_match_the_production_source() {
        // ⭐⭐ **防漂移**门禁：资产 = 生产源 ∘ （去注释 + 令牌插值内联）
        //   ⇒ 生产源改了 ⇒ 本测试**变红** ⇒ ⇒ 资产与生产**不会**悄悄分叉。
        //
        // ⚠ 逐字比较 ⇒ 「着色器主体一字不改」这条纪律**可复核**。
        let cases = [
            ("BG_VERT", INTRO_BACKDROP_VERTEX_GLSL),
            ("BG_FRAG", INTRO_BACKDROP_FRAGMENT_GLSL),
            ("FIRE_VERT", INTRO_SPRITE_VERTEX_GLSL),
            ("FIRE_FRAG", INTRO_SPRITE_FRAGMENT_GLSL),
        ];
        for (name, asset) in cases {
            let raw = extract(PRODUCTION_FIRE_TS, name)
                .unwrap_or_else(|| panic!("生产 fire.ts 里取不到 `{name}`"));
            let expected = strip_comments(&interpolate(&raw));
            // ⚠ 逐行比较（**忽略行尾空白**）：空白在 GLSL 里不参与语义 ⇒
            //   ⇒ 归一化后仍不一致 ⇒ 一定是**语义**差异 ⇒ 必须重新生成资产。
            let asset_lines: Vec<&str> = asset.lines().map(|l| l.trim_end()).collect();
            let expected_lines: Vec<&str> = expected.lines().map(|l| l.trim_end()).collect();
            assert_eq!(
                asset_lines, expected_lines,
                "资产 `{name}` 与生产源不一致 ⇒ 生产改了这套着色器 ⇒ \
                 请重新生成资产（只允许「去注释 + 令牌插值内联」两步变换）"
            );
        }
    }

    #[test]
    fn intro_backdrop_assets_parse_into_naga_ir() {
        let vertex = parse(INTRO_BACKDROP_VERTEX_GLSL, ShaderStage::Vertex)
            .expect("背景顶点着色器必须能过 naga");
        assert_eq!(vertex.entry_points.len(), 1);
        let fragment = parse(INTRO_BACKDROP_FRAGMENT_GLSL, ShaderStage::Fragment)
            .expect("背景片元着色器必须能过 naga");
        assert_eq!(fragment.entry_points.len(), 1);
    }

    #[test]
    fn intro_backdrop_declares_the_twenty_seven_production_uniforms() {
        // 生产 `BG_UNIFORMS` 的 27 项 ⇒ 资产必须**同名同序** ⇒
        // ⇒ 索引 = 成员序 = 声明序（场景层按索引写）。
        let decls = uniform_declarations(INTRO_BACKDROP_FRAGMENT_GLSL);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names, INTRO_BACKDROP_UNIFORM_NAMES,
            "背景 uniform 的名字与顺序必须与生产一致"
        );
        assert_eq!(decls[0].type_name, "vec2");
        assert_eq!(decls[7].type_name, "vec2", "uMoonPos 是 vec2");
        // 生产 `BG_UNIFORMS` 里的 vec2：uRes / uMoonPos / uShockApex / uBodyPos
        // / uGroundPos / uCorePos / uFlashPos ⇒ **7** 个。
        assert_eq!(decls.iter().filter(|d| d.type_name == "vec2").count(), 7);
    }

    #[test]
    fn intro_sprite_assets_parse_into_naga_ir() {
        let vertex = parse(INTRO_SPRITE_VERTEX_GLSL, ShaderStage::Vertex)
            .expect("粒子顶点着色器必须能过 naga");
        assert_eq!(vertex.entry_points.len(), 1);
        let fragment = parse(INTRO_SPRITE_FRAGMENT_GLSL, ShaderStage::Fragment)
            .expect("粒子片元着色器必须能过 naga");
        assert_eq!(fragment.entry_points.len(), 1);
    }

    #[test]
    fn intro_sprite_declares_the_forty_production_uniforms() {
        let decls = uniform_declarations(INTRO_SPRITE_VERTEX_GLSL);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names, INTRO_SPRITE_UNIFORM_NAMES,
            "粒子 uniform 的名字与顺序必须与生产一致"
        );
    }
}
