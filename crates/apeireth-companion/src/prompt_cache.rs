//! `apeireth-companion::prompt_cache` — Prompt Cache 稳定化 (吸收 NemesisBot 思想, 重写).
//!
//! 原则: **稳定前缀逐字节不变, 动态字段收敛到单一注入点**.
//! - system prompt + 已确认历史轮次 = 稳定前缀 (byte-identical, 最大化缓存命中)
//! - 时间/环境等动态字段作为一条 ephemeral system 消息, 只插到**最新 user 消息之前**
//! - 这样缓存 miss 只计「尾部 user + 动态注人」, 不重算前缀
//!
//! 0 假装: 这里做「组装结构」保证稳定; 缓存命中率指标 (cache hit/miss) 是上层的事.

use serde_json::{json, Value};

/// 组装消息: 稳定前缀 (system + history) 原样 + 动态注人单点.
///
/// `dynamic_note` 非空时, 作为 `{"role":"system","content":...}` 插入到
/// **最后一个 user 消息之前** (与 NemesisBot 同思路: 稳定前缀 = system + 注人前的历史,
/// 动态字段只影响尾部, 缓存 miss 只计尾部).
pub fn build_messages(system: &str, history: &[Value], dynamic_note: Option<&str>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::with_capacity(history.len() + 2);
    out.push(json!({"role": "system", "content": system}));
    if history.is_empty() {
        return out;
    }
    // 找到最后一个 user 消息的下标 (没有则插到末尾)
    let last_user = history
        .iter()
        .rposition(|m| m["role"].as_str() == Some("user"));
    match (dynamic_note, last_user) {
        (Some(note), Some(idx)) => {
            out.extend(history[..idx].iter().cloned());
            out.push(json!({"role": "system", "content": format!("[当前状态] {note}")}));
            out.extend(history[idx..].iter().cloned());
        }
        (Some(note), None) => {
            out.extend(history.iter().cloned());
            out.push(json!({"role": "system", "content": format!("[当前状态] {note}")}));
        }
        (None, _) => out.extend(history.iter().cloned()),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history() -> Vec<Value> {
        vec![
            json!({"role": "user", "content": "任务: 写错题本"}),
            json!({"role": "assistant", "content": "我先查记忆", "tool_calls": []}),
            json!({"role": "tool", "tool_call_id": "c1", "content": "找到 2 条"}),
            json!({"role": "user", "content": "继续"}),
        ]
    }

    #[test]
    fn stable_prefix_is_byte_identical() {
        let h = history();
        let a = build_messages("固定系统提示", &h, Some("2026-08-16 06:40"));
        let b = build_messages("固定系统提示", &h, Some("2026-08-17 07:00"));
        // 前缀 (system + 直到最新 user 前的历史) 逐字节相同 — 只有动态注人不同
        assert_eq!(a[0]["content"], b[0]["content"], "system prompt 必须逐字节稳定");
        // 动态注人: 位置在最后一个 user 之后 (插在它前面 = history[..=last_user] 后)
        let note_pos_a = a
            .iter()
            .position(|m| m["content"].as_str().unwrap_or("").contains("[当前状态]"))
            .unwrap();
        let note_pos_b = b
            .iter()
            .position(|m| m["content"].as_str().unwrap_or("").contains("[当前状态]"))
            .unwrap();
        assert_eq!(a[note_pos_a]["content"], json!("[当前状态] 2026-08-16 06:40"));
        assert_eq!(b[note_pos_b]["content"], json!("[当前状态] 2026-08-17 07:00"));
        // 除动态注人外, 其它消息序列一致
        let mut a2 = a.clone();
        let mut b2 = b.clone();
        a2.remove(note_pos_a);
        b2.remove(note_pos_b);
        assert_eq!(a2, b2, "除动态注人外消息必须一致");
    }

    #[test]
    fn dynamic_note_inserted_before_last_user() {
        let h = history();
        let msgs = build_messages("sys", &h, Some("现在 6:40"));
        let note_idx = msgs
            .iter()
            .position(|m| m["content"].as_str().unwrap_or("").contains("[当前状态]"))
            .unwrap();
        // 注人后紧跟着最后一条 user ("继续") — 动态字段只影响尾部
        assert_eq!(msgs[note_idx + 1]["content"], json!("继续"));
        // 注人前的稳定前缀 = system + history[..last_user], 与 h 对应段逐字节一致
        assert_eq!(msgs[0]["content"], json!("sys"));
        assert_eq!(msgs[1], h[0]);
        assert_eq!(msgs[2], h[1]);
        assert_eq!(msgs[3], h[2]);
        assert_eq!(msgs[note_idx - 1], h[2], "注人前应接 tool 结果");
    }

    #[test]
    fn no_note_means_history_only() {
        let h = history();
        let msgs = build_messages("sys", &h, None);
        assert_eq!(msgs.len(), 1 + h.len());
        assert!(msgs.iter().all(|m| !m["content"].as_str().unwrap_or("").contains("[当前状态]")));
    }

    #[test]
    fn empty_history_returns_system_only() {
        let msgs = build_messages("sys", &[], Some("x"));
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], json!("system"));
    }
}
