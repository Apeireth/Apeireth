//! 6 历史流类型 (canonical, O-6 锚 #2 兑现, 2026-08-27).
//!
//! 6 流是 apeireth 历史流抽象 (per v2 era): 思想 / 提案 / 行动 / 关系 / 演化 / 反思.
//! 每个流对应一张表 + append-only 触发器. 跨 v1 / v2 / v2.0.0-rc 三个阶段 LOCKED.
//!
//! **位置**: 之前在 `apeireth-memory::StreamKind` (engine crate); O-6 重构批次
//! 搬到 `apeireth_core::kernel::StreamKind` (canonical), 让 plugin trait 能直接引用
//! (不再走 `&str` + `serde_json::Value` 占位, per v2-arch-refactor-batch.md "关于
//! StreamKind / HistoryEntry" 注释).
//!
//! **v1 compat**: `apeireth_memory::StreamKind` 通过 re-export 仍可访问, 100+
//! consumer 0 破 (类型一致, 不同路径).
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: domain enum 在 kernel 与 9 哲学锚 + 3 不变脊柱 同位, 跨 v2 三个阶段 LOCKED
//! 2. 系统: plugin → core (单向), memory → core (单向), 0 循环依赖
//! 3. 架构: 1 处 source-of-truth (kernel::StreamKind) + 1 处 legacy re-export (memory)
//!
//! **0 触碰 LOCKED**: enum 6 变体不动, 字段不动, Display/FromStr 不动 (6 名字符串兼容)

use serde::{Deserialize, Serialize};

/// 6 历史流类型 (canonical, LOCKED)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamKind {
    /// 思想流.
    Thought,
    /// 提案流.
    Proposal,
    /// 行动流.
    Action,
    /// 关系流.
    Relation,
    /// 演化流.
    Evolution,
    /// 反思期流.
    Reflection,
}

impl StreamKind {
    /// 6 变体的字符串名 (LOCKED, 与 SQLite 表名 1:1)
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thought => "thought",
            Self::Proposal => "proposal",
            Self::Action => "action",
            Self::Relation => "relation",
            Self::Evolution => "evolution",
            Self::Reflection => "reflection",
        }
    }

    /// 全部 6 变体 (for migrations + 索引迭代)
    pub const ALL: [StreamKind; 6] = [
        Self::Thought,
        Self::Proposal,
        Self::Action,
        Self::Relation,
        Self::Evolution,
        Self::Reflection,
    ];
}

/// 字符串 → StreamKind (用于 `MemoryBackend` trait method 用 typed enum 替代 `&str` 占位)
impl core::str::FromStr for StreamKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "thought" => Ok(Self::Thought),
            "proposal" => Ok(Self::Proposal),
            "action" => Ok(Self::Action),
            "relation" => Ok(Self::Relation),
            "evolution" => Ok(Self::Evolution),
            "reflection" => Ok(Self::Reflection),
            other => Err(format!(
                "unknown stream kind: {other}; expected one of 6: thought/proposal/action/relation/evolution/reflection"
            )),
        }
    }
}

impl core::fmt::Display for StreamKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// StreamKind 扩展 trait (memory-specific extension methods)
/// 必须在 core kernel 定义 (与 StreamKind 同 crate), 否则 orphan rule E0117
/// 让 memory crate 不能 `impl StreamKindExt for StreamKind`.
pub trait StreamKindExt {
    /// 返回对应的物理表名 (snake_case)
    fn table_name_ext(self) -> &'static str;
    /// D2 §5 对应的语义命名 (供 UI / 报告使用)
    fn semantic_name_ext(self) -> &'static str;
}

impl StreamKindExt for StreamKind {
    fn table_name_ext(self) -> &'static str {
        match self {
            Self::Thought => "thought_stream",
            Self::Proposal => "proposal_stream",
            Self::Action => "action_stream",
            Self::Relation => "relation_stream",
            Self::Evolution => "evolution_stream",
            Self::Reflection => "reflection_stream",
        }
    }

    fn semantic_name_ext(self) -> &'static str {
        match self {
            Self::Thought => "思想 (Thought)",
            Self::Proposal => "提案 (Proposal)",
            Self::Action => "行动 (Action)",
            Self::Relation => "关系 (Relation)",
            Self::Evolution => "演化 (Evolution)",
            Self::Reflection => "反思期 (Reflection Period)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LOCKED: 6 变体不动
    #[test]
    fn six_variants_locked() {
        let all: Vec<StreamKind> = StreamKind::ALL.to_vec();
        assert_eq!(all.len(), 6);
        assert_eq!(all[0], StreamKind::Thought);
        assert_eq!(all[5], StreamKind::Reflection);
    }

    /// roundtrip: as_str → FromStr 一致
    #[test]
    fn as_str_fromstr_roundtrip() {
        for kind in StreamKind::ALL {
            let s = kind.as_str();
            let parsed: StreamKind = s.parse().expect("parse");
            assert_eq!(parsed, kind);
        }
    }

    /// 未知字符串返 Err (不是 panic, 不是假装)
    #[test]
    fn unknown_kind_returns_err() {
        let result: Result<StreamKind, _> = "not-a-stream".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("unknown stream kind"));
    }

    /// Display 与 as_str 一致
    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", StreamKind::Thought), "thought");
        assert_eq!(format!("{}", StreamKind::Reflection), "reflection");
    }
}