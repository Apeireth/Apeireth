//! 守望巡检漏斗 · 候选模型与精查取证 (筛查段产出 / 精查段深查)。
//!
//! 口径 (与 V5-b 体检同族硬语义):
//! - **词面即身份**: [`Candidate::key`] = `<体检项 id>|<暴露面词面>`, 全程稳定,
//!   增量 diff 与断点续查都认这个键;
//! - **零明文取证**: 明文点取证是**字段级脱敏后**的行 ([`crate::sensitive_path`]
//!   的字段值脱敏, 密钥字段值恒为 `[redacted]`), 凭据面件**内容零读取**
//!   (只取路径元数据), 备份包只读清单与文件名 —— 取证行里没有任何秘密值;
//! - **如实未测**: 无取证对象的候选 (聚合/截断面、未测项) 显式给「无取证对象」
//!   证据, 不编造取证。

use std::path::Path;

use crate::backup::{BackupManifest, MANIFEST_FILE_NAME};
use crate::exposure_audit::credential_surface::{is_credential_surface, walk_entries};
use crate::exposure_audit::report::{ExposureReport, RiskLevel};
use crate::exposure_audit::ExposureAuditConfig;
use crate::sensitive_path::{redact_secret_field_values, REDACTED};

/// 单件取证的取证行上限 (超出如实记截断, 不静默丢)。
pub(crate) const MAX_EVIDENCE_LINES: usize = 8;

/// 备份清单逐件核对的行数上限 (失控大清单保护; 超限如实记截断)。
pub(crate) const MAX_MANIFEST_ROWS: usize = 200;

/// 明文点取证的单件读取上限 (与体检项二同口径: 大件不进内存)。
pub(crate) const DEEP_MAX_READ_BYTES: u64 = 1024 * 1024;

/// 一条风险候选 (筛查段产出; 词面身份贯穿全程)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// 候选键 (`<体检项 id>|<暴露面词面>`), 全程稳定身份。
    pub key: String,
    /// 体检项 id (合同词面)。
    pub item_id: String,
    /// 暴露面词面 (相对路径 / 监听地址端口 / 聚合面 —— 均非秘密)。
    pub surface: String,
    /// 筛查段风险级。
    pub risk: RiskLevel,
    /// 筛查段一句事实 (中性表述)。
    pub detail: String,
}

/// 候选键 (稳定身份)。
pub fn candidate_key(item_id: &str, surface: &str) -> String {
    format!("{item_id}|{surface}")
}

impl Candidate {
    /// 一条候选。
    pub fn new(
        item_id: impl Into<String>,
        surface: impl Into<String>,
        risk: RiskLevel,
        detail: impl Into<String>,
    ) -> Self {
        let item_id = item_id.into();
        let surface = surface.into();
        Self {
            key: candidate_key(&item_id, &surface),
            item_id,
            surface,
            risk,
            detail: detail.into(),
        }
    }

    /// 筛查段产出: 六项速扫报告 → 风险候选 (逐项展开, 同键取最高风险;
    /// 顺序稳定 = 体检项顺序 × 发现顺序)。
    pub fn from_report(report: &ExposureReport) -> Vec<Candidate> {
        let mut out: Vec<Candidate> = Vec::new();
        let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for item in &report.items {
            for finding in &item.findings {
                let candidate = Candidate::new(
                    item.id.clone(),
                    finding.surface.clone(),
                    finding.risk,
                    finding.detail.clone(),
                );
                match seen.get(&candidate.key) {
                    Some(index) => {
                        let existing: &mut Candidate = &mut out[*index];
                        if candidate.risk > existing.risk {
                            existing.risk = candidate.risk;
                        }
                    }
                    None => {
                        seen.insert(candidate.key.clone(), out.len());
                        out.push(candidate);
                    }
                }
            }
        }
        out
    }
}

/// 精查取证的种类 (每种取证的口径不同, 报告里显式区分)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepEvidenceKind {
    /// 明文点 · 字段级取证 (密钥字段值已脱敏为 `[redacted]`)。
    PlaintextFields,
    /// 备份排除项 · 逐件核对 (清单行 + 排除行逐条对账)。
    ManifestAudit,
    /// 凭据面 · 路径元数据取证 (内容零读取)。
    PathMetadata,
    /// 无逐件取证对象 (如实说明, 不编造证据)。
    ScopeNote,
}

impl DeepEvidenceKind {
    /// JSON 词面。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlaintextFields => "plaintext_fields",
            Self::ManifestAudit => "manifest_audit",
            Self::PathMetadata => "path_metadata",
            Self::ScopeNote => "scope_note",
        }
    }

    /// 从词面解析 (未知词面 = None, 调用方按坏档处理)。
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "plaintext_fields" => Some(Self::PlaintextFields),
            "manifest_audit" => Some(Self::ManifestAudit),
            "path_metadata" => Some(Self::PathMetadata),
            "scope_note" => Some(Self::ScopeNote),
            _ => None,
        }
    }
}

/// 一份精查取证 (每条取证行都已脱敏或纯词面, 零秘密值)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepEvidence {
    /// 对应候选键。
    pub key: String,
    /// 取证种类。
    pub kind: DeepEvidenceKind,
    /// 一句取证结论 (中性表述, 零秘密值)。
    pub summary: String,
    /// 取证行 (已脱敏/纯词面; 超上限部分聚进 summary 并记截断)。
    pub lines: Vec<String>,
    /// 取证行/逐件数是否触及上限被截断 (如实可查)。
    pub truncated: bool,
}

impl DeepEvidence {
    /// 无逐件取证对象的如实证据。
    pub fn note(key: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            kind: DeepEvidenceKind::ScopeNote,
            summary: summary.into(),
            lines: Vec::new(),
            truncated: false,
        }
    }
}

/// 风险级词面 → 枚举 (坏档 = None)。
pub(crate) fn risk_from_word(word: &str) -> Option<RiskLevel> {
    match word {
        "none" => Some(RiskLevel::None),
        "low" => Some(RiskLevel::Low),
        "medium" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        _ => None,
    }
}

/// 精查段入口: 按体检项分派逐项深查 (全程只读零写)。
pub(crate) fn deep_check(
    audit: &ExposureAuditConfig,
    bundle: Option<&Path>,
    candidate: &Candidate,
) -> DeepEvidence {
    let is_meta_surface = candidate.surface.starts_with('(');
    if is_meta_surface {
        return DeepEvidence::note(
            candidate.key.clone(),
            "聚合/截断面无逐件取证对象 (发现总数已在筛查段如实计数), 本段不编造证据",
        );
    }
    match candidate.item_id.as_str() {
        "plaintext_points" => plaintext_forensics(&audit.data_root, candidate),
        "backup_exclusions" => manifest_audit(candidate, bundle),
        "credential_files" => path_metadata(&audit.data_root, candidate),
        "listening_ports" => DeepEvidence::note(
            candidate.key.clone(),
            "监听面精查口径: 逐件取证在复核段二次测量 (瞬时态在复核段降噪), 本段不重复采集",
        ),
        "token_bindings" => DeepEvidence::note(
            candidate.key.clone(),
            "绑定面判定真源在装载面, 本段只读消费判定结果, 不重复判定",
        ),
        _ => DeepEvidence::note(
            candidate.key.clone(),
            "本项无逐件取证对象 (未测面或无本地取证源), 如实无证据",
        ),
    }
}

/// 明文点 · 字段级取证 (脱敏后呈现): 只呈现**字段值已被脱敏**的行,
/// 密钥字段值恒为 `[redacted]`, 原始值零采集、零呈现。
fn plaintext_forensics(root: &Path, candidate: &Candidate) -> DeepEvidence {
    let key = candidate.key.clone();
    let path = root.join(&candidate.surface);
    let Ok(meta) = std::fs::metadata(&path) else {
        return DeepEvidence::note(key, "取证目标不可达 (只读 stat 失败), 本件未取证");
    };
    if meta.len() > DEEP_MAX_READ_BYTES {
        return DeepEvidence::note(
            key,
            format!(
                "取证目标超过单件读取上限 ({} 字节), 本件未取证 (如实未测)",
                DEEP_MAX_READ_BYTES
            ),
        );
    }
    let Ok(bytes) = std::fs::read(&path) else {
        return DeepEvidence::note(key, "取证目标读取失败, 本件未取证 (如实未测)");
    };
    let text = String::from_utf8_lossy(&bytes);
    let redacted = redact_secret_field_values(&text);
    let mut lines: Vec<String> = Vec::new();
    let mut changed = 0usize;
    for (raw_line, redacted_line) in text.lines().zip(redacted.lines()) {
        if raw_line == redacted_line {
            continue;
        }
        changed += 1;
        if lines.len() < MAX_EVIDENCE_LINES {
            lines.push(redacted_line.trim_end().to_string());
        }
    }
    let truncated = changed > lines.len();
    DeepEvidence {
        key,
        kind: DeepEvidenceKind::PlaintextFields,
        summary: format!(
            "字段级取证: {changed} 行含密钥字段明文值, 取证行已脱敏为 {REDACTED} (值零采集, 零明文呈现)"
        ),
        lines,
        truncated,
    }
}

/// 备份排除项 · 逐件核对: 清单行逐条对凭据面词表, 显式排除行逐条核对
/// 「真的没进包」(清单行 + 包内文件名两面都查, 不打开包内内容)。
fn manifest_audit(candidate: &Candidate, bundle: Option<&Path>) -> DeepEvidence {
    let key = candidate.key.clone();
    let Some(bundle) = bundle else {
        return DeepEvidence::note(key, "无可核对备份包, 逐件核对未测 (诚实未测口径)");
    };
    let manifest_path = bundle.join(MANIFEST_FILE_NAME);
    let manifest: BackupManifest = match std::fs::read(&manifest_path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<BackupManifest>(&bytes).map_err(|error| error.to_string())
        }) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return DeepEvidence::note(key, format!("备份清单不可解析, 逐件核对未测: {reason}"));
        }
    };

    let mut lines: Vec<String> = Vec::new();
    let mut violations = 0usize;
    let mut truncated = false;

    let mut push_line = |lines: &mut Vec<String>, line: String| {
        if lines.len() < MAX_EVIDENCE_LINES {
            lines.push(line);
        }
    };

    // 面一: 清单行逐件对凭据面词表。
    let mut rows_checked = 0usize;
    for entry in &manifest.files {
        if rows_checked >= MAX_MANIFEST_ROWS {
            truncated = true;
            break;
        }
        rows_checked += 1;
        for word in [&entry.path, &entry.relpath] {
            let normalized = word.replace('\\', "/");
            if is_credential_surface(Path::new(""), Path::new(&normalized)) {
                violations += 1;
                push_line(
                    &mut lines,
                    format!("违规: 清单行命中凭据面词表 (凭据不得随备份外带): {normalized}"),
                );
            }
        }
    }

    // 面二: 包内文件名 (清单可能说谎, 文件名是第二双眼)。
    let files_root = bundle.join("files");
    let walk = walk_entries(&files_root);
    let mut bundle_words: Vec<String> = Vec::new();
    for (_, rel, is_dir) in &walk.entries {
        if !*is_dir {
            bundle_words.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    // 面三: 显式排除行逐件核对「真的没进包」。
    let mut excluded_checked = 0usize;
    for excluded in &manifest.excluded {
        if excluded_checked >= MAX_MANIFEST_ROWS {
            truncated = true;
            break;
        }
        excluded_checked += 1;
        let norm = excluded.path.replace('\\', "/");
        let leaked = manifest.files.iter().any(|entry| {
            let rel = entry.relpath.replace('\\', "/");
            let path = entry.path.replace('\\', "/");
            rel == norm
                || path == norm
                || rel.ends_with(&format!("/{norm}"))
                || path.ends_with(&format!("/{norm}"))
        }) || bundle_words
            .iter()
            .any(|word| word == &norm || word.ends_with(&format!("/{norm}")));
        if leaked {
            violations += 1;
            push_line(
                &mut lines,
                format!("违规: 显式排除项出现在包内 (排除记账说谎): {norm}"),
            );
        }
    }

    let checked = rows_checked + excluded_checked;
    if violations > lines.len() {
        truncated = true;
    }
    DeepEvidence {
        key,
        kind: DeepEvidenceKind::ManifestAudit,
        summary: format!(
            "逐件核对: 清单 {rows_checked} 行 + 显式排除 {excluded_checked} 条 (共 {checked} 件), 违规 {violations} 件; 只读清单与文件名, 不打开包内内容"
        ),
        lines,
        truncated,
    }
}

/// 凭据面 · 路径元数据取证: 只取 stat 面 (存在/字节/目录), **内容零读取**
/// (V5-b 硬语义: 绝不打开凭据件)。
fn path_metadata(root: &Path, candidate: &Candidate) -> DeepEvidence {
    let key = candidate.key.clone();
    let path = root.join(&candidate.surface);
    match std::fs::metadata(&path) {
        Ok(meta) => {
            let kind = if meta.is_dir() { "目录" } else { "文件" };
            DeepEvidence {
                key,
                kind: DeepEvidenceKind::PathMetadata,
                summary: "路径元数据取证 (凭据面内容零读取)".to_string(),
                lines: vec![format!(
                    "{}: 存在, {} 字节, {kind}; 内容零读取",
                    candidate.surface,
                    meta.len()
                )],
                truncated: false,
            }
        }
        Err(error) => DeepEvidence::note(key, format!("路径元数据取证失败: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exposure_audit::report::{ExposureItem, Finding};

    fn report_with(items: Vec<ExposureItem>) -> ExposureReport {
        ExposureReport::new(0, Path::new("/data"), items)
    }

    #[test]
    fn candidates_carry_stable_keys_and_dedupe_to_max_risk() {
        let report = report_with(vec![ExposureItem::measured(
            "plaintext_points",
            "明文风险点",
            vec![
                Finding::new("a.toml", RiskLevel::Medium, "d1"),
                Finding::new("a.toml", RiskLevel::High, "d2"),
            ],
            "advice",
        )]);
        let candidates = Candidate::from_report(&report);
        assert_eq!(candidates.len(), 1, "同键候选去重");
        assert_eq!(candidates[0].key, "plaintext_points|a.toml");
        assert_eq!(candidates[0].risk, RiskLevel::High, "同键取最高风险");
    }

    #[test]
    fn plaintext_forensics_redacts_field_values_and_reports_changed_lines() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("app.toml"),
            "name = \"apeireth\"\napi_key = \"sk-raw-value-42\"\n",
        )
        .unwrap();
        let candidate = Candidate::new("plaintext_points", "app.toml", RiskLevel::Medium, "d");
        let evidence = plaintext_forensics(dir.path(), &candidate);
        assert_eq!(evidence.kind, DeepEvidenceKind::PlaintextFields);
        let rendered = evidence.lines.join("\n");
        assert!(rendered.contains(REDACTED), "{rendered}");
        assert!(
            !rendered.contains("sk-raw-value-42"),
            "取证行零明文: {rendered}"
        );
        assert!(!evidence.summary.contains("sk-raw-value-42"));
    }

    #[test]
    fn path_metadata_never_reads_credential_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("creds.json"), r#"{"token":"raw-7"}"#).unwrap();
        let candidate = Candidate::new("credential_files", "creds.json", RiskLevel::Medium, "d");
        let evidence = path_metadata(dir.path(), &candidate);
        let rendered = format!("{} {}", evidence.summary, evidence.lines.join("\n"));
        assert!(rendered.contains("零读取"), "{rendered}");
        assert!(!rendered.contains("raw-7"), "元数据取证零内容: {rendered}");
    }

    #[test]
    fn risk_words_roundtrip_and_unknown_word_is_none() {
        for word in ["none", "low", "medium", "high"] {
            let risk = risk_from_word(word).expect("known word");
            assert_eq!(risk.as_str(), word);
        }
        assert_eq!(risk_from_word("bogus"), None);
    }
}
