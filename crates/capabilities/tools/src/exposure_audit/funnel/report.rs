//! 守望巡检漏斗 · 报告段产出 (漏斗报告 + 增量视角)。
//!
//! 报告如实回答四件事: ①**各段账** (每段进/出/耗时/是否被成本闸截断);
//! ②**降噪账** (筛查多少、确认多少、复核剔除多少瞬时态 —— 降噪率可度量);
//! ③**增量账** (与上次巡检 diff: 新增/消失/未变; 未完成的会话如实不比);
//! ④**取证账** (精查证据, 全部脱敏后呈现, 零秘密值)。
//!
//! JSON 合同稳定 (`schema`/`schema_version`), 可留档、可再 diff。

use serde_json::{json, Value};

use crate::exposure_audit::funnel::diff::DiffSummary;
use crate::exposure_audit::funnel::evidence::{Candidate, DeepEvidence};
use crate::exposure_audit::funnel::task_seam::PatrolTaskAccount;
use crate::exposure_audit::funnel::FunnelBudget;
use crate::exposure_audit::report::format_utc_ms;

/// 漏斗报告 JSON 合同的稳定身份 (`schema` 字段)。
pub const FUNNEL_REPORT_SCHEMA: &str = "apeireth.patrol_funnel_report";

/// 漏斗报告 JSON 合同的形状版本 (`schema_version` 字段)。
pub const FUNNEL_REPORT_SCHEMA_VERSION: u32 = 1;

/// 段名词面 (固定顺序: 筛查 → 精查 → 复核 → 报告)。
pub const STAGE_ORDER: [&str; 4] = ["screen", "deep", "recheck", "report"];

/// 一段的账 (进/出/耗时/截断, 全部如实)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageStats {
    /// 段名 (`screen` / `deep` / `recheck` / `report`)。
    pub stage: String,
    /// 进入本段的件数。
    pub candidates_in: usize,
    /// 本段实际处理的件数 (本会话口径)。
    pub processed: usize,
    /// 本会话结束时留给续跑的件数 (成本闸截断的余量)。
    pub held_back: usize,
    /// 本会话是否被成本闸截断 (超限截断如实)。
    pub truncated: bool,
    /// 本段耗时 (毫秒, 注入口径)。
    pub duration_ms: u64,
}

impl StageStats {
    fn to_json_value(&self) -> Value {
        json!({
            "stage": self.stage,
            "candidates_in": self.candidates_in,
            "processed": self.processed,
            "held_back": self.held_back,
            "truncated": self.truncated,
            "duration_ms": self.duration_ms,
        })
    }

    fn render_text(&self) -> String {
        let truncation = if self.truncated {
            format!(" | 成本闸截断 (余量 {} 留待续跑)", self.held_back)
        } else {
            String::new()
        };
        format!(
            "  {} 段: 进 {} / 出 {} / 耗时 {} ms{truncation}",
            self.stage, self.candidates_in, self.processed, self.duration_ms
        )
    }
}

/// 降噪账 (复核段降误报的可度量口径)。
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseStats {
    /// 筛查段候选数 (进漏斗的)。
    pub screened: usize,
    /// 复核段确认上报数 (二次测量一致)。
    pub confirmed: usize,
    /// 复核段剔除: 瞬时态 (二次测量不见)。
    pub dropped_transient: usize,
    /// 复核段剔除: 风险级漂移 (二次测量不一致)。
    pub dropped_risk_shift: usize,
    /// 未经复核、不作结论的留存数 (复核段被成本闸截断的余量)。
    pub held_unverified: usize,
    /// 降噪率 = (瞬时态 + 风险级漂移) / 筛查候选; 筛查为 0 记 0。
    pub noise_reduction_rate: f64,
}

impl NoiseStats {
    /// 从各段计数汇总降噪账 (口径见 [`NoiseStats::noise_reduction_rate`])。
    pub fn from_counts(
        screened: usize,
        confirmed: usize,
        dropped_transient: usize,
        dropped_risk_shift: usize,
        held_unverified: usize,
    ) -> Self {
        let dropped = (dropped_transient + dropped_risk_shift) as f64;
        let noise_reduction_rate = if screened == 0 {
            0.0
        } else {
            dropped / screened as f64
        };
        Self {
            screened,
            confirmed,
            dropped_transient,
            dropped_risk_shift,
            held_unverified,
            noise_reduction_rate,
        }
    }

    fn to_json_value(&self) -> Value {
        json!({
            "screened": self.screened,
            "confirmed": self.confirmed,
            "dropped_transient": self.dropped_transient,
            "dropped_risk_shift": self.dropped_risk_shift,
            "held_unverified": self.held_unverified,
            "noise_reduction_rate": self.noise_reduction_rate,
        })
    }
}

/// 一条二次确认的上报候选 (复核段放行 + 精查取证)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedCandidate {
    /// 候选本体 (筛查段词面)。
    pub candidate: Candidate,
    /// 是否做过精查 (风险级低于精查线的候选如实 false)。
    pub deep_checked: bool,
    /// 精查取证 (脱敏后呈现; 未精查 = None)。
    pub evidence: Option<DeepEvidence>,
}

impl ConfirmedCandidate {
    fn to_json_value(&self) -> Value {
        json!({
            "key": self.candidate.key,
            "item_id": self.candidate.item_id,
            "surface": self.candidate.surface,
            "risk": self.candidate.risk.as_str(),
            "detail": self.candidate.detail,
            "deep_checked": self.deep_checked,
            "evidence": self.evidence.as_ref().map(|evidence| json!({
                "kind": evidence.kind.as_str(),
                "summary": evidence.summary,
                "lines": evidence.lines,
                "truncated": evidence.truncated,
            })),
        })
    }

    fn render_text(&self) -> String {
        let mut out = format!(
            "  · {} [{}] {}\n",
            self.candidate.surface,
            self.candidate.risk.label(),
            self.candidate.detail
        );
        if let Some(evidence) = &self.evidence {
            out.push_str(&format!(
                "      取证 ({}): {}\n",
                evidence.kind.as_str(),
                evidence.summary
            ));
            for line in &evidence.lines {
                out.push_str(&format!("        {line}\n"));
            }
        }
        out
    }
}

/// 一份漏斗报告 (巡检一次的完整账)。
#[derive(Debug, Clone, PartialEq)]
pub struct FunnelReport {
    /// 巡检 id。
    pub patrol_id: String,
    /// 本会话是否为断点续查的续跑。
    pub resumed: bool,
    /// 执行段计数 (= 已跑会话数)。
    pub attempts: u32,
    /// 本次巡检是否已完成 (未完成 = 已落库可续跑; 未完成不与上次 diff)。
    pub completed: bool,
    /// 生成时刻 (epoch 毫秒)。
    pub generated_at_ms: i64,
    /// 巡检任务账 (队列同账)。
    pub task: PatrolTaskAccount,
    /// 四段的账 (固定顺序)。
    pub stages: Vec<StageStats>,
    /// 降噪账。
    pub noise: NoiseStats,
    /// 二次确认的上报候选 (含脱敏取证)。
    pub confirmed: Vec<ConfirmedCandidate>,
    /// 与上次巡检的增量 (未完成会话 = None, 如实不比)。
    pub diff: Option<DiffSummary>,
    /// 成本闸 (口径回显, 可复核)。
    pub budget: FunnelBudget,
}

impl FunnelReport {
    /// JSON 合同值 (稳定键)。
    pub fn to_json_value(&self) -> Value {
        json!({
            "schema": FUNNEL_REPORT_SCHEMA,
            "schema_version": FUNNEL_REPORT_SCHEMA_VERSION,
            "patrol_id": self.patrol_id,
            "resumed": self.resumed,
            "attempts": self.attempts,
            "completed": self.completed,
            "generated_at_ms": self.generated_at_ms,
            "generated_at_utc": format_utc_ms(self.generated_at_ms),
            "task": self.task.to_json_value(),
            "stages": self.stages.iter().map(StageStats::to_json_value).collect::<Vec<Value>>(),
            "noise": self.noise.to_json_value(),
            "confirmed": self.confirmed.iter().map(ConfirmedCandidate::to_json_value).collect::<Vec<Value>>(),
            "diff": self.diff.as_ref().map(|diff| json!({
                "baseline_patrol_id": diff.baseline_patrol_id,
                "new": diff.new,
                "gone": diff.gone,
                "unchanged": diff.unchanged,
            })),
            "budget": self.budget.to_json_value(),
        })
    }

    /// JSON 渲染 (可留档、可再 diff)。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_value()).unwrap_or_else(|_| "{}".to_string())
    }

    /// 文本渲染 (漏斗报告正文形态)。
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "守望巡检漏斗报告 · 筛查→精查→复核→报告 · 被测面只读零写\n巡检: {} | 会话: 第 {} 次 | {}\n生成时刻: {} (epoch_ms={})\n口径: 候选二次测量一致才上报; 取证脱敏后呈现 (零明文); 成本闸超限截断如实; 未完成会话已落库可续跑。\n\n",
            self.patrol_id,
            self.attempts,
            if self.completed { "已完成" } else { "未完成 (断点续查中)" },
            format_utc_ms(self.generated_at_ms),
            self.generated_at_ms,
        );
        out.push_str("各段账:\n");
        for stage in &self.stages {
            out.push_str(&stage.render_text());
            out.push('\n');
        }
        out.push_str(&format!(
            "降噪账: 筛查 {} / 确认 {} / 剔除瞬时态 {} / 剔除风险漂移 {} / 留存未复核 {} / 降噪率 {:.2}\n\n",
            self.noise.screened,
            self.noise.confirmed,
            self.noise.dropped_transient,
            self.noise.dropped_risk_shift,
            self.noise.held_unverified,
            self.noise.noise_reduction_rate,
        ));
        out.push_str(&format!("上报候选 ({}):\n", self.confirmed.len()));
        for candidate in &self.confirmed {
            out.push_str(&candidate.render_text());
        }
        out.push('\n');
        match &self.diff {
            Some(diff) => {
                out.push_str(&format!(
                    "增量 (基线 {}): 新增 {} / 消失 {} / 未变 {}\n",
                    diff.baseline_patrol_id.as_deref().unwrap_or("(首次巡检)"),
                    diff.new.len(),
                    diff.gone.len(),
                    diff.unchanged.len()
                ));
                for key in &diff.new {
                    out.push_str(&format!("  + 新增 {key}\n"));
                }
                for key in &diff.gone {
                    out.push_str(&format!("  - 消失 {key}\n"));
                }
                for key in &diff.unchanged {
                    out.push_str(&format!("  = 未变 {key}\n"));
                }
            }
            None => out.push_str("增量: 未完成会话不与上次比 (如实不产出 diff)\n"),
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exposure_audit::report::RiskLevel;

    fn sample_report() -> FunnelReport {
        let candidate = Candidate::new("plaintext_points", "a.toml", RiskLevel::Medium, "d");
        FunnelReport {
            patrol_id: "patrol-0".to_string(),
            resumed: false,
            attempts: 1,
            completed: true,
            generated_at_ms: 0,
            task: PatrolTaskAccount::new("patrol-0", "keeper"),
            stages: STAGE_ORDER
                .iter()
                .map(|stage| StageStats {
                    stage: (*stage).to_string(),
                    candidates_in: 1,
                    processed: 1,
                    held_back: 0,
                    truncated: false,
                    duration_ms: 3,
                })
                .collect(),
            noise: NoiseStats::from_counts(1, 1, 0, 0, 0),
            confirmed: vec![ConfirmedCandidate {
                candidate,
                deep_checked: true,
                evidence: Some(DeepEvidence {
                    key: "plaintext_points|a.toml".to_string(),
                    kind:
                        crate::exposure_audit::funnel::evidence::DeepEvidenceKind::PlaintextFields,
                    summary: "s".to_string(),
                    lines: vec!["api_key = \"[redacted]\"".to_string()],
                    truncated: false,
                }),
            }],
            diff: Some(DiffSummary::between(
                None,
                &[],
                &["plaintext_points|a.toml".to_string()],
            )),
            budget: FunnelBudget::default(),
        }
    }

    #[test]
    fn funnel_json_contract_shape_is_stable() {
        let value = sample_report().to_json_value();
        assert_eq!(value["schema"], FUNNEL_REPORT_SCHEMA);
        assert_eq!(value["schema_version"], FUNNEL_REPORT_SCHEMA_VERSION);
        assert_eq!(value["completed"], true);
        assert_eq!(value["stages"][0]["stage"], "screen");
        assert_eq!(value["stages"][3]["stage"], "report");
        assert_eq!(value["noise"]["confirmed"], 1);
        assert_eq!(
            value["confirmed"][0]["evidence"]["lines"][0],
            "api_key = \"[redacted]\""
        );
        assert_eq!(value["diff"]["new"][0], "plaintext_points|a.toml");
        assert!(value["budget"]["deep"]["max_candidates"].is_number());
    }

    #[test]
    fn noise_rate_is_zero_without_screened_candidates() {
        let noise = NoiseStats::from_counts(0, 0, 0, 0, 0);
        assert_eq!(noise.noise_reduction_rate, 0.0);
        let noisy = NoiseStats::from_counts(4, 3, 1, 0, 0);
        assert!((noisy.noise_reduction_rate - 0.25).abs() < f64::EPSILON);
    }
}
