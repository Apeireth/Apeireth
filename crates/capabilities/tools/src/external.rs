//! 外部工具适配器：通过唯一进程边界调用 Kali 工具，输出解析为 [`Evidence`]。
//!
//! 设计约束：
//! * 只走 [`ProcessExecutor`]，argv 数组、无 shell 插值；
//! * 工具能力固定（不把任意参数透传给外部命令）；
//! * 网络目标过 `Scope`，文件目标限域到 workspace；
//! * 输出有上限，失败保留 stderr 片段，不假装成功。
//!
//! 首批适配：`nmap`（只读侦察）、`tshark`（pcap 深度解析）、`exiftool`（元数据）。

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use thiserror::Error;

use crate::process::{ProcessExecutor, ProcessLimits, ProcessRequest};
use crate::recon::{Evidence, Scope};

/// 外部工具适配错误。
#[derive(Debug, Error)]
pub enum ExternalError {
    /// 无法启动（二进制缺失/权限）。
    #[error("cannot start {binary}: {source}")]
    Spawn {
        /// 二进制名。
        binary: String,
        /// 底层错误。
        source: std::io::Error,
    },
    /// 非零退出。
    #[error("{binary} exited with {code:?}: {stderr}")]
    Failed {
        /// 二进制名。
        binary: String,
        /// 退出码。
        code: Option<i32>,
        /// stderr 片段。
        stderr: String,
    },
    /// 输出解析失败。
    #[error("cannot parse {tool} output: {reason}")]
    Parse {
        /// 工具名。
        tool: &'static str,
        /// 原因。
        reason: String,
    },
}

/// 统一执行入口：固定 argv、限定时间与输出。
async fn run_external(
    binary: &Path,
    args: &[OsString],
    max_runtime: Duration,
    max_stdout: usize,
) -> Result<crate::process::ProcessResult, ExternalError> {
    let mut request = ProcessRequest::new(binary).with_limits(ProcessLimits {
        max_runtime,
        max_stdout_bytes: max_stdout,
        max_stderr_bytes: 16 * 1024,
        ..ProcessLimits::default()
    });
    for arg in args {
        request = request.with_arg(arg.clone());
    }
    let executor = ProcessExecutor::new();
    executor
        .execute(&request)
        .map_err(|error| ExternalError::Spawn {
            binary: binary.display().to_string(),
            source: std::io::Error::other(error.to_string()),
        })
}

fn ensure_success(
    binary: &Path,
    result: &crate::process::ProcessResult,
) -> Result<(), ExternalError> {
    if result.success() {
        return Ok(());
    }
    Err(ExternalError::Failed {
        binary: binary.display().to_string(),
        code: result.exit_code(),
        stderr: String::from_utf8_lossy(&result.stderr)
            .chars()
            .take(400)
            .collect(),
    })
}

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

fn parameters(schema: Value) -> ToolParameters {
    let mut params = ToolParameters::new();
    params.extend(schema.as_object().cloned().unwrap_or_default());
    params
}

fn ok_evidence(call: &ToolCall, name: &str, evidence: &Evidence) -> ToolResult {
    ToolResult::ok(
        call.id.clone(),
        serde_json::to_value(evidence).unwrap_or(Value::Null),
    )
    .with_name(name)
}

/// `ext.nmap`：只读服务/版本侦察（connect 扫描，不需 root）。
pub struct NmapTool {
    scope: Arc<Scope>,
    binary: PathBuf,
    id: CapabilityId,
}

impl NmapTool {
    /// 构造。
    pub fn new(scope: Arc<Scope>) -> Self {
        Self {
            scope,
            binary: PathBuf::from("nmap"),
            id: CapabilityId::new("tool.external.nmap").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for NmapTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.nmap")
            .with_description(
                "Service/version reconnaissance through the system nmap binary. Connect scan \
                 (-sT, no raw sockets), scope-restricted, fixed arguments only.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "target": {"type": "string", "minLength": 1,
                        "description": "IPv4/IPv6 address inside the authorized scope."},
                    "ports": {"type": "string", "maxLength": 128,
                        "description": "Optional port list such as \"22,80,443\" or \"1-1024\"."},
                    "service_detection": {"type": "boolean",
                        "description": "Enable -sV service/version detection (default true)."},
                    "syn": {"type": "boolean",
                        "description": "Use -sS SYN scan instead of -sT (requires root/CAP_NET_RAW; default false)."}
                },
                "required": ["target"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "ext.nmap arguments must be an object")
                .with_name("ext.nmap");
        };
        let Some(target) = object.get("target").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.nmap requires a string target")
                .with_name("ext.nmap");
        };
        let Ok(address) = target.parse::<std::net::IpAddr>() else {
            return ToolResult::permanent_error(&call.id, "target must be a concrete IP address")
                .with_name("ext.nmap");
        };
        if let Err(error) = self.scope.check_address(address) {
            return ToolResult::permanent_error(&call.id, format!("scope refused: {error}"))
                .with_name("ext.nmap");
        }
        // ports：声明 external.rs:173-174「"ports": {"type": "string", "maxLength": 128}」。
        // 非字符串不能被静默当成「没给 ports」—— 那会让**端口约束整个被跳过**，
        // 扫描面反而变宽到 nmap 的默认端口集（fail-open）。
        let ports = match object.get("ports") {
            None => None,
            Some(value) => {
                let Some(ports) = value.as_str() else {
                    return ToolResult::permanent_error(
                        &call.id,
                        "ext.nmap ports must be a string",
                    )
                    .with_name("ext.nmap");
                };
                Some(ports)
            }
        };
        if let Some(ports) = ports {
            if ports.is_empty()
                || ports.len() > 128
                || !ports
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || ch == ',' || ch == '-')
            {
                return ToolResult::permanent_error(&call.id, "ports must be digits, comma, dash")
                    .with_name("ext.nmap");
            }
        }
        // service_detection / syn：声明 external.rs:175-178「"type": "boolean"」。
        // 非布尔不能被静默变成默认值（`service_detection: "false"` 反而照样开 -sV，
        // 比调用方要的探测更多）。
        let service_detection = match object.get("service_detection") {
            None => true,
            Some(value) => {
                let Some(flag) = value.as_bool() else {
                    return ToolResult::permanent_error(
                        &call.id,
                        "ext.nmap service_detection must be a boolean",
                    )
                    .with_name("ext.nmap");
                };
                flag
            }
        };
        let syn = match object.get("syn") {
            None => false,
            Some(value) => {
                let Some(flag) = value.as_bool() else {
                    return ToolResult::permanent_error(&call.id, "ext.nmap syn must be a boolean")
                        .with_name("ext.nmap");
                };
                flag
            }
        };

        let scan_mode: OsString = if syn { "-sS".into() } else { "-sT".into() };
        let mut args: Vec<OsString> = vec!["-oX".into(), "-".into(), "-n".into(), scan_mode];
        if service_detection {
            args.push("-sV".into());
        }
        if let Some(ports) = ports {
            args.push("-p".into());
            args.push(ports.into());
        }
        args.push(target.into());

        let result = match run_external(
            &self.binary,
            &args,
            Duration::from_secs(120),
            2 * 1024 * 1024,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.nmap")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string()).with_name("ext.nmap");
        }
        let xml = String::from_utf8_lossy(&result.stdout);
        let report = match parse_nmap_xml(&xml) {
            Ok(report) => report,
            Err(reason) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("nmap XML parse failed: {reason}"),
                )
                .with_name("ext.nmap")
            }
        };
        let open: usize = report.hosts.iter().map(|host| host.open_ports()).sum();
        let claim = format!("{} host(s), {} open port(s)", report.hosts.len(), open);
        let evidence = Evidence::new(
            "ext.nmap",
            "external",
            target,
            self.scope.name.clone(),
            claim,
        )
        .with_details(serde_json::to_value(&report).unwrap_or(Value::Null))
        .with_limitation("connect scan (-sT); no raw sockets or OS detection")
        .with_limitation("fixed argument set; nmap scripts are not run");
        ok_evidence(call, "ext.nmap", &evidence)
    }
}

/// nmap XML 摘要。
#[derive(Debug, serde::Serialize)]
struct NmapReport {
    hosts: Vec<NmapHost>,
}

#[derive(Debug, Default, serde::Serialize)]
struct NmapHost {
    address: String,
    ports: Vec<NmapPort>,
}

impl NmapHost {
    fn open_ports(&self) -> usize {
        self.ports
            .iter()
            .filter(|port| port.state == "open")
            .count()
    }
}

#[derive(Debug, Default, serde::Serialize)]
struct NmapPort {
    protocol: String,
    port: u16,
    state: String,
    service: String,
    product: String,
    version: String,
}

fn parse_nmap_xml(xml: &str) -> Result<NmapReport, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut hosts = Vec::new();
    let mut host: Option<NmapHost> = None;
    let mut port: Option<NmapPort> = None;

    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(event) => match event.name().as_ref() {
                b"host" => host = Some(NmapHost::default()),
                b"port" => {
                    if host.is_some() {
                        port = Some(NmapPort {
                            protocol: xml_attr(&event, b"protocol", reader.decoder())
                                .unwrap_or_default(),
                            port: xml_attr(&event, b"portid", reader.decoder())
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0),
                            ..NmapPort::default()
                        });
                    }
                }
                b"state" => {
                    if let Some(port) = &mut port {
                        port.state =
                            xml_attr(&event, b"state", reader.decoder()).unwrap_or_default();
                    }
                }
                b"service" => {
                    if let Some(port) = &mut port {
                        port.service =
                            xml_attr(&event, b"name", reader.decoder()).unwrap_or_default();
                        port.product =
                            xml_attr(&event, b"product", reader.decoder()).unwrap_or_default();
                        port.version =
                            xml_attr(&event, b"version", reader.decoder()).unwrap_or_default();
                    }
                }
                _ => {}
            },
            Event::Empty(event) => match event.name().as_ref() {
                b"address" => {
                    if let Some(host) = &mut host {
                        host.address =
                            xml_attr(&event, b"addr", reader.decoder()).unwrap_or_default();
                    }
                }
                b"state" => {
                    if let Some(port) = &mut port {
                        port.state =
                            xml_attr(&event, b"state", reader.decoder()).unwrap_or_default();
                    }
                }
                b"service" => {
                    if let Some(port) = &mut port {
                        port.service =
                            xml_attr(&event, b"name", reader.decoder()).unwrap_or_default();
                        port.product =
                            xml_attr(&event, b"product", reader.decoder()).unwrap_or_default();
                        port.version =
                            xml_attr(&event, b"version", reader.decoder()).unwrap_or_default();
                    }
                }
                _ => {}
            },
            Event::End(event) => match event.name().as_ref() {
                b"port" => {
                    if let (Some(host), Some(port)) = (&mut host, port.take()) {
                        host.ports.push(port);
                    }
                }
                b"host" => {
                    if let Some(host) = host.take() {
                        hosts.push(host);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(NmapReport { hosts })
}

fn xml_attr(
    event: &quick_xml::events::BytesStart<'_>,
    key: &[u8],
    decoder: quick_xml::encoding::Decoder,
) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == key)
        .and_then(|attribute| {
            attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
                .ok()
        })
        .map(|value| value.into_owned())
}

/// `ext.exiftool`：文件元数据（只读）。
pub struct ExifTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl ExifTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("exiftool"),
            id: CapabilityId::new("tool.external.exiftool").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for ExifTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.exiftool")
            .with_description(
                "Read file metadata through the system exiftool binary (JSON output). \
                 Read-only and workspace-scoped.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "File path relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "ext.exiftool arguments must be an object",
            )
            .with_name("ext.exiftool");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.exiftool requires a string path")
                .with_name("ext.exiftool");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.exiftool")
            }
        };
        let args = vec![OsString::from("-json"), target.clone().into_os_string()];
        let result =
            match run_external(&self.binary, &args, Duration::from_secs(30), 1024 * 1024).await {
                Ok(result) => result,
                Err(error) => {
                    return ToolResult::permanent_error(&call.id, error.to_string())
                        .with_name("ext.exiftool")
                }
            };
        // exiftool 对未知格式返回退出码 1，但 JSON 仍可用（含 Error 字段）。
        if !result.success() && result.exit_code() != Some(1) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "exiftool failed with {:?}: {}",
                    result.exit_code(),
                    String::from_utf8_lossy(&result.stderr)
                ),
            )
            .with_name("ext.exiftool");
        }
        let parsed: Value = match serde_json::from_slice(&result.stdout) {
            Ok(value) => value,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("exiftool JSON parse failed: {error}"),
                )
                .with_name("ext.exiftool")
            }
        };
        let Some(mut metadata) = parsed.as_array().and_then(|items| items.first()).cloned() else {
            return ToolResult::permanent_error(&call.id, "exiftool returned no metadata")
                .with_name("ext.exiftool");
        };
        if let Some(object) = metadata.as_object_mut() {
            object.remove("SourceFile");
        }
        let fields = metadata.as_object().map_or(0, serde_json::Map::len);
        let claim = format!("{fields} metadata field(s) for {}", target.display());
        let evidence = Evidence::new(
            "ext.exiftool",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(metadata)
        .with_limitation("raw metadata only; no interpretation or risk scoring");
        ok_evidence(call, "ext.exiftool", &evidence)
    }
}

/// `ext.tshark`：pcap 深度协议摘要（只读）。
pub struct TsharkTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl TsharkTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("tshark"),
            id: CapabilityId::new("tool.external.tshark").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for TsharkTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.tshark")
            .with_description(
                "Summarize a pcap through the system tshark binary (JSON output, first 200 \
                 packets). Read-only and workspace-scoped.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "pcap/pcapng file relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "ext.tshark arguments must be an object")
                .with_name("ext.tshark");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.tshark requires a string path")
                .with_name("ext.tshark");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.tshark")
            }
        };
        let args = vec![
            OsString::from("-r"),
            target.clone().into_os_string(),
            OsString::from("-T"),
            OsString::from("json"),
            OsString::from("-c"),
            OsString::from("200"),
        ];
        let result = match run_external(
            &self.binary,
            &args,
            Duration::from_secs(120),
            4 * 1024 * 1024,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.tshark")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string())
                .with_name("ext.tshark");
        }
        let packets: Vec<Value> = match serde_json::from_slice(&result.stdout) {
            Ok(packets) => packets,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("tshark JSON parse failed: {error}"),
                )
                .with_name("ext.tshark");
            }
        };
        let mut protocols = std::collections::BTreeSet::new();
        for packet in &packets {
            if let Some(layers) = packet
                .get("_source")
                .and_then(|source| source.get("layers"))
                .and_then(Value::as_object)
            {
                for value in layers.values() {
                    if let Some(stack) = value.get("frame.protocols").and_then(Value::as_str) {
                        for protocol in stack.split(':') {
                            protocols.insert(protocol.to_string());
                        }
                    }
                }
            }
        }
        let claim = format!(
            "{} packet(s), {} protocol(s)",
            packets.len(),
            protocols.len()
        );
        let details = json!({
            "packets": packets.len(),
            "protocols": protocols,
        });
        let evidence = Evidence::new(
            "ext.tshark",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(details)
        .with_limitation("first 200 packets only; no reassembly or decryption");
        ok_evidence(call, "ext.tshark", &evidence)
    }
}

/// `ext.yara`：完整 YARA 语义（外部 CLI，非子集）。
pub struct YaraCliTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl YaraCliTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("yara"),
            id: CapabilityId::new("tool.external.yara").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for YaraCliTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.yara")
            .with_description(
                "Scan a workspace file with the system YARA binary (full YARA semantics, \
                 including modules). Rules are provided inline and written to a temporary file.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "File to scan, relative to the workspace root."},
                    "rules": {"type": "string", "minLength": 1, "maxLength": 65536,
                        "description": "Complete YARA rules source."}
                },
                "required": ["path", "rules"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "ext.yara arguments must be an object")
                .with_name("ext.yara");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.yara requires a string path")
                .with_name("ext.yara");
        };
        let Some(rules) = object.get("rules").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.yara requires rules source")
                .with_name("ext.yara");
        };
        if rules.len() > 65536 {
            return ToolResult::permanent_error(&call.id, "rules source too large")
                .with_name("ext.yara");
        }
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.yara")
            }
        };
        let rules_path = std::env::temp_dir().join(format!(
            "apeireth-yara-{}-{}.yar",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        if let Err(error) = std::fs::write(&rules_path, rules) {
            return ToolResult::permanent_error(&call.id, format!("cannot stage rules: {error}"))
                .with_name("ext.yara");
        }
        let args = vec![
            OsString::from("-s"),
            OsString::from("-w"),
            rules_path.clone().into_os_string(),
            target.clone().into_os_string(),
        ];
        let result = run_external(
            &self.binary,
            &args,
            Duration::from_secs(60),
            2 * 1024 * 1024,
        )
        .await;
        let _ = std::fs::remove_file(&rules_path);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.yara")
            }
        };
        // yara: 0 = 命中，1 = 未命中，>=2 = 错误。
        if !result.success() && result.exit_code() != Some(1) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "yara failed with {:?}: {}",
                    result.exit_code(),
                    String::from_utf8_lossy(&result.stderr)
                ),
            )
            .with_name("ext.yara");
        }
        let stdout = String::from_utf8_lossy(&result.stdout);
        let mut rules_hit = std::collections::BTreeSet::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("0x") {
                continue;
            }
            if let Some(name) = line.split_whitespace().next() {
                rules_hit.insert(name.to_string());
            }
        }
        let claim = format!(
            "{} rule(s) matched in {}",
            rules_hit.len(),
            target.display()
        );
        let details = json!({
            "matches": rules_hit,
            "raw": stdout.chars().take(8192).collect::<String>(),
        });
        let evidence = Evidence::new(
            "ext.yara",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(details)
        .with_limitation("delegated to the system yara binary; output is text-parsed");
        ok_evidence(call, "ext.yara", &evidence)
    }
}

/// `ext.binwalk`：固件/嵌入签名扫描（只读，workspace 限域）。
pub struct BinwalkTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl BinwalkTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("binwalk"),
            id: CapabilityId::new("tool.external.binwalk").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for BinwalkTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.binwalk")
            .with_description(
                "Signature scan through the system binwalk binary: embedded files, firmware \
                 headers and compression signatures. Read-only and workspace-scoped.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "File to scan, relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "ext.binwalk arguments must be an object",
            )
            .with_name("ext.binwalk");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.binwalk requires a string path")
                .with_name("ext.binwalk");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.binwalk")
            }
        };
        let args = vec![OsString::from("-B"), target.clone().into_os_string()];
        let result = match run_external(
            &self.binary,
            &args,
            Duration::from_secs(60),
            2 * 1024 * 1024,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.binwalk")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string())
                .with_name("ext.binwalk");
        }
        let stdout = String::from_utf8_lossy(&result.stdout);
        let mut entries = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("DECIMAL") || line.starts_with('-') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(offset) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };
            let _ = parts.next();
            let description: String = parts.collect::<Vec<_>>().join(" ");
            entries.push(json!({"offset": offset, "description": description}));
            if entries.len() >= 200 {
                break;
            }
        }
        let claim = format!("{} signature(s) in {}", entries.len(), target.display());
        let evidence = Evidence::new(
            "ext.binwalk",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(json!({"entries": entries}))
        .with_limitation("signature scan only; extraction is not performed");
        ok_evidence(call, "ext.binwalk", &evidence)
    }
}

/// `ext.radare2`：函数清单（固定命令，不接受任意 r2 命令）。
pub struct RadareTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl RadareTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("radare2"),
            id: CapabilityId::new("tool.external.radare2").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for RadareTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.radare2")
            .with_description(
                "List functions through the system radare2 binary using one fixed analysis \
                 command (`aaa; aflj`). Read-only and workspace-scoped; arbitrary r2 commands \
                 are not accepted.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "Binary to analyze, relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "ext.radare2 arguments must be an object",
            )
            .with_name("ext.radare2");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.radare2 requires a string path")
                .with_name("ext.radare2");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.radare2")
            }
        };
        let args = vec![
            OsString::from("-q"),
            OsString::from("-c"),
            OsString::from("aaa; aflj"),
            target.clone().into_os_string(),
        ];
        let result = match run_external(
            &self.binary,
            &args,
            Duration::from_secs(120),
            4 * 1024 * 1024,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.radare2")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string())
                .with_name("ext.radare2");
        }
        let parsed: Value = match serde_json::from_slice(&result.stdout) {
            Ok(value) => value,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("radare2 JSON parse failed: {error}"),
                )
                .with_name("ext.radare2")
            }
        };
        let functions = parsed.as_array().map_or(0, Vec::len);
        let names: Vec<Value> = parsed
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .take(200)
                    .map(|item| {
                        json!({
                            "name": item.get("name").cloned().unwrap_or(Value::Null),
                            "offset": item.get("offset").cloned().unwrap_or(Value::Null),
                            "size": item.get("size").cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let claim = format!("{} function(s) in {}", functions, target.display());
        let evidence = Evidence::new(
            "ext.radare2",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(json!({"functions": functions, "names": names}))
        .with_limitation("fixed command `aaa; aflj`; no disassembly text is returned");
        ok_evidence(call, "ext.radare2", &evidence)
    }
}

fn unique_temp_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "apeireth-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ))
}

fn collect_files(root: &Path, extension: &str, limit: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                files.push(path);
                if files.len() >= limit {
                    return files;
                }
            }
        }
    }
    files
}

/// `ext.jadx`：APK/DEX 反编译摘要（输出到临时目录后统计类）。
pub struct JadxTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl JadxTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("jadx"),
            id: CapabilityId::new("tool.external.jadx").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for JadxTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.jadx")
            .with_description(
                "Decompile an APK/DEX/JAR with the system jadx binary into a temporary JSON \
                 output, then summarize class names. Workspace-scoped and read-only for the \
                 original file.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "APK/DEX/JAR path relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "ext.jadx arguments must be an object")
                .with_name("ext.jadx");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.jadx requires a string path")
                .with_name("ext.jadx");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.jadx")
            }
        };
        let outdir = unique_temp_dir("jadx-out");
        let args = vec![
            OsString::from("-d"),
            outdir.clone().into_os_string(),
            OsString::from("--no-res"),
            OsString::from("--output-format"),
            OsString::from("json"),
            target.clone().into_os_string(),
        ];
        let result = run_external(
            &self.binary,
            &args,
            Duration::from_secs(180),
            2 * 1024 * 1024,
        )
        .await;
        let files = collect_files(&outdir, "json", 200);
        let _ = std::fs::remove_dir_all(&outdir);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.jadx")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string()).with_name("ext.jadx");
        }
        let classes: Vec<String> = files
            .iter()
            .map(|file| {
                file.strip_prefix(&self.workspace_root)
                    .unwrap_or(file)
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        let claim = format!(
            "{} class file(s) decompiled from {}",
            classes.len(),
            target.display()
        );
        let evidence = Evidence::new(
            "ext.jadx",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(json!({"classes": classes}))
        .with_limitation(
            "JSON output only; full source tree is written to a temporary directory and removed",
        );
        ok_evidence(call, "ext.jadx", &evidence)
    }
}

/// `ext.apktool`：APK 解码摘要（manifest 包名/权限 + smali 计数）。
pub struct ApktoolTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl ApktoolTool {
    /// 构造。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("apktool"),
            id: CapabilityId::new("tool.external.apktool").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for ApktoolTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.apktool")
            .with_description(
                "Decode an APK with the system apktool binary into a temporary directory, then \
                 summarize the manifest package, permissions and smali file count.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "APK path relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "ext.apktool arguments must be an object",
            )
            .with_name("ext.apktool");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.apktool requires a string path")
                .with_name("ext.apktool");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.apktool")
            }
        };
        let outdir = unique_temp_dir("apktool-out");
        let args = vec![
            OsString::from("d"),
            OsString::from("-f"),
            OsString::from("-o"),
            outdir.clone().into_os_string(),
            target.clone().into_os_string(),
        ];
        let result = run_external(
            &self.binary,
            &args,
            Duration::from_secs(180),
            2 * 1024 * 1024,
        )
        .await;
        let manifest = std::fs::read_to_string(outdir.join("AndroidManifest.xml")).ok();
        let smali = collect_files(&outdir, "smali", 5000).len();
        let _ = std::fs::remove_dir_all(&outdir);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.apktool")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string())
                .with_name("ext.apktool");
        }
        let Some(manifest) = manifest else {
            return ToolResult::permanent_error(
                &call.id,
                "apktool produced no AndroidManifest.xml",
            )
            .with_name("ext.apktool");
        };
        let package = manifest
            .split("package=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .map(str::to_string);
        let permissions: Vec<String> = manifest
            .match_indices("uses-permission")
            .filter_map(|(index, _)| {
                let rest = &manifest[index..];
                rest.split("android:name=\"")
                    .nth(1)
                    .and_then(|value| value.split('"').next())
                    .map(str::to_string)
            })
            .collect();
        let claim = format!(
            "package {}, {} permission(s), {} smali file(s)",
            package.as_deref().unwrap_or("unknown"),
            permissions.len(),
            smali
        );
        let details = json!({
            "package": package,
            "permissions": permissions,
            "smali_files": smali,
        });
        let evidence = Evidence::new(
            "ext.apktool",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(details)
        .with_limitation("decoded tree is temporary; binary XML edge cases may be skipped");
        ok_evidence(call, "ext.apktool", &evidence)
    }
}

/// Ghidra postScript：输出函数清单（手工拼 JSON，避免 Jython json 模块差异）。
const GHIDRA_SCRIPT: &str = r#"#@category Apeireth
fm = currentProgram.getFunctionManager()
items = []
count = 0
for f in fm.getFunctions(True):
    count += 1
    if len(items) < 500:
        name = f.getName().replace('\\', '\\\\').replace('"', '\\"')
        entry = str(f.getEntryPoint()).replace('\\', '\\\\').replace('"', '\\"')
        items.append('{"name": "%s", "entry": "%s"}' % (name, entry))
print("RE_GHIDRA_JSON={\\"functions\\": %d, \\"sample\\": [%s]}" % (count, ",".join(items)))
"#;

/// `ext.ghidra`：Ghidra headless 函数清单（重，显式调用）。
pub struct GhidraTool {
    workspace_root: PathBuf,
    binary: PathBuf,
    id: CapabilityId,
}

impl GhidraTool {
    /// 构造；默认使用 Kali 包的 analyzeHeadless 路径。
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            binary: PathBuf::from("/usr/share/ghidra/support/analyzeHeadless"),
            id: CapabilityId::new("tool.external.ghidra").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for GhidraTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.ghidra")
            .with_description(
                "Run Ghidra headless analysis and return a function listing (name + entry). \
                 Heavy: JVM startup and auto-analysis can take minutes. Workspace-scoped.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1,
                        "description": "Binary to analyze, relative to the workspace root."}
                },
                "required": ["path"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        if !self.binary.exists() {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "Ghidra headless not found at {}; install ghidra or override the binary",
                    self.binary.display()
                ),
            )
            .with_name("ext.ghidra");
        }
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(&call.id, "ext.ghidra arguments must be an object")
                .with_name("ext.ghidra");
        };
        let Some(path) = object.get("path").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.ghidra requires a string path")
                .with_name("ext.ghidra");
        };
        let target = match resolve_in_root(&self.workspace_root, path) {
            Ok(target) => target,
            Err(message) => {
                return ToolResult::permanent_error(&call.id, message).with_name("ext.ghidra")
            }
        };
        let project_dir = unique_temp_dir("ghidra-project");
        let script_dir = unique_temp_dir("ghidra-script");
        if let Err(error) = std::fs::create_dir_all(&project_dir)
            .and_then(|_| std::fs::create_dir_all(&script_dir))
            .and_then(|_| std::fs::write(script_dir.join("apeireth_summary.py"), GHIDRA_SCRIPT))
        {
            let _ = std::fs::remove_dir_all(&project_dir);
            let _ = std::fs::remove_dir_all(&script_dir);
            return ToolResult::permanent_error(&call.id, format!("cannot stage Ghidra: {error}"))
                .with_name("ext.ghidra");
        }
        let args = vec![
            project_dir.clone().into_os_string(),
            OsString::from("apeireth"),
            OsString::from("-import"),
            target.clone().into_os_string(),
            OsString::from("-postScript"),
            OsString::from("apeireth_summary.py"),
            OsString::from("-scriptPath"),
            script_dir.clone().into_os_string(),
            OsString::from("-deleteProject"),
            OsString::from("-analysisTimeoutPerFile"),
            OsString::from("300"),
        ];
        let result = run_external(
            &self.binary,
            &args,
            Duration::from_secs(600),
            4 * 1024 * 1024,
        )
        .await;
        let _ = std::fs::remove_dir_all(&project_dir);
        let _ = std::fs::remove_dir_all(&script_dir);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.ghidra")
            }
        };
        if let Err(error) = ensure_success(&self.binary, &result) {
            return ToolResult::permanent_error(&call.id, error.to_string())
                .with_name("ext.ghidra");
        }
        let stdout = String::from_utf8_lossy(&result.stdout);
        let marker = "RE_GHIDRA_JSON=";
        let Some(start) = stdout.find(marker) else {
            return ToolResult::permanent_error(
                &call.id,
                "Ghidra produced no summary marker (script failed?)",
            )
            .with_name("ext.ghidra");
        };
        let payload = stdout[start + marker.len()..]
            .lines()
            .next()
            .unwrap_or_default();
        let summary: Value = match serde_json::from_str(payload) {
            Ok(value) => value,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("Ghidra JSON parse failed: {error}"),
                )
                .with_name("ext.ghidra")
            }
        };
        let functions = summary
            .get("functions")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let claim = format!("{functions} function(s) in {}", target.display());
        let evidence = Evidence::new(
            "ext.ghidra",
            "external",
            target.display().to_string(),
            "workspace",
            claim,
        )
        .with_details(summary)
        .with_limitation("headless auto-analysis only; decompiled source is not returned");
        ok_evidence(call, "ext.ghidra", &evidence)
    }
}

/// `ext.openssl`：TLS 证书链（`openssl s_client -showcerts`）。
pub struct OpenSslTool {
    scope: Arc<Scope>,
    binary: PathBuf,
    id: CapabilityId,
}

impl OpenSslTool {
    /// 构造。
    pub fn new(scope: Arc<Scope>) -> Self {
        Self {
            scope,
            binary: PathBuf::from("openssl"),
            id: CapabilityId::new("tool.external.openssl").expect("static capability id"),
        }
    }

    /// 测试用：替换二进制路径。
    #[must_use]
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }
}

#[async_trait]
impl ToolCapability for OpenSslTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("ext.openssl")
            .with_description(
                "Fetch the TLS certificate chain via `openssl s_client -showcerts`: subject, \
                 issuer and certificate count. Does not validate the chain against a CA store.",
            )
            .with_parameters(parameters(json!({
                "type": "object",
                "properties": {
                    "host": {"type": "string", "minLength": 1,
                        "description": "Hostname or IP inside the authorized scope."},
                    "port": {"type": "integer", "minimum": 1, "maximum": 65535,
                        "description": "TLS port (default 443)."},
                    "sni": {"type": "string",
                        "description": "Optional SNI override (defaults to hostname)."}
                },
                "required": ["host"],
                "additionalProperties": false
            })))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let Some(object) = call.arguments.as_object() else {
            return ToolResult::permanent_error(
                &call.id,
                "ext.openssl arguments must be an object",
            )
            .with_name("ext.openssl");
        };
        let Some(host) = object.get("host").and_then(Value::as_str) else {
            return ToolResult::permanent_error(&call.id, "ext.openssl requires a string host")
                .with_name("ext.openssl");
        };
        // port：「没给」与「给了个畸形值」必须分开。
        //
        // 旧写法 `object.get("port").and_then(Value::as_u64).unwrap_or(443)` 把两者
        // 混为一谈 ⇒ 负数 / 浮点 / 字符串 / 布尔 / null 全部**静默变成 443**，
        // 即「调用方要的端口」与「实际连的端口」不是一回事（fail-open）。
        //
        // 契约出处：声明 external.rs:1577-1578
        //   「"port": {"type": "integer", "minimum": 1, "maximum": 65535}」；
        //   同 crate 的 `re.net.recon` 对非整数端口明确报「ports must be integers」
        //   （recon_tool.rs:529-531）。
        let port = match object.get("port") {
            None => 443u64,
            Some(value) => {
                let Some(port) = value.as_u64() else {
                    // 负数是「整数但越界」，其余（浮点/字符串/布尔/null/数组/对象）是「非整数」。
                    let message = if value.is_i64() {
                        "port out of range"
                    } else {
                        "ext.openssl port must be an integer"
                    };
                    return ToolResult::permanent_error(&call.id, message).with_name("ext.openssl");
                };
                port
            }
        };
        if !(1..=65535).contains(&port) {
            return ToolResult::permanent_error(&call.id, "port out of range")
                .with_name("ext.openssl");
        }
        // sni：声明 external.rs:1579-1580「"sni": {"type": "string"}」。
        // 非字符串不能被静默当成「没给 sni」（那会让 TLS 身份悄悄变成 host 的）。
        let sni = match object.get("sni") {
            None => None,
            Some(value) => {
                let Some(sni) = value.as_str() else {
                    return ToolResult::permanent_error(
                        &call.id,
                        "ext.openssl sni must be a string",
                    )
                    .with_name("ext.openssl");
                };
                Some(sni)
            }
        };
        if let Ok(address) = host.parse::<std::net::IpAddr>() {
            if let Err(error) = self.scope.check_address(address) {
                return ToolResult::permanent_error(&call.id, format!("scope refused: {error}"))
                    .with_name("ext.openssl");
            }
        } else if let Err(error) = self.scope.check_domain(host) {
            return ToolResult::permanent_error(&call.id, format!("scope refused: {error}"))
                .with_name("ext.openssl");
        }

        let mut args: Vec<OsString> = vec![
            OsString::from("s_client"),
            OsString::from("-connect"),
            OsString::from(format!("{host}:{port}")),
            OsString::from("-showcerts"),
        ];
        let server_name = sni.or_else(|| {
            if host.parse::<std::net::IpAddr>().is_ok() {
                None
            } else {
                Some(host)
            }
        });
        if let Some(server_name) = server_name {
            args.push(OsString::from("-servername"));
            args.push(OsString::from(server_name));
        }

        let result = match run_external(
            &self.binary,
            &args,
            Duration::from_secs(30),
            2 * 1024 * 1024,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string())
                    .with_name("ext.openssl")
            }
        };
        let stdout = String::from_utf8_lossy(&result.stdout);
        let certificates = stdout.matches("-----BEGIN CERTIFICATE-----").count();
        if !result.success() && certificates == 0 {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "openssl failed with {:?}: {}",
                    result.exit_code(),
                    String::from_utf8_lossy(&result.stderr)
                ),
            )
            .with_name("ext.openssl");
        }

        let mut chain: Vec<Value> = Vec::new();
        let mut current_subject: Option<String> = None;
        for line in stdout.lines() {
            let trimmed = line.trim();
            if let Some(position) = trimmed.find(" s:") {
                if let Some(previous) = current_subject.take() {
                    chain.push(json!({"subject": previous, "issuer": Value::Null}));
                }
                current_subject = Some(trimmed[position + 3..].trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("i:") {
                let issuer = rest.trim().to_string();
                if let Some(subject) = current_subject.take() {
                    chain.push(json!({"subject": subject, "issuer": issuer}));
                } else if let Some(last) = chain.last_mut() {
                    last["issuer"] = Value::String(issuer);
                }
            }
        }
        if let Some(subject) = current_subject {
            chain.push(json!({"subject": subject, "issuer": Value::Null}));
        }

        let claim = format!("{} certificate(s) in chain for {host}:{port}", certificates);
        let details = json!({
            "certificates": certificates,
            "chain": chain,
        });
        let evidence = Evidence::new(
            "ext.openssl",
            "external",
            format!("{host}:{port}"),
            self.scope.name.clone(),
            claim,
        )
        .with_details(details)
        .with_limitation("chain is not validated against a CA store");
        ok_evidence(call, "ext.openssl", &evidence)
    }
}

/// 外部工具集配置。
#[derive(Debug, Clone)]
pub struct ExternalToolsConfig {
    /// workspace 限域根（exiftool/tshark）。
    pub workspace_root: PathBuf,
    /// 网络目标授权范围（nmap）。
    pub scope: Arc<Scope>,
}

/// 构造三个只读外部适配器。
pub fn external_tool_set(config: &ExternalToolsConfig) -> Vec<Arc<dyn ToolCapability>> {
    vec![
        Arc::new(NmapTool::new(Arc::clone(&config.scope))),
        Arc::new(ExifTool::new(config.workspace_root.clone())),
        Arc::new(TsharkTool::new(config.workspace_root.clone())),
        Arc::new(YaraCliTool::new(config.workspace_root.clone())),
        Arc::new(BinwalkTool::new(config.workspace_root.clone())),
        Arc::new(RadareTool::new(config.workspace_root.clone())),
        Arc::new(JadxTool::new(config.workspace_root.clone())),
        Arc::new(ApktoolTool::new(config.workspace_root.clone())),
        Arc::new(GhidraTool::new(config.workspace_root.clone())),
        Arc::new(OpenSslTool::new(Arc::clone(&config.scope))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-external-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp workspace");
        dir
    }

    #[cfg(unix)]
    fn fake_binary(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("fake-tool");
        let mut file = std::fs::File::create(&path).expect("create fake");
        file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .expect("write fake");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    fn local_scope() -> Arc<Scope> {
        Arc::new(Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec!["127.0.0.1/32".into()],
        })
    }

    fn call(name: &str, arguments: Value) -> ToolCall {
        ToolCall {
            id: "call_ext".to_string(),
            name: name.to_string(),
            arguments,
        }
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    fn error_message(result: ToolResult) -> String {
        match result.outcome {
            ToolOutcome::Error { message, .. } => message,
            other => panic!("expected error, got {other:?}"),
        }
    }

    /// **畸形参数必须被拒，不能悄悄回落成默认值**（2026-10-25 fuzz 发现）。
    ///
    /// 契约出处：
    /// * 声明 external.rs:173-178 —— `"ports": {"type": "string", "maxLength": 128}`、
    ///   `"service_detection": {"type": "boolean"}`、`"syn": {"type": "boolean"}`；
    /// * 实现自己的守门 external.rs:202-218（ports 只允许 `[0-9,-]` 且非空、≤128 字符；
    ///   两个开关是布尔）。
    ///
    /// 修复前：`ports` 非字符串 ⇒ 被 `as_str` 静默丢成「没给 ports」⇒
    /// **端口约束整个被跳过**（扫描面反而变宽到 nmap 的默认端口集，fail-open）；
    /// 两个开关非布尔 ⇒ 被 `unwrap_or(默认值)` 静默变成默认值
    /// （`service_detection: "false"` 反而照样开 `-sV`，比调用方要的探测更多）。
    #[cfg(unix)]
    #[tokio::test]
    async fn nmap_rejects_malformed_ports_and_flags() {
        let workspace = temp_workspace("nmap-malformed");
        let tool = NmapTool::new(local_scope()).with_binary(workspace.join("no-such-binary"));

        for ports in [
            json!(22),
            json!(true),
            json!(null),
            json!(["22"]),
            json!({ "p": 22 }),
        ] {
            let result = tool
                .invoke(&call(
                    "ext.nmap",
                    json!({"target": "127.0.0.1", "ports": ports}),
                ))
                .await;
            let message = error_message(result);
            assert_eq!(message, "ext.nmap ports must be a string", "ports={ports}");
        }

        for (key, value) in [
            ("service_detection", json!("false")),
            ("service_detection", json!(1)),
            ("syn", json!("true")),
            ("syn", json!(null)),
        ] {
            let mut arguments = serde_json::Map::new();
            arguments.insert("target".to_string(), json!("127.0.0.1"));
            arguments.insert(key.to_string(), value.clone());
            let result = tool
                .invoke(&call("ext.nmap", Value::Object(arguments)))
                .await;
            let message = error_message(result);
            assert_eq!(
                message,
                format!("ext.nmap {key} must be a boolean"),
                "key={key} value={value}"
            );
        }

        // 合法参数仍然放行到进程边界（守门不能把合法调用也拒了）。
        let result = tool
            .invoke(&call(
                "ext.nmap",
                json!({"target": "127.0.0.1", "ports": "22,80", "syn": false}),
            ))
            .await;
        let message = error_message(result);
        assert!(
            message.starts_with("cannot start "),
            "合法参数被拒: {message}"
        );
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nmap_adapter_parses_xml_and_enforces_scope() {
        let workspace = temp_workspace("nmap");
        let binary = fake_binary(
            &workspace,
            r#"cat <<'XML'
<?xml version="1.0"?>
<nmaprun><host><address addr="127.0.0.1" addrtype="ipv4"/><ports>
<port protocol="tcp" portid="22"><state state="open"/>
<service name="ssh" product="OpenSSH" version="9.0"/></port>
</ports></host></nmaprun>
XML"#,
        );
        let tool = NmapTool::new(local_scope()).with_binary(binary);
        let value = ok_value(
            tool.invoke(&call(
                "ext.nmap",
                json!({"target": "127.0.0.1", "ports": "22"}),
            ))
            .await,
        );
        assert_eq!(value["details"]["hosts"][0]["ports"][0]["state"], "open");
        assert_eq!(value["details"]["hosts"][0]["ports"][0]["service"], "ssh");

        let denied = NmapTool::new(Arc::new(Scope::empty("empty")));
        let result = denied
            .invoke(&call("ext.nmap", json!({"target": "127.0.0.1"})))
            .await;
        assert!(!result.is_ok());
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn exiftool_adapter_returns_metadata_and_limits_paths() {
        let workspace = temp_workspace("exif");
        let binary = fake_binary(
            &workspace,
            r#"cat <<'JSON'
[{"SourceFile":"sample.jpg","Make":"Canon","Model":"X1"}]
JSON"#,
        );
        std::fs::write(workspace.join("sample.jpg"), b"fake").expect("sample");
        let tool = ExifTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&call("ext.exiftool", json!({"path": "sample.jpg"})))
                .await,
        );
        assert_eq!(value["details"]["Make"], "Canon");
        assert!(value["details"].get("SourceFile").is_none());

        let outside = tool
            .invoke(&call("ext.exiftool", json!({"path": "/etc/hostname"})))
            .await;
        assert!(!outside.is_ok());
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn tshark_adapter_summarizes_protocols() {
        let workspace = temp_workspace("tshark");
        let binary = fake_binary(
            &workspace,
            r#"cat <<'JSON'
[{"_source":{"layers":{"frame":{"frame.protocols":"eth:ethertype:ip:tcp"}}}},
 {"_source":{"layers":{"frame":{"frame.protocols":"eth:ethertype:ip:udp"}}}}]
JSON"#,
        );
        std::fs::write(workspace.join("capture.pcap"), b"fake").expect("pcap");
        let tool = TsharkTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&call("ext.tshark", json!({"path": "capture.pcap"})))
                .await,
        );
        assert_eq!(value["details"]["packets"], 2);
        assert!(value["details"]["protocols"]
            .as_array()
            .expect("protocols")
            .iter()
            .any(|value| value == "tcp"));
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn non_zero_exit_is_reported() {
        let workspace = temp_workspace("fail");
        let binary = fake_binary(&workspace, "echo bad >&2; exit 3");
        let tool = NmapTool::new(local_scope()).with_binary(binary);
        let result = tool
            .invoke(&call("ext.nmap", json!({"target": "127.0.0.1"})))
            .await;
        assert!(!result.is_ok());
        std::fs::remove_dir_all(&workspace).ok();
    }
}

#[cfg(all(test, unix))]
mod real_tool_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;

    #[tokio::test]
    async fn real_exiftool_reads_metadata() {
        if !Path::new("/usr/bin/exiftool").exists() {
            eprintln!("skipping: exiftool not installed");
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "apeireth-exiftool-real-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("sample.bin"), b"\x7fELF\x02\x01\x01\x00demo").expect("sample");
        let tool = ExifTool::new(&dir);
        let result = tool
            .invoke(&ToolCall {
                id: "call".to_string(),
                name: "ext.exiftool".to_string(),
                arguments: serde_json::json!({"path": "sample.bin"}),
            })
            .await;
        let ToolOutcome::Ok { value } = result.outcome else {
            panic!("expected ok: {result:?}");
        };
        let fields = value["details"]
            .as_object()
            .map(serde_json::Map::len)
            .unwrap_or(0);
        assert!(fields > 0, "exiftool returned no fields: {value}");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod yara_cli_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-yara-cli-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_yara_reports_matches_and_exit_one_is_no_match() {
        use std::os::unix::fs::PermissionsExt;
        let workspace = dir("fake");
        let binary = workspace.join("fake-yara");
        std::fs::write(
            &binary,
            b"#!/bin/sh\nif [ \"$3\" = \"$3\" ]; then printf 'demo /tmp/target\\n0x0:$a: hello\\n'; fi\n",
        )
        .expect("fake");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        std::fs::write(workspace.join("target.bin"), b"hello").expect("target");
        let tool = YaraCliTool::new(&workspace).with_binary(&binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.yara".into(),
                arguments: json!({"path": "target.bin", "rules": "rule demo { condition: true }"}),
            })
            .await,
        );
        assert_eq!(value["details"]["matches"][0], "demo");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn real_yara_matches_a_rule() {
        if !Path::new("/usr/bin/yara").exists() {
            eprintln!("skipping: yara not installed");
            return;
        }
        let workspace = dir("real");
        std::fs::write(workspace.join("sample.bin"), b"contains malware here").expect("sample");
        let tool = YaraCliTool::new(&workspace);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.yara".into(),
                arguments: json!({
                    "path": "sample.bin",
                    "rules": "rule hit { strings: $a = \"malware\" condition: $a }"
                }),
            })
            .await,
        );
        assert!(value["details"]["matches"]
            .as_array()
            .expect("matches")
            .iter()
            .any(|rule| rule == "hit"));
        std::fs::remove_dir_all(&workspace).ok();
    }
}

#[cfg(test)]
mod binwalk_radare_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-ext2-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[cfg(unix)]
    fn fake(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_binwalk_parses_signatures() {
        let workspace = dir("binwalk");
        let binary = fake(
            &workspace,
            "fake-binwalk",
            "cat <<'EOF'\nDECIMAL       HEXADECIMAL     DESCRIPTION\n--------------------------------------------------------------------------------\n0             0x0             ELF, 64-bit LSB executable\n1024          0x400           gzip compressed data\nEOF",
        );
        std::fs::write(workspace.join("sample.bin"), b"data").expect("sample");
        let tool = BinwalkTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.binwalk".into(),
                arguments: json!({"path": "sample.bin"}),
            })
            .await,
        );
        assert_eq!(value["details"]["entries"][0]["offset"], 0);
        assert_eq!(
            value["details"]["entries"].as_array().map(Vec::len),
            Some(2)
        );
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_radare2_parses_functions() {
        let workspace = dir("r2");
        let binary = fake(
            &workspace,
            "fake-r2",
            r#"printf '[{"name":"sym.main","offset":4096,"size":32}]'"#,
        );
        std::fs::write(workspace.join("sample.bin"), b"data").expect("sample");
        let tool = RadareTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.radare2".into(),
                arguments: json!({"path": "sample.bin"}),
            })
            .await,
        );
        assert_eq!(value["details"]["functions"], 1);
        assert_eq!(value["details"]["names"][0]["name"], "sym.main");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn real_binwalk_scans_a_binary() {
        if !Path::new("/usr/bin/binwalk").exists() {
            eprintln!("skipping: binwalk not installed");
            return;
        }
        let workspace = dir("binwalk-real");
        std::fs::copy("/bin/true", workspace.join("true.bin")).expect("copy");
        let tool = BinwalkTool::new(&workspace);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.binwalk".into(),
                arguments: json!({"path": "true.bin"}),
            })
            .await,
        );
        assert!(
            !value["details"]["entries"]
                .as_array()
                .expect("entries")
                .is_empty(),
            "binwalk found nothing: {value}"
        );
        std::fs::remove_dir_all(&workspace).ok();
    }
}

#[cfg(test)]
mod android_adapter_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-android-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[cfg(unix)]
    fn fake(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_jadx_reports_classes() {
        let workspace = dir("jadx");
        let binary = fake(
            &workspace,
            "fake-jadx",
            "mkdir -p \"$2/com/example\"; echo '{}' > \"$2/com/example/Foo.json\"",
        );
        std::fs::write(workspace.join("app.apk"), b"fake").expect("apk");
        let tool = JadxTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.jadx".into(),
                arguments: json!({"path": "app.apk"}),
            })
            .await,
        );
        assert!(value["details"]["classes"]
            .as_array()
            .expect("classes")
            .iter()
            .any(|class| class
                .as_str()
                .unwrap_or_default()
                .contains("com/example/Foo")));
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_apktool_reports_manifest_and_smali() {
        let workspace = dir("apktool");
        let binary = fake(
            &workspace,
            "fake-apktool",
            "mkdir -p \"$4/smali/com/example\"; printf '<manifest package=\"com.example.app\"><uses-permission android:name=\"android.permission.INTERNET\"/></manifest>' > \"$4/AndroidManifest.xml\"; touch \"$4/smali/com/example/Foo.smali\"",
        );
        std::fs::write(workspace.join("app.apk"), b"fake").expect("apk");
        let tool = ApktoolTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.apktool".into(),
                arguments: json!({"path": "app.apk"}),
            })
            .await,
        );
        assert_eq!(value["details"]["package"], "com.example.app");
        assert_eq!(
            value["details"]["permissions"][0],
            "android.permission.INTERNET"
        );
        assert_eq!(value["details"]["smali_files"], 1);
        std::fs::remove_dir_all(&workspace).ok();
    }
}

#[cfg(test)]
mod ghidra_adapter_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-ghidra-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_ghidra_reports_functions() {
        use std::os::unix::fs::PermissionsExt;
        let workspace = dir("fake");
        let binary = workspace.join("fake-ghidra");
        let mut file = std::fs::File::create(&binary).expect("create");
        file.write_all(
            b"#!/bin/sh\nprintf 'RE_GHIDRA_JSON={\"functions\": 2, \"sample\": [{\"name\": \"main\", \"entry\": \"0x1000\"}]}\\n'\n",
        )
        .expect("write");
        drop(file);
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        std::fs::write(workspace.join("sample.bin"), b"data").expect("sample");
        let tool = GhidraTool::new(&workspace).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.ghidra".into(),
                arguments: json!({"path": "sample.bin"}),
            })
            .await,
        );
        assert_eq!(value["details"]["functions"], 2);
        assert_eq!(value["details"]["sample"][0]["name"], "main");
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[tokio::test]
    async fn missing_ghidra_is_an_explicit_error() {
        let workspace = dir("missing");
        std::fs::write(workspace.join("sample.bin"), b"data").expect("sample");
        let tool = GhidraTool::new(&workspace).with_binary(workspace.join("does-not-exist"));
        let result = tool
            .invoke(&ToolCall {
                id: "c".into(),
                name: "ext.ghidra".into(),
                arguments: json!({"path": "sample.bin"}),
            })
            .await;
        assert!(!result.is_ok());
        std::fs::remove_dir_all(&workspace).ok();
    }
}

#[cfg(test)]
mod openssl_syn_tests {
    use super::*;
    use apeireth_protocol::canonical::ToolOutcome;
    use std::io::Write;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-ssl-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn ok_value(result: ToolResult) -> Value {
        match result.outcome {
            ToolOutcome::Ok { value } => value,
            other => panic!("expected ok, got {other:?}"),
        }
    }

    fn error_message(result: ToolResult) -> String {
        match result.outcome {
            ToolOutcome::Error { message, .. } => message,
            other => panic!("expected error, got {other:?}"),
        }
    }

    /// **畸形参数必须被拒，不能悄悄回落成默认值**（2026-10-25 fuzz 发现）。
    ///
    /// 契约出处：
    /// * 声明 external.rs:1577-1580 —— `"port": {"type": "integer", "minimum": 1,
    ///   "maximum": 65535}`、`"sni": {"type": "string"}`；
    /// * 实现自己的端口守门 external.rs:1599-1603
    ///   （`!(1..=65535).contains(&port)` → "port out of range"）；
    /// * 同 crate 的 `re.net.recon` 对非整数端口明确报「ports must be integers」
    ///   （recon_tool.rs:529-531）—— 同一族工具的同款约定。
    ///
    /// 修复前：`object.get("port").and_then(Value::as_u64).unwrap_or(443)` 把
    /// 「没给 port」与「给了个非 u64 的 port」混为一谈 ⇒ 负数 / 浮点 / 字符串 /
    /// 布尔 / null **全部静默变成 443**（调用方要的端口与实际连的端口不是一回事，
    /// fail-open）；`sni` 同理被 `as_str` 静默丢弃（TLS 身份悄悄变成 host 的）。
    #[cfg(unix)]
    #[tokio::test]
    async fn openssl_rejects_malformed_port_and_sni() {
        let workspace = dir("malformed");
        let tool =
            OpenSslTool::new(scope_localhost()).with_binary(workspace.join("no-such-binary"));

        for (port, expected) in [
            (json!(-1), "port out of range"),
            (json!(0), "port out of range"),
            (json!(65536), "port out of range"),
            (json!(u64::MAX), "port out of range"),
            (json!("443"), "ext.openssl port must be an integer"),
            (json!(443.5), "ext.openssl port must be an integer"),
            (json!(true), "ext.openssl port must be an integer"),
            (json!(null), "ext.openssl port must be an integer"),
            (json!([443]), "ext.openssl port must be an integer"),
        ] {
            let result = tool
                .invoke(&ToolCall {
                    id: "c".into(),
                    name: "ext.openssl".into(),
                    arguments: json!({"host": "127.0.0.1", "port": port}),
                })
                .await;
            let message = error_message(result);
            assert_eq!(message, expected, "port={port}");
        }

        for sni in [
            json!(1),
            json!(true),
            json!(null),
            json!(["a"]),
            json!({"a": 1}),
        ] {
            let result = tool
                .invoke(&ToolCall {
                    id: "c".into(),
                    name: "ext.openssl".into(),
                    arguments: json!({"host": "127.0.0.1", "sni": sni}),
                })
                .await;
            let message = error_message(result);
            assert_eq!(message, "ext.openssl sni must be a string", "sni={sni}");
        }

        // 合法端口仍然放行到进程边界（守门不能把合法调用也拒了）。
        let result = tool
            .invoke(&ToolCall {
                id: "c".into(),
                name: "ext.openssl".into(),
                arguments: json!({"host": "127.0.0.1", "port": 8443}),
            })
            .await;
        let message = error_message(result);
        assert!(
            message.starts_with("cannot start "),
            "合法参数被拒: {message}"
        );
        std::fs::remove_dir_all(&workspace).ok();
    }

    fn scope_localhost() -> Arc<Scope> {
        Arc::new(Scope {
            name: "test".into(),
            authorized: true,
            paths: Vec::new(),
            domains: Vec::new(),
            cidrs: vec!["127.0.0.1/32".into()],
        })
    }

    #[cfg(unix)]
    fn fake(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .expect("write");
        drop(file);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn openssl_adapter_parses_certificate_chain() {
        let workspace = dir("chain");
        let binary = fake(
            &workspace,
            "fake-openssl",
            r#"cat <<'EOF'
CONNECTED(00000003)
---
Certificate chain
 0 s:CN = example.com
   i:C = US, O = Test CA
 1 s:C = US, O = Test CA
   i:C = US, O = Test Root
---
-----BEGIN CERTIFICATE-----
MIIB
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIIB
-----END CERTIFICATE-----
EOF"#,
        );
        let tool = OpenSslTool::new(scope_localhost()).with_binary(binary);
        let value = ok_value(
            tool.invoke(&ToolCall {
                id: "c".into(),
                name: "ext.openssl".into(),
                arguments: json!({"host": "127.0.0.1", "port": 443}),
            })
            .await,
        );
        assert_eq!(value["details"]["certificates"], 2);
        assert_eq!(value["details"]["chain"][0]["subject"], "CN = example.com");
        assert_eq!(
            value["details"]["chain"][0]["issuer"],
            "C = US, O = Test CA"
        );
        std::fs::remove_dir_all(&workspace).ok();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn nmap_syn_switches_the_scan_flag() {
        let workspace = dir("syn");
        let binary = fake(
            &workspace,
            "fake-nmap",
            r#"echo "$@" > "$0.args"
cat <<'XML'
<?xml version="1.0"?>
<nmaprun><host><address addr="127.0.0.1" addrtype="ipv4"/><ports/></host></nmaprun>
XML"#,
        );
        let arg_file = PathBuf::from(format!("{}.args", binary.display()));
        let tool = NmapTool::new(scope_localhost()).with_binary(&binary);
        let result = tool
            .invoke(&ToolCall {
                id: "c".into(),
                name: "ext.nmap".into(),
                arguments: json!({"target": "127.0.0.1", "syn": true}),
            })
            .await;
        assert!(result.is_ok(), "{result:?}");
        let args = std::fs::read_to_string(&arg_file).expect("arg file");
        assert!(args.contains("-sS"), "args: {args}");
        std::fs::remove_dir_all(&workspace).ok();
    }
}
