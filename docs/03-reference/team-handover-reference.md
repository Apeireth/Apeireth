# Apeireth 2.0 团队接手一站式参考全景手册 (One-Stop Team Handover Reference Manual)

> **版本**: 2.0.0-preview  
> **更新日期**: 2026-08-30  
> **适用对象**: 核心研发团队、架构师、安全审计员与后续接手人员  
> **核心原则**: 严守 9 哲学锚 (S-1~S-3, O-1~O-6)、5 项 LOCKED 资产与 100% 纯 Safe Rust 零 unsafe

---

## 目录
1. [项目全貌与 16-Crate 依赖拓扑](#1-项目全貌与-16-crate-依赖拓扑)
2. [5 项 LOCKED 核心资产与 9 哲学锚守则](#2-5-项-locked-核心资产与-9-哲学锚守则)
3. [全域系统能力与核心 API 一览表](#3-全域系统能力与核心-api-一览表)
4. [外部 170+ 标杆与 VCP 行级吸收演进图谱](#4-外部-170-标杆与-vcp-行级吸收演进图谱)
5. [研发纪律、代码规范与“0 假装 PASS”原则](#5-研发纪律代码规范与0-假装-pass原则)
6. [一键构建、测试、代码检查与常用指令](#6-一键构建测试代码检查与常用指令)
7. [团队接手常见问题 (FAQ) 与排障指南](#7-团队接手常见问题-faq-与排障指南)

---

## 1. 项目全貌与 16-Crate 依赖拓扑

Apeireth 2.0 采用严格的**四层单向依赖微内核架构**，杜绝跨层逆向引用与循环依赖：

```mermaid
graph TD
    subgraph Layer4 [Layer 4: 适配器与网关层 (Adapters)]
        GW[apeireth-gateway: 8帧WS+分句推流+BargeIn]
        CLI[apeireth-cli: 统一启动与会话执行]
        SDK[apeireth-sdk: 跨平台安全SDK]
    end

    subgraph Layer3 [Layer 3: 工具与沙箱层 (Capabilities)]
        TOOLS[apeireth-tools-canonical: 事务Patch+Pre/Post绊线+MCP+JobObject]
    end

    subgraph Layer2 [Layer 2: 引擎与认知层 (Engine)]
        RT[apeireth-runtime: 内核+自驱心跳+FlowLock+Harness自愈]
        MEM[apeireth-memory: 5D记忆+双时态图谱+事实链+做梦+活维基知识编译]
        PROV[apeireth-provider: Anthropic/OpenAI/MiniMax多模型路由]
        PERC[apeireth-perception: MiniMax LIVE语音+3D-PAD情感+屏幕感知]
        STOR[apeireth-storage: SQLite连接池与持久化迁移]
        ORG[apeireth-organ: 9大认知器官与世界模型]
    end

    subgraph Layer1 [Layer 1: 基石与安全治理层 (Foundation)]
        CORE[apeireth-core: 9哲学锚/13键原则/三洋葱不可变脊柱]
        GOV[apeireth-governance: OWASP投毒清洗/不可信信封/PII/限流]
        CRED[apeireth-credentials: 内存零化Zeroize/Fail-closed门控]
        ORCH[apeireth-orchestration: 7Advisor辩论/发言仲裁/PromptCache]
        PROT[apeireth-protocol: 统一消息与流转协议]
        PLUG[apeireth-plugin: 插件注册与生命周期描述]
    end

    Layer4 --> Layer3
    Layer4 --> Layer2
    Layer3 --> Layer1
    Layer2 --> Layer1
```

---

## 2. 5 项 LOCKED 核心资产与 9 哲学锚守则

### 2.1 5 项 LOCKED 核心资产（严禁改动、严禁破坏）

| LOCKED 核心资产 | 源码位置 / 定义 | 保护说明 |
|---|---|---|
| **1. 9 项哲学锚本体** | `crates/foundation/core/src/eight_anchors.rs` | S-1~S-3、O-1~O-6 枚举定义严格锁定 |
| **2. 13 键 LOCKED 判别词汇表** | `crates/foundation/core/src/philosophy.rs` | `ALL_THIRTEEN_KEYS` 13 键原则判定逻辑锁定 |
| **3. 3 项不可变脊柱** | `crates/foundation/core/src/onion.rs` | Self-Disable (L0) / L0 HA (500ms) / 13 键 Verdict Cache |
| **4. workspace.version** | 根目录 `Cargo.toml` | `version = "1.2.0"` 保持不变 |
| **5. R11 baseline 3 值** | `0.8682 / 0.8532 / 0.9063` | 历史评测基准常数锁定 |

### 2.2 9 大哲学锚落地清单
1. **S-1 北极星 (North Star)**：一切以长期共生 Companion 基地为目标。
2. **S-2 实事求是 (Truth from Facts)**：杜绝空中楼阁，全部代码真实可跑。
3. **S-3 质量工程化 (Quality Engineering)**：强类型建模、Serde 支持、完备单测。
4. **O-1 安全优先 (Security First)**：Fail-Closed 默认拒绝、进程硬隔离、内存物理清零。
5. **O-2 站在前人肩膀上 (Standing on Shoulders)**：全面吸收业界前沿经验与标杆。
6. **O-3 干到底 (Follow Through)**：不留烂尾，模块完成从定义、实现到重导出与测试的完整闭环。
7. **O-4 任何人都能接手 (Maintainability)**：详尽文档、标准 `///` 注释与清晰架构。
8. **O-5 0 装 PASS (Zero Fake Pass)**：代码库中绝对零 `todo!`、零 `unimplemented!`、零伪装 mock。
9. **O-6 永远追求最优 (Pursue Optimality)**：系统最优、架构最优、算法时空复杂度最优。

---

## 3. 全域系统能力与核心 API 一览表

详细规范参见 [`docs/01-architecture/system-capabilities.md`](../01-architecture/system-capabilities.md) 与 [`docs/03-reference/capabilities-matrix.md`](capabilities-matrix.md)。

* **OWASP ASI-01 工具投毒清洗**：`ToolDescAuditor::audit(desc)`（过滤零宽/Bidi/越权词汇）
* **不可信内容信封包裹**：`UntrustedContentWrapper::wrap(source, content)`（`<<<[` 逃逸中和）
* **PII 与环境变量脱敏**：`PiiDetector::redact(text)`（8 类敏感实体脱敏）
* **发言权仲裁机**：`SpeechOutputArbiter::arbitrate(request)`（Queue / Drop / Interrupt）
* **Prompt Cache 稳定器**：`PromptCacheStabilizer::assemble_messages()`（80%+ 缓存命中率）
* **五维时空记忆**：`FiveDimensionalMemory::export_browser_entries()`（Working ~ Persona）
* **双时态事实图谱**：`BitemporalGraph::search_facts(query, now_ms)`（版本链 + 残差特异性）
* **密码事实时间线**：`ArbitrationTimeline::append_event()`（SHA-256 哈希链 + Merkle Root）
* **昼夜梦境引擎**：`DreamEngine::advance_cycle()`（6 阶段认知循环）
* **活维基知识编译**：`WikiFsEngine::run_lint()`（双链拓扑 + 死链/孤岛反熵 Lint）
* **AI 自驱心跳与心流锁**：`HeartbeatScheduler::acquire_flow_lock()`（抢占式二叉堆）
* **Harness 失败自进化修补**：`HarnessPatchEngine::synthesize_patches()`（DeepSeek R1 范式）
* **事务级多文件补丁**：`TransactionalPatchApplier::apply_patch()`（两阶段提交 + 自动原子回滚）
* **8 帧全双工 WebSocket**：`SentenceDivider` 标点流式分句（TTFAB < 300ms）

---

## 4. 外部 170+ 标杆与 VCP 行级吸收演进图谱

* **1.0 遗产与 170+ 标杆白皮书**：[`docs/01-architecture/v2-master-lineage-and-upgrade-blueprint.md`](../01-architecture/v2-master-lineage-and-upgrade-blueprint.md)
* **VCPToolBox 深度对比报告**：[`docs/01-architecture/vcp-vs-apeireth-deep-comparison.md`](../01-architecture/vcp-vs-apeireth-deep-comparison.md)
* **VCP 行级吸收指南**：[`docs/03-reference/vcp-line-level-absorption-guide.md`](vcp-line-level-absorption-guide.md)

### 核心演进路线图：
1. **第一批 (已实装)**：OWASP ASI-01 审计、不可信信封包裹、8 类 PII、Pre/Post 工具绊线、事务补丁、MiniMax LIVE 语音、8 帧 Duplex 网关、5D 记忆与双时态图谱。
2. **第二批 (已实装)**：发言仲裁机、Prompt Cache 稳定器、AI 自驱心跳与 FlowLock 心流锁、失败轨迹自愈修补、活维基知识编译与反熵治理。
3. **第三批 (规划中)**：浪潮流体动力学（LIF 脉冲传导）、Gram-Schmidt 正交残差金字塔、加权中心化 PCA 语义主轴与跨节点透明超栈文件穿透。

---

## 5. 研发纪律、代码规范与“0 假装 PASS”原则

1. **纯 Safe Rust 编译防线**：全工作区声明 `#![deny(unsafe_code)]`，凭据库声明 `#![forbid(unsafe_code)]`，绝对严禁引入未经审计的 C-FFI 或 unsafe 裸指针。
2. **0 假装通过 (Zero Fake Pass)**：严禁在生产代码中使用 `todo!()`、`unimplemented!()` 或静态硬编码的 mock 数据欺骗测试。
3. **Clippy 0 警告底线**：提交前必须确保 `cargo clippy --workspace --all-targets -- -D warnings` 为 0 警告。
4. **单向依赖规则**：上层可以依赖下层（Adapters $\to$ Capabilities/Engine $\to$ Foundation），严禁下层反向引用上层，严禁同层循环依赖。

---

## 6. 一键构建、测试、代码检查与常用指令

```powershell
# 1. 离线全量编译 (推荐)
cargo build --workspace --offline

# 2. 离线全量单元测试与集成测试
cargo test --workspace --offline

# 3. 严格 Clippy 静态代码检查 (0 警告门禁)
cargo clippy --workspace --all-targets --offline -- -D warnings

# 4. 运行特定模块测试 (以 memory 为例)
cargo test -p apeireth-memory --offline

# 5. 启动 CLI 交互模式
cargo run -p apeireth-cli --bin apeireth -- chat

# 6. 一键推送到远程仓库
git push origin main
git push origin v2.0.0-preview --force
```

---

## 7. 团队接手常见问题 (FAQ) 与排障指南

### Q1: 为什么执行 `cargo test` 时提示某些外部依赖无法下载？
* **解答**：本项目已支持完全离线构建。请务必添加 `--offline` 参数，直接利用本地 cargo 缓存。

### Q2: 为什么不可信输入包裹后变成了 `<<< [`？
* **解答**：这是 `untrusted_mark.rs` 的故意设计！外部文本若包含 `<<<[` 会导致解析器提前闭合信封，强制在中间插入空格是消解逃逸攻击的标准防御方案。

### Q3: 为什么大模型有时候不命中 Prompt Cache？
* **解答**：请检查是否在 System Prompt 或历史轮次中动态拼接了时间、电量等动态字符串。必须使用 `PromptCacheStabilizer`，将所有挥发性上下文单点注入至最新一条 User 消息顶部。

### Q4: 如何查看事实时间线是否被篡改？
* **解答**：调用 `ArbitrationTimeline::verify_integrity()`，若返回 `Ok(root_hash)` 说明哈希链与 Merkle 树 100% 吻合；若被篡改将精准指出发生断裂的序号。
