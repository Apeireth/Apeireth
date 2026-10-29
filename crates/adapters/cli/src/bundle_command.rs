//! `apeireth bundle` —— 生成随身 U 盘便携包（一键可用）。
//!
//! # 这个命令做什么
//!
//! 把一个能独立运行的目录造出来：复制当前可执行文件、写两个平台的启动脚本、
//! 写便携配置与清单。用户把整个目录拷到 U 盘，插上任意机器就能跑 ——
//! 数据落在包内 `./data/`（相对路径），换盘符/挂载点都不受影响。
//!
//! # 为什么之前没有这个命令（而 README 说它「还没发货」）
//!
//! 合成器（[`crate::portable_bundle::PortableBundleSynthesizer`]）早写好了，
//! 但**没有任何入口去落盘** —— 而且真去落盘时会发现三处硬伤（2026-10-24 修）：
//!
//! 1. 启动脚本调的 `apeireth serve --data-dir "./data"` **两个都不存在**
//!    （CLI 是 `gateway serve`，数据目录走 `APEIRETH_DATA_DIR`）；
//! 2. 脚本设的 `APEIRETH_HOME` / `APEIRETH_PORTABLE` **全仓无人读取**；
//! 3. 配置里 `allow_p2p_mesh: true` —— 而 `p2p_mesh` 是明文桩。
//!
//! 所以「补一个 CLI 入口」之前必须先修那三处，否则造出来的是**跑不起来的包**。
//!
//! # 0 假装边界
//!
//! - **不覆盖**已存在的文件：跳过并**逐一报出来**（不静默丢，也不悄悄改用户的文件）；
//! - **只写数据**（二进制 / 脚本 / 配置 / 清单），不创建数据库，不初始化任何状态 ——
//!   首次运行时由程序自己建；
//! - 目录必须不存在或为空时才创建成功；已有内容里只补缺的文件。

use std::path::{Path, PathBuf};

use crate::portable_bundle::PortableBundleSynthesizer;

/// 便携包里的文件名（各平台启动脚本 + 配置 + 清单）。
pub const WINDOWS_LAUNCHER: &str = "run_apeireth.bat";
/// POSIX 启动脚本名。
pub const POSIX_LAUNCHER: &str = "run_apeireth.sh";
/// 便携配置文件名。
pub const PORTABLE_CONFIG: &str = "portable-config.json";
/// 清单文件名。
pub const MANIFEST_FILE: &str = "manifest.json";
/// 复制进包里的可执行文件名（Windows 加 `.exe`）。
pub const BINARY_NAME: &str = "apeireth";

/// 生成结果（成功写出与如实跳过各一份账）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BundleReport {
    /// 目标目录。
    pub output_dir: PathBuf,
    /// 成功写出的文件（相对文件名）。
    pub written: Vec<String>,
    /// 已存在因此**跳过**的文件（相对文件名）。
    pub skipped_existing: Vec<String>,
    /// 可执行文件来源（`None` = 当前进程路径不可得，未复制）。
    pub binary_source: Option<PathBuf>,
}

impl BundleReport {
    /// 人类可读摘要（命令的 stdout）。
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("便携包已生成: {}\n", self.output_dir.display()));
        for name in &self.written {
            out.push_str(&format!("  写出  {name}\n"));
        }
        match &self.binary_source {
            Some(source) => {
                out.push_str(&format!("  复制  {BINARY_NAME} ← {}\n", source.display()));
                if let Ok(meta) = std::fs::metadata(source) {
                    let megabytes = meta.len() / (1024 * 1024);
                    if megabytes >= 200 {
                        out.push_str(&format!(
                            "        注意: 该可执行文件 {megabytes} MB（带调试信息）—— \
                             拷到 U 盘前建议改用 release 构建重跑，体积会小一个量级。\n"
                        ));
                    }
                }
            }
            None => out.push_str(
                "  未复制可执行文件: 无法定位当前进程路径 —— \
                 请手动把 apeireth 可执行文件放进包目录（否则启动脚本会找不到它）\n",
            ),
        }
        if !self.skipped_existing.is_empty() {
            out.push_str("以下文件已存在，**未覆盖**（如需替换请自行删除后重跑）:\n");
            for name in &self.skipped_existing {
                out.push_str(&format!("  跳过  {name}\n"));
            }
        }
        out.push_str("\n用法: 把整个目录拷到 U 盘，Windows 双击 run_apeireth.bat，\n");
        out.push_str("      Linux/macOS 执行 ./run_apeireth.sh。数据落在包内 ./data/。\n");
        out
    }
}

/// 解析并执行 `bundle` 子命令。
///
/// 用法: `apeireth bundle --out <目录> [--name <名字>] [--version <版本>] [--now <秒级时间戳>]`
///
/// `--now` 只用于测试与可重现构建（不给则取系统时间）。
pub fn run_bundle(args: &[String]) -> Result<String, String> {
    let mut out: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut now_secs: Option<u64> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "bundle --out 需要一个目录参数".to_string())?;
                out = Some(PathBuf::from(value));
            }
            "--name" => {
                index += 1;
                name = Some(
                    args.get(index)
                        .ok_or_else(|| "bundle --name 需要一个值".to_string())?
                        .clone(),
                );
            }
            "--version" => {
                index += 1;
                version = Some(
                    args.get(index)
                        .ok_or_else(|| "bundle --version 需要一个值".to_string())?
                        .clone(),
                );
            }
            "--now" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or_else(|| "bundle --now 需要一个秒级时间戳".to_string())?;
                now_secs = Some(
                    raw.parse::<u64>()
                        .map_err(|_| format!("bundle --now 不是合法秒级时间戳: {raw}"))?,
                );
            }
            "--help" | "-h" => {
                return Ok(
                    "用法: apeireth bundle --out <目录> [--name <名字>] [--version <版本>]\n\
                     \n\
                     生成随身 U 盘便携包: 复制可执行文件 + 两平台启动脚本 + 便携配置 + 清单。\n\
                     数据落在包内 ./data/（相对路径，换盘符/挂载点不受影响）。\n\
                     已存在的文件不会被覆盖（会如实报出来）。"
                        .to_string(),
                );
            }
            other => return Err(format!("bundle 不认识的参数: {other}")),
        }
        index += 1;
    }

    let output_dir = out
        .ok_or_else(|| "bundle 需要 --out <目录>（往磁盘写东西的命令不给默认位置）".to_string())?;
    let synthesizer = PortableBundleSynthesizer::new(
        name.as_deref().unwrap_or("Apeireth-Companion"),
        version.as_deref().unwrap_or("2.0.0-preview"),
    );
    let created = now_secs.unwrap_or_else(system_epoch_secs);
    let report = write_bundle(&output_dir, &synthesizer, created)?;
    Ok(report.render())
}

/// 把便携包落到 `output_dir`（核心实现，测试直调口）。
pub fn write_bundle(
    output_dir: &Path,
    synthesizer: &PortableBundleSynthesizer,
    created_at_secs: u64,
) -> Result<BundleReport, String> {
    std::fs::create_dir_all(output_dir)
        .map_err(|error| format!("无法创建目录 {}: {error}", output_dir.display()))?;

    let mut report = BundleReport {
        output_dir: output_dir.to_path_buf(),
        ..BundleReport::default()
    };

    // 文本产物：已存在即跳过（不覆盖用户已改过的脚本/配置）。
    // 清单**不在这里写** —— 它最后写，因为 `included_files` 要读盘上真实文件。
    for (name, content, executable) in [
        (
            WINDOWS_LAUNCHER,
            synthesizer.generate_windows_launcher(),
            false,
        ),
        (POSIX_LAUNCHER, synthesizer.generate_posix_launcher(), true),
        (
            PORTABLE_CONFIG,
            synthesizer.generate_portable_config(),
            false,
        ),
    ] {
        write_text_if_absent(output_dir, name, &content, executable, &mut report)?;
    }

    // 可执行文件：复制当前进程（真正的「零安装」靠它）
    if let Ok(current) = std::env::current_exe() {
        let binary_name = binary_file_name();
        let target = output_dir.join(binary_name);
        if target.exists() {
            report.skipped_existing.push(binary_name.to_string());
        } else {
            match std::fs::copy(&current, &target) {
                Ok(_) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt as _;
                        let _ = std::fs::set_permissions(
                            &target,
                            std::fs::Permissions::from_mode(0o755),
                        );
                    }
                    report.binary_source = Some(current);
                }
                Err(error) => {
                    return Err(format!(
                        "复制可执行文件失败（{} → {}）: {error}",
                        current.display(),
                        target.display()
                    ));
                }
            }
        }
    }

    // 清单**最后写**，且 `included_files` 取盘上**实际存在**的文件 ——
    // 清单的天职是描述现实，不是描述愿望。之前合成器里硬编码的清单就是反例
    // （列了 `apeireth.exe` / `apeireth.json` / `data/apeireth.db` 三个不存在的东西）。
    let mut manifest = synthesizer.synthesize_manifest(created_at_secs);
    manifest.included_files = list_bundle_files(output_dir, MANIFEST_FILE);
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("清单序列化失败: {error}"))?;
    write_text_if_absent(
        output_dir,
        MANIFEST_FILE,
        &manifest_json,
        false,
        &mut report,
    )?;

    Ok(report)
}

/// 盘上实际存在的便携包文件（相对文件名，升序；排除清单自身）。
fn list_bundle_files(dir: &Path, exclude: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path();
                    if !path.is_file() {
                        return None;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    (name != exclude).then_some(name)
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// 本平台的可执行文件名（Windows 带 `.exe`）。
fn binary_file_name() -> &'static str {
    if cfg!(windows) {
        "apeireth.exe"
    } else {
        BINARY_NAME
    }
}

/// 写一个文本文件；已存在则记入 `skipped_existing`，绝不覆盖。
fn write_text_if_absent(
    dir: &Path,
    name: &str,
    content: &str,
    executable: bool,
    report: &mut BundleReport,
) -> Result<(), String> {
    let path = dir.join(name);
    if path.exists() {
        report.skipped_existing.push(name.to_string());
        return Ok(());
    }
    std::fs::write(&path, content).map_err(|error| format!("写入 {name} 失败: {error}"))?;
    if executable {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .map_err(|error| format!("给 {name} 加可执行位失败: {error}"))?;
        }
        #[cfg(not(unix))]
        {
            let _ = &path;
        }
    }
    report.written.push(name.to_string());
    Ok(())
}

/// 当前 Unix 秒（`--now` 缺省时用）。
fn system_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthesizer() -> PortableBundleSynthesizer {
        PortableBundleSynthesizer::new("Test-Companion", "9.9.9")
    }

    #[test]
    fn writes_the_whole_bundle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("usb");
        let report = write_bundle(&target, &synthesizer(), 1_700_000_000).expect("bundle");
        for name in [
            WINDOWS_LAUNCHER,
            POSIX_LAUNCHER,
            PORTABLE_CONFIG,
            MANIFEST_FILE,
        ] {
            assert!(
                target.join(name).exists(),
                "{name} 应已写出: {:?}",
                report.written
            );
            assert!(
                report.written.contains(&name.to_string()),
                "{name} 应记入写出账"
            );
        }
        assert!(report.skipped_existing.is_empty(), "{report:?}");
        // 清单元数据可解析且与合成器一致
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(target.join(MANIFEST_FILE)).unwrap())
                .expect("manifest 应是合法 JSON");
        assert_eq!(manifest["agent_name"], "Test-Companion");
        assert_eq!(manifest["version"], "9.9.9");
    }

    /// 清单的 `included_files` 必须是**盘上真实存在**的文件。
    ///
    /// 反例（本轮修掉）：合成器里硬编码了 `apeireth.exe` / `apeireth.json` /
    /// `data/apeireth.db` 三个不存在的东西 —— 清单在说谎，用户拿它核对包内容
    /// 会以为少了文件（或以为有根本不存在的配置）。
    #[test]
    fn manifest_lists_only_files_that_actually_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("usb");
        write_bundle(&target, &synthesizer(), 1_700_000_000).expect("bundle");

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(target.join(MANIFEST_FILE)).unwrap())
                .expect("manifest json");
        let listed: Vec<String> = manifest["included_files"]
            .as_array()
            .expect("included_files 应是数组")
            .iter()
            .map(|value| value.as_str().unwrap_or_default().to_string())
            .collect();
        assert!(!listed.is_empty(), "清单不能是空的");
        for name in &listed {
            assert!(
                target.join(name).is_file(),
                "清单列了不存在的文件 {name}（清单必须描述现实）: {listed:?}"
            );
        }
        // 三个文本产物 + 可执行文件（若当前进程可定位）都在
        for required in [WINDOWS_LAUNCHER, POSIX_LAUNCHER, PORTABLE_CONFIG] {
            assert!(
                listed.iter().any(|name| name == required),
                "清单漏了真实存在的 {required}: {listed:?}"
            );
        }
        // 旧硬编码里的三个假条目不得再出现
        for phantom in ["apeireth.json", "data/apeireth.db"] {
            assert!(
                !listed.iter().any(|name| name == phantom),
                "清单里仍有不存在的 {phantom}: {listed:?}"
            );
        }
    }

    /// 启动脚本必须调**真实存在**的命令与变量 —— 这是本轮修掉的硬伤。
    #[test]
    fn launchers_use_the_real_command_and_real_env_var() {
        let synthesizer = synthesizer();
        for script in [
            synthesizer.generate_windows_launcher(),
            synthesizer.generate_posix_launcher(),
        ] {
            assert!(
                script.contains("gateway serve"),
                "必须调真实子命令 `gateway serve`:\n{script}"
            );
            assert!(
                !script.contains(" serve --data-dir"),
                "`serve --data-dir` 不存在（旧脚本硬伤）:\n{script}"
            );
            assert!(
                script.contains("APEIRETH_DATA_DIR"),
                "数据目录靠这个变量（真实存在）:\n{script}"
            );
            assert!(
                !script.contains("APEIRETH_HOME") && !script.contains("APEIRETH_PORTABLE"),
                "不得再设无人读取的变量（设了没人读 = 骗人）:\n{script}"
            );
        }
    }

    /// 便携包不得默认开明文 P2P。
    #[test]
    fn portable_config_does_not_enable_the_plaintext_mesh() {
        let config: serde_json::Value =
            serde_json::from_str(&synthesizer().generate_portable_config()).expect("config json");
        assert_eq!(
            config["network"]["allow_p2p_mesh"], false,
            "p2p_mesh 是明文桩（加密在 v2.1），便携包绝不能默认打开"
        );
    }

    /// 已存在的文件**不覆盖**，且如实报出来。
    #[test]
    fn existing_files_are_skipped_and_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("usb");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join(POSIX_LAUNCHER), "# 用户改过的脚本").unwrap();

        let report = write_bundle(&target, &synthesizer(), 1_700_000_000).expect("bundle");
        assert_eq!(
            std::fs::read_to_string(target.join(POSIX_LAUNCHER)).unwrap(),
            "# 用户改过的脚本",
            "绝不覆盖用户已改过的文件"
        );
        assert!(
            report
                .skipped_existing
                .contains(&POSIX_LAUNCHER.to_string()),
            "跳过要如实记账: {report:?}"
        );
        assert!(!report.written.contains(&POSIX_LAUNCHER.to_string()));
    }

    /// 命令行解析：缺 `--out` 必须拒绝（往磁盘写东西不给默认位置）。
    #[test]
    fn cli_requires_an_output_directory() {
        let error = run_bundle(&[]).expect_err("缺 --out 必须拒绝");
        assert!(error.contains("--out"), "{error}");

        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("usb");
        let summary = run_bundle(&[
            "--out".to_string(),
            target.display().to_string(),
            "--now".to_string(),
            "1700000000".to_string(),
        ])
        .expect("带 --out 应成功");
        assert!(summary.contains("便携包已生成"), "{summary}");
        assert!(target.join(MANIFEST_FILE).exists());
    }

    #[test]
    fn unknown_flag_is_rejected() {
        let error = run_bundle(&["--nope".to_string()]).expect_err("未知参数必须拒绝");
        assert!(error.contains("不认识的参数"), "{error}");
    }
}
