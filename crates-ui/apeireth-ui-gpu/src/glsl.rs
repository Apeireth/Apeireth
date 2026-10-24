//! GLSL ES 3.0 → naga IR **前置变换器**（纯文本变换，不需 GPU）。
//!
//! 存在理由（上游事实 + 实测事实，非本 crate 的偏好）：
//!
//! | # | 限制 | 出处 | 本 crate 的对策 |
//! |---|---|---|---|
//! | X1 | naga 的 GLSL 前端**没有 ES profile**：`#version 300 es` 是硬解析错误 | `naga-30.0.1/src/front/glsl/parser.rs:267`（只收 `#version 440｜450｜460`）、`:290`（只收 `core` profile） | [`strip_version_directive`]：删掉 `#version` 行 |
//! | X2 | naga 的 GLSL 前端要求 uniform **显式带 `layout(binding=N)`** | `naga-30.0.1/src/front/glsl/variables.rs:618` | [`inject_layout_bindings`]（逐个注入）/ [`pack_uniforms_into_block`]（合并成一块） |
//! | **X3** | **WebGL2 后端的 `max_uniform_buffers_per_shader_stage` 装不下 16 个独立 uniform buffer** | **实测**（本机 Chromium 154 + 软件 WebGL）：限额 **14**，`create_bind_group_layout` 直接报 `Too many bindings of type UniformBuffers in Stage ShaderStages(FRAGMENT), limit is 14, count was 16`（GLES3 规范下限是 12） | [`pack_uniforms_into_block`]：把 N 个 plain uniform **合并进一个 std140 uniform 块** ⇒ **1 个 binding** |
//! | **X4** | **本引擎的两个后端都把片元坐标的 y 轴变成「向下」，而 GLSL ES 规定「向上」** | GLSL ES 3.0 规范（`gl_FragCoord` 原点在**左下角**）；WebGPU 的 `@builtin(position)` 原点在**左上角**；`wgpu-hal-30.0.1/src/gles/web.rs:301-312`（GL 路径把主渲染上下翻转：「GL's presentation is not flipped, but main rendering is. Therefore, we Y-flip the output positions in the shader, and also this blit.」） | [`fix_frag_coord_semantics`]：源级把 `gl_FragCoord` 换成 `vec4(x, height - y, z, w)` |
//!
//! 四步都是**方言外壳**层面的机械变换，**不触碰着色器数学**（光学数值零改写）。
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
//! 修正只改**坐标的取法**（`y → height - y`），不改任何表达式与数值。
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

/// uniform 块里的一个成员声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformBlockMember {
    /// 成员名（uniform 名，或 padding 成员名）。
    pub name: String,
    /// 类型名（`float` / `vec2` / `vec3` / `vec4`）。
    pub type_name: String,
    /// 该成员是不是引擎补齐用的 **padding 成员**（不被着色器引用）。
    pub is_padding: bool,
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

/// 删掉 `#version` 指令行（naga 的 GLSL 前端没有 ES profile）。
///
/// 其余预处理行（`#extension` / `#define` / `#ifdef` …）**原样保留**。
pub fn strip_version_directive(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        if line.trim_start().starts_with("#version") {
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
    /// 行号（0 起）。
    line: usize,
    /// 声明原文（不含缩进与换行）。
    text: String,
}

/// 扫描源码里的**顶层** plain uniform 声明（位置 + 原文）。
///
/// - 只认**顶层**（花括号深度 0）的 `uniform` 声明；
/// - 跨行声明按**整体**处理（拼成一条）；
/// - 已带 `layout(...)` 的 uniform **块**不在此列（那是另一种形态）。
fn scan_uniform_sites(src: &str) -> Vec<UniformSite> {
    let mut sites = Vec::new();
    let mut depth = 0i32;
    let mut pending: Option<(usize, String)> = None;

    for (index, line) in src.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim_start();

        if let Some((start, body)) = pending.take() {
            let body = format!("{body} {bare}");
            depth += brace_delta(bare);
            if bare.contains(';') {
                sites.push(UniformSite {
                    line: start,
                    text: body.trim().to_string(),
                });
            } else {
                pending = Some((start, body));
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
                    text: trimmed.to_string(),
                });
            } else {
                pending = Some((index, trimmed.to_string()));
            }
            continue;
        }

        depth += brace_delta(bare);
    }

    if let Some((start, body)) = pending {
        sites.push(UniformSite {
            line: start,
            text: body.trim().to_string(),
        });
    }
    sites
}

/// uniform 块的成员清单（含引擎补齐用的 padding 成员）。
///
/// 规则：
/// 1. 成员按**声明序**排列（⇒ 成员序 = 映射表下标）；
/// 2. 若按 std140 算出的块大小**不是 16 的倍数**，末尾补一个 `vec4`
///    padding 成员（WebGL2 后端的硬要求，见
///    [`UNIFORM_BLOCK_PADDING_MEMBER`]）。
pub fn uniform_block_members(decls: &[UniformDecl]) -> Vec<UniformBlockMember> {
    let mut members: Vec<UniformBlockMember> = decls
        .iter()
        .map(|decl| UniformBlockMember {
            name: decl.name.clone(),
            type_name: decl.type_name.clone(),
            is_padding: false,
        })
        .collect();
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
        let alignment = float_std140_alignment(&member.type_name);
        let size = float_std140_size(&member.type_name);
        cursor = cursor.div_ceil(alignment) * alignment;
        cursor += size;
    }
    if supported && !cursor.is_multiple_of(16) {
        members.push(UniformBlockMember {
            name: UNIFORM_BLOCK_PADDING_MEMBER.to_string(),
            type_name: "vec4".to_string(),
            is_padding: true,
        });
    }
    members
}

/// 把源码里的**顶层 plain uniform 声明合并进一个 std140 uniform 块**（X3）。
///
/// 输入（生产版着色器的形态）：
/// ```text
/// uniform vec2  uRes;
/// uniform vec2  uCenter;
/// uniform float uScale;
/// ```
/// 输出：
/// ```text
/// layout(std140, binding=0) uniform EngineUniforms {
///     vec2 uRes;
///     vec2 uCenter;
///     float uScale;
/// };
/// ```
///
/// - 成员**按声明序**排列 ⇒ 成员序 = 声明序 = 映射表下标；
/// - 着色器正文里的 `uRes` 等标识符**不用改**（GLSL 允许块成员不带实例名
///   直接访问；naga 的 GLSL 前端实测支持，见本模块的单测）；
/// - **不改任何表达式与数值** —— 只改声明外壳；
/// - 源码里没有 uniform 时**原样返回**（不造空块）。
///
/// # Errors
///
/// 声明里出现数组形态（`uniform float w[4];`）→
/// [`GlslError::UnsupportedDeclaration`]（本引擎只支持标量 / 向量）。
pub fn pack_uniforms_into_block(src: &str, block_binding: u32) -> Result<String, GlslError> {
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

    // 块文本（成员清单含引擎补齐的 padding 成员）。
    let mut block =
        format!("layout(std140, binding={block_binding}) uniform {UNIFORM_BLOCK_NAME} {{\n");
    for member in uniform_block_members(&decls) {
        block.push_str(&format!("    {} {};\n", member.type_name, member.name));
    }
    block.push_str("};");

    // 重建源码：声明所在行留空（保持行号），第一处声明的位置放块。
    let lines: Vec<&str> = clean.split_inclusive('\n').collect();
    let mut out = String::with_capacity(clean.len() + block.len() + 64);
    let mut inserted = false;
    for (index, line) in lines.iter().enumerate() {
        let is_site = sites.iter().any(|site| site.line == index);
        if is_site {
            if !inserted {
                out.push_str(&block);
                inserted = true;
            }
            // 保留换行 ⇒ 行号不变。
            if line.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        out.push_str(line);
    }
    if !inserted {
        out.push_str(&block);
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
/// # Panics
///
/// 类型不受支持时 panic —— 调用方须先用 [`is_supported_float_type`] 判定。
pub fn float_component_count(type_name: &str) -> usize {
    match type_name {
        "float" => 1,
        "vec2" => 2,
        "vec3" => 3,
        "vec4" => 4,
        other => panic!("unsupported float type: {other}"),
    }
}

/// 该浮点类型的 **std140 字节大小**（`vec3` 占 12）。
///
/// # Panics
///
/// 类型不受支持时 panic（见 [`float_component_count`]）。
pub fn float_std140_size(type_name: &str) -> u64 {
    (float_component_count(type_name) as u64) * 4
}

/// 该浮点类型的 **std140 基对齐**（`float`=4 / `vec2`=8 / `vec3`/`vec4`=16）。
///
/// # Panics
///
/// 类型不受支持时 panic（见 [`float_component_count`]）。
pub fn float_std140_alignment(type_name: &str) -> u64 {
    match type_name {
        "float" => 4,
        "vec2" => 8,
        "vec3" | "vec4" => 16,
        other => panic!("unsupported float type: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 变换链与解析
// ---------------------------------------------------------------------------

/// 完整的 GLSL ES 3.0 → naga 可解析文本 的变换链。
///
/// 依次执行：剥注释 → 删 `#version` → 把 plain uniform 合并进一个
/// std140 uniform 块（X3）。
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
    pack_uniforms_into_block(&no_version, block_binding)
}

/// **片元阶段**的变换链：在 [`transform_with_block_binding`] 之外，额外做
/// `gl_FragCoord` 的**坐标语义修正**（见 [`fix_frag_coord_semantics`]）。
///
/// 顶点阶段**不做**这件事（GLSL 里 `gl_FragCoord` 只存在于片元阶段）。
///
/// # Errors
///
/// 声明形态不受支持 → [`GlslError::UnsupportedDeclaration`]。
pub fn transform_fragment_with_block_binding(
    src: &str,
    block_binding: u32,
    framebuffer_height: u32,
) -> Result<String, GlslError> {
    let transformed = transform_with_block_binding(src, block_binding)?;
    Ok(fix_frag_coord_semantics(&transformed, framebuffer_height))
}

// ---------------------------------------------------------------------------
// 变换 4（X4）：`gl_FragCoord` 的坐标语义修正
// ---------------------------------------------------------------------------

/// 把源码里的 `gl_FragCoord` 换成**GLSL ES 规范语义**的等价写法。
///
/// ## 为什么必须做（方言外壳层，**不碰数学**）
///
/// GLSL ES 3.0 规定 `gl_FragCoord` 的原点在**左下角**、y 轴**向上**
/// （`gl_FragCoord.xy ∈ [0.5, width-0.5] × [0.5, height-0.5]`）。
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
/// 本函数在**源级**把 `gl_FragCoord` 换成 `vec4(x, height - y, z, w)`
/// ⇒ 引擎里的片元坐标与 GLSL ES 规范**逐点一致** ⇒ 同一份着色器源在引擎与
/// 生产路径上画出的画面一致。
///
/// ## 行为细节
///
/// - **单趟扫描**：替换文本里的 `gl_FragCoord` **不再被二次替换**；
/// - **词边界**感知：`myGl_FragCoord` / `gl_FragCoordExtra` 等**不受影响**；
/// - **不插行** ⇒ 变换前后**行号一一对应**（错误信息仍指回原始行）；
/// - 源码里没有 `gl_FragCoord` ⇒ **原样返回**。
pub fn fix_frag_coord_semantics(src: &str, framebuffer_height: u32) -> String {
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
        assert_eq!(float_component_count("float"), 1);
        assert_eq!(float_component_count("vec3"), 3);
        assert_eq!(float_std140_size("float"), 4);
        assert_eq!(float_std140_size("vec2"), 8);
        assert_eq!(float_std140_size("vec3"), 12);
        assert_eq!(float_std140_size("vec4"), 16);
    }

    #[test]
    fn float_std140_alignments_match_the_spec() {
        assert_eq!(float_std140_alignment("float"), 4);
        assert_eq!(float_std140_alignment("vec2"), 8);
        assert_eq!(float_std140_alignment("vec3"), 16);
        assert_eq!(float_std140_alignment("vec4"), 16);
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
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE, 600);
        assert!(
            out.contains("600.0 - gl_FragCoord.y"),
            "片元坐标的 y 必须翻成 GLSL 规范（向上）：{out}"
        );
        assert!(
            out.contains(
                "vec4(gl_FragCoord.x, 600.0 - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w)"
            ),
            "展开式必须逐分量保留其余分量：{out}"
        );
    }

    #[test]
    fn frag_coord_correction_keeps_line_numbers() {
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE, 600);
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
            fix_frag_coord_semantics(source, 600),
            source,
            "源码里没有 gl_FragCoord 时必须原样返回"
        );
    }

    #[test]
    fn frag_coord_correction_respects_identifier_boundaries() {
        let source = "float myGl_FragCoord = 1.0;\nfloat gl_FragCoordExtra = 2.0;\n";
        let out = fix_frag_coord_semantics(source, 600);
        assert_eq!(out, source, "相似标识符不得被误改（词边界必须生效）");
    }

    #[test]
    fn frag_coord_correction_does_not_recurse() {
        let out = fix_frag_coord_semantics(FRAG_COORD_SAMPLE, 600);
        assert_eq!(
            out.matches("600.0 - gl_FragCoord.y").count(),
            1,
            "替换文本里的 gl_FragCoord 不得被二次替换：{out}"
        );
        assert!(!out.contains("600.0 - 600.0"), "不得出现递归替换：{out}");
    }

    #[test]
    fn fragment_transform_applies_the_correction_with_the_framebuffer_height() {
        let out =
            transform_fragment_with_block_binding(FRAG_COORD_SAMPLE, 0, 480).expect("片元变换成功");
        assert!(
            out.contains("480.0 - gl_FragCoord.y"),
            "片元变换必须带上**表面高度**：{out}"
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
            !out.contains("600.0 - gl_FragCoord.y"),
            "顶点阶段不得做坐标语义修正：{out}"
        );
        assert!(out.contains("gl_FragCoord.xy"), "{out}");
    }

    #[test]
    fn corrected_source_still_parses_into_naga_ir() {
        let out =
            transform_fragment_with_block_binding(FRAG_COORD_SAMPLE, 0, 600).expect("片元变换成功");
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
            600,
        )
        .expect("生产资产的片元变换必须成功");
        assert!(
            out.contains("600.0 - gl_FragCoord.y"),
            "生产资产确实用到 gl_FragCoord ⇒ 修正必须生效"
        );
        let options = naga::front::glsl::Options {
            stage: ShaderStage::Fragment,
            defines: Default::default(),
        };
        let parsed = naga::front::glsl::Frontend::default().parse(&options, &out);
        assert!(parsed.is_ok(), "修正后的生产资产必须能被 naga 解析");
    }
}
