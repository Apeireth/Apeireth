[Document-Meta]
Document:        08-MULTIPARADIGM-FRONTEND-AND-SECURITY-VISION.md
Version:         2.0.0-V2-DRAFT
R-Cycle:         v2-strategy
Last-Modified:   2026-08-14
Status:          🟡 DRAFT (基于前辈推荐 7 项目 + 项目现状综合)
Author:          Codex (策略分析) + 主人修正
Source-of-Truth: 决策 #30~#56 决策链 + `Desktop\重要参考项目，产品方向前辈.txt` (7 链接)
0 主动 commit:   严守 (写到主仓但不 commit)
0 装 PASS 严守:  严守 (本文件为综合愿景, 不是已实施)

---

# Apeireth v2.0 — 多前端范式 × 安全场景 综合升级方向文档

> **核心定位 (一句话)**:
> **Apeireth 不再只是 "AGI 操作系统 / Rust 重写 VCP 全栈", 而是升级为 "可陪伴的、可信赖的、AI 代理平台" — 插件化后端撑起多前端, 核心场景是 AI 安全。**

---

## 0. TL;DR

**项目愿景升级**:
- **旧定位**: AGI 操作系统 (Rust 重写 VCP 全栈, 对标 5 战区)
- **新定位**: **可陪伴的、可信赖的、AI 代理平台** — 三组维度合一:
  1. **多前端范式** (TUI + Tauri 桌面 + Live2D 桌宠)
  2. **插件化后端** (Cordis 思路 Rust 实现, 借鉴 deepseek-harness)
  3. **核心场景: AI 安全** (Agent + MCP + Skill 编排, 借鉴 VulnClaw/DeepSec)

**三个不重复的关键拼图 (后端"撑得起"的关键)**:
- **A. 插件机制**: TUI/Tauri/Live2D/Web 都是 "消费者", 通过统一 ACP 调后端, 不需要为每个前端写适配
- **B. Skill 引擎**: VulnClaw/DeepSec 成功的核心 = Skill 编排; 你有 `apeireth-skills` 但缺 "模板引擎" + "权限边界"
- **C. Privacy 护栏**: 借鉴 opencode-vibeguard, 所有出站 LLM 请求先替换敏感字符串, 响应回来再反向替换; 应该是后端可插拔, 不是前端责任

**9 organ 重新定位**: 之前是 "9 个技术模块", 现在是 **9 个人设维度** — 桌宠让 organ 有灵魂, 安全让 organ 有职责, 插件让 organ 可扩展。

**v1.5 (中期) 新增项 (vs 现有 ROADMAP)**:
- Tauri 2 终极前端 prototype (不是 stub, 是真做)
- Live2D 桌宠前端 (v1.5 标志特性, 扩大用户群体的关键)
- `apeireth-guard` 隐私护栏插件 (借鉴 opencode-vibeguard)
- `apeireth-skill-runtime` Skill 引擎 (参考 VulnClaw Skill 编排)

**v2.0 (长期) 标志**:
- **Plugin Marketplace** (Cordis 思路 Rust 实现)
- **AI 安全场景套件** (渗透测试 / 代码审计 / 漏洞扫描 / 合规检查)
- **桌宠角色市场** (用户可上传 Live2D / VRM 模型)

---

## 1. 背景: 前辈的 7 个链接不是 7 个项目, 是 3 组维度

**来源**: `Desktop\重要参考项目，产品方向前辈.txt`

```
1. https://project-neko.cn
2. https://github.com/Project-N-E-K-O/N.E.K.O        ← 桌宠方向
3. https://github.com/MIO-456/Lumi_Nox               ← 桌宠方向
4. https://github.com/deepseek-ai/deepseek-harness/  ← 插件化后端范式
5. https://github.com/inkdust2021/opencode-vibeguard ← 隐私护栏插件
6. https://github.com/Netw0rkNoob/VulnClaw          ← AI 安全场景 (实际作者 Unclecheng-li)
7. https://github.com/Unclecheng-li/DeepSec         ← AI 安全场景
```

### 1.1 三组维度拆解

| 维度 | 项目 | 关键启示 |
|---|---|---|
| **前端形态: 桌宠** | NEKO + Lumi_Nox | 透明 Live2D 窗口悬浮桌面; 模型层 / 消息流层解耦; 9 organ 人格化呈现 |
| **后端范式: 插件化** | deepseek-harness + opencode-vibeguard | Cordis 插件元框架 (沙箱 Python 进程 + JSON-RPC, 被称 "Agent 界的 Android"); Privacy 护栏插件 |
| **核心场景: AI 安全** | VulnClaw + DeepSec | LLM + MCP + Skill 编排 = 自动化 "信息收集 → 漏洞发现 → 利用 → 报告"; Unclecheng-li 是 "AI 安全研究员 + jk 美少女" |

### 1.2 单看任何一组都不完整

- **只做桌宠** → 失去严肃用户 (开发者 / 企业 / 安全人员)
- **只做安全** → 失去泛用户 (年轻人 / 陪伴需求)
- **只做插件化** → 失去落地形态 (没人用裸后端)
- **三者合一** → **真正扩大用户群体**, 且共用同一后端 = 最大化工程红利

### 1.3 同源性观察 (值得留意)

- NEKO 社区 + VulnClaw 社区在 AI Agent 圈子有重叠
- "桌宠 + 安全" 组合在 AI Agent 圈子是有先例的方向
- Unclecheng-li 同时做 VulnClaw + DeepSec (安全深度), 又自称 jk 美少女 (情感化倾向) — **前辈用同一作者的两个项目告诉你: 安全和情感不是冲突, 是同一人设**

---

## 2. 项目现状摘要 (精炼版, 不重复顶层 ROADMAP)

**完整 ROADMAP 见**: `ROADMAP.md` (顶层, 1.0/1.1/1.5/2.0 主时间线)
**详细借脑见**: `docs/r149/`, `docs/r150/`, `docs/r153/` 等

### 2.1 后端支柱 (v1.0 已发布, master `abf12243`)

| 类别 | 关键 crate | 用途 |
|---|---|---|
| 核心 | `apeireth-core` (105KB), `apeireth-bus`, `apeireth-runtime` (7 模块编排) | L0 HA + 总线 + 运行时 |
| AI 抽象 | `apeireth-provider` (5→1 合并), `apeireth-asi` (24 维), `apeireth-acp` | LLM 网关 + 测量 + 通信协议 |
| 决策/记忆 | `apeireth-council` (7 advisor), `apeireth-memory` (LightMemo+DailyNote+SQLite) | 多视角决策 + 持久化 |
| 安全 | `apeireth-sovereignty` (274KB + Hyperlight 微 VM), `apeireth-arbitration` (HASH-SQL) | 隔离 + 不可篡改审计 |
| 管线 | `apeireth-pipeline-g5` (5 阶段通用), `apeireth-pipeline` (chat 专用) | 可插拔管线 |
| 工具 | `apeireth-tool-registry/runtime/search/shell/fetch/browser/filesystem/codesearch/image-gen/image-process/approval` | 11 件套 |
| 协议 | `apeireth-protocol` (139KB, 4 协议 facade), `apeireth-mcp`, `apeireth-vector` (qdrant), `apeireth-relation` (SurrealDB) | 协议归一化 |
| 工作流 | `apeireth-workflow` (Temporal 模式, R152 NEW) | Workflow + Activity + EventHistory |
| 插件 | `apeireth-extension`, `apeireth-skills` | 扩展 + Skill 基础 |
| 借鉴 | clap / hyper / servers(MCP) / PyO3 / kani / langgraph / superpowers (8/11 ✅) | 借鉴已实施 |

**76 active crate 总数**, 5618+ tests pass (R167 数据)

### 2.2 前端现状

| 前端 | 状态 | 关键文件 | 备注 |
|---|---|---|---|
| **TUI** | 🟢 当前 dev 主线 | `crates/apeireth-tui/` (254KB) | ratatui 0.30 + crossterm; 9 organ dashboard; 5 页面架构 |
| **Web** | 🟡 已实现但未主推 | `crates/apeireth-web/` (135KB) | Leptos 0.7 SSR + WASM hydration; 已接 Council/Sovereignty/ASI/Memory |
| **Tauri** | 🔴 仅 stub, 已冻结 | `frontend/tauri-prototype/` + `crates/_frozen/apeireth-tauri-stub` | R17 stub 从未实船; R19 战役计划用真前端; R145 冻结 |
| **Live2D 桌宠** | ⚪ 不存在 | — | **本文件建议 v1.5 新增** |

### 2.3 后端对三组维度的现有支撑

**桌宠维度**: ⚠️ 缺
- 有 9 organ 设计, 但没有"渲染层接入点" (TUI 用 ratatui; Web 用 Leptos; 桌宠缺)
- 有 `apeireth-acp` 协议, 但未确认对"渲染层消费者"足够友好

**插件化维度**: ⚠️ 半成品
- 有 `apeireth-extension`, 但未明确"外部开发者扩展路径"
- 有 `apeireth-skills`, 但缺"Skill 模板引擎" + "Skill 权限边界"

**AI 安全维度**: 🟢 后端能力已就位, 缺场景串接
- 隔离: `apeireth-sovereignty` + Hyperlight 微 VM
- 决策: `apeireth-council` 7 advisor (可解释)
- 审计: `apeireth-arbitration` HASH-SQL (不可篡改)
- 工具白名单: `apeireth-tool-approval`
- 管线: `apeireth-pipeline-g5` 5 阶段 (天然安全检查模型)
- 评分: `apeireth-asi` 24 维 (可量化)
- 重放: `apeireth-workflow` Temporal 模式

**结论**: 安全后端能力**已经够强**, 缺的是 **Skill 编排层** + **场景模板** 就能对标 VulnClaw 且远超。

---

## 3. 愿景升级 (愿景层 vs 工程层)

### 3.1 旧愿景 (per `docs/v2-strategy/00-VISION.md` v2)

> Apeireth = VCP 的全栈 Rust 重写 + 双洋葱 + Self-Disable + 形式化安全
> 5 战区同时打 (战区 1 Coding Agent + 战区 2 LLM 网关 + 战区 3 Multi-Agent + 战区 4 长期记忆 + 战区 5 工具协议)
> 战区 6 (UI/Web Admin) 交给其他团队

### 3.2 新愿景 (本文件升级)

> **Apeireth = 可陪伴的、可信赖的、AI 代理平台**
>
> **可陪伴** = 多种前端形态满足不同陪伴场景 (TUI 给开发者 / Tauri 给工具党 / Live2D 桌宠给泛用户)
> **可信赖** = Sovereignty 隔离 + Council 解释 + Arbitration 审计 + Guard 隐私护栏, 让用户敢把工作交给它
> **AI 代理平台** = 插件化后端 (Cordis 思路) + Skill 生态 + 安全场景套件, 让用户能扩展它

### 3.3 战略三轴

```
        情感轴 (桌宠 + UI 体验)
            ↑
            │
            │ • 9 organ 人格化
            │ • Live2D/VRM 模型
            │ • 永远顶层 + 消息气泡
            │
 严肃轴 ←──┼──→ 实用轴
 (安全)    │   (插件化)
            │
            │ • Tauri 桌面软件
            │ • TUI 终端
            │ • 多 Provider / 多前端协议
            │
            ↓
        扩展轴
```

**Apeireth 的独特之处**: 三轴平衡, 不偏废任何一端。VulnClaw 偏严肃轴; NEKO 偏情感轴; deepseek-harness 偏扩展轴; **Apeireth 要做三轴交叉的"全能代理平台"**。

---

## 4. 三前端范式路线

### 4.1 TUI (现状 = 当前 dev 主线)

**状态**: 🟢 已有, 持续优化
**目标用户**: 开发者 / SSH 远程 / 运维
**价值**: 完整功能密度, 最高可访问性
**关键文件**: `crates/apeireth-tui/`
**参考路线**: `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md`
**v1.5 计划**:
- 接入新 runtime (主人决策点)
- 真 MiniMax API worker (替 SimulatedWorker)
- 9 organ dashboard 增强 (从状态展示到可视化分析)
- i18n 完整化

### 4.2 Tauri 桌面软件 (现状 = stub → 需实做)

**状态**: 🔴 stub 冻结, 需解冻 + 真做
**目标用户**: 中度 GUI 用户 / 习惯桌面软件的人 / 企业内网
**价值**: 完整 GUI 体验, 本地优先, 跨平台 (Win/Mac/Linux)
**关键路径**:
- `frontend/tauri-prototype/` (现有 stub)
- `crates/_frozen/apeireth-tauri-stub` (需解冻并迁移)
**参考**: `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` §🎯 战略锚定 (主人原话: "我们最后要做的前端应该是 Tauri")
**技术栈建议**:
- Tauri 2 + 原生 WebView (Rust 嵌入式)
- 前端框架: 复用 Leptos 或换 SolidJS (按审美 + 性能选)
- 后端调用: HTTP to `apeireth-api` (瘦客户端模式, 借鉴 TUI 经验)
**v1.5 计划**:
- 解冻 `apeireth-tauri-stub` → 真 Tauri 2 工程
- 抄 TUI 的 HTTP 集成模式 (瘦客户端硬约束)
- 9 organ 桌面 widget (面板 / 雷达图 / 流式输出)
- 工具白名单 + 审批 UI

### 4.3 Live2D 桌宠 (现状 = 不存在 → v1.5 标志新增)

**状态**: ⚪ 不存在, v1.5 标志特性
**目标用户**: 泛用户 / 年轻人 / 陪伴需求 / 不懂技术的人
**价值**: **情感化入口, 破圈关键** (从开发者圈扩展到大众)
**关键路径**: 新建 `crates/apeireth-desktop-pet/` 或 `frontend/pet/` + `crates/apeireth-pet-runtime/`
**技术栈两条路 (待主人拍板)**:
- **路线 A — Live2D**: NEKO / Lumi_Nox 风格; 模型生态成熟; 但 **Cubism SDK 商业授权** (年费 / 单项目授权)
- **路线 B — VRM**: 开源 (MIT); Three.js / babylon.js; 社区模型多 (VTube Studio / RiBLA Broadcast); 更现代
**两条路对后端接口影响**:
- 共同点: 都通过 `apeireth-acp` 调后端
- 差异: Live2D 需要 Cubism SDK JS binding; VRM 需要 GLB loader + 骨骼动画 mapping
**关键设计 (来自 NEKO + Lumi_Nox 综合)**:
1. **透明窗口 + 永远顶层** (`window.set_always_on_top(true)` + 透明背景)
2. **模型层 / 消息流层解耦**: L2D/VRM 只渲染; 后端只管消息流
3. **9 organ 人格化映射**: brain/mind/voice/body/... → 角色的表情/姿态/回应速度
4. **唤起方式**: 可设为"角落陪伴"或"消息气泡主动弹出"
5. **多角色切换**: 模型库 = 人设库; 不同性格接同一后端

### 4.4 三前端共享后端的关键架构

```
┌─────────────────────────────────────────────────┐
│   TUI / Tauri / Live2D / Web / CLI (前端消费者)  │
│   每个前端 = 一个独立"插件消费者", 不互相耦合     │
└─────────────────────────────────────────────────┘
                        │
                        ▼ HTTP/JSON-RPC/WebSocket
              ┌──────────────────────┐
              │   apeireth-acp       │  ← 统一协议 facade
              │   (Agent Comm Proto) │
              └──────────────────────┘
                        │
        ┌───────────┬───┴───┬─────────────┐
        ▼           ▼       ▼             ▼
   runtime     pipeline-g5 tools        memory
        │           │       │             │
        └───────── sovereignty ──────────┘
              (隔离 + 审计 + 解释)
```

**核心约束 (抄 TUI 经验)**:
- 前端**不 import `apeireth_api::*` 等后端 lib** (瘦客户端硬约束)
- 前端只调 HTTP/JSON-RPC/WebSocket
- 新增前端 = 写一个新 "消费者", 不动后端

---

## 5. 安全维度的具体落地

### 5.1 对标 VulnClaw (现状 vs 差距)

**VulnClaw 做到的事**:
- LLM + MCP 工具链 + Skill 参考资料 + 自然语言输入
- 自动 "信息收集 → 漏洞发现 → 利用 → 报告生成"
- 兼容 OpenAI / Anthropic / MiniMax / DeepSeek

**Apeireth 已有 (VulnClaw 没有)**:
- ✅ 多 Provider (`apeireth-provider` 已合并 5→1)
- ✅ MCP (`apeireth-mcp`)
- ✅ 工具白名单 (`apeireth-tool-approval`)
- ✅ 决策解释 (`apeireth-council` 7 advisor)
- ✅ 隔离 (`apeireth-sovereignty` + Hyperlight 微 VM)
- ✅ 审计 (`apeireth-arbitration` HASH-SQL)
- ✅ 工作流重放 (`apeireth-workflow` Temporal)

**Apeireth 缺 (v1.5 要补)**:
- ❌ **Skill 引擎** (`apeireth-skills` 有, 但缺模板引擎 + 权限边界)
- ❌ **隐私护栏** (`apeireth-guard` 不存在)
- ❌ **安全场景模板** (渗透测试 / 代码审计 / 漏洞扫描 / 合规检查 模板)

### 5.2 9 organ 人格化矩阵 (核心洞察)

| Organ | 桌宠人格 | 安全场景职责 |
|---|---|---|
| **ear** (`apeireth-ear`) | "听"主人说话 | 监听系统事件 (文件改动 / 网络异常 / 登录告警) |
| **eye** (`apeireth-eye`) | "看"屏幕 | 可视化扫描结果 / 漏洞雷达图 |
| **hand** (`apeireth-hand`) | "动手"帮主人 | 执行安全工具 / 运行扫描 |
| **heart** (`apeireth-heart`) | "感受"主人情绪 | 用户偏好 / 情感反馈 / 风险承受度 |
| **brain** (`apeireth-brain`) | "思考" | LLM 调用 + 推理 |
| **mind** (`apeireth-mind`) | "深层思考" | `apeireth-council` 7 advisor = **可解释的安全决策** |
| **body** (`apeireth-body`) | "身体动作" | Live2D 动画驱动 (扫描时紧张 / 报告时舒展) |
| **memory** (`apeireth-memory`) | "记忆" | 持久化 + semantic_search = **历史漏洞库** |
| **voice** (`apeireth-voice`) | "说话" | 报告播报 / 语音告警 (R151+ 规划 GPT-Realtime-2) |

**关键洞察**: 9 organ **不是 9 个独立模块**, 是 9 个**人设维度**。桌宠给 organ "灵魂"; 安全给 organ "职责"; 插件给 organ "扩展能力"。

### 5.3 安全场景套件规划 (v2.0)

| 场景 | Skill 模板 | 涉及 organ | 备注 |
|---|---|---|---|
| **渗透测试** | `pentest-skill` | hand + brain + eye + mind | 对标 VulnClaw |
| **代码审计** | `code-audit-skill` | eye + brain + mind | 静态分析 + LLM 解释 |
| **漏洞扫描** | `vuln-scan-skill` | hand + eye | 集成 nmap / nuclei 等 |
| **合规检查** | `compliance-skill` | mind + memory | SOC2 / ISO27001 / 等保 |

---

## 6. 后端"撑得起"的关键拼图 (本文件建议补的 4 块)

### 6.1 拼图 A: 插件机制 (最关键)

**现状**: `apeireth-extension` 存在, 但未明确"外部开发者扩展路径"
**目标**: 借鉴 deepseek-harness 的 Cordis 思路, **用 Rust 实现**:
- 每个插件是**独立 crate**, 通过 trait 接入
- 插件可以是: 前端消费者 / 安全 Skill / 模型适配器 / 工具扩展
- **前端不是"客户端", 是"插件消费者"** — TUI/Tauri/Live2D 都是消费者

**关键 trait (建议)**:
```rust
pub trait FrontendConsumer {
    fn render_loop(&mut self, event: AcpEvent) -> Result<RenderOp, Error>;
    fn handle_input(&mut self, input: UserInput) -> Result<AcpRequest, Error>;
}
```

### 6.2 拼图 B: Skill 引擎 (VulnClaw 关键)

**现状**: `apeireth-skills` 存在, 有 SkillRegistry + SkillRecommender + SkillExecutor + 5 phase state machine (R125)
**缺**: **Skill 模板引擎** + **权限边界**
**建议补**:
- `apeireth-skill-runtime` 新 crate, 提供:
  - Skill YAML/JSON 模板格式
  - Skill 权限边界 (工具白名单 + 资源配额)
  - Skill 依赖解析 (Skill A 依赖 Skill B 时)
  - Skill 热加载 (借鉴 TUI 的 tool-policy.json notify)

### 6.3 拼图 C: Privacy 护栏 (借鉴 opencode-vibeguard)

**现状**: 不存在
**目标**: `apeireth-guard` 新 crate:
- 所有出站 LLM 请求先过 Guard, 替换敏感字符串 (API key / password / token / email / IP)
- 响应回来再反向替换 (placeholder → 原值)
- 借鉴 opencode-vibeguard 思路, 但用 Rust 实现 + 集成 `apeireth-sovereignty` 隔离
- 应该是**后端可插拔**, 不是前端的责任
- 0 信任: 默认开启, 用户可配置脱敏规则

**关键 trait (建议)**:
```rust
pub trait GuardRule {
    fn name(&self) -> &str;
    fn detect(&self, text: &str) -> Vec<Match>;
    fn replace(&self, text: &str, matches: &[Match]) -> String;
    fn restore(&self, text: &str, mapping: &HashMap<String, String>) -> String;
}
```

### 6.4 拼图 D: 统一前端协议 (`apeireth-acp` 收敛)

**现状**: `apeireth-acp` 存在, 但**未确认对"渲染层消费者"足够友好**
**目标**:
- 收敛成**单一 REST/JSON-RPC endpoint** (类似 `/v1/chat`)
- 新增 WebSocket 推送事件 (用于桌宠的消息气泡)
- 文档化 "前端消费者集成手册"
- 借鉴 deepseek-harness 的"沙箱化插件 + JSON-RPC" 模式

---

## 7. v1.1 / v1.5 / v2.0 路线图补充

> **完整时间线见顶层 `ROADMAP.md`**; 本节只列**新增项**.

### 7.1 v1.1 (短期, 8/11 - 9/14) — 现状

**当前在做**:
- ✅ TUI 持续打磨 (R164-R169 多次 lint/cleanup 闭环)
- ✅ 借鉴 LiteLLM / opencode / Guardrails 重试 (R127-2 派活)
- ✅ 0 装 PASS 严守、不主动 commit (C1/C2 严守)
- ✅ LIVE MiniMax 验证 (R168, HTTP 200, 5.5s latency, 680 tokens)

**本文件新增**: 无 (v1.1 主要做后端, 不动前端方向)

### 7.2 v1.5 (中期, 9 - 12 月) — 现有 ROADMAP + 本文件新增

**现有 ROADMAP**:
- ASI Python 整合 (R11 baseline 严守)
- Tauri 终极前端 prototype
- 5 拆 crate
- StateGraph 4 协议 handler trait 真接

**本文件新增** (建议):
- 🆕 **Live2D 桌宠前端** (v1.5 标志特性, 破圈关键)
  - 新建 `crates/apeireth-desktop-pet/` 或 `frontend/pet/`
  - 9 organ 人格化映射
  - 永远顶层 + 透明窗口
  - 模型库 (Live2D 或 VRM, 待主人拍板)
- 🆕 **`apeireth-guard` 隐私护栏插件**
  - 借鉴 opencode-vibeguard
  - 集成 `apeireth-sovereignty`
  - 默认开启 + 用户可配置
- 🆕 **`apeireth-skill-runtime` Skill 引擎**
  - Skill 模板 + 权限边界 + 依赖解析
  - 热加载
- 🆕 **Tauri 2 真前端 prototype** (解冻 stub)
  - 抄 TUI 瘦客户端模式
  - 9 organ 桌面 widget

### 7.3 v2.0 (长期, 2027+) — 现有 ROADMAP + 本文件新增

**现有 ROADMAP**:
- R128+ 升级
- 主人 1.0 release 流程
- GitHub remote
- 终极路线图

**本文件新增** (建议):
- 🆕 **Plugin Marketplace**
  - 外部插件可上传、可审核、可分发
  - Cordis 思路 Rust 实现
- 🆕 **AI 安全场景套件** (v2.0 杀手锏)
  - 渗透测试 / 代码审计 / 漏洞扫描 / 合规检查
  - 直接对标 VulnClaw + 远超 (审计/隔离/解释/持久化)
- 🆕 **桌宠角色市场**
  - 用户可上传 Live2D / VRM 模型
  - 9 organ 人设模板
- 🆕 **开发者生态**
  - 第三方开发者可基于 ACP 写前端 / 安全 Skill / 模型适配器
  - ACP 文档化 + SDK (已有 `apeireth-sdk`, 需扩展)

---

## 8. 借鉴来源 (完整 7 链接)

| # | 链接 | 用途 | 借鉴 ID (建议, 待主人拍板) |
|---|---|---|---|
| 1 | https://project-neko.cn | 桌宠前端范式 | `R2XX-BORROW-Project-N-E-K-O-NEKO-2026-08` |
| 2 | https://github.com/Project-N-E-K-O/N.E.K.O | 桌宠 GitHub 源码 | (同上) |
| 3 | https://github.com/MIO-456/Lumi_Nox | 桌宠 + 模型库 | `R2XX-BORROW-MIO-456-Lumi_Nox-2026-08` |
| 4 | https://github.com/deepseek-ai/deepseek-harness/ | 插件化后端范式 | `R2XX-BORROW-deepseek-ai-deepseek-harness-2026-08` |
| 5 | https://github.com/inkdust2021/opencode-vibeguard | Privacy 护栏 | `R2XX-BORROW-inkdust2021-opencode-vibeguard-2026-08` |
| 6 | https://github.com/Netw0rkNoob/VulnClaw | AI 安全 Skill 编排 (作者 Unclecheng-li) | `R2XX-BORROW-Unclecheng-li-VulnClaw-2026-08` |
| 7 | https://github.com/Unclecheng-li/DeepSec | AI 安全深度方向 | `R2XX-BORROW-Unclecheng-li-DeepSec-2026-08` |

**借鉴进度 (per 决策 #55 §3 + 决策 #33 §2.3 C2)**:
- ✅ 借鉴 8/11 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers) 已真实施
- ⏳ 借鉴 3/11 (LiteLLM / opencode / Guardrails) 准备重试
- ❌ 借鉴 1/11 (OpenCog AGPL-3.0) 跳过
- 🆕 **建议新增借鉴 7 项 (本文件)**: 桌宠/插件化/安全 三组维度 — 待主人拍板是否纳入下一轮借鉴清单

**0 装 PASS 严守**: 本文件为愿景文档, 借鉴 ID 待主人拍板后才进入真实施阶段。

---

## 9. 风险与权衡

### 9.1 桌宠路线的风险

- **Live2D 商业授权风险**: Cubism SDK 商业授权年费或单项目授权, 商业化前必须明确
  - **缓解**: VRM 路线开源替代, 或用 Live2D 社区版 (个人/非商用)
- **桌宠的"打扰"风险**: 永远顶层可能被用户视为干扰
  - **缓解**: 多模式切换 (陪伴/气泡/最小化), 用户可配
- **目标用户群审美**: NEKO / Lumi_Nox 都是 "猫娘/二次元" 路线, 不一定匹配 Apeireth 的"严肃工具"定位
  - **缓解**: 模型库可选 (二次元 / 写实 / 抽象), 桌宠皮肤化, 默认走 "专业工具伙伴" 风格

### 9.2 插件化路线的风险

- **Rust 插件生态 vs Python 插件生态**: deepseek-harness 用 Python (Cordis), Apeireth 用 Rust, 生态成熟度差距大
  - **缓解**: 借鉴 langgraph / superpowers 等 Rust 借鉴经验; 同时提供 Python 绑定 (已有 `apeireth-pybridge`)
- **插件安全**: 第三方插件可能引入漏洞
  - **缓解**: 所有插件走 `apeireth-sovereignty` 隔离; 插件审核机制

### 9.3 安全路线的风险

- **法律风险**: 渗透测试工具在某些司法管辖区有法律限制
  - **缓解**: 默认只对"自己授权的目标"启用, 强提示用户责任
- **误报风险**: LLM 误判漏洞可能造成用户决策错误
  - **缓解**: `apeireth-council` 7 advisor 多视角审视; `apeireth-arbitration` 审计留痕

---

## 10. 待主人拍板项 (汇总)

按重要性排:

| # | 拍板项 | 备选 | 建议 |
|---|---|---|---|
| 1 | **桌宠技术路线**: Live2D (商业授权) vs VRM (开源) | 两条路都做; 先 Live2D prototype 后再 VRM | 主人原话倾向 "扩大用户群体", 建议 **VRM** (开源/社区多/无授权风险), 先做 prototype 验证 9 organ 人格化 |
| 2 | **`apeireth-guard` 是否 v1.5 必做** | v1.5 必做 vs v2.0 推迟 | 建议 **v1.5 必做** (Privacy 是 v2.0 插件市场的信任基础) |
| 3 | **`apeireth-skill-runtime` 是否 v1.5 必做** | v1.5 必做 vs v1.6 | 建议 **v1.5 必做** (VulnClaw 对标必要条件) |
| 4 | **Tauri stub 是否解冻 + 真做** | 解冻真做 vs 维持冻结 | 建议 **解冻 + v1.5 真做** (主人原话: "我们最后要做的前端应该是 Tauri") |
| 5 | **借鉴清单是否新增 7 项** | 全加 / 选加 / 不加 | 建议 **全加, 但优先级分批**: 桌宠路线决策后加桌宠借鉴; 其余进 v2.0 调研 |
| 6 | **Plugin Marketplace 是否 v2.0 核心** | 核心 vs 推迟 | 建议 **核心** (Cordis 思路是后端范式, 不可推迟) |
| 7 | **是否设置 "v1.5 多前端协调人"** | 设 vs 不设 | 建议 **设** (TUI/Tauri/桌宠/Web 跨前端, 需要一个协调视角) |

---

## 11. 相关文档索引

- **顶层**: `ROADMAP.md` (1.0/1.1/1.5/2.0 主时间线)
- **v2 战略**: `docs/v2-strategy/` (00-VISION / 01-INDUSTRY-LANDSCAPE / 02-VCP-DEEP-COMPARISON / 03-EXTREME-PLAN / 04-CRATE-CONSOLIDATION / 05-EXECUTION-NOW / 06-TUI-UPGRADE-ROADMAP / 07-VCP-GAP-UPGRADE-PLAN / **08-本文件**)
- **借脑**: `docs/r149/`, `docs/r150/`, `docs/r153/` 等
- **借鉴源文件**: `Desktop\重要参考项目，产品方向前辈.txt` (7 链接)
- **关键 crate**: `crates/apeireth-acp/`, `crates/apeireth-skills/`, `crates/apeireth-extension/`, `crates/apeireth-sovereignty/`, `crates/apeireth-council/`

---

## 12. 一句话总结

> **Apeireth v2.0 = 插件化后端 (Cordis) + 三前端范式 (TUI/Tauri/桌宠) + AI 安全核心场景 (VulnClaw/DeepSec), 9 organ 既是技术模块也是人设维度, 桌宠让 organ 有灵魂, 安全让 organ 有职责, 插件让 organ 可扩展。**

---

*End of document.*
