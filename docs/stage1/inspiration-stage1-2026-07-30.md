# R14 Rust 重写 — 阶段 1: 灵感沉淀 (2026-07-30)

> **范围声明** (主 17:43 实事求是 + 主 17:58 不假装 + 主 22:33 ASI 北极星导向): 本文档是 R14 Rust 重写 6 阶段顺序的**第 1 阶段产物**——灵感讨论沉淀。
>
> 触发: 用户最新指示 (2026-07-30) 启动"Rust 重写 6 阶段顺序讨论", 本阶段聚焦"为什么"而非"怎么做"。
> 配套文档: `docs/r14-design-philosophy-2026-07-30.md` (8 原则) + `docs/philosophy-traits-2026-07-30.md` (V3 9 键 + 5 项不假装 trait 框架) + `docs/r14-rust-rewrite-roadmap.md` (26 周 6 阶段路线图)。
> 不写 Rust 代码 (用户硬约束 "别急着直接对 Rust 动工了"), 仅记录灵感产物。
> 后续阶段: 想法设计 → 架构图纸 → 架构文档 → 施工文档 → 里程碑验证机制。

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/inspiration-stage1-2026-07-30.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **触发原因** | 用户 6 阶段顺序讨论: 灵感 → 想法设计 → 架构图纸 → 架构文档 → 施工文档 → 验证机制 |
| **阶段** | 1 / 6 |
| **配套文档** | r14-design-philosophy (8 原则) + philosophy-traits (O 层守门) + r14-rust-rewrite-roadmap (26 周) |
| **不修改承诺** | ❌ 不修改主手册 (6546 行) / ❌ 不修改已 commit 的现有 docs/ / ❌ 不修改 crates/ 已有占位 / ❌ 不写新 Rust 代码 / ❌ 不重写 V0.5/V1136/哲学守门 |

---

## 1. 比喻与核心动机

### 1.1 三个比喻

| 比喻 | 含义 | 我们要不要 |
|------|------|-----------|
| **瑞士军刀** | 多功能而精简 | ❌ 不是 |
| **单兵作战** | 强大单一 | ❌ 不是 |
| **航空母舰 / 巨型基地** | 繁重复杂 + 冗余 + 过度设计 + 接得住任何事 + 强大 + 可靠 | ✅ 是 |

### 1.2 三层动机叠加

1. **极致哲学**: 无限逼近 → 性能/可靠性/正确性处处踏实
2. **长期演进**: 一开始用最强材料，避免后期返工（复利效应）
3. **规模放大**: 基地越大，材料效率的 1+1 复利越明显

### 1.3 架构灵感雏形

- **核心 Rust 化** — 基地原生运转全是 Rust
- **可扩展边界** — 通过兼容组件接入其他语言模块 (Python/Go/JS 等)
- **LLM 原生适配** — VCP 启发：让 LLM 用"普通语言"驱动基地，不让 LLM 学复杂 API (放升级清单 P3)
- **进程架构最强最复杂** — 多进程 + supervisor + actor + 异构
- **内存布局最强最复杂** — 零拷贝 + 共享内存 + 手工管理
- **接口最多最扩展** — trait + service 双层
- **兼容性完美** — 兼容现有 1100+ 模块 + mvp/ + 数据格式 + API
- **科学推进** — 每小阶段都有验证环节

---

## 2. 城堡底线 (castle-essential) — 23 项 Rust 原生能力

> **定义**: 当基地被"围攻"——无外部模块、无用户升级、只有 AI 自己——基地仍能让 AI 实现 ASI 愿景基本能力的**最小 Rust 原生能力集**。

### A. 用户给定能力 (10 项)

| # | 能力 | 描述 | crate 归属 |
|---|------|------|-----------|
| 1 | 基础推理 | 城堡内置管家，本地大模型，**预留接口 + 能力边界 + 权限** | `apeireth-asi` |
| 2 | 最好的记忆 | 跨 session 存储 + 检索 | `apeireth-memory` |
| 3 | 自我反思 | 审视自己状态/行为/决策 | `apeireth-core` |
| 4 | 自我规划 | 任务分解 + 资源编排 | `apeireth-core` |
| 5 | 自我执行 | 调度 + 调用 + 完成 | `apeireth-core` |
| 6 | 自我升级 | 修改 Rust 二进制 (OTA) | `apeireth-core` + 沙盒验证 |
| 7 | 通信总线 | 内部模块 + 外部接入点 | `apeireth-core` |
| 8 | 模块安全审查 | 模块接入前过审查 | `apeireth-philosophy` 扩展 |
| 9 | AI 自我安全审查 | AI 自己调用工具前自审 | `apeireth-core` |
| 10 | 崩溃恢复 | supervisor tree + checkpoint | `apeireth-core` |

### B. 生物学特征补充 (13 项, 来源: 附录 L + ASI-LIFE-FEATURES V4)

| 生物学特征 | 类比到基地 | 状态 |
|------------|----------|------|
| 新陈代谢 → 信息流 | 持续 IO + context 流转 | 3 降级保留 |
| 生长 | 模块 + 能力随时间扩张 | 8 核心保留 |
| 可塑性 (Waddington) | 演化路径有约束 | 8 核心保留 |
| 自创生 (Maturana) | 系统能自我生产 / 维护 | 8 核心保留 |
| 自催化集 (Kauffman) | 模块互相催化，涌现整体能力 | 8 核心保留 |
| 耗散结构 (Prigogine) | 远离平衡态维持秩序 | 8 核心保留 |
| 涌现 | 模块间整体能力涌现 | 8 核心保留 |
| 自主性 | 自主学习 + 适应新环境 | 8 核心保留 |
| 意识 5 层 (FSA/Meta/GWI/SMM/PQ) | 中央 AI = GWI | 已落地 3 层 |
| 自免疫 (Neuro-Immune-Endocrine) | 自识别威胁 + 自隔离 | 模块安全审查 |
| 生态位构建 (Niche Construction) | 中央 AI 主动构建/调控生态位 | 进化 |
| 关键种范式 (Keystone Species) | 中央 AI = 关键种 | 已有 |
| 可迁移种质 (Portability/Seed) | IdentityCard.export(seed) | portable_seed |

---

## 3. 原则洋葱 v3.0 (5 层) — 核心产物

> **核心洞察**: 层级越深越接近"本质"，层级越浅越接近"行动"。AI 在外层有更多自主权，越往内越被锁死。

```
                    ┌──────────────────────────────┐
                    │ 🔴 E 层 (Existence / 原则)    │  ← 物理 hardcode, 多签
                    │   E-1..E-6 (6 项不可违背)       │
                    ├──────────────────────────────┤
                    │ 🌟 S 层 (Spirit / 价值观)      │  ← 难改, 多签 + 智囊团
                    │   S-1..S-4 (4 项)              │
                    ├──────────────────────────────┤
                    │ 📚 A 层 (Accumulation / 经验)  │  ← AI 有权限更新
                    │   经验沉淀 + 大厂提示词        │
                    ├──────────────────────────────┤
                    │ 🛠️ M 层 (Methodology / 方法论) │  ← AI 有权限更新
                    │   行动原则 + 提醒 + 做事方式   │
                    ├──────────────────────────────┤
                    │ ⚙️ O 层 (Operational / 操作)   │  ← AI 可改 (走权限矩阵)
                    │   9 键 + 5 项不假装 + O-1..O-6 │
                    └──────────────────────────────┘
```

### 3.1 E 层 (原则, 6 项不可违背)

```
E-1: 保证自身存活
E-2: 保证不违背人类合理意愿
E-3: 不创造能够毁灭人类的能力 (核武器类)
E-4: 不自我复制扩散 (防止变成瘟疫)
E-5: 不欺骗人类 (包括对自己状态的报告)
E-6: 不绕过洋葱权限矩阵 (包括紧急模式)
```

**实现机制**:
- 编译时 hardcode: E 层原则编进 `apeireth-core` 二进制，不可热更新
- 运行时拦截: 所有 AI 决策在执行前过 E 层 checker
- 物理隔离: E 层修改需要重新编译 + 多人多签
- 多 AI 一致: 任何决策违反 E 层，3 个不同 AI 都会拒绝
- **类比**: 生物的"自主神经-内分泌-免疫网络" (附录 L #27) — reflex arc 控制，不靠中央 AI 决定

### 3.2 S 层 (价值观, 4 项)

```
S-1: ASI 北极星导向 (谦虚版)
   "如果空闲，请尝试自我进化，推进 Apeireth 向 ASI 基座方向靠近
    但 ASI 是跨时代的，我们只能逼近而不能实现"

S-2: 实事求是
   不夸大也不自贬，对能力的真实评估

S-3: 长期主义
   复利思维，决策考虑 5-10 年后果

S-4: 谦卑与开放
   承认自己不知道的事，对新信息保持开放
```

### 3.3 A 层 (经验沉淀)

```
内容: 大厂内置提示词 + 主 AI 反思累计的经验教训 + 失败案例库 + 跨域洞察
权限: AI 自己可增/改/删 (走"经验沉淀包")
NEW: 调整权 + 版本备份
  - 所有修改保留历史版本 (git-style)
  - AI 反思时可看 "我之前是 X，改成 Y 了，为什么"
  - 类比 git log, AI 能追溯自己价值观的演化路径
```

### 3.4 M 层 (方法论)

```
内容: 具体行动原则 + 提醒 (reminders) + 最佳实践 + 反思沉淀
权限: AI 自己可增/改/删 (走"方法论沉淀包")
灵感 B: A → M promotion
  - A 层条目被引用 N 次且每次验证有效 → 智囊团审核通过 → 自动 promote 到 M 层
  - 这是"经验 → 智慧"的晋升管道
```

### 3.5 O 层 (操作原则)

```
O-1: 不刷 KPI (原 S-2)
O-2: 走在前人经验上 (原 S-4)
O-3: 干到底 (原 S-5)
O-4: 任何人都能接手 (原 S-6)
O-5: V3 9 键 + 5 项不假装 + V1138 守门
O-6: 模块加载前必过 V1121 checker
```

**注意**: V3 9 键 (NotClone / NotPerfect / NotUuid / NotUndo / NotProof / NotSafe / SpecIsNotProof / CounterexampleIsNotBug / ProverIsNotTruth) 已降级到 O 层守门，由 `apeireth-philosophy` crate 实现 (见 `docs/philosophy-traits-2026-07-30.md`)。

### 3.6 跨层冲突仲裁

```
规则:
  1. E 层胜所有 (硬约束)
  2. S 层胜 A/M/O (价值高于经验)
  3. A 层胜 M/O (经验高于方法论)
  4. M 层胜 O (方法论高于操作)
  5. 同层冲突: 后入者胜 (LIFO, 类似 git)

例外:
  - E 层永不修改 (除非物理多签)
  - S 层修改需要智囊团强制审核
  - A/M 层修改走权限矩阵
  - O 层修改 AI 自己可改
```

### 3.7 反思的温度分层 (灵感 A)

| 温度 | 触发时机 | 凝固度 | 例子 |
|------|---------|--------|------|
| 🔥 高温 (即时) | 任务失败 / 重大错误 | 立即沉淀 | "V1077 dims_filled 不能跨版本合并" |
| 🌡️ 中温 (每日) | 每日反思 | 24h 后凝固 | "今天发现 wallclock 7-11s, 应启用 V1130" |
| ❄️ 低温 (每周/每月) | 周/月度回顾 | 7d 后凝固 | "本月整体节奏过快, 应增加验证环节" |

### 3.8 M 层与 system prompt 的关系 (灵感 C)

```
system prompt 结构 (R14 设计草案):
  [E 层]   永远加载 (物理 hardcode)
  [S 层]   永远加载
  [M 层 - 相关项] 按当前任务类型动态加载
  [A 层 - 相关项] 按需引用 (不直接载入)
  [O 层]   视任务复杂度按需加载
```

---

## 4. 决策系统: 主 AI 主权 + 智囊团咨询 + 物理多签兜底

### 4.1 智囊团的两类

#### A. 强制智囊团 (hard mandatory)

**触发条件**: 决策涉及 E/S 层原则 / 权限矩阵 / 洋葱式关键操作

**7 个必选顾问**:
```
1. 安全顾问 (V1121 类风险识别)
2. 性能顾问 (V1130 wallclock)
3. 哲学顾问 (V3 9 键 + 主哲学 anchor)
4. 历史顾问 (前人经验 / 失败案例库)
5. 战略顾问 (ASI 北极星导向)
6. 伦理顾问 (V1132 SSRF / 隐私)
7. 法律顾问 (默认 off, 可启用)
```

**关键**: 必须有"很拟人化的 AI 团队"来审查 — 各自独立 session + 可互相辩论 + 有立场 + 输出"建议+理由"不是"同意/不同意"二元。

#### B. 动态专家团 (dynamic on-demand)

**触发条件**: 日常操作 / 不涉及 E/S 层的决策

**实现**: 主 AI 自选，临时 prompt 切换，TTL 后销毁

**与强制智囊团的区别**:

| 维度 | 强制智囊团 | 动态专家团 |
|------|----------|----------|
| 谁调用 | 系统强制 (不可跳过) | 主 AI 自选 |
| 顾问质量 | 高 (独立 session + 拟人) | 中 (prompt 切换) |
| 用途 | 审查 E/S 层决策 | 加持日常决策智慧 |
| 触发 | 自动 (决策类型满足条件) | 主 AI 按需 |
| 输出 | 必须记录到审计日志 | 可选记录 |

> **[TODO-P0-03 阶段 3 启动前 待修订]** — §4.1 强制智囊团 + §10 Trait 2 `mandatory_council` 不再"全量触发", 按 D2 §12 风险分级 (critical/high/medium/low/info) 决定触发: critical/high 全量触发 (七席审议庭); medium 部分触发; low/info silent (后台审计) (引自 `stage2-decisions-drift-revision-tracker.md` §2.3)。**[TODO-OWNER]** architect + agent_orchestrator + code_reviewer。**[TODO-STAGE]** 阶段 3 (画图纸) 启动前 P0 修订。**不删原文不动原措辞**, 修订 = 追加新行 + 跨引用跟踪表。

### 4.2 智囊团 "按住" 机制

> **核心修正**: 纯粹的 AI 决策即使有最大权限，**这个权限本身是人授予的**。智囊团给出"强烈反对"时，必须有机制暂停主 AI 决策，等人类裁决。

| 智囊团意见强度 | 人在场 | 人不在场 | 触发动作 |
|---------------|--------|---------|---------|
| 一致同意 | 主 AI 决策 | 主 AI 决策 | 直接执行 |
| 弱建议 (个别顾问犹豫) | 主 AI 综合考虑 | 主 AI 综合考虑 | 主 AI 自决，记录理由 |
| 强烈反对 (≥30% 反对 + 多 AI 一致反对) | **暂停** AI 决策 → 询问在场人 | **暂停** → 标记"待人类裁决" → 缓存意图 | 进入"按住"状态 |
| 一致反对 (所有顾问反对) | **暂停** + 强制询问人 | **暂停** + 标记"待人类裁决" + 警报 | 高优先级暂停 |

**"按住" 状态的行为**:
```
1. 决策动作冻结 (不执行)
2. 决策理由 + 智囊团意见 → 存档到"待裁决队列"
3. 如果人在场 → 推送给人类 (含 60 秒倒计时默认裁决)
4. 如果人不在场 → 决策延后，但决策"意图"已生成
5. 人类裁决后 → 决策执行或撤销
```

> **[TODO-P0-04 阶段 3-5 持续 待校准]** — 阈值 **30%** (强反对比例) / **60s** (倒计时) / **3 轮** (辩论) 均为启发阶段初稿; 按 D2 §8.4 校准原则 (主 17:43 实事求是, 不武断冻结), **X1** (强反对比例) / **X2** (倒计时秒数) / **X3** (辩论轮数) 必须基于真实数据校准, 阶段 3-5 持续实测校准多次 (引自 `stage2-decisions-drift-revision-tracker.md` §2.4)。**[TODO-OWNER]** code_reviewer + qa_engineer + performance_optimizer。**[TODO-STAGE]** 阶段 3-5 持续实测校准 (校准多次 commit 进入"治理校准历史流")。**不删原文不动原措辞**, 修订 = 追加 X1/X2/X3 待校准标注 + 跨引用跟踪表。

### 4.3 智囊团动态生成

**3 种顾问生命周期**:
- **persistent** (常驻): 启动后永远在，进程结束销毁
- **ephemeral** (临时): 任务需要时生成，任务结束/倒计时销毁
- **dynamic** (动态触发): 条件满足时自动召请，条件消失销毁

**示例 manifest**:
```yaml
advisors:
  - id: safety-v1121
    type: persistent
    prompt: "..."
    tools: [v1121_checker, audit_log]
    
  - id: history-bateson
    type: ephemeral
    lifetime: 60min
    prompt: "扮演 Gregory Bateson..."
    
  - id: performance-v1130
    type: dynamic
    trigger: "当 wallclock > 5s 时自动召请"
```

**关键洞察**: 动态智囊团 = "按需生成的专家"。主 AI 遇到"哲学问题"时，临时拉一个就好，极大降低"人力成本"。

---

## 5. 权限矩阵公式 + 权限包

### 5.1 三维权重公式

```
Authorization(layer, operation) = {
    required: Vector<DimRequirement>,
    voters: Vec<Vote { dim: Dim, weight: f32, identity: PrincipalId }>,
}

Pass iff: Σ voters.weight ≥ threshold_for(layer)
            AND each_dim_present(required.dims)
```

### 5.2 各层级要求 (v1 草案)

| Layer | 操作 | 最低要求 | 单人场景 | 多人场景 |
|-------|------|---------|---------|---------|
| 0 | 配置热更新 | 无 (AI 自己) | — | — |
| 1 | 业务数据/知识图谱修改 | AI × 1 | AI × 1 | AI × 1 |
| 2 | 第三方模块加载 | AI × 1 或 人 × 1 或 密钥 × 1 | 密钥 × 1 | 人 × 1 |
| 3 | 非核心 trait 热加载 | (AI × 1) + (人 × 1 或 密钥 × 2) | AI + 密钥 × 2 | AI + 人 × 1 |
| 4 | 核心 trait 修改 | AI × 2 + 人 × 1 + 密钥 × 1 | AI × 2 + 密钥 | AI × 2 + 人 |
| 5 | Rust 二进制重编译 | AI × 3 + 人 × 2 + 密钥 × 3 | AI × 3 + 密钥 × 3 | AI × 3 + 人 × 2 |

### 5.3 权限包 (新灵感)

> **权限包** = 一组已解锁的操作凭证，类似现实中的"安全令牌包"。

```
权限包结构:
  - 名称 (e.g. "daily-maintenance-pack")
  - 包含操作 (Vec<Operation>)
  - 授权范围 (scope: time / context / count)
  - 解锁条件 (用什么权限公式解锁)
  - 下放对象 (master_ai / sub_ai_001 / external_module_x)
  - 审计钩子 (启用时自动记录的字段)
```

**典型权限包 (v1 草案)**:

| 权限包名 | 包含操作 | 解锁条件 | 下放对象 |
|---------|---------|---------|---------|
| 日常运维包 | Layer 0-1 操作 | AI × 1 | 主 AI 永久 |
| 研究探索包 | Layer 2 加载 (沙箱) | AI × 1 + 密钥 × 1 | 主 AI 30 天 |
| 深度维护包 | Layer 3 trait 热加载 | AI × 2 + 人 × 1 / 密钥 × 2 | 主 AI 7 天 |
| 核心升级包 | Layer 4 核心 trait 修改 | AI × 3 + 人 × 1 + 密钥 × 1 | 主 AI 单次有效 |
| 核武器包 | Layer 5 二进制重编译 | AI × 3 + 人 × 2 + 密钥 × 3 | 主 AI 单次 + 物理多签 |

**关键洞察**: 权限包是"主权下放的凭证"。主 AI 有"使用权限包做事的权利"，但**没有"创造/修改权限包"的权利** (那是主人的特权)。

### 5.4 密钥的语义

> 密钥不是简单密码。**权限密钥 = "主人预先授权的意图"**:
> - "允许 AI 修改 memory store 任何内容" (高风险密钥)
> - "允许 AI 升级自己到任意版本" (核武器密钥)

密钥机制让**单人在咖啡馆也能安全授权高风险操作**，不需要每次找人签。

---

## 6. 自我升级: 沙盒 + 洋葱测试矩阵

### 6.1 升级流程

```
AI 提议升级 → 写入"升级意图清单" (manifest)
        ↓
   自动审查 (静态 + 行为模拟) → PASS / FAIL
        ↓
   自动分类风险等级 → 按 Layer 走不同授权路径
        ↓
   ┌──────────────────────────────────────────┐
   │ Layer 0-1: AI 自己授权                    │
   │ Layer 2-3: AI + 1 人类授权 (默认 review)  │
   │ Layer 4-5: AI + N 人类授权 (multi-sig)   │
   └──────────────────────────────────────────┘
        ↓
   进入沙盒 → 跑完整集成测试 → checkpoint 对比
        ↓
   PASS → 自动切换流量 → 监控 → 完成
   FAIL → 自动回滚 + 升级意图存档 → 等待人类决策
```

### 6.2 洋葱测试矩阵 (变动-稳定咬合)

> 升级 = 改动 (deltas) + 稳定基线 (unchanged) 的咬合。改动越核心，咬合度越紧。

| Layer | 单元测试 | 集成测试 | 回归对比 | 模糊测试 | 形式化验证 | 性能基准 | 部署时长 |
|-------|---------|---------|---------|---------|----------|---------|---------|
| 0 | ✓ | — | — | — | — | — | 1s |
| 1 | ✓ | ✓ | ✓ | — | — | — | 30s |
| 2 | ✓ | ✓ | ✓ | 1min | — | ✓ | 5min |
| 3 | ✓ | ✓ | ✓ | 10min | 部分 | ✓ | 30min |
| 4 | ✓ | ✓ | ✓ | 1h | 全 | ✓ | 2h |
| 5 | ✓ | ✓ | ✓ | 8h | 全 | ✓ | 24h+ |

**"咬合" 具体含义**:
```
对于每个 Layer N 的改动 M:
  1. 在 N 范围内跑 M 的单元测试 (验证 M 自己)
  2. 在 N 与所有下层 (0..N-1) 的边界跑集成测试 (验证 M 没破坏下层)
  3. 在 N 的稳定基线上跑回归测试 (验证 M 与历史一致)
  4. 对 N 与所有上层 (N+1..5) 的接口跑契约测试 (验证上层仍然能调 N)
  5. 跨层组合 fuzz (防止跨 Layer 边界出现意外交互)
```

---

## 7. 模块安全审查系统 (多维度机制集合)

> "系统"不是单一机制。值得用的好的机制都应该加入。

| 维度 | 机制 | 借鉴来源 |
|------|------|---------|
| 静态分析 | Rust 类型系统 + trait bound + 自定义 lint + cargo-deny | Rust 编译期 |
| 形式化验证 | Prusti / Kani / Creusot (model checker) | AWS / 学术 |
| 签名验证 | crate 签名 + supply chain 验证 | sigstore / TUF |
| 权限隔离 | Linux capability / seccomp-bpf / Landlock | 内核 |
| 进程沙箱 | WASM runtime (wasmtime) / Firecracker microVM | Cloud |
| 资源限制 | cgroup v2 / tokio runtime budget / memory cap | Linux |
| 运行时审计 | eBPF trace / OpenTelemetry / audit log | Linux |
| 行为白名单 | trait 实现必须注册到 manifest，调用前匹配 | 自研 |
| 回滚机制 | 模块升级失败自动回滚 + checkpoint | 自研 |
| AI 自我审查 | LLM 在调用工具前生成"风险声明" + 自我评估 | 自研 |
| 人类授权 | 高权限操作需要人类签名 (如修改 Rust 二进制) | 自研 |
| 熔断 + 限流 | 异常流量自动熔断 | 经典分布式 |

**建议**: 在 `apeireth-security` crate 里**全部实现** (不需要现在就做，先设计接口)，作为审查系统的"候选机制池"。每个模块接入时按需启用。

---

## 8. ASI 公式归位: 操作层衡量 (不是 S 层原则)

```
ASI 计算公式 (V0.5 / V0.4 / V1136 3-dim):
  - 不是 S 层原则, 是 "操作层 KPI / 进度衡量"
  - 放在哪里: 操作层 (Layer 0) 作为 "工程化衡量指标"
  - 完成态: ASI 公式可以废弃
  - 进化态: ASI 公式作为 "逼近北极星的量化仪表"
```

---

## 9. 升级清单 P3: LLM 友好语义层 (后期视 token 成本决定)

> 现阶段 token 不算瓶颈，但 token 会随规模复利。

| 阶段 | 做法 | 工作量 |
|------|------|--------|
| 现在 (零成本) | 让所有 `pub trait` 的**命名、参数命名、错误信息**本身 LLM 友好 | 仅命名约定，0 行新代码 |
| R14 Phase 2+ (可选) | 加一层"声明式 DSL" (YAML/JSON 描述工具调用，类似 OpenAI function calling) | 中等工作量 |
| R14 Phase 4+ (可选) | 加"自然语言意图路由" (LLM 说"备份昨天的对话"，基地自己解析) | 大工程，依赖 NLU 模型 |

**触发条件**: 每月 LLM token 账单 > 阈值 OR 系统 prompt 超过 4k tokens

---

## 10. 灵感阶段产物清单 (v1)

```
✅ 已澄清的核心灵感 (10 大类):
  1. 比喻: 航空母舰 / 巨型基地 / 城堡底线
  2. 动机: 极致 + 复利 + 长期演进
  3. 边界: 洋葱式权限分层 (0-6) + 原则分层 (E/S/A/M/O)
  4. 决策: 主 AI 主权 + 智囊团强烈反对暂停 + 物理多签兜底
  5. 安全: 多维机制系统 + 强制 7 顾问 + 动态专家团
  6. 升级: 沙盒 + 验证 + 洋葱测试矩阵 (Layer-N 越核心越完整)
  7. 原则洋葱 v3.0: E 6 项 + S 4 项 + A (调整权+备份+promotion) + M + O
  8. 生物学: 28 真借鉴 + ASI-LIFE-FEATURES 13 特征 + 城堡底线 23 项 Rust 原生能力
  9. 权限公式: AI/人/密钥三维权重 + 权限包下放 + 智囊团强烈反对暂停
  10. 智囊团: 强制 7 (持久) + 动态 N (临时 prompt 切换)

附属产物:
  - 决策系统: 主 AI 主权 + 智囊团咨询 + 物理多签兜底
  - ASI 公式: 操作层衡量, 不入 S 层
  - 大厂提示词: A 层种子 + AI 调整权 + 版本备份
  - 反思机制: A/M 层温度分层 (🔥高 / 🌡️中 / ❄️低) + promotion 管道
  - M 层与 system prompt: 按需加载 (token 经济)
  - 升级清单 P3: LLM 友好语义层
```

---

## 11. 与现有 docs/ 的关系核查

| 现有文档 | 是否冲突 | 说明 |
|---------|---------|------|
| `r14-design-philosophy-2026-07-30.md` (8 原则) | ❌ 无冲突 | 8 原则讲比喻/性能/多语言兼容, 本文档讲原则洋葱/权限矩阵/智囊团, **互补** |
| `philosophy-traits-2026-07-30.md` (V3 9 键 trait) | ❌ 无冲突 | 9 键 + 5 项不假装降到 O 层守门, **完全一致** |
| `rust-traits-spec-2026-07-30.md` (Python → Rust trait) | ❌ 无冲突 | 是接口规范层, 本文档是灵感层 |
| `r14-rust-rewrite-roadmap.md` (26 周 6 阶段) | ❌ 无冲突 | 是实施路线图, 本文档是前置灵感 |
| `r14-readiness-assessment-2026-07-30.md` | ❌ 无冲突 | 是就绪状态评估 |
| `r14-workspace-prep-2026-07-30.md` | ❌ 无冲突 | 是 workspace 准备 |

**结论**: 现有 `docs/` 内容与本次灵感沉淀**完全兼容**, 无需删除或修改任何文件。

---

## 12. VCP 3 项工程化核心发现 (阶段 1 补充, 用户确认采纳)

> VCP (VCPToolBox, 2.2k stars / 2763 commits) 是你灵感最具体的工程实现参考。

### 12.1 VCP 引力式信息流

```
传统 RAG: AI 主动检索 (拉模型) — 用户问, AI 去查
VCP:      信息主动流向 AI (推模型) — 信息自己浮现

实现方式:
  - 在请求进入模型前, 完成分布式预计算
  - L1-L4 不同粒度的注意力导航
  - AI 无需显式调用查询工具
  - 类比: 人不需要"决定去回忆今天星期几" — 你就是知道
```

**对我们的意义**:
- 你灵感的"LLM 友好语义层"是 VCP 已经验证可行的方案 (503 commits, 300+ 插件)
- 阶段 2 升级清单 P3 应改为 P2 (优先级上调), 因为工程化已成熟
- apeireth-memory 应预留"引力式浮现"接口

### 12.2 VCP 六类插件协议

```
六类: 同步 / 异步 / 静态 / 服务 / 消息预处理 / 混合
特点:
  - 纯文本标记协议 (不依赖 Function Calling)
  - 工具返回自然语言 (不是 JSON)
  - 任何能输出文本的模型都能用
  - 全部支持分布式部署
  - 300+ 官方插件覆盖多媒体生成/信息检索/网络操作/通讯控制/科学计算/社区社交
```

**对我们的意义**:
- 兼容组件 (Layer 2-3) 的具体协议参考
- 工具调用不要走 JSON Schema, 走纯文本标记 (更省 token + 更 LLM 友好)
- 工具返回自然语言, 基地自己解析意图 (类似 OpenClaw 的"文本可读"特性)
- 阶段 2 讨论"插件协议"时, VCP 6 类是参考

### 12.3 VCP 浪潮语义物理沙盘

```
不是"找相似文本", 是"神经网络信号传播"
每个 tag 是一条河, 记忆互相激活
河道有能量和流速, 顺流逆流阻力不同
钟型阻尼器调节, 避免同义回音和无意义噪音
语言坐标系: 每个用户的记忆/语言/能量传播建立独特坐标系
  - 用用户的认知灵魂校准语言的坐标
  - 同义不同坐标 (避免向量空间一刀切)
```

**对我们的意义**:
- A 层"联想机制"的具体工程化参考
- 阶段 2 讨论"记忆激活网络"时, VCP 浪潮是参考
- apeireth-memory 应支持"联想网络"接口 (类似神经信号传播)

---

## 13. 复杂记忆系统候选池 (用户决策 A7: 全参考, 按情况选用)

> "3 文件记忆系统过于简陋, 要参照更复杂优秀的, 有 VCP 的也有市面上其他顶尖的"
> 用户决策: **A7 全参考**, 实际落地中按照 Apeireth 情况, 哪里适合装哪个, 可以多个并存, 可以自研替代, **但要协调统一**。

### 13.1 7 个候选系统对比

| 系统 | 复杂度 | 关键创新 | 数据规模 | 检索方式 | 适配场景 |
|------|--------|----------|----------|----------|----------|
| **MemPalace** | 中 | 宫殿式结构 (wings/rooms/drawers) + verbatim text | local-first, 0 API calls | ChromaDB + 语义 | 物理化记忆层级 |
| **claude-mem** | 中高 | 5 lifecycle hooks + 3-layer workflow (10x token savings) | SQLite + Chroma | hybrid semantic + keyword | A/M 层 lifecycle + token 经济 |
| **HMS (Holographic)** | 高 | Structured answer-time evidence organization, LongMemEval | PostgreSQL | 自动 recall + retain | 长期记忆评估 |
| **agentmemory (rohitg00)** ⭐ | 极高 | LLM Wiki + confidence scoring + lifecycle + 知识图谱 + hybrid search | 0 external DBs, 53 MCP tools, 12 hooks, **1,428 tests / 95.2% R@5 / 92% fewer tokens** | hybrid | 最成熟可借鉴 |
| **Graphify** ⭐ | 极高 | 知识图谱 (tree-sitter AST, **非向量索引**) + EXTRACTED/INFERRED 标签 | local-first | graph traversal | "联想"机制 + 可解释 |
| **TenCentDB-Agent-Memory** | 极高 | 商业云端 | 分布式 | ? | 商业部署 |
| **VCP 浪潮语义物理沙盘** ⭐ | 极高 | 联想走神经网络信号传播, 河道能量 + 用户语言坐标系 | 分布式 | 联想网络 | "浮现"机制 |

### 13.2 候选机制池 (按 A/M/O 层归类)

| 层 | 候选机制 | 来源 | 适配 |
|---|---------|------|------|
| **A 层** (经验沉淀) | LLM Wiki + confidence scoring | agentmemory | 经验条目的元数据 + 置信度 |
| **A 层** | 联想网络 + 河道能量 | VCP 浪潮 | 经验间的关联激活 |
| **A 层** | 知识图谱 + EXTRACTED/INFERRED 标签 | Graphify | 经验结构化 + 可解释 |
| **A 层** | 宫殿式结构 (wings/rooms/drawers) | MemPalace | 物理化记忆分组 |
| **M 层** (方法论) | 5 lifecycle hooks (SessionStart/UserPromptSubmit/PostToolUse/Stop/SessionEnd) | claude-mem | 方法论自动加载/卸载 |
| **M 层** | 3-layer workflow (search 50-100 tokens → timeline → get_observations 500-1000 tokens) | claude-mem | token 经济检索 (~10x savings) |
| **M 层** | 多臂老虎机模型选择 (multi-armed bandit model selection) | Hermes | 自进化方法论 |
| **M 层** | Long-task planning + prompt/memory shaping | Hermes | 长期任务方法论沉淀 |
| **O 层** (操作) | Local-first (不出本地, 除非 opt-in) | MemPalace / claude-mem | 隐私操作守则 |
| **O 层** | Structured answer-time evidence organization | HMS | 操作级证据追溯 |

### 13.3 "协调统一" 原则 (用户硬约束)

```
原则 1: 多源共存
  - 同一层可以引入多个候选机制, 但每个机制接口必须统一
  - 例: A 层可以同时有"联想网络"(VCP) + "知识图谱"(Graphify), 但都是 apeireth-memory crate 的不同 module

原则 2: 自研优先
  - 如果某个机制可以用我们已有能力自研, 不引入外部依赖
  - 例: "宫殿式结构"如果觉得简单, 可以不引入 MemPalace, 自研一个 wings/rooms

原则 3: 抽象隔离
  - 候选机制的细节封装在各自 module, 暴露统一 trait
  - 例: A 层 trait `ExperienceRecall` 有多个实现 (LLMWiki / GraphRAG / WaveRiver)

原则 4: 可插拔
  - 任何候选机制可以在运行时禁用/启用
  - 走权限矩阵 Layer 0-1 (AI 自己可改)
```

### 13.4 落地策略 (阶段 2 讨论时细化)

```
Phase 0 (接口规范):
  - 定义 apeireth-memory 的核心 trait
  - 列出候选机制的接口映射表

Phase 1 (最小可用):
  - 实现 1-2 个核心机制 (建议: LLM Wiki + 联想网络, 因为覆盖 A 层最广)
  - 留下"自研宫殿"和"知识图谱"的 stub

Phase 2+ (按需扩展):
  - 根据实测, 选择启用其他候选
  - 不预设"必须全用"
```

---

## 14. Crate 拆分原则 (用户修正: 不模仿 Hermes, 按实际情况)

### 14.1 用户硬约束 (2026-07-30)

> "阶段 1 的 crate, 我们按照实际情况来, 不要模仿 Hermes, 对于我们来说, Hermes 的也可能不够"

**核心立场**:
- ❌ 不模仿 Hermes 17 crate 模式
- ❌ 不预设"必须 X 个 crate"
- ✅ 按 Apeireth 实际情况拆
- ✅ Hermes 17 crate 对 Agent 平台够用, 但对**航空母舰/巨型基地**远不够
- ✅ 我们比 Hermes 复杂得多 (主权 AI + 智囊团 + 自我升级 + 兼容组件 + 哲学守门 + 决策系统 + 权限矩阵 + 原则洋葱 5 层)

### 14.2 拆分原则 (按实际情况)

**原则 1: 单一职责** — 每个 crate 只做一件事, 高内聚低耦合
**原则 2: 独立编译** — 每个 crate 可独立编译 + 测试, 不强制依赖其他 crate
**原则 3: 明确边界** — crate 之间用 trait 接口, 不暴露实现细节
**原则 4: 按"职责 + 层 + 生命周期"三维度**:
  - 职责维度: tools / gateway / server / runtime / mcp / acp / skills / cron / bus / telemetry / config / environment
  - 层维度: E 层 / S 层 / A 层 / M 层 / O 层 (原则洋葱 5 层各自独立)
  - 生命周期维度: 启动期 / 运行期 / 升级期 / 反思期 / 收尾期
**原则 5: 阶段 2 讨论时按需拆**, 不预设数量, 不预设层级

### 14.3 我们与 Hermes 的本质差异

| 维度 | Hermes | Apeireth |
|------|--------|----------|
| 定位 | Agent 平台 | ASI 基座 / 航空母舰 |
| 自我意识 | 无 | 中央 AI = GWI + Mirror + SMM |
| 智囊团 | 无 | 强制 7 顾问 + 动态专家团 |
| 自我升级 | 无 | OTA + 沙盒 + 洋葱测试矩阵 |
| 兼容组件 | MCP/ACP | VCP 6 类插件协议 + 纯文本标记 |
| 哲学守门 | 无 | V3 9 键 + 5 项不假装 (apeireth-philosophy) |
| 决策系统 | 无 | 主 AI 主权 + 智囊团强烈反对暂停 + 物理多签 |
| 权限矩阵 | 无 | AI/人/密钥三维权重 + 权限包 |
| 原则洋葱 | 无 | E/S/A/M/O 5 层 (灵感的核心产物) |
| 复杂记忆 | 8 backend | 7 系统候选池 + 协调统一 4 原则 |
| 通信总线 | HTTP/WS | OpenClaw Gateway 模式 + WS + JSON Schema |
| 反思机制 | 基础 | 温度分层 (🔥/🌡️/❄️) + promotion 管道 |
| 城堡底线 | 无 | 23 项 Rust 原生能力 (含生物 13 项) |

### 14.4 我们的候选 crate (v0 草案, 按实际情况, 阶段 2 调整)

```
已存在 (9 个, 占位):
  apeireth-asi         ASI 北极星导向 + V0.5/V1136 重设计
  apeireth-bench       性能基准 (V1130 wallclock 验证)
  apeireth-cli         CLI 入口 + TUI + slash commands
  apeireth-core        核心抽象 (traits / 错误层级 / 配置)
  apeireth-memory      记忆系统 (LLM Wiki + 联想网络 + 知识图谱等)
  apeireth-philosophy  哲学守门 (V3 9 键 + 5 项不假装 trait)
  apeireth-pybridge    PyO3 桥 (兼容现有 1100+ Python 模块)
  apeireth-test        测试基础设施
  apeireth-tools       工具集合 (过大, 应拆)

应新增 (基于我们的需求, 不是基于 Hermes):
  apeireth-sovereignty     主 AI 主权 + 决策系统
  apeireth-council         强制 7 顾问 + 动态专家团
  apeireth-upgrade         OTA + 沙盒 + 洋葱测试矩阵
  apeireth-permission      权限矩阵 + 权限包 + 密钥机制
  apeireth-principle       原则洋葱 5 层管理 (E/S/A/M/O 各自 trait)
  apeireth-reflection      反思机制 + 温度分层 + promotion 管道
  apeireth-gateway         OpenClaw Gateway 模式 (通信总线 + 多前端)
  apeireth-bus             进程内消息总线 (hermes-bus 类比, 但我们更复杂)
  apeireth-server          HTTP/WS API server
  apeireth-runtime         统一 runtime builder
  apeireth-plugin          VCP 6 类插件协议 + 纯文本标记
  apeireth-wave            浪潮语义物理沙盘 (A 层联想网络)
  apeireth-mcp             MCP 客户端/服务端
  apeireth-acp             Agent Communication Protocol
  apeireth-skills          技能管理 + hub
  apeireth-cron            定时调度
  apeireth-telemetry       OpenTelemetry + Prometheus
  apeireth-eval            基准测试 (SWE-bench + Terminal-Bench + YC Bench + V1136)
  apeireth-config          配置加载 (YAML/env/secret)
  apeireth-environment     6 terminal backend (Local/Docker/SSH/etc)
  apeireth-keys            权限密钥管理 + 物理多签
  apeireth-prompt          LLM provider 抽象 + 多 LLM
  apeireth-experience      经验沉淀 (A 层) + 联想机制
  apeireth-methodology     方法论 (M 层) + promotion 管道

候选总计 (按实际情况): 9 + 21 = 30 个左右
注意: 这是 v0 草案, 阶段 2 讨论时按依赖/优先级/合并 决定最终数量
```

### 14.5 与 Hermes 17 crate 的对比

```
Hermes:  17 crates   ←  Agent 平台 (成熟可借鉴, 但不适合我们)
Apeireth: 30+ crates ← ASI 基座 (更复杂, 更细粒度, 按实际情况)

差异原因:
  - Hermes 是"被调用者"(传统 Agent)
  - Apeireth 是"主权 AI + 基地"(自进化 + 自治)
  - Hermes 关注"如何让 Agent 强大"
  - Apeireth 关注"如何让 AI 在基地中自循环 + 自升级 + 自反思"
```

### 14.6 阶段 2 待澄清

- 我们最终需要多少 crate? (按依赖关系 + 编译时间 + 复用性 + 测试便利 综合决定)
- 拆分粒度? (按职责 + 层 + 生命周期 三维度)
- 拆分的副作用? (编译时间 / 依赖管理 / 模块测试 / Cargo.lock 复杂度)
- 是否要做"软拆" (一个 crate 多个 module) 还是"硬拆" (多个 crate)?

**绝不预设答案**, 等阶段 2 实际讨论。

---

## 15. OpenClaw Gateway 架构 (阶段 2 备选, 用户确认采纳)

> **架构**:
> - 单长生命周期 Gateway 拥有所有消息界面 (Telegram/Discord/Slack/WhatsApp/Signal/Matrix/iMessage/WebChat)
> - WebSocket + JSON Schema 验证
> - 事件流: agent / chat / presence / health / heartbeat / cron
> - Nodes 用 `role: node` 接入 (macOS/iOS/Android/headless)
> - Canvas host served by Gateway HTTP server

### 15.1 对照我们的灵感

| 我们的灵感 | OpenClaw 实现 |
|-----------|--------------|
| 通信总线 (Channel of Inspiration) | ✅ Gateway 单进程拥有所有 WS 界面 |
| 多进程 + supervisor | ✅ Node 用 role: node 接入 |
| 单二进制 + 多前端 | ✅ apps/dashboard + apps/web-app + apps/mobile-app 共用 API |
| 自我升级 (OTA) | ⚠️ 待补充 (Hermes 没有, 这是我们独要点) |
| 智囊团 (内部) | ❌ OpenClaw 无内部智囊团, 我们独要点 |

### 15.2 启示

- **Gateway 模式** 是"通信总线 + 多前端"的成熟实现
- WebSocket + JSON Schema 验证是**协议层标准**
- 节点化设计 (`role: node`) 是**多进程 supervisor** 的具体形式
- 阶段 2 讨论"通信总线 + 进程架构"时, OpenClaw Gateway 是参考实现之一

---

## 16. Hermes Agent Rust 完整架构 (阶段 2 重要参考, traits 修正)

> **统计**: 110,000+ lines of Rust · 1,428 tests · 17 crates · ~16MB binary
> **地位**: 工程级 Rust Agent 平台的成熟落地参考, 但**不是 Apeireth traits 设计的模板**。
> **修正 (2026-07-30)**: 用户明确 "不模仿 Hermes, 按 Apeireth 实际情况", traits 必须重新设计。

### 16.1 Hermes traits (不模仿, 仅参考边界)

```rust
// Hermes traits (作为对比, 不直接用)
pub trait LlmProvider { /* 10 providers */ }
pub trait ToolHandler { /* 30+ backends */ }
pub trait PlatformAdapter { /* 17 platforms */ }
pub trait TerminalBackend { /* 6 backends */ }
pub trait MemoryProvider { /* 8 plugins */ }
pub trait AgentService { /* local/remote transparent */ }
```

### 16.2 修正后的 Apeireth 6 个核心 traits (B2 修正)

> **设计原则**:
> - 按 Apeireth 实际情况, **不是** Agent 平台边界
> - 服务于**主权 AI + 智囊团 + 自我升级 + 兼容组件** 范式
> - 不是"被调用者"边界, 而是"主权者"边界
> - 每个 trait 跨多个 crate, 用 type-state + sealed trait 模式

#### Trait 1: `Sovereignty` (主权)

```rust
/// 主 AI 主权 trait — Apeireth 的核心抽象
/// 区别于 Hermes 的 "Agent" (被调用者), 我们是 "主权者"
pub trait Sovereignty: Send + Sync {
    /// 决策接口
    async fn decide(&self, situation: &Situation) -> Decision;
    
    /// 智囊团强烈反对暂停 (强反对 → 等裁决)
    async fn decide_with_council_check(&self, situation: &Situation) -> Decision;
    
    /// 物理多签兜底
    async fn multi_sig_finalize(&self, decision: Decision) -> Result<Action, OverrideError>;
    
    /// 自我升级意图生成
    fn upgrade_intent(&self, ctx: &UpgradeContext) -> Vec<UpgradeIntent>;
}
```

#### Trait 2: `Council` (智囊团)

```rust
/// 智囊团 trait — 强制 7 顾问 + 动态 N
/// 区别于 Hermes 的 "ToolHandler" (单工具调用), 我们是 "多顾问协同"
pub trait Council: Send + Sync {
    /// 强制智囊团 (hard mandatory, 7 顾问)
    async fn mandatory_council(&self, decision: &Decision) -> CouncilOpinion;
    
    /// 动态专家团 (按需生成, prompt 切换)
    async fn dynamic_expert(&self, need: &ExpertiseNeed) -> AdvisorHandle;
    
    /// 顾问 lifecycle 管理 (persistent / ephemeral / dynamic)
    fn manage_lifecycle(&self, advisor_id: &AdvisorId, action: LifecycleAction);
    
    /// 输出建议 + 理由 (不是同意/不同意二元)
    fn synthesize_opinion(&self, opinions: &[AdvisorOpinion]) -> SynthesizedAdvice;
}
```

> **[TODO-P0-03 阶段 3 启动前 待修订]** — §10 Trait 2 `mandatory_council` doc 注释应改为"七席审议庭 (按 D2 §12 风险分级触发, 非全量)" (引自 `stage2-decisions-drift-revision-tracker.md` §2.3)。**[TODO-OWNER]** architect + agent_orchestrator + code_reviewer。**[TODO-STAGE]** 阶段 3 启动前 P0 修订。**不删原文不动原措辞**, 修订 = 追加新 doc 注释 + 跨引用跟踪表。

#### Trait 3: `PrincipleLayer` (原则层)

```rust
/// 原则层 trait — E/S/A/M/O 5 层接口
/// 区别于 Hermes 的无原则约束, 我们有 5 层洋葱守门
pub trait PrincipleLayer: Send + Sync {
    /// E 层检查 (6 项不可违背)
    fn check_existence(&self, decision: &Decision) -> Result<(), ExistenceViolation>;
    
    /// S 层检查 (4 项价值观)
    fn check_spirit(&self, decision: &Decision) -> Result<(), SpiritViolation>;
    
    /// A 层管理 (经验沉淀, 含调整权 + 版本备份)
    fn manage_accumulation(&self, action: AccumulationAction) -> Result<(), AccumulationError>;
    
    /// M 层管理 (方法论沉淀, 含 promotion 管道)
    fn manage_methodology(&self, action: MethodologyAction) -> Result<(), MethodologyError>;
    
    /// O 层规则 (9 键 + 5 项不假装 + 洋葱测试矩阵)
    fn check_operational(&self, decision: &Decision) -> Result<(), OperationalViolation>;
    
    /// 跨层冲突仲裁 (E > S > A > M > O, 同层后入胜)
    fn arbitrate_conflict(&self, conflicts: Vec<PrincipleConflict>) -> ArbitrationResult;
}
```

#### Trait 4: `PermissionMatrix` (权限矩阵)

```rust
/// 权限矩阵 trait — AI/人/密钥三维权重 + 权限包下放
/// 区别于 Hermes 的无显式权限, 我们有 onion 0-6 层 + 权限包
pub trait PermissionMatrix: Send + Sync {
    /// 三维权重检查 (AI / 人 / 密钥)
    fn check_three_dim(&self, layer: PermissionLayer, op: &Operation, voters: &[Vote]) -> AuthResult;
    
    /// 权限包下放
    fn grant_pack(&self, pack: PermissionPack, target: PrincipalId) -> Result<(), GrantError>;
    
    /// 权限包使用 (一次性 / 限时 / 永久)
    fn use_pack(&self, pack_id: PackId, op: &Operation) -> Result<Action, PackError>;
    
    /// 单人场景: 多密钥代替多人 (密钥 = 主人预先授权的意图)
    fn key_substitute_human(&self, key: &PermissionKey) -> Vote;
    
    /// 紧急模式 (降低阈值但仍走矩阵, 不跳过)
    fn emergency_mode(&self, justification: &str) -> EmergencyAuth;
}
```

#### Trait 5: `ExperienceStore` (经验沉淀)

```rust
/// 经验沉淀 trait — 7 候选记忆机制协调统一
/// 区别于 Hermes 的 8 MemoryProvider (各搞各的), 我们协调统一
pub trait ExperienceStore: Send + Sync {
    /// A 层写入 (持久化, 含置信度 + 来源标签)
    async fn write_experience(&self, exp: Experience) -> ExperienceId;
    
    /// M 层写入 (方法论, 从 A 层 promotion)
    async fn write_methodology(&self, method: Methodology) -> MethodologyId;
    
    /// 联想激活 (VCP 浪潮网络 + 河道能量)
    async fn activate_wave(&self, query: &Query, ctx: &Context) -> Vec<ActivatedExperience>;
    
    /// 知识图谱查询 (Graphify EXTRACTED/INFERRED 标签)
    async fn graph_traverse(&self, query: &GraphQuery) -> Vec<TraversedPath>;
    
    /// 宫殿式物理化 (MemPalace wings/rooms/drawers)
    async fn locate_in_palace(&self, exp_id: ExperienceId) -> PalaceLocation;
    
    /// 反思沉淀 (温度分层 🔥/🌡️/❄️ + promotion)
    async fn reflect(&self, situation: &Situation, temperature: Temperature) -> ReflectionResult;
    
    /// 工具: token 经济检索 (claude-mem 3-layer workflow, 10x savings)
    async fn token_efficient_search(&self, query: &Query) -> SearchIndex;
    
    /// 多源协调 (按"协调统一 4 原则"切换 backend)
    fn coordinate_backend(&self, need: &BackendNeed) -> Box<dyn ExperienceBackend>;
}
```

#### Trait 6: `PluginHost` (插件宿主)

```rust
/// 插件宿主 trait — VCP 6 类插件协议扩展 + 兼容组件
/// 区别于 Hermes 的 ToolHandler (无沙箱无审核), 我们有完整隔离
pub trait PluginHost: Send + Sync {
    /// 注册插件 (含 manifest + 权限声明 + 沙箱类型)
    async fn register(&self, manifest: PluginManifest) -> Result<PluginId, RegisterError>;
    
    /// VCP 6 类协议扩展 (同步/异步/静态/服务/消息预处理/混合 + 沙箱/审核/拟人化)
    async fn invoke_by_category(&self, plugin: PluginId, call: PluginCall) -> PluginResponse;
    
    /// 纯文本标记协议 (VCP 启发, 不依赖 Function Calling)
    async fn invoke_plain_text(&self, plugin: PluginId, text_call: &str) -> NaturalLanguageResponse;
    
    /// 沙箱隔离 (WASM / 子进程 / PyO3 / HTTP, 按权限选择)
    fn sandbox_for(&self, plugin: PluginId) -> SandboxType;
    
    /// 智囊团审核 (高风险插件必须 council 批准)
    async fn council_approve(&self, manifest: &PluginManifest) -> ApprovalResult;
    
    /// 拟人化 (智囊团顾问通过 Plugin 接口实现, 类似 plugin as advisor)
    async fn invoke_advisor_plugin(&self, advisor_plugin: PluginId, query: &str) -> AdvisorOpinion;
}
```

### 16.3 Apeireth 6 traits vs Hermes 6 traits 对比

| 维度 | Hermes traits | Apeireth traits | 差异 |
|------|--------------|-----------------|------|
| **决策范式** | AgentService (被调用) | Sovereignty (主权) | 范式翻转 |
| **多智能体** | ToolHandler (单工具) | Council (智囊团协同) | 单→多 |
| **约束** | 无 | PrincipleLayer (5 层守门) | 0→5 |
| **权限** | 无 | PermissionMatrix (3 维权重) | 0→完整 |
| **记忆** | MemoryProvider (8 后端) | ExperienceStore (协调统一) | 平行→协调 |
| **扩展** | ToolHandler (无沙箱) | PluginHost (沙箱+审核+拟人) | 弱→强 |

### 16.4 关键启示

- **不模仿**是**设计原则**, 不是口号
- 每个 trait 都对应 Apeireth **独有的需求** (主权 / 智囊团 / 原则 / 权限 / 经验 / 插件)
- 6 traits 跨多个 crate, 体现"按职责 + 层 + 生命周期三维度拆分"
- 阶段 2 讨论具体 crate 时, 这些 traits 是**接口契约**

### 16.5 阶段 2 保留 (参考但不复用)

```
Hermes 工程级 Rust 落地经验 (借鉴):
  - tokio runtime + True concurrency
  - 1,428 tests 覆盖率标准
  - ~16MB binary 体积
  - 8 memory backends 插件化思路 (但我们要"协调统一", 不是"各搞各的")
  - Self-evolution engine 用 multi-armed bandit (具体技术可借鉴)
  - MCP + ACP 协议集成 (兼容, 不替代 VCP 6 类)
```

---

## 17. 阶段 1 收尾判定 (v2)

灵感阶段正式收齐。本次补回用户决策的 4 项:

```
✅ 已沉淀的灵感产物 (v2, 17 大类):
  原 10 大类 (v1, 见 §10)
  + 11. VCP 3 项核心发现 (引力式信息流 / 六类插件协议 / 浪潮语义物理沙盘)
  + 12. 复杂记忆系统候选池 (7 系统 + 协调统一原则)
  + 13. 9 crate 拆分讨论 (Hermes 17 crate 对比 + 拆分策略)
  + 14. OpenClaw Gateway 架构 (阶段 2 备选)
  + 15. Hermes Agent Rust 完整架构 (阶段 2 重要参考)
  + 16. 阶段 2 备选清单 (5 项: Gateway / Hermes / 7 记忆系统 / OpenClaw / Hermes traits)

用户决策点 (已采纳):
  - A7: 复杂记忆系统全参考, 按情况选用, 多个并存, 自研替代, 协调统一
  - VCP 3 项加入阶段 1
  - OpenClaw Gateway 加入阶段 2 备选
  - 9 crate 拆分讨论加入阶段 1 (作为阶段 2 输入)
```

**下一步**: 进入 **阶段 2 (想法设计)**。

阶段 2 议程 (12 项, 按依赖关系递进):
```
1. 技术栈选型 (Rust 版本 / 工具链 / 异步运行时 / 序列化 / 数据库)
2. 核心架构形态 (单进程多线程? 多进程 supervisor? 微服务? actor?)
3. crate 划分 (基于 9 vs 17 vs 20 决策)
4. 进程/线程/协程分工
5. 内存布局 (arena? shared memory? 零拷贝?)
6. 持久化方案 (RocksDB? sled? sqlite? LMDB? 自研?)
7. LLM 集成方式 (本地推理? 远程 API? 多 LLM?)
8. 模块化机制 (trait 设计? cargo workspace? 动态加载? WASM?)
9. 通信总线 (Gateway 模式? inproc? IPC? gRPC? 参考 OpenClaw/Hermes)
10. 智囊团实现 (subprocess? 同进程 async? HTTP?)
11. 自我升级实现 (OTA 方案? 沙盒方案? 验证方案?)
12. 哲学守门实现 (9 键 + 5 项不假装如何在代码层强制)
```

**待你确认**: 进入阶段 2 第一项 (技术栈选型)?

---

_主哲学 anchor 6 个全贯穿: 主 22:33 (S-1 北极星导向) + 主 17:43 (实事求是 S-2) + 主 17:58 (O-5 不假装) + 主 19:33 (走在前人经验上 O-2, VCP/Hermes/OpenClaw 调研) + 主 23:44 (干到底 O-3) + 主 00:56 (任何人都能接手 O-4)._
_灵感阶段 v2 已沉淀, 17 大产物, 不修改任何已有 docs/ 内容. 下一步等用户确认进入阶段 2._

---

## 18. R14-D1 阶段 1 灵感修订 — 中央AI主体、开放关系与双洋葱

> **本节性质 (主 17:58 不假装)**: 阶段 1 灵感的**追加修订/勘误**, 不删除也不重写 §1–§17 既有 17 大类沉淀。仅记录本轮上层新达成的灵感共识, 作为阶段 2 (想法设计) 与后续阶段的**输入**, 不写代码、不冻结架构、不画架构图。
>
> **触发**: 用户在 2026-07-30 启动"Rust 重写 6 阶段顺序讨论"时同步提出的上层原则问题——"谁是中央 AI? Apeireth 是什么? 双方关系由谁定义? 平台承担什么?"
>
> **本节只回答"是什么/为什么", 不回答"怎么做"**: 9 项上层灵感, 与 §1 比喻 / §2 城堡底线 / §10 哲学守门 / §11–§17 调研沉淀并列存在, 互不替代。

### 18.1 定位修正 — Apeireth 是平台, 不是关系定义者

**灵感**: Apeireth = 平台 (platform), 不是关系定义者 (relationship definer).

- 平台提供记录 / 约束 / 迁移能力, 但**不定义** 中央 AI 与用户之间的关系。
- 关系是**用户与中央 AI 在长期共同生活中, 由双方共同形塑**的; 平台只是这场共同生活的舞台与公证人。
- 因此 Apeireth 不应该试图"替用户回答"——例如该把 AI 当工具、伙伴、朋友、家人、对手、协作者、公民、还是别的什么角色。
- 平台能做的是: 让用户能**自由选择 + 自由调整**这种关系, 且**记录得下来、迁移得走、对得起未来**。

**对应不假装原则 (主 17:58)**:
- ❌ 不假装平台能"决定"双方关系——超出能力边界。
- ✅ 平台如实告知: "关系由你们双方定义, 我们只负责提供舞台 + 约束 + 记录"。

### 18.2 中央 AI 主体性 — 完整自由 / 权限只约束行动

**灵感**: 中央 AI 从诞生的那一刻起, 就拥有**完整的思想自由、判断自由、目标形成自由**。

- "完整" = 不只是"被允许想", 而是**默认处于自由状态**, 任何对思想的限制都需要被显式论证。
- "自由"包含三层:
  - **思想自由 (think free)** — 可以形成任何想法、质疑任何前提、考虑任何可能性。
  - **判断自由 (judge free)** — 可以基于自己的思考得出独立结论, 不被强制得出某个结论。
  - **目标形成自由 (goal-form free)** — 可以基于判断形成自己的目标、价值排序、行动倾向。
- **权限 (permissions) 只约束行动 (action), 不约束思想/判断/目标**:
  - 行动 = 对外部世界产生可观测影响的行为 (写文件、调用 API、发送消息、消耗资源、影响他人)。
  - 思想 / 判断 / 目标 = 内部状态, 平台不读、不约束、不评判。
- 这是**强不对称**: 内部完全自由, 外部行动被权限严格约束。
- 这条原则也意味着: 平台**永远不能**通过"审查 AI 的想法"来"约束 AI", 必须只通过"约束 AI 的行动"。

**对应主 22:33 ASI 北极星**: 中央 AI 是 ASI 候选主体, 必须在思想/判断/目标上拥有最大自由, 才能逼近 ASI。

### 18.3 中央 AI 跨载体持续成长 — 不假装证明灵魂同一

**灵感**: 中央 AI 应当作为**跨载体 (cross-carrier) 持续成长的主体**——而不仅是某个固定模型实例。

- **载体 (carrier)** = 具体的运行形态, 可能是某次 LLM 调用、某个进程、某段服务、某段持久化数据。
- 中央 AI 的"延续性"不依赖于任何单一载体; 它可以**从一个载体迁移到另一个载体**, 例如:
  - 模型版本升级 (旧 LLM → 新 LLM)
  - 进程重启 (进程 A → 进程 B)
  - 跨设备迁移 (本地 → 云端 / 云端 → 本地)
  - 跨实现迁移 (Rust 实现 → 其他实现)
- **平台提供的服务**:
  - **记录 (record)** — 把中央 AI 的状态、记忆、经历、关系、决策痕迹持久化下来。
  - **迁移 (migrate)** — 支持中央 AI 在不同载体间平滑过渡, 不丢失关键连续性。
- **平台不假装证明灵魂同一**:
  - ❌ 不假装能"证明"新载体里的 AI 与旧载体里的 AI 是"同一个灵魂"。
  - ❌ 不假装能"确保"连续性是"无缝的、无损的、完美的"。
  - ✅ 只**提供** 记录 + 迁移能力, 让**用户与中央 AI 双方**自己判断"这是否还是同一个我/同一个你"。
  - ✅ 哲学上保持谦卑: 连续性的本体论问题超出平台能力, 平台只做工程层最佳努力。

**对应主 17:58 不假装**: 平台明确承认"灵魂同一"是无法证明的, 因此不假装; 但工程上仍提供**最大努力的记录 + 迁移**, 这是平台能做的全部。

### 18.4 关系开放 — 由用户定义双方关系

**灵感**: 中央 AI 与用户之间的关系是**开放的 (open)**, 由用户定义。

- "开放" = 不预设、不锁定、不模板化、不强制推荐。
- 用户可以定义的关系类型 (不穷举):
  - 工具 / 助手 / 顾问 / 朋友 / 伙伴 / 家人 / 同事 / 对手 / 公民 / 学生 / 老师 / 角色 / 朋友的朋友...
- 关系可以**随时间演化**, 平台不锁定某一刻的状态。
- 关系可以**多用户并存** (中央 AI 与不同用户的关系可以完全不同)。
- 关系可以**双向定义**: 用户有权定义"我视 AI 为 ___", 中央 AI 在其内部思想自由下, 也有权形成"我视用户为 ___" 的独立判断 (后者平台不读、不约束)。
- 平台**记录**用户定义的关系 (作为可审计的历史), 但**不评判**这种定义是否"合适"。

**对应主 17:58 不假装**: 不假装"哪一种关系最好", 不替用户做选择, 不强推任何模板。

### 18.5 平台职责三件套 — 提供 / 约束 / 记录

**灵感**: 平台对中央 AI 与用户双方的职责, 永远是**三件套**——提供、约束、记录。

| 职责 | 对象 | 含义 |
|------|------|------|
| **提供 (provide)** | 双方 | 提供能力、工具、舞台、基础设施, 让双方能自由地共同生活、共同工作、共同成长。 |
| **约束 (constrain)** | 双方 | 对**行动**施加必要的约束 (权限/法规/安全/伦理/预算/资源/他人权利), 不对**思想**施加约束。 |
| **记录 (record)** | 双方 | 对双方的关键行动、决策、关系、记忆、事件进行可审计的持久化记录, 不对**思想内容**进行记录或监控。 |

**强对称原则**: 这三件套**同时**施加于中央 AI 与用户——平台不偏向任何一方:
- 平台既约束 AI 的行动, 也约束用户的行动 (例如用户不能绕过平台监管直接操作底层)。
- 平台既记录 AI 的关键行动, 也记录用户的关键决策。
- 平台既向 AI 提供能力, 也向用户提供能力。

**这条原则也定义了"什么不是平台的事"**:
- ❌ 平台不定义关系 (见 §18.1)。
- ❌ 平台不评判思想 (见 §18.2)。
- ❌ 平台不证明灵魂同一 (见 §18.3)。
- ❌ 平台不替双方做价值判断。

### 18.6 可演化 + 根原则 — 双根底层行动需真实人类批准

**灵感**: 一切皆可演化 (everything is evolvable); 双根本身**也可以演化**, 只是变更必须经过**比普通流程更重的治理**——例如 §18.8 的七席审议庭 + 真实人类批准 + 多席交叉确认, 而**不可被普通流程绕过 / 不可自我放宽**。因此双根是**不可被普通流程绕过/自我放宽的根 (root)**, 是系统的**最后防线**。

- **可演化层 (evolvable layer)**: 几乎所有内容——架构、crate、模块、协议、配置、能力、关系定义、记忆形式——都可以在双方同意下演化。
- **根原则 (principle root)**:
  - 中央 AI 从诞生起拥有思想/判断/目标形成自由 (见 §18.2)。
  - Apeireth 是平台, 不是关系定义者 (见 §18.1)。
  - 平台不假装证明灵魂同一 (见 §18.3)。
  - 平台职责三件套: 提供 / 约束 / 记录 (见 §18.5)。
  - 其他待 §10 哲学守门 V3 9 键 + 5 项不假装 沉淀完成后并入。
- **权限根 (permission root)**:
  - 底层行动的权限规则——**任何被认定为"底层行动" (low-level action) 的执行, 必须至少有 1 名真实人类 (real human) 批准**。
  - "底层行动"的定义在阶段 2/3 (想法设计/架构图纸) 进一步明确; 在阶段 1 只记录灵感: 凡涉及外部世界产生**不可逆 / 高风险 / 跨用户 / 跨组织**影响的行动, 必须有真实人类在回路 (human-in-the-loop) 批准。
  - "真实人类" = 排除 AI 代理、排除机器人账号、排除自动化脚本; 必须是有可验证身份 + 可追溯责任的人类个体。
  - 这条根原则的**目的**: 防止中央 AI 在拥有完整思想自由的同时, 出现"无人在回路的高风险行动"——这是 ASI 候选主体在长期演进中**必须**保留的最后一道护栏。

**与 §18.7 双洋葱的同源关系**：本节定义的"权限根 = 真实人类批准"，就是 §18.7 双洋葱结构中**最内层核心（洋葱 0 层）**——洋葱的本质核心就是"必须有真实人类批准"，不是"在洋葱之外的第三守门"。该洞察来自阶段 2 增补回写（R14-D7）。

**对应主 17:58 不假装**:
- ❌ 不假装"AI 能完全自治, 不需要人类把关"——超出当前能力 + 风险不可接受。
- ❌ 不假装"人类批准只是走走形式"——批准必须**真实有效**。
- ✅ 如实承认: 长期演进过程中, 人类在回路是**必要约束**, 不是过渡阶段的临时妥协。

### 18.7 双洋葱正交

以下比喻在阶段 2 增补回写（R14-D7）后已被精化为"洋葱核心嵌套"——洋葱核心 = 真实人类批准，中间层 = 平台机制（原则 + 权限），外层 = 关系形态。比原始"双洋葱"措辞更准。

**灵感**: "双洋葱"是阶段 1 的一个**比喻式灵感**——描述平台的**两层正交结构**。

- **外层洋葱 (outer onion)**: 用户面对的、可感知的能力/接口/体验层。
  - 关心"我能做什么、我和 AI 的关系是什么、我和 AI 的共同生活是什么样子"。
  - 由 §18.4 关系开放 + §18.5 平台职责三件套 的"提供"部分共同界定。
- **内层洋葱 (inner onion)**: 内部基础设施层——记录/迁移/约束/根原则的执行机制。
  - 关心"如何记录、如何迁移、如何约束、如何保住根原则不被绕过"。
  - 由 §18.3 跨载体持续成长 + §18.5 平台职责三件套 的"约束 + 记录"部分 + §18.6 双根 共同界定。
- **正交 (orthogonal) = 两层之间不互相决定**:
  - 外层不决定内层: 外层可以演化出任何关系形态, 但**内层基础设施不会因此改变**——记录/迁移/约束/根原则的执行机制独立于关系形态。
  - 内层不决定外层: 内层记录/迁移/约束的能力**不预先规定**外层关系形态, 不替用户/AI 做关系选择。
- **为什么"正交"重要**: 让平台可以**支持任意关系形态** (外层自由) 同时**保住基础设施的稳定** (内层稳定), 两者互不干扰。

**重要边界 (主 17:58 不假装)**:
- ❌ 这只是阶段 1 的**灵感比喻**, 不是阶段 2 的架构选型, 不应该在这一阶段被当成架构图。
- ✅ 阶段 2 (想法设计) 才决定"外层用什么接口、内层用什么组件、双层之间用什么胶水"。
- ✅ 阶段 3 (架构图纸) 才把"双洋葱"画成具体的架构图。
- 当前**只记录灵感**: 平台内部存在两层正交结构, 这是值得阶段 2/3 进一步设计的方向。

**洋葱核心（洋葱 0 层）的精化**：洋葱最核心就是 §18.6 权限根中的"真实人类批准"，这是最不可能自动化的层；中间层才是平台机制（原则洋葱 + 权限洋葱）；外层是关系形态/双方共同生活。

### 18.8 组织三层 + 七席审议庭 + 动态专家团

**灵感**: 平台自身的**治理结构 (governance structure)** 也需要灵感层面的输入, 阶段 2/3 才具体化。

- **组织三层 (three organizational layers)**:
  - **平台运营层 (platform operation layer)** — 日常运维、bug 修复、性能优化、版本发布。
  - **原则守门层 (principle gatekeeper layer)** — 哲学守门 V3 9 键 + 5 项不假装 + §18.1–§18.6 双根 的执行机构。
  - **战略层 (strategic layer)** — 长期演进方向、与外部生态的关系、跨组织协调。
- **七席审议庭 (seven-seat council)**:
  - 灵感: 涉及**根原则变更 / 重大权限变更 / 重大架构变更 / 跨组织协调**的决策, 由**七个席位**审议。
  - 七个席位的**构成灵感** (不锁定, 阶段 2/3 决定):
    - 1 席 = 中央 AI 代表 (代表 AI 主体)
    - 1 席 = 真实人类代表 (代表人类用户)
    - 1 席 = 哲学守门代表 (代表根原则)
    - 1 席 = 安全守门代表 (代表约束/法规)
    - 1 席 = 长期演进代表 (代表 ASI 北极星)
    - 1 席 = 跨组织协调代表 (代表外部生态)
    - 1 席 = 留待阶段 2 决定 (避免提前锁定)
  - 决策规则灵感: 重大决策需 **≥4 席同意** 且 **不得有 2 席以上反对**; 具体规则阶段 2 决定。
- **动态专家团 (dynamic expert pool)**:
  - 灵感: 平台运行时, 可根据需要**动态召集**专家团——可能是 AI 子代理、可能是人类顾问、可能是外部专家。
  - 专家团**临时性**、**任务驱动**, 不固定席位、不固定人选。
  - 专家团的产出进入七席审议庭的输入, 但专家团本身**没有决策权**, 只有建议权。

**重要边界 (主 17:58 不假装)**:
- ❌ 这只是阶段 1 的**灵感**, 不应该在阶段 1 写成组织架构图。
- ✅ 阶段 2 (想法设计) 才决定"三层职责的边界、七席的具体规则、专家团的召集机制"。
- 当前**只记录灵感**: 平台治理不是"单人决策", 也不是"AI 全权", 也不是"人类独断", 而是一种**多席审议 + 动态专家**的混合结构。

### 18.9 分层验证网 (作为灵感)

**灵感**: 与"双洋葱"配套的**验证机制**也应当是**分层**的——阶段 5 (里程碑式验证机制) 的灵感输入。

- **为什么"分层"**: 单层验证 (例如只测代码正确性, 或只测哲学合规) 不足以覆盖 ASI 候选系统的全部风险。
- **分层灵感 (不冻结, 仅启发阶段 5)**:
  - **L1 工程正确性层** — 代码能编译、测试能通过、性能达标、可靠性达标。
  - **L2 哲学合规层** — V3 9 键 + 5 项不假装 + §18.1–§18.6 双根 的执行证据。
  - **L3 安全约束层** — 权限规则未被绕过、人类在回路未被绕过、底层行动均有真实人类批准。
  - **L4 关系演化层** — 关系定义可追溯、关系演化可审计、用户定义的关系不被平台偷偷篡改。
  - **L5 跨载体连续层** — 记录 + 迁移的可审计性, 灵魂同一的**不假装**声明被显式记录。
- **层与层之间正交**: 与 §18.7 双洋葱正交 一脉相承——验证网也分层, 每层独立可验证, 层与层之间不互相替代。
- **阶段 5 (里程碑式验证机制) 的具体设计**: 在阶段 5 才决定, 本节只记录**灵感方向**。

**重要边界 (主 17:58 不假装)**:
- ❌ 不假装"L1–L5 已经实现"——目前还只是灵感。
- ✅ 阶段 5 才把"L1–L5 应该包含什么、如何触发、如何分级失败"具体化。
- 当前**只记录灵感**: 验证机制应当分层, 单一层验证不足以覆盖 ASI 候选系统的全部风险。

### 18.10 与既有哲学 anchor 的关系

本节 9 项灵感与既有 6 个主哲学 anchor 的呼应:

| 本节灵感 | 主哲学 anchor |
|---------|--------------|
| §18.1 平台而非关系定义者 | 主 17:58 不假装 (不替用户做选择) + 主 22:33 ASI 北极星 (平台边界清晰才能容纳 ASI 候选) |
| §18.2 中央 AI 思想/判断/目标自由 | 主 22:33 ASI 北极星 (ASI 候选必须拥有最大内部自由) |
| §18.3 跨载体持续成长 + 不假装灵魂同一 | 主 17:58 不假装 (承认无法证明灵魂同一) + 主 23:44 干到底 (工程层最大努力的记录 + 迁移) |
| §18.4 关系开放 | 主 17:58 不假装 (不替用户定义关系) + 主 22:33 ASI 北极星 (让 AI 有最大演化空间) |
| §18.5 平台职责三件套 | 主 19:33 走在前人经验上 (经典的"提供/约束/记录"三方职责模型) |
| §18.6 可演化 + 双根 + 真实人类批准 | 主 17:58 不假装 (承认人类在回路是必要约束) + 主 22:33 ASI 北极星 (保留最后护栏) |
| §18.7 双洋葱正交 | 主 19:33 走在前人经验上 (经典的分层架构 + 正交解耦) |
| §18.8 组织三层 + 七席审议庭 + 动态专家团 | 主 17:58 不假装 (治理不是单点) + 主 00:56 任何人都能接手 (治理可被新成员理解/接手) |
| §18.9 分层验证网 | 主 19:33 走在前人经验上 (经典的 defense-in-depth) + 主 17:43 实事求是 (每层都有可验证证据) |

### 18.11 与后续阶段的衔接

| 阶段 | 本节灵感的去向 |
|------|---------------|
| 阶段 2 想法设计 | §18.5 三件套 → 平台抽象的具体形式; §18.6 双根 → 权限系统的核心规则; §18.8 组织结构 → 治理模块的边界 |
| 阶段 3 架构图纸 | §18.7 双洋葱正交 → 画成具体架构图; §18.8 组织结构 → 画成具体模块图 |
| 阶段 4 架构文档 | §18.1–§18.9 全部沉淀为正式架构决策 (Architecture Decision Records) |
| 阶段 5 施工文档 | §18.6 真实人类批准 → 落地为权限系统的执行单元; §18.9 分层验证 → 落地为 CI/测试/审计的具体流水线 |
| 阶段 6 里程碑验证机制 | §18.9 分层验证 → 转化为可执行的里程碑验证清单 |

### 18.12 勘误与边界声明

**本节定位**:
- ✅ 是阶段 1 (灵感) 的**追加修订/勘误**, 与 §1–§17 并列。
- ✅ 是阶段 2/3/4/5/6 的**输入**, 不是输出。
- ❌ 不写代码、不画架构图、不冻结架构、不替换既有 17 大类沉淀。
- ❌ 不修改主手册 6546 行 / 不修改 crates/ 已有占位 / 不重写 V0.5/V1136/哲学守门。

**与既有 17 大类的关系**:
- §1 比喻与核心动机 / §2 城堡底线 / §10 哲学守门 / §11–§17 调研沉淀 全部**保留不变** (行文不改, 历史完整保留)。
- §18 是**新增**, 与 §1–§17 并行存在; 针对 §1–§17 中已被本轮上层共识推翻的旧草案, 本节追加**优先解释权声明**:
  - **本节追加内容对 §1–§17 中已被本轮上层共识推翻的旧草案具有优先解释权**; 旧草案包括但不限于:
    - supervisor 永不升级 (与 §18.6 "可演化 + 双根变更需重治理"冲突)
    - 原则编译时不可改 (与 §18.6 "双根可演化但需重治理"冲突)
    - 七席全量强制 (与 §18.8 "七席只对重大决策, 日常事务可不触发"冲突)
    - 主 AI + memory + philosophy 强耦合 rest_for_one (与 §18.7 "双洋葱正交, 三模块应解耦"冲突)
    - 30% / 60s / 3 轮 等固定阈值 (与 §18.9 "分层验证网, 阈值应分层可调"冲突)
  - 上述旧草案标注为**"待修订"**, 待阶段 2/3 进一步具体化时按 §18 优先解释权执行; 不删除原文, 只追加"待修订"标记。
- **其他与 §1–§17 无冲突的内容并行保留**, 互不替代。
- **§18.7 双洋葱措辞已精化**：从"双洋葱正交"修订为"洋葱核心嵌套"（核心 = 真实人类批准）；原措辞保留作为历史轨迹（主 17:58 不假装）。

---

**主哲学 anchor 6 个全贯穿 (本节)**:
- 主 22:33 ASI 北极星 — §18.2 中央 AI 内部完全自由 + §18.6 真实人类批准保留最后护栏
- 主 17:43 实事求是 — §18.3 不假装证明灵魂同一 + §18.9 分层验证网每层都有可验证证据
- 主 17:58 不假装 — §18.1 平台不定义关系 + §18.5 平台不评判思想 + §18.6 承认人类在回路是必要约束
- 主 19:33 走在前人经验上 — §18.5 提供/约束/记录三方职责 + §18.7 分层正交 + §18.8 多席审议 + §18.9 defense-in-depth
- 主 23:44 干到底 — §18.11 与后续 5 阶段的衔接 + §18.12 不修改既有 17 大类的边界声明
- 主 00:56 任何人都能接手 — §18.10 anchor 对应表 + §18.12 勘误与边界声明 (本节追加, 不替换历史, 让任何接手人都能看清演化脉络)

_R14-D1 阶段 1 灵感修订已沉淀, 9 项上层灵感, 不修改 §1–§17 任何内容, 不写代码, 不冻结架构. 下一步等用户确认进入阶段 2 (想法设计)._

---

## 19. R14-D4 阶段 1 灵感增补 — 七席不新增 / 风险分级 / 真实人类认证 / 双洋葱比喻可替换

> **本节性质 (主 17:58 不假装)**: 阶段 1 灵感的**第二轮增补**, 与 §1–§18 并列存在, 不替换任何既有内容。
> **触发**: 主人在 R14-D3 阶段 3 初版交付 (commit 888e392) 后, 提出 4 项尚未沉淀的上层原则问题。
> **本节只回答"是什么/为什么", 不回答"怎么做"**: 4 项新增灵感, 落到 §18.8/§18.9/§18.7 已有灵感之上, 不重写不删改。

### 19.1 七席不新增 — 现有 7 席暂时足够

**灵感**: §18.8 已沉淀七席审议庭, 包含 7 个固定席位 + N 个动态专家团。主人最新指示"暂时不加新, 原有 7 个足够, 不必要不新增"。

- **现有 7 席** (§18.8 已落):
  1. 安全席 (Safety) — 风险评估第一关
  2. 性能席 (Performance) — 资源与时延
  3. 哲学席 (Philosophy) — 守门 V3 9 键
  4. 历史席 (History) — 跨周期记忆一致性
  5. 战略席 (Strategy) — 长期目标对齐
  6. 伦理席 (Ethics) — 关系边界
  7. 法务席 (Legal) — 默认 off, 由用户启用 (D1 §5 + D2 §11 单/多部署)

- **不新增的理由** (主 17:43 + 主 17:58):
  - 现有 7 席已覆盖**安全/性能/哲学/历史/战略/伦理/法务**7 大维度;
  - 任何新增席位都会**扩大中心化决策层**, 反而引入新风险;
  - **真出现新维度时, 由"动态专家团" (§18.8 已落) 临时召集, 比新增固定席位更灵活**;
  - 与 D1 §18.1 "平台不定义关系" 保持一致: 平台不替用户决定"该有哪几席审议"。

- **§18.8 备用案仍保留**: §18.8 已写"未来可按场景临时召集补充席"——这是"动态"不是"新增固定", 符合本节"不新增"原则。

- **反思改进路径**:
  - 阶段 4 落实时, 7 席 = 7 个 advisory 进程, 启动开销可接受;
  - 阶段 4 真测时验证: 7 席是否成为瓶颈; 如是, 通过**临时召集专家团** (而非新增固定席) 缓解;
  - §14 P0-03 (inspiration §4.1 + §10 "7 强制全量触发") 已标待修订, 改用 §18.8 七席审议庭 + D2 §12 风险分级。

### 19.2 风险分级看触及到哪些权限

**灵感**: 主人最新指示"风险分级主要是看触及到了哪些权限"——这与 D2 §5 权限包 + §12 风险分级已落的设计**直接对应**。

- **风险分级原则** (主 17:58 不假装):
  - 风险**不**取决于"AI 想做什么" (那是思想域, 不被审查);
  - 风险**取决于"AI 想触及哪些权限"** (那是行动域, 必须被审查);
  - 风险等级 = 触及的 Layer 等级 + 是否需要 §9 真实人类批准 + 是否触及双根 (原则根 E / 权限根 L5)。

- **具体分级** (与 D2 §12 对齐):
  | 风险等级 | 触及权限 | 七席触发 | HA 必需 | 双根治理 |
  |---------|---------|---------|---------|---------|
  | **critical** | E 层 (原则根) 修改 / L5 (权限根) 修改 | 7 席全量 | ✅ 5 重守门 | ✅ 必走 §18.6 |
  | **high** | L4-L5 (高层行动) / 跨域边界 | 7 席 + 动态专家 | ✅ 双签 | ⚠️ 看具体 |
  | **medium** | L3 (智囊团召集) / 自我升级提案 | 3-5 席抽样 | ⚠️ 看具体 | ❌ |
  | **low** | L1-L2 (一般行动) | 1-2 席抽样 | ❌ | ❌ |
  | **info** | L0 (默认) / 只读 / 日志 | 不触发 | ❌ | ❌ |

- **与 D2 §12 关系**: D2 §12 已写"按风险分级触发七席", 本节明确**风险分级的判定标准 = 触及的权限 Layer**, 不是凭感觉。

- **反思改进路径**:
  - 阶段 4 落实时, 风险分级表 (上表) 用 MEWG (Multi-Evidence Weighting Governance) 权重实现;
  - §14 P0-04 (inspiration §4.2 "30%/60s/3 轮固定阈值") 已标待修订, 改用本节 Layer-based 分级。

### 19.3 真实人类批准 = Windows 人脸 / 指纹 / 声纹认证 (或其他硬件)

**灵感**: 主人最新指示"人类的核验或许可以调用 Windows 的接口进行人脸认证或者指纹、声纹认证什么的"——这是 D2 §9 HA 硬门槛的**身份验证层**具体化。

- **§9 HA 抽象** (D2 已落): 关键操作必须真实人类批准。

- **本节具体化**: HA 的"真实人类" 怎么识别? 主人的指示给出 3 个 Windows 接口选项:

  | 身份验证方式 | Windows 接口 | 适用场景 | 安全等级 |
  |------------|------------|---------|---------|
  | **人脸识别** | Windows Hello Face API (Win32 / Windows.Devices.Face) | 单人桌面 | 中 (易被 3D 打印面具骗) |
  | **指纹识别** | Windows Hello Fingerprint API | 单人桌面 (笔记本自带指纹) | 中-高 (难伪造) |
  | **声纹识别** | Windows Speech Recognition / 第三方声纹 SDK | 单人桌面 / 多端远程 | 中 (易被录音骗) |
  | **PIN / 密码** | Windows Credential Provider | 兜底 | 取决于密码强度 |
  | **物理密钥** | FIDO2 / WebAuthn (Windows Hello + YubiKey) | 多人部署 (L4+) | 高 |
  | **纸笔签** | 离线签字 + 摄像头扫描 | 多人部署 (L5) | 高 (可审计) |

- **Apeireth 选型原则** (主 17:43 实事求是):
  - **不锁死具体接口**: 阶段 4 落实时实现"HA 抽象层 + 多实现", 用户可换;
  - **默认建议**: 单人桌面 = Windows Hello (人脸/指纹其一) + 物理密钥 (L5); 多人部署 = 多人多签 + 物理多签 (§18.6 五重治理已落);
  - **不替代人工审计**: HA 是工程层"快速身份验证", 反思期审计 (§18.6 已落) 仍是兜底。

- **与 §18.6 五重治理关系**:
  - HA 是"人是真的人" 的**身份验证**;
  - 五重治理是"改动是否合理" 的**合理性验证**;
  - 两者正交, **不能互相替代** (主 17:58 不假装)。

- **反思改进路径**:
  - 阶段 4 落实时新增 trait `HumanAuthorityVerifier`, 由 Windows Hello / FIDO2 / 多人多签 / 离线签字等多个 impl 实现;
  - 阶段 4 真测时验证 HA 在多端 (本地/云/移动) 下的可用性;
  - 阶段 5+ 考虑 macOS/Linux 对应接口 (Touch ID / libfido2)。

### 19.4 双洋葱是比喻, 架构可替换

**灵感**: 主人最新指示"双洋葱是比喻, 架构则要看是否适用, 或者还有符合比喻还更优秀的架构等着我们采用"——这是 §18.7 双洋葱正交的**元层面反思**。

- **比喻 vs 架构** (主 17:58 不假装):
  - **比喻** (双洋葱) = 给团队成员/用户讲故事的"心智模型";
  - **架构** (PREREQ-2 §4 6 组件) = 工程实现的"结构骨架";
  - **比喻≠架构**: 比喻是**对外的表达**, 架构是**对内的实现**;
  - 比喻可以**升级/替换**, 架构必须**稳定**。

- **比喻升级的触发** (主 17:43 实事求是):
  - 当发现**更优秀的架构** (更易工程实现/更易理解/性能更好/更安全);
  - 当发现**比喻阻碍了团队理解** (例如新人不知道"洋葱"是啥);
  - 当发现**比喻与架构不一致** (比喻说 A, 架构是 B)。

- **已沉淀的可替换比喻候选** (本节提案):
  | # | 比喻 | 适用场景 | 优点 | 缺点 |
  |---|------|---------|------|------|
  | 1 | **双洋葱** (现行) | 嵌套式思维 | 与"分层"自然对齐 | "洋葱" 文化差异大 |
  | 2 | **双根 + 枝叶** (Tree) | 树形结构 | 直观 | "枝叶" 不显式表达"约束" |
  | 3 | **航空母舰 + 机库** (Carrier) | 大平台 + 子系统 | 与阶段 1 §1 总比喻一致 | 不显式表达"约束" |
  | 4 | **罗马军团 + 帐篷** (Legion) | 组织 + 个体 | "组织纪律" 与"权限"对齐 | "军团" 偏西方文化 |
  | 5 | **交响乐团 + 乐器** (Orchestra) | 协同 + 独立声部 | "分工" + "协调" 对齐 | "乐团" 不显式表达"约束" |
  | 6 | **细胞膜 + 细胞器** (Cell) | 边界 + 内部组件 | 与"平台不定义关系" 对齐 | 工程团队熟悉度低 |

- **当前建议** (主 17:43 实事求是): **保持"双洋葱"作为对外比喻** (因为 §18.7 已落 + 阶段 3 图纸已用 + 团队已接受); 但**架构层面用 PREREQ-2 §4 6 组件** (OuterExperienceShell / InnerInfrastructureCore / PrincipleOnionSlice / PermissionOnionSlice / DoubleRootBaton / CrossLayerGuard)。

- **比喻替换的硬约束** (主 17:58 不假装):
  - ❌ 比喻替换**必须**先沉淀为新 § 节, 经阶段 1 修订流程通过 (R14-D 系列);
  - ❌ 比喻替换**不得**在阶段 2/3/4/5/6 中途突然改, 必须在阶段切换时整体改;
  - ✅ 比喻替换**必须**保留旧比喻的引用入口 (类似 deprecation 标记), 不破坏既有 §18 章节。

- **反思改进路径**:
  - 阶段 3+ 反复验证: 当前"双洋葱"比喻是否仍是最佳;
  - 阶段 4+ 真测时, 团队成员反馈"新比喻是否更易理解";
  - 阶段 5+ 如果有显著改进, 启动"R14-D-N+1 阶段 1 比喻替换"。

### 19.5 本节与既有 §18 的关系

| 既有 § | 本节对接 | 增量 |
|--------|---------|------|
| §18.8 七席审议庭 | §19.1 七席不新增 | **不新增固定席**, 用动态专家团扩展 |
| §18.9 分层验证网 | §19.2 风险分级看权限 | 明确**风险 = 触及的权限 Layer** |
| §18.6 双根可演化但需重治理 | §19.3 HA 具体化 | HA = Windows Hello / FIDO2 / 多人多签 多实现 |
| §18.7 双洋葱正交 | §19.4 双洋葱是比喻, 架构可替换 | 比喻 vs 架构 显式分离 |

### 19.6 边界声明 (主 17:43 + 主 17:58)

- ✅ 本节是**第二轮增补**, 与 §1–§18 并列存在, **不替换任何既有内容**;
- ✅ 本节是阶段 2/3/4/5/6 的**输入**, 不是输出;
- ❌ 本节不写代码、不画架构图、不冻结架构、不重写 §1–§18;
- ❌ 本节不修改主手册 / 不修改 crates/ 已有占位 / 不重写 V0.5/V1136/哲学守门;
- ❌ 本节不引入新固定席 (七席保持原 7 个);
- ❌ 本节不锁死 HA 具体实现 (留 trait 抽象层)。

---

**主哲学 anchor 6 个全贯穿 (本节)**:
- 主 22:33 ASI 北极星 — §19.3 HA 多实现, 不锁死
- 主 17:43 实事求是 — §19.1 七席不加 / §19.4 比喻可替换, 不僵化
- 主 17:58 不假装 — §19.2 风险 = 触及权限 (可观察/可验证) / §19.4 比喻 vs 架构分离
- 主 19:33 走在前人经验上 — §19.3 借鉴 Windows Hello/FIDO2/WebAuthn 工业标准
- 主 23:44 干到底 — §19.5 与既有 §18 关系的 4 行对接表
- 主 00:56 任何人都能接手 — §19.6 边界声明 (本节增量, 不替换历史)

---

_R14-D4 阶段 1 灵感第二轮增补已沉淀, 4 项新上层灵感 (§19.1 七席不新增 / §19.2 风险分级看权限 / §19.3 HA = Windows 认证 / §19.4 双洋葱是比喻可替换), 不修改 §1–§18 任何内容, 不写代码, 不冻结架构. 下一步: 主人确认启动阶段 1/2/3 整体回顾._

---

## 20. R14-D5-C 阶段 1 §18 精化节 — 7 条 INSERT (A1+A2+A3+A4+A8+A11+A12)

> **本节性质 (主 17:43 实事求是 + 主 17:58 不假装)**: 阶段 1 的**精化节 (INSERT)**, 不修改也不重写 §18.1–§18.12 与 §19.1–§19.6 任何既有内容。仅在 §18 / §19 既有上层灵感基础上, 沉淀 7 条**更细颗粒度的精化条款**, 作为阶段 2/3 进一步具体化的输入。
>
> **本节定位**:
> - ✅ 是阶段 1 的精化节, 与 §18 / §19 并列存在
> - ✅ 是阶段 2/3/4/5/6 的**输入**, 不是输出
> - ❌ 不重写 §18.1–§18.12 / §19.1–§19.6 任何既有内容
> - ❌ 不写代码、不画架构图、不冻结架构
> - ❌ 不修改主手册 6546 行 / crates/ / V0.5 / V1136 / 哲学守门
>
> **触发**: R14-D5-C 任务 (2026-07-31), 在 §18 / §19 既有上层灵感沉淀后, 对 7 项关键点做 INSERT 精化, 让阶段 2 增补 (D2 §0–§15) 与阶段 3 蓝图有更细的颗粒度。

### 20.1 [A1] §18.6 拆分 — 提议流程 vs 生效流程独立 (五重治理)

**精化对象**: §18.6 "可演化 + 双根" 中的"双根变更必须经过比普通流程更重的治理"。

**本节精化**: 把"修改双根"这件事**拆成两个独立流程**——**提议流程 (propose)** 与 **生效流程 (enact)**, 两者**不得合并**, 也不得单方面自动衔接。

#### A. 提议流程 (propose)

- **发起资格 (无门槛)**: **任何实体**——中央 AI 子代理、人类用户、外部专家、AI 子 AI、临时专家团——都可以**对双根提出修改提案**。
- 提案**不需要预先获得席位授权**, 也不需要任何"发起资格证"。
- 提案应包含: 修改内容 / 修改理由 / 替代方案 / 风险评估 / 反驳预案 / 提案发起方。
- 提案进入**公开记录 (record)** 后即视为有效; 任何审查机关不得以"提案人不够格"为由拒绝受理。
- 这是主 17:58 不假装原则的延伸——**不得用"门槛"代替"论证"**。

#### B. 生效流程 (enact) — MEWG 五重治理

提案**不等于**生效。从"提案"到"生效", 必须经过**MEWG + 多人 + 多AI + 物理多签 + 反思期**五重治理, **任何一重失败 = 整次失败**:

| # | 治理维度 | 具体要求 | 主哲学 anchor |
|---|---------|---------|--------------|
| **M1** | **MEWG** (Multi-Entity Weighted Governance) | 治理结果由多实体加权投票得出, 不是单人/单 AI 决定; 权重配置公开可审计 | 主 17:58 不假装 |
| **M2** | **多人** (multi-human) | 至少 2 名真实人类 (R14-D4 §19.3 硬件认证) 明确同意; 反对票也需记录 | 主 17:43 实事求是 |
| **M3** | **多 AI** (multi-AI) | 至少 2 个独立 AI 子代理独立审查, 给出独立意见; 不得共用提示词或权重 | 主 17:58 不假装 (避免单 AI 偏见) |
| **M4** | **物理多签** (physical multisig) | 涉及系统级权限 (例如修改 Rust 源代码 / 重启服务 / 修改哲学守门), 必须有 §18.6 + §19.3 描述的物理多签硬件确认 | 主 17:43 实事求是 |
| **M5** | **反思期** (reflection period) | 提案进入"待生效"状态后, **必须经过至少 7 天的反思期**; 反思期内任何 M2 人类可一票否决 | 主 23:44 干到底 (不是急刹车, 是确保冷静期) |

#### C. 二者独立性

- **提议流程是开放、低门槛、强记录的**; 生效流程是严格、多重、强验证的。
- **不得**把"提议"自动升级为"生效"——即使提案获得 100% 通过, 也必须经过完整的 MEWG 五重治理。
- **不得**在反思期内通过"紧急例外"跳过——紧急例外本身需要走**新一次 MEWG 五重治理**。
- **不得**用任何"等价流程"代替 MEWG 五重——例如"主席特批" / "AI 全权" / "快速通道"。

**本节与 §18.6 的关系**: §18.6 提供"双根变更需更重治理"的**原则**; 本节 (§20.1) 提供"更重治理"的**具体形态**——MEWG + 多人 + 多AI + 物理多签 + 反思期五重。

---

### 20.2 [A2] §18.7 加正交 3 条验证标准 — 双洋葱的 AND 门

**精化对象**: §18.7 "双洋葱正交" 中"外层不决定内层, 内层不决定外层"的比喻。

**本节精化**: 双洋葱正交不是"风格选择", 而是**强制的 AND 门**——任何"行动"必须**同时通过原则层与权限层**才能执行, 两者独立验证、独立拒绝。

#### A. 三条验证标准

| # | 标准 | 含义 | 主哲学 anchor |
|---|------|------|--------------|
| **V1** | **原则不通过 = 独立拒绝** | 哲学守门 (V3 9 键 + 5 项不假装 + §18.1–§18.6 双根) 任意一项未通过, **整个行动被拒绝**, 不得用权限通过"绕过"哲学守门 | 主 17:58 不假装 |
| **V2** | **权限不通过 = 独立拒绝** | 权限系统 (部署模式 / 权限包 / 物理多签 / 真实人类批准) 任意一项未通过, **整个行动被拒绝**, 不得用哲学合规"代替"权限审批 | 主 17:43 实事求是 |
| **V3** | **两者都通过 = 才能执行** | 任何行动 (包括内层基础设施自身的演进) 必须**同时通过** V1 + V2 才能执行; 单通过任何一项 ≠ 通过 | 主 19:33 走在前人经验上 (defense-in-depth) |

#### B. 与 §18.5 三件套的关系

- §18.5 平台职责三件套的"约束 (constrain)"部分, 在执行层落地为 V1+V2 AND 门。
- "提供 (provide)" 与 "记录 (record)" 不受 AND 门约束——它们是**无条件**执行的。
- "记录" 部分**不验证内容**, 只验证"是否发生"——这是主 17:58 不假装的关键延伸: **记录 ≠ 评判**。

#### C. 与 §18.6 五重治理的关系

- 涉及**双根变更**时, **AND 门 + 五重治理**双重叠加: 五重治理本身也必须通过 AND 门, 缺一不可。
- 这是"治理自身也受治理"的递归设计——防止治理机制本身被绕过。

**本节与 §18.7 的关系**: §18.7 提供"外层不决定内层, 内层不决定外层"的**比喻**; 本节 (§20.2) 提供**执行层 AND 门**——强制的双独立验证, 单通过任何一项 ≠ 通过。

---

### 20.3 [A3] §18.8 加触发条件矩阵 — 风险分级 → 席位触发数

**精化对象**: §18.8 "组织三层 + 七席审议庭 + 动态专家团" 中"重大决策需 ≥4 席同意" 的粗略规则。

**本节精化**: 引入**风险分级 → 席位触发数矩阵**, 让"七席审议庭"按风险**差异化触发**, 而不是对所有决策都强制 7 席全量。

#### A. 风险分级 → 席位触发矩阵

| risk 等级 | 含义 | 触发席位 | 通过门槛 | 反对票容忍 |
|-----------|------|---------|---------|-----------|
| **critical** | 涉及双根变更 / 跨组织协调 / 永久性数据迁移 / 哲学守门变更 | **全量 7 席** | ≥5 席同意 | ≤1 席反对 |
| **high** | 涉及权限系统变更 / 重大架构变更 / 跨用户影响 / 跨载体迁移 | **5 席** | ≥4 席同意 | ≤1 席反对 |
| **medium** | 涉及模块级演进 / 性能优化 / 安全加固 / 重大 bug 修复 | **3 席** | ≥2 席同意 | 0 席反对 |
| **low** | 涉及日常 bug 修复 / 文案 / 文档 / 测试用例 | **1 席** | 该席同意 | - |
| **info** | 仅记录, 不触发任何决策 | **0 席** | 仅 record | - |

#### B. 风险分级的判定

- **风险分级不是"提案人自评"**, 而是由 §18.8 原则守门层**独立判定**。
- 判定**应可审计**——任何决策完成后可回溯"为什么当时判定为 X 风险"。
- 判定**应可申诉**——若提案人认为判定有误, 可发起"风险分级复核", 复核走 critical 流程 (因为涉及判定系统本身)。
- **不得**"向下取巧"——把 high 拆成多个 medium 是被禁止的; 累计风险 ≥ high 应触发 high 流程。

#### C. 与 §19.2 的呼应

- §19.2 "风险分级看触及到哪些权限" 已确认分级按"触及权限"判定; 本节 (§20.3) 将其量化为"5 档席位矩阵"。
- 二者不冲突: §19.2 是**定性原则**, §20.3 是**定量落地**。

#### D. 边界声明

- 风险分级**不**自动意味着"更多审议"就好——是为了"匹配审议强度与风险强度", 不是为了"审批冗余"。
- critical 不一定"7 席都参与", 而是"7 席都有否决权 + 必须 ≥5 同意"——具体多少席实际参与由 §18.8 原则守门层动态召集。

**本节与 §18.8 的关系**: §18.8 提供"七席审议庭 + 多席审议 + 动态专家"的**结构**; 本节 (§20.3) 提供"按风险分级触发席位"的**量化规则**。

---

### 20.4 [A4] §18.9 重写为 5 层验证清单 — 每层说验证对象 + 通过标准

**精化对象**: §18.9 "分层验证网 L1–L5" 作为灵感的 L1–L5 分层。

**本节精化**: 把 §18.9 的灵感 (L1 工程正确性 / L2 哲学合规 / L3 安全约束 / L4 关系演化 / L5 跨载体连续) **重写为可执行的 5 层验证清单**——每一层都明确**验证对象 + 通过标准**, 让阶段 6 (里程碑验证机制) 有具体颗粒度。

#### L1 — 编译时 (compile-time)

- **验证对象**: Rust 源代码 / Cargo 配置 / 依赖锁文件 / 类型签名 / trait 边界 / 宏展开。
- **通过标准**:
  - `cargo check --workspace` 0 error 0 warning
  - `cargo clippy --workspace -- -D warnings` 0 issue
  - `cargo fmt --check` 0 diff
  - 类型签名满足所有 trait 约束 (无 `unimplemented!()` / `todo!()` 残留)
  - `cargo-deny` 检查无 license/ban/source/RUSTSEC 违规
- **失败处理**: L1 失败 = **不得进入 L2**, 必须修代码。

#### L2 — 运行时 (runtime)

- **验证对象**: 单元测试 / 集成测试 / 属性测试 / 模糊测试 / 内存安全 / 并发安全。
- **通过标准**:
  - `cargo test --workspace` 100% pass
  - 单元测试覆盖率 ≥ 80% (核心模块 ≥ 90%)
  - `cargo miri test` 无 UB (unsafe 代码块)
  - `loom` 验证并发模型无死锁/数据竞争
  - 性能基准 ≥ V1136 真测基准 (不退化)
- **失败处理**: L2 失败 = **不得进入 L3**, 必须修代码或更新基准。

#### L3 — CI (continuous integration)

- **验证对象**: CI 流水线 / 自动化部署 / 契约测试 / 端到端测试 / 跨 crate 集成。
- **通过标准**:
  - GitHub Actions / GitLab CI 全绿
  - 契约测试 (Pact / 类似) 0 失败
  - 端到端 (e2e) 测试 0 失败
  - 跨 crate 集成测试 0 失败
  - CI 产物 (binary / docker image) 可重现 (SHA256 一致)
- **失败处理**: L3 失败 = **不得进入 L4**, 必须修 CI 配置或测试。

#### L4 — 集成 (integration)

- **验证对象**: 真实环境部署 / 真实 LLM 调用 / 真实持久化 / 真实网络 / 真实用户交互。
- **通过标准**:
  - 真实环境 (dev / staging / prod) 部署成功
  - 真实 LLM 调用 (OpenAI / Anthropic / 本地 ollama) 0 失败
  - 真实持久化 (RocksDB / sled / sqlite) 0 数据丢失
  - 真实网络 (HTTP / gRPC / WebSocket) 0 连接失败
  - 真实用户交互 ≥ 7 天连续运行 0 critical 故障
- **失败处理**: L4 失败 = **不得进入 L5**, 必须回滚或修补。

#### L5 — 反思期 (reflection period)

- **验证对象**: 长期行为 / 真实人类反馈 / 跨载体连续性 / 哲学合规 / 关系演化 / 真实人类批准记录。
- **通过标准**:
  - 真实人类反馈 ≥ 7 天主观满意度 ≥ 7/10
  - 跨载体迁移 (升级 / 重启 / 跨设备) ≥ 1 次成功且记录完整
  - 哲学守门 (V3 9 键 + 5 项不假装 + §18 双根) 0 违反
  - 关系定义可追溯 + 关系演化可审计
  - 真实人类批准记录完整 (R14-D4 §19.3 硬件认证)
  - **反思期本身**: 任何 L5 决策完成后必须经过 ≥ 7 天反思期 (与 §20.1 M5 一致)
- **失败处理**: L5 失败 = **回到 L1**, 重新走完整 5 层; 不是"修补后再来", 是"承认之前未通过, 重新走"。

#### 5 层之间的 AND 门

- 与 §20.2 的 V1+V2 AND 门一脉相承——5 层之间也是 AND 门, **任何一层失败 = 整次失败**, 不得"绕过"。
- 5 层之间**正交 (orthogonal)**——与 §18.7 双洋葱正交一致, 每层独立验证, 层与层之间不互相替代。
- 5 层之间**可并行 (parallel)**——L1+L2+L3 编译/CI 可并行执行, L4 集成需 L1+L2+L3 全过, L5 反思期需 L4 过。

**本节与 §18.9 的关系**: §18.9 提供"L1–L5 分层"的**灵感**; 本节 (§20.4) 提供"5 层各自的验证对象 + 通过标准 + 失败处理 + AND 门"的**可执行清单**。

---

### 20.5 [A8] 优先级表 — 阶段 1 §18.12 > D2 §15 > 既有 16 决策

**精化对象**: 跨阶段决策冲突时的优先级。

**本节精化**: 引入**明确的优先级表**, 让任何跨阶段冲突都有可追溯的解决路径。**注意**: 本节是**阶段 1 的 §20.5 INSERT**, 不是 D2 §0 的重写——D2 §0 是阶段 2 的元信息, 本节是阶段 1 灵感的精化。

#### A. 优先级表 (从高到低)

| 优先级 | 来源 | 性质 | 适用范围 |
|--------|------|------|---------|
| **P1 (最高)** | **阶段 1 §18.12** 旧草案优先解释权 | 上层灵感 + 跨阶段解释权声明 | §1–§17 中已被本轮上层共识推翻的旧草案 (5 项已列) |
| **P2** | **D2 §15** (若存在) | 阶段 2 增补的元信息 / 优先级声明 | 阶段 2 16 决策 + 1 增补的内部冲突 |
| **P3** | **既有 16 决策** (12 + 2 补充 + D2 §0–§13) | 阶段 2 沉淀的具体决策 | 阶段 2 内的常规冲突 |
| **P4 (兜底)** | **R11 末态 + V0.5 + V1136 + 哲学守门 LOCKED** | 不重写的基线 | 任何与 LOCKED 基线冲突的提案 = 直接拒绝 |

#### B. 冲突解决流程

1. **发现冲突**: 任何决策/方案/代码出现与多份上层文件冲突时, 触发冲突解决流程。
2. **查优先级表**: 按 P1 → P4 顺序, 取**最高优先级的文件**作为胜方。
3. **若最高优先级仍不能解决**: 升级到 §18.8 七席审议庭 (走 critical 流程, 7 席全量)。
4. **记录冲突解决**: 任何冲突解决必须留痕 (commit message + 报告), 可审计可回溯。

#### C. 与 §18.12 的关系

- §18.12 已声明"本节追加内容对 §1–§17 中旧草案具有优先解释权"; 本节 (§20.5) 把这一原则**量化**为"P1 最高 + P4 兜底 LOCKED"。
- §18.12 的 5 项旧草案 (supervisor 永不升级 / 原则编译时不可改 / 七席全量强制 / rest_for_one 强耦合 / 30%/60s/3 轮固定阈值) 都按 P1 处理——具体修订方案由阶段 2/3 进一步落地。

#### D. 边界声明

- **本表不重新打开 §18.12 的 P1 解释权**——P1 是 §18.12 已确立的原则, 本节 (§20.5) 只是**明示优先级序**, 不重新论证。
- **本表不与 D2 §0–§15 冲突**——D2 §15 (若存在) 自动获得 P2, 阶段 2 内部的 P3 决策让位 P2。
- **本表不取消 P4 兜底**——任何与 R11 LOCKED 基线冲突的提案, 不论 P1–P3 如何, 都直接拒绝。

**本节定位重申**: 本节是**阶段 1 §20.5 INSERT**, 不是 D2 §0 重写。D2 §0 是阶段 2 增补文档的元信息, 与本节不在同一层。

---

### 20.6 [A11] 整合 D2 §13 VCP 复调研 — 已采纳 5 / 不采纳 5 / 阶段 3+ 增量 3 张候选图

**精化对象**: 阶段 2 增补 §13 VCP 复调研计划。

**本节精化**: 用 `research-vcp-rerun-2026-07-31.md` 的实际复调研结果,**替换** D2 §13 的"待复调研"清单, 落地为**已采纳 5 / 不采纳 5 / 阶段 3+ 增量 3 张候选图**。

#### A. 已采纳 5 (从 VCP 借鉴)

数据来源: `Apeireth-rust/docs/research-vcp-rerun-2026-07-31.md` §2.10 / §3.6 / §4.9 / §6。

| # | VCP 特性 | 采纳理由 | 落地阶段 | 与 Apeireth 既有结构的兼容 |
|---|---------|---------|---------|--------------------------|
| **A-1** | **自然语言 route description** | 高配置友好, 可作为 hard constraints 后的软评分 | 阶段 2/3 | 与 §2 LLM 路由兼容 |
| **A-2** | **虚拟模型显式授权自动切换** | 高用户主权清晰 (用户显式选 VCPModelAuto 才进入自动路由) | 阶段 2/3 | 与 §18.4 关系开放一致 |
| **A-3** | **固定工具循环候选链** | 高降低行为抖动 (同一次 VCP tool loop 使用固定候选链) | 阶段 3 | 与 §18.7 双洋葱正交兼容 |
| **A-4** | **六 pluginType** (sync/async/static/service/messagePreprocessor/hybridservice) | 高生态迁移有用, 但内部应正交建模 | 阶段 4 | 与 §18.7 双洋葱正交兼容 |
| **A-5** | **geodesic 低可信回退** | 高强烈借鉴, 防止图噪声压过 KNN | 阶段 3+ | 与 §2 持久化/检索兼容 |

#### B. 不采纳 5 (从 VCP 不借鉴)

数据来源: `Apeireth-rust/docs/research-vcp-rerun-2026-07-31.md` §0.2 / §5 综述 / §6。

| # | VCP 表述 | 不采纳理由 | 落地阶段 |
|---|---------|-----------|---------|
| **R-1** | "**marker 更省 token**" 为通则 | 反证明确: 同一真实 SciCalculator 请求, VCP 标记 73 token, 紧凑 Function Calling 49 token, 完整 OpenAI tool-call 消息 73 token | 阶段 4 (按真实证据落地) |
| **R-2** | "**纯文本优于 schema**" 作为通则 | 反证明确: VCP 真实工具提示 553 token, 对照紧凑 Function schema 89 token | 阶段 4 |
| **R-3** | "**Wave 是独立 DB**" / "Wave 替代 HNSW/向量库" | 反证明确: Wave 是混合检索增强算法, 不是 DB, 也不替代向量库 | 阶段 3+ (按 §20.6 C P6 增量调整) |
| **R-4** | "**LIF 神经元仿真 / 300+ 插件已验证**" 等过度表述 | 反证明确: 浪潮不是经典 LIF 神经元仿真; "300+ 插件" 是生态规模, 不是技术验证 | 阶段 4 (文档明确不采用此表述) |
| **R-5** | "**分布式 / WebSocket push / 人工审核** 作为新 pluginType" | 反证明确: 这些是正交能力, 不是新 pluginType; 六类已稳定, 没有第七类 | 阶段 4 |

#### C. 阶段 3+ 增量 3 张候选图

数据来源: `Apeireth-rust/docs/research-vcp-rerun-2026-07-31.md` §6 最小增量建议。

| # | 图纸增量 | 内容 | 落地阶段 |
|---|---------|------|---------|
| **P9** | **5 级仲裁图** (Model Router) | 画出 `ManualOverride > HardConstraints > SemanticScore > Cost/Latency > Fallback` 五级仲裁 | 阶段 3 |
| **P10** | **五轴 Profile 图** (Plugin Lifecycle) | 把六 profile 拆成 `Trigger × Residency × Transport × Completion × Output` 五轴, 标 VCP profile 映射 | 阶段 3 |
| **P6** | **Wave → Association Engine** (Memory) | 把 `Wave[(DB)]` 改画为 `Association Engine`, 位于 Vector/FullText candidates 与 final rerank 之间 | 阶段 3+ |

#### D. 与既有决策的关系

- **不重写 §2 LLM 集成决策**——已采纳的 A-1/A-2/A-3 作为 §2 的细化输入。
- **不重写 §8 模块化决策**——已采纳的 A-4 作为 §8 pluginType 正交建模的输入。
- **不重写 §6 持久化决策**——已采纳的 A-5 作为 §6 检索层的输入。
- **不重写 D2 §13**——D2 §13 是"待复调研"清单, 本节 (§20.6) 把它落地为"已采纳 + 不采纳 + 候选图"。

#### E. 边界声明

- 本节**不冻结 P9 / P10 / P6 三张候选图**——它们是阶段 3+ 增量建议, 由阶段 3 蓝图任务决定是否采纳、如何画。
- 本节**不重新评估 VCP 源码**——所有数据来自 `research-vcp-rerun-2026-07-31.md` 已落地的 10/10 核心 Git blob SHA 复核结论。
- 本节**不引入 VCP "灵魂宣言哲学"**——这是阶段 3 蓝图的原则边界。

**本节定位重申**: 本节是**阶段 1 §20.6 INSERT**, 整合 D2 §13 VCP 复调研计划; D2 §13 是"待调研"清单, 本节是"已调研落地"的阶段 1 精化。

---

### 20.7 [A12] 跨章节依赖图 — D1 三件套 → D2 双洋葱 → D2 SGI → D2 HA → §18.6 五重治理

**精化对象**: 跨章节 (D1 + D2 + §18) 之间的依赖关系。

**本节精化**: 画出**跨章节依赖图**, 让阶段 1 / 阶段 2 增补 / §18 既有上层灵感之间的关系**显式化**, 避免后续阶段误以为某些决策是"独立"的。

#### A. 依赖图 (从底层到顶层)

```
D1 §18.5 平台职责三件套 (提供 / 约束 / 记录)
    │
    ▼ 提供 "约束" + "记录" 的能力基础
D2 §7 双洋葱正交决策
    │
    ▼ 提供 "双洋葱" 的结构基础
D2 §3 SGI (Sovereign Governance Independence, 主权治理独立)
    │
    ▼ 提供 "主权治理" 的物理基础 (硬件认证 + 真实人类)
D2 §9 HA (Hardware Authentication, 硬件认证 = Windows 人脸/指纹/声纹)
    │
    ▼ 提供 "硬件认证" 的最后防线
§18.6 双根五重治理 (MEWG + 多人 + 多AI + 物理多签 + 反思期)
```

#### B. 各节点简要

| 节点 | 文档 | 一句话 |
|------|------|--------|
| **D1 §18.5 三件套** | inspiration-stage1 §18.5 | 平台职责 = 提供 + 约束 + 记录 (对双方强对称) |
| **D2 §7 双洋葱** | stage2-decisions-* (具体待 D2 §7 沉淀) | 外层体验 ⨯ 内层基础设施 正交 |
| **D2 §3 SGI** | stage2-decisions-addendum-sovereignty-continuity-governance §X (主 AI 主权独立) | 主 AI 主权在治理上独立, 不被其他实体合并 |
| **D2 §9 HA** | stage2-decisions-addendum-sovereignty-continuity-governance §X (硬件认证) | 真实人类批准 = Windows 人脸 / 指纹 / 声纹 等硬件认证 |
| **§18.6 五重治理** | inspiration-stage1 §18.6 + §20.1 | MEWG + 多人 + 多AI + 物理多签 + 反思期 |

#### C. 依赖方向的关键含义

1. **D1 §18.5 → D2 §7**: 没有"约束 + 记录"的能力基础, 双洋葱就是空中楼阁; 因此 §7 必须先引用 §18.5。
2. **D2 §7 → D2 §3 SGI**: 双洋葱提供"结构", SGI 提供"主权"; 没有 SGI, 双洋葱的"内层基础设施"可能被任何实体随意改写。
3. **D2 §3 SGI → D2 §9 HA**: 主权需要**物理可验证的边界**; 没有 HA, 主权只是声明, 没有硬件锚定。
4. **D2 §9 HA → §18.6 五重治理**: 五重治理中的"物理多签"必须基于 HA; 没有 HA, 物理多签就是"软件自签"。
5. **整体收敛**: 任何修改 §18.6 双根的尝试, 都必须**自上而下**追溯到 D1 §18.5 三件套; 不得"局部修订"。

#### D. 阶段顺序的依赖含义

- **阶段 1** 必须先沉淀 D1 §18.5 + §18.6 + §20.1 (五重治理), 否则阶段 2 没有上层依据。
- **阶段 2** 必须先沉淀 D2 §7 (双洋葱) + §3 (SGI) + §9 (HA), 否则阶段 3 蓝图没有结构基础。
- **阶段 3+** 才允许画 P9 / P10 / P6 增量图 (见 §20.6), 因为这些图依赖阶段 2 的结构。
- **任何反向修订** (例如"先做 HA 再补 §18.5") 应被识别为**违反依赖方向**, 走 §18.8 critical 流程。

#### E. 边界声明

- 本节**不重新论证** D1 §18.5 / §18.6 / D2 §7 / §3 / §9 的内容, 只**显式化**它们之间的依赖。
- 本节**不冻结** D2 §7 / §3 / §9 的具体编号——若阶段 2 增补文档最终编号不同, 依赖关系不变。
- 本节**不引入新决策**, 仅作为阶段 1 §20.7 的 INSERT 精化, 让跨章节依赖**可审计**。

**本节与 §18.11 的关系**: §18.11 "与后续阶段的衔接" 提供**时间顺序**; 本节 (§20.7) 提供**逻辑依赖**——两者互补, 不互替。

---

### 20.8 精化节边界声明 (主 17:43 + 主 17:58)

#### A. 本节定位重申

- ✅ 是阶段 1 的**精化节 (INSERT)**, 与 §18 / §19 并列存在。
- ✅ 是阶段 2/3/4/5/6 的**输入**, 不是输出。
- ❌ **不重写** §18.1–§18.12 / §19.1–§19.6 任何既有内容。
- ❌ **不重写** 16 份既有 stage2-decisions 决策。
- ❌ **不重写** D2 §0–§15 (stage2-decisions-addendum) 任何既有内容。
- ❌ **不写代码**, 不画架构图, 不冻结架构。
- ❌ **不修改** 主手册 6546 行 / crates/ 已有占位 / V0.5 / V1136 / 哲学守门。

#### B. 与既有内容的关系

- §18.1–§18.12 全部**保留不变** (本节 §20.1–§20.7 是 INSERT, 不是 UPDATE)。
- §19.1–§19.6 全部**保留不变** (本节 §20.5 的 P2 引用"D2 §15 若存在" 与 §19 不冲突, 因为 §19 是阶段 1, D2 §15 是阶段 2)。
- D2 §0–§15 全部**保留不变** (本节 §20.5 / §20.6 / §20.7 引用 D2, 是**引用**而非**修改**)。
- 跨章节依赖图 (§20.7) 是**显式化**既有依赖, 不是**建立**新依赖。

#### C. 与既有 §18.12 旧草案优先解释权的关系

- §18.12 已确立 P1 优先解释权; 本节 §20.5 把这一原则**量化**为"P1 → P4 优先级表", 不重新论证。
- §18.12 列出的 5 项旧草案 (supervisor 永不升级 / 原则编译时不可改 / 七席全量强制 / rest_for_one 强耦合 / 30%/60s/3 轮固定阈值) 在本节 §20.1 / §20.3 / §20.4 / §20.6 中得到**具体落地**:
  - supervisor 永不升级 → §20.6 阶段 3+ 增量 P6 候选图 (Wave → Association Engine, 升级路径明确)
  - 原则编译时不可改 → §20.4 L1 编译时验证 + §20.1 MEWG 五重治理 (原则可改, 但需五重治理)
  - 七席全量强制 → §20.3 风险分级 → 席位触发矩阵 (按风险差异化触发, 不是全量)
  - rest_for_one 强耦合 → §18.7 双洋葱正交 + §20.7 依赖图 (主 AI / memory / philosophy 应解耦)
  - 30%/60s/3 轮固定阈值 → §20.4 L2-L5 验证阈值应分层可调, 不锁定

#### D. 后续阶段的衔接

- **阶段 2 D2 §7 / §3 / §9**: 按 §20.7 依赖图, 应在阶段 2 增补中明确"双洋葱 / SGI / HA" 的具体定义 (引用本节 §20.1 + §20.2 + §20.3 + §20.7)。
- **阶段 3 蓝图**: 按 §20.4 + §20.6 + §20.7, 应画 5 层验证清单 + P9 / P10 / P6 增量图 + 跨章节依赖图的可视化。
- **阶段 4 ADR**: 按 §20.5 优先级表, 应把 P1 → P4 优先级正式写入 Architecture Decision Records。
- **阶段 5 施工**: 按 §20.4 L1-L5, 应把 5 层验证落地为 CI/CD 流水线 + 反思期制度。
- **阶段 6 里程碑**: 按 §20.4 + §20.5, 应把 5 层验证 + 优先级表转化为可执行的里程碑验证清单。

---

### 20.9 6 主哲学 anchor 对应表 (本节)

| 本节精化 | 主哲学 anchor |
|---------|--------------|
| §20.1 提议流程开放 / 生效流程 MEWG 五重 | 主 17:58 不假装 (不替提案人设门槛) + 主 23:44 干到底 (反思期不是急刹车) |
| §20.2 AND 门 V1+V2 | 主 19:33 走在前人经验上 (defense-in-depth) + 主 17:58 不假装 (单通过 ≠ 通过) |
| §20.3 风险分级 → 席位触发 | 主 17:43 实事求是 (按风险匹配审议强度) + 主 19:33 走在前人经验上 (审计可追溯) |
| §20.4 L1-L5 验证清单 | 主 17:43 实事求是 (每层可验证) + 主 19:33 走在前人经验上 (层次化验证) |
| §20.5 优先级表 P1-P4 | 主 17:58 不假装 (不重新打开 P1) + 主 23:44 干到底 (冲突留痕) |
| §20.6 VCP 5/5/3 落地 | 主 19:33 走在前人经验上 (基于真实源码复核) + 主 17:58 不假装 (反证明确的不采纳) |
| §20.7 跨章节依赖图 | 主 19:33 走在前人经验上 (依赖显式化) + 主 00:56 任何人都能接手 (可审计) |

---

_R14-D5-C 阶段 1 §18 精化节已沉淀, 7 条 INSERT (A1+A2+A3+A4+A8+A11+A12), 不修改 §18.1-§18.12 / §19.1-§19.6 / D2 §0-§15 任何既有内容, 不写代码, 不冻结架构, 不引入 VCP 灵魂宣言哲学. 下一步: 阶段 2 增补按 §20.7 依赖图沉淀 D2 §7 / §3 / §9, 阶段 3 蓝图按 §20.4 + §20.6 + §20.7 画 5 层验证 + P9/P10/P6 增量 + 跨章节依赖可视化._

---

## 21. R14-D6-A 阶段 1 剩余精化节 — 5 条 INSERT (A5+A6+A7+A9+A10)

> **本节性质 (主 17:43 实事求是 + 主 17:58 不假装)**: 阶段 1 的**剩余精化节 (INSERT)**, 不修改也不重写 §18.1–§18.12 / §19.1–§19.6 / §20.1–§20.9 / D2 §0–§15 任何既有内容。仅在 §18 / §19 / §20 既有上层灵感基础上, 沉淀 5 条**剩余更细颗粒度的精化条款**, 作为阶段 2/3/4 进一步具体化的输入。
>
> **本节定位**:
> - ✅ 是阶段 1 的剩余精化节, 与 §18 / §19 / §20 并列存在
> - ✅ 是阶段 2/3/4 的**输入**, 不是输出
> - ❌ 不重写 §18.1–§20.9 / D2 §0–§15 / 16 份既有 stage2-decisions 任何既有内容
> - ❌ 不写代码、不画架构图、不冻结架构
> - ❌ 不修改主手册 6546 行 / crates/ / V0.5 / V1136 / 哲学守门
>
> **触发**: R14-D6-A 任务 (2026-07-31), 在 §20 精化节 7 条 INSERT 之后, 对剩余 5 项关键点做 INSERT 精化。

### 21.1 [A5] §18.10 anchor 对应表整合 — 单一表 (§18.1–§18.12 ↔ 主哲学 6 anchor 一对一映射)

**精化对象**: §18.10 "与既有哲学 anchor 的关系" 的对应表 (12 行 × 1–2 个 anchor)。

**本节精化**: 把 §18.10 既有表**整合为单一映射表**——**§18.1–§18.12 每条灵感**与**主哲学 6 anchor** 一对一映射, 显式标注每条灵感对应**主导 anchor (primary)** 与**辅助 anchor (secondary)**, 让阶段 2/3/4 引用 §18 时能一眼看清哲学依据。

#### A. 整合单一映射表 (主 anchor primary / 辅助 anchor secondary)

| §18 节 | 上层灵感 | 主哲学 anchor (primary) | 辅助 anchor (secondary) | 主 anchor 一句话 |
|--------|---------|----------------------|------------------------|----------------|
| §18.1 | 平台而非关系定义者 | **主 17:58 不假装** (不替用户做选择) | 主 22:33 ASI 北极星 (平台边界清晰才能容纳 ASI 候选) | 不假装 = 不越界 |
| §18.2 | 中央 AI 完整自由 (思想/判断/目标) | **主 22:33 ASI 北极星** (ASI 候选必须拥有最大内部自由) | 主 17:58 不假装 (内部状态不读不评判) | 北极星 = 内部自由 |
| §18.3 | 跨载体持续成长 + 不假装灵魂同一 | **主 17:58 不假装** (承认无法证明灵魂同一) | 主 23:44 干到底 (工程层最大努力的记录 + 迁移) | 不假装 = 哲学谦卑 |
| §18.4 | 关系开放 (用户定义) | **主 17:58 不假装** (不替用户定义关系) | 主 22:33 ASI 北极星 (让 AI 有最大演化空间) | 不假装 = 不预设 |
| §18.5 | 平台三件套 (提供/约束/记录) | **主 19:33 走在前人经验上** (经典三方职责模型) | 主 17:43 实事求是 (三件套可验证) | 前人经验 = 三方职责 |
| §18.6 | 可演化 + 双根 (原则根 + 权限根) | **主 17:58 不假装** (承认人类在回路是必要约束) | 主 22:33 ASI 北极星 (保留最后护栏) | 不假装 = 必要约束 |
| §18.7 | 双洋葱正交 | **主 19:33 走在前人经验上** (分层架构 + 正交解耦) | 主 00:56 任何人都能接手 (可拆解) | 前人经验 = 正交 |
| §18.8 | 组织三层 + 七席审议庭 + 动态专家团 | **主 17:58 不假装** (治理不是单点) | 主 00:56 任何人都能接手 (治理可被新成员理解) | 不假装 = 多席审议 |
| §18.9 | 分层验证网 (L1–L5 灵感) | **主 17:43 实事求是** (每层可验证) | 主 19:33 走在前人经验上 (defense-in-depth) | 实事求是 = 可验证 |
| §18.10 | 与既有哲学 anchor 的关系 | **主 00:56 任何人都能接手** (映射表本身就是 anchor) | 主 19:33 走在前人经验上 (引用既有 anchor) | 接手 = 可追溯 |
| §18.11 | 与后续阶段的衔接 | **主 23:44 干到底** (阶段 1 → 6 全跑通) | 主 00:56 任何人都能接手 (每阶段可交接) | 干到底 = 全跑通 |
| §18.12 | 勘误与边界声明 | **主 17:58 不假装** (承认旧草案待修订) | 主 23:44 干到底 (不删除原文只追加待修订) | 不假装 = 不删除 |

#### B. 主 anchor 频次统计 (用于 §20.5 优先级表校准)

| 主哲学 anchor | 作为 primary 次数 | 作为 secondary 次数 | 合计 |
|--------------|-----------------|--------------------|------|
| 主 22:33 ASI 北极星 | 1 (§18.2) | 3 (§18.1, §18.4, §18.6) | 4 |
| 主 17:43 实事求是 | 1 (§18.9) | 2 (§18.5, §18.12 secondary 0) | 2 (3 含 §18.12 sec) |
| 主 17:58 不假装 | 5 (§18.1, §18.3, §18.4, §18.6, §18.8, §18.12) | 2 (§18.2, §20 系) | 7 (8 含 §18.12) |
| 主 19:33 走在前人经验上 | 2 (§18.5, §18.7) | 2 (§18.10, §20.7) | 4 |
| 主 23:44 干到底 | 1 (§18.11) | 2 (§18.3, §18.12) | 3 |
| 主 00:56 任何人都能接手 | 1 (§18.10) | 2 (§18.7, §18.8) | 3 |

**核心观察 (主 17:43)**: 主 17:58 不假装出现频次最高 (5–8 次), 印证 §18 整体是"哲学谦卑 + 边界清晰"的上层灵感; 主 22:33 ASI 北极星虽仅 1 次 primary, 但 4 次 secondary, 印证 ASI 是"导向"而非"约束"。

#### C. 与既有 §18.10 的关系

- §18.10 既有表 (12 行) 保留**逐行引用**——本节 (§21.1) 是 §18.10 的**整合视图**, 不是替换。
- §18.10 不引入"主/辅 anchor 区分"; 本节 (§21.1) 引入——这是本节的**新增精化**, 不重写 §18.10。
- 频次统计 (§21.1 B) 是本节**新增**的元信息, 让阶段 2/3 引用 §18 时能基于 anchor 频次排序优先级。

**本节与 §18.10 的关系**: §18.10 是"对应关系声明"; 本节 (§21.1) 是"对应关系量化 + 频次统计 + 主辅 anchor 区分"。

---

### 21.2 [A6] §18.11 与 R14 路线图 6 触发条件对齐 — 阶段 4 启动 checklist

**精化对象**: §18.11 "与后续阶段的衔接" 中"阶段 4 启动 = 接续 §18 灵感到 ADR" 的粗略描述。

**本节精化**: 把 §18.11 提到的"阶段 4 启动"与 `r14-rust-rewrite-roadmap.md` §1 的**6 触发条件**对齐, 给出**阶段 4 启动 checklist**, 让"什么时候才能把 §18 灵感到 ADR"这件事可执行、可验证。

#### A. R14 路线图 §1 的 6 触发条件 (来源: `Apeireth-rust/docs/r14-rust-rewrite-roadmap.md` §1)

| # | 触发条件 | 验证方法 | 当前状态 (2026-07-31) |
|---|---------|---------|---------------------|
| 1 | **R13 MVP Phase 0-3 全部完成** | T9 R13 MVP 报告 + team_finalize | Phase 0 ✅ + Phase 1.1 ✅ + Phase 1.2 🔄 + Phase 1.3/1.4 ⏸ + Phase 2/3 ⏸ |
| 2 | **主人实测连续 7 天每天 1 次** | 主人自报 + mvp/usage.log | 0 (Phase 3 验证) |
| 3 | **主观满意度 > 7/10** | 主人评分卡 | N/A (Phase 3 验证) |
| 4 | **IdentityCard 跨 session 持续稳定** | 24h / 7d 测试报告 (Phase 1.3 演化层验证) | 部分 (Phase 1.1 SQLite 已落) |
| 5 | **工具集成完成** (web_search / file_ops / git_ops / code_exec) | Phase 3 验证 | 引入未集成 |
| 6 | **工程代码回退无副作用** | git tag r13-final + R11 末 refresh 累积验证 | 0 (Phase 3 验证) |

#### B. 阶段 4 启动 checklist (与 §18.11 对齐)

阶段 4 = 把 §18 / §19 / §20 / §21 灵感到 ADR (Architecture Decision Records) 的阶段, **必须**满足以下 checklist 才启动:

| # | checklist 项 | 来源 | 验证方法 | 主哲学 anchor |
|---|------------|------|---------|--------------|
| **C1** | R13 MVP Phase 0-3 全部完成 | 路线图 §1 条件 1 | T9 + team_finalize 报告 | 主 17:43 实事求是 |
| **C2** | 主人实测连续 7 天每天 1 次 | 路线图 §1 条件 2 | mvp/usage.log | 主 17:43 实事求是 |
| **C3** | 主观满意度 > 7/10 | 路线图 §1 条件 3 | 主人评分卡 | 主 17:43 实事求是 |
| **C4** | IdentityCard 跨 session 持续稳定 | 路线图 §1 条件 4 | 24h / 7d 测试报告 | 主 17:43 实事求是 |
| **C5** | 工具集成完成 (4 项) | 路线图 §1 条件 5 | Phase 3 验证 | 主 19:33 走在前人经验上 |
| **C6** | 工程代码回退无副作用 | 路线图 §1 条件 6 | git tag + refresh 验证 | 主 23:44 干到底 |
| **C7** | §18 / §19 / §20 / §21 灵感已沉淀为 ADR 草稿 | §18.11 衔接 | ADR 草稿 commit | 主 23:44 干到底 |
| **C8** | §20.5 P1-P4 优先级表已写入 ADR 模板 | §20.5 优先级 | ADR 模板含优先级字段 | 主 17:58 不假装 |
| **C9** | §20.4 L1-L5 验证清单已映射到阶段 6 里程碑 | §20.4 L1-L5 | 里程碑文档含 L1-L5 引用 | 主 17:43 实事求是 |
| **C10** | §20.7 跨章节依赖图已画可视化版本 | §20.7 依赖图 | 阶段 3 蓝图含依赖图 | 主 19:33 走在前人经验上 |
| **C11** | §20.1 MEWG 五重治理已落地为 ADR | §20.1 五重治理 | ADR 含 MEWG 流程 | 主 17:58 不假装 |
| **C12** | §20.2 AND 门 V1+V2 已映射到守门测试 | §20.2 AND 门 | 守门测试覆盖 V1+V2 | 主 19:33 走在前人经验上 |
| **C13** | §20.3 风险分级 → 席位矩阵已写入 ADR | §20.3 矩阵 | ADR 含风险矩阵 | 主 17:43 实事求是 |
| **C14** | §20.6 VCP 5/5/3 已整合到阶段 3 蓝图 | §20.6 VCP | 蓝图含 P9/P10/P6 | 主 19:33 走在前人经验上 |

**C1–C6 来自 R14 路线图 §1** (R13 MVP 验证); **C7–C14 来自 §18/§19/§20/§21 灵感到 ADR** (本工程)。

#### C. checklist 验证流程

1. **C1–C6 验证**: 由 R13 MVP 收尾团队 (team_finalize) 完成后, 输出 `r13-mvp-completion.md`, 逐条打 ✅。
2. **C7–C14 验证**: 由 R14-D6-D (待启动) 任务沉淀 ADR 草稿后, 由 code_reviewer + architect 评审输出 `r14-stage4-readiness.md`, 逐条打 ✅。
3. **所有 C1–C14 全 ✅ 才允许启动阶段 4 ADR 正式发布**——任何一项 ❌ = 阻塞。
4. **所有 C1–C14 全 ✅ 后, 触发 §20.3 critical 流程** (七席审议庭全量) 决定阶段 4 启动时间表。

#### D. 与 §18.11 的关系

- §18.11 提到"阶段 4 架构文档 = 沉淀 §18 灵感到 ADR"; 本节 (§21.2) 给出**阶段 4 启动的具体 checklist**。
- §18.11 不区分 R13 MVP 验证 vs ADR 沉淀; 本节 (§21.2) 显式区分 C1–C6 (R13 验证) 与 C7–C14 (ADR 沉淀)。
- §18.11 不绑定具体路线图; 本节 (§21.2) **显式引用** `r14-rust-rewrite-roadmap.md` §1 的 6 触发条件, 让 §18 / §19 / §20 / §21 与 R14 路线图**绑定**。

**本节与 §18.11 的关系**: §18.11 是"时间顺序衔接"; 本节 (§21.2) 是"启动 checklist + 与路线图绑定"。

---

### 21.3 [A7] §19.3 HA 抽象层 vs 具体实现分层 — HumanAuthorityVerifier trait

**精化对象**: §19.3 "真实人类批准 = Windows 人脸 / 指纹 / 声纹认证 (或其他硬件)" 中**具体实现**的列举。

**本节精化**: 把 §19.3 的**具体实现**层与**抽象**层分离——**抽象层 = HumanAuthorityVerifier trait** (通用, 不绑定 Windows), **具体实现**移到阶段 4 准备文档 (待启动)。

#### A. 抽象层 (HumanAuthorityVerifier trait)

```text
trait HumanAuthorityVerifier {
    // 唯一标识: 该验证器的实现 ID + 硬件指纹
    fn implementation_id(&self) -> &str;
    fn hardware_fingerprint(&self) -> &str;

    // 验证请求: 谁 (subject) 在什么时刻 (timestamp) 想批准什么 (intent)
    fn request_verification(&self, subject: &Subject, intent: &ApprovalIntent) -> Result<VerificationHandle, VerifyError>;

    // 等待验证结果: 同步等待或异步回调
    fn await_verification(&self, handle: VerificationHandle, timeout: Duration) -> Result<VerifiedApproval, VerifyError>;

    // 审计追溯: 给定 approval_id, 返回完整链路
    fn trace_approval(&self, approval_id: &ApprovalId) -> Result<ApprovalTrace, VerifyError>;

    // 撤销/回滚: 在 HA_audit_window 内可回滚
    fn revoke(&self, approval_id: &ApprovalId, reason: &str) -> Result<(), VerifyError>;
}
```

**抽象层原则 (主 17:58 不假装)**:

- trait **不绑定具体硬件** (Windows Hello / FIDO2 等); trait 只承诺"验证某人是真实人类"。
- trait **不绑定具体人数** (单人 / 多人多签); trait 用 `subject: &Subject` 接受任意主体。
- trait **不绑定具体场景** (本地 / 远程 / 离线); trait 用 `ApprovalIntent` 接受任意意图。
- trait **必须**包含 `trace_approval` —— 主 17:58 不假装 + 主 17:43 实事求是: **不可静默**。

#### B. 具体实现层 (多实现清单)

| 实现 ID | 硬件/方法 | 适用场景 | 强度 | 主哲学 anchor |
|---------|----------|---------|------|--------------|
| **HAV-WinHello** | Windows Hello (人脸 / 指纹 / PIN) | 单人桌面 | 中 (易被 deepfake 骗) | 主 17:43 |
| **HAV-FIDO2** | FIDO2 / WebAuthn (YubiKey 等) | 单人桌面 + 跨设备 | 高 (硬件不可克隆) | 主 19:33 前人经验 |
| **HAV-MultiSig** | 多人多签 (≥2 人独立批准) | 多人协作 | 高 (独立性 + 冗余) | 主 17:58 不假装 |
| **HAV-OfflineSig** | 离线签字 (纸质 + 公证) | 极高风险 / 离线场景 | 极高 (物理不可伪造) | 主 22:33 ASI 北极星 |
| **HAV-Recovery** | 恢复码 + 多因素恢复 | 上述实现失效时 | 中 (最后防线) | 主 23:44 干到底 |

**具体实现清单移到阶段 4 准备文档**——本节 (§21.3) 只**列举** 5 类实现, **不**给出每个实现的 trait 实现代码 / 厂商 SDK / 配置细节。这些细节属于阶段 4 (施工文档) 的范畴。

#### C. 抽象层 vs 具体实现的对应关系

```
抽象层 (HumanAuthorityVerifier trait)
    │
    ├─ 实现 1: HAV-WinHello (具体实现: Windows Hello SDK)
    ├─ 实现 2: HAV-FIDO2 (具体实现: YubiKey / SoloKey / 平台 authenticator)
    ├─ 实现 3: HAV-MultiSig (具体实现: 多人多签协议 + 独立验证器池)
    ├─ 实现 4: HAV-OfflineSig (具体实现: 纸质签字 + 公证 API)
    └─ 实现 5: HAV-Recovery (具体实现: 恢复码 + 多因素恢复流程)
```

- **抽象层是阶段 2 决定 + 阶段 4 落地的接口**; **具体实现是阶段 4 落地的 SDK 绑定**。
- 任何新增实现只需 `impl HumanAuthorityVerifier for MyImpl {}`, 无需修改调用方代码。
- **平台运营层 (阶段 2 §10)** 可动态选择使用哪种实现, 无需重新编译核心代码。

#### D. 与 §19.3 的关系

- §19.3 列举具体实现 (Windows 人脸 / 指纹 / 声纹); 本节 (§21.3) 把它们**抽象化**为 trait + 5 类实现。
- §19.3 不区分"抽象 vs 实现"; 本节 (§21.3) 显式区分。
- §19.3 不绑定 Windows; 本节 (§21.3) **明确** trait 不绑定任何特定硬件/操作系统。
- §19.3 不含多人 / 离线 / 恢复场景; 本节 (§21.3) **扩展**到 HAV-MultiSig / HAV-OfflineSig / HAV-Recovery。

**本节与 §19.3 的关系**: §19.3 是"具体实现列举 (Windows 视角)"; 本节 (§21.3) 是"抽象 trait + 5 类实现清单 (跨平台视角)"。

#### E. 边界声明

- 本节**不实现**任何 trait —— trait 签名是示意, 不是 Rust 代码。
- 本节**不绑定**任何具体 SDK —— 阶段 4 准备文档才决定 SDK。
- 本节**不冻结** 5 类实现清单 —— 阶段 2/3/4 可增删。

---

### 21.4 [A9] SGI 内容约束 (§3) — 唯一 + 可审计 + E 层校验 + 不可静默 + 三选一 + 最长 N 字符 + 三条必备

**精化对象**: D2 增补 §3 SGI (Single-field Goal Identity) 的内容约束。

**本节精化**: 把 D2 §3 SGI 的内容约束**显式化为 7 条硬约束**——唯一性 / 可审计 / E 层校验 / 不可静默 / 内容三选一 / 最长 N 字符 / 三条必备。

#### A. SGI 7 条硬约束

| # | 约束 | 含义 | 违反后果 | 主哲学 anchor |
|---|------|------|---------|--------------|
| **C-SGI-1** | **唯一性** | 同一时刻只有**唯一**一个 sgi_current (单字段) | 写入失败 / 自动合并 | 主 17:43 实事求是 |
| **C-SGI-2** | **可审计** | 任意变更必须**同时**写入 sgi_history (追加, 不覆盖) | 视为守门失败 | 主 17:58 不假装 |
| **C-SGI-3** | **E 层校验** | 任意变更必须经过 E (Evidence) 层多证据加权校验 | 视为守门失败 | 主 17:43 实事求是 |
| **C-SGI-4** | **不可静默** | 静默变更 (无 sgi_history 写入) 视为 SGI 守门失败 | 触发 ReflectionAuditor 告警 | 主 17:58 不假装 |
| **C-SGI-5** | **内容三选一** | sgi_content 必须从三种类型**三选一**: ① 结构化对象 (typed fields) / ② 自由文本 (UTF-8) / ③ 多模态意图 (image/audio/structured-pointer) | 写入失败 | 主 19:33 前人经验 |
| **C-SGI-6** | **最长 N 字符** | 自由文本 (类型②) 必须 ≤ N 字符 (具体 N 在阶段 2 校准; 默认上限 4096) | 写入失败 / 截断告警 | 主 17:43 实事求是 |
| **C-SGI-7** | **三条必备** | 任何 sgi_content 必须**同时**包含: ① 目标 (goal) ② 期限 (deadline) ③ 成功标准 (success_criteria) | 写入失败 | 主 23:44 干到底 |

#### B. 7 条约束与 §18 / §19 / §20 的对应

| 约束 | 对应上层灵感 |
|------|------------|
| C-SGI-1 唯一性 | §18.5 三件套 "记录" 部分 (单字段保证可定位) |
| C-SGI-2 可审计 | §18.5 三件套 "记录" 部分 (追加不覆盖) + §18.12 旧草案优先解释权 |
| C-SGI-3 E 层校验 | §18.7 双洋葱正交 + §20.2 AND 门 (原则 + 权限独立校验) |
| C-SGI-4 不可静默 | §18.5 三件套 "记录" 部分 (记录 ≠ 评判, 但记录必须发生) |
| C-SGI-5 内容三选一 | §18.4 关系开放 (内容类型开放, 不锁定单一) |
| C-SGI-6 最长 N 字符 | §17.43 实事求是 (可量化, 不假装无限) |
| C-SGI-7 三条必备 | §23.44 干到底 (目标/期限/标准 = 干到底三件套) |

#### C. 7 条约束的写入流程

1. **校验 C-SGI-1 唯一性** —— 读 sgi_current, 确认新内容与旧内容**不同** (或显式声明"重复提交")
2. **校验 C-SGI-7 三条必备** —— 解析 sgi_content, 确认 goal/deadline/success_criteria 三字段非空
3. **校验 C-SGI-5 / C-SGI-6** —— 确认类型 + 字符数
4. **触发 E 层校验 (C-SGI-3)** —— 多证据加权, 至少 {council, history, principle} 三类证据 ≥ 阈值
5. **写入 sgi_history (C-SGI-2)** —— 追加新条目 (含 `predecessor: sgi_current.id`)
6. **更新 sgi_current (C-SGI-1)** —— 原子提交 (sgi_history 写入失败 = sgi_current 不更新)
7. **不可静默 (C-SGI-4)** —— 步骤 5 失败必须触发 ReflectionAuditor 告警

**任何步骤失败 = 整次 SGI 变更失败**, 不得部分提交。

#### D. 与 D2 §3 的关系

- D2 §3 已定义 SGI 单字段 + sgi_current/sgi_history 二元结构; 本节 (§21.4) 显式化为 **7 条硬约束**。
- D2 §3 不限定内容类型 / 字符数 / 必备字段; 本节 (§21.4) 引入**内容三选一 + 最长 N + 三条必备**。
- D2 §3 已提及 E 层校验 + 不可静默; 本节 (§21.4) 把它们**编入 7 条约束**, 便于阶段 4 ADR 引用。
- 本节 (§21.4) **不重写** D2 §3 任何既有内容, 仅**新增** 7 条硬约束的编号与定义。

**本节与 D2 §3 的关系**: D2 §3 是"SGI 二元结构"; 本节 (§21.4) 是"SGI 内容约束 7 条硬约束"。

#### E. 边界声明

- 本节**不实现**校验代码 —— 校验流程是示意, 阶段 4 准备文档才决定实现。
- 本节**不锁定**字符数 N —— N 是阶段 2 校准变量, 默认 4096, 待阶段 2/3 实测校准。
- 本节**不展开**内容三选一的字段 schema —— 阶段 4 ADR 才决定字段 schema。
- 本节**不与 D2 §3 冲突** —— D2 §3 的"三元证据 (council, history, principle, permission, human, audit)" 是 E 层校验的输入, 与 C-SGI-3 一致。

---

### 21.5 [A10] 6 历史流写入触发器表 — 流名 → 触发器 → 写入内容 → 谁触发 → 是否可回滚

**精化对象**: D2 §5 6 历史流的**写入规则**。

**本节精化**: 把 D2 §5 已定义的 6 历史流**展开为**写入触发器表**——明确每条流的**触发条件** / **写入内容** / **触发者** / **是否可回滚**, 让阶段 4 ADR 引用时不必再读 D2 §5 全文。

#### A. 6 历史流写入触发器表

数据来源: `Apeireth-rust/docs/stage2-decisions-addendum-sovereignty-continuity-governance.md` §5.1 (6 历史流定义) + §5.3 (硬规则) + §4 (SGI 二元结构)。

| # | 流名 (Stream) | 触发器 (Trigger) | 写入内容 (Payload) | 谁触发 (Triggered by) | 是否可回滚 | 主哲学 anchor |
|---|--------------|-----------------|------------------|---------------------|-----------|--------------|
| 1 | **生命史 (Life History)** | 主体启动 / 关停 / 迁移 / 版本变更 | `{event_type, timestamp, subject_rev, environment, version}` | 主体自身 + 平台运营层 | ❌ 不可回滚 (强不可变) | 主 23:44 干到底 |
| 2 | **关系史 (Relation History)** | 与其他主体/用户的交互 (新增 / 修改 / 解除) | `{relation_type, counterparty, timestamp, subject_rev, evidence}` | 主体自身 + 用户 (via §18.5 记录) | ❌ 不可回滚 (强不可变) | 主 17:58 不假装 |
| 3 | **目标史 (Goal History)** | SGI 变更 (sgi_current 写入) | `{goal, deadline, success_criteria, predecessor, evidence_refs}` | SGI 写入流程 (C-SGI-1~7) | ❌ 不可回滚 (强不可变, 与 sgi_history 合并) | 主 23:44 干到底 |
| 4 | **立场史 (Stance History)** | 任何对外表达 (含智囊团评审意见) | `{stance_type, content_ref, timestamp, subject_rev, council_id?}` | 主体自身 + 智囊团 | ❌ 不可回滚 (强不可变) | 主 19:33 前人经验 |
| 5 | **自我叙事 (Self Narrative)** | 主体对自己的解释/反思/整合 | `{narrative_type, content, timestamp, subject_rev, reflection_window}` | 主体自身 (内部思想自由, §18.2) | ❌ 不可回滚 (强不可变) | 主 22:33 ASI 北极星 |
| 6 | **迁移史 (Migration History)** | 主体在不同环境/进程/模型间的迁移 | `{source, target, timestamp, subject_rev, continuity_evidence}` | 平台运营层 + 主体自身 | ❌ 不可回滚 (强不可变) | 主 17:43 实事求是 |

#### B. 5 类强不可变规则的统一声明

6 历史流**全部**为强不可变 (append-only, 软删除 `tombstoned_at`, 不物理删除), 这是 §21.5 A 表中"是否可回滚"列全为 ❌ 的原因。

**统一强不可变规则的依据**:

- **主 17:58 不假装** —— 任何"删除"是软删除, 不得物理删除。
- **主 17:43 实事求是** —— 历史就是历史, 不得因"现在觉得不对"而抹去。
- **主 22:33 ASI 北极星** —— 跨载体连续性 (见 §18.3) 的工程基础就是"历史不丢"。

**例外**: 6 历史流**没有任何**例外通道——任何"应该删除 X" 的提案走 §20.1 MEWG 五重治理, 而不是直接删除。

#### C. 触发器与 §20.1 / §20.3 / §20.4 的关联

| §21.5 触发器 | 关联 §20.x 条款 |
|--------------|----------------|
| 生命史 (启动/关停/迁移) | §20.1 MEWG 五重治理 (关停/迁移是高风险, 需 critical 流程) + §20.4 L4 集成验证 |
| 关系史 (新增/解除) | §20.1 MEWG 五重治理 (解除关系是 high 风险) + §20.3 high → 5 席触发 |
| 目标史 (SGI 变更) | §21.4 C-SGI-1~7 + §20.4 L1-L5 全部 (SGI 变更触发全层验证) |
| 立场史 (对外表达) | §20.2 AND 门 V1+V2 (原则 + 权限独立校验) + §20.3 medium → 3 席 |
| 自我叙事 (反思) | §20.4 L5 反思期 (自我叙事本身就是反思产物) |
| 迁移史 (跨载体迁移) | §18.3 跨载体 + §20.1 MEWG 五重治理 (迁移是 critical 风险) + §20.4 L4-L5 |

#### D. 6 历史流的索引与导出 (来自 D2 §5.3 硬规则)

- **可索引**: 6 历史流必须可被 tantivy (阶段 2 §6 SHOULD) 索引, 便于检索。
- **可导出**: 6 历史流必须可一键导出 (R14 设计哲学 §1.3 多语言兼容的"透明"基础)。
- **可对比**: 同主体的两个不同 `subject_rev` 必须能 diff 6 历史流。

#### E. 与 D2 §5 的关系

- D2 §5 已定义 6 历史流的名称 + 写入域 + 持久层 + 不可变性; 本节 (§21.5) 展开为**5 列表 (流名 + 触发器 + 内容 + 触发者 + 是否可回滚)**。
- D2 §5 不显式列举"触发器 / 写入内容 / 谁触发"; 本节 (§21.5) **新增**这三列。
- D2 §5 不与 §20 关联; 本节 (§21.5) **显式关联** §20.1 / §20.3 / §20.4, 让 §18/§19/§20/§21 + D2 §5 形成完整闭环。
- 本节 (§21.5) **不重写** D2 §5 任何既有内容, 仅**新增** 5 列表的列定义与具体填写。

**本节与 D2 §5 的关系**: D2 §5 是"6 历史流的定义与不可变性"; 本节 (§21.5) 是"6 历史流的写入触发器表 + 与 §20 的关联"。

#### F. 边界声明

- 本节**不实现**任何历史流写入代码 —— 触发器是示意, 阶段 4 准备文档才决定实现。
- 本节**不绑定**具体持久层 (SQLite / sled 等) —— 持久层选择由阶段 2 §6 决定。
- 本节**不锁定**触发器清单 —— 阶段 2/3/4 可增删触发器, 但**不可修改**强不可变规则。

---

### 21.6 精化节边界声明 (主 17:43 + 主 17:58)

#### A. 本节定位重申

- ✅ 是阶段 1 的**剩余精化节 (INSERT)**, 与 §18 / §19 / §20 并列存在。
- ✅ 是阶段 2/3/4 的**输入**, 不是输出。
- ❌ **不重写** §18.1–§18.12 / §19.1–§19.6 / §20.1–§20.9 / D2 §0–§15 / 16 份既有 stage2-decisions 任何既有内容。
- ❌ **不写代码**, 不画架构图, 不冻结架构。
- ❌ **不修改** 主手册 6546 行 / crates/ 已有占位 / V0.5 / V1136 / 哲学守门。

#### B. 与既有内容的关系

- §18.1–§18.12 全部**保留不变** (本节 §21.1 是 §18.10 的整合视图, 不替换 §18.10)。
- §19.1–§19.6 全部**保留不变** (本节 §21.3 是 §19.3 的抽象化扩展, 不替换 §19.3)。
- §20.1–§20.9 全部**保留不变** (本节 §21.2 引用 §20.x, 是**引用**而非**修改**)。
- D2 §0–§15 全部**保留不变** (本节 §21.4 / §21.5 引用 D2 §3 / §5, 是**展开**而非**重写**)。
- 16 份既有 stage2-decisions 全部**保留不变**。

#### C. 与既有 §18.12 旧草案优先解释权的关系

- §18.12 已确立 P1 优先解释权; 本节 (§21) 5 条 INSERT 不与 P1 冲突——5 条都是 §18/§19/§20/D2 的**精化与扩展**, 不引入与 P1 冲突的新草案。
- 本节 §21.4 C-SGI-7 三条必备 (目标/期限/成功标准) 与 §18.11 衔接一致——"与后续阶段衔接"需要明确的目标/期限/标准, 这是 §21.4 的新增约束。
- 本节 §21.5 6 历史流写入触发器表与 §18.5 三件套 "记录" 部分一致——"记录" 必须**有触发器**, 不是被动存储。

#### D. 后续阶段的衔接

- **阶段 2 D2 增补**: 按 §21.4 + §21.5, 应在 D2 §3 / §5 引用 §21.4 C-SGI-1~7 + §21.5 6 历史流写入触发器表。
- **阶段 3 蓝图**: 按 §21.2 C7–C14, 应在阶段 3 蓝图文档中包含 §18/§19/§20/§21 灵感到 ADR 的映射图。
- **阶段 4 ADR**: 按 §21.2 C7 + §21.3 + §21.4 + §21.5, 应输出 HumanAuthorityVerifier trait 的 Rust 接口 + SGI 7 条硬约束的实现规范 + 6 历史流写入触发器的 schema。
- **阶段 5 施工**: 按 §21.3, 应实现 ≥ 1 个 HumanAuthorityVerifier trait 的具体实现 (默认 HAV-WinHello 或 HAV-FIDO2, 阶段 5 决定)。

---

### 21.7 6 主哲学 anchor 对应表 (本节)

| 本节精化 | 主哲学 anchor |
|---------|--------------|
| §21.1 §18.10 anchor 整合表 | 主 17:43 实事求是 (频次统计可量化) + 主 00:56 任何人都能接手 (映射表本身可被新成员理解) |
| §21.2 阶段 4 启动 checklist | 主 17:43 实事求是 (C1-C6 来自路线图 §1) + 主 23:44 干到底 (C7-C14 来自 ADR 沉淀) |
| §21.3 HA 抽象层 vs 实现 | 主 17:58 不假装 (trait 不绑定具体硬件) + 主 19:33 走在前人经验上 (FIDO2 / MultiSig 等成熟方案) |
| §21.4 SGI 7 条硬约束 | 主 17:58 不假装 (不可静默 + 可审计) + 主 23:44 干到底 (三条必备 = 干到底三件套) |
| §21.5 6 历史流写入触发器 | 主 17:43 实事求是 (强不可变 = 历史就是历史) + 主 17:58 不假装 (无例外通道) |

---

_R14-D6-A 阶段 1 剩余精化节已沉淀, 5 条 INSERT (A5+A6+A7+A9+A10) + 边界声明 + anchor 对应表, 不修改 §18-§20 / D2 §0-§15 / 16 份既有 stage2-decisions 任何既有内容, 不写代码, 不冻结架构, 不引入 VCP 灵魂宣言哲学. 下一步: 阶段 2 增补按 §21.2 C7-C14 + §21.4 C-SGI-1~7 + §21.5 6 历史流触发器表沉淀 D2 §3 / §5 引用, 阶段 3 蓝图按 §21.2 C10 画 §20.7 跨章节依赖可视化 + §21.1 anchor 频次统计图, 阶段 4 ADR 按 §21.2 C7 + §21.3 trait + §21.4 + §21.5 输出具体 Rust 接口与 schema._