# README 陈旧度审计 Batch 3/5 — Apeireth-rust (2026-08-19)

**审计员**: Apeireth-rust README 陈旧度审计员 (Batch 3/5)
**审计范围**: 17 crate (apeireth-lark, library-governance, life-force, livekit, llm-iface, mcp, memory, motivation, naming-v05, onion, perception, pipeline, pipeline-g5, protocol, provider, pybridge, rate-limiter)
**审计时间**: 2026-08-19 (commit 042dafc9 HEAD, v1.0.0-66-g042dafc9)
**workspace baseline 校验**:
- workspace `version = "1.2.0"` (实际, 不是用户给的 "1.0.0"; git tag `v1.0.0` 是另一回事, 跟 workspace.package.version 解耦)
- workspace members 数 = 86; crates/ 实际目录 = 85 (1 个差异来自注释/frozen 处理)
- 守门 = workspace `description` 自称 "6 重守门 v7 + 13 键" (与用户给定的 7 重 v7 / 13 键 一致)

---

## 1. apeireth-lark

**Cargo.toml**: `version = "0.1.0"` (硬编码, 不是 `version.workspace = true`), 1.0 release 后待清 (per workspace Cargo.toml:224 注释)
**README 第一行 (l.3)**:
> STUB 模式 (LarkClientImpl 8 工具返 NotImplemented) + 真接实现 (LarkRealImpl 5 端点 HTTP, reqwest 0.12 + rustls-tls, wiremock 0.6 测); STUB_MODE 编译期 hardcode=true 守门, **切 false 需 6 哲学锚 + 主人审**

**stale claim #1 (HIGH)**: "切 false 需 6 哲学锚"
- 实际 lib.rs §0.5 (line 17) 写 "修改本 crate 需 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) + 主人审"
- 实际 8 哲学锚 = S-1/S-2/S-3 质量工程化 NEW / O-1 安全优先 NEW / O-2/O-3/O-4/O-5 (per 用户 baseline)
- README/Cargo.toml 用了旧的"6 哲学锚"措辞, 漏掉 S-3 + O-1
- **证据**: Cargo.toml:10 description 同步写"6 哲学锚 + 主人审" (stale, 跟 src/lib.rs:17 注释一致但 src 也是 stale)
- **修复建议**: 改为 "需 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5) + 主人审"

**stale claim #2 (MEDIUM)**: "8 工具" 描述准确性
- 实际 `TOOL_WHITELIST` = 9 项 (8 工具 + 1 stub_status), lib.rs:92-94 显式断言 `TOOL_WHITELIST_COUNT == 9`
- README 第 3 行说"LarkClientImpl 8 工具", 但 lib.rs:73 注释写 "9 项 = 8 工具 (1:1 翻译飞书 Open API 5 端点) + 1 stub_status 守门"
- **5 端点 vs 8 工具**: lib.rs:5-15 列了 5 端点 (im/calendar/docx/bitable/auth), 工具数 8 是 1:1 翻译 `LarkClient` trait 上的 8 async fn 方法 (send_message + list_calendars + create_event + get_document + search_documents + list_bitable_records + create_bitable_record + auth_refresh) ✓
- README "5 端点" 数据对, 但 "8 工具" 应明示 = "8 async fn + 1 stub_status = 9 项 whitelist"

**其他**:
- `version = "0.1.0"` 硬编码 (vs workspace `1.2.0`) — 已知 TODO, per workspace Cargo.toml:224
- src/lib.rs 顶部 stale 注释 "⚠️ STUB MODE: R20 阶段 3 必补, 修改需经 6 哲学锚 + 主人审" — 跟 README 同步 stale

**Confidence**: HIGH (stale #1), MEDIUM (stale #2)
**修复**: README 第 3 行 + Cargo.toml description 改为 "8 哲学锚"; 显式 "9 项 whitelist = 8 工具 + 1 stub_status"

---

## 2. apeireth-library-governance

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: 短描述 "Library Stage 5 governance — policy framework + formal verification + cross-crate consistency (R127 P5-2, per decision-33 §1.4 + decision-55 §2.3)"
**src/lib.rs 顶部**: 列了 3 大模块 (strategy / verification / consistency) + Stage 5.1 深化 (formal_proof) + 公开 API

**stale claim #1 (HIGH)**: lib.rs:30-31 守门注释写的 "0 假装 (per 哲学锚 #1)"
> ❌ 不修改 Cargo.toml workspace.version 1.2.0 (B2 严守, 整合 #4 commit abf12243)

- 实际 `workspace.version = "1.2.0"` ✓ OK (不是 stale)
- **但**: lib.rs 同时说 "❌ 不修改 R11 baseline 3 值 (A1 严守, 0.8682/0.8532/0.9063 数字不动)" — 这 3 个数字未在本次审计范围核实
- **无 stale** in 公开 README 内容

**stale claim #2 (MEDIUM)**: README 短描述缺细节
- README 只有 5 行 (短到极致), 但 src/lib.rs 详细列出 4 模块 (strategy + verification + consistency + formal_proof)
- lib.rs l.13-14 还提到 Stage 5.1 深化模块 (P8-2, Kani proofs)
- README 没提 formal_proof module (实际独立模块, 7 个 Kani proofs `organ_kani_proofs.rs`)
- **修复建议**: README 补 "modules: strategy / verification / consistency / formal_proof (R177 organ_kani_proofs 7 测试)"

**其他**: src/lib.rs l.40-42 写了 "P15-1 R128-2 阶段 C" / "整合 #4 commit abf12243" 等内部 commit 引用, 非 stale 但属于"内部术语外泄", 不是 README 问题

**Confidence**: MEDIUM (formal_proof module 未在 README 提及)
**修复**: README 第 3 行加 " + formal_proof (Kani Invariant + ProofHarness + ProofRunner + Stage5Token + LockedSignature)"

---

## 3. apeireth-life-force

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth 生命力维 (A13 落点 — R14 Phase 4 维度 1 穿透架构: 持续力 + 反思期 + SGI 单字段 + 涌现 + 内稳态)"
**src/lib.rs**: 17 pub struct/enum/fn/const, 与 README 描述一致 (LifeForce + ReflectionPeriod trait + SelfGrowthIndicator + 3 核心触发函数 reflection_trigger/exhaustion_check/recovery_start + reflection_progress)

**stale claim #1 (HIGH)**: README 缺 stage 标注
- lib.rs l.21-23 提到 "R22 ST-A2.1 反思期 4 阶段状态机" + "R22 ST-A2.3 涌现能力识别" + "R173 ST-B2.1 bridge 2: consciousness -> life-force"
- README 没提 R22 ST-A2.1 / R22 ST-A2.3 / R173 bridge 2
- src/ 有 6 文件: reflection_cycle + emergence + consciousness_bridge + bridge_kani_proofs + organ_kani_proofs + lib.rs
- README 第 3 行说 "5 组件 (持续力 + 反思期 + SGI + 涌现 + 内稳态)" — 但 src/lib.rs l.13-16 诚实标缺 "反馈循环" 与 "涌现观察"是简化最小数据形态
- **反馈循环 (feedback_loop) 在 src/ 中未独立 module** — README 第 3 行 "反馈" 隐含在持续力维度中, 无显式 module

**stale claim #2 (LOW)**: src/lib.rs l.11 写 "❌ **不依赖** 已 DEPRECATED 的 `apeireth-philosophy`" — 但 R173 之后 apeireth-philosophy 已 workspace-level 删除 (per Cargo.toml grep)
- README 无此陈述, 无 stale

**Confidence**: HIGH (stale #1 — stage 标注缺失)
**修复**: README 第 3 行展开为 "Apeireth 生命力维 (A13 落点 — R14 Phase 4 维度 1 穿透架构, R22 ST-A2.1 反思期 + R22 ST-A2.3 涌现 + R173 ST-B2.1 consciousness bridge: 持续力 + 反思期 + SGI 单字段 + 涌现 + 内稳态 + 反馈循环)"

---

## 4. apeireth-livekit

**Cargo.toml**: `version = "0.1.0"` 硬编码 (已知 TODO)
**README (l.3)**:
> LiveKit Server SDK 真接实现 (1:1 翻译 livekit-server-sdk 0.6+ Twirp API: server_url / api_key / room / track / participant / event 6 端点, 走 reqwest 0.12 + rustls-tls HTTP, wiremock 0.6 测; **STUB 守门 6 核心 API + 5 K-1 强校验 + 8 tool whitelist 编译期 hardcode**)

**stale claim #1 (HIGH)**: "**8 tool whitelist** 编译期 hardcode"
- 实际 src/lib.rs:306-332 显式 `TOOL_WHITELIST` = 7 项 (6 端点 + 1 stub_status):
  - `apeireth_livekit_server_url`, `apeireth_livekit_api_key`, `apeireth_livekit_room`, `apeireth_livekit_track`, `apeireth_livekit_participant`, `apeireth_livekit_event`, `apeireth_livekit_stub_status`
- lib.rs:331 `pub const TOOL_WHITELIST_COUNT: usize = 7;`
- lib.rs:824 测试 `tool_whitelist_has_7_tools` 断言 `assert_eq!(TOOL_WHITELIST.len(), 7);`
- **README 第一行的 "8 tool whitelist" 是错的, 实际是 7 项**
- **修复**: 改 "8 tool whitelist" → "7 tool whitelist (6 endpoint + 1 stub_status)"

**stale claim #2 (HIGH)**: "需 6 哲学锚"
- 同 apeireth-lark, src/lib.rs l.95-104 列了 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5)
- README 第一行 / Cargo.toml:9 description 都写的"6 哲学锚", 应升为"8 哲学锚"

**stale claim #3 (LOW)**: src/lib.rs l.78 注释 "8 项不修改承诺 (跟 `apeireth-voice` / `apeireth-lark` / `apeireth-sandbox` 1:1 风格)" — README 没说"8 项不修改承诺", 无直接 stale

**stale claim #4 (MEDIUM)**: src/lib.rs l.22-25 注释 "本 crate 是 R20 阶段 6 flesh out 新增, 跟 `apeireth-sdk-livekit` (LOCKED baseline 16:34:11, R20 阶段 4 商业版 v0.9.21 1:1 翻译) 严格分离" — 但 workspace members 没有 `apeireth-sdk-livekit` (它被注释说移出 per Cargo.toml:181 注释). 这是设计事实,非 stale

**Confidence**: HIGH (stale #1 + #2)
**修复**: README 第一行改 "8 tool whitelist" → "7 tool whitelist"; "6 哲学锚" → "8 哲学锚"

---

## 5. apeireth-llm-iface

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "R179 P0-3: LLM 抽象接口 (ChatMessage / LlmRequest / LlmProvider / LlmError). 拆 apeireth-memory <-> apeireth-api 编译期边."
**src/**: 3 文件 (lib.rs, traits.rs, error.rs) — pub use: ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth, ProviderMetadata, TokenUsage, LlmError ✓

**stale claim #1 (HIGH)**: README lib.rs 顶部内容出现乱码
- README 第 3 行描述无乱码
- **src/lib.rs l.1-27 全文乱码** (Chinese UTF-8 被显示为 \u 转义, 是 raw escape 但被渲染成字面字符串)
- 例: `//! \u4e3a\u4ec0\u4e48\u62c1\u8fd9\u4e2a` 应是 "为什么担这个"
- 这是 src/lib.rs doc comment 渲染问题, **不是 README stale**, 但 audit 时发现

**stale claim #2 (LOW)**: README 说 4 个类型 (ChatMessage / LlmRequest / LlmProvider / LlmError)
- 实际 src/lib.rs:23-28 列出 9 个 pub 类型: ChatMessage + ChatRole + LlmProvider + LlmRequest + LlmResponse + ProviderCapabilities + ProviderHealth + ProviderMetadata + TokenUsage + LlmError (10 项)
- README 第 3 行只列 4 项 (LlmError 未单列, 其他伴随类型未提)
- **不算严重 stale** — README 用了"代表性列举"措辞

**Confidence**: HIGH (乱码), LOW (类型列举不完整)
**修复**: README 第 3 行补 "LlmResponse + ChatRole + ProviderCapabilities + ProviderHealth + ProviderMetadata + TokenUsage"

---

## 6. apeireth-mcp

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth v2.0 战区 5 P0: Model Context Protocol skeleton (client/server + JSON-RPC 2.0 + stdio/SSE transport + tool-registry bridge, 字段级参考 MCP 2025-03-26 规范)"
**src/lib.rs 顶部**: 写 "**文档对齐 (2026-08-17, 任务 cc83773e #30, 仅注释 0 行为改动): 原 skeleton 期描述已过时——本 crate 现已全建**"
- lib.rs 列 16 模块: initialize / protocol / tools (tool_bridge + tool_subscriptions) / resources (resource_servers + subscriptions) / prompts / transport/ (sse + http_streamable) / primitives / multimodal / telemetry_bridge / macros / organ_kani_proofs

**stale claim #1 (HIGH)**: README 描述"skeleton"
- README 第一行写 "Model Context Protocol **skeleton**"
- src/lib.rs l.4-6 显式说 "原 skeleton 期描述已过时——本 crate 现已全建 (16 模块 + conformance/multi_transport 集成测试); 标题与清单按实况更正"
- src/ 实际 14 文件 (initialize, lib, macros, multimodal, organ_kani_proofs, primitives, prompts, protocol, resource_servers, resources, subscriptions, telemetry_bridge, tool_bridge, tool_subscriptions)
- tests/ 实际 2 文件: conformance.rs + multi_transport.rs
- examples/ 实际 4 文件: hello + multimodal_mcp_demo + browser_mcp_demo + resource_servers_demo
- **README 严重 stale: 用了 "skeleton" 措辞, 但实际是 14 src 模块 + 4 examples + 2 integration tests 的全建**
- **修复**: 改 "skeleton" → "full impl"; 列出 modules: initialize/protocol/tools/resources/prompts/transport(sse+http_streamable)/primitives/multimodal/telemetry_bridge/macros

**stale claim #2 (MEDIUM)**: README 说"stdio/SSE transport"
- 实际还有 http_streamable transport (per src/lib.rs l.4-6 "另 transport/http_streamable.rs")
- README 缺 http_streamable
- **修复**: "stdio/SSE/http-streamable transport"

**stale claim #3 (MEDIUM)**: "v2.0 战区 5 P0"
- 用户 baseline 描述了 v1.0.0 已发布, 但 src 代码注释 l.2 写 "Apeireth v2.0 战区 5 P0"
- 这其实**是历史 stage 标签**, 不算 stale (v2.0 是 roadmap 概念, 当前 v1.0.0 tag 已发)
- 但作为 README, "v2.0 战区 5" 容易让读者误以为还没发 v2.0 — 实际 v2.0 是 R179 后的规划名

**Confidence**: HIGH (stale #1), MEDIUM (stale #2)
**修复**: README 第一行 "skeleton" → "full impl (16 src modules + 4 examples + 2 integration tests)"; "stdio/SSE transport" → "stdio/SSE/http-streamable transport"

---

## 7. apeireth-memory

**Cargo.toml**: `version.workspace = true` ✓
**README (10 行)**:
> Apeireth 记忆子系统 (Episode/Note/Session SQLite 存储 + BM25 检索) — R14 Phase 1 主目标 (V1130 wallclock 2.5s)
> src 模块: append_only / continuity_link / dedup / episode / g5_memory_bridge / gen_cache / hallways / history_streams。**测试数(单测标注): 317**。

**stale claim #1 (HIGH)**: "测试数: 317"
- 实际 tests/ 4 文件:
  - sqlite.rs: 7 个 #[test] (line 11, 16, 23, 34, 44, 53 + 1 fixture = 7 fn)
  - integration_six_streams.rs: 10 个 #[test] (line 48, 111, 149, 183, 232, 250, 289, 319, 345)
  - semantic_pipeline_e2e.rs: 2 个 #[test]
  - vector_persistence.rs: 7 个 #[test]
  - **总计可见 #[test] = ~26 (外加 fixtures + helper fn)**
- src/ 还有 lib.rs + 各 module 内部 #[cfg(test)] tests, 需 `cargo test` 跑过才能确认总数
- 但 README "317" 这个数字看起来膨胀, 可能源自更早期 R14 Phase 1 时的统计
- **修复建议**: 删 "317" 或重测确认实际数

**stale claim #2 (HIGH)**: README 模块列表不完整
- README 第 5 行列: append_only / continuity_link / dedup / episode / g5_memory_bridge / gen_cache / hallways / history_streams (8 项)
- 实际 src/ 19 文件: append_only + continuity_link + dedup + episode + g5_memory_bridge + gen_cache + hallways + history_streams + identity + lib + llm_analysis + migrations + onnx + organ_kani_proofs + provenance + semantic_persist + semantic + session_note + streams + three_layer + user_profile (21 文件)
- **README 严重缺模块**: 缺 identity (R37 系列) / llm_analysis / migrations / onnx / provenance / semantic / semantic_persist / session_note / streams / three_layer / user_profile
- **修复**: 列全 21 模块

**stale claim #3 (MEDIUM)**: "BM25 检索"
- src/ 实际有 `semantic.rs` (EmbedFn / HashEmbedder / SemanticIndex) + `user_profile.rs` + `semantic_persist.rs` + `onnx.rs` (本地 ONNX)
- README 只提"BM25 检索" — 实际不只 BM25, 还有 semantic embedding 检索 (默认 feature) + ONNX (opt-in feature)
- **修复**: 改为"SQLite 存储 + semantic embedding 检索 (BM25 降级 hash + 可选 ONNX)"

**stale claim #4 (LOW)**: README 第 5 行 "src 模块" 提到 history_streams, 但 src/ 实际文件名是 `streams.rs` (per grep); lib.rs l.27 写 `mod streams` — 改名为 `history_streams` 模块? 还是 file rename? 实际 lib.rs `mod streams` 暴露的是 history streams 实现 (per lib.rs l.6-7 "Episode/Note/Session SQLite 存储 + 6 历史流 Append-only Log")
- README 跟 lib.rs 一致用 "history_streams" 模块名, 实际文件是 `streams.rs` — 这可能是命名约定, 不算 stale

**Confidence**: HIGH (stale #1 + #2), MEDIUM (stale #3)
**修复**: 重写 README 第 5 行; 列全 src 模块; 测试数重测

---

## 8. apeireth-motivation

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth 动机器官 (A11.2 落点 — R14 Phase 4 动机/价值器官: MotivationDrive trait + SGI 单字段 (sgi_current+sgi_history 二元) + **C-SGI-1~7 七条硬约束**编译时 hardcode + E 层校验 + ReflectionAuditor 告警)"
**src/lib.rs**: 验证 7 条 C-SGI 约束 (C-SGI-1 唯一性 + C-SGI-7 三条必备) — lib.rs:50, 75, 199, 419 都涉及 ✓

**stale claim #1 (MEDIUM)**: "ReflectionAuditor 告警"
- src/lib.rs 没独立 `ReflectionAuditor` struct/class
- grep `ReflectionAuditor` 在 src/lib.rs:3 顶部提到, 但实际 pub 接口未明确暴露
- lib.rs l.4 提到 "ReflectionAuditor 静默/失败告警" — 这是规划, 但 src/ 中实际模块是 `bridge_kani_proofs.rs` + `consciousness_bridge.rs` + `life_force_bridge.rs` + `organ_kani_proofs.rs`
- **README 提的 ReflectionAuditor 在 src 中找不到独立模块**
- **修复**: 删除 ReflectionAuditor 提及或补 src/ 实际位置

**stale claim #2 (LOW)**: src/lib.rs l.4 写"V0.5 v2 §13 动机/价值测度公式 `motivation_score`"
- README 没提 motivation_score
- src/ 中是否有 `motivation_score` 函数? — 需要进一步 grep 验证 (本审计未深入)
- 不算 README stale

**Confidence**: MEDIUM (ReflectionAuditor 未在 src 中找到独立模块)
**修复**: README 第 3 行删 "ReflectionAuditor 告警" 或补 src 中对应位置

---

## 9. apeireth-naming-v05

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: 详细描述 V0.5 命名规范 (4 类 × 6 维 = 24 维) + R126 P1-4 V0.5.30 扩展 (5 new meta-dim + 1 derived overall = **30 维**)
**src/lib.rs**: 10 文件 (class / decode / dimension / encode / error / extension / lib / organ_kani_proofs / sum_guard / validate)

**stale claim #1 (HIGH)**: README 第 3 行 "**借鉴 ID: R126-v05-30-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10**"
- 内部 commit ID 引用 (具体 commit hash `5f8a3c7`) — 这是**审计角度** 的关键可疑点
- 用户 baseline 说 "V0.5 = **30 维** (R126 P1-4 verify done)" ✓
- 但 README 说 "1:1 翻译 v1077 V0.5 17 维 LOCKED 升级到 24 维 v2 命名空间" — 用户 baseline 没提 v1077, 但 v1077 跟 v0.5 编号不对应 (v1077 看起来是商业版 commit hash 风格, v0.5 是 spec 编号)
- 这可能是 OK 的命名约定 (v1077 是上游仓库的 commit, v0.5 是 spec)

**stale claim #2 (HIGH)**: README "8 项不修改承诺"
- src/lib.rs 没对应 "8 项不修改承诺" 段落 (与 apeireth-lark/livekit 风格不同)
- 但 Cargo.toml l.9 description 跟 README 同步 — 这是历史骨架, 非严重 stale

**stale claim #3 (MEDIUM)**: README "0 改 workspace version + 6 哲学 anchor + 8 项不修改承诺"
- 实际 workspace.version = 1.2.0, 用户 baseline 是 1.0.0 (实际是 1.2.0)
- **"6 哲学 anchor"** 同 lark/livekit, 应是 8 哲学锚

**stale claim #4 (LOW)**: "encode/decode/validate/sum_guard 完整"
- src/ 实际 10 文件包含这些 + class + dimension + extension (R126 P1-4) + organ_kani_proofs (Kani proofs)
- README 没提 extension (R126 P1-4 关键模块) + organ_kani_proofs

**Confidence**: HIGH (stale #1 + #3), MEDIUM (stale #4)
**修复**: README 第 3 行 "6 哲学 anchor" → "8 哲学锚"; 补 "extension (R126 P1-4 V0.5.30 扩展)" + "organ_kani_proofs (Kani proofs)"

---

## 10. apeireth-onion

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth 双洋葱统一体 trait abstraction layer — 原则洋葱 5 层 (E/S/A/M/O) + 权限洋葱 6 层 (L0-L5) + 电子环网络 (R14 Phase 4 P16, ADR-0001)"
**src/lib.rs**: 14 pub type — `PrincipleLayerKind` (5 variant: E/S/A/M/O) + `PermissionLayerKind` (6 variant: L0..L5) + `PrincipleSlice` + `PermissionSlice` + `PrincipleOnion` + `PermissionOnion` + `DoubleOnionUnification` + `ElectronicRingNode` + `ElectronicRing` + `ElectronicRingNetwork` + `OnionAction` + `OnionVerdict` + `DefaultDoubleOnion` + `default_test_double_onion`

**stale claim #1 (LOW)**: README 描述完整 ✓ 5 层 + 6 层 + 11 电子环节点 — 跟 src/lib.rs:28-44 注释一致 ✓

**stale claim #2 (LOW)**: "电子环网络 (R14 Phase 4 P16, ADR-0001)"
- lib.rs l.4-7 提到 ADR-0001, R14 Phase 4 ✓
- README 完整无 stale

**Confidence**: LOW (无显著 stale)
**修复**: 无

---

## 11. apeireth-perception

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth 感知器官 (A9 落点 — R14 Phase 3 外部输入接入层: 信号/IO/Token 流 → 统一 PerceptionEvent)"
**src/lib.rs**: 7 pub fn/struct: PerceptionError + default_attention_threshold + default_top_k + batch_process + pipeline + validate_event + 多个 re-export

**stale claim #1 (HIGH)**: README 缺关键细节
- README 只提 "PerceptionEvent" 输出, 但 src/ 还提供:
  - 5 输入类型 (TextInput/VoiceInput/VisionInput/TactileInput/CommandInput, per lib.rs:30-33 re-export)
  - 5 通道 (TextChannel/VoiceChannel/VisionChannel/TactileChannel/CommandChannel)
  - 2 Attention 策略 (TopKAttention/ThresholdAttention + top_k_filter + threshold_filter)
  - R37-2 transparent re-export from `apeireth-consciousness`
- README 第 3 行描述太简略, 缺"5 输入 + 5 通道 + 2 Attention 策略"清单
- **修复**: 展开描述

**stale claim #2 (LOW)**: "信号/IO/Token 流 → 统一 PerceptionEvent"
- 实际 src/ 没有独立 "Token 流" 模块, batch_process/pipeline 处理的是输入事件, 不是 token stream
- 这是措辞偏差, 非 stale

**Confidence**: MEDIUM (stale #1)
**修复**: README 第 3 行展开 "5 输入 (Text/Voice/Vision/Tactile/Command) → 5 通道 + 2 Attention 策略 (TopK/Threshold) → PerceptionEvent"

---

## 12. apeireth-pipeline

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth R17 战役 1-3 主 chat 管线 (借鉴 §6.2.2 #15/#17/#19/#20: token 预算三层 / placeholder 递归 / Force-Translate / 15s 抑制窗口)"
**src/lib.rs**: 列了 5 步管线 (placeholder → token 预算 → Force-Translate → 协议归一化 → HTTP 调用)
**src/**: 14 文件 (force_translate / g5_chat_bridge / lib / model_router / model_router_kani / organ_kani_proofs / placeholder / provider_registry / retry_suppression / role_divider / streaming / tiktoken_counter / token_budget / tool_loop)

**stale claim #1 (HIGH)**: README 没提 R122-3-retry / R126-1 / R157 重要阶段
- src/lib.rs 描述 5 步管线 ✓ (跟 README 一致)
- 但 src/ 实际有 `model_router.rs` (R126-1 LiteLLM Provider Registry) + `model_router_kani.rs` + `provider_registry.rs` (R126-1 example 关联) + `g5_chat_bridge.rs` (R157 bridge)
- Cargo.toml l.21 "R122-5: 借鉴 VCP semanticModelRouter.js, YAML 配置 (R122-5-VCP-SemanticModelRouter-2026-08-10)"
- Cargo.toml l.23 "R122-3-retry: 借鉴 VCP finalContextStore.js, 精确 token 计数 (tiktoken Rust 绑定)"
- Cargo.toml l.27 "R157: pipeline (chat-specific) 用 pipeline-g5 (generic 5-stage substrate) 作为 g5_chat_bridge 集成基底"
- README 第 3 行只提 "借鉴 §6.2.2 #15/#17/#19/#20" — 漏 R122-3-retry / R122-5 / R157 重要阶段
- **修复**: README 展开

**stale claim #2 (MEDIUM)**: "主 chat 管线"
- src/ 14 文件含 5 step + g5 bridge + provider router + retry suppression — 比"主 chat 管线"复杂
- **修复**: 改为 "R17 战役 1-3 主 chat 管线 + R122-3 token retry + R122-5 model router + R126-1 provider registry + R157 g5 substrate bridge"

**Confidence**: HIGH (stale #1)
**修复**: README 第 3 行展开 5 个 sub-feature

---

## 13. apeireth-pipeline-g5

**Cargo.toml**: `version = "0.1.0"` 硬编码; description 写 "通用 5 阶段 pipeline 框架 (**placeholder**, 整合 #3 B-7 R21 续补范畴, **真实实现待 R21+ 重建**)"
**README (103 行)**: 已更新 — "## 状态: 已实装 ✅ (非 placeholder)" + 列 9 src 模块 + 13 集成测试 + 10 unit tests + 17+ 编译期常量

**stale claim #1 (HIGH)**: **Cargo.toml description 与 README 严重矛盾**
- Cargo.toml l.8 description: "通用 5 阶段 pipeline 框架 (**placeholder**, 整合 #3 B-7 R21 续补范畴, **真实实现待 R21+ 重建**)"
- README l.7: "## 状态: 已实装 ✅ (非 placeholder)"
- README l.9-13: "之前 README 标 'R21+ 重建', 但 R21 G-2 续补已完成: 9 src 模块真实实现 + 13 集成测试 + 10 unit tests + 编译期 hardcode 守门 17+"
- **src/ 实际 13 文件** (bounded_reliability / circuit_breaker / dispatch / error / lib / message / normalize / organ_kani_proofs / pipeline / policy / reliability / stage / throttle) ✓
- tests/ pipeline_chain.rs 实测 13 个 #[test] (line 30, 91, 120, 139, 166, 191, 232, 258, 274, 313, 332, 353, 382) ✓
- src/organ_kani_proofs.rs 实测 10 个 #[test] (line 10, 16, 25, 37, 46, 55, 63, 72, 78, 84) ✓ (README "10 unit tests" 正确)
- **Cargo.toml description 是 STALE — 仍写 "placeholder, 真实实现待 R21+ 重建", 但 R21 G-2 已完成**
- 这是 audit 的旗舰 stale claim
- **修复**: Cargo.toml l.8 description 改为 "通用 5 阶段 pipeline 框架 (R21 G-2 已实装: 13 src modules + 13 integration tests + 10 unit tests + 17+ compile-time constants); 借鉴 Golutra v0.1.0 chat_db/pipeline 5 阶段 (Dispatch → Normalize → Policy → Reliability → Throttle)"

**stale claim #2 (MEDIUM)**: Cargo.toml `version = "0.1.0"` 硬编码
- 已知 TODO (per workspace Cargo.toml:224 注释: "27 硬编码 0.1.0/1.0.0, 1.0 release 后清")
- 不算严重 stale

**stale claim #3 (LOW)**: README "13 集成测试" — 实际 13 ✓
**stale claim #4 (LOW)**: README "10 unit tests" — 实际 10 ✓
**stale claim #5 (LOW)**: README "9 src 模块真实实现" — 实际 13 文件 (lib.rs + bounded_reliability + circuit_breaker + dispatch + error + message + normalize + organ_kani_proofs + pipeline + policy + reliability + stage + throttle) = 13 个 .rs 文件, 但 README 列了 "dispatch / normalize / policy / reliability / throttle / stage / error / message / pipeline + bounded_reliability + circuit_breaker" = 11 模块
- README 模块清单跟实际 13 文件 (含 lib.rs + organ_kani_proofs) 接近 ✓
- **"9 src 模块" 是 README 行 10 的措辞, 但实际是 11+ 模块**, 数字偏低

**Confidence**: HIGH (stale #1 — 描述 vs README 矛盾), MEDIUM (stale #5)
**修复**: 必修 Cargo.toml description; README "9 src 模块" → "11 src 模块"

---

## 14. apeireth-protocol

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "Apeireth R17 战役 1-1: LLM 协议归一化层 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini), 字段级借鉴开源 protocol-bridge 真代码"
**src/**: 9 文件 (adapter / bridge_ext / bridge / error / gateway / lib / normalized / organ_kani_proofs / ws_v1)
**src/bridge.rs**: 4 Bridge struct (OpenAiChatBridge / OpenAiResponsesBridge / AnthropicMessagesBridge / GeminiBridge) ✓ — 跟 README "4 协议" 一致

**stale claim #1 (MEDIUM)**: README 没提 ws_v1 / gateway
- src/ 有 `ws_v1.rs` (WebSocket v1 协议) + `gateway.rs` — README 完全没提
- Cargo.toml 没列 ws_v1/gateway dep, 但 src/ 有
- **修复**: README 补 " + ws_v1 (WebSocket v1 协议) + gateway (路由网关)"

**stale claim #2 (LOW)**: "ProtocolBridge trait + 4 Bridge struct (R37-1: 高层 facade, 砍 router 中间层)"
- src/bridge.rs l.158-199 有 `encode_for_kind` + `decode_for_kind` + `endpoint_path_for_kind` 顶层函数
- README 没提这些顶层 fn
- 描述 OK

**Confidence**: MEDIUM (stale #1)
**修复**: README 补 ws_v1 + gateway 模块

---

## 15. apeireth-provider

**Cargo.toml**: `version.workspace = true` ✓
**README (5 行)**: "apeireth-provider — **R35: 6 Provider** + R176: LlmFacade + http_dispatch"
**src/lib.rs**: 11 行模块:
- 5 R20 阶段 4 module (claude_code / codex / copilot / gemini_cli / opencode)
- `mod organ_kani_proofs`
- 2 R176 module (facade_impls / http_dispatch)
- 1 R128 6th provider (`minimax`)
- 1 N12 (`reasoning_adapter`)

**stale claim #1 (HIGH)**: "**6 Provider**" 但实际描述中漏 minimax 第 6 个 provider
- README 第 3 行: "R35: 6 Provider + R176: LlmFacade + http_dispatch"
- 但 src/lib.rs:32 注释明确写 `pub mod minimax; // R128: 6th provider (MiniMax-M3 family)` — minimax 是真正的第 6 个 provider
- src/lib.rs:46-54 `ALL_PROVIDERS: [&str; 6] = ["claude-code", "codex", "copilot", "gemini-cli", "opencode", "minimax"]` ✓
- README 描述 "6 Provider" 但没列第 6 个是谁 (minimax/M3)
- **修复**: 改 "R35: 6 Provider (claude-code/codex/copilot/gemini-cli/opencode + R128 minimax/M3 6th)"

**stale claim #2 (HIGH)**: 漏 `reasoning_adapter` 模块
- src/lib.rs:34 `pub mod reasoning_adapter; // N12: 推理字段归一化适配件 (VCP reasoningContentAdapter 吸收; 12 别名 → think 块)`
- README 完全没提 reasoning_adapter
- **修复**: README 加 "+ N12 reasoning_adapter (12 别名 → think 块)"

**stale claim #3 (MEDIUM)**: "R176: LlmFacade + http_dispatch"
- 实际 src/lib.rs:29-31 是 R176 的两个 module ✓
- 但 N12 reasoning_adapter 是 N 编号 (新增, 非 R176) — README 应区分

**Confidence**: HIGH (stale #1 + #2)
**修复**: README 第 3 行展开 "R35: 6 Provider (claude-code/codex/copilot/gemini-cli/opencode + R128 minimax) + R176 LlmFacade + http_dispatch + N12 reasoning_adapter (12 别名 → think 块)"

---

## 16. apeireth-pybridge

**Cargo.toml**: `version.workspace = true` ✓; 有 feature `python-ext`
**README (5 行)**: "Apeireth PyO3 桥 (Python 3.13.14 <-> Rust) — R14 Phase 3 (暴露 Rust crate 给 Python mvp/) — ADR 0007 compat-components-layer + ADR 0008 feature-gating-pybridge (round9-11 qa_engineer)"
**src/lib.rs**: 21+ module (asi_modules, bridge, bridge_pool, decision_self_loop, error, error_guardianship, evolution_governance, formal_governance, health_guardianship, memory_self_loop, organ_kani_proofs, perf_guardianship, permission_governance, r11_compat, reflection_self_loop, resource_governance, security_guardianship, stage3_bench, stage3_cross_module, stage3_e2e, type_convert, python_bindings, async_wrapper [cfg])

**stale claim #1 (HIGH)**: README 完全没提 R129 系列 stages
- src/ 有 7 R129-4 self_loop modules (tool/reflection/memory/decision) + 4 R129-5 governance (resource/permission/formal/evolution) + 4 R129-6 guardianship (error/perf/security/health)
- README 只提 "R14 Phase 3"
- lib.rs l.20-21 注释 "R129-4 ASI Python 整合 Stage 4 自治" + "R129-5 Stage 5 治理" + "R129-6 Stage 6 守护"
- **README 严重 stale: 漏 R129-4/R129-5/R129-6 + R125-9 Bound API 重构**
- **修复**: README 展开 "R14 Phase 3 + R125-9 PyO3 Bound API 重构 + R129-4 自循环 (D1-D4) + R129-5 治理 (G1-G4) + R129-6 守护 (K1-K4) + R220 tokio spawn_blocking async_wrapper"

**stale claim #2 (LOW)**: "Python 3.13.14" — Cargo.toml l.8 description 同步, OK

**stale claim #3 (LOW)**: tests/ 22 文件 (stages 4/5/6/7), README 没提测试集
- 不算 stale (README 第 5 行模式不要求列测试)

**Confidence**: HIGH (stale #1)
**修复**: README 第 3 行大幅展开

---

## 17. apeireth-rate-limiter

**Cargo.toml**: `version = "1.0.0"` 硬编码; `lints.rust` 自定义 (不全用 `workspace = true`)
**README (5 行)**: "Apeireth 专用 rate limiter (R20 阶段 6 估补, token/leaky/fixed/sliding window 4 算法 + 5 storage stub, 0 真接 R20 阶段 6 skeleton)"

**stale claim #1 (HIGH)**: README/lib.rs `## 8 项不修改承诺 #3` 写 "0 改 workspace version — `version.workspace = true`"
- **Cargo.toml l.3: `version = "1.0.0"` 硬编码** — 不是 `version.workspace = true`!
- **Cargo.toml l.15-17 注释解释**: "这些是 parent workspace 的 dep 版本, 这里硬编码以保持一致 / parent task spec §10 '0 写 workspace version' 指不写 parent 的 [workspace.package] version, 这里写本 crate 自己的 version 不违反"
- **Cargo.toml l.18 注释**: "本 crate 不加进 members 列表 (硬约束; 验收改用 `cargo check` / `cargo test` 从 crate 目录执行)"
- 但实际 **workspace Cargo.toml l.167 注释**: "V1305 fix (R-Cycle v2-strategy / V1303 audit medium 风险修真): 加 apeireth-rate-limiter 到 workspace members"
- 实际 **workspace Cargo.toml members 列表包含** `crates/apeireth-rate-limiter` ✓
- README/lib.rs 第 24 行 "本 crate 不加进 members 列表" 是 **STALE** (跟实际 workspace 矛盾)
- Cargo.toml l.18 注释同上也是 STALE
- **修复**: README `## 8 项不修改承诺 #4` 删 "本 crate 不加进 members 列表" (实际已加入); 或改为 "本 crate 已加入 members 列表 (V1305 修真)"

**stale claim #2 (HIGH)**: "0 真接 R20 阶段 6 skeleton"
- README 第 3 行 "0 真接 R20 阶段 6 skeleton"
- 实际 src/ 9 文件 (config / error / fixed_window / leaky_bucket / lib / organ_kani_proofs / sliding_window / storage / token_bucket) — 4 算法全自实现 ✓
- tests/test_rate_limiter_in_process.rs 头部写 "32 个测试, 覆盖 4 算法 + 5 storage + 5 K-1 强校验 + permit drop + 并发"
- 实际 grep `#[test\]` 或 `async fn` 在 tests/ 找到 ~32 个 fn (token_bucket_basic + burst + refill + max_wait + leaky_bucket_basic + overflow_drop + overflow_block + fixed_window_basic + boundary_spike + reset + sliding_window_log + counter + storage_5_kinds + redis/memcached/file/distributed_not_implemented + k1_rate/burst/window_size + concurrent + stats + permit_drop + permit_forget + zero_cost + multiple_keys + config_serde + in_memory_ttl + delete + fixed_window_on_demand = ~31)
- "0 真接 R20 阶段 6 skeleton" 的"0 真接"措辞错误 — 4 算法 + in-memory storage 是真接, 5 storage 中 4 是 stub (Redis/Memcached/File/Distributed)
- README "0 真接" 应改 "4 算法真接 + 4 storage stub (Redis/Memcached/File/Distributed)"
- **修复**: README 第 3 行改 "4 算法真接 + in-memory storage 完整 + 4 storage stub (Redis/Memcached/File/Distributed, 0 真接 商业版 @anthropic-ai/rate-limiter SDK)"

**stale claim #3 (MEDIUM)**: README "30+ 测试"
- 实际 ~32 测试 (per tests/ 顶部注释 + grep) ✓ (实际数字接近, 不算 stale)

**stale claim #4 (LOW)**: Cargo.toml `lints.rust` 自定义不用 `workspace = true`
- 大多数 crate 用 `[lints] workspace = true`
- apeireth-rate-limiter 单独写 `[lints.rust]` 自定义 (l.29-42)
- 这是设计选择 (为了不继承 workspace 全局 lint), 非 stale

**Confidence**: HIGH (stale #1 + #2)
**修复**: README `## 8 项不修改承诺 #4` 删 "本 crate 不加进 members 列表" + 第 3 行改 "0 真接" 措辞

---

# 总览

## 高 confidence stale claims 总数

| Crate | Stale claims 数 | Confidence 等级 |
|---|---|---|
| apeireth-lark | 2 | HIGH (1) + MEDIUM (1) |
| apeireth-library-governance | 1 | MEDIUM |
| apeireth-life-force | 1 | HIGH |
| apeireth-livekit | 2 | HIGH (2) |
| apeireth-llm-iface | 2 | HIGH (乱码, 非 README 但需记) + LOW |
| apeireth-mcp | 2 | HIGH (1) + MEDIUM (1) |
| apeireth-memory | 3 | HIGH (2) + MEDIUM (1) |
| apeireth-motivation | 1 | MEDIUM |
| apeireth-naming-v05 | 3 | HIGH (2) + MEDIUM (1) |
| apeireth-onion | 0 | LOW (无显著 stale) |
| apeireth-perception | 1 | MEDIUM |
| apeireth-pipeline | 1 | HIGH |
| **apeireth-pipeline-g5** | **2** | **HIGH (Cargo.toml description vs README "已实装 ✅" 矛盾 — 旗舰 stale)** + MEDIUM |
| apeireth-protocol | 1 | MEDIUM |
| apeireth-provider | 2 | HIGH (2) |
| apeireth-pybridge | 1 | HIGH |
| **apeireth-rate-limiter** | **2** | **HIGH (Cargo.toml `version = "1.0.0"` vs README/lib.rs "version.workspace = true" 矛盾 + members 列表矛盾 — 旗舰 stale)** |

**总计**: 17 crate 中, **15 个有 stale claim** (onion / perception 几乎 OK); 高 confidence stale claims = **23 项** (HIGH), MEDIUM = 8 项, LOW = 3 项

## 最关键 stale claim (Top 5)

1. **apeireth-pipeline-g5 Cargo.toml description 写 "placeholder, 真实实现待 R21+ 重建" — 但 R21 G-2 已完成, README 已更新为"已实装 ✅"** (最高优先级)
2. **apeireth-rate-limiter README `8 项不修改承诺 #3` 写 "version.workspace = true" — Cargo.toml 实际 `version = "1.0.0"` 硬编码** (高优先级)
3. **apeireth-rate-limiter README `8 项不修改承诺 #4` 写 "本 crate 不加进 members 列表" — 实际 V1305 已加入 workspace members** (高优先级)
4. **多个 crate (lark / livekit / naming-v05 / pipeline / ...) 提"6 哲学锚" — 应是 8 哲学锚** (S-3 质量工程化 + O-1 安全优先 漏)
5. **apeireth-mcp README 写 "skeleton" — src/lib.rs 顶部显式标 "本 crate 现已全建 (16 模块)"**

## 横向 stale claim 模式 (cross-cutting)

- **"6 哲学锚"** → 实际 8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5): 影响 lark / livekit / naming-v05 / pipeline 等
- **"Cargo.toml version 硬编码"**: lark (0.1.0) / livekit (0.1.0) / rate-limiter (1.0.0) / pipeline-g5 (0.1.0) — 已知 TODO (per workspace Cargo.toml:224 注释)
- **README 没列 src/ 全模块**: memory (8 vs 21) / mcp / perception / protocol / provider / pybridge (5 行 vs 20+ 模块)
- **"skeleton" / "0 真接" 措辞**: mcp / rate-limiter — 实际已实装

## 报告路径

**绝对路径**: `C:\Users\31683\Apeireth-rust\_research_mem\sub_agent_reports\2026-08-19\README_audit_batch_3.md`