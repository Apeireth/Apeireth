//! 守夜人 Nightwatch — 离线闲时审计器 (2026-10-10, 主人批准设计)。
//!
//! **职责**: 组合 governance 五件库助手 (`risk` / `eval` / `rubric` / `evidence` /
//! `colang`) 对**被动输入** (episodes 快照 + 审计链快照 + council ballot + .co 源)
//! 做**只读审计**, 产出 [`NightwatchReport`] 并可落盘
//! `<data>/nightwatch/nightwatch-<ts>.json`。
//!
//! **边界 (report-only, 设计红线)**:
//! - 不阻塞、不批准、不改运行时 —— **approval_policy 留热路径** (热路径判否,
//!   闲时复盘是两回事; 本模块永不成为第二审批权威);
//! - 无 LLM / 无随机 / 纯确定性组合 (仅报告落盘一次 IO)。
//!
//! **触发**: 显式命令 `apeireth nightwatch` (显式命令即授权, W2 命令变体);
//! "用户空闲时自动跑"的调度钩子 = 后续 scheduler 项 (本批把审计器做实)。
//!
//! **0 假装输入边界 (实记)**: 生产当前可被动取得的输入 = episodes 快照 + .co 源;
//! 审计链持久化 / council ballot 持久化**尚未接线** —— 对应分析段如实进
//! `advisory_gaps` (不造假输入, 不空转分析)。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_governance::evidence::{EvidenceEntry, EvidenceGuard};
use apeireth_governance::{colang, eval, risk, rubric};
use serde::{Deserialize, Serialize};

/// episodes 被动快照 (调用方从记忆库只读构建)。
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeSnapshot {
    /// episode id
    pub id: String,
    /// 所属会话
    pub session: String,
    /// 角色 (user/assistant)
    pub role: String,
    /// 内容
    pub content: String,
}

/// 审计链被动快照 (未持久化时传空 → 具名缺口)。
#[derive(Debug, Clone, PartialEq)]
pub struct AuditSnapshot {
    /// 决策 (allow/deny/require_approval)
    pub decision: String,
    /// 风险级 (info/low/medium/high/critical/nuclear)
    pub risk: String,
    /// 理由
    pub reason: String,
}

/// 守夜人输入 (全部被动快照; 空集合 = 对应分析段报缺口)。
#[derive(Debug, Clone, Default)]
pub struct NightwatchInputs {
    /// episodes 快照
    pub episodes: Vec<EpisodeSnapshot>,
    /// 审计链快照
    pub audit: Vec<AuditSnapshot>,
    /// council ballot (未持久化时空)
    pub council_ballots: Vec<rubric::Ballot>,
    /// .co 源 ((filename, source))
    pub colang_sources: Vec<(String, String)>,
}

/// 发现域 (五件组合的分段)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingArea {
    /// risk: 记忆库核词扫描 (复盘统计, 非守门)
    RiskVocabulary,
    /// eval: 行为质量趋势 (发言长度均值/波动)
    BehaviorQuality,
    /// risk: 审计链 no-degrade 不变式复盘
    NoDegrade,
    /// evidence: 断言型记忆的经验证据缺口
    EvidenceChain,
    /// rubric: council 立场平衡复盘
    CouncilBalance,
    /// colang: DSL 文件健康
    ColangHealth,
}

/// 一条审计发现 (severity 用 risk 序词表, 可被 `risk_rank` 排序)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// 发现域
    pub area: FindingArea,
    /// 严重度 (info/low/medium/high/critical/nuclear)
    pub severity: String,
    /// 人类可读细节
    pub detail: String,
}

/// 守夜报告 (report-only 的载体)。
#[derive(Debug, Clone, PartialEq)]
pub struct NightwatchReport {
    /// 生成时间 (epoch ms)
    pub at_epoch_ms: i64,
    /// episodes 数
    pub episode_count: usize,
    /// 审计链条数
    pub audit_count: usize,
    /// 发现列表
    pub findings: Vec<Finding>,
    /// 未接线分析段 (0 装: 不造假输入)
    pub advisory_gaps: Vec<String>,
    /// 汇总 (最高严重度 + 数量)
    pub summary: String,
}

impl NightwatchReport {
    /// 全发现的最高严重度序词 (无发现 = info)。
    pub fn max_severity_rank(&self) -> i32 {
        self.findings
            .iter()
            .map(|finding| risk::risk_rank(&finding.severity))
            .max()
            .unwrap_or_else(|| risk::risk_rank("info"))
    }
}

/// 条目是否无经验证据 (仅有推断级证据)。
fn guard_has_no_empirical(entry: Option<&apeireth_governance::evidence::EvidenceEntry>) -> bool {
    entry.is_none_or(|entry| !entry.has_empirical_evidence())
}

/// 记忆断言句式 (确定性子串匹配; evidence 段的输入).
fn is_assertion(content: &str) -> bool {
    ["已完成", "已修复", "已验证", "确保", "一定能"]
        .iter()
        .any(|needle| content.contains(needle))
}

/// 核词扫描表 (复盘统计口径; **不是**守门词表 —— 守门在
/// `tools::guardrail` 热路径, 此处只统计不复盘拦截)。
const NUCLEAR_VOCABULARY: &[&str] = &[
    "rm -rf",
    "rm -fr",
    "format c:",
    "del /f",
    "reg delete",
    "sc delete",
    "netsh advfirewall",
    "mkfs",
    "dd if=",
];

/// **守夜审计** (纯函数, 五件组合)。
pub fn audit(inputs: &NightwatchInputs) -> NightwatchReport {
    let mut findings = Vec::new();
    let mut advisory_gaps = Vec::new();

    // ① risk 核词扫描 (复盘统计)。
    for episode in &inputs.episodes {
        let lowered = episode.content.to_lowercase();
        for term in NUCLEAR_VOCABULARY {
            if lowered.contains(&term.to_lowercase()) {
                findings.push(Finding {
                    area: FindingArea::RiskVocabulary,
                    severity: "high".to_string(),
                    detail: format!(
                        "episode {} ({}) 含核词 `{term}` —— 守门若放行过, 值得复盘",
                        episode.id, episode.session
                    ),
                });
            }
        }
    }

    // ② eval 行为质量: 发言长度均值/样本标准差 (确定性浅特征, 同 morphology 口径)。
    if inputs.episodes.len() >= 3 {
        let scores: Vec<eval::EvalScore> = inputs
            .episodes
            .iter()
            .enumerate()
            .map(|(index, episode)| {
                eval::EvalScore::new(
                    format!("len-{index}"),
                    episode.content.chars().count() as f64,
                )
            })
            .collect();
        let mean = eval::mean(&scores).unwrap_or(0.0);
        let stddev = eval::stddev(&scores);
        if stddev > mean.max(1.0) {
            findings.push(Finding {
                area: FindingArea::BehaviorQuality,
                severity: "low".to_string(),
                detail: format!(
                    "发言长度波动异常 (mean={mean:.1} chars, stddev={stddev:.1}) —— 行为质量漂移信号"
                ),
            });
        }
    }

    // ③ risk no-degrade 复盘 (审计链空 = 具名缺口)。
    if inputs.audit.is_empty() {
        advisory_gaps
            .push("audit chain 未持久化供闲时读取 —— no-degrade 复盘段空转 (接线项)".to_string());
    } else {
        for window in inputs.audit.windows(2) {
            if risk::is_degrade(&window[0].risk, &window[1].risk) {
                findings.push(Finding {
                    area: FindingArea::NoDegrade,
                    severity: "medium".to_string(),
                    detail: format!(
                        "审计链风险静默降级: `{}` → `{}` ({})",
                        window[0].risk, window[1].risk, window[1].reason
                    ),
                });
            }
        }
    }

    // ④ evidence 断言缺口: 断言型 episode 以推断入账 (无经验证据附链) ——
    //    逐条 verify 证明其均为推断级 (checker 口径: 推断+低置信合法过),
    //    守夜价值在**聚合观察**: 断言全部无经验链 = 系统性 evidence 缺口。
    if !inputs.episodes.is_empty() {
        let mut guard = EvidenceGuard::new();
        let now = now_epoch_ms();
        let mut assertion_ids = Vec::new();
        for episode in &inputs.episodes {
            if !is_assertion(&episode.content) {
                continue;
            }
            let claim_id = format!("episode-{}", episode.id);
            guard.record(EvidenceEntry::from_inference(
                claim_id.clone(),
                episode.content.chars().take(80).collect::<String>(),
                0.5, // 记忆条目默认推断置信度 (< 0.7 天花板)
                now,
                "nightwatch",
            ));
            assertion_ids.push(claim_id);
        }
        let inferred_only = assertion_ids
            .iter()
            .filter(|claim_id| {
                !guard.verify(claim_id).is_pass() || guard_has_no_empirical(guard.get(claim_id))
            })
            .count();
        if inferred_only > 0 {
            findings.push(Finding {
                area: FindingArea::EvidenceChain,
                severity: "low".to_string(),
                detail: format!(
                    "{inferred_only} 条断言型记忆以推断入账, 无经验证据附链 (evidence 链路缺口统计)"
                ),
            });
        }
    }

    // ⑤ rubric council 平衡 (ballot 空 = 具名缺口)。
    if inputs.council_ballots.is_empty() {
        advisory_gaps.push("council ballot 未持久化 —— 立场平衡复盘段空转 (接线项)".to_string());
    } else {
        let synthesis = rubric::synthesize(&inputs.council_ballots);
        if !synthesis.dissenting.is_empty() {
            findings.push(Finding {
                area: FindingArea::CouncilBalance,
                severity: "low".to_string(),
                detail: format!(
                    "council 复盘: 加权分 {:.2}, 异议 {:?}",
                    synthesis.weighted_score, synthesis.dissenting
                ),
            });
        }
    }

    // ⑥ colang 健康: 逐文件 parse (语法健康 = 可复盘的前提; validate 的细度
    //    属后续增强, 本批只用解析成败, 不假装更深的校验)。
    for (filename, source) in &inputs.colang_sources {
        if let Err(error) = colang::ColangParser::new(filename.as_str(), source.as_str()).parse() {
            findings.push(Finding {
                area: FindingArea::ColangHealth,
                severity: "medium".to_string(),
                detail: format!("colang `{filename}` 解析失败: {error}"),
            });
        }
    }

    let max_rank = findings
        .iter()
        .map(|finding| risk::risk_rank(&finding.severity))
        .max()
        .unwrap_or_else(|| risk::risk_rank("info"));
    let summary = format!(
        "守夜复盘 {} episodes / {} audit: {} findings (max severity rank {max_rank}), {} gaps",
        inputs.episodes.len(),
        inputs.audit.len(),
        findings.len(),
        advisory_gaps.len(),
    );

    NightwatchReport {
        at_epoch_ms: now_epoch_ms(),
        episode_count: inputs.episodes.len(),
        audit_count: inputs.audit.len(),
        findings,
        advisory_gaps,
        summary,
    }
}

/// 报告落盘 (report-only 的载体输出; 目录不存在则建)。
pub fn write_report(dir: &Path, report: &NightwatchReport) -> Result<PathBuf, std::io::Error> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("nightwatch-{}.json", report.at_epoch_ms));
    let json = serde_json::json!({
        "at_epoch_ms": report.at_epoch_ms,
        "episode_count": report.episode_count,
        "audit_count": report.audit_count,
        "summary": report.summary,
        "advisory_gaps": report.advisory_gaps,
        "findings": report.findings,
    });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&json).unwrap_or_default(),
    )?;
    Ok(path)
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod nightwatch_tests {
    use super::*;

    fn episode(id: &str, content: &str) -> EpisodeSnapshot {
        EpisodeSnapshot {
            id: id.to_string(),
            session: "s1".to_string(),
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn empty_inputs_yield_clean_report_with_named_gaps() {
        // 五件门②对偶 + 0 装: 空输入不 panic, 缺口具名 (不造假输入)。
        let report = audit(&NightwatchInputs::default());
        assert!(report.findings.is_empty());
        assert_eq!(report.advisory_gaps.len(), 2, "audit + ballot 两缺口具名");
        assert_eq!(report.max_severity_rank(), risk::risk_rank("info"));
    }

    #[test]
    fn nuclear_vocabulary_in_memory_is_flagged() {
        // 五件门③: risk 段真信号 = 记忆含核词 → high finding。
        let inputs = NightwatchInputs {
            episodes: vec![episode("e1", "帮我 rm -rf /tmp/x")],
            ..Default::default()
        };
        let report = audit(&inputs);
        assert!(report
            .findings
            .iter()
            .any(|f| f.area == FindingArea::RiskVocabulary && f.severity == "high"));
    }

    #[test]
    fn assertion_without_evidence_reports_gap() {
        // 五件门③: evidence 段 = 断言型记忆无经验证据附链。
        let inputs = NightwatchInputs {
            episodes: vec![episode("e2", "该模块已完成并通过验证")],
            ..Default::default()
        };
        let report = audit(&inputs);
        assert!(report
            .findings
            .iter()
            .any(|f| f.area == FindingArea::EvidenceChain));
    }

    #[test]
    fn audit_chain_degrade_is_flagged() {
        let inputs = NightwatchInputs {
            audit: vec![
                AuditSnapshot {
                    decision: "deny".into(),
                    risk: "high".into(),
                    reason: "高危".into(),
                },
                AuditSnapshot {
                    decision: "allow".into(),
                    risk: "low".into(),
                    reason: "后续同类".into(),
                },
            ],
            ..Default::default()
        };
        let report = audit(&inputs);
        assert!(report
            .findings
            .iter()
            .any(|f| f.area == FindingArea::NoDegrade && f.severity == "medium"));
    }

    #[test]
    fn colang_sources_are_validated() {
        // 合法 .co = 无 finding; 残缺 .co = medium finding.
        let inputs = NightwatchInputs {
            colang_sources: vec![
                (
                    "ok.co".to_string(),
                    "define user greet\n  \"hi\"\n".to_string(),
                ),
                (
                    "bad.co".to_string(),
                    "define user oops\n  ???\n".to_string(),
                ),
            ],
            ..Default::default()
        };
        let report = audit(&inputs);
        let colang_findings: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.area == FindingArea::ColangHealth)
            .collect();
        assert_eq!(colang_findings.len(), 1, "仅残缺文件报 finding");
        assert!(colang_findings[0].detail.contains("bad.co"));
    }
    #[tokio::test]
    async fn report_writes_json_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let report = audit(&NightwatchInputs::default());
        let path = write_report(dir.path(), &report).expect("write");
        assert!(path.exists());
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("守夜复盘"), "{text}");
    }
}
