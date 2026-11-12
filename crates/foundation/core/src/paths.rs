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
    let override_dir = std::env::var("APEIRETH_DATA_DIR").ok();
    let profile = std::env::var("USERPROFILE").ok();
    let home = std::env::var("HOME").ok();
    resolve_data_root(override_dir.as_deref(), profile.as_deref(), home.as_deref())
}

fn resolve_data_root(
    override_dir: Option<&str>,
    profile: Option<&str>,
    home: Option<&str>,
) -> PathBuf {
    if let Some(dir) = override_dir.filter(|dir| !dir.trim().is_empty()) {
        return PathBuf::from(dir);
    }
    PathBuf::from(profile.or(home).unwrap_or(".")).join(".apeireth")
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
        assert_eq!(
            resolve_data_root(
                Some("/tmp/apeireth-path-test"),
                Some("profile"),
                Some("home")
            ),
            PathBuf::from("/tmp/apeireth-path-test")
        );
        assert_eq!(
            resolve_data_root(Some("  custom-root  "), None, None),
            PathBuf::from("  custom-root  "),
            "a nonempty override is preserved verbatim"
        );
    }

    #[test]
    fn data_root_falls_back_to_home() {
        assert_eq!(
            resolve_data_root(None, Some("profile"), Some("home")),
            PathBuf::from("profile").join(".apeireth")
        );
        assert_eq!(
            resolve_data_root(Some(" \t"), None, Some("home")),
            PathBuf::from("home").join(".apeireth")
        );
        assert_eq!(
            resolve_data_root(None, None, None),
            PathBuf::from(".").join(".apeireth")
        );
        assert_eq!(
            resolve_data_root(None, Some(""), Some("home")),
            PathBuf::from("").join(".apeireth"),
            "an existing empty profile keeps the original precedence"
        );
    }

    #[test]
    fn production_root_uses_the_current_environment_without_mutating_it() {
        let override_dir = std::env::var("APEIRETH_DATA_DIR").ok();
        let profile = std::env::var("USERPROFILE").ok();
        let home = std::env::var("HOME").ok();
        assert_eq!(
            default_data_root(),
            resolve_data_root(override_dir.as_deref(), profile.as_deref(), home.as_deref())
        );
        assert_eq!(std::env::var("APEIRETH_DATA_DIR").ok(), override_dir);
    }
}
