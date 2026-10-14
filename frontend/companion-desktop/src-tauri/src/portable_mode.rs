// 星槎 · 便携随身面（星港 I 批原生侧，K-starraft.md 的第一批真实落地）
//
// 契约（K-starraft.md）：
//   1. 便携判定 —— 可执行文件同级出现 `portable.flag`（文件）或 `data/`
//      （目录）即成立便携模式，数据目录随身 = exe_dir/data；
//   2. 0 装 —— 标记形态不符（flag 是目录、data 是文件）不猜不将就，
//      一律判非便携；
//   3. 覆盖点 —— 数据目录链路（backend_supervisor::logger_app_data_dir）
//      优先走本目录，无标记维持常规安装链路；
//   4. 便携不降安全基线 —— 密钥仍走系统凭据库（keychain 语义不变），
//      本面只管数据目录随身。
//
// 测试：本文件 #[cfg(test)] 单元测试（有/无标记、标记形态、嵌套路径）。

use std::path::{Path, PathBuf};

/// 便携标记文件名（存在为文件即成立）。
pub const PORTABLE_FLAG_NAME: &str = "portable.flag";
/// 便携数据目录名（存在为目录即成立，同时是随身数据落点）。
pub const PORTABLE_DATA_DIR_NAME: &str = "data";

/// 判定便携模式：exe 同级有 `portable.flag` 文件或 `data/` 目录 → 返回
/// `exe_dir/data`；否则 None（常规安装）。形态不符不猜（flag 是目录、
/// data 是文件都不算数）。
pub fn resolve_portable_data_dir(exe_dir: &Path) -> Option<PathBuf> {
    let flag = exe_dir.join(PORTABLE_FLAG_NAME);
    let data = exe_dir.join(PORTABLE_DATA_DIR_NAME);
    if flag.is_file() || data.is_dir() {
        return Some(data);
    }
    None
}

/// 便携模式状态（命令回执；data_dir 用字符串如实外显路径）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PortableModeStatus {
    pub active: bool,
    pub data_dir: Option<String>,
}

/// 查询当前运行是否处于便携模式（供前端「星槎」面外显）。
#[tauri::command]
pub fn get_portable_mode() -> Result<PortableModeStatus, String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法定位可执行文件：{e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "可执行文件无父目录，无法判定便携模式".to_string())?;
    Ok(match resolve_portable_data_dir(dir) {
        Some(data) => PortableModeStatus {
            active: true,
            data_dir: Some(data.to_string_lossy().into_owned()),
        },
        None => PortableModeStatus {
            active: false,
            data_dir: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEMP_SEQ: AtomicU32 = AtomicU32::new(0);

    /// 一次性临时目录（便携判定夹具；测试自清理）。
    fn temp_dir() -> PathBuf {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-portable-test-{}-{seq}",
            std::process::id()
        ));
        fs_err::remove_dir_all(&dir).ok();
        fs_err::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }

    #[test]
    fn flag_file_activates_portable() {
        let dir = temp_dir();
        fs_err::write(dir.join(PORTABLE_FLAG_NAME), b"portable").expect("写标记文件");
        assert_eq!(
            resolve_portable_data_dir(&dir),
            Some(dir.join(PORTABLE_DATA_DIR_NAME))
        );
        fs_err::remove_dir_all(&dir).ok();
    }

    #[test]
    fn data_dir_alone_activates_portable() {
        let dir = temp_dir();
        fs_err::create_dir_all(dir.join(PORTABLE_DATA_DIR_NAME)).expect("建 data 目录");
        assert_eq!(
            resolve_portable_data_dir(&dir),
            Some(dir.join(PORTABLE_DATA_DIR_NAME))
        );
        fs_err::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrong_marker_shapes_do_not_activate() {
        // flag 是目录、data 是文件：形态不符，0 装不猜。
        let dir = temp_dir();
        fs_err::create_dir_all(dir.join(PORTABLE_FLAG_NAME)).expect("flag 建成目录");
        assert_eq!(resolve_portable_data_dir(&dir), None);
        fs_err::remove_dir_all(&dir).ok();

        let dir = temp_dir();
        fs_err::write(dir.join(PORTABLE_DATA_DIR_NAME), b"not a dir").expect("data 建成文件");
        assert_eq!(resolve_portable_data_dir(&dir), None);
        fs_err::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nothing_means_regular_install() {
        let dir = temp_dir();
        assert_eq!(resolve_portable_data_dir(&dir), None);
        fs_err::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nested_exe_dir_joins_correctly() {
        // 嵌套路径：a/b/c 下立标记，落点仍是 a/b/c/data。
        let root = temp_dir();
        let nested = root.join("a").join("b").join("c");
        fs_err::create_dir_all(&nested).expect("建嵌套目录");
        fs_err::write(nested.join(PORTABLE_FLAG_NAME), b"portable").expect("写标记");
        assert_eq!(
            resolve_portable_data_dir(&nested),
            Some(nested.join(PORTABLE_DATA_DIR_NAME))
        );
        fs_err::remove_dir_all(&root).ok();
    }

    #[test]
    fn both_markers_still_one_data_dir() {
        let dir = temp_dir();
        fs_err::write(dir.join(PORTABLE_FLAG_NAME), b"portable").expect("写标记");
        fs_err::create_dir_all(dir.join(PORTABLE_DATA_DIR_NAME)).expect("建 data");
        assert_eq!(
            resolve_portable_data_dir(&dir),
            Some(dir.join(PORTABLE_DATA_DIR_NAME))
        );
        fs_err::remove_dir_all(&dir).ok();
    }
}
