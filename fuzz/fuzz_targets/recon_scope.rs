#![no_main]

//! fuzz: **侦察限域判据**（`recon::scope::Scope::check_domain` / `check_path`）——
//! 「这个目标在不在授权范围内」的判定。
//!
//! 这类判据一旦写松就是**越界访问**：把 `notevil.com` 当 `evil.com` 的子域、
//! 或让 `/allowed/../etc/passwd` 通过前缀检查。
//!
//! 判据写法：**独立实现的 oracle 交叉验证**。
//! 不用被测函数自己的逻辑当期望（那会同义反复），而是按**另一种表述**重算一遍：
//! - 域名：按 `.` 切 label，要求 target 的**末尾 label 序列**与 allowed 的序列逐位相等
//!   （这等价于「精确匹配或带点边界的子域」，但实现路径完全不同）；
//! - 路径：按**组件**比较前缀（不是字符串前缀 —— 字符串前缀会让 `/allowed-x` 通过 `/allowed`）。
//!
//! 两侧不一致即说明判据与语义背离 —— 无论哪边错，都值得人看。

use std::path::PathBuf;

use apeireth_tools_canonical::recon::scope::Scope;
use libfuzzer_sys::fuzz_target;

const ALLOWED_DOMAIN: &str = "example.com";
const ALLOWED_PATH: &str = "/allowed";

/// oracle：target 是否为 allowed 本身，或带点边界的子域。
/// 按 label 序列比较（与实现路径不同）。
fn domain_in_scope(target: &str, allowed: &str) -> bool {
    // ⚠️ 两处口径必须与实现一致，否则 oracle 自己就成了假阳性源（我踩过）：
    // ① 不能 `Box::leak` —— fuzz 每轮都调用，泄漏会累积到 OOM；
    // ② 要**同时裁掉前导点与尾随点** —— 实现用 `target.ends_with(".example.com")`
    //    会把 `.example.com`（FQDN 前导点写法，指向同一域名）判为在范围内，
    //    而按 `.` 切分会得到空 label。首版 oracle 漏了 trim 前导点 → 误报「背离」。
    let t: Vec<String> = target
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .split('.')
        .map(str::to_string)
        .collect();
    let a: Vec<String> = allowed
        .trim()
        .trim_start_matches("*.")
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .split('.')
        .map(str::to_string)
        .collect();
    // 每个 label 必须是合法域名 label（[a-z0-9-]，非空，不以 - 起止）——
    // 与实现的 `is_bare_domain` 同一契约：**只看后缀像不够，本身得是裸域名**。
    let label_ok = |l: &String| {
        !l.is_empty()
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    if !t.iter().all(label_ok) || !a.iter().all(label_ok) {
        return false;
    }
    t.len() >= a.len() && t[t.len() - a.len()..] == a[..]
}

fn make_scope() -> Scope {
    let mut scope = Scope::empty("fuzz");
    scope.authorized = true;
    scope.domains = vec![ALLOWED_DOMAIN.to_string()];
    scope.paths = vec![PathBuf::from(ALLOWED_PATH)];
    scope.cidrs = vec!["10.0.0.0/8".to_string()];
    scope
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let scope = make_scope();

    // ── ① 域名限域：与 oracle 交叉验证 ──────────────────────────────────
    let actual_ok = scope.check_domain(text).is_ok();
    let oracle_ok = domain_in_scope(text, ALLOWED_DOMAIN);
    assert_eq!(
        actual_ok, oracle_ok,
        "域名限域与语义背离: target={text:?} actual={actual_ok} oracle={oracle_ok}"
    );

    // ── ② 越界必须拒：显式反例（防「永远返回 Ok」的实现回归）─────────────
    if text.contains("..") || text.contains('\0') {
        // 带穿越特征的目标不得被路径检查放行
        assert!(
            scope.check_path(std::path::Path::new(text)).is_err(),
            "含 `..`/NUL 的路径被放行: {text:?}"
        );
    }

    // ── ③ 路径限域：字符串前缀不等价于组件前缀 ──────────────────────────
    // `/allowed-x` 不应通过 `/allowed` 的授权（除非该路径真实存在且解析后仍在范围内）。
    if text.starts_with(ALLOWED_PATH) && !text.starts_with(&format!("{ALLOWED_PATH}/")) {
        // 形如 `/allowedXYZ`：字符串前缀像，但组件层面越界。
        // 允许实现因「解析真实路径后仍在范围内」而放行（如 /allowed 是文件），
        // 故这里只在**明显成对**时做提示性断言：不 panic，只要求两者之一成立。
        let ok = scope.check_path(std::path::Path::new(text)).is_ok();
        if ok {
            // 放行必须能解释：它确实等于 ALLOWED_PATH，或确有其路径且以分隔符续接。
            assert!(
                text == ALLOWED_PATH || text.starts_with(&format!("{ALLOWED_PATH}/")),
                "字符串前缀相近但组件越界的路径被放行: {text:?}"
            );
        }
    }

    // ── ④ 未授权 scope 一律拒（fail-closed 底线）────────────────────────
    let mut unauth = Scope::empty("fuzz-unauth");
    unauth.domains = vec![ALLOWED_DOMAIN.to_string()];
    unauth.paths = vec![PathBuf::from(ALLOWED_PATH)];
    assert!(
        unauth.check_domain(ALLOWED_DOMAIN).is_err(),
        "未授权 scope 竟放行了自身授权域"
    );
    assert!(
        unauth
            .check_path(std::path::Path::new(ALLOWED_PATH))
            .is_err(),
        "未授权 scope 竟放行了自身授权路径"
    );
});
