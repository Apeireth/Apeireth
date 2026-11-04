//! 暴露面体检 CLI 面 `--out` / `--bundle` 旗标值的形态校验证据。
//!
//! 覆盖: ① 旗标形状的值 (以 `-` 开头) 拒绝即帧 + 退出码非零 + 零落盘;
//! ② 空值拒绝; ③ `--out` 指向目录拒绝 (报告落档要文件路径); ④ 正常路径照旧
//! 写出报告; ⑤ `--bundle` 同款校验 (目录形态的备份包路径保留现状)。
//!
//! 口径: 全部在临时目录里跑, 「目录内容为空」即「一个字节都没落」; 二进制面
//! 真跑一次 (`CARGO_BIN_EXE_apeireth`), 退出码与当前目录零变化由真进程钉住。

use std::path::{Path, PathBuf};
use std::process::Command;

use apeireth_cli::exposure::{run_exposure_on, USAGE};
use apeireth_tools_canonical::exposure_audit::ExposureAuditConfig;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// 临时目录 (先清后建; 名字带 pid 防并行撞名)。
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-out-flag-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 目录内容清单 (目录带尾斜杠); 空清单 = 零落盘。
fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs_err::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        out.push(if entry.path().is_dir() {
            format!("{name}/")
        } else {
            name
        });
    }
    out.sort();
    out
}

/// ① `--out --json`: 拒绝即帧 (点名旗标 + 点名值 + 说清拒因) 且零落盘。
#[test]
fn flag_shaped_out_value_is_refused_and_writes_nothing() {
    let dir = temp_dir("shaped-out");
    let config = ExposureAuditConfig::new(&dir);
    let error = run_exposure_on(&config, &args(&["report", "--out", "--json"]))
        .expect_err("旗标形状的值必须拒绝");
    assert!(error.contains("--out"), "点名旗标: {error}");
    assert!(error.contains("--json"), "点名值: {error}");
    assert!(error.contains("'-'"), "说清拒因: {error}");
    assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    assert!(
        listing(&dir).is_empty(),
        "拒绝时零落盘: {:?}",
        listing(&dir)
    );
}

/// ①' 真进程面: `--out --json` 退出码非零 + 当前目录零新文件。
#[test]
fn binary_refuses_flag_shaped_out_value_with_nonzero_exit_and_empty_cwd() {
    let dir = temp_dir("binary-out");
    let output = Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["exposure", "report", "--out", "--json"])
        .current_dir(&dir)
        .env("APEIRETH_DATA_DIR", &dir)
        .output()
        .expect("run cli");
    assert!(
        !output.status.success(),
        "退出码必须非零: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--out") && stderr.contains("--json"),
        "错误帧点名旗标与值: {stderr}"
    );
    assert!(
        listing(&dir).is_empty(),
        "拒绝时当前目录零新文件: {:?}",
        listing(&dir)
    );
}

/// ② 空值 (`--out ""` / `--bundle ""`): 拒绝即帧 + 零落盘。
#[test]
fn empty_flag_value_is_refused() {
    let dir = temp_dir("empty-out");
    let config = ExposureAuditConfig::new(&dir);
    for flag in ["--out", "--bundle"] {
        let error =
            run_exposure_on(&config, &args(&["report", flag, ""])).expect_err("空值必须拒绝");
        assert!(error.contains(flag), "点名旗标: {error}");
        assert!(error.contains("non-empty"), "说清拒因: {error}");
        assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    }
    assert!(
        listing(&dir).is_empty(),
        "拒绝时零落盘: {:?}",
        listing(&dir)
    );
}

/// ③ `--out` 指向已存在目录: 明确拒 (报告落档要文件路径), 目录里零新文件。
#[test]
fn out_target_that_is_a_directory_is_refused() {
    let dir = temp_dir("out-dir");
    let target = dir.join("target-dir");
    fs_err::create_dir_all(&target).expect("target dir");
    let config = ExposureAuditConfig::new(&dir);
    let error = run_exposure_on(
        &config,
        &args(&["report", "--out", target.to_str().unwrap()]),
    )
    .expect_err("目录目标必须拒绝");
    assert!(error.contains("--out"), "点名旗标: {error}");
    assert!(error.contains("directory"), "说清拒因: {error}");
    assert!(error.contains(USAGE), "错误即帧带用法: {error}");
    assert!(
        listing(&target).is_empty(),
        "目录里零新文件: {:?}",
        listing(&target)
    );
}

/// ④ 正常路径照旧: `--out report.json` 成功, 报告文件真写出 (不误伤)。
#[test]
fn normal_out_path_still_writes_the_report() {
    let dir = temp_dir("normal-out");
    let config = ExposureAuditConfig::new(&dir);
    let out_path = dir.join("report.json");
    let output = run_exposure_on(
        &config,
        &args(&["report", "--out", out_path.to_str().unwrap()]),
    )
    .expect("正常路径必须成功");
    assert!(output.contains("report written to"), "{output}");
    assert!(out_path.is_file(), "报告文件必须写出");
    assert!(fs_err::read_to_string(&out_path)
        .unwrap()
        .contains("暴露面体检报告"));
    assert_eq!(listing(&dir), vec!["report.json".to_string()]);
}

/// ⑤ `--bundle` 同款校验: 旗标形状的值拒; 目录形态 (备份包目录) 保留现状。
#[test]
fn bundle_flag_value_gets_the_same_shape_gate() {
    let dir = temp_dir("bundle");
    let config = ExposureAuditConfig::new(&dir);
    let error = run_exposure_on(&config, &args(&["report", "--bundle", "--json"]))
        .expect_err("旗标形状的值必须拒绝");
    assert!(error.contains("--bundle"), "点名旗标: {error}");
    assert!(error.contains("--json"), "点名值: {error}");
    assert!(error.contains("'-'"), "说清拒因: {error}");
    assert!(
        listing(&dir).is_empty(),
        "拒绝时零落盘: {:?}",
        listing(&dir)
    );

    // 目录形态保留现状: 备份包本就是目录, 该目录即核对目标 (不拒、不写)。
    let bundle_dir = dir.join("bundle-a");
    fs_err::create_dir_all(&bundle_dir).expect("bundle dir");
    let output = run_exposure_on(
        &config,
        &args(&["report", "--bundle", bundle_dir.to_str().unwrap()]),
    )
    .expect("目录形态的备份包路径必须照旧接受");
    assert!(
        output.contains("核对目标"),
        "备份包目录仍是核对目标: {output}"
    );
}
