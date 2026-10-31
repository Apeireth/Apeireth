# A1 冷启动优化与复测账

[Document-Meta]
Document: docs/04-internal/kernel-convergence-a1-cold-start-2026-10-29.md
Version: 1.1 (A1 源码提交与文档真源收口)
Last-Modified: 2026-10-29
Status: Partial；Dev 首建冷启动仍超过原目标
Author: Codex

## 结论与冻结范围

原口径 Dev 冷启动 P50 从本轮修改前 **16.1917 ms** 降到修改后 **11.9248 ms**，
仍未达到原目标 **<10 ms**；不以基准进程 exit 0 冒充阈值通过。
空闲内存 P50 **30.1133 MiB**，满足原同数值 **35 MiB** 目标。
[Final Acceptance v3.1](kernel-convergence-final-acceptance-2026-10-28.md) 保持 **Partial**。
前轮 18.3321 ms / 29.1055 MiB 是先前观测，不替换本轮同环境前后对照。

基线 `a1b0a7260cc674fe6657c1dd5092a20da477cca5`，分支
`refactor/kc1-kernel-convergence`。优化测量阶段按只改源码/bench 的指令留下未提交改动；
本次提交收口已将相同源码冻结到 `1497fe7fc18ccf557e9d13aa58504b79bedd2f06`，文档与状态真源另行提交。
未改 main；推送和外部 STATE.md 的最终回执由本次提交收口会话记录。
源码冻结清单为 `/tmp/apeireth-a1/a1-source-sha256.txt`。
证据均存 `/tmp/apeireth-a1/`，命令日志各有同名 `.exit`；
`run.py` 先落日志、记录子进程真实退出码，再输出摘要，未使用管道退出码判定。

日期为 2026-10-29（Asia/Shanghai；日志时间戳为 UTC），平台 Linux x86_64，
rustc/cargo 1.97.1；完整工具链、页大小、基准环境和构建配置见
`a1-optimize-environment.log`。磁盘未达到清理阈值，本轮无缓存清理。

## 改动与等价边界

初始化优化在共享 memory migration runner 中，级别为 **PRODUCTION WIRED**：
现有 `SqliteMemoryStore` 和池化 memory 迁移入口继续调用同一 runner，
没有只服务基准的替代实现、默认开关变更或延迟必需初始化。

- 在原 `BEGIN IMMEDIATE` 写锁内确认 `sqlite_master` 完全为空，才使用
  `migrations/fresh_tables.sql` 一次声明 episodes、sessions、notes 的最终列结构与补充索引。
  它完整施加 V3–V5 的列、默认值与索引效果，避免对空表反复 ADD COLUMN。
- 其余 schema、全部原触发器和全部 migration 版本记录仍在同一事务内完成。
  V1 的 INIT_SQL 已经执行，不再重复执行相同初始化；簿记 INSERT 复用 prepared statement，
  真空 schema 不逐版本查询不存在的历史记录。
- 任何已有对象都走历史 SQL 升级路径；已迁移库仍先验证 schema 家族并沿用原版本短路。
  事务失败整体回滚，并发首开在获取写锁后重判是否为空。没有改 WAL、同步级别、
  foreign_keys、busy_timeout、持久化策略、历史 migration SQL 或迁移清单。
- 原迁移单测移到 `migrations/tests.rs`，保留原用例，并增加独立历史路径与首建路径的
  schema 对照、逐历史版本升级后的数据检查、容量不足时的整体回滚与并发首开检查。
  对照含全部 schema 对象、列顺序/类型/默认值、外键、索引 SQL、触发器正文和版本名称；
  原 append-only、治理与存储行为回归继续执行。

`profile-cold-start` 仅增加 `profile-runtime-restart` 诊断结果：销毁首个 Runtime，
在同目录重新执行完整生产组装并计时。`cold-start`、`idle-footprint` 的阈值、样本量、
warmup、工作负载和 assembly 实现没有修改；诊断结果不进入 `all` 或原性能目标行。
未引入外部依赖，Cargo.lock、workspace.version 和 LOCKED 项无改动。

## 复现命令与口径

Dev 沿用指定环境（不生成 debug info，但仍是 unoptimized Dev，debug assertions 保留）：

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run -p apeireth-bench-harness --locked -j 2 -- cold-start
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run -p apeireth-bench-harness --locked -j 2 -- idle-footprint
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run -p apeireth-bench-harness --locked -j 2 -- profile-cold-start
```

Release 使用仓库原 `[profile.release]`：opt-level=3、fat LTO、codegen-units=1、strip=true，
没有为测量覆盖这些选项。构建不计时，随后用同一个 harness、默认样本与原计时边界：

```bash
CARGO_INCREMENTAL=0 cargo build --release -p apeireth-bench-harness --locked -j 2
CARGO_INCREMENTAL=0 cargo run --release -p apeireth-bench-harness --locked -j 2 -- cold-start
CARGO_INCREMENTAL=0 cargo run --release -p apeireth-bench-harness --locked -j 2 -- profile-cold-start
CARGO_INCREMENTAL=0 cargo run --release -p apeireth-bench-harness --locked -j 2 -- idle-footprint
```

冷启动/诊断均为默认 30 samples、2 warmups，未设置 BENCH_SCALE 或 BENCH_QUICK。
首建每轮创建全新目录；内存库参照施加相同完整 schema，避免文件 I/O。
“已有库重建”是同进程销毁 Runtime 后重新组装，已有数据库与 OS 缓存热，
不是重启操作系统、清空页缓存或测量进程启动；异步运行时创建、目录创建和 Runtime 销毁均不计时。
首建与重建均保留原 ready 快照断言，不把单库 reopen 当作完整 Runtime 重建。

## 前后与矩阵

下表单位均为 ms；空闲驻留另用 MiB。

| 同环境 Dev cold-start | min | mean | P50 | max | 日志前缀 |
| --- | ---: | ---: | ---: | ---: | --- |
| 修改前 | 14.9073 | 16.3661 | 16.1917 | 18.6666 | a1-optimize-before-cold |
| 首轮直接声明完整列结构 | 10.8040 | 12.4486 | 12.2331 | 14.8651 | a1-optimize-round1-cold |
| 最终源码 | 10.3293 | 13.0146 | 11.9248 | 42.8533 | a1-optimize-final-dev-cold |

后续试验中的同步 PRAGMA 顺序调整没有显示出足以排除波动的收益，最终已撤回；
`a1-optimize-round2-cold.log` 保留中间实测，不能用作最终源码证据。
最终空闲内存为独立子进程原探针读数，见 `a1-optimize-final-dev-idle.log`。

<!-- A1_MATRIX_START -->
| Profile | 初始化方式 | min ms | mean ms | P50 ms | max ms | 日志前缀 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Dev | First init (empty DB) | 10.3293 | 13.0146 | 11.9248 | 42.8533 | a1-optimize-final-dev-cold |
| Dev | Existing DB, full Runtime reassembly | 2.4584 | 2.6596 | 2.6157 | 3.0500 | a1-optimize-final-dev-profile |
| Release | First init (empty DB) | 5.3061 | 6.5283 | 5.9860 | 9.4111 | a1-optimize-release-cold |
| Release | Existing DB, full Runtime reassembly | 1.1250 | 1.3907 | 1.3582 | 1.7562 | a1-optimize-release-profile |
<!-- A1_MATRIX_END -->

Release 独立子进程待机 P50 为 **12.0859 MiB**，见 `a1-optimize-release-idle.log` / `.exit`。
各组 min/mean/max、逐段统计与原目标判定均保留在日志中。Dev 与 Release 不混用，
不能以 Release 达标替代当前 Dev harness 的验收，也没有修改前 Release 数据可用于回归归因。

## 分段归因与剩余阻塞

同款 `profile-cold-start` 分段 P50（ms），前后日志为
`a1-optimize-before-profile.log` 与 `a1-optimize-final-dev-profile.log`：

| 分段或参照 | 修改前 | 最终源码 |
| --- | ---: | ---: |
| Memory 文件库首开，含完整 schema | 15.5324 | 9.7796 |
| Memory 内存库，同 schema | 11.0844 | 6.6609 |
| Memory 已有库 reopen | 1.3814 | 1.3914 |
| Session 文件库首开，与 Memory 并行 | 4.2746 | 3.6256 |
| 模块构造 | 0.1843 | 0.1911 |
| Runtime 注册、初始化和 ProviderRouter 构造合计 | 0.8058 | 0.7998 |

主要收益来自减少首建 schema 的重复解析，而非延后工作到第一回合；
已有库 reopen 仍接近修改前。文件库与内存库参照之差包含文件系统、WAL 和调度等影响，
是粗略 I/O 参照，不是系统调用隔离测量，也不能把各段 P50 直接相加作为总耗时。
两个存储并行，关键路径受较慢的一侧约束。

ProviderRouter 创建仍包含在 Runtime build 分段内，未单独计时；harness 没有 LLM provider 插件，
不测外部 provider 创建、联网、模型加载或真实服务握手，不能把这些成本写成零。

**现存性能阻塞仍是 Dev 首建 >10 ms。** 即使去掉重复 ADD COLUMN，内存库施加完整 schema
仍观测到 6.6609 ms P50，文件库首开为 9.7796 ms，之后还需模块、注册、就绪检查与调度。
继续压缩这一成本需要评估更完整的 schema 物化方案，例如带版本与完整性校验的预构建 schema 镜像，
涉及历史升级等价、跨版本兼容、文件格式与迁移清单的一致性；本批未引入这套架构。
按模块拆分必需 schema 的初始化时机同样需要先明确 ready 契约，不能直接删表或 lazy 掉必需迁移。
这些是后续设计边界，不构成“已经证明无法再作局部优化”的结论。
本批保留全部必需初始化、拒绝降低门槛或改变 Dev profile，故以 Partial 收口。

## 验证与交接

验证命令的 Dev/Test 环境均为
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`。
下列执行均 exit 0，详情和真实退出码见同前缀 `.log` / `.exit`：

| 检查 | 命令主体 | 证据前缀 |
| --- | --- | --- |
| Memory 行为与迁移回归 | cargo test -p apeireth-memory --lib --test sqlite --test integration_six_streams --test persona_layer --locked -j 2 | a1-optimize-memory-regression |
| 生产认知 | cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2 | a1-optimize-production-cognitive-v1 |
| Portable Host | cargo test -p apeireth-runtime-assembly --test portable_host --locked -j 2 | a1-optimize-portable-host |
| clippy | cargo clippy -p apeireth-memory -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-bench-harness --all-targets --locked -j 2 -- -D warnings | a1-optimize-clippy |
| 格式 | cargo fmt --check | a1-optimize-fmt |
| 架构 | python scripts/arch_guard.py --strict | a1-optimize-arch |
| 文档生成物 | python scripts/docs_truth.py --verify --check | a1-optimize-docs-truth |
| 声明逐项核验 | python scripts/docs_truth.py --verify | a1-optimize-docs-verify |

`docs_truth.py --verify --check` 在当前脚本中执行生成物检查，所以另行运行单独 `--verify`。
优化测量阶段的独立核验是 report-only exit 0，当时明确报告 `mem.lesson_object` 为 STALE，
原因是 migration 源码尚未提交；该历史日志保留原样，不能将当时的退出码写成声明全部新鲜。
提交收口阶段追加 `cargo test -p apeireth-memory --lib lesson --locked -j 2` 和
`cargo test -p apeireth-cli --lib lessons_command --locked -j 2`，均沿用上面的 Dev/Test 环境并 exit 0，
见 `a1-commit-claim-memory.log` / `.exit` 与 `a1-commit-claim-cli.log` / `.exit`。
复核确认 lesson 的治理语义、默认状态与未完成边界未变；将该 claim 的 verified_at 更新为
`1497fe7fc18ccf557e9d13aa58504b79bedd2f06`，补充首建 SQL 证据路径并生成 STATUS.md。
提交收口的独立核验与生成物检查见 `a1-commit-docs-verify.log`、`a1-commit-docs-check.log`。
测试数字真源与 README 徽章没有改动。

本轮未跑 full gate_full（留给 A4），没有硬件或外部 live E2E 实测。
`/tmp/apeireth-a1/A1-OPTIMIZE-RESULT.md` 记录最终文件清单、矩阵、剩余项和建议提交正文；
本次提交收口补齐新文档在 INDEX.csv 的索引，按源码、文档分提交；后续推送与 STATE.md 记录完整 SHA 和回执。
仓库钩子缺少可执行位，Git 未自动运行钩子；本次另以 bash 显式执行 pre-commit 与 commit-msg，
不以跳过钩子的提示冒充通过，也未改变仓库钩子配置。A1 完成后下一项为 A2，A4 再执行全量验收。
