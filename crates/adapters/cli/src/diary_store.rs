//! 跨日记检索接线面（**只读**）的构造。
//!
//! # 为什么单独一个模块
//!
//! 尺寸棘轮（`cli/src/lib.rs` 在基线边缘，只减不增）：装配根只留调用，
//! 构造与理由放在这里。
//!
//! # 存在性即开关（不新增 env 名）
//!
//! 日记目录 = `apeireth dream` 的落盘位（`<data>/diary`，按日一个 JSON 文件）：
//! - 目录不存在 ⇒ 从没写过日记 ⇒ **不挂**日记面 ⇒ 检索路径逐字节零行为变化；
//! - 目录存在 ⇒ 检索命中事实可附**关联日记片段**（「你上周写过……」）。
//!
//! # 只读
//!
//! 检索路径只 `list_days` / `read_day`，**绝不写、绝不删**日记；
//! 写入面仍然只有 `apeireth dream`（显式命令即授权）。

use std::path::Path;
use std::sync::Arc;

use apeireth_memory::diary::{DiaryStore, FileDiaryStore};

/// 构造跨日记检索接线面的日记句柄（**只读**）。
///
/// 缺席 = 检索路径不碰日记 ⇒ 零行为变化。
pub fn diary_store_for_data_dir(data_dir: &Path) -> Option<Arc<dyn DiaryStore>> {
    let root = data_dir.join("diary");
    if !root.is_dir() {
        return None;
    }
    Some(Arc::new(FileDiaryStore::new(root)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::diary::{DiaryEntry, DiaryStore as _};

    #[test]
    fn absent_diary_directory_yields_no_store() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(diary_store_for_data_dir(dir.path()).is_none());
    }

    #[test]
    fn present_diary_directory_yields_a_readable_store() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("diary");
        std::fs::create_dir_all(&root).expect("create diary dir");
        let store = diary_store_for_data_dir(dir.path()).expect("store is constructed");
        store
            .append(
                "2026-08-28",
                DiaryEntry::new("dream", "今天主人教我写 Rust", 1),
            )
            .expect("append");
        assert_eq!(store.list_days().expect("days"), vec!["2026-08-28"]);
    }
}
