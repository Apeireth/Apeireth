//! # apeireth-sdk-im — IM 快捷接入适配族 (通知/消息/卡片 wire 面)
//!
//! 本模块是「桌面伙伴接进 IM」的 SDK 侧 wire 层: 渠道 kind 词表 +
//! 入站信封解码 + 出站信封组装 + 审批卡片渲染 + 分段/截断策略 +
//! 出站传输 (含断线重连预算) + 共享秘密脱敏。网关侧 (渠道配置装配 /
//! 消息桥 / 审批闭合) 消费本模块, 不重复实现 wire 契约。
//!
//! 与 [`crate::lark`] 适配族的关系: **只消费, 不重写**。`im-feishu` 渠道的
//! 入站信封解析消费 [`crate::lark::webhook::WebhookEvent`], 文本/卡片内容
//! 形状消费 [`crate::lark::message`] 的内容类型, 消息预算锚
//! [`crate::lark::MAX_MESSAGE_TEXT_BYTES`]。
//!
//! ## 面清单
//!
//! | 面 | 内容 |
//! |---|---|
//! | [`channel`] | 渠道 kind (`im-feishu` / `im-wecom` / `im-qq`) + 传输目标 + 脱敏 |
//! | [`message`] | 入站信封解码 (文本消息 / 卡片按钮 / 握手) + 出站信封组装 |
//! | [`card`] | 审批卡片 (命令文本 / 风险级 / 批准·拒绝按钮) + 按钮回调载荷 |
//! | [`segment`] | 回复分段 / 长度截断策略 (显式口径, 截断可见) |
//! | [`transport`] | [`ImSender`] trait + HTTP 实现 + [`ImReconnectPolicy`] |
//! | [`secret`] | 共享秘密持有 (Debug 脱敏) + 入站签名校验 |
//! | [`error`] | 事实错误 + 闭合分类 (可重试性) |
//!
//! ## 安全口径
//!
//! - 渠道共享秘密 0 明文进日志 ([`secret::ImSecret`] Debug/Display 恒脱敏);
//! - 入站签名恒定时间比较, 失败只报事实类别;
//! - 卡片按钮载荷只带闭合身份与超时戳, 不带可执行参数 (展示面 ≠ 授权面)。

pub mod card;
pub mod channel;
pub mod error;
pub mod message;
pub mod secret;
pub mod segment;
pub mod transport;

pub use crate::im::card::{
    is_known_action, render_approval_card, validate_risk_level, ImApprovalCard, ImButtonPayload,
    IM_BUTTON_APPROVE, IM_BUTTON_CANCEL, IM_BUTTON_REJECT, IM_RISK_LEVELS,
};
pub use crate::im::channel::{
    is_absolute_http_url, ImChannelKind, ImChannelTarget, IM_CHANNEL_KIND_COUNT,
};
pub use crate::im::error::{ImError, ImErrorClass, ImResult};
pub use crate::im::message::{
    build_card_body, build_text_body, parse_inbound_event, ImCardAction, ImInboundEvent,
    ImInboundMessage, ImOutboundText, ImSendReceipt,
};
pub use crate::im::secret::{ImSecret, IM_SIGNATURE_PREFIX};
pub use crate::im::segment::{
    segment_reply, ImSegmentPolicy, ImSegmentation, DEFAULT_MAX_SEGMENTS, DEFAULT_TRUNCATION_MARKER,
};
pub use crate::im::transport::{outbound_budget, ImHttpSender, ImReconnectPolicy, ImSender};

/// 归一化 wire 面的 schema 版本 (入站/出站信封契约的同步锚点)。
pub const IM_WIRE_SCHEMA_VERSION: u32 = 1;

/// kind 数与词表守门 (编译期)。
const _: () = assert!(
    IM_CHANNEL_KIND_COUNT == 3,
    "渠道 kind 词表是 3 个中性渠道 id"
);
