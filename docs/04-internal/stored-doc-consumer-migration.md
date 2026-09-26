# stored_doc 消费方接线 — 迁移清单与测试数

本批把仓内 JSON 配置/记录消费方逐个甄别并接线到 `foundation/core/src/stored_doc.rs`
（存储文档信封 + 两级容错：单文档拒开 / 逐记录跳过计数 + 显式迁移留审计）。

## 基础件补充（叠加，不改既有 API）

`crates/foundation/core/src/stored_doc.rs` 新增四个入口，供消费方复用同一套语义：

| 入口 | 语义 |
| --- | --- |
| `open_single_compat` | 信封文件严格校验（不符拒开）；前信封旧裸体 body **只读兼容**（不回写、不迁移）；两者皆不可解析 → 拒开（不回退默认） |
| `scan_records_compat` | 逐记录扫描：信封记录逐条校验，旧裸体行读作无戳记录，坏行/版本不符行跳过并计数 |
| `migrate_file_from_legacy` | 显式迁移：旧裸体 → 当前信封，持久写回 + 追加 `<文件>.migrate-audit.jsonl` 审计行；已是信封则拒绝（版本迁移归 `migrate_file`） |
| `migrate_records_from_legacy` | 显式逐记录迁移：旧裸体行包信封，信封行/坏行原样保留（不丢行、不代行裁决）+ 审计留痕 |

判定信封与否只看 `name`/`version`/`body` 键是否存在：信封字段写坏的文件按信封走严格校验并拒开，
不会退化成旧裸体解析而静默取默认值。默认打开路径在任何情况下都**不静默迁移**。

## 迁移清单 — 迁（6 个消费方文件族）

| 消费方 | 文件 | 类别 | 落点 | 甄别理由 |
| --- | --- | --- | --- | --- |
| 桌面工作区配置 | `companion-config.json` | 单文档 → `open_single_compat` 拒开 | `frontend/companion-desktop/src-tauri/src/workspace.rs` | 原读取「读坏用默认」（读/解析失败即回退 app-data 兜底落位），正是静默吞损坏的典型；信封身份 + 版本戳 + 拒开补齐语义 |
| 桌面 provider 环境配置 | `backend-provider-env.json` | 单文档 | `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs` | 同上（坏配置原来回退默认 env）；现坏配置让启动构造**显式失败**，不带默认配置继续跑 |
| 桌面能力开关配置 | `backend-capability-env.json` | 单文档 | 同上 | 同上 |
| goal 快照 | `{sanitized-id}.json`（每目标一档） | 单文档 | `crates/engine/organ/src/goal.rs`（`GoalStore`） | 整档快照 JSON、读写对称，适合信封拒开 + 显式迁移；load 的缺文件 = None 语义保留，坏档不再可能被当空态吞掉 |
| 学习日志 | `tuning-log.jsonl` | 逐记录 → `scan_records_compat` 跳过计数 | 写端 `crates/engine/runtime-assembly/src/canonical/self_tuning_wire.rs`；读端 `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs` | 唯一的 jsonl 记录消费面；原读端静默丢坏行且不计数；现每行一个逐记录信封（版本戳随行、逐行裁决），坏行读作不存在并计数，旧裸体行只读兼容 |
| （读端展示面） | 同上 | 同上 | `read_tuning_log_file` 返回 `TuningLogRead { entries, skipped }`；UI 契约仍为记录列表，跳过计数落桌面日志 | 好行照读零回归 + 坏行可观测 |

显式迁移入口（均留 `<文件>.migrate-audit.jsonl` 审计行，失败不落盘）：

- `workspace::migrate_legacy_workspace_config`
- `BackendSupervisor::migrate_legacy_provider_env_config` / `migrate_legacy_capability_env_config`
- `GoalStore::migrate_legacy_snapshot`
- `migrate_legacy_tuning_log_file`（runtime-assembly 导出，逐记录迁移）

各消费方读取路径均不迁移旧文件；迁移由调用方显式触发。

## 迁移清单 — 不迁（逐个理由）

| 消费方 | 文件 | 不迁理由 |
| --- | --- | --- |
| reflexion 状态 | `reflexions.json`（`crates/engine/memory/src/reflexion.rs`） | 运行态存储而非配置/记录面：带严格内部不变量（游标≤总数、失败序列严格递增、next_seq 归一）+ 历史裁剪 + 比信封通用 Malformed 更细的类型化损坏错误；且本批件二同文件正换其变更锁——格式迁移与锁替换不叠加在同一消费方，保回归面隔离。schema 真实演进时走 `migrate_file` 显式版本迁移 |
| 续行快照 | 每快照一档（`crates/foundation/orchestration/src/continuation.rs`） | 自定义协议：一次性消费（读取即删）+ 文件系统原子认领（claim 文件 fail-closed，含崩溃遗留声明）+ 自有损坏语义（CorruptSnapshot / AlreadyConsumed）；「快照即本体、消费即销毁」与版本化信封 + 迁移回写语义冲突 |
| 工作量折叠 | `crates/foundation/orchestration/src/work_consumption.rs` | 无落盘面：纯内存 WorkLog，不存在 JSON 文件消费方可迁 |
| 调度状态 | `schedule.json`（`durable_schedule.rs`） | 整档状态机快照：写序号 + CAS（冲突码直连线协议）+ 共享文件锁 + FIFO 交付队列；读档损坏已有 fail-closed 信码（`schedule_corrupt`）。判自定义持久化面，本批不动（后续批次可走 `migrate_file`） |
| 收件队列 | `inbox.json`（`durable_inbox.rs`） | 同上：整档状态机快照 + 认领/不重投递语义，损坏已有 `inbox_corrupt` 信码 |
| 记忆后端记录 | `episodes.jsonl` + `streams/*.jsonl`（`crates/engine/memory/src/backend/file.rs`） | 自定义追加式记录格式：幂等去重（重复 id 拒绝）、墓碑、序号流；行语义是存储后端契约，不迁 |
| 凭据存储 | `creds.json`（`crates/foundation/credentials`） | 敏感字段 + keyring 联动 + 自有审计与权限档位；格式受安全契约约束，不迁 |
| 评估数据集 | `guard.jsonl`（`crates/engine/guard/src/dataset.rs`） | 多态记录导出格式（record_type 族），消费面是导出/评估工具，不迁 |
| 观测日志 | `traces.jsonl` / `daemon-audit.jsonl` / `memory-flags.jsonl`（`crates/adapters/cli/src/gateway_panels.rs`） | 多写者追加的观测日志（守护进程与 CLI 跨版本共同追加）；面板读取是尽力展示面（坏行容忍即其契约），跨版本写者兼容优先，不迁 |
| 簇存储 | `meta_thinking_chains.json` + 簇目录（`crates/engine/memory/src/cluster_store.rs`） | 自定义目录式簇存储（每簇一目录 + chains 索引文件），索引文件不是独立单文档面，不迁 |
| 沙箱配额快照 | `quota.json`（`crates/adapters/sdk/src/sandbox/quota.rs`） | 崩溃对账用纯写出快照（每次变更整档重写），无配置读取消费方；拒开语义不适用，不迁 |
| 令牌缓存 | `tenant-token.json`（`crates/adapters/sdk/src/lark/auth.rs`） | 可再生缓存（坏即重取），拒开/版本迁移无收益，不迁 |
| 会话/认知库 | SQLite（`sqlite_session`、memory sqlite backend 等） | SQLite，非 JSON 文档面 |

## 测试数

件一新增 **19** 条（覆盖要求的五类行为）：

- 基础件兼容层（`stored_doc.rs`）7：兼容打开不改写旧文件（好数据逐字节等价）、信封形状坏档拒开不回退默认、兼容打开仍执行身份/版本契约、旧裸体显式迁移留审计、迁移守卫（拒信封 / 失败不落盘）、逐记录兼容扫描（坏行跳过计数 + 旧裸体照读）、逐记录显式迁移（信封行/坏行原样保留 + 审计）。
- goal 快照（`goal.rs`）3：好数据 body 逐字节等价、旧裸体迁移前后都可读（默认打开不静默迁移 + 审计）、坏档/串档拒开不回退默认。
- 学习日志写端（`self_tuning_wire.rs`）2：坏行跳过计数 + 旧裸体行照读（逐字节等价）、显式逐记录迁移留审计。
- 桌面工作区配置（`workspace.rs`）3：坏配置拒开不回退默认、好数据逐字节等价、旧文件迁移前后都可读 + 审计。
- 桌面环境配置与学习日志读端（`backend_supervisor.rs`）4：坏 provider 配置拒开（含启动构造失败）、好数据逐字节等价、provider/capability 旧文件迁移前后都可读 + 审计、信封行与旧裸体行共存照读（另有 1 条旧测试补上坏行跳过计数断言、1 条旧测试改信封断言）。

件二新增 **3** 条（`reflexion.rs`）：死锁接管（持有者 PID 已不存在的残留锁可接管、不留残）、活跃持有者不被抢（有限等待后 StoreBusy 且锁保持原样）、变更完成后共享锁归还（外部持有者立即可取）。

合计新增 **22** 条测试；既有测试零删改断言语义（仅随签名/落盘信封同步调整断言口径）。

## 校验

- `cargo test --workspace --all-targets --locked`：绿（cargo exit 0，137 个测试二进制全部 `test result: ok`，0 FAILED；完整日志 `artifacts/piece1-2-workspace-test.log`）。
- reflexion 并发双写测试 `file_store_concurrent_instances_preserve_both_writes_and_unique_sequences` 改造后连跑 5 次 ×2 轮全过（0.01–0.03s/次）。
- `cargo clippy --workspace --all-targets --locked`：0 warning / 0 error（日志 `artifacts/piece1-2-clippy.log`）。
- 桌面 workspace（frontend/companion-desktop/src-tauri）自带测试：52 单测 + 8 集成全过。
- fmt：本批触碰的文件 0 漂移（逐文件 rustfmt 校验通过）；`cargo fmt --all --check` 余下漂移均在并行批次在改的文件（gateway `barge_in.rs`/`events.rs`/`presence.rs`、`tests/events.rs`），按「叠加不覆盖」未触碰。
- `scripts/check-neutral-terms.ps1`：40 个变更文件零命中。
