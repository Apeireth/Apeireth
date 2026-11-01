//! fuzz: 内容面脱敏与标签剥离（外部内容 → 我方展示面）。
//!
//! 两个靶子：
//! - `strip_tags`（content_feed）—— **先实体反转义再剥 `<...>`**。手写 HTML 剥标签，
//!   是经典的 XSS 绕过面：顺序反了（先剥标签后反转义）就会让 `&lt;script&gt;`
//!   变成真标签。这里反转义在前，所以要验的是**反转义后的剥离是否仍然有效**。
//! - `redact_attributes`（agent_trace）—— 敏感键值脱敏。
//!
//! **判据（契约断言）**：
//! `strip_tags` 的输出**不得再含 `<`** —— `<` 一旦出现即进入标签模式并被消费到底，
//! 所以输出里不可能再有 `<`，也就**不可能形成真标签**（XSS 绕过的前提）。
//!
//! ⚠️ 第一版我写成「不得含 `<` 或 `>`」，被 fuzz 抓到 —— **是断言过严，不是库的洞**：
//! 单独的 `>` 是普通文本（"a>b" → "a>b"），剥标签只移除 `<...>` 序列，不该吃掉它。
//! 这是我在同一个 target 上第二次犯「harness 自己的锅」（上一处是漏 `#![no_main]`）。
//! 教训同款：**fuzz 报 crash 先问是不是 harness 自己的锅**，尤其当断言是我自己加的。
//!
//! 另注：不能用「幂等性」当判据 —— `strip_tags` 是**单次**反转义语义，
//! `&amp;lt;` → `&lt;`（再剥一次才变 `<`），两次调用不等价是**正确**的。

#![no_main]

use apeireth_tools_canonical::content_feed::summary::strip_tags;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let out = strip_tags(s);
    assert!(
        !out.contains('<'),
        "strip_tags 输出仍含 `<`（存在反转义绕过，可形成真标签）: in={s:?} out={out:?}"
    );
});
