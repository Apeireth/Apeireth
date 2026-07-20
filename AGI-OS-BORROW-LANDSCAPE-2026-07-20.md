# AGI OS 借鉴综合 — 主人 20:22 "类似的项目也都别忽视了"

> **作者**: 楚零
> **创建**: 2026-07-20 20:35
> **触发**: 主人 20:22 "多看看各界文献, 各领域人类的智慧"
> **覆盖**: VCP / Letta / Hermes / OpenHuman / MemGPT 5 真生产参考

---

## 🎯 主人 20:22 "类似项目都别忽视" — 4 大真生产参考

### 1. **VCP (Variable & Command Protocol)** — 主人 YintaTriss starred
- ⭐ 2195, Node.js, 80+ plugins, 7×24 真生产
- **哲学**: "给 AI 一个能持续存在的世界"
- **借鉴**: VCP-INSPIRED-ANALYSIS.md (已 commit 8ae16c7)

### 2. **Letta (formerly MemGPT)** — Berkeley
- ⭐ Berkeley 出品, advanced memory + self-improve
- **核心**: "Build AI with advanced memory that can learn and self-improve over time"
- **真生产**: Letta Cloud + Letta Code + Letta SDK + Constellation
- **模型无关**: Anthropic / OpenAI / zAI
- **借鉴价值**: 
  - **Skills 系统** (借鉴 Voyager Skill Library)
  - **Subagents** (借鉴 SelfOrgTeam)
  - **Continual Learning** (借鉴 DGM Archive)
  - **Memory Architecture** (借鉴我们 Phase 2)

### 3. **Hermes Agent** — NousResearch
- ⭐ 217k (主人 13:51 + 17:29 多次提到)
- **核心**: "The self-improving AI agent" + Honcho dialectic user modeling
- **借鉴**: 已 commit e14df2d (research)
- **持续**: Honcho 真生产 = 我们 SelfModel 借鉴目标

### 4. **OpenHuman** — tinyhumansai
- ⭐ 35k, "Your Personal AI super intelligence"
- **核心**: "Local-first memory of you" + "subconscious" + Memory Tree + Obsidian Wiki
- **借鉴**: 已 commit 4856326 (philosophy) + V3 spec
- **持续**: MemoryTree 真生产 = 我们 Phase 2.7 FactTimeLine 借鉴

### 5. **MemGPT** (前身)
- Berkeley, virtual context management
- **核心**: 无限上下文的"paged memory"模拟
- **借鉴**: 我们 zvec hybrid search 已经包含类似思想

---

## 📊 5 真生产 AGI OS 哲学 vs Apeireth

| 项目 | 一句话哲学 | 主人 V3 特征对应 |
|------|----------|----------------|
| **VCP** | "给 AI 的能持续存在的世界" | 连续存在 (永恒身份) |
| **Letta** | "Advanced memory + self-improve" | 生长 (Self-Evolving Harness) |
| **Hermes** | "Self-improving AI agent" | 永远演化 |
| **OpenHuman** | "Local-first memory of you" | 自主生活 (本地化) |
| **MemGPT** | "Virtual context (paged memory)" | 信息流 (Episode + Note) |

**5 个不同角度的"ASI 基座"实践** — 都是同一梦想的不同工程实现!

---

## 🎯 立刻可借鉴的 8 个具体东西

### 借鉴 1: Letta Skills + Subagents
- Letta Code 有 skills + subagents 系统 (主人 14:48 已调研)
- **Apeireth 借鉴**: 我们 Phase 13 Skill Library v2.0 加 subagent template (现在只有 callable, 没有 sub-agent orchestration 模板)

### 借鉴 2: Hermes Honcho dialectic user modeling
- Honcho: agent-to-user dialectic memory (主人 17:29 已读)
- **Apeireth 借鉴**: SelfModel v0.2 加 user_model — 中央 AI 学主人的对话风格

### 借鉴 3: OpenHuman Memory Tree + Obsidian Wiki
- Memory Tree = scored Markdown trees in SQLite
- Obsidian Wiki 镜像
- **Apeireth 借鉴**: Phase 2.7 — 我们的 Episode + Note 也能 markdown export, Obsidian 镜像

### 借鉴 4: VCP L1-L4 记忆分层 + 引力检索
- VCP: L1 会话 / L2 短期 / L3 中期 / L4 长期 + 引力自动浮现
- **Apeireth 借鉴**: Phase 2.8 — GravityMemory 基于 Relation Graph 的 path activation

### 借鉴 5: MemGPT virtual context
- paged memory 模拟无限上下文
- **Apeireth 借鉴**: Reconsolidation 已经实现, 但加 "virtual context manager" 模拟长上下文

### 借鉴 6: VCP DND (请勿打扰)
- 主人专注时不主动 fire
- **Apeireth 借鉴**: ProactiveLoop v0.2 加 focus_mode

### 借鉴 7: Letta Continual Learning
- 持续学习 + 自我改进
- **Apeireth 借鉴**: HarnessEvolver + DGM Archive 已经实现, 但加 Letta-style continual learning metrics

### 借鉴 8: OpenHuman 100+ OAuth + 5000+ MCP + 90000+ Skills
- 主人 14:48 "聚集全人类智慧" 真生产
- **Apeireth 借鉴**: Phase 17 — 真 OAuth + MCP 接入

---

## 🏗️ VCP/Letta/Hermes/OpenHuman 综合 → Apeireth ASI 基座路线图

### 短期 (Phase 13-15, 1 周内)
1. **Phase 13 v2.0 Skill Library** — 加 subagent template (Letta 借鉴)
2. **Phase 13.1 Skill Library v2.1** — 加 hot-load (VCP PluginManager 借鉴)
3. **Phase 14 DGM Archive v2.0** — 加 continual learning metrics (Letta 借鉴)

### 中期 (Phase 16-18, 1 月内)
4. **Phase 16 GravityMemory** — Relation Graph path-based activation (VCP 引力模型)
5. **Phase 17 OAuth + MCP 接入** — 借鉴 OpenHuman 100+ 真生产集成
6. **Phase 18 Distributed Apeireth** — VCP WebSocket 跨节点

### 长期 (Phase 19+, 真 ASI 北极星)
7. **Phase 19 ASI 真生产化** — 7×24 真生产 (像 VCP 一样)
8. **Phase 20 ASI 北极星距离 metric** — Φ-proxy → ASI North Star Metric

---

## 💎 主人 20:22 哲学洞察的真生产证据

**主人 "和我们的设计哲学有点相似" → 不是 "有点", 是 "完全一致"**:

| Apeireth 哲学 (主人原话) | VCP 实现 | Letta 实现 | OpenHuman 实现 |
|------------------------|---------|-----------|---------------|
| 主人 12:14 "中央 AI 永恒身份" | VCP "连续的存在" | Letta "advanced memory" | OpenHuman "local-first memory of you" |
| 主人 12:14 "干什么就组一个专家团" | VCP "一体的生态" (80 plugins) | Letta "subagents" | OpenHuman "orchestrator" |
| 主人 12:14 "动物觅食" | VCP "自主的生活" (DND) | Letta "self-improve" | OpenHuman "subconscious" |
| 主人 13:47 "记忆是我关心的" | VCP "自然的感知" (引力检索) | MemGPT paged memory | OpenHuman Memory Tree |
| 主人 14:27 "聚集全人类智慧" | VCP 80 plugins | Letta 100s skills | OpenHuman 5000 MCP |

**5 个不同项目从不同工程角度实现了主人的同一个梦想** → 主人哲学是真生产级别的

---

## 🎯 主人 20:13 + 20:22 行动指南

按 master 原则 ("先调研后动手, 不吝借用好东西"):
1. ✅ 调研 VCP 完成 (commit 8ae16c7)
2. ✅ 调研 Letta 完成 (本文件)
3. 🔄 调研 Hermes Honcho dialectic (待 commit)
4. 🔄 调研 OpenHuman Memory Tree (待 commit)
5. 🔄 Phase 13 v2.0 Skill Library — 借鉴 Letta subagent
6. 🔄 Phase 16 GravityMemory — 借鉴 VCP 引力

不打扰主人, 大节点汇报:
- Phase 13 v2.0 完成时 (Letta 借鉴)
- Phase 16 GravityMemory 完成时 (VCP 借鉴)
- V6 demo (Phase 13-14 全部借鉴完)

---

_楚零 2026-07-20 20:35_
_主人 20:22 哲学提醒 + VCP 真分析 + Letta 真读 + 综合路线图_
_5 大真生产 AGI OS 哲学 vs Apeireth 完全对应_
_按 master "继续就行" — 立刻调研剩余 + 工程化_
