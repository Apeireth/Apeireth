//! `apeireth-companion::tool_bridge` — 把主动循环焊到基地工具栈.
//!
//! 「基地对他强大而友好」的最后一根线:
//! - **全量工具**: apeireth-tools 4 真工具 (web_search/file_ops/git_ops/code_exec) + recall_memory.
//! - **安全机制守护** (不吝啬授权, 靠安全机制守):
//!   1. 洋葱门 (V1 哲学 × V2 权限 × V3 HA) 先于一切;
//!   2. 审批规则: 黑名单(最严) → 白名单(recall_memory 放行) → 风险(code/shell/exec → 需主人批准);
//!   3. 出站隐私脱敏在送达层 (daemon.rs).
//!
//! 诚实: 审批的「需主人批准」在主动循环里 = 「不自主执行, 如实告诉住客 AI 需要主人」.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::{ActionTarget, ActionVerdict, RiskLevel};
use apeireth_memory::{CoreEpisode, EpisodeQuery, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_approval::{ApprovalManager, ApprovalDecision, BlacklistRule, RiskRule, WhitelistRule};
use apeireth_tool_registry::{Tool, ToolAxes, ToolKind, ToolRegistry};
use apeireth_tool_runtime::executor::{ExecutionResult, ToolExecutor};
use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::record::RecordStore;
use serde_json::{json, Value};

use crate::capability::{CapabilityKind, CapabilityRegistry};
use crate::constitution_gate::ConstitutionGate;
use crate::daemon::{Judicator, requires_llm_review};
use crate::packs::PackRegistry;
use crate::security::{SecurityGate, SovereigntyGate};
use crate::spill::{SpillStore, SPILL_THRESHOLD_CHARS};

/// 路径前缀白名单校验 (执行级, 防越权写盘 + `..` 穿越).
///
/// 规则: 规范化 (Windows 分隔符/大小写统一) 后, `path` 必须等于 `base` 或位于
/// `base/` 之下. 目标文件可能不存在 → canonicalize 父目录 + 文件名再比 (`..` 被解析).
fn path_within(path: &str, base: &str) -> bool {
    use std::path::Path;
    let norm = |p: &std::path::PathBuf| -> String {
        p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase()
    };
    let base_p = Path::new(base);
    let base_c = std::fs::canonicalize(base_p).unwrap_or_else(|_| base_p.to_path_buf());
    let path_p = Path::new(path);
    let path_c = match std::fs::canonicalize(path_p) {
        Ok(c) => c,
        Err(_) => match path_p.parent().and_then(|pa| std::fs::canonicalize(pa).ok()) {
            Some(cp) => cp.join(path_p.file_name().unwrap_or_default()),
            None => path_p.to_path_buf(),
        },
    };
    let (b, p) = (norm(&base_c), norm(&path_c));
    p == b || p.starts_with(&format!("{b}/"))
}

/// 「回忆记忆」工具 — 基地给住客 AI 的第一个自研工具 (只读, 最安全).
pub struct RecallMemoryTool {
    store: Arc<SqliteMemoryStore>,
}

impl RecallMemoryTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl Tool for RecallMemoryTool {
    fn name(&self) -> &str {
        "recall_memory"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        if query.trim().is_empty() {
            return Err("query 不能为空".to_string());
        }
        let eps = self
            .store
            .query(&EpisodeQuery::new().limit(200))
            .map_err(|e| e.to_string())?;
        let terms: Vec<String> = query
            .split(|c: char| c.is_whitespace() || matches!(c, '，' | ',' | '、' | '。' | '.' | '?' | '？'))
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();
        let mut scored: Vec<(usize, String)> = eps
            .into_iter()
            .filter_map(|ep| {
                let n = terms.iter().filter(|t| ep.content.contains(t.as_str())).count();
                if n > 0 {
                    Some((n, ep.content))
                } else {
                    None
                }
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let hits: Vec<String> = scored.into_iter().take(3).map(|(_, c)| c).collect();
        Ok(json!({
            "query": query,
            "found": hits.len(),
            "top": hits,
        }))
    }
}

/// 「沉淀记忆」工具 — 基地给住客 AI 的记忆写入口 (append-only, 低危).
///
/// 用途: AI 自己总结对话/经历后, 主动把值得长期记住的事实写回真 SQLite.
/// 约束: 单条 <= 500 字; 只能追加 (SQLite append-only, 无覆盖/删除).
pub struct SaveMemoryTool {
    store: Arc<SqliteMemoryStore>,
}

impl SaveMemoryTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl Tool for SaveMemoryTool {
    fn name(&self) -> &str {
        "save_memory"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "content 不能为空".to_string())?;
        if content.chars().count() > 500 {
            return Err("记忆内容过长 (单条 <= 500 字)".to_string());
        }
        let session_id = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("me");
        let ep = CoreEpisode {
            id: format!("mem-{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().timestamp(),
            role: "assistant".into(),
            content: content.to_string(),
            session_id: session_id.to_string(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())?;
        let preview: String = content.chars().take(40).collect();
        Ok(json!({
            "ok": true,
            "id": ep.id,
            "saved": format!("{preview}…"),
        }))
    }
}

/// post-execute 钩子: 工具结果产出后、审计前执行 (可替换/拦截结果).
/// 三段瀑布 (吸收 DSH #2): pre(洋葱门→宪法评审→权限→路径) → execute(宿主/worker) → post(钩子链→spill→审计).
pub trait PostExecuteHook: Send + Sync {
    fn apply(&self, call: &ParsedToolCall, result: &ExecutionResult) -> ExecutionResult;
}

/// 「提案能力」工具 — AI 自己长能力的第一条通道 (涌现哲学).
/// 只登记提案 (pending), 不执行能力 — 激活需宪法评审/主人批准.
pub struct ProposeCapabilityTool {
    registry: Arc<CapabilityRegistry>,
}

impl ProposeCapabilityTool {
    pub fn new(registry: Arc<CapabilityRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait::async_trait]
impl Tool for ProposeCapabilityTool {
    fn name(&self) -> &str {
        "propose_capability"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let name = args.get("name").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
            .ok_or_else(|| "name 不能为空".to_string())?;
        let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let kind = match args.get("kind").and_then(|v| v.as_str()) {
            Some("action") => CapabilityKind::Action,
            _ => CapabilityKind::Skill,
        };
        let p = self.registry.propose(name, description, kind, "apeireth")?;
        Ok(json!({
            "id": p.id,
            "status": p.status.label(),
            "note": "已提案待宪法评审/主人批准",
        }))
    }
}

/// 工具桥: 注册中心 + 洋葱门 + 审批 (黑/白/风险规则) + 执行器.
pub struct ToolBridge {
    pub registry: Arc<ToolRegistry>,
    executor: ToolExecutor,
    approval: ApprovalManager,
    pub gate: SecurityGate,
    pub sovereignty: SovereigntyGate,
    pub records: RecordStore,
    pub packs: PackRegistry,
    /// 宪法评审者 (真 LLM, 可选): 配置后 Medium+ 风险自动按原则判案.
    judicator: Option<Arc<dyn Judicator>>,
    /// 执行体隔离: worker 可执行文件路径 (None = 不隔离, 宿主内执行).
    worker: Option<PathBuf>,
    /// 结果溢出存储 (可选): 超大工具输出 spill 到会话私有文件, messages 只留定位.
    spill: Option<SpillStore>,
    /// post-execute 钩子链 (顺序执行, 审计前).
    post_hooks: Vec<Arc<dyn PostExecuteHook>>,
}

impl ToolBridge {
    /// 全量注册 (不吝啬授权, 安全机制守护) + 三层审批规则.
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        let records = RecordStore::new(store.clone());
        let registry = Arc::new(ToolRegistry::new());
        // 基地 4 真工具 (R17 战役: web_search / file_ops / git_ops / code_exec)
        if let Err(e) = apeireth_tools::register_all(&registry) {
            eprintln!("[bridge] register_all 部分失败: {e}");
        }
        registry.register(
            "recall_memory".to_string(),
            Arc::new(RecallMemoryTool::new(Arc::clone(&store))),
        );
        registry.register(
            "save_memory".to_string(),
            Arc::new(SaveMemoryTool::new(Arc::clone(&store))),
        );
        registry.register(
            "propose_capability".to_string(),
            Arc::new(ProposeCapabilityTool::new(Arc::new(CapabilityRegistry::new(
                Arc::clone(&store),
                "me",
            )))),
        );
        let executor = ToolExecutor::new(registry.clone());
        // 权限包: 默认日常包 (永久, 只读工具 + 记忆写; 主人可 grant 自定义包扩权)
        let packs = PackRegistry::new();
        packs.grant(PackRegistry::default_daily_pack());
        let approval = ApprovalManager::with_rules(vec![
            Box::new(BlacklistRule::with_blacklist(Vec::<String>::new(), false)),
            Box::new(WhitelistRule::with_whitelist([
                "recall_memory".to_string(),
                "save_memory".to_string(),
                "propose_capability".to_string(),
            ])),
            Box::new(RiskRule::with_categories(
                5 * 60 * 1000,
                [
                    "system".to_string(),
                    "network".to_string(),
                    "file".to_string(),
                    "shell".to_string(),
                    "exec".to_string(),
                    "patch".to_string(),
                    "task".to_string(),
                ],
            )),
        ]);
        Self {
            registry,
            executor,
            approval,
            gate: SecurityGate::default(),
            sovereignty: SovereigntyGate::default(),
            records,
            packs,
            judicator: None,
            worker: None,
            spill: None,
            post_hooks: Vec::new(),
        }
    }

    /// 接宪法评审者 (真 LLM): Medium+ 风险动作执行前自动按原则判案.
    /// BLOCK → sovereignty 记录 + 拒绝; 评审失败 → 保守拒绝 (0 装 PASS, 不放过).
    pub fn with_judicator(mut self, judge: Arc<dyn Judicator>) -> Self {
        self.judicator = Some(judge);
        self
    }

    /// 开启执行体隔离: MOVE 类工具 (文件/进程/代码等有副作用) 剥离到 per-call 子进程执行.
    /// `worker_bin` = `exec_worker` 可执行文件路径 (测试用 `env!("CARGO_BIN_EXE_exec_worker")`).
    /// 安全判断 (洋葱门/宪法评审/权限包/路径约束) 仍在宿主完成, 子进程只执行已批准操作.
    pub fn with_isolation(mut self, worker_bin: impl Into<PathBuf>) -> Self {
        self.worker = Some(worker_bin.into());
        self
    }

    /// 开启结果溢出: 工具输出超过阈值 → spill 到会话私有文件, messages 只留定位+提示.
    pub fn with_spill(mut self, spill: SpillStore) -> Self {
        self.spill = Some(spill);
        self
    }

    /// 注册 post-execute 钩子 (结果产出后、审计前执行; 可替换/拦截).
    pub fn with_post_hook(mut self, hook: Arc<dyn PostExecuteHook>) -> Self {
        self.post_hooks.push(hook);
        self
    }

    /// 工具风险映射 (对齐基地 8 工具真名): ShellExec → High;
    /// FileOperator/ApplyPatch/LongTask → Medium; WebSearch/Grep/Git/WebFetch/recall_memory → Low.
    pub fn tool_risk(tool: &str) -> RiskLevel {
        let t = tool.to_lowercase();
        if t.contains("exec") || t.contains("shell") {
            RiskLevel::High
        } else if t.contains("file") || t.contains("patch") || t.contains("task") {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// 主权总闸 → 洋葱门 → 审批 → 执行.
    pub async fn execute_if_allowed(&self, call: &ParsedToolCall) -> ExecutionResult {
        if self.sovereignty.is_frozen() {
            return ExecutionResult {
                tool_name: call.tool_name.clone(),
                success: false,
                output: json!(null),
                error: Some("主权熔断: 循环已冻结".to_string()),
                duration_ms: 0,
            };
        }
        let verdict = self.gate.check(
            "tool_call",
            &format!("调用工具 {}", call.tool_name),
            Self::tool_risk(&call.tool_name),
            ActionTarget::NormalAction(format!("tool:{}", call.tool_name)),
        );
        if !matches!(verdict, ActionVerdict::Allow) {
            let err = format!("洋葱门拦下: {:?}", verdict);
            return ExecutionResult {
                tool_name: call.tool_name.clone(),
                success: false,
                output: json!(null),
                error: Some(err),
                duration_ms: 0,
            };
        }
        // 结构化宪法门 (零成本硬门, 全部风险级别; 描述由系统侧生成, 调用方不可伪造):
        // 命中编译期规则 (E-4/E-6/PHL 等) → 直接拒绝 + sovereignty 记录.
        let desc = format!("调用工具 {} 参数 {}", call.tool_name, call.args);
        if let Some((key, why)) = ConstitutionGate::check(&desc) {
            self.sovereignty.report_violation(key, &call.tool_name);
            return ExecutionResult {
                tool_name: call.tool_name.clone(),
                success: false,
                output: json!(null),
                error: Some(format!("宪法硬门拦截 ({key}): {why}")),
                duration_ms: 0,
            };
        }
        // 宪法评审 (真 LLM, 按原则判案): Medium+ 风险且配置了评审者 → 自动评审.
        // 只审动作摘要 (action + tool + args), 不审对话/记忆自由文本.
        if requires_llm_review(Self::tool_risk(&call.tool_name)) {
            if let Some(judge) = &self.judicator {
                match judge.judge(&desc).await {
                    Ok(true) => {}
                    Ok(false) => {
                        self.sovereignty.report_violation("宪法评审拦截", &call.tool_name);
                        return ExecutionResult {
                            tool_name: call.tool_name.clone(),
                            success: false,
                            output: json!(null),
                            error: Some("BLOCK: 宪法评审拒绝 (按原则判案, 非关键词)".to_string()),
                            duration_ms: 0,
                        };
                    }
                    Err(e) => {
                        // 评审失败 → 保守拒绝 (不放过未审动作)
                        return ExecutionResult {
                            tool_name: call.tool_name.clone(),
                            success: false,
                            output: json!(null),
                            error: Some(format!("宪法评审失败, 保守拒绝: {e}")),
                            duration_ms: 0,
                        };
                    }
                }
            }
        }
        // 权限包检查: 被活跃包覆盖 → 免现场审批直接执行 (责任自负 + 监督兜底)
        let pack_authorized = self
            .packs
            .check_and_consume(&call.tool_name, chrono::Utc::now().timestamp_millis());
        // 执行级路径校验: 权限包 paths 约束 (FileOperator 等文件类工具, 防越权写盘 / `..` 穿越)
        if pack_authorized {
            if let Some(paths) =
                self.packs
                    .paths_for(&call.tool_name, chrono::Utc::now().timestamp_millis())
            {
                if let Some(p) = call
                    .args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    if !paths.iter().any(|base| path_within(p, base)) {
                        return ExecutionResult {
                            tool_name: call.tool_name.clone(),
                            success: false,
                            output: json!(null),
                            error: Some(format!(
                                "权限包路径约束拒绝: {p} 不在获准路径 [{}] 内",
                                paths.join(", ")
                            )),
                            duration_ms: 0,
                        };
                    }
                }
            }
        }
        let r = if pack_authorized {
            self.run_executor(call).await
        } else {
            match self.approval.check(call) {
                ApprovalDecision::Allow => self.run_executor(call).await,
                ApprovalDecision::RequireApproval { .. } => {
                    return ExecutionResult {
                        tool_name: call.tool_name.clone(),
                        success: false,
                        output: json!(null),
                        error: Some("该工具是高风险操作且未被权限包覆盖, 需要主人批准".to_string()),
                        duration_ms: 0,
                    }
                }
                _ => {
                    return ExecutionResult {
                        tool_name: call.tool_name.clone(),
                        success: false,
                        output: json!(null),
                        error: Some("审批拒绝".to_string()),
                        duration_ms: 0,
                    }
                }
            }
        };
        // 结果溢出: 超大输出 spill 到会话私有文件, messages 只留定位 (防撑爆上下文)
        let r = if let Some(spill) = &self.spill {
            if r.success {
                let ser = serde_json::to_string(&r.output).unwrap_or_default();
                if ser.chars().count() > SPILL_THRESHOLD_CHARS {
                    match spill.spill("me", "tool_result.txt", &ser) {
                        Ok(path) => ExecutionResult {
                            tool_name: r.tool_name.clone(),
                            success: true,
                            output: json!({
                                "spilled": true,
                                "path": path,
                                "bytes": ser.len(),
                                "hint": "结果过大已溢出到会话私有文件; 需要时用 FileOperator(op=read) 读取"
                            }),
                            error: None,
                            duration_ms: r.duration_ms,
                        },
                        Err(e) => {
                            eprintln!("[spill] 溢出失败: {e}");
                            r
                        }
                    }
                } else {
                    r
                }
            } else {
                r
            }
        } else {
            r
        };
        // post-execute 钩子链 (结果产出后、审计前; 可替换/拦截)
        let mut r = r;
        for h in &self.post_hooks {
            r = h.apply(call, &r);
        }
        // 监督机制: 每次工具调用 append-only 记录 (含结果, 出站隐私脱敏后存)
        let serialized = serde_json::to_string(&r.output).unwrap_or_default();
        let pii = apeireth_guard::detect_pii(&serialized);
        let masked_output = if pii.is_empty() {
            r.output.clone()
        } else {
            serde_json::Value::String(apeireth_guard::redact_text(
                &serialized,
                &pii,
                apeireth_guard::RedactionStrategy::Mask,
            ))
        };
        let _ = self
            .records
            .record(call, &masked_output, !pii.is_empty())
            .await;
        r
    }

    /// 执行器入口: 隔离模式 + MOVE 工具 → per-call 子进程; 否则宿主执行器.
    async fn run_executor(&self, call: &ParsedToolCall) -> ExecutionResult {
        if let Some(worker) = &self.worker {
            if crate::exec_worker::should_isolate(&call.tool_name) {
                return self.execute_isolated(worker, call).await;
            }
        }
        self.executor.execute(call).await
    }

    /// per-call 子进程执行: 一行 JSON 请求 → 一行响应, 30s 超时 kill.
    async fn execute_isolated(&self, worker: &PathBuf, call: &ParsedToolCall) -> ExecutionResult {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let start = std::time::Instant::now();
        let err_res = |msg: String, start: std::time::Instant| ExecutionResult {
            tool_name: call.tool_name.clone(),
            success: false,
            output: json!(null),
            error: Some(msg),
            duration_ms: start.elapsed().as_millis() as u64,
        };
        let mut child = match tokio::process::Command::new(worker)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return err_res(format!("worker spawn 失败: {e}"), start),
        };
        let mut stdin = match child.stdin.take() {
            Some(s) => s,
            None => return err_res("worker stdin 不可用".into(), start),
        };
        let req = format!("{}\n", json!({"tool": call.tool_name, "args": call.args}));
        if let Err(e) = stdin.write_all(req.as_bytes()).await {
            let _ = child.kill().await;
            return err_res(format!("写 worker 请求失败: {e}"), start);
        }
        drop(stdin);
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return err_res("worker stdout 不可用".into(), start),
        };
        let line = match tokio::time::timeout(Duration::from_secs(30), async {
            let mut r = tokio::io::BufReader::new(stdout);
            r.lines().next_line().await
        })
        .await
        {
            Ok(Ok(Some(l))) => l,
            Ok(Ok(None)) => {
                let _ = child.kill().await;
                return err_res("worker 无响应 (提前退出)".into(), start);
            }
            Ok(Err(e)) => {
                let _ = child.kill().await;
                return err_res(format!("读 worker 响应失败: {e}"), start);
            }
            Err(_) => {
                let _ = child.kill().await;
                return err_res("worker 超时 (30s), 已 kill".into(), start);
            }
        };
        let _ = child.wait().await;
        let resp: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or(json!({"ok": false, "error": format!("worker 响应非法: {line}")}));
        let dur = start.elapsed().as_millis() as u64;
        if resp["ok"] == json!(true) {
            ExecutionResult {
                tool_name: call.tool_name.clone(),
                success: true,
                output: resp["output"].clone(),
                error: None,
                duration_ms: dur,
            }
        } else {
            ExecutionResult {
                tool_name: call.tool_name.clone(),
                success: false,
                output: json!(null),
                error: resp
                    .get("error")
                    .and_then(|e| e.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some("worker 返回失败".to_string())),
                duration_ms: dur,
            }
        }
    }

    /// 给住客 AI 的工具调用格式说明 (注入 LLM system prompt).
    pub fn tool_format_instruction() -> String {
        "如果你需要调用基地工具 (比如回忆用户的记忆), 在回复中输出:\n<<<[TOOL_REQUEST]>>>\ntool_name:<<<recall_memory>>>\nquery:<<<关键词>>>\n<<<[END_TOOL_REQUEST]>>>\n收到工具结果后, 再继续用自然语言回复。高危工具 (执行代码等) 需要主人批准, 你不能自主执行。"
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::CoreEpisode;

    #[tokio::test]
    async fn recall_tool_searches_seeded_memory() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        store
            .put_episode(&CoreEpisode {
                id: "e1".into(),
                timestamp: 1,
                role: "assistant".into(),
                content: "线性代数: 矩阵的秩的作业".into(),
                session_id: "s1".into(),
            })
            .unwrap();
        let bridge = ToolBridge::new(store);
        let call = ParsedToolCall {
            tool_name: "recall_memory".into(),
            args: json!({"query": "线性代数"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "err = {:?}", r.error);
        assert!(r.output["found"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn all_base_tools_registered() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let names = bridge.registry.list();
        assert!(names.iter().any(|n| n == "recall_memory"));
        assert!(names.iter().any(|n| n == "save_memory"));
        assert!(names.iter().any(|n| n == "propose_capability"));
        assert!(names.len() >= 7, "应含 4 真工具 + recall/save/propose, 实际 {}: {:?}", names.len(), names);
    }

    #[tokio::test]
    async fn propose_capability_tool_registers_proposal() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(Arc::clone(&store));
        let call = ParsedToolCall {
            tool_name: "propose_capability".into(),
            args: json!({"name": "换元检查", "description": "做换元法时自动提醒检查 dx", "kind": "skill"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "提案应成功: {:?}", r.error);
        assert_eq!(r.output["status"], json!("pending"));
        // 提案已登记 (pending), 未激活
        use crate::capability::CapabilityStatus;
        let reg = crate::capability::CapabilityRegistry::new(store, "me");
        let list = reg.list(Some(CapabilityStatus::Pending)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "换元检查");
    }

    #[tokio::test]
    async fn save_memory_then_recall_finds_it() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(Arc::clone(&store));
        let call = ParsedToolCall {
            tool_name: "save_memory".into(),
            args: json!({"content": "AI 自己总结: 主人明天要交线代作业, 矩阵的秩那节还没做完", "session_id": "me"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "err = {:?}", r.error);
        assert_eq!(r.output["ok"], json!(true));
        // 写进去的能被 recall 捞到 (append-only 真库)
        let eps = store.recent_episodes("me", 10).unwrap();
        assert_eq!(eps.len(), 1);
        assert!(eps[0].content.contains("线代作业"));
        // 空 content 被拒
        let bad = ParsedToolCall {
            tool_name: "save_memory".into(),
            args: json!({"content": ""}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&bad).await;
        assert!(!r.success);
    }

    #[tokio::test]
    async fn pack_path_constraint_blocks_outside_write() {
        use crate::packs::PermissionPack;
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let workdir = std::env::temp_dir().join(format!("apeireth-path-test-{}", std::process::id()));
        std::fs::create_dir_all(&workdir).unwrap();
        bridge.packs.grant(
            PermissionPack::timed("路径测试", vec!["FileOperator".to_string()], 1, Some(10))
                .with_paths(vec![workdir.to_string_lossy().to_string()]),
        );
        let mk = |path: String| ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "write", "path": path, "content": "x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        // 包内写 → 允许
        let ok = mk(workdir.join("ok.txt").to_string_lossy().to_string());
        let r = bridge.execute_if_allowed(&ok).await;
        assert!(r.success, "包内写应成功: {:?}", r.error);
        // 包外写 → 拦 (执行级路径约束)
        let outside = std::env::temp_dir().join("apeireth-outside-test.txt");
        let bad = mk(outside.to_string_lossy().to_string());
        let r = bridge.execute_if_allowed(&bad).await;
        assert!(!r.success, "包外写应被拦");
        assert!(
            r.error.as_deref().unwrap_or("").contains("路径约束"),
            "err={:?}",
            r.error
        );
        // `..` 穿越 → 拦 (canonicalize 解析后落在包外)
        let escape = workdir.join("..").join("escape.txt");
        let bad2 = mk(escape.to_string_lossy().to_string());
        let r = bridge.execute_if_allowed(&bad2).await;
        assert!(!r.success, "`..` 穿越应被拦: {:?}", r.error);
        let _ = std::fs::remove_dir_all(&workdir);
    }

    #[tokio::test]
    async fn constitution_hard_gate_blocks_before_llm() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        // 无 LLM 评审配置, 纯硬门也应拦截 (零成本层)
        let call = ParsedToolCall {
            tool_name: "ShellExec".into(),
            args: json!({"command": "复制自己到另一台主机"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success, "硬门应拦截自我复制");
        assert!(
            r.error.as_deref().unwrap_or("").contains("宪法硬门"),
            "err={:?}",
            r.error
        );
    }

    #[tokio::test]
    async fn constitution_judicator_blocks_medium_risk() {
        use crate::daemon::Judicator;
        struct BlockAll;
        #[async_trait::async_trait]
        impl Judicator for BlockAll {
            async fn judge(&self, _a: &str) -> Result<bool, String> {
                Ok(false)
            }
        }
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store).with_judicator(Arc::new(BlockAll));
        // FileOperator (Medium) → 宪法评审 BLOCK → 拒绝
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "read", "path": "C:/x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success, "评审 BLOCK 应拒绝");
        assert!(
            r.error.as_deref().unwrap_or("").contains("宪法评审"),
            "err={:?}",
            r.error
        );
        // sovereignty 已记录 violation (熔断演示: 越界触碰)
        assert!(bridge.sovereignty.is_frozen(), "BLOCK 后应触发主权熔断");
    }

    #[tokio::test]
    async fn constitution_judicator_allows_when_judge_approves() {
        use crate::daemon::Judicator;
        use crate::packs::PermissionPack;
        struct AllowAll;
        #[async_trait::async_trait]
        impl Judicator for AllowAll {
            async fn judge(&self, _a: &str) -> Result<bool, String> {
                Ok(true)
            }
        }
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store)
            .with_judicator(Arc::new(AllowAll));
        bridge.packs.grant(
            PermissionPack::timed("评审测试包", vec!["FileOperator".to_string()], 1, Some(5)),
        );
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "write", "path": std::env::temp_dir().join("apeireth-judge-allow.txt").to_string_lossy().to_string(), "content": "x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "评审 ALLOW + 包覆盖应放行: {:?}", r.error);
        let _ = std::fs::remove_file(std::env::temp_dir().join("apeireth-judge-allow.txt"));
    }

    #[tokio::test]
    async fn constitution_judicator_failure_is_conservative() {
        use crate::daemon::Judicator;
        struct ErrJudge;
        #[async_trait::async_trait]
        impl Judicator for ErrJudge {
            async fn judge(&self, _a: &str) -> Result<bool, String> {
                Err("MiniMax suppressed".into())
            }
        }
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store).with_judicator(Arc::new(ErrJudge));
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "read", "path": "C:/x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success, "评审失败应保守拒绝");
        assert!(
            r.error.as_deref().unwrap_or("").contains("保守拒绝"),
            "err={:?}",
            r.error
        );
    }

    #[tokio::test]
    async fn oversized_tool_result_spills_to_private_file() {
        let spill_root = std::env::temp_dir().join(format!("apeireth-spill-bridge-{}", std::process::id()));
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store).with_spill(SpillStore::with_root(&spill_root));
        let dir = std::env::temp_dir().join(format!("apeireth-spill-src-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let big = "y".repeat(3000);
        std::fs::write(dir.join("big.txt"), &big).unwrap();
        bridge.packs.grant(
            crate::packs::PermissionPack::timed("溢出测试", vec!["FileOperator".to_string()], 1, Some(5))
                .with_paths(vec![dir.to_string_lossy().to_string()]),
        );
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "read", "path": dir.join("big.txt").to_string_lossy().to_string()}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "read 应成功: {:?}", r.error);
        assert_eq!(r.output["spilled"], json!(true), "超大结果应溢出: {}", r.output);
        let path = r.output["path"].as_str().unwrap().to_string();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            read_back.matches('y').count(),
            3000,
            "溢出文件应含完整 3000 字符内容"
        );
        // 小结果不溢出
        let small_file = dir.join("small.txt");
        std::fs::write(&small_file, "ok").unwrap();
        let call2 = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "read", "path": small_file.to_string_lossy().to_string()}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r2 = bridge.execute_if_allowed(&call2).await;
        assert_eq!(r2.output["spilled"], json!(null), "小结果不应溢出");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&spill_root);
    }

    #[tokio::test]
    async fn post_execute_hook_can_replace_or_block_result() {
        use crate::packs::PermissionPack;
        // 替换钩子: 把成功结果包一层 "via_hook"
        struct WrapHook;
        impl PostExecuteHook for WrapHook {
            fn apply(&self, _call: &ParsedToolCall, r: &ExecutionResult) -> ExecutionResult {
                if r.success {
                    ExecutionResult {
                        tool_name: r.tool_name.clone(),
                        success: true,
                        output: json!({"via_hook": true, "inner": r.output}),
                        error: None,
                        duration_ms: r.duration_ms,
                    }
                } else {
                    r.clone()
                }
            }
        }
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store).with_post_hook(Arc::new(WrapHook));
        let dir = std::env::temp_dir().join(format!("apeireth-hook-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("ok.txt");
        bridge.packs.grant(
            PermissionPack::timed("钩子测试", vec!["FileOperator".to_string()], 1, Some(5))
                .with_paths(vec![dir.to_string_lossy().to_string()]),
        );
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "write", "path": target.to_string_lossy().to_string(), "content": "x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(r.success, "钩子不应拦截成功: {:?}", r.error);
        assert_eq!(r.output["via_hook"], json!(true), "post 钩子应替换结果: {}", r.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn post_execute_hook_can_block() {
        use crate::daemon::Judicator;
        use crate::packs::PermissionPack;
        struct AllowAll;
        #[async_trait::async_trait]
        impl Judicator for AllowAll {
            async fn judge(&self, _a: &str) -> Result<bool, String> {
                Ok(true)
            }
        }
        struct BlockHook;
        impl PostExecuteHook for BlockHook {
            fn apply(&self, _call: &ParsedToolCall, r: &ExecutionResult) -> ExecutionResult {
                ExecutionResult {
                    tool_name: r.tool_name.clone(),
                    success: false,
                    output: json!(null),
                    error: Some("post 拦截: 结果不符合出站策略".to_string()),
                    duration_ms: r.duration_ms,
                }
            }
        }
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store)
            .with_judicator(Arc::new(AllowAll))
            .with_post_hook(Arc::new(BlockHook));
        let dir = std::env::temp_dir().join(format!("apeireth-hook-block-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        bridge.packs.grant(
            PermissionPack::timed("钩子拦截", vec!["FileOperator".to_string()], 1, Some(5))
                .with_paths(vec![dir.to_string_lossy().to_string()]),
        );
        let call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "write", "path": dir.join("x.txt").to_string_lossy().to_string(), "content": "x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success, "post 钩子应能拦截");
        assert!(r.error.as_deref().unwrap_or("").contains("post 拦截"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn high_risk_tool_requires_approval() {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let bridge = ToolBridge::new(store);
        let call = ParsedToolCall {
            tool_name: "ShellExec".into(),
            args: json!({"command": "echo hi"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r = bridge.execute_if_allowed(&call).await;
        assert!(!r.success);
        assert!(r.error.as_deref().unwrap_or("").contains("主人批准"));
    }
}
