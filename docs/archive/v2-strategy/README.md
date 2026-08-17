> ⚠️ **已归位 (2026-08-15)**: 本目录的 00-VISION / 08-FRONTEND 已移入 `docs/stage1/` (顶层+产品设计); 01-07 已移入 `docs/stage2/` (具体想法)。本 README 仅保留历史索引。详见 `docs/document-relocation-map.md`。

---
[Document-Meta]
Document: README.md
Version: 2.0.0-V2
R-Cycle: v2-strategy
Last-Modified: 2026-08-04
Status: 🟡 DRAFT v2 (基于实测)
Author: Codex (策略分析)

---

# v2-strategy — Apeireth v2.0 战略重塑包(v2)

> **v2 关键修正**:之前 v1 文档误信 docs/17 的局部自我贬低,**严重误判**了 Apeireth 的代码状态。
> **实测数据**:Apeireth **不是空壳**——2.6MB Rust 代码,37/39 crate 有真实工程代码。
> **v2 战略**:主人决策**战区 1-5 全要**,做"**VCP 的全栈 Rust 重写 + 独家的安全原语 + 双洋葱 + 形式化**"。

---

## 阅读顺序

1. **`00-VISION.md`** — 50 行战略愿景(2-3 分钟)
2. **`01-INDUSTRY-LANDSCAPE.md`** — 5 战区行业坐标(10 分钟)
3. **`02-VCP-DEEP-COMPARISON.md`** — VCP 深度对比(15 分钟)
4. **`03-EXTREME-PLAN.md`** — 18 个月极致版路线图(20 分钟)
5. **`04-CRATE-CONSOLIDATION.md`** — 44 crate 重组方案(10 分钟)
6. **`05-EXECUTION-NOW.md`** — 本周 7 步立即执行(5 分钟)
7. **`06-TUI-UPGRADE-ROADMAP.md`** — TUI 升级路线图(TUI → Tauri 集成测试床, 4 步走)

---

## v2 vs v1 关键修正

| 维度 | v1 错误判断 | v2 实测正确判断 |
|---|---|---|
| **apeireth-sovereignty** | ❌ 哲学摆设,建议砍 | ✅ **274KB 真实代码**,战区 5 安全核心 |
| **apeireth-asi** | ❌ 熵增宇宙论,建议砍 | ✅ 92KB 真实代码,战区 3 进化机制 |
| **apeireth-council** | ❌ 只是哲学摆设 | ✅ **98KB 18 文件**,7 advisor 真实实现 |
| **apeireth-tools** | ❌ 800B 占位 | ✅ **82KB 真实代码**,5 trait 真实现 |
| **apeireth-tui** | ❌ 刚写完 | ✅ **255KB 5 页面全栈 ratatui** |
| **apeireth-protocol** | ❌ 占位 | ✅ **139KB 4 协议归一化真做** |
| **战区选择** | 建议只打 2-3-4 战区 | 主人决策:**战区 1-5 全要** |
| **核心定位** | 建议"窄 Runtime" | "**VCP 全栈 Rust 重写 + 独家安全原语**" |
| **砍掉数量** | 建议砍 11 个 | **只清理 4 个真小** |
| **新增数量** | 建议新增 5 个 microkernel | **新增 5 个 P0 战区 crate** |

---

## 5 战区最终选择(主人决策)

| 战区 | 是否打 | 实测基础 | 18 月目标 |
|---|---|---|---|
| 1. Terminal Coding Agent | ✅ 打 | tui 255KB | SWE-bench ≥ 60% |
| 2. LLM Gateway | ✅ 打 | 4 crate 449KB | 5+ provider |
| 3. Multi-Agent | ✅ 打 | 9 crate 408KB | 5+ advisor 协作 |
| 4. Memory | ✅ 打 | memory 120KB + bus 74KB | Letta 水平 |
| 5. Tool Protocol | ✅ 打 | 5 crate 311KB + MCP 新增 | MCP 100% 兼容 |
| 6. UI/Web Admin | ❌ 移交 | web 135KB 交给其他团队 | — |

---

## 实测代码量 Top 10(支撑 5 战区)

| Crate | 代码量 | 战区 |
|---|---|---|
| sovereignty | 274 KB | 战区 5 |
| tui | 255 KB | 战区 1 |
| api | 197 KB | 战区 2 |
| upgrade | 151 KB | 战区 3 |
| protocol | 139 KB | 战区 2 |
| web | 135 KB | (移交) |
| memory | 120 KB | 战区 4 |
| evolution | 107 KB | 战区 3 |
| core | 105 KB | L0 HA |
| council | 98 KB | 战区 3 |

**总代码量 ~2.6 MB Rust**——这是真实的工程投入,不是我之前说的"哲学空壳"。

---

## 真正需要处理的 4 个真小 crate

| Crate | 实测 | 文件头自标 | 处理 |
|---|---|---|---|
| **apeireth-philosophy** | 1.8 KB | ⚠️ DEPRECATED (2026-07-31) | 物理删除(履行承诺) |
| **apeireth-test** | 618 B | "R14 skeleton (Python mvp/ 接口兼容待 Phase 1)" | 删除(R14 已过) |
| **apeireth-bench** | 2.8 KB | — | 保留但扩充(SWE-bench 真实跑分) |
| **apeireth-desktop** | lib 591B / main 26KB | — | 改名 apeireth-tauri-stub + DEPRECATED |

---

## 新增 5 个 crate(战区短板补齐)

| 新 crate | 战区 | 优先级 | 必要性 |
|---|---|---|---|
| apeireth-mcp | 战区 5 | 🔴 P0 | MCP 必须上车 |
| apeireth-graph | 战区 3 | 🔴 P0 | 缺图编排 |
| apeireth-vector | 战区 4 | 🟡 P1 | 缺向量检索 |
| apeireth-sdk | 战区 1/4/5 | 🟡 P1 | 多语言 SDK |
| apeireth-formal | 战区 5 | 🟢 P2 | 形式化验证 |

---

## 最终 44 crate 结构

- **战区 1** (1): tui
- **战区 2** (5): api, protocol, http-client, pipeline, extension
- **战区 3** (10): council, supervisor, **graph(新)**, evolution, central, perception, cognition, consciousness, motivation, life-force, **companion(A12.5 2026-08-14 主人拍板新增)**
- **战区 4** (3): memory, bus, **vector(新)**
- **战区 5** (8): tools, tool-runtime, tool-approval, tool-registry, sovereignty, constraint, **mcp(新)**, **formal(新)**
- **L0 HA** (3): core, onion, action
- **价值/行为** (6): value, relation, upgrade, agent, asi, verify
- **跨战区工具** (5): cli, pybridge, **sdk(新)**, bench, ~~test~~
- **移交/废弃** (3): web(移交), ~~philosophy~~, desktop → tauri-stub

**最终 44 个 crate**——其中 5 个新增,2 个删除,1 个重命名,1 个 DEPRECATED。

---

## 不修改承诺(LOCKED)

| 不动 | 原因 |
|---|---|
| Cargo.lock | R11 baseline |
| docs/stage1-6 LOCKED | 历史决策 |
| apeireth-core(L0 HA 核心) | "永远不变" |
| 8 项不修改承诺 | 已确立 |

apeireth-philosophy **不是 LOCKED**,文件头自标 DEPRECATED,物理删除是**履行承诺**。

---

## 一句话总结(v2)

> **Apeireth 不是哲学空壳,是 2.6MB Rust 真实工程的 39 crate 全栈 AI Agent 平台。**
>
> **v2 战略:不是"砍到 18 做窄 Runtime",而是"5 战区全打,对标 VCP 但用 Rust 重写并加入独家安全原语"**。
>
> **这是基于实测的修正,不是我之前基于二手描述的误判。**

---

## 风险

| 风险 | 对策 |
|---|---|
| 主人不接受 v2 战略 | 这是基于实测的判断,真实数据支持 |
| 资源不够 5 战区 | 严格优先级:先 MCP + graph + memory + tool 分类 |
| 物理删除 philosophy 误判 | git revert 一键回滚;文件头已自标 DEPRECATED |
| 形式化验证延后 | P2 优先级,不阻塞主流程 |

---

_Last update_: 2026-08-04 (v2 基于实测)