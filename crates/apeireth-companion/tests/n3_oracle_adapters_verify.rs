//! N3 验收临时集成测试: 在并行 WIP 阻塞 lib test 目标期间, 独立验证 oracle_adapters
//! 公共 API 全路径 (拉取/规范化/失败降级/到期 resolve + Brier/校准挂接).
//! 注: lib 单测 (oracle_adapters.rs 内 13 用例) 是正式验收, 本文件为过渡验证, 通过后可删.

use std::sync::Arc;

use apeireth_companion::oracle_adapters::*;
use apeireth_memory::SqliteMemoryStore;
use async_trait::async_trait;

struct MockRawFetch {
    status: u16,
    body: String,
}

#[async_trait]
impl RawFetch for MockRawFetch {
    async fn get(&self, _url: &str) -> Result<(u16, String), AdapterError> {
        Ok((self.status, self.body.clone()))
    }
}

fn mem() -> Arc<SqliteMemoryStore> {
    Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
}

#[tokio::test]
async fn coingecko_and_macro_adapters_via_mock_raw() {
    // 加密: 200 → 规范化报价; 429 → 可降级限流
    let ok = CoinGeckoAdapter::with_raw(Arc::new(MockRawFetch { status: 200, body: r#"{"bitcoin":{"usd":61234.5}}"#.into() }));
    let q = ok.fetch_quote("BTC").await.unwrap();
    assert_eq!(q.provider, "coingecko");
    assert_eq!(q.unit, "USD");
    assert!((q.value - 61234.5).abs() < 1e-9);
    let rl = CoinGeckoAdapter::with_raw(Arc::new(MockRawFetch { status: 429, body: "limited".into() }));
    assert!(rl.fetch_quote("BTC").await.unwrap_err().degradable());
    // 宏观利率: 200 → %; 未知 symbol → Unsupported
    let body = r#"{"data":[{"attributes":{"avg_interest_rate_amt":"3.51"}}]}"#;
    let m = MacroRatesAdapter::with_raw(Arc::new(MockRawFetch { status: 200, body: body.into() }));
    let q = m.fetch_quote(TREASURY_AVG_RATE).await.unwrap();
    assert_eq!(q.unit, "%");
    assert_eq!(m.fetch_quote("CPI").await.unwrap_err(), AdapterError::Unsupported("CPI".into()));
}

#[tokio::test]
async fn fallback_degrades_full_pipeline() {
    // 主源限流 → mock 降级 → 登记/到期 resolve 全路径不阻塞
    let primary = Arc::new(MockAdapter::new("primary"));
    primary.set_quote("BTC", 100.0);
    primary.fail_with(AdapterError::RateLimited("429".into()));
    let fallback = Arc::new(MockAdapter::new("fallback"));
    fallback.set_quote("BTC", 100.0);
    let fa = Arc::new(FallbackAdapter::new(primary.clone(), fallback.clone()));
    // Unsupported 不降级 (能力边界直抛)
    assert_eq!(fa.fetch_quote("XYZ").await.unwrap_err(), AdapterError::Unsupported("XYZ".into()));

    let p = ForecastPipeline::new(fa, mem(), "sess-n3");
    let df = p.register_direction_forecast("BTC", 0, 0.6).await.unwrap();
    assert_eq!(df.baseline.provider, "fallback");
    fallback.set_quote("BTC", 150.0);
    let out = p.resolve_due(&df.forecast_id).await.unwrap();
    assert!(out.actual);
    assert!((out.brier - 0.16).abs() < 1e-9); // (0.6-1)²
    let (n, mean_brier, _hint) = p.registry().calibration().unwrap();
    assert_eq!(n, 1);
    assert!((mean_brier - 0.16).abs() < 1e-9);
}

#[tokio::test]
async fn pipeline_edges() {
    let mock = Arc::new(MockAdapter::new("mock"));
    mock.set_quote("BTC", 100.0);
    let p = ForecastPipeline::new(mock.clone(), mem(), "sess-n3-2");
    // 未到期
    let df_future = p.register_direction_forecast("BTC", 60_000, 0.5).await.unwrap();
    assert!(p.resolve_due(&df_future.forecast_id).await.unwrap_err().contains("未到期"));
    // 平盘判未成真
    let df = p.register_direction_forecast("BTC", 0, 0.9).await.unwrap();
    let out = p.resolve_due(&df.forecast_id).await.unwrap();
    assert!(!out.actual);
    assert!((out.brier - 0.81).abs() < 1e-9);
    // 重复 resolve 报错
    assert!(p.resolve_due(&df.forecast_id).await.unwrap_err().contains("已 resolve"));
    // 登记后涨价 → 对照成真
    let df2 = p.register_direction_forecast("BTC", 0, 0.5).await.unwrap(); // 基线 100
    mock.set_quote("BTC", 130.0);
    let out2 = p.resolve_due(&df2.forecast_id).await.unwrap();
    assert!(out2.actual);
}

#[tokio::test]
async fn adapter_registry_hotplug() {
    let mut reg = AdapterRegistry::new();
    reg.register(Arc::new(MockAdapter::new("mock")));
    reg.register(Arc::new(MacroRatesAdapter::with_raw(Arc::new(MockRawFetch { status: 200, body: r#"{"data":[{"attributes":{"avg_interest_rate_amt":"3.51"}}]}"#.into() }))));
    assert_eq!(reg.list(), vec!["macro-rates".to_string(), "mock".to_string()]);
    assert!(reg.get("mock").is_some());
    assert!(reg.get("不存在").is_none());
}
