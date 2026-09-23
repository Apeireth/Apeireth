//! **W2 §4.1 dreaming 接线** — 做梦引擎的生产胶水与确定性思考器 (2026-10-10)。
//!
//! 三个设计决策点的拍板记录 (工程判断, 理由随行):
//!
//! 1. **`MetaThinker` 生产实现选型 = LLM 版 + 确定性降级链**: LLM 思考器
//!    (`LlmMetaThinker`, 在 `apeireth-runtime-assembly::canonical::dream_llm`, 与
//!    organ 的 LLM 桥同居装配层) 失败时**降级**到本模块的 [`DeterministicMetaThinker`]
//!    并在产出上留痕 —— 元层原则"评审类机制只降级不枪毙"。确定性版单列于此
//!    (memory 层保持零 LLM 依赖, 且降级链的兜底必须永远可用)。
//! 2. **触发载体 = CLI 显式命令 `apeireth dream`** (显式授权 = 无需旋钮, 最诚实;
//!    验收门走命令变体: 显式命令即授权 + 默认不自动跑)。空闲语义 (15min) 无法在
//!    模块钩子里实现 (无回合=无钩子); gateway 空闲 watcher 留作 W2 后续观察项。
//! 3. **落库去向 = [`DiaryStore`]** (引擎设计本意 "苏醒阶段写入日记");
//!    [`dream_and_journal`] 是"执行 6 阶段循环 + 苏醒写日记"的唯一胶水口。
//!
//! **0 假装边界**:
//! - [`InMemoryProceduralStore`] 是引擎契约点名的固化载体, 重启即散 ——
//!   `record_habit` 产出以报告文本为准 (日记留档), 持久化习惯库属后续层;
//! - 本模块**不自触发** (无 idle watcher / 无后台任务), 做梦只在显式命令下发生。

use crate::diary::{DiaryEntry, DiaryStore};
use crate::dreaming::{DreamEngine, DreamError, DreamReport};
use crate::meta_thinking::{MetaThinkInput, MetaThinkOutput, MetaThinker};
use crate::procedural::InMemoryProceduralStore;

/// **确定性规则思考器** (降级链兜底 / 无 LLM 环境的诚实浅推演)。
///
/// 纯规则、无外部调用、同输入恒同输出 (可重放)。产出自带 `[规则推演]` 前缀 ——
/// 0 装自标注: 阅读报告者永远分得清"真思考"与"规则推演"。
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicMetaThinker;

/// 思考量展示截断 (防超长 query 灌爆推演文本)。
const THOUGHT_SNIPPET_CHARS: usize = 120;

fn snippet(text: &str) -> String {
    let mut out: String = text.chars().take(THOUGHT_SNIPPET_CHARS).collect();
    if text.chars().count() > THOUGHT_SNIPPET_CHARS {
        out.push('…');
    }
    out
}

impl MetaThinker for DeterministicMetaThinker {
    fn think(
        &self,
        input: &MetaThinkInput,
    ) -> Result<MetaThinkOutput, crate::meta_thinking::MetaThinkError> {
        let thought = match &input.previous_thought {
            None => format!(
                "[规则推演] 阶段{}·簇「{}」: 就「{}」建立初始坐标, 列出可核对的既有结论与缺口。",
                input.stage,
                input.cluster,
                snippet(&input.query)
            ),
            Some(prev) => format!(
                "[规则推演] 阶段{}·簇「{}」: 承接上阶结论({} 字)与簇上下文({} 字)交叉校核 —— \
                 一致者保留, 冲突者标注「待裁」, 缺口转下阶。原始输入:「{}」。",
                input.stage,
                input.cluster,
                prev.chars().count(),
                input.cluster_context.chars().count(),
                snippet(&input.query)
            ),
        };
        Ok(MetaThinkOutput::new(thought))
    }
}

/// **做梦-日记胶水**: 执行完整 6 阶段做梦循环, 苏醒阶段把
/// [`DreamReport::to_markdown`] 落入 [`DiaryStore`] (source = `"dream"`)。
///
/// 契约: 周期失败 → `Err` (做梦是本任务本体, 不是评审机制, 失败如实报);
/// 周期成功但日记写入失败 → `Err` 且消息**显式声明周期已成** (0 装: 不把
/// "写日记失败"伪装成 "做梦失败", 也不把丢档伪装成功)。
pub fn dream_and_journal(
    engine: &mut DreamEngine,
    recent_memories: &[String],
    thinker: &dyn MetaThinker,
    procedural_store: &InMemoryProceduralStore,
    diary: &dyn DiaryStore,
    date: &str,
    now_ms: i64,
) -> Result<DreamReport, String> {
    let report: DreamReport = engine
        .execute_dream_cycle(recent_memories, thinker, procedural_store)
        .map_err(|e: DreamError| format!("dream cycle failed: {e}"))?;
    let entry = DiaryEntry::new("dream", report.to_markdown(), now_ms);
    if let Err(e) = diary.append(date, entry) {
        return Err(format!(
            "dream cycle completed but journal write failed (date={date}): {e}; \
             report follows at call site: {}",
            report.to_markdown().chars().count()
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diary::InMemoryDiaryStore;
    use crate::dreaming::DreamEngineConfig;

    fn sample_procedural() -> InMemoryProceduralStore {
        InMemoryProceduralStore::new(1000)
    }

    #[test]
    fn deterministic_thinker_is_replayable_and_self_labels() {
        let t = DeterministicMetaThinker;
        let input = MetaThinkInput {
            stage: 1,
            cluster: "经验簇".to_string(),
            query: "今日修复了审批卡死".to_string(),
            cluster_context: "ctx".to_string(),
            previous_thought: None,
        };
        let a = t.think(&input).unwrap();
        let b = t.think(&input).unwrap();
        assert_eq!(a, b, "同输入恒同输出 (可重放)");
        assert!(
            a.thought.starts_with("[规则推演]"),
            "0 装自标注: {0}",
            a.thought
        );

        let input2 = MetaThinkInput {
            stage: 2,
            previous_thought: Some(a.thought.clone()),
            ..input
        };
        let c = t.think(&input2).unwrap();
        assert!(
            c.thought.contains("交叉校核"),
            "递进推演形态: {0}",
            c.thought
        );
    }

    #[test]
    fn dream_engine_is_pull_only_defaults_untouched() {
        // W2 验收门(命令变体)② "默认不自动跑": 引擎无自触发 —— 构造后停在 Awake,
        // 不执行循环则分毫不动 (无后台线程/无计时器/无 Drop 魔法)。
        let mut engine = DreamEngine::new(DreamEngineConfig::default());
        assert_eq!(engine.stage(), crate::dreaming::DreamStage::Awake);
        let report = dream_and_journal(
            &mut engine,
            &[],
            &DeterministicMetaThinker,
            &sample_procedural(),
            &InMemoryDiaryStore::new(),
            "2026-10-13",
            0,
        )
        .expect("explicit pull runs");
        assert!(report.dream_id.starts_with("dream_"));
        assert_eq!(
            engine.stage(),
            crate::dreaming::DreamStage::Awake,
            "循环收尾回 Awake"
        );
    }

    #[test]
    fn dream_writes_diary_entry_holding_the_report() {
        let mut engine = DreamEngine::new(DreamEngineConfig::default());
        let diary = InMemoryDiaryStore::new();
        let procedural = sample_procedural();
        let memories = vec!["今日: 修复审批卡死".to_string()];

        let report = dream_and_journal(
            &mut engine,
            &memories,
            &DeterministicMetaThinker,
            &procedural,
            &diary,
            "2026-10-10",
            1_760_000_000_000,
        )
        .expect("dream cycle + journal");

        let day = diary.read_day("2026-10-10").expect("read day");
        assert_eq!(
            day.entries.len(),
            1,
            "苏醒阶段恰写一条日记: {0:?}",
            day.entries
        );
        assert_eq!(day.entries[0].source, "dream");
        assert_eq!(
            day.entries[0].body,
            report.to_markdown(),
            "日记正文 = 报告 markdown (引擎本意)"
        );
        assert!(!report.to_markdown().is_empty());
    }

    #[test]
    fn dream_with_no_memories_runs_the_structural_path() {
        // 引擎自证: 空记忆走"常规认知结构自整定"而非崩溃 —— 胶水同样成立。
        let mut engine = DreamEngine::new(DreamEngineConfig::default());
        let diary = InMemoryDiaryStore::new();
        let procedural = sample_procedural();
        let report = dream_and_journal(
            &mut engine,
            &[],
            &DeterministicMetaThinker,
            &procedural,
            &diary,
            "2026-10-11",
            1_760_000_000_001,
        )
        .expect("empty-memory dream");
        assert_eq!(diary.read_day("2026-10-11").unwrap().entries.len(), 1);
        assert!(report.to_markdown().contains("dream_"), "报告含 dream id");
    }

    #[test]
    fn journal_failure_is_reported_honestly_without_faking_the_cycle() {
        struct FailingDiary;
        impl DiaryStore for FailingDiary {
            fn append(
                &self,
                _date: &str,
                _entry: DiaryEntry,
            ) -> Result<(), crate::diary::DiaryError> {
                Err(crate::diary::DiaryError::InvalidDate("forced".to_string()))
            }
            fn read_day(
                &self,
                _date: &str,
            ) -> Result<crate::diary::DayPage, crate::diary::DiaryError> {
                unreachable!("test only calls append")
            }
            fn list_days(&self) -> Result<Vec<String>, crate::diary::DiaryError> {
                Ok(Vec::new())
            }
            fn search(
                &self,
                _keyword: &str,
            ) -> Result<Vec<crate::diary::DiaryHit>, crate::diary::DiaryError> {
                Ok(Vec::new())
            }
        }
        let mut engine = DreamEngine::new(DreamEngineConfig::default());
        let err = dream_and_journal(
            &mut engine,
            &[],
            &DeterministicMetaThinker,
            &sample_procedural(),
            &FailingDiary,
            "2026-10-12",
            0,
        )
        .unwrap_err();
        assert!(
            err.contains("dream cycle completed but journal write failed"),
            "0 装: 丢档不伪装成功, 且不掩盖周期已成 —— {err}"
        );
    }
}
