//! 工具调用前置防御（Pre-Call Guard）与后置出站凭据绊线（Post-Call Tripwire）.
//!
//! 在能力执行前拦截路径穿越与高危 Shell 注入，在能力执行后扫描输出中的敏感凭据，
//! 阻断凭据外泄并防止被大模型长程记忆污染.
//! 出境脱敏双向闸接线面 [`ToolGuardrail::sweep_output_for_egress`]: 先占位族
//! 出境脱敏 (可还原), 再既有凭据绊线 (单向标签), 两族共存各零回归.

use crate::redaction_bridge::{EgressRedaction, RedactionBridge};
use apeireth_governance::input_security::RedactionPlaceholder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// 前置守门拦截错误.
#[derive(Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PreCallGuardError {
    #[error("检测到路径穿越或敏感系统路径访问: {0}")]
    PathTraversal(String),
    /// 目标路径落在工作区目录之外 —— 目录边界拒绝. 与词法攻击面/受保护
    /// 路径不同, 这类拒绝可经一次性升级 (见 `escalation` 模块) 就地引导.
    #[error("{0}")]
    OutsideWorkspace(String),
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

/// 出境双层扫结果: 占位族脱敏 (登记入簿, 受控回程可还原) + 既有凭据绊线
/// (PEM/口令赋值等形状, 单向标签) —— 两族在同一份输出上共存.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressSweepResult {
    pub redaction: EgressRedaction,
    pub tripwire: TripwireScanResult,
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
    hay.windows(nee.len())
        .position(|w| w.eq_ignore_ascii_case(nee))
}

/// base64url 字母表判定 (JWT 三段共用).
fn is_base64url_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// opaque Bearer 令牌值可出现的字节 (base64 / base64url / URL-safe 可见字符).
fn is_opaque_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'~' | b'+' | b'/' | b'=' | b'-')
}

/// 定位首个 JWT 形状令牌, 返回 `(start, end)` 字节区间.
///
/// 判据 (手写, 不引 regex 依赖): 以 `eyJ` 起头 (base64url 编码的 JSON 对象头,
/// 首字节 `{` + ASCII 字母必然落在这一前缀上) —— 其后为恰好三段 `.` 分隔的
/// base64url 段: 头段 ≥8、载荷段 ≥4、签名字段 ≥8 (或为空, 即 `alg=none` 的
/// 未签名 `h.p.` 形态), 整体 ≥20 字节. 任一条件不满足即不当凭据 (保守: 宁可
/// 漏判也不误伤长段 base64 文本)。
fn find_jwt_span(text: &str) -> Option<(usize, usize)> {
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("eyJ") {
        let start = from + rel;
        let mut seg_lens = [0usize; 3];
        let mut seg = 0usize;
        let mut i = start;
        while i < hay.len() {
            let b = hay[i];
            if is_base64url_byte(b) {
                seg_lens[seg] += 1;
                i += 1;
            } else if b == b'.' && seg < 2 {
                seg += 1;
                i += 1;
            } else {
                break;
            }
        }
        if seg == 2
            && seg_lens[0] >= 8
            && seg_lens[1] >= 4
            && (seg_lens[2] == 0 || seg_lens[2] >= 8)
            && (i - start) >= 20
        {
            return Some((start, i));
        }
        from = start + 3;
        if from >= text.len() {
            break;
        }
    }
    None
}

/// 定位首个 `Authorization: Bearer <opaque>` 形状的值区间, 返回值 `(start, end)`.
///
/// 只认显式头语法: 回看窗口内须有 `authorization` 关键字, 其后 `bearer` + 空白 +
/// 单段 opaque 值 (16..=4096 字节、无空白)。短值 / 散文里的 "Bearer" 一词 /
/// 不含 Authorization 上下文的短语都不命中 (保守: 漏判可接受, 误伤正常文本不可
/// 接受 —— 这一族形状本身模糊, 宁窄勿宽)。
fn find_bearer_span(text: &str) -> Option<(usize, usize)> {
    let hay = text.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = find_ascii_ci(&text[from..], "bearer") {
        let kw_start = from + rel;
        let after_kw = kw_start + "bearer".len();
        // 回看窗口: 头部语法上下文 (`Authorization` 与之相隔通常 < 32 字节)。
        let window_start = kw_start.saturating_sub(64);
        let context_ok = find_ascii_ci(&text[window_start..kw_start], "authorization").is_some();
        // 关键字前不得紧贴字母数字 (避免词内命中)。
        let boundary_ok = kw_start == 0 || !hay[kw_start - 1].is_ascii_alphanumeric();
        let mut i = after_kw;
        let mut saw_space = false;
        while i < hay.len() && (hay[i] == b' ' || hay[i] == b'\t') {
            saw_space = true;
            i += 1;
        }
        if boundary_ok && context_ok && saw_space {
            let value_start = i;
            while i < hay.len() && is_opaque_token_byte(hay[i]) {
                i += 1;
            }
            let value_len = i - value_start;
            if (16..=4096).contains(&value_len) {
                return Some((value_start, i));
            }
        }
        from = after_kw;
        if from >= text.len() {
            break;
        }
    }
    None
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
                    return Err(PreCallGuardError::OutsideWorkspace(format!(
                        "目标路径超出工作区边界: {}",
                        path_str
                    )));
                }
            }
        }

        Ok(())
    }

    /// 前置 Shell 命令防注入与破坏性检查.
    ///
    /// 删除族同权口径 (本层不改判, 理由在此写明): 破坏面是**窄词面灾难清单**
    /// (灾难形态字面量 + 系统配置写动词), 不做删除族全量拦截 —— 普通删除族
    /// (`del` / `Remove-Item` / `[System.IO.File]::Delete` 等静态调用面) 一律
    /// 同档放行, 经写意图总闸 + shell 审批链统一兜住; 两条删除 API 路径在本层
    /// 判定逐一相同 (同为放行档), 对称性由 tests/delete_path_parity.rs 锁定。
    /// 灾难字面量不扩面到 cmdlet / 静态调用形态的理由: 词面清单无法可靠判定
    /// 灾难作用域, 单边扩面会造成两条删除路径的审批档判定不一致 —— 恰是删除
    /// 同权要消除的权力差; 守门不是沙箱的替代, 灾难级作用域由总闸 + 人工审批
    /// + 沙箱三层兜住。
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

        // 4. PEM 私钥块扫描 (头部 → END 行整块换标签)。只换头部会把 base64
        //    私钥体留在输出里 —— 确证级泄漏债, 整块收口; 无 END 行的截断块
        //    退化为头部替换 (0 装: 不声称截断块全扫)。
        if sanitized.contains("-----BEGIN") && sanitized.contains("PRIVATE KEY-----") {
            leaked_kinds.push(LeakedCredentialKind::PemPrivateKey);
            if let Some(start) = sanitized.find("-----BEGIN") {
                let end_marker = sanitized[start..]
                    .find("-----END")
                    .map(|offset| start + offset);
                let block_end = end_marker.and_then(|end_marker| {
                    sanitized[end_marker..]
                        .find("KEY-----")
                        .map(|offset| end_marker + offset + "KEY-----".len())
                });
                let fallback_end = sanitized[start..]
                    .find("KEY-----")
                    .map(|offset| start + offset + "KEY-----".len());
                if let Some(full_end) = block_end.or(fallback_end) {
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

        // 6. JWT 扫描 (header.payload.signature). 此前 `JwtToken` 变体只有声明、
        //    没有检测面 —— 命名即承诺的空槽, 本批补齐 (D18).
        if let Some((start, end)) = find_jwt_span(&sanitized) {
            leaked_kinds.push(LeakedCredentialKind::JwtToken);
            sanitized.replace_range(start..end, "[REDACTED_JWT]");
        }

        // 7. 通用 Bearer 令牌扫描 (`Bearer <opaque>`). 与 6 同属「声明了类别但没接
        //    检测」的空槽; 6 先收口后本步只会看到 `[REDACTED_JWT]` 标签 (含 `[`,
        //    不在 opaque 字母表内), 两族互不吞并。
        if let Some((start, end)) = find_bearer_span(&sanitized) {
            leaked_kinds.push(LeakedCredentialKind::GenericBearer);
            sanitized.replace_range(start..end, "[REDACTED_BEARER]");
        }

        // 8. 明文口令赋值扫描 (W1 §2.2 设计点名: `password\s*[:=]\s*\S+`; 手写等价,
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
                    // 占位符族豁免: 受控占位符不是明文口令 —— 单向截断会毁掉
                    // 受控回程的还原锚点。替换面只认占位符族, 其余值一律照旧截断。
                    if value.len() >= 4 && RedactionPlaceholder::parse(value.trim()).is_none() {
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

    /// 出境双层扫 (工具输出进入会话历史/出境面之前的脱敏收口).
    ///
    /// 顺序固定: ① 出境占位族脱敏 (真值 → `__AP_<类>_<指纹12>__`, 登记入簿,
    /// 受控回程可还原) ② 既有凭据绊线 (PEM/口令赋值等未被 PII 检测器覆盖的
    /// 凭据形状, 单向标签)。两族共存: 占位族在受控回程还原, 单向标签永远
    /// 保持脱敏。
    pub fn sweep_output_for_egress(
        raw_output: &str,
        bridge: &mut RedactionBridge,
    ) -> EgressSweepResult {
        let redaction = bridge.redact_for_egress(raw_output);
        let tripwire = Self::scan_and_sanitize_output(&redaction.text);
        EgressSweepResult {
            redaction,
            tripwire,
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
        assert!(
            res.sanitized_output.contains("下一行完好"),
            "只截命中值到行尾"
        );
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

    /// PEM 整块收口: base64 私钥体不得留在输出 (只换头部 = 泄漏债, 已清)。
    #[test]
    fn pem_block_redaction_covers_key_body_not_just_header() {
        let raw = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0\n-----END RSA PRIVATE KEY-----\ntrailing";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(
            !res.sanitized_output.contains("MIIEowIBAAKCAQEA0"),
            "{}",
            res.sanitized_output
        );
        assert!(
            res.sanitized_output.contains("trailing"),
            "{}",
            res.sanitized_output
        );
        assert!(res.sanitized_output.contains("[REDACTED_PEM_PRIVATE_KEY]"));
    }

    /// 占位符族豁免: 受控占位符不是明文口令 (还原锚点不可被单向截断毁掉),
    /// 真口令照旧截断 —— 两头都钉。
    #[test]
    fn password_scan_keeps_placeholder_family_and_still_redacts_real_passwords() {
        let token = "__AP_ENV_SECRET_a1b2c3d4e5f6__";
        let res = ToolGuardrail::scan_and_sanitize_output(&format!("password = {token}\nok"));
        assert!(res.is_clean, "{res:?}");
        assert!(
            res.sanitized_output.contains(token),
            "{}",
            res.sanitized_output
        );

        let res2 = ToolGuardrail::scan_and_sanitize_output("password = realsecret\nok");
        assert!(!res2.is_clean);
        assert!(res2.sanitized_output.contains("[REDACTED_PASSWORD]"));
        assert!(!res2.sanitized_output.contains("realsecret"));
    }

    /// D18 收口: `JwtToken` 变体此前只有声明、零检测面 (命名即承诺的空槽)。
    #[test]
    fn jwt_token_is_detected_and_redacted() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                   eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ.\
                   SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let raw = format!("上游返回: Authorization: Bearer {jwt} (之后正常)");
        let res = ToolGuardrail::scan_and_sanitize_output(&raw);
        assert!(!res.is_clean);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(res.sanitized_output.contains("[REDACTED_JWT]"));
        assert!(!res.sanitized_output.contains("eyJ"));
        assert!(
            res.sanitized_output.contains("之后正常"),
            "非命中文段原样保留: {}",
            res.sanitized_output
        );
    }

    /// 形状不完整 (两段 / 签名过短) 或只是长 base64 文本 —— 不当 JWT。
    #[test]
    fn jwt_shape_requires_three_segments_and_long_enough_signature() {
        for raw in [
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ4In0", // 只有两段
            "eyJhbGciOiJIUzI1NiJ9.abc.def",         // 段长不足
            "prefix eyJhbGciOiJIUzI1NiJ9 suffix",   // 单词形
            "no eyJ marker here",                   // 无前缀
        ] {
            let res = ToolGuardrail::scan_and_sanitize_output(raw);
            assert!(res.is_clean, "{raw} 不该被当 JWT: {res:?}");
        }
    }

    /// 未签名 `h.p.` 形态 (alg=none, 尾段为空) 也算 JWT 形状。
    #[test]
    fn unsigned_jwt_with_trailing_dot_is_detected() {
        let raw = "token=eyJhbGciOiJub25lIn0.eyJzdWIiOiJ4In0.";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(res.sanitized_output.contains("[REDACTED_JWT]"));
    }

    /// `GenericBearer` 同为空槽, 本批补齐: 只认 Authorization 头部上下文。
    #[test]
    fn authorization_bearer_token_is_detected_and_redacted() {
        let raw = "request headers: Authorization: Bearer opaqueTokenValue1234567890abc end";
        let res = ToolGuardrail::scan_and_sanitize_output(raw);
        assert!(res
            .leaked_kinds
            .contains(&LeakedCredentialKind::GenericBearer));
        assert!(res.sanitized_output.contains("[REDACTED_BEARER]"));
        assert!(!res
            .sanitized_output
            .contains("opaqueTokenValue1234567890abc"));
        assert!(res.sanitized_output.contains("end"));
    }

    /// 散文里的 "Bearer" / 无 Authorization 上下文 / 短值 —— 一律不误伤。
    #[test]
    fn bearer_word_without_header_context_is_not_flagged() {
        for raw in [
            "The bearer configuration mechanism is documented elsewhere.",
            "Bearer opaqueTokenValue1234567890abc",
            "Authorization: Bearer short",
            "authorization header explained",
        ] {
            let res = ToolGuardrail::scan_and_sanitize_output(raw);
            assert!(res.is_clean, "{raw} 不该被当 Bearer: {res:?}");
        }
    }

    /// 两族互不吞并: JWT 先收口, Bearer 步只看到 `[REDACTED_JWT]` 标签 (含 `[`
    /// 不在 opaque 字母表), 不重复计类。
    #[test]
    fn jwt_and_bearer_kinds_do_not_double_flag() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                   eyJzdWIiOiIxMjM0NTY3ODkwIn0.\
                   SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let raw = format!("Authorization: Bearer {jwt}");
        let res = ToolGuardrail::scan_and_sanitize_output(&raw);
        assert!(res.leaked_kinds.contains(&LeakedCredentialKind::JwtToken));
        assert!(
            !res.leaked_kinds
                .contains(&LeakedCredentialKind::GenericBearer),
            "{res:?}"
        );
        assert!(!res.sanitized_output.contains("[REDACTED_BEARER]"));
    }
}
