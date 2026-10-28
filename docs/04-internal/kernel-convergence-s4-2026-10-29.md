# Kernel Convergence — S4 Profile / Memory authorization mapping

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s4-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: S4 已落地，指定窄门禁通过；整体 Production Convergence 仍为 Partial
Author: Codex
```

## 基线与交付范围

分支 `refactor/kc1-kernel-convergence`；前轮从 clean 工作树开始 S4，HEAD 为 `47e1ced9`，
S3 源码为 `5adc0493`。本轮续接同一 HEAD 上的 S4 脏工作树；
`git fetch origin && git status && git log -5 --oneline` 成功，沿用现有实现，未重写 S4。
依据[债务图 D4 / S4](kernel-convergence-remaining-debt-map-2026-10-29.md)、
[S1](kernel-convergence-s1-2026-10-29.md)、[S2](kernel-convergence-s2-2026-10-29.md)、
[S3](kernel-convergence-s3-2026-10-29.md)、ADR-0018～0022 与
[历史验收账](kernel-convergence-final-acceptance-2026-10-28.md)；取材仅为本地源码与文档。
不改历史验收结论，不重造 Registry、Dispatcher、资源工厂、MCP 生命周期或多实例 Runtime。

**IMPLEMENTED（Host 显式授权组合）**：`ProfileMemoryMappings` 把具体 MemoryCoordinator / 来源 scope、
Preference 来源会话和认知 store 对象绑定到完整 `(ProfileId, NamespaceId)`，支持撤销。
`ProductionModules::register_with_profile_mappings` 保留原工具模块装配与注册路径；
`ProfileMemoryMappings::register_into` 可用于可裁剪 Host。
映射没有默认启用；原桌面 bootstrap 未选择该 API，本批不做 S5。
真实 Runtime 的可选映射路径经验证后，证据记在下节；整体等级保持 Partial。

无 principal 的原全局认知继续调用同一个旧业务对象与旧数据库，不重建、不自动认领或迁移旧记录。
S3 的 principal fail-closed 仍在 `run_hook`，包括 with_module / with_behavior；只有当前 Host grant
明确列出的 callback 才能通过，且仍需要原内部 capability 的 scope 与治理授权。注册不授权。

## 授权、所有权与数据源

| 数据面 | 授权读取 | 可写范围 / 归属 |
| --- | --- | --- |
| Global | Host 显式列入 read_scopes | 来源记录只读；不会把 Global 变成目标的写分区 |
| Session / 无 metadata 的旧记录 | 精确来源 Session scope；缺 metadata 仍按真实旧 Session fail-narrow | 新记录落目标私有分区，不覆盖旧 Session |
| Project / Persona / User | 精确身份 scope；无隐式全局或 typed identity 扩权 | 只读来源；目标私有写另行授权 |
| Working / Episodic | 同一排序管线先过滤授权 scope，再检索；工具边界再查 scope 与治理内容 | `(ProfileId, NamespaceId)` 分区；更新/删除仍核对 owner、冻结审批、revision CAS 与 protected |
| Preference | 独立 store grant；SharedPreferenceSource 只查显式来源会话并核对返回行的原 session | 共享 adapter 拒绝 record / forget；私有学习使用目标专属 store 对象与 WritablePrivate 声明 |
| Persona 常量注入 | 显式只读共享 store 可选 | 共享注入不授予 consolidation 写权；私有 Persona 沉淀须绑定专属 store |
| Typed materialization / recall | typed candidate 须精确匹配授权 scope；不沿 typed identity 自动扩权 | private sink 属于目标 store，提交前核对 scope / 取消并固定真实来源；CAS 等仍交原 sink |
| Organ / Bio 等可变状态 | 不映射则不执行；不得把全局状态当私有缺省 | WritablePrivate 绑定目标专属对象；Organ 不提供只读共享声明，映射拒绝这种声明 |
| Partner | 可选 with_read_only_snapshot：只读已有记录，不创建或演化 | 私有模块用专属 store 与身份；共享快照不接收私人互动写回 |

`CognitiveStoreBinding.store` 是可信 Host 对后端资源的稳定标识，必须覆盖模块携带的实际 store / 状态资源，
不能为同一资源换名以规避所有权。授权表拒绝同一 store 的跨目标可写共享、共享与私有混用，以及仍在绑定中的
同一模块 Arc 跨目标可写复用；撤销不删除 store 的归属记录。Host 本来即可直接持有数据库，资源标识不是对恶意
Host 的沙箱。模型只能拿到原工具 schema，不能提交 principal、namespace、scope、store 或 grant 参数。

`ProfileMemoryAccess::recall_module` / `writeback_module` 配置原 MemoryRecallModule / MemoryWritebackModule，
可继续链式配置原排序、proactive、访问回执、调优、materializer、typed sink 与 consolidation。
带访问回执 store / recorder 或自调优 wire 的 recall 属于可写状态，不能声明为 ReadOnlyShared；
须以 WritablePrivate 的目标专属对象绑定，原回执与调优语义仍执行。
原生 MemoryModule 可选择同一个映射；当 Host 已选择兼容 recall 时，其自动 recall 只记审计消费，
避免自动重复注入；模型明确调用 retrieve 仍是普通受治理工具调用。

精确 scope 召回关闭未映射的旧无 scope Preference / graph 附件，并取消 typed identity 的隐式可见性。
这些数据需要自己的 store / scope grant，不能因 coordinator 被选中就一并继承。
后台 organizer 的 retrieve / write 仍走原 Dispatcher，只有私有可写映射可启动；不授予 delete / update。
启用映射端口后，未映射 MemoryModule 请求明确返回 namespace_denied，绝不换查构造器中的全局 coordinator。

## 撤销、切换与来源

Runtime 只持通用 `PrincipalAuthorizations`，不知道 Memory / Persona 策略。
每个授权有不可复用的 epoch 和撤销信号；回合与审批恢复在统一执行入口捕获 epoch，撤销取消该执行 future。
Dispatcher 覆盖 context 中的 principal_epoch；receipt 额外绑定 principal / epoch，执行前后重查，不能由调用参数伪造。
模块子任务还核对父会话的 principal / epoch；旧 invoker 不能借新的 child 会话接上新授权。
Native worker 在等写锁之后与返回前重验；organizer 保留原 epoch 到子任务结束。

epoch 与 Session 一起保存并随 fork / serde reload 保留；旧 epoch 的会话、审批与工具上下文不能采用替换后的 grant。
撤销后，Host 必须创建新会话才能继续使用新授权；旧持久 transcript 不会被自动清空或改标。
带 epoch 的私有会话在未装原授权端口的 Runtime 中也拒绝执行；Host 重启后的授权不自动复活旧缓存。
原 activate_profile 仍停止并销毁旧 main，使用新 Session；本批不重写该实例服务。
已在撤销前完成的存储操作不回滚；撤销不是数据删除或已经发送内容的撤回承诺。

Episode.session_id 继续作为私有存储分区键，但 provenance.source_session / source_request / source_trace
来自真实执行上下文。物化输出不能伪造这些来源；召回 source_ref 与访问回执也保留真实会话，
不把分区键冒称原始来源；缺少真实来源时不伪造 source_ref。
私有写回 ID 绑定真实 request，避免跨会话 Provider 响应 ID 碰撞；无 principal 的原稳定 ID 与物化 Session scope 保持。
Provider 动态数据仍用不可信 V1 工具对，业务历史过滤、治理、预算和静态 Host policy 保持。

## Runtime 对抗证据

新增 `profile_memory_authorization` 使用真实 Runtime / AgentRuntime / ProductionModules / MemoryModule，覆盖：

- 只有获授 Profile 能读 Global / Project / Persona 与共享 Persona；其他 Profile 即便拿到工具 scope，仍不能读、写或收到该缓存。
- Production 兼容 recall 与原生 Memory 同选只投影一次，动态文本不进入 system；私有写来源保持真实 session / request / trace。
- 共享记录 mutation 在冻结审批通过后仍由存储边界拒绝；额外 namespace / visible_scopes 参数被原 schema 拒绝。
- 撤销取消等待中的模型响应；旧会话、fork、reload、无授权 Runtime 和旧审批不能接新 grant。
- Profile switch 取消旧执行，新 main 不接旧 memory / response；后台 organizer 撤销后不能落 summary。
- 共享 Partner 不演化，Preference 只读源会话、Organ 私有状态不流入未授权 Profile；非法可写复用与只读共享声明拒绝。
- typed / generic materialization 保持私有分区和真实来源，提取结果尝试写 Global 被边界拒绝。
- 即便 scoped backend 返回过宽候选，也重查 scope；NULL metadata 不自动变全局；授权集合内排序与原 coordinator 一致。

另覆盖有效映射下的私有 miss：即使 coordinator 带旧 Global 与 typed identity，也不自动扩权；
只有显式 User scope 的目标可读 typed 候选。

原 convergence_memory、cognitive_convergence_vertical、production_cognitive_v1、portable_host 及相关
runtime / memory / assembly / plugin 回归全部保留；不删测试、不加 ignore、不放宽原安全断言。

## 验证账

环境：2026-10-29（Asia/Shanghai），Linux x86_64，rustc / cargo 1.97.1，Python 3.13.5，PowerShell 7.5.4。
本轮仅执行 owner 指定的窄门禁；Cargo 构建 jobs=2，保留默认 profile 与 debug assertions。
逐项命令、UTC 日期、日志与退出码落在 `/workspace/apeireth-codex/s4n-results.json`、
`s4n-*.log` / `s4n-*.exit`。执行脚本为同目录 `s4n-run.py`；以下结果按落盘退出码复核。
测试数字不汇总、不替代 [test-baseline](../03-reference/test-baseline.md)。

| 命令 | 日志（同名 `.exit`） | 退出码 |
| --- | --- | --- |
| `cargo test -p apeireth-runtime-assembly --test profile_memory_authorization --locked -j 2` | `s4n-01-profile.log` | 0 |
| `cargo test -p apeireth-runtime-assembly --test convergence_memory --locked -j 2` | `s4n-02-memory.log` | 0 |
| `cargo test -p apeireth-runtime-assembly --test cognitive_convergence_vertical --locked -j 2` | `s4n-03-vertical.log` | 0 |
| `cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2` | `s4n-04-production.log` | 0 |
| `cargo test -p apeireth-runtime-assembly --test portable_host --locked -j 2` | `s4n-05-portable.log` | 0 |
| `cargo test -p apeireth-runtime --all-targets --locked -j 2` | `s4n-06-runtime.log` | 0 |
| `cargo test -p apeireth-memory --all-targets --locked -j 2` | `s4n-07-memory.log` | 0 |
| `cargo test -p apeireth-plugin --all-targets --locked -j 2` | `s4n-08-plugin.log` | 0 |
| `cargo clippy -p apeireth-plugin -p apeireth-memory -p apeireth-runtime -p apeireth-runtime-assembly --all-targets --locked -j 2 -- -D warnings` | `s4n-09-clippy.log` | 0 |
| `cargo check -p apeireth-runtime-assembly --no-default-features --locked -j 2` | `s4n-10-minimal.log` | 0 |
| `cargo test -p apeireth-runtime-assembly --no-default-features --features memory,profile-store --test portable_host --locked -j 2` | `s4n-11-portable-memory.log` | 0 |
| `cargo fmt --check` | `s4n-12-fmt.log` | 0 |
| `python scripts/arch_guard.py --strict` | `s4n-13-arch.log` | 0 |
| `python scripts/arch_doc.py --check` | `s4n-14a-arch-doc.log` | 0 |
| `python scripts/docs_links.py --check` | `s4n-14b-docs-links.log` | 0 |
| `python scripts/docs_index.py --check` | `s4n-14c-docs-index.log` | 0 |

前轮全量证据保留于 `s4-full-results-before-recall-ownership.json` 和相应日志 / 退出码，
只代表 recall ownership 微调前的源码；前轮微调后定向证据为 `s4-recall-ownership.log` / `.exit`。
被停止的重复全量执行不记通过。本轮没有重跑 workspace test 或 workspace clippy，
也不把微调前全量结果冒称最终源码的全量验证。

本轮源码与前轮微调后的 `s4-source-manifest.json` 完全一致；复核脚本为 `s4n-finalize-notes.py`，
冻结证据为 `s4n-evidence.json`。LOCKED 与 Cargo.toml / Cargo.lock 无差异；原测试文件未改，
新增对抗用例不加 ignore。全部指定窄门禁通过后，源码与随行说明先提交；随后单独将受影响
claims 的 `verified_at` 锚定该源码 SHA，并从原真源生成 STATUS / wiring；级别和默认开关不抬升。


## 边界与后续

整体 Production Convergence 仍为 Partial。S5 桌面受管默认主入口、S6 剩余兼容与最终平台验收留后续；
Mesh / NAT / P2P 保留，可裁剪 Host 不引入 desktop 依赖。没有真实外部 API、Android 设备、桌面 UI 或硬件联调实测，
不声明 HARDWARE VALIDATED。不改 LOCKED、workspace.version、Cargo.lock 或外部依赖；不加架构豁免。

O-6：总体先关闭真实跨身份读写与缓存复活路径，拒绝只隔离召回文本；系统复用原治理、排序、CAS、typed sink、
任务与 Profile 切换，拒绝复制存储或绕过 Dispatcher；架构把通用 epoch 放 Runtime、来源与 store 策略放 assembly，
拒绝领域策略塞进 Kernel 或越界改 S5。
