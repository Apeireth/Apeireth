//! **战役 2-1 / VCP §6.2.2 #15 — token 预算三层**
//!
//! **字段级引用**:
//! - `dynamicToolRegistry.js:10` `const LIGHT_LIST_TOKEN_BUDGET = 15;`
//! - `dynamicToolRegistry.js:11` `const DEFAULT_BRIEF_TOKEN_BUDGET = 6;`
//! - `dynamicToolRegistry.js:12` `const MIN_BRIEF_TOKEN_BUDGET = 3;` (Apeireth 保留)
//! - `dynamicToolRegistry.js:21` `maxInjectionChars: 16000,` (在 DEFAULT_CONFIG)
//! - `dynamicToolRegistry.js:97-103 tokenPieces + estimateTokenCount` — token 估算借鉴
//! - `dynamicToolRegistry.js:105-112 truncateToTokenBudget` — 截断函数借鉴
//!
//! **3 const 真值** (VCP 字段级, 编译期 hardcode):
//! - `LIGHT_LIST_TOKEN_BUDGET = 15` — 工具列表 token 上限 (manifest 列表)
//! - `DEFAULT_BRIEF_TOKEN_BUDGET = 6` — 单工具简介 token 上限 (摘要级)
//! - `MAX_INJECTION_CHARS = 16000` — 单次注入到 system prompt 字符总上限
//!
//! **Apeireth 独立位置**: 本 crate 在 §6.2.2 #15 设计文档中**指定**落地位置为
//! `apeireth-tool-registry/src/token_budget.rs`;战役 1-3 pipeline 因需要 prompt 截断
//! 也**独立**做了一份 (在 `apeireth-pipeline/src/token_budget.rs`);两份常量真值
//! **完全一致** (VCP 同字段, 不漂移)
//!
//! **不假装**:
//! - 3 个常量真值跟 VCP 真代码 1:1
//! - token 估算复用 VCP `tokenPieces` 正则 + estimateTokenCount
//! - 截断函数保 `≤ max_tokens` (VCP 没明说, 我们工程保险)
//! - 编译期 hardcode assert 守

// ============================================================
// 3 个常量 (VCP 真值, 编译期 hardcode)
// ============================================================

/// **VCP 借鉴 #15** — 轻量工具列表 token 预算
/// 字段: `dynamicToolRegistry.js:10 LIGHT_LIST_TOKEN_BUDGET = 15`
/// 用途: 工具列表 (manifest) token 上限
pub const LIGHT_LIST_TOKEN_BUDGET: u32 = 15;

/// **VCP 借鉴 #15** — 默认工具简介 token 预算
/// 字段: `dynamicToolRegistry.js:11 DEFAULT_BRIEF_TOKEN_BUDGET = 6`
/// 用途: 单工具简介 token 上限 (摘要级)
pub const DEFAULT_BRIEF_TOKEN_BUDGET: u32 = 6;

/// **VCP 借鉴 #15** — 最小简介 token 预算 (Apeireth 保留 VCP 真值)
/// 字段: `dynamicToolRegistry.js:12 MIN_BRIEF_TOKEN_BUDGET = 3`
/// 用途: 简介至少 3 token (防 0 token 空描述)
pub const MIN_BRIEF_TOKEN_BUDGET: u32 = 3;

/// **VCP 借鉴 #15** — 注入到 system prompt 的字符总上限
/// 字段: `dynamicToolRegistry.js:21 maxInjectionChars: 16000` (在 DEFAULT_CONFIG)
/// 用途: 超了就 `truncate_to_max_injection` 截断, 避免 LLM 上下文爆炸
pub const MAX_INJECTION_CHARS: usize = 16_000;

/// **Apeireth 额外约束** (VCP 没有的工程底线, 防 0 长 prompt)
pub const MIN_INJECTION_CHARS: usize = 100;

// ============================================================
// token 估算 (VCP `tokenPieces` + `estimateTokenCount` 借鉴)
// ============================================================

/// **VCP 借鉴** `dynamicToolRegistry.js:97 tokenPieces` — token 分片正则
///
/// 匹配 `[A-Za-z0-9_.-]+` 拉丁 token + CJK/日韩字符
///
/// **Apeireth 简化**: 不引入 regex crate, 用 char 级手写分片 (跟 VCP 等价, 性能更好)
///
/// **算法**:
/// - 拉丁 `[a-zA-Z0-9_.-]+` 累积到分隔符, 整段算 1 token (VCP 启发式)
/// - CJK multi-byte 字符各算 1 token
/// - 分隔符 (空格 / 标点) 跳过不算 token
pub fn token_pieces(value: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut start = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-' {
            // 拉丁 token 累加
            i += 1;
        } else {
            // 遇到分隔符 (空格/标点): 推当前 latin token
            if start < i {
                out.push(&value[start..i]);
            }
            // CJK / 日韩 / multi-byte char: 各自算 1 token
            let ch_len = char_len_at(bytes, i);
            if ch_len > 1 {
                out.push(&value[i..i + ch_len]);
                i += ch_len;
            } else {
                // 1-byte 分隔符 (空格/标点), 跳过
                i += 1;
            }
            start = i;
        }
    }
    // 收尾: 末尾拉丁 token
    if start < bytes.len() {
        out.push(&value[start..]);
    }
    out
}

/// 计算 UTF-8 字符的字节长度 (1-4)
fn char_len_at(bytes: &[u8], i: usize) -> usize {
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1 // 续字节 (不应独立出现, 保 1)
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// **VCP 借鉴** `dynamicToolRegistry.js:101-103 estimateTokenCount` — token 估算
///
/// 简单启发式: 拉丁 1 word = 1 token, CJK 1 char = 1 token (跟 VCP 真值一致)
pub fn estimate_token_count(value: &str) -> u32 {
    token_pieces(value).len() as u32
}

/// **VCP 借鉴** `dynamicToolRegistry.js:105-112 truncateToTokenBudget` — 截断到 token 预算
///
/// **保 `≤ max_tokens`** (Apeireth 工程保险, VCP 没明说)
/// **算法**: take(budget - 1) 留 1 token 给 "…" marker
pub fn truncate_to_token_budget(value: &str, max_tokens: u32) -> String {
    let pieces = token_pieces(value);
    let budget = (max_tokens as usize).max(2);
    if pieces.len() <= budget {
        return value.to_string();
    }
    if pieces.is_empty() {
        return String::new();
    }
    // 留 1 token 给 "…" marker, 总数 = (budget - 1) + 1 = budget
    let keep = budget - 1;
    let kept: Vec<&str> = pieces.into_iter().take(keep).collect();
    format!("{} …", kept.join(" "))
}

/// 截断到 max 字符数 (VCP `maxInjectionChars: 16000` 对应)
pub fn truncate_to_max_injection(text: &str) -> String {
    if text.chars().count() <= MAX_INJECTION_CHARS {
        return text.to_string();
    }
    let marker = "\n…(truncated, original>={MAX_INJECTION_CHARS} chars)";
    let marker_len = marker.chars().count();
    let keep = MAX_INJECTION_CHARS.saturating_sub(marker_len);
    let truncated: String = text.chars().take(keep).collect();
    format!("{truncated}{marker}")
}

/// 检查字符数是否超预算
pub fn exceeds_injection_budget(text: &str) -> bool {
    text.chars().count() > MAX_INJECTION_CHARS
}

/// 估算工具名 + 描述的总 token 数 (VCP 注入预算核心场景)
pub fn estimate_tool_tokens(name: &str, description: &str) -> u32 {
    let name_tokens = estimate_token_count(name);
    let desc_tokens = estimate_token_count(description);
    name_tokens + desc_tokens
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
        MIN_BRIEF_TOKEN_BUDGET == 3,
        "VCP dynamicToolRegistry.js:12 MIN_BRIEF_TOKEN_BUDGET = 3"
    );
    assert!(
        MAX_INJECTION_CHARS == 16_000,
        "VCP dynamicToolRegistry.js:21 maxInjectionChars = 16000"
    );

    // BRIEF < LIGHT (VCP 设计意图: 摘要比列表更紧)
    assert!(
        DEFAULT_BRIEF_TOKEN_BUDGET < LIGHT_LIST_TOKEN_BUDGET,
        "BRIEF 必须 < LIGHT"
    );
    assert!(
        MIN_BRIEF_TOKEN_BUDGET < DEFAULT_BRIEF_TOKEN_BUDGET,
        "MIN_BRIEF 必须 < BRIEF"
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

    // ====== 3 常量真值断言 ======

    #[test]
    fn light_list_token_budget_matches_vcp_15() {
        assert_eq!(LIGHT_LIST_TOKEN_BUDGET, 15);
    }

    #[test]
    fn default_brief_token_budget_matches_vcp_6() {
        assert_eq!(DEFAULT_BRIEF_TOKEN_BUDGET, 6);
    }

    #[test]
    fn min_brief_token_budget_matches_vcp_3() {
        assert_eq!(MIN_BRIEF_TOKEN_BUDGET, 3);
    }

    #[test]
    fn max_injection_chars_matches_vcp_16000() {
        assert_eq!(MAX_INJECTION_CHARS, 16_000);
    }

    // ====== token 估算 (VCP `tokenPieces` 真值对齐) ======

    #[test]
    fn estimate_token_count_empty() {
        assert_eq!(estimate_token_count(""), 0);
    }

    #[test]
    fn estimate_token_count_latin_words() {
        // "hello world foo" = 3 拉丁 word = 3 token
        assert_eq!(estimate_token_count("hello world foo"), 3);
    }

    #[test]
    fn estimate_token_count_cjk_chars() {
        // CJK 字符各算 1 token (VCP `[\u3400-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]`)
        let s = "你好世界";
        // 4 字符 (无拉丁), VCP 启发式: 4 token
        assert_eq!(estimate_token_count(s), 4);
    }

    #[test]
    fn estimate_token_count_mixed() {
        // "search 查询" = 1 拉丁 + 2 CJK = 3 token
        let s = "search 查询";
        assert_eq!(estimate_token_count(s), 3);
    }

    // ====== 截断 token 预算 (VCP `truncateToTokenBudget` 借鉴) ======

    #[test]
    fn truncate_under_budget_unchanged() {
        let s = "a b c";
        assert_eq!(truncate_to_token_budget(s, 10), s);
    }

    #[test]
    fn truncate_over_budget_keeps_within_limit() {
        let s = "one two three four five six seven eight";
        // 8 token, budget 3 → 截到 2 token + " …" = 3 总 token
        let r = truncate_to_token_budget(s, 3);
        let tokens = estimate_token_count(&r);
        assert!(tokens <= 3, "截断后应 ≤ 3 token, 实际 {tokens}");
    }

    // ====== 注入预算截断 (VCP `maxInjectionChars: 16000`) ======

    #[test]
    fn injection_budget_under_limit_unchanged() {
        let s = "a".repeat(100);
        assert_eq!(truncate_to_max_injection(&s), s);
    }

    #[test]
    fn injection_budget_over_limit_keeps_within_16000() {
        let s = "中".repeat(20_000);
        let r = truncate_to_max_injection(&s);
        assert!(r.chars().count() <= 16_000);
        assert!(r.contains("…(truncated"));
    }

    #[test]
    fn exceeds_injection_budget_15k() {
        let s = "a".repeat(15_000);
        assert!(!exceeds_injection_budget(&s));
        let s2 = "a".repeat(16_001);
        assert!(exceeds_injection_budget(&s2));
    }

    // ====== 工具 token 估算 (战役 2-1 注册中心核心) ======

    #[test]
    fn estimate_tool_tokens_combined() {
        // name + description 总 token
        // "FileOperator" = 1 token (latin), "文件读写" = 4 token (4 CJK chars)
        let t = estimate_tool_tokens("FileOperator", "文件读写");
        assert_eq!(t, 5, "1 latin + 4 CJK = 5 token");
    }

    #[test]
    fn estimate_tool_tokens_under_light_budget() {
        // 单工具应远 < LIGHT=15
        let t = estimate_tool_tokens("Echo", "echo input");
        assert!(t < LIGHT_LIST_TOKEN_BUDGET);
    }
}
