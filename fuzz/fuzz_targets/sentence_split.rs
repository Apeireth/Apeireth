//! fuzz: 流式分句器（`SentenceDivider::push_chunk`）。
//!
//! **面**：吃 LLM 逐 token 的流式文本，**按标点切句**。它内部用字节索引做
//! `buffer[start..end]` 切片——多字节标点/混合文本下是典型 panic 点。
//! 判据：不 panic，且切出来的每句必须是合法 UTF-8（`to_string()` 已隐含）。
#![no_main]

use apeireth_gateway::duplex_gateway::SentenceDivider;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let mut d = SentenceDivider::new();
    // 分两段喂，覆盖「跨 chunk 边界」的缓冲态。
    // ⚠️ 第一次跑 fuzz 就在这一行抓到 panic —— **是我 harness 的错，不是库的错**：
    // `s.len()/2` 落在多字节字符中间时 `split_at` 直接 panic（输入 `[0xD7,0xBB]` 两字节即触发）。
    // 修法：只在 **char 边界**切。这条留作教训——fuzz 报 crash 先问「是不是 harness 自己的锅」。
    let mid = s
        .char_indices()
        .nth(s.chars().count() / 2)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (a, b) = s.split_at(mid);
    let _ = d.push_chunk(a);
    let _ = d.push_chunk(b);
    let _ = d.flush();
});
