//! The MCP tool bridge: external tools become dynamic tools of this runtime.
//!
//! This module is the last-mile wiring between the MCP protocol library in
//! `apeireth_plugin::mcp` (JSON-RPC envelopes, initialize lifecycle, wire
//! schema, reconnect policy — consumed, never copied) and the canonical tool
//! execution chain (`crate::exec_pipeline`):
//!
//! 1. **Configuration surface** ([`config`]): `APEIRETH_MCP_SERVERS` (JSON
//!    array) is the primary entry, the data directory's `mcp-servers.json`
//!    (stored-document fail-closed semantics) is the secondary one. A broken
//!    configuration refuses to open; it never degrades to defaults. Startup
//!    logs are redacted: URLs may be recorded, token/secret parameters never.
//! 2. **Connections** ([`connection`]): one connection per configured server,
//!    with an initialize lifecycle state machine and disconnect-reconnect
//!    that re-discovers the tool list after every reconnect.
//! 3. **Dynamic tools** ([`tool`] + [`bridge`]): each tool a server exposes
//!    through `tools/list` is registered under the `mcp:<server>:<tool>`
//!    namespace and executes through the five-stage pipeline — a pre-execute
//!    **deny-only gate**, monotonic guards, an around deadline (MCP call
//!    timeouts close as `timeout.*`), the post-execute correction channel,
//!    and the output normalization contract that freezes MCP result
//!    structures into [`ToolOutcome`](crate::exec_pipeline::ToolOutcome).
//!    Each tool freezes its execution inputs ([`tool::McpDynamicTool`]), so
//!    the runtime's approval card shows what is about to run and the resumed
//!    call runs the approved payload, not the ambient environment.
//! 4. **Governance mapping** ([`governance`]): an external tool's permission
//!    declaration maps to explicit [`PermissionPolicy`] grants; with no
//!    explicit authorization the call is **refused** by the gate
//!    (`pipeline.pre_deny`), and `APEIRETH_MCP_READONLY_PRESET=1` releases
//!    read-only tools into the authorization surface (full pipeline).
//!
//! # Where the human approval happens
//!
//! The bridge's pre-execute gate is **deny-only**: it refuses what the
//! shared policy refuses and passes everything else through. It never asks —
//! an `ask` verdict closes as a permanent error (`pipeline.pre_ask`), not as
//! a suspended approval, and would break the runtime's approval-resume path
//! (the resumed call would ask a second time and die even though the human
//! had already approved). **The human approval right belongs to the runtime
//! governance chain (T1) alone**; the tool pipeline (T2) only refuses.
//! Layering: runtime governance chain first, tool pipeline second.
//!
//! The bridge's policy is meant to be the **shared production policy** (see
//! `McpToolBridge::new_with_policy`); the production assembly still builds the
//! bridge with `McpToolBridge::new`, so the bridge's policy is a library-level
//! gate, not a production one.
//!
//! [`mock`] is an in-process JSON-RPC server test double (no sockets, no subprocesses) for the e2e tests.
//!
//! # The session redaction domain (one session, one domain)
//!
//! The placeholder family is only useful if **every surface of one session
//! agrees on the alias** of a value. Two domains inside one session produce
//! two aliases for one value: the model can no longer correlate them, and the
//! controlled return path of one surface cannot restore what the other
//! surface produced. [`SessionRedactionDomain`] is the single owner both
//! surfaces share: the external tool result face sweeps its answers in it
//! ([`tool::McpDynamicTool`]), and the provider-facing egress gate of the
//! runtime assembly redacts and restores in it. Sharing the **instance** (not
//! merely the key) is what makes the two faces recognise each other — the
//! registry that holds the true value lives in the domain.
//!
//! **生产已接线**（`PRODUCTION WIRED`，默认关：`APEIRETH_ENABLE_MCP=1`）：
//! **发现 / 登记 / 授权 / 派发四环都有生产调用点** —— ① **发现**
//! `McpToolBridge::refresh()` ← 组合根 `crates/adapters/cli/src/external_tools.rs`
//! 的 `wire_at_startup` → 装配面的 `wire_external_tools`；
//! ② **登记** `Runtime::register_dynamic_tool()` ⇒ 外部工具进**运行时能力表**
//! ⇒ **真的进 `tool_declarations()`**（模型面工具名 `mcp:<server>:<tool>`）；
//! ③ **授权** = 生产权限策略的**族授权 + 族逐次人批**
//! （`authorize_external_tool_family`）⇒ 族内能力一律逐次人批；
//! ④ **派发** = 能力表按模型面名命中（`find_by_name`）。
//!
//! **装配面只注册两份东西**：**资源工厂形态**（`McpDefinition` —— 运行时初始化时
//! 交出活桥）与**登记账袋**（`McpModule` —— 自述面晚绑定「实际登记件数」）
//! ⇒ **不重复连进程、不重复构造桥对象**（活桥只有一只）。
//!
//! **仍属实的遗留**：桥内闸用的仍是**桥内自建策略**（生产装配走
//! `McpToolBridge::new`，**不是**共享生产策略 `new_with_policy`）⇒ 运行时裁决
//! 与桥内闸不是同一份策略（族级放行补齐等效效果，但两者不是同一把锁）。
//!
//! **仍未实测**：真第三方 MCP 服务器端到端未跑过 ⇒ `HARDWARE VALIDATED` 不成立
//! （`DEFAULT ENABLED` 也不成立：默认关）。

pub mod bridge;
pub mod catalog;
pub mod config;
pub mod connection;
pub mod definition;
pub mod family;
pub mod governance;
pub mod mock;
pub mod module;
mod process_owner;
pub use module::McpPlugin;
pub mod tool;
pub mod transport;

pub use bridge::{DiscoveryReport, McpBridgeError, McpToolBridge, McpToolRegistry};
pub use catalog::{McpCatalogError, McpToolCatalog, McpToolEntry};
pub use config::{
    doc_compat, readonly_preset_value, redact_args, redact_url, server_list_path, McpBridgeOptions,
    McpConfigError, McpServerConfig, McpServerSpec, McpTransportKind, DEFAULT_CALL_TIMEOUT_MS,
    MCP_READONLY_PRESET_ENV, MCP_SERVERS_ENV, MCP_SERVERS_FILE,
};
pub use connection::{parse_tool_list, DiscoveredTool, McpServerConnection};
pub use definition::McpDefinition;
pub use family::{is_capability_in_family, MCP_FAMILY_ROOT};
pub use governance::{McpPermissionMapping, McpRiskMappingHook, McpToolDenyGuard};
pub use mock::{InProcessChannel, InProcessChannelFactory, InProcessMcpServer, MockToolSpec};
pub use tool::{mcp_output_schema, normalized_mcp_value, tool_result_from_mcp, McpDynamicTool};
pub use transport::{
    open_channel, response_deadline_for_call, HttpChannel, McpChannel, McpChannelFactory,
    SpecChannelFactory, SseChannel, StaticChannelFactory, StdioChannel, RESPONSE_TIMEOUT,
    TRANSPORT_DEADLINE_GRACE,
};

// ---------------------------------------------------------------------------
// 会话脱敏域 (one session, one domain)
// ---------------------------------------------------------------------------

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_core::kernel::SessionId;
use sha2::{Digest, Sha256};

use crate::guardrail::{EgressSweepOutcome, ToolGuardrail};
use crate::redaction_bridge::{EgressRedaction, LedgerEntry, RedactionBridge, RestoreReport};

/// 一个会话的脱敏域 —— 占位族的**唯一持有者**.
///
/// 一个域 = 一份会话密钥 + 一本本会话登记簿（真值驻留处）+ 一本脱敏账本。
/// 域内同值恒同占位 (`__AP_<类别>_<指纹12>__`), 域间换别名; 受控回程只还原本
/// 域登记过的指纹。
///
/// # 为什么是「一个会话一个域」
///
/// 占位符的价值在于**同一会话内各面互认**: 若一个会话里有两个域, 同一个值会
/// 得到两个占位符 ⇒ 模型无法跨面关联, 且一个面的受控回程还原不了另一个面
/// 产的占位符。本类型就是两面**共用的那一个持有者**:
///
/// * **外部工具结果面** ([`tool::McpDynamicTool`]) 在本域里扫它的回信;
/// * **provider 出网闸** (runtime assembly 层的脱敏闸) 在本域里脱敏与还原。
///
/// 共用的是**实例**而不只是密钥: 真值住在域内的登记簿里, 受控回程从那儿读。
///
/// # 会话密钥怎么来
///
/// 密钥是**派生**的, 源码里没有任何字面量密钥: [`derive_session_key`] 用
/// 「进程级秘密 + 会话 id」做 SHA-256 派生。进程级秘密由内核随机 id 源
/// (uuid v4 熵) 一次性铸造, 不出本模块; 密钥不进日志、不进账本; 被写进日志的
/// 会话 id **不能**反推出密钥。
pub struct SessionRedactionDomain {
    bridge: RedactionBridge,
    session_id: SessionId,
}

impl SessionRedactionDomain {
    /// 新建一个域 (新会话 id ⇒ 派生新密钥 ⇒ 新域).
    ///
    /// 未接域注入面时, 外部工具面用它保持既有行为 (**每工具一域**): 形状、
    /// 命中面、账本口径与接线前逐字节一致, 变的只是别名取值 (本来就是随机的).
    pub fn new() -> Self {
        Self::for_session(SessionId::new())
    }

    /// 某个会话的域: 会话密钥由会话 id 派生.
    pub fn for_session(session_id: SessionId) -> Self {
        Self::with_session_key(session_id, derive_session_key(session_id))
    }

    /// 某个会话的域, 密钥由调用方显式给出 (跨进程续接同一域时用).
    ///
    /// 续接域的登记簿是**空的** —— 登记簿不跨进程; 续接的意义是别名稳定,
    /// 不是真值延续.
    pub fn with_session_key(session_id: SessionId, session_key: [u8; 32]) -> Self {
        Self {
            bridge: RedactionBridge::with_session_key(session_key),
            session_id,
        }
    }

    /// 本域属于哪个会话.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 出境脱敏: 真值面 → 占位符 (检测器命中 + 已登记值的裸/变体形态).
    ///
    /// 只有占位族, 永不吐真值.
    pub fn redact_for_egress(&mut self, text: &str) -> EgressRedaction {
        self.bridge.redact_for_egress(text)
    }

    /// 结果面整扫: 占位族脱敏 → 凭据绊线单向标签 (顺序与既有守门员一致).
    ///
    /// 这是外部工具结果面走的那条路 ([`crate::guardrail::EgressSweeper`] 的
    /// 同一套语义), 换来的只是「扫在哪个域里」可由调用方决定.
    pub fn sweep_result_face(&mut self, raw: &str) -> EgressSweepOutcome {
        EgressSweepOutcome {
            report: ToolGuardrail::sweep_output_for_egress(raw, &mut self.bridge),
        }
    }

    /// 受控回程还原 (双闸): 已登记占位符 → 真值; 未登记的照实留痕.
    ///
    /// 同一个域里, 别的面产的占位符在这里认账 —— 这是「跨面同域可互认」的
    /// 落点.
    pub fn restore_for_local(&mut self, text: &str) -> RestoreReport {
        self.bridge.restore_for_local(text)
    }

    /// 脱敏账本条目 (零真值: 类别 / 指纹 / 时间 / 还原与否).
    pub fn ledger_entries(&self) -> &[LedgerEntry] {
        self.bridge.ledger_entries()
    }

    /// 脱敏账本 JSONL (零真值).
    pub fn ledger_jsonl(&self) -> String {
        self.bridge.ledger_jsonl()
    }

    /// 登记簿里的真值数 (域的大小).
    pub fn registered_values(&self) -> usize {
        self.bridge.registered_values()
    }

    /// 两面共用的持有句柄 (同一实例 ⇒ 同一域).
    pub fn shared(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }
}

impl Default for SessionRedactionDomain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SessionRedactionDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 密钥永不入 Debug 面 (真值同理: 登记簿不在这里).
        f.debug_struct("SessionRedactionDomain")
            .field("session_id", &self.session_id)
            .field("registered", &self.bridge.registered_values())
            .field("ledger_entries", &self.bridge.ledger_entries().len())
            .finish()
    }
}

/// 派生某个会话的会话密钥 (32 字节).
///
/// 密钥 = SHA-256(域分离标签 ‖ 进程级秘密 ‖ 会话 id 字节):
///
/// * **不写死**: 源码里没有任何密钥字面量; 密钥每次由会话 id 现派生;
/// * **同进程同会话恒同**: 同一会话的两面 (外部工具面 / 出网闸) 各派生一次
///   也拿到同一把密钥 ⇒ 别名一致;
/// * **不出日志**: 进程级秘密不出本模块, 密钥不进账本 (账本只有指纹);
/// * **会话 id 泄露不致命**: 少了进程级秘密, 光有会话 id 推不出密钥.
pub fn derive_session_key(session_id: SessionId) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"apeireth/session-redaction-domain/v1");
    hasher.update(process_secret());
    hasher.update(session_id.as_uuid().as_bytes());
    hasher.finalize().into()
}

/// 进程级秘密: 一次性铸造 (uuid v4 熵源 × 2), 不出本模块.
///
/// 没有 getter、没有日志面、没有账本面 —— 它只在 [`derive_session_key`] 里
/// 当盐用.
fn process_secret() -> &'static [u8; 32] {
    static SECRET: OnceLock<[u8; 32]> = OnceLock::new();
    SECRET.get_or_init(|| {
        let first = SessionId::new().as_uuid();
        let second = SessionId::new().as_uuid();
        let mut secret = [0u8; 32];
        secret[..16].copy_from_slice(first.as_bytes());
        secret[16..].copy_from_slice(second.as_bytes());
        secret
    })
}
