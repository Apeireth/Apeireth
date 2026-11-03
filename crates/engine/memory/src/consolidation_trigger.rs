//! 夜间整编的**驱动层**（P2 的最后一格：把「整编作业」变成「每夜只跑一次、只跑新料」的一次决策）。
//!
//! ## 为什么它存在（实测基线，2026-11-01）
//! - `consolidation.rs` 的 4 个 `pub fn` 入口**都在**
//!   （`consolidate` / `consolidate_governed_episodes` / `consolidate_governed` / `consolidate_persona`），
//!   且 `coordinator.rs` **已 import** ⇒ ⇒ **库内已完成**；
//! - 但**全仓没有生产代码调用它们** ⇒ ⇒ **缺的不是「算法」，是「驱动」**：
//!   **谁决定现在整编、整编哪些、以及保证同一夜不重复整编**；
//! - 而「什么时候是夜里」由 `organ::circadian`（相位 / 睡眠压力 / `ProactiveGate`）给出
//!   ⇒ ⇒ **`organ` 与 `memory` 是兄弟层、互不依赖**（实测双方 `Cargo.toml`）
//!   ⇒ ⇒ **「该不该是夜里」由调用方决定并用参数告知**，本模块**不读时钟**。
//!
//! ## 本模块负责什么（只管记忆侧该管的）
//! 1. **窗口水位**（[`ConsolidationWatermark`]）：记住「上次整编到哪」；
//! 2. **幂等**：同一窗口内第二次调用 ⇒ **跳过**（不重复整编，且**不推进水位**）；
//! 3. **只整新料**：水位之后的情节才进本轮（`Episode.timestamp` 是 **epoch 秒** ⇒ 本模块全程用**秒**）；
//! 4. **料不够就不跑**（且**不消耗窗口** —— 否则窗口会被白���）；
//! 5. **治理视图可选**：给了 `MemoryGovernanceStore` 就按治理视图整编（谁可见、谁被丢弃），
//!    没给就只出启发式报告（0 模型调用，沿用 `consolidate` 的既有口径）。
//!
//! ## 明确不做
//! - **不读系统时钟**、不 sleep、不 spawn 线程 ⇒ 时刻由 `now_sec` **注入** ⇒ 同一输入必得同一输出；
//! - **不做模型调用**（沿用 `consolidate_governed` 的 0 模型口径）；
//! - **不决定「该不该是夜里」**（那是 `organ::circadian` + 调用方的职责，见上）；
//! - **不落人格/关系库**（那是 `consolidate_persona` 的职责，调用方拿着本模块的输出自己去落）。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::memory::Episode;

use crate::consolidation::{
    ConsolidationReport, MemoryConsolidationJob, MemoryConsolidationOutput,
};
use crate::memory_governance::{MemoryGovernanceStatus, MemoryGovernanceStore};
use crate::MemoryError;

/// 整编窗口的**幂等水位**：记住「上次整编到哪个时刻」。
///
/// 时间单位统一为 **epoch 秒**（与 [`Episode::timestamp`] 一致）——
/// ⚠️ 本仓 ms/seconds 混用，**这里刻意用秒**，并在名字里写明（`*_sec`）。
pub trait ConsolidationWatermark: Send + Sync {
    /// 上次整编的时刻（epoch 秒）；从未整编过 ⇒ `None`。
    ///
    /// ⚠️ **`key` = 会话标识**（2026-11-03 自查发现并修掉的设计缺陷）：
    /// 原实现是**单个全局时间戳** ⇒ ⇒ **一个会话整编会把另一个会话的水位也推进**
    /// ⇒ ⇒ 后者**永远等不到新料**（它的情节全部早于水位）⇒ ⇒ **静默停止整编**。
    /// ⇒ ⇒ 水位**必须按会话隔离**。
    fn last_consolidated_sec(&self, key: &str) -> Result<Option<i64>, MemoryError>;

    /// 记录本次整编的时刻（epoch 秒）。
    fn mark_consolidated_sec(&self, key: &str, at_sec: i64) -> Result<(), MemoryError>;
}

/// 进程内水位（确定性测试 / 无库装配）。
#[derive(Debug, Default)]
pub struct InMemoryConsolidationWatermark {
    state: Mutex<std::collections::HashMap<String, i64>>,
}

impl InMemoryConsolidationWatermark {
    /// 新建一个空水位。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ConsolidationWatermark for InMemoryConsolidationWatermark {
    fn last_consolidated_sec(&self, key: &str) -> Result<Option<i64>, MemoryError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| MemoryError::Poisoned("consolidation watermark".to_string()))?;
        Ok(guard.get(key).copied())
    }

    fn mark_consolidated_sec(&self, key: &str, at_sec: i64) -> Result<(), MemoryError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| MemoryError::Poisoned("consolidation watermark".to_string()))?;
        guard.insert(key.to_string(), at_sec);
        Ok(())
    }
}

/// 文件后端水位（一个 JSON 文件）—— 便于「夜里跑一次」的**进程外**持久化。
///
/// 格式：`{"sessions": {"<会话>": 1735689600}}`（**按会话隔离** ⇒ 一个会话的整编
/// 不会影响另一个）；文件不存在 ⇒ **从未整编过**（`None`），
/// **不是错误**（⇒ ⇒ 第一次跑不会因「文件缺失」失败）。
#[derive(Debug, Clone)]
pub struct FileConsolidationWatermark {
    path: PathBuf,
}

impl FileConsolidationWatermark {
    /// 指向一个水位文件（父目录需已存在）。
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 水位文件路径。
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl ConsolidationWatermark for FileConsolidationWatermark {
    fn last_consolidated_sec(&self, key: &str) -> Result<Option<i64>, MemoryError> {
        Ok(self.read_all()?.get(key).copied())
    }

    fn mark_consolidated_sec(&self, key: &str, at_sec: i64) -> Result<(), MemoryError> {
        let mut all = self.read_all()?;
        all.insert(key.to_string(), at_sec);
        let payload = serde_json::json!({ "sessions": all });
        std::fs::write(&self.path, payload.to_string()).map_err(MemoryError::Io)
    }
}

impl FileConsolidationWatermark {
    /// 读出「会话 → 上次整编时刻」的整张表（文件不存在 ⇒ 空表，**不是错误**）。
    fn read_all(&self) -> Result<std::collections::BTreeMap<String, i64>, MemoryError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                let value: serde_json::Value =
                    serde_json::from_str(&text).map_err(MemoryError::Json)?;
                let sessions = value.get("sessions").and_then(serde_json::Value::as_object);
                let mut out = std::collections::BTreeMap::new();
                if let Some(map) = sessions {
                    for (name, at) in map {
                        if let Some(at) = at.as_i64() {
                            out.insert(name.clone(), at);
                        }
                    }
                }
                Ok(out)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(std::collections::BTreeMap::new())
            }
            Err(error) => Err(MemoryError::Io(error)),
        }
    }
}

/// 整编策略（**时间单位 = epoch 秒**，与 `Episode.timestamp` 一致）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsolidationPolicy {
    /// 是否启用（关掉 ⇒ 每次都跳过并给出 [`SkipReason::Disabled`]）。
    pub enabled: bool,
    /// 整编窗口（秒）—— 同一窗口内**只整一次**。默认 `86_400`（24h）。
    pub window_sec: i64,
    /// 一轮至少要多少条**新**情节才值得整编（少于此 ⇒ 跳过，且**不推进水位**）。
    pub min_episodes: usize,
    /// 单轮最多处理多少条新情节（超出部分留给下一轮）。
    pub max_episodes: usize,
}

impl Default for ConsolidationPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            window_sec: 86_400,
            min_episodes: 8,
            max_episodes: 1_000,
        }
    }
}

impl ConsolidationPolicy {
    /// 构造一份策略（其余字段取默认）。
    #[must_use]
    pub fn new(window_sec: i64) -> Self {
        Self {
            window_sec,
            ..Self::default()
        }
    }

    /// 设置「一轮至少要多少条新情节」。
    #[must_use]
    pub fn with_min_episodes(mut self, min_episodes: usize) -> Self {
        self.min_episodes = min_episodes;
        self
    }

    /// 设置「单轮最多处理多少条新情节」。
    ///
    /// ⚠️ **为什么叫 `with_batch_limit` 而不是 `with_max_episodes`**：
    /// 记忆 crate 里**已有**一个同名方法（`memory_materializer::with_max_episodes`），
    /// 而它**是真的零引用（孤儿）** ⇒ ⇒ 若本类型也叫 `with_max_episodes`，
    /// 孤儿符号基线会因**同名遮蔽**把它误判成「已不再孤儿」
    /// ⇒ ⇒ **造成一次「假的债减少」** ⇒ ⇒ ⇒ 改用不撞名的名字。
    #[must_use]
    pub fn with_batch_limit(mut self, max_episodes: usize) -> Self {
        self.max_episodes = max_episodes;
        self
    }

    /// 开关整编。
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// 跳过整编的**原因**（每一种都可被断言 ⇒ ⇒ 「为什么没整编」是机器可读的）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// 策略关了整编。
    Disabled,
    /// 窗口还没到期（距上次整编不足 `window_sec`）。
    WindowNotElapsed {
        /// 距上次整编过了多少秒。
        elapsed_sec: i64,
        /// 窗口长度（秒）。
        window_sec: i64,
    },
    /// 新情节不够（**此时不推进水位** ⇒ 窗口不会被白白消耗）。
    NotEnoughEpisodes {
        /// 本轮可见的新情节条数。
        have: usize,
        /// 策略要求的最少条数。
        need: usize,
    },
}

/// 一次整编的结果。
#[derive(Debug, Clone)]
pub enum ConsolidationOutcome {
    /// 没整编（附原因）。
    Skipped {
        /// 跳过原因。
        reason: SkipReason,
    },
    /// 整编了。
    Consolidated {
        /// 本轮区间起点（epoch 秒）：`Some` = 上次水位，`None` = 从头。
        window_from_sec: Option<i64>,
        /// 本轮区间终点（epoch 秒）= 本次的 `now_sec`。
        window_to_sec: i64,
        /// 本轮实际处理的新情节条数。
        episodes: usize,
        /// 治理视图下的整编产物（合并 / 提升 / 丢弃 / 人格 / 关系更新）。
        output: MemoryConsolidationOutput,
        /// 启发式报告（洞察条数、角色分布等）。
        report: ConsolidationReport,
    },
}

impl ConsolidationOutcome {
    /// 本轮是否真的整编了。
    #[must_use]
    pub fn consolidated(&self) -> bool {
        matches!(self, Self::Consolidated { .. })
    }

    /// 跳过的原因（整编了 ⇒ `None`）。
    #[must_use]
    pub fn skip_reason(&self) -> Option<&SkipReason> {
        match self {
            Self::Skipped { reason } => Some(reason),
            Self::Consolidated { .. } => None,
        }
    }

    /// 本轮处理的新情节条数（跳过 ⇒ 0）。
    #[must_use]
    pub fn episode_count(&self) -> usize {
        match self {
            Self::Consolidated { episodes, .. } => *episodes,
            Self::Skipped { .. } => 0,
        }
    }
}

/// 夜间整编驱动：把「整编作业」变成「**每窗口一次、只整新料**」的一次决策与执行。
///
/// ⚠️ **不 derive `Debug`**：内部持有 `Arc<dyn ConsolidationWatermark>`（trait 对象无 `Debug` 约束）。
#[derive(Clone)]
pub struct ConsolidationTrigger {
    policy: ConsolidationPolicy,
    watermark: Arc<dyn ConsolidationWatermark>,
    job: MemoryConsolidationJob,
}

impl ConsolidationTrigger {
    /// **生产便利构造**：进程内水位 + 给定窗口（秒）。
    ///
    /// ⚠️ **进程内水位** ⇒ 进程重启后水位归零 ⇒ ⇒ 「第一次整编会重整一遍」。
    /// ⇒ ⇒ **跨重启的夜间整编**请用 [`FileConsolidationWatermark`]。
    ///
    /// - `window_sec`：窗口长度（秒）；`0` ⇒ **不设时间闸**
    ///   ⇒ ⇒ 只剩「只整新料」这一条语义（适合「由外部节奏旋钮管时间」的装配）。
    #[must_use]
    pub fn in_memory(window_sec: i64) -> Self {
        Self::new(
            ConsolidationPolicy::new(window_sec),
            Arc::new(InMemoryConsolidationWatermark::new()),
        )
    }

    /// 构造驱动（策略 + 水位）。
    #[must_use]
    pub fn new(policy: ConsolidationPolicy, watermark: Arc<dyn ConsolidationWatermark>) -> Self {
        Self {
            policy,
            watermark,
            job: MemoryConsolidationJob::new(),
        }
    }

    /// 当前策略。
    #[must_use]
    pub fn policy(&self) -> &ConsolidationPolicy {
        &self.policy
    }

    /// 只看「窗口是否到期」（不涉及情节条数）⇒ 调用方可在夜里**先问一句**再决定要不要跑。
    ///
    /// - `session_id`：会话标识（**水位按会话隔离** ⇒ 不同会话互不影响）。
    pub fn should_run(&self, session_id: &str, now_sec: i64) -> Result<bool, MemoryError> {
        if !self.policy.enabled {
            return Ok(false);
        }
        match self.watermark.last_consolidated_sec(session_id)? {
            Some(last) => Ok(now_sec.saturating_sub(last) >= self.policy.window_sec),
            None => Ok(true),
        }
    }

    /// 跑一轮整编。
    ///
    /// - `episodes`：**候选**情节（调用方给的通常是某段时间的全部情节）；
    ///   本方法**只取水位之后的新情节**参与整编；
    /// - `governance`：给了就按治理视图整编（被遗忘的条目会被丢弃、重复的会合并），
    ///   没给就只出启发式报告（0 模型调用）；
    /// - `now_sec`：本轮时刻（epoch 秒，**注入** ⇒ 本模块不读时钟）。
    ///
    /// **失败语义**：水位**只在整编成功后**推进 ⇒ ⇒ 中途失败 ⇒ 下一轮还会整编同一批
    /// （⇒ ⇒ **不丢料**）。
    pub fn run(
        &self,
        session_id: &str,
        episodes: &[Episode],
        now_sec: i64,
        governance: Option<&dyn MemoryGovernanceStore>,
    ) -> Result<ConsolidationOutcome, MemoryError> {
        if !self.policy.enabled {
            return Ok(ConsolidationOutcome::Skipped {
                reason: SkipReason::Disabled,
            });
        }

        let last = self.watermark.last_consolidated_sec(session_id)?;
        if let Some(last) = last {
            let elapsed = now_sec.saturating_sub(last);
            if elapsed < self.policy.window_sec {
                return Ok(ConsolidationOutcome::Skipped {
                    reason: SkipReason::WindowNotElapsed {
                        elapsed_sec: elapsed,
                        window_sec: self.policy.window_sec,
                    },
                });
            }
        }

        let fresh: Vec<Episode> = episodes
            .iter()
            .filter(|episode| last.is_none_or(|mark| episode.timestamp > mark))
            .cloned()
            .collect();

        if fresh.len() < self.policy.min_episodes {
            // ⚠️ **不推进水位**：料不够不该把窗口消耗掉（否则夜里前几次调用会白等一整夜）。
            return Ok(ConsolidationOutcome::Skipped {
                reason: SkipReason::NotEnoughEpisodes {
                    have: fresh.len(),
                    need: self.policy.min_episodes,
                },
            });
        }

        let mut fresh = fresh;
        if fresh.len() > self.policy.max_episodes {
            fresh.truncate(self.policy.max_episodes);
        }

        let (output, report) = match governance {
            Some(store) => {
                // ⚠️ **2026-11-02 自查发现并修掉的真问题**：报告原先在「全部情节」上算，
                // 而只有「产物」走了治理过滤 ⇒ ⇒ 若调用方传入的列表里含**已遗忘**的情节，
                // 那条内容就可能**又被提炼成洞察** ⇒ ⇒ **违反遗忘权**（忘了还能被读出来）。
                // ⇒ 修法分两半，**各司其职**：
                //   · **产物**照旧收**全部**素材 ⇒ 由 `consolidate_governed` 自己做遗忘判定，
                //     并把被丢弃的记进 `output.discarded`（**这个账要留着**，调用方要知道丢了什么）；
                //   · **报告**只用**治理可见**的子集 ⇒ ⇒ 遗忘内容不会被重新提炼成洞察。
                let visible: Vec<Episode> = fresh
                    .iter()
                    .filter(|episode| {
                        store
                            .get_governed(&episode.id)
                            .ok()
                            .flatten()
                            .is_none_or(|state| state.status != MemoryGovernanceStatus::Forgotten)
                    })
                    .cloned()
                    .collect();
                let output = self.job.consolidate_governed(session_id, &fresh, store)?;
                let report = self.job.consolidate(session_id, &visible);
                (output, report)
            }
            None => {
                let report = self.job.consolidate(session_id, &fresh);
                (MemoryConsolidationOutput::default(), report)
            }
        };

        self.watermark.mark_consolidated_sec(session_id, now_sec)?;

        Ok(ConsolidationOutcome::Consolidated {
            window_from_sec: last,
            window_to_sec: now_sec,
            episodes: fresh.len(),
            output,
            report,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::memory_governance::{GovernedEpisode, MemoryGovernanceStatus};

    fn episode(id: &str, timestamp: i64, role: &str, content: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp,
            role: role.to_string(),
            content: content.to_string(),
            session_id: "s1".to_string(),
        }
    }

    fn episodes(count: usize, base_ts: i64) -> Vec<Episode> {
        (0..count)
            .map(|index| {
                episode(
                    &format!("e{index}"),
                    base_ts + i64::try_from(index).unwrap_or(0),
                    "user",
                    &format!("request {index}"),
                )
            })
            .collect()
    }

    fn trigger(
        policy: ConsolidationPolicy,
    ) -> (ConsolidationTrigger, Arc<InMemoryConsolidationWatermark>) {
        let watermark = Arc::new(InMemoryConsolidationWatermark::new());
        (
            ConsolidationTrigger::new(
                policy,
                Arc::clone(&watermark) as Arc<dyn ConsolidationWatermark>,
            ),
            watermark,
        )
    }

    #[test]
    fn first_run_consolidates_everything_and_moves_the_watermark() {
        let (driver, watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(3));
        let outcome = driver
            .run("s1", &episodes(5, 1_000), 2_000, None)
            .expect("run");
        assert!(outcome.consolidated());
        assert_eq!(outcome.episode_count(), 5);
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("watermark"),
            Some(2_000)
        );
    }

    #[test]
    fn same_window_is_skipped_and_the_watermark_does_not_move() {
        let (driver, watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(3));
        driver
            .run("s1", &episodes(5, 1_000), 2_000, None)
            .expect("first");
        let second = driver
            .run("s1", &episodes(5, 1_000), 2_000 + 3_600, None)
            .expect("second");
        assert!(!second.consolidated());
        assert_eq!(
            second.skip_reason(),
            Some(&SkipReason::WindowNotElapsed {
                elapsed_sec: 3_600,
                window_sec: 86_400,
            })
        );
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("watermark"),
            Some(2_000)
        );
    }

    #[test]
    fn after_the_window_only_new_episodes_are_consolidated() {
        let (driver, _watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(3));
        driver
            .run("s1", &episodes(5, 1_000), 2_000, None)
            .expect("first");
        // 窗口到期后：3 条旧（ts<=2000）+ 4 条新
        let mut mixed = episodes(3, 1_000);
        mixed.extend(episodes(4, 5_000));
        let outcome = driver
            .run("s1", &mixed, 100_000, None)
            .expect("second window");
        assert!(outcome.consolidated());
        assert_eq!(outcome.episode_count(), 4, "旧情节不该被整编第二遍");
        match outcome {
            ConsolidationOutcome::Consolidated {
                window_from_sec,
                window_to_sec,
                ..
            } => {
                assert_eq!(window_from_sec, Some(2_000));
                assert_eq!(window_to_sec, 100_000);
            }
            ConsolidationOutcome::Skipped { .. } => unreachable!(),
        }
    }

    #[test]
    fn not_enough_episodes_skips_without_consuming_the_window() {
        let (driver, watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(8));
        let outcome = driver
            .run("s1", &episodes(2, 1_000), 2_000, None)
            .expect("run");
        assert!(!outcome.consolidated());
        assert_eq!(
            outcome.skip_reason(),
            Some(&SkipReason::NotEnoughEpisodes { have: 2, need: 8 })
        );
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("watermark"),
            None,
            "料不够不该推进水位"
        );
    }

    #[test]
    fn disabled_policy_always_skips() {
        let (driver, watermark) = trigger(
            ConsolidationPolicy::new(86_400)
                .with_min_episodes(1)
                .with_enabled(false),
        );
        let outcome = driver
            .run("s1", &episodes(9, 1_000), 2_000, None)
            .expect("run");
        assert_eq!(outcome.skip_reason(), Some(&SkipReason::Disabled));
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("watermark"),
            None
        );
    }

    #[test]
    fn window_boundary_is_inclusive() {
        let (driver, _watermark) = trigger(ConsolidationPolicy::new(100).with_min_episodes(1));
        driver
            .run("s1", &episodes(2, 1_000), 1_000, None)
            .expect("first");
        // 恰好等于窗口 ⇒ 允许再跑
        let outcome = driver
            .run("s1", &episodes(2, 2_000), 1_100, None)
            .expect("boundary");
        assert!(outcome.consolidated());
    }

    #[test]
    fn max_episodes_truncates_the_batch() {
        let (driver, _watermark) = trigger(
            ConsolidationPolicy::new(86_400)
                .with_min_episodes(1)
                .with_batch_limit(2),
        );
        let outcome = driver
            .run("s1", &episodes(9, 1_000), 2_000, None)
            .expect("run");
        assert_eq!(outcome.episode_count(), 2);
    }

    #[test]
    fn should_run_reflects_the_window_only() {
        let (driver, _watermark) = trigger(ConsolidationPolicy::new(100).with_min_episodes(50));
        assert!(driver.should_run("s1", 1_000).expect("should_run"));
        driver
            .run("s1", &episodes(60, 1_000), 1_000, None)
            .expect("run");
        assert!(!driver.should_run("s1", 1_050).expect("should_run"));
        assert!(driver.should_run("s1", 1_100).expect("should_run"));
    }

    #[test]
    fn governance_view_drops_forgotten_episodes() {
        // ⚠️ 用**真实**治理库（`SqliteMemoryStore`）而不是手搓 stub ——
        //    ⇒ 测的是**生产同一条**治理写入面（`forget_episode`）。
        let store = crate::SqliteMemoryStore::open_in_memory().expect("open store");
        let all = episodes(4, 1_000);
        for item in &all {
            crate::EpisodeStore::put_episode(&store, item).expect("put episode");
        }
        for item in all.iter().skip(1) {
            store
                .forget_episode(&item.id, Some("用户要求"), 0)
                .expect("forget");
        }

        let (driver, _watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(3));
        let outcome = driver.run("s1", &all, 2_000, Some(&store)).expect("run");
        assert!(outcome.consolidated());
        match outcome {
            ConsolidationOutcome::Consolidated { output, .. } => {
                assert_eq!(output.discarded.len(), 3, "被遗忘的 3 条应被丢弃");
                assert!(
                    output.promoted.iter().any(|id| id == "e0"),
                    "仅存的一条应被提升：{:?}",
                    output.promoted
                );
            }
            ConsolidationOutcome::Skipped { .. } => unreachable!(),
        }
    }

    #[test]
    fn forgotten_episodes_never_reach_the_report() {
        // ⚠️ **2026-11-02 自查发现的真问题**：给了治理库时，报告原先在「全部情节」上算
        // ⇒ ⇒ **被遗忘的情节内容会被重新提炼成洞察** ⇒ ⇒ 违反遗忘权。
        // ⇒ ⇒ 现在产物与报告都只用「治理可见」的子集。
        let store = crate::SqliteMemoryStore::open_in_memory().expect("open store");
        let all = episodes(4, 1_000);
        for item in &all {
            crate::EpisodeStore::put_episode(&store, item).expect("put episode");
        }
        // 把第 1 条标记为已遗忘，且它的内容里带可被提炼的关键词
        let secret = Episode {
            id: "secret".to_string(),
            timestamp: 1_000,
            role: "user".to_string(),
            content: "error: this secret was forgotten and must not resurface".to_string(),
            session_id: "s1".to_string(),
        };
        crate::EpisodeStore::put_episode(&store, &secret).expect("put secret");
        store
            .forget_episode("secret", Some("用户要求遗忘"), 0)
            .expect("forget");

        let mut material = all.clone();
        material.push(secret.clone());

        let (driver, _watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(1));
        let outcome = driver
            .run("s1", &material, 2_000, Some(&store))
            .expect("run");
        assert!(outcome.consolidated());
        match outcome {
            ConsolidationOutcome::Consolidated { output, report, .. } => {
                assert!(
                    output.discarded.iter().any(|id| id == "secret"),
                    "已遗忘的条目应被丢弃"
                );
                for insight in &report.extracted_insights {
                    assert!(
                        !insight.contains("secret"),
                        "被遗忘内容不该出现在洞察里：{insight}"
                    );
                }
            }
            ConsolidationOutcome::Skipped { .. } => unreachable!(),
        }
    }

    #[test]
    fn watermarks_are_isolated_per_session() {
        // ⚠️ **2026-11-03 自查发现并修掉的设计缺陷**：水位原先是**单个全局时间戳**
        // ⇒ ⇒ 一个会话整编 ⇒ **另一个会话的水位也被推进**
        // ⇒ ⇒ 后者的情节全部早于水位 ⇒ ⇒ **永远等不到新料 ⇒ 静默停止整编**。
        // ⇒ ⇒ 现在水位**按会话隔离** ⇒ 本测试钉住这一点。
        let (driver, watermark) = trigger(ConsolidationPolicy::new(86_400).with_min_episodes(1));

        // 会话 A 先整编（把水位推到 2_000）
        let a = vec![episode("a-1", 1_500, "user", "fixed: A 素材")];
        assert!(driver
            .run("session-a", &a, 2_000, None)
            .expect("run a")
            .consolidated());

        // 会话 B 的水位**必须仍是「从未整编过」** ⇒ ⇒ B 照样能整编
        assert_eq!(
            watermark
                .last_consolidated_sec("session-b")
                .expect("watermark b"),
            None,
            "会话 B 的水位不该被会话 A 影响"
        );
        let b = vec![episode("b-1", 1_500, "user", "fixed: B 素材")];
        assert!(
            driver
                .run("session-b", &b, 2_100, None)
                .expect("run b")
                .consolidated(),
            "会话 B 应能独立整编"
        );

        // A 的水位是 2_000，B 的是 2_100 ⇒ ⇒ 互不覆盖
        assert_eq!(
            watermark.last_consolidated_sec("session-a").expect("a"),
            Some(2_000)
        );
        assert_eq!(
            watermark.last_consolidated_sec("session-b").expect("b"),
            Some(2_100)
        );
    }

    #[test]
    fn file_watermark_round_trips_and_missing_file_means_never_run() {
        let dir =
            std::env::temp_dir().join(format!("consolidation-watermark-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join("watermark.json");
        let _ = std::fs::remove_file(&path);

        let watermark = FileConsolidationWatermark::new(&path);
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("missing file"),
            None
        );
        watermark.mark_consolidated_sec("s1", 1_234).expect("write");
        assert_eq!(
            watermark.last_consolidated_sec("s1").expect("read"),
            Some(1_234)
        );

        // 另一个实例读同一个文件 ⇒ 进程外持久化成立
        let reopened = FileConsolidationWatermark::new(&path);
        assert_eq!(
            reopened.last_consolidated_sec("s1").expect("reopen"),
            Some(1_234)
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
