//! 性质族 5 · 路径沙箱不逃逸 harness (capabilities/tools sensitive_path +
//! gateway file_fetcher)。
//!
//! 对应宣称: 任意字节串含 `..` / NUL / 绝对敏感路径均被拒; 白名单模式下
//! 根外路径必拒。命题覆盖两条判定链:
//!   - file_fetcher::TransparentFileFetcher::validate_path_safety —— 穿越/NUL
//!     拒绝 + 白名单根约束 (精确刻画, 双向);
//!   - capabilities/tools::sensitive_path::is_sensitive_path —— 敏感目录/
//!     敏感文件名模式对任意子分量的封闭性 (任意子名必拒)。
//!
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::file_fetcher::{FileFetchError, TransparentFileFetcher};
use super::sensitive_path::is_sensitive_path;
use std::path::{Path, PathBuf};

/// 任意 N 字节串 (含非法 UTF-8 → lossy 替换; 含 NUL/`..`/控制字符均可)。
fn bounded_string<const N: usize>() -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(N);
    for _ in 0..N {
        bytes.push(kani::any::<u8>());
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 证明: 无白名单时 validate_path_safety 的判定精确等于"含 `..` 或 NUL"——
/// 任意 ≤8 字节串含穿越/NUL 必拒 (PathTraversal), 不含则必过 (不过度拒绝)。
/// 边界: 路径串 8 字节, allowed_roots 为空 (白名单语义见下个 harness),
/// unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_sandbox_validate_rejects_traversal_and_nul() {
    let fetcher = TransparentFileFetcher::new("cache", Vec::new());
    let s = bounded_string::<8>();
    let hostile = s.contains("..") || s.contains('\0');

    match fetcher.validate_path_safety(&s) {
        Ok(()) => assert!(!hostile, "含 `..` 或 NUL 的任意字节串必拒"),
        Err(err) => assert!(
            hostile && matches!(err, FileFetchError::PathTraversal(_)),
            "无白名单时拒绝 iff 含穿越/NUL, 且错误类型为 PathTraversal"
        ),
    }
}

/// 证明: 白名单模式下根外路径必拒 —— 任意 ≤8 字节串, validate 通过则
/// 该路径在词法上必坐落于允许根之下 (且不含穿越/NUL)。
/// 边界: 单允许根 "root" (词法 starts_with 语义, 与实现同口径; 不含
/// 符号链接/真实 fs 解析), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_sandbox_whitelist_rejects_outside_root() {
    let root = PathBuf::from("root");
    let fetcher = TransparentFileFetcher::new("cache", vec![root.clone()]);
    let s = bounded_string::<8>();

    if let Ok(()) = fetcher.validate_path_safety(&s) {
        assert!(!s.contains("..") && !s.contains('\0'), "通过则不含穿越/NUL");
        assert!(
            PathBuf::from(&s).starts_with(&root),
            "白名单模式: 通过则路径必在允许根之下 (根外必拒)"
        );
    }
}

/// 证明: 敏感目录封闭性 —— 已知敏感目录 (.ssh/.aws/.gnupg/.secret/.secrets/
/// .kube/.docker 与 .config/gcloud) 之下, 任意文件名分量的路径必被判定敏感。
/// 边界: 文件名分量为任意 ≤4 字节串 (不含 `/` `\` `:` —— join 的单段名契约,
/// 含分隔符时 Path::join 会改写根, 属另一语义), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_sandbox_sensitive_dir_blocks_any_child() {
    let root = Path::new("root");
    let name = bounded_string::<4>();
    if name.contains('/') || name.contains('\\') || name.contains(':') {
        return; // 早退守卫: 单段名契约 (等价 kani::assume)
    }

    for dir in [
        ".ssh", ".aws", ".gnupg", ".secret", ".secrets", ".kube", ".docker",
    ] {
        let path = root.join(dir).join(&name);
        assert!(is_sensitive_path(root, &path), "敏感目录下任意子名必拒");
    }
    let gcloud = root.join(".config").join("gcloud").join(&name);
    assert!(
        is_sensitive_path(root, &gcloud),
        ".config/gcloud 下任意子名必拒"
    );
}

/// 证明: 敏感后缀封闭性 —— 私钥/凭据后缀 (.key/.pem/.p12/.pfx/.jks/.kdbx)
/// 之前缀任意文件名的路径必被判定敏感。
/// 边界: 文件名分量任意 ≤4 字节串 (单段名契约, 同上), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_sandbox_sensitive_suffix_blocks_any_name() {
    let root = Path::new("root");
    let name = bounded_string::<4>();
    if name.contains('/') || name.contains('\\') || name.contains(':') {
        return; // 早退守卫: 单段名契约 (等价 kani::assume)
    }

    for suffix in [".key", ".pem", ".p12", ".pfx", ".jks", ".kdbx"] {
        let path = root.join(format!("{name}{suffix}"));
        assert!(is_sensitive_path(root, &path), "私钥后缀 + 任意前缀必拒");
    }
}

/// 证明: 凭据类前缀/内嵌模式封闭性 —— `.env.{任意}`、`id_rsa.{任意}`、
/// `{任意}apikey` 的路径必被判定敏感。
/// 边界: 文件名分量任意 ≤4 字节串 (单段名契约, 同上), unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_sandbox_credential_pattern_blocks_any_affix() {
    let root = Path::new("root");
    let name = bounded_string::<4>();
    if name.contains('/') || name.contains('\\') || name.contains(':') {
        return; // 早退守卫: 单段名契约 (等价 kani::assume)
    }

    let env_variant = root.join(format!(".env.{name}"));
    assert!(is_sensitive_path(root, &env_variant), ".env.* 变体必拒");

    let key_variant = root.join(format!("id_rsa.{name}"));
    assert!(is_sensitive_path(root, &key_variant), "id_rsa.* 变体必拒");

    let apikey_variant = root.join(format!("{name}apikey"));
    assert!(is_sensitive_path(root, &apikey_variant), "内嵌 apikey 必拒");
}
