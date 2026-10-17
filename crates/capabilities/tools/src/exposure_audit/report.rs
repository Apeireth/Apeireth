//! 暴露面体检报告的报告模型 (V5-b 安全守望 · 防御语自查面)。
//!
//! 「帮你看清你暴露了什么」的产出物: 分项 (发现数 / 风险级 / 整改建议一句 /
//! 分项评分) + 总评 (均分 + 评级 + 一句话结论), 文本与 JSON 双渲染。
//! 诚实口径: 能测的测, 不能测的**显式「未测」** (评分与总评不计入, 不猜数);
//! 凭据值/密钥值永不出现在报告里 (发现只报路径词面与计数)。

use serde_json::{json, Value};

/// 报告 JSON 合同的稳定身份 (`schema` 字段)。
pub const REPORT_SCHEMA: &str = "apeireth.exposure_report";

/// 报告 JSON 合同的形状版本 (`schema_version` 字段)。
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// 分项评分扣分: 高风险发现。
pub const DEDUCTION_HIGH: u32 = 25;

/// 分项评分扣分: 中风险发现。
pub const DEDUCTION_MEDIUM: u32 = 10;

/// 分项评分扣分: 低风险发现。
pub const DEDUCTION_LOW: u32 = 3;

/// 风险级 (防御语: 整改优先级, 不是攻击语的可利用性断言)。
///
/// 序: `None < Low < Medium < High` (取最大值即体检项风险级)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// 无风险发现 (干净面)。
    None,
    /// 低: 信息面宽, 建议收口但不急。
    Low,
    /// 中: 暴露面偏宽或明文风险点, 择期整改。
    Medium,
    /// 高: 凭据/问责链风险点, 优先整改。
    High,
}

impl RiskLevel {
    /// JSON 词面。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// 报告文本词面 (中文短词)。
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
        }
    }
}

/// 体检项测量状态 (诚实口径: 未测即显式未测, 不冒充干净)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// 已测 (本地面真扫过)。
    Measured,
    /// 未测 (超出本地只读自查范围或无可核对面), 显式标注。
    NotMeasured,
}

impl ItemStatus {
    /// JSON 词面。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::NotMeasured => "not_measured",
        }
    }

    /// 报告文本词面。
    pub fn label(self) -> &'static str {
        match self {
            Self::Measured => "已测",
            Self::NotMeasured => "未测",
        }
    }
}

/// 一条暴露面发现 (只报词面与事实计数, 零秘密值)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// 暴露面词面 (相对路径 / 监听地址端口 / 绑定名 —— 均非秘密)。
    pub surface: String,
    /// 该发现的风险级。
    pub risk: RiskLevel,
    /// 一句事实 (中性表述)。
    pub detail: String,
}

impl Finding {
    /// 一条发现。
    pub fn new(surface: impl Into<String>, risk: RiskLevel, detail: impl Into<String>) -> Self {
        Self {
            surface: surface.into(),
            risk,
            detail: detail.into(),
        }
    }

    fn to_json_value(&self) -> Value {
        json!({
            "surface": self.surface,
            "risk": self.risk.as_str(),
            "detail": self.detail,
        })
    }

    fn render_text(&self) -> String {
        format!(
            "  · {} [{}] {}",
            self.surface,
            self.risk.label(),
            self.detail
        )
    }
}

/// 一个体检项 (发现数 / 风险级 / 整改建议一句 / 分项评分)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposureItem {
    /// 体检项 id (稳定合同词面)。
    pub id: String,
    /// 体检项标题。
    pub title: String,
    /// 测量状态 (未测显式标注)。
    pub status: ItemStatus,
    /// 发现列表 (未测项恒空)。
    pub findings: Vec<Finding>,
    /// 整改建议 (一句)。
    pub advice: String,
    /// 补充说明 (测量口径 / 未测原因; 可空)。
    pub note: Option<String>,
}

impl ExposureItem {
    /// 已测体检项。
    pub fn measured(
        id: impl Into<String>,
        title: impl Into<String>,
        findings: Vec<Finding>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: ItemStatus::Measured,
            findings,
            advice: advice.into(),
            note: None,
        }
    }

    /// 未测体检项 (显式「未测」+ 原因; 评分与总评不计入)。
    pub fn not_measured(
        id: impl Into<String>,
        title: impl Into<String>,
        note: impl Into<String>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: ItemStatus::NotMeasured,
            findings: Vec::new(),
            advice: advice.into(),
            note: Some(note.into()),
        }
    }

    /// 追加补充说明。
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// 发现数。
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// 风险级 (未测 = `None`, 不估计、不冒充)。
    pub fn risk(&self) -> Option<RiskLevel> {
        if self.status == ItemStatus::NotMeasured {
            return None;
        }
        Some(
            self.findings
                .iter()
                .map(|finding| finding.risk)
                .max()
                .unwrap_or(RiskLevel::None),
        )
    }

    /// 分项评分 (100 起扣: 高 -25 / 中 -10 / 低 -3, 下限 0; 未测 = `None`)。
    pub fn score(&self) -> Option<u32> {
        if self.status == ItemStatus::NotMeasured {
            return None;
        }
        let mut score = 100u32;
        for finding in &self.findings {
            score = score.saturating_sub(match finding.risk {
                RiskLevel::High => DEDUCTION_HIGH,
                RiskLevel::Medium => DEDUCTION_MEDIUM,
                RiskLevel::Low => DEDUCTION_LOW,
                RiskLevel::None => 0,
            });
        }
        Some(score)
    }

    fn to_json_value(&self) -> Value {
        json!({
            "id": self.id,
            "title": self.title,
            "status": self.status.as_str(),
            "finding_count": self.finding_count(),
            "risk": self.risk().map(RiskLevel::as_str),
            "score": self.score(),
            "advice": self.advice,
            "note": self.note,
            "findings": self
                .findings
                .iter()
                .map(Finding::to_json_value)
                .collect::<Vec<Value>>(),
        })
    }

    fn render_text(&self, index: usize, total: usize) -> String {
        let risk = self
            .risk()
            .map(RiskLevel::label)
            .unwrap_or("未测")
            .to_string();
        let score = self
            .score()
            .map(|score| score.to_string())
            .unwrap_or_else(|| "未测".to_string());
        let mut out = format!(
            "[{index}/{total}] {} ({}) — {} | 发现 {} | 风险 {} | 评分 {}\n",
            self.title,
            self.id,
            self.status.label(),
            self.finding_count(),
            risk,
            score
        );
        for finding in &self.findings {
            out.push_str(&finding.render_text());
            out.push('\n');
        }
        if let Some(note) = &self.note {
            out.push_str(&format!("      说明: {note}\n"));
        }
        out.push_str(&format!("      建议: {}\n", self.advice));
        out
    }
}

/// 总评 (已测项均分 + 评级 + 一句话结论)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverallScore {
    /// 总评分 (已测项分项评分的算术均值; 未测不计入)。
    pub score: u32,
    /// 评级 (≥90 优 / ≥75 良 / ≥60 中 / 其余 差)。
    pub grade: String,
    /// 一句话结论。
    pub verdict: String,
    /// 已测项数。
    pub measured_items: usize,
    /// 未测项数 (显式口径)。
    pub not_measured_items: usize,
}

impl OverallScore {
    /// 从体检项列表汇总总评。
    pub fn from_items(items: &[ExposureItem]) -> Self {
        let measured: Vec<&ExposureItem> = items
            .iter()
            .filter(|item| item.status == ItemStatus::Measured)
            .collect();
        let not_measured_items = items.len() - measured.len();
        let score = if measured.is_empty() {
            0
        } else {
            let total: u32 = measured.iter().map(|item| item.score().unwrap_or(0)).sum();
            total / measured.len() as u32
        };
        let grade = match score {
            90..=100 => "优",
            75..=89 => "良",
            60..=74 => "中",
            _ => "差",
        }
        .to_string();
        let has_high = items.iter().any(|item| {
            item.findings
                .iter()
                .any(|finding| finding.risk == RiskLevel::High)
        });
        let has_medium = items.iter().any(|item| {
            item.findings
                .iter()
                .any(|finding| finding.risk == RiskLevel::Medium)
        });
        let verdict = if has_high {
            "存在高风险暴露点, 按整改建议逐项收口后复检 (高风险面优先)".to_string()
        } else if has_medium {
            "存在中风险暴露点, 择期收口并保持周期巡检".to_string()
        } else {
            "本地暴露面干净, 保持只读自查与周期巡检".to_string()
        };
        Self {
            score,
            grade,
            verdict,
            measured_items: measured.len(),
            not_measured_items,
        }
    }

    fn to_json_value(&self) -> Value {
        json!({
            "score": self.score,
            "grade": self.grade,
            "verdict": self.verdict,
            "measured_items": self.measured_items,
            "not_measured_items": self.not_measured_items,
        })
    }
}

/// 一份暴露面体检报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposureReport {
    /// 生成时刻 (epoch 毫秒; 回拨钳 0, 不编时间)。
    pub generated_at_ms: i64,
    /// 体检数据根 (本地数据面)。
    pub data_root: String,
    /// 体检项 (固定顺序, 含显式未测项)。
    pub items: Vec<ExposureItem>,
    /// 总评。
    pub overall: OverallScore,
}

impl ExposureReport {
    /// 汇总一份报告 (总评由体检项列表算出)。
    pub fn new(
        generated_at_ms: i64,
        data_root: &std::path::Path,
        items: Vec<ExposureItem>,
    ) -> Self {
        let overall = OverallScore::from_items(&items);
        Self {
            generated_at_ms,
            data_root: data_root.to_string_lossy().into_owned(),
            items,
            overall,
        }
    }

    /// JSON 合同值 (稳定键: schema / schema_version / items / overall ...)。
    pub fn to_json_value(&self) -> Value {
        json!({
            "schema": REPORT_SCHEMA,
            "schema_version": REPORT_SCHEMA_VERSION,
            "generated_at_ms": self.generated_at_ms,
            "generated_at_utc": format_utc_ms(self.generated_at_ms),
            "data_root": self.data_root,
            "read_only": true,
            "items": self
                .items
                .iter()
                .map(ExposureItem::to_json_value)
                .collect::<Vec<Value>>(),
            "overall": self.overall.to_json_value(),
        })
    }

    /// JSON 渲染 (可留档、可 diff —— 暴露面收敛可度量)。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_value()).unwrap_or_else(|_| "{}".to_string())
    }

    /// 文本渲染 (体检报告样例的正文形态)。
    pub fn to_text(&self) -> String {
        let total = self.items.len();
        let mut out = format!(
            "暴露面体检报告 · 只读零写 · 本地自查\n数据根: {}\n生成时刻: {} (epoch_ms={})\n口径: 只测本地面; 标注「未测」的项未测量、不计入总评; 凭据值/密钥值永不出现在本报告。\n\n",
            self.data_root,
            format_utc_ms(self.generated_at_ms),
            self.generated_at_ms
        );
        for (index, item) in self.items.iter().enumerate() {
            out.push_str(&item.render_text(index + 1, total));
            out.push('\n');
        }
        out.push_str(&format!(
            "总评: {}/100 ({}) — 已测 {} 项 / 未测 {} 项\n一句话: {}\n",
            self.overall.score,
            self.overall.grade,
            self.overall.measured_items,
            self.overall.not_measured_items,
            self.overall.verdict
        ));
        out
    }
}

/// epoch 毫秒 → UTC 词面 (`YYYY-MM-DDTHH:MM:SS.mmmZ`; 纯算术, 不引新依赖)。
pub fn format_utc_ms(ms: i64) -> String {
    let millis = ms.rem_euclid(1000);
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// 儒略日 → 公历年月日 (民用历天数算法, 无依赖)。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(risk: RiskLevel) -> Finding {
        Finding::new("surface", risk, "fact")
    }

    #[test]
    fn item_score_deducts_by_risk_and_floor_at_zero() {
        let clean = ExposureItem::measured("id", "title", vec![], "advice");
        assert_eq!(clean.score(), Some(100));
        assert_eq!(clean.risk(), Some(RiskLevel::None));

        let mixed = ExposureItem::measured(
            "id",
            "title",
            vec![
                finding(RiskLevel::High),
                finding(RiskLevel::Medium),
                finding(RiskLevel::Low),
                finding(RiskLevel::None),
            ],
            "advice",
        );
        assert_eq!(mixed.score(), Some(100 - 25 - 10 - 3));
        assert_eq!(mixed.risk(), Some(RiskLevel::High));

        let flooded =
            ExposureItem::measured("id", "title", vec![finding(RiskLevel::High); 8], "advice");
        assert_eq!(flooded.score(), Some(0), "评分下限 0");
    }

    #[test]
    fn not_measured_items_have_no_score_and_no_estimated_risk() {
        let item = ExposureItem::not_measured("id", "title", "reason", "advice");
        assert_eq!(item.score(), None, "未测不评分");
        assert_eq!(item.risk(), None, "未测不估计风险");
        assert_eq!(item.finding_count(), 0);
        assert_eq!(item.note.as_deref(), Some("reason"));
    }

    #[test]
    fn overall_averages_measured_items_and_grades() {
        let items = vec![
            ExposureItem::measured("a", "a", vec![], "advice"),
            ExposureItem::measured("b", "b", vec![finding(RiskLevel::Medium)], "advice"),
            ExposureItem::not_measured("c", "c", "reason", "advice"),
        ];
        let overall = OverallScore::from_items(&items);
        assert_eq!(overall.score, (100 + 90) / 2);
        assert_eq!(overall.grade, "优");
        assert_eq!(overall.measured_items, 2);
        assert_eq!(overall.not_measured_items, 1);
        assert!(
            overall.verdict.contains("中风险"),
            "中风险发现要进一句话结论: {}",
            overall.verdict
        );

        let flooded = vec![ExposureItem::measured(
            "a",
            "a",
            vec![finding(RiskLevel::High); 3],
            "advice",
        )];
        let overall = OverallScore::from_items(&flooded);
        assert_eq!(overall.score, 25);
        assert_eq!(overall.grade, "差");
        assert!(
            overall.verdict.contains("高风险"),
            "高风险发现要进一句话结论: {}",
            overall.verdict
        );
    }

    #[test]
    fn json_contract_shape_is_stable() {
        let report = ExposureReport::new(
            0,
            std::path::Path::new("/data"),
            vec![
                ExposureItem::measured(
                    "credential_files",
                    "凭据文件分布",
                    vec![Finding::new("a/.env", RiskLevel::Medium, "fact")],
                    "advice",
                ),
                ExposureItem::not_measured("external_surface", "外网暴露面", "reason", "advice"),
            ],
        );
        let value = report.to_json_value();
        assert_eq!(value["schema"], REPORT_SCHEMA);
        assert_eq!(value["schema_version"], REPORT_SCHEMA_VERSION);
        assert_eq!(value["read_only"], true);
        assert_eq!(value["generated_at_ms"], 0);
        assert_eq!(value["generated_at_utc"], "1970-01-01T00:00:00.000Z");
        assert_eq!(value["data_root"], "/data");
        let items = value["items"].as_array().expect("items array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "credential_files");
        assert_eq!(items[0]["status"], "measured");
        assert_eq!(items[0]["finding_count"], 1);
        assert_eq!(items[0]["risk"], "medium");
        assert_eq!(items[0]["score"], 90);
        assert_eq!(items[0]["findings"][0]["surface"], "a/.env");
        assert_eq!(items[1]["status"], "not_measured");
        assert_eq!(items[1]["risk"], Value::Null, "未测不估计风险");
        assert_eq!(items[1]["score"], Value::Null, "未测不评分");
        assert_eq!(items[1]["note"], "reason");
        assert!(value["overall"]["score"].is_number());
        assert!(value["overall"]["grade"].is_string());
    }

    #[test]
    fn utc_rendering_matches_known_instants() {
        assert_eq!(format_utc_ms(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(format_utc_ms(1_760_000_000_123), "2025-10-09T08:53:20.123Z");
    }
}
