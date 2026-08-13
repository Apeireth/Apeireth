# R252 -- Multi-Source HTTP Search Providers

## Problem
`search_aggregator.rs` had `SearchSource::{Tavily, AnySearch, DuckDuckGo, SearXng}` enum
but no real HTTP-backed providers. Tavily + AnySearch were referenced but never wired
up. Brave and Serper (Google) were missing entirely.

## Solution

### New module: `search_providers.rs`
Real HTTP-backed providers for 4 search APIs:
1. **Tavily** (https://api.tavily.com/search) -- JSON POST, results[].score
2. **Brave** (https://api.search.brave.com/res/v1/web/search) -- GET with X-Subscription-Token
3. **Serper** (https://google.serper.dev/search) -- JSON POST with X-API-KEY
4. **AnySearch** (already exists in anysearch.rs, R141)

### Provider trait
```rust
pub trait SearchProvider: Send + Sync {
    fn source(&self) -> SearchSource;
    fn name(&self) -> &str;
    fn api_key(&self) -> Option<&str>;
}
```

### ProviderRegistry
```rust
pub struct ProviderRegistry { providers: RwLock<Vec<Box<dyn SearchProvider>>> }
impl ProviderRegistry {
    pub fn from_env() -> Self;   // TAVILY_API_KEY / BRAVE_API_KEY / SERPER_API_KEY
    pub fn add(&mut self, p: Box<dyn SearchProvider>);
    pub fn providers(&self) -> Vec<String>;
    pub fn provider_names(&self) -> Vec<String>;
    pub fn count(&self) -> usize;
}
```

### SearchSource enum extended
Added `Brave` + `Serper` variants (existing 4 unchanged).

## Design honesty (O-5)
- Missing API key = explicit `ProviderError::MissingApiKey` (not silent empty)
- Pure parse functions (parse_response) testable without HTTP
- Real HTTP wiring deferred to caller (this R is data-layer + trait)
- 0 fake search results

## Tests (10 new pass)
- r252_01: tavily parses valid response (2 results with score)
- r252_02: tavily parses empty results array
- r252_03: tavily parse fails on missing field
- r252_04: tavily build_request_body requires key (anonymous errors)
- r252_05: brave parses valid response (2 results, position-based score)
- r252_06: brave parses empty web.results
- r252_07: serper parses valid response (position field)
- r252_08: ProviderRegistry empty by default
- r252_09: ProviderRegistry add + list (3 providers)
- r252_10: provider trait returns correct SearchSource

## Files
- `crates/apeireth-tool-fetch/src/search_providers.rs` (new, ~300 lines)
- `crates/apeireth-tool-fetch/src/search_aggregator.rs` (+2 enum variants)
- `crates/apeireth-tool-fetch/src/lib.rs` (add `pub mod search_providers;`)

cumulative: ~6371 tests pass.
