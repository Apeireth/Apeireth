# Kernel Convergence — S5 default managed desktop main

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s5-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: S5 指定窄门禁通过；整体 Production Convergence 仍为 Partial
Author: Codex
```

## 基线与边界

初始 `git fetch origin && git status && git log -8 --oneline` 成功，分支为
`refactor/kc1-kernel-convergence`，工作树 clean，HEAD `e51a8d5c` 位于 S4 源码 `9015d9f0` 之上。
源码提交为 `2ee10a39869fe2d3723188d04114673896e5afb1`；后续文档证据提交不修改 Rust / Cargo。
本批只实施[债务图 D3 / S5](kernel-convergence-remaining-debt-map-2026-10-29.md)；
沿用 [S1](kernel-convergence-s1-2026-10-29.md)、[S2](kernel-convergence-s2-2026-10-29.md)、
[S3](kernel-convergence-s3-2026-10-29.md)、[S4](kernel-convergence-s4-2026-10-29.md)、
ADR-0018～0022 与[历史验收账](kernel-convergence-final-acceptance-2026-10-28.md)。
取材仅为本地源码和文档；未修改历史验收结论。

## 执行与授权

**PRODUCTION WIRED / DEFAULT ENABLED（桌面默认 main）**：CLI 的生产装配显式调用
`ProductionModules::desktop_agent_policy`，随后由 `build_desktop_main_runtime` 调用现有
`RuntimeBuilder::build_agent_runtime` / `into_main_runtime`，创建 Host-owned `ManagedAgent`。
每个 Host 只有该 AgentRuntime 持有可变 Registry；兼容 Runtime 返回值是观察与执行委托面。
默认域的 CLI、gateway、审批、流式和 care/heartbeat 共享这个面和同一个 main AgentId。
原本独立存储、装配的伙伴域仍各有其 Host，不能跨域合并数据库、权限或认知对象。

`MainExecution` 是现有 ManagedAgent 的完整请求适配，接收原 TurnRequest、delta sink 和审批参数。
Runtime 的普通、流式与审批入口统一委托它；每次在短控制读锁内复制不可变执行视图，随后进入
原 Kernel，释放控制锁后才等待 Hook / Provider。它没有模型循环、Provider 客户端、工具目录或独立 Runtime。
模块 child 仍由 S1/S2 实例任务执行；同步 child 借用父 turn slot，使用独立 Session，未重入父会话锁。

- 外部 SessionId 保留；每个会话的历史、静态 system、model、intent/security、principal、授权 epoch、
  scope ceiling 与冻结审批由原 Kernel 处理。适配器绑定同一个 main AgentId，不复制或拼接旧历史。
  不允许认领其他实例已经拥有的 Session；会话原 scope 与主实例所选工具集合取交集。
- gateway 原会话写闸、客户端归属审计、域校验和用户审批权限继续在原入口执行。
  heartbeat 的实际提交缝增加同一会话写闸，再走原 execute；生产线程和直接测试都使用该缝。
- main 内部只按 Session 串行；不同会话共享原 Host / Provider 局部并发额度。
  一个会话待审批不阻断其他会话，原审批 ID、冻结参数、恢复结果和重复消费拒绝均保留。
  输入、执行错误和 ApprovalResolution 原样返回，不用文本任务接口压平其类型。
- 主实例快照与 requests 汇总其所有绑定会话（requests 延续每会话有界近期事实窗口）；真实 Provider / Dispatcher 仍是唯一记账处。
  有限 main budget 同时绑定 owner ceiling，换 Session 不能重置额度；未知 usage 仍拒绝后续有限预算请求。
  桌面 Host 显式选择 `into_main_runtime_with_session_quotas`，延续原 SessionQuota 设置和未知 usage 的记录语义，
  不新增小型私有任务的累计次数或 token 上限；该选择不关闭共享 runtime budget，未知 usage 不伪造为已报告零值；
  原有模块预算、实例记录上限、治理和工具超时不放宽。
- Host 显式为选中的认知模块赋予 child grant；内部 callback capability 仍由原 register_with_policy 授权。
  主实例选择启动目录作为 scope，模块子任务仍取 Host grant、父 scope 与任务 allowlist 的交集。
  注册本身不授予治理权限，新工具也不能借已有会话范围越权。
- Registry、Dispatcher、Module Factory、MCP 生命周期和多实例 Runtime 沿用既有实现。
  Runtime 的控制兼容入口委托现有 Host 控制面；同步控制遇忙明确失败，不同步等待 Provider。
  显式 shutdown 撤销并有界排水主实例及模块工作；单个会话错误不会新建 main 或重置账本。

S4 的 ProfileMemoryMappings API 保持可选，桌面 bootstrap 本批**不选择映射**。
带 principal 的会话继续遵守 S3 默认拒绝的旧全局认知边界；已有授权 epoch、原生 MemoryModule 和显式映射 API 保留。
无 principal 的桌面会话继续使用原数据库及认知业务对象，不做数据迁移或隐式认领。

PortableHost 没有改用桌面组合；新增桌面装配位于 desktop feature 内。
最小 native 测试实际检查 normal dependency graph，拒绝桌面工具、Organ、Guard、Memory/Storage、Gateway 与 SQLite。
保留 Mesh/NAT/P2P；没有新增外部依赖或修改 Cargo.lock、workspace.version、LOCKED 项。

## 证据与验证

环境：2026-10-29（Asia/Shanghai），Linux x86_64，rustc / cargo 1.97.1、PowerShell 7.5.4、Python 3.13.5。
Cargo jobs=2，默认 dev/test profile，debug assertions 保留。
原始输出与落盘退出码位于 `/workspace/apeireth-codex/s5-*.log` / `s5-*.exit`；命令清单与统计脚本同前缀保存。
本批测试数字只记录定向实测，不替代 [test-baseline](../03-reference/test-baseline.md) 的历史全量口径。

| 命令 | 本批实测 passed / failed / ignored 或结果 |
| --- | --- |
| `cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2` | 7 / 0 / 0 |
| `cargo test -p apeireth-cli --test care_heartbeat_host --locked -j 2` | 4 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test managed_main --locked -j 2` | 4 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2` | 5 / 0 / 0 |
| `cargo test -p apeireth-gateway --test canonical_entry_e2e --locked -j 2` | 10 / 0 / 0 |
| `cargo test -p apeireth-gateway --test multi_client_session --locked -j 2` | 12 / 0 / 0 |
| `cargo test -p apeireth-gateway --test domain_partner_chat --locked -j 2` | 7 / 0 / 0 |
| `cargo test -p apeireth-gateway --test user_domain_isolation --locked -j 2` | 7 / 0 / 0 |
| `cargo test -p apeireth-runtime --test cognitive_module_abi --locked -j 2` | 29 / 0 / 0 |
| `cargo test -p apeireth-runtime --all-targets --locked -j 2` | 380 / 0 / 0（含上列 cognitive，不重复相加） |
| `cargo test -p apeireth-runtime-assembly --test convergence_agents --test convergence_memory --locked -j 2` | 22 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test portable_host --locked -j 2` | 7 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --no-default-features --test portable_host --locked -j 2` | 5 / 0 / 0 |
| `cargo check -p apeireth-runtime-assembly --no-default-features --locked -j 2` | 通过（exit 0） |
| `cargo clippy -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-cli -p apeireth-gateway --all-targets --locked -j 2 -- -D warnings` | 通过，零警告 |
| `cargo fmt --check` | 通过（exit 0） |
| `python3 scripts/arch_guard.py --strict` | 通过（exit 0） |
| `python3 scripts/arch_doc.py --check` | 通过（exit 0） |
| `python3 scripts/docs_links.py --check` | 通过（exit 0） |
| `python3 scripts/docs_index.py --check` | 通过（exit 0） |

命令清单为 `s5-final-results.json`；`s5-counts.py` 从原日志生成 `s5-test-counts.json`，不重复累加重叠目标。
门禁前后的 Rust / Cargo SHA256 清单相同，见 `s5-source-manifest.json`。
测试名称与 ignore 保留检查为 `s5-preserved-tests.json`。按本批授权未跑 workspace 全量。
源码提交前 Docs Truth 检出受影响 claims 的 STALE，未伪称已消除；本次独立文档提交已锚定上述源码 SHA。
`docs_truth.py --verify --block-stale`、`--check` 与 `wiring_report.py --verify` 分别执行并通过，
没有 FAIL / STALE 或生成物漂移。`s5-truth-verify-final.log` / `s5-truth-check-final.log` / `s5-wiring-final.log`
及同名 `.exit` 保存证据；保留既有宽泛能力等级，仅为默认桌面 main 新增独立 claim。



新增真实 bootstrap 测试覆盖 CLI→HTTP 同会话续接、独立 SSE delta、静态 system、真实审批恢复与重复消费、
heartbeat 提交、共享 main 身份与 Provider usage owner，以及 shutdown 后拒绝执行。
care_heartbeat_host 另以实际生产线程证明同一 managed main 的发起、持久化和用量归属。
managed_main 集成覆盖 Provider 阻塞期间热注册/stop、不同 Session 并发且历史隔离、
Host shutdown 取消及保账、他实例会话认领拒绝、跨 Session 不重置预算。
production_cognitive_v1 的生产 load / stop / permission / guessed receipt / module-only 用例改走 managed desktop；
旧 builder 的角色、stop 和 private principal 用例同时保留原装配并新增 managed 装配。
单槽 Host 继续覆盖认知子任务预算及无重入死锁；全部旧测试名称和安全断言保留，不新增 ignore。

## 交付与后续

源码及随行说明先提交；本次独立文档提交已将受影响 claims 的 verified_at 锚定上述源码并生成 STATUS / wiring，避免自引用 SHA。
默认 managed main 使用独立 claim 描述，已有宽泛多实例能力等级不因本批自动提升。
整体 Production Convergence 仍为 **Partial**；S6 的余留 V1 兼容审计与最终 gates 未在本批启动。
没有 workspace 全量、真实外部 API、外部 MCP 服务、原生桌面 UI、Android 交叉编译或设备实测声明；无 HARDWARE VALIDATED。

O-6：总体选择默认 bootstrap 的完整多入口切片，拒只改 CLI chat 或牺牲 HTTP 功能；
系统将执行归属接到现有 ManagedAgent，拒第二 Provider loop、全局等待锁或审批平层；
架构保留原 Runtime 观察 ABI 并在实例层适配完整请求，拒让 PortableHost 依赖桌面模块。

迭代日志保留首轮编译失败（适配入口定位、可见性、异步递归装箱与 PluginContext 构造修正），
见 `s5-check-first.log`、`s5-check-second.log` 和 `s5-cli-first.log`；这些轮次不记为通过。
当前环境 hooks 没有执行位，提交前显式以 bash 运行原 pre-commit / commit-msg，再通过 commit.ps1 的显式 author、secret / neutral 与 ArchStrict 提交；不改变全局 hooksPath 或跳过门禁。
