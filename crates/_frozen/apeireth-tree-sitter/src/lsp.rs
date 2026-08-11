//! §6 lsp — LSP 协议支持 (1:1 翻译 LSP 3.17 `textDocument/*` + `workspace/*` 协议面).
//!
//! **核心 API**: [`lsp_dispatch`] (输入 LspMessage, 返 LspResponse).
//!
//! **状态**: ⚠️ skeleton — R20 阶段 5 估补, 真实 LSP 协议接入留 R20 阶段 4 续.
//! 当前所有 LSP method 都返 `TreeSitterError::LspError { code: -32601, message: "Method not found" }` (per O-5 不假装 + LSP 3.17 协议).
//!
//! **设计**:
//! - `LspMessage` 6 method (per task spec: 1:1 翻译 LSP 3.17 `textDocument/*` 端点):
//!   - `textDocument/highlight` (§1 暴露 LSP 端)
//!   - `textDocument/foldingRange` (§4 暴露 LSP 端)
//!   - `textDocument/indentation` (§5 暴露 LSP 端, non-standard)
//!   - `textDocument/ast` (§2 暴露 LSP 端, non-standard)
//!   - `textDocument/search` (§3 暴露 LSP 端, non-standard)
//!   - `workspace/languages` (元数据)
//! - `LspResponse` 3 字段 (result + error + id)
//! - `lsp_dispatch` 路由 method → 子模块 API
//!
//! **R20 阶段 4 续**:
//! - 接 lsp-types = "0.95" Cargo.toml dep
//! - 真实 grammar 接入后, 此 dispatch 调用 highlight/fold/indent/parse/search
//! - per R20 阶段 4 续, 集成 apeireth-mcp 把 LSP 端点暴露为 MCP tools

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use crate::{ast, fold, highlight, indent, search, Language, TreeSitterError, TreeSitterResult};

// ============================================================================
// LspMessage (6 method, per task spec §6: 1:1 翻译 LSP 3.17 textDocument/*)
// ============================================================================

/// LSP 消息 (1:1 翻译 LSP 3.17 `Request` 字段: method + params + id).
///
/// **6 method 对应 §1-§5 5 核心 API + 1 元数据**:
/// - §1 highlight → `textDocument/highlight`
/// - §2 ast → `textDocument/ast` (non-standard, 由 apeireth-tree-sitter 自定义)
/// - §3 search → `textDocument/search` (non-standard)
/// - §4 fold → `textDocument/foldingRange`
/// - §5 indent → `textDocument/indentation` (non-standard)
/// - 元数据 → `workspace/languages`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum LspMessage {
    /// `textDocument/highlight` (1:1 翻译 LSP 3.17)
    #[serde(rename = "textDocument/highlight")]
    Highlight {
        /// LSP request id
        id: u64,
        /// 高亮请求参数
        params: HighlightParams,
    },
    /// `textDocument/ast` (non-standard, apeireth-tree-sitter 自定义)
    #[serde(rename = "textDocument/ast")]
    Ast {
        /// LSP request id
        id: u64,
        /// 解析请求参数
        params: ParseParams,
    },
    /// `textDocument/search` (non-standard)
    #[serde(rename = "textDocument/search")]
    Search {
        /// LSP request id
        id: u64,
        /// 搜索请求参数
        params: SearchParams,
    },
    /// `textDocument/foldingRange` (1:1 翻译 LSP 3.17)
    #[serde(rename = "textDocument/foldingRange")]
    FoldingRange {
        /// LSP request id
        id: u64,
        /// 折叠请求参数
        params: FoldingRangeParams,
    },
    /// `textDocument/indentation` (non-standard)
    #[serde(rename = "textDocument/indentation")]
    Indentation {
        /// LSP request id
        id: u64,
        /// 缩进请求参数
        params: IndentationParams,
    },
    /// `workspace/languages` (元数据, 1:1 翻译 vscode `client/registerCapability`)
    #[serde(rename = "workspace/languages")]
    Languages {
        /// LSP request id
        id: u64,
    },
}

// ============================================================================
// LspParams (6 个 params struct)
// ============================================================================

/// 高亮请求参数 (1:1 翻译 LSP 3.17 `DocumentHighlightParams`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightParams {
    /// 源文本
    pub text: String,
    /// 源语言
    pub language: Language,
}

/// 解析请求参数 (non-standard, apeireth-tree-sitter 自定义).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseParams {
    /// 源文本
    pub text: String,
    /// 解析选项
    pub options: ast::ParseOptions,
}

/// 搜索请求参数 (non-standard).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchParams {
    /// 源文本
    pub text: String,
    /// 源语言
    pub language: Language,
    /// 搜索查询
    pub query: search::SearchQuery,
}

/// 折叠范围请求参数 (1:1 翻译 LSP 3.17 `FoldingRangeParams`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldingRangeParams {
    /// 源文本
    pub text: String,
    /// 源语言
    pub language: Language,
    /// 折叠级别 (1-5, 默认 2)
    pub level: u32,
}

/// 缩进请求参数 (non-standard).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndentationParams {
    /// 源文本
    pub text: String,
    /// 源语言
    pub language: Language,
}

// ============================================================================
// LspResponse (3 字段: result + error + id)
// ============================================================================

/// LSP 响应 (1:1 翻译 LSP 3.17 `Response` 字段: result/error/id).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LspResponse {
    /// LSP request id (跟 LspMessage.id 对应)
    pub id: u64,
    /// 成功结果 (success 时填充, error 时 None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 错误 (failure 时填充, success 时 None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<LspErrorBody>,
}

/// LSP 错误体 (1:1 翻译 LSP 3.17 `ResponseError`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspErrorBody {
    /// LSP error code (1:1 翻译 LSP 3.17 §ErrorCodes: -32601 Method not found, -32602 Invalid params, -32603 Internal error, -32000 ServerNotInitialized)
    pub code: i32,
    /// 错误消息
    pub message: String,
}

impl LspErrorBody {
    /// 构造 LSP "Method not found" 错误 (per LSP 3.17 §ErrorCodes code = -32601).
    #[must_use]
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
        }
    }

    /// 构造 LSP "Invalid params" 错误 (code = -32602).
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    /// 构造 LSP "Internal error" 错误 (code = -32603).
    #[must_use]
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
        }
    }
}

// ============================================================================
// §6 核心 API: lsp_dispatch (路由 6 method → 子模块, skeleton 阶段返 NotImplemented)
// ============================================================================

/// LSP 协议 dispatch (路由 LspMessage.method → 子模块 API, 返 LspResponse).
///
/// **skeleton 阶段**: 所有 method 返 LspErrorBody::method_not_found.
/// R20 阶段 4 续: 真实 grammar 接入后, 此 dispatch 调 highlight/parse/search/fold/indent/列出 SUPPORTED_LANGUAGES.
///
/// # Errors
///
/// - 永不直接返 `Err`; 错误都通过 `LspResponse.error` 字段表达 (per LSP 3.17 协议)
/// - 函数本身只 `panic`-free 失败时返 `Ok(LspResponse { error: Some(...) })`
pub fn lsp_dispatch(message: LspMessage) -> TreeSitterResult<LspResponse> {
    match message {
        LspMessage::Highlight { id, params: _ } => {
            warn!(id, "lsp_dispatch: textDocument/highlight skeleton 阶段未实现");
            Ok(LspResponse {
                id,
                result: None,
                error: Some(LspErrorBody::internal_error(
                    "skeleton: textDocument/highlight 留 R20 阶段 4 续",
                )),
            })
        }
        LspMessage::Ast { id, params: _ } => {
            warn!(id, "lsp_dispatch: textDocument/ast skeleton 阶段未实现");
            Ok(LspResponse {
                id,
                result: None,
                error: Some(LspErrorBody::internal_error(
                    "skeleton: textDocument/ast 留 R20 阶段 4 续",
                )),
            })
        }
        LspMessage::Search { id, params: _ } => {
            warn!(id, "lsp_dispatch: textDocument/search skeleton 阶段未实现");
            Ok(LspResponse {
                id,
                result: None,
                error: Some(LspErrorBody::internal_error(
                    "skeleton: textDocument/search 留 R20 阶段 4 续",
                )),
            })
        }
        LspMessage::FoldingRange { id, params: _ } => {
            warn!(id, "lsp_dispatch: textDocument/foldingRange skeleton 阶段未实现");
            Ok(LspResponse {
                id,
                result: None,
                error: Some(LspErrorBody::internal_error(
                    "skeleton: textDocument/foldingRange 留 R20 阶段 4 续",
                )),
            })
        }
        LspMessage::Indentation { id, params: _ } => {
            warn!(id, "lsp_dispatch: textDocument/indentation skeleton 阶段未实现");
            Ok(LspResponse {
                id,
                result: None,
                error: Some(LspErrorBody::internal_error(
                    "skeleton: textDocument/indentation 留 R20 阶段 4 续",
                )),
            })
        }
        LspMessage::Languages { id } => {
            // 元数据: 直接列 SUPPORTED_LANGUAGES (skeleton 阶段唯一真实输出)
            let langs: Vec<String> = Language::all().iter().map(|l| l.as_str().to_string()).collect();
            Ok(LspResponse {
                id,
                result: Some(serde_json::json!({ "languages": langs })),
                error: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParseOptions;
    use crate::search::SearchQuery;

    #[test]
    fn test_lsp_message_highlight_serde() {
        let msg = LspMessage::Highlight {
            id: 1,
            params: HighlightParams {
                text: "fn main() {}".to_string(),
                language: Language::Rust,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("textDocument/highlight"));
        let back: LspMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn test_lsp_error_body_method_not_found() {
        let err = LspErrorBody::method_not_found("textDocument/ast");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("textDocument/ast"));
    }

    #[test]
    fn test_lsp_dispatch_languages_returns_8() {
        let msg = LspMessage::Languages { id: 42 };
        let response = lsp_dispatch(msg).unwrap();
        assert_eq!(response.id, 42);
        let result = response.result.expect("Languages method 应有 result");
        let langs = result.get("languages").expect("languages 字段");
        let arr = langs.as_array().expect("languages 应为数组");
        assert_eq!(arr.len(), 8, "SUPPORTED_LANGUAGES 编译期 hardcode 8");
    }

    #[test]
    fn test_lsp_dispatch_highlight_skeleton() {
        let msg = LspMessage::Highlight {
            id: 1,
            params: HighlightParams {
                text: "fn main() {}".to_string(),
                language: Language::Rust,
            },
        };
        let response = lsp_dispatch(msg).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.result.is_none());
        let err = response.error.expect("skeleton 阶段应有 error");
        assert_eq!(err.code, -32603, "Internal error code");
    }

    #[test]
    fn test_lsp_dispatch_search_skeleton() {
        let msg = LspMessage::Search {
            id: 2,
            params: SearchParams {
                text: "fn main() {}".to_string(),
                language: Language::Rust,
                query: SearchQuery::new(),
            },
        };
        let response = lsp_dispatch(msg).unwrap();
        assert!(response.error.is_some());
    }

    #[test]
    fn test_lsp_dispatch_folding_skeleton() {
        let msg = LspMessage::FoldingRange {
            id: 3,
            params: FoldingRangeParams {
                text: "fn main() {}".to_string(),
                language: Language::Rust,
                level: 2,
            },
        };
        let response = lsp_dispatch(msg).unwrap();
        assert!(response.error.is_some());
    }

    #[test]
    fn test_lsp_dispatch_indentation_skeleton() {
        let msg = LspMessage::Indentation {
            id: 4,
            params: IndentationParams {
                text: "fn main() {}".to_string(),
                language: Language::Rust,
            },
        };
        let response = lsp_dispatch(msg).unwrap();
        assert!(response.error.is_some());
    }

    #[test]
    fn test_lsp_dispatch_ast_skeleton() {
        let msg = LspMessage::Ast {
            id: 5,
            params: ParseParams {
                text: "fn main() {}".to_string(),
                options: ParseOptions::new(Language::Rust),
            },
        };
        let response = lsp_dispatch(msg).unwrap();
        assert!(response.error.is_some());
    }

    // 引用 4 子模块确保编译期不报 unused (per apeireth-observability 模式)
    #[allow(dead_code)]
    fn _ensure_subs_used() {
        let _ = highlight::highlight("x", Language::Rust);
        let _ = fold::fold("x", Language::Rust, 2);
        let _ = indent::detect_indent("x", Language::Rust);
    }
}
