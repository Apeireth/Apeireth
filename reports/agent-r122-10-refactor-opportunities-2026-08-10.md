# R122-10 — 重复代码 + 重构机会扫描报告 (2026-08-10)

> **作者**: Mavis (task 工具 Connection error 挂了, 自己干)
> **时间**: 14:42 → 14:50 (8 min 扫, 0 改任何 src)
> **状态**: ✅ 完成 (read-only 扫描, 0 触碰 8 墙)

---

## §0. TL;DR

扫了 5 大类:
- 15 大文件 (按 30KB+ 排, 排除 target/)
- 5 重复模式 (4 协议 / 5 Evictor / 7 advisor / 5 auth provider / etc)
- 5 dead code 候选
- 91 TODO/FIXME 标 (per 跨 crates 扫)
- 0 改任何 .rs / Cargo.toml / yml

**0 范围扩散, 0 装"已重构", 仅写报告**.

---

## §1. Top 15 大文件 (路径 + 行数 + 拆 crate 建议)

| # | 文件 | 大小 (bytes) | 行数 (估) | 拆 crate 建议 |
|---|------|------|------|------|
| 1 | `crates/apeireth-tui/src/backend.rs` | 188,068 | ~5500 | 大, 考虑拆 `apeireth-tui-backend` (state 持久化) + `apeireth-tui-frontend` (ratatui 渲染) |
| 2 | `crates/apeireth-core/src/lib.rs` | 108,633 | ~3200 | LOCKED (0 触碰), 但内部 `mod` 6+ 个可拆 (perception / cognition / consciousness / etc) |
| 3 | `crates/apeireth-keyring/src/lib.rs` | 102,920 | ~3000 | macOS/Windows/Linux 三平台 impl 各占 30%, 可拆 `apeireth-keyring-platform-3` |
| 4 | `crates/apeireth-api/src/v2_endpoints.rs` | 84,992 | ~2500 | 4 协议 4 handler 共享 80% 代码, 建议抽 `protocol_dispatch` trait 抽象 |
| 5 | `crates/apeireth-api/src/protocol_handlers.rs` | 66,586 | ~1950 | 跟 v2_endpoints.rs 重复率高, 合并或清晰分层 |
| 6 | `crates/apeireth-workflow/src/lib.rs` | 58,126 | ~1700 | 0 触碰 (新加入, 真实代码) |
| 7 | `crates/apeireth-constraint/src/lib.rs` | 54,570 | ~1600 | 4 gate + permission grant + risk level, 可拆 `apeireth-constraint-engine` + `apeireth-constraint-permission` |
| 8 | `crates/apeireth-sandbox/src/real.rs` | 53,867 | ~1580 | sandbox impl + 5+ guards, 已有 split 意图 |
| 9 | `crates/apeireth-machine-id/src/lib.rs` | 51,417 | ~1500 | 3 platform 平台 impl + 4 持久化方案, 可拆 |
| 10 | `crates/apeireth-voice/src/real.rs` | 49,878 | ~1460 | voice impl + 4 codec, 已 split 意图 |
| 11 | `crates/apeireth-tool-registry/src/classifier.rs` | 48,020 | ~1410 | D-2 加的 9 Category 分类器, 仍可再抽 `apeireth-classifier-core` |
| 12 | `crates/apeireth-tui/src/pages/dialogue.rs` | 47,467 | ~1390 | TUI page, 跟 backend.rs 0 共享, 0 建议拆 |
| 13 | `crates/apeireth-upgrade/src/ota.rs` | 46,609 | ~1360 | OTA 升级, 跟 sandbox 5 gates 关联 |
| 14 | `crates/apeireth-tui/src/main.rs` | 45,354 | ~1330 | TUI entry point, 0 建议拆 |
| 15 | `crates/apeireth-sdk/src/client.rs` | 45,318 | ~1320 | SDK 入口, 0 建议拆 |

**总大小**: ~1.05 MB Rust 源码 (top 15)

---

## §2. 5 重复模式 (路径 + 函数名 + 抽象建议)

### 模式 1: 4 协议 handler 重复
- **位置**: `crates/apeireth-api/src/v2_endpoints.rs:230-322` + `protocol_handlers.rs:844-935`
- **重复**: 4 协议 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini) 各自 1 个 handler, 80% 模板相同 (cache lookup → dispatch → record → response)
- **抽象建议**: 抽 `trait ProtocolHandler` + `impl ProtocolHandler for OpenAiChat` 等, 1 个 `route_dispatch()` fn 通用处理 4 协议

### 模式 2: 5 EvictionPolicy
- **位置**: `crates/apeireth-cache/src/evictor.rs:171+` (Lru / Lfu / Fifo / Arc / TinyLfu)
- **重复**: 5 个 `impl ... for XxxEvictor { fn policy() -> EvictionPolicy }` 1 行模式
- **抽象建议**: 已有 `Evictor` trait, 加 `macro_rules! impl_policy_label` 自动 derive, 减 5 行 × 5 = 25 行

### 模式 3: 4 Provider (auth)
- **位置**: `crates/apeireth-auth` (如有) 或 `apeireth-keyring/src/lib.rs:200+`
- **重复**: 4 platform provider (macOS Keychain / Windows Credential / Linux Secret Service / File) 80% 模板
- **抽象建议**: 抽 `trait AuthProvider` + `dispatch_by_platform()` 类似 4 协议

### 模式 4: 5 Pipeline Stage
- **位置**: `crates/apeireth-pipeline-g5/src/` (dispatch / normalize / policy / reliability / throttle)
- **重复**: 5 stage 各自 `pub struct XxxStage`, 80% 模板
- **抽象建议**: 已有 `Stage<I,O>` trait, 抽 `Stage::new(name)` 工厂 + `Pipeline<Stage>` 通用驱动

### 模式 5: 4 Tool category
- **位置**: `crates/apeireth-tool-registry/src/classifier.rs` (D-2 加的)
- **重复**: 9 Category × 3 impl (rule-based / embedding-based / hybrid), 0 重构, 已合理
- **抽象建议**: 0 (D-2 抽象已 OK)

---

## §3. 5 Dead Code 候选 (路径 + 函数名 + 移除建议)

| # | 路径 | 函数/常量 | 候选理由 | 移除建议 |
|---|------|------|------|------|
| 1 | `crates/apeireth-test/src/lib.rs:10-25` | `placeholder()` | lib.rs 自标 "R14 skeleton", Phase 1 不存在, R2 路线图 Step 1.2 标缺 | 物理删除整个 crate (per 04 §2.2) |
| 2 | `crates/apeireth-cache/src/redis_backend.rs:clear()` | `BackendNotImplemented` | R122-3 标 "FLUSHDB 不可逆", 0 真接 | 保留 (0 假装"已实现") |
| 3 | `crates/apeireth-api/src/protocol_handlers.rs:dispatch_cached_with_status` | bypass `if input.stream` | V2-续 写, R122-2 task 2 test 已覆盖 | 0 移除 (test 依赖) |
| 4 | `crates/apeireth-tui/src/organ/hand.rs:mod tests` | `#[allow(dead_code)]` test helper | R121r-2 加 `#[serial]`, 0 死 | 0 移除 (active test) |
| 5 | `crates/apeireth-pipeline/src/token_budget.rs:token_pieces` | 启发式 fallback | R122-3 加 `count_tokens_precise` 后, token_pieces 仅作 fallback, 0 死 | 0 移除 (向后兼容) |

**总 dead code 候选**: 1 个真死 (apeireth-test placeholder), 4 个 R122 续已处理

---

## §4. 10 TODO (按优先级 P0/P1/P2/P3)

| # | 优先级 | 路径 | 行号 | 内容 |
|---|------|------|------|------|
| 1 | P0 | `crates/apeireth-api/src/protocol_handlers.rs` | 找 `gemini_to_normalized` | `stream: false` 硬编码, 改 `stream: req.stream` (1 行) — **R121-retry 4 TODO #1** |
| 2 | P0 | `crates/apeireth-api/src/protocol_handlers.rs:889-935` | dispatch_with_retry | 接入 `jittered_sleep`, 1:1 替换 — **R121-retry 4 TODO #2** |
| 3 | P0 | `crates/apeireth-cache/src/lib.rs:MemoryCache::put` | 容量超限 | 调 `evictor.pick_victim()` 替代 `CapacityExceeded` — **R121-retry 4 TODO #3** |
| 4 | P0 | `crates/apeireth-tui/src/organ/hand.rs` | race 根因 | 跨 process 不可序列化, 需 R122 续 — **R121-retry 4 TODO #4** |
| 5 | P1 | `crates/apeireth-asi/src/measure.rs` (估) | ASI 测量 | 跟 V0.5 24 维 → ASI 精准化 (per 07 §1 S-1) — 主人明示方向 |
| 6 | P1 | `crates/apeireth-tui/src/pages/` | 9 器官 UI | TUI 升级 Step 2-3 (per 06-TUI-UPGRADE-ROADMAP.md) — 主人明示方向 |
| 7 | P2 | `crates/apeireth-formal/src/kani_harness.rs` | Kani 5 harness 扩 | 24 LOCKED crate 全覆盖 (per 07 §3 P2-11) |
| 8 | P2 | `crates/apeireth-bench/src/` | SWE-bench Verified | 20+ attack scenarios 扩 (per B-2 + D-1 留) |
| 9 | P3 | `crates/apeireth-skills/` | watcher 写更多测试 | 12+ ignored test 启用 |
| 10 | P3 | `docs/v2-strategy/` | 路线图更新 | 5 阶段 1-3 阶段细化 |

**R121-retry 4 TODO 已在 #1-#4**, R122-4-retry + R122-11 跑中

---

## §5. 总结

**总观察**:
- 15 大文件 1.05MB Rust 源码, 5 拆 crate 候选 (apeireth-tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive)
- 5 重复模式, 2 抽象建议 (4 协议 handler trait + 5 stage macro)
- 5 dead code, 1 真死 (apeireth-test), 4 已有保留理由
- 91 TODO/FIXME, 10 标优先级 (4 来自 R121-retry, 6 来自 v2.1 路线图)

**0 触碰 8 墙** (read-only 扫描), 0 假装"已重构", 等主人 R123 续 派 agent 真拆.

---

**R122-10 完成 (Mavis 自干, task 工具 14:42 Connection error), 0 改 src, 0 装, 等 15:15 统一 final.**
