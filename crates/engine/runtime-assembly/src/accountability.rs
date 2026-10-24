//! 问责闭环 · 第 1 步：影响声明 + 索引。
//!
//! 上位：`docs/04-internal/backend-enhancement/accountability-loop-design-2026-10-23.md` §2.1
//! 与 ADR-0014 §5。触发判据（外部顾问）：**「记录 ≠ 负责」** ——
//! 「若系统知道某记忆被纠正、且知道旧结论曾驱动求职/购买/发信，却只改视图然后宣布
//! 『已纠正』—— **那叫数据纠正，不叫责任闭环**」。
//!
//! # 本模块是什么
//!
//! 一次**外部行动**发生时，留下一条**影响声明**（Impact Statement）：
//! 「我用了哪些事实、依据哪些授权、影响了谁、还能不能撤回、去了哪」。
//!
//! # 纪律
//!
//! - **零真值**：只存**指涉 id**（记忆条目 id、权限 id、目的地名），**不含内容原文**。
//!   与 `RedactionBridge` 的零真值账本同纪律。
//! - **append-only**：坏行读作不存在。
//! - **只声明影响，不代用户行动**：本模块**不自动外发**任何东西
//!   （通知义务 ≠ 自动发信，见设计文档 §7）。
//! - **边界词表闭合**：`Boundary` 是枚举，不增不减不漂移。
//!
//! # 为什么现在做
//!
//! 承诺措辞里有一句必须兑现的话：「**在发现影响时通知你**」。
//! 「发现影响」的前提是**知道哪条事实影响过哪些行动** —— 那就是这份索引。
//! 没有它，纠正旧事实时我们只能说「已纠正」，而不能说「它影响过这三次行动」。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 行动跨越的边界（**闭合词表**：不增不减不漂移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Boundary {
    /// 内容送出到模型服务商（模型上下文出境）。
    Provider,
    /// 内容送到第三方系统（连接器 / 外部 API）。
    ThirdParty,
    /// 用户把内容发给组织（邮件 / 表单 / 消息）。
    UserToOrg,
    /// 从推测走向行动（内部结论驱动了外部行为）。
    InferenceToAction,
}

impl Boundary {
    /// 全部边界（行序 = 规范序）。
    pub const ALL: [Self; 4] = [
        Self::Provider,
        Self::ThirdParty,
        Self::UserToOrg,
        Self::InferenceToAction,
    ];

    /// 稳定标签（报告面用）。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::ThirdParty => "third_party",
            Self::UserToOrg => "user_to_org",
            Self::InferenceToAction => "inference_to_action",
        }
    }

    /// 解析标签。
    pub fn parse(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.label() == label)
    }
}

/// 行动影响了谁。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Affected {
    /// 只影响自己（本人）。
    SelfOnly,
    /// 影响了具名方（**只存指涉 id，不存名字原文**）。
    NamedParty,
    /// 影响对象不明（如群发）。
    Unknown,
}

impl Affected {
    /// 稳定标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::SelfOnly => "self",
            Self::NamedParty => "named_party",
            Self::Unknown => "unknown",
        }
    }
}

/// 一条影响声明（**零真值**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactStatement {
    /// 行动 id（唯一）。
    pub action_id: String,
    /// 发生时刻（epoch 毫秒）。
    pub at_ms: i64,
    /// 跨越的边界。
    pub boundary: Boundary,
    /// 用了哪些记忆条目（**只存 id**）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts_used: Vec<String>,
    /// 哪些是推测（与事实分开记 —— 混在一起就分不清「它以为」和「它知道」）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inferences: Vec<String>,
    /// 哪些权限允许了它（**只存 id**）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<String>,
    /// 影响了谁。
    pub affected: Affected,
    /// 结果是否可逆。
    pub reversible: bool,
    /// 目的地（**不含内容**：如 `provider:anthropic`）。
    pub destination: String,
    /// 后果状态（**第 3 步接**；当前恒 `unknown`，绝不假装知道）。
    #[serde(default = "default_outcome")]
    pub outcome: Outcome,
}

/// 后果状态（闭合词表）。
///
/// `unknown` 是**合法且必要**的一档 —— 「我们不知道结果」必须能被表示，
/// 否则系统只能二选一：假装知道，或不记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// 尚未确认。
    Pending,
    /// 已确认成功。
    Confirmed,
    /// 已确认失败。
    Failed,
    /// **不知道**（显式，不假装）。
    Unknown,
}

impl Outcome {
    /// 稳定标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

fn default_outcome() -> Outcome {
    Outcome::Unknown
}

/// 影响声明账本（append-only JSONL）+ **事实 → 行动**索引。
///
/// 索引是通知义务的基础：纠正一条事实时，能回答「它影响过哪些行动」。
#[derive(Debug, Clone)]
pub struct AccountabilityLedger {
    path: Option<PathBuf>,
}

impl AccountabilityLedger {
    /// 新建（`path` 为 `None` = 不持久）。
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    /// 账本路径。
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// 追加一条影响声明。返回是否落盘成功。
    pub fn append(&self, statement: &ImpactStatement) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        let Ok(line) = serde_json::to_string(statement) else {
            return false;
        };
        append_line(path, &line).is_ok()
    }

    /// 读回全部声明（坏行读作不存在）。
    pub fn statements(&self) -> Vec<ImpactStatement> {
        let Some(path) = &self.path else {
            return Vec::new();
        };
        read_statements(path)
    }

    /// **事实 → 行动**索引：这条事实曾影响过哪些行动。
    ///
    /// 通知义务问的问题就是它。
    pub fn actions_referencing(&self, fact_id: &str) -> Vec<ImpactStatement> {
        self.statements()
            .into_iter()
            .filter(|statement| statement.facts_used.iter().any(|id| id == fact_id))
            .collect()
    }

    /// 索引全貌（fact_id → 行动 id 列表）。
    pub fn index(&self) -> BTreeMap<String, Vec<String>> {
        let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for statement in self.statements() {
            for fact in &statement.facts_used {
                let entry = index.entry(fact.clone()).or_default();
                if !entry.contains(&statement.action_id) {
                    entry.push(statement.action_id.clone());
                }
            }
        }
        index
    }

    /// 计数（自述面读它）。
    pub fn counts(&self) -> AccountabilityCounts {
        let statements = self.statements();
        AccountabilityCounts {
            total: statements.len(),
            irreversible: statements.iter().filter(|s| !s.reversible).count(),
            affecting_others: statements
                .iter()
                .filter(|s| s.affected != Affected::SelfOnly)
                .count(),
            unknown_outcome: statements
                .iter()
                .filter(|s| s.outcome == Outcome::Unknown)
                .count(),
        }
    }
}

/// 账本计数（自述面用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityCounts {
    /// 声明总数。
    pub total: usize,
    /// 不可逆的行动数。
    pub irreversible: usize,
    /// 影响到他人（非 self）的行动数。
    pub affecting_others: usize,
    /// 后果未知的行动数。
    pub unknown_outcome: usize,
}

/// 追加一行（带换行）。父目录不存在时先建。
fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// 读回声明（坏行读作不存在）。
fn read_statements(path: &Path) -> Vec<ImpactStatement> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<ImpactStatement>(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-accountability-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    fn statement(action_id: &str, facts: Vec<&str>, affected: Affected) -> ImpactStatement {
        ImpactStatement {
            action_id: action_id.to_string(),
            at_ms: 1_700_000_000_000,
            boundary: Boundary::Provider,
            facts_used: facts.into_iter().map(String::from).collect(),
            inferences: Vec::new(),
            grants: vec!["grant-1".into()],
            affected,
            reversible: false,
            destination: "provider:anthropic".into(),
            outcome: Outcome::Unknown,
        }
    }

    #[test]
    fn boundary_vocabulary_is_closed_and_round_trips() {
        assert_eq!(Boundary::ALL.len(), 4);
        for boundary in Boundary::ALL {
            assert_eq!(Boundary::parse(boundary.label()), Some(boundary));
        }
        assert_eq!(Boundary::parse("nonsense"), None);
    }

    #[test]
    fn appends_and_reads_back() {
        let path = temp_path("basic");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        assert!(ledger.append(&statement("act-1", vec!["ep-1"], Affected::SelfOnly)));
        let statements = ledger.statements();
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].action_id, "act-1");
        assert_eq!(statements[0].outcome, Outcome::Unknown);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn index_answers_which_actions_a_fact_affected() {
        let path = temp_path("index");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], Affected::SelfOnly));
        ledger.append(&statement(
            "act-2",
            vec!["ep-1", "ep-2"],
            Affected::SelfOnly,
        ));
        ledger.append(&statement("act-3", vec!["ep-9"], Affected::SelfOnly));

        let affected = ledger.actions_referencing("ep-1");
        assert_eq!(affected.len(), 2);
        assert_eq!(affected[0].action_id, "act-1");
        assert_eq!(affected[1].action_id, "act-2");

        let index = ledger.index();
        assert_eq!(index.get("ep-1").map(Vec::len), Some(2));
        assert_eq!(index.get("ep-2").map(Vec::len), Some(1));
        // 未被引用的事实没有条目（负例：不被引用就不该被影响声明打扰）。
        assert!(!index.contains_key("ep-404"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn counts_distinguish_irreversible_and_others() {
        let path = temp_path("counts");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], Affected::SelfOnly));
        ledger.append(&statement("act-2", vec!["ep-2"], Affected::NamedParty));
        let counts = ledger.counts();
        assert_eq!(counts.total, 2);
        assert_eq!(counts.irreversible, 2);
        assert_eq!(counts.affecting_others, 1);
        assert_eq!(counts.unknown_outcome, 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_lines_are_read_as_absent() {
        let path = temp_path("bad");
        std::fs::create_dir_all(path.parent().unwrap()).expect("dir");
        std::fs::write(&path, "{\"action_id\":\"a\"}\nNOT_JSON\n").expect("write");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        assert!(ledger.statements().is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn statement_never_stores_raw_content() {
        let path = temp_path("zerotrue");
        let ledger = AccountabilityLedger::new(Some(path.clone()));
        ledger.append(&statement("act-1", vec!["ep-1"], Affected::SelfOnly));
        let raw = std::fs::read_to_string(&path).expect("read");
        // 零真值：只有 id 与元数据。
        assert!(raw.contains("ep-1"));
        assert!(!raw.contains("@"));
        assert!(!raw.contains("alice"));
        let _ = std::fs::remove_file(&path);
    }
}
