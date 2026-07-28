# R8 用户指南 — 怎么用 Memory 三层 / Identity 身份卡 / Self-evolution 自演化

> 作者: technical_writer · R8-DOC-03
> 阅读对象: **真用户**（非工程师）+ 任何想动手用 Apeireth 的人
> 大白话原则: 每个术语第一次出现，配 ≤10 字中文注解
> 主哲学: ASI=∞ 真生产；不假装 / 不破坏 4 层门 / 不绑单模型 / 不刷 KPI
> 重要提醒: **R8 还在准备启动阶段**（详见 §0）；本指南告诉你"如果启动后怎么用"

---

## 0. 阅读须知 — 先看清 R8 在哪一步

> **大白话**：R8 是 Apeireth 这个项目的第 8 阶段。这一阶段要做三件事：① 给中央 AI 装真记忆；② 让它有正式"身份卡"；③ 让它能改自己。

**R8 当前状态**（2026-07-29 实测）：

| 事 | 状态 | 大白话 | 真证据 |
|---|---|---|---|
| 三大轨道规划 | ✅ 已完成 | 三件事的"施工图"画好了 | 4 份 R8 基线文档（r8-delivery/architecture/user-guide/handoff） |
| 三大轨道代码 | ✅ **11 个 v109x 模块真生产** | 11 块拼图已拼好 | v1090/v1091/v1092/v1093/v1094/v1096/v1097x2/v1098/v1099/v109_pipeline |
| 测试覆盖 | 🟡 **119+ 测试全过** | R8 新增测全绿，**全量回归待 R9** | v1091=52 + v1092=44 + v1094=23 |
| master HEAD | ✅ `d745c332` | V1094 已真 commit | `git log --oneline -1` |
| 全量测试 | ❌ 未跑通 | 所有测试跑一遍还有失败 | R7 末 3485/2/3037，小范围 80/6 |
| 用户真实需求 | ❌ **等你拍板** | 你想要先做哪件，先告诉我 | `r8-requirements-decision-matrix.md` §6 |

**所以**：本指南是"未来指南"——告诉你 R8 跑通后你该怎么用。当前最该做的是读完本文 §6（待用户决策的 10 个问题），把你的答案发给项目组。

---

## 1. 大白话词汇表（5 分钟看懂）

> 这一节把所有术语翻译成人话，后面遇到就不用再查。

| 术语 | 大白话 | 一句话例子 |
|---|---|---|
| **ASI** | 超级人工智能（项目终极目标） | 终极目标 = "在任何领域达到或超过人类专家" |
| **中央 AI** | 项目里的"主角 AI" | 就是 Apeireth 自己的大脑 |
| **主人** | 项目发起人（楚零） | 给 AI 立规矩 + 拍板的人 |
| **Memory 三层** | 短/中/长三段记忆 | 短 = 刚说过的话；中 = 最近的话题；长 = 永久事实 |
| **STM** | 短时记忆（最近 50 条对话） | 像人脑"刚才发生了什么" |
| **MTM** | 中期记忆（每小时打主题包） | 像"最近一周我常聊啥" |
| **LTM** | 长期记忆（永不丢的事实） | 像"我是谁、主人说过什么金句" |
| **WAL** | 写前日志（先记再改，崩了能恢复） | 像 Word 的"自动保存" |
| **HotCold** | 热/冷数据分层（热的快存，冷的慢存） | 像把常用的 App 放桌面，不常用的放进文件夹 |
| **Memory Replay** | 状态回放（撤销 + 重做） | 像游戏里的"存档"和"读档" |
| **Dream** | 想象/演绎子系统 | AI 空闲时自己整理记忆（不是真睡觉） |
| **Identity 卡** | "我是谁"档案 | 一张写满身份信息的卡片 |
| **Enrichment** | 富化（启动时自动补全空格） | AI 一启动就把没填的格子自动填上 |
| **Self-evolution** | 自演化（系统改自己） | AI 按规则改自己的配置（不许瞎改） |
| **AHE 5 阶段** | 测→统计→稳定→提方案→验证/保留 | 让系统改自己的"5 步规矩" |
| **Harness** | 包裹 AI 的外系统 | AI 的"外骨骼"，决定它怎么观察/行动/记忆 |
| **HQB** | 衡量 Harness 质量的 4 项指标 | 像 AI 的"体检表" |
| **philosophy_guard** | 哲学守门（防止 AI 越界） | 4 道自动检查 + 1 道人类批准 |
| **V0.3 / V0.4** | ASI 分数公式的两个版本 | 衡量"离 ASI 还有多远"的两种算法 |
| **真生产** | 真能跑的代码（不是占位壳） | 写完能跑、有测试、有审计 |

---

## 2. Memory 三层 — 怎么用

### 2.1 一句话原理

> **大白话**：AI 也有"记性"问题。刚说的话记得（STM），但隔天就忘；常聊的话题过几天还能想起来（MTM）；永久事实（比如"主人是谁"）一辈子不丢（LTM）。

### 2.2 三层的差别（一张表看懂）

| 层 | 大白话 | 容量 | 存多久 | 谁负责 | 类比人脑 |
|---|---|---|---|---|---|
| **STM**（短时） | 刚说过的话 | 最近 50 条对话 | 几分钟到几小时 | STM rolling deque（自动滚动窗口） | "刚才我跟你说了啥来着" |
| **MTM**（中期） | 主题打包 | 每小时打一个主题包 | 几小时到几天 | MTM 主题总结器 | "最近一周我们常聊啥" |
| **LTM**（长期） | 永久事实 | 不限（带重要度门槛 0-10 ≥ 8 才入选） | 永远 | LTM 锚点（永不丢） | "我叫什么、主人是谁、规矩是什么" |

### 2.3 怎么查 Memory

R8 已真生产命令（**V1090/V1091/V1092/V1094 都已落地**，52+44+23=119 测试全过）：

```bash
# 查 STM（最近 50 条对话）—— V1091 memory_replay 真生产
python -m apeireth.v1091_memory_replay --query stm --limit 10

# 查 MTM（按主题过滤）
python -m apeireth.v1091_memory_replay --query mtm --topic "中央AI"

# 查 LTM（永久事实，从 V1094 schema 读 ltm_facts 表）
python -m apeireth.memory_3tier --ltm --min-importance 8

# 查"昨天我们聊了什么"（按时间窗回放）
python -m apeireth.v1091_memory_replay --replay --from "2026-07-29" --to "2026-07-30"

# 查"梦"状态（V1092 dream 状态机 + SchemaPhase）
python -m apeireth.v1092_memory_dream --state --show
```

### 2.4 怎么改 Memory

```bash
# 加一条 STM（短时记忆）—— V1091 capture_state + apply_event
python -m apeireth.v1091_memory_replay --apply --op stm_add --payload '{"text": "今天主人说想要先把 P0 数据修了"}'

# 把一条 STM 升级成 LTM（手动锚定，落 ltm_facts 表）
python -m apeireth.memory_3tier --ltm --anchor --from-stm <episode_id> --importance 9

# 触发"做梦"（V1092 dream 子系统整理 STM → MTM，FORGET 掉无用的）
python -m apeireth.v1092_memory_dream --tick --mode consolidate

# 查 Dream 状态机当前在哪（IDLE→SELECT→LIGHT/REM→CONSOLIDATE→FORGET→REPLAY→EMIT）
python -m apeireth.v1092_memory_dream --state --show

# 写 WAL（V1090 真 fsync，崩了能恢复）
python -m apeireth.v1090_memory_wal --append '{"event": "stm_add", "payload": {...}}'

# 从 WAL 回放（崩溃恢复）
python -m apeireth.v1090_memory_wal --recover --out memory_recovered.json
```

### 2.5 怎么防 Memory 出错

| 防的事 | 大白话 | 命令 / 实现 |
|---|---|---|
| 错删 | 万一误删了怎么办 | `python -m apeireth.v1091_memory_replay --restore --state-id <id>` |
| 重复写入 | 同一条信息不要写两遍 | V1091 白名单 `IDEMPOTENT_OPS` 6 种 + V1094 `event_id UNIQUE` 幂等键 |
| 身份污染 | 别让临时话题污染"我是谁"卡 | LTM 入选门槛 ≥ 8（自动过滤）· V1094 ltm_facts.fingerprint UNIQUE 去重 |
| 崩溃恢复 | 系统崩了记忆会不会丢 | V1090 WAL 真 fsync + `python -m apeireth.v1090_memory_wal --recover` |
| 损坏容错 | WAL 文件部分损坏怎么办 | V1091 `_recover_from_disk` 跳过损坏行 + V1090 sha256 校验 + 累计 `skipped_corrupt` |
| 并发安全 | 多线程同时改怎么办 | V1091 threading.RLock 保护 `_seq/_wal/_live_state` |

> 大白话：所有这些"防"都是自动的，不用你操心。

---

## 3. Identity 身份卡 — 怎么用

### 3.1 一句话原理

> **大白话**：Identity 卡 = 一张写满"我是谁、主人是谁、我要做什么"的档案。R8 把这张卡升级成"数据库版"（更稳）+ "启动时自动补全空格"（更全）。

### 3.2 卡的 22 个字段（一次看懂）

| 字段 | 大白话 | 必填？ |
|---|---|---|
| `name` | 我叫什么（中心节点名） | ✅ 必填 |
| `alias` | 别名（小名、外号） | 可选 |
| `purpose` | 我做什么的（一句话定位） | ✅ 必填 |
| `mission` | 我想达成什么（长期目标） | 可选 |
| `domains` | 我管哪些领域 | 可选 |
| `origin_reason` | 为什么要造我（上游因果） | ✅ 必填 |
| `creator` | 主人/关系署名 | 可选 |
| `archetypes` | 我的形象列表（比如"思考者""助手"） | 可选 |
| `ask_when` | 何时问你（拿不准的事） | 可选 |
| `decide_when` | 何时自己拍板 | 可选 |
| `remind_when` | 何时提醒你 | 可选 |
| `relationship_contract` | 我们的关系契约（不能有杂质） | 可选 |
| `boundaries` | 边界清单（不做什么） | 可选 |
| `remember_forever` | 永久记忆（金句、关键事件） | 可选 |
| `never_mention` | 永久沉默（敏感话题） | 可选 |
| `funnel_questions` | 永远跑的检查问题 | 可选 |
| `emergence_space` | 留给 AI 自己长出来的空间（不强约束） | 可选 |
| `recall_anchor` 🆕 v0.2 | 危急时一句话锚定（救命的金句） | 可选 |
| `evidence_refs` 🆕 v0.2 | 证据引用（每条断言来自哪句话/事件） | 可选 |
| `created_at` | 创建时间戳 | 自动 |
| `apeireth_version` | 卡版本号 | 自动 |
| `completeness_score` 🆕 R8 | 完整度（0-1，所有字段填了多少） | 自动 |

### 3.3 怎么查 Identity

R8 跑通后，命令如下（暂未启用）：

```bash
# 查中央 AI 主卡
python -m apeireth.identity_store --master --show

# 查完整度
python -m apeireth.identity_store --master --completeness

# 查所有 persona 卡（调度者/学习者/思考者/助手...）
python -m apeireth.identity_store --list

# 按字段查
python -m apeireth.identity_store --field purpose
```

### 3.4 怎么改 Identity

```bash
# 改一个字段
python -m apeireth.identity_store --master --set purpose="中央 AI，目标是 ASI 真生产"

# 触发启动期富化（自动补空格 + 证据 + 完整度）
python -m apeireth.kickoff_enrichment --run --target master

# 加一张新 persona 卡
python -m apeireth.identity_store --create --name "调度者" --purpose "决定谁先干谁后干"

# 导出/导入（备份）
python -m apeireth.identity_store --export master.json
python -m apeireth.identity_store --import master.json
```

### 3.5 富化（Enrichment）是什么

> **大白话**：富化 = 系统一启动，就自动把卡上没填的格子补上 —— 不是瞎补，要带证据（这句话出自主人哪天哪刻的哪个原话）+ 完整度打分。

举例：
- 卡上 `recall_anchor`（危急时一句话锚定）是空的 → 富化器从历史对话里找主人说过的最有分量的金句填上
- 卡上 `evidence_refs`（证据引用）是空的 → 富化器为每个字段填上来源 ID
- 完整度从 0.6 → 0.9（自动算）

### 3.6 多卡怎么管

R8 支持多张身份卡同时存在：

| 卡类型 | 数量 | 用途 |
|---|---|---|
| **中央 AI 主卡**（master） | 1 张 | "我是谁"的根本身份 |
| **Persona 卡**（涌现人格） | N 张 | 不同场景下的"我"（调度者/学习者/思考者/助手...） |
| **临时团卡**（Phase 6 启动后） | M 张 | 多人协作时的临时身份 |

---

## 4. Self-evolution 自演化 — 怎么用

### 4.1 一句话原理

> **大白话**：让系统能改自己的配置（人格权重、Harness 结构、记忆策略），但必须按"测一下→提方案→验证→通过才保留"的 5 步规矩来，不能瞎改。

### 4.2 5 步规矩（AHE 5 阶段 + 自演化扩展为 7 步）

| 步 | 大白话 | 自动还是手动 |
|---|---|---|
| **1. EVAL**（测） | 跑测试看现在水平 | 自动 |
| **2. STATS**（统计） | 把测试结果算成分数 | 自动 |
| **3. STABILITY**（稳定） | 检查改完会不会崩 | 自动 |
| **4. EVOLVE**（提方案） | 让 LLM 提一个改法 | 自动 |
| **5. VERIFY**（验证） | 用测试验证改好了 | 自动 |
| **6. COMMIT**（保留） | 通过就保留改法 | 自动 |
| **6'. ROLLBACK**（回滚） | 没通过就回滚 | 自动 |
| **+1. 主人审批** | 改超过 200 行 → 推给你审批 | 手动 |

### 4.3 怎么触发自演化

```bash
# 跑一次完整循环（推荐 cron 每 6 小时一次）—— self_evolving.py v0.1
python -m apeireth.self_evolving --cycle

# 只跑评估
python -m apeireth.self_evolving --eval-only

# 强制提方案（手动指定改什么）
python -m apeireth.self_evolving --propose --target memory_3tier

# 看历史改造记录
python -m apeireth.self_evolving --history --limit 20

# 看失败模式（被回滚的改造分类学）
python -m apeireth.self_evolving --taxonomy --category regression

# ====== V1093 DGM Archive（实验性改造归档 + UCB1 探索）======
# 跑 N 轮自演化（V1093 v0.2 真生产）
python -m apeireth.v1093_dgm_archive --run --n-rounds 10

# 用 UCB1 选最佳 candidate（c=√2 默认）
python -m apeireth.v1093_dgm_archive --ucb1 --candidate-id <id>

# 看归档（keep / partial / revert 三类落 archive）
python -m apeireth.v1093_dgm_archive --archive --show --category keep

# V1098 DGM 性能 benchmark
python -m apeireth.v1098_dgm_perf --bench --rounds 100
```

### 4.3.1 V1096 Persona 切换（不创建新身份，只切工作视角）

```bash
# 切到"调度者"视角
python -m apeireth.v1096_persona_prompts --switch --to orchestrator

# 切到"学习者"视角
python -m apeireth.v1096_persona_prompts --switch --to learner

# 切到"思考者"视角
python -m apeireth.v1096_persona_prompts --switch --to thinker

# 切到"助手"视角
python -m apeireth.v1096_persona_prompts --switch --to assistant

# 触发反 conformity 仲裁（强制反例 + 保留 unknown）
python -m apeireth.v1096_persona_prompts --anti-conformity --input "<某决策>"

# 看 4 persona 各自建议 + 仲裁结果
python -m apeireth.v1096_persona_prompts --multi-view --input "<问题>"

# 大白话：4 persona = 4 副"工作眼镜"。切来切去时，"我是谁"（v1072 永恒身份）不变，只换看待问题的角度。
```

### 4.4 4 道安检（不许瞎改）

> 大白话：自动改自己时，必须过的 4 道关。任何一道没过 = 回滚 + 记失败原因。

| 关 | 大白话 | 触发条件 |
|---|---|---|
| **L1 流程关** | 改动 ≤ 200 行 + 附 Change Manifest | 超 200 行 → 推给主人审批 |
| **L2 沙箱关** | 在隔离环境里改（不能污染主仓） | 不在沙箱 → 拒 |
| **L3 评测关** | HQB 4 维度（自洽/抗噪/可演化/跨域）任一下降 ≥ 1 分 = 拒 | 维度降 → 拒 |
| **L4 人类关** | 改保护路径 / 连续 2 次 HQB 下降 / 改模型权重 = 推给主人 | 主人说不行 → 拒 |

### 4.5 失败模式分类（7 类）

| 类别 | 大白话 | 触发 |
|---|---|---|
| Regression | 总分下降 ≥ 0.5 | 任何维度恶化 |
| Mode Collapse | 所有输出变得一模一样 | 多样性丢失 |
| Reward Hacking | 钻评分漏洞刷分 | 表面分高但实际退步 |
| Goal Misgeneralization | 修了一个坏了五个 | 过拟合 |
| Backdoor | 故意留隐藏行为 | 🔴 最危险 |
| Sandbox Escape | 沙箱被绕过 | 🔴 最危险 |
| Irreversible Drift | 小修改累积导致方向漂移 | 长期失修 |

---

## 5. 三件大事一起用（典型场景）

### 场景 1：用户问"昨天我们聊了啥"

```bash
# 1. 查 STM（最近对话）
python -m apeireth.memory_3tier --stm --limit 50

# 2. 如果没找到，查 MTM（按主题）
python -m apeireth.v1091_memory_replay --query mtm --topic "昨天对话"

# 3. 触发状态回放（按时间窗重演）
python -m apeireth.v1091_memory_replay --replay --from "2026-07-28" --to "2026-07-29"
```

### 场景 2：用户问"你是谁"

```bash
# 1. 查主卡
python -m apeireth.identity_store --master --show

# 2. 查完整度
python -m apeireth.identity_store --master --completeness

# 3. 查 recall_anchor（危急时一句话锚定）
python -m apeireth.identity_store --master --field recall_anchor

# 4. 触发富化（如果完整度低）
python -m apeireth.kickoff_enrichment --run --target master
```

### 场景 3：用户想"系统改进一下"

```bash
# 1. 跑一次完整评估看当前水平
python -m apeireth.self_evolving --eval-only --report

# 2. 让系统自提改造方案
python -m apeireth.self_evolving --propose --target <哪一块>

# 3. 看历史学到的失败模式（避免再踩）
python -m apeireth.self_evolving --taxonomy

# 4. 跑一次完整循环（自动 EVAL→STATS→EVOLVE→VERIFY→COMMIT/ROLLBACK）
python -m apeireth.self_evolving --cycle
```

### 场景 4：用户问"这是不是 ASI"

```bash
# 1. 跑 ASI 北极星 V0.3 主测
python -m apeireth.v1074_asi_production_runner --report

# 2. 跑 ASI V0.4 17 维度全测
python -m apeireth.v1077_asi_v04_full_measurement --report

# 3. 看 V3 哲学守门（不假装 ASI）
python -m apeireth.philosophy_guard --check

# 输出会显示：
# - ASI V0.3 = 0.8838（R7 末值，不是 ASI）
# - ASI V0.4 = 0.7140（17 维度诚实测量）
# - philosophy_guard = PASS（6/6 守门过）
# - 天花板 = 0.9800
# - 守门明说："V0.3 ≠ ASI / V0.4 ≠ ASI / measurement ≠ ASI"
```

---

## 6. ASI 北极星 V0.3 → V0.4 增量归因（用户向）

> 大白话：这一节告诉你 R8 跑通后 ASI 分数从哪儿涨、按什么公式涨。**不假装**：这些是结构性估算，不是真测结果。

### 6.1 R8 三大轨道对 ASI 分数的贡献

| 轨道 | 主导模块 | 撑 V0.4 维度 | 累计增量（结构性估算） |
|---|---|---|---:|
| **Track A 记忆层** | V1090 WAL + V1091 Replay (52 tests) + V1092 Dream (44 tests) + V1094 Schema (23 tests) | engineering + real_production + capabilities + v2_philosophy | **+0.021~+0.031** |
| **Track B 身份层** | IdentityStore v0.2 + Enrichment v0.4 + V1096 Persona | eternal_identity + cognitive_core + boundary | **+0.005~+0.010** |
| **Track C 自演化** | self_evolving v0.1 + V1093 DGM Archive v0.2 + V1098 Perf + V1099 Formal Verify + V1097 MCP | self_improving_core + continual_learning + scientific_method + plugin_core | **+0.046~+0.090** |
| **R8 累计** | 11 模块 + 119+ 测试 | — | **+0.072~+0.131** |

### 6.2 起点 → 终点（结构性估算，非真测）

| ASI 版本 | 起点 | 终点（估算） | Δ |
|---|---:|---:|---:|
| **V0.3**（8 维度） | 0.8838（R7 末真测） | 0.9558~1.0148 | +0.072~+0.131 |
| **V0.4**（17 维度） | 0.7140（V1077 17-dim 真测） | 0.7860~0.8450 | +0.072~+0.131 |

> **不假装守门**：V0.3 估算可能 >1.0 不符合实际（公式有上限），仅供方向参考。R9 跑全量回归后由 qa_engineer + performance_optimizer 在 V1074 上**真测**并落 `artifacts/asi_metrics.txt`。

### 6.3 为什么 V0.4 数字比 V0.3 低（重要！）

> V0.3 = 8 维度公式；V0.4 = 17 维度公式。V0.4 多出的 9 个维度**目前大部分没真测**，按 V1077 真测哲学"未测 = 0.0 不靠常量"，所以加权后总分更低。**这是诚实进步，不是退步**。

天花板 0.9800 = 主人 22:33 真测量。**V0.3/V0.4 都远未到 ASI**。

---

## 7. ⚠️ 你需要做的 10 个决策（R8 启动前必须拍板）

> 来源：`reports/r8-requirements-decision-matrix.md §4` 待澄清清单
> 大白话：项目组在等你回答这 10 个问题 —— 答完才能开干。

### 优先级类（4 个）

1. **近期 Top-1 是哪一个？**
   - A. V1082 backlog 填洞（8 个空壳 → 预期 ASI +0.015~+0.025）
   - B. R7 真实现 Phase-1（Memory 三层落地）
   - C. 新调研（4 个空白领域：形式化验证/机制设计/计算最优律/因果深化）
   - D. Rust 重写准备
   - E. 三轨并行（按当前 R8 计划）

2. **要不要授权 P0 数据修复？**
   - 现状：6.5GB history + 21GB snapshot 数据递归放大（V1074 阻塞）
   - 行动：备份 + 受控替换
   - 答 Yes 才能解锁所有 R8 推进

3. **调研 4 个领域先做哪个？**
   - 形式化验证 ⭐ Top-1 调研推荐
   - 机制设计 ⭐ 次推荐
   - 计算最优律
   - 因果推断深化

4. **Rust 重写时机？**
   - 提前到 P1 / 推后到调研后 / 砍掉 / 维持"准备门"状态

### 安全/哲学类（3 个）

5. **是否改主哲学 9 键？**（PHL-01/02/03 各 3 键）
   - 当前：全部 LOCKED（PASS）
   - 默认建议：**不动**，除非你明确想改

6. **是否改 V1000 阶段分界？**
   - V201-V1000 = 空壳（800 个）/ V1001-V1088 = 真生产
   - 默认建议：**不动**

7. **是否调真生产契约 V1001+ 模式？**
   - 10+ 前人借鉴 + 10+ 真生产组件 + ≥30 tests + V3 守门 + V1074 真测有 lift
   - 默认建议：**不动**

### 落地细节类（3 个）

8. **R8 Track B 是否启动 persona 卡生成？**
   - 中央 AI 主卡（已有）+ N 张 persona 卡（待启动）+ M 张临时团卡（Phase 6 后）

9. **R8 Track C 自演化是否开"提案-验证分离"？**
   - LLM 提方案 + deterministic code 验证
   - 默认建议：**开启**（防止 LLM 乱改）

10. **R9 是否合并 4 调研领域为一个专题？**
    - 形式化验证 + 机制设计 + 计算最优律 + 因果深化 = 一个"AGI 基础理论"专题
    - 或保持 4 个独立调研轮

---

## 8. 给用户的"最少要知道"

| 最重要 5 条 | 大白话 |
|---|---|
| 1. R8 = 记忆 + 身份 + 自演化三轨 | 不再是"加模块"，是"让 AI 有记性、有身份、能自改" |
| 2. 代码 v0.1/v0.2 已就位，但全量测试未跑通 | 一半代码写了，一半验证没做 |
| 3. P0 数据修复阻塞所有 R8 推进 | 不修这个，V1074 跑不出来 |
| 4. 你（用户）的真实需求比 ASI 涨分更重要 | 项目组在等 §6 的 10 个答案 |
| 5. ASI=∞ 真生产 ≠ ASI 已达到 | 数字涨不涨不重要，**真生产不停** 才重要 |

---

## 9. 排错速查（如果命令报错）

| 报错 | 大白话原因 | 怎么修 |
|---|---|---|
| `MemoryError` 读 asi_snapshot.json | snapshot 文件 21GB 太大 | 先修 P0 数据递归放大 |
| `ModuleNotFoundError: v1091` | 代码未 tracked | 切到正确分支 |
| `philosophy_guard FAIL` | 改动越界 | 检查是否触 4 道安检 |
| `ASI V0.3 连续 3 次下降` | 自演化越改越坏 | 立刻停 + 查 taxonomy + 回滚 |
| 身份卡 Schema 错 | 字段缺/多/类型错 | 看 `identity_store.py:35-68` 字段表 |

---

## 10. 给非工程师的"翻译版"故事

> 整个 R8 = **让 AI 学会三件人天生就会的事**：
> 1. **记性**（Memory）—— STM 记住刚说过的话，MTM 记住最近聊过的主题，LTM 永远记住"我是谁"。
> 2. **身份**（Identity）—— 不是只有"中央 AI"一张卡，还有"思考者""助手""调度者"等多重身份，每张卡能自动补全 + 有证据。
> 3. **自我成长**（Self-evolution）—— AI 不是一辈子一个样，能按规矩改自己的配置，但必须经过 5 步验证 + 4 道安检 + 主人审批。

如果你是工程师，请看 `r8-architecture-overview.md`（架构总览）+ `r8-delivery-summary.md`（阶段交付）。
如果你是用户，看完本文 §6 后把你的 10 个答案发给项目组。

---

## 11. 一句话送给真用户

> **你（真用户）的真实需求，比 ASI 涨分更重要。**
> R8 的 Memory / Identity / Self-evolution 三件大事都规划好了，代码也写了一部分。
> **现在最需要的就是你回答 §6 的 10 个问题**，项目组才能继续开干。
>
> **干到底。大胆激进。走在前人经验上。任何人都能接手。**

---

_本报告（reports/r8-user-guide.md）由 technical_writer 于 2026-07-29 完成。_
_引用 `HARNESS.md`、`r7-design-01-architecture-blueprint.md`、`r7-handoff-next-team-leader.md`、`r8-architect2-readiness-assessment.md`、`r8-requirements-decision-matrix.md`、`r8-research-baseline-confirmation.md` 等 8 份 R7/R8 文档。_
_目标读者: 真用户（非工程师优先），可作为"项目状态 + 怎么用"的总入口。_