# 0017. KC-1 内核收束：通用完成观察口与注册式内部控制命令

- **Status**: Accepted（2026-10-27，与实现同批；owner 已在本轮 KC-1 SPEC 中裁决）
- **Date**: 2026-10-27
- **Owners**: runtime / architecture-line
- **Supersedes**: —
- **契约影响**: `docs/architecture/contract.toml` 新增 `[kernel]`；`[extension_points]` 新增 `tool_completion_observer` / `control_command`；`scripts/arch-baseline.txt` 新增 2 条 `ext` 登记
- **例外关联**: —
- **预算净额**: 依赖边 ±0（runtime 仅新增内部 path dev-dep `apeireth-organ`，生产边不变）/ 开关 ±0 / crate ±0 / 文件尺寸净减（runtime 侧 −1294 行领域实现）

## 1. 背景与问题

`main@4ea9c260` 的 Runtime Kernel 直接持有领域痛觉状态并在失败收口驱动领域记账：

- `crates/engine/runtime/src/canonical/runtime.rs:319` `pub(super) bio_pain: Mutex<BioPainState>`；
- `crates/engine/runtime/src/canonical/execute.rs:3401-3413` 在内核完成收口里直接调用 `observe_failure/observe_success`；
- `execute.rs:3031-3055` 对 `exit_plan_mode` 做业务工具名硬编码分支。

这使内核拥有领域状态与业务派发入口，新增领域能力需要改执行循环。KC-1 要求恢复机制型微内核边界，同时保持既有事件、审批、会话与执行语义不变。

## 2. 决策

1. 在 `foundation/plugin::observation` 增加最小通用端口 `ToolCompletionObserver` / `ToolCompletionFact`。内核在唯一完成收口 `record_call_completed`（串行派发 / 并行正常提交 / 取消排水）中、在 `ToolFailed` 与 `CapabilityCompleted` 落账之后发出**一次**事实；未安装观察者时零额外开销。
2. 痛觉与再生案例账本整体迁至 `apeireth-organ::nociception`，由 `NociceptionObserver` 实现上述端口；装配层负责按 `APEIRETH_ENABLE_BIO_PAIN` 构造并装入。
3. 在 runtime 既有 `CapabilityRegistry` 身份空间内增加 `ControlCommand` 注册面；`exit_plan_mode` 的描述符与实现归位 `canonical/plan_mode_control.rs`，执行循环只做通用控制查找。
4. 同名工具/控制注册即拒；插件工具在 `start_all` 后由 `reject_control_tool_collisions` 在启动期拒绝；执行点仍保留 fail-closed 兜底。

**不改什么**：不引入第二套事件系统、不新增独立控制注册表、不重写审批/治理/会话语义、不搬 foundation 的 A 类通用机制。

## 3. O-6 三阶审查

- **总体最优**：先处理已确证的领域状态回流与硬编码分派，而不是重写整个 runtime；账本整体外迁保留既有行为，再用一个通用端口解耦。拒：①继续在 runtime 内加开关（领域状态仍在内核）；②新建第二套事件系统（与 runtime event spine 竞争）；③一次性拆分全部 assembly（超范围、风险高）。
- **系统最优**：领域账本放 `engine/organ`，观察口 trait 放 `foundation/plugin`（runtime 与 organ 都已依赖的 ABI 层，0 新 crate 边），装配在 CLI/assembly。拒：①observer trait 放 runtime（organ 反向依赖 kernel）；②账本放 runtime-assembly（装配层承担领域算法，违反 KC-3）。
- **架构最优**：控制命令复用既有 `CapabilityRegistry` 身份空间，描述符与实现分离；同名即拒、执行点 fail-closed。拒：①为 `exit_plan_mode` 新建独立注册表（第二套注册体系）；②扩 `ToolCapability` ABI 传入 `&mut Session`（把领域/会话能力塞进通用工具契约）。

## 4. 迁移计划（分步、每步可验证）

1. 新增 `foundation/plugin::observation` 端口与单测 — 验证：`cargo test -p apeireth-plugin observation`。
2. 账本迁至 `organ::nociception` 并实现 observer — 验证：`cargo test -p apeireth-organ --lib nociception`。
3. runtime 移除领域字段/开关/快照，改为发出一次完成事实 — 验证：`cargo test -p apeireth-runtime --test bio_pain_wiring`、`--test kernel_boundary`。
4. CLI/assembly 改绑 observer 句柄 — 验证：`cargo check -p apeireth-runtime-assembly -p apeireth-cli --all-targets --locked`。
5. 注册式控制命令 + 冲突拒绝 — 验证：`cargo test -p apeireth-runtime --test canonical_plan_mode`、`--lib capability::tests`。
6. 架构守卫 `[kernel]` + `--self-test` — 验证：`python scripts/arch_guard.py --strict`、`--self-test`。

## 5. 回滚

- 回滚代码：`git revert <KC-1 commit>`；随后在 `contract.toml` 删除 `[kernel]` 与两条 `[extension_points]`，并在 `arch-baseline.txt` 删除对应 `ext` 两行。
- 回滚后 runtime 恢复持有 `bio_pain` 与 `exit_plan_mode` 硬编码分支；`binary`/配置面无需数据迁移（账本原为进程内状态，无持久化）。
- 用户侧开关保持不变：`APEIRETH_ENABLE_BIO_PAIN` 仍为 env > 配置 > 默认关。

## 6. 验收

- [x] `python scripts/arch_guard.py --strict` 通过（VIOLATIONS 0）；`--self-test` 4/4 PASS
- [x] 相关测试：`bio_pain_wiring` 4 / `canonical_plan_mode` 5 / `kernel_boundary` 4 / organ nociception 24 / capability 6 / assembly bio_mechanisms 22
- [x] 契约条款更新：`[kernel]` + `[extension_points]` 两条
- [x] 文档随行：能力矩阵、claims、wiring、README §6、CHANGELOG
- [x] LOCKED 5 项 0 触碰 / 0 新外部 dep（仅内部 path dev-dep `apeireth-organ`）

## 7. 后果（Consequences）

- **正面**：内核不再持有已确认的领域状态，也不再匹配业务工具名；领域模块可通过通用端口接入/移除；CI 有文本级回退守卫。
- **负面/接受的代价**：观察口是进程内端口，不序列化；静态守卫不能证明宏展开/动态分派；`bio_pain` 快照类型路径发生公开 API 迁移（调用方改为 `organ::nociception`）。
- **后续 stage**：KC-5 全量 `gate_full.ps1` 收口；foundation 的 B 类领域算法（`nociception` 内核、`recovery_protocol` 等）归属迁移另立阶段。
