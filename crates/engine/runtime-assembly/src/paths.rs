//! 数据根路径的**单一真源**。
//!
//! 此前 `consequence_backfill::default_data_root()` 与 CLI 的
//! `default_panel_data_dir()` 是**两份逐字相同**的实现（同 env、同回退路径），
//! 改一处忘另一处就会让「生产写账」与「CLI 读账」指向不同文件。
//! ⇒ 抽到这里，两边都委派过来。

use std::path::PathBuf;

/// 数据根目录：`$APEIRETH_DATA_DIR`（非空时）否则 `$USERPROFILE`/`$HOME` 下的 `.apeireth`。
pub fn default_data_root() -> PathBuf {
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth")
}
