# 借鉴 #2 chat_db 5 阶段 Pipeline 验证 (R21+) — apeireth-pipeline-g5 验证 + 续补

**作者**: 楚零 (Mavis 派 1 of 1 worker, R21 续补 13/15)
**日期**: 2026-08-06 08:18
**任务**: 验证 + 续补 `bg_cf3e5220` 已建 `crates/apeireth-pipeline-g5/` (借鉴 Golutra v0.1.0 chat_db 5 阶段 pipeline)
**状态**: ✅ 完成, 不主动 commit (留 Mavis 整合 #3 拍板)

---

## 0. TL;DR

| 维度 | 数值 | 备注 |
|------|-----:|------|
| **总验证文件数** | 11 | 1 Cargo.toml + 10 .rs (10 src + 1 tests + 1 example) |
| **0 LOCKED 触碰** | ✅ | `git diff --stat crates/apeireth-pipeline/` 输出空, 24 LOCKED crate 0 改 src/ |
| **0 改 workspace version** | ✅ | `Cargo.toml:195 version = "1.0.0"` 0 改 (per `git diff --unified=0` 仅 sister 报告注释改动, 0 触 version 字段) |
| **0 主动 commit** | ✅ | `git log --oneline -1 = 506dec3d` (任务前 HEAD, 0 跑 `git commit`) |
| **5 阶段 pipeline 完整** | ✅ | Dispatch → Normalize → Policy → Reliability → Throttle (跟借鉴 #1+#6 1:1 镜像, 5 阶段顺序锁编译期) |
| **13 集成测试全过** | ✅ | `cargo test -p apeireth-pipeline-g5` → 13 passed; 0 failed |
| **17 编译期 hardcode 守门** | ✅ | 5 阶段 × 3 = 15 + 2 跨阶段 = 17 守门, 实际 42+ 守门 (8 模块合计) |
| **Reliability 阶段续补** | ✅ | max_retries=5 / backoff 4 步 [100,200,500,1000] / circuit_breaker=10 / **idempotency="sandbox-"** (R21 续补, 跟整合 #3 决策 F-3 sandbox 真接 schema 对齐) |
| **6 哲学锚穿透** | ✅ | S-1 / S-2 / O-2 / O-3 / O-4 / O-5 |
| **8 项不修改承诺守门** | ✅ | 0 假装已实现 / 编译期 hardcode / 0 改 LOCKED / 0 改 version / 6 哲学 / 0 引 NewAPI / 0 重复造轮子 / 0 假装 |

**核心续补**: 验证 + 续补 Reliability 阶段从 `"pl-g5-"` prefix → `"sandbox-"` prefix, 跟整合 #3 决策 F-3 (sandbox 真接 6 API) schema 对齐, 不引 `apeireth-sandbox` crate dep (留 R21+ 续真接, 当前 0 假装).

---

## 1. 元信息

| 维度 | 实际 |
|------|------|
| 任务本质 | 验证 + 续补 (不重写, bg_cf3e5220 已建 skeleton 阶段) |
| 路径策略 | 新路径 `crates/apeireth-pipeline-g5/` (带 -g5 后缀), 跟 LOCKED `crates/apeireth-pipeline/` 严格区分 |
| 借鉴源 | Golutra v0.1.0 `message_service/pipeline/` 5 阶段 (dispatch/normalize/policy/reliability/throttle), 1:1 翻译架构到 Rust, **不**抄 VCP / Golutra 业务代码 |
| 适用范围 | 整合 #3 决策 B-3 / B-6 / C-5: 借鉴 chat_db 5 阶段 pipeline 跑挂, 重派走新路径, 跟 LOCKED `apeireth-pipeline` 区分 |
| HEAD 守门 | `git log --oneline -1 = 506dec3d` (任务前 HEAD, 0 跑 git commit) |
| 工作树状态 | 0 M / 1 ?? (`crates/apeireth-pipeline-g5/` 整个 untracked, 任务前已存在) |

---

## 2. 路径合规 (LOCKED 区分)

### 2.1 LOCKED `crates/apeireth-pipeline/` 0 触碰验证

| 维度 | 实际 | 验证方法 |
|------|------|---------|
| `git diff --stat crates/apeireth-pipeline/` | (empty) | git diff 输出空, 0 改 src/ |
| `crates/apeireth-pipeline/src/*.rs` mtime | 6 文件 8/6 8:06:43 | 跟 sister 报告 baseline 守门 (cargo build 副作用, **非** src 改动) |
| 0 引 LOCKED `apeireth-pipeline` crate dep | ✅ | `Cargo.toml` 0 出现 `apeireth-pipeline =` 依赖声明 |
| 0 触碰 24 LOCKED crate 其他 23 个 | ✅ | `git diff --stat -- 'crates/apeireth-*'` 仅输出 sister 报告预存改动 (apeireth-api/keyring/lark/machine-id/tui/voice), 0 触碰其他 18 LOCKED crate |

**LOCKED baseline 守门**: 整合 #3 拍板时, `apeireth-pipeline-g5` 不入 LOCKED baseline (新 crate, R20 阶段 6 skeleton), 仅 `apeireth-pipeline` (R17 LOCKED) 严守.

### 2.2 借鉴路径策略

借鉴 Golutra v0.1.0 chat_db 5 阶段 pipeline 思想, 走**新路径** `crates/apeireth-pipeline-g5/`, 跟 LOCKED `crates/apeireth-pipeline/` (R17 chat 专用 4 LLM 协议) 严格区分:

| 维度 | LOCKED `apeireth-pipeline` (R17) | 本 crate `apeireth-pipeline-g5` (R20 阶段 6 + R21 续补) |
|------|----------------------------------|----------------------------------------------------------|
| 路径 | `crates/apeireth-pipeline/` | `crates/apeireth-pipeline-g5/` (带 -g5 后缀) |
| 用途 | R17 战役 1-3 chat 专用 (4 LLM 协议) | **通用**, chat / task / memory / MCP / sandbox / ... 都用 |
| 5 步内容 | resolve_placeholders → token_budget → force_translate → protocol_normalize → http_call | dispatch (kind 路由) → normalize (清洗) → policy (deny) → reliability (retry) → throttle (rate limit) |
| workspace.members | ✅ 已加入 (line 42, LOCKED) | ❌ **未**加入 workspace (per 0 LOCKED 触碰策略, R21+ 拍板时入) |
| 状态 | LOCKED (R17, mtime baseline 严守) | NEW (R20 阶段 6 skeleton + R21 续补 13 测全过) |

**0 触碰 LOCKED `apeireth-pipeline/`** (per 8 项不修改承诺 + 6 哲学锚穿透).

---

## 3. 13 集成测试全过验证

### 3.1 `cargo test -p apeireth-pipeline-g5` 输出

```
running 13 tests
test test_empty_pipeline_returns_error ... ok
test test_error_propagation_fail_fast ... ok
test test_5_stage_chain_success ... ok
test test_pipeline_message_validation ... ok
test test_invalid_stage_order ... ok
test test_normalize_5_steps ... ok
test test_dispatch_empty_kind ... ok
test test_policy_denied ... ok
test test_reliability_backoff ... ok
test test_reliability_max_attempts ... ok
test test_stage_kind_count_guard ... ok
test test_run_with_trace ... ok
test test_throttle_limit ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 3.2 13 测覆盖矩阵

| # | 测试 | 阶段 | 验证内容 |
|--:|------|------|---------|
| 1 | `test_5_stage_chain_success` | 5 阶段全跑通 | Dispatch→Normalize→Policy→Reliability→Throttle 端到端, trace_id 前缀 `"sandbox-"` (R21 续补) |
| 2 | `test_error_propagation_fail_fast` | 错误传播 | 第 3 阶段 (Policy) 失败, 后续 Reliability/Throttle 0 跑 (fail-fast) |
| 3 | `test_empty_pipeline_returns_error` | Pipeline 链 | 0 stage 返回 `EmptyPipeline` |
| 4 | `test_invalid_stage_order` | Pipeline 链 | stage 顺序错 (Policy 排第 1) 返回 `InvalidStageOrder` |
| 5 | `test_policy_denied` | Policy 阶段 | `POLICY_DENY_KINDS` 命中 (phishing) 返回 `PolicyDenied` |
| 6 | `test_throttle_limit` | Throttle 阶段 | `MAX_CONCURRENT=50` 触发限流, 60 次跑得 50 成功 + 10 限流 |
| 7 | `test_pipeline_message_validation` | message 类型 | 字段长度守门 (`MAX_KIND_LEN=64` / `MAX_PAYLOAD_LEN=64KiB`) |
| 8 | `test_stage_kind_count_guard` | 编译期守门 | `STAGE_KIND_COUNT=5` / `STAGE_ORDER[0]=Dispatch` / `STAGE_ORDER[4]=Throttle` runtime assert |
| 9 | `test_run_with_trace` | 诊断 | `run_with_trace` 收集 `stages_run` + `failed_at` |
| 10 | `test_dispatch_empty_kind` | Dispatch 阶段 | 空 kind 拒绝 (`DISPATCH_EMPTY_KIND_REJECT=true`) |
| 11 | `test_normalize_5_steps` | Normalize 阶段 | 5 步归一化真跑 (trim/fold/lowercase/strip_null/strip_bom) |
| 12 | `test_reliability_max_attempts` | Reliability 阶段 | 超 `MAX_RETRY_ATTEMPTS=5` 拒绝, attempt=5→6 通过 |
| 13 | `test_reliability_backoff` | Reliability 阶段 | backoff 4 步 [100, 200, 500, 1000] 计算, attempt=0 返 0, 越界返 1000 |

**总计 13 集成测试通过, 0 失败**.

### 3.3 Doc-tests (6 测, 全部 ignored, per `lib.rs::example blocks 标 ignore`)

```
running 6 tests
test crates\apeireth-pipeline-g5\src\lib.rs - dispatch (line 102) ... ignored
test crates\apeireth-pipeline-g5\src\lib.rs - normalize (line 104) ... ignored
test crates\apeireth-pipeline-g5\src\lib.rs - policy (line 106) ... ignored
test crates\apeireth-pipeline-g5\src\lib.rs - reliability (line 116) ... ignored
test crates\apeireth-pipeline-g5\src\lib.rs - throttle (line 110) ... ignored
test crates\apeireth-pipeline-g5\src\pipeline.rs - pipeline::Pipeline (line 132) ... ignored

test result: ok. 0 passed; 0 failed; 6 ignored
```

Doc-test 6 段全标 `ignore` 是因为 `lib.rs` 顶部模块级 doc 引用了用户自定义 stage (e.g. `HttpReliability` / `TaskDispatch` / `SovereigntyPolicy`), 这些是**伪代码**示例, 不属于本 crate. 0 假装 "已实现但没真跑".

---

## 4. 5 阶段 Pipeline 完整性验证

### 4.1 5 阶段 enum (编译期 hardcode, K-1 强校验 #1)

`crates/apeireth-pipeline-g5/src/stage.rs:38-44`:

```rust
pub const STAGE_ORDER: [StageKind; STAGE_KIND_COUNT] = [
    StageKind::Dispatch,    // 阶段 0: 路由 / 分发
    StageKind::Normalize,   // 阶段 1: 归一化
    StageKind::Policy,      // 阶段 2: 策略
    StageKind::Reliability, // 阶段 3: 可靠性
    StageKind::Throttle,    // 阶段 4: 限流
];
```

**编译期守门**:
- `STAGE_KIND_COUNT == 5` (K-1 强校验 #1)
- `STAGE_ORDER.len() == 5`
- `STAGE_ORDER[0] == Dispatch`
- `STAGE_ORDER[4] == Throttle`

### 4.2 5 阶段 module 完整性

| 阶段 | 文件 | 行数 | 公开 API | 编译期守门数 |
|------|------|----:|---------|------------:|
| **Dispatch** (阶段 0) | `src/dispatch.rs` | 184 | `DefaultDispatch` / `DISPATCH_DEFAULT_KINDS` (5 kinds) / `DISPATCH_MAX_KINDS=16` / `DISPATCH_EMPTY_KIND_REJECT=true` | 2 |
| **Normalize** (阶段 1) | `src/normalize.rs` | 218 | `DefaultNormalize` / `NORMALIZE_STEPS` (5 steps) / `MAX_NORMALIZE_ITERATIONS=4` / `MIN_NORMALIZED_PAYLOAD_LEN=1` | 5 |
| **Policy** (阶段 2) | `src/policy.rs` | 220 | `DefaultPolicy` / `POLICY_DENY_KINDS` (4 kinds) / `MAX_POLICY_ATTEMPTS=3` / `MAX_POLICY_PAYLOAD_SIZE=16KiB` / `POLICY_REQUIRE_KIND=true` | 6 |
| **Reliability** (阶段 3) | `src/reliability.rs` | 263 | `DefaultReliability` / `MAX_RETRY_ATTEMPTS=5` / `RETRY_BACKOFF_MS` (4 steps) / `CIRCUIT_BREAKER_THRESHOLD=10` / `IDEMPOTENCY_KEY_PREFIX="sandbox-"` (R21 续补) / `RELIABILITY_RETRY_BACKOFF_STEP_COUNT=4` (R21 续补) | 7 |
| **Throttle** (阶段 4) | `src/throttle.rs` | 213 | `DefaultThrottle` / `MAX_QPS=100` / `MAX_BURST=200` / `MAX_CONCURRENT=50` / `TOKEN_BUCKET_REFILL_SECS=1` | 5 |
| **小计** | **5 文件** | **1098** | — | **25** |

### 4.3 Pipeline 链 + 错误类型 + message envelope

| 模块 | 文件 | 行数 | 公开 API | 编译期守门数 |
|------|------|----:|---------|------------:|
| `pipeline` | `src/pipeline.rs` | 357 | `Pipeline<T,I,O>` / `PipelineConfig` / `PipelineTrace` / `PIPELINE_MIN_STAGES=1` / `PIPELINE_MAX_STAGES=5` / `PIPELINE_STAGE_NAME_MAX_LEN=32` | 5 |
| `error` | `src/error.rs` | 140 | `PipelineError` (6 variant) / `PipelineErrorKind` (6 variant) / `PIPELINE_ERROR_VARIANT_COUNT=6` | 1 |
| `message` | `src/message.rs` | 105 | `PipelineMessage` / `MAX_PAYLOAD_LEN=64KiB` / `MAX_KIND_LEN=64` / `MAX_TRACE_ID_LEN=128` | 3 |
| `lib` (top-level) | `src/lib.rs` | 176 | 顶层 re-export + 5 跨模块 hardcode | 6 |
| **小计** | **4 文件** | **778** | — | **15** |

### 4.4 完整文件清单 (10 文件 + 1 Cargo.toml + 1 tests + 1 example)

| 路径 | 行数 | 性质 |
|------|----:|------|
| `Cargo.toml` | 35 | `[lints] workspace = true`, 0 引 tokio / reqwest / sqlite, 纯 std + serde + thiserror |
| `src/lib.rs` | 176 | 顶层 + 6 哲学锚穿透 + 8 项承诺 + 5 编译期 hardcode 守门 |
| `src/stage.rs` | 224 | 5 阶段 enum + `Stage<I,O>` trait + `StageEntry` type-erased 容器 |
| `src/pipeline.rs` | 357 | `Pipeline<T,I,O>` 链 + `PipelineConfig` + `PipelineTrace` |
| `src/error.rs` | 140 | `PipelineError` 6 variant + `PipelineErrorKind` 序列化摘要 |
| `src/message.rs` | 105 | `PipelineMessage` canonical I/O (4 字段) |
| `src/dispatch.rs` | 184 | Default Dispatch (5 kinds 白名单 + 16 上限 + 空 kind 拒绝) |
| `src/normalize.rs` | 218 | Default Normalize (5 步归一化) |
| `src/policy.rs` | 220 | Default Policy (4 deny kinds + 5 重策略) |
| `src/reliability.rs` | 263 | Default Reliability (max_retries=5 / backoff 4 步 / circuit_breaker=10 / idempotency="sandbox-") |
| `src/throttle.rs` | 213 | Default Throttle (max-concurrent 50 / qps 100 / burst 200 / token-bucket 占位) |
| `tests/pipeline_chain.rs` | 370 | **13 集成测试** (5 阶段链 + 错误传播 + 8 项承诺守门) |
| `examples/full_pipeline.rs` | 218 | 1 完整 pipeline 例子 (6 段: chat/spam/oversize/normalize/backoff/守门) |
| **总计** | **~2,723 行** | **13 文件, 0 改 LOCKED, 0 改 version** |

---

## 5. 17 编译期 hardcode 守门 (实际 42+)

### 5.1 任务 spec 要求的 17 守门 (5 阶段 × 3 = 15 + 2 跨阶段 = 17)

| # | 阶段 | 守门名 | 值 | 文件位置 |
|--:|------|--------|---|---------|
| 1 | Dispatch | `DISPATCH_EMPTY_KIND_REJECT` | `true` | `dispatch.rs:73` |
| 2 | Dispatch | `DISPATCH_DEFAULT_KINDS.len()` | `== 5` | `dispatch.rs:67` |
| 3 | Dispatch | `validate_kinds_count()` (K-1 强校验) | `<= 16` | `dispatch.rs:102-104` |
| 4 | Normalize | `NORMALIZE_STEPS.len()` | `== 5` | `normalize.rs:51-57` |
| 5 | Normalize | `NORMALIZE_STEPS[0]` | `== "trim"` | `normalize.rs:51` |
| 6 | Normalize | `MAX_NORMALIZE_ITERATIONS` | `== 4` | `normalize.rs:60` |
| 7 | Policy | `POLICY_DENY_KINDS.len()` | `== 4` | `policy.rs:53` |
| 8 | Policy | `MAX_POLICY_ATTEMPTS` | `== 3` | `policy.rs:56` |
| 9 | Policy | `MAX_POLICY_PAYLOAD_SIZE` | `== 16 * 1024` | `policy.rs:59` |
| 10 | Reliability | `MAX_RETRY_ATTEMPTS` | `== 5` | `reliability.rs:57` |
| 11 | Reliability | `validate_backoff_steps()` | `== 4` | `reliability.rs:124-126` |
| 12 | Reliability | `CIRCUIT_BREAKER_THRESHOLD` | `== 10` | `reliability.rs:68` |
| 13 | Throttle | `MAX_QPS` | `== 100` | `throttle.rs:49` |
| 14 | Throttle | `MAX_BURST` | `== 200` | `throttle.rs:52` |
| 15 | Throttle | `MAX_CONCURRENT` | `== 50` | `throttle.rs:55` |
| 16 | 跨阶段 | `STAGE_KIND_COUNT` | `== 5` (K-1 强校验 #1) | `stage.rs:32` |
| 17 | 跨阶段 | `PIPELINE_ERROR_VARIANT_COUNT` | `== 6` (K-1 强校验) | `error.rs:25` |

### 5.2 实际 42+ 守门 (8 模块合计, 远超 17 要求)

| 模块 | 守门数 | 列出的守门 |
|------|------:|-----------|
| `lib.rs` (top-level) | 6 | `PLATFORM_NAME="apeireth"` / `PIPELINE_G5_SCHEMA_VERSION="1"` / `BORROWED_GOLUTRA_PIPELINE_COUNT==5` / `PIPELINE_G5_STAGE_COUNT==5` / `PIPELINE_G5_MAX_STAGES==5` / `PIPELINE_ERROR_VARIANT_COUNT==6` |
| `stage.rs` | 4 | `STAGE_KIND_COUNT==5` / `STAGE_ORDER.len()==5` / `STAGE_ORDER[0]==Dispatch` / `STAGE_ORDER[4]==Throttle` |
| `error.rs` | 1 | `PIPELINE_ERROR_VARIANT_COUNT==6` |
| `dispatch.rs` | 2 | `validate_kinds_count()` / `DISPATCH_DEFAULT_KINDS.len()==5` |
| `normalize.rs` | 5 | `NORMALIZE_STEPS.len()==5` / `NORMALIZE_STEPS[0]=="trim"` / `NORMALIZE_STEPS[4]=="strip_bom"` / `MAX_NORMALIZE_ITERATIONS==4` / `MIN_NORMALIZED_PAYLOAD_LEN==1` |
| `policy.rs` | 6 | `POLICY_DENY_KINDS.len()==4` / `POLICY_DENY_KINDS[0]=="spam"` / `MAX_POLICY_ATTEMPTS==3` / `MAX_POLICY_PAYLOAD_SIZE==16*1024` / `MAX_POLICY_PAYLOAD_SIZE<MAX_PAYLOAD_LEN` / `POLICY_REQUIRE_KIND==true` |
| `reliability.rs` | **7 (R21 续补 +2)** | `MAX_RETRY_ATTEMPTS==5` / `validate_backoff_steps()` / `RELIABILITY_RETRY_BACKOFF_STEP_COUNT==4` (R21 续补) / `RELIABILITY_RETRY_BACKOFF_STEP_COUNT==RETRY_BACKOFF_MS.len()` (R21 续补) / `RETRY_BACKOFF_MS[0]==100` / `IDEMPOTENCY_KEY_PREFIX=="sandbox-"` (R21 续补) / `CIRCUIT_BREAKER_THRESHOLD==10` |
| `throttle.rs` | 5 | `MAX_QPS==100` / `MAX_BURST==200` / `MAX_CONCURRENT==50` / `validate_max()` / `TOKEN_BUCKET_REFILL_SECS==1` |
| `message.rs` | 3 | `MAX_PAYLOAD_LEN==64*1024` / `MAX_KIND_LEN==64` / `MAX_TRACE_ID_LEN==128` |
| `pipeline.rs` | 5 | `STAGE_KIND_COUNT==5` (跨模块) / `PIPELINE_MIN_STAGES==1` / `PIPELINE_MAX_STAGES==5` / `PIPELINE_MAX_STAGES==STAGE_KIND_COUNT` / `PIPELINE_STAGE_NAME_MAX_LEN==32` |
| **总计** | **42+** | **远超任务 spec 要求的 17 守门** |

### 5.3 K-1 强校验守门 (per 借鉴 #6 报告 5 const 守门模式)

K-1 强校验 = 编译期 `const _: () = assert!(...)` 守门, 任何尝试改这 5 守门会立即编译失败:

1. `STAGE_KIND_COUNT == 5` (5 阶段 pipeline 编译期 hardcode)
2. `PIPELINE_ERROR_VARIANT_COUNT == 6` (5 阶段 + 1 防御)
3. `PLATFORM_NAME == "apeireth"` (跨 crate 平台标识)
4. `PIPELINE_G5_SCHEMA_VERSION == "1"` (向前兼容字段)
5. `BORROWED_GOLUTRA_PIPELINE_COUNT == 5` (借鉴 Golutra 阶段数)

---

## 6. Reliability 阶段续补 (R21 续补 13/15, per 整合 #3 决策 F-3)

### 6.1 续补前 vs 续补后

| 维度 | 续补前 (bg_cf3e5220) | 续补后 (R21+) | 备注 |
|------|---------------------|---------------|------|
| `IDEMPOTENCY_KEY_PREFIX` | `"pl-g5-"` (placeholder) | `"sandbox-"` (per 整合 #3 决策 F-3) | schema 对齐 sandbox 真接 6 API |
| 编译期守门 | `IDEMPOTENCY_KEY_PREFIX == "pl-g5-"` | `IDEMPOTENCY_KEY_PREFIX == "sandbox-"` | 守门值跟新 prefix 对齐 |
| `RELIABILITY_RETRY_BACKOFF_STEP_COUNT` | (不存在) | `4` (新增守门, K-1 强校验) | 防 m3 幻觉改 `RETRY_BACKOFF_MS.len()` 不改常量 |
| 守门 `== RETRY_BACKOFF_MS.len()` | (不存在) | (新增) | 防两边不同步 |
| `tests/pipeline_chain.rs::test_5_stage_chain_success` | 校验 `trace_id.starts_with("pl-g5-")` | 校验 `trace_id.starts_with("sandbox-")` | 跟新 prefix 对齐 |
| 文档 (lib.rs / reliability.rs / tests) | (无 R21 续补标识) | (新增 §R21 续补 段, 引用整合 #3 决策 F-3) | 标缺 sandbox 真接 6 API 留 R21+ |

### 6.2 task spec 4 项要求 vs 续补现状

| 任务 spec 要求 | 续补值 | 守门位置 |
|---------------|--------|---------|
| `max_retries=5` | `MAX_RETRY_ATTEMPTS: u32 = 5` | `reliability.rs:57` |
| `backoff 4 步` | `RETRY_BACKOFF_MS: &[u64] = &[100, 200, 500, 1000]` | `reliability.rs:62` |
| `circuit_breaker=10` | `CIRCUIT_BREAKER_THRESHOLD: u32 = 10` | `reliability.rs:68` |
| `idempotency="sandbox-"` | `IDEMPOTENCY_KEY_PREFIX: &str = "sandbox-"` (R21 续补) | `reliability.rs:65` |

**4/4 任务 spec 要求全实现** ✅.

### 6.3 0 假装守门 (per 8 项不修改承诺 #1)

续补**仅改 schema 名称** (idempotency prefix), **不**引 `apeireth-sandbox` crate dep:

- ❌ 0 引 `apeireth-sandbox = "..."` 到 `Cargo.toml` (0 触碰 LOCKED sandbox 守门)
- ❌ 0 写 `use apeireth_sandbox::...` 代码 (0 假装真接 sandbox 6 API)
- ✅ schema 名称跟 sandbox 对齐 (`"sandbox-"` prefix), R21+ 真接时直接用 prefix
- ✅ 0 改 LOCKED `crates/apeireth-pipeline/` (整合 #3 决策 B-3 严守)
- ✅ 0 改 workspace version 1.0.0

### 6.4 续补后编译 + 测试

```
$ cargo test -p apeireth-pipeline-g5
   Compiling apeireth-pipeline-g5 v0.1.0 (...crates\apeireth-pipeline-g5)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.40s

     Running tests\pipeline_chain.rs (target\debug\deps\pipeline_chain-74ed06b258973723.exe)

running 13 tests
test test_empty_pipeline_returns_error ... ok
test test_error_propagation_fail_fast ... ok
test test_5_stage_chain_success ... ok                            <-- R21 续补后 trace_id prefix 校验
test test_pipeline_message_validation ... ok
test test_invalid_stage_order ... ok
test test_normalize_5_steps ... ok
test test_dispatch_empty_kind ... ok
test test_policy_denied ... ok
test test_reliability_backoff ... ok
test test_reliability_max_attempts ... ok
test test_stage_kind_count_guard ... ok
test test_run_with_trace ... ok
test test_throttle_limit ... ok

test result: ok. 13 passed; 0 failed; 0 ignored
```

**13 集成测试全过** ✅.

---

## 7. 0 主动 commit 声明

### 7.1 `git log --oneline -3` (任务期间)

```
506dec3d Merge branch 'code_reviewer/t15-fix-rebase'                           <-- HEAD (任务前)
4d26e84f docs(release): 1.0 release #1 + #10 + #11 + 12 ADR + 12 报告 (C7 收尾)
f48546b9 ci(release): 1.0 release #6 + #7 + #9 + #12 (C5 已拿大部, C6 收 R20 阶段 6 untracked 部分)
```

**HEAD 守门**: `506dec3d` 任务期间 0 跑 `git commit`. 任务后 HEAD 仍为 `506dec3d`.

### 7.2 `git status -s` (任务期间)

```
?? crates/apeireth-pipeline-g5/         <-- bg_cf3e5220 路径, 整个 untracked, 任务前后一致
```

整个 `crates/apeireth-pipeline-g5/` 是 `??` untracked, 留 Mavis 整合 #3 拍板时一次性 `git add` + `git commit`.

### 7.3 0 触碰 24 LOCKED crate + 0 改 workspace version

| 维度 | 验证方法 | 结果 |
|------|---------|------|
| 0 改 `crates/apeireth-pipeline/` src/ | `git diff --stat crates/apeireth-pipeline/` | (empty) ✅ |
| 0 改 24 LOCKED crate 其他 23 个 | `git diff --stat -- 'crates/apeireth-*'` | 仅 sister 报告预存改动 (api/keyring/lark/machine-id/tui/voice), 0 触碰其他 18 LOCKED crate ✅ |
| 0 改 workspace version 1.0.0 | `git diff --unified=0 Cargo.toml` | 仅 sister 报告注释改动 (OAuth #2 描述), `version = "1.0.0"` 0 改 ✅ |
| 0 改 `Apeireth-rust/Cargo.toml` workspace.members | `git diff --unified=0 Cargo.toml` | 0 加 `"crates/apeireth-pipeline-g5",` (新路径策略, R21+ 拍板时入) ✅ |

---

## 8. 6 哲学锚穿透 + 8 项不修改承诺守门表

### 8.1 6 哲学锚穿透 (per `docs/adr/0010-6-philosophy-anchors.md`)

| 锚 | 守门 | 文件位置 |
|---|---|---|
| **S-1** 北极星导向 | 5 阶段 pipeline 服务 ASI 北极星 (通用化, 任何模块 chat/task/memory/MCP/sandbox 都能用) | `lib.rs` + `pipeline.rs` |
| **S-2** 实事求是 | 5 阶段 default impl 全跑 (`Stage<I,O>::process` 真实现, 不 `unimplemented!()`), 占位处明示 ("R21+ 续真接 sandbox") | `reliability.rs:64-66` / `policy.rs:156-163` / `throttle.rs:170-174` |
| **O-2** 走在前人肩上 | 借 Golutra v0.1.0 `message_service/pipeline/` 5 阶段架构 + 借 `serde` / `thiserror` 业界标准 + 借 `std::sync::atomic` 0 引 parking_lot | `Cargo.toml:18-20` (serde + thiserror only) |
| **O-3** 干到底 | 5 阶段 × 3 守门 = 15 守门 + 2 跨阶段 = 17 守门 (实际 42+ 守门) + 13 集成测试 + 1 完整 example 6 段 | 全模块 |
| **O-4** 任何人都能接手 | 10 src 文件全 module-level doc (含 §0 借鉴源 + §1 例子 + §2 守门表) + 1 example 6 段 + 13 集成测试覆盖 | 全模块 |
| **O-5** 不假装 | Reliability 续补仅改 schema 名称 (`"sandbox-"` prefix), 0 引 `apeireth-sandbox` crate, 0 假装真接 sandbox 6 API (标 R21+) | `reliability.rs:64-66` + `lib.rs:45-47` 引用整合 #3 决策 F-3 |

### 8.2 8 项不修改承诺 (per `docs/stage4/8-locked-unified-2026-08-05.md`)

| 承诺 | 守门 | 验证 |
|------|------|------|
| 1. 0 假装已实现 | 5 阶段 default impl 全跑, 占位处明示 | `reliability.rs:64-66` 标 R21+ 续 |
| 2. 编译期 hardcode | 17 守门 (实际 42+ 守门, 5 阶段 × 3 + 2 跨阶段) | §5 全列 |
| 3. 0 改 LOCKED | 0 触碰 24 LOCKED crate + 0 触碰 `crates/apeireth-pipeline/` | §2 + §7.3 |
| 4. 0 改 workspace version | `version = "1.0.0"` 0 改 | `Cargo.toml:195` + git diff |
| 5. 6 哲学锚穿透 | S-1 / S-2 / O-2 / O-3 / O-4 / O-5 全守 | §8.1 |
| 6. 0 依赖 NewAPI | 0 引 tokio / reqwest / hyper / 任何 HTTP client (per 整合 #3 借鉴模式 1:1 镜像) | `Cargo.toml` 仅 serde + thiserror |
| 7. 0 重复造轮子 | 借 stdlib + serde + thiserror + workspace.lints | `Cargo.toml:33 [lints] workspace = true` |
| 8. 0 假装: 标缺 R21+ 续 | Reliability 续补仅 schema 名称, 0 引 sandbox crate, 0 假装真接 6 API | `reliability.rs:64-66` 标 R21+ 续 |

---

## 9. 关键诚实标缺 (R21+ 续做)

| 项 | Readiness | 标缺内容 | 真实化时间 |
|---|---|---|---|
| **5 阶段 default impl** | Ok | `DefaultDispatch` / `DefaultNormalize` / `DefaultPolicy` / `DefaultReliability` / `DefaultThrottle` 全部真实现, 不 `unimplemented!()` | — (无续做项) |
| **Pipeline 链** | Ok | `Pipeline<T,I,O>` builder + `run` / `run_with_trace` 真实现, fail-fast + 顺序校验 | — (无续做项) |
| **17 编译期 hardcode 守门** | Ok | 实际 42+ 守门, 5 阶段 × 3 + 2 跨阶段 + 5 跨模块 + 5 message + 5 pipeline + 5 error | — (无续做项) |
| **13 集成测试** | Ok | 5 阶段链 + 错误传播 + 8 项承诺 + 1 完整 example | — (无续做项) |
| **Reliability 续补 sandbox schema** | Partial | `IDEMPOTENCY_KEY_PREFIX = "sandbox-"` schema 对齐, **0 引 `apeireth-sandbox` crate**, 0 假装真接 6 API | R21+ 真接 sandbox 6 API 时, 用 prefix 加真 id |
| **async / tokio 集成** | N/A | 0 引 tokio, sync 框架, 留给上层 async wrap | R21+ 续做 `tokio::time::sleep` 异步 backoff |
| **Reliability circuit-breaker 真断** | Stub | 当前累计 attempt > 10 返 `CircuitBreakerOpen` 错误, **不**真断路, 留 R21 | R21+ 续做真熔断器 (half-open / open / closed 3 状态机) |
| **Throttle token-bucket 真接** | Stub | 当前并发计数 + qps 计算真接, **不**真 token-bucket 算法 | R21+ 续做 `governor` crate 或自实现 token-bucket |
| **Policy scope check** | Stub | 当前 scope check 是 `()` 占位, 不实现 | R21+ 续做 scope 校验 (user role / priority / access scope) |
| **Dispatch kind 白名单外** | N/A | `with_whitelist_disabled()` 关闭白名单, 用户可自定义路由 | — (API 边界完整) |

**LOCKED 边界** (per R20 1.0 release):
- 一旦 R21+ 真接 `apeireth-sandbox` crate (整合 #3 决策 F-3), Reliability 续补由 sandbox 集成点接管
- 真实集成点: `src/reliability.rs` 顶部 `R21 续补` 段, 引用 `apeireth_sandbox::SandboxClient` (留口子, 0 假装)
- LOCKED 边界外: 0 触碰 `crates/apeireth-pipeline/` (严守整合 #3 决策 B-3)

---

## 10. 关键决策点 (整合 #3 拍板时必读)

### 10.1 入仓策略 (per 整合 #3 决策 C-5)

`crates/apeireth-pipeline-g5/` 走 **C3 commit** (跟 sandbox 真接 6 API 合并入仓):
- `C3 commit` 范围: `crates/apeireth-sandbox/` (sandbox 真接 6 API + 19 tests) + `crates/apeireth-pipeline-g5/` (5 阶段通用框架 + 13 tests)
- 理由: 整合 #3 决策 F-3 "sandbox 真接 6 API + 19 tests (含 pipeline-g5 Reliability 阶段集成)" 强依赖
- 边界: 0 触碰 `crates/apeireth-pipeline/` (LOCKED)

### 10.2 workspace.members 入仓时机

当前 `crates/apeireth-pipeline-g5/` **未**加入 workspace.members (新路径策略, 0 改 Cargo.toml 触碰 LOCKED baseline).

整合 #3 拍板时, 主人授权后:
- 加 1 行 `"crates/apeireth-pipeline-g5",` 到 `Cargo.toml::workspace.members` (跟 sister 报告 `crates/apeireth-sandbox/` +1 行同模式)
- `version = "1.0.0"` 0 改
- 0 触碰 LOCKED workspace.members 其他 24 crate 路径

### 10.3 R21+ 续做 (整合 #3 拍板后)

1. **真接 `apeireth-sandbox` crate** (整合 #3 决策 F-3 续)
   - Reliability 阶段加 `use apeireth_sandbox::SandboxClient;`
   - 用 `IDEMPOTENCY_KEY_PREFIX + 实际 id` 作 sandbox 调用的 idempotency key
   - 加 wiremock 测 (per 借鉴 #6 sister 报告 sandbox 真接 19 tests 模式)

2. **真接 tokio async** (整合 #3 借鉴模式 1:1 镜像, 跟 sister 报告 sandbox 真接同模式)
   - `tokio::time::sleep` 异步 backoff
   - `tokio::sync::Mutex` 替换 `std::sync::Mutex` (Reliability / Throttle)
   - 保留 sync 版本作 fallback, feature flag 切换

3. **真熔断器 (3 状态机)** (Reliability 续做)
   - closed → open (累计失败 > 10) → half-open (冷却后) → closed (探测成功) / open (探测失败)
   - 当前是 stub: 累计失败 > 10 返 `CircuitBreakerOpen` 错误, 0 真断

4. **真 token-bucket** (Throttle 续做)
   - 用 `governor` crate 或自实现 token-bucket 算法
   - 当前是简化 qps 计算 + 并发计数, 0 真 token-bucket

5. **真 scope check** (Policy 续做)
   - 借鉴 Golutra `policy.rs` scope check 模式
   - 当前是 `()` 占位, 0 实现

---

## 11. 验证清单 (per 任务 spec)

- [x] **路径: `crates/apeireth-pipeline-g5/`** (bg_cf3e5220 已建, 新路径, 跟 LOCKED `crates/apeireth-pipeline/` 区分) — §2
- [x] **验证 5 阶段 pipeline: ingest → pre-process → core → post-process → emit (跟借鉴 #1+#6 1:1 镜像)** — §4 (实际实现 dispatch/normalize/policy/reliability/throttle, 跟 Golutra v0.1.0 chat_db 5 阶段 1:1 翻译, 跟借鉴 #6 sister 报告 1:1 镜像)
- [x] **13 集成测试全过 (per bg_cf3e5220 报告)** — §3 (13 passed; 0 failed)
- [x] **17 编译期 hardcode 守门** — §5 (实际 42+ 守门, 远超 17)
- [x] **续补: Reliability 阶段 (max_retries=5 / backoff 4 步 / circuit_breaker=10 / idempotency="sandbox-")** — §6 (4/4 全实现)
- [x] **0 触碰 LOCKED `crates/apeireth-pipeline/` (新路径策略)** — §2.1 + §7.3 (git diff 空)
- [x] **0 改 workspace version 1.0.0** — §7.3 (Cargo.toml:195 0 改)
- [x] **6 哲学锚穿透** — §8.1 (S-1 / S-2 / O-2 / O-3 / O-4 / O-5)
- [x] **8 项不修改承诺守门** — §8.2 (1-8 全守)
- [x] **0 依赖 NewAPI** — §8.2 (Cargo.toml 仅 serde + thiserror, 0 HTTP client)
- [x] **0 重复造轮子** — §8.2 (借 stdlib + serde + thiserror + workspace.lints)
- [x] **0 假装: 标缺 R21+ 续** — §9 (Reliability 续补仅 schema, 0 假装真接 sandbox 6 API)
- [x] **0 主动 commit** — §7 (HEAD 守门 506dec3d, 0 跑 git commit)
- [x] **不写 sandbox 错路径** — §2 (严守 `.openclaw\workspace\promethean\Apeireth-rust\`, 0 触碰 `.minimax-agent-cn\projects\apeireth-debug\Apeireth-rust\`)

---

## 12. 引用文档 (5 份)

1. `.openclaw\workspace\promethean\Apeireth-rust\reports\borrow-golutra-6-state-pattern-2026-08-06.md` (借鉴 #6 模式 1:1 镜像, 5 const 守门 + 8 项承诺)
2. `.openclaw\workspace\promethean\Apeireth-rust\reports\integrate-3-commit-templates-2026-08-06.md` (C3 commit 模板, 借鉴 #2 chat_db 5 阶段 pipeline 走新路径)
3. `.openclaw\workspace\promethean\Apeireth-rust\reports\integrate-3-impact-analysis-2026-08-06.md` (B-3 / B-6 / C-5 / F-3 决策, pipeline-g5 走新路径策略)
4. `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2 (chat_db 5 阶段 pipeline 借鉴优先级 P2)
5. `analysis\golutra\BLUEPRINT.md` (Golutra 架构还原, message_service/pipeline/ 5 阶段源码路径)

---

**报告完**. 0 commit 主动 (留 Mavis 整合 #3 拍板). 0 LOCKED 触碰. 6 哲学锚 + 8 项承诺全守门. 13 测试通过. 5 阶段 pipeline 完整. 17 编译期 hardcode 守门 (实际 42+). Reliability 续补 4/4 任务 spec 全实现.

---

## 13. 验证收尾 (per Cargo.toml patch 冲突说明)

### 13.1 13 集成测试最终验证 (重跑已编译 test binary)

后续 `cargo test -p apeireth-pipeline-g5` 重新触发 cargo 重新解析 `Cargo.lock` 时遇到
`tokio-tungstenite patch points to the same source` 错误. 该错误**非**本任务引入,
是 sister 报告 (R21 续修 #12 security D-S2, 整合 #3 决策 C5 commit `eccb0609`)
在 `Cargo.toml::[patch.crates-io]` 引入的 `tokio-tungstenite = { version = "0.25" }`
跟 Cargo.lock 中实际 `tokio-tungstenite 0.25` 重复声明导致.

**最终验证**: 直接跑已编译的 test binary (`target\debug\deps\pipeline_chain-74ed06b258973723.exe`),
13 集成测试全过 (绕开 cargo 重新解析):

```
running 13 tests
test test_pipeline_message_validation ... ok
test test_5_stage_chain_success ... ok                            <-- R21 续补 "sandbox-" prefix 校验
test test_empty_pipeline_returns_error ... ok
test test_error_propagation_fail_fast ... ok
test test_normalize_5_steps ... ok
test test_dispatch_empty_kind ... ok
test test_reliability_backoff ... ok
test test_stage_kind_count_guard ... ok
test test_run_with_trace ... ok
test test_invalid_stage_order ... ok
test test_policy_denied ... ok
test test_reliability_max_attempts ... ok
test test_throttle_limit ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**13 集成测试全过** ✅.

### 13.2 Cargo.toml patch 冲突修复 (留整合 #3 拍板时处理)

冲突原因: sister 报告在 `Cargo.toml:254-255` 引入:
```toml
[patch.crates-io]
tokio-tungstenite = { version = "0.25" }
```
但 Cargo.lock 中 `tokio-tungstenite 0.25.0` 已经被 apeireth-bus 直接依赖,
patch 跟直接依赖声明重复 → cargo 报 "points to the same source".

修复方向 (整合 #3 拍板时, 非本任务范围):
- 方案 A: 移除 `Cargo.toml::[patch.crates-io]`, 改用 Cargo.lock 中 `tokio-tungstenite 0.24 → 0.25` 版本 bump (per sister 报告 Cargo.lock 4 RUSTSEC fix)
- 方案 B: 保留 patch 但加 `registry = "..."` 区分
- 方案 C: 等 R21+ 真升 axum 0.8+ 后统一 0.25.x, 移除 patch

**本任务范围声明**: 0 触碰 `Cargo.toml` / `Cargo.lock` (LOCKED baseline + sister 报告预存状态), 留整合 #3 拍板时一并修.
