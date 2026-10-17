//! V5-b 暴露面体检报告的集成证据: 各扫描项真数据面发现 / 干净面零误报 /
//! 评分与建议输出 / 只读零写 (盘面零变化断言) / JSON 合同 / 未测如实标注 /
//! 防误报 (fixture 面)。
//!
//! 口径: 全部 fixture 落临时目录, 体检全程只读; 凭据值零采集 —— 每个涉密
//! fixture 都带「不得出现在报告里」断言 (零明文留痕)。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_tools_canonical::exposure_audit::{
    listener_item, parse_netstat_snapshot, run_exposure_audit_at, scan_credential_files,
    scan_plaintext_points, token_binding_item, BindingSurface, ExposureAuditConfig, ExposureItem,
    ExposureReport, ItemStatus, ListenerScope, ListenerSnapshot, RiskLevel, REPORT_SCHEMA,
    REPORT_SCHEMA_VERSION,
};

/// 干净探测口: 空监听面 + 已核对一致的绑定面 (fixture 面防误报的基线)。
fn clean_config(root: &Path) -> ExposureAuditConfig {
    ExposureAuditConfig::new(root)
        .with_listeners(Arc::new(|| {
            Ok(ListenerSnapshot {
                listeners: Vec::new(),
                unparsed_rows: 0,
                duplicate_rows: 0,
            })
        }))
        .with_bindings(Arc::new(|| BindingSurface::Checked {
            clients: 2,
            users: 2,
            problems: Vec::new(),
        }))
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// 真数据面 fixture: 凭据件 + 密钥材料 + 明文密钥配置 + 干净件混布。
fn seed_dirty_data_root(root: &Path) {
    write(
        &root.join("creds.json"),
        r#"{"token":"super-secret-value-1"}"#,
    );
    write(
        &root.join("server.pem"),
        "PRIVATE KEY MATERIAL secret-value-2",
    );
    fs::create_dir_all(root.join(".ssh")).unwrap();
    write(&root.join(".ssh").join("config"), "ignored");
    write(&root.join("client-tokens.json"), r#"{"clients":[]}"#);
    write(&root.join("im-channels.json"), r#"{"channels":[]}"#);
    write(
        &root.join("app-config.toml"),
        "name = \"apeireth\"\napi_key = \"sk-live-must-not-echo-3\"\n",
    );
    write(&root.join("notes.json"), r#"{"name":"plain","timeout":30}"#);
    write(&root.join("users.json"), r#"{"users":[]}"#);
}

/// 盘面快照 (相对路径 + 大小 + mtime + 字节): 只读零写的比对基准。
fn snapshot(root: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime, Vec<u8>)> {
    fn collect(
        root: &Path,
        dir: &Path,
        out: &mut Vec<(PathBuf, u64, std::time::SystemTime, Vec<u8>)>,
    ) {
        let mut entries: Vec<fs::DirEntry> =
            fs::read_dir(dir).unwrap().filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, out);
                continue;
            }
            let meta = fs::metadata(&path).unwrap();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            out.push((
                rel,
                meta.len(),
                meta.modified().unwrap(),
                fs::read(&path).unwrap(),
            ));
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out
}

// ── 项一: 凭据文件分布 ────────────────────────────────────────────────

/// 真数据面发现: 凭据件/密钥材料逐条命中, 密钥材料评高风险, 值零采集。
#[test]
fn credential_surface_finds_fixture_hits_and_rates_key_material() {
    let dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dir.path());
    let item = scan_credential_files(dir.path());
    assert_eq!(item.id, "credential_files");
    assert!(item.finding_count() >= 5, "findings: {:?}", item.findings);

    let surfaces: Vec<&str> = item
        .findings
        .iter()
        .map(|finding| finding.surface.as_str())
        .collect();
    for expected in [
        "creds.json",
        "server.pem",
        "client-tokens.json",
        "im-channels.json",
    ] {
        assert!(
            surfaces.contains(&expected),
            "missing {expected} in {surfaces:?}"
        );
    }
    assert!(
        item.findings
            .iter()
            .any(|finding| finding.surface == ".ssh" && finding.risk == RiskLevel::High),
        "密钥目录评高风险: {:?}",
        item.findings
    );
    assert_eq!(item.risk(), Some(RiskLevel::High));
    assert!(item.score().unwrap() < 100, "有发现就要扣分");
    assert!(
        item.advice.contains("收口"),
        "整改建议一句: {}",
        item.advice
    );

    // 零明文留痕: fixture 里的秘密值绝不出现在渲染输出里。
    let report_text = format!("{item:?}");
    assert!(
        !report_text.contains("super-secret-value-1"),
        "{report_text}"
    );
}

/// 干净面零误报: 普通项目件/用户表/配置不命中凭据面。
#[test]
fn credential_surface_clean_tree_has_zero_findings() {
    let dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dir.path());
    // 清走全部凭据件, 只留干净面。
    for rel in [
        "creds.json",
        "server.pem",
        "client-tokens.json",
        "im-channels.json",
    ] {
        fs::remove_file(dir.path().join(rel)).unwrap();
    }
    fs::remove_dir_all(dir.path().join(".ssh")).unwrap();
    let item = scan_credential_files(dir.path());
    assert_eq!(item.finding_count(), 0, "findings: {:?}", item.findings);
    assert_eq!(item.risk(), Some(RiskLevel::None));
    assert_eq!(item.score(), Some(100));
}

// ── 项二: 明文风险点 ─────────────────────────────────────────────────

/// 明文风险点: 密钥字段计数命中, 但字段值**永不出报告** (防泄漏断言)。
#[test]
fn plaintext_points_find_secret_fields_but_never_echo_values() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(
        &dir.path().join("app-config.toml"),
        "name = \"apeireth\"\napi_key = \"sk-live-must-not-echo-3\"\npassword: hunter2-echo-4\n",
    );
    let item = scan_plaintext_points(dir.path());
    assert_eq!(item.finding_count(), 1, "findings: {:?}", item.findings);
    assert_eq!(item.findings[0].surface, "app-config.toml");
    assert_eq!(item.findings[0].risk, RiskLevel::Medium);
    assert!(
        item.findings[0].detail.contains("2 个密钥字段"),
        "计数口径: {}",
        item.findings[0].detail
    );
    let rendered = format!("{item:?}");
    for secret in ["sk-live-must-not-echo-3", "hunter2-echo-4"] {
        assert!(!rendered.contains(secret), "值泄漏: {rendered}");
    }
}

/// 明文风险点干净面: 无密钥字段的配置零发现 (防误报)。
#[test]
fn plaintext_points_clean_config_has_zero_findings() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(
        &dir.path().join("notes.json"),
        r#"{"name":"plain","timeout":30}"#,
    );
    write(&dir.path().join("users.json"), r#"{"users":[]}"#);
    write(&dir.path().join("readme.md"), "# title\n\nbody: text\n");
    let item = scan_plaintext_points(dir.path());
    assert_eq!(item.finding_count(), 0, "findings: {:?}", item.findings);
    assert_eq!(item.score(), Some(100));
}

// ── 项三: 开放端口面 ─────────────────────────────────────────────────

/// fixture 面防误报: 全接口监听评中风险、回环监听评无风险、连接态行不算、
/// 重复行合并计数 (不虚增发现数)。
#[test]
fn listener_fixture_classifies_scopes_and_ignores_connection_rows() {
    let text = "\
Active Connections\n\
\n\
  Proto  Local Address          Foreign Address        State           PID\n\
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       100\n\
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       100\n\
  TCP    127.0.0.1:3080         0.0.0.0:0              LISTENING       200\n\
  TCP    192.168.1.5:7777       0.0.0.0:0              LISTENING       300\n\
  TCP    127.0.0.1:3080         127.0.0.1:50000        ESTABLISHED     200\n\
  UDP    0.0.0.0:5353           *:*                                    400\n";
    let snapshot = parse_netstat_snapshot(text);
    assert_eq!(
        snapshot.listeners.len(),
        4,
        "连接态行不得计入, 重复行只留一行: {:?}",
        snapshot.listeners
    );
    assert_eq!(snapshot.unparsed_rows, 0);
    assert_eq!(snapshot.duplicate_rows, 1, "重复行如实计数");

    let item = listener_item(Ok(snapshot));
    assert_eq!(item.finding_count(), 4);
    assert!(item.note.as_deref().unwrap().contains("合并 1 行"));
    assert_eq!(item.risk(), Some(RiskLevel::Medium), "全接口监听在列");
    assert!(
        item.findings
            .iter()
            .any(|finding| finding.surface.starts_with("tcp 0.0.0.0:135")
                && finding.risk == RiskLevel::Medium),
        "全接口监听评中风险: {:?}",
        item.findings
    );
    assert!(
        item.findings
            .iter()
            .any(|finding| finding.surface.starts_with("tcp 127.0.0.1:3080")
                && finding.risk == RiskLevel::None),
        "回环监听评无风险: {:?}",
        item.findings
    );
    assert!(
        item.findings
            .iter()
            .any(|finding| finding.surface.starts_with("udp 0.0.0.0:5353")),
        "已绑定 UDP 也算监听面: {:?}",
        item.findings
    );
    assert!(
        item.note.as_deref().unwrap().contains("未测"),
        "外网可达性未测要喊破"
    );
}

/// 干净面零误报: 只有回环监听时风险无、评分满。
#[test]
fn listener_loopback_only_surface_is_clean() {
    let text = "\
  Proto  Local Address          Foreign Address        State           PID\n\
  TCP    127.0.0.1:3080         0.0.0.0:0              LISTENING       200\n";
    let item = listener_item(Ok(parse_netstat_snapshot(text)));
    assert_eq!(item.risk(), Some(RiskLevel::None));
    assert_eq!(item.score(), Some(100));

    // 采集失败 = 显式未测, 不冒充干净。
    let unavailable = listener_item(Err("probe refused".to_string()));
    assert_eq!(unavailable.status, ItemStatus::NotMeasured);
    assert!(unavailable.note.unwrap().contains("probe refused"));
}

// ── 项四: 备份包排除项核对 ───────────────────────────────────────────

fn manifest_json(files: &[&str], excluded: &[(&str, &str)]) -> String {
    let files_json: Vec<String> = files
        .iter()
        .map(|path| {
            format!(
                r#"{{"category":"users","path":"{path}","origin":"C:/x/{path}","relpath":"{path}","bytes":1,"sha256":"00","mtime_ms":null}}"#
            )
        })
        .collect();
    let excluded_json: Vec<String> = excluded
        .iter()
        .map(|(path, reason)| format!(r#"{{"path":"{path}","reason":"{reason}"}}"#))
        .collect();
    format!(
        r#"{{"format_version":1,"product_version":"fixture","created_at_ms":0,"data_root":"C:/data","files":[{}],"excluded":[{}]}}"#,
        files_json.join(","),
        excluded_json.join(",")
    )
}

/// 干净备份包: 清单行 + 包内文件都不命中凭据面, 显式排除行带原因 → 零发现。
#[test]
fn backup_bundle_clean_exclusions_pass() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    write(
        &bundle.join("manifest.json"),
        &manifest_json(
            &["files/users/users.json", "files/skills/card.json"],
            &[(
                "creds.json",
                "credential artifact (explicit exclusion list)",
            )],
        ),
    );
    write(&bundle.join("files/users/users.json"), r#"{"users":[]}"#);
    write(&bundle.join("files/skills/card.json"), r#"{"skill":1}"#);

    let item = apeireth_tools_canonical::exposure_audit::backup_exclusion_item(Some(&bundle));
    assert_eq!(item.finding_count(), 0, "findings: {:?}", item.findings);
    assert_eq!(item.risk(), Some(RiskLevel::None));
    assert_eq!(item.score(), Some(100));
    assert!(item.note.unwrap().contains("已核对"));
}

/// 凭据混入备份包 = 高风险发现 (清单面与包内文件面两双眼都逮住)。
#[test]
fn backup_bundle_with_credential_artifact_is_high_risk() {
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    write(
        &bundle.join("manifest.json"),
        &manifest_json(
            &["files/users/users.json", "files/users/client-tokens.json"],
            &[(
                "creds.json",
                "credential artifact (explicit exclusion list)",
            )],
        ),
    );
    write(&bundle.join("files/users/users.json"), r#"{"users":[]}"#);
    write(
        &bundle.join("files/users/client-tokens.json"),
        r#"{"clients":[]}"#,
    );
    // 清单说谎场景: 清单没记, 但包里躺着密钥材料。
    write(&bundle.join("files/skills/extra.pem"), "not in manifest");

    let item = apeireth_tools_canonical::exposure_audit::backup_exclusion_item(Some(&bundle));
    let surfaces: Vec<&str> = item
        .findings
        .iter()
        .map(|finding| finding.surface.as_str())
        .collect();
    assert!(
        surfaces
            .iter()
            .any(|surface| surface.contains("client-tokens.json")),
        "清单行命中: {surfaces:?}"
    );
    assert!(
        surfaces.iter().any(|surface| surface.contains("extra.pem")),
        "包内实文件也要逮住 (清单可能说谎): {surfaces:?}"
    );
    assert_eq!(item.risk(), Some(RiskLevel::High));
    // 三条发现 = 清单面 + 包内面两双眼各指认一次 client-tokens.json + extra.pem
    // (同一泄漏两面命中是核对语义本身, 不去重成假干净)。
    assert_eq!(item.finding_count(), 3);
    assert_eq!(item.score(), Some(25));
}

// ── 项五: 客户端令牌表绑定完整性 ─────────────────────────────────────

/// 绑定不一致 = 问责链断 = 高风险, 每条问题一条发现 (W3-② 判定结果呈现)。
#[test]
fn token_binding_inconsistency_is_high_risk_and_consistent_is_clean() {
    let broken = token_binding_item(&BindingSurface::Checked {
        clients: 2,
        users: 1,
        problems: vec![
            "client \"tui\" declares user id \"ghost\", which is not in the user table".to_string(),
        ],
    });
    assert_eq!(broken.finding_count(), 1);
    assert_eq!(broken.risk(), Some(RiskLevel::High));
    assert!(
        broken.advice.contains("发令牌序"),
        "整改建议一句: {}",
        broken.advice
    );

    let clean = token_binding_item(&BindingSurface::Checked {
        clients: 2,
        users: 2,
        problems: Vec::new(),
    });
    assert_eq!(clean.finding_count(), 0);
    assert_eq!(clean.score(), Some(100));

    let absent = token_binding_item(&BindingSurface::Absent {
        reason: "用户表未配置".to_string(),
    });
    assert_eq!(absent.status, ItemStatus::NotMeasured);
}

// ── 报告整体: JSON 合同 / 未测标注 / 评分 / 只读零写 ─────────────────

/// JSON 合同: 顶层键 + 六项固定顺序 + 每项 (发现数/风险/评分/建议) 全在。
#[test]
fn json_contract_pins_schema_fields_and_item_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dir.path());
    let bundle = dir.path().join("bundle");
    write(
        &bundle.join("manifest.json"),
        &manifest_json(
            &["files/users/users.json"],
            &[("creds.json", "excluded: 凭据不随行")],
        ),
    );
    write(&bundle.join("files/users/users.json"), r#"{"users":[]}"#);
    let report = run_exposure_audit_at(
        &clean_config(dir.path()).with_bundle(&bundle),
        1_760_000_000_123,
    );
    let value = report.to_json_value();

    assert_eq!(value["schema"], REPORT_SCHEMA);
    assert_eq!(value["schema_version"], REPORT_SCHEMA_VERSION);
    assert_eq!(value["read_only"], true);
    assert_eq!(value["generated_at_ms"], 1_760_000_000_123i64);
    let items = value["items"].as_array().expect("items array");
    assert_eq!(items.len(), 6);
    let ids: Vec<&str> = items
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "credential_files",
            "plaintext_points",
            "listening_ports",
            "backup_exclusions",
            "token_bindings",
            "external_surface",
        ]
    );
    for item in items {
        assert!(item["title"].is_string());
        assert!(item["finding_count"].is_number());
        assert!(item["advice"].is_string());
        assert!(
            !item["advice"].as_str().unwrap().trim().is_empty(),
            "整改建议必有"
        );
        assert!(["measured", "not_measured"].contains(&item["status"].as_str().unwrap()));
    }
    let overall = &value["overall"];
    assert!(overall["score"].is_number());
    assert!(overall["grade"].is_string());
    assert!(overall["verdict"].is_string());
    assert_eq!(overall["measured_items"], 5);
    assert_eq!(overall["not_measured_items"], 1);

    // JSON 与文本双渲染都零明文 (凭据值/密钥值不进报告)。
    let json = report.to_json();
    let text = report.to_text();
    for secret in [
        "super-secret-value-1",
        "sk-live-must-not-echo-3",
        "secret-value-2",
    ] {
        assert!(!json.contains(secret), "JSON 值泄漏: {secret}");
        assert!(!text.contains(secret), "文本值泄漏: {secret}");
    }
}

/// 未测如实标注: 外网暴露面恒未测; 无备份包时备份项也未测 (不冒充干净)。
#[test]
fn unmeasured_items_are_labeled_honestly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let report = run_exposure_audit_at(&clean_config(dir.path()), 0);
    let unmeasured: Vec<&ExposureItem> = report
        .items
        .iter()
        .filter(|item| item.status == ItemStatus::NotMeasured)
        .collect();
    let ids: Vec<&str> = unmeasured.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, vec!["backup_exclusions", "external_surface"]);
    for item in unmeasured {
        assert_eq!(item.score(), None);
        assert_eq!(item.risk(), None);
        let note = item.note.as_deref().unwrap();
        assert!(note.contains("未测"), "未测原因要喊破: {note}");
    }
    let text = report.to_text();
    assert!(text.contains("未测"), "文本报告要见「未测」字样");
    assert!(text.contains("不计入总评"), "口径要写明未测不计入总评");
}

/// 评分与总评: 脏面 fixture 的总评要低于干净面, 且结论点名高风险。
#[test]
fn overall_score_reflects_fixture_findings() {
    let clean_dir = tempfile::tempdir().expect("temp dir");
    let clean_report = run_exposure_audit_at(&clean_config(clean_dir.path()), 0);
    assert_eq!(clean_report.overall.score, 100);
    assert_eq!(clean_report.overall.grade, "优");

    let dirty_dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dirty_dir.path());
    let mut config = clean_config(dirty_dir.path());
    config = config.with_listeners(Arc::new(|| {
        Ok(ListenerSnapshot {
            listeners: parse_netstat_snapshot(
                "  TCP    0.0.0.0:4444          0.0.0.0:0              LISTENING       9\n",
            )
            .listeners,
            unparsed_rows: 0,
            duplicate_rows: 0,
        })
    }));
    config = config.with_bindings(Arc::new(|| BindingSurface::Checked {
        clients: 1,
        users: 0,
        problems: vec!["client \"tui\" declares user id \"ghost\"".to_string()],
    }));
    let dirty_report = run_exposure_audit_at(&config, 0);
    assert!(
        dirty_report.overall.score < clean_report.overall.score,
        "脏面总评必须更低: {} vs {}",
        dirty_report.overall.score,
        clean_report.overall.score
    );
    assert!(
        dirty_report.overall.verdict.contains("高风险"),
        "结论点名高风险: {}",
        dirty_report.overall.verdict
    );
    assert_ne!(dirty_report.overall.grade, "优");
}

/// 报告样例 (可复跑): fixture 面跑出的文本报告, 六节 + 分项评分 + 总评全在;
/// 全文随 `--nocapture` 打印留档 (样例即证据)。
#[test]
fn report_sample_text_renders_every_section() {
    let dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dir.path());
    let bundle = dir.path().join("bundle");
    write(
        &bundle.join("manifest.json"),
        &manifest_json(
            &["files/users/users.json", "files/users/client-tokens.json"],
            &[(
                "creds.json",
                "credential artifact (explicit exclusion list)",
            )],
        ),
    );
    write(&bundle.join("files/users/users.json"), r#"{"users":[]}"#);
    let mut config = clean_config(dir.path()).with_bundle(&bundle);
    config = config.with_listeners(Arc::new(|| {
        Ok(ListenerSnapshot {
            listeners: parse_netstat_snapshot(
                "  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       100\n\
                 TCP    127.0.0.1:3080         0.0.0.0:0              LISTENING       200\n",
            )
            .listeners,
            unparsed_rows: 0,
            duplicate_rows: 0,
        })
    }));
    let report = run_exposure_audit_at(&config, 1_760_000_000_123);
    let text = report.to_text();
    for section in [
        "凭据文件分布 (credential_files)",
        "明文风险点 (plaintext_points)",
        "开放端口面 (本机监听) (listening_ports)",
        "备份包排除项核对 (backup_exclusions)",
        "客户端令牌表绑定完整性 (token_bindings)",
        "外网暴露面 (公网入口可达性) (external_surface)",
    ] {
        assert!(text.contains(section), "缺节 {section}:\n{text}");
    }
    assert!(text.contains("建议:"), "每项整改建议一句");
    assert!(text.contains("总评:"), "总评在尾");
    assert!(text.contains("评分"), "分项评分在列");
    assert!(text.contains("未测"), "未测如实标注");
    println!("{text}");
}

/// 只读零写: 体检跑完盘面零变化 (路径集/大小/mtime/字节全等)。
#[test]
fn audit_is_read_only_zero_write_on_the_data_surface() {
    let dir = tempfile::tempdir().expect("temp dir");
    seed_dirty_data_root(dir.path());
    // 备份包也一起核对 (包面同样零写)。
    let bundle = dir.path().join("bundle");
    write(
        &bundle.join("manifest.json"),
        &manifest_json(&["files/users/users.json"], &[("creds.json", "excluded")]),
    );
    write(&bundle.join("files/users/users.json"), r#"{"users":[]}"#);

    let before = snapshot(dir.path());
    let mut config = clean_config(dir.path()).with_bundle(&bundle);
    config = config.with_bindings(Arc::new(|| BindingSurface::Checked {
        clients: 1,
        users: 1,
        problems: Vec::new(),
    }));
    let report: ExposureReport = run_exposure_audit_at(&config, 0);
    // 报告本身也只回话: 渲染不触碰盘面。
    let _ = report.to_text();
    let _ = report.to_json();
    let after = snapshot(dir.path());

    assert_eq!(before.len(), after.len(), "盘面文件数不得变");
    for (left, right) in before.iter().zip(after.iter()) {
        assert_eq!(left.0, right.0, "路径不得变");
        assert_eq!(left.1, right.1, "大小不得变");
        assert_eq!(left.2, right.2, "mtime 不得变");
        assert_eq!(left.3, right.3, "字节不得变");
    }
}
