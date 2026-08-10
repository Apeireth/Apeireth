# 阶段 2 决策：Crate 划分 (2026-07-30)

> **范围**: R14 Rust 重写 crate 划分决策 (阶段 2 第三项)
> **触发**: 用户最新指示 "你给推荐吧"
> **依据**: 阶段 1 §14 (候选 30 个) + 阶段 2 §2 B+E 架构 + 单一职责 + 独立编译 + supervisor 子树
> **配套文档**: `stage2-decisions-architecture.md` + `stage2-decisions-tech-stack.md` + `inspiration-stage1-2026-07-30.md`

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-crate-split.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 3/12) |
| **决策** | **30 个 crate (v1 推荐, 可收敛)** |
| **依据** | 阶段 1 §14 候选 + B+E supervisor 子树 + 原则洋葱 5 层 + 生命周期 4 阶段 |

---

## 1. 决策总览

**v1 推荐 30 个 crate**, 按"职责 + 层 + 生命周期 + supervisor 子树"四维度划分:

```
apeireth 巨型基地 (30 crate)
├── 入口层 (1)
├── 核心抽象层 (2)
├── 智能层 (3)
├── 智囊团层 (1)
├── 原则洋葱层 (1)
├── 经验方法论层 (3)
├── 权限层 (2)
├── 兼容组件层 (4)
├── 升级层 (1)
├── 通信总线层 (4)
├── 持久化层 (1)
├── 调度层 (3)
├── 监控层 (2)
└── 测试层 (2)
```

---

## 2. 完整 crate 清单 (v1, 30 个)

### 入口层 (1)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-cli` | CLI 入口 + TUI + slash commands | ✅ 已存在 | 保持 |

### 核心抽象层 (2)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-core` | 核心抽象 (traits / 错误 / 类型 / 配置加载) | ✅ 已存在 | 保持, 包含基础 trait |
| `apeireth-runtime` | 统一 runtime builder + tokio 多线程 + 异步任务调度 | ❌ 新增 | 候选 |

### 智能层 (3)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-asi` | ASI 北极星导向 + V0.5/V1136 重设计 | ✅ 已存在 | 保持 |
| `apeireth-sovereignty` | 主 AI 主权 trait 实现 (决策 + 暂停 + 物理多签) | ❌ 新增 | 候选 (灵感 §16) |
| `apeireth-prompt` | LLM provider 抽象 (OpenAI/Anthropic/本地/Ollama) | ❌ 新增 | 候选 |

### 智囊团层 (1)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-council` | 智囊团 (强制 7 + 动态 N + 3 种生命周期) | ❌ 新增 | 候选 (灵感 §16) |

### 原则洋葱层 (1)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-principle` | 原则洋葱 5 层管理 (E/S/A/M/O 各自 trait) | ❌ 新增 | 候选 (灵感 §16) |
| `apeireth-philosophy` | V3 9 键 + 5 项不假装 trait 框架 | ✅ 已存在 | 保持 (作为 principle 子模块) |

### 经验方法论层 (3)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-memory` | 记忆系统 (A/M 层经验沉淀 + 7 候选协调) | ✅ 已存在 | 保持, **扩容** |
| `apeireth-experience` | 经验沉淀 (LLM Wiki + 知识图谱 + 联想网络) | ❌ 新增 | 候选 (inspiration §13) |
| `apeireth-methodology` | 方法论 (M 层 + promotion 管道) | ❌ 新增 | 候选 |
| `apeireth-reflection` | 反思机制 (温度分层 + 3 种粒度) | ❌ 新增 | 候选 (灵感 A) |

### 权限层 (2)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-permission` | 权限矩阵 (AI/人/密钥三维权重 + 权限包) | ❌ 新增 | 候选 (灵感 §16) |
| `apeireth-keys` | 权限密钥管理 + 物理多签 (YubiKey/手机/密码管理器) | ❌ 新增 | 候选 |

### 兼容组件层 (4)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-plugin` | VCP 6 类插件协议扩展 (同步/异步/静态/服务/消息预处理/混合 + 沙箱/审核/拟人化) | ❌ 新增 | 候选 |
| `apeireth-tools` | 工具集合 (30+ 工具实现) | ✅ 已存在 | 保持 |
| `apeireth-pybridge` | PyO3 兼容桥 (现有 1100+ Python 模块) | ✅ 已存在 | 保持 |
| `apeireth-mcp` | MCP 客户端 (兼容外部 MCP 服务) | ❌ 新增 | 候选 |
| `apeireth-environment` | 6 terminal backend (Local/Docker/SSH/Daytona/Modal/Singularity) | ❌ 新增 | 候选 |

### 升级层 (1)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-upgrade` | OTA + 沙盒 + 洋葱测试矩阵 + traffic-shifter | ❌ 新增 | 候选 |

### 通信总线层 (4)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-bus` | 统一通信总线 (inproc / Unix socket / pipe / gRPC) | ❌ 新增 | 候选 |
| `apeireth-gateway` | OpenClaw 模式单长生命周期网关 | ❌ 新增 | 候选 (灵感 §15) |
| `apeireth-server` | HTTP/WS API server (供 dashboard / web app / mobile 接入) | ❌ 新增 | 候选 |
| `apeireth-supervisor` | Erlang/OTP supervisor 树 (B+E 核心) | ❌ 新增 | 候选 |

### 持久化层 (1)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-data` | 数据抽象层 (SQLite/RocksDB/Qdrant/Tantivy/浪潮/时序) | ❌ 新增 | 候选 (阶段 2 §1e) |

### 调度层 (3)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-cron` | 定时调度 (heartbeat / OTA check / 反思触发) | ❌ 新增 | 候选 |
| `apeireth-skills` | 技能管理 (加载/卸载/版本) | ❌ 新增 | 候选 |
| `apeireth-acp` | Agent Communication Protocol (与 OpenClaw 等兼容) | ❌ 新增 | 候选 |

### 监控层 (2)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-telemetry` | OpenTelemetry + Prometheus + 审计日志 | ❌ 新增 | 候选 |
| `apeireth-config` | 配置加载 (YAML/env/secret) | ❌ 新增 | 候选 |

### 测试层 (2)

| Crate | 职责 | R11 现状 | 备注 |
|-------|------|---------|------|
| `apeireth-test` | 测试基础设施 + 模拟器 + 集成测试框架 | ✅ 已存在 | 保持, **扩容** |
| `apeireth-bench` | 性能基准 (V1130 wallclock + V1136 真测 + 多场景) | ✅ 已存在 | 保持 |
| `apeireth-eval` | 基准测试套件 (SWE-bench + Terminal-Bench + YC Bench) | ❌ 新增 | 候选 |

---

## 3. 按 B+E supervisor 子树分组

```
apeireth-supervisor (Level 0, 永存)
  │
  ├── core-supervisor (rest_for_one)         ← 强耦合
  │     ├── apeireth-sovereignty (主 AI)
  │     ├── apeireth-memory (记忆 → experience + methodology)
  │     └── apeireth-philosophy (V3 9 键)
  │
  ├── council-supervisor (one_for_one)        ← 互相独立
  │     ├── apeireth-council (智囊团)
  │     │     ├── 7 persistent 顾问
  │     │     └── N dynamic/ephemeral
  │     └── apeireth-reflection (反思机制)
  │
  ├── plugin-supervisor (transient)           ← 异构子进程
  │     ├── apeireth-plugin (VCP 6 类协议)
  │     ├── apeireth-pybridge (PyO3)
  │     ├── apeireth-mcp (MCP)
  │     ├── apeireth-environment (6 terminal)
  │     └── apeireth-acp (Agent Communication)
  │
  └── upgrade-supervisor (rest_for_one)
        ├── apeireth-upgrade (OTA + 沙盒 + 测试矩阵)
        └── sandbox-validator (升级验证)
```

**核心依赖** (peer 关系):
- apeireth-principle ← 所有 (任何决策都要过 5 层守门)
- apeireth-permission ← 所有 (任何操作都要过权限矩阵)
- apeireth-bus ← 所有 (通信总线)
- apeireth-data ← memory/experience/methodology/reflection (持久化)
- apeireth-runtime ← 所有 (统一 runtime)

---

## 4. 拆分原则 (用户硬约束贯彻)

### 4.1 五条拆分原则

1. **单一职责** — 每个 crate 只做一件事, 高内聚低耦合
2. **独立编译** — 每个 crate 可独立编译 + 测试, 不强制依赖其他 crate
3. **明确边界** — crate 之间用 trait 接口, 不暴露实现细节
4. **按 supervisor 子树** — 一个 crate 对应一个 supervisor 子树组件
5. **不模仿 Hermes** — 按 Apeireth 实际情况, 不预设数量

### 4.2 拆分权衡

**好处**:
- ✅ 内聚高, 复用强
- ✅ 测试独立 (每个 crate 单独测)
- ✅ 升级友好 (热替换 crate)
- ✅ 文档清晰 (每个 crate 一份)

**代价**:
- ⚠️ 编译时间变长 (30 个 crate 全编译 ~2x 时间)
- ⚠️ Cargo.lock 复杂度增加
- ⚠️ 依赖管理需要纪律

### 4.3 与 Hermes 对比

| 项目 | Hermes | Apeireth v1 推荐 |
|------|--------|------------------|
| 总 crate | 17 | 30 |
| 工具 crate | 1 (hermes-tools) | 4 (plugin/tools/pybridge/mcp) |
| 通信 crate | 1 (hermes-bus) | 4 (bus/gateway/server/supervisor) |
| LLM provider | 在 agent 内 | 独立 crate (apeireth-prompt) |
| 记忆 crate | 1 (hermes-memory) | 4 (memory/experience/methodology/reflection) |
| 权限 crate | ❌ 无 | 2 (permission/keys) |
| 升级 crate | ❌ 无 | 1 (upgrade) |

**差异原因**:
- 我们有**自我升级 + 物理多签 + 原则洋葱 + 权限矩阵** → 增量
- 我们有**复杂记忆 7 候选协调** → 增量
- 我们有**智囊团强制 7 + 动态 N** → 增量

---

## 5. 30 个 crate 的依赖图 (顶层)

```
                          apeireth-cli (入口)
                                │
                                ▼
                        apeireth-runtime (统一 runtime)
                                │
                                ▼
                       apeireth-supervisor (Erlang/OTP)
                       /        |          \
                      ▼         ▼           ▼
              core-sup  council-sup  plugin-sup / upgrade-sup
                │           │              │
                ▼           ▼              ▼
         sovereignty    council       plugin/pybridge/
         memory         reflection    mcp/environment
         philosophy                                │
                │                                   ▼
                ▼                              apeireth-tools
         principle                              apeireth-acp
         permission
         experience
         methodology
                │
                ▼
         apeireth-data (多 DB 协调)
                │
                ▼
         apeireth-bus (通信总线)
                │
                ▼
         apeireth-gateway (OpenClaw 模式)
         apeireth-server (HTTP/WS)
         apeireth-telemetry (遥测)
         apeireth-cron (定时)
         apeireth-skills (技能)
         apeireth-prompt (LLM provider)
         apeireth-keys (密钥)
         apeireth-config (配置)
         apeireth-environment (terminal)
         apeireth-acp (Agent Communication)
         apeireth-mcp (MCP)
         apeireth-upgrade (OTA)
                │
                ▼
         apeireth-test (测试)
         apeireth-bench (基准)
         apeireth-eval (基准套件)
```

---

## 6. Cargo.toml workspace 配置 (v1 草案)

```toml
[workspace]
resolver = "2"
members = [
    # 入口层
    "crates/apeireth-cli",
    
    # 核心抽象层
    "crates/apeireth-core",
    "crates/apeireth-runtime",
    
    # 智能层
    "crates/apeireth-asi",
    "crates/apeireth-sovereignty",
    "crates/apeireth-prompt",
    
    # 智囊团层
    "crates/apeireth-council",
    
    # 原则洋葱层
    "crates/apeireth-principle",
    "crates/apeireth-philosophy",
    
    # 经验方法论层
    "crates/apeireth-memory",
    "crates/apeireth-experience",
    "crates/apeireth-methodology",
    "crates/apeireth-reflection",
    
    # 权限层
    "crates/apeireth-permission",
    "crates/apeireth-keys",
    
    # 兼容组件层
    "crates/apeireth-plugin",
    "crates/apeireth-tools",
    "crates/apeireth-pybridge",
    "crates/apeireth-mcp",
    "crates/apeireth-environment",
    
    # 升级层
    "crates/apeireth-upgrade",
    
    # 通信总线层
    "crates/apeireth-bus",
    "crates/apeireth-gateway",
    "crates/apeireth-server",
    "crates/apeireth-supervisor",
    
    # 持久化层
    "crates/apeireth-data",
    
    # 调度层
    "crates/apeireth-cron",
    "crates/apeireth-skills",
    "crates/apeireth-acp",
    
    # 监控层
    "crates/apeireth-telemetry",
    "crates/apeireth-config",
    
    # 测试层
    "crates/apeireth-test",
    "crates/apeireth-bench",
    "crates/apeireth-eval",
]
```

---

## 7. 阶段 2 第三项收尾判定

crate 划分已沉淀：**30 个 crate (v1 推荐)**。

**关键设计**:
- ✅ 按 B+E supervisor 子树 + 原则洋葱 5 层 + 职责单一原则
- ✅ 不模仿 Hermes 17 crate (差异明确)
- ✅ 经验方法论层拆 4 个 (memory / experience / methodology / reflection)
- ✅ 权限层拆 2 个 (permission / keys)
- ✅ 通信总线层拆 4 个 (bus / gateway / server / supervisor)

**R14 增量**:
- 新增 21 个候选 crate (从 9 → 30)
- apeireth-memory 内部扩容 (从单 crate 到 experience/methodology/reflection 协同)
- apeireth-test 内部扩容 (含 eval 子模块)

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (crate 划分服务 ASI 方向)
- 主 17:43 S-2 (基于 B+E 架构, 不重写)
- 主 17:58 O-5 (不假装, 每个 crate 有明确职责)
- 主 19:33 O-2 (Erlang/OTP supervisor 是成熟模式)
- 主 23:44 O-3 (干到底, 立刻沉淀)
- 主 00:56 O-4 (任何接手者都能查依赖图)

**下一步**: 阶段 2 第四项 — **进程/线程/协程分工**

---

## 8. 后续可收敛方向 (v2 备选)

如果 30 crate 编译时间过长 / Cargo.lock 太复杂, 可收敛到 20-22:

```
收敛候选 (8 个合并):
  wave 并入 memory          (A 层联想是 memory 子功能)
  reflection 并入 principle (反思是原则层执行机制)
  keys 并入 permission      (密钥是权限一部分)
  mcp 并入 plugin           (MCP 是 plugin 子类)
  environment 并入 plugin   (terminal backend 是 plugin 之一)
  skills 并入 tools         (技能是工具子集)
  telemetry 并入 server     (遥测是 server 的一部分)
  eval 并入 test            (基准是测试子集)

收敛后: 30 - 8 = 22 个 (类似 Hermes 17 但更细分)
```

**注意**: v1 推荐 30 个, 收敛是**未来选项**, 不是必须. 巨型基地哲学允许冗余.

---

_主哲学 anchor 6 个全贯穿. crate 划分 30 个已沉淀, 下一步等用户确认进入阶段 2 第四项 (进程/线程/协程分工)._