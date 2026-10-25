#![no_main]

//! fuzz: **IM 审批风险级推导**（`approval_risk_level`）—— 审批要求不得被治理文本降级。
//!
//! 这个函数从**治理文本**（`reason` + `hook`）里推导风险等级，而风险等级决定
//! 「要不要人批」。所以它必须满足一条安全性质：
//!
//! > **永不低于 `high` 基线** —— 伪造/构造治理文本不能把风险降级，
//! > 从而不能绕过人批要求。
//!
//! 该函数的结构本身就是 fail-safe 的：`best` 从 `2`（high）起，词表匹配只能
//! `max` 上去。本 target 把这条**结构性保证固化成断言**，防止将来被"优化"成
//! 「按文本选级」而静默降级（那会是审批绕过）。
//!
//! 判据：
//! 1. 结果必属词表标签之一（不返回未定义级）；
//! 2. **rank(结果) ≥ 2**（info/low=0 · medium=1 · high=2 · critical=3 · nuclear=4）；
//! 3. 升级必须来自**真实整词命中**：结果是 `critical` / `nuclear` 时，
//!    输入必然含该整词（防止"级别凭空空跳"）；
//! 4. 确定性：同输入两次调用结果相同；
//! 5. 任意 UTF-8 不 panic。

use apeireth_gateway::im_approval::approval_risk_level;
use libfuzzer_sys::fuzz_target;

/// 逐字取自 `im_approval.rs` 的 `RISK_VOCABULARY`。
const VOCAB: &[(&str, i32)] = &[
    ("info", 0),
    ("low", 0),
    ("medium", 1),
    ("high", 2),
    ("critical", 3),
    ("nuclear", 4),
];

/// 人批基线：不得低于此级。
const BASELINE_RANK: i32 = 2;

fn rank_of(label: &str) -> Option<i32> {
    VOCAB.iter().find(|(l, _)| *l == label).map(|(_, r)| *r)
}

/// 与实现同口径的「整词」判定（按非字母数字切分）。
fn has_whole_word(haystack_lower: &str, word: &str) -> bool {
    haystack_lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == word)
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // 拆成两半当 reason / hook（两段都喂，覆盖拼接面）。
    let mid = text
        .char_indices()
        .nth(text.chars().count() / 2)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let (reason, hook) = text.split_at(mid);

    let level = approval_risk_level(reason, hook);

    // ① 结果必在词表内
    let rank =
        rank_of(&level).unwrap_or_else(|| panic!("返回了词表外的风险级: {level:?}（in={text:?}）"));

    // ② **核心安全性质：永不低于 high 基线**
    assert!(
        rank >= BASELINE_RANK,
        "风险级被降级到 high 以下 —— 审批要求可被治理文本绕过: level={level} rank={rank} in={text:?}"
    );

    // ③ 升级必须来自真实整词命中
    let haystack = format!("{reason} {hook}").to_ascii_lowercase();
    if rank > BASELINE_RANK {
        assert!(
            has_whole_word(&haystack, &level),
            "风险级升到 {level} 但输入里没有该整词 —— 级别来源不明: in={text:?}"
        );
    }

    // ④ 确定性
    assert_eq!(
        level,
        approval_risk_level(reason, hook),
        "同一输入两次调用结果不一致"
    );

    // ⑤ 空文本也要落在基线（不是 panic / 不是空串）
    if text.trim().is_empty() {
        assert_eq!(level, "high", "空治理文本必须落 high 基线: got {level:?}");
    }
});
