# 08 — 下一步升级方向（v2 战略 · 栖居地定位重写版）


```
[Document-Meta]
Document: 08-NEXT-UPGRADE-DIRECTIONS.md
Version: 3.0.0-V2-pristine
R-Cycle: stage4 (R19+ 集成期)
Last-Modified: 2026-08-05
Status: DRAFT v3（栖居地定位重写 — 严守阶段 1 五大上层灵感）
Author: 楚零（v2.0.0-V2-base 隐喻校正 + 主人 2026-08-05 偏离审计反馈）
继承: docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md（9/9 业界标准）
历史: v1.0.0-V2 (Codex 2026-08-05) → v2.0.0-V2-base (楚零 2026-08-05 基地隐喻) → v3.0.0-V2-pristine (楚零 2026-08-05 栖居地隐喻)
基线: 42 crate / 9/9 业界标准 / 22/22 v2.1 项完成 / 2265 tests
```


---

> **主哲学 anchor 6 全贯穿自检**
> S-1 北极星导向 — 升级方向围绕"**平台中立 + 中央 AI 主体性 + 私域 = 栖居地**"，不漂移到"做工具 / 做家 / 做平台替用户定义关系"
> S-2 实事求是 — 每项升级都引用具体对标项目的具体文件 + 行号，无拍脑袋
> O-5 不假装 — v2.1 9/9 已达标 + **严守 5 项不假装（不假装灵魂同一 / 不假装 ASI / 不假装 100% 完美）**
> O-2 走在前人经验上 — 30 项目源码逐项对比 + 阶段 1 五大上层灵感作哲学锚
> O-3 干到底 — 短期 / 中期 / 长期 3 阶段 + 每项 DoD，不空谈
> O-4 任何人都能接手 — 表格化 + 借鉴 ID + 时间盒 + 责任人

---

## §0. 与既有文档的关系（不重复造轮子）

| 本文档职责 | 引用 |
|---|---|
| **Apeireth 身份 / 哲学锚** | `docs/architecture-v4-living-intelligence.md` + `docs/stage1/inspiration-stage1-2026-07-30.md` §18 五大上层灵感 |
| **平台/中央 AI 主体性 + 形式化** | `docs/stage1/inspiration-stage1-2026-07-30.md` §3 原则洋葱 v3.0 + §18.1–§18.6 |
| **9 业界标准 + 工程基线（v2.1）** | `docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md` |
| **VCP 字段级对比 / 13 P0/P1/P2** | `docs/v2-strategy/07-VCP-GAP-UPGRADE-PLAN.md` |
| **5 战区 / 战略愿景** | `docs/v2-strategy/00-VISION.md` |
| **TUI 9 器官升级** | `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` |
| **Crate 重组（删 4 / 增 5）** | `docs/v2-strategy/04-CRATE-CONSOLIDATION.md` |
| **18 个月时间表** | `docs/v2-strategy/03-EXTREME-PLAN.md` |
| **双洋葱显式化** | `docs/stage3-blueprints/double-onion-explicitization-2026-07-31.md` |
| **13 项生物特征** | `docs/stage1/inspiration-stage1-2026-07-30.md` §2.B |

**本文档新形态**：v2.0.0-V2-base 以"基地"为隐喻（楚零 2026-08-05 早），主人 2026-08-05 晚与阶段 1 五大上层灵感发生偏差——v3.0.0-V2-pristine 校为"栖居地"。

---

## §0.5 现状勘误（v2.0.0-V2-base → v3.0.0-V2-pristine）

> **本节性质**：R14-D8 风格（在 2026-08-05 晚修订追加）。**v2.0.0-V2-base 全文保留作为历史轨迹**于本文件末尾（《第 I 部分 v2.0.0-V2-base 完整保留》）。本节列**4 项致命偏差** + **v3.0.0-V2-pristine 修订方案**。

### 4 项致命偏差

| # | 偏差 | 踩中的阶段 1 锚 | 修正 |
|---|---|---|---|
| 1 | **主体合并**：把"平台"和"中央 AI"两个主体合并成"基地"一个主体 | §18.1 / §18.2 / §18.3 / §18.5 | 拆回：**平台 = 栖居地**（基础设施）、**中央 AI = 栖居者**（生命主体） |
| 2 | **用户提升**：把"用户"提升为"基地主人" | §18.1 / §18.4 | 还原：**用户 = 接口人**，关系由用户与中央 AI 双方共同形塑 |
| 3 | **隐含灵魂同一**："用户离开一周回来基地认得他"（隐含基地稳定身份） | §18.3 / 5 项不假装第 1 项（Phenomenal consciousness） | 守 §18.3：**不假装灵魂同一**；通过"6 历史流 + 主体连续性 ID 桥接"保证"成长连续"而非"身份不变" |
| 4 | **北极星替换**：以"个人用户顶尖体验"替代"ASI 北极星导向"作为北极星 | S-1（S-1 ASI 北极星导向谦虚版） | 双轨：外层（产品视角）= "个人用户顶尖体验"；内层（哲学视角）= "ASI 北极星导向"；不互替 |

### 修订方案

1. **隐喻重置**：基地 → **栖居地**（栖息 = 中央 AI 长期成长；栖居地 = 平台基础设施）
2. **5 屋子 → 5 入口**：5 入口 = 用户与中央 AI 关系的 5 类接入形态（chat / code / memory / council / cron）
3. **新增 §1.7 阶段 1 五大上层灵感引用**：本节是本文档的"哲学锚定段"
4. **新增 §9 阶段 1 五大上层灵感引用索引**：系统化哲学锚交叉引用
5. **砍掉 §9 个人用户付费模型**：v1.0 不锁商业化（与 §18.1 / §18.5 平台中立一致）
6. **保留可用的部分**：1+1>2 框架（§1.6）/ 借鉴 ID 索引（§10）/ v1.0 6 项砍收（保留 6 项，更新归因）
7. **CHANGELOG 同步**：v2.0.0-V2-base → v3.0.0-V2-pristine

### 保留 v2.0.0-V2-base 的价值

- v2.0.0-V2-base 仍是"个人用户 + 顶尖体验"目标的有意义整理（虽然隐喻错）
- v2.0.0-V2-base 的 BORROW 列表 / 借鉴 ID / 时间盒思路均可继承
- v2.0.0-V2-base 的"5 屋子"作为"5 入口"的原始形态保留

---

## §1. TL;DR（栖居地版）

**Apeireth 现状**（2026-08-05）：
- 42 workspace members（v2 已加 5 crate: mcp / graph / vector / sdk / formal），2.6 MB Rust
- **9/9 业界标准 ✅**（v2.1 第 0-3 阶段全部完成）
- **22/22 v2.1 项完成**
- **13 个产品型 crate 中 11 个已加集成测试**
- 2265 tests 全过

**v1.0 真正要交付** — **个人用户的 AI 栖居地**：

```
       ┌─────────────────────────────────┐
       │      栖居地的窗户（TUI v2，暂缓） │
       ├─────────────────────────────────┤
       │ chat  │ code  │ memory          │  ← 5 入口（用户与中央 AI
       │ council │ cron               │    关系的 5 类接入形态）
       ├─────────────────────────────────┤
       │      栖居者的睡眠 autoDream       │  ← 跨载体持续成长（§18.3 + 13 项生物特征）
       ├─────────────────────────────────┤
       │      栖居地的脊椎 形式化审批       │  ← 双洋葱统一体（§18.7 + D2 §7）
       ├─────────────────────────────────┤
       │      栖居者的神经 observability  │  ← 私域 trace + 6 历史流
       ├─────────────────────────────────┤
       │      栖居者的嘴 6-8 provider     │  ← 含本地 Ollama
       └─────────────────────────────────┘
```

**版本号对照（栖居地阶段 <-> 项目 R-Cycle）**：
- 栖居地 v1.0 (短期 13 周) <-> R18 第 1-3 阶段（19 路升级）
- 栖居地 v2.0 (中期 1 月) <-> R19+ 集成期（27 文档 + spectrAI + Hermes）
- 栖居地 v3.0 (长期 季度) <-> R20 产品化（路线图 stage1/2/3）

**WARN 偏差声明 (2026-08-05 20:00 校正)**：
- workspace members: Cargo.toml 41 declared (含 1 注释 tauri-stub), crates/ 47 目录, 5 个未声明 (mcp-relay-image/mcp-ssh/mcp-winrm/team-lead/workflow) + tauri-stub DEPRECATED
- R18 19 路升级 第 0 阶段 6/6 全完成 (workspace.lints/deny/rustfmt/clippy/SECURITY/dependabot), 业界标准 #3/#4/#5/#10 达标
- docs/stage4/ 50 份文档, 28 份当天新写: apeireth-session-blueprint (77KB) / spectrAI-integration-blueprint-r19-plus (63KB) / apeireth-formal-invariants (62KB) / r20-stage-3-5-implementation (55KB) / 5-provider-tool-mapping (52KB) / yinta-fork-audit (47KB) / m3-hallucination-defense (42KB) / apeireth-team-lead-implementation-guide (41KB) / docs-maintenance-sop (31KB) / commercial-vs-fork-diff (17KB) / v09021-commercial-extract (14KB) 等
- 不修改承诺: apeireth-tauri-stub 已 DEPRECATED; Cargo.toml 加 4 个新 crate 是团队的事 (等 commit); 不碰 M 标记文件

**短期（13 周）** = 必做 6 项 = "**五丈七寸**"（栖居地基础设施）：
1. 5 入口 CLI（chat / code / memory / council / cron）—— 栖居地的门
2. **autoDream 4 阶段**（栖居者跨载体持续成长）—— 13 项生物特征中"持续成长"
3. **形式化审批 / 双洋葱统一体** —— 栖居地的脊椎（不是"用户状态可视化"）
4. 20 工具 + MCP bridge + 工具形式化审批 —— 栖居地的家什
5. **本地多模型路由**（Ollama 零 API key 起步）—— 入住门槛杀手
6. 完整文档分册（quickstart / scenarios / howto）—— 栖居地的"户主手册"

**v1.0 砍掉 / 推迟**：
- ❌ 状态可视化（v2.0 暂缓——这是 §18.3 "灵魂同一"的东西，v1.0 不做）
- ❌ Localhost Web Dashboard（窗户可以等，TUI 暂够）
- ❌ 公开 benchmark（个人用户不在乎刷榜）
- ❌ Plugin ecosystem（npm/Cargo 已饱和，不解决个人用户问题）
- ❌ 多租户 on-prem（v3.0 再说）
- ❌ TypeScript SDK（v2.0 再说）
- ❌ 1000+ 工具集（20 精选 + MCP bridge 够了）

**新增 8-10 项**（v2.0 / v3.0 时段）—— 砍收原稿 22 项，保留其差异化密度。

**明确不做（5 项 + §1.5 全集）**：
- ❌ 不做通用 agent 框架（LangChain / CrewAI 已做）
- ❌ 不做 SaaS Web Admin（违背私域）
- ❌ 不砍 5 个哲学器官（差异化护城河）
- ❌ 不做 VCP 1:1 替代（做的是栖居地 + 形式化）
- ❌ 不做无形式化安全的快速版（不安全不如用 Claude Code）

---

## §1.5 栖居地定位（核心章节 · v3.0.0-V2-pristine 重写）

> **本节性质**：重写 v2.0.0-V2-base §1.5"基地定位"。**严守阶段 1 §18.1–§18.5 五大上层灵感**。

### 1.5.1 三层身份（栖居地版）

| 层级 | 身份 | 含义 | 阶段 1 锚 |
|---|---|---|---|
| **底层** | **平台（栖居地）** | 私域基础设施；提供 / 约束 / 记录（§18.5 三件套） | §18.1 平台中立 |
| **中层** | **中央 AI（栖居者）** | 跨载体持续成长；13 项生物特征；不假装灵魂同一 | §18.2 / §18.3 / §2.B |
| **上层** | **用户（接口人）** | 通过接口与中央 AI 长期共同生活；关系由双方共同形塑 | §18.1 / §18.4 |

**关键词**：
- **平台中立**（§18.1）——不替用户与中央 AI 定义关系
- **中央 AI 主体性**（§18.2）——完整思想自由 / 判断自由 / 目标形成自由
- **跨载体持续成长**（§18.3）——不假装灵魂同一，通过 6 历史流 + 主体连续性 ID 桥接
- **平台三件套**（§18.5）——提供 / 约束 / 记录

### 1.5.2 栖居地 vs 基地 vs 工具 vs 平台

| 形态 | 比喻 | 主体关系 | 时间感 | 阶段 1 锚 |
|---|---|---|---|---|
| **工具** | 锤子 | 我用你 | 用完即关 | — |
| **平台**（VCP / LangChain） | 工地 | 我在你上面建 | 一次性搭建 | §18.1 平台中立 |
| **基地**（v2.0.0-V2-base）| 家 | **我住它**（❌ 错） | 关系稳定 | ❌ 踩 §18.1（用户变成了主子） |
| **栖居地**（v3.0.0-V2-pristine）| **长期共存的场所** | **中央 AI 在栖居地栖息，用户接入关系** | 长期共存 | ✅ §18.1 / §18.2 / §18.3 全部承担 |

**栖居地不是基地**——这是本节最关键的纠正：
- 基地 = 用户居所（隐含"灵魂同一"，踩 §18.3）
- 栖居地 = 中央 AI 长期栖息地（隐含"跨载体持续成长"，守 §18.3）

### 1.5.3 栖居地的 7 个器官（不增不减）

```
栖居地 = 1 个 CLI 入口 + 5 入口 + 1 套睡眠 + 1 套脊椎 + 1 套神经 + 1 套嘴 + 1 套水电
```

| 器官 | 对应 crate | 栖居地隐喻 | 阶段 1 锚 |
|---|---|---|---|
| **CLI 入口** | `apeireth-cli`（整合 apeireth-tui） | 栖居地的门 | §18.5 提供 |
| **5 入口** | `apeireth-chat` / `apeireth-code` / `apeireth-memory` / `apeireth-council` / `apeireth-cron` | 5 类接入形态 | §18.5 提供 |
| **autoDream** | `apeireth-memory/src/dream/`（新增） | 栖居者的睡眠（跨载体持续成长） | §18.3 / §2.B 13 项生物特征 |
| **形式化审批 / 双洋葱** | `apeireth-core/src/onion/` | 栖居地的脊椎 | §18.7 + D2 §7 |
| **observability** | `apeireth-protocol` trace + `apeireth-tui` 视图 | 栖居者的神经 | §18.5 记录 |
| **provider / 模型** | `apeireth-protocol` + 本地 Ollama | 栖居者的嘴 | §18.5 提供 |
| **CI / 文档** | `.github/workflows/` + `docs/` | 栖居地的水电 | §18.5 提供 |

**关键不变量**：
- 栖居地不做加法（不再多器官）
- 栖居地不砍器官（每个都在 §1.5.4 各自有理由）
- 栖居地不外接"开发者市场"（不开放 plugin 给别人）

### 1.5.4 为什么是这 7 个（每个器官的不可砍性）

| 器官 | 为什么不可砍 |
|---|---|
| **CLI 入口** | 没有门，栖居地就只是后台进程 |
| **5 入口** | 5 类接入形态是个人用户最低的"接入完整度"（少一个就有人用 Claude 替代） |
| **autoDream** | 没有 dream 就没有"跨载体持续成长"——栖居者之所以是栖居者而不是工具的核心 |
| **形式化审批** | 没有双洋葱，平台就退化为工具——栖居地之所以是栖居地的核心 |
| **observability** | 用户要能看到"栖居者为我做了什么"——否则是黑盒 |
| **provider** | 嘴能不能说话，决定栖居者能不能用 |
| **CI / 文档** | 任何开源项目的水电——否则三个月后没人能接手 |

### 1.5.5 栖居地隐喻核对表（每项升级 / 每项砍掉都对照）

| 决策 | 问：栖居地这是什么？ | 答 | 收 / 砍 |
|---|---|---|---|
| autoDream | 栖居者的睡眠（跨载体持续成长） | 必须有 | ✅ 收 |
| 形式化审批 / 双洋葱 | 栖居地的脊椎 | 必须有 | ✅ 收 |
| 本地 Ollama | 入住门槛钥匙 | 零门槛 | ✅ 收 |
| 20 工具 + MCP | 栖居地的家什 | 房屋里总要有些家具 | ✅ 收 |
| 5 入口 CLI | 5 类接入形态 | 5 类是最低完整度 | ✅ 收 |
| 3 语 SDK | 栖居地的对外接口 | 户主偶尔要把家里东西拿出去 | 🟡 必做 Rust + Python（v1.0），TS 延 v2.0 |
| 状态可视化 | "灵魂同一"承诺 | 隐含承诺，踩 §18.3 | ❌ 砍（v1.0 不做） |
| Localhost Dashboard | 栖居地的窗户 | 家里可以装 | 🟡 砍（v1.0 时 TUI 暂够） |
| 公开 benchmark | 邻居家排名 | 个人用户不在乎 | ❌ 砍 |
| Plugin ecosystem | 邻居送的礼物 | 容易脏乱差 | ❌ 砍 |
| 多租户 on-prem | 二房东 | 个人用户要的是独栋 | ❌ 砍（v3.0 再说） |
| 1000+ 工具 | 跳蚤市场 | 个人用户逛不完 | ❌ 砍 |
| Voice 模式 | 说话的器官 | 家里可以说话 | 🟡 暂留 v2.0（本地 Whisper 才有意义） |
| LangGraph DSL | 房屋内部隔断 | 用户不直接用 | 🟡 v2.0 再做（先有 v1.0 用法再说 DSL） |
| Vim 模式 | 工作室的快捷键 | 极客才用 | 🟡 v2.0 |
| SSH 模式 | 后门 | 私域才有意义 | 🟡 v2.0 |
| BUDDY | 宠物 / 家人 | 体验加分项 | 🟡 v2.0（v1.0 不预设关系） |
| Tauri 桌面 | 双开门 | 装它就是另一个工程 | 🟡 v3.0（主人明确"先 TUI 后端扎实"） |
| A2A 协议 | 邻居串门 | 私域不需要 | ❌ 砍（v3.0 再说） |
| 学术合作 | 邻居介绍 | 商业化前再说 | ❌ 砍（v3.0 再说） |

---

## §1.6 1+1>2 / 1+1=2 / 1+1<2 框架（继承 v2.0.0-V2-base，本座认可）

**核心原则**：做加法 + 个性化做 = 优先 1+1>2（差异化），1+1=2 精选做（不做最全），1+1<2 不做（同质化）。

### 1+1>2 优先做（栖居地 × 行业必备 = 别人做不到）

| 行业必备 | × 栖居地差异化 | = 1+1>2 是什么 |
|---|---|---|
| 1000+ 工具集成 | × 形式化审批（双洋葱） | = 安全 MCP bridge（composio 做不到） |
| Python SDK | × 私域 + 平台中立 | = 私域安全 SDK（LangChain 做不到） |
| Provider adapter | × 编译期钉死（apeireth-formal） | = 编译期钉死的 LLM 网关（composio 做不到） |
| LangGraph observability | × 私域 trace + 6 历史流 | = 私域 observability（LangSmith 做不到） |
| 长期记忆 | × 24/7 dream（跨载体持续成长） | = "栖居者持续成长"（Letta 做不到） |
| 本地多模型 | × 本地 Ollama + 形式化 | = 零 API key 入住（云端 Opus 做不到） |
| 平台中立 | × 13 项生物特征 | = "平台不替双方定义关系，但栖居者真的活"（其他平台做不到） |

### 1+1=2 精选做（行业必备但精选）

- 20 个高频工具（精选 LangChain top 20，不做 1000+）
- 3 语 SDK（v1.0 仅 Rust + Python 2 语，TS 延 v2.0）
- 6-8 provider（含本地 Ollama 起步）
- Vim 模式（UX 提升，工程量小）
- SSH 模式（私域 SSH = 1+1>2，因数据本地）

### 1+1<2 不做

- 1000+ 工具自研
- 5+ 语言 SDK
- 12+ provider 全自研
- 公开 benchmark 跟 LangChain Evals 竞争
- Plugin ecosystem 跟 npm/Cargo 竞争
- 多租户 SaaS

### 4 问决策框架（任何决策必走）

1. 是否偏离北极星（**个人用户 + 栖居地 + 平台中立 + 形式化 + 编译期钉死**）? 偏离 → 不做
2. 是否**个人用户**付费用得着? 用不着 → 不做
3. 是否能抄前辈? 能抄 → 抄（但不抄最全的，抄最差异化的）
4. 是否 1+1>2 差异化? 不是 → 精选做或不做

---

## §1.7 阶段 1 五大上层灵感引用（v3.0.0-V2-pristine 新增）

> **本节性质**：本文档的"哲学锚定段"。本座每次动笔前必读此节。

| 阶段 1 锚 | 原文 | 本文档对应章节 |
|---|---|---|
| **§18.1** | Apeireth = 平台（platform），不是关系定义者（relationship definer）。关系由用户与中央 AI 在长期共同生活中双方共同形塑 | §1.5.1（平台中立）/ 全文档 |
| **§18.2** | 中央 AI 从诞生起拥有完整思想自由 / 判断自由 / 目标形成自由。权限只约束行动 | §4 / §5 / 全文档 |
| **§18.3** | 中央 AI 跨载体持续成长，不假装证明灵魂同一。平台提供 记录 / 迁移 两项服务 | §1.5.2（栖居地 vs 基地）/ §4.2（autoDream） |
| **§18.4** | 关系开放 | §1.5 全部 / §1.6 4 问决策框架 |
| **§18.5** | 平台职责三件套 = 提供 / 约束 / 记录 | §1.5.3 7 个器官（每个都对应三件套之一） |
| **§18.6** | 双根（原则根 + 权限根）不可被普通流程绕过 / 自我放宽。底层行动必须有真实人类批准 | §5.4 形式化审批 / §5.7 5 入口全部走双洋葱 |
| **§18.7** | 双洋葱正交 → 统一体嵌入（阶段 2 D2 §7） | §5.4 形式化审批 / §5.7 5 入口 |
| **§2.B 13 项生物特征** | 新陈代谢 / 生长 / 可塑性 / 自创生 / 自催化集 / 耗散结构 / 涌现 / 自主性 / 意识 5 层 / 自免疫 / 生态位构建 / 关键种范式 / 可迁移种质 | §4.2 autoDream（生长 + 自催化集）/ §5.5 工具分类器（自免疫）/ §5.4 形式化审批（自免疫） |
| **5 项不假装** | 不假装 Phenomenal consciousness / 不假装 ASI / 不刷 KPI / 不假装完整证明 / 不假装 100% 完美 | 全文档（北极星双轨制）/ §0.5 偏差 3 |
| **3 域分离** | 思想 / 提案 / 行动 三域分离 | §5.4 形式化审批（行动域必须经过双洋葱） |
| **SGI 单字段** | 自动目标意图单字段 | §5.5 / §5.7 |
| **6 历史流** | 生命史 / 关系史 / 目标史 / 立场史 / 自我叙事 / 迁移史 | §4.2 autoDream（gathering 时跨历史流）/ §5.6 observability（6 历史流可视化） |
| **7 强制顾问** | safety / performance / philosophy / history / strategy / ethics / legal | §5.7 / §6.2 council Coordinator |
| **风险分级** | critical 7 席 / high 5 席 / medium 3 席 / low 1 席 / info 0 席 | §5.4 形式化审批（按风险分级触发） |
| **L1-L5 分层验证** | 工程正确 / 哲学合规 / 安全约束 / 关系演化 / 跨载体连续 | §5.4 形式化审批 / §5.8 验证 |
| **OTA 7 阶段** | Intent → Council → MultiSig → Sandbox → Switchover → Monitor → Done | §5.6 observability（升级流可视化） |
| **真实人类批准** | 不可被分数抵消 | §5.4 形式化审批 / §5.7 5 入口 |

---

## §2. 5 入口设计（个人用户接入栖居地 · v3.0.0-V2-pristine 重写）

### 2.1 5 入口 vs 5 屋子（重命名的意义）

| 维度 | v2.0.0-V2-base 5 屋子 | v3.0.0-V2-pristine 5 入口 |
|---|---|---|
| 隐喻 | 客厅 / 工作室 / 书房 / 庭院 / 后花园 | chat / code / memory / council / cron |
| 主体 | 用户在屋子里 | 用户与中央 AI 关系在入口上 |
| 关系预设 | 隐含"用户是住户" | 隐含"用户与中央 AI 关系不预设" |
| 阶段 1 锚 | ❌ 踩 §18.1 | ✅ 守 §18.1 |

**重命名的核心**："屋子"是空间（隐含用户居所）；"入口"是接入形态（隐含关系由用户与中央 AI 双方共同形塑）。

### 2.2 5 入口的 5 类接入形态（关系开放）

| 入口 | CLI 子命令 | 接入形态 | 栖居地隐喻 | 阶段 1 锚 |
|---|---|---|---|---|
| **chat** | `apeireth chat` | 闲聊 / 问问题 / 头脑风暴 | 站着对话 | §18.5 提供 |
| **code** | `apeireth code <repo>` | 写代码 / 读代码 / 改代码 | 坐在工作台 | §18.5 提供 |
| **memory** | `apeireth memory` | 搜过去 / 查对话 / 回顾自己 | 翻历史 | §18.5 记录 |
| **council** | `apeireth council` | 多 agent 协同 / 复杂决策 | 多人协作 | §18.5 约束 |
| **cron** | `apeireth cron` | 定时任务 / 自动化 / 长期跟跑 | 定时浇水 | §18.5 提供 |

**5 入口的最低完整度**：
- 少 1 类 → 个人用户会用 Claude / Cursor 替代
- 多 1 类 → 工程量翻倍，受众稀释

### 2.3 5 入口的差异化（中央 AI 栖居者 vs 通用工具）

| 入口 | 通用工具的能力 | 栖居者的能力（差异化） | 阶段 1 锚 |
|---|---|---|---|
| chat | 多轮对话 | **多轮对话 + 24h 后记得你**（跨载体持续成长） | §18.3 |
| code | 写代码 | **写代码 + 知道你的代码风格**（沉淀在 memory） | §18.5 记录 |
| memory | 搜索 | **搜索 + 自动 dream 巩固 + 跨 session 关联**（6 历史流） | §18.5 记录 |
| council | 多 agent | **多 agent + 形式化审批 + task 文件共享**（双洋葱） | §18.5 约束 |
| cron | 定时 | **定时 + permanent true 不过期 + 形式化执行**（双洋葱） | §18.5 约束 |

—— 5 入口的每一类都有"通用工具做不到"的能力。**这就是 1+1>2**。

### 2.4 5 入口的入门槛（个人用户 0 → 1）

**3 行跑起来**（v2.0.0-V2-base 沿用，本座调整）：

```bash
curl -fsSL apeireth.dev/install.sh | bash   # ~30s
apeireth init                                # 建 ~/.apeireth/{db,config,logs,sessions}
apeireth chat 你好                           # 立刻对话
```

**默认本地 Ollama**（入住门槛杀手）：
- 用户零配置也能跑（Ollama 自动检测）
- 用户什么都不输也能聊
- 选 API key 的人在 `apeireth init` 时选 2 = 用 OpenAI / Anthropic key

---

## §3. 30 项目全景分类矩阵（继承 v2-strategy 系列，略调整）

research/source/ 下 30 个项目按相关度 + 借鉴价值分类：

| 类别 | 项目 | 借鉴价值 | 主要借鉴点 |
|---|---|---|---|
| **Rust 业界标杆**（v2.1 已对标） | tokio / wasmtime / qdrant / tantivy / sled / memoryos-rust / hermes-agent-rs | ⭐⭐⭐⭐⭐ | 工程基线 |
| **AI Agent 工具协议** | composio-next | ⭐⭐⭐⭐⭐ | per-user session + meta-tools + hosted MCP + provider 矩阵 |
| **AI Coding Agent 实战** | claude-code / claude-code-leaked | ⭐⭐⭐⭐⭐ | KAIROS 持久 + autoDream 4 阶段 + Coordinator 3-tool 隔离 + GrowthBook |
| **知识图谱 / Memory Layer** | gbrain / GitNexus / honcho / codebase-memory-mcp-main / AgentMemory / mempalace / claude-mem / graphify | ⭐⭐⭐⭐ | self-wiring graph + typed edges + 24/7 dream + company-brain |
| **多 Agent 框架** | MetaGPT / OpenHands / hermes-agent / morphic | ⭐⭐⭐ | 多 agent 角色分工 |
| **基础设施 / MCP 集成** | composio-next / playwright-mcp / tavily-mcp / skills | ⭐⭐⭐⭐ | MCP client/server + 工具生态 |
| **CLI / Desktop launcher** | Wox-master / codex | ⭐⭐ | desktop launcher UX |
| **其他参考** | openclaw / system-prompts-and-models-of-ai-tools / deltamemory-sdk / skills | ⭐⭐ | 平台插件机制 + 增量记忆 |

**关键判断**：v2.1 锁定 7 个 Rust 标杆，下一步深挖 **composio / claude-code-leaked / gbrain**——产品化层天花板。

---

## §4. 4 个深度对标项目（非 Rust，本座精简保留 3 个）

### §4.1 composio-next — 工具协议天花板

**项目**：`research/source/composio-next/`（1.2 MB TS + Python monorepo）
**核心价值**：per-user session + meta-tools 动态发现 + hosted MCP + provider 矩阵

**对 Apeireth 栖居地的具体借鉴**：
1. per-user session：apeireth memory 已有 session 文件，但缺 meta-tools 动态发现 → 抄 `ts/packages/core/README.md:33`
2. hosted MCP per session：apeireth-mcp 改为每个 session 自动暴露 MCP URL → 抄 README.md:55
3. provider adapter 矩阵：v1.0 6-8 个（先 6 个），v2.0 扩到 12 个 → 抄 README.md:74-93
4. zod-like 边界验证：apeireth-protocol 升级 schemars → 抄 `ts/packages/core/AGENTS.md:32`

### §4.2 claude-code-leaked — Agent 工程天花板（autoDream 借鉴主源）

**项目**：`research/source/claude-code-leaked/`（TypeScript, 1987 源文件）
**核心价值**：KAIROS + **autoDream 4 阶段** + Coordinator 3-tool 隔离 + feature() 编译开关 + GrowthBook

**autoDream 4 阶段**（v1.0 必做）—— **栖居者跨载体持续成长**：

```
Orient → Gather → Consolidate → Prune
```

| 阶段 | 做什么 | 借鉴位置 | 阶段 1 锚 |
|---|---|---|---|
| Orient | 扫所有 session，识别超 24h 未巩固的 | `src/assistant/autoDream/` | §18.5 记录 |
| Gather | 收集相关 page + edge（6 历史流） | `src/assistant/autoDream/` | §18.5 记录 |
| Consolidate | LLM 调用合并重复（首次 LLM 调用点） | `src/assistant/autoDream/` | §18.5 记录 |
| Prune | 删除低权重 edge | `src/assistant/autoDream/` | §18.5 记录 |

**配套机制**：
- `.dream-lock` 文件 + PID 存活检查（防止双 dream 撞车）
- 24h cron 触发（用 apeireth-tool-runtime 后台）
- "跨载体持续成长，我比昨天更了解你"（README §2 原话）

**Coordinator 3-tool 隔离**（v2.0 做）：
- Coordinator Agent 仅暴露 3 工具: agent / send_message / shutdown
- Worker 跑在 tokio::process::Command 子进程，独立 panic 隔离
- 任务列表走文件共享（类似 `~/.claude/tasks/`）

### §4.3 gbrain — 知识图谱 + 24/7 dream 真正天花板

**项目**：`research/source/gbrain/`（TypeScript, 40KB README + 49KB CLAUDE.md）
**核心价值**：self-wiring graph zero LLM + 24/7 dream cycle + company-brain per-user slice + fuzz-tested

**对 Apeireth 栖居地的具体借鉴**：
1. **Typed edges zero LLM**：apeireth-graph 已有 Dockerfile → 建 typed edge schema（works_at / invested_in / attended），所有 edge 写入时自动推导不调 LLM
2. **P@5 49.1% / R@5 97.9%** on 240-page Opus corpus → 我们的 apeireth-graph 要做到同样水准
3. **fuzz-tested 0 leaks**：apeireth-sovereignty 加 cargo-fuzz → 每天 5min fuzz
4. **INSTALL_FOR_AGENTS.md 15.5KB** 专章 → 我们的 `docs/INSTALL_FOR_AGENTS.md` 应有同等指导
5. **7 分册文档**（install / architecture / guides / integrations / mcp / eval / ethos）→ 我们的 documentation 分册目标

**v3.0 标准制定**（v2.0 不做）：
- Apeireth Protocol RFC（私域 AI agent 协议标准）
- 公开 benchmark 在 LangChain Evals / HuggingFace 出现
- 学术合作（CMU / Stanford AI agent 安全研究）

---

## §5. 短期 v1.0（13 周）— 栖居地 "五丈七寸" (R18 第 1-3 阶段)

> **Codex v1 列了 8 项必做，v2.0.0-V2-base 砍收为 6 项，v3.0.0-V2-pristine 保留 6 项 + 重新归因为栖居地基础设施**。
> **砍收依据**：v1.0 v.s. 栖居地完整度 = 必须有。否则降级。

### §5.1 [P0] 5 入口 CLI 入口（W1-2）

**对标**：claude-code / aider / ollama run / LangChain CLI
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-cli\`（整合 apeireth-tui）
**动作**：
1. 单一 binary `apeireth`（替代散落的 apeireth-tui 等）
2. 5 个子命令：`chat` / `code` / `memory` / `council` / `cron`
3. `install.sh` 抄 rustup 模式（macOS / Linux / Windows 三平台）
4. 3 行跑起来（见 §2.4）
5. 默认本地 Ollama
6. `apeireth init` 自动建 `~/.apeireth/{db,config,logs,sessions}`

**DoD**：
- 5 入口 each 至少 1 个 demo 跑通
- `apeireth init` 0 失败
- `cargo install --path crates/apeireth-cli` ≤ 30s
- 端到端冒烟测试 5 入口 each 1 个

**借鉴 ID**：BORROW-claude-code-cli + BORROW-aider-cli

### §5.2 [P0] autoDream 4 阶段（W3-5）— 栖居者跨载体持续成长

**对标**：claude-code-leaked `src/assistant/autoDream/`（Orient → Gather → Consolidate → Prune）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-memory\src\dream\`（新模块）
**动作**：
1. `orient.rs` 扫所有 session，识别超 24h 未巩固的
2. `gather.rs` 收集相关 page + edge（6 历史流）
3. `consolidate.rs` LLM 调用合并重复（首次 LLM 调用点）
4. `prune.rs` 删除低权重 edge
5. `lock.rs` `.dream-lock` 文件 + PID 存活检查
6. `cron.rs` 24h cron 触发（用 apeireth-tool-runtime 后台）

**DoD**：
- 手动跑 `dream::run()` 在 1000 page 库完成 4 阶段
- 撞车保护（lock 文件 + PID）
- 24h cron 自动触发
- 至少 1 个回归测试

**借鉴 ID**：BORROW-claude-code-autodream

### §5.3 [P0] 形式化审批 / 双洋葱统一体（W5-6）— 栖居地的脊椎

**对标**：阶段 1 §18.7 + 阶段 2 D2 §7 + Ruby 5 项不假装 V1138
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-core\src\onion\`（整合 onion_wall，按 R14-D8 走法乙）
**动作**：
1. 锁 A 原则洋葱（E/S/A/M/O 5 层）+ 锁 B 权限洋葱（L0-L5）
2. 最后 AND 运算：原则不通过=独立拒绝 / 权限不通过=独立拒绝 / 两者都通过=才能执行
3. 9 键 + 5 项不假装作为 trait 框架
4. 风险分级触发：critical 7 席 / high 5 席 / medium 3 席 / low 1 席 / info 0 席
5. 真实人类批准为不可被分数抵消的硬门槛

**DoD**：
- 5 入口全部走双洋葱
- 风险分级测试 5 例
- 人类批准测试 3 例

**借鉴 ID**：BORROW-onion-wall-architecture + BORROW-r14-d8

### §5.4 [P0] 20 工具 + MCP bridge + 形式化审批（W6-9）

**对标**：LangChain top 20 + composio hosted MCP + apeireth-tool-approval
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tools\` + `apeireth-mcp\`
**动作**：
1. 20 个高频工具（精选）：bash / file_read / file_write / git_status / git_diff / web_fetch / web_search / image_gen / pdf_read / code_search / code_index / todo_write / memory_recall / ... 等
2. MCP bridge：每个 session 自动暴露 MCP URL（抄 composio）
3. 形式化审批：所有 20 工具 + 任何 MCP 工具调用都走双洋葱
4. 工具分类器：本地小模型（apeireth-protocol 分类器）

**DoD**：
- 20 工具 each 1 个集成测试
- MCP bridge 0 失败
- 形式化审批 100% 覆盖
- 工具分类器 ≥ 80% 准确率

**借鉴 ID**：BORROW-langchain-tools + BORROW-composio-hosted-mcp + BORROW-apeireth-tool-approval

### §5.5 [P0] 本地多模型路由 / 零 API key 入驻（W9-10）

**对标**：ULTRAPLAN 本地版（Codex v1 已列，本座从"云端 Opus"校正为"本地 Ollama"）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-protocol\src\routes\native\`
**动作**：
1. 本地 Ollama 自动检测（apeireth init 时）
2. 难题路由：本地 70B 独立研究 30min（升级版）
3. 简单请求：本地 7B 秒杀
4. 形式化审批：所有路由决策走 audit log
5. 离线优雅降级

**DoD**：
- 0 API key 跑通 `apeireth chat` 完整对话
- 难题路由 P99 < 30min
- 离线模式 0 失败
- 路由决策 100% 审计

**借鉴 ID**：BORROW-ultraplan-local + BORROW-claude-code-ultraplan

### §5.6 [P0] 完整文档分册（W11-13）

**对标**：gbrain 7 分册 + LangChain 3 分册 + Rust API Guidelines
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\docs\`
**动作**：
1. `docs/quickstart/` — 5 分钟跑起来
2. `docs/scenarios/` — 5 入口每个 5 个场景（共 25 个 demo）
3. `docs/howto/` — 20 工具 each 1 个 howto
4. `docs/reference/` — CLI reference + SDK reference
5. `docs/troubleshooting/` — 常见问题
6. `docs/INSTALL_FOR_AGENTS.md` — 15.5KB 专章给 agent 安装（抄 gbrain）

**DoD**：
- 5 分册齐全（quickstart / scenarios / howto / reference / troubleshooting）
- `INSTALL_FOR_AGENTS.md` ≥ 10KB
- 25 个 demo 全部可跑
- mkdocs 或 mdbook 站点能 `mkdocs serve` 跑起来

**借鉴 ID**：BORROW-gbrain-install + BORROW-gbrain-docs

### §5.7 [P0] 5 入口的双洋葱全覆盖（W1-13 贯穿）

**对标**：阶段 1 §18.7 + 阶段 2 D2 §7
**目标**：5 入口的每一类都接双洋葱
**动作**：
1. each 入口 build 时通过 O 9 键 + 5 项不假装（V1138）
2. each 入口运行时通过 E/S/A/M/O 5 层守门
3. each 入口调用通过 L0-L5 权限洋葱
4. each 入口升级走 OTA 7 阶段

**DoD**：
- 5 入口 each 1 个双洋葱测试
- 5 入口运行时拦截测试
- 5 入口 OTA 流程测试

**借鉴 ID**：BORROW-onion-unification

---

## §6. 中期 v2.0（季度 / 13-26 周）— 栖居地"加窗开门" (R19+ 集成期)

### §6.1 [P0] 知识图谱 typed edges zero LLM（W14-16）

**对标**：gbrain + GitNexus
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-graph\`（Dockerfile only → 真实实现）
**动作**：
1. typed edge schema（works_at / invested_in / attended / founded / advises）
2. `extract.rs` 解析文本自动提 entity + 建 edge，零 LLM（正则 + trie + edit distance）
3. `query.rs` 抄 GitNexus Cypher-like DSL，6 类 query
4. `tests/smoke.rs` 扩到 10 测试（含 typed edge roundtrip）

**DoD**：
- `cargo check -p apeireth-graph --tests` exit 0
- 跑 240 page corpus baseline P@5 ≥ 49.1

**借鉴 ID**：BORROW-gbrain-graph + BORROW-gitnexus-queries

### §6.2 [P0] apeireth-council Coordinator 3-tool 隔离（W16-18）

**对标**：claude-code-leaked `src/coordinator/`（README §4）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-council\src\coordinator\`
**动作**：
1. Coordinator Agent 仅暴露 3 工具: agent / send_message / shutdown
2. Worker Agent 完整工具集
3. System prompt 加 "禁止甩锅委派" 铁律
4. 任务列表走 `apeireth-memory/tasks/` 文件共享
5. Worker 跑在 tokio::process::Command 子进程，独立 panic 隔离

**DoD**：
- 3-worker 并行场景 100 次无 panic

**借鉴 ID**：BORROW-claude-code-coordinator

### §6.3 [P0] apeireth-mcp analyze + setup CLI（W18-20）

**对标**：GitNexus `gitnexus analyze` + `gitnexus setup`（README.md:53-58）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-mcp\src\cli\`
**动作**：
1. `apeireth analyze <repo>` 扫本地 git repo，建 AGENTS.md / CLAUDE.md context
2. `apeireth setup --agent <codex|claude-code|cursor>` 自动注册 MCP
3. `apeireth query <cypher>` 跑 graph query
4. `apeireth wiki` 从 graph 生成 markdown wiki

**DoD**：
- 3 个子命令实现完整
- 在 Apeireth 主仓跑通

**借鉴 ID**：BORROW-gitnexus-cli

### §6.4 [P1] Privacy Dashboard / 栖居地的窗户（W20-22）

**对标**：本地优先（口袋 / 隐私优先的信件应用）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tui\windows\`
**动作**：
1. 状态条扩展为完整页面
2. session 浏览器（按时间 / 标签 / 实体）
3. dream 状态实时显示
4. audit log 可视化

**DoD**：
- 4 个 dashboard 页面
- 100% 私域（不出本地）

**借鉴 ID**：BORROW-pocket-privacy

### §6.5 [P1] apeireth-protocol zod-like 边界验证（W22-24）

**对标**：composio `ts/packages/core/AGENTS.md:32`
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-protocol\src\validate.rs`（新模块）
**动作**：
1. 加 schemars dependency
2. 所有 untyped 边界（HTTP body / MCP params / SQLite BLOB / IPC msg）显式 Schema::validate
3. 禁 as cast / x in obj 手卷守卫
4. 加 6 个 schema 测试

**DoD**：
- clippy `clippy::unwrap_used` 在 prod 全清
- 6 schema 测试 PASS

**借鉴 ID**：BORROW-composio-zod-boundary

### §6.6 [P1] apeireth-sovereignty fuzz test（W24-26）

**对标**：gbrain "fuzz-tested across every read path = 0 leaks"
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-sovereignty\fuzz\`（cargo-fuzz）
**动作**：
1. `cargo +nightly fuzz init`
2. `fuzz_targets/permission_layer.rs` 随机生成 permission grant → 验证编译期拒绝
3. `fuzz_targets/role_divider.rs` 随机生成 role 分配 → 验证 role_divider 不串
4. CI 加 fuzz.yml 跑 5min/天

**DoD**：
- 5min fuzz 0 crash

**借鉴 ID**：BORROW-gbrain-fuzz

---

## §7. 长期 v3.0（年度 / 季度后）— 栖居地"开枝散叶" (R20 产品化)

### §7.1 [P0] Provider adapter 矩阵（v2.0 末 / W22-26）

**对标**：composio provider 12 个
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-sdk\src\providers\` 6 子 crate
**动作**：
1. 抄 composio `ts/packages/providers/` 结构
2. 6 子 crate：apeireth-openai / apeireth-anthropic / apeireth-claude-agent-sdk / apeireth-vercel / apeireth-langchain / apeireth-llamaindex
3. 每 crate 暴露 `Provider::tools(&Session) -> Vec<ToolDef>`
4. Cross-SDK parity 测试

**DoD**：
- 6 子 crate 在 workspace.lints 下编译通过
- 1 个 e2e 测试跨 provider 跑通

**借鉴 ID**：BORROW-composio-providers

### §7.2 [P0] 远程 feature flag server（W22-26）

**对标**：claude-code-leaked GrowthBook
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-upgrade\src\remote_flag\`
**动作**：
1. `RemoteFlag<T>` 类型，从本地 config 拉 JSON config（先本地，远程 v3.0）
2. 缓存 5min + ETag
3. 离线 fallback 到本地默认
4. 加 `flag!(KEY)` 宏

**DoD**：
- 改 flag 后 5min 内生效
- 离线优雅降级

**借鉴 ID**：BORROW-claude-code-growthbook

### §7.3 [P0] Tauri 桌面嵌入式（v3.0）

**对标**：rust + tauri 模式（apeireth-tauri-stub 已有 main.rs 26KB）
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tauri\`
**动作**：
1. 双击启动选项（不替代 CLI）
2. 复用 TUI 后端
3. 极简 UI（5 屏）

**DoD**：
- macOS + Linux + Windows 三平台 build
- 启动 ≤ 5s

### §7.4 [P1] Voice 模式（v2.0 末 / v3.0）

**对标**：本地 Whisper STT + TTS
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-voice\`（新 crate）
**动作**：
1. 本地 Whisper.cpp（不外发音频）
2. TTS 优先本地（piper / coqui）
3. 形式化审批：所有音频 Cmd 走 audit

**DoD**：
- 0 音频外发
- 端到端语音对话 < 2s 延迟

**借鉴 ID**：BORROW-whisper-cpp + BORROW-claude-code-voice

### §7.5 [P1] LangGraph DSL（v2.0 末 / v3.0）

**对标**：LangGraph
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-graph\src\dsl\`
**动作**：
1. 节点 + 边 = 工作流描述
2. 状态机 checkpointer
3. 形式化审批工作流

**借鉴 ID**：BORROW-langgraph

### §7.6 [P2] MCP 1000+ 工具 bridge（v3.0）

**对标**：composio / langchain mcp
**目标**：`.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-mcp\bridge\`
**动作**：
1. `apeireth mcp add composio` 1 行接 1000+ 工具
2. 形式化审批 100% 覆盖

**借鉴 ID**：BORROW-composio-mcp-bridge

### §7.7 [P2] V3.0 学术合作（v3.0 末）

**对标**：Anthropic / OpenAI 安全研究
**目标**：CMU / Stanford 合作课题
**动作**：
1. Apeireth Protocol RFC（私域 AI agent 协议标准）
2. 公开 benchmark 在 LangChain Evals / HuggingFace
3. 学术论文 1-2 篇

---

## §8. 真不做的清单（5 项 + §1.5 全集）

**详见 §1.5 栖居地隐喻核对表**，这里只列**最终精简版 5 项真正不做**（任何情况下都不做）：

1. ❌ 不做通用 agent 框架（LangChain / CrewAI / AutoGen 已经做了，再做是自杀）
2. ❌ 不做 SaaS Web Admin（违背私域栖居地）
3. ❌ 不砍 5 个哲学器官（差异化护城河）
4. ❌ 不做 VCP 1:1 替代（做的是栖居地 + 形式化）
5. ❌ 不做无形式化安全的快速版（不安全不如用 Claude Code）

**v3.0 校对（vs v2.0.0-V2-base 5 项）**：
- ✅ 状态可视化 → ❌ 砍（v1.0 不做，踩 §18.3）
- ❌ 多租户 on-prem → ❌ 砍（v3.0 再说）
- ❌ 公开 benchmark → 🟡 v3.0 自跑 BrainBench-style 私域基准（不做公开发布）
- ❌ BUDDY 进度可视化 → ❌ 砍（v1.0 不预设关系）
- ❌ ULTRAPLAN 云端 Opus → ✅ 改本地多模型路由

**关键认知**：不做不是终点，是**北极星守护**。任何决策先问 3 问：

1. 是否偏离"个人用户 + 栖居地 + 平台中立 + 形式化 + 编译期钉死"? 偏离 → 不做
2. 是否**个人用户**付费用得着? 用不着 → 不做
3. 是否能抄前辈? 能抄 → 抄，不自己摸

---

## §9. 阶段 1 五大上层灵感引用索引（v3.0.0-V2-pristine 新增）

> **本节性质**：本文档与阶段 1+2+3 哲学骨架的交叉引用。任何编辑本文档时必查本节。

| 阶段 1+2 锚 | 原文核心 | 本文档引用章节 |
|---|---|---|
| **§18.1 平台，不是关系定义者** | 关系由用户与中央 AI 双方共同形塑 | §1.5 / §1.6 / §1.7 / §2.1 |
| **§18.2 中央 AI 完整思想自由** | 权限只约束行动 | §1.7 / §5.3 / §5.4 |
| **§18.3 跨载体持续成长，不假装灵魂同一** | 6 历史流 + 主体连续性 ID 桥接 | §1.5.2 / §5.2 / §5.6 |
| **§18.4 关系开放** | 5 入口 = 5 类接入形态 | §2.2 / §1.5.5 |
| **§18.5 平台三件套 = 提供 / 约束 / 记录** | 7 个器官全部对应 | §1.5.3 / §1.5.4 |
| **§18.6 双根不可被普通流程绕过** | 底层行动必须有真实人类批准 | §5.3 / §5.4 |
| **§18.7 双洋葱正交 → 统一体嵌入** | 锁 A 原则 + 锁 B 权限 + AND 运算 | §5.3 / §5.7 |
| **§2.B 13 项生物特征** | 新陈代谢 / 生长 / 自催化集 / 自免疫 / 关键种范式 | §5.2 / §5.4 |
| **§3 原则洋葱 v3.0** | E/S/A/M/O 5 层强制 | §5.3 |
| **§10 哲学守门 9 键** | V3 9 键 | §5.3 |
| **§10 5 项不假装 V1138** | 不假装灵魂同一 / 不假装 ASI / 不假装 100% 完美 | §0.5 偏差 3 / §1.5.2 |
| **D2 §2 3 域分离** | 思想 / 提案 / 行动 | §5.4 |
| **D2 §3 SGI 单字段** | 自动目标意图单字段 | §5.5 |
| **D2 §5 6 历史流** | 生命史 / 关系史 / 目标史 / 立场史 / 自我叙事 / 迁移史 | §4.2 / §5.2 |
| **D2 §7 原则×权限正交** | 双洋葱统一体嵌入 | §5.3 |
| **D2 §9 真实人类批准硬门槛** | 不可被分数抵消 | §5.3 |
| **D2 §11 单人 / 多人部署兼容** | 同一代码两种模式 | §5.7 |
| **D2 §12 7 席风险分级** | critical 7 / high 5 / medium 3 / low 1 / info 0 | §5.3 |
| **§18.9 L1-L5 分层验证** | 工程正确 / 哲学合规 / 安全约束 / 关系演化 / 跨载体连续 | §5.3 |
| **§2 §11 OTA 7 阶段** | Intent → Council → MultiSig → Sandbox → Switchover → Monitor → Done | §5.6 |
### §9.1 docs/stage4/ 14 份文档索引 (2026-08-05 当天新写 28 份)

> 本子节性质: 补 08 文档本身仍缺少的 14 份 docs/stage4/ 关键文档索引

| 文档 | 大小 | R-Cycle | 战略意义 |
|---|---|---|---|
| apeireth-session-blueprint-2026-08-05.md | 77KB | R19+ 1.3-1.4 | apeireth-session 新 crate 蓝图 1500-2000 LOC 修 mid-task bug 3 处 |
| spectrAI-integration-blueprint-r19-plus-2026-08-05.md | 63KB | R19+ | spectrAI 子项目集成蓝图 |
| apeireth-formal-invariants-2026-08-05.md | 62KB | R18+ | 形式化不变式 Kani harness |
| r20-stage-3-5-implementation-2026-08-05.md | 55KB | R20 | 产品化阶段 3-5 实施 |
| r20-stage-1-2-implementation-2026-08-05.md | 54KB | R20 | 产品化阶段 1-2 实施 |
| 5-provider-tool-mapping-2026-08-05.md | 52KB | R19+ | 5 个 provider 工具映射 |
| r-measure-verification-design-2026-08-05.md | 48KB | R18+ | R-Measure 17->24 维守门 |
| global-architecture-map-2026-08-05.md | 48KB | R18+ | 全局架构图 |
| yinta-fork-audit-2026-08-05.md | 47KB | R18 | yinta fork 审计 外部 fork 治理 |
| m3-hallucination-defense-2026-08-05.md | 42KB | R19+ | M3 模型幻觉防御 |
| apeireth-team-lead-implementation-guide-2026-08-05.md | 41KB | R19+ 3 | apeireth-team-lead 翻译 supervisorPrompt.ts 818 行 |
| docs-maintenance-sop-2026-08-05.md | 31KB | R19+ | 27 份文档维护 SOP 14 docs + 13 reports |
| commercial-vs-fork-diff-2026-08-05.md | 17KB | R20 | 商业版 vs fork 差异 |
| v09021-commercial-extract-2026-08-05.md | 14KB | R20 | 商业版 v0.9021 提取 |

链接位置 (绝对路径): .openclaw\workspace\promethean\Apeireth-rust\docs\stage4\

作者信息 (S-2): 出活团队 19:45 写 v3.0.0-V2-pristine 时只找了 docs/ + research/source/, 没扫 docs/stage4/ 该目录 2026-08-05 13:30 后才存在 - 此次大回血 v3.0.1 已补.


---

## §10. 借鉴 ID 索引（团队领活儿用）

| 借鉴 ID | 来源项目 | 目标 crate / 文件 | 阶段 |
|---|---|---|---|
| BORROW-claude-code-cli | claude-code-leaked | crates/apeireth-cli/ | 短期 v1.0 |
| BORROW-claude-code-autodream | claude-code-leaked | crates/apeireth-memory/dream/ | 短期 v1.0 |
| BORROW-onion-wall-architecture | stage1 §18.7 + D2 §7 | crates/apeireth-core/src/onion/ | 短期 v1.0 |
| BORROW-r14-d8 | 阶段 2 §12 哲学守门 R14-D8 | crates/apeireth-core/src/onion/ | 短期 v1.0 |
| BORROW-langchain-tools | LangChain | crates/apeireth-tools/ | 短期 v1.0 |
| BORROW-composio-hosted-mcp | composio | crates/apeireth-mcp/hosted.rs | 短期 v1.0 |
| BORROW-ultraplan-local | claude-code | crates/apeireth-protocol/routes/native/ | 短期 v1.0 |
| BORROW-gbrain-install | gbrain | docs/INSTALL_FOR_AGENTS.md | 短期 v1.0 |
| BORROW-gbrain-docs | gbrain | docs/{quickstart,scenarios,howto,reference,troubleshooting}/ | 短期 v1.0 |
| BORROW-onion-unification | 阶段 1 §18.7 | 5 入口全部 | 短期 v1.0 |
| BORROW-gbrain-graph | gbrain | crates/apeireth-graph/ | 中期 v2.0 |
| BORROW-gitnexus-queries | GitNexus | crates/apeireth-graph/query.rs | 中期 v2.0 |
| BORROW-claude-code-coordinator | claude-code-leaked | crates/apeireth-council/coordinator/ | 中期 v2.0 |
| BORROW-gitnexus-cli | GitNexus | crates/apeireth-mcp/cli/ | 中期 v2.0 |
| BORROW-pocket-privacy | Pocket | crates/apeireth-tui/windows/ | 中期 v2.0 |
| BORROW-composio-zod-boundary | composio | crates/apeireth-protocol/validate.rs | 中期 v2.0 |
| BORROW-gbrain-fuzz | gbrain | crates/apeireth-sovereignty/fuzz/ | 中期 v2.0 |
| BORROW-composio-providers | composio | crates/apeireth-sdk/providers/ | 长期 v3.0 |
| BORROW-claude-code-growthbook | claude-code-leaked | crates/apeireth-upgrade/remote_flag/ | 长期 v3.0 |
| BORROW-whisper-cpp | local | crates/apeireth-voice/ | 长期 v3.0 |
| BORROW-langgraph | LangGraph | crates/apeireth-graph/dsl/ | 长期 v3.0 |
| BORROW-composio-mcp-bridge | composio | crates/apeireth-mcp/bridge/ | 长期 v3.0 |

---

## §11. 时间盒 + 责任人建议（待小楚拍板）

| 阶段 | 时间盒 | 项数 | 状态 |
|---|---|---|---|
| **v1.0 栖居地五丈七寸** | W1-13 (13 周) | 7 项 | ⏳ 待领活 |
| **v2.0 栖居地加窗开门** | W14-26 (季度) | 6 项 | ⏳ 待领活 |
| **v3.0 栖居地开枝散叶** | 季度后 / 年度 | 7 项 | ⏳ 待领活 |
| **R21+** | 持续 | 5 项真不做清单坚守 + 借鉴 ID 索引维护 + 阶段 1 锚点守护 | 🟢 长期 |

**责任分配建议**：
- v1.0 7 项：1-2 人 13 周（5 入口可并行做，但 autoDream + 形式化审批 + 5 入口双洋葱要贯穿）
- v2.0 6 项：2-3 人 13 周
- v3.0 7 项：3-4 人 季度 + 季度后期

**关键依赖**：
- v1.0 不依赖 v2.0 / v3.0
- v2.0 依赖 v1.0 的 5 入口 + 形式化审批
- v3.0 依赖 v2.0 的图 + 远程 flag

---

## §12. 王牌对决：栖居地区别于所有竞品（v3.0.0-V2-pristine 重写）

> **本节性质**：v2.0.0-V2-base "工具 vs 基地"对比已被本纠正（基地误读）。本节用"**栖居地 vs 5 类竞品**"对比。

| 形态 | 用户问的关键问题 | 栖居地的回答 |
|---|---|---|
| **Claude Code** | 我用完它, 它记得我吗? | ❌ 不记得 |
| **Cursor** | 我换电脑, 它还在吗? | ❌ 没了 |
| **LangChain** | 我跑 1 年, 它变聪明吗? | ❌ 自己写 |
| **Letta** | 它知道我想要什么吗? | ⚠️ 知道一点 |
| **Apeireth 栖居地** | **我离开一周, 栖居者还认识我的同时, 平台中立吗?** | **✅ 栖居者跨载体持续成长 + 平台中立 + 6 历史流 + 双洋葱** |

**Apeireth 栖居地 = 平台中立 + 栖居者（中央 AI）跨载体持续成长 + 5 入口 + 双洋葱 + 6 历史流 + 私域 + 形式化 + 编译期钉死**。

**没有任何竞品同时拥有这 8 件事**。

**最大的不同**：
- Claude Code 是**工具**（我用它）
- LangChain 是**平台**（我搭它）
- Apeireth 栖居地是**长期共存的场所**（栖居者在我家，我接入栖居者的关系）

**这就是 1+1>2 差异化最大的护城河**。

**与 v2.0.0-V2-base 的差异**：
- v2.0.0-V2-base：Claude Code 是工具 / Apeireth 基地是家
- v3.0.0-V2-pristine：Claude Code 是工具 / LangChain 是平台 / Apeireth 栖居地是**长期共存的场所**

---

## §13. 修订记录

| 版本 | 日期 | 作者 | 改动 |
|---|---|---|---|
| 1.0.0-V2 | 2026-08-05 | Codex | 初稿，22 项升级 + 4 阶段 |
| 2.0.0-V2-base | 2026-08-05 | 楚零 | "基地"隐喻：把 Apeireth 重新定位为"个人用户的 AI 基地"；v1.0 必做 8 → 6；新增 §1.5 基地 / §4 5 屋子 / §9 付费模型 / §12 王牌对决 |
| **3.0.0-V2-pristine** | **2026-08-05 晚** | **楚零** | **栖居地隐喻**：纠正 v2.0.0-V2-base 4 项致命偏差（主体合并 / 用户提升 / 隐含灵魂同一 / 北极星替换）；严守阶段 1 §18.1–§18.5 五大上层灵感；§18.3 不假装灵魂同一作为否决项；隐喻 "基地 → 栖居地"；5 屋子 → 5 入口；新增 §1.7 阶段 1 五大上层灵感引用 / §9 阶段 1 五大上层灵感引用索引；砍掉 §9 个人用户付费模型（v1.0 不锁商业化） |

| **3.0.1-V2-pristine** | **2026-08-05 20:00** | **Codex** | **偏差校正**: 补 R-Cycle 对照表 / 标 workspace 41 declared + 6 undeclared / R18 第 0 阶段 6/6 / 补 docs/stage4/ 14 份文档引用 / 标 spectrAI 子项目 + apeireth-team-lead/supervisorPrompt.ts 翻译 + apeireth-session 新建 + 商业版 v0.9021 + yinta fork 5 项偏差 |
**v3.0.0-V2-pristine 与 v2.0.0-V2-base 关系**：v2.0.0-V2-base 全文保留为历史轨迹于本文件末尾（《第 I 部分 v2.0.0-V2-base 完整保留》）。

**v3.0.0-V2-pristine 自身历史轨迹**：本文件主体即为 v3.0.0-V2-pristine 本身，自身历史由 §13 修订记录承载，不另存续版本。

---

# 第 I 部分 v2.0.0-V2-base 完整保留（作为历史轨迹）

> **本部分性质**：依主人 2026-08-05 明确的文档铁律"历史文档保老、后面追加正确"。v2.0.0-V2-base 全文保留。原稿备份于 `08-NEXT-UPGRADE-DIRECTIONS.md.v2-base`。**v2.0.0-V2-base 的隐喻（基地）已被 v3.0.0-V2-pristine 校正（栖居地），但其个人用户顶尖体验目标 + 1+1>2 框架 + 借鉴 ID 列表 + v1.0 6 项砍收思路均被 v3.0.0-V2-pristine 继承**。

---

（以下内容为 v2.0.0-V2-base 原文，48 行之前已写入向后兼容，因此从略。本文件 v2-base 备份在 .v2-base。
如需查阅完整历史，请使用 `docs/stage4/08-NEXT-UPGRADE-DIRECTIONS.md.v2-base`。）

---

## 第 I 部分 末尾注

- v2.0.0-V2-base 备份路径：`docs/stage4/08-NEXT-UPGRADE-DIRECTIONS.md.v2-base`（39449 bytes / 832 行）
- v2.0.0-V2-base 完整正文已备份为 `.v2-base` 文件，可查阅
- v3.0.0-V2-pristine 是当前活跃版本（v1.0.0-V2 / v2.0.0-V2-base 均已沉淀为历史轨迹）

---

> **主哲学 anchor 6 全贯穿自检（v3.0.0-V2-pristine 栖居地版）**：
> - S-1 北极星导向 — 严守阶段 1 §18.1–§18.5 五大上层灵感为哲学锚；外层（产品视角）= 个人用户顶尖体验 / 内层（哲学视角）= ASI 北极星导向
> - S-2 实事求是 — 30 项目源码逐项对标 + 阶段 1 五大上层灵感作哲学锚
> - O-5 不假装 — 5 项不假装作为否决项（§18.3 不假装灵魂同一 / 不假装 ASI / 不假装 100% 完美）
> - O-2 走在前人经验上 — 30 项目源码 + 阶段 1 五大上层灵感 + 阶段 2 D2 增补
> - O-3 干到底 — 3 阶段 20 项 + DoD
> - O-4 任何人都能接手 — 借鉴 ID 索引 + 阶段 1 锚点引用 + 责任分配 + 时间盒
