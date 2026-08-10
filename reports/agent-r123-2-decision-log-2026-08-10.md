# R123-2 Decision Log — 4 协议 handler 抽 trait 抽象

**Date**: 2026-08-10
**Coder**: R123-2
**Final**: `reports/agent-r123-2-final-2026-08-10.md`

---

## 决策 #1: trait error type 选 `String` 而非 `ProtocolError`

**Context**:
- 任务 spec 字面写 `fn dispatch(&self, req: NormalizedRequest) -> Result<NormalizedResponse>` (shorthand)
- 现有 `protocol_handlers::dispatch` 返 `Result<NormalizedResponse, String>` (per 1.0 行为)
- `apeireth_protocol::ProtocolError` 也有, 是 `apeireth_protocol::error::ProtocolError`

**选择**: `Result<NormalizedResponse, String>`

**理由**:
1. **0 漂移 1.0 行为**: 现有 4 协议 handler 走 `Result<NormalizedResponse, String>`, 本 trait 沿用, 0 假装引 `ProtocolError`
2. **测试代码更轻**: 8 unit test 0 需 import `ProtocolError`, 减少 1 个 dep
3. **未来易改**: R123+ 续真接时, impl 内部把 `ProtocolError` 包装成 `String` 即可, 0 改 trait 签名

**风险**: 错误信息粒度比 `ProtocolError` 粗, 但 trait 是 facade 角色, 错误诊断责任在 impl, facade 字符串足够

**Apply when**: 任何 R123+ 续 agent 续接本 trait 时, 0 改 trait error type, 0 改 8 unit test 期望

---

## 决策 #2: 0 触碰现有 4 协议 handler impl (硬约束)

**Context**:
- 任务 spec 硬约束 #6 + #8: 0 改 11 agent 公共 API 签名, 0 装 (O-5)
- R122-1 (cache) + R122-4-retry (jitter) 改过 `protocol_handlers.rs`, 0 归我管
- 任务说 "R123+ 续真接", 我 0 真接

**选择**: 仅新建 1 个 `protocol_handler_trait.rs` mod + 加 1 行 mod 声明, 0 改 `protocol_handlers.rs` / `v2_endpoints.rs` / `server.rs` 任何 fn

**理由**:
1. **0 范围扩散**: 1 新 mod + 1 lib.rs 行 + 1 example + 1 Cargo.toml 块 = 4 文件最小动作
2. **0 越界 commit**: 0 触碰 R122-1 / R122-4-retry 已 commit 的 `protocol_handlers.rs` 改动
3. **R123+ 续清晰**: 后人 1 行 `register(OpenAiChat, OpenAiChatHandler::new())` 即可启用, 0 改 4 协议 handler impl
4. **0 装"已替换"**: trait skeleton 跟现有 4 协议 handler impl 0 接线, 0 假装"已重构"

**风险**: 0 (0 改任何现有文件, 0 副作用)

**Apply when**: 任何 R123+ 续 agent 想"借机重构 4 协议 handler" — 0 允许, 走新 task

---

## 决策 #3: `HandlerRegistry` 用 `HashMap<ProtocolKind, Box<dyn ProtocolHandler + Send + Sync>>` 而非 `BTreeMap`

**Context**:
- `ProtocolKind` impl `Hash + Eq + Ord + PartialOrd` (per `apeireth_protocol::gateway.rs:32`)
- HashMap / BTreeMap 都 OK

**选择**: HashMap

**理由**:
1. **跟 `ProtocolGateway` 一致**: `apeireth_protocol::gateway.rs:140` 用 `HashMap<ProtocolKind, Arc<dyn ProtocolBridge>>`, 我用 Box 而非 Arc (本 trait 是 server 内单例, 0 需 Arc)
2. **O(1) dispatch**: 4 协议规模下 HashMap 跟 BTreeMap 性能同, 但 HashMap 通用
3. **HashMap 已在 11 个文件用**: 0 新 dep, 0 新 import 模式

**风险**: 0 (HashMap 是 std 默认选择)

**Apply when**: 0 (一次性决策, 0 改)

---

## 决策 #4: `route_dispatch` 当前 1:1 调 `handler.dispatch(req)`, 0 接 cache

**Context**:
- 任务 spec 说 "1 个 `route_dispatch()` fn 通用处理 4 协议"
- 80% 模板是 cache lookup → dispatch → record, 但本任务 0 接 cache
- 现有 `protocol_handlers::dispatch_cached_with_status` 已在 B2/B5 战区 2 接好 (R120/R122-1)

**选择**: route_dispatch 当前 1:1 调 handler.dispatch, 0 接 cache lookup / record

**理由**:
1. **0 装"已替换"**: 不假装"已接 cache", 1:1 透明, 注释明确 Phase 1/2
2. **R123+ 续清晰**: 后续 agent 1 步加 cache lookup, 0 改 route_dispatch 签名, 0 改 8 unit test
3. **0 漂移 1.0**: 当前 1.0 走 `dispatch_cached_with_status`, route_dispatch 是替换入口, R123+ 续真接时切

**风险**: 0 (功能等价于"先直调 handler", 0 副作用)

**Apply when**: R123+ 续想"route_dispatch 加 cache lookup" → 改 1 个 fn body, 0 改 trait 签名, 0 改 test

---

## 决策 #5: 8 unit test 命名 = spec 字面名 (per spec §2)

**Context**:
- 任务 spec §2 列了 8 个 test 名, 1:1 字面
- 我加 `mod protocol_handler_trait_tests` 跟现有 `mod replay_cache_tests` 风格一致

**选择**: 8 test 名 1:1 用 spec 字面名, 不简化不重命名

**理由**:
1. **0 装"已验收"**: spec 怎么写, 我就怎么 test, 验收人 1:1 对
2. **0 漂移 spec**: 不擅自加 test, 也不删 test, 8 个全过 = 0 漂移
3. **跟项目 test 风格一致**: `protocol_handlers::tests::*` 跟 `replay_cache::replay_cache_tests::*` 都是字面名

**风险**: 0

**Apply when**: 任何 R123+ 续 agent 想"加 test" → 0 改 8 spec test, 仅加 R123+ 自己 test

---

## 决策 #6: example 用 `apeireth_api::*` re-export 路径, 0 直接 `apeireth_api::apeireth_protocol::*`

**Context**:
- `lib.rs:137-150` 有 `pub use apeireth_protocol::{ContentPart, ..., ProtocolKind, ...}` re-export
- `apeireth_api::apeireth_protocol::*` 是 crate 内部路径, 0 在 lib.rs 暴露 (`apeireth_protocol` 不是 `pub mod`)

**选择**: example 用 `use apeireth_api::{ContentPart, MessageRole, ..., ProtocolKind}`

**理由**:
1. **0 漂移 public API**: 用 lib.rs re-export, 0 假设内部 crate 路径
2. **example 风格跟其他一致**: 现有 `hello_api.rs` 用 `use apeireth_api::llm::{...}`, 同模式
3. **0 装内部路径**: 不假装 example 能直接 `apeireth_api::apeireth_protocol::X`

**风险**: 0

**Apply when**: 0 (一次性决策, 0 改)

---

## 决策 #7: `ProtocolHandler` trait 不加 `Send + Sync` supertrait bound

**Context**:
- `HandlerRegistry` 内部 `Box<dyn ProtocolHandler + Send + Sync>` (Send+Sync 在 trait object 上, 不在 trait 上)
- 如果 trait 自身加 `Send + Sync` supertrait, 0 必要且减少 impl 灵活性

**选择**: trait 不加 supertrait, 仅 `Box<dyn ProtocolHandler + Send + Sync>` 时强制 Send+Sync

**理由**:
1. **灵活 impl**: `SyncHandler` (非 Send) 也能 impl `ProtocolHandler` (e.g. Rc<RefCell<...>> 状态)
2. **Send+Sync 强加在 registry**: registry 跨 await 必须 Send+Sync, 跟 axum State 兼容
3. **测试编译期验**: `assert_send_sync::<Box<dyn ProtocolHandler + Send + Sync>>()` 在 test #8 验过

**风险**: 0 (标准 trait object 模式)

**Apply when**: 0 (一次性决策, 0 改)

---

## 决策 #8: 不修 359 pre-existing warnings (0 装 "clean build")

**Context**:
- `cargo build -p apeireth-api` 报 359 warnings, 全部是 `missing_docs` on `v1_tools/message.rs` / `observability/health.rs` / `ws_v1.rs` 等
- 我的新 mod + example 加了 0 warnings
- 修这些 warnings = 触碰 LOCKED 文件, 违反硬约束

**选择**: 0 修 pre-existing warnings, 0 装"clean build"

**理由**:
1. **0 触碰 LOCKED**: 359 warnings 全在 R20/R121 已 commit 的文件, 0 归 R123-2
2. **0 范围扩散**: 修 359 warnings = 改 N 个文件, 严重超 1h10m 预算
3. **0 装"已修"**: 主哲学锚 #1 不假装, 现有 359 warnings 是 1.0 行为, 我 0 漂移

**风险**: 0 (warnings 不影响 build / test 0 error 0 failed)

**Apply when**: 0 (一次性决策, 0 改)

---

## 决策 #9: `ContentPart::text_only` + `Vec<ContentPart>` 字段 (从编译错误学到的)

**Context**:
- 第一次写 test 用 `ContentPart::text(...)` → 编译错 (真名是 `text_only`)
- 第一次写 test 用 `content: ContentPart::...` → 编译错 (真型是 `Vec<ContentPart>`)

**选择**: 改用 `vec![ContentPart::text_only(...)]`

**理由**:
1. **0 漂移 1.0**: 跟现有 `protocol_handlers.rs` 用 `ContentPart::text_only` + `content: vec![...]` 1:1 对齐
2. **编译期不漂移**: Rust 类型系统强校验, 0 假装"struct field 是裸 ContentPart"

**风险**: 0 (已修, 编译过, 8 test 全过)

**Apply when**: 0 (一次性错误, 0 改)

---

## 决策 #10: 不真接 `ProtocolHandler for OpenAiChat` 等 4 impl (硬约束 #8)

**Context**:
- 任务 spec 硬约束 #8: 0 装 (O-5), 仅加 trait + registry + 8 test, **不**真接替换 4 协议 handler impl
- 现有 4 协议 handler impl 在 `protocol_handlers.rs` 1950 行, 4 个独立 `*_to_normalized` / `*_from_normalized` fn

**选择**: 0 impl `ProtocolHandler for OpenAiChat` 等 4 个真接, 0 触碰现有 4 协议 handler

**理由**:
1. **0 装"已重构"**: R122-10 仅是"扫", 本任务仅"抽 trait", 真接是 R123+ 续
2. **0 触碰现有 11 公共 API**: 现有 `openai_chat_to_normalized` 等 0 改, 0 假"已 deprecated"
3. **R123+ 续明确**: 后续 agent 1 步 `impl ProtocolHandler for OpenAiChat { fn dispatch = |req| protocol_handlers::dispatch_cached_with_status(pipeline, OpenAiChat, req, Some(cache)).await }`

**风险**: 0 (0 改任何现有文件, 0 副作用)

**Apply when**: R123+ 续 agent 真接时, 0 改本 trait 任何定义, 仅加 `impl ProtocolHandler for X` 4 个 + `register` 4 行

---

**R123-2 decision log 10 条, 0 范围扩散, 0 装, 0 越界 commit, 等 R123+ 续真接.**
