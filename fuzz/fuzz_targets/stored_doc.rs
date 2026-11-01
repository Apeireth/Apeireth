#![no_main]

//! fuzz: **存储文档信封**（`StoredDoc`）—— 数据完整性 / 不静默损坏。
//!
//! 该模块要解决的原问题写在它自己的模块注释里：
//! > 「JSON 配置/记录层『读坏用默认』会静默吞掉损坏」
//! 它给出的两级明确语义：
//! - [`open_single`]（单文档/配置类）：**打开即校验**，不符/畸形**拒绝打开**并报结构化错误 ——
//!   **不静默迁移，不回退默认**；
//! - [`scan_records`]（逐记录类/jsonl）：单条坏行**读作不存在**（跳过**并计数**），不砖化整个单元。
//!
//! 因此可断言（全部取自上述承诺，不自造期望）：
//!
//! 1. 两个入口吃任意字节都**不得 panic**；
//! 2. `open_single` 返回 `Ok` ⇒ 信封**必自洽**（`version ∈ compatible_versions`）且
//!    `name`/版本/可读集全部通过契约 —— 即「不放行畸形文档」；
//! 3. **`scan_records` 的不丢行不变量**：`records.len() + skipped` **必须等于非空行数**。
//!    少一行 = 有记录被**静默丢弃且未计数**（正是该模块承诺要避免的失效模式）；
//! 4. `scan_records` 返回的每条记录都必须自洽（坏记录只能进 `skipped`，不能进 `records`）。

use std::path::PathBuf;

use apeireth_core::stored_doc::{DocCompat, StoredDoc, open_single, scan_records};
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Body {
    #[serde(default)]
    v: String,
}

const DOC_NAME: &str = "fuzz_doc";

fn compat() -> DocCompat {
    DocCompat::new(DOC_NAME, 3, vec![1, 2, 3])
}

/// 本次调用独占的临时文件路径（pid + 原子计数，避免并发撞名）。
fn temp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "apeireth-stored-doc-fuzz-{}-{tag}-{n}.tmp",
        std::process::id()
    ))
}

fuzz_target!(|data: &[u8]| {
    // `scan_records` 走 `read_to_string`；非法 UTF-8 会在 IO 层被拒（非本 target 的关注点）。
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let compat = compat();

    // ── ① open_single：任意字节都不得 panic；Ok 则信封必须自洽且过契约 ─────────
    let single = temp_path("single");
    std::fs::write(&single, data).ok();
    if let Ok(doc) = open_single::<Body>(&single, &compat) {
        assert_eq!(doc.name, compat.name, "open_single 放行了姓名不匹配的文档");
        assert!(
            doc.is_self_consistent(),
            "open_single 放行了不自洽的信封: version={} compatible={:?}",
            doc.version,
            doc.compatible_versions
        );
        assert!(
            doc.version <= compat.current_version,
            "open_single 放行了未来版本: {} > {}",
            doc.version,
            compat.current_version
        );
        assert!(
            compat.readable_versions.contains(&doc.version),
            "open_single 放行了不可读版本: {}",
            doc.version
        );
    }
    let _ = std::fs::remove_file(&single);

    // ── ②③④ scan_records：不 panic + 不丢行 + 放行的记录必须自洽 ──────────────
    let jsonl = temp_path("jsonl");
    std::fs::write(&jsonl, data).ok();
    if let Ok(scan) = scan_records::<Body>(&jsonl, &compat) {
        let non_blank = text.lines().filter(|l| !l.trim().is_empty()).count();

        // ③ 不丢行不变量：记录数 + 跳过数 必须**恰好**覆盖所有非空行。
        assert_eq!(
            scan.records.len() + scan.skipped,
            non_blank,
            "静默丢行: records={} + skipped={} ≠ 非空行数={}（有记录既没被读入也没被计数）",
            scan.records.len(),
            scan.skipped,
            non_blank
        );

        // ④ 放行的每条记录都必须满足与 open_single 同一套契约。
        for doc in &scan.records {
            assert_eq!(doc.name, compat.name, "scan_records 放行了姓名不匹配的记录");
            assert!(
                doc.is_self_consistent(),
                "scan_records 放行了不自洽的记录: version={} compatible={:?}",
                doc.version,
                doc.compatible_versions
            );
            assert!(
                compat.readable_versions.contains(&doc.version),
                "scan_records 放行了不可读版本的记录: {}",
                doc.version
            );
            assert!(
                doc.version <= compat.current_version,
                "scan_records 放行了未来版本的记录: {}",
                doc.version
            );
        }
    }
    let _ = std::fs::remove_file(&jsonl);

    // ── ⑤ 往返：save_single 写出的文档必须能被 open_single 读回（同一契约两端一致）──
    let rt = temp_path("roundtrip");
    let body = Body {
        v: text.to_string(),
    };
    if apeireth_core::stored_doc::save_single(&rt, &compat, body.clone(), 0o600).is_ok() {
        let back = open_single::<Body>(&rt, &compat)
            .unwrap_or_else(|e| panic!("save_single 写出的文档必须能被 open_single 读回: {e:?}"));
        assert_eq!(back.body, body, "save→open 往返内容不一致");
    }
    let _ = std::fs::remove_file(&rt);

    // 抑制未使用告警（StoredDoc 类型在签名中使用）
    let _: Option<StoredDoc<Body>> = None;
});
