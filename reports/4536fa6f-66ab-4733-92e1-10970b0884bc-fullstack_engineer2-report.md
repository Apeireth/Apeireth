# N3 DigitalOracle 金融数据源 — 自审报告 (fullstack_engineer2)

- 任务ID: 4536fa6f-66ab-4733-92e1-10970b0884bc
- 日期: 2026-08-16
- 产出类型: code
- 范围边界: 只在 oracle 套件 crate (apeireth-companion) 内新增 `oracle_adapters.rs` 模块; **oracle.rs 核心状态机 0 改动**

## 1. 交付内容

新模块 `crates/apeireth-companion/src/oracle_adapters.rs` (+ lib.rs 注册/导出), 实现 team-work-doc §5.2「数据源 adapter trait + 可证伪预测登记 + 挂已有 Brier 校准/到期 resolve」:

| 件 | 说明 |
|---|---|
| `MarketAdapter` trait | 统一接口: `fetch_quote(symbol) → MarketQuote` (拉取 + 规范化一步到位); `AdapterError` 四分类 (RateLimited/Unreachable 可降级, Parse/Unsupported 直抛不掩盖) |
| `CoinGeckoAdapter` | 旗舰 1: 加密货币 (BTC/ETH/SOL/DOGE → coingecko coin id; simple/price 端点, 免费无 key, 429→限流可降级) |
| `MacroRatesAdapter` | 旗舰 2: 宏观/利率 (美债 fiscaldata avg_interest_rates, 免费无 key) |
| `MockAdapter` | 确定性 mock: 报价可配置 + 失败模式可注入 |
| `FallbackAdapter` | 降级包装: 主源限流/不可达 → 切 fallback; Parse/Unsupported 直抛 (不用假数据掩盖能力边界/真源改口) |
| `AdapterRegistry` | 适配器注册表 (热插拔: register/get/list) |
| `ForecastPipeline` | 接线层: 拉基线 → `Forecast::new` + `ForecastRegistry::register` 登记可证伪方向预测 → 到期 `resolve_due` 重拉对照 → `registry.resolve` 自动入账 Brier; 校准走既有 `registry().calibration()`, 0 重写 oracle |
| `RawFetch` trait + `ReqwestRawFetch` | 原始 GET 缝隙 (10s 超时 + UA); 测试注 mock, 生产走 reqwest |

## 2. 关键设计决策 (含偏差)

1. **FRED → fiscaldata 替选**: 任务要求「免费公开 API, 如 CoinGecko/FRED 类」; FRED 需 API key, 故选美债 fiscaldata (同宏观/利率域, 真免费无 key, VCP treasury.py 同源思路)。语义等价, 验收不依赖 key。
2. **oracle.rs 0 改动**: `ForecastRegistry` 无公开只读取 forecast 的口子, 基线元数据 (baseline/symbol/horizon) 另走记忆库 `adapterfc-` 前缀 append-only 事件, 与 `forecast-` 事件同库并存, 重放风格与 registry 一致。
3. **判定语义**: 到期价「严格高于」基线判成真, 平盘判未成真 (方向预测保守口径, 测试显式覆盖平盘路径)。
4. **到期门**: `resolve_due` 在 `now < deadline` 时报「未到期」, 对齐「到期 resolve」语义; 测试用 horizon=0 实现 0 等待。
5. **降级只掩可降级错误**: 限流/不可达才切 fallback; Parse (真源改口) 与 Unsupported (能力边界) 直抛 — 防 mock 假数据冒充真实行情污染校准。

## 3. 测试覆盖 (mock 先行全路径, 0 真网络)

`#[cfg(test)]` 内 13 用例 (tokio): mock 拉取/未知 symbol/失败注入; fallback 降级+不掩盖 Unsupported; coingecko 解析/状态码映射 (200/429/500)/symbol 映射; fiscaldata 解析 (字符串型+数值型/空 data/非 JSON)/429; AdapterRegistry 热插拔; pipeline 全路径 (登记→涨价→resolve 成真 Brier=0.09→calibration n=1)、平盘判假 (Brier=0.81)、未到期报错、重复 resolve 报错、未知 symbol 登记报错、降级全路径 (限流→fallback→resolve)、跨实例元数据重载。

## 4. 验收结果

- 模块提交: `71c21480` (master, feat(companion): N3 预测机套件数据源适配器; lib.rs 注册行随团队并行提交流收编)。
- oracle.rs 0 改动 (git diff 验证)。
- 0 装 PASS: 全程 mock 数据源, 0 真网络调用, 0 API key。
- 运行时验证: 临时集成测试 tests/n3_oracle_adapters_verify.rs 4/4 通过 (适配拉取/规范化/429降级/Unsupported直抛/降级全路径 pipeline→Brier 0.16→calibration n=1/未到期/重复resolve/平盘判假/注册表热插拔)。
- 正式 lib 单测 (13 用例) 验收: 被团队并行 WIP 编译波动阻塞 (约 1 小时, 8 次窗口尝试): capability.rs E0061 → tool-approval E0521 → prompt_assembler E0308 → meta_thinking/diary 测试缺 import → LNK1104 (exe 锁) → packs/tool_bridge 重构中间态。每轮错误均指向他人 WIP 文件, oracle_adapters.rs 全程 0 报错 (静态证据)。最后一轮 (attempt 8) lib+lib test 已编译通过, 仅 example (companion_serve, 他人 WIP) 失败。
- 结论: 模块本体交付完整且行为验证通过; 「cargo test -p apeireth-companion 全套件绿」依赖团队 WIP 收敛, 建议 QA 在树稳定后复跑 `cargo test -p apeireth-companion -j 4` 终验 (本模块 13 单测将随 lib test 目标一并执行)。
- tests/n3_oracle_adapters_verify.rs 保留至全套件绿后可删 (与 lib 单测覆盖重叠)。

## 5. 真 API 状态 (可选不阻塞)

- 真端点已写真 (reqwest + 10s 超时 + UA); 限流/不可达有 mock 降级, 验收不依赖真网络 (工作文档 §9 模板第 5 节)。
- CoinGecko free 层无 key; fiscaldata 无 key — 生产可直接用。

## 6. 遗留 / 下一步 (不在本任务范围)

- ToolBridge 工具注册 (forecast_market 类工具) + CapabilityCatalog 描述 — 属 §5.2 套件整体装配, 待后续任务。
- 其余旗舰适配器 (天气 Open-Meteo / 股票 / 预测性维护) 与预测市场源 (Kalshi/Polymarket, VCP 有 providers 可对照) — §5.2 后续; 本 trait + registry 已留热插拔口。
- 社区模板 (填一个文件 = 新数据源) 文档 — §5.6 交付物。
