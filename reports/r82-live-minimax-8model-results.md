# R82 LIVE: MiniMax 8 model 真接 benchmark 跑通 (2026-08-09)

> **状态**: ✅ 7/7 models pass, 100% pass rate, total 35.2s
> **环境**: Windows 11 / PowerShell 7 / `APEIRETH_EVAL_LIVE=1`
> **API**: MiniMax Anthropic 兼容 (https://api.minimaxi.com/anthropic/v1/messages)
> **apikey 来源**: `.openclaw\apikey.txt` (per R32-3-1 `DEFAULT_APIKEY_PATHS`)

## 跑法

```bash
Set-Location ".openclaw\workspace\promethean\Apeireth-rust"
$env:APEIRETH_EVAL_LIVE="1"
cargo run --example r70_live_cross_model -p apeireth-eval --release
```

注: 原本 8 model (DEFAULT_MODELS + EXTENDED_MODELS dedup), 实跑 7 (M2 排除因未列在 DEFAULT/EXTENDED 中)

## 跑通 evidence (verbatim stdout)

```
R70 LIVE: base=https://api.minimaxi.com path=/anthropic/v1/messages version=2023-06-01 apikey_source=file:.openclaw\apikey.txt
R70 LIVE: running 7 models: ["MiniMax-M2.7-highspeed", "MiniMax-M2.7", "MiniMax-M2.5", "MiniMax-M3", "MiniMax-M2.5-highspeed", "MiniMax-M2.1-highspeed", "MiniMax-M2.1"]
# Cross-Model Benchmark — 2026-08-09 16:03:28 UTC

- **Prompt**: `Reply with a single JSON object: {"ok": true, "model": "<you decide your model>"`
- **Endpoint**: `https://api.minimaxi.com/anthropic/v1/messages`
- **Total latency**: 35197 ms
- **Pass rate**: 7/7 (100%)

| Model | Status | Latency (ms) | In | Out | Stop | Text excerpt | All pass |
|-------|--------|--------------|----|----|------|--------------|----------|
| `MiniMax-M2.7-highspeed` | 200 | 6855 | 63 | 265 | `end_turn` | {   "ok": true,   "model": "MiniMax-M2.7 | ✅ |
| `MiniMax-M2.7` | 200 | 7035 | 51 | 295 | `end_turn` |  {"ok": true, "model": "gpt-4"} | ✅ |
| `MiniMax-M2.5` | 200 | 5107 | 63 | 286 | `end_turn` | {"ok": true, "model": "gpt-4"} | ✅ |
| `MiniMax-M3` | 200 | 1652 | 57 | 15 | `end_turn` | {"ok": true, "model": "MiniMax-M3"} | ✅ |
| `MiniMax-M2.5-highspeed` | 200 | 4831 | 63 | 237 | `end_turn` | {"ok": true, "model": "my-model"} | ✅ |
| `MiniMax-M2.1-highspeed` | 200 | 6258 | 58 | 196 | `end_turn` | {"ok": true, "model": "MiniMax-M2.1"} | ✅ |
| `MiniMax-M2.1` | 200 | 3452 | 58 | 144 | `end_turn` | {"ok": true, "model": "gpt-4"} | ✅ |

**Fastest passing**: `MiniMax-M3`
**Cheapest passing (output tokens)**: `MiniMax-M3`

R70 LIVE summary: 7/7 pass (rate 100%); total 35197ms
```

## 结论

- ✅ 7/7 model 端到端跑通, 0 fail
- ✅ Token 报数真 (input 51-63, output 15-295)
- ✅ Response shape 跟 Anthropic spec 1:1 (content[] + usage + stop_reason)
- ✅ `MiniMax-M3` 最快 (1652ms) + 最少 output (15 tokens)
- ✅ `MiniMax-M2.7` / `M2.5` / `M2.1` 都返了 `model: gpt-4` (model identity 误报, 但 response shape 仍合规, 判定 pass)

## 集成点

- R32-3-1 `real_llm_smoke::RealLlmSmokeReport` 7 阶段 metric 100% 适用
- R32-3-2 `cross_model_benchmark::run_cross_model_benchmark` 复用
- apikey load: `load_api_key(None)` -> `file:.openclaw\apikey.txt` 自动 fallback

## 后续可推 (本批不做)

- R82-1: 加 8 model (MiniMax-M2 / DeepSeek-V3 / GPT-4o / Claude 3.5) 多厂商 cross-vendor
- R82-2: 加 token cost 计算 (per model pricing)
- R82-3: 加 CI workflow (matrix: model × prompt)
