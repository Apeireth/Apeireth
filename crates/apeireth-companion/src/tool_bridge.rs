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

use crate::daemon::{Judicator, requires_llm_review};
use crate::packs::PackRegistry;
use crate::security::{SecurityGate, SovereigntyGate};

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
        let executor = ToolExecutor::new(registry.clone());
        // 权限包: 默认日常包 (永久, 只读工具 + 记忆写; 主人可 grant 自定义包扩权)
        let packs = PackRegistry::new();
        packs.grant(PackRegistry::default_daily_pack());
        let approval = ApprovalManager::with_rules(vec![
            Box::new(BlacklistRule::with_blacklist(Vec::<String>::new(), false)),
            Box::new(WhitelistRule::with_whitelist([
                "recall_memory".to_string(),
                "save_memory".to_string(),
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
        // 宪法评审 (真 LLM, 按原则判案): Medium+ 风险且配置了评审者 → 自动评审.
        // 只审动作摘要 (action + tool + args), 不审对话/记忆自由文本.
        if requires_llm_review(Self::tool_risk(&call.tool_name)) {
            if let Some(judge) = &self.judicator {
                let desc = format!("调用工具 {} 参数 {}", call.tool_name, call.args);
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
        assert!(names.len() >= 6, "应含 4 真工具 + recall_memory + save_memory, 实际 {}: {:?}", names.len(), names);
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
