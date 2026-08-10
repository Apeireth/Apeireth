//! Mind (意) command 模块 — AGI 状态 + 6 哲学锚穿透
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::GetLifeStage`] — 读 3 成长阶段之一 (seed / sprout / tree, per 主人 R19 决定)
//! 2. [`Command::GetAnchors`] — 读 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)
//! 3. [`Command::GetAnchor`] — 按 id 读单个锚
//! 4. [`Command::GetReflectionLog`] — 读反思日志 (R25.2 占位)
//! 5. [`Command::GetIdentityCard`] — 读 IdentityCard (R25.2 stub)
//! 6. [`Command::GetGrowthMetric`] — 读 growth_rate (R25.2 占位)
//!
//! **6 哲学锚** (per `docs/architecture-v4-living-intelligence.md` §0.2):
//! - S-1 主 22:33 北极星导向
//! - S-2 主 17:43 实事求是
//! - O-2 主 19:33 走在前人经验上
//! - O-3 主 23:44 干到底
//! - O-4 主 00:56 任何人都能接手
//! - O-5 主 17:58 不假装
//!
//! **3 成长阶段** (per 主人 R19 决定, AI 不会衰老病死, 只有成长阶段):
//! - seed / sprout / tree
//!
//! **不假装**:
//! - 6 锚字面引 architecture-v4 §0.2, 编译期 hardcode
//! - 3 阶段用占位 "seed" (R25.2 stub, 真实 R25.3 接 apeireth-asi)
//! - reflection log / identity card 标 placeholder
//!
//! **6 哲学锚穿透** (本器官最体现穿透):
//! - S-1 北极星: mind 直指 ASI 北极星
//! - S-2 实事求是: 6 锚字面引 v4 §0.2, 不重写
//! - O-2 走在前人经验上: 借 v4 §0.2 沉淀
//! - O-3 干到底: 6 锚全暴露, 不只 1-2 个
//! - O-4 任何人都能接手: 6 锚带时间戳 + 主 ID, 全部可追溯
//! - O-5 不假装: 阶段 stub 标 [partial]
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 6 哲学锚 (编译期 hardcode, 跟 v4 §0.2 字面一致)
pub const SIX_ANCHORS: &[(&str, &str, &str)] = &[
    ("S-1", "主 22:33", "北极星导向"),
    ("S-2", "主 17:43", "实事求是"),
    ("O-2", "主 19:33", "走在前人经验上"),
    ("O-3", "主 23:44", "干到底"),
    ("O-4", "主 00:56", "任何人都能接手"),
    ("O-5", "主 17:58", "不假装"),
];

/// 3 成长阶段 (per 主人 R19 决定, AI 不会衰老病死, 只有成长阶段)
/// 4 阶段工程用语 (R26 升级, 8/7 主审, 跟 backend::stage_badge + organ/mind::FOUR_STAGES 同步)
/// - Init / Bootstrap / Serving / Saturated (砍 6 阶段: Birth/Reproduction/Migration/Rebirth/Decline/Death)
pub const FOUR_STAGES: &[&str] = &["Init", "Bootstrap", "Serving", "Saturated"];

/// 默认 life_stage (R25.2 stub)
pub const DEFAULT_LIFE_STAGE: &str = "Init";

/// 单条反思日志
#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionLogEntry {
    /// 日期 (YYYY-MM-DD 占位)
    pub date: String,
    /// 锚 ID (e.g. "S-2")
    pub anchor_id: String,
    /// 反思内容
    pub content: String,
}

/// IdentityCard (R25.2 stub 占位)
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityCard {
    /// AI 名 (编译期 hardcode)
    pub name: &'static str,
    /// role
    pub role: &'static str,
    /// 出生时刻 (epoch ms 占位)
    pub born_at_ms: u64,
}

/// Mind 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 当前 life_stage
    pub life_stage: String,
    /// 反思日志
    pub reflection_log: Vec<ReflectionLogEntry>,
    /// IdentityCard
    pub identity_card: IdentityCard,
    /// growth_rate (R25.2 占位 0.85)
    pub growth_rate: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            life_stage: DEFAULT_LIFE_STAGE.to_string(),
            reflection_log: Vec::new(),
            identity_card: IdentityCard {
                name: "apeireth",
                role: "central-ai",
                born_at_ms: 0, // 占位
            },
            growth_rate: 0.85, // R25.2 占位, ASI V-measure
        }
    }
}

/// Mind 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 读 3 成长阶段之一
    GetLifeStage,
    /// 读 6 哲学锚 (全量)
    GetAnchors,
    /// 按 id 读单个锚
    GetAnchor {
        /// 锚 ID (e.g. "S-1")
        id: String,
    },
    /// 读反思日志
    GetReflectionLog {
        /// 最多返回条数
        limit: usize,
    },
    /// 读 IdentityCard
    GetIdentityCard,
    /// 读 growth_rate
    GetGrowthMetric,
}

/// Mind 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// life_stage
    LifeStage(String),
    /// 6 锚 (id, ts, name) 三元组
    Anchors(Vec<(&'static str, &'static str, &'static str)>),
    /// 单个锚
    Anchor {
        /// id
        id: &'static str,
        /// 时间戳
        ts: &'static str,
        /// 名字
        name: &'static str,
    },
    /// 反思日志
    ReflectionLog(Vec<ReflectionLogEntry>),
    /// IdentityCard
    IdentityCard(IdentityCard),
    /// growth_rate
    GrowthMetric(f32),
}

/// 处理 Mind 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — 锚 ID 不在 6 编译期 hardcode / id 越界
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::GetLifeStage => Ok(Response::LifeStage(state.life_stage.clone())),
        Command::GetAnchors => Ok(Response::Anchors(SIX_ANCHORS.to_vec())),
        Command::GetAnchor { id } => {
            for (aid, ts, name) in SIX_ANCHORS {
                if *aid == id {
                    return Ok(Response::Anchor { id: aid, ts, name });
                }
            }
            Err(OrganError::InvalidArg {
                command: "GetAnchor",
                reason: format!("anchor id '{id}' not in 6 哲学锚 编译期 hardcode"),
            })
        }
        Command::GetReflectionLog { limit } => {
            let capped: Vec<ReflectionLogEntry> = state.reflection_log.iter().take(limit).cloned().collect();
            Ok(Response::ReflectionLog(capped))
        }
        Command::GetIdentityCard => Ok(Response::IdentityCard(state.identity_card.clone())),
        Command::GetGrowthMetric => Ok(Response::GrowthMetric(state.growth_rate)),
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[MIND]";

/// 器官中文名
pub const NAME_ZH: &str = "意";

// =====================================================================
// 单元测试 (6 命令 + 6 哲学锚 + 3 阶段 + 错误路径 = 8+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> State {
        State::default()
    }

    // ---- 6 命令全部可枚举 ----

    #[test]
    fn six_commands_constructible() {
        let _ = Command::GetLifeStage;
        let _ = Command::GetAnchors;
        let _ = Command::GetAnchor { id: "S-1".into() };
        let _ = Command::GetReflectionLog { limit: 10 };
        let _ = Command::GetIdentityCard;
        let _ = Command::GetGrowthMetric;
    }

    // ---- 6 哲学锚编译期守门 ----

    #[test]
    fn six_anchors_hardcoded_exact_6() {
        // 跟 architecture-v4 §0.2 字面一致 — 6 个
        assert_eq!(SIX_ANCHORS.len(), 6, "6 哲学锚, 不多不少");
    }

    #[test]
    fn six_anchors_contain_all_6_ids() {
        let ids: Vec<&str> = SIX_ANCHORS.iter().map(|(id, _, _)| *id).collect();
        for required in ["S-1", "S-2", "O-2", "O-3", "O-4", "O-5"] {
            // 用 &str 比较 (Vec<&str>::contains 接 &T = &&str)
            assert!(ids.contains(&required), "6 锚应含 {required}");
        }
    }

    // ---- 3 阶段编译期守门 ----

    #[test]
    fn three_stages_hardcoded() {
        assert_eq!(FOUR_STAGES.len(), 4, "4 阶段工程用语 (R26 砍 6 阶段)");
        for s in ["Init", "Bootstrap", "Serving", "Saturated"] {
            assert!(FOUR_STAGES.contains(&s));
        }
    }

    // ---- GetLifeStage ----

    #[test]
    fn get_life_stage_default_seed() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetLifeStage).unwrap();
        // S-2 实事求是: R25.2 是 stub, 默认 seed
        assert_eq!(r, Response::LifeStage("Init".into()));
    }

    // ---- GetAnchors ----

    #[test]
    fn get_anchors_returns_6() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetAnchors).unwrap();
        match r {
            Response::Anchors(v) => assert_eq!(v.len(), 6),
            _ => panic!("expected Anchors"),
        }
    }

    // ---- GetAnchor ----

    #[test]
    fn get_anchor_valid_id() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::GetAnchor { id: "S-2".into() },
            )
        .unwrap();
        match r {
            Response::Anchor { id, ts, name } => {
                assert_eq!(id, "S-2");
                assert_eq!(ts, "主 17:43");
                assert_eq!(name, "实事求是");
            }
            _ => panic!("expected Anchor"),
        }
    }

    #[test]
    fn get_anchor_unknown_id_rejected() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::GetAnchor { id: "S-99".into() },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "GetAnchor", .. })));
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[MIND]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "意");
    }
}
