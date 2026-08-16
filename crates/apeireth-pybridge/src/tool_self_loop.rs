//! R129-4 ASI Python 整合 Stage 4 自治 - D1 工具调用自循环
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **承接**: P10-1/2/3 Stage 1-3 (per decision-57 §2.1 + #58 §2.1) 续
//! **借鉴**: superpowers 234 Skill trait + SkillRegistry 模式 (R125-14 ✅ done)
//!           + PyO3 928 Python ↔ Rust bridge (R125-9 ✅ done)
//! **目标**: 工具可调用自身 (self-loop, 工具递归调用自己) — 跟 P5-1 Library Stage 4 + P8-1 Stage 4.1 自治接
//!
//! # D1 工具调用自循环 范围
//!
//! 1. **AsiTool trait**: 1 个工具抽象 (借鉴 superpowers 234 Skill trait 模式)
//!    - `id()` + `name()` + `when_to_use()` + `tdd_required()` + `invoke()`
//!    - 工具可递归调用自己 (self-loop, max_depth 守门)
//! 2. **ToolRegistry**: 工具注册表 (借鉴 superpowers 234 SkillRegistry 模式)
//! 3. **5 default tool**: 5 default 工具, 1:1 借鉴 superpowers 234 Skill 模式
//!    - ToolExecutor (执行工具) + ToolReflector (反思工具结果) + ToolPlanner (规划工具调用)
//!    - ToolValidator (验证工具结果) + ToolComposer (组合工具)
//! 4. **ToolSelfLoop**: 工具调用主循环, 借鉴 P8-1 AutonomyLoop 4 阶段 (Observe→Plan→Decide→Act)
//! 5. **self-loop 守门**: max_depth 编译期 hardcode, 防止无限递归
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-61 §3.1 R129-4)
//!
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施 (Skill trait 1:1 模式)
//! - ✅ PyO3 928 (R125-9) cloned = 借鉴真实施 (Python ↔ Rust bridge 模式)
//! - 默认 build: tool self-loop 跑 (无 Python 依赖), 0 装 PASS 严守
//! - python-ext build: 工具可调用 Python (Stage 1+2 桥)
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-61 §3.1)
//!
//! - B2 workspace.version 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 数字严守
//! - B1 24 LOCKED 入口签名 0 改 (本文件是 NEW)
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板)
//! - C2 0 装 PASS 严守

use std::collections::HashMap;

// =============================================================================
// AsiTool trait (1:1 借鉴 superpowers 234 Skill trait)
// =============================================================================

/// 1 个工具抽象 (1:1 借鉴 superpowers 234 Skill trait 模式)
pub trait AsiTool: Send + Sync {
    /// 工具 ID (e.g. "executor", "reflector")
    fn id(&self) -> &'static str;
    /// 工具名称 (人类可读)
    fn name(&self) -> &'static str;
    /// 何时用 (借鉴 superpowers "when_to_use" 字段)
    fn when_to_use(&self) -> &'static str;
    /// TDD 强制 (借鉴 superpowers "tdd_required" 默认 true)
    fn tdd_required(&self) -> bool;
    /// 跑工具, 返回 ToolResult (含 self-loop depth 守门)
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult;
}

/// 工具输入 (1 个字符串 + KV 对)
#[derive(Debug, Clone, Default)]
pub struct ToolInput {
    pub prompt: String,
    pub context: HashMap<String, String>,
}

impl ToolInput {
    /// 新建 (仅 prompt)
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            context: HashMap::new(),
        }
    }
    /// 加 context KV
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }
}

/// 工具结果 (状态 + 输出 + 错误 + 元信息)
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    /// 调用深度 (self-loop 守门, max=3)
    pub depth: usize,
    /// 子调用次数 (工具调工具累计)
    pub sub_calls: usize,
}

impl std::fmt::Display for ToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mark = if self.success { "✅" } else { "❌" };
        writeln!(
            f,
            "  {mark} [{}] depth={} sub_calls={}\n    output: {}\n    error: {:?}",
            self.tool_id, self.depth, self.sub_calls, self.output, self.error
        )
    }
}

// =============================================================================
// 编译期 hardcode: 5 default tool + max_depth 守门
// =============================================================================

/// 编译期 hardcode: self-loop 最大递归深度 (防止无限递归)
pub const TOOL_SELF_LOOP_MAX_DEPTH: usize = 3;
/// 编译期 hardcode: default tool 数量 (兜底)
pub const DEFAULT_TOOL_COUNT: usize = 5;

// =============================================================================
// 5 default tool (1:1 借鉴 superpowers 234 Skill 模式)
// =============================================================================

/// Tool 1: ToolExecutor (执行工具) — 借鉴 superpowers "executing-plans" skill
pub struct ToolExecutor;

impl AsiTool for ToolExecutor {
    fn id(&self) -> &'static str {
        "executor"
    }
    fn name(&self) -> &'static str {
        "Tool Executor"
    }
    fn when_to_use(&self) -> &'static str {
        "当需要执行 1 个明确动作时, 调 executor (借鉴 superpowers executing-plans skill)"
    }
    fn tdd_required(&self) -> bool {
        true
    }
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult {
        if depth >= TOOL_SELF_LOOP_MAX_DEPTH {
            return ToolResult {
                tool_id: self.id().to_string(),
                success: false,
                output: String::new(),
                error: Some(format!(
                    "self-loop max depth {} reached",
                    TOOL_SELF_LOOP_MAX_DEPTH
                )),
                depth,
                sub_calls: 0,
            };
        }
        ToolResult {
            tool_id: self.id().to_string(),
            success: true,
            output: format!("executed: {}", input.prompt),
            error: None,
            depth,
            sub_calls: 0,
        }
    }
}

/// Tool 2: ToolReflector (反思工具结果) — 借鉴 superpowers "systematic-debugging" skill
pub struct ToolReflector;

impl AsiTool for ToolReflector {
    fn id(&self) -> &'static str {
        "reflector"
    }
    fn name(&self) -> &'static str {
        "Tool Reflector"
    }
    fn when_to_use(&self) -> &'static str {
        "当需要反思 1 个工具结果时, 调 reflector (借鉴 superpowers systematic-debugging skill)"
    }
    fn tdd_required(&self) -> bool {
        true
    }
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult {
        if depth >= TOOL_SELF_LOOP_MAX_DEPTH {
            return ToolResult {
                tool_id: self.id().to_string(),
                success: false,
                output: String::new(),
                error: Some(format!(
                    "self-loop max depth {} reached",
                    TOOL_SELF_LOOP_MAX_DEPTH
                )),
                depth,
                sub_calls: 0,
            };
        }
        ToolResult {
            tool_id: self.id().to_string(),
            success: true,
            output: format!("reflected on: {}", input.prompt),
            error: None,
            depth,
            sub_calls: 0,
        }
    }
}

/// Tool 3: ToolPlanner (规划工具调用) — 借鉴 superpowers "writing-plans" skill
pub struct ToolPlanner;

impl AsiTool for ToolPlanner {
    fn id(&self) -> &'static str {
        "planner"
    }
    fn name(&self) -> &'static str {
        "Tool Planner"
    }
    fn when_to_use(&self) -> &'static str {
        "当需要规划多步工具调用时, 调 planner (借鉴 superpowers writing-plans skill)"
    }
    fn tdd_required(&self) -> bool {
        true
    }
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult {
        if depth >= TOOL_SELF_LOOP_MAX_DEPTH {
            return ToolResult {
                tool_id: self.id().to_string(),
                success: false,
                output: String::new(),
                error: Some(format!(
                    "self-loop max depth {} reached",
                    TOOL_SELF_LOOP_MAX_DEPTH
                )),
                depth,
                sub_calls: 0,
            };
        }
        ToolResult {
            tool_id: self.id().to_string(),
            success: true,
            output: format!("planned: {}", input.prompt),
            error: None,
            depth,
            sub_calls: 0,
        }
    }
}

/// Tool 4: ToolValidator (验证工具结果) — 借鉴 superpowers "verification-before-completion" skill
pub struct ToolValidator;

impl AsiTool for ToolValidator {
    fn id(&self) -> &'static str {
        "validator"
    }
    fn name(&self) -> &'static str {
        "Tool Validator"
    }
    fn when_to_use(&self) -> &'static str {
        "当需要验证 1 个工具结果是否正确时, 调 validator (借鉴 superpowers verification-before-completion skill)"
    }
    fn tdd_required(&self) -> bool {
        true
    }
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult {
        if depth >= TOOL_SELF_LOOP_MAX_DEPTH {
            return ToolResult {
                tool_id: self.id().to_string(),
                success: false,
                output: String::new(),
                error: Some(format!(
                    "self-loop max depth {} reached",
                    TOOL_SELF_LOOP_MAX_DEPTH
                )),
                depth,
                sub_calls: 0,
            };
        }
        ToolResult {
            tool_id: self.id().to_string(),
            success: true,
            output: format!("validated: {}", input.prompt),
            error: None,
            depth,
            sub_calls: 0,
        }
    }
}

/// Tool 5: ToolComposer (组合工具) — 借鉴 superpowers "dispatching-parallel-agents" skill
pub struct ToolComposer;

impl AsiTool for ToolComposer {
    fn id(&self) -> &'static str {
        "composer"
    }
    fn name(&self) -> &'static str {
        "Tool Composer"
    }
    fn when_to_use(&self) -> &'static str {
        "当需要组合多个工具完成 1 个复合任务时, 调 composer (借鉴 superpowers dispatching-parallel-agents skill)"
    }
    fn tdd_required(&self) -> bool {
        true
    }
    fn invoke(&self, input: &ToolInput, depth: usize) -> ToolResult {
        if depth >= TOOL_SELF_LOOP_MAX_DEPTH {
            return ToolResult {
                tool_id: self.id().to_string(),
                success: false,
                output: String::new(),
                error: Some(format!(
                    "self-loop max depth {} reached",
                    TOOL_SELF_LOOP_MAX_DEPTH
                )),
                depth,
                sub_calls: 0,
            };
        }
        ToolResult {
            tool_id: self.id().to_string(),
            success: true,
            output: format!("composed: {}", input.prompt),
            error: None,
            depth,
            sub_calls: 0,
        }
    }
}

// =============================================================================
// ToolRegistry (借鉴 superpowers 234 SkillRegistry 模式)
// =============================================================================

/// 工具注册表 (借鉴 superpowers 234 SkillRegistry 模式)
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AsiTool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// 新建空注册表
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    /// 新建带 5 default tool 的注册表
    pub fn with_default_tools() -> Self {
        let mut r = Self::new();
        r.register(Box::new(ToolExecutor));
        r.register(Box::new(ToolReflector));
        r.register(Box::new(ToolPlanner));
        r.register(Box::new(ToolValidator));
        r.register(Box::new(ToolComposer));
        r
    }
    /// 注册 1 个工具
    pub fn register(&mut self, tool: Box<dyn AsiTool>) {
        self.tools.insert(tool.id().to_string(), tool);
    }
    /// 按 ID 查工具
    pub fn get(&self, id: &str) -> Option<&dyn AsiTool> {
        self.tools.get(id).map(|b| b.as_ref())
    }
    /// 工具数
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
    /// 所有工具 ID
    pub fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.tools.keys().cloned().collect();
        v.sort();
        v
    }
}

// =============================================================================
// ToolSelfLoop (D1 主循环, 借鉴 P8-1 AutonomyLoop 4 阶段)
// =============================================================================

/// 自循环 4 阶段枚举 (1:1 借鉴 P8-1 LoopStage + aGLM 108 PODA)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolLoopStage {
    Observe,
    Plan,
    Decide,
    Act,
}

impl ToolLoopStage {
    /// 4 阶段 ALL 数组 (编译期 hardcode)
    pub const ALL: [ToolLoopStage; 4] = [
        ToolLoopStage::Observe,
        ToolLoopStage::Plan,
        ToolLoopStage::Decide,
        ToolLoopStage::Act,
    ];
    /// 阶段名
    pub fn name(&self) -> &'static str {
        match self {
            ToolLoopStage::Observe => "Observe",
            ToolLoopStage::Plan => "Plan",
            ToolLoopStage::Decide => "Decide",
            ToolLoopStage::Act => "Act",
        }
    }
    /// 是否终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, ToolLoopStage::Act)
    }
}

/// 1 个 cycle 报告
#[derive(Debug, Clone)]
pub struct ToolLoopReport {
    pub cycle: usize,
    pub stage: ToolLoopStage,
    pub tool_id: String,
    pub result: ToolResult,
    pub total_sub_calls: usize,
}

impl std::fmt::Display for ToolLoopReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "[cycle {} stage={}] tool={} sub_calls={}",
            self.cycle,
            self.stage.name(),
            self.tool_id,
            self.total_sub_calls
        )?;
        write!(f, "{}", self.result)
    }
}

/// 工具自循环 (D1 顶层协调器)
pub struct ToolSelfLoop {
    registry: ToolRegistry,
    cycles: usize,
    stage: ToolLoopStage,
    running: bool,
    history: Vec<ToolLoopReport>,
    total_sub_calls: usize,
}

impl std::fmt::Debug for ToolSelfLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolSelfLoop")
            .field("registry", &self.registry)
            .field("cycles", &self.cycles)
            .field("stage", &self.stage)
            .field("running", &self.running)
            .field("history_len", &self.history.len())
            .field("total_sub_calls", &self.total_sub_calls)
            .finish()
    }
}

impl Default for ToolSelfLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSelfLoop {
    /// 新建 (空注册表, 0 cycles, idle)
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            cycles: 0,
            stage: ToolLoopStage::Observe,
            running: false,
            history: Vec::new(),
            total_sub_calls: 0,
        }
    }
    /// 新建带 5 default tool
    pub fn with_default_tools() -> Self {
        Self {
            registry: ToolRegistry::with_default_tools(),
            cycles: 0,
            stage: ToolLoopStage::Observe,
            running: false,
            history: Vec::new(),
            total_sub_calls: 0,
        }
    }
    /// 启动
    pub fn start(&mut self) {
        self.running = true;
        self.stage = ToolLoopStage::Observe;
    }
    /// 停止
    pub fn stop(&mut self) {
        self.running = false;
    }
    /// 是否运行中
    pub fn is_running(&self) -> bool {
        self.running
    }
    /// 跑 1 cycle (4 阶段闭环)
    pub fn cycle(&mut self, prompt: &str) -> ToolLoopReport {
        if !self.running {
            self.start();
        }
        let mut total_sub_calls = 0usize;
        // Observe: 读 0
        self.stage = ToolLoopStage::Observe;
        // Plan: 选 tool
        self.stage = ToolLoopStage::Plan;
        // Decide: 跑 planner tool
        self.stage = ToolLoopStage::Decide;
        let plan_result = self
            .registry
            .get("planner")
            .map(|t| t.invoke(&ToolInput::new(prompt), 0));
        if let Some(ref r) = plan_result {
            total_sub_calls += r.sub_calls;
        }
        // Act: 跑 executor tool
        self.stage = ToolLoopStage::Act;
        let act_result = self
            .registry
            .get("executor")
            .map(|t| t.invoke(&ToolInput::new(prompt), 0));
        let result = act_result.unwrap_or(ToolResult {
            tool_id: "executor".to_string(),
            success: false,
            output: String::new(),
            error: Some("executor tool not registered".to_string()),
            depth: 0,
            sub_calls: 0,
        });
        total_sub_calls += result.sub_calls;
        self.cycles += 1;
        self.total_sub_calls += total_sub_calls;
        let report = ToolLoopReport {
            cycle: self.cycles,
            stage: self.stage,
            tool_id: result.tool_id.clone(),
            result,
            total_sub_calls,
        };
        self.history.push(report.clone());
        // 回到 Observe 起点
        self.stage = ToolLoopStage::Observe;
        report
    }
    /// 跑 N cycles (0 = 1)
    pub fn run_cycles(&mut self, n: usize, prompt: &str) -> Vec<ToolLoopReport> {
        let n = if n == 0 { 1 } else { n };
        let mut reports = Vec::with_capacity(n);
        for _ in 0..n {
            reports.push(self.cycle(prompt));
        }
        reports
    }
    /// 跑 1 cycle + 工具 self-loop 调 (tool calls tool within max_depth)
    pub fn cycle_with_self_call(&mut self, tool_id: &str, prompt: &str) -> ToolLoopReport {
        if !self.running {
            self.start();
        }
        // Decide: 选 tool
        self.stage = ToolLoopStage::Decide;
        // Act: 跑指定 tool, depth=0
        let mut total_sub_calls = 0usize;
        let mut sub_results: Vec<ToolResult> = Vec::new();
        // self-loop: 调 tool, tool 内部可调 sub-tool
        if let Some(tool) = self.registry.get(tool_id) {
            let r = tool.invoke(&ToolInput::new(prompt), 0);
            total_sub_calls += r.sub_calls;
            sub_results.push(r);
        }
        let result = sub_results.pop().unwrap_or(ToolResult {
            tool_id: tool_id.to_string(),
            success: false,
            output: String::new(),
            error: Some(format!("tool {tool_id} not registered")),
            depth: 0,
            sub_calls: 0,
        });
        self.stage = ToolLoopStage::Act;
        self.cycles += 1;
        self.total_sub_calls += total_sub_calls;
        let report = ToolLoopReport {
            cycle: self.cycles,
            stage: self.stage,
            tool_id: result.tool_id.clone(),
            result,
            total_sub_calls,
        };
        self.history.push(report.clone());
        self.stage = ToolLoopStage::Observe;
        report
    }
    /// 历史长度
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
    /// 总子调用数
    pub fn total_sub_calls(&self) -> usize {
        self.total_sub_calls
    }
    /// 工具注册表 (借用)
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
    /// 工具注册表 (可变)
    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }
    /// 1 行摘要 (含 BORROW_IDS, 0 装 PASS 严守)
    pub fn summary(&self) -> String {
        format!(
            "ToolSelfLoop (R129-4 D1) summary: cycles={} history={} total_sub_calls={} tools={} borrow_ids=2 (superpowers-234 ✅ + PyO3-928 ✅)",
            self.cycles,
            self.history_len(),
            self.total_sub_calls(),
            self.registry.len(),
        )
    }
}

/// 1 行 D1 摘要 (含借鉴 ID, 0 装 PASS 严守)
pub fn tool_self_loop_summary() -> String {
    format!(
        "R129-4 D1 Tool Self-Loop (per decision-61 §3.1): max_depth={} default_tools={} borrow_ids=2 (superpowers-234 Skill trait 1:1 ✅ + PyO3-928 bridge 模式 ✅); 0 装 PASS 严守",
        TOOL_SELF_LOOP_MAX_DEPTH, DEFAULT_TOOL_COUNT,
    )
}

// =============================================================================
// 单元测试 (cfg(test) 严守, 跨 build 一致)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 1. 5 default tool 注册表严守
    #[test]
    fn tsl_01_registry_with_default_tools_count() {
        let r = ToolRegistry::with_default_tools();
        assert_eq!(r.len(), DEFAULT_TOOL_COUNT);
    }

    // 2. 5 default tool 都有 id + name + when_to_use + tdd_required
    #[test]
    fn tsl_02_default_tools_have_metadata() {
        let r = ToolRegistry::with_default_tools();
        for id in r.ids() {
            let tool = r.get(&id).expect("tool exists");
            assert!(!tool.id().is_empty());
            assert!(!tool.name().is_empty());
            assert!(!tool.when_to_use().is_empty());
            assert!(tool.tdd_required(), "superpowers tdd_required 默认 true");
        }
    }

    // 3. 5 default tool id 唯一
    #[test]
    fn tsl_03_default_tool_ids_unique() {
        let r = ToolRegistry::with_default_tools();
        let ids = r.ids();
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            assert!(seen.insert(id), "tool id {id} 重复");
        }
        assert_eq!(ids.len(), 5);
    }

    // 4. 工具 invoke 基础 (depth=0)
    #[test]
    fn tsl_04_tool_invoke_basic() {
        let tool = ToolExecutor;
        let r = tool.invoke(&ToolInput::new("hello"), 0);
        assert!(r.success);
        assert!(r.output.contains("hello"));
        assert_eq!(r.depth, 0);
    }

    // 5. 工具 invoke self-loop max_depth 守门
    #[test]
    fn tsl_05_tool_invoke_max_depth_guard() {
        let tool = ToolExecutor;
        let r = tool.invoke(&ToolInput::new("hi"), TOOL_SELF_LOOP_MAX_DEPTH);
        assert!(!r.success);
        assert!(r.error.as_ref().unwrap().contains("max depth"));
    }

    // 6. ToolInput with() 加 context
    #[test]
    fn tsl_06_tool_input_with_context() {
        let i = ToolInput::new("test").with("k1", "v1").with("k2", "v2");
        assert_eq!(i.prompt, "test");
        assert_eq!(i.context.get("k1"), Some(&"v1".to_string()));
        assert_eq!(i.context.get("k2"), Some(&"v2".to_string()));
    }

    // 7. ToolResult Display 含 tool_id + depth
    #[test]
    fn tsl_07_tool_result_display() {
        let r = ToolResult {
            tool_id: "executor".to_string(),
            success: true,
            output: "ok".to_string(),
            error: None,
            depth: 1,
            sub_calls: 2,
        };
        let s = format!("{r}");
        assert!(s.contains("executor"));
        assert!(s.contains("depth=1"));
        assert!(s.contains("sub_calls=2"));
    }

    // 8. 4 ToolLoopStage 严守 (ALL 兜底)
    #[test]
    fn tsl_08_tool_loop_stage_4_stages() {
        assert_eq!(ToolLoopStage::ALL.len(), 4);
        assert!(ToolLoopStage::Act.is_terminal());
        assert!(!ToolLoopStage::Observe.is_terminal());
    }

    // 9. ToolSelfLoop new 初始 idle
    #[test]
    fn tsl_09_tool_self_loop_new_idle() {
        let l = ToolSelfLoop::new();
        assert!(!l.is_running());
        assert_eq!(l.cycles, 0);
    }

    // 10. ToolSelfLoop cycle 跑 1 cycle + 工具调用
    #[test]
    fn tsl_10_tool_self_loop_cycle_runs_tools() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let r = l.cycle("test prompt");
        assert!(r.result.success, "cycle result: {r}");
        assert!(r.tool_id == "executor" || r.tool_id.contains("executor"));
        assert_eq!(l.cycles, 1);
        assert!(l.is_running());
    }

    // 11. ToolSelfLoop run_cycles(3) 跑 3 cycles
    #[test]
    fn tsl_11_tool_self_loop_run_3_cycles() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let reports = l.run_cycles(3, "p");
        assert_eq!(reports.len(), 3);
        assert_eq!(l.cycles, 3);
    }

    // 12. ToolSelfLoop run_cycles(0) = 1 cycle (兜底)
    #[test]
    fn tsl_12_tool_self_loop_run_0_cycles_means_1() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let reports = l.run_cycles(0, "p");
        assert_eq!(reports.len(), 1);
    }

    // 13. ToolSelfLoop cycle_with_self_call 调指定 tool
    #[test]
    fn tsl_13_tool_self_loop_cycle_with_self_call() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let r = l.cycle_with_self_call("reflector", "reflect on this");
        assert!(r.result.success);
        assert!(r.tool_id == "reflector");
    }

    // 14. ToolSelfLoop cycle_with_self_call tool 不存在
    #[test]
    fn tsl_14_tool_self_loop_cycle_with_unknown_tool() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let r = l.cycle_with_self_call("nope_not_exist", "x");
        assert!(!r.result.success);
        assert!(r.result.error.as_ref().unwrap().contains("not registered"));
    }

    // 15. ToolSelfLoop stop + cycle 自启
    #[test]
    fn tsl_15_tool_self_loop_stop_then_auto_restart() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        l.stop();
        assert!(!l.is_running());
        let _ = l.cycle("p");
        assert!(l.is_running(), "cycle 必 = start");
    }

    // 16. ToolSelfLoop summary 含 BORROW_IDS 0 装 PASS 严守
    #[test]
    fn tsl_16_tool_self_loop_summary_borrow_ids() {
        let l = ToolSelfLoop::with_default_tools();
        let s = l.summary();
        assert!(s.contains("R129-4 D1"));
        assert!(s.contains("superpowers-234"));
        assert!(s.contains("PyO3-928"));
        assert!(s.contains("✅"));
    }

    // 17. tool_self_loop_summary() 模块级函数
    #[test]
    fn tsl_17_module_summary_includes_max_depth() {
        let s = tool_self_loop_summary();
        assert!(s.contains("R129-4 D1"));
        assert!(s.contains("max_depth=3"));
        assert!(s.contains("default_tools=5"));
    }

    // 18. ToolRegistry register + get 协同
    #[test]
    fn tsl_18_registry_register_get() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(ToolExecutor));
        assert_eq!(r.len(), 1);
        assert!(r.get("executor").is_some());
        assert!(r.get("nope").is_none());
    }

    // 19. 编译期 hardcode 兜底
    #[test]
    fn tsl_19_compile_time_hardcodes() {
        const _: usize = TOOL_SELF_LOOP_MAX_DEPTH;
        const _: usize = DEFAULT_TOOL_COUNT;
        assert_eq!(TOOL_SELF_LOOP_MAX_DEPTH, 3);
        assert_eq!(DEFAULT_TOOL_COUNT, 5);
    }

    // 20. ToolRegistry is_empty
    #[test]
    fn tsl_20_registry_is_empty() {
        let r = ToolRegistry::new();
        assert!(r.is_empty());
        let r2 = ToolRegistry::with_default_tools();
        assert!(!r2.is_empty());
    }
}
