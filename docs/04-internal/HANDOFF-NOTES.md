# 接手人手册 (HANDOFF-NOTES.md)

> **给谁看**：从零接手 v2 工程的新人. 你**不**知道这个项目, 这份文档给你**第一个**上下文.
> **HEAD**：以 `main` 当前提交为准（本文件随 cognitive module wiring 同步更新）。
> **状态（2026-08-28）**：RC-1/2/3/4/5/6/8/9/10 已有真实实现或适配，RC-11 v1→APX2 migration utility 已落地；canonical cognitive module ABI 已完成，记忆/偏好/写回/Judge-backed assessment/Council adapter/Experience extraction 已接入单一 composition root。MiniMax provider E2E 仍需凭证；Orchestrator、偏好学习、长程 reflection、非文本 perception 仍明确延期。

```yaml
[Document-Meta]
Document:        docs/04-internal/HANDOFF-NOTES.md
Version:         Handoff-Rev-1.0
Last-Modified:   2026-08-28
Status:          🟢 活跃 (接手人入口)
```

---

## 1. 项目 1 段简介

**Apeireth** 是 Rust 写的 AI 伙伴底座 (base), 不是 AI 本身 — LLM 是 tenant, 换 model 不重做 base. v2 是从 v1 (86-crate, 完整 9 器官) 工程重构后的形态: **15-crate 工作区, 单 SQLite WAL, external hook 治理, OpenAI Chat 兼容入口**. 当前主线 = `main` 分支 (默认), 旧 v1 走 `archive/v1.0-master` (永久维护). v2 设计哲学 / 8+1 哲学锚 / 13 键 / 三洋葱 / L0 HA / 0 装 PASS 全部 **LOCKED 跨阶段 0 改**, 变的是工程形态.

---

## 2. 哲学锚 9 项速览 (LOCKED, 0 改)

来源：`docs/01-architecture/philosophy.md`. 任何改动 LOCKED 项 = 锚违约.

| # | 锚 | 一句话 |
|---|---|---|
| **S-1** | 北极星 | 一切服务 ASI 北极星 (五原型). |
| **S-2** | 实事求是 | 写前验证; 真相高于叙事. |
| **S-3** | 质量工程化 | 工程严谨压倒叙事 — CI 闸 + Kani + clippy 0 警告. |
| **O-1** | 安全优先 | 安全压倒其他 — 9 重守门 + 13 键 + 3 项不可变脊柱. |
| **O-2** | 前人肩上 | 借 + 标注 + 改 (不抄). |
| **O-3** | 干到底 | 不做半截活. |
| **O-4** | 任何人都能接手 | 文档单独能 onboard. **本文件就是锚兑现**. |
| **O-5** | 不假装 (0 装 PASS) | `unimplemented!()` 必须显式标注, 绝不静默. |
| **O-6** | 永远追求最优 (新, 2026-08-27) | **三阶审查 (总体 > 系统 > 架构) + 不做借口清单 + 可检查信号**. 工程兑现: clippy 0 警告 + commit 0 静默失败 + trait 位置必写理由. |

**重点 O-6**：每条工程决策动手前必过三阶审查. "工作量太大 / 等以后做 / alpha 阶段先这样 / v1 时代这样 / 用户没要求 / 派子代理就行" 都是**显式拒绝**的借口. 子代理**可**调研, **主代理必**拍板.

---

## 3. RC 进展状态表 (10 个 RC + 3 cognitive commit)

本批 18 commit (`ef075420..HEAD`), 含 O-6 锚兑现 14 commit + 真实现 5 commit + 子代理反馈修 2 commit + cognitive module 3 commit (其他 dev).

| RC | 状态 | 内容 | 关键 commit |
|---|---|---|---|
| **RC-1** | ✅ 真实现 | MemoryBackend trait → SQLite 纯 SQL (绕开 mutex) | `43ec9635` |
| **RC-2** | ✅ 真实现 | Experience trait → SQLite (WikiEntry / KG / Association 全 impl) + RC-8 改名 (子代理 C 反馈) | `4e4fba89` |
| **RC-3** | ✅ 真实现 | PreferenceStore → SQLite (Noop impl 收, 真 SQL 兑现, 子代理审查修) | `03f5ed71` / `61cc0421` |
| **RC-4** | ✅ 真实现 | SelfAssessmentStore → SQLite (场景 D 例 2) | `042ad4eb` |
| **RC-5** | ✅ provider adapter；E2E 待 key | MiniMax `LlmFactory` 真实 adapter 已落地；Orchestrator harness 仍延期 | `02faa6d0` |
| **RC-6** | ✅ bounded advisor adapter；provider E2E 待 key | 7 个 LLM advisor slot 并行、10s/60s bounded、DeferToHuman；canonical runtime 通过 `ModuleInvoker` 接入 | `a3768fd6` / `863df70f` |
| **RC-7** | ⏳ 待硬件 | Perception → 真 modality (Whisper / xcap), Text impl 已就位, 4 modality forward-declared | (PerceptionInput trait in `crates/foundation/plugin/src/perception.rs:86`) |
| **RC-8** | ✅ 真实现 + 改名 | SubSupervisor → tokio::process 真实 + 改名 (子代理 A 反馈) | `67fc66a0` / `4e4fba89` |
| **RC-9** | ✅ 真实现 | Keyring 真接入 CLI bootstrap (4 backend + selector → EnvCredentialResolver fallback) | `aa661a66` |
| **RC-10** | ✅ 真实现 | File AES-256-GCM 加密 + metadata-bound APX2 header | `2214fb01` |
| **RC-11** | ✅ migration utility | 离线 v1→APX2 重签；截断与超长 logical id fail-closed | `a565f011` |
| **cognitive wiring** | ✅ 本轮 | 唯一 slot ledger + production composition + Memory/Preference/Assessment/Writeback + Judge adapter；Critic/Reflection/Planner 不重复造 loop | `docs/04-internal/cognitive-module-wiring.md` |

子代理审查 5 项修正 (`61cc0421`): RC-1 真 SQL impl 兑现 + RC-3 真 SQL impl + SelfAssessment 单 source of truth.

---

## 4. 15-crate 拓扑 + 7 capability trait 边界

```
crates/
├── foundation/         (7 — 抽象 / 协议 / 治理)
│   ├── core/           (13 键哲学 + L2 哲学标准)
│   ├── protocol/       (NormalizedTool + 协议归一化)
│   ├── plugin/         ◀ 7 capability trait 集中地 (O-6 重构后)
│   ├── governance/     (L1 hook 闸: Permission / CredentialDisclosure / PromptInjection)
│   ├── credentials/    (KeyringCredentialResolver, 4 backend)
│   └── orchestration/  (Council + TeamLead + Orchestrator trait, alpha 阶段 0 装)
├── engine/             (5 — 执行 / 调度)
│   ├── runtime/        (canonical agent loop + governance pipeline 接线)
│   ├── provider/       (3 provider: MiniMax / Anthropic / OpenAI-compatible)
│   ├── storage/        (SQLite WAL + reader pool + migrations)
│   ├── memory/         (M1B 记忆 primitive, trait 边界已锁)
│   └── perception/     (PerceptionInput 5 modality, alpha Text impl)
├── capabilities/       (1)
│   └── tools/          (5 内置工具: filesystem/search/repo 只读默认开; shell/fetch opt-in)
└── adapters/           (3 — 入口)
    ├── gateway/        (:8080 HTTP, OpenAI Chat 兼容)
    ├── cli/            (session / chat / gateway serve sub-command)
    └── sdk/            (对外 API)
```

**7 capability trait** 全部在 **`crates/foundation/plugin/src/`** (O-6 重构 Refactor-1/2/3 兑现, `30d342fa` / `f2cfaa76`):
- `memory_backend.rs:75` `pub trait MemoryBackend`
- `experience.rs` `pub trait Experience` (5 sub-trait: WikiEntry / KG / Association)
- `perception.rs:86` `pub trait PerceptionInput`
- `preference.rs:56` `pub trait PreferenceStore`
- `self_assessment.rs:66` `pub trait SelfAssessmentStore`
- `credentials.rs:58` `pub trait CredentialResolver`
- `llm_factory.rs:151` `pub trait LlmFactory` (RC-5 前置)
- `capabilities/tools/src/supervisor.rs:132` `pub trait SubSupervisor` (engine 侧, impl 位置)

**Runtime 注入模式**：`Arc<dyn Trait>` 走 `Runtime::build_canonical_runtime_from_env`, 单向依赖 (foundation 不依赖 engine, engine 依赖 foundation trait).

---

## 5. 0 触碰 LOCKED 5 项 (LOCKED 数据不可改)

| LOCKED 项 | 位置 | 数字 / 内容 |
|---|---|---|
| **9 哲学锚** | `docs/01-architecture/philosophy.md` | S-1~3 + O-1~6 全部 LOCKED, 0 增 0 减 0 改 |
| **13 键 verdict cache** | `crates/foundation/core/src/philosophy.rs:142` | `RUNTIME_ENFORCED = false` (永久降级为哲学标准, **不**接 runtime 强制) |
| **3 项不可变脊柱** | `crates/foundation/governance/` | Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义 |
| **workspace.version** | `Cargo.toml:43` | `version = "1.2.0"` (产品轴 tag + workspace 轴双轴制) |
| **R11 baseline 3 值** | ASI R-Measure | `0.8682 / 0.8532 / 0.9063` 数字严守 (per `Cargo.toml:94` 注释) |

**子代理反馈已修 2 项 (本批)**：
- **R1 build break** (`61cc0421`) — 子代理审查发现 RC-1 + RC-3 真 SQL impl 缺, 当时是 Noop 占位; 已修 = 全部真 SQL 重写.
- **R2 命名错位** (`4e4fba89`) — 子代理 C 反馈 RC-8 `SubSupervisor` 命名与 v1 重名但语义不同; 已改 = 加注释明确区分.

---

## 6. 关键风险 (按子代理 B 报告 5 项, 接手人注意)

| # | 风险 | 严重度 | 谁能解决 | 接手人该做 |
|---|---|---|---|---|
| **R-B1** | RC-5 provider E2E、Orchestrator harness、RC-7 perception 仍受外部条件或范围限制 | 高 | 主代理 + 需 LLM API key + 硬件 | RC-6 bounded adapter 已落地；后续只补真实 provider E2E、Orchestrator harness 与 perception，不回退到第二 loop. |
| **R-B2** | 9 器官 (W1/W2/W3/E4/F4/F1/F6/E7) 全部在 `legacy/donor/apeireth-companion`, 未移植 v2 主链 | 高 | 主代理 + 6-8 周工作量 (ROADMAP §4 P6) | **不要**在 rc 阶段硬塞 — 长程任务继续走 v1 branch (`archive/v1.0-master`). v2 rc 阶段走标准 OpenAI Chat 兼容. |
| **R-B3** | LLM 调用成本 (主对话 + 每 N turn 自评 + 偏好 recall) | 中 | 优化 `LlmFactory` 默认 model (cheap model) | 接 RC-5 时按 `v2.0.0-rc-roadmap.md` §5 风险行缓解: PerSpec 缓存 + advisor 可降 5→3. |
| **R-B4** | v1 → v2 数据 schema 兼容 (rc 阶段假设兼容, v2.0.0+ 引入新表) | 中 | migrations 走幂等 `IF NOT EXISTS` (per `crates/engine/storage/src/migrations.rs`) | 数据迁移按 `migration-v1-to-v2.md` §4.4 步骤走, **先备份 v1 db**. |
| **R-B5** | Cognitive module 集成 (`a699c5f5`/`1d227d6a`/`64e64f46`) 与现有 runtime 边界不清 | 中 | 其他 dev 推, 主代理 review | 接 RC-5 时查 `crates/engine/runtime/src/canonical/module.rs` (cognitive ABI 入口), 不与 orchestrator 重复设计. |

---

## 7. 下一步 (按优先级)

1. **RC-5/7 与 provider E2E** — RC-5 MiniMax adapter 与 RC-6 bounded Council 已落地；下一步是凭证条件下的 provider E2E、Orchestrator harness，以及硬件相关的 Whisper / xcap.
2. **Cognitive module 集成 review** (其他 dev 推, 接手人看 3 commit) — `a699c5f5` ABI / `1d227d6a` integration / `64e64f46` lifecycle. 重点看 `crates/engine/runtime/src/canonical/module.rs` 与 `execute.rs` 边界.
3. **v1.0 parity 完成 (ROADMAP §4 P3-P6)** — 子代理 B 估 14-19 周: M1B 记忆移植 (P3) → perception trait (P4) → tool-runtime + supervisor + SelfAssessment (P5) → council + team-lead + cognition (P6).
4. **13 键永久降级后** 仍有 3 用法 (hook deny reason / CapabilityDescriptor risk 分级 / ROADMAP §5 语义定义) — 不接回 runtime 强制.
5. **前端 companion-desktop 对接 v2 gateway** — v2.0.0 阶段, 当前 0.5.0 前端接 v1.

### 7.1 Cognitive module wiring (本轮新增)

权威 slot ledger 见 `docs/04-internal/cognitive-module-wiring.md`。不要
在 CLI、Gateway、SDK 或 Orchestrator 内另注册一套模块：CLI 的
`build_canonical_runtime_from_env` 是当前 production composition root，Gateway
复用它，SDK 目前是 client-only。Embedding caller 只能通过
`ProductionCognitiveModules` + `RuntimeBuilder::with_module` 显式添加模块，
并接受 runtime 的 duplicate-id / hook / round / side-call 守门。

默认无额外模型成本：memory/preference recall、AfterTurn writeback 与
保守 Experience extraction 走注入 backend；Judge/Council 只有对应环境开关才
开启。Council 通过 runtime-owned `ModuleInvoker` 做有界 typed advisor side-call，
默认最多 7 个 advisor、单 advisor 10s、整体 60s；真实 provider E2E 仍需凭证。
Experience 只在 episode durable commit 成功后提炼有界摘要与显式 marker，并保留
source episode evidence；不宣称长程 cognition、偏好学习或完整语义 LLM extraction
已完成。

### 7.2 远程验证记录 (2026-08-28)

本轮验证统一在 `desktop-dcce212558a843ed-20260806111728416` 的
`D:\apx\apeireth-rust` 执行，远程验证最终提交为 `21eb5291`，
并确认远程 `HEAD == origin/main`、工作树 clean；工具链为 `rustc 1.97.1 / cargo 1.97.1`。
以下结果均为远程结果：

- `cargo test -p apeireth-runtime`
- `cargo test -p apeireth-memory`
- `cargo test -p apeireth-orchestration`
- `cargo test -p apeireth-cli`
- `cargo test -p apeireth-governance`
- `cargo test -p apeireth-tools-canonical`
- `cargo test -p apeireth-credentials`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- 本轮触碰 Rust 文件的定点 `rustfmt --check` 与 `git diff --check`

`cargo clippy --workspace --all-targets --all-features -- -D warnings` 首次因远程机
未安装 Python 3.x 阻断；随后在远程配置官方 Python 3.12.10 embeddable interpreter，
以 `PYO3_PYTHON=D:\apx\python312-embed\python.exe` 重跑并通过。没有修改 PyO3
语义或禁用 Python feature。all-features 运行产生的 tracked SDK header 已恢复，
远程验证目录最终 clean；Python runtime 保留在 `D:\apx` 作为验证机前置。

---

## 8. 接手人 5 条 actionable advice

1. **跑 baseline** — `cargo test --workspace --locked` 应 0 失败, `cargo clippy --workspace --all-targets --locked -- -D warnings` 应 0 警告 (本批实测: 测试通过, clippy 干净). 不通过 = O-6 锚违约, 不可推迟.
2. **RC-8 命名注意** — `crates/capabilities/tools/src/supervisor.rs:132` 的 `SubSupervisor` 是 **v2 进程级 supervisor** (重启 child 进程), **不**是 v1 `apeireth-team-lead` 14 调度工具. 别混淆.
3. **哲学锚编号有 ledger** — 哲学锚 #1-#9 是稳定编号, 但 commit message 里 "O-6 锚 #11 / #18 / #23" 等是 O-6 重构批次的**子项序号** (不是哲学锚数), 看 `o6-session-log-2026-08-27.md` §1 commit 表对应.
4. **deprecated consumer 清理** — 当前 active workspace 扫描显示 `#[allow(deprecated)]`
   与 `#[deprecated]` consumer 均为 **0**；旧 legacy/archive 与历史文档保留作考古资料，
   不从生产主线删除。不要为已不存在的消费者伪造迁移或回退兼容边界。
5. **认知模块生命周期不变量** — `64e64f46` 修复 cognitive hook lifecycle, runtime 在 `execute.rs` 加了 invariant assertions. RC-5 接 orchestrator 时**不**要绕过 module.rs 的 hook 注册表 — 走 `register_hook` ABI, 不直接 mut state.

---

## 9. 必跑命令 (接手时验证 baseline)

```bash
# 1. 全 workspace 测试 (本批实测 0 失败)
cargo test --workspace --locked 2>&1 | tail -5

# 2. clippy 严格档 0 警告 (O-6 锚兑现 #1)
cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -3

# 3. O-6 锚兑现 5 重守门 (CI workflow, push 自动跑)
cat .github/workflows/o6-anchor.yml  # 4 步: clippy + test + legacy shim + LOCKED 数据 0 触碰

# 4. 本地启动 v2 (需 API key)
export APEIRETH_MINIMAX_API_KEY=sk-...
cargo run -p apeireth-cli -- gateway serve --port 8080

# 5. 看子代理反馈 (commit message 内嵌, 没有独立 report 文件)
git log --oneline --grep="子代理"  # 列出所有子代理相关 commit
# 关键 5 commit:
#   61cc0421 (子代理审查 3 项修正: RC-1/RC-3 真 SQL + SelfAssessment 单一 source)
#   67fc66a0 (子代理 A 错误类型注释)
#   4e4fba89 (子代理 C 反馈 RC-8 改名 + RC-2 真 SQLite)
#   ca0f48e9 / ed0a0913 (子代理 D 拍板 NoopLlmFactory / 真 core drain)

# 6. 验证 git remote
git remote -v
# origin: github.com/Apeireth/apeireth-rust.git (主, push 走代理)
# jimmy:  github.com/Jimmyxiao2009/Apeireth-rust.git (fork)
```

---

## 10. 参考文档清单

### 必读 (5 个, 顺序读)

1. `ROADMAP.md` — 顶层状态 + §3 5 重守门 + §4 P1-P8 + §5 硬墙
2. `docs/01-architecture/philosophy.md` — 9 哲学锚 (重点 O-6)
3. `docs/04-internal/v2.0.0-rc-roadmap.md` — 10 RC 任务 + 验收 + 14-19 周时间表
4. `docs/04-internal/migration-v1-to-v2.md` — v1 → v2 切路径 (3 种路径 A/B/C)
5. `docs/04-internal/o6-session-log-2026-08-27.md` — 本次会话反思 (子代理教训 / 0 装锚)

### 应读 (10 个, 按主题)

| 主题 | 文档 |
|---|---|
| 重构背景 | `docs/01-architecture/v2-arch-refactor-batch.md` (5 Refactor + O-6 兑现) |
| 缺口盘点 | `docs/04-internal/v2-unabsorbed-features.md` (6 A + 7 B + 14 C + 14 D) |
| 长程 AI | `docs/04-internal/scene-d-v2-plan.md` (3 例 / multi-instance / 7-11 周) |
| 凭证 | `docs/04-internal/secret-management-policy.md` (4 backend 选型) |
| 团队手册 | `docs/04-internal/next-team-handbook.md` (v2 维护) |
| CI 防御 | `docs/04-internal/ci-fix-log-2026-08.md` (本批修过的 CI 问题) |
| 维护 | `docs/04-internal/maintenance-guide.md` (legacy 兼容) |
| 插件开发 | `docs/04-internal/plugin-authoring-guide.md` (plugin crate 新增) |
| Async 决策 | `docs/04-internal/async-trait-decision-matrix.md` (RC-5 时用) |
| 借力 | `docs/04-internal/borrow-from-jimmyxiao2009.md` (O-2 锚兑现史) |

### 可选 (3 个, 进阶)

- `docs/01-architecture/vision.md` — 5 原型 + ASI 北极星详述 (S-1 锚)
- `docs/archive/conventions/09-anchor.md` — 9 锚 v0 时代原文 (哲学层保留)
- `docs/archive/rounds/r179/r179-session-handoff-2026-08-15.md` — v1 时代 handoff 模板参考

---

## 11. 一句话总结

**v2.0.0-alpha.1 = 骨架 + 主链 + governance P0 + 13 键降级** (15 crate / 全 workspace 测试通过 / 0 clippy 警告).
**v2.0.0-rc.1 = 接真 backend** (RC-1/2/3/4/5/6/8/9/10/11 已有实现或适配；provider E2E、Orchestrator、RC-7 perception 与长程 cognition 仍延期).
**v2.0.0 = 完整功能 + frontend** (rc 后 ~6-8 周, 含至少 1 器官移植).

设计哲学 / 8+1 锚 / 13 键 / 三洋葱 / L0 HA / 0 装 PASS 跨 v2 三个阶段 **0 改**. O-6 永远是守门人 — 你也是.

---

_本手册 v1 首发 (2026-08-27): 接手人**从零**能继续干. 所有命令 / commit / file:line 在 HEAD `ad1c6d44` 实测可跑. 0 触碰 LOCKED 数据. 派子代理**可**调研, 主代理**必**拍板._
