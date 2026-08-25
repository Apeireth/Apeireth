//! apeireth-stock - Stock data store (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 StockStore + 真 symbol catalog

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock { pub symbol: String, pub name: String, pub sector: String }

pub struct StockCatalog { pub items: HashMap<String, Stock> }

impl StockCatalog {
    pub fn new() -> Self { Self { items: HashMap::new() } }
    pub fn add(&mut self, s: Stock) { self.items.insert(s.symbol.clone(), s); }
    pub fn get(&self, sym: &str) -> Option<&Stock> { self.items.get(sym) }
    pub fn by_sector(&self, sec: &str) -> Vec<&Stock> {
        self.items.values().filter(|s| s.sector == sec).collect()
    }
}

impl Default for StockCatalog { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut c = StockCatalog::new();
        c.add(Stock { symbol: "AAPL".into(), name: "Apple".into(), sector: "Tech".into() });
        assert_eq!(c.get("AAPL").unwrap().name, "Apple");
    }
    #[test]
    fn test_sector() {
        let mut c = StockCatalog::new();
        c.add(Stock { symbol: "A".into(), name: "a".into(), sector: "Tech".into() });
        c.add(Stock { symbol: "B".into(), name: "b".into(), sector: "Fin".into() });
        assert_eq!(c.by_sector("Tech").len(), 1);
    }
}
