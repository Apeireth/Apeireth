//! 工具调用前置防御（Pre-Call Guard）与后置出站凭据绊线（Post-Call Tripwire）.
//!
//! 在能力执行前拦截路径穿越与高危 Shell 注入，在能力执行后扫描输出中的敏感凭据，
//! 阻断凭据外泄并防止被大模型长程记忆污染.

use std::path::Path;
use serde::{Deserialize, Serialize};
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

impl ToolGuardrail {
    pub fn new() -> Self {
        Self
    }

    /// 前置路径安全检查.
    pub fn verify_path_access(workspace_root: &Path, requested_path: &Path) -> Result<(), PreCallGuardError> {
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
            "/etc/shadow", "/etc/passwd", "/etc/sudoers",
            "/root", "/var/run", "/dev",
            "c:\\windows\\system32", "c:\\windows\\system",
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
            if let (Ok(canonical_root), Ok(canonical_target)) = (workspace_root.canonicalize(), requested_path.canonicalize()) {
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
            "rm -rf /", "rm -rf /*", "rmdir /s /q c:\\",
            "mkfs.", "dd if=", "format c:",
            ":(){ :|:& };:", // Fork 炸弹
            "shutdown -h now", "shutdown /s", "reboot",
        ];

        for cmd in &forbidden_commands {
            if lower.contains(cmd) {
                return Err(PreCallGuardError::DangerousCommandInjection(format!(
                    "拦截高危系统破坏指令: {}",
                    cmd
                )));
            }
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
            let end = candidate.find(|c: char| c.is_whitespace() || c == '"' || c == '\'').unwrap_or(candidate.len());
            let token = &candidate[..end];
            if token.len() >= 20 {
                leaked_kinds.push(LeakedCredentialKind::OpenAiKey);
                sanitized = sanitized.replace(token, "[REDACTED_OPENAI_KEY]");
            }
        }

        // 2. AWS Key 扫描 (AKIA...)
        if let Some(pos) = sanitized.find("AKIA") {
            let candidate = &sanitized[pos..];
            let end = candidate.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(candidate.len());
            let token = &candidate[..end];
            if token.len() >= 16 && token.len() <= 32 {
                leaked_kinds.push(LeakedCredentialKind::AwsAccessKey);
                sanitized = sanitized.replace(token, "[REDACTED_AWS_KEY]");
            }
        }

        // 3. GitHub PAT 扫描 (ghp_... 或 github_pat_...)
        if let Some(pos) = sanitized.find("ghp_") {
            let candidate = &sanitized[pos..];
            let end = candidate.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(candidate.len());
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
                let end = candidate.find(|c: char| c.is_whitespace() || c == '"' || c == '\'').unwrap_or(candidate.len());
                let token = &candidate[..end];
                if token.len() >= 20 {
                    leaked_kinds.push(LeakedCredentialKind::SlackToken);
                    sanitized = sanitized.replace(token, "[REDACTED_SLACK_TOKEN]");
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
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::AwsAccessKey));
        assert!(res.sanitized_output.contains("[REDACTED_OPENAI_KEY]"));
        assert!(res.sanitized_output.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn test_post_call_tripwire_pem_key() {
        let raw = "Output: -----BEGIN RSA PRIVATE KEY----- MIIEowIBAAKCAQEA0... -----END RSA PRIVATE KEY-----";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(!res.is_clean);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::PemPrivateKey));
        assert!(res.sanitized_output.contains("[REDACTED_PEM_PRIVATE_KEY]"));
    }
}
