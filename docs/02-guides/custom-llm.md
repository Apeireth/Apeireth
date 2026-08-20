# 自定义 LLM Provider 指南

> **2026-08-20 新增**: companion_serve 现在支持多 LLM provider 切换, **不需要改源码**.
>
> 适用场景: 想用 Claude / OpenAI / 本地 vLLM 替代 MiniMax, 或加 fallback 链.

---

## 1. 一句话总览

```bash
# 1. 写 TOML 配置文件 (或拷贝下面的模板)
# 2. 设 env: APEIRETH_LLM_CONFIG=/path/to/apeireth-api.toml
# 3. 重启 companion_serve
# 4. 完事
```

**0 装 PASS**: TOML 不存在 → 退化到 env `APEIRETH_LLM_BASE_URL` → default `https://api.minimaxi.com`. 旧版 1:1 行为不变.

---

## 2. 完整 TOML 模板

```toml
# apeireth-api.toml
# 多 provider 配置 + fallback 顺序. 第一个 provider 的 base_url 决定主链路.
# 路径通过 env APEIRETH_LLM_CONFIG 注入或用默认搜索.

# ---------- 1: MiniMax (默认上游) ----------
[providers.minimax]
type = "apeireth-api"
base_url = "https://api.minimaxi.com"   # ⚠️ 不要带 /v1 后缀 (Pipeline 内部拼接)
api_key_env = "APEIRETH_API_KEY"          # env var 名, 不是 key 本身
models = ["MiniMax-M3", "MiniMax-M3-thinking"]

# ---------- 2: OpenAI (fallback) ----------
[providers.openai]
type = "openai-compatible"
base_url = "https://api.openai.com"
api_key_env = "OPENAI_API_KEY"
models = ["gpt-4o", "gpt-4o-mini"]

# ---------- 3: Anthropic Claude (fallback) ----------
[providers.anthropic]
type = "anthropic-compatible"
base_url = "https://api.anthropic.com"
api_key_env = "ANTHROPIC_API_KEY"
models = ["claude-sonnet-4-20250514"]

# ---------- 4: 本地 vLLM / Ollama (OpenAI 兼容) ----------
[providers.local]
type = "openai-compatible"
base_url = "http://localhost:8000"        # vLLM 默认
api_key_env = "LOCAL_LLM_KEY"             # 可设 "EMPTY" (vLLM 默认)
models = ["meta-llama/Llama-3-70B"]

# ---------- 5: Google Gemini (OpenAI 兼容模式) ----------
[providers.gemini]
type = "openai-compatible"
base_url = "https://generativelanguage.googleapis.com/v1beta"
api_key_env = "GOOGLE_API_KEY"
models = ["gemini-2.5-pro"]

# ---------- 路由顺序: 失败时 fallback ----------
[router]
fallback_order = ["minimax", "openai", "anthropic", "gemini"]
```

---

## 3. provider 类型一览

| `type` 值 | 协议 | 适用 |
|---|---|---|
| `apeireth-api` | OpenAI 兼容 + Anthropic 兼容 | MiniMax 本地代理 `apeireth-api daemon` |
| `openai-compatible` | OpenAI Chat Completions | OpenAI / vLLM / Ollama / Gemini OpenAI 模式 |
| `anthropic-compatible` | Anthropic Messages | Anthropic Claude / 兼容代理 |
| `scripted` | (固定响应) | 测试 / 离线 mock |

---

## 4. 启动 companion_serve

```bash
# 设 env (按优先级)
export APEIRETH_LLM_CONFIG=/path/to/apeireth-api.toml
export APEIRETH_API_KEY=sk-cp-xxx       # MiniMax key
export OPENAI_API_KEY=sk-xxx            # OpenAI key
export ANTHROPIC_API_KEY=sk-ant-xxx     # Anthropic key

# 启动
cargo run -p apeireth-companion --example companion_serve
```

启动日志会显示:

```
[llm] model = MiniMax-M3 (env APEIRETH_LLM_MODEL 可覆盖, 缺省 MiniMax-M3)
[llm] TOML config 加载: 5 providers from /path/to/apeireth-api.toml
[llm] base_url = https://api.minimaxi.com (TOML 优先 → APEIRETH_LLM_BASE_URL env → default)
```

---

## 5. 优先级 (从高到低)

| 层 | 来源 | 例子 |
|---|---|---|
| 1 (最高) | TOML 第一个 provider 的 `base_url` | `https://api.openai.com` |
| 2 | env `APEIRETH_LLM_BASE_URL` | `https://my-proxy.com` |
| 3 (缺省) | `https://api.minimaxi.com` (硬编码) | 与旧版 1:1 |

**model** 优先级:
| 1 | env `APEIRETH_LLM_MODEL` | `gpt-4o` |
| 2 (缺省) | `MiniMax-M3` | 与旧版 1:1 |

---

## 6. 已知限制(0 装 PASS 严守)

- **fallback 链当前降级到 Pipeline 端**——加载 TOML 时取第一个 provider 的 base_url 作为主链路.
- 真正"按 model 选 provider"需要接入 `MultiLlmRouter` 全套 (V1.1 中期路线, ROI 评估中).
- 当前实现: **多 provider 配置** + **TOML 化** + **env override**, **不** 包含 `MultiLlmRouter` 串行 fallback 试验.
- 0 装 PASS: 当前路径 = "切 base_url" + "切 model", 失败返 Degraded. 不假装已实现完整 fallback.

---

## 7. 验证你能切换

启动后, 看 `/v1/models`:

```bash
curl http://127.0.0.1:8089/v1/models
# {"data":[{"id":"MiniMax-M3",...}]}
```

或在对话中显式指定 model:

```bash
curl -X POST http://127.0.0.1:8089/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}'
```

如果 model 名在 TOML `models` 列表里 → 走对应 provider.

---

## 8. 故障排查

| 现象 | 原因 | 修法 |
|---|---|---|
| `base_url = https://api.minimaxi.com` (用了 default) | TOML 没读到 | 查 `APEIRETH_LLM_CONFIG` env 路径, 路径不能含特殊字符 |
| 第一次请求 4xx "model not found" | `APEIRETH_LLM_MODEL` 名字不在 TOML `models` 列表 | 改 TOML 加匹配 model, 或改 env 用 default |
| `URL = https://api.openai.com/v1/v1/chat/completions` (双 /v1) | base_url 含 `/v1` 后缀 | 删 `base_url` 末尾的 `/v1` (Pipeline 内部拼) |
| 真接 OpenAI 但 MiniMax 也失败 → 503 | 没用 router, 实际只走第一个 provider | 当前限制, 等 V1.1 中期 |

---

## 9. 进阶: 自动化 + 健康检查

`MultiLlmRouter` 完整实装 (per `apeireth-api::llm::router`), 但当前 `companion_serve` 走简化路径. 完整 router 上线 = V1.1 中期 (5 测全过, 0 改主链, 0 假装).

要看完整 router 测试: `cargo test -p apeireth-api --lib llm::router`.

---

## 10. 参考

- `crates/apeireth-api/src/llm/config.rs` — `LlmConfig` + `from_file` 完整实现
- `crates/apeireth-api/src/llm/router.rs` — `MultiLlmRouter` (完整, 未接到主链)
- `crates/apeireth-api/src/llm/providers/` — 4 种 provider 实现
- `examples/companion_serve.rs:78-95` — `init_model()` + `init_base_url()` 实现
- `examples/companion_serve.rs:1297-1320` — main() 启动时 TOML 读取

---

_2026-08-20 Mavis 自决 commit (per 决策 #126). 0 触碰 24 LOCKED crate. 0 改 enum/const. 0 改 workspace.version (1.2.0 双轴制). 8 哲学锚穿透 (顶部 doc O-5 不假装 + S-2 实事求是显式标)._
