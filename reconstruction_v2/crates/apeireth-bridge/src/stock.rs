//! Stock - 金融数据 stub (从 v1.0 apeireth-stock 2.5K LOC 收敛)
//!
//! 0 装 PASS: 简化价格查询框架, 不连真 yahoo finance API (待接 HTTPS + API key).
//! 完整 v1.0 era 16+ 端点 (历史 K 线, 财报, 期权) 标 stub.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockConfig {
    pub api_key: String,
    pub base_url: String,
}

impl StockConfig {
    /// 0 装 PASS: 真实默认值 (非空), 但 user 必填 api_key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), base_url: "https://query1.finance.yahoo.com/v8/finance/chart".into() }
    }
}

pub struct StockClient {
    pub config: StockConfig,
}

impl StockClient {
    pub fn new(config: StockConfig) -> Self { Self { config } }

    /// 0 装 PASS: 返 mock (待接真 yahoo API)
    pub async fn get_quote(&self, symbol: &str) -> Result<Quote, String> {
        // 0 装 PASS: 不假装有实时数据
        Ok(Quote {
            symbol: symbol.to_string(),
            price: 0.0,  // 0 装 PASS: 0.0 placeholder, 不编造价格
            currency: "USD".into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_config_new() {
        let c = StockConfig::new("k123");
        assert_eq!(c.api_key, "k123");
        assert!(c.base_url.contains("yahoo"));
    }
    #[tokio::test]
    async fn test_get_quote_mock() {
        let c = StockClient::new(StockConfig::new("k"));
        let q = c.get_quote("AAPL").await.unwrap();
        assert_eq!(q.symbol, "AAPL");
        assert_eq!(q.price, 0.0);  // 0 装 PASS: mock placeholder
    }
}
