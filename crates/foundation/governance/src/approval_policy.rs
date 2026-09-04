//! Policy helpers recovered from the donor tool-approval **rule engine**.
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

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Decision;

/// Default high-risk capability prefixes (donor RiskRule).
pub const DEFAULT_HIGH_RISK_PREFIXES: [&str; 3] = ["system", "network", "file"];

/// Default approval window (VCP `getTimeoutMs` = 5 minutes).
pub const APPROVAL_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Frequency window (1 minute).
pub const FREQUENCY_WINDOW_MS: u64 = 60_000;

/// Frequency threshold (3 calls inside the window, including the current one).
pub const FREQUENCY_MAX_CALLS: u32 = 3;

/// VCP silent-reject suffix.
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

/// Parsed `approvalList` entry (`Tool` or `Tool:command`, optional silent suffix).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedApprovalEntry {
    pub raw: String,
    pub base: String,
    pub silent: bool,
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

/// Prefix match used by the donor RiskRule (`system.exec`, `file_write`, …).
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

/// Best `approvalList` match: command-level (specificity 2) beats tool-level
/// (specificity 1); at equal specificity a silent entry wins.
pub fn best_approval_match<'a>(
    entries: &'a [ParsedApprovalEntry],
    capability: &str,
    commands: &[String],
) -> Option<(&'a ParsedApprovalEntry, Option<String>)> {
    let mut best: Option<(&ParsedApprovalEntry, u8, Option<String>)> = None;
    for entry in entries {
        if entry.base == capability {
            consider(&mut best, entry, 1, None);
        }
        for command in commands {
            if entry.base == format!("{capability}:{command}") {
                consider(&mut best, entry, 2, Some(command.clone()));
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

    /// Evaluate in donor order: Blacklist → Trust → Frequency → Risk →
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
                return (
                    Decision::require_approval(format!("approval list matched [{}]", entry.raw)),
                    PolicyMatch {
                        matched_rule: Some("approval_list".into()),
                        matched_command,
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
