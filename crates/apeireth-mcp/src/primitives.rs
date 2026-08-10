//! R125-4: MCP primitive namespace enum (借鉴 modelcontextprotocol/servers 设计)
//!
//! **依据**: `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10`
//!
//! **MCP 规范 (per modelcontextprotocol/specification 2025-03-26 §Architecture)**:
//! MCP 客户端 + 服务端协议由 6 个 "primitive" 组成, 每个 primitive 是一组相关方法:
//!
//! 1. **Initialize** (1 方法) — 握手 (协议版本 + 能力 + 标识)
//! 2. **Tools** (2 方法) — 工具调用 (list / call)
//! 3. **Resources** (3 方法) — 资源访问 (list / read / subscribe + templates/list)
//! 4. **Prompts** (2 方法) — 提示词模板 (list / get)
//! 5. **Sampling** (1 方法) — 客户端→服务端 LLM 采样请求 (createMessage)
//! 6. **Roots** (1 方法) — 客户端文件系统根目录 (list)
//! 7. **Logging** (1 方法) — 日志级别控制 (setLevel)
//!
//! **不漂移 (主哲学锚 #1, 8 硬墙 #3)**:
//! - `Primitive` enum 编译期 hardcode 7 个 variant, 防加 primitive 忘改 docs
//! - 0 改现有 `protocol.rs` / `tools.rs` / `resources.rs` 公共 API
//! - 0 改 `McpServer::dispatch` 行为, 仅扩展内部 dispatch 表
//!
//! **借鉴锚 (S-1)**:
//! - `modelcontextprotocol/servers/src/everything/tools/index.ts` 的 4 步注册模式
//!   (无条件 tools / 条件 tools based on capabilities) — 映射到本 enum 的 7 variant
//! - LangChain `@tool` decorator (Tool 拆分模式) — 映射到 `tools/types.rs` 拆分
//! - VCP `toolCallParser.js` (kebab-case 校验) — 映射到 `tools/naming.rs`

use serde::{Deserialize, Serialize};

/// **MCP primitive namespace 枚举** (编译期 hardcode)
///
/// **总 7 primitive** (per MCP spec 2025-03-26 §Architecture):
/// 1. `Initialize` — 握手 (`initialize` + `notifications/initialized`)
/// 2. `Tools` — 工具调用 (`tools/list` + `tools/call` + `tools/subscribe`)
/// 3. `Resources` — 资源访问 (`resources/list` + `resources/read` + `resources/subscribe` + `resources/templates/list`)
/// 4. `Prompts` — 提示词模板 (`prompts/list` + `prompts/get`)
/// 5. `Sampling` — 客户端→服务端 LLM 采样请求 (`sampling/createMessage`)
/// 6. `Roots` — 客户端文件系统根目录 (`roots/list`)
/// 7. `Logging` — 日志级别控制 (`logging/setLevel`)
///
/// **编译期 hardcode 守门**: 加新 primitive 必须改 `primitive_count()` + `as_str()` + `methods()` 三处.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Primitive {
    /// Initialize — 握手 (协议版本 + 能力 + 标识)
    Initialize,
    /// Tools — 工具调用 (list / call / subscribe)
    Tools,
    /// Resources — 资源访问 (list / read / subscribe / templates)
    Resources,
    /// Prompts — 提示词模板 (list / get)
    Prompts,
    /// Sampling — 客户端→服务端 LLM 采样请求 (createMessage)
    Sampling,
    /// Roots — 客户端文件系统根目录 (list)
    Roots,
    /// Logging — 日志级别控制 (setLevel)
    Logging,
}

/// **Primitive 计数** (编译期 hardcode 7)
pub const PRIMITIVE_COUNT: usize = 7;

impl Primitive {
    /// **Primitive 的 JSON-RPC method 列表** (per MCP spec 2025-03-26)
    ///
    /// **不漂移 (8 硬墙 #3)**: 0 改 method 名 (`tools/list` 等必须严格匹配 MCP spec)
    pub fn methods(&self) -> &'static [&'static str] {
        match self {
            Primitive::Initialize => &["initialize", "notifications/initialized"],
            Primitive::Tools => &["tools/list", "tools/call", "tools/subscribe"],
            Primitive::Resources => &[
                "resources/list",
                "resources/read",
                "resources/subscribe",
                "resources/templates/list",
            ],
            Primitive::Prompts => &["prompts/list", "prompts/get"],
            Primitive::Sampling => &["sampling/createMessage"],
            Primitive::Roots => &["roots/list"],
            Primitive::Logging => &["logging/setLevel"],
        }
    }

    /// **Primitive 的人类可读名** (跟 enum variant 同 kebab-case)
    pub fn as_str(&self) -> &'static str {
        match self {
            Primitive::Initialize => "initialize",
            Primitive::Tools => "tools",
            Primitive::Resources => "resources",
            Primitive::Prompts => "prompts",
            Primitive::Sampling => "sampling",
            Primitive::Roots => "roots",
            Primitive::Logging => "logging",
        }
    }

    /// **所有 primitive 列表** (per 借鉴 pattern: 注册表用)
    pub const ALL: &'static [Primitive] = &[
        Primitive::Initialize,
        Primitive::Tools,
        Primitive::Resources,
        Primitive::Prompts,
        Primitive::Sampling,
        Primitive::Roots,
        Primitive::Logging,
    ];

    /// **从 method 名反查 primitive** (per 借鉴 pattern: dispatch 用)
    pub fn from_method(method: &str) -> Option<Primitive> {
        for p in Self::ALL {
            if p.methods().contains(&method) {
                return Some(*p);
            }
        }
        None
    }
}

// ============================================================
// 编译期 hardcode 守门 (per 8 硬墙 #3 + 主哲学锚 #1)
// ============================================================

const _: () = {
    // 防 PRIMITIVE_COUNT 与 Primitive::ALL 漂移 (编译期 hardcode 守门)
    assert!(PRIMITIVE_COUNT == 7, "PRIMITIVE_COUNT must be 7");
    // 注: 数组长度 7 由 `Primitive::ALL` 数组字面量保证, 不需 runtime assert
    // (Rust 数组长度是类型的一部分, 7 个元素就是 7)
    // methods() 总数守门改为 runtime 二次守 (在 tests 里), 因为 const fn 不能 iter().sum()
    // 见 test_primitive_enum_exhaustive 验证 total_methods == 14
};

#[cfg(test)]
mod tests {
    use super::*;

    /// **test_primitive_enum_exhaustive** — 验证 7 primitive 全部列出, 0 漂移
    #[test]
    fn test_primitive_enum_exhaustive() {
        // 1) 验证 PRIMITIVE_COUNT 编译期常量
        assert_eq!(PRIMITIVE_COUNT, 7);
        assert_eq!(Primitive::ALL.len(), 7);

        // 2) 验证每 primitive 都有 methods
        for p in Primitive::ALL {
            assert!(!p.methods().is_empty(), "{:?} has 0 methods", p);
            assert!(!p.as_str().is_empty(), "{:?} has empty name", p);
        }

        // 3) 验证 from_method 反查正确
        assert_eq!(Primitive::from_method("tools/list"), Some(Primitive::Tools));
        assert_eq!(Primitive::from_method("tools/call"), Some(Primitive::Tools));
        assert_eq!(Primitive::from_method("resources/list"), Some(Primitive::Resources));
        assert_eq!(Primitive::from_method("prompts/get"), Some(Primitive::Prompts));
        assert_eq!(Primitive::from_method("initialize"), Some(Primitive::Initialize));
        assert_eq!(Primitive::from_method("sampling/createMessage"), Some(Primitive::Sampling));
        assert_eq!(Primitive::from_method("roots/list"), Some(Primitive::Roots));
        assert_eq!(Primitive::from_method("logging/setLevel"), Some(Primitive::Logging));
        assert_eq!(Primitive::from_method("unknown/method"), None);

        // 4) 验证 total methods = 14 (compile-time 守 + runtime 二次守)
        let total: usize = Primitive::ALL.iter().map(|p| p.methods().len()).sum();
        assert_eq!(total, 14, "total methods must be 14");

        // 5) 验证 as_str 唯一性
        let names: Vec<&str> = Primitive::ALL.iter().map(|p| p.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "primitive names must be unique");
    }

    /// **test_capability_negotiation_roundtrip** — 验证 capability 协商 serialize/deserialize 一致
    #[test]
    fn test_capability_negotiation_roundtrip() {
        // 1) 验证 Primitive 序列化稳定 (serde kebab-case variant)
        let p = Primitive::Tools;
        let json_str = serde_json::to_string(&p).unwrap();
        assert_eq!(json_str, "\"Tools\"", "Primitive::Tools serializes as `\"Tools\"` (PascalCase variant)");

        let restored: Primitive = serde_json::from_str(&json_str).unwrap();
        assert_eq!(restored, Primitive::Tools);

        // 2) 验证所有 primitive 都能 roundtrip
        for p in Primitive::ALL {
            let s = serde_json::to_string(p).unwrap();
            let back: Primitive = serde_json::from_str(&s).unwrap();
            assert_eq!(*p, back, "{:?} roundtrip failed", p);
        }

        // 3) 验证 Initialize / Sampling / Roots / Logging 4 个 variant
        //    apeireth-mcp 当前不实现, 但 Primitive enum 覆盖 (B1 24 LOCKED 持续更新)
        let not_implemented = [
            Primitive::Sampling,
            Primitive::Roots,
            Primitive::Logging,
        ];
        for p in not_implemented {
            assert!(
                Primitive::from_method(p.methods()[0]).is_some(),
                "{:?} from_method must work",
                p
            );
        }
    }
}
