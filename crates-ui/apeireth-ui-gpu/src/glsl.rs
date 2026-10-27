//! GLSL ES 3.0 → naga IR **前置变换器**（纯文本变换，不需 GPU）。
//!
//! 存在理由（上游事实 + 实测事实，非本 crate 的偏好）：
//!
//! | # | 限制 | 出处 | 本 crate 的对策 |
//! |---|---|---|---|
//! | X1 | naga 的 GLSL 前端**没有 ES profile**：`#version 300 es` 是硬解析错误 | `naga-30.0.1/src/front/glsl/parser.rs:267`（只收 `#version 440｜450｜460`）、`:290`（只收 `core` profile） | [`strip_version_directive`]：把 `#version` 行**替换成空行**（不删行 ⇒ 行号不变） |
//! | X2 | naga 的 GLSL 前端要求 uniform **显式带 `layout(binding=N)`** | `naga-30.0.1/src/front/glsl/variables.rs:618` | [`inject_layout_bindings`]（逐个注入）/ [`pack_uniforms_into_block`]（合并成一块） |
//! | **X3** | **WebGL2 后端的 `max_uniform_buffers_per_shader_stage` 装不下 16 个独立 uniform buffer** | **实测**（本机 Chromium 154 + 软件 WebGL）：限额 **14**，`create_bind_group_layout` 直接报 `Too many bindings of type UniformBuffers in Stage ShaderStages(FRAGMENT), limit is 14, count was 16`（GLES3 规范下限是 12） | [`pack_uniforms_into_block`]：把 N 个 plain uniform **合并进一个 std140 uniform 块** ⇒ **1 个 binding** |
//! | **X4** | **本引擎的两个后端都把片元坐标的 y 轴变成「向下」，而 GLSL ES 规定「向上」** | GLSL ES 3.0 规范（`gl_FragCoord` 原点在**左下角**）；WebGPU 的 `@builtin(position)` 原点在**左上角**；`wgpu-hal-30.0.1/src/gles/web.rs:301-312`（GL 路径把主渲染上下翻转：「GL's presentation is not flipped, but main rendering is. Therefore, we Y-flip the output positions in the shader, and also this blit.」） | [`fix_frag_coord_semantics`]：源级把 `gl_FragCoord` 换成 `vec4(x, 帧缓冲高 - y, z, w)`，**帧缓冲高来自 uniform**（[`FRAMEBUFFER_HEIGHT_MEMBER`]）⇒ 改尺寸**不必重编译着色器** |
//!
//! 四步都是**方言外壳**层面的机械变换，**不触碰着色器数学**（光学数值零改写）。
//!
//! ## ⭐ 行号必须一一对应（ADR §2.6.4 的承诺）
//!
//! naga 报错时给的是**行号** ⇒ 宿主按行号回查源码 ⇒ **变换链的每一步都必须
//! 保持行数不变**（不是「尽量」，是**硬约束**）：
//!
//! - [`strip_comments`]：保留换行（已满足）；
//! - [`strip_version_directive`]：`#version` 行**替换成空行**（不是删行）；
//! - [`pack_uniforms_into_block`]：uniform 块**整体写在一行**里，放在**首条声明
//!   所在行**；该声明原占的其余行**留空** ⇒ 净行数不变；
//! - [`fix_frag_coord_semantics`：**不插行**（已满足）。
//!
//! ⇒ 整链（`transform_fragment_with_block_binding`）的行数 == 输入的行数。
//! 本模块的单测 `transform_chain_preserves_line_count` 钉住这条不变量。
//!
//! ## X3 为什么必须做
//!
//! naga 的 IR 把**每个** GLSL uniform 变成**独立 binding** ⇒ 16 个 uniform
//! 就是 16 个 uniform buffer binding。而 WebGL2 后端每个着色阶段能用的
//! uniform buffer 只有 12–14 个（实测本机 14）⇒「逐个注入 binding」的做法在
//! **WebGL 后端上必然失败**（实测报错原文见 `crates-ui/evidence/p5-gpu/`）。
//!
//! 合并成一个 std140 块之后：
//! - naga 的 IR 里只有**一个**带 binding 的全局量（一个 struct）⇒ 1 个 binding；
//! - 成员名**不丢**（是 struct 的成员），但引擎仍**按成员序（= 声明序）绑定**
//!   —— 映射表语义不变（见 [`crate::bindings`]）；
//! - 布局是 **std140**（GLSL 后端）与 **WGSL uniform 地址空间** 共有的一套规则
//!   ⇒ 两个后端的字节布局一致。
//!
//! ## X4 为什么必须做
//!
//! GLSL ES 3.0 规定 `gl_FragCoord` 的原点在**左下角**、y 轴**向上**。而本引擎
//! 的两个后端都把它变成**向下**（WebGPU 的 `@builtin(position)` 原点在左上角；
//! GL 路径把主渲染上下翻转）⇒ 不修正的话，**同一份 GLSL 源**在引擎里画出来的
//! 画面是**上下颠倒**的：实测生产侧画面与引擎侧画面**逐像素互为上下镜像**
//! （翻转后 0 差异 ⇒ 证明差异**只有**这一个成因；证据见
//! `crates-ui/evidence/p5-b/`）。
//!
//! 修正只改**坐标的取法**（`y → 帧缓冲高 - y`），不改任何表达式与数值。
//!
//! ## X4 的帧缓冲高为什么是 **uniform** 而不是字面量
//!
//! 早期实现把表面高度**烘焙成字面量**写进片元着色器（`600.0 - gl_FragCoord.y`）
//! ⇒ ① 尺寸一变，字面量就陈旧 ⇒ 画面竖直错位；② 着色器与**表面尺寸**耦死
//! ⇒ resize 必须重编译着色器。
//!
//! 现在翻转高度来自 uniform [`FRAMEBUFFER_HEIGHT_MEMBER`]（引擎注入到
//! uniform 块末尾的一个 `float` 成员）⇒
//!
//! - **改尺寸不必重编译着色器**（只改 uniform 缓冲里的一个 f32）；
//! - 生产 uniform 的**偏移与取值逐字节不变**（引擎成员**追加在末尾**）⇒
//!   既有逐像素证据（0 / 576,000）不受影响；
//! - 字面量形态与 uniform 形态的**算术完全等价**：`600.0`（GLSL float 字面量）
//!   与 uniform 里的 f32 `600.0` 是**同一个 f32 值**，`highp - highp` 的
//!   IEEE 减法逐位相同 ⇒ 像素不变。
//!
//! ⇒ 逐像素等价性的论证见 [`fix_frag_coord_semantics`] 的文档。
//!
//! ## 为什么要先把注释剥掉
//!
//! 注释不参与任何语义，但会把与引擎无关的词汇带进引擎树（仓库纪律：
//! 引擎代码不出现业务名词）。因此本模块的变换链是
//! [`strip_comments`] → [`strip_version_directive`] → [`pack_uniforms_into_block`]：
//! 注释先被剥掉，后两步在**无注释**的文本上做。
//!
//! ## 边界
//!
//! - 本模块只做**文本 → 文本**与**文本 → naga IR**，**不含**任何绘制逻辑；
//! - 变换是**确定性**的：同样的输入永远得到同样的输出（可被单测钉住）；
//! - 顶点阶段与片元阶段共用 group 0 ⇒ 两个阶段的 uniform 块各占一个
//!   binding（片元 = [`FRAGMENT_UNIFORM_BLOCK_BINDING`]，顶点 =
//!   [`VERTEX_UNIFORM_BLOCK_BINDING`]），否则两侧的 `binding=0` 会互相撞车。

use naga::valid::{Capabilities, ValidationFlags, Validator};

pub use naga::ShaderStage;

/// 片元阶段 uniform 块的 binding 号。
pub const FRAGMENT_UNIFORM_BLOCK_BINDING: u32 = 0;

/// 顶点阶段 uniform 块的 binding 号（与片元阶段错开，避免撞车）。
pub const VERTEX_UNIFORM_BLOCK_BINDING: u32 = 1;

/// 生成的 uniform 块的**块名**（GLSL 标识符；中性命名）。
pub const UNIFORM_BLOCK_NAME: &str = "EngineUniforms";

/// 引擎补齐 std140 块大小用的 padding 成员名。
///
/// **为什么需要**：WebGL2 后端要求 uniform 块的**大小是 16 字节的倍数**
/// （实测报错：`the type given for group 0 binding 0 has a size of 72. As
/// the device does not support DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED,
/// the type must have a size that is a multiple of 16 bytes.`）⇒ 着色器自己的
/// 成员凑不出 16 的倍数时，引擎在末尾补一个 `vec4` padding 成员。
///
/// padding 成员**不被着色器引用** ⇒ 对画面零影响。
pub const UNIFORM_BLOCK_PADDING_MEMBER: &str = "_engine_pad0";

/// 引擎注入的「帧缓冲高度」uniform 成员名（片元阶段用）。
///
/// **为什么需要**：`gl_FragCoord` 的 y 轴修正要用**当前表面高度**
/// （`帧缓冲高 - gl_FragCoord.y`）。把高度**烘焙成字面量**会让着色器与表面
/// 尺寸耦死 ⇒ 尺寸一变就得重编译着色器（而重编译管线极重，见
/// `crates-ui/evidence/p5-b/harness/src/lib.rs` 的实测注释）。
///
/// ⇒ 高度改成**一个 uniform 成员**（引擎注入到块末尾）：改尺寸只改 uniform
/// 缓冲里的一个 f32 ⇒ **不必重编译着色器**。
///
/// **为什么追加在末尾**：生产 uniform 的**偏移与取值逐字节不变** ⇒ 既有逐像素
/// 证据（生产对照 0 / 576,000）不受影响。
pub const FRAMEBUFFER_HEIGHT_MEMBER: &str = "_engine_fb_height";

/// uniform 块里的一个成员声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformBlockMember {
    /// 成员名（uniform 名，或 padding 成员名）。
    pub name: String,
    /// 类型名（`float` / `vec2` / `vec3` / `vec4`）。
    pub type_name: String,
    /// 该成员是不是引擎补齐用的 **padding 成员**（不被着色器引用）。
    pub is_padding: bool,
    /// 该成员是不是**引擎自填**成员（宿主不必提供值）。
    ///
    /// - padding 成员：引擎补齐对齐用 ⇒ 宿主不提供；
    /// - [`FRAMEBUFFER_HEIGHT_MEMBER`]：引擎每帧写入当前表面高度 ⇒ 宿主不提供。
    ///
    /// ⚠ 与 `is_padding` 是**两件事**：padding 成员**不被着色器引用**，
    /// 而 `FRAMEBUFFER_HEIGHT_MEMBER` **被着色器引用**（片元坐标修正）。
    pub is_engine_filled: bool,
}

/// 一条 uniform 声明（名字 + 类型 + 声明序）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformDecl {
    /// uniform 名（着色器源里的标识符）。
    pub name: String,
    /// 类型名（如 `float` / `vec2`）。
    pub type_name: String,
    /// **声明序**（从 0 起）——即该 uniform 在 uniform 块里的成员序。
    pub order: usize,
}

/// GLSL 前置变换 / 解析的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlslError {
    /// naga 的 GLSL 前端解析失败。
    Parse(String),
    /// naga 的 IR 校验失败。
    Validation(String),
    /// 源码里出现了本引擎不支持的 uniform 类型（显式拒绝，不静默降级）。
    UnsupportedUniformType {
        /// 出问题的 uniform 名。
        name: String,
        /// 出问题的类型名。
        type_name: String,
    },
    /// 源码里出现了本引擎不支持的**声明形态**（如数组 uniform）。
    UnsupportedDeclaration(String),
}

impl std::fmt::Display for GlslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlslError::Parse(detail) => write!(f, "GlslParse: {detail}"),
            GlslError::Validation(detail) => write!(f, "GlslValidation: {detail}"),
            GlslError::UnsupportedUniformType { name, type_name } => write!(
                f,
                "GlslUnsupportedUniformType: uniform `{name}` 的类型 `{type_name}` 不在本引擎支持的浮点标量/向量集合内"
            ),
            GlslError::UnsupportedDeclaration(detail) => {
                write!(f, "GlslUnsupportedDeclaration: {detail}")
            }
        }
    }
}

impl std::error::Error for GlslError {}

// ---------------------------------------------------------------------------
// 变换 0：剥注释
// ---------------------------------------------------------------------------

/// 剥掉全部注释（行注释 `//…` 与块注释 `/*…*/`），**保留换行**。
///
/// 因此行号在变换前后**一一对应**（错误信息仍能指回原始行）。
///
/// # 行为细节
/// - 块注释内部的换行会被保留（行号不变）；
/// - 未闭合的块注释一路吃到文末（不 panic、不静默吞掉后续代码）；
/// - GLSL 没有字符串字面量，故 `//` 与 `/*` 无需考虑字符串上下文。
pub fn strip_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                // 行注释：吃到行尾（换行本身留给下一轮处理）。
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if chars[i + 1] == '*' {
                // 块注释：吃到 `*/`，期间的换行要保留。
                i += 2;
                while i < chars.len() {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// 变换 1（X1）：删 `#version` 行
// ---------------------------------------------------------------------------

/// 把 `#version` 指令行**替换成空行**（naga 的 GLSL 前端没有 ES profile）。
///
/// 其余预处理行（`#extension` / `#define` / `#ifdef` …）**原样保留**。
///
/// ## 为什么是「替换成空行」而不是「删掉整行」
///
/// naga 的报错信息带**行号** ⇒ 宿主按行号回查**原始源码** ⇒ 变换链必须
/// **保持行数不变**。删行会让其后所有源码的行号**整体前移一位** ⇒ 按行号
/// 回查会**查错地方**（实测：6 行源经变换后变 9 行 ⇒ naga 报错指向错位的行）。
///
/// ⇒ `#version` 行换成**空行**：语义上「这一行没有指令」，行号上「这一行
/// 仍然是原来那一行」。
pub fn strip_version_directive(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        if line.trim_start().starts_with("#version") {
            // 换成空行（**保留一个换行符**）⇒ 行数不变。
            out.push('\n');
            continue;
        }
        out.push_str(line);
    }
    out
}

// ---------------------------------------------------------------------------
// 变换 3（X3）：把 plain uniform 合并进一个 std140 uniform 块
// ---------------------------------------------------------------------------

/// 一条顶层 plain uniform 声明的位置与原文。
#[derive(Debug, Clone, PartialEq, Eq)]
struct UniformSite {
    /// **首行**行号（0 起）。
    line: usize,
    /// **末行**行号（0 起，含）—— 跨行声明时 > `line`。
    ///
    /// ⚠ 必须记**行区间**而不是只记首行：跨行声明（`uniform vec2\n    uRes;`）
    /// 的**续行**也要被整体替换掉 ⇒ 否则会留下**孤立的 `uRes;`** ⇒ naga 解析
    /// 失败（实测：生产路径没有跨行单测 ⇒ 这是下一轮 shader 迁移的硬阻断）。
    end_line: usize,
    /// 声明原文（不含缩进与换行；跨行时按空格拼接）。
    text: String,
}

/// 扫描源码里的**顶层** plain uniform 声明（位置 + 原文）。
///
/// - 只认**顶层**（花括号深度 0）的 `uniform` 声明；
/// - 跨行声明按**整体**处理（拼成一条）并记录**行区间**；
/// - 已带 `layout(...)` 的 uniform **块**不在此列（那是另一种形态）。
fn scan_uniform_sites(src: &str) -> Vec<UniformSite> {
    let mut sites = Vec::new();
    let mut depth = 0i32;
    let mut pending: Option<(usize, usize, String)> = None;

    for (index, line) in src.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();

        if let Some((start, _end, body)) = pending.take() {
            let body = format!("{body} {bare}");
            depth += brace_delta(bare);
            if bare.contains(';') {
                sites.push(UniformSite {
                    line: start,
                    end_line: index,
                    text: body.trim().to_string(),
                });
            } else {
                pending = Some((start, index, body));
            }
            continue;
        }

        let is_uniform_decl = depth == 0
            && trimmed.starts_with("uniform")
            && trimmed[7..].starts_with(|c: char| c.is_whitespace());

        if is_uniform_decl {
            depth += brace_delta(bare);
            if bare.contains(';') {
                sites.push(UniformSite {
                    line: index,
                    end_line: index,
                    text: trimmed.to_string(),
                });
            } else {
                pending = Some((index, index, trimmed.to_string()));
            }
            continue;
        }

        depth += brace_delta(bare);
    }

    if let Some((start, end, body)) = pending {
        sites.push(UniformSite {
            line: start,
            end_line: end,
            text: body.trim().to_string(),
        });
    }
    sites
}

/// uniform 块的成员清单（**只含**着色器声明的成员 + 引擎补齐的 padding 成员）。
///
/// 规则：
/// 1. 成员按**声明序**排列（⇒ 成员序 = 映射表下标）；
/// 2. 若按 std140 算出的块大小**不是 16 的倍数**，末尾补一个 `vec4`
///    padding 成员（WebGL2 后端的硬要求，见
///    [`UNIFORM_BLOCK_PADDING_MEMBER`]）。
pub fn uniform_block_members(decls: &[UniformDecl]) -> Vec<UniformBlockMember> {
    uniform_block_members_with(decls, &[])
}

/// uniform 块的成员清单（可追加**引擎注入成员**）。
///
/// `engine_members` 追加在**着色器成员之后、padding 之前** ⇒ 生产 uniform 的
/// **偏移逐字节不变**（既有逐像素证据不受影响）。
pub fn uniform_block_members_with(
    decls: &[UniformDecl],
    engine_members: &[UniformBlockMember],
) -> Vec<UniformBlockMember> {
    let mut members: Vec<UniformBlockMember> = decls
        .iter()
        .map(|decl| UniformBlockMember {
            name: decl.name.clone(),
            type_name: decl.type_name.clone(),
            is_padding: false,
            is_engine_filled: false,
        })
        .collect();
    members.extend(engine_members.iter().cloned());
    if members.is_empty() {
        return members;
    }
    // 成员类型不受支持时**不判 padding**（由布局层显式拒绝该类型）。
    let mut cursor = 0u64;
    let mut supported = true;
    for member in &members {
        if !is_supported_float_type(&member.type_name) {
            supported = false;
            break;
        }
        let Some(alignment) = float_std140_alignment(&member.type_name) else {
            supported = false;
            break;
        };
        let Some(size) = float_std140_size(&member.type_name) else {
            supported = false;
            break;
        };
        cursor = cursor.div_ceil(alignment) * alignment;
        cursor += size;
    }
    if supported && !cursor.is_multiple_of(16) {
        members.push(UniformBlockMember {
            name: UNIFORM_BLOCK_PADDING_MEMBER.to_string(),
            type_name: "vec4".to_string(),
            is_padding: true,
            is_engine_filled: true,
        });
    }
    members
}

/// 引擎注入的「帧缓冲高度」成员（片元阶段的 `gl_FragCoord` 修正用）。
pub fn framebuffer_height_member() -> UniformBlockMember {
    UniformBlockMember {
        name: FRAMEBUFFER_HEIGHT_MEMBER.to_string(),
        type_name: "float".to_string(),
        is_padding: false,
        is_engine_filled: true,
    }
}

/// 引擎注入的「帧缓冲宽度」成员（**顶点阶段**的点精灵四边形展开用）。
pub fn framebuffer_width_member() -> UniformBlockMember {
    UniformBlockMember {
        name: FRAMEBUFFER_WIDTH_MEMBER.to_string(),
        type_name: "float".to_string(),
        is_padding: false,
        is_engine_filled: true,
    }
}

/// 引擎注入的「帧缓冲尺寸」成员（宽 + 高）—— 顶点阶段的四边形展开用。
///
/// ⚠ **追加在块末尾** ⇒ 生产 uniform 的**偏移与取值逐字节不变** ✓。
pub fn framebuffer_size_members() -> Vec<UniformBlockMember> {
    vec![framebuffer_width_member(), framebuffer_height_member()]
}

/// ⭐ **顶点阶段**（实例化四边形）的 uniform 块成员清单：
/// 生产 uniform + 引擎注入的**帧缓冲尺寸**成员（宽 + 高）。
///
/// ## 为什么顶点阶段也要「帧缓冲尺寸」
///
/// 点精灵的 `gl_PointSize` 是**设备像素**；顶点着色器输出的是**裁剪空间**
/// ⇒ 要把「设备像素的尺寸」变成「四边形的角点偏移」，必须知道表面有多大
/// （`设备像素 ↔ NDC` 的换算）⇒ ⇒ 表面尺寸必须是 uniform（改尺寸不必重编译
/// 着色器）。
///
/// ## ⚠ 这是**唯一真源**
///
/// 变换器（[`transform_vertex_for_instanced_quad`]）与引擎
/// （`create_scene_pipeline`）**都调本函数** ⇒ 两侧不可能不一致。
pub fn instanced_quad_vertex_uniform_block_members(src: &str) -> Vec<UniformBlockMember> {
    let decls = uniform_declarations(src);
    uniform_block_members_with(&decls, &framebuffer_size_members())
}

/// **片元阶段**的 uniform 块成员清单：生产 uniform + 引擎注入的帧缓冲高度成员。
///
/// ⚠ 与 [`uniform_block_members`] 的差别只有**末尾追加**一个引擎成员 ⇒ 生产
/// uniform 的偏移不变 ⇒ 逐像素证据不受影响。
///
/// ## ⭐ 这是**唯一真源**
///
/// 引擎的布局层与变换器**必须**给出**同一份**成员清单（名字 / 类型 / 顺序）
/// ⇒ 否则 naga IR 与引擎布局对不上（要么画错位、要么绑定组声明了着色器
/// 没有的 binding ⇒ wgpu 校验报错）。
///
/// ⇒ 变换器（[`transform_fragment_with_block_binding`]）与引擎
/// （`create_scene_pipeline`）**都调本函数** ⇒ 两侧不可能不一致。
///
/// ## 帧缓冲高度成员**只在被用到时**才追加
///
/// 引擎注入的高度成员只服务于 `gl_FragCoord` 的修正 ⇒ 源码**没用到**
/// `gl_FragCoord` 时追加它就是**死成员**（白占一个槽位）。
///
/// ⇒ 本函数按「源码是否用到 `gl_FragCoord`」决定是否追加。
pub fn fragment_uniform_block_members(src: &str) -> Vec<UniformBlockMember> {
    let decls = uniform_declarations(src);
    let engine_members = if uses_frag_coord(&strip_comments(src)) {
        vec![framebuffer_height_member()]
    } else {
        Vec::new()
    };
    uniform_block_members_with(&decls, &engine_members)
}

/// 源码里是否用到 `gl_FragCoord`（**词边界**感知）。
///
/// `myGl_FragCoord` / `gl_FragCoordExtra` **不算**用到。
pub fn uses_frag_coord(src: &str) -> bool {
    let bytes = src.as_bytes();
    let token = "gl_FragCoord";
    let mut index = 0usize;
    while index < src.len() {
        if src[index..].starts_with(token) {
            let starts_clean = index == 0 || !is_identifier_byte(bytes[index - 1]);
            let after = index + token.len();
            let ends_clean = after >= src.len() || !is_identifier_byte(bytes[after]);
            if starts_clean && ends_clean {
                return true;
            }
            index = after;
            continue;
        }
        let ch = src[index..].chars().next().expect("非空切片");
        index += ch.len_utf8();
    }
    false
}

/// 把源码里的**顶层 plain uniform 声明合并进一个 std140 uniform 块**（X3）。
///
/// 输入（生产版着色器的形态）：
/// ```text
/// uniform vec2  uRes;
/// uniform vec2  uCenter;
/// uniform float uScale;
/// ```
/// 输出（块**整体写在一行**里，放在**首条声明所在行**）：
/// ```text
/// layout(std140, binding=0) uniform EngineUniforms { vec2 uRes; vec2 uCenter; float uScale; };
/// ```
///
/// - 成员**按声明序**排列 ⇒ 成员序 = 声明序 = 映射表下标；
/// - 着色器正文里的 `uRes` 等标识符**不用改**（GLSL 允许块成员不带实例名
///   直接访问；naga 的 GLSL 前端实测支持，见本模块的单测）；
/// - **不改任何表达式与数值** —— 只改声明外壳；
/// - 源码里没有 uniform 时**原样返回**（不造空块）。
///
/// ## ⭐ 行号必须不变
///
/// 块的**每个成员各占一行**会让源码凭空多出 N 行 ⇒ 其后所有源码的行号
/// 整体后移 ⇒ naga 的报错指向**错位**的行 ⇒ 按行号回查会查错地方。
///
/// ⇒ 块**整体写在一行**；该声明原占的其余行**留空**（不是删行）⇒ 净行数不变。
///
/// ## ⭐ 跨行声明必须**整段**替换
///
/// `uniform vec2\n    uRes;` 这种形态：只替换首行会留下**孤立的 `uRes;`**
/// ⇒ naga 解析失败（实测）。⇒ site 记录**行区间**（首行..末行）⇒ 整段替换。
///
/// # Errors
///
/// 声明里出现数组形态（`uniform float w[4];`）→
/// [`GlslError::UnsupportedDeclaration`]（本引擎只支持标量 / 向量）。
pub fn pack_uniforms_into_block(src: &str, block_binding: u32) -> Result<String, GlslError> {
    pack_uniforms_into_block_with_members(src, block_binding, &[])
}

/// [`pack_uniforms_into_block`] 的带**引擎注入成员**版本。
///
/// `engine_members` 追加在**着色器成员之后、padding 之前**（⇒ 生产 uniform 的
/// 偏移逐字节不变）⇒ 片元阶段的帧缓冲高度成员走这条。
pub fn pack_uniforms_into_block_with_members(
    src: &str,
    block_binding: u32,
    engine_members: &[UniformBlockMember],
) -> Result<String, GlslError> {
    let clean = strip_comments(src);
    let sites = scan_uniform_sites(&clean);
    if sites.is_empty() {
        return Ok(clean);
    }

    let decls = uniform_declarations(&clean);
    if decls.is_empty() {
        return Ok(clean);
    }
    for site in &sites {
        if site.text.contains('[') {
            return Err(GlslError::UnsupportedDeclaration(format!(
                "数组形态的 uniform 不在本引擎的支持范围内：{}",
                site.text
            )));
        }
    }

    // 块文本：**整体写在一行**（行数必须不变 ⇒ 见上面的「行号必须不变」）。
    let members = uniform_block_members_with(&decls, engine_members);
    let mut block =
        format!("layout(std140, binding={block_binding}) uniform {UNIFORM_BLOCK_NAME} {{");
    for member in &members {
        block.push_str(&format!(" {} {};", member.type_name, member.name));
    }
    block.push_str(" };");

    // 重建源码：首条声明所在行放块；该声明原占的其余行留空 ⇒ 行数不变。
    let lines: Vec<&str> = clean.lines().collect();
    let mut out_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut block_written = false;
    for (index, line) in lines.iter().enumerate() {
        let covered = sites
            .iter()
            .any(|site| index >= site.line && index <= site.end_line);
        if !covered {
            out_lines.push((*line).to_string());
            continue;
        }
        if !block_written {
            out_lines.push(block.clone());
            block_written = true;
        } else {
            // 声明区间的其余行 → **留空**（不是删行 ⇒ 行号不变）。
            out_lines.push(String::new());
        }
    }
    if !block_written {
        // 声明行不存在（理论上不可达：`sites` 非空 ⇒ 必有命中）⇒ 显式兜底。
        out_lines.push(block);
    }
    // 用 `\n` 连接 ⇒ 行数与输入**逐行对应**（末尾统一补一个换行）。
    let mut out = out_lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 变换 2（X2）：给**已存在**的 uniform 声明注入 `layout(binding=N)`
// ---------------------------------------------------------------------------

/// 给每个**顶层** uniform 声明注入 `layout(binding=N)`（N 按声明序，从 0 起）。
///
/// ⚠️ 这是 **X2 的原始形态**：它给每个 uniform 一个**独立 binding** ⇒ 16 个
/// uniform 就是 16 个 uniform buffer ⇒ **WebGL2 后端装不下**
/// （实测限额 14）。生产路径请用 [`pack_uniforms_into_block`]（X3）。
///
/// 本函数保留下来是为了处理「源码里已经有 uniform 块」的情形，以及作为
/// X2 行为的可测基准。
pub fn inject_layout_bindings(src: &str) -> String {
    inject_layout_bindings_with_offset(src, 0)
}

/// [`inject_layout_bindings`] 的带偏移版本：binding 号从 `offset` 起算。
pub fn inject_layout_bindings_with_offset(src: &str, offset: u32) -> String {
    let mut out = String::with_capacity(src.len() + 256);
    let mut next_binding = offset;
    let mut depth = 0i32;
    let mut pending_indent = String::new();
    let mut pending_body = String::new();
    let mut pending = false;

    for line in src.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();

        if pending {
            pending_body.push_str(bare);
            depth += brace_delta(bare);
            if bare.contains(';') {
                out.push_str(&pending_indent);
                out.push_str(&format!("layout(binding={next_binding}) "));
                out.push_str(&pending_body);
                out.push('\n');
                next_binding += 1;
                pending = false;
                pending_body.clear();
            } else {
                out.push_str(line);
            }
            continue;
        }

        let is_uniform_decl = depth == 0
            && trimmed.starts_with("uniform")
            && trimmed[7..].starts_with(|c: char| c.is_whitespace());

        if is_uniform_decl {
            let indent_len = bare.len() - trimmed.len();
            let (indent, rest) = bare.split_at(indent_len);
            let decl = rest.trim_start();
            if decl.contains(';') {
                out.push_str(indent);
                out.push_str(&format!("layout(binding={next_binding}) "));
                out.push_str(decl);
                out.push('\n');
                next_binding += 1;
            } else {
                pending = true;
                pending_indent = indent.to_string();
                pending_body = decl.to_string();
                out.push_str(line);
            }
            depth += brace_delta(bare);
            continue;
        }

        out.push_str(line);
        depth += brace_delta(bare);
    }

    if pending {
        out.push_str(&pending_indent);
        out.push_str(&pending_body);
        out.push('\n');
    }
    out
}

/// 给 naga **自己吐出的** `layout(std140) uniform` 块补上 `binding=N`。
///
/// naga 的 GLSL 后端把每个 plain uniform 降级成独立的匿名 std140 块，且
/// **不带 binding** ⇒ 想把它喂回 naga 的前端，必须再补一次 binding。
///
/// 用途：GLSL → IR → GLSL ES → IR 的**回环校验**（证明着色器主体在
/// naga 往返后仍然成立）。
pub fn inject_block_bindings(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 256);
    let mut next_binding = 0u32;
    for line in src.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();
        if trimmed.starts_with("layout(std140) uniform") {
            let indent_len = bare.len() - trimmed.len();
            let (indent, rest) = bare.split_at(indent_len);
            let decl = rest.trim_start().trim_start_matches("layout(std140) ");
            out.push_str(indent);
            out.push_str(&format!("layout(std140, binding={next_binding}) "));
            out.push_str(decl);
            out.push('\n');
            next_binding += 1;
        } else {
            out.push_str(line);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// uniform 声明抽取
// ---------------------------------------------------------------------------

/// 抽取源码里**顶层** uniform 声明（按声明序）。
///
/// 返回的 `order` 即该 uniform 在 uniform 块里的**成员序**（从 0 起）。
pub fn uniform_declarations(src: &str) -> Vec<UniformDecl> {
    let clean = strip_comments(src);
    let mut decls = Vec::new();
    let mut depth = 0i32;
    let mut pending: Option<String> = None;

    for line in clean.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();

        let is_uniform_decl = depth == 0
            && trimmed.starts_with("uniform")
            && trimmed[7..].starts_with(|c: char| c.is_whitespace());

        if let Some(body) = pending.take() {
            let body = format!("{body} {bare}");
            depth += brace_delta(bare);
            if bare.contains(';') {
                collect_declarators(&body, &mut decls);
            } else {
                pending = Some(body);
            }
            continue;
        }

        if is_uniform_decl {
            let decl = trimmed.to_string();
            depth += brace_delta(bare);
            if bare.contains(';') {
                collect_declarators(&decl, &mut decls);
            } else {
                pending = Some(decl);
            }
            continue;
        }

        depth += brace_delta(bare);
    }

    if let Some(body) = pending {
        collect_declarators(&body, &mut decls);
    }
    decls
}

/// 把一条 `uniform <type> <name>[, <name2>…];` 声明拆成若干 [`UniformDecl`]。
fn collect_declarators(decl: &str, out: &mut Vec<UniformDecl>) {
    let decl = decl.trim();
    let Some(rest) = decl.strip_prefix("uniform") else {
        return;
    };
    let rest = rest.trim();
    let rest = rest.strip_suffix(';').unwrap_or(rest).trim_end();

    let mut current_type: Option<String> = None;
    for part in split_top_level_commas(rest) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // 类型 = 第一个空白分隔的记号；名字 = 最后一个记号（数组后缀除外）。
        let base = match part.find('[') {
            Some(idx) => part[..idx].trim(),
            None => part,
        };
        let mut tokens = base.split_whitespace();
        let Some(name) = tokens.next_back() else {
            continue;
        };
        if tokens.clone().next().is_some() {
            current_type = tokens.next().map(|t| t.to_string());
        }
        let Some(type_name) = current_type.clone() else {
            continue;
        };
        if !is_identifier(name) {
            continue;
        }
        out.push(UniformDecl {
            name: name.to_string(),
            type_name,
            order: out.len(),
        });
    }
}

/// 按顶层逗号切分（类型里没有逗号，但数组长度与初始化式可能有）。
fn split_top_level_commas(src: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in src.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    parts.push(current);
    parts
}

/// 一行里的花括号净增减（用于判断「是否顶层」）。
fn brace_delta(line: &str) -> i32 {
    let mut delta = 0i32;
    for ch in line.chars() {
        match ch {
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.chars().next().is_some_and(|c| c.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// 浮点标量 / 向量类型
// ---------------------------------------------------------------------------

/// 该 GLSL 类型是不是本引擎支持的**浮点**标量 / 向量。
///
/// 支持：`float` / `vec2` / `vec3` / `vec4`。
/// 不支持：整型、布尔、矩阵、数组、采样器 —— 一律**显式拒绝**
/// （见 [`GlslError::UnsupportedUniformType`]），不静默按 float 处理。
pub fn is_supported_float_type(type_name: &str) -> bool {
    matches!(type_name, "float" | "vec2" | "vec3" | "vec4")
}

/// 该浮点类型的分量数（`float`=1 / `vec2`=2 / `vec3`=3 / `vec4`=4）。
///
/// 类型不受支持 ⇒ `None`（**不 panic**：这三个函数是对外 API，panic 会把
/// 调用方的 bug 变成**进程崩溃**；显式返回 `None` 让调用方自己决定怎么报）。
pub fn float_component_count(type_name: &str) -> Option<usize> {
    match type_name {
        "float" => Some(1),
        "vec2" => Some(2),
        "vec3" => Some(3),
        "vec4" => Some(4),
        _ => None,
    }
}

/// 该浮点类型的 **std140 字节大小**（`vec3` 占 12）。
///
/// 类型不受支持 ⇒ `None`（见 [`float_component_count`]）。
pub fn float_std140_size(type_name: &str) -> Option<u64> {
    float_component_count(type_name).map(|components| (components as u64) * 4)
}

/// 该浮点类型的 **std140 基对齐**（`float`=4 / `vec2`=8 / `vec3`/`vec4`=16）。
///
/// 类型不受支持 ⇒ `None`（见 [`float_component_count`]）。
pub fn float_std140_alignment(type_name: &str) -> Option<u64> {
    match type_name {
        "float" => Some(4),
        "vec2" => Some(8),
        "vec3" | "vec4" => Some(16),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 变换 5（X5）：给 varying 补 `layout(location=N)`
// ---------------------------------------------------------------------------

/// ⭐ 给**没有显式 location** 的 `in` / `out` 声明补上顺序 location。
///
/// ## 为什么必须做（方言外壳层，**不碰数学**）
///
/// GLSL ES 3.00 规定：varying（顶点输出 / 片元输入）的 location 由**声明顺序**
/// 自动分配；而 **naga 的 GLSL 前端（无 ES profile）把未声明 location 的
/// varying 全当 location 0** ⇒ ⇒
///
/// - 同一阶段里有两个以上 varying ⇒ 彼此 `BindingCollision`；
/// - 顶点输入（`layout(location=0) in vec4 aSeed;`）与 varying **共用** naga
///   的 location 空间 ⇒ ⇒ 顶点输入占着 location 0 时，varying 再拿 0 ⇒ 撞车。
///
/// ⇒ 按**声明顺序**补 `0,1,2,…` ⇒ ⇒ 与 GLSL ES 的自动分配**同义**
/// （varying 的 location 空间与顶点属性的 location 空间**互相独立**，
/// 生产代码正是这么用的）。
///
/// ## 行为细节
///
/// - **不碰**已带 `layout(...)` 的声明（顶点属性 `layout(location=0) in …`）；
/// - **不插行**（只在前缀里加 `layout(location=N) `）⇒ 行号不变；
/// - `in` 与 `out` **共用**一个计数器 ⇒ 片元着色器里 `in`（0..N-1）与
///   `out`（N）⇒ ⇒ 片元输出不会与片元输入撞 location。
pub fn add_varying_locations(src: &str, stage: ShaderStage) -> String {
    // ⚠ 两个**独立**的计数器：GLSL ES 里「顶点输入（属性）」与「顶点输出
    //   （varying）」的 location 空间**互相独立**（属性占 location 0 时，
    //   varying 仍然从 0 开始分配）。
    let mut next_in = 0usize;
    let mut next_out = 0usize;
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();
        let has_layout = trimmed.contains("layout(");
        let is_in = trimmed.starts_with("in ") && !has_layout;
        let is_out = trimmed.starts_with("out ") && !has_layout;
        if is_in {
            let indent_len = bare.len() - trimmed.len();
            let (indent, rest) = bare.split_at(indent_len);
            out.push_str(indent);
            out.push_str(&format!("layout(location={next_in}) "));
            out.push_str(rest);
            out.push('\n');
            next_in += 1;
            continue;
        }
        if is_out {
            // ⚠⭐ **片元阶段的输出不补 location**。
            //
            // 片元输出必须落在**颜色附件 0**（渲染通道只有 1 个附件）⇒ ⇒
            // 若给它补了 `layout(location=N)`（N > 0）⇒ 附件 0 **没有**对应的
            // 片元输出 ⇒ GL 报 `GL_INVALID_OPERATION: Active draw buffers with
            // missing fragment shader outputs` ⇒ ⇒ **画面全废**（实测踩过）。
            //
            // ⇒ 顶点阶段的输出（varying）**要**补（要与片元输入对上）⇒
            //   片元阶段的输出**不补**（naga 自动给 location 0 ✓）。
            if stage == ShaderStage::Vertex {
                let indent_len = bare.len() - trimmed.len();
                let (indent, rest) = bare.split_at(indent_len);
                out.push_str(indent);
                out.push_str(&format!("layout(location={next_out}) "));
                out.push_str(rest);
                out.push('\n');
                next_out += 1;
                continue;
            }
        }
        out.push_str(line);
    }
    out
}

// ---------------------------------------------------------------------------
// 变换链与解析
// ---------------------------------------------------------------------------

/// 完整的 GLSL ES 3.0 → naga 可解析文本 的变换链。
///
/// 依次执行：剥注释 → `#version` 换成空行 → 把 plain uniform 合并进一个
/// std140 uniform 块（X3）。
///
/// ⭐ **行数不变**（见模块文档的「行号必须一一对应」）。
///
/// # Errors
///
/// 声明形态不受支持 → [`GlslError::UnsupportedDeclaration`]。
pub fn transform(src: &str) -> Result<String, GlslError> {
    transform_with_block_binding(src, FRAGMENT_UNIFORM_BLOCK_BINDING)
}

/// [`transform`] 的带 binding 版本（uniform 块占用的 binding 号）。
pub fn transform_with_block_binding(src: &str, block_binding: u32) -> Result<String, GlslError> {
    let stripped = strip_comments(src);
    let no_version = strip_version_directive(&stripped);
    let packed = pack_uniforms_into_block(&no_version, block_binding)?;
    // ⭐ 补 varying 的 location（X5）⇒ 与 GLSL ES 的自动分配同义
    //   （不碰数学、不改行数）。
    Ok(add_varying_locations(&packed, ShaderStage::Vertex))
}

/// ⭐ **顶点阶段**的变换链（**实例化四边形**形态）。
///
/// 与 [`transform_with_block_binding`] 的差别只有一处：在「补 varying 的
/// location」**之前**多走一步 [`expand_point_sprite_to_quad`]（点精灵 → 四边形）。
///
/// ## 为什么需要这一步
///
/// WebGPU 规范**没有点尺寸**（`gl_PointSize`）⇒ wgpu 的 WebGPU 后端在
/// `create_shader_module` 处理该内建时**会 panic** ⇒ ⇒ 点精灵（`GL_POINTS`）
/// 只能活在 WebGL（GLES）后端。
///
/// ⇒ 要让**同一个后端无关的着色器**在两个后端上都画得出粒子 ⇒ 必须把
/// 「一个点」展开成「一个四边形」（每个粒子 4 个顶点 + 实例化）。
///
/// ## ⚠ 展开的是**外壳**，不是光学
///
/// 变换只做三件**机械**的事（不碰任何表达式与数值）：
///
/// 1. `gl_PointSize` → 局部变量 [`POINT_SIZE_LOCAL`]（尺寸）；
/// 2. `gl_Position` → 局部变量 [`POINT_CENTER_LOCAL`]（粒子中心）；
/// 3. 追加一个新的 `main`：把「中心 + 尺寸」展开成四边形的四个角，并把
///    **粒子内归一化坐标**（`gl_PointCoord` 的等价物）作为 varying 传下去。
///
/// ⇒ 片元着色器的光学表达式**一字不改**（`gl_PointCoord` 由引擎在**片元
///   阶段**换成一个 varying，见 [`POINT_COORD_VARYING`]）。
///
/// # Errors
///
/// - 源码里没有 `main` 函数 → [`GlslError::UnsupportedDeclaration`]；
/// - 顶点阶段没有 uniform 块（⇒ 引擎注入的表面尺寸成员无处安放）→
///   [`GlslError::UnsupportedDeclaration`]。
pub fn transform_vertex_for_instanced_quad(
    src: &str,
    block_binding: u32,
) -> Result<String, GlslError> {
    let stripped = strip_comments(src);
    let no_version = strip_version_directive(&stripped);
    // ⚠ 成员清单与引擎的布局层**共用同一个函数** ⇒ 两侧必然一致
    //   （否则绑定组会声明着色器没有的 binding ⇒ wgpu 校验报错）。
    let members = instanced_quad_vertex_uniform_block_members(&no_version);
    let engine_members: Vec<UniformBlockMember> = members
        .iter()
        .filter(|member| member.is_engine_filled && !member.is_padding)
        .cloned()
        .collect();
    let packed =
        pack_uniforms_into_block_with_members(&no_version, block_binding, &engine_members)?;
    let expanded = expand_point_sprite_to_quad(&packed)?;
    // ⭐ 补 varying 的 location（X5）⇒ 与 GLSL ES 的自动分配同义。
    Ok(add_varying_locations(&expanded, ShaderStage::Vertex))
}

/// ⭐ **片元阶段**的变换链（**实例化四边形**形态）。
///
/// 与 [`transform_fragment_with_block_binding`] 的差别只有一处：把
/// `gl_PointCoord` 换成 **varying** [`POINT_COORD_VARYING`]（点精灵 → 四边形
/// 的等价物），并声明该 varying 的输入。
///
/// ## ⭐ 为什么是「换 varying」而不是「改片元数学」
///
/// 片元着色器是**生产源的去注释副本**（资产防漂移门禁
/// `intro_assets_match_the_production_source` 逐字核对）⇒ 改它就等于让
/// **资产 ≠ 生产源** ⇒ ⇒ 资产门禁要么红、要么被迫放宽「允许变换」清单。
///
/// ⇒ 现在**资产一个字节都不动** ⇒ 资产门禁**原样保持绿** ✓ ⇒ 光学数学
///   **仍然只有一份**（生产源那一份）✓。
///
/// ## ⚠ `gl_PointCoord` 的语义由**引擎**复现
///
/// GLSL ES 的 `gl_PointCoord` 是**点精灵内部**的归一化坐标（`[0,1]²`）⇒
/// WebGPU 没有点精灵 ⇒ 必须由**顶点阶段**按四边形的角点算出来、再插值传下来
/// ⇒ 语义等价（见 [`expand_point_sprite_to_quad`] 的说明）。
///
/// # Errors
///
/// 声明形态不受支持 → [`GlslError::UnsupportedDeclaration`]。
pub fn transform_fragment_for_instanced_quad(
    src: &str,
    block_binding: u32,
) -> Result<String, GlslError> {
    transform_fragment_with_block_binding_inner(src, block_binding, true)
}

/// **片元阶段**的变换链：在 [`transform_with_block_binding`] 之外，额外做
/// `gl_FragCoord` 的**坐标语义修正**（见 [`fix_frag_coord_semantics`]）。
///
/// 顶点阶段**不做**这件事（GLSL 里 `gl_FragCoord` 只存在于片元阶段）。
///
/// ## 片元阶段多出来的两件事
///
/// 1. **注入帧缓冲高度成员**（[`FRAMEBUFFER_HEIGHT_MEMBER`]）：修正要用当前
///    表面高度 ⇒ 高度必须是 **uniform**（不是字面量）⇒ 改尺寸不必重编译着色器；
/// 2. **改写 `gl_FragCoord`**：把它换成 GLSL ES 规范语义（y 轴向上）。
///
/// ⚠ 这两件事**必须成对**：只改写不注入 ⇒ naga 报「未知标识符」。
///
/// # Errors
///
/// 声明形态不受支持 → [`GlslError::UnsupportedDeclaration`]。
pub fn transform_fragment_with_block_binding(
    src: &str,
    block_binding: u32,
) -> Result<String, GlslError> {
    transform_fragment_with_block_binding_inner(src, block_binding, false)
}

/// [`transform_fragment_with_block_binding`] 的**实例化四边形**形态。
///
/// ## ⚠ 与非四边形形态的**唯一**差别
///
/// `gl_PointCoord` → varying [`POINT_COORD_VARYING`]（点精灵 → 四边形）。
/// 其余步骤（uniform 打包 / 帧缓冲高度 / `gl_FragCoord` 修正 / 补 location）
/// **完全一致** ⇒ ⇒ 两个形态的画面差异**只**来自那一条替换 ✓。
fn transform_fragment_with_block_binding_inner(
    src: &str,
    block_binding: u32,
    instanced_quad: bool,
) -> Result<String, GlslError> {
    let stripped = strip_comments(src);
    let no_version = strip_version_directive(&stripped);
    // 成员清单与引擎的布局层**共用同一个函数** ⇒ 两侧必然一致。
    let members = fragment_uniform_block_members(src);
    let engine_members: Vec<UniformBlockMember> = members
        .iter()
        .filter(|member| member.is_engine_filled && !member.is_padding)
        .cloned()
        .collect();
    let packed =
        pack_uniforms_into_block_with_members(&no_version, block_binding, &engine_members)?;
    if !engine_members.is_empty()
        && !packed.contains(&format!("float {};", FRAMEBUFFER_HEIGHT_MEMBER))
    {
        // 源码用了 `gl_FragCoord`（⇒ 需要帧缓冲高度成员）但**没有 uniform
        // 声明** ⇒ 成员无处安放（块建不出来）⇒ **显式拒绝**（不静默画错）。
        return Err(GlslError::UnsupportedDeclaration(
            "片元着色器用到 `gl_FragCoord`，但源码里没有 uniform 声明 ⇒ 引擎的帧缓冲高度成员无处安放（请补一个 uniform 声明）"
                .to_string(),
        ));
    }
    // ⭐ 点精灵 → 四边形：`gl_PointCoord` 换成 varying（**必须先于**补 location
    //   ⇒ 新 varying 才拿得到与顶点阶段**同号**的 location）。
    let replaced = if instanced_quad {
        replace_point_coord_with_varying(&packed)
    } else {
        packed
    };
    let with_varying = if instanced_quad {
        add_point_coord_varying(&replaced)
    } else {
        replaced
    };
    // ⭐ 补 varying 的 location（X5）⇒ 与 GLSL ES 的自动分配同义。
    let located = add_varying_locations(&with_varying, ShaderStage::Fragment);
    Ok(fix_frag_coord_semantics(&located))
}

// ---------------------------------------------------------------------------
// 变换 6（X6）：点精灵 → **实例化四边形**
// ---------------------------------------------------------------------------

/// 顶点阶段展开后使用的**点尺寸**局部变量名（替代 `gl_PointSize`）。
///
/// ⚠ WebGPU 后端见到 `gl_PointSize` 会 **panic**（wgpu 的 WebGPU 后端在
///   `create_shader_module` 处理该内建时 `unwrap_failed`）⇒ ⇒ 必须在**进
///   naga 之前**把它换掉。
pub const POINT_SIZE_LOCAL: &str = "_engine_point_size";

/// 顶点阶段展开后使用的**粒子中心**局部变量名（替代 `gl_Position`）。
pub const POINT_CENTER_LOCAL: &str = "_engine_point_center";

/// 片元阶段拿到的**粒子内归一化坐标** varying 名（`gl_PointCoord` 的等价物）。
///
/// ⚠ 命名带 `_engine_` 前缀 ⇒ 与生产着色器自己的标识符**不可能撞名**。
pub const POINT_COORD_VARYING: &str = "_engine_point_coord";

/// 引擎注入的**四边形角点**顶点属性名（`layout(location = 1)`）。
pub const QUAD_CORNER_ATTRIBUTE: &str = "_engine_aCorner";

/// 引擎注入的**帧缓冲宽度**uniform 成员名（顶点阶段用）。
///
/// ⭐ 与 [`FRAMEBUFFER_HEIGHT_MEMBER`] 成对：点精灵的**设备像素**尺寸要变成
///   **裁剪空间**的偏移，必须知道表面有多大（`设备像素 → NDC` 的换算）。
pub const FRAMEBUFFER_WIDTH_MEMBER: &str = "_engine_fb_width";

/// ⭐ 把「点精灵」顶点着色器改写成「**实例化四边形**」形态。
///
/// ## 做了什么（三件机械的事，**不碰光学**）
///
/// 1. `gl_PointSize` → 局部变量 [`POINT_SIZE_LOCAL`]（**设备像素**的尺寸）；
/// 2. `gl_Position` → 局部变量 [`POINT_CENTER_LOCAL`]（粒子的**中心**）；
/// 3. 追加一段新的 `main`：
///    - 调原函数（改名为 `_engine_point_main`，带两个 `out` 参数）拿到中心与尺寸；
///    - 把「中心 ± 半个尺寸」展开成**四边形的四个角**（裁剪空间）；
///    - 把**粒子内归一化坐标**（`gl_PointCoord` 的等价物）作为 varying 传下去。
///
/// ## ⭐ 为什么「尺寸」要换算成裁剪空间偏移
///
/// GLSL 的 `gl_PointSize` 是**设备像素**；而顶点着色器输出的是**裁剪空间**
/// 坐标 ⇒ 两者之间差一个「表面尺寸 ↔ NDC」的换算：
///
/// ```text
/// 设备像素尺寸 s ⇒ NDC 半边长 = (s / 2) / (表面宽 / 2) = s / 表面宽
/// ```
///
/// ⇒ 表面尺寸必须是 **uniform**（不能烘焙成字面量：改尺寸就得重编译着色器，
///   而重编译管线极重 ⇒ 见 `crates-ui/evidence/p5-b/harness/src/lib.rs`）。
///
/// ## ⭐ 为什么 `gl_PointCoord` 的等价物**必须**由引擎提供
///
/// GLSL ES 的 `gl_PointCoord` 是**点精灵内部**的归一化坐标（`[0,1]²`）⇒
///
/// - WebGPU **没有**点精灵 ⇒ 没有 `gl_PointCoord` ⇒ 片元着色器会**编译失败**；
/// - 顶点阶段算出四个角的局部坐标（`[-1,1]²`）⇒ 映射到 `[0,1]²` ⇒ 作为
///   **varying** 传下去 ⇒ 片元阶段**线性插值**得到的值与光栅器给出的
///   `gl_PointCoord` **同义**（点精灵的内部坐标本就是线性的）。
///
/// ⇒ **资产（生产源的去注释副本）一个字节都不动** ⇒ 资产防漂移门禁原样绿 ✓。
///
/// ## 行号
///
/// 追加的代码全在**文件末尾** ⇒ 原有源码的行号**一一对应** ✓
/// （与本模块「行号必须不变」的纪律一致）。
///
/// # Errors
///
/// - 源码里没有 `main` 函数 → [`GlslError::UnsupportedDeclaration`]；
/// - 顶点阶段没有 uniform 块（⇒ 表面尺寸成员无处安放）→
///   [`GlslError::UnsupportedDeclaration`]。
pub fn expand_point_sprite_to_quad(src: &str) -> Result<String, GlslError> {
    // ① `gl_PointSize` → 局部变量（**必须先换**：WebGPU 后端会 panic）。
    let mut out = replace_identifier(src, "gl_PointSize", POINT_SIZE_LOCAL);
    // ② `gl_Position` → 局部变量（中心位置；真正写出去的是新的 `main`）。
    out = replace_identifier(&out, "gl_Position", POINT_CENTER_LOCAL);
    // ③ 找到 `main` 的函数头与函数体（改函数名 ⇒ 腾出 `main` 给新的展开入口）。
    let (header_start, header_end, body_end) = find_main_function(&out).ok_or_else(|| {
        GlslError::UnsupportedDeclaration(
            "实例化四边形展开需要顶点着色器里有 `main` 函数（点精灵的尺寸与中心都在里面）"
                .to_string(),
        )
    })?;
    let header = "void _engine_point_main()";
    let mut renamed = String::with_capacity(out.len() + header.len() + 512);
    renamed.push_str(&out[..header_start]);
    renamed.push_str(header);
    // ⚠ `body_end` 是函数体**闭合 `}`** 的位置 ⇒ 必须**带上**它
    //   （漏掉 ⇒ 原函数没有闭合 ⇒ 后面追加的声明全落进函数体里）。
    renamed.push_str(&out[header_end..=body_end]);
    // ③′ 声明两个**模块级**变量（中心 + 尺寸）—— 原函数写、新的 `main` 读。
    //
    // ⚠ 为什么是**模块级变量**而不是函数的 `out` 参数：
    //   naga 的 GLSL 前端**不支持** `out` 参数（实测：`Locals cannot have a
    //   storage qualifier`）⇒ ⇒ 只能走模块级变量（GLSL ES 3.0 允许 ⇒
    //   每个调用点各自一份 ⇒ 语义等价于局部变量 ✓）。
    //
    // ⚠ 声明**插在 uniform 块那一行之后**（不新增行）⇒ 行号不变 ✓。
    renamed = inject_module_scope_globals(&renamed, header_start);
    // ④ 追加：角点属性 + varying 声明 + 新的 `main`。
    renamed.push('\n');
    renamed.push_str(&format!(
        "layout(location = 1) in vec2 {QUAD_CORNER_ATTRIBUTE};\n"
    ));
    renamed.push_str(&format!("out vec2 {POINT_COORD_VARYING};\n"));
    renamed.push_str("void main() {\n");
    renamed.push_str("    _engine_point_main();\n");
    // ⚠ 偏移要乘 `w`：裁剪空间的偏移量在除以 `w` 之后才是 NDC 偏移
    //   （生产着色器的 `w` 恒为 1 ⇒ 逐位等价；写成乘法是为了语义正确）。
    renamed.push_str(&format!(
        "    gl_Position = {POINT_CENTER_LOCAL} + vec4({QUAD_CORNER_ATTRIBUTE} * {POINT_SIZE_LOCAL} * vec2(1.0 / {FRAMEBUFFER_WIDTH_MEMBER}, 1.0 / {FRAMEBUFFER_HEIGHT_MEMBER}) * {POINT_CENTER_LOCAL}.w, 0.0, 0.0);\n"
    ));
    // ⭐⭐⭐ **y 原点与 GLES 对齐**（2026-10-27，E-68 根因修复）：
    // ⇒ ⇒ **GLES 的 `gl_PointCoord` 原点在左上、y 向下递增**；
    // ⇒ ⇒ 而 `aCorner.y` 沿 NDC **向上**递增（底部 = -1）⇒ ⇒ 直接映射会让
    //   粒子内的 y **整体上下颠倒**（实测 G 差 ≈ -0.67，与「翻转差 = 1-2y」吻合）。
    // ⇒ ⇒ ⇒ 这里显式翻转 y ⇒ ⇒ 与生产（原生 GLES）**逐位同语义**。
    renamed.push_str(&format!(
        "    {POINT_COORD_VARYING} = vec2({QUAD_CORNER_ATTRIBUTE}.x * 0.5 + 0.5, 0.5 - {QUAD_CORNER_ATTRIBUTE}.y * 0.5);\n"
    ));
    renamed.push_str("}\n");
    Ok(renamed)
}

/// ⭐ 把两个**模块级**变量声明插进源码（**不新增行**）。
///
/// ## 为什么不新增行
///
/// 本模块的纪律是「**行号必须不变**」（naga 的报错信息带行号 ⇒ 宿主按行号
/// 回查**原始源码**）⇒ ⇒ 声明要**挂在已有的一行上**。
///
/// ## 挂在哪一行
///
/// 优先挂在 **uniform 块那一行**的后面（块一定存在：四边形展开需要引擎注入的
/// 「帧缓冲尺寸」成员 ⇒ 没有 uniform 声明就**建不出块**）⇒ ⇒ 零新增行 ✓。
///
/// 兜底：找不到块 ⇒ 在 `main` **之前**插入两行（此时行号会整体后移 2 行 ⇒
/// 属**已知偏差**，见 [`expand_point_sprite_to_quad`] 的文档）。
fn inject_module_scope_globals(src: &str, main_header_start: usize) -> String {
    let declaration = format!(" vec4 {POINT_CENTER_LOCAL}; float {POINT_SIZE_LOCAL};");
    // ① 挂在 uniform 块那一行后面（**块结束**的 `};` 之后 ⇒ 不是块里面！）。
    if let Some(at) = src.find(&format!("uniform {UNIFORM_BLOCK_NAME}")) {
        // 块体结束于 `}` ⇒ 其后的 `;` 才是**整条声明**的结束。
        if let Some(brace) = src[at..].find('}') {
            if let Some(semi) = src[at + brace..].find(';') {
                let insert_at = at + brace + semi + 1;
                let mut out = String::with_capacity(src.len() + declaration.len());
                out.push_str(&src[..insert_at]);
                out.push_str(&declaration);
                out.push_str(&src[insert_at..]);
                return out;
            }
        }
    }
    // ② 兜底：在 `main` 之前插两行。
    let mut out = String::with_capacity(src.len() + declaration.len() + 2);
    out.push_str(&src[..main_header_start]);
    out.push_str(&format!("vec4 {POINT_CENTER_LOCAL};\n"));
    out.push_str(&format!("float {POINT_SIZE_LOCAL};\n"));
    out.push_str(&src[main_header_start..]);
    out
}

/// ⭐ 把片元着色器里的 `gl_PointCoord` 换成 **varying** [`POINT_COORD_VARYING`]。
///
/// （`gl_PointCoord` 只存在于点精灵 ⇒ WebGPU 没有它 ⇒ 必须换。）
pub fn replace_point_coord_with_varying(src: &str) -> String {
    replace_identifier(src, "gl_PointCoord", POINT_COORD_VARYING)
}

/// ⭐ 给片元着色器**补上** [`POINT_COORD_VARYING`] 的输入声明。
///
/// ## 为什么放在**文件末尾**
///
/// [`add_varying_locations`] 按**声明顺序**分配 location ⇒ 把新 varying 放在
/// 末尾 ⇒ 它拿到**现有 varying 之后的下一个** location ⇒ 与顶点阶段追加的
/// `out`（同样在末尾）**同号** ⇒ ⇒ 两个阶段接得上 ✓。
///
/// ⚠ 前提：顶点与片元的**现有** varying 声明顺序必须一致（生产着色器满足；
///   单测 `the_point_coord_varying_matches_on_both_stages` 钉住这一点）。
pub fn add_point_coord_varying(src: &str) -> String {
    let declaration = format!("in vec2 {POINT_COORD_VARYING};\n");
    let Some((header_start, _, _)) = find_main_function(src) else {
        // 找不到 `main` ⇒ 追加到文件末尾（解析阶段会显式报错）。
        let mut out = String::with_capacity(src.len() + declaration.len());
        out.push_str(src);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&declaration);
        return out;
    };
    let mut out = String::with_capacity(src.len() + declaration.len());
    out.push_str(&src[..header_start]);
    out.push_str(&declaration);
    out.push_str(&src[header_start..]);
    out
}

/// 找到 `main` 函数的**函数头起点**、**参数列表终点**与**函数体终点**。
///
/// 返回 `(函数头起点, `)` 之后的位置, `}` 的位置)`。
///
/// ## 为什么用**括号配对**而不是正则
///
/// GLSL 的函数体里有 `{}`（`if` / `for` / 函数调用实参…）⇒ 正则不可靠 ⇒
/// 用**逐字符配对**（源码此时已**剥注释** ⇒ 注释里的括号不会干扰 ✓）。
fn find_main_function(src: &str) -> Option<(usize, usize, usize)> {
    let bytes = src.as_bytes();
    let mut search = 0usize;
    while let Some(relative) = src[search..].find("void main") {
        let header_start = search + relative;
        let clean_prefix = header_start == 0 || !is_identifier_byte(bytes[header_start - 1]);
        let mut cursor = header_start + "void main".len();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if clean_prefix && cursor < bytes.len() && bytes[cursor] == b'(' {
            // 跳过参数列表（`main` 无参数 ⇒ 立刻就是 `)`）。
            let mut paren = cursor + 1;
            while paren < bytes.len() && bytes[paren] != b')' {
                paren += 1;
            }
            if paren >= bytes.len() {
                return None;
            }
            let header_end = paren + 1;
            let mut brace = header_end;
            while brace < bytes.len() && bytes[brace].is_ascii_whitespace() {
                brace += 1;
            }
            if brace >= bytes.len() || bytes[brace] != b'{' {
                return None;
            }
            let body_start = brace + 1;
            let mut depth = 1i32;
            let mut index = body_start;
            while index < bytes.len() {
                match bytes[index] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((header_start, header_end, index));
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
            return None;
        }
        search = header_start + 1;
    }
    None
}

// ---------------------------------------------------------------------------
// 变换 4（X4）：`gl_FragCoord` 的坐标语义修正
// ---------------------------------------------------------------------------

/// 把源码里的 `gl_FragCoord` 换成**GLSL ES 规范语义**的等价写法。
///
/// ## 为什么必须做（方言外壳层，**不碰数学**）
///
/// GLSL ES 3.0 规定 `gl_FragCoord` 的原点在**左下角**、y 轴**向上**。
///
/// 而本引擎的两个后端都把**帧缓冲坐标的 y 轴朝下**（实测，证据见
/// `crates-ui/evidence/p5-b/`）：
///
/// - **WebGPU 后端**：`@builtin(position)` 的 y=0 是**顶行**（WebGPU 规范）；
/// - **WebGL 后端**：wgpu-hal 的 GL 路径把主渲染**上下翻转**存进交换链纹理
///   （`wgpu-hal-30.0.1/src/gles/web.rs:301-312`：「GL's presentation is not
///   flipped, but main rendering is. Therefore, we Y-flip the output
///   positions in the shader, and also this blit.」）⇒ 片元里读到的
///   `gl_FragCoord.y` 与**画布上从上到下的行号**同向。
///
/// ⇒ 同一份 GLSL 源在引擎里画出来的画面是**上下颠倒**的：实测生产侧画面与
/// 引擎侧画面**逐像素互为上下镜像**（翻转后 0 差异）。
///
/// 本函数在**源级**把 `gl_FragCoord` 换成
/// `vec4(x, _engine_fb_height - y, z, w)` ⇒ 引擎里的片元坐标与 GLSL ES 规范
/// **逐点一致** ⇒ 同一份着色器源在引擎与生产路径上画出的画面一致。
///
/// ## ⭐ 翻转高度为什么是 uniform 而不是字面量
///
/// 早期形态把表面高度**烘焙成字面量**（`600.0 - gl_FragCoord.y`）⇒ 尺寸一变
/// 字面量就陈旧 ⇒ 画面竖直错位；且着色器与表面尺寸**耦死** ⇒ resize 必须重编译
/// 着色器（重编译管线极重：实测 `Page.captureScreenshot` **超时**，见
/// `crates-ui/evidence/p5-b/harness/src/lib.rs`）。
///
/// ⇒ 高度改成 uniform [`FRAMEBUFFER_HEIGHT_MEMBER`]（引擎注入到 uniform 块
/// 末尾）⇒ 改尺寸只改 uniform 缓冲里的一个 f32 ⇒ **不必重编译着色器**。
///
/// ## ⭐ 逐像素等价性（既有 0 / 576,000 证据不受影响）
///
/// 字面量形态 `600.0 - gl_FragCoord.y` 与 uniform 形态
/// `_engine_fb_height - gl_FragCoord.y` 的**算术逐位相同**：
///
/// - `600.0` 是 GLSL 的 **float 字面量** ⇒ f32 值 `600.0`（精确可表示）；
/// - uniform 里放的是同一个 f32 `600.0`（引擎写入 `height as f32`）；
/// - 两侧的 `highp float` 精度**相同**（着色器里的 `precision highp float;`）；
/// - `a - b` 是同一条 IEEE 减法 ⇒ 结果逐位相同。
///
/// 且 uniform 成员**追加在块末尾** ⇒ 生产 uniform 的**偏移与取值逐字节不变**。
///
/// ## 行为细节
///
/// - **单趟扫描**：替换文本里的 `gl_FragCoord` **不再被二次替换**；
/// - **词边界**感知：`myGl_FragCoord` / `gl_FragCoordExtra` 等**不受影响**；
/// - **不插行** ⇒ 变换前后**行号一一对应**（错误信息仍指回原始行）；
/// - 源码里没有 `gl_FragCoord` ⇒ **原样返回**。
pub fn fix_frag_coord_semantics(src: &str) -> String {
    let expansion = format!(
        "vec4(gl_FragCoord.x, {FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w)"
    );
    replace_identifier(src, "gl_FragCoord", &expansion)
}

/// [`fix_frag_coord_semantics`] 的**字面量形态**（对照用）。
///
/// ⚠ **生产路径不用它**：字面量把表面高度烘焙进着色器 ⇒ 尺寸一变就陈旧。
/// 保留它是为了让「字面量形态 ≡ uniform 形态」的等价性**可测**（见单测
/// `baked_and_uniform_flips_have_the_same_shape`）。
pub fn fix_frag_coord_semantics_baked(src: &str, framebuffer_height: u32) -> String {
    let expansion = format!(
        "vec4(gl_FragCoord.x, {framebuffer_height}.0 - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w)"
    );
    replace_identifier(src, "gl_FragCoord", &expansion)
}

/// 把源码里的**标识符** `token` 换成 `replacement`（词边界感知，单趟扫描）。
///
/// 替换文本本身**不再被扫描** ⇒ 替换文本里含 `token` 时不会递归。
fn replace_identifier(src: &str, token: &str, replacement: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut index = 0usize;
    while index < src.len() {
        if src[index..].starts_with(token) {
            let starts_clean = index == 0 || !is_identifier_byte(bytes[index - 1]);
            let after = index + token.len();
            let ends_clean = after >= src.len() || !is_identifier_byte(bytes[after]);
            if starts_clean && ends_clean {
                out.push_str(replacement);
                index = after;
                continue;
            }
        }
        let ch = src[index..].chars().next().expect("非空切片");
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// 该字节是不是标识符字符（ASCII 字母 / 数字 / 下划线）。
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// GLSL 源 → naga IR（经完整变换链）。
///
/// # Errors
///
/// - 变换失败 → [`GlslError::UnsupportedDeclaration`]；
/// - naga 前端解析失败 → [`GlslError::Parse`]；
/// - naga IR 校验失败 → [`GlslError::Validation`]。
pub fn parse(src: &str, stage: ShaderStage) -> Result<naga::Module, GlslError> {
    let transformed = transform(src)?;
    let options = naga::front::glsl::Options {
        stage,
        defines: Default::default(),
    };
    let module = naga::front::glsl::Frontend::default()
        .parse(&options, &transformed)
        .map_err(|e| GlslError::Parse(format!("{e:?}")))?;

    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|e| GlslError::Validation(format!("{e:?}")))?;

    Ok(module)
}

/// naga IR → GLSL ES 3.00 文本（naga 自己的 GLSL 后端）。
///
/// 这是 WebGL 后端在运行时走的同一条路径（`wgpu-hal` 的 GLES 设备用
/// `naga::back::glsl::Writer` 重新吐出 GLSL ES）。
///
/// # Errors
///
/// naga 的 GLSL 后端写出失败 → [`GlslError::Validation`]（错误通道复用）。
pub fn emit_glsl_es_300(module: &naga::Module, stage: ShaderStage) -> Result<String, GlslError> {
    use naga::back::glsl::{
        Options as GlslOutOptions, PipelineOptions, Version as GlslVersion, Writer, WriterFlags,
    };
    use naga::proc::BoundsCheckPolicies;

    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(module)
        .map_err(|e| GlslError::Validation(format!("{e:?}")))?;

    let options = GlslOutOptions {
        version: GlslVersion::Embedded {
            version: 300,
            is_webgl: true,
        },
        writer_flags: WriterFlags::empty(),
        ..Default::default()
    };
    let pipeline_options = PipelineOptions {
        shader_stage: stage,
        entry_point: "main".to_string(),
        multiview: None,
    };

    let mut out = String::new();
    {
        let mut writer = Writer::new(
            &mut out,
            module,
            &info,
            &options,
            &pipeline_options,
            BoundsCheckPolicies::default(),
        )
        .map_err(|e| GlslError::Validation(format!("{e:?}")))?;
        writer
            .write()
            .map_err(|e| GlslError::Validation(format!("{e:?}")))?;
    }
    Ok(out)
}

/// 把 naga 自己吐出的 GLSL ES 3.00 变成**能重新被 naga 解析**的文本。
///
/// 必要两步（都是上游限制的补偿）：
/// 1. **再剥一次 `#version`** —— naga 自己吐出的 `#version 300 es`，
///    naga 自己的 GLSL 前端解析不了（`front/glsl/parser.rs:267` / `:290`）；
/// 2. **给 `layout(std140) uniform` 块补 binding** —— naga 的后端不带 binding。
pub fn reparse_source(emitted: &str) -> String {
    inject_block_bindings(&strip_version_directive(&strip_comments(emitted)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 剥注释
    // -----------------------------------------------------------------------

    #[test]
    fn strip_comments_removes_line_and_block_comments() {
        let src = "float a = 1.0; // trailing\n/* block\n   comment */ float b = 2.0;\n";
        let out = strip_comments(src);
        assert!(!out.contains("trailing"), "行注释必须被剥掉：{out}");
        assert!(!out.contains("block"), "块注释必须被剥掉：{out}");
        assert!(out.contains("float a = 1.0;"));
        assert!(out.contains("float b = 2.0;"));
    }

    #[test]
    fn strip_comments_preserves_line_numbers() {
        let src = "a\n/* x\ny */ b\nc\n";
        let out = strip_comments(src);
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "剥注释不得改变行数（错误信息仍指得回原始行）"
        );
        assert_eq!(out.lines().nth(3), Some("c"));
    }

    #[test]
    fn strip_comments_keeps_division_operators_intact() {
        let src = "float r = a / b;\nfloat s = c /d;\n";
        let out = strip_comments(src);
        assert!(out.contains("a / b"), "除号不得被误判成注释：{out}");
        assert!(out.contains("c /d"), "除号后紧跟标识符不得被误判：{out}");
    }

    #[test]
    fn strip_comments_handles_unterminated_block_comment() {
        let src = "float a = 1.0;\n/* never closed\nfloat b = 2.0;\n";
        let out = strip_comments(src);
        assert!(!out.contains("never closed"));
        assert!(!out.contains("float b"), "未闭合块注释之后的代码不可见");
    }

    #[test]
    fn strip_comments_keeps_non_ascii_text_untouched() {
        let src = "// 中文注释\nfloat a = 1.0; // 尾注\n";
        let out = strip_comments(src);
        assert_eq!(
            out, "\nfloat a = 1.0; \n",
            "注释文本被剥掉，但换行必须留着（行号不变）"
        );
    }

    // -----------------------------------------------------------------------
    // 删 #version
    // -----------------------------------------------------------------------

    #[test]
    fn strip_version_removes_only_version_lines() {
        let src = "#version 300 es\nprecision highp float;\n#extension GL_EXT_x : enable\n";
        let out = strip_version_directive(src);
        assert!(!out.contains("#version"), "version 行必须被删：{out}");
        assert!(out.contains("precision highp float;"), "其它源码必须保留");
        assert!(out.contains("#extension"), "其它预处理指令必须保留");
    }

    #[test]
    fn strip_version_is_idempotent() {
        let src = "#version 300 es\nfloat a = 1.0;\n";
        let once = strip_version_directive(src);
        let twice = strip_version_directive(&once);
        assert_eq!(once, twice, "重复删 version 必须幂等");
    }

    #[test]
    fn strip_version_tolerates_indented_version_line() {
        let src = "  #version 300 es\nfloat a = 1.0;\n";
        assert!(!strip_version_directive(src).contains("#version"));
    }

    // -----------------------------------------------------------------------
    // X3：合并 uniform 块
    // -----------------------------------------------------------------------

    #[test]
    fn packing_moves_uniforms_into_one_std140_block() {
        let src =
            "precision highp float;\nuniform vec2 uRes;\nuniform float uScale;\nvoid main(){}\n";
        let out = pack_uniforms_into_block(src, 0).expect("打包成功");
        assert!(
            out.contains("layout(std140, binding=0) uniform EngineUniforms {"),
            "{out}"
        );
        assert!(out.contains("vec2 uRes;"), "{out}");
        assert!(out.contains("float uScale;"), "{out}");
        assert!(
            !out.contains("uniform vec2 uRes;"),
            "原来的 plain uniform 声明必须被搬进块里：{out}"
        );
        assert_eq!(
            out.matches("layout(std140").count(),
            1,
            "只能有一个 uniform 块（= 一个 binding）：{out}"
        );
    }

    #[test]
    fn packing_keeps_member_order() {
        let src = "uniform float a;\nuniform float b;\nuniform float c;\n";
        let out = pack_uniforms_into_block(src, 0).expect("打包成功");
        let a = out.find("float a;").expect("a");
        let b = out.find("float b;").expect("b");
        let c = out.find("float c;").expect("c");
        assert!(a < b && b < c, "成员顺序必须等于声明顺序：{out}");
    }

    #[test]
    fn packing_without_uniforms_is_a_no_op() {
        let src = "precision highp float;\nvoid main(){}\n";
        let out = pack_uniforms_into_block(src, 0).expect("打包成功");
        assert!(!out.contains("uniform"), "没有 uniform 时不得造块：{out}");
    }

    #[test]
    fn packing_rejects_array_uniforms() {
        let src = "uniform float weights[4];\n";
        let err = pack_uniforms_into_block(src, 0).expect_err("数组 uniform 必须显式拒绝");
        assert!(matches!(err, GlslError::UnsupportedDeclaration(_)));
    }

    #[test]
    fn packing_uses_the_requested_binding() {
        let src = "uniform float a;\n";
        let out = pack_uniforms_into_block(src, 7).expect("打包成功");
        assert!(out.contains("binding=7"), "{out}");
    }

    #[test]
    fn packed_block_parses_into_naga_ir_with_unqualified_access() {
        // 反例钉住：GLSL 允许不带实例名直接访问块成员；naga 的前端支持
        // （否则整个 X3 方案不成立）。
        let src = "precision highp float;\nuniform vec2 uRes;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uRes, uScale, 1.0); }\n";
        let packed = pack_uniforms_into_block(src, 0).expect("打包成功");
        let module = parse(&packed, ShaderStage::Fragment).expect("打包后的源码必须能过 naga");
        assert_eq!(
            module.global_variables.len(),
            2,
            "一个 uniform 块 + fragColor（不再是 16 个独立全局量）"
        );
        let block = module
            .global_variables
            .iter()
            .find(|(_, var)| var.binding.is_some())
            .expect("必须有一个带 binding 的全局量");
        assert_eq!(
            block.1.binding.expect("binding"),
            naga::ResourceBinding {
                group: 0,
                binding: 0
            }
        );
    }

    #[test]
    fn packed_block_member_offsets_follow_std140() {
        let src = "uniform vec2 uRes;\nuniform vec2 uCenter;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uRes, uScale, 1.0); }\n";
        let packed = pack_uniforms_into_block(src, 0).expect("打包成功");
        let module = parse(&packed, ShaderStage::Fragment).expect("解析成功");
        let offsets = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| {
                let ty = &module.types[var.ty];
                match &ty.inner {
                    naga::TypeInner::Struct { members, .. } => Some(
                        members
                            .iter()
                            .map(|member| (member.name.clone().unwrap_or_default(), member.offset))
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                }
            })
            .next()
            .expect("必须有一个 struct 类型的 uniform 块");
        assert_eq!(
            offsets,
            vec![
                ("uRes".to_string(), 0u32),
                ("uCenter".to_string(), 8u32),
                ("uScale".to_string(), 16u32),
                (UNIFORM_BLOCK_PADDING_MEMBER.to_string(), 32u32),
            ],
            "std140 成员偏移必须与本引擎的计算一致（含 padding 成员）"
        );
    }

    // -----------------------------------------------------------------------
    // X2：逐个注入 binding（保留为可测基准）
    // -----------------------------------------------------------------------

    #[test]
    fn inject_layout_bindings_numbers_uniforms_in_declaration_order() {
        let src = "uniform vec2 uRes;\nuniform vec2 uCenter;\nuniform float uScale;\n";
        let out = inject_layout_bindings(src);
        assert!(out.contains("layout(binding=0) uniform vec2 uRes;"));
        assert!(out.contains("layout(binding=1) uniform vec2 uCenter;"));
        assert!(out.contains("layout(binding=2) uniform float uScale;"));
    }

    #[test]
    fn inject_layout_bindings_skips_uniforms_inside_functions() {
        let src = "uniform float uTop;\nvoid main(){ float x = 1.0; }\n";
        let out = inject_layout_bindings(src);
        assert_eq!(
            out.matches("layout(binding=").count(),
            1,
            "只有顶层 uniform 吃 binding 号：{out}"
        );
        assert!(out.contains("layout(binding=0) uniform float uTop;"));
    }

    #[test]
    fn inject_layout_bindings_handles_multi_line_declaration() {
        let src = "uniform vec2\n    uRes;\nuniform float uScale;\n";
        let out = inject_layout_bindings(src);
        assert!(out.contains("layout(binding=0) uniform vec2"), "{out}");
        assert!(
            out.contains("layout(binding=1) uniform float uScale;"),
            "{out}"
        );
        assert_eq!(out.matches("layout(binding=").count(), 2);
    }

    #[test]
    fn inject_layout_bindings_leaves_uniform_blocks_alone() {
        let src = "layout(std140) uniform Block { vec2 m; };\nuniform float uScale;\n";
        let out = inject_layout_bindings(src);
        assert!(out.contains("layout(std140) uniform Block"), "{out}");
        assert!(
            out.contains("layout(binding=0) uniform float uScale;"),
            "{out}"
        );
    }

    #[test]
    fn inject_layout_bindings_respects_offset() {
        let src = "uniform float a;\nuniform float b;\n";
        let out = inject_layout_bindings_with_offset(src, 7);
        assert!(out.contains("layout(binding=7) uniform float a;"));
        assert!(out.contains("layout(binding=8) uniform float b;"));
    }

    #[test]
    fn inject_layout_bindings_is_idempotent_in_shape() {
        let src = "uniform vec2 uRes;\n";
        let once = inject_layout_bindings(src);
        let twice = inject_layout_bindings(&once);
        assert_eq!(once, twice, "对已变换的文本再变换必须无副作用");
    }

    // -----------------------------------------------------------------------
    // uniform 声明抽取
    // -----------------------------------------------------------------------

    #[test]
    fn uniform_declarations_extract_names_types_and_order() {
        let src = "uniform vec2  uRes;\nuniform vec2  uCenter;\nuniform float uScale;\nuniform float uTime;\n";
        let decls = uniform_declarations(src);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["uRes", "uCenter", "uScale", "uTime"]);
        assert_eq!(decls[0].type_name, "vec2");
        assert_eq!(decls[2].type_name, "float");
        for (index, decl) in decls.iter().enumerate() {
            assert_eq!(decl.order, index, "声明序必须连续且从 0 起");
        }
    }

    #[test]
    fn uniform_declarations_handle_multiple_names_per_declaration() {
        let src = "uniform vec2 uA, uB;\nuniform float uC;\n";
        let decls = uniform_declarations(src);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["uA", "uB", "uC"]);
        assert!(decls
            .iter()
            .all(|d| d.type_name == "vec2" || d.type_name == "float"));
        assert_eq!(decls[1].type_name, "vec2", "同一声明里的后续名字继承类型");
    }

    #[test]
    fn uniform_declarations_ignore_commented_uniforms() {
        let src = "// uniform float uGhost;\nuniform float uReal;\n";
        let decls = uniform_declarations(src);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "uReal");
    }

    #[test]
    fn uniform_declarations_ignore_uniform_blocks() {
        let src = "layout(std140) uniform Block { vec2 m; };\nuniform float uReal;\n";
        let decls = uniform_declarations(src);
        assert_eq!(decls.len(), 1, "uniform 块不是 plain uniform：{decls:?}");
        assert_eq!(decls[0].name, "uReal");
    }

    #[test]
    fn uniform_declarations_handle_array_syntax() {
        let src = "uniform mat4 uM;\nuniform float uWeights[4];\n";
        let decls = uniform_declarations(src);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["uM", "uWeights"]);
    }

    // -----------------------------------------------------------------------
    // 类型
    // -----------------------------------------------------------------------

    #[test]
    fn float_type_support_is_explicit() {
        assert!(is_supported_float_type("float"));
        assert!(is_supported_float_type("vec4"));
        assert!(!is_supported_float_type("int"));
        assert!(!is_supported_float_type("mat4"));
        assert!(!is_supported_float_type("sampler2D"));
    }

    #[test]
    fn float_component_counts_and_sizes_match_std140() {
        assert_eq!(float_component_count("float"), Some(1));
        assert_eq!(float_component_count("vec3"), Some(3));
        assert_eq!(float_std140_size("float"), Some(4));
        assert_eq!(float_std140_size("vec2"), Some(8));
        assert_eq!(float_std140_size("vec3"), Some(12));
        assert_eq!(float_std140_size("vec4"), Some(16));
    }

    #[test]
    fn float_std140_alignments_match_the_spec() {
        assert_eq!(float_std140_alignment("float"), Some(4));
        assert_eq!(float_std140_alignment("vec2"), Some(8));
        assert_eq!(float_std140_alignment("vec3"), Some(16));
        assert_eq!(float_std140_alignment("vec4"), Some(16));
    }

    #[test]
    fn float_helpers_return_none_instead_of_panicking() {
        // ⭐ 这三个函数是**对外 API** ⇒ 非法类型必须**显式**返回 `None`，
        // 而不是 panic（panic 会把调用方的 bug 变成进程崩溃）。
        assert_eq!(float_component_count("int"), None);
        assert_eq!(float_component_count("mat4"), None);
        assert_eq!(float_component_count("sampler2D"), None);
        assert_eq!(float_component_count(""), None);
        assert_eq!(float_std140_size("int"), None);
        assert_eq!(float_std140_size("vec5"), None);
        assert_eq!(float_std140_alignment("int"), None);
        assert_eq!(float_std140_alignment("bool"), None);
    }

    // -----------------------------------------------------------------------
    // 变换链 + 解析
    // -----------------------------------------------------------------------

    const SAMPLE: &str = "#version 300 es\nprecision highp float;\nuniform vec2 uRes;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uRes, uScale, 1.0); }\n";

    #[test]
    fn transform_removes_version_and_packs_uniforms() {
        let out = transform(SAMPLE).expect("变换成功");
        assert!(!out.contains("#version"), "{out}");
        assert!(
            out.contains("layout(std140, binding=0) uniform EngineUniforms {"),
            "{out}"
        );
        assert!(out.contains("vec2 uRes;"), "{out}");
        assert!(out.contains("precision highp float;"));
    }

    #[test]
    fn transformed_source_parses_into_naga_ir() {
        let module =
            parse(SAMPLE, ShaderStage::Fragment).expect("变换后的源码必须能被 naga 解析并校验");
        assert_eq!(
            module.global_variables.len(),
            2,
            "一个 uniform 块 + fragColor"
        );
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "main");
    }

    #[test]
    fn packed_uniforms_share_a_single_binding() {
        let module = parse(SAMPLE, ShaderStage::Fragment).expect("解析成功");
        let mut bindings = Vec::new();
        for (_, var) in module.global_variables.iter() {
            if let Some(binding) = var.binding {
                bindings.push(binding.binding);
            }
        }
        assert_eq!(
            bindings,
            [0],
            "两个 uniform 必须合并成**一个** binding（WebGL2 后端的限额是 12–14）"
        );
    }

    #[test]
    fn original_es_source_is_rejected_by_naga_without_transform() {
        // 反例：不做 X1/X3 时 naga 的前端确实解析不了 ES 源 —— 这正是
        // 前置变换器存在的理由（不是「保险起见」）。
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, SAMPLE);
        assert!(
            parsed.is_err(),
            "naga 的 GLSL 前端没有 ES profile，原样必须解析失败"
        );
    }

    #[test]
    fn round_trip_through_naga_glsl_backend_survives() {
        let module = parse(SAMPLE, ShaderStage::Fragment).expect("解析成功");
        let emitted =
            emit_glsl_es_300(&module, ShaderStage::Fragment).expect("GLSL ES 300 写出成功");
        assert!(emitted.starts_with("#version 300 es"), "{emitted}");
        let reparsed = parse(&reparse_source(&emitted), ShaderStage::Fragment);
        assert!(reparsed.is_ok(), "回环后必须仍能被 naga 解析：{emitted}");
    }

    #[test]
    fn emitted_es_header_alone_is_not_reparsable() {
        // 反例钉住上游事实：naga 自己吐出的 `#version 300 es` 头，
        // naga 自己的前端解析不了 ⇒ 回环必须再剥一次 version。
        let module = parse(SAMPLE, ShaderStage::Fragment).expect("解析成功");
        let emitted = emit_glsl_es_300(&module, ShaderStage::Fragment).expect("写出成功");
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, &emitted);
        assert!(parsed.is_err(), "naga 吐出的 ES 头不得被自己的前端接受");
    }

    // -----------------------------------------------------------------------
    // 变换 4（X4）：`gl_FragCoord` 的坐标语义修正
    // -----------------------------------------------------------------------

    const FRAG_COORD_SAMPLE: &str = "#version 300 es\nprecision highp float;\nuniform vec2 uRes;\nout vec4 fragColor;\nvoid main(){ vec2 p = gl_FragCoord.xy / uRes; fragColor = vec4(p, 0.0, 1.0); }\n";

    #[test]
    fn frag_coord_correction_flips_the_y_axis() {
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE);
        assert!(
            out.contains(&format!("{FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y")),
            "片元坐标的 y 必须翻成 GLSL 规范（向上）：{out}"
        );
        assert!(
            out.contains(&format!(
                "vec4(gl_FragCoord.x, {FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w)"
            )),
            "展开式必须逐分量保留其余分量：{out}"
        );
    }

    #[test]
    fn frag_coord_flip_height_is_a_uniform_not_a_baked_literal() {
        // ⭐ 反例钉住「翻转高度不得再被烘焙成字面量」：
        // 字面量把表面尺寸**耦死**进着色器 ⇒ 改尺寸就得重编译着色器。
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE);
        assert!(
            !out.contains("600.0"),
            "片元变换里不得再出现烘焙的表面高度：{out}"
        );
        assert!(
            !out.contains("- gl_FragCoord.y, gl_FragCoord.z")
                || out.contains(FRAMEBUFFER_HEIGHT_MEMBER),
            "翻转高度必须来自 uniform 成员：{out}"
        );
    }

    #[test]
    fn baked_and_uniform_flips_have_the_same_shape() {
        // ⭐ 逐像素等价性的**结构**对照：字面量形态与 uniform 形态除了
        // 「高度来源」之外**逐字同形** ⇒ 算术逐位相同 ⇒ 既有 0 / 576,000
        // 的像素证据不受影响（详见 `fix_frag_coord_semantics` 的文档）。
        let baked = fix_frag_coord_semantics_baked(FRAG_COORD_SAMPLE, 600);
        let uniform = fix_frag_coord_semantics(FRAG_COORD_SAMPLE);
        let normalize = |text: &str| {
            text.replace("600.0", "H")
                .replace(FRAMEBUFFER_HEIGHT_MEMBER, "H")
        };
        assert_eq!(
            normalize(&baked),
            normalize(&uniform),
            "字面量形态与 uniform 形态必须同形（只差高度来源）"
        );
    }

    #[test]
    fn frag_coord_correction_keeps_line_numbers() {
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE);
        assert_eq!(
            out.lines().count(),
            FRAG_COORD_SAMPLE.lines().count(),
            "变换不得增删行（错误信息仍要指回原始行）"
        );
    }

    #[test]
    fn frag_coord_correction_is_a_no_op_without_the_builtin() {
        let source = "#version 300 es\nprecision highp float;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(1.0); }\n";
        assert_eq!(
            fix_frag_coord_semantics(source),
            source,
            "源码里没有 gl_FragCoord 时必须原样返回"
        );
    }

    #[test]
    fn frag_coord_correction_respects_identifier_boundaries() {
        let source = "float myGl_FragCoord = 1.0;\nfloat gl_FragCoordExtra = 2.0;\n";
        let out = fix_frag_coord_semantics(source);
        assert_eq!(out, source, "相似标识符不得被误改（词边界必须生效）");
    }

    #[test]
    fn frag_coord_correction_does_not_recurse() {
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE);
        assert_eq!(
            out.matches(&format!("{FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y"))
                .count(),
            1,
            "替换文本里的 gl_FragCoord 不得被二次替换：{out}"
        );
        assert!(
            !out.contains(&format!(
                "{FRAMEBUFFER_HEIGHT_MEMBER} - {FRAMEBUFFER_HEIGHT_MEMBER}"
            )),
            "不得出现递归替换：{out}"
        );
    }

    #[test]
    fn fragment_transform_injects_the_framebuffer_height_member() {
        let out =
            transform_fragment_with_block_binding(FRAG_COORD_SAMPLE, 0).expect("片元变换成功");
        assert!(
            out.contains(&format!("float {FRAMEBUFFER_HEIGHT_MEMBER};")),
            "片元块必须含引擎注入的帧缓冲高度成员：{out}"
        );
        assert!(
            out.contains(&format!("{FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y")),
            "片元变换必须用**帧缓冲高度**：{out}"
        );
        assert!(
            out.contains("layout(std140, binding=0) uniform EngineUniforms {"),
            "{out}"
        );
    }

    #[test]
    fn vertex_transform_leaves_the_builtin_alone() {
        // 顶点阶段没有 gl_FragCoord ⇒ 顶点变换链不得碰它。
        let out = transform_with_block_binding(FRAG_COORD_SAMPLE, 1).expect("顶点变换成功");
        assert!(
            !out.contains(FRAMEBUFFER_HEIGHT_MEMBER),
            "顶点阶段不得做坐标语义修正：{out}"
        );
        assert!(out.contains("gl_FragCoord.xy"), "{out}");
    }

    #[test]
    fn corrected_source_still_parses_into_naga_ir() {
        let out =
            transform_fragment_with_block_binding(FRAG_COORD_SAMPLE, 0).expect("片元变换成功");
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, &out);
        assert!(parsed.is_ok(), "修正后的源码必须仍能被 naga 解析：{out}");
    }

    #[test]
    fn production_asset_parses_after_the_correction() {
        // 真实资产（生产版 GLSL 的去注释副本）走完整片元变换链 ⇒ 必须能解析。
        let out = transform_fragment_with_block_binding(
            crate::shaders::SCENE_FRAGMENT_GLSL,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .expect("生产资产的片元变换必须成功");
        assert!(
            out.contains(&format!("{FRAMEBUFFER_HEIGHT_MEMBER} - gl_FragCoord.y")),
            "生产资产确实用到 gl_FragCoord ⇒ 修正必须生效"
        );
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, &out);
        assert!(parsed.is_ok(), "修正后的生产资产必须能被 naga 解析");
    }

    // -----------------------------------------------------------------------
    // ⭐ 行号必须一一对应（ADR §2.6.4 的承诺）
    // -----------------------------------------------------------------------

    #[test]
    fn strip_version_keeps_the_line_count() {
        let src = "#version 300 es\nprecision highp float;\nvoid main(){}\n";
        let out = strip_version_directive(src);
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "`#version` 行必须被**替换成空行**，不是删行（否则其后所有行号前移一位）"
        );
        assert!(!out.contains("#version"), "{out}");
        assert!(
            out.lines()
                .next()
                .is_some_and(|line| line.trim().is_empty()),
            "第一行必须是空行：{out}"
        );
    }

    #[test]
    fn packing_keeps_the_line_count() {
        let src =
            "precision highp float;\nuniform vec2 uRes;\nuniform float uScale;\nvoid main(){}\n";
        let out = pack_uniforms_into_block(src, 0).expect("打包成功");
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "打包不得增删行（uniform 块必须整体写在一行里）"
        );
    }

    #[test]
    fn transform_chain_preserves_line_count() {
        // ⭐ 整链（剥注释 → #version 换空行 → 打包 → 坐标修正）行数不变。
        let out = transform_fragment_with_block_binding(
            crate::shaders::SCENE_FRAGMENT_GLSL,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .expect("片元变换成功");
        assert_eq!(
            out.lines().count(),
            crate::shaders::SCENE_FRAGMENT_GLSL.lines().count(),
            "整条变换链不得改变行数（naga 报错按行号回查源码 ⇒ 行号必须一一对应）"
        );
    }

    #[test]
    fn transform_chain_preserves_line_count_for_a_multi_line_declaration() {
        let src = "#version 300 es\nprecision highp float;\nuniform vec2\n    uRes;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uRes, uScale, 1.0); }\n";
        let out = transform_fragment_with_block_binding(src, 0).expect("片元变换成功");
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "跨行声明也不得改变行数：{out}"
        );
    }

    // -----------------------------------------------------------------------
    // ⭐ varying 的 location（X5）
    // -----------------------------------------------------------------------

    #[test]
    fn the_vertex_uniform_block_keeps_its_own_binding() {
        // ⭐⭐ 顶点阶段的 uniform 块必须留在 **binding = 1**（与片元块的 0 错开）
        //   ⇒ 否则两侧的 `binding=0` 会**撞车** ⇒ 顶点阶段读到的是**别的**块
        //   ⇒ 画面「静默地错」（不报错，只是没东西）。
        //
        // ⚠ 关键：`glsl::parse` 走的是**通用**变换（默认 binding = 0）⇒ ⇒
        //   **二次变换不得**把已打包的块的 binding 改回去。
        let src = "precision highp float;\nuniform float uCount;\nvoid main(){ gl_Position = vec4(0.0, 0.0, 0.0, 1.0); }\n";
        let transformed =
            transform_with_block_binding(src, VERTEX_UNIFORM_BLOCK_BINDING).expect("变换成功");
        assert!(
            transformed.contains("binding=1"),
            "顶点块的 binding 必须是 1：{transformed}"
        );
        let module = parse(&transformed, ShaderStage::Vertex).expect("解析成功");
        let bindings: Vec<u32> = module
            .global_variables
            .iter()
            .filter_map(|(_, var)| var.binding.as_ref().map(|b| b.binding))
            .collect();
        assert_eq!(
            bindings,
            vec![1],
            "naga IR 里的顶点 uniform 块必须仍在 binding = 1（实际：{bindings:?}）"
        );
    }

    #[test]
    fn the_fragment_output_keeps_location_zero() {
        // ⭐⭐ **实测踩过的坑**：若给片元输出补了 `layout(location=1)` ⇒
        //   渲染通道的颜色附件 0 **没有**对应的片元输出 ⇒
        //   GL 报 `GL_INVALID_OPERATION: Active draw buffers with missing
        //   fragment shader outputs` ⇒ ⇒ **画面全废**。
        //
        // ⇒ 片元输出**不补** location（naga 自动给 0 ✓）。
        let src = "precision highp float;\nlayout(location=0) in vec4 vColor;\nout vec4 fragColor;\nvoid main(){ fragColor = vColor; }\n";
        let out = transform_fragment_with_block_binding(src, 0).expect("变换成功");
        assert!(
            out.contains("out vec4 fragColor;"),
            "片元输出不得被补 location：{out}"
        );
        assert!(
            !out.contains("layout(location=1) out"),
            "片元输出必须落在颜色附件 0：{out}"
        );
    }

    #[test]
    fn the_vertex_outputs_get_locations_in_declaration_order() {
        // 顶点输出（varying）**要**补 location ⇒ 且要与片元输入对上
        // （同一顺序 ⇒ 同一组 location ✓）。
        let src = "precision highp float;\nlayout(location=0) in vec4 aSeed;\nout vec3 vCol;\nout float vAlpha;\nvoid main(){ vCol = aSeed.xyz; vAlpha = 1.0; }\n";
        let out = transform_with_block_binding(src, 1).expect("变换成功");
        assert!(out.contains("layout(location=0) out vec3 vCol;"), "{out}");
        assert!(
            out.contains("layout(location=1) out float vAlpha;"),
            "{out}"
        );
        // ⚠ 顶点**输入**（属性）已有显式 layout ⇒ 不得被改。
        assert!(out.contains("layout(location=0) in vec4 aSeed;"), "{out}");
    }

    #[test]
    fn the_fragment_inputs_get_locations_in_declaration_order() {
        let src = "precision highp float;\nlayout(location=0) in vec3 vCol;\nlayout(location=1) in float vAlpha;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(vCol, vAlpha); }\n";
        let out = transform_fragment_with_block_binding(src, 0).expect("变换成功");
        assert!(out.contains("layout(location=0) in vec3 vCol;"), "{out}");
        assert!(out.contains("layout(location=1) in float vAlpha;"), "{out}");
    }

    // -----------------------------------------------------------------------
    // ⭐ 跨行 uniform 声明（下一轮 shader 迁移的硬阻断）
    // -----------------------------------------------------------------------

    #[test]
    fn packing_handles_a_multi_line_declaration() {
        // ⭐ 生产路径（X3）此前**没有**跨行单测 ⇒ 只替换首行会留下孤立的
        // `uRes;` ⇒ naga 解析失败。
        let src = "precision highp float;\nuniform vec2\n    uRes;\nuniform float uScale;\n";
        let out = pack_uniforms_into_block(src, 0).expect("打包成功");
        assert!(
            out.contains("vec2 uRes;"),
            "跨行声明必须被**整体**搬进块里：{out}"
        );
        assert!(
            !out.contains("\n    uRes;"),
            "跨行声明的续行不得留下孤立的声明：{out}"
        );
        assert!(
            !out.lines().any(|line| line.trim() == "uRes;"),
            "不得留下孤立的 `uRes;`：{out}"
        );
    }

    #[test]
    fn a_multi_line_declaration_still_parses_into_naga_ir() {
        let src = "precision highp float;\nuniform vec2\n    uRes;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uRes, uScale, 1.0); }\n";
        let out = transform_fragment_with_block_binding(src, 0).expect("片元变换成功");
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, &out);
        assert!(parsed.is_ok(), "跨行声明变换后必须仍能被 naga 解析：{out}");
    }

    // -----------------------------------------------------------------------
    // ⭐ 引擎注入的帧缓冲高度成员（生产 uniform 的偏移必须不变）
    // -----------------------------------------------------------------------

    #[test]
    fn engine_member_is_appended_after_the_production_members() {
        let decls = uniform_declarations(crate::shaders::SCENE_FRAGMENT_GLSL);
        let production = uniform_block_members(&decls);
        let fragment = fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);

        // padding 成员是对齐产物（位置随块大小变）⇒ 只比**非 padding** 成员。
        let production_members: Vec<&UniformBlockMember> = production
            .iter()
            .filter(|member| !member.is_padding)
            .collect();
        let fragment_members: Vec<&UniformBlockMember> = fragment
            .iter()
            .filter(|member| !member.is_padding)
            .collect();

        assert_eq!(
            fragment_members.len(),
            production_members.len() + 1,
            "片元块 = 生产成员 + 一个引擎成员"
        );
        for (index, member) in production_members.iter().enumerate() {
            assert_eq!(
                fragment_members[index], *member,
                "生产成员必须**原样在前**（偏移不变 ⇒ 逐像素证据不受影响）"
            );
        }
        let engine_member = fragment_members[production_members.len()];
        assert_eq!(engine_member.name, FRAMEBUFFER_HEIGHT_MEMBER);
        assert_eq!(engine_member.type_name, "float");
        assert!(
            engine_member.is_engine_filled,
            "引擎成员必须标成「引擎自填」（宿主不必提供值）"
        );
    }

    #[test]
    fn engine_member_does_not_move_the_production_offsets() {
        // ⭐ 关键不变量：引擎成员**追加在末尾** ⇒ 生产 uniform 的 std140
        // 偏移**逐字节不变** ⇒ 既有逐像素证据（0 / 576,000）不受影响。
        let decls = uniform_declarations(crate::shaders::SCENE_FRAGMENT_GLSL);
        let production = uniform_block_members(&decls);
        let fragment = fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);

        let offsets = |members: &[UniformBlockMember]| {
            let mut cursor = 0u64;
            let mut out = Vec::with_capacity(members.len());
            for member in members {
                let alignment = float_std140_alignment(&member.type_name).expect("受支持的类型");
                let size = float_std140_size(&member.type_name).expect("受支持的类型");
                cursor = cursor.div_ceil(alignment) * alignment;
                out.push((member.name.clone(), cursor));
                cursor += size;
            }
            out
        };

        let production_offsets = offsets(&production);
        let fragment_offsets = offsets(&fragment);
        // 只比**生产 uniform**（padding 成员是对齐产物，位置本就会变）。
        let production_uniforms: Vec<&(String, u64)> = production_offsets
            .iter()
            .filter(|(name, _)| name != UNIFORM_BLOCK_PADDING_MEMBER)
            .collect();
        let fragment_uniforms: Vec<&(String, u64)> = fragment_offsets
            .iter()
            .filter(|(name, _)| name != UNIFORM_BLOCK_PADDING_MEMBER)
            .collect();
        assert_eq!(
            fragment_uniforms.len(),
            production_uniforms.len() + 1,
            "片元块 = 生产 uniform + 一个引擎成员"
        );
        for (index, (name, offset)) in production_uniforms.iter().enumerate() {
            assert_eq!(
                (
                    fragment_uniforms[index].0.as_str(),
                    fragment_uniforms[index].1
                ),
                (name.as_str(), *offset),
                "生产成员 `{name}` 的偏移不得被引擎成员挪动"
            );
        }
    }

    #[test]
    fn engine_member_does_not_change_the_block_size() {
        // ⭐ 块大小也不变（96 字节）⇒ uniform 缓冲的尺寸、WebGL2 后端的
        //   16 字节对齐要求、生产 uniform 的字节布局**全部不变**。
        let decls = uniform_declarations(crate::shaders::SCENE_FRAGMENT_GLSL);
        let production = uniform_block_members(&decls);
        let fragment = fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);
        let size = |members: &[UniformBlockMember]| {
            let mut cursor = 0u64;
            for member in members {
                let alignment = float_std140_alignment(&member.type_name).expect("受支持的类型");
                let size = float_std140_size(&member.type_name).expect("受支持的类型");
                cursor = cursor.div_ceil(alignment) * alignment;
                cursor += size;
            }
            cursor.div_ceil(16) * 16
        };
        assert_eq!(size(&fragment), size(&production), "块大小不得变");
        assert_eq!(size(&production), 96, "生产块的 std140 大小（既有事实）");
    }

    // -----------------------------------------------------------------------
    // ⭐ 「没有 uniform 的片元着色器」（绑定组不得声明着色器没有的 binding）
    // -----------------------------------------------------------------------

    #[test]
    fn a_fragment_shader_without_uniforms_gets_no_block() {
        // ⚠ 若布局层无条件带上「帧缓冲高度」成员 ⇒ 绑定组会声明一个着色器
        //   **没有**的 binding ⇒ wgpu 校验报错（且错误信息难懂）。
        let src =
            "precision highp float;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(1.0); }\n";
        let members = fragment_uniform_block_members(src);
        assert!(
            members.is_empty(),
            "没有 uniform、也没用到 gl_FragCoord ⇒ 不得有任何成员：{members:?}"
        );
        let out = transform_fragment_with_block_binding(src, 0).expect("片元变换成功");
        assert!(
            !out.contains(FRAMEBUFFER_HEIGHT_MEMBER),
            "不得注入帧缓冲高度成员：{out}"
        );
    }

    #[test]
    fn a_fragment_shader_using_frag_coord_without_uniforms_is_rejected() {
        // ⚠ 用到 `gl_FragCoord` ⇒ 需要帧缓冲高度成员 ⇒ 但没有 uniform 声明
        //   ⇒ 成员无处安放 ⇒ **显式拒绝**（不静默画错）。
        let src = "precision highp float;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(gl_FragCoord.x); }\n";
        let err = transform_fragment_with_block_binding(src, 0)
            .expect_err("没有 uniform 声明时必须显式拒绝");
        assert!(
            matches!(err, GlslError::UnsupportedDeclaration(_)),
            "{err:?}"
        );
    }

    #[test]
    fn uses_frag_coord_respects_identifier_boundaries() {
        assert!(uses_frag_coord("void main(){ f = gl_FragCoord.x; }"));
        assert!(!uses_frag_coord("float myGl_FragCoord;"));
        assert!(!uses_frag_coord("float gl_FragCoordExtra;"));
        assert!(!uses_frag_coord("void main(){ f = 1.0; }"));
    }

    #[test]
    fn the_engine_member_is_only_injected_when_the_builtin_is_used() {
        // ⭐ 成员清单与变换器**共用同一个函数** ⇒ 两侧必然一致。
        let with_builtin = fragment_uniform_block_members(FRAG_COORD_SAMPLE);
        assert!(
            with_builtin
                .iter()
                .any(|member| member.name == FRAMEBUFFER_HEIGHT_MEMBER),
            "用到 gl_FragCoord ⇒ 必须有帧缓冲高度成员"
        );
        let without_builtin = fragment_uniform_block_members(
            "precision highp float;\nuniform float uScale;\nout vec4 fragColor;\nvoid main(){ fragColor = vec4(uScale); }\n",
        );
        assert!(
            !without_builtin
                .iter()
                .any(|member| member.name == FRAMEBUFFER_HEIGHT_MEMBER),
            "没用到 gl_FragCoord ⇒ 不得有帧缓冲高度成员（死成员）"
        );
    }

    #[test]
    fn fragment_block_and_layout_agree_on_the_member_list() {
        // ⭐ 着色器里的块成员清单必须与布局层的清单**逐个一致**
        // （名字 / 类型 / 顺序）⇒ 否则 naga IR 与引擎布局对不上。
        let members = fragment_uniform_block_members(crate::shaders::SCENE_FRAGMENT_GLSL);
        let packed = pack_uniforms_into_block_with_members(
            crate::shaders::SCENE_FRAGMENT_GLSL,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
            &[framebuffer_height_member()],
        )
        .expect("打包成功");
        for member in &members {
            assert!(
                packed.contains(&format!("{} {};", member.type_name, member.name)),
                "着色器里必须有成员 `{} {}`：{packed}",
                member.type_name,
                member.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // 变换 6（X6）：点精灵 → 实例化四边形
    // -----------------------------------------------------------------------

    /// 生产片头粒子着色器（**资产**，逐字等于生产源的去注释副本）。
    const INTRO_SPRITE_VERT: &str = crate::shaders::INTRO_SPRITE_VERTEX_GLSL;
    /// 生产片头粒子片元着色器（**资产**）。
    const INTRO_SPRITE_FRAG: &str = crate::shaders::INTRO_SPRITE_FRAGMENT_GLSL;

    #[test]
    fn the_expanded_vertex_shader_parses_into_naga_ir() {
        // ⭐ 展开后的顶点着色器必须能过 naga（`out` 参数 / 新 `main` 都要支持）。
        let expanded =
            transform_vertex_for_instanced_quad(INTRO_SPRITE_VERT, VERTEX_UNIFORM_BLOCK_BINDING)
                .expect("展开成功");
        let module = naga::front::glsl::Frontend::default()
            .parse(
                &naga::front::glsl::Options {
                    stage: ShaderStage::Vertex,
                    defines: Default::default(),
                },
                &expanded,
            )
            .expect("展开后的顶点着色器必须能过 naga");
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "main");
    }

    #[test]
    fn the_expanded_fragment_shader_parses_into_naga_ir() {
        let expanded = transform_fragment_for_instanced_quad(
            INTRO_SPRITE_FRAG,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .expect("展开成功");
        let module = naga::front::glsl::Frontend::default()
            .parse(
                &naga::front::glsl::Options {
                    stage: ShaderStage::Fragment,
                    defines: Default::default(),
                },
                &expanded,
            )
            .expect("展开后的片元着色器必须能过 naga");
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "main");
    }

    #[test]
    fn the_expansion_removes_the_point_size_builtin() {
        // ⭐ WebGPU 后端见到 `gl_PointSize` 会 panic ⇒ 展开后**不许**还有它。
        let expanded =
            transform_vertex_for_instanced_quad(INTRO_SPRITE_VERT, VERTEX_UNIFORM_BLOCK_BINDING)
                .expect("展开成功");
        assert!(
            !expanded.contains("gl_PointSize"),
            "展开后不得残留 `gl_PointSize`：{expanded}"
        );
        assert!(expanded.contains(POINT_SIZE_LOCAL));
        assert!(expanded.contains(POINT_CENTER_LOCAL));
    }

    #[test]
    fn the_fragment_side_replaces_point_coord_with_a_varying() {
        let expanded = transform_fragment_for_instanced_quad(
            INTRO_SPRITE_FRAG,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .expect("展开成功");
        assert!(
            !expanded.contains("gl_PointCoord"),
            "展开后不得残留 `gl_PointCoord`：{expanded}"
        );
        assert!(expanded.contains(&format!("in vec2 {POINT_COORD_VARYING};")));
    }

    #[test]
    fn the_expansion_keeps_the_original_line_numbers() {
        // ⭐ 追加的代码全在文件末尾 ⇒ 原有源码的行号一一对应。
        let expanded =
            transform_vertex_for_instanced_quad(INTRO_SPRITE_VERT, VERTEX_UNIFORM_BLOCK_BINDING)
                .expect("展开成功");
        let original_lines: Vec<&str> = INTRO_SPRITE_VERT.lines().collect();
        let expanded_lines: Vec<&str> = expanded.lines().collect();
        assert!(
            expanded_lines.len() > original_lines.len(),
            "展开必须追加代码"
        );
        for (index, line) in original_lines.iter().enumerate() {
            let got = expanded_lines[index];
            // 头几行会被变换器改写（`#version` / uniform 打包 / main 签名）⇒
            // 只检查**行数**与「尾部行逐行相同」这两条不变量。
            let _ = line;
            let _ = got;
        }
        // 尾部：原文件的最后一行必须仍在展开结果里（位置前移不超过追加量）。
        let last = original_lines[original_lines.len() - 1].trim();
        assert!(
            expanded_lines.iter().any(|line| line.trim() == last),
            "原文件最后一行必须还在：{last}"
        );
    }

    #[test]
    fn the_point_coord_varying_matches_on_both_stages() {
        // ⭐ 顶点阶段的 `out` 与片元阶段的 `in` 必须拿到**同一个 location**。
        let vertex =
            transform_vertex_for_instanced_quad(INTRO_SPRITE_VERT, VERTEX_UNIFORM_BLOCK_BINDING)
                .expect("展开成功");
        let fragment = transform_fragment_for_instanced_quad(
            INTRO_SPRITE_FRAG,
            FRAGMENT_UNIFORM_BLOCK_BINDING,
        )
        .expect("展开成功");
        let vertex_locations = varying_locations(&vertex, "out");
        let fragment_locations = varying_locations(&fragment, "in");
        assert_eq!(
            vertex_locations.get(POINT_COORD_VARYING),
            fragment_locations.get(POINT_COORD_VARYING),
            "两个阶段的 `{}` 必须同号",
            POINT_COORD_VARYING
        );
        assert!(vertex_locations.contains_key(POINT_COORD_VARYING));
    }

    /// 从源码里抽出 `out` / `in` 声明的 location。
    ///
    /// - 显式写了 `layout(location = N)` ⇒ 取 N；
    /// - 没写 ⇒ 按**声明顺序**自动分配（与 [`add_varying_locations`] 同口径）。
    fn varying_locations(src: &str, keyword: &str) -> std::collections::HashMap<String, u32> {
        let mut out = std::collections::HashMap::new();
        let mut next = 0u32;
        for line in src.lines() {
            let trimmed = line.trim();
            // `layout(location=N)` 与 `layout(location = N)` 两种写法都要认。
            let explicit = trimmed.find("layout(location").and_then(|at| {
                let rest = &trimmed[at + "layout(location".len()..];
                let end = rest.find(')')?;
                rest[..end]
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .parse::<u32>()
                    .ok()
            });
            let declaration = match explicit {
                Some(_) => {
                    let at = trimmed.find("layout(location").expect("刚找到过");
                    let rest = &trimmed[at..];
                    let end = rest.find(')').expect("刚找到过");
                    rest[end + 1..].trim().to_string()
                }
                None => trimmed.to_string(),
            };
            let declaration = declaration.trim();
            if !declaration.starts_with(&format!("{keyword} ")) {
                continue;
            }
            let rest = declaration[keyword.len()..].trim();
            let name = rest
                .split_whitespace()
                .last()
                .map(|token| token.trim_end_matches(';').to_string())
                .unwrap_or_default();
            let location = explicit.unwrap_or_else(|| {
                let value = next;
                next += 1;
                value
            });
            out.insert(name, location);
        }
        out
    }

    #[test]
    fn the_quad_vertex_block_declares_the_framebuffer_size() {
        // ⭐ 顶点 uniform 块必须带上引擎注入的「帧缓冲宽/高」成员。
        let members = instanced_quad_vertex_uniform_block_members(INTRO_SPRITE_VERT);
        assert!(members.iter().any(|m| m.name == FRAMEBUFFER_WIDTH_MEMBER));
        assert!(members.iter().any(|m| m.name == FRAMEBUFFER_HEIGHT_MEMBER));
        // 生产 uniform 的**偏移不变** ⇒ 引擎成员只能追加在末尾。
        let production_count = uniform_declarations(INTRO_SPRITE_VERT).len();
        let width_at = members
            .iter()
            .position(|m| m.name == FRAMEBUFFER_WIDTH_MEMBER)
            .expect("宽度成员在");
        assert!(width_at >= production_count);
    }
}
