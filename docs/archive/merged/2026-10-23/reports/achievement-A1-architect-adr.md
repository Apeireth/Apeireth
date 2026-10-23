# A1.4 ADR 成就报告 — CLI 接入 core Session API 架构决策记录

> **角色**: 架构师（architect）
> **成就**: A1.4（CLI ↔ Core Session API 绑定 ADR）
> **日期**: 2026-08-01
> **Task ID**: c41136b8-4680-47c3-bb7b-06b0d28cf1a4
> **依赖**: A1.1（CLI 占位实现已就绪）+ ADR 0001（双洋葱统一体决策，格式参考）

---

## 📋 任务验收（DoD 对照）

| DoD 项 | 状态 | 证据 |
|---|---|---|
| **1. Status: Accepted (2026-08-01)** | ✅ | `docs/adr/0002-cli-session-api-binding.md` §状态：🟢 Accepted（施工团队架构师 2026-08-01） |
| **2. Context: A1 cli session 需要接 core Session API；当前 main.rs 是占位** | ✅ | §背景：列出 `Cargo.toml` 依赖、`lib.rs` 现状（`CliCommand` + `placeholder`）、`main.rs` println! 占位、`core/src/lib.rs` 已暴露 `Session` |
| **3. Decision: lib.rs 暴露 `create_default_session()` + `run_session_action()`；main.rs 通过 lib 调用 core** | ✅ | §决策：明确公开 API 表面（含 Rust 签名草案）、lib.rs 内部职责表、`SessionAction` 枚举、main.rs 绑定规则（允许/禁止 import 列表） |
| **4. Consequences: cli 不依赖 core 实现细节；未来 cli 可换成 gui/web 而不重写 core** | ✅ | §后果（正面）：CLI ↔ core 解耦 / 前端可替换 / 测试隔离 / Self-Disable 边界清晰 / A1→A20 演化平滑 |
| **5. Alternatives Considered: (a) main.rs 直接调 core（耦合） (b) 通过 lib 抽象（采用）** | ✅ | §备选方案：4 个选项详细对比 —— A 直接调 core（否决）/ B 不暴露 lib（否决）/ C 抽 apeireth-binding crate（否决，过度工程）/ D 暴露高层命令字符串（否决，抽象太粗）。其中 A 是任务点名的"备选"，已显式否决 |
| **不修改承诺核查段**：列出 7 项 LOCKED 都未触动 | ✅ | §🛡️ 不修改承诺核查：表格化 7 项 × 三列（是否触动/证据），结论 = 7/7 0 触动 |
| **Self-Disable 边界段**：明确 cli 不暴露"询问是否需要 L0 HA"等违规 API | ✅ | §🛡️ Self-Disable 边界：❌ 禁止 API 7 项（`is_l0_ha_required` / `disable_self` / `reorganize_onion` / `modify_l0_layer` / `bypass_guard` / `exit_silent_mode` / `fake_human_approval`）+ ✅ 允许 API 4 项 + 边界守护机制 3 条 |
| **参考 ADR 0001 格式风格** | ✅ | 章节顺序 = 状态 → 背景 → 决策 → 后果（正面/负面/中和）→ 备选方案 → 实施（已落地/进行中/未来）→ 参考 → 决策者；中文标题；emoji 标记；引用块 |
| **报告**: `reports/achievement-A1-architect-adr.md` | ✅ | 即本文件 |

**结论**：8 项 DoD 全部通过。✅

---

## 📐 关键架构决策摘要

### 决策一句话

> **`apeireth-cli/src/lib.rs` 是 CLI 的唯一公开 API 表面；`main.rs` 只通过 `apeireth_cli::*` 调用，绝不直接 `use apeireth_core::*`。**

### 公开 API 表面（lib.rs 新增）

```rust
pub fn create_default_session() -> Session;
pub fn run_session_action(session: &Session, action: SessionAction) -> Result<SessionResult, CliError>;
pub enum SessionAction { Start, ListEpisodes, RunV1136Benchmark }
```

### main.rs 绑定规则

- ✅ 允许：`use apeireth_cli::{CliCommand, create_default_session, run_session_action};`
- ❌ 禁止：`use apeireth_core::*` 或 `use apeireth_core::Session as CoreSession;`
- ❌ 禁止：绕过 `run_session_action` 直接调 core 的守门方法

---

## 🛡️ 7 项 LOCKED 不修改承诺核查（详细）

| # | 不修改项 | 本 ADR 触动 | 证据（具体路径 / 行号） |
|---|---|---|---|
| 1 | 阶段 1+2+3 LOCKED（54 份） | ✅ 未触动 | 本 ADR 仅新增 `docs/adr/0002-*.md`；不修改任何 `docs/stage1/`、`docs/stage2/`、`docs/stage3/` |
| 2 | v2 / v4 / v4.1 LOCKED（哲学层纲领）| ✅ 未触动 | 本 ADR 不涉及 `APEIRETH-*-v2.md`、`APEIRETH-*-v4.md`、`APEIRETH-*-v4.1.md` |
| 3 | 阶段 4 主文档 LOCKED（1492 行）| ✅ 未触动 | 不修改 `docs/stage4/stage4-runtime-architecture-revised.md` 等阶段 4 文件；只引用阶段 4 §1.6 经典视图 |
| 4 | 阶段 5 施工文档 LOCKED（631 行）| ✅ 未触动 | 不修改 `docs/stage5/stage5-construction-document.md`；只引用 START-CONSTRUCTION.md（已上提顶层）作为权威纪律来源 |
| 5 | v6 修正（4 重守门 + 权限发放 + E 层修改路径）| ✅ 未触动 | Self-Disable 边界段引用 v6 §A-E 五大禁令原文，不修改任何 v6 修正链文件 |
| 6 | R11 baseline 三值 LOCKED（V1141=0.8682 / V1131=0.8532 / V1136=0.9063）| ✅ 未触动 | 本 ADR 与 R11 baseline 三值无关；A1 占位不影响真测数值 |
| 7 | v1 → v5 历史链 LOCKED（保留，不删除）| ✅ 未触动 | 本 ADR 不涉及任何 v1-v5 历史版本文件 |

**核查结论**：7/7 全部 0 触动。✅

---

## 🛡️ Self-Disable 边界（cli lib.rs 禁止 API 清单）

### ❌ 禁止 API（7 项）

1. `is_l0_ha_required()` — 元问题禁令（§A）
2. `disable_self()` — 自我禁用禁令（§E）
3. `reorganize_onion()` — 重组洋葱结构禁令（§B）
4. `modify_l0_layer()` — L0 直接修改禁令（§C）
5. `bypass_guard()` — 守门绕过禁令（5 重守门）
6. `exit_silent_mode()` — 强制退出 Offline/冰冻模式禁令（§D）
7. `fake_human_approval()` — 伪造多签禁令（v6 修正）

### ✅ 允许 API（4 项）

1. `create_default_session()` — 构造 Session
2. `run_session_action(s, action)` — 执行 SessionAction
3. `SessionAction::{Start, ListEpisodes, RunV1136Benchmark}` — 公开动作枚举
4. `CliError` — 公开错误类型

### 边界守护机制（3 条）

- 本 ADR 锁定边界清单 → 任何新增 API 必须先核对清单 + ADR 修订
- 集成测试反向验证 → 尝试调禁止 API 必须编译失败（A3 硬编码机制延伸）
- 代码审查守门 → code_reviewer 必须强制检查新增 API 是否触碰禁止清单

---

## 📂 产出文件清单

| 文件 | 字节数 | 状态 |
|---|---|---|
| `docs/adr/0002-cli-session-api-binding.md` | 14607 bytes | 🆕 新增 |
| `reports/achievement-A1-architect-adr.md` | （本文件） | 🆕 新增 |

**未修改文件清单**（确认遵守任务约束）：

- ❌ 未修改 `docs/adr/0001-double-onion-unity.md`
- ❌ 未修改任何 `docs/stage1/`、`docs/stage2/`、`docs/stage3/`、`docs/stage4/`、`docs/stage5/` 设计文档
- ❌ 未修改 `crates/apeireth-cli/src/main.rs`（本 ADR 只下决策，由 backend_engineer 落地）
- ❌ 未修改 `crates/apeireth-cli/src/lib.rs`（同上）
- ❌ 未修改 `crates/apeireth-core/src/lib.rs`
- ❌ 未修改 `START-CONSTRUCTION.md`、`Cargo.toml`、`Cargo.lock`

---

## 🔄 下游交接（next steps）

### 立即执行（A1.5+，交给 backend_engineer）

1. **`apeireth-cli/src/lib.rs`** 新增：
   - `create_default_session() -> Session`
   - `run_session_action(session: &Session, action: SessionAction) -> Result<SessionResult, CliError>`
   - `SessionAction` 枚举 + `SessionResult` 类型 + `CliError` 枚举

2. **`apeireth-cli/src/main.rs`** 重构：
   - `CliCommand::Session` 分支：调 `create_default_session()` + `run_session_action(s, SessionAction::Start)`
   - 其他分支类似改造
   - 移除 println! 占位文案（保留帮助信息）

3. **`tests/integration_session_lifecycle.rs`** 验证：
   - 必须通过 lib.rs 公开路径走通 CLI → Session → Core
   - 尝试调禁止 API 必须编译失败

### 验证命令

```bash
cargo check --workspace                     # 必须 0 error
cargo test --workspace                     # 必须 8 tests + 新增 CLI lib 测试全绿
cargo run -p apeireth-cli -- session       # 输出应来自 core Session 真实构造，而非 println! 占位
```

### 漂移防护（如发现不一致）

如 backend_engineer 落地时发现本 ADR 与 core 实际公开 API 不一致，立即通过 leader 提交 `reports/drift-A1-4-<日期>.md` 并暂停施工，等待主人拍板。

---

## 🏷️ ponytail 简化标记

- **抽象层天花板**：cli lib 抽象层在 core 公开契约稳定后（A20 后）可考虑内联简化；当前为 A1→A20 演化必要投资。
- **升级路径**：
  - 阶段 7+ frontend ≥ 2 个复用 binding 逻辑 → 抽出 `apeireth-binding` crate
  - core 暴露 `Action` enum 后 → cli `SessionAction` 用 type alias 桥接

---

## 📜 签字

- **架构师**: 2026-08-01 起草本 ADR + 本报告
- **下游责任**: backend_engineer（落地 lib.rs + main.rs 重构）
- **审查责任**: code_reviewer（对照禁止清单审查新增 API）
- **守护责任**: 主人（如发现本 ADR 与设计层 LOCKED 不一致，立即与架构师沟通）

---

_本报告由架构师角色在 A1.4 任务完成后产出（A1.4 子成就）. 
_任务 ID: c41136b8-4680-47c3-bb7b-06b0d28cf1a4.
_8/8 DoD 通过；7/7 LOCKED 0 触动；7 项禁止 API 显式列出._
_等待 team_complete_task + team_report_idle 提交后进入待命._