[Document-Meta]
Document: 04-CRATE-CONSOLIDATION.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (基于实测)
Author: Codex (策略分析)

---

# Crate 重组方案 v2 — 基于实测

> v2 关键修正:之前我说"39 → 18 砍掉 21 个",**完全错了**。
> 实测:**37/39 有真实代码,真正可处理的只有 4 个**。
> 正确策略:**保留 39 个 crate 主体,只清理 4 个真小 + 新增 5 个 + 强化部分**。

---

## 1. 实测代码量(再次强调,作为决策依据)

| 状态 | Crate 数 | 说明 |
|---|---|---|
| ✅ 真实代码(>10KB) | **37** | 应保留并强化 |
| ⚠️ 真小/占位/DEPRECATED | **4** | 需要逐个处理 |
| **总计** | **39 + 5 新增 = 44** | 最终目标 |

---

## 2. 真正可处理的 4 个 crate(基于实测,不基于主观判断)

### 2.1 apeireth-philosophy (1.8KB)

**实测证据**:
- 文件头部明确自标 `⚠️ DEPRECATED (2026-07-31)`
- 文件头说:"此 crate 已被 `apeireth-core` 替代"
- 文件头说:"阶段 7+ 物理删除此 crate 目录"
- lib.rs 只有 `placeholder()` 函数返回废弃字符串

**处理**:**物理删除**(履行文件头承诺),把 workspace member 移除。这是项目方**自己说要做的事**,不是我的主观建议。

### 2.2 apeireth-test (618B)

**实测证据**:
- lib.rs 自标 "R14 skeleton (Python mvp/ 接口兼容待 Phase 1)"
- 只有 `placeholder()` 函数

**处理**:**评估后删除或重写**——
- 多数测试在各 crate 内,独立 test crate 价值有限
- 但若有跨 crate 集成测试需求,应**扩充到 ≥ 10KB**,提供真实集成测试
- **建议**:删掉,R14 已过,Phase 1 不存在了

### 2.3 apeireth-bench (2.8KB)

**实测证据**:
- 只有 1 个文件,2.8KB
- 但**职责关键**:SWE-bench / AgentBench 跑分

**处理**:**保留但扩充**——
- 真实做 SWE-bench Verified 跑分框架
- 加 AgentBench 子集
- 加 Self-Disable 攻击场景库
- 目标:≥ 20KB 真实代码

### 2.4 apeireth-desktop (lib 591B / main 26KB)

**实测证据**:
- lib.rs 是占位
- 但 `src/main.rs` 26KB 是真 Tauri 代码
- 还有 `tauri.conf.json` + `gen/icons/` + `Cargo.toml` 完整

**处理**:**不删,但需重构**——
- R17 战役 3 已决定砍 Tauri 前端(交给其他团队)
- 保留 main.rs 的 Tauri 代码作为**参考实现**
- 但 workspace member 标记为 DEPRECATED
- 或改名为 `apeireth-tauri-stub` 并明确"参考实现,不在产品里"

---

## 3. 新增 5 个 crate(基于 5 战区短板)

| 新 crate | 战区 | 必要性 | 优先级 |
|---|---|---|---|
| **apeireth-mcp** | 战区 5 | 必须上车 MCP | 🔴 P0 |
| **apeireth-graph** | 战区 3 | 缺图编排(LangGraph 风格) | 🔴 P0 |
| **apeireth-vector** | 战区 4 | 缺向量检索 | 🟡 P1 |
| **apeireth-sdk** | 战区 1/4/5 | 多语言 SDK 统一 | 🟡 P1 |
| **apeireth-formal** | 战区 5 | 形式化验证(Kani) | 🟢 P2 |

---

## 4. 必须**强化**的现有 crate(不合并,只补全)

### 战区 1:TUI 增强
| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-tui** (255KB) | 5 页面 ratatui 全栈 | 加工作树隔离、Subagent、Hooks、Plan Mode |

### 战区 2:Gateway 补齐
| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-protocol** (139KB) | 4 协议 | 加 Gemini / Cohere / Ollama(本地) |
| **apeireth-api** (197KB) | axum HTTP | 加 Response replay cache(对标 VCP) |

### 战区 3:Multi-Agent
| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-council** (98KB) | 7 advisor | 加图编排、协作模式、角色宪法 |
| **apeireth-supervisor** (22KB) | 调度核心 | 强化,作为图编排执行器 |

### 战区 4:Memory 升级
| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-memory** (120KB) | SQLite 持久化 | 加向量检索、用户画像、跨会话图、自动压缩 |

### 战区 5:Tool Protocol
| Crate | 现状 | 需补 |
|---|---|---|
| **apeireth-tool-registry** (68KB) | 工具注册 | **加小模型分类器(对标 VCP)** |

---

## 5. 不该合并的(基于实测)

之前 v1 文档建议"合并 perception/cognition/consciousness/motivation/life-force 到 apeireth-mind"。**这是错的**:

| Crate | 实测代码量 | 合并理由是否成立 |
|---|---|---|
| apeireth-perception | 29 KB (4 文件) | ❌ 29KB 真实代码,合并丢失清晰度 |
| apeireth-cognition | 29 KB (4 文件) | ❌ 同上 |
| apeireth-consciousness | 15 KB | ❌ 同上 |
| apeireth-motivation | 33 KB | ❌ 同上 |
| apeireth-life-force | 18 KB | ❌ 同上 |
| **总计** | **124 KB 真实代码** | ❌ **不该合并** |

这些 crate 每个都有清晰职责,合并会破坏架构清晰度,得不偿失。

---

## 6. 不该砍的(基于实测)

之前 v1 文档建议"砍 apeireth-sovereignty 274KB(哲学摆设)"。**这是大错特错**:

| Crate | 实测代码量 | 真实作用 |
|---|---|---|
| **apeireth-sovereignty** | **274 KB** (22 文件) | **战区 5 安全核心**,绝对不能砍 |
| **apeireth-asi** | 92 KB (8 文件) | 战区 3 进化机制,有真实代码 |
| **apeireth-council** | 98 KB (18 文件) | 战区 3 Multi-Advisor 系统 |
| **apeireth-evolution** | 107 KB (6 文件) | 战区 3 进化 |
| **apeireth-onion** | 30 KB | 双洋葱核心 |

**这些 crate 全部保留并强化。**

---

## 7. 重组后的最终结构(44 个 crate)

### 战区 1:Terminal Agent (1 个)
- apeireth-tui(强化)

### 战区 2:LLM Gateway (5 个)
- apeireth-api
- apeireth-protocol
- apeireth-http-client
- apeireth-pipeline
- apeireth-extension(可能并入其他)

### 战区 3:Multi-Agent (10 个)
- apeireth-council
- apeireth-supervisor
- apeireth-graph(新增)
- apeireth-evolution
- apeireth-central
- apeireth-perception
- apeireth-cognition
- apeireth-consciousness
- apeireth-motivation
- apeireth-life-force

### 战区 4:Memory (3 个)
- apeireth-memory
- apeireth-bus
- apeireth-vector(新增)

### 战区 5:Tool Protocol (8 个)
- apeireth-tools
- apeireth-tool-runtime
- apeireth-tool-approval
- apeireth-tool-registry
- apeireth-sovereignty
- apeireth-constraint
- apeireth-mcp(新增)
- apeireth-formal(新增)

### L0 HA 核心 (3 个)
- apeireth-core
- apeireth-onion
- apeireth-action

### 价值/行为/进化 (6 个)
- apeireth-value
- apeireth-relation
- apeireth-upgrade
- apeireth-agent
- apeireth-asi
- apeireth-verify

### 跨战区工具 (5 个)
- apeireth-cli
- apeireth-pybridge
- apeireth-sdk(新增)
- apeireth-bench(强化)
- apeireth-test(待删)

### 移交/废弃 (3 个)
- apeireth-web(移交其他团队)
- apeireth-desktop(DEPRECATED)
- apeireth-philosophy(删除)

**总计 44 个 crate**(其中 2 个需删除/降级)

---

## 8. 工程执行清单

### Week 1:清理
- [ ] 物理删除 apeireth-philosophy(履行文件头承诺)
- [ ] 删除 apeireth-test(理由:R14 已过,Phase 1 不存在)
- [ ] apeireth-desktop 改名为 apeireth-tauri-stub + DEPRECATED 标记
- [ ] 备份 `Cargo.toml` 的 `members` 段

### Week 2-3:新增 crate skeleton
- [ ] apeireth-mcp(优先,P0)
- [ ] apeireth-graph(优先,P0)
- [ ] apeireth-vector(P1)
- [ ] apeireth-sdk(P1)
- [ ] apeireth-formal(P2)

### Week 4-6:强化
- [ ] apeireth-tool-registry 加小模型分类器(对标 VCP)
- [ ] apeireth-memory 加向量检索
- [ ] apeireth-protocol 加 Gemini/Ollama
- [ ] apeireth-api 加 Response replay cache
- [ ] apeireth-bench 扩充为 SWE-bench 真实跑分

### Week 7-8:验证
- [ ] cargo check 全部 44 crate 通过
- [ ] cargo test ≥ 2265(不丢)
- [ ] cargo clippy 0 warning

---

## 9. 不修改承诺(LOCKED)

| 不动 | 原因 |
|---|---|
| Cargo.lock | R11 baseline |
| docs/stage1-6 LOCKED | 历史决策 |
| apeireth-core(L0 HA 核心) | "永远不变" |
| 现有 8 项不修改承诺 | 已确立 |

apeireth-philosophy **不是 LOCKED**(自标 DEPRECATED),所以物理删除是履行承诺,不是破坏承诺。

---

_Last update_: 2026-08-04 (v2)
