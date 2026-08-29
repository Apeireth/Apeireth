//! `apeireth-memory::procedural` — 程序性记忆与习惯固化引擎 (N.E.K.O 5 维记忆第五维 / 习惯规则链).
//!
//! ## 核心哲学 (S-1 北极星 + O-2 吸收前人经验)
//! 区别于事实性记忆 (Semantic) 与情景对话 (Episodic)，程序性记忆 (Procedural Memory) 记录的是：
//! **“如何做某事 (How-to)”与“用户的特定习惯与自动化动作链 (Condition-Action Habits)”**。
//!
//! 当系统检测到用户在特定情境下有反复高频的正确操作（例如特定的编译排错命令组合、特定代码重构偏好）：
//! 1. 自动记录触发条件 (Condition) 与动作配方 (Action Recipe)；
//! 2. 依据使用频次与成功率动态计算习惯置信度 (Confidence)；
//! 3. 在后续遇到相似情境时，优先通过模式匹配注入程序性经验，大幅减少 LLM 重新推演与犯错成本。
//!
//! ## 安全与约束
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 未定义行为；
//! - 有界存储（`max_habits` 防 OOM）；
//! - 零外部不可信 C-FFI 依赖，支持 Serde 全序列化。

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 程序性记忆错误.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProceduralError {
    #[error("触发条件或动作配方不能为空")]
    EmptyInput,
    #[error("未找到指定的习惯规则: {0}")]
    NotFound(String),
}

/// 单条习惯规则 / 技能配方.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitPattern {
    /// 唯一标识.
    pub id: String,
    /// 触发条件 / 场景模式 (例如 "cargo_build_err_missing_dep", "commit_message_format").
    pub trigger_condition: String,
    /// 固化的操作配方 / 技能指令.
    pub action_recipe: String,
    /// 累计匹配调用次数.
    pub usage_count: u64,
    /// 成功执行次数.
    pub success_count: u64,
    /// 当前计算置信度 (0.0..=1.0).
    pub confidence: f64,
    /// 最后一次使用毫秒时间戳.
    pub last_used_ms: i64,
    /// 是否已通过阈值晋升为高阶强规则.
    pub is_promoted: bool,
}

impl HabitPattern {
    pub fn new(id: impl Into<String>, trigger: impl Into<String>, recipe: impl Into<String>) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Self {
            id: id.into(),
            trigger_condition: trigger.into(),
            action_recipe: recipe.into(),
            usage_count: 1,
            success_count: 1,
            confidence: 0.8, // 初始先验置信度
            last_used_ms: now_ms,
            is_promoted: false,
        }
    }

    /// 记录一次执行反馈并更新置信度.
    pub fn record_feedback(&mut self, success: bool) {
        self.usage_count = self.usage_count.saturating_add(1);
        if success {
            self.success_count = self.success_count.saturating_add(1);
        }
        // 基于拉普拉斯平滑计算置信度: (success + 1) / (usage + 2)
        self.confidence = (self.success_count as f64 + 1.0) / (self.usage_count as f64 + 2.0);
        if self.usage_count >= 5 && self.confidence >= 0.85 {
            self.is_promoted = true;
        } else if self.confidence < 0.6 {
            self.is_promoted = false;
        }

        self.last_used_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
    }
}

/// 习惯匹配召回结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitMatch {
    pub habit: HabitPattern,
    /// 匹配度得分 (0.0..=1.0).
    pub match_score: f64,
}

/// 程序性记忆存储抽象 trait.
pub trait ProceduralStore: Send + Sync {
    /// 记录或更新一个习惯规则.
    fn record_habit(
        &self,
        trigger: &str,
        recipe: &str,
        success: bool,
    ) -> Result<HabitPattern, ProceduralError>;

    /// 匹配相关习惯规则 (按匹配分与置信度加权排序).
    fn match_habits(&self, condition_query: &str, top_k: usize) -> Vec<HabitMatch>;

    /// 手动晋升/固化特定习惯.
    fn promote_habit(&self, id: &str) -> Result<(), ProceduralError>;

    /// 列出所有已固化的习惯规则.
    fn list_promoted_habits(&self) -> Vec<HabitPattern>;

    /// 获取全部习惯数量.
    fn count(&self) -> usize;
}

/// 内存实现的程序性记忆管理器.
#[derive(Debug, Clone)]
pub struct InMemoryProceduralStore {
    habits: Arc<Mutex<HashMap<String, HabitPattern>>>,
    max_capacity: usize,
}

impl Default for InMemoryProceduralStore {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl InMemoryProceduralStore {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            habits: Arc::new(Mutex::new(HashMap::new())),
            max_capacity: max_capacity.max(1),
        }
    }

    fn generate_id(trigger: &str) -> String {
        let cleaned: String = trigger
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        format!("habit_{}", cleaned.to_lowercase())
    }

    /// 简单的模糊子串/分词重合度计算.
    fn compute_match_score(query: &str, trigger: &str) -> f64 {
        let q_lower = query.to_lowercase();
        let t_lower = trigger.to_lowercase();

        if q_lower == t_lower {
            return 1.0;
        }
        if q_lower.contains(&t_lower) || t_lower.contains(&q_lower) {
            return 0.85;
        }

        // 分词重合度 (按空格/下划线切分)
        let q_tokens: Vec<&str> = q_lower.split(|c: char| c.is_whitespace() || c == '_').filter(|s| !s.is_empty()).collect();
        let t_tokens: Vec<&str> = t_lower.split(|c: char| c.is_whitespace() || c == '_').filter(|s| !s.is_empty()).collect();

        if q_tokens.is_empty() || t_tokens.is_empty() {
            return 0.0;
        }

        let mut matched: u32 = 0;
        for qt in &q_tokens {
            if t_tokens.contains(qt) {
                matched += 1;
            }
        }

        f64::from(matched) / (q_tokens.len().max(t_tokens.len()) as f64)
    }
}

impl ProceduralStore for InMemoryProceduralStore {
    fn record_habit(
        &self,
        trigger: &str,
        recipe: &str,
        success: bool,
    ) -> Result<HabitPattern, ProceduralError> {
        let trigger = trigger.trim();
        let recipe = recipe.trim();
        if trigger.is_empty() || recipe.is_empty() {
            return Err(ProceduralError::EmptyInput);
        }

        let id = Self::generate_id(trigger);
        let mut lock = self.habits.lock().unwrap();

        if let Some(existing) = lock.get_mut(&id) {
            existing.action_recipe = recipe.to_string();
            existing.record_feedback(success);
            Ok(existing.clone())
        } else {
            // 容量控制
            if lock.len() >= self.max_capacity {
                // 淘汰最久未使用的习惯
                if let Some(oldest_key) = lock
                    .iter()
                    .min_by_key(|(_, h)| h.last_used_ms)
                    .map(|(k, _)| k.clone())
                {
                    lock.remove(&oldest_key);
                }
            }

            let mut habit = HabitPattern::new(&id, trigger, recipe);
            if !success {
                habit.record_feedback(false);
            }
            lock.insert(id, habit.clone());
            Ok(habit)
        }
    }

    fn match_habits(&self, condition_query: &str, top_k: usize) -> Vec<HabitMatch> {
        let lock = self.habits.lock().unwrap();
        let mut matches: Vec<HabitMatch> = lock
            .values()
            .filter_map(|h| {
                let score = Self::compute_match_score(condition_query, &h.trigger_condition);
                if score > 0.3 {
                    // 综合排序分 = 文本匹配分 * 0.6 + 置信度 * 0.4
                    let final_score = score * 0.6 + h.confidence * 0.4;
                    Some(HabitMatch {
                        habit: h.clone(),
                        match_score: final_score,
                    })
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(top_k);
        matches
    }

    fn promote_habit(&self, id: &str) -> Result<(), ProceduralError> {
        let mut lock = self.habits.lock().unwrap();
        let habit = lock.get_mut(id).ok_or_else(|| ProceduralError::NotFound(id.to_string()))?;
        habit.is_promoted = true;
        Ok(())
    }

    fn list_promoted_habits(&self) -> Vec<HabitPattern> {
        let lock = self.habits.lock().unwrap();
        let mut promoted: Vec<HabitPattern> = lock.values().filter(|h| h.is_promoted).cloned().collect();
        promoted.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        promoted
    }

    fn count(&self) -> usize {
        self.habits.lock().unwrap().len()
    }
}

/// 将召回的程序性习惯渲染为 Markdown 提示词 (可注入 System Prompt).
pub fn render_procedural_prompt(habits: &[HabitMatch]) -> String {
    if habits.is_empty() {
        return String::new();
    }

    let mut out = String::from("### 【程序性记忆与固化习惯 (Procedural Skills)】\n");
    for (i, m) in habits.iter().enumerate() {
        let star = if m.habit.is_promoted { " ⭐[已固化高阶习惯]" } else { "" };
        out.push_str(&format!(
            "{}. 场景: `{}` (置信度: {:.2}){}\n   推荐操作配方: {}\n",
            i + 1,
            m.habit.trigger_condition,
            m.habit.confidence,
            star,
            m.habit.action_recipe
        ));
    }
    out
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_update_habit_confidence() {
        let store = InMemoryProceduralStore::new(10);
        let habit = store
            .record_habit("cargo_test_fail", "运行 cargo test --offline 并检查第一处报错", true)
            .unwrap();

        assert_eq!(habit.trigger_condition, "cargo_test_fail");
        assert_eq!(habit.usage_count, 1);
        assert!((habit.confidence - 0.8).abs() < 1e-4);

        // 连续 5 次成功
        for _ in 0..5 {
            store
                .record_habit("cargo_test_fail", "运行 cargo test --offline 并检查第一处报错", true)
                .unwrap();
        }

        let matches = store.match_habits("cargo test fail", 1);
        assert_eq!(matches.len(), 1);
        assert!(matches[0].habit.is_promoted);
        assert!(matches[0].habit.confidence > 0.85);
    }

    #[test]
    fn test_match_habits_ranking() {
        let store = InMemoryProceduralStore::new(10);
        store.record_habit("git_conflict_resolve", "使用 git status 检查未合并文件", true).unwrap();
        store.record_habit("git_commit_style", "遵循 Angular commit message 规范", true).unwrap();

        let matches = store.match_habits("git conflict happen in repo", 5);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].habit.trigger_condition, "git_conflict_resolve");
    }

    #[test]
    fn test_render_procedural_prompt() {
        let store = InMemoryProceduralStore::new(10);
        store.record_habit("build_error", "查看 target/ 目录或检查 rustc 版本", true).unwrap();
        let matches = store.match_habits("build_error", 1);

        let rendered = render_procedural_prompt(&matches);
        assert!(rendered.contains("【程序性记忆与固化习惯"));
        assert!(rendered.contains("build_error"));
        assert!(rendered.contains("查看 target/ 目录"));
    }
}
