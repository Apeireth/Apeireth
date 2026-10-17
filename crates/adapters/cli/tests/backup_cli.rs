//! G5 备份恢复 CLI 面测试: 七类数据面全量往返 (逐字节一致) + 审计行
//! (backup.export/restore 记时间与清单数) + verify 独立 + 不覆盖 + 凭据不随行
//! + self_status 备份节真值 (last-backup.json 同源)。

use std::path::{Path, PathBuf};

use apeireth_cli::backup::{
    backup_ledger_path, backup_sources, run_backup_at, BackupCliPaths, USAGE,
};

fn temp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("apeireth-backup-cli-{tag}-"))
        .tempdir()
        .expect("temp dir")
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, bytes).expect("write fixture");
}

fn rel(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// 播种生产同款数据面 (会话库/认知库/记忆库/技能卡/关怀规则/用户表/域文档)。
fn seed_data_root(root: &Path) -> BackupCliPaths {
    write(&root.join("sessions.sqlite3"), b"session-db-v1");
    write(&root.join("sessions.sqlite3-wal"), b"session-wal-v1");
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
        &root.join("users/alice/data/sessions.sqlite3"),
        b"session-db-alice",
    );
    write(
        &root.join("users/alice/data/cognitive.sqlite3"),
        b"memory-db-alice",
    );
    write(&root.join("skills/card-a.json"), b"skill-a-v1");
    write(&root.join("skills/card-b.json"), b"skill-b-v1");
    write(&root.join("care_rules.json"), br#"{"rules":[]}"#);
    BackupCliPaths::new(
        root,
        root.join("sessions.sqlite3"),
        root.join("cognitive.sqlite3"),
    )
}

/// 七类数据面全量往返: `backup export` → `backup restore --to` → 逐字节一致。
#[test]
fn cli_round_trip_covers_the_seven_categories_and_restores_byte_for_byte() {
    let root = temp_dir("roundtrip");
    let data = root.path().join("data");
    let paths = seed_data_root(&data);
    let bundle = root.path().join("bundle");

    let summary =
        run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).expect("backup export");
    assert!(summary.contains("entries="), "{summary}");
    assert!(summary.contains("bundle="), "{summary}");

    let manifest_path = bundle.join("manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("manifest")).expect("json");
    assert_eq!(
        manifest["format_version"],
        serde_json::json!(apeireth_tools_canonical::BACKUP_FORMAT_VERSION)
    );
    assert!(
        manifest["created_at_ms"].as_i64().unwrap_or(0) > 0,
        "记时间"
    );
    let categories: Vec<String> = manifest["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|entry| entry["category"].as_str().expect("category").to_string())
        .collect();
    for expected in [
        "sessions",
        "cognitive",
        "memory",
        "skills",
        "care_rules",
        "users",
        "domains",
    ] {
        assert!(
            categories.iter().any(|category| category == expected),
            "七类目缺 {expected}: {categories:?}"
        );
    }

    let restored = root.path().join("restored");
    let summary = run_backup_at(
        &paths,
        &args(&[
            "restore",
            bundle.to_str().unwrap(),
            "--to",
            restored.to_str().unwrap(),
        ]),
    )
    .expect("backup restore");
    assert!(summary.contains("entries="), "{summary}");

    for entry in manifest["files"].as_array().expect("files") {
        let relpath = entry["relpath"].as_str().expect("relpath");
        let original = std::fs::read(data.join(rel(relpath))).expect("original bytes");
        let placed = std::fs::read(restored.join(rel(relpath))).expect("restored bytes");
        assert_eq!(original, placed, "逐字节一致: {relpath}");
    }
}

/// 治理面: 审计行落面板账本 (`daemon-audit.jsonl`), 记时间与清单数。
#[test]
fn cli_backup_writes_audit_rows_with_time_and_manifest_counts() {
    let root = temp_dir("audit");
    let data = root.path().join("data");
    let paths = seed_data_root(&data);
    let bundle = root.path().join("bundle");
    run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).expect("export");
    run_backup_at(
        &paths,
        &args(&[
            "restore",
            bundle.to_str().unwrap(),
            "--to",
            root.path().join("out").to_str().unwrap(),
        ]),
    )
    .expect("restore");

    let audit_text =
        std::fs::read_to_string(data.join("daemon-audit.jsonl")).expect("audit ledger");
    let rows: Vec<serde_json::Value> = audit_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("audit row json"))
        .collect();
    let export_row = rows
        .iter()
        .find(|row| row["event"] == serde_json::json!("backup.export"))
        .expect("backup.export 审计行");
    let restore_row = rows
        .iter()
        .find(|row| row["event"] == serde_json::json!("backup.restore"))
        .expect("backup.restore 审计行");
    for row in [&export_row, &restore_row] {
        assert!(
            row["ts"].as_i64().unwrap_or(0) > 0,
            "审计行必须记时间: {row}"
        );
        let detail = row["detail"].as_str().expect("detail");
        assert!(detail.contains("entries="), "审计行必须记清单数: {detail}");
    }
}

/// verify 独立 (只验不恢复) + 坏包拒绝即帧 + 导出不覆盖已有 bundle。
#[test]
fn cli_verify_is_read_only_and_refuses_corruption_and_overwrite() {
    let root = temp_dir("verify");
    let data = root.path().join("data");
    let paths = seed_data_root(&data);
    let bundle = root.path().join("bundle");
    run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).expect("export");

    let summary =
        run_backup_at(&paths, &args(&["verify", bundle.to_str().unwrap()])).expect("verify");
    assert!(summary.contains("ok entries="), "{summary}");
    // verify 零审计行 (只验不恢复, 如实注)。
    let audit_text = std::fs::read_to_string(data.join("daemon-audit.jsonl")).unwrap_or_default();
    assert!(
        !audit_text.contains("backup.verify"),
        "verify 是只读校验, 不记恢复/导出审计行"
    );

    // 坏包: verify 与 restore 都拒绝即帧。
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(bundle.join("manifest.json")).expect("manifest"))
            .expect("json");
    let victim_rel = manifest["files"][0]["path"].as_str().expect("path");
    std::fs::write(bundle.join(rel(victim_rel)), b"tampered").expect("tamper");
    assert!(
        run_backup_at(&paths, &args(&["verify", bundle.to_str().unwrap()])).is_err(),
        "坏包必须拒绝"
    );
    assert!(
        run_backup_at(
            &paths,
            &args(&[
                "restore",
                bundle.to_str().unwrap(),
                "--to",
                root.path().join("out").to_str().unwrap()
            ])
        )
        .is_err(),
        "坏包必须拒恢复 (宁拒不损)"
    );

    // 不覆盖: 已有 bundle 目标拒绝即帧。
    assert!(
        run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).is_err(),
        "已有 bundle 不得覆盖"
    );

    // 参数面: 用法帧 (错误即帧)。
    let error = run_backup_at(&paths, &args(&["export"])).expect_err("missing dir");
    assert!(error.contains(USAGE), "{error}");
}

/// 凭据不随行 (CLI 面同口径): 数据根上的密钥/令牌面零随行。
#[test]
fn cli_credentials_stay_out_of_the_bundle() {
    let root = temp_dir("creds");
    let data = root.path().join("data");
    let paths = seed_data_root(&data);
    write(
        &data.join("creds.json"),
        br#"{"api_key":"SECRET-CREDS-CLI"}"#,
    );
    write(
        &data.join("client-tokens.json"),
        br#"{"token":"SECRET-TOKEN-CLI"}"#,
    );
    write(&data.join("apeireth-keyring.bin"), b"SECRET-KEYRING-CLI");
    let bundle = root.path().join("bundle");
    run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).expect("export");

    let manifest_text =
        std::fs::read_to_string(bundle.join("manifest.json")).expect("manifest text");
    for secret in ["SECRET-CREDS-CLI", "SECRET-TOKEN-CLI", "SECRET-KEYRING-CLI"] {
        assert!(!manifest_text.contains(secret), "打包内零密钥: {secret}");
    }
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).expect("json");
    for entry in manifest["files"].as_array().expect("files") {
        let path = entry["path"].as_str().expect("path");
        for forbidden in ["creds.json", "client-tokens.json", "apeireth-keyring"] {
            assert!(!path.contains(forbidden), "凭据件不得进包: {path}");
        }
    }
}

/// self_status 备份节真值来源: CLI 导出落 `last-backup.json`, 探测口同源读真值。
#[test]
fn self_status_backup_probe_reads_the_cli_ledger() {
    let root = temp_dir("status");
    let data = root.path().join("data");
    let paths = seed_data_root(&data);
    let bundle = root.path().join("bundle");

    // 尚无备份: 显式 null + 原因 (不编时间不编件数)。
    let probe = apeireth_runtime_assembly::backup_status_probe(backup_ledger_path(&data));
    let status = probe().expect("probe");
    assert_eq!(status.last_backup_at_ms, None);
    assert_eq!(status.manifest_entries, None);
    assert!(status.reason.is_some());

    run_backup_at(&paths, &args(&["export", bundle.to_str().unwrap()])).expect("export");
    let status = probe().expect("probe after export");
    assert!(
        status.last_backup_at_ms.unwrap_or(0) > 0,
        "最近备份时间真值"
    );
    assert_eq!(status.manifest_entries, Some(10), "七类 10 件的清单数真值");
    assert_eq!(status.reason, None);
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
