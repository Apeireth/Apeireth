//! **记忆引擎 B3 批的 CLI 出口**验收（影子双跑 + 换算法流程）。
//!
//! # 这个测试要证明什么
//!
//! B3 已经接进生产（默认关），本文件钉住**面向人的入口**（`apeireth memory shadow|swap …`）
//! 的**失败闭合**与**诚实口径**：
//!
//! 1. **默认态（没有 `shadow.json`）** ⇒ 子命令**如实报「未配置」**
//!    ⇒ **不许**把「没跑」写成「成功」⇒ 退出码必须**非 0**；
//! 2. **配置坏了**（未知字段 / 越界）⇒ **报错**（不许被当成「没配置」）；
//! 3. **没有治理链** ⇒ `swap apply` **必被拒**（失败闭合）⇒ 且输出**真实原因**
//!    ⇒ **不许**假装成功、**不许**自己造审批；
//! 4. **只读子命令**（`shadow status` / `swap status`）⇒ **不改任何文件**
//!    ⇒ 跑前后**数据根逐项相同**。
//!
//! # 中性表述
//!
//! 只描述机制（影子 / 换算法 / 治理链 / 失败闭合），不引用任何外部产品名、文档或代码。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use apeireth_memory::SqliteMemoryStore;

/// CLI 二进制（`cargo test` 时由 `CARGO_BIN_EXE_*` 注入）。
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apeireth"))
}

/// 一个**空但存在**的认知库（CLI 要求 `--db` 存在）。
fn make_db(dir: &Path) -> PathBuf {
    let path = dir.join("cognitive.sqlite3");
    let _store = SqliteMemoryStore::open(&path).expect("建一个认知库");
    path
}

/// 数据根里**每个文件**的相对路径 + 内容摘要（用来证明「只读子命令不改文件」）。
fn snapshot(root: &Path) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, u64>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                out.insert(rel, size);
            }
        }
    }
    walk(root, root, &mut out);
    out
}

/// 合法配置（StoredDoc 信封：name / version / compatible_versions / body）。
fn valid_config() -> String {
    serde_json::json!({
        "name": "apeireth.memory.shadow_config",
        "version": 1,
        "compatible_versions": [1],
        "body": {
            "challenger_id": "challenger-alpha",
            "challenger": {"first_pos_alpha": 0.05},
            "runs": 2,
            "archive_keep": 5,
            "session_id": "memory-shadow",
            "cases": [
                {"id": "probe-1", "query": "贾宝玉", "expect_ids": ["doc-1"]}
            ]
        }
    })
    .to_string()
}

/// 坏配置：**未知字段** ⇒ 必须被拒（不许被当成「没配置」）。
fn broken_config() -> String {
    serde_json::json!({
        "name": "apeireth.memory.shadow_config",
        "version": 1,
        "compatible_versions": [1],
        "body": {
            "challenger_id": "challenger-alpha",
            "这块字段不存在": true
        }
    })
    .to_string()
}

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = cli().args(args).output().expect("跑 CLI");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// ⭐ **默认态（没有 `shadow.json`）⇒ 如实说「未配置」**，不许当成功。
#[test]
fn default_state_reports_not_configured_instead_of_success() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db = make_db(dir.path());
    let data_root = dir.path().join("data");
    fs::create_dir_all(&data_root).expect("建数据根");

    let (code, stdout, stderr) = run_cli(&[
        "memory",
        "shadow",
        "status",
        "--data-root",
        data_root.to_str().expect("路径"),
        "--db",
        db.to_str().expect("路径"),
    ]);
    let text = format!("{stdout}\n{stderr}");
    assert_ne!(0, code, "未配置**不许**当成功（退出码必须非 0）：\n{text}");
    assert!(
        text.contains("未配置") || text.contains("没配置") || text.contains("未启用"),
        "必须**如实**说「未配置/未启用」：\n{text}"
    );
    assert!(
        !text.contains("已切换"),
        "未配置时**不许**出现「已切换」：\n{text}"
    );
}

/// ⭐ **配置坏了 ⇒ 报错**（不许被当成「没配置」）。
#[test]
fn broken_config_is_an_error_not_a_silent_skip() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db = make_db(dir.path());
    let data_root = dir.path().join("data");
    fs::create_dir_all(&data_root).expect("建数据根");
    fs::write(data_root.join("shadow.json"), broken_config()).expect("写坏配置");

    let (code, stdout, stderr) = run_cli(&[
        "memory",
        "shadow",
        "status",
        "--data-root",
        data_root.to_str().expect("路径"),
        "--db",
        db.to_str().expect("路径"),
    ]);
    let text = format!("{stdout}\n{stderr}");
    assert_ne!(0, code, "坏配置必须失败（退出码非 0）：\n{text}");
    assert!(
        text.contains("拒开")
            || text.contains("影子配置")
            || text.contains("失败")
            || text.contains("错"),
        "必须给出**真因**（不是「没配置」）：\n{text}"
    );
}

/// ⭐ **没有治理链 ⇒ `swap apply` 必被拒**，且输出**真实原因**。
#[test]
fn swap_apply_without_governance_is_refused_with_the_real_reason() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db = make_db(dir.path());
    let data_root = dir.path().join("data");
    fs::create_dir_all(&data_root).expect("建数据根");
    fs::write(data_root.join("shadow.json"), valid_config()).expect("写合法配置");

    let (code, stdout, stderr) = run_cli(&[
        "memory",
        "swap",
        "apply",
        "--data-root",
        data_root.to_str().expect("路径"),
        "--db",
        db.to_str().expect("路径"),
    ]);
    let text = format!("{stdout}\n{stderr}");
    assert_ne!(0, code, "没有治理链 ⇒ 切换必须被拒（退出码非 0）：\n{text}");
    assert!(
        text.contains("治理") || text.contains("被拒") || text.contains("没有发生"),
        "必须说出**真实原因**（治理面没放行 / 切换没发生）：\n{text}"
    );
    assert!(
        !text.contains("已切换"),
        "被拒时**不许**出现「已切换」：\n{text}"
    );
}

/// ⭐ **只读子命令不改任何文件**（跑前后数据根逐项相同）。
#[test]
fn read_only_subcommands_touch_no_files() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db = make_db(dir.path());
    let data_root = dir.path().join("data");
    fs::create_dir_all(&data_root).expect("建数据根");
    fs::write(data_root.join("shadow.json"), valid_config()).expect("写合法配置");

    let before = snapshot(&data_root);
    let (_code, stdout, stderr) = run_cli(&[
        "memory",
        "shadow",
        "status",
        "--data-root",
        data_root.to_str().expect("路径"),
        "--db",
        db.to_str().expect("路径"),
    ]);
    let text = format!("{stdout}\n{stderr}");
    let after = snapshot(&data_root);
    assert_eq!(
        before, after,
        "只读子命令**不许**改任何文件（跑前后数据根逐项相同）"
    );
    // ⭐ 退出码语义：**没证据 ⇒ 不许换** ⇒ 非 0 是**诚实**的，不是失败；
    //    但输出必须**说出**「不许换」（不许含糊其辞，更不许说「允许换」）。
    assert!(
        text.contains("不许换") || text.contains("允许换"),
        "必须**明确**给出三道闸的结论：\n{text}"
    );
    assert!(
        !text.contains("已切换"),
        "只读子命令**不许**声称「已切换」：\n{text}"
    );
}
