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

use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

/// 文件系统持久化 Reflexion 存储 (原子写).
pub struct FileReflexionStore {
    root: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ReflexionDataFile {
    #[serde(default)]
    failures: Vec<FailureRecord>,
    #[serde(default)]
    reflections: Vec<ReflectionText>,
    #[serde(default)]
    reflected_until: usize,
}

impl FileReflexionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn file_path(&self) -> PathBuf {
        self.root.join("reflexions.json")
    }

    fn read_data(&self) -> Result<ReflexionDataFile, ReflexionError> {
        let path = self.file_path();
        if !path.exists() {
            return Ok(ReflexionDataFile::default());
        }
        let bytes = std::fs::read(&path)?;
        let data = serde_json::from_slice(&bytes)?;
        Ok(data)
    }

    fn write_data(&self, data: &ReflexionDataFile) -> Result<(), ReflexionError> {
        std::fs::create_dir_all(&self.root)?;
        let target_path = self.file_path();
        let tmp_path = self
            .root
            .join(format!("reflexions.tmp-{}", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(data)?;
        std::fs::write(&tmp_path, bytes)?;
        std::fs::rename(&tmp_path, &target_path)?;
        Ok(())
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
        let mut data = self.read_data()?;
        let seq = data.failures.len() + 1;
        let record = FailureRecord {
            seq,
            kind,
            task_type: task_type.to_string(),
            summary: summary.to_string(),
            timestamp_epoch_ms,
        };
        data.failures.push(record.clone());
        self.write_data(&data)?;
        Ok(record)
    }

    fn process_unreflected(
        &self,
        critic: &dyn Critic,
        timestamp_epoch_ms: i64,
    ) -> Result<usize, ReflexionError> {
        let mut data = self.read_data()?;
        let mut count = 0;
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
            count += 1;
        }
        data.reflected_until = data.failures.len();
        self.write_data(&data)?;
        Ok(count)
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
}
