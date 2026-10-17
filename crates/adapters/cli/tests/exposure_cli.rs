//! V5-b 暴露面体检 CLI 面的证据: 报告样例 (文本/JSON 合同) / 未测如实标注 /
//! W3-② 绑定不一致进发现 / 只读零写 (盘面零变化) / 用法帧。
//!
//! 口径: fixture 落临时目录; 体检只读, `--out` 是唯一写点且只写报告文件。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_cli::exposure::{run_exposure_at, run_exposure_on, USAGE};
use apeireth_cli::user_admin::run_user_admin_at;
use apeireth_core::stored_doc::StoredDoc;
use apeireth_gateway::{
    ClientTokenConfig, ClientTokenSpec, CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION,
    CLIENT_TOKENS_FILE,
};
use apeireth_tools_canonical::exposure_audit::{
    parse_netstat_snapshot, BindingSurface, ExposureAuditConfig, ListenerSnapshot,
};

const FIXTURE_NETSTAT: &str = "\
Active Connections\n\
\n\
  Proto  Local Address          Foreign Address        State           PID\n\
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       100\n\
  TCP    127.0.0.1:3080         0.0.0.0:0              LISTENING       200\n";

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-exposure-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

fn fixture_config(root: &Path) -> ExposureAuditConfig {
    ExposureAuditConfig::new(root)
        .with_listeners(Arc::new(|| {
            Ok(ListenerSnapshot {
                listeners: parse_netstat_snapshot(FIXTURE_NETSTAT).listeners,
                unparsed_rows: 0,
                duplicate_rows: 0,
            })
        }))
        .with_bindings(Arc::new(|| BindingSurface::Checked {
            clients: 1,
            users: 1,
            problems: Vec::new(),
        }))
}

/// 写一份客户端令牌表 (`client-tokens.json`, StoredDoc 信封)。
fn write_tokens(dir: &Path, entries: &[(&str, &str, Option<&str>)]) {
    let clients = entries
        .iter()
        .map(|(name, token, user_id)| ClientTokenSpec {
            name: (*name).to_string(),
            token: (*token).to_string(),
            user_id: user_id.map(str::to_string),
        })
        .collect();
    let doc = StoredDoc::new(
        CLIENT_TOKENS_DOC_NAME,
        CLIENT_TOKENS_DOC_VERSION,
        vec![CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(clients),
    );
    fs_err::write(
        dir.join(CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

/// 盘面快照 (相对路径 + 字节): 只读零写的比对基准。
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<fs_err::DirEntry> = fs_err::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, out);
                continue;
            }
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            out.push((rel, fs_err::read(&path).unwrap()));
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out
}

/// 文本报告: 六项齐、每项带建议、外网面与 (无包时) 备份面显式「未测」。
#[test]
fn report_text_lists_items_with_advice_and_honest_unmeasured_labels() {
    let dir = temp_dir("text");
    fs_err::write(dir.join("creds.json"), br#"{"token":"never-echo-1"}"#).unwrap();
    let output = run_exposure_on(&fixture_config(&dir), &args(&["report"])).expect("report");
    assert!(output.contains("暴露面体检报告"), "{output}");
    assert!(output.contains("只读零写"), "口径写明只读零写");
    for (index, id) in [
        "credential_files",
        "plaintext_points",
        "listening_ports",
        "backup_exclusions",
        "token_bindings",
        "external_surface",
    ]
    .iter()
    .enumerate()
    {
        assert!(
            output.contains(id),
            "第 {} 项 {id} 要在报告里:\n{output}",
            index + 1
        );
    }
    assert!(output.contains("未测"), "未测要喊破");
    assert!(output.contains("建议:"), "每项整改建议一句");
    assert!(output.contains("总评:"), "总评在尾");
    assert!(!output.contains("never-echo-1"), "凭据值零明文: {output}");
}

/// JSON 合同经 CLI 出口逐字成立 (`--json`), 且可反序列化留档可 diff。
#[test]
fn report_json_round_trips_through_the_cli_face() {
    let dir = temp_dir("json");
    fs_err::write(
        dir.join("app-config.toml"),
        b"api_key = \"sk-never-echo-2\"\n",
    )
    .unwrap();
    let output = run_exposure_on(&fixture_config(&dir), &args(&["report", "--json"]))
        .expect("report --json");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    assert_eq!(value["schema"], "apeireth.exposure_report");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["read_only"], true);
    let items = value["items"].as_array().expect("items array");
    assert_eq!(items.len(), 6);
    assert!(items.iter().all(|item| item["advice"].is_string()));
    assert_eq!(
        items[5]["status"].as_str().unwrap(),
        "not_measured",
        "外网暴露面显式未测"
    );
    assert!(!output.contains("sk-never-echo-2"), "明文密钥值零明文");
}

/// W3-② 真判定链: 令牌↔用户绑定不一致 (悬空 user_id) 以高风险发现进报告。
#[test]
fn binding_inconsistency_from_the_real_gate_becomes_a_high_risk_finding() {
    let dir = temp_dir("binding");
    // 发令牌序逆用 (fixture 故意造病): 用户表只有 owner-1/tui, 令牌却声明悬空 user_id。
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    write_tokens(
        &dir,
        &[("tui", "tok-tui-0123456789abcdef", Some("ghost-user"))],
    );

    let output = run_exposure_at(&dir, &args(&["report", "--json"])).expect("report");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    let items = value["items"].as_array().expect("items array");
    let binding = items
        .iter()
        .find(|item| item["id"] == "token_bindings")
        .expect("token_bindings item");
    assert_eq!(binding["status"], "measured");
    assert_eq!(binding["risk"], "high", "问责链断评高风险: {binding}");
    assert_eq!(binding["finding_count"], 1);
    let rendered = binding.to_string();
    assert!(
        rendered.contains("ghost-user"),
        "发现要指名悬空绑定: {rendered}"
    );
    assert!(
        !rendered.contains("tok-tui-0123456789abcdef"),
        "令牌值零明文"
    );
}

/// 一致的绑定面零发现 (干净面零误报, 走真判定链)。
#[test]
fn consistent_bindings_yield_zero_findings_on_the_real_gate() {
    let dir = temp_dir("binding-clean");
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    write_tokens(
        &dir,
        &[("tui", "tok-tui-0123456789abcdef", Some("owner-1"))],
    );

    let output = run_exposure_at(&dir, &args(&["report", "--json"])).expect("report");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
    let binding = value["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "token_bindings")
        .expect("token_bindings item");
    assert_eq!(binding["finding_count"], 0, "{binding}");
    assert_eq!(binding["risk"], "none");
}

/// 只读零写: CLI 体检跑完盘面零变化; `--out` 只写报告文件本身。
#[test]
fn cli_report_is_read_only_and_out_writes_only_the_report_file() {
    let dir = temp_dir("zero-write");
    fs_err::write(dir.join("creds.json"), br#"{"token":"never-echo-3"}"#).unwrap();
    fs_err::write(dir.join("users.json"), br#"{"users":[]}"#).unwrap();
    let before = snapshot(&dir);

    let out_path = dir.join("report.txt");
    let output = run_exposure_on(&fixture_config(&dir), &args(&["report", "--out"]))
        .expect_err("--out 需要值");
    assert!(output.contains("Usage:"), "{output}");
    let output = run_exposure_on(
        &fixture_config(&dir),
        &args(&["report", "--out", out_path.to_str().unwrap()]),
    )
    .expect("report --out");
    assert!(output.contains("report written to"), "{output}");
    assert!(fs_err::read_to_string(&out_path)
        .unwrap()
        .contains("暴露面体检报告"));

    // 盘面零变化: 除用户显式指定的报告文件外, 被测面一个字节都没动。
    let before_map: std::collections::BTreeMap<PathBuf, Vec<u8>> = before.into_iter().collect();
    let after_map: std::collections::BTreeMap<PathBuf, Vec<u8>> =
        snapshot(&dir).into_iter().collect();
    assert_eq!(after_map.len(), before_map.len() + 1, "只多了报告文件");
    for (rel, bytes) in &before_map {
        assert_eq!(after_map.get(rel), Some(bytes), "{rel:?} 不得变");
    }
    assert!(after_map.contains_key(&PathBuf::from("report.txt")));
}

/// 用法帧: 未知动词/未知旗标一律拒, 错误即帧带用法。
#[test]
fn unknown_verb_and_flags_refuse_with_the_usage_frame() {
    let dir = temp_dir("usage");
    for args in [
        args(&["verify"]),
        args(&["report", "--nope"]),
        args(&["report", "--json", "extra"]),
    ] {
        let error = run_exposure_at(&dir, &args).expect_err("must refuse");
        assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    }
}
