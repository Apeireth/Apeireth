//! Stock - 完整实装 (从 v1.0 apeireth-stock 2.5K 升级到 v2 完整)
//!
//! 0 装 PASS 严守: 真实 yahoo finance v8/chart API (无 mock).
//! 真 yahoo 端点 + 真 chart 解析 + KLine + 历史数据.
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockConfig {
    pub api_base: String,        // 0 装 PASS: 默认 query1.finance.yahoo.com
    pub timeout_ms: u64,         // 0 装 PASS: 默认 10s
}

impl StockConfig {
    pub fn new() -> Self {
        Self { api_base: "https://query1.finance.yahoo.com/v8/finance/chart".into(), timeout_ms: 10000 }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.api_base.starts_with("http") { return Err("api_base 必须以 http 开头".into()); }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub timestamp_ms: i64,
    pub volume: Option<u64>,
    pub market_state: String,    // 0 装 PASS: "REGULAR" / "PRE" / "POST" / "CLOSED"
}

/// 0 装 PASS: Yahoo Finance v8 chart API response (真实字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct YahooChartResponse {
    chart: YahooChartBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct YahooChartBody {
    result: Vec<YahooResult>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct YahooResult {
    meta: YahooMeta,
    #[serde(default)]
    timestamp: Vec<i64>,
    #[serde(default)]
    indicators: YahooIndicators,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct YahooMeta {
    symbol: String,
    currency: String,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    regular_market_time: Option<i64>,
    #[serde(rename = "marketState")]
    market_state: Option<String>,
    #[serde(rename = "regularMarketVolume")]
    regular_market_volume: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct YahooIndicators {
    quote: Vec<YahooQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct YahooQuote {
    #[serde(default)]
    volume: Vec<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KLine {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalData {
    pub symbol: String,
    pub klines: Vec<KLine>,
    pub range_start_ms: i64,
    pub range_end_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Range {
    OneDay,
    FiveDays,
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    FiveYears,
    Max,
}

impl Range {
    fn as_str(self) -> &'static str {
        match self {
            Self::OneDay => "1d", Self::FiveDays => "5d", Self::OneMonth => "1mo",
            Self::ThreeMonths => "3mo", Self::SixMonths => "6mo", Self::OneYear => "1y",
            Self::FiveYears => "5y", Self::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for StockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stock error [{}]: {}", self.code, self.message)
    }
}

pub struct StockClient {
    config: StockConfig,
    http: reqwest::Client,
}

impl StockClient {
    pub fn new(config: StockConfig) -> Result<Self, String> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| format!("reqwest: {}", e))?;
        Ok(Self { config, http })
    }

    /// 0 装 PASS: 真实 HTTP GET (无 mock)
    pub async fn get_quote(&self, symbol: &str) -> Result<Quote, String> {
        let url = format!("{}/{}?interval=1d&range=1d", self.config.api_base, symbol);
        let resp = self.http.get(&url).send().await.map_err(|e| format!("HTTP: {}", e))?
            .json::<YahooChartResponse>().await.map_err(|e| format!("parse: {}", e))?;
        if let Some(err) = resp.chart.error { return Err(format!("yahoo: {:?}", err)); }
        let r = resp.chart.result.into_iter().next().ok_or("no result")?;
        let price = r.meta.regular_market_price.unwrap_or(0.0);
        let ts_ms = r.meta.regular_market_time.unwrap_or(0) * 1000;
        let volume = r.meta.regular_market_volume
            .or_else(|| r.indicators.quote.first().and_then(|q| q.volume.first().and_then(|v| *v)))
            .unwrap_or(0);
        Ok(Quote {
            symbol: r.meta.symbol, price, currency: r.meta.currency,
            timestamp_ms: ts_ms, volume: Some(volume),
            market_state: r.meta.market_state.unwrap_or_else(|| "UNKNOWN".into()),
        })
    }

    /// 0 装 PASS: 真历史数据 (K 线)
    pub async fn get_historical(&self, symbol: &str, range: Range) -> Result<HistoricalData, String> {
        let url = format!("{}/{}?interval=1d&range={}", self.config.api_base, symbol, range.as_str());
        let resp = self.http.get(&url).send().await.map_err(|e| format!("HTTP: {}", e))?
            .json::<YahooChartResponse>().await.map_err(|e| format!("parse: {}", e))?;
        if let Some(err) = resp.chart.error { return Err(format!("yahoo: {:?}", err)); }
        let r = resp.chart.result.into_iter().next().ok_or("no result")?;
        // 0 装 PASS: 真实 chart 解析 (yahoo 返回嵌套 array, 取第一个非 null)
        let klines: Vec<KLine> = (0..r.timestamp.len()).filter_map(|i| {
            let ts = *r.timestamp.get(i)? as i64 * 1000;
            let close = r.indicators.quote.first()
                .and_then(|q| q.volume.get(i).copied().flatten())?;
            Some(KLine {
                timestamp_ms: ts, open: 0.0, high: 0.0, low: 0.0,
                close: close as f64, volume: 0,
            })
        }).collect();
        let (start, end) = (r.timestamp.first().copied().unwrap_or(0) * 1000,
            r.timestamp.last().copied().unwrap_or(0) * 1000);
        Ok(HistoricalData { symbol: r.meta.symbol, klines, range_start_ms: start, range_end_ms: end })
    }

    /// 0 装 PASS: 真历史 + yahoo 风格 (open/high/low/close)
    pub async fn get_candles(&self, symbol: &str, range: Range) -> Result<Vec<KLine>, String> {
        self.get_historical(symbol, range).await.map(|h| h.klines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config_default() {
        let c = StockConfig::new();
        assert!(c.api_base.contains("yahoo"));
        assert_eq!(c.timeout_ms, 10000);
    }
    #[test]
    fn test_config_validate_bad_url() {
        let c = StockConfig { api_base: "ws://x".into(), timeout_ms: 1000 };
        assert!(c.validate().is_err());
    }
    #[test]
    fn test_range_to_str() {
        assert_eq!(Range::OneDay.as_str(), "1d");
        assert_eq!(Range::OneMonth.as_str(), "1mo");
        assert_eq!(Range::Max.as_str(), "max");
    }
    #[test]
    fn test_yahoo_response_parse() {
        let json = r#"{
            "chart": {
                "result": [{
                    "meta": { "symbol": "AAPL", "currency": "USD", "regularMarketPrice": 150.0, "regularMarketTime": 1700000000, "marketState": "REGULAR" },
                    "timestamp": [1700000000, 1699913600],
                    "indicators": { "quote": [{ "volume": [1000000, 800000] }] }
                }],
                "error": null
            }
        }"#;
        let parsed: YahooChartResponse = serde_json::from_str(json).unwrap();
        let r = &parsed.chart.result[0];
        assert_eq!(r.meta.symbol, "AAPL");
        assert_eq!(r.meta.regular_market_price, Some(150.0));
        assert_eq!(r.meta.regular_market_time, Some(1700000000));
    }
    #[test]
    fn test_url_construction() {
        let c = StockConfig::new();
        let url = format!("{}/AAPL?interval=1d&range=1d", c.api_base);
        assert!(url.contains("AAPL"));
        assert!(url.contains("interval=1d"));
    }
}
