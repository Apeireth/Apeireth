//! 标的元数据 + Provenance 枚举.

use serde::{Deserialize, Serialize};

/// 数据源可信度 (T0 = 官方一手, T1 = 验证二手, T2 = 推算, ...).
///
/// 本 crate 暂只实现 `FinanceDatabase` (T0) — `apeireth-memory` 的 5 变体
/// (Dialog/Tool/Reflection/Observation/Manual) 不适用于静态数据资产.
///
/// 序列化: snake_case 字符串, 与 `apeireth_memory::Provenance` 命名一致 (便于跨 crate serde 互转).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// FinanceDatabase (官方 GitHub 仓库, 30 万+ 标的, T0 信任等级).
    #[serde(rename = "finance_database")]
    FinanceDatabase,
    /// 手工注入 (测试 / 一次性 fix).
    #[default]
    Manual,
}

impl Provenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provenance::FinanceDatabase => "finance_database",
            Provenance::Manual => "manual",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "finance_database" => Provenance::FinanceDatabase,
            _ => Provenance::Manual,
        }
    }
}

/// 一条标的元数据 (per task spec).
///
/// 字段映射 (CSV → struct):
/// - `symbol` / `ticker()`: 主键, 必填 (两者等价, ticker 是 accessor)
/// - `name` / `sector` / `industry` / `exchange` / `country` / `currency`: 字符串, 空字符串兜底
/// - `market_cap`: Option<f64>, 空值/非法 → None
/// - `ipo_year`: Option<i32> (兼容旧 spec)
/// - `ipo_date`: Option<String> (新 spec, ISO 8601 字符串如 "1980-12-12")
/// - `delisted_date`: Option<String> (新 spec, 同格式)
/// - `provenance`: 默认 `FinanceDatabase` (T0)
/// - `last_updated_ms`: epoch ms, 默认当前时间
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SymbolMeta {
    pub symbol: String,
    pub name: String,
    pub sector: String,
    pub industry: String,
    pub exchange: String,
    pub country: String,
    pub currency: String,
    #[serde(default)]
    pub market_cap: Option<f64>,
    #[serde(default)]
    pub ipo_year: Option<i32>,
    /// 新 spec 字段: IPO 日期 (ISO 8601 字符串, 如 "1980-12-12").
    #[serde(default)]
    pub ipo_date: Option<String>,
    /// 新 spec 字段: 退市日期 (ISO 8601 字符串).
    #[serde(default)]
    pub delisted_date: Option<String>,
    #[serde(default)]
    pub provenance: Provenance,
    pub last_updated_ms: i64,
}

impl SymbolMeta {
    /// 行是否合法 (symbol 非空).
    pub fn is_valid(&self) -> bool {
        !self.symbol.trim().is_empty()
    }

    /// Ticker accessor (新 spec 命名约定, 与 `symbol` 字段等价).
    pub fn ticker(&self) -> &str {
        &self.symbol
    }

    /// 给数据库写入用 (13 列元组: 11 旧 + ipo_date + delisted_date).
    pub fn to_row(&self) -> Vec<rusqlite::types::Value> {
        use rusqlite::types::Value;
        vec![
            Value::Text(self.symbol.clone()),
            Value::Text(self.name.clone()),
            Value::Text(self.sector.clone()),
            Value::Text(self.industry.clone()),
            Value::Text(self.exchange.clone()),
            Value::Text(self.country.clone()),
            Value::Text(self.currency.clone()),
            match self.market_cap {
                Some(v) => Value::Real(v),
                None => Value::Null,
            },
            match self.ipo_year {
                Some(v) => Value::Integer(v as i64),
                None => Value::Null,
            },
            match &self.ipo_date {
                Some(s) => Value::Text(s.clone()),
                None => Value::Null,
            },
            match &self.delisted_date {
                Some(s) => Value::Text(s.clone()),
                None => Value::Null,
            },
            Value::Text(self.provenance.as_str().to_string()),
            Value::Integer(self.last_updated_ms),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_roundtrip() {
        assert_eq!(
            Provenance::from_db("finance_database"),
            Provenance::FinanceDatabase
        );
        assert_eq!(Provenance::from_db("manual"), Provenance::Manual);
        assert_eq!(Provenance::from_db("garbage"), Provenance::Manual); // 未知降级 Manual
        assert_eq!(Provenance::FinanceDatabase.as_str(), "finance_database");
        assert_eq!(Provenance::Manual.as_str(), "manual");
    }

    #[test]
    fn provenance_serde_roundtrip() {
        let p = Provenance::FinanceDatabase;
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(s, "\"finance_database\"");
        let back: Provenance = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn symbol_meta_serde_full() {
        let m = SymbolMeta {
            symbol: "AAPL".into(),
            name: "Apple Inc.".into(),
            sector: "Technology".into(),
            industry: "Consumer Electronics".into(),
            exchange: "NASDAQ".into(),
            country: "US".into(),
            currency: "USD".into(),
            market_cap: Some(2_900_000_000_000.0),
            ipo_year: Some(1980),
            ipo_date: Some("1980-12-12".into()),
            delisted_date: None,
            provenance: Provenance::FinanceDatabase,
            last_updated_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: SymbolMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn symbol_meta_serde_optional_none() {
        let m = SymbolMeta {
            symbol: "TEST".into(),
            name: "Test Co.".into(),
            sector: "".into(),
            industry: "".into(),
            exchange: "".into(),
            country: "".into(),
            currency: "".into(),
            market_cap: None,
            ipo_year: None,
            ipo_date: None,
            delisted_date: None,
            provenance: Provenance::Manual,
            last_updated_ms: 100,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: SymbolMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert!(back.market_cap.is_none());
        assert!(back.ipo_year.is_none());
        assert!(back.ipo_date.is_none());
        assert!(back.delisted_date.is_none());
    }

    #[test]
    fn symbol_meta_serde_with_delisted_date() {
        // 测试 delisted 字段存在场景
        let m = SymbolMeta {
            symbol: "LEHQY".into(),
            name: "Lehman Brothers Holdings Inc".into(),
            ipo_date: Some("1994-09-13".into()),
            delisted_date: Some("2008-09-15".into()),
            ..SymbolMeta::default()
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: SymbolMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.delisted_date, Some("2008-09-15".to_string()));
    }

    #[test]
    fn is_valid_requires_symbol() {
        let mut m = SymbolMeta::default();
        assert!(!m.is_valid());
        m.symbol = "AAPL".into();
        assert!(m.is_valid());
        m.symbol = "   ".into();
        assert!(!m.is_valid());
    }

    #[test]
    fn ticker_accessor_returns_symbol() {
        let m = SymbolMeta {
            symbol: "BRK.B".into(),
            ..SymbolMeta::default()
        };
        assert_eq!(m.ticker(), "BRK.B");
    }

    #[test]
    fn to_row_includes_all_13_fields() {
        // 13 = 11 旧 + ipo_date + delisted_date
        let m = SymbolMeta {
            symbol: "AAPL".into(),
            market_cap: Some(100.0),
            ipo_year: Some(2000),
            ipo_date: Some("2000-01-01".into()),
            delisted_date: None,
            ..SymbolMeta::default()
        };
        let row = m.to_row();
        assert_eq!(row.len(), 13);
        assert_eq!(row[0], rusqlite::types::Value::Text("AAPL".into()));
        assert_eq!(row[7], rusqlite::types::Value::Real(100.0));
        assert_eq!(row[8], rusqlite::types::Value::Integer(2000));
        assert_eq!(row[9], rusqlite::types::Value::Text("2000-01-01".into()));
        assert_eq!(row[10], rusqlite::types::Value::Null); // delisted_date
    }
}
