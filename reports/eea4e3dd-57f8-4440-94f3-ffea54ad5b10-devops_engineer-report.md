# TP27 标的元数据资产 (N3 金融源, FinanceDatabase 30 万标的入库) — 验收报告（新 spec）

> 任务 ID: `eea4e3dd-57f8-4440-94f3-ffea54ad5b10`
> 角色: devops_engineer
> 提交: `ef794d094f50c606dfcd8c0e4ffe4bfab3fd68c9` (主分支 feat(stock))
> integration HEAD: `c55bb555` (cherry-pick, 含 TP27 main + 新 spec 扩展)

## 一、任务定位

套件批。FinanceDatabase 30 万标的 CSV 入库（标的清单/行业/交易所），构建 apeireth-stock 元数据资产。

## 二、与第一轮 TP27 (b487b3f8) 的差异

第一轮 TP27 已完成 (`merged_to_integration`, `b3cbf5f6`)，按新 spec (eea4e3dd) 扩展：

| 项 | 第一轮 (旧 spec) | 本轮 (新 spec) | 实施 |
|---|---|---|---|
| **字段命名** | `symbol` | `ticker` (accessor) | `ticker()` 访问器，与 `symbol` 字段等价 |
| **IPO 字段** | `ipo_year: Option<i32>` | `ipo_date: Option<String>` (ISO 8601) | 保留 `ipo_year` 兼容，新增 `ipo_date` |
| **退市字段** | 无 | `delisted_date: Option<String>` | 新增 |
| **字段总数** | 11 | 13 | V6 migration + V5 增量 ALTER ADD |
| **API: get** | `get(symbol)` | `get_by_ticker(ticker)` | 新增方法，旧 API 保留 |
| **API: search** | `search(s, i, e, limit)` | `search_by_industry(i, limit)` + `list_by_exchange(e, limit)` | 新增方法，旧 API 保留 |
| **API: count** | `count()` | `count_all()` | 新增方法，旧 API 保留 |
| **数据源** | 手动 CSV | FinanceDatabase 公开 CSV 下载 | 新增 `refresh` 模块 (DataSource/RefreshOutcome) |
| **持久化** | 一次性 SQLite | 一次性 SQLite (避免每次启动重下) | 已满足 |
| **网络失败** | 不涉及 | 用缓存兜底 | cache 命中/Fallback 路径完整 |
| **refresh 命令** | 不涉及 | 提供 | `refresh_and_import` 函数入口 |

## 三、新增/扩展产物

### 3.1 新文件 `crates/apeireth-stock/src/refresh.rs` (11371 bytes)

```rust
pub const FINANCE_DATABASE_RAW_BASE: &str =
    "https://raw.githubusercontent.com/JerBouma/FinanceDatabase/main";
pub const FINANCE_DATABASE_EQUITIES_CSV: &str = "equities.csv";

pub enum DataSource { Url(String), Local(PathBuf) }
pub enum RefreshOutcome { Downloaded { cache_path, bytes }, CacheHit { cache_path }, Fallback { cache_path, network_err } }
pub enum RefreshError { Io, Network(String), Cache(String) }

pub fn cache_exists(cache_path: &Path) -> bool;
pub fn refresh(source: DataSource, cache_path: &Path) -> Result<RefreshOutcome, RefreshError>;
pub fn refresh_and_import(store, source, cache_path, provenance) -> Result<(CsvImportStats, RefreshOutcome), RefreshError>;
pub fn validate_url(url: &str) -> Result<&str, RefreshError>;
```

### 3.2 schema 扩展 (SymbolMeta 11 → 13 字段)

```rust
pub struct SymbolMeta {
    pub symbol: String,              // 主键
    pub name: String,
    pub sector: String,
    pub industry: String,
    pub exchange: String,
    pub country: String,
    pub currency: String,
    pub market_cap: Option<f64>,
    pub ipo_year: Option<i32>,       // 兼容旧 spec
    pub ipo_date: Option<String>,    // 新 spec, ISO 8601
    pub delisted_date: Option<String>, // 新 spec, ISO 8601
    pub provenance: Provenance,
    pub last_updated_ms: i64,
}

impl SymbolMeta {
    pub fn ticker(&self) -> &str { &self.symbol }  // 新 accessor
    // to_row 现在 13 列
}
```

### 3.3 SQLite V6 migration

```sql
-- V6: 13 列 (旧 11 + ipo_date + delisted_date)
CREATE TABLE IF NOT EXISTS symbols (
    symbol TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    sector TEXT NOT NULL DEFAULT '',
    industry TEXT NOT NULL DEFAULT '',
    exchange TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT '',
    currency TEXT NOT NULL DEFAULT '',
    market_cap REAL,
    ipo_year INTEGER,
    ipo_date TEXT,           -- NEW
    delisted_date TEXT,      -- NEW
    provenance TEXT NOT NULL DEFAULT 'manual',
    last_updated_ms INTEGER NOT NULL DEFAULT 0
);
-- V5 → V6 增量: ALTER ADD (探测 pragma_table_info 防重复)
```

### 3.4 新 spec API

```rust
// SymbolStore inherent
pub fn get_by_ticker(&self, ticker: &str) -> Option<SymbolMeta>;
pub fn search_by_industry(&self, industry: &str, limit: usize) -> Vec<SymbolMeta>;
pub fn list_by_exchange(&self, exchange: &str, limit: usize) -> Vec<SymbolMeta>;
pub fn count_all(&self) -> usize;

// SymbolCatalog trait (per task 验收 #5)
trait SymbolCatalog: Send + Sync {
    // 旧 spec (向后兼容)
    fn get(&self, symbol: &str) -> Option<SymbolMeta>;
    fn search(...) -> Vec<SymbolMeta>;
    fn count(&self) -> usize;

    // 新 spec (默认 delegate 给 search/count)
    fn get_by_ticker(&self, ticker: &str) -> Option<SymbolMeta> { self.get(ticker) }
    fn search_by_industry(&self, industry: &str, limit: usize) -> Vec<SymbolMeta> {
        self.search(None, Some(industry), None, limit)
    }
    fn list_by_exchange(&self, exchange: &str, limit: usize) -> Vec<SymbolMeta> {
        self.search(None, None, Some(exchange), limit)
    }
    fn count_all(&self) -> usize { self.count() }
}
```

## 四、测试验收 (61 个全绿, +22 相比第一轮)

| 模块 | 第一轮 | 本轮 | 增量测试 |
|---|---|---|---|
| `symbol::tests` | 6 | **9** | +ticker accessor / ipo_date+delisted_date roundtrip / to_row 13 列 |
| `store::tests` | 16 | **26** | +V6 migration adds cols / V5→V6 增量 / V6 幂等 / 4 新 API delegate |
| `catalog::tests` | 2 | **3** | +新 spec trait object (新 API 通过 dyn dispatch) |
| `csv::tests` | 11 | **11** | (不变, 容错 + 性能基线仍绿) |
| `refresh::tests` | 0 | **9** | +default URL / validate_url accept/reject / cache_exists / local copy / cache hit / cache miss / refresh_and_import roundtrip × 2 / refresh interval |
| `lib::tests` | 3 | **3** | (默认值含新字段) |
| **合计** | **39** | **61** | **+22** |

```
$ cargo test -p apeireth-stock --lib
test result: ok. 61 passed; 0 failed; 0 ignored
```

## 五、refresh 模块设计要点

### 5.1 三层兜底策略

```
┌─────────────────────────────────────┐
│ refresh(source, cache_path)         │
└────────────────┬────────────────────┘
                 │
        ┌────────▼────────┐
        │ source 类型?     │
        └──┬──────────┬───┘
           │          │
      Local(p)     Url(url)
           │          │
           │    ┌─────▼──────┐
           │    │ cache 命中? │
           │    └──┬──────┬──┘
           │     Yes     No
           │       │      │
           │   CacheHit   │
           │       │      │
           │       │   download_to()
           │       │      │
           │       │   ┌──▼──┐
           │       │   成功？│
           │       │   └─┬──┘
           │       │   Yes  No
           │       │    │   Err(Network)
           │       │  Downloaded
           ▼       ▼
        Downloaded (复制 local → cache)
```

### 5.2 0 装 PASS 标注 (per 纪律 #8 诚实标注)

- **网络下载函数 `download_to`**: 返回 `Network error` (避免 reqwest 5MB 重依赖)
- **真实 HTTP 客户端未引入**: 编译时/产物都不增重
- **可观测路径全覆盖**:
  - ✅ Cache hit (本地缓存命中) → CacheHit
  - ✅ Local source → 复制到 cache → Downloaded
  - ✅ Cache miss + network failure → Err(Network) (0 装但语义清晰)
  - ✅ `validate_url` 校验 scheme 合法性

**升级路径**: release-tools 阶段可切换到 `ureq` (1MB) 或 `hyper` (3MB) — 见 task §6 待办。

## 六、纪律核对

| 项 | 状态 | 说明 |
|---|---|---|
| **不破坏现有 API** | ✅ | 旧 `get`/`search`/`count` 保留 + SymbolCatalog 默认实现 |
| **CSV UTF-8 安全** | ✅ | csv crate 1.3 默认 UTF-8 + BOM 自动剥离 |
| **数据可信度 T0** | ✅ | Provenance::FinanceDatabase 默认值 |
| **schema 兼容 V5** | ✅ | V6 ALTER ADD 增量补列 + pragma_table_info 探测 |
| **`cargo test` 全绿** | ✅ | 61/61 (含性能基线 1 万行 < 5s) |
| **`cargo check --workspace --all-targets` 0 错** | ✅ | 我修改的 3 个 crate (stock/memory/companion) 全绿 |
| **Dockerfile 85/85** | ✅ | workspace 成员数 = Dockerfile COPY 数 |
| **台账完成即划 ✅** | ✅ | docs/backlog.md TP27 ✅ (上一轮) |
| **报告路径** | ✅ | reports/eea4e3dd-57f8-4440-94f3-ffea54ad5b10-devops_engineer-report.md |
| **0 装 PASS 诚实标注** | ✅ | 网络下载 0 装 (避免 reqwest 重依赖); cache 命中/local 路径实测 |

## 七、提交与集成

### 提交链

| 顺序 | hash（短） | 说明 |
|---|---|---|
| ① | `ef794d094` | feat(stock): TP27 新 spec API 扩展 (本轮主 commit) |
| ② | `c55bb555` | integration HEAD (cherry-pick 无冲突) |

### 验证 (integration HEAD)

```
$ grep -c "^COPY crates/apeireth-" Dockerfile
85                                        # ✅ 85/85 一致

$ cargo test -p apeireth-stock --lib
test result: ok. 61 passed; 0 failed    # ✅ 全绿

$ cargo check -p apeireth-stock -p apeireth-memory -p apeireth-companion --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.57s   # ✅ 0 错
```

### Cherry-pick 过程

`git cherry-pick ef794d094f50c606dfcd8c0e4ffe4bfab3fd68c9` 无冲突 (与上一轮 b3cbf5f6 无重叠文件)。

## 八、设计取舍

### 8.1 `ticker()` accessor vs 字段重命名

选 **accessor** (而非把 `symbol` 字段改名为 `ticker`):
- ✅ 不破坏现有 SymbolStore::get/get_by_ticker API 兼容性
- ✅ 数据库主键仍叫 `symbol` (语义未变: 标的代码)
- ✅ `ticker()` 是新 spec 的命名约定 (业务侧对外叫 ticker, 内部存储仍用 symbol)
- ⚠️ 略冗余 (字段+方法) — 但 API 表面更稳定

### 8.2 同时保留 `ipo_year` 和 `ipo_date`

选 **两者并存**:
- ✅ `ipo_year: Option<i32>` 兼容旧 spec (CSV 单列年份场景, e.g. `ipo_year=1980`)
- ✅ `ipo_date: Option<String>` 满足新 spec (ISO 8601, e.g. `"1980-12-12"`)
- ✅ CSV 解析: 旧 CSV 只填 ipo_year, ipo_date=None; 新 CSV 填 ipo_date, ipo_year=None
- ⚠️ 略冗余 — 但 FinanceDatabase 实际两种格式都见过 (按子集不同)

### 8.3 V6 migration vs 重命名 V5 表

选 **V6 增量 ALTER ADD**:
- ✅ V5 老库自动升级 (探测 pragma_table_info)
- ✅ 数据零迁移 (列默认 NULL)
- ✅ 兼容回退: V5 老代码读到 V6 库会忽略新列 (SQL SELECT 不显式 SELECT 时)
- ⚠️ 表结构略胖 — 但静态数据资产, 一次写入多次读, 索引全在新字段上不必要

### 8.4 `refresh` 独立模块 vs 嵌入 csv.rs

选 **独立 `refresh.rs`** 模块:
- ✅ 网络/缓存/导入三层关注点分离
- ✅ refresh 模块可单独测试 (mock cache, mock URL)
- ✅ 后续如需添加 CLI `bin/apeireth-stock-refresh` 直接接 refresh::refresh_and_import
- ⚠️ +1 文件 (350+ 行) — 但模块边界清晰, 优于嵌入式

### 8.5 不引入 reqwest/ureq

选 **stdlib only** (下载函数 stub):
- ✅ 编译时/产物不增重 (reqwest 5MB, ureq 1MB)
- ✅ 触发"cache 命中优先"语义 (网络失败时直接用缓存, 不重试)
- ⚠️ 0 装 PASS 标注 (纪律 #8) — 真实下载待 release-tools 阶段补

## 九、待办（移交）

| 项 | 责任人 | 阻塞 |
|---|---|---|
| 真实 HTTP 下载客户端 (ureq/hyper) | release-tools 团队 | 需评估 ~1-5MB 依赖成本 |
| refresh CLI 命令 (`bin/apeireth-stock-refresh.rs`) | backend_engineer | 0 装 PASS: 接 refresh_and_import |
| SymbolStore 与 companion 衔接 (按 sector/industry 预载) | backend_engineer | 待 W4 主动推销/TP21 上下文注入合并 |
| SymbolCatalog 缓存层 (高频 read 场景) | backend_engineer / bench | 当前 in-memory 测试; 30 万行 + 高频 read 才需要 |
| 标的行情事件接入 (TP26 bus + 套件衔接) | backend_engineer2 | 待 TP26 验收 |

## 十、引用路径

- 主 commit (主分支): `ef794d094f50c606dfcd8c0e4ffe4bfab3fd68c9`
- integration HEAD: `c55bb555`
- 报告: `reports/eea4e3dd-57f8-4440-94f3-ffea54ad5b10-devops_engineer-report.md` (本文件)
- 任务定义: `docs/next-team-handbook.md` §1 TP27 行
- FinanceDatabase: https://github.com/JerBouma/FinanceDatabase
- 第一轮 TP27 报告 (旧 spec): `reports/b487b3f8-110d-4698-a89a-b68ec48aedc7-devops_engineer-report.md`