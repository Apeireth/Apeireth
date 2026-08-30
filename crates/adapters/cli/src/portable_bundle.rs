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
        r#"@echo off
setlocal
cd /d "%~dp0"
echo [Apeireth] Starting Portable USB Agent...
set APEIRETH_HOME=%~dp0data
set APEIRETH_PORTABLE=1
if not exist "data" mkdir "data"
apeireth.exe serve --data-dir "./data"
pause
"#
        .to_string()
    }

    /// Generates POSIX shell script for USB execution.
    pub fn generate_posix_launcher(&self) -> String {
        r#"#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
cd "$DIR"
echo "[Apeireth] Starting Portable USB Agent..."
export APEIRETH_HOME="$DIR/data"
export APEIRETH_PORTABLE=1
mkdir -p "$DIR/data"
chmod +x ./apeireth
./apeireth serve --data-dir "./data"
"#
        .to_string()
    }

    /// Generates default portable configuration JSON.
    pub fn generate_portable_config(&self) -> String {
        serde_json::json!({
            "agent_name": self.agent_name,
            "version": self.version,
            "storage": {
                "mode": "portable_relative",
                "database_path": "./data/apeireth.db",
                "memory_streams_dir": "./data/streams",
                "vault_dir": "./data/vault"
            },
            "network": {
                "bind_host": "127.0.0.1",
                "bind_port": 8787,
                "allow_p2p_mesh": true
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
            relative_db_path: "./data/apeireth.db".into(),
            included_files: vec![
                "apeireth.exe".into(),
                "apeireth".into(),
                "run_apeireth.bat".into(),
                "run_apeireth.sh".into(),
                "apeireth.json".into(),
                "data/apeireth.db".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portable_bundle_launcher_scripts() {
        let synth = PortableBundleSynthesizer::default();
        let bat = synth.generate_windows_launcher();
        let sh = synth.generate_posix_launcher();

        assert!(bat.contains("APEIRETH_PORTABLE=1"));
        assert!(bat.contains("%~dp0data"));
        assert!(sh.contains("APEIRETH_PORTABLE=1"));
        assert!(sh.contains("chmod +x ./apeireth"));
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
