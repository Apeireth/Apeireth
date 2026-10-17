//! 本地数据面自查 · 项四: 备份包里的排除项核对 (凭据不随行的自查)。
//!
//! 防御语自查: 备份包是数据面最容易"整包外带"的出口 —— 排除项核对即确认
//! 「凭据不随行」在**这一份包里**真的成立: 清单行与包内实文件两面都不得命中
//! 凭据面词表 (sensitive_path 族 + 凭据排除表), 显式排除行必须带原因 (不静默
//! 丢件)。全程只读: 只读清单与文件名, 不打开包内文件内容。

use std::collections::BTreeSet;
use std::path::Path;

use crate::backup::{BackupManifest, MANIFEST_FILE_NAME};
use crate::exposure_audit::credential_surface::{
    is_credential_surface, walk_entries, MAX_WALK_DEPTH,
};
use crate::exposure_audit::report::{ExposureItem, Finding, RiskLevel};

/// 单个体检项的发现条数上限 (超出部分聚合)。
const MAX_FINDINGS: usize = 25;

/// 包内逐文件快照的根目录名 (与 [`crate::backup`] 的包格式同词面)。
const BUNDLE_FILES_DIR: &str = "files";

/// 词面判定: manifest 行 / 包内相对路径是否命中凭据面 (任一路径段命中即算)。
fn path_is_credential_surface(word: &str) -> bool {
    is_credential_surface(Path::new(""), Path::new(word))
}

/// 项四: 备份包排除项核对 (无包可核对 = 显式未测)。
pub fn backup_exclusion_item(bundle: Option<&Path>) -> ExposureItem {
    let Some(bundle) = bundle else {
        return ExposureItem::not_measured(
            "backup_exclusions",
            "备份包排除项核对",
            "未发现可核对的备份包 (尚未导出或未指定 --bundle), 排除项核对未测",
            "先执行 apeireth backup export 生成备份包, 再复检排除项 (凭据不随行)",
        );
    };
    let manifest_path = bundle.join(MANIFEST_FILE_NAME);
    let manifest: BackupManifest = match std::fs::read(&manifest_path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<BackupManifest>(&bytes).map_err(|error| error.to_string())
        }) {
        Ok(manifest) => manifest,
        Err(reason) => {
            return ExposureItem::measured(
                "backup_exclusions",
                "备份包排除项核对",
                vec![Finding::new(
                    manifest_path.to_string_lossy().into_owned(),
                    RiskLevel::High,
                    format!("备份清单不可解析 (排除面无从核对): {reason}"),
                )],
                "重新导出备份包并复检排除项; 不可解析的包不得作为恢复来源",
            )
            .with_note(format!("核对目标: {}", bundle.to_string_lossy()));
        }
    };

    let mut flagged: BTreeSet<String> = BTreeSet::new();
    let mut hits: Vec<(String, RiskLevel, String)> = Vec::new();
    let mut push = |word: String, risk: RiskLevel, detail: String| {
        if flagged.insert(word.clone()) {
            hits.push((word, risk, detail));
        }
    };

    // 面一: 清单行 (bundle 路径与逻辑归属路径两面都查)。
    let mut manifest_rows = 0usize;
    for entry in &manifest.files {
        manifest_rows += 1;
        for word in [&entry.path, &entry.relpath] {
            if path_is_credential_surface(word) {
                push(
                    word.clone(),
                    RiskLevel::High,
                    "清单行命中凭据面词表 (凭据不得随备份外带)".to_string(),
                );
            }
        }
    }

    // 面二: 包内实文件 (清单可能说谎, 文件名是第二双眼)。
    let files_root = bundle.join(BUNDLE_FILES_DIR);
    let walk = walk_entries(&files_root);
    let mut bundle_files = 0usize;
    for (_, rel, is_dir) in &walk.entries {
        if *is_dir {
            continue;
        }
        bundle_files += 1;
        let word = rel.to_string_lossy().replace('\\', "/");
        if path_is_credential_surface(&word) {
            push(
                word,
                RiskLevel::High,
                "包内文件名命中凭据面词表 (凭据不得随备份外带)".to_string(),
            );
        }
    }

    // 发现列表 (上限截断如实聚合, 不静默丢发现)。
    let mut findings: Vec<Finding> = Vec::new();
    let mut hidden = 0usize;
    for (word, risk, detail) in hits {
        if findings.len() >= MAX_FINDINGS {
            hidden += 1;
            continue;
        }
        findings.push(Finding::new(word, risk, detail));
    }
    if hidden > 0 {
        findings.push(Finding::new(
            "(聚合)",
            RiskLevel::High,
            format!("另有 {hidden} 处凭据面命中超出条目上限未逐条列出"),
        ));
    }

    // 面三: 显式排除行必须带原因 (不静默丢件)。
    let mut reasonless = 0usize;
    for entry in &manifest.excluded {
        if entry.reason.trim().is_empty() {
            reasonless += 1;
        }
    }
    if reasonless > 0 {
        findings.push(Finding::new(
            "(排除行缺原因)",
            RiskLevel::Low,
            format!("{reasonless} 条显式排除行没有排除原因 (排除面记账不完整)"),
        ));
    }
    if walk.truncated {
        findings.push(Finding::new(
            "(遍历截断)",
            RiskLevel::Low,
            format!("包内遍历触及深度上限 (深度≤{MAX_WALK_DEPTH}), 未覆盖部分未测"),
        ));
    }

    let note = format!(
        "已核对: 清单 {manifest_rows} 行 + 包内 {bundle_files} 个文件名 + {} 条显式排除行; 只读文件名, 不打开包内内容",
        manifest.excluded.len()
    );
    ExposureItem::measured(
        "backup_exclusions",
        "备份包排除项核对",
        findings,
        "保持凭据不随行: 导出后复检排除项, 凭据面件只留显式排除记录、绝不进包",
    )
    .with_note(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_word_shapes_hit_the_credential_surface() {
        assert!(path_is_credential_surface("files/users/client-tokens.json"));
        assert!(path_is_credential_surface("files/skills/a/.env"));
        assert!(path_is_credential_surface("creds.json"));
        assert!(!path_is_credential_surface("files/users/users.json"));
        assert!(!path_is_credential_surface("files/skills/card.json"));
    }

    #[test]
    fn no_bundle_is_unmeasured() {
        let item = backup_exclusion_item(None);
        assert_eq!(
            item.status,
            crate::exposure_audit::report::ItemStatus::NotMeasured
        );
        assert!(item.note.unwrap().contains("未测"));
    }
}
