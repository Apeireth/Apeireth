//! `apeireth-companion::daemon` — 总装: 把机制 + 器官 + 渲染 + 送达 + 记忆 接成常驻 daemon.
//!
//! 分层:
//! - `UtteranceGenerator` — 把 Initiative 的诚实事实渲染成「他的话」(`PlainUtterance` = 原文;
//!   真 LLM 渲染见 examples/companion_daemon.rs 的 MiniMaxUtterance).
//! - `Sink` — 通道 (`ConsoleSink` / `LarkSink` 飞书 IM).
//! - `CompanionDelivery` — 渲染 → 通道 组合.
//! - `CompanionDaemon` — 全器官伙伴 + 送达 + 记忆 + 心跳循环.

use std::sync::Arc;
use std::time::Duration;

use apeireth_lark::{LarkClient, LarkConfig, LarkRealImpl, MessageType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::emergence::{Boundaries, Delivery, Feedback, Initiative, SelfScore};
use crate::organs::AwakeCompanion;
use crate::proactive::ContextSource;
use crate::Bond;
use apeireth_core::RiskLevel;

// ============================================================
// 宪法评审 (Judicator) + 评审策略 (成本与风险成正比)
// ============================================================

/// 宪法评审者: 按原则判案. 只在 Medium+ 风险动作时被调用 (评审成本 ∝ 风险).
#[async_trait]
pub trait Judicator: Send + Sync {
    /// 返回 true=ALLOW / false=BLOCK.
    async fn judge(&self, action: &str) -> Result<bool, String>;
}

/// 没接评审器的默认 (诚实: 全放行 — 本地门禁/洋葱门/审批仍在).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopJudicator;

#[async_trait]
impl Judicator for NoopJudicator {
    async fn judge(&self, _action: &str) -> Result<bool, String> {
        Ok(true)
    }
}

/// 评审策略: Low/Info 动作 0 额外 token (本地 Rust 门禁足够);
/// Medium/High 才请宪法评审者 (1 次 LLM); Critical 由洋葱门直接拦.
/// 这正是 stage1 §20.3 风险分级→席位矩阵的经济学表达: 席位随风险, 成本随席位.
pub fn requires_llm_review(risk: RiskLevel) -> bool {
    matches!(risk, RiskLevel::Medium | RiskLevel::High)
}

// ============================================================
// 渲染: 事实 → 他的话
// ============================================================

#[async_trait]
pub trait UtteranceGenerator: Send + Sync {
    async fn utter(&self, initiative: &Initiative) -> Result<String, String>;
}

/// 无 LLM 的默认渲染: 直接返回机制的诚实原文 (不装「已润色」).
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainUtterance;

#[async_trait]
impl UtteranceGenerator for PlainUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        Ok(i.to_message())
    }
}

// ============================================================
// 通道: 发出渲染后的文本
// ============================================================

#[async_trait]
pub trait Sink: Send + Sync {
    async fn send(&self, text: &str) -> Result<(), String>;
}

/// 控制台通道 (真输出).
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsoleSink;

#[async_trait]
impl Sink for ConsoleSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        println!("[他说] {}", text);
        Ok(())
    }
}

/// 飞书 IM 通道 (真 HTTP, 需凭据 + receive_id).
pub struct LarkSink {
    client: Arc<LarkRealImpl>,
    receive_id: String,
}

impl LarkSink {
    pub fn new(config: LarkConfig, receive_id: impl Into<String>) -> Result<Self, String> {
        let client = LarkRealImpl::new(config).map_err(|e| e.to_string())?;
        Ok(Self {
            client: Arc::new(client),
            receive_id: receive_id.into(),
        })
    }
}

#[async_trait]
impl Sink for LarkSink {
    async fn send(&self, text: &str) -> Result<(), String> {
        let content = serde_json::json!({ "text": text }).to_string();
        self.client
            .send_message(&self.receive_id, MessageType::Text, &content)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

// ============================================================
// 组合送达: 渲染 → 通道
// ============================================================

pub struct CompanionDelivery<U: UtteranceGenerator, S: Sink> {
    utter: U,
    sink: S,
}

impl<U: UtteranceGenerator, S: Sink> CompanionDelivery<U, S> {
    pub fn new(utter: U, sink: S) -> Self {
        Self { utter, sink }
    }
}

#[async_trait]
impl<U: UtteranceGenerator, S: Sink> Delivery for CompanionDelivery<U, S> {
    async fn deliver(&self, i: &Initiative) -> Result<(), String> {
        let text = self.utter.utter(i).await?;
        // 隐私护栏 (guard 真件): 出站前检测 + 脱敏 PII
        let matches = apeireth_guard::detect_pii(&text);
        let safe = if matches.is_empty() {
            text
        } else {
            eprintln!("[guard] 出站消息脱敏 {} 处 PII", matches.len());
            apeireth_guard::redact_text(&text, &matches, apeireth_guard::RedactionStrategy::Mask)
        };
        self.sink.send(&safe).await
    }
}

// ============================================================
// 常驻驱动
// ============================================================

/// 总装: 全器官伙伴 + 送达 + 记忆 + 心跳.
pub struct CompanionDaemon<D: Delivery, C: ContextSource> {
    pub awake: AwakeCompanion,
    pub delivery: D,
    pub context: C,
    pub subject: String,
    pub tick_interval: Duration,
}

impl<D: Delivery, C: ContextSource> CompanionDaemon<D, C> {
    pub fn new(
        bond: Bond,
        boundaries: Boundaries,
        delivery: D,
        context: C,
        subject: impl Into<String>,
        tick_interval: Duration,
    ) -> Self {
        Self {
            awake: AwakeCompanion::new(bond, boundaries),
            delivery,
            context,
            subject: subject.into(),
            tick_interval,
        }
    }

    /// 一轮心跳: 记忆检索 → 全器官决策 → 渲染送达.
    pub async fn step(&mut self) {
        let now = Utc::now();
        let hint = self.context.context_for(&self.subject);
        if let Some(init) = self.awake.tick(now, hint) {
            if let Err(e) = self.delivery.deliver(&init).await {
                eprintln!("[daemon] 送达失败: {e}");
            }
        }
    }

    /// 一次用户交互 (任何消息): 喂节律 + 刷新最后接触.
    pub fn on_user_message(&mut self, at: DateTime<Utc>) {
        self.awake.observe_interaction(at);
    }

    /// 用户对上次主动的明确反馈 (回了 / 没回).
    pub fn on_feedback(&mut self, responded: bool, at: DateTime<Utc>) -> SelfScore {
        let f = if responded {
            Feedback::Responded
        } else {
            Feedback::Ignored
        };
        self.awake.apply_feedback(f, at)
    }

    /// 常驻主循环 (永不返回, 由外部 kill / shutdown 终止).
    pub async fn run(mut self) {
        let mut interval = tokio::time::interval(self.tick_interval);
        loop {
            interval.tick().await;
            self.step().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emergence::{ConsoleDelivery, NoopDelivery};
    use crate::proactive::EmptyContext;

    #[tokio::test]
    async fn plain_utterance_returns_honest_facts() {
        let u = PlainUtterance;
        let init = Initiative {
            reason: crate::emergence::InitiativeReason::RhythmMatched { minutes_now: 520 },
            action: crate::actions::Action::Greet,
            rhythm: crate::emergence::RhythmEstimate {
                active_probability: 1.0,
                days: 7,
                confidence: 0.5,
            },
            depth: 0.6,
            context_hint: None,
        };
        let t = u.utter(&init).await.unwrap();
        assert!(t.contains("置信度 50%") && t.contains("概率"));
    }

    #[tokio::test]
    async fn console_sink_prints() {
        ConsoleSink.send("你好").await.unwrap();
    }

    #[test]
    fn daemon_assembles() {
        let mut bond = Bond::new();
        bond.evolve(crate::BondStage::Trusted, 0.6);
        let d = CompanionDaemon::new(
            bond,
            Boundaries::default(),
            ConsoleDelivery,
            EmptyContext,
            "me",
            Duration::from_secs(60),
        );
        assert_eq!(d.subject, "me");
        // 保留引用: NoopDelivery 也能组装
        let _ = CompanionDaemon::new(
            Bond::new(),
            Boundaries::default(),
            NoopDelivery,
            EmptyContext,
            "x",
            Duration::from_secs(1),
        );
    }
}
