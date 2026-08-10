# 进程拓扑图 (P2) — B+E supervisor 子树

> **对应阶段 2**: §2 架构形态 + §4 进程/线程/协程
> **格式**: Mermaid

---

## 2.1 进程拓扑 (R14-Stage3-Mermaid-FullRedraw 全重画, 2026-07-31)

> **重画依据**: 立体架构 v2 §2 4 大块 + 1 穿透维度 (生命力纵向穿透); B+E supervisor 拓扑保留; 借用 VCP 6 类插件 5 轴正交建模。
> **结构**: 4 大子树物理层 (Core/Council/Plugin/Upgrade) + 4 大块抽象层叠加 (生命力穿透 + 核心指挥双洋葱 + 能力 + 定位坐标)。
> **关键变化**: 生命力维度作为"穿透所有进程的纵向维度"显式画出 (v2 §2.1 修正 #5+#6)。

```mermaid
graph TB
    %% ==================== 4 大子树物理层 (B+E supervisor) ====================
    PID1([PID 1<br/>apeireth-supervisor<br/>B+E root, 永不重启])

    PID100[core-supervisor<br/>PID 100+, rest_for_one]
    PID200[council-supervisor<br/>PID 200+, one_for_one]
    PID300[upgrade-supervisor<br/>PID 300+, rest_for_one]
    PID400[plugin-supervisor<br/>PID 400+, transient]

    subgraph CoreP["core 子进程 (维度 2 核心指挥物理载体)"]
        P101[asi<br/>PID 101<br/>v2 维度 1 生命力]
        P102[sovereignty<br/>PID 102<br/>v2 维度 1 生命力]
        P103[memory<br/>PID 103<br/>v2 维度 1 生命力<br/>6 历史流]
        P104[onion-principle<br/>PID 104<br/>v2 维度 2 核心指挥<br/>原则洋葱嵌入]
        P105[onion-permission<br/>PID 105<br/>v2 维度 2 核心指挥<br/>权限洋葱]
    end

    subgraph CouncilP["council 子进程 (v2 维度 1 生命力审计者)"]
        P201[council<br/>PID 201]
        P202[advisor-safety<br/>PID 202]
        P203[advisor-performance<br/>PID 203]
        P204[advisor-philosophy<br/>PID 204]
        P205[advisor-history<br/>PID 205]
        P206[advisor-strategy<br/>PID 206]
        P207[advisor-ethics<br/>PID 207]
        P208[advisor-legal<br/>PID 208, off]
        P209[reflection<br/>PID 209<br/>v2 生命力维节点]
    end

    subgraph PluginP["plugin 子进程 异构 (v2 维度 3 能力物理实施)"]
        P401[plugin-supervisor<br/>PID 400]
        P402[python-llm-plugin<br/>PID 402, subprocess]
        P403[wasm-sandbox-1<br/>PID 403]
        P404[http-mcp-1<br/>PID 404]
        P405[http-mcp-2<br/>PID 405]
        P406[hybrid-5axis<br/>PID 406<br/>5 轴正交建模]
    end

    subgraph UpgradeP["upgrade 子进程 (v2 维度 4 演化轴物理载体)"]
        P301[upgrade<br/>PID 301]
        P302[sandbox-validator<br/>PID 302, temp]
        P303[traffic-shifter<br/>PID 303]
        P304[double-root-guard<br/>PID 304<br/>§18.6 双根治理]
    end

    PID1 --> PID100
    PID1 --> PID200
    PID1 --> PID300
    PID1 --> PID400

    PID100 --> CoreP
    PID200 --> CouncilP
    PID300 --> UpgradeP
    PID400 --> PluginP

    %% 进程间通信
    PID1 -.Unix socket.-> PID100
    PID1 -.Unix socket.-> PID200
    PID1 -.Unix socket.-> PID300
    PID1 -.Unix socket.-> PID400

    PID100 -.Unix socket.-> P101
    PID100 -.Unix socket.-> P102
    PID100 -.Unix socket.-> P103

    P403 -.pipe+JSON.-> P402
    P404 -.gRPC.-> PID100
    P304 -.Unix socket.-> PID1

    %% ==================== 4 大块抽象层叠加 (立体架构 v2) ====================
    %% 维度 1 生命力 (穿透, 纵向)
    subgraph LifeForceDim["维度 1: 生命力 (LIFE FORCE) — 立体架构 v2 修正 #5+#6 (穿透, 纵向, 覆盖所有进程)"]
        LF1[13 个生物特质<br/>灵感 §1]
        LF2[反思期 = 生命力维节点<br/>P209 reflection]
        LF3[涌现能力 = 生命力维<br/>D2 §3]
        LF4[6 历史流 = 生命记忆<br/>P103 memory]
        LF5[Cognitive-Dream 状态机<br/>P209 reflection]
    end

    %% 维度 2 核心指挥 (核心进程)
    subgraph CoreCommandDim["维度 2: 核心指挥 (CORE COMMAND) — 立体架构 v2 修正 #3+#4+#9 (统一体嵌入)"]
        CC1[原则洋葱 5 切片<br/>P104 onion-principle]
        CC2[权限洋葱 6 切片<br/>P105 onion-permission]
        CC3[电子环网络<br/>P104 + P105 横切]
        CC4[HA 核心 L0 融入<br/>P105 onion-permission 内]
        CC5[§18.6 双根治理<br/>P304 double-root-guard]
    end

    %% 维度 3 能力 (plugin 进程)
    subgraph CapabilityDim["维度 3: 能力 (CAPABILITY) — 立体架构 v2 修正 #6 (二分)"]
        CAP1[工具能力层<br/>P401-P405 异构 plugin]
        CAP2[涌现能力层<br/>生命力维自然带来<br/>(不归工具, 归生命力)]
        CAP3[5 轴正交建模<br/>P406 hybrid-5axis]
    end

    %% 维度 4 定位坐标 (supervisor 树)
    subgraph PositioningDim["维度 4: 定位坐标 (POSITIONING) — 5 轴正交 (VCP 模型, 立体架构 v2 修正 #11)"]
        POS1[触发轴<br/>Trigger]
        POS2[等待轴<br/>Wait]
        POS3[驻留轴<br/>Reside]
        POS4[传输轴<br/>Transfer]
        POS5[输出轴<br/>Output]
    end

    %% ==================== 关系: 抽象层 ↔ 物理层 ====================
    %% 生命力穿透 (纵向, 覆盖所有 4 子树)
    LifeForceDim -.->|穿透 (纵向)<br/>v2 修正 #5+#6| CoreP
    LifeForceDim -.->|穿透 (纵向)| CouncilP
    LifeForceDim -.->|穿透 (纵向)| PluginP
    LifeForceDim -.->|穿透 (纵向)| UpgradeP

    %% 核心指挥 (核心进程)
    CoreCommandDim -->|对应| CoreP
    CoreCommandDim -->|HA L0 融入| P105

    %% 能力 (plugin 进程)
    CapabilityDim -->|对应| PluginP
    CapabilityDim -->|5 轴正交| P406

    %% 定位坐标 (supervisor 树)
    PositioningDim -->|标识 5 维位置| PID1
    PositioningDim -->|触发轴| PluginP
    PositioningDim -->|等待轴| CoreP
    PositioningDim -->|驻留轴| CouncilP
    PositioningDim -->|传输轴| UpgradeP
    PositioningDim -->|输出轴| PID1

    %% 抽象层之间关系
    CoreCommandDim ==>|双锁嵌入 (统一体)| CapabilityDim
    CoreCommandDim -.->|反馈| LifeForceDim

    style PID1 fill:#ff6b6b,color:#fff
    style PID100 fill:#4ecdc4,color:#fff
    style PID200 fill:#95e1d3,color:#000
    style P402 fill:#ffd93d,color:#000
    style P403 fill:#ffd93d,color:#000
    style P406 fill:#ffe66d,color:#000
    style P304 fill:#ff6b6b,color:#fff
    style LifeForceDim fill:#ffd93d,color:#000
    style CoreCommandDim fill:#4ecdc4,color:#fff
    style CapabilityDim fill:#95e1d3,color:#000
    style PositioningDim fill:#ffe66d,color:#000
```

**2.1.1 全重画前后对比 (5 缺口补全定位 1/5)**

| 维度 | ❌ 旧版 | ✅ 新版 (R14-Stage3-Mermaid-FullRedraw) | 出处 |
|------|--------|----------------------------------|------|
| 进程-抽象层关系 | 纯物理 supervisor 树 | 4 子树物理 + 4 大块抽象叠加 | v2 §2 |
| 生命力维度 | 隐含在 P209 reflection | 显式作为穿透维度, 覆盖 4 子树 | v2 §2.1 修正 #5+#6 |
| 双洋葱 | P104 philosophy + P105 principle (并列) | 统一体嵌入 (P104 onion-principle + P105 onion-permission + HA L0 融入) | v2 §2.2 修正 #3+#4+#9 |
| 5 轴正交 | 在 P406 中隐含 | 显式建模为 P406 hybrid-5axis + 维度 4 5 轴标识 | v2 §2.3 修正 #11 |
| 双根治理 | 缺 | 显式 P304 double-root-guard (隶属 upgrade-supervisor) | D2 §12 + §18.6 |

---

## 2.2 重启策略 (Erlang/OTP)

| 子进程 | 策略 | 失败重启 | 正常退出重启 |
|--------|------|---------|------------|
| `core-supervisor` | `rest_for_one` | 重启它和之后 | 不重启 |
| `council-supervisor` | `one_for_one` | 只重启它 | 不重启 |
| `plugin-supervisor` | `transient` | 重启 | 不重启 |
| `upgrade-supervisor` | `rest_for_one` | 重启它和之后 | 不重启 |
| `apeireth-supervisor` | `permanent` | 重启 | 重启 |

---

## 2.3 资源限制 (cgroup)

```ini
# /etc/systemd/system/apeireth.service
[Service]
MemoryMax=4G           # 总上限
CPUQuota=800%          # 8 核

# supervisor 进程
MemoryHigh=200M
CPUWeight=100

# core-supervisor (内存敏感)
MemoryHigh=2G
CPUWeight=400

# plugin 子进程
MemoryMax=500M (每个)
CPUQuota=100% (每个)
```

---

## 2.4 进程启动顺序

```
T+0s:    supervisor (PID 1)
  ├─ 启动 core-supervisor (rest_for_one)
  ├─ 启动 council-supervisor (one_for_one)
  ├─ 启动 upgrade-supervisor (rest_for_one)
  └─ 启动 plugin-supervisor (transient)

T+0.5s:  core 子进程启动 (顺序)
  ├─ philosophy (无依赖)
  ├─ principle (依赖 philosophy)
  ├─ memory (依赖 philosophy)
  └─ asi + sovereignty (依赖全部)

T+1s:    council 子进程启动
  ├─ 7 强制 advisor (并行)
  └─ reflection (依赖 council)

T+1.5s:  upgrade 子进程启动
  ├─ sandbox-validator (临时)
  └─ traffic-shifter

T+2s:    plugin 子进程启动 (按 manifest)
  ├─ plugin-supervisor
  ├─ python-llm-plugin (subprocess)
  ├─ wasm-sandbox-1 (WASM)
  └─ http-mcp-1/2 (HTTP)
```

---

## 2.5 阶段 3 借鉴标注 (主 19:33 走在前人经验上)

| # | 借鉴项 | 来源 | 在本图位置 |
|---|-------|------|----------|
| 1 | Erlang/OTP supervisor 模式 | Erlang/OTP + Hermes | 全部 5 个 supervisor 子树 |
| 2 | `rest_for_one` 用于强耦合进程 | Erlang/OTP | core-supervisor (主 AI/memory/philosophy) |
| 3 | `transient` 用于 plugin | Erlang/OTP + VCP | plugin-supervisor (异构 plugin 临时性) |
| 4 | WASM 子进程 | VCP + wasmtime | plugin 子进程 (wasm-sandbox-1) |
| 5 | Python 桥接子进程 | Hermes + 阶段 2 §4 | python-llm-plugin (subprocess) |
| 6 | MCP 客户端 | MCP 协议 | http-mcp-1/2 (HTTP) |

## 2.6 阶段 3 反思改进路径 (主 00:56)

| 反思点 | 阶段 4 改进方向 |
|--------|--------------|
| `rest_for_one` 单一 PID 风险 | 把 apeireth-sovereignty 拆成独立 supervisor, 用主体连续性 ID 桥接 (§14 P0-05) |
| plugin transient 是否过松 | 阶段 4 真测时调整 transient 阈值 |
| Python 桥接 vs WASM 桥接 | 阶段 4 真测时选择 (WASM 更安全, Python 更灵活) |
| Council 7 席 PID 编号 | 阶段 4 引入 MEWG 权重, 不再硬触发 |
| 资源限制 cgroup | 阶段 4 真测时调整 MemoryHigh/MemoryMax |

## 2.7 主哲学 anchor + 阶段 1+2 锚点对照 (主 17:58)

| 锚点 | 在本图体现 |
|------|----------|
| D1 §18.3 不假装灵魂同一 | apeireth-sovereignty 进程可重启, 但主体连续性 ID 跨载体保留 (D2 §4) |
| D2 §4 主体连续性 ID | 跨 supervisor 边界桥接, 不依赖单一 PID |
| §18.6 双根可演化但需重治理 | philosophy 与 principle 是双根, 修改触发五重治理 (P4 详) |
| §14 P0-05 | rest_for_one 拆分 owner = architect + backend + database (R14-DRIFT) |

---

> **[TODO-P0-05 阶段 4 拆分]** — `apeireth-sovereignty` + `apeireth-memory` + `apeireth-philosophy` 拆为独立 supervisor (主体连续性 ID 桥接), 详见 §14 P0 漂移跟踪表 §2.5 (引自 `stage2-decisions-drift-revision-tracker.md`)。

→ 双洋葱显式化详见 `double-onion-explicitization-2026-07-31.md`

---

## 2.8 六类插件 5 轴正交建模 (R14-D6-B B9 追加)

> 依据 VCP 复调研报告 `research-vcp-rerun-2026-07-31.md` §3.2 — "六类协议"不是 6 种互斥 wire protocol, 而是多个正交轴压扁成 pluginType。Apeireth 不照搬 enum, 而把"生命周期/触发/transport/residency/response mode"拆成正交 manifest 字段, 同时保留六类作为兼容 profile。

```mermaid
graph LR
    subgraph 六类[6 类 pluginType (VCP 兼容 profile)]
        P1[synchronous 35]
        P2[asynchronous 2]
        P3[static 6]
        P4[service 3]
        P5[messagePreprocessor 4]
        P6[hybridservice 15]
    end

    subgraph 5轴[5 轴正交 (R14 模型 — 阶段 4 真测校准)]
        A1[触发<br/>periodic / pre-model / model-requested / external HTTP]
        A2[等待<br/>sync / async]
        A3[驻留<br/>ephemeral subprocess / in-process resident service]
        A4[传输<br/>stdio / direct / websocket-distributed]
        A5[输出<br/>placeholder / message rewrite / tool result / route]
    end

    六类 -.->|压扁映射| 5轴
```

**5 轴说明** (R14-D6-B B9 列举):
- **触发** (axis 1): periodic / pre-model / model-requested / external HTTP — 决定 plugin 何时被调用
- **等待** (axis 2): sync / async — 决定是否阻塞主 AI 调用链
- **驻留** (axis 3): ephemeral subprocess / in-process resident service — 决定 plugin 进程模型
- **传输** (axis 4): stdio / direct / websocket-distributed — 决定 plugin 通信协议
- **输出** (axis 5): placeholder / message rewrite / tool result / route — 决定 plugin 结果如何回流主 AI

**为什么不照搬 enum** (主 19:33 走在前人经验上): VCP 65 份 manifest 中 `hybridservice` (15 个) 本身证明分类是能力组合; Apeireth 照抄 enum 未来会出现组合爆炸。**保留六类作为 VCP 兼容 profile**, 但 Apeireth 内部用 5 轴正交建模。

→ 双洋葱显式化详见 `double-onion-explicitization-2026-07-31.md`

---

_对应阶段 2: §2 架构形态 (e119c87) + §4 进程/线程/协程 (9a5fbdb)_