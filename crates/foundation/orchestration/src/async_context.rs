//! async_context: 四层异步上下文生命周期与隔离编排管线
//!
//! 本模块为独立实现，解决的是通用上下文卫生问题——单一线性消息数组会被
//! 长任务与工具海量输出污染，因此按生命周期把消息分入四个隔离层：
//!    - EphemeralAsyncUser: 即抛型临时中间态（单轮推理有效，读完即销毁，0 历史污染）；
//!    - DurableSyncUser: 核心有效事实（永久沉淀进会话历史/SQLite）；
//!    - SummaryStatusUser: 极简状态与耗时摘要（<10 tokens，保留长程任务脉络）；
//!    - NotificationHUDUser: 系统警报与实时仪表盘事件（挂起直到被感知消费）。
//! 各层全部有界（超限淘汰最旧），严格遵循 `#![forbid(unsafe_code)]` 与单向依赖架构。

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

/// 粗粒度 token 估算：每 4 个字符折 1 token，保底 1。
const CHARS_PER_TOKEN: usize = 4;

impl AsyncContextMessage {
    pub fn new(
        id: impl Into<String>,
        kind: AsyncArrayKind,
        role: impl Into<String>,
        content: impl Into<String>,
        timestamp_ms: u64,
    ) -> Self {
        let content_str = content.into();
        let token_estimate = content_str.chars().count() / CHARS_PER_TOKEN + 1;
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

/// 即抛队列上限 (M7: 防工具海量输出把即抛层撑爆; 超出丢最旧).
pub const MAX_EPHEMERAL_QUEUE: usize = 256;
/// 持久事实历史上限 (M7: 防长跑无界增长; 超出丢最旧, 更早的事实应已落 SQLite).
pub const MAX_DURABLE_HISTORY: usize = 1_024;
/// 摘要留存上限 (M7: 同上).
pub const MAX_SUMMARY_HISTORY: usize = 512;

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
    ///
    /// M7: 四层全部有界 — 超限淘汰**最旧** (HUD 层沿用 max_hud_items 口径)。
    pub fn push_message(&mut self, msg: AsyncContextMessage) {
        match msg.kind {
            AsyncArrayKind::EphemeralAsyncUser => {
                push_bounded(&mut self.ephemeral_queue, msg, MAX_EPHEMERAL_QUEUE);
            }
            AsyncArrayKind::DurableSyncUser => {
                push_bounded(&mut self.durable_history, msg, MAX_DURABLE_HISTORY);
            }
            AsyncArrayKind::SummaryStatusUser => {
                push_bounded(&mut self.summary_history, msg, MAX_SUMMARY_HISTORY);
            }
            AsyncArrayKind::NotificationHUDUser => {
                if self.hud_notifications.len() >= self.max_hud_items {
                    self.hud_notifications.pop_front();
                }
                self.hud_notifications.push_back(msg);
            }
        }
    }

    /// 组装当前轮次发给大模型的完整上下文。
    ///
    /// 顺序：历史摘要 → 持久事实 → 未消费 HUD 通知 → 即抛中间态。
    pub fn assemble_prompt_context(&self) -> Vec<AsyncContextMessage> {
        self.summary_history
            .iter()
            .chain(self.durable_history.iter())
            .chain(self.hud_notifications.iter())
            .chain(self.ephemeral_queue.iter())
            .cloned()
            .collect()
    }

    /// 推理后生命周期结算：销毁全部 Ephemeral，可选清空已感知的 HUD。
    /// 返回被销毁的即抛消息数。
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

/// 有界追加：入队后超限即从队首（最旧）淘汰。
fn push_bounded(layer: &mut Vec<AsyncContextMessage>, msg: AsyncContextMessage, cap: usize) {
    layer.push(msg);
    while layer.len() > cap {
        layer.remove(0);
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

    /// M7: 三个 Vec (ephemeral / durable / summary) 不得无界增长.
    #[test]
    fn m7_async_context_vecs_bounded() {
        let mut pipeline = AsyncContextPipeline::new(3);
        for i in 0..(MAX_DURABLE_HISTORY + MAX_SUMMARY_HISTORY + MAX_EPHEMERAL_QUEUE + 10) {
            let id = format!("m{i}");
            pipeline.push_message(AsyncContextMessage::new(
                id.clone(),
                AsyncArrayKind::DurableSyncUser,
                "user",
                "fact",
                i as u64,
            ));
            pipeline.push_message(AsyncContextMessage::new(
                id.clone(),
                AsyncArrayKind::SummaryStatusUser,
                "system",
                "sum",
                i as u64,
            ));
            pipeline.push_message(AsyncContextMessage::new(
                id,
                AsyncArrayKind::EphemeralAsyncUser,
                "tool",
                "eph",
                i as u64,
            ));
        }
        assert_eq!(pipeline.export_durable_facts().len(), MAX_DURABLE_HISTORY);
        assert_eq!(pipeline.export_summary_records().len(), MAX_SUMMARY_HISTORY);
        // 最旧的被淘汰, 最新的保留.
        let durable = pipeline.export_durable_facts();
        assert_eq!(
            durable.last().unwrap().id,
            format!(
                "m{}",
                MAX_DURABLE_HISTORY + MAX_SUMMARY_HISTORY + MAX_EPHEMERAL_QUEUE + 9
            ),
            "最新条目应在库"
        );
        // 清理后 ephemeral 归零 (它还有 post_inference_cleanup 的生命周期).
        pipeline.post_inference_cleanup(false);
        assert_eq!(
            pipeline.total_messages_count(),
            MAX_DURABLE_HISTORY + MAX_SUMMARY_HISTORY
        );
    }
}
