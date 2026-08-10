[Document-Meta]
Document: 00-VISION.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (基于实测数据重做)
Author: Codex (策略分析) + 主人修正

---

# Apeireth v2.0 — 战略愿景(v2,基于实测数据)

> **v2 修正**:v1 文档误信了 docs/17 的局部自我贬低,把对 1 个 crate 的"占位"描述推广到了全部 39 个 crate。
> **实测结果**:39 个 crate 中,**37 个有真实代码**(>10KB),总代码量约 2.6 MB Rust。**真正的小 crate 只有 4 个**。
> **v2 战区选择**:主人 2026-08-04 决定,**战区 1-5 全要**(除 UI 战区 6 交给别的团队)。

---

## 一句话定位(基于主人决策)

> **Apeireth = VCP 的全栈 Rust 重写 + 双洋葱 + Self-Disable + 形式化安全**
>
> 在 **战区 1(终端 Coding Agent) + 战区 2(LLM 网关) + 战区 3(Multi-Agent) + 战区 4(长期记忆) + 战区 5(工具协议)** 5 个战场上同时打,**对标 VCP 但用 Rust 重写并加上独有的安全原语和哲学架构**。
>
> 战区 6(UI/Web Admin) 交给其他团队。

---

## 我们的真实身份(基于实测代码)

| 维度 | 实测数据 |
|---|---|
| Workspace members | **39 个 crate** |
| 真实代码量 | **~2.6 MB Rust**(src/ 总字节数) |
| 已有真实代码 crate | **37 个**(>10KB) |
| 真正小/占位 crate | **4 个**:philosophy (DEPRECATED 自标) / test (R14 skeleton 自标) / bench / desktop (lib 占位但 main.rs 26KB 真 Tauri 代码) |
| 测试通过率 | 2265 passed / 0 failed(主人确认) |
| 当前版本 | v1.0.0(2026-08-04 已发 tag) |

### 实测代码量 Top 10(都是我之前误判为"空壳"的)

| 排名 | Crate | 代码量 | 战区 |
|---|---|---|---|
| 1 | apeireth-sovereignty | **274 KB** (22 文件) | 战区 5(主权/安全) |
| 2 | apeireth-tui | **255 KB** (12 文件) | 战区 1(终端 Coding Agent) |
| 3 | apeireth-api | **197 KB** (14 文件) | 战区 2(LLM 网关) |
| 4 | apeireth-upgrade | **151 KB** (10 文件) | 战区 3(升级机制) |
| 5 | apeireth-protocol | **139 KB** (10 文件) | 战区 2(协议归一化) |
| 6 | apeireth-web | **135 KB** (12 文件) | 战区 6(交给其他团队) |
| 7 | apeireth-memory | **120 KB** (8 文件) | 战区 4(长期记忆) |
| 8 | apeireth-evolution | **107 KB** (6 文件) | 战区 3(进化) |
| 9 | apeireth-core | **105 KB** (1 文件 lib.rs) | L0 HA 核心 |
| 10 | apeireth-council | **98 KB** (18 文件) | 战区 3(Multi-Agent) |

**关键事实**:这些 crate 都是**真实投入的工程代码**,不是我之前文档里说的"占位"/"空壳"。

---

## 5 战区定位(主人决策)

### 战区 1:终端 Coding Agent
- **基础**:`apeireth-tui` 255KB(5 页面全栈 ratatui: Bridge/Dialogue/Growth/History/Settings)
- **对标**:Claude Code / Codex CLI / Cursor
- **Apeireth 优势**:Rust 性能 + 双洋葱哲学 + Self-Disable 安全

### 战区 2:LLM 网关 / 协议适配
- **基础**:`apeireth-api` 197KB + `apeireth-protocol` 139KB + `apeireth-http-client` 37KB + `apeireth-pipeline` 76KB
- **对标**:VCP / LiteLLM / One-API
- **Apeireth 优势**:Rust 类型安全 + 4 协议归一化真做了

### 战区 3:Multi-Agent 编排
- **基础**:`apeireth-council` 98KB(18 文件 7 advisor) + `apeireth-supervisor` 22KB + `apeireth-evolution` 107KB
- **对标**:LangGraph / AutoGen / CrewAI / MetaGPT
- **Apeireth 优势**:哲学化的器官抽象(consciousness/perception/cognition)是真有代码

### 战区 4:长期记忆
- **基础**:`apeireth-memory` 120KB(8 文件) + `apeireth-bus` 74KB
- **对标**:Letta / Mem0 / Honcho / Zep
- **Apeireth 优势**:基于 SQLite + 双洋葱隔离的记忆治理

### 战区 5:工具协议
- **基础**:`apeireth-tools` 82KB(5 trait 真实现) + `apeireth-tool-runtime` 95KB + `apeireth-tool-approval` 70KB + `apeireth-tool-registry` 68KB
- **对标**:MCP / Composio / VCP 85 插件
- **Apeireth 优势**:Rust 类型化工具 + Self-Disable + 形式化审批

---

## 我们的核心护城河(VCP 没有的)

1. **Self-Disable 防护** — 全行业唯一的"agent 失控硬性 kill switch"原语
2. **双洋葱架构的形式化** — 原理洋葱 + 权限洋葱,编译期可证
3. **Rust 编译期保证** — VCP 只能运行时检查的 invariant,Apeireth 编译时锁死
4. **L0 HA 核心** — `apeireth-core` 105KB,"永不变更"的硬编码脊柱
5. **哲学器官的工程化承载** — consciousness/perception/cognition 不是空壳,而是 14-29KB 的真实 trait

---

## 我们不做的(战区 6 之外)

| 不做 | 原因 |
|---|---|
| ❌ Web Admin UI | 战区 6 交给其他团队 |
| ❌ Tauri 桌面端 | R17 已砍,确认 |
| ❌ 浏览器自动化自研 | 走 MCP 集成 |
| ❌ 图像/视频生成自研 | 走 MCP 集成 |
| ❌ 自研 plugin store | 走 MCP server registry |

---

## 真正需要修正的(基于实测)

| 修正项 | 原因 |
|---|---|
| ~~砍 11 个 crate~~ | 错了。**真正可砍的只有 4 个**:philosophy (DEPRECATED 自标)、test (R14 skeleton 自标)、bench (2.8KB 但需评估)、desktop (lib 占位但有 main.rs) |
| ~~合并 7 组哲学器官~~ | 错了。**不该合并**——它们每个都有 14-29KB 真实代码,合并反而丢失架构清晰度 |
| ~~新增 apeireth-runtime 微内核~~ | **应该新增但定位不同**——不是替代,而是 supervisor 的运行时增强 |

---

## 一句话总结(v2)

**Apeireth 不要做第 11 个 LLM 网关,也不要做"Agent Runtime"——要做"VCP 的 Rust 重写 + 独家的形式化安全 + Self-Disable + 双洋葱",在 5 个战场上同时打,但用 Rust 类型系统和哲学架构形成 VCP 永远无法复制的差异化。**

---

_Last update_: 2026-08-04 (v2 基于实测重做)
