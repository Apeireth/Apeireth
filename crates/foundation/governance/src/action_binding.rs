//! 动作绑定摘要（G2.5 Action-bound 审批）——「批准的到底是什么」的确定性摘要与
//! 执行边界重算比对。
//!
//! 病灶（威胁模型 G2.5，接 G2 族归一之后）：审批批准的是**动作内容**，
//! 但批准之后载荷仍可能被改（参数 / 目标 / argv 顺序变）再执行 —— 批准为
//! 改过参数的动作背书 = 静默越权。整改口径：**动作摘要绑定** ——
//!
//! 1. **审批时算摘要**：对"批准的动作"（操作 / 执行主体 / 目标 / 工具面版本 /
//!    结构化 argv）做**规范化序列化 → 摘要**，摘要随审批记录落库（跨重启持久）；
//! 2. **执行边界重算比对**：派发前对**将执行的活动作**重算摘要，与批准时的
//!    摘要比对；不一致 → **拒绝即帧**（不静默降级），并落
//!    [`AUDIT_EVENT_BINDING_MISMATCH`]（`approval.binding_mismatch`）审计行；
//! 3. **一次性授权绑定**：`AllowedOnce` 一次性审批（一审批一动作恰好一次）
//!    升级为**绑定摘要**——"这一次就是这一次"字面成真（见
//!    [`crate::approval_closure::AllowedOnceGrant`]）；族集授权（同族免重批）
//!    仍是族级口径、**不绑目标**（G2 语义零变更，绑定只护单次授权）。
//!
//! # 规范化函数规格（自研实现，字节语义以本文件测试向量为准）
//!
//! 取 RFC 8785（JCS）的规范化思路，独立实现 [`canonical_json_bytes`]：
//!
//! | 规则 | 语义 |
//! |---|---|
//! | 对象 | 键按 **UTF-8 字节序**升序；无空白；键按 JSON 字符串规则转义 |
//! | 数组 | **元素顺序原样保留**（顺序是语义的一部分，改顺序 = 改动作） |
//! | 字符串 | 最小转义：`\"` `\\` `\b` `\t` `\n` `\f` `\r`，其余 C0 控制符 `\u00xx`（小写 hex）；非 ASCII 原样 UTF-8 |
//! | 数字 | 按解析值的确定性十进制文本（整数与浮点表示视为不同值，`1` ≠ `1.0`，确定性优先） |
//! | 字面量 | `true` / `false` / `null` |
//! | 空白 | 任何位置零空白 |
//!
//! 摘要 = 规范化字节的 SHA-256（小写 hex）；短摘要 = hex 前 12 位（审批卡
//! "动作摘要"行用，短摘要而非全量载荷）。
//!
//! # 与族集授权的关系（不推翻 G2）
//!
//! 族记忆（`approval_remember`）继续管"同族免重批"，族内不绑目标仍是族级口径；
//! 绑定摘要只护**单次授权**（`AllowedOnce` / 单次审批记录），两者不互相越界。
//!
//! # 委派信任收窄（min(parent, requested)）
//!
//! 审批传递链上权限只减不增：[`DelegationChain::delegate`] 里子代理请求大于
//! 父授权即**拒**（不是静默钳制），有效授权恒为 `min(parent, requested)`。

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use apeireth_core::kernel::Timestamp;

use crate::audit::{AuditHashChain, AuditRecord};

/// 审批动作绑定摘要不匹配的审计事件 kind（拒绝即帧的审计行）。
pub const AUDIT_EVENT_BINDING_MISMATCH: &str = "approval.binding_mismatch";

/// 短摘要（审批卡"动作摘要"行）的 hex 位数。
pub const SHORT_DIGEST_CHARS: usize = 12;

/// **规范化序列化（自研）**：任意 JSON 值 → 确定性 UTF-8 字节。
///
/// 规则见模块文档「规范化函数规格」：对象键按 UTF-8 字节序排序、数组保序、
/// 字符串最小转义、数字取解析值的确定性十进制文本、零空白。
/// 同一语义值恒得同一字节串（键顺序无关），不同语义值（含数组换序）必然不同。
pub fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Vec::with_capacity(64);
            out.push(b'{');
            for (i, (key, value)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                push_json_string(&mut out, key);
                out.push(b':');
                out.extend_from_slice(&canonical_json_bytes(value));
            }
            out.push(b'}');
            out
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(64);
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&canonical_json_bytes(item));
            }
            out.push(b']');
            out
        }
        Value::String(s) => {
            let mut out = Vec::with_capacity(s.len() + 2);
            push_json_string(&mut out, s);
            out
        }
        Value::Null => b"null".to_vec(),
        Value::Bool(b) => if *b { "true" } else { "false" }.as_bytes().to_vec(),
        Value::Number(n) => n.to_string().into_bytes(),
    }
}

/// JSON 字符串最小转义（含首尾引号）：`\"` `\\` `\b` `\t` `\n` `\f` `\r`，
/// 其余 C0 控制符（U+0000–U+001F）`\u00xx`（小写 hex）；非 ASCII 原样 UTF-8。
fn push_json_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => out.extend_from_slice(c.to_string().as_bytes()),
        }
    }
    out.push(b'"');
}

/// 一条被批准（或即将执行）的动作的绑定内容（G2.5 ActionBinding）。
///
/// 摘要绑定到**字节级动作**而非意图描述：五个字段任何一个变化（含结构化
/// argv 的顺序）都会得到不同摘要，批准随之失效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBinding {
    /// 批准的动作种类（能力 / 工具名）。
    pub operation: String,
    /// 执行主体（会话 / 代理标识）。
    pub agent: String,
    /// 动作目标（被操作对象，如命令文本 / 路径；派生规则见 [`ActionBinding::derive_target`]）。
    pub target: String,
    /// 工具面版本（工具 schema 版本；工具面变更使批准失效）。
    pub tool_schema_version: String,
    /// 结构化 argv（有序；数组顺序是语义的一部分）。
    pub parameters: Value,
}

impl ActionBinding {
    /// 构造一条动作绑定（字段全显式）。
    pub fn new(
        operation: impl Into<String>,
        agent: impl Into<String>,
        target: impl Into<String>,
        tool_schema_version: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self {
            operation: operation.into(),
            agent: agent.into(),
            target: target.into(),
            tool_schema_version: tool_schema_version.into(),
            parameters,
        }
    }

    /// 目标派生规则（确定性）：参数里的 `command` 字符串 → 命令文本；
    /// 否则 `path` 字符串 → 路径；都没有 → 空串（非命令/非路径面）。
    pub fn derive_target(parameters: &Value) -> String {
        if let Some(command) = parameters.get("command").and_then(Value::as_str) {
            return command.to_string();
        }
        if let Some(path) = parameters.get("path").and_then(Value::as_str) {
            return path.to_string();
        }
        String::new()
    }

    /// 构造一条目标按 [`ActionBinding::derive_target`] 派生的动作绑定。
    pub fn with_derived_target(
        operation: impl Into<String>,
        agent: impl Into<String>,
        tool_schema_version: impl Into<String>,
        parameters: Value,
    ) -> Self {
        let target = Self::derive_target(&parameters);
        Self::new(operation, agent, target, tool_schema_version, parameters)
    }

    /// 绑定的规范化 JSON 值（键序即规范化序）。
    pub fn canonical_value(&self) -> Value {
        serde_json::json!({
            "agent": self.agent,
            "operation": self.operation,
            "parameters": self.parameters,
            "target": self.target,
            "tool_schema_version": self.tool_schema_version,
        })
    }

    /// 绑定的规范化字节（[`canonical_json_bytes`]）。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        canonical_json_bytes(&self.canonical_value())
    }

    /// 绑定摘要：规范化字节的 SHA-256（小写 hex，64 位）。
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 短摘要（hex 前 [`SHORT_DIGEST_CHARS`] 位）：审批卡"动作摘要"行用——
    /// 短摘要而非全量载荷。
    pub fn short_digest(&self) -> String {
        self.digest()[..SHORT_DIGEST_CHARS].to_string()
    }
}

/// 绑定漂移面：批准动作与活动作之间**哪一个被改了**（审计行归因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingDrift {
    /// 动作种类变了。
    Operation,
    /// 执行主体变了。
    Agent,
    /// 动作目标变了。
    Target,
    /// 工具面版本变了。
    ToolSchemaVersion,
    /// 结构化 argv 的值变了。
    Parameters,
    /// 结构化 argv 的**顺序**变了（值集合相同、顺序不同）。
    ArgumentOrder,
}

impl BindingDrift {
    /// 全部漂移面（词表冻结面；新增漂移面必须显式入表并补测试）。
    pub const ALL: [Self; 6] = [
        Self::Operation,
        Self::Agent,
        Self::Target,
        Self::ToolSchemaVersion,
        Self::Parameters,
        Self::ArgumentOrder,
    ];

    /// 稳定标签（审计行机器可读）。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Operation => "operation",
            Self::Agent => "agent",
            Self::Target => "target",
            Self::ToolSchemaVersion => "tool_schema_version",
            Self::Parameters => "parameters",
            Self::ArgumentOrder => "argument_order",
        }
    }
}

impl fmt::Display for BindingDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 摘要比对不一致（拒绝即帧的因）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingMismatch {
    /// 哪一个被改了。
    pub drift: BindingDrift,
    /// 批准时冻结的摘要。
    pub expected_digest: String,
    /// 执行边界重算的摘要。
    pub actual_digest: String,
}

/// 漂移归因（确定性）：按字段序找第一个不同面；参数面值不同但数组多重集相同
/// → [`BindingDrift::ArgumentOrder`]，否则 [`BindingDrift::Parameters`]。
pub fn classify_binding_drift(approved: &ActionBinding, live: &ActionBinding) -> BindingDrift {
    if approved.operation != live.operation {
        BindingDrift::Operation
    } else if approved.agent != live.agent {
        BindingDrift::Agent
    } else if approved.target != live.target {
        BindingDrift::Target
    } else if approved.tool_schema_version != live.tool_schema_version {
        BindingDrift::ToolSchemaVersion
    } else if order_normalized(&approved.parameters) == order_normalized(&live.parameters) {
        // 值都相同但摘要不同是不可能的（摘要由字段唯一决定）；走到这里只可能是
        // 顺序面差异已被归一化抹平 —— 归因到顺序。
        BindingDrift::ArgumentOrder
    } else {
        BindingDrift::Parameters
    }
}

/// 顺序归一（仅用于漂移归因）：数组按元素规范化字节排序后重建。
/// 不改写值、不参与摘要计算（摘要绑定顺序，见 [`canonical_json_bytes`]）。
fn order_normalized(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            let mut normalized: Vec<Value> = items.iter().map(order_normalized).collect();
            normalized.sort_by_key(|item| canonical_json_bytes(item));
            Value::Array(normalized)
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), order_normalized(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// 执行边界重算比对：批准动作与活动作的摘要一致 → `Ok`；不一致 → 拒绝即帧
/// 的因 [`BindingMismatch`]（带漂移归因）。
pub fn verify_action_binding(
    approved: &ActionBinding,
    live: &ActionBinding,
) -> Result<(), BindingMismatch> {
    let expected = approved.digest();
    let actual = live.digest();
    if expected == actual {
        return Ok(());
    }
    Err(BindingMismatch {
        drift: classify_binding_drift(approved, live),
        expected_digest: expected,
        actual_digest: actual,
    })
}

/// `approval.binding_mismatch` 审计行（拒绝即帧的账）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingMismatchAudit {
    /// 被批动作的身份（如 approval id / 能力身份）。
    pub subject: String,
    /// 哪一个被改了。
    pub drift: BindingDrift,
    /// 批准时冻结的摘要。
    pub expected_digest: String,
    /// 执行边界重算的摘要。
    pub actual_digest: String,
}

impl BindingMismatchAudit {
    /// 从比对结果构造审计行。
    pub fn new(subject: impl Into<String>, mismatch: &BindingMismatch) -> Self {
        Self {
            subject: subject.into(),
            drift: mismatch.drift,
            expected_digest: mismatch.expected_digest.clone(),
            actual_digest: mismatch.actual_digest.clone(),
        }
    }

    /// 审计事件 kind（[`AUDIT_EVENT_BINDING_MISMATCH`]）。
    pub const fn event_kind(&self) -> &'static str {
        AUDIT_EVENT_BINDING_MISMATCH
    }

    /// 审计行明细（单行、含全部摘要与漂移面）。
    pub fn detail(&self) -> String {
        format!(
            "subject={} drift={} expected={} actual={}",
            self.subject, self.drift, self.expected_digest, self.actual_digest
        )
    }

    /// 落一条审计记录（哈希串链，防篡改审计面）。
    pub fn record<'a>(&self, chain: &'a mut AuditHashChain, at: Timestamp) -> &'a AuditRecord {
        chain.append(self.event_kind(), self.detail(), at)
    }
}

/// 委派链上的授权档位（AGT min(parent, requested) 假说的权限格）。
///
/// 序即权限大小（`None < Limited < Standard < Full`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationAuthority {
    /// 无授权。
    None,
    /// 受限授权（只读 / 最小面）。
    Limited,
    /// 标准授权。
    Standard,
    /// 完整授权（仅链首）。
    Full,
}

impl DelegationAuthority {
    /// 全部档位（升序）。
    pub const ALL: [Self; 4] = [Self::None, Self::Limited, Self::Standard, Self::Full];

    /// 稳定标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Limited => "limited",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }

    /// 硬上限 `min(parent, requested)`：有效授权永远不超过父授权。
    pub const fn narrow(parent: Self, requested: Self) -> Self {
        if (requested as u8) <= (parent as u8) {
            requested
        } else {
            parent
        }
    }
}

impl fmt::Display for DelegationAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 委派被拒：子代理请求大于父授权（请求越权即拒，不静默钳制）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationRefusal {
    /// 父授权档位。
    pub parent: DelegationAuthority,
    /// 子代理请求档位。
    pub requested: DelegationAuthority,
}

impl fmt::Display for DelegationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "delegation refused: requested {} exceeds parent {} (authority may only narrow)",
            self.requested, self.parent
        )
    }
}

impl std::error::Error for DelegationRefusal {}

/// 审批传递链：权限只随深度**单调收窄**（AGT 多级收窄假说）。
///
/// [`DelegationChain::delegate`] 中子代理请求大于当前授权即拒；
/// 每级有效授权 = `min(当前授权, 请求)`，因此链条上任何一级都不可能
/// 比祖先更宽。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationChain {
    root_authority: DelegationAuthority,
    authority: DelegationAuthority,
    hops: usize,
}

impl DelegationChain {
    /// 链首（根代理的授权档位）。
    pub fn root(authority: DelegationAuthority) -> Self {
        Self {
            root_authority: authority,
            authority,
            hops: 0,
        }
    }

    /// 向下委派一级：请求大于当前授权 → 拒；否则有效授权 = `min(当前, 请求)`。
    pub fn delegate(
        &mut self,
        requested: DelegationAuthority,
    ) -> Result<DelegationAuthority, DelegationRefusal> {
        if (requested as u8) > (self.authority as u8) {
            return Err(DelegationRefusal {
                parent: self.authority,
                requested,
            });
        }
        self.authority = DelegationAuthority::narrow(self.authority, requested);
        self.hops += 1;
        Ok(self.authority)
    }

    /// 当前（最深一级）有效授权。
    pub fn authority(&self) -> DelegationAuthority {
        self.authority
    }

    /// 链首授权（上界）。
    pub fn root_authority(&self) -> DelegationAuthority {
        self.root_authority
    }

    /// 已委派深度（链首为 0）。
    pub fn depth(&self) -> usize {
        self.hops
    }

    /// 单调性断言面：链条上任何一级有效授权不超过链首（只减不增）。
    pub fn is_monotonic(&self) -> bool {
        (self.authority as u8) <= (self.root_authority as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn binding() -> ActionBinding {
        ActionBinding::new(
            "shell",
            "sess-1",
            "echo hi",
            "tool-schema-v1",
            json!({ "argv": ["bash", "-c", "echo hi"], "cwd": "/tmp" }),
        )
    }

    /// ① 规范化测试向量（钉死字节语义）：对象键排序 / 数组保序 / 字符串
    /// 最小转义（含 C0 控制符与非 ASCII 原样）/ 数字确定性文本 / 零空白。
    #[test]
    fn canonical_serialization_vectors_pin_key_order_escapes_and_numbers() {
        // 对象键按 UTF-8 字节序排序。
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&json!({ "b": 1, "a": 2 }))).unwrap(),
            "{\"a\":2,\"b\":1}"
        );
        // 数组保序（顺序是语义的一部分）。
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&json!([3, 1, 2]))).unwrap(),
            "[3,1,2]"
        );
        // 字符串最小转义：引号 / 反斜杠 / 常用短转义 / 其余 C0 控制符 \u00xx
        // （小写 hex）/ DEL 与非 ASCII 原样。
        let escaped = json!({ "s": "a\"b\\c\nd\u{7}e\u{7f}f\u{503c}" });
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&escaped)).unwrap(),
            format!("{{\"s\":\"a\\\"b\\\\c\\nd\\u0007e{}f值\"}}", "\u{7f}")
        );
        // 数字：整数与浮点表示视为不同值（确定性优先，不做数值等价归一）。
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&json!({ "n": 1 }))).unwrap(),
            "{\"n\":1}"
        );
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&json!({ "n": 1.0 }))).unwrap(),
            "{\"n\":1.0}"
        );
        // 嵌套：每层各自排序，数组内部不排。
        assert_eq!(
            String::from_utf8(canonical_json_bytes(
                &json!({ "x": { "z": [1, {"b": true, "a": null}], "y": false } })
            ))
            .unwrap(),
            "{\"x\":{\"y\":false,\"z\":[1,{\"a\":null,\"b\":true}]}}"
        );
        // 零空白 + 字面量。
        assert_eq!(
            String::from_utf8(canonical_json_bytes(&json!([true, false, null]))).unwrap(),
            "[true,false,null]"
        );
    }

    /// ② 摘要确定性：同一动作恒同摘要（键顺序无关、重复计算无关），
    /// 跨"解析-重建"也稳定（跨重启持久的前提）。
    #[test]
    fn the_same_action_always_yields_the_same_digest() {
        let a = ActionBinding::new(
            "shell",
            "sess-1",
            "echo hi",
            "v1",
            json!({ "argv": ["bash", "-c", "echo hi"], "cwd": "/tmp" }),
        );
        // 语义相等、键顺序不同的载荷。
        let b = ActionBinding::new(
            "shell",
            "sess-1",
            "echo hi",
            "v1",
            json!({ "cwd": "/tmp", "argv": ["bash", "-c", "echo hi"] }),
        );
        assert_eq!(a.digest(), b.digest(), "键顺序不得影响摘要");
        assert_eq!(a.digest(), a.digest(), "重复计算必须恒同");
        // 序列化 round-trip 后摘要不变（摘要随记录落库、跨重启重算比对的前提）。
        let restored: ActionBinding =
            serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(restored.digest(), a.digest());
        // 短摘要 = hex 前 12 位。
        assert_eq!(a.short_digest().len(), SHORT_DIGEST_CHARS);
        assert!(a.digest().starts_with(&a.short_digest()));
    }

    /// ③ 篡改检测·改参数：参数值变 → 摘要不符，拒绝即帧（漂移归因 parameters）。
    #[test]
    fn parameter_changes_change_the_digest_and_are_refused() {
        let approved = binding();
        let mut tampered = binding();
        tampered.parameters = json!({ "argv": ["bash", "-c", "echo hi"], "cwd": "/etc" });
        assert_ne!(approved.digest(), tampered.digest());
        let mismatch = verify_action_binding(&approved, &tampered).unwrap_err();
        assert_eq!(mismatch.drift, BindingDrift::Parameters);
        assert_eq!(mismatch.expected_digest, approved.digest());
        assert_eq!(mismatch.actual_digest, tampered.digest());
    }

    /// ④ 篡改检测·改目标：目标变 → 摘要不符（漂移归因 target）。
    #[test]
    fn target_changes_change_the_digest_and_are_refused() {
        let approved = binding();
        let mut tampered = binding();
        tampered.target = "rm -rf /".to_string();
        let mismatch = verify_action_binding(&approved, &tampered).unwrap_err();
        assert_eq!(mismatch.drift, BindingDrift::Target);
    }

    /// ⑤ 篡改检测·改顺序：argv 顺序变（值集合相同）→ 摘要不符，
    /// 漂移归因 argument_order。
    #[test]
    fn argument_order_changes_change_the_digest_and_are_refused() {
        let approved = ActionBinding::new(
            "shell",
            "sess-1",
            "",
            "v1",
            json!({ "argv": ["rm", "-rf", "/tmp/x"] }),
        );
        let reordered = ActionBinding::new(
            "shell",
            "sess-1",
            "",
            "v1",
            json!({ "argv": ["/tmp/x", "-rf", "rm"] }),
        );
        assert_ne!(
            approved.digest(),
            reordered.digest(),
            "argv 顺序是动作语义的一部分"
        );
        let mismatch = verify_action_binding(&approved, &reordered).unwrap_err();
        assert_eq!(mismatch.drift, BindingDrift::ArgumentOrder);
    }

    /// ⑥ 比对面：完全一致通过；工具面版本 / 动作种类 / 执行主体漂移各自归因，
    /// 漂移词表冻结。
    #[test]
    fn verify_passes_identical_bindings_and_classifies_every_drift() {
        let approved = binding();
        assert!(verify_action_binding(&approved, &binding()).is_ok());

        let mut schema = binding();
        schema.tool_schema_version = "tool-schema-v2".to_string();
        assert_eq!(
            verify_action_binding(&approved, &schema).unwrap_err().drift,
            BindingDrift::ToolSchemaVersion
        );

        let mut operation = binding();
        operation.operation = "git_write".to_string();
        assert_eq!(
            verify_action_binding(&approved, &operation)
                .unwrap_err()
                .drift,
            BindingDrift::Operation
        );

        let mut agent = binding();
        agent.agent = "sess-2".to_string();
        assert_eq!(
            verify_action_binding(&approved, &agent).unwrap_err().drift,
            BindingDrift::Agent
        );

        assert_eq!(
            BindingDrift::ALL.len(),
            6,
            "漂移词表冻结: 六面齐全（操作/主体/目标/工具面/参数/顺序）"
        );
    }

    /// ⑦ 审计行：`approval.binding_mismatch` 词表 + 明细含漂移面与双摘要，
    /// 落进哈希串链审计。
    #[test]
    fn binding_mismatch_audit_rows_use_the_approval_binding_mismatch_vocabulary() {
        let approved = binding();
        let mut tampered = binding();
        tampered.parameters = json!({ "argv": ["bash", "-c", "rm -rf /"], "cwd": "/tmp" });
        let mismatch = verify_action_binding(&approved, &tampered).unwrap_err();
        let audit = BindingMismatchAudit::new("apr_1", &mismatch);

        assert_eq!(audit.event_kind(), "approval.binding_mismatch");
        assert_eq!(audit.event_kind(), AUDIT_EVENT_BINDING_MISMATCH);
        let detail = audit.detail();
        assert!(detail.contains("subject=apr_1"), "{detail}");
        assert!(detail.contains("drift=parameters"), "{detail}");
        assert!(
            detail.contains(&format!("expected={}", approved.digest())),
            "{detail}"
        );
        assert!(
            detail.contains(&format!("actual={}", tampered.digest())),
            "{detail}"
        );

        let mut chain = AuditHashChain::new();
        let record = audit.record(
            &mut chain,
            Timestamp::from_epoch_millis(1_700_000_000_000).unwrap(),
        );
        assert_eq!(record.event_kind, AUDIT_EVENT_BINDING_MISMATCH);
        assert!(
            record.subject.contains("drift=parameters"),
            "{}",
            record.subject
        );
        assert_eq!(chain.verify(), Ok(()), "审计链必须可自证");
    }

    /// ⑧ 委派收窄：子代理请求 > 父授权 = 拒（不静默钳制）；
    /// 请求 ≤ 父授权时有效授权 = min(parent, requested)。
    #[test]
    fn delegation_refuses_a_child_requesting_more_than_its_parent() {
        let mut chain = DelegationChain::root(DelegationAuthority::Standard);
        let refusal = chain
            .delegate(DelegationAuthority::Full)
            .expect_err("子请求大于父授权必须拒");
        assert_eq!(refusal.parent, DelegationAuthority::Standard);
        assert_eq!(refusal.requested, DelegationAuthority::Full);
        assert_eq!(
            chain.authority(),
            DelegationAuthority::Standard,
            "被拒不得改权"
        );

        // 请求收窄 = 有效授权取 min(parent, requested) = requested。
        assert_eq!(
            chain.delegate(DelegationAuthority::Limited).unwrap(),
            DelegationAuthority::Limited
        );
        assert_eq!(
            DelegationAuthority::narrow(DelegationAuthority::Full, DelegationAuthority::Limited),
            DelegationAuthority::Limited
        );
        assert_eq!(
            DelegationAuthority::narrow(DelegationAuthority::Limited, DelegationAuthority::Full),
            DelegationAuthority::Limited,
            "min(parent, requested) 硬上限"
        );
    }

    /// ⑨ 委派收窄：多级链单调不增（只减不增），每级请求越权即拒。
    #[test]
    fn delegation_authority_narrows_monotonically_down_the_chain() {
        let mut chain = DelegationChain::root(DelegationAuthority::Full);
        chain.delegate(DelegationAuthority::Standard).unwrap();
        assert!(chain.is_monotonic());
        chain.delegate(DelegationAuthority::Limited).unwrap();
        assert!(chain.is_monotonic());
        // 深度处回要高档位 = 拒。
        assert!(chain.delegate(DelegationAuthority::Standard).is_err());
        assert!(chain.delegate(DelegationAuthority::Full).is_err());
        assert_eq!(chain.authority(), DelegationAuthority::Limited);
        assert_eq!(chain.depth(), 2);
        assert_eq!(chain.root_authority(), DelegationAuthority::Full);
        assert!(chain.is_monotonic(), "链条任何一级不得宽过链首");
    }
}
