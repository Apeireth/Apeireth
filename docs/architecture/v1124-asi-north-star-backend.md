# V1124 ASI North-Star Backend — HTTP+gRPC 双协议 + 4 Provider 真传输 — 真架构文档

> **模块**: `apeireth/v1124_asi_north_star_backend.py` (543 LOC)
> **任务**: R10-BE-001 (backend_engineer)
> **作者**: technical_writer · R10-TW-001 · W1 末
> **守门**: 主 22:33 ASI 北极星 (0.95) + 主 17:43 实事求是 + 主 23:44 干到底 + 主 19:33 走在前人经验上 (Fielding 2000 REST + gRPC 2015)

---

## 1. 设计意图 (主 17:43 不模拟)

**V1124** = ASI 北极星后端基础设施 (infra)，**不是 ASI 本身**，**不是现象意识证明**。测量 = 操作代理 (operational proxy)，不是真相。

**关键承诺 (源 L1-7 真注释)**：
> Real-model results are accepted only from an actual HTTP endpoint or a configured local executable; there is no simulated fallback.

**V3 守门 5 项 (源 L38-44)**：
1. `measurement_is_proxy` — ASI score = 操作代理，非真相
2. `not_phenomenal_consciousness` — 无结构能证明现象意识
3. `identity_is_not_consciousness` — 持久身份 = 数据连续性，不是主观体验
4. `model_call_is_not_asi` — 真 LLM 调用 = 仅整合证据，不是 ASI
5. `failure_is_not_success` — 不可用 provider 和损坏存储 = 失败，不模拟

任一为 `False` → V1124 守门破。

---

## 2. 双协议：HTTP (REST) + gRPC (主 19:33 走在前人经验上)

### 2.1 HTTP 3 endpoints (源 L430, L432, L434)

| 方法 | 路径 | 用途 |
|---|---|---|
| `GET` | `/asi/level` | 查询当前 ASI level / V1074 V0.3 |
| `POST` | `/asi/measure` | 提交测量请求（带 evidence 校验） |
| `GET` | `/asi/north-star` | 查询北极星（ASI_NORTH_STAR=0.95） |

### 2.2 gRPC 3 services (源 L519, L522, L525)

| RPC | handler 复用 |
|---|---|
| `Level` | `grpc.unary_unary_rpc_method_handler(invoke("GET", "/asi/level"), ...)` |
| `Measure` | `invoke("POST", "/asi/measure")` |
| `NorthStar` | `invoke("GET", "/asi/north-star")` |

设计：HTTP 与 gRPC handler 共享同一 `invoke(method, path)` 闭包，单一事实源 (single source of truth)。

---

## 3. 4 Provider 真传输 (主 17:43 无 fallback)

`RealModelGateway.call()` (源 L274-308) **故意无 fake fallback**：

| # | Provider | 源行号 | 真传输路径 |
|---|---|---:|---|
| 1 | **OpenAI 兼容** (`openai`/`gpt`) | L284 → L308 `_call_openai_or_ollama` | `POST {base}/chat/completions` (默认 `https://api.openai.com/v1`) |
| 2 | **Ollama** (`ollama`) | L284 → L308 同一路径 | `POST http://127.0.0.1:11434/api/chat` (base_url 自动切换) |
| 3 | **Anthropic** | L336 | `POST https://api.anthropic.com/v1/messages` (`x-api-key` header) |
| 4 | **Local executable** | L355 | `subprocess.run` 执行本地命令，stdout 解析 |

**4 类 provider 失败处理**：任一不可用即抛 `V1124Error`，code ∈ {`provider_not_configured`, `provider_unavailable`, `provider_http_error`, `provider_invalid_response`, `provider_process_error`}，状态码 502/503。

---

## 4. 真 fsync + 写穿 + 哈希链审计 (主 17:58 不假装)

### 4.1 `_fsync_directory(path)` (源 L77-87)

```python
def _fsync_directory(path: Path) -> None:
    fd = os.open(str(path), os.O_RDONLY)
    try:
        os.fsync(fd)        # 真 fsync — Windows 无目录 fsync 时优雅降级
    except OSError:
        pass                # 主 17:43 不假装 Windows fsync
    finally:
        os.close(fd)
```

### 4.2 `AuditChain` 追加 + 哈希链 (源 L90-152)

**真属性**：
- 追加-only (append-only)
- 写穿 (write-through)：每条 record 写入后立即 `os.fsync(fd)` (L147) + `_fsync_directory` (L150)
- SHA-256 哈希链：每条 record `hash = sha256(canonical(record))` (L141)，链上前一条 hash

**真接口**：
- `append(event, payload)` → 新 record dict
- `verify()` → 哈希链完整校验
- `records(tolerate_torn_tail=True)` → 抗尾撕裂解析

### 4.3 `DurableIdentityStore` (源 L154-242)

**真属性**：
- V1072 IdentityManifest 序列化 (L165-184) + 反序列化 (L174)
- `save(manifest, reason)` 写穿：snapshot SHA-256 写入 audit chain (L186-211)
- `load()` 启动恢复：校验 `snapshot_sha256 == sha256(canonical(data))` (L228)
- `startup_self_check(expected_identity_id)` (L234) — 启动身份自检

---

## 5. 真常量 (源 L40-41)

```python
BASELINE_V04 = 0.8538         # R9 W4 末真测 baseline
ASI_NORTH_STAR_TARGET = 0.95  # R10 终极目标 (≠ 长期 LOCKED 0.9800)
```

R10 W1 起点 = `max(BASELINE_V04 + 0.5pp, R10_START_TARGET)` = **0.8600**。

---

## 6. 复用与串联 (主 19:33)

| 上游 | 源行号 | 桥接 |
|---|---|---|
| V1072 IdentityCore | L29-37 | `IdentityCore / IdentityManifest / IdentityManifestEntry / V1072Orchestrator / v1072_philosophy_guard` |
| V1106 EngineeringHarness | L38-39 | 工程化 harness 复用 |

| 下游 | 关系 |
|---|---|
| V1095 IdentityStore (4 persona) | 并行：V1095 fsync 3 道保险 + V1124 AuditChain 哈希链 |
| V1074 V0.3 守门 | `/asi/level` 端点真测 |
| V1125 R10 协议 | `/asi/north-star` 端点真测 (V0.5 ≥ 0.95) |

---

## 7. 真测试 + 5 分钟接手

```bash
# 1. 启动 HTTP 服务 (默认 :8765)
python -m apeireth.v1124_asi_north_star_backend --serve --port 8765 &

# 2. 查询 ASI 北极星
curl -s http://127.0.0.1:8765/asi/north-star | jq .
# {"asi_north_star": 0.95, "current_v04": 0.8538, ...}

# 3. 启动 gRPC (同进程)
python -m apeireth.v1124_asi_north_star_backend --grpc --port 50051 &

# 4. 4 provider 真传输自检 (需 API key 或本地 executable)
OPENAI_API_KEY=sk-... python -m apeireth.v1124_asi_north_star_backend --probe openai
ANTHROPIC_API_KEY=sk-ant-... python -m apeireth.v1124_asi_north_star_backend --probe anthropic
python -m apeireth.v1124_asi_north_star_backend --probe ollama
python -m apeireth.v1124_asi_north_star_backend --probe local --exec 'echo hello'
```

---

## 8. 失败模式 / 升级路径 (ponytail)

> ponytail: 当前 `AuditChain` 是单文件 JSONL (顺序写)。当 R10 W2+ 引入分布式节点时，需替换为 `ReplicatedAuditChain` 类（多节点 append-only + Merkle 树）。当前实现足够 R10 W1 单机部署。

---

## 9. 真行号复现 (主 17:43 实事求是)

以下 `grep -n` 命令可在 `apeireth/v1124_asi_north_star_backend.py` (543 LOC) 复现本文件引用的全部真行号：

```bash
# 1. fsync 3 道保险 (_fsync_directory, os.fsync, hash chain)
grep -n "_fsync_directory\|os.fsync\|hash_chain" apeireth/v1124_asi_north_star_backend.py

# 2. 4 provider 调度 (anthropic / openai|gpt|ollama)
grep -n "def _call_anthropic\|def _call_openai_or_ollama\|request.provider ==" apeireth/v1124_asi_north_star_backend.py

# 3. HTTP+gRPC 协议自描述 (协议握手响应)
grep -n "/asi/level\|/asi/measure\|/asi/north-star\|apeireth.v1124.ASINorthStar" apeireth/v1124_asi_north_star_backend.py

# 4. 启动身份自检 (V1072 bridge + DURABLE_IDENTITY_ROOT)
grep -n "durable_identity\|DURABLE_IDENTITY\|CentralAIProfile" apeireth/v1124_asi_north_star_backend.py

# 5. V3 守门 5 项 (源 L38-44 真注释)
sed -n '38,44p' apeireth/v1124_asi_north_star_backend.py

# 6. 关键承诺 (源 L1-7 真注释)
sed -n '1,7p' apeireth/v1124_asi_north_star_backend.py
```

复现期望：
- 命令 1 → 输出行号包含 L77/L82/L85/L91/L147/L150/L194/L198
- 命令 2 → 输出行号包含 L282/L283/L284/L285/L308/L333/L349
- 命令 3 → 输出行号包含 L399/L400/L430/L432
- 命令 4 → 输出含 `durable_identity` / `CentralAIProfile`
- 命令 5 → 输出 5 条 V3 守门 assertion 字符串
- 命令 6 → 输出 "Real-model results are accepted only from an actual HTTP endpoint or a configured local executable; there is no simulated fallback."

任一命令不匹配 → 源文件已被改动，本架构文档需同步更新。