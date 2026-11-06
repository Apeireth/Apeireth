//! fuzz: 网关的**安全判据**面（不是找 panic，是找「判据被绕过」）。
//!
//! 三个靶子：
//! - `FileFetcher::validate_path_safety` —— 路径穿越防御。**契约断言**：
//!   返回 `Ok` 时，输入必不含 `..` 与 `\0`（这是它自己文档承诺的口径）。
//! - `is_loopback_bind` —— 回环判定（fail-closed 设计）。**UTF-8 边界 panic 点**：
//!   它按字节剥方括号与 `:port`，多字节字符可能切在边界上。
//! - `panel_admin_face` / `domain_routed_surface` —— 管理面/路由判定，同属"判据被绕过"类。
#![no_main]

use std::path::PathBuf;

use apeireth_gateway::domain_context::domain_routed_surface;
use apeireth_gateway::file_fetcher::TransparentFileFetcher;
use apeireth_gateway::panels::panel_admin_face;
use apeireth_gateway::remote_access::is_loopback_bind;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // 1) 路径穿越防御：契约 = Ok ⇒ 不含 ".." 且不含 NUL
    let fetcher = TransparentFileFetcher::new("/tmp/fuzz-cache", vec![PathBuf::from("/allowed")]);
    if fetcher.validate_path_safety(s).is_ok() {
        assert!(
            !s.contains("..") && !s.contains('\0'),
            "validate_path_safety 放行了含 `..`/NUL 的路径: {s:?}"
        );
    }

    // 2) 回环判定：只要不 panic（剥方括号/端口的字节切片是风险点）
    let _ = is_loopback_bind(s);

    // 3) 管理面 / 路由判定：判据不得 panic
    let _ = panel_admin_face(s);
    let _ = domain_routed_surface(s);
});
