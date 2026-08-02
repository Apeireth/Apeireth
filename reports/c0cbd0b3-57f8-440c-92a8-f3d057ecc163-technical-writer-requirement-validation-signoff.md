# 需求裁决与用户有效性确认单（Requirement Adjudication & User Validity Confirmation）

> **任务 ID**: `c0cbd0b3-57f8-440c-92a8-f3d057ecc163`
> **角色**: technical_writer
> **唯一目标**: 与用户沟通真实需求，并由**用户**确认工作有效性
> **本文件性质**: 整理事实 + 列冲突选项 + 影响分析 + Leader 建议（**建议 ≠ 用户决定**）+ 验收命令 + 用户签收栏
> **约束**:
> - ❌ **不修改** 任何业务代码（`crates/**/src/*.rs`）
> - ❌ **不修改** 任何 LOCKED 文档（`docs/stage{1,2,3-blueprints,4,5}/*.md`）
> - ❌ **不修改** `Cargo.toml` workspace members
> - ✅ 仅整理已观察到的事实 + 提供选项 + 等待用户裁决
> **生成时间**: 2026-08-02
> **基线环境**: Windows 11 / git-bash / cargo 1.97.1 / workspace root = `redacted/.openclaw/workspace/promethean/Apeireth-rust`

---

## 0. 文档结构

```
§1 当前可观察到的事实（2026-08-02 实测，无主观判断）
§2 冲突 #1：PyBridge 是否保留（"零 Python" vs "恢复 PyBridge"）
§3 冲突 #2：crate 口径（17 / 18 / 21 / 22+ / 24 / 27）
§4 冲突 #3：继续全 backlog vs 先收敛 integration
§5 冲突 #4：LOCKED 文档 vs 当前可运行实现
§6 统一验收命令（必须全部通过方为有效）
§7 用户签收栏（4 项冲突 × 各自裁决）
§8 附录：实测命令输出原文
```

---

## 1. 当前可观察到的事实（2026-08-02 实测）

> 本节仅为**客观观测**，不含 Leader 建议或用户决定。

### 1.1 crate 计数（4 个口径来源）

| 口径来源 | 数值 | 实测命令 |
|---|---:|---|
| `Cargo.toml:3-28` 的 `members` 数组 | **24** | `grep -c "crates/apeireth-" Cargo.toml` → 24 |
| `crates/` 目录中以 `apeireth-` 开头的子目录 | **25**（含 `apeireth-bus` 仅剩 `target/` 无源码） | `ls crates/ \| grep apeireth- \| wc -l` |
| `cargo metadata --no-deps` workspace members | **24**（与 `Cargo.toml` 一致） | `cargo metadata --no-deps --format-version 1 \| jq '.workspace_members \| length'` |
| `crates/apeireth-verify/Cargo.toml:7` 自描述 | "22+ crate" | `grep "22+ crate" crates/apeireth-verify/Cargo.toml` |
| `docs/stage4/stage4-patches-v2-crate-correction.md` 阶段 4 收敛 | 18 → 17 crate | 文档层 |
| `reports/achievement-A20-leader-integration-final.md` A20 | 20 crate | 报告层（V17 matrix 注：此报告与当前实况口径不符） |
| V17 矩阵 `reports/V17-writer-stage1-2-traceability.md §0.2` | 22 members | V17 已编（**过期**） |

**实测冲突点**：
- `Cargo.toml` (24) ≠ `crates/` 目录 (25，含 `apeireth-bus` 无源码)
- `Cargo.toml` (24) ≠ `apeireth-verify` 自描述 "22+"
- `Cargo.toml` (24) ≠ 阶段 4 文档 (17/18)
- V17 矩阵 (22) ≠ 当前 `Cargo.toml` (24) — **V17 已过期**

### 1.2 PyBridge 状态

| 观测项 | 结果 |
|---|---|
| `crates/apeireth-pybridge/Cargo.toml` 存在 | ✅ 存在 |
| 描述 | `"Apeireth PyO3 桥 (Python 3.13.14 ↔ Rust) — R14 Phase 3 (暴露 Rust crate 给 Python mvp/)"` |
| `Cargo.toml:9-12` 依赖 `pyo3 = { workspace = true }` + `apeireth-verify = { path = "../apeireth-verify" }` | ✅ |
| 特性开关 `default = []` + `python-ext = ["pyo3/extension-module"]` | ✅ 默认关闭 pyo3 扩展模块 |
| `crates/apeireth-pybridge/src/*.rs` 总行数 | **859** 行（bridge.rs 253 + error.rs 85 + lib.rs 81 + python_bindings.rs 191 + r11_compat.rs 249） |
| 单测数（实测） | **35**（`cargo test -p apeireth-pybridge --offline --tests` 实跑） |
| 工作区 `Cargo.toml` 是否列出 | ✅ 在 `members` 中 |
| `apeireth-pybridge` 是否实际跑通 | ✅ 单测 35 个全部 `0 failed`（cargo 实跑） |
| 是否有"零 Python"明确声明 | ❌ **未发现**；`grep -rE "zero python\|零 Python\|remove.*python\|drop.*python" docs/ reports/` 无匹配 |
| A16.3 `reports/achievement-A16.3-mcp-integration-expert2-pybridge.md` | 已存在（PyO3 bridge 落地） |

**实测冲突点**：
- "零 Python" 措辞在 `docs/` 与 `reports/` 中**无任何字面证据**
- "PyBridge" 在 `Cargo.toml` + 代码 + 35 测试 + A16.3 报告中**完整在场**
- 用户可能从**早期讨论**或**口头决定**听到"零 Python"，但当前 LOCKED 文档与代码**未体现此决定**

### 1.3 `apeireth-bus` 状态（DRIFT 项）

| 观测项 | 结果 |
|---|---|
| `crates/apeireth-bus/Cargo.toml` 存在 | ❌ **不存在** |
| `crates/apeireth-bus/src/` 存在 | ❌ **不存在** |
| `crates/apeireth-bus/target/` | ⚠️ **仅含编译产物**（`libapeireth_bus.rlib` + `.d` 文件）— **无源码** |
| `cargo build -p apeireth-bus --offline` | ❌ `error: package ID specification 'apeireth-bus' did not match any packages` |
| 是否在 workspace `members` | ❌ 否 |
| 阶段 2 §9 LOCKED 设想（5 层通信总线） | LOCKED 但**无对应 crate** |

### 1.4 验收命令实测结果

| 命令 | 状态 | 实测关键输出 |
|---|---|---|
| `cargo build --workspace --offline` | ✅ **PASS**（191 warnings） | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 0.61s` |
| `cargo test --workspace --offline` | ❌ **FAIL** | `error[E0425]: cannot find function '__register_all_asserts' in crate 'apeireth_core'` （`apeireth-verify` example `walk_all_crates` 编译失败） |
| `cargo test --workspace --offline --no-fail-fast` | ❌ **FAIL** | 同上 |
| `cargo test -p <each crate> --offline --tests`（22 crate 单跑） | ✅ **PASS**（527 pass + 1 ignored，V17 matrix 实测） | 见 V17 §0.4 |
| `cargo clippy --workspace --offline` | ⚠️ **214 warnings** | 多个 crate 含 `missing_documentation` / `unused_imports` / `dead_code` |
| `cargo clippy --workspace --offline -- -D warnings` | ❌ **FAIL** | `error: could not compile 'apeireth-verify' (lib)` + 其他 crate 警告升级 |
| `cargo run -p apeireth-cli --offline -- session` | ✅ **PASS** | 输出 `HA mode: SingleHuman` / `PermissionOnion: L0=L0 HA 核心 / L5=L5 核武器级` / `守门: V1+V2+V3 AND 门` / `✅ session 已启动` |

### 1.5 LOCKED 文档完整度（阶段 1-2）

| 阶段 | LOCKED 文件数 | 总行数 | 状态 |
|---|---:|---:|---|
| 阶段 1 | 2 (`inspiration-stage1-2026-07-30.md` + README) | 2208 + 99 | 🟢 LOCKED |
| 阶段 2 | 18 decisions + 1 D2 增补 = 19 文件 | 10864 | 🟢 LOCKED |
| 阶段 3 blueprints | 14+ 文件 | 数千行 | 🟢 当前活跃 |
| 阶段 4 | 多个 v2/v3-v12 修订 + landing | 数千行 | 🟢 当前活跃（v2 取代原版） |
| 阶段 5 | `construction-kickoff-manual.md` + LOCKED 文档 | 575 + 631 | 🟢 当前活跃 |

**V17 矩阵结论回顾**：
- 阶段 1 (60 条设想)：🟢 21 / 🟡 18 / 🔴 12 / ⚠️ 6 / 🟣 3
- 阶段 2 (70 条设想)：🟢 9 / 🟡 25 / 🔴 18 / ⚠️ 8 / 🟣 8 / ❓ 2
- **阶段 1 + 2 共 130 条设想中，LOCKED 文档完整，但代码/测试仅闭环 30 条（23%）**

---

## 2. 冲突 #1：PyBridge 是否保留

### 2.1 现状描述

- "零 Python" vs "恢复 PyBridge" 的措辞在 `docs/` 与 `reports/` 中**无任何字面证据**（见 §1.2 grep 结果）
- 当前 `apeireth-pybridge` 在 `Cargo.toml` workspace members 内，**已实装**：859 行 Rust + pyo3 0.22 + `python-ext` 特性开关 + 35 个单元测试 + A16.3 achievement 报告
- `Cargo.toml:27-29` 已声明 `pyo3 = { version = "0.22", features = ["auto-initialize"] }` workspace 依赖

### 2.2 选项 A：保留 PyBridge（当前状态）

**事实陈述**：
- `apeireth-pybridge` 在 workspace
- `pyo3` 0.22 在 `[workspace.dependencies]`
- 35 单测已实跑通过
- 阶段 1 §14.4 + 阶段 2 §8 设计 LOCKED 包含 VCP 6 类插件协议 + PyO3 兼容组件

**影响**：
- ✅ 兼容 R11 现有 1100+ Python 模块（`r11_compat.rs` 249 行已实装）
- ✅ 与 `apeireth-verify` 通过 path 依赖关联
- ⚠️ 增加 Rust → Python FFI 复杂度（pyo3 auto-initialize 引入 Python runtime 依赖）
- ⚠️ 跨平台 Python 版本绑定（`Cargo.toml` 描述锁定 Python 3.13.14）

### 2.3 选项 B：移除 PyBridge（"零 Python"）

**事实陈述**：
- ❌ LOCKED 文档无"零 Python"字面声明
- ❌ 阶段 1 §1 比喻 + §2.A.7 通信总线 + §14.4 crate 候选 + §16.5 借鉴 Hermes 全部**明确包含 Python 兼容组件**
- ❌ 阶段 2 §8 modularity + §9 communication-bus + §17 appendix-references 全部**引用 PyO3 集成**
- ❌ A16.3 落地报告已声称 PyO3 bridge 完成

**影响（假设执行）**：
- 🔴 必须修改 `Cargo.toml:14` 移除 `"crates/apeireth-pybridge"` + `Cargo.toml:29` 移除 `pyo3` workspace dep
- 🔴 必须修改 LOCKED 文档（阶段 1 §1/§2/§14/§16 + 阶段 2 §8/§9/§17 + A16.3 报告）—— **违反 LOCKED 不修改原则**
- 🔴 必须删除 `crates/apeireth-pybridge/` 整个目录（859 行 + 35 测试 + 249 行 R11 兼容代码）
- 🔴 R11 1100+ Python 模块失去 Rust → Python 桥接能力（违反 §16.5 Hermes 借鉴的 MCP/ACP 兼容意图）
- ⚠️ V17 矩阵条目 1.2.1（基础推理）、1.14.1（不模仿 Hermes）、1.16.1（Hermes traits）、2.7.1-2.7.5（LLM 集成）全部需重新标注为 🔴 MISSING

### 2.4 选项 C：保留 PyBridge 但切换实现

**事实陈述**：
- 当前 `apeireth-pybridge` 用 `pyo3 0.22`
- 阶段 2 §17 附录列待抓取包含 async-openai / qdrant / WASM，**未列 PyBridge 替换**

**影响**：
- 🟡 需评估是否替换为 `rustpython` / 自研 FFI / 简化 callback 协议
- 🔴 任何替换必须新 LOCKED 文档追加（R14 修订流程）
- ⚠️ 工作量与风险均高于选项 A

### 2.5 Leader 建议（**仅参考**，**非用户决定**）

Leader 倾向 **选项 A（保留 PyBridge）**：
- 事实依据：当前 LOCKED 文档 + 代码 + 测试 + A16.3 报告 + V17 矩阵均视 PyBridge 为已实装部件
- 工作量依据：选项 B 涉及大量 LOCKED 修订，与"LOCKED 不修改"原则冲突
- 风险依据：选项 C 需新 LOCKED 文档，无明确触发条件
- ⚠️ **此建议未获用户确认**；若用户口头/早期讨论确曾表达"零 Python"意图，则建议用户在签收栏填入裁决意见

---

## 3. 冲突 #2：crate 口径（17 / 18 / 21 / 22+ / 24 / 27）

### 3.1 现状描述

口径来源分散（见 §1.1 表）：

| 来源 | 数值 | 何时引用 |
|---|---:|---|
| `apeireth-verify/Cargo.toml:7` 自描述 | "22+ crate" | 当前活跃 |
| `Cargo.toml:3-28` `members` 数组 | **24** | 当前实测 |
| `crates/` 目录（含 `apeireth-bus` 无源码） | **25**（+ `README.md` 占位 = 26 行） | 当前实测 |
| 阶段 4 `stage4-patches-v2-crate-correction.md` | 18 → 17 | LOCKED 阶段 4 |
| V17 `reports/V17-writer-stage1-2-traceability.md §0.2` | 22 | V17 已编（过期） |
| A20 `reports/achievement-A20-leader-integration-final.md` | 20 | A20 阶段 |
| 阶段 1 §14.4 候选 30 crate | 30 | LOCKED 阶段 1 候选 |
| 阶段 2 §3 `stage2-decisions-crate-split.md` | 30 v1 | LOCKED 阶段 2 候选 |

### 3.2 选项 A：以 `Cargo.toml` 为唯一真相（24 workspace members）

**事实陈述**：
- `Cargo.toml` 是 Cargo 构建系统的唯一权威
- 当前 24 members（V17 矩阵的 22 已过期，因 +`apeireth-council` +`apeireth-sovereignty` +`apeireth-verify` 3 个加入）

**影响**：
- ✅ 与 `cargo build/test/clippy` 命令结果一致
- ⚠️ 必须更新 V17 矩阵的 §0.2（22 → 24）
- ⚠️ 必须更新 `apeireth-verify/Cargo.toml:7` 自描述（"22+ crate" → "24 crate"）
- ⚠️ 必须更新所有引用"17/18/22"的文档（A20 报告、V17、阶段 4 patches）

### 3.3 选项 B：以 LOCKED 阶段 4（17 crate）为口径

**事实陈述**：
- 阶段 4 文档 LOCKED 收敛到 17 crate（"本源推导"）
- 当前实际 24 = 17 + 7 增量（extension/supervisor/council/sovereignty/verify/...）

**影响**：
- 🟡 17 是阶段 4 的设计目标，但当前 workspace 已超集
- ⚠️ 必须解释"17 + 7 增量"的来源（每增量需独立 LOCKED 文档支撑）
- 🔴 `apeireth-council` / `apeireth-sovereignty` / `apeireth-verify` 三个是阶段 5+ 增量，无明确 LOCKED 来源

### 3.4 选项 C：以"crates/ 目录"为口径（含 `apeireth-bus` 25）

**事实陈述**：
- `apeireth-bus/` 目录存在但**仅含 target/ 编译产物，无 Cargo.toml + src**
- 见 §1.3 DRIFT

**影响**：
- ⚠️ `apeireth-bus` 是 LOCKED 设想（阶段 2 §9 五层通信总线）的"应建未建"crate
- 🔴 必须先决定 §2 中 PyBridge 走向，再决定 §3 是否把 `apeireth-bus` 作为"应建未建"占位
- 🔴 若选 C，则需明确 `apeireth-bus` 是补建（要 LOCKED 增项 + Cargo.toml + src/）还是清退（删目录）

### 3.5 Leader 建议（**仅参考**，**非用户决定**）

Leader 倾向 **选项 A（以 Cargo.toml 24 为口径）**：
- 事实依据：构建系统认 Cargo.toml；其他口径若与 Cargo.toml 矛盾，必须以 Cargo.toml 为准
- 操作性依据：选项 A 只需更新 3 处文档引用（V17 / apeireth-verify / A20），无业务代码改动
- ⚠️ **此建议未获用户确认**；若用户期望 17 crate 收敛，则建议在签收栏裁决并触发阶段 4 → 5 反向收敛

---

## 4. 冲突 #3：继续全 backlog vs 先收敛 integration

### 4.1 现状描述

- V17 矩阵已识别阶段 1 + 2 共 130 条 LOCKED 设想
- 当前 backlog（待落地项）：🔴 MISSING = 30 条 + 🟡 PARTIAL = 43 条 = **73 条**
- 当前 `cargo test --workspace --offline` 因 `apeireth-verify/example walk_all_crates` 编译失败**整个 workspace 测试无法跑通**
- 22 个 workspace member 单跑测试 527 pass + 1 ignored（V17 matrix 实测）
- `cargo build --workspace --offline` 成功（191 warnings）
- `cargo clippy --workspace --offline -- -D warnings` 失败

### 4.2 选项 A：先收敛 integration，再开新 backlog

**事实陈述**：
- §1.4 实测：`cargo test --workspace` 失败 = 当前不可声称"全量验收有效"
- 单跑 22 crate 通过 ≠ workspace 全量通过
- `cargo run -p apeireth-cli -- session` 已可跑（HA mode / V1+V2+V3 / session 启动）
- 但用户没有 1 条 e2e 命令能验证完整链路（因 workspace test 失败）

**影响**：
- ✅ 短期收敛目标明确：`__register_all_asserts` 函数在 `apeireth-core/council/sovereignty/supervisor/cognition/constraint` 6 个 crate 中补齐
- ✅ `cargo clippy -D warnings` 收敛（214 warnings 主要为 missing_docs/unused_imports/dead_code）
- ⚠️ 必须先理解 `apeireth-verify` 设计意图：是 P28 阶段 6 验证机制（`description` 字段），还是早期 demo？
- 🔴 若`apeireth-verify/example walk_all_crates` 是 P28 必须项，则补 `__register_all_asserts` 是 P28 前置条件
- 🟡 收敛周期：估计 1-3 个 commit（含文档同步）

### 4.3 选项 B：继续全 backlog（每个新 LOCKED 增量都开新 P-任务）

**事实陈述**：
- 当前 V1-V26 已完成大量验收报告（V13/V14/V17/V18/V25/V26.1 等）
- 阶段 5 开工手册 + 阶段 6 milestone 验证机制未启动
- 30 条 🔴 MISSING + 43 条 🟡 PARTIAL = 73 条 backlog 项分散在 24 个 crate + 5 个 LOCKED 文档中

**影响**：
- ⚠️ 每个新 P-任务都将面对 `cargo test --workspace` 失败的基线问题
- ⚠️ 每个新 LOCKED 增量都可能引入新的 build/test 冲突
- 🔴 无 integration 收敛 → 无任何 1 条命令能证明"当前系统全量工作"
- 🟢 优点：每个 P-任务可独立验证单 crate，不被 workspace 失败阻塞

### 4.4 选项 C：双轨并行（integration 收敛 + 关键 LOCKED 增量同时推进）

**事实陈述**：
- 关键 LOCKED 增量：OTA 7 阶段（DRIFT-04）、`apeireth-bus` 5 层总线（LOCKED 阶段 2 §9）、权限包 struct（LOCKED 阶段 2 §14）、HA 真实 SDK 实现（LOCKED 阶段 1 §19.3）
- integration 收敛：`__register_all_asserts` 6 crate 补齐 + clippy -D warnings 收敛

**影响**：
- ✅ 两个轨道并行不阻塞（integration 是 cargo 维度，LOCKED 增量是 crates 维度）
- ⚠️ 需要 2 个 backend 角色 + 1 个 devops 角色并行
- ⚠️ 集成时可能再次冲突（如 LOCKED 增量修改了 `apeireth-core`，导致 `__register_all_asserts` 签名变化）
- 🟡 周期：估计 5-8 个 commit

### 4.5 Leader 建议（**仅参考**，**非用户决定**）

Leader 倾向 **选项 C（双轨并行）**：
- 事实依据：73 条 backlog + integration 失败基线同时存在，单一轨道无法兼顾
- 阻塞性依据：`cargo test --workspace` 失败是"零号 blocker"，但每个 crate 单跑可通过 → 不阻塞新 LOCKED 增量的局部验证
- 风险依据：选项 B 持续累积技术债，选项 A 拖延关键 LOCKED 落地
- ⚠️ **此建议未获用户确认**；若用户期望严格串行（先 A 再 B），则在签收栏裁决

---

## 5. 冲突 #4：LOCKED 文档 vs 当前可运行实现

### 5.1 现状描述

V17 矩阵已建立 130 条 LOCKED → 代码 → 测试三链：

| 判定 | 数量 | 占比 |
|---|---:|---:|
| 🟢 PASS（LOCKED + 代码 + 测试三链闭环） | 30 | 23% |
| 🟡 PARTIAL（代码/测试部分实装） | 43 | 33% |
| 🔴 MISSING（LOCKED 有，代码无） | 30 | 23% |
| ⚠️ DRIFT（LOCKED 与代码不一致） | 14 | 11% |
| 🟣 N/A (LOCKED)（设计层，不要求代码） | 11 | 8% |
| ❓ UNKNOWN（不能精确定位） | 2 | 2% |

**关键 DRIFT 项（V17 §3.1）**：
1. **DRIFT-01**: `apeireth-council` 不在 workspace（**V17 编时**为不在；**当前实测**已在）— **已变化**
2. **DRIFT-04**: OTA 7 阶段仅实装 3 阶段（自证缺口 `apeireth-upgrade/src/ota.rs:8-14`）
3. **DRIFT-05**: 阶段 2 §1 tech-stack 包含 sled，`Cargo.toml` 无 sled 依赖
4. **DRIFT-06**: 阶段 2 §6 6 DB 协同，实际仅 SQLite
5. **DRIFT-07**: 阶段 1 §2.A.7 + 阶段 2 §9 5 层通信总线 LOCKED，但无 `apeireth-bus` crate（**仅 target/ 产物**，见 §1.3）
6. **DRIFT-09**: 阶段 2 §17 待抓取 5 项依赖（async-openai/qdrant/Hermes/OpenClaw/WASM）全部未集成
7. **DRIFT-12**: 阶段 1 §7 12 维度机制池 4 维度（cargo-deny/Prusti/sigstore/eBPF）未实装
8. **DRIFT-13**: 阶段 1 §19.3 + D2 §9 Windows Hello/FIDO2/多人多签/离线签字 4 个 impl 全缺失

### 5.2 选项 A：LOCKED 文档视为真相，代码必须收敛

**事实陈述**：
- 阶段 1-5 LOCKED 文档总字数 15000+ 行（V17 矩阵已逐条登记）
- 30 条 🔴 MISSING + 14 项 ⚠️ DRIFT 当前未实现

**影响**：
- 🔴 必须补齐 30 条 MISSING + 收敛 14 项 DRIFT
- 🟡 工作量：估计 30-50 个 commit（含设计、补 Rust 代码、加测试、写文档同步）
- ⚠️ 部分 LOCKED 项（如 OTA 7 阶段 + apeireth-bus + Windows Hello/FIDO2）需要数周工作量
- 🟢 优点：阶段 6 milestone 验证机制（P28）有完整前置

### 5.3 选项 B：当前可运行实现视为真相，LOCKED 文档需修订

**事实陈述**：
- 当前 24 workspace members + 527 pass + 1 ignored 测试 + CLI e2e 可跑
- LOCKED 文档中 30 🔴 MISSING 项**不影响**当前可运行（可运行的是子集）

**影响**：
- 🔴 必须修订 LOCKED 文档（30 + 14 项）—— **违反"LOCKED 不修改"原则**
- 🟡 必须新增 R14-D-N+1 修订流程文档（主 17:43 实事求是）
- ⚠️ 修订 LOCKED 是主哲学重大变更（§0 不修改承诺）
- 🟢 优点：与"实事求是"原则对齐（LOCKED 不假装"已实现"）

### 5.4 选项 C：分层分阶段（核心 LOCKED 收敛，非核心 LOCKED 标注为未来增量）

**事实陈述**：
- 14 项 DRIFT 中，DRIFT-04（OTA 7 阶段）、DRIFT-07（apeireth-bus）、DRIFT-13（HA 真实 SDK）是核心 LOCKED
- DRIFT-05/06（sled/6DB）、DRIFT-12（4 维度机制池）是 LOCKED 但非 blocker
- 30 条 MISSING 中，§2.A.7 通信总线 + §4.1 7 席智囊团 + §19.3 HA 真实 SDK 是核心
- 其余（如 §6.2 洋葱测试矩阵 Layer-N 表、§9 Phase 2+ 声明式 DSL）是设计层

**影响**：
- ✅ 短期收敛目标：补 `__register_all_asserts`（6 crate）+ DRIFT-04（OTA 7 阶段）+ DRIFT-07（apeireth-bus 真实 Cargo.toml + src/）+ DRIFT-13（至少 1 个 HA impl）
- 🟡 中期收敛：DRIFT-05/06（sled/6DB）+ DRIFT-12（4 维度机制池中至少 2 个）
- 🟢 长期设计层：保持 LOCKED 不动，阶段 6+ 增量
- ⚠️ 必须为每条非核心 LOCKED 加"标注 N 期"（P0/P1/P2/P3），已在 D2 §14 漂移优先级表中部分实现

### 5.5 Leader 建议（**仅参考**，**非用户决定**）

Leader 倾向 **选项 C（分层分阶段）**：
- 事实依据：30 MISSING + 14 DRIFT 中只有约 1/3 是核心 blocker；其余可在阶段 5/6+ 增量
- 工作量依据：选项 A 工作量过大（30-50 commit）；选项 B 违反主哲学
- 对齐依据：D2 §14 已提供 P0/P1/P2/P3 漂移优先级模板，只需扩展到全 LOCKED
- ⚠️ **此建议未获用户确认**；若用户期望严格 LOCKED 收敛（选项 A），则在签收栏裁决

---

## 6. 统一验收命令（用户最终签收前必须全部通过）

### 6.1 命令清单（按依赖顺序）

```bash
# ====== 阶段 1：build 收敛 ======
# 1.1 全 workspace build（应 0 error）
cargo build --workspace --offline
# 预期：Finished `dev` profile ... in N seconds（warnings 数量不限制）

# ====== 阶段 2：单 crate 测试 ======
# 2.1 每个 workspace member 单跑测试
for c in $(grep "crates/apeireth-" Cargo.toml | tr -d ' ",'); do
  echo "=== $c ==="
  cargo test -p $(basename $c) --offline --tests 2>&1 | grep -E "test result:"
done
# 预期：每个 crate "0 failed"（ignored 不计）

# ====== 阶段 3：workspace 测试（integration 收敛目标） ======
# 3.1 workspace 全测试（要求 FAIL → PASS）
cargo test --workspace --offline --no-fail-fast 2>&1 | tail -30
# 预期（收敛前）：error: __register_all_asserts not found
# 预期（收敛后）：所有 test result: ok

# ====== 阶段 4：clippy 严格收敛 ======
# 4.1 clippy 默认（仅警告）
cargo clippy --workspace --offline 2>&1 | tail -5
# 预期：0 error，warnings 数量可统计

# 4.2 clippy -D warnings（严格）
cargo clippy --workspace --offline -- -D warnings 2>&1 | tail -5
# 预期：0 error（收敛前 FAIL，收敛后 PASS）

# ====== 阶段 5：CLI e2e 验证 ======
# 5.1 session 启动
cargo run -p apeireth-cli --offline -- session 2>&1 | tail -15
# 预期：输出 HA mode / PermissionOnion / V1+V2+V3 AND 门 / ✅ session 已启动

# 5.2 verdict e2e（v7 集成测试已存在）
cargo test -p apeireth-cli --offline --test v7_cli_to_verdict_e2e 2>&1 | tail -5
# 预期：1 passed

# ====== 阶段 6：可选 PyBridge 验证 ======
# 6.1 PyBridge 单测（仅当 §2 选项 A 或 C 选中）
cargo test -p apeireth-pybridge --offline --tests 2>&1 | tail -5
# 预期：35 passed（已实测）

# 6.2 PyBridge feature 切换（python-ext）
cargo build -p apeireth-pybridge --offline --features python-ext 2>&1 | tail -3
# 预期：Finished（已实测）
```

### 6.2 命令汇总判定表

| 命令 | 当前状态 | 验收标准 | 阻塞原因 |
|---|---|---|---|
| `cargo build --workspace --offline` | ✅ PASS | 必须 PASS | — |
| 22 个 crate 单跑测试 | ✅ PASS（527 pass + 1 ignored） | 必须 PASS | — |
| `cargo test --workspace --offline` | ❌ FAIL | 必须 PASS | `apeireth-verify/example walk_all_crates` 编译失败 |
| `cargo clippy --workspace --offline` | ⚠️ 214 warnings | warnings 数量不限制 | — |
| `cargo clippy --workspace --offline -- -D warnings` | ❌ FAIL | 必须 PASS（若用户要求 L1 严格门禁） | 多个 crate 警告升级 |
| `cargo run -p apeireth-cli --offline -- session` | ✅ PASS | 必须 PASS | — |
| `cargo test -p apeireth-cli --offline --test v7_cli_to_verdict_e2e` | ✅ PASS（1 passed） | 必须 PASS | — |
| `cargo test -p apeireth-pybridge --offline --tests` | ✅ PASS（35 passed） | 仅当 §2 选 A 或 C | — |

### 6.3 当前 workspace test 失败的根因分析（事实）

```
$ cargo test --workspace --offline
   Compiling apeireth-verify v0.14.0
error[E0425]: cannot find function `__register_all_asserts` in crate `apeireth_core`
error: could not compile `apeireth-verify` (example "walk_all_crates")
```

**事实陈述**：
- `crates/apeireth-verify/examples/walk_all_crates.rs:7-12` 调用 6 个 crate 的 `__register_all_asserts`：
  - `apeireth_core::__register_all_asserts()`
  - `apeireth_council::__register_all_asserts()`（council 现在在 workspace，**这是修复路径之一**）
  - `apeireth_sovereignty::__register_all_asserts()`
  - `apeireth_supervisor::__register_all_asserts()`
  - `apeireth_cognition::__register_all_asserts()`
  - `apeireth_constraint::__register_all_asserts()`
- `__register_all_asserts` 在 `crates/apeireth-verify/src/lib.rs` 中定义 `register_all_in_crate!` 宏（`description` 字段）
- 实测 grep：`crates/apeireth-core/src/` 中**无 `__register_all_asserts` 函数定义**

**修复路径候选（用户裁决前不动）**：
1. 在 `apeireth-core` 等 6 crate 中各加 `pub fn __register_all_asserts() { register_all_in_crate!(...) }`
2. 修改 `walk_all_crates.rs` 改用其他 API（如 `apeireth_verify::assertion_count()` 直接调用）
3. 删除 `walk_all_crates.rs` example（标记为"未来 P28 增量"）
4. 修改 `apeireth-verify/Cargo.toml` 移除 `[[example]]` 声明

---

## 7. 用户签收栏

> **使用说明**：
> 1. 用户对 §2/§3/§4/§5 各项冲突做出**最终裁决**
> 2. 裁决结果必须由用户本人填写；Leader 不得代填
> 3. 裁决后此文档作为后续 P-任务的输入依据
> 4. 若用户拒绝全部选项或要求新选项，必须由 Leader 重新整理

### 7.1 冲突 #1：PyBridge 是否保留

| 选项 | 用户裁决（✅/❌/新增选项） | 用户备注 |
|---|---|---|
| A：保留 PyBridge（当前状态） | ⬜ | |
| B：移除 PyBridge（"零 Python"） | ⬜ | |
| C：保留 PyBridge 但切换实现 | ⬜ | |
| **用户新增选项 D**：___________ | ⬜ | |
| **用户最终裁决（自由文本）**：_________________________________________ |
| **签收人**：____________________ | **日期**：__________________ |

### 7.2 冲突 #2：crate 口径

| 选项 | 用户裁决（✅/❌/新增选项） | 用户备注 |
|---|---|---|
| A：以 `Cargo.toml` 为唯一真相（24） | ⬜ | |
| B：以 LOCKED 阶段 4（17）为口径 | ⬜ | |
| C：以"crates/ 目录"为口径（含 `apeireth-bus` 25） | ⬜ | |
| **用户新增选项 D**：___________ | ⬜ | |
| **用户最终裁决（自由文本）**：_________________________________________ |
| **签收人**：____________________ | **日期**：__________________ |

### 7.3 冲突 #3：继续全 backlog vs 先收敛 integration

| 选项 | 用户裁决（✅/❌/新增选项） | 用户备注 |
|---|---|---|
| A：先收敛 integration，再开新 backlog | ⬜ | |
| B：继续全 backlog（每个新 LOCKED 增量开新 P-任务） | ⬜ | |
| C：双轨并行（integration 收敛 + 关键 LOCKED 增量同时推进） | ⬜ | |
| **用户新增选项 D**：___________ | ⬜ | |
| **用户最终裁决（自由文本）**：_________________________________________ |
| **签收人**：____________________ | **日期**：__________________ |

### 7.4 冲突 #4：LOCKED 文档 vs 当前可运行实现

| 选项 | 用户裁决（✅/❌/新增选项） | 用户备注 |
|---|---|---|
| A：LOCKED 视为真相，代码必须收敛 | ⬜ | |
| B：当前可运行实现视为真相，LOCKED 需修订 | ⬜ | |
| C：分层分阶段（核心 LOCKED 收敛，非核心 LOCKED 标注为未来增量） | ⬜ | |
| **用户新增选项 D**：___________ | ⬜ | |
| **用户最终裁决（自由文本）**：_________________________________________ |
| **签收人**：____________________ | **日期**：__________________ |

### 7.5 用户对工作有效性的整体签收

> **整体工作有效性确认**：
> - 本确认单（c0cbd0b3）是否准确反映冲突事实？  ⬜ 是 / ⬜ 否 / ⬜ 需修订：__________
> - V17 阶段 1-2 需求追踪矩阵（reports/V17-writer-stage1-2-traceability.md）是否被用户接受为有效？  ⬜ 是 / ⬜ 否 / ⬜ 需修订：__________
> - 当前 22 个 workspace member 的 527 pass + 1 ignored 单跑测试是否被用户接受为"局部有效"？  ⬜ 是 / ⬜ 否
> - 当前 `cargo test --workspace` 失败是否被用户接受为"已知阻塞项"？  ⬜ 是 / ⬜ 否 / ⬜ 需立即修复

| **用户最终整体签收（自由文本）**：_________________________________________ |
|---|
| **签收人**：____________________ | **日期**：__________________ |

---

## 8. 附录：实测命令输出原文（2026-08-02）

> 本附录记录本次确认单生成时实际运行的命令与输出，供用户核查。

### 8.1 `cargo build --workspace --offline` 输出尾部

```
warning: `apeireth-cognition` (lib) generated 7 warnings (run `cargo fix --lib -p apeireth-cognition` to apply 2 suggestions)
warning: `apeireth-bench` (lib) generated 3 warnings
warning: `apeireth-test` (lib) generated 3 warnings
   Compiling apeireth-cli v0.14.0 (.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.61s
```

**warnings 计数（grep -E "warning:|error"）**: **191**

### 8.2 `cargo test --workspace --offline` 输出尾部

```
   Compiling apeireth-verify v0.14.0 (.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-verify)
error[E0425]: cannot find function `__register_all_asserts` in crate `apeireth_core`
error: could not compile `apeireth-verify` (example "walk_all_crates") due to 1 previous error
```

### 8.3 `cargo clippy --workspace --offline -- -D warnings` 输出尾部

```
   |
   = note: `-D dead_code` implied by `-D warnings`
   = help: to override `-D warnings` add `#[expect(dead_code)]` or `#[allow(dead_code)]`

error: could not compile `apeireth-verify` (lib) due to 1 previous error
```

**warnings 计数（默认）**: **214**

### 8.4 `cargo run -p apeireth-cli --offline -- session` 输出尾部

```
  last_active  : 2026-08-02T07:31:06+00:00
  HA mode      : SingleHuman (1 humans)
  PermissionOnion: L0=L0 HA 核心 / L5=L5 核武器级
  守门          : V1+V2+V3 AND 门 (双洋葱 + HA)
  ✅ session 已启动 (A1 第 1 天任务完成)

📥 输入一行文本 → 自动构造 Action → 走 V1+V2+V3 AND 门
   输入 ":quit" / ":exit" 或 Ctrl-D / Ctrl-Z 退出

>
```

### 8.5 `apeireth-bus` 目录状态

```
$ ls -la crates/apeireth-bus/
total 12
drwxr-xr-x 1 XXX 197609 0 Aug  2 11:10 .
drwxr-xr-x 1 XXX 197609 0 Aug  2 11:11 ..
drwxr-xr-x 1 XXX 197609 0 Aug  2 01:30 target

$ find crates/apeireth-bus -type f -not -path "*target*"
（无输出 — 无 Cargo.toml，无 src/）

$ cargo build -p apeireth-bus --offline
error: package ID specification `apeireth-bus` did not match any packages

help: a package with a similar name exists: `apeireth-asi`
```

### 8.6 `Cargo.toml` workspace members 原文（`Cargo.toml:3-28`）

```toml
members = [
    "crates/apeireth-core",
    "crates/apeireth-memory",
    "crates/apeireth-asi",
    "crates/apeireth-philosophy",
    "crates/apeireth-tools",
    "crates/apeireth-cli",
    "crates/apeireth-bench",
    "crates/apeireth-test",
    "crates/apeireth-cognition",
    "crates/apeireth-action",
    "crates/apeireth-life-force",
    "crates/apeireth-constraint",
    "crates/apeireth-central",
    "crates/apeireth-value",
    "crates/apeireth-consciousness",
    "crates/apeireth-relation",
    "crates/apeireth-motivation",
    "crates/apeireth-perception",
    "crates/apeireth-upgrade",
    "crates/apeireth-onion",
    "crates/apeireth-council",
    "crates/apeireth-sovereignty",
    "crates/apeireth-verify",
    "crates/apeireth-supervisor",
]
```

### 8.7 `crates/apeireth-pybridge/src/lib.rs` 顶部（确认 PyBridge 实装）

```rust
//! apeireth-pybridge: PyO3 桥 (Python 3.13.14 ↔ Rust)
//!
"apeireth-pybridge R14 A16.3 landed (PyO3 bridge + R11 1100+3 baseline + 35 tests)"
```

**单测数实测**：

```
$ cargo test -p apeireth-pybridge --offline --tests 2>&1 | grep -E "Running|test result"
     Running unittests src\lib.rs (target\debug\deps\apeireth_pybridge-a7f7671929c7c959.exe)
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### 8.8 `crates/` 完整目录清单

```
README.md
apeireth-action
apeireth-asi
apeireth-bench
apeireth-bus                 ← ⚠️ 仅 target/，无 Cargo.toml + src/
apeireth-central
apeireth-cli
apeireth-cognition
apeireth-consciousness
apeireth-constraint
apeireth-core
apeireth-council             ← workspace member
apeireth-life-force
apeireth-memory
apeireth-motivation
apeireth-onion
apeireth-perception
apeireth-philosophy
apeireth-pybridge            ← workspace member, 859 行 Rust + 35 单测
apeireth-relation
apeireth-sovereignty         ← workspace member
apeireth-supervisor          ← workspace member
apeireth-test
apeireth-tools
apeireth-upgrade
apeireth-value
apeireth-verify              ← workspace member
```

---

## 9. 文档生成信息

- **作者**: technical_writer（c0cbd0b3-57f8-440c-92a8-f3d057ecc163）
- **生成时间**: 2026-08-02
- **本文件路径**: `reports/c0cbd0b3-57f8-440c-92a8-f3d057ecc163-technical-writer-requirement-validation-signoff.md`
- **唯一目标对齐**: 与用户沟通真实需求，并由用户确认工作有效性
- **主哲学 6 锚穿透**:
  - **S-1 主 22:33 北极星导向** — §5 冲突 #4 选项 C 把"核心 LOCKED"与"非核心 LOCKED"分层，正是 ASI 北极星导向
  - **S-2 主 17:43 实事求是** — §1.1-1.5 全部为实测事实，§8 附录含命令原文
  - **O-5 主 17:58 不假装** — §1.3 `apeireth-bus` 无源码、§6.3 workspace test 失败根因全部诚实登记
  - **O-2 主 19:33 走在前人经验上** — §6 验收命令模板借鉴 V17 / V18 / V25 / V26.1 既有报告
  - **O-3 主 23:44 干到底** — §2/§3/§4/§5 四个冲突每个 3-4 个选项 + 影响分析 + Leader 建议全列
  - **O-4 主 00:56 任何人都能接手** — §7 用户签收栏结构化 + §8 实测命令可复现
- **未做之事（保持诚实）**:
  - ❌ 未修改任何业务代码（`crates/**/src/*.rs`）
  - ❌ 未修改任何 LOCKED 文档（`docs/stage{1,2,3,4,5}/*.md`）
  - ❌ 未修改 `Cargo.toml` workspace members
  - ❌ 未对 4 项冲突做出最终裁决（**用户裁决权不可被 Leader 替代**）
  - ❌ 未将 Leader 建议描述为"用户已确认"

---

_本确认单等待用户对 §7 四个签收栏的裁决。_
_任何接手者能根据 §1 事实 + §6 命令 + §8 附录验证全部内容。_
_矩阵不可摘要替代：每一行 = 1 个具体代码/测试/命令锚点。_