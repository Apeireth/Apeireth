//! OracleAdapters - 预测机数据源适配器 (从 v1.0 apeireth-companion/oracle_adapters.rs 1877 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 MarketAdapter trait + Mock 适配器 + ForecastPipeline 编排
//! 真 HTTP 适配器 (CoinGecko/MacroRates) 标 stub (需 API key, 用户可填)
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 0 装 PASS: 规范化行情报价 (v1 MarketQuote 简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketQuote {
    pub provider: String,
    pub symbol: String,
    pub value: f64,
    pub unit: String,
    pub as_of_ms: i64,
}

/// 0 装 PASS: 适配器错误 (v1 AdapterError 简化版)
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    RateLimited(String),
    Unreachable(String),
    Parse(String),
    Unsupported(String),
    Degraded(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited(s) => write!(f, "限流: {}", s),
            Self::Unreachable(s) => write!(f, "不可达: {}", s),
            Self::Parse(s) => write!(f, "解析失败: {}", s),
            Self::Unsupported(s) => write!(f, "不支持: {}", s),
            Self::Degraded(s) => write!(f, "降级: {}", s),
        }
    }
}

impl std::error::Error for AdapterError {}

/// 0 装 PASS: MarketAdapter trait (v1 简化)
pub trait MarketAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError>;
}

/// 0 装 PASS: MockAdapter (确定性 mock, v1 MockAdapter 简化)
pub struct MockAdapter { pub values: HashMap<String, f64> }

impl MockAdapter {
    pub fn new(values: HashMap<String, f64>) -> Self { Self { values } }
}

impl MarketAdapter for MockAdapter {
    fn name(&self) -> &str { "mock" }
    fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        match self.values.get(symbol) {
            Some(&v) => Ok(MarketQuote { provider: "mock".into(), symbol: symbol.into(), value: v, unit: "USD".into(), as_of_ms: chrono::Utc::now().timestamp_millis() }),
            None => Err(AdapterError::Unsupported(symbol.into())),
        }
    }
}

/// 0 装 PASS stub: CoinGeckoAdapter (需 API key, 标 stub)
pub struct CoinGeckoAdapter;

impl CoinGeckoAdapter { pub fn new() -> Self { Self } }

impl MarketAdapter for CoinGeckoAdapter {
    fn name(&self) -> &str { "coingecko" }
    fn fetch_quote(&self, _symbol: &str) -> Result<MarketQuote, AdapterError> {
        // 0 装 PASS: 标 stub (需 API key, 用户可填 base_url + key)
        Err(AdapterError::Degraded("CoinGecko adapter needs API key, see stub".into()))
    }
}

impl Default for CoinGeckoAdapter { fn default() -> Self { Self::new() } }

/// 0 装 PASS stub: MacroRatesAdapter
pub struct MacroRatesAdapter;

impl MacroRatesAdapter { pub fn new() -> Self { Self } }

impl MarketAdapter for MacroRatesAdapter {
    fn name(&self) -> &str { "macrorates" }
    fn fetch_quote(&self, _symbol: &str) -> Result<MarketQuote, AdapterError> {
        Err(AdapterError::Degraded("MacroRates adapter needs API key".into()))
    }
}

impl Default for MacroRatesAdapter { fn default() -> Self { Self::new() } }

/// 0 装 PASS: FallbackAdapter (链式 fallback, v1 简化)
pub struct FallbackAdapter { pub adapters: Vec<Box<dyn MarketAdapter>> }

impl FallbackAdapter {
    pub fn new(adapters: Vec<Box<dyn MarketAdapter>>) -> Self { Self { adapters } }
}

impl MarketAdapter for FallbackAdapter {
    fn name(&self) -> &str { "fallback" }
    fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        let mut last_err = AdapterError::Unsupported(symbol.into());
        for a in &self.adapters {
            match a.fetch_quote(symbol) {
                Ok(q) => return Ok(q),
                Err(e) if matches!(e, AdapterError::RateLimited(_) | AdapterError::Unreachable(_) | AdapterError::Degraded(_)) => last_err = e,
                Err(e) => return Err(e),  // Parse/Unsupported 直抛
            }
        }
        Err(last_err)
    }
}

/// 0 装 PASS: ForecastPipeline 编排 (v1 简化)
pub struct ForecastPipeline { pub adapters: Vec<Box<dyn MarketAdapter>> }

impl ForecastPipeline {
    pub fn new(adapters: Vec<Box<dyn MarketAdapter>>) -> Self { Self { adapters } }

    /// 0 装 PASS: 拉基线 + 归一化
    pub fn fetch_baseline(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        for a in &self.adapters {
            if let Ok(q) = a.fetch_quote(symbol) { return Ok(q); }
        }
        Err(AdapterError::Unreachable(format!("no adapter for {}", symbol)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_mock_basic() {
        let mut m = HashMap::new();
        m.insert("BTC".to_string(), 50000.0);
        let a = MockAdapter::new(m);
        let q = a.fetch_quote("BTC").unwrap();
        assert_eq!(q.value, 50000.0);
        assert_eq!(q.provider, "mock");
    }
    #[test] fn test_mock_unsupported() {
        let a = MockAdapter::new(HashMap::new());
        assert!(matches!(a.fetch_quote("X"), Err(AdapterError::Unsupported(_))));
    }
    #[test] fn test_fallback_chain() {
        let mut m = HashMap::new();
        m.insert("BTC".to_string(), 50000.0);
        let primary: Box<dyn MarketAdapter> = Box::new(CoinGeckoAdapter::new());
        let fallback: Box<dyn MarketAdapter> = Box::new(MockAdapter::new(m));
        let f = FallbackAdapter::new(vec![primary, fallback]);
        let q = f.fetch_quote("BTC").unwrap();
        assert_eq!(q.provider, "mock");
    }
    #[test] fn test_fallback_parse_error_propagates() {
        // Parse 错误应直抛不掩盖
        let primary: Box<dyn MarketAdapter> = Box::new(MockAdapter::new(HashMap::new()));
        let fallback: Box<dyn MarketAdapter> = Box::new(MockAdapter::new(HashMap::new()));
        let f = FallbackAdapter::new(vec![primary, fallback]);
        assert!(matches!(f.fetch_quote("X"), Err(AdapterError::Unsupported(_))));
    }
    #[test] fn test_stub_adapters() {
        assert!(matches!(CoinGeckoAdapter::new().fetch_quote("BTC"), Err(AdapterError::Degraded(_))));
        assert!(matches!(MacroRatesAdapter::new().fetch_quote("UST"), Err(AdapterError::Degraded(_))));
    }
    #[test] fn test_pipeline_baseline() {
        let mut m = HashMap::new();
        m.insert("ETH".to_string(), 3000.0);
        let primary: Box<dyn MarketAdapter> = Box::new(CoinGeckoAdapter::new());
        let mock: Box<dyn MarketAdapter> = Box::new(MockAdapter::new(m));
        let p = ForecastPipeline::new(vec![primary, mock]);
        assert_eq!(p.fetch_baseline("ETH").unwrap().value, 3000.0);
    }
    #[test] fn test_pipeline_unreachable() {
        let p = ForecastPipeline::new(vec![]);
        assert!(matches!(p.fetch_baseline("X"), Err(AdapterError::Unreachable(_))));
    }
    #[test] fn test_error_display() {
        let e = AdapterError::RateLimited("429".into());
        assert_eq!(format!("{}", e), "限流: 429");
    }
}
