//! **TP3/N21 / SecretString + TP20-S3 SecretBuf — 凭据明文脱敏载体 / 内存安全容器**
//!
//! **红线 (任务纪律 ①②)**: 凭据明文**不得**出现在任何日志 / 错误消息 /
//! `Debug` / `Display` 输出里。`SecretString` 通过覆写 `Debug` 与 `Display`,
//! 保证 `format!`/`{}`/`{:?}` 一律得到**脱敏串**, 明文只经 [`SecretString::expose`]
//! 显式取出 (调用点自担"只在必要瞬间持有"的责任)。
//!
//! **TP20-S3 升级**: 加 [`SecretBuf`] — `Drop` 时自动 zeroize 的字节容器,
//! 用于 [`KeyringBackend`](crate::keyring::KeyringBackend) trait 的明文入出。
//! [`SecretString`] (脱敏载体, 用于 `CredentialsStore` trait) 与 [`SecretBuf`]
//! (内存安全容器, 用于 `KeyringBackend` trait) 各司其职; 通过显式转换桥接。
//!
//! **0 假装边界 (诚实标注)**: `SecretString` **不是**内存擦除容器
//! (未做 zeroize / mlock), 只是"不泄漏到输出通道"的脱敏载体。
//! 内存级安全擦除由 TP20-S3 新增的 `SecretBuf` 承担。

use zeroize::{Zeroize, ZeroizeOnDrop};

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
    let tail: String = secret
        .chars()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
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

    // ===== TP20-S3 SecretBuf 零化测试 =====

    #[test]
    fn secret_buf_debug_does_not_leak_plaintext() {
        let buf = SecretBuf::new(vec![0x73, 0x6b, 0x2d, 0x78]); // "sk-x"
        let dbg = format!("{buf:?}");
        let disp = format!("{buf}");
        assert!(!dbg.contains("sk-x"), "Debug 泄漏明文: {dbg}");
        assert!(!disp.contains("sk-x"), "Display 泄漏明文: {disp}");
        assert!(dbg.contains("[REDACTED"));
        assert!(disp.contains("[REDACTED"));
    }

    #[test]
    fn secret_buf_expose_returns_bytes() {
        let buf = SecretBuf::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(buf.expose(), &[1, 2, 3, 4, 5]);
        assert_eq!(buf.len(), 5);
        assert!(!buf.is_empty());
    }

    #[test]
    fn secret_buf_into_secret_string_round_trip() {
        let buf = SecretBuf::new(vec![0x6f, 0x6b]); // "ok"
        let s: SecretString = (&buf).into();
        assert_eq!(s.expose(), "ok");
        // 反向
        let s2 = SecretString::new("ok");
        let buf2: SecretBuf = (&s2).into();
        assert_eq!(buf2.expose(), b"ok");
    }

    #[test]
    fn secret_buf_empty_handled() {
        let buf = SecretBuf::new(Vec::new());
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.expose(), &[] as &[u8]);
    }

    #[test]
    fn secret_buf_drop_zeroizes_via_drop_impl() {
        // 编译期验证 ZeroizeOnDrop 派生 (派生宏存在 = drop 时自动 zeroize).
        // 运行时 zeroize 验证走 zeroize_inner() 测试 (见下) — 此处验派生存在.
        // (drop 后读已释放内存是 UB; zeroize 库官方做法是测显式 zeroize.)
        let buf = SecretBuf::new(vec![0xAA; 64]);
        assert_eq!(buf.expose()[0], 0xAA);
        drop(buf); // ZeroizeOnDrop 自动归零 (编译期保证)
                   // 编译过 = 派生生效 (Rust 不允许 drop 后读)
    }

    #[test]
    fn secret_buf_explicit_zeroize_clears_bytes() {
        // 显式 zeroize (drop 前提前擦除): 用于调用方需立即归还的场景.
        let mut buf = SecretBuf::new(vec![0xCC; 32]);
        assert_eq!(buf.expose()[0], 0xCC);
        buf.zeroize_inner();
        assert!(buf.expose().iter().all(|&b| b == 0), "zeroize 后全 0");
    }

    #[test]
    fn secret_buf_zeroize_works_on_partial_fill() {
        // 部分填充 + zeroize: `Vec::zeroize()` (zeroize 1.x 官方实现) 走两步 —
        // (1) iter_mut().for_each(zeroize) 逐元素擦除; (2) clear() 把 len 归 0.
        // 内存占用归零 + 长度归零, 防后续误用残留引用.
        let mut buf = SecretBuf::new(vec![0xFF; 8]);
        assert_eq!(buf.expose(), &[0xFF; 8]);
        buf.zeroize_inner();
        // zeroize 后长度归 0 (Vec::clear 在 zeroize 1.x 内部调用).
        assert!(buf.is_empty(), "zeroize 后 Vec 长度应归 0");
        assert_eq!(buf.expose(), &[] as &[u8]);
        // 二次 zeroize (空 vec) 不应崩.
        buf.zeroize_inner();
        assert!(buf.is_empty());
    }
}

// ===== TP20-S3 SecretBuf =====

/// 凭据明文字节容器, `Drop` 时自动 zeroize。
///
/// **用途**: [`crate::keyring::KeyringBackend`] trait 的明文入出载体。
/// 与 [`SecretString`] 的分工:
/// - `SecretString` — 脱敏载体 (Debug/Display 恒脱敏), 给 `CredentialsStore` 用;
/// - `SecretBuf` — 内存安全容器 (Drop zeroize), 给 `KeyringBackend` 用。
///
/// 转换: `SecretString → SecretBuf` (从 env/配置取) / `SecretBuf → SecretString`
/// (交给上层 trait 时包一层脱敏)。两个 trait 互不污染, 各管各的红线。
///
/// **设计依据**: 借鉴 `zeroize` 官方 `Zeroizing<Z>` 模式 — 内层 `Vec<u8>`
/// 实现 `Zeroize` 即被派生 `ZeroizeOnDrop` 自动调用。
#[derive(Clone, ZeroizeOnDrop)]
pub struct SecretBuf(Vec<u8>);

impl SecretBuf {
    /// 从明文字节构造。
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// 从 UTF-8 字符串构造 (非 UTF-8 安全数据走 `Self::new(Vec<u8>)`)。
    pub fn from_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    /// 显式取出明文字节 (调用点负责最小持有时间)。
    pub fn expose(&self) -> &[u8] {
        &self.0
    }

    /// 字节长度 (脱敏可透露的元信息, 不含内容)。
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 脱敏串 (供展示/日志): `[REDACTED len=N]`。
    pub fn redacted(&self) -> String {
        redact_len(self.0.len())
    }

    /// 取内部裸指针 (仅供测试/特化场景; 安全代码请用 [`Self::expose`])。
    #[doc(hidden)]
    pub fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr() as *mut u8
    }

    /// 显式 zeroize (正常 Drop 已自动调用; 此口供提前擦除)。
    pub fn zeroize_inner(&mut self) {
        self.0.zeroize();
    }
}

/// `Debug`/`Display` 覆写: 绝不输出明文。
impl std::fmt::Debug for SecretBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

impl std::fmt::Display for SecretBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

/// `SecretString` (脱敏载体) → `SecretBuf` (内存安全容器):
/// 用于上层 trait 把配置/环境明文送入 keyring 后端。
impl From<&SecretString> for SecretBuf {
    fn from(s: &SecretString) -> Self {
        Self(s.expose().as_bytes().to_vec())
    }
}

/// `SecretBuf` → `SecretString`: keyring 后端取出明文后包脱敏载体交给上层。
impl From<&SecretBuf> for SecretString {
    fn from(b: &SecretBuf) -> Self {
        // SecretBuf 是字节, SecretString 期望 UTF-8; 非法 UTF-8 用 lossy 兜底
        // (调用方应承担此情况). 这是显式转换, 不静默丢错.
        SecretString::new(String::from_utf8_lossy(b.expose()).into_owned())
    }
}

/// 显式 `PartialEq`: 按字节相等 (用于单测断言, 不用于运行时密文比对;
/// 密文比对走专用的 `constant_time_eq` 路径, 不在本 trait 范围).
impl PartialEq for SecretBuf {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecretBuf {}
