# a2557c25 缺口矩阵 + 工程决定 → 派活清单 (round 5)

**Task ID**: a2557c25-46a2-4b12-a388-8e62a1790115  
**Role**: architect  
**Status**: ✅ 完成（仅派活清单，不实施代码）

---

## 0. 任务说明

**输入依据**：
- reports/d8437877-locked-stage5-gap-matrix.md（阶段 1–5 LOCKED → 工程缺口矩阵，3 GAP + 10 漂移）
- 本轮工程期 ADR 决定（架构师推荐方案，§5 用户裁决栏）

**约束**：
- ❌ 不实施代码（仅做派活清单）
- ✅ 5–8 件实事任务
- ✅ 每任务含：crate 范围 / 必需 trait+fn+tests / 验收命令 / LOCKED 边界
- ✅ Leader 按此派活给后端工程师 / 全栈工程师 / 等

---

## 1. 派活优先级（按缺口影响半径）

| 序 | 任务 ID | 名称 | 缺口 | 优先级 | 估时 | 派活对象 |
|---|---|---|---|---|---|---|
| 1 | `round5-01-evolution` | 新建 `apeireth-evolution` crate (G1) | G1 | P0 | 2 天 | backend_engineer2 |
| 2 | `round5-02-bus-extract` | 从 central 抽离 `apeireth-bus` 5 层总线 (G2) | G2 | P0 | 1.5 天 | backend_engineer2 |
| 3 | `round5-03-extension-skeleton` | `apeireth-extension` WASM 骨架 + plugin trait (G3-A 方案) | G3 | P1 | 2 天 | fullstack_engineer |
| 4 | `round5-04-trait-fail-6` | 补全 V24 trait acceptance 6 FAIL trait | trait 落地 | P1 | 1 天 | backend_engineer2 |
| 5 | `round5-05-pybridge-r11-100` | pybridge 接 R11 1100+3 模块前 100 个 | 阶段 5 §2 #17 | P1 | 2 天 | mcp_integration_expert2 |
| 6 | `round5-06-central-onion-bridge` | central → onion 双洋葱 trait 集成 | 数据流 | P2 | 1 天 | backend_engineer2 |
| 7 | `round5-07-supervisor-q14-real` | supervisor Q14 集成测试接真实 sub-supervisor | 工程验证 | P2 | 0.5 天 | backend_engineer2 |
| 8 | `round5-08-stage6-milestone` | 阶段 6 里程碑验证机制 + verify crate 接入 | 阶段 5 §6 | P2 | 1 天 | devops_engineer2 |

**总估时**: ~10.5 人日（P0+P1 = 8.5 天 → 1 个 sprint 完成）

---

## 2. 任务详情（5–8 件）

### 任务 1：`round5-01-evolution` — 新建 `apeireth-evolution` crate（G1）

**crate 范围**：
- 新建 `crates/apeireth-evolution/`（Cargo.toml + src/lib.rs + tests/）
- 加入 workspace.members

**必需 trait + fn**：
```rust
// crates/apeireth-evolution/src/lib.rs
pub trait Learning { fn learn(&mut self, episode: &Episode) -> Result<(), EvolutionError>; }
pub trait Abstraction { fn abstract_concept(&self, examples: &[Episode]) -> Concept; }
pub trait SelfModification { fn propose_patch(&self, current: &Codebase) -> Patch; }
pub trait Extension { fn extend_capability(&mut self, plugin: Box<dyn Plugin>) -> Result<(), EvolutionError>; }

// 必须 fn
pub fn safe_evolve(state: &mut SystemState) -> Result<EvolutionLog, EvolutionError>;
pub fn revert_to_snapshot(snapshot_id: SnapshotId) -> Result<(), EvolutionError>;
```

**tests 最低要求**：
- `tests/evolution_traits_acceptance.rs`（7+ tests：每个 trait 至少 1 个）
- trait 实现 mock struct（不依赖真 LLM）

**验收命令**：
```bash
cargo build -p apeireth-evolution --offline         # 必须 0 error
cargo test -p apeireth-evolution --offline          # 必须 7+ passed
cargo build --workspace --offline                   # 不能破坏其他 26 crate
```

**LOCKED 边界**：
- ✅ 不修改 `docs/stage5/stage5-construction-document.md`（§2 #6 已 LOCKED）
- ✅ 不修改 `docs/stage4/architecture-stage4-engineering-landing.md`
- ⚠️ onion + upgrade 中的演化相关代码暂不动（后续再收敛）

---

### 任务 2：`round5-02-bus-extract` — 从 central 抽离 `apeireth-bus` 5 层总线（G2）

**crate 范围**：
- 新建 `crates/apeireth-bus/`（Cargo.toml + src/lib.rs + tests/）
- 从 `crates/apeireth-central/src/` 移除 bus 相关代码 → 改为 `pub use apeireth_bus::*`

**必需 trait + fn**：
```rust
// crates/apeireth-bus/src/lib.rs
pub trait Transport { fn send(&self, msg: &Message) -> Result<(), BusError>; }
pub trait ControlPlane { fn route(&self, msg: &Message) -> Channel; }
pub trait DataPlane { fn forward(&self, payload: &Bytes) -> Result<(), BusError>; }

// 5 层 (L1 inproc / L2 unix-socket / L3 pipe / L4 grpc-ws / L5 持久化)
pub fn create_bus(layer: BusLayer) -> Box<dyn Transport>;
pub fn split_control_data(bus: Box<dyn Transport>) -> (Box<dyn ControlPlane>, Box<dyn DataPlane>);
```

**tests 最低要求**：
- `tests/bus_5layer.rs`（5 个 layer 集成 test）
- `tests/control_data_separation.rs`（控制/数据面隔离 test）

**验收命令**：
```bash
cargo build -p apeireth-bus --offline             # 0 error
cargo test -p apeireth-bus --offline              # 7+ passed
cargo build -p apeireth-central --offline         # central 仍编译（仅 use 引用 bus）
cargo test --workspace --offline                  # 全 workspace 不退化
```

**LOCKED 边界**：
- ❌ 不修改 `docs/stage5/stage5-construction-document.md` §2 #15 LOCKED 表述
- ✅ central 内部结构调整允许（仍保持 PID 1 入口职责）

---

### 任务 3：`round5-03-extension-skeleton` — `apeireth-extension` WASM 骨架 + plugin trait（G3-A）

**crate 范围**：
- 新建 `crates/apeireth-extension/`（Cargo.toml + src/lib.rs + tests/）
- 引入 `wasmtime` 依赖（workspace.dependencies）

**必需 trait + fn**：
```rust
// crates/apeireth-extension/src/lib.rs
pub trait Plugin { 
    fn name(&self) -> &str; 
    fn plugin_type(&self) -> PluginType; 
    fn execute(&self, input: &PluginInput) -> Result<PluginOutput, PluginError>; 
}

// 6 类 pluginType: Tool / Sensor / Action / Memory / Reason / Render
pub fn load_wasm_plugin(bytes: &[u8]) -> Result<Box<dyn Plugin>, PluginError>;
pub fn register_plugin(plugin: Box<dyn Plugin>, registry: &mut PluginRegistry);

// 5 轴正交: Capability / Safety / Lifecycle / Observability / Versioning
```

**tests 最低要求**：
- `tests/wasm_load_mock.rs`（mock plugin 加载）
- `tests/plugin_6types.rs`（6 类 pluginType 各 1 test）
- `tests/5_axes.rs`（5 轴属性检查）

**验收命令**：
```bash
cargo build -p apeireth-extension --offline          # 0 error
cargo test -p apeireth-extension --offline           # 12+ passed
wasmtime --version                                   # wasmtime 依赖解析成功
```

**LOCKED 边界**：
- ❌ 不修改 `docs/stage4/architecture-stage4-engineering-landing.md` (Stage4 §3 LOCKED)
- ✅ 可推迟实际 WASM 字节码执行（仅骨架 + trait + mock）

---

### 任务 4：`round5-04-trait-fail-6` — 补全 V24 trait acceptance 6 FAIL

**crate 范围**：
- 涉及 V24 报告中 6 FAIL trait 所在 crate（具体 crate 由 backend_engineer2 判定）
- 大概率涉及：perception (Attention) / cognition (MetaCognition) / action (Silence) / consciousness (Recovering) 等

**必需 trait + fn**：
- 6 FAIL trait 的完整实现 + tests
- 每个 trait 必须有：trait 定义 + 至少 1 mock 实现 + 至少 1 集成 test

**tests 最低要求**：
- `tests/trait_fail_completion.rs` 或各 crate 内的 `tests/`
- V24 FAIL → PASS 转化证据（diff of V24 vs V25 报告）

**验收命令**：
```bash
cargo test --workspace --offline                    # 全 workspace 测试
cargo test --doc --workspace --offline              # doc tests
# V24 FAIL 6 trait 在新报告中应标记为 PASS
```

**LOCKED 边界**：
- ❌ 不修改 `reports/V24-stage4-trait-acceptance.md`（仅生成 V25 增量报告）
- ✅ trait 函数签名遵循 Stage4 §3 推导（不偏离本源）

---

### 任务 5：`round5-05-pybridge-r11-100` — pybridge 接 R11 1100+3 模块前 100 个

**crate 范围**：
- `crates/apeireth-pybridge/src/r11_compat.rs`（已有 249 行 + 1103 modules）
- 验证前 100 个 R11 模块可加载 + 调用

**必需 trait + fn**：
```rust
// 新增 fn（r11_compat.rs）
pub fn load_r11_module_top_n(n: usize) -> Vec<LoadResult>;  // 加载前 n 个
pub fn is_r11_callable(module: &str, func: &str) -> bool;
```

**tests 最低要求**：
- `tests/pybridge_r11_100.rs`（前 100 个模块加载 + 至少 1 调用）
- 与现有 `tests/pybridge_q29.rs` 不冲突

**验收命令**：
```bash
cargo test -p apeireth-pybridge --offline --test pybridge_r11_100   # 5+ passed
cargo test -p apeireth-pybridge --offline                          # 现有 45 tests 仍 PASS
```

**LOCKED 边界**：
- ❌ 不修改 `docs/stage5/stage5-construction-document.md` §2 #17 LOCKED
- ✅ pyo3 feature flag 仍按 `python-ext` 可选启用

---

### 任务 6：`round5-06-central-onion-bridge` — central → onion 双洋葱 trait 集成

**crate 范围**：
- `crates/apeireth-central/src/lib.rs`（使用 onion 抽象）
- `crates/apeireth-onion/src/`（被引用）

**必需 trait + fn**：
```rust
// crates/apeireth-central/src/lifecycle.rs（新增）
pub use apeireth_onion::{PrincipleLayer, PermissionLayer, OnionContext};

impl CentralOrchestrator {
    pub fn with_onion_context(ctx: OnionContext) -> Self;
    pub fn enforce_principle(&self, action: &Action) -> Verdict;
}
```

**tests 最低要求**：
- `crates/apeireth-central/tests/onion_integration.rs`（双洋葱 5 层 + 6 层各 1 test）

**验收命令**：
```bash
cargo build -p apeireth-central --offline          # 0 error
cargo test -p apeireth-central --offline           # 5+ passed
cargo build --workspace --offline                  # 全 workspace
```

**LOCKED 边界**：
- ❌ 不修改 `docs/stage4/architecture-stage4-engineering-landing.md`
- ✅ onion 抽象层定义不动

---

### 任务 7：`round5-07-supervisor-q14-real` — supervisor Q14 集成测试接真实 sub-supervisor

**crate 范围**：
- `crates/apeireth-supervisor/tests/supervisor_q14.rs`（已存在）
- 接真实 sub-supervisor（之前是 mock）

**必需 trait + fn**：
- 5 个 sub-supervisor trait 实现（不依赖 mock）
- Q14 测试矩阵：5 秒自动重启 / 失败回滚 / Council 评估触发 / 子进程独立升级

**tests 最低要求**：
- `tests/supervisor_q14_real.rs`（4 个维度各 1 个真实集成 test）

**验收命令**：
```bash
cargo test -p apeireth-supervisor --offline --test supervisor_q14_real   # 4+ passed
cargo test -p apeireth-supervisor --offline                              # 现有 tests 仍 PASS
```

**LOCKED 边界**：
- ❌ 不修改 P25 supervisor trait 公开签名
- ✅ 内部 sub-supervisor 实现可重构

---

### 任务 8：`round5-08-stage6-milestone` — 阶段 6 里程碑验证机制 + verify crate 接入

**crate 范围**：
- `crates/apeireth-verify/`（V26.2 已实装）
- 阶段 6 milestone 定义 + 触发机制

**必需 trait + fn**：
```rust
// crates/apeireth-verify/src/milestone.rs（新增）
pub trait Milestone { 
    fn name(&self) -> &str; 
    fn check(&self, state: &SystemState) -> MilestoneStatus; 
}

pub fn register_milestone(m: Box<dyn Milestone>);
pub fn run_all_milestones(state: &SystemState) -> Vec<MilestoneReport>;
```

**tests 最低要求**：
- `tests/milestone_smoke.rs`（至少 3 个 milestone 定义 + 检查）

**验收命令**：
```bash
cargo build -p apeireth-verify --offline           # 0 error
cargo test -p apeireth-verify --offline            # 3+ passed
cargo test --workspace --offline                   # 879 → 882+ passed
```

**LOCKED 边界**：
- ❌ 不修改 `docs/stage5/stage5-construction-document.md` §6 LOCKED 表述
- ✅ verify crate 内部实现可演进

---

## 3. 派活汇总表（Leader 直接照抄）

| ID | 任务名 | crate | 必需 trait | tests 数 | 验收命令 |
|---|---|---|---|---|---|
| round5-01 | 新建 evolution crate (G1) | new evolution | 4 trait (Learning/Abstraction/SelfModification/Extension) | 7+ | `cargo build && cargo test -p apeireth-evolution` |
| round5-02 | 抽离 bus crate (G2) | new bus | 3 trait (Transport/ControlPlane/DataPlane) | 7+ | `cargo test -p apeireth-bus` + workspace |
| round5-03 | extension WASM 骨架 (G3) | new extension | 1 trait (Plugin) + 6 pluginType + 5 axes | 12+ | `cargo test -p apeireth-extension` |
| round5-04 | V24 trait FAIL 6 补全 | 6 crate | V24 6 FAIL trait | 6+ | `cargo test --workspace` |
| round5-05 | pybridge R11 100 模块 | pybridge | 2 fn (load_r11_module_top_n / is_r11_callable) | 5+ | `cargo test -p apeireth-pybridge --test pybridge_r11_100` |
| round5-06 | central → onion 集成 | central+onion | bridge impl | 5+ | `cargo test -p apeireth-central` |
| round5-07 | supervisor Q14 真实测试 | supervisor | 5 sub-supervisor | 4+ | `cargo test -p apeireth-supervisor --test supervisor_q14_real` |
| round5-08 | 阶段 6 milestone + verify | verify | Milestone trait | 3+ | `cargo test -p apeireth-verify` + workspace |

---

## 4. LOCKED 边界全局规则（所有任务统一遵守）

❌ **禁止修改的 LOCKED 文档**：
- `docs/stage1/inspiration-stage1-2026-07-30.md`（stage1 LOCKED）
- `docs/stage2/stage2-decisions-*.md` 全部 18 个（stage2 LOCKED）
- `docs/stage3-blueprints/*.md` 14 个 stage3 LOCKED
- `docs/stage4/architecture-*.md`（stage4 LOCKED，1492 行）
- `docs/stage5/stage5-construction-document.md`（stage5 LOCKED，§2 #6/#15/#16 待裁决）

✅ **允许的范围**：
- `crates/*/src/` 实现代码
- `crates/*/tests/` 测试代码
- `crates/*/examples/` 示例代码
- `Cargo.toml` workspace.dependencies + workspace.members
- `reports/` 增量报告（不覆盖 LOCKED 报告）

---

## 5. 验收全局命令（每任务完成后必跑）

```bash
# 单任务验收
cargo build -p <crate> --offline                    # 必须 0 error
cargo test -p <crate> --offline                     # 必须 tests 数达标
cargo clippy -p <crate> --offline -- -D warnings    # clippy 通过

# 全局回归
cargo build --workspace --offline                  # 必须 0 error
cargo test --workspace --offline                   # 879 → 更多 passed
cargo test --doc --workspace --offline             # doc tests
```

---

## 6. 提交

- report: `reports/a2557c25-round5-engineering-decisions-tasks.md`（本文件）
- 状态: ✅ 完成（仅派活清单，不实施代码）
- 后续: Leader 按 §3 表派活给 backend_engineer2 / fullstack_engineer / mcp_integration_expert2 / devops_engineer2
