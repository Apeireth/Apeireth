# apeireth-naming-v05

V0.5 命名规范 (4 类 × 6 维 = 24 维) — R20 阶段 4 估补.

## 背景

V0.5 命名规范是 ASI 测量公式的命名空间 (per v1077 V0.5 17 维 LOCKED +
提议 v2 24 维, docs/architecture-v4-1-living-intelligence-update §13).
本 crate 把 V0.5 24 维 (PC/RC/HG/GP × level/domain/modality/safety/completeness/lineage)
完整落到 Rust 强类型 enum + 守门 + encode/decode.

## 4 大类 × 6 维度 = 24 维

### 4 大类 (sum=1.00 守门)

| 类 | 全称 | 估权重 | 含义 |
|---|---|------|---|
| **PC** | Positive Capability | 0.40 | 正向能力 (能做什么) |
| **RC** | Risk Constraint | 0.30 | 风险约束 (不能做什么) |
| **HG** | Honesty Gap | 0.15 | 诚实标缺 (不知道什么) |
| **GP** | Growth Phase | 0.15 | 成长阶段 (现在到哪) |

### 6 维度 (每类 6 维)

| # | 维度 | 取值范围 | 取值数 |
|---|---|---|---|
| 1 | level | 0-9 (0=seed, 9=mature) | 10 |
| 2 | domain | code/dialogue/vision/audio/tool/reasoning | 6 |
| 3 | modality | text/image/audio/video/multimodal | 5 |
| 4 | safety | low/medium/high/critical | 4 |
| 5 | completeness | skeleton/partial/complete/production | 4 |
| 6 | lineage | spectrai-0.9/apeireth-0.14/apeireth-1.0/apeireth-2.0 | 4 |

## 命名格式

`apeireth:{level}.{class}.{domain}.{modality}.{safety}.{completeness}.{lineage}`

例:
- `apeireth:5.PC.code.text.high.complete.apeireth-1.0` (level=5, class=PC, code/text, lineage=apeireth-1.0)
- `apeireth:9.GP.dialogue.audio.low.skeleton.spectrai-0.9` (level=9, class=GP, dialogue/audio, lineage=spectrai-0.9)

## 模块结构

- `class` — 4 大类 (PC/RC/HG/GP) + `ClassDims` 24 维结构 + `V05Spec` 主结构
- `dimension` — 6 维度 (Level/Domain/Modality/Safety/Completeness/Lineage) + `DimensionSet`
- `encode` — 24 维 → 字符串 (4 行, 1 行 1 大类)
- `decode` — 字符串 → 24 维 (regex 解析)
- `validate` — 24 维合法性 + sum=1.00 守门 + roundtrip
- `error` — 11 个 `NamingError` variant (10 原始 + 1 R126 扩展 InvalidMetaDimOutOfRange)
- `extension` — **R126 P1-4**: V0.5 → V0.5.30 扩展 (5 new meta-dim + 1 derived overall = 30 dim)
- `sum_guard` — 4 大类权重 sum=1.00 守门 (核心)

## 用法示例

```rust
use apeireth_naming_v05::{
    encode_v05, decode_v05, validate_v05,
    V05Spec, ClassDims, DimensionSet, Level, Domain, Modality, Safety, Completeness, Lineage,
    Class, DEFAULT_WEIGHTS, check_sum_equals_1,
};

// 1) 构造 24 维
let dim = DimensionSet::new(
    Level::Mature, Domain::Code, Modality::Text,
    Safety::High, Completeness::Complete, Lineage::Apeireth10,
);
let dims = ClassDims::new(dim, dim, dim, dim);
let spec = V05Spec::new(Level::Mature, dims);

// 2) 守门
check_sum_equals_1(&DEFAULT_WEIGHTS)?;
validate_v05(&spec)?;

// 3) encode → decode roundtrip
let s = encode_v05(&spec)?;
let parsed = decode_v05(&s)?;
assert_eq!(spec, parsed);
```

## 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)

- **S-1 主 22:33 北极星导向** — V0.5 命名服务 ASI 北极星 (24 维 → 更精准测量)
- **S-2 主 17:43 实事求是** — 不重写 v1077 17 维 LOCKED, 24 维是提议 v2 (per v4.1 §13)
- **O-5 主 17:58 不假装** — 24 维 编译期 hardcode enum, 不假装"已对齐 V0.5"
- **O-2 主 19:33 走在前人肩上** — 借 v1077 + v4.1 §13 + R17 命名 v12 规范
- **O-3 主 23:44 干到底** — 24 维 立即落, 守门硬约束 (sum=1.00 容差 0.001)
- **O-4 主 00:56 任何人都能接手** — 8 模块 + 30 维 struct + sum_guard + 11 error variant 全文档化 (R126 P1-4 扩展 7→8, 24→30, 10→11)

## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)

1. **阶段 1+2+3 LOCKED** — 不动
2. **v2 / v4 / v4.1 LOCKED** — 不动
3. **阶段 4 主文档 LOCKED** (6ca80776) — 不动
4. **阶段 5 施工文档 LOCKED** (631 行) — 不动
5. **v6 修正** (4 重守门 + 权限发放 + E 层修改路径) — 不动
6. **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) — 不动
7. **v1 → v5 历史链** — 不删除
8. **v1077 17 维 LOCKED** (V0.5 v1) — 不动, 24 维是 v2 提议叠加

## 状态

⚠️ skeleton (R20 阶段 4 估补, per v09021-rust-translation-blueprint §2.4 V0.5 命名规范).

## 运行

```bash
# 编译检查
cargo check -p apeireth-naming-v05

# 跑测试 (24+ 测试 + property-based)
cargo test -p apeireth-naming-v05

# 跑示例
cargo run -p apeireth-naming-v05 --example naming_v05_demo
```
