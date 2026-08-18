//! `apeireth-companion::experience` — 自成长管道 Level 0/1: 经验库 + 经验驱动能力提案.
//!
//! 主人设想 (2026-08-16): 「沉淀总结经验入经验库」→ 反复验证成功 → 促 AI 提案能力.
//!
//! 设计:
//! - 经验 = 结构化可复用条目 {场景/做法/结果/结论/验证次数/EMA 评分}, 区别于个人记忆
//! - 存储: 复用 episodes (append-only), id 前缀 `exp-`, content = JSON (对齐 action_stream 模式)
//! - 验证: verify_experience(成功/失败) → verify_count++ + EMA score 更新
//! - 晋级: verify_count >= 3 且 score >= 0.7 → ready_for_capability (促 AI 用 propose_capability)
//!
//! 0 假装 (诚实):
//! - 验证来源未区分 (AI 复盘/主人反馈同走 verify, 后续可加 verifier 字段)
//! - EMA α=0.7 是初始参数 (对齐 capability.rs 的 EMA 语义), 未用真实数据拟合
//! - 「经验 → 能力提案」是提示驱动 (注入提示 + AI 自觉提案), 不是强制

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 经验条目 (存 episodes content JSON, id 前缀 exp-).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    /// 逻辑经验链标识 (同一经验的状态变更是新 id + 同 chain; 查询按 chain 取最新版).
    pub chain: String,
    /// 链内单调版本号 (变更 +1; 去重取 rev 最大, 与时间戳无关, 确定性).
    pub rev: u64,
    /// 触发场景 (何时用这条经验).
    pub scene: String,
    /// 做法.
    pub practice: String,
    /// 结果描述.
    pub result: String,
    /// 结论: success / failure / partial.
    pub outcome: String,
    /// 验证次数 (verify_experience 递增).
    pub verify_count: u64,
    /// EMA 评分 [0,1].
    pub score: f64,
    /// 是否达到提案阈值 (验证次数 + 评分).
    pub ready: bool,
    /// 是否已促提案 (防重复催促).
    pub proposed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 提案阈值 (编译期 hardcode, 0 假装: 初始参数).
pub const PROMOTE_MIN_VERIFIES: u64 = 3;
pub const PROMOTE_MIN_SCORE: f64 = 0.7;
/// EMA 平滑因子 (对齐 capability.rs).
pub const EMA_ALPHA: f64 = 0.7;

/// 经验库: 读写 + 验证 + 晋级判定 (基于 episodes, append-only).
pub struct ExperienceStore {
    store: Arc<SqliteMemoryStore>,
}

impl ExperienceStore {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }

    /// 沉淀一条经验 (append-only).
    pub fn save(&self, exp: &Experience) -> Result<(), String> {
        let content = serde_json::to_string(exp).map_err(|e| format!("序列化经验失败: {e}"))?;
        let ep = CoreEpisode {
            id: exp.id.clone(),
            timestamp: exp.created_at,
            role: "assistant".into(),
            content,
            session_id: "me".into(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// 列出全部经验 (按 chain 去重取最新版; updated_at 降序; scene 过滤可选).
    pub fn list(&self, scene: Option<&str>) -> Vec<Experience> {
        let eps = self.store.recent_episodes("me", 500).unwrap_or_default();
        let mut by_chain: std::collections::HashMap<String, Experience> =
            std::collections::HashMap::new();
        for e in eps
            .iter()
            .filter(|e| e.id.starts_with("exp-"))
            .filter_map(|e| serde_json::from_str::<Experience>(&e.content).ok())
            .filter(|x| scene.map_or(true, |s| x.scene.contains(s)))
        {
            // 按 rev 取最大 (确定性; 同 rev 时后读到的胜出)
            match by_chain.get(&e.chain) {
                Some(existing) if existing.rev > e.rev => {}
                _ => {
                    by_chain.insert(e.chain.clone(), e);
                }
            }
        }
        let mut out: Vec<Experience> = by_chain.into_values().collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        out
    }

    /// 验证一条经验 (成功/失败) → count++ + EMA 更新 + ready 判定.
    /// append-only 语义: 状态变更 = 新 id + 同 chain (旧版保留, 查询取最新).
    pub fn verify(&self, chain_or_id: &str, success: bool) -> Result<Experience, String> {
        let mut exps = self.list(None);
        let idx = exps
            .iter()
            .position(|e| e.chain == chain_or_id || e.id == chain_or_id)
            .ok_or_else(|| format!("经验不存在: {chain_or_id}"))?;
        let mut exp = exps.swap_remove(idx);
        let outcome_value = match (success, exp.outcome.as_str()) {
            (true, _) => 1.0,
            (false, "success") => 0.0,
            (false, "partial") => 0.3,
            (false, _) => 0.0,
        };
        exp.verify_count += 1;
        exp.score = exp.score * EMA_ALPHA + outcome_value * (1.0 - EMA_ALPHA);
        exp.ready = exp.verify_count >= PROMOTE_MIN_VERIFIES && exp.score >= PROMOTE_MIN_SCORE;
        exp.updated_at = chrono::Utc::now().timestamp_millis();
        exp.rev += 1; // 链内单调递增 (确定性版本)
        exp.id = format!("exp-{}", uuid::Uuid::new_v4()); // 新版本 id, chain 不变
        self.save(&exp)?;
        Ok(exp)
    }

    /// 达到提案阈值且未促提案的经验 (Level 1: 驱动 AI 提案能力).
    pub fn ready_for_capability(&self) -> Vec<Experience> {
        self.list(None)
            .into_iter()
            .filter(|e| e.ready && !e.proposed)
            .collect()
    }

    /// 标记已促提案 (防重复催促; append-only: 新 id + 同 chain).
    pub fn mark_proposed(&self, chain_or_id: &str) -> Result<(), String> {
        let mut exps = self.list(None);
        let idx = exps
            .iter()
            .position(|e| e.chain == chain_or_id || e.id == chain_or_id)
            .ok_or_else(|| format!("经验不存在: {chain_or_id}"))?;
        let mut exp = exps.swap_remove(idx);
        exp.proposed = true;
        exp.updated_at = chrono::Utc::now().timestamp_millis();
        exp.rev += 1;
        exp.id = format!("exp-{}", uuid::Uuid::new_v4());
        self.save(&exp)
    }

    /// 注入提示文本: 待提案经验清单 (对话预处理链用).
    pub fn build_promotion_hint(&self) -> String {
        let ready = self.ready_for_capability();
        if ready.is_empty() {
            return String::new();
        }
        let mut s = String::from(
            "【经验晋级提示】以下经验已验证达标, 考虑用 propose_capability 提案为能力:\n",
        );
        for e in ready.iter().take(5) {
            s.push_str(&format!(
                "  • {} (验证 {} 次, 评分 {:.2}) — 做法: {}\n",
                e.scene, e.verify_count, e.score, e.practice
            ));
        }
        s
    }
}

// ============================================================
// 工具: save_experience / list_experience / verify_experience
// ============================================================

/// 「沉淀经验」工具 — 自成长管道入口 (经验库写入).
pub struct SaveExperienceTool {
    store: Arc<SqliteMemoryStore>,
}

impl SaveExperienceTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for SaveExperienceTool {
    fn name(&self) -> &str {
        "save_experience"
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let scene = args
            .get("scene")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "scene (触发场景) 不能为空".to_string())?;
        let practice = args
            .get("practice")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "practice (做法) 不能为空".to_string())?;
        let result = args
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let outcome = args
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("partial")
            .trim()
            .to_string();
        if !["success", "failure", "partial"].contains(&outcome.as_str()) {
            return Err("outcome 应为 success/failure/partial".to_string());
        }
        let now = chrono::Utc::now().timestamp();
        let id = format!("exp-{}", uuid::Uuid::new_v4());
        let exp = Experience {
            id: id.clone(),
            chain: id,
            rev: 1,
            scene: scene.to_string(),
            practice: practice.to_string(),
            result,
            outcome,
            verify_count: 0,
            score: 0.5,
            ready: false,
            proposed: false,
            created_at: now,
            updated_at: now,
        };
        let store = ExperienceStore::new(Arc::clone(&self.store));
        store.save(&exp)?;
        Ok(
            json!({"ok": true, "id": exp.id, "note": "经验已入经验库, 验证 3 次且评分达标后自动促能力提案"}),
        )
    }
}

/// 「查经验」工具 — 经验库查询 (只读).
pub struct ListExperienceTool {
    store: Arc<SqliteMemoryStore>,
}

impl ListExperienceTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for ListExperienceTool {
    fn name(&self) -> &str {
        "list_experience"
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let scene = args
            .get("scene")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let store = ExperienceStore::new(Arc::clone(&self.store));
        let list = store.list(scene);
        Ok(json!({
            "count": list.len(),
            "experiences": list.iter().map(|e| json!({
                "id": e.id, "scene": e.scene, "practice": e.practice,
                "result": e.result, "outcome": e.outcome,
                "verify_count": e.verify_count, "score": e.score,
                "ready": e.ready, "proposed": e.proposed,
            })).collect::<Vec<_>>(),
            "note": "经验库是自成长管道地基; ready=true 的经验可提案为能力"
        }))
    }
}

/// 「验证经验」工具 — 复盘/反馈时验证经验 (count++ + EMA + 晋级判定).
pub struct VerifyExperienceTool {
    store: Arc<SqliteMemoryStore>,
}

impl VerifyExperienceTool {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl apeireth_tool_registry::Tool for VerifyExperienceTool {
    fn name(&self) -> &str {
        "verify_experience"
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        apeireth_tool_registry::ToolKind::Sync
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "需要 id (经验 id)".to_string())?;
        let success = args
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let store = ExperienceStore::new(Arc::clone(&self.store));
        let exp = store.verify(id, success)?;
        Ok(json!({
            "ok": true, "id": exp.id,
            "verify_count": exp.verify_count, "score": exp.score,
            "ready": exp.ready,
            "note": if exp.ready { "已验证达标, 建议提案为能力 (propose_capability)" } else { "继续验证" }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::Tool;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    fn sample(id: &str) -> Experience {
        Experience {
            id: id.into(),
            chain: id.into(),
            rev: 1,
            scene: "主人学习高数换元法".into(),
            practice: "精练8题+套路卡, 每题自检 dx".into(),
            result: "错题率下降".into(),
            outcome: "success".into(),
            verify_count: 0,
            score: 0.5,
            ready: false,
            proposed: false,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn save_list_roundtrip() {
        let s = ExperienceStore::new(store());
        s.save(&sample("exp-1")).unwrap();
        let list = s.list(None);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].scene, "主人学习高数换元法");
        // scene 过滤
        assert_eq!(s.list(Some("高数")).len(), 1);
        assert_eq!(s.list(Some("线代")).len(), 0);
    }

    #[test]
    fn verify_accumulates_and_promotes() {
        let s = ExperienceStore::new(store());
        s.save(&sample("exp-v")).unwrap();
        // 3 次成功验证 (前两次未达标, 第三次达标)
        let mut last = s.verify("exp-v", true).unwrap();
        assert!(!last.ready, "1 次验证未达标");
        last = s.verify("exp-v", true).unwrap();
        assert!(!last.ready, "2 次验证未达标");
        last = s.verify("exp-v", true).unwrap();
        assert!(last.ready, "3 次成功 + score>=0.7 → ready");
        assert_eq!(last.verify_count, 3);
        let ready = s.ready_for_capability();
        assert_eq!(ready.len(), 1);
        // 版本化: 同一 chain 只保留最新 (list 去重)
        assert_eq!(s.list(None).len(), 1);
        // 失败验证拉低评分
        let e = s.verify("exp-v", false).unwrap();
        assert!(e.score < 0.9);
        // 标记提案后不再催促
        s.mark_proposed("exp-v").unwrap();
        assert!(s.ready_for_capability().is_empty());
    }

    #[test]
    fn failure_never_promotes() {
        let s = ExperienceStore::new(store());
        let mut exp = sample("exp-f");
        exp.outcome = "failure".into();
        s.save(&exp).unwrap();
        for _ in 0..5 {
            let _ = s.verify("exp-f", false).unwrap();
        }
        let ready = s.ready_for_capability();
        assert!(ready.is_empty(), "失败经验不晋级");
    }

    #[tokio::test]
    async fn tools_work_via_store() {
        let st = store();
        let save = SaveExperienceTool::new(Arc::clone(&st));
        let r = save.call(json!({"scene": "写 Rust 代码", "practice": "先写测试再实现", "outcome": "success"})).await.unwrap();
        let id = r["id"].as_str().unwrap().to_string();
        let list = ListExperienceTool::new(Arc::clone(&st));
        let l = list.call(json!({})).await.unwrap();
        assert_eq!(l["count"], json!(1));
        let verify = VerifyExperienceTool::new(Arc::clone(&st));
        let v = verify
            .call(json!({"id": id, "success": true}))
            .await
            .unwrap();
        assert_eq!(v["verify_count"], json!(1));
        // 非法 outcome 拒绝
        assert!(save
            .call(json!({"scene": "x", "practice": "y", "outcome": "maybe"}))
            .await
            .is_err());
    }
}
