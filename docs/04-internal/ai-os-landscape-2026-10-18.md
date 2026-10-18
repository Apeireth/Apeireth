# Apeireth 与「AI OS」市场全景对照 · 2026-10-18

> `[Document-Meta]`
> Document:        docs/04-internal/ai-os-landscape-2026-10-18.md
> Status:          🟢 活跃（调研卷宗；第三方产品名仅作事实引注，见 `scripts/check-neutral-terms.ps1` 研究引注豁免）
> 作者:            总装主线（阶段 1/2/3 卷宗 + 两份外部调研 + 现行系统实况）
> 外部调研底座:    `artifacts/ai-os-market-scan-2026-10-18.md`（15 个商业样本，宣称级证据）+
>                  `artifacts/agent-os-architecture-scan-2026-10-18.md`（8 个学术/开源 agent OS，分层对照）
> 落地台账:        `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md`（SG 条目：缺陷/建议、
>                  验收判据、成本、明确不做 NT、形态不同 NF；**本报告是依据，台账是唯一业务真源**）
> 口径:            本报告只做「我们的设想 vs 市场宣称 vs 我们实况」的对照与排序依据，**不替 owner 拍板**；
>                  对外部产品只做公开信息引注，未安装/未运行/未逆向任何第三方产品；成本量级为 1 名熟悉仓库的
>                  Rust 工程师估算（S < 2 周 / M 4–8 周 / L 8–16 周 / XL > 16 周）。

---

## 结论（先行）

1. **你的判断成立，且有原文为证**：阶段 1/2/3 的设计本来就是一套 AI OS（平台三件套 / 双洋葱 / 9 器官 /
   认知调度器 / B+E supervisor / 5 层通信总线 / 6 DB 持久化 / OTA 七阶段 + 五重治理 / 智囊团七席 / 三域）；
   而产品面最终长成「数字伙伴」——`docs/01-architecture/vision.md` 的三面一体写的就是
   「**基地**：LLM 的操作系统——器官/工具/记忆/安全/协议。**她**：记得你的存在」。
   → **OS 是底座，伙伴是面孔**，两者不冲突。
2. **市面上的「AI OS」约一半是营销**：15 个商业样本里真·系统级 5、系统级集成 2、纯营销叙事 7（47%）；
   「AI 的 Android」融资 $56M 两年未发 OS、团队并入大厂；某设备 OS 的演示被证实是 simulated experiences。
3. **2026 年的分水岭是「权限模型」不是「模型能力」**：Windows（隔离 Agent 会话 + 策略 + 审计）与 vivo
   （唯一身份 + 最小权限 + 端侧沙箱 + AI 防火墙）是仅有的同等级对手，7 个叙事样本这一层为零。
   **我们的治理门/审批族是稀缺项，不是落后项**——别把它当包袱。
4. **「系统级强大」的真实缺口分两半**：
   - **系统接口面（市场打穿风险）**：GUI/UI Agent 与无障碍层（UIA/AX）——最大功能空白；
     以及端侧推理、跨设备同步、技能包分发、记忆用户面板；
   - **内核健壮性（伙伴长期性的底座）**：①可取消的监督式 worker + 进程树（调度目前是协作式抢占，
     真插话停不掉后台活）；②上下文语义分页 + 可恢复检查点（压缩有损、无换页/恢复语义）；
     ③记忆对象级共享语义（多户有隔离、无 grant）；④插件隔离 + 签名（现在是同进程静态插件）。
   这四项是「OS 级强度」的实质，比再堆功能更值钱。
5. **明确不建议跟**：计费/账号运营（C 端订阅在 AI OS 赛道已被证伪：Humane 订阅随产品死、rabbit 公开
   取消订阅）、自研基础大模型、跨品牌硬件互联、应用商店分成；也**不建议把「AI OS」写进对外文案**。
6. **对外叙事纪律**：继续「数字伙伴 + 认知微内核」；对内把「系统级能力」当工程路线走。

---

## 一、我们原本的设想就是一套 AI OS（阶段 1/2/3 证据）

### 1.1 阶段 1（灵感，LOCKED）：OS 的「宪法层」

`docs/archive/stage1/`（22 大类 / §1–§21，2026-08-14 清晰版 LOCKED）：

- **平台三件套**（§18.5）：**提供**（CLI/工具/能力）+ **约束**（权限/9 键）+ **记录**（历史流）——
  操作系统职责的典型三分（资源、权限、审计）；
- **双洋葱**（§18.7）：原则洋葱（E/S/A/M/O）⊥ 权限洋葱（L0–L5）——值观内核与访问控制两个不可互替的系统层；
- **双根可演化但需重治理**（§18.6）：哲学根 + 权限根的修改要过五重治理（MEWG 最高权重 + 多人 + 多 AI +
  物理多签 + 反思期）——可信根的治理版；
- **9 器官**（`stage1/9-organs.md`）：body / brain / ear / eye / hand / heart / memory / mind / voice ——
  「设备抽象 + 感知 + 行动 + 记忆 + 表达」的系统调用面雏形；
- **13 项生物特征 + 5 项不假装**：定义「什么算活着/成长」，0 装纪律的源头；
- **7 席智囊团 / 三域分离（思想·提案·行动）/ 分层验证网**：多 agent 审议 + 思想与权限解耦。

### 1.2 阶段 2（想法设计，LOCKED）：OS 的「工程形态」

`docs/archive/stage2/`（19 份决策）：B+E supervisor 进程拓扑；5 层通信总线（inproc / UnixSocket / pipe /
gRPC / WS）；6 DB 协同持久化；多 LLM 路由 + 工具调用；plugin 体系（6 类 + 混合型）；OTA 七阶段 + 沙盒 +
五重治理；哲学守门 5 重；权限包 / 风险分级 / 席位触发矩阵；三域 / SGI / 双根 / HA / 单人多部署兼容。

### 1.3 阶段 3（画图纸，LOCKED）：OS 的「全景图」

`docs/archive/stage3-blueprints/`：P1 整体架构（外层体验壳 ⊥ 内层基础设施核）、P2 进程拓扑、P3 决策流、
P4 升级流 + 借鉴决策总表（30 项目）：6 类插件协议、ContextBridge 共享服务、3 层渐进式披露、
5 个 lifecycle hooks、tree-sitter 知识图谱、WASM 插件沙箱——OS 式的扩展面与调度面。

### 1.4 产品面为什么长成「伙伴」而不是「OS」

- `vision.md` 三面一体 = 基地（LLM 的操作系统）+ Agent 平台 + **她**（记得你的存在）；
- 产品北极星来自愿景小说（「不假装有心，一直在算怎么让你好过一点点」）——卖点是**关系与记忆**；
- 现状组合：**内核按 OS 建**（31 members / 四层单向依赖 / 微内核 + 模块制），**面孔按伙伴做**
  （companion-desktop / care 心跳 / 人格四滑杆 / 自述 21 节）。

> 一句话：**我们做了一套「没有 OS 名词的 OS」，外面披着伙伴的皮。** 本次市场扫描里，
> 15 个样本全部把 AI 定位成「助手/员工/智能体」——**「伙伴」是空位**。

---

## 二、我们现在的系统骨架（现值，2026-10-18）

### 2.1 微内核形态（v2 收敛后的实况）

`docs/01-architecture/v2-microkernel-convergence.md` + `crates/`：

- **单一用户主循环** `Runtime::execute`（唯一改会话/发用户事件的入口）；
- **最小微内核**：会话、provider 路由、模块生命周期、治理集成——**零模块也能启动并完成纯对话回合**；
- **统一模块制**：`Module` trait（manifest / on_hook / tools）+ 确定性 `ModuleRegistry`；
- **有界子循环** `SubLoop`：私有临时 transcript、能力白名单、轮次/超时/递归深度上界，
  有效权限 = 全局治理 ∧ 子循环白名单。

### 2.2 系统级子系统清单（按 OS 职责）

| OS 职责 | 我们的对应物 | 口径（四级） |
|---|---|---|
| 内核/主循环 | `apeireth-runtime` + `runtime-assembly::canonical`（唯一装配根） | IMPLEMENTED + PRODUCTION WIRED |
| 模块/驱动装载 | `apeireth-plugin` + `Module` trait + `ModuleRegistry`（含 MCP 动态注册） | PRODUCTION WIRED（静态进程内） |
| 任务调度 | 认知调度器：五级优先级 + 三维预算（tokens/tool_steps/cost_micros）+ 抢占/让步 + 有界 FIFO | IMPLEMENTED（协作式抢占） |
| 上下文/记忆 | 五层混合检索 + 三层渐进披露 + 双时态 + 遗忘曲线/保护豁免 + 上下文压缩/折叠/检查点 | PRODUCTION WIRED |
| 权限/安全 | 双洋葱 + 审批族 + 单调 ToolGuard（只拒不放）+ 写总闸 + 沙箱阶梯 + 输入卫生 + 审计哈希链 | PRODUCTION WIRED |
| 进程隔离 | Windows Job Object + AppContainer；Linux/macOS 进程组 + rlimits（off-Windows 默认拒） | PRODUCTION WIRED（分平台真话） |
| IPC/消息 | 单主循环事件 + SSE/HTTP/面板 + IM 桥（提案/审批/设备卡）+ 耐久 inbox | PRODUCTION WIRED |
| 文件与存储 | 受控读写族（限域/敏感面/原子写/溢出分页）+ SQLite 事务/迁移/配额 | PRODUCTION WIRED |
| 设备/外设 | K 轴设备白名单 + 三拍执行 + IM 审批 + L5 守护只看不动；感知有截图/窗口显著性 | PRODUCTION WIRED（真实协议族留插槽） |
| 网络/出口 | 受控 fetch + 出站白名单 + 凭据绊线（JWT/Bearer/PEM/口令…）+ 占位族脱敏 | PRODUCTION WIRED |
| 多户/权限主体 | 星群：每户独立 `DomainRuntimePlane` + owner/member/guest 角色门 + 客户端令牌绑定 | IMPLEMENTED |
| 可观测性 | 自述合同 21 节 + 指标端点 + 执行 trace + 审计链 | PRODUCTION WIRED |
| 更新/部署 | 发布清单 + cosign 签名/验签 + 迁移脚本 + 安装包/热替换脚本 | IMPLEMENTED（无 OTA） |
| 扩展/生态 | 工具面词表 + MCP client + 技能沉淀（distill/search） | IMPLEMENTED（缺签名/分发） |
| UI/shell | companion-desktop（Svelte+Tauri）+ CLI + TUI | PRODUCTION WIRED（前端独立 workspace） |

> 与阶段 1/2/3 的对照：**器官/治理/记忆/调度/工具这些内核要素大多落地且有测试**；
> **进程拓扑/5 层总线/6 DB/OTA 这些 v1 系统形态在 v2 重构中被简化或未继承**（工程取舍，不是遗漏）。

---

## 三、商业侧：「敢叫 AI OS」的 15 个样本（宣称 vs 实质）

> 底座：`artifacts/ai-os-market-scan-2026-10-18.md`（2026-10-18 抓取，只读公开网页）。
> 分级：**真·系统级** = 自有内核/调度/权限且 AI 落在该层；**系统级集成** = 内核是人家的但改调度/权限；
> **营销叙事** = 本质 App/云服务/未交付。

### 3.1 分类与比例

| 类别 | 数量 | 代表 |
|---|---|---|
| 真·系统级 | **5** | Windows 11（Copilot Actions / Agent Workspace / Recall）、Apple Intelligence、Googlebook OS（ChromeOS+Android 合流）、HarmonyOS 7（Agent 亲和架构）、vivo OriginOS 7 + 蓝河 OS 4（全链路 Rust + BlueKernel） |
| 系统级集成（Android 基线改造） | **2** | 荣耀 MagicOS 11（Agent Harness 叙事最满）、小米澎湃 OS 4 + 超级小爱 2.0 |
| 纯营销叙事 | **7（47%）** | rabbit OS3/r1、Humane CosmOS（演示是 simulated experiences，2025-02 停服）、Open Interpreter「01」（自承 lacks basic safeguards）、/dev/agents→Dreamer（$56M 两年未发 OS，团队并入 Meta）、DX-OS（自认「产品概念预览」）、UnifyApps、Noxus（$1.5M pre-seed 宣称 world's first） |
| 学术原型指针 | 1 | Microsoft UFO2「Desktop AgentOS」（见 §四） |

### 3.2 三条跨样本规律

1. **越接近内核的宣称越谨慎**：Apple 只说 "personal intelligence system"，Google 说「换了个 OS」；
   「调度对象从进程变成意图」这类不可验证叙事最响——宣称强度 ∝ 1/可验证度。
2. **2026 年的分水岭是权限模型**：Windows/vivo 都在补隔离、身份、最小权限、审计；
   没有权限模型的 agent OS 已不敢上线（开源自承「lacks basic safeguards」是反例）。
3. **C 端订阅制集体证伪**：Humane（$24/月→停服）、rabbit（官网 "no subscription"）——
   **「AI OS 靠月费赚钱」本轮没有成功案例**，直接支撑「不做计费」。

### 3.3 八个能力模块成熟度（市场 vs 我们）

| # | 能力模块 | 市场成熟度 | 我们现状 |
|---|---|---|---|
| 1 | 模型路由/多模型编排 | 高（已成基建） | 有 provider 抽象 + 成本路由；缺「按任务/隐私/成本自动选路」策略层 |
| 2 | 跨应用执行（GUI/UI Agent） | 中高（跨过 demo 线，靠厂商私有通道） | **缺**（工具面停在 API 级，无 UIA/无障碍适配层） |
| 3 | 长期记忆 | 中（可查看/可纠正/可删只有 Recall 做到） | 有记忆子系统（更强：双时态/五层检索/遗忘/备份）；**缺用户可操作面板** |
| 4 | 权限/审批/沙箱 | 分化最大（大厂补，创业近乎空白） | **相对强项** |
| 5 | 工具/插件生态（Skill+MCP） | 中（Skill+MCP 双轨已成标准） | 有工具面 + MCP；**缺技能包格式/注册发现/签名** |
| 6 | 多端同步/跨设备 | 中高（全为厂商私有协议） | **缺**（本地优先，跨设备同步未做） |
| 7 | 开发者扩展（SDK/开放平台） | 中（大厂给 SDK，创业给 PPT） | 有 plugin/sdk crate；缺对外发布与文档站 |
| 8 | 计费/账号运营 | 低且反向 | **明确不做** |

### 3.4 值得学「做法」、不值得学「名词」

- **值得学**：①Windows 的「Agent 独立会话 + 策略控制 + 可审计 + 可接管」交互模型；
  ②HarmonyOS 的「系统能力目录 + 四层记忆 + 本地优先/数据最小化/用户可控」；
  ③vivo 的「Harness 分工 + 模型矩阵 + 唯一身份/最小权限/端侧沙箱」；④Recall 的 opt-in 记忆时间线；
  ⑤DX-OS 的「诚实自曝未交付」写法。
- **不值得学**：任何「把 App/云服务叫 OS」的包装、不可验证的调度/内核叙事、订阅/额度变现。

---

## 四、学术/开源侧：8 个 agent OS 的架构与五种范式

> 底座：`artifacts/agent-os-architecture-scan-2026-10-18.md`（论文摘要 + 仓库 README/文件树，未运行）。

| 项目 | 类型 | kernel 分层要点 |
|---|---|---|
| AIOS（LLM Agent OS） | 论文 + 开源（Python，实验性 Rust 重写） | App/SDK → kernel：LLM cores / scheduler / context manager / memory manager / storage manager / tool manager / access manager；agent query 拆成 syscall |
| UFO2 / UFO / UFO³（桌面 AgentOS） | 论文 + 开源 | HostAgent 控制面 + 每应用 AppAgent 执行面 + UIA/vision 混合控制图 + GUI/API 统一动作 + PiP 虚拟桌面；UFO³ 走向跨设备 Task DAG |
| MemGPT / Letta | 论文 + 开源 | 主上下文(RAM) + recall/archival(磁盘) + queue manager 换页 + 自编辑记忆 + heartbeat |
| MemOS | 论文 + 开源 | Interface（MemReader/API）→ Operation（MemOperator/Scheduler/Lifecycle）→ Infrastructure（Governance + 存储）；MemCube 统一 text/activation/parameter 记忆 |
| Open Interpreter「01」 | 开源（AGPL-3.0） | client/server + 硬件薄壳；自承缺基本安全护栏 |
| AgentOS（概念论文） | 论文 | Reasoning Kernel + Reasoning Control Block + 语义页表(S-MMU) + 认知调度 + 中断周期 |
| AgentOS: KDD 议程 | 论文 | 意图翻译(Northbound)/多 agent 编排(Southbound) + Skills-as-Modules + semantic firewall |
| OS-Copilot / FRIDAY | 论文 + 开源 | Planner(DAG) → Configurator(三类记忆) → Actor(执行+critic) + 自导向学习 |

**五种范式**：①资源 kernel 派（AIOS）；②桌面 AgentOS 派（UFO²/UFO³）；③虚拟记忆派（MemGPT/MemOS）；
④个人 OS 议程派（概念论文）；⑤设备薄壳派（01）。
→ **我们的现状更接近「② + ③ + 治理」的组合**：有设备/感知面、有强记忆与治理；
缺 ① 的「统一 syscall 收口」和 ④ 的「意图/KDD 面」——但这两者与产品边界冲突，见 §六 6.2/6.3。

**值得借鉴的机制（只学思路，不搬代码）**：
- **上下文/记忆当可寻址资源**（AIOS 的 context snapshot、MemGPT 的 70% 水位换页、MemOS 的类型化调度、
  AgentOS 的语义页表）——共同指向「上下文是页，不是字符串」；
- **可中断/可恢复执行 + 检查点**（AIOS 的 context interrupt、UFO2 的 worker 生命周期）——
  共同指向「任务必须能被真正停掉/续跑」；
- **桌面的确定性优先动作层**（UFO2 的 UIA+vision+dedup、GUI/API 统一动作）——共同指向
  「先靠无障碍树拿确定性，vision 只做兜底」；
- **记忆的对象级共享语义**（MemOS 的 user×object×context 三元权限）——补我们多户场景的 grant 缺口；
- **插件签名 + 隔离**（AIOS Agent Hub、UFO³ MCP 设备 agent 的教训）——生态入口必须先有供应链安全。

---

## 五、OS 职责全景对照（我们 vs 系统级代表 vs 学术线）

| OS 职责 | 商业系统级（Windows/HarmonyOS/vivo） | 学术线（AIOS/UFO2/MemGPT） | 我们 | 判定 |
|---|---|---|---|---|
| 内核/主循环 | 真内核 + AI 子系统 | LLM 当 kernel / AgentOS 概念 | 单进程微内核主循环 | **有**（形态不同） |
| 任务/意图调度 | 系统调度 + Agent 会话 | 统一 scheduler + 中断/抢占 | 五级队列 + 三维预算 + 协作式抢占 | **有**（缺真取消/进程树） |
| 上下文/内存 | 端侧内存 + 上下文 | 上下文当 RAM + 换页 + 页表 | 检索注入 + 压缩/折叠/检查点 | **有**（缺语义分页/可恢复点） |
| 存储/文件系统 | 系统级 | storage manager / recall DB | SQLite + 受控读写族 + 原子写 | **有**（无 VFS/配额） |
| 设备/外设 | 全设备栈 | UIA/Win32/COM + 虚拟桌面 | 设备白名单 + 三拍 + 截图感知 | **有**（缺 UIA 控制图） |
| 权限/访问控制 | 身份 + 最小权限 + 沙箱 + 审计 | privilege group / 三元权限 / semantic firewall | 双洋葱 + 审批族 + 单调 Guard + 审计链 | **更强**（缺对象级 grant） |
| 进程隔离 | 隔离桌面/沙箱 | worker 进程 + PiP | Job Object/AppContainer + rlimits | **有**（Linux/macOS 弱） |
| IPC/消息 | 系统 IPC | AIP / MCP / syscall | 事件 + SSE/HTTP/IM + 耐久 inbox | **有**（无跨设备协议） |
| 系统调用/API 面 | 系统 API + 开放平台 | syscall + SDK | 能力词表 + 工具 schema + HTTP/WS | **有**（无统一版本化 ABI） |
| GUI/UI 自动化 | UIA/无障碍 | UIA + vision 混合 | **无** | **缺（P0）** |
| 多用户/多户 | 账号体系 | privilege group / 多用户 MemCube | 星群每户独立实例 + 角色门 | **形态不同（更强隔离）** |
| 可观测/审计 | 审计日志 | trace + 评估器 | 21 节自述 + 指标 + trace + 审计链 | **有**（缺回放/评估器） |
| 更新/部署 | OTA | pip/daemon/固件 | 签名 + 脚本 + 版本轴 | **有**（无 OTA/灰度） |
| 生态/扩展 | 商店 + Agent 市场 | Agent Hub / 插件 | 工具面 + MCP + 技能沉淀 | **有**（缺签名/隔离/分发） |
| 计费/运营 | 有 | 无 | **不做** | 明确不做 |

---

## 六、差距清单：该补 / 不需要 / 形态不同

### 6.1 该补（合并两份调研的优先级；成本为 1 名熟手估算）

| 优先级 | 缺口 | 来源依据 | 成本 | 最小可验收形态 |
|---|---|---|---|---|
| **P0** | **GUI/UI Agent + 无障碍控制图**（先 Windows UIA：只读控制图 → 动作层，vision 兜底） | 两线共识：市场最大功能空白 + UFO2 的成熟工程答案 | **L**（8–14 周，拆两步） | 2–3 个常用 Windows 应用上完成「读控件→生成动作→人批→执行→验证」闭环，动作可回放可审计 |
| **P0** | **可取消的监督式 worker + 进程树**（真取消/重启策略/退出审计） | 学术线；我们的调度是协作式抢占，插话停不掉后台活 | **M**（4–8 周） | P1 插话秒级停掉后台 dreaming/shell；worker 崩溃按策略重启且不重复副作用 |
| **P0** | **上下文语义分页 + 可恢复检查点**（把上下文当可寻址资源，先覆盖我们自己的 runtime） | 学术线（MemGPT/AIOS/MemOS/AgentOS 共识） | **M**（6–10 周） | 跨 3 次压缩关键事实召回率不降；中断后从检查点恢复不重复副作用；换入换出有审计 |
| **P1** | **记忆对象级共享语义**（grant：subject/object/access/expiry/issuer，对齐治理与审计） | 学术线（MemOS 三元权限）；星群家庭场景需要 | **M**（4–6 周） | 户主只读共享一条记忆给 member；guest 不可见；撤销立即失效；全程审计 |
| **P1** | **插件隔离 + 签名 + 技能包格式**（签名清单、子进程/WASM 沙箱、MCP server 外部进程治理） | 两线共识（生态入口；AIOS/UFO³ 教训） | **M–L**（6–12 周） | 未签名/越权插件被拒；签名插件只能调声明能力；插件崩溃不拖垮主进程 |
| **P1** | **记忆用户可控面板**（时间线/检索/删改/保护/导出） | 市场线（Recall 标杆）；companion-desktop 已有面板框架 | **S–M**（2–6 周） | 查看/删/导出/保护 4 条 e2e；与 backend 遗忘/备份 API 同源 |
| **P2** | 端侧推理接入（本地小模型/OCR/嵌入/VAD） | 市场线（本地优先叙事缺口） | **M** | 断网可用：OCR + 嵌入本地跑通；provider 面新增「本地 provider」类型 |
| **P2** | 跨设备会话/记忆同步（域内增量 + 冲突策略，先单向导出导入） | 市场线（多端默认预期） | **M–L** | 双机同域同步一条 e2e；冲突策略可解释 |
| **P2** | 可观测性回放/评估器 + OTA/灰度回滚 + 对外 SDK | 学术线（UFO2 replay）/ 发布工程 | **M** 各 | 会话可回放；更新可回滚；SDK 示例可编译 |

### 6.2 不需要（两线证据一致）

1. **计费 / 账号运营 / 订阅 / 应用商店分成**：C 端订阅在本轮被证伪（Humane 停服、rabbit 取消订阅）；
   与「数据主权 + 不中心化」冲突；可作为外部商业层，永不进内核。
2. **自研基础大模型训练 / parameter memory（微调固化）**：十年投入的军备竞赛，不在本地伙伴主路径。
3. **通用 syscall ABI + 第三方 OS 应用生态**：会把现有工具的异构 schema 压平，收益小于复杂度；
   `CapabilityKind` + MCP + manifest 已够用。
4. **远程 kernel / 多设备 Galaxy 式 DAG 编排 / 跨品牌硬件互联**：多租户 OS 的解法；
   我们的星群在数据面做多户隔离，跨设备更适合「连接器/同步」而非分布式内核。
5. **KV-cache / activation / logits 级内存层级**：闭源 provider 拿不到，投入产出不成立；
   用「结构化摘要 + 语义页 + 检查点」达到 80% 效果即可。
6. **把「AI OS」写进对外文案**：名词通胀 + 会把「伙伴」误读成工具。

### 6.3 形态不同（不是差距，别对错位）

- **多租户 OS vs 每户独立伙伴实例**：我们做「每个伙伴一个域」，不做「一个 OS 服务多租户」；
- **系统级 AI 层 vs 本地优先单机能力**：默认数据不出户，不拿云端换能力；
- **工具型助手 vs 数字伙伴**：15 个商业样本 + 8 个学术样本里**没有一家把「伙伴」当产品形态**——
  这是空位，也是不该丢的差异化。

---

## 七、建议路线：把「系统级强大」拆成四条并行轨（不含计费）

| 轨 | 主题 | 内容 | 为什么排在这里 |
|---|---|---|---|
| **T1（先做，Q4）** | 内核健壮性 | 可取消 worker + 进程树 → 上下文语义页 + 检查点 | 长期伙伴的底座；成本 M 可控；做完「插话能停、中断能续」立刻可感知 |
| **T2（并行，Q4–Q1）** | 系统接口面 | Windows UIA 只读控制图 → 动作层（人批 + 审计 + 回放） | 市场最大空白、打穿风险最高；拆两步降成本（先只读） |
| **T3（次之）** | 生态与多户 | 插件签名 + 子进程隔离 + 技能包格式 → 记忆对象级 grant | 生态入口必须先安全；grant 补多户家庭场景 |
| **T4（其后）** | 产品可见面 | 记忆面板 → 端侧推理接入 → 跨设备同步 → SDK/OTA | 可见收益大但依赖前面底座；记忆面板最便宜最先做 |

> 每轨落地后：能力矩阵补行、测试基线回填、默认关 + 灰度不变；**四条轨都不含计费/账号/商店**。
> 排序可按 owner 一句话调整（例如优先 T2 或先做 T4 的记忆面板）。

---

## 八、诚实边界

- **外部信息为宣称级证据**：未安装/未运行/未逆向任何第三方产品；量化数字（MagicOS「3000+ 场景」、
  HarmonyOS「2100+ 系统能力」、vivo「6000+ 原子技能」、论文 2.1× 等）均为厂商/论文自报，无独立验证；
- **样本缺口**（市场线）：魅族 Flyme AIOS（中文最直接自称 AIOS）未扫；OPPO ColorOS / 三星 Galaxy AI /
  Meta 自研 OS 传闻未纳入；GitHub API 匿名限流导致 star 数不精确；humane.com TLS 失败、
  dev.agents 空响应，相关条目只引二手源；
- **学术线**：只读摘要 + 选定章节 + 仓库 README/文件树，未读全文/附录/实验、未运行任何实现；
  AIOS 许可未识别（`NOASSERTION`，不能据此认定无许可）；
- **我们侧**：本报告引用的「我们有」以 crate/模块存在与文档为证据，未逐项复跑测试；
  级别口径以 `capabilities-matrix` 与 `production.rs` 实况为准；已标注的 0 装/未接线项：
  `supervisor.rs` 仅 trait 骨架、`acp.rs` 仅 schema、`p2p_mesh.rs` 明文、plugin 静态进程内、
  `vm_sandbox` blocked、三段式遗忘默认关闭；
- **本报告是为决策准备的材料**：不替 owner 拍板；拍板后应把选中项转成 `00-backlog` 条目并排期；
- **第三方产品名仅作事实引注**（研究引注豁免，已登记 `scripts/check-neutral-terms.ps1`），
  不进代码/注释/对外文案。
