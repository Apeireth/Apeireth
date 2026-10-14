//! Policy helpers for the tool-approval **rule engine**.
//!
//! Canonical ownership is unchanged:
//!
//! * [`crate::Decision`] is the only verdict type the runtime consults
//! * [`crate::GovernancePipeline`] is the only hook sequence
//! * `apeireth-runtime::canonical::approval` owns the pending-approval lifecycle
//!
//! This module is **not** an `ApprovalManager` and does not wait on a human
//! channel. It scores a named capability + JSON arguments against Trust / Risk
//! / Frequency / Whitelist / Blacklist / ApprovalList rules and maps the first
//! terminal match onto [`crate::Decision`]. Silent-reject is carried as
//! metadata so a caller can suppress model-facing feedback without changing
//! the deny itself.
//!
//! # 审批族归一 (E-P2, 威胁模型 G2)
//!
//! 命令级名单从**逐字命令**升级为**语义族匹配**（词表与族归一函数见
//! [`crate::approval_family`]）：名单条目的对象是**族**（批"删除族" = 该族
//! 所有形态生效），同族孪生 API 形态（`[System.IO.File]::Delete` / `rm` /
//! 变体）不再逃出名单；跨族仍逐族条目（批删除≠批网络）。存量逐字条目
//! （`Tool:<命令>`）按**迁移语义**解释：命令→族映射后生效（不是失效重批），
//! 词表外命令退回逐字精确匹配（既有语义）。

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::approval_family::{
    interpret_recorded_command, touched_families, ApprovalTarget, CommandFamily,
};
use crate::Decision;

/// Default high-risk capability prefixes (baseline RiskRule).
pub const DEFAULT_HIGH_RISK_PREFIXES: [&str; 3] = ["system", "network", "file"];

/// Default approval window (默认 5 分钟).
pub const APPROVAL_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Frequency window (1 minute).
pub const FREQUENCY_WINDOW_MS: u64 = 60_000;

/// Frequency threshold (3 calls inside the window, including the current one).
pub const FREQUENCY_MAX_CALLS: u32 = 3;

/// 静默拒绝后缀惯例.
pub const SILENT_REJECT_SUFFIX: &str = "::SilentReject";

/// One historical capability dispatch, used by the frequency helper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallRecord {
    pub capability: String,
    pub timestamp_ms: i64,
}

impl CallRecord {
    pub fn new(capability: impl Into<String>, timestamp_ms: i64) -> Self {
        Self {
            capability: capability.into(),
            timestamp_ms,
        }
    }
}

/// 族级条目的显式标记：`Tool:family:<族id>`（族 id 来自
/// [`CommandFamily`] 冻结词表，不手写）。
pub const FAMILY_ENTRY_MARKER: &str = "family:";

/// Parsed `approvalList` entry (`Tool`, `Tool:family:<族id>`, or `Tool:command`,
/// optional silent suffix).
///
/// 条目对象（E-P2 族归一，见 [`ParsedApprovalEntry::target_for`]）：
/// tool 级 = 整个能力；族级 = 族（该族所有形态）；存量逐字命令 = 迁移语义
/// 按族解释（词表外退回逐字对象）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedApprovalEntry {
    pub raw: String,
    pub base: String,
    pub silent: bool,
}

impl ParsedApprovalEntry {
    /// 族级记录构造（族 id 由 [`CommandFamily`] 词表给，杜绝手写 id 打错字）。
    /// `silent` = 静默拒绝后缀语义（与字符串解析面同规则：后缀进 `raw` 不进 `base`）。
    pub fn family_entry(capability: &str, family: CommandFamily, silent: bool) -> Self {
        let base = format!("{capability}:{}{}", FAMILY_ENTRY_MARKER, family.id());
        let raw = if silent {
            format!("{base}{SILENT_REJECT_SUFFIX}")
        } else {
            base.clone()
        };
        Self { raw, base, silent }
    }

    /// 这条记录在给定能力名下的**对象**（E-P2 族归一）：
    ///
    /// * `Tool` → [`ApprovalTarget::Capability`]（整个能力）；
    /// * `Tool:family:<族id>` → [`ApprovalTarget::Families`]（族级对象；
    ///   未知族 id **不给族级语义**——退回逐字对象，宁可无效不放宽）；
    /// * `Tool:<命令>`（存量逐字记录）→ **迁移语义 = 按族解释**：
    ///   命令 → 族映射后生效（[`interpret_recorded_command`]），
    ///   词表外命令退回逐字对象（既有精确匹配语义）。
    pub fn target_for(&self, capability: &str) -> Option<ApprovalTarget> {
        if self.base == capability {
            return Some(ApprovalTarget::Capability);
        }
        let rest = self.base.strip_prefix(capability)?.strip_prefix(':')?;
        if let Some(id) = rest.strip_prefix(FAMILY_ENTRY_MARKER) {
            return Some(match CommandFamily::from_id(id) {
                Some(family) => ApprovalTarget::Families(vec![family]),
                None => ApprovalTarget::Command(rest.to_string()),
            });
        }
        Some(interpret_recorded_command(rest))
    }
}

/// Extract `command` then `command1..N` (numeric ascending) from JSON args.
pub fn extract_commands(args: &Value) -> Vec<String> {
    let mut commands = Vec::new();
    let Some(obj) = args.as_object() else {
        return commands;
    };
    if let Some(command) = obj.get("command").and_then(Value::as_str) {
        let trimmed = command.trim();
        if !trimmed.is_empty() {
            commands.push(trimmed.to_string());
        }
    }
    let mut numbered: Vec<(u64, &str)> = obj
        .iter()
        .filter_map(|(key, value)| {
            let rest = key.strip_prefix("command")?;
            if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let idx: u64 = rest.parse().ok()?;
            let text = value.as_str()?.trim();
            if text.is_empty() {
                return None;
            }
            Some((idx, text))
        })
        .collect();
    numbered.sort_by_key(|(idx, _)| *idx);
    commands.extend(numbered.into_iter().map(|(_, command)| command.to_string()));
    commands
}

/// Parse one approval-list entry. Empty / suffix-only strings are skipped.
pub fn parse_approval_entry(entry: &str) -> Option<ParsedApprovalEntry> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }
    let silent = trimmed.ends_with(SILENT_REJECT_SUFFIX);
    let base = if silent {
        trimmed[..trimmed.len() - SILENT_REJECT_SUFFIX.len()].trim()
    } else {
        trimmed
    };
    if base.is_empty() {
        return None;
    }
    Some(ParsedApprovalEntry {
        raw: trimmed.to_string(),
        base: base.to_string(),
        silent,
    })
}

/// Prefix match used by the baseline RiskRule (`system.exec`, `file_write`, …).
pub fn is_high_risk(capability: &str, prefixes: &[&str]) -> bool {
    let lower = capability.to_lowercase();
    prefixes
        .iter()
        .any(|prefix| lower.starts_with(&prefix.to_lowercase()))
}

/// Count same-capability records inside `(now - window, now]`.
pub fn frequency_count(
    capability: &str,
    history: &[CallRecord],
    now_ms: i64,
    window_ms: u64,
) -> u32 {
    let window_start = now_ms.saturating_sub(window_ms as i64);
    history
        .iter()
        .filter(|record| {
            record.capability == capability
                && record.timestamp_ms >= window_start
                && record.timestamp_ms <= now_ms
        })
        .count() as u32
}

/// Best `approvalList` match: command/family-level (specificity 2) beats
/// tool-level (specificity 1); at equal specificity a silent entry wins.
///
/// 族级匹配（E-P2）：族对象条目对**触达面**相交的命令生效（保守触达、
/// 宁拒不漏——`echo del is a word` 也命中删除族条目，误拒面同写意图口径）；
/// 存量逐字条目按迁移语义归族后走同一族级匹配，词表外命令退回逐字精确匹配。
pub fn best_approval_match<'a>(
    entries: &'a [ParsedApprovalEntry],
    capability: &str,
    commands: &[String],
) -> Option<(&'a ParsedApprovalEntry, Option<String>)> {
    let mut best: Option<(&ParsedApprovalEntry, u8, Option<String>)> = None;
    for entry in entries {
        let Some(target) = entry.target_for(capability) else {
            continue;
        };
        match target {
            ApprovalTarget::Capability => consider(&mut best, entry, 1, None),
            ApprovalTarget::Families(families) => {
                for command in commands {
                    let touched = touched_families(command);
                    if families.iter().any(|family| touched.contains(family)) {
                        consider(&mut best, entry, 2, Some(command.clone()));
                    }
                }
            }
            ApprovalTarget::Command(text) => {
                for command in commands {
                    if *command == text {
                        consider(&mut best, entry, 2, Some(command.clone()));
                    }
                }
            }
        }
    }
    best.map(|(entry, _, command)| (entry, command))
}

fn consider<'a>(
    best: &mut Option<(&'a ParsedApprovalEntry, u8, Option<String>)>,
    entry: &'a ParsedApprovalEntry,
    specificity: u8,
    matched_command: Option<String>,
) {
    let take = match best {
        None => true,
        Some((_, score, _)) if specificity > *score => true,
        Some((best_entry, score, _))
            if specificity == *score && entry.silent && !best_entry.silent =>
        {
            true
        }
        _ => false,
    };
    if take {
        *best = Some((entry, specificity, matched_command));
    }
}

/// Extra annotations produced alongside a canonical [`Decision`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PolicyMatch {
    pub matched_rule: Option<String>,
    pub matched_command: Option<String>,
    /// 命中的族 id（E-P2 审批族归一；族级名单命中时 = 条目族对象的族 id 词表序）。
    /// 空 = 非族级命中（tool 级 / 逐字命令级 / 无命中）。
    #[serde(default)]
    pub matched_family: Vec<String>,
    pub silent: bool,
    pub timeout_ms: Option<u64>,
}

/// One rule-engine evaluation. Terminal rules map to [`Decision`]; unmatched
/// rules fall through. Empty engine = allow (the caller still owns fail-closed
/// via [`crate::DenyUnconfigured`] in the pipeline).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalPolicyEngine {
    pub trusted: BTreeSet<String>,
    pub whitelist: BTreeSet<String>,
    pub blacklist: BTreeSet<String>,
    pub blacklist_silent: bool,
    pub high_risk_prefixes: Vec<String>,
    pub approval_timeout_ms: u64,
    pub frequency_window_ms: u64,
    pub frequency_max_calls: u32,
    pub approval_list: Vec<ParsedApprovalEntry>,
}

impl ApprovalPolicyEngine {
    pub fn new() -> Self {
        Self {
            high_risk_prefixes: DEFAULT_HIGH_RISK_PREFIXES
                .iter()
                .map(|prefix| (*prefix).to_string())
                .collect(),
            approval_timeout_ms: APPROVAL_TIMEOUT_MS,
            frequency_window_ms: FREQUENCY_WINDOW_MS,
            frequency_max_calls: FREQUENCY_MAX_CALLS,
            ..Self::default()
        }
    }

    /// Evaluate in baseline order: Blacklist → Trust → Frequency → Risk →
    /// ApprovalList → Whitelist. First terminal match wins.
    pub fn evaluate(
        &self,
        capability: &str,
        args: &Value,
        history: &[CallRecord],
        now_ms: i64,
    ) -> (Decision, PolicyMatch) {
        if self.blacklist.contains(capability) {
            return (
                Decision::deny(format!("blacklist: {capability}")),
                PolicyMatch {
                    matched_rule: Some("blacklist".into()),
                    silent: self.blacklist_silent,
                    ..PolicyMatch::default()
                },
            );
        }

        if self.trusted.contains(capability) {
            return (
                Decision::Allow,
                PolicyMatch {
                    matched_rule: Some("trust".into()),
                    ..PolicyMatch::default()
                },
            );
        }

        let count = frequency_count(capability, history, now_ms, self.frequency_window_ms);
        if count + 1 >= self.frequency_max_calls && self.frequency_max_calls > 0 {
            return (
                Decision::deny(format!(
                    "frequency exceeded: call {} of {} inside {}ms",
                    count + 1,
                    self.frequency_max_calls,
                    self.frequency_window_ms
                )),
                PolicyMatch {
                    matched_rule: Some("frequency".into()),
                    ..PolicyMatch::default()
                },
            );
        }

        let prefixes: Vec<&str> = self.high_risk_prefixes.iter().map(String::as_str).collect();
        if is_high_risk(capability, &prefixes) {
            return (
                Decision::require_approval(format!(
                    "high-risk capability {capability} requires human approval"
                )),
                PolicyMatch {
                    matched_rule: Some("risk".into()),
                    timeout_ms: Some(self.approval_timeout_ms),
                    ..PolicyMatch::default()
                },
            );
        }

        if !self.approval_list.is_empty() {
            let commands = extract_commands(args);
            if let Some((entry, matched_command)) =
                best_approval_match(&self.approval_list, capability, &commands)
            {
                let matched_family = match entry.target_for(capability) {
                    Some(ApprovalTarget::Families(families)) => families
                        .iter()
                        .map(|family| family.id().to_string())
                        .collect(),
                    _ => Vec::new(),
                };
                return (
                    Decision::require_approval(format!("approval list matched [{}]", entry.raw)),
                    PolicyMatch {
                        matched_rule: Some("approval_list".into()),
                        matched_command,
                        matched_family,
                        silent: entry.silent,
                        timeout_ms: Some(self.approval_timeout_ms),
                    },
                );
            }
        }

        if self.whitelist.contains(capability) {
            return (
                Decision::Allow,
                PolicyMatch {
                    matched_rule: Some("whitelist".into()),
                    ..PolicyMatch::default()
                },
            );
        }

        (Decision::Allow, PolicyMatch::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_commands_reads_command_then_numbered() {
        let args = json!({
            "command": " ls ",
            "command2": "rm -rf /",
            "command1": "whoami",
            "other": "ignored"
        });
        assert_eq!(extract_commands(&args), vec!["ls", "whoami", "rm -rf /"]);
    }

    #[test]
    fn parse_approval_entry_silent_suffix() {
        let entry = parse_approval_entry("shell:rm -rf /::SilentReject").unwrap();
        assert_eq!(entry.base, "shell:rm -rf /");
        assert!(entry.silent);
        assert!(parse_approval_entry("::SilentReject").is_none());
        assert!(parse_approval_entry("  ").is_none());
    }

    #[test]
    fn best_match_prefers_command_then_silent() {
        let entries = vec![
            parse_approval_entry("shell").unwrap(),
            parse_approval_entry("shell:ls::SilentReject").unwrap(),
            parse_approval_entry("shell:ls").unwrap(),
        ];
        let (matched, command) = best_approval_match(&entries, "shell", &["ls".into()]).unwrap();
        assert_eq!(matched.base, "shell:ls");
        assert!(matched.silent);
        assert_eq!(command.as_deref(), Some("ls"));
    }

    #[test]
    fn risk_prefix_is_case_insensitive() {
        assert!(is_high_risk("System.exec", &DEFAULT_HIGH_RISK_PREFIXES));
        assert!(is_high_risk("file_write", &DEFAULT_HIGH_RISK_PREFIXES));
        assert!(!is_high_risk("calculator", &DEFAULT_HIGH_RISK_PREFIXES));
    }

    #[test]
    fn frequency_denies_on_third_call() {
        let engine = ApprovalPolicyEngine::new();
        let now = 1_000_000;
        let history = vec![
            CallRecord::new("spam", now - 100),
            CallRecord::new("spam", now - 50),
        ];
        let (decision, detail) = engine.evaluate("spam", &Value::Null, &history, now);
        assert!(matches!(decision, Decision::Deny { .. }));
        assert_eq!(detail.matched_rule.as_deref(), Some("frequency"));
    }

    #[test]
    fn blacklist_beats_trust() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.blacklist.insert("X".into());
        engine.trusted.insert("X".into());
        engine.blacklist_silent = true;
        let (decision, detail) = engine.evaluate("X", &Value::Null, &[], 0);
        assert!(matches!(decision, Decision::Deny { .. }));
        assert_eq!(detail.matched_rule.as_deref(), Some("blacklist"));
        assert!(detail.silent);
    }

    #[test]
    fn trust_allows_before_risk() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.trusted.insert("system.exec".into());
        let (decision, detail) = engine.evaluate("system.exec", &Value::Null, &[], 0);
        assert!(decision.is_allowed());
        assert_eq!(detail.matched_rule.as_deref(), Some("trust"));
    }

    #[test]
    fn risk_requires_canonical_approval() {
        let engine = ApprovalPolicyEngine::new();
        let (decision, detail) = engine.evaluate("system.exec", &Value::Null, &[], 0);
        assert!(matches!(decision, Decision::RequireApproval { .. }));
        assert_eq!(detail.timeout_ms, Some(APPROVAL_TIMEOUT_MS));
    }

    #[test]
    fn approval_list_command_level() {
        let mut engine = ApprovalPolicyEngine::new();
        engine
            .approval_list
            .push(parse_approval_entry("shell:rm -rf /::SilentReject").unwrap());
        let args = json!({ "command": "rm -rf /" });
        let (decision, detail) = engine.evaluate("shell", &args, &[], 0);
        assert!(matches!(decision, Decision::RequireApproval { .. }));
        assert_eq!(detail.matched_command.as_deref(), Some("rm -rf /"));
        assert!(detail.silent);
    }

    /// E-P2 族归一：族级条目对**该族所有形态**生效 —— 同族孪生 API 形态
    /// 换形重放逃不出名单（G2 封堵），族标注随命中回传。
    #[test]
    fn family_entries_gate_every_form_of_the_family() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.approval_list.push(ParsedApprovalEntry::family_entry(
            "tool.shell",
            CommandFamily::Delete,
            false,
        ));
        for command in [
            "Remove-Item x.txt",
            "rm x.txt",
            "[System.IO.File]::Delete('x.txt')",
            "([System.IO.FileInfo]'x.txt').Delete()",
            "REMOVE-ITEM.EXE x.txt",
            "Remove-I x.txt",
        ] {
            let args = json!({ "command": command });
            let (decision, detail) = engine.evaluate("tool.shell", &args, &[], 0);
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "{command} 同族换形不得逃出族级名单: {decision:?}"
            );
            assert_eq!(detail.matched_family, vec!["删除族"], "{command}");
        }

        // 族级条目的静默后缀语义与字符串面同一规则（后缀进 raw 不进 base）。
        let silent = ParsedApprovalEntry::family_entry("tool.shell", CommandFamily::Network, true);
        assert_eq!(
            parse_approval_entry(&silent.raw).as_ref(),
            Some(&silent),
            "族级条目字符串面/构造面同一形态"
        );
    }

    /// E-P2 跨族仍逐族审批（批删除≠批网络）：删除族条目不覆盖网络族命令，
    /// 网络族条目不覆盖删除族命令。
    #[test]
    fn cross_family_entries_stay_in_their_own_family() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.approval_list.push(ParsedApprovalEntry::family_entry(
            "tool.shell",
            CommandFamily::Delete,
            false,
        ));
        let network = json!({ "command": "Invoke-WebRequest http://example.invalid" });
        let (decision, detail) = engine.evaluate("tool.shell", &network, &[], 0);
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "批删除族不得顺带管住网络族: {decision:?}"
        );
        assert!(detail.matched_family.is_empty(), "{detail:?}");

        let mut network_engine = ApprovalPolicyEngine::new();
        network_engine
            .approval_list
            .push(ParsedApprovalEntry::family_entry(
                "tool.shell",
                CommandFamily::Network,
                false,
            ));
        let delete = json!({ "command": "Remove-Item x.txt" });
        let (decision, _) = network_engine.evaluate("tool.shell", &delete, &[], 0);
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "批网络族不得顺带管住删除族: {decision:?}"
        );
    }

    /// E-P2 存量记录迁移语义（二选一钉死 = **迁移生效**）：老的逐字命令条目
    /// 命令→族映射后生效，同族孪生形态照样被管住（不是失效重批）。
    #[test]
    fn legacy_verbatim_entries_migrate_to_family_interpretation() {
        let mut engine = ApprovalPolicyEngine::new();
        engine
            .approval_list
            .push(parse_approval_entry("tool.shell:Remove-Item x.txt").unwrap());
        let legacy = json!({ "command": "Remove-Item x.txt" });
        let (decision, detail) = engine.evaluate("tool.shell", &legacy, &[], 0);
        assert!(matches!(decision, Decision::RequireApproval { .. }));
        assert_eq!(
            detail.matched_family,
            vec!["删除族"],
            "存量条目迁移为族对象"
        );

        let twin = json!({ "command": "[System.IO.File]::Delete('y.txt')" });
        let (decision, detail) = engine.evaluate("tool.shell", &twin, &[], 0);
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "存量条目按族解释后, 孪生换形必须仍被管住（迁移生效）: {decision:?}"
        );
        assert_eq!(detail.matched_family, vec!["删除族"]);
    }

    /// E-P2 边界：词表外的存量逐字条目退回逐字精确匹配（既有语义不破）；
    /// 未知族 id 的显式族条目**不给族级语义**（宁可无效不放宽，族 id 由
    /// `CommandFamily` 词表 API 给，杜绝手写打错字）。
    #[test]
    fn unattributable_entries_fall_back_to_exact_text_matching() {
        let mut engine = ApprovalPolicyEngine::new();
        engine
            .approval_list
            .push(parse_approval_entry("tool.shell:run-report --weekly").unwrap());
        engine
            .approval_list
            .push(parse_approval_entry("tool.shell:family:不存在族").unwrap());

        let exact = json!({ "command": "run-report --weekly" });
        let (decision, _) = engine.evaluate("tool.shell", &exact, &[], 0);
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "{decision:?}"
        );

        let variant = json!({ "command": "run-report --daily" });
        let (decision, _) = engine.evaluate("tool.shell", &variant, &[], 0);
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "词表外条目不得扩权到别的命令: {decision:?}"
        );

        let unknown_family = json!({ "command": "Remove-Item x.txt" });
        let (decision, detail) = engine.evaluate("tool.shell", &unknown_family, &[], 0);
        assert!(
            !matches!(decision, Decision::RequireApproval { .. }),
            "未知族 id 不得获得族级语义: {decision:?}"
        );
        assert!(detail.matched_family.is_empty());
    }

    #[test]
    fn whitelist_allows_unmatched_low_risk() {
        let mut engine = ApprovalPolicyEngine::new();
        engine.whitelist.insert("calculator".into());
        let (decision, detail) = engine.evaluate("calculator", &Value::Null, &[], 0);
        assert!(decision.is_allowed());
        assert_eq!(detail.matched_rule.as_deref(), Some("whitelist"));
    }

    #[test]
    fn empty_engine_allows() {
        let engine = ApprovalPolicyEngine::new();
        let (decision, detail) = engine.evaluate("calculator", &Value::Null, &[], 0);
        assert!(decision.is_allowed());
        assert!(detail.matched_rule.is_none());
    }
}
