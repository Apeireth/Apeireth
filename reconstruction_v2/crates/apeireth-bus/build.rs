//! build.rs — compile `proto/bus.proto` via tonic-build.
//!
//! protoc is auto-downloaded once via `protoc-bin-vendored` so the host
//! machine does NOT need a pre-installed protoc binary.

use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Resolve protoc binary (auto-download to OUT_DIR style cache, no install).
    let protoc_path = match protoc_bin_vendored::protoc_bin_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[apeireth-bus build.rs] protoc download failed: {e}");
            return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, e)));
        }
    };
    std::env::set_var("PROTOC", &protoc_path);
    println!(
        "[apeireth-bus build.rs] using vendored protoc: {}",
        protoc_path.display()
    );

    // 2. Compile proto.
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/bus.proto"], &["proto"])?;
    Ok(())
}
