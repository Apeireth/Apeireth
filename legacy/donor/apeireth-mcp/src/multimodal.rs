//! **apeireth-mcp / multimodal — 多模态生成 dispatcher template (R123-4)**
//!
//! **依据**: docs/v2-strategy/07 §2 P2-13 (多模态生成 via MCP, 9 个 Gen 插件)
//!
//! **战略 (07 §2 P2-13)**:
//! - **不自研** — 通过 `apeireth-mcp` 接 9 个 Gen 插件 MCP server
//! - 借鉴 VCP `Plugin/*Gen*/` 9 个 (ComfyUI / Flux / Doubao / GPTImage / Gemini / Agnes / AgnesVideo / DMX / NanoBanana)
//! - DoD: MCP 接入 ≥ 3 个多模态 MCP server, `apeireth-tools` 暴露统一 multimodal interface
//!
//! **本 module 范围 (R123-4 dispatcher template, 0 真接)**:
//! - 1 个 `GenPlugin` enum (9 variants) — 路由表
//! - 1 个 `OutputFormat` enum (6 variants) — 覆盖常见模态 (image 3 + video 1 + 3d 1 + vector 1)
//! - 1 个 `GenRequest` struct (5 fields) — call dispatcher 的入参
//! - 1 个 `GenResponse` struct (5 fields) — dispatcher 的出参 (含 image_url / video_url / model_url 3 字段, 不假设哪种)
//! - `plugin_endpoint(p)` — per plugin URL 模板 (0 真连, 仅占位)
//! - `gen_dispatch(req)` — dispatcher template, 永远返 `Err("plugin X not connected")` (O-5 诚实标缺, R124+ 真接)
//! - `dispatch_multimodal_handler(args)` — 给 `mcp::tool_bridge::handler_from_fn` 用的 serde bridge
//! - `multimodal_tool_def()` — 给 `McpServer::register_tool(def, handler)` 用的 `ToolDef`
//!
//! **不假装 (O-5)**:
//! - ✅ 9 plugin 路由表 + URL 模板真建 (编译期 hardcode)
//! - ✅ `gen_dispatch` 永远 `Err` (0 HTTP, 0 reqwest 调用, 0 I/O)
//! - ✅ 6 output format enum + 8+ unit test 覆盖
//! - ❌ 0 真接任何 Gen 插件 (R124+ 真接, 9 插件 1 个 1 个)
//! - ❌ 0 改 apeireth-tool-registry / Tool trait
//!
//! **不修改承诺 (R123-4 0 触碰)**:
//! - ✅ 0 改 lib.rs 现有 700 行 (仅 1 行 `pub mod multimodal;`)
//! - ✅ 0 改 tools.rs / tool_bridge.rs / Cargo.toml `[package]` / `[dependencies]`
//! - ✅ 0 改 24 LOCKED crate mtime
//! - ✅ 0 改 workspace.version (1.1.0)
//! - ✅ 0 改 11 agent 公共 API 签名 (走 `handler_from_fn` 显式注册)
//!
//! **借鉴 ID**: `R123-4-VCP-MultimodalMCP-2026-08-10` (per 07 §3 P2-13)
//! **下一步**: R124+ 真接 9 plugin 1 个 1 个, 每个 plugin 1 个 sub-`GenPluginBackend` impl

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::tool_bridge::ToolDef;

// ============================================================
// 编译期 hardcode
// ============================================================

/// **9 个 Gen 插件总数** (编译期 hardcode, 防加 variant 忘改 docs)
pub const GEN_PLUGIN_COUNT: usize = 9;

/// **6 个 output format 总数** (编译期 hardcode)
pub const OUTPUT_FORMAT_COUNT: usize = 6;

/// **默认 size 1024x1024** (per VCP `Plugin/DoubaoGen*/plugin-manifest.json` 默认 1024)
pub const DEFAULT_SIZE: (u32, u32) = (1024, 1024);

const _: () = {
    // 防 GEN_PLUGIN_COUNT 与 enum variant 漂移
    assert!(GEN_PLUGIN_COUNT == 9, "GEN_PLUGIN_COUNT must be 9");
    assert!(OUTPUT_FORMAT_COUNT == 6, "OUTPUT_FORMAT_COUNT must be 6");
};

// ============================================================
// GenPlugin (9 个 Gen 插件 enum)
// ============================================================

/// **9 个 Gen 插件 enum** (per 07 §2 P2-13, VCP `Plugin/*Gen*/` 9 个)
///
/// **不漂移**:
/// - ✅ 跟 VCP 9 plugin 名 1:1 (大小写统一, GPTImage 不是 GPT-Image)
/// - ✅ serde tag = `"name"` (snake_case for Comfy/Flux/..., PascalCase for Doubao/GPTImage/Gemini/Agnes/DMX/NanoBanana)
/// - ❌ 0 真接任何 plugin (R124+ 真接)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenPlugin {
    /// ComfyUI (本地 workflow 提交, 默认 8188)
    #[serde(rename = "comfyui")]
    ComfyUI,
    /// FLUX.1 (Black Forest Labs, pro/dev/schnell)
    #[serde(rename = "flux")]
    Flux,
    /// 豆包文生图 (字节火山方舟)
    #[serde(rename = "doubao")]
    Doubao,
    /// GPT-Image / DALL-E (OpenAI)
    #[serde(rename = "gpt_image")]
    GPTImage,
    /// Imagen 3 / 4 (Google Gemini)
    #[serde(rename = "gemini")]
    Gemini,
    /// Agnes SD (Stability Diffusion-like, 本地 7860)
    #[serde(rename = "agnes")]
    Agnes,
    /// Agnes 视频版
    #[serde(rename = "agnes_video")]
    AgnesVideo,
    /// DeepMind-X (占位)
    #[serde(rename = "dmx")]
    DMX,
    /// NanoBanana (占位)
    #[serde(rename = "nano_banana")]
    NanoBanana,
}

impl GenPlugin {
    /// **全部 9 variant** (per R124 真接时的 router 列表)
    pub const ALL: &'static [GenPlugin] = &[
        GenPlugin::ComfyUI,
        GenPlugin::Flux,
        GenPlugin::Doubao,
        GenPlugin::GPTImage,
        GenPlugin::Gemini,
        GenPlugin::Agnes,
        GenPlugin::AgnesVideo,
        GenPlugin::DMX,
        GenPlugin::NanoBanana,
    ];

    /// **plugin 名 (snake_case, 给 mcp tool name 用)**
    pub fn as_str(&self) -> &'static str {
        match self {
            GenPlugin::ComfyUI => "comfyui",
            GenPlugin::Flux => "flux",
            GenPlugin::Doubao => "doubao",
            GenPlugin::GPTImage => "gpt_image",
            GenPlugin::Gemini => "gemini",
            GenPlugin::Agnes => "agnes",
            GenPlugin::AgnesVideo => "agnes_video",
            GenPlugin::DMX => "dmx",
            GenPlugin::NanoBanana => "nano_banana",
        }
    }

    /// **plugin 中文描述** (per 07 §2 P2-13 列表 + VCP `Plugin/<X>/plugin-manifest.json:description`)
    pub fn description(&self) -> &'static str {
        match self {
            GenPlugin::ComfyUI => "ComfyUI workflow 提交 (本地 8188, 通用后端)",
            GenPlugin::Flux => "FLUX.1 [pro/dev/schnell] (Black Forest Labs)",
            GenPlugin::Doubao => "豆包文生图 (字节火山方舟 ark.cn-beijing.volces.com)",
            GenPlugin::GPTImage => "OpenAI gpt-image-1 / dall-e-3 (api.openai.com)",
            GenPlugin::Gemini => "Google Imagen 3 / 4 (Gemini API)",
            GenPlugin::Agnes => "Agnes SD (本地 7860, Stability Diffusion-like)",
            GenPlugin::AgnesVideo => "Agnes 视频版 (本地 7860/sdg/video)",
            GenPlugin::DMX => "DeepMind-X (占位, R124+ 真接)",
            GenPlugin::NanoBanana => "NanoBanana (占位, R124+ 真接)",
        }
    }
}

// ============================================================
// OutputFormat (6 个常见模态)
// ============================================================

/// **6 个 output format enum** (覆盖常见多模态: 3 image + 1 video + 1 3d + 1 vector)
///
/// **不漂移**:
/// - ✅ Png / Jpg / Gif (静态图 3)
/// - ✅ Mp4 (视频 1)
/// - ✅ Webp (现代 web 图 1, per 主人 2026-08-04 web 路线)
/// - ✅ Glb (3D 模型 1, per 主人 2026-08-04 立体架构)
/// - ❌ 0 含音频 (主人决定 R124+ 单独加 audio format, 不混入 multimodal dispatcher)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// PNG (静态图, 默认)
    #[serde(rename = "png")]
    Png,
    /// JPG (静态图)
    #[serde(rename = "jpg")]
    Jpg,
    /// GIF (动图)
    #[serde(rename = "gif")]
    Gif,
    /// MP4 (视频)
    #[serde(rename = "mp4")]
    Mp4,
    /// WebP (现代 web 图)
    #[serde(rename = "webp")]
    Webp,
    /// GLB (3D 模型, glTF binary)
    #[serde(rename = "glb")]
    Glb,
}

impl OutputFormat {
    /// **全部 6 variant** (per R124 真接时的 mime type 路由)
    pub const ALL: &'static [OutputFormat] = &[
        OutputFormat::Png,
        OutputFormat::Jpg,
        OutputFormat::Gif,
        OutputFormat::Mp4,
        OutputFormat::Webp,
        OutputFormat::Glb,
    ];

    /// **MIME type** (per IANA)
    pub fn mime_type(&self) -> &'static str {
        match self {
            OutputFormat::Png => "image/png",
            OutputFormat::Jpg => "image/jpeg",
            OutputFormat::Gif => "image/gif",
            OutputFormat::Mp4 => "video/mp4",
            OutputFormat::Webp => "image/webp",
            OutputFormat::Glb => "model/gltf-binary",
        }
    }

    /// **文件后缀** (含 `.`)
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Png => ".png",
            OutputFormat::Jpg => ".jpg",
            OutputFormat::Gif => ".gif",
            OutputFormat::Mp4 => ".mp4",
            OutputFormat::Webp => ".webp",
            OutputFormat::Glb => ".glb",
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Png
    }
}

// ============================================================
// GenRequest (5 fields)
// ============================================================

/// **GenRequest — dispatcher 入参** (5 fields)
///
/// **字段**:
/// - `plugin` — 路由到哪个 Gen 插件 (9 variant)
/// - `prompt` — 文本 prompt (必填)
/// - `seed` — 可选 seed (per VCP `Plugin/FluxGen*/plugin-manifest.json:seed`)
/// - `size` — 可选 (width, height), 默认 1024x1024
/// - `output_format` — 默认 Png
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenRequest {
    /// 路由到哪个 Gen 插件
    pub plugin: GenPlugin,
    /// 文本 prompt (必填)
    pub prompt: String,
    /// 可选 seed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// 可选 (width, height), 默认 1024x1024
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(u32, u32)>,
    /// 输出格式, 默认 Png
    #[serde(default)]
    pub output_format: OutputFormat,
}

impl GenRequest {
    /// **新建 (plugin + prompt, 其他默认)**
    pub fn new(plugin: GenPlugin, prompt: impl Into<String>) -> Self {
        Self {
            plugin,
            prompt: prompt.into(),
            seed: None,
            size: None,
            output_format: OutputFormat::default(),
        }
    }

    /// **chain 风格 builder**
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_size(mut self, w: u32, h: u32) -> Self {
        self.size = Some((w, h));
        self
    }

    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// **resolve size (None → DEFAULT_SIZE)**
    pub fn resolve_size(&self) -> (u32, u32) {
        self.size.unwrap_or(DEFAULT_SIZE)
    }
}

// ============================================================
// GenResponse (5 fields, 0 假设哪种 url)
// ============================================================

/// **GenResponse — dispatcher 出参** (5 fields)
///
/// **0 假设哪种 url**:
/// - 静态图/视频/3D 模型 3 种都可能, 3 个 `Option<String>` 字段并存
/// - R124+ 真接时填充对应字段
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenResponse {
    /// 哪个 plugin 返的
    pub plugin: GenPlugin,
    /// 静态图 URL (Png / Jpg / Gif / Webp)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// 视频 URL (Mp4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    /// 3D 模型 URL (Glb)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,
    /// 实际用的 seed (per VCP `Plugin/FluxGen*/plugin-manifest.json:seed_used`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_used: Option<u64>,
}

impl GenResponse {
    /// **空响应 (R123-4 dispatcher 永远 Err, 这仅供 test 用)**
    pub fn empty(plugin: GenPlugin) -> Self {
        Self {
            plugin,
            image_url: None,
            video_url: None,
            model_url: None,
            seed_used: None,
        }
    }
}

// ============================================================
// plugin_endpoint (per plugin URL 模板, 0 真连)
// ============================================================

/// **per plugin URL 模板** (0 真连, 仅占位)
///
/// **借鉴**:
/// - ComfyUI / Agnes / AgnesVideo → 本地端口 (8188 / 7860)
/// - Flux / Doubao / GPTImage / Gemini → 公开 API (R124+ 真接时填 API key)
/// - DMX / NanoBanana → 占位 (R124+ 真接时再定)
pub fn plugin_endpoint(plugin: GenPlugin) -> &'static str {
    match plugin {
        GenPlugin::ComfyUI => "http://localhost:8188/prompt",
        GenPlugin::Flux => "https://api.flux.ai/v1/generate",
        GenPlugin::Doubao => "https://ark.cn-beijing.volces.com/api/v3/images/generations",
        GenPlugin::GPTImage => "https://api.openai.com/v1/images/generations",
        GenPlugin::Gemini => "https://generativelanguage.googleapis.com/v1beta/models",
        GenPlugin::Agnes => "http://localhost:7860/sdg/prompt",
        GenPlugin::AgnesVideo => "http://localhost:7860/sdg/video",
        GenPlugin::DMX => "https://api.deepmindx.com/v1/generate",
        GenPlugin::NanoBanana => "https://api.nanobanana.ai/v1/generate",
    }
}

// ============================================================
// gen_dispatch (template, 0 真接, 永远 Err)
// ============================================================

/// **多模态 dispatcher** (R123-4 template placeholder, 0 真接)
///
/// **0 真接**: 不调 reqwest, 不调任何 HTTP client, 直接 `Err`
///
/// **R124+ 真接**:
/// - 9 plugin 各 1 个 sub-backend (ComfyUI / Flux / Doubao / ...), 1 个 1 个接
/// - 每个 backend 1 个 `pub async fn real_dispatch(req: GenRequest) -> Result<GenResponse, String>`
/// - 本 fn 改为 `match req.plugin { ComfyUI => comfy_dispatch(req).await, Flux => flux_dispatch(req).await, ... }`
pub fn gen_dispatch(req: GenRequest) -> Result<GenResponse, String> {
    // O-5 诚实标缺
    Err(format!(
        "plugin '{}' not connected (R123-4 template placeholder, R124+ 真接 via apeireth-mcp multimodal sub-backends)",
        req.plugin.as_str()
    ))
}

// ============================================================
// mcp tool handler bridge (serde bridge for handler_from_fn)
// ============================================================

/// **mcp tool handler bridge** — sync serde bridge
///
/// **设计**:
/// - 输入 `Value` (MCP 端传 JSON args)
/// - 内部反序列化为 `GenRequest`
/// - 调 `gen_dispatch` (永远 Err per R123-4 template)
/// - **R124+ 真接**: 改为 `gen_dispatch(req).map(|r| serde_json::to_value(r).unwrap())`
///
/// **用法 (in example / TUI)**:
/// ```ignore
/// use apeireth_mcp::multimodal::{dispatch_multimodal_handler_async, multimodal_tool_def};
/// use apeireth_mcp::tool_bridge::handler_from_fn;
/// let def = multimodal_tool_def();
/// let handler = handler_from_fn(dispatch_multimodal_handler_async);
/// server.register_tool(def, handler);
/// ```
pub fn dispatch_multimodal_handler(args: Value) -> Result<Value, String> {
    let req: GenRequest =
        serde_json::from_value(args).map_err(|e| format!("invalid GenRequest JSON: {e}"))?;
    // 0 真接 — O-5 标缺
    // R124+ 真接时改为: gen_dispatch(req).and_then(|r| serde_json::to_value(r).map_err(|e| e.to_string()))
    Err(format!(
        "plugin '{}' not connected (R123-4 template placeholder, R124+ 真接)",
        req.plugin.as_str()
    ))
}

/// **mcp tool handler bridge (async 版)** — 给 `tool_bridge::handler_from_fn` 用
///
/// `handler_from_fn` 要求 `Fn(Value) -> Future<Result<Value, String>>`. 本 async wrapper
/// 内部调 sync 版 `dispatch_multimodal_handler`, 保持 R124+ 真接时只改 1 处.
pub async fn dispatch_multimodal_handler_async(args: Value) -> Result<Value, String> {
    dispatch_multimodal_handler(args)
}

// ============================================================
// multimodal_tool_def (给 McpServer::register_tool 用)
// ============================================================

/// **multimodal tool 的 ToolDef** (mcp tools/list 返回项)
///
/// **ToolDef.name** = `"multimodal"` (跟 dispatch_multimodal_handler 配对)
pub fn multimodal_tool_def() -> ToolDef {
    ToolDef {
        name: "multimodal".to_string(),
        description: format!(
            "Multimodal generation dispatcher (R123-4 template, 0 真接, R124+ 真接). Routes to 9 Gen plugins: {}",
            GenPlugin::ALL
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        inputSchema: json!({
            "type": "object",
            "properties": {
                "plugin": {
                    "type": "string",
                    "enum": GenPlugin::ALL.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
                    "description": "Which Gen plugin to route to"
                },
                "prompt": {
                    "type": "string",
                    "description": "Text prompt for generation"
                },
                "seed": {
                    "type": "integer",
                    "description": "Optional seed for reproducibility (u64)",
                    "minimum": 0
                },
                "size": {
                    "type": "array",
                    "description": "Optional [width, height], default 1024x1024",
                    "items": {"type": "integer", "minimum": 1},
                    "minItems": 2,
                    "maxItems": 2
                },
                "output_format": {
                    "type": "string",
                    "enum": OutputFormat::ALL.iter().map(|f| format!("{:?}", f).to_lowercase()).collect::<Vec<_>>(),
                    "description": "Output format, default png"
                }
            },
            "required": ["plugin", "prompt"]
        }),
    }
}

// ============================================================
// 单元测试 (8+, 0 依赖网络)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- GenPlugin 9 variants -----

    #[test]
    fn gen_plugin_9_variants_distinct_endpoints() {
        // 9 plugin 各自有独立 endpoint (0 重复)
        let endpoints: Vec<&str> = GenPlugin::ALL.iter().map(|p| plugin_endpoint(*p)).collect();
        let unique: std::collections::HashSet<&str> = endpoints.iter().copied().collect();
        assert_eq!(
            unique.len(),
            9,
            "9 plugin should have 9 unique endpoints, got {}",
            endpoints.len()
        );
        for (i, p) in GenPlugin::ALL.iter().enumerate() {
            assert!(
                !plugin_endpoint(*p).is_empty(),
                "endpoint for {p:?} should not be empty"
            );
            // 必须是 http:// 或 https://
            let url = plugin_endpoint(*p);
            assert!(
                url.starts_with("http://") || url.starts_with("https://"),
                "endpoint for {p:?} should start with http(s)://, got {url}"
            );
            // 全部 9 个 plugin 都在 ALL 列表
            assert_eq!(endpoints[i], plugin_endpoint(*p));
        }
    }

    #[test]
    fn gen_plugin_as_str_9_distinct() {
        let names: Vec<&str> = GenPlugin::ALL.iter().map(|p| p.as_str()).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 9, "9 plugin should have 9 unique names");
        // 关键名字检查 (per 07 §2 P2-13 列表)
        assert!(names.contains(&"comfyui"));
        assert!(names.contains(&"flux"));
        assert!(names.contains(&"doubao"));
        assert!(names.contains(&"gpt_image"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"agnes"));
        assert!(names.contains(&"agnes_video"));
        assert!(names.contains(&"dmx"));
        assert!(names.contains(&"nano_banana"));
    }

    // ----- GenRequest serde -----

    #[test]
    fn gen_request_serde_round_trip_with_all_fields() {
        let req = GenRequest::new(GenPlugin::Flux, "a cyberpunk cat wearing neon")
            .with_seed(42)
            .with_size(512, 768)
            .with_output_format(OutputFormat::Webp);
        let json_str = serde_json::to_string(&req).unwrap();
        let back: GenRequest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(req, back);
        // 关键字段都序列化进去了
        let v: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["plugin"], "flux");
        assert_eq!(v["prompt"], "a cyberpunk cat wearing neon");
        assert_eq!(v["seed"], 42);
        assert_eq!(v["size"], serde_json::json!([512, 768]));
        assert_eq!(v["output_format"], "webp");
    }

    #[test]
    fn gen_request_seed_optional_works() {
        // None case
        let req_none = GenRequest::new(GenPlugin::Doubao, "test");
        assert!(req_none.seed.is_none());
        let v_none = serde_json::to_value(&req_none).unwrap();
        assert!(
            v_none.get("seed").is_none(),
            "None seed should be skipped via skip_serializing_if"
        );
        // Some case
        let req_some = GenRequest::new(GenPlugin::Doubao, "test").with_seed(1234567890);
        let v_some = serde_json::to_value(&req_some).unwrap();
        assert_eq!(v_some["seed"], 1234567890);
    }

    #[test]
    fn gen_request_size_default_1024x1024() {
        // DEFAULT_SIZE 常量
        assert_eq!(DEFAULT_SIZE, (1024, 1024));
        // None size → resolve_size = DEFAULT_SIZE
        let req = GenRequest::new(GenPlugin::ComfyUI, "x");
        assert_eq!(req.resolve_size(), (1024, 1024));
        // Some size → resolve_size = Some
        let req2 = GenRequest::new(GenPlugin::ComfyUI, "x").with_size(2048, 1024);
        assert_eq!(req2.resolve_size(), (2048, 1024));
    }

    // ----- gen_dispatch 永远 Err (O-5) -----

    #[test]
    fn gen_dispatch_returns_not_connected_error_for_all_9_plugins() {
        for plugin in GenPlugin::ALL.iter() {
            let req = GenRequest::new(*plugin, "test prompt");
            let err = gen_dispatch(req).unwrap_err();
            assert!(
                err.contains("not connected"),
                "dispatch for {plugin:?} should say 'not connected', got: {err}"
            );
            assert!(
                err.contains(plugin.as_str()),
                "dispatch error for {plugin:?} should contain plugin name '{}', got: {err}",
                plugin.as_str()
            );
        }
    }

    #[test]
    fn dispatch_multimodal_handler_returns_not_connected_error() {
        // 给 mcp handler 用的 wrapper
        let args = json!({
            "plugin": "gpt_image",
            "prompt": "test",
            "size": [512, 512],
            "output_format": "png"
        });
        let err = dispatch_multimodal_handler(args).unwrap_err();
        assert!(err.contains("not connected"));
        assert!(err.contains("gpt_image"));
    }

    // ----- OutputFormat 6 variants -----

    #[test]
    fn output_format_6_variants_serialize_correctly() {
        // 6 variant 都对
        let cases = [
            (OutputFormat::Png, "png"),
            (OutputFormat::Jpg, "jpg"),
            (OutputFormat::Gif, "gif"),
            (OutputFormat::Mp4, "mp4"),
            (OutputFormat::Webp, "webp"),
            (OutputFormat::Glb, "glb"),
        ];
        for (fmt, expected_str) in cases {
            let s = serde_json::to_string(&fmt).unwrap();
            assert_eq!(s, format!("\"{expected_str}\""), "serialize {fmt:?}");
            let back: OutputFormat = serde_json::from_str(&s).unwrap();
            assert_eq!(fmt, back, "round trip {fmt:?}");
        }
        // Default = Png
        assert_eq!(OutputFormat::default(), OutputFormat::Png);
        // ALL 6
        assert_eq!(OutputFormat::ALL.len(), 6);
    }

    #[test]
    fn output_format_mime_and_extension() {
        assert_eq!(OutputFormat::Png.mime_type(), "image/png");
        assert_eq!(OutputFormat::Png.extension(), ".png");
        assert_eq!(OutputFormat::Jpg.mime_type(), "image/jpeg");
        assert_eq!(OutputFormat::Mp4.mime_type(), "video/mp4");
        assert_eq!(OutputFormat::Webp.mime_type(), "image/webp");
        assert_eq!(OutputFormat::Glb.mime_type(), "model/gltf-binary");
        assert_eq!(OutputFormat::Glb.extension(), ".glb");
    }

    // ----- GenResponse -----

    #[test]
    fn gen_response_serializes_with_image_url_field() {
        let resp = GenResponse {
            plugin: GenPlugin::Flux,
            image_url: Some("https://example.com/flux/abc.png".to_string()),
            video_url: None,
            model_url: None,
            seed_used: Some(42),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["plugin"], "flux");
        assert_eq!(v["image_url"], "https://example.com/flux/abc.png");
        assert!(
            v.get("video_url").is_none(),
            "None video_url should be skipped"
        );
        assert!(
            v.get("model_url").is_none(),
            "None model_url should be skipped"
        );
        assert_eq!(v["seed_used"], 42);
    }

    // ----- multimodal tool def (mcp tool list 注册检查) -----

    #[test]
    fn multimodal_tool_listed_in_mcp_registry() {
        // multimodal_tool_def 返 ToolDef, name = "multimodal"
        let def = multimodal_tool_def();
        assert_eq!(def.name, "multimodal");
        assert!(!def.description.is_empty());
        assert!(
            def.description.contains("Flux")
                || def.description.contains("comfyui")
                || def.description.contains("doubao")
        );
        // inputSchema 必填 plugin + prompt
        assert_eq!(def.inputSchema["type"], "object");
        assert!(def.inputSchema["properties"]["plugin"].is_object());
        assert!(def.inputSchema["properties"]["prompt"].is_object());
        let required = def.inputSchema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "plugin"));
        assert!(required.iter().any(|v| v == "prompt"));
        // plugin enum 9 个
        let plugin_enum = def.inputSchema["properties"]["plugin"]["enum"]
            .as_array()
            .unwrap();
        assert_eq!(plugin_enum.len(), 9);
    }

    // ----- 编译期 hardcode 守 -----

    #[test]
    fn compile_time_constants_check() {
        assert_eq!(GEN_PLUGIN_COUNT, 9);
        assert_eq!(OUTPUT_FORMAT_COUNT, 6);
        assert_eq!(DEFAULT_SIZE, (1024, 1024));
        assert_eq!(GenPlugin::ALL.len(), 9);
        assert_eq!(OutputFormat::ALL.len(), 6);
    }
}
