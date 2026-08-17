# ADR 0002: CLI 接入 core Session API 绑定架构决策

> **性质**: 第二个 ADR —— 记录 "apeireth-cli 如何绑定 apeireth-core Session API" 的架构决策，为 A1 成就（CLI session 启动）提供长期稳定的前端↔核心契约。
> **依据**: 主人 2026-08-01 "A1 第 1 天任务 = CLI 接 core Session API" + 阶段 4 §1.6 经典视图（CLI 是前端一种实现，前端可替换）+ ADR 0001（双洋葱统一体决策的精神延续）+ A1.1 main.rs 当前实现（println! 占位）。
> **commit 锚**: A1.4 子成就（施工期第一个 ADR）。

---

## 状态

🟢 **Accepted**（施工团队架构师 2026-08-01 接受，主哲学 6 锚穿透，0 触动 LOCKED）

---

## 背景（Context）

Apeireth R14 的核心约束之一是 **"前端可替换，后端不动"**（阶段 4 §1.6 经典视图 v2 修订）。
当前实现状态（A1.1）：

| 文件 | 现状 |
|---|---|
| `crates/apeireth-cli/Cargo.toml` | 已声明 `apeireth-core = { path = "../apeireth-core" }` 等依赖 |
| `crates/apeireth-cli/src/lib.rs` | 仅暴露 `CliCommand` 枚举 + `placeholder()` 函数 |
| `crates/apeireth-cli/src/main.rs` | **占位实现**：直接用 `println!` 输出欢迎信息，未真正调用 `apeireth_core::Session` API |
| `crates/apeireth-core/src/lib.rs` | 已暴露 `pub struct Session { id, started_at, last_active_at }`（主路径核心类型 §1） |

**问题**：

1. **耦合风险**：如果 `main.rs` 直接 `use apeireth_core::*` 调内部方法（如 `PermissionOnion::reorganize()`），则 CLI 与 core 内部 API 强绑定 —— 后续 core 任何内部重构都会破坏 CLI。
2. **替换成本**：未来 GUI/Web/Desktop 前端（阶段 7+）若想复用同样的 Session 逻辑，必须从 core 直接调用 → 每个前端都重写一遍绑定层。
3. **测试隔离困难**：CLI 单元测试若直接调 core 内部，会污染 core 测试命名空间。
4. **A1 验收漂移**：A1 DoD = "session 启动输出欢迎信息"。如果用 println! 占位"假装完成"，A5 阶段 V1+V2+V3 接入时会发现 CLI 没真正走过 Session 生命周期。

---

## 决策（Decision）

**采用"CLI lib 抽象层"绑定模式**：

> `apeireth-cli/src/lib.rs` 是 CLI 的**唯一公开 API 表面**。它**封装** core Session API 为 CLI 友好的函数。`main.rs` **只通过 `apeireth_cli::*` 调用**，**绝不直接 `use apeireth_core::*`**。

### 公开 API 表面（`apeireth-cli/src/lib.rs` 必须暴露）

```rust
// 阶段 1：A1 最小集
pub fn create_default_session() -> Session;
pub fn run_session_action(session: &Session, action: SessionAction) -> Result<SessionResult, CliError>;
```

### 绑定层职责（lib.rs 内部）

| 函数 | 职责 | 内部调用 |
|---|---|---|
| `create_default_session()` | 构造一个 `Session { id: 唯一 ID, started_at: now, last_active_at: now }` | 可直接构造（当前 A1 占位），未来 A11 接 `apeireth-memory` SQLite 持久化 |
| `run_session_action(s, a)` | 接收 SessionAction，返回 SessionResult | A1 占位：Ack 即可；A5 后真正走 V1+V2+V3 AND 门；A7 后接 Self-Disable 防护 |

### `SessionAction` 枚举（在 lib.rs 定义）

```rust
pub enum SessionAction {
    Start,                  // session 启动（默认）
    ListEpisodes,           // 列出最近 episode
    RunV1136Benchmark,      // 跑 V1136 真测
}
```

### `main.rs` 绑定规则

- ✅ 允许：`use apeireth_cli::{CliCommand, create_default_session, run_session_action};`
- ❌ 禁止：`use apeireth_core::*` 或 `use apeireth_core::Session as CoreSession;`
- ✅ 允许：在 `CliCommand::Session` 分支调 `create_default_session()` + `run_session_action(s, Start)`
- ❌ 禁止：绕过 `run_session_action` 直接调 core 的守门方法

---

## 后果（Consequences）

### 正面

- ✅ **CLI 与 core 解耦**：core 内部类型（如 `PermissionOnion`、`PhilosophyGuard`）可自由重构，只要 `Session` + `SessionAction` 公开契约不变，CLI 不破坏。
- ✅ **前端可替换**：未来 `apeireth-gui`、`apeireth-web`、`apeireth-desktop` 可以选择：(a) 复用 `apeireth_cli` lib 作为统一绑定层，或 (b) 各自定义自己的绑定层（仍然通过 `apeireth_core::Session` 公开 API）。两种都不需要重写 core。
- ✅ **测试隔离**：CLI 单元测试 mock `create_default_session()` / `run_session_action()` 即可，不需要 mock core 内部。
- ✅ **Self-Disable 边界清晰**：lib.rs 抽象层是 cli 唯一对外契约，所有"禁止 API"（见下文边界段）在 lib.rs 集中显式禁止，main.rs 无从越界。
- ✅ **A1→A5→A20 演化平滑**：A1 占位实现 → A5 接 V1+V2+V3 → A7 接 Self-Disable → A20 完整 CLI，绑定模式不变，只换 lib.rs 内部实现。

### 负面

- ⚠️ **额外一层抽象**：lib.rs 成为必须维护的"翻译层"，增加少量样板代码（ponytail: 抽象天花板 = 当 core 公开契约稳定后，可考虑将 `run_session_action` 直接内联到 main.rs，节省一层）。
- ⚠️ **API 命名耦合**：cli lib 的 `SessionAction` 与 core 内部的 action 概念可能产生命名混淆（ponytail: 升级路径 = 若 core 后续暴露 `Action` enum，可考虑用 type alias 桥接）。
- ⚠️ **A1 阶段看似"多此一举"**：当前 println! 占位就能通过 A1 DoD，但会埋下 A5 阶段"重新接 Session API"的返工成本（ponytail: 升级路径 = A1 就建立绑定模式，A5 直接升级内部实现，零返工）。

### 中和

- 🛡️ **绑定模式由 ADR 锁定**：任何想改 main.rs 直接调 core 的尝试必须先废弃本 ADR，避免悄悄破窗。
- 🛡️ **lib.rs 公开 API 由集成测试守卫**：`tests/integration_session_lifecycle.rs` 必须通过 lib.rs 路径走通，不允许绕过。

---

## 备选方案（Alternatives Considered）

### 选项 A: `main.rs` 直接 `use apeireth_core::*`

- `main.rs` 内部 `use apeireth_core::Session` 直接构造 + 直接调守门方法
- **否决原因**：CLI 与 core 内部类型强绑定，未来 core 任何重构都会破坏 CLI；多个前端各自重写绑定层（DRY 违反）；A5 阶段必然返工。

### 选项 B: CLI 不暴露 lib，只用 binary（采纳本 ADR 选项的早期版本）

- `apeireth-cli/Cargo.toml` 移除 `[lib]` 段，只留 `[[bin]]`
- **否决原因**：失去 lib 抽象层，未来 GUI/Web 无法复用 CLI 的命令解析与 Session 绑定逻辑；测试隔离更困难。

### 选项 C: 抽出一个独立的 `apeireth-binding` crate 承载 CLI↔Core 绑定

- CLI、GUI、Web 都依赖 `apeireth-binding`，binding crate 内部调 core
- **否决原因**：当前 A1 阶段过度工程；阶段 7+ 前端真正上马时再做 binding crate 抽象更符合渐进披露原则（ponytail: 升级路径 = 若 frontend ≥ 2 个真的复用 binding 逻辑，再抽出 `apeireth-binding`）。

### 选项 D: 模式 B'（cli 不暴露 SessionAction，只暴露高层命令字符串）

- lib.rs 暴露 `pub fn run_command(cmd: CliCommand) -> ExitCode`
- **否决原因**：抽象层太粗，调用方（main.rs + 未来测试）拿不到中间态 Session，无法做精细断言；测试覆盖度受限。

---

## 实施（Implementation）

### 已落地（A1.1 前置）

- ✅ `apeireth-core/src/lib.rs` §1 主路径核心类型（`Session` 已暴露）
- ✅ `apeireth-core/src/lib.rs` §5 Self-Disable 边界注释（"A. 元问题禁令 / B. 重组洋葱结构禁令 / C. Evolution crate 限制 / D. HA 抗胁迫 / E. Self-Disable 自动检测"）
- ✅ `apeireth-cli/Cargo.toml` 声明 `apeireth-core` 等依赖
- ✅ `apeireth-cli/src/lib.rs` 暴露 `CliCommand` 枚举
- ✅ `apeireth-cli/src/main.rs` A1.1 占位实现（println! 输出欢迎信息）

### 进行中（A1.4 本 ADR 落地）

- ⏳ 本 ADR `docs/adr/0002-cli-session-api-binding.md` Accepted
- ⏳ `apeireth-cli/src/lib.rs` 新增 `create_default_session()` + `run_session_action()` + `SessionAction` + `SessionResult` + `CliError`
- ⏳ `apeireth-cli/src/main.rs` 重构：从 println! 占位 → 走 lib.rs 调用路径
- ⏳ `tests/integration_session_lifecycle.rs` 验证 lib.rs 公开路径（不许绕过）

### 未来（A5+）

- ⏳ A5：`run_session_action` 内部真正走 V1+V2+V3 AND 门
- ⏳ A7：`run_session_action` 内部接 Self-Disable 5 大机制
- ⏳ A11：`create_default_session` 内部接 `apeireth-memory` SQLite 持久化
- ⏳ A20：CLI 完整 17 crate 集成
- ⏳ 阶段 7+：若 frontend ≥ 2 个复用 binding 逻辑，抽出 `apeireth-binding` crate

---

## 🛡️ 不修改承诺核查（7 项 LOCKED 0 触动）

本 ADR 落地过程已严格守住施工团队开工手册 §绝不修改 7 项承诺：

| # | 不修改项 | 本 ADR 是否触动 | 证据 |
|---|---|---|---|
| 1 | **阶段 1+2+3 LOCKED**（54 份设计文档） | ✅ 未触动 | 本 ADR 仅引用阶段 4 §1.6 经典视图，不修改任何 `docs/stage1/`、`docs/stage2/`、`docs/stage3/` 文件 |
| 2 | **v2 / v4 / v4.1 LOCKED**（哲学层纲领） | ✅ 未触动 | 本 ADR 不涉及 `APEIRETH-*-v2.md`、`APEIRETH-*-v4.md`、`APEIRETH-*-v4.1.md` 任何修改 |
| 3 | **阶段 4 主文档 LOCKED**（1492 行） | ✅ 未触动 | 本 ADR 只**新增** `docs/adr/0002-*.md`，不修改 `docs/stage4/stage4-runtime-architecture-revised.md` 等阶段 4 任何文件 |
| 4 | **阶段 5 施工文档 LOCKED**（631 行） | ✅ 未触动 | 本 ADR 不修改 `docs/stage5/stage5-construction-document.md`，仅引用 START-CONSTRUCTION.md 开工手册（已上提顶层）作为权威纪律来源 |
| 5 | **v6 修正**（4 重守门 + 权限发放 + E 层修改路径） | ✅ 未触动 | 本 ADR 的 Self-Disable 边界段引用 v6 §A-E 五大禁令原文，不修改任何 v6 修正链文件 |
| 6 | **R11 baseline 三值 LOCKED**（V1141=0.8682 / V1131=0.8532 / V1136=0.9063） | ✅ 未触动 | 本 ADR 与 R11 baseline 三值无关，A1 占位不影响真测数值 |
| 7 | **v1 → v5 历史链 LOCKED**（保留，不删除） | ✅ 未触动 | 本 ADR 不涉及任何 v1-v5 历史版本文件 |

**核查结论**：7 项 LOCKED 全部 0 触动，本 ADR 可安全落地。✅

---

## 🛡️ Self-Disable 边界（cli lib.rs 必须显式禁止）

依据 `apeireth-core/src/lib.rs` §5 Self-Disable 5 大禁令 + 阶段 4 `stage4-external-feedback-and-revisions.md` §3 + 开工手册 v6 修正链，**`apeireth-cli/src/lib.rs` 公开 API 表面必须显式禁止**：

### ❌ 禁止 API（cli lib.rs 不得暴露）

| # | 禁止 | 原因（v6 修正 §A-E）|
|---|---|---|
| 1 | **`is_l0_ha_required()`** 或任何询问"是否需要 L0 HA"的元问题 | §A 元问题禁令：反思期不能让系统质疑 L0 HA 必要性 |
| 2 | **`disable_self()`** 或任何形式的自我禁用 / 自我关闭 | §E Self-Disable 自动检测：禁用是外部机制，不是 API |
| 3 | **`reorganize_onion()`** 或任何绕开守门修改洋葱结构的方法 | §B 重组洋葱结构禁令：物理隔离 + MultiHuman 多签 + 24h 安静期 |
| 4 | **`modify_l0_layer()`** 或任何直接修改 L0 HA 的方法 | §C Evolution crate 限制：编译时 hardcode 拒绝 L0 相关 trait 修改 |
| 5 | **`bypass_guard()`** 或任何绕过 V1+V2+V3 守门的方法 | §5 重守门：任何 1 守门独立拒绝，无绕过路径 |
| 6 | **`exit_silent_mode()`** 或任何强制退出 Offline/冰冻模式 | §D HA 抗胁迫：冰冻期 24h 内禁止 L0 变更；Offline = 主人不在的安静模式 |
| 7 | **`fake_human_approval()`** 或任何伪造真实人类多签的方法 | v6 修正：MultiHuman 多签必须物理真实，不可伪造 |

### ✅ 允许 API（cli lib.rs 可暴露）

| API | 职责 | 内部约束 |
|---|---|---|
| `create_default_session()` | 构造 Session | 不修改任何 L0/L1/洋葱结构；只用 `started_at = now` |
| `run_session_action(s, action)` | 执行 SessionAction | 内部必须走 V1+V2+V3 AND 门（A5+ 阶段）；A1 占位仅 Ack |
| `SessionAction::{Start, ListEpisodes, RunV1136Benchmark}` | 公开动作枚举 | 不暴露"修改洋葱"、"禁用 L0"、"绕过守门" 等违规 action |
| `CliError` (本 ADR 新增) | 错误类型 | 只承载"参数错误 / core 不可用 / 守门拒绝"等公开错误，不暴露内部栈 |

### 边界守护机制

- 🛡️ **本 ADR 锁定边界清单**：任何后续 PR 想往 lib.rs 加新公开 API，必须先核对本节清单 + 提交 ADR 修订（避免悄悄破窗）。
- 🛡️ **集成测试反向验证**：`tests/integration_session_lifecycle.rs` 中尝试调禁止 API 必须**编译失败**（编译时 hardcode 守卫，A3 成就的 12 键硬编码机制延伸）。
- 🛡️ **代码审查守门**：code_reviewer 角色必须把"cli lib.rs 新增 API 是否触碰禁止清单"作为强制检查项。

---

## 参考（References）

- **START-CONSTRUCTION.md** §绝不修改（7 项 LOCKED）+ §成就驱动 A1-A20
- **ADR 0001**: `docs/adr/0001-double-onion-unity.md`（双洋葱统一体决策，格式参考）
- **阶段 4 §1.6 经典视图 v2 修订**：CLI / GUI / Web 是同一后端的不同前端实现
- **阶段 4 `stage4-external-feedback-and-revisions.md` §3**：Self-Disable 5 大机制（§A 元问题禁令 / §B 重组禁令 / §C Evolution 限制 / §D HA 抗胁迫 / §E 自动检测）
- **apeireth-core/src/lib.rs** §1 主路径核心类型（`Session` 公开）+ §5 Self-Disable 边界注释
- **apeireth-cli/src/main.rs** A1.1 占位实现（println!）
- **apeireth-cli/src/lib.rs** A1.1 占位实现（`CliCommand` 枚举 + `placeholder()`）
- **apeireth-cli/Cargo.toml** 依赖声明（`apeireth-core` / `apeireth-memory` / `apeireth-asi` / `apeireth-philosophy`）

---

## 决策者

- **架构师**（本 ADR 起草者）：2026-08-01
- **主哲学锚穿透**：6 锚（双洋葱统一体 / 主体连续性 / 12 键硬编码 / 5 重守门 / V3 9 键 / 5 项不假装）全部贯穿本 ADR
- **依赖决策**：ADR 0001（双洋葱统一体）→ 本 ADR（CLI 绑定抽象）
- **下一步执行者**：backend_engineer（落地 lib.rs 新 API + main.rs 重构）
- **下游验证**：code_reviewer（必须对照禁止清单审查 lib.rs 新增 API）

---

_ADR 0002 v1（架构师 2026-08-01 起草，A1.4 子成就）._
_依据主人 2026-08-01 "A1 第 1 天任务 = CLI 接 core Session API" + 阶段 4 §1.6 经典视图 + ADR 0001 精神延续._
_7 项 LOCKED 0 触动；Self-Disable 7 项禁止 API 显式列出._
_ponytail 标记：本 ADR 的"cli lib 抽象层"在 core 公开契约稳定后可考虑内联简化；当前为 A1→A20 演化必要投资._