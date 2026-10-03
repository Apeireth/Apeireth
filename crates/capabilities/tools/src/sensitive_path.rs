//! Shared workspace path policy for local read tools.
//!
//! This is intentionally narrower than a blanket dotfile ban. Ordinary
//! project metadata such as `.gitignore` and `.cargo/config.toml` remains
//! readable, while common environment files, key material, credential stores,
//! and private-key directories are kept out of filesystem and search results.
//!
//! 敏感面边界 (fail-closed, 自省通道批):
//! - **路径级拒绝**: 凭据存储件 (`creds.json`)、钥匙串导出物、`.env` 族、
//!   密钥材料 —— 读取类工具一律拒绝, 拒绝信息即帧 (pre_deny 语义)。
//! - **字段级脱敏**: 配置文件**可以读**, 但 key/token/secret 等密钥字段的
//!   **字段值**脱敏为 `[redacted]` (与启动日志脱敏同一语义) —— 非敏感字段
//!   照读, 零回归。

use std::path::Path;

use crate::mcp_bridge::config::is_secret_key;

/// Whether `path` contains a known sensitive workspace path.
///
/// `root` and `path` should be canonical paths when available. The helper also
/// works with a lexical child path, which lets callers protect a sensitive
/// symlink name before following its target.
pub(crate) fn is_sensitive_path(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let components: Vec<String> = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(|component| component.to_ascii_lowercase())
        .collect();

    for (index, component) in components.iter().enumerate() {
        if is_sensitive_directory(component)
            || is_sensitive_file_name(component)
            || (component == ".config"
                && components
                    .get(index + 1)
                    .is_some_and(|next| next == "gcloud"))
        {
            return true;
        }
    }

    false
}

fn is_sensitive_directory(name: &str) -> bool {
    // L 组 (2026-09-24 审计): .kube/config (集群凭据) 与 .docker/config.json
    // (registry auth) 补列 —— 常见密钥文件不在表的缺口。
    matches!(
        name,
        ".ssh" | ".aws" | ".gnupg" | ".secret" | ".secrets" | ".kube" | ".docker"
    )
}

fn is_sensitive_file_name(name: &str) -> bool {
    if name == ".env" || name.starts_with(".env.") {
        return true;
    }

    if name == "id_rsa"
        || name.starts_with("id_rsa.")
        || name == "id_ed25519"
        || name.starts_with("id_ed25519.")
    {
        return true;
    }

    if name == "credentials"
        || name.starts_with("credentials.")
        || name == "secret"
        || name == "secrets"
        || name.starts_with("secret.")
        || name.starts_with("secrets.")
    {
        return true;
    }

    // L 组 (2026-09-24 审计): npm registry token (.npmrc 的 authToken) 与
    // 常见裸 token 文件补列。输出侧 tripwire 是兜底, 这里先把它们挡在只读
    // 工具视野之外。
    if name == ".npmrc" || name == "token.txt" {
        return true;
    }

    // Git/curl 凭据存储与 apikey 文件 — 2026-10-06 真机: 工作区根=用户主目录
    // 时, 模型亲眼见到 .git-credentials/_netrc/apikey-ultra.txt/GeminiApiKey.txt
    // 躺在工作区里. "apikey" 用 contains 而非前缀, 覆盖 GeminiApiKey.txt 这类
    // 品牌前缀变体; .git-credentials 用后缀匹配覆盖 Users31683.git-credentials.
    if name.ends_with(".git-credentials")
        || name == ".netrc"
        || name == "_netrc"
        || name.contains("apikey")
    {
        return true;
    }

    // 产品自身的网关/会话数据库 — 内容敏感, 从只读工具视野中屏蔽.
    if name == "apeireth_gateway.db" || name == "apeireth_v2.db" || name == "apeireth.db" {
        return true;
    }

    // 凭据存储面 (自省通道批): `<data>/creds.json` (文件凭据存储落盘件) 与
    // 钥匙串导出物 (导出转储 / 导出后端数据件) —— 都是凭据本体, 只读工具
    // 一律拒绝 (fail-closed)。`apeireth-keyring.master.key` 已被下方 `.key`
    // 扩展名规则覆盖, 此处补数据件与通用导出物。
    if name == "creds.json"
        || name.starts_with("creds.json.")
        || name.ends_with("-keyring.bin")
        || name.ends_with(".keyring")
        || name.ends_with(".keychain")
        || name.starts_with("keychain-export")
    {
        return true;
    }

    if ["pem", "key", "p12", "pfx", "jks", "kdbx"]
        .iter()
        .any(|extension| name.ends_with(&format!(".{extension}")))
    {
        return true;
    }

    ["private-key", "private_key", "privatekey"]
        .iter()
        .any(|marker| name.contains(marker))
}

/// 拒绝信息即帧: 凭据/密钥面读取拒绝的稳定帧文本 (`pipeline.pre_deny` 语义)。
///
/// 帧 = 完整、关联、可分类的失败描述 (不是空洞): 稳定 code + 来源 + 事实
/// 原因「凭据面不可读（安全契约）」。文件读取类工具对凭据面一律用它回话。
pub(crate) fn credential_surface_refusal() -> String {
    let failure = crate::exec_pipeline::PipelineFailure::PreDenied {
        source: "credential_surface_guard".to_string(),
        reason: "requested path is protected: credential surface is unreadable (security contract); 凭据面不可读（安全契约）".to_string(),
    };
    format!("{}: {}", failure.code(), failure.message())
}

/// 敏感字段值脱敏标记 (与启动日志脱敏同一语义)。
pub(crate) const REDACTED: &str = "[redacted]";

/// 对结构化文本做**字段值**脱敏: key/token/secret 等密钥字段的值替换为
/// [`REDACTED`], 字段名与非敏感字段逐字节保留。
///
/// 匹配口径与启动日志脱敏同一语义 ([`is_secret_key`]): `key` / `"key"` /
/// `dotted.name` 形态的字段名后跟 `:` 或 `=` 且存在非空值即脱敏 (覆盖
/// JSON / YAML / TOML / env 等常见配置行, 含单行内联 JSON); 无字段形态的
/// 行 (正文 / 代码语句 / 纯列表项) 原样返回; 容器值 (`{` / `[`) 不动, 其
/// 内层字段由各自的键值对单独裁决。
pub(crate) fn redact_secret_field_values(content: &str) -> String {
    content
        .split_inclusive('\n')
        .map(redact_line)
        .collect::<String>()
}

/// 一行内所有密钥字段值的脱敏 (字节区间编辑, 从右往左应用)。
fn redact_line(line: &str) -> String {
    let (body, newline) = match line.strip_suffix('\n') {
        Some(rest) => (rest.strip_suffix('\r').unwrap_or(rest), true),
        None => (line, false),
    };

    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut covered_until = 0usize;
    for (separator_at, separator) in body.match_indices([':', '=']) {
        if separator_at < covered_until {
            continue;
        }
        let Some((key_start, key)) = field_key_before(body, separator_at) else {
            continue;
        };
        if key.is_empty() || !is_secret_key(key) {
            continue;
        }
        let Some((value_start, value_end, replacement)) =
            secret_value_after(body, separator_at + separator.len())
        else {
            continue;
        };
        let Some(before) = body.get(..key_start) else {
            continue;
        };
        let boundary = before.chars().next_back();
        let boundary_ok = match boundary {
            None => true,
            Some(c) => c.is_whitespace() || matches!(c, '{' | '[' | ','),
        };
        if !boundary_ok {
            continue;
        }
        edits.push((value_start..value_end, replacement));
        covered_until = value_end;
    }

    if edits.is_empty() {
        return line.to_string();
    }
    let mut out = body.to_string();
    for (range, replacement) in edits.into_iter().rev() {
        out.replace_range(range, &replacement);
    }
    if newline {
        out.push('\n');
    }
    out
}

/// 分隔符之前的字段名: 引号键或裸标识符键, 返回 (键名起始字节, 键名)。
fn field_key_before(body: &str, separator_at: usize) -> Option<(usize, &str)> {
    let before = body.get(..separator_at)?;
    let key_end = before.trim_end_matches([' ', '\t']).len();
    if key_end == 0 {
        return None;
    }
    let key_region = &before[..key_end];
    let last = key_region.as_bytes()[key_end - 1];
    if last == b'"' || last == b'\'' {
        let quote = last as char;
        let inner = &key_region[..key_end - 1];
        let open = inner.rfind(quote)?;
        Some((open, &inner[open + 1..]))
    } else {
        match key_region
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
        {
            Some(at) => {
                let start = at
                    + key_region[at..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(0);
                Some((start, &key_region[start..]))
            }
            None => Some((0, key_region)),
        }
    }
}

/// 分隔符之后的值区间与替换文本; 容器值/空值返回 `None`。
fn secret_value_after(body: &str, from: usize) -> Option<(usize, usize, String)> {
    let rest = body.get(from..)?;
    let lead = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let value_start = from + lead;
    let value = body.get(value_start..)?;
    let first = value.chars().next()?;
    if first == '{' || first == '[' {
        return None;
    }
    if first == '"' || first == '\'' {
        let close = value[1..].find(first).map(|at| at + 1 + first.len_utf8());
        let end = close.unwrap_or(value.len());
        return Some((
            value_start,
            value_start + end,
            format!("{first}{REDACTED}{first}"),
        ));
    }
    let cut = [" #", " //", "\t#", "\t//", ",", "}", "]"]
        .iter()
        .filter_map(|marker| value.find(marker))
        .min()
        .unwrap_or(value.len());
    if cut == 0 {
        return None;
    }
    Some((value_start, value_start + cut, REDACTED.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn protects_known_sensitive_files_and_directories() {
        let root = Path::new("workspace");
        for path in [
            ".env",
            ".env.local",
            "foo.pem",
            "foo.key",
            "foo.p12",
            "foo.pfx",
            "id_rsa",
            "id_rsa.backup",
            "id_ed25519",
            "id_ed25519.pub",
            "credentials",
            "credentials.json",
            ".git-credentials",
            ".netrc",
            "_netrc",
            ".npmrc",
            "token.txt",
            "Users31683.git-credentials",
            "GeminiApiKey.txt",
            "apikey-ultra.txt",
            "apikey.txt",
            "apeireth_gateway.db",
            "apeireth_v2.db",
            "secret",
            "secrets.production",
            ".ssh/config",
            ".aws/credentials",
            ".kube/config",
            ".docker/config.json",
            ".config/gcloud/application_default_credentials.json",
        ] {
            assert!(
                is_sensitive_path(root, &root.join(path)),
                "expected protected path: {path}"
            );
        }
    }

    #[test]
    fn does_not_block_normal_project_dotfiles() {
        let root = Path::new("workspace");
        for path in [
            ".gitignore",
            ".cargo/config.toml",
            "README.md",
            "Cargo.toml",
            "src/lib.rs",
        ] {
            assert!(
                !is_sensitive_path(root, &PathBuf::from(root).join(path)),
                "unexpected protected path: {path}"
            );
        }
    }

    #[test]
    fn protects_credential_store_and_keychain_export_artifacts() {
        let root = Path::new("workspace");
        for path in [
            "creds.json",
            "creds.json.bak",
            "data/creds.json",
            "apeireth-keyring.bin",
            "backup.keyring",
            "dump.keychain",
            "keychain-export.json",
            "keychain-export-2026.txt",
            "apeireth-keyring.master.key",
        ] {
            assert!(
                is_sensitive_path(root, &root.join(path)),
                "expected protected path: {path}"
            );
        }
    }

    #[test]
    fn credential_surface_refusal_is_a_pre_deny_frame() {
        let frame = credential_surface_refusal();
        assert!(frame.contains("pipeline.pre_deny"), "{frame}");
        assert!(frame.contains("protected"), "{frame}");
        assert!(
            frame.contains("凭据面不可读（安全契约）"),
            "拒绝信息必须说明安全契约: {frame}"
        );
    }

    #[test]
    fn redacts_secret_field_values_across_config_shapes() {
        for (input, expected) in [
            ("api_key = \"sk-123\"", "api_key = \"[redacted]\""),
            ("token=abc123", "token=[redacted]"),
            ("password: hunter2", "password: [redacted]"),
            (
                "  \"secret\": \"top-secret\",",
                "  \"secret\": \"[redacted]\",",
            ),
            ("db.api_key = \"x\"", "db.api_key = \"[redacted]\""),
            ("KEY=sk-1 # comment", "KEY=[redacted] # comment"),
            (
                "{\"inline\": 1, \"api_key\": \"sk-9\"}",
                "{\"inline\": 1, \"api_key\": \"[redacted]\"}",
            ),
        ] {
            let actual = redact_secret_field_values(input);
            assert_eq!(actual, expected, "input: {input}");
        }
    }

    #[test]
    fn redaction_leaves_non_sensitive_fields_and_prose_untouched() {
        for input in [
            "name = \"apeireth\"",
            "timeout: 30",
            "the monkey in the tree",
            "[section.header]",
            "- plain list item",
            "",
            "counts: {\"sessions\": 3}",
        ] {
            assert_eq!(
                redact_secret_field_values(input),
                input,
                "must be byte-identical: {input}"
            );
        }
    }

    #[test]
    fn redaction_never_echoes_the_original_secret_value() {
        let content = "api_key = \"sk-live-super-secret\"\nname = \"keep\"\n";
        let redacted = redact_secret_field_values(content);
        assert!(!redacted.contains("sk-live-super-secret"), "{redacted}");
        assert!(redacted.contains("api_key = \"[redacted]\""), "{redacted}");
        assert!(redacted.contains("name = \"keep\""), "{redacted}");
    }
}
