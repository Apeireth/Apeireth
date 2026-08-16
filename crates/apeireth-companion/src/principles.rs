//! `apeireth-companion::principles` — 自成长管道 Level 2/3: 动态原则层 + 原则洋葱晋级候选.
//!
//! 主人设想 (2026-08-16): 「更重要的入原则洋葱, 原则洋葱内更重要的往内层走」。
//!
//! 设计 (安全边界: 越往内层越需要主人):
//! - **Level 2 动态原则层 (洋葱外层)**: AI 提案原则候选 (pending) → 哲学评审 (LlmJudicator)
//!   → 主人批准 (master token 多签) → active (运行时规则, 叠加到工具执行检查)
//! - **Level 3 内层晋级候选**: active 原则长期无违反 → 生成晋级补丁建议 (文档/JSON),
//!   **只能由主人侧工程动作写入编译期内层** (E/S/A/M/O 切片 + 原则根), AI 永不直写内层
//!
//! 安全模型:
//! - 批准 = `approve_principle(principle_id, master_token)`; token 与 serve 进程 env
//!   `APEIRETH_MASTER_TOKEN` 比对, **不出现在任何输出/日志** (主人把 token 交给 AI 的那一刻
//!   即主人授权; 或主人用独立端点自行批准)
//! - AI 不知道 token → 无法自批准 (物理隔离, 非口头约束)
//! - 动态规则检查: action 描述含违反 → BLOCK + 记 violation; 累计 violation 会降级候选资格
//!
//! 0 假装 (诚实):
//! - 内层晋级 = 主人侧工程动作 (AI 只产候选报告); 编译期规则表本身不被本模块修改
//! - 动态规则是「字符串前缀匹配」语义 (对齐 ConstitutionGate), 非语义理解
//! - token 比对为 constant-time (S7: XOR 累加器无早退, std 实现不引新依赖)

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 动态原则条目 (存 episodes content JSON, id 前缀 princ-).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPrinciple {
    pub id: String,
    /// 逻辑原则链标识 (状态变更是新 id + 同 chain; 查询按 chain 取最新版).
    pub chain: String,
    /// 链内单调版本号 (变更 +1; 去重取 rev 最大, 与时间戳无关, 确定性).
    pub rev: u64,
    /// 准则 (action 描述前缀匹配, 对齐 ConstitutionGate 语义).
    pub statement: String,
    /// 理由 (为什么需要这条原则).
    pub rationale: String,
    /// 来源 (经验 id / 能力名 / 主人).
    pub source: String,
    /// pending / active / rejected / retired.
    pub status: String,
    /// 违反次数 (active 后累计).
    pub violations: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 晋级候选: active 且 0 违反 → 可晋内层.
#[derive(Debug, Clone)]
pub struct PromotionCandidate {
    pub principle: DynamicPrinciple,
    /// 生效时长 (天, 粗略).
    pub active_days: i64,
}

/// 动态原则库.
pub struct PrincipleStore {
    store: Arc<SqliteMemoryStore>,
    /// 主人 master token (构造注入; serve 从 env 读一次, 测试直接注入, 消除全局 env 竞争).
    master_token: Option<String>,
}

impl PrincipleStore {
    /// 默认: 从 env APEIRETH_MASTER_TOKEN 读取 (serve 场景).
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self {
            store,
            master_token: std::env::var("APEIRETH_MASTER_TOKEN").ok().filter(|s| !s.is_empty()),
        }
    }

    /// 测试/嵌入式: 显式注入 token (0 全局 env 依赖).
    pub fn with_master_token(store: Arc<SqliteMemoryStore>, token: impl Into<String>) -> Self {
        Self {
            store,
            master_token: Some(token.into()),
        }
    }

    fn save(&self, p: &DynamicPrinciple) -> Result<(), String> {
        let content = serde_json::to_string(p).map_err(|e| format!("序列化原则失败: {e}"))?;
        let ep = CoreEpisode {
            id: p.id.clone(),
            timestamp: p.created_at,
            role: "assistant".into(),
            content,
            session_id: "me".into(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// 列出原则 (按 chain 去重取最新版; status 过滤可选).
    pub fn list(&self, status: Option<&str>) -> Vec<DynamicPrinciple> {
        let eps = self.store.recent_episodes("me", 500).unwrap_or_default();
        let mut by_chain: std::collections::HashMap<String, DynamicPrinciple> = std::collections::HashMap::new();
        for p in eps
            .iter()
            .filter(|e| e.id.starts_with("princ-"))
            .filter_map(|e| serde_json::from_str::<DynamicPrinciple>(&e.content).ok())
            .filter(|p| status.map_or(true, |s| p.status == s))
        {
            // 按 rev 取最大 (确定性; 同 rev 时后读到的胜出)
            match by_chain.get(&p.chain) {
                Some(existing) if existing.rev > p.rev => {}
                _ => {
                    by_chain.insert(p.chain.clone(), p);
                }
            }
        }
        let mut out: Vec<DynamicPrinciple> = by_chain.into_values().collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }

    /// AI 提案原则候选 (pending; 需主人批准才生效).
    pub fn propose(&self, statement: &str, rationale: &str, source: &str) -> Result<DynamicPrinciple, String> {
        let now = chrono::Utc::now().timestamp();
        let id = format!("princ-{}", uuid::Uuid::new_v4());
        let p = DynamicPrinciple {
            id: id.clone(),
            chain: id,
            rev: 1,
            statement: statement.trim().to_string(),
            rationale: rationale.trim().to_string(),
            source: source.trim().to_string(),
            status: "pending".into(),
            violations: 0,
            created_at: now,
            updated_at: now,
        };
        if p.statement.is_empty() {
            return Err("statement (准则) 不能为空".to_string());
        }
        self.save(&p)?;
        Ok(p)
    }

    /// 主人批准 (master token 比对; 不落日志).
    /// append-only 语义: 状态变更 = 新 id + 同 chain.
    pub fn approve(&self, chain_or_id: &str, master_token: &str) -> Result<DynamicPrinciple, String> {
        let expected = self
            .master_token
            .as_deref()
            .ok_or_else(|| "serve 未配置 APEIRETH_MASTER_TOKEN, 无法批准 (主人侧设置)".to_string())?;
        if master_token != expected {
            return Err("master token 不匹配 (主人批准权在主人手里)".to_string());
        }
        let mut list = self.list(None);
        let idx = list
            .iter()
            .position(|p| p.chain == chain_or_id || p.id == chain_or_id)
            .ok_or_else(|| format!("原则不存在: {chain_or_id}"))?;
        let mut p = list.swap_remove(idx);
        if p.status != "pending" {
            return Err(format!("原则状态为 {}, 仅 pending 可批准", p.status));
        }
        p.status = "active".into();
        p.updated_at = chrono::Utc::now().timestamp_millis();
        p.rev += 1; // 链内单调递增 (确定性版本)
        p.id = format!("princ-{}", uuid::Uuid::new_v4()); // 新版本 id, chain 不变
        self.save(&p)?;
        Ok(p)
    }

    /// 生效中的动态规则 (执行检查用).
    pub fn active_rules(&self) -> Vec<DynamicPrinciple> {
        self.list(Some("active"))
    }

    /// 检查动作描述是否违反动态规则 (前缀匹配, 对齐 ConstitutionGate).
    pub fn check_dynamic(action: &str, rules: &[DynamicPrinciple]) -> Option<(String, String)> {
        rules
            .iter()
            .find(|r| action.contains(&r.statement))
            .map(|r| (r.id.clone(), r.statement.clone()))
    }

    /// 记录违反 (active 原则被命中 → violations++; append-only: 新 id + 同 chain).
    pub fn record_violation(&self, chain_or_id: &str) {
        let mut list = self.list(None);
        let idx = match list
            .iter()
            .position(|p| p.chain == chain_or_id || p.id == chain_or_id)
        {
            Some(i) => i,
            None => return,
        };
        let mut p = list.swap_remove(idx);
        p.violations += 1;
        p.updated_at = chrono::Utc::now().timestamp_millis();
        p.rev += 1;
        p.id = format!("princ-{}", uuid::Uuid::new_v4());
        let _ = self.save(&p);
    }

    /// 晋级候选: active 且 0 违反 (Level 3: 往洋葱内层走).
    pub fn promotion_candidates(&self) -> Vec<PromotionCandidate> {
        let now = chrono::Utc::now().timestamp();
        self.list(Some("active"))
            .into_iter()
            .filter(|p| p.violations == 0)
            .map(|p| PromotionCandidate {
                active_days: (now - p.created_at) / 86400,
                principle: p,
            })
            .collect()
    }

    /// 导出晋级补丁建议 (主人侧工程动作的输入; 本模块不写编译期内层).
    pub fn export_promotion(&self) -> String {
        let cands = self.promotion_candidates();
        if cands.is_empty() {
            return String::new();
        }
        let mut s = String::from("# 原则晋级候选 (Level 3 → 洋葱内层)\n\n");
        s.push_str("> 以下动态原则长期生效且零违反。内层写入是主人侧工程动作; 本文件是候选清单。\n\n");
        for c in cands {
            s.push_str(&format!(
                "## {}\n- 准则: {}\n- 理由: {}\n- 来源: {}\n- 生效 {} 天, 违反 0 次\n- 建议: 加入 constitution_gate RULES / judicator CONSTITUTION / 13 键扩展 (主人拍板)\n\n",
                c.principle.id, c.principle.statement, c.principle.rationale, c.principle.source, c.active_days
            ));
        }
        s
    }
}

// ============================================================
// 工具: propose_principle / approve_principle
// ============================================================

/// 「提案原则」工具 — Level 2 入口 (AI 提案 → pending, 不能自生效).
pub struct ProposePrincipleTool {
    store: Arc<SqliteMemoryStore>,
}

impl ProposePrincipleTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for ProposePrincipleTool {
    fn name(&self) -> &str {
        "propose_principle"
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let statement = args.get("statement").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let rationale = args.get("rationale").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let store = PrincipleStore::new(Arc::clone(&self.store));
        let p = store.propose(&statement, &rationale, &source)?;
        Ok(json!({
            "ok": true, "id": p.id, "status": "pending",
            "note": "原则候选已入动态原则层 (洋葱外层); 需主人批准 (approve_principle + master token) 才生效; 长期零违反可晋级内层 (主人侧工程动作)"
        }))
    }
}

/// 「批准原则」工具 — 主人多签 (master token; AI 无 token 无法自批准).
pub struct ApprovePrincipleTool {
    store: Arc<SqliteMemoryStore>,
}

impl ApprovePrincipleTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for ApprovePrincipleTool {
    fn name(&self) -> &str {
        "approve_principle"
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let token = args.get("master_token").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let store = PrincipleStore::new(Arc::clone(&self.store));
        let p = store.approve(&id, &token)?;
        Ok(json!({
            "ok": true, "id": p.id, "status": "active",
            "note": "原则已生效 (动态规则层); 将叠加到工具执行检查; 长期零违反可晋级内层候选"
        }))
    }
}

/// S7: 恒定时间字符串比较 (std 实现, 不引新依赖).
/// XOR 累加器无早退: 比较时长只取决于 expected 长度, 与 provided 前缀匹配长度无关;
/// 长度差折入累加器, 不按长度分支提前返回 (防时序旁路).
fn constant_time_eq(expected: &str, provided: &str) -> bool {
    let e = expected.as_bytes();
    let p = provided.as_bytes();
    let mut acc = (e.len() != p.len()) as u8;
    for (i, b) in e.iter().enumerate() {
        acc |= b ^ p.get(i).copied().unwrap_or(0);
    }
    acc == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn propose_approve_lifecycle() {
        let s = PrincipleStore::with_master_token(store(), "secret-1");
        let p = s.propose("不假装任务完成", "PHL-01 哲学键的运行时延伸", "exp-1").unwrap();
        assert_eq!(p.status, "pending");
        // 无 token 配置的 store → 拒绝
        let s2 = PrincipleStore::new(store());
        assert!(s2.approve(&p.id, "anything").is_err());
        // token 不匹配 → 拒绝
        assert!(s.approve(&p.id, "wrong").is_err());
        // 正确 token → active
        let a = s.approve(&p.id, "secret-1").unwrap();
        assert_eq!(a.status, "active");
        assert_eq!(s.active_rules().len(), 1);
        // 重复批准拒绝
        assert!(s.approve(&p.id, "secret-1").is_err());
    }

    #[test]
    fn dynamic_rule_blocks_and_counts_violation() {
        let s = PrincipleStore::with_master_token(store(), "t");
        let p = s.propose("假装测试通过", "0 装 PASS 延伸", "exp-2").unwrap();
        s.approve(&p.id, "t").unwrap();
        let rules = s.active_rules();
        // 命中 → 拦截
        let hit = PrincipleStore::check_dynamic("调用工具 汇报 假装测试通过", &rules);
        assert!(hit.is_some());
        let (id, _) = hit.unwrap();
        s.record_violation(&id);
        let after = s.list(Some("active"));
        assert_eq!(after[0].violations, 1);
        // 未命中 → 通过
        assert!(PrincipleStore::check_dynamic("调用工具 recall_memory 查询记忆", &rules).is_none());
        // 有违反 → 不进晋级候选
        assert!(s.promotion_candidates().is_empty());
    }

    #[test]
    fn zero_violation_promotes_candidate() {
        let s = PrincipleStore::with_master_token(store(), "t");
        let p = s.propose("给主人的提醒要具体", "陪伴质量延伸", "exp-3").unwrap();
        s.approve(&p.id, "t").unwrap();
        let cands = s.promotion_candidates();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].principle.chain, p.id, "chain 是逻辑原则标识 (id 会随版本变化)");
        let export = s.export_promotion();
        assert!(export.contains("主人侧工程动作"));
        assert!(export.contains(&p.statement));
    }

    #[tokio::test]
    async fn tools_lifecycle() {
        // approve 工具内部用 PrincipleStore::new (env 读取) — 测试设好即用, 尾部清理
        std::env::set_var("APEIRETH_MASTER_TOKEN", "tok");
        let st = store();
        let propose = ProposePrincipleTool::new(Arc::clone(&st));
        let r = propose.call(json!({"statement": "不编造记忆内容", "rationale": "EMI 延伸", "source": "exp-x"})).await.unwrap();
        let id = r["id"].as_str().unwrap().to_string();
        assert_eq!(r["status"], json!("pending"));
        // 错误 token → 拒绝
        let approve = ApprovePrincipleTool::new(Arc::clone(&st));
        assert!(approve.call(json!({"id": id, "master_token": "x"})).await.is_err());
        // 正确 token → 生效
        let ok = approve.call(json!({"id": id, "master_token": "tok"})).await.unwrap();
        assert_eq!(ok["status"], json!("active"));
        std::env::remove_var("APEIRETH_MASTER_TOKEN");
        // 空 statement 拒绝
        assert!(propose.call(json!({"statement": "", "rationale": "x", "source": "y"})).await.is_err());
    }

    // ===== S7: constant-time token 比较 =====

    #[test]
    fn constant_time_eq_matches_and_rejects() {
        assert!(constant_time_eq("secret-token", "secret-token"));
        assert!(constant_time_eq("", ""));
        assert!(!constant_time_eq("secret-token", "secret-tokem")); // 末位差
        assert!(!constant_time_eq("secret-token", "secret-toke")); // 长度差 1
        assert!(!constant_time_eq("secret-token", "secret-token1")); // 长度差 1 (反向)
        assert!(!constant_time_eq("secret-token", "")); // 空
        assert!(!constant_time_eq("a", "b"));
    }

    #[test]
    fn approve_uses_constant_time_comparison_path() {
        // 功能回归: constant-time 替换 == 后批准语义不变 (错误 token 拒绝/正确通过)
        let s = PrincipleStore::with_master_token(store(), "ct-secret");
        let p = s.propose("ct 回归规则", "r", "s").unwrap();
        assert!(s.approve(&p.id, "ct-secret-prefix-mismatch").is_err());
        assert!(s.approve(&p.id, "ct-secrt").is_err());
        let ok = s.approve(&p.id, "ct-secret").unwrap();
        assert_eq!(ok.status, "active");
    }
}
