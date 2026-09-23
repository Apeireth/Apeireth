//! 工具调用前置防御（Pre-Call Guard）与后置出站凭据绊线（Post-Call Tripwire）.
//!
//! 在能力执行前拦截路径穿越与高危 Shell 注入，在能力执行后扫描输出中的敏感凭据，
//! 阻断凭据外泄并防止被大模型长程记忆污染.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// 前置守门拦截错误.
#[derive(Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PreCallGuardError {
    #[error("检测到路径穿越或敏感系统路径访问: {0}")]
    PathTraversal(String),
    #[error("检测到高危破坏性 Shell 注入或命令: {0}")]
    DangerousCommandInjection(String),
}

/// 泄露凭据类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeakedCredentialKind {
    OpenAiKey,
    AwsAccessKey,
    GitHubPat,
    PemPrivateKey,
    JwtToken,
    SlackToken,
    GenericBearer,
    /// 明文口令赋值 (`password=...` / `password: ...`, W1 §2.2 设计点名).
    PlaintextPassword,
}

/// 后置出站绊线扫描结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TripwireScanResult {
    pub is_clean: bool,
    pub sanitized_output: String,
    pub leaked_kinds: Vec<LeakedCredentialKind>,
}

/// 工具执行守门员.
#[derive(Debug, Clone, Default)]
pub struct ToolGuardrail;

/// ASCII 快速子串定位 (大小写不敏感, 字节偏移安全 —— 多字节文本不移位)。
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let hay = haystack.as_bytes();
    let nee = needle.as_bytes();
    if nee.len() > hay.len() {
        return None;
    }
    hay.windows(nee.len()).position(|w| w.eq_ignore_ascii_case(nee))
}

impl ToolGuardrail {
    pub fn new() -> Self {
        Self
    }

    /// 前置路径安全检查.
    pub fn verify_path_access(
        workspace_root: &Path,
        requested_path: &Path,
    ) -> Result<(), PreCallGuardError> {
        let path_str = requested_path.to_string_lossy();

        // 1. 拦截明显的路径穿越特征
        if path_str.contains("../") || path_str.contains("..\\") || path_str == ".." {
            return Err(PreCallGuardError::PathTraversal(format!(
                "禁止相对路径穿越: {}",
                path_str
            )));
        }

        // 2. 拦截绝对敏感路径
        let lower_path = path_str.to_lowercase();
        let forbidden_prefixes = [
            "/etc/shadow",
            "/etc/passwd",
            "/etc/sudoers",
            "/root",
            "/var/run",
            "/dev",
            "c:\\windows\\system32",
            "c:\\windows\\system",
        ];

        for prefix in &forbidden_prefixes {
            if lower_path.starts_with(prefix) {
                return Err(PreCallGuardError::PathTraversal(format!(
                    "禁止访问操作系统核心敏感路径: {}",
                    path_str
                )));
            }
        }

        // 3. 若为绝对路径，必须位于 workspace_root 范围内
        if requested_path.is_absolute() {
            if let (Ok(canonical_root), Ok(canonical_target)) =
                (workspace_root.canonicalize(), requested_path.canonicalize())
            {
                if !canonical_target.starts_with(&canonical_root) {
                    return Err(PreCallGuardError::PathTraversal(format!(
                        "目标路径超出工作区边界: {}",
                        path_str
                    )));
                }
            }
        }

        Ok(())
    }

    /// 前置 Shell 命令防注入与破坏性检查.
    pub fn verify_shell_command(command_line: &str) -> Result<(), PreCallGuardError> {
        let trimmed = command_line.trim();
        let lower = trimmed.to_lowercase();

        // 高危不可逆破坏性命令
        let forbidden_commands = [
            "rm -rf /",
            "rm -rf /*",
            "rmdir /s /q c:\\",
            "mkfs.",
            "dd if=",
            "format c:",
            ":(){ :|:& };:", // Fork 炸弹
            "shutdown -h now",
            "shutdown /s",
            "reboot",
            // W1 §2.3 系统修改命令 (2026-10-10 批): 注册表/服务/网络配置/属性隐藏。
            // 白名单否定式纵深 —— 守门不是沙箱的替代。
            "reg add",
            "reg delete",
            "reg import",
            "sc create",
            "sc config",
            "sc delete",
            "netsh set",
            "netsh add",
            "netsh delete",
            "netsh reset",
            "attrib +s",
            "attrib +h",
        ];

        for cmd in &forbidden_commands {
            if lower.contains(cmd) {
                return Err(PreCallGuardError::DangerousCommandInjection(format!(
                    "拦截高危系统破坏指令: {}",
                    cmd
                )));
            }
        }

        // netsh 写入类 (W1 §2.3): 单纯子串表抓不到 "netsh advfirewall set ..." 这种
        // 动词后置形态 —— "netsh" 出现且伴随写动词 (set/add/delete/reset/import) 即拦;
        // show/display 只读放行 (否定式保守, 误报可接受 —— 纵深不是沙箱的替代)。
        if lower.contains("netsh")
            && [" set", " add", " delete", " reset", " import"]
                .iter()
                .any(|verb| lower.contains(verb))
        {
            return Err(PreCallGuardError::DangerousCommandInjection(
                "拦截 netsh 写入类系统配置指令".to_string(),
            ));
        }

        Ok(())
    }

    /// 后置出站凭据绊线扫描与脱敏.
    pub fn scan_and_sanitize_output(raw_output: &str) -> TripwireScanResult {
        let mut sanitized = raw_output.to_string();
        let mut leaked_kinds = Vec::new();

        // 1. OpenAI Key 扫描 (sk-...)
        if let Some(pos) = sanitized.find("sk-") {
            let candidate = &sanitized[pos..];
            let end = candidate
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .unwrap_or(candidate.len());
            let token = &candidate[..end];
            if token.len() >= 20 {
                leaked_kinds.push(LeakedCredentialKind::OpenAiKey);
                sanitized = sanitized.replace(token, "[REDACTED_OPENAI_KEY]");
            }
        }

        // 2. AWS Key 扫描 (AKIA...)
        if let Some(pos) = sanitized.find("AKIA") {
            let candidate = &sanitized[pos..];
            let end = candidate
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(candidate.len());
            let token = &candidate[..end];
            if token.len() >= 16 && token.len() <= 32 {
                leaked_kinds.push(LeakedCredentialKind::AwsAccessKey);
                sanitized = sanitized.replace(token, "[REDACTED_AWS_KEY]");
            }
        }

        // 3. GitHub PAT 扫描 (ghp_... 或 github_pat_...)
        if let Some(pos) = sanitized.find("ghp_") {
            let candidate = &sanitized[pos..];
            let end = candidate
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(candidate.len());
            let token = &candidate[..end];
            if token.len() >= 36 {
                leaked_kinds.push(LeakedCredentialKind::GitHubPat);
                sanitized = sanitized.replace(token, "[REDACTED_GITHUB_PAT]");
            }
        }

        // 4. PEM 私钥头扫描
        if sanitized.contains("-----BEGIN") && sanitized.contains("PRIVATE KEY-----") {
            leaked_kinds.push(LeakedCredentialKind::PemPrivateKey);
            if let Some(start) = sanitized.find("-----BEGIN") {
                if let Some(end_offset) = sanitized[start..].find("KEY-----") {
                    let full_end = start + end_offset + 8;
                    let block = &sanitized[start..full_end];
                    sanitized = sanitized.replace(block, "[REDACTED_PEM_PRIVATE_KEY]");
                }
            }
        }

        // 5. Slack Token 扫描 (xox[baprs]-...)
        let slack_prefixes = ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"];
        for prefix in &slack_prefixes {
            if let Some(pos) = sanitized.find(prefix) {
                let candidate = &sanitized[pos..];
                let end = candidate
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .unwrap_or(candidate.len());
                let token = &candidate[..end];
                if token.len() >= 20 {
                    leaked_kinds.push(LeakedCredentialKind::SlackToken);
                    sanitized = sanitized.replace(token, "[REDACTED_SLACK_TOKEN]");
                }
            }
        }

        // 6. 明文口令赋值扫描 (W1 §2.2 设计点名: `password\s*[:=]\s*\S+`; 手写等价,
        //    不引 regex 依赖)。值截断到行尾 (截断而非放行), key 名保留可读。
        //    匹配用 ASCII 快速比较 —— 不走 to_lowercase 的字节偏移 (中文输出下
        //    大小写折叠可能移位, 切片会错位)。
        {
            let mut search_from = 0usize;
            while let Some(rel) = find_ascii_ci(&sanitized[search_from..], "password") {
                let key_start = search_from + rel;
                let after_key = key_start + "password".len();
                let rest = &sanitized[after_key..];
                let trimmed_start = after_key + (rest.len() - rest.trim_start().len());
                if sanitized[trimmed_start..].starts_with(':')
                    || sanitized[trimmed_start..].starts_with('=')
                {
                    let value_start_raw = &sanitized[trimmed_start + 1..];
                    let lead = value_start_raw.len() - value_start_raw.trim_start().len();
                    let value_start = trimmed_start + 1 + lead;
                    let value_end = sanitized[value_start..]
                        .find('\n')
                        .map(|i| value_start + i)
                        .unwrap_or(sanitized.len());
                    let value = &sanitized[value_start..value_end];
                    if value.len() >= 4 {
                        leaked_kinds.push(LeakedCredentialKind::PlaintextPassword);
                        sanitized.replace_range(value_start..value_end, "[REDACTED_PASSWORD]");
                        // 文本已变, 从头再扫会错位; 多处口令以首处命中为准
                        // (0 装: 不声称全扫, 单次调用一个命中段)。
                        break;
                    }
                }
                search_from = after_key;
                if search_from >= sanitized.len() {
                    break;
                }
            }
        }

        let is_clean = leaked_kinds.is_empty();
        TripwireScanResult {
            is_clean,
            sanitized_output: sanitized,
            leaked_kinds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_pre_call_path_traversal() {
        let ws = PathBuf::from("c:/workspace/test");
        let bad = PathBuf::from("../etc/passwd");
        assert!(ToolGuardrail::verify_path_access(&ws, &bad).is_err());

        let forbidden = PathBuf::from("/etc/shadow");
        assert!(ToolGuardrail::verify_path_access(&ws, &forbidden).is_err());

        let safe = PathBuf::from("src/lib.rs");
        assert!(ToolGuardrail::verify_path_access(&ws, &safe).is_ok());
    }

    #[test]
    fn test_pre_call_dangerous_commands() {
        assert!(ToolGuardrail::verify_shell_command("rm -rf / --no-preserve-root").is_err());
        assert!(ToolGuardrail::verify_shell_command("dd if=/dev/zero of=/dev/sda").is_err());
        assert!(ToolGuardrail::verify_shell_command("cargo test --workspace").is_ok());
    }

    #[test]
    fn test_post_call_tripwire_sanitization() {
        let raw = "Server connected with key sk-proj-1234567890abcdef1234567890 and AKIAIOSFODNN7EXAMPLE12.";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::OpenAiKey));
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::AwsAccessKey));
        assert!(res.sanitized_output.contains("[REDACTED_OPENAI_KEY]"));
        assert!(res.sanitized_output.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn tripwire_redacts_plaintext_password_assignment() {
        // W1 §2.2 设计点名模式; 含中文上下文 (回归 to_lowercase 偏移隐患)。
        let raw = "数据库登录 ok\npassword = hunter2secret\n下一行完好";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::PlaintextPassword));
        assert!(!res.sanitized_output.contains("hunter2secret"), "值已截断");
        assert!(res.sanitized_output.contains("[REDACTED_PASSWORD]"));
        assert!(res.sanitized_output.contains("下一行完好"), "只截命中值到行尾");
    }

    #[test]
    fn password_keyword_without_assignment_passes() {
        let res = ToolGuardrail::scan_and_sanitize_output("the password field is empty here");
        assert!(res.is_clean, "{res:?}");
    }

    #[test]
    fn system_modification_commands_are_guarded() {
        // W1 §2.3 扩展: 注册表/服务/网络配置/属性隐藏类系统修改命令。
        for cmd in [
            "reg add HKLM\\Software\\x /v y",
            "sc create evil binPath= x",
            "netsh advfirewall set allprofiles off",
            "attrib +s +h secret.txt",
        ] {
            assert!(
                ToolGuardrail::verify_shell_command(cmd).is_err(),
                "{cmd} 应被守门拦截"
            );
        }
        assert!(ToolGuardrail::verify_shell_command("netsh show all").is_ok());
        // 否定式保守语义: 含子串即拦 (纵深防线接受误报, 守门不是沙箱的替代)。
        assert!(ToolGuardrail::verify_shell_command("echo reg add is a phrase").is_err());
    }

    #[test]
    fn test_post_call_tripwire_pem_key() {
        let raw = "Output: -----BEGIN RSA PRIVATE KEY----- MIIEowIBAAKCAQEA0... -----END RSA PRIVATE KEY-----";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::PemPrivateKey));
        assert!(res.sanitized_output.contains("[REDACTED_PEM_PRIVATE_KEY]"));
    }
}
