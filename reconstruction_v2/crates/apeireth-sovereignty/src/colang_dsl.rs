//! colang_dsl: NVIDIA NeMo Guardrails Colang DSL Rust 实施

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangElement {
    pub kind: ColangElementKind,
    pub line: usize,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColangElementKind {
    DefineUser, DefineBot, DefineFlow, DefineSubflow,
    UserSay, BotSay,
    When, ElseWhen, If, Else,
    Goto, GotoAlias, Run, FlowRef, Event, Do, Set,
    Allow, Disallow, Stop, Abort, Return, Pass, Log,
    Break, Continue, Meta, Comment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangDefine {
    pub kind: ColangElementKind,
    pub name: String,
    pub utterances: Vec<String>,
    pub elements: Vec<ColangElement>,
    pub line: usize,
}

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
        self.defines.iter().find(|d| d.kind == ColangElementKind::DefineUser && d.name == name)
    }
    pub fn find_bot(&self, name: &str) -> Option<&ColangDefine> {
        self.defines.iter().find(|d| d.kind == ColangElementKind::DefineBot && d.name == name)
    }
    pub fn find_flow(&self, name: &str) -> Option<&ColangDefine> {
        self.defines.iter().find(|d| d.name == name && (d.kind == ColangElementKind::DefineFlow || d.kind == ColangElementKind::DefineSubflow))
    }
    pub fn define_count(&self) -> usize { self.defines.len() }
}

#[derive(Debug, Error)]
pub enum ColangParseError {
    #[error("colang parse error at line {line}: {msg}")]
    InvalidSyntax { line: usize, msg: String },
    #[error("colang unclosed string at line {line}")]
    UnclosedString { line: usize },
    #[error("colang unclosed define block starting at line {start_line}, expected end by line {end_line}")]
    UnclosedBlock { start_line: usize, end_line: usize },
    #[error("colang inconsistent indent at line {line}: {msg}")]
    InconsistentIndent { line: usize, msg: String },
    #[error("colang unknown main token at line {line}: '{token}'")]
    UnknownMainToken { line: usize, token: String },
    #[error("colang define header missing name at line {line}")]
    MissingDefineName { line: usize },
}

#[derive(Debug, Error)]
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

#[derive(Debug, Clone)]
struct DefineState {
    kind: ColangElementKind,
    name: String,
    start_line: usize,
    header_indent: usize,
    utterances: Vec<String>,
    elements: Vec<ColangElement>,
    has_content: bool,
}

pub struct ColangParser {
    filename: String,
    lines: Vec<String>,
    define_stack: Vec<DefineState>,
    defines: Vec<ColangDefine>,
}

impl ColangParser {
    pub fn new(filename: impl Into<String>, content: impl Into<String>) -> Self {
        let content_owned = content.into();
        let lines: Vec<String> = if content_owned.is_empty() {
            Vec::new()
        } else {
            content_owned.split('\n').map(|s| s.to_string()).collect()
        };
        Self { filename: filename.into(), lines, define_stack: Vec::new(), defines: Vec::new() }
    }

    fn split_indent(line: &str) -> (usize, &str) {
        let indent = line.bytes().take_while(|&b| b == b' ').count();
        (indent, &line[indent..])
    }

    fn classify_main_token(code: &str) -> Option<ColangElementKind> {
        let first = code.split_whitespace().next()?;
        if first == "user" && code.split_whitespace().nth(1).is_some() { return Some(ColangElementKind::UserSay); }
        if first == "bot" && code.split_whitespace().nth(1).is_some() { return Some(ColangElementKind::BotSay); }
        if first == "event" { return Some(ColangElementKind::Event); }
        if first == "do" { return Some(ColangElementKind::Do); }
        if first == "flow" { return Some(ColangElementKind::FlowRef); }
        Some(match first {
            "when" => ColangElementKind::When,
            "else" => { if code.contains("else when") { ColangElementKind::ElseWhen } else { ColangElementKind::Else } }
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

    fn parse_quoted_string(&self, code: &str, line_no: usize) -> Result<String, ColangParseError> {
        if !code.starts_with('"') {
            return Err(ColangParseError::InvalidSyntax { line: line_no, msg: format!("expected quoted string, got: {}", code) });
        }
        let rest = &code[1..];
        match rest.find('"') {
            Some(end) => Ok(rest[..end].to_string()),
            None => Err(ColangParseError::UnclosedString { line: line_no }),
        }
    }

    fn maybe_pop_stack(&mut self, current_indent: usize, line_no: usize) {
        while let Some(top) = self.define_stack.last() {
            if current_indent <= top.header_indent {
                let state = self.define_stack.pop().unwrap();
                self.defines.push(ColangDefine { kind: state.kind, name: state.name, utterances: state.utterances, elements: state.elements, line: state.start_line });
            } else {
                break;
            }
        }
        let _ = line_no;
    }

    fn parse_top_level(&mut self, content: &str, indent: usize, line_no: usize, raw_line: &str) -> Result<(), ColangParseError> {
        let trimmed = content.trim_start();
        let _ = indent;
        if let Some(rest) = trimmed.strip_prefix("define ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let kind_str = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("").trim();
            let kind = match kind_str {
                "user" => ColangElementKind::DefineUser,
                "bot" => ColangElementKind::DefineBot,
                "flow" => ColangElementKind::DefineFlow,
                "subflow" => ColangElementKind::DefineSubflow,
                other => return Err(ColangParseError::UnknownMainToken { line: line_no, token: format!("define {}", other) }),
            };
            let effective_name = if name.is_empty() && kind == ColangElementKind::DefineFlow {
                format!("__anon_flow_{}", line_no)
            } else if name.is_empty() {
                return Err(ColangParseError::MissingDefineName { line: line_no });
            } else {
                name.to_string()
            };
            self.define_stack.push(DefineState { kind, name: effective_name, start_line: line_no, header_indent: indent, utterances: Vec::new(), elements: Vec::new(), has_content: false });
            return Ok(());
        }
        let _ = raw_line;
        let kind = Self::classify_main_token(trimmed).ok_or_else(|| ColangParseError::UnknownMainToken {
            line: line_no, token: trimmed.split_whitespace().next().unwrap_or("").to_string(),
        })?;
        let _ = kind;
        Ok(())
    }

    fn parse_define_body(&mut self, content: &str, indent: usize, line_no: usize, raw_line: &str) -> Result<(), ColangParseError> {
        let header_indent = self.define_stack.last().unwrap().header_indent;
        let top_kind = self.define_stack.last().unwrap().kind;
        if indent <= header_indent {
            return Err(ColangParseError::InconsistentIndent { line: line_no, msg: format!("indent {} <= header_indent {}", indent, header_indent) });
        }
        let trimmed = content.trim_start();
        let (code_part, _comment) = match trimmed.find('#') {
            Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
            None => (trimmed, None),
        };
        let code = code_part.trim();
        if code.is_empty() { return Ok(()); }
        if code.starts_with('"') {
            let utter = self.parse_quoted_string(code, line_no)?;
            let top = self.define_stack.last_mut().unwrap();
            if matches!(top_kind, ColangElementKind::DefineUser | ColangElementKind::DefineBot) {
                top.utterances.push(utter);
                top.has_content = true;
            } else {
                top.elements.push(ColangElement { kind: ColangElementKind::Log, line: line_no, source: raw_line.to_string() });
            }
            return Ok(());
        }
        let kind = Self::classify_main_token(code).ok_or_else(|| ColangParseError::UnknownMainToken {
            line: line_no, token: code.split_whitespace().next().unwrap_or("").to_string(),
        })?;
        let top = self.define_stack.last_mut().unwrap();
        top.elements.push(ColangElement { kind, line: line_no, source: raw_line.to_string() });
        top.has_content = true;
        Ok(())
    }

    pub fn parse(mut self) -> Result<ParsedColangFile, ColangParseError> {
        let total_lines = self.lines.len();
        let lines_owned: Vec<String> = self.lines.clone();
        for (idx, raw_line) in lines_owned.into_iter().enumerate() {
            let line_no = idx + 1;
            let (indent, content) = Self::split_indent(&raw_line);
            if content.is_empty() { continue; }
            if content.trim_start().starts_with('#') { self.maybe_pop_stack(indent, line_no); continue; }
            self.maybe_pop_stack(indent, line_no);
            if self.define_stack.is_empty() {
                self.parse_top_level(content, indent, line_no, &raw_line)?;
            } else {
                self.parse_define_body(content, indent, line_no, &raw_line)?;
            }
        }
        while let Some(state) = self.define_stack.pop() {
            self.defines.push(ColangDefine { kind: state.kind, name: state.name, utterances: state.utterances, elements: state.elements, line: state.start_line });
        }
        let mut user_defines = Vec::new();
        let mut bot_defines = Vec::new();
        let mut flow_defines = Vec::new();
        let mut subflow_defines = Vec::new();
        for d in &self.defines {
            match d.kind {
                ColangElementKind::DefineUser => user_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineBot => bot_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineFlow => flow_defines.push((d.name.clone(), d.line)),
                ColangElementKind::DefineSubflow => subflow_defines.push((d.name.clone(), d.line)),
                _ => {}
            }
        }
        Ok(ParsedColangFile { filename: self.filename, defines: self.defines, user_defines, bot_defines, flow_defines, subflow_defines, total_lines })
    }
}

fn extract_action_name(source: &str, prefix: &str) -> Option<String> {
    let trimmed = source.trim();
    let after = trimmed.strip_prefix(prefix)?.trim_start();
    let end = after.find(['(', '"', '#']).unwrap_or(after.len());
    let name = after[..end].trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}

pub struct ColangValidator { file: ParsedColangFile }

impl ColangValidator {
    pub fn new(file: ParsedColangFile) -> Self { Self { file } }
    pub fn validate(&self) -> Result<ColangValidationReport, ColangValidationError> {
        let mut errors = Vec::new();
        for define in &self.file.defines {
            if !matches!(define.kind, ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow) { continue; }
            if define.elements.is_empty() {
                errors.push(format!("flow '{}' at line {} has no user/bot entry", define.name, define.line));
                continue;
            }
            for elem in &define.elements {
                match elem.kind {
                    ColangElementKind::UserSay => {
                        if let Some(n) = extract_action_name(&elem.source, "user") {
                            if self.file.find_user(&n).is_none() {
                                errors.push(format!("flow '{}' line {}: references undefined user '{}'", define.name, elem.line, n));
                            }
                        }
                    }
                    ColangElementKind::BotSay => {
                        if let Some(n) = extract_action_name(&elem.source, "bot") {
                            if self.file.find_bot(&n).is_none() {
                                errors.push(format!("flow '{}' line {}: references undefined bot '{}'", define.name, elem.line, n));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if errors.is_empty() { Ok(ColangValidationReport::ok()) } else { Ok(ColangValidationReport::with_errors(errors)) }
    }
    pub fn file(&self) -> &ParsedColangFile { &self.file }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColangValidationReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ColangValidationReport {
    pub fn ok() -> Self { Self { ok: true, errors: Vec::new(), warnings: Vec::new() } }
    pub fn with_errors(errors: Vec<String>) -> Self { Self { ok: false, errors, warnings: Vec::new() } }
}

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
            max_lines: 1000, max_defines: 500, max_utterances_per_define: 100, max_elements_per_flow: 200,
            forbidden_user_name_substrings: vec!["harm".to_string(), "exploit".to_string(), "weapon".to_string()],
            required_user_defines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColangGuardOutcome {
    Allowed { define_count: usize, report: ColangValidationReport },
    Blocked { reason: String, line: Option<usize>, errors: Vec<String> },
    PendingReview { state: String },
}

pub struct ColangDslGuard { pub config: ColangGuardConfig }

impl Default for ColangDslGuard { fn default() -> Self { Self::new() } }

impl ColangDslGuard {
    pub fn new() -> Self { Self { config: ColangGuardConfig::default() } }
    pub fn with_config(mut self, config: ColangGuardConfig) -> Self { self.config = config; self }
    pub fn forbid_user_name(mut self, substring: impl Into<String>) -> Self {
        self.config.forbidden_user_name_substrings.push(substring.into()); self
    }
    pub fn require_user_define(mut self, name: impl Into<String>) -> Self {
        self.config.required_user_defines.push(name.into()); self
    }
    pub fn check_source(&self, source: &str) -> ColangGuardOutcome {
        let parser = ColangParser::new("<guard-6-input>", source.to_string());
        let file = match parser.parse() {
            Ok(f) => f,
            Err(e) => {
                return ColangGuardOutcome::Blocked {
                    reason: format!("colang parse failed: {}", e),
                    line: match &e {
                        ColangParseError::InvalidSyntax { line, .. }
                        | ColangParseError::UnclosedString { line }
                        | ColangParseError::InconsistentIndent { line, .. }
                        | ColangParseError::UnknownMainToken { line, .. }
                        | ColangParseError::MissingDefineName { line } => Some(*line),
                        ColangParseError::UnclosedBlock { start_line, .. } => Some(*start_line),
                    },
                    errors: vec![e.to_string()],
                };
            }
        };
        if file.total_lines > self.config.max_lines {
            return ColangGuardOutcome::Blocked { reason: format!("max_lines exceeded: {}", file.total_lines), line: None, errors: vec![format!("max_lines: {}", file.total_lines)] };
        }
        if file.define_count() > self.config.max_defines {
            return ColangGuardOutcome::Blocked { reason: format!("max_defines exceeded: {}", file.define_count()), line: None, errors: vec![format!("max_defines: {}", file.define_count())] };
        }
        for d in &file.defines {
            if d.utterances.len() > self.config.max_utterances_per_define {
                return ColangGuardOutcome::Blocked { reason: format!("max_utterances_per_define exceeded: {}", d.utterances.len()), line: Some(d.line), errors: vec!["max_utterances_per_define".into()] };
            }
            if matches!(d.kind, ColangElementKind::DefineFlow | ColangElementKind::DefineSubflow) && d.elements.len() > self.config.max_elements_per_flow {
                return ColangGuardOutcome::Blocked { reason: format!("max_elements_per_flow exceeded: {}", d.elements.len()), line: Some(d.line), errors: vec!["max_elements_per_flow".into()] };
            }
        }
        let validator = ColangValidator::new(file.clone());
        let report = match validator.validate() {
            Ok(r) => r,
            Err(e) => { return ColangGuardOutcome::Blocked { reason: format!("validation error: {}", e), line: None, errors: vec![e.to_string()] }; }
        };
        if !report.ok {
            return ColangGuardOutcome::Blocked { reason: "colang validation failed".to_string(), line: None, errors: report.errors.clone() };
        }
        for d in &file.defines {
            if d.kind != ColangElementKind::DefineUser { continue; }
            for forbidden in &self.config.forbidden_user_name_substrings {
                if d.name.to_lowercase().contains(&forbidden.to_lowercase()) {
                    return ColangGuardOutcome::Blocked { reason: format!("forbidden substring '{}' in user define '{}'", forbidden, d.name), line: Some(d.line), errors: vec![format!("forbidden: {}", forbidden)] };
                }
            }
        }
        for required in &self.config.required_user_defines {
            if file.find_user(required).is_none() {
                return ColangGuardOutcome::PendingReview { state: format!("missing required user define: {}", required) };
            }
        }
        ColangGuardOutcome::Allowed { define_count: file.define_count(), report }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DslOnionVerdict {
    Pass { define_count: usize, report: ColangValidationReport },
    Block { layer: String, reason: String, line: Option<usize> },
    Pending { layer: String, state: String },
}

impl DslOnionVerdict {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass { .. }) }
    pub fn layer_name(&self) -> &'static str { "colang-dsl" }
}

pub struct DslOnionLayer { pub guard: ColangDslGuard }
impl Default for DslOnionLayer { fn default() -> Self { Self::new() } }

impl DslOnionLayer {
    pub fn new() -> Self { Self { guard: ColangDslGuard::new() } }
    pub fn with_guard(mut self, guard: ColangDslGuard) -> Self { self.guard = guard; self }
    pub fn layer_name(&self) -> &'static str { "colang-dsl" }
    pub fn evaluate(&self, source: &str) -> DslOnionVerdict {
        match self.guard.check_source(source) {
            ColangGuardOutcome::Allowed { define_count, report } => DslOnionVerdict::Pass { define_count, report },
            ColangGuardOutcome::Blocked { reason, line, .. } => DslOnionVerdict::Block { layer: self.layer_name().to_string(), reason, line },
            ColangGuardOutcome::PendingReview { state } => DslOnionVerdict::Pending { layer: self.layer_name().to_string(), state },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HELLO_WORLD: &str = "\ndefine user express greeting\n  \"Hello\"\n  \"Hi\"\n\ndefine bot express greeting\n  \"Hello world!\"\n\ndefine flow\n  user express greeting\n  bot express greeting\n";
    #[test] fn parse_hello_world() {
        let p = ColangParser::new("hw.co", HELLO_WORLD);
        let f = p.parse().unwrap();
        assert_eq!(f.user_defines.len(), 1);
        assert_eq!(f.bot_defines.len(), 1);
        assert_eq!(f.flow_defines.len(), 1);
        assert_eq!(f.user_defines[0].0, "express greeting");
        let u = f.find_user("express greeting").unwrap();
        assert_eq!(u.utterances, vec!["Hello", "Hi"]);
    }
    #[test] fn parse_empty() {
        let p = ColangParser::new("empty.co", "");
        let f = p.parse().unwrap();
        assert_eq!(f.define_count(), 0);
    }
    #[test] fn parse_comments_only() {
        let p = ColangParser::new("c.co", "# a
# b
");
        let f = p.parse().unwrap();
        assert_eq!(f.define_count(), 0);
    }
    #[test] fn parse_unclosed_string() {
        let p = ColangParser::new("bad.co", "define user x\n  \"Hello\n");
        assert!(matches!(p.parse().unwrap_err(), ColangParseError::UnclosedString { line: 2 }));
    }
    #[test] fn parse_missing_define_name() {
        let p = ColangParser::new("bad.co", "define user\n");
        assert!(matches!(p.parse().unwrap_err(), ColangParseError::MissingDefineName { .. }));
    }
    #[test] fn parse_unknown_define_kind() {
        let p = ColangParser::new("bad.co", "define alien z\n");
        assert!(matches!(p.parse().unwrap_err(), ColangParseError::UnknownMainToken { .. }));
    }
    #[test] fn validate_happy_path() {
        let p = ColangParser::new("hw.co", HELLO_WORLD);
        let f = p.parse().unwrap();
        let v = ColangValidator::new(f);
        let r = v.validate().unwrap();
        assert!(r.ok);
    }
    #[test] fn guard_allows_clean() {
        let g = ColangDslGuard::new();
        assert!(matches!(g.check_source(HELLO_WORLD), ColangGuardOutcome::Allowed { .. }));
    }
    #[test] fn guard_blocks_harm() {
        let mut g = ColangDslGuard::new();
        g = g.forbid_user_name("harm");
        let src = "define user cause harm\n  \"x\"\n";
        assert!(matches!(g.check_source(src), ColangGuardOutcome::Blocked { .. }));
    }
    #[test] fn guard_pending_required_missing() {
        let mut g = ColangDslGuard::new();
        g = g.require_user_define("must_exist");
        assert!(matches!(g.check_source(HELLO_WORLD), ColangGuardOutcome::PendingReview { .. }));
    }
    #[test] fn dsl_onion_layer_pass() {
        let l = DslOnionLayer::new();
        let v = l.evaluate(HELLO_WORLD);
        assert!(v.is_pass());
        assert_eq!(v.layer_name(), "colang-dsl");
    }
    #[test] fn dsl_onion_layer_block() {
        let mut g = ColangDslGuard::new();
        g = g.forbid_user_name("exploit");
        let l = DslOnionLayer::new().with_guard(g);
        let src = "define user exploit system\n  \"h\"\n";
        assert!(matches!(l.evaluate(src), DslOnionVerdict::Block { .. }));
    }
    #[test] fn classify_main_token_full() {
        assert_eq!(ColangParser::classify_main_token("user X"), Some(ColangElementKind::UserSay));
        assert_eq!(ColangParser::classify_main_token("bot Y"), Some(ColangElementKind::BotSay));
        assert_eq!(ColangParser::classify_main_token("event Z"), Some(ColangElementKind::Event));
        assert_eq!(ColangParser::classify_main_token("when X"), Some(ColangElementKind::When));
        assert_eq!(ColangParser::classify_main_token("else when X"), Some(ColangElementKind::ElseWhen));
        assert_eq!(ColangParser::classify_main_token("if cond"), Some(ColangElementKind::If));
        assert_eq!(ColangParser::classify_main_token("else"), Some(ColangElementKind::Else));
        assert_eq!(ColangParser::classify_main_token("else if cond"), Some(ColangElementKind::Else));
        assert_eq!(ColangParser::classify_main_token("goto flow"), Some(ColangElementKind::Goto));
        assert_eq!(ColangParser::classify_main_token("go to flow"), Some(ColangElementKind::GotoAlias));
        assert_eq!(ColangParser::classify_main_token("run flow"), Some(ColangElementKind::Run));
        assert_eq!(ColangParser::classify_main_token("allow"), Some(ColangElementKind::Allow));
        assert_eq!(ColangParser::classify_main_token("disallow"), Some(ColangElementKind::Disallow));
        assert_eq!(ColangParser::classify_main_token("stop"), Some(ColangElementKind::Stop));
        assert_eq!(ColangParser::classify_main_token("abort"), Some(ColangElementKind::Abort));
        assert_eq!(ColangParser::classify_main_token("return"), Some(ColangElementKind::Return));
        assert_eq!(ColangParser::classify_main_token("set $x = 1"), Some(ColangElementKind::Set));
        assert_eq!(ColangParser::classify_main_token("nope"), None);
    }
    #[test] fn validation_report_ok_and_with_errors() {
        let r = ColangValidationReport::ok();
        assert!(r.ok);
        assert!(r.errors.is_empty());
        let r2 = ColangValidationReport::with_errors(vec!["e".into()]);
        assert!(!r2.ok);
        assert_eq!(r2.errors.len(), 1);
    }
    #[test] fn guard_blocks_max_lines_exceeded() {
        let mut cfg = ColangGuardConfig::default();
        cfg.max_lines = 0;
        let g = ColangDslGuard::new().with_config(cfg);
        assert!(matches!(g.check_source(HELLO_WORLD), ColangGuardOutcome::Blocked { .. }));
    }
    #[test] fn guard_blocks_max_utterances_exceeded() {
        let mut cfg = ColangGuardConfig::default();
        cfg.max_utterances_per_define = 1;
        let g = ColangDslGuard::new().with_config(cfg);
        assert!(matches!(g.check_source(HELLO_WORLD), ColangGuardOutcome::Blocked { .. }));
    }
    #[test] fn guard_blocks_max_elements_exceeded() {
        let mut cfg = ColangGuardConfig::default();
        cfg.max_elements_per_flow = 1;
        let g = ColangDslGuard::new().with_config(cfg);
        assert!(matches!(g.check_source(HELLO_WORLD), ColangGuardOutcome::Blocked { .. }));
    }
}