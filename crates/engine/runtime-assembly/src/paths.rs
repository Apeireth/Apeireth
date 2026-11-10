//! 数据根路径的**单一真源**。
//!
//! 此前 `consequence_backfill::default_data_root()` 与 CLI 的
//! `default_panel_data_dir()` 是**两份逐字相同**的实现（同 env、同回退路径），
//! 改一处忘另一处就会让「生产写账」与「CLI 读账」指向不同文件。
//! ⇒ 抽到这里，两边都委派过来。

use std::path::PathBuf;

/// 数据根目录：`$APEIRETH_DATA_DIR`（非空时）否则 `$USERPROFILE`/`$HOME` 下的 `.apeireth`。
///
/// ⚠️ **实现已下沉到 `apeireth_core::paths`**（唯一真源）⇒ 这里只做**委派**。
/// 原因：记忆层（`apeireth-memory`）**不能**依赖本 crate（会成环），
/// 但两边都需要同一份口径 ⇒ 落在两边都依赖的 `apeireth-core`。
/// （此前这里与记忆层各有一份**逐字相同**的实现 ⇒ 漂移就会让
/// 「生产写账」与「CLI 读账」指向不同文件。）
pub fn default_data_root() -> PathBuf {
    apeireth_core::paths::default_data_root()
}
