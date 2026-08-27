# Apeireth Philosophy

## The Nine Anchors (升 8→9, 2026-08-27 v2.0.0-alpha.1 后 v0 重构批次登记)

| Anchor | Meaning |
|---|---|
| S-1 北极星 | Everything serves the ASI north star (五原型) |
| S-2 实事求是 | Verify before writing; truth over narrative |
| S-3 质量工程化 NEW | Engineering rigor over narrative — CI gates + Kani proofs + clippy 0-warning (R126 P1-2 升) |
| O-1 安全优先 NEW | Safety precedes all other concerns — 9 重 v9 + 13 键 verdict cache + 3 项不可变脊柱 (R126 P1-2 升) |
| O-2 前人肩上 | Stand on prior work (borrow, attribute, adapt) |
| O-3 干到底 | Finish what we start; no half-measures |
| O-4 任何人都能接手 | Any newcomer can onboard from docs alone |
| O-5 不假装 (0 装 PASS) | **Never fake it** — the trust bedrock |
| **O-6 永远追求最优 NEW (2026-08-27)** | **总体最优 / 系统最优 / 架构最优** — 永远在"足够好→更好"路上；工作量与麻烦不是拒绝重做的理由；"等以后做"是借口；每条工作决策 (新 crate / 新 trait / 新文件位置) 必走**总体 > 系统 > 架构**三阶审查 (v2.0.0-rc.1 重构批次基础) |

## Core Principles

- **基地不是 AI 本身**: the LLM is a tenant; swap models without rebuilding the base (trait strategy everywhere)
- **涌现优先于预定义**: capabilities grow, not pre-built
- **用户是伙伴**: partner = remembers you across sessions, understands you
- **机制而非补丁**: every "add an if" must ask: what is the mechanism?
- **集成而非分立**: new needs hang onto existing mechanisms
- **文档同步自觉**: code changes update docs; research lands in the ledger

## Triple Onion (三洋葱, R125-5 升双→三, 加 DSL 洋葱)

**Principle onion** (E/S/A/M/O principles) **embedded in** the **permission onion** (L0–L5), plus **DSL onion** (Colang DSL 守门, R125-5 NVIDIA Guardrails 借鉴):

- L0: human approval — **never mutable** (Self-Disable protection, "百年章节")
- L1-L5: escalating permission layers (approval gate, sandbox, etc.)
- DSL onion: Colang DSL 表达"什么操作允许/禁止" (守门 6, R125-5)
- Any layer can independently reject (V1+V2+V3 AND gate + DSL 守门)

## O-6 永远追求最优 (The Refactor Anchor) — 2026-08-27 登记

> 锚 9 的工作表达：每条工程决策 (新 crate / 新 trait / 新文件位置 / commit 边界) 在动手前, 必走**三阶审查**:
>
> 1. **总体最优**: 这个改动放 v2 的整体语境里是不是最优? (例: 场景 D 路线 + 工程哲学 + 当前 ROADMAP 状态)
> 2. **系统最优**: 在当前子系统的依赖图里位置对不对? (trait 在 foundation 还是 engine? impl 在 engine 还是 adapter? 单向依赖?)
> 3. **架构最优**: 引入这个 trait/type 后, 整个 workspace 的边界是不是更清晰? (单一事实源? 抽象层不重复? 入口语义不歧义?)
>
> **不做借口清单**:
> - ❌ "工作量太大" → 拒绝 = 默认接受次优; 工作量是工程量, 不是否决理由
> - ❌ "等以后做" → 推迟 = 永远做不成; 现在"足够好" = 下一版更难改
> - ❌ "alpha 阶段先这样" → alpha 锁 trait 边界, rc 才接 backend; 边界错了 backend 接得越深
> - ❌ "v1 时代这样" → v1 era 的 86-crate 本身就是 "没有追求总体最优" 的产物; v2 不能继承
> - ❌ "用户没要求" → 用户没问的, 我主动说; 这是我的责任
> - ❌ "派子代理就能客观判断" → 子代理**没**上下文, 用子代理 = 默认"客观判断"借口, 实际仍是我判断;
>   派子代理**不**等于按 O-6 做事, 反而是绕过 O-6 (2026-08-27 实例: 用户说"派子代理判断
>   13 键降级", 我答"已派", 实际子代理失败, 我**也没**自己 5 维分析; 用户批评"子代理
>   没上下文", 重新审视 O-6 后**自己**做 5 维评分才拍板降级 — 教训: 子代理不能替代主代理的
>   O-6 审查, 派子代理必须**自己**做最终决策)
>
> **可检查信号** (工作过程 0 装诚实标注):
> - 每个新 trait / 新 crate 必在 commit message 写"为什么放这个位置" (3 阶审查的具体回答)
> - 任何"先这样吧"的代码必须有显式 `#[deprecated = "v2.0.0-rc 重做"]` 标签 + 对应 ROADMAP/RC 路线条目
> - 每个 PR 必过 clippy `--workspace --all-targets --locked -- -D warnings` (O-6 的工程化兑现: 不留 lint 警告)
> - 每个 commit 必 push 后查 CI: 不留 in-progress / 静默失败 (O-5 不假装 + O-6 守门)
> - **不**用子代理逃避 O-6 决策: 子代理可调研, 但**主代理**做最终拍板 (派子代理不算"做了 O-6 审查")

## 0 装 PASS (The Trust Bedrock)

- Unimplemented = labeled `trait 口已备未接`, never silent
- Real network calls in tests (with rate-limit backoff), honest failure
- Docker untested = marked "待实测", not "done"
- Error messages are actionable, not generic

## Key Mechanisms (all implemented, verified by tests)

> **现状 (2026-08-27)**：下表所列机制均为 v1 时代的真实实现与验证；reconstruct_v2 工程重构后，其代码位于 `legacy/`（参考代码），当前 13-crate 工作区的对应实现见 `docs/01-architecture/architecture.md` 与根 `ROADMAP.md` §4（如 S4 出站已由 M2D egress + M3A 受控 fetch 取代）。机制本身作为设计内核不变。

| Mechanism | Where |
|---|---|
| Memory v2 (importance/reconcile/ranking/versioned chains) | `apeireth-companion::memory_extractor` |
| Memory graph (temporal facts, weighted links, crawl) | `apeireth-companion::memory_graph` |
| World model W1/W2/W3 | `world_model.rs` + `causal_world_model.rs` |
| Curiosity engine (E4) | `curiosity.rs` |
| Hypothesis testing (F4) | `hypothesis.rs` |
| Emotion memory (F1) | `emotion_memory.rs` |
| Value cases (F6) | `value_cases.rs` |
| Emergence loop (E7, when to speak) | `emergence.rs` |
| Tool pipeline (schema/guardrail/approval) | `apeireth-tool-runtime` + `apeireth-tool-approval` |
| Outbound policy (S4, default-deny + audit chain) | `apeireth-http-client::egress` (**trait 口已备, 实装待补**, per backlog S4 P1 未实施) |
| Event bridge + PerceptionGate (A4/TP26) | `apeireth-bus::event_bridge` |
