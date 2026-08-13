//! R208 VCP 5 类高层分类 (基于现有 6 类 ToolKind, R185 调研推荐).
//!
//! **来源**: R185 VCP 官网调研提到 5 类插件 (工具 / 直觉反射 / 服务 / 消息预处理 / 消息分发).
//! R208 把现有 6 类 ToolKind 映射到 5 类高层分类.
//!
//! **0 触碰**: ToolKind 6 类 enum 0 改. VcpCategory 是 additive 上层分类.
//!
//! **不假装** (O-5):
//! - 5 类高层分类, 字段级引用 VCP 官网调研
//! - from_tool_kind() 显式映射 (6 -> 5), 编译期 hardcode
//! - 不假装 LLM 知道 5 类 vs 6 类, 仅提供 trait

#![allow(missing_docs)] // R208: 0 触碰现有 API 文档

use serde::{Deserialize, Serialize};

use crate::types::ToolKind;

/// VCP 5 类高层插件分类 (字段级引用 vcptoolbox.com/learn-vcp).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VcpCategory {
    /// 工具: 常规可调用
    Tool,
    /// 直觉反射: 常驻 hook 自动响应 (类似 systemd)
    ReactiveIntuition,
    /// 服务: 常驻后台
    Service,
    /// 消息预处理: 拦截 + 修改 (VCP 核心业务区)
    MessagePreprocessor,
    /// 消息分发: 路由到不同 Agent/前端
    MessageDispatcher,
}

impl VcpCategory {
    /// 5 类编译期 hardcode
    pub const COUNT: usize = 5;
    pub const ALL: [VcpCategory; 5] = [
        Self::Tool, Self::ReactiveIntuition, Self::Service,
        Self::MessagePreprocessor, Self::MessageDispatcher,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::ReactiveIntuition => "reactive_intuition",
            Self::Service => "service",
            Self::MessagePreprocessor => "message_preprocessor",
            Self::MessageDispatcher => "message_dispatcher",
        }
    }

    /// VCP 字段级原字符串 (借鉴 + 重命名, 0 直接使用)
    pub const fn as_legacy_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::ReactiveIntuition => "intuition",
            Self::Service => "service",
            Self::MessagePreprocessor => "preprocessor",
            Self::MessageDispatcher => "dispatcher",
        }
    }

    /// 6 类 -> 5 类映射
    pub fn from_tool_kind(kind: ToolKind) -> Self {
        match kind {
            // Sync / Async / Static 算 "工具"
            ToolKind::Sync | ToolKind::Async | ToolKind::Static => Self::Tool,
            // Service / Hybridservice 算 "服务"
            ToolKind::Service | ToolKind::Hybridservice => Self::Service,
            // MessagePreprocessor 直接映射
            ToolKind::MessagePreprocessor => Self::MessagePreprocessor,
        }
    }
}

// 编译期守门
const _: () = assert!(VcpCategory::COUNT == 5);
const _: () = assert!(VcpCategory::ALL.len() == 5);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_count() {
        assert_eq!(VcpCategory::COUNT, 5);
        assert_eq!(VcpCategory::ALL.len(), 5);
    }

    #[test]
    fn t02_as_str() {
        assert_eq!(VcpCategory::Tool.as_str(), "tool");
        assert_eq!(VcpCategory::ReactiveIntuition.as_str(), "reactive_intuition");
        assert_eq!(VcpCategory::Service.as_str(), "service");
        assert_eq!(VcpCategory::MessagePreprocessor.as_str(), "message_preprocessor");
        assert_eq!(VcpCategory::MessageDispatcher.as_str(), "message_dispatcher");
    }

    #[test]
    fn t03_as_legacy_str() {
        // 借鉴 VCP 5 类原名, 我们重命名为 system 视角
        assert_eq!(VcpCategory::Tool.as_legacy_str(), "tool");
        assert_eq!(VcpCategory::ReactiveIntuition.as_legacy_str(), "intuition");
    }

    #[test]
    fn t04_from_tool_kind_sync() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::Sync), VcpCategory::Tool);
    }

    #[test]
    fn t05_from_tool_kind_async() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::Async), VcpCategory::Tool);
    }

    #[test]
    fn t06_from_tool_kind_static() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::Static), VcpCategory::Tool);
    }

    #[test]
    fn t07_from_tool_kind_service() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::Service), VcpCategory::Service);
    }

    #[test]
    fn t08_from_tool_kind_hybrid() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::Hybridservice), VcpCategory::Service);
    }

    #[test]
    fn t09_from_tool_kind_message_preprocessor() {
        assert_eq!(VcpCategory::from_tool_kind(ToolKind::MessagePreprocessor),
                   VcpCategory::MessagePreprocessor);
    }

    #[test]
    fn t10_all_categories_distinct() {
        let all = VcpCategory::ALL;
        for i in 0..all.len() {
            for j in (i+1)..all.len() {
                assert_ne!(all[i], all[j], "categories {} and {} should differ", i, j);
            }
        }
    }
}
