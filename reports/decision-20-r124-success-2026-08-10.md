# Decision #20 — R124-1/3 success 备注 + R125-1 推荐 (LiteLLM Provider Registry)

**Date**: 2026-08-10 16:19
**Author**: Mavis (root session, 主人 "你拍" 授权有效)
**Status**: ✅ 决策登记 (推荐路线, 派活由 Mavis 调度下个 tick 决定)

---

## 1. 触发事件

R124 GitHub 调研 wave 3 任务 (16:14 派):

| Task ID | 模块 | 状态 | 报告 |
|---|---|---|---|
| `bg_ce7b9e8f` R124-1 | 战区 1-2 (TUI + LLM Gateway, 8 模块) | ✅ **succeeded 16:19** | 41,744 bytes, 28 候选 + 30 借鉴 + 22 ID |
| `bg_ea620f18` R124-2 | 战区 3 (Multi-Agent, 13 模块) | 🟡 running (报告 46,995 bytes 已写, 待 mark done) | 报告 47KB |
| `bg_1b4494f4` R124-3 | 战区 4-5+L0+跨战区 (16 模块) | ✅ **succeeded 16:19** | 49,243 bytes, 64 候选 + 68 借鉴 + 77 ID |

**距 16 cap 12 slots**: 5 succeeded + 2 running = 7 active, 还剩 9 slots.

---

## 2. R124-1 Top 5 优先借鉴 (R125+ 续实施路线)

| 排名 | 借鉴 | ROI | 模块 | 周期 |
|---|---|---|---|---|
| **Top 1** | **LiteLLM style provider registry 抽象** (协议层 + 适配层) | 当前 324.6KB → 借鉴后 115KB, **-209.6KB (-65%)** | `apeireth-pipeline` + `apeireth-protocol` + `apeireth-api` | 3-5 天 |
| Top 2 | hyper 池 + clap derive (2 项) | TUI/cli/http-client 多处复用 | `apeireth-http-client` + `apeireth-cli` | 4-6 小时/项 |
| Top 3 | LangGraph 状态机 + OpenCode 子代理 | 双重 1-2 周 | `apeireth-pipeline` + `apeireth-tui` | 1-2 周 |
| Top 4 | agent 协议字段扩展 | TUI/acp/api 三处一致 | `apeireth-acp` | 1-2 天 |
| Top 5 | TUI 多代理 | ratatui 5 nav 提速 30% | `apeireth-tui` | 1-2 天 |

**R124-3 配套借鉴** (R125+ 续可拼):
- Top 1: `modelcontextprotocol/servers` → `apeireth-mcp` 协议对齐
- Top 2: `PyO3/PyO3` → `apeireth-pybridge` 重构
- Top 3: `NVIDIA-NeMo/Guardrails` → `apeireth-sovereignty` Colang DSL 借鉴
- Top 4: `model-checking/kani` → `apeireth-formal` 覆盖度扩展
- Top 5: `asg017/sqlite-vec` → `apeireth-vector` 单文件降级路径

---

## 3. R125-1 推荐实施 spec (LiteLLM Provider Registry)

**实施位置**: `crates/apeireth-pipeline/src/provider_registry.rs` (NEW mod, lib.rs 加 pub mod)

**核心骨架** (50 min 内可完成 17:30 截止):
```rust
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn endpoint_url(&self) -> &'static str;
    async fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse, ProviderError>;
    fn supports_stream(&self) -> bool { true }
}

pub struct ProviderRegistry {
    providers: HashMap<ProviderKind, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, p: Arc<dyn Provider>) { ... }
    pub fn dispatch(&self, kind: ProviderKind, req: NormalizedRequest) -> Result<...> { ... }
    pub fn kinds(&self) -> Vec<ProviderKind> { ... }
}

// 1 个 stub provider (OpenAI 协议), 8 unit test
pub struct OpenAiChatProvider;  // 走 protocol_handlers::dispatch 现有 1.0 行为
```

**整合 R122-5** (`crates/apeireth-api/src/llm/semantic_router.rs`):
- 0 替换 semantic_router, 0 触碰 R122-5 commit (df6dfb69)
- semantic_router 作为 ProviderRegistry 的"路由器"上层, 0 破坏
- R122-5 仍是 default routing, R125-1 提供"协议层透明切换"能力

**8 硬墙全守**:
1. ✅ workspace.version 1.1.0 (0 改)
2. ✅ R11 baseline 0.8682/0.8532/0.9063 (0 触碰 `integration_r_measure.rs:42-44`)
3. ✅ 24 LOCKED crate mtime (0 触碰)
4. ✅ 6 哲学锚 (0 触碰)
5. ✅ 9 organ logic (0 触碰)
6. ✅ 11 agent 公共 API (0 改, Provider trait 是新增 0 改 11 现有)
7. ✅ 0 装 (O-5) — 1 个 stub provider 真实接 openai 协议, 0 假装多 provider
8. ✅ 0 主动 commit (Mavis 整合 #3 拍板)

**8 unit test 必过**:
1. `register_and_dispatch_openai`
2. `register_and_dispatch_anthropic` (新 stub)
3. `register_4_protocols_4_entries`
4. `dispatch_unknown_kind_returns_error`
5. `endpoint_url_per_provider_static`
6. `supports_stream_default_true`
7. `registry_send_sync_compiles`
8. `semantic_router_integration_unchanged` (验证 R122-5 0 漂移)

**17:30 截止**: 50 min 内可完成骨架 + 1 provider + 8 test, 完整 LiteLLM style 4+ provider 留 R125-2/3 续接.

---

## 4. 0 拍板 R125-1 立即派

**Mavis 调度建议**: R125-1 派 LiteLLM Provider Registry 骨架 (50 min 17:30 截止).

**理由**:
1. R124-1 调研已确认 Top 1 ROI = -209.6KB, 最高回报
2. 50 min 内可完成骨架 + 1 provider (openai) + 8 test, 0 触碰 8 硬墙
3. 完整 LiteLLM style 4+ provider 留 R125-2/3 续, 不超 17:30 截止
4. 距 16 cap 12 slots 还剩 9, 资源充足

**拍板风险**:
- R123-1 (bg_4bb44b63) clippy 9 批 + doc 1 批, 32 min 日志没更新, 仍在跑 — Mavis 5 min 节奏观察, 不干预
- R124-2 (bg_ea620f18) 报告 47KB 已写完, task 仍 running — Mavis 下个 tick 应自动 mark done

---

## 5. 拍板执行

- [x] 写本决策文件 `decision-20-r124-success-2026-08-10.md`
- [x] 推荐 R125-1 派 LiteLLM Provider Registry 骨架
- [ ] Mavis 调度下个 tick 决定派 / 不派 (R123-1 / R124-2 优先)
- [ ] 17:30 写 R123+R124 final report + 拍板 commit (主人 "你拍" 授权持续)
