//! `apeireth-sdk` build.rs — cbindgen C-ABI header 生成
//!
//! **R122-8 决策**: 仅 `--features c` 启用时调 cbindgen, 默认 build 0 装 cbindgen
//!   (cbindgen 在 `[build-dependencies]` 也是 `optional = true`, cfg-gated).
//!
//! **生成目标**: `crates/apeireth-sdk/apeireth_sdk.h` (5 fn C 声明)
//! **不漂移**: 0 改 workspace 顶层, 0 触碰 24 LOCKED, 0 假装"100% 多语言支持".
//!
//! **0 装原则 (O-5 实质)**: `cargo build -p apeireth-sdk` (无 features) → 0 调 cbindgen
//!   → 0 生成 header → 0 污染 default build.

#[cfg(feature = "c")]
fn main() {
    use std::path::PathBuf;

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let header_path = crate_dir.join("apeireth_sdk.h");

    // cbindgen 0.26 标准 Builder API (不用 Config struct, 0 触碰 private field)
    let header_comment = "// apeireth-sdk C-ABI header (R122-8 auto-generated, 0 改 24 LOCKED)\n\
                         // O-5 实质: 0 假装 100% multi-lang, 仅 5 fn demo 桥接.\n\
                         // 0 改 workspace.version 1.1.0, 0 触碰 11 agent 公共 API 签名.\n\
                         // Skeleton 桥接 1:1 c.rs 5 fn (count_tokens_c / hash_request_c /\n\
                         // version_c / compile_info_c / free_string_c).\n\
                         // 编译指令: cargo build -p apeireth-sdk --features c\n";

    cbindgen::Builder::new()
        .with_crate(crate_dir.to_str().expect("crate_dir utf-8"))
        .with_language(cbindgen::Language::C)
        .with_header(header_comment)
        .with_include_guard("APEIRETH_SDK_H")
        .with_pragma_once(true)
        .generate()
        .expect("cbindgen 0.26 generate 失败 (R122-8 5 fn C-ABI 桥接)")
        .write_to_file(&header_path);

    println!("cargo:rerun-if-changed=src/c.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cbindgen R122-8 header 写: {}", header_path.display());
}

#[cfg(not(feature = "c"))]
fn main() {
    // 0 启用 c feature → 0 装 cbindgen, 0 生成 header, 0 跑 build script
    // O-5 实质守门: 默认 build 0 跨语言污染
}
