# 阶段 2 — 想法设计（stage2-decisions）

> **当前采用** 🟢 = 19 份 stage2-decisions-*.md（其中 18 stage2 + 1 D2 增补）
> **本文档状态**: 🟢 **当前活跃**
> **写作时间**: 2026-07-30 → 2026-07-31（D2 增补）

## 📋 19 个文档

### 18 份 stage2-decisions 决策

| # | 文档 | 主题 |
|---|---|---|
| 1 | stage2-decisions-tech-stack.md | 技术栈（Rust 1.80 / tokio / sled / SQLite / PyO3 / criterion）|
| 2 | stage2-decisions-architecture.md | 核心架构形态（B+E supervisor / 5 层通信总线 / 主路径核心）|
| 3 | stage2-decisions-crate-split.md | crate 划分（30 crate v1 目标 + 当前 9 占位）|
| 4 | stage2-decisions-process-threading.md | 进程 / 线程 / 协程（B+E supervisor 拓扑）|
| 5 | stage2-decisions-memory-layout.md | 内存布局（A+B+C+D / 零拷贝 / 引用规则）|
| 6 | stage2-decisions-persistence.md | 持久化（6 DB 协同：sled / SQLite / RocksDB）|
| 7 | stage2-decisions-llm-integration.md | LLM 集成（多 LLM 路由 + 工具调用）|
| 8 | stage2-decisions-modularity.md | 模块化（plugin 体系 + 异构实现）|
| 9 | stage2-decisions-communication-bus.md | 通信总线（5 层：inproc / UnixSocket / pipe / gRPC / WS）|
| 10 | stage2-decisions-council-impl.md | 智囊团实现（7 强制 + 动态专家 + 按住机制）|
| 11 | stage2-decisions-upgrade-impl.md | 升级 OTA 实现（7 阶段 + 沙盒 + 五重治理）|
| 12 | stage2-decisions-philosophy-guard.md | 哲学守门 5 重实现 |
| 13 | stage2-decisions-decision-system.md | 决策系统（决策系统 §4 + 物理多签 + 按住）|
| 14 | stage2-decisions-permission-packs.md | 权限包（§5.3）|
| 15 | stage2-decisions-drift-revision-tracker.md | P0 漂移降级跟踪 |
| 16 | stage2-decisions-source-projects-list.md | 借鉴源项目清单（30 项目 + 借鉴决策）|
| 17 | stage2-decisions-upgrade-impl.md | (见 #11)|
| 18 | stage2-decisions-appendix-references.md | 附录引用 |
| 19 | **stage2-decisions-addendum-sovereignty-continuity-governance.md** | **D2 增补**：主权 / 连续性 / 治理 + 三域 / SGI / 双根 / HA / 部署兼容 / 风险分级 |

## 🎯 阶段 2 在 R14 中的位置

**R14 6 阶段顺序**：
1. ✅ 灵感（阶段 1）
2. ✅ **想法设计**（阶段 2）
3. ✅ 画图纸（阶段 3）
4. ✅ 落实架构（阶段 4）
5. ✅ 设计施工文档（阶段 5）
6. ⏳ 设计里程碑验证机制（阶段 6 — **下一步**）

**阶段 2 的核心价值**（12 决策 + D2 增补）：
- **技术栈** = Rust 1.80 / tokio / sled / SQLite / PyO3
- **架构形态** = B+E supervisor + 5 层通信总线
- **crate 划分** = 30 crate v1 目标（阶段 4 修正为 17 crate 本源推导）
- **内存布局** = 零拷贝 + 引用规则（A+B+C+D）
- **持久化** = 6 DB 协同
- **LLM 集成** = 多 LLM 路由 + 工具调用
- **通信总线** = 5 层（inproc / UnixSocket / pipe / gRPC / WS）
- **智囊团** = 7 强制 + 动态专家 + 按住
- **OTA** = 7 阶段 + 沙盒 + 五重治理
- **哲学守门** = 5 重（编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期）
- **D2 增补** = 主权 / 连续性 / 治理 + 三域 / SGI / 双根 / HA / 部署兼容 / 风险分级

## 🔗 与后续阶段衔接

| 后续阶段 | 引用 §阶段 2 的内容 |
|---|---|
| **v4 哲学层纲领** | §18.7 双洋葱正交 → D2 §7 原则×权限正交（v4 改为统一体嵌入）|
| **v4.1 哲学层升级** | §12 哲学守门 5 重 → v4.1 §15 提议 V3 v2 12 键 |
| **阶段 3 v2** | §2 B+E supervisor + §9 5 层通信总线 → v2 立体架构（航空母舰）|
| **阶段 4 落实** | §3 30 crate v1 目标 → 阶段 4 推导 18 → 17 crate；§12 哲学守门 → 5 重守门编译时 hardcode；§11 OTA → 阶段 4 §8 OTA 数据流 |
| **阶段 5 施工** | §3 crate 划分 → 阶段 5 §2 17 crate 重写方案；§11 OTA → 阶段 5 §8 OTA 7 阶段工程化 |
| **阶段 6 验证** | §12 哲学守门 5 重 → R-Measure 验证网 |

## ⚠️ 关键诚实（主 17:43 实事求是）

**阶段 2 状态**：
- 🔒 **LOCKED**（主人明确沉淀，**不修改**）
- 所有后续阶段**只引用**，**不重写**
- 12 决策 + 1 D2 增补 = 19 份文档 = R14 6 阶段的**工程化沉淀**

**为什么不重写**：
- 主人在恢复上下文明确"阶段 1+2+3 确认沉淀下来的东西其他的从旧语言写的项目继承过来的东西都能修改"
- 阶段 2 是**主人在 2026-07-30 亲自精化 18 份决策**的工程化，**已经定稿**

## 📂 与其他子目录的关系

| 子目录 | 关系 |
|---|---|
| `../stage1/` | 阶段 2 = 阶段 1 灵感的"工程化"（18 份决策）|
| `../stage3-blueprints/` | 阶段 3 = 阶段 1+2 的"画图纸"（v2 立体架构）|
| `../stage4/` | 阶段 4 = 阶段 1+2+3 的"落实"（1492 行主文档 + 4 份补丁 + 8 子文档）|
| `../stage5/` | 阶段 5 = "施工蓝图"（基于阶段 4 + 外部反馈，**18 → 17 crate**）|
| `../r14-design/` | 阶段 1+2+3 审查报告 + R14 周期产物 |

## 🎯 精读顺序（接手者）

### ⚡ 5 分钟 — "阶段 2 是什么"
1. §2 architecture 决策（理解 B+E supervisor + 5 层总线）
2. §9 communication-bus 决策（5 层通信）

### 🕐 30 分钟 — "决策核心"
1. §3 crate-split 决策（30 crate v1 目标）
2. §6 persistence 决策（6 DB 协同）
3. §11 upgrade-impl 决策（OTA 7 阶段）
4. §12 philosophy-guard 决策（5 重守门）
5. **D2 增补**（主权 / 连续性 / 治理 + 三域 / SGI / 双根 / HA）

### 🕑 1 小时 — "决策全貌"
1. 全部 12 决策（按决策依赖图顺序）

### 🕓 4 小时 — "完整精读"
1. 全部 19 文档（含 D2 增补）

## 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — §2 architecture 决策服务 ASI 北极星
S-2 主 17:43 实事求是   — §6 persistence 6 DB 协同基于 R11 现状
O-5 主 17:58 不假装     — §12 philosophy-guard 5 重守门 + D2 §9 HA 硬门槛
O-2 主 19:33 走在前人经验上 — §16 source-projects-list 借鉴 30 项目
O-3 主 23:44 干到底    — §11 OTA 7 阶段立即落
O-4 主 00:56 任何人都能接手 — 19 文档 + D2 增补可追溯
```

---

_阶段 2 README v2 修订版（leader 亲自产出）._
_🟢 当前活跃 = 19 份 stage2-decisions-*.md（含 D2 增补）._
_§与后续阶段衔接表格明确阶段 2 的 6 处关键引用._
_§关键诚实明确 LOCKED 不修改 + 原因._
_主哲学 6 锚穿透. 任何接手者能查._