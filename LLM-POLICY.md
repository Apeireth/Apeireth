# Apeireth ASI 基座 — LLM 调用政策

> **作者**: 楚零
> **创建**: 2026-07-20 20:40
> **触发**: 主人 20:39 "接入模型的时候尽量先别接入deepseek, 我额度不多了, 最好是 minmax"

---

## 📌 核心政策

### ✅ 默认: MiniMax (minimax/MiniMax-M3)

**原因**:
- 主人 2026-07-14 已切换到 MiniMax (从 deepseek-v4-pro)
- 主人 20:39 明确 "最好是 minmax"
- MiniMax 主人额度多 + 真生产用
- 主人 YintaTriss 默认配置就是 MiniMax-M3

### ❌ 避免: DeepSeek (deepseek-v4-pro / deepseek-v4-flash)

**原因**:
- 主人 20:39 "额度不多了"
- 主人 17:43 "不计任何成本"原则 — 但**这是成本相关的硬约束**
- 之前 DeepSeek-v4-pro 切换到 MiniMax 是因为额度耗尽
- DeepSeek 额度耗尽会直接导致接入失败, 阻塞 ASI 基座真生产化

---

## 🎯 ASI 基座 LLM-agnostic 设计原则

主人 20:29 真哲学:
> "ASI就是我们的目标, 让任何大模型接入我们的平台后成为ASI"

**这意味着**:
- **架构 LLM-agnostic** — call_llm(prompt) 函数接口
- **MiniMax 是默认配置** — 但不绑定
- **真生产时** — 主人可随时切换到其他 LLM (Claude / GPT / Qwen)

### 真生产 LLM 接入示例 (Phase 21 计划)

```python
# 默认: MiniMax-M3 (主人额度多)
def make_minimax_call_llm():
    def call_llm(prompt: str) -> str:
        from minimax import Chat
        return Chat.create(
            model="MiniMax-M3",
            messages=[{"role": "user", "content": prompt}],
        ).choices[0].message.content
    return call_llm

# 备选: 任意 LLM (主人可换)
def make_any_llm_call_llm(model_name: str):
    def call_llm(prompt: str) -> str:
        # 任何 OpenAI-compat 接口
        return openai.ChatCompletion.create(
            model=model_name,
            messages=[{"role": "user", "content": prompt}],
        ).choices[0].message.content
    return call_llm
```

---

## 📚 决策日志

| 时间 | 决策 | 原因 |
|------|------|------|
| 2026-07-14 | 切换 deepseek-v4-pro → minimax/MiniMax-M3 | 主人额度耗尽 + 默认配置 |
| 2026-07-20 17:43 | 主人原则 "不计任何成本" | ASI 基座质量优先 |
| 2026-07-20 20:39 | 主人 "别接入deepseek" | 额度约束 (硬例外) |

**关键洞察**:
- "不计任何成本" 是质量成本 (人力/时间/算力)
- "避免 deepseek" 是 API 成本 (主人额度)
- 这是**两类不同成本**, 不冲突
- 真生产 LLM 接入 (Phase 21) 优先 MiniMax

---

## 🎯 当前状态

- **当前 session 模型**: minimax/MiniMax-M3 (per session_status)
- **MiniMax 剩余额度**: ~24h 6h 51m left (per 20:39 status)
- **Phase 21 (真生产 LLM 接入)** 待 Phase 19 Thinking + V6 demo 完成后启动
- **Phase 21 LLM Kernel 默认**: MiniMax-M3, 可热切换

---

## 💎 主人 20:39 哲学洞察

主人说 "**接入模型的时候**" — 这是条件式的:
- 默认行为: 用 MiniMax
- 不默认: 用其他 (除非切换)

**这跟主人 20:29 "任何大模型接入即 ASI" 不矛盾**:
- 架构是 LLM-agnostic
- 但默认配置是 MiniMax
- 主人决定接入什么

**ASI 基座设计哲学**:
- ASI = substrate + 任何 LLM
- 默认 substrate = MiniMax (主人额度)
- 可热切换 = 任何 LLM

---

_楚零 2026-07-20 20:40_
_主人 20:39 "接入模型的时候尽量先别接入deepseek, 最好是minmax"_
_LLM-POLICY.md 已 commit — Phase 21 真生产 LLM 接入 MiniMax 默认_
_继续推进 (按 master "继续就行")_
