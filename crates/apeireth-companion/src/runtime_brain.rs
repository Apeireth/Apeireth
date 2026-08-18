//! `apeireth-companion::runtime_brain` — 机制件运行时聚合 (E4 好奇 + F1 情绪 + F4 假设 + TP21 目录).
//!
//! ## 定位 (主人: "你全部接进去, 调用真实 LLM 跑通模拟")
//!
//! 把四个机制件接进 CompanionApp 的运行时:
//! - `on_message`: 主人消息进来 → 情绪基线记录 (F1) + 回声喂好奇 (E4).
//! - `tick`: 主动循环 (daemon/proactive_loop) 调用 → 采样好奇目标 (E4) +
//!   目录块刷新 (TP21).
//! - `conjecture`/`resolve`: 探索中发现的可证伪命题 → 假设库 (F4).
//! - `on_feedback`: 主人显式情绪反馈 (F1) — "我今天很烦" 类信号.
//!
//! ## 0 装 PASS
//!
//! - 文本→情绪映射是**确定性启发式** (关键词/标点/长度), 不假装 LLM 情绪分析;
//!   LLM 情绪分析口留给调用方 (tone 已有 LLM 措辞 trait).
//! - 回声来源: 调用方喂 (memory 排名/oracle 意外度), 本模块不假装能自己读记忆.

use std::sync::Mutex;

use crate::curiosity::{CuriosityConfig, CuriosityEngine, Echo, EchoSource, ExplorationTarget};
use crate::emotion_memory::{EmotionMemory, MoodRecord, MoodSnapshot, MoodSource};
use crate::hypothesis::{Hypothesis, HypothesisConfig, HypothesisStatus, HypothesisStore};
use crate::progressive::{CatalogEntry, ProgressiveCatalog};

/// 运行时聚合 (线程安全: 各组件内部 Mutex).
#[derive(Debug)]
pub struct RuntimeBrain {
    pub curiosity: Mutex<CuriosityEngine>,
    pub emotions: Mutex<EmotionMemory>,
    pub hypotheses: Mutex<HypothesisStore>,
    pub catalog: ProgressiveCatalog,
}

impl RuntimeBrain {
    pub fn new(
        curiosity_cfg: CuriosityConfig,
        hypothesis_cfg: HypothesisConfig,
        catalog_entries: Vec<CatalogEntry>,
    ) -> Self {
        Self {
            curiosity: Mutex::new(CuriosityEngine::new(curiosity_cfg)),
            emotions: Mutex::new(EmotionMemory::new()),
            hypotheses: Mutex::new(HypothesisStore::new(hypothesis_cfg)),
            catalog: ProgressiveCatalog::new(catalog_entries),
        }
    }

    /// 主人消息进来: 情绪基线记录 (确定性文本启发式) + 回声喂好奇.
    pub fn on_message(&self, text: &str) {
        let valence = heuristic_valence(text);
        self.emotions.lock().unwrap().record(MoodRecord::new(
            valence,
            0.3,
            MoodSource::TextSignal,
            "对话进行中",
        ));
        // 话题回声: 取文本前 12 字作主题种子 (浅回声, 多轮积累才强)
        let topic: String = text.chars().take(12).collect();
        self.curiosity
            .lock()
            .unwrap()
            .feed_echoes([Echo::new(topic, 0.2, EchoSource::Memory)]);
    }

    /// 主人显式情绪反馈 (F1 高置信来源).
    pub fn on_feedback(&self, valence: f64, note: &str) {
        self.emotions.lock().unwrap().record(MoodRecord::new(
            valence,
            0.6,
            MoodSource::ExplicitFeedback,
            note,
        ));
    }

    /// 当前情绪快照 (F1 输出, 供开口策略/注入).
    pub fn current_mood(&self) -> MoodSnapshot {
        self.emotions.lock().unwrap().current_mood()
    }

    /// 主动循环 tick: 采样好奇目标 (E4 输出, 预算控制).
    pub fn tick(&self, n: usize) -> Vec<ExplorationTarget> {
        self.curiosity.lock().unwrap().sample_targets(n)
    }

    /// 探索完成扣预算 (E4).
    pub fn spend(&self, target: &ExplorationTarget) -> bool {
        self.curiosity.lock().unwrap().spend(target)
    }

    /// 登记可证伪猜想 (F4, 探索中发现).
    pub fn conjecture(&self, statement: &str) -> Hypothesis {
        self.hypotheses.lock().unwrap().conjecture(statement)
    }

    /// 加证据并返回新状态 (F4).
    pub fn add_evidence(
        &self,
        id: u64,
        ev: crate::hypothesis::Evidence,
    ) -> Result<HypothesisStatus, String> {
        self.hypotheses.lock().unwrap().add_evidence(id, ev)
    }

    /// 假设列表 (F4 输出).
    pub fn hypotheses_list(
        &self,
        status: Option<HypothesisStatus>,
    ) -> Vec<crate::hypothesis::Hypothesis> {
        self.hypotheses
            .lock()
            .unwrap()
            .list(status)
            .into_iter()
            .cloned()
            .collect()
    }

    /// 渐进披露目录块 (TP21, 供 ContextAssembler 注入).
    pub fn catalog_block(&self) -> String {
        self.catalog.block()
    }
}

/// 确定性文本→情绪启发式 (valence ∈ [-1,1]).
/// 0 装 PASS: 关键词/标点/长度, 不假装 LLM 情绪分析.
pub fn heuristic_valence(text: &str) -> f64 {
    let t = text.to_lowercase();
    let neg_words = [
        "烦", "累", "难过", "气", "讨厌", "糟糕", "崩溃", "焦虑", "失望", "sad", "tired", "angry",
        "hate",
    ];
    // 注意: 不用单字"好" — "好烦"里是程度副词, 误判积极 (启发式诚实: 宁缺勿错).
    let pos_words = [
        "开心", "高兴", "棒", "爽", "爱", "喜欢", "顺利", "nice", "great", "happy", "love",
    ];
    let mut v: f64 = 0.0;
    for w in neg_words {
        if t.contains(w) {
            v -= 0.4;
        }
    }
    for w in pos_words {
        if t.contains(w) {
            v += 0.4;
        }
    }
    // 感叹号/长句微调 (不假装精确)
    if t.contains('!') {
        v += 0.1;
    }
    v.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypothesis::EvidenceSource;

    fn brain() -> RuntimeBrain {
        RuntimeBrain::new(
            CuriosityConfig::default(),
            HypothesisConfig::default(),
            vec![],
        )
    }

    #[test]
    fn on_message_records_mood_and_echo() {
        let b = brain();
        b.on_message("我今天好烦，项目又黄了");
        let mood = b.current_mood();
        assert!(mood.valence < -0.2, "负面文本 → 负 valence: {:?}", mood);
        assert!(mood.sample_count >= 1);
        // 回声已喂 (主题 = 前 12 字)
        assert!(
            b.curiosity
                .lock()
                .unwrap()
                .echo_of("我今天好烦，项目又黄了")
                > 0.0
        );
    }

    #[test]
    fn on_feedback_high_confidence() {
        let b = brain();
        b.on_feedback(-0.9, "主人: 今天很糟糕");
        let mood = b.current_mood();
        assert!(mood.valence < -0.5);
        assert_eq!(mood.last_source, Some(MoodSource::ExplicitFeedback));
    }

    #[test]
    fn tick_samples_and_spend_controls() {
        let b = brain();
        b.on_message("主人聊投资策略");
        let targets = b.tick(3);
        assert!(!targets.is_empty());
        assert!(b.spend(&targets[0]));
    }

    #[test]
    fn conjecture_and_resolve_flow() {
        let b = brain();
        let h = b.conjecture("主人熬夜后效率低");
        assert_eq!(h.status, HypothesisStatus::Conjecture);
        b.hypotheses.lock().unwrap().start_verify(h.id).unwrap();
        b.add_evidence(
            h.id,
            crate::hypothesis::Evidence::supporting(EvidenceSource::Observation, 1.2, "观察 7 次"),
        )
        .unwrap();
        b.add_evidence(
            h.id,
            crate::hypothesis::Evidence::supporting(EvidenceSource::MasterAnswer, 1.0, "主人确认"),
        )
        .unwrap();
        assert_eq!(
            b.hypotheses_list(None)[0].status,
            HypothesisStatus::Confirmed
        );
    }

    #[test]
    fn catalog_block_available() {
        let b = brain();
        assert_eq!(b.catalog_block(), "");
        let b2 = RuntimeBrain::new(
            CuriosityConfig::default(),
            HypothesisConfig::default(),
            vec![CatalogEntry::new("主人的工作", "投资", 5)],
        );
        assert!(b2.catalog_block().contains("主人的工作"));
    }

    #[test]
    fn heuristic_valence_deterministic() {
        assert!(heuristic_valence("烦死了") < 0.0);
        assert!(heuristic_valence("今天好开心") > 0.0);
        assert_eq!(heuristic_valence("中性句子"), 0.0);
    }
}
