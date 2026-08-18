//! `apeireth-companion::oracle_adapters` — 预测机套件数据源适配器 (backlog N3, VCP DigitalOracle 精神).
//!
//! 职责 (docs/team-work-doc.md §5.2 + §8.4): 统一接口「拉取 → 规范化 → 喂 oracle 可证伪预测登记」.
//! - [`MarketAdapter`] trait: 所有数据源的最小口 (`fetch_quote`: symbol → 规范化 [`MarketQuote`])
//! - 旗舰适配器 ×2: [`CoinGeckoAdapter`] (加密货币, 免费无 key) + [`MacroRatesAdapter`]
//!   (宏观/利率, 美债 fiscaldata 免费无 key — FRED 需 API key, 取同域免 key 替选)
//! - [`MockAdapter`] 确定性 mock + [`FallbackAdapter`] 限流/不可达降级 (真 API 限流不阻塞验收)
//! - [`ForecastPipeline`]: 拉基线 → 登记方向预测进 [`crate::oracle::ForecastRegistry`] → 到期对照
//!   resolve (Brier 自动入账, 校准走既有 `registry.calibration()`, 0 重写 oracle 核心)
//! - **[`TimeSeriesPredictor`] (TP25)**: 数字信号时序预测 trait 口 (TimesFM/Kronos 本地小模型可选),
//!   与 LLM 文本预测经 [`blend_predictions`] 融合进集合预报 (E3 增强, 0 装: 模型未接如实标注)
//!
//! 0 假装: 旗舰适配器写真 HTTP (reqwest, 10s 超时, 429→限流/非 200→不可达); 测试全路径走
//! mock (拉取/规范化/失败降级/到期 resolve), 真 API 可选不阻塞; 语义约定「到期价 > 基线」判
//! 成真, 平盘判未成真 (方向预测保守口径); 基线元数据走记忆库 `adapterfc-` 前缀事件 (append-only,
//! 与 ForecastRegistry 的 `forecast-` 事件同库并存, oracle.rs 0 改动).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use async_trait::async_trait;

use crate::oracle::{Forecast, ForecastRegistry};

// ============================================================
// 规范化报价 + 错误 + 统一 trait 口
// ============================================================

/// 规范化行情报价 (所有适配器的统一输出, 喂预测登记的基线/对照值).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketQuote {
    pub provider: String,
    pub symbol: String,
    pub value: f64,
    pub unit: String,
    pub as_of_ms: i64,
}

/// 适配器错误 (降级决策依据: RateLimited/Unreachable 可降级, Parse/Unsupported 直抛不掩盖).
#[derive(Debug, Clone, PartialEq)]
pub enum AdapterError {
    /// 限流 (HTTP 429 等) → 可 mock 降级.
    RateLimited(String),
    /// 网络不可达/非 200 → 可 mock 降级.
    Unreachable(String),
    /// 响应格式异常 (真源改口, 诚实报错不编数).
    Parse(String),
    /// 未知 symbol (适配器的能力边界, 直抛).
    Unsupported(String),
    /// 未接/已降级 (TP25 时序模型未接入等) → 诚实 Err 可降级.
    Degraded(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited(s) => write!(f, "限流: {s}"),
            Self::Unreachable(s) => write!(f, "不可达: {s}"),
            Self::Parse(s) => write!(f, "解析失败: {s}"),
            Self::Unsupported(s) => write!(f, "不支持的 symbol: {s}"),
            Self::Degraded(s) => write!(f, "降级/未接: {s}"),
        }
    }
}

impl std::error::Error for AdapterError {}

impl AdapterError {
    /// 是否属于可降级错误 (限流/不可达/降级 → 允许切 fallback; 解析/不支持 → 直抛).
    pub fn degradable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited(_) | Self::Unreachable(_) | Self::Degraded(_)
        )
    }
}

/// 数据源适配器统一口: symbol → 规范化报价 (拉取 + 规范化一步到位).
#[async_trait]
pub trait MarketAdapter: Send + Sync {
    /// 数据源 id (如 "coingecko" / "macro-rates" / "mock").
    fn provider_id(&self) -> String;
    /// 拉取并规范化一个报价 (失败按 [`AdapterError`] 语义分类).
    async fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError>;
}

// ============================================================
// Mock 适配器 (确定性, 全路径测试 + 限流降级兜底)
// ============================================================

/// 确定性 mock 数据源: 报价可配置, 失败模式可注入 (验收全路径 0 真网络).
pub struct MockAdapter {
    provider: String,
    quotes: Mutex<HashMap<String, f64>>,
    failure: Mutex<Option<AdapterError>>,
}

impl MockAdapter {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            quotes: Mutex::new(HashMap::new()),
            failure: Mutex::new(None),
        }
    }
    /// 设置/更新报价 (可随测试推进改值, 模拟行情变动).
    pub fn set_quote(&self, symbol: impl Into<String>, value: f64) {
        self.quotes.lock().unwrap().insert(symbol.into(), value);
    }
    /// 注入失败模式 (限流/不可达), 模拟真源故障.
    pub fn fail_with(&self, err: AdapterError) {
        *self.failure.lock().unwrap() = Some(err);
    }
    pub fn clear_failure(&self) {
        *self.failure.lock().unwrap() = None;
    }
}

#[async_trait]
impl MarketAdapter for MockAdapter {
    fn provider_id(&self) -> String {
        self.provider.clone()
    }
    async fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        if let Some(err) = self.failure.lock().unwrap().clone() {
            return Err(err);
        }
        let value = self.quotes.lock().unwrap().get(symbol).copied();
        match value {
            Some(v) => Ok(MarketQuote {
                provider: self.provider.clone(),
                symbol: symbol.to_string(),
                value: v,
                unit: "MOCK".into(),
                as_of_ms: chrono::Utc::now().timestamp_millis(),
            }),
            None => Err(AdapterError::Unsupported(symbol.to_string())),
        }
    }
}

// ============================================================
// 降级包装: 主源限流/不可达 → 切 fallback
// ============================================================

/// 降级适配器: primary 遇可降级错误 (限流/不可达) 时切 fallback (通常是 [`MockAdapter`]);
/// Parse/Unsupported 直抛不掩盖 (真源改口要暴露, 不能用假数据冒充).
pub struct FallbackAdapter {
    primary: Arc<dyn MarketAdapter>,
    fallback: Arc<dyn MarketAdapter>,
}

impl FallbackAdapter {
    pub fn new(primary: Arc<dyn MarketAdapter>, fallback: Arc<dyn MarketAdapter>) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl MarketAdapter for FallbackAdapter {
    fn provider_id(&self) -> String {
        format!("{}+fallback", self.primary.provider_id())
    }
    async fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        match self.primary.fetch_quote(symbol).await {
            Ok(q) => Ok(q),
            Err(e) if e.degradable() => self.fallback.fetch_quote(symbol).await,
            Err(e) => Err(e),
        }
    }
}

/// 适配器注册表 (热插拔: register 即接入, 供上层工具/套件按 provider 取用).
#[derive(Default)]
pub struct AdapterRegistry {
    map: HashMap<String, Arc<dyn MarketAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, adapter: Arc<dyn MarketAdapter>) {
        self.map.insert(adapter.provider_id(), adapter);
    }
    pub fn get(&self, provider: &str) -> Option<&Arc<dyn MarketAdapter>> {
        self.map.get(provider)
    }
    /// 已注册数据源 id (排序, 确定性).
    pub fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.map.keys().cloned().collect();
        ids.sort();
        ids
    }
}

// ============================================================
// 原始 HTTP 口 (可注入 mock, 旗舰适配器的可测缝隙)
// ============================================================

/// 原始 GET 口: (状态码, 响应体); 网络层错误 → Unreachable.
#[async_trait]
pub trait RawFetch: Send + Sync {
    async fn get(&self, url: &str) -> Result<(u16, String), AdapterError>;
}

/// reqwest 真实现 (10s 超时, UA 标识; 状态码语义由适配器解读).
pub struct ReqwestRawFetch {
    client: reqwest::Client,
}

impl ReqwestRawFetch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("apeireth-oracle-adapters/1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for ReqwestRawFetch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RawFetch for ReqwestRawFetch {
    async fn get(&self, url: &str) -> Result<(u16, String), AdapterError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AdapterError::Unreachable(format!("{url}: {e}")))?;
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| AdapterError::Unreachable(format!("读响应体失败: {e}")))?;
        Ok((status, body))
    }
}

// ============================================================
// 旗舰适配器 1/2: CoinGecko 加密货币 (免费无 key)
// ============================================================

/// CoinGecko 加密货币适配器 (simple/price 端点, 免费无 key; 429 → 限流可降级).
pub struct CoinGeckoAdapter {
    raw: Arc<dyn RawFetch>,
    base_url: String,
}

impl CoinGeckoAdapter {
    pub fn new() -> Self {
        Self::with_raw(Arc::new(ReqwestRawFetch::new()))
    }
    /// 注入原始 GET 口 (测试注 mock, 生产默认 reqwest).
    pub fn with_raw(raw: Arc<dyn RawFetch>) -> Self {
        Self {
            raw,
            base_url: "https://api.coingecko.com/api/v3".into(),
        }
    }
    /// symbol → CoinGecko coin id (能力边界内的小表, 未知直抛 Unsupported).
    pub fn coin_id(symbol: &str) -> Result<&'static str, AdapterError> {
        match symbol.to_ascii_uppercase().as_str() {
            "BTC" | "BITCOIN" => Ok("bitcoin"),
            "ETH" | "ETHEREUM" => Ok("ethereum"),
            "SOL" | "SOLANA" => Ok("solana"),
            "DOGE" | "DOGECOIN" => Ok("dogecoin"),
            other => Err(AdapterError::Unsupported(other.to_string())),
        }
    }
}

impl Default for CoinGeckoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 simple/price 响应: `{"bitcoin":{"usd":61234.5}}` → 价格.
pub fn parse_simple_price(body: &str, coin_id: &str) -> Result<f64, AdapterError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AdapterError::Parse(format!("非 JSON: {e}")))?;
    v.pointer(&format!("/{coin_id}/usd"))
        .and_then(|x| x.as_f64())
        .ok_or_else(|| AdapterError::Parse(format!("响应缺 {coin_id}.usd: {body}")))
}

#[async_trait]
impl MarketAdapter for CoinGeckoAdapter {
    fn provider_id(&self) -> String {
        "coingecko".into()
    }
    async fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        let coin = Self::coin_id(symbol)?;
        let url = format!(
            "{}/simple/price?ids={coin}&vs_currencies=usd",
            self.base_url
        );
        let (status, body) = self.raw.get(&url).await?;
        match status {
            200 => {}
            429 => return Err(AdapterError::RateLimited(format!("coingecko 429: {body}"))),
            s => return Err(AdapterError::Unreachable(format!("coingecko HTTP {s}"))),
        }
        let value = parse_simple_price(&body, coin)?;
        Ok(MarketQuote {
            provider: self.provider_id(),
            symbol: symbol.to_ascii_uppercase(),
            value,
            unit: "USD".into(),
            as_of_ms: chrono::Utc::now().timestamp_millis(),
        })
    }
}

// ============================================================
// 旗舰适配器 2/2: 宏观/利率 (美债 fiscaldata, 免费无 key)
// ============================================================

/// 宏观/利率适配器: 美债平均利率 (fiscaldata.treasury.gov, 免费无 key).
/// FRED 同域但需 API key, 故取免 key 替选 (N3 要求「免费公开 API」).
pub struct MacroRatesAdapter {
    raw: Arc<dyn RawFetch>,
    url: String,
}

/// 该适配器唯一 symbol: 美债平均利率.
pub const TREASURY_AVG_RATE: &str = "TREASURY_AVG_RATE";

impl MacroRatesAdapter {
    pub fn new() -> Self {
        Self::with_raw(Arc::new(ReqwestRawFetch::new()))
    }
    pub fn with_raw(raw: Arc<dyn RawFetch>) -> Self {
        Self {
            raw,
            url: "https://api.fiscaldata.treasury.gov/services/api/fiscal_service/v2/accounting/od/avg_interest_rates?sort=-record_date&page[size]=1&format=json".into(),
        }
    }
}

impl Default for MacroRatesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// 解析 fiscaldata 响应: `{"data":[{"attributes":{"avg_interest_rate_amt":"3.51",...}}]}` → 利率(%).
pub fn parse_fiscaldata_rate(body: &str) -> Result<f64, AdapterError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AdapterError::Parse(format!("非 JSON: {e}")))?;
    let amt = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|item| item.pointer("/attributes/avg_interest_rate_amt"))
        .ok_or_else(|| {
            AdapterError::Parse(format!("响应无 data[0].avg_interest_rate_amt: {body}"))
        })?;
    amt.as_f64()
        .or_else(|| amt.as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| AdapterError::Parse(format!("avg_interest_rate_amt 非数值: {amt}")))
}

#[async_trait]
impl MarketAdapter for MacroRatesAdapter {
    fn provider_id(&self) -> String {
        "macro-rates".into()
    }
    async fn fetch_quote(&self, symbol: &str) -> Result<MarketQuote, AdapterError> {
        if symbol.to_ascii_uppercase() != TREASURY_AVG_RATE {
            return Err(AdapterError::Unsupported(symbol.to_string()));
        }
        let (status, body) = self.raw.get(&self.url).await?;
        match status {
            200 => {}
            429 => return Err(AdapterError::RateLimited(format!("fiscaldata 429: {body}"))),
            s => return Err(AdapterError::Unreachable(format!("fiscaldata HTTP {s}"))),
        }
        let value = parse_fiscaldata_rate(&body)?;
        Ok(MarketQuote {
            provider: self.provider_id(),
            symbol: TREASURY_AVG_RATE.into(),
            value,
            unit: "%".into(),
            as_of_ms: chrono::Utc::now().timestamp_millis(),
        })
    }
}

// ============================================================
// 预测管线: 拉基线 → 登记可证伪预测 → 到期对照 resolve (挂既有 ForecastRegistry)
// ============================================================

/// 基线元数据 (记忆库 `adapterfc-` 前缀事件, append-only; oracle.rs 0 改动的接线层).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterForecastMeta {
    pub forecast_id: String,
    pub symbol: String,
    pub provider: String,
    pub unit: String,
    pub baseline_value: f64,
    pub horizon_ms: i64,
    pub registered_at_ms: i64,
}

const ADAPTER_FC_PREFIX: &str = "adapterfc-";

/// 登记回执 (含基线报价快照).
#[derive(Debug, Clone)]
pub struct DirectionForecast {
    pub forecast_id: String,
    pub statement: String,
    pub probability: f64,
    pub deadline_ms: i64,
    pub baseline: MarketQuote,
}

/// 到期对照结果 (actual + Brier 入账 + 对照时报价).
#[derive(Debug, Clone)]
pub struct ResolveOutcome {
    pub forecast_id: String,
    pub actual: bool,
    pub brier: f64,
    pub current: MarketQuote,
}

/// 预测管线: 适配器 + ForecastRegistry 的挂接层 (不重写 oracle, 只喂登记/对照).
pub struct ForecastPipeline {
    adapter: Arc<dyn MarketAdapter>,
    registry: ForecastRegistry,
    store: Arc<SqliteMemoryStore>,
    session_id: String,
}

impl ForecastPipeline {
    pub fn new(
        adapter: Arc<dyn MarketAdapter>,
        store: Arc<SqliteMemoryStore>,
        session_id: impl Into<String>,
    ) -> Self {
        let session_id = session_id.into();
        let registry = ForecastRegistry::new(store.clone(), session_id.clone());
        Self {
            adapter,
            registry,
            store,
            session_id,
        }
    }

    /// 既有预测登记表入口 (Brier 校准走 `registry().calibration()`, 0 重写).
    pub fn registry(&self) -> &ForecastRegistry {
        &self.registry
    }

    /// 登记方向预测: 拉当前价作基线 → 「horizon 后高于基线」可证伪陈述 → 入 registry + 存基线元数据.
    pub async fn register_direction_forecast(
        &self,
        symbol: &str,
        horizon_ms: i64,
        probability: f64,
    ) -> Result<DirectionForecast, String> {
        let baseline = self
            .adapter
            .fetch_quote(symbol)
            .await
            .map_err(|e| format!("拉取基线失败: {e}"))?;
        let now = chrono::Utc::now().timestamp_millis();
        let deadline_ms = now + horizon_ms.max(0);
        let statement = format!(
            "{}后 {} 高于基线 {} {} (数据源: {})",
            humanize(horizon_ms),
            baseline.symbol,
            baseline.value,
            baseline.unit,
            baseline.provider
        );
        let forecast = Forecast::new(statement.clone(), probability, deadline_ms);
        self.registry.register(&forecast)?;
        let meta = AdapterForecastMeta {
            forecast_id: forecast.id.clone(),
            symbol: baseline.symbol.clone(),
            provider: baseline.provider.clone(),
            unit: baseline.unit.clone(),
            baseline_value: baseline.value,
            horizon_ms,
            registered_at_ms: now,
        };
        let ep = CoreEpisode {
            id: format!("{ADAPTER_FC_PREFIX}{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().timestamp(),
            role: "system".into(),
            content: serde_json::to_string(&meta).map_err(|e| e.to_string())?,
            session_id: self.session_id.clone(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())?;
        Ok(DirectionForecast {
            forecast_id: forecast.id,
            statement,
            probability: forecast.probability,
            deadline_ms,
            baseline,
        })
    }

    /// 到期对照: 重拉现价 vs 基线 (严格高于判成真, 平盘判未成真) → registry.resolve 入账 Brier.
    pub async fn resolve_due(&self, forecast_id: &str) -> Result<ResolveOutcome, String> {
        let meta = self.load_meta(forecast_id)?;
        let deadline = meta.registered_at_ms + meta.horizon_ms;
        let now = chrono::Utc::now().timestamp_millis();
        if now < deadline {
            return Err(format!("未到期 (deadline={deadline}, now={now})"));
        }
        let current = self
            .adapter
            .fetch_quote(&meta.symbol)
            .await
            .map_err(|e| format!("对照拉取失败: {e}"))?;
        let actual = current.value > meta.baseline_value;
        let brier = self.registry.resolve(forecast_id, actual)?;
        Ok(ResolveOutcome {
            forecast_id: forecast_id.to_string(),
            actual,
            brier,
            current,
        })
    }

    /// 从记忆库找回基线元数据 (append-only 扫描 `adapterfc-` 前缀, 同 registry 重放风格).
    fn load_meta(&self, forecast_id: &str) -> Result<AdapterForecastMeta, String> {
        let eps = self
            .store
            .recent_episodes(&self.session_id, 500)
            .map_err(|e| e.to_string())?;
        eps.iter()
            .filter(|e| e.id.starts_with(ADAPTER_FC_PREFIX))
            .filter_map(|e| serde_json::from_str::<AdapterForecastMeta>(&e.content).ok())
            .find(|m| m.forecast_id == forecast_id)
            .ok_or_else(|| format!("预测元数据不存在: {forecast_id}"))
    }
}

/// horizon 人类可读化 (陈述句用).
fn humanize(ms: i64) -> String {
    if ms <= 0 {
        "即时".into()
    } else if ms < 3_600_000 {
        format!("{}分钟", ms / 60_000)
    } else if ms < 86_400_000 {
        format!("{}小时", ms / 3_600_000)
    } else {
        format!("{}天", ms / 86_400_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    /// 测试缝隙: 固定 (状态码, 响应体) 的原始 GET.
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

    // ---------- mock 适配器 ----------

    #[tokio::test]
    async fn mock_adapter_quote_and_unsupported() {
        let m = MockAdapter::new("mock");
        m.set_quote("BTC", 100.0);
        let q = m.fetch_quote("BTC").await.unwrap();
        assert_eq!(q.value, 100.0);
        assert_eq!(q.provider, "mock");
        // 未配置 symbol → Unsupported
        assert_eq!(
            m.fetch_quote("XYZ").await.unwrap_err(),
            AdapterError::Unsupported("XYZ".into())
        );
    }

    #[tokio::test]
    async fn mock_adapter_failure_modes() {
        let m = MockAdapter::new("mock");
        m.set_quote("BTC", 100.0);
        m.fail_with(AdapterError::RateLimited("模拟 429".into()));
        assert!(matches!(
            m.fetch_quote("BTC").await,
            Err(AdapterError::RateLimited(_))
        ));
        m.clear_failure();
        assert!(m.fetch_quote("BTC").await.is_ok());
    }

    // ---------- 降级包装 ----------

    #[tokio::test]
    async fn fallback_degrades_on_rate_limit_but_not_on_unsupported() {
        let primary = Arc::new(MockAdapter::new("primary"));
        primary.set_quote("BTC", 100.0);
        let fallback = Arc::new(MockAdapter::new("fallback"));
        fallback.set_quote("BTC", 42.0);
        let fa = FallbackAdapter::new(primary.clone(), fallback.clone());
        // 正常 → 主源
        assert_eq!(fa.fetch_quote("BTC").await.unwrap().value, 100.0);
        // 主源限流 → 降级 fallback
        primary.fail_with(AdapterError::RateLimited("429".into()));
        let q = fa.fetch_quote("BTC").await.unwrap();
        assert_eq!(q.value, 42.0);
        assert_eq!(q.provider, "fallback");
        // 主源 Unsupported → 直抛不降级 (不用假数据掩盖能力边界)
        assert_eq!(
            fa.fetch_quote("XYZ").await.unwrap_err(),
            AdapterError::Unsupported("XYZ".into())
        );
    }

    // ---------- CoinGecko 适配器 ----------

    #[test]
    fn coingecko_symbol_mapping() {
        assert_eq!(CoinGeckoAdapter::coin_id("btc").unwrap(), "bitcoin");
        assert_eq!(CoinGeckoAdapter::coin_id("ETH").unwrap(), "ethereum");
        assert!(CoinGeckoAdapter::coin_id("XYZ").is_err());
    }

    #[test]
    fn coingecko_parse_simple_price() {
        let body = r#"{"bitcoin":{"usd":61234.5}}"#;
        assert!((parse_simple_price(body, "bitcoin").unwrap() - 61234.5).abs() < 1e-9);
        // 缺字段 / 非 JSON → Parse (不编数)
        assert!(matches!(
            parse_simple_price(body, "ethereum"),
            Err(AdapterError::Parse(_))
        ));
        assert!(matches!(
            parse_simple_price("not json", "bitcoin"),
            Err(AdapterError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn coingecko_status_mapping_and_quote() {
        // 200 → 规范化报价
        let ok = Arc::new(MockRawFetch {
            status: 200,
            body: r#"{"bitcoin":{"usd":61234.5}}"#.into(),
        });
        let a = CoinGeckoAdapter::with_raw(ok);
        let q = a.fetch_quote("BTC").await.unwrap();
        assert_eq!(q.provider, "coingecko");
        assert_eq!(q.symbol, "BTC");
        assert_eq!(q.unit, "USD");
        assert!((q.value - 61234.5).abs() < 1e-9);
        // 429 → 限流 (可降级)
        let rl = CoinGeckoAdapter::with_raw(Arc::new(MockRawFetch {
            status: 429,
            body: "rate limited".into(),
        }));
        assert!(rl.fetch_quote("BTC").await.unwrap_err().degradable());
        // 500 → 不可达 (可降级)
        let err500 = CoinGeckoAdapter::with_raw(Arc::new(MockRawFetch {
            status: 500,
            body: "oops".into(),
        }));
        assert!(err500.fetch_quote("BTC").await.unwrap_err().degradable());
    }

    // ---------- 宏观/利率适配器 ----------

    #[test]
    fn fiscaldata_parse_rate() {
        let body = r#"{"data":[{"attributes":{"avg_interest_rate_amt":"3.51","record_date":"2026-07-31T00:00:00Z"}}]}"#;
        assert!((parse_fiscaldata_rate(body).unwrap() - 3.51).abs() < 1e-9);
        // 数值型也收; 空 data / 非 JSON → Parse
        let body_num = r#"{"data":[{"attributes":{"avg_interest_rate_amt":3.25}}]}"#;
        assert!((parse_fiscaldata_rate(body_num).unwrap() - 3.25).abs() < 1e-9);
        assert!(matches!(
            parse_fiscaldata_rate(r#"{"data":[]}"#),
            Err(AdapterError::Parse(_))
        ));
        assert!(matches!(
            parse_fiscaldata_rate("x"),
            Err(AdapterError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn macro_rates_fetch_and_boundary() {
        let body = r#"{"data":[{"attributes":{"avg_interest_rate_amt":"3.51"}}]}"#;
        let a = MacroRatesAdapter::with_raw(Arc::new(MockRawFetch {
            status: 200,
            body: body.into(),
        }));
        let q = a.fetch_quote(TREASURY_AVG_RATE).await.unwrap();
        assert_eq!(q.unit, "%");
        assert!((q.value - 3.51).abs() < 1e-9);
        // 未知 symbol → Unsupported; 429 → 限流
        assert_eq!(
            a.fetch_quote("CPI").await.unwrap_err(),
            AdapterError::Unsupported("CPI".into())
        );
        let rl = MacroRatesAdapter::with_raw(Arc::new(MockRawFetch {
            status: 429,
            body: "slow down".into(),
        }));
        assert!(rl
            .fetch_quote(TREASURY_AVG_RATE)
            .await
            .unwrap_err()
            .degradable());
    }

    // ---------- 适配器注册表 (热插拔) ----------

    #[tokio::test]
    async fn adapter_registry_hotplug() {
        let mut reg = AdapterRegistry::new();
        reg.register(Arc::new(MockAdapter::new("mock")));
        reg.register(Arc::new(CoinGeckoAdapter::with_raw(Arc::new(
            MockRawFetch {
                status: 429,
                body: String::new(),
            },
        ))));
        assert_eq!(
            reg.list(),
            vec!["coingecko".to_string(), "mock".to_string()]
        );
        assert!(reg.get("mock").is_some());
        assert!(reg.get("不存在").is_none());
    }

    // ---------- 预测管线: 拉取/规范化/到期 resolve 全路径 ----------

    #[tokio::test]
    async fn pipeline_register_resolve_full_path() {
        let mock = Arc::new(MockAdapter::new("mock"));
        mock.set_quote("BTC", 100.0);
        let p = ForecastPipeline::new(mock.clone(), mem(), "sess-1");
        // 登记: horizon=0 即到期, 概率 0.7
        let df = p.register_direction_forecast("BTC", 0, 0.7).await.unwrap();
        assert!(
            df.statement.contains("BTC"),
            "陈述应含 symbol: {}",
            df.statement
        );
        assert!((df.probability - 0.7).abs() < 1e-9);
        assert_eq!(df.baseline.value, 100.0);
        // 行情上涨 → 到期对照成真, Brier = (0.7-1)² = 0.09
        mock.set_quote("BTC", 110.0);
        let out = p.resolve_due(&df.forecast_id).await.unwrap();
        assert!(out.actual);
        assert!(
            (out.brier - 0.09).abs() < 1e-9,
            "Brier 应 = 0.09: {}",
            out.brier
        );
        // 校准挂接: 既有 registry.calibration() 可见 1 条已对照
        let (n, mean_brier, _hint) = p.registry().calibration().unwrap();
        assert_eq!(n, 1);
        assert!((mean_brier - 0.09).abs() < 1e-9);
    }

    #[tokio::test]
    async fn pipeline_resolve_false_when_price_flat_or_down() {
        let mock = Arc::new(MockAdapter::new("mock"));
        mock.set_quote("BTC", 100.0);
        let p = ForecastPipeline::new(mock.clone(), mem(), "sess-2");
        let df = p.register_direction_forecast("BTC", 0, 0.9).await.unwrap();
        // 平盘 → 未成真 (严格高于判成真, 保守口径), Brier = 0.9² = 0.81
        let out = p.resolve_due(&df.forecast_id).await.unwrap();
        assert!(!out.actual);
        assert!((out.brier - 0.81).abs() < 1e-9);
    }

    #[tokio::test]
    async fn pipeline_not_due_error() {
        let mock = Arc::new(MockAdapter::new("mock"));
        mock.set_quote("BTC", 100.0);
        let p = ForecastPipeline::new(mock, mem(), "sess-3");
        let df = p
            .register_direction_forecast("BTC", 60_000, 0.5)
            .await
            .unwrap();
        let err = p.resolve_due(&df.forecast_id).await.unwrap_err();
        assert!(err.contains("未到期"), "应报未到期: {err}");
    }

    #[tokio::test]
    async fn pipeline_double_resolve_error() {
        let mock = Arc::new(MockAdapter::new("mock"));
        mock.set_quote("BTC", 100.0);
        let p = ForecastPipeline::new(mock.clone(), mem(), "sess-4");
        let df = p.register_direction_forecast("BTC", 0, 0.5).await.unwrap();
        p.resolve_due(&df.forecast_id).await.unwrap();
        mock.set_quote("BTC", 200.0);
        let err = p.resolve_due(&df.forecast_id).await.unwrap_err();
        assert!(err.contains("已 resolve"), "重复 resolve 应报错: {err}");
    }

    #[tokio::test]
    async fn pipeline_unknown_symbol_register_error() {
        let mock = Arc::new(MockAdapter::new("mock"));
        let p = ForecastPipeline::new(mock, mem(), "sess-5");
        let err = p
            .register_direction_forecast("XYZ", 0, 0.5)
            .await
            .unwrap_err();
        assert!(err.contains("拉取基线失败"), "{err}");
    }

    #[tokio::test]
    async fn pipeline_degraded_full_path_via_fallback() {
        // 主源限流 → 降级 mock → 登记/到期 resolve 全路径不阻塞 (验收「限流不阻塞」)
        let primary = Arc::new(MockAdapter::new("primary"));
        primary.set_quote("BTC", 100.0);
        primary.fail_with(AdapterError::RateLimited("429".into()));
        let fallback = Arc::new(MockAdapter::new("fallback"));
        fallback.set_quote("BTC", 100.0);
        let fa = Arc::new(FallbackAdapter::new(primary, fallback.clone()));
        let p = ForecastPipeline::new(fa, mem(), "sess-6");
        let df = p.register_direction_forecast("BTC", 0, 0.6).await.unwrap();
        assert_eq!(df.baseline.provider, "fallback");
        fallback.set_quote("BTC", 150.0);
        let out = p.resolve_due(&df.forecast_id).await.unwrap();
        assert!(out.actual);
        assert!((out.brier - 0.16).abs() < 1e-9); // (0.6-1)² = 0.16
    }

    #[tokio::test]
    async fn pipeline_meta_reload_across_instances() {
        // 基线元数据走记忆库 → 换实例 (同库) 仍可对照 (append-only 重放风格)
        let store = mem();
        let mock = Arc::new(MockAdapter::new("mock"));
        mock.set_quote("BTC", 100.0);
        let p1 = ForecastPipeline::new(mock.clone(), store.clone(), "sess-7");
        let df = p1.register_direction_forecast("BTC", 0, 0.5).await.unwrap();
        mock.set_quote("BTC", 120.0);
        let p2 = ForecastPipeline::new(mock, store, "sess-7");
        let out = p2.resolve_due(&df.forecast_id).await.unwrap();
        assert!(out.actual);
    }
}

// ============================================================
// TP25: 时序预测器 trait 口 (TimesFM/Kronos 本地小模型可选)
// ============================================================

/// 数字信号时序预测 trait (TP25, E3 增强).
/// 实现方: TimesFM/Kronos 等本地小模型适配器 — **0 装 PASS: 模型未接, trait 口已备**.
pub trait TimeSeriesPredictor: Send + Sync {
    /// 预测: 输入历史序列 (时间序), 输出 horizon 步预测.
    fn predict(&self, series: &[f64], horizon: usize) -> Result<Vec<f64>, AdapterError>;
    /// 模型标识 (审计/降级用).
    fn provider(&self) -> &str;
}

/// 默认实现: 未接模型 → 诚实 Err (0 装 PASS: 不假装能预测).
#[derive(Debug, Default)]
pub struct NoopTimeSeriesPredictor;

impl TimeSeriesPredictor for NoopTimeSeriesPredictor {
    fn predict(&self, _series: &[f64], _horizon: usize) -> Result<Vec<f64>, AdapterError> {
        Err(AdapterError::Degraded(
            "NoopTimeSeriesPredictor: 时序模型未接入 (TP25 trait 口已备, 接 TimesFM/Kronos 时替换)"
                .into(),
        ))
    }
    fn provider(&self) -> &str {
        "noop"
    }
}

/// 数字预测 + LLM 文本预测融合 (集合预报, E3 增强).
/// 置信度加权平均: (digital*dc + textual*tc) / (dc+tc).
/// 双方置信度都为 0 → 退化为 0.5 (无信息先验, 0 装: 不假装有信息).
pub fn blend_predictions(digital: f64, textual: f64, digital_conf: f64, textual_conf: f64) -> f64 {
    let dc = digital_conf.max(0.0);
    let tc = textual_conf.max(0.0);
    if dc + tc <= 0.0 {
        return 0.5;
    }
    let blended = (digital * dc + textual * tc) / (dc + tc);
    blended.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tp25_tests {
    use super::*;

    #[test]
    fn noop_predictor_is_honest() {
        let p = NoopTimeSeriesPredictor;
        let err = p.predict(&[1.0, 2.0, 3.0], 5).unwrap_err();
        assert!(matches!(err, AdapterError::Degraded(_)), "{err:?}");
        assert_eq!(p.provider(), "noop");
    }

    #[test]
    fn blend_confidence_weighted() {
        // 数字高置信 0.7 + 文本低置信 0.5 → 偏向数字
        let b = blend_predictions(0.7, 0.5, 0.9, 0.1);
        assert!((b - 0.68).abs() < 1e-9, "b={b} (期望 0.68)");
        // 双方零置信 → 0.5 无信息先验
        assert_eq!(blend_predictions(0.9, 0.1, 0.0, 0.0), 0.5);
        // 等置信 → 平均
        let eq = blend_predictions(0.8, 0.6, 1.0, 1.0);
        assert!((eq - 0.7).abs() < 1e-9);
    }

    #[test]
    fn mock_predictor_injectable() {
        struct ConstPredictor(f64);
        impl TimeSeriesPredictor for ConstPredictor {
            fn predict(&self, _s: &[f64], horizon: usize) -> Result<Vec<f64>, AdapterError> {
                Ok(vec![self.0; horizon])
            }
            fn provider(&self) -> &str {
                "const-mock"
            }
        }
        let p = ConstPredictor(0.65);
        let out = p.predict(&[1.0], 3).unwrap();
        assert_eq!(out, vec![0.65; 3]);
        assert_eq!(p.provider(), "const-mock");
    }
}
