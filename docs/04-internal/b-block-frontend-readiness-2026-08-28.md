# B 块 frontend 对接 真实施 readiness (2026-08-28)

**作者**: Sub-Agent (主代理 Mavis 派) | **HEAD**: `22c6e72b` (per R13 review §4.2)

---

## 0. 任务 brief 偏差纠正

| brief 写 | 实际 | 影响 |
|---|---|---|
| `crates/engine/gateway/` | `crates/adapters/gateway/` | 路径错 |
| `runtime.ts` 指 :8090 | `frontend/companion-desktop/src/lib/runtime.ts`, `baseUrl` 配置型默认 `:3000` | 路径 + 事实错 |
| WS protocol v1 per 12 slot | `ws_v1.rs` = R20 阶段 2 蓝图 (web_search/file_ops 8 帧), **跟 9 organ + 12 slot 无关** | 概念错, WS v1 不在 B 块 4-6 周 |

5/5 中 3 处偏差. **不要按 brief 字面派单**.

---

## 1. v2 gateway 当前 3 路由 (`canonical_entry.rs:168-174`)

| # | Method | Path | Handler | 真生产可用? |
|---|---|---|---|---|
| 1 | GET | `/health` | L184-189 → `{"status":"ok","execution_owner":"apeireth-runtime::canonical"}` | ✅ |
| 2 | POST | `/v1/chat` | L191-202 (`native_chat`, `CanonicalChatRequest` L20-33) | ⛔ internal only |
| 3 | POST | `/v1/chat/completions` | L204-265 (`openai_chat`, OpenAI Chat 兼容) | ✅ **但仅非流式** |

**HTTP error 映射** (L267-290): `Denied → 403`, `ApprovalRequired → 409`, `NoProvider/NoHealthyProvider/Misconfigured → 503`, `Provider/ProvidersExhausted → 502`. **Adapter 不持有** provider routing, governance, sessions, tool dispatch (L1-6 注释).

---

## 2. frontend runtime.ts 真账 (1411 行, 非 brief 所述)

**不指 :8090**: `loadConfig()` (L126-169) 默认 `baseUrl = 'http://127.0.0.1:3000'`, Pattern 项目移植来的 OpenAI-compatible adapter.

**调用 20+ 端点, v2 gateway 实际只实现 3 个**:

| 端点 | 调用点 | v2 gateway 有? |
|---|---|---|
| `GET /health` | L196-203 | ✅ |
| `POST /v1/chat/completions` (流式 + 非流式) | L356-579, L582-599 | ✅ 但**仅非流式**, runtime.ts 流永远立即结束 |
| `GET /v1/models` | L233-246, L335-341 | ⛔ |
| `GET /v1/panel/{sessions,memory/streams,memory/episodes,graph,audit,traces}` (6 个) | L255-310, L815-1411 | ⛔ |
| `GET /v1/tools/list` / `GET /v1/panel/tools` | L949-978 | ⛔ |
| `GET /v1/apeireth/capabilities` | L683-708 | ⛔ (有 legacy fallback) |
| `GET /v1/apeireth/events` (SSE) | L1146-1201 | ⛔ |
| `POST /v1/memory/append` | L1052-1070 | ⛔ |
| `POST /v1/apeireth/{sessions ×5, memory/episodes ×4, grant, grants/revoke}` | L1309-1381 | ⛔ |
| `GET /v1/apeireth/{approval-requests, grants}` | L982-1013, L1366-1376 | ⛔ |
| `GET /v1/organs` | L1109-1118 | ⛔ |

**缺**: 9 organ stream hook, 治理 hook 透传 (403/409 区分), 主人审批 modal 触发, Tauri shell 真接, keyring 集成.

**Tauri shell 边界** (README L120-123): 0 apeireth-* dep, 独立 workspace, `cargo test --workspace` 不碰. **B 块真实施 = 纯 Svelte 5 UI + runtime.ts 改造, 不改根 workspace Rust.**

---

## 3. R9 spec vs 当前实现 gap

| gap | R9 spec 标 | v2 gateway 真账 | frontend 真账 | 估时 |
|---|---|---|---|---|
| **#1 SSE 路径** | R9 §4.1+§4.3 | **0 装**, 3 路由全非流式 | `streamChat()` (L356-579) 真写 SSE 解析, 但后端**永远返完整 JSON** | 1-2 周 (gateway 加 `Accept` 检测 + mpsc) |
| **#2 9 organ stream hook** | R9 §4.3 | runtime `execute()` (L99) 单 loop, **不串联 OrganOrchestrator** | 0 organ 帧解析 | R12 working tree 续跑 1-3 周 + gateway 改造 1-2 周 |
| **#3 治理 hook 透传** | R9 §7.1 | HTTP error 映射**已写** (L267-290) | `classifyHttpError()` (L92-97) 把 401/403/404/5xx 全归 `'auth'`/`'http'`, **不区分** ApprovalRequired vs Denied | frontend 1 周 + gateway `/v1/apeireth/grant` 端点 3 天 |
| **#4 缺失端点** | R9 §2.4 提 3 个 | 0 装 | 0 装 | panel read-only 6 端点 1 周, capabilities 3 天, events SSE 1 周, grant 3 天, session CRUD 1 周, memory CRUD 1 周, Whisper 2-3 周 (硬件) |
| **#5 Authorization 0 校验** | R13 §3.1 | `canonical_router()` (L168-174) **无 auth middleware** | 每个 fetch 加 `Bearer` 头但**后端 0 校验 = 完全无 auth** | 3-5 天 (安全阻塞) |

**R9 spec 错账** (R13 review §1.1-1.3 待主代理亲做修正): §0 TL;DR §25 + §5.1 §330 "4 WIRED + 1 SLOT READY + 6 DEFERRED" → 真账 **6 WIRED + 6 DEFERRED**; §5.2 §342 "OrganOrchestrator 待 R11" → 真账 **R11 spec done + R12 working tree 已起**; §10 §483 "5 actionable" → **9 actionable** (5 done + 4 新加 #6/#7/#8/#9).

---

## 4. 真实施 起点 (按 ROI 排序)

| 改动 | 估时 | 依赖 |
|---|---|---|
| A. gateway SSE 路径 (canonical_entry.rs 加 `Accept` 检测 + mpsc) | 1-2 周 | 0 |
| B. runtime.ts `baseUrl` 切 `:8080` + 流式验证 | 2-3 天 | A |
| C. gateway Authorization middleware + keyring | 3-5 天 | 0 |
| D. R12 OrganOrchestrator 真实施 (已派续跑) | 1-3 周 | 9 organ done ✅ |
| E. runtime.ts 9 organ stream frame 解析 (R9 §4.3 schema) | 3-5 天 | A + D |
| F. runtime.ts 403/409 区分 + 主人审批 modal | 1 周 | C + G |
| G. gateway `POST /v1/apeireth/grant` | 3 天 | 0 |
| H. gateway panel read-only 6 端点 | 1 周 | 0 |
| I. gateway `/v1/models` + `/v1/apeireth/capabilities` | 3 天 | 0 |
| J. Tauri 2 keyring 集成 (plugin-store / stronghold) | 1 周 | C |
| K. gateway 9 organ SSE frame emit | 1 周 | A + D |
| L. Whisper 真接 (R14 spec done, 并行) | 2-3 周 | 硬件 |
| M. E2E + 5 重守门 | 1-2 周 | A-L 全 |

**并行序**:
- Week 1-2: A + C + D 并行
- Week 2-3: B + H + I 并行
- Week 3-4: F + G + K 并行
- Week 4-5: E + J + L 并行
- Week 5-6: M 阻塞所有

**9 organ 暴露范围**: 默认**不向用户暴露** (per O-5 + Q1), 仅 dry_run 模式触发; E7 emergence 8 重门控严守 (R7 独立判断).

---

## 5. 0 触碰 LOCKED 验证

| LOCKED 项 | 前端对接触碰? | 验证 |
|---|---|---|
| 5 项 LOCKED (`10-locked.md` + `philosophy.md`) | ❌ | frontend 独立 workspace, 0 改根 cargo |
| 9 哲学锚本体 (`eight_anchors.rs:58-79`) | ❌ | runtime.ts O-5 = 调失败如实报错, 不假装 OK |
| 13 键 (`philosophy.rs:142 RUNTIME_ENFORCED = false`) | ❌ | 13 键降级哲学标准, frontend 不引入 |
| workspace.version = "1.2.0" (`Cargo.toml:43`) | ❌ | frontend 在 `frontend/companion-desktop/`, 0 改根 `Cargo.toml` |
| R11 baseline (12 slot + Cargo.lock) | ❌ | R12 working tree 不在 frontend 范围 |
| 3 不可变脊柱 | ❌ | 前端不涉及 |

**结论**: B 块 frontend 真实施**完全 0 触碰 LOCKED 6 项**. 验证 = companion-desktop-ci.yml 独立 gate + 根 `cargo test --locked` 0 破.

---

## 6. 估时真账

| 块 | spec 估时 | 真账 | 阻塞依赖 |
|---|---|---|---|
| A. gateway SSE | (R9 §4.3) | 1-2 周 | 0 |
| B. runtime.ts :8080 | (R9 quickstart) | 2-3 天 | A |
| C. gateway auth | (R13 §3.1) | 3-5 天 | 0 |
| D. R12 OrganOrchestrator | (R11 §8.4) | 1-3 周 (已起) | 9 organ done |
| E. 9 organ stream 解析 | (R9 §4.3) | 3-5 天 | A + D |
| F. 主人审批 modal | (R9 §7.1) | 1 周 | C + G |
| G. grant 端点 | (R9 §7.1) | 3 天 | 0 |
| H. panel read-only 6 | — | 1 周 | 0 |
| I. /v1/models + capabilities | (R9 §2.4.3) | 3 天 | 0 |
| J. Tauri keyring | (R9 quickstart) | 1 周 | C |
| K. 9 organ SSE emit | (R9 §4.3) | 1 周 | A + D |
| L. Whisper | (R14) | 2-3 周 (并行) | 硬件 |
| M. E2E + 5 重守门 | (R9 §9.2) | 1-2 周 | A-L |

**真实 critical path**:
- **最短** (假设并行 + UI 一次到位): **4-5 周**
- **现实** (sub-agent 串行 + 集成): **6-8 周** ← 推荐估时
- **悲观** (R12 跑飞 + UI 反复): **8-10 周**

**对比 R13 §3.5 估 5-6 周**: R13 把并行 + 决策一次到位作 baseline, 估时合理. **真账 6-8 周** (加 buffer).

**M (E2E + 5 重守门) 真账 1-2 周**:
- `cargo clippy` / `cargo test --locked`: 已知 0 warning / 1713 passed (R13 §4.2), **0 周**
- `pnpm check` (svelte-check): **未跑过** (README 已知), **估 1-3 天修**
- integration test (mock + 真 LLM, README L73-75): mock 路径已写, **估 1 周跑全 E2E**
- legacy compat / 13 键 / 9 哲学锚 / workspace.version / R11 baseline: 0 触碰, **0 周**

---

## 7. 风险 + 阻塞

- **R1 [严重/技术]**: 9 organ SSE frame schema (R9 §4.3) 是 proposal 未真生产跑过, 1 周内可能发现 schema 缺字段 / cognitive module 串联时序冲突.
- **R2 [严重/技术]**: runtime.ts 1411 行 + 20+ 端点 + CoT 分流 (W6 L401-478) 高复杂度 legacy, 重写**易破** Pattern 移植能力. 建议**只改 baseUrl + SSE 路径**, 不全重写.
- **R3 [中/技术]**: Tauri 2 keyring (plugin-store / stronghold) 跟 Tauri shell 0 apeireth-* dep 边界冲突. **决策点**.
- **R4 [严重/估时]**: R12 OrganOrchestrator working tree 已起未 commit, R11 自报 1-3 周, 真账 3-5 周. **最长串行依赖**.
- **R5 [中/估时]**: spec 4-6 周假设并行, 悲观 8-10 周.
- **R6 [严重/UI 主代理亲做]**: 9 organ 8 种 UI 怎么展示 (E4 浅尝轮盘 / F1 PAD / F4 hypothesis 卡片 / F6 value 徽章 / W1/W2 反事实分支 / W3 edges 列表 / E7 emergence 主动开口 / Memory merge 摘要 + 1 总览 layout). 当前 UI 0 装.
- **R7 [中/UI 主代理亲做]**: 主人审批 modal 触发时机, R9 §7.1 无 spec.
- **R8 [低/frontend 框架]**: Svelte 5 + Tauri 2 已选, 0 风险.
- **R9 [严重/9 organ 暴露 主代理亲做]**: 9 organ 默认**不向用户暴露** (O-5 + Q1), 真生产应暴露多少? spec 没写.
- **R10 [中/9 organ 暴露]**: E7 emergence 8 重门控严守 (R7), frontend 不能为 UI 跳过门控.

---

## 8. 派 sub-agent 还是主代理亲自做?

### 8.1 推荐: 3 派 + 1 亲做 + 续 R12

| 块 | 推荐 | 理由 | 估时 |
|---|---|---|---|
| gateway 真接层 (A + C + G + H + I + K) | **派 sub-agent A** | 纯 Rust, 路径清晰, 不需 UI 决策 | 3-4 周 |
| frontend runtime.ts (B + E + F) | **派 sub-agent B** | 纯前端, 但**等 A + UI 决策冻结** | 2-3 周 |
| Tauri shell 集成 (J) | **派 sub-agent C** | 独立 workspace, 不污染根 | 1 周 |
| 9 organ UI 决策 + 主人审批 modal UI | **主代理亲做** | UI 主观性强, sub-agent 不应决策 | 1-2 周 (并行) |
| R12 OrganOrchestrator 真实施 | **续派 R12** (已派) | working tree 已起, 续跑 | 1-3 周 |

### 8.2 估时比较

- 主代理全亲自做: 估 8-10 周 (串行 + UI 反复)
- 全派 sub-agent: 估 10-12 周 (UI 返工 + 接口误解)
- **3 派 + 1 亲做 + 续 R12 (推荐): 估 6-8 周** (并行最优)

### 8.3 真实施启动前置 (主代理亲做核验 6 项)

1. R9 spec 4 处错账修正 (R13 §8.1)
2. R10 spec 12 slot 数字错账修正 (R13 §9.1)
3. R12 working tree 能否跑通 (决定串行依赖)
4. 9 organ UI 暴露范围决策 (§7.5 R9)
5. 主人审批 modal 行为决策 (§7.3 R7)
6. Tauri keyring 决策 (§7.1 R3)

**6 项核验全过 → 派 A + B + C 三块并行 + 续 R12, 估 6-8 周 2027-Q1 启动, 2027-Q2 完**.

---

## 9. 1 段交付 (给主代理 Mavis)

**B 块 frontend 真实施 readiness = 🟡 部分可启动**. v2 gateway 3 路由 + frontend runtime.ts 1411 行 20+ 端点 + R9/R13 接力审 + 13 改动 ROI 排序全摸清, critical path 6-8 周真账.

**阻塞**: R12 working tree 续跑 + 9 organ UI 决策 + 主人审批 modal 决策 (主代理亲做). **brief 偏差 3 处** (路径 2 + 概念 1), 不影响结论但**不能按 brief 字面派单**.

**派单**: 主代理亲做 6 项核验 (R9 4 处错账 + R10 数字 + R12 跑通 + UI 2 决策 + keyring) → 派 sub-agent A (gateway) + B (frontend) + C (Tauri) 并行 + 续 R12, **估 6-8 周 2027-Q1 启动, 2027-Q2 完**.
