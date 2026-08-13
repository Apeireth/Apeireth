# R165 架构体检 + 死码归档

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R165 (架构净化 + 一体化优雅)
> **日期**: 2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 总览

| 子项 | 目标 | 状态 |
|---|---|---|
| 全仓架构体检 | 0-调用者死码 / 重复造轮子 / 同质冗余识别 | ✅ |
| 归档 `apeireth-protocol-bridge` | 0 业务调用者, 移到 `_archived/` | ✅ |
| 归档 `apeireth-formal` | 0 外部依赖, 移到 `_archived/` (governance::formal_proof 承担生产路径) | ✅ |
| workspace members 79 → 77 | 2 死码出 workspace | ✅ |

**结果**: `cargo check --workspace` 0 errors / 0 actionable warnings. 2 个真死码归档.

---

## 1. 架构体检报告 (R165 R 周期审计)

### 1.1 体检方法

全仓 78 active crates 跨 crate 引用扫描:
- 0 外部调用者 (≤ 3 callers): 候选死码/低使用
- 1-5 外部调用者: 候选低使用
- ≥ 6 外部调用者: 健康

### 1.2 0 外部调用者 (2 真死码)

| crate | 行数 | 状态 |
|---|---:|---|
| `apeireth-protocol-bridge` | 9 src files | 0 callers — 死码 |
| `apeireth-formal` | 19 src files + Kani proofs | 0 callers (governance::formal_proof 是实际生产路径) |

`apeireth-formal` 详查: 原设计是 Kani 形式化验证引擎 (213 passing tests in lib). 但所有形式化集成通过 `apeireth-library-governance::formal_proof` 进行, governance 有独立的 formal_proof::ProofResult / defensive_proof / proof_harnesses. 两个 formal infra 并行存在 — **治理层的 formal_proof 已被选为生产路径, apeireth-formal 沦为研究骨架**.

### 1.3 1-5 外部调用者 (潜在低使用)

| crate | refs | 评估 |
|---|---:|---|
| `apeireth-action` | 1 | leave as leaf, 单一归属 |
| `apeireth-perception` | 1 | leave as leaf |
| `apeireth-sdk` | 1 | leave as leaf |
| `apeireth-workflow` | 1 | leave as leaf (R165 R2 候选升级: Temporal-style Activity) |
| `apeireth-lark` | 1 | leave as leaf (飞书 SDK) |
| `apeireth-voice` | 1 | leave as leaf |
| `apeireth-tool-search` | 2 | leave as leaf (inverted index + VSearch + scoring) |
| `apeireth-central` | 3 | leave as leaf |
| `apeireth-motivation` | 3 | leave as leaf |
| `apeireth-arbitration` | 3 | leave as leaf |
| `apeireth-eval` | 4 | leave as leaf |
| `apeireth-state` | 4 | leave as leaf (R150 statechart 已加) |
| `apeireth-life-force` | 5 | leave as leaf |
| `apeireth-team-lead` | 5 | leave as leaf |

**注**: 这些都是叶子 crate, 1-5 refs 在 78-crate workspace 里属于健康基线. 0 ref 才是真死码.

### 1.4 重复造轮子识别

| 模块 | 重复位置 | 评估 |
|---|---|---|
| `token_budget.rs` | `apeireth-pipeline/src/` + `apeireth-tool-registry/src/` | 函数签名不同 (parameterized vs fixed), 域名分得清, 保留 |
| `formal_proof` | `apeireth-library-governance/src/formal_proof.rs` (生产路径) + `_archived/apeireth-formal/` (R165 归档) | 归档后单一来源 |
| `protocol` | `apeireth-protocol/` (4 LLM 协议归一化, 11 callers) + `_archived/apeireth-protocol-bridge/` (R165 归档) | 归档后单一来源 |

**结论**: 归一化前有 3 个 "重复" 模块, 归一化后剩 1 个 (token_budget 真业务双轨合法).

---

## 2. 归档实施

### 2.1 `apeireth-protocol-bridge` → `_archived/apeireth-protocol-bridge`

**历史**: R141 创建, 旨在合并 VCP 5 协议为 1 bridge (per v2 plan §9.5). 因 9 源文件依赖 8 个 crate (apeireth-tools/tool-registry/tool-runtime/tool-approval/core/sovereignty/protocol/api), 范畴重叠于 `apeireth-protocol` (4 LLM 协议). 后者已主导生产, bridge 沦为未引用的扩展.

**Move 操作**: `mv crates/apeireth-protocol-bridge crates/_archived/apeireth-protocol-bridge`
**Cargo.toml 改动**: 注释 workspace members 行, 加 R165 archive note

### 2.2 `apeireth-formal` → `_archived/apeireth-formal`

**历史**: R122-9 (V2.1 P2-11) 创建, Kani 形式化验证骨架. 设计作为底层 formal infra 给其他 crate 用. 但 `apeireth-library-governance` (R127 P5-2) 自带 `formal_proof` 模块, 成为实际生产路径. governance::formal_proof 与 formal crate 没有任何依赖关系 — 二者独立实现.

**架构选择**: 0 引外部 dep 原则 + 一体化优美 — 选 `governance::formal_proof` 为生产 canonical, `apeireth-formal` 归档保留作研究骨架.

**Move 操作**: `mv crates/apeireth-formal crates/_archived/apeireth-formal`
**Cargo.toml 改动**: 注释 workspace members 行, 加 R165 archive note
**cfg(kani) workspace.lints**: 保留 — `apeireth-library-governance/src/verification.rs:77` 仍用 `#[cfg(kani)]`, 仍需 workspace 级 cfg 白名单

### 2.3 验证

```
cargo check --workspace: 0 errors, 0 actionable warnings (was: 1 R163 + 0 L165)
  - 1 remaining: nom v1.2.4 future-incompat (third-party, 不可修)
```

---

## 3. 0 触碰清单

| 项 | 状态 |
|---|---|
| workspace.version 1.2.0 | ✅ 0 改 |
| Self-Disable 判定逻辑 | ✅ 0 改 |
| L0 HA 物理隔离定义 | ✅ 0 改 |
| 13-key verdict cache 语义含义 | ✅ 0 改 |
| 24 LOCKED 撤销状态 (R148) | ✅ 0 改 |
| V0.5 30 维 / V1136 / R11 baseline 3 值 | ✅ 0 改 |
| 9-key 原始 baseline | ✅ 0 改 |
| docs/v4 / v4.1 / v2 / V0.5 / V1136 / 9键原始 | ✅ 0 改 |
| `apeireth-library-governance::formal_proof` production path | ✅ 0 改 (强化: 归一化后唯一源) |

---

## 4. 借鉴 ID (O-5 不假装)

| ID | 来源 | 用处 |
|---|---|---|
| `R165-ARCH-AUDIT-cross-crate-reference-2026-08` | community convention: rg-based dead code audit (per `cargo-udeps`, `cargo-machete` 思想, 0 引入依赖) | 全仓跨 crate 引用扫描, 识别 2 死码 |
| `R165-FORMAL-CONSOLIDATE-governance-canonical-2026-08` | apeireth 自身双轨治理决策 | apeireth-formal 归档, governance::formal_proof 为生产 canonical |

---

## 5. 文档交叉引用

- `docs/r165/r165-architecture-audit-and-deadcode-archive.md` (本文件)
- `Cargo.toml` (workspace members, 2 行注释为 archive note)
- `_archived/apeireth-protocol-bridge/` (9 源文件 + Cargo.toml 保留)
- `_archived/apeireth-formal/` (19 源文件 + Kani proofs + Cargo.toml 保留)
- `crates/apeireth-library-governance/README.md` (R165 canonical 强化, 待办)

---

## 6. 下一步 (R166+)

**R166 候选** (按 ROI 排):
- R166a: apeireth-workflow Temporal-style Activity (per R150 skipped, R149 P1 #7)
- R166b: apeireth-tool-fetch 已 done, 调研 GitHub 同期项目 (rmcp vs spider-rs vs headless_chrome)
- R166c: sovereignty Hyperlight 调研 + 集成设计 (R149 P2 #13)
- R166d: voice GPT-Realtime-2 接入 (per master 提供的 minimax apikey in `.openclaw`)

**R166+ 终极路径**:
- 终极目标 = 全做全补弱 + 一体化优美
- 干到底, 不停
- 时间充裕, token 充裕