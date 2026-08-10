fn main() {
    // V2 Day 1 Step 1.3: tauri-desktop 主入口已 DEPRECATED, autobins=false 关掉了 bin target.
    // 仅当 R19 worker 显式构建 bin (如 `cargo build --bin apeireth-tauri-stub`) 时再跑 tauri_build,
    // 避免 "cargo:rustc-link-arg-bins" 在无 bin 时报错。
    if std::env::var("CARGO_BIN_NAME").is_ok() {
        tauri_build::build()
    }
}
