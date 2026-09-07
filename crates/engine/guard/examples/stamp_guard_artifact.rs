//! Stamp canonical SHA256 onto a Guard joint artifact.

use std::path::PathBuf;

use apeireth_guard::stamp_artifact_file;

fn main() {
    let dest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../artifacts/guard-joint-shadow-v0.json");
    match stamp_artifact_file(&dest) {
        Ok(sha) => println!("stamped {} sha={sha}", dest.display()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
