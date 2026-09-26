//! 桌面侧配置文件的存储文档信封适配 (严格打开 / 显式迁移).
//!
//! 桌面侧配置 (`companion-config.json` / `backend-provider-env.json` /
//! `backend-capability-env.json`) 统一走主 workspace 存储基础件
//! `apeireth_core::stored_doc` 的两级容错语义:
//!
//! - **打开即校验** (单文档类): 畸形 / 串档 / 版本不符一律**拒开**报结构化错误,
//!   不「读坏用默认」;
//! - **旧裸体文件只读兼容**: 前信封格式照常读出, 但默认打开路径**不静默迁移**
//!   (不回写、不升级版本戳);
//! - **迁移必须显式**: [`migrate_legacy_config`] 走 `migrate_file_from_legacy`,
//!   重写为当前信封并追加 `<文件>.migrate-audit.jsonl` 审计行。

use std::path::Path;

use apeireth_core::stored_doc::{self, DocCompat, MigrationAudit, StoredDocError};

/// 打开配置文档: 文件缺失 = `None` (尚无配置, 是否可用默认值由调用方决定);
/// 存在但畸形/串档/版本不符 = **报错拒开** (绝不回退默认值);
/// 前信封旧裸体文件只读兼容 (不静默迁移)。
pub fn load_config<T: serde::de::DeserializeOwned>(
    path: &Path,
    compat: &DocCompat,
) -> Result<Option<T>, String> {
    match stored_doc::open_single_compat::<T>(path, compat) {
        Ok(opened) => Ok(Some(opened.into_body())),
        Err(StoredDocError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(None)
        }
        Err(error) => Err(format!("配置文件拒开 ({}): {error}", path.display())),
    }
}

/// 保存配置文档 (信封 + 原子持久写): 盖当前版本戳。
pub fn save_config<T: serde::Serialize>(
    path: &Path,
    compat: &DocCompat,
    body: &T,
) -> Result<(), String> {
    stored_doc::save_single(path, compat, body, stored_doc::DEFAULT_DOC_MODE)
        .map(|_| ())
        .map_err(|error| format!("配置写入失败 ({}): {error}", path.display()))
}

/// 显式迁移旧裸体配置 → 当前信封格式: 重写文件 + 追加审计行。
///
/// 这是**显式**入口: 默认打开路径 ([`load_config`]) 永不迁移; 失败不落盘。
pub fn migrate_legacy_config<T>(path: &Path, compat: &DocCompat) -> Result<MigrationAudit, String>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    stored_doc::migrate_file_from_legacy::<T, T, _>(path, compat, stored_doc::DEFAULT_DOC_MODE, Ok)
        .map_err(|error| format!("配置迁移失败 ({}): {error}", path.display()))
}
