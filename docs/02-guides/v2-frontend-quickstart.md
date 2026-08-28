# v2 Frontend Quickstart (2026-08-28, 子代理 R9 写, 主代理 Mavis 待审)

> **本文档定位**: companion-desktop 对接 v2 gateway 快速集成指南 (估真生产前 4-6 周实施, 2027-Q1 启动).
> **HEAD 状态**: `7d990297` (Round 6 完, A 块 + 4 doc drift fix + SDK 真 bug fix + §8.5 hook + §4.5 术语表). 历史 v2.0.0-rc.1 tag @ `b9026186`.
> **何时写**: 子代理 R9 在 rc.1 收盘后写本指南 + spec (`v2-gateway-frontend-integration-spec.md`).
> **关系文档**: `v2-gateway-frontend-integration-spec.md` (本指南的完整契约).

```
[Document-Meta]
Document:        docs/02-guides/v2-frontend-quickstart.md
Version:         Quickstart-1.0
Last-Modified:   2026-08-28
Status:          🟡 Spec 完成 (真实施 4-6 周, 2027-Q1 启动)
Author:          子代理 R9
```

---

## 1. 前提

- **v2 gateway 0 装**: HEAD `b9026186` 拍板 v2.0.0-rc.1 (per 子代理 R9 必读 #1). 15-crate workspace, 3 路由 (per `canonical_entry.rs:168-174`).
- **9 organ done**: E4/F4/F6/F1/W1/W2/W3/E7/Memory 9 organ trait 抽象 + 真实现 (per `crates/engine/organ/src/lib.rs:11-32`, 整合 #2 commit `bbf70293`).
- **认知模块 6/12 WIRED**: 6 WIRED + 6 DEFERRED (`memory_recall` / `preference_recall` / `judge` / `council` / `self_assessment` / `memory_writeback`, judge/council 为 WIRED, OFF by default; per `cognitive-module-wiring.md:22-35`).
- **companion-desktop 0.5.0**: 当前指向 v1 :8090, 待真生产迁移到 :8080 (per `frontend/companion-desktop/README.md:6-7`).

---

## 2. 快速集成 (3 步)

### 步骤 1: 启动 v2 gateway

```bash
# 根 workspace 启动 v2 gateway
cd C:\Users\31683\Apeireth-rust
cargo run --locked --bin apeireth -- gateway serve --port 8080
# 期望: HTTP 服务监听 :8080, 路由 /health + /v1/chat + /v1/chat/completions

# 或 Docker (待真生产)
docker run -p 8080:8080 -e APEIRETH_LLM_BACKEND=minimax apeireth:2.0.0
```

**验证**:

```bash
curl http://localhost:8080/health
# 期望: {"status":"ok","execution_owner":"apeireth-runtime::canonical"}
```

### 步骤 2: companion-desktop 配置环境变量

```bash
# companion-desktop 启动前, 设置 gateway URL (从 :8090 改 :8080)
export APEIRETH_GATEWAY_URL=http://localhost:8080/v1
# 或 Windows PowerShell
$env:APEIRETH_GATEWAY_URL = "http://localhost:8080/v1"

# 启动 dev server (per `frontend/companion-desktop/README.md:54-57`)
cd frontend/companion-desktop
pnpm install
pnpm dev  # http://localhost:1420
```

**runtime.ts 修改** (待真实施, 估 4-6 周内):

```typescript
// frontend/companion-desktop/src/lib/runtime.ts (待 R11 实施)
// 当前: baseUrl = "http://localhost:8090/v1"  (legacy v1 companion)
// 真生产: baseUrl = process.env.APEIRETH_GATEWAY_URL ?? "http://localhost:8080/v1"
```

### 步骤 3: 真生产 token (API key)

```bash
# v2 gateway 当前无 Authorization header 校验 (per `canonical_entry.rs` 无 auth middleware)
# 真生产估加 bearer token 校验 (估 4-6 周内)

export APEIRETH_API_KEY=sk-apeireth-...  # 真生产凭证
# 或 Windows PowerShell
$env:APEIRETH_API_KEY = "sk-apeireth-..."
```

**真生产授权 header** (OpenAI 兼容):

```http
POST /v1/chat/completions
Authorization: Bearer sk-apeireth-...
Content-Type: application/json
```

---

## 3. 验证 (3 测)

### 3.1 `curl POST /v1/chat/completions` 真接 LLM 1.16s

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "MiniMax-M3",
    "messages": [
      {"role": "system", "content": "You are Apeireth v2.0-rc.1 cognitive companion."},
      {"role": "user", "content": "今天心情如何?"}
    ]
  }'

# 期望响应 (per `canonical_entry.rs:127-156`):
# {
#   "id": "<req_id>",
#   "object": "chat.completion",
#   "created": 1756382400,
#   "model": "MiniMax-M3",
#   "choices": [{
#     "index": 0,
#     "message": {"role": "assistant", "content": "主人, ..."},
#     "finish_reason": "stop"
#   }],
#   "usage": {"prompt_tokens": 42, "completion_tokens": 18, "total_tokens": 60},
#   "apeireth": {
#     "session_id": "01H...",
#     "trace_id": "...",
#     "served_by": "minimax-m3-thinking",
#     "rounds": 1
#   }
# }
```

**真接 LLM 时间**: 估 1.16s (per RC-5 MiniMax adapter 真接, 子代理 D 验证).

### 3.2 `curl POST /v1/audio/transcriptions` 真接 Whisper (估 30-60s)

```bash
# 当前状态: ⏳ RC-7 Perception trait 架构 done (commit `6e918c12`),
# 但 Whisper 真接需硬件 + API key, 估 2-3 周真生产.

curl -X POST http://localhost:8080/v1/audio/transcriptions \
  -H "Authorization: Bearer sk-apeireth-..." \
  -F "file=@recording.wav" \
  -F "model=whisper-1"

# 期望响应: {"text": "今天心情如何?"}
```

**真接 Whisper 时间**: 估 30-60s (per `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` RC-7 估).

### 3.3 `curl GET /v1/models` 返 9 organ + MiniMax M3

```bash
# 当前状态: ⏳ /v1/models 0 装 (per spec §2.4.3), 真生产估加.

curl http://localhost:8080/v1/models

# 期望响应 (本 spec 提案 schema):
# {
#   "object": "list",
#   "data": [
#     {"id": "MiniMax-M3", "object": "model", "created": 1756382400, "owned_by": "MiniMax"},
#     {"id": "minimax-m3-thinking", "object": "model", "created": 1756382400, "owned_by": "MiniMax"}
#   ]
# }
```

**说明**: 9 organ 不暴露为独立 model ID — 9 organ 通过 cognitive module 内部注入 (`cognitive-module-wiring.md:37-43` 注册顺序), frontend 通过 OpenAI Chat 兼容路径自动走 9 organ.

---

## 4. 0 装诚实 (子代理 R9 独立判断)

### 4.1 v2 gateway 9 organ 0 装

- v2 gateway 当前**仅**3 路由非流式 (`canonical_entry.rs:168-174`)
- 9 organ 已 trait 抽象 + 真实现 (`crates/engine/organ/src/lib.rs:11-32`), 但**串联**逻辑 0 装 — OrganOrchestrator 待 R11 实施
- 真生产前 9 organ 必须通过 `OrganOrchestrator` 串联到 runtime + cognitive module 12 slot

### 4.2 真生产前 0 装诱导 prevention

- **不假装 "全做完"**: 本指南+spec 仅完成 spec 范畴, 真实施 4-6 周 (估 2027-Q1 启动)
- **不假装 "frontend 已对接"**: companion-desktop 当前仍指 :8090 (per `frontend/companion-desktop/README.md:6-7`)
- **不假装 "9 organ 集成 OrganOrchestrator"**: 当前 0 装 (per `cognitive-module-wiring.md` + `organ.rs` 串联逻辑 0 装)

### 4.3 真账

- v2.0.0-rc.1 release tag 拍板完成 (HEAD `b9026186`)
- 真生产前阻塞 #2 = frontend 对接, **估 4-6 周, 2027-Q1 启动**
- 真生产前阻塞 2.5/4 完成 (per `v2.0.0-release-path.md:36`)

---

## 5. 详细契约

见 `docs/02-guides/v2-gateway-frontend-integration-spec.md` (本指南的完整 spec).

**章节映射**:
- §2 端点契约 → quickstart §3 验证
- §3 9 organ 集成路径 (L0-L5) → spec 完整, quickstart 不展开
- §4 stream 协议 (SSE / WebSocket) → 估真生产实施
- §5 认知模块集成 (12 slot) → 估真生产实施 OrganOrchestrator
- §6 错误处理 → spec 完整
- §7 安全 (3 governance hook) → spec 完整
- §8 真生产前阻塞 → quickstart §4.3 真账
- §9 部署 checklist → quickstart §2 步骤

---

## 6. 接手人后续真实施 (估 4-6 周)

per spec §9.4 + `frontend/companion-desktop/README.md:111-117` 已知 follow-up:

1. **week 1-2**: runtime.ts 重写指向 :8080 + SSE stream 路径实施
2. **week 2-3**: 9 organ stream hook 集成 (per spec §4.3)
3. **week 3-4**: 认知模块集成 + OrganOrchestrator 实施 (per spec §5)
4. **week 4-5**: 治理 hook HTTP 透传 + 主人审批 UI
5. **week 5-6**: Whisper 真接 (per RC-7) + GET /v1/models + E2E test

**真生产前必跑**:
- `cargo clippy --workspace --all-targets --locked -- -D warnings` (0 警告)
- `cargo test --workspace --locked` (0 FAILED)
- `cargo test -p apeireth-migration --locked` (RC-11 集成测试)
- `pnpm check` (companion-desktop svelte-check, per `frontend/companion-desktop/README.md:67`)
- 5 重守门自动验证 (`.github/workflows/o6-anchor.yml`, per `FINAL-HANDOFF-V2.0.0-RC.1.md:113-121`)

---

**End of Quickstart (v1.0, 子代理 R9 写, 主代理 Mavis 待审)**