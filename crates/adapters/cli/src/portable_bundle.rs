//! Portable USB Agent Bundle Synthesizer (随身 U 盘生命体便携化打包器).
//!
//! # Architectural Foundations
//!
//! Enables zero-install, plug-and-play Apeireth companion deployment from a single USB flash drive:
//! - **Relative Path Isolation**: Binds SQLite database and memory streams to relative `./data/` directories,
//!   ensuring portability across varying Windows drive letters (`D:`, `E:`, `F:`) and POSIX mount points;
//! - **Single-Binary & Asset Bundling**: Synthesizes directory manifest and configuration files;
//! - **Multi-Platform Launcher Scripts**: Automatically generates `run_apeireth.bat` (Windows) and
//!   `run_apeireth.sh` (Linux/macOS) with auto-detecting current directory environment variables.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};

/// Portable Agent Package Metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableBundleManifest {
    pub agent_name: String,
    pub version: String,
    pub bundle_id: String,
    pub created_at_secs: u64,
    pub relative_data_dir: String,
    pub relative_db_path: String,
    pub included_files: Vec<String>,
}

/// Portable USB Bundle Synthesizer.
#[derive(Debug, Clone)]
pub struct PortableBundleSynthesizer {
    pub agent_name: String,
    pub version: String,
}

impl Default for PortableBundleSynthesizer {
    fn default() -> Self {
        Self {
            agent_name: "Apeireth-Companion".into(),
            version: "2.0.0-preview".into(),
        }
    }
}

impl PortableBundleSynthesizer {
    pub fn new(agent_name: &str, version: &str) -> Self {
        Self {
            agent_name: agent_name.to_string(),
            version: version.to_string(),
        }
    }

    /// Generates Windows batch script for plug-and-play USB execution.
    pub fn generate_windows_launcher(&self) -> String {
        // 2026-10-24 修：原脚本调 `apeireth.exe serve --data-dir "./data"` ——
        // **两个都不存在**：CLI 没有 `serve` 子命令（是 `gateway serve`），
        // 也没有 `--data-dir` 旗标（数据目录走 `APEIRETH_DATA_DIR` 环境变量）。
        // 还顺手设了两个全仓无人读取的变量（`APEIRETH_HOME` / `APEIRETH_PORTABLE`）——
        // 设了没人读的变量等于骗人：用户会以为它在起作用。
        //
        // 现在只设**真实存在**的 `APEIRETH_DATA_DIR`，并调**真实存在**的命令。
        r#"@echo off
setlocal
cd /d "%~dp0"
echo [Apeireth] Starting Portable USB Agent...
if not exist "data" mkdir "data"
set APEIRETH_DATA_DIR=%~dp0data
apeireth.exe gateway serve
pause
"#
        .to_string()
    }

    /// Generates POSIX shell script for USB execution.
    pub fn generate_posix_launcher(&self) -> String {
        // 同 Windows 版：修为真实命令（`gateway serve`）+ 真实变量
        // （`APEIRETH_DATA_DIR`），不再设无人读取的 `APEIRETH_HOME`/`APEIRETH_PORTABLE`。
        r#"#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
cd "$DIR"
echo "[Apeireth] Starting Portable USB Agent..."
mkdir -p "$DIR/data"
export APEIRETH_DATA_DIR="$DIR/data"
chmod +x ./apeireth 2>/dev/null || true
./apeireth gateway serve
"#
        .to_string()
    }

    /// Generates default portable configuration JSON.
    pub fn generate_portable_config(&self) -> String {
        // 2026-10-24 修两处「说不实」：
        // 1. `database_path` 原写 `./data/apeireth.db` —— 真实文件名是
        //    `cognitive.sqlite3`（见 cli `default_panel_data_dir` + 认知库命名）；
        // 2. 本文件**不被程序读取**：真实数据目录由启动脚本设的
        //    `APEIRETH_DATA_DIR` 决定。所以加 `_note` 如实说明它是
        //    **便携布局的说明性配置**，改了不会改变程序行为 ——
        //    否则用户改这里发现没效果，只会以为是 bug。
        serde_json::json!({
            "_note": "说明性配置（仅供参考）: 实际数据目录由启动脚本设置的 APEIRETH_DATA_DIR 决定, 修改本文件不改变程序行为。",
            "agent_name": self.agent_name,
            "version": self.version,
            "storage": {
                "mode": "portable_relative",
                "data_dir": "./data",
                "database_path": "./data/cognitive.sqlite3",
                "storage_dir": "./data",
                "vault_dir": "./data/vault"
            },
            "network": {
                "bind_host": "127.0.0.1",
                "bind_port": 8787,
                // 2026-10-24 修：原来是 `true`，但 `p2p_mesh` 是**明文桩**
                // （无 Noise 握手、无洋葱封装，加密在 v2.1 路线图）——
                // 把它在便携包里默认打开，等于让「U 盘一插就能用」的用户
                // 在不知情下跑明文点对点。默认 false，要开自己显式改。
                "allow_p2p_mesh": false
            }
        })
        .to_string()
    }

    /// Synthesizes complete manifest for USB packaging.
    pub fn synthesize_manifest(&self, created_at_secs: u64) -> PortableBundleManifest {
        PortableBundleManifest {
            agent_name: self.agent_name.clone(),
            version: self.version.clone(),
            bundle_id: format!(
                "bundle_{}_{created_at_secs}",
                self.agent_name.to_lowercase()
            ),
            created_at_secs,
            relative_data_dir: "./data".into(),
            // 2026-10-24 修: 真实认知库文件名是 `cognitive.sqlite3`（此前后缀写错）。
            relative_db_path: "./data/cognitive.sqlite3".into(),
            // 2026-10-24 修：原来这里是**硬编码的假清单** ——
            // 列了 `apeireth.exe`（Linux 上没有）、`apeireth.json`（本项目里
            // 根本没有这个文件名，实际是 `portable-config.json`）、
            // `data/apeireth.db`（既不叫这个名也还没被创建）。
            // 现在由落盘方按**真实写到盘上的文件**填充（见 `bundle_command`）。
            included_files: vec![
                "run_apeireth.bat".into(),
                "run_apeireth.sh".into(),
                "portable-config.json".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 启动脚本必须**真能用**：调真实存在的子命令、设真实存在的变量。
    ///
    /// 2026-10-24 重写：原测试断言的是 `APEIRETH_PORTABLE=1` ——
    /// 而那是个**全仓无人读取的变量**（旧脚本还调了不存在的
    /// `serve --data-dir`）。测试把缺陷钉住了，所以接线时必须连测试一起改：
    /// 现在断言「真实命令 + 真实变量」，并且**反向断言**那两个假变量不再出现。
    #[test]
    fn test_portable_bundle_launcher_scripts() {
        let synth = PortableBundleSynthesizer::default();
        let bat = synth.generate_windows_launcher();
        let sh = synth.generate_posix_launcher();

        for script in [&bat, &sh] {
            assert!(
                script.contains("gateway serve"),
                "必须调真实子命令 `gateway serve`:\n{script}"
            );
            assert!(
                script.contains("APEIRETH_DATA_DIR"),
                "数据目录靠真实变量 `APEIRETH_DATA_DIR`:\n{script}"
            );
            assert!(
                !script.contains(" serve --data-dir"),
                "`serve --data-dir` 不存在（旧脚本硬伤）:\n{script}"
            );
            assert!(
                !script.contains("APEIRETH_PORTABLE") && !script.contains("APEIRETH_HOME"),
                "不得设无人读取的变量:\n{script}"
            );
        }
        // 平台专有部分
        assert!(bat.contains("%~dp0data"), "Windows 数据目录相对包内");
        assert!(bat.contains("apeireth.exe gateway serve"), "{bat}");
        assert!(sh.contains("chmod +x ./apeireth"), "{sh}");
        assert!(sh.contains("$DIR/data"), "POSIX 数据目录相对包内: {sh}");
    }

    #[test]
    fn test_portable_bundle_manifest_generation() {
        let synth = PortableBundleSynthesizer::new("Apeireth-Pro", "2.0.0");
        let manifest = synth.synthesize_manifest(1780000000);

        assert_eq!(manifest.agent_name, "Apeireth-Pro");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.relative_data_dir, "./data");
        assert!(manifest
            .included_files
            .contains(&"run_apeireth.bat".to_string()));
    }
}
