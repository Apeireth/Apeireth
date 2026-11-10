# A1b 独立启动定义与进程测量

[Document-Meta]
Document: docs/04-internal/kernel-convergence-a1b-startup-definitions-2026-10-29.md
Version: 1.1 (daily normal_start APPROVED; original evidence retained)
Last-Modified: 2026-10-29
Status: APPROVED；daily normal_start 达标，Kernel 按已批准定义重新判定
Author: Codex

## 当前批准与日常判定（2026-10-29）

日常核心启动定义已由 **repository owner / user (xiao ouyang)** 于 **2026-10-29（Asia/Shanghai）** 批准，
用户答复词为 **「当然」**。状态由 PROPOSED_UNAPPROVED 改为 **APPROVED**；依据本次明确授权记录，毋须再确认。
批准定义原文：

> With existing-DB **normal_start P50 ≤ 10 ms** as the **daily core startup criterion**
> (existing DB, full process restart, normal OS caches; same READY/session-pair assertions as A1b).

使用本笔记保留的 Dev normal_start **P50=7.5642 ms（≤10 ms，PASS）**，P95=9.0289 ms、n=30。
这是 A1b 既有测量，不是本次新增样本；原 spawn→READY、独立进程/Runtime shutdown、已有 DB 与
session-pair-v1 内容/断言全部保持。结合 A4 所列必需软件 PASS / GATE_FULL=0，
[Final Acceptance 当前判断](kernel-convergence-final-acceptance-2026-10-28.md) 为
**Kernel Convergence Complete (under approved startup definition)**。

边界仍显式保留：

- A4 **OLD_COLD_P50=11.4143 ms，原 <10 ms 未达（unmet，历史）**。A1/A1b 的更早数值与旧 benchmark 原样保留；旧 gate 不再作为日常定义的当前阻塞项。
- 真正冷缓存 `cold_start` 仍 **UNAVAILABLE**：真实 drop_caches 写入被拒绝，Dev/Release n=0；是环境限制，不冒称 pass，不用 normal_start 补数。
- 仅日常 Dev normal_start P50≤10 ms 获批准；first_run 目标与 relative+absolute regression detection 继续 **PROPOSED / advisory**。原表中 P95/Release、cold_start、warm_resume 附加预算也未自动批准。
- 没有历史同定义性能对照，不能把此次规则批准称作新的性能优化或“无回归”；没有改报告脚本/旧 gate 输出或关键初始化。
- Phase D 的远端 required checks、mergeStateStatus 和自动删除策略冲突独立保留，本次不授予 PR 合并或 E 清理许可。

以下直至文件结尾为 **A1b 批准前的原始证据与提案记录**；其中 Partial、PROPOSED_UNAPPROVED 与“待批准”是历史口径，
`normal_start` 日常 P50 的现行批准状态以上段为准，其余提案不因历史措辞而被静默丢弃。

## A1b 原始判定与提案（批准前历史冻结口径）

**正式验收仍为 Partial。** 原 `cold-start` 的本批最终 Dev P50 为 **11.5277 ms**，
仍未满足原 **<10 ms**。新 `normal_start` 的 Dev P50 为 **7.5642 ms**，
Release P50 为 **3.2616 ms**，但这些数值不替换原门槛。
[A1 历史账](kernel-convergence-a1-cold-start-2026-10-29.md) 中的
**11.9248 ms** 及优化前后数据保持原样；[Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md)
继续 Partial。本批不改变 README、正式性能目标或生产初始化行为。

**明确提案，待用户批准：以 `normal_start` P50 ≤10 ms（已有 DB、完整进程重启、正常 OS 缓存）
作为日常核心启动验收指标。** 建议继续使用本批 Dev 环境作为固定日常口径，Release 单独报告。
提案状态为 `PROPOSED_UNAPPROVED`，本次指令允许提出定义，不等于批准切换正式 gate。
若判定 Complete 仅依赖采用该新定义，应写
**“Complete pending approval of the new startup definition”**，并在原规格下继续 Partial。
本批还缺真实冷缓存测量，不宣告整体 Complete，也不将该缺口记成通过。

起点为 A3 docs commit `7a5718fd65dcd63c567eb8f3575b8e3e8794b426`；
新增 harness/scripts 的源代码提交为 `100b7aa9ba9695a1de8ed0fdb09d878fc69aff2d`。
级别为 **IMPLEMENTED（bench-only）**，产品 Runtime、memory migration、Host 接线均未修改。
原 `cold-start`、`assembly`、`profile-cold-start`、`idle-footprint`、`support` 文件与起点逐字节一致。
新命令可由 `list` 发现，显式单项执行，不加入旧 `all` 或旧 `ROW` 目标汇总。

## 四种定义与同一就绪合同

| 命令 | 计时与生命周期 | DB / 缓存条件 |
| --- | --- | --- |
| `first_run` | 父进程开始 spawn 前计时；子进程创建 Tokio、完整生产组装、建立初始会话、恢复并校验会话；父进程收到有效 READY 回执后停止计时 | 每个样本目录中两库均不存在；正常 OS 缓存，不声称清空文件缓存 |
| `normal_start` | 相同 spawn→READY 边界；每个样本重新启动可执行程序和 Tokio，无进程内池或 Runtime 复用 | 独立 seed 子进程建立同一 fixture，显式 shutdown 并 wait 退出后开始采样；已有 DB，正常 OS 缓存 |
| `cold_start` | 与 normal_start 相同的子进程、数据、组装和 READY 条件 | 每次启动前 `sync` 成功，然后真实写 `/proc/sys/vm/drop_caches` 为 `3`；cache drop 在计时外；失败则不可出冷缓存统计 |
| `warm_resume` | 同一子进程中 Runtime 已完整组装；父进程发送 RESUME 到收到有效回执；子进程实际 load 持久会话并计算 surface view | 同一已有 fixture；Runtime 保留，只用于此模式；不重新初始化 DB 或 Runtime |

前三种模式的正常收尾均为：父进程停止计时后发 STOP，子进程等待 `Runtime::shutdown`，
检查失败列表为空、drop Runtime/Tokio，父进程 wait 成功后才可开始下一样本。
进程创建、动态装载、子进程主入口、Tokio 与响应回执开销均进入前三种模式的总耗时；
目录准备、seed 和关闭耗时不计入启动样本，但均同步完成，没有未关闭 Runtime 被下一样本复用。
编译时间不进入计时。异常退出或错误回执使命令失败，不回落到同进程测量。

所有模式沿用 [assembly.rs](../../benches/src/assembly.rs) 的相同生产组装：
记忆与会话库打开/迁移并行，构造相同记忆召回/写回与文件/搜索/仓库模块，注册、build，
检查能力和行为模块快照非空。无跳过必要迁移、异步延后关键初始化或裁剪模块。
与旧 harness 一样，未装 LLM provider 插件、未启用额外 opt-in 模块；不测联网、模型加载或完整 CLI listener 启动。
ProviderRouter 创建包含在 runtime_build 中，没有把未单独计时的外部 Provider 创建写成零。

固定 fixture 为 `session-pair-v1`：Memory 库具有完整 schema、无业务记忆条目；Session 库有一个
固定 ID、两条 user/assistant 消息、两条 append log、revision=2 的会话。
fixture 时间固定，只作用于测试数据，所有性能计时仍用真实 `Instant`。
first_run 的首次会话写入在 state_init 段内；已有库路径不改数据、不在缺失时自动创建会话。
每次 READY/RESUMED 都验证原消息、log/revision 与重建后的 surface 相同，错库/缺会话直接失败。
因此 first_run 必须包含首次写入成本，其余模式使用相同内容；不能将 first_run 总数当作旧组装区间的同口径替代。

warm_resume 测量的是已有 Runtime 的会话恢复和可读 surface，不是新模型回合、进行中 Provider 请求重启、
操作系统休眠恢复或 UI 动画。Tokio/存储/模块的初始化在 `warm_resume_setup` 单独记录，**n=1**；
它们在每次 resume 中不重新执行，属于“不适用”，不是测得零成本。resume 样本仍实际读库/反序列化/重放。

## 环境与矩阵

测量日期为 **2026-10-29**（Asia/Shanghai；日志 UTC），Linux x86_64、kernel 6.12.94+、
glibc 2.41、Intel Xeon Processor 虚拟机、MemTotal=16386368 kB，数据目录位于 `/tmp` 的 ext4。
工具链 rustc/cargo **1.97.1**；完整版本与环境见 `/tmp/apeireth-a1b/environment.json` 和 `a1b-environment.log`。
Dev 使用 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`，仍为 unoptimized、debug assertions 开启。
Release 使用原 opt-level=3 / fat LTO / codegen-units=1 / strip 配置，仅设置 `CARGO_INCREMENTAL=0`。
未设置 BENCH_SCALE/BENCH_QUICK 的正式测量默认 **30 samples、2 warmups**，逐样本 RAW 已记录。
P50/P95 均为 nearest-rank；样本较少，不能据此宣告高置信尾延迟上界。

下表单位 ms；最终 Dev 轮已等构建结束并逐命令等待退出，未并发跑基准。
Release 的逐命令数据另列，不混用 profile，也不以 Release 成功替换 Dev gate。

| Profile | 模式 | n | P50 | P95 | mean | max | 日志前缀 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Dev | first_run | 30 | 18.5946 | 26.2623 | 20.4073 | 47.0813 | a1b-measured-dev-first_run |
| Dev | normal_start | 30 | 7.5642 | 9.0289 | 7.7353 | 9.9780 | a1b-measured-dev-normal_start |
| Dev | cold_start | 0 | 未实测 | 未实测 | — | — | a1b-measured-dev-cold_start |
| Dev | warm_resume | 30 | 0.0937 | 0.1379 | 0.1136 | 0.4730 | a1b-measured-dev-warm_resume |
| Release | first_run | 30 | 10.1184 | 12.7339 | 13.1529 | 103.0883 | a1b-release-first_run |
| Release | normal_start | 30 | 3.2616 | 3.9829 | 3.3142 | 4.2399 | a1b-release-normal_start |
| Release | cold_start | 0 | 未实测 | 未实测 | — | — | a1b-release-cold_start |
| Release | warm_resume | 30 | 0.0173 | 0.0351 | 0.0220 | 0.1370 | a1b-release-warm_resume |

两个 cold_start 命令都在真实打开 drop_caches 写入端口时收到 **Permission denied (os error 13)**，
进程 exit=1；没有输出有效 STARTUP 时长，没有用 normal_start 数字填空，没有借内存压力或建议驱逐假装清缓存。
当前用户为非特权用户；本批没有提权或改变系统权限。需在允许真实 cache drop 的隔离 runner 补测。
因此这两个模式的各阶段 P50/P95 同样未实测，不能合成估值。

first_run 的尾部异常样本保留在 mean/max 与 RAW 中，未剔除。
早期诊断轮 `a1b-dev-*`、`a1b-final-dev-*` 和最初开发失败日志均保留；其中一个 Dev 启动序列
未显式等前一外层命令结束，故最终报告使用上述串行轮，不混合挑选样本。
旧 gate 本批早期 P50 为 11.4742 ms、中间轮 12.9655 ms，最终 11.5277 ms；均未达标，
这些波动不构成产品性能优化或回归归因，本批未优化产品源码。

## 分段结果与归因

下表每格为 **P50 / P95 ms**，每列 n=30。存储两段并行，不能将各阶段分位数相加。
assembly 包含两个存储、模块、runtime_build 和原 ready 快照；total 另含进程/Tokio/状态恢复。
process_and_ipc 是每样本 total 减去 Tokio、assembly、state_init、resume 的差，包含主入口、检查、协议与调度开销，
不是单独隔离测量的内核 spawn 成本。若内部段超过墙钟即失败，不钳成零假装正确。

| 阶段 | Dev first_run | Dev normal_start | Release first_run | Release normal_start |
| --- | ---: | ---: | ---: | ---: |
| Tokio | 0.5816 / 0.9198 | 0.5253 / 0.8960 | 0.4355 / 0.5699 | 0.3727 / 0.5303 |
| Memory open + schema/migration | 11.6040 / 19.4395 | 1.8084 / 2.2866 | 7.1890 / 9.9052 | 1.0658 / 1.3469 |
| Session open + migration（与 Memory 并行） | 4.1907 / 5.1774 | 0.9259 / 1.3354 | 3.2726 / 4.9653 | 0.7099 / 1.0507 |
| Module construction | 0.5804 / 0.6703 | 0.4873 / 0.6184 | 0.1438 / 0.1985 | 0.1288 / 0.1662 |
| Runtime registration/build | 1.1191 / 1.2932 | 1.0394 / 1.3186 | 0.2751 / 0.3786 | 0.2578 / 0.3206 |
| Assembly 到原 ready | 13.5670 / 21.5104 | 3.6024 / 4.1105 | 7.7782 / 10.6390 | 1.5354 / 1.9781 |
| 初始会话写入 | 0.4662 / 0.6970 | 不执行（已有相同数据） | 0.2910 / 0.5017 | 不执行（已有相同数据） |
| 初次恢复与校验 | 0.9020 / 1.1247 | 0.8115 / 1.0972 | 0.5191 / 0.8280 | 0.3105 / 0.4676 |
| process_and_ipc 差值 | 2.9025 / 3.5927 | 2.5292 / 3.1539 | 1.1152 / 1.5124 | 0.9518 / 1.2816 |

warm_resume 的子进程实际恢复段为 Dev **0.0814 / 0.1231 ms**、Release **0.0091 / 0.0164 ms**；
加上父子请求/响应即为矩阵 total。初始化各段的单次 setup 原始值另在对应日志，不能把 n=1 当成初始化分布。
首建主要成本仍在 Memory schema 路径；正常重启不用再施加已完成迁移，但仍检查版本并完整装配。
这里没有按段分离纯文件系统成本，也没有把近似差值解释为系统调用级别归因。

## 原 benchmark 的连续性

旧连字符 `cold-start` 的原始负载保持：同一基准进程提前建好多线程 Tokio，2 次 fresh warmup，
随后 30 个样本各建新目录，测 SQLite Memory/Session 打开与迁移、相同 ProductionModules、注册/build/ready；
目录创建、Tokio 创建、Runtime drop 不计时，无 session-pair-v1 初始化或恢复。
它测的是**首次建库的组装**，没有重启测量进程，也没有 drop OS 缓存。
旧输出中的“真冷”是历史“新数据目录/无已有 DB”用语，不能当作本批 `cold_start` 的冷文件缓存证据。
本批保留旧源文件与历史原始输出，不追溯改写 A1 数据。

| Profile / 检查 | P50 | n | 日志前缀 |
| --- | ---: | ---: | --- |
| Dev legacy cold-start | 11.5277 ms，原 gate 未达标 | 30 | a1b-measured-dev-cold-start |
| Release legacy cold-start | 6.0429 ms，仅 Release 参照 | 30 | a1b-release-cold-start |
| Dev idle-footprint | 30.0703 MiB，≤35 | 5 | a1b-measured-dev-idle-footprint |
| Release idle-footprint | 12.2812 MiB，≤35 | 5 | a1b-release-idle |

## 待批准预算与回归检查

以下均为**建议预算，不是正式 gate**。首次建库允许 schema 材料化、写入和进程创建成本；
正常启动单独约束常用入口。冷缓存预算是待校准起点，需先取得真实样本再评审。

| 模式 | Dev 建议 P50 / P95 ms | Release 建议 P50 / P95 ms | 用途 |
| --- | ---: | ---: | --- |
| first_run | ≤30 / ≤60 | ≤20 / ≤40 | 首建体验与 schema 回归监测；给完整必需初始化留出空间 |
| normal_start | ≤10 / ≤20 | ≤10 / ≤20 | P50≤10 的新日常核心验收提案；P95 是附加趋势预算 |
| cold_start | ≤25 / ≤50 | ≤20 / ≤40 | 真实文件缓存清空后的建议预算，目前未校准、未实测 |
| warm_resume | ≤1 / ≤2 | ≤1 / ≤2 | 已有 Runtime 的状态恢复，不与进程启动替换比较 |

[startup_report.py](../../scripts/startup_report.py) 汇总原始 STARTUP 回执、保存环境与分段，并始终另报旧 Dev gate。
以本批 `startup-report.json` 作为新定义的首份基线；目前没有历史同定义的性能对照，不能声称“无回归”。
后续 `--baseline` 比较要求环境、fixture、profile、样本数相同；P50 增长同时超过 **20% 和 1 ms**，
或 P95 增长同时超过 **30% 和 2 ms** 时标记 REGRESSION、exit=1，尤其覆盖 first_run / cold_start。
环境漂移拒绝归因；缺失模式标 INCOMPLETE、exit=2，不把冷缓存缺失当作 NO_REGRESSION。
绝对预算另作为提案展示，未接入 gate_full，也不自动改变正式验收。

## 复跑与验证

完整证据位于 `/tmp/apeireth-a1b/`，收口归档到 `/workspace/apeireth-codex/a1b-evidence/`；
每条命令日志和同名 `.exit` 记录真实退出码。源文件 SHA256 见 `a1b-source-sha256.json`，
环境见 `environment.json`，汇总见 `startup-report.json`。例如：

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- first_run
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- normal_start
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- cold_start
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- warm_resume
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- cold-start
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- idle-footprint
```

Release 用 `CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run --release -p apeireth-bench-harness --locked -j 2 -- <同一命令>`，
预先 build 后逐命令等待退出，不在编译负载下采样。cold_start 的权限失败应如实保留非零退出码。
报告复现时给 `--logs` 传矩阵的八个日志文件，传 `--environment environment.json`、
`--legacy-dev a1b-measured-dev-cold-start.log`、`--out startup-report.json`；
未来同口径新一轮可加 `--baseline <本批 startup-report.json>`，不要将同一轮自比称作性能回归证据。

| 验证 | 命令主体 / 证据前缀 | 结果 |
| --- | --- | --- |
| 新 harness 合同 | cargo test -p apeireth-bench-harness --all-targets --locked -j 2；a1b-harness-tests | exit 0；独立 PID、warm 单 Runtime、子进程回收、缺库/缺会话拒绝与首建拒绝复用 |
| 回归策略 | python -m unittest discover -s scripts -p test_startup_report.py -v；a1b-report-tests-final | exit 0；首建/冷缓存增长、噪声阈值、环境变化与缺失证据 |
| Python 编译 | python -m py_compile scripts/startup_report.py scripts/test_startup_report.py；a1b-python-compile | exit 0 |
| clippy | cargo clippy -p apeireth-bench-harness --all-targets --locked -j 2 -- -D warnings；a1b-clippy | exit 0 |
| fmt | cargo fmt --check；a1b-final-fmt | exit 0 |
| arch | python scripts/arch_guard.py --strict；a1b-final-arch | exit 0 |
| 文档真源 / 链接 / 索引 | docs_truth.py --verify --strict / --check；仓库 pre-commit 文档门禁 | 收口独立复核，回执归档 |

测试/clippy 使用 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`。
未增加依赖、ignore 或削弱既有断言，LOCKED 项、Cargo.lock 与版本均未动。
本批不运行 full gate_full，A3 的 CLI TCP 夹具复验缺口仍保留；下一步为 **A4**，本批不启动 A4。
