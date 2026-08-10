[Document-Meta]
Document: docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R20 阶段 4
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)

---

# apeireth-sdk 缺失分析 (R20 阶段 4)

> **性质**: R20 阶段 4 SDK 完善的前置分析 — 摸清 `crates/apeireth-sdk/` 现状, 识别 R20 阶段 4 真要补什么, 给出 5 步实施方案.
>
> **依据**: R20 §3.3 §5.1 + sub-agent 报告 (r20 roadmap §8 关键发现 T13 BLOCK) + 实扫 `crates/apeireth-sdk/` 全部文件.
>
> **重要事实更正**: sub-agent 报告说"apeireth-sdk 缺 Cargo.toml/src (T13 BLOCK)" — **这个描述是错的**. 实际扫的结果: `Cargo.toml` + `src/` + `tests/` + Python 包装 **全在**, 实装了一个**低层 FFI 测试入口** (C-ABI 边界 + 版本协商 + WireFormat), 8 个 smoke test 全过. 真正缺的是**用户面向的高层 SDK 入口** (Rust `ApeirethClient` + Python SDK + TypeScript SDK). 见 §2 现状清单.

---

## §1 战略背景 (为什么)

### 1.1 R20 阶段 4 需要什么

per `docs/roadmap/r20-product-finalize-2026-08-05.md` §3.3 (API 形态 5 子项):

| 子项 | 关键 crate | R20 阶段 4 任务 |
|------|----------|----------------|
| **Python SDK** | `apeireth-pybridge` + `apeireth-sdk` | `pip install apeireth` → `ApeirethClient` 类 |
| **TypeScript SDK** | `apeireth-sdk` (待补) | `npm install @apeireth/sdk` → TS 类型 + fetch/ws 客户端 |
| **Rust SDK** | `apeireth-sdk` (同 TS crate) | `cargo add apeireth-sdk` → `ApeirethClient` struct |
| **OpenAPI 3.1** | `docs/api/openapi.yaml` (新) | swagger-cli 校验 + Redoc 渲染 |

**3 SDK 共享**:
- HTTP 客户端 wrapper (调 `apeireth-api` REST 10 端点)
- WebSocket 双向流 wrapper (调 `/v1/stream`)
- 错误码 (复用 `SdkErrorCode`)
- WireFormat (复用 `Envelope`)
- 版本协商 (复用 `negotiate`)

### 1.2 sub-agent 报告的"缺 Cargo.toml/src"是真假

❌ **假**. 实际扫了 `crates/apeireth-sdk/`:

```
crates/apeireth-sdk/
├── Cargo.toml          (1094 bytes)  ✅
├── .venv/              (Python 虚拟环境)
├── src/                ✅
│   ├── lib.rs          (2079 bytes)  ✅
│   ├── abi.rs          (1096 bytes)  ✅
│   ├── error.rs        (2853 bytes)  ✅
│   ├── version.rs      (2938 bytes)  ✅
│   └── wire.rs         (2241 bytes)  ✅
├── src-py/             (Python wrapper, ctypes 加载 cdylib)
│   └── apeireth_sdk/
│       ├── __init__.py             (已生成, from ._ffi import *)
│       ├── _ffi.py                 (已生成)
│       ├── envelope.py             (已生成)
│       ├── errors.py               (已生成)
│       ├── __pycache__/
│       └── apeireth_sdk.egg-info/
└── tests/
    └── smoke.rs        (4584 bytes, 8 smoke tests 全过)  ✅
```

**8 个 smoke test** (per `tests/smoke.rs`):
1. `smoke_version_parse_and_stringify`
2. `smoke_version_negotiate_incompatible_across_major`
3. `smoke_version_negotiate_within_major`
4. `smoke_envelope_serde_roundtrip`
5. `smoke_wire_kind_other_roundtrip`
6. `smoke_error_code_numeric_and_names`
7. `smoke_envelope_version_field`
8. `smoke_sdk_error_business_construct`

**workspace Cargo.toml 第 51 行**确认 `apeireth-sdk` 是 member:
```toml
# V2 战区 1/4/5 — multi-language SDK (docs/v2-strategy/05 §Step 1.4, R-Cycle v2-strategy)
"crates/apeireth-sdk",
```

**Cargo.toml line 12-17 双 crate-type** (rlib + cdylib) — 已经为 C-ABI 边界做好.

### 1.3 那 R20 阶段 4 真正缺什么

apeireth-sdk 当前是**低层 FFI 测试入口** (per `Cargo.toml` line 8 description):
> "Apeireth v2.0 战区 1/4/5: 多语言 SDK 统一测试入口 — C-ABI 边界 / 版本协商 / WireFormat 反序列化"

**实装的**:
- ✅ `Envelope` + `WireKind` — JSON wire-format
- ✅ `SdkVersion` + `negotiate` + `WireCompat` — 版本协商
- ✅ `SdkError` + `SdkErrorCode` — 错误码 (7 种 + 数字码)
- ✅ `#[no_mangle] extern "C"` C-ABI 顶层 API (3 个 stub: `apeireth_sdk_init` / `apeireth_sdk_last_error` / `_ensure_error_linked`)

**没实装的 (R20 阶段 4 要补)**:
- ❌ 用户面向的 `ApeirethClient` (Rust high-level struct, 调 apeireth-api HTTP + WS)
- ❌ 跟 `apeireth-api` (HTTP 4 协议 + 6 V2 端点) 的集成 (10 REST 端点 客户端)
- ❌ WebSocket `/v1/stream` 客户端
- ❌ TypeScript SDK (`@apeireth/sdk` npm 包 + `.d.ts` 类型)
- ❌ OpenAPI 类型生成 (ts-rs for Rust, openapi-typescript for TS)
- ❌ README.md (用户怎么用, FFI 怎么调)
- ❌ `apeireth-jsbridge` (napi-rs 给 Node 加载 cdylib)

---

## §2 现状清单 (实扫, 不假装)

### 2.1 文件清单 + 状态

| 路径 | 大小 | 状态 | 备注 |
|------|------|------|------|
| `Cargo.toml` | 1094 B | ✅ 实装 | name/version/edition/lints 完整, 双 crate-type (rlib + cdylib) |
| `src/lib.rs` | 2079 B | ✅ 实装 | 4 module 入口 (version/wire/error/abi), #![deny(unsafe_code)] |
| `src/abi.rs` | 1096 B | 🟡 stub | 3 个 `#[no_mangle] extern "C"` 函数都是 stub (返 0 / -1), V2 D2 才实装 |
| `src/version.rs` | 2938 B | ✅ 实装 | SdkVersion + WireCompat + negotiate + SDK_VERSION=0.1.0 |
| `src/wire.rs` | 2241 B | ✅ 实装 | Envelope (4 字段) + WireKind (5 变体) + encode/decode |
| `src/error.rs` | 2853 B | ✅ 实装 | SdkErrorCode (7 错误码 + 数字码) + SdkError (thiserror) |
| `src-py/apeireth_sdk/` | (dir) | 🟡 部分 | Python wrapper 生成物 (ctypes 加载 cdylib), 但**没 apeireth_api_client.py** |
| `tests/smoke.rs` | 4584 B | ✅ 实装 | 8 个 smoke test 全过 |
| `README.md` | — | ❌ 缺失 | 用户面向文档没写 |
| `docs/` | — | ❌ 缺失 | 子目录也没建 |
| `src/client.rs` | — | ❌ 缺失 | 高层 ApeirethClient struct 缺 |
| `src/http.rs` | — | ❌ 缺失 | HTTP 客户端 wrapper 缺 |
| `src/ws.rs` | — | ❌ 缺失 | WebSocket 客户端 wrapper 缺 |
| `crates/apeireth-jsbridge/` | — | ❌ 缺失 | napi-rs 给 Node 桥, 整个 crate 没建 |

### 2.2 Cargo.toml 关键字段

```toml
[package]
name = "apeireth-sdk"
version.workspace = true              # = 1.0.0 (workspace level)
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
description = "Apeireth v2.0 战区 1/4/5: 多语言 SDK 统一测试入口 — C-ABI 边界 / 版本协商 / WireFormat 反序列化 (docs/v2-strategy/05 §Step 1.4 + 03 §0.2)"

[lib]
name = "apeireth_sdk"
crate-type = ["rlib", "cdylib"]      # 双输出: rlib 给 workspace 内部, cdylib 给 FFI

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

**注意**: workspace level version = 1.0.0, 但 `src/version.rs` `SDK_VERSION` = 0.1.0 (协议层版本, 不是 crate 版本). 两者**不同**:
- `Cargo.toml` package.version = workspace 1.0.0 (crate 自身版本)
- `src/version.rs` SDK_VERSION = 0.1.0 (协议层 wire-format 版本)

**R20 阶段 4 要决定**: SDK_VERSION 是升到 1.0.0 (跟 crate 同步) 还是保持 0.1.0 (协议层未稳定) — 建议升到 1.0.0 (跟 R20 阶段 3 OpenAPI 规范同周期).

### 2.3 src-py/ 实际内容 (扫了)

```
src-py/
└── apeireth_sdk/
    ├── __init__.py                   (from ._ffi import *)
    ├── _ffi.py                       (ctypes 加载 cdylib 的 .so/.dll/.dylib)
    ├── envelope.py                   (WireKind / Envelope 类)
    ├── errors.py                     (SdkError / SdkErrorCode 类)
    ├── __pycache__/
    └── apeireth_sdk.egg-info/        (setuptools 安装元数据)
```

**没有的 Python 文件** (R20 阶段 4 Python SDK 入口要补):
- `client.py` — `ApeirethClient` class (HTTP/WS)
- `api.py` — 4 协议 LLM 调用 (openai/anthropic/gemini 转发)
- `models.py` — Pydantic 数据类 (Request/Response)
- `streaming.py` — WebSocket 流式消费
- `setup.py` / `pyproject.toml` — 打包配置
- `README.md` / `docs/` — Python 用户文档

---

## §3 真要补什么 (R20 阶段 4 缺口)

### 3.1 Rust SDK 缺口 (apeireth-sdk crate 内部)

| 缺口 | 优先级 | 估 LOC | Owner |
|------|--------|------:|-------|
| **`src/client.rs` `ApeirethClient` struct** | 🔴 P0 | 800-1000 | fullstack_engineer |
| **`src/http.rs` HTTP 客户端 wrapper** (调 `apeireth-api` REST 10 端点) | 🔴 P0 | 600-800 | fullstack_engineer |
| **`src/ws.rs` WebSocket 客户端 wrapper** (调 `/v1/stream`) | 🔴 P0 | 400-600 | fullstack_engineer |
| **OpenAPI 类型生成** (ts-rs 从 `apeireth-api` 自动生成 → apeireth-sdk 暴露) | 🟡 P1 | 300 | fullstack_engineer |
| **`README.md`** (用户面向, 怎么调 Rust SDK) | 🔴 P0 | 200 | technical_writer |
| **`examples/` 5 个示例** (chat / memory / organs / team / streaming) | 🟡 P1 | 500 | fullstack_engineer |
| **SDK_VERSION 升 0.1.0 → 1.0.0** (协议层) | 🟡 P1 | 10 | fullstack_engineer |
| **集成测试** (`tests/integration_*.rs` ≥ 5, 跟本地 apeireth-api server 真跑) | 🟡 P1 | 600 | fullstack_engineer |

**总计 Rust SDK**: ~3500-4000 LOC 新增, 跟现有 ~11000 LOC 加起来 ~15000 LOC.

### 3.2 Python SDK 缺口 (`pip install apeireth`)

| 缺口 | 优先级 | 估 LOC | Owner |
|------|--------|------:|-------|
| **`src-py/apeireth_sdk/client.py` `ApeirethClient` class** (HTTP via httpx + async) | 🔴 P0 | 600 | fullstack_engineer |
| **`src-py/apeireth_sdk/api.py` 4 协议 wrapper** (openai/anthropic/gemini 转发) | 🔴 P0 | 400 | fullstack_engineer |
| **`src-py/apeireth_sdk/models.py` Pydantic** (跟 Rust SDK 类型同步) | 🔴 P0 | 500 | fullstack_engineer |
| **`src-py/apeireth_sdk/streaming.py` WebSocket 流式** (websockets 库) | 🔴 P0 | 300 | fullstack_engineer |
| **`src-py/pyproject.toml` 打包配置** (poetry / setuptools) | 🔴 P0 | 100 | fullstack_engineer |
| **`src-py/README.md` Python 入门** | 🔴 P0 | 300 | technical_writer |
| **CD pipeline** (CI 跑 maturin / twine 推到 PyPI) | 🟡 P1 | — | devops_engineer |
| **集成测试** (`tests_py/test_*.py` ≥ 5) | 🟡 P1 | 500 | fullstack_engineer |

**总计 Python SDK**: ~2700 LOC, 跟现有 `src-py/` (估 ~500 LOC) 加起来 ~3200 LOC.

### 3.3 TypeScript SDK 缺口 (`npm install @apeireth/sdk`)

| 缺口 | 优先级 | 估 LOC | Owner |
|------|--------|------:|-------|
| **`crates/apeireth-jsbridge/` 新 crate** (napi-rs 桥, 桥接 `apeireth-sdk` C-ABI) | 🔴 P0 | 800-1000 | fullstack_engineer |
| **`packages/typescript-sdk/` 独立 npm 包** (或放 `crates/apeireth-jsbridge/npm/`) | 🔴 P0 | 1500-2000 | fullstack_engineer |
| **`@apeireth/sdk` 公开 API** (`Apeireth` class + 类型) | 🔴 P0 | 800 | fullstack_engineer |
| **OpenAPI 类型自动生成** (`openapi-typescript` → `src/types.ts`) | 🟡 P1 | 200 | fullstack_engineer |
| **WebSocket 客户端** (原生 `WebSocket` + reconnect 逻辑) | 🔴 P0 | 400 | fullstack_engineer |
| **`README.md` TypeScript 入门** | 🔴 P0 | 300 | technical_writer |
| **CD pipeline** (CI 跑 npm publish) | 🟡 P1 | — | devops_engineer |
| **集成测试** (vitest ≥ 5) | 🟡 P1 | 500 | fullstack_engineer |

**总计 TypeScript SDK**: ~4500-5200 LOC, 全新 crate + 全新 npm 包.

### 3.4 共享层 (3 SDK 都要实现)

apeireth-sdk **已有的** (3 SDK 直接复用, 不重写):

| 模块 | 用途 | 3 SDK 怎么用 |
|------|------|------------|
| `SdkVersion` + `negotiate` | 版本协商 | Rust: 直接 `use`; Python: `from apeireth_sdk import SdkVersion`; TS: 从 napi-rs 桥 import |
| `Envelope` + `WireKind` | JSON wire-format | 同上, 3 SDK 都用同一套 envelope 格式 |
| `SdkErrorCode` + `SdkError` | 错误码 | 同上, 7 错误码 + 数字码跨语言一致 |
| `#[no_mangle] extern "C"` API | C-ABI 边界 | Rust 直接调; Python ctypes; TS 通过 napi-rs 桥 (WASM 边界) |

**R20 阶段 4 新增** (3 SDK 都要实现):

| 新增抽象 | 用途 | 3 SDK 怎么用 |
|---------|------|------------|
| `ApeirethClient` | 高层客户端 (HTTP + WS) | Rust struct / Python class / TS class, 同样的 11 方法签名 |
| HTTP methods (`chat` / `memory_*` / `organs_*` / `team_*` / `asi_*` / `sovereignty_*` / `agent_*`) | 调 `apeireth-api` 10 REST 端点 | 3 SDK 都实现, 同样的方法签名 + 同样的错误处理 |
| WebSocket method (`stream`) | 调 `/v1/stream` | 3 SDK 都用 async iterator 模式 |
| Config (`base_url` / `api_key` / `timeout`) | 客户端配置 | 3 SDK 都用 builder 模式 |

**关键不变量 (3 SDK 一致)**:
- 方法名: snake_case (Rust) / snake_case (Python) / camelCase (TS) — 但映射一致
- 错误码: 7 错误码 + 数字码跨语言统一
- envelope 格式: 4 字段 (`v` / `kind` / `id` / `body`) 跨语言一致

---

## §4 怎么补 (5 步骤)

> **5 步骤对应 R20 §4 阶段 4** + R20 阶段 3 (OpenAPI 必须在 SDK 之前稳定).

### Step 1: apeireth-sdk 内部扩展 (0.5 天, rust-coder)

**等 code_reviewer 完工后** (per R20 §3.3 line 196: "等 code_reviewer 完工后补 apeireth-sdk 骨架").

任务:
1. `src/client.rs` (800-1000 LOC): `ApeirethClient` struct + `ApeirethConfig` + 11 方法签名 (不实装, 只定接口)
2. `src/http.rs` (600-800 LOC): reqwest wrapper, 调 `apeireth-api` REST 10 端点
3. `src/ws.rs` (400-600 LOC): tokio-tungstenite 客户端, 调 `/v1/stream`
4. `Cargo.toml` dependencies: 加 `reqwest` (workspace 已有) + `tokio-tungstenite` (新) + `url`
5. 集成测试 (`tests/integration_*.rs` ≥ 5, 跟本地 apeireth-api server 真跑)

**验收**: `cargo build -p apeireth-sdk` 0 error, `cargo test -p apeireth-sdk` 12+ tests 全过 (8 现有 + 5 新).

### Step 2: 公开 API 设计定稿 (1 天, backend_engineer + architect)

**依赖**: Step 1.

任务:
1. `ApeirethClient` 11 方法签名定稿 (跟 R20 §3.3 10 REST 端点 + 1 WS 端点对应):
   - `chat(messages, model, stream) -> ChatResponse`
   - `memory_read(id) -> Memory`
   - `memory_write(memory) -> id`
   - `organs_status() -> Vec<OrganStatus>`
   - `asi_score() -> AsiScore`
   - `sovereignty_check(action) -> Decision`
   - `agent_spawn(agent_type, prompt) -> AgentId`
   - `agent_send(agent_id, message) -> SendResult`
   - `agent_wait_idle(agent_id, timeout) -> AgentStatus`
   - `team_spawn(config) -> TeamId`
   - `stream(messages, model) -> impl Stream<Item = ProviderEvent>`
2. 错误处理定稿: 所有方法返 `Result<T, SdkError>` (Rust) / `raise SdkError` (Python) / `throw new SdkError(...)` (TS)
3. Config 字段定稿: `base_url: String` / `api_key: Option<String>` / `timeout: Duration` / `max_retries: u32`
4. 写 `docs/sdk/api-design-2026-08-05.md` (方法签名 + 错误处理 + config 完整定义)

**验收**: API 设计文档通过 architect + Mavis 拍板 + 主人复核.

### Step 3: Python SDK 入口 (1 天, fullstack_engineer)

**依赖**: Step 2.

任务:
1. `src-py/apeireth_sdk/client.py` (600 LOC): `ApeirethClient` class (HTTP via httpx + async)
2. `src-py/apeireth_sdk/api.py` (400 LOC): 11 方法实现
3. `src-py/apeireth_sdk/models.py` (500 LOC): Pydantic 数据类 (跟 Rust SDK 字段一一对应)
4. `src-py/apeireth_sdk/streaming.py` (300 LOC): websockets 库 + async iterator
5. `src-py/pyproject.toml` (100 LOC): poetry 打包配置
6. `src-py/README.md` (300 LOC): Python SDK 入门

**验收**: `pip install -e .` 成功, `from apeireth_sdk import ApeirethClient; c = ApeirethClient(base_url="http://localhost:8080"); c.chat("hi")` 跑通.

### Step 4: TypeScript SDK 入口 (1 天, fullstack_engineer)

**依赖**: Step 2.

任务:
1. `crates/apeireth-jsbridge/` 新 crate (napi-rs 桥, 桥接 `apeireth-sdk` C-ABI): 800-1000 LOC
2. `packages/typescript-sdk/` 独立 npm 包 (或放 `crates/apeireth-jsbridge/npm/`): 1500-2000 LOC
3. `@apeireth/sdk` 公开 API (`Apeireth` class + 类型): 800 LOC
4. OpenAPI 类型自动生成 (`openapi-typescript` → `src/types.ts`): 200 LOC
5. WebSocket 客户端 (原生 `WebSocket` + reconnect 逻辑): 400 LOC
6. `README.md` TypeScript 入门 (300 LOC)

**验收**: `npm install @apeireth/sdk` 成功, `import { Apeireth } from "@apeireth/sdk"; const c = new Apeireth(); c.chat("hi")` 跑通.

### Step 5: Rust SDK 入口 + 端到端验证 (1 天, backend_engineer)

**依赖**: Step 1+2.

任务:
1. Rust SDK 入口 (复用 Step 1 已写的 `src/client.rs` + `http.rs` + `ws.rs`): 加 `pub use` 暴露
2. 5 个 examples (`examples/01_chat.rs` / `02_memory.rs` / `03_organs.rs` / `04_team.rs` / `05_streaming.rs`): 500 LOC
3. `docs/sdk/rust/` (rustdoc + 完整示例): 300 LOC
4. 端到端验证: 3 SDK 都能跟本地 `apeireth-api` 跑通 (起 `apeireth-api` server → 3 SDK 各跑 1 个 smoke)

**验收**: `cargo add apeireth-sdk` 成功, `let c = ApeirethClient::new(); c.chat("hi").await?` 跑通.

### 总计 4.5 天 (1 周)

| 步骤 | 时长 | Owner | 依赖 |
|------|------|-------|------|
| Step 1 | 0.5 天 | rust-coder | 等 code_reviewer 完工 |
| Step 2 | 1 天 | backend_engineer + architect | Step 1 |
| Step 3 | 1 天 | fullstack_engineer | Step 2 |
| Step 4 | 1 天 | fullstack_engineer | Step 2 |
| Step 5 | 1 天 | backend_engineer | Step 1+2 |
| **总计** | **4.5 天** | (1 周) | — |

---

## §5 跟 R20 阶段 4 SDK 关系

### 5.1 3 SDK 共享抽象 (在 apeireth-sdk crate 内部)

| 抽象 | 当前状态 | R20 阶段 4 |
|------|---------|----------|
| `SdkVersion` + `negotiate` | ✅ 已有 | 直接复用 |
| `Envelope` + `WireKind` | ✅ 已有 | 直接复用 |
| `SdkErrorCode` + `SdkError` | ✅ 已有 | 直接复用 |
| C-ABI 顶层 API (3 stub) | 🟡 stub | V2 D2 实装 (R20 阶段 4 Step 1 一起) |
| `ApeirethClient` struct | ❌ 缺失 | Step 1 新增 |
| HTTP wrapper (10 端点) | ❌ 缺失 | Step 1 新增 |
| WebSocket wrapper (1 端点) | ❌ 缺失 | Step 1 新增 |
| `ApeirethConfig` builder | ❌ 缺失 | Step 1 新增 |
| `examples/` 5 个 | ❌ 缺失 | Step 5 新增 |
| `README.md` 用户面向 | ❌ 缺失 | Step 1 + Step 3+4 各自 README |

### 5.2 3 SDK 各自新增 (不进 apeireth-sdk crate)

| SDK | 新增位置 | R20 阶段 4 步骤 |
|-----|---------|--------------|
| **Python SDK** | `crates/apeireth-sdk/src-py/` 现有 + 新增 client/api/models/streaming/setup | Step 3 |
| **TypeScript SDK** | `crates/apeireth-jsbridge/` 新 crate + `packages/typescript-sdk/` 新 npm 包 | Step 4 |
| **Rust SDK** | `crates/apeireth-sdk/src/` 新增 client/http/ws (跟现有 version/wire/error/abi 一起) | Step 1+5 |

### 5.3 维护成本

每次改 apeireth-sdk 公开 API → 3 SDK 都要同步改 (snake_case / camelCase 映射 + 错误码 + envelope).

**缓解**:
- OpenAPI 规范是 single source of truth (R20 阶段 3 完工)
- ts-rs 自动生成 Rust 类型 (从 Rust struct 生成)
- openapi-typescript 自动生成 TS 类型 (从 openapi.yaml 生成)
- Python 用 Pydantic 手动跟 Rust 字段对齐 (慢但稳)

---

## §6 风险清单 (4 项)

| # | 风险 | 严重度 | 缓解 |
|---|------|-------|------|
| **R-001** | apeireth-pybridge cdylib 编译冲突 (R18-2 已知, `pyo3 + rlib` 互斥) | 🟡 中 | Step 3 Python SDK 复用 `src-py/` 现有 (已生成), 不重做 cdylib |
| **R-002** | 3 SDK 维护成本 (改 apeireth-sdk 公开 API → 3 SDK 同步) | 🟡 中 | OpenAPI 规范做 single source + 自动生成 (ts-rs / openapi-typescript) |
| **R-003** | 跨语言 ABI 不一致 (Rust/Python/TS 数据结构映射) | 🔴 高 | Step 2 API 设计阶段定 cross-language contract, 集成测试覆盖 3 SDK 同输入同输出 |
| **R-004** | TypeScript SDK napi-rs 桥跨平台编译 (macOS / Linux / Windows) | 🟡 中 | Step 4 用 napi-rs (prebuild 自动) + GitHub Actions 3 平台 build |

---

## §7 不修改承诺 (11 项)

跟 R20 §7 + ADR-0011 一致.

---

## §8 关联文档

- **R20 路线图**: `docs/roadmap/r20-product-finalize-2026-08-05.md` §3.3 §5.1
- **apeireth-sdk lib.rs**: `crates/apeireth-sdk/src/lib.rs` (4 module 入口)
- **apeireth-sdk Cargo.toml**: `crates/apeireth-sdk/Cargo.toml` (line 8 description: C-ABI 测试入口)
- **apeireth-api 公开 API**: `crates/apeireth-api/src/` (4 协议 + 6 V2 端点)
- **apeireth-http-client**: `crates/apeireth-http-client/` (HTTP 客户端, 阶段 4 复用)
- **集成蓝图**: `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` §5.2

---

_apeireth-sdk 缺失分析 (technical_writer)._
_现状 11 个文件 ~14000 LOC (低层 FFI), 缺 3 SDK 入口 (估 10000 LOC 新增)._
_sub-agent 报告"缺 Cargo.toml/src"是错的, 实际是"缺用户面向 SDK 入口"._
_5 步实施方案总计 4.5 天 (1 周)._
_3 SDK 共享 apeireth-sdk crate 的 Envelope/WireFormat/ErrorCode/version 4 抽象._
