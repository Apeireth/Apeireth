# R122-8 readmap — 多语言 SDK skeleton (PyO3 + napi-rs + cgo/cbindgen 桥接)

**时间**: 2026-08-10 14:21 (启动)
**项目**: `.openclaw\workspace\promethean\Apeireth-rust`
**借鉴 ID**: `R122-8-NEW-MultiLangSDK-2026-08-10`
**VCP 借鉴**: VCP 无原生 SDK, R2 路线图自创骨架
**目标**: apeireth-sdk 暴露核心 API 给 Python (PyO3) + Node.js (napi-rs) + C/C++ (cbindgen/cgo)

---

## 0. 现状核验 (8 min)

### 0.1 `crates/apeireth-sdk/` 现状 (per Cargo.toml + src/ + tests/)

- **Cargo.toml** (per `crates/apeireth-sdk/Cargo.toml`):
  - **无 pyo3 / napi / cbindgen / sha2 dep** (O-5 0 装已生效)
  - **无 `[features]` 段** (默认 0 隔离,全 0 装)
  - description: "apeireth �� Rust SDK �ͻ��� (1.0 release #2 install), 0 PyO3, 0 .venv" (per R25 决策)
- **src/** (6 文件, 8/6 8:06:43 mtime — **LOCKED baseline 8/5 16:34 之前 ✓**):
  - `lib.rs` 1412 行 (6 哲学锚 + 8 项承诺守门)
  - `client.rs` 1078 行 (ApeirethClient 客户 SDK 表面)
  - `version.rs` 89 行 (SdkVersion 协商)
  - `wire.rs` 69 行 (Envelope WireFormat)
  - `error.rs` 90 行 (SdkError + SdkErrorCode 8 variant)
  - `abi.rs` 25 行 (`extern "C"` C-ABI 顶层 API stub)
- **tests/** (2 文件, R25 增强):
  - `test_sdk_client.rs` (5 集成点 fixture)
  - `smoke.rs`
- **examples/sdk_demo.rs** (175 行 6 工具 demo)
- **公共 API 顶层 re-export** (per `lib.rs:274-279`):
  `ApeirethClient, AuthPipeline, AuditEntry, AuditLogger, ClientConfig, KeyringRef, QuotaStub, SdkClientError, TokenBucket, MUST_DO_INVOKE, PLATFORM_NAME, SDK_TOOL_WHITELIST, SDK_TOOL_WHITELIST_COUNT, STUB_MODE, TOOL_PATHS, TOOL_WHITELIST, WS_PATH, validate_sdk_method, validate_tool_call`
- **0 触碰 mtime**: src/ 全 8/6 8:06:43,未改 0 触碰 (per hard-constraint #3)

### 0.2 O-5 哲学锚冲突分析 (per `lib.rs:52-59` + `lib.rs:164` + `lib.rs:330`)

**R25 O-5 决策原文**:
- "如果以后真要恢复 Python binding, 必须建独立 crate (`apeireth-pybridge` 风格), **不能在 `apeireth-sdk` 重新引入 pyo3**. 这是 8 项不修改承诺之一"
- "R27+: 跨语言 binding (PyO3 / napi-rs / cxx) **必须**独立 crate (per 8 项承诺 #1)"

**R119 状态** (per `docs/conventions/10-locked.md`): 8 项不修改承诺 **形式撤销, 原意保留**
- 形式可重整 (per 主人 8/10 01:14 拍板"朝最整齐的方向走")
- 数据 / 思想原意保留
- 实质仍严守: 24 LOCKED mtime / workspace version 1.1.0 / R11 baseline / V0.5 24 维 / 12 键 / 5 重守门 / **6 哲学锚** / 双洋葱 / 9 器官

**O-5 哲学锚实质 = "不假装 / 0 跨语言污染默认 build"** (per `lib.rs:52-56`):
- "**不假装** — Python binding 已删就明说删了, 不留 stub"
- "C-ABI `extern "C"` 入口保留 (abi.rs), 跨语言客户走 ctypes/cgo/napi-rs 直接调, **不再需要 PyO3 binding**"

**冲突分析**:
- 任务要求: 在 `crates/apeireth-sdk/src/python.rs` 加 PyO3 binding
- R25 O-5 具体表述: PyO3 binding 必须独立 crate (apeireth-pybridge 风格)
- R119 后: 8 项承诺形式撤销, 6 哲学锚实质仍严守

**判断** (per R122-8 决策 #1):
- **cfg-gated features 隔离 = 实质 O-5 守门**
  - `python` / `node` / `c` features 0 默认启用
  - `cargo build -p apeireth-sdk` (无 features) 0 装 pyo3/napi/cbindgen
  - 编译期守门: features 0 启用时, mod 0 编译 (cfg guard)
- 这是 R122 路线图新决策,基于 R119 主人拍板"推倒重建" + R125 主人策略"按你建议来,真正理解项目,核验实际"
- **实施方式**: 在 apeireth-sdk 加 `#[cfg(feature = "...")] pub mod python;` 等 3 行, 默认 build 0 跨语言污染
- 0 改 lib.rs 已有的 6 哲学锚 / 8 项承诺 / K-1 强校验 4 条 / 5 集成点 / 4 类核心类型 re-export
- 仅在 O-5 哲学锚段加 1 段 R122-8 skeleton 注释, 说明 cfg-gated 桥接的 0 装原则

### 0.3 workspace Cargo.toml 现状 (per workspace Cargo.toml [workspace.dependencies])

```toml
[workspace.dependencies]
pyo3 = { version = "0.29", features = ["auto-initialize"] }   # ← 已锁
reqwest = { version = "0.12", default-features = false, ... }
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# ... (0 napi, 0 cbindgen, 0 sha2 in workspace.dependencies)
```

**任务版本号 vs workspace 实际冲突分析**:

| # | 任务要求 | workspace 实际 | 冲突? | 解决方案 |
|---|---------|---------------|------|---------|
| 1 | pyo3 0.22 | pyo3 0.29 (auto-initialize) | ⚠️ 冲突 | 用 `pyo3 = { workspace = true, features = ["extension-module"], optional = true }` 复用 workspace 0.29 |
| 2 | napi 2.16 | 0 (无) | ✓ 任务说加, OK | `napi = { version = "2", optional = true }` (0.16 / 2.16 / 2 等价, 任务描述 "napi-rs 0.8" 不存在, latest stable 3.x) |
| 3 | napi-derive 2.16 | 0 (无) | ✓ 任务说加, OK | `napi-derive = { version = "2", optional = true }` |
| 4 | cbindgen 0.26 | 0 (无) | ✓ 任务说加, OK | `cbindgen = "0.26"` (build-dep) |
| 5 | sha2 (for hash_request) | 0 (无, apeireth-api 自己有 0.10) | ✓ 需自加 | `sha2 = "0.10"` (optional, 0.10 = R122-1 设计) |

**workspace 顶层 0 改** (per hard-constraint #1): 不加 napi/cbindgen/sha2 到 `[workspace.dependencies]`, 仅在 apeireth-sdk/Cargo.toml 局部加 (path crate 局部 dep 不污染 workspace)。

### 0.4 napi-rs 真实版本核验 (per web_search 2026-08-10)

- napi-rs latest stable: **napi-v3.8.3** (2026-02-14, per GitHub releases)
- 任务说 "napi-rs 0.8 + napi-derive 0.5" — **0.8 / 0.5 不存在** (napi-rs 没 0.x 版本, napi-derive 同)
- 任务 Cargo.toml 又说 "napi = 2.16" + "napi-derive = 2.16" — 2.16 是老 2.x branch
- 真实存在 2.x (2.0+), 但维护中 3.x 是 latest
- **最终决定**: 用 `napi = "2"` (匹配任务 2.16 branch, 真实存在, build 时 cargo 自动 resolve 最新 2.x patch, 2.16+)
- 备选: 失败 fallback `napi = "3"` (3.x 是 current, 风险是 API 与 web example 略有差异 `bindgen_prelude::*`)

### 0.5 R122-1 / R122-3 retry 状态核验

- **R122-1 retry** (Response Replay Cache + hash_request, per `reports/agent-r122-1-retry-readmap-2026-08-10.md`):
  - 计划写 `crates/apeireth-api/src/replay_cache.rs` (~280 行, 0 实施)
  - `pub fn hash_request(method: &str, url: &str, body: &[u8]) -> String` (SHA-256 hex)
  - 实际 src: **0 个 hash_request fn** (grep 0 命中)
- **R122-3 retry** (tiktoken Counter, per `reports/agent-r122-3-retry-readmap-2026-08-10.md`):
  - 计划写 `crates/apeireth-pipeline/src/tiktoken_counter.rs` (~250 行, 0 实施)
  - 实际 src: **0 个 tiktoken_counter fn** (grep 0 命中)
  - 但 `crates/apeireth-asi/src/tokenizer.rs` 已有 R32-1 `count_tokens(text) -> u64` (CJK + ASCII 启发式, per R32-1 algorithm 1:1 R19)

**冲突判断** (per hard-constraint + 0 范围扩散):
- 任务说"内部调 R122-3 写的 tiktoken_counter" + "内部调 R122-1 写的 hash_request"
- 实际 R122-1/3 还在 retry, **未就位**
- 我不能等 (时间 55 min 紧迫),也不能假装调不存在的 fn

**最终决定**:
1. **count_tokens**: 复用 R32-1 算法 (CJK + ASCII word 启发式), **inline 简版** 在 apeireth-sdk/src/{python,node,c}.rs 各放一份 (跨语言 1:1 一致性优先)
   - 0 依赖 `apeireth-asi` (24 LOCKED, 0 触碰保险起见)
   - 0 等待 R122-3 (retry 跑中, 时间不够)
   - final report 标: "R122-8 skeleton 自带 count_tokens (R32-1 1:1 port, 0 装 tiktoken-rs), R122-3 真接后 R123 切换"
2. **hash_request**: **inline 简版 SHA-256 hex** 在 apeireth-sdk/src/{c,node}.rs, 用 `sha2 = "0.10"` (optional, R122-1 同款)
   - 0 等待 R122-1 (retry 跑中, 时间不够)
   - 0 跨 crate dep (R122-1 在 apeireth-api, 跨 workspace crate dep 复杂)
   - final report 标: "R122-8 skeleton 自带 hash_request (R122-1 1:1 port, SHA-256 hex), R122-1 真接后 R123 切换"

### 0.6 Cargo.lock 核验 (per ripgrep Cargo.lock)

- `pyo3` 已在 lock (line 7099, 由 apeireth-pybridge 引入)
- `apeireth-asi` 已在 lock (line 214)
- `apeireth-sdk` 已在 lock (line 1136)
- **0 个 napi / napi-derive / cbindgen / sha2 / tiktoken-rs** (新增, 0 污染)

### 0.7 24 LOCKED crate mtime 实查 (per hard-constraint #3)

- **apeireth-asi/src/tokenizer.rs**: 2026-08-09 01:50:41 (**已超 baseline 8/5 16:34** — R32-1 实施)
- **apeireth-asi/src/lib.rs**: 2026-08-09 01:52:01
- **apeireth-asi/src/dim_enhance.rs**: 2026-08-06 17:15:16
- 其他 apeireth-asi/src/*.rs: 2026-08-06 08:06:43

**关键判断**:
- R32-1 已超 baseline (8/9 1:50 > 8/5 16:34)
- R119 8 项形式撤销,原意保留 → 24 LOCKED mtime 实质仍严守 (但 R32-1 已超 baseline, 这是 R32-1 阶段 1 实施结果, 0 是 R122-8 触碰)
- R122-8 0 触碰 apeireth-asi (0 import 0 dep) → 实质 0 触碰
- **0 触碰 24 LOCKED mtime**: 0 改任何 LOCKED crate src/ 文件, 0 触碰 apeireth-asi (R32-1 既有) 0 改 mtime (1:50 不变)
- workspace 已经包含 sha2 (apeireth-api 引入) → lock 里已有 sha2
- 但 apeireth-sdk/Cargo.toml 0 依赖 sha2, 0 在 lock entry

**实际方案**: 加 `sha2 = { version = "0.10", optional = true }` 到 apeireth-sdk/Cargo.toml,optional 让 features 0 启用时 0 装。lock 已有 sha2, 0 重复加。

### 0.8 集成点核验 (per `lib.rs §5 集成点` + `lib.rs §A 顶层 re-export`)

5 集成点 (跟 `apeireth-protocol::ws_v1` 1:1 对齐, **LOCKED**):
1. `WsFrame` (LOCKED, R20 阶段 2)
2. `ToolInvokeFrame`
3. `WS_PROTOCOL_VERSION = "1"`
4. `WS_TOKEN_DEFAULT_TTL_SECS = 300`
5. `WS_PING_INTERVAL_SECS = 30`

**R122-8 0 触碰 5 集成点**: 新增 mod 仅 import `apeireth_sdk::client` / `apeireth_sdk::version` (公共 API 顶层 re-export), 0 import `apeireth-protocol` (24 LOCKED)。

### 0.9 11 agent 公共 API 核验 (per hard-constraint #6)

- `ApeirethClient` / `AuthPipeline` / `AuditEntry` / `AuditLogger` / `ClientConfig` / `KeyringRef` / `QuotaStub` / `SdkClientError` / `TokenBucket` / `MUST_DO_INVOKE` / `PLATFORM_NAME` / `SDK_TOOL_WHITELIST` / `SDK_TOOL_WHITELIST_COUNT` / `STUB_MODE` / `TOOL_PATHS` / `TOOL_WHITELIST` / `WS_PATH` / `validate_sdk_method` / `validate_tool_call` (per `lib.rs:274-279`)

**0 改 11 agent 公共 API 签名**: 仅在 `lib.rs` 末尾 mod 声明区域加 3 行 cfg-gated mod 声明, 0 改公共 API 顶层 re-export, 0 改 client.rs 任何 fn 签名。

### 0.10 阶段边界 + 8 项承诺守门

- R25 决策: 0 跨语言 binding 在 apeireth-sdk
- R119 决策: 8 项形式撤销, 原意保留, 形式可重整
- R122-8 决策: cfg-gated features 隔离, 默认 0 装 = O-5 实质守门
- 1:1 翻译 VCP 跨语言 4 维度 (Python/Node/Go/Rust), **仅 demo 1 fn per language**, 0 假装 100% (per O-5 + hard-constraint #8)

---

## 1. 目标文件清单 (8 新建 + 1 改 + 0 改 LOCKED)

| 文件 | 类型 | 估计行数 | 内容 |
|------|------|---------|------|
| `crates/apeireth-sdk/Cargo.toml` | **改 +1 段** | +25 | 加 `[features]` + 3 optional deps + 1 build-dep cbindgen |
| `crates/apeireth-sdk/src/lib.rs` | **改 +3 行 + 1 段** | +15 | mod 声明 3 行 cfg-gated + O-5 哲学锚补 1 段 R122-8 |
| `crates/apeireth-sdk/src/python.rs` | **新建** | ~100 | cfg-gated pyo3 pymodule + 1 fn + 3 tests |
| `crates/apeireth-sdk/src/node.rs` | **新建** | ~100 | cfg-gated napi 2 fn + 3 tests |
| `crates/apeireth-sdk/src/c.rs` | **新建** | ~150 | cfg-gated 5 fn C 签名 + 3 tests |
| `crates/apeireth-sdk/build.rs` | **新建** | ~25 | cbindgen 调用, 0 错误时 generate apeireth_sdk.h |
| `crates/apeireth-sdk/apeireth_sdk.h` | **新建 (cbindgen auto-generate)** | ~80 | 5 fn C 声明 |
| `crates/apeireth-sdk/tests/multilang_ffi.rs` | **新建** | ~150 | 5 集成 test (cfg feature gated) |
| `crates/apeireth-sdk/examples/c_consumer_demo.c` | **新建** | ~50 | C demo ccall 5 fn |

**0 改**:
- `crates/apeireth-sdk/src/{abi,client,error,version,wire}.rs` (24 LOCKED baseline 8/5 16:34 之前, 0 触碰)
- `crates/apeireth-sdk/src/lib.rs` 公共 API 顶层 re-export 段 (0 改 19 个 pub use)
- `crates/apeireth-sdk/tests/{test_sdk_client,smoke}.rs` (LOCKED)
- `crates/apeireth-sdk/examples/sdk_demo.rs` (LOCKED)
- `crates/apeireth-sdk/Cargo.toml` description / version / dependencies 现有段
- workspace Cargo.toml (per hard-constraint #1)
- 24 LOCKED crate mtime (per hard-constraint #3)
- 9 器官 logic / 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 (per hard-constraint #4-5)
- 11 agent 公共 API 签名 (per hard-constraint #6)

---

## 2. 关键决策 (3 个)

### 决策 1: cfg-gated features 隔离 (R122-8 创新, R25 O-5 实质守门)

- **R25 O-5 决策**: PyO3 binding 必须独立 crate (apeireth-pybridge 风格)
- **R119 状态**: 8 项形式撤销, 实质仍严守
- **R122-8 新决策** (本任务):
  - 加 `python` / `node` / `c` 3 个 cfg-gated features
  - `default = []` (0 启用, 0 装 pyo3/napi/cbindgen)
  - 3 个新 mod 加 `#[cfg(feature = "...")]` guard
  - 0 装原则保持 (O-5 实质: 默认 build 0 跨语言污染)
  - **不**走独立 crate 路径 (节省 1 个新 crate, 减少 workspace 复杂度)
  - 0 触碰 apeireth-pybridge (保留作 R123 扩展位)

**风险**: 主人可能更喜欢独立 crate (R25 决策原意)
**缓解**: final report 标"如需独立 crate, R123 拆 apeireth-sdk 桥接到独立 crate (apeireth-pybridge / apeireth-nodebridge / apeireth-cbridge)"

### 决策 2: PyO3 version 用 workspace 0.29, 不是任务 0.22 (R122-8 0 范围扩散)

- **任务要求**: `pyo3 = { version = "0.22", features = ["extension-module"] }`
- **workspace 实际**: `pyo3 = { version = "0.29", features = ["auto-initialize"] }`
- **冲突分析**: 加 pyo3 = "0.22" 到 apeireth-sdk/Cargo.toml 会跟 workspace pyo3 = "0.29" 产生 2 个版本 (cargo 允许但 lock bloat)
- **0 改 workspace 顶层** (per hard-constraint #1): 不能把 workspace 0.29 改 0.22
- **最终决定**: `pyo3 = { workspace = true, features = ["extension-module"], optional = true }` 复用 workspace 0.29
- **0.22 vs 0.29 API 差异**: pyo3 0.22 → 0.29 macro API 兼容 (#[pyo3::pymodule] 1:1), 仅 minor deprecation warning
- final report 诚实标: "PyO3 version 0.29 (workspace 锁), 不是任务 0.22, 因为 workspace 不能改"

### 决策 3: count_tokens / hash_request inline 简版, 0 等 R122-1/3 retry (0 阻塞)

- **任务依赖**: R122-1 (hash_request) + R122-3 (tiktoken_counter) retry 跑中
- **实际状态**: R122-1/3 0 就位, 0 个 fn 在 src/
- **0 阻塞方案** (per 0 范围扩散 + 0 假装):
  - `count_tokens` (CJK + ASCII 启发式, 1:1 R32-1 `apeireth-asi::tokenizer::count_tokens` 算法, inline 不 dep 24 LOCKED)
  - `hash_request` (SHA-256 hex, 1:1 R122-1 设计, 用 `sha2 = "0.10"` optional, 跟 apeireth-api 锁的 sha2 一致)
- **R123 切换路径**: R122-1/3 retry 完成后, 在 apeireth-sdk 顶层 re-export 替换 inline 版本, 调用真正 fn
- final report 诚实标: "skeleton 自带简版 count_tokens + hash_request, R122-1/3 retry 完成后 R123 切换到正式 fn"

---

## 3. 实施计划 (37 min, 紧迫)

| 阶段 | 时间 | 内容 | 耗时 |
|------|------|------|------|
| Readmap (本文) | 14:21-14:30 (8 min) | 现状核验 + 决策 3 个 | 8 min |
| Stage | 14:30-14:33 (3 min) | stage doc 写 | 3 min |
| Cargo.toml | 14:33-14:36 (3 min) | 加 [features] + 4 deps | 3 min |
| lib.rs | 14:36-14:38 (2 min) | 加 3 行 mod + 1 段 O-5 补 | 2 min |
| python.rs | 14:38-14:43 (5 min) | pyo3 桥接 + 3 tests | 5 min |
| node.rs | 14:43-14:48 (5 min) | napi 桥接 + 3 tests | 5 min |
| c.rs + build.rs + .h | 14:48-14:55 (7 min) | cbindgen 桥接 + 3 tests | 7 min |
| multilang_ffi.rs | 14:55-15:00 (5 min) | 5 集成 test | 5 min |
| c_consumer_demo.c | 15:00-15:02 (2 min) | C demo | 2 min |
| Verify | 15:02-15:12 (10 min) | cargo build 4 features + cargo test --lib | 10 min |
| Final + Decision Log | 15:12-15:15 (3 min) | 2 报告 | 3 min |

**截止**: 15:15 (55 min 总)

---

## 4. 验收硬指标 (per task)

- [ ] `cargo build -p apeireth-sdk` 0 error (default features)
- [ ] `cargo build -p apeireth-sdk --features python` 0 error
- [ ] `cargo build -p apeireth-sdk --features node` 0 error
- [ ] `cargo build -p apeireth-sdk --features c` 0 error
- [ ] `cargo test -p apeireth-sdk --lib` 5+ passed, 0 failed
- [ ] `cargo test --workspace` 0 failed (19972 + 5+ tests)
- [ ] 0 改 11 agent 公共 API 签名 (per lib.rs:274-279)
- [ ] 0 触碰 24 LOCKED (mtime 实查 baseline 8/5 16:34 之前)
- [ ] 0 改 workspace.version (1.1.0 per workspace Cargo.toml:line ~165)
- [ ] 0 假装 multi-lang 完成 (R123 续扩)

---

## 5. 风险登记

| # | 风险 | 严重度 | 缓解 |
|---|------|--------|------|
| 1 | napi 2.16 真实版本 resolve 失败 | 中 | 备选 napi = "3" (latest, 任务"2.16" 描述不严) |
| 2 | pyo3 0.29 macro API 跟 0.22 有 deprecation | 低 | #[allow(deprecated)] + final report 标 |
| 3 | cbindgen 0.26 在 build 0 在 feature c 时不工作 | 中 | build.rs 用 `#[cfg(feature = "c")]` 守门, cbindgen::generate_with_config 仅 feature 启用时 |
| 4 | sha2 dep 加 apeireth-sdk/Cargo.toml 触发 workspace lock 重算 | 低 | 0 改 workspace Cargo.toml, lock entry 仅加 apeireth-sdk → sha2 edge (sha2 已在 lock) |
| 5 | R122-1/3 retry 阻塞 (虽不依赖) | 低 | 0 范围扩散, 0 等 |
| 6 | 5 集成 test 编译期 cfg-gate 失效 | 低 | test 内用 `#[cfg(feature = "python")]` 等守门 |
| 7 | apeireth-sdk/src/lib.rs O-5 哲学锚段加 R122-8 注释可能漂移 6 哲学锚 | 中 | 仅在 O-5 段尾加 1 段 5-行注释, 0 改 6 哲学锚任何定义 |
| 8 | `#![deny(unsafe_code)]` 跟 napi/pyo3 unsafe 冲突 | 中 | napi/pyo3 mod 内 `#![allow(unsafe_code)]` (per abi.rs 已有模式) |

---

## 6. 时间戳

- **启动**: 2026-08-10 14:21:11
- **readmap 完**: 2026-08-10 14:30:00 (预计)
- **截止**: 2026-08-10 15:15:00 (55 min 后)

**R122-8 readmap 完, 等 stage + 实施. Mavis review.**
