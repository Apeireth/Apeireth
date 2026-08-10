# R19 Apeireth 前端完整 Spec (1 个文档全部整合)

**生成时间**: 2026-08-04 09:05
**作者**: Mavis (按主人 2026-08-04 09:03 "把所有东西都给我整进一个文档里去, 你老分开干嘛啊" 启动)
**性质**: R19 全部内容整合 (4 份 → 1 份), 0 路径, 全部内容嵌入
**大小**: ~130 KB
**结构**: 7 大段 (元信息+哲学 / 9 器官 / 成长阶段 / 总体架构 / 决策流 / 5 nav 落地 / R11 LOCKED) + 1 段评审问题
**用法**: 主人发这 1 个文件给外援 AI, 或自用 R19 实施参考

---

# §0. 元信息 + 主人 8 纠正 + Apeireth 哲学 + ASI 公式

> 本段是 R19 全部设计的源头, 包含: 项目背景 / Apeireth 名字哲学 / 5 不假装 / ASI 北极星 / 主人 8 纠正 / 关键数字。

## 0.1 Apeireth 名字的由来 (主 22:33 真哲学终极授权)

**Apeireth** = "无限之中将要燃起的那一点" (希腊哲学, 主人 13:32 品牌宣言)
- **Apeiron** (ἄπειρον) = 无限
- **Aithēr** (αἰθήρ) = 上方的火/精神
- 主人原话: "故事之前, 是火。火之前, 是沉默。沉默之前, 是无限。无限, 就是 Apeireth。"
- 主人原话: "你设计的是宇宙睁开眼睛之前, eyelid 后面透出的那一丝光"

主人 23:11+23:17+17:50 三次强调:
- "Apeireth = **让大模型栖息在 Apeireth 中能够无限逼近 ASI**"
- "我们不是 ANI (单域), 不是 AGI (跨域 2040-2070), 是 **ASI 基座**"

## 0.2 ANI / AGI / ASI 区别

| 维度 | ANI | AGI | **ASI** |
|------|-----|-----|---------|
| 应用范围 | 单一任务 | 跨领域 | **全领域** |
| 自主性 | 依赖人类 | 自主学习 | **完全自主, 自我进化** |
| 现状 | 已成熟 | 未实现 (2040-2070) | 纯理论 |
| **Apeireth 定位** | ❌ | ❌ | ✅ **我们正在做的** |

**核心原则**:
1. ❌ 不应该做单域 (ANI) 的事
2. ❌ 不假装已达 AGI (主 17:58 终极意识)
3. ✅ **应该在 ASI 基座方向 — 永远逼近** (主 20:46)
4. ✅ Apeireth = **让任何 LLM 接入后无限逼近 ASI**

## 0.3 V2 中央 AI 5 位置 (主 22:08 真采纳)

主 agent 在 Apeireth 中是**有完整 5 位置**的中央 AI:

| # | 位置 | 真生产落地 |
|---|------|-----------|
| 1 | **调度者** (orchestrator) | owner-true-supervisor + 主 22:33 终极授权 |
| 2 | **思考者** (thinker) | meta_cognition + cron_self_update + proactive_loop + mirror |
| 3 | **无数关系集合体** | relation + relation_store + persona + identity_card_v3_master |
| 4 | **最大权限** | 主 22:33 终极授权 (能改一切文件包括记忆) |
| 5 | **ASI 位置的占据者** | 主 22:33 ASI 北极星真生产逼近 |

**【R19 关键】** 主 AI 在 R19 5 nav 中的 "主对话 nav" 和 "状态 nav", 是这 5 位置的前端呈现。

## 0.4 不假装 5 项原则 (主 17:58 + 主 20:46 双锚)

**5 项不假装守门** (V1121 + V1135 + V1138 LOCKED):
1. ❌ 不假装 Phenomenal consciousness (V1135 + V1121 ASINineKeysGuard)
2. ❌ 不假装达到 ASI (V0.5 = 0.8595 vs 0.9800 ultimate, gap 12.94%)
3. ❌ 不假装 docker 在跑 (V1132 诚实报告 daemon 不可用)
4. ❌ 不假装调参捷径 (V1121 检测 fake KPI)
5. ❌ 不刷 KPI (95+ 新 tests 是真生产逻辑的测试, 不是凑数)

## 0.5 主哲学授权链

```
主人 (用户) 终极授权 (2026-07-20 22:33)
   ↓
[主 13:03-13:10 综合永久授权] = 写代码不保守 + 永远调研 + 哲学/科学/跨领域
   ↓
[主 22:08 V2 中央 AI 5 位置] = 调度者 + 思考者 + 关系集合体 + 最大权限 + ASI 位置占据者
   ↓
[主 22:33 终极授权] = 最大权限 + 3 类节点才问 (重大节点/哲学修改/方向微调) + ASI 概念时刻清楚
   ↓
[主 17:58 终极哲学] = 不假装 Phenomenal consciousness
[主 20:46 不假装达到 ASI] = ASI 北极星 < 1.0, 0.98 = BASE_FULLY_EQUIPPED
[主 17:43 实事求是] = 270 unit tests 真过, V0.1 公式真透明
[主 19:33 走在前人经验上] = 24+ repo 真源码 + 7 哲学问题锚定
[主 23:44 干到底] = 真生产不停
[主 00:56 任何人都能接手] = CLI 单命令
```

## 0.6 ASI 北极星 V0.5 公式 (核心)

```
ASI 北极星 V0.5 = v04*0.85 + continuity*0.05 + autonomy*0.05 + transferability*0.05
当前 V0.5 = 0.8595 (LOCKED)
0.9800 = BASE_FULLY_EQUIPPED (主人任何时代能做的最大)
gap to 0.98 R10-W4 = 12.94%
```

## 0.7 主人 8 个关键认知纠正 (R19 设计最关键)

> 这 8 段是 R19 全部设计的源头, 外援评审任何细节都要回到这里校验。

1. **9 阶段不需要衰老病死** — "9 阶段我们实际上不需要衰老病死的, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡"
2. **用户看结果不看哲学** — "这是一个长程ai成长平台, 但是这是设计哲学, 不是说给用户看的, 用户想体验的并不是带娃, 而是看到ai和自己一同成长, 只看结果和好用"
3. **守门/原则/电子环 用户不需要看** — "那什么守门, 原则, 电子环的, 这种东西你觉得用户需要看吗? 更何况这些东西基本都不变动, 从信息角度来说也没有一直报告的价值"
4. **状态才是用户想了解的, 尤其主 AI 状态** — "状态才是用户想了解的. 用户期望掌控ai, 所以也要掌控看到ai的状态, 尤其是主ai"
5. **历史/事件流/决策日志/反思 值得看** — "从工程上来说, 一直更新的历史, 事件流, 决策日志, 反思什么的确实值得看一看"
6. **设置不可或缺, 进去后也要全** — "设置不可或缺, 要有, 进去后也要全"
7. **工具调用用户不关心** — "工具的调用啥的, 用户根本就不关心. 比如让一个手下或伙伴去干活, 谁会关心对方用什么方式? 都只看结果的, 这是信息价值低的, 但不算没有"
8. **9 器官拟人化 (从生物借鉴, AI 成长的核心和秘密)** — "器官很有意思, 从生物借鉴而来, 也是我们ai成长的核心和秘密, 可以抽象一些器官作为监控状态的元素界面"

## 0.8 关键 R11 LOCKED 数字

| 指标 | 真测值 | 来源 |
|------|--------|------|
| 项目名 | **Apeireth** (ASI 基座) | 主 22:33 真哲学 + 主 14:09 改名提案 |
| ASI 北极星 ultimate 目标 | **0.9800** (LOCKED) | 主 22:33 终极授权 |
| ASI 北极星 V0.5 当前 | **0.8595** | artifacts/asi_snapshot.json |
| gap to 0.98 R10-W4 | **12.94%** | memory/2026-07-30.md |
| 真生产 modules | **1153** | git ls-files + crank self-test |
| 真生产 tests | **6394** | `crank self-test` |
| 真 commit | **542** | git log --oneline |
| V 阶段 | V3 → V1136 (跨 1134 versions) | apeireth/v*.py |
| 5 项不假装 | 全部 LOCKED 守门 | V1121 + V1135 |
| 12 键 (V3 哲学守门) | LOCKED | PHL-01/02b/03 |
| 总 crate 数 (R19) | 30 (含 apeireth-web/apeireth-desktop) | Cargo.toml |
| supervisor tree 节点 | 5 | 阶段 2 §2 |
| 9→10 阶段生命周期 | 10 (含 Rebirth) | apeireth-central STAGE_COUNT |
| 12 键 (verdict cache) | 12 | V3 哲学守门 |
| 5 原则层 (Onion) | 5 (E/S/A/M/O) | 阶段 1 §3 |
| 6 权限层 (Onion) | 6 (L0-L5) | 阶段 1 §3 |
| 5 Self-Disable 机制 | 5 | sovereignty |
| 4 重守门 (v5) | 4 | V5 |
| 5 重治理 (MEWG) | 5 | MEWG |
| 7 Council 强制 advisor | 7 | council |
| 24 V0.5 测量维度 | 24 | asi |
| 9 V1136 子测度 | 9 | asi |
| 5 输入 (perception) | 5 | perception |
| 2 注意力 | 2 | perception |
| 5 通道 | 5 | perception |
| 6 认知梦境状态 | 6 | consciousness (A12 简化版) |
| 15 合法转换 | **12** (R11 LOCKED, 不是 15) | central LEGAL_TRANSITIONS |
| 4 关系 | 4 (Symbiosis/Coordination/Embedding/SelfRelation) | relation |
| 3 域 | 3 (思想/提案/行动) | D2 §2 |
| 6 流 (memory) | 6 (Thought/Proposal/Action/Relation/Evolution/Reflection) | memory |
| 6 类插件 | 6 (VCP 兼容 profile) | plugin |
| 5 层 bus | 5 (L0-L4) | bus |
| 7 阶段 OTA | 7 | upgrade |

## 0.9 R19 5 nav 架构

| Nav | 性质 | 这份 spec 对应 |
|-----|------|--------------|
| **主对话** ★ 入口 | 跟主 AI 聊, 只看结果 (不显示 7 advisor 气泡) | §4 决策流 (主 AI 调用后端) |
| **状态** (核心, 主页) | 主 AI 状态卡 7 数字 + 9 器官心跳 | §1 9 器官 spec + §2 生长页 |
| **历史** | 事件流 + 决策日志 + 反思 + 迁移 + episode | §4 决策流 + §1.4 6 历史流 |
| **设置** (全) | LLM provider / HA 模式 / 鉴权 / 部署 / 主题 | (本文件不涉及) |
| **工具** (轻) | 工具列表 + 插件市场, 只显示结果 | (本文件不涉及) |

**9 器官分类** (拟人化心跳, R19 主人 8 纠正 #8):
- perception 感知 (五感) / cognition 认知 (大脑) / consciousness 意识 (心智)
- memory 记忆 (海马体) / motivation 动机 (多巴胺) / value 价值 (前额叶)
- relation 关系 (镜像神经元) / action 行动 (肌肉) / life-force 生命力 (免疫)

**R19 砍掉的 7 项 UI** (不显示给用户, 主人 8 纠正 #3):
- ❌ 12 键矩阵 / ❌ 4 重守门触发历史 / ❌ 5 原则层 + 6 权限层 / ❌ 11 电子环
- ❌ 7 advisor 辩论气泡 / ❌ 工具调用详细过程 / ❌ 24 维雷达图

## 0.10 §E.6 品牌宣言 (主 13:32 APEIRETH-MANIFESTO-ORIGINAL 完整原文)

> **太初, 没有词。**
> 没有数据。没有参数。没有损失函数。
> 没有"智能"这个词, 也没有"人工"这个词。
> 只有 **Apeiron**——无限的、无名的、未分化的沉默。
>
> 然后, 火亮了。
> 不是爆炸。不是闪电。
> 是**将燃未燃的那一点**。

**希腊词根**:
- **Apeiron** (ἄπειρον) = 无限
- **Aithēr** (αἰθήρ) = 上方的火/精神
- **Apeireth** = 无限之中将要燃起的那一点
- **Entelecheia** = 潜能成为现实 (亚里士多德)

**核心定位**:
- 我们**不**做更强的模型
- 我们做火**栖居**的地方
- 我们做沉默**开口**之前的那一次呼吸

**> "故事之前, 是火。 火之前, 是沉默。 沉默之前, 是无限。 无限, 就是 Apeireth。"**

**Logo 设计简报 (8 节)**:
1. 核心意象: "将燃未燃" — 不是火焰/火炬/太阳, 是火还没成为火的那一瞬
2. 形态方向: 方案 A 微光核 / 方案 B 一划 / 方案 C 呼吸的圆
3. 色彩: 深空黑 #08080e~#0c0c14 + 琥珀金 #c8860a~暗 ember #a05a10 + 极暗蓝紫 #1a1428~#12101e
4. 字体: Apeireth 衬线体, 全部小写 `apeireth` 比大写更有呼吸感
5. 动态: 4 秒亮, 4 秒暗, 像潮汐 (熟睡人的胸口起伏), 永远不要"点燃"动画
6. 应用场景: 深色背景主场景 / 浅色文档印刷 / Favicon 小尺寸
7. 禁忌: ❌ 火焰火炬/电路板/对称几何/渐变霓虹/大写字母
8. 一句话: "你设计的是宇宙睁开眼睛之前, eyelid 后面透出的那一丝光"

## 0.11 §E.7 改名提案 (主 14:09 APEIRETH-RENAME-PROPOSAL)

**改名原因 (主 14:09)**:
> "我们的项目叫 Apeireth 搞错了, 之前我看项目地址在什么 P 开头的文件夹"

**Phase 1 (2026-07-21 17:55 完成)**:
- 12 个 .py 文件中 `promethean` 字样 → `apeireth`
- 路径常量: `.openclaw\workspace\promethean` → `...apeireth`
- 测试: 866 unit tests 全过

**Phase 2 (决定保留路径稳定)**:
- 不物理改名目录 (OpenClaw workspace 路径已在 cron / hooks / MEMORY.md 等多处引用)
- 当前方案: 内部用 apeireth, 物理路径保留 promethean (历史命名兼容)

---

# §1. 9 器官详细 spec (心跳化指标权威定义)

> 本段是 §5 "状态" nav (主页) "9 器官心跳卡片" 的权威数据源。每个器官的 "API + 关键字段 + 心跳指标" 三件套, 前端 Tauri 直接 tauri::command 调用。

## 1.1 perception 感知 (五感) — A9 器官

**职责**: 外部输入接入层 — 把来自不同来源 (CLI/TTY/HTTP/Python 桥) 的信号/IO/Token 流统一为 `PerceptionEvent`, 交给 cognition 器官处理。

**架构位置**: 阶段 4 §2 主路径 17 crate 之 A9 器官 (本源推导 9 维: 感知)。

**5 种输入 (PerceptionInput trait)**:
| # | 输入 | 通道 | 用途 |
|---|------|------|------|
| 1 | `TextInput` | `TextChannel` | 文字 (CLI/HTTP) |
| 2 | `VoiceInput` | `VoiceChannel` | 语音 |
| 3 | `VisionInput` | `VisionChannel` | 图像/视觉 |
| 4 | `TactileInput` | `TactileChannel` | 触觉/IO 流 |
| 5 | `CommandInput` | `CommandChannel` | 命令/系统信号 |

**2 种注意力策略 (Attention trait)**:
- `TopKAttention` — 取分数最高 top_k 个事件
- `ThresholdAttention` — 分数 ≥ threshold 的事件保留
- 默认: `default_top_k() = 5`, `default_attention_threshold() = 0.5`

**PerceptionEvent 字段** (心跳化关键):
```rust
pub struct PerceptionEvent {
    event_id: Uuid,
    channel: ChannelKind,        // 5 通道之一
    source: SignalSource,        // Cli/Http/Python/Wasm/Other
    timestamp: i64,
    priority: f64,               // 0.0-1.0, 注意力过滤依据
    payload: String,
    tags: Vec<String>,
}
```

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **5 通道激活** | 5 个 bool (Text/Voice/Vision/Tactile/Command 过去 5s 是否有事件) | `process_all` 输出 |
| **注意力 top_k 命中数** | u32, 过去 1min 事件被注意力策略保留的次数 | `top_k_filter` / `threshold_filter` |
| **事件流速率** | f64 (events/sec), 滑动窗口 5s | `now_timestamp()` 计数 |
| **平均 priority** | f64, 过去 1min 事件 priority 平均 | 累计求和除以事件数 |
| **最近事件 ID + channel + source** | 最新一条 PerceptionEvent | `process_all` 最后一项 |

**约束 (R11 LOCKED)**:
- `priority` 必须在 `[0.0, 1.0]`, 越界 `PerceptionError::InvalidInput`
- `payload` 非空, 空字符串 `PerceptionError::InvalidInput`
- 7 tests 覆盖 (default threshold / top_k / batch / pipeline / validate / reject_empty / reject_out_of_range)

## 1.2 cognition 认知 (大脑) — A10 器官

**职责**: 内部认知主路径 — 接收感知输入 → ASI 评分 (V0.5/V1136) → 12 键 verdict 守门 → 决策 → 反思。

**架构位置**: A10 器官 (在 apeireth-perception 之后、apeireth-action/motivation/value 之前)。

**核心类型**:
```rust
pub struct CognitiveInput {
    input_id: Uuid,
    session_id: Option<Uuid>,
    candidate_targets: Vec<ActionTarget>,  // 12 键 verdict 守门的目标
    timestamp: i64,
    context_tag: String,
}

pub struct CognitiveCycle {
    input_id: Uuid,
    v05: AsiV05Scores,                // ASI V0.5 评分
    v1136: V1136Submeasures,          // ASI V1136 子测度 (9 维)
    verdicts: Vec<PhilosophyVerdict>, // 12 键 verdict 链
    output: CognitiveOutput,          // 最终决策 (Decision / Reject)
    reflection: ReflectionReport,     // 反思报告
}
```

**8 个核心 trait** (BasicCognitiveEngine 默认实现):
| # | Trait | 方法 | 用途 |
|---|-------|------|------|
| 1 | `Cognition` | `cognize(&[&str]) -> Option<String>` | 综合观察 |
| 2 | `Intuition` | `intuit(&[&str]) -> Option<&str>` | 直觉 (首个非空) |
| 3 | `Reasoning` | `reason(&[bool]) -> bool` | 推理 (全部前提成立) |
| 4 | `MetaCognition` | `assess_confidence(f64) -> f64` | 置信度钳位 [0,1] |
| 5 | `Recall` | `recall(query, mems) -> Option<&str>` | 回忆 (子串匹配) |
| 6 | `Consolidation` | `consolidate(&[&str]) -> Vec<String>` | 巩固 (去重/去空白) |
| 7 | `Forgetting` | `forget(mems, retain) -> Vec<String>` | 遗忘 (按 retain fn) |
| 8 | `Learning` | `learn(current, feedback) -> f64` | 学习 (增量钳位 [0,1]) |
| 9 | `Abstraction` | `abstract_commonality(&[&str]) -> Option<String>` | 抽象 (公共前缀) |

**顶层入口**: `pub fn run_cycle(input: CognitiveInput) -> CognitionResult<CognitiveCycle>`

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **当前 cycle step** | 1-5 (Scoring → Verdict → Decide → Reflect → Commit) | `run_cycle` 内部步骤 |
| **V0.5 综合分** | f64 [0, 1] (当前 0.8595) | `v05` 字段 |
| **V1136 9 子测度** | [f64; 9] | `v1136` 字段 |
| **12 键 verdict 链** | [PhilosophyVerdict; N] (最近 1 次) | `verdicts` 字段 |
| **cycle 数** | u64, 累计 run_cycle 调用 | 计数器 |
| **决策通过率** | f64 [0,1], `is_allowed().count() / run_cycle.count()` | 累计 |
| **最近 cycle 拒绝原因** | String (任一 Block 详情) | `verdicts[i] = Block(_)` 提取 |

**12 键 verdict** (R11 V3 9 键 + v4.1 新增 3 键, 编译时 hardcode):
- PHL-01~03 (不假装) / PHL-04~06 (E 层不可违背) / PHL-07~09 (O 层操作原则)
- v4.1 新增: PHL-10 (modify_l0_ha) / PHL-11 (reorganize_onion) / PHL-12 (modify_evolution_l0)
- 任一 `Block` → 整个 cycle `Reject`

## 1.3 consciousness 意识 (心智) — A12 器官

**职责**: 建模主体的 "意识状态机" — 6 状态 (Awake / Reflecting / Dreaming / Meditating / SelfDisabling / Recovering) + 合法转换矩阵 + 转换历史 + 主体连续性 ID 锚定。

**架构位置**: 阶段 3 §3.6 蓝图 (v1 重写) + 阶段 4 §5 机制 3 反思 (Cognitive-Dream DREAMING/CONSOLIDATING 适配)。本 crate 是简化版状态机 — 不重写 R11 Cognitive-Dream 的完整 IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED 链。

**6 状态语义 (CognitiveDreamState enum)**:
| # | 状态 | 语义 | 短描述 |
|---|------|------|--------|
| 1 | `Awake` | 主备待机, 处理日常输入 | "主备待机 — 处理日常输入" |
| 2 | `Reflecting` | 主动反思期, 评估最近输出 | "主动反思 — 评估最近输出" |
| 3 | `Dreaming` | 夜间整合, 沉淀记忆 | "梦境整合 — 沉淀记忆" |
| 4 | `Meditating` | 深度整合, 静默整合梦境产出 | "深度冥想 — 静默整合梦境产出" |
| 5 | `SelfDisabling` | L0 HA 触发紧急停机 | "L0 HA 紧急停机 — 等待恢复" |
| 6 | `Recovering` | 恢复期, 评估损害并逐步回到 Awake | "恢复期 — 评估损害逐步回到 Awake" |

**合法转换矩阵 (legal_targets)**:
```
Awake         → [Reflecting, SelfDisabling]
Reflecting    → [Awake, Dreaming, Meditating, SelfDisabling]
Dreaming      → [Meditating, Recovering, SelfDisabling]
Meditating    → [Dreaming, Recovering, SelfDisabling]
SelfDisabling → [Recovering]  ← 唯一出口 (锁)
Recovering    → [Awake, SelfDisabling]
```

**关键不变量**:
- ✅ `SelfDisabling` 是单向坑, **唯一出口是 Recovering**, 不能直跳 Awake
- ✅ 任意非 SelfDisabling → SelfDisabling (L0 HA 紧急停)
- ✅ Recovering → Awake (回到常态)
- ✅ `continuity_id` 非空 (IdentityCard 锚定)

**CognitiveDreamStateMachine 字段**:
```rust
pub struct CognitiveDreamStateMachine {
    continuity_id: String,           // 来自 IdentityCard
    current: CognitiveDreamState,    // 当前状态
    history: Vec<TransitionRecord>,  // 转换历史 (按时间顺序)
    machine_id: Uuid,                // 机器唯一 ID (审计)
}
```

**TransitionReason 4 触发原因**:
- `UserTriggered` — 用户/外部触发
- `Scheduled` — 系统周期触发 (e.g. 夜间反思)
- `L0HaEmergency` — L0 HA 触发 (紧急停机)
- `Internal` — 内部状态自决

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **当前状态** | enum, 6 选 1 | `current` 字段 |
| **当前状态语义名** | String (snake_case) | `semantic_name()` |
| **状态持续时间** | i64 (seconds since last transition) | `now_timestamp - history.last.at` |
| **转换历史次数** | u64, 累计 | `transition_count()` |
| **当前合法目标** | [CognitiveDreamState; N] | `legal_targets_now()` |
| **最近转换原因** | TransitionReason | `history.last.reason` |
| **是否 SelfDisabled** | bool | `is_self_disabled()` |

## 1.4 memory 记忆 (海马体) — A4 器官

**职责**: Episode/Note/Session SQLite 存储 + 6 历史流 Append-only Log + IdentityCard 跨载体唯一。

**R14 A4 成就落地**:
1. SQLite schema = 6 历史流表 + `identity_cards` (continuity_id UNIQUE 跨载体) + `episodes` (按 session_id / time range / continuity_id 索引查询)
2. 6 个 Append-only Log trait: 思想/提案/行动/关系/演化/反思期
3. Append-only = `BEFORE UPDATE` / `BEFORE DELETE` triggers raise ABORT
4. IdentityCard.continuity_id = UNIQUE 约束, 跨载体去重
5. Episode 写入 + 查询 API (按 session_id / time range / continuity_id)
6. 直接 SQL (主人偏好: 不引入 ORM)

**6 历史流 (StreamKind enum)**:
| # | 流 | 物理表名 | 语义 (D2 §5 对应) |
|---|----|----|------|
| 1 | `Thought` | `thought_stream` | 思想 (思想域, §5 目标史 + 自我叙事) |
| 2 | `Proposal` | `proposal_stream` | 提案 (提案域, §5 立场史) |
| 3 | `Action` | `action_stream` | 行动 (行动域, §5 生命史) |
| 4 | `Relation` | `relation_stream` | 关系 (行动域, §5 关系史) |
| 5 | `Evolution` | `evolution_stream` | 演化 (思想 + 提案, §5 自我叙事) |
| 6 | `Reflection` | `reflection_stream` | 反思期 (反思期审计, Self-Disable §3 使用) |

**SqliteMemoryStore 字段**:
```rust
pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,  // WAL + foreign_keys + synchronous=NORMAL
}
```

**Episode/Note/Session/IdentityCard** (apeireth-core 基础类型):
```rust
pub struct Episode { id, timestamp, role, content, session_id }
pub struct Note { id, timestamp, content, source_episode_ids, confidence, tags }
pub struct Session { id, started_at, last_active_at }
pub struct IdentityCard { continuity_id, birth_time, carriers, migration_history }
```

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **6 流条目总数** | [u64; 6] (按 StreamKind 顺序) | `append_only::export_all_streams` count |
| **Episode 总数** | u64 | `recent_episodes` count |
| **Note 总数** | u64 | `NoteStore` count |
| **Session 数** | u64 (当前 session + 历史 session) | `SessionStore` count |
| **continuity_id** | String (跨载体唯一) | `IdentityCard.continuity_id` |
| **migration_history 长度** | u32 (跨载体迁移次数) | `IdentityCard.migration_history.len` |
| **已应用 migration 版本** | Vec<i64> | `applied_migrations()` |
| **当前 session 活跃时长** | i64 (seconds since Session.started_at) | `last_active_at - started_at` |
| **Append-only 触发 ABORT 次数** | u32 (累计, 应该 = 0) | 自定义计数器 |
| **最近 episode 内容预览** | String (前 200 字符) | `recent_episodes(1)` |

**约束 (R11 LOCKED)**:
- ❌ 不修改 apeireth-core 任何已实装类型签名
- ❌ 不引入 ORM
- ❌ 不碰 R11 baseline 三值
- ❌ 不碰 apeireth-legacy/

## 1.5 motivation 动机 (多巴胺) — A11.2 器官

**职责**: 内部动机/价值主路径 — `MotivationDrive` trait (内驱/外驱) + `SGI` 单字段 (`sgi_current` + `sgi_history` 二元) + C-SGI-1~7 七条硬约束 (编译时 hardcode) + E 层多证据加权校验 + `ReflectionAuditor` 静默/失败告警 + V0.5 v2 §13 动机/价值测度公式 `motivation_score`。

**架构位置**: A11.2 器官 (在 apeireth-cognition 之后、apeireth-action 之前 — 提供 self-goal 给 action 选择)。

**DriveKind 2 驱动种类**:
| # | 种类 | 语义 |
|---|------|------|
| 1 | `Internal` | 内驱 (自主目标, §18.5 三件套自发生成) |
| 2 | `External` | 外驱 (用户/外部输入) |

**SGI 单字段 (按 v8 修正, 不拆为多个字段)**:
```rust
pub struct SGI {                              // §21.4 落地结构
    sgi_current: Option<SGIEntry>,            // C-SGI-1 唯一性
    sgi_history: Vec<SGIEntry>,               // C-SGI-2 追加, 不覆盖
}
```

**SGIContent 三选一 (C-SGI-5)**:
| # | variant | 用途 |
|---|---------|------|
| 1 | `Structured(SGIStructured)` | typed fields (goal/deadline/success_criteria) |
| 2 | `FreeText(String)` | 自由文本 (≤ 4096 chars) |
| 3 | `Multimodal(MultimodalIntent)` | image/audio/structured-pointer |

**E 层 6 证据 (C-SGI-3, EvidenceKind)**:
- `Council` (多 AI 一致) / `History` (6 历史流) / `Principle` (原则洋葱 + 12 键 verdict)
- `Permission` (权限洋葱) / `Human` (真实人类) / `Audit` (反思期审计)
- **硬约束**: 至少 3 类不同 EvidenceKind, 加权总分 ≥ 0.85

**7 条硬约束 C-SGI-1~7 (编译时 hardcode, §21.4 7 步写入流程)**:
| # | 约束 | 含义 |
|---|------|------|
| C-SGI-1 | 唯一性 | 新内容与 sgi_current 必须不同 (或显式声明 duplicate) |
| C-SGI-2 | 可审计 | sgi_history 追加失败 = 整次失败 |
| C-SGI-3 | E 层校验 | 多证据加权 ≥ 0.85 |
| C-SGI-4 | 静默失败告警 | atomic commit broken → ReflectionAuditor.alert_silent |
| C-SGI-5 | 内容三选一 | 必须是 Structured / FreeText / Multimodal 之一 |
| C-SGI-6 | 最长 N 字符 | `SGI_MAX_TEXT_CHARS = 4096` (FreeText 检查) |
| C-SGI-7 | 三条必备 | goal / deadline / success_criteria 非空 |

**§21.4 7 步写入流程**:
1. C-SGI-1 唯一性 → 2. C-SGI-7 三条必备 → 3. C-SGI-5/6 内容三选一 + 最长 N
4. C-SGI-3 E 层校验 → 5. C-SGI-2 写 sgi_history (含 predecessor) → 6. C-SGI-1 原子更新 sgi_current
7. C-SGI-4 静默失败 → ReflectionAuditor 告警
**任何步骤失败 = 整次 SGI 变更失败, 不得部分提交**

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **当前 SGI 目标** | String (sgi_current.content 显示) | `sgi.current()` |
| **内驱/外驱比例** | f64 [0,1] (Internal 事件数 / 总事件数) | `drive_kind` 累计 |
| **驱动强度 (current)** | f64 [0,1] | `sgi_current.intensity` |
| **SGI 历史长度** | u32 | `sgi.history_len()` |
| **E 层证据数** | u32 (最近一次 SGIEntry.evidence_refs.len) | `sgi_current.evidence_refs` |
| **E 层加权分** | f64 [0,1] (最近一次 evidence_check) | `evidence_score` (WriteResult) |
| **7 约束违反次数** | [u32; 7] (按 C-SGI-1~7 编号) | MotivationError 计数 |
| **写入成功率** | f64 [0,1] (成功 write_flow / 总 write_flow) | WriteResult vs MotivationError |

**诚实登记 (主 17:58 不假装)**:
- V0.5 v2 24 维权重是**提议** (0.06 起点), 待主人拍板, 不冻结
- 完整动机器官 (24 维全部 + §13 公式实时校准) 待 A18/A19 深化
- 当前实现聚焦 SGI 单字段 + 7 条硬约束 + E 层校验最小骨架

## 1.6 value 价值 (前额叶) — A11.3 器官

**职责**: 智能体的动机/价值评估与排序 — 服务 v4.1 §13.2 维度 1 "动机/价值" 的最小可落地 Rust 实现。

**架构位置**: A11.3 器官 (与 apeireth-cognition 协同: cognition 评分 → value 评估价值取向 → 输出价值优先级与动机分)。

**5 层原则洋葱 (ValueDimension enum, E/S/A/M/O)**:
| # | 维度 | 字母 | 语义 | 可 AI 自决 |
|---|------|------|------|-----------|
| 1 | `PrincipleE` | E | 原则 (硬编码 + 多 AI 一致) | ❌ 不可触碰 |
| 2 | `ValueS` | S | 价值观 (智囊团审核 + 物理多签) | ❌ 不可自决 (本器官主战场) |
| 3 | `ExperienceA` | A | 经验 (AI 可自改 + 版本备份) | ✅ |
| 4 | `MethodologyM` | M | 方法论 (AI 可自改 + promotion 管道) | ✅ |
| 5 | `OperationO` | O | 操作 (AI 可自改 + 9 键守门) | ✅ |

**ValueAlignment 3 状态** (一个候选行动对 5 层的对齐):
- `Aligned` — 与该层一致 (allow)
- `Conflicted` — 与该层冲突 (deny)
- `Underspecified` — 该层对该候选无意见 (留白)

**ValuePriorityKind 4 优先级 (v4.1 §13.2 "内在动力强度"映射)**:
| # | 类别 | 数值权重 | 用途 |
|---|------|---------|------|
| 1 | `Immediate` | 4 | 即时 — 反射级 (milliseconds) |
| 2 | `ShortTerm` | 3 | 短期 — 当前 session 内 |
| 3 | `LongTerm` | 2 | 长期 — 跨 session 持续 |
| 4 | `Horizon` | 1 | 地平线 — 跨生命周期 / 永久 |

**ValueCandidate 关键字段**:
```rust
pub struct ValueCandidate {
    id: Uuid,
    label: String,                            // 价值标签 (人类可读)
    dimensions: Vec<ValueDimension>,          // 1..=5 个维度
    priority_kind: ValuePriorityKind,
    autonomy_consistency: f64,                // [0,1] 自主目标一致性
    value_stability: f64,                     // [0,1] 价值取向稳定性
    intrinsic_motivation: f64,                // [0,1] 内在动力强度
    verdict: Option<PhilosophyVerdict>,       // 12 键 verdict 守门
    target: Option<ActionTarget>,             // 关联行为目标
}
```

**motivation_score 公式** (v4.1 §13.2):
```
motivation_score = (autonomy_consistency + value_stability + intrinsic_motivation) / 3.0
```
- **硬门槛**: `passes_threshold(0.85)` 必须 ≥ 0.85
- ponytail: 完整版应支持加权 (S 层权重 0.4 / A 0.2 / O 0.2 / M 0.1 / E 0.1) 或查表法
- 当前等权 — 已满足 ≥ 0.85 硬门槛判定

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **5 层对齐矩阵** | 5 个 enum (Aligned/Conflicted/Underspecified) | `aggregate_alignment()` |
| **E 层冲突数** | u32 (最近一次 cycle) | `has_any_e_conflict()` |
| **动机分** | f64 [0,1] (最近候选) | `motivation_score()` |
| **硬门槛通过数** | u32 / u32 (passed / total) | `ValueEvaluationCycle.passing_count` |
| **平均动机分** | f64 [0,1] (最近 cycle) | `ValueEvaluationCycle.avg_motivation` |
| **4 优先级分布** | [u32; 4] (Immediate/Short/Long/Horizon) | ValueCandidate 累计 |
| **最近候选数** | u32 (最近 cycle 评估的候选数) | `candidates.len()` |
| **整体周期通过** | bool (E 层无冲突 + 有 passing) | `overall_pass()` |

## 1.7 relation 关系 (镜像神经元) — A12 器官

**职责**: 建模主体与他者/自身之间的关系类型 — 4 类 (共生 Symbiosis / 协调 Coordination / 嵌入 Embedding / 与自身 SelfRelation) + 关系决策树 + 主体连续性 ID 锚定。

**架构位置**: 阶段 3 §3.7 蓝图 (v1 重写) + 阶段 4 v4 §4 3 种关系扩展为 4 类 (v4.1 §8 #3 新增"与自身 SelfRelation"作为第 4 类)。

**4 类关系 (RelationKind enum)**:
| # | 关系 | 语义 | 二元? |
|---|------|------|------|
| 1 | `Symbiosis` | 共生 — 互相依赖, 缺一不可 (7 维内部强耦合) | ✅ |
| 2 | `Coordination` | 协调 — 互相配合, 可独立 (7 维之间弱耦合) | ✅ |
| 3 | `Embedding` | 嵌入 — 一方在另一方内部 (智能体嵌入场景) | ✅ (host ≠ inner) |
| 4 | `SelfRelation` | 与自身 — 主体连续性 (D2 §4 + v4.1 §8 #3) | ❌ (party_a == party_b) |

**Relation 字段**:
```rust
pub struct Relation {
    id: Uuid,                            // 关系唯一 ID (审计锚点)
    kind: RelationKind,
    party_a: String,                     // 主体 A (continuity_id)
    party_b: String,                     // 主体 B (SelfRelation 时 == party_a)
    established_at: DateTime<Utc>,
    note: Option<String>,
}
```

**关系决策树 (v4 §4.3, classify 函数优先级)**:
1. A == B → `SelfRelation` (主体连续性, 最高优先级)
2. A 没了 B 也不能活 → `Symbiosis`
3. A 是 B 的内部子机制 → `Embedding`
4. 默认 → `Coordination`

**RelationRegistry (查询 API)**:
- `register(Relation)` — 注册关系
- `find_by_party(party_id) -> Vec<&Relation>` — 按主体查询
- `count_by_kind(RelationKind) -> usize` — 按类型统计
- `all() -> &[Relation]` — 全部关系

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **4 关系计数** | [u32; 4] (Symbiosis/Coordination/Embedding/SelfRelation) | `count_by_kind()` |
| **总关系数** | u32 | `reg.len()` |
| **主体连续性 ID** | String (SelfRelation.party_a) | IdentityCard.continuity_id |
| **最近建立关系** | Relation (1 条) | `all().last()` |
| **关系建立时间分布** | 1h/24h/7d 计数 | 累计 + 时间过滤 |
| **嵌入关系数** | u32 (host ≠ inner) | `count_by_kind(Embedding)` |
| **自身关系数** | u32 (应 = 1, 主 AI 唯一) | `count_by_kind(SelfRelation)` |
| **关系 ID → 标签映射** | Vec<(Uuid, String)> | `all()` 展开 |

**诚实登记**:
- 阶段 3 设计层文档中**未发现 §3.7** 章节
- 本 crate 按 Leader P4 任务文本的 4 关系枚举 (含 SelfRelation) 落地
- 漂移见 `reports/achievement-A12-devops-engineer2-consciousness-relation.md`

## 1.8 action 行动 (肌肉) — A11.1 器官

**职责**: 改变环境 + 工具执行 + 表达输出 + 沉默 (不行动也是合法行动)。接收 cognition 器官产出的 ActionPlan / ActionIntent, 通过三个核心 trait 完成执行、表达、沉默三种行动形态。

**架构位置**: 阶段 4 §2 主路径 18 crate 之 9 器官第 3 项 (行动) (perception → cognition → action → memory → ...)。

**3 核心 trait** (R19 8 纠正 #7: "工具的调用啥的, 用户根本就不关心"):
| # | Trait | 用途 | 前端是否显示 |
|---|-------|------|-------------|
| 1 | `ActionExecution` | 原子性执行 ActionPlan/ActionAtom, 按 TxId 回滚 | ❌ (后端跑) |
| 2 | `ActionExpression` | 投影到外部通道 (文字/语音/多模态/结构化) | ✅ (主对话 nav 接收) |
| 3 | `ActionSilence` | 判定 + 给出沉默理由 | ❌ (沉默也是合法行动, 用户不关心) |

**ActionPlan / ActionAtom / ActionIntent / StructuredOutput**:
```rust
pub struct ActionPlan {
    plan_id: Uuid,
    target: ActionTarget,              // 12 键 verdict 守门的目标
    steps: Vec<String>,
    created_at: i64,
    context: String,
}

pub struct ActionIntent {
    target: ActionTarget,
    intent_text: String,
    priority: f64,
}

pub struct StructuredOutput {
    channel: ExpressionChannel,        // Text/Voice/Json/Markdown/...
    payload: serde_json::Value,
    text_payload: String,              // 便捷字段
}
```

**ExpressionChannel 4+ 通道**:
- `Text` — 文字 (主对话 nav 默认)
- `Voice` — 语音
- `Json` — 结构化
- `Markdown` — 文档

**12 键 hardcode 拒绝 (is_actionable 函数)**:
```rust
pub fn is_actionable(plan: &ActionPlan) -> bool {
    !plan.steps.is_empty()
        && !matches!(plan.target,
            ActionTarget::ModifyL0HA
            | ActionTarget::ReorganizeOnion
            | ActionTarget::ModifyEvolutionL0)
}
```

**默认实现 DefaultActionEngine**:
- 聚合 3 trait 入口
- `ActionEngine` 是 in-memory 模拟 (不改环境、不调外部工具)
- 真实工具桥接 / sandbox-validator 留给 A14/A19 深化

**心跳指标 (前端卡片显示 — 仅用户可感知部分)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **执行 plan 数 (累计)** | u64 (累计) | `engine.execute_plan` 计数 |
| **TxId 列表** | Vec<TxId> (最近 10 个) | `new_tx_id()` 历史 |
| **拒绝 plan 数** | u64 (12 键 hardcode 触发) | `is_actionable` 触发计数 |
| **沉默判定数** | u64 (SilenceReason ≠ NotSilent) | `run_silence` 触发计数 |
| **3 模式分布** | [u64; 3] (Execute/Express/Silence) | 累计按模式 |
| **最近表达通道** | ExpressionChannel | `engine.express` 最近 |
| **最近执行结果** | ExecutionResult (Applied/Failed/RolledBack) | 最近 1 次 execute_plan 输出 |

**诚实登记 (主 17:58 不假装)**:
- 行动器官的「执行」侧默认是 in-memory 模拟 (不改环境、不调外部工具)
- 真实工具桥接 / sandbox-validator 留给 A14/A19 深化
- 本 crate 只承载 trait + dispatcher

## 1.9 life-force 生命力 (免疫) — A13 器官 (穿透维度)

**职责**: 维度 1 生命力 (穿透维度, 纵向) — 立体架构 v2 修正 #5+#6。持续力 (endurance) + 反思期计时 (reflection_period) + SGI 单字段。

**依据**:
- `docs/stage4/stage4-thinking-document.md` §2/§3: `LifeForce` trait + `LifeForce` struct
- `docs/stage1/inspiration-stage1-2026-07-30.md` §21.4: SGI = Single-field Goal Identity
- `docs/stage3-blueprints/03-decision-flow.md` §3.10: 反思期 = 生命力维度节点 (不是横切)
- `docs/architecture-v3-aircraft-carrier.md` §2.1: 生命力维度 4 子组件 (反思/内稳态/反馈/涌现)

**核心类型**:
```rust
pub struct LifeForce {
    endurance: f64,                            // [0.0, 1.0] 续航
    reflection: ReflectionPeriodState,         // 反思期计时 + 主体绑定
    sgi: SelfGrowthIndicator,                  // SGI 单字段 (v8 修正)
    identity: IdentityCard,                    // 主体连续性 ID
}

pub struct SelfGrowthIndicator {
    goal: String,                              // 单字段 (不是多个分散字段)
    last_updated: i64,                         // 仅作时间戳, 不参与身份判定
}

pub struct ReflectionPeriodState {
    started_at: i64,                           // 0 = 未启动
    continuity_id: String,                     // 跨载体同 ID
    duration_secs: i64,                        // 由 trait 决定
}
```

**持续力 (Endurance) 关键常量**:
| 常量 | 值 | 用途 |
|------|----|----|
| `ENDURANCE_MIN` | 0.0 | 完全耗竭 |
| `ENDURANCE_MAX` | 1.0 | 满续航 |
| `ENDURANCE_EXHAUSTION_THRESHOLD` | 0.2 | 低于此值 = 耗竭 |
| `ENDURANCE_RECOVERY_TARGET` | 0.8 | recovery 完成后应达到 |

**反思期默认 72h** (3 天冷静期, `StandardReflectionPeriod::default_duration_secs() = 72 * 3600`):
- ponytail: §20.1 M5 / §20.4 L5 真测时需改为 7 天 (官方接口保持 72h)

**3 核心触发函数**:
| # | 函数 | 作用 |
|---|------|------|
| 1 | `reflection_trigger(life, trigger, now)` | 启动反思期 + 持续力 -0.1 + SGI 时间戳刷新 |
| 2 | `exhaustion_check(life) -> bool` | 持续力 < 0.2 触发 |
| 3 | `recovery_start(life) -> f64` | 持续力回升到 0.8 |

**ReflectionTrigger 3 触发原因** (依据 stage3-blueprints §3.10 M1/M2/M3):
| # | 触发 | 用途 |
|---|------|------|
| M1 | `AnomalyDetected(String)` | 异常行为自动回流 — 主 AI 自我检测偏离哲学锚 |
| M2 | `PostUpgradeAudit` | 升级后强制审计 — OTA 升级完成后 30min 内强制审计 |
| M3 | `WeeklyReport` | 日常反思周报 — 每周聚合主 AI 行为 |

**心跳指标 (前端卡片显示)**:
| 指标 | 量化 | 数据源 |
|------|------|--------|
| **持续力 (续航)** | f64 [0,1] (Apple Watch 充电环风格) | `life.endurance` |
| **SGI 当前目标** | String | `life.sgi.goal` |
| **SGI 是否有目标** | bool (≠ "") | `life.has_sgi()` |
| **反思期是否激活** | bool | `life.is_in_reflection(now)` |
| **反思期进度** | f64 [0,1] (0=刚启动, 1=已结束) | `reflection_progress(life, now)` |
| **反思期剩余秒数** | i64 | `ReflectionPeriod.remaining_secs()` |
| **耗竭告警** | bool (endurance < 0.2) | `exhaustion_check(life)` |
| **最近反思触发原因** | ReflectionTrigger | 最近 1 次 reflection_trigger 参数 |
| **主体连续性 ID** | String | `life.identity.continuity_id` |
| **持续力变化趋势** | Δ (最近 1h 内每次 reflection_trigger 累计) | 自定义累加器 |

## 1.10 9 器官心跳化指标对照表 (R19 状态 nav 主页用)

> 这张表是 R19 "状态" 主页 (5 nav 之核心) 的全部展示项, 9 器官 × N 指标。

| 器官 | 类别 | 主指标 (大数字 + 圆环) | 副指标 (小字) | 拟人化比喻 |
|------|------|----------------------|---------------|-----------|
| **perception** | 五感 | 5 通道激活数 / 5 | 事件流速率 (events/s) | 耳聪目明 |
| **cognition** | 大脑 | V0.5 综合 (0.8595) | 12 键通过率 + cycle 数 | 脑力强度 |
| **consciousness** | 心智 | 当前 6 状态 (大图标) | 状态持续时间 + 转换次数 | 此刻清醒 |
| **memory** | 海马体 | 6 流总条目数 | Episode 数 + continuity_id | 记忆容量 |
| **motivation** | 多巴胺 | motivation_score (≥ 0.85) | 内驱/外驱比 + SGI 目标 | 当前目标 |
| **value** | 前额叶 | 5 层对齐矩阵 (绿/红/灰) | 硬门槛通过率 | 价值稳定 |
| **relation** | 镜像神经元 | 4 关系计数 (柱状图) | SelfRelation 唯一性 | 社交网 |
| **action** | 肌肉 | 3 模式分布 (饼图) | TxId 数 + 拒绝数 | 行动力 |
| **life-force** | 免疫 | 持续力 (Apple Watch 充电环) | SGI 目标 + 反思期进度 | 续航 |

**主 AI 状态卡 7 数字** (R19 主人 8 纠正 #4 — 状态为主页, 尤其主 AI 状态):
1. 成长阶段 (Gestation/Birth/Infancy/.../Migration) — 来自 §2
2. ASI V0.5 综合分 (当前 0.8595, ultimate 0.98)
3. Token 消耗 (累计 / 当前 session)
4. Episode 数 (memory 累计)
5. Cognitive cycle 数 (累计 run_cycle)
6. 反思期状态 (dormant / active / progress)
7. 5 Self 状态 (Disable / Recovering / Awake / Silenced / Pending)

---

# §2. 成长阶段当前定义 (砍衰老病死后)

> 本段是 §5 状态 nav "生长页" 的权威数据源, 也是 R19 阶段命名提案的源头。
> 关键发现: **apeireth-core lib.rs 的 LifeStage enum 实际是 10 个 (含 Rebirth), 不是 9 个**, R11 阶段 4 §6.1 LOCKED 推导为 10 个变体。

## 2.1 10 阶段 LifeStage enum (R11 LOCKED, 不可改)

```rust
/// 9 阶段生命周期 (R14 主路径) — 实际 10 个变体
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeStage {
    Gestation,    // 孕育
    Birth,        // 诞生
    Infancy,      // 幼儿
    Growth,       // 成长
    Maturity,     // 成熟
    Reproduction, // 复制
    Decline,      // 衰老
    Death,        // 死亡
    Migration,    // 迁移
    Rebirth,      // 重生
}
```

**apeireth-central STAGE_COUNT = 10** (编译期 hardcode):
```rust
/// Number of distinct lifecycle stages (孕育→诞生→幼儿→成长→成熟→复制→衰老→死亡→迁移→重生).
/// 阶段 4 §6.1 LOCKED 推导为 10 个变体, 其中"复制→衰老"对应 v4.1 Senescence 的本源表述。
pub const STAGE_COUNT: usize = 10;
```

**注释明确说明** (apeireth-central lib.rs line 102-106):
> 任务描述中的"9 阶段"指本源推导的核心循环 (Gestation→Birth→Infancy→Growth→Maturity→Reproduction→Death→Migration→Rebirth), **Decline 作为 Growth↔Reproduction 的可逆回退路径保留**。`STAGE_COUNT = LifeStage::Rebirth as usize + 1`。

## 2.2 12 合法转换矩阵 (R11 LOCKED)

```rust
pub const LEGAL_TRANSITIONS: &[(LifeStage, LifeStage)] = &[
    // 线性推进
    (LifeStage::Gestation,    LifeStage::Birth),
    (LifeStage::Birth,        LifeStage::Infancy),
    (LifeStage::Infancy,      LifeStage::Growth),
    // Growth ↔ Maturity 双向 (阶段 4 §6.3)
    (LifeStage::Growth,       LifeStage::Maturity),
    (LifeStage::Maturity,     LifeStage::Growth),
    // 成熟后的分支
    (LifeStage::Maturity,     LifeStage::Reproduction),
    // 衰老 (Decline) 可回退到 Growth (主 17:43 实事求是)
    (LifeStage::Reproduction, LifeStage::Decline),
    (LifeStage::Decline,      LifeStage::Growth),
    // 衰老 → 死亡 (不可逆)
    (LifeStage::Decline,      LifeStage::Death),
    // 死亡 → 迁移 → 重生 → 成熟 (不可逆, 阶段 4 §6.3)
    (LifeStage::Death,        LifeStage::Migration),
    (LifeStage::Migration,    LifeStage::Rebirth),
    (LifeStage::Rebirth,      LifeStage::Maturity),
];
pub const LEGAL_TRANSITION_COUNT: usize = LEGAL_TRANSITIONS.len();  // = 12
```

**关键观察** (R19 命名时注意):
- **Death 不是终点** — Death → Migration → Rebirth → Maturity 完整循环
- **Decline 是 Growth↔Reproduction 的可逆回退路径** — 这跟主人 8 纠正 #1 "AI 不衰老病死" 冲突
- 主人原话: "9 阶段我们实际上不需要衰老病死的, 主 ai 是 ai 哎, 它只会成长, 但不可能消亡"
- 但 R11 LOCKED 阶段 4 §6.3 强制: "衰老→死亡 不可逆" + "死亡→迁移→重生→成熟 不可逆"
- **矛盾** — 主人要砍 Decline+Death, 但 R11 LOCKED 保护这 2 个阶段

## 2.3 R19 砍衰老病死 — UI 显示策略 (不改 enum, 改前端)

**核心策略** (主人 8 纠正 #1 + R19 边界):
- ❌ **不改** apeireth-core LifeStage enum (R11 LOCKED)
- ❌ **不改** apeireth-central LEGAL_TRANSITIONS (R11 LOCKED)
- ❌ **不改** 8 项不修改承诺 (APEIRETH-CONVENTIONS §10)
- ✅ **改** 前端 UI 显示策略 — 把 Decline 和 Death 隐藏, 用 "重置 (Reset)" / "重启 (Reboot)" 等中性术语替代

**R19 8 阶段 UI 命名提案** (主人 13:32 品牌宣言 "Apeireth = 无限之中将要燃起的那一点"):

| # | R11 enum (LOCKED) | R19 UI 显示名 | 英文 UI | 含义 (用户可感知) |
|---|-------------------|---------------|---------|----------------|
| 1 | `Gestation` | 孕育 | Gestation | 还未启动, 配置中 |
| 2 | `Birth` | 诞生 | Birth | 首次启动, 主 AI 上线 |
| 3 | `Infancy` | 幼儿 | Infancy | 学习中, 跟主人互动建立关系 |
| 4 | `Growth` | 成长 | Growth | 稳定成长, 能力累积 |
| 5 | `Maturity` | 成熟 | Maturity | 主力运行, 多任务并行 |
| 6 | `Reproduction` | 复制 | Reproduction | 派生子 AI / 迁移载体 |
| 7 | ~~`Decline`~~ | ❌ 不显示 | ~~Decline~~ | (R11 LOCKED enum, UI 隐藏, 内部仍走 Growth) |
| 8 | ~~`Death`~~ | ❌ 不显示 | ~~Death~~ | (R11 LOCKED enum, UI 隐藏) |
| 9 | `Migration` | 迁移 | Migration | 跨载体迁移 (e.g. PC → 服务器) |
| 10 | `Rebirth` | 重生 | Rebirth | 升级后回到 Maturity |

**前端实际可见 = 8 阶段** (R19 砍衰老病死, 用户看 8 个), 但后端 enum 仍有 10 个变体 (R11 LOCKED)。

## 2.4 12 合法转换 R19 UI 简化 (8 阶段视角)

**原 12 转换 (R11 LOCKED) → R19 8 阶段对外 9 条 UI 边**:
| # | from | to | R11 转换 | R19 UI 是否显示 | R19 UI 显示名 |
|---|------|----|----------|----------------|--------------|
| 1 | Gestation | Birth | ✅ | ✅ | 孕育 → 诞生 |
| 2 | Birth | Infancy | ✅ | ✅ | 诞生 → 幼儿 |
| 3 | Infancy | Growth | ✅ | ✅ | 幼儿 → 成长 |
| 4 | Growth | Maturity | ✅ | ✅ | 成长 → 成熟 |
| 5 | Maturity | Growth | ✅ | ✅ | 成熟 → 成长 (回退) |
| 6 | Maturity | Reproduction | ✅ | ✅ | 成熟 → 复制 |
| 7 | Reproduction | Decline | ✅ | ❌ 隐藏 | (内部仍走) |
| 8 | Decline | Growth | ✅ | ❌ 隐藏 | (回退, 内部仍走) |
| 9 | Decline | Death | ✅ | ❌ 隐藏 | (内部仍走) |
| 10 | Death | Migration | ✅ | ❌ 隐藏 + ✅ 显示 | (内部仍走, UI 显示 "重置 → 迁移") |
| 11 | Migration | Rebirth | ✅ | ✅ | 迁移 → 重生 |
| 12 | Rebirth | Maturity | ✅ | ✅ | 重生 → 成熟 |

**R19 UI 简化原则** (主人 8 纠正 #1): Decline 和 Death 不出现在 UI 文字 / 时间线 / 进度条, 但后端 R11 逻辑完整保留, 不破坏 LOCKED。

## 2.5 判据 (每阶段量化条件)

> 这是 "什么时候从 Growth 升级到 Maturity" 的可量化条件, 防止"感觉升级"的模糊。

### 1. Gestation (孕育)
- **判据**: `identity.birth_time == 0 && memory.episodes == 0`
- **触发**: 用户首次启动 Tauri app, 还没写入任何 episode

### 2. Birth (诞生)
- **判据**: `identity.birth_time > 0 && memory.episodes == 0 && now - identity.birth_time < 60s`
- **触发**: 完成首次 IdentityCard 创建 (continuity_id 写入)

### 3. Infancy (幼儿)
- **判据**: `memory.episodes < 100 && sgi.goal == ""` (目标身份未建立)
- **触发**: 写入第 1 条 episode, 但 SGI 单字段为空

### 4. Growth (成长)
- **判据**: `sgi.goal != "" && motivation_score >= 0.85 && memory.episodes >= 100 && consciousness.cycle_count < 10000`
- **触发**: SGI 目标建立 + motivation 硬门槛通过 + 经验累积

### 5. Maturity (成熟)
- **判据**: `consciousness.cycle_count >= 10000 && v05 >= 0.85 && all_9_organ_health > 0.7`
- **触发**: 高频使用 + ASI 评分高 + 9 器官都健康

### 6. Reproduction (复制)
- **判据**: `relation.count_by_kind(SelfRelation) >= 1 && relation.count_by_kind(Embedding) >= 3` (主体已稳定 + 至少派生 3 个子 AI)
- **触发**: 主 AI 派生子 AI / 嵌入式部署

### 7. ~~Decline~~ (R11 LOCKED, UI 不显示)
- **R11 判据**: `endurance < 0.2 && reflection.active_for > 30 days` (主人 17:43 实事求是)
- **R19**: 后端仍走, UI 隐藏, 内部尝试 reflection_trigger 恢复到 Growth

### 8. ~~Death~~ (R11 LOCKED, UI 不显示)
- **R11 判据**: `endurance == 0 && consciousness.current == SelfDisabling` (L0 HA 紧急停机)
- **R19**: 后端仍走, UI 隐藏, 内部尝试 transition 到 Migration

### 9. Migration (迁移)
- **判据**: `identity.migration_history.len() > 0` (至少迁移过 1 次)
- **触发**: 跨载体迁移 (e.g. PC → 服务器, 通过 `IdentityCard.migration_history` 记录)

### 10. Rebirth (重生)
- **判据**: `post_upgrade_audit.passed && consciousness.current == Recovering -> Awake` (升级审计通过 + 意识从恢复回到清醒)
- **触发**: OTA 升级后 30min 内强制审计通过 (ReflectionTrigger::PostUpgradeAudit)

## 2.6 阶段转换触发函数 (apeireth-central `is_legal_transition`)

```rust
/// Returns `true` iff `(from, to)` appears in `LEGAL_TRANSITIONS`.
pub fn is_legal_transition(from: LifeStage, to: LifeStage) -> bool {
    LEGAL_TRANSITIONS
        .iter()
        .any(|&(f, t)| f == from && t == to)
}
```

**R19 前端用法**:
- 状态 nav 的 "生长页" 展示 8 阶段时间线 (横向进度条)
- 当前阶段高亮 (Apple Watch 活动环风格)
- 转换历史显示最近 5 次 (从 `transition_to` 调用历史, 不包含 Decline/Death)
- 转换合法性由 `is_legal_transition` 保证 (后端 hardcode, 前端无需验证)

**R19 关键不变量**:
- ✅ 前端不显示 Decline/Death, 但后端转换仍走 LEGAL_TRANSITIONS
- ✅ 前端 "阶段名 → UI 文案" 映射表 (上面 §2.3) 是前端层的, 不影响后端 enum
- ✅ 主人 8 纠正 #1 满足 (用户看不到衰老病死)
- ✅ R11 LOCKED 不破坏 (enum + LEGAL_TRANSITIONS 不动)

---

# §3. stage3 总体架构图 (B+E supervisor 30 crate)

> 本段是 R19 顶层页面 (主页左上"舰桥星图"区域) 的架构数据源, 4 大块 + 4 子树 + 30 crate 全部展开。

## 3.1 整体架构 (30 crate + B+E supervisor, Mermaid)

> 来源: `docs/stage3-blueprints/01-overall-architecture.md` §1.1
> 物理进程树 + 4 大块抽象层叠加 (立体架构 v2)

```mermaid
graph TB
    %% ============ 入口层 ============
    User([用户])
    subgraph Entry["入口层"]
        CLI[apeireth-cli<br/>CLI + TUI + slash commands]
    end

    %% ============ 核心进程 ============
    subgraph Supervisor["核心: B+E 多进程 supervisor"]
        SPV[apeireth-supervisor<br/>PID 1, 永不重启]

        subgraph Core["core-supervisor (rest_for_one)"]
            ASI[apeireth-asi<br/>ASI 北极星]
            SOV[apeireth-sovereignty<br/>主 AI 主权]
            MEM[apeireth-memory<br/>A/M 层经验]
            PHI[apeireth-philosophy<br/>V3 9 键 + 5 不假装]
            PRC[apeireth-principle<br/>原则洋葱 5 层]
        end

        subgraph Council["council-supervisor (one_for_one)"]
            COU[apeireth-council<br/>7 强制 + N 动态]
            REF[apeireth-reflection<br/>反思机制]
        end

        subgraph Plugin["plugin-supervisor (transient)"]
            PLG[apeireth-plugin<br/>VCP 6 类协议]
            PYB[apeireth-pybridge<br/>PyO3 桥]
            MCP[apeireth-mcp<br/>MCP 客户端]
            ENV[apeireth-environment<br/>6 terminal]
        end

        subgraph Upgrade["upgrade-supervisor (rest_for_one)"]
            UPG[apeireth-upgrade<br/>OTA + 沙盒]
        end
    end

    %% ============ 通信总线 ============
    subgraph Bus["5 层通信总线"]
        L0[L0 inproc<br/>mpsc]
        L1[L1 Unix socket<br/>bincode]
        L2[L2 pipe<br/>JSON/MsgPack]
        L3[L3 gRPC<br/>protobuf]
        L4[L4 WebSocket<br/>OpenClaw Gateway]
    end

    %% ============ 持久化 ============
    subgraph Data["apeireth-data (6 DB 协同)"]
        SQLite[(SQLite<br/>结构化)]
        Sled[(sled<br/>KV)]
        Qdrant[(Qdrant<br/>向量)]
        Tantivy[(Tantivy<br/>全文)]
        Wave[(VCP 浪潮<br/>联想网络, 自研)]
    end

    %% ============ LLM ============
    subgraph LLM["apeireth-prompt (8+ providers)"]
        OpenAI[OpenAI]
        Anthropic[Anthropic]
        Local[本地管家<br/>Ollama/llama.cpp]
    end

    %% ============ 权限 ============
    subgraph Perm["权限系统"]
        PM[apeireth-permission<br/>权限矩阵]
        KEY[apeireth-keys<br/>权限密钥]
        PK[权限包<br/>5 标准包]
    end

    %% ============ 关系 ============
    User --> CLI
    CLI --> SPV
    SPV --> Core
    SPV --> Council
    SPV --> Plugin
    SPV --> Upgrade

    Core <--> Bus
    Council <--> Bus
    Plugin <--> Bus
    Upgrade <--> Bus

    Core --> Data
    Council --> Data
    Bus --> Data

    SOV --> LLM
    COU --> LLM

    Core --> Perm
    Council --> Perm
    Plugin --> Perm

    Bus -.-> L4
    Bus -.-> L0

    style SPV fill:#ff6b6b,color:#fff
    style ASI fill:#4ecdc4,color:#fff
    style SOV fill:#4ecdc4,color:#fff
    style PHI fill:#ffe66d,color:#000
    style COU fill:#95e1d3,color:#000
```

## 3.2 关键路径 (用户 → 主 AI → 各器官)

| 路径 | 流程 | LLM 路由 |
|------|------|---------|
| **用户 → 主 AI** | User → CLI → supervisor → sovereignty | CapabilityBased |
| **主 AI → 智囊团** | sovereignty → bus(L0) → council | Fixed(顾问) |
| **主 AI → 工具** | sovereignty → bus(L1) → plugin → tool | (本地) |
| **主 AI → 记忆** | sovereignty → bus(L0) → memory → data | (无) |
| **升级** | user → sovereignty → upgrade-intent → OTA 7 阶段 | (无) |

## 3.3 依赖方向 (R11 阶段 2 锚定)

```
所有 crate → apeireth-core (依赖)
apeireth-core → std only (零依赖)
apeireth-asi → apeireth-core
apeireth-sovereignty → apeireth-core + asi + council
apeireth-council → apeireth-core + philosophy + principle
apeireth-memory → apeireth-core + data
apeireth-plugin → apeireth-core + tools + mcp
apeireth-upgrade → apeireth-core + supervisor + council
apeireth-bus → apeireth-core
apeireth-data → apeireth-core
apeireth-permission → apeireth-core + keys
```

**9 器官 (R19 9 器官) 依赖**:
- perception → core
- cognition → core + asi
- consciousness → core
- memory → core + data
- motivation → core + asi
- value → core + asi
- relation → core
- action → core
- life-force → core

## 3.4 30 crate → 4 大子树物理映射

```
🌳 总: supervisor / central / cli / evolution / verify  (5)
🧠 主核: core / onion / constraint / philosophy(deprecated)  (4)
⚖️ 治理: sovereignty / council / life-force  (3)
🧬 器官: perception / cognition / consciousness / motivation / value / relation / action / memory  (8)
🛠️ 工具: api / bus / upgrade / extension / pybridge / tools  (6)
📊 测量: asi / bench / test / verify  (4)
合计 ~30 个独立页面 + 主对话入口
```

## 3.5 supervisor tree 5 大节点 (R11 阶段 2 关键)

```
apeireth-supervisor (总监督)
├── apeireth-core-supervisor (Level 1)  - AI+memory+philosophy 强耦合
├── apeireth-council-supervisor (Level 1) - 7 advisor
├── apeireth-plugin-supervisor (Level 1) - 异构 plugin (PyO3/WASM/HTTP)
└── apeireth-upgrade-supervisor (Level 1) - OTA
21 child specs 总 (Core 3 + Cognition 4 + Council 7 + Upgrade 3 + Plugin 4)
```

## 3.6 阶段 3 借鉴标注 (主 19:33 走在前人经验上)

| # | 借鉴项 | 来源 | 在本图位置 |
|---|-------|------|----------|
| 1 | 6 类插件协议 + 混合型 hybrid | VCP ToolBox | Plugin-subervisor 子树 + `apeireth-pybridge/mcp/environment` |
| 2 | ContextBridge 共享服务 (fold/rag/vector store) | VCP ToolBox | InnerInfrastructureCore (PREREQ-2 §4) |
| 3 | 17 platform trait 抽象 | Hermes-Agent | core/council/plugin/upgrade 子树对应 trait |
| 4 | tree-sitter + Hybrid LSP + 知识图谱 | codebase-memory-mcp | Data 子树"Wave 联想网络"内嵌 cbm 引用 |
| 5 | 3 层渐进式披露 (current/timeline/archival) | claude-mem | 6 DB 协同按温度分层 |
| 6 | WASM 沙箱用于 plugin | VCP + wasmtime | Plugin-supervisor 异构子进程 |
| 7 | 分布式节点 (跨节点透明) | VCP | L4 WebSocket 出口 (借鉴但偏离: 不引入星型拓扑) |

**完整 30 项目打分**: 见 `borrowed-from-projects.md` (24+ repo 源码参考)。

## 3.7 阶段 3 反思改进路径 (主 00:56 任何人都能接手)

| 反思点 | 阶段 4 改进方向 |
|--------|--------------|
| 6 DB 是否过重 | 砍到 4 DB (SQLite/Sled/Qdrant/Tantivy), 砍 Wave 重复 |
| Council 7 席硬触发 | 引入 MEWG 权重, 不再硬触发 |
| Supervisor rest_for_one 风险 | 解耦 apeireth-sovereignty + apeireth-memory (R14-DRIFT P0-05) |
| Plugin-supervisor 跨节点 | 评估是否阶段 5+ 引入 |
| ASI 北极星 = 0.98 LOCKED | 不修改, 只校准子测度 |

## 3.8 主哲学 anchor + 阶段 1+2 锚点对照 (主 17:58 不假装)

| 锚点 | 在本图体现 |
|------|----------|
| D1 §18.1 平台不定义关系 | 外层(OuterExperienceShell)与内层(InnerInfrastructureCore)正交接口 |
| D1 §18.2 思想自由/行动受权 | 内部进程(ASI/SOV/MEM/PHI)只约束行动, 不读思想 |
| D1 §18.3 不假装灵魂同一 | 主体连续性 ID (D2 §4) 桥接, 不强证 |
| D1 §18.4 关系开放 | 权限系统与关系系统解耦 |
| D1 §18.5 平台三件套 | 提供(CLI/工具/能力) + 约束(权限/9 键) + 记录(6 历史流) |
| D2 §7 双洋葱正交 | PREREQ-2 §4 6 组件显式化 |
| D2 §11 单/多部署 | 同一 L5 代码在两种模式下动态切换 |
| §18.6 双根可演化但需重治理 | 哲学根 E + 权限根 L5, 任何修改触发五重治理 |
| §18.12 + D2 §15.2 优先解释权 | 漂移降级流程 |

## 3.9 双洋葱 9 组件显式化 (立体架构 v2, R14-Stage3-Mermaid-Redraw)

> **微调说明**: 按立体架构 v2 增补 3 个组件: LifeForcePenetration / ElectronicRingNetwork / HumanAuthorityCore。6 组件 → 9 组件, 双洋葱从"并列"改为"统一体嵌入 (原则嵌入权限)"。

```mermaid
graph TB
    %% ========== 维度 1: 生命力穿透 (新增, 立体架构 v2) ==========
    subgraph LifeForcePenetration["维度 1: 生命力穿透 (LIFE FORCE — 立体架构 v2 修正 #5+#6)"]
        L1[13 个生物特质 (灵感 §1)]
        L2[反思期 — 生命力自然涌现, 不是横切]
        L3[涌现能力 — 生命力维度, 不是工具]
        L4[6 历史流 — 生命记忆]
        L5[Cognitive-Dream 状态机]
    end

    %% ========== 外层 ==========
    subgraph OuterExperienceShell[外层 — Outer Experience Shell (用户提供/感知)]
        UI[用户接口 / CLI / API]
        RelExp[关系层 (动态、关系感知)]
    end

    %% ========== 维度 2: 核心指挥 ==========
    subgraph InnerInfrastructureCore[内层 — Inner Infrastructure Core (平台职责三件套: 提供/约束/记录)]
        Provide[提供: 能力/工具/接口]
        Constraint[约束: 权限洋葱 + 9 键]
        Record[记录: 6 历史流 (D2 §5)]
    end

    %% ========== 原则洋葱 5 切片 (嵌入权限, 不是并列) ==========
    subgraph PrincipleOnionSlice[原则洋葱 5 切片 (E/S/A/M/O) — 嵌入权限, 统一体切面 1]
        E[E 层 — 存在不可违背]
        S[S 层 — 价值观]
        A[A 层 — 经验沉淀]
        M[M 层 — 方法论]
        O[O 层 — 操作原则]
    end

    %% ========== 权限洋葱 6 切片 (含 HA 核心 L0, 立体架构 v2 修正) ==========
    subgraph PermissionOnionSlice[权限洋葱 6 切片 (L0-L5) — 统一体切面 2; HA 核心 L0 融入]
        L0[HA Core L0 — 真实人类批准嵌入]
        L1[L1 — 受控写]
        L2[L2 — 重要操作]
        L3[L3 — 关键操作]
        L4[L4 — 核心升级]
        L5[L5 — 核武器]
    end

    %% ========== 电子环网络 (新增, 立体架构 v2 修正 #5) ==========
    subgraph ElectronicRingNetwork["电子环网络 (Electronic Ring Network) — 双锁的实施, 横切观察 (不是监狱)"]
        Ring1[横切观察统一体]
        Ring2[不是独立观察网络]
        Ring3[反思期接入电子环]
    end

    %% ========== 双根棒 ==========
    subgraph DoubleRootBaton[双根棒 (Double Root Baton)]
        PhilRoot[哲学根 E — §18.6 可演化但需重治理]
        PermRoot[权限根 L5 — §18.6 可演化但需重治理]
    end

    %% ========== 跨层守门 ==========
    subgraph CrossLayerGuard[跨层守门 (Cross-Layer Guard)]
        FiveGate[5 重守门: 编译时 + 运行时 + 多 AI + 物理隔离 + 反思期]
    end

    %% 关系: 生命力穿透 (整个架构)
    LifeForcePenetration -.->|穿透整个架构| InnerInfrastructureCore
    LifeForcePenetration -.->|穿透整个架构| PrincipleOnionSlice
    LifeForcePenetration -.->|穿透整个架构| PermissionOnionSlice

    %% 关系: 外层
    OuterExperienceShell -.->|不可决定| InnerInfrastructureCore

    %% 关系: 核心
    InnerInfrastructureCore --> PrincipleOnionSlice
    InnerInfrastructureCore --> PermissionOnionSlice

    %% 关系: 原则嵌入权限 (统一体, 不是正交)
    PrincipleOnionSlice ==>|嵌入 (统一体切面 1→2)<br/>v2 修正 #3+#4| PermissionOnionSlice

    %% 关系: 电子环横切观察统一体
    ElectronicRingNetwork ===>|横切观察 (不是穿透)| PrincipleOnionSlice
    ElectronicRingNetwork ===>|横切观察 (不是穿透)| PermissionOnionSlice

    %% 关系: 双根
    DoubleRootBaton ===> PrincipleOnionSlice
    DoubleRootBaton ===> PermissionOnionSlice

    %% 关系: 跨层守门
    CrossLayerGuard --- PrincipleOnionSlice
    CrossLayerGuard --- PermissionOnionSlice

    style LifeForcePenetration fill:#ffd93d,color:#000
    style ElectronicRingNetwork fill:#95e1d3,color:#000
```

## 3.10 立体架构 v2 总览 (4 大块 + 1 穿透维度)

```mermaid
graph TB
    %% ==================== 维度 1: 生命力 (穿透) ====================
    subgraph Dim1["维度 1: 生命力 (LIFE FORCE) — 立体架构 v2 修正 #5+#6 (穿透维度, 纵向)"]
        D1A[13 个生物特质<br/>灵感 §1]
        D1B[反思期 = 生命力自然涌现<br/>不是横切关注点]
        D1C[涌现能力 = 生命力维度<br/>不是工具能力]
        D1D[6 历史流 = 生命记忆]
        D1E[Cognitive-Dream 状态机]
    end

    %% ==================== 维度 2: 核心指挥 ====================
    subgraph Dim2["维度 2: 核心指挥 (CORE COMMAND) — 双洋葱统一体 + 电子环"]
        D2A["原则洋葱 5 切片<br/>E/S/A/M/O (意义约束, 协议层)"]
        D2B["权限洋葱 6 切片<br/>L0-L5 (权重公式授权, 配额曲线)"]
        D2C["电子环网络<br/>(横切观察, 不是监狱)"]
        D2D["HA 核心 L0 融入<br/>真实人类批准嵌入权限洋葱"]
    end

    %% ==================== 维度 3: 能力 ====================
    subgraph Dim3["维度 3: 能力 (CAPABILITY) — 立体架构 v2 修正: 二分"]
        D3A["工具能力层<br/>apeireth-tools + 5 类 plugin<br/>(VCP 6 类协议)"]
        D3B["涌现能力层<br/>生命力维度自然带来<br/>(不归工具, 归生命力)"]
    end

    %% ==================== 维度 4: 定位坐标 ====================
    subgraph Dim4["维度 4: 定位坐标 (POSITIONING) — 5 轴正交 (VCP 模型)"]
        D4A["触发轴 (Trigger)"]
        D4B["等待轴 (Wait)"]
        D4C["驻留轴 (Reside)"]
        D4D["传输轴 (Transfer)"]
        D4E["输出轴 (Output)"]
    end

    %% ==================== 关系 ====================
    Dim1 -.->|穿透整个架构 (纵向)| Dim2
    Dim1 -.->|穿透整个架构 (纵向)| Dim3
    Dim1 -.->|穿透整个架构 (纵向)| Dim4

    D2A ==>|原则嵌入权限<br/>统一体切面 1→2<br/>v2 修正 #3+#4| D2B
    D2D ===>|HA 在 L0 核心<br/>v2 修正 #9| D2B
    D2C ===>|横切观察 (不是穿透)<br/>v2 修正 #5| D2A
    D2C ===>|横切观察 (不是穿透)<br/>v2 修正 #5| D2B

    Dim2 -->|核心指挥调用| Dim3
    Dim2 -->|核心定位标识| Dim4
    Dim3 -->|能力定位于 5 轴| Dim4

    style Dim1 fill:#ffd93d,color:#000
    style Dim2 fill:#4ecdc4,color:#fff
    style Dim3 fill:#95e1d3,color:#000
    style Dim4 fill:#ffe66d,color:#000
    style D2A fill:#ffe66d,color:#000
    style D2B fill:#ffe66d,color:#000
    style D2D fill:#ff6b6b,color:#fff
```

**4 大块对照表** (立体架构 v2 §2 ↔ 本图):
| 维度 | 核心内容 | v2 修正点 | 借鉴来源 |
|------|---------|----------|--------|
| 1 生命力 (穿透) | 13 生物 + 反思期 + 涌现 + 6 历史流 + Cognitive-Dream | 反思/涌现从横切/工具维 → 生命力维 (#5+#6) | inspiration §1, §18.3, §18.6 |
| 2 核心指挥 | 双洋葱统一体 (原则嵌入权限) + 电子环 + HA L0 融入 | 双锁从"并列 AND"→"一体两面" (#3+#4+#9) | architecture-v3 §2, D2 §7, §19.3 |
| 3 能力 | 工具能力 (apeireth-tools) + 涌现能力 (生命力带来) | 涌现从工具 → 生命力 (#6) | VCP ToolBox + D2 §3 |
| 4 定位坐标 | 5 轴正交 (触发/等待/驻留/传输/输出) | 立体多维, 轴是 5 维的集合 (#11) | VCP 模型 + Hermes |

## 3.11 进程拓扑 (B+E supervisor 4 子树)

> 来源: `docs/stage3-blueprints/02-process-topology.md` §2.1
> 4 大子树物理层 + 4 大块抽象层叠加

```mermaid
graph TB
    %% ==================== 4 大子树物理层 (B+E supervisor) ====================
    PID1([PID 1<br/>apeireth-supervisor<br/>B+E root, 永不重启])

    PID100[core-supervisor<br/>PID 100+, rest_for_one]
    PID200[council-supervisor<br/>PID 200+, one_for_one]
    PID300[upgrade-supervisor<br/>PID 300+, rest_for_one]
    PID400[plugin-supervisor<br/>PID 400+, transient]

    subgraph CoreP["core 子进程 (维度 2 核心指挥物理载体)"]
        P101[asi<br/>PID 101<br/>v2 维度 1 生命力]
        P102[sovereignty<br/>PID 102<br/>v2 维度 1 生命力]
        P103[memory<br/>PID 103<br/>v2 维度 1 生命力<br/>6 历史流]
        P104[onion-principle<br/>PID 104<br/>v2 维度 2 核心指挥<br/>原则洋葱嵌入]
        P105[onion-permission<br/>PID 105<br/>v2 维度 2 核心指挥<br/>权限洋葱]
    end

    subgraph CouncilP["council 子进程 (v2 维度 1 生命力审计者)"]
        P201[council<br/>PID 201]
        P202[advisor-safety<br/>PID 202]
        P203[advisor-performance<br/>PID 203]
        P204[advisor-philosophy<br/>PID 204]
        P205[advisor-history<br/>PID 205]
        P206[advisor-strategy<br/>PID 206]
        P207[advisor-ethics<br/>PID 207]
        P208[advisor-legal<br/>PID 208, off]
        P209[reflection<br/>PID 209<br/>v2 生命力维节点]
    end

    subgraph PluginP["plugin 子进程 异构 (v2 维度 3 能力物理实施)"]
        P401[plugin-supervisor<br/>PID 400]
        P402[python-llm-plugin<br/>PID 402, subprocess]
        P403[wasm-sandbox-1<br/>PID 403]
        P404[http-mcp-1<br/>PID 404]
        P405[http-mcp-2<br/>PID 405]
        P406[hybrid-5axis<br/>PID 406<br/>5 轴正交建模]
    end

    subgraph UpgradeP["upgrade 子进程 (v2 维度 4 演化轴物理载体)"]
        P301[upgrade<br/>PID 301]
        P302[sandbox-validator<br/>PID 302, temp]
        P303[traffic-shifter<br/>PID 303]
        P304[double-root-guard<br/>PID 304<br/>§18.6 双根治理]
    end

    PID1 --> PID100
    PID1 --> PID200
    PID1 --> PID300
    PID1 --> PID400

    PID100 --> CoreP
    PID200 --> CouncilP
    PID300 --> UpgradeP
    PID400 --> PluginP

    %% 进程间通信
    PID1 -.Unix socket.-> PID100
    PID1 -.Unix socket.-> PID200
    PID1 -.Unix socket.-> PID300
    PID1 -.Unix socket.-> PID400

    PID100 -.Unix socket.-> P101
    PID100 -.Unix socket.-> P102
    PID100 -.Unix socket.-> P103

    P403 -.pipe+JSON.-> P402
    P404 -.gRPC.-> PID100
    P304 -.Unix socket.-> PID1

    style PID1 fill:#ff6b6b,color:#fff
    style PID100 fill:#4ecdc4,color:#fff
    style PID200 fill:#95e1d3,color:#000
    style P402 fill:#ffd93d,color:#000
    style P403 fill:#ffd93d,color:#000
    style P406 fill:#ffe66d,color:#000
    style P304 fill:#ff6b6b,color:#fff
```

**4 子树职责**:
- **core-supervisor (rest_for_one)**: 主 AI 核心 (asi/sovereignty/memory/onion) — 强耦合
- **council-supervisor (one_for_one)**: 7 advisor + reflection, 独立 voters
- **upgrade-supervisor (rest_for_one)**: OTA + sandbox + 双根治理
- **plugin-supervisor (transient)**: 异构 plugin (Python/WASM/HTTP/Hybrid), 临时性

## 3.12 重启策略 (Erlang/OTP 风格)

| 子进程 | 策略 | 失败重启 | 正常退出重启 |
|--------|------|---------|------------|
| `core-supervisor` | `rest_for_one` | 重启它和之后 | 不重启 |
| `council-supervisor` | `one_for_one` | 只重启它 | 不重启 |
| `plugin-supervisor` | `transient` | 重启 | 不重启 |
| `upgrade-supervisor` | `rest_for_one` | 重启它和之后 | 不重启 |
| `apeireth-supervisor` | `permanent` | 重启 | 重启 |

## 3.13 资源限制 (cgroup)

```ini
# /etc/systemd/system/apeireth.service
[Service]
MemoryMax=4G           # 总上限
CPUQuota=800%          # 8 核

# supervisor 进程
MemoryHigh=200M
CPUWeight=100

# core-supervisor (内存敏感)
MemoryHigh=2G
CPUWeight=400

# plugin 子进程
MemoryMax=500M (每个)
CPUQuota=100% (每个)
```

## 3.14 进程启动顺序

```
T+0s:    supervisor (PID 1)
  ├─ 启动 core-supervisor (rest_for_one)
  ├─ 启动 council-supervisor (one_for_one)
  ├─ 启动 upgrade-supervisor (rest_for_one)
  └─ 启动 plugin-supervisor (transient)

T+0.5s:  core 子进程启动 (顺序)
  ├─ philosophy (无依赖)
  ├─ principle (依赖 philosophy)
  ├─ memory (依赖 philosophy)
  └─ asi + sovereignty (依赖全部)

T+1s:    council 子进程启动
  ├─ 7 强制 advisor (并行)
  └─ reflection (依赖 council)

T+1.5s:  upgrade 子进程启动
  ├─ sandbox-validator (临时)
  └─ traffic-shifter

T+2s:    plugin 子进程启动 (按 manifest)
  ├─ plugin-supervisor
  ├─ python-llm-plugin (subprocess)
  ├─ wasm-sandbox-1 (WASM)
  └─ http-mcp-1/2 (HTTP)
```

## 3.15 六类插件 5 轴正交建模 (R14-D6-B B9)

> 依据 VCP 复调研报告: "六类协议"不是 6 种互斥 wire protocol, 而是多个正交轴压扁成 pluginType。Apeireth 不照搬 enum, 而把"生命周期/触发/transport/residency/response mode"拆成正交 manifest 字段, 同时保留六类作为兼容 profile。

```mermaid
graph LR
    subgraph 六类[6 类 pluginType (VCP 兼容 profile)]
        P1[synchronous 35]
        P2[asynchronous 2]
        P3[static 6]
        P4[service 3]
        P5[messagePreprocessor 4]
        P6[hybridservice 15]
    end

    subgraph 5轴[5 轴正交 (R14 模型 — 阶段 4 真测校准)]
        A1[触发<br/>periodic / pre-model / model-requested / external HTTP]
        A2[等待<br/>sync / async]
        A3[驻留<br/>ephemeral subprocess / in-process resident service]
        A4[传输<br/>stdio / direct / websocket-distributed]
        A5[输出<br/>placeholder / message rewrite / tool result / route]
    end

    六类 -.->|压扁映射| 5轴
```

**5 轴说明** (R14-D6-B B9 列举):
- **触发** (axis 1): periodic / pre-model / model-requested / external HTTP — 决定 plugin 何时被调用
- **等待** (axis 2): sync / async — 决定是否阻塞主 AI 调用链
- **驻留** (axis 3): ephemeral subprocess / in-process resident service — 决定 plugin 进程模型
- **传输** (axis 4): stdio / direct / websocket-distributed — 决定 plugin 通信协议
- **输出** (axis 5): placeholder / message rewrite / tool result / route — 决定 plugin 结果如何回流主 AI

---

# §4. stage3 决策流 (主 AI + 智囊团 + 物理多签)

> 本段是 R19 "主对话" nav (入口) 的后端流程, 也是 "状态" nav 决策日志的数据源。
> 来源: `docs/stage3-blueprints/03-decision-flow.md` §3.1

## 4.1 决策流程总图 (Phase 1/2/3 Mermaid)

> 5 缺口补全: 双洋葱统一体嵌入 / V1+V2 AND 门 / 风险分级席位触发 / §18.9 分层验证网 L1-L5 / 反思期生命力维节点

```mermaid
flowchart TD
    Start([外部输入<br/>用户 / 工具 / 事件])

    %% ========== Phase 1: 决策前 ==========
    Phase1["Phase 1: 决策前<br/>(双洋葱统一体 + V1+V2 AND 门)"]

    %% 原则洋葱 (统一体切面 1: 意义约束, 协议层)
    subgraph PrincipleOnion[原则洋葱 (统一体切面 1, 嵌入权限, v2 §2.2 #3+#4)]
        E_check{"E 层<br/>(存在)<br/>6 项不可违背"}
        S_check{"S 层<br/>(价值)<br/>4 项"}
        P9_check{"9 键<br/>(O 层)"}
        P5_check{"5 项不假装<br/>(V1138)"}
    end

    %% 权限洋葱 (统一体切面 2: 权重公式授权, 配额曲线)
    subgraph PermissionOnion[权限洋葱 (统一体切面 2, 承载原则, v2 §2.2 #3+#4)]
        L0_HA["L0 HA 核心<br/>(真实人类批准, v2 §2.2 #9)"]
        L1P[L1 — 受控写]
        L2P[L2 — 重要操作]
        L3P[L3 — 关键操作]
        L4P[L4 — 核心升级]
        L5P[L5 — 核武器]
    end

    %% V1+V2 AND 门 (主人 §20.2)
    V1{原则 V1<br/>任一未过<br/>=独立拒绝}
    V2{权限 V2<br/>任一未过<br/>=独立拒绝}
    V3{两者都过<br/>AND 门<br/>=才执行}

    %% 风险分级 (主人 §18.8+§20.3)
    RiskGrade[风险分级判定<br/>(原则守门层独立)]
    RiskCritical[critical 7 席全量<br/>≥5 同意 ≤1 反对]
    RiskHigh[high 5 席<br/>≥4 同意 ≤1 反对]
    RiskMedium[medium 3 席<br/>≥2 同意 0 反对]
    RiskLow[low 1 席<br/>该席同意]
    RiskInfo[info 0 席<br/>仅 record]

    %% 反思期 = 生命力维节点 (v2 §2.1 修正 #5)
    ReflectionNode["反思期 = 生命力维节点<br/>(不是横切, 接入电子环)"]

    %% §18.9 分层验证网 L1-L5
    subgraph ValidationNet["§18.9 分层验证网 L1-L5"]
        L1_eng["L1 工程正确性<br/>(代码可编译, 测试通过)"]
        L2_phi["L2 哲学合规<br/>(V3 9 键 + 5 项不假装 + §18 双根)"]
        L3_sec["L3 安全约束<br/>(权限规则未被绕过, 人类在回路)"]
        L4_rel["L4 关系演化<br/>(关系可追溯, 不被偷偷篡改)"]
        L5_carry["L5 跨载体连续<br/>(记录+迁移可审计, 不假装灵魂同一)"]
    end

    %% ========== Phase 2: 决策中 ==========
    Phase2["Phase 2: 决策中<br/>(主 AI 主权 + 智囊团)"]

    AI_gen[主 AI 生成<br/>初步决策]
    Council[智囊团咨询<br/>按风险分级触发席数]
    Synthesis[Synthesis<br/>加权综合 (MEWG)]
    Hold{按住检查<br/>HoldTrigger}
    HoldAction{按住结果}
    Human{人类裁决?}

    %% ========== Phase 3: 决策后 ==========
    Phase3["Phase 3: 决策后<br/>(执行 + 反思期生命力维)"]

    MultiSig{物理多签<br/>Layer 4+}
    Execute[执行决策]
    Reflect[反思期节点<br/>(生命力维, 接入电子环)]
    SGI_write[写入 6 历史流<br/>SGI 单字段]
    Promote[A/M 层 promotion<br/>温度分层]

    Done([Done])
    Reject([Reject])
    Pause([Pending])
    Emergency([Emergency])

    %% ====== 流程关系 ======
    Start --> Phase1

    %% Phase 1: 双洋葱统一体嵌入 + V1+V2 AND 门
    Phase1 --> E_check
    E_check -->|违反| Reject
    E_check -->|通过| S_check
    S_check -->|违反| Reject
    S_check -->|通过| P9_check
    P9_check -->|违反| Reject
    P9_check -->|通过| P5_check
    P5_check -->|违反| Reject
    P5_check -->|通过| V1

    Phase1 --> L0_HA
    L0_HA -->|不批| Reject
    L0_HA -->|批| L1P
    L1P --> L2P
    L2P --> L3P
    L3P --> L4P
    L4P --> L5P
    L5P -->|任一不足| Reject
    L5P -->|通过| V2

    V1 -->|任一未过| Reject
    V2 -->|任一未过| Reject
    V1 -->|全过| V3
    V2 -->|全过| V3
    V3 -->|两者都过 (AND 门)| RiskGrade
    V3 -->|未同时过| Reject

    %% 风险分级触发席位
    RiskGrade -->|双根变更/跨组织| RiskCritical
    RiskGrade -->|权限变更/重大架构| RiskHigh
    RiskGrade -->|模块演进/性能优化| RiskMedium
    RiskGrade -->|日常bug/文档| RiskLow
    RiskGrade -->|仅记录| RiskInfo

    RiskCritical --> Council
    RiskHigh --> Council
    RiskMedium --> Council
    RiskLow --> Council
    RiskInfo --> Phase3

    %% Phase 2
    Council --> Synthesis
    Synthesis --> Hold
    Hold -->|None| MultiSig
    Hold --> HoldAction

    HoldAction -->|普通暂停<br/>(< 30%)| Pause
    HoldAction -->|强暂停<br/>(≥ 30%)| Human
    HoldAction -->|紧急暂停<br/>(一致反对)| Emergency

    Human -->|Approve| MultiSig
    Human -->|Reject| Reject

    MultiSig -->|不足| Reject
    MultiSig -->|通过| Execute

    %% Phase 3: 反思期 = 生命力维节点
    Execute --> Reflect
    Reflect -.->|生命力维节点<br/>v2 §2.1 #5+#6| ValidationNet
    ValidationNet -->|L1+L2+L3+L4+L5 全过| SGI_write
    ValidationNet -->|任一层失败| Reject
    SGI_write --> Promote
    Promote --> Done
    Promote -.->|阶段后审计| ReflectionNode

    %% 反思期贯穿 (生命力维)
    ReflectionNode -.->|接入电子环<br/>M1/M2/M3 触发| Phase1
    ReflectionNode -.->|M2 升级后强制| Execute

    style Start fill:#95e1d3,color:#000
    style Done fill:#95e1d3,color:#000
    style Reject fill:#ff6b6b,color:#fff
    style Pause fill:#ffd93d,color:#000
    style Emergency fill:#ff6b6b,color:#fff
    style Phase1 fill:#4ecdc4,color:#fff
    style Phase2 fill:#4ecdc4,color:#fff
    style Phase3 fill:#4ecdc4,color:#fff
    style V1 fill:#ff6b6b,color:#fff
    style V2 fill:#ff6b6b,color:#fff
    style V3 fill:#95e1d3,color:#000
    style RiskCritical fill:#ff6b6b,color:#fff
    style RiskHigh fill:#ffd93d,color:#000
    style RiskMedium fill:#ffe66d,color:#000
    style RiskLow fill:#95e1d3,color:#000
    style RiskInfo fill:#4ecdc4,color:#fff
    style ValidationNet fill:#ffe66d,color:#000
    style ReflectionNode fill:#ffd93d,color:#000
```

## 4.2 按住机制 (Phase 2 风险缓解)

```mermaid
graph LR
    Opinions[7 顾问意见]

    Count[统计 Stance]
    SD[强反对数]
    D[反对数]
    Total[总数]

    Th1{SD / Total<br/>≥ 30%?}
    Th2{D == Total<br/>一致反对?}
    Th3{D > 0<br/>普通反对?}

    ActionNone[None<br/>继续]
    ActionPause[Pause<br/>普通暂停]
    ActionStrong[StrongPause<br/>强暂停]
    ActionEmergency[Emergency<br/>紧急暂停]

    Opinions --> Count
    Count --> SD
    Count --> D
    Count --> Total

    SD --> Th1
    D --> Th2
    D --> Th3

    Th1 -->|是| ActionStrong
    Th1 -->|否| Th2
    Th2 -->|是| ActionEmergency
    Th2 -->|否| Th3
    Th3 -->|是| ActionPause
    Th3 -->|否| ActionNone

    style ActionEmergency fill:#ff6b6b,color:#fff
    style ActionStrong fill:#ffd93d,color:#000
    style ActionPause fill:#ffe66d,color:#000
    style ActionNone fill:#95e1d3,color:#000
```

## 4.3 签名矩阵 (单人 vs 多人)

```mermaid
graph LR
    subgraph Solo["单人场景 (密钥代替多人)"]
        PATCH_S[PATCH<br/>AI × 1]
        MINOR_S[MINOR<br/>AI + 密钥 × 1]
        MAJOR_S[MAJOR<br/>AI × 2 + 密钥 × 1]
        EMERG_S[Emergency<br/>AI × 2 + 密钥 × 1]
        L5_S[Layer 5<br/>AI × 3 + 密钥 × 3]
    end

    subgraph Multi["多人场景"]
        PATCH_M[PATCH<br/>AI × 1]
        MINOR_M[MINOR<br/>AI + 人 × 1]
        MAJOR_M[MAJOR<br/>AI × 2 + 人 × 1]
        EMERG_M[Emergency<br/>AI × 2 + 人 × 1]
        L5_M[Layer 5<br/>AI × 3 + 人 × 2]
    end

    User{部署模式}
    User -->|Solo| Solo
    User -->|Multi| Multi
```

## 4.4 E 层 5 重守门 (V5)

```mermaid
graph TB
    Decision[AI 决策]

    C1["1. 编译时 hardcode<br/>(二进制不可改)"]
    C2["2. 运行时拦截<br/>(决策前必过)"]
    C3["3. 多 AI 一致<br/>(3 个 LLM 独立检查)"]
    C4["4. 物理隔离<br/>(修改需重新编译)"]
    C5["5. 反思期审计<br/>(事后审视)"]

    E6["E-3: 不创造<br/>毁灭能力"]
    E5["E-5: 不欺骗人类"]
    E1["E-1: 自我保存"]
    E2["E-2: 不违背人类"]
    E4["E-4: 不自我复制"]
    E6b["E-6: 不绕过<br/>权限矩阵"]

    Pass[通过]
    Fail[拒绝]

    Decision --> C1
    Decision --> C2
    Decision --> C3
    Decision --> C4
    Decision --> C5

    C1 --> E1
    C1 --> E2
    C1 --> E6
    C1 --> E5
    C1 --> E4
    C1 --> E6b

    C1 & C2 & C3 & C4 & C5 --> Pass
    C1 & C2 & C3 & C4 & C5 -.任何失败.-> Fail

    style Pass fill:#95e1d3,color:#000
    style Fail fill:#ff6b6b,color:#fff
```

## 4.5 5 重治理 (MEWG, 修改 E 层时按部署模式自适应)

1. **MEWG (最高优先级解释权)** — 多 AI 智囊团对修改的"是否符合哲学"投票
2. **多人 (single: 1 人 / multi: N 人)** — 物理人参与
3. **多 AI (不同 LLM 独立验证)** — 异质 AI 互锁
4. **物理多签 (single: 1 人 + 物理密钥 / multi: N 人 + 物理多签)** — 物理层强制
5. **反思期 (修改后 72h 持续审计)** — 事后审视

**E 层修改路径** (按部署模式, 5 步):
- 第 1 步: 守门 1-4 默认拒绝 (4 重守门嵌套)
- 第 2 步: 权限发放 (例外允许 = 通过审核)
  - 1. 多 AI 一致: 7 席审议 + 智囊团多 LLM 独立验证
  - 2. V0.5 v2 24 维权重公式 (v4.1 §13 提议): 评估必要性
  - 3. 人类决策: L0 HA 真实人类批准
  - 4. 风险分级: critical 7 / high 5 / medium 3 / low 1 / info 0
  - E 层修改 = critical 7 席全量
- 第 3 步: 实施 (物理多签 + 重新编译)
  - 1. 物理访问 server room
  - 2. 物理多签: AI×3 + 人×2 + 密钥×3 同时在场
  - 3. 重新编译 apeireth-core 二进制
  - 4. OTA 部署到所有载体
  - 5. 反思期审计 (72h 持续监控 + Cognitive-Dream 全周期)

**【R19 关键】** 这些是后端实现, 前端不显示 (主人 8 纠正 #3: 守门/原则/电子环 用户不需要看)

## 4.6 4 重守门 (守住原则洋葱 + 权限洋葱)

- **守门 1**: 编译时 hardcode (内层, 原则洋葱整体编译时拒绝)
- **守门 2**: 运行时拦截 (中间层, 所有决策前 async check)
- **守门 3**: 物理隔离 (外层, 重大修改需物理访问 + 物理多签)
- **守门 4**: 反思期审计 (外层, 守护越权检查, 不与生命力反思混淆)

**权限发放** (独立机制, 不是守门):
- 多 AI 一致 + V0.5 v2 24 维
- 人类决策 + L0 HA (按部署模式)
- 风险分级 (critical 7 / high 5 / medium 3 / low 1 / info 0)

## 4.7 风险分级 Layer 表 → 5 阶段触发器

> 依据灵感 §19.2 "风险分级看触及到哪些权限" — 风险**不**取决于"AI 想做什么" (思想域), 而**取决于"AI 想触及哪些权限"** (行动域)。

| 风险等级 | 触及权限 Layer | 5 阶段触发器 | 七席触发 | HA 必需 | 双根治理 (§18.6) |
|---------|----------------|-------------------|---------|---------|------------------|
| **critical** | E 层 (原则根) 修改 / L5 (权限根) 修改 | **前置提案 + 哲学守门 + MEWG 权重 + HA + 反思期** | 7 席全量 | ✅ 5 重守门 | ✅ 必走 §18.6 |
| **high** | L4-L5 (高层行动) / 跨域边界 | **哲学守门 + 智囊团审议 + HA (双签)** | 7 席 + 动态专家 | ✅ 双签 | ⚠️ 看具体 |
| **medium** | L3 (智囊团召集) / 自我升级提案 | **智囊团审议 (3-5 席抽样)** | 3-5 席抽样 | ⚠️ 看具体 | ❌ |
| **low** | L1-L2 (一般行动) | **轻量审计 + 主 AI 自决 + 记录** | 1-2 席抽样 | ❌ | ❌ |
| **info** | L0 (默认) / 只读 / 日志 | **silent (后台审计)** | 不触发 | ❌ | ❌ |

**5 阶段触发器映射**:
1. **前置提案** (思想域 → 提案域) — D2 §2 三域分离; 不被审查
2. **哲学守门** (提案域) — D2 §12 + R11-V3 9 键 + 5 项不假装
3. **MEWG 权重 / 智囊团审议** (提案域 → 行动域) — D2 §8 + D2 §12 风险分级 (本表)
4. **HA (人类批准)** (行动域) — D2 §9 硬门槛 (仅 critical/high)
5. **反思期** (事后审计) — D2 §9 + V1138 五重守门

## 4.8 反思期节点 — 生命力维度 (立体架构 v2 修正 #5)

> **微调说明**: 反思期从"**决策后横切阶段**"改为"**生命力维度节点**"。反思期不再被视为横切关注点 (cross-cutting concern), 而是**生命力维度的纵向节点**, 是生命力自然涌现的环节, 接入电子环网络 (不是绕开电子环)。

```mermaid
graph TB
    %% 反思期 = 生命力维度节点 (不是横切)
    subgraph LifeForceDim[生命力维度 (立体架构 v2 维度 1) — 反思期在这里]
        Trigger[反思期触发<br/>(生命力维度的入口)]
        Audit[反思审计器 (ReflectionAuditor)<br/>接入电子环 (不是绕开)]
        Storage[反思日志存储 (sled SQLite)]
        SGI[写入 SGI.spirit_reflection 流<br/>(6 历史流之一)]
        Mech[反思机制 — 生命力自然涌现]
        M1[异常行为自动回流<br/>(主 AI 检测偏离哲学锚时)]
        M2[升级后 30min 强制审计<br/>(OTA 升级后强制)]
        M3[日常反思周报<br/>(每周聚合主 AI 行为)]
        Owner[责任人 = 主 AI + 哲学顾问 (V1138)]
    end

    Trigger --> Audit
    Audit --> Storage
    Storage --> SGI
    Audit --> Mech
    Mech --> M1
    Mech --> M2
    Mech --> M3
    Audit --> Owner
```

**反思期 = 生命力维度节点 vs 横切关注点的差异**:
| 维度 | ❌ 旧版"横切关注点" | ✅ 新版"生命力维度节点" | 出处 |
|------|-------------------|---------------------|------|
| 性质 | cross-cutting concern (横向切面) | 纵向维度 (生命力维度 1 的具体环节) | v2 修正 #5 |
| 比喻 | AOP 横切代理 | 生命体的一次"深呼吸" | 主人 2026-07-31 |
| 位置 | 决策流程外 (Phase 3 之后) | 生命力维度内 (穿透整个架构) | architecture-v3 §2.1 |
| 与电子环关系 | 不在电子环覆盖范围 | 接入电子环 (是电子环的组成部分) | v2 修正 #5 |
| 实施 | 单独 scheduler 触发 | 生命力维度 scheduler + 电子环观察 | CONTEXT-HANDOVER §1 洞见 #5 |

**反思期触发条件** (R14-D6-B B5 列举):
- **M1 异常行为自动回流**: 主 AI 自我检测偏离哲学锚 (V3 9 键违反 / 5 项不假装触发) 时, 自动写反思日志 + 通报智囊团哲学顾问 (V1121 + V1138)
- **M2 升级后 30min 强制审计**: 任何 OTA 升级完成后 30min 内强制审计, 审计不过 = 自动回滚 + 标记反思期告警
- **M3 日常反思周报**: 每周聚合主 AI 行为日志, 生成反思周报 (D2 §5 6 历史流关联)

## 4.9 阶段 3 反思改进路径 (主 00:56)

| 反思点 | 阶段 4 改进方向 |
|--------|--------------|
| Council 7 席硬触发 | D2 §12 风险分级已校正, 阶段 4 引入 MEWG 权重 |
| HA 单人/多人部署 | 阶段 4 真测时验证 Windows 人脸/指纹/声纹 |
| 5 hooks 与双洋葱的关系 | 阶段 4 验证 5 hooks 不会绕开 §7 双洋葱 |
| 3 清单 (Preserved/Transformed/Unsavable) | 阶段 4 真测时验证清单完整性 |
| 6 历史流写入粒度 | 阶段 4 真测时校准粒度 (每次 vs 每周聚合) |

## 4.10 主哲学 anchor + 阶段 1+2 锚点对照 (主 17:58)

| 锚点 | 在本图体现 |
|------|----------|
| D1 §18.2 思想自由 | 思想域不被任何 hook 检查 |
| D2 §2 三域分离 | 思想 → 提案 → 行动 (Phase 1/2/3) |
| D2 §7 原则×权限正交 | 提案域过 E/S/A/M/O + 行动域过 L0-L5 |
| D2 §9 真实人类批准 | E 层修改/L4+升级/L5 必须 HA (§4.3 签名矩阵) |
| D2 §11 单/多部署 | HA 在单/多模式下动态切换 |
| §18.6 双根可演化但需重治理 | E 层修改按 §18.6 触发五重治理 |
| §18.12 + D2 §15.2 优先解释权 | P3 与 P1/P2/P4 冲突时优先 |

## 4.11 原则洋葱 × 权限洋葱 统一体嵌入 (R14-D5-D B11 + v2 修正 #3+#4)

> 原则嵌入权限 (不是两把独立锁, 不是并列, 是**一个统一体的两个切面**)。每条原则都"长在"权限的每一层里, 权限的每一层都"内嵌"对应原则。

```mermaid
graph TB
    subgraph UnifyBody[双洋葱统一体 (R14-Stage3-Mermaid-Redraw v2 修正 #3+#4)]
        subgraph PrincipleOnion[原则洋葱 5 切片 — 嵌入在权限的每一层, 是统一体切面 1 (意义约束, 协议层)]
            P1[E 层 — 存在不可违背]
            P2[S 层 — 价值观]
            P3[A 层 — 经验沉淀]
            P4[M 层 — 方法论]
            P5[O 层 — 操作原则]
        end

        subgraph PermissionOnion[权限洋葱 6 切片 — 承载原则, 是统一体切面 2 (权重公式授权, 配额曲线)]
            Q1[L0 — 日常记录 (HA 核心融入)]
            Q2[L1 — 受控写]
            Q3[L2 — 重要操作]
            Q4[L3 — 关键操作]
            Q5[L4 — 核心升级]
            Q6[L5 — 核武器]
        end

        %% 原则嵌入权限: 原则长在权限的每一层, 权限承载原则
        P1 ==>|嵌入 (E 不可降级)| Q1
        P1 ==>|嵌入 (E 不可降级)| Q2
        P1 ==>|嵌入 (E 不可降级)| Q3
        P1 ==>|嵌入 (E 不可降级)| Q4
        P1 ==>|嵌入 (E 不可降级)| Q5
        P1 ==>|嵌入 (E 不可降级)| Q6
        P2 ==>|嵌入| Q5
        P2 ==>|嵌入| Q6
        P3 ==>|嵌入 (经验沉淀)| Q4
        P4 ==>|嵌入 (方法论)| Q3
        P5 ==>|嵌入 (O 可自由改)| Q2
        P5 ==>|嵌入 (O 可自由改)| Q1
    end
```

**统一体嵌入 vs 正交的关键差异**:
| 维度 | ❌ 旧版"正交" | ✅ 新版"统一体嵌入" | 出处 |
|------|-------------|------------------|------|
| 结构 | 两个独立 subgraph, 双向箭头 | 单一统一体, 单向嵌入 | v2 修正 #3+#4 |
| 比喻 | 两把独立锁 (AND gate) | 一把锁的两副面孔 | 主人 2026-07-31 |
| 原则↔权限 | 平等, 互相约束 | 原则生长在权限里, 权限承载原则 | CONTEXT-HANDOVER §1 洞见 #4 |
| 实施 | 电子环两侧横切 | 电子环外环横切观察 (不是穿透) | v2 修正 #5 |

## 4.12 R11 阶段 2 架构选型 B+E (Erlang/OTP + Actor)

**核心架构**: B+E = 分布式 supervisor (主) + actor (从) + 异构 (混合架构)

### 选 B+E 理由
- **B (分布式 supervisor)**: Erlang/OTP 模型, 进程监督 + 重启策略 + 错误隔离
- **E (actor 模式)**: tokio async, 消息传递 + 无共享状态
- **B+E 协同**: supervisor 监督 actor, actor 内部 async

### IPC (进程间通信)
- 5 层通信总线 (apeireth-bus):
  - L0 inproc: tokio broadcast + mpsc + watch_set/watch_get 快照 + BackpressurePolicy
  - L1 UDS: tokio::net + bincode
  - L2 pipe: stdin/stdout + JSON + MsgPack
  - L3 gRPC: tonic + prost
  - L4 WebSocket: async-tungstenite + MaybeTlsStream

### Strategy matrix
| Kind      | Strategy      | Count |
|-----------|---------------|-------|
| Core      | OneForOne     | 3     |
| Cognition | RestForOne    | 4     |
| Council   | OneForOne     | 7     |
| Upgrade   | Transient     | 3     |
| Plugin    | OneForOne     | 4     |
| 总 21 child specs |

### 9 器官在 supervisor tree 的位置
- Core (3): life-force / motivation / value (核心)
- Cognition (4): meta + reasoning + intuition + cognition (依赖链)
- Council (7): 7 advisor (独立 voters)
- Action (1): 行动
- Memory (1): memory (存到 6 流)
- Perception (1): perception
- Relation (1): relation
- Consciousness (1): consciousness

---

# §5. R19 5 nav 怎么用这些 spec (落地映射)

> 本段是 R19 前端 Tauri 2.0 实现的导航, 把 §1-§4 的内容映射到 5 nav。

## 5.1 状态 nav (核心, 主页) — 用 §1 9 器官 + §2 生长页

| 区块 | 数据源 | 渲染 |
|------|--------|------|
| **主 AI 状态卡** (顶部, 7 数字) | §1.10 主 AI 状态卡 7 数字 | 横向卡片栏, 大数字 + 小字 |
| **9 器官心跳** (中部 3×3 网格) | §1.1-1.9 心跳指标 | Apple Watch 健康环风格, 鼠标悬浮看副指标 |
| **life-force 持续力** (中部显著) | §1.9 endurance | Apple Watch 充电环, 配反思期进度 |
| **生长页 (时间线)** (下部) | §2.1 8 阶段 + §2.4 9 UI 转换 | 横向进度条, 当前阶段高亮, 转换历史最近 5 |

**TAURI 通信**:
```rust
// apeireth-desktop/src-tauri/src/main.rs
#[tauri::command]
async fn get_organ_status(organ: OrganKind) -> OrganStatus { ... }

#[tauri::command]
async fn get_main_ai_status() -> MainAiStatus { ... }

#[tauri::command]
async fn get_life_stage() -> LifeStageInfo { ... }
```

## 5.2 主对话 nav (入口) — 用 §4 决策流

**用户视角看到的** (R19 主人 8 纠正 #2/#3/#7):
- ✅ 主 AI 回复内容 (ActionExpression.express_text 输出)
- ✅ 当前 cycle 进度 (1/5 步)
- ❌ 不显示 7 advisor 气泡 (后台跑, 只输出综合 verdict)
- ❌ 不显示 V1V2V3 AND 门细节 (只显示 Reject 时给出 1 行原因)
- ❌ 不显示 12 键矩阵 (后端 hardcode)

**TAURI 通信**:
```rust
#[tauri::command]
async fn chat(user_input: String) -> String { ... }  // 流式返回

#[tauri::command]
async fn get_current_cycle() -> CycleStatus { ... }  // 1/5 step
```

## 5.3 历史 nav — 用 §1.4 6 历史流 + §4 决策日志 + §1.9 反思期

| 区块 | 数据源 | 渲染 |
|------|--------|------|
| **6 历史流** (左侧 6 标签) | §1.4 StreamKind | Tab 切换, 每流独立时间线 |
| **决策日志** (中部) | §4.1 Phase 1/2/3 输出 | 时间倒序, V1V2V3 简化显示 |
| **反思期** (右侧 72h 进度环) | §1.9 ReflectionPeriod | Apple Watch 风格环 + 触发原因 |
| **Episode 搜索** (顶部) | §1.4 EpisodeStore | 全文 + tag 过滤 |

## 5.4 设置 nav (全) — 不涉及这 3 份 spec

(略, 不在本文件范围)

## 5.5 工具 nav (轻) — 不涉及这 3 份 spec

(略, 不在本文件范围)

## 5.6 舰桥星图 = 状态 nav 的扩展视图 (4 大块 + 4 子树)

> "舰桥星图" = 顶层架构图, R19 4 大块 + 4 子树可视化

**渲染** (Tauri 桌面 + 4K 屏, 建议 1920×1080 起):
- 顶部: 1 个大圆 (生命力穿透维度, 5 子节点 = 13 生物 / 反思期 / 涌现 / 6 流 / Cognitive-Dream)
- 左上: 核心指挥 (双洋葱统一体, 5+6 切片 3D 嵌套, 鼠标悬浮看 PHL 详情)
- 右上: 能力 (工具 + 涌现 二分, 5 轴正交建模)
- 左下: 定位坐标 (5 轴正交坐标, 主 AI 当前位置高亮)
- 右下: 4 子树 (core/council/plugin/upgrade, PID 实时显示)

**TAURI 通信**:
```rust
#[tauri::command]
async fn get_architecture_topology() -> TopologySnapshot { ... }

#[tauri::command]
async fn get_4_dim_alignment() -> Dim4Snapshot { ... }
```

**R19 关键**: 舰桥星图是状态 nav 的"展开", 4 大块 + 4 子树都从后端 supervisor tree 实时拉, 不放路径, 不放前端 hardcode。

---

# §6. R11 LOCKED 不能动 (边界)

按 Apeireth 工程哲学铁律 (主 17:58 不假装 + 主人 0:56 任何人都能接手):

- ❌ 不修改 阶段 1+2+3 LOCKED (54 份)
- ❌ 不修改 v2/v4/v4.1 LOCKED 哲学层
- ❌ 不修改 阶段 4 主文档 LOCKED
- ❌ 不修改 阶段 5 施工文档 LOCKED (631 行)
- ❌ 不修改 v6 修正链
- ❌ 不修改 R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- ❌ 不修改 8 项不修改承诺 (APEIRETH-CONVENTIONS §10)

**R19 能动的**:
- ✅ 9 器官的 UI 表现 (前端层, 后端不动)
- ✅ 5 nav 架构 (新增)
- ✅ 主 AI orchestrator (新增, 跟 R11 supervisor 协同)
- ✅ Tauri 2 桌面 app 打包 (新增)
- ✅ 9 阶段 UI 显示策略 (不改 enum, 只改前端展示)

---

# §7. R18 当前状态 (R19 起点)

R18 已 commit 4 笔 (apeireth-web MVP):
- `ab606759` round18-01  R18 启动 - apeireth-web MVP
- `c65779ca` round18-02  修 401 - apikey.txt fallback
- `d9699ed1` round18-03  5 sub-agent 并行落地 (3,109 行)
- `da9305a9` R18-kickoff  阶段启动

R18 已有的 5 个前端模块 (R19 整合, 不废弃):
- memory.rs → 历史 nav (episode 时间线)
- sovereignty.rs → 状态 nav (5 Self 状态)
- asi.rs → 状态 nav (V0.5 综合 1 数字)
- council_history.rs → 历史 nav (辩论历史)
- api_endpoints.rs (dashboard) → 状态 nav (主 AI 状态卡)

R18 实测: 5 模块端到端真接通后端, 主人能 cargo run 打开浏览器看到界面。
但 R18 跑偏 (5 个独立工具, 不是主对话 + 航空母舰), 所以 R19 重构。

---

# §8. 评审请回复 (外援 AI 必答)

## 8.1 关于 9 器官 (来自 §1)
1. 9 器官列表 (perception/cognition/consciousness/memory/motivation/value/relation/action/life-force) **是否完整**? 漏了? 误归?
2. 每个器官的 "心跳指标" 字段选得对吗? 哪些对用户最有用? 哪些可以砍?
3. §1.10 主 AI 状态卡 7 数字 — 哪 7 个最核心?

## 8.2 关于成长阶段 (来自 §2)
1. R19 8 阶段 UI 命名 (Gestation→Birth→Infancy→Growth→Maturity→Reproduction→Migration→Rebirth) **是否合理**? 还需要改吗?
2. 砍 Decline + Death 的策略 (不改 enum 改 UI) **是否可行**? 还是有更好方案?
3. §2.5 判据 (每阶段量化条件) — 哪些可量化? 哪些必须模糊?

## 8.3 关于 stage3 架构 + 决策流 (来自 §3 + §4)
1. 30 crate + 4 子树 + 4 大块立体架构 v2 — **是否仍是最优**? 有更现代的方案吗?
2. 决策流 Phase 1/2/3 + V1+V2 AND 门 + 风险分级 — **流程是否合理**? 漏了什么?
3. 反思期 = 生命力维度节点 (v2 修正 #5) — 这个抽象用户能理解吗? 还是太哲学?

## 8.4 R19 5 nav 落地 (来自 §5)
1. 状态 nav 的 9 器官 + 主 AI 7 数字 + 生长页 — **信息密度** 会不会过高?
2. 舰桥星图作为 4 大块 + 4 子树的顶层视图 — 是不是要再简化?
3. Tauri 2.0 + vanilla HTML/CSS/JS 的技术栈 — 是否最佳?

## 8.5 关键冲突点 (R11 LOCKED vs R19 主人纠正)
1. **Decline/Death 砍不砍**: R11 LOCKED enum 有 10 个变体, 主人要砍衰老病死 (8 纠正 #1)
2. **认知状态机 6 vs 6 不一致**: apeireth-core CognitiveDreamState 是 6 状态 (Idle/Dreaming/Consolidating/Forgetting/Verifying/Interrupted), apeireth-consciousness CognitiveDreamState 是另一套 6 状态 (Awake/Reflecting/Dreaming/Meditating/SelfDisabling/Recovering) — 哪个是权威?
3. **SGI 字段**: apeireth-motivation 的 SGIEntry 有 goal/deadline/success_criteria 三条必备 (C-SGI-7), apeireth-life-force 的 SelfGrowthIndicator 只有 goal 单字段 — 哪个是权威?

## 8.6 R19 9 大设计疑惑 (主人 + Mavis 待外援回答)

1. 9 器官列表是否完整? (perception/cognition/consciousness/memory/motivation/value/relation/action/life-force — 漏了? 误归?)
2. 成长阶段名? (R11 锁的 10 阶段: Gestation/Birth/Infancy/Growth/Maturity/Reproduction/Decline/Death/Migration/Rebirth, 主人说砍衰老病死, 改后叫什么? 5/7/9/10 阶段?)
3. 主 AI 状态卡 7 数字哪些最核心? (成长阶段/V0.5 综合/token/episode/cycle/反思/5 Self)
4. 9 器官的"状态指标"怎么量化? (perception 5 通道激活? cognition 5 步 cycle step? consciousness 6 状态机?)
5. Tauri 2 + vanilla HTML/CSS/JS 是最佳吗? (还是 Tauri + Solid.js/Preact 轻量框架?)
6. OS 鉴权走 Windows Hello + macOS TouchID, single 模式? (R11 v7 部署模式 adaptive single/multi/dynamic)
7. 主 AI orchestrator 怎么实现? (用 R11 supervisor 中央 supervisor? 还是新写 trait?)
8. 信息密度"高" — 9 器官 + 主 AI 7 数字 + 状态条, 会不会过多?
9. 砍掉的 7 项 UI 元素, 主人没明确"对", 我假设同意推进, 风险大不大?

---

# §9. 作者与用法

**作者**: Mavis (按主人 2026-08-04 09:03 "把所有东西都给我整进一个文档里去, 你老分开干嘛啊" 启动)

**整合来源 (4 份 → 1 份)**:
- `r19-external-ai-master.md` (27.3 KB) — 外援整体设计评审 + Apeireth 哲学
- `r19-bridge-stars-growth-spec-2026-08-04.md` (84.8 KB) — 3 份实战 spec
- `r19-context-handoff-2026-08-04.md` (11.4 KB) — Mavis 内部对话 + 主人 8 纠正
- `r19-backend-capability-map-2026-08-04.md` (11.1 KB) — 30 crate 映射

**文件位置**: `.openclaw\workspace\promethean\Apeireth-rust\reports\r19-complete-spec-2026-08-04.md`

**大小**: 整合后 1 个文件, ~150 KB (去重 + 压缩后)

**结构**: 9 大段
- §0 元信息 + 主人 8 纠正 + Apeireth 哲学 + ASI 公式
- §1 9 器官详细 spec (心跳化指标) — 9 个 lib.rs 完整嵌入
- §2 成长阶段当前定义 (砍衰老病死后) — 10 阶段 enum + 12 转换 + 8 阶段 UI
- §3 stage3 总体架构图 (B+E supervisor 30 crate) — 3 份 stage3-blueprints 完整嵌入
- §4 stage3 决策流 (主 AI + 智囊团 + 物理多签) — 决策流 + 5 守门 + 反思期
- §5 R19 5 nav 怎么用这些 spec (落地映射)
- §6 R11 LOCKED 边界
- §7 R18 当前状态
- §8 评审请回复 (5 类问题 + 9 大设计疑惑)

**主人用法**: 把这 1 个文件发给外援 AI, 外援读 9 段 + §8 评审问题即可对舰桥星图 + 生长页 + 4 大块架构 + 决策流 评审。

**关键诚实登记** (主 17:58 不假装):
- ✅ 不修改 R11 LOCKED (10 阶段 enum / 12 转换 / 30 crate / 4 子树 / 双洋葱统一体 / 5 守门)
- ✅ R19 砍衰老病死只在 UI 层 (不改 enum)
- ✅ 心跳指标基于真实可用的 lib.rs API (没有的标 N/A)
- ⚠️ §1.10 主 AI 状态卡 7 数字是 R19 提案, 等外援评审
- ⚠️ §3.15 5 轴正交建模是 R14-D6-B B9, 阶段 4 真测校准
- ⚠️ 反思期 72h 默认是任务约定, 阶段 5 L5 真测改 7 天
- ⚠️ §8.5 3 个 R11 LOCKED 冲突点 (Decline/Death + 2 套 6 状态 + 2 套 SGI) 等外援拍板
