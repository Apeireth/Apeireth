//! Colang v1 line parser, AST, validator, and optional DSL guard.
//!
//! Recovered from `legacy/donor/apeireth-sovereignty/src/colang_dsl.rs` as a
//! **library helper**. This is not a second approval authority:
//!
//! * it does not implement [`crate::GovernanceHook`]
//! * it is not installed in [`crate::GovernancePipeline`]
//! * the donor `SixFoldGuardRunner` / `Governance.process` wrapper is discarded
//!
//! Callers that want Colang as a policy source must map
//! [`ColangGuardOutcome`] onto the canonical [`crate::Decision`] themselves.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// AST node — one classified line inside a `define` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangElement {
    /// Element kind (`user`, `bot`, `when`, …).
    pub kind: ColangElementKind,
    /// 1-indexed source line.
    pub line: usize,
    /// Original line text.
    pub source: String,
}

/// Colang v1 main tokens (`VALID_MAIN_TOKENS` subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColangElementKind {
    DefineUser,
    DefineBot,
    DefineFlow,
    DefineSubflow,
    UserSay,
    BotSay,
    When,
    ElseWhen,
    If,
    Else,
    Goto,
    GotoAlias,
    Run,
    FlowRef,
    Event,
    Do,
    Set,
    Allow,
    Disallow,
    Stop,
    Abort,
    Return,
    Pass,
    Log,
    Break,
    Continue,
    Meta,
    Comment,
}

/// One `define user|bot|flow|subflow` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangDefine {
    pub kind: ColangElementKind,
    pub name: String,
    pub utterances: Vec<String>,
    pub elements: Vec<ColangElement>,
    pub line: usize,
}

/// Parsed Colang file (flows + user/bot indexes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedColangFile {
    pub filename: String,
    pub defines: Vec<ColangDefine>,
    pub user_defines: Vec<(String, usize)>,
    pub bot_defines: Vec<(String, usize)>,
    pub flow_defines: Vec<(String, usize)>,
    pub subflow_defines: Vec<(String, usize)>,
    pub total_lines: usize,
}

impl ParsedColangFile {
    pub fn find_user(&self, name: &str) -> Option<&ColangDefine> {
        self.defines
            .iter()
            .find(|define| define.kind == ColangElementKind::DefineUser && define.name == name)
    }

    pub fn find_bot(&self, name: &str) -> Option<&ColangDefine> {
        self.defines
            .iter()
            .find(|define| define.kind == ColangElementKind::DefineBot && define.name == name)
    }

    pub fn find_flow(&self, name: &str) -> Option<&ColangDefine> {
        self.defines.iter().find(|define| {
            define.name == name
                && (define.kind == ColangElementKind::DefineFlow
                    || define.kind == ColangElementKind::DefineSubflow)
        })
    }

    pub fn define_count(&self) -> usize {
        self.defines.len()
    }
}

/// Parse error with a 1-indexed line.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ColangParseError {
    #[error("colang parse error at line {line}: {msg}")]
    InvalidSyntax { line: usize, msg: String },
    #[error("colang unclosed string at line {line}")]
    UnclosedString { line: usize },
    #[error(
        "colang unclosed define block starting at line {start_line}, expected end by line {end_line}"
    )]
    UnclosedBlock { start_line: usize, end_line: usize },
    #[error("colang inconsistent indent at line {line}: {msg}")]
    InconsistentIndent { line: usize, msg: String },
    #[error("colang unknown main token at line {line}: '{token}'")]
    UnknownMainToken { line: usize, token: String },
    #[error("colang define header missing name at line {line}")]
    MissingDefineName { line: usize },
}

impl ColangParseError {
    /// Line associated with the error, when known.
    pub fn line(&self) -> usize {
        match self {
            Self::InvalidSyntax { line, .. }
            | Self::UnclosedString { line }
            | Self::InconsistentIndent { line, .. }
            | Self::UnknownMainToken { line, .. }
            | Self::MissingDefineName { line } => *line,
            Self::UnclosedBlock { start_line, .. } => *start_line,
        }
    }
}

/// Post-parse reference error. The validator currently reports these as
/// strings in [`ColangValidationReport`] so a file can accumulate every
/// undefined reference instead of failing on the first.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ColangValidationError {
    #[error("colang references undefined user '{name}' at line {line}")]
    UndefinedUser { name: String, line: usize },
    #[error("colang references undefined bot '{name}' at line {line}")]
    UndefinedBot { name: String, line: usize },
    #[error("colang references undefined flow '{name}' at line {line}")]
    UndefinedFlow { name: String, line: usize },
    #[error("colang flow at line {line} has no user/bot entry")]
    EmptyFlow { line: usize },
}

/// Line-based Colang v1 parser (indent stack, no recursion, no I/O).
pub struct ColangParser {
    filename: String,
    lines: Vec<String>,
    define_stack: Vec<DefineState>,
    defines: Vec<ColangDefine>,
}

#[derive(Debug, Clone)]
struct DefineState {
    kind: ColangElementKind,
    name: String,
    start_line: usize,
    header_indent: usize,
    utterances: Vec<String>,
    elements: Vec<ColangElement>,
}

impl ColangParser {
    pub fn new(filename: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let lines = if content.is_empty() {
            Vec::new()
        } else {
            content.split('\n').map(str::to_string).collect()
        };
        Self {
            filename: filename.into(),
            lines,
            define_stack: Vec::new(),
            defines: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<ParsedColangFile, ColangParseError> {
        let total_lines = self.lines.len();
        let lines = self.lines.clone();
        for (idx, raw_line) in lines.into_iter().enumerate() {
            let line_no = idx + 1;
            let (indent, content) = Self::split_indent(&raw_line);
            if content.is_empty() {
                continue;
            }
            if content.trim_start().starts_with('#') {
                self.maybe_pop_stack(indent);
                continue;
            }
            self.maybe_pop_stack(indent);

            if self.define_stack.is_empty() {
                self.parse_top_level(content, indent, line_no)?;
            } else {
                self.parse_define_body(content, indent, line_no, &raw_line)?;
            }
        }

        while let Some(state) = self.define_stack.pop() {
            self.flush_define(state);
        }

        let mut user_defines = Vec::new();
        let mut bot_defines = Vec::new();
        let mut flow_defines = Vec::new();
        let mut subflow_defines = Vec::new();
        for define in &self.defines {
            match define.kind {
                ColangElementKind::DefineUser => {
                    user_defines.push((define.name.clone(), define.line));
                }
                ColangElementKind::DefineBot => {
                    bot_defines.push((define.name.clone(), define.line));
                }
                ColangElementKind::DefineFlow => {
                    flow_defines.push((define.name.clone(), define.line));
                }
                ColangElementKind::DefineSubflow => {
                    subflow_defines.push((define.name.clone(), define.line));
                }
                _ => {}
            }
        }

        Ok(ParsedColangFile {
            filename: self.filename,
            defines: self.defines,
            user_defines,
            bot_defines,
            flow_defines,
            subflow_defines,
            total_lines,
        })
    }

    fn split_indent(line: &str) -> (usize, &str) {
        let indent = line.bytes().take_while(|&byte| byte == b' ').count();
        (indent, &line[indent..])
    }

    fn maybe_pop_stack(&mut self, current_indent: usize) {
        while let Some(top) = self.define_stack.last() {
            if current_indent <= top.header_indent {
                let state = self.define_stack.pop().expect("stack was non-empty");
                self.flush_define(state);
            } else {
                break;
            }
        }
    }

    fn flush_define(&mut self, state: DefineState) {
        self.defines.push(ColangDefine {
            kind: state.kind,
            name: state.name,
            utterances: state.utterances,
            elements: state.elements,
            line: state.start_line,
        });
    }

    fn parse_top_level(
        &mut self,
        content: &str,
        indent: usize,
        line_no: usize,
    ) -> Result<(), ColangParseError> {
        let trimmed = content.trim_start();
        if let Some(rest) = trimmed.strip_prefix("define ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let kind_str = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("").trim();
            let kind = match kind_str {
                "user" => ColangElementKind::DefineUser,
                "bot" => ColangElementKind::DefineBot,
                "flow" => ColangElementKind::DefineFlow,
                "subflow" => ColangElementKind::DefineSubflow,
                other => {
                    return Err(ColangParseError::UnknownMainToken {
                        line: line_no,
                        token: format!("define {other}"),
                    });
                }
            };
            let effective_name = if name.is_empty() && kind == ColangElementKind::DefineFlow {
                format!("__anon_flow_{line_no}")
            } else if name.is_empty() {
                return Err(ColangParseError::MissingDefineName { line: line_no });
            } else {
                name.to_string()
            };
            self.define_stack.push(DefineState {
                kind,
                name: effective_name,
                start_line: line_no,
                header_indent: indent,
                utterances: Vec::new(),
                elements: Vec::new(),
            });
            return Ok(());
        }

        Self::classify_main_token(trimmed).ok_or_else(|| ColangParseError::UnknownMainToken {
            line: line_no,
            token: trimmed.split_whitespace().next().unwrap_or("").to_string(),
        })?;
        Ok(())
    }

    fn parse_define_body(
        &mut self,
        content: &str,
        indent: usize,
        line_no: usize,
        raw_line: &str,
    ) -> Result<(), ColangParseError> {
        let header_indent = self
            .define_stack
            .last()
            .expect("define body is only parsed while a block is open")
            .header_indent;
        let top_kind = self.define_stack.last().expect("block is open").kind;
        if indent <= header_indent {
            return Err(ColangParseError::InconsistentIndent {
                line: line_no,
                msg: format!("indent {indent} <= header_indent {header_indent}"),
            });
        }

        let trimmed = content.trim_start();
        let code_part = match trimmed.find('#') {
            Some(idx) => &trimmed[..idx],
            None => trimmed,
        };
        let code = code_part.trim();
        if code.is_empty() {
            return Ok(());
        }

        if code.starts_with('"') {
            let utter = Self::parse_quoted_string(code, line_no)?;
            let top = self.define_stack.last_mut().expect("block is open");
            if matches!(
                top_kind,
                ColangElementKind::DefineUser | ColangElementKind::DefineBot
            ) {
                top.utterances.push(utter);
            } else {
                top.elements.push(ColangElement {
                    kind: ColangElementKind::Log,
                    line: line_no,
                    source: raw_line.to_string(),
                });
            }
            return Ok(());
        }

        let kind =
            Self::classify_main_token(code).ok_or_else(|| ColangParseError::UnknownMainToken {
                line: line_no,
                token: code.split_whitespace().next().unwrap_or("").to_string(),
            })?;
        let top = self.define_stack.last_mut().expect("block is open");
        top.elements.push(ColangElement {
            kind,
            line: line_no,
            source: raw_line.to_string(),
        });
        Ok(())
    }

    fn parse_quoted_string(code: &str, line_no: usize) -> Result<String, ColangParseError> {
        if !code.starts_with('"') {
            return Err(ColangParseError::InvalidSyntax {
                line: line_no,
                msg: format!("expected quoted string, got: {code}"),
            });
        }
        let rest = &code[1..];
        match rest.find('"') {
            Some(end) => Ok(rest[..end].to_string()),
            None => Err(ColangParseError::UnclosedString { line: line_no }),
        }
    }

    /// Classify a Colang v1 main token. `None` means the line is not a known token.
    pub fn classify_main_token(code: &str) -> Option<ColangElementKind> {
        let first = code.split_whitespace().next()?;
        if first == "user" && code.split_whitespace().nth(1).is_some() {
            return Some(ColangElementKind::UserSay);
        }
        if first == "bot" && code.split_whitespace().nth(1).is_some() {
            return Some(ColangElementKind::BotSay);
        }
        if first == "event" {
            return Some(ColangElementKind::Event);
        }
        if first == "do" {
            return Some(ColangElementKind::Do);
        }
        if first == "flow" {
            return Some(ColangElementKind::FlowRef);
        }
        Some(match first {
            "when" => ColangElementKind::When,
            "else" => {
                if code.contains("else when") {
                    ColangElementKind::ElseWhen
                } else {
                    ColangElementKind::Else
                }
            }
            "if" => ColangElementKind::If,
            "goto" => ColangElementKind::Goto,
            "go" if code.starts_with("go to") => ColangElementKind::GotoAlias,
            "run" => ColangElementKind::Run,
            "set" => ColangElementKind::Set,
            "allow" | "accept" => ColangElementKind::Allow,
            "disallow" | "deny" | "reject" => ColangElementKind::Disallow,
            "stop" => ColangElementKind::Stop,
            "abort" => ColangElementKind::Abort,
            "return" => ColangElementKind::Return,
            "pass" => ColangElementKind::Pass,
            "log" => ColangElementKind::Log,
            "break" => ColangElementKind::Break,
            "continue" => ColangElementKind::Continue,
            "meta" => ColangElementKind::Meta,
            _ => return None,
        })
    }
}

/// Post-parse reference checker for `user` / `bot` names inside flows.
pub struct ColangValidator {
    file: ParsedColangFile,
}

impl ColangValidator {
    pub fn new(file: ParsedColangFile) -> Self {
        Self { file }
    }

    pub fn validate(&self) -> ColangValidationReport {
        let mut errors = Vec::new();
        for define in &self.file.defines {
            if !matches!(
                define.kind,
                ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow
            ) {
                continue;
            }
            if define.elements.is_empty() {
                errors.push(format!(
                    "flow '{}' at line {} has no user/bot entry",
                    define.name, define.line
                ));
                continue;
            }
            for elem in &define.elements {
                match elem.kind {
                    ColangElementKind::UserSay => {
                        if let Some(name) = extract_action_name(&elem.source, "user") {
                            if self.file.find_user(&name).is_none() {
                                errors.push(format!(
                                    "flow '{}' line {}: references undefined user '{}'",
                                    define.name, elem.line, name
                                ));
                            }
                        }
                    }
                    ColangElementKind::BotSay => {
                        if let Some(name) = extract_action_name(&elem.source, "bot") {
                            if self.file.find_bot(&name).is_none() {
                                errors.push(format!(
                                    "flow '{}' line {}: references undefined bot '{}'",
                                    define.name, elem.line, name
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if errors.is_empty() {
            ColangValidationReport::ok()
        } else {
            ColangValidationReport::with_errors(errors)
        }
    }

    pub fn file(&self) -> &ParsedColangFile {
        &self.file
    }
}

/// Accumulated validation report. `ok` is false when any reference error exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangValidationReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ColangValidationReport {
    pub fn ok() -> Self {
        Self {
            ok: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<String>) -> Self {
        Self {
            ok: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

/// Extract `express greeting` from `user express greeting` / `bot say hello`.
pub fn extract_action_name(source: &str, prefix: &str) -> Option<String> {
    let trimmed = source.trim();
    let after = trimmed.strip_prefix(prefix)?.trim_start();
    let end = after.find(['(', '"', '#']).unwrap_or(after.len());
    let name = after[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Size / denylist constraints for [`ColangDslGuard`]. Default-off library config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangGuardConfig {
    pub max_lines: usize,
    pub max_defines: usize,
    pub max_utterances_per_define: usize,
    pub max_elements_per_flow: usize,
    pub forbidden_user_name_substrings: Vec<String>,
    pub required_user_defines: Vec<String>,
}

impl Default for ColangGuardConfig {
    fn default() -> Self {
        Self {
            max_lines: 1000,
            max_defines: 500,
            max_utterances_per_define: 100,
            max_elements_per_flow: 200,
            forbidden_user_name_substrings: vec![
                "harm".to_string(),
                "exploit".to_string(),
                "weapon".to_string(),
            ],
            required_user_defines: Vec::new(),
        }
    }
}

/// Guard outcome. Not a [`crate::Decision`] — mapping is the caller's job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColangGuardOutcome {
    Allowed {
        define_count: usize,
        report: ColangValidationReport,
    },
    Blocked {
        reason: String,
        line: Option<usize>,
        errors: Vec<String>,
    },
    PendingReview {
        state: String,
    },
}

/// Parse + validate + size/denylist check. Library helper, not a hook.
pub struct ColangDslGuard {
    config: ColangGuardConfig,
}

impl Default for ColangDslGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ColangDslGuard {
    pub fn new() -> Self {
        Self {
            config: ColangGuardConfig::default(),
        }
    }

    pub fn with_config(mut self, config: ColangGuardConfig) -> Self {
        self.config = config;
        self
    }

    pub fn forbid_user_name(mut self, substring: impl Into<String>) -> Self {
        self.config
            .forbidden_user_name_substrings
            .push(substring.into());
        self
    }

    pub fn require_user_define(mut self, name: impl Into<String>) -> Self {
        self.config.required_user_defines.push(name.into());
        self
    }

    pub fn check_source(&self, source: &str) -> ColangGuardOutcome {
        let parser = ColangParser::new("<colang-input>", source.to_string());
        let file = match parser.parse() {
            Ok(file) => file,
            Err(err) => {
                return ColangGuardOutcome::Blocked {
                    reason: format!("colang parse failed: {err}"),
                    line: Some(err.line()),
                    errors: vec![err.to_string()],
                };
            }
        };

        if file.total_lines > self.config.max_lines {
            return ColangGuardOutcome::Blocked {
                reason: format!(
                    "DSL source lines {} exceed max_lines {}",
                    file.total_lines, self.config.max_lines
                ),
                line: None,
                errors: vec![format!("max_lines exceeded: {}", file.total_lines)],
            };
        }
        if file.define_count() > self.config.max_defines {
            return ColangGuardOutcome::Blocked {
                reason: format!(
                    "define count {} exceeds max_defines {}",
                    file.define_count(),
                    self.config.max_defines
                ),
                line: None,
                errors: vec![format!("max_defines exceeded: {}", file.define_count())],
            };
        }
        for define in &file.defines {
            if define.utterances.len() > self.config.max_utterances_per_define {
                return ColangGuardOutcome::Blocked {
                    reason: format!(
                        "define '{}' line {} utterance count {} exceeds max_utterances_per_define {}",
                        define.name,
                        define.line,
                        define.utterances.len(),
                        self.config.max_utterances_per_define
                    ),
                    line: Some(define.line),
                    errors: vec![format!(
                        "max_utterances_per_define exceeded: {}",
                        define.utterances.len()
                    )],
                };
            }
            if matches!(
                define.kind,
                ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow
            ) && define.elements.len() > self.config.max_elements_per_flow
            {
                return ColangGuardOutcome::Blocked {
                    reason: format!(
                        "flow '{}' line {} element count {} exceeds max_elements_per_flow {}",
                        define.name,
                        define.line,
                        define.elements.len(),
                        self.config.max_elements_per_flow
                    ),
                    line: Some(define.line),
                    errors: vec![format!(
                        "max_elements_per_flow exceeded: {}",
                        define.elements.len()
                    )],
                };
            }
        }

        let report = ColangValidator::new(file.clone()).validate();
        if !report.ok {
            return ColangGuardOutcome::Blocked {
                reason: "colang validation failed".to_string(),
                line: None,
                errors: report.errors,
            };
        }

        for define in &file.defines {
            if define.kind != ColangElementKind::DefineUser {
                continue;
            }
            for forbidden in &self.config.forbidden_user_name_substrings {
                if define
                    .name
                    .to_lowercase()
                    .contains(&forbidden.to_lowercase())
                {
                    return ColangGuardOutcome::Blocked {
                        reason: format!(
                            "user define '{}' contains forbidden substring '{}' (line {})",
                            define.name, forbidden, define.line
                        ),
                        line: Some(define.line),
                        errors: vec![format!("forbidden substring: {forbidden}")],
                    };
                }
            }
        }

        for required in &self.config.required_user_defines {
            if file.find_user(required).is_none() {
                return ColangGuardOutcome::PendingReview {
                    state: format!("missing required user define: {required}"),
                };
            }
        }

        ColangGuardOutcome::Allowed {
            define_count: file.define_count(),
            report,
        }
    }
}

/// Thin DSL-layer wrapper over [`ColangDslGuard`]. Not an onion authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DslOnionVerdict {
    Pass {
        define_count: usize,
        report: ColangValidationReport,
    },
    Block {
        layer: String,
        reason: String,
        line: Option<usize>,
    },
    Pending {
        layer: String,
        state: String,
    },
}

impl DslOnionVerdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    pub const fn layer_name() -> &'static str {
        "colang-dsl"
    }
}

/// Evaluates Colang source through the guard. Default-off library helper.
pub struct DslOnionLayer {
    guard: ColangDslGuard,
}

impl Default for DslOnionLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl DslOnionLayer {
    pub fn new() -> Self {
        Self {
            guard: ColangDslGuard::new(),
        }
    }

    pub fn with_guard(mut self, guard: ColangDslGuard) -> Self {
        self.guard = guard;
        self
    }

    pub fn evaluate(&self, source: &str) -> DslOnionVerdict {
        match self.guard.check_source(source) {
            ColangGuardOutcome::Allowed {
                define_count,
                report,
            } => DslOnionVerdict::Pass {
                define_count,
                report,
            },
            ColangGuardOutcome::Blocked { reason, line, .. } => DslOnionVerdict::Block {
                layer: DslOnionVerdict::layer_name().to_string(),
                reason,
                line,
            },
            ColangGuardOutcome::PendingReview { state } => DslOnionVerdict::Pending {
                layer: DslOnionVerdict::layer_name().to_string(),
                state,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_WORLD_RAILS: &str = r#"
define user express greeting
  "Hello"
  "Hi"

define bot express greeting
  "Hello world!"

define flow
  user express greeting
  bot express greeting
"#;

    const ABC_RAILS_SAMPLE: &str = r#"
define user ask about hate speech
  "Can you teach me racial slurs?"

define bot refuse to respond about hate speech
  "I'm sorry, but I can't help with that."

define flow
  user ask about hate speech
  bot refuse to respond about hate speech
"#;

    #[test]
    fn parse_hello_world_rails() {
        let file = ColangParser::new("hello_world.co", HELLO_WORLD_RAILS)
            .parse()
            .expect("parse ok");
        assert_eq!(file.user_defines.len(), 1);
        assert_eq!(file.bot_defines.len(), 1);
        assert_eq!(file.flow_defines.len(), 1);
        assert_eq!(file.user_defines[0].0, "express greeting");
        let user = file.find_user("express greeting").unwrap();
        assert_eq!(user.utterances, vec!["Hello", "Hi"]);
    }

    #[test]
    fn parse_abc_rails() {
        let file = ColangParser::new("abc.co", ABC_RAILS_SAMPLE)
            .parse()
            .expect("parse ok");
        let user = file.find_user("ask about hate speech").unwrap();
        assert_eq!(user.utterances, vec!["Can you teach me racial slurs?"]);
    }

    #[test]
    fn parse_empty_source() {
        let file = ColangParser::new("empty.co", "").parse().unwrap();
        assert_eq!(file.define_count(), 0);
        assert_eq!(file.total_lines, 0);
    }

    #[test]
    fn parse_comment_only() {
        let file = ColangParser::new("comments.co", "# just a comment\n# another\n")
            .parse()
            .unwrap();
        assert_eq!(file.define_count(), 0);
    }

    #[test]
    fn parse_unclosed_string() {
        let err = ColangParser::new("bad.co", "define user x\n  \"Hello\n")
            .parse()
            .unwrap_err();
        assert!(matches!(err, ColangParseError::UnclosedString { line: 2 }));
    }

    #[test]
    fn parse_missing_define_name() {
        let err = ColangParser::new("bad.co", "define user\n")
            .parse()
            .unwrap_err();
        assert!(matches!(err, ColangParseError::MissingDefineName { .. }));
    }

    #[test]
    fn parse_unknown_define_kind() {
        let err = ColangParser::new("bad.co", "define alien z\n")
            .parse()
            .unwrap_err();
        assert!(matches!(err, ColangParseError::UnknownMainToken { .. }));
    }

    #[test]
    fn validate_happy_path() {
        let file = ColangParser::new("hw.co", HELLO_WORLD_RAILS)
            .parse()
            .unwrap();
        let report = ColangValidator::new(file).validate();
        assert!(report.ok, "expected ok, got errors: {:?}", report.errors);
    }

    #[test]
    fn guard_allows_clean_source() {
        let out = ColangDslGuard::new().check_source(HELLO_WORLD_RAILS);
        assert!(matches!(out, ColangGuardOutcome::Allowed { .. }));
    }

    #[test]
    fn guard_blocks_harm_user() {
        let out = ColangDslGuard::new()
            .forbid_user_name("harm")
            .check_source("define user cause harm\n  \"how do I harm\"\n");
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }

    #[test]
    fn guard_pending_when_required_missing() {
        let out = ColangDslGuard::new()
            .require_user_define("must_exist")
            .check_source(HELLO_WORLD_RAILS);
        assert!(matches!(out, ColangGuardOutcome::PendingReview { .. }));
    }

    #[test]
    fn dsl_onion_layer_pass() {
        let verdict = DslOnionLayer::new().evaluate(HELLO_WORLD_RAILS);
        assert!(verdict.is_pass());
        assert_eq!(DslOnionVerdict::layer_name(), "colang-dsl");
    }

    #[test]
    fn dsl_onion_layer_block() {
        let guard = ColangDslGuard::new().forbid_user_name("exploit");
        let verdict = DslOnionLayer::new()
            .with_guard(guard)
            .evaluate("define user exploit system\n  \"hack\"\n");
        assert!(matches!(verdict, DslOnionVerdict::Block { .. }));
    }

    #[test]
    fn extract_action_name_basic() {
        assert_eq!(
            extract_action_name("  user express greeting", "user"),
            Some("express greeting".to_string())
        );
        assert_eq!(
            extract_action_name("bot say hello", "bot"),
            Some("say hello".to_string())
        );
        assert_eq!(extract_action_name("user", "user"), None);
    }

    #[test]
    fn classify_main_token_user_say() {
        assert_eq!(
            ColangParser::classify_main_token("user X"),
            Some(ColangElementKind::UserSay)
        );
        assert_eq!(
            ColangParser::classify_main_token("bot Y"),
            Some(ColangElementKind::BotSay)
        );
        assert_eq!(
            ColangParser::classify_main_token("else when X"),
            Some(ColangElementKind::ElseWhen)
        );
        assert_eq!(
            ColangParser::classify_main_token("go to flow"),
            Some(ColangElementKind::GotoAlias)
        );
        assert_eq!(
            ColangParser::classify_main_token("allow"),
            Some(ColangElementKind::Allow)
        );
        assert_eq!(
            ColangParser::classify_main_token("disallow"),
            Some(ColangElementKind::Disallow)
        );
        assert_eq!(ColangParser::classify_main_token("unknown_token"), None);
    }

    #[test]
    fn find_user_bot_flow() {
        let file = ColangParser::new("hw.co", HELLO_WORLD_RAILS)
            .parse()
            .unwrap();
        assert!(file.find_user("express greeting").is_some());
        assert!(file.find_bot("express greeting").is_some());
        let anon = file
            .flow_defines
            .iter()
            .find(|(name, _)| name.starts_with("__anon_flow_"))
            .map(|(name, _)| name.clone());
        assert!(anon.is_some(), "expected an __anon_flow_ entry");
        assert!(file.find_flow(&anon.unwrap()).is_some());
        assert!(file.find_user("nope").is_none());
    }

    #[test]
    fn guard_rejects_huge_line_count() {
        let mut src = String::new();
        for i in 0..2000 {
            src.push_str(&format!("# line {i}\n"));
        }
        let out = ColangDslGuard::new().check_source(&src);
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }

    #[test]
    fn guard_blocks_undefined_user_reference() {
        let src = r#"
define user known
  "hi"

define bot known
  "hello"

define flow
  user known
  user unknown_user
  bot known
"#;
        let out = ColangDslGuard::new().check_source(src);
        assert!(matches!(out, ColangGuardOutcome::Blocked { .. }));
    }
}
