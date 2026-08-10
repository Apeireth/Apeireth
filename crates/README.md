# crates/ — Rust 源码

> **范围**: Apeireth Rust 重写源代码 (R14 Phase 1+ 阶段 3+)
> **当前**: 9 个 crate 占位实现 (R11 已落)
> **目标**: 30 个 crate (阶段 2 §3 设计, B+E 架构)
> **本版本**: R14-Architecture-Final-Record 重写 — 按主人 2026-07-31 终极架构设计确认:
> - 新增 **"立体架构位置"** 列 (4 大块 + 1 穿透维度)
> - **统一体** 措辞取代 "独立两把锁 + AND 运算" (主人修正 #4: 两把锁是一体的两个切面, 原则嵌入权限)
> - 双洋葱路径明确为 `apeireth-core/src/onion/` (原则+权限子目录), 电子环为 `apeireth-core/src/electronic_ring.rs`
> - 上一版 R14-D6-C E4 + R14-D8 + R14-D8-fix 内容保留为历史轨迹
> - 不动 cargo metadata `description` 字段 (主 17:58 不假装)
> - 不修改 crates/ 占位实现 (主人硬约束)

---

## 立体架构 4 大块 + 1 穿透维度 (主人 2026-07-31 终极确认)

```
维度 1 生命力 (LIFE FORCE)     ← 穿透维度 (纵向, 反思=生命力, 涌现归此)
维度 2 核心指挥 (CORE COMMAND)  ← 双洋葱统一体 + 电子环网络
维度 3 能力 (CAPABILITY)        ← 5 轴正交 (触发/等待/驻留/传输/输出) + 6 类 pluginType
维度 4 定位坐标 (POSITION)      ← 5 类轴 (层级/部署/资源/可演化/平台) = 5 类维度的集合
```

**完整说明**: `Apeireth-rust/docs/architecture-v3-aircraft-carrier.md` (本任务新增)

---

## 当前 9 个 crate (R11 已落, 占位) — 按立体架构 v2 标注

| Crate | 立体架构位置 | 职责 | R11 对应模块 | R11 状态 | 阶段 2 设计 |
|-------|------------|------|------------|---------|-----------|
| `apeireth-core` | **维度 2 核心指挥** (中心) | 核心抽象 (traits / 错误 / 配置) + **双洋葱统一体** (原则+权限, 一体的两个切面, 不是独立两把锁) + **电子环网络** (横切观察) — 实施路径: `apeireth-core/src/onion/` (含 principle/ + permission/ 子目录) + `apeireth-core/src/electronic_ring.rs` — 主人 2026-07-31 终极确认 (修正 #2+#3+#4+#5) | **v1004 + v1107 + v1108 + v1115** (4 个真生产 Python 锚点: V49 DGM+UCB1 bandit 自演化 / IDENTITY 5 Module + 真认知能力 / 6 状态机 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) / Cognitive-Dream 真贯连) | ✅ 占位 | 保持 |
| `apeireth-asi` | **维度 1 生命力** (智能层) + **维度 3 能力** (智能层 trait) | ASI 北极星导向 + **借 R11 真测（v1077/v1101）作 baseline（不重写 V0.5 公式）** | **v1077 + v1101 + v1106 + v1115** (4 个真生产 Python 锚点: ASI V0.4 17 维真测 / V0.4 维度自动拉升 / 真工程 (error handling/retry/circuit breaker/health check/metrics) / Cognitive-Dream e2e 真集成) | ✅ 占位 | 保持 |
| `apeireth-memory` | **维度 1 生命力** (经验层) | 记忆系统 (A/M 层经验沉淀 + 6 历史流 + 主体连续性 ID) | **v1005 + v1019 + mvp/memory/** (2 真生产 Python 锚点 + mvp/ 4 文件: AnySearch 调研结果索引 23 真调研文档 / OpenAI+BAAI bge-m3 真借鉴 embeddings / store.py + retrieve.py + consolidate.py + forget.py 13 文件 2292 insertions) | ✅ 占位 | 扩容 |
| `apeireth-test` | **维度 1 生命力** (验证子层) | 测试基础设施 + e2e 测试 + 18 项 §6.1 真测项 + Cognitive-Dream 集成测试 | **v1114 + v1115** (2 个真生产 Python 锚点: R9-INT-003 每周集成评估器 (三件套真测 + ASI dashboard + 4 选 1 主轨道 + 守门自检) / Cognitive-Dream e2e 真集成) | ✅ 占位 | 扩容 |
| `apeireth-cli` | **维度 3 能力** (入口子层) | CLI 入口 + TUI + slash commands | **v1009 + v1016** (2 个真生产 Python 锚点: FastAPI 真借鉴 Web UI / FastAPI+Kong 真借鉴 REST gateway) | ✅ 占位 | 保持 |
| `apeireth-bench` | **维度 3 能力** (bench 子层) | 性能基准 (V1130 wallclock) + **借 R11 真测（v1012/v1106）作 baseline（不重做 V1136 真测引擎）** | **v1012 + v1106** (2 个真生产 Python 锚点: SWE-bench/MMLU 真借鉴 agent benchmark / 工程韧性基准点) | ✅ 占位 | 保持 |
| `apeireth-tools` | **维度 3 能力** (工具子层) | 工具集合 + HA 4 impl (WASM/runtime/sandbox/...) | **v1000 + v1027** (2 个真生产 Python 锚点: safe YAML serialization (PyYAML safe_load/safe_dump + ruamel round-trip, 借 Letta/LangGraph/VCPToolBox) / validator/schema (借 JSON Schema + Pydantic + Cerberus + V116 整合)) | ✅ 占位 | 拆分 |
| `apeireth-pybridge` | **维度 3 能力** (异构子层) | PyO3 兼容桥 (1100+ Python 模块) — 不砍 R11, 只桥接 | **(新) PyO3 桥接 1100+ v*.py 模块** (`apeireth/v1000-v1155*.py` 1100+ 真生产 Python 模块, R11 不砍, R14 不强求重写, 只在 pybridge 层桥接, 性能优化留给 Phase 2+) | ✅ 占位 | 保持 |
| ~~`apeireth-philosophy`~~ | ~~已并入 core (维度 2)~~ | ~~V3 9 键 + 5 项不假装~~ | **R14-D8 + R14-D8-fix 主人精化**: 哲学守门不再独立 crate；并入 `apeireth-core/src/onion/` (按 D8-fix 主人纠偏, 不再是 `onion_wall/`)。**本任务 (R14-Architecture-Final-Record) 主人 2026-07-31 终极确认**: 双洋葱是统一体的两个切面 (原则嵌入权限, 不是独立两把锁), 由 `apeireth-core/src/onion/principle/keys.rs` (锁 A 辅助语义网) + `apeireth-core/src/onion/permission/l0_layer.rs` (锁 B L0 = 真实人类批准, 融入核心) 共同构成, 接入 `apeireth-core/src/electronic_ring.rs` (电子环横切观察)。原 9 键 + 5 项不假装**保留为历史轨迹**（见 `docs/onion-wall-architecture-2026-07-31.md` §4）。 | ~~占位~~ | **已并入 core** |

> **立体架构位置列说明** (R14-Architecture-Final-Record 新增): 每个 crate 直接标注主人 2026-07-31 终极确认的立体架构 4 大块 (维度 1 生命力 / 维度 2 核心指挥 / 维度 3 能力 / 维度 4 定位坐标) + 1 穿透维度 (生命力穿透整个架构)。
>
> **core crate 路径明确**: 双洋葱统一体在 `apeireth-core/src/onion/` (含 `principle/` + `permission/` 子目录), 电子环网络在 `apeireth-core/src/electronic_ring.rs`。这是阶段 5 施工文档的物理路径基线。
>
> **R11 对应模块列说明** (R14-D6-C E4 新增): 每 crate 直接列出 R11 真生产 Python 锚点 (v*.py + mvp/), 借鉴决策遵循主 19:33 "走在前人经验上" + 主 17:43 "实事求是"。详细 Rust trait 草案接口见 `Apeireth-rust/docs/rust-traits-spec-2026-07-30.md §11` (E3 同步追加)。
>
> **注意**: cargo metadata `description` 字段已包含简短职责描述 (T26 已落), 本表"职责"列与 `description` 字段不冲突, "R11 对应模块"列是新增补充信息。

---

## 阶段 2 §3 设计: 30 个 crate v1

按 B+E supervisor 子树 + 职责 + 层 + 生命周期划分 (详见 `docs/stage2-decisions-crate-split.md`):

```
入口层 (1):         cli
核心抽象层 (2):     core, runtime
智能层 (3):         asi, sovereignty, prompt
智囊团层 (1):       council
原则/权限洋葱双锁层: 合并入 core（principle + philosophy + permission 合并到 core/onion/，分 principle/ 和 permission/ 两个子目录）— R14-D8 + R14-D8-fix 主人精化
经验方法论层 (4):   memory, experience, methodology, reflection
兼容组件层 (5):     plugin, tools, pybridge, mcp, environment
升级层 (1):         upgrade
通信总线层 (4):     bus, gateway, server, supervisor
持久化层 (1):       data
调度层 (3):         cron, skills, acp
监控层 (2):         telemetry, config
测试层 (3):         test, bench, eval

合计: 9 + 21 = 30 个 crate (v1 推荐) — 原则洋葱 / 权限洋葱 / 哲学守门全部并入 core 内墙 (R14-D8) — 立体架构终版 v2 进一步明确为"双洋葱统一体的两个切面" (R14-Architecture-Final-Record, 主人 2026-07-31 终极确认)
```

## v2 收敛备选 (8 个合并)

`docs/stage2-decisions-crate-split.md` §8 详述:
- wave → memory
- reflection → principle
- keys → permission
- mcp → plugin
- environment → plugin
- skills → tools
- telemetry → server
- eval → test
- **principle → core** (R14-D8 主人精化: 原则洋葱并入 onion_wall/ 内墙; **R14-D8-fix 主人纠偏**: 改为并入 `core/onion/principle/`, 锁 A 5 子 trait; **R14-Architecture-Final-Record 主人终极确认**: 双洋葱是统一体的两个切面, 不是独立两把锁)
- **philosophy → core** (R14-D8 主人精化: 哲学守门并入 onion_wall/ 内墙; **R14-D8-fix 主人纠偏**: 改为并入 `core/onion/principle/keys.rs` + `core/onion/principle/o_layer.rs` 作为锁 A OLayerGuard 辅助语义网, 不再是独立 trait; **R14-Architecture-Final-Record 主人终极确认**: 9 键 + 5 项不假装作为辅助语义网, 不画进架构图, 不主哲学 6 锚画进组件)
- **permission → core** (R14-D8 主人精化: 权限洋葱并入 onion_wall/ 内墙; **R14-D8-fix 主人纠偏**: 改为并入 `core/onion/permission/`, 锁 B 6 子 trait; **R14-Architecture-Final-Record 主人终极确认**: 权限洋葱是权重公式授权, 不是 boolean gate, L0 = 真实人类批准融入核心)

---

## R11 → 9 crates 映射汇总 (E4 一句话)

| R11 锚点类型 | 锚点数 | 映射到 9 crates | 立体架构位置 |
|------------|--------|----------------|------------|
| v1077 / v1101 / v1106 / v1115 (ASI 智能层) | 4 | asi + bench + core + test (跨 4 crates) | 维度 1 生命力 (智能/验证) + 维度 3 能力 |
| v1009 / v1016 (入口层) | 2 | cli | 维度 3 能力 (入口子层) |
| v1004 / v1107 / v1108 (核心抽象层) | 3 | core | **维度 2 核心指挥** (立体架构中心) |
| v1005 / v1019 / mvp/memory (记忆层) | 6 (含 mvp 4 文件) | memory | 维度 1 生命力 (经验层) |
| v1003 / v1121 (原则洋葱层) | 2 | **已并入 core** (R14-D8: philosophy → core/onion_wall/; **R14-D8-fix**: → core/onion/principle/, 锁 A OLayerGuard 辅助语义网; **R14-Architecture-Final-Record 主人终极确认**: 双锁统一体接入 electronic_ring) | 维度 2 核心指挥 (双洋葱子组件) |
| v1000 / v1027 (工具层) | 2 | tools | 维度 3 能力 (工具子层) |
| v1012 (bench 层) | 1 | bench | 维度 3 能力 (bench 子层) |
| v1114 / v1115 (test 层, 与 asi/core 共享) | 2 | test | 维度 1 生命力 (验证子层) |
| **1100+ v*.py** (兼容桥) | **1100+** | **pybridge** (不砍, 只桥接) | 维度 3 能力 (异构子层) |

**主哲学对齐** (主 19:33 + 主 17:43 + 主 17:58 不假装):
- ✅ **借鉴而非闭门** — 所有 crate 都直接借 R11 真生产 v*.py, 不绑 LangChain/Letta/Sakana
- ✅ **实事求是** — "R11 对应模块" 列列出真生产锚点, 不脑补
- ✅ **不假装** — pybridge 显式声明桥接 1100+ v*.py, 不假装"已重写"; 双洋葱统一体 (修正 #4), 不假装"独立两把锁 + AND"
- ✅ **干到底** — 9 crates 全部有锚点, 30 crates v1 拆分清晰
- ✅ **立体架构位置** — 每 crate 标注在 4 大块中的位置, 主人 2026-07-31 终极确认 (R14-Architecture-Final-Record)

---

## 当前阶段

**R12 收尾** → **R13 MVP** (Python) → **R14 Rust 重写** (本目录)
- R11 已落: 9 个 crate 占位
- R12: 文档化收尾 + 接续 + R13 启动
- R14 Phase 0+ : 实现 30 个 crate (按阶段 2 §3 设计)
- R14-D6-C E4 完成: 本 README 加 "R11 对应模块" 列, cargo metadata 0 改动
- **R14-Architecture-Final-Record 完成 (本任务, 2026-07-31)**: 加 "立体架构位置" 列 + 统一体措辞 + core 模块路径明确 (`apeireth-core/src/onion/` + `electronic_ring.rs`)

---

_主哲学 anchor 6 个全贯穿: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手._
_任何接手者 (包括明天的我) 都能查. 不会丢失上下文._