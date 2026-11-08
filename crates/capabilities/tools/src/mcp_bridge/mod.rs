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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use apeireth_core::kernel::SessionId;
use apeireth_governance::input_security::{
    session_fingerprint, PiiDetector, PiiKind, RedactionPlaceholder,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::guardrail::{EgressSweepOutcome, EgressSweepResult, ToolGuardrail};
use crate::redaction_bridge::{
    EgressRedaction, LedgerEntry, LedgerEvent, RedactionBridge, RedactionHit, RedactionSurface,
    RestoreReport,
};

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
    /// 会话密钥（本域指纹派生的盐；**不进 Debug 面**，与桥内那份同值）。
    session_key: [u8; 32],
    /// **类 B 登记簿**：类 B 占位符指纹 → 真值。
    ///
    /// **与占位族的登记簿分开**（这是类 B 通道的硬保证）：占位族登记簿里的值
    /// 会被**受控回程面**（模型回信 → 本地）还原 ⇒ 真值可能进 transcript；
    /// 类 B 登记簿里的值**只**认**显式的本地消费面**
    /// （[`SessionRedactionDomain::restore_for_local_consumer`]）
    /// ⇒ 类 B 真值**永不**进 transcript / 事件日志 / 账本。
    class_b_registry: BTreeMap<String, String>,
    /// **类 B 脱敏账本**（零真值：类别 / 指纹 / 时间 / 还原与否）。
    class_b_ledger: Vec<LedgerEntry>,
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
            session_key,
            class_b_registry: BTreeMap::new(),
            class_b_ledger: Vec::new(),
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

    /// **类 B 结果面清扫**: 凭据材料 → **可逆占位符**（登记进类 B 登记簿）.
    ///
    /// # 与占位族清扫的差别（这是类 B 通道的全部意义）
    ///
    /// | 面 | 登记簿 | 谁能还原 | 真值会进 transcript 吗 |
    /// |---|---|---|---|
    /// | 占位族清扫 ([`Self::sweep_result_face`]) | 占位族登记簿 | 受控回程面（**自动**） | **会** |
    /// | **类 B 清扫**（本函数） | **类 B 登记簿** | **只有显式的本地消费面** | **不会** |
    ///
    /// # 三步
    ///
    /// ① **形状检测**: 占位族检测器（PII 族）+ 类 B 残余族（占位族检测器**不覆盖**
    ///    的凭据形状: PEM 私钥块 / JWT / 明文口令赋值 / 长 AWS key / 长 PAT /
    ///    Slack 令牌 / 带异字符的 `sk-`）;
    /// ② **登记 + 换占位符**: 命中的真值登记进**类 B 登记簿**（指纹 = 会话指纹,
    ///    **与占位族指纹不同盐** ⇒ 两本簿的指纹永不相撞）⇒ 换成
    ///    `__AP_<类别>_<指纹12>__` ⇒ **模型侧只见占位符**;
    /// ③ **单向绊线兜底**: 检测器漏网的形状由既有单向绊线贴单向标签（真值丢弃,
    ///    但**模型面不泄漏**）⇒ 宁可丢值, 不可漏值.
    ///
    /// # 兜底怎么跑（与既有那族一致）
    ///
    /// 兜底跑的是**既有那趟双层扫**（占位族 + 单向绊线）, 但跑在**已经换过占位符
    /// 的文本**上 ⇒ 占位族认不出已换过的（占位符是最高保护层）⇒ 这一趟实际只
    /// 做「漏网形状的单向标签」⇒ **类 B 真值不会落进占位族登记簿** ⇒ 受控回程
    /// 面（自动还原）依然认不出它.
    ///
    /// # 诚实边界
    ///
    /// ③ 是**兜底**, 不是主路: 残余族与占位族检测器同口径, 但**不是逐字同源**
    /// （本通道自带一份可逆检测, 绊线那份是不可逆标签面）⇒ 极端形态仍可能
    /// 落到「单向标签」这一档（**丢值不漏值**）.
    pub fn sweep_class_b_result_face(&mut self, raw: &str) -> EgressSweepOutcome {
        let spans = class_b_secret_spans(raw);
        let mut text = raw.to_string();
        let mut hits: Vec<RedactionHit> = Vec::new();
        // 从后往前替换 ⇒ 偏移不失效。
        for span in spans.iter().rev() {
            let value = &text[span.start..span.end];
            let fingerprint = self.register_class_b_value(span.kind, value);
            let placeholder = RedactionPlaceholder::new(span.kind, &fingerprint).render();
            text.replace_range(span.start..span.end, &placeholder);
            hits.push(RedactionHit {
                kind: span.kind,
                fingerprint,
                surface: RedactionSurface::Egress,
            });
        }
        hits.reverse();
        // **兜底**: 既有那趟双层扫（占位族 + 单向绊线）。
        let sweep = ToolGuardrail::sweep_output_for_egress(&text, &mut self.bridge);
        hits.extend(sweep.redaction.hits.iter().cloned());
        EgressSweepOutcome {
            report: EgressSweepResult {
                redaction: EgressRedaction {
                    text: sweep.redaction.text,
                    hits,
                },
                tripwire: sweep.tripwire,
            },
        }
    }

    /// 登记一个类 B 真值（进类 B 登记簿）⇒ 返回它的会话指纹.
    ///
    /// 指纹的盐与占位族**不同**（域分离标签派生）⇒ 同一个值在两本簿里是两个
    /// 指纹 ⇒ 占位族登记簿认不出的类 B 占位符, 反之亦然 ⇒ 两本簿互不串味.
    fn register_class_b_value(&mut self, kind: PiiKind, value: &str) -> String {
        let fingerprint = session_fingerprint(&self.class_b_key(), value);
        self.class_b_registry
            .entry(fingerprint.clone())
            .or_insert_with(|| value.to_string());
        self.class_b_ledger.push(LedgerEntry {
            ts_unix_ms: now_unix_ms(),
            category: kind.label().to_string(),
            fingerprint: fingerprint.clone(),
            event: LedgerEvent::Redacted,
            restored: false,
            surface: RedactionSurface::Egress,
        });
        fingerprint
    }

    /// 类 B 登记簿的指纹盐（与占位族的会话密钥域分离）.
    fn class_b_key(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"apeireth/class-b-registry/v1");
        hasher.update(&self.session_key);
        hasher.finalize().into()
    }

    /// 受控回程还原 (双闸): 已登记占位符 → 真值; 未登记的照实留痕.
    ///
    /// 同一个域里, 别的面产的占位符在这里认账 —— 这是「跨面同域可互认」的
    /// 落点.
    ///
    /// ⚠️ **只认账占位族登记簿**（`RedactionBridge` 那一本）⇒ **类 B 真值不在
    /// 这里**（类 B 真值只走 [`SessionRedactionDomain::restore_for_local_consumer`]）.
    pub fn restore_for_local(&mut self, text: &str) -> RestoreReport {
        self.bridge.restore_for_local(text)
    }

    /// **本地消费面**（显式调用）: 把**类 B** 占位符解析回真值.
    ///
    /// 这是类 B 通道里**唯一**允许真值离开占位族的面 —— 调用方是**本地消费方**
    /// （拿真值去做本地的事: 调本地 API / 落本地存储 / 交给宿主）.
    ///
    /// # 硬红线: 为什么模型面永远不会拿到真值
    ///
    /// * 本函数**必须被显式调用** —— 结果面 / 出境面 / 受控回程面**都不调它**;
    /// * 类 B 真值登记在**类 B 登记簿**（与占位族登记簿分开）⇒ 受控回程面
    ///   （[`SessionRedactionDomain::restore_for_local`]）**认不出**类 B 占位符
    ///   ⇒ 模型回信里带回的占位符**不会被自动还原** ⇒ 真值不进 transcript;
    /// * 本面只认账**本域类 B 登记簿**里的指纹; 其余占位符原样留着（那是占位族
    ///   自己的面, 本面不代劳、也不记账）.
    ///
    /// # 零真值泄漏
    ///
    /// 返回值 [`RestoreReport`] 的 `Debug` 面**只报指纹与长度**, 不打印文本
    /// ⇒ 日志面永不落明文; 脱敏账本只落**类别 + 指纹**（见 [`Self::ledger_jsonl`]）.
    pub fn restore_for_local_consumer(&mut self, text: &str) -> RestoreReport {
        let matches = RedactionPlaceholder::scan(text);
        let mut out = text.to_string();
        let mut restored = Vec::new();
        for matched in matches.iter().rev() {
            let fingerprint = matched.placeholder.fingerprint.clone();
            let Some(value) = self.class_b_registry.get(&fingerprint) else {
                // 不是本域类 B 登记的指纹 ⇒ 本面不认账（照实留着）。
                continue;
            };
            out.replace_range(matched.start..matched.end, value);
            restored.push(fingerprint.clone());
            self.class_b_ledger.push(LedgerEntry {
                ts_unix_ms: now_unix_ms(),
                category: matched.placeholder.kind.label().to_string(),
                fingerprint,
                event: LedgerEvent::Restored,
                restored: true,
                surface: RedactionSurface::LocalRestore,
            });
        }
        restored.reverse();
        RestoreReport {
            text: out,
            restored,
            unrestored: Vec::new(),
        }
    }

    /// 脱敏账本条目 (零真值: 类别 / 指纹 / 时间 / 还原与否).
    ///
    /// **占位族 + 类 B 两本账合并**（类 B 的条目在后）⇒ 一个域只有一条账.
    pub fn ledger_entries(&self) -> Vec<LedgerEntry> {
        let mut entries = self.bridge.ledger_entries().to_vec();
        entries.extend(self.class_b_ledger.iter().cloned());
        entries
    }

    /// 脱敏账本 JSONL (零真值).
    pub fn ledger_jsonl(&self) -> String {
        let mut out = self.bridge.ledger_jsonl();
        for entry in &self.class_b_ledger {
            out.push_str(
                &serde_json::to_string(entry).expect("ledger entry serialization cannot fail"),
            );
            out.push('\n');
        }
        out
    }

    /// 登记簿里的真值数 (域的大小: 占位族 + 类 B).
    pub fn registered_values(&self) -> usize {
        self.bridge.registered_values() + self.class_b_registry.len()
    }

    /// **类 B 登记簿**里的真值数（类 B 通道的体量）.
    pub fn class_b_values(&self) -> usize {
        self.class_b_registry.len()
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
            .field("registered", &self.registered_values())
            .field("class_b_registered", &self.class_b_registry.len())
            .field("ledger_entries", &self.bridge.ledger_entries().len())
            .field("class_b_ledger_entries", &self.class_b_ledger.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 域注册表 (one domain per session)
// ---------------------------------------------------------------------------

/// **域注册表** —— `session_id → 域` 的持有者.
///
/// # 为什么需要它（把「一个域」降成「一个会话一只域」）
///
/// 3c-1 落的 [`SessionRedactionDomain`] 是「一个会话一个域」的**持有者**，但**注入
/// 面是进程级**的（外部工具桥持**一只**域）⇒ ⇒ **一进程一域** ⇒ ⇒ 多会话并发时，
/// **跨会话的同一个值**会得到**同一个占位符** ⇒ ⇒ 跨会话可关联 ⇒ ⇒ 违裁决 #4
/// 「一个会话一个域」。
///
/// 本类型把粒度落到**会话**：
///
/// * [`SessionDomainRegistry::domain_for`] —— **逐调用按会话取域**（没有就建一只）;
/// * [`SessionDomainRegistry::end_session`] —— **会话结束 ⇒ 域销毁**（登记簿随之消失）。
///
/// # 不变量（跨会话绝不共用域）
///
/// * **跨会话 ⇒ 不同占位符** ⇒ 模型**不可**跨会话关联同一个值;
/// * **跨会话 ⇒ 不可互还原** —— 别会话的域**认不出**本会话登记过的指纹;
/// * **同会话 ⇒ 恒同域** ⇒ 同会话跨两个工具仍是**同一个**占位符（模型可关联）。
///
/// # 生命周期（如实登记）
///
/// * **域按会话创建**：第一次用到某会话时建一只，密钥由**会话 id** 派生
///   （[`derive_session_key`]）⇒ 同会话恒同别名;
/// * **会话结束 ⇒ 域销毁**：[`Self::end_session`] 把域从注册表里摘掉 ⇒ 域连同
///   它的**登记簿（真值驻留处）**与账本一起消失;
/// * ⚠️ **跨进程 / 跨重启续接不成立**：登记簿**只在内存**，且会话密钥掺了
///   **进程级秘密**（[`derive_session_key`]）⇒ 新进程里同一个会话 id 派生出的
///   密钥**不同** ⇒ 别名不同、真值也没了。**本类型不假装支持续接。**
pub struct SessionDomainRegistry {
    /// 活着的域：`session_id → 域句柄`。
    domains: Mutex<BTreeMap<SessionId, Arc<Mutex<SessionRedactionDomain>>>>,
}

impl SessionDomainRegistry {
    /// 新建一只**空**注册表（还没有任何会话的域）。
    pub fn new() -> Self {
        Self {
            domains: Mutex::new(BTreeMap::new()),
        }
    }

    /// **逐调用按会话取域**：这个会话的域（没有就建一只）。
    ///
    /// 同一会话**恒同**一只 ⇒ 同会话跨两个工具是同一个占位符；
    /// 不同会话**各持一只** ⇒ 跨会话同值是不同占位符。
    pub fn domain_for(&self, session: SessionId) -> Arc<Mutex<SessionRedactionDomain>> {
        let mut domains = self
            .domains
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(
            domains.entry(session).or_insert_with(|| {
                Arc::new(Mutex::new(SessionRedactionDomain::for_session(session)))
            }),
        )
    }

    /// **会话结束 ⇒ 域销毁**（登记簿随之消失）。
    ///
    /// 返回 `true` = 这个会话的域确实存在过（已被销毁）；`false` = 本来就没有
    /// （重复结束会话是**幂等**的，不报错）。
    pub fn end_session(&self, session: SessionId) -> bool {
        let mut domains = self
            .domains
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        domains.remove(&session).is_some()
    }

    /// 注册表里**活着的域数**（= 活着的会话数）。
    pub fn len(&self) -> usize {
        self.domains
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// 一个域都没有。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 某个会话的域**在不在**（观测面）。
    pub fn contains(&self, session: SessionId) -> bool {
        self.domains
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(&session)
    }
}

impl Default for SessionDomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SessionDomainRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 只报**计数** ⇒ Debug 面零真值（域里的登记簿不在这里）。
        f.debug_struct("SessionDomainRegistry")
            .field("sessions", &self.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 类 B 声明面 (the declaration surface)
// ---------------------------------------------------------------------------

/// **类 B 声明面** —— 「本工具的结果面合法地就是凭据」的声明清单.
///
/// # 为什么是这个形态
///
/// * **可序列化**: `serde` 的 `Serialize`/`Deserialize` ⇒ 能进配置文件（生产侧
///   从数据目录的一份 JSON 读）、能进审计导出;
/// * **可审计**: 载体是**工具标识**（模型面名 `mcp:<server>:<tool>`）+ 一句
///   理由; [`ClassBDeclaration::audit_lines`] 给出逐条摘要（**零真值**: 标识与
///   理由都不是秘密 —— 理由由操作者写, 写坏了也不含值）;
/// * **默认不含**: [`Default`] = **空清单** ⇒ 默认**没有任何工具**被声明 ⇒
///   结果面行为与接线前**逐字节不变**.
///
/// # 声明了什么
///
/// 声明的是**「这个工具的结果面合法地就是凭据」**这件事, 不是「可以泄露」——
/// 声明后该工具的结果面走**可逆占位族**（模型侧只见占位符）, 真值只由**显式的
/// 本地消费面**交还（[`SessionRedactionDomain::restore_for_local_consumer`]）.
/// **未声明的工具行为不变**（真值仍被单向销毁 —— 那是既有误伤现状, 本通道
/// **不扩大**它, 也**不假装**它已修）.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassBDeclaration {
    /// 被声明为类 B 的工具（模型面名 `mcp:<server>:<tool>`）.
    ///
    /// 用**模型面名**而不是能力 id: 模型面名是操作者在工具目录里**看得见**的
    /// 那个名字（`mcp:<server>:<tool>`）⇒ 声明清单与工具目录**逐字对得上**,
    /// 审计时一眼能核.
    #[serde(default)]
    pub tools: BTreeSet<String>,
    /// 声明理由（审计用; **不得含真值**）.
    #[serde(default)]
    pub reason: Option<String>,
}

impl ClassBDeclaration {
    /// 空声明（= 没有工具被声明）.
    pub fn empty() -> Self {
        Self::default()
    }

    /// 用一组模型面名建声明.
    pub fn with_tools<I, S>(tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tools: tools.into_iter().map(Into::into).collect(),
            reason: None,
        }
    }

    /// 挂上声明理由（审计用）.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// 这个模型面名被声明为类 B 吗.
    pub fn declares(&self, model_name: &str) -> bool {
        self.tools.contains(model_name)
    }

    /// 声明条数.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 没有声明任何工具.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// **可审计**的逐条摘要（零真值: 只有工具标识与声明理由）.
    pub fn audit_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .tools
            .iter()
            .map(|tool| format!("class-b declared: {tool}"))
            .collect();
        if let Some(reason) = &self.reason {
            lines.push(format!("class-b reason: {reason}"));
        }
        lines
    }

    /// 从一份 JSON 文件读声明（**文件不存在 = 空声明** ⇒ 默认不含任何声明）.
    ///
    /// 文件格式:
    /// ```json
    /// { "tools": ["mcp:demo:get_my_api_key"], "reason": "该工具按契约返回凭据" }
    /// ```
    ///
    /// **文件坏了 = 不猜**: 返回错误 ⇒ 调用方按 **fail-closed** 处理（声明保持空
    /// ⇒ 通道保持关）并如实报错, 不静默降级、也不假装已开.
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, ClassBLoadError> {
        if !path.exists() {
            return Ok(Self::empty());
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }
}

/// 类 B 声明文件（**默认名**）—— 与 MCP 服务器清单同一个数据目录.
///
/// 生产侧的读取点: 装配根（`runtime-assembly/src/canonical/production.rs`）。
pub const CLASS_B_DECLARATION_FILE: &str = "mcp-class-b.json";

/// 类 B 声明的读取失败（**fail-closed**: 读不到 = 声明为空, 不猜）.
#[derive(Debug, thiserror::Error)]
pub enum ClassBLoadError {
    /// 文件读不到.
    #[error("类 B 声明文件读取失败: {0}")]
    Io(#[from] std::io::Error),
    /// 文件不是合法的声明 JSON.
    #[error("类 B 声明文件解析失败: {0}")]
    Parse(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// 类 B 形状检测 (可逆凭据形状 → 区间)
// ---------------------------------------------------------------------------

/// 一个被判定为**凭据材料**的文本区间.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClassBSecretSpan {
    kind: PiiKind,
    start: usize,
    end: usize,
}

/// 类 B 通道的凭据形状检测: 占位族检测器（PII 族）+ 类 B 残余族.
///
/// 输出**去重后**的区间（重叠取先到者）⇒ 同一段文本不会被登记两次.
fn class_b_secret_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans: Vec<ClassBSecretSpan> = Vec::new();
    // ① 占位族检测器（PII 族）: 邮箱 / 电话 / 凭据键 / 环境变量密钥 / URL 凭据 …
    for finding in PiiDetector::findings(text) {
        spans.push(ClassBSecretSpan {
            kind: finding.kind,
            start: finding.start,
            end: finding.end,
        });
    }
    // ② 类 B 残余族: 占位族检测器**不覆盖**的凭据形状.
    spans.extend(class_b_residual_spans(text));
    dedupe_class_b_spans(spans)
}

/// 区间去重（重叠取先到者）.
fn dedupe_class_b_spans(mut spans: Vec<ClassBSecretSpan>) -> Vec<ClassBSecretSpan> {
    spans.sort_by_key(|span| (span.start, span.end));
    let mut kept: Vec<ClassBSecretSpan> = Vec::new();
    for span in spans {
        if kept
            .iter()
            .any(|other| span.start < other.end && other.start < span.end)
        {
            continue;
        }
        kept.push(span);
    }
    kept
}

/// 类 B 残余族: 占位族检测器（PII 族）**不覆盖**的凭据形状.
///
/// 口径与既有单向绊线**同族**（PEM 私钥块 / JWT / 明文口令赋值 / `sk-` /
/// AWS key / PAT / Slack 令牌）, 但**产出真值区间**（不是单向标签）⇒ 类 B 通道
/// 能把它们登记进类 B 登记簿 ⇒ 显式本地消费面可还原.
fn class_b_residual_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    spans.extend(pem_private_key_spans(text));
    spans.extend(pem_marker_spans(text));
    spans.extend(jwt_spans(text));
    spans.extend(password_assignment_spans(text));
    spans.extend(prefixed_credential_spans(text));
    spans
}

/// **裸标记**残余: 文本里出现 `PRIVATE KEY-----` 却没有可整块收口的私钥块
/// （缺 END 行 / BEGIN 在标记之后）⇒ 该标记本身要登记（否则模型面看得见
/// 「这里曾经是私钥」的碎片）.
fn pem_marker_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    const MARKER: &str = "PRIVATE KEY-----";
    let mut cursor = 0usize;
    while let Some(rel) = text[cursor..].find(MARKER) {
        let start = cursor + rel;
        spans.push(ClassBSecretSpan {
            kind: PiiKind::CredentialKey,
            start,
            end: start + MARKER.len(),
        });
        cursor = start + MARKER.len();
    }
    spans
}

/// PEM 私钥块（块首行 → **该块自己的** END 行）.
///
/// 与单向绊线同锚: 块首与块尾必须落在**同一个块**上（前置块如 CERTIFICATE 的
/// END 行之后, 第一个 `KEY-----` 其实落在**私钥块的 BEGIN 行**里 ⇒ 只按
/// 「全文第一个 BEGIN」取尾会只换掉块首行, 私钥体留在明文里）.
fn pem_private_key_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = text[cursor..].find("-----BEGIN") {
        let begin_at = cursor + rel;
        let header_end = text[begin_at..]
            .find('\n')
            .map(|at| begin_at + at)
            .unwrap_or(text.len());
        let header = &text[begin_at..header_end];
        if header.contains("PRIVATE KEY-----") {
            let block_end = text[header_end..].find("-----END").and_then(|offset| {
                let end_at = header_end + offset;
                text[end_at..]
                    .find("KEY-----")
                    .map(|offset| end_at + offset + "KEY-----".len())
            });
            match block_end {
                Some(full_end) => {
                    spans.push(ClassBSecretSpan {
                        kind: PiiKind::CredentialKey,
                        start: begin_at,
                        end: full_end,
                    });
                    cursor = full_end;
                }
                None => {
                    // 截断块（无 END 行）⇒ 只登记块首行（**不声称**全扫）.
                    spans.push(ClassBSecretSpan {
                        kind: PiiKind::CredentialKey,
                        start: begin_at,
                        end: header_end,
                    });
                    break;
                }
            }
        } else {
            // 不是私钥块 ⇒ 跳到**该块自己的** END 行之后, 继续往后找.
            let end_at = text[header_end..]
                .find("-----END")
                .map(|at| header_end + at);
            cursor = match end_at {
                Some(at) => text[at..]
                    .find('\n')
                    .map(|offset| at + offset + 1)
                    .unwrap_or(text.len()),
                None => text.len(),
            };
        }
    }
    spans
}

/// JWT（`header.payload.signature`, base64url 三段）.
fn jwt_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("eyJ") {
        let start = from + rel;
        let mut seg_lens = [0usize; 3];
        let mut seg = 0usize;
        let mut index = start;
        while index < hay.len() {
            let byte = hay[index];
            if is_base64url_byte(byte) {
                seg_lens[seg] += 1;
                index += 1;
            } else if byte == b'.' && seg < 2 {
                seg += 1;
                index += 1;
            } else {
                break;
            }
        }
        let shaped = seg == 2
            && seg_lens[0] >= 8
            && seg_lens[1] >= 4
            && (seg_lens[2] == 0 || seg_lens[2] >= 8)
            && (index - start) >= 20;
        if shaped {
            spans.push(ClassBSecretSpan {
                kind: PiiKind::CredentialKey,
                start,
                end: index,
            });
            from = index;
        } else {
            from = start + 3;
        }
        if from >= text.len() {
            break;
        }
    }
    spans
}

/// 明文口令赋值（`password: value` / `password=value`）—— 只登记**值**, 键名留可读.
fn password_assignment_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = find_ascii_ci(&text[search_from..], "password") {
        let key_start = search_from + rel;
        let after_key = key_start + "password".len();
        let rest = &text[after_key..];
        let trimmed_start = after_key + (rest.len() - rest.trim_start().len());
        if text[trimmed_start..].starts_with(':') || text[trimmed_start..].starts_with('=') {
            let value_start_raw = &text[trimmed_start + 1..];
            let lead = value_start_raw.len() - value_start_raw.trim_start().len();
            let value_start = trimmed_start + 1 + lead;
            let value_end = text[value_start..]
                .find('\n')
                .map(|offset| value_start + offset)
                .unwrap_or(text.len());
            let value = &text[value_start..value_end];
            // 受控占位符不是明文口令（占位符族豁免）.
            if value.len() >= 4 && RedactionPlaceholder::parse(value.trim()).is_none() {
                spans.push(ClassBSecretSpan {
                    kind: PiiKind::CredentialKey,
                    start: value_start,
                    end: value_end,
                });
                search_from = value_end;
                continue;
            }
        }
        search_from = after_key;
        if search_from >= text.len() {
            break;
        }
    }
    spans
}

/// 前缀族（`sk-` / `AKIA` / `ghp_` / `xox*`）—— 与单向绊线**同分隔符口径**.
///
/// 口径差在哪: 占位族检测器用**字符类**收尾（`sk-[A-Za-z0-9_-]{20,}`）⇒ 带异
/// 字符的令牌（`sk-a!b…`）它只认出前缀那一小段; 绊线用**分隔符**收尾（空白/引号）
/// ⇒ 认出整枚. 本族取**分隔符**口径 ⇒ 异字符令牌也整枚登记.
fn prefixed_credential_spans(text: &str) -> Vec<ClassBSecretSpan> {
    let mut spans = Vec::new();
    // `sk-`: 空白或引号收尾, 整枚 ≥ 20 字节.
    push_prefixed_spans(
        text,
        "sk-",
        |c| c.is_whitespace() || c == '"' || c == '\'',
        |token| token.len() >= 20,
        &mut spans,
    );
    // `AKIA`: 非字母数字收尾, 16..=32 字节.
    push_prefixed_spans(
        text,
        "AKIA",
        |c| !c.is_ascii_alphanumeric(),
        |token| token.len() >= 16 && token.len() <= 32,
        &mut spans,
    );
    // `ghp_`: 非字母数字（**不含 `_`**, 前缀自身带 `_`）收尾, ≥ 36 字节.
    push_prefixed_spans(
        text,
        "ghp_",
        |c| !c.is_ascii_alphanumeric() && c != '_',
        |token| token.len() >= 36,
        &mut spans,
    );
    // Slack 令牌: `xox[baprs]-`, 空白或引号收尾, ≥ 20 字节.
    for prefix in ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"] {
        push_prefixed_spans(
            text,
            prefix,
            |c| c.is_whitespace() || c == '"' || c == '\'',
            |token| token.len() >= 20,
            &mut spans,
        );
    }
    spans
}

/// 前缀族扫描器（与单向绊线的 `redact_all` 同口径, 但产出**区间**而不是标签）.
fn push_prefixed_spans(
    text: &str,
    prefix: &str,
    is_delimiter: impl Fn(char) -> bool,
    accept: impl Fn(&str) -> bool,
    out: &mut Vec<ClassBSecretSpan>,
) {
    let mut cursor = 0usize;
    while cursor < text.len() {
        let Some(rel) = text[cursor..].find(prefix) else {
            break;
        };
        let start = cursor + rel;
        let tail = &text[start..];
        let end = tail.find(|c: char| is_delimiter(c)).unwrap_or(tail.len());
        let token = &tail[..end];
        if accept(token) {
            out.push(ClassBSecretSpan {
                kind: PiiKind::CredentialKey,
                start,
                end: start + end,
            });
            cursor = start + end;
        } else {
            // 前缀后面不是合法凭证 ⇒ 跳过这一小段继续找.
            cursor = start + prefix.len();
        }
    }
}

/// ASCII 快速子串定位（大小写不敏感, 字节偏移安全 —— 多字节文本不移位）.
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let hay = haystack.as_bytes();
    let nee = needle.as_bytes();
    if nee.len() > hay.len() {
        return None;
    }
    hay.windows(nee.len())
        .position(|window| window.eq_ignore_ascii_case(nee))
}

/// base64url 字母表（JWT 三段用）.
fn is_base64url_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// Unix 毫秒时间戳（账本条目用）.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
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
