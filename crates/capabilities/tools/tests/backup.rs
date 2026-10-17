//! G5 备份恢复引擎测试: 往返矩阵 + 宁拒不损 + 凭据不随行 + 原子落位 +
//! 版本兼容 (显式拒) + verify 独立 + 审计行 + 增量语义 (不丢不覆盖)。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_tools_canonical::backup::{
    export_backup, read_backup_ledger, restore_backup, verify_backup, BackupAudit, BackupCategory,
    BackupError, BackupHooks, BackupItem, BackupSources, RestoreOptions, BACKUP_FORMAT_VERSION,
    BACKUP_LEDGER_FILE_NAME, MANIFEST_FILE_NAME,
};
use apeireth_tools_canonical::{
    backup_status_probe, render_snapshot, BackupStatus, SelfStatusIdentity, SelfStatusSnapshot,
    SelfStatusTool,
};

/// 审计口的录供体 (逐行留证, 断言事件名/明细/清单数)。
#[derive(Default)]
struct RecordingAudit {
    rows: Mutex<Vec<(String, String)>>,
}

impl BackupAudit for RecordingAudit {
    fn record(&self, event: &str, detail: &str) {
        self.rows
            .lock()
            .expect("audit lock poisoned (0 装诚实)")
            .push((event.to_string(), detail.to_string()));
    }
}

fn temp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("apeireth-backup-{tag}-"))
        .tempdir()
        .expect("temp dir")
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, bytes).expect("write fixture");
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).expect("read fixture")
}

fn rel(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// 播种一个七类数据面的微缩版 (引擎级: 各类目至少一件, 共 8 件)。
fn seed_layout(root: &Path) -> BackupSources {
    write(&root.join("sessions.sqlite3"), b"session-db-v1");
    write(&root.join("cognitive.sqlite3"), b"cognitive-db-v1");
    write(
        &root.join("users.json"),
        br#"{"users":[{"user_id":"alice","role":"owner"}]}"#,
    );
    write(
        &root.join("users/alice/domain.json"),
        br#"{"name":"alice-domain"}"#,
    );
    write(
        &root.join("users/alice/data/cognitive.sqlite3"),
        b"memory-db-alice",
    );
    write(&root.join("skills/skill-a.json"), b"skill-a-v1");
    write(&root.join("skills/skill-b.json"), b"skill-b-v1");
    write(&root.join("care_rules.json"), br#"{"rules":[]}"#);

    let items = vec![
        BackupItem::file(
            BackupCategory::Sessions,
            root.join("sessions.sqlite3"),
            "sessions.sqlite3",
        ),
        BackupItem::file(
            BackupCategory::Cognitive,
            root.join("cognitive.sqlite3"),
            "cognitive.sqlite3",
        ),
        BackupItem::file(
            BackupCategory::Memory,
            root.join("users/alice/data/cognitive.sqlite3"),
            "users/alice/data/cognitive.sqlite3",
        ),
        BackupItem::tree(BackupCategory::Skills, root.join("skills"), "skills"),
        BackupItem::file(
            BackupCategory::CareRules,
            root.join("care_rules.json"),
            "care_rules.json",
        ),
        BackupItem::file(BackupCategory::Users, root.join("users.json"), "users.json"),
        BackupItem::file(
            BackupCategory::Domains,
            root.join("users/alice/domain.json"),
            "users/alice/domain.json",
        ),
    ];
    BackupSources::new(root, items)
}

/// 全量往返: 导出 → 校验 → 搬家恢复 → **逐字节一致**断言 (每件成员)。
#[test]
fn full_round_trip_restores_every_member_byte_for_byte() {
    let root = temp_dir("roundtrip");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");

    let manifest = export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");
    assert_eq!(manifest.format_version, BACKUP_FORMAT_VERSION);
    assert_eq!(manifest.entry_count(), 8, "七类微缩版共 8 件");
    assert_eq!(manifest.category_counts().len(), 7, "七类目齐");

    let verified = verify_backup(&bundle).expect("verify");
    assert_eq!(verified.files, manifest.files, "verify 读到同一清单");

    let restored = root.path().join("restored");
    let report = restore_backup(
        &bundle,
        &RestoreOptions::to_root(&restored),
        &BackupHooks::none(),
    )
    .expect("restore");
    assert_eq!(report.entries, 8);
    for entry in &manifest.files {
        assert_eq!(
            read(&data.join(rel(&entry.relpath))),
            read(&restored.join(rel(&entry.relpath))),
            "成员必须逐字节一致: {}",
            entry.relpath
        );
    }
}

/// 同机原位恢复: 成员文件回到 bundle 字节态 (原位 = manifest `origin`)。
#[test]
fn in_place_restore_returns_members_to_their_bundle_bytes() {
    let root = temp_dir("inplace");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    // 备份后成员被改动 → 原位恢复回卷到 bundle 字节态。
    write(&data.join("skills/skill-a.json"), b"skill-a-v2-changed");
    let report = restore_backup(&bundle, &RestoreOptions::in_place(), &BackupHooks::none())
        .expect("restore in place");
    assert_eq!(report.entries, 8);
    assert_eq!(read(&data.join(rel("skills/skill-a.json"))), b"skill-a-v1");
}

/// manifest 损坏 (坏 JSON / 缺失) = 拒恢复 + 拒校验, 数据面零触碰 (宁拒不损)。
#[test]
fn corrupt_manifest_is_refused_and_nothing_touches_the_data_plane() {
    let root = temp_dir("manifest-corrupt");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    let before = read(&data.join("sessions.sqlite3"));
    std::fs::write(bundle.join(MANIFEST_FILE_NAME), b"{ not json").expect("corrupt manifest");
    assert!(matches!(
        verify_backup(&bundle),
        Err(BackupError::ManifestCorrupt(_))
    ));
    assert!(matches!(
        restore_backup(
            &bundle,
            &RestoreOptions::to_root(root.path().join("out")),
            &BackupHooks::none()
        ),
        Err(BackupError::ManifestCorrupt(_))
    ));
    assert_eq!(
        read(&data.join("sessions.sqlite3")),
        before,
        "损坏清单的恢复尝试必须零触碰 (宁拒不损)"
    );

    std::fs::remove_file(bundle.join(MANIFEST_FILE_NAME)).expect("remove manifest");
    assert!(matches!(
        verify_backup(&bundle),
        Err(BackupError::ManifestCorrupt(_))
    ));
}

/// 逐文件摘要不符 (字节翻转) = 拒恢复: 一个字节都不落盘 (宁拒不损)。
#[test]
fn a_single_flipped_byte_refuses_the_whole_restore() {
    let root = temp_dir("digest");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    let manifest = export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    let victim = bundle.join(rel(&manifest.files[0].path));
    let mut bytes = read(&victim);
    bytes[0] ^= 0xFF;
    std::fs::write(&victim, &bytes).expect("flip byte");

    assert!(matches!(
        verify_backup(&bundle),
        Err(BackupError::DigestMismatch { .. })
    ));
    let target_root = root.path().join("out");
    assert!(matches!(
        restore_backup(
            &bundle,
            &RestoreOptions::to_root(&target_root),
            &BackupHooks::none()
        ),
        Err(BackupError::DigestMismatch { .. })
    ));
    assert!(
        !target_root.exists(),
        "摘要不符的恢复尝试必须零落盘 (宁拒不损)"
    );
    assert_eq!(read(&data.join("sessions.sqlite3")), b"session-db-v1");
}

/// 凭据不随行: 密钥/令牌面显式排除 (含误列源与树内件), 打包内零密钥断言。
#[test]
fn credentials_never_travel_with_a_bundle() {
    let root = temp_dir("creds");
    let data = root.path().join("data");
    let mut sources = seed_layout(&data);
    // 对抗面 ①: 凭据件被误列成显式源 (照样不进包)。
    write(
        &data.join("creds.json"),
        br#"{"api_key":"SECRET-CREDS-VALUE"}"#,
    );
    sources.items.push(BackupItem::file(
        BackupCategory::Users,
        data.join("creds.json"),
        "creds.json",
    ));
    // 对抗面 ②: 凭据件藏在被递归收录的树里。
    write(
        &data.join("skills/creds.json"),
        br#"{"api_key":"SECRET-TREE-CREDS"}"#,
    );
    write(&data.join("skills/id_rsa"), b"SECRET-PRIVATE-KEY");
    write(&data.join("skills/keep.json"), b"legit-skill");
    // 对抗面 ③: 数据根上的其余密钥/令牌面 (不在任何类目, 双保险)。
    write(&data.join("apeireth-keyring.bin"), b"SECRET-KEYRING-BYTES");
    write(
        &data.join("apeireth-keyring.master.key"),
        b"SECRET-MASTER-KEY",
    );
    write(
        &data.join("client-tokens.json"),
        br#"{"token":"SECRET-CLIENT-TOKEN"}"#,
    );
    write(
        &data.join("im-channels.json"),
        br#"{"secret":"SECRET-SHARED-SIGNING"}"#,
    );
    write(&data.join(".env"), b"APEIRETH_API_KEY=SECRET-ENV-VALUE");

    let bundle = root.path().join("bundle");
    let manifest = export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    // 打包内零密钥断言: 逐文件字节 + 清单文本全文扫, 秘值零命中。
    let bundle_text = std::fs::read_to_string(bundle.join(MANIFEST_FILE_NAME)).expect("manifest");
    for secret in [
        "SECRET-CREDS-VALUE",
        "SECRET-TREE-CREDS",
        "SECRET-PRIVATE-KEY",
        "SECRET-KEYRING-BYTES",
        "SECRET-MASTER-KEY",
        "SECRET-CLIENT-TOKEN",
        "SECRET-SHARED-SIGNING",
        "SECRET-ENV-VALUE",
    ] {
        assert!(
            !bundle_text.contains(secret),
            "manifest 不得携带秘值: {secret}"
        );
        for entry in &manifest.files {
            let bytes = read(&bundle.join(rel(&entry.path)));
            assert!(
                !bytes
                    .windows(secret.len())
                    .any(|window| window == secret.as_bytes()),
                "打包件 {} 不得携带秘值 {secret}",
                entry.path
            );
        }
    }
    // 件名零命中 (密钥/令牌面文件一个都不进包)。
    for entry in &manifest.files {
        for forbidden in [
            "creds.json",
            "id_rsa",
            "apeireth-keyring",
            "client-tokens.json",
            "im-channels.json",
            ".env",
        ] {
            assert!(
                !entry.path.contains(forbidden),
                "凭据件不得进包: {}",
                entry.path
            );
        }
    }
    // 合法数据面照常随行 (排除不误伤) + 排除面逐条记账 (不静默丢件)。
    assert!(manifest
        .files
        .iter()
        .any(|entry| entry.path.contains("keep.json")));
    for name in ["creds.json", "skills/creds.json", "skills/id_rsa"] {
        assert!(
            manifest
                .excluded
                .iter()
                .any(|excluded| excluded.path == name),
            "排除面必须逐条记账: {name}"
        );
    }
}

/// 原子落位: 换名中途失败 = 回滚原状 (恢复起始态逐字节保留) + 零临时残留。
#[test]
fn mid_placement_failure_rolls_back_to_the_original_state() {
    let root = temp_dir("atomic");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    // 备份后 live 状态前进 (恢复起始态 = "NEW-CARE-RULES")。
    write(&data.join("care_rules.json"), b"NEW-CARE-RULES");
    // 后续目标 (`skills/skill-a.json`) 在恢复起始态被非空目录占位:
    // 落位到它必须拒绝即帧 (拒换非文件占位), 且发生在前面目标 (bundle 路径序
    // 首件 `files/care_rules/care_rules.json`) 已落位之后 —— 回滚必须整链生效。
    std::fs::remove_file(data.join(rel("skills/skill-a.json"))).expect("remove target 2");
    let blocker = data.join(rel("skills/skill-a.json"));
    std::fs::create_dir_all(&blocker).expect("blocker dir");
    write(&blocker.join("occupied.txt"), b"stay");

    let error = restore_backup(&bundle, &RestoreOptions::in_place(), &BackupHooks::none());
    assert!(
        matches!(error, Err(BackupError::Refused(_))),
        "非文件占位必须拒绝即帧"
    );
    // 原状断言: 目标 #1 回到恢复起始态字节, 占位目录原样, 零临时/保全残留。
    assert_eq!(
        read(&data.join("care_rules.json")),
        b"NEW-CARE-RULES",
        "中途失败必须回滚原状 (恢复起始态)"
    );
    assert_eq!(read(&blocker.join("occupied.txt")), b"stay");
    let residue = walk_names(&data);
    assert!(
        !residue.iter().any(|name| name.contains("apeireth-restore")),
        "零临时/保全残留: {residue:?}"
    );
}

/// 版本兼容语义 = **显式拒** (二选一钉死): 不论旧版 (0) 还是新版 (2),
/// 与本实现不符一律显式拒绝并报双方版本号, 不静默迁移、不猜。
#[test]
fn version_mismatch_is_explicitly_refused_for_old_and_new_formats() {
    let root = temp_dir("version");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    for forged in [0u32, 2u32] {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&read(&bundle.join(MANIFEST_FILE_NAME))).expect("manifest json");
        manifest["format_version"] = serde_json::json!(forged);
        std::fs::write(
            bundle.join(MANIFEST_FILE_NAME),
            serde_json::to_vec_pretty(&manifest).expect("rewrite"),
        )
        .expect("write forged manifest");

        match verify_backup(&bundle) {
            Err(BackupError::VersionMismatch { found, expected }) => {
                assert_eq!(found, forged);
                assert_eq!(expected, BACKUP_FORMAT_VERSION);
            }
            other => panic!("版本不匹配必须显式拒: {other:?}"),
        }
        assert!(matches!(
            restore_backup(&bundle, &RestoreOptions::in_place(), &BackupHooks::none()),
            Err(BackupError::VersionMismatch { .. })
        ));
    }
}

/// verify 独立: 只验不恢复 (数据面零读写), 坏包独立报坏。
#[test]
fn verify_is_independent_and_never_touches_the_data_plane() {
    let root = temp_dir("verify");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    let manifest = export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    let snapshot_before = walk_files(&data);
    assert_eq!(verify_backup(&bundle).expect("verify").entry_count(), 8);
    assert_eq!(
        walk_files(&data),
        snapshot_before,
        "verify 必须零触碰数据面"
    );

    // 坏包独立报坏 (不看数据面脸色)。
    let victim = bundle.join(rel(&manifest.files[0].path));
    std::fs::write(&victim, b"tampered").expect("tamper");
    assert!(matches!(
        verify_backup(&bundle),
        Err(BackupError::DigestMismatch { .. })
    ));
}

/// 治理面: 审计行记时间与清单数 (backup.export / backup.restore) +
/// 「最近备份」记录落账 (self_status 备份节真值来源)。
#[test]
fn audit_rows_and_ledger_record_time_and_manifest_counts() {
    let root = temp_dir("audit");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    let audit = Arc::new(RecordingAudit::default());
    let hooks = BackupHooks::none()
        .with_ledger(data.join(BACKUP_LEDGER_FILE_NAME))
        .with_audit(audit.clone());

    let manifest = export_backup(&sources, &bundle, &hooks).expect("export");
    let report = restore_backup(
        &bundle,
        &RestoreOptions::to_root(root.path().join("restored")),
        &hooks,
    )
    .expect("restore");
    assert_eq!(report.entries, manifest.entry_count());

    let rows = audit.rows.lock().expect("audit lock poisoned (0 装诚实)");
    assert_eq!(rows.len(), 2, "导出与恢复各一行审计");
    assert_eq!(rows[0].0, "backup.export");
    assert!(
        rows[0]
            .1
            .contains(&format!("entries={}", manifest.entry_count())),
        "导出审计行必须记清单数: {}",
        rows[0].1
    );
    assert_eq!(rows[1].0, "backup.restore");
    assert!(
        rows[1].1.contains(&format!("entries={}", report.entries)),
        "恢复审计行必须记清单数: {}",
        rows[1].1
    );

    // 「最近备份」记录 (时间 + 清单数真值)。
    let ledger = read_backup_ledger(&data.join(BACKUP_LEDGER_FILE_NAME))
        .expect("ledger read")
        .expect("ledger present");
    assert_eq!(ledger.exported_at_ms, manifest.created_at_ms);
    assert_eq!(ledger.manifest_entries, manifest.entry_count());
}

/// 增量语义钉死: 备份后新增数据**不丢不覆盖** (非成员不动、下一份全量收进);
/// 导出永不覆盖已有 bundle。
#[test]
fn incremental_semantics_are_pinned_end_to_end() {
    let root = temp_dir("incremental");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle_one = root.path().join("bundle-1");
    export_backup(&sources, &bundle_one, &BackupHooks::none()).expect("export 1");

    write(&data.join(rel("skills/skill-c.json")), b"skill-c-new");
    write(
        &data.join(rel("skills/skill-a.json")),
        b"skill-a-v2-changed",
    );

    restore_backup(
        &bundle_one,
        &RestoreOptions::to_root(&data),
        &BackupHooks::none(),
    )
    .expect("restore bundle-1 over the moved-on data root");
    assert_eq!(
        read(&data.join(rel("skills/skill-c.json"))),
        b"skill-c-new",
        "备份后新增的非成员必须不丢不覆盖"
    );
    assert_eq!(
        read(&data.join(rel("skills/skill-a.json"))),
        b"skill-a-v1",
        "成员回到 bundle 字节态 (恢复的定义)"
    );

    // 下一份全量把新增收进 (增量数据不丢)。
    let bundle_two = root.path().join("bundle-2");
    let manifest_two =
        export_backup(&sources, &bundle_two, &BackupHooks::none()).expect("export 2");
    assert!(
        manifest_two
            .files
            .iter()
            .any(|entry| entry.relpath == "skills/skill-c.json"),
        "备份后新增数据必须进下一份全量"
    );

    // 不覆盖: 已有 bundle 目标拒绝即帧。
    assert!(matches!(
        export_backup(&sources, &bundle_two, &BackupHooks::none()),
        Err(BackupError::Refused(_))
    ));
}

/// 路径安全闸: 清单里的穿越/绝对路径拒绝即帧 (防 bundle 目录穿越)。
#[test]
fn hostile_manifest_paths_are_refused() {
    let root = temp_dir("traversal");
    let data = root.path().join("data");
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    export_backup(&sources, &bundle, &BackupHooks::none()).expect("export");

    for hostile in ["../evil", "files/../../evil", "/abs/evil"] {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&read(&bundle.join(MANIFEST_FILE_NAME))).expect("manifest json");
        manifest["files"][0]["relpath"] = serde_json::json!(hostile);
        std::fs::write(
            bundle.join(MANIFEST_FILE_NAME),
            serde_json::to_vec_pretty(&manifest).expect("rewrite"),
        )
        .expect("write hostile manifest");
        assert!(
            matches!(
                restore_backup(
                    &bundle,
                    &RestoreOptions::to_root(root.path().join("out")),
                    &BackupHooks::none()
                ),
                Err(BackupError::ManifestCorrupt(_))
            ),
            "穿越路径必须拒绝即帧: {hostile}"
        );
    }
}

/// self_status 备份节: 最近备份时间/清单件数真值; 尚无备份显式 null + 原因。
#[test]
fn backup_status_reports_truth_or_explicit_null() {
    let root = temp_dir("status");
    let data = root.path().join("data");
    let ledger_path = data.join(BACKUP_LEDGER_FILE_NAME);

    // 尚无备份: 显式 null + 原因 (不编时间不编件数)。
    let probe = backup_status_probe(ledger_path.clone());
    let status = probe().expect("probe");
    assert_eq!(status.last_backup_at_ms, None);
    assert_eq!(status.manifest_entries, None);
    assert!(status
        .reason
        .as_deref()
        .is_some_and(|reason| !reason.is_empty()));
    let mut snapshot = SelfStatusSnapshot::minimal(SelfStatusIdentity::default(), Vec::new());
    snapshot.backup = Some(status);
    let value = render_snapshot(&snapshot);
    assert_eq!(
        value["backup"]["last_backup_at_ms"],
        serde_json::json!(null)
    );
    assert_eq!(value["backup"]["manifest_entries"], serde_json::json!(null));
    assert!(SelfStatusTool::output_schema().validate(&value).is_ok());

    // 导出后: 真值 (时间 = manifest 时间, 件数 = 清单件数)。
    let sources = seed_layout(&data);
    let bundle = root.path().join("bundle");
    let hooks = BackupHooks::none().with_ledger(&ledger_path);
    let manifest = export_backup(&sources, &bundle, &hooks).expect("export");
    let status = probe().expect("probe after export");
    assert_eq!(status.last_backup_at_ms, Some(manifest.created_at_ms));
    assert_eq!(status.manifest_entries, Some(manifest.entry_count()));
    assert_eq!(status.reason, None);
    let mut snapshot = SelfStatusSnapshot::minimal(SelfStatusIdentity::default(), Vec::new());
    snapshot.backup = Some(BackupStatus {
        last_backup_at_ms: status.last_backup_at_ms,
        manifest_entries: status.manifest_entries,
        reason: None,
    });
    let value = render_snapshot(&snapshot);
    assert_eq!(
        value["backup"]["manifest_entries"],
        serde_json::json!(manifest.entry_count())
    );
    assert!(SelfStatusTool::output_schema().validate(&value).is_ok());
}

/// 收集数据根下的全部文件 (递归, 排序稳定)。
fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// 收集数据根下的全部文件名 (相对词面)。
fn walk_names(root: &Path) -> Vec<String> {
    walk_files(root)
        .into_iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}
