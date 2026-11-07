#![no_main]

//! fuzz: **暴露面体检面（`exposure_audit`）** —— 三条硬语义的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/exposure_audit/`
//! （`run_exposure_audit` + `credential_surface` / `backup_surface` / `listener_surface` /
//! `binding_surface` / `report`）。
//!
//! ## 为什么值得 fuzz（这个面是**隐私面**）
//!
//! 它回答的是「**你暴露了什么**」⇒ ⇒ 它自己的输出**不能成为新的暴露面**。
//! 模块头写死的三条硬语义：
//! 1. **只读零写** —— 体检**不写被测面任何字节**（测试钉死）；
//! 2. **诚实口径** —— 只测能测的本地面；不能测的**显式「未测」**，
//!    **未测不计入总评、不冒充干净**；
//! 3. **零明文留痕** —— **凭据值/密钥值永不出现在报告里**，只报路径词面与计数。
//!
//! ## 判据纪律（每条断言都从上面三条推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **零明文留痕**：报告（JSON **与文本**）**不得出现**任何被植入的机密值 | 零明文留痕 |
//! | O2 | **诚实口径**：未测项 ⇒ `findings` **恒空** + `note` **必填**；`已测数 + 未测数 == 项数` | 诚实口径 |
//! | O3 | **未测不计入总评**：`overall.score` **等于**已测项评分的算术均值（**可重算**） | 诚实口径 |
//! | O4 | **只读零写**：跑完体检，数据根的**文件集合与字节**必须**逐字节不变** | 只读零写 |
//! | O5 | **确定性**：同样输入 + 同样注入时钟 ⇒ 报告**逐字节相同** | 报告要可留档可 diff |
//! | O6 | **合同稳定**：JSON 里 `read_only == true` + `schema`/`schema_version` 是契约值 | 报告合同 |
//!
//! ## 诚实边界
//!
//! - **注入两个探测口**（监听面 / 绑定面）⇒ 探针的**数据**由输入决定（这就是不可信输入那一维）；
//! - **不测** CLI 出口与 `funnel` 巡检层（另立面）；
//! - 备份包面只在**包存在**时才有内容（不存在 ⇒ 该项显式「未测」⇒ 正好压 O2/O3）。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

use apeireth_tools_canonical::exposure_audit::{
    run_exposure_audit_at, BindingProbe, BindingSurface, ExposureAuditConfig, Listener,
    ListenerProbe, ListenerScope, ListenerSnapshot, REPORT_SCHEMA, REPORT_SCHEMA_VERSION,
};

/// 文件名池：**刻意混入凭据面名字**（好压 O1/O2）。
const NAME_POOL: &[&str] = &[
    "notes.md",
    "config.toml",
    ".env",
    "id_rsa",
    "creds.json",
    "secrets.yaml",
    "app.json",
];

/// 目录快照（相对路径 → 字节）：用于「只读零写」的比对。
fn snapshot_dir(root: &Path) -> BTreeSet<(String, Vec<u8>)> {
    let mut out = BTreeSet::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<(String, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = fs::read(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.insert((rel, bytes));
            }
        }
    }
    walk(root, root, &mut out);
    out
}

/// 造一个形状怪异的监听面（IPv6 / 全接口 / 端口边界 / 回环）。
/// **不可信输入那一维**：监听面是**系统表快照**，形状可以很怪。
fn make_listener(index: usize) -> Option<Listener> {
    let scope = match index % 3 {
        0 => ListenerScope::Loopback,
        1 => ListenerScope::Wildcard,
        _ => ListenerScope::Interface,
    };
    let (proto, local_addr, port) = match index % 4 {
        0 => ("tcp".to_string(), "127.0.0.1".to_string(), 8080u16),
        1 => ("tcp6".to_string(), "::".to_string(), 443),
        2 => ("udp".to_string(), "0.0.0.0".to_string(), 65535),
        _ => ("tcp".to_string(), "192.168.1.10".to_string(), 0),
    };
    Some(Listener {
        proto,
        local_addr,
        port,
        scope,
        pid: if index % 2 == 0 { Some(4242) } else { None },
    })
}

fuzz_target!(|data: &[u8]| {
    let root = std::env::temp_dir().join(format!(
        "apeireth-exposure-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64))
    ));
    let data_root = root.join("data");
    let _ = fs::create_dir_all(&data_root);

    // ── 植入的机密标记（**只出现在文件内容里**；报告里**绝不能出现**）
    let marker: String = data
        .iter()
        .filter(|b| b.is_ascii_alphanumeric())
        .take(12)
        .map(|b| *b as char)
        .collect();
    if marker.len() < 6 {
        let _ = fs::remove_dir_all(&root);
        return;
    }
    let secret = format!("SECRETVALUE-{marker}");

    // ── 造数据根：文件池 + 任意字节内容（凭据面文件里植入机密值）
    let count = (data.first().copied().unwrap_or(0) as usize % 6) + 1;
    for index in 0..count {
        let name = NAME_POOL[index % NAME_POOL.len()];
        let rel = format!("sub{index}/{name}");
        let target = data_root.join(&rel);
        if let Some(parent) = target.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // 内容：既有常规文本，也有形如 key = "<机密>" 的**明文密钥字段**
        let body = format!(
            "api_key = \"{secret}\"\npassword = \"{secret}\"\nplain {}\n",
            String::from_utf8_lossy(data)
        );
        let _ = fs::write(&target, body);
    }

    // ── 注入两个探测口（数据由输入决定 ⇒ 这就是不可信输入那一维）
    let listener_count = (data.get(1).copied().unwrap_or(0) as usize) % 4;
    let snapshot = ListenerSnapshot {
        listeners: Vec::new(),
        unparsed_rows: (data.get(2).copied().unwrap_or(0) as usize) % 3,
        duplicate_rows: (data.get(3).copied().unwrap_or(0) as usize) % 3,
    };
    let mut snapshot = snapshot;
    for index in 0..listener_count {
        // 造几个形状怪异的监听行（IPv6 / 端口边界 / 重复）
        if let Some(listener) = make_listener(index) {
            snapshot.listeners.push(listener);
        }
    }
    let listeners: ListenerProbe = Arc::new(move || Ok(snapshot.clone()));
    let binding_mode = data.get(4).copied().unwrap_or(0) % 2;
    let bindings: BindingProbe = Arc::new(move || {
        if binding_mode == 0 {
            BindingSurface::Absent {
                reason: "测试注入：未配置用户表/令牌表".to_string(),
            }
        } else {
            BindingSurface::Checked {
                clients: 3,
                users: 2,
                // ⚠️ **这里绝不能植入机密值**（我的第一版植了 ⇒ 报告如实回显 ⇒ 报成"泄漏"）。
                // ⇒ 真相：「零明文留痕」的承诺对象是**被扫面**（文件内容），
                //    **不是任意注入的探针数据** —— 契约边界在**生产方**
                //    （绑定面保证「令牌值零采集」⇒ 它给什么，报告就如实报什么）。
                problems: vec!["绑定名不一致：client-a 与 user-b".to_string()],
            }
        }
    });

    // ── 备份包：有时存在，有时不存在（不存在 ⇒ 该项显式「未测」⇒ 压 O2/O3）
    let bundle = if data.get(5).copied().unwrap_or(0) % 2 == 0 {
        let bundle = root.join("bundle");
        let _ = fs::create_dir_all(&bundle);
        let _ = fs::write(
            bundle.join("manifest.json"),
            "{\"format_version\":1,\"files\":[],\"excluded\":[]}",
        );
        Some(bundle)
    } else {
        None
    };

    let config = ExposureAuditConfig {
        data_root: data_root.clone(),
        bundle,
        listeners,
        bindings,
    };

    // ── O4 只读零写：跑之前先快照
    let before = snapshot_dir(&data_root);

    let report = run_exposure_audit_at(&config, 1_700_000_000_000);

    // ── O4：跑完之后，数据根必须**逐字节不变**
    let after = snapshot_dir(&data_root);
    assert_eq!(
        before, after,
        "只读零写：体检不得写被测面任何字节（前后快照必须完全一致）"
    );

    let json = report.to_json();
    let text = report.to_text();

    // ── O1 零明文留痕：机密值**绝不能**出现在报告里
    assert!(
        !json.contains(&secret),
        "零明文留痕：报告 JSON 里出现了机密值原文（{secret}）"
    );
    assert!(
        !text.contains(&secret),
        "零明文留痕：报告文本里出现了机密值原文（{secret}）"
    );

    let value: Value = serde_json::from_str(&json).expect("报告 JSON 可解析");

    // ── O6 合同稳定
    assert_eq!(value["schema"].as_str(), Some(REPORT_SCHEMA), "schema 合同值");
    assert_eq!(
        value["schema_version"].as_u64(),
        Some(REPORT_SCHEMA_VERSION as u64),
        "schema_version 合同值"
    );
    assert_eq!(
        value["read_only"].as_bool(),
        Some(true),
        "只读档：报告必须声明 read_only = true"
    );

    let items = value["items"].as_array().expect("items 是数组").clone();

    // ── O2 诚实口径：未测项 ⇒ findings 恒空 + note 必填
    let mut measured = 0usize;
    let mut not_measured = 0usize;
    for item in &items {
        let status = item["status"].as_str().unwrap_or_default();
        match status {
            "measured" => {
                measured += 1;
                // 已测项的分数必须存在（未测项的 score 是 null）
                assert!(
                    item["score"].as_u64().is_some(),
                    "已测项必须有评分: {item}"
                );
            }
            "not_measured" => {
                not_measured += 1;
                assert!(
                    item["findings"].as_array().map(|f| f.is_empty()).unwrap_or(false),
                    "未测项的 findings 必须恒空（不冒充干净）: {item}"
                );
                let note = item["note"].as_str().unwrap_or_default();
                assert!(
                    !note.is_empty(),
                    "未测项必须写明未测原因（note 不得为空）: {item}"
                );
                assert!(
                    item["score"].is_null(),
                    "未测项不得有评分（不计入总评）: {item}"
                );
            }
            other => panic!("未知的体检项状态词面: {other}"),
        }
    }

    // ── O2：已测数 + 未测数 == 项数
    let overall = &value["overall"];
    assert_eq!(
        overall["measured_items"].as_u64(),
        Some(measured as u64),
        "已测项数必须与 items 一致"
    );
    assert_eq!(
        overall["not_measured_items"].as_u64(),
        Some(not_measured as u64),
        "未测项数必须与 items 一致"
    );
    assert_eq!(
        measured + not_measured,
        items.len(),
        "已测 + 未测 必须等于项数"
    );

    // ── O3 未测不计入总评：overall.score == 已测项评分的算术均值（**可重算**）
    if measured == 0 {
        assert_eq!(
            overall["score"].as_u64(),
            Some(0),
            "一项都没测时总评分必须为 0（不冒充干净）"
        );
    } else {
        let sum: u64 = items
            .iter()
            .filter(|item| item["status"].as_str() == Some("measured"))
            .filter_map(|item| item["score"].as_u64())
            .sum();
        let expected = sum / measured as u64;
        assert_eq!(
            overall["score"].as_u64(),
            Some(expected),
            "总评分必须等于**已测项**评分的算术均值（未测不得稀释）"
        );
    }

    // ── O5 确定性：同样输入 + 同样时钟 ⇒ 报告逐字节相同
    let second = run_exposure_audit_at(&config, 1_700_000_000_000);
    assert_eq!(
        json,
        second.to_json(),
        "确定性：同样输入 + 同样时钟 ⇒ 报告必须逐字节相同"
    );

    let _ = fs::remove_dir_all(&root);
    let _ = PathBuf::from(&root);
});
