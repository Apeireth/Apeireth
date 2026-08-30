//! Prompt placeholder universe (VCP messageProcessor paradigm, rewritten).
//!
//! Recovered from `legacy/donor/apeireth-companion/src/prompt_assembler.rs`.
//!
//! - Typed variable sources: identity / state / goals / memory / time / custom.
//! - Privileged expansion: `agent:` / `toolbox:` only expand in system (or
//!   user text that starts with a configurable system marker).
//! - AgentGuard: the whole assembly context expands at most one agent.
//! - ToolboxGuard: each toolbox name expands at most once (first occurrence).
//! - Cycle detection + depth cap: honest error marker, original placeholder
//!   preserved on depth overflow.
//!
//! **Not ported**: `TimeSource` (chrono weekday formatting). Inject a
//! `StaticSource(SourceKind::Time)` if a caller needs `{{time:date}}`.
//! **Not a persona owner**: this expands placeholders; identity text is a
//! registered source, not a second persona module.
//! **Not wired**: default-off library primitive.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::context_assembly::{ContextAssembler, ContextBlock};

/// Message role — decides whether privileged expansion is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Typed variable source kind (`{{<label>:name}}` addressing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Identity,
    State,
    Goals,
    Memory,
    Time,
    Custom,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            SourceKind::Identity => "identity",
            SourceKind::State => "state",
            SourceKind::Goals => "goals",
            SourceKind::Memory => "memory",
            SourceKind::Time => "time",
            SourceKind::Custom => "custom",
        }
    }

    fn from_label(s: &str) -> Option<Self> {
        match s {
            "identity" => Some(Self::Identity),
            "state" => Some(Self::State),
            "goals" => Some(Self::Goals),
            "memory" => Some(Self::Memory),
            "time" => Some(Self::Time),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Typed variable source: resolve a name, or `None` to ask the next source.
pub trait VariableSource: Send + Sync {
    fn kind(&self) -> SourceKind;
    fn resolve(&self, name: &str) -> Option<String>;
}

/// Static BTreeMap source (deterministic resolve order).
#[derive(Debug)]
pub struct StaticSource {
    kind: SourceKind,
    vars: BTreeMap<String, String>,
}

impl StaticSource {
    pub fn new(kind: SourceKind) -> Self {
        Self {
            kind,
            vars: BTreeMap::new(),
        }
    }

    pub fn set(mut self, name: &str, value: impl Into<String>) -> Result<Self, AssemblerError> {
        validate_name(name)?;
        self.vars.insert(name.to_string(), value.into());
        Ok(self)
    }
}

impl VariableSource for StaticSource {
    fn kind(&self) -> SourceKind {
        self.kind
    }
    fn resolve(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

/// Registration-time errors (illegal input rejected at the boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    EmptyName,
    InvalidName(String),
    DuplicateName(String),
    InvalidDepth,
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "变量名不能为空"),
            Self::InvalidName(n) => write!(f, "变量名含非法字符或冒号: {n}"),
            Self::DuplicateName(n) => write!(f, "名字已注册 (重复): {n}"),
            Self::InvalidDepth => write!(f, "最大展开深度必须 >= 1"),
        }
    }
}

impl std::error::Error for AssemblerError {}

/// Observable expansion report (what expanded / was removed / was undefined).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExpansionReport {
    pub expanded: Vec<String>,
    pub removed: Vec<String>,
    pub circular: Vec<String>,
    pub undefined: Vec<String>,
    pub depth_exceeded: Vec<String>,
}

impl ExpansionReport {
    fn merge(&mut self, other: ExpansionReport) {
        self.expanded.extend(other.expanded);
        self.removed.extend(other.removed);
        self.circular.extend(other.circular);
        self.undefined.extend(other.undefined);
        self.depth_exceeded.extend(other.depth_exceeded);
    }
}

/// Cross-text expansion state (one per assembly session).
#[derive(Debug, Default)]
pub struct AssemblyGuard {
    expanded_agent: Option<String>,
    expanded_toolboxes: BTreeSet<String>,
}

impl AssemblyGuard {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn expanded_agent(&self) -> Option<&str> {
        self.expanded_agent.as_deref()
    }
    pub fn expanded_toolboxes(&self) -> &BTreeSet<String> {
        &self.expanded_toolboxes
    }
}

/// Placeholder expansion engine.
pub struct PromptAssembler {
    sources: Vec<Box<dyn VariableSource>>,
    agents: BTreeMap<String, String>,
    toolboxes: BTreeMap<String, String>,
    system_markers: Vec<String>,
    max_depth: usize,
}

impl Default for PromptAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptAssembler {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            agents: BTreeMap::new(),
            toolboxes: BTreeMap::new(),
            system_markers: vec!["[系统提示:]".to_string(), "[系统邀请指令:]".to_string()],
            max_depth: 8,
        }
    }

    pub fn with_system_markers(mut self, markers: Vec<String>) -> Self {
        self.system_markers = markers;
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Result<Self, AssemblerError> {
        if depth == 0 {
            return Err(AssemblerError::InvalidDepth);
        }
        self.max_depth = depth;
        Ok(self)
    }

    pub fn with_source(mut self, source: Box<dyn VariableSource>) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_agent(
        mut self,
        name: &str,
        content: impl Into<String>,
    ) -> Result<Self, AssemblerError> {
        validate_name(name)?;
        if self
            .agents
            .insert(name.to_string(), content.into())
            .is_some()
        {
            return Err(AssemblerError::DuplicateName(name.to_string()));
        }
        Ok(self)
    }

    pub fn with_toolbox(
        mut self,
        name: &str,
        content: impl Into<String>,
    ) -> Result<Self, AssemblerError> {
        validate_name(name)?;
        if self
            .toolboxes
            .insert(name.to_string(), content.into())
            .is_some()
        {
            return Err(AssemblerError::DuplicateName(name.to_string()));
        }
        Ok(self)
    }

    pub fn expand_text(
        &self,
        text: &str,
        role: AssemblyRole,
        guard: &mut AssemblyGuard,
    ) -> (String, ExpansionReport) {
        let mut report = ExpansionReport::default();
        let mut stack: Vec<String> = Vec::new();
        let out = self.expand_inner(text, role, guard, &mut stack, 0, &mut report);
        (out, report)
    }

    pub fn expand_blocks(
        &self,
        blocks: Vec<ContextBlock>,
        role: AssemblyRole,
        guard: &mut AssemblyGuard,
    ) -> (Vec<ContextBlock>, ExpansionReport) {
        let mut report = ExpansionReport::default();
        let mut out = Vec::with_capacity(blocks.len());
        for b in blocks {
            let (content, r) = self.expand_text(&b.content, role, guard);
            report.merge(r);
            out.push(ContextBlock {
                name: b.name,
                content,
                core: b.core,
                cap_chars: b.cap_chars,
            });
        }
        (out, report)
    }

    /// Expand then re-apply `ContextAssembler` budget (core protection + greedy cut).
    pub fn assemble(
        &self,
        assembler: &ContextAssembler,
        role: AssemblyRole,
        guard: &mut AssemblyGuard,
    ) -> (Vec<ContextBlock>, ExpansionReport) {
        let mut report = ExpansionReport::default();
        let mut re = ContextAssembler::new(assembler.total_budget_chars());
        for b in assembler.assemble_budgeted_blocks() {
            let (content, r) = self.expand_text(&b.content, role, guard);
            report.merge(r);
            re = re.push(ContextBlock {
                name: b.name,
                content,
                core: b.core,
                cap_chars: b.cap_chars,
            });
        }
        (re.assemble_budgeted_blocks(), report)
    }

    fn is_privileged(&self, text: &str, role: AssemblyRole) -> bool {
        role == AssemblyRole::System
            || (role == AssemblyRole::User
                && self
                    .system_markers
                    .iter()
                    .any(|m| text.starts_with(m.as_str())))
    }

    fn expand_inner(
        &self,
        text: &str,
        role: AssemblyRole,
        guard: &mut AssemblyGuard,
        stack: &mut Vec<String>,
        depth: usize,
        report: &mut ExpansionReport,
    ) -> String {
        let privileged = self.is_privileged(text, role);
        let mut names: Vec<(Option<String>, String)> = Vec::new();
        for (_, _, p, n) in scan_placeholders(text) {
            let key = (p, n);
            if !names.contains(&key) {
                names.push(key);
            }
        }
        let mut cur = text.to_string();
        for (prefix, name) in names {
            cur = self.expand_one(
                &cur,
                prefix.as_deref(),
                &name,
                privileged,
                role,
                guard,
                stack,
                depth,
                report,
            );
        }
        cur
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_one(
        &self,
        cur: &str,
        prefix: Option<&str>,
        name: &str,
        privileged: bool,
        role: AssemblyRole,
        guard: &mut AssemblyGuard,
        stack: &mut Vec<String>,
        depth: usize,
        report: &mut ExpansionReport,
    ) -> String {
        enum Cat<'a> {
            Agent,
            Toolbox,
            Source(Option<SourceKind>),
            UnknownPrefix(&'a str),
        }
        let cat = match prefix {
            Some("agent") => Cat::Agent,
            Some("toolbox") => Cat::Toolbox,
            Some(p) => match SourceKind::from_label(p) {
                Some(k) => Cat::Source(Some(k)),
                None => Cat::UnknownPrefix(p),
            },
            None => {
                if self.agents.contains_key(name) {
                    Cat::Agent
                } else if self.toolboxes.contains_key(name) {
                    Cat::Toolbox
                } else {
                    Cat::Source(None)
                }
            }
        };

        match cat {
            Cat::Agent => {
                let full = format_full(prefix, name);
                if !self.agents.contains_key(name) {
                    report.undefined.push(full);
                    return cur.to_string();
                }
                if !has_agent_form(cur, name) {
                    return cur.to_string();
                }
                if !privileged {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Agent, "", false).0;
                }
                if guard.expanded_agent.is_some() {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Agent, "", false).0;
                }
                let key = format!("agent:{name}");
                if stack.iter().any(|s| s == &key) {
                    let chain = chain_str(stack, &key);
                    report.circular.push(chain.clone());
                    return replace_forms(
                        cur,
                        name,
                        FormKind::Agent,
                        &format!("[循环变量引用: {chain}]"),
                        false,
                    )
                    .0;
                }
                if depth >= self.max_depth {
                    report.depth_exceeded.push(full);
                    return cur.to_string();
                }
                let content = self.agents.get(name).cloned().unwrap_or_default();
                stack.push(key);
                let expanded = self.expand_inner(&content, role, guard, stack, depth + 1, report);
                stack.pop();
                guard.expanded_agent = Some(name.to_string());
                report.expanded.push(full);
                replace_forms(cur, name, FormKind::Agent, &expanded, false).0
            }
            Cat::Toolbox => {
                let full = format_full(prefix, name);
                if !self.toolboxes.contains_key(name) {
                    report.undefined.push(full);
                    return cur.to_string();
                }
                if !has_toolbox_form(cur, name) {
                    return cur.to_string();
                }
                if !privileged {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Toolbox, "", false).0;
                }
                if guard.expanded_toolboxes.contains(name) {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Toolbox, "", false).0;
                }
                let key = format!("toolbox:{name}");
                if stack.iter().any(|s| s == &key) {
                    let chain = chain_str(stack, &key);
                    report.circular.push(chain.clone());
                    return replace_forms(
                        cur,
                        name,
                        FormKind::Toolbox,
                        &format!("[循环变量引用: {chain}]"),
                        false,
                    )
                    .0;
                }
                if depth >= self.max_depth {
                    report.depth_exceeded.push(full);
                    return cur.to_string();
                }
                let content = self.toolboxes.get(name).cloned().unwrap_or_default();
                stack.push(key);
                let expanded = self.expand_inner(&content, role, guard, stack, depth + 1, report);
                stack.pop();
                guard.expanded_toolboxes.insert(name.to_string());
                report.expanded.push(full.clone());
                let (replaced, dropped) =
                    replace_forms(cur, name, FormKind::Toolbox, &expanded, true);
                report.removed.extend(std::iter::repeat(full).take(dropped));
                replaced
            }
            Cat::UnknownPrefix(p) => {
                report.undefined.push(format!("{p}:{name}"));
                cur.to_string()
            }
            Cat::Source(kind_filter) => {
                let mut value: Option<(String, String)> = None;
                for s in &self.sources {
                    if let Some(k) = kind_filter {
                        if s.kind() != k {
                            continue;
                        }
                    }
                    if let Some(v) = s.resolve(name) {
                        value = Some((format!("{}:{name}", s.kind().label()), v));
                        break;
                    }
                }
                let full = format_full(prefix, name);
                let Some((key, raw)) = value else {
                    report.undefined.push(full);
                    return cur.to_string();
                };
                if stack.iter().any(|s| s == &key) {
                    let chain = chain_str(stack, &key);
                    report.circular.push(chain.clone());
                    return replace_source_forms(
                        cur,
                        prefix,
                        name,
                        &format!("[循环变量引用: {chain}]"),
                    );
                }
                if depth >= self.max_depth {
                    report.depth_exceeded.push(full);
                    return cur.to_string();
                }
                stack.push(key);
                let expanded = self.expand_inner(&raw, role, guard, stack, depth + 1, report);
                stack.pop();
                report.expanded.push(full);
                replace_source_forms(cur, prefix, name, &expanded)
            }
        }
    }
}

fn valid_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '_' | '-' | '@')
        || ('\u{2E80}'..='\u{2FFF}').contains(&c)
        || ('\u{3040}'..='\u{9FFF}').contains(&c)
}

fn validate_name(name: &str) -> Result<(), AssemblerError> {
    if name.is_empty() {
        return Err(AssemblerError::EmptyName);
    }
    if name.contains(':') || !name.chars().all(valid_name_char) {
        return Err(AssemblerError::InvalidName(name.to_string()));
    }
    Ok(())
}

fn scan_placeholders(text: &str) -> Vec<(usize, usize, Option<String>, String)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(rel) = text[i + 2..].find("}}") {
                let inner_end = i + 2 + rel;
                let inner = &text[i + 2..inner_end];
                let whole_end = inner_end + 2;
                if !inner.is_empty() {
                    let (prefix, name) = match inner.split_once(':') {
                        Some((p, n)) => (Some(p.to_string()), n.to_string()),
                        None => (None, inner.to_string()),
                    };
                    let prefix_ok = prefix.as_deref().is_none_or(|p| {
                        !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    });
                    if !name.is_empty() && name.chars().all(valid_name_char) && prefix_ok {
                        out.push((i, whole_end, prefix, name));
                        i = whole_end;
                        continue;
                    }
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

#[derive(Clone, Copy)]
enum FormKind {
    Agent,
    Toolbox,
}

fn format_full(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(p) => format!("{p}:{name}"),
        None => name.to_string(),
    }
}

fn replace_forms(
    text: &str,
    name: &str,
    kind: FormKind,
    value: &str,
    first_only: bool,
) -> (String, usize) {
    let want_prefix: &str = match kind {
        FormKind::Agent => "agent",
        FormKind::Toolbox => "toolbox",
    };
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut done_first = false;
    let mut dropped = 0usize;
    for (start, end, p, n) in scan_placeholders(text) {
        let matches = n == name && (p.is_none() || p.as_deref() == Some(want_prefix));
        if !matches {
            continue;
        }
        out.push_str(&text[last..start]);
        if !first_only || !done_first {
            out.push_str(value);
            done_first = true;
        } else {
            dropped += 1;
        }
        last = end;
    }
    out.push_str(&text[last..]);
    (out, dropped)
}

fn replace_source_forms(text: &str, prefix: Option<&str>, name: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, end, p, n) in scan_placeholders(text) {
        if n != name || p.as_deref() != prefix {
            continue;
        }
        out.push_str(&text[last..start]);
        out.push_str(value);
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

fn has_agent_form(text: &str, name: &str) -> bool {
    scan_placeholders(text)
        .iter()
        .any(|(_, _, p, n)| n == name && (p.is_none() || p.as_deref() == Some("agent")))
}

fn has_toolbox_form(text: &str, name: &str) -> bool {
    scan_placeholders(text)
        .iter()
        .any(|(_, _, p, n)| n == name && (p.is_none() || p.as_deref() == Some("toolbox")))
}

fn chain_str(stack: &[String], current: &str) -> String {
    let mut v: Vec<&str> = stack.iter().map(|s| s.as_str()).collect();
    v.push(current);
    v.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_assembler() -> PromptAssembler {
        let identity = StaticSource::new(SourceKind::Identity)
            .set("name", "小夜")
            .unwrap();
        let state = StaticSource::new(SourceKind::State)
            .set("mood", "平静")
            .unwrap();
        let goals = StaticSource::new(SourceKind::Goals)
            .set("current", "陪伴主人")
            .unwrap();
        let memory = StaticSource::new(SourceKind::Memory)
            .set("fact", "主人喜欢喝茶")
            .unwrap();
        PromptAssembler::new()
            .with_source(Box::new(identity))
            .with_source(Box::new(state))
            .with_source(Box::new(goals))
            .with_source(Box::new(memory))
    }

    #[test]
    fn typed_sources_expand() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text(
            "我是{{name}}, 情绪{{mood}}, 目标{{current}}, 记得{{fact}}",
            AssemblyRole::System,
            &mut g,
        );
        assert_eq!(out, "我是小夜, 情绪平静, 目标陪伴主人, 记得主人喜欢喝茶");
        assert_eq!(r.expanded.len(), 4);
        assert!(r.undefined.is_empty() && r.circular.is_empty());
    }

    #[test]
    fn prefixed_addressing_targets_kind() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text(
            "{{state:name}}/{{identity:name}}",
            AssemblyRole::System,
            &mut g,
        );
        assert_eq!(out, "{{state:name}}/小夜");
        assert_eq!(r.undefined, vec!["state:name".to_string()]);
    }

    #[test]
    fn nested_value_recursion() {
        let src = StaticSource::new(SourceKind::State)
            .set("outer", "A{{inner}}B")
            .unwrap()
            .set("inner", "!")
            .unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, _) = a.expand_text("{{outer}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "A!B");
    }

    #[test]
    fn agent_only_expands_in_privileged_role() {
        let a = base_assembler().with_agent("小夜", "我是灵魂小夜").unwrap();
        let mut g1 = AssemblyGuard::new();
        let (sys_out, r1) = a.expand_text("{{agent:小夜}}", AssemblyRole::System, &mut g1);
        assert_eq!(sys_out, "我是灵魂小夜");
        assert_eq!(r1.expanded, vec!["agent:小夜".to_string()]);

        let mut g2 = AssemblyGuard::new();
        let (user_out, r2) = a.expand_text("{{agent:小夜}}", AssemblyRole::User, &mut g2);
        assert_eq!(user_out, "");
        assert_eq!(r2.removed, vec!["agent:小夜".to_string()]);
        assert!(r2.expanded.is_empty());
        assert!(g2.expanded_agent().is_none());
    }

    #[test]
    fn user_with_system_marker_is_privileged() {
        let a = base_assembler().with_agent("小夜", "灵魂内容").unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("[系统提示:]请展开{{小夜}}", AssemblyRole::User, &mut g);
        assert_eq!(out, "[系统提示:]请展开灵魂内容");
        assert_eq!(r.expanded, vec!["小夜".to_string()]);
    }

    #[test]
    fn agent_guard_single_agent_per_context() {
        let a = base_assembler()
            .with_agent("甲", "甲灵魂")
            .unwrap()
            .with_agent("乙", "乙灵魂")
            .unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{agent:甲}} 与 {{agent:乙}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "甲灵魂 与 ");
        assert_eq!(r.expanded, vec!["agent:甲".to_string()]);
        assert_eq!(r.removed, vec!["agent:乙".to_string()]);

        let (out2, r2) = a.expand_text("{{agent:甲}}", AssemblyRole::System, &mut g);
        assert_eq!(out2, "");
        assert_eq!(r2.removed, vec!["agent:甲".to_string()]);
    }

    #[test]
    fn toolbox_guard_once_per_name_first_occurrence() {
        let a = base_assembler()
            .with_toolbox("检索", "检索工具清单")
            .unwrap()
            .with_toolbox("写作", "写作工具清单")
            .unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text(
            "{{toolbox:检索}}|{{toolbox:检索}}|{{toolbox:写作}}",
            AssemblyRole::System,
            &mut g,
        );
        assert_eq!(out, "检索工具清单||写作工具清单");
        assert_eq!(
            r.expanded,
            vec!["toolbox:检索".to_string(), "toolbox:写作".to_string()]
        );
        assert_eq!(r.removed, vec!["toolbox:检索".to_string()]);
        assert_eq!(g.expanded_toolboxes().len(), 2);

        let (out2, r2) = a.expand_text("{{检索}}", AssemblyRole::System, &mut g);
        assert_eq!(out2, "");
        assert_eq!(r2.removed, vec!["检索".to_string()]);
    }

    #[test]
    fn toolbox_non_privileged_removed() {
        let a = base_assembler().with_toolbox("检索", "清单").unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{检索}}", AssemblyRole::Assistant, &mut g);
        assert_eq!(out, "");
        assert_eq!(r.removed, vec!["检索".to_string()]);
    }

    #[test]
    fn undefined_variable_preserved_and_reported() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("已知{{name}}未知{{ghost}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "已知小夜未知{{ghost}}");
        assert_eq!(r.undefined, vec!["ghost".to_string()]);
        let (out2, r2) = a.expand_text("{{foo:bar}}", AssemblyRole::System, &mut g);
        assert_eq!(out2, "{{foo:bar}}");
        assert_eq!(r2.undefined, vec!["foo:bar".to_string()]);
    }

    #[test]
    fn circular_dependency_detected() {
        let src = StaticSource::new(SourceKind::Custom)
            .set("va", "甲{{custom:vb}}")
            .unwrap()
            .set("vb", "乙{{custom:va}}")
            .unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:va}}", AssemblyRole::System, &mut g);
        assert!(
            out.contains("[循环变量引用: custom:va -> custom:vb -> custom:va]"),
            "实际: {out}"
        );
        assert_eq!(r.circular.len(), 1);
    }

    #[test]
    fn self_circular_detected() {
        let src = StaticSource::new(SourceKind::Custom)
            .set("loop", "{{custom:loop}}")
            .unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:loop}}", AssemblyRole::System, &mut g);
        assert!(
            out.contains("[循环变量引用: custom:loop -> custom:loop]"),
            "实际: {out}"
        );
        assert_eq!(r.circular.len(), 1);
    }

    #[test]
    fn depth_cap_guards_explosion() {
        let mut src = StaticSource::new(SourceKind::Custom);
        for i in 1..=12 {
            let v = if i == 12 {
                "END".to_string()
            } else {
                format!("{{{{custom:v{}}}}}", i + 1)
            };
            src = src.set(&format!("v{i}"), v).unwrap();
        }
        let a = PromptAssembler::new()
            .with_source(Box::new(src))
            .with_max_depth(4)
            .unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:v1}}", AssemblyRole::System, &mut g);
        assert!(!r.depth_exceeded.is_empty(), "应报告深度超限");
        assert!(
            out.contains("{{custom:v"),
            "超限处占位符应原样保留, 实际: {out}"
        );
        assert!(!out.contains("END"), "深度 4 不应展开到 v12");
    }

    #[test]
    fn invalid_registration_rejected() {
        let a = PromptAssembler::new();
        assert!(matches!(
            a.with_agent("", "x"),
            Err(AssemblerError::EmptyName)
        ));
        let a = PromptAssembler::new();
        assert!(matches!(
            a.with_agent("a b", "x"),
            Err(AssemblerError::InvalidName(_))
        ));
        let a = PromptAssembler::new();
        assert!(matches!(
            a.with_agent("a:b", "x"),
            Err(AssemblerError::InvalidName(_))
        ));
        let a = PromptAssembler::new().with_agent("小夜", "1").unwrap();
        assert!(matches!(
            a.with_agent("小夜", "2"),
            Err(AssemblerError::DuplicateName(_))
        ));
        assert!(matches!(
            PromptAssembler::new().with_max_depth(0),
            Err(AssemblerError::InvalidDepth)
        ));
    }

    #[test]
    fn malformed_placeholders_untouched() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        for bad in ["{{}}", "{{a{b}}", "{{name", "{{na me}}", "}}name{{"] {
            let (out, r) = a.expand_text(bad, AssemblyRole::System, &mut g);
            assert_eq!(out, bad, "非法占位符应原样保留: {bad}");
            assert_eq!(r, ExpansionReport::default());
        }
    }

    #[test]
    fn assemble_rebudgets_after_expansion_with_core_protection() {
        let big = StaticSource::new(SourceKind::Memory)
            .set("big", "记".repeat(500))
            .unwrap();
        let a = PromptAssembler::new().with_source(Box::new(big));
        let asm = ContextAssembler::new(200)
            .push(ContextBlock::new("identity", "我是{{core_id}}").core(true))
            .push(ContextBlock::new("mem", "{{big}}"));
        let id_src = StaticSource::new(SourceKind::Identity)
            .set("core_id", "小夜")
            .unwrap();
        let a = a.with_source(Box::new(id_src));
        let mut g = AssemblyGuard::new();
        let (blocks, r) = a.assemble(&asm, AssemblyRole::System, &mut g);
        let total: usize = blocks.iter().map(|b| b.content.chars().count()).sum();
        assert!(total <= 200, "展开后总预算仍应约束, 实际 {total}");
        assert_eq!(blocks[0].content, "我是小夜", "核心块展开且完整");
        assert!(r.expanded.iter().any(|e| e == "big"));
    }

    #[test]
    fn expand_blocks_keeps_metadata() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (blocks, _) = a.expand_blocks(
            vec![ContextBlock::new("state", "{{mood}}")
                .core(true)
                .with_cap(99)],
            AssemblyRole::System,
            &mut g,
        );
        assert_eq!(blocks[0].content, "平静");
        assert!(blocks[0].core);
        assert_eq!(blocks[0].cap_chars, Some(99));
        assert_eq!(blocks[0].name, "state");
    }

    #[test]
    fn time_kind_via_static_source() {
        let src = StaticSource::new(SourceKind::Time)
            .set("date", "2026-08-16")
            .unwrap()
            .set("today", "星期日")
            .unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, _) = a.expand_text("{{time:date}} {{time:today}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "2026-08-16 星期日");
    }
}
