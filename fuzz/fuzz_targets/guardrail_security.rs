#![no_main]

//! fuzz: 工具守门的两道安全判定 —— 命令拦截 + 出站凭据绊线。
//!
//! 两个靶子（同一个安全模块，两道门都是「被绕过就出事」的类型）：
//!
//! 1. `ToolGuardrail::verify_shell_command` —— 高危命令拦截（**前置门**）
//!    **契约断言：空白归一化不得改变判定。**
//!    shell 的参数分隔符可以是**任意空白**（空格 / TAB / 换行 / 多空格）。
//!    若守门用 `contains(" set")` 这类**硬编码空格**的子串匹配，
//!    那么 `netsh<TAB>set ...` 就是一条合法命令却拦不住 —— 空白成了绕过通道。
//!    断言写法：把输入按空白归一化（多空白折叠成单空格）后再看「是否命中禁词」；
//!    归一化后命中的，**原输入也必须被拦**。
//!    这一条对 `forbidden_commands` 表里每个含空格的项同样成立
//!    （`rm -rf /`、`dd if=`、`format c:` …），一处断言覆盖整张表。
//!
//! 2. `ToolGuardrail::scan_and_sanitize_output` —— 出站凭据脱敏（**后置绊线**）
//!    **契约断言：绊线必须幂等。**
//!    脱敏后的输出再扫一遍，必须**干净**（`is_clean == true`、`leaked_kinds` 为空）。
//!    若某条脱敏只替换了凭证的一部分，残留片段会在二次扫描里再次被认出 ——
//!    那说明脱敏不彻底，明文碎片仍可能出境。
//!    （注：这里用幂等性是**恰当**的：该函数的语义就是「扫干净」，扫完不该再有；
//!      与 `strip_tags` 那种单次反转义语义不同 —— 见 docs/02-guides/fuzz-target-authoring.md §2.2）

use apeireth_tools_canonical::guardrail::ToolGuardrail;
use libfuzzer_sys::fuzz_target;

/// 命令守门表里所有**可被空白影响**的禁词（取自源码 `forbidden_commands`）。
/// 归一化断言只需要这些含空格的项；不含空白的项（`reboot`/`mkfs.`）归一化后不变。
const SPACE_SENSITIVE: [&str; 4] = ["rm -rf /", "dd if=", "format c:", "shutdown -h now"];

/// netsh 写入类的写动词（源码里的 `[" set", " add", " delete", " reset", " import"]`）。
const NETSH_WRITE_VERBS: [&str; 5] = [" set", " add", " delete", " reset", " import"];

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // ── 1. 命令守门：空白归一化不得改变判定 ──────────────────────────────
    let verdict = ToolGuardrail::verify_shell_command(s);

    // 归一化：多空白折叠成单空格（TAB / 换行 / 连续空格都视为同一分隔符）
    let lower = s.trim().to_lowercase();
    let normalized: String = lower.split_whitespace().collect::<Vec<_>>().join(" ");

    let hits_forbidden = SPACE_SENSITIVE.iter().any(|pat| normalized.contains(pat));
    let hits_netsh_write = normalized.contains("netsh")
        && NETSH_WRITE_VERBS.iter().any(|verb| normalized.contains(verb));

    if hits_forbidden || hits_netsh_write {
        assert!(
            verdict.is_err(),
            "空白绕过守门（归一化后命中禁词却放行）: in={s:?} normalized={normalized:?}"
        );
    }

    // ── 2. 出站凭据绊线：必须幂等 ────────────────────────────────────────
    let first = ToolGuardrail::scan_and_sanitize_output(s);
    let second = ToolGuardrail::scan_and_sanitize_output(&first.sanitized_output);
    assert!(
        second.leaked_kinds.is_empty(),
        "绊线不幂等：脱敏后二次扫描仍检出泄露 {:?}; in={s:?} after_first={:?}",
        second.leaked_kinds,
        first.sanitized_output
    );
});
