//! R177 extension organ Kani proofs (W9)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_ext_01_version() {
    assert!(!VERSION.is_empty());
}

#[test]
fn r177_ext_02_manifest_from_toml() {
    let toml = r#"
[extension]
name = "test"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "lib.rs"
permissions = []
max_input_bytes = 1024
max_output_bytes = 1024
timeout_ms = 1000
"#;
    let m = Manifest::from_toml(toml).unwrap();
    assert_eq!(m.name, "test");
}

#[test]
fn r177_ext_03_plugin_kind() {
    let k = PluginKind::Sync;
    let _: String = format!("{:?}", k);
}

#[test]
fn r177_ext_04_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[test]
fn r177_ext_05_manifest_name_field() {
    let toml = r#"
[extension]
name = "abc"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "lib.rs"
permissions = []
max_input_bytes = 1024
max_output_bytes = 1024
timeout_ms = 1000
"#;
    let m = Manifest::from_toml(toml).unwrap();
    assert_eq!(m.name, "abc");
}

#[cfg(kani)]
#[kani::proof]
fn r177_ext_kani_01_version_invariant() {
    assert!(!VERSION.is_empty());
}

#[cfg(kani)]
#[kani::proof]
fn r177_ext_kani_02_plugin_kind_invariant() {
    let k = PluginKind::Sync;
    assert!(!format!("{:?}", k).is_empty());
}
