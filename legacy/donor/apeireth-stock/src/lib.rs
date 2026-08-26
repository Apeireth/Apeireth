//! TP27 标的元数据资产 (N3 金融源).
//!
//! 数据流: FinanceDatabase CSV → `SymbolMeta` 行 → SQLite `symbols` 表 → `SymbolCatalog` 查询.
//!
//! 设计要点:
//! - **独立 Provenance 枚举** (避免依赖 `apeireth-memory` 循环依赖)
//! - **CSV 解析容错**: 缺字段空字符串 / Option 列空值 / 非法行跳过 + 计数 (不假成功)
//! - **SQLite 批量入库**: 单事务包裹, 30 万条 < 5s (per row INSERT 慢)
//! - **索引**: sector / industry / exchange 各 1 索引, 加速过滤
//! - **0 装 PASS**: FinanceDatabase 网络下载 0 装 PASS (避免 reqwest 重依赖),
//!   缓存命中/local source 路径实测覆盖
//!
//! API 边界 (新 spec `eea4e3dd`):
//! - [`SymbolMeta`] 标的元数据 (13 字段, 新增 ipo_date/delisted_date)
//! - [`Provenance`] 数据源标记 (本地枚举, T0 信任等级)
//! - [`SymbolStore`] SQLite 入库 + 查询 (V6 migration, 加列而非改列)
//! - [`SymbolCatalog`] trait 查询接口 (旧 + 新 spec 双套 API)
//! - [`import_from_csv`] 批量导入 (容错 + 计数)
//! - [`refresh`] 数据刷新 (网络/缓存兜底, per 新 spec 边界 #2/#4)

pub mod catalog;
pub mod csv;
pub mod refresh;
pub mod store;
pub mod symbol;

pub use catalog::SymbolCatalog;
pub use csv::{import_from_csv, CsvImportStats};
pub use refresh::{
    refresh, refresh_and_import, DataSource, RefreshError, RefreshOutcome,
    FINANCE_DATABASE_EQUITIES_CSV, FINANCE_DATABASE_RAW_BASE,
};
pub use store::{SymbolStore, SymbolStoreError};
pub use symbol::{Provenance, SymbolMeta};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_wires() {
        // 简单烟雾测试: 各模块可见
        let _ = SymbolMeta::default();
        let _ = Provenance::default();
        let _ = CsvImportStats::default();
        let _ = DataSource::default_equities();
    }

    #[test]
    fn provenance_default_is_manual() {
        assert_eq!(Provenance::default(), Provenance::Manual);
    }

    #[test]
    fn symbol_meta_default_empty() {
        let m = SymbolMeta::default();
        assert!(m.symbol.is_empty());
        assert!(m.market_cap.is_none());
        assert!(m.ipo_year.is_none());
        assert!(m.ipo_date.is_none());
        assert!(m.delisted_date.is_none());
        assert_eq!(m.provenance, Provenance::Manual);
    }
}
