
# ADR 0031: 3 re-export organ 概念统一 (consciousness/life-force/value)

> **状态**: 🟢 Accepted (主人 2026-08-14 终极授权 + 自行拍板)
> **commit 锚**: 本 ADR + `docs/audit/R174-comprehensive-audit.md` §4.2 缺 5
> **最后更新**: 2026-08-14 23:10
> **触发**: 全面审计发现 3 organ crate 是 transparent re-export, 概念混淆

---

## 1. 背景

R37-2 引入 transparent re-export 让 9 organ 命名兼容:
- `apeireth-consciousness` → re-export 到 `apeireth-perception`
- `apeireth-life-force` → re-export 到 `apeireth-memory`
- `apeireth-value` → re-export 到 `apeireth-motivation`

后果:
- 9 organ 命名骗自己 (per O-5 不假装违反)
- 实际只有 6 organ 实装 + 3 organ 入口
- spirit 蓝图 (R23+ 设计) 假设 9 个独立 organ, 实际是 6 个
- 下游调用方 `use apeireth_consciousness::X` 仍能用 (R37-2 后 0 breaking)

## 2. 决策 (Decision)

### 2.1 概念权威

| 入口 crate | 实际实装 crate | 关系 | 文档语义 |
|-----------|---------------|------|----------|
| `apeireth-consciousness` | `apeireth-perception` | re-export | "意识是感知的特殊形式 (CognitiveDreamStateMachine 借 perception 实现)" |
| `apeireth-life-force` | `apeireth-memory` | re-export | "生命力依赖 memory 持久化 (reflection cycle 借 memory 存储)" |
| `apeireth-value` | `apeireth-motivation` | re-export | "价值是动机的目标 (V0.5 §13 motivation_score 含 value 维度)" |

### 2.2 维持 re-export 决策

**选项 A (本 ADR 拍板)**: 维持 R37-2 re-export, 加 ADR 文档化概念统一
- 好处: 0 触碰下游 import, 0 breaking
- 好处: 9 organ 概念跟 spirit 蓝图对齐 (虽然实装是 6)
- 好处: workspace member 保留 = 0 改 Cargo.toml
- 坏处: 概念不诚实 (9 ≠ 6)
- **缓解**: 本 ADR §2.1 文档化 3 个 re-export 关系 + §3.1 标诚实缺

**选项 B (否决)**: 删 3 个 re-export crate, 仅留 6 organ
- 破坏 1.0 release 兼容 (下游 `use apeireth_consciousness::X` 失效)
- 破坏 spirit blueprint §2 设计 (9 organ 不是 6)
- 工作量: 大 (workspace member 删 + 全 workspace 替换 import)

**选项 C (否决)**: 3 re-export crate 各自实装独立 organ
- 工作量: 巨大 (3 个独立 organ 设计与实现)
- 时间: 估 2-3 个月
- 当前 spirit 蓝图已经定义 re-export, 实装独立 = 蓝图变更

### 2.3 文档化规则

每条 re-export 在 lib.rs 顶部 doc 必须显式写:
```rust
//! apeireth-consciousness: 意识子系统
//!
//! **关系**: R37-2 transparent re-export 到 `apeireth-perception`
//! **实装**: CognitiveDreamStateMachine 在 `apeireth-perception` 实现
//! **不漂移**: 0 改 perception 任何 1 行
```

(当前 consciousness/lib.rs 已部分写明, life-force 和 value lib.rs 需对齐)

## 3. 后果

### 3.1 正面

- ✅ 9 organ 概念跟 spirit 蓝图对齐
- ✅ 0 breaking, 下游 import 0 改
- ✅ workspace member 0 改
- ✅ 概念统一通过 ADR 文档化

### 3.2 负面 (诚实标缺)

- ⚠️ 9 organ ≠ 6 organ 实装, 概念不诚实 (per O-5 不假装部分违反)
- ⚠️ 未来 spirit 蓝图升级 (加独立 organ 实装) 需要大重构
- ⚠️ life-force 和 value lib.rs doc 需补 §2.3 规则 (R174 后估补)

## 4. 不漂移

- 0 改 3 re-export crate lib.rs 任何 1 行 (本 ADR 仅记录)
- 0 改 3 实装 crate lib.rs 任何 1 行
- 0 改 workspace version (1.2.0 严守)

## 5. 6 哲学锚穿透

- ✅ **S-1**: 借鉴 Kubernetes facade pattern (kube crate 多 facade 复用底层)
- ✅ **S-2**: 基于 R37-2 commit + workspace member 实查, 0 编造
- ✅ **O-2**: 不上 UI, 纯文档对齐
- ✅ **O-3**: §2.1 表格 1 眼看清 re-export 关系
- ✅ **O-4**: §2.2 选项对比让接手者 1 眼明白选 A 原因
- ✅ **O-5**: §3.2 诚实标 "9 ≠ 6 实装"

## 6. 8 项不修改承诺

- ✅ 不假装: §2.3 标 re-export 关系
- ✅ 编译期 hardcode: workspace member 编译期
- ✅ 不改 LOCKED: 0 触碰
- ✅ 不改 workspace version: 1.2.0
- ✅ 6 哲学锚穿透: §5 自检
- ✅ 不依赖 NewAPI
- ✅ 不重复造轮子: K8s facade pattern
- ✅ 诚实标缺: §3.2 标 "9 ≠ 6"

---

_作者: 楚零_
_日期: 2026-08-14_
