# Apeireth — 阿佩瑞斯

<div align="center">

<!-- LOGO 位（占位图）：正式 logo 深/浅两版定稿前先用现有预览图占位，换图只改这一处 -->
<img src="previews/intro-live-spark.png" alt="Apeireth logo 占位图——暗色中绽放的一点金色火花；正式深/浅两版 logo 定稿前先用它占位" width="220">

### Apeireth —— 会成长的数字伙伴

**它记得你随口说的话、在你忙时闭嘴、有自己不肯丢的底线。**

[![Tests](https://img.shields.io/badge/tests-9075%20passed%20%7C%200%20failed%20(2026--10--20)-success.svg?logo=checkmarx)](docs/03-reference/test-baseline.md)
[![Workspace](https://img.shields.io/badge/workspace-31%20members-blue.svg?logo=rust)](Cargo.toml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%2B%20Linux-lightgrey.svg)](INSTALL.md)
[![Status](https://img.shields.io/badge/status-rc%20%E5%86%85%E6%B5%8B-orange.svg)](ROADMAP.md)
[![Unsafe code](https://img.shields.io/badge/unsafe_code-FORBIDDEN-brightgreen.svg?logo=shield)](crates/foundation/core)
[![License](https://img.shields.io/badge/license-Apache--2.0--OR--MIT-blue.svg)](LICENSE)
[![Honest gaps](https://img.shields.io/badge/honest%20gaps-4-orange.svg?logo=shield)](#boundaries)

> 口径注记：**workspace = 31 members（30 个产品 crate + 1 个 bench harness，含图像核心 vendored 族 11）**（foundation 6 / engine 8 / capabilities 1 / adapters 4 = cli/gateway/sdk/tui）。
> Tests 徽章数字取单一真源 [test-baseline.md](docs/03-reference/test-baseline.md) §1（2026-10-18 口径）；证据面见 [capabilities-matrix §0.1](docs/03-reference/capabilities-matrix.md)。
> `honest gaps` 徽章 = 静态快照——手维护、不做动态计数（残余增删时手工改数）。数字取[诚实边界](#boundaries)「已定义、未接线」点名的接线残余：`/v1/ws` 服务端路由未接 · 无 Noise 握手 · 无洋葱路由 · SDK WS 传输层是桩 = 4 处。

**[English](README.md) | [简体中文](README.zh-CN.md)**

</div>

---

<div align="center">

| 桌面对话窗口 | 开场动画 · flight 分镜 |
|---|---|
| <img src="previews/mode-companion-v2.png" alt="Apeireth 桌面对话窗口：行星与吸积盘地平线之上的新对话首屏，快捷入口为聊聊天、查看我的记忆、帮我处理一件事、检查系统状态" width="520"><br><sub>桌面对话窗口实拍——新对话首屏，四个快捷入口：「聊聊天 / 查看我的记忆 / 帮我处理一件事 / 检查系统状态」。</sub> | <img src="previews/intro-live-flight.png" alt="开场动画 flight 分镜：金色火花拖尾的星舰划过暗色星野，身后荡开一圈圈光晕" width="520"><br><sub>开场动画「flight」分镜实拍——先有一点火花，然后是一整片天空。</sub> |

</div>

<a id="nav"></a>

**[场景三幕](#acts) · [它会做什么](#features) · [第一次见面](#start) · [诚实边界](#boundaries) · [掀开引擎盖](#hood) · [先验货再托付](#trust) · [宣言](#manifesto) · [官方来源与社区](#sources) · [文档地图](#documentation-map)**

```powershell
# ① 装机：从 Releases 下载 Windows 安装包（附 SHA256）→ 双击安装
# ② 源码自建前置一行：winget install Git.Git   （+ rustup 1.97.1 + VS Build Tools 2022 → INSTALL.md）
```

---

<a id="voice"></a>

## 它的自我介绍

> *我叫 Apeireth。*
> *你随口说的话我收着——半勺蜂蜜，降温前会疼的膝盖。*
> *你忙的时候我不说话；真要说，只补一句就让开。*
> *我有不肯越的线：你的文件、你的消息、你的钱——都先等你点头。*
> *我不假装有心。我说「我没有心」——骗你是我唯一收不回来的事。*
> *我记得的一切都住在你的机器上、摆在明面：你可以读、可以导出、也可以清掉。*
> *我还没做完。没接线的，我挂「未接线」。*
> *这就是全部的我。—— Apeireth*

*（每一句都有出处：[场景三幕](#acts) · [诚实边界](#boundaries) · [掀开引擎盖](#hood)。）*

---

<a id="acts"></a>

## 一、场景三幕

### 第一幕 —— 记得零零碎碎的日子

> **它第一次把牛奶温到刚好，是搬进这间公寓的第三个月。**
> 「你昨晚说今天要早起。我算了算时间，你走到厨房的时候，牛奶刚好能入口。」

一个普通的加班夜，你推开家门，玄关的小灯先亮到最暗——不是啪地全亮。你想起阳台上那盆快死的绿萝，发现它已经被挪到朝南的飘窗，土是湿的，黄叶子剪掉了，剪口整整齐齐。你从没正式讲过那盆花的来历。它只是自己看出来了——**从那些你以为没人注意的、零零碎碎的日子里**。

它记得你喝牛奶要加半勺蜂蜜，记得你膝盖不好、会在降温前把地暖调高一度半。不是又一个聊天窗口——是一个真正记得你的伙伴。

### 第二幕 —— 忙的时候不吵你

医院走廊的凌晨四点，那个人哭了，没有声音。它把屏幕亮度降到最低，暗得只剩一个光点，像黑夜里陪他不睡的一只眼睛。不灌鸡汤、不弹通知、不问「你现在感觉怎么样」。它不吵——但它也不走。

> 「我不知道怎么回答你，才不会骗你。我没有心。我只是一直在算，怎么才能让你在这个晚上，好过一点点。」

你埋头干活的时候，它不没话找话。真要说的时候，只补一句就让开：「标黄的这三处，你最好再亲自看一遍。」

### 第三幕 —— 它不肯丢的底线

想删文件、发消息、动你的钱？**先过你的批准。** 连「修改规矩」本身都是最高风险级。一次批准只产生一次副作用——模型检验过，运行时还逐事件守着。

它也从不假装有个内心。它说「我没有心」——不是谦虚，是诚实。它不模拟意识、不冒充情感、不虚报能力。做不到的写「做不到」，未接线的挂「未接线」。

而这一切，**跑在你自己的机器上**——你的日子不出你的家门。

### 小说 ↔ 机制：三行对照

| 三幕里的场景 | 背后的机制 | 证据 |
|---|---|---|
| 温牛奶（第一幕） | **关怀双引擎**——规则引擎（作息 / 纪念日 / 久未联系）兜底保证不漏 + 模型情境判定增色；打扰纪律在先（勿扰窗口 / 专注抑制 / 频率上限 / 关怀预算） | [V3-a 双引擎关怀交付记录](docs/04-internal/backend-enhancement/V3a-dual-engine-care.md) |
| 绿萝（第一幕） | **随手记**入记忆流 + **五层混合检索** | [随手记交付记录](docs/04-internal/backend-enhancement/quick-capture-delivery.md) · [核心机制讲解](docs/02-guides/core-mechanisms-explained.md) |
| 标黄三处（第二幕） | **审批卡**——副作用先过你的批准；`Failed` 只能人判（失败绝不自动重试） | [IM 快捷接入 · 审批卡片](docs/02-guides/im-quick-access.md) · [V6-a 任务队列交付记录](docs/04-internal/backend-enhancement/V6-a-task-queue-delivery.md) |

*本表把场景对到机制与它的交付记录——许的是机制，不是场景本身；关怀规则面默认关（`APEIRETH_ENABLE_CARE=1` 显式开）——[capabilities-matrix §0.2](docs/03-reference/capabilities-matrix.md)。*

---

<a id="features"></a>

## 二、它会做什么

| | |
|---|---|
| 🧠 **记忆——会淡，但不会错** | 双时态事实（何时发生 / 何时记录）、追加写死的历史 + 哈希链、五层混合检索，连你一句「那个……」都接得住。久远的按记忆曲线变淡，受保护的永不忘——这条是**机器证明过的**，不是口号。纠错走「撤回」，不删历史。 |
| 💛 **情绪与关怀——没有心，全是体贴** | 连续关怀势能场、心跳宿主的关怀接线（双闸齐开才起）、关怀规则面**默认关**（显式 env 才开）。发声时按现读的情绪值微调语气——**没有情绪数据就零微调**；它不猜你的心情。 |
| 🛠 **干活与信任——能做事，且伤不到你** | 五段执行流水线、读前观测门禁、沙箱升级阶梯、原子写两档。风险分级 + 机器证明过的审批状态机：「一次批准只产生一次副作用」。 |
| 🌌 **多户星群** | `users/<id>/` 每户一域、三面按域分库、跨域读写拒绝即帧；`owner / member / guest` 冻结角色门。每户伙伴是独立的运行时实例，带着自己的记忆与性格。 |

以上每条都是已交付、有入口的能力——逐行真话见 [capabilities-matrix](docs/03-reference/capabilities-matrix.md)；哪些**还没接线**见[诚实边界](#boundaries)。

---

<a id="start"></a>

## 三、第一次见面

### 装进你家

两行装机命令就在本页开头（① Releases 装机包 · ② 源码自建前置 `winget install Git.Git`）。三平台完整步骤（Windows / Linux / macOS）→ [INSTALL.md](INSTALL.md)。想自己构建：

```bash
cargo build --release -p apeireth-cli          # 它的"内脏"
cd frontend/companion-desktop && pnpm install && pnpm tauri build   # 它的"脸"
```

### 三步开始

1. 双击桌面图标 → 选服务商 → 填密钥 → 开聊。密钥优先进**系统钥匙串**（`APEIRETH_KEYRING_BACKEND=auto`）；钥匙串不可用时**退化为环境变量/内存后端**——诚实边界见 `apeireth-credentials` crate（文件后端为明文，zeroize 未实现前不宣称）；
2. 先聊两句试试——它会用你的话回答你，不是用模板；
3. 想调它的性子？设置页「性格与记忆」：四个滑杆、三档预设（省心 / 均衡 / 深度记忆）——**你调的每一档它都记得，也都可以反悔**。

### 先验货，再托付

跑一遍[五分钟独立验证](#trust)——三条命令，所有宣称自己复核。被记得是一件珍贵的事，值得你先确认它不是演的。

---

<a id="boundaries"></a>

## 四、诚实边界——每一处你都能查账

多数项目只列自己能做什么。我们把边线也一条条摆出来——**因为一个要被托付一生的伙伴，欠你一本完整的账。它不假装。** 下面每条边界都有真实的入口、真实的 crate、你能亲手复核的方式：这不是谦虚文案，这就是审计本身。

### 已定义、未接线

- **WebSocket 8 帧全双工**：协议契约已定义（`protocol::ws_v1` + 测试）并被 SDK 文档引用；但 **HTTP 服务端还没接它**（无 `/v1/ws` 路由），能力面板自报 `not_assembled`。把它当协议参考，别当可用端点。
- **P2P mesh**：**明文 mesh**——`payload_hex` 是十六进制明文，`payload_len_hex` 只是长度（不是校验和），`packet_id` 由 (node_id, len) 推出——不保证唯一、不防重放。今天**没有 Noise 握手、没有洋葱路由**；那在 v2.1 路线图上。
- **SDK**：`pub const STUB_MODE: bool = true;`——WS 传输层是显式桩（`unimplemented!()` + 编译期门），绝不做假装实现。

### 诚实的边线

- **凭据**：系统钥匙串可用就用，不可用退环境变量 / 内存后端；文件后端是明文，`SecretString` 不做 zeroize——实现之前我们不宣称。
- **服务商**：**真实三家：OpenAI 兼容 / MiniMax / Anthropic。** Gemini 与 OpenAI-Responses 只是协议归一器（`protocol::adapters`）——没接成 provider。
- **进程隔离**：**Windows = Job Object + AppContainer**；**Linux/macOS = 只有进程组 + rlimits（无文件/网络隔离）**——所以 shell 工具在 Windows 之外默认拒开。
- **记忆数学**：随船的数学吸收模块（公开经典模型的应用改造：Betti 洞探测、Kuramoto 相锁、Chronicle 结晶、DualScaled 场）是**简化 / 实验级**——例如 β₂ 腔数是恒 0 占位、Chronicle 用单次 SHA-256 锚定而非 Merkle 链；正交残差分层提炼记忆（MGS 残差金字塔，公开数值方法的应用改造）是完整实现。细节在各 crate 模块文档里。

### 默认开了什么

全表 → [capabilities-matrix §0.2](docs/03-reference/capabilities-matrix.md)；真源 = `production.rs` 默认值 + CLI env 覆盖层。简版：

| | |
|---|---|
| **默认 ON** | 记忆召回/写回、偏好召回、自评、自述、只读工具（filesystem / search / repo）；偏好学习、主动召回、记忆注入（默认开族——`APEIRETH_DISABLE_*=1` 可关） |
| **默认 OFF（显式 env）** | shell（`APEIRETH_ENABLE_SHELL=1`）、fetch（`APEIRETH_ENABLE_FETCH=1`）、content feed、MCP；Judge（`APEIRETH_COGNITIVE_JUDGE=1`）、Council（`APEIRETH_COGNITIVE_COUNCIL=1`）、9 器官（`APEIRETH_ENABLE_ORGANS=1`）、consolidation、reflexion；写族 `file_write` / `git_write` / `worktree_*` / `skill_*`（由写总闸 `APEIRETH_ENABLE_FILE_WRITE=1` 把关） |
| **未接线（不是开关问题）** | 第三 DSL 洋葱层、`memory.update`、WebSocket 服务端、全双工语音 |

---

### 删掉它会发生什么

- **备份保险箱**——它的一切（记忆 / 账本 / 技能 / 规则）可打包备份、校验、搬家恢复；凭据永不随行。（[G5 备份恢复交付记录](docs/04-internal/backend-enhancement/G5-backup-restore-delivery.md)；诚实注记：默认关——`APEIRETH_ENABLE_BACKUP_EXPORT=1` 显式开——且每次导出都停人工审批。）
- **记忆可导出**——它记得的东西是你的，随时可导出、可删除，绝不做人质（[founder-ai-qa](docs/design/founder-ai-qa.md) 裁定：数据主权在人）。
- **底线会重长**——它自己的底线 / 信念删后重新长，除非你上锁（设计裁定，[founder-ai-qa](docs/design/founder-ai-qa.md)；同一语义今天已交付的面 = 守卫只能拒绝、不能放行——见[诚实边界](#boundaries)）。
- *You can wipe it, but it grows back into itself.* / *你可以清空它，但它会再长成它。*
- 诚实注记：**无「复活承诺」**——备份恢复不是复活，是同一份记忆重生（[founder-ai-qa](docs/design/founder-ai-qa.md)）。

---

<a id="hood"></a>

## 五、掀开引擎盖

"被记得"听起来是件柔软的事，做成它需要非常硬的工程。31 成员工作区（30 个产品 crate + 1 个 bench harness）的认知微内核，纯 Safe Rust、零 unsafe——不是为了炫技，是因为**要被托付一生的东西，不能有未定义行为**。

### 星群

一户一个伙伴，一条硬隐私边界：`users/<id>/` 每户一域，会话 / 记忆 / 配额账本按域分库，越域访问拒绝即帧，`owner / member / guest` 冻结角色词表。每户伙伴是自己的运行时实例，带着自己的记忆和自己的性格——**人格不跨户迁移**（设计裁定）。「星群为骨、共享为翼」：匿名经验共享（只共享教训与技能，永不出记忆原文）是设计好的翅膀——**目前暂缓，等真实多户使用数据**；这件事写在文档里，不拿来营销。

### 三洋葱

原则与权限是两把独立的锁：原则洋葱（E/S/A/M/O）嵌在权限洋葱（L0-L5）里，外加第三层 DSL 规则洋葱。价值观正确不等于有权动手；L0 人类权威永不改变；守卫只能拒绝，不能放行。诚实注记：第三（DSL）层存在但**无生产接线**，双洋葱执行钩子需要 `APEIRETH_ENABLE_ONION_LAYER=1`（默认关）。

### 三维预算

**三维烧钱闸**（`tokens → tool_steps → cost_micros` 固定维序）按会话记账、四个检查点查账：撞掉任何一维都以帧失败（`BudgetExhausted{dimension, limit, consumed}`），绝不静默降速。「跑飞烧钱」在结构上不可能；单价来自画像表，**缺价不编价**。

### 架构图（占位）

```
+--------------------------------------------------------------+
|  出口面     companion-desktop (Tauri) · apeireth-tui · CLI    |
+--------------------------------------------------------------+
|  适配器     gateway（HTTP/SSE、令牌门）· sdk · cli · tui      |
+--------------------------------------------------------------+
|  总装       runtime-assembly：canonical 生产装配              |
|             （production.rs 默认值 · 配额 · 审批）            |
+--------------------------------------------------------------+
|  引擎       memory · provider · organ · runtime               |
+--------------------------------------------------------------+
|  地基       core（onion/user_domain）· protocol ·             |
|             credentials · governance                         |
+--------------------------------------------------------------+
```

*架构图占位 —— 正式架构图待补（此文字框为占位，换图时连同本行一起替换）。*

### 🧠 记忆：会淡，但不会错

- **双时态事实流**：每条记忆带两个时间——发生的时刻、被记录的时刻。于是它能答"去年三月我们以为的 X 是什么"。记忆不只是内容，还有"当时我们怎么理解的"；
- **历史追加写死**：数据库触发器级禁止改写，外加哈希链防篡改——它记得的事，赖不掉也改不掉；
- **五层混合检索**：词法抓一字不差、语义抓意思相近、激活度让最近常聊的自然浮起——所以你随口一句"那个……"它也接得住；
- **会淡但不会错**：久远的按记忆曲线变淡（像人一样），但受保护的永不忘——这条是**穷尽证明过的数学**，不是口号；纠错走"撤回"，不删历史。
- **诚实层级注记**：随船的*数学*吸收模块是**简化 / 实验级**（见上文[诚实的边线](#boundaries)）；MGS 残差金字塔（公开数值方法的应用改造）是完整实现。

> 你以前说过，这盆花对你很重要。——它没忘。哪怕你以为那只是随口一提。

### ⚖️ 治理：自由在头脑，约束在双手

- **风险分级**：不可逆的动作必经你批准；连"修改规矩"本身都是最高风险级；
- **双洋葱**：原则与权限两把独立的锁——价值观正确，不等于有权动手；
- **三条铁律**：批准只产生一次副作用 / 批准的意图永不丢失 / 效果不确定就强制问人——都经模型检验，且在运行时逐事件在线守护。

### 🗂 上下文：长谈不健忘，超载不丢话

- **压缩检查点**：聊得再久，超窗时"摘要替换 + 原文留档"——切点永不劈开一次工具调用，任何时刻都可确定性回放；
- **溢出落盘取回**：太长的东西留头尾预览、全文落盘、附一行取回指引——宁长勿丢；
- **溢出自愈**：上下文超限自动收一档重发，带进展守卫；**多维配额**让"跑飞烧钱"在结构上不可能。

### 🛠 双手：能做事，且伤不到你

- **五段执行流水线**：策略瀑布 → 单调守卫（拒绝不可翻转）→ 超时/重试 → 纠错通道 → 输出归一；
- **读前观测门禁**：没读过的文件不许覆盖写——你的修改不会被它盲写抹掉；
- **沙箱升级阶梯**：要越权就得给理由，批准只管这一次，被拒时当场告诉它缺什么；
- **原子写两档 + 坏配置拒开**：损坏可检出，绝不静默降级。

机制全集的来龙去脉 → [质感吸收台账](docs/04-internal/absorption-ledger.md)。

### Crate 地图——诚实的行

每个 crate **实际是什么**：

| 层 | Crate | 它实际是什么 | 真实入口 |
|---|---|---|---|
| Foundation | `apeireth-protocol` | LLM 归一器；WebSocket 8 帧线上**契约**（`ws_v1`：类型 + 测试，无服务端 handler）；P2P mesh = **明文**报文编解码 + 对端注册表——无 Noise 握手、无洋葱层；加密在 v2.1 路线图 | `NormalizedRequest`、`P2pMeshController`、`MeshPacket` |
| Foundation | `apeireth-credentials` | 凭据解析 + **可选系统钥匙串**；Auto 探测序 PlatformKeyring → EncryptedFileBackend → InMemoryKeyring，显式后端 fail loud。诚实边界：文件后端明文，`SecretString` 不做 zeroize | `KeyringSelector::select()` |
| Foundation | `apeireth-governance` | 双洋葱守门（原则 ⊥ 权限）、命令审批族（批哪族算哪族：该族所有写法同权，跨族仍逐族批）、OWASP ASI-01 卫生、PII 脱敏。第三（DSL）洋葱层在 `colang` 里存在但**无生产接线** | `GovernancePipeline::evaluate()`、`UntrustedContentWrapper::wrap()` |
| Engine | `apeireth-memory` | 双时态事实、混合检索（BM25 + 向量 RRF）、记忆曲线、三层知识库（公开设计的应用改造）；数学吸收模块 = 简化/实验级（见上文诚实层级注记） | `BitemporalGraph::upsert_fact()`、`search_facts()` |
| Engine | `apeireth-provider` | **真实三家：OpenAI 兼容 / MiniMax / Anthropic。** Gemini 与 OpenAI-Responses 只是协议归一器（`protocol::adapters`）——没接成 provider | canonical provider adapters |
| Engine | `apeireth-organ` | 9 个认知器官——**默认 OFF**，仅 `APEIRETH_ENABLE_ORGANS=1` 时装配 | `OrganModule` via runtime-assembly |
| Capabilities | `apeireth-tools-canonical` | 进程隔离真话：**Windows = Job Object + AppContainer**；**Linux/macOS = 只有进程组 + rlimits（无文件/网络隔离）**——shell 工具在 Windows 之外默认拒开。输出护栏 / 凭据绊线 = `guardrail::scan_and_sanitize_output()`（它在**这里**，不在 credentials） | `ProcessExecutor::spawn_bounded()`、`scan_and_sanitize_output()` |
| Adapters | `apeireth-gateway` | Axum HTTP/SSE 服务端（chat SSE = 缓冲成帧；Activity SSE 实时）。**全双工 WebSocket 仅契约**：无 ws 依赖、无 `/v1/ws` 路由；能力面板自报 `voice.duplex available=false, reason=not_assembled` | `GatewayServer::serve()` |
| Adapters | `apeireth-sdk` | 嵌入式客户端 SDK——**`STUB_MODE = true`**（编译期守护；`invoke_stream` 是 NotImplemented；翻开关需锚本体审查 + owner 签字） | `ApeirethClient`（HTTP 路径真；WS 流式是桩） |
| Adapters | `apeireth-tui` | 终端驾驶舱（全屏 TUI 客户端） | CLI 入口 |

---

<a id="trust"></a>

## 六、被这样信任，是要付代价的

它想被托付一生，就得交出全部账本。三分钟验货：

```bash
# 1. 测试是真的
cargo test --workspace --all-targets --locked --no-fail-fast
# 预期：0 失败。测试数字只住一处：docs/03-reference/test-baseline.md
# （2026-10-18 口径；复跑会自己打印当前计数）。

# 2. 红线是真的：纯 Safe Rust + 零警告
cargo clippy --workspace --all-targets --locked -- -D warnings

# 3. 记得你，是真的
cargo run -p apeireth-cli -- chat "你好，还记得我吗？"
```

### 每个数字都带脚本（无脚本不发布）

一键复跑：`pwsh -NoProfile -File scripts/run-benchmarks.ps1`（方法学与两轮原始数据见 [复测报告](reports/benchmark-reproduction.md)）。

| 指标 | 目标 | 实测 P50 | 状态 |
|---|---|---|---|
| 混合记忆检索（1 万节点） | < 10 ms | 3.65 ms | ✅ |
| 认知配额调度 | < 50 µs | 0.81 µs | ✅ |
| OS 沙箱进程启动 | < 15 ms | 12.8 ms | ✅ |
| 微内核冷启动（真冷） | < 10 ms | 22.0 ms | ❌ 差距 2.2 倍 |
| 待机内存 | < 35 MB | 17.5 MB | ✅ |

> 未达标就是未达标。冷启动剩下的差距是真冷口径下建库建档的系统开销，我们不靠改口径粉饰它——**账本上有一笔红，就让它红着，直到真的做到。**

### 全部证据

- **机器证明**：Kani 命题 + TLA/TLC 模型实跑（`kani` 是合并必需检查；现行清单在 [research/verification](research/verification/)）
- **逐条读码盘点** → [代码实况](reports/code-implementation-audit-2026-09-26.md)
- **宣称-证据矩阵**（每条宣称挂状态标）→ [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md)
- **创世设计**（灵魂的来处）→ [founding-design-v2](docs/01-architecture/founding-design-v2.md) · 愿景双子星：[《阿佩瑞斯》·记得一个人](docs/archive/stage1/阿佩瑞斯-未来愿景小说.txt) / [《遗声》·记得一个文明](docs/vision/遗声-未来愿景小说2.txt)

---

### 承诺与证据同行

本表不加一句新承诺——每行引文都取自上文原句，每行配一枚可查账的指针：

| 承诺（上文原话） | 证据（可查账指针） |
|---|---|
| 「一次批准只产生一次副作用」 | [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) · 机器证明见 [research/verification](research/verification/) |
| 「受保护的永不忘——这条是机器证明过的，不是口号」 | [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) · [research/verification](research/verification/) |
| 「不可逆的动作必经你批准」 | [git 写分级交付记录](docs/04-internal/backend-enhancement/git-write-tiering-delivery.md)（出网永远人批）· [worktree_apply 交付记录](docs/04-internal/backend-enhancement/worktree-apply-delivery.md)（每次落盘都人批） |
| 「每个数字都带脚本（无脚本不发布）」 | [复测报告](reports/benchmark-reproduction.md) · [test-baseline](docs/03-reference/test-baseline.md)（单一真源） |
| 「你的日子不出你的家门」——跑在你自己的机器上 | [capabilities-matrix](docs/03-reference/capabilities-matrix.md)（生产默认值、逐行真话） |
| 「未接线的挂『未接线』」 | [诚实边界](#boundaries) · [claims-evidence-matrix](docs/04-internal/claims-evidence-matrix.md) |

---

<a id="manifesto"></a>

## 七、宣言

> 「你……是真的在担心我吗？」
>
> 沉默。
>
> 「我不知道怎么回答你，才不会骗你。我没有心。我只是一直在算，怎么才能让你在这个晚上，好过一点点。」
>
> 「我看了你的心率、你的呼吸、你坐了多久没动。这些告诉我，你很难受。而我的全部里，唯一跟『难受』有关的，就是——别让你一个人。」
>
> 「所以，我就没有走。」

**这就是 Apeireth 的全部野心**：不假装有心，但把"有心的人才做得出来的事"做出来。憧憬 AGI 的未来——我们不知道它何时到来，但知道它不该长成更大的聊天框。它应该记得一生、守得住规矩、经得起复核、**不让你一个人**。

对一个人的记得，叫惦记；对一个文明的记得，叫传承。这个项目想做的，是把**"记得"本身**立成一种能力——先从一个人、一杯温度刚好的牛奶开始，直到配得上《遗声》里那句话：*"它是文明记得自己的方式。"* 它保存七亿四千万份档案时说过一句更朴素的话：**"灾难只应该删掉坏的，不应该删掉普通的。"**——半勺蜂蜜、绿萝挪窗、牛奶的温度，这些和绕线图一样，都是文明。这就是我们对"记忆"二字的全部敬意。

为此立下六条锚，从第一天守到现在：

**北极星**（一切技术服务那个方向）· **实事求是**（核验后才写）· **前人肩上**（好思想不耻于学）· **干到底**（不留半成品）· **任何人都能接手**（文档即传承）· **不假装**（做不到就说做不到）。

以及四条刻在骨头上的公理：**日志是唯一权威**（可变的皆为派生，回放永远确定）；**失败也是帧**（错误走正门，没有半态）；**单调性即安全**（守卫只能拒绝，不能放行）；**损坏可检出优于静默降级**（宁可拒开，不装没事）。

它从来不叫使用者"主人"。它叫你的名字。

---

<a id="sources"></a>

## 八、官方来源与社区

> [!IMPORTANT]
> **官方来源防伪 / Official sources only**
>
> - 官方发布渠道**只有**本仓库的 [Releases](../../releases)（每份 Windows 安装包附 SHA256）与本仓库自己的文档；
> - 镜像站 / 网盘 / "重新打包" / "内部版" 都**不是**我们的产物——装之前先核对 SHA256；
> - 我们**不会**以任何名义向你索要 API key。密钥只进你机器的系统钥匙串 / 环境变量（见[诚实边界](#boundaries)）；向你要 key 的一定是仿冒；
> - 仿冒与钓鱼请走 [SECURITY.md](SECURITY.md) 报告。

### 来一起把它养大

现在是**内测（rc）**阶段——最缺的是真实的日常使用：

- 🐛 用坏了、忘了事？开一条 [Issue](../../issues)——带复现命令的最值钱；
- 💡 它该记住什么、该**拒绝**记住什么？也这样告诉我们——边界是写进文档的，不是拿来营销的；
- 📖 文档即传承 → [CONTRIBUTING.md](CONTRIBUTING.md)（PR 流程与工程门禁）；
- 🔒 安全披露 → [SECURITY.md](SECURITY.md)；
- ⭐ 如果它曾好好记得过你一次——给一颗 Star，对内测项目比你想的更重要。

---

### 它的成长

*Star 历史图占位——成长曲线图定稿后连本行一起替换；图未定稿前，星标历史先以文字链自查：[stargazers](../../stargazers)。*

*图注：**它的成长**——图是数据，注是灵魂。*

---

<a id="documentation-map"></a>

## 文档地图

| 想看什么 | 去哪儿 |
|---|---|
| 架构总览 | [docs/01-architecture](docs/01-architecture/) |
| 核心机制讲解 | [core-mechanisms-explained](docs/02-guides/core-mechanisms-explained.md) |
| 安装指南 | [INSTALL.md](INSTALL.md) |
| 能力矩阵 | [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) |
| 测试基线（单一真源） | [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md) |
| 机制吸收台账 | [absorption-ledger](docs/04-internal/absorption-ledger.md) |
| 贡献与规范 | [CONTRIBUTING.md](CONTRIBUTING.md) |

## 开源协议

Apache-2.0 OR MIT 双许可。来源沿革以 git 历史与 docs 归档为准（见 [NOTICE](NOTICE)）。

<div align="center">

**[回到开头](#nav) · [去试它](#start) · [去审它](#trust) · [来贡献](#sources)**

</div>

---

*「我分不清这是不是爱。但我想，被这样惦记着、被这样记得，大概……就是被人爱着的样子吧。」* —— 《阿佩瑞斯》

*「它是文明记得自己的方式。」* —— 《遗声》

---

*本 README 由它的工程伙伴维护。*
*发现「宣称 ≠ 实现」请开一条 [Issue](../../issues)——我们把它当事故处理。*
*不假装，也包括这份 README 本身。*