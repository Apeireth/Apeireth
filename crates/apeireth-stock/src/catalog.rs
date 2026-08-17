//! SymbolCatalog trait — 查询接口 (per task spec).
//!
//! `SymbolStore` 实现此 trait; 外部通过 `dyn SymbolCatalog` 注入使用.

use crate::symbol::SymbolMeta;

/// 标的目录查询接口 (per task 验收 #5).
pub trait SymbolCatalog: Send + Sync {
    /// 按主键查询单条.
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
    /// 标的总数.
    fn count(&self) -> usize;
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
}