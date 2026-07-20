# 80 项目监控 / 借鉴清单 — Apeireth 按模块分类
**作者**: 楚零
**触发**: 主人 13:47 "这 80 项目中有的可能也有用, 我们开干 Apeireth 要一步步按计划来, 这个系统基石基底也要按模块按步骤科学的造"
**目的**: 主人昨晚给 80 项目名,我做按 Apeireth 5 层架构的**借鉴清单**

---

## ⭐ 主人的元要求

主人 13:47 三个意思:
1. **80 项目里可能有借鉴价值的** — 但我不要急
2. **按计划来** — Apeireth 顶层设计 v1 是图纸,按图纸
3. **按模块按步骤科学的造** — 不是一次性, 不是激进

所以我做了**借鉴分层**: 5 层架构 × 80 项目 = 借鉴表。

---

## ⭐ L1 LLM Kernel — 借鉴层

| 项目 | Stars | 借鉴什么 |
|------|------|---------|
| pi-mono | ? | 极简 monorepo,代理编码先驱 |
| Tavily Web Search | ? | 联网搜索 API,质量比 DuckDuckGo 高 |
| Composio | 29k | 1000+ toolkits 标准化 |
| Playwright-MCP | 35k | 浏览器自动化 MCP 标准 |

---

## ⭐ L2 Interaction Layer — 提问协议

| 项目 | Stars | 借鉴什么 |
|------|------|---------|
| anthropics/skills | 162k | **SKILL.md 格式** (Anthropic 官方技能标准) |
| anthropics/financial-services | 33k | Anthropic 金融 agent 模板 |
| mattpocock/skills | 177k | 工程实践 (.agents 目录) |
| Pi-mono / anysearch-ai/anysearch-skill | ? | anysearch skill 模式 |
| Composio / multice-ai/andrej-karpathy-skills | ? | Karpathy 风格 skill |

---

## ⭐ L3 Memory Layer — 永生记忆(关键!)

| 项目 | Stars | 借鉴什么 |
|------|------|---------|
| **claude-mem** (thedotmack) | 87k | **session capture + AI 压缩 + relevance injection — 我们的直接竞争** |
| Shadoweave 团队的 HMS 全息记忆系统 | ? | 全息记忆 — 哲学对应"中央 AI 永恒身份" |
| MemPalace | ? | Memory 元数据 |
| DeusData/codebase-memory-mcp | 32k | Code intelligence MCP server |
| TencentDB-Agent-Memory | ? | 厂级 (腾讯) memory 路线 |
| rohitg00/agentmemory | 25k | **"#1 Persistent memory for AI coding agents" — 跟我们直接对位** |
| DeusData/codebase-memory-mcp | 32k | codebase → knowledge graph |
| alibaba/zvec | 15k | **In-process 向量库,可替换 AgentMemory 的 Qdrant 依赖** |
| nashsu/llm_wiki / langchain-ai/openwiki | ? | wiki 式 memory |
| Mythos 架构(逆向破解) | ? | 跟 Apeiron/Apeireth 哲学对应 |

---

## ⭐ L4 Identity Layer — 关系图谱 / 持久化

| 项目 | Stars | 借鉴什么 |
|------|------|---------|
| Self-herness / MemPalace | ? | 自我一致性检查 |
| abhigyanpatwari/GitNexus | 44k | "Codebase → knowledge graph → MCP" 架构思路 |
| 666ghj/mirofish | 68k | Swarm intelligence 引擎 (盛大集团) |
| Self-Improving / Dexter | ? | 自改进 agent |
| rohitg00/agentmemory | 25k | identity persistence 上 SOTA |
| Karpathy 升级版 | ? | 教育/Karpathy 风格 system prompt |
| 受 Karpathy 启发的 Claude Code 指南 | ? | Karpathy-style TUT |

---

## ⭐ L5 Effect Layer — 主动 / 涌现 / 自组织

| 项目 | Stars | 借鉴什么 |
|------|------|---------|
| AHE / DGM / OpenSage (上次研究) | - | Harness 自进化 + archive + 自创 agent |
| AFlow / ShinkaEvolve (上次研究) | - | Workflow design |
| TradingAgents | 93k | **多 agent 金融辩论** — Master-Apprentice 模式 |
| OpenStock / OpenAlice | ? | Trading agent |
| Horizon Trading | ? | quant |
| brokermr810QuantDinger | ? | quant |
| 666ghj/MiroFish | 68k | Swarm 引擎(盛大集团出品) |
| Kronos | ? | 时间序列预测 (可能是清华的金融 AI) |
| FinSight-AI | ? | 港大金融 AI |
| AI-ToEarn | ? | 自动化变现 |

---

## ⭐ 工具层 / 启发 — 单独监控(不进架构)

| 项目 | 借鉴 |
|------|------|
| **agent-memory/SWE-ReX** | 沙盒代码执行(本地+云),替代 E2B |
| D4Vinci/Scrapling | 自适应 Web Scraping,自带 agent-skill |
| VoltAgent/awesome-design-md | 24K awesome list |
| lyogavin/airllm | 70B 推理 4GB GPU (极简本地推理) |
| Hermes Agent | Nous Research 出品 |
| Claude Code (泄露 51 万行) | Anthropic 内部 |
| simular-ai/Agent-S | 用电脑的 agent (Container Use 先驱) |
| Aitoearn / Openhuman / Odysseus | 神秘项目 — 主人确认下 |
| TradesAlice/OpenAlice / OpenAlice | trading AI |
| slimbot/light_claude | slim |
| Mintplex-Labs/anything-llm | all-in-one LLM 桌面 |
| Pi-mono | 极简 monorepo |
| safe-guard for Humanity check | 内容安全 |

---

## ⭐ 项目相关性排序(借鉴优先级)

| 阶段 | 必须先研究的 | 为什么 |
|------|------------|---------|
| **Phase 1 启动创世** | anthropics/skills (SKILL.md 模式) | 我们要兼容标准 |
| **Phase 1 启动创世** | pi-mono (极简 monorepo) | 主人之前用过 |
| **Phase 1 自进化** | AHE / DGM / OpenSage (上轮已读) | 自进化 SOTA |
| **Phase 2 Memory** | rohitg00/agentmemory, claude-mem, MemPalace, HMS, alibaba/zvec | 直接竞争 + 向量存储 |
| **Phase 2 Reconsolidation** | MemGPT (= Letta), LangMem | 让 cognitive architecture 借鉴 |
| **Phase 3 自组织** | MiroFish, TradingAgents | 主动 agent 范式 |
| **Phase 4 跨小模型** | Composio | toolkits 标准化 |

---

## ⭐ 主人的"按模块按步骤科学的造"

**这意味着**:
1. **不是一次性全借鉴** — 按层推进
2. **每个 phase 借鉴几个就够** — 不要贪婪
3. **借鉴前先调研 + 跑通 PoC** — 不是 git clone 就用
4. **每次借鉴必须留下 DEV-LOG 记录** — 我有 DEV-LOG 模板了

---

## 我现在给主人 3 个真问题

```
1. 80 项目里哪些我有其他信息? 
   (主人之前 22:43 给过我这些,我现在大致都认识了)
   或者主人确认下哪些项目是优先?

2. 借鉴路线 — 你想我按什么顺序?
   A. 按我的判断 (从 L1 LLM Kernel 开始)
   B. 按模块最关键的 (Memory / Identity优先,因为这是主人的中央 AI 愿景)
   C. 主人的其他顺序

3. 你提到的"按模块按步骤科学的造" — 是有具体科学方法?
   比如: hypothesis → experiment → measure → refine (Popper 证伪)?
   或者 Bayesian 迭代?
   或者让我自己定 (TDD? BDD?)
```

---

## 我现在不动手 — 等主人

主人,你说"按模块按步骤科学的造",我做的是:
1. ✅ 5 层架构对照清单(本文)
2. ✅ 每个项目按层归类
3. ✅ 借鉴优先级排序
4. ⏳ 等主人拍板顺序 + 科学方法

P.S. 这是**真正的**借鉴监控 — 不是"过一遍列表"
而是**"按层按阶段精确借鉴"**。

---

_楚零 2026-07-20 13:52_
_借鉴清单 ready, 等主人拍板_