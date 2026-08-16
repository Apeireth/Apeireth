//! **TP3/N21 / SecretString — 凭据明文脱敏载体**
//!
//! **红线 (任务纪律 ①②)**: 凭据明文**不得**出现在任何日志 / 错误消息 /
//! `Debug` / `Display` 输出里。`SecretString` 通过覆写 `Debug` 与 `Display`,
//! 保证 `format!`/`{}`/`{:?}` 一律得到**脱敏串**, 明文只经 [`SecretString::expose`]
//! 显式取出 (调用点自担"只在必要瞬间持有"的责任)。
//!
//! **0 假装边界 (诚实标注)**: 本类型**不是**内存擦除/防 swap 的安全容器
//! (未做 zeroize / mlock), 只是"不泄漏到输出通道"的脱敏载体。
//! 内存级安全擦除属后续层, 此处如实标注。

/// 凭据明文脱敏包装。
///
/// `Debug` / `Display` 输出恒为 `[REDACTED len=<字节数>]`, 不含明文。
/// 明文仅经 [`Self::expose`] 显式取出。
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// 从明文构造 (构造即纳入脱敏保护)。
    pub fn new(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    /// 显式取出明文 (调用点负责最小持有时间; 不写日志)。
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// 明文字节长度 (脱敏可透露的元信息, 不含内容)。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 脱敏串 (供展示/日志安全拼接): `[REDACTED len=N]`。
    pub fn redacted(&self) -> String {
        redact_len(self.0.len())
    }
}

/// 覆写 `Debug`: 绝不输出明文。
impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

/// 覆写 `Display`: 绝不输出明文。
impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

/// 按长度生成脱敏串: `[REDACTED len=<n>]`。
///
/// 只透露长度 (定位用元信息), 不透露任何明文字符。
pub fn redact_len(len: usize) -> String {
    format!("[REDACTED len={len}]")
}

/// 对任意字符串做"保首尾、掩中段"的展示脱敏 (仅供 UI 提示, 非存储)。
///
/// 例: `sk-abc...xyz`。长度 ≤ 4 时全掩, 不透任何字符。
/// **注**: 此为展示友好脱敏, 与日志红线用的 [`SecretString`] (全掩) 分途。
pub fn mask_for_display(secret: &str) -> String {
    let n = secret.len();
    if n <= 4 {
        return "*".repeat(n.max(4));
    }
    let head: String = secret.chars().take(3).collect();
    let tail: String = secret.chars().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_leaks_plaintext() {
        let s = SecretString::new("super-secret-token-12345");
        let dbg = format!("{s:?}");
        let disp = format!("{s}");
        assert!(!dbg.contains("super-secret"), "Debug 泄漏明文: {dbg}");
        assert!(!disp.contains("super-secret"), "Display 泄漏明文: {disp}");
        assert!(dbg.contains("[REDACTED"));
        assert!(disp.contains("[REDACTED"));
    }

    #[test]
    fn redacted_carries_length_only() {
        let s = SecretString::new("abcdef");
        assert_eq!(s.redacted(), "[REDACTED len=6]");
        assert_eq!(s.len(), 6);
        assert!(!s.is_empty());
    }

    #[test]
    fn expose_returns_plaintext_for_authorized_use() {
        let s = SecretString::new("master-token-value");
        assert_eq!(s.expose(), "master-token-value");
    }

    #[test]
    fn empty_secret_handled() {
        let s = SecretString::new("");
        assert!(s.is_empty());
        assert_eq!(s.redacted(), "[REDACTED len=0]");
    }

    #[test]
    fn mask_for_display_short_fully_masked() {
        assert_eq!(mask_for_display("ab"), "****");
        assert_eq!(mask_for_display(""), "****");
        // 短串不透任何原字符
        assert!(!mask_for_display("ab").contains('a'));
    }

    #[test]
    fn mask_for_display_long_keeps_head_tail() {
        let m = mask_for_display("sk-live-abcdefghijklmnop");
        assert!(m.starts_with("sk-"), "应保留头部: {m}");
        assert!(m.ends_with("nop"), "应保留尾部: {m}");
        assert!(m.contains('…'));
        assert!(!m.contains("live-abcdefghij"), "中段不应透出: {m}");
    }
}
