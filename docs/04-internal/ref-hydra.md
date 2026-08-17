# Hydra 深读笔记（2026-08-16，源码验证）

来源：`Downloads\ref\hydra`（agentralabs/hydra，74 workspace crates / 956 .rs / 123,434 行 / 2,106 tests / **整仓仅 1 个 commit**）。

## 一句话

**Hydra = 「营销极端化、骨架真实」的单体原型**：提示分层、置信度数学、结构化宪法、写前日志等工程细节扎实，值得 Apeireth 抄作业；但核心记忆引擎缺失导致**仓库不可构建**、宪法门是自报式装饰、functor/8 层记忆/技能包格式全是「声明未实现」。

## 验证结论

| 项 | 结论 | 关键证据 |
|---|---|---|
| 宪法治理 | ✅ 真（确定性规则式）但执行自报式 🟡 | 7 条编译期 law + action_type 前缀匹配，无 LLM；调用方自报 context；ACTIVE 循环对用户命令检查恒通过（loop_active.rs:57-74） |
| 自写基因组 | ✅ 真（多路自写知识对）| situation/approach + Beta-Binomial 置信度；做梦结晶阈值 obs≥5 && 成功率≥0.75；写的是**知识数据**非代码 |
| 持久记忆 | 🟡 设计扎实但**不可构建** | 依赖外部 agentic-memory repo（不存在）→ cargo metadata 失败；8 层记忆只有 2 层有写入者；检索是子串 IDF 非「128 维向量」 |
| Drop TOML learns | ✅ 真但=知识注入非身份 | genome.toml 灌 genome；34 个 functor/skill.toml/actions.toml **无代码解析** |
| 74 crates | 🟡 69 可达 5 孤儿 | hydra-llm 自认未接线；edition 混用 |
| skills | 🟡 声明丰富消费面窄 | 373 entries 与 README 450+/303 三处数字打架 |

## 比 Apeireth 强 / 值得吸收（按价值）

1. **结构化宪法门**（LlmJudicator 前加廉价硬门；**规避坑：action_type 必须系统侧生成非调用方自报**）
2. **反幻觉记忆注入**（EMI/NEC：禁止说「based on our previous conversations」视为 fabrication——直接消除「我记得」幻觉）
3. **提示 Tier 0-7 分层 + HEFP 校准 + 凭据脱敏**（prompt 装配层直接抄）
4. **Beta-Binomial 置信度**（conf=91% [89%-93%] obs=25000 strength=STRONG，纯 Rust）——能力提案/自测的数学化自信度
5. **写前日志 + SHA256 内容哈希 + 不可变收据**（SessionLog 加 content_hash 校验链）
6. **append-only 因果时间索引 + 决策图**（事件溯源补 causal_root）
7. **executor 13 级升级（FAILED 不存在）+ HardDenied 需证据**
8. **vault 凭据 access 模型**（read/write/delete/spend/max_spend）——权限包细化到花钱上限
9. **morphic hash-chain 身份**（与 continuity_id 互补）
10. **guardrail kill-switch + dead-man-switch + boot lock + 崩溃自愈**

## 吹牛清单

- 「AgenticMemory 128 维向量 20 年格式」→ 外部 repo 缺失，**workspace 无法构建**
- 「7 laws checked every 100ms」→ 100ms 检查的是状态不变量，且 constitution-reachability 自认 always passes
- 「Hydra says 'Based on our prior conversations'」→ 代码 EMI 模板**明确禁止**此说法
- skills 数字三处打架；「self-writing genome writes to skills/generated/」→ 目录不存在；「17 middlewares」→ 实际 13
- 「Zero physical limitations / Year 5: 2,000+ entries / LLM 用量 5%」→ 纯路线图营销

## Apeireth 已超越

宪法语义评审（LLM 判 E 层 vs 前缀匹配）✓ / 能力演化（真长能力可执行 vs 知识对字符串）✓ / 插件生态（可执行 vs 未接线 TOML）✓ / 可构建 ✓ / 结构化记忆检索 ✓ / SessionLog 完整闭环 ✓
