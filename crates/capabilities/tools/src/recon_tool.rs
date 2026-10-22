//! Recon 工具的 Canonical 封装（先落地 `re.pack`，其余工具按同一模式接入）。
//!
//! 封装负责：参数 schema、workspace 路径限域、大小上限、Evidence 组装与结果归一。
//! 底层分析逻辑在 [`crate::recon`]，不直接接触运行时/治理。

use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::recon::{analyze_pack, Evidence, EvidenceItem, PackOptions};

/// 单文件分析上限（64 MiB）。
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

/// `re.pack`：字符串 / 熵 / 魔数 / 嵌入雕刻。
pub struct ReconPackTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconPackTool {
    /// 以 `workspace_root` 为限域根构造工具。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.pack").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "minLength": 1,
                    "description": "File path relative to the workspace root (or an absolute path inside it)."
                },
                "min_string_len": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 64,
                    "description": "Shortest printable string to extract (default 4)."
                },
                "max_strings": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65536,
                    "description": "Maximum extracted strings (default 4096)."
                }
            },
            "required": ["path"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }

    fn resolve(&self, raw: &str) -> Result<PathBuf, String> {
        let candidate = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.workspace_root.join(raw)
        };
        let root = self
            .workspace_root
            .canonicalize()
            .map_err(|error| format!("workspace root is not readable: {error}"))?;
        let target = candidate
            .canonicalize()
            .map_err(|error| format!("target path cannot be resolved: {error}"))?;
        if !target.starts_with(&root) {
            return Err("target path is outside the workspace scope".to_string());
        }
        Ok(target)
    }
}

#[async_trait]
impl ToolCapability for ReconPackTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.pack")
            .with_description(
                "Static pack/obfuscation reconnaissance: printable strings, Shannon entropy, \
                 magic detection, embedded-region carving, and packing hints. Read-only; \
                 never executes the target.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.pack arguments must be an object")
                .with_name("re.pack");
        };
        for key in object.keys() {
            if !matches!(key.as_str(), "path" | "min_string_len" | "max_strings") {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("re.pack does not accept argument {key:?}"),
                )
                .with_name("re.pack");
            }
        }
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.pack requires a string path")
                .with_name("re.pack");
        };
        let min_string_len = object
            .get("min_string_len")
            .and_then(Value::as_u64)
            .unwrap_or(4) as usize;
        let max_strings = object
            .get("max_strings")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as usize;
        if !(1..=64).contains(&min_string_len) {
            return ToolResult::permanent_error(&call.id, "min_string_len must be 1..=64")
                .with_name("re.pack");
        }
        if !(1..=65536).contains(&max_strings) {
            return ToolResult::permanent_error(&call.id, "max_strings must be 1..=65536")
                .with_name("re.pack");
        }

        let target = match self.resolve(path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.pack")
            }
        };
        let metadata = match std::fs::metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("cannot stat target: {error}"),
                )
                .with_name("re.pack")
            }
        };
        if metadata.len() > MAX_INPUT_BYTES {
            return ToolResult::permanent_error(
                &call.id,
                format!("target exceeds the {MAX_INPUT_BYTES}-byte input limit"),
            )
            .with_name("re.pack");
        }
        let bytes = match std::fs::read(&target) {
            Ok(bytes) => bytes,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("cannot read target: {error}"),
                )
                .with_name("re.pack")
            }
        };

        let options = PackOptions {
            min_string_len,
            max_strings,
        };
        let report = analyze_pack(&bytes, &options);
        let digest = format!("sha256:{}", hex(&Sha256::digest(&bytes)));
        let claim = format!(
            "{} bytes, entropy {:.2} bit/byte, {} strings, magic {}, packed hint {}",
            report.size,
            report.entropy_bits,
            report.strings.len(),
            report.magic.as_deref().unwrap_or("none"),
            report.packed_hint.as_deref().unwrap_or("none"),
        );
        let evidence = Evidence::new(
            "re.pack",
            "0.1.0",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
        .with_evidence(EvidenceItem::new("file", path).with_digest(digest))
        .with_limitation("static inspection only; the target is never executed");

        ToolResult::ok(
            call.id.clone(),
            serde_json::to_value(&evidence).unwrap_or(Value::Null),
        )
        .with_name("re.pack")
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 在 workspace 根下解析并限域一个路径。
fn resolve_in_root(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let candidate = if Path::new(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        root.join(raw)
    };
    let root = root
        .canonicalize()
        .map_err(|error| format!("workspace root is not readable: {error}"))?;
    let target = candidate
        .canonicalize()
        .map_err(|error| format!("target path cannot be resolved: {error}"))?;
    if !target.starts_with(&root) {
        return Err("target path is outside the workspace scope".to_string());
    }
    Ok(target)
}

/// 按上限读取文件。
fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("cannot stat target: {error}"))?;
    if metadata.len() > max_bytes {
        return Err(format!("target exceeds the {max_bytes}-byte input limit"));
    }
    std::fs::read(path).map_err(|error| format!("cannot read target: {error}"))
}

fn evidence_result(call: &ToolCall, tool: &str, evidence: &Evidence) -> ToolResult {
    ToolResult::ok(
        call.id.clone(),
        serde_json::to_value(evidence).unwrap_or(Value::Null),
    )
    .with_name(tool)
}

/// `re.binfmt`：ELF / PE / Mach-O 静态结构。
pub struct ReconBinFmtTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconBinFmtTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.binfmt").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1,
                    "description": "Binary path relative to the workspace root." }
            },
            "required": ["path"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconBinFmtTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.binfmt")
            .with_description(
                "Static binary structure inspection for ELF, PE and Mach-O: header, sections, \
                 segments, imports and exports. Read-only; the target is never executed.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.binfmt arguments must be an object")
                .with_name("re.binfmt");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.binfmt requires a string path")
                .with_name("re.binfmt");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.binfmt")
            }
        };
        let bytes = match read_bounded(&target, MAX_INPUT_BYTES) {
            Ok(bytes) => bytes,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.binfmt")
            }
        };
        let report = match crate::recon::parse_binary(&bytes) {
            Ok(report) => report,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("binary parse failed: {error}"),
                )
                .with_name("re.binfmt")
            }
        };
        let claim = format!(
            "{} {} / {} bits / {} segments / {} sections",
            report.format,
            report.machine,
            report.bits,
            report.segments.len(),
            report.sections.len()
        );
        let evidence = Evidence::new(
            "re.binfmt",
            "0.1.0",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
        .with_evidence(
            EvidenceItem::new("file", path)
                .with_digest(format!("sha256:{}", hex(&Sha256::digest(&bytes)))),
        )
        .with_limitation("static structure only; no disassembly or execution");
        evidence_result(call, "re.binfmt", &evidence)
    }
}

/// `re.yara`：YARA 子集规则扫描。
pub struct ReconYaraTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconYaraTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.yara").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1,
                    "description": "File to scan, relative to the workspace root." },
                "rules": { "type": "string", "minLength": 1,
                    "description": "YARA-subset rules source (strings/hex/boolean conditions)." }
            },
            "required": ["path", "rules"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconYaraTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.yara")
            .with_description(
                "Scan a file with YARA-subset rules (literal strings, hex with wildcards, \
                 boolean conditions). This is not a full YARA implementation: no regex, \
                 modules or external variables. Read-only.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.yara arguments must be an object")
                .with_name("re.yara");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.yara requires a string path")
                .with_name("re.yara");
        };
        let Some(rules_source) = object.get("rules").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.yara requires rules source")
                .with_name("re.yara");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.yara")
            }
        };
        let bytes = match read_bounded(&target, MAX_INPUT_BYTES) {
            Ok(bytes) => bytes,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.yara")
            }
        };
        let rules = match crate::recon::RuleSet::parse(rules_source) {
            Ok(rules) => rules,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, format!("rule parse failed: {error}"))
                    .with_name("re.yara")
            }
        };
        let report = rules.scan(&bytes);
        let claim = format!(
            "{} rule(s) evaluated, {} matched",
            report.rules,
            report.matches.len()
        );
        let evidence = Evidence::new(
            "re.yara",
            "0.1.0",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
        .with_evidence(
            EvidenceItem::new("file", path)
                .with_digest(format!("sha256:{}", hex(&Sha256::digest(&bytes)))),
        );
        evidence_result(call, "re.yara", &evidence)
    }
}

/// `net.recon`：TCP connect 扫描（scope 由操作员配置，空 scope 拒绝一切）。
pub struct ReconNetTool {
    scope: std::sync::Arc<crate::recon::Scope>,
    id: CapabilityId,
}

impl ReconNetTool {
    /// 构造；`scope` 为空时所有目标都会被拒绝。
    pub fn new(scope: std::sync::Arc<crate::recon::Scope>) -> Self {
        Self {
            scope,
            id: CapabilityId::new("tool.recon.net").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "minLength": 1,
                    "description": "Target IPv4/IPv6 address inside the authorized scope." },
                "ports": { "type": "array", "minItems": 1, "maxItems": 1024,
                    "items": { "type": "integer", "minimum": 1, "maximum": 65535 } },
                "timeout_ms": { "type": "integer", "minimum": 50, "maximum": 5000,
                    "description": "Per-connect timeout (default 300 ms)." }
            },
            "required": ["target", "ports"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconNetTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.net.recon")
            .with_description(
                "TCP connect scan with passive banner capture, restricted to the operator's \
                 authorized scope. No raw sockets and no protocol-specific probes.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "re.net.recon arguments must be an object",
            )
            .with_name("re.net.recon");
        };
        let Some(target) = object.get("target").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.net.recon requires a string target")
                .with_name("re.net.recon");
        };
        let target: std::net::IpAddr = match target.parse() {
            Ok(address) => address,
            Err(_) => {
                return ToolResult::permanent_error(
                    &call.id,
                    "target must be a concrete IPv4/IPv6 address",
                )
                .with_name("re.net.recon")
            }
        };
        let Some(ports) = object.get("ports").and_then(Value::as_array) else {
            return ToolResult::permanent_error(&call.id, "re.net.recon requires a ports array")
                .with_name("re.net.recon");
        };
        let mut requested = Vec::with_capacity(ports.len());
        for port in ports {
            let Some(port) = port.as_u64() else {
                return ToolResult::permanent_error(&call.id, "ports must be integers")
                    .with_name("re.net.recon");
            };
            if !(1..=65535).contains(&port) {
                return ToolResult::permanent_error(&call.id, "port out of range")
                    .with_name("re.net.recon");
            }
            requested.push(port as u16);
        }
        let timeout_ms = object
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(300)
            .clamp(50, 5000);
        let options = crate::recon::ScanOptions {
            timeout: std::time::Duration::from_millis(timeout_ms),
            max_ports: 1024,
            banner_probe: true,
        };
        let report = match crate::recon::scan_ports(target, &requested, &self.scope, &options) {
            Ok(report) => report,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, format!("scan refused: {error}"))
                    .with_name("re.net.recon")
            }
        };
        let claim = format!(
            "{} of {} ports open on {}",
            report.open,
            report.ports.len(),
            report.target
        );
        let evidence = Evidence::new(
            "net.recon",
            "0.1.0",
            report.target.clone(),
            self.scope.name.clone(),
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
        .with_limitation("connect scan only; passive banner only");
        evidence_result(call, "re.net.recon", &evidence)
    }
}

/// `re.dotnet`：CLR 元数据探测。
pub struct ReconDotnetTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconDotnetTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.dotnet").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1,
                    "description": "PE file to inspect, relative to the workspace root." }
            },
            "required": ["path"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconDotnetTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.dotnet")
            .with_description(
                "Detect CLR metadata in a PE file: runtime version, metadata version, entry \
                 token, heap streams, and TypeDef/MethodDef/TypeRef/AssemblyRef tables. \
                 Method signatures and IL bodies are not decoded.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.dotnet arguments must be an object")
                .with_name("re.dotnet");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.dotnet requires a string path")
                .with_name("re.dotnet");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.dotnet")
            }
        };
        let bytes = match read_bounded(&target, MAX_INPUT_BYTES) {
            Ok(bytes) => bytes,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.dotnet")
            }
        };
        let evidence = match crate::recon::detect_dotnet(&bytes) {
            Ok(report) => {
                let claim = format!(
                    ".NET assembly, runtime {}, metadata {}, streams {}",
                    report.runtime_version,
                    report.metadata_version,
                    report.streams.len()
                );
                Evidence::new(
                    "re.dotnet",
                    "0.1.0",
                    target.display().to_string(),
                    "workspace",
                    claim,
                )
                .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
                .with_limitation("method signatures and IL bodies are not decoded")
            }
            Err(crate::recon::DotnetError::NotDotnet) | Err(crate::recon::DotnetError::NotPe) => {
                Evidence::new(
                    "re.dotnet",
                    "0.1.0",
                    target.display().to_string(),
                    "workspace",
                    "not a .NET assembly",
                )
                .with_details(json!({ "is_dotnet": false }))
            }
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("CLR metadata parse failed: {error}"),
                )
                .with_name("re.dotnet")
            }
        };
        evidence_result(call, "re.dotnet", &evidence)
    }
}

/// `re.js`：词法级 JS / Electron 静态观察。
pub struct ReconJsTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconJsTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.js").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1,
                    "description": "JavaScript file to inspect, relative to the workspace root." }
            },
            "required": ["path"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconJsTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.js")
            .with_description(
                "Lexical JavaScript/Electron inspection: string literals, module imports, \
                 sourceMappingURL markers and dynamic-eval hints. Not a full AST analysis.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.js arguments must be an object")
                .with_name("re.js");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.js requires a string path")
                .with_name("re.js");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.js")
            }
        };
        let bytes = match read_bounded(&target, MAX_INPUT_BYTES) {
            Ok(bytes) => bytes,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.js")
            }
        };
        let lossy = String::from_utf8_lossy(&bytes);
        let report = crate::recon::analyze_js(&lossy, &crate::recon::JsOptions::default());
        let claim = format!(
            "{} strings, {} imports, {} dynamic-eval sites",
            report.strings.len(),
            report.imports.len(),
            report.dynamic_eval
        );
        let mut evidence = Evidence::new(
            "re.js",
            "0.1.0",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null));
        if lossy.contains('\u{FFFD}') {
            evidence = evidence.with_limitation("input is not valid UTF-8; bytes were replaced");
        }
        evidence_result(call, "re.js", &evidence)
    }
}

/// `net.proto`：pcap 协议还原。
pub struct ReconPcapTool {
    workspace_root: PathBuf,
    id: CapabilityId,
}

impl ReconPcapTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            id: CapabilityId::new("tool.recon.pcap").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1,
                    "description": "Classic pcap file, relative to the workspace root." }
            },
            "required": ["path"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconPcapTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.pcap")
            .with_description(
                "Parse a classic pcap or pcapng capture into flows and DNS questions. No TCP \
                 reassembly and no TLS decryption.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.pcap arguments must be an object")
                .with_name("re.pcap");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.pcap requires a string path")
                .with_name("re.pcap");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.pcap")
            }
        };
        let bytes = match read_bounded(&target, MAX_INPUT_BYTES) {
            Ok(bytes) => bytes,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("re.pcap")
            }
        };
        let report = match crate::recon::parse_capture(&bytes) {
            Ok(report) => report,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, format!("pcap parse failed: {error}"))
                    .with_name("re.pcap")
            }
        };
        let claim = format!(
            "{} packets, {} flows, {} DNS questions",
            report.packet_count,
            report.flows.len(),
            report.dns_queries.len()
        );
        let evidence = Evidence::new(
            "net.proto",
            "0.1.0",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null));
        evidence_result(call, "re.pcap", &evidence)
    }
}

/// `web.probe`：HTTP 指纹（scope 由操作员配置）。
pub struct ReconWebTool {
    scope: std::sync::Arc<crate::recon::Scope>,
    id: CapabilityId,
}

impl ReconWebTool {
    /// 构造；空 scope 拒绝一切目标。
    pub fn new(scope: std::sync::Arc<crate::recon::Scope>) -> Self {
        Self {
            scope,
            id: CapabilityId::new("tool.recon.web").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "minLength": 1,
                    "description": "http(s) URL inside the authorized scope." }
            },
            "required": ["url"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconWebTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.web.probe")
            .with_description(
                "Single GET probe: status, security headers and technology hints, restricted \
                 to the authorized scope. Redirects are not followed and the body is not read.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "re.web.probe arguments must be an object",
            )
            .with_name("re.web.probe");
        };
        let Some(url) = object.get("url").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.web.probe requires a string url")
                .with_name("re.web.probe");
        };
        let report = match crate::recon::probe_web(
            url,
            &self.scope,
            &crate::recon::WebProbeOptions::default(),
        )
        .await
        {
            Ok(report) => report,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, format!("web probe failed: {error}"))
                    .with_name("re.web.probe")
            }
        };
        let claim = format!(
            "HTTP {} from {}, {} security header(s) present",
            report.status,
            report.server.as_deref().unwrap_or("unknown server"),
            report
                .security_headers
                .values()
                .filter(|value| value.is_some())
                .count()
        );
        let evidence = Evidence::new(
            "web.probe",
            "0.1.0",
            report.url.clone(),
            self.scope.name.clone(),
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null));
        evidence_result(call, "re.web.probe", &evidence)
    }
}

/// `re.tls`：主动 TLS 握手指纹（scope 限域，不读证书链）。
pub struct ReconTlsTool {
    scope: std::sync::Arc<crate::recon::Scope>,
    id: CapabilityId,
}

impl ReconTlsTool {
    /// 构造。
    pub fn new(scope: std::sync::Arc<crate::recon::Scope>) -> Self {
        Self {
            scope,
            id: CapabilityId::new("tool.recon.tls").expect("static capability id"),
        }
    }

    fn parameters() -> ToolParameters {
        let schema = json!({
            "type": "object",
            "properties": {
                "host": { "type": "string", "minLength": 1,
                    "description": "Hostname or IP inside the authorized scope." },
                "port": { "type": "integer", "minimum": 1, "maximum": 65535,
                    "description": "TLS port (default 443)." },
                "sni": { "type": "string",
                    "description": "Optional SNI override (defaults to host when it is a name)." }
            },
            "required": ["host"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(schema.as_object().cloned().unwrap_or_default());
        params
    }
}

#[async_trait]
impl ToolCapability for ReconTlsTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("re.tls")
            .with_description(
                "Active TLS handshake fingerprint: negotiated version, cipher suite, ALPN and \
                 SNI. Does not fetch or validate certificates. Scope-restricted.",
            )
            .with_parameters(Self::parameters())
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "re.tls arguments must be an object")
                .with_name("re.tls");
        };
        let Some(host) = object.get("host").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "re.tls requires a string host")
                .with_name("re.tls");
        };
        let port = object.get("port").and_then(Value::as_u64).unwrap_or(443);
        if !(1..=65535).contains(&port) {
            return ToolResult::permanent_error(&call.id, "port out of range").with_name("re.tls");
        }
        let sni = object.get("sni").and_then(Value::as_str);

        // scope：IP 走网段，域名走域名校验。
        if let Ok(address) = host.parse::<std::net::IpAddr>() {
            if let Err(error) = self.scope.check_address(address) {
                return ToolResult::permanent_error(&call.id, format!("scope refused: {error}"))
                    .with_name("re.tls");
            }
        } else if let Err(error) = self.scope.check_domain(host) {
            return ToolResult::permanent_error(&call.id, format!("scope refused: {error}"))
                .with_name("re.tls");
        }

        let address = match (host, port as u16).to_socket_addrs() {
            Ok(mut addresses) => match addresses.next() {
                Some(address) => address,
                None => {
                    return ToolResult::permanent_error(&call.id, "host did not resolve")
                        .with_name("re.tls")
                }
            },
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("cannot resolve host: {error}"),
                )
                .with_name("re.tls")
            }
        };

        let mut stream =
            match std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3))
            {
                Ok(stream) => stream,
                Err(error) => {
                    return ToolResult::permanent_error(
                        &call.id,
                        format!("connect failed: {error}"),
                    )
                    .with_name("re.tls")
                }
            };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let hello = build_client_hello(sni.or_else(|| {
            if host.parse::<std::net::IpAddr>().is_ok() {
                None
            } else {
                Some(host)
            }
        }));
        if let Err(error) = stream.write_all(&hello) {
            return ToolResult::permanent_error(&call.id, format!("write failed: {error}"))
                .with_name("re.tls");
        }
        let mut buffer = vec![0u8; 16 * 1024];
        let read = match stream.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, format!("read failed: {error}"))
                    .with_name("re.tls")
            }
        };
        buffer.truncate(read);
        let server = match crate::recon::parse_server_hello(&buffer) {
            Ok(server) => server,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("server hello parse failed: {error}"),
                )
                .with_name("re.tls")
            }
        };
        let claim = format!(
            "TLS 0x{:04x}, cipher 0x{:04x}, ALPN {}, SNI {}",
            server.version,
            server.cipher_suite,
            server.alpn.as_deref().unwrap_or("none"),
            sni.unwrap_or(host)
        );
        let details = json!({
            "host": host,
            "port": port,
            "version": server.version,
            "cipher_suite": server.cipher_suite,
            "alpn": server.alpn,
            "supported_version": server.supported_version,
        });
        let evidence = Evidence::new(
            "re.tls",
            "0.1.0",
            format!("{host}:{port}"),
            self.scope.name.clone(),
            claim,
        )
        .with_details(details)
        .with_limitation("no certificate chain or validation (use the openssl adapter)");
        evidence_result(call, "re.tls", &evidence)
    }
}

fn push_extension(out: &mut Vec<u8>, kind: u16, data: &[u8]) {
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&(data.len() as u16).to_be_bytes());
    out.extend_from_slice(data);
}

/// 构造一个最小 ClientHello：TLS1.2 legacy version + TLS1.3 支持 + SNI + h2/http1.1。
fn build_client_hello(sni: Option<&str>) -> Vec<u8> {
    let mut extensions = Vec::new();
    if let Some(name) = sni {
        let name = name.as_bytes();
        let mut data = Vec::new();
        data.extend_from_slice(&((name.len() + 3) as u16).to_be_bytes());
        data.push(0);
        data.extend_from_slice(&(name.len() as u16).to_be_bytes());
        data.extend_from_slice(name);
        push_extension(&mut extensions, 0x0000, &data);
    }
    push_extension(&mut extensions, 0x002b, &[4, 0x03, 0x04, 0x03, 0x03]);
    let mut alpn = Vec::new();
    alpn.extend_from_slice(&9u16.to_be_bytes());
    alpn.push(2);
    alpn.extend_from_slice(b"h2");
    alpn.push(8);
    alpn.extend_from_slice(b"http/1.1");
    push_extension(&mut extensions, 0x0010, &alpn);

    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x03]);
    body.extend_from_slice(&[0x11; 32]);
    body.push(0);
    body.extend_from_slice(&10u16.to_be_bytes());
    for suite in [0x1301u16, 0x1302, 0x1303, 0xc02f, 0xc030] {
        body.extend_from_slice(&suite.to_be_bytes());
    }
    body.push(1);
    body.push(0);
    body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    body.extend_from_slice(&extensions);

    let mut handshake = vec![1u8];
    handshake.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
    handshake.extend_from_slice(&body);
    let mut record = vec![0x16, 0x03, 0x01];
    record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
    record.extend_from_slice(&handshake);
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-recon-tool-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    fn call(arguments: Value) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            name: "re.pack".to_string(),
            arguments,
        }
    }

    #[tokio::test]
    async fn pack_tool_returns_evidence() {
        let workspace = temp_workspace("pack");
        let sample = workspace.join("sample.bin");
        let mut data = b"\x7fELF".to_vec();
        data.extend_from_slice(&[0u8; 64]);
        data.extend_from_slice(b"hello world");
        std::fs::write(&sample, &data).expect("write sample");

        let tool = ReconPackTool::new(&workspace);
        let result = tool.invoke(&call(json!({"path": "sample.bin"}))).await;
        assert!(result.is_ok(), "{result:?}");
        let ToolOutcome::Ok { value } = result.outcome else {
            panic!("expected ok outcome");
        };
        assert_eq!(value["tool"], "re.pack");
        assert_eq!(value["details"]["magic"], "elf");
        assert!(value["evidence"][0]["digest"]
            .as_str()
            .unwrap_or_default()
            .starts_with("sha256:"));
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn path_outside_workspace_is_rejected() {
        let workspace = temp_workspace("scope");
        let tool = ReconPackTool::new(&workspace);
        let result = tool.invoke(&call(json!({"path": "/etc/hostname"}))).await;
        assert!(!result.is_ok(), "path must be rejected: {result:?}");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn unknown_arguments_are_rejected() {
        let workspace = temp_workspace("args");
        let tool = ReconPackTool::new(&workspace);
        let result = tool
            .invoke(&call(json!({"path": "x", "unexpected": true})))
            .await;
        assert!(!result.is_ok());
        std::fs::remove_dir_all(&workspace).ok();
    }
}

/// 构造全部 8 个 recon 工具：文件类以 `workspace_root` 限域，
/// 网络/Web 类以操作员提供的 [`crate::recon::Scope`] 限域（空 scope 拒绝一切）。
pub fn recon_tool_set(
    workspace_root: impl Into<PathBuf>,
    scope: std::sync::Arc<crate::recon::Scope>,
) -> Vec<std::sync::Arc<dyn ToolCapability>> {
    let root = workspace_root.into();
    vec![
        std::sync::Arc::new(ReconPackTool::new(root.clone())),
        std::sync::Arc::new(ReconBinFmtTool::new(root.clone())),
        std::sync::Arc::new(ReconYaraTool::new(root.clone())),
        std::sync::Arc::new(ReconDotnetTool::new(root.clone())),
        std::sync::Arc::new(ReconJsTool::new(root.clone())),
        std::sync::Arc::new(ReconPcapTool::new(root)),
        std::sync::Arc::new(ReconNetTool::new(std::sync::Arc::clone(&scope))),
        std::sync::Arc::new(ReconWebTool::new(std::sync::Arc::clone(&scope))),
        std::sync::Arc::new(ReconTlsTool::new(scope)),
    ]
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;
    use std::net::TcpListener;

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-recon-wrap-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    fn call(name: &str, arguments: Value) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            name: name.to_string(),
            arguments,
        }
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok outcome, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn binfmt_wrapper_reports_elf() {
        let workspace = temp_workspace("binfmt");
        // 最小 ELF64 头（无节/无段），避免依赖运行环境里的二进制大小。
        let mut elf = vec![0u8; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2; // 64-bit
        elf[5] = 1; // little endian
        elf[6] = 1; // version
        elf[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
        elf[18..20].copy_from_slice(&0x3eu16.to_le_bytes()); // x86_64
        elf[24..32].copy_from_slice(&0x401000u64.to_le_bytes()); // entry
        elf[52..54].copy_from_slice(&64u16.to_le_bytes()); // ehsize
        elf[58..60].copy_from_slice(&64u16.to_le_bytes()); // shentsize
        std::fs::write(workspace.join("self.bin"), &elf).expect("write");
        let tool = ReconBinFmtTool::new(&workspace);
        let value = ok_value(
            tool.invoke(&call("re.binfmt", json!({"path": "self.bin"})))
                .await,
        );
        assert_eq!(value["details"]["format"], "elf");
        assert_eq!(value["details"]["machine"], "x86_64");
        assert_eq!(value["tool"], "re.binfmt");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn yara_wrapper_matches_rules() {
        let workspace = temp_workspace("yara");
        std::fs::write(workspace.join("sample.txt"), b"contains malware here").expect("write");
        let tool = ReconYaraTool::new(&workspace);
        let rules = r#"rule hit { strings: $a = "malware" condition: $a }"#;
        let value = ok_value(
            tool.invoke(&call(
                "re.yara",
                json!({"path": "sample.txt", "rules": rules}),
            ))
            .await,
        );
        assert_eq!(value["details"]["matches"][0]["rule"], "hit");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn net_wrapper_scans_authorized_target() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.write_all(b"SSH-2.0-wrap\r\n");
            }
        });
        let scope = std::sync::Arc::new(crate::recon::Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec!["127.0.0.1/32".into()],
        });
        let tool = ReconNetTool::new(scope);
        let value = ok_value(
            tool.invoke(&call(
                "re.net.recon",
                json!({"target": "127.0.0.1", "ports": [port]}),
            ))
            .await,
        );
        assert_eq!(value["details"]["open"], 1);
        handle.join().ok();
    }

    #[tokio::test]
    async fn empty_scope_denies_network_tools() {
        let scope = std::sync::Arc::new(crate::recon::Scope::empty("empty"));
        let tool = ReconNetTool::new(scope);
        let result = tool
            .invoke(&call(
                "re.net.recon",
                json!({"target": "127.0.0.1", "ports": [80]}),
            ))
            .await;
        assert!(!result.is_ok(), "{result:?}");
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-recon-reg-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    fn call(name: &str, arguments: Value) -> ToolCall {
        ToolCall {
            id: "call_1".to_string(),
            name: name.to_string(),
            arguments,
        }
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok outcome, got {other:?}"),
        }
    }

    fn scoped_to_localhost() -> std::sync::Arc<crate::recon::Scope> {
        std::sync::Arc::new(crate::recon::Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec!["127.0.0.1/32".into()],
        })
    }

    #[test]
    fn tool_set_exposes_all_nine() {
        let workspace = temp_workspace("set");
        let tools = recon_tool_set(&workspace, scoped_to_localhost());
        assert_eq!(tools.len(), 9);
        let mut ids: Vec<String> = tools
            .iter()
            .map(|tool| tool.id().as_str().to_string())
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 9);
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn js_wrapper_extracts_imports_and_eval() {
        let workspace = temp_workspace("js");
        std::fs::write(
            workspace.join("app.js"),
            b"const fs = require('node:fs'); eval('1+1');",
        )
        .expect("write");
        let tool = ReconJsTool::new(&workspace);
        let value = ok_value(tool.invoke(&call("re.js", json!({"path": "app.js"}))).await);
        assert_eq!(value["details"]["imports"][0], "node:fs");
        assert_eq!(value["details"]["dynamic_eval"], 1);
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn dotnet_wrapper_reports_negative_result_as_evidence() {
        let workspace = temp_workspace("dotnet");
        std::fs::write(workspace.join("plain.txt"), b"not a pe").expect("write");
        let tool = ReconDotnetTool::new(&workspace);
        let value = ok_value(
            tool.invoke(&call("re.dotnet", json!({"path": "plain.txt"})))
                .await,
        );
        assert_eq!(value["details"]["is_dotnet"], false);
        assert!(value["claim"]
            .as_str()
            .unwrap_or_default()
            .contains("not a .NET"));
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn web_wrapper_probes_scoped_host() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nServer: fixture/1\r\nContent-Length: 0\r\n\r\n",
                );
            }
        });
        let tool = ReconWebTool::new(scoped_to_localhost());
        let value = ok_value(
            tool.invoke(&call(
                "re.web.probe",
                json!({ "url": format!("http://127.0.0.1:{port}/") }),
            ))
            .await,
        );
        assert_eq!(value["details"]["status"], 200);
        handle.join().ok();
    }
}

#[cfg(test)]
mod tls_tool_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn server_hello() -> Vec<u8> {
        let mut extensions = Vec::new();
        // supported_versions: TLS 1.3
        extensions.extend_from_slice(&[0x00, 0x2b, 0x00, 0x02, 0x03, 0x04]);
        // ALPN "h2"
        let alpn = [0x00u8, 0x03, 0x02, b'h', b'2'];
        extensions.extend_from_slice(&[0x00, 0x10, 0x00, alpn.len() as u8]);
        extensions.extend_from_slice(&alpn);

        let mut body = vec![0x03, 0x03];
        body.extend_from_slice(&[0x22; 32]);
        body.push(0);
        body.extend_from_slice(&0x1301u16.to_be_bytes());
        body.push(0);
        body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        body.extend_from_slice(&extensions);

        let mut handshake = vec![2u8];
        handshake.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        handshake.extend_from_slice(&body);
        let mut record = vec![0x16, 0x03, 0x03];
        record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake);
        record
    }

    #[tokio::test]
    async fn tls_tool_reports_negotiated_parameters() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 4096];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(&server_hello());
            }
        });
        let scope = std::sync::Arc::new(crate::recon::Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec!["127.0.0.1/32".into()],
        });
        let tool = ReconTlsTool::new(scope);
        let result = tool
            .invoke(&ToolCall {
                id: "call_tls".to_string(),
                name: "re.tls".to_string(),
                arguments: json!({"host": "127.0.0.1", "port": port}),
            })
            .await;
        let ToolOutcome::Ok { value } = result.outcome else {
            panic!("expected ok, got {result:?}");
        };
        assert_eq!(value["details"]["cipher_suite"], 0x1301);
        assert_eq!(value["details"]["alpn"], "h2");
        handle.join().ok();
    }

    #[tokio::test]
    async fn tls_tool_rejects_out_of_scope_host() {
        let scope = std::sync::Arc::new(crate::recon::Scope::empty("empty"));
        let tool = ReconTlsTool::new(scope);
        let result = tool
            .invoke(&ToolCall {
                id: "call_tls".to_string(),
                name: "re.tls".to_string(),
                arguments: json!({"host": "192.0.2.1"}),
            })
            .await;
        assert!(!result.is_ok());
    }
}
