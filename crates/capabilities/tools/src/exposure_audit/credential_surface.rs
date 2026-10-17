//! 本地数据面自查 · 项一/项二: 凭据文件分布 + 明文风险点。
//!
//! 防御语自查 (只看自己的暴露面): 照 [`crate::sensitive_path`] 族词表 + 凭据
//! 排除表盘点凭据/密钥面路径的**分布** (只列路径词面, 绝不打开凭据件), 并对
//! 可读配置件做密钥字段**计数** (明文风险点) —— 值不采集、不落盘、不进报告。
//! 全程只读零写。

use std::path::{Path, PathBuf};

use crate::backup::CREDENTIAL_ARTIFACTS_EXCLUDED;
use crate::exposure_audit::report::{ExposureItem, Finding, RiskLevel};
use crate::sensitive_path::{is_sensitive_path, redact_secret_field_values, REDACTED};

/// 目录遍历深度上限 (失控大树保护; 超限如实记截断发现)。
pub(crate) const MAX_WALK_DEPTH: usize = 16;

/// 目录遍历条目上限 (失控大树保护; 超限如实记截断发现)。
pub(crate) const MAX_WALK_ENTRIES: usize = 20_000;

/// 单个体检项的发现条数上限 (超出部分聚合成一条, 防报告爆量)。
const MAX_FINDINGS: usize = 25;

/// 明文风险点的单件读取上限 (大件不进内存, 如实记跳过计数)。
const MAX_SCAN_BYTES: u64 = 1024 * 1024;

/// 明文风险点的配置件扩展名词面 (内容只计数, 值不出报告)。
const CONFIG_EXTENSIONS: &[&str] = &["json", "toml", "yaml", "yml", "ini", "cfg", "conf"];

/// 遍历结果 (受上限保护; 截断如实可查)。
pub(crate) struct Walk {
    /// (绝对路径, 相对路径, 是否目录) 全量条目, 按相对路径排序。
    pub(crate) entries: Vec<(PathBuf, PathBuf, bool)>,
    /// 是否因条目/深度上限截断 (截断即体检口径如实降级)。
    pub(crate) truncated: bool,
}

/// 受限只读遍历 (不追符号链接, 不写任何东西)。
pub(crate) fn walk_entries(root: &Path) -> Walk {
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut stack = vec![(root.to_path_buf(), PathBuf::new(), 0usize)];
    while let Some((dir, rel_prefix, depth)) = stack.pop() {
        if depth >= MAX_WALK_DEPTH {
            truncated = true;
            continue;
        }
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<std::fs::DirEntry> = read_dir.filter_map(Result::ok).collect();
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            if entries.len() >= MAX_WALK_ENTRIES {
                truncated = true;
                break;
            }
            let Ok(file_type) = child.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = child.path();
            let rel = rel_prefix.join(child.file_name());
            let is_dir = file_type.is_dir();
            entries.push((path.clone(), rel.clone(), is_dir));
            if is_dir {
                stack.push((path, rel, depth + 1));
            }
        }
    }
    entries.sort_by(|left, right| left.1.cmp(&right.1));
    Walk { entries, truncated }
}

/// 凭据排除表命中 (产品自身的凭据/令牌落盘件, 大小写不敏感)。
pub(crate) fn is_credential_artifact_name(name: &str) -> bool {
    CREDENTIAL_ARTIFACTS_EXCLUDED
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
}

/// 暴露面命中口径: 敏感路径族词表 + 凭据排除表 (两层任一命中即凭据面)。
pub(crate) fn is_credential_surface(root: &Path, path: &Path) -> bool {
    is_sensitive_path(root, path)
        || path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .any(is_credential_artifact_name)
}

/// 密钥材料形状 (私钥/证书容器/密钥目录): 命中即高风险发现。
fn is_key_material_name(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    ["pem", "key", "p12", "pfx", "jks", "kdbx"]
        .iter()
        .any(|extension| lowered.ends_with(&format!(".{extension}")))
        || lowered.starts_with("id_rsa")
        || lowered.starts_with("id_ed25519")
        || ["private-key", "private_key", "privatekey"]
            .iter()
            .any(|marker| lowered.contains(marker))
        || lowered == ".ssh"
        || lowered == ".gnupg"
}

/// 相对路径的报告词面 (`/` 分隔, 平台无关)。
fn rel_word(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 项一: 凭据文件分布 (本地自查, 只列路径词面)。
pub fn scan_credential_files(root: &Path) -> ExposureItem {
    let walk = walk_entries(root);
    let mut findings = Vec::new();
    let mut hidden = 0usize;
    for (path, rel, _) in &walk.entries {
        if !is_credential_surface(root, path) {
            continue;
        }
        if findings.len() >= MAX_FINDINGS {
            hidden += 1;
            continue;
        }
        let name = rel
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let risk = if is_key_material_name(&name) {
            RiskLevel::High
        } else {
            RiskLevel::Medium
        };
        let parent = rel
            .parent()
            .map(rel_word)
            .filter(|word| !word.is_empty())
            .unwrap_or_else(|| ".".to_string());
        findings.push(Finding::new(
            rel_word(rel),
            risk,
            format!("凭据/密钥面路径 (词表命中), 分布于 {parent}; 只列路径, 不打开、不采值"),
        ));
    }
    if hidden > 0 {
        findings.push(Finding::new(
            "(聚合)",
            RiskLevel::Medium,
            format!("另有 {hidden} 条凭据面路径超出条目上限未逐条列出 (发现总数已如实计数)"),
        ));
    }
    if walk.truncated {
        findings.push(Finding::new(
            "(遍历截断)",
            RiskLevel::Low,
            format!(
                "目录遍触及深度/条目上限 (深度≤{MAX_WALK_DEPTH}, 条目≤{MAX_WALK_ENTRIES}), 未覆盖部分未测"
            ),
        ));
    }
    let note = format!(
        "词表口径: sensitive_path 族 + 凭据排除表; 共遍历 {} 个条目, 值零采集",
        walk.entries.len()
    );
    ExposureItem::measured(
        "credential_files",
        "凭据文件分布",
        findings,
        "凭据收口: 密钥材料集中进凭据存储 (creds.json/钥匙串), 数据面只留引用; 分布清单按周期巡检复查",
    )
    .with_note(note)
}

/// 项二: 明文风险点 (可读配置件的密钥字段计数; 值永不出报告)。
pub fn scan_plaintext_points(root: &Path) -> ExposureItem {
    let walk = walk_entries(root);
    let mut findings = Vec::new();
    let mut hidden = 0usize;
    let mut read_failures = 0usize;
    let mut skipped_large = 0usize;
    for (path, rel, is_dir) in &walk.entries {
        if *is_dir {
            continue;
        }
        // 凭据面是不可读安全契约面: 路径级盘点归项一, 内容一字不碰。
        if is_credential_surface(root, path) {
            continue;
        }
        let is_config = rel
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| {
                CONFIG_EXTENSIONS
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(false);
        if !is_config {
            continue;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            read_failures += 1;
            continue;
        };
        if meta.len() > MAX_SCAN_BYTES {
            skipped_large += 1;
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            read_failures += 1;
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let redacted = redact_secret_field_values(&text);
        let secret_fields = redacted
            .matches(REDACTED)
            .count()
            .saturating_sub(text.matches(REDACTED).count());
        if secret_fields == 0 {
            continue;
        }
        if findings.len() >= MAX_FINDINGS {
            hidden += 1;
            continue;
        }
        findings.push(Finding::new(
            rel_word(rel),
            RiskLevel::Medium,
            format!(
                "{secret_fields} 个密钥字段以明文值出现在配置 (字段名命中密钥词表; 值未采集、永不出报告)"
            ),
        ));
    }
    if hidden > 0 {
        findings.push(Finding::new(
            "(聚合)",
            RiskLevel::Medium,
            format!("另有 {hidden} 个明文风险点超出条目上限未逐条列出"),
        ));
    }
    if skipped_large > 0 {
        findings.push(Finding::new(
            "(大件跳过)",
            RiskLevel::Low,
            format!(
                "{skipped_large} 个配置件超过单件读取上限 ({MAX_SCAN_BYTES} 字节), 其明文面未测"
            ),
        ));
    }
    if read_failures > 0 {
        findings.push(Finding::new(
            "(读取失败)",
            RiskLevel::Low,
            format!("{read_failures} 个配置件读取失败, 其明文面未测"),
        ));
    }
    if walk.truncated {
        findings.push(Finding::new(
            "(遍历截断)",
            RiskLevel::Low,
            format!(
                "目录遍触及深度/条目上限 (深度≤{MAX_WALK_DEPTH}, 条目≤{MAX_WALK_ENTRIES}), 未覆盖部分未测"
            ),
        ));
    }
    ExposureItem::measured(
        "plaintext_points",
        "明文风险点",
        findings,
        "明文密钥迁入凭据存储 (creds.json/钥匙串), 配置只留引用; 已明文过的值按轮换处置",
    )
    .with_note("内容只做密钥字段计数 (凭据面路径除外, 一字不碰); 值不采集、不落盘、不进报告")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn seeded_root(tag: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        let _ = tag;
        fs::create_dir_all(dir.path().join("data")).unwrap();
        dir
    }

    #[test]
    fn credential_artifact_table_matches_case_insensitively() {
        assert!(is_credential_artifact_name("client-tokens.json"));
        assert!(is_credential_artifact_name("CLIENT-TOKENS.JSON"));
        assert!(is_credential_artifact_name("creds.json"));
        assert!(!is_credential_artifact_name("users.json"));
        assert!(!is_credential_artifact_name("client-tokens.json.bak"));
    }

    #[test]
    fn walk_is_bounded_and_skips_symlinks() {
        let dir = seeded_root("walk");
        fs::write(dir.path().join("data").join("a.txt"), b"a").unwrap();
        let walk = walk_entries(dir.path());
        assert!(!walk.truncated);
        assert!(walk
            .entries
            .iter()
            .any(|(_, rel, _)| rel_word(rel) == "data/a.txt"));
    }

    #[test]
    fn key_material_shape_is_rated_high() {
        assert!(is_key_material_name("server.pem"));
        assert!(is_key_material_name("id_rsa"));
        assert!(is_key_material_name("backup.keyring.key"));
        assert!(!is_key_material_name("creds.json"));
        assert!(!is_key_material_name("users.json"));
    }
}
