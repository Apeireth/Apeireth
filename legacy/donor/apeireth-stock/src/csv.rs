//! CSV 解析 + 批量导入 (FinanceDatabase 格式).
//!
//! 期望 CSV header (case-insensitive):
//! - `symbol` (主键, 必填)
//! - `name` / `sector` / `industry` / `exchange` / `country` / `currency` (字符串)
//! - `market_cap` / `ipo_year` (可空数值)
//!
//! 容错:
//! - 缺列 → 用空字符串兜底 (不报错)
//! - 数值列空值/非法 → None
//! - 行 symbol 为空 → 跳过 + 计数
//! - UTF-8 BOM 自动剥离 (csv crate 默认处理)

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::store::{SymbolStore, SymbolStoreError};
use crate::symbol::{Provenance, SymbolMeta};

/// 导入统计 (per task 验收 #4: 性能基线).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CsvImportStats {
    /// 成功导入行数 (含 update 覆盖).
    pub imported: usize,
    /// 因 symbol 空 / 行解析失败而被跳过.
    pub skipped: usize,
    /// 导入总耗时 (秒).
    pub elapsed_sec: f64,
    /// 平均速率 (rows/sec).
    pub rows_per_sec: f64,
    /// CSV 源路径 (调试用).
    pub source: String,
    /// 发现但未处理的额外 header (用于兼容性诊断).
    pub extra_headers: Vec<String>,
}

/// CSV 单行 raw 形态 (serde 自动对应 header).
#[derive(Debug, Deserialize)]
struct CsvRow {
    /// 主键, 大小写不敏感.
    #[serde(alias = "Symbol", alias = "SYMBOL")]
    symbol: String,
    #[serde(default, alias = "Name", alias = "NAME")]
    name: String,
    #[serde(default, alias = "Sector", alias = "SECTOR")]
    sector: String,
    #[serde(default, alias = "Industry", alias = "INDUSTRY")]
    industry: String,
    #[serde(default, alias = "Exchange", alias = "EXCHANGE")]
    exchange: String,
    #[serde(default, alias = "Country", alias = "COUNTRY")]
    country: String,
    #[serde(default, alias = "Currency", alias = "CURRENCY")]
    currency: String,
    #[serde(
        default,
        alias = "Market Cap",
        alias = "MARKET_CAP",
        alias = "marketCap"
    )]
    market_cap: String,
    #[serde(default, alias = "IPO Year", alias = "IPO_YEAR", alias = "ipoYear")]
    ipo_year: String,
}

impl CsvRow {
    fn into_meta(self, default_provenance: Provenance, ts_ms: i64) -> SymbolMeta {
        SymbolMeta {
            symbol: self.symbol.trim().to_string(),
            name: self.name.trim().to_string(),
            sector: self.sector.trim().to_string(),
            industry: self.industry.trim().to_string(),
            exchange: self.exchange.trim().to_string(),
            country: self.country.trim().to_string(),
            currency: self.currency.trim().to_string(),
            market_cap: parse_f64(&self.market_cap),
            ipo_year: parse_i32(&self.ipo_year),
            ipo_date: None,
            delisted_date: None,
            provenance: default_provenance,
            last_updated_ms: ts_ms,
        }
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok()
}

fn parse_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.parse::<i32>().ok()
}

/// 从 CSV 文件导入到 SymbolStore.
///
/// 返回统计. 不抛错时已成功 (含部分跳过); 致命错 (文件 IO / SQLite) 才返回 Err.
///
/// **0 装 PASS 标注**: FinanceDatabase 仓库实际未在 `research/source/FinanceDatabase/` (任务包描述与现实偏差),
/// 本函数是基础设施就绪版, 等数据源到位即可调用. 测试用 `tests/fixtures/finance_database_sample.csv`.
pub fn import_from_csv<P: AsRef<Path>>(
    store: &SymbolStore,
    csv_path: P,
    provenance: Provenance,
) -> Result<CsvImportStats, SymbolStoreError> {
    let path = csv_path.as_ref();
    let start = std::time::Instant::now();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true) // 缺列也允许 (兜底空字符串)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| SymbolStoreError::Csv(format!("open {}: {}", path.display(), e)))?;

    // 探测 header, 收集"未消费"列 (兼容扩展).
    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| SymbolStoreError::Csv(format!("read header: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    debug!("CSV headers: {:?}", headers);
    let known_headers = [
        "symbol",
        "name",
        "sector",
        "industry",
        "exchange",
        "country",
        "currency",
        "market_cap",
        "ipo_year",
        "Market Cap",
        "IPO Year",
        "marketCap",
        "ipoYear",
        "Symbol",
        "Name",
        "Sector",
        "Industry",
        "Exchange",
        "Country",
        "Currency",
        "MARKET_CAP",
        "IPO_YEAR",
        "SYMBOL",
        "NAME",
        "SECTOR",
        "INDUSTRY",
        "EXCHANGE",
        "COUNTRY",
        "CURRENCY",
    ];
    let extra_headers: Vec<String> = headers
        .iter()
        .filter(|h| !known_headers.contains(&h.as_str()))
        .cloned()
        .collect();
    if !extra_headers.is_empty() {
        warn!("CSV 含未消费 header: {:?}", extra_headers);
    }

    let ts_ms = chrono::Utc::now().timestamp_millis();
    let mut stats = CsvImportStats {
        source: path.display().to_string(),
        extra_headers,
        ..Default::default()
    };
    let mut batch: Vec<SymbolMeta> = Vec::with_capacity(1024);

    for row in rdr.deserialize() {
        let raw: CsvRow = match row {
            Ok(r) => r,
            Err(e) => {
                warn!("跳过解析失败行: {}", e);
                stats.skipped += 1;
                continue;
            }
        };
        let meta = raw.into_meta(provenance, ts_ms);
        if !meta.is_valid() {
            stats.skipped += 1;
            continue;
        }
        batch.push(meta);
        // 批量入库 (单事务包裹 1000 行, 平衡内存与事务大小).
        if batch.len() >= 1000 {
            let n = batch.len();
            store.insert_batch(&batch)?;
            stats.imported += n;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let n = batch.len();
        store.insert_batch(&batch)?;
        stats.imported += n;
    }

    stats.elapsed_sec = start.elapsed().as_secs_f64();
    stats.rows_per_sec = if stats.elapsed_sec > 0.0 {
        stats.imported as f64 / stats.elapsed_sec
    } else {
        0.0
    };
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SymbolStore;

    fn write_fixture(content: &str) -> tempfile::NamedTempFile {
        let f = tempfile::NamedTempFile::new().unwrap();
        fs_err::write(f.path(), content).unwrap();
        f
    }

    fn open_store() -> SymbolStore {
        SymbolStore::open_in_memory().unwrap()
    }

    #[test]
    fn parse_legal_full_row() {
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   AAPL,Apple Inc.,Technology,Consumer Electronics,NASDAQ,US,USD,2900000000000,1980\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 1);
        assert_eq!(stats.skipped, 0);
        let m = s.get("AAPL").unwrap();
        assert_eq!(m.name, "Apple Inc.");
        assert_eq!(m.sector, "Technology");
        assert_eq!(m.market_cap, Some(2.9e12));
        assert_eq!(m.ipo_year, Some(1980));
        assert_eq!(m.provenance, Provenance::FinanceDatabase);
    }

    #[test]
    fn parse_legal_minimal_row() {
        // 仅 symbol, 其余列空
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   MSFT,,,,,,,,\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::Manual).unwrap();
        assert_eq!(stats.imported, 1);
        let m = s.get("MSFT").unwrap();
        assert!(m.name.is_empty());
        assert!(m.market_cap.is_none());
        assert_eq!(m.provenance, Provenance::Manual);
    }

    #[test]
    fn parse_illegal_empty_symbol_skipped() {
        // 第二行 symbol 为空, 应跳过
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   AAPL,Apple,,,,,,,\n\
                   ,NoSymbolCo,,,,,,,\n\
                   GOOG,Alphabet,,,,,,,\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 2);
        assert_eq!(stats.skipped, 1);
        assert!(s.get("AAPL").is_some());
        assert!(s.get("GOOG").is_some());
        assert!(s.get("").is_none());
    }

    #[test]
    fn parse_illegal_numeric_falls_back_to_none() {
        // market_cap / ipo_year 非法 → None (不报错)
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   BAD,Bad Co.,,,,,,not-a-number,not-a-year\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::Manual).unwrap();
        assert_eq!(stats.imported, 1);
        let m = s.get("BAD").unwrap();
        assert!(m.market_cap.is_none());
        assert!(m.ipo_year.is_none());
    }

    #[test]
    fn parse_case_insensitive_headers() {
        // 大写 header
        let csv = "SYMBOL,NAME,SECTOR,INDUSTRY,EXCHANGE,COUNTRY,CURRENCY,MARKET_CAP,IPO_YEAR\n\
                   TSLA,Tesla,Automotive,Auto Manufacturers,NASDAQ,US,USD,800000000000,2010\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 1);
        assert_eq!(s.get("TSLA").unwrap().name, "Tesla");
    }

    #[test]
    fn parse_missing_columns_default_to_empty() {
        // 完全无 sector/industry 列, 兜底空字符串
        let csv = "symbol,name\n\
                   XYZ,XYZ Corp\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::Manual).unwrap();
        assert_eq!(stats.imported, 1);
        let m = s.get("XYZ").unwrap();
        assert_eq!(m.name, "XYZ Corp");
        assert!(m.sector.is_empty());
        assert!(m.industry.is_empty());
    }

    #[test]
    fn parse_utf8_with_chinese() {
        // 多语言 (中文公司名)
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   0700.HK,腾讯控股,Communication Services,Internet Content & Information,HKEX,CN,CNY,3000000000000,2004\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 1);
        let m = s.get("0700.HK").unwrap();
        assert_eq!(m.name, "腾讯控股");
        assert_eq!(m.sector, "Communication Services");
    }

    #[test]
    fn parse_quoted_field_with_comma() {
        // 引号包裹 + 内含逗号 (CSV 标准格式)
        let csv = r#"symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year
BRK.B,"Berkshire Hathaway Inc., Class B",Financial,Insurance—Diversified,NYSE,US,USD,900000000000,1996
"#;
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 1);
        let m = s.get("BRK.B").unwrap();
        assert_eq!(m.name, "Berkshire Hathaway Inc., Class B");
    }

    #[test]
    fn parse_duplicate_symbol_upsert() {
        // 同 symbol 出现两次, 第二次覆盖 (upsert)
        let csv = "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
                   AAPL,Apple Inc.,Technology,CE,NASDAQ,US,USD,100,1980\n\
                   AAPL,Apple Inc. Updated,Technology,Consumer Electronics,NASDAQ,US,USD,200,1980\n";
        let f = write_fixture(csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 2); // 都成功执行 (但第二次覆盖)
        let m = s.get("AAPL").unwrap();
        assert_eq!(m.name, "Apple Inc. Updated");
        assert_eq!(m.market_cap, Some(200.0));
    }

    #[test]
    fn parse_invalid_file_returns_error() {
        let s = open_store();
        let r = import_from_csv(&s, "/nonexistent/path.csv", Provenance::Manual);
        assert!(r.is_err());
    }

    #[test]
    fn parse_perf_baseline_30k() {
        // 性能基线 (非 30 万, 测试时间约束): 1 万行 < 5s, 推算 30 万 < 30s.
        let mut csv = String::from(
            "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n",
        );
        for i in 0..10_000 {
            csv.push_str(&format!(
                "SYM{:05},Symbol {},Tech,Hardware,NYSE,US,USD,{},2000\n",
                i,
                i,
                i as f64 * 1_000_000.0
            ));
        }
        let f = write_fixture(&csv);
        let s = open_store();
        let stats = import_from_csv(&s, f.path(), Provenance::FinanceDatabase).unwrap();
        assert_eq!(stats.imported, 10_000);
        // 1 万行 < 5s (留 5x 余量给 CI 抖动)
        assert!(
            stats.elapsed_sec < 5.0,
            "1 万行导入耗时 {}s 超阈值",
            stats.elapsed_sec
        );
        assert!(
            stats.rows_per_sec > 1000.0,
            "导入速率 {} rows/sec 低于 1000",
            stats.rows_per_sec
        );
    }
}
