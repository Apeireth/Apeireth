//! 与系统参考工具的差分测试：用 readelf 的结果校验自研 ELF 解析。
//!
//! 没有 readelf（非 Linux / 未安装）时自动跳过，不制造环境依赖。

use std::process::Command;

use apeireth_tools_canonical::recon::parse_binary;

fn readelf_available() -> bool {
    Command::new("readelf")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn field<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(str::trim)
}

#[test]
fn elf_header_matches_readelf() {
    if !cfg!(target_os = "linux") || !readelf_available() {
        eprintln!("skipping: readelf is not available");
        return;
    }
    let exe = std::env::current_exe().expect("current exe");
    let output = Command::new("readelf")
        .arg("-h")
        .arg(&exe)
        .output()
        .expect("run readelf");
    assert!(output.status.success(), "readelf failed");
    let text = String::from_utf8_lossy(&output.stdout);

    let bytes = std::fs::read(&exe).expect("read exe");
    let report = parse_binary(&bytes).expect("parse exe");

    let class = field(&text, "Class:").unwrap_or_default();
    let expected_bits = if class.contains("ELF64") { 64 } else { 32 };
    assert_eq!(report.bits, expected_bits, "ELF class mismatch");

    let entry = field(&text, "Entry point address:").unwrap_or_default();
    let expected_entry =
        u64::from_str_radix(entry.trim_start_matches("0x"), 16).expect("parse readelf entry");
    assert_eq!(report.entry, expected_entry, "ELF entry mismatch");

    let sections = field(&text, "Number of section headers:").unwrap_or_default();
    let expected_sections: usize = sections.parse().expect("parse section count");
    assert_eq!(
        report.sections.len(),
        expected_sections.min(512),
        "ELF section count mismatch"
    );
}

#[test]
fn elf_section_names_match_readelf() {
    if !cfg!(target_os = "linux") || !readelf_available() {
        eprintln!("skipping: readelf is not available");
        return;
    }
    let exe = std::env::current_exe().expect("current exe");
    let output = Command::new("readelf")
        .arg("-S")
        .arg("--wide")
        .arg(&exe)
        .output()
        .expect("run readelf");
    assert!(output.status.success(), "readelf -S failed");
    let text = String::from_utf8_lossy(&output.stdout);

    let bytes = std::fs::read(&exe).expect("read exe");
    let report = parse_binary(&bytes).expect("parse exe");

    // readelf -S 每个节头的一行以 "[ N] name type ..." 开头。
    let names: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let after_bracket = trimmed.strip_prefix('[')?;
            let close = after_bracket.find(']')?;
            // 表头行是 "[Nr] Name ..."，索引必须是数字。
            after_bracket[..close].trim().parse::<usize>().ok()?;
            let rest = after_bracket[close + 1..].trim_start();
            let name = rest.split_whitespace().next()?;
            // 空名字的节（如索引 0）会让下一列类型变成首 token；只比较点号开头的真实节名。
            name.starts_with('.').then(|| name.to_string())
        })
        .collect();

    assert!(!names.is_empty(), "readelf produced no section names");
    let ours: Vec<&str> = report.sections.iter().map(|s| s.name.as_str()).collect();
    for expected in &names {
        assert!(
            ours.contains(&expected.as_str()),
            "missing section {expected:?} parsed from readelf output"
        );
    }
}

#[test]
fn parses_real_system_elf_binaries() {
    if !cfg!(target_os = "linux") {
        return;
    }
    // 真实样本：动态链接、PIE、可能 stripped 的系统二进制。
    for path in [
        "/bin/ls",
        "/bin/cat",
        "/usr/bin/python3",
        "/usr/bin/git",
        "/bin/bash",
    ] {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let report = parse_binary(&bytes).unwrap_or_else(|error| {
            panic!("failed to parse real binary {path}: {error}");
        });
        assert_eq!(report.format, "elf", "{path} should be ELF");
        assert!(!report.sections.is_empty(), "{path} has no sections");
        assert!(report.entry > 0, "{path} has no entry point");
    }
}
