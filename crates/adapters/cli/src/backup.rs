//! G5 备份恢复 CLI 面: `apeireth backup export|restore|verify` (本地操作)。
//!
//! 同一引擎 ([`apeireth_tools_canonical::backup`]) 的第二个出口: 模型面是
//! `tool.backup_export` (AI 先用), 本面是人类/运维的包装出口 (不另起炉灶、
//! 不两套逻辑)。三个动词:
//!
//! - `export <DIR>`: 全量打包七类数据面 + manifest.json (逐文件摘要 + 版本 +
//!   时间); 目标已存在 = 拒绝即帧 (不覆盖)。凭据不随行 (显式排除 + 测试钉死);
//! - `restore <BUNDLE> [--to <ROOT>]`: manifest 校验 (摘要不符拒恢复 = 宁拒不损)
//!   → 原子落位 (先写临时再换名, 中途失败回滚原状) → 版本兼容 = 显式拒;
//!   `--to <ROOT>` = 搬家恢复 (逐件按 relpath 重组到新根), 缺省 = 同机原位;
//! - `verify <BUNDLE>`: 只验不恢复 (独立校验, 不触碰数据面)。
//!
//! 治理面: CLI 面是本地操作; `backup.export` / `backup.restore` 记审计行
//! (时间 + 清单件数, 落 `daemon-audit.jsonl` 与面板审计同一账本);
//! `verify` 是只读校验, 零审计行 (如实注)。self_status 备份节读
//! `last-backup.json` 的真值 (最近备份时间/清单件数, 尚无备份显式 null)。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_tools_canonical::backup::{
    export_backup, restore_backup, verify_backup, BackupAudit, BackupCategory, BackupExportConfig,
    BackupHooks, BackupItem, BackupManifest, BackupSources, RestoreOptions,
    BACKUP_LEDGER_FILE_NAME,
};
use apeireth_tools_canonical::skill::SKILLS_SUBDIR;

/// 本面的用法帧 (错误即帧: 参数错 = 带用法的正式通道)。
pub const USAGE: &str = "Usage:\n  apeireth backup export <DIR>\n  apeireth backup restore <BUNDLE> [--to <ROOT>]\n  apeireth backup verify <BUNDLE>";

/// 审计行的服务词面 (面板审计同账本, 服务字段分域)。
const AUDIT_SERVICE: &str = "backup";

/// 面板审计账本文件名 (数据根下, 与 [`crate::gateway_panels`] 同一账本)。
const AUDIT_LEDGER_FILE: &str = "daemon-audit.jsonl";

/// 备份面的三路数据面路径 (env 解析; 测试可显式构造)。
///
/// 七类数据面的原位口径: 会话库 = `session_db` (+ WAL 侧车 + 逐域
/// `users/<id>/data/sessions.sqlite3*`); 认知库 = `cognitive_db` (+ 侧车);
/// 记忆库 = 逐域 `users/<id>/data/cognitive.sqlite3*`; 技能卡 = `<root>/skills/`;
/// 关怀规则 = `<root>/care_rules.json` (+ `care_rules/`); 用户表 =
/// `<root>/users.json`; 域文档 = `<root>/users/<id>/domain.json`。
#[derive(Debug, Clone)]
pub struct BackupCliPaths {
    /// 数据根 (面板持久档所在)。
    pub data_root: PathBuf,
    /// 默认域会话库 (含配额账本)。
    pub session_db: PathBuf,
    /// 默认域认知库/记忆库。
    pub cognitive_db: PathBuf,
}

impl BackupCliPaths {
    /// 显式三路 (测试与非 env 装配用)。
    pub fn new(
        data_root: impl Into<PathBuf>,
        session_db: impl Into<PathBuf>,
        cognitive_db: impl Into<PathBuf>,
    ) -> Self {
        Self {
            data_root: data_root.into(),
            session_db: session_db.into(),
            cognitive_db: cognitive_db.into(),
        }
    }

    /// env 解析 (与运行时装配同一条路径口径: `APEIRETH_DATA_DIR` /
    /// `APEIRETH_SESSION_DB` / `APEIRETH_COGNITIVE_DB` 覆写语义逐字不变)。
    pub fn from_env() -> Self {
        Self::new(
            crate::default_panel_data_dir(),
            crate::session_db_path(),
            crate::cognitive_db_path(),
        )
    }
}

/// 组装备份数据源: 七类数据面全量 (缺失源跳过, 类目计数如实)。
pub fn backup_sources(paths: &BackupCliPaths) -> BackupSources {
    let data_root = &paths.data_root;
    let mut items = Vec::new();

    // 会话库 (默认域 + WAL 侧车; 侧车缺失 = 跳过)。
    push_sqlite_family(
        &mut items,
        BackupCategory::Sessions,
        &paths.session_db,
        &PathBuf::from("sessions.sqlite3"),
    );
    // 认知库 (默认域 + WAL 侧车)。
    push_sqlite_family(
        &mut items,
        BackupCategory::Cognitive,
        &paths.cognitive_db,
        &PathBuf::from("cognitive.sqlite3"),
    );

    // 逐域数据面 (`users/<id>/`): 域文档 + 会话库/记忆库 (每户独立记忆域)。
    let users_root = data_root.join("users");
    if let Ok(entries) = std::fs::read_dir(&users_root) {
        let mut domains: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        domains.sort();
        for domain_dir in domains {
            let Some(name) = domain_dir.file_name().map(|name| name.to_owned()) else {
                continue;
            };
            // 域文档 (`users/<id>/domain.json`)。
            let domain_doc = domain_dir.join("domain.json");
            if domain_doc.is_file() {
                items.push(BackupItem::file(
                    BackupCategory::Domains,
                    &domain_doc,
                    PathBuf::from("users").join(&name).join("domain.json"),
                ));
            }
            // 逐域会话库 / 记忆库 (含 WAL 侧车)。
            let user_data = domain_dir.join("data");
            push_sqlite_family(
                &mut items,
                BackupCategory::Sessions,
                &user_data.join("sessions.sqlite3"),
                &PathBuf::from("users")
                    .join(&name)
                    .join("data")
                    .join("sessions.sqlite3"),
            );
            push_sqlite_family(
                &mut items,
                BackupCategory::Memory,
                &user_data.join("cognitive.sqlite3"),
                &PathBuf::from("users")
                    .join(&name)
                    .join("data")
                    .join("cognitive.sqlite3"),
            );
        }
    }

    // 技能卡 (技能库整树, 每卡一 StoredDoc)。
    items.push(BackupItem::tree(
        BackupCategory::Skills,
        data_root.join(SKILLS_SUBDIR),
        PathBuf::from(SKILLS_SUBDIR),
    ));
    // 关怀规则 (`care_rules.json` + `care_rules/` 整树; 现无写入方 = 0 件,
    // manifest 如实记)。
    items.push(BackupItem::file(
        BackupCategory::CareRules,
        data_root.join("care_rules.json"),
        PathBuf::from("care_rules.json"),
    ));
    items.push(BackupItem::tree(
        BackupCategory::CareRules,
        data_root.join("care_rules"),
        PathBuf::from("care_rules"),
    ));
    // 用户表 (`users.json`)。
    items.push(BackupItem::file(
        BackupCategory::Users,
        data_root.join("users.json"),
        PathBuf::from("users.json"),
    ));

    BackupSources::new(data_root.clone(), items)
}

/// 一个 sqlite 库 + WAL/SHM 侧车的源组 (缺失件跳过, 引擎侧同口径)。
fn push_sqlite_family(
    items: &mut Vec<BackupItem>,
    category: BackupCategory,
    origin: &Path,
    relpath: &Path,
) {
    items.push(BackupItem::file(category, origin, relpath));
    for sidecar in ["-wal", "-shm"] {
        let mut origin_name = origin
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        origin_name.push_str(sidecar);
        let mut rel_name = relpath
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        rel_name.push_str(sidecar);
        items.push(BackupItem::file(
            category,
            origin.with_file_name(origin_name),
            relpath.with_file_name(rel_name),
        ));
    }
}

/// 引擎钩子: 「最近备份」记录 (self_status 备份节真值) + 面板审计账本同源
/// 的审计行 (backup.export / backup.restore, 记时间与清单件数)。
pub fn backup_hooks(data_root: &Path) -> BackupHooks {
    BackupHooks::none()
        .with_ledger(data_root.join(BACKUP_LEDGER_FILE_NAME))
        .with_audit(Arc::new(DaemonAudit {
            path: data_root.join(AUDIT_LEDGER_FILE),
        }))
}

/// 备份节探测口的数据源路径 (self_status 备份节读它: 最近备份时间/清单件数)。
pub fn backup_ledger_path(data_root: &Path) -> PathBuf {
    data_root.join(BACKUP_LEDGER_FILE_NAME)
}

/// `tool.backup_export` 的注入配置 (数据源 + 钩子, 与 CLI 面同一引擎同一口径)。
pub fn backup_export_config(paths: &BackupCliPaths) -> BackupExportConfig {
    BackupExportConfig::new(backup_sources(paths), backup_hooks(&paths.data_root))
}

/// 面板审计账本的审计口 (尽力而为追加: 审计失败绝不掀翻备份本体)。
struct DaemonAudit {
    path: PathBuf,
}

impl BackupAudit for DaemonAudit {
    fn record(&self, event: &str, detail: &str) {
        let entry = apeireth_gateway::AuditDto {
            ts: now_millis(),
            event: event.to_string(),
            service: AUDIT_SERVICE.to_string(),
            detail: Some(detail.to_string()),
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            use std::io::Write as _;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}

/// 当前时刻 (epoch 毫秒; 回拨钳 0, 不编时间)。
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// `apeireth backup` 入口 (env 解析数据面路径)。
pub fn run_backup(args: &[String]) -> Result<String, String> {
    run_backup_at(&BackupCliPaths::from_env(), args)
}

/// 显式路径入口 (测试与非 env 装配用; 语义与 [`run_backup`] 逐字一致)。
pub fn run_backup_at(paths: &BackupCliPaths, args: &[String]) -> Result<String, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Err(USAGE.to_string()),
    };
    match verb {
        "export" => {
            let target = match rest {
                [target] if !target.trim().is_empty() => PathBuf::from(target),
                _ => return Err(format!("backup export requires exactly one <DIR>\n{USAGE}")),
            };
            let manifest = export_backup(
                &backup_sources(paths),
                &target,
                &backup_hooks(&paths.data_root),
            )
            .map_err(|error| error.to_string())?;
            Ok(render_export_summary(&target, &manifest))
        }
        "restore" => {
            let (bundle, options) = parse_restore_args(rest)?;
            let report = restore_backup(&bundle, &options, &backup_hooks(&paths.data_root))
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "backup restore: ok entries={} (bundle={})",
                report.entries,
                bundle.display()
            ))
        }
        "verify" => {
            let bundle = match rest {
                [bundle] if !bundle.trim().is_empty() => PathBuf::from(bundle),
                _ => {
                    return Err(format!(
                        "backup verify requires exactly one <BUNDLE>\n{USAGE}"
                    ))
                }
            };
            let manifest = verify_backup(&bundle).map_err(|error| error.to_string())?;
            Ok(format!(
                "backup verify: ok entries={} format_version={} (bundle={})",
                manifest.entry_count(),
                manifest.format_version,
                bundle.display()
            ))
        }
        _ => Err(USAGE.to_string()),
    }
}

/// 解析 restore 参数 (`<BUNDLE> [--to <ROOT>]`; 未知/悬空旗标 = 用法帧)。
fn parse_restore_args(args: &[String]) -> Result<(PathBuf, RestoreOptions), String> {
    let mut bundle: Option<PathBuf> = None;
    let mut to_root: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--to" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(format!("backup restore --to requires a value\n{USAGE}"));
                };
                if value.trim().is_empty() {
                    return Err(format!(
                        "backup restore --to requires a non-empty root\n{USAGE}"
                    ));
                }
                to_root = Some(PathBuf::from(value));
            }
            flag if flag.starts_with("--") => {
                return Err(format!("unknown backup restore argument: {flag}\n{USAGE}"));
            }
            value => {
                if bundle.is_some() {
                    return Err(format!(
                        "backup restore takes exactly one <BUNDLE>\n{USAGE}"
                    ));
                }
                bundle = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }
    let bundle = bundle.ok_or_else(|| format!("backup restore requires a <BUNDLE>\n{USAGE}"))?;
    let options = match to_root {
        Some(root) => RestoreOptions::to_root(root),
        None => RestoreOptions::in_place(),
    };
    Ok((bundle, options))
}

/// 导出摘要 (件数 + 逐类目计数 + bundle 位; 清单数即审计行口径)。
fn render_export_summary(target: &Path, manifest: &BackupManifest) -> String {
    let counts = manifest
        .category_counts()
        .into_iter()
        .map(|(category, count)| format!("{category}={count}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "backup export: bundle={} entries={} excluded={} format_version={} [{}]",
        target.display(),
        manifest.entry_count(),
        manifest.excluded.len(),
        manifest.format_version,
        counts
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_args_parse_bundle_and_optional_relocation_root() {
        let (bundle, options) =
            parse_restore_args(&s(&["bundle-a", "--to", "D:\\restored"])).unwrap();
        assert_eq!(bundle, PathBuf::from("bundle-a"));
        assert_eq!(options.to_root, Some(PathBuf::from("D:\\restored")));

        let (bundle, options) = parse_restore_args(&s(&["bundle-b"])).unwrap();
        assert_eq!(bundle, PathBuf::from("bundle-b"));
        assert_eq!(options.to_root, None);
    }

    #[test]
    fn restore_args_reject_unknown_and_dangling_shapes() {
        for args in [
            vec![],
            vec!["--to".to_string()],
            vec!["--to".to_string(), "   ".to_string()],
            vec!["a".to_string(), "--nope".to_string()],
            vec!["a".to_string(), "b".to_string()],
        ] {
            assert!(parse_restore_args(&args).is_err(), "must refuse: {args:?}");
        }
    }

    #[test]
    fn sqlite_family_sources_cover_main_db_and_sidecars() {
        let mut items = Vec::new();
        push_sqlite_family(
            &mut items,
            BackupCategory::Sessions,
            Path::new("/data/sessions.sqlite3"),
            Path::new("sessions.sqlite3"),
        );
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].origin, PathBuf::from("/data/sessions.sqlite3"));
        assert_eq!(items[1].origin, PathBuf::from("/data/sessions.sqlite3-wal"));
        assert_eq!(items[2].origin, PathBuf::from("/data/sessions.sqlite3-shm"));
    }

    fn s(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }
}
