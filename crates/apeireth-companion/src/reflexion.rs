//! E1 口头强化闭环 (Reflexion 式; 进化调研批, 主人拍板"团队能干的直接安排").
//!
//! **痛点**: 反思有周期无喂回 —— 失败轨迹产生反思, 但同类任务重试时不注入.
//! 本模块补上"喂回"半边: 失败轨迹 → CRITIC 反思文本 → 反思记忆 → 同类任务重试注入.
//!
//! **职责链 (四段)**:
//! 1. 失败轨迹采集: [`ReflexionStore::record_failure`] 结构化登记三类失败
//!    (决策拒绝/验证失败/经验失败), 事件源复用已有反思/审计机制
//!    (实接线为 trait 口, 0 装 PASS — 不改 reflection.rs 周期机制本体)
//! 2. CRITIC 反思: [`Critic`] trait 口 (LLM 版预留) + 确定性规则版 [`RuleCritic`]
//!    先行: 失败类型 + 上下文摘要 → 结构化反思模板
//! 3. 反思记忆: 反思文本按任务类型标签落盘 (reflections.json, seq 序确定性)
//! 4. 重试注入: [`ReflexionStore::retry_injection`] 按任务类型相似度
//!    (精确 > 子串) 检索相关反思 → 预算内注入块
//!
//! **0 装 PASS 标注 (诚实)**:
//! - LLM 版 CRITIC 未接 (trait 口已留, 现仅确定性规则版)
//! - 失败事件实接线 (reflection/审计事件 → record_failure) 未接, 留公开入口
//! - 注入块消费侧 (任务重试上下文渲染) 未接线, 留方法口

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 注入块截断标记 (预算不足时尾行提示, 诚实声明"有省略").
pub const TRUNCATION_MARK: &str = "…(已截断)";

/// 三类失败事件 (与调研原文对齐).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureKind {
    /// 决策被拒绝 (方案/提议被否)
    DecisionRejected,
    /// 验证失败 (验收/断言不过)
    ValidationFailed,
    /// 经验失败 (复用旧经验不适配)
    ExperienceFailed,
}

impl FailureKind {
    /// 稳定标签 (落盘与检索用, 勿改既有值).
    pub fn tag(&self) -> &'static str {
        match self {
            FailureKind::DecisionRejected => "decision_rejected",
            FailureKind::ValidationFailed => "validation_failed",
            FailureKind::ExperienceFailed => "experience_failed",
        }
    }
}

/// 一条失败轨迹记录 (seq 由存储分配, 到达序确定性).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRecord {
    pub seq: usize,
    pub kind: FailureKind,
    /// 任务类型标签 (检索键, 如 deploy/refactor/investigate)
    pub task_type: String,
    /// 上下文摘要 (事件源提供的简述)
    pub summary: String,
}

/// 一条反思记忆 (seq = 来源失败记录 seq; task_type 标签随源).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionText {
    pub seq: usize,
    pub task_type: String,
    pub text: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FailureFile {
    #[serde(default)]
    failures: Vec<FailureRecord>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ReflectionFile {
    #[serde(default)]
    reflections: Vec<ReflectionText>,
    /// 已反思到的最大失败 seq (0 = 尚未反思任何; 存储从 1 计, 记录 seq 从 0 计)
    #[serde(default)]
    reflected_until: usize,
}

/// E1 错误 (非法输入显式拒绝, 不 panic).
#[derive(Debug, Error)]
pub enum ReflexionError {
    #[error("内容为空")]
    EmptyContent,
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// CRITIC: 失败轨迹 → 反思文本.
///
/// **LLM 版预留口 (0 装)**: 现仅确定性规则版 [`RuleCritic`];
/// 未来 LLM 实现此 trait 即可插拔, 调用方不变.
pub trait Critic: Send + Sync {
    fn reflect(&self, f: &FailureRecord) -> String;
}

/// 确定性规则版 CRITIC (先行): 失败类型 + 上下文摘要 → 结构化反思模板.
/// 纯函数: 同输入同输出, 0 随机 0 时间依赖.
pub struct RuleCritic;

impl Critic for RuleCritic {
    fn reflect(&self, f: &FailureRecord) -> String {
        let (label, lesson, retry) = match f.kind {
            FailureKind::DecisionRejected => (
                "决策被拒",
                "决策依据不足或约束不清",
                "重试前先逐项列明约束并与需求方确认, 备好替代方案",
            ),
            FailureKind::ValidationFailed => (
                "验证失败",
                "验收标准未达成",
                "分解验收项逐条复现, 先定位再修复, 修复后复验再提交",
            ),
            FailureKind::ExperienceFailed => (
                "经验失败",
                "既有经验不适配当前情境",
                "核对经验依赖的环境与前提是否变化, 必要时降级为第一性原理处理",
            ),
        };
        format!(
            "[反思·{}] task_type={} kind={} | 事实: {} | 教训: {} | 重试策略: {}",
            label,
            f.task_type,
            f.kind.tag(),
            f.summary,
            lesson,
            retry
        )
    }
}

/// 反思闭环存储 (root 注入; 失败与反思分文件, seq 序确定性).
pub struct ReflexionStore {
    root: PathBuf,
    critic: Arc<dyn Critic>,
}

impl ReflexionStore {
    /// root = 反思根目录 (如 `<memory_path>/reflexion`); critic 注入 (可换 LLM 版).
    pub fn new(root: impl Into<PathBuf>, critic: Arc<dyn Critic>) -> Self {
        Self { root: root.into(), critic }
    }

    /// 默认确定性规则版 CRITIC 便捷构造.
    pub fn with_rule_critic(root: impl Into<PathBuf>) -> Self {
        Self::new(root, Arc::new(RuleCritic))
    }

    fn failures_path(&self) -> PathBuf {
        self.root.join("failures.json")
    }

    fn reflections_path(&self) -> PathBuf {
        self.root.join("reflections.json")
    }

    /// 登记一条失败轨迹; 返回分配 seq. 空 task_type/summary 显式拒绝.
    pub fn record_failure(
        &self,
        kind: FailureKind,
        task_type: &str,
        summary: &str,
    ) -> Result<usize, ReflexionError> {
        if task_type.trim().is_empty() || summary.trim().is_empty() {
            return Err(ReflexionError::EmptyContent);
        }
        let mut file = read_json::<FailureFile>(&self.failures_path());
        let seq = file.failures.len();
        file.failures.push(FailureRecord {
            seq,
            kind,
            task_type: task_type.trim().to_string(),
            summary: summary.trim().to_string(),
        });
        write_json(&self.failures_path(), &file)?;
        Ok(seq)
    }

    /// 全部失败记录 (到达序). IO 失败 → 空 (诚实降级).
    pub fn failures(&self) -> Vec<FailureRecord> {
        read_json::<FailureFile>(&self.failures_path()).failures
    }

    /// CRITIC 步: 对所有尚未反思的失败记录生成反思文本入记忆库.
    /// 返回本次新反思条数 (0 = 无未反思记录). 确定性: seq 升序处理.
    pub fn critic_step(&self) -> Result<usize, ReflexionError> {
        let failures = self.failures();
        let mut rfile = read_json::<ReflectionFile>(&self.reflections_path());
        let until = rfile.reflected_until;
        let mut added = 0usize;
        for f in failures.iter().skip_while(|f| (f.seq + 1) <= until) {
            let text = self.critic.reflect(f);
            rfile.reflections.push(ReflectionText {
                seq: f.seq,
                task_type: f.task_type.clone(),
                text,
            });
            rfile.reflected_until = f.seq + 1;
            added += 1;
        }
        if added > 0 {
            write_json(&self.reflections_path(), &rfile)?;
        }
        Ok(added)
    }

    /// 全部反思记忆 (存储序). IO 失败 → 空.
    pub fn reflections(&self) -> Vec<ReflectionText> {
        read_json::<ReflectionFile>(&self.reflections_path()).reflections
    }

    /// 同类任务重试注入块: 按 task_type 相似度 (精确 2 > 子串 1) 检索,
    /// 同分取最新 (seq 大者优先), 字符预算内截断.
    /// 无匹配/预算放不下头部+至少一条 → 空串 (诚实: 不注入半残块).
    pub fn retry_injection(&self, task_type: &str, budget_chars: usize) -> String {
        if task_type.trim().is_empty() {
            return String::new();
        }
        let rs = self.reflections();
        let mut hits: Vec<(u8, &ReflectionText)> = rs
            .iter()
            .filter_map(|r| {
                let s = match_score(task_type, &r.task_type);
                (s > 0).then_some((s, r))
            })
            .collect();
        // 分数降序, 同分 seq 降序 (最新优先) — 全确定性
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.seq.cmp(&a.1.seq)));
        if hits.is_empty() {
            return String::new();
        }
        let header = format!("【反思强化】同类任务经验 task_type={}", task_type);
        // 行格式 "\n· {text}" → 固定 3 字符 + text
        let mark_len = 1 + TRUNCATION_MARK.chars().count();
        let mut kept: Vec<&ReflectionText> = Vec::new();
        let mut used = header.chars().count();
        let mut truncated = false;
        for (_, r) in hits {
            let line_len = 3 + r.text.chars().count();
            if used + line_len > budget_chars {
                truncated = !kept.is_empty();
                break;
            }
            used += line_len;
            kept.push(r);
        }
        while truncated && used + mark_len > budget_chars && !kept.is_empty() {
            let r = kept.pop().expect("kept 非空");
            used -= 3 + r.text.chars().count();
            if kept.is_empty() {
                return String::new();
            }
        }
        if kept.is_empty() {
            return String::new();
        }
        let mut out = header;
        for r in kept {
            out.push_str(&format!("\n· {}", r.text));
        }
        if truncated {
            out.push_str(&format!("\n{}", TRUNCATION_MARK));
        }
        out
    }
}

/// 任务类型相似度 (确定性): 精确 = 2, 任一方向子串 = 1, 否则 0.
fn match_score(query: &str, tag: &str) -> u8 {
    if query == tag {
        2
    } else if !query.is_empty() && !tag.is_empty() && (tag.contains(query) || query.contains(tag)) {
        1
    } else {
        0
    }
}

/// 读 JSON 文件; 缺失/损坏 → 默认值 (诚实降级, 不 panic).
fn read_json<T: Default + serde::de::DeserializeOwned>(path: &std::path::Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// 写 JSON 文件 (自动建父目录).
fn write_json<T: Serialize>(path: &std::path::Path, v: &T) -> Result<(), ReflexionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(v)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn tmp_root() -> PathBuf {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "apeireth-reflexion-test-{}-{}-{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn record_failure_registers_three_kinds_and_persists() {
        let root = tmp_root();
        let store = ReflexionStore::with_rule_critic(&root);
        let s0 = store.record_failure(FailureKind::DecisionRejected, "deploy", "方案被否: 未给回滚预案").unwrap();
        let s1 = store.record_failure(FailureKind::ValidationFailed, "deploy", "验收第 3 项断言失败").unwrap();
        let s2 = store.record_failure(FailureKind::ExperienceFailed, "refactor", "旧迁移经验不适用新 schema").unwrap();
        assert_eq!((s0, s1, s2), (0, 1, 2), "seq 按到达序分配");
        // 新实例重读 → 持久化成立
        let store2 = ReflexionStore::with_rule_critic(&root);
        let fs = store2.failures();
        assert_eq!(fs.len(), 3);
        assert_eq!(fs[1].kind, FailureKind::ValidationFailed);
        assert_eq!(fs[2].task_type, "refactor");
        // 空输入显式拒绝
        assert!(store.record_failure(FailureKind::ValidationFailed, "", "x").is_err());
        assert!(store.record_failure(FailureKind::ValidationFailed, "t", "  ").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn critic_step_generates_structured_reflections_once() {
        let root = tmp_root();
        let store = ReflexionStore::with_rule_critic(&root);
        store.record_failure(FailureKind::ValidationFailed, "deploy", "测试红: 超时断言").unwrap();
        store.record_failure(FailureKind::ExperienceFailed, "deploy", "缓存经验不适配").unwrap();
        assert_eq!(store.critic_step().unwrap(), 2, "首次反思两条");
        assert_eq!(store.critic_step().unwrap(), 0, "无未反思记录 → 0 (幂等)");
        let rs = store.reflections();
        assert_eq!(rs.len(), 2);
        assert!(rs[0].text.contains("验证失败"), "反思含类型标签: {}", rs[0].text);
        assert!(rs[0].text.contains("task_type=deploy"), "反思含任务类型: {}", rs[0].text);
        assert!(rs[0].text.contains("重试策略"), "反思含重试策略段");
        // 新增失败 → 增量反思
        store.record_failure(FailureKind::DecisionRejected, "deploy", "被否: 依据不足").unwrap();
        assert_eq!(store.critic_step().unwrap(), 1, "增量反思一条");
        assert_eq!(store.reflections().len(), 3);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn retry_injection_prefers_exact_and_truncates_by_budget() {
        let root = tmp_root();
        let store = ReflexionStore::with_rule_critic(&root);
        store.record_failure(FailureKind::ValidationFailed, "deploy-flow", "子串相关经验").unwrap();
        store.record_failure(FailureKind::DecisionRejected, "deploy", "精确相关经验").unwrap();
        store.record_failure(FailureKind::ExperienceFailed, "cooking", "无关经验").unwrap();
        store.critic_step().unwrap();
        let full = store.retry_injection("deploy", 2000);
        assert!(full.starts_with("【反思强化】"), "块头: {full}");
        assert!(full.contains("精确相关经验"), "应含精确匹配: {full}");
        assert!(full.contains("子串相关经验"), "应含子串匹配: {full}");
        assert!(!full.contains("无关经验"), "不应含无关任务类型: {full}");
        let exact_pos = full.find("精确相关经验").unwrap();
        let sub_pos = full.find("子串相关经验").unwrap();
        assert!(exact_pos < sub_pos, "精确匹配应排子串之前: {full}");
        assert!(!full.contains(TRUNCATION_MARK), "预算充足不截断");
        // 预算截断
        let cut = store.retry_injection("deploy", 240);
        assert!(cut.contains(TRUNCATION_MARK), "预算不足应有截断标记: {cut}");
        assert!(cut.chars().count() <= 240, "预算硬上限: {}", cut.chars().count());
        // 预算过小 → 空串诚实降级
        assert!(store.retry_injection("deploy", 20).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_store_all_paths_honest_empty() {
        let root = tmp_root();
        let store = ReflexionStore::with_rule_critic(&root);
        assert!(store.failures().is_empty());
        assert!(store.reflections().is_empty());
        assert_eq!(store.critic_step().unwrap(), 0, "空失败 → 反思 0 条");
        assert!(store.retry_injection("deploy", 500).is_empty(), "空记忆 → 注入空串");
        assert!(store.retry_injection("", 500).is_empty(), "空 task_type → 空串");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        // RuleCritic 纯函数复测
        let f = FailureRecord {
            seq: 0,
            kind: FailureKind::ValidationFailed,
            task_type: "deploy".into(),
            summary: "确定性复测样例".into(),
        };
        let a = RuleCritic.reflect(&f);
        for _ in 0..5 {
            assert_eq!(RuleCritic.reflect(&f), a, "CRITIC 同输入同输出");
        }
        // 存储级复测: 两实例同操作序列 → 注入块一致
        let (r1, r2) = (tmp_root(), tmp_root());
        for root in [&r1, &r2] {
            let s = ReflexionStore::with_rule_critic(root);
            s.record_failure(FailureKind::DecisionRejected, "deploy", "输入甲").unwrap();
            s.record_failure(FailureKind::ExperienceFailed, "deploy-x", "输入乙").unwrap();
            s.critic_step().unwrap();
        }
        let s1 = ReflexionStore::with_rule_critic(&r1);
        let s2 = ReflexionStore::with_rule_critic(&r2);
        assert_eq!(
            s1.retry_injection("deploy", 1000),
            s2.retry_injection("deploy", 1000),
            "同操作序列同输出"
        );
        for _ in 0..3 {
            assert_eq!(s1.retry_injection("deploy", 1000), s1.retry_injection("deploy", 1000));
        }
        let _ = std::fs::remove_dir_all(&r1);
        let _ = std::fs::remove_dir_all(&r2);
    }
}
