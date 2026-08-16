# 自审报告 — N12 语义模型路由 + 推理字段归一化（gateway/provider 层, P1）

- **任务 ID**: d6bc5357-bbc8-4ad4-aa7a-748ff67d7c9d
- **角色**: mcp_integration_expert
- **日期**: 2026-08-16
- **提交**: `5fa725e` (N12① gateway) + provider 提交 (N12②, 紧随其后) + docs 提交

## 1. 改动文件（全部在任务包边界内）

| 文件 | 改动 |
|---|---|
| `crates/apeireth-gateway/src/semantic_router.rs` | **新增** ~830 行：语义模型路由适配件 + 17 测试 |
| `crates/apeireth-gateway/src/lib.rs` | `pub mod semantic_router` + re-export + MODULES 7→8 |
| `crates/apeireth-gateway/src/organ_kani_proofs.rs` | MODULES 断言 7→8（1 行） |
| `crates/apeireth-provider/src/reasoning_adapter.rs` | **新增** ~490 行：推理字段归一化适配件 + 22 测试 |
| `crates/apeireth-provider/src/lib.rs` | `pub mod reasoning_adapter`（2 行） |
| `crates/apeireth-provider/src/http_dispatch.rs` | 响应路径接归一化（7 行，默认配置关闭=行为 0 变化） |
| `docs/maintenance-guide.md` | 模块地图 +2 行；env 清单新增 gateway/provider 适配层段 |
| `docs/backlog.md` | N12 划 ✅ + 提交号 |

**边界自查**：未触碰 tool-runtime / tool-approval / tool-registry / context-fold；未动 apeireth-acp（只读引用 LlmFacade 类型）；git status 核对只含自己文件（期间 cargo fmt 曾波及他人文件，已全量回滚后手工重应用最小 diff）。

## 2. 机制摘要

### ① 语义模型路由（semantic_router.rs, VCP semanticModelRouter 吸收）
- 虚拟模型名：`ApeirethModelAuto`（可配）或预设名直接作为 model 字段；非虚拟名原样放行（inactive）。
- 意图选模型：最后用户消息 × 0.7 + 最后 AI 消息 × 0.3（权重可配）→ 上下文向量，与 route 描述余弦相似度排位，阈值 0.18（可配）过滤。
- 容灾链：命中 route（`failoverPool=false` 不入链）→ `defaultModel` → `fallbackModels`，去重；`dispatch` 按链序逐个调 executor，首个成功即返回，全链失败报 `AllCandidatesFailed`（附全链明细）。
- 降级路径（0 装 PASS）：总开关关/非虚拟名 → inactive 放行；空上下文/嵌入失败 → 默认计划 + 诚实 reason；单条 route 向量化失败只跳过该 route。

### ② 推理字段归一化（reasoning_adapter.rs, VCP reasoningContentAdapter 吸收）
- 12 别名（与 VCP REASONING_KEYS 1:1）：`reasoning_content / reasoning / reasoning_chunk / reasoningChunk / reasoning_summary / reasoningSummary / reasoning_details / reasoningDetails / reasoning_text / reasoningText / thinking / thoughts`。
- 递归提取（嵌套对象 TEXT_VALUE_KEYS 优先 / 数组拼接）→ 片段级去重 → `<think>...</think>` 包装（tag 归一化为 think/thinking）→ 前置于可见内容。
- 按目标模型能力下发：模型子串白名单过滤（大小写不敏感）；默认 enabled=false + 空过滤器 = 任何模型都不转换（对齐 VCP 保守默认）。
- 出向：`remove_reasoning_fields` 剥离全部别名（转发给不支持推理字段的下游前用）。
- 便捷入口 `normalize_chat_completion_body`：完整 Chat Completions 响应体归一化（message+delta 提取 → 首条 message content 重写 → 别名剥离）。

### ③ trait 口（LLM/嵌入实现注入）
- `Embedder`（async）：文本向量化注入点，真实现（MiniMax embedding 等）留部署层。
- `ModelExecutor`（async）：LLM 执行注入点，真实现（provider dispatch）留部署层。
- 本次交付以 mock 实现（KeywordEmbedder / FailingEmbedder / ScriptExecutor）完成全路径测试 = "mock 路由可测"验收达成。

## 3. 测试结果

```
cargo test -p apeireth-gateway  -j 4  → 103 passed; 0 failed（新增 17）
cargo test -p apeireth-provider -j 4  →  68 passed; 0 failed（新增 22）
cargo check 两 crate → 0 warning
```

覆盖：路由命中 / 低于阈值降级 / 容灾链构造与去重 / failoverPool 排除 / inactive 放行（非虚拟名+总开关关）/ 空上下文降级 / 嵌入失败降级 / dispatch 容灾切换 / 全链失败明细 / 配置 normalize（非法路由丢弃/去重/默认预设回退/非法 JSON 报错）/ 12 别名逐一提取 / 未知字段与非对象失败路径 / 嵌套与数组结构 / think 块包装与标签归一 / 过滤器大小写与空过滤器 / 响应体端到端 / 非法 JSON 原样返回。

## 4. 0 假装标注

| 项 | 状态 |
|---|---|
| 路由/归一化核心机制 | ✅ 已实现 + mock 全测 |
| Embedder/ModelExecutor 真实现 | **未接**（trait 口已备, 真 embedding/LLM 留部署层注入） |
| 路由器接入 Gateway 帧管线 | **未接**（gateway.rs 只做准入, LLM 路径在上游; 接线留部署层, 本任务验收标准为 mock 可测） |
| 配置热加载 | 未做（VCP 有 fs.watch; 不造轮子, 需要时走现有配置重载机制） |
| 描述向量持久缓存 | 未做（仅进程内; VCP 的 SQLite kv_store 缓存未移植） |
| 流式 SSE chunk 级归一化 | 未做（模块面向完整 JSON; 流式拼接后调用即可） |
| http_dispatch 接线 | ✅ 已接（默认配置关闭 → 现有行为 0 变化, 显式 env 开启） |
| 台账"13 别名" | VCP 源码实为 12（REASONING_KEYS 逐个数过）, 按 12 实现并在 backlog 如实注明 |

## 5. 对 VCP 的偏差（吸收时的小改进, 已留痕）

1. **跨源去重粒度**：VCP 按整源文本去重（流式 delta 与 message 累积重复时会双发 r1\nr1）；本项目按片段级全局去重，消除双发。测试 `additional_sources_merged_and_deduped` 锚定。
2. **虚拟模型默认名**：`VCPModelAuto` → `ApeirethModelAuto`（命名空间对齐）。
3. **env 前缀**：VCP `ReasoningToContentEnabled/Model/Tag` → `APEIRETH_REASONING_ENABLED/MODEL_FILTERS/TAG`（对齐 maintenance-guide env 锚点规范）。

## 6. 给集成守门员的合并提示

- `apeireth-gateway/src/lib.rs` 动了 3 处（mod 注册 / re-export / MODULES 常量 7→8 + 对应测试与 kani proof 断言）；若他人并发改 lib.rs 注意 MODULES 计数。
- `apeireth-provider/src/http_dispatch.rs` 只在响应解析处插入 7 行归一化调用（默认关闭路径），不影响既有 6 provider 测试（已全绿验证）。
- 无新增依赖（gateway 用现有 async-trait/parking_lot/serde/thiserror；provider 纯 serde_json）。
- 后续接线任务（不属本包）：真 Embedder（MiniMax embedding）与 ModelExecutor（provider dispatch）实现、companion_serve/api 层的 model 字段路由接入。
