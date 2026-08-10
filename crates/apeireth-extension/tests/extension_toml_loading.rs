//! Integration: extension.toml 严格 schema 加载

use apeireth_extension::manifest::Manifest;
use apeireth_extension::types::PluginKind;

const OK: &str = r#"
[extension]
name = "toml-plugin"
version = "0.1.0"
kind = "sync"
description = "Loaded from TOML"
entry = "lib.rs"
permissions = ["invoke"]
max_input_bytes = 65536
max_output_bytes = 65536
timeout_ms = 1000
"#;

#[test]
fn toml_01_load_valid() {
    let m = Manifest::from_toml(OK).unwrap();
    assert_eq!(m.name, "toml-plugin");
    assert_eq!(m.kind, PluginKind::Sync);
    assert_eq!(m.permissions.len(), 1);
}

#[test]
fn toml_02_load_all_6_kinds() {
    for k in PluginKind::ALL {
        let text = format!(
            r#"
[extension]
name = "load-{k}"
version = "1.0.0"
kind = "{k}"
description = "load test"
entry = "lib.rs"
permissions = ["invoke"]
max_input_bytes = 1024
max_output_bytes = 1024
timeout_ms = 1
"#
        );
        let m = Manifest::from_toml(&text).unwrap();
        assert_eq!(m.kind, *k);
        assert_eq!(m.name, format!("load-{}", k));
    }
}

#[test]
fn toml_03_invalid_toml_syntax() {
    let bad = r#"
[extension
name = "x"
"#;
    let res = Manifest::from_toml(bad);
    assert!(res.is_err());
}

#[test]
fn toml_04_missing_required_field() {
    let bad = r#"
[extension]
name = "x"
# version missing
kind = "sync"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#;
    let res = Manifest::from_toml(bad);
    assert!(res.is_err());
}

#[test]
fn toml_05_size_limits_below_minimum() {
    let bad = r#"
[extension]
name = "x"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "x.rs"
permissions = ["invoke"]
max_input_bytes = 10
max_output_bytes = 10
timeout_ms = 1
"#;
    let res = Manifest::from_toml(bad);
    assert!(res.is_err());
}

#[test]
fn toml_06_kind_string_variants() {
    // "preprocessor" alias should work
    let text = r#"
[extension]
name = "x"
version = "0.1.0"
kind = "preprocessor"
description = "x"
entry = "x.rs"
permissions = ["read"]
max_input_bytes = 1024
max_output_bytes = 1024
timeout_ms = 1
"#;
    let m = Manifest::from_toml(text).unwrap();
    assert_eq!(m.kind, PluginKind::MessagePreprocessor);
}
