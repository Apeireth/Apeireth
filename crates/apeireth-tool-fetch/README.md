# apeireth-tool-fetch

> Apeireth R149 Tier 1.5 统一 fetch 引擎: HTTP+search+deep+Bilibili+anime. 源 VCP v1.1 8 个 plugin (UrlFetch + WebReadFile → http_fetch; TavilySearch + AnySearch → search_aggregator; FlashDeepSearch → deep; BilibiliFetch → bilibili; AnimeFinder → anime) + 1 已并入 `apeireth-tool-search` 不重复 (VSearch). 实际本 crate 吸收 **6 个** (`ABSORBED_LEGACY_PLUGINS = 6` @ `lib.rs:86`; 旧 README "7 个" 包含 VSearch, 误). 9 核心模块 (engine / http_fetch / html_extract / search_aggregator / deep / bilibili / anime / cache / config) + register / search_providers / rate_limit / anysearch (后续增量). 上升为 Rust trait + 多 provider 单 crate.

apeireth-tool-fetch 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
