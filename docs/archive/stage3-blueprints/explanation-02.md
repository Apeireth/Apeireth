# P2 进程拓扑图 — 说明文档

> **对应图**: `02-process-topology.md`
> **本文件性质**: P2 图的"心智模型", 解释**为什么是这样 / 借鉴了什么 / 反思改进路径**。

---

## §1. 为什么是这样

P2 是 P1 的"展开图", 把 4 大子树 (Core/Council/Plugin/Upgrade) 拆成具体进程, 回答 3 件事:

1. **谁是 PID 1** — `apeireth-supervisor` permanent 永不退出
2. **谁跟谁** — 父子关系 + 重启策略 (Erlang/OTP)
3. **谁用什么策略** — rest_for_one / one_for_one / transient

P2 的核心设计选择:

- **B+E 双根 supervisor**: 借鉴 Erlang/OTP, 把 Core 子树用 `rest_for_one`(主 AI/memory/philosophy 强耦合, P0-05 已标待拆分), 其他子树用 `one_for_one` / `transient`
- **PID 1 permanent**: 进程死则系统重启, 这是工程智慧, 不动摇 (阶段 2 §4 已落)
- **核心进程按依赖顺序启动**: philosophy → principle → memory → asi → sovereignty (T+0.5s)

## §2. 借鉴了什么

| # | 借鉴项 | 来源 | 借鉴强度 | 在 P2 中的位置 |
|---|-------|------|---------|----------|
| 1 | **Erlang/OTP supervisor 模式** | Erlang/OTP + Hermes | ★★★★★ | 全部 5 个 supervisor 子树 |
| 2 | **`rest_for_one` 用于强耦合进程** | Erlang/OTP | ★★★★★ | core-supervisor (主 AI/memory/philosophy) |
| 3 | **`transient` 用于 plugin** | Erlang/OTP + VCP | ★★★★ | plugin-supervisor (异构 plugin 临时性) |
| 4 | **WASM 子进程** | VCP + wasmtime | ★★★★ | plugin 子进程 (wasm-sandbox-1) |
| 5 | **Python 桥接子进程** | Hermes + 阶段 2 §4 | ★★★ | python-llm-plugin (subprocess) |
| 6 | **MCP 客户端** | MCP 协议 | ★★★ | http-mcp-1/2 (HTTP) |

## §3. 反思改进路径

| 反思点 | 当前状态 | 阶段 4 改进方向 |
|--------|---------|--------------|
| **`rest_for_one` 单一 PID 风险** | §14 P0-05 已标待拆分 | 阶段 4 把 `apeireth-sovereignty` 拆成独立 supervisor, 用主体连续性 ID 桥接 |
| **plugin transient 是否过松** | D2 §4 已落 transient | 阶段 4 真测时调整 transient 阈值 |
| **Python 桥接 vs WASM 桥接** | D2 §4 已落两种 | 阶段 4 真测时选择 (WASM 更安全, Python 更灵活) |
| **Council 7 席 PID 编号** | P2 写 7 个 advisor PID | 阶段 4 引入 MEWG 权重, 不再硬触发 |
| **资源限制 cgroup** | P2 §2.3 已写 systemd 配置 | 阶段 4 真测时调整 MemoryHigh/MemoryMax |

## §4. 与阶段 1+2 锚点对照

| 锚点 | 在 P2 中的体现 |
|------|--------------|
| **D1 §18.3 不假装灵魂同一** | apeireth-sovereignty 进程可重启, 但主体连续性 ID 跨载体保留 (D2 §4) |
| **D2 §4 主体连续性 ID** | 主体连续性 ID 跨 supervisor 边界桥接, 不依赖单一 PID |
| **§18.6 双根可演化但需重治理** | philosophy 与 principle 是双根, 修改触发五重治理 (P4 升级流详) |
| **§14 P0-05** | rest_for_one 拆分 owner = architect + backend + database (R14-DRIFT 已落) |

---

_本说明文档 4 节, 对应 P2 图; 6 项借鉴 + 5 项反思 + 4 项锚点对照._

## §5. 与 P1 的关系

P2 是 P1 的"展开": P1 写"哪些子树", P2 写"哪些进程 + 父子关系 + 重启策略"。
P2 不重复 P1 的总线/数据/LLM/权限图, 这些在 P1 已覆盖。
P2 必须与 P3 (决策流) + P4 (升级流) 一起看, 才能完整理解 Apeireth 的运行方式。

## §6. 给读者的速查路径

1. **5 分钟**: 看 P2 §2.1 Mermaid 主体 (74 行), 抓住 5 大 supervisor + 25 个子进程
2. **15 分钟**: 看 P2 §2.2 重启策略表 + §2.3 资源限制 + §2.4 启动顺序
3. **30 分钟**: 看本文件 §1 + §2, 知道为什么 + 借鉴了什么
4. **1 小时**: 通读 P1/P2/P3/P4 四张图 + 4 张说明 + 借鉴决策总表