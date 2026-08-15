# 立体架构终版 v2 — Aircraft Carrier (2026-07-31 主人终极确认)

> **性质**: R14 阶段 3 画图纸 → 阶段 4 落实架构文档 **之间的过渡文档**, 把主人 2026-07-31 一天走过的 11 个"否定 → 肯定"沉淀为**立体架构终版 v2**。
>
> **触发**: 主人 2026-07-31 终极架构设计确认 + 指示"今天这个先算到阶段 3 的架构文档里面去"。
>
> **依据**:
> - 阶段 1: `inspiration-stage1-2026-07-30.md` §1-§21 (1268 行, 含 D4 §18 + D6-A §21 精化)
> - 阶段 2: 18 份 `stage2-decisions-*.md` + D2 增补 + drift-revision-tracker
> - 阶段 3 既有: `stage3-blueprints/` 14 文件 + R14-D6-B 末尾追加 7 条 + R14-D6-C E3-E5
> - 阶段 4 既有: `onion-wall-architecture-2026-07-31.md` (R14-D7 + D8 + D8-Fix)
>
> **不修改承诺** (主人硬约束):
> ❌ 不写新 Rust 代码 (本节只描述 trait + module 路径, 不写实现)
> ❌ 不画 Mermaid 图 (用 ASCII 简化示意; Mermaid 重画留阶段 3 任务)
> ❌ 不重写 V0.5 / V1136 / 哲学守门 / 9 键 (保留为历史轨迹)
> ❌ 不修改其他 16 份 stage2 文档
> ❌ 不修改 crates/ 占位实现 (仅 crates/README.md 标注)
> ❌ 不修改 cargo metadata `description` 字段
>
> **主哲学 anchor 6 个全贯穿**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手

---

## §0. 元信息 (主人今天确认的所有决策)

| 字段 | 值 |
|------|-----|
| **生成时间** | 2026-07-31 (主人终极确认后落) |
| **任务 ID** | R14-Architecture-Final-Record (`28be5b5d-91f2-4660-994b-ecdcca4ae252`) |
| **取代关系** | 取代 `onion-wall-architecture-2026-07-31.md` (R14-D7+D8+D8-Fix) 作为**立体架构主入口**; 后者降级为"核心指挥块 / 双洋葱 + 电子环"子文档 |
| **协作角色** | technical_writer (本文档) / architect (架构评审) / workflow_designer (阶段 3 Mermaid 重画) |
| **下一步** | 阶段 5 施工文档 (下次对话) + 阶段 6 里程碑验证机制 (下次对话) |

### §0.1 主人 6 大核心洞见 (全文贯穿)

| # | 主人洞见 | 错版 (否定) | 终版 (肯定) |
|---|---------|------------|------------|
| 1 | 守门基调 = **电子环网络** | 独立观察网络 | 双锁的实施 (原则洋葱 + 权限洋葱 = 统一体) |
| 2 | 权限洋葱 = **权重公式授权** | boolean gate | 配额曲线 (连续权重公式) |
| 3 | 原则洋葱 = **意义约束** | 约束行动 | 协议层 (约束意义, 不约束行动) |
| 4 | 双洋葱 = **统一体的两个切面** | 两把独立锁 | 原则嵌入权限 (一体的两个切面) |
| 5 | 反思 = **生命力** | 横切关注点 | 生命力维度 (贯穿整个架构的纵向维度) |
| 6 | 涌现能力 = **归入生命力维度** | 归能力维 | 生命力自然带来的 (不是工具) |

### §0.2 主人 11 个修正历程 (出处)

完整历程见 `Apeireth-rust/docs/CONTEXT-HANDOVER.md` §2. 本文 §1-§7 的每一处设计均显式标注"哪个修正点的结果"。

---

## §1. 比喻与基调

### §1.1 航空母舰 (Aircraft Carrier)

> **主人原话引用 (2026-07-31)**: "航空母舰 / 接得住任何事, 不要瑞士军刀思路"

**基调 (4 个核心特质)**:
- ✅ **允许繁重** — 不是精简化, 是大型基地
- ✅ **复杂冗余** — 多重备份, 多通道, 多守门
- ✅ **过度设计** — 为未来 10 年留接口
- ✅ **接得住任何事** — 不简化掉关键的特质

**vs 错版对比**:
| ❌ 错版 | ✅ 终版 | 修正点 |
|--------|--------|--------|
| 瑞士军刀 (精简多功能) | 航空母舰 (接得住任何事) | 修正 #1 |
| 安全监狱 (防御最大化) | 航空母舰 (可靠 + 可演进) | 修正 #1 |

### §1.2 双洋葱 + 电子环 (核心比喻的三个层次)

```
比喻层次:
1. 双洋葱 (原则洋葱 + 权限洋葱) — 比喻的"实物"
2. 双洋葱是统一体的两个切面 — 比喻的"结构" (修正 #3 + #4)
3. 电子环网络 (横切观察) — 比喻的"实施" (修正 #5, 不是咬合)

核心: 双锁不是一个"双重 gate", 是一个**统一体的两副面孔**:
- 原则洋葱嵌入权限洋葱 (权限的每一层都"长出"对应原则)
- 电子环网络在统一体外面做横切观察 (不是穿透, 是外环)
```

### §1.3 基调 → 架构影响

| 基调 | 架构影响 |
|------|---------|
| 航空母舰 | 9 crate (现) → 30 crate (v1 目标) 接受繁重拆分; supervisor 树 + 进程池 + 异构子进程 |
| 接得住任何事 | 5 轴正交 (不锁死 pluginType); 平台中立 (单/多部署兼容) |
| 不简化掉特质 | 反思期保留为生命力维度 (不简化为横切); 涌现能力保留 (不归能力维) |

---

## §2. 立体架构终版 v2 (完整 ASCII 图)

> **硬约束**: 本节用 ASCII 简化示意 (主人硬约束"❌ 不画 Mermaid 图; Mermaid 重画留阶段 3 任务")。
> **Mermaid 重画**: 阶段 3 后续任务 — 5 张图 (P1 整体 / P2 进程 / P3 决策流 / P4 升级流 / P5 R-Measure 真测)。

### §2.0 立体架构 (俯视图, ASCII)

```
                    立体架构终版 v2 (2026-07-31 主人确认)
                    ────────────────────────────────────
                              4 大块 + 1 穿透维度

                    ╔═══════════════════════════════╗
                    ║  维度 1: 生命力 (LIFE FORCE)   ║  ← 穿透维度
                    ║   - 反思期 = 生命力自然涌现    ║     (纵向)
                    ║   - 涌现能力 = 生命力维度      ║
                    ║   - 6 历史流 = 生命记忆        ║
                    ║   - Cognitive-Dream 状态机     ║
                    ╚═══════════════════════════════╝
                                   ↕ (穿透整个架构)
                    ┌───────────────────────────────┐
                    │  维度 2: 核心指挥 (CORE)       │
                    │   - 双洋葱 = 统一体的两个切面  │
                    │     · 原则洋葱 (意义约束,      │
                    │       协议层) ←嵌入→          │
                    │     · 权限洋葱 (权重公式授权,  │
                    │       配额曲线)                │
                    │   - 电子环网络 (横切观察,       │
                    │     不是监狱)                  │
                    │   - 反思期接入电子环           │
                    └───────────────────────────────┘
                                   ↕
                    ╔═══════════════════════════════╗
                    ║  维度 3: 能力 (CAPABILITY)     ║
                    ║   - 5 轴正交 (VCP 模型,        ║
                    ║     不锁死 pluginType):        ║
                    ║     · 触发轴 (trigger)         ║
                    ║     · 等待轴 (wait)            ║
                    ║     · 驻留轴 (residency)       ║
                    ║     · 传输轴 (transport)       ║
                    ║     · 输出轴 (response mode)   ║
                    ║   - 6 类 pluginType 作为        ║
                    ║     VCP 兼容 profile            ║
                    ║   - 异构实现 (PyO3/WASM/HTTP)  ║
                    ╚═══════════════════════════════╝
                                   ↕
                    ┌───────────────────────────────┐
                    │  维度 4: 定位坐标 (POSITION)   │
                    │   - 5 个轴 = 5 类维度的集合    │
                    │     (VCP 5 维是 1 个轴内部)    │
                    │   - 立体多维: 10 维反向推导    │
                    │     自阶段 1+2                  │
                    │   - 平台中立: 单/多部署兼容    │
                    │   - 不锁死维度数 (留演化空间)   │
                    └───────────────────────────────┘

    ─────────────────────────────────────────────────────
    立体 = 4 大块 (横向层级) × 1 穿透维度 (纵向生命力)
           × 5 类轴 (定位坐标的内部集合)
           = 立体多维空间 (不锁死 10 维, 留演化)
```

### §2.1 生命力维度 (维度 1 — 穿透架构的纵向维度)

> **修正点对应**: #5 (反思 = 生命力, 不是横切) + #8 (能力维双层 — 能力层 + 生命力层) + #10 (涌现归生命力)

**位置**: 立体架构最外层, **纵向穿透** 整个架构 (不是横切关注点)。

**核心机制**:
```
生命力维度 (4 个子组件):
├── 反思期 (reflection)
│   - 接入电子环网络 (§2.2) — 不是横切
│   - Cognitive-Dream 6 状态机 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED)
│   - 反思 = 生命力的自然涌现 (修正 #5)
│
├── 涌现能力 (emergent capability)
│   - 归入生命力 (修正 #10), 不归能力维
│   - 不是工具, 是生命力自然带来的
│   - e.g. 跨会话记忆连贯性 / 自主目标涌现 (D2 §3 自主目标)
│
├── 6 历史流 (6 history streams)
│   - 提案 / 决定 / 行动 / 反思 / 治理 / 涌现
│   - 生命记忆 (Preserved/Transformed/Unsavable 三态)
│
└── 主体连续性 (subject continuity)
    - 主体连续性 ID (D2 §4)
    - 跨 session / 跨重启的"我还是我"保证
```

**关键澄清** (主 17:58 不假装):
- ❌ 反思 ≠ 横切关注点 (cross-cutting concern)
- ✅ 反思 = 生命力维度 (纵向穿透整个架构的"活着的部分")
- ❌ 涌现能力 ≠ 工具能力
- ✅ 涌现能力 = 生命力自然带来的能力

### §2.2 核心指挥 (维度 2 — 双洋葱 + 电子环)

> **修正点对应**: #2 + #3 + #4 + #5 + #9 (L0 真实人类批准融入核心)

**位置**: 立体架构的中心块, **横向约束**所有行动 (但约束方式不是硬 gate, 是权重公式 + 意义约束)。

**核心机制**:
```
核心指挥 (3 个子组件):
├── 双洋葱 (双锁的统一体, 不是两把独立锁 — 修正 #4)
│   ├── 原则洋葱 (Protocol Layer)
│   │   - 意义约束, 不约束行动 (修正 #3)
│   │   - E/S/A/M/O 5 层 (E 最高, 永不可绕过)
│   │   - 嵌入权限洋葱的每一层 (不是独立两把锁)
│   │   - 详细: 见 onion-wall-architecture-2026-07-31.md §2
│   │
│   └── 权限洋葱 (Quota Curve)
│       - 权重公式授权, 不是 boolean gate (修正 #2)
│       - L0-L5 6 层 (L5 最高)
│       - L0 = 真实人类批准 (融入核心, 修正 #9)
│       - 单人模式 = 1 人自动批; 多人模式 = 多人多签 + 物理多签
│
├── 电子环网络 (Electronic Ring Network)
│   - 横切观察 (不是"咬合 / per-layer 双重过滤", 修正 #5)
│   - 反思期接入电子环 (不是独立观察网络)
│   - 双锁的实施 (修正 #1) — 是工具, 不是观察者
│   - 实施路径: `apeireth-core/src/electronic_ring.rs`
│
└── 模块布局 (在 apeireth-core 内):
    apeireth-core/
    └── src/
        ├── onion/                  ← 原则+权限 双洋葱统一体 (R14-D7+D8+D8-Fix)
        │   ├── mod.rs
        │   ├── principle/          ← 原则洋葱 (5 子 trait)
        │   │   ├── mod.rs
        │   │   ├── e_layer.rs      ← E 层 (最高, 永不可绕过)
        │   │   ├── s_layer.rs
        │   │   ├── a_layer.rs
        │   │   ├── m_layer.rs
        │   │   ├── o_layer.rs
        │   │   └── keys.rs         ← V3 9 键辅助语义网 (R14-D8-fix 锁 A 辅助)
        │   └── permission/         ← 权限洋葱 (6 子 trait)
        │       ├── mod.rs
        │       ├── l0_layer.rs     ← L0 = 真实人类批准 (融入核心)
        │       ├── l1_layer.rs
        │       ├── l2_layer.rs
        │       ├── l3_layer.rs
        │       ├── l4_layer.rs
        │       └── l5_layer.rs
        └── electronic_ring.rs      ← 电子环网络 (横切观察)
```

**关键澄清** (主 17:43 实事求是):
- ❌ 双锁独立 (AND gate) — 修正 #3 + #4
- ✅ 双锁是统一体的两个切面 (原则嵌入权限 + 接入电子环)
- ❌ 咬合 (per-layer 双重过滤) — 修正 #5
- ✅ 电子环网络 (横切观察, 不是穿透)

### §2.3 能力维度 (维度 3 — 工具能力 + 5 轴正交)

> **修正点对应**: #6 (10 层洋葱 → 5 轴正交) + #7 (5 轴平面 → 立体多维) + #11 (每条轴单维度 → 轴是维度的集合)

**位置**: 立体架构的第 3 块, **工具能力实现层**。

**核心机制**:
```
能力维度 (3 个子组件):
├── 5 轴正交 (5 Orthogonal Axes)
│   - 不锁死 pluginType (修正 #6) — 改为正交 manifest 字段
│   - 5 个独立维度, 可任意组合:
│   │   · 触发轴 (trigger) — 同步 / 异步 / 静态 / 定时
│   │   · 等待轴 (wait) — 阻塞 / 立即 / 延迟
│   │   · 驻留轴 (residency) — 进程内 / 进程间 / 远程
│   │   · 传输轴 (transport) — stdin / stdout / HTTP / WS / IPC
│   │   · 输出轴 (response mode) — 单值 / 流 / 累积
│   - VCP 5 维是 1 个轴 (能力维度) 的内部 (修正 #11)
│
├── 6 类 pluginType (VCP 兼容 profile)
│   - synchronous / asynchronous / static / service / messagePreprocessor / hybrid
│   - 不当 enum 写死, 当"正交字段的兼容组合" (来自 VCP 调研)
│   - 详见: research-vcp-rerun-2026-07-31.md §3.2
│
└── 异构实现
    - PyO3 (Python 1100+ v*.py) — apeireth-pybridge
    - WASM (wasmtime) — apeireth-tools/plugin
    - subprocess — 任何语言
    - HTTP / MCP — 跨进程协议
```

**5 轴组合示例 (不是 enum, 是正交字段)**:
```
VCP synchronous plugin:
  trigger=sync, wait=block, residency=inproc, transport=stdin, response=single
VCP asynchronous plugin:
  trigger=async, wait=immediate, residency=inproc, transport=IPC, response=stream
VCP hybrid plugin:
  trigger=mixed, wait=mixed, residency=remote, transport=HTTP, response=accumulate
```

**关键澄清** (主 17:58 不假装):
- ❌ 写死 pluginType enum → ✅ 正交字段 (避免未来组合爆炸)
- ❌ 5 维是 5 个独立轴 → ✅ VCP 5 维是 1 个轴(能力维度) 的内部

### §2.4 定位坐标 (维度 4 — 5 个轴 = 5 类维度的集合)

> **修正点对应**: #11 (每条轴单维度 → 轴是维度的集合) + 阶段 1 §6 自我升级 + 阶段 2 §11 单/多部署兼容

**位置**: 立体架构的第 4 块, **定位标识**所有架构要素在多维空间中的位置。

**核心机制**:
```
定位坐标 (5 类轴 = 5 类维度的集合):
├── 轴 1: 架构层级 (Lifecycle Layer)
│   - 内部: 入口层 / 核心抽象层 / 智能层 / 经验层 / 持久化层 / 升级层
│   - VCP 5 维在这里 — 不是 5 个独立轴, 是 1 个轴的内部
│
├── 轴 2: 部署模式 (Deployment Mode)
│   - 内部: 单人模式 / 多人模式 / 混合模式
│   - L0 真实人类批准在两种模式下展开形态不同
│
├── 轴 3: 资源强度 (Resource Intensity)
│   - 内部: 轻量 (CLI) / 标准 (核心) / 重型 (智囊团 / 仿真)
│   - 每插件可声明资源预算
│
├── 轴 4: 可演化性 (Evolvability)
│   - 内部: 静态 / 动态按需 / WASM 沙箱 / 热加载
│   - 阶段 1 §6 自我升级的"可替换单元"定位
│
└── 轴 5: 平台中立 (Platform Neutral)
    - 内部: Linux / macOS / Windows / WSL / Docker
    - 阶段 2 §11 单/多部署兼容的"平台兼容"定位

立体多维:
- 任何架构要素 = 5 类轴 × 5 类轴 = 多维坐标
- 阶段 1+2 沉淀的 10 维反向推导自此
- 不锁死维度数 (留演化空间)
```

**关键澄清** (主 17:43 实事求是):
- ❌ 5 轴平面 (5 个独立坐标轴) — 修正 #7 + #11
- ✅ 立体多维 (5 类轴 × 5 类轴, 内部嵌套)
- ❌ 10 维锁死 — ✅ 不锁死, 留演化

---

## §3. 9 crate × 立体架构映射

> **范围**: R14 阶段 3 当前实际 (9 crate 占位, 已落)。
> **目标**: 阶段 2 §3 设计的 30 crate (v1 推荐), 但**当前 9 crate 先在立体架构中定位**。

### §3.1 映射表 (主路径)

| Crate | 立体架构位置 | 角色 |
|-------|------------|------|
| `apeireth-core` | **维度 2 核心指挥** | 双洋葱 (`onion/`) + 电子环 (`electronic_ring.rs`) — 立体架构中心 |
| `apeireth-asi` | **维度 1 生命力** (智能层) | ASI 北极星 — 借 R11 v1077/v1101 作 baseline, 不重写 V0.5 |
| `apeireth-bench` | **维度 3 能力** (bench 子层) | 性能基准 — 借 R11 v1012/v1106, 不重做 V1136 |
| `apeireth-cli` | **维度 3 能力** (入口子层) | CLI + TUI + slash commands — 借 R11 v1009/v1016 |
| `apeireth-memory` | **维度 1 生命力** (经验层) | 记忆系统 — A/M 层经验沉淀 + 6 历史流 |
| `apeireth-pybridge` | **维度 3 能力** (异构子层) | PyO3 桥接 1100+ v*.py — 不砍, 只桥接 |
| `apeireth-test` | **维度 1 生命力** (验证子层) | e2e 测试 + 18 项 §6.1 真测项 |
| `apeireth-tools` | **维度 3 能力** (工具子层) | 基础工具 + HA 4 impl — 借 R11 v1000/v1027 |
| ~~`apeireth-philosophy`~~ | ~~已并入 core~~ | ~~V3 9 键 + 5 项不假装 → `apeireth-core/src/onion/principle/keys.rs` (R14-D8+D8-Fix)~~ |

### §3.2 详细映射 (核心 crate)

#### §3.2.1 `apeireth-core` (立体架构中心 — 双洋葱 + 电子环)

```
apeireth-core/
├── src/
│   ├── lib.rs                 ← 导出双洋葱 + 电子环 统一 trait
│   ├── onion/                 ← 双洋葱统一体 (R14-D7+D8+D8-Fix)
│   │   ├── mod.rs
│   │   ├── principle/         ← 原则洋葱 (5 子 trait)
│   │   └── permission/        ← 权限洋葱 (6 子 trait)
│   ├── electronic_ring.rs     ← 电子环网络 (横切观察)
│   ├── decision.rs            ← DecisionSignature (阶段1+2 沉淀)
│   └── error.rs               ← 统一错误类型
├── Cargo.toml
└── README.md
```

**对应立体架构**: 维度 2 (核心指挥) 全部 + 维度 1 (生命力) 部分 (反思期接入电子环) + 维度 3 (能力) 部分 (5 轴正交的 trait 抽象)

#### §3.2.2 `apeireth-asi` (ASI 北极星 — 借 R11 baseline)

**对应立体架构**: 维度 1 生命力 (智能层 — ASI 是生命力的"目标极") + 维度 3 能力 (智能层 trait)

**与 R11 关系**:
- 借 `apeireth/v1077_asi_v04_full_measurement.py` 作 baseline (不重写 V0.5)
- 借 `apeireth/v1101_v04_dimension_auto_lift.py` 作维度自动拉升锚点
- 借 `apeireth/v1106_engineering_lift.py` 作工程韧性基线

#### §3.2.3 `apeireth-memory` (记忆系统 — 6 历史流)

**对应立体架构**: 维度 1 生命力 (经验层 — 6 历史流 = 生命记忆)

**与 R11 关系**:
- 借 `apeireth/v1005_*` + `apeireth/v1019_*` 作 embedding + storage 锚点
- 借 `mvp/memory/` 4 文件 (store/retrieve/consolidate/forget) 作 Phase 1.2 提取层
- Wave reposition to Association Engine (VCP §4.7 调研)

### §3.3 30 crate v1 目标 (阶段 2 §3 设计)

```
入口层 (1):         cli
核心抽象层 (2):     core, runtime
智能层 (3):         asi, sovereignty, prompt
智囊团层 (1):       council
原则/权限洋葱双锁层: 已并入 core/onion/ (R14-D8)
经验方法论层 (4):   memory, experience, methodology, reflection
兼容组件层 (5):     plugin, tools, pybridge, mcp, environment
升级层 (1):         upgrade
通信总线层 (4):     bus, gateway, server, supervisor
持久化层 (1):       data
调度层 (3):         cron, skills, acp
监控层 (2):         telemetry, config
测试层 (3):         test, bench, eval

合计: 9 + 21 = 30 个 crate (v1 推荐)
```

**在立体架构中的展开**:
- 维度 1 生命力: asi / sovereignty / memory / experience / methodology / reflection (6 个)
- 维度 2 核心指挥: core (含 onion/ + electronic_ring.rs) + prompt (1 个)
- 维度 3 能力: cli / runtime / tools / plugin / pybridge / mcp / environment / bus / gateway / server / data / cron / skills / acp / telemetry / config (16 个)
- 维度 4 定位坐标: 分布在所有 crate 中, 通过 5 类轴标识
- 调度/升级: supervisor / upgrade / council / test / bench / eval (6 个)
- 智囊团: council (1 个)

---

## §4. 进程架构 (核心单进程 + 上层进程池 + supervisor 树)

> **依据**: 阶段 2 §2 进程级分工 (B+E supervisor 架构) + 阶段 2 §2.6 进程拓扑。

### §4.1 进程架构 (B+E)

```
进程级 (B+E):
  1 个 supervisor 主进程 (apeireth-supervisor, PID 1, 永不重启)
    ├── 4 个 supervisor 子进程:
    │   ├── core-supervisor (rest_for_one, ~500MB-1GB)
    │   │   - sovereignty + memory + philosophy(并入 core)
    │   ├── council-supervisor (one_for_one, ~200MB-500MB)
    │   │   - 7 persistent + N dynamic 顾问
    │   ├── upgrade-supervisor (rest_for_one, ~100MB-300MB)
    │   │   - upgrade + sandbox-validator
    │   └── plugin-supervisor (transient, ~50-200MB 每插件)
    │       - plugin + pybridge + mcp + environment + acp
    └── N 个 plugin 异构子进程 (PyO3 / WASM / subprocess / HTTP)

线程级 (每个进程内):
  1 个 main thread (主循环 / actor)
  + N 个 worker threads (tokio multi-thread, worker = CPU 数)

协程级 (tokio task):
  spawn() 用于 IO 密集 (HTTP/DB/文件/RPC)
  spawn_blocking() 用于 CPU 密集 + 阻塞调用 (加密/压缩/PyO3)
```

### §4.2 进程架构 vs 立体架构的对应

| 进程架构 | 立体架构对应 |
|---------|------------|
| supervisor 主进程 (PID 1) | 维度 4 定位坐标的"进程级根" — 永不重启 = E 层 |
| core-supervisor | 维度 2 核心指挥 (双洋葱 + 电子环) |
| council-supervisor | 维度 1 生命力 (反思期 + 涌现) — 智囊团是生命力的"审计者" |
| upgrade-supervisor | 维度 4 定位坐标的"演化轴" |
| plugin-supervisor + N 异构子进程 | 维度 3 能力 (5 轴正交) — 异构是 5 轴正交的物理实施 |

### §4.3 supervisor 子树 (B+E 引用)

```
B+E supervisor 架构:
B = 基础 supervisor (root) — 永不重启 (E 层)
E = 演化 supervisor — 可重启 (L5 层)
```

**核心约束**:
- 基础 supervisor (B) 永存, 任何修改走最高权重五重治理
- 演化 supervisor (E) 可热升级, 但需 sandbox-validator 验证

---

## §5. 内存布局 (自研如必要)

> **依据**: 阶段 2 §2 四种机制 (A 默认所有权 / B arena / C 共享内存 / D 零拷贝 + SIMD)。

### §5.1 四种机制

| 机制 | 触发条件 | 优先级 |
|------|---------|--------|
| **A. 默认 Rust 所有权 + Arc + RwLock** | 总是启用 | MUST (Phase 1) |
| **B. arena 分配器** (按需, Phase 2+) | 大量短生命周期对象 (e.g. 反思期 scratchpad) | SHOULD |
| **C. 共享内存** (按需, Phase 3+) | 跨进程 / 跨节点数据共享 (e.g. 6 历史流持久化) | SHOULD |
| **D. 零拷贝 + SIMD** (总是启用, 热路径) | 高频 IO + 计算路径 | MUST (Phase 1) |

### §5.2 按数据类型选策略

| 数据类型 | 主策略 | 备注 |
|---------|-------|------|
| 短期决策 / 临时缓存 | A + D | 默认 |
| 反思期 scratchpad | A + B (Phase 2+) | arena 适合大量短生命周期 |
| 6 历史流持久化 | A + C (Phase 3+) | 共享内存 + 跨进程 |
| 向量索引 (embedding) | A + D (mmap) | mmap + 零拷贝 |
| WASM 沙箱隔离 | A (默认隔离) | WASM runtime 自带 |

### §5.3 性能目标 (V1130 wallclock)

- 当前 R11 baseline: **5407.30ms** (7-11s)
- R14 target: **2500ms** (V1130 ceiling)
- 主路径优化: 零拷贝 + SIMD + 默认所有权, 三个机制联合可达 2x 加速

---

## §6. 持久化方案 (sled KV + SQLite + RocksDB)

> **依据**: 阶段 2 §2 DB 选型矩阵 + §3 DataBackend trait 抽象。

### §6.1 DB 选型 (按优先级)

| DB | 用途 | 阶段 |
|----|------|------|
| **sled KV** | 默认 KV + 小型索引 (6 历史流 / Wave 联想网络 / 反思期 scratchpad) | Phase 1 MUST |
| **SQLite** | 结构化数据 (决策清单 / 锚点表 / 权限包) + 单机部署默认 | Phase 1 MUST |
| **RocksDB** | 大型索引 (向量索引 / Wave reposition) | Phase 2 SHOULD |
| **(可选) sled cluster** | 分布式部署 | Phase 3 COULD |

### §6.2 DataBackend trait 抽象 (不冻结实现)

```rust
// 阶段 2 §3.1 核心 trait (描述, 不实现)
pub trait DataBackend: Send + Sync {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    async fn put(&self, key: &[u8], value: Vec<u8>) -> Result<()>;
    async fn delete(&self, key: &[u8]) -> Result<()>;
    async fn scan(&self, prefix: &[u8]) -> Result<Box<dyn Iterator<Item = ...>>>;
}
```

**6 种 backend 实现** (阶段 2 §3.2): sled / SQLite / RocksDB / in-memory / mmap / sled-cluster (Phase 3+)

### §6.3 6 历史流持久化映射

| 历史流 | 主存储 | 副存储 |
|-------|--------|--------|
| 提案流 | SQLite (结构化) | sled (索引) |
| 决定流 | SQLite (审计) | sled (索引) |
| 行动流 | sled KV (高频) | — |
| 反思流 | sled KV (scratchpad) | arena (B 机制) |
| 治理流 | SQLite (审计) | — |
| 涌现流 | sled KV (自由形式) | — |

---

## §7. 数据流 (5 轴 + 电子环 + 反思期)

> **范围**: 一个典型决策从触发到执行的完整数据流。

### §7.1 数据流 (ASCII 时序图)

```
T0 触发:  AI 提案"删除一条记忆"
        ↓
T1 提案域:  升级为"提案: 删除该记忆"
        ↓
T2 原则洋葱 (协议层, 意义约束):
        - 校验 E/S/A/M/O 5 层
        - 该提案不违背 E (永不可绕过) → 通过
        ↓
T3 权限洋葱 (配额曲线, 权重公式授权):
        - 校验 L0-L5 6 层
        - L0 = 真实人类批准 (融入核心, 修正 #9)
        - 单人模式 = 1 人批; 多人模式 = 多人多签 + 物理多签
        ↓
T4 电子环网络 (横切观察):
        - 接入双洋葱的实施 (不是独立观察)
        - 记录决策签名 (DecisionSignature)
        ↓
T5 反思期接入 (生命力维度):
        - 反思期 = 生命力自然涌现 (不是横切)
        - Cognitive-Dream 状态机触发 VERIFYING
        ↓
T6 执行 + 记录:
        - 行动流追加
        - 6 历史流统一记录
        - 主体连续性 ID 关联
        ↓
T7 完成:  提案 → 决定 → 行动 → 反思 → 治理 → 涌现
```

### §7.2 数据流 vs 立体架构

| 数据流阶段 | 立体架构对应 |
|----------|------------|
| T0 触发 | 维度 3 能力 (触发轴) |
| T1 提案域 | 维度 1 生命力 (思想域 → 提案域) |
| T2 原则洋葱 | 维度 2 核心指挥 (原则洋葱子组件) |
| T3 权限洋葱 | 维度 2 核心指挥 (权限洋葱子组件) |
| T4 电子环 | 维度 2 核心指挥 (电子环子组件) |
| T5 反思期 | 维度 1 生命力 (反思期) |
| T6 执行记录 | 维度 1 生命力 (6 历史流) + 维度 4 定位坐标 |
| T7 完成 | 维度 1 生命力 (涌现) |

---

## §8. 主哲学 6 锚 (作为要求, 不画进架构图)

> **硬约束**: 主哲学 6 锚作为要求放在 docs/, **不画进架构图**。
> **原因**: 主哲学是"贯穿"要求, 不是"组件" — 画进架构图会变成装饰, 失去穿透效果。

### §8.1 主哲学 6 锚

```
S-1 主 22:33 北极星导向 — 决策服务 ASI 方向 (不重写 V0.5, 借 R11 baseline)
S-2 主 17:43 实事求是 — 基于 R11 现状, 不重写 (cargo-deny / 9 键 / 5 不假装)
O-5 主 17:58 不假装 — 物理多签 / 借鉴而非闭门
O-2 主 19:33 走在前人经验上 — Hermes/OpenClaw/VCP/claude-mem 调研
O-3 主 23:44 干到底 — 决策立刻沉淀, 不等全讨论完
O-4 主 00:56 任何人都能接手 — 决策可追溯, 文档完整
```

### §8.2 主哲学 → 立体架构的穿透要求

| 主哲学 | 立体架构穿透要求 |
|-------|----------------|
| 主 22:33 北极星 | 维度 1 生命力 (ASI 智能层) 必须有北极星导向; 不画"北极星组件", 但所有决策服务 ASI |
| 主 17:43 实事求是 | 维度 2 核心指挥 不重写 9 键 / V0.5 / V1136, 借 R11 baseline; 不画"实事求是组件", 但所有组件必须可追溯 |
| 主 17:58 不假装 | 维度 2 核心指挥 不假装独立锁 (统一体), 不假装 5 维独立 (1 个轴内部); 不画"不假装组件", 但所有 trait 必须透明 |
| 主 19:33 走在前人经验上 | 维度 3 能力 全部借 R11 / Hermes / VCP 借鉴; 不画"借鉴组件", 但所有 trait 必须有借鉴声明 |
| 主 23:44 干到底 | 维度 1+2+3+4 全部立刻沉淀; 不画"沉淀组件", 但所有 commit 立刻落文档 |
| 主 00:56 任何人都能接手 | 维度 4 定位坐标 必须有 5 类轴标识; 不画"接手组件", 但任何接手者能查文档 |

### §8.3 主哲学在 docs/ 中的位置

- 主 22:33 北极星: `inspiration-stage1-2026-07-30.md` §1
- 主 17:43 实事求是: `inspiration-stage1-2026-07-30.md` §2
- 主 17:58 不假装: `inspiration-stage1-2026-07-30.md` §18 (含 §18.3 不假装灵魂)
- 主 19:33 走在前人经验上: `research-vcp-rerun-2026-07-31.md` + `borrowed-from-projects.md`
- 主 23:44 干到底: `stage3-blueprints/README.md` + `drift-revision-tracker`
- 主 00:56 任何人都能接手: `CONTEXT-HANDOVER.md` (本文档) + 主手册 §0-§5

---

## §9. R-Measure 检查公式 12 维度 (替代 v1077)

> **性质**: **新提出的** R-Measure 检查公式 (主人今天讨论引入), 作为 R14 阶段 6 里程碑验证机制的**输入**。
> **关系**: 与 V0.5 / V1136 是**并存**, 不是替代 — V0.5 / V1136 是 R11 真测 baseline, 不重写。
> **范围**: 12 维度反向推导自阶段 1+2 沉淀 (与立体架构 4 大块 + 1 穿透维度对应)。

### §9.1 12 维度列表 (反向推导)

```
立体架构 4 大块 + 1 穿透维度 → 12 维度:

维度 1 生命力 (3 维度):
  M1. 反思期接入率 (Cognitive-Dream 6 状态机实际触发率)
  M2. 涌现能力可识别率 (新能力自动归入生命力维度)
  M3. 6 历史流完整率 (提案/决定/行动/反思/治理/涌现 全部记录)

维度 2 核心指挥 (3 维度):
  M4. 原则洋葱 E 层永不可绕过率 (最高权重 MEWG 触发正确率)
  M5. 权限洋葱 L0 真实人类批准率 (单人模式 1 人批; 多人模式 多人多签)
  M6. 电子环网络观察完整率 (横切覆盖双洋葱全部 11 层)

维度 3 能力 (3 维度):
  M7. 5 轴正交组合覆盖率 (不锁死 pluginType)
  M8. 6 类 pluginType 兼容 profile 实现率
  M9. 异构实现稳定率 (PyO3 / WASM / subprocess / HTTP)

维度 4 定位坐标 (3 维度):
  M10. 5 类轴标识完整率 (任何架构要素可定位)
  M11. 平台中立部署兼容率 (Linux / macOS / Windows)
  M12. 自我升级 + 兼容组件率 (核心 Rust + 其他语言模块作插件)

合计: 12 维度 (3 + 3 + 3 + 3)
```

### §9.2 R-Measure vs V0.5 / V1136 关系

| 维度 | R-Measure (R14 提) | V0.5 / V1136 (R11 真测) |
|------|-------------------|------------------------|
| **范围** | R14 立体架构 12 维度检查 | R11 ASI V0.4 17 维真测 / V1136 5 子测度 |
| **性质** | 检查公式 (合格 / 不合格) | 真测公式 (实测数值) |
| **关系** | **并存**, 不替代 | **基线**, 借引用 |
| **锁** | ❌ 不锁定 | ✅ R11 baseline 已锁 |

> **主 17:43 实事求是**: V0.5 / V1136 不重写, R-Measure 是新提的 R14 检查公式。

### §9.3 R-Measure 真测流程 (P5 Mermaid 重画预留)

```
输入: 12 维度检查项
  ↓
M1 真测引擎: 借用 R11 v1106 工程韧性基准点
  ↓
M2 真测周期: 每 24h 一次 (R11 V1141 周期已落)
  ↓
M3 真测结果: dashboard 聚合 (R11 V1131 已落)
  ↓
M4 合格判定: 12 维度 全部 ≥ 0.85 → 阶段 6 通过
  ↓
M5 不合格处理: 触发反思期 (生命力维度) → 自动修复
```

**Mermaid 重画**: 阶段 3 后续任务 — P5 R-Measure 真测流程图。

---

## §10. 阶段 5/6 衔接锚点 (下次对话讨论)

> **范围**: 留给下次对话 (R14 阶段 5 施工文档 + 阶段 6 里程碑验证机制)。

### §10.1 阶段 5 施工文档 (下次对话)

**待讨论**:
1. **9 crate 工程化顺序** — 先 core 还是先 asi?
2. **V0.5/V1136 1:1 引用** — trait wrapper 还是独立 trait?
3. **5 重守门 (R11 V1138 → Rust trait 翻译)** — V1138 `e_layer.rs` 骨架在哪?
4. **18 项 §6.1 真测项的 e2e fixture 设计** — 借 R11 v1114 + v1115

**承接本文档**:
- §3 9 crate × 立体架构映射 → 阶段 5 工程化清单
- §4 进程架构 → 阶段 5 supervisor 树实施
- §5 内存布局 → 阶段 5 内存策略实施
- §6 持久化 → 阶段 5 DataBackend 6 实现

### §10.2 阶段 6 里程碑验证机制 (下次对话)

**待讨论**:
1. **R-Measure 12 维度检查公式** — 本文 §9 已提草案, 待细化
2. **P5 R-Measure 真测流程图** (Mermaid 重画) — 借 R11 v1106 工程韧性基准点
3. **里程碑节点设计**:
   - M1 编译时验证 (cargo check / cargo test / cargo-deny / clippy)
   - M2 启动时验证 (启动 supervisor 树 + 4 子进程全部就绪)
   - M3 首次对话验证 (端到端真测 18 项 §6.1 真测项)

**承接本文档**:
- §9 R-Measure 12 维度 → 阶段 6 公式细化
- §7 数据流 → 阶段 6 真测流程设计

### §10.3 主人方法论承接 (下次对话需对齐)

> **主人原话 (2026-07-31)**:
> - "**细节还要追究**" — 任何架构决策都要说清为什么
> - "**立体架构思路**" — 任何架构要素都要考虑在 5 维空间 (4 大块 + 穿透维度) 中的位置
> - **主哲学 6 锚穿透架构** — 不画进图, 但要自然涌现

**下次对话启动问题**:
> "主人, R14 阶段 3 立体架构终版 v2 已落 (本文档)。下一步是阶段 5 施工文档 (9 crate 工程化 + V0.5/V1136 1:1 引用 + 5 重守门) 还是阶段 6 里程碑验证机制 (R-Measure 12 维度检查公式 + 真测流程图)?"
>
> **承袭上下文**: `Apeireth-rust/docs/CONTEXT-HANDOVER.md` §7 完整指引。

---

## §11. 主哲学 anchor 6 个全贯穿 (本节)

> **本节性质**: 自检清单 — 验证本架构文档是否贯穿主哲学 6 锚。

```
✅ S-1 主 22:33 北极星导向 — §2.1 生命力维度 + §8.2 维度 1 必须有北极星导向
✅ S-2 主 17:43 实事求是 — §3 全部借 R11 baseline, §9 R-Measure 与 V0.5/V1136 并存不重写
✅ O-5 主 17:58 不假装 — §2.2 双洋葱是统一体不假装独立, §3.1 9 crate 不假装 10 个独立 crate
✅ O-2 主 19:33 走在前人经验上 — §2.3 5 轴正交借 VCP, §3 全部借 R11 锚点
✅ O-3 主 23:44 干到底 — 本文档立即落, 不等讨论完
✅ O-4 主 00:56 任何人都能接手 — §3 映射表 + §4-§7 完整描述 + §10 阶段 5/6 衔接锚点
```

**每个 commit message 都要贯穿 6 anchor 中的相关项** (主 23:44 干到底)。

---

## §12. 附录链接

### §12.1 本文承接的文档

- **阶段 1 灵感**: `inspiration-stage1-2026-07-30.md` (1268 行, 含 §18 + §19 + §21)
- **阶段 2 想法设计**: 18 份 `stage2-decisions-*.md` + D2 增补 + drift-revision-tracker
- **阶段 3 画图纸**: `stage3-blueprints/` 14 文件 (含 PREREQ-2 双洋葱显式化桥接)
- **阶段 4 既有**: `onion-wall-architecture-2026-07-31.md` (R14-D7+D8+D8-Fix, 现降级为双洋葱子文档)

### §12.2 本文产出的文档

- **本文**: `Apeireth-rust/docs/architecture-v3-aircraft-carrier.md` (立体架构终版 v2)
- **crates 标注**: `Apeireth-rust/crates/README.md` (重写 — 标注 9 crate 在立体架构中的位置)
- **CONTEXT-HANDOVER**: `Apeireth-rust/docs/CONTEXT-HANDOVER.md` (终极版, 覆盖 2026-07-30 末班旧版)
- **完成报告**: `reports/R14-architecture-final-record-report.md` (本任务产出)

### §12.3 主手册

- `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` (6546 行 + 附录 M/N, LOCKED)

---

_Writing complete: 2026-07-31 (主人终极架构设计确认 + 立体架构终版 v2 落)_
_不写 Rust 代码 / 不画 Mermaid / 不重写 V0.5/V1136/9键 / 不修改 16 份 stage2 / 不修改 crates 占位实现 — 全部遵守_
_主哲学 anchor 6 个全贯穿. 任何接手者 (包括明天的我) 都能查. 不会丢失上下文._