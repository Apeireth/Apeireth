//! `apeireth-companion::prompt_assembler` — 提示词装配引擎 (占位符变量宇宙).
//!
//! 背景 (backlog N9, P0; team-work-doc §8.4 VCP 可吸收清单 "Apeireth 空白区最高价值"):
//! VCP messageProcessor (research/source/vcptoolbox/modules/messageProcessor.js, 只读调研)
//! 提出「占位符变量宇宙」范式 — 提示词模板中的 `{{变量}}` 由分型来源展开。
//! 本模块吸收其**写法范式** (不抄代码):
//! - **分型变量源**: identity/state/goals/memory/time 按类型提供变量 (`VariableSource` trait)
//! - **特权角色**: `agent:`/`toolbox:` 型变量只在 system 展开 (防 user 消息注入探测),
//!   兼容 VCP Tavern 语义: 以系统标记开头的 user 文本视为特权 (标记可配置)
//! - **AgentGuard**: 整个装配上下文只展开一个 agent (灵魂级安全, 仿 VCP expandedAgentName);
//!   第二个 agent 占位符 (含同名复现) 静默移除
//! - **ToolboxGuard**: 每种 toolbox 全上下文至多展开一次 (仿 VCP expandedToolboxes),
//!   同文本内首个占位符展开、其余移除
//! - **循环依赖检测**: 展开递归栈 + 深度上限; 环 → 诚实错误标记 + 报告 (0 装 PASS)
//! - **集成而非分立**: 直接消费 [`crate::context::ContextAssembler`] 输出,
//!   展开后复用其预算/核心块保护语义再截断一次
//!
//! 占位符语法: `{{name}}` 或 `{{prefix:name}}`; name 字符集 = ASCII 字母数字 + `_` `-` `@`
//! + CJK (U+2E80-U+2FFF, U+3040-U+9FFF)。prefix 为 `agent` / `toolbox` 或变量源类型标签
//! (`identity` / `state` / `goals` / `memory` / `time` / `custom`)。
//! 未带 prefix 的名字按 "agents → toolboxes → 变量源注册序" 解析。
//!
//! 0 假装标注 (没做什么):
//! - 未接线 companion_serve / assemble.rs 实际链路 (接线属后续任务; 本模块为独立机制件, 单测验证)
//! - 无动态折叠 (VCP DynamicFold 依赖 embedding 相似度 — 属 N11 context-fold 域)
//! - 无文件递归变量源 (.txt 引用) / 无异步结果占位符 — 后续以 `VariableSource` 实现扩展
//! - toolbox 为静态文本块 (VCP 为 fold_blocks + 阈值分档, 此处按机制边界简化)
//! - 引擎是同步的 (变量源为内存数据); 需要 IO 的来源用异步预取后注入静态源

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::{Datelike, FixedOffset, Weekday};
use thiserror::Error;

use crate::context::{ContextAssembler, ContextBlock};

/// 消息角色 — 决定是否允许特权展开.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 变量源类型 (分型变量源: 按类型提供变量).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// 身份/人格 (identity)
    Identity,
    /// 当前状态 (state)
    State,
    /// 目标 (goals)
    Goals,
    /// 记忆 (memory)
    Memory,
    /// 时间 (time)
    Time,
    /// 自定义 (custom)
    Custom,
}

impl SourceKind {
    /// 类型标签 — `{{<label>:name}}` 寻址用.
    pub fn label(&self) -> &'static str {
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

/// 分型变量源: 按类型提供变量值.
pub trait VariableSource: Send + Sync {
    fn kind(&self) -> SourceKind;
    /// 解析变量值; None = 本源未知 (引擎继续问下一个源, 最终记 undefined).
    fn resolve(&self, name: &str) -> Option<String>;
}

/// 静态变量源 (有序映射, 确定性解析).
#[derive(Debug)]
pub struct StaticSource {
    kind: SourceKind,
    vars: BTreeMap<String, String>,
}

impl StaticSource {
    pub fn new(kind: SourceKind) -> Self {
        Self { kind, vars: BTreeMap::new() }
    }

    /// 注入变量; 名字须符合占位符字符集且不含 `:`.
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

/// 时间变量源 — `{{time:date}}` / `{{time:time}}` / `{{time:today}}` (无前缀同名亦可).
///
/// 持 `Arc<dyn apeireth_core::clock::Clock>` — 注入 VirtualClock 可快进测试 (0 真等待).
/// 默认时区 +08:00 (Asia/Shanghai, 对齐项目 DEFAULT_TIMEZONE).
pub struct TimeSource {
    clock: Arc<dyn apeireth_core::clock::Clock>,
    offset_secs: i32,
}

impl TimeSource {
    pub fn new(clock: Arc<dyn apeireth_core::clock::Clock>) -> Self {
        Self { clock, offset_secs: 8 * 3600 }
    }

    /// 自定义报告时区 (小时偏移).
    pub fn with_offset_hours(mut self, hours: i32) -> Self {
        self.offset_secs = hours * 3600;
        self
    }
}

impl VariableSource for TimeSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Time
    }
    fn resolve(&self, name: &str) -> Option<String> {
        let offset = FixedOffset::east_opt(self.offset_secs)?;
        let now = self.clock.now().with_timezone(&offset);
        match name {
            "date" => Some(now.format("%Y-%m-%d").to_string()),
            "time" => Some(now.format("%H:%M:%S").to_string()),
            "today" => Some(weekday_zh(now.weekday()).to_string()),
            _ => None,
        }
    }
}

fn weekday_zh(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "星期一",
        Weekday::Tue => "星期二",
        Weekday::Wed => "星期三",
        Weekday::Thu => "星期四",
        Weekday::Fri => "星期五",
        Weekday::Sat => "星期六",
        Weekday::Sun => "星期日",
    }
}

/// 装配器注册期错误 (非法输入在边界拒绝).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssemblerError {
    #[error("变量名不能为空")]
    EmptyName,
    #[error("变量名含非法字符或冒号: {0}")]
    InvalidName(String),
    #[error("名字已注册 (重复): {0}")]
    DuplicateName(String),
    #[error("最大展开深度必须 >= 1")]
    InvalidDepth,
}

/// 展开报告 — 0 装 PASS 的可观测载体 (做了什么/拒了什么/没解析什么).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExpansionReport {
    /// 成功展开的占位符全形 (如 `agent:小夜` / `state:mood`).
    pub expanded: Vec<String>,
    /// 被静默移除的占位符 (非特权角色 / AgentGuard / ToolboxGuard 拦截).
    pub removed: Vec<String>,
    /// 循环依赖链 (如 `a -> b -> a`).
    pub circular: Vec<String>,
    /// 未定义变量 (占位符原样保留, 不虚构内容).
    pub undefined: Vec<String>,
    /// 超出最大展开深度 (占位符原样保留).
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

/// 装配守卫 — 跨文本/跨块共享的展开状态 (一次会话装配一个).
///
/// 承载 VCP 两大守卫语义:
/// - `expanded_agent`: AgentGuard — 全上下文只允许一个 agent 展开
/// - `expanded_toolboxes`: ToolboxGuard — 每种 toolbox 至多展开一次
#[derive(Debug, Default)]
pub struct AssemblyGuard {
    expanded_agent: Option<String>,
    expanded_toolboxes: BTreeSet<String>,
}

impl AssemblyGuard {
    pub fn new() -> Self {
        Self::default()
    }
    /// 已展开的 agent 名 (诊断).
    pub fn expanded_agent(&self) -> Option<&str> {
        self.expanded_agent.as_deref()
    }
    /// 已展开的 toolbox 名集合 (诊断).
    pub fn expanded_toolboxes(&self) -> &BTreeSet<String> {
        &self.expanded_toolboxes
    }
}

/// 提示词装配引擎 — 占位符变量宇宙.
///
/// 用法: 注册分型变量源 + agent/toolbox 内容 → `expand_text` (单文本)
/// 或 `assemble` (消费 ContextAssembler, 展开后重新预算化).
pub struct PromptAssembler {
    sources: Vec<Box<dyn VariableSource>>,
    agents: BTreeMap<String, String>,
    toolboxes: BTreeMap<String, String>,
    /// user 文本视为特权的起始标记 (VCP Tavern 注入语义, 可配置).
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
            // VCP 默认特权标记 (Tavern 式系统注入 user 消息).
            system_markers: vec!["[系统提示:]".to_string(), "[系统邀请指令:]".to_string()],
            max_depth: 8,
        }
    }

    /// 覆盖特权 user 标记 (空 vec = 仅 system 角色特权).
    pub fn with_system_markers(mut self, markers: Vec<String>) -> Self {
        self.system_markers = markers;
        self
    }

    /// 最大展开深度 (防非法输入导致的爆炸); 必须 >= 1.
    pub fn with_max_depth(mut self, depth: usize) -> Result<Self, AssemblerError> {
        if depth == 0 {
            return Err(AssemblerError::InvalidDepth);
        }
        self.max_depth = depth;
        Ok(self)
    }

    /// 注册分型变量源 (注册序 = 无前缀解析优先级).
    pub fn with_source(mut self, source: Box<dyn VariableSource>) -> Self {
        self.sources.push(source);
        self
    }

    /// 注册 agent 提示词 (特权展开 + AgentGuard 单次守卫).
    pub fn with_agent(mut self, name: &str, content: impl Into<String>) -> Result<Self, AssemblerError> {
        validate_name(name)?;
        if self.agents.insert(name.to_string(), content.into()).is_some() {
            return Err(AssemblerError::DuplicateName(name.to_string()));
        }
        Ok(self)
    }

    /// 注册 toolbox 内容块 (特权展开 + ToolboxGuard 每种一次).
    pub fn with_toolbox(mut self, name: &str, content: impl Into<String>) -> Result<Self, AssemblerError> {
        validate_name(name)?;
        if self.toolboxes.insert(name.to_string(), content.into()).is_some() {
            return Err(AssemblerError::DuplicateName(name.to_string()));
        }
        Ok(self)
    }

    /// 单文本展开: 返回 (展开后文本, 报告).
    pub fn expand_text(&self, text: &str, role: AssemblyRole, guard: &mut AssemblyGuard) -> (String, ExpansionReport) {
        let mut report = ExpansionReport::default();
        let mut stack: Vec<String> = Vec::new();
        let out = self.expand_inner(text, role, guard, &mut stack, 0, &mut report);
        (out, report)
    }

    /// 逐块展开 (不做预算重算; 需要预算用 [`Self::assemble`]).
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
            out.push(ContextBlock { name: b.name, content, core: b.core, cap_chars: b.cap_chars });
        }
        (out, report)
    }

    /// 与 ContextAssembler 集成 (集成而非分立):
    /// 先走既有预算截断 → 逐块展开 → 展开后**复用** ContextAssembler 预算语义再截断一次
    /// (核心块保护 + 单块 cap + 总预算贪心砍大头).
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
            re = re.push(ContextBlock { name: b.name, content, core: b.core, cap_chars: b.cap_chars });
        }
        (re.assemble_budgeted_blocks(), report)
    }

    // ---------- 内部 ----------

    fn is_privileged(&self, text: &str, role: AssemblyRole) -> bool {
        role == AssemblyRole::System
            || (role == AssemblyRole::User && self.system_markers.iter().any(|m| text.starts_with(m.as_str())))
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
        // 唯一名字 (首现序); 展开逐个名字, 替换结果可能引入新占位符 —
        // 那些新占位符来自「值」, 而值在替换前已被递归展开过, 故不会漏/不会循环.
        let mut names: Vec<(Option<String>, String)> = Vec::new();
        for (_, _, p, n) in scan_placeholders(text) {
            let key = (p, n);
            if !names.contains(&key) {
                names.push(key);
            }
        }
        let mut cur = text.to_string();
        for (prefix, name) in names {
            cur = self.expand_one(&cur, prefix.as_deref(), &name, privileged, role, guard, stack, depth, report);
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
        // 分类: agent / toolbox / 分型变量源
        enum Cat<'a> {
            Agent,
            Toolbox,
            Source(Option<SourceKind>),
            /// 前缀不是已知类型标签 (如 `{{foo:x}}`) — 记 undefined.
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
                // 无占位符残留 (可能已被同义形展开) → 跳过, 避免误报.
                if !has_agent_form(cur, name) {
                    return cur.to_string();
                }
                // 特权闸: 非特权角色静默移除 (防注入探测).
                if !privileged {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Agent, "", false).0;
                }
                // AgentGuard: 全上下文仅一个 agent (同名复现也移除).
                if guard.expanded_agent.is_some() {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Agent, "", false).0;
                }
                let key = format!("agent:{name}");
                if stack.iter().any(|s| s == &key) {
                    let chain = chain_str(stack, &key);
                    report.circular.push(chain.clone());
                    return replace_forms(cur, name, FormKind::Agent, &format!("[循环变量引用: {chain}]"), false).0;
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
                // ToolboxGuard: 每种 toolbox 全上下文至多一次.
                if guard.expanded_toolboxes.contains(name) {
                    report.removed.push(full);
                    return replace_forms(cur, name, FormKind::Toolbox, "", false).0;
                }
                let key = format!("toolbox:{name}");
                if stack.iter().any(|s| s == &key) {
                    let chain = chain_str(stack, &key);
                    report.circular.push(chain.clone());
                    return replace_forms(cur, name, FormKind::Toolbox, &format!("[循环变量引用: {chain}]"), false).0;
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
                // 仿 VCP replaceFirstAliasPlaceholder: 首个占位符展开, 其余移除 (重复移除留痕).
                let (replaced, dropped) = replace_forms(cur, name, FormKind::Toolbox, &expanded, true);
                report.removed.extend(std::iter::repeat(full).take(dropped));
                replaced
            }
            Cat::UnknownPrefix(p) => {
                report.undefined.push(format!("{p}:{name}"));
                cur.to_string()
            }
            Cat::Source(kind_filter) => {
                // 分型变量源: 按注册序问每个源 (kind 前缀限定则只问该类型).
                let mut value: Option<(String, String)> = None; // (环检测 key, 值)
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
                    return replace_source_forms(cur, prefix, name, &format!("[循环变量引用: {chain}]"));
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

// ---------- 占位符扫描 / 替换 ----------

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

/// 扫描 `{{...}}` 占位符 → (整体起, 整体止, prefix, name); 非法/未闭合跳过.
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
                    let prefix_ok = prefix.as_deref().map_or(true, |p| {
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

/// 替换 agent/toolbox 形态: `{{name}}` 与 `{{agent:name}}` (或 toolbox:) 两形同替.
/// `first_only=true` 时首个替换为 value, 其余替换为空 (仿 VCP replaceFirstAliasPlaceholder);
/// 返回 (替换后文本, 被移除的重复占位符数) — 供报告留痕 (0 装 PASS).
fn replace_forms(text: &str, name: &str, kind: FormKind, value: &str, first_only: bool) -> (String, usize) {
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

/// 替换变量源形态: 仅精确匹配 prefix 形态 (None ↔ `{{name}}`, Some(p) ↔ `{{p:name}}`).
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
    scan_placeholders(text).iter().any(|(_, _, p, n)| n == name && (p.is_none() || p.as_deref() == Some("agent")))
}

fn has_toolbox_form(text: &str, name: &str) -> bool {
    scan_placeholders(text).iter().any(|(_, _, p, n)| n == name && (p.is_none() || p.as_deref() == Some("toolbox")))
}

fn chain_str(stack: &[String], current: &str) -> String {
    let mut v: Vec<&str> = stack.iter().map(|s| s.as_str()).collect();
    v.push(current);
    v.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::{Clock, VirtualClock};
    use chrono::{TimeZone, Utc};

    fn base_assembler() -> PromptAssembler {
        let identity = StaticSource::new(SourceKind::Identity).set("name", "小夜").unwrap();
        let state = StaticSource::new(SourceKind::State).set("mood", "平静").unwrap();
        let goals = StaticSource::new(SourceKind::Goals).set("current", "陪伴主人").unwrap();
        let memory = StaticSource::new(SourceKind::Memory).set("fact", "主人喜欢喝茶").unwrap();
        PromptAssembler::new()
            .with_source(Box::new(identity))
            .with_source(Box::new(state))
            .with_source(Box::new(goals))
            .with_source(Box::new(memory))
    }

    // ---------- 正常路径 ----------

    #[test]
    fn typed_sources_expand() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("我是{{name}}, 情绪{{mood}}, 目标{{current}}, 记得{{fact}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "我是小夜, 情绪平静, 目标陪伴主人, 记得主人喜欢喝茶");
        assert_eq!(r.expanded.len(), 4);
        assert!(r.undefined.is_empty() && r.circular.is_empty());
    }

    #[test]
    fn prefixed_addressing_targets_kind() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        // 类型前缀寻址: state:name 只问 State 源 (name 在 Identity 源, State 源没有 → undefined)
        let (out, r) = a.expand_text("{{state:name}}/{{identity:name}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "{{state:name}}/小夜");
        assert_eq!(r.undefined, vec!["state:name".to_string()]);
    }

    #[test]
    fn nested_value_recursion() {
        // 值里的占位符递归展开
        let src = StaticSource::new(SourceKind::State)
            .set("outer", "A{{inner}}B").unwrap()
            .set("inner", "!").unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, _) = a.expand_text("{{outer}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "A!B");
    }

    // ---------- 特权角色 ----------

    #[test]
    fn agent_only_expands_in_privileged_role() {
        let a = base_assembler().with_agent("小夜", "我是灵魂小夜").unwrap();
        let mut g1 = AssemblyGuard::new();
        let (sys_out, r1) = a.expand_text("{{agent:小夜}}", AssemblyRole::System, &mut g1);
        assert_eq!(sys_out, "我是灵魂小夜");
        assert_eq!(r1.expanded, vec!["agent:小夜".to_string()]);

        // user 角色: 静默移除 (防注入), 且 agent 内容绝不外泄
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

    // ---------- AgentGuard ----------

    #[test]
    fn agent_guard_single_agent_per_context() {
        let a = base_assembler()
            .with_agent("甲", "甲灵魂").unwrap()
            .with_agent("乙", "乙灵魂").unwrap();
        // 同一文本两个 agent: 首现展开, 第二个静默移除
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{agent:甲}} 与 {{agent:乙}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "甲灵魂 与 ");
        assert_eq!(r.expanded, vec!["agent:甲".to_string()]);
        assert_eq!(r.removed, vec!["agent:乙".to_string()]);

        // 跨文本共享守卫: 已展开甲, 后续任何 agent (含同名) 均移除
        let (out2, r2) = a.expand_text("{{agent:甲}}", AssemblyRole::System, &mut g);
        assert_eq!(out2, "");
        assert_eq!(r2.removed, vec!["agent:甲".to_string()]);
    }

    // ---------- ToolboxGuard ----------

    #[test]
    fn toolbox_guard_once_per_name_first_occurrence() {
        let a = base_assembler()
            .with_toolbox("检索", "检索工具清单").unwrap()
            .with_toolbox("写作", "写作工具清单").unwrap();
        let mut g = AssemblyGuard::new();
        // 同名 toolbox 出现两次: 首个展开, 其余移除; 不同名正常展开
        let (out, r) = a.expand_text("{{toolbox:检索}}|{{toolbox:检索}}|{{toolbox:写作}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "检索工具清单||写作工具清单");
        assert_eq!(r.expanded, vec!["toolbox:检索".to_string(), "toolbox:写作".to_string()]);
        assert_eq!(r.removed, vec!["toolbox:检索".to_string()]);
        assert_eq!(g.expanded_toolboxes().len(), 2);

        // 跨文本: 已展开过的 toolbox 再出现 → 移除
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

    // ---------- 失败路径: 未定义 ----------

    #[test]
    fn undefined_variable_preserved_and_reported() {
        let a = base_assembler();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("已知{{name}}未知{{ghost}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "已知小夜未知{{ghost}}");
        assert_eq!(r.undefined, vec!["ghost".to_string()]);
        // 未知前缀同样记 undefined
        let (out2, r2) = a.expand_text("{{foo:bar}}", AssemblyRole::System, &mut g);
        assert_eq!(out2, "{{foo:bar}}");
        assert_eq!(r2.undefined, vec!["foo:bar".to_string()]);
    }

    // ---------- 失败路径: 循环依赖 ----------

    #[test]
    fn circular_dependency_detected() {
        let src = StaticSource::new(SourceKind::Custom)
            .set("va", "甲{{custom:vb}}").unwrap()
            .set("vb", "乙{{custom:va}}").unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:va}}", AssemblyRole::System, &mut g);
        assert!(out.contains("[循环变量引用: custom:va -> custom:vb -> custom:va]"), "实际: {out}");
        assert_eq!(r.circular.len(), 1);
    }

    #[test]
    fn self_circular_detected() {
        let src = StaticSource::new(SourceKind::Custom).set("loop", "{{custom:loop}}").unwrap();
        let a = PromptAssembler::new().with_source(Box::new(src));
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:loop}}", AssemblyRole::System, &mut g);
        assert!(out.contains("[循环变量引用: custom:loop -> custom:loop]"), "实际: {out}");
        assert_eq!(r.circular.len(), 1);
    }

    // ---------- 失败路径: 非法输入 ----------

    #[test]
    fn depth_cap_guards_explosion() {
        // 长链 v1 → v2 → … → v12, 深度上限 4 → 超限占位符原样保留并报告
        // (注意: format! 中 {{ 是转义, 需要 {{{{ 才能生成字面双花括号占位符)
        let mut src = StaticSource::new(SourceKind::Custom);
        for i in 1..=12 {
            let v = if i == 12 { "END".to_string() } else { format!("{{{{custom:v{}}}}}", i + 1) };
            src = src.set(&format!("v{i}"), v).unwrap();
        }
        let a = PromptAssembler::new().with_source(Box::new(src)).with_max_depth(4).unwrap();
        let mut g = AssemblyGuard::new();
        let (out, r) = a.expand_text("{{custom:v1}}", AssemblyRole::System, &mut g);
        assert!(!r.depth_exceeded.is_empty(), "应报告深度超限");
        assert!(out.contains("{{custom:v"), "超限处占位符应原样保留, 实际: {out}");
        assert!(!out.contains("END"), "深度 4 不应展开到 v12");
    }

    #[test]
    fn invalid_registration_rejected() {
        let a = PromptAssembler::new();
        assert!(matches!(a.with_agent("", "x"), Err(AssemblerError::EmptyName)));
        let a = PromptAssembler::new();
        assert!(matches!(a.with_agent("a b", "x"), Err(AssemblerError::InvalidName(_))));
        let a = PromptAssembler::new();
        assert!(matches!(a.with_agent("a:b", "x"), Err(AssemblerError::InvalidName(_))));
        let a = PromptAssembler::new().with_agent("小夜", "1").unwrap();
        assert!(matches!(a.with_agent("小夜", "2"), Err(AssemblerError::DuplicateName(_))));
        assert!(matches!(PromptAssembler::new().with_max_depth(0), Err(AssemblerError::InvalidDepth)));
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

    // ---------- ContextAssembler 集成 ----------

    #[test]
    fn assemble_rebudgets_after_expansion_with_core_protection() {
        // 展开会让非核心块膨胀 → assemble 复用 ContextAssembler 预算再截断; 核心块保护
        let big = StaticSource::new(SourceKind::Memory).set("big", "记".repeat(500)).unwrap();
        let a = PromptAssembler::new().with_source(Box::new(big));
        let asm = ContextAssembler::new(200)
            .push(ContextBlock::new("identity", "我是{{core_id}}").core(true))
            .push(ContextBlock::new("mem", "{{big}}"));
        let id_src = StaticSource::new(SourceKind::Identity).set("core_id", "小夜").unwrap();
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
            vec![ContextBlock::new("state", "{{mood}}").core(true).with_cap(99)],
            AssemblyRole::System,
            &mut g,
        );
        assert_eq!(blocks[0].content, "平静");
        assert!(blocks[0].core);
        assert_eq!(blocks[0].cap_chars, Some(99));
        assert_eq!(blocks[0].name, "state");
    }

    // ---------- 时间源 + 虚拟时钟 ----------

    #[test]
    fn time_source_with_virtual_clock_fastforward() {
        let start = Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap();
        let vc = VirtualClock::new(start);
        let clock: Arc<dyn Clock> = Arc::new(vc.clone());
        let a = PromptAssembler::new().with_source(Box::new(TimeSource::new(clock)));
        let mut g = AssemblyGuard::new();
        let (out, _) = a.expand_text("{{time:date}} {{time:today}}", AssemblyRole::System, &mut g);
        assert_eq!(out, "2026-08-16 星期日");

        // 快进 1 天 (虚拟时间, 0 真等待) → 时间变量跟随
        vc.advance(chrono::Duration::days(1));
        let mut g2 = AssemblyGuard::new();
        let (out2, _) = a.expand_text("{{time:date}} {{time:today}}", AssemblyRole::System, &mut g2);
        assert_eq!(out2, "2026-08-17 星期一");
    }
}
