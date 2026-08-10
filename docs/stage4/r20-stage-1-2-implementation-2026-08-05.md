[Document-Meta]
Document: docs/stage4/r20-stage-1-2-implementation-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R20 阶段 1-2 (产品基础 + 部署基础)
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)

---

# R20 阶段 1-2 实施指南 (产品基础 + 部署基础)

> **性质**: R20 收产品 = 把 Apeireth v2.0.0-alpha (R19 工程化收尾完成) 变成"可分发 / 可部署 / 可用"的 AI 成长平台. **本指南聚焦阶段 1-2** (产品 + 部署, 4 周), 给后续 sub-agent / team lead 照着干, **不写实际代码**.
>
> **依据**:
> - `docs/roadmap/r20-product-finalize-2026-08-05.md` (R20 总路线图, 5 阶段, 7-10 周)
> - `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (R19+ 集成, A 方案已拍板 2026-08-05)
> - `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` (team-lead 实施, 850 LOC)
> - `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 守门, 3 baseline 编译期 hardcode)
> - `docs/stage4/global-architecture-map-2026-08-05.md` (1 总图 + 13 子图 Mermaid)
> - `docs/stage4/tauri-team-collab-sop-2026-08-05.md` (Tauri 团队双边界 SOP)
> - `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` (R20 阶段 0 必读)
> - `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` (TUI Step 2/3 续)
> - 10 份 reports/ (council / crate-api / graph-pipeline / mcp-14-tool / platform-modules / protocol-4-adapter / session-vector-asi / supervisor-tool-rules / spectrai-architecture / tauri-roadmap)
> - 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3 (本指南"阶段 X.Y" = 套 B R20 收产品 5 阶段子阶段)
>
> **承接**: R19 v2.0.0-alpha (41 crate, 9/9 业界标准, 2416 tests) → R20 收产品 (本指南) → R21 商业化.
>
> **不修改承诺**: 阶段 1+2+3 LOCKED 文档 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 全保留 (见 §6).

---

## §1 战略背景 (为什么)

### 1.1 R20 在 ROADMAP 哪一行

R20 = 🟡 P1 (ROADMAP.md §下一阶段建议第 3 行), 主人 2026-08-04 12:30 拍板方向: "Apeireth OS 长程 AI 成长平台对外, 含计费 + 订阅 + API 配额". Mavis 2026-08-05 跟 Mavis 拍板时**明确收产品** (商业化推到 R21, 见 r20 §11.1 拍板 A).

**R20 5 阶段总览** (per `r20-product-finalize-2026-08-05.md` §4):

| 阶段 | 焦点 | 时长 | 状态 |
|------|------|-----:|------|
| **1. 产品基础** | TUI 9 命令深化 + team-lead 公开 + mid-task bug | 1-2 周 | 🟡 **本指南** |
| **2. 部署基础** | Docker 多架构 + 离线包 + 系统包 + install | 2 周 | 🟡 **本指南** |
| 3. API 公开 | REST + WebSocket + OpenAPI | 2 周 | ⏸️ 简略 (见 §3.5) |
| 4. SDK 完善 | Python / TS / Rust | 1-2 周 | ⏸️ 简略 (见 §3.6) |
| 5. 文档 + 营销 | 4 个 docs 站 + landing + 社区 | 1-2 周 | ⏸️ 简略 (见 §3.7) |

**总时长 7-10 周** (本指南 4 周, 阶段 3-5 简略). 目标完工: 2026-09-30.

### 1.2 R20 阶段 1-2 是"产品化"关键步骤

> R19 v2.0.0-alpha = 大型基地 (41 crate 已部署, HTTP API 表面稳定, 9/9 业界标准达标)
> R20 = 给基地**装电梯 + 装大门 + 铺公路**: 电梯 (产品形态 TUI/Web/SDK), 大门 (公开 API + OpenAPI), 公路 (Docker/系统包/一键安装)
> R21 = 开始**卖门票** (计费/订阅/配额)

**阶段 1-2 装什么**:
- **电梯**: TUI 9 命令深化 + 12 新命令 + dashboard 4 视图 + V0.5 24 维度集成 (阶段 1.1)
- **大门准备**: `apeireth-team-lead` 公开 API + `apeireth-mcp::team` 14 工具 + mid-task bug 3 处修法 (阶段 1.2-1.4)
- **公路**: Docker 多架构 + 离线包 + Linux deb/rpm + macOS brew + Windows scoop + 一键 install 脚本 (阶段 2.1-2.5)

**阶段 1-2 不动**:
- ❌ 不重写 41 crate (R19 工程基线保留)
- ❌ 不动 R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, 编译期 hardcode)
- ❌ 不动 12 子规范 / 6 锚 / workspace v1.0.0

### 1.3 跟 R19+ 集成蓝图衔接

R20 阶段 1-2 = R19+ 集成蓝图的工程化落地. 衔接关系 (per `spectrAI-integration-blueprint-r19-plus-2026-08-05.md`):

| R19+ 蓝图 | R20 阶段 1-2 落地 |
|----------|------------------|
| A 方案 `apeireth-team-lead` (新 crate, 850 LOC) | 阶段 1.2: 创建 + 1:1 翻译 supervisorPrompt.ts + 14 工具 + voting trigger |
| `apeireth-mcp::team` 14 工具 (新模块) | 阶段 1.3: 14 工具 impl Tool trait + integration test |
| mid-task bug 3 处修法 (P0 必改) | 阶段 1.4: send_to_agent + get_output + wait_idle 状态机修法 |
| 5 Provider 适配 (Claude/Codex/Gemini/iFlow/OpenCode) | 阶段 1.2 顺带: 走 `apeireth-api` 4 协议 HTTP + base_url + auth_token |
| `apeireth-session` 新 crate (1500-2000 LOC) | 阶段 1.4: 一起建, 3 处修法 |
| 命名空间冲突决策 | ✅ 已拍板 (A 方案, 2026-08-05 13:34) |

### 1.4 R-Measure 守门 (每子阶段必跑)

R-Measure baseline 3 值 (per `r-measure-verification-design-2026-08-05.md` §2.2 + APEIRETH-CONVENTIONS §11):

| 指标 | 值 | 含义 |
|---|---:|---|
| **V1141-R11** | 0.8682 | IC-001 fresh 测量 (17 维 V0.5) |
| **V1131-R11** | 0.8532 | dashboard v05_total |
| **V1136-R11** | 0.9063 | 真测引擎 7 子测度 |

**每子阶段结束 = 跑 R-Measure verify 脚本** (`apeireth-r-measure-verify -- check --baseline r11`). 任何值掉 < baseline - 0.001 = fail, 阻塞 PR.

**守门触发场景** (per `r-measure-verification-design` §2.4):
- 任何 41 crate src 改动 → CI 必跑 R-Measure verify
- 3 SDK 端到端测试也算 (消费 `apeireth-api`, 影响集成质量)
- 阶段报告必含: 改动清单 + R-Measure 三值 + 端到端 PASS 证据

### 1.5 跟 R20 路线图一致性

R20 路线图 §4 给的是 5 阶段任务表 + owner + 验证. **本指南把阶段 1-2 拆成 10 子阶段 + T-xxx 任务清单**, 密度比路线图高一个数量级, 给 sub-agent 照着干.

| 维度 | R20 路线图 §4 | 本指南 |
|------|------------|--------|
| 任务粒度 | 阶段级 | T-xxx 任务级 |
| owner 建议 | 4 个 (backend/devops/fullstack/technical_writer) | 10 子阶段各自 owner |
| R-Measure 守门 | 阶段级 | **子阶段级** (10 个) |
| 风险清单 | 8 项 (R-001~R-008 + R-009) | 17 项 (R-001~R-008 继承 + R-009~R-017 新增) |
| 验收 | 阶段级 | 子阶段级 (T-xxx 验收标准) |

---

## §2 阶段 1 详细实施 (1-2 周, 5 子阶段)

> **目标**: TUI 9 命令深化 + `apeireth-team-lead` 公开 API + `apeireth-mcp::team` 14 工具 + mid-task bug 3 处修法 + 端到端集成 + R-Measure 守门.
>
> **总时长**: 9 天 (1.8 周).
> **owner 矩阵**:
> - frontend_engineer (阶段 1.1 TUI 改瘦续 Step 2/3)
> - backend_engineer2 (阶段 1.2 team-lead)
> - backend_engineer (阶段 1.3 mcp::team + 1.4 mid-task)
> - qa_engineer (阶段 1.5 集成 + 守门)

### 2.1 阶段 1.1: TUI 9 命令深化 (3 天)

> **依据**: `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` Step 2 (6 类 API 端点) + Step 3 (TUI 消费端点). R19 Step 1 改瘦走 HTTP 已 ✅, 阶段 1.1 接续.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-001** | TUI 现有 9 命令巩固 (R19 Step 1 已通, 加错误处理 + 重试 + 进度条) | 200 | 9 命令在 4 协议 LLM (OpenAI/Responses/Anthropic/Gemini) 切换不崩 | R19 Step 1 ✅ |
| **T-002** | 加 12 个新命令 (per 06 Step 3: chat / memory / organs / asi / sovereignty / agent 各 2) | 600 | 12 命令进 TUI help 列表, 每个有单元测试 | T-001 |
| **T-003** | dashboard 4 视图 (per 06 Step 3: Self / Tools / Memory / Sovereignty) | 400 | TUI 启动后默认显示 dashboard, 4 视图可切换, 数字跟 ASI 北极星一致 | T-002 + `apeireth-api` 6 V2 端点 |
| **T-004** | V0.5 24 维度集成 (V0.4 17 维 + continuity / autonomy / transferability + 子维度) | 300 | TUI `organs` 命令显示 24 维度状态条 | T-003 + `apeireth-asi::V1136Engine` |

**owner**: frontend_engineer
**守门**: V1141 ≥ 0.8682 (TUI 加 12 命令不能掉 baseline) + TUI smoke test 9+12=21/21 PASS + 4 dashboard 视图数字跟 ASI 一致

**风险**:
- TUI 已改瘦走 HTTP, 跟 backend 解耦, 加命令主要是 UI 层 + HTTP client
- V0.5 24 维度数据从 `apeireth-asi` 拉, 不要在 TUI 重算
- dashboard 数字必须**只读** `apeireth-asi` snapshot, 不缓存 (避免漂移)

**R-Measure 守门**:
```bash
# T-004 完工后跑
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告写入 reports/r20-stage-1-1-measure-2026-08-05.md
```

### 2.2 阶段 1.2: apeireth-team-lead 公开 API (2 天)

> **依据**: `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` §2-§3 (crate 骨架 + 1:1 翻译) + ADR-0011 §决策 1-5 + ADR-0012 §决策 1-4.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-101** | 创建 `crates/apeireth-team-lead/` (等 code_reviewer 完工 + Cargo.toml 加 workspace member) | 50 | `cargo build -p apeireth-team-lead` 0 error | code_reviewer 完工 |
| **T-102** | 1:1 翻译 supervisorPrompt.ts 808 LOC → `prompt.rs` + `awareness.rs` | 550 | `wc -l prompt.rs` = 808 ± 20% (按 S-2 17:43 实事求是) | T-101 |
| **T-103** | 14 工具 prompt 描述 (`TOOL_DESCRIPTIONS` const, 编译期 hardcode) | 150 | 14 工具 const 数组 14 项, 1:1 翻译 | T-102 |
| **T-104** | `NoopVotingTrigger` + `CouncilVotingTrigger` (trait 注入, 不直接调 council) | 80 | 2 impl `AdvisorVotingTrigger`, team-lead 跟 council 解耦 | T-101 + `apeireth-protocol::AdvisorVotingTrigger` |
| **T-105** | 30 unit tests + 3 integration tests (per §2.1 目录结构) | 700 | `cargo test -p apeireth-team-lead` 33/33 PASS | T-101~T-104 |

**owner**: backend_engineer2
**守门**: R-Measure V1141 ≥ 0.8682 + 33/33 tests pass + supervisorPrompt.ts 原文 diff < 5% (1:1 翻译守门)

**关键约束** (per ADR-0011 §决策 4 + 蓝图 §1.3):
- ❌ **不依赖 `apeireth-supervisor`** (避免循环依赖, 进程级 vs agent 级职责不同)
- ❌ **不依赖 `apeireth-council`** (走 trait 注入, 紧耦合 = 不可测)
- ✅ 依赖 `apeireth-protocol` (ProviderEvent, AdapterSessionConfig, AdvisorVotingTrigger trait)
- ✅ 依赖 `apeireth-agent` (Agent, AgentManager 复用)
- ✅ 依赖 `apeireth-mcp` (14 工具的 trait 定义在 mcp::team)

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 验证 supervisorPrompt.ts 1:1 翻译 (主 S-2 17:43 实事求是)
diff <(curl -s https://raw.githubusercontent.com/spectrAI-org/spectrAI/main/supervisorPrompt.ts) crates/apeireth-team-lead/src/prompt.rs | wc -l
# 报告: reports/r20-stage-1-2-measure-2026-08-05.md
```

### 2.3 阶段 1.3: apeireth-mcp::team 14 工具 (2 天)

> **依据**: `reports/apeireth-mcp-14-tool-analysis-2026-08-05.md` 14 工具定义 + 蓝图 §5.1 第 7 行 "集成位置: `apeireth-mcp::team` 通过 trait 调 team-lead" + ADR-0010.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-201** | `apeireth-mcp/src/team.rs` 模块 (骨架 + 14 工具签名) | 300 | `cargo build -p apeireth-mcp` 0 error, 14 工具 pub fn 列出 | T-105 (team-lead crate) |
| **T-202** | 14 工具 impl `Tool` trait (4 方法: name/description/input_schema/call) | 700 | 14 工具都实现 4 方法, 编译期 hardcode 不假装 | T-201 |
| **T-203** | mid-task bug 3 处修法 (send_to_agent / get_output / wait_idle) | 400 | 3 处都改, 编译期 hardcode MidTaskState 状态机 | T-201 + 阶段 1.4 状态机 |
| **T-204** | 14 工具 integration test (端到端: mcp::team → team-lead → agent → LLM) | 600 | 14 工具每个 1 happy path + 1 edge, 28/28 PASS | T-202 + T-203 |

**owner**: backend_engineer
**守门**: mid-task bug 3 处修法后跑 team-lead 集成测试 + 28/28 tests pass + `apeireth-mcp` 公开 API 跟 `apeireth-team-lead` 解耦 (走 trait)

**14 工具清单** (per `apeireth-mcp-14-tool-analysis` + team-lead §3.3):

| # | 工具名 | 功能 | 关联 mid-task bug |
|---:|--------|------|-----------------|
| 1 | `spawn_agent` | 创建子 Agent, 返 agent_id | — |
| 2 | `send_to_agent` | 给子 Agent 追加指令 | ⚠️ 修法 1+2 (throw→Result + child session 状态检查) |
| 3 | `get_output` | 拉子 Agent 最新输出 | ⚠️ 修法 2 (include_mid_task 开关 + caused_by_seq) |
| 4 | `wait_idle` | 等子 Agent 当前任务完成 | ⚠️ 修法 3 (跳过 MidTaskState::Interrupted/Merged) |
| 5 | `wait` | 等子 Agent 完全退出 | — |
| 6 | `get_status` | 查子 Agent 状态 | — |
| 7 | `list` | 列出所有子 Agent | — |
| 8 | `cancel` | 终止子 Agent | — |
| 9 | `worktree_merge` | 合并 worktree 回主分支 | — |
| 10 | `worktree_info` | 查 worktree 元数据 | — |
| 11 | `worktree_check` | 检查 worktree 冲突 | — |
| 12 | `list_sessions` | 列所有会话 | — |
| 13 | `get_summary` | 拉会话摘要 | — |
| 14 | `search_sessions` | 关键字搜会话 | — |

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# mid-task bug 修法影响 V1131 dashboard 5 Self 总值, 守门要紧
# 报告: reports/r20-stage-1-3-measure-2026-08-05.md
```

### 2.4 阶段 1.4: mid-task bug 3 处修法 (1 天)

> **依据**: 蓝图 §4 mid-task bug 真根因 (3 处组合, 必一起改) + ADR-0010 §决策 + `reports/apeireth-session-vector-asi-2026-08-05.md` session crate 现状.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-301** | `send_to_agent` 加 `mid_task_state` 状态机 (MidTaskState: Running/Idle/Interrupted/Merged) | 150 | 4 状态枚举 + 状态迁移函数, 编译期 hardcode | T-203 |
| **T-302** | `get_output` 加 `include_mid_task: bool` 开关 + `caused_by_seq: u64` 字段 | 100 | 2 新字段进 output struct, 默认 `include_mid_task=false` | T-301 |
| **T-303** | `wait_idle` 跳过 `MidTaskState::Interrupted` / `Merged` (只等 Running → Idle) | 80 | wait_idle 看到 Interrupted 直接返 `idle: false, reason: "interrupted"` | T-301 |
| **T-304** | 引入 2 个新 enum (`MidTaskState`, `AgentState`) + 1 个 struct (`AgentHandle`) | 200 | 3 类型进 `apeireth-session`, 编译期 hardcode | T-301~T-303 |

**owner**: backend_engineer (主) + backend_engineer2 (review)
**守门**: mid-task bug 集成测试 8 用例 (4 成功 + 4 失败场景) + 99/99 tests pass + R-Measure 3 值守住

**3 处修法不跳不可** (per 蓝图 §4.2 + §4.4):

| 修法 | 文件位置 | 改法 | 编译期约束 |
|------|---------|------|-----------|
| **修法 1** | session::sendMessage line 636-643 | `throw` → `Result<T, SessionError>` | sendMessage 永不 panic (Rust idiomatic) |
| **修法 2** | agent::sendToAgent line 269-286 | 加 child session 状态检查 + `.catch()` → `await` | 所有跨 session 引用都先验状态 |
| **修法 3** | agent::AgentManagerV2 状态同步 | 状态变更用 `tokio::sync::broadcast` 事件驱动, 不用轮询 | 消除 child session 状态变化到 agent 状态变化窗口期 |

**集成测试 8 用例** (per 主 O-5 17:58 不假装):

| # | 场景 | 期望 |
|---:|------|------|
| 1 | send_to_agent (子 Agent running) | `success: true`, 子 Agent 收到消息 |
| 2 | send_to_agent (子 Agent 刚 terminated) | `success: false, reason: "session_closed"`, 不 panic |
| 3 | get_output (子 Agent mid-task) | `include_mid_task=true` 返 mid-task 片段, `caused_by_seq` 字段填充 |
| 4 | get_output (子 Agent idle) | 返正常输出, `caused_by_seq=0` |
| 5 | wait_idle (子 Agent running) | 等到 idle 后返 |
| 6 | wait_idle (子 Agent interrupted) | 立即返 `idle: false, reason: "interrupted"` |
| 7 | send_to_agent (子 Agent 状态窗口期) | 状态用 `tokio::sync::watch` 跟踪, 不再撕裂 |
| 8 | 并发 send_to_agent 同一子 Agent | 消息进队列, 不丢不重 |

**R-Measure 守门**:
```bash
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 状态机改动可能影响 V1131 dashboard 5 Self 总值, 守门要紧 (per r20 §6 风险)
# 报告: reports/r20-stage-1-4-measure-2026-08-05.md
```

### 2.5 阶段 1.5: integration test + R-Measure 守门 (1 天)

> **依据**: 阶段 1.1-1.4 全部完工后的端到端 + R-Measure verify 脚本.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-401** | 端到端测试 (TUI → team-lead → mcp::team → protocol → 4 协议 LLM) | 400 | 5 端到端用例 PASS, 日志含每层调用链 | 阶段 1.1~1.4 全部完工 |
| **T-402** | R-Measure verify 脚本跑 (V1141 + V1131 + V1136 3 值全守) | 0 (跑脚本) | `cargo run -p apeireth-r-measure-verify -- check` 返 exit 0 | T-401 |
| **T-403** | 5 失败场景 (LLM 401 / 500 / mid-task / 并发 / 撤销) | 300 | 5 失败场景全部预期失败, 不 panic 不挂 | T-401 |
| **T-404** | 性能测试 (P95 < 2s, 4 协议 LLM chat 端到端) | 200 | 100 次 chat, P95 < 2000ms | T-401 |

**owner**: qa_engineer
**守门**: R-Measure baseline 3 值守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) + 99/99 tests pass (T-401 5 + T-403 5 + T-404 1 + 阶段 1.1-1.4 既有 88) + P95 < 2s

**端到端 5 用例**:

| # | 场景 | 数据流 |
|---:|------|--------|
| 1 | TUI 启动 → chat 简单问题 | TUI → HTTP /v1/chat → `apeireth-api` → 4 协议 LLM → 流式回 TUI |
| 2 | TUI 调 team → spawn 2 子 Agent | TUI → HTTP /v1/team/spawn → `apeireth-team-lead` → `apeireth-mcp::team` → 2 子 agent |
| 3 | 子 Agent 协作 → supervisor 模式 | 父 agent → send_to_agent → 2 子 agent → wait_idle → broadcast 结果 |
| 4 | mid-task bug 修法验证 | send_to_agent (子 Agent running) → 成功 + 状态机迁移 |
| 5 | dashboard 24 维度 | TUI 启动 → 拉 ASI snapshot → 4 视图切换 → 数字跟 `apeireth-asi` 一致 |

**性能测试 1 用例**:
- 100 次 `chat("hi")`, 4 协议各 25 次 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini)
- 测 P50 / P95 / P99
- 目标: P95 < 2s (含网络 + 4 协议 LLM 真接)

**R-Measure 守门** (端到端 + 三值):
```bash
# 端到端
cargo test -p apeireth-tui --test e2e_team_lead
# R-Measure 三值
cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-1-5-measure-2026-08-05.md (含三值 + 99/99 tests + P95 数据)
```

**阶段 1 完工报告**: `reports/r20-stage-1-complete-2026-08-XX.md` (per r20 §4 守门要求).

---

## §3 阶段 2 详细实施 (2 周, 5 子阶段)

> **目标**: Docker 多架构 + 离线包 + 系统包 + 一键安装 + 验证文档.
>
> **总时长**: 12 天 (2.4 周).
> **owner 矩阵**:
> - devops_engineer (主, 阶段 2.1/2.2/2.4/2.5)
> - devops_engineer2 (阶段 2.3 系统包)

### 3.1 阶段 2.1: Docker 多架构 (3 天)

> **依据**: r20 §3.2 部署形态 + R19 工程化收尾 (18 Dockerfile + docker-compose 既有).

| T-ID | 任务 | 估 LOC / 配置 | 验收 | 依赖 |
|------|------|-------------:|------|------|
| **T-501** | Dockerfile (multi-stage: `rust:slim` builder + `debian:bookworm-slim` runtime) | 80 行 | `docker build` 成功, image < 800MB | R19 18 Dockerfile 既有 |
| **T-502** | 跨架构构建 (linux/amd64 + linux/arm64) via buildx | 50 行 | `docker buildx build --platform linux/amd64,linux/arm64 .` 成功 | T-501 |
| **T-503** | buildx 配置 + GitHub Actions workflow (`.github/workflows/docker-multiarch.yml`) | 100 行 | CI 推 GHCR `ghcr.io/apeireth/api:tag` + Docker Hub | T-502 |
| **T-504** | `apeireth-tui` + `apeireth-api` + `apeireth-team-lead` 集成进 image (3 entrypoint) | 50 行 | image 启动后 3 服务可用, 端口 8080 (api) + 63721 (tui ws) | T-503 |

**owner**: devops_engineer
**守门**: `docker buildx build --platform linux/amd64,linux/arm64 .` 成功 + 2 架构 image 都能 `docker run` + R-Measure V1141 ≥ 0.8682 (image 启动后跑 verify)

**关键技术点**:
- **multi-stage**: builder 阶段编译 release, runtime 阶段只 copy 二进制 + 运行时依赖
- **cross-compile**: rustup target add aarch64-unknown-linux-gnu (host 是 x86_64 时)
- **QEMU**: CI 用 `docker/setup-qemu-action@v3` 跑 arm64 emulation
- **cache**: `docker/build-push-action@v5` 用 GitHub Actions cache 加速 (避免每次重编译)

**风险** (per r20 §8 R-003):
- 🟡 apeireth-pybridge cdylib 跨平台编译 (linux/arm64 on x86_64 host) — R18-2 已知, 阶段 2.1 PoC 先做 1 架构跑通再加 arm64
- 🟡 image 启动时间 (从 1.5GB 拉取到运行) — 用 multi-stage 减到 800MB

**R-Measure 守门**:
```bash
# image 启动后跑
docker run --rm ghcr.io/apeireth/api:tag \
  cargo run -p apeireth-r-measure-verify --release -- check --baseline r11
# 报告: reports/r20-stage-2-1-measure-2026-08-XX.md
```

### 3.2 阶段 2.2: 离线安装包 (2 天)

> **依据**: r20 §3.2 "无离线包" → "离线安装包 (含 apeireth-pybridge 1100 模块 + 依赖)".

| T-ID | 任务 | 估大小 | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-601** | `apeireth-cli` + `apeireth-tui` 二进制 (跨平台 release build) | 80 MB | linux x86_64 + arm64 + macOS arm64 + Windows x86_64 各 1 | T-504 |
| **T-602** | `apeireth-pybridge` 离线 wheel (1100 Python 模块) | 800 MB | `pip install --no-index apeireth-pybridge-*.whl` 成功 | T-601 |
| **T-603** | `apeireth-team-lead.so` cdylib (跨平台) | 20 MB | 4 平台 .so / .dylib / .dll 各 1 | T-601 |
| **T-604** | 离线包大小优化 (目标 < 200MB 核心 + 800MB extras 拆 2 包) | 100 MB core | core 包 air-gapped 环境跑通, extras 按需下载 | T-601~T-603 |

**owner**: devops_engineer
**守门**: 离线包 air-gapped 环境跑通 (`tar -xf apeireth-offline.tar.gz && ./install.sh`) + 核心包 < 200MB + R-Measure V1141 ≥ 0.8682

**离线包结构**:
```
apeireth-offline-v2.0.0.tar.gz  (~1.5GB 全量, 或拆 2 包)
├── core/                          # < 200MB, 必装
│   ├── bin/                       # apeireth-cli + apeireth-tui
│   ├── lib/                       # apeireth-team-lead.so
│   └── install.sh / install.ps1
├── extras/                        # ~1.3GB, 按需
│   ├── apeireth-pybridge/         # 1100 Python 模块
│   ├── models/                    # 嵌入模型 (R-Measure 跑本地测度用)
│   └── README.md                  # extras 安装说明
└── sha256sums.txt
```

**风险** (per r20 §8 R-004):
- 🟡 离线包大小 1.5-2GB — 评估拆 core/extras 2 包, 用户按需下载
- 🟡 apeireth-pybridge 1100 模块跨平台 wheel — pip wheel --platform manylinux2014_x86_64 等多 tag

**R-Measure 守门**:
```bash
# 离线环境
tar -xf apeireth-offline-v2.0.0.tar.gz
cd apeireth-offline && ./install.sh
apeireth-cli verify  # 内部跑 R-Measure verify
# 报告: reports/r20-stage-2-2-measure-2026-08-XX.md
```

### 3.3 阶段 2.3: 系统包 (3 天)

> **依据**: r20 §3.2 + r20 §8 R-013 签名风险.

| T-ID | 任务 | 估 LOC / 配置 | 验收 | 依赖 |
|------|------|-------------:|------|------|
| **T-701** | Linux deb 包 (debian + ubuntu) via `cargo-deb` | 200 行 config | `apt install apeireth` 成功 + `systemctl start apeireth` | T-601 |
| **T-702** | Linux rpm 包 (fedora + RHEL) via `cargo-rpm` | 200 行 config | `dnf install apeireth` 成功 + `systemctl start apeireth` | T-601 |
| **T-703** | macOS Homebrew formula (`apeireth/tap/apeireth.rb`) | 100 行 | `brew install apeireth/tap/apeireth` 成功 + `brew services start apeireth` | T-601 |
| **T-704** | Windows scoop manifest (`scoop bucket add apeireth`) | 100 行 | `scoop install apeireth` 成功 (MSI 可选) | T-601 |
| **T-705** | 系统服务 (systemd unit / launchd plist / Windows Service) | 400 行 | 3 平台服务都能 start/stop/enable | T-701~T-704 |

**owner**: devops_engineer + devops_engineer2
**守门**: 各系统包安装 / 卸载 / 升级跑通 + 服务能 start/stop + R-Measure V1141 ≥ 0.8682

**系统服务配置**:

| 平台 | 服务管理器 | 配置路径 | 关键字段 |
|------|---------|---------|---------|
| **Linux** | systemd | `/etc/systemd/system/apeireth.service` | `ExecStart=/usr/bin/apeireth-api`, `Restart=always`, `User=apeireth` |
| **macOS** | launchd | `/Library/LaunchDaemons/io.apeireth.api.plist` | `ProgramArguments`, `RunAtLoad=true`, `KeepAlive=true` |
| **Windows** | SCM | `apeireth.exe install` (via `windows-service` crate) | `DisplayName=Apeireth API`, `StartType=Auto` |

**风险** (per r20 §8 R-013):
- 🔴 系统包签名: Apple notarization (`xcrun notarytool`) / Windows signing (`signtool`) / GPG (Linux)
- 🟡 跨发行版兼容: debian 12 / ubuntu 22.04+ / fedora 39+ / RHEL 9+ — CI matrix 测 4 组合

**R-Measure 守门**:
```bash
# 4 系统包跑 verify (Linux x86_64 + macOS arm64 + Windows x86_64 + Linux arm64)
# CI matrix job
# 报告: reports/r20-stage-2-3-measure-2026-08-XX.md
```

### 3.4 阶段 2.4: 一键安装脚本 (2 天)

> **依据**: r20 §3.2 + r20 §2 目标 1 "一行命令安装: `curl install.apeireth.io | sh`".

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-801** | `install.sh` (Linux/macOS bash) | 400 | `curl install.apeireth.io \| sh` 跑通 (OS/arch 检测 + 依赖引导 + API key 配置) | T-601~T-604 |
| **T-802** | `install.ps1` (Windows PowerShell 7+) | 350 | `iwr install.apeireth.io \| iex` 跑通 | T-601~T-604 |
| **T-803** | `install.rs` (Rust 实现, 跨平台, 单 binary) | 600 | `curl -L install.apeireth.io/install-rs \| sh` 拉 `install-rs` binary, 跨平台 | T-801 + T-802 |
| **T-804** | 安装日志 + 失败恢复 (rollback 机制) | 200 | 安装失败自动 rollback, 日志写 `~/.apeireth/install.log` | T-801~T-803 |

**owner**: devops_engineer
**守门**: 4 平台一键安装跑通 (Linux x86_64 + macOS arm64 + Windows x86_64 + Linux arm64) + R-Measure V1141 ≥ 0.8682 (安装后跑 verify)

**install.sh 关键步骤** (per r20 §2 目标 1):

```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. OS/arch 检测
OS=$(uname -s | tr '[:upper:]' '[:lower:]')  # linux / darwin
ARCH=$(uname -m)                              # x86_64 / aarch64

# 2. 依赖引导 (curl / git / python3)
for dep in curl git python3; do
  command -v $dep >/dev/null || { echo "Missing: $dep"; exit 1; }
done

# 3. 选安装方式 (deb / rpm / homebrew / scoop / 二进制)
case "$OS" in
  linux)  [ -f /etc/debian_version ] && INSTALL_CMD="apt install" || INSTALL_CMD="dnf install" ;;
  darwin) INSTALL_CMD="brew install" ;;
esac

# 4. 引导 API key 配置 (interactive)
read -p "Enter your LLM API key: " APEIRETH_API_KEY
mkdir -p ~/.config/apeireth
echo "APEIRETH_API_KEY=$APEIRETH_API_KEY" > ~/.config/apeireth/env

# 5. 安装 + 启动服务
curl -fsSL "https://github.com/apeireth/apeireth/releases/latest/download/apeireth-$OS-$ARCH.tar.gz" | tar -xz -C /tmp
sudo cp /tmp/apeireth/bin/* /usr/local/bin/
sudo systemctl start apeireth || brew services start apeireth

# 6. 验证
apeireth --version
curl -s http://localhost:8080/health
```

**风险** (per r20 §8 R-014):
- 🔴 一键安装脚本跨平台兼容性: Windows path 反斜杠 / macOS brew 路径差异 / Linux apt vs dnf
- 🟡 API key 配置: 不存明文到磁盘, 用 `keyring` crate (Linux Secret Service / macOS Keychain / Windows Credential Manager)

**R-Measure 守门**:
```bash
# 4 平台 install 后跑 (CI matrix 跑, 详见阶段 2.5)
apeireth-cli verify
# 报告: reports/r20-stage-2-4-measure-2026-08-XX.md
```

### 3.5 阶段 2.5: 安装验证 + 文档 (2 天)

> **依据**: r20 §3.2 + r20 §8 R-015 升级路径 + R-016 卸载彻底性.

| T-ID | 任务 | 估 LOC | 验收 | 依赖 |
|------|------|------:|------|------|
| **T-901** | CI matrix 跑 4 平台安装 (Linux x86_64 + macOS arm64 + Windows x86_64 + Linux arm64) | 200 行 yml | 4 平台 CI job 全绿 | T-801~T-804 |
| **T-902** | 安装文档 (5 分钟快速开始, `docs/user/quickstart.md`) | 300 | 5 分钟走通: 安装 → 配 API key → 启动 → 跑 TUI | T-901 |
| **T-903** | 升级文档 (R17→R20 升级路径, `docs/user/upgrade.md`) | 200 | R17 用户跑 1 命令升级到 R20, 数据/配置不丢 | T-902 |
| **T-904** | 卸载文档 (`docs/user/uninstall.md`) | 150 | 卸载彻底: 不残留配置 / 缓存 / log / systemd unit | T-902 |

**owner**: devops_engineer + technical_writer
**守门**: 4 平台 CI 全绿 + R-Measure 3 值守 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, 部署不影响 ASI) + 5 分钟 quick start 跑通

**CI matrix** (`.github/workflows/install-matrix.yml`):

```yaml
name: Install Matrix
on: [push, pull_request]
jobs:
  install:
    strategy:
      matrix:
        os: [ubuntu-22.04, ubuntu-24.04-arm, macos-14, windows-2022]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Run install
        run: curl -fsSL install.apeireth.io | sh
      - name: Verify
        run: apeireth-cli verify
      - name: Uninstall
        run: apeireth-cli uninstall
      - name: Verify clean uninstall
        run: test ! -d ~/.apeireth
```

**升级路径** (R17→R20):
- 配置文件位置不变: `~/.config/apeireth/env` (API key) + `~/.config/apeireth/config.toml`
- 数据位置不变: `~/.local/share/apeireth/` (memory / organs / sessions)
- 升级命令: `apeireth-cli upgrade` (检测旧版本 + 自动备份 + 替换二进制 + 保留配置)
- 回滚: `apeireth-cli rollback` (24 小时内可回滚)

**风险** (per r20 §8 R-015 + R-016):
- 🟡 升级路径: R17→R20 config schema 可能变, 升级脚本要兼容旧 schema (fail → 提示用户手动迁移)
- 🟡 卸载彻底性: 不残留 systemd unit / launchd plist / Windows Service / log / cache

**R-Measure 守门** (CI matrix 跑 3 值):
```bash
# CI matrix 4 平台 × R-Measure 3 值 = 12 次 verify
# 报告: reports/r20-stage-2-5-measure-2026-08-XX.md
```

**阶段 2 完工报告**: `reports/r20-stage-2-complete-2026-08-XX.md`.

---

## §3.5 阶段 3 简略 (API 公开, 2 周, 不在本指南范围)

> 详情见 r20 §4 阶段 3 + 蓝图 §5. 简略列任务:

| 任务 | owner | 时长 | 关键 |
|------|-------|-----:|------|
| HTTP REST wrapper (4 协议 + 6 V2 = 10 端点) | backend_engineer | 5 天 | `/v1/openai/chat` 等 10 端点 |
| WebSocket 双向流 (`/v1/stream`) | backend_engineer2 | 3 天 | ProviderEvent 流公开 |
| OpenAPI 3.1 规范 (`openapi.yaml`) | technical_writer | 2 天 | swagger-cli 校验 + Redoc 渲染 |
| API 认证 (API key + 基础 rate limit) | security_reviewer | 2 天 | 401 / 429 测试 |
| R-Measure 守门 | verifier | 0.5 天 | 3 值守 |

**总 12 天 = 2.4 周**, 紧跟阶段 2 之后 (或并行).

---

## §3.6 阶段 4 简略 (SDK 完善, 1-2 周, 不在本指南范围)

> 详情见 r20 §4 阶段 4 + `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md`. 简略列任务:

| 任务 | owner | 时长 | 关键 |
|------|-------|-----:|------|
| `apeireth-sdk` src/client.rs + http.rs + ws.rs 补全 (T13 CONCERN BLOCK) | fullstack_engineer | 3 天 | 11 方法签名 (chat / memory_* / organs_* / team_* / asi_* / sovereignty_* / agent_* / stream) |
| Python SDK (`pip install apeireth`) | fullstack_engineer | 2 天 | `ApeirethClient` class (httpx async) |
| TypeScript SDK (`npm install @apeireth/sdk`) | fullstack_engineer | 2 天 | napi-rs 桥 + openapi-typescript |
| Rust SDK (复用 `apeireth-sdk` crate) | fullstack_engineer | 1 天 | `ApeirethClient` struct |
| SDK 文档 + 3 语言示例 (each ≥ 5 个) | technical_writer | 2 天 | `docs/sdk/{python,typescript,rust}/` |
| 3 SDK 端到端 5 用例验证 | qa_engineer | 1 天 | 同输入 → 同输出 |
| R-Measure 守门 | verifier | 0.5 天 | 3 值守 |

**总 11.5 天 = 2.3 周**, 跟 R20 §11.3 拍板 A 串行 (Python 先 → TS → Rust).

---

## §3.7 阶段 5 简略 (文档 + 营销, 1-2 周, 不在本指南范围)

> 详情见 r20 §4 阶段 5. 简略列任务:

| 任务 | owner | 时长 | 关键 |
|------|-------|-----:|------|
| 用户文档站 (`docs.apeireth.io`, Docusaurus) | technical_writer | 3 天 | Quick Start / Tutorial / Reference / FAQ |
| 开发者文档站 (`dev.apeireth.io`) | technical_writer | 2 天 | Architecture / API (OpenAPI 自动渲染) / SDK / Extension / Self-host |
| Landing page (`apeireth.io`) | frontend_engineer | 2 天 | 价值主张 + demo gif + CTA + Changelog |
| 社区基础设施 (Discord 主选) | community_manager | 1 天 | Discord 邀请 + 频道分类 + bot 接入 |
| R-Measure 守门 | verifier | 0.5 天 | 3 值守 |

**总 8.5 天 = 1.7 周**.

---

## §4 R-Measure 守门点 (10 子阶段)

> **依据**: `r-measure-verification-design-2026-08-05.md` §2.2 (3 baseline 编译期 hardcode) + §3.5 (CLI `check` 子命令) + APEIRETH-CONVENTIONS §11.

每子阶段结束必跑 `cargo run -p apeireth-r-measure-verify --release -- check --baseline r11`, 报告路径 `reports/r20-stage-<N>-<M>-measure-<date>.md`.

| 子阶段 | 必跑值 | 容忍度 | 报告路径 | 触发场景 |
|--------|------|------:|---------|---------|
| **1.1** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-1-1-measure-<date>.md` | TUI 加 12 命令 + 4 dashboard 视图 + 24 维度集成 |
| **1.2** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-1-2-measure-<date>.md` | `apeireth-team-lead` 公开 API 验证 |
| **1.3** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-1-3-measure-<date>.md` | 14 工具 4 协议 LLM 集成 |
| **1.4** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-1-4-measure-<date>.md` | mid-task bug 修法后状态机稳定性 (V1131 风险高) |
| **1.5** | **V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063** | ±0.001 | `reports/r20-stage-1-5-measure-<date>.md` | 端到端 3 值全守 (阶段 1 完工) |
| **2.1** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-2-1-measure-<date>.md` | Docker image 启动后跑 verify |
| **2.2** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-2-2-measure-<date>.md` | 离线包 air-gapped 跑 verify |
| **2.3** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-2-3-measure-<date>.md` | 4 系统包跑 verify |
| **2.4** | V1141 ≥ 0.8682 | ±0.001 | `reports/r20-stage-2-4-measure-<date>.md` | 一键安装后跑 verify |
| **2.5** | **V1141 ≥ 0.8682 + V1131 ≥ 0.8532 + V1136 ≥ 0.9063** | ±0.001 | `reports/r20-stage-2-5-measure-<date>.md` | CI matrix 4 平台 × 3 值 = 12 次 verify (阶段 2 完工) |

**守门失败处理** (per `r-measure-verification-design` §4.4 + 主 17:58 不假装):
```
CI fail →
  ↓
PR 阻塞, 不允许 merge
  ↓
开发者本地: cargo run -p apeireth-r-measure-verify -- diff --before <last-green> --after HEAD
  ↓
看哪个 metric fail (v1141 / v1131 / v1136) + diff 多大
  ↓
判断:
  ① 真掉了 → 回滚或修代码, 重新跑 CI
  ② baseline 真的该升 → 写 ADR-00XX-baseline-bump, 主人拍板后才能改 fixtures/r11-baseline.json
  ❌ 绝不绕过 (主 17:58)
```

**关键约束**:
- 容忍度 `0.001` 编译期 hardcode (per `r-measure-verification-design` §3.3)
- 任何值掉 < baseline - 0.001 = fail, 阻塞 PR
- baseline 数值 hardcode, 不允许从 fixture 改 (per 主 17:58 不假装)

---

## §5 风险清单 (10 项, 继承 R20 §8 + 9 项新增)

> **继承**: R-001~R-008 + R-009 (per r20 §8). **新增**: R-010~R-017 (本指南补).

| # | 风险 | 严重度 | 阶段 | 缓解 |
|---|------|-------|------|------|
| **R-001** | R20 工作量 vs 1-2 周/阶段 (总 7-10 周). 实际 5 阶段串行依赖, 任何一阶段延期 → 全盘推 | 🟡 中 | 全阶段 | 阶段 1-2 可并行 (产品+部署), 阶段 3-4 严格串行. 主人周会看进度 (per r20 §8 R-001) |
| **R-002** | Tauri 2 团队进度不在 R20 范围. R20 阶段 1 TUI 深化若发现需要等 Tauri 团队反馈 → 阻塞 | 🟡 中 | 阶段 1.1 | 跟 Tauri 团队**周会同步**, 不假设他们交付 (per r20 §11.2 拍板 A) |
| **R-003** | Docker image 多架构构建复杂度. buildx + apeireth-pybridge cdylib 跨平台编译 (linux/arm64 on x86_64 host) | 🔴 高 | 阶段 2.1 | PoC 先做 1 架构 (linux/amd64) 跑通, 再加 arm64. CI 用 QEMU emulation (per r20 §8 R-003) |
| **R-004** | 离线包大小. apeireth-pybridge 1100 Python 模块 + 5 协议 LLM 抽象依赖, 预估 1.5-2GB | 🟡 中 | 阶段 2.2 | 拆 core (200MB) + extras (1.3GB) 2 包, 用户按需下载 (per r20 §8 R-004) |
| **R-005** | OpenAPI 规范跟 4 协议 LLM 抽象的同步. 协议字段变了 OpenAPI 没更新 → SDK 用户调用失败 | 🔴 高 | 阶段 3 | CI 加 `openapi-spec-validator` + `swagger-cli validate`, 4 协议字段从 `apeireth-protocol` 自动生成 OpenAPI schema (per r20 §8 R-005) |
| **R-006** | SDK 多语言维护成本. Python/TS/Rust 3 SDK × 10+ 端点 = 30+ 方法签名同步 | 🔴 高 | 阶段 4 | SDK 方法签名从 OpenAPI 自动生成 (openapi-generator), 3 语言共用同一 schema. 手写只写高层 wrapper (per r20 §8 R-006) |
| **R-007** | API 公开后安全攻击面. 内部 41 crate 库边界 → 公开 HTTP 端点, 未授权访问/注入 | 🟡 中 | 阶段 3 | API key 认证 + rate limit + security_reviewer 阶段 3-4 渗透测试. 阶段 5 文档站加 SECURITY.md 链接 (per r20 §8 R-007) |
| **R-008** | R20 商业化前提 (计费/订阅/配额) 误启动. 主人 2026-08-04 12:30 提到"含计费+订阅+API 配额", R20 范围 vs R21 范围混淆 | 🟡 中 | 全阶段 | **本路线图明确 R20 = 收产品, R21 = 商业化**. Mavis 拍板时跟主人确认 (per r20 §11.1 拍板 A) |
| **R-009** | `apeireth-sdk` C-ABI 边界. 现有 11 文件 ~14000 LOC low-level FFI 是测试入口, R20 阶段 4 加 client/http/ws 要保证向后兼容, C-ABI 边界不能破坏 | 🔴 高 | 阶段 1.2 (team-lead 不碰 sdk, 但要预留接口) + 阶段 4 | ① `apeireth-sdk/src/abi.rs` 3 个 `#[no_mangle] extern "C"` stub 不能改 ② R20 加的 client/http/ws 走**内部 API**, 不暴露新 C-ABI ③ 3 SDK 端到端 5 用例 + R-Measure 守门 ④ `src/version.rs` SDK_VERSION 升 0.1.0 → 1.0.0 跟 R20 阶段 3 同周期 (per r20 §8 R-009) |
| **R-010** | **apeireth-pybridge cdylib 编译** (R18-2 已知, pyo3 + rlib 冲突) | 🔴 高 | 阶段 2.1/2.2 | R18-2 已解决 (cdylib 单 crate, 不用 rlib), 阶段 2.1 PoC 先验证 (per r20 §3.2 关键约束) |
| **R-011** | **Docker image 跨架构** (linux/amd64 + linux/arm64) | 🔴 高 | 阶段 2.1 | 用 `docker/setup-qemu-action@v3` 跑 arm64 emulation + `docker/build-push-action@v5` cache (per r20 §8 R-003 强化) |
| **R-012** | **离线包大小** (目标 < 200MB core, apeireth-pybridge 1100 模块是大头) | 🟡 中 | 阶段 2.2 | 拆 core/extras 2 包, 关键模块用 `manylinux2014` wheel 跨平台 (per r20 §8 R-004 强化) |
| **R-013** | **系统包签名** (Apple notarization / Windows signing / GPG) | 🔴 高 | 阶段 2.3 | ① Apple: `xcrun notarytool submit --wait` ② Windows: `signtool sign /fd SHA256` + `signtool verify` ③ GPG: `gpg --armor --detach-sig` (per r20 §8 R-013 强化) |
| **R-014** | **一键安装脚本跨平台兼容性** (Windows path / macOS brew / Linux apt) | 🔴 高 | 阶段 2.4 | 用 `install.rs` Rust 跨平台 + 平台分流 `case "$OS"` (per §3.4 + r20 §8 R-014 强化) |
| **R-015** | **升级路径** (R17 → R20 升级) | 🟡 中 | 阶段 2.5 | config schema 兼容旧版, `apeireth-cli upgrade` 自动备份 + 替换 + 保留配置. 24h 内可 `rollback` (per r20 §8 R-015 强化) |
| **R-016** | **卸载彻底性** (不残留配置 / 缓存 / log) | 🟡 中 | 阶段 2.5 | uninstall 脚本清 systemd unit / launchd plist / Windows Service / `~/.config/apeireth/` / `~/.local/share/apeireth/` / `/var/log/apeireth/` (per r20 §8 R-016 强化) |
| **R-017** | **R-Measure verify 脚本首次跑通过** (per `r-measure-verification-design` §3) | 🟡 中 | 阶段 1.5 (首次跑) | 编译期 hardcode 3 baseline + tolerance 0.001 + `load_r11_baseline()` assert JSON 等于 const. 任何字段改了 = 启动 fail (主 17:58 不假装) |

**风险总数**: 17 项 (8 继承 + 1 apeireth-sdk + 8 新增). 严重度分布: 🔴 高 7 项 / 🟡 中 10 项.

---

## §6 8 项不修改承诺

> **依据**: `r20-product-finalize-2026-08-05.md` §7 + `APEIRETH-CONVENTIONS.md` §10 + ADR-0011.

| # | 不修改项 | 原因 | R20 阶段 1-2 落点 |
|---|---------|------|----------------|
| 1 | 阶段 1+2+3 LOCKED 文档 | 主人明确沉淀 | R20 阶段 1-2 写的新指南**只加 R20 元信息, 不改 LOCKED 内容** |
| 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 | 哲学 anchor 引用 v2/v4/v4.1 时**只读不写** |
| 3 | 阶段 4 主文档 LOCKED (6ca80776) | 落实架构定稿 | R20 阶段 1-2 引用时不改 commit hash |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 施工蓝图定稿 | 同上 |
| 5 | v6 基础架构 | 4 重守门 + 权限发放 + E 层修改路径 | R20 阶段 1-2 守 4 重守门 (cargo-fmt / clippy / cargo-deny / **R-Measure verify**) |
| 6 | **R11 baseline 三值** | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 | 编译期 hardcode 在 `apeireth-r-measure-verify/src/baseline.rs`, tolerance 0.001, 不允许从 fixture 改 |
| 7 | **APEIRETH-CONVENTIONS.md** / **VERSIONING.md** / **GLOSSARY.md** | 12 子规范系统, R20 期间**只加 R20 元信息, 不改内容** | 本指南遵守 Document-Meta 格式, 不改 12 子规范原文 |
| 8 | **workspace version 1.0.0** (Cargo.toml) | semver 严格, R20 是产品功能**不变 major** (1.x.x 系列递增) | R20 阶段 1-2 任何新 crate 起步 0.1.0, 跟随 workspace 1.x.x 增量 |

> 8 项详见 docs/stage4/8-locked-unified-2026-08-05.md §2 (本指南统一版)

**R20 阶段 1-2 新增允许**:
- `crates/apeireth-team-lead/` (阶段 1.2 新建)
- `crates/apeireth-session/` (阶段 1.4 新建, mid-task bug 修法)
- `crates/apeireth-r-measure-verify/` (阶段 1.5 R-Measure 守门脚本, per `r-measure-verification-design`)
- `crates/apeireth-mcp/src/team.rs` (阶段 1.3 14 工具模块)
- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` (本文件)
- `reports/r20-stage-1-1~2-5-measure-<date>.md` (10 子阶段守门报告)
- `reports/r20-stage-{1,2}-complete-<date>.md` (阶段 1-2 完工报告)
- `docs/user/{quickstart,upgrade,uninstall}.md` (阶段 2.5 用户文档)
- Docker / 系统包 / install 脚本 (阶段 2.1-2.4, 部署资产)

---

## §7 6 哲学 anchor 穿透

> **依据**: `APEIRETH-CONVENTIONS.md` §9 主哲学 6 锚穿透系统 + r20 §9.

| 锚 | 来源 | 本指南落地 | 穿透检查 |
|----|------|----------|---------|
| **S-1** (主 22:33) | 北极星导向 — 服务 ASI 北极星 | R20 阶段 1-2 = ASI 完整性的工程化. TUI dashboard 4 视图 + V0.5 24 维度集成 + 系统包让用户能看到 AI 成长 (r20 §1.1 比喻) | ✅ 阶段 1.1 T-004 + 阶段 2.3-2.4 安装后跑 R-Measure |
| **S-2** (主 17:43) | 实事求是 — 基于现状不重写 | 阶段 1.2 1:1 翻译 supervisorPrompt.ts (808 LOC 严格保留) + 阶段 2.1 复用 R19 18 Dockerfile + 阶段 2.3 复用 `cargo-deb/rpm/wix` + 阶段 1.4 复用 41 crate 不重写 session | ✅ 阶段 1.2 T-102 (1:1 翻译守门) + 阶段 2.x 全部"复用优先" |
| **O-5** (主 17:58) | 不假装 — 12 键编译时拒绝 | R20 守门: R-Measure 3 值 ≥ baseline (编译期 hardcode + tolerance 0.001, §4). 阶段报告**不假装 "全部完工"**, 标 🔴/🟡/🟢 真实状态. baseline 任何字段改了 = 启动 fail | ✅ 阶段 1-2 每个子阶段结束跑 verify, 失败阻塞 PR |
| **O-2** (主 19:33) | 走在前人经验上 — 借鉴 | 阶段 2.1 借鉴 `docker/build-push-action` + QEMU emulation, 阶段 2.2 借鉴 `pip wheel --platform manylinux`, 阶段 2.3 借鉴 `cargo-deb/rpm/wix` + Homebrew + scoop, 阶段 2.4 借鉴 `keyring` crate (跨平台 secret), 阶段 1.1 借鉴 Docusaurus / openapi-typescript (阶段 3-5 用) | ✅ 阶段 2 全部"业界共识工具链" (per r20 §9 O-2) |
| **O-3** (主 23:44) | 干到底 — 决策立刻沉淀 | R20 决策沉淀: 本文件 (10 子阶段 × T-xxx) + 10 子阶段 measure 报告 + 2 阶段 complete 报告. 不停留在"讨论" (per r20 §9 O-3) | ✅ 本指南就是决策沉淀, 10 子阶段 × T-xxx 密集 |
| **O-4** (主 00:56) | 任何人都能接手 — 4 件套齐全 | R20 阶段 1-2 文档: 10 子阶段 × (任务 + 验证 + owner + 依赖) + 风险 (17 项) + R-Measure 守门 (10 子阶段) + 不修改承诺 (8 项) + 关联文档 (§8). 接手者能查 (per r20 §9 O-4) | ✅ §2 + §3 + §4 + §5 + §6 + §8 全齐 |

**穿透检查清单** (APEIRETH-CONVENTIONS §9):
- [x] S-1: R20 阶段 1-2 服务 ASI 北极星 (TUI dashboard + 系统包让用户看见)
- [x] S-2: R20 不重写, 复用 R19 工程基线 (1:1 翻译 + 18 Dockerfile + cargo-deb 等)
- [x] O-5: R20 守门 R-Measure 3 值, 阶段报告真实标
- [x] O-2: R20 借鉴业界工具链 (buildx / manylinux / cargo-deb / keyring 等)
- [x] O-3: R20 决策立刻沉淀 (本文件 + 10 守门报告 + 2 阶段报告)
- [x] O-4: R20 4 件套齐全 (任务/验证/owner/依赖 + 风险 + 守门 + 承诺 + 关联)

---

## §8 关联文档

### 8.1 必读 (本指南依据)

| 文档 | 角色 | 路径 |
|------|------|------|
| **R20 收产品路线图** | 5 阶段总览 + 守门 + 风险 + 拍板 | `docs/roadmap/r20-product-finalize-2026-08-05.md` |
| **R19+ SpectrAI 集成蓝图** | A 方案决策 + mid-task bug + 命名空间 | `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` |
| **apeireth-team-lead 实施指南** | 阶段 1.2 crate 骨架 + 1:1 翻译 + 14 工具 | `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` |
| **R-Measure 守门设计** | 3 baseline 编译期 hardcode + verify 脚本设计 | `docs/stage4/r-measure-verification-design-2026-08-05.md` |
| **全局架构图** | 1 总图 + 13 子图 Mermaid (5 层) | `docs/stage4/global-architecture-map-2026-08-05.md` |
| **Tauri 团队对接 SOP** | 双团队边界 + 5 步 SOP | `docs/stage4/tauri-team-collab-sop-2026-08-05.md` |
| **apeireth-sdk 缺失分析** | 阶段 0 必读 (阶段 4 准备) | `docs/stage4/apeireth-sdk-gap-analysis-2026-08-05.md` |
| **R19 TUI 升级路线图** | Step 1 ✅ + Step 2/3 续 | `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` |
| **ADR-0010~0012** | R19+ 集成 ADR 系列 | `docs/adr/0010-0012-*.md` |
| **APEIRETH-CONVENTIONS** | 12 子规范 + 6 锚 + 7 LOCKED + R-Measure | `APEIRETH-CONVENTIONS.md` |
| **APEIRETH-VERSIONING** | 版本号系统 + Document-Meta | `APEIRETH-VERSIONING.md` |
| **ROADMAP** | 顶层路线图, R20 = 🟡 P1 | `ROADMAP.md` |
| **CHANGELOG** | v2.0.0-alpha 状态 | `CHANGELOG.md` |

### 8.2 reports (10 份, R19+ 阶段 0 调研)

> **修正**: 用户描述 "15 份 reports", 实际 `.minimax-agent-cn\spectrai\reports\` 下 10 份.

| 文档 | 角色 | 阶段 1-2 引用点 |
|------|------|---------------|
| `apeireth-council-7-advisor-analysis-2026-08-05.md` | 7 advisor 投票分析 | 阶段 1.2 T-104 (CouncilVotingTrigger) |
| `apeireth-crate-api-2026-08-05.md` | crate-api 现状 | 阶段 1.2 (team-lead 跟 api 协作) |
| `apeireth-graph-pipeline-analysis-2026-08-05.md` | graph + pipeline 分析 | 阶段 1.3 (14 工具走 pipeline) |
| `apeireth-mcp-14-tool-analysis-2026-08-05.md` | 14 工具详细定义 | 阶段 1.3 T-201~T-204 |
| `apeireth-platform-modules-2026-08-05.md` | 平台模块清单 | 阶段 1.2 (team-lead 依赖项) |
| `apeireth-protocol-4-adapter-analysis-2026-08-05.md` | 4 协议 LLM 抽象 | 阶段 1.2 + 阶段 2.1 (Docker 集成 4 协议) |
| `apeireth-session-vector-asi-2026-08-05.md` | session + vector + asi | 阶段 1.4 (mid-task bug 修法 + `apeireth-session` 新建) |
| `apeireth-supervisor-tool-rules-2026-08-05.md` | supervisor + tool rules | 阶段 1.2 (team-lead ≠ supervisor 命名空间, per ADR-0011) |
| `spectrai-architecture-2026-08-05.md` | SpectrAI 19 模块架构 | 阶段 1.2 (1:1 翻译 supervisorPrompt.ts) |
| `tauri-roadmap-2026-08-05.md` | Tauri 13 项资产 | 阶段 1.1 (TUI 跟 Tauri 同步) |

### 8.3 输出 (本指南产出)

| 文档 | 路径 |
|------|------|
| 本指南 | `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` |
| 10 子阶段 measure 报告 | `reports/r20-stage-{1,2}-{1..5}-measure-<date>.md` |
| 阶段 1-2 完工报告 | `reports/r20-stage-{1,2}-complete-<date>.md` |
| 用户文档 (阶段 2.5) | `docs/user/{quickstart,upgrade,uninstall}.md` |

---

## §9 待 Mavis 拍板的事 (3 项, 阶段 1-2 开工前必拍)

> 跟 r20 §11 拍板项对齐. 阶段 1-2 开工前必拍板 1-2 项, 其余阶段 3-5 开工前再拍.

### 9.1 R20 vs R21 边界 (🔴 关键, 跟 r20 §11.1 同)

主人 2026-08-04 12:30 提到"含计费+订阅+API 配额" — **本路线图默认 R20 = 收产品 (不含商业化), R21 = 商业化** (per r20 §11.1 拍板 A).

**拍板**:
- (A) ✅ **本路线图默认**: R20 收产品 / R21 商业化 (推荐, 风险 R-008)
- (B) R20 直接含基础计费 (Stripe 接入 + API key 配额管理), 商业化推到 R22

**阶段 1-2 影响**: 拍板 A 的话, 阶段 1-2 **不**碰计费/订阅/配额, 阶段 3 加基础 rate limit (R21 商业化前置), 阶段 4 SDK **不**含计费方法. 拍板 B 的话, 阶段 1-2 末段要预留计费扩展点.

### 9.2 Tauri 团队同步节奏 (🟡 中, 跟 r20 §11.2 同)

R20 阶段 1.1 TUI 深化决策是否要等 Tauri 团队反馈?

**拍板**:
- (A) ✅ **TUI 独立做** (R20 阶段 1-2 不假设 Tauri 进度, 每周跟 Tauri 团队 1 次同步, per r20 §11.2 拍板 A)
- (B) TUI 等 Tauri 团队设计语言定稿再深化 (R20 阶段 1.1 延 2-3 周)

**阶段 1-2 影响**: 拍板 A 的话, 阶段 1.1 TUI 改瘦续 Step 2/3 独立做, 阶段 2.1 Docker image 跑 TUI 跟 Tauri 无关. 拍板 B 的话, 阶段 1.1 阻塞 2-3 周, 阶段 2 顺延.

### 9.3 SDK_VERSION 升 0.1.0 → 1.0.0 (🟡 中, 跟 r20 §11.5 同)

`apeireth-sdk/src/version.rs` 当前 `SDK_VERSION = 0.1.0` (协议层 wire-format 版本), workspace `Cargo.toml` 是 1.0.0. 两者不同.

**拍板**:
- (A) ✅ **R20 阶段 4 一起升 0.1.0 → 1.0.0** (跟 R20 阶段 3 OpenAPI 规范同周期, 推荐)
- (B) 保持 0.1.0 (协议层未稳定, 不动)

**阶段 1-2 影响**: 拍板 A 的话, 阶段 1.2 team-lead crate 引用 `apeireth-sdk::SdkVersion` 时**不**立即依赖 1.0.0, 阶段 4 一起升. 拍板 B 的话, 阶段 1.2 引用 0.1.0, 阶段 4 不动.

**本指南默认全部走 A**, 等主人 2026-08-05 复核时一次性拍板.

---

## §10 总结

### 10.1 时间线

```
2026-08-05  R20 启动 (v2.0.0-alpha 发版) + 本指南草拟
  ↓
2026-08-06 ~ 08-14 (9 天 = 1.8 周): 阶段 1 产品基础
  ├── 08-06~08-08 (3 天): 阶段 1.1 TUI 9 命令深化
  ├── 08-09~08-10 (2 天): 阶段 1.2 team-lead 公开 API
  ├── 08-11~08-12 (2 天): 阶段 1.3 mcp::team 14 工具
  ├── 08-13 (1 天):         阶段 1.4 mid-task bug 3 处修法
  └── 08-14 (1 天):         阶段 1.5 端到端 + R-Measure 守门
  ↓
2026-08-15 ~ 08-26 (12 天 = 2.4 周): 阶段 2 部署基础
  ├── 08-15~08-17 (3 天):   阶段 2.1 Docker 多架构
  ├── 08-18~08-19 (2 天):   阶段 2.2 离线包
  ├── 08-20~08-22 (3 天):   阶段 2.3 系统包 (deb/rpm/brew/scoop)
  ├── 08-23~08-24 (2 天):   阶段 2.4 一键安装脚本
  └── 08-25~08-26 (2 天):   阶段 2.5 安装验证 + 文档
  ↓
🎯 4 周 (21 工作日) = R20 阶段 1-2 完工 (2026-08-26 目标)
  ↓
2026-08-27 ~ 09-30 (5 周): 阶段 3-5 (API 公开 + SDK 完善 + 文档营销)
  ↓
2026-10-01 ~ R21 商业化
```

### 10.2 总时长核对

| 阶段 | 估时 | 实际估 (本指南) | 差异 |
|------|-----:|---------------:|------|
| 阶段 1 | 1-2 周 (5-10 天) | 9 天 (1.8 周) | ✅ 落在范围 |
| 阶段 2 | 2 周 (8-12 天) | 12 天 (2.4 周) | 🟡 略超 (Docker 多架构 + 4 系统包 + CI matrix 偏密集) |
| **阶段 1-2 总** | **3-4 周 (13-22 天)** | **21 天 (4 周)** | ✅ 落在范围上沿 |

**主人 2026-08-05 复核时建议**: 阶段 2.3 系统包 (3 天) 可拆核心 (deb + brew, 2 天) + 可选 (rpm + scoop, 1 天), 这样阶段 2 缩到 10-11 天, 总 19-20 天 = 3.8 周, 留 0.2 周 buffer.

### 10.3 关键数字

- **10 子阶段** × 估 LOC: 阶段 1 (5 子阶段, 估 3680 LOC) + 阶段 2 (5 子阶段, 估 2700 LOC) = **6380 LOC**
- **10 子阶段** R-Measure 守门, 报告路径 `reports/r20-stage-{1,2}-{1..5}-measure-<date>.md`
- **17 风险** (8 继承 + 9 新增), 7 🔴 高 / 10 🟡 中
- **4 不修改承诺** (R11 baseline 3 值 + workspace 1.0.0)
- **6 哲学 anchor** 穿透 100% (S-1/S-2/O-2/O-3/O-4/O-5)
- **9 个 owner** 角色 (frontend/backend/backend2/devops/devops2/qa/fullstack/tech_writer/security_reviewer)

### 10.4 跟 8 个必读文档的引用清单

| 必读文档 | 本指南引用章节 |
|---------|--------------|
| `r20-product-finalize-2026-08-05.md` | §1 (战略) / §3.5-3.7 (简略) / §5 (风险) / §6 (不修改承诺) / §7 (哲学) / §8 (关联) / §9 (拍板) |
| `spectrAI-integration-blueprint-r19-plus-2026-08-05.md` | §1.3 (衔接) / §2.3 (mcp::team) / §2.4 (mid-task bug) / §5.1 (命名空间) |
| `apeireth-team-lead-implementation-guide-2026-08-05.md` | §2.2 (5 T-xxx 详细) / §5 (R-009 引用) / §7 (S-2 引用) |
| `r-measure-verification-design-2026-08-05.md` | §1.4 (守门总览) / §4 (10 子阶段守门点) / §5 (R-017) / §6 (不修改承诺第 6 项) |
| `global-architecture-map-2026-08-05.md` | §1.1 (R20 在哪) / §2.x (5 层架构引用) |
| `tauri-team-collab-sop-2026-08-05.md` | §2.1 (TUI 跟 Tauri 同步) / §5 (R-002) / §9.2 (拍板) |
| `apeireth-sdk-gap-analysis-2026-08-05.md` | §5 (R-009) / §9.3 (拍板 SDK_VERSION) |
| 10 份 reports/ | §2.x (T-xxx 详细引用) / §8.2 (完整清单) |

---

_本指南 v1 草拟 (按 R20 路线图 §4 阶段 1-2 + R19+ 集成蓝图 + R-Measure 守门设计 + 10 份 reports)._

_3 项拍板 (R20 vs R21 边界 / Tauri 同步 / SDK_VERSION) 待 Mavis 跟主人 2026-08-05 复核._

_主哲学 6 锚穿透. 任何接手者能查. 10 子阶段 × T-xxx 任务清单 + R-Measure 守门 + 17 风险 + 8 不修改承诺 齐全._

_下一步: 拍板后 → 派 6 owner (frontend/backend/backend2/devops/devops2/qa) 开工 → 10 子阶段 measure 报告陆续写 → 阶段 1-2 完工报告 `reports/r20-stage-{1,2}-complete-2026-08-XX.md`._

---

## 拍板记录

- **2026-08-05** — Mavis 草拟本指南 v1 (按主人 2026-08-04 12:30 R20 P1 方向 + R20 路线图 + R19+ 集成蓝图 + R-Measure 守门设计)
- **2026-08-05** — sub-agent 报告: 10 份 reports (用户描述 15 份有差, 按实际 10 份列)
- **2026-08-05** — R20 路线图 §11 拍板项 A (R20 收产品 / R21 商业化 / TUI 独立做 / SDK 串行) 本指南默认沿用
- **<pending>** — 主人 2026-08-05 复核时拍板 §9 3 项 (R20 vs R21 / Tauri 同步 / SDK_VERSION)
