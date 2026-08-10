# Round16 Week 2 报告 — Apeireth 通用 API 扩展平台聚合网关

**日期**: 2026-08-03
**作者**: 楚零（按主人授权）
**HEAD**: ed40bab0 + 后续 commit

---

## 🎯 本周目标 (主人 21:00 决定)

> "除了计费之类的先不管, 扩展性拉满的都要加"

**不做**: quota / billing / subscription / 用户管理
**全做**: Channels 管理 + Router 路由 + Auto-ban + TOML 配置 + 多 provider 类型

---

## ✅ 完工内容

### 新增模块: `crates/apeireth-api/src/gateway/`

**5 个文件**:
- `mod.rs` — 模块入口 + re-export
- `error.rs` — GatewayError (thiserror)
- `channel.rs` — Channel + ChannelType + ChannelStatus + StatusCodeMapping (借鉴 NewAPI 30 列 schema)
- `manager.rs` — ChannelManager (CRUD + 内存表 + filter + stats)
- `router.rs` — GatewayRouter (weight + priority + auto_ban + health tracking)
- `config.rs` — TOML 配置加载 (GatewayConfig + ChannelConfig)

### 8 种 Channel 类型 (Week 1 stub → Week 2 实装)

| ChannelType | 默认 base_url | 协议 |
|-------------|--------------|------|
| **Minimax** | `https://api.minimaxi.com/v1` | OpenAI-compatible |
| OpenAI | `https://api.openai.com/v1` | OpenAI |
| Anthropic | `https://api.anthropic.com` | Anthropic Messages |
| Ollama | `http://localhost:11434/v1` | OpenAI-compatible |
| Gemini | `https://generativelanguage.googleapis.com/v1beta` | Gemini |
| AzureOpenAI | (需配置) | Azure OpenAI |
| OpenAICompatible | (需配置) | 通用 |
| Scripted | (mock) | 测试用 |

### 借鉴 NewAPI 完整 30 列 schema

```rust
pub struct Channel {
    pub id: u32,
    pub channel_type: ChannelType,
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: String,
    pub models: Vec<String>,
    pub group: String,
    pub status: ChannelStatus,           // Enabled / Disabled / AutoBanned
    pub weight: u32,
    pub priority: i32,
    pub auto_ban: u32,                  // 连续失败 N 次触发 auto_ban
    pub model_mapping: HashMap<String, String>,    // 暴露 → 上游模型
    pub status_code_mapping: Vec<StatusCodeMapping>,  // 上游 4xx → 用户 code
    pub header_override: HashMap<String, String>,
    pub param_override: HashMap<String, serde_json::Value>,
    pub setting: Option<serde_json::Value>,
    pub test_model: Option<String>,
    pub created_time: i64,
    pub test_time: i64,
    pub response_time_ms: u64,
    pub used_quota: i64,
    pub balance: Option<f64>,
    pub remark: Option<String>,
}
```

### GatewayRouter 路由逻辑 (Week 2 增强)

```
选 channel 流程:
1. list_for_model(model, user_group)  → 候选渠道 (按可用性过滤)
2. filter AutoBanned + cooldown        → healthy 候选
3. if total_weight > 0:
   - pick = counter % total_weight  (Knuth MMIX 均匀分布)
   - 累积权重算法选最优
4. else:
   - 按 (group 匹配优先, priority 最低) 排序
5. reason: WeightRandom | PriorityHigh | OnlyAvailable | Fallback
```

### ChannelManager CRUD

- `create(Channel) -> id` — 自动分配 ID
- `get(id)`, `get_mut(id, |ch|...)` — 查询 / 修改
- `delete(id)` — 删除
- `list(filter)` — 列表（按 type/status/group/model 过滤）
- `list_for_model(model, user_group)` — 按可用性过滤
- `set_status(id, status)` — 启用/禁用/自动封禁
- `stats()` — 总数/启用/禁用/自动封禁计数

### TOML 配置

`apeireth-gateway.toml`:
```toml
[[channels]]
id = 1
type = "minimax"
name = "Minimax-Primary"
api_key_env = "MINIMAX_API_KEY"     # env var (不落盘 key)
models = ["MiniMax-M3"]
weight = 80
priority = 1
auto_ban = 3

[[channels]]
id = 2
type = "openai-compatible"
name = "OpenAI-Fallback"
api_key_env = "OPENAI_API_KEY"
models = ["gpt-4o-mini"]
weight = 20
priority = 5
group = "vip"                          # vip 用户优先用 vip 渠道
```

**API key 通过 env var 引用**，不落盘 (符合 Apeireth 不落 key 规范)

---

## 📊 验收数字

```
cargo build -p apeireth-api: 0 error (3.37s)
cargo test -p apeireth-api:  58 passed / 0 failed / 0 ignored
cargo build --workspace:   0 error (3.54s)

gateway_demo 跑通:
  - 3 个 channel 创建 + 1 个禁用
  - 100 次路由: Minimax 80, Fallback 20, Ollama 0  ✅ 80/20 分布
  - 3 次失败 → AutoBanned (连续失败触发)
  - Auto-ban 后路由 fallback 到 NoChannelAvailable
  - unban → 重新启用
```

## 📝 30 个单元测试覆盖

- `gateway::channel::tests` (8 个): 构造 / 默认值 / 类型 default URL / 可用性 / model mapping / status code mapping / auto_ban / status is_active / with_*
- `gateway::manager::tests` (8 个): 创建 + ID / 删除 / 列表过滤 / 列表 for_model / set_status / stats / get_mut / 自动跳过已用 ID
- `gateway::router::tests` (10 个): priority / weight random / no channel / 跳过 disabled / 跳过 autobanned / success reset / auto_ban 阈值 / unban / group 隔离 / model mapping
- `gateway::config::tests` (4 个): 最小 TOML / 多 channel / 缺 env 报错 / status_code_mapping

---

## 🚦 Week 2+ 未做（主人明确说"扩展性拉满的都要加"已全做）

| # | 项目 | 决定 |
|---|------|------|
| D | **Users / Tokens / Quota / Subscription** | ❌ 主人明确说"除了计费之类的先不管" |
| HTTP server (axum) + 5 endpoint | ⏸️ Week 2 后续 | HTTP server 让用户能调 |
| 渠道持久化 (SQLite) | ⏸️ Week 3+ | 当前是内存表 |

---

## ✅ Week 2 主任务 = 聚合网关数据层 完成

**已就绪**:
- ✅ Channels CRUD (CRUD + 内存表)
- ✅ 路由决策 (weight + priority + auto_ban + health)
- ✅ 8 种 ChannelType (Minimax / OpenAI / Anthropic / Ollama / Gemini / Azure / Custom / Scripted)
- ✅ TOML 配置驱动
- ✅ NewAPI 真实特性借鉴 (status_code_mapping / model_mapping / weight / priority / auto_ban / group 隔离)

**主任务未做**:
- HTTP server (axum) — Week 2 后续
- 用户/token 鉴权 — 跳过 (计费)

---

**作者**: 楚零（按主人 2026-08-03 21:00 授权）
**架构匹配**: NewAPI channels 表 30 列 + VCP 真实路由决策
**不修改承诺**: 8 项 100% 守住 (新建模块是 v15+ 叠加)