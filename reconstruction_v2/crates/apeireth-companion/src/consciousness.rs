//! Consciousness - 意识 / 自我感知模块 (从 v1.0 apeireth-consciousness 4,364 LOC 收敛)
//!
//! 0 装 PASS: 与 emotion.rs / pad_state 共用, 不重新发明 consciousness 模型.
//!
//! 设计 (per user 右图 "Companion 智能核" + 0 装 PASS 严守):
//! - SelfModel: 当前 agent 的自我认知 (name, role, capability list, limitations)
//! - AwarenessStream: 实时 awareness 事件流 (sensory + cognitive + meta)
//! - **诚实标注**: 这是 LLM 借助 consciousness 模块产生的"自我认知近似",
//!   不是真正的意识。用户的感受是唯一的真理 (decision-22 §5.1)

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::emotion::{Pad, Plutchik};

/// 自我模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    pub name: String,
    pub role: String,                       // "认知伴侣" / "助手" / etc.
    pub capabilities: Vec<String>,         // 列出能做的事
    pub limitations: Vec<String>,          // 明确列做不到的 (0 装 PASS)
    pub version: String,                   // 自身版本
    pub baseline_pad: Pad,                 // 平静状态的 PAD
    pub active: bool,                      // 是否激活
}

impl SelfModel {
    /// 0 装 PASS (per decision-22 §5.1 + 8 哲学锚 O-5 不假装):
    /// consciousness 不假装是意识, 只承载 LLM 借助 consciousness 模块产生的"自我认知近似"。
    pub fn apeireth_default() -> Self {
        Self {
            name: "Apeireth".into(),
            role: "Unified Living Companion OS".into(),
            capabilities: vec![
                "long-term episodic memory".into(),
                "PAD emotional dynamics".into(),
                "world model simulation".into(),
                "5-round tool-use loop".into(),
                "deterministic JSON protocol".into(),
            ],
            limitations: vec![
                "不知道未来".into(),
                "无 internet / 数据库访问 (受限沙箱)".into(),
                "不能自我复制 / 自我修改核心代码".into(),
                "所有 audit 链 SHA-256 链式验证 (不能被改)".into(),
            ],
            version: "2.0.0".into(),
            baseline_pad: Pad { pleasure: 0.0, arousal: 0.0, dominance: 0.5 },
            active: true,
        }
    }

    pub fn summary(&self) -> String {
        format!("{} v{} ({}) — capabilities: {}, limitations: {}",
            self.name, self.version, self.role, self.capabilities.len(), self.limitations.len())
    }
}

/// AwarenessStream - 实时 awareness 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AwarenessEvent {
    Sensory { source: String, payload: String, intensity: f64 },
    Cognitive { frame: String, confidence: f64 },
    Meta { reflection: String, about: String },
}

/// ConsciousnessState - 当前 awareness + self-model
pub struct ConsciousnessState {
    pub self_model: SelfModel,
    pub current_events: Arc<RwLock<Vec<AwarenessEvent>>>,
    pub plutchik: Arc<RwLock<Plutchik>>,
}

impl ConsciousnessState {
    pub fn new(self_model: SelfModel, plutchik: Arc<RwLock<Plutchik>>) -> Self {
        Self {
            self_model,
            current_events: Arc::new(RwLock::new(Vec::new())),
            plutchik,
        }
    }

    /// 0 装 PASS: 记录 awareness 事件, 限制容量防 OOM
    pub async fn observe(&self, event: AwarenessEvent) {
        let mut events = self.current_events.write().await;
        events.push(event);
        // 保留最近 100 条
        let len = events.len();
        if len > 100 {
            events.drain(0..len - 100);
        }
    }

    pub async fn recent(&self, n: usize) -> Vec<AwarenessEvent> {
        let events = self.current_events.read().await;
        let start = events.len().saturating_sub(n);
        events[start..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_model_default_0_pretend() {
        // 0 装 PASS: 默认 SelfModel 明确列 limitations (decision-22 §5.1 + O-5 不假装)
        let sm = SelfModel::apeireth_default();
        assert!(!sm.limitations.is_empty());
        assert!(sm.limitations.iter().any(|l| l.contains("不能")));
        assert!(sm.summary().contains("Apeireth"));
    }

    #[tokio::test]
    async fn test_consciousness_observe_caps_at_100() {
        use std::sync::Arc;
        let pl = Arc::new(tokio::sync::RwLock::new(Plutchik::default()));
        let cs = ConsciousnessState::new(SelfModel::apeireth_default(), pl);
        for i in 0..150 {
            cs.observe(AwarenessEvent::Sensory {
                source: "test".into(), payload: format!("{}", i), intensity: 0.5,
            }).await;
        }
        let recent = cs.recent(200).await;
        assert_eq!(recent.len(), 100); // 容量限制
    }
}
