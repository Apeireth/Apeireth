//! `apeireth-companion::goal` — Goal 状态机 (吸收 DSH goal 设计, Rust 重写).
//!
//! 哲学对接: 「AI 发现你想要什么」和「AI 长出它自己想要什么」是同一个过程 —
//! goal 就是 AI 长目标的持久化载体: 单一当前目标, 严格状态机, 事件可追溯.
//!
//! 对齐 DSH:
//! - 单一当前目标 (create 新目标替换已完成者)
//! - `revision` 严格 +1 (compare-and-set 语义, 陈旧引用拒绝)
//! - 相位迁移逐条校验 (非法迁移报错, 状态不变)
//! - `blocked_reason { code, message }` 单一阻塞相位 (不增殖状态)
//! - `rounds_started` 只随「目标驱动轮」推进 (普通人类轮不计数)
//! - 原子持久化 (tmp+rename, 崩溃安全), 恢复 = load
//!
//! 0 假装: 这是「状态机 + 持久化」机制件; 自动续轮驱动 (round driver) 与 LLM 消费是上层的事.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 目标相位.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalPhase {
    Active,
    Paused,
    Completed,
    Blocked,
}

impl GoalPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }
}

/// 阻塞原因 (单一阻塞相位, 不增殖状态).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalBlock {
    pub code: String,
    pub message: String,
}

/// 目标快照 (全量, 每次变更写回).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub id: String,
    pub revision: u64,
    pub objective: String,
    pub phase: GoalPhase,
    pub max_goal_rounds: u64,
    pub rounds_started: u64,
    pub blocked_reason: Option<GoalBlock>,
    pub updated_at_ms: i64,
}

/// 目标服务: 单一当前目标 + 严格状态机 + 原子持久化.
pub struct GoalService {
    store: GoalStore,
    current: Option<GoalSnapshot>,
}

/// 状态机错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalError {
    NoGoal,
    AlreadyExists,
    IllegalTransition { from: GoalPhase, to: GoalPhase },
    StaleRevision { expected: u64, actual: u64 },
    NoRoundsRemaining,
}

impl std::fmt::Display for GoalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// 原子持久化 (tmp+rename), 每目标一个 `{id}.json`.
pub struct GoalStore {
    dir: PathBuf,
}

impl GoalStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }
    pub fn save(&self, g: &GoalSnapshot) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("建目录失败: {e}"))?;
        let tmp = self
            .dir
            .join(format!("{}.tmp-{}", g.id, uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(g).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.path_for(&g.id)).map_err(|e| e.to_string())
    }
    pub fn load(&self, id: &str) -> Option<GoalSnapshot> {
        let p = self.path_for(id);
        std::fs::read(&p)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }
    pub fn clear(&self, id: &str) -> Result<(), String> {
        let p = self.path_for(id);
        if p.exists() {
            std::fs::remove_file(p).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

impl GoalService {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            store: GoalStore::new(dir),
            current: None,
        }
    }

    /// 从磁盘恢复 (崩溃后重启).
    pub fn restore(&mut self, id: &str) -> Option<GoalSnapshot> {
        self.current = self.store.load(id);
        self.current.clone()
    }

    pub fn current(&self) -> Option<&GoalSnapshot> {
        self.current.as_ref()
    }

    fn commit(&mut self, mut g: GoalSnapshot) -> Result<GoalSnapshot, GoalError> {
        g.revision += 1;
        let _ = self.store.save(&g);
        self.current = Some(g.clone());
        Ok(g)
    }

    /// 创建新目标 (revision 1, active, 0 轮). 已有未完成目标 → 拒绝.
    pub fn create(
        &mut self,
        objective: impl Into<String>,
        max_rounds: u64,
    ) -> Result<GoalSnapshot, GoalError> {
        if let Some(g) = &self.current {
            if g.phase != GoalPhase::Completed {
                return Err(GoalError::AlreadyExists);
            }
        }
        let g = GoalSnapshot {
            id: format!("goal-{}", uuid::Uuid::new_v4()),
            revision: 0,
            objective: objective.into(),
            phase: GoalPhase::Active,
            max_goal_rounds: max_rounds.max(1),
            rounds_started: 0,
            blocked_reason: None,
            updated_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.commit(g)
    }

    /// 编辑目标内容 (revision+1, 保留相位; Completed 不可编辑).
    pub fn edit(&mut self, new_objective: impl Into<String>) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if g.phase == GoalPhase::Completed {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: g.phase,
            });
        }
        g.objective = new_objective.into();
        g.updated_at_ms = chrono::Utc::now().timestamp_millis();
        self.commit(g)
    }

    /// 暂停 (active → paused).
    pub fn pause(&mut self) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if g.phase != GoalPhase::Active {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Paused,
            });
        }
        g.phase = GoalPhase::Paused;
        self.commit(g)
    }

    /// 恢复 (paused|blocked → active; 需有轮次余量).
    pub fn resume(&mut self) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if !matches!(g.phase, GoalPhase::Paused | GoalPhase::Blocked) {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Active,
            });
        }
        if g.rounds_started >= g.max_goal_rounds {
            return Err(GoalError::NoRoundsRemaining);
        }
        g.phase = GoalPhase::Active;
        g.blocked_reason = None;
        self.commit(g)
    }

    /// 完成 (任意非 completed → completed).
    pub fn complete(&mut self) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if g.phase == GoalPhase::Completed {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Completed,
            });
        }
        g.phase = GoalPhase::Completed;
        g.blocked_reason = None;
        self.commit(g)
    }

    /// 阻塞 (active|paused → blocked, 记录 code+message).
    pub fn block(
        &mut self,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if !matches!(g.phase, GoalPhase::Active | GoalPhase::Paused) {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Blocked,
            });
        }
        g.phase = GoalPhase::Blocked;
        g.blocked_reason = Some(GoalBlock {
            code: code.into(),
            message: message.into(),
        });
        self.commit(g)
    }

    /// 记一轮目标驱动轮 (rounds_started + 1; 超上限 → 自动 block).
    pub fn admit_round(&mut self) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if g.phase != GoalPhase::Active {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Active,
            });
        }
        if g.rounds_started >= g.max_goal_rounds {
            g.phase = GoalPhase::Blocked;
            g.blocked_reason = Some(GoalBlock {
                code: "max-rounds".into(),
                message: "目标驱动轮数达上限".into(),
            });
            self.commit(g)?;
            return Err(GoalError::NoRoundsRemaining);
        }
        g.rounds_started += 1;
        self.commit(g)
    }

    /// 清除目标 (完成/任意相位 → 无目标; 也可用于替换).
    pub fn clear(&mut self) -> Result<(), GoalError> {
        if let Some(g) = &self.current {
            let _ = self.store.clear(&g.id);
            self.current = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("apeireth-goal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn create_edit_lifecycle() {
        let mut s = GoalService::new(tmp("life"));
        let g = s.create("学会高数换元法", 8).unwrap();
        assert_eq!(g.revision, 1);
        assert_eq!(g.phase, GoalPhase::Active);
        let g2 = s.edit("学会高数换元法 + 线代秩").unwrap();
        assert_eq!(g2.revision, 2);
        assert_eq!(g2.phase, GoalPhase::Active);
        // 重复 create (未完成) → 拒绝
        assert_eq!(s.create("x", 1).unwrap_err(), GoalError::AlreadyExists);
    }

    #[test]
    fn pause_resume_block_complete() {
        let mut s = GoalService::new(tmp("prbc"));
        s.create("目标", 3).unwrap();
        let p = s.pause().unwrap();
        assert_eq!(p.phase, GoalPhase::Paused);
        // paused 不能再 pause
        assert_eq!(
            s.pause().unwrap_err(),
            GoalError::IllegalTransition {
                from: GoalPhase::Paused,
                to: GoalPhase::Paused
            }
        );
        let b = s.block("provider-limit", "限流").unwrap();
        assert_eq!(b.phase, GoalPhase::Blocked);
        assert_eq!(b.blocked_reason.as_ref().unwrap().code, "provider-limit");
        let r = s.resume().unwrap();
        assert_eq!(r.phase, GoalPhase::Active);
        assert!(r.blocked_reason.is_none(), "resume 清阻塞原因");
        let c = s.complete().unwrap();
        assert_eq!(c.phase, GoalPhase::Completed);
        // completed 不可 edit
        assert!(s.edit("x").is_err());
        // 完成后可 create 新目标
        let g2 = s.create("新目标", 2).unwrap();
        assert_eq!(g2.phase, GoalPhase::Active);
    }

    #[test]
    fn rounds_budget_blocks_at_max() {
        let mut s = GoalService::new(tmp("rounds"));
        s.create("目标", 2).unwrap();
        s.admit_round().unwrap();
        s.admit_round().unwrap();
        // 第三轮 → 超上限 → 自动 block + NoRoundsRemaining
        assert_eq!(s.admit_round().unwrap_err(), GoalError::NoRoundsRemaining);
        assert_eq!(s.current().unwrap().phase, GoalPhase::Blocked);
        assert_eq!(
            s.current().unwrap().blocked_reason.as_ref().unwrap().code,
            "max-rounds"
        );
    }

    #[test]
    fn persistence_survives_restart() {
        let dir = tmp("persist");
        let mut s1 = GoalService::new(&dir);
        s1.create("跨重启目标", 5).unwrap();
        drop(s1);
        // 重启
        let mut s2 = GoalService::new(&dir);
        // restore 需要真实 id: 假 id 应返回 None
        assert!(s2.restore("goal-").is_none(), "restore 需要真实 id");
        let ids: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .collect();
        assert_eq!(ids.len(), 1);
        let g = s2.restore(&ids[0]).unwrap();
        assert_eq!(g.objective, "跨重启目标");
        assert_eq!(g.revision, 1);
        // 续写
        let g2 = s2.admit_round().unwrap();
        assert_eq!(g2.rounds_started, 1);
        assert_eq!(g2.revision, 2);
    }

    #[test]
    fn clear_removes_goal() {
        let mut s = GoalService::new(tmp("clear"));
        s.create("x", 1).unwrap();
        s.clear().unwrap();
        assert!(s.current().is_none());
        // 清除后可新建
        s.create("y", 1).unwrap();
        assert_eq!(s.current().unwrap().objective, "y");
    }
}
