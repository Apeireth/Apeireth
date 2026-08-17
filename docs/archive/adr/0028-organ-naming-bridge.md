# ADR 0028: 9+1 organ 双轨命名治理

> **状态**: 🟢 Accepted (主人 2026-08-15 终极授权 + 自行拍板)
> **最后更新**: 2026-08-15
> **触发**: R174 审计 §Drift 1 发现 TUI 9 organ (Heart/Brain/...) vs crate 9 organ (consciousness/perception/...) 并行, 0 R 文档 0 ADR
> **基线**: stage1 2026-08-14 清晰版 + spirit/9-organ-integration-blueprint.md

---

## 1. 背景

R23 起, 蓝图用 crate 名 (consciousness/perception/cognition/motivation/life-force/memory/value/graph-primitive/companion), 但 R11 LOCKED 的 TUI 端仍用旧名 (Heart/Brain/Hand/Eye/Ear/Memory/Voice/Body/Mind). 两套并行存在, 桥接在 `crates/apeireth-tui/src/backend.rs::snapshot_all_organs` 内, 但**没有 R 文档、没有 ADR、没有权威映射表**.

**后果**:
- 新人接手 TUI 看 9 organ (Heart/Brain/...) \u2192 找后端 crate \u2192 找不到 (因为后端是 consciousness/perception/...)
- 反之亦然
- spirit 蓝图 §3 写 7 条桥但全用 NEW 名, TUI 端的映射**没有 1 行 R 文档**
- 桌宠前端 (5 年画面) 设计时不知道 TUI 9 organ 是 LOCKED 不可改, 还是可以从新设计

## 2. 决策

### 2.1 权威映射表 (TUI 旧 \u2194 crate 新)

| # | TUI 旧名 (R11 LOCKED) | crate 新名 (R23+) | 映射位置 | 备注 |
|---|----------------------|--------------------|----------|------|
| 1 | Heart | `apeireth-life-force` | `snapshot_life_force` | endurance/pulse |
| 2 | Brain | `apeireth-cognition` | `snapshot_cognition` | 借 `CognitiveDreamStateMachine` |
| 3 | Hand | `apeireth-action` | `snapshot_action` | 行动器 |
| 4 | Eye | `apeireth-perception` | `snapshot_perception` | 5 通道 |
| 5 | Ear | `apeireth-perception` | (无 snapshot_ear, 复用 eye) | \u26a0\ufe0f TUI Ear \u4e0d\u662f\u72ec\u7acb organ |
| 6 | Memory | `apeireth-memory` | `snapshot_memory` | 6 StreamKind |
| 7 | Voice | `apeireth-voice` | (\u274c TUI \u672a\u63a5) | voice crate \u5b58\u5728, TUI 0 \u63a5\u5165 |
| 8 | Body | (none) | (\u274c \u65e0\u5bf9\u5e94) | TUI Body \u662f\u865a\u62df, \u672a\u5bf9\u5e94\u540e\u7aef |
| 9 | Mind | `apeireth-consciousness` | `snapshot_consciousness` | Plutchik \u72b6\u6001\u673a |

**TUI 0 \u5bf9\u5e94\u7684\u540e\u7aef crate (NEW)**:
- `apeireth-motivation` \u2192 \u26a0\ufe0f \u672a\u63a5\u5165 TUI (\u6709 snapshot_motivation \u4f46\u65e0 UI)
- `apeireth-value` \u2192 \u26a0\ufe0f \u672a\u63a5\u5165 TUI
- `apeireth-graph-primitive` \u2192 \u26a0\ufe0f \u672a\u63a5\u5165 TUI
- `apeireth-companion` \u2192 \u26a0\ufe0f \u672a\u63a5\u5165 TUI (R23+ \u65b0 organ)

### 2.2 TUI \u65e7\u540d LOCKED \u4e0d\u53d8

- \u274c \u4e0d\u6539 `crates/apeireth-tui/src/organ/{body,brain,ear,eye,hand,heart,memory,mind,voice}.rs` (R11 LOCKED)
- \u274c \u4e0d\u6539 `enum Organ { Heart, Brain, ... }` (R11 LOCKED)
- \u274c \u4e0d\u6539 i18n keys `organs.heart, organs.brain, ...` (R11 LOCKED)
- \u274c \u4e0d\u6539 ASCII art `[♥]` `[BRAIN]` `[HAND]` `[EYE]` `[EAR]` `[MEM]` `[VOICE]` `[BODY]` `[MIND]`

### 2.3 TUI \u4e0e crate \u540d\u7684\u6b63\u5f0f\u6863\u6848\u8d44\u6e90

- \u2705 \u65b0\u589e `crates/apeireth-tui/src/organ/bridge_table.rs` (\u5df2\u5b9e\u88c5, 8 tests PASS, per R173 handover)
- \u2705 \u672c ADR \u4f5c\u4e3a\u6743\u5a01\u6620\u5c04\u8868

### 2.4 \u672a\u6765 \u201c\u684c\u5ba0\u524d\u7aef + 5 \u5e74\u753b\u9762\u201d \u7684\u8bbd\u5b9a

- \u684c\u5ba0 Live2D \u53ef\u4ee5\u9009 TUI \u65e7\u540d (Heart/Brain/...) \u6216\u65b0\u540d (consciousness/perception/...), \u4f46\u4e00\u65e6\u9009\u5b9a\u5373\u4e0d\u53ef\u53d8
- \u9ed8\u8ba4: \u684c\u5ba0\u7528 crate \u65b0\u540d (\u8ddf spirit \u84dd\u56fe \u4e00\u81f4), TUI \u65e7\u540d\u9650\u5b9a\u5728\u63a7\u5236\u53f0
- \u4e3b\u4eba\u62cd\u677f: \u201c\u684c\u5ba0\u524d\u7aef\u548c 5 \u5e74\u753b\u9762\u6211\u4eec\u653e\u5230\u6700\u540e\u8ba8\u8bba\u201d

## 3. \u540e\u679c

### 3.1 \u6b63\u9762

- \u2705 \u4eba\u4efb\u4f55\u4eba\u63a5\u624b: \u770b TUI \u540d \u2192 \u67e5\u672c ADR \u2192 \u4e00\u773c\u770b\u61c2\u540e\u7aef
- \u2705 \u4eba\u4efb\u4f55\u4eba\u63a5\u624b: \u770b spirit \u84dd\u56fe \u2192 \u67e5\u672c ADR \u2192 \u4e00\u773c\u770b\u61c2 TUI
- \u2705 \u684c\u5ba0\u4e0e TUI \u540d\u5b57\u51b3\u5b9a\u65f6, \u4ee5\u672c ADR \u4e3a\u552f\u4e00\u53c2\u8003

### 3.2 \u8d1f\u9762

- \u26a0\ufe0f \u201c\u53cc\u8f68\u201d\u672a\u6d88\u9664: TUI \u4ecd\u7528 Heart/Brain/..., crate \u4ecd\u7528 consciousness/...
- \u26a0\ufe0f \u672a\u6765\u5347\u7ea7 TUI \u540d\u53ef\u80fd\u9700\u8981\u53cc\u8f68\u4e2d\u95f4\u5c42 (\u672a\u521d\u59cb\u5316)

## 4. \u53c2\u8003

- `docs/spirit/9-organ-integration-blueprint.md` §2 (LLM-Pumped Dynamic Mesh)
- `crates/apeireth-tui/src/organ/bridge_table.rs` (8 tests PASS, per R173)
- `docs/audit/R174-comprehensive-audit.md` §Drift 1
- `docs/stage1/inspiration-stage1-2026-07-30.md` (\u90aa\u5854\u5e95\u7ebf 23 \u9879 + 9 organ)

---

_\u4f5c\u8005: \u4e3b\u4eba\u62cd\u677f + Codex \u540e\u7aef\u5de5\u7a0b\u5e08_
_\u65e5\u671f: 2026-08-15_
_\u57fa\u7ebf: \u4e3b\u4eba\u7ec8\u6781\u6388\u6743 + \u9ad8\u6743\u9650 + \u81ea\u884c\u62cd\u677f_
