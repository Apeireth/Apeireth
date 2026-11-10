//! 桌宠「自动接管」偏好 (`pet-prefs.json`, **默认关闭**).
//!
//! 口径 (2026-10-17): 桌宠默认不自动接管 —— 主窗收起 (关进托盘) 时右下角不再
//! 出现形象窗 (用户口径: 神秘未完成图案默认禁用). 两处操控并存:
//! - 托盘「桌宠」= 手动召唤 (一次性显隐, 不写本偏好);
//! - 设置 › 外观 › 形象面板的总开关 = 本偏好 (经 `set_pet_enabled` 持久化).
//!
//! 存储走主 workspace 信封基础件 (严格打开): 文件缺失 = 默认关闭; 畸形/版本
//! 不符拒开时按默认关闭运行并 eprintln 如实报错 —— 绝不把坏值当真, 也绝不
//! 把读到的坏值回写.

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::DocCompat;
use serde::{Deserialize, Serialize};

use crate::logging::DesktopLogger;
use crate::stored_config;

/// 偏好文件名 (与 backend-provider-env.json 同居 app-data 目录).
pub const PET_PREFS_FILE: &str = "pet-prefs.json";

const PET_PREFS_DOC_NAME: &str = "desktop-pet-prefs";
const PET_PREFS_DOC_VERSION: u32 = 1;

/// 桌宠偏好正文.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PetPrefs {
    /// 主窗收起时桌宠是否自动接管常驻. 缺字段与默认实现都落 `false`
    /// (默认关闭 —— 2026-10-17 定的口径).
    #[serde(default)]
    pub auto_takeover: bool,
}

fn doc_compat() -> DocCompat {
    DocCompat::exact(PET_PREFS_DOC_NAME, PET_PREFS_DOC_VERSION)
}

/// 偏好文件位置: 与 supervisor 同一条数据目录链 (便携契约优先, 常规 = 日志目录
/// 的父级, 即 app-data 目录).
pub fn pet_prefs_path(logger: &DesktopLogger) -> PathBuf {
    if let Some(portable) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|dir| crate::portable_mode::resolve_portable_data_dir(&dir))
    {
        return portable.join(PET_PREFS_FILE);
    }
    logger
        .log_directory()
        .parent()
        .map(|dir| dir.join(PET_PREFS_FILE))
        .unwrap_or_else(|| logger.log_directory().join(PET_PREFS_FILE))
}

/// 读偏好. 文件缺失 = 默认关闭; 畸形/拒开 = 默认关闭 + 如实报错 (不回写).
pub fn load(path: &Path) -> PetPrefs {
    match stored_config::load_config::<PetPrefs>(path, &doc_compat()) {
        Ok(Some(prefs)) => prefs,
        Ok(None) => PetPrefs::default(),
        Err(error) => {
            eprintln!("pet prefs load failed, defaulting to disabled: {error}");
            PetPrefs::default()
        }
    }
}

/// 写偏好 (信封 + 原子持久写).
pub fn save(path: &Path, prefs: &PetPrefs) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs_err::create_dir_all(dir)
            .map_err(|error| format!("failed to create app-data dir: {error}"))?;
    }
    stored_config::save_config(path, &doc_compat(), prefs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pet-prefs-test-{}-{name}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).unwrap();
        dir.join(PET_PREFS_FILE)
    }

    #[test]
    fn default_is_disabled() {
        assert!(!PetPrefs::default().auto_takeover);
    }

    #[test]
    fn missing_file_reads_as_disabled() {
        let path = scratch("missing");
        assert!(!load(&path).auto_takeover);
    }

    #[test]
    fn save_load_roundtrip() {
        let path = scratch("roundtrip");
        save(
            &path,
            &PetPrefs {
                auto_takeover: true,
            },
        )
        .unwrap();
        assert!(load(&path).auto_takeover);
        save(
            &path,
            &PetPrefs {
                auto_takeover: false,
            },
        )
        .unwrap();
        assert!(!load(&path).auto_takeover);
    }

    #[test]
    fn corrupt_file_defaults_to_disabled_without_rewrite() {
        let path = scratch("corrupt");
        fs_err::write(&path, "{ this is not json").unwrap();
        assert!(!load(&path).auto_takeover);
        // 拒开不回写: 原字节保持 (严格打开纪律)
        assert_eq!(fs_err::read_to_string(&path).unwrap(), "{ this is not json");
    }
}
