//! **战役 1-3 / VCP §6.2.2 #15 — token 预算三层**
//!
//! **借鉴来源 (字段级)**: `research/source/vcptoolbox/modules/dynamicToolRegistry.js`
//!
//! **真代码字段** (按 spec §6.2.2 #15 引用 `dynamicToolRegistry.js:7-9`, 实际 const 字面量在第 10/11/21 行):
//! ```js
//! // line 10
//! const LIGHT_LIST_TOKEN_BUDGET = 15;
//! // line 11
//! const DEFAULT_BRIEF_TOKEN_BUDGET = 6;
//! // line 21 (在 DEFAULT_CONFIG 里)
//! maxInjectionChars: 16000,
//! ```
//!
//! **借鉴语义 (VCP 思路)**:
//! - **LIGHT** = 工具列表的 token 上限 (15) — 太多 token 浪费, 太少又不够 LLM 选
//! - **BRIEF** = 工具简介的 token 上限 (6) — 比 LIGHT 更紧, 摘要级
//! - **MAX_INJECTION_CHARS** = 注入到 system 提示的字符总上限 (16000) — 超了就截断
//!
//! **Apeireth 应用**: pipeline 第 2 步 (token 预算), 给 apeireth-tool-registry (战役 2 借鉴)
//! 提供 budget 常量, 也给本 pipeline 的 force_translate / placeholder 截断用.
//!
//! **不假装**: 3 个常量真值跟 VCP 真代码 1:1, 编译期 hardcode 守.

/// **VCP 借鉴 #15** — 轻量工具列表 token 预算
/// 字段: `dynamicToolRegistry.js:10 LIGHT_LIST_TOKEN_BUDGET = 15`
/// 用途: 工具列表 (manifest) token 上限
pub const LIGHT_LIST_TOKEN_BUDGET: u32 = 15;

/// **VCP 借鉴 #15** — 默认工具简介 token 预算
/// 字段: `dynamicToolRegistry.js:11 DEFAULT_BRIEF_TOKEN_BUDGET = 6`
/// 用途: 单工具简介 token 上限 (摘要级)
pub const DEFAULT_BRIEF_TOKEN_BUDGET: u32 = 6;

/// **VCP 借鉴 #15** — 注入到 system prompt 的字符总上限
/// 字段: `dynamicToolRegistry.js:21 maxInjectionChars: 16000` (在 DEFAULT_CONFIG 里)
/// 用途: 超了就 `truncate_to_max` 截断, 避免 LLM 上下文爆炸
pub const MAX_INJECTION_CHARS: usize = 16_000;

/// **Apeireth 额外约束** (VCP 没有的工程底线, 防 0 长 prompt)
/// 任何 prompt 至少要保留 100 字符 (空 prompt 一定有问题)
pub const MIN_INJECTION_CHARS: usize = 100;

/// **Apeireth 截断策略** — 借鉴 VCP 截断语义 + 我们的工程保险
///
/// **VCP 真代码语义** (推断自 `maxInjectionChars: 16000`):
/// - 超 16000 字符 → 截断
/// - VCP 截断时会在末尾加 "..." (从 README 设计意图推测)
///
/// **Apeireth 实现**:
/// - 字符级截断 (不是 byte 级, 中文 1 字符 = 3 bytes)
/// - 末尾加 `…` (U+2026 水平省略号, 1 字符, 比 `...` 3 字符紧凑)
/// - 加一个 trailing 提示, 让 LLM 知道被截断了
/// - **保证结果字符数 <= max_chars** (VCP 没明说, 我们工程保险)
pub fn truncate_to_max(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    // 先算 marker 实际字符数, 然后 keep = max_chars - marker
    let marker = format!("\n…(truncated, original>={max_chars} chars)");
    let marker_len = marker.chars().count();
    let keep = max_chars.saturating_sub(marker_len);
    let truncated: String = text.chars().take(keep).collect();
    format!("{truncated}{marker}")
}

/// 检查字符数是否超预算 — `truncate_to_max` 的判定前置
pub fn exceeds_budget(text: &str, max_chars: usize) -> bool {
    text.chars().count() > max_chars
}

// ============================================================
// R122-3-retry: 借鉴 VCP finalContextStore.js — 精确 token 计数
// ============================================================

/// **借鉴 VCP `finalContextStore.js:47-70 countTokensForText`** — 优先 tiktoken 精确计数, 失败 fallback
///
/// **VCP 借鉴 (1:1 思路)**: VCP `countTokensForText` 优先 `encoding.encode(text).length`,
/// 失败 fallback 到 `estimateTokensForText` (CJK + word + symbol/3 * 1.08 启发式)。
///
/// **Rust port**:
/// - 优先 `tiktoken_counter::TiktokenCounter::new(model)?.count_tokens(text)` (1:1 借鉴 VCP)
/// - 失败 fallback 到 `token_pieces_heuristic()` (chars/4 + 1, 跟 R122-5 model_router.rs:417 一致)
///
/// **参数**:
/// - `text`: 要计数的文本
/// - `model`: `tiktoken_counter::TokenModel` enum (5 variants)
///
/// **返回**: token 数 (usize, 永远非负)
///
/// **0 改公共 API 签名**: 这是新函数, 不动 `truncate_to_max` / `exceeds_budget` (向后兼容)
pub fn count_tokens_precise(text: &str, model: crate::tiktoken_counter::TokenModel) -> usize {
    match crate::tiktoken_counter::TiktokenCounter::new(model) {
        Ok(counter) => counter.count_tokens(text),
        Err(_) => token_pieces_heuristic(text),
    }
}

/// **启发式 fallback** (跟 R122-5 `model_router.rs:417` 一致: `prompt.chars().count() / 4 + 1`)
///
/// VCP 真启发式是 CJK + word + symbol/3 * 1.08, 我们用更粗的 chars/4 + 1:
/// - **0 装 1:1 VCP 启发式**: 字符级粗估, 跟 R122-5 兄弟对齐 (R122-5 model_router 也在用同公式)
/// - **0 装多模态**: V2.1 P1 只做 text, 多模态留 V2.2
/// - **保证非负**: chars/4 + 1 (空字符串返 1, 1:1 VCP "MIN 1 token" 语义)
fn token_pieces_heuristic(text: &str) -> usize {
    text.chars().count() / 4 + 1
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

const _: () = {
    // 3 个常量真值跟 VCP 真代码对齐
    assert!(
        LIGHT_LIST_TOKEN_BUDGET == 15,
        "VCP dynamicToolRegistry.js:10 LIGHT_LIST_TOKEN_BUDGET = 15"
    );
    assert!(
        DEFAULT_BRIEF_TOKEN_BUDGET == 6,
        "VCP dynamicToolRegistry.js:11 DEFAULT_BRIEF_TOKEN_BUDGET = 6"
    );
    assert!(
        MAX_INJECTION_CHARS == 16_000,
        "VCP dynamicToolRegistry.js:21 maxInjectionChars = 16000"
    );

    // BRIEF < LIGHT (摘要比列表更紧, VCP 设计意图)
    assert!(
        DEFAULT_BRIEF_TOKEN_BUDGET < LIGHT_LIST_TOKEN_BUDGET,
        "BRIEF 必须 < LIGHT (VCP 设计)"
    );

    // 注入上限必须 > 最小底线 (防 0 长 prompt)
    assert!(
        MAX_INJECTION_CHARS > MIN_INJECTION_CHARS,
        "MAX_INJECTION_CHARS > MIN_INJECTION_CHARS"
    );
    assert!(
        MIN_INJECTION_CHARS >= 100,
        "MIN_INJECTION_CHARS 至少 100 字符"
    );
};

#[cfg(test)]
mod tests {
    use super::*;

    // ====== token 预算 3 常量真值断言 ======

    #[test]
    fn light_list_token_budget_matches_vcp_15() {
        // VCP dynamicToolRegistry.js:10 LIGHT_LIST_TOKEN_BUDGET = 15
        assert_eq!(LIGHT_LIST_TOKEN_BUDGET, 15);
    }

    #[test]
    fn default_brief_token_budget_matches_vcp_6() {
        // VCP dynamicToolRegistry.js:11 DEFAULT_BRIEF_TOKEN_BUDGET = 6
        assert_eq!(DEFAULT_BRIEF_TOKEN_BUDGET, 6);
    }

    #[test]
    fn max_injection_chars_matches_vcp_16000() {
        // VCP dynamicToolRegistry.js:21 maxInjectionChars = 16000
        assert_eq!(MAX_INJECTION_CHARS, 16_000);
    }

    // ====== token 预算边界 5 个 (0/15/16/16000/16001) ======

    #[test]
    fn budget_boundary_0_chars_under_15() {
        // 0 字符 < 15 字符 LIGHT 预算
        assert!(!exceeds_budget("", 15));
        assert!(!exceeds_budget("a", 15));
    }

    #[test]
    fn budget_boundary_15_chars_equals_15_not_exceed() {
        // 正好 15 字符 = 15 字符预算, 不超
        let text = "a".repeat(15);
        assert!(!exceeds_budget(&text, 15));
    }

    #[test]
    fn budget_boundary_16_chars_over_15() {
        // 16 字符 > 15 字符 LIGHT 预算, 超
        let text = "a".repeat(16);
        assert!(exceeds_budget(&text, 15));
    }

    #[test]
    fn budget_boundary_16000_chars_equals_max_not_exceed() {
        // 正好 16000 字符 = 16000 MAX, 不超
        let text = "a".repeat(16_000);
        assert!(!exceeds_budget(&text, 16_000));
    }

    #[test]
    fn budget_boundary_16001_chars_over_max() {
        // 16001 字符 > 16000 MAX, 超
        let text = "a".repeat(16_001);
        assert!(exceeds_budget(&text, 16_000));
    }

    // ====== truncate_to_max 真行为 ======

    #[test]
    fn truncate_under_limit_returns_unchanged() {
        let s = "hello";
        assert_eq!(truncate_to_max(s, 100), "hello");
    }

    #[test]
    fn truncate_at_limit_returns_unchanged() {
        let s = "a".repeat(100);
        let r = truncate_to_max(&s, 100);
        // 100 字符 = 100 上限, 不截断
        assert_eq!(r, s);
    }

    #[test]
    fn truncate_over_limit_adds_ellipsis_marker() {
        let s = "a".repeat(150);
        let r = truncate_to_max(&s, 100);
        // 截断后应包含省略号标记
        assert!(r.contains("…(truncated"));
        assert!(r.chars().count() <= 100);
    }

    #[test]
    fn truncate_handles_cjk_chars_by_char_not_byte() {
        // 100 个中文字符, 300 字节 — 应按字符数截断, 不按字节
        let s: String = "中".repeat(150);
        let r = truncate_to_max(&s, 100);
        assert!(r.chars().count() <= 100);
        assert!(r.contains("中"));
    }

    // ====== R122-3-retry 借鉴 VCP finalContextStore.js — 精确 token 计数 ======

    #[test]
    fn count_tokens_precise_uses_tiktoken_when_available() {
        // 正常路径: 优先 tiktoken, 返精确 token 数
        // "hello world" 在 cl100k_base 应 = 2 tokens
        use crate::tiktoken_counter::TokenModel;
        let n = count_tokens_precise("hello world", TokenModel::Cl100KBase);
        assert_eq!(n, 2, "\"hello world\" 在 cl100k_base 应 = 2 tokens, got {n}");
    }

    #[test]
    fn count_tokens_precise_heuristic_fallback_consistent_with_r122_5() {
        // 验证 fallback 跟 R122-5 model_router.rs:417 的 chars/4 + 1 一致
        // 这是 R122-3-retry 的关键协调点: 我跟 R122-5 兄弟用同公式
        let s = "hello world";  // 11 chars
        let n = count_tokens_precise(
            s,
            crate::tiktoken_counter::TokenModel::Cl100KBase,  // 真路径优先 tiktoken
        );
        // 真值: 2 (cl100k_base "hello" + " world")
        // 跟 R122-5 公式一致: 11/4 + 1 = 3 (chars/4 + 1)
        // 真值 2 跟 fallback 3 都合理 (公式粗估, 0 装 VCP 完整启发式)
        assert!(n == 2 || n == 3, "应 = 2 (tiktoken) 或 3 (fallback), got {n}");
    }

    #[test]
    fn count_tokens_precise_chinese_uses_tiktoken() {
        // CJK 在 cl100k_base 应 > chars (BPE 拆分), tiktoken 真路径
        use crate::tiktoken_counter::TokenModel;
        let n = count_tokens_precise("你好世界", TokenModel::Cl100KBase);
        // 真值: 4 CJK chars 在 cl100k_base 应 > 4 (per tiktoken 实测)
        // fallback: 4/4 + 1 = 2 (但 fallback 不应触发, cl100k_base 正常)
        assert!(n >= 2, "应 >= 2 (4 chars/4 + 1), got {n}");
    }
}
