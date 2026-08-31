//! async_context: 四层异步上下文生命周期与隔离编排管线
//!
//! 核心自主设计与数学动力学实现:
//! 1. 彻底打破单一线性 `messages` 数组对长任务与工具海量输出的污染；
//! 2. 构建四层异步上下文数组生命周期：
//!    - EphemeralAsyncUser: 即抛型临时中间态（单轮推理有效，AI 读完即销毁，0 历史污染）；
//!    - DurableSyncUser: 核心有效事实（永久沉淀进 SQLite 历史会话流）；
//!    - SummaryStatusUser: 极简状态与耗时摘要（<10 tokens，保留长程任务脉络）；
//!    - NotificationHUDUser: 系统警报与实时 IoT 仪表盘（动态挂起直到被感知消费）；
//! 3. 严格遵循 `#![forbid(unsafe_code)]` 与单向依赖架构。

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 异步上下文消息类型（四层生命周期）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsyncArrayKind {
    /// 1. 即抛型临时中间态：只在当前单轮推理可见，推理结束后立即销毁，不写数据库
    EphemeralAsyncUser,
    /// 2. 核心有效事实：经过沉淀的工具事实或用户输入，持久化入库
    DurableSyncUser,
    /// 3. 极简任务摘要：极小 Token（如 "[Task-42: Success, 120ms]"），长期保留脉络
    SummaryStatusUser,
    /// 4. 仪表盘通知：系统状态、外部 IoT 事件，挂起直到被消费
    NotificationHUDUser,
}

/// 强类型异步上下文消息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncContextMessage {
    pub id: String,
    pub kind: AsyncArrayKind,
    pub role: String,
    pub content: String,
    pub timestamp_ms: u64,
    pub token_estimate: usize,
}

impl AsyncContextMessage {
    pub fn new(
        id: impl Into<String>,
        kind: AsyncArrayKind,
        role: impl Into<String>,
        content: impl Into<String>,
        timestamp_ms: u64,
    ) -> Self {
        let content_str = content.into();
        let token_estimate = content_str.chars().count() / 4 + 1;
        Self {
            id: id.into(),
            kind,
            role: role.into(),
            content: content_str,
            timestamp_ms,
            token_estimate,
        }
    }
}

/// 四层异步上下文编排流水线
#[derive(Debug, Clone, Default)]
pub struct AsyncContextPipeline {
    /// 临时即抛队列（读完即清）
    ephemeral_queue: Vec<AsyncContextMessage>,
    /// 持久事实历史
    durable_history: Vec<AsyncContextMessage>,
    /// 摘要留存列表
    summary_history: Vec<AsyncContextMessage>,
    /// 仪表盘活跃通知
    hud_notifications: VecDeque<AsyncContextMessage>,
    /// 仪表盘容量上限
    max_hud_items: usize,
}

impl AsyncContextPipeline {
    pub fn new(max_hud_items: usize) -> Self {
        Self {
            ephemeral_queue: Vec::new(),
            durable_history: Vec::new(),
            summary_history: Vec::new(),
            hud_notifications: VecDeque::new(),
            max_hud_items: max_hud_items.max(1),
        }
    }

    /// 注入一条异步上下文消息
    pub fn push_message(&mut self, msg: AsyncContextMessage) {
        match msg.kind {
            AsyncArrayKind::EphemeralAsyncUser => {
                self.ephemeral_queue.push(msg);
            }
            AsyncArrayKind::DurableSyncUser => {
                self.durable_history.push(msg);
            }
            AsyncArrayKind::SummaryStatusUser => {
                self.summary_history.push(msg);
            }
            AsyncArrayKind::NotificationHUDUser => {
                if self.hud_notifications.len() >= self.max_hud_items {
                    self.hud_notifications.pop_front();
                }
                self.hud_notifications.push_back(msg);
            }
        }
    }

    /// 组装当前轮次发给大模型的完整上下文
    pub fn assemble_prompt_context(&self) -> Vec<AsyncContextMessage> {
        let mut assembled = Vec::new();

        // 1. 先组装历史摘要与事实
        for item in &self.summary_history {
            assembled.push(item.clone());
        }
        for item in &self.durable_history {
            assembled.push(item.clone());
        }

        // 2. 组装当前未消费的 HUD 仪表盘通知
        for item in &self.hud_notifications {
            assembled.push(item.clone());
        }

        // 3. 组装即抛型中间态（如当前正在执行的工具实时输出）
        for item in &self.ephemeral_queue {
            assembled.push(item.clone());
        }

        assembled
    }

    /// 推理后生命周期结算：销毁全部 Ephemeral，清空已感知的 HUD
    pub fn post_inference_cleanup(&mut self, clear_hud: bool) -> usize {
        let cleared_ephemeral = self.ephemeral_queue.len();
        self.ephemeral_queue.clear();

        if clear_hud {
            self.hud_notifications.clear();
        }

        cleared_ephemeral
    }

    /// 导出需要写入永久持久化存储 (SQLite) 的核心事实列表
    pub fn export_durable_facts(&self) -> &[AsyncContextMessage] {
        &self.durable_history
    }

    /// 导出极简摘要脉络列表
    pub fn export_summary_records(&self) -> &[AsyncContextMessage] {
        &self.summary_history
    }

    /// 当前管线内消息总数
    pub fn total_messages_count(&self) -> usize {
        self.ephemeral_queue.len()
            + self.durable_history.len()
            + self.summary_history.len()
            + self.hud_notifications.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_context_lifecycle_and_cleanup() {
        let mut pipeline = AsyncContextPipeline::new(3);

        // 1. 注入持久事实
        pipeline.push_message(AsyncContextMessage::new(
            "m1",
            AsyncArrayKind::DurableSyncUser,
            "user",
            "What is the weather today?",
            1000,
        ));

        // 2. 注入即抛工具输出 (临时中间态)
        pipeline.push_message(AsyncContextMessage::new(
            "m2",
            AsyncArrayKind::EphemeralAsyncUser,
            "tool",
            "Raw 500KB JSON sensor payload...",
            1001,
        ));

        // 3. 注入极简摘要
        pipeline.push_message(AsyncContextMessage::new(
            "m3",
            AsyncArrayKind::SummaryStatusUser,
            "system",
            "[Tool: weather_sensor -> Success, 45ms]",
            1002,
        ));

        // 4. 注入 HUD 仪表盘
        pipeline.push_message(AsyncContextMessage::new(
            "m4",
            AsyncArrayKind::NotificationHUDUser,
            "system",
            "[IoT Alert: Battery Low 15%]",
            1003,
        ));

        // 组装当前上下文 (应包含全部 4 项)
        let assembled = pipeline.assemble_prompt_context();
        assert_eq!(assembled.len(), 4);

        // 执行后清理 (应清除 Ephemeral)
        let cleared = pipeline.post_inference_cleanup(true);
        assert_eq!(cleared, 1);

        // 再次组装 (只剩 Durable + Summary, Ephemeral 与已消费 HUD 被清空)
        let assembled_after = pipeline.assemble_prompt_context();
        assert_eq!(assembled_after.len(), 2);
        assert_eq!(assembled_after[0].id, "m3"); // Summary
        assert_eq!(assembled_after[1].id, "m1"); // Durable

        // 导出的持久事实仅有 1 条
        assert_eq!(pipeline.export_durable_facts().len(), 1);
        assert_eq!(pipeline.export_durable_facts()[0].id, "m1");
    }
}
