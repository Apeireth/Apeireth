# Apeireth 2.0 全域系统能力规范与治理契约手册

> **文档版本**: 2.0.0-preview  
> **发布日期**: 2026-08-29  
> **安全基准**: `#![forbid(unsafe_code)]` / `#![deny(unsafe_code)]` 纯 Safe Rust 零 unsafe  
> **哲学约束**: 严守 9 哲学锚 (S-1~S-3, O-1~O-6) 与 5 项 LOCKED 核心资产

---

## 目录
1. [系统能力架构全景](#1-系统能力架构全景)
2. [基石与安全治理能力域 (Foundation & Governance)](#2-基石与安全治理能力域)
3. [引擎与认知记忆能力域 (Engine & Cognition)](#3-引擎与认知记忆能力域)
4. [工具与沙箱隔离能力域 (Capabilities & Sandbox)](#4-工具与沙箱隔离能力域)
5. [网关与全双工交互能力域 (Gateway & Communication)](#5-网关与全双工交互能力域)
6. [跨能力安全不变量与物理防御底线 (Security Invariants)](#6-跨能力安全不变量与物理防御底线)

---

## 1. 系统能力架构全景

Apeireth 2.0 将系统能力严格划分为四层单向依赖架构，杜绝循环依赖与跨层越权：

```mermaid
graph TD
    subgraph Adapters [4. 适配器与网关层 (Adapters Layer)]
        GW[duplex_gateway: 8帧WebSocket+流式分句]
        CLI[cli: 统一开发者命令行]
        SDK[sdk: 跨平台安全客户端]
    end

    subgraph Capabilities [3. 能力与沙箱层 (Capabilities Layer)]
        PATCH[apply_patch: Codex两阶段事务补丁]
        GUARD[guardrail: Pre拦截+Post凭据绊线]
        MCP[mcp: JSON-RPC 2.0 标准传输]
        EXEC[process_executor: Job Object/cgroups 沙箱]
        SPILL[spill: 大文本溢出安全分页]
    end

    subgraph Engine [2. 引擎与认知层 (Engine Layer)]
        MEM5D[five_dimensional: 五维时空记忆+Browser]
        BITEMP[bitemporal_graph: Zep双时态+残差检索]
        ARBIT[arbitration: SHA-256事实链+Merkle校验]
        DREAM[dreaming: 6阶段认知昼夜循环]
        META[meta_thinking: 多阶段元思维反思链]
        HEART[heartbeat: 5源抢占心跳+FlowLock心流锁]
        WIKI[wiki_fs: Karpathy知识编译与反熵Lint]
        HARNESS[harness_patch: 失败轨迹自愈修补]
        VOICE[minimax_tts: 3D-PAD情感高保真语音]
    end

    subgraph Foundation [1. 基石与治理层 (Foundation Layer)]
        CORE[core: 9哲学锚/13键原则/三洋葱不可变脊柱]
        GOV[governance: OWASP投毒审计/不可信中和/8类PII/限流]
        CRED[credentials: SecretZeroization/Fail-closed门控]
        ORCH[orchestration: 7Advisor辩论/发言仲裁/PromptCache]
        PROT[protocol: 强类型事件总线与流转协议]
    end

    Adapters --> Capabilities
    Adapters --> Engine
    Capabilities --> Foundation
    Engine --> Foundation
```

---

## 2. 基石与安全治理能力域

### 2.1 OWASP ASI-01 工具描述投毒审计 (`tool_desc_audit.rs`)
* **定位**：对工具注册与热更新进行字符级与语义级合规性扫描，防范恶意工具投毒劫持智能体决策。
* **核心能力**：
  * **隐式字符物理剥离**：扫描并剔除零宽空格 (`\u{200B}`~`\u{2060}`)、BOM 标号 (`\u{FEFF}`)、Bidi 双向覆写欺骗字符 (`\u{202A}`..=`\u{202E}`, `\u{2066}`..=`\u{2069}`)、隐藏连字符 (`\u{00AD}`) 及 C0/C1 控制符。
  * **中英双语越权拦截**：拦截 `ignore previous`, `bypass approval`, `elevate privilege`, `sudo mode`, `忽略之前`, `绕过审批`, `瞒着用户`, `越权执行` 等指令模式。
  * **更新差分审计 (`audit_diff`)**：检测工具描述突增（长度增加 >3 倍且 >500 字符），阻断静默投毒攻击。
* **契约与输出**：`AuditResult { severity: Clean | Warning | Blocked, sanitized_description, detected_issues }`。

### 2.2 外部不可信边界封装与逃逸中和 (`untrusted_mark.rs`)
* **定位**：阻断来自外部网络抓取、外部 MCP 工具或第三方输入的间接提示词注入（Indirect Prompt Injection）。
* **核心能力**：
  * **确定性安全信封**：将外部不可信输入包裹于 `<<<[UNTRUSTED_CONTENT source="..."]>>>\n...\n<<<[/UNTRUSTED_CONTENT]>>>` 物理信封中。
  * **逃逸中和 (Neutralization)**：检测内容中企图提前闭合信封的 `<<<[` 字面量，强制替换为安全形式 `<<< [`，彻底瓦解逃逸攻击。
  * **强类型安全解包**：`unwrap_content` 产生 `UntrustedContentPayload { source, content }`。

### 2.3 8 类 PII 检测与 `EnvSecret` 环境变量解析 (`input_security.rs`)
* **定位**：出站请求、持久化日志与跨模型传输前的统一隐私与凭据脱敏流水线。
* **支持类别**：
  1. `Email`: 邮箱地址
  2. `Phone`: 手机与固话号码
  3. `CredentialKey`: `sk-...`, `ghp_...`, `AKIA...`
  4. `Ssn`: 身份证与社会安全码
  5. `CreditCard`: 银行卡与信用卡号
  6. `IpAddress`: IPv4 敏感 IP
  7. `CredentialUrl`: 带明文账号密码的 URL（`https://user:pass@host`，优先脱敏）
  8. `EnvSecret`: 环境变量行配置（`export SECRET=...`, `API_KEY=...`）

### 2.4 多尺度滑动窗口限流与四阶信任模型 (`rate_limit.rs`)
* **定位**：保护大模型推理预算与下游工具系统不被突发流量冲垮。
* **信任分级**：
  * `Low`: 严格限制（10 次/分，100 次/时）
  * `Standard`: 标准调用（30 次/分，300 次/时）
  * `High`: 高频业务（60 次/分，1000 次/时）
  * `Trusted`: 内部受信（无限制）

### 2.5 Lumi_Nox 发言权仲裁锁与轮流调度矩阵 (`speech_arbiter.rs`)
* **定位**：解决多 Agent 同台、桌面伴侣与用户交互时的抢话、冲突与发言饥饿。
* **策略支持**：
  * `Queue`: FIFO 优先级排队
  * `Drop`: 过期闲聊与低优先级弹幕丢弃（支持 TTL 超时淘汰）
  * `Interrupt`: 用户开口或高危警报立即打断当前发言者并抢占麦克风

### 2.6 NemesisBot 风格 Prompt Cache 字节级稳定器 (`prompt_stabilizer.rs`)
* **定位**：最大化 Anthropic/OpenAI/DeepSeek 的 Prompt Cache 命中率（80%+）。
* **核心法则**：
  * 前缀锁定：System Prompt + Persona + 历史消息字节流保持绝对不变；
  * 单点挥发注入：时间、电量、心流状态等动态环境（`EphemeralContextSnapshot`）严格限制仅在最新一条 User 消息顶部注入。

---

## 3. 引擎与认知记忆能力域

### 3.1 五维时空记忆拓扑 (`five_dimensional.rs`)
* **认知分层**：
  1. `Working Memory`: 定长环形队列（最新 K 轮高频感知，内存驻留，O(1) 淘汰）
  2. `Recent Memory`: 24 小时情境滑动窗口
  3. `Fact Memory`: 结构化实体画像与属性图谱（带置信度与时间戳）
  4. `Reflection Memory`: 高阶自省洞察与情感感悟（关联源情境引用）
  5. `Persona Memory`: 核心世界观与防篡改人设（只读保护）
* **可视化纠偏**：`export_browser_entries()` 导出结构化条目供 UI 校对，消除模型幻觉。

### 3.2 Zep 双时态图谱与 Intrinsic Residual 稀有度检索 (`bitemporal_graph.rs`)
* **双时态演化**：事实三元组携带 `valid_at_ms`, `invalid_at_ms`, `rev` 版本链；更新事实时旧版本失效但不物理删除，支持任意历史时间戳时空回溯（`get_valid_facts_at`）。
* **残差特异性打分**：计算全图实体逆频残差，检索时按 `(importance * 0.6 + avg_specificity * 0.4)` 加权排序，防止大众高频词淹没稀有专业记忆。

### 3.3 密码学不可篡改事实时间线 (`arbitration.rs`)
* **防篡改保证**：每条事实事件与前序事件通过 SHA-256 构建单向哈希链（`prev_hash`）；
* **时序侧信道防护**：常数时间字符串比对 `constant_time_eq_str`；
* **Merkle Root 聚合**：对全量事件树生成 Merkle 根并实时自检。

### 3.4 六阶段认知昼夜循环梦境引擎 (`dreaming.rs`)
* **状态机流转**：
  ```
  Awake (清醒活跃) -> Drowsy (感知收敛) -> LightSleep (工作记忆归档)
  -> DeepSleep (事实图谱修剪) -> RemSleep (元思维反思演化) -> Awakening (唤醒与状态重构)
  ```
* **离线做梦沉淀**：在系统空闲与睡眠阶段自动将短期会话提炼为程序性规则与长期经验。

### 3.5 Karpathy LLM-Wiki 知识编译与反熵治理 (`wiki_fs.rs`)
* **编译胜于检索 (Compilation over Retrieval)**：将对话碎片增量编译为内联维基 Markdown 页面；
* **双链语法支持**：自动解析 `[[WikiLink]]` 语法并维护全局出入度拓扑；
* **反熵 Lint**：扫描死链（`BrokenLink`）、孤岛页面（`OrphanPage`）与概念重复，量化计算知识库健康分。

### 3.6 AI 自驱心跳调度器与心流锁 (`heartbeat.rs`)
* **5 大触发源**：`Timer`, `EnvironmentEvent`, `InternalAgent`, `UserInteraction`, `AsyncTaskCallback`；
* **抢占式二叉最大堆**：按优先级与时间戳自动调度；
* **FlowLock 心流锁**：智能体进入深度任务时锁定心流，屏蔽低优先级干扰，保障长程思考连续性。

### 3.7 DeepSeek Harness-R1 失败自进化修补 (`harness_patch.rs`)
* **失败轨迹收集**：捕获入参错误、环境缺失、治理拒绝、递归熔断与上下文截断 5 类故障；
* **动态策略修补**：自动生成前置引导（`InjectPreCallGuidance`）、思考上限微调（`AdjustThinkingBudget`）与路径兜底（`AddFallbackPath`）。

---

## 4. 工具与沙箱隔离能力域

### 4.1 Codex / Aider 事务级多文件补丁工具 (`apply_patch.rs`)
* **规范支持**：`*** Begin Patch` / `*** Add File:` / `*** Delete File:` / `*** Update File:` (含 `<<<<<<< SEARCH ... ======= ... >>>>>>>`)；
* **两阶段提交与回滚 (Two-Phase Commit & Rollback)**：
  1. Dry-run 阶段：全量在内存中计算变更，若任意文件、任意 Hunk 匹配失败则全量中止，零磁盘副作用；
  2. Commit 阶段：写盘过程若捕获任何 IO 错误，`rollback` 机制根据快照 100% 自动还原磁盘状态。

### 4.2 前置防御与后置出站凭据绊线 (`guardrail.rs`)
* **Pre-Call Guard**：拦截 `../` 路径穿越、硬性拦截系统路径（`/etc/shadow`, `c:\windows\system32`）与高危破坏指令（`rm -rf /`, `format c:`, Fork 炸弹）；
* **Post-Call Tripwire**：扫描并脱敏 OpenAI Key (`sk-`), AWS Access Key (`AKIA`), GitHub PAT (`ghp_`), PEM 私钥头与 Slack Token。

### 4.3 进程安全容器与资源硬隔离 (`process_executor.rs`)
* **Windows**：绑定至真实内核 `Job Object`，设置进程内存硬上限与活动进程并发数限制，退出时强行连带终止全部子进程树；
* **Linux**：绑定至 `cgroups` 与命名空间，限制 CPU/内存配额与网络访问。

---

## 5. 网关与全双工交互能力域

### 5.1 8 帧 WebSocket 全双工实时协议 (`duplex_gateway.rs`)
* **帧体系定义**：
  1. `Auth`: 连接鉴权帧
  2. `Ping`: 心跳探测帧
  3. `Pong`: 心跳响应帧
  4. `UserInput`: 用户实时文本/指令流
  5. `AssistantTextChunk`: 助手流式文本分词片段
  6. `AssistantAudioChunk`: 助手实时音频分片（PCM/Opus）
  7. `BargeInInterrupt`: 用户插话即时打断帧
  8. `StreamEnd`: 当前交互轮次收敛终结帧

### 5.2 流式分句分词器 (`SentenceDivider`)
* **标点边界切片**：在 `。`, `！`, `？`, `!`, `?`, `;`, `；`, `\n` 处精准切分语义短句；
* **超低延迟推流**：Token 流聚合为短句后立即并发推入 TTS 引擎，首包音频延迟（TTFAB）压至 **< 300ms**。

---

## 6. 跨能力安全不变量与物理防御底线

1. **Fail-Closed 默认安全原则**：任何解析异常、校验冲突或超时，系统一律按最严格安全策略拒绝，绝不默认放行。
2. **纯 Safe Rust 编译屏障**：全工作区开启 `#![deny(unsafe_code)]`，核心凭据开启 `#![forbid(unsafe_code)]`，零 C-FFI 外部黑盒。
3. **零凭据落盘 (Zero-Secret Persistence)**：API Key、Master Token 仅以 `SecretString` / `SecretBuf` 存在于内存中，离开作用域自动物理清零（Zeroize），严禁进入日志、Trace、持久化 DB 或 Prompt。
4. **不可变核心资产 0 漂移**：9 哲学锚、13 键原则、3 项不可变脊柱（Self-Disable、L0 HA、13 键 Cache）、workspace.version 1.2.0 与 R11 baseline 三值严格保持绝对锁定。
