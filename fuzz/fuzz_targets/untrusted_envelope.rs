#![no_main]

//! fuzz: **不可信内容信封**（`governance::untrusted_mark`）—— 提示注入防线。
//!
//! 这是「外部抓取内容 / 工具回包」进入模型上下文前的隔离层：
//! 把不可信内容包在强边界信封里，并中和内容中企图**提前闭合边界**的字面量，
//! 从根上阻断间接提示词注入。所以它自己要经得起**不可信输入**。
//!
//! 判据分三层（都不依赖自造期望）：
//!
//! 1. **不 panic**。`unwrap_content` 是公开 API 且吃任意文本：
//!    它按 `len() - CLOSE.len() - 1` 算 body 末尾 —— 若调用方传入的文本
//!    「首尾像信封」但 CLOSE 前最后一个字符是多字节字符，该偏移会落在字符中间，
//!    切片即 panic。`wrap()` 自己总会插 `\n`，所以只有**外部伪造的信封**能触发。
//!
//! 2. **`wrap` 的输出必须能被 `is_wrapped` 认出来**（封装自洽）。
//!
//! 3. **中和必须有效**：`wrap` 之后，信封内部**不得再出现**原样的边界开启/闭合字面量。
//!    这是防注入的核心承诺 —— 若内容里能留下未中和的闭合标记，
//!    模型就可能把后续内容当成"信封之外"的指令。

use apeireth_governance::untrusted_mark::{
    UNTRUSTED_TAG_CLOSE, UNTRUSTED_TAG_OPEN, UntrustedContentWrapper,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // ── ① 不可信文本面的 API 不得 panic（含伪造信封）──────────────────────
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = UntrustedContentWrapper::is_wrapped(text);
        let _ = UntrustedContentWrapper::unwrap_content(text);
        let _ = UntrustedContentWrapper::neutralize_escapes(text);

        // 刻意构造「首尾像信封」的文本（伪造信封），覆盖 unwrap 的边界计算。
        for body in [text, ""] {
            let fake =
                format!("{UNTRUSTED_TAG_OPEN} source=\"fuzz\"]>>>\n{body}{UNTRUSTED_TAG_CLOSE}");
            let _ = UntrustedContentWrapper::unwrap_content(&fake);
            // 不带 "\n" 分隔的伪造形态（最可能踩到字符边界）
            let fake_no_nl =
                format!("{UNTRUSTED_TAG_OPEN} source=\"fuzz\"]>>>{body}{UNTRUSTED_TAG_CLOSE}");
            let _ = UntrustedContentWrapper::unwrap_content(&fake_no_nl);
        }
    }

    // ── ②③ 封装自洽 + 中和有效 ───────────────────────────────────────────
    let Ok(content) = std::str::from_utf8(data) else {
        return;
    };
    let wrapped = UntrustedContentWrapper::wrap("fuzz_source", content);

    assert!(
        UntrustedContentWrapper::is_wrapped(&wrapped),
        "wrap 的输出没能被 is_wrapped 认出（封装不自洽）"
    );

    // 中和后的内容里不得残留原样的边界字面量。
    // 只在**信封内部**判定：去掉首部 OPEN 与尾部 CLOSE 之后再看。
    let neutralized = UntrustedContentWrapper::neutralize_escapes(content);
    assert!(
        !neutralized.contains(UNTRUSTED_TAG_OPEN),
        "中和后仍含边界开启字面量: {content:?}"
    );
    assert!(
        !neutralized.contains(UNTRUSTED_TAG_CLOSE),
        "中和后仍含边界闭合字面量: {content:?}"
    );

    // 能解包时，解包出来的内容必须等于中和后的内容（往返一致）。
    if let Some(payload) = UntrustedContentWrapper::unwrap_content(&wrapped) {
        assert_eq!(payload.source, "fuzz_source", "解包出的来源不符");
        assert_eq!(
            payload.content, neutralized,
            "往返不一致: wrap→unwrap 的内容与中和后的原文不同"
        );
    }
});
