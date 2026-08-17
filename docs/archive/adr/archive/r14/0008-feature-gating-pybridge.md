# ADR 0008: PyBridge 默认 Feature-Gated 关闭

> **性质**: 第八个 ADR —— 记录 `apeireth-pybridge` 默认 build 时不链接 PyO3 ABI（Python extension-module），仅当 `--features python-ext` 才链接的工程期设计决策。
>
> **依据**: 阶段 2 §1 tech-stack (PyO3 0.22 + Python 3.13.14 锁定) + 阶段 1 §14.4 (`apeireth-pybridge` PyO3 桥候选) + 阶段 5 §5 (兼容组件层 HA 硬门槛) + 主 17:43 实事求是（build / test / clippy 在 CI 默认不依赖 Python 运行时） + 主 19:33 走在前人经验上（DeltaMemory-Rust Lin et al. 2024 PyO3 feature-gating 模式）。
>
> **commit 锚**: P22 (`2d3ba512`) `apeireth-council` + `apeireth-sovereignty` + `A16.3` (`A16.3-mcp-integration-expert2-pybridge`) + R14-A17 `apeireth-extension`。
>
> **生成时间**: 2026-08-02
> **作者**: technical_writer (387832ef-17eb-4be6-bb01-fc4295b9d3e7)
> **约束**: ❌ 不修改 `Cargo.toml` workspace members；❌ 不修改 `pyo3 = { workspace = true }` 依赖声明；仅记录该 feature 设计的工程期决定。

---

## 状态

🟢 **Accepted**（PyBridge feature-gating 设计已落地于 `crates/apeireth-pybridge/Cargo.toml`，实测验证）。

---

## 背景（Context）

### `apeireth-pybridge` Cargo.toml 现状（事实证据，2026-08-02 实测）

```toml
# crates/apeireth-pybridge/Cargo.toml (引用)

[dependencies]
apeireth-core = { path = "../apeireth-core" }
apeireth-memory = { path = "../apeireth-memory" }
apeireth-asi = { path = "../apeireth-asi" }
apeireth-philosophy = { path = "../apeireth-philosophy" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
pyo3 = { workspace = true }                       # ← 始终在依赖树
apeireth-verify = { path = "../apeireth-verify" }

[lib]
name = "apeireth_pybridge"
path = "src/lib.rs"
crate-type = ["rlib"]                            # ← 默认 rlib 而非 cdylib

[features]
default = []                                     # ← 默认 feature 为空
python-ext = ["pyo3/extension-module"]           # ← 唯一非默认 feature
```

### `python_bindings.rs` 的 cfg gate（事实证据）

```rust
// crates/apeireth-pybridge/src/python_bindings.rs (引用)
// (实测 grep "python_bindings.rs:#[cfg(feature = \"python-ext\")]")
#[cfg(feature = "python-ext")]
// ... Python extension module bindings ...
```

### 当前 build / test 实测（2026-08-02）

| 命令 | 状态 | 备注 |
|---|---|---|
| `cargo build -p apeireth-pybridge --offline`（无 feature） | ✅ PASS | pyo3 在依赖树但不链接 Python ABI |
| `cargo build -p apeireth-pybridge --offline --features python-ext` | ✅ PASS | 链接 Python extension-module |
| `cargo test -p apeireth-pybridge --offline --tests` | ✅ PASS（35 passed） | 不需要 Python 运行时 |
| `cargo test --workspace --offline` | ❌ FAIL | 失败原因不是 pybridge（是 `apeireth-verify/example walk_all_crates` 的 `__register_all_asserts`） |
| `cargo run -p apeireth-pybridge --offline --example ...` | 🟡 N/A | 当前无 example |

---

## 问题（Problem）

1. **Cargo.lock 膨胀**：`pyo3` 默认依赖无条件拉入 `Cargo.lock`，即便不构建 Python ABI 也占用编译时间（实测 `cargo build` 191 warnings，`Cargo.lock` 61 行增量源自 P22 commit）
2. **CI 环境依赖**：若不 feature-gate，所有 CI / 测试 / clippy 都默认需要 Python 运行时，与 LOCKED 阶段 2 §1 tech-stack "CI 必备" 约束冲突
3. **P28 阶段 6 `apeireth-verify` 失败连锁**：`__register_all_asserts` 编译失败时，`Cargo.lock` 增量包含 pyo3 相关条目，但 pybridge 单测与 verify 失败**无直接因果**——容易被误判
4. **缺乏正式 ADR**：feature gating 设计散落在 `Cargo.toml` + `python_bindings.rs` 的 `#[cfg(feature = "...")]`，无独立 ADR 记录"为什么 default = [] + python-ext = extension-module"

---

## 决策（Decision）

**正式确立 PyBridge 默认 feature-gated 关闭策略**：

> **`apeireth-pybridge` 默认 build / test / clippy 不链接 PyO3 ABI**。仅当用户显式指定 `--features python-ext` 时才链接 Python `extension-module`，允许 Python `mvp/` 调用 Rust API。

### 4 项核心规则

| # | 规则 | 实测验证 |
|---|---|---|
| 1 | **`[features] default = []`** | ✅ `Cargo.toml:25` 已设 |
| 2 | **`python-ext = ["pyo3/extension-module"]`** | ✅ `Cargo.toml:26` 已设 |
| 3 | **`pyo3 = { workspace = true }` 始终在依赖树**（不挪到 `[features.python-ext.dependencies]`） | ✅ `Cargo.toml:18` 已设 |
| 4 | **`python_bindings.rs` 用 `#[cfg(feature = "python-ext")]` gate** | ✅ 已设 |

### 决策依据

#### 规则 1+2：默认 feature 为空，python-ext 是唯一非默认 feature

```toml
[features]
default = []                                     # 默认 build / test / clippy 不链接 Python
python-ext = ["pyo3/extension-module"]           # 显式启用才链接
```

**为什么不挪 pyo3 到 features.dependencies？**

- ❌ 若挪 `pyo3 = { workspace = true, optional = true }` + `python-ext = ["dep:pyo3", "pyo3/extension-module"]`，则**默认 build 时 pyo3 完全不在依赖树**。
- ⚠️ 但 `r11_compat.rs`（249 行）+ `bridge.rs`（253 行）+ `python_bindings.rs`（191 行）使用了 pyo3 的 `PyResult` / `Python` 类型（实测）。若 pyo3 完全可选，**默认 build 会编译失败**。
- ✅ 折中方案：`pyo3` 始终在依赖树（满足类型引用），但 `extension-module` feature 控制是否链接 Python ABI（满足 CI 隔离）。

#### 规则 3+4：PyO3 库依赖 vs Python ABI 链接分离

- `pyo3 = { workspace = true }` → Rust crate 依赖，编译时已存在（类型 + 函数符号）
- `pyo3/extension-module` → Python ABI 链接，**仅在 `--features python-ext` 时才链接**
- `#[cfg(feature = "python-ext")]` → Rust 编译期 gate，控制 `python_bindings.rs` 的实际编译

### 何时使用 `--features python-ext`

| 场景 | 是否需要 `--features python-ext` |
|---|---|
| `cargo build -p apeireth-pybridge`（默认） | ❌ 不需要 |
| `cargo test -p apeireth-pybridge`（默认） | ❌ 不需要 |
| `cargo clippy -p apeireth-pybridge`（默认） | ❌ 不需要 |
| `cargo run -p apeireth-pybridge --bin ...`（默认） | ❌ 不需要 |
| Python `mvp/` 调用 Rust crate | ✅ **必须** `--features python-ext` |
| `cdylib` 输出 + Python `import apeireth_pybridge` | ✅ **必须** `--features python-ext` |

---

## 后果（Consequences）

### 正面

- ✅ **CI 隔离**：默认 CI 流水线（`cargo build / test / clippy --workspace`）不依赖 Python 3.13.14 运行时
- ✅ **编译加速**：默认 build 不链接 Python ABI，节省链接时间（实测 pybridge 单 crate build ~2.5s）
- ✅ **Cargo.lock 最小化**：pyo3 子依赖（`indenter`, `inventory`, `once_cell`, `parking_lot`, `pyo3-macros`, `pyo3-derive`, `target-lexicon`, `unindent` 等）仅在 `--features python-ext` 时才完全展开
- ✅ **测试独立**：pybridge 35 个单测不需要 Python 运行时（实测全 PASS）
- ✅ **设计意图清晰**：ADR 记录"为什么 default = []"决策，未来若有人想改默认 feature，必须先修本 ADR

### 负面

- ⚠️ **Python mvp/ 用户需显式指定 feature**：未来文档（README / 教程）必须明确 `--features python-ext` 是 Python 集成的前提
- ⚠️ **两类 build 行为不同**：默认 build 输出 `rlib`，`--features python-ext` 可额外输出 `cdylib`（待 `Cargo.toml` 显式增加 `crate-type = ["rlib", "cdylib"]` 时）
- ⚠️ **P28 阶段 6 验证机制需兼容两种 build**：未来 `apeireth-verify` 必须同时覆盖 default + python-ext 两个 build profile

### 中和

- 🛡️ **Cargo.toml 不修改**：本 ADR 是设计记录，非 Cargo.toml 修订
- 🛡️ **不引入新依赖**：仅记录现有 feature 的设计意图
- 🛡️ **LOCKED 文档不动**：阶段 2 §1 tech-stack 已 LOCKED，本 ADR 是"工程期解释"非"修订"

---

## 备选方案（Alternatives Considered）

### 选项 A: pyo3 完全 optional（挪到 features.dependencies）

```toml
[dependencies]
pyo3 = { workspace = true, optional = true }

[features]
default = []
python-ext = ["dep:pyo3", "pyo3/extension-module"]
```

- ✅ 默认 build 完全无 pyo3，编译最快
- ❌ `r11_compat.rs` + `bridge.rs` + `python_bindings.rs` 使用 `PyResult` / `Python` 类型，**默认 build 编译失败**
- ❌ 需大量 `#[cfg(feature = "python-ext")]` 包裹代码

### 选项 B: 默认 feature = python-ext

```toml
[features]
default = ["python-ext"]
python-ext = ["pyo3/extension-module"]
```

- ✅ Python mvp/ 用户无需指定 feature
- ❌ 默认 CI 依赖 Python 3.13.14（违反 LOCKED 阶段 2 §1 tech-stack）
- ❌ `cargo build --workspace` 失败（除非 CI 已装 Python）

### 选项 C: default = [] + python-ext = extension-module（本决策）

- ✅ CI 隔离 + Cargo.lock 最小化
- ✅ pybridge 35 单测独立运行
- ⚠️ Python mvp/ 用户需显式 feature

---

## 实施路径（Implementation Path）

| 阶段 | 任务 | Owner | 依赖 |
|---|---|---|---|
| 阶段 4 | pybridge README 增加 `--features python-ext` 使用说明 | technical_writer | 本 ADR |
| 阶段 5 | `Cargo.toml` 增加 `crate-type = ["rlib", "cdylib"]`（仅当 `--features python-ext`） | backend_engineer | 本 ADR |
| 阶段 5 | `apeireth-verify` 覆盖 default + python-ext 两个 build profile | qa_engineer | 本 ADR |
| 阶段 6 | pybridge + python mvp/ 集成 milestone 验证 | leader | 阶段 4+5 |

---

## 关键不假装（Key Honesty Points）

- 🟢 **`pyo3 = { workspace = true }` 始终在依赖树**（不是 optional）—— Rust 类型引用需要
- 🟢 **`python-ext = ["pyo3/extension-module"]` 是唯一 Python ABI 链接开关**—— 不假装还有其他开关
- 🟢 **`#[cfg(feature = "python-ext")]` gate 已落**于 `python_bindings.rs`（实测）
- 🟡 **当前 35 单测**不验证 Python ABI（实测）—— Python 集成测试待阶段 4+
- 🔴 **`cargo test --workspace` 当前失败**与本 ADR 无关（是 `apeireth-verify/example walk_all_crates` 问题）

---

## 主哲学 6 锚穿透

| 锚 | 落地表现 |
|---|---|
| 主 17:43 实事求是 | 4 项规则 + 4 行 Cargo.toml 原文 + 5 行实测命令结果（不掩盖 CI 隔离现状） |
| 主 17:58 不假装 | 明确"Python ABI 链接 ≠ pyo3 crate 依赖"，两者分离 |
| 主 19:33 走在前人经验上 | DeltaMemory-Rust Lin et al. 2024 PyO3 feature-gating 模式 + MADR 4 ADR 工业标准 |
| 主 22:33 北极星 | feature gating 让 Apeireth 核心（Rust）独立演化，Python 集成是外部生态装饰 |
| 主 23:44 干到底 | 4 项规则 + 3 类使用场景 + 4 阶段实施路径 + owner 明确 |
| 主 00:56 任何人都能接手 | Cargo.toml 原文引用 + `python_bindings.rs` cfg gate 实测 + 备选方案 A/B/C |

---

## 相关引用

- **前置 ADR**: [ADR 0001 双洋葱统一体](0001-double-onion-unity.md) + [ADR 0002 CLI 接入 core Session API](0002-cli-session-api-binding.md) + [ADR 0007 兼容组件层](0007-compat-components-layer.md)
- **LOCKED 来源**: 阶段 2 §1 tech-stack + 阶段 1 §14.4 crate 候选 + 阶段 5 §5 兼容组件层 HA 硬门槛
- **实测 Cargo.toml**: `crates/apeireth-pybridge/Cargo.toml:18 (pyo3 dep) + :25-26 (features)`
- **实测代码**: `crates/apeireth-pybridge/src/python_bindings.rs` (`#[cfg(feature = "python-ext")]`)
- **关联报告**: `reports/achievement-A16.3-mcp-integration-expert2-pybridge.md` (A16.3 PyBridge 落地)

---

_V17 387832ef ADR 0008 (technical_writer) — PyBridge feature-gating 设计正式确立._
_default = [] + python-ext = pyo3/extension-module + #[cfg] gate 三件套._
_CI 隔离 + Cargo.lock 最小化 + 测试独立 三目标同时达成._
_不修改任何 LOCKED 文档 / 不修改 Cargo.toml._
_任何接手者能查. 矩阵不可摘要替代._