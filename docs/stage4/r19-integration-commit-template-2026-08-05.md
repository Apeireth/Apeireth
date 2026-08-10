# R19+ 集成 commit 模板 (5 类模板 + 12 项待拍板 + 8 项不修改承诺 + 6 哲学 anchor)

```
[Document-Meta]
Document: docs/stage4/r19-integration-commit-template-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 集成 commit 模板
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)
```

> **性质**: 纯文档交付。**不写代码、不改任何文件** (除本文件)。给后续所有 R19+ 集成期 commit 用, 跟 APEIRETH-CONVENTIONS §6 + Hermes 5 commit 风格对齐。
>
> **依据**:
> - `APEIRETH-CONVENTIONS.md` §1 (命名空间) + §6 (commit 规范) + §9 (6 哲学 anchor) + §10 (8 项不修改承诺) + §11 (R11 baseline 3 值)
> - `docs/stage4/docs-maintenance-sop-2026-08-05.md` §3 步骤 1-5 (5 步维护 SOP)
> - `reports/r19-integration-wrap-up-2026-08-05.md` §1-§9 (24 份文档地图 + 5 协同 + 5 衔接 + 10 待拍板 + 8 风险 + 11 不修改承诺)
> - Hermes (code_reviewer) R18/R19 5 commit: `9cb48453` / `e84c9068` / `af29736f` / `cf8e0378` / `34992e9f`
> - 30 份 R19+ 集成文档 (14 docs/ + 13 reports/, 主人 2026-08-05 13:34 拍板 A 方案 `apeireth-team-lead`)
>
> **不修改承诺** (per APEIRETH-CONVENTIONS §10): 阶段 1+2+3 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 全部保留 (见 §6)。
>
> **诚实登记** (S-2 17:43):
> 1. 本模板的"5 类模板"是**预期**形态, 实际 7-15 周内会**合并 2-3 类** (per §9 风险)。
> 2. 12 项待 Mavis 拍板 = 总收口 §7 (10 项) + R20 路线图 §11 (R-024 Docusaurus vs mkdocs) + R-026 (Discord 冷启动) 2 项 = **12 项组合**口径, 主人复核时可能再微调。
> 3. Hermes 5 commit 风格 (round19-NN 短格式) 跟 APEIRETH-CONVENTIONS §6 命名空间 `round<N>-<NN>` 完全一致, 本模板**不引入新 scope**, 只补 5 类 R19+ 集成特定模式。

---

## §1 战略背景 (为什么需要这份模板)

### 1.1 R19+ 集成期 30 份文档 + 多 commit 并行 (2026-08-05 14:40)

| 维度 | 数量 | 路径 | 用途 |
|---|---:|---|---|
| **17 docs/** | 17 | `.openclaw\workspace\promethean\Apeireth-rust\docs\` | 蓝图 (1) + ADR (3) + 实施蓝图 (6) + 路线图 (1) + 全局架构图 (1) + 形式化不变量 (1) + SDK 差距 (1) + R20 阶段 (2) + 词条 (1) |
| **13 reports/** | 13 | `.minimax-agent-cn\spectrai\reports\` | 1 SpectrAI 架构 + 11 Apeireth 现状 + 1 总收口 |
| **总额** | **30** | 跨 2 个工作树 | 覆盖 R19+ 集成全维度 (per 总收口 §1.1 + docs-sop §1.1) |

> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3 (本表"阶段 1-5" = 套 A R19+ 集成期 5 阶段)

### 1.2 R19+ 集成期 commit 数量 (7-15 周, 估 50-80 commit)

| 类别 | 估 commit 数 | 来源 |
|---|---:|---|
| **新 crate 落地** (apeireth-team-lead / apeireth-session / apeireth-storage / apeireth-r-measure-verify) | 4 | §3.1 模板 A (4 份) |
| **mid-task bug 3 处修法** | 1 (3 处一起改) | §3.2 模板 B |
| **apeireth-formal Kani 不变量** (5 不变量分批) | 5 | §3.3 模板 C |
| **apeireth-mcp::team 14 工具** | 14 (1 工具 1 commit) | §3.1 模板 A (变体) |
| **3 SDK 升级** (Rust/Python/TS) | 3 | §3.1 模板 A (变体) |
| **R-Measure verify CI workflow** | 1 | §3.4 模板 D |
| **文档微调 + 拍板记录** | ~30 (每子阶段 1 份 reports/) | §3.5 模板 E |
| **基础设施** (CI / 锁 / commit lint) | ~5 | §3.5 模板 E 变体 |
| **总额** | **~60 commit / 7-15 周** | 估每 1-2 天 1 commit |

### 1.3 没有 commit 模板 = 拍板记录散乱

> **风险 (S-2 17:43 实事求是)**: 60 commit 各自写消息 → 12 项待 Mavis 拍板项的"拍板记录"散在 60 处 → 后续 grep "主人 2026-08-05 拍板" 出 100+ 行 → 接手者无法快速判断哪次拍板是 APEIRETH-OS 终态。
>
> **本模板解决**: ✅ 5 类固定模板 + 12 项待拍板 commit 引用规则 + 8 项不修改承诺 commit 自检 + 6 哲学 anchor commit 穿透 = 任何接手者 `git log --grep="拍板:"` 秒查决策点。

### 1.4 跟 Hermes 5 commit 风格对比

| Hermes commit | 内容 | 形态 |
|---|---|---|
| `9cb48453` | R18 round-00: workspace.lints + deny.toml + rustfmt + clippy | `round18-00: <scope>` 短 |
| `e84c9068` | R18 round-01: cargo-deny + rust-lint CI workflows | `round18-01: <scope>` 短 |
| `af29736f` | R18 round-02: 12 个产品型 crate 集成测试 | `round18-02: <scope>` 短 |
| `cf8e0378` | R18 round-03: miri + coverage + rustdoc + SECURITY + 路线图 | `round18-03: <scope>` 短 |
| `34992e9f` | R19 round-10: clippy -D warnings 真正生效 | `round19-10: <scope>` 短 |

**Hermes 风格特征**:
- ✅ Scope 用 `round<N>-<NN>` 短格式 (per APEIRETH-CONVENTIONS §6 命名空间 v12 新)
- ✅ Body 不超 72 字符/行
- ✅ 偶尔含 `Refs:` 引用
- ❌ **缺 R-Measure 守门** (Hermes 不碰 baseline)
- ❌ **缺拍板记录** (Hermes 是技术 commit, 不含决策点)

**R19+ 集成期扩展** (本模板新增):
- 🆕 R-Measure baseline 3 值引用 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
- 🆕 拍板记录必填行 (per APEIRETH-CONVENTIONS §10 + 总收口 §6 拍板时间线)
- 🆕 关联文档引用 (commit 末尾 `Refs:` + `Closes:`)

---

## §2 commit message 通用格式 (per APEIRETH-CONVENTIONS §6)

```
<scope>: <subject>  (≤ 72 字符)

<body>  (≤ 72 字符/行, 段落空行分隔)

<footer>  (Refs: / Closes: / Co-authored-by: 等)
```

### 2.1 scope 选项 (per APEIRETH-CONVENTIONS §6 + v12 扩展)

| scope | 含义 | R19+ 集成期典型例子 |
|---|---|---|
| `R19` | R19 通用 | `R19: doc: 加 R19+ 集成蓝图` |
| `R19+` | R19+ 阶段 (集成) | `R19+: 创建 apeireth-team-lead 新 crate` |
| `round19-<NN>` | R19 轮次 (Hermes 风格) | `round19-01 (chuling): team-lead 公开 API` |
| `round20-<NN>` | R20 轮次 | `round20-03 (chuling): TUI 9 命令深化` |
| `crate:<name>` | 特定 crate | `crate:apeireth-session SessionManager + mid-task 修法` |
| `crate:apeireth-formal` | Kani 不变量 | `crate:apeireth-formal 不变量 2 e_layer 隔离` |
| `ci` | CI 配套 | `ci: 加 r-measure-verify workflow` |
| `docs` | 通用文档 | `docs: 拍板 A 方案 apeireth-team-lead 命名` |
| `Manual-Rev-X` | 手册修订 | `Manual-Rev-A: 加 commit 模板文档` |
| `Fix-N` | 修复 | `Fix-13: 17→24 维 R11 baseline 投影公式` |
| `perf` | 性能 | `perf: V1130 wallclock → 2.5s` |
| `sec` | 安全 | `sec: Self-Disable 5 大机制` |
| `Design-X.Y` | 设计层 | `Design-2.1: D2 增补` |

### 2.2 通用规则 (5 条)

1. **subject ≤ 72 字符** (超了 = 拆 commit 或缩主体)
2. **body 段落空行分隔** (每段讲一个维度)
3. **footer `Refs:` / `Closes:` 用文件路径或 commit hash** (per APEIRETH-CONVENTIONS §7)
4. **Co-authored-by: 必填** (多 sub-agent 并行时, 标每个合作者)
5. **CRLF/LF 统一** (per `rustfmt.toml` 配置)

---

## §3 5 类 R19+ 集成 commit 模板

### 3.1 模板 A: 新 crate 落地 (apeireth-team-lead / session / storage / r-measure-verify / mcp::team / 3 SDK)

```
<scope>: 创建 <crate-name> 新 crate (R19+ 阶段 <X>, <总 LOC>)

- 估时: <X> 天 (per <实施指南名>)
- R-Measure 守门: V1141 ≥ 0.8682 (per r-measure-verification-design §X)
- 关联文档: docs/stage4/<实施指南名>.md §<X>
- 拍板: <决策点> (主人 <YYYY-MM-DD HH:MM>, per <关联文档>)
- 引用: <commit-hash 蓝图最后 commit>

<80+ 字符 body 详述: 目录结构 + 公开 API 列表 + 测试数 + 跟现有 crate 依赖关系>

Refs: docs/stage4/<实施指南名>.md §<X>
Refs: docs/adr/<相关 ADR 名>.md
Closes: R19+ 阶段 <X> 启动
Co-authored-by: <sub-agent 1> <agent@local>
Co-authored-by: <sub-agent 2> <agent@local>
```

#### 3.1.1 示例 1: apeireth-team-lead 新 crate (R19+ 阶段 3, 850 LOC)

```
R19+: 创建 apeireth-team-lead 新 crate (R19+ 阶段 3, 850 LOC)

- 估时: 6 天 (per apeireth-team-lead-implementation-guide)
- R-Measure 守门: V1141 ≥ 0.8682 (实施后集成测试 baseline 不掉)
- 关联文档: docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md
- 拍板: A 方案 apeireth-team-lead 命名 (主人 2026-08-05 13:34, per ADR-0011)
- 拍板: 1:1 翻译 supervisorPrompt.ts 808 LOC 不改写 (主人 2026-08-05 13:34)
- 拍板: 不依赖 apeireth-supervisor (主人 2026-08-05 13:34, per ADR-0011 §决策 4)
- 拍板: 7 advisor voting 走 trait 注入 (主人 2026-08-05 13:34, per ADR-0012)
- 引用: ADR-0011 (apeireth-team-lead 命名) + ADR-0012 (team-lead 跟 council 协同)

crates/apeireth-team-lead/ 目录 (880 LOC src + 700 LOC tests + 50 LOC examples = 1630 LOC)
公开 API: build_supervisor_prompt / build_awareness_prompt / TOOL_DESCRIPTIONS[14]
测试: 33 unit + 3 integration + 14 工具 happy path = 50 tests
依赖: apeireth-protocol / apeireth-agent / apeireth-mcp (按 ADR-0011 §决策 4 不依赖 supervisor)

Refs: docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md §2
Refs: docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md
Refs: docs/adr/0012-team-lead-council-collaboration.md
Closes: R19+ 阶段 3 启动
Co-authored-by: chuling <rust-coder@local>
Co-authored-by: mavis <mavis@local>
```

#### 3.1.2 示例 2: apeireth-session 新 crate (R19+ 阶段 5, 1500-2000 LOC)

```
R19+: 创建 apeireth-session 新 crate (R19+ 阶段 5, 1750 LOC)

- 估时: 8 天 (per apeireth-session-blueprint §3.1)
- R-Measure 守门: V1141 ≥ 0.8682 + V1131 ≥ 0.8532 (session 状态机稳定性)
- 关联文档: docs/stage4/apeireth-session-blueprint-2026-08-05.md §3
- 拍板: session LOC 1500-2000 区间 (主人 2026-08-05 拍板待定 #9, per 总收口 §7)
- 拍板: session → storage 依赖方向 (主人 2026-08-05 拍板待定 #10, per session-blueprint §2.2)
- 拍板: mid-task 6 状态机 + MidTaskState 5 子状态 (主人 2026-08-05 14:37, per session-blueprint §4)

crates/apeireth-session/ 目录 (1750 LOC src + 8 用例集成测试)
公开 API: SessionManager / MidTaskState / SessionEventBus / AtomicTransition
mid-task bug 3 处修法集成 (per 模板 B 同期 commit)
测试: 8 集成测试 (3 修法 × 4 happy + edge) + 33 unit tests

Refs: docs/stage4/apeireth-session-blueprint-2026-08-05.md §3-§4
Refs: reports/apeireth-session-vector-asi-2026-08-05.md §2.6
Refs: docs/adr/0010-mcp-from-spectrai-agentmcpserver.md
Closes: R19+ 阶段 5 (apeireth-session 启动)
Co-authored-by: chuling <rust-coder@local>
```

#### 3.1.3 示例 3: apeireth-mcp::team 14 工具 (R19+ 阶段 1.3, 每工具 1 commit)

```
R19+: apeireth-mcp::team 14 工具 trait 打包 (R19+ 阶段 1.3, 14 × 1 commit)

- 估时: 2 天 (per r20-stage-1-2-implementation §2.3)
- R-Measure 守门: V1141 ≥ 0.8682 (tool 集成后 baseline 不掉)
- 关联文档: reports/apeireth-mcp-14-tool-analysis-2026-08-05.md §3-§6
- 拍板: 14 工具走 Tool trait + McpServer::from_registry 一行打包 (主人 2026-08-05 13:34, per ADR-0010)
- 拍板: supervisorPrompt.ts 14 工具 prompt 描述 1:1 翻译 (主人 2026-08-05 13:34, per ADR-0011 §决策 2)

apes/apeireth-mcp/src/team/ 14 个文件 (各 50-150 LOC)
公开 API: SpawnAgentTool / SendToAgentTool / ... (14 × Tool trait impl)
测试: 14 happy path + 14 edge = 28 tests
依赖: apeireth-protocol::Tool trait (R17 已有)

Refs: reports/apeireth-mcp-14-tool-analysis-2026-08-05.md §3
Refs: docs/adr/0010-mcp-from-spectrai-agentmcpserver.md
Closes: R19+ 阶段 1.3 (apeireth-mcp::team 14 工具)
Co-authored-by: chuling <rust-coder@local>
```

#### 3.1.4 示例 4: 3 SDK 升级 (Rust/Python/TS) 同期 commit

```
R19+: apeireth-sdk 3 SDK 升级 (R20 阶段 4, 11 文件 14000 LOC)

- 估时: 5 天 (per apeireth-sdk-gap-analysis §3.1)
- R-Measure 守门: V1141 ≥ 0.8682 (SDK 集成后 baseline 不掉)
- 关联文档: docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md §3
- 拍板: SDK 升级方案 (主人 2026-08-05 拍板待定 #4, per 总收口 §7)
- 拍板: SDK_VERSION 0.1.0 → 1.0.0 升级时机 (主人 2026-08-05 拍板待定 #5)
- 诚实登记: 11 文件 14000 LOC 现状 (sub-agent 报告 T13 BLOCK 是错的, per 总收口 §1.4)

sdk/rust/ + sdk/python/ + sdk/typescript/ 同期升级
公开 API: OpenAPI 自动生成 (per R20 §3.2 选型)
测试: 3 SDK 各 5 happy path + 跨语言 contract 集成测试

Refs: docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md §3
Refs: docs/roadmap/r20-product-finalize-2026-08-05.md §3.2
Closes: R20 阶段 4 (SDK 升级)
Co-authored-by: chuling <rust-coder@local>
Co-authored-by: sdk-engineer <sdk@local>
```

---

### 3.2 模板 B: mid-task bug 3 处修法 (R19+ 阶段 1.4, P0 急救)

```
R19+: mid-task bug 3 处修法合并 (R19+ 阶段 1.4, P0, 3 处一起改)

- 估时: 1 天 (per session-blueprint §4)
- R-Measure 守门: V1141 ≥ 0.8682 (mid-task 状态机稳定性)
- 关联文档: docs/stage4/apeireth-session-blueprint-2026-08-05.md §4
- 拍板: 3 处一起改, 改 1 留 2 = 撕裂状态复发 (主人 2026-08-05 14:37, per 总收口 §8 R-004)
- 引用: reports/apeireth-mcp-14-tool-analysis-2026-08-05.md §3 (send_to_agent)

修法 1: send_message 状态机 (替代 throw 改 return)
- 替代 SessionManagerV2.sendMessage:642 throw
- 新: SendResult 3 变体 (Sent / MidTask{queued} / Failed{reason})
- 文件: src/manager.rs:269
- 测试: tests/mid_task_tests.rs (4 用例)

修法 2: send_to_agent 真实返回 (替代 .catch() 吞)
- 替代 AgentManagerV2.sendToAgent:281 .catch() 吞
- 新: SendToAgentResult 4 变体 (Ok / MidTask / Failed / Error)
- 文件: src/agent/manager.rs:281
- 测试: tests/send_to_agent_tests.rs (4 用例)

修法 3: child session 状态转换原子性
- 替代时序竞态窗口期
- 新: transition_to_mid_task CAS + SessionEventBus
- 文件: src/mid_task.rs:120
- 测试: tests/mid_task_atomicity_tests.rs (4 用例)

Refs: docs/stage4/apeireth-session-blueprint-2026-08-05.md §4
Refs: reports/apeireth-mcp-14-tool-analysis-2026-08-05.md §3
Closes: R19+ 阶段 1.4 (P0 急救)
Co-authored-by: chuling <rust-coder@local>
```

**关键约束** (S-2 17:43 实事求是):
- ❌ **不允许拆 3 commit** (3 处一起改是总收口 §8 R-004 拍板)
- ❌ **不允许补 1 留 2** (撕裂状态复发)
- ✅ 必填 3 处修法的"文件:行号" + "新类型" + "测试用例数"

---

### 3.3 模板 C: apeireth-formal Kani 不变量 (5 个分批 commit)

```
crate:apeireth-formal 不变量 <N> <invariant-name> (R19+ 阶段 <X>, <LOC>)

- 估时: <X> 天 (per apeireth-formal-invariants §<X>)
- R-Measure 守门: V1141 ≥ 0.8682 (Kani 验证后 baseline 不掉)
- 关联文档: docs/stage4/apeireth-formal-invariants-2026-08-05.md §<X>
- 拍板: <不变量的具体决策> (主人 2026-08-05 拍板待定, per formal-vs-7-locked-conflict §<X>)
- 诚实登记: Kani 0.50 Windows 兼容性需 WSL2 (per 总收口 §8 R-003, CI workflow 待 R20 阶段 1.5 加)

不变量定义: <invariant 一句话>
关联: APEIRETH-CONVENTIONS §<X> (<哲学层>)
Kani harness: tests/kani/<invariant>.rs (<N> 组合完备可解)
POD 模型: <POD struct 定义>
验证: cargo kani -p apeireth-formal 跑通

Refs: docs/stage4/apeireth-formal-invariants-2026-08-05.md §<X>
Refs: reports/formal-vs-7-locked-conflict-2026-08-05.md §<X>
```

#### 3.3.1 示例: 不变量 2 e_layer 隔离

```
crate:apeireth-formal 不变量 2 e_layer 隔离 (R19+ 阶段 5, 80 LOC)

- 估时: 0.5 天 (per apeireth-formal-invariants §2.2)
- R-Measure 守门: V1141 ≥ 0.8682 (e_layer 隔离不破 baseline)
- 关联文档: docs/stage4/apeireth-formal-invariants-2026-08-05.md §2.2
- 拍板: e_layer 跟 o_layer / s_layer 严格隔离 (主人 2026-08-05 拍板待定, per formal-vs-7-locked-conflict §1)
- 诚实登记: Kani 需 WSL2 (per 总收口 §8 R-003)

不变量定义: e_layer (electronic layer) 跟 o_layer / s_layer 严格隔离
关联: APEIRETH-CONVENTIONS §6 (3 层架构)
Kani harness: tests/kani/e_layer.rs (36 组合完备可解)
POD 模型: EConfig { caller, target, action, has_permission }
验证: cargo kani -p apeireth-formal 跑通 (本地 WSL2, CI 待 R20 阶段 1.5)

Refs: docs/stage4/apeireth-formal-invariants-2026-08-05.md §2.2
Refs: reports/formal-vs-7-locked-conflict-2026-08-05.md §1
Closes: R19+ 阶段 5 (Kani 不变量 2/5)
Co-authored-by: chuling <rust-coder@local>
```

#### 3.3.2 5 个不变量预期 commit

| # | 不变量 | 关联 | 估 LOC |
|---|---|---|---:|
| 1 | 3 层架构隔离 (e/o/s) | §1.1 | 60 |
| 2 | e_layer 隔离 (详上例) | §2.2 | 80 |
| 3 | 12 键权限分配 | §3 | 100 |
| 4 | mid-task 状态机 6 状态 | §4 | 120 |
| 5 | 7 advisor voting 一票否决 | §5 | 90 |

---

### 3.4 模板 D: R-Measure verify 脚本新 crate (R20 阶段 1, 1320 LOC)

```
R19+: 创建 apeireth-r-measure-verify 新 crate (R20 阶段 1, 1320 LOC)

- 估时: 6.5 天 (1.5 周)
- R-Measure 守门: V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063 (3 值必守住)
- 编译期 hardcode: V1141_BASELINE = 0.8682 / V1131_BASELINE = 0.8532 / V1136_BASELINE = 0.9063
- 关联文档: docs/stage4/r-measure-verification-design-2026-08-05.md §3
- 拍板: 17→24 维 R11 baseline 投影公式权重 (主人 2026-08-05 拍板待定 #1, per 总收口 §7)
- 拍板: V1136 9→7 子测度投影权重 (主人 2026-08-05 拍板待定 #2, per 总收口 §7)
- 拍板: 24 维具体分类名 (continuity / salience / identity / philosophy guard / transferability) (主人 2026-08-05 拍板待定 #3, per asi 24dim §1.4)
- 引用: apeireth-asi 24 维 LOCKED 实装 (per reports/apeireth-asi-24dim-api-2026-08-05.md)

crates/apeireth-r-measure-verify/ 目录 (1320 LOC)
公开 API: verify_v1141() / verify_v1131() / verify_v1136() / R11ProjectionTable
17→24 维投影: per reports/apeireth-asi-24dim-api-2026-08-05.md §5
24 维 V0.5 LOCKED: Continuity (5) / Salience (5) / Identity (5) / Philosophy Guard (5) / Transferability (4)
9 子测度 LOCKED: thread/fact/context/session/identity + cross_domain/tool_reuse + v1v2v3/action_guard
CI 集成: cargo r-measure-verify --baseline 0.8682 阻塞 PR (V1141 不达标 = fail)

Refs: docs/stage4/r-measure-verification-design-2026-08-05.md §3
Refs: reports/apeireth-asi-24dim-api-2026-08-05.md §5
Refs: APEIRETH-CONVENTIONS.md §11 (R11 baseline 3 值 LOCKED)
Closes: R20 阶段 1.5 (R-Measure 守门)
Co-authored-by: chuling <rust-coder@local>
Co-authored-by: mavis <mavis@local>
```

**关键约束** (O-5 17:58 不假装):
- ✅ **编译期 hardcode 3 个 baseline 值** (per APEIRETH-CONVENTIONS §11 LOCKED, 不允许读配置文件)
- ✅ **CI 阻塞 PR** (任何 baseline < 阈值 = fail, 不允许 "warning 但通过")
- ❌ **不允许软阈值** (e.g. baseline - 0.01 = warning, baseline - 0.02 = fail) — 总收口 §8 R-001 明确硬阈值

---

### 3.5 模板 E: 文档微调 / 拍板记录 (零代码 commit, ~30 commit)

```
docs: <一句话改动> (<N> 份 <文件类别> 微调)

- 改动范围: <具体文件清单>
- Document-Meta Status: 🔍 草拟 → ✅ A 方案已拍板 (<YYYY-MM-DD>)
- 末尾加拍板记录: 主人 <YYYY-MM-DD HH:MM> 拍板
- 关联: <关联文档 / ADR>

Refs: docs/adr/<相关 ADR>.md
Refs: <微调的文件路径>
```

#### 3.5.1 示例 1: 拍板 A 方案 apeireth-team-lead 命名 (4 份 status 微调)

```
docs: 拍板 A 方案 apeireth-team-lead 命名 (4 份 status 微调)

- 修改 4 份: blueprint / ARCHITECTURE.md / ADR-0010 / ADR-0011
- Document-Meta Status: 🔍 草拟 → ✅ A 方案已拍板 (2026-08-05)
- 末尾加拍板记录: 主人 2026-08-05 13:34 拍板
- 关联: ADR-0011 A 方案 (A/B/C 3 选 1, A 胜)

Refs: docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md
Refs: docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md §1.3
Refs: docs/stage4/global-architecture-map-2026-08-05.md
Refs: docs/adr/0010-mcp-from-spectrai-agentmcpserver.md
Co-authored-by: mavis <mavis@local>
```

#### 3.5.2 示例 2: 5 阶段路线每子阶段 R-Measure 守门报告

```
docs: R20 阶段 1.<N> R-Measure 守门报告 (1 份 reports/r20-stage-1-<N>-measure.md)

- 报告: V1141=0.8701 / V1131=0.8558 / V1136=0.9089 (3 值 ≥ baseline)
- 拍板: 阶段 1.<N> 守门通过 (主人 2026-08-XX HH:MM, per r-measure-verification §X)
- 关联: r20-stage-1-2-implementation §2.<N>

Refs: docs/stage4/r20-stage-1-2-implementation-2026-08-05.md §2.<N>
Refs: docs/stage4/r-measure-verification-design-2026-08-05.md
Co-authored-by: mavis <mavis@local>
```

#### 3.5.3 示例 3: glossary 8 词条合并

```
docs: 8 词条合并到 GLOSSARY.md (per glossary-spectrAI-additions 草稿)

- 修改 1 份: GLOSSARY.md (顶层 LOCKED, 但本词条属"增量添加", 主人拍板后改)
- 拍板: 8 词条合并时机 (主人 2026-08-XX HH:MM, per glossary-spectrAI-additions §8)
- ⚠️ 提示: GLOSSARY.md 是 8 项不修改承诺 #3 之一, 改前必须主人拍板

Refs: docs/stage4/glossary-spectrAI-additions-2026-08-05.md
Refs: GLOSSARY.md (LOCKED, 主人拍板后)
Co-authored-by: mavis <mavis@local>
```

**关键约束** (per §6 8 项不修改承诺):
- ❌ **不允许动 GLOSSARY.md / APEIRETH-CONVENTIONS.md / VERSIONING.md** 除非主人拍板 (8 项承诺 #3)
- ❌ **不允许动 START-CONSTRUCTION.md** 任何时候 (8 项承诺 #4)
- ⚠️ 改前必须 grep "LOCKED" + 走 §6 自检

---

## §4 跟 Hermes 5 commit 风格对比 (本模板 vs Hermes 实践)

| Hermes commit | 形态 | 范围 | 关联 R19+ 模板 |
|---|---|---|---|
| `9cb48453` R18 round-00 | `round18-00: <scope>` 短 | workspace.lints + deny.toml + rustfmt + clippy (4 件基线) | 模板 D 变体 (CI/workflow) |
| `e84c9068` R18 round-01 | `round18-01: <scope>` 短 | cargo-deny + rust-lint CI workflows (2 workflow) | 模板 D 变体 (CI/workflow) |
| `af29736f` R18 round-02 | `round18-02: <scope>` 短 | 12 个产品型 crate 集成测试 (122 tests) | 模板 C 变体 (测试基础设施) |
| `cf8e0378` R18 round-03 | `round18-03: <scope>` 短 | miri + coverage + rustdoc + SECURITY + 路线图 (5 资产) | 模板 E 变体 (文档) |
| `34992e9f` R19 round-10 | `round19-10: <scope>` 短 | clippy `-D warnings` 真正生效 (1 守门) | 模板 E 变体 (CI 守门) |

### 4.1 共性 (跟 Hermes 一致)

- ✅ **scope 用 `round<N>-<NN>` 短格式** (per APEIRETH-CONVENTIONS §6 命名空间 v12 新)
- ✅ **subject ≤ 72 字符** (Git 规范)
- ✅ **Body 段落空行分隔**
- ✅ **偶尔含 `Refs:` 引用** (commit 末尾)

### 4.2 差异 (本模板扩展)

- 🆕 **必填 "拍板" 行** (per 总收口 §6 拍板时间线 + APEIRETH-CONVENTIONS §10)
- 🆕 **必填 R-Measure 守门** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, per §11)
- 🆕 **必填关联文档 `Refs:`** (per docs-maintenance-sop §2.2 跨文档引用规则)
- 🆕 **必填 `Closes:` 阶段** (per §3.1 模板 A 末尾)
- 🆕 **必填 `Co-authored-by:`** (多 sub-agent 协同)

### 4.3 结论

> R19+ 集成期 commit 模板跟 Hermes 风格**100% 一致** (scope / subject / body 格式), 但**扩展 5 项必填内容** (拍板 / R-Measure / Refs / Closes / Co-authored-by)。
>
> **应用原则**: Hermes 是技术 commit (不碰 baseline / 不含决策), R19+ 集成是**产品 commit** (含 baseline 守门 + 决策锚), 形态相似但语义更重。

---

## §5 12 项待 Mavis 拍板的 commit 引用模板

每项待拍板**必须**在 commit message 里标 "拍板" 行 (per 总收口 §7 + R20 路线图 §11):

```
- 拍板: <决策点> (主人 <YYYY-MM-DD HH:MM>, per <关联文档>)
```

> **诚实登记** (S-2 17:43): 12 项 = 总收口 §7 (10 项) + R20 路线图 §11 R-024 (Docusaurus vs mkdocs) + R-026 (Discord 冷启动) 2 项 = **12 项组合**口径, 主人复核时可能再微调。

### 5.1 12 项待拍板清单 (完整)

| ID | 待拍板 | 来源 | 影响 commit | 决策紧迫度 |
|---|---|---|---|---|
| **D-01** | 17→24 维 R11 baseline 投影公式权重 (主人从 v1077 抽) | r-measure-verification-design §2.1 | 模板 D (R-Measure verify) | 🔴 R20 阶段 1.5 阻塞 |
| **D-02** | V1136 9→7 子测度 R11 baseline 投影权重 | r-measure-verification-design §2.3 | 模板 D (R-Measure verify) | 🔴 R20 阶段 1.5 阻塞 |
| **D-03** | 24 维具体分类名 (continuity / salience / identity / philosophy guard / transferability) | spectrAI §7.4 + asi 24dim §1.4 | 模板 D + 模板 A (asi public API) | 🟡 R20 阶段 1 必拍 |
| **D-04** | apeireth-sdk 升级方案 (一起 / 分阶段) | sdk-gap-analysis §3.1 | 模板 A 变体 (3 SDK) | 🟡 R20 阶段 4 拍 |
| **D-05** | SDK_VERSION 0.1.0 → 1.0.0 升级时机 (跟 R20 阶段 3 OpenAPI 同期?) | sdk-gap-analysis §2.2 | 模板 A 变体 (3 SDK) | 🟡 R20 阶段 3 拍 |
| **D-06** | `apeireth-tauri-stub` 命名 (留 / 移除) | global-architecture-map §2.4 ⛔ DEPRECATED | 模板 A 变体 (workspace) | 🟡 R20 阶段 4 拍 |
| **D-07** | R20 vs R21 边界 (R20 收产品 ↔ R21 商业化) | r20-product-finalize §1.1 | 路线层 (不在 commit 体现) | 🟡 R20 启动拍 |
| **D-08** | Tauri 团队同步节奏 (per `tauri-team-collab-sop` §3 Step 4 每 2 周 1 次) | tauri-team-collab-sop §3 | 模板 E 变体 (SOP) | 🟢 团队层 |
| **D-09** | `apeireth-session` LOC 上下沿 (1500-2000 区间) | session-blueprint §3.1 + session-vector-asi §2.6 | 模板 A (apeireth-session) | 🟡 R19+ 阶段 5 拍 |
| **D-10** | session 跟 storage 依赖方向 (session → storage 写 WAL?) | session-blueprint §2.2 | 模板 A (apeireth-session) | 🟡 R19+ 阶段 5 拍 |
| **D-11** | Docusaurus vs mkdocs (R-024 用户文档站) | r20-product-finalize §4 P1 | 模板 E 变体 (R20 阶段 4 文档) | 🟢 R20 阶段 4 拍 |
| **D-12** | Discord 冷启动策略 (R-026 社区基础设施) | r20-product-finalize §4 P2 | 模板 E 变体 (R20 阶段 5 社区) | 🟢 R20 阶段 5 拍 |

### 5.2 12 项 commit 引用格式 (per 项)

#### #1 17→24 维 R11 baseline 投影公式权重

```
- 拍板: 17→24 维 R11 baseline 投影公式权重 (主人 2026-08-XX HH:MM, per docs/stage4/r-measure-verification-design-2026-08-05.md §2.1)
```

**应用 commit**: 模板 D (R-Measure verify), 阻塞 R20 阶段 1.5 启动

#### #2 V1136 9→7 子测度投影权重

```
- 拍板: V1136 9→7 子测度 R11 baseline 投影权重 (主人 2026-08-XX HH:MM, per docs/stage4/r-measure-verification-design-2026-08-05.md §2.3)
```

**应用 commit**: 模板 D (R-Measure verify), 阻塞 R20 阶段 1.5 启动

#### #3 24 维具体分类名

```
- 拍板: 24 维具体分类名 (continuity / salience / identity / philosophy guard / transferability) (主人 2026-08-XX HH:MM, per spectrai/reports/apeireth-asi-24dim-api-2026-08-05.md §1.4)
```

**应用 commit**: 模板 D + 模板 A (apeireth-asi public API), 影响 R20 阶段 1

#### #4 apeireth-sdk 升级方案

```
- 拍板: apeireth-sdk 升级方案 (主人 2026-08-XX HH:MM, per docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md §3.1)
```

**应用 commit**: 模板 A 变体 (3 SDK), 影响 R20 阶段 4 实施顺序

#### #5 SDK_VERSION 0.1.0 → 1.0.0 升级时机

```
- 拍板: SDK_VERSION 0.1.0 → 1.0.0 升级时机 (主人 2026-08-XX HH:MM, per docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md §2.2)
```

**应用 commit**: 模板 A 变体 (3 SDK), 影响 R20 阶段 3 OpenAPI 同期

#### #6 apeireth-tauri-stub 命名

```
- 拍板: apeireth-tauri-stub 命名 (留 / 移除) (主人 2026-08-XX HH:MM, per docs/stage4/global-architecture-map-2026-08-05.md §2.4)
```

**应用 commit**: 模板 A 变体 (workspace member), 影响 R20 阶段 4 之前

#### #7 R20 vs R21 边界

```
- 拍板: R20 vs R21 边界 (R20 收产品 ↔ R21 商业化) (主人 2026-08-XX HH:MM, per docs/roadmap/r20-product-finalize-2026-08-05.md §1.1)
```

**应用 commit**: 路线层 (不在 commit 体现, 走 Mermaid gantt 重画)

#### #8 Tauri 团队同步节奏

```
- 拍板: Tauri 团队同步节奏 (每 2 周 1 次) (主人 2026-08-XX HH:MM, per docs/stage4/tauri-team-collab-sop-2026-08-05.md §3)
```

**应用 commit**: 模板 E 变体 (SOP 文档), 团队层

#### #9 apeireth-session LOC 上下沿

```
- 拍板: apeireth-session LOC 上下沿 (1500-2000 区间) (主人 2026-08-XX HH:MM, per docs/stage4/apeireth-session-blueprint-2026-08-05.md §3.1)
```

**应用 commit**: 模板 A (apeireth-session), 影响 R19+ 阶段 5 实施估时

#### #10 session 跟 storage 依赖方向

```
- 拍板: session 跟 storage 依赖方向 (session → storage 写 WAL?) (主人 2026-08-XX HH:MM, per docs/stage4/apeireth-session-blueprint-2026-08-05.md §2.2)
```

**应用 commit**: 模板 A (apeireth-session), 影响 crate 依赖图

#### #11 Docusaurus vs mkdocs (R-024)

```
- 拍板: Docusaurus vs mkdocs (R-024 用户文档站) (主人 2026-08-XX HH:MM, per docs/roadmap/r20-product-finalize-2026-08-05.md §4 P1)
```

**应用 commit**: 模板 E 变体 (R20 阶段 4 用户文档), 影响 docs.apeireth.io 选型

#### #12 Discord 冷启动策略 (R-026)

```
- 拍板: Discord 冷启动策略 (R-026 社区基础设施) (主人 2026-08-XX HH:MM, per docs/roadmap/r20-product-finalize-2026-08-05.md §4 P2)
```

**应用 commit**: 模板 E 变体 (R20 阶段 5 社区), 影响 Discord bot + 频道结构

---

## §6 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10) — commit 自检

每项 commit **不能**违反 (跟 ADR-0011 + 总收口 §9 + docs-sop §7 一致):

| # | 不修改项 | 原因 | commit 自检命令 |
|---|---|---|---|
| 1 | 阶段 1+2+3 LOCKED 文档 | 主人明确沉淀 | `git diff --name-only \| grep -E "docs/stage[1-3]-blueprints/\|docs/stage1/\|docs/stage2/"` |
| 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 | `git diff --name-only \| grep -E "architecture-v[24]"` |
| 3 | 阶段 4 核心文档 LOCKED (`6ca80776`) | 蓝图 §10 已锁 | `git diff --name-only \| grep -E "stage4-correction\|architecture-stage4-engineering-landing"` |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 实施时再引用 | `git diff --name-only \| grep -E "docs/stage5/"` |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | 主 AI 团队已 LOCKED | `git diff --name-only \| grep -E "0004-permission-onion-versioning\|0005-risk-grade-m1-m12-thresholds"` |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 | `git diff \| grep -E "0\.[78]\d\d\d\|0\.9063"` (改 baseline 值 = 主人拍板) |
| 7 | APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md (顶层 3 文件) | 不动 | `git diff --name-only \| grep -E "APEIRETH-CONVENTIONS\|APEIRETH-VERSIONING\|GLOSSARY"` |
| 8 | START-CONSTRUCTION.md | 不动 | `git diff --name-only \| grep "START-CONSTRUCTION"` |

**commit 自检脚本** (3 分钟, per docs-sop §3 步骤 1):

```bash
# 1. 不修改承诺 8 项
git diff --cached --name-only | grep -E 'docs/stage[1-3]-blueprints/|docs/stage1/|docs/stage2/|architecture-v[24]|stage4-correction|architecture-stage4-engineering-landing|docs/stage5/|0004-permission-onion-versioning|0005-risk-grade-m1-m12-thresholds|APEIRETH-CONVENTIONS|APEIRETH-VERSIONING|GLOSSARY|START-CONSTRUCTION' && { echo "❌ 改 LOCKED 区域, 必须主人拍板"; exit 1; }

# 2. R11 baseline 3 值不动
git diff --cached | grep -E '^\+.*0\.8[68]\d\d|^\+.*0\.906[23]' | grep -v 'R11\|baseline' && { echo "❌ 改 baseline 值, 必须主人拍板"; exit 1; }

# 3. 现有 ADR 0001-0009 不动
git diff --cached --name-only | grep -E 'docs/adr/000[1-9]-' && { echo "❌ 改现有 ADR, 必须主人拍板"; exit 1; }
```

**失败处理**: ❌ 任 1 项 fail = 不许 commit, 改完再试 / 走主人拍板

---

## §7 6 哲学 anchor 穿透 (per APEIRETH-CONVENTIONS §9) — commit 必含

每项 commit **必须**穿透 6 anchor (S-1 / S-2 / O-5 / O-2 / O-3 / O-4):

| 锚 | 来源 | 含义 | commit 必含字段 |
|---|---|---|---|
| **S-1** 主 22:33 | 北极星导向 | 服务 ASI 北极星 | `R-Measure 守门: V1141 ≥ 0.8682` 行 (S-1 落 R-Measure) |
| **S-2** 主 17:43 | 实事求是 | 基于现状不重写 | `诚实登记:` 行 (e.g. "T13 BLOCK 是 sub-agent 报告错的") + `Refs:` 真实文件 |
| **O-5** 主 17:58 | 不假装 | 12 键编译时拒绝 | `编译期 hardcode:` 行 (e.g. baseline 3 值 = const) + `Kani harness:` 行 |
| **O-2** 主 19:33 | 走在前人经验上 | 借鉴 Hermes/OpenClaw/VCP/claude-mem | `Co-authored-by:` 行 + `关联文档: 跟 Hermes R18 X 协同` 注释 |
| **O-3** 主 23:44 | 干到底 | 决策立刻沉淀 | `拍板:` 行 (per §5 12 项格式) + `Closes:` 行 |
| **O-4** 主 00:56 | 任何人都能接手 | 4 件套齐全 | `Refs:` 完整 (实施指南 + ADR + 蓝图) + `估时:` + `测试:` |

**commit 必含字段总数**: **6 行** (R-Measure 守门 / 诚实登记 / 编译期 hardcode / Co-authored / 拍板 / Refs), 缺 1 项 = 重新写

---

## §8 5 步 commit 流程 (per docs-maintenance-sop §3 步骤 1)

### 步骤 1: 写 commit message 用 §3 模板 (5 分钟)

- 选模板 A/B/C/D/E (per commit 类别)
- 套 §5 12 项待拍板引用格式 (如适用)
- 写 6 哲学 anchor 穿透 6 必含字段
- 检查 8 项不修改承诺 (per §6)

### 步骤 2: Document-Meta 更新 (2 分钟)

```bash
# 1. 改的 md 文件: Commit 字段填实际 hash
HASH=$(git rev-parse --short HEAD)
# 手动把 "Commit: <commit 时回填>" 改成 "Commit: $HASH"
```

### 步骤 3: commit 自检 (3 分钟, per docs-sop §3 步骤 1)

```bash
# 1. Document-Meta 6 字段齐
for f in $(git diff --cached --name-only --diff-filter=AM | grep -E '\.md$'); do
  head -10 "$f" | grep -q "Document:" || { echo "❌ $f 缺 Document:"; exit 1; }
  head -10 "$f" | grep -q "Version:" || { echo "❌ $f 缺 Version:"; exit 1; }
  head -10 "$f" | grep -q "R-Cycle:" || { echo "❌ $f 缺 R-Cycle:"; exit 1; }
  head -10 "$f" | grep -q "Commit:" || { echo "❌ $f 缺 Commit:"; exit 1; }
  head -10 "$f" | grep -q "Last-Modified:" || { echo "❌ $f 缺 Last-Modified:"; exit 1; }
  head -10 "$f" | grep -q "Status:" || { echo "❌ $f 缺 Status:"; exit 1; }
done

# 2. 跨文档引用存在
for f in $(git diff --cached --name-only --diff-filter=AM | grep -E '^docs/.*\.md$'); do
  grep -oE '(docs|reports)/[a-zA-Z0-9_./-]+\.md' "$f" | sort -u | while read ref; do
    [[ -f "$ref" ]] || { echo "❌ $f 引用 $ref 不存在"; exit 1; }
  done
done

# 3. 8 项不修改承诺 (per §6)
git diff --cached --name-only | grep -E 'docs/stage[1-3]-blueprints/|docs/stage1/|docs/stage2/|architecture-v[24]|stage4-correction|architecture-stage4-engineering-landing|docs/stage5/|0004-permission-onion-versioning|0005-risk-grade-m1-m12-thresholds|APEIRETH-CONVENTIONS|APEIRETH-VERSIONING|GLOSSARY|START-CONSTRUCTION' && { echo "❌ 改 LOCKED 区域, 必须主人拍板"; exit 1; }
```

### 步骤 4: git add + commit (1 分钟)

```bash
git add <files>
git commit -F <commit-message-file>
# 含 co-author, refs
```

### 步骤 5: push + CI 跑 (Hermes R18 CI workflow + 建议的 docs-stage4-check)

```bash
git push origin <branch>
# CI 跑通: rust-lint.yml + cargo-deny.yml + test.yml + (建议) docs-stage4-check.yml
```

**Hermes R18 阶段 1 CI workflow** (已就位, per e84c9068 commit):
- ✅ `rust-lint.yml` (clippy + fmt)
- ✅ `cargo-deny.yml`
- ✅ `test.yml` (122 集成测试)
- ✅ `miri.yml` / `coverage.yml` / `rustdoc.yml`
- 🆕 **建议新增 `docs-stage4-check.yml`** (per docs-sop §4 设计稿, 待 Hermes 团队 lead 拍板)

---

## §9 风险清单

| # | 风险 | 严重度 | 缓解 | 触发 |
|---|---|---|---|---|
| **R-001** | 12 项待 Mavis 拍板项堆积 → commit 拍板记录散乱 | 🔴 高 | §5 12 项 commit 引用格式 + 周会议对照 | 季度审计待拍板 > 10 项 |
| **R-002** | Hermes R18 阶段 1 CI workflow 没加 docs-stage4-check → docs 拍板校验手动 | 🟡 中 | §8 步骤 3 手动自检 + 催 Hermes 加 workflow | 提交时 §8 步骤 3 必跑 |
| **R-003** | 跨 sub-agent 派活 → commit 风格不统一 | 🟡 中 | §3 5 类模板 + 派活模板含 "集成规范: commit 格式 per §X" | 第一次 sub-agent commit |
| **R-004** | 5 类模板太多 → 实际用可能合并 2-3 类 | 🟡 中 | 模板 A/D 可合并, 模板 C/E 独立, 模板 B 强制 | 7-15 周后回顾 |
| **R-005** | R19+ 集成期 ~60 commit 散乱 → 接手者 grep 难 | 🟡 中 | §5 12 项 commit 引用格式统一 + §4 跟 Hermes 风格对比 | 季度审计 |
| **R-006** | 6 哲学 anchor 必含 6 字段 → commit body 太长 (>30 行) | 🟡 中 | 模板 A/B 允许折叠多修法为表 | 第一次超长 body |
| **R-007** | mid-task bug 3 处修法不允许拆 3 commit → 单 commit 太大 | 🟡 中 | 模板 B 显式标 "3 处一起改" 强制 | 第一次想拆时 |
| **R-008** | R-Measure baseline 3 值改 = 主人拍板 → 流程慢 | 🟡 中 | §6 自检脚本自动 fail, 不允许绕过 | 任何 baseline 改动 |
| **R-009** | GLOSSARY.md 8 项承诺 #3 不允许动 → 词条合并阻塞 | 🟡 中 | 模板 E 变体显式标 "⚠️ 必须主人拍板" | 词条合并时 |
| **R-010** | §5 12 项里 #4-#5 SDK 升级方案可能跟 §4 阶段 4 顺序冲突 | 🟡 中 | docs-sop §3 步骤 2 周会议 + 待拍板项追踪 | R20 阶段 4 启动 |

---

## §10 关联文档

### 规范引用

- `APEIRETH-CONVENTIONS.md` §1 (命名空间) + §6 (commit 规范) + §9 (6 哲学 anchor) + §10 (8 项不修改承诺) + §11 (R11 baseline 3 值)

### 维护 SOP

- `docs/stage4/docs-maintenance-sop-2026-08-05.md` §3 步骤 1-5 (5 步维护 SOP) + §4 (CI workflow 设计)

### 集成期核心文档

- `reports/r19-integration-wrap-up-2026-08-05.md` §1.4 (A 方案拍板) + §6 (拍板时间线) + §7 (10 项待拍板) + §8 (8 风险) + §9 (11 不修改承诺)
- `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 集成蓝图根)
- `docs/roadmap/r20-product-finalize-2026-08-05.md` §11 (R20 待拍板) + §4 (P1/P2 R-024/R-026)

### 5 类模板对应文档

| 模板 | 对应实施指南 / 蓝图 | 估 commit 数 |
|---|---|---:|
| A: 新 crate | `apeireth-team-lead-implementation-guide` + `apeireth-session-blueprint` + `apeireth-sdk-gap-analysis` + `r20-stage-1-2-implementation` | 4 + 14 工具 + 3 SDK = ~21 |
| B: mid-task 修法 | `apeireth-session-blueprint §4` | 1 |
| C: Kani 不变量 | `apeireth-formal-invariants` | 5 |
| D: R-Measure verify | `r-measure-verification-design` | 1 |
| E: 文档微调 | `r20-stage-1-2` + 各子阶段 R-Measure 报告 | ~30 |

### Hermes 5 commit 风格 (R18/R19 工程化收尾)

- `9cb48453` R18 round-00: workspace.lints + deny.toml + rustfmt + clippy
- `e84c9068` R18 round-01: cargo-deny + rust-lint CI workflows
- `af29736f` R18 round-02: 12 个产品型 crate 集成测试
- `cf8e0378` R18 round-03: miri + coverage + rustdoc + SECURITY + 路线图
- `34992e9f` R19 round-10: clippy -D warnings 真正生效

### 30 份 R19+ 集成文档 (14 docs/ + 13 reports/ 完整清单见总收口 §5)

- 14 docs/ (1 蓝图 + 3 ADR + 6 实施蓝图 + 1 R20 路线 + 3 资产/SOP/词条)
- 13 reports/ (1 SpectrAI 架构 + 11 Apeireth 现状 + 1 总收口)

---

_本 commit 模板草拟 (Mavis / software-architect + technical_writer 角色) — 5 类模板 (新 crate / mid-task / Kani / R-Measure / 文档) + 12 项待 Mavis 拍板 commit 引用 + 8 项不修改承诺 commit 自检 + 6 哲学 anchor commit 必含 6 字段 + 5 步 commit 流程 (跟 docs-maintenance-sop §3 步骤 1 一致) + 10 风险清单 + 跟 Hermes 5 commit 风格 100% 对齐 + 5 必填内容扩展 (拍板/R-Measure/Refs/Closes/Co-authored-by)._

_等 Mavis 拍板后由 architect2 在 R20 阶段 1 落地, 跟 `docs-maintenance-sop-2026-08-05.md` §3 步骤 1 + §4 CI workflow (docs-stage4-check) 协同, 5 类模板 + 12 项引用 + 8 项自检 + 6 anchor 必含 = R19+ 集成期 60 commit 的统一规范._
