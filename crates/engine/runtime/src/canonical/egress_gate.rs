//! 出网闸端口（第 5 条边界：模型上下文 → provider）。
//!
//! 背景见 `docs/04-internal/backend-enhancement/egress-gate-wiring-design-2026-10-23.md`
//! 与 ADR-0014 §1.2：对 agent 而言「把上下文发给模型」**就是行动**，
//! 因此需要一条与工具审批同级的出口判定。
//!
//! # 分层
//!
//! 本文件只放**端口与判定枚举**：runtime 不依赖 tools / governance 的具体实现
//! （那些在 assembly 层）。默认实现 [`AllowAllEgress`] 一律放行 ——
//! **未接线时逐字节零行为变化**，这是本端口能安全落地的前提。
//!
//! # 三档语义
//!
//! - [`EgressVerdict::Pass`]：原样发出；
//! - [`EgressVerdict::Redacted`]：以脱敏后的请求发出（占位出境，回程另由脱敏闸还原）；
//! - [`EgressVerdict::Deny`]：**拒绝即帧**——不发请求，回可读原因。
//!
//! fail-closed 由实现方保证：判定失败 / 策略读不到 / 闸内部错误，一律返回 `Deny`。

use std::fmt;

use apeireth_protocol::canonical::{NormalizedRequest, NormalizedResponse};

/// 出网闸的判定结果。
#[derive(Debug, Clone)]
pub enum EgressVerdict {
    /// 原样放行。
    Pass,
    /// 以替换后的请求发出（脱敏：真实值已换成占位）。
    Redacted(NormalizedRequest),
    /// 拒绝本次出网；`reason` 必须可读（会进用户可见的拒绝帧）。
    Deny {
        /// 人类可读的拒绝原因（不含敏感真值）。
        reason: String,
    },
}

/// **本地消费面**的结果：占位符 → 本地真值。
///
/// `text` 是**本地**文本（**含真值**）⇒ 它只许交给**本地消费方**，
/// 永不进模型可见面（出境请求 / 模型回信 / transcript）。
///
/// `Debug` 面**只报指纹与长度**，不打印文本 ⇒ 日志面永不落明文。
#[derive(Clone, PartialEq, Eq)]
pub struct LocalConsumerRestore {
    /// 还原后的本地文本（**含真值** ⇒ 只给本地消费方**）。
    pub text: String,
    /// 已还原的占位符指纹（登记簿命中）。
    pub restored: Vec<String>,
}

impl fmt::Debug for LocalConsumerRestore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalConsumerRestore")
            .field("text_len", &self.text.len())
            .field("restored", &self.restored)
            .finish()
    }
}

impl Default for LocalConsumerRestore {
    fn default() -> Self {
        Self {
            text: String::new(),
            restored: Vec::new(),
        }
    }
}

/// 出网闸端口：所有 provider 请求在发出前经过这里。
///
/// 实现方（assembly 层）负责接入脱敏闸、对象级判定与授权策略；
/// 本端口刻意保持同步、只读、无副作用 —— 判定结果由调用方执行。
pub trait EgressGate: Send + Sync {
    /// 判定一次出网请求。
    fn vet(&self, request: &NormalizedRequest) -> EgressVerdict;

    /// **受控回程还原**（双闸的闸二）：把模型回信里的占位符换回本地真值。
    ///
    /// 默认实现 = 不动（未接线时逐字节零行为变化）。
    ///
    /// 语义边界（与 `RedactionBridge::restore_for_local` 一致）：
    /// - 只还原本会话登记簿登记过的指纹，未登记的占位**不还原、照实留痕**；
    /// - 还原只发生在**受控回程面**（本地落盘前），不是出境面；
    /// - 对出境面不可逆 —— 出去的请求里只有占位，没有真值。
    fn restore(&self, _response: &mut NormalizedResponse) {}

    /// **本地消费面**（**显式调用**）：把文本里的占位符换回本地真值。
    ///
    /// 默认实现 = 原样返回（未接线时逐字节零行为变化）。
    ///
    /// # 语义边界（与 [`Self::restore`] 的差别就是全部意义）
    ///
    /// - [`Self::restore`] 是**自动**的（模型回信 → 本地），它认的是**占位族
    ///   登记簿** ⇒ 它还原的值**可能进 transcript**;
    /// - 本方法是**显式**的（本地消费方主动来取），它认的是**类 B 登记簿**
    ///   ⇒ **类 B 真值永不进 transcript**;
    /// - 两者**共用同一个域** ⇒ 同一个占位符在两面都认账（跨族互认）。
    ///
    /// **绝不在模型可见面调用** —— 调用方是本地消费方。
    fn restore_for_local_consumer(&self, text: &str) -> LocalConsumerRestore {
        LocalConsumerRestore {
            text: text.to_string(),
            restored: Vec::new(),
        }
    }
}

/// 默认闸：一律放行（未接线时的零行为变化实现）。
#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAllEgress;

impl EgressGate for AllowAllEgress {
    fn vet(&self, _request: &NormalizedRequest) -> EgressVerdict {
        EgressVerdict::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedMessage};

    fn request() -> NormalizedRequest {
        let message = NormalizedMessage {
            role: MessageRole::User,
            content: vec![ContentPart::Text {
                text: "hello".to_string(),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        };
        NormalizedRequest::new("test-model", vec![message])
    }

    #[test]
    fn default_gate_allows_everything() {
        let gate = AllowAllEgress;
        assert!(matches!(gate.vet(&request()), EgressVerdict::Pass));
    }
}
