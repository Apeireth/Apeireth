# P1 整体架构图 — 说明文档 (主 00:56 任何人都能接手)

> **对应图**: `01-overall-architecture.md`
> **本文件性质**: P1 图的"心智模型", 解释**为什么是这样 / 借鉴了什么 / 反思改进路径**。
> **不写 Rust 代码 / 不冻结架构 / 不重写 P1 既有 Mermaid**。

---

## §1. 为什么是这样

P1 是阶段 3 的"主干图", 它要把 5 件事讲清楚:

1. **谁在跑** (入口: CLI/MCP/PyBridge) — D1 §18.5 平台三件套的"提供"
2. **谁管谁** (B+E supervisor 拓扑) — 阶段 2 §2 架构形态 + §4 进程
3. **谁管什么** (Core/Council/Plugin/Upgrade 4 大子树) — D1 §18 + D2 §7 双洋葱
4. **谁记得** (6 DB 协同) — 阶段 2 §6 持久化
5. **谁批准** (权限洋葱 + 物理多签) — §18.6 双根可演化但需重治理

P1 的设计原则:

- **航空母舰级复杂度**: 允许繁重/复杂/冗余/过度设计 (阶段 1 §1 比喻)
- **B+E 双根 supervisor**: 借鉴 Erlang/OTP + Hermes, 阶段 2 §4 已落
- **双洋葱正交**: 内层不替外层做关系选择, 外层不替内层做治理决定 (D2 §7 + PREREQ-2 已桥接)
- **6 组件显式化**: PREREQ-2 §4 已固定 OuterExperienceShell / InnerInfrastructureCore / PrincipleOnionSlice / PermissionOnionSlice / DoubleRootBaton / CrossLayerGuard

## §2. 借鉴了什么

| # | 借鉴项 | 来源项目 | 借鉴强度 | 在图中的位置 |
|---|-------|---------|---------|----------|
| 1 | **6 类插件协议 (混合型 hybrid)** | VCP ToolBox | ★★★★★ | Plugin-supervisor 子树 + hybrid 类型标注 |
| 2 | **ContextBridge 共享服务** (fold/rag/vector store) | VCP ToolBox | ★★★★★ | InnerInfrastructureCore (PREREQ-2 §4) |
| 3 | **17 platform trait 抽象** | Hermes-Agent | ★★★★★ | core/council/plugin/upgrade 子树对应 trait |
| 4 | **tree-sitter + Hybrid LSP + 知识图谱** | codebase-memory-mcp | ★★★★ | Data 子树"Wave 联想网络"内嵌 cbm 引用 |
| 5 | **3 层渐进式披露** (current/timeline/archival) | claude-mem | ★★★★ | 6 DB 协同按温度分层 |
| 6 | **WASM 沙箱用于 plugin** | VCP + wasmtime | ★★★ | Plugin-subervisor 异构子进程 (HTTP/WASM/subprocess) |
| 7 | **分布式节点** (VCP 星型网络) | VCP | ★★★ | L4 WebSocket 出口 (借鉴但偏离: 我们不引入星型拓扑, 仅借鉴"跨节点透明") |

详细打分见 `borrowed-from-projects.md` §3.1。

## §3. 反思改进路径

| 反思点 | 当前状态 | 阶段 4 改进方向 |
|--------|---------|--------------|
| **数据层 6 DB 是否过重** | D2 §6 已落 6 DB | 阶段 4 真测时砍到 4 DB (SQLite/Sled/Qdrant/Tantivy), 砍 Wave 重复 |
| **Council 7 席硬触发** | D2 §12 风险分级已校正 | 阶段 4 落实时引入 MEWG 权重, 不再硬触发 |
| **Supervisor `rest_for_one` 风险** | §14 P0-05 已标待拆分 | 阶段 4 把 `apeireth-sovereignty` + `apeireth-memory` 解耦为独立 supervisor |
| **Plugin-supervisor 跨节点** | VCP 已支持 | 阶段 4 评估是否阶段 5+ 引入 |
| **ASI 北极星 = 0.98 LOCKED** | 阶段 1 §2 已定 | 阶段 4 落实时不修改, 只校准子测度 |

## §4. 与阶段 1+2 的锚点对照

| 锚点 | 在 P1 中的体现 |
|------|--------------|
| D1 §18.1 平台不定义关系 | 外层(OuterExperienceShell)与内层(InnerInfrastructureCore) 正交接口 |
| D1 §18.2 思想自由/行动受权 | 内部进程(ASI/SOV/MEM/PHI)只约束行动, 不读思想 |
| D1 §18.3 不假装灵魂同一 | 主体连续性 ID (D2 §4) 桥接, 但不强证灵魂同一 |
| D1 §18.4 关系开放 | 权限系统与关系系统解耦, 用户自定义 |
| D1 §18.5 平台三件套 | 提供(CLI/工具/能力) + 约束(权限/9 键) + 记录(6 历史流) |
| D2 §7 双洋葱正交 | PREREQ-2 §4 6 组件显式化 |
| D2 §11 单/多部署 | 同一 L5 代码在两种模式下切换 (部署时动态) |
| §18.6 双根可演化但需重治理 | 哲学根 E + 权限根 L5, 任何修改触发五重治理 |
| §18.12 + D2 §15.2 优先解释权 | 漂移降级流程 |

---

_本说明文档 4 节, 对应 P1 图; 5 项设计原则 + 7 项借鉴 + 5 项反思 + 9 项锚点对照._

## §6. 给读者的速查路径

1. **5 分钟**: 看 P1 图的 Mermaid 主体 (142 行), 抓住 4 大子树 + 6 DB + 5 总线
2. **15 分钟**: 看本文件 §1 + §2, 知道为什么 + 借鉴了什么
3. **30 分钟**: 看 P2/P3/P4 三张图, 抓住进程 + 决策 + 升级流
4. **1 小时**: 通读阶段 1 §18 + 阶段 2 D2 + 4 张图 + 4 张说明 + 借鉴决策总表, 形成完整心智模型