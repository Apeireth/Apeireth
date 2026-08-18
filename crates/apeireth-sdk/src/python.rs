//! `apeireth-sdk::python` — PyO3 桥接 (R122-8 Multi-Lang SDK skeleton, cfg feature = "python")
//!
//! **状态**: skeleton, 仅 demo 1 fn `py_count_tokens`, 0 假装 100% 多语言支持 (O-5).
//!
//! **O-5 实质守门**: 仅 `--features python` 启用时编译, 默认 build 0 装 pyo3.
//! **R122-8 决策**: cfg-gated features 隔离 (per lib.rs §A R122-8 段 + Cargo.toml `[features]`).
//! **R122-3 协作**: R122-3 tiktoken_counter retry 跑中, R122-8 inline 简版 count_tokens
//!   (R32-1 `apeireth-asi::tokenizer::count_tokens` 1:1 port, 0 dep 24 LOCKED apeireth-asi).
//!   R122-3 retry 完成后 R123 切换到正式 fn.
//!
//! **PyO3 version**: workspace 0.29 (per workspace Cargo.toml [workspace.dependencies]),
//! 不是任务 0.22 (workspace 不能改, 加 0.22 会跟 workspace 0.29 双版本 lock bloat).
//!
//! **公共 API** (PyO3 暴露给 Python):
//! - `apeireth_sdk_py.count_tokens(text: str, model: str) -> int` — R32-1 启发式
//! - 模块名: `apeireth_sdk_py`
//!
//! **用法** (Python 端, 假设 .so 已编译):
//! ```python
//! from apeireth_sdk import apeireth_sdk_py
//! n = apeireth_sdk_py.count_tokens("Hello, 世界!", "cl100k_base")
//! print(n)  # 启发式 token 数
//! ```

#![cfg(feature = "python")]
// PyO3 macro 内部用 unsafe 桥接 CPython C-API, 0 改 apeireth-sdk 顶层 #![deny(unsafe_code)]
// (per abi.rs 已有模式: extern "C" 桥接局部 #![allow(unsafe_code)])
#![allow(unsafe_code)]

use pyo3::prelude::*;

// ============================================================================
// R32-1 1:1 port: count_tokens 启发式 (CJK + ASCII word + symbol/3 ceil)
// ============================================================================

/// **R32-1 启发式** (1:1 翻译 `apeireth-asi::tokenizer::count_tokens`):
/// - ASCII word (字母数字 + 下划线) = 1 token
/// - CJK char = 1 token
/// - 其他 char (空格/标点/emoji) = ceil(1/3) = 1 token
///
/// **不假装**: 0 调 tiktoken-rs, 0 假装 LLM 真实 tokenizer. 仅 demo 跨语言一致性.
/// R122-3 retry 完成后 R123 切换到 `apeireth-pipeline::tiktoken_counter::count_tokens_precise`.
fn count_tokens_heuristic(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut tokens: u32 = 0;
    let mut ascii_word_chars: u32 = 0;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            ascii_word_chars += 1;
        } else {
            if ascii_word_chars > 0 {
                tokens += 1;
                ascii_word_chars = 0;
            }
            if is_cjk(c) {
                tokens += 1;
            } else {
                tokens += 1; // ceil(1/3) = 1
            }
        }
    }
    if ascii_word_chars > 0 {
        tokens += 1;
    }
    tokens
}

/// 6 unicode block (CJK 跟 CJK Extension 一致, 1:1 apeireth-asi::tokenizer::is_cjk)
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}'
    )
}

// ============================================================================
// PyO3 桥接 (1 pymodule + 1 fn, R122-8 skeleton 1:1)
// ============================================================================

/// **PyO3 module 入口**: 编译后 Python 用 `import apeireth_sdk_py` 加载.
///
/// **签名**: `#[pyo3::pymodule] pub fn apeireth_sdk_py(_py: Python, m: &Bound<PyModule>) -> PyResult<()>`
/// (per PyO3 0.29 API, `&PyModule` 在 0.22+ 已 deprecated, 0.29 用 `&Bound<PyModule>`)
#[pymodule]
pub fn apeireth_sdk_py(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_count_tokens, m)?)?;
    Ok(())
}

/// **PyO3 fn 暴露给 Python** (1 fn, R122-8 skeleton):
/// - `text: str` — 输入文本
/// - `model: str` — 模型名 (e.g. "cl100k_base", "gpt-4o", 0 实际使用, 仅签名占位)
/// - 返 token 数 (u32 → Python int)
///
/// **不假装**: 0 用 model 参数, 0 调 tiktoken. 启发式算法 1:1 R32-1.
///
/// **pub**: 让 multilang_ffi 集成测试能 import 验证
#[pyfunction]
pub fn py_count_tokens(text: &str, model: &str) -> PyResult<u32> {
    // model 参数 0 使用 (签名占位, 跨语言 1:1 一致性优先)
    let _ = model;
    Ok(count_tokens_heuristic(text))
}

// ============================================================================
// 3 unit tests (per task spec, 0 Python 运行时测试, 仅 Rust 单元)
// ============================================================================

#[cfg(test)]
mod python_ffi_tests {
    use super::*;

    /// **Test #1**: ASCII 单词 token count.
    #[test]
    fn py_count_tokens_ascii_only() {
        // "hello" = 1 word = 1 token
        assert_eq!(count_tokens_heuristic("hello"), 1);
        // "hello world" = 2 words + 1 space (1/3 ceil=1) = 3
        assert_eq!(count_tokens_heuristic("hello world"), 3);
        // 空串 = 0
        assert_eq!(count_tokens_heuristic(""), 0);
    }

    /// **Test #2**: CJK 字符 token count.
    #[test]
    fn py_count_tokens_cjk_only() {
        // 2 CJK = 2 tokens
        assert_eq!(count_tokens_heuristic("你好"), 2);
        // 100 CJK = 100 tokens
        let s: String = "中".repeat(100);
        assert_eq!(count_tokens_heuristic(&s), 100);
    }

    /// **Test #3**: ASCII + CJK 混合.
    #[test]
    fn py_count_tokens_mixed_ascii_cjk() {
        // "hello 世界" = 1 word + 1 space + 2 CJK = 4
        assert_eq!(count_tokens_heuristic("hello 世界"), 4);
        // "Hello, world!" = 1 word + 1 punct (1/3 ceil=1) + 1 space + 1 word + 1 punct = 5
        // (实际: H-e-l-l-o word 1 + , punct 1 + space 1 + w-o-r-l-d word 1 + ! punct 1 = 5)
        assert_eq!(count_tokens_heuristic("Hello, world!"), 5);
    }
}
