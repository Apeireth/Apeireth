//! SymbolCatalog trait — 查询接口 (per task spec).
//!
//! `SymbolStore` 实现此 trait; 外部通过 `dyn SymbolCatalog` 注入使用.
//!
//! 新 spec (eea4e3dd) 4 个方法命名:
//! - `get_by_ticker` (旧 spec `get`)
//! - `search_by_industry` (旧 spec `search` 子集)
//! - `list_by_exchange` (旧 spec `search` 子集)
//! - `count_all` (旧 spec `count`)
//! 旧 spec 方法保留 (向后兼容).

use crate::symbol::SymbolMeta;

/// 标的目录查询接口 (per task 验收 + 新 spec 扩展).
pub trait SymbolCatalog: Send + Sync {
    // ============ 旧 spec API (向后兼容) ============

    /// 按主键查询单条 (旧 API).
    fn get(&self, symbol: &str) -> Option<SymbolMeta>;
    /// 多字段过滤 (任一字段 None 即不参与; limit 截断).
    ///
    /// 排序约定: market_cap DESC 优先 (有市值的标的优先), 然后 symbol ASC 稳定.
    fn search(
        &self,
        sector: Option<&str>,
        industry: Option<&str>,
        exchange: Option<&str>,
        limit: usize,
    ) -> Vec<SymbolMeta>;
    /// 标的总数 (旧 API).
    fn count(&self) -> usize;

    // ============ 新 spec API (eea4e3dd) ============

    /// 按 ticker 查询单条 (新 spec, ticker == symbol).
    fn get_by_ticker(&self, ticker: &str) -> Option<SymbolMeta> {
        self.get(ticker)
    }

    /// 按行业搜索 (新 spec).
    fn search_by_industry(&self, industry: &str, limit: usize) -> Vec<SymbolMeta> {
        self.search(None, Some(industry), None, limit)
    }

    /// 按交易所列出 (新 spec).
    fn list_by_exchange(&self, exchange: &str, limit: usize) -> Vec<SymbolMeta> {
        self.search(None, None, Some(exchange), limit)
    }

    /// 标的总数 (新 spec API).
    fn count_all(&self) -> usize {
        self.count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SymbolStore;
    use crate::symbol::Provenance;

    fn make(s: &str, sector: &str) -> SymbolMeta {
        SymbolMeta {
            symbol: s.into(),
            name: format!("{s} Inc."),
            sector: sector.into(),
            industry: "Test".into(),
            exchange: "NYSE".into(),
            country: "US".into(),
            currency: "USD".into(),
            market_cap: Some(100.0),
            ipo_year: Some(2000),
            ipo_date: None,
            delisted_date: None,
            provenance: Provenance::FinanceDatabase,
            last_updated_ms: 0,
        }
    }

    fn dyn_catalog() -> Box<dyn SymbolCatalog> {
        Box::new(SymbolStore::open_in_memory().unwrap())
    }

    #[test]
    fn trait_object_basic() {
        let store = SymbolStore::open_in_memory().unwrap();
        store.upsert(&make("A", "Tech")).unwrap();
        store.upsert(&make("B", "Finance")).unwrap();

        let cat: &dyn SymbolCatalog = &store;
        assert_eq!(cat.count(), 2);
        assert_eq!(cat.get("A").unwrap().name, "A Inc.");
        let r = cat.search(Some("Tech"), None, None, 10);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn dyn_box_call() {
        // 验证 dyn dispatch 路径
        let cat: Box<dyn SymbolCatalog> = dyn_catalog();
        cat.get("X"); // 编译期证明 trait dispatch 可用
    }

    #[test]
    fn trait_new_spec_api() {
        let store = SymbolStore::open_in_memory().unwrap();
        store.upsert(&make("AAPL", "Tech")).unwrap();

        let cat: &dyn SymbolCatalog = &store;
        // 新 spec API
        assert_eq!(cat.get_by_ticker("AAPL").unwrap().ticker(), "AAPL");
        assert_eq!(cat.count_all(), 1);
        let r = cat.search_by_industry("Test", 10);
        assert_eq!(r.len(), 1);
        let r2 = cat.list_by_exchange("NYSE", 10);
        assert_eq!(r2.len(), 1);
    }
}