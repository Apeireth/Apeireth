//! 数据根路径的**唯一真源**。
//!
//! # 为什么在 core
//!
//! 「数据根在哪」这件事历史上出过**两份逐字相同**的实现：
//! * `apeireth_runtime_assembly::paths::default_data_root()`；
//! * `apeireth_memory::wiki_corpus_real::default_data_root()`（记忆层语料接线时镜像了一份）。
//!
//! 两份实现一旦漂移，就会让「**生产写账**」与「**CLI 读账**」指向**不同文件**
//! ⇒ 体检命令读不到刚写的记忆、备份漏文件、迁移回滚找不到目标。
//!
//! 而记忆层**不能**依赖 `runtime-assembly`（那会成环：`runtime-assembly → memory`），
//! ⇒ 唯一能同时被两边依赖的位置是**更底层的 `apeireth-core`** ⇒ 落在这里。
//!
//! 口径（**逐字沿用**原实现，未改任何行为）：
//! 1. `APEIRETH_DATA_DIR`（非空）⇒ 用它；
//! 2. 否则 `USERPROFILE`（Windows）→ `HOME`（类 Unix）⇒ 拼 `.apeireth`；
//! 3. 都没有 ⇒ 当前目录 `.`（**不 panic**）。

use std::path::{Path, PathBuf};

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

/// 一个数据根 → 它的**子目录**（如 `diary` / `sessions`）。
///
/// **为什么单独给一个函数**：`<数据根>/<名字>` 这个形状出现在多处
/// （日记目录、认知库目录…）⇒ 各写各的 `join("diary")` 就会拼错字面量。
pub fn data_subdir(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdir_is_joined_under_the_data_root() {
        let root = Path::new("/tmp/apeireth-data");
        assert_eq!(
            data_subdir(root, "diary"),
            PathBuf::from("/tmp/apeireth-data/diary")
        );
    }

    #[test]
    fn data_root_prefers_the_env_override() {
        // ⚠️ 本测试改环境变量 ⇒ 与其它并行测试可能互相干扰 ⇒ 只断言「有值时用它」，
        // 不去删别人的环境变量（改回去可能覆盖别人的设置）。
        // 真实断言交给「调用方集成测试」与 CLI 侧的既有测试。
        let original = std::env::var("APEIRETH_DATA_DIR").ok();
        std::env::set_var("APEIRETH_DATA_DIR", "/tmp/apeireth-path-test");
        let resolved = default_data_root();
        // 还原（尽力而为）。
        match original {
            Some(value) => std::env::set_var("APEIRETH_DATA_DIR", value),
            None => std::env::remove_var("APEIRETH_DATA_DIR"),
        }
        assert_eq!(resolved, PathBuf::from("/tmp/apeireth-path-test"));
    }

    #[test]
    fn data_root_falls_back_to_home() {
        // 没有 env 覆盖时 ⇒ `$HOME/.apeireth`（或 `$USERPROFILE/.apeireth`）。
        // ⚠️ 不改环境变量（避免与并行测试互相干扰）⇒ 只断言「非空且以 .apeireth 结尾」。
        let resolved = default_data_root();
        assert!(!resolved.as_os_str().is_empty());
        assert!(
            resolved.ends_with(".apeireth"),
            "数据根应以 .apeireth 结尾：{}",
            resolved.display()
        );
    }
}
