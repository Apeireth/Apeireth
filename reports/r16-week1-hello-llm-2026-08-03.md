# Round16 Week 1 报告 — `apeireth-llm` 多 provider 抽象平台 (NewAPI 风格)

**日期**: 2026-08-03
**作者**: 楚零（按主人授权）
**路径**: `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-llm\`

---

## ✅ 完工内容

### 1. 新建 crate `apeireth-llm`（加入 workspace）

**11 个 Rust 文件** + 1 个 Cargo.toml：

```
crates/apeireth-llm/
├── Cargo.toml                                    ← workspace member
├── src/
│   ├── lib.rs                                    ← re-exports + 编译期 hardcode 常量
│   ├── traits.rs                                 ← LlmProvider trait + Request/Response/Error/Capability/Health
│   ├── error.rs                                  ← LlmError (thiserror + retryable 分类)
│   ├── router.rs                                 ← MultiLlmRouter (fallback + health tracking)
│   ├── config.rs                                 ← TOML 配置加载 (apeireth-api.toml)
│   ├── middleware.rs                             ← LoggingMiddleware + RetryMiddleware + MiddlewareChain
│   └── providers/
│       ├── mod.rs
│       ├── apeireth_api.rs                       ← ⭐ 主适配 (ApeirethApiProvider)
│       ├── openai_compat.rs                      ← 通用 OpenAI-compatible
│       └── scripted.rs                           ← 测试用 mock
└── examples/
    ├── hello_llm.rs                              ← 主人验收用 (需 APEIRETH_API_KEY)
    ├── router_demo.rs                            ← Router + Middleware 演示 (跑通)
    └── config_demo.rs                            ← TOML 配置驱动演示 (跑通)
```

### 2. 测试

```
cargo test -p apeireth-llm

test result: ok. 22 passed; 0 failed; 0 ignored
```

**22 个单元测试覆盖**:
- `config::tests`: 4 个 (TOML 解析 + build_router)
- `providers::apeireth_api::tests`: 4 个 (env 缺失 / response 解析 / capabilities / supports_model)
- `providers::openai_compat::tests`: 1 个 (basic)
- `providers::scripted::tests`: 4 个 (basic / default / case insensitive / multiple scripts)
- `router::tests`: 6 个 (basic / fallback on error / provider_names / supports_model 等)
- `middleware::tests`: 3 个 (logging / retry / chain)

### 3. Workspace 编译验证

```
cargo build --workspace

Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.14s
0 error
```

**没破坏其他 28 个 crate**。

### 4. Examples 跑通

| Example | 跑通 | 备注 |
|---------|------|------|
| `hello_llm` | ⚠️ 等 API key | 优雅报错 `Config("APEIRETH_API_KEY env var not set")` |
| `router_demo` | ✅ | 显示 router.try / router.success tracing 日志 |
| `config_demo` | ✅ | TOML 解析 + Router 构建 + 3 个测试场景 |

---

## 🏗️ 架构亮点（"具有优秀扩展性的通用平台"）

### 1. 5 个扩展点（加新 provider 极简）

**位置**: `crates/apeireth-llm/src/providers/` + `config.rs::build_provider`

**加新 provider 的步骤**:
1. 创建 `xxx.rs` 实现 `LlmProvider` trait
2. 在 `providers/mod.rs` 加 `pub mod xxx;`
3. 在 `config.rs::build_provider` 加 match arm
4. 在 TOML 用 `type = "xxx"`

**零修改**: trait / router / middleware / 其他 provider

### 2. 配置驱动 (TOML)

`apeireth-api.toml`:
```toml
[providers.apeireth-api]
type = "apeireth-api"
base_url = "http://localhost:3000/v1"
api_key_env = "APEIRETH_API_KEY"
models = ["MiniMax-M3", "MiniMax-M3-thinking"]

[providers.openai]
type = "openai-compatible"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
models = ["gpt-4o"]

[providers.test]
type = "scripted"
api_key_env = "APEIRETH_LLM_NO_KEY"
scripts = { "hello" = "hi back" }

[router]
fallback_order = ["apeireth-api", "openai", "test"]
```

### 3. Capability 声明系统

```rust
pub struct ProviderCapabilities(u32);

impl ProviderCapabilities {
    pub const NONE: Self = Self(0);
    pub const CHAT: Self = Self(1 << 0);
    pub const STREAMING: Self = Self(1 << 1);
    pub const TOOLS: Self = Self(1 << 2);
    pub const VISION: Self = Self(1 << 3);
    pub const JSON_MODE: Self = Self(1 << 4);
    pub const SYSTEM_PROMPT: Self = Self(1 << 5);
    pub const THINKING: Self = Self(1 << 6);
    pub const LONG_CONTEXT: Self = Self(1 << 7);
    pub const CUSTOM_TEMPERATURE: Self = Self(1 << 8);
}
```

每个 provider 声明自己支持什么能力 (bitmap)。Router / Council 等消费者据此路由。

### 4. 可插拔中间件

```rust
let chain = MiddlewareChain::new()
    .with(Arc::new(LoggingMiddleware::new()))
    .with(Arc::new(RetryMiddleware::new(3, 500)));

// 日志 + retry + ... 任意组合
chain.run(provider, req).await?;
```

Week 2+ 加: `MetricsMiddleware` / `CacheMiddleware` / `RateLimitMiddleware`

### 5. 错误分类 + retryable 区分

```rust
impl LlmError {
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            LlmError::RateLimited { .. }
            | LlmError::Timeout { .. }
            | LlmError::Network { .. }
        )
    }
}
```

Router 只对 retryable 错误触发 fallback，其他错误（AuthFailed / BadResponse）直接返回 —— **不会浪费配额**。

### 6. Health Tracking (EMA)

```rust
pub struct ProviderHealth {
    pub healthy: bool,
    pub latency_p50_ms: u64,        // 指数移动平均
    pub error_rate: f64,             // 0.0 - 1.0
    pub consecutive_failures: u32,
}
```

每次调用更新，连续 3 次失败标 unhealthy。

### 7. 编译期 hardcode

```rust
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;
pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_RETRY_BACKOFF_BASE_MS: u64 = 500;
pub const DEFAULT_MAX_CONCURRENT: usize = 32;

const _: () = {
    assert!(DEFAULT_TIMEOUT_MS >= 1_000);
    assert!(DEFAULT_MAX_RETRIES <= 10);
    assert!(DEFAULT_RETRY_BACKOFF_BASE_MS >= 50);
    assert!(DEFAULT_MAX_CONCURRENT >= 1);
};
```

**Apeireth 工程文化**: 关键值编译期 const，**违背编译错误**。

---

## 🧪 主人验收步骤

```powershell
# 1. 设置 API key（主人给）
$env:APEIRETH_API_KEY = "<key>"

# 2. 跑 hello_llm (Round16 Week 1 主验收)
cd .openclaw\workspace\promethean\Apeireth-rust
cargo run -p apeireth-llm --example hello_llm

# 期望输出：
# ✅ Provider: apeireth-api
# ✅ Response 内容
# ✅ Token usage (prompt / completion / total)
# ✅ latency_ms
# ✅ finish_reason: stop
```

---

## ⚠️ Week 1 不做的事（Week 2+ 计划）

| 项目 | 状态 | 计划 |
|------|------|------|
| Streaming (SSE) | ❌ 未实装 | Week 2+ |
| Tool calling | ❌ 未实装 | Week 2+ |
| Vision input | ❌ 未实装 | Week 2+ |
| Ollama 专用 provider | ❌ stub | Week 2+（用 OpenAI-compatible 替代） |
| Anthropic 专用 provider | ❌ stub | Week 2+（如需 Claude 原生 tool use） |
| 热重载配置 | ❌ 未实装 | Week 2+ |
| HTTP server (axum) | ❌ 未实装 | **Week 2 主任务** |
| Council 7 advisor 真接入 | ❌ 未动 | **Week 3 主任务** |

---

## 📊 数字

| 维度 | 值 |
|------|-----|
| 新增文件 | 11 个 Rust + 1 个 Cargo.toml |
| 总行数（src） | ~2,500 行（含测试） |
| 测试 | 22 passed / 0 failed |
| cargo build warnings | 119 个（主要是 missing_docs + unused imports） |
| cargo build errors | 0 |
| compile time | 12.14s（workspace 全量） |
| 编译期 const assert | 4 个 |
| Provider 实现 | 3 个实装（apeireth-api / openai-compat / scripted）+ 2 个 stub（ollama / anthropic）|
| 中间件 | 2 个实装（logging / retry）+ 3 个 stub（metrics / cache / rate-limit） |
| Capability flags | 9 个 |

---

## ✅ 验收结论

**Week 1 主体完工，等主人 API key 真接通验证**。

- ✅ 编译通过
- ✅ 测试通过（22/22）
- ✅ router_demo + config_demo 跑通
- ⏳ hello_llm 等 `APEIRETH_API_KEY`

**没有突破 8 项不修改承诺**（新建 crate 是叠加，不修改 LOCKED）。

---

**作者**: 楚零（按主人 2026-08-03 19:31 授权）
**下次开工**: 主人给 API key + 跑 hello_llm