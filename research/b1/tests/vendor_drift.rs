//! vendor 漂移检测测试 (协议 §7.2 验收: "评的是引擎真实代码")。
//!
//! 运行时读 `../../crates/engine/memory/src/<file>` 与 `research/b1/src/topo/<file>`
//! 副本, 剥掉文件头 `// vendored from ...` 注释后做 SHA-256 比对, 不一致即 fail。
//! 升级路径: 引擎源变更后, 重新执行逐字节 vendor 拷贝并更新文件头 commit 与本文件常量。

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// vendor 时钉死的引擎 commit (git rev-parse HEAD)。
const COMMIT: &str = "291442c9807a3a5f2af4838b87eb31204e0cdfac";

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn check(file: &str) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_path = manifest.join("../../crates/engine/memory/src").join(file);
    let vendor_path = manifest.join("src/topo").join(file);

    let src_bytes = fs::read(&src_path).expect("读引擎源文件");
    let vendor_bytes = fs::read(&vendor_path).expect("读 vendor 副本");

    // 文件头第一行必须是 vendored 注释 (含正确 commit)。
    let header_end = vendor_bytes
        .iter()
        .position(|&b| b == b'\n')
        .expect("vendor 文件无换行");
    let header = std::str::from_utf8(&vendor_bytes[..header_end]).expect("vendor 文件头非 UTF-8");
    let expect_header = format!("// vendored from crates/engine/memory/src/{file} @ {COMMIT}");
    assert_eq!(header, expect_header, "vendor 文件头不符 (commit 变更?)");

    let body = &vendor_bytes[header_end + 1..];
    let src_hash = sha256_hex(&src_bytes);
    let body_hash = sha256_hex(body);
    assert_eq!(
        body_hash, src_hash,
        "vendor 副本与引擎源漂移: {file}\n  engine: {src_hash}\n  vendor:  {body_hash}\n  \
         升级路径: 重新执行逐字节 vendor 拷贝并更新文件头 commit (协议 §7.2)。"
    );
}

#[test]
fn betti_hole_detector_vendor_matches_engine() {
    check("betti_hole_detector.rs");
}

#[test]
fn kuramoto_resonance_vendor_matches_engine() {
    check("kuramoto_resonance.rs");
}
