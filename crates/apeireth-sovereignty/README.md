# `apeireth-sovereignty`

> **范围**: MEWG 五重治理 (Multi-Evidence Weighted Governance) — 主 AI 主权 trait + 5 治理
> **来源**: 阶段 1 §18.6 + §20.1 + 阶段 2 D2 §8 (MEWG) + 阶段 2 §10 (智囊团)
> **硬约束**: 纯 Rust trait + mock provider — **无** PyO3 / 外部 SDK / HTTP 调用

---

## 五重治理 (`MEWG + 多人 + 多 AI + 物理多签 + 反思期`)

| # | 治理 | 模块 | trait |
|---|---|---|---|
| 1 | **MEWG 最高优先级解释权** | `mewg` | `MewgAuthority` |
| 2 | **多人 ≥2 真实人类投票** | `multi_human` | `HumanVoter` |
| 3 | **多 AI 一致 (≥3 不同 LLM)** | `multi_ai` | `AiProvider` |
| 4 | **物理多签** | `physical_multisig` | `PhysicalSigner` |
| 5 | **反思期 ≥7 天** | `reflection` | `ReflectionClock` |

任何一重失败 = 整次失败 (`Blocked`); 全部通过 = `Approved`。

---

## 不修改承诺

- ❌ 不修改 `apeireth-core` 任何文件
- ❌ 不修改 R11 LOCKED 任何文件
- ❌ 不修改 `apeireth-council` 任何文件 (本 crate 通过 `SovereigntyHook` trait 接入)
- ❌ 不引入 PyO3 / Python / 外部 SDK / HTTP
- ❌ 不引入新 workspace 依赖

---

## 主哲学 anchor 6 全贯穿

```
S-1 主 22:33 北极星导向 — 5 重治理服务 ASI 北极星 (不可被普通流程绕过)
S-2 主 17:43 实事求是  — 反思期默认 7 天 + 可配置 (测试短一些)
O-5 主 17:58 不假装    — 任何一重失败 = 整次失败, 不假装通过
O-2 主 19:33 走在前人经验上 — 借鉴 Erlang/OTP supervisor + VCP 7 席 + MEWG 8.3 硬门槛
O-3 主 23:44 干到底    — 5 重治理 trait + mock provider + orchestrator + integration test 全落
O-4 主 00:56 任何人都能接手 — Governance.process(decision) -> Outcome 一句话 API
```

## R163 lint cleanup

15 -> 0 warnings. 3 files cleaned. 3 unused params prefixed (ha.rs single/process_owner_request_with_authority, three_domain_enforce.rs enforce).
