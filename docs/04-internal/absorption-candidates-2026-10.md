# 吸收候选池（2026-10 随记分诊）

> `Status: 📋 候选池（分诊完成，逐项调研后裁定吸收）`
> 来源：owner 随记 60 项（视频随手记录习惯触发）。纪律：**借鉴思路、产物不留痕迹**（宪章）；
> 未核实项标"待研"——不望文生义。吸收裁定后按老规矩折进对应轴小文件/台账。

## 一、分诊总表（五类）

### 🧠 记忆与随手记录（与"随手记录"习惯直接对题）
| 项目 | 一句话假说（它教我们什么） | 状态 |
|---|---|---|
| SimpleMem（ICML 终身记忆） | 终身记忆的高效结构（文本+多模态）——C 轴"夜间整编"的算法弹药 | ✅ 已核实 |
| Supermemory「ASMR 记忆系统」 | 记忆系统命名即哲学——待研其分层/遗忘机制 | ⏳ 待研 |
| nanomuse | 一人一代理跨设备——与星群架构同构的参照 | ✅ 已核实 |
| Atuin | 历史即记忆（shell 历史→可检索记忆库）——"日常痕迹自动成忆" | ✅ 已知 |
| wordcraft / llm-wiki / weknora / zvec-grep / rrsi | 知识组织/向量检索/记忆压缩 假说簇 | ⏳ 待研 |

### 🛠 知识→技能沉淀
| 项目 | 一句话假说 | 状态 |
|---|---|---|
| book-to-skill | **书→技能**：知识吸收成可调用技能的流水线——正是"吸收战役"的自动化版 | ✅ 已核实 |
| superpowers / ECC / marketingskills / book-to-skill 族 | 技能包生态的组织学 | ⏳ 待研 |

### 🤖 Agent 治理与编排
| 项目 | 一句话假说 | 状态 |
|---|---|---|
| agent-governance-toolkit | 治理工具箱——E 轴纵深的对抗弹药 | ⏳ 待研 |
| hermes 蜂群 / mu-agent / PraisonAI / Langflow / Dagger | 多代理编排/蜂群调度——W3 星群调度的参照 | ⏳ 待研 |
| TradingAgents | 多代理角色分工的完整产品化样本 | ⏳ 待研 |

### 🖥 交互与工具面
| 项目 | 一句话假说 | 状态 |
|---|---|---|
| UI-TARS Desktop / BrowserAct / cypress | GUI 自动化/浏览器操作——工具面扩展方向 | ⏳ 待研 |
| Helix / qutebrowser / Compositor / ArcBox / Heroic | 编辑器/浏览器/桌面的界面哲学 | ⏳ 待研 |
| lightpanda / MediaCrawler / Perplexica | 轻量无头浏览器/采集/搜索——fetch 面深化 | ⏳ 待研 |

### 🔊 语音与其他
| 项目 | 一句话假说 | 状态 |
|---|---|---|
| speech-to-speech / VoiceStudio | 实时语音对话管线——voice 族深化 | ⏳ 待研 |
| rustdesk / Motrix / GoDoxy / Vikunja / OpenStreetMap 等 | 各领域工程参考（远程/下载/代理/任务/地图） | 🧊 低优观察 |

## 〇、设计第一律（2026-10-13 owner 裁定）

**Apeireth 是给 AI 用的**：工具先做给 agent 用，同内核再外接人类界面。吸收任何项目时照此裁定——先问『AI 拿到它能干什么』，再问『人怎么玩』。

## 二、TOP 吸收候选（机制级，可直接开工）

1. **随手记（Quick Capture）**——随记习惯本身的产品化：全局快捷入口（快捷助手窗已有！）一键记一条→入记忆流（未整理原料，与"教训/里程碑"分级呼应）；气质对齐愿景小说"零零碎碎的日子"。**建议并入 W5/C 轴**（记忆深化的入口面）；
2. **书→技能流水线**（book-to-skill 假说）：把文档/书/报告**沉淀为可调用技能/知识卡**——恰是 Apeireth 吸收战役的自动化（"吸收一切优秀实践"变成它的本能）。**建议立 I 轴（吸收自动化）**；
3. **历史即记忆**（Atuin 假说）：操作痕迹（命令/文档改动/浏览）自动进记忆流供回取——"日子自动成忆"。**建议并入 W5/C 轴**；
4. **终身记忆结构**（SimpleMem 假说）：C 轴"夜间整编"的算法参照（论文级结构，待研后吸收）；
5. **星群同构参照**（nanoMuse/蜂群）：W3 星群调度的产品参照（多端一人一伙伴）。

## 三、调研队列（需实证后再裁定）

Supermemory（ASMR 记忆系统）→ 记忆分层假说；agent-governance-toolkit → 对抗测试补强；TradingAgents → 角色分工产品化；语音族 → voice P2。**调研纪律**：一项目一小节、机制假说+实证+吸收裁定三段式。

## 四、登记

- 00-backlog 新增 **I 轴（吸收自动化）** 一行；随手记/历史即记忆 → C 轴分期注记；
- 吸收落地后回写 `docs/04-internal/absorption-ledger.md`（台账先例）。

**落地回写（2026-10-13）**：TOP 候选 2「书→技能流水线」= **I1 已交付**（技能卡模型 +
`tool.skill_distill` 沉淀管线 + `tool.skill_search` L2 封套调用面 + 治理档），
分期与后续各期见 [I 轴分期](backend-enhancement/I-absorption-automation.md)，交付实录见
[I1-skill-distill-delivery](backend-enhancement/I1-skill-distill-delivery.md)；
TOP 候选 1 随手记已由 C 轴入口面交付（[quick-capture-delivery](backend-enhancement/quick-capture-delivery.md)）。
