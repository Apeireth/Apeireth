//! `apeireth-memory::reflexion` — 口头强化反思闭环 (Reflexion 式失败轨迹与重试注入).
//!
//! **设计哲学 (失败沉淀与经验喂回)**:
//! - **① 失败轨迹采集 (`record_failure`)**: 结构化登记三类失败（决策被拒 / 验证未过 / 经验不匹配）；
//! - **② CRITIC 反思生成 (`Critic` Trait + `RuleCritic`)**: 将失败原因提炼为指导性反思教训；
//! - **③ 反思记忆沉淀**: 结构化序列落盘，记录对应 task_type 标签与单调递增 seq；
//! - **④ 同类任务重试注入 (`retry_injection`)**: 在任务重试或同类任务发起时，按任务标签精准召回反思并在字符预算内注入上下文，超限时诚实追加 `TRUNCATION_MARK`。
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决 Agent 在多步长程执行中反复犯相同错误的问题，提供确定性失败喂回
//! 2. 系统: 位于 `engine/memory`，提供抽象的 `ReflexionStore` Trait 与内存/文件实现
//! 3. 架构: 强类型数据模型，0 unsafe, 0 外部 C 扩展

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 注入块截断标记.
pub const TRUNCATION_MARK: &str = "…(已截断)";

/// 三类失败事件.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// 决策被拒绝 (方案/提议被否决)
    DecisionRejected,
    /// 验证失败 (测试/断言不过)
    ValidationFailed,
    /// 经验失败 (复用旧经验不适配)
    ExperienceFailed,
}

impl FailureKind {
    /// 稳定标签.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::DecisionRejected => "decision_rejected",
            Self::ValidationFailed => "validation_failed",
            Self::ExperienceFailed => "experience_failed",
        }
    }
}

/// 一条失败轨迹记录.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRecord {
    /// 单调递增序列号
    pub seq: usize,
    /// 失败类型
    pub kind: FailureKind,
    /// 任务类型标签 (如 "deploy", "refactor", "database_migration")
    pub task_type: String,
    /// 上下文摘要
    pub summary: String,
    /// 记录时间戳 (毫秒)
    pub timestamp_epoch_ms: i64,
}

/// 一条反思记忆.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionText {
    /// 对应的来源失败记录 seq
    pub seq: usize,
    /// 任务类型标签
    pub task_type: String,
    /// 反思正文
    pub text: String,
    /// 反思生成时间戳 (毫秒)
    pub timestamp_epoch_ms: i64,
}

/// Reflexion 错误.
#[derive(Debug, Error)]
pub enum ReflexionError {
    #[error("内容或标签为空")]
    EmptyContent,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("存储锁中毒")]
    LockPoisoned,
    #[error("reflexion 历史上限必须大于零")]
    InvalidHistoryCap,
    #[error("reflexion 持久化存储正被另一个写入者占用: {path}")]
    StoreBusy { path: PathBuf },
    #[error("reflexion 存储状态损坏: failure seq 必须严格递增且非零 (previous: {previous:?}, current: {current})")]
    CorruptedSequence {
        previous: Option<usize>,
        current: usize,
    },
    #[error(
        "reflexion 存储状态损坏: next_seq ({next_seq}) 必须大于最后一个 failure seq ({last_seq})"
    )]
    CorruptedNextSequence { next_seq: usize, last_seq: usize },
    #[error("reflexion 序列号已耗尽")]
    SequenceExhausted,
    /// 持久化状态损坏: 反思游标 `reflected_until` 超过失败记录总数,
    /// 说明落盘文件被外部篡改或截断损坏.
    ///
    /// **策略 (P1 硬化)**: 显式返回类型化错误, 不 panic, 不静默按有效状态
    /// 继续, 也不自动"修复"游标 — 损坏状态的处置 (修复/重置/废弃) 是
    /// 运维侧的显式决策.
    #[error("reflexion 存储状态损坏: reflected_until ({cursor}) 超过失败记录总数 ({len})")]
    CorruptedCursor { cursor: usize, len: usize },
}

/// CRITIC: 失败轨迹 → 反思文本 Trait.
pub trait Critic: Send + Sync {
    /// 针对失败轨迹生成反思建议.
    fn reflect(&self, failure: &FailureRecord) -> String;
}

/// 纯确定性规则版 Critic (0 LLM 依赖).
#[derive(Debug, Default)]
pub struct RuleCritic;

impl Critic for RuleCritic {
    fn reflect(&self, failure: &FailureRecord) -> String {
        match failure.kind {
            FailureKind::DecisionRejected => format!(
                "【决策受挫反思 · {}】方案被否决。教训：重新审视约束边界与权限，避免单方面冒进（原因：{}）。",
                failure.task_type, failure.summary
            ),
            FailureKind::ValidationFailed => format!(
                "【验证未过反思 · {}】测试/断言失败。教训：检查前后置前置条件与边界值，严格本地验证（原因：{}）。",
                failure.task_type, failure.summary
            ),
            FailureKind::ExperienceFailed => format!(
                "【经验不匹配反思 · {}】旧经验复用失效。教训：识别当前环境与历史场景的本质差异，勿盲目套用（原因：{}）。",
                failure.task_type, failure.summary
            ),
        }
    }
}

/// 纯函数：根据反思列表与任务标签构建带预算控制的重试注入块.
pub fn render_retry_injection(
    reflections: &[ReflectionText],
    task_type: &str,
    budget_chars: usize,
) -> Option<String> {
    let kw = task_type.trim().to_lowercase();
    if kw.is_empty() || budget_chars == 0 {
        return None;
    }

    // 评分机制：精确匹配 (2分) > 子串/前缀匹配 (1分)
    let mut scored: Vec<(u8, &ReflectionText)> = reflections
        .iter()
        .filter_map(|r| {
            let r_kw = r.task_type.trim().to_lowercase();
            if r_kw == kw {
                Some((2, r))
            } else if r_kw.contains(&kw) || kw.contains(&r_kw) {
                Some((1, r))
            } else {
                None
            }
        })
        .collect();

    if scored.is_empty() {
        return None;
    }

    // 稳定降序：分数高的优先，同分则序号大的优先（最新的反思优先进入上下文）
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.seq.cmp(&a.1.seq)));

    let header = format!("【历史失败反思备忘 · {}】", task_type.trim());
    let header_chars = header.chars().count();
    if header_chars > budget_chars {
        return None;
    }

    let mut lines = vec![header];
    let mut current_chars = header_chars;
    let mut truncated = false;

    for (_, r) in scored {
        let item = format!("- [教训 #{}] {}", r.seq, r.text);
        let item_chars = item.chars().count();
        // +1 用于 '\n' 换行符
        if current_chars + item_chars + 1 > budget_chars {
            truncated = true;
            break;
        }
        lines.push(item);
        current_chars += item_chars + 1;
    }

    if truncated {
        let mark_chars = TRUNCATION_MARK.chars().count();
        // 如果加入 TRUNCATION_MARK 后超出字符预算，回溯弹出末尾项直到完全放得下
        while lines.len() > 1 && current_chars + mark_chars + 1 > budget_chars {
            if let Some(popped) = lines.pop() {
                current_chars -= popped.chars().count() + 1;
            }
        }
        if current_chars + mark_chars + 1 <= budget_chars {
            lines.push(TRUNCATION_MARK.to_string());
        }
    }

    if lines.len() <= 1 {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Reflexion 存储与检索 Trait.
pub trait ReflexionStore: Send + Sync {
    /// 登记一条失败轨迹.
    fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
    ) -> Result<FailureRecord, ReflexionError>;

    /// 处理尚未反思的失败轨迹并生成反思记忆.
    fn process_unreflected(
        &self,
        critic: &dyn Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError>;

    /// 列出所有失败记录.
    fn list_failures(&self) -> Result<Vec<FailureRecord>, ReflexionError>;

    /// 列出所有已沉淀的反思记忆.
    fn list_reflections(&self) -> Result<Vec<ReflectionText>, ReflexionError>;

    /// 为指定任务类型的重试生成上下文注入块.
    fn retry_injection(
        &self,
        task_type: &str,
        budget_chars: usize,
    ) -> Result<Option<String>, ReflexionError>;
}

/// 内存版 Reflexion 存储 (供测试与嵌入场景).
#[derive(Debug, Default)]
pub struct InMemoryReflexionStore {
    failures: Mutex<Vec<FailureRecord>>,
    reflections: Mutex<Vec<ReflectionText>>,
    reflected_until: Mutex<usize>,
}

impl InMemoryReflexionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ReflexionStore for InMemoryReflexionStore {
    fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
    ) -> Result<FailureRecord, ReflexionError> {
        if task_type.trim().is_empty() || summary.trim().is_empty() {
            return Err(ReflexionError::EmptyContent);
        }
        let mut failures = self
            .failures
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        let seq = failures.len() + 1;
        let record = FailureRecord {
            seq,
            kind,
            task_type: task_type.to_string(),
            summary: summary.to_string(),
            timestamp_epoch_ms,
        };
        failures.push(record.clone());
        Ok(record)
    }

    fn process_unreflected(
        &self,
        critic: &dyn Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError> {
        let failures = self
            .failures
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        let mut reflections = self
            .reflections
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        let mut reflected_until = self
            .reflected_until
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;

        let mut count = 0;
        for failure in failures.iter().skip(*reflected_until) {
            let text = critic.reflect(failure);
            reflections.push(ReflectionText {
                seq: failure.seq,
                task_type: failure.task_type.clone(),
                text,
                timestamp_epoch_ms,
            });
            count += 1;
        }
        *reflected_until = failures.len();
        Ok(count)
    }

    fn list_failures(&self) -> Result<Vec<FailureRecord>, ReflexionError> {
        let failures = self
            .failures
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        Ok(failures.clone())
    }

    fn list_reflections(&self) -> Result<Vec<ReflectionText>, ReflexionError> {
        let reflections = self
            .reflections
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        Ok(reflections.clone())
    }

    fn retry_injection(
        &self,
        task_type: &str,
        budget_chars: usize,
    ) -> Result<Option<String>, ReflexionError> {
        let reflections = self
            .reflections
            .lock()
            .map_err(|_| ReflexionError::LockPoisoned)?;
        Ok(render_retry_injection(
            &reflections,
            task_type,
            budget_chars,
        ))
    }
}

/// 文件持久化失败轨迹的默认上限。
///
/// 上限仅适用于 [`FileReflexionStore`]；内存实现是测试/嵌入用的非持久化
/// 存储，不对进程内生命周期施加此持久化配额。超过上限时保留最新记录，
/// 并同步移除其已淘汰记录的反思文本。
pub const DEFAULT_FILE_HISTORY_CAP: usize = 256;

/// 文件系统持久化 Reflexion 存储。
///
/// 所有变更操作先通过同一根目录中的 `reflexions.lock` 执行原子文件创建
/// 来取得跨线程/进程的排他所有权，再进行读取、修改和原子替换写入。因此，
/// 同一根目录的两个 [`FileReflexionStore`] 实例不会静默丢失彼此的更新。
///
/// **崩溃语义：** 锁文件在正常返回（包括错误返回）时由 RAII 清理。若进程
/// 在持锁期间异常终止，锁文件可能保留；后续写入会在有限等待后返回
/// [`ReflexionError::StoreBusy`]，而不会猜测性删除可能仍属于活跃写入者的锁。
/// 运维人员确认没有活跃写入者后可以显式移除该锁文件。
pub struct FileReflexionStore {
    root: PathBuf,
    max_history: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ReflexionDataFile {
    #[serde(default)]
    failures: Vec<FailureRecord>,
    #[serde(default)]
    reflections: Vec<ReflectionText>,
    #[serde(default)]
    reflected_until: usize,
    /// 下一个可分配的序列号。旧版文件没有该字段时为 0，并会从现有记录
    /// 推导出安全的下一个值后在下一次成功变更中持久化。
    #[serde(default)]
    next_seq: usize,
}

impl ReflexionDataFile {
    fn validate_and_normalize(&mut self) -> Result<(), ReflexionError> {
        // P1 硬化仍然在任何数据消费之前执行：绝不让损坏的游标进入切片操作。
        if self.reflected_until > self.failures.len() {
            return Err(ReflexionError::CorruptedCursor {
                cursor: self.reflected_until,
                len: self.failures.len(),
            });
        }

        let mut previous = None;
        for failure in &self.failures {
            if failure.seq == 0 || previous.is_some_and(|seq| failure.seq <= seq) {
                return Err(ReflexionError::CorruptedSequence {
                    previous,
                    current: failure.seq,
                });
            }
            previous = Some(failure.seq);
        }

        let minimum_next = match previous {
            Some(last_seq) => last_seq
                .checked_add(1)
                .ok_or(ReflexionError::SequenceExhausted)?,
            None => 1,
        };
        if self.next_seq == 0 {
            // Backwards-compatible normalization for state files written before
            // `next_seq` existed. New writes never persist zero.
            self.next_seq = minimum_next;
        } else if self.next_seq < minimum_next {
            return Err(ReflexionError::CorruptedNextSequence {
                next_seq: self.next_seq,
                last_seq: previous.unwrap_or_default(),
            });
        }
        Ok(())
    }

    fn reserve_next_seq(&mut self) -> Result<usize, ReflexionError> {
        let seq = self.next_seq;
        self.next_seq = seq
            .checked_add(1)
            .ok_or(ReflexionError::SequenceExhausted)?;
        Ok(seq)
    }
}

/// A filesystem-level exclusive mutation claim.
///
/// `create_new(true)` is an atomic create operation on the target filesystem,
/// so this is intentionally not a process-local mutex. Holding a path rather
/// than an open descriptor also lets the guard delete it portably on Windows.
struct MutationLock {
    path: PathBuf,
}

impl MutationLock {
    const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
    const RETRY_DELAY: Duration = Duration::from_millis(1);

    fn acquire(root: &Path) -> Result<Self, ReflexionError> {
        std::fs::create_dir_all(root)?;
        let path = root.join("reflexions.lock");
        let deadline = Instant::now() + Self::WAIT_TIMEOUT;

        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        return Err(ReflexionError::StoreBusy { path });
                    }
                    // Bounded backoff avoids a hot spin while allowing a concurrent
                    // writer to finish. Tests synchronize with barriers/channels,
                    // not arbitrary sleeps.
                    std::thread::sleep(Self::RETRY_DELAY);
                }
                Err(error) => return Err(ReflexionError::Io(error)),
            }
        }
    }
}

impl Drop for MutationLock {
    fn drop(&mut self) {
        // This must not replace the original operation's result. A leftover lock
        // remains fail-closed and is surfaced as StoreBusy on the next mutation.
        let _ = std::fs::remove_file(&self.path);
    }
}

impl FileReflexionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_history: DEFAULT_FILE_HISTORY_CAP,
        }
    }

    /// Creates a store with an explicit persistent-history cap.
    pub fn with_history_cap(
        root: impl Into<PathBuf>,
        max_history: usize,
    ) -> Result<Self, ReflexionError> {
        if max_history == 0 {
            return Err(ReflexionError::InvalidHistoryCap);
        }
        Ok(Self {
            root: root.into(),
            max_history,
        })
    }

    fn file_path(&self) -> PathBuf {
        self.root.join("reflexions.json")
    }

    fn read_data(&self) -> Result<ReflexionDataFile, ReflexionError> {
        let path = self.file_path();
        if !path.exists() {
            let mut data = ReflexionDataFile::default();
            data.validate_and_normalize()?;
            return Ok(data);
        }
        let bytes = std::fs::read(&path)?;
        let mut data: ReflexionDataFile = serde_json::from_slice(&bytes)?;
        data.validate_and_normalize()?;
        Ok(data)
    }

    fn trim_history(&self, data: &mut ReflexionDataFile) {
        if data.failures.len() <= self.max_history {
            return;
        }

        let removed = data.failures.len() - self.max_history;
        data.failures.drain(..removed);
        // `reflected_until` is a count over the retained prefix. If already
        // reflected records were evicted, subtract them; if unreflected records
        // were evicted, no retained prefix may be claimed as reflected.
        data.reflected_until = data
            .reflected_until
            .saturating_sub(removed)
            .min(data.failures.len());

        let retained_sequences: Vec<usize> = data.failures.iter().map(|f| f.seq).collect();
        data.reflections
            .retain(|reflection| retained_sequences.binary_search(&reflection.seq).is_ok());
    }

    fn write_data(&self, data: &ReflexionDataFile) -> Result<(), ReflexionError> {
        std::fs::create_dir_all(&self.root)?;
        let target_path = self.file_path();
        let tmp_path = self
            .root
            .join(format!("reflexions.tmp-{}", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(data)?;

        let result = (|| -> Result<(), ReflexionError> {
            let mut tmp_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)?;
            tmp_file.write_all(&bytes)?;
            tmp_file.sync_all()?;
            drop(tmp_file);
            #[cfg(windows)]
            {
                if target_path.exists() {
                    let _ = std::fs::remove_file(&target_path);
                }
            }
            std::fs::rename(&tmp_path, &target_path)?;
            Ok(())
        })();
        if result.is_err() {
            // Best-effort cleanup only. The original error is more actionable;
            // a leftover unique temp file cannot affect future reads or claims.
            let _ = std::fs::remove_file(&tmp_path);
        }
        result
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut ReflexionDataFile) -> Result<T, ReflexionError>,
    ) -> Result<T, ReflexionError> {
        let _lock = MutationLock::acquire(&self.root)?;
        let mut data = self.read_data()?;
        // An old, oversized file is bounded before any new mutation observes it.
        self.trim_history(&mut data);
        let result = operation(&mut data)?;
        self.trim_history(&mut data);
        data.validate_and_normalize()?;
        self.write_data(&data)?;
        Ok(result)
    }
}

impl ReflexionStore for FileReflexionStore {
    fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
        timestamp_epoch_ms: i64,
    ) -> Result<FailureRecord, ReflexionError> {
        if task_type.trim().is_empty() || summary.trim().is_empty() {
            return Err(ReflexionError::EmptyContent);
        }
        self.mutate(|data| {
            let record = FailureRecord {
                seq: data.reserve_next_seq()?,
                kind,
                task_type: task_type.to_string(),
                summary: summary.to_string(),
                timestamp_epoch_ms,
            };
            data.failures.push(record.clone());
            Ok(record)
        })
    }

    fn process_unreflected(
        &self,
        critic: &dyn Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError> {
        self.mutate(|data| {
            // Keep the explicit P1 check adjacent to the slice operation even
            // though read_data has already validated the persisted state.
            if data.reflected_until > data.failures.len() {
                return Err(ReflexionError::CorruptedCursor {
                    cursor: data.reflected_until,
                    len: data.failures.len(),
                });
            }
            let start = data.reflected_until;
            let failures_slice = data.failures[start..].to_vec();

            for failure in &failures_slice {
                let text = critic.reflect(failure);
                data.reflections.push(ReflectionText {
                    seq: failure.seq,
                    task_type: failure.task_type.clone(),
                    text,
                    timestamp_epoch_ms,
                });
            }
            data.reflected_until = data.failures.len();
            Ok(failures_slice.len())
        })
    }

    fn list_failures(&self) -> Result<Vec<FailureRecord>, ReflexionError> {
        let data = self.read_data()?;
        Ok(data.failures)
    }

    fn list_reflections(&self) -> Result<Vec<ReflectionText>, ReflexionError> {
        let data = self.read_data()?;
        Ok(data.reflections)
    }

    fn retry_injection(
        &self,
        task_type: &str,
        budget_chars: usize,
    ) -> Result<Option<String>, ReflexionError> {
        let data = self.read_data()?;
        Ok(render_retry_injection(
            &data.reflections,
            task_type,
            budget_chars,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc, Barrier};
    use std::thread;

    use super::*;

    #[test]
    fn in_memory_reflexion_lifecycle() {
        let store = InMemoryReflexionStore::new();
        let critic = RuleCritic;

        let failure = store
            .record_failure(
                FailureKind::ValidationFailed,
                "deploy",
                "端口 8080 被占用",
                1000,
            )
            .unwrap();
        assert_eq!(failure.seq, 1);

        let count = store.process_unreflected(&critic, 2000).unwrap();
        assert_eq!(count, 1);

        let reflections = store.list_reflections().unwrap();
        assert_eq!(reflections.len(), 1);
        assert!(reflections[0].text.contains("验证未过反思"));

        let injection = store.retry_injection("deploy", 500).unwrap().unwrap();
        assert!(injection.contains("【历史失败反思备忘 · deploy】"));
        assert!(injection.contains("端口 8080 被占用"));
    }

    #[test]
    fn file_store_atomic_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileReflexionStore::new(tmp.path());
        let critic = RuleCritic;

        store
            .record_failure(
                FailureKind::DecisionRejected,
                "refactor",
                "单次 PR 变更超过 2000 行",
                1000,
            )
            .unwrap();

        let processed = store.process_unreflected(&critic, 2000).unwrap();
        assert_eq!(processed, 1);

        let injection = store.retry_injection("refactor", 500).unwrap().unwrap();
        assert!(injection.contains("决策受挫反思"));
    }

    #[test]
    fn ranking_exact_over_substring_and_budget_backtrack() {
        let store = InMemoryReflexionStore::new();
        let critic = RuleCritic;

        // 插入两条：一条是 "deploy-backend"（子串），一条是 "deploy"（精确匹配）
        store
            .record_failure(
                FailureKind::ExperienceFailed,
                "deploy-backend",
                "旧部署脚本失效",
                1000,
            )
            .unwrap();
        store
            .record_failure(FailureKind::ValidationFailed, "deploy", "配置丢失", 2000)
            .unwrap();

        store.process_unreflected(&critic, 3000).unwrap();

        // 检索 "deploy"：精确匹配项 (seq 2) 必须排在 (seq 1) 之前
        let injection = store.retry_injection("deploy", 1000).unwrap().unwrap();
        let pos_exact = injection.find("教训 #2").unwrap();
        let pos_sub = injection.find("教训 #1").unwrap();
        assert!(pos_exact < pos_sub, "精确匹配项应排在前面");
    }

    /// P1 硬化: 损坏游标 (reflected_until > failures.len()) → 类型化错误,
    /// 不 panic, 不静默修复, 且错误中可观测 cursor/len.
    #[test]
    fn file_store_corrupt_cursor_is_rejected_without_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("reflexions.json");
        // 手工写入损坏状态: 游标 10, 但失败记录只有 1 条
        let corrupt = r#"{"failures":[{"seq":1,"kind":"validation_failed","task_type":"deploy","summary":"端口被占","timestamp_epoch_ms":1000}],"reflections":[],"reflected_until":10}"#;
        std::fs::write(&path, corrupt).unwrap();

        let store = FileReflexionStore::new(tmp.path());
        let err = store
            .process_unreflected(&RuleCritic, 2000)
            .expect_err("损坏游标必须显式报错");
        match err {
            ReflexionError::CorruptedCursor { cursor, len } => {
                assert_eq!(cursor, 10);
                assert_eq!(len, 1);
            }
            other => panic!("expected CorruptedCursor, got {other:?}"),
        }
        // 损坏状态保持原样 (不做静默修复): 再次读取仍是损坏的 cursor
        let err_again = store.process_unreflected(&RuleCritic, 3000).unwrap_err();
        assert!(matches!(err_again, ReflexionError::CorruptedCursor { .. }));
    }

    /// P1 硬化: 游标等于总数 → 合法空工作 (Ok(0)).
    #[test]
    fn file_store_cursor_at_len_is_valid_empty_work() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileReflexionStore::new(tmp.path());
        store
            .record_failure(FailureKind::ValidationFailed, "deploy", "配置丢失", 1000)
            .unwrap();
        let first = store.process_unreflected(&RuleCritic, 2000).unwrap();
        assert_eq!(first, 1);
        // 游标 == len: 再次处理 → 0 条待反思, 不报错
        let second = store.process_unreflected(&RuleCritic, 3000).unwrap();
        assert_eq!(second, 0);
    }

    /// P1 硬化: 游标小于总数 → 正常处理剩余条目, 既有排序/预算行为不变.
    #[test]
    fn file_store_cursor_below_len_processes_remaining() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileReflexionStore::new(tmp.path());
        store
            .record_failure(FailureKind::ValidationFailed, "deploy", "配置丢失", 1000)
            .unwrap();
        assert_eq!(store.process_unreflected(&RuleCritic, 2000).unwrap(), 1);
        // 追加 2 条后游标 (1) < len (3) → 仅处理新增 2 条
        store
            .record_failure(FailureKind::DecisionRejected, "deploy", "方案被否", 3000)
            .unwrap();
        store
            .record_failure(FailureKind::ExperienceFailed, "deploy", "旧经验失效", 4000)
            .unwrap();
        let count = store.process_unreflected(&RuleCritic, 5000).unwrap();
        assert_eq!(count, 2);
        let reflections = store.list_reflections().unwrap();
        assert_eq!(reflections.len(), 3);
        // retry_injection 正常
        assert!(store.retry_injection("deploy", 2000).unwrap().is_some());
    }

    #[test]
    fn file_store_history_cap_keeps_cursor_and_reflections_consistent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileReflexionStore::with_history_cap(tmp.path(), 2).unwrap();

        let first = store
            .record_failure(FailureKind::ValidationFailed, "deploy", "first", 1000)
            .unwrap();
        assert_eq!(first.seq, 1);
        assert_eq!(store.process_unreflected(&RuleCritic, 2000).unwrap(), 1);

        let second = store
            .record_failure(FailureKind::DecisionRejected, "deploy", "second", 3000)
            .unwrap();
        let third = store
            .record_failure(FailureKind::ExperienceFailed, "deploy", "third", 4000)
            .unwrap();
        assert_eq!((second.seq, third.seq), (2, 3));

        // The deterministic policy retains the newest two failures. Its cursor
        // is rebased from the old retained prefix, so both retained failures are
        // correctly recognized as still unreflected.
        let failures = store.list_failures().unwrap();
        assert_eq!(
            failures
                .iter()
                .map(|failure| failure.seq)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert_eq!(store.process_unreflected(&RuleCritic, 5000).unwrap(), 2);
        assert_eq!(store.process_unreflected(&RuleCritic, 6000).unwrap(), 0);

        let reflections = store.list_reflections().unwrap();
        assert_eq!(
            reflections
                .iter()
                .map(|reflection| reflection.seq)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );

        // The persisted next-sequence cursor is independent from the retained
        // vector length, so eviction cannot cause a sequence collision.
        let fourth = store
            .record_failure(FailureKind::ValidationFailed, "deploy", "fourth", 7000)
            .unwrap();
        assert_eq!(fourth.seq, 4);
        assert_eq!(
            store
                .list_failures()
                .unwrap()
                .iter()
                .map(|failure| failure.seq)
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn file_store_concurrent_instances_preserve_both_writes_and_unique_sequences() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let start = Arc::new(Barrier::new(3));
        let (tx, rx) = mpsc::channel();

        let mut workers = Vec::new();
        for summary in ["writer-a", "writer-b"] {
            let root = root.clone();
            let start = Arc::clone(&start);
            let tx = tx.clone();
            workers.push(thread::spawn(move || {
                let store = FileReflexionStore::new(root);
                start.wait();
                tx.send(store.record_failure(
                    FailureKind::ValidationFailed,
                    "deploy",
                    summary,
                    1000,
                ))
                .unwrap();
            }));
        }
        drop(tx);

        start.wait();
        let results: Vec<_> = rx.into_iter().collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(results.len(), 2);
        for result in results {
            assert!(
                result.is_ok(),
                "concurrent writer must not be lost: {result:?}"
            );
        }

        let store = FileReflexionStore::new(root);
        let failures = store.list_failures().unwrap();
        assert_eq!(failures.len(), 2);
        assert_eq!(
            failures
                .iter()
                .map(|failure| failure.seq)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        let mut summaries = failures
            .iter()
            .map(|failure| failure.summary.as_str())
            .collect::<Vec<_>>();
        summaries.sort_unstable();
        assert_eq!(summaries, vec!["writer-a", "writer-b"]);
    }

    #[test]
    fn malformed_json_is_typed_non_destructive_and_releases_mutation_claim() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("reflexions.json");
        let malformed = b"{ this is not json";
        std::fs::write(&path, malformed).unwrap();
        let store = FileReflexionStore::new(tmp.path());

        let err = store
            .record_failure(FailureKind::ValidationFailed, "deploy", "bad json", 1000)
            .unwrap_err();
        assert!(matches!(err, ReflexionError::Json(_)));
        assert_eq!(std::fs::read(&path).unwrap(), malformed);
        assert!(
            !tmp.path().join("reflexions.lock").exists(),
            "an error path must release the mutation claim"
        );

        // A subsequent valid state can be mutated immediately; the previous
        // parse failure did not leave a process-local or filesystem deadlock.
        std::fs::write(&path, b"{}").unwrap();
        assert!(store
            .record_failure(FailureKind::ValidationFailed, "deploy", "recovered", 2000)
            .is_ok());
    }

    #[test]
    fn file_store_rejects_zero_history_cap() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            FileReflexionStore::with_history_cap(tmp.path(), 0),
            Err(ReflexionError::InvalidHistoryCap)
        ));
    }
}
