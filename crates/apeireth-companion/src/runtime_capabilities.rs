//! `apeireth-companion::runtime_capabilities` — Runtime Capability Manifest.
//!
//! 核心设计原则: **Backend owns capabilities. Frontend presents capabilities.**
//!
//! Desktop 不再通过逐个撞 endpoint (404 probing) 来猜后端能力, 而是在启动时
//! 拉取一份 versioned / machine-readable 的 Capability Manifest, 据此 gate UI 按钮.
//!
//! 这是 **runtime contract (information)**, 不是 authorization — 前端不可信:
//! 即便 manifest 声明 `memory.forget = true`, 后端所有 mutation 仍必须验证权限与状态.
//! (capability 是告知, 不是授权; 见 Phase 8 安全现实检查.)
//!
//! ## Capability ID 稳定性
//! 能力 ID 形如 `sessions.create` / `memory.forget` / `permissions.revoke`, 是稳定字符串.
//! Desktop 依据 capability ID 判定按钮可用性, **不**依据 UI 中文名称或 endpoint 路径.
//!
//! ## Forward compatibility
//! 未知字段一律保留 (serde 不 deny_unknown), 旧 Desktop 读新 runtime manifest 不会崩,
//! 新 Desktop 读旧 runtime manifest 用 legacy profile 兜底.

use serde::{Deserialize, Serialize};

/// Manifest schema 版本. 仅当 manifest 结构发生 **不兼容** 变更时才递增
/// (新增可选 capability / 新增可选字段 = 兼容, 不递增).
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// 单条能力声明.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    /// 稳定能力 ID, 如 `sessions.create`.
    pub id: String,
    /// 是否支持.
    pub supported: bool,
    /// 是否可读 (查询).
    #[serde(default)]
    pub read: bool,
    /// 是否可写 (mutation).
    #[serde(default)]
    pub write: bool,
    /// 该能力自身的语义版本 (仅当能力协议变化才递增).
    #[serde(default = "default_capability_version")]
    pub version: u32,
    /// 该能力暴露的操作 (稳定字符串列表, 如 `["list","create","archive"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<String>,
}

fn default_capability_version() -> u32 {
    1
}

/// 一个能力组 (如 sessions / memory / permissions / trace).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGroup {
    /// 组名 (稳定, 如 `sessions`).
    pub name: String,
    /// 组内能力.
    pub capabilities: Vec<Capability>,
}

/// Runtime 元信息 (只暴露 public 信息, 绝不泄漏 DB path / API key / master token).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeInfo {
    /// 服务名 (如 `apeireth-companion-serve`).
    pub service: String,
    /// 服务版本 (cargo pkg version).
    pub version: String,
}

/// Capability Manifest — runtime 能力契约.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    /// Manifest schema 版本.
    pub schema_version: u32,
    /// Runtime 元信息.
    pub runtime: RuntimeInfo,
    /// 能力组列表.
    pub capabilities: Vec<CapabilityGroup>,
    /// 是否为 legacy 兼容 profile (runtime 无原生 manifest 端点时, 客户端构造的保守声明).
    #[serde(default)]
    pub legacy: bool,
}

impl CapabilityManifest {
    /// 查找某 capability ID 是否声明为 supported.
    ///
    /// 未知 ID 一律返回 false (保守: 不假装支持).
    pub fn is_supported(&self, id: &str) -> bool {
        self.find(id).map_or(false, |c| c.supported)
    }

    /// 查找某 capability ID 的完整声明 (跨所有组).
    pub fn find(&self, id: &str) -> Option<&Capability> {
        self.capabilities
            .iter()
            .flat_map(|g| g.capabilities.iter())
            .find(|c| c.id == id)
    }

    /// 列出所有 supported 的 capability ID (供诊断 / RuntimeModal 展示).
    pub fn supported_ids(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .flat_map(|g| g.capabilities.iter())
            .filter(|c| c.supported)
            .map(|c| c.id.clone())
            .collect()
    }
}

/// Manifest 构建器: 各 Phase 按真实接线状态声明能力.
///
/// 默认全部 `supported = false`; 只有当后端真的接了对应端点/操作时才打开.
/// 这保证 manifest 永远诚实 — 不假装支持未实现的能力.
#[derive(Debug, Clone, Default)]
pub struct CapabilityManifestBuilder {
    groups: Vec<CapabilityGroup>,
}

impl CapabilityManifestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加或替换一个能力组.
    pub fn group(mut self, name: &str, caps: Vec<Capability>) -> Self {
        if let Some(existing) = self.groups.iter_mut().find(|g| g.name == name) {
            existing.capabilities = caps;
        } else {
            self.groups.push(CapabilityGroup {
                name: name.to_string(),
                capabilities: caps,
            });
        }
        self
    }

    /// 构建正式 manifest (非 legacy).
    pub fn build(self, runtime: RuntimeInfo) -> CapabilityManifest {
        CapabilityManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            runtime,
            capabilities: self.groups,
            legacy: false,
        }
    }
}

/// 声明本 runtime 当前真实支持的能力 (Phase 1 起点).
///
/// 仅声明 Phase 1 时**已真实接线**的能力. 后续 Phase (session/memory/permission/trace)
/// 完成端点后, 在此追加对应 capability 的 `supported = true`.
pub fn current_manifest() -> CapabilityManifest {
    let runtime = RuntimeInfo {
        service: "apeireth-companion-serve".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // --- chat: OpenAI 兼容对话端点 (已存在) ---
    let chat = vec![cap("chat.completions", true, true, true, &["stream"])];

    // --- health: 健康检查 (已存在) ---
    let health = vec![cap("health", true, true, false, &["check"])];

    // --- models: 模型列表 (已存在) ---
    let models = vec![cap("models.list", true, true, false, &["list"])];

    // --- sessions: 后端会话生命周期 (Phase 2 已接线 mutation) ---
    let sessions = vec![
        cap("sessions.read", true, true, false, &["list", "get", "timeline"]),
        cap("sessions.create", true, false, true, &["create"]),
        cap("sessions.rename", true, false, true, &["rename"]),
        cap("sessions.archive", true, false, true, &["archive"]),
        cap("sessions.restore", true, false, true, &["restore"]),
        cap("sessions.close", true, false, true, &["close"]),
    ];

    // --- memory: 记忆 (Phase 3 已接线 update/forget/protect/unprotect) ---
    let memory = vec![
        cap("memory.read", true, true, false, &["list", "search", "streams", "graph"]),
        cap("memory.append", true, false, true, &["append"]),
        cap("memory.update", true, false, true, &["update"]),
        cap("memory.forget", true, false, true, &["forget"]),
        cap("memory.protect", true, false, true, &["protect"]),
        cap("memory.unprotect", true, false, true, &["unprotect"]),
    ];

    // --- tools: 工具注册表 + 调用 ---
    let tools = vec![
        cap("tools.list", true, true, false, &["list"]),
        cap("tools.invoke", true, false, true, &["invoke"]),
    ];

    // --- permissions: 授权 (Phase 4 已接线 revoke/grants.read/evaluate) ---
    // policy.write (持久化策略模型) 本轮不实现; policy.read = evaluate (只读评估).
    let permissions = vec![
        cap("permissions.requests.read", true, true, false, &["list"]),
        cap("permissions.grant", true, false, true, &["grant"]),
        cap("permissions.revoke", true, false, true, &["revoke"]),
        cap("permissions.grants.read", true, true, false, &["list"]),
        cap("permissions.policy.read", true, true, false, &["evaluate"]),
        cap("permissions.policy.write", false, false, false, &[]),
    ];

    // --- activity: 实时活动 (SSE + audit) ---
    let activity = vec![
        cap("activity.sse", true, true, false, &["subscribe"]),
        cap("activity.audit", true, true, false, &["list"]),
    ];

    // --- trace: 结构化 Agent 执行轨迹 (待 Phase 5) ---
    let trace = vec![
        cap("trace.read", false, false, false, &["list", "detail"]),
        cap("trace.subscribe", false, false, false, &[]),
    ];

    CapabilityManifestBuilder::new()
        .group("chat", chat)
        .group("health", health)
        .group("models", models)
        .group("sessions", sessions)
        .group("memory", memory)
        .group("tools", tools)
        .group("permissions", permissions)
        .group("activity", activity)
        .group("trace", trace)
        .build(runtime)
}

/// 构造一条 capability 声明 (supported / read / write / version=1).
fn cap(id: &str, supported: bool, read: bool, write: bool, ops: &[&str]) -> Capability {
    Capability {
        id: id.to_string(),
        supported,
        read,
        write,
        version: 1,
        operations: ops.iter().map(|s| s.to_string()).collect(),
    }
}

/// Legacy 兼容 profile: 当 runtime **没有**原生 `/v1/apeireth/capabilities` 端点时
/// (旧 runtime), 客户端用此保守声明 — 只声明经过历史契约证明存在的能力,
/// **不**推测任何 mutation.
///
/// 这是 capability flags, 不是 fake data: 它告诉前端「这些只读/对话能力可用」,
/// 其余一律视为不支持 (UI 降级为只读/隐藏).
pub fn legacy_manifest(service_version: &str) -> CapabilityManifest {
    let runtime = RuntimeInfo {
        service: "apeireth-legacy-runtime".to_string(),
        version: service_version.to_string(),
    };
    // 只声明历史契约证明存在的只读能力 + chat
    let groups = vec![
        CapabilityGroup {
            name: "chat".into(),
            capabilities: vec![cap("chat.completions", true, true, true, &["stream"])],
        },
        CapabilityGroup {
            name: "health".into(),
            capabilities: vec![cap("health", true, true, false, &["check"])],
        },
        CapabilityGroup {
            name: "models".into(),
            capabilities: vec![cap("models.list", true, true, false, &["list"])],
        },
        CapabilityGroup {
            name: "sessions".into(),
            capabilities: vec![cap("sessions.read", true, true, false, &["list", "timeline"])],
        },
        CapabilityGroup {
            name: "memory".into(),
            capabilities: vec![cap("memory.read", true, true, false, &["list", "search"])],
        },
        CapabilityGroup {
            name: "tools".into(),
            capabilities: vec![cap("tools.list", true, true, false, &["list"])],
        },
        CapabilityGroup {
            name: "permissions".into(),
            capabilities: vec![cap("permissions.requests.read", true, true, false, &["list"])],
        },
        CapabilityGroup {
            name: "activity".into(),
            capabilities: vec![
                cap("activity.sse", true, true, false, &["subscribe"]),
                cap("activity.audit", true, true, false, &["list"]),
            ],
        },
    ];
    CapabilityManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        runtime,
        capabilities: groups,
        legacy: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_stable_schema_version() {
        let m = current_manifest();
        assert_eq!(m.schema_version, MANIFEST_SCHEMA_VERSION);
        assert!(!m.legacy);
    }

    #[test]
    fn known_runtime_supports_chat_and_readonly() {
        let m = current_manifest();
        // 已接线的真实能力
        assert!(m.is_supported("chat.completions"));
        assert!(m.is_supported("health"));
        assert!(m.is_supported("sessions.read"));
        assert!(m.is_supported("memory.read"));
        assert!(m.is_supported("memory.append"));
        assert!(m.is_supported("tools.list"));
        assert!(m.is_supported("permissions.grant"));
        assert!(m.is_supported("activity.sse"));
    }

    #[test]
    fn unknown_capability_is_unsupported() {
        let m = current_manifest();
        // 未知 ID 保守返回 false (不假装支持)
        assert!(!m.is_supported("memory.purge"));
        assert!(!m.is_supported("nonexistent.thing"));
        assert!(m.find("nonexistent.thing").is_none());
    }

    #[test]
    fn unimplemented_mutations_declared_unsupported() {
        let m = current_manifest();
        // 这些 mutation 尚未接线 (Phase 5 trace / Phase 4 policy.write), 必须诚实声明 unsupported.
        // (sessions.* P2, memory.* P3, permissions.revoke P4 已接线, 不在此列.)
        assert!(!m.is_supported("permissions.policy.write"));
        assert!(!m.is_supported("trace.read"));
    }

    #[test]
    fn permissions_revoke_supported_after_phase4() {
        let m = current_manifest();
        // Phase 4 接线后: grant 可见性 + 撤销 + 评估.
        assert!(m.is_supported("permissions.revoke"));
        assert!(m.is_supported("permissions.grants.read"));
        assert!(m.is_supported("permissions.policy.read"));
    }

    #[test]
    fn sessions_mutations_supported_after_phase2() {
        let m = current_manifest();
        // Phase 2 接线后: session 生命周期 mutation 全部 supported.
        assert!(m.is_supported("sessions.create"));
        assert!(m.is_supported("sessions.rename"));
        assert!(m.is_supported("sessions.archive"));
        assert!(m.is_supported("sessions.restore"));
        assert!(m.is_supported("sessions.close"));
    }

    #[test]
    fn memory_mutations_supported_after_phase3() {
        let m = current_manifest();
        // Phase 3 接线后: memory 治理 mutation 全部 supported.
        assert!(m.is_supported("memory.update"));
        assert!(m.is_supported("memory.forget"));
        assert!(m.is_supported("memory.protect"));
        assert!(m.is_supported("memory.unprotect"));
    }

    #[test]
    fn legacy_profile_is_conservative() {
        let m = legacy_manifest("0.0.1");
        assert!(m.legacy);
        // legacy 只声明只读 + chat, 不推测 mutation
        assert!(m.is_supported("chat.completions"));
        assert!(m.is_supported("memory.read"));
        assert!(!m.is_supported("memory.append")); // legacy 不假定 append
        assert!(!m.is_supported("memory.forget"));
        assert!(!m.is_supported("sessions.create"));
        assert!(!m.is_supported("permissions.grant")); // legacy 不假定 grant
        assert!(!m.is_supported("trace.read"));
    }

    #[test]
    fn forward_compat_unknown_fields_preserved() {
        // 未知字段 (未来 runtime 新增的 capability / 属性) 不能让旧 parser 崩.
        // serde 默认不 deny_unknown, 验证一段带未知字段的 JSON 能反序列化.
        let json = r#"{
            "schema_version": 1,
            "runtime": {"service":"x","version":"1"},
            "capabilities": [{"name":"future","capabilities":[
                {"id":"future.cap","supported":true,"read":true,"write":false,"version":2,"operations":["x"],"unknown_field":"ok"}
            ]}],
            "legacy": false,
            "future_top_level": 42
        }"#;
        let m: CapabilityManifest = serde_json::from_str(json).unwrap();
        assert!(m.is_supported("future.cap"));
        assert_eq!(m.find("future.cap").unwrap().version, 2);
    }

    #[test]
    fn manifest_no_secret_leak() {
        let m = current_manifest();
        let json = serde_json::to_string(&m).unwrap();
        // manifest 绝不暴露 secret / 内部路径
        assert!(!json.contains("api_key"));
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("master_token"));
        assert!(!json.contains("masterToken"));
        assert!(!json.contains("password"));
        assert!(!json.contains(".sqlite"));
        assert!(!json.contains("APPDATA"));
    }

    #[test]
    fn capability_ids_are_stable_dotted_strings() {
        let m = current_manifest();
        for id in m.supported_ids() {
            // 稳定能力 ID 形如 group.op (如 sessions.create) 或单字根能力 (如 health).
            // 约束: 仅小写字母/数字/点/下划线, 无空格/大写/特殊字符 (保证跨语言稳定).
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'),
                "capability id 含非法字符: {id}"
            );
            assert!(!id.contains(' '), "capability id 含空格: {id}");
        }
    }

    #[test]
    fn supported_ids_nonempty() {
        let m = current_manifest();
        let ids = m.supported_ids();
        assert!(!ids.is_empty(), "manifest 应至少声明一个 supported 能力");
    }
}
