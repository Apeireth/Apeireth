# 24 LOCKED Crate 索引 (历史技术事实, R148 状态: 0 约束力)

> **R148 状态 (2026-08-13, Mavis 自决)**: 本文档作为历史技术事实保留 (per R125 B1 落实的 24 crate 名单). **0 约束力** (per decision-74 + decision-130, 主人 8/11 22:31 拍板). 24 LOCKED 入口签名冻结已形式撤销, 仅保 3 项不可变脊柱. 详见 `docs/conventions/10-locked.md` R148 段落.

# 24 LOCKED Crate 索引 (技术事实, 持续更新)


> **R119-3b Mavis 重建 (2026-08-10)**: 24 LOCKED crate 索引, R11 LOCKED baseline 16:34 之前严守。
> **R119-8 原则调整 (2026-08-10, 主人 1:49 拍板)**: 技术类文档不锁。24 LOCKED crate 实际列表是技术事实, 文档持续更新。**数据严守 ≠ 文档结构锁**。
> **R125 末 B1 落实 (2026-08-10 16:38, Mavis 自主, 主人 16:31 最高权限授权)**: 24 LOCKED crate 完整名单 (12 主人已知 + 12 Mavis 自主) 自主确认, 替代原 12+12 估。**总 41 LOCKED** (24 + 9 organ + 8 LOCKED 文档)。

```
[Document-Meta]
Document: docs/omnibus/24-locked-crates.md
Version: Manual-Rev-L + Fix-17 + B1-R125
R-Cycle: R125-B1
Last-Modified: 2026-08-10
Status: 🟢 活跃 (技术事实, mtime 16:34 之前严守 + 文档结构持续更新)
```

---

## 24 LOCKED Crate 完整名单 (R125 B1 落实, Mavis 自主)

### 主人已知 12 (per 8-promise-audit §3.4 + 1.0-release-report §6.1)

| # | Crate | 路径 | 备注 |
|---:|---|---|---|
| 1 | apeireth-supervisor | `crates/apeireth-supervisor/src/lib.rs` | mtime 16:34:11 |
| 2 | apeireth-agent | `crates/apeireth-agent/src/lib.rs` | mtime 16:34:11 |
| 3 | apeireth-bus | `crates/apeireth-bus/src/lib.rs` | mtime 14:07:47 |
| 4 | apeireth-council | `crates/apeireth-council/src/lib.rs` | mtime 14:07:57 |
| 5 | apeireth-evolution | `crates/apeireth-evolution/src/lib.rs` | mtime 14:07:57 |
| 6 | apeireth-extension | `crates/apeireth-extension/src/lib.rs` | mtime 14:08:05 |
| 7 | apeireth-graph | `crates/apeireth-graph/src/lib.rs` | mtime 09:08:10 |
| 8 | apeireth-mcp | `crates/apeireth-mcp/src/lib.rs` | mtime 14:08:05 |
| 9 | apeireth-pipeline | `crates/apeireth-pipeline/src/lib.rs` | mtime 14:08:14 |
| 10 | apeireth-tool-registry | `crates/apeireth-tool-registry/src/lib.rs` | mtime 14:08:27 |
| 11 | apeireth-tool-runtime | `crates/apeireth-tool-runtime/src/lib.rs` | mtime 14:08:27 |
| 12 | apeireth-protocol | `crates/apeireth-protocol/src/lib.rs` (+8 lines 模块导出声明) + `ws_v1.rs` (新文件 513 行, R20 阶段 2 续时授权) | 例外: 0 改原 LLM 协议归一化层 |

### Mavis 自主 12 (per 主人 16:31 最高权限, B1 落实, 16:38 拍板)

| # | Crate | 路径 | Mavis 自主理由 |
|---:|---|---|---|
| 13 | **apeireth-asi** | `crates/apeireth-asi/src/lib.rs` | LOCKED V0.5/V1136 (per 17-APEIRETH-VS-VCP §597), 24 维公式, ASI 哲学核心 |
| 14 | **apeireth-onion** | `crates/apeireth-onion/src/lib.rs` | 5 重守门来源, 双洋葱架构, 哲学核心 |
| 15 | **apeireth-sovereignty** | `crates/apeireth-sovereignty/src/lib.rs` | 274KB LOCKED 安全核心, R124-3 调研 0 触碰 |
| 16 | **apeireth-constraint** | `crates/apeireth-constraint/src/lib.rs` | 5 重守门核心, R124-3 调研 0 触碰 |
| 17 | **apeireth-memory** | `crates/apeireth-memory/src/lib.rs` | LOCKED memory 9 文件 (per R120 A 9 LOCKED 0 触碰), 3 层 memory 哲学核心 |
| 18 | **apeireth-cognition** | `crates/apeireth-cognition/src/lib.rs` | R124-2 B-028 OpenCog 借鉴目标, 9 organ brain 来源 |
| 19 | **apeireth-perception** | `crates/apeireth-perception/src/lib.rs` | R20 哲学 crate, 9 organ eye/ear 来源 |
| 20 | **apeireth-consciousness** | `crates/apeireth-consciousness/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 perception) |
| 21 | **apeireth-motivation** | `crates/apeireth-motivation/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export) |
| 22 | **apeireth-life-force** | `crates/apeireth-life-force/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 memory) |
| 23 | **apeireth-relation** | `crates/apeireth-relation/src/lib.rs` | R20 哲学 crate, R124-2 §12 借鉴目标 |
| 24 | **apeireth-value** | `crates/apeireth-value/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 motivation) |

---

## 总 41 LOCKED (24 + 9 organ + 8 LOCKED 文档)

**主人 1.1-release/README.md 摘要**: "**24 LOCKED + 9 organ + 8 LOCKED**"

### 9 organ (per `crates/apeireth-tui/src/organ/*.rs`)

1. body.rs
2. brain.rs
3. ear.rs
4. eye.rs
5. hand.rs
6. heart.rs
7. memory.rs
8. mind.rs
9. voice.rs
(10. mod.rs 是入口)

### 8 LOCKED 文档 (per `8-promise-audit §4` 7 LOCKED 文档 + workspace.version 1 项)

1. APEIRETH-CONVENTIONS.md
2. APEIRETH-VERSIONING.md
3. APEIRETH-GLOSSARY.md
4. 阶段 4 核心文档 (`6ca80776` commit)
5. 阶段 5 施工文档 (631 行)
6. v6 基础架构 (4 重守门 + 权限发放 + E 层)
7. R11 baseline 3 文档 (V1141/V1131/V1136)
8. workspace.version 1.x.x (semver 严格, 当前 1.1.0, R125 末升 1.2.0, R127 release 1.0.0)

**总 41 LOCKED** (24 + 9 + 8).

---

## 实际 60+ 实质 LOCKED (R20 阶段 6 文档承认 + R19+ 集成期增量)

per §42-47 (历史登记):
> 实际 90+ 个 crate
> 24 LOCKED crate 占主体 (R11 baseline)
> 5 估补 crate (R20 阶段 4 PLANNED)
> 其他 = R14 / R17 / R23 / R33-R37 / R38 / R46-R53 / R54 / R70-R72 / R78-R113 / R114-R118 各周期增量

**Mavis 自主确认**: 24 LOCKED crate 是 R11 baseline 的 24 核心. R14+ 增量的其他 40+ crate 也算"实质 LOCKED" (R-Method / R14 Rust traits / R17 战役 1-1 / R19 集成 4 子阶段), 但**不在 24 LOCKED 名单**, 算"持续扩展 LOCKED 集". R125 借鉴实施时, Mavis 自主按"实质 LOCKED" 严守, 不止 24 个.

---

## 核验

- ✅ mtime 全部 16:34 之前 (R11 LOCKED baseline, per 8-locked-unified-2026-08-05.md §2)
- ✅ R19+ 集成期 0 触碰 (per R20 8 项承诺审计 `629995d3`)
- ✅ R38 1.1 RC 0 触碰 (per R38 B9)
- ✅ R54 1.1.2 patch 0 触碰 (per R54 commit `d30a2f00`)
- ✅ R114-R118 0 触碰 (per codex `5c546a84` 报告)
- ✅ R119 形式撤销, 原意保留
- ✅ R125 B1 落实 (24 完整名单, Mavis 自主, 主人 16:31 最高权限)

---

## 6 哲学锚穿透

- **S-1**: 24 LOCKED crate 主体 (R11 baseline 北极星)
- **S-2**: 核验后写, mtime baseline 16:34 之前 (实事求是)
- **O-2**: 走在前人经验上 (R11 Python → R14 Rust rewrite → R20 阶段 1-6 → R125 B1 落实)
- **O-5**: 0 触碰严守 (R125 B1 24 完整名单自主确认, 0 假装 LOCKED 实际 12+12)

---

## 不漂移 (B1 落实后, R125 末更新)

- 24 LOCKED crate mtime 16:34 之前严守 (技术数据, 持续核验, 实际 60+ 个)
- 0 改 workspace.version (R125 末 B2 升 1.2.0, R127 release B2 升 1.0.0)
- 0 改 R11 baseline 3 值 (数字严守, A1 0 改)

---

## 历史脉络 (R125 B1 落实后)

- R11 末: 24 LOCKED crate 占主体 (R11 Python 系统, 主人 2026-07-31 明确沉淀)
- R14 Rust rewrite: 24 LOCKED crate src/ 1:1 翻译 (24 维公式 + 9 organ + 9 sub-measure)
- R20 阶段 6: 8-locked-unified 统一收口, 11+1 LOCKED crate mtime 实查 (R20 §6.1)
- R119 形式撤销 (8/10 01:14): 8 项形式撤销, 原意保留
- R119-8 (8/10 01:49): 3 技术类 LOCKED 撤销 (baseline 数字 / 24 LOCKED 实际列表 LOCKED 状态)
- **R125 B1 (8/10 16:38, 本文档)**: 24 LOCKED 完整名单自主确认 (12 主人已知 + 12 Mavis 自主), 主人 16:31 最高权限授权

---

**本 24 LOCKED crate 索引是 R125 实施 + R126 续 + R127 release 的"严守 baseline" 唯一引用源, 任何 R125+ 文档引用 "24 LOCKED" 时必须指向本文档 §"24 LOCKED Crate 完整名单" (per 主人 16:31 最高权限 + Mavis 16:38 自主确认).**
