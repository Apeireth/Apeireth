//! JSONL 研究日志 + config_hash (对齐 research/runners 的零依赖口径).
//!
//! research/runners 中 `config_hash` / `log_event` 是私有 fn, 本 crate 按同一风格
//! 重新实现 (只写 research/b1, 不改 runners)。schema 对齐 research/logs/README.md。

/// FNV-1a 64 的 8 hex 位 config hash (零依赖, 与 research/runners 同款)。
/// `extra` 参与哈希: 不同 seed / 阈值 / 上限 / 模式不得互相覆盖日志。
pub fn config_hash(seed: u64, experiment: &str, extra: &str) -> String {
    let s = format!("{seed}:{extra}:{experiment}");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:08x}")[..8].to_string()
}

/// 追加一条 JSONL 事件。`payload` 必须是已序列化的合法 JSON 文本
/// (调用方用 `serde_json::to_string` 生成, 转义由 serde 保证)。
pub fn log_event(
    file: &mut String,
    ts: &str,
    experiment: &str,
    seed: u64,
    config_hash: &str,
    event: &str,
    payload: &str,
) {
    file.push_str(&format!(
        "{{\"ts\":\"{ts}\",\"experiment\":\"{experiment}\",\"seed\":{seed},\"config_hash\":\"{config_hash}\",\"event\":\"{event}\",\"payload\":{payload}}}\n"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_hash_is_deterministic_and_sensitive() {
        let a = config_hash(42, "b1-betti", "150:0.05");
        let b = config_hash(42, "b1-betti", "150:0.05");
        let c = config_hash(42, "b1-betti", "150:0.10");
        let d = config_hash(43, "b1-betti", "150:0.05");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_eq!(a.len(), 8);
    }

    #[test]
    fn log_event_emits_one_json_line() {
        let mut f = String::new();
        log_event(&mut f, "t", "exp", 42, "hash", "evt", "{\"x\":1}");
        assert!(f.ends_with('\n'));
        // 单行 JSONL: 不得包含裸换行 (payload 已由 serde 转义)。
        assert_eq!(f.lines().count(), 1);
    }
}
