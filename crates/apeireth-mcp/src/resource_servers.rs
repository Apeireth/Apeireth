//! R33-3-1: 3 真接 ResourceServer impl — File / Organ / Convention
//!
//! **设计**: 复用 R33-3 spec (resources/list + resources/read) + R33-1 conventions_scanner
//! 真接, 给 Apeireth 提供 MCP protocol 层可挂的 3 个真资源 server.
//!
//! **URI 命名空间** (仿 MCP 2025-03-26 spec §resources naming):
//! - `file:///<path>` — FileResourceServer (受限 base dir, 防越界)
//! - `organ://<organ_name>` — OrganResourceServer (TUI 9 organ 静态 metadata)
//! - `convention://<key>` — ConventionResourceServer (复用 R33-1 ProjectConventions)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 MCP 协议基础 (resources.rs 0 触碰)
//! - 0 引入网络 / 异步 (全 sync, 0 I/O 阻塞, 0 spawn)
//! - 0 业务耦合 (apeireth-mcp 加 apeireth-tools 依赖仅用于 ConventionResourceServer
//!   复用 R33-1 scanner, 非 apeireth-tui/api 耦合)
//!
//! **借鉴**:
//! - MCP spec §resources (2025-03-26) URI 命名约定
//! - LSP `file://` URI 风格
//! - R33-1 Aider conventions_scanner (ProjectConventions::scan + to_system_prompt_block)
//! - VCP `routes/pluginStatic.js` (Plugin 静态资源暴露模式, 单层 dispatch > 多层代理)

#![allow(clippy::result_large_err)]

use std::path::{Component, Path, PathBuf};

use apeireth_tools::conventions_scanner::ProjectConventions;
use serde_json::json;

use crate::protocol::JsonRpcError;
use crate::resources::{
    Resource, ResourceContent, ResourceServer, RESOURCE_INVALID_URI, RESOURCE_NOT_FOUND,
    RESOURCE_READ_FAILED,
};

// ============================================================
// FileResourceServer — 受限文件系统暴露
// ============================================================

/// FileResourceServer: 暴露 base_dir 下的文件, 防越界 (`..` / 绝对路径拦截)
///
/// **安全**:
/// - `read()` 不允许 URI 含 `..` 或 absolute path component
/// - canonicalize 后必须 still 在 base_dir 下
/// - 单文件最大 1 MiB (防 OOM), 超限返 RESOURCE_READ_FAILED
///
/// **URI 形态**: `file:///<relative-or-absolute-path>` (相对 base_dir)
///
/// **list()**: 递归扫 base_dir, 返所有 regular files (e.g. `file:///src/main.rs`)
#[derive(Debug, Clone)]
pub struct FileResourceServer {
    /// 根目录 (所有 file:// 路径相对此)
    base_dir: PathBuf,
    /// 单文件最大字节数 (防 OOM)
    max_file_bytes: usize,
    /// list() 递归深度上限 (防环 / 超大目录)
    max_depth: usize,
}

impl FileResourceServer {
    /// 创建 server, base_dir 必须存在 (否则返 Err 字符串给调用方, 0 panic)
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, String> {
        let base = base_dir.as_ref().to_path_buf();
        if !base.exists() {
            return Err(format!("base_dir 不存在: {}", base.display()));
        }
        if !base.is_dir() {
            return Err(format!("base_dir 不是目录: {}", base.display()));
        }
        Ok(Self {
            base_dir: base,
            max_file_bytes: 1024 * 1024, // 1 MiB
            max_depth: 8,
        })
    }

    /// 自定义最大文件字节 (默认 1 MiB)
    pub fn with_max_file_bytes(mut self, n: usize) -> Self {
        self.max_file_bytes = n;
        self
    }

    /// 自定义 list() 递归深度 (默认 8)
    pub fn with_max_depth(mut self, n: usize) -> Self {
        self.max_depth = n;
        self
    }

    /// 从 file:// URI 抽 path 部分, 验证无 `..` / 绝对
    ///
    /// 返 `Err(String)` 给 read() 包成 JsonRpcError
    fn extract_path<'a>(&self, uri: &'a str) -> Result<PathBuf, String> {
        let prefix = "file:///";
        let path_str = uri.strip_prefix(prefix).ok_or_else(|| {
            format!("URI 必须以 {prefix} 开头, 实际: {uri}")
        })?;
        // Decode percent-encoding (e.g. %20 → space)
        let decoded = percent_decode(path_str);
        let p = Path::new(&decoded);
        // 拦截绝对路径
        if p.is_absolute() {
            return Err(format!("绝对路径不允许: {decoded}"));
        }
        // 拦截 `..` components
        for comp in p.components() {
            if matches!(comp, Component::ParentDir) {
                return Err(format!("`..` 不允许: {decoded}"));
            }
        }
        Ok(p.to_path_buf())
    }

    /// 安全解析: 跟 base_dir 拼接 → canonicalize → 校验仍在 base 下
    fn resolve_safe(&self, rel: &Path) -> Result<PathBuf, String> {
        let joined = self.base_dir.join(rel);
        let canonical = joined.canonicalize().map_err(|e| {
            format!("canonicalize 失败 ({}): {e}", joined.display())
        })?;
        let base_canonical = self.base_dir.canonicalize().map_err(|e| {
            format!("base canonicalize 失败: {e}")
        })?;
        if !canonical.starts_with(&base_canonical) {
            return Err(format!(
                "路径越界: {} 不在 {} 下",
                canonical.display(),
                base_canonical.display()
            ));
        }
        Ok(canonical)
    }

    /// MIME 猜 (按扩展名, 简单常见类型)
    fn guess_mime(path: &Path) -> &'static str {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => "text/x-rust",
            Some("toml") => "text/x-toml",
            Some("md") => "text/markdown",
            Some("json") => "application/json",
            Some("py") => "text/x-python",
            Some("js") | Some("mjs") => "text/javascript",
            Some("ts") => "text/typescript",
            Some("html") | Some("htm") => "text/html",
            Some("css") => "text/css",
            Some("yaml") | Some("yml") => "text/yaml",
            Some("sh") | Some("bash") => "text/x-shellscript",
            Some("txt") => "text/plain",
            _ => "application/octet-stream",
        }
    }

    /// 递归扫 base_dir, 返所有 regular files (夹在 max_depth 内)
    fn walk_files(&self, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if depth > self.max_depth {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_files(&path, depth + 1, out)?;
            } else if path.is_file() {
                out.push(path);
            }
        }
        Ok(())
    }
}

impl ResourceServer for FileResourceServer {
    fn list(&self) -> Vec<Resource> {
        let mut files = Vec::new();
        if self.walk_files(&self.base_dir, 0, &mut files).is_err() {
            return Vec::new(); // I/O 失败返空, 不假装
        }
        files
            .into_iter()
            .filter_map(|p| {
                let rel = p.strip_prefix(&self.base_dir).ok()?.to_path_buf();
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let uri = format!("file:///{rel_str}");
                let name = rel.file_name()?.to_string_lossy().into_owned();
                Some(
                    Resource::new(uri, name)
                        .with_description(format!("file under {}", self.base_dir.display()))
                        .with_mime_type(Self::guess_mime(&rel).to_string()),
                )
            })
            .collect()
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let rel = self.extract_path(uri).map_err(|e| {
            JsonRpcError::new(RESOURCE_INVALID_URI, format!("file:// URI 解析失败: {e}"))
        })?;
        let canonical = self.resolve_safe(&rel).map_err(|e| {
            JsonRpcError::new(RESOURCE_INVALID_URI, format!("file:// 安全解析失败: {e}"))
        })?;
        let meta = std::fs::metadata(&canonical).map_err(|e| {
            JsonRpcError::new(RESOURCE_READ_FAILED, format!("stat 失败: {e}"))
        })?;
        if !meta.is_file() {
            return Err(JsonRpcError::new(
                RESOURCE_READ_FAILED,
                format!("不是 regular file: {}", canonical.display()),
            ));
        }
        if meta.len() > self.max_file_bytes as u64 {
            return Err(JsonRpcError::new(
                RESOURCE_READ_FAILED,
                format!(
                    "文件过大: {} bytes (上限 {} bytes)",
                    meta.len(),
                    self.max_file_bytes
                ),
            ));
        }
        let text = std::fs::read_to_string(&canonical).map_err(|e| {
            JsonRpcError::new(RESOURCE_READ_FAILED, format!("read 失败 (UTF-8 only): {e}"))
        })?;
        Ok(ResourceContent::new(uri, text).with_mime_type(Self::guess_mime(&rel).to_string()))
    }
}

/// 简单 percent-decode (RFC 3986 unreserved + a-zA-Z0-9 透传, %XX 转字节)
///
/// 不处理 + (form 风格), 只解 %XX, e.g. "hello%20world" → "hello world"
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ============================================================
// OrganResourceServer — TUI 9 organ 静态 metadata
// ============================================================

/// OrganResourceServer: 暴露 TUI 9 organ 的静态 metadata
///
/// **URI 形态**: `organ://<organ_name>` (e.g. `organ://memory`, `organ://hand`)
/// 或 `organ://_all` 一次性读所有 9 organ.
///
/// **0 TUI 耦合**: 仅静态 9 organ 列表 + 状态 marker (R22 ST-A1.8 readiness);
/// 不读 TUI runtime state (那需要 apeireth-tui 依赖, 违背 0 业务耦合).
#[derive(Debug, Clone)]
pub struct OrganResourceServer {
    /// 9 organ 静态 metadata: (name, page_label, description, readiness_marker)
    organs: Vec<OrganMeta>,
}

/// 1 个 organ 的 metadata
#[derive(Debug, Clone)]
struct OrganMeta {
    uri_name: &'static str,
    page_label: &'static str,
    description: &'static str,
    /// R22 ST-A1.8 readiness marker (ok / partial / stub), 静态编译期 hardcode
    readiness: &'static str,
}

/// 9 organ 静态列表 (per R37-2 TUI organ/ 真接清单)
const ORGAN_LIST: &[OrganMeta] = &[
    OrganMeta { uri_name: "body",    page_label: "BODY",    description: "躯体状态 (生命体征 + energy level)", readiness: "ok" },
    OrganMeta { uri_name: "brain",   page_label: "BRAIN",   description: "推理中枢 (思考 / 计划 / 元认知)",       readiness: "ok" },
    OrganMeta { uri_name: "ear",     page_label: "EAR",     description: "听觉 (环境音频 + 输入流)",             readiness: "ok" },
    OrganMeta { uri_name: "eye",     page_label: "EYE",     description: "视觉 (屏幕 + 鼠标 / 键盘)",            readiness: "ok" },
    OrganMeta { uri_name: "hand",    page_label: "HAND",    description: "操作 (工具调用记录 + 日历统计)",       readiness: "ok" },
    OrganMeta { uri_name: "heart",   page_label: "HEART",   description: "情绪 (情绪曲线 + 偏好)",               readiness: "ok" },
    OrganMeta { uri_name: "memory",  page_label: "MEMORY",  description: "记忆 (3 层状态, R22 ST-A1.8)",         readiness: "partial" },
    OrganMeta { uri_name: "mind",    page_label: "MIND",    description: "意向 (3 阶段: init / process / serve)", readiness: "ok" },
    OrganMeta { uri_name: "voice",   page_label: "VOICE",   description: "发声 (TTS + 主动播报)",                readiness: "ok" },
];

impl Default for OrganResourceServer {
    fn default() -> Self {
        Self {
            organs: ORGAN_LIST.to_vec(),
        }
    }
}

impl OrganResourceServer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ResourceServer for OrganResourceServer {
    fn list(&self) -> Vec<Resource> {
        self.organs
            .iter()
            .map(|o| {
                Resource::new(
                    format!("organ://{}", o.uri_name),
                    format!("{} ({})", o.page_label, o.uri_name),
                )
                .with_description(format!(
                    "{} — readiness: {}",
                    o.description, o.readiness
                ))
                .with_mime_type("application/json".to_string())
            })
            .collect()
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let prefix = "organ://";
        let name = uri.strip_prefix(prefix).ok_or_else(|| {
            JsonRpcError::new(
                RESOURCE_INVALID_URI,
                format!("URI 必须以 {prefix} 开头, 实际: {uri}"),
            )
        })?;
        if name == "_all" {
            // 一次返 9 organ JSON 数组
            let arr: Vec<_> = self
                .organs
                .iter()
                .map(|o| {
                    json!({
                        "uri_name": o.uri_name,
                        "page_label": o.page_label,
                        "description": o.description,
                        "readiness": o.readiness,
                    })
                })
                .collect();
            let text = serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string());
            return Ok(ResourceContent::new(uri, text)
                .with_mime_type("application/json".to_string()));
        }
        let organ = self.organs.iter().find(|o| o.uri_name == name).ok_or_else(|| {
            JsonRpcError::new(
                RESOURCE_NOT_FOUND,
                format!("organ 不存在: {name} (已知: {})",
                    self.organs.iter().map(|o| o.uri_name).collect::<Vec<_>>().join(", ")),
            )
        })?;
        let text = serde_json::to_string_pretty(&json!({
            "uri_name": organ.uri_name,
            "page_label": organ.page_label,
            "description": organ.description,
            "readiness": organ.readiness,
            "uri": format!("organ://{}", organ.uri_name),
        }))
        .unwrap_or_else(|_| "{}".to_string());
        Ok(ResourceContent::new(uri, text).with_mime_type("application/json".to_string()))
    }
}

// ============================================================
// ConventionResourceServer — 复用 R33-1 ProjectConventions
// ============================================================

/// ConventionResourceServer: 暴露 R33-1 ProjectConventions 真接数据
///
/// **URI 形态**:
/// - `convention://_summary` — 一行摘要 (e.g. "edition 2021, resolver 2, 36 members, 8 deps")
/// - `convention://_system_prompt_block` — R33-1 Aider 风格的 system prompt block
/// - `convention://_raw_json` — 完整 ProjectConventions JSON
///
/// **lazy**: 不在构造时 scan, 首次 read 时才 scan (0 启动开销)
#[derive(Debug, Clone)]
pub struct ConventionResourceServer {
    workspace_root: PathBuf,
    /// 缓存 scan 结果 (lazy + 一次性)
    cached: std::sync::Arc<std::sync::OnceLock<ProjectConventions>>,
}

impl ConventionResourceServer {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            cached: std::sync::Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// 取 / 触发 ProjectConventions::scan
    fn conv(&self) -> &ProjectConventions {
        self.cached.get_or_init(|| ProjectConventions::scan(&self.workspace_root))
    }
}

impl ResourceServer for ConventionResourceServer {
    fn list(&self) -> Vec<Resource> {
        vec![
            Resource::new(
                "convention://_summary",
                "ProjectConventions.summary (R33-1)",
            )
            .with_description("一行摘要: edition / resolver / members count / deps count")
            .with_mime_type("text/plain".to_string()),
            Resource::new(
                "convention://_system_prompt_block",
                "ProjectConventions.to_system_prompt_block (R33-1 Aider style)",
            )
            .with_description("R33-1 Aider 风格 system prompt block, 直接注入 LLM system message")
            .with_mime_type("text/plain".to_string()),
            Resource::new(
                "convention://_raw_json",
                "ProjectConventions (raw JSON, R33-1)",
            )
            .with_description("完整 ProjectConventions 结构, JSON 序列化 (字段级)")
            .with_mime_type("application/json".to_string()),
        ]
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        let prefix = "convention://";
        let name = uri.strip_prefix(prefix).ok_or_else(|| {
            JsonRpcError::new(
                RESOURCE_INVALID_URI,
                format!("URI 必须以 {prefix} 开头, 实际: {uri}"),
            )
        })?;
        let conv = self.conv(); // lazy + OnceLock
        match name {
            "_summary" => Ok(ResourceContent::new(uri, conv.summary())
                .with_mime_type("text/plain".to_string())),
            "_system_prompt_block" => Ok(ResourceContent::new(uri, conv.to_system_prompt_block())
                .with_mime_type("text/plain".to_string())),
            "_raw_json" => {
                let text = serde_json::to_string_pretty(&json!({
                    "workspace_root": conv.workspace_root,
                    "edition": conv.edition,
                    "rust_version": conv.rust_version,
                    "resolver": conv.resolver,
                    "members_count": conv.members_count,
                    "workspace_deps_count": conv.workspace_deps_count,
                    "lint_categories": conv.lint_categories,
                    "key_deps": conv.key_deps,
                    "scan_error": conv.scan_error,
                }))
                .unwrap_or_else(|_| "{}".to_string());
                Ok(ResourceContent::new(uri, text)
                    .with_mime_type("application/json".to_string()))
            }
            _ => Err(JsonRpcError::new(
                RESOURCE_NOT_FOUND,
                format!(
                    "convention key 不存在: {name} (已知: _summary, _system_prompt_block, _raw_json)"
                ),
            )),
        }
    }
}

// ============================================================
// CompositeResourceServer — 多 server 路由 (按 URI scheme)
// ============================================================

/// CompositeResourceServer: 按 URI scheme (`file://` / `organ://` / `convention://`)
/// 路由到对应 sub-server, 给上层统一 1 个 ResourceServer handle.
///
/// **URI 分发**:
/// - `file://*` → FileResourceServer
/// - `organ://*` → OrganResourceServer
/// - `convention://*` → ConventionResourceServer
/// - 其它 → RESOURCE_INVALID_URI
#[derive(Default)]
pub struct CompositeResourceServer {
    file: Option<FileResourceServer>,
    organ: Option<OrganResourceServer>,
    convention: Option<ConventionResourceServer>,
}

impl CompositeResourceServer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_file(mut self, s: FileResourceServer) -> Self {
        self.file = Some(s);
        self
    }
    pub fn with_organ(mut self, s: OrganResourceServer) -> Self {
        self.organ = Some(s);
        self
    }
    pub fn with_convention(mut self, s: ConventionResourceServer) -> Self {
        self.convention = Some(s);
        self
    }
}

impl ResourceServer for CompositeResourceServer {
    fn list(&self) -> Vec<Resource> {
        let mut all = Vec::new();
        if let Some(s) = &self.file {
            all.extend(s.list());
        }
        if let Some(s) = &self.organ {
            all.extend(s.list());
        }
        if let Some(s) = &self.convention {
            all.extend(s.list());
        }
        all
    }

    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError> {
        if uri.starts_with("file://") {
            return self.file.as_ref().ok_or_else(|| {
                JsonRpcError::new(
                    RESOURCE_INVALID_URI,
                    "file:// scheme 未注册 (CompositeResourceServer 没配 FileResourceServer)",
                )
            })?.read(uri);
        }
        if uri.starts_with("organ://") {
            return self.organ.as_ref().ok_or_else(|| {
                JsonRpcError::new(
                    RESOURCE_INVALID_URI,
                    "organ:// scheme 未注册 (CompositeResourceServer 没配 OrganResourceServer)",
                )
            })?.read(uri);
        }
        if uri.starts_with("convention://") {
            return self.convention.as_ref().ok_or_else(|| {
                JsonRpcError::new(
                    RESOURCE_INVALID_URI,
                    "convention:// scheme 未注册 (CompositeResourceServer 没配 ConventionResourceServer)",
                )
            })?.read(uri);
        }
        Err(JsonRpcError::new(
            RESOURCE_INVALID_URI,
            format!("未知 URI scheme: {uri} (已知: file://, organ://, convention://)"),
        ))
    }
}
// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod resource_servers_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 1 个临时目录, 测试结束自动清理
    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "apeireth-mcp-rs-{}",
                AtomicUsize::fetch_add(&COUNTER, 1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&base).unwrap();
            Self(base)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn write(&self, name: &str, content: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::write(&p, content).unwrap();
            p
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    // ---------- FileResourceServer ----------

    #[test]
    fn file_server_new_rejects_nonexistent() {
        let r = FileResourceServer::new("/this/path/should/not/exist/anywhere_xyz");
        assert!(r.is_err());
    }

    #[test]
    fn file_server_list_and_read() {
        let tmp = TempDir::new();
        tmp.write("hello.rs", "fn main() { println!(\"hi\"); }\n");
        tmp.write("config.toml", "[package]\nname = \"x\"\n");
        let sub = tmp.0.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("inner.md"), "# inner\n").unwrap();

        let s = FileResourceServer::new(tmp.path()).unwrap();
        let resources = s.list();
        assert!(resources.iter().any(|r| r.uri == "file:///hello.rs"));
        assert!(resources.iter().any(|r| r.uri == "file:///config.toml"));
        assert!(resources.iter().any(|r| r.uri == "file:///sub/inner.md"));

        let c = s.read("file:///hello.rs").unwrap();
        assert!(c.text.contains("fn main"));
        assert_eq!(c.mime_type.as_deref(), Some("text/x-rust"));

        let c2 = s.read("file:///config.toml").unwrap();
        assert_eq!(c2.mime_type.as_deref(), Some("text/x-toml"));
    }

    #[test]
    fn file_server_rejects_parent_traversal() {
        let tmp = TempDir::new();
        tmp.write("ok.rs", "ok");
        let s = FileResourceServer::new(tmp.path()).unwrap();
        let r = s.read("file:///../escape.rs");
        assert!(r.is_err());
        let code = r.unwrap_err().code;
        assert_eq!(code, RESOURCE_INVALID_URI);
    }

    #[test]
    fn file_server_rejects_absolute_path() {
        let tmp = TempDir::new();
        let s = FileResourceServer::new(tmp.path()).unwrap();
        let r = s.read("file:///C:/Windows/System32/drivers/etc/hosts");
        // 绝对路径 (Windows / Linux) 都被拒绝
        assert!(r.is_err());
    }

    #[test]
    fn file_server_rejects_missing_file() {
        let tmp = TempDir::new();
        let s = FileResourceServer::new(tmp.path()).unwrap();
        let r = s.read("file:///no_such_file.rs");
        // canonicalize 失败 → RESOURCE_INVALID_URI (走 extract_path / resolve_safe)
        assert!(r.is_err());
    }

    #[test]
    fn file_server_percent_decoding() {
        let tmp = TempDir::new();
        tmp.write("space name.rs", "// has space\n");
        let s = FileResourceServer::new(tmp.path()).unwrap();
        let c = s.read("file:///space%20name.rs").unwrap();
        assert!(c.text.contains("has space"));
    }

    #[test]
    fn file_server_max_bytes_limit() {
        let tmp = TempDir::new();
        let big = "x".repeat(2048);
        tmp.write("big.rs", &big);
        let s = FileResourceServer::new(tmp.path()).unwrap().with_max_file_bytes(100);
        let r = s.read("file:///big.rs");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_READ_FAILED);
    }

    #[test]
    fn file_server_invalid_uri_scheme() {
        let tmp = TempDir::new();
        let s = FileResourceServer::new(tmp.path()).unwrap();
        let r = s.read("organ://memory");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_INVALID_URI);
    }

    // ---------- OrganResourceServer ----------

    #[test]
    fn organ_server_lists_9_organs() {
        let s = OrganResourceServer::new();
        let list = s.list();
        assert_eq!(list.len(), 9);
        assert!(list.iter().any(|r| r.uri == "organ://memory"));
        assert!(list.iter().any(|r| r.uri == "organ://hand"));
        assert!(list.iter().any(|r| r.uri == "organ://brain"));
    }

    #[test]
    fn organ_server_read_single() {
        let s = OrganResourceServer::new();
        let c = s.read("organ://memory").unwrap();
        assert!(c.text.contains("\"uri_name\": \"memory\""));
        assert!(c.text.contains("\"readiness\": \"partial\"")); // R22 ST-A1.8 已知 partial
    }

    #[test]
    fn organ_server_read_all() {
        let s = OrganResourceServer::new();
        let c = s.read("organ://_all").unwrap();
        let arr: serde_json::Value = serde_json::from_str(&c.text).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 9);
    }

    #[test]
    fn organ_server_unknown_returns_not_found() {
        let s = OrganResourceServer::new();
        let r = s.read("organ://nonexistent");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn organ_server_invalid_scheme() {
        let s = OrganResourceServer::new();
        let r = s.read("file:///x");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_INVALID_URI);
    }

    // ---------- ConventionResourceServer ----------

    #[test]
    fn convention_server_list_has_3_keys() {
        let tmp = TempDir::new();
        // 1 个最小 Cargo.toml (scan 不会返 scan_error)
        tmp.write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"a\"]\n\n[workspace.package]\nedition = \"2021\"\nrust-version = \"1.75\"\n\n[workspace.dependencies]\nserde = \"1\"\n",
        );
        let s = ConventionResourceServer::new(tmp.path());
        let list = s.list();
        assert_eq!(list.len(), 3);
        assert!(list.iter().any(|r| r.uri == "convention://_summary"));
        assert!(list.iter().any(|r| r.uri == "convention://_system_prompt_block"));
        assert!(list.iter().any(|r| r.uri == "convention://_raw_json"));
    }

    #[test]
    fn convention_server_read_summary_and_block() {
        let tmp = TempDir::new();
        tmp.write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"a\"]\n\n[workspace.package]\nedition = \"2021\"\nrust-version = \"1.75\"\n\n[workspace.dependencies]\nserde = \"1\"\n",
        );
        let s = ConventionResourceServer::new(tmp.path());
        let summary = s.read("convention://_summary").unwrap();
        assert!(summary.text.contains("2021"));
        assert!(summary.text.contains("2")); // resolver
        let block = s.read("convention://_system_prompt_block").unwrap();
        assert!(block.text.contains("项目约定"));
        let raw = s.read("convention://_raw_json").unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw.text).unwrap();
        assert_eq!(json["edition"], "2021");
        assert_eq!(json["resolver"], "2");
        assert_eq!(json["members_count"], 1);
    }

    #[test]
    fn convention_server_scan_error_visible() {
        let tmp = TempDir::new();
        // No Cargo.toml → scan_error
        let s = ConventionResourceServer::new(tmp.path());
        let raw = s.read("convention://_raw_json").unwrap();
        assert!(raw.text.contains("scan_error"));
        assert!(raw.text.contains("not found"));
    }

    #[test]
    fn convention_server_unknown_key() {
        let tmp = TempDir::new();
        let s = ConventionResourceServer::new(tmp.path());
        let r = s.read("convention://_bogus");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_NOT_FOUND);
    }

    #[test]
    fn convention_server_lazy_idempotent() {
        let tmp = TempDir::new();
        tmp.write(
            "Cargo.toml",
            "[workspace]\nresolver = \"2\"\nmembers = [\"a\"]\n\n[workspace.package]\nedition = \"2021\"\n",
        );
        let s = ConventionResourceServer::new(tmp.path());
        let _ = s.read("convention://_summary").unwrap();
        // 删 Cargo.toml 后, cached 仍返旧值 (OnceLock 一次性)
        fs::remove_file(tmp.path().join("Cargo.toml")).unwrap();
        let again = s.read("convention://_summary").unwrap();
        assert!(again.text.contains("2021")); // cached 旧值
    }

    // ---------- CompositeResourceServer ----------

    #[test]
    fn composite_routes_by_scheme() {
        let tmp = TempDir::new();
        tmp.write("a.rs", "fn a() {}");
        let file = FileResourceServer::new(tmp.path()).unwrap();
        let organ = OrganResourceServer::new();
        let conv = ConventionResourceServer::new(tmp.path());
        let composite = CompositeResourceServer::new()
            .with_file(file)
            .with_organ(organ)
            .with_convention(conv);

        // file://
        let c = composite.read("file:///a.rs").unwrap();
        assert!(c.text.contains("fn a()"));
        // organ://
        let c = composite.read("organ://memory").unwrap();
        assert!(c.text.contains("memory"));
        // convention://
        let c = composite.read("convention://_summary").unwrap();
        // 缺 Cargo.toml → scan_error 字符串
        assert!(!c.text.is_empty(), "convention summary should not be empty");
    }

    #[test]
    fn composite_unknown_scheme_rejected() {
        let composite = CompositeResourceServer::new()
            .with_file(FileResourceServer::new(TempDir::new().path()).unwrap());
        let r = composite.read("http://example.com");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_INVALID_URI);
    }

    #[test]
    fn composite_missing_subserver_rejected() {
        // 没配 organ server → organ:// 拒绝
        let composite = CompositeResourceServer::new();
        let r = composite.read("organ://memory");
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().code, RESOURCE_INVALID_URI);
    }

    #[test]
    fn composite_list_unions_all() {
        let tmp = TempDir::new();
        tmp.write("a.rs", "ok");
        let composite = CompositeResourceServer::new()
            .with_file(FileResourceServer::new(tmp.path()).unwrap())
            .with_organ(OrganResourceServer::new());
        let list = composite.list();
        // 至少 1 file + 9 organs = 10
        assert!(list.len() >= 10);
        assert!(list.iter().any(|r| r.uri.starts_with("file://")));
        assert!(list.iter().any(|r| r.uri.starts_with("organ://")));
    }
}
