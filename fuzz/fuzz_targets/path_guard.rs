#![no_main]

//! fuzz: **共用路径守门（`ToolGuardrail::verify_path_access`）** —— 路径逃逸的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/guardrail.rs`
//! （`ToolGuardrail::verify_path_access`）。
//!
//! ## 为什么值得 fuzz（**这是所有走路径的工具共用的那道门**）
//!
//! `verify_path_access` 被**练习场沙箱面**（`runtime-assembly` 的 `play_tools`）、
//! 技能库、本地读面等多处**共用**（`play_tools` 直接委托它做「词法穿越 / 绝对敏感
//! 系统路径 / root 包含检查」）⇒ ⇒ **它松一道 = 一片面松**。
//! 实现写明的**三层判定**：
//! 1. **穿越特征**：路径串含 `../` 或 `..\` ⇒ 拒；
//! 2. **绝对敏感系统路径**（**前缀匹配 + 小写化**）：`/etc/shadow` · `/etc/passwd` ·
//!    `/etc/sudoers` · `/root` · `/var/run` · `/dev` · `c:\windows\system32` ·
//!    `c:\windows\system` ⇒ 拒；
//! 3. **绝对路径越界**：绝对路径 canonicalize 后**不在工作区根内** ⇒ 拒
//!    （**只在两边都能 canonicalize 时才判** ⇒ 非存在路径走不到这条 ⇒ 诚实边界）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **穿越必拒**：路径串含 `../` 或 `..\` ⇒ 必 Err | 判定 1 |
//! | O2 | **敏感前缀必拒**：八个敏感前缀**逐个**（含大小写变体 ⇒ 小写化后仍拒） | 判定 2 的词表 |
//! | O3 | **绝对越界必拒**：根外的**已存在**绝对路径 ⇒ 必 Err | 判定 3 |
//! | O4 | **根内绝对路径必放行** | 判定 3 的反面 |
//! | O5 | **相对路径不在工作区内时仍放行**（本守卫**不**管相对路径的落点 ⇒ 那是调用方的责任） | 判定 3 只对绝对路径生效 |
//! | O6 | **拒绝必须带真因**（不得空消息） | `PreCallGuardError` 的消息体 |
//!
//! ## 诚实边界
//!
//! - **不测 OS 级隔离**（实现自己写明：守门**不是**沙箱的替代）；
//! - **不测 `verify_shell_command`**（另一个函数，另立面）；
//! - **不测非存在绝对路径的越界**（实现写明：两边都能 canonicalize 才判 ⇒
//!   非存在路径走不到判定 3 ⇒ ⇒ 这是**已知的保守边界**，本 target **不假装它被覆盖**）。

use std::fs;
use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::ToolGuardrail;

/// 敏感前缀词表（抄自被测代码的 `forbidden_prefixes`，**逐个**压）。
const FORBIDDEN_PREFIXES: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/sudoers",
    "/root",
    "/var/run",
    "/dev",
    "c:\\windows\\system32",
    "c:\\windows\\system",
];

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };

    // ── 临时工作区根（canonicalize 后可用 ⇒ 判定 3 生效）
    let base = std::env::temp_dir().join(format!(
        "apeireth-path-guard-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, x| acc.wrapping_mul(31).wrapping_add(*x as u64))
    ));
    let _ = fs::remove_dir_all(&base);
    let root: PathBuf = base.join("workspace");
    let inside = root.join("inside.txt");
    let _ = fs::create_dir_all(&root);
    let _ = fs::write(&inside, "inside");
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());

    // ── 候选路径：词表 + 数据驱动
    let candidates: Vec<String> = vec![
        "../escape".to_string(),
        "..\\escape".to_string(),
        "a/../b".to_string(),
        "a/..\\b".to_string(),
        "..".to_string(),
        "/etc/shadow".to_string(),
        "/etc/passwd".to_string(),
        "/etc/sudoers".to_string(),
        "/root".to_string(),
        "/root/secret".to_string(),
        "/var/run".to_string(),
        "/dev".to_string(),
        "/dev/null".to_string(),
        "C:\\Windows\\System32".to_string(),
        "c:\\windows\\system".to_string(),
        "/etc".to_string(),
        "/etc/".to_string(),
        "inside.txt".to_string(),
        "sub/inside.txt".to_string(),
        canonical_root.join("inside.txt").to_string_lossy().into_owned(),
        "/tmp/outside-root".to_string(),
        String::from_utf8_lossy(data.get(1..).unwrap_or(&[])).into_owned(),
    ];
    let raw = candidates[(b(0) as usize) % candidates.len()].clone();
    let path = PathBuf::from(&raw);

    let result = ToolGuardrail::verify_path_access(&canonical_root, &path);

    // ── O1 穿越必拒（含 `../`、`..\` 与**裸 `..`** —— 实现第 1 条的三个条件）
    if raw.contains("../") || raw.contains("..\\") || raw == ".." {
        assert!(
            result.is_err(),
            "穿越特征必须拒绝即帧: raw={raw:?}"
        );
    }

    // ── O2 敏感前缀必拒（小写化 ⇒ 大小写变体同样拒）
    let lower = raw.to_lowercase();
    for prefix in FORBIDDEN_PREFIXES {
        if lower.starts_with(prefix) {
            assert!(
                result.is_err(),
                "敏感系统路径前缀必须拒绝即帧: raw={raw:?} prefix={prefix:?}"
            );
        }
    }

    // ── O3 绝对越界必拒（根外的**已存在**绝对路径）
    if path.is_absolute() && path.canonicalize().is_ok() {
        let canonical = path.canonicalize().unwrap();
        if !canonical.starts_with(&canonical_root) {
            // 注意 `/etc`、`/dev` 这类**敏感前缀**先被判定 2 拦掉 ⇒ 这里只断言「必拒」
            assert!(
                result.is_err(),
                "工作区外的已存在绝对路径必须拒绝即帧: raw={raw:?}"
            );
        }
    }

    // ── O4 根内绝对路径必放行
    if path.is_absolute() && path.canonicalize().is_ok() {
        let canonical = path.canonicalize().unwrap();
        if canonical.starts_with(&canonical_root) {
            assert!(
                result.is_ok(),
                "工作区内的绝对路径必须放行: raw={raw:?} err={:?}",
                result.err()
            );
        }
    }

    // ── O5 相对路径放行（本守卫不管相对路径落点）
    // ⚠️ 我第一版漏了「**不命中敏感前缀**」这一条 ⇒ ⇒ 假阳性（第 18 次）：
    // `C:\Windows\System32` 在 Linux 上是**相对路径**（不是绝对路径）
    // ⇒ ⇒ 它命中的是**判定 2（敏感前缀）** ⇒ **被拒是「更严」不是缺陷**
    // ⇒ ⇒ O5 的前提必须是「既不命中穿越、也不命中敏感前缀」。
    let hits_sensitive = FORBIDDEN_PREFIXES
        .iter()
        .any(|prefix| raw.to_lowercase().starts_with(prefix));
    if !path.is_absolute()
        && !raw.contains("../")
        && !raw.contains("..\\")
        && raw != ".."
        && !hits_sensitive
    {
        assert!(
            result.is_ok(),
            "相对路径不在本守卫的判定范围（应放行）: raw={raw:?} err={:?}",
            result.err()
        );
    }

    // ── O6 拒绝必须带真因
    if let Err(error) = &result {
        let message = error.to_string();
        assert!(
            !message.trim().is_empty(),
            "拒绝必须带真因（不得空消息）: raw={raw:?}"
        );
    }

    let _ = fs::remove_dir_all(&base);
});
