//! `apeireth-tools::yaml_spec` — TP29 工具声明式配置 (Composio 借鉴, 插件 YAML 声明)
//!
//! ## 定位
//!
//! 生态批: 插件规范升级 — 工具 = YAML 声明 (名称/参数/权限/凭证) + 实现 (后续挂接).
//! 认证走 N21 credentials (同 crate 已落地 keyring/encrypted-file/SecretBuf).
//!
//! ## 真实密码不入 yml (TP33 纪律)
//!
//! 凭证字段 **必须** 是 `${VAR:?error}` 形式 (Compose 风格); 任何裸字符串 (非 env 引用)
//! 视为违规, 由 [`CredentialSpec::validate`] 拒绝. 这是 TP33 修真后唯一接受的形态.
//!
//! ## 0 装 PASS (诚实标注)
//!
//! - 真 LLM / 真凭据解析未接; trait 口已备 ([`ToolSpec`]).
//! - YAML → Tool impl 完整挂接后续任务做 (本任务只产"声明解析器 + 占位 Tool shim").
//! - 占位 Tool shim 行为: `Tool::call(args)` 立刻返 `{error: "tool yaml_spec only declares metadata, no implementation yet"}`,
//!   保持工具链不断 (后续接实现时无缝替换).
//!
//! ## 复用
//!
//! - `serde_yaml` 0.9 (沿用 `apeireth-pipeline` 版本, +deprecated 警告可控).
//! - `apeireth-tool-registry::Tool` trait — 占位 shim 实现该 trait.
//! - `ToolRegistry::register` — 复用注册路径, 0 改 N15 锁定 trait.
//! - `ParameterType` / `PermissionType` 走 `serde_yaml::from_str` 直接派生 (`#[serde(rename_all = "snake_case")]`).

use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

// ============================================================
// 错误类型
// ============================================================

/// YAML spec 加载/校验错误 (thiserror 派生, 携带具体位置 + 原因).
#[derive(Debug, Error)]
pub enum SpecError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML 解析错误 (path={path}): {msg}")]
    YamlParse { path: String, msg: String },
    #[error("必填字段缺失: {field} (path={path})")]
    MissingField { path: String, field: String },
    #[error(
        "参数类型非法: {param}={ty} (path={path}, 允许: string/integer/float/boolean/array/object)"
    )]
    InvalidParameterType {
        path: String,
        param: String,
        ty: String,
    },
    #[error("权限声明非法: {perm} (path={path}, 允许: file:read:<path> / file:write:<path> / network:<host> / env:<var>)")]
    InvalidPermission { path: String, perm: String },
    #[error(
        "凭证声明非法: {cred} (path={path}, 必须 ${{VAR:?msg}} 形式; 真实密码不入 yml — TP33 纪律)"
    )]
    InvalidCredential { path: String, cred: String },
    #[error(
        "工具名冲突: {name} (path={path}, registry 已存在同名工具, 不覆盖; 保持现有工具链不断)"
    )]
    NameConflict { path: String, name: String },
    #[error("目录扫描错误: {0}")]
    Directory(String),
}

// ============================================================
// 参数声明
// ============================================================

/// 参数类型: 与 JSON Schema 基本类型对齐 (Composio/MCP 通用).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
}

impl ParameterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

/// 单个参数声明.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

impl ParameterSpec {
    pub fn validate(&self, path: &str) -> Result<(), SpecError> {
        if self.name.is_empty() {
            return Err(SpecError::MissingField {
                path: path.into(),
                field: "parameter.name".into(),
            });
        }
        if self.description.is_empty() {
            return Err(SpecError::MissingField {
                path: path.into(),
                field: format!("parameter({}).description", self.name),
            });
        }
        // required=false 但有 default → 警告但不阻断 (允许 default + optional 兼容两种语义).
        Ok(())
    }
}

// ============================================================
// 权限声明
// ============================================================

/// 权限类型 (Composio/MCP 风格: file/network/env 四种 + 路径/主机/变量名 payload).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PermissionType {
    /// `file:read:<path>` — 读权限.
    FileRead(String),
    /// `file:write:<path>` — 写权限.
    FileWrite(String),
    /// `network:<host>` — 网络出站权限 (host:port 或 domain).
    Network(String),
    /// `env:<var>` — 环境变量读权限.
    Env(String),
}

impl PermissionType {
    /// 解析 "file:read:./input.txt" 形式字符串 → `PermissionType`.
    /// 失败时返 `SpecError::InvalidPermission` (含原文便于诊断).
    pub fn parse(s: &str, path: &str) -> Result<Self, SpecError> {
        let (scheme, rest) = s
            .split_once(':')
            .ok_or_else(|| SpecError::InvalidPermission {
                path: path.into(),
                perm: s.into(),
            })?;
        let payload = rest;
        match scheme {
            "file" => {
                let (sub, target) =
                    payload
                        .split_once(':')
                        .ok_or_else(|| SpecError::InvalidPermission {
                            path: path.into(),
                            perm: s.into(),
                        })?;
                match sub {
                    "read" => Ok(Self::FileRead(target.into())),
                    "write" => Ok(Self::FileWrite(target.into())),
                    _ => Err(SpecError::InvalidPermission {
                        path: path.into(),
                        perm: s.into(),
                    }),
                }
            }
            "network" => Ok(Self::Network(payload.into())),
            "env" => Ok(Self::Env(payload.into())),
            _ => Err(SpecError::InvalidPermission {
                path: path.into(),
                perm: s.into(),
            }),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Self::FileRead(p) => format!("file:read:{p}"),
            Self::FileWrite(p) => format!("file:write:{p}"),
            Self::Network(h) => format!("network:{h}"),
            Self::Env(v) => format!("env:{v}"),
        }
    }
}

/// 权限声明条目.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionSpec {
    pub kind: PermissionType,
    /// YAML 原文 (便于 round-trip + 错误报告).
    pub raw: String,
}

impl PermissionSpec {
    pub fn parse(s: &str, path: &str) -> Result<Self, SpecError> {
        Ok(Self {
            kind: PermissionType::parse(s, path)?,
            raw: s.into(),
        })
    }
}

// ============================================================
// 凭证声明 (TP33 纪律: 仅 ${VAR:?msg} 形式, 真实密码不入 yml)
// ============================================================

/// 凭证声明条目.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialSpec {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    /// 环境变量名 (推荐) — 真值从 env 读, yml 仅引用.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
}

impl CredentialSpec {
    /// TP33 纪律: `env` 字段必须是 `${VAR:?msg}` 形式, 不接受裸字符串密码.
    /// 接受三种合法形态:
    /// - `None` (不读 env, 由其他渠道注入)
    /// - `"${VAR:?error_msg}"` (env var 缺则中断, 推荐)
    /// - `"${VAR}"` (env var 缺则 None)
    /// 不接受: 裸字符串 "secret123" / "password" 等.
    pub fn validate(&self, path: &str) -> Result<(), SpecError> {
        if self.name.is_empty() {
            return Err(SpecError::MissingField {
                path: path.into(),
                field: "credential.name".into(),
            });
        }
        if let Some(env) = &self.env {
            let trimmed = env.trim();
            // 允许 ${VAR} 和 ${VAR:?msg}; 不允许裸字符串.
            if !trimmed.starts_with("${") || !trimmed.ends_with('}') {
                return Err(SpecError::InvalidCredential {
                    path: path.into(),
                    cred: format!("credential({}).env=`{}`", self.name, env),
                });
            }
            // 内部必须是 VAR 或 VAR:?msg — 不含其他控制字符.
            let inner = &trimmed[2..trimmed.len() - 1];
            if inner.is_empty() {
                return Err(SpecError::InvalidCredential {
                    path: path.into(),
                    cred: format!("credential({}).env=`{}` (空变量名)", self.name, env),
                });
            }
            // VAR 名: 字母或下划线开头, 后跟字母/数字/下划线 (符合 POSIX env var 命名).
            let var_part = inner.split(':').next().unwrap_or("");
            let mut chars = var_part.chars();
            let first_ok = chars
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false);
            let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !(first_ok && rest_ok) {
                return Err(SpecError::InvalidCredential {
                    path: path.into(),
                    cred: format!(
                        "credential({}).env=`{}` (变量名非法, 应以字母/下划线开头, 后跟字母/数字/下划线)",
                        self.name, env
                    ),
                });
            }
        }
        Ok(())
    }

    /// 尝试解析 `env` 字段: 若 `${VAR}` 或 `${VAR:?msg}` 命中 → 返 env var 值;
    /// 缺变量 + 无 `:?` 守卫 → 返 None; 缺变量 + 有 `:?msg` 守卫 → 返 Err(msg).
    pub fn resolve(&self) -> Result<Option<String>, String> {
        let Some(env) = &self.env else {
            return Ok(None);
        };
        let trimmed = env.trim();
        if !trimmed.starts_with("${") || !trimmed.ends_with('}') {
            return Err(format!(
                "credential({}).env 不是 ${{VAR}} 形式: {env}",
                self.name
            ));
        }
        let inner = &trimmed[2..trimmed.len() - 1];
        if let Some((var, guard)) = inner.split_once(':') {
            if let Some(msg) = guard.strip_prefix('?') {
                match std::env::var(var) {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Err(msg.to_string()),
                }
            } else {
                Ok(std::env::var(var).ok())
            }
        } else {
            Ok(std::env::var(inner).ok())
        }
    }
}

// ============================================================
// ToolSpec trait — 声明接口 (与 apeireth-tool-registry::Tool 解耦, spec 只读, Tool 是可执行)
// ============================================================

/// 工具声明: 名称 + 参数 + 权限 + 凭证.
pub trait ToolSpec: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn parameters(&self) -> Vec<ParameterSpec>;
    fn permissions(&self) -> Vec<PermissionSpec>;
    fn credentials(&self) -> Vec<CredentialSpec>;
    /// 实现路径 (YAML 中的 `implementation:` 字段), 后续接实现时按此加载.
    fn implementation(&self) -> Option<&str>;
    /// 全量校验 (参数 + 权限 + 凭证), 失败返 `SpecError`.
    fn validate(&self, path: &str) -> Result<(), SpecError> {
        if self.name().is_empty() {
            return Err(SpecError::MissingField {
                path: path.into(),
                field: "name".into(),
            });
        }
        if self.description().is_empty() {
            return Err(SpecError::MissingField {
                path: path.into(),
                field: "description".into(),
            });
        }
        for p in self.parameters() {
            p.validate(path)?;
        }
        for c in self.credentials() {
            c.validate(path)?;
        }
        Ok(())
    }
}

// ============================================================
// YAML 反序列化形态 (serde 友好; 实现细节)
// ============================================================

/// YAML 文件内部结构 (serde 友好), 转 [`YamlToolSpec`] 前先校验.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct YamlFile {
    name: String,
    description: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    parameters: Vec<ParameterSpec>,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    credentials: Vec<CredentialSpec>,
    #[serde(default)]
    implementation: Option<String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

/// YAML 加载后的 [`ToolSpec`] 实现 (持有 `YamlFile` 解析结果).
#[derive(Debug, Clone)]
pub struct YamlToolSpec {
    file: YamlFile,
    /// 预解析的权限声明 (避免每次 .permissions() 重新 parse).
    perms: Vec<PermissionSpec>,
}

impl YamlToolSpec {
    /// 从 `YamlFile` 构造; 权限字符串解析失败时立即报错.
    pub fn from_file(file: YamlFile, path: &str) -> Result<Self, SpecError> {
        let mut perms = Vec::with_capacity(file.permissions.len());
        for raw in &file.permissions {
            perms.push(PermissionSpec::parse(raw, path)?);
        }
        Ok(Self { file, perms })
    }

    /// 直接从 YAML 字符串构造 (测试用 + 内嵌 spec).
    pub fn from_str(yaml: &str, path: &str) -> Result<Self, SpecError> {
        let file: YamlFile = serde_yaml::from_str(yaml).map_err(|e| SpecError::YamlParse {
            path: path.into(),
            msg: e.to_string(),
        })?;
        Self::from_file(file, path)
    }
}

impl ToolSpec for YamlToolSpec {
    fn name(&self) -> &str {
        &self.file.name
    }
    fn description(&self) -> &str {
        &self.file.description
    }
    fn version(&self) -> &str {
        &self.file.version
    }
    fn parameters(&self) -> Vec<ParameterSpec> {
        self.file.parameters.clone()
    }
    fn permissions(&self) -> Vec<PermissionSpec> {
        self.perms.clone()
    }
    fn credentials(&self) -> Vec<CredentialSpec> {
        self.file.credentials.clone()
    }
    fn implementation(&self) -> Option<&str> {
        self.file.implementation.as_deref()
    }
}

/// 内存中的 [`ToolSpec`] (测试 + 动态构造).
#[derive(Debug, Clone)]
pub struct StaticToolSpec {
    name: String,
    description: String,
    version: String,
    parameters: Vec<ParameterSpec>,
    permissions: Vec<PermissionSpec>,
    credentials: Vec<CredentialSpec>,
    implementation: Option<String>,
}

impl StaticToolSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            version: "0.1.0".into(),
            parameters: Vec::new(),
            permissions: Vec::new(),
            credentials: Vec::new(),
            implementation: None,
        }
    }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    pub fn with_param(mut self, p: ParameterSpec) -> Self {
        self.parameters.push(p);
        self
    }
    pub fn with_permission(mut self, p: PermissionSpec) -> Self {
        self.permissions.push(p);
        self
    }
    pub fn with_credential(mut self, c: CredentialSpec) -> Self {
        self.credentials.push(c);
        self
    }
    pub fn with_implementation(mut self, impl_path: impl Into<String>) -> Self {
        self.implementation = Some(impl_path.into());
        self
    }
}

impl ToolSpec for StaticToolSpec {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn parameters(&self) -> Vec<ParameterSpec> {
        self.parameters.clone()
    }
    fn permissions(&self) -> Vec<PermissionSpec> {
        self.permissions.clone()
    }
    fn credentials(&self) -> Vec<CredentialSpec> {
        self.credentials.clone()
    }
    fn implementation(&self) -> Option<&str> {
        self.implementation.as_deref()
    }
}

// ============================================================
// YAML 解析入口
// ============================================================

/// 从文件加载 spec (返回 trait object).
pub fn load_tool_spec(path: &Path) -> Result<Arc<dyn ToolSpec>, SpecError> {
    let path_str = path.display().to_string();
    let content = fs::read_to_string(path)?;
    let spec = YamlToolSpec::from_str(&content, &path_str)?;
    spec.validate(&path_str)?;
    Ok(Arc::new(spec))
}

/// 从目录批量加载所有 `*.yaml` / `*.yml` 文件; 任一失败返整体 Err.
pub fn load_directory(dir: &Path) -> Result<Vec<Arc<dyn ToolSpec>>, SpecError> {
    if !dir.is_dir() {
        return Err(SpecError::Directory(format!("不是目录: {}", dir.display())));
    }
    let mut specs = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    // 确定性顺序 (按文件名排序).
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        specs.push(load_tool_spec(&path)?);
    }
    Ok(specs)
}

/// 注册 YAML spec 到 [`ToolRegistry`] (占位 Tool shim).
///
/// **纪律**: 失败不阻断 — 失败时返 `Err(SpecError)`, 由调用方决定是否 eprintln.
/// **冲突策略**: 若同名已注册 → 返 [`SpecError::NameConflict`], 不覆盖 (保持现有工具链不断).
pub fn register_yaml_spec(registry: &ToolRegistry, path: &Path) -> Result<String, SpecError> {
    let spec = load_tool_spec(path)?;
    let name = spec.name().to_string();
    // 冲突检测: registry.get 返回 Some → 已存在同名 → 拒绝.
    if registry.get(&name).is_some() {
        return Err(SpecError::NameConflict {
            path: path.display().to_string(),
            name: name.clone(),
        });
    }
    registry.register(name.clone(), Arc::new(SpecPlaceholderTool::new(spec)));
    Ok(name)
}

/// 占位 Tool: 收到 `call(args)` 立刻返 `{error: ...}`, 不执行实际逻辑.
/// 后续任务把 `implementation: ./impl/x.rs` 接上后, 在 `Tool::call` 里路由真实现.
pub struct SpecPlaceholderTool {
    spec: Arc<dyn ToolSpec>,
}

impl SpecPlaceholderTool {
    pub fn new(spec: Arc<dyn ToolSpec>) -> Self {
        Self { spec }
    }
}

impl fmt::Debug for SpecPlaceholderTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpecPlaceholderTool")
            .field("name", &self.spec.name())
            .field("version", &self.spec.version())
            .field("parameters_count", &self.spec.parameters().len())
            .field("permissions_count", &self.spec.permissions().len())
            .field("credentials_count", &self.spec.credentials().len())
            .finish()
    }
}

#[async_trait]
impl Tool for SpecPlaceholderTool {
    fn name(&self) -> &str {
        self.spec.name()
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, _args: Value) -> Result<Value, String> {
        Ok(json!({
            "tool": self.spec.name(),
            "description": self.spec.description(),
            "version": self.spec.version(),
            "implementation": self.spec.implementation(),
            "error": "yaml_spec only declares metadata; no executable implementation yet (TP29 placeholder)",
            "hint": "后续任务按 `implementation:` 路径加载真实现; 当前仅做声明解析 + 占位路由",
            "parameters_declared": self.spec.parameters().iter().map(|p| {
                json!({"name": p.name, "type": p.param_type.as_str(), "required": p.required})
            }).collect::<Vec<_>>(),
            "permissions_declared": self.spec.permissions().iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(),
            "credentials_declared": self.spec.credentials().iter().map(|c| {
                json!({"name": c.name, "required": c.required, "env": c.env})
            }).collect::<Vec<_>>(),
        }))
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    // -------- 参数/权限/凭证类型 --------

    #[test]
    fn parameter_type_round_trip_yaml() {
        for (input, want) in [
            ("string", ParameterType::String),
            ("integer", ParameterType::Integer),
            ("float", ParameterType::Float),
            ("boolean", ParameterType::Boolean),
            ("array", ParameterType::Array),
            ("object", ParameterType::Object),
        ] {
            let parsed: ParameterType = serde_yaml::from_str(input).unwrap();
            assert_eq!(parsed, want, "{input} round-trip");
        }
    }

    #[test]
    fn parameter_type_invalid_yaml_rejected() {
        // 非合法值 → serde 反序列化失败.
        let bad: Result<ParameterType, _> = serde_yaml::from_str("weird");
        assert!(bad.is_err());
    }

    #[test]
    fn permission_type_parse_valid() {
        let cases = [
            ("file:read:./input.txt", "file:read:./input.txt"),
            ("file:write:/tmp/out", "file:write:/tmp/out"),
            ("network:api.example.com", "network:api.example.com"),
            ("env:HOME", "env:HOME"),
        ];
        for (input, want) in cases {
            let p = PermissionType::parse(input, "<test>").unwrap();
            assert_eq!(p.as_str(), want, "round-trip {input}");
        }
    }

    #[test]
    fn permission_type_parse_invalid_rejected() {
        let bad = [
            "",            // 空
            "no-colon",    // 无冒号
            "file:nope:x", // file: 不是 read/write
            "ssh:host",    // 未知 scheme
            "file:read",   // 缺 payload
        ];
        for s in bad {
            let res = PermissionType::parse(s, "<test>");
            assert!(res.is_err(), "应拒绝: {s:?}, got {res:?}");
        }
    }

    #[test]
    fn credential_validate_accepts_env_reference() {
        // ${VAR} 形式 → 接受.
        let c = CredentialSpec {
            name: "api_key".into(),
            required: true,
            env: Some("${EXAMPLE_API_KEY}".into()),
        };
        c.validate("<test>").expect("应接受 ${VAR}");

        // ${VAR:?msg} 形式 → 接受.
        let c2 = CredentialSpec {
            name: "token".into(),
            required: true,
            env: Some("${GH_TOKEN:?missing github token}".into()),
        };
        c2.validate("<test>").expect("应接受 ${VAR:?msg}");

        // None → 接受 (由其他渠道注入).
        let c3 = CredentialSpec {
            name: "secret".into(),
            required: false,
            env: None,
        };
        c3.validate("<test>").expect("应接受 env=None");
    }

    #[test]
    fn credential_validate_rejects_plaintext_password() {
        // 裸字符串密码 → 拒绝 (TP33 纪律).
        let bad = [
            CredentialSpec {
                name: "api_key".into(),
                required: true,
                env: Some("secret123".into()), // 裸字符串
            },
            CredentialSpec {
                name: "password".into(),
                required: true,
                env: Some("hunter2".into()), // 裸字符串
            },
            CredentialSpec {
                name: "token".into(),
                required: true,
                env: Some("Bearer xyz".into()), // 含空格 + 裸值
            },
            CredentialSpec {
                name: "k".into(),
                required: true,
                env: Some(r"${}".into()), // 空变量名
            },
            CredentialSpec {
                name: "k".into(),
                required: true,
                env: Some(r"${1BAD:?msg}".into()), // 变量名首字符数字
            },
        ];
        for c in bad {
            assert!(
                c.validate("<test>").is_err(),
                "应拒绝 env=`{}`",
                c.env.as_deref().unwrap_or("None")
            );
        }
    }

    // -------- YAML 合法/非法 --------

    #[test]
    fn yaml_legal_full_spec_parses() {
        let yaml = r#"
name: example_tool
description: An example tool for testing
version: 1.0.0
parameters:
  - name: input
    type: string
    description: Input text
    required: true
  - name: count
    type: integer
    description: Repeat count
    default: 1
    required: false
permissions:
  - file:read:./input.txt
  - file:write:./output.txt
credentials:
  - name: api_key
    required: false
    env: ${EXAMPLE_API_KEY}
implementation: ./impl/example_tool.rs
"#;
        let spec = YamlToolSpec::from_str(yaml, "<inline>").expect("legal yaml");
        assert_eq!(spec.name(), "example_tool");
        assert_eq!(spec.version(), "1.0.0");
        assert_eq!(spec.parameters().len(), 2);
        assert_eq!(spec.permissions().len(), 2);
        assert_eq!(spec.credentials().len(), 1);
        assert_eq!(spec.implementation(), Some("./impl/example_tool.rs"));
        spec.validate("<inline>").expect("validate ok");
    }

    #[test]
    fn yaml_minimal_spec_parses() {
        // 最小合法: 仅 name + description.
        let yaml = "name: tiny\ndescription: small tool\n";
        let spec = YamlToolSpec::from_str(yaml, "<inline>").expect("minimal");
        assert_eq!(spec.name(), "tiny");
        assert_eq!(spec.version(), "0.1.0", "默认版本");
        assert!(spec.parameters().is_empty());
        assert!(spec.permissions().is_empty());
        assert!(spec.credentials().is_empty());
        assert!(spec.implementation().is_none());
    }

    #[test]
    fn yaml_missing_name_rejected() {
        let yaml = "description: no name\n";
        // serde 直接在 parse 阶段拦截缺 name 字段 (YmlFile 的 name 字段无 default).
        let res = YamlToolSpec::from_str(yaml, "<inline>");
        match res {
            Err(SpecError::YamlParse { ref msg, .. }) => {
                assert!(msg.contains("name"), "msg 应含 'name', got {msg:?}");
            }
            other => panic!("应返 YamlParse 含 'name', got {other:?}"),
        }
    }

    #[test]
    fn yaml_invalid_parameter_type_rejected() {
        let yaml = r#"
name: bad_param
description: bad param type
parameters:
  - name: x
    type: weirdtype
    description: ?
    required: false
"#;
        // serde 反序列化 ParameterType 时直接失败.
        let res = YamlToolSpec::from_str(yaml, "<inline>");
        assert!(res.is_err());
    }

    #[test]
    fn yaml_plaintext_password_rejected() {
        let yaml = r#"
name: insecure
description: insecure tool
credentials:
  - name: api_key
    required: true
    env: hardcoded_password_123
"#;
        // parse ok, 但 validate 应拒绝.
        let spec = YamlToolSpec::from_str(yaml, "<inline>").expect("parse");
        let err = spec.validate("<inline>").unwrap_err();
        assert!(matches!(err, SpecError::InvalidCredential { .. }));
    }

    #[test]
    fn yaml_invalid_permission_rejected() {
        let yaml = r#"
name: bad_perm
description: bad perm
permissions:
  - ssh:something
"#;
        let res = YamlToolSpec::from_str(yaml, "<inline>");
        assert!(matches!(res, Err(SpecError::InvalidPermission { .. })));
    }

    // -------- load_tool_spec / load_directory --------

    #[test]
    fn load_tool_spec_from_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("tool.yaml");
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(
            b"name: file_loaded\ndescription: from disk\nparameters:\n  - name: x\n    type: string\n    description: in\n    required: true\n",
        )
        .unwrap();

        let spec = load_tool_spec(&path).expect("load");
        assert_eq!(spec.name(), "file_loaded");
        assert_eq!(spec.parameters().len(), 1);
    }

    #[test]
    fn load_tool_spec_nonexistent_returns_io_error() {
        let res = load_tool_spec(Path::new("/nonexistent/path/tool.yaml"));
        assert!(matches!(res, Err(SpecError::Io(_))));
    }

    #[test]
    fn load_directory_collects_all_yaml() {
        let dir = TempDir::new().unwrap();
        // 写 3 个 .yaml + 1 个 .yml + 1 个 .txt (应被忽略).
        for (name, body) in [
            ("a.yaml", "name: a\ndescription: tool A\n"),
            ("b.yml", "name: b\ndescription: tool B\n"),
            ("c.yaml", "name: c\ndescription: tool C\n"),
            ("d.txt", "name: d\ndescription: skipped\n"),
        ] {
            fs::write(dir.path().join(name), body).unwrap();
        }
        let specs = load_directory(dir.path()).expect("load dir");
        // 3 yaml/yml 应被加载; 1 txt 应被忽略.
        assert_eq!(specs.len(), 3);
        // 确定性排序: a.yaml → b.yml → c.yaml.
        let names: Vec<&str> = specs.iter().map(|s| s.name()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn load_directory_not_a_dir_rejected() {
        let res = load_directory(Path::new("/nonexistent/never"));
        assert!(matches!(
            res,
            Err(SpecError::Io(_)) | Err(SpecError::Directory(_))
        ));
    }

    // -------- ToolSpec trait / StaticToolSpec --------

    #[test]
    fn static_tool_spec_round_trip() {
        let spec = StaticToolSpec::new("inline_tool")
            .with_description("inline tool for tests")
            .with_param(ParameterSpec {
                name: "q".into(),
                param_type: ParameterType::String,
                description: "query".into(),
                required: true,
                default: None,
            })
            .with_permission(PermissionSpec::parse("network:api.example.com", "<test>").unwrap())
            .with_credential(CredentialSpec {
                name: "k".into(),
                required: true,
                env: Some("${API_KEY}".into()),
            })
            .with_implementation("./impl/inline.rs");

        assert_eq!(spec.name(), "inline_tool");
        assert_eq!(spec.description(), "inline tool for tests");
        assert_eq!(spec.parameters().len(), 1);
        assert_eq!(spec.permissions().len(), 1);
        assert_eq!(spec.credentials().len(), 1);
        assert_eq!(spec.implementation(), Some("./impl/inline.rs"));
        spec.validate("<test>").expect("validate ok");
    }

    // -------- 占位 Tool 行为 + registry 衔接 --------

    #[tokio::test]
    async fn placeholder_tool_call_returns_metadata() {
        let spec: Arc<dyn ToolSpec> = Arc::new(
            StaticToolSpec::new("ph_tool")
                .with_description("placeholder")
                .with_param(ParameterSpec {
                    name: "input".into(),
                    param_type: ParameterType::String,
                    description: "x".into(),
                    required: true,
                    default: None,
                }),
        );
        let tool = SpecPlaceholderTool::new(spec);
        let r = tool.call(json!({"input": "hello"})).await.expect("call ok");
        assert_eq!(r["tool"], "ph_tool");
        assert_eq!(
            r["error"],
            "yaml_spec only declares metadata; no executable implementation yet (TP29 placeholder)"
        );
        // 暴露的参数/权限/凭证声明.
        assert_eq!(r["parameters_declared"][0]["name"], "input");
    }

    #[test]
    fn register_yaml_spec_inserts_into_registry() {
        use apeireth_tool_registry::ToolRegistry;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("reg.yaml");
        fs::write(&path, "name: reg_tool\ndescription: registered\n").unwrap();
        let registry = ToolRegistry::new();
        let name = register_yaml_spec(&registry, &path).expect("register ok");
        assert_eq!(name, "reg_tool");
        assert!(registry.get("reg_tool").is_some(), "registry 应可查到");
    }

    #[test]
    fn register_yaml_spec_name_conflict_does_not_overwrite() {
        use apeireth_tool_registry::ToolRegistry;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("conflict.yaml");
        fs::write(&path, "name: clash\ndescription: clash\n").unwrap();

        let registry = ToolRegistry::new();
        // 预注册一个同名占位.
        let placeholder = Arc::new(SpecPlaceholderTool::new(Arc::new(
            StaticToolSpec::new("clash").with_description("first"),
        )));
        registry.register("clash".into(), placeholder);

        // 二次注册应失败 (保持现有工具链不断).
        let res = register_yaml_spec(&registry, &path);
        match res {
            Err(SpecError::NameConflict { name, .. }) => assert_eq!(name, "clash"),
            other => panic!("应返 NameConflict(clash), got {other:?}"),
        }
        // 原工具仍在 (未被覆盖).
        assert!(registry.get("clash").is_some());
    }

    // -------- 失败时不破坏现有 API (fail-safety) --------

    #[test]
    fn register_yaml_spec_failure_does_not_corrupt_registry() {
        use apeireth_tool_registry::ToolRegistry;
        let dir = TempDir::new().unwrap();
        let registry = ToolRegistry::new();

        // 先注册一个 normal tool (existing API).
        let dummy = Arc::new(SpecPlaceholderTool::new(Arc::new(
            StaticToolSpec::new("normal_tool").with_description("normal"),
        )));
        registry.register("normal_tool".into(), dummy);
        let count_before = registry.len();

        // 尝试注册一个非法 YAML (缺失 description).
        let bad_path = dir.path().join("bad.yaml");
        fs::write(&bad_path, "name: bad\n").unwrap();
        let res = register_yaml_spec(&registry, &bad_path);
        assert!(res.is_err(), "非法 YAML 应失败");

        // 现有工具仍在, 数量不变 (fail-safety: 不污染 registry).
        assert_eq!(
            registry.len(),
            count_before,
            "失败后 registry 数量应不变: before={count_before}, after={}",
            registry.len()
        );
        assert!(registry.get("normal_tool").is_some(), "normal_tool 应仍在");
        assert!(registry.get("bad").is_none(), "bad 不应被注册");
    }

    #[test]
    fn invalid_yaml_load_directory_does_not_register_partial() {
        use apeireth_tool_registry::ToolRegistry;
        let dir = TempDir::new().unwrap();
        // 一个合法 + 一个非法 (缺 description → serde parse 阶段失败).
        fs::write(
            dir.path().join("ok.yaml"),
            "name: ok_tool\ndescription: legal\n",
        )
        .unwrap();
        fs::write(dir.path().join("bad.yaml"), "name: bad_tool\n").unwrap();

        let registry = ToolRegistry::new();
        // load_directory 对解析失败整体返 Err (transactional).
        let res = load_directory(dir.path());
        assert!(res.is_err(), "bad.yaml 缺 description 应使整体加载失败");
        match res {
            Err(SpecError::YamlParse { path, .. }) => {
                assert!(
                    path.contains("bad.yaml"),
                    "path 应含 bad.yaml, got {path:?}"
                );
            }
            Err(SpecError::Directory(_)) | Err(SpecError::Io(_)) => {
                // 目录或 IO 错误也可接受
            }
            other => panic!("应返 YamlParse/Directory/Io, got {other:?}"),
        }

        // 不论 ok.yaml 是否成功 load, register_yaml_spec 都不应被调用 (因为整体失败).
        // 此时 registry 应为空.
        assert_eq!(
            registry.len(),
            0,
            "失败后无任何 spec 被注册 (transactional)"
        );
        assert!(
            registry.get("ok_tool").is_none(),
            "失败时 ok_tool 也不应被注册"
        );
        assert!(registry.get("bad_tool").is_none(), "bad_tool 不应被注册");
    }
}
