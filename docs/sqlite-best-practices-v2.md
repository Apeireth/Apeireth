# SQLite Best Practices V2（rusqlite 0.32 / Apeireth）

本文从 V2 `apeireth-memory`、`apeireth-vector`、`apeireth-api`、`apeireth-mcp` 的 rusqlite 版本冲突与 memory×vector 集成经验提炼。目标不是列一份“越多 PRAGMA 越快”的清单，而是在 **不破坏旧数据库、不制造 `libsqlite3-sys links` 冲突、可恢复** 的前提下给出默认做法。

适用基线：workspace 将 `rusqlite` 锁为 `0.32`，当前采用 `bundled`。所有使用 SQLite 的 crate 应继承 workspace 依赖，不自行选择 minor 版本或底层 SQLite 来源。

## 1. 只允许一个 workspace 级 rusqlite / libsqlite3-sys 决策

Rust/Cargo 对带 `links = "sqlite3"` 的原生库要求依赖图中只有一个链接提供者。两个 crate 分别钉不同 rusqlite minor，常会间接解析出不兼容的 `libsqlite3-sys`，即使 Rust API 没有交叉调用，workspace 仍会在依赖解析或链接阶段失败。

根 manifest 统一：

```toml
[workspace.dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
```

成员 crate 只写：

```toml
[dependencies]
rusqlite = { workspace = true }
```

禁止写 `rusqlite = "0.31"`、重复设置 `bundled`、或由某个 leaf crate 私自打开 `load_extension`。升级时一次升级 workspace lock，并至少执行：

```bash
cargo tree -i libsqlite3-sys
cargo check --workspace
```

应只看到一个 `libsqlite3-sys` 版本。若第三方库强制旧版本，先升级或 feature-gate 它，不要用 patch 堆出两个 sqlite3 链接者。

## 2. bundled 与 system feature 必须是部署级选择，不是 crate 级偏好

### 默认选择 bundled 的场景

* 桌面应用、CLI、离线 agent，需要单一可复现二进制；
* Windows/macOS/Linux CI 要一致 SQLite 能力；
* 需要确保运行时版本支持预期 PRAGMA/SQL；
* 宿主机未承诺 sqlite development headers 与 ABI。

优点是构建可复现、避免系统 SQLite 过旧；代价是编译时间、二进制体积、安全更新需随应用重新发布。

### 选择 system SQLite 的场景

* Linux 发行版/容器有明确的系统补丁与 ABI 管理；
* 安全团队要求 SQLite CVE 由系统包统一修复；
* 必须共享宿主提供的 SQLite 扩展；
* 构建环境可以验证 header 与 runtime library 一致。

**不要在同一 workspace 混搭。** system/bundled 属于最终产物策略，应通过顶层 feature 或不同构建 profile 决定，并在 CI 分别验证。运行时可记录 `SELECT sqlite_version()` 与 compile options，便于事故定位。切换来源不会自动迁移数据库文件，但不同 SQLite 能力可能影响新 DDL；迁移必须按最低受支持版本设计。

## 3. 连接生命周期：短事务，明确所有权，不跨 await 持锁

`rusqlite::Connection` 不是“随处 Clone 的异步连接”。Apeireth 的 memory store 通过受控连接访问，vector backend 持有独立连接；这比把裸连接暴露给各模块更容易维持事务边界。

建议：

* 一个请求借连接、开启短事务、提交后立即归还；
* 不要在持有 `MutexGuard<Connection>` 或 transaction 时 `.await` 网络/模型调用；
* embedding 先在事务外计算，得到结果后再开写事务；
* memory 与 vector 当前是两个 DB 文件，不能假装具备跨库原子提交；采用可重试的索引作业与幂等 upsert；
* 单进程低并发先用一个受控连接，确认锁竞争后再引入 pool，避免为 skeleton 增加依赖。

需要 pool 时，pool lifetime 应与服务进程一致，而不是每请求创建。连接建立钩子必须为 **每条连接** 设置 connection-local PRAGMA（例如 `foreign_keys`、`busy_timeout`）；不能只在 migration 连接执行一次。pool 最大连接数由并发读需求决定，SQLite 仍只有一个 writer，盲目增大 pool 会增加 `SQLITE_BUSY`，不会增加写吞吐。

## 4. WAL 是读写并发默认项，但要管理 checkpoint

文件数据库推荐在打开阶段显式请求：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
```

WAL 让 reader 通常不阻塞 writer，适合记忆读取与 episode 追加并行。`journal_mode` 是持久数据库属性，请读取返回值确认实际得到 `wal`；只执行、不检查会掩盖只读文件系统或不支持环境。`:memory:` 没有同样的 WAL 文件语义，不能用内存测试证明生产 checkpoint 正常。

注意事项：

* WAL 会产生 `-wal` 与 `-shm`，备份/复制时不能只拷贝主 `.db`；
* 长 reader 会阻止 checkpoint 回收，导致 WAL 持续增长；
* 不要每个请求 `wal_checkpoint(TRUNCATE)`，这会制造抖动；
* 监控 WAL bytes、checkpoint duration、busy 次数和最老读事务；
* 正常退出或运维窗口可做受控 checkpoint；异常退出依赖 SQLite 恢复，而不是手删 WAL。

网络共享文件系统、某些容器 volume 或多主机同时打开同一 DB 时，不应默认 WAL 安全。SQLite 文件应由单主机/单实例协调；需要跨主机 HA 就使用服务型数据库。

## 5. busy_timeout 是瞬时竞争缓冲，不是锁问题修复器

每条连接设置有限等待：

```rust
use std::time::Duration;
connection.busy_timeout(Duration::from_secs(5))?;
```

或者等价 PRAGMA，但优先用 rusqlite API，类型更清晰。建议交互请求从 1–5 秒起步，后台迁移可按维护窗口单独配置。timeout 太短会在正常 checkpoint/短写事务时产生偶发失败；太长会让请求卡住并耗尽线程池。

应用仍必须：

1. 缩短写事务，事务内不做 embedding、HTTP、文件 IO；
2. 对明确可重试且幂等的操作做有上限的指数退避和 jitter；
3. 区分 `BUSY/LOCKED` 与约束错误，不能重试所有 SQLite 错误；
4. 记录等待时间与尝试次数，超过阈值报警。

`busy_timeout=30s` 不能修复一个跨 await 持锁的事务。先修生命周期，再调 timeout。

## 6. migration 必须单调、幂等、可审计，并保护旧数据

只用 `CREATE TABLE IF NOT EXISTS` 不能完整表达 schema 演进；它不会为旧表补列、补约束，也不会证明列类型正确。推荐维护版本表或 `PRAGMA user_version`，每个版本在一个显式事务中前进：

```text
BEGIN IMMEDIATE;
  verify current version;
  apply exactly one forward migration;
  validate invariants;
  record new version;
COMMIT;
```

安全规则：

* migration 开始前拒绝未知的未来版本，避免旧二进制写坏新 schema；
* `ADD COLUMN` 先 nullable 或给兼容 default，后台 backfill 后再由新代码使用；
* rename/drop/rebuild 使用“新表→复制→校验 count/约束→交换表名”，不要直接丢旧表；
* 大 backfill 分批提交并记录游标，避免单个巨大 WAL；
* DDL 与版本号必须同事务提交；失败回滚后可重跑；
* 启动时 migration 由单一 leader 执行，其他实例等待 schema ready；
* 上线顺序遵守 expand/contract：先让旧/新代码都能读写，再移除旧列；SQLite 没有必要为了整洁立即 drop 列。

向量维度、embedding model/revision 不只是应用配置，也属于索引 schema。更换模型不要覆盖历史 BLOB：建立新 generation，双写或后台重建，切读指针后再延迟清理旧 generation。

## 7. PRAGMA 调优要按连接/数据库作用域分类，并验证返回值

推荐起始配置（文件库）：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -32768;       -- 约 32 MiB，需按进程连接数预算
PRAGMA mmap_size = 268435456;     -- 仅在平台验证后使用
PRAGMA wal_autocheckpoint = 1000;
```

这不是可盲抄的“最快配置”：

* `foreign_keys`、`busy_timeout` 等是 connection-local，每条 pool connection 都要设置；
* `journal_mode` 会影响数据库文件并返回实际模式；
* `cache_size` 为负数时近似 KiB，且每连接 cache 会累加内存；
* `mmap_size` 受平台、地址空间和 SQLite build 影响，收益需测；
* `synchronous=OFF` 可能在断电/OS crash 时丢数据，不用于记忆主库；
* `locking_mode=EXCLUSIVE` 会破坏正常多连接使用，不作为默认优化；
* `read_uncommitted` 不等于通用性能开关，不应牺牲一致性。

初始化后查询关键 PRAGMA 并写诊断日志。每次只改变一个参数，用文件 DB、真实页缓存和恢复测试验证；微基准快不代表 crash-safe。

## 8. 外键约束必须真正启用，并为访问路径建索引

SQLite 默认/连接行为可能导致外键未启用，因此每条连接都执行并验证：

```sql
PRAGMA foreign_keys = ON;
PRAGMA foreign_keys;
```

migration 结束执行：

```sql
PRAGMA foreign_key_check;
PRAGMA integrity_check;
```

外键列通常需要显式索引，否则删除/更新 parent 时会扫描 child 表。删除策略要写清：历史 episode 通常适合 RESTRICT/append-only，而派生向量索引可允许重建，不要随意 `ON DELETE CASCADE` 抹掉审计数据。

当前 memory 与 vector 分库，SQLite 无法跨文件声明可靠外键。`metadata.episode_id` 是软引用，所以搜索遇到 ghost vector 时允许跳过；同时应有定期 reconciliation：找出缺失 episode 的向量、缺失向量的 episode，按 generation 幂等修复。不要把 JSON metadata 当成数据库强约束。

## 9. VACUUM、ANALYZE 与空间回收按指标触发

`VACUUM` 重写整个数据库，需要额外磁盘空间并可能长时间占锁，不应在每次启动或每次删除后运行。实践建议：

* append-only memory 主库通常不需要频繁 VACUUM；
* 大批删除/索引 generation 切换后，在维护窗口评估 `freelist_count/page_count`；
* 需要渐进回收时，在建库前决定 `auto_vacuum=INCREMENTAL`，因为改变模式本身可能需要 VACUUM；
* 使用 `PRAGMA incremental_vacuum(N)` 小步回收并观察延迟；
* 数据分布大变后运行 `PRAGMA optimize` / 必要的 `ANALYZE`，而不是固定高频全库分析；
* VACUUM 前确认可用空间、备份和最长维护时间。

`VACUUM INTO` 可用于生成一致的紧凑副本，但仍需验证目标文件、权限、可用空间与恢复流程。空间变小不是首要 SLO，数据安全和尾延迟优先。

## 10. 索引、查询与备份必须用计划和恢复演练闭环

B-tree 索引服务于 SQL filter/order/join，不会自动加速 vector cosine。为真实访问模式建立最少索引：例如 memory 已有 `(session_id, timestamp)`，可以同时支持 session 过滤和时间排序；不要再创建反序 `(timestamp, session_id)`，除非 `EXPLAIN QUERY PLAN` 和生产查询证明需要。

规则：

* 参数绑定，不拼接用户输入；动态 LIMIT 也应先做数值边界校验；
* 用 `EXPLAIN QUERY PLAN` 确认关键查询走预期索引；
* 每个索引增加写放大和文件大小，重复/前缀冗余索引应删除；
* 向量 BLOB 只应读取需要的候选；在 brute-force backend 中，`idx_vec_items_dim` 只过滤维度，不能改变同维度全扫；
* top-k 用有界堆、metadata 延迟解析比添加无关 SQL 索引更有效。

备份必须使用 SQLite backup API、`VACUUM INTO` 或停写/checkpoint 后的一致复制策略；WAL 活跃时只复制 `.db` 不可靠。每份备份记录 schema version、SQLite version、校验和与加密密钥版本。季度至少做一次“在空目录恢复→跑 migration→`integrity_check`→抽样业务查询”，因为没有恢复演练的备份只是未经验证的文件。

## 推荐打开模板

以下顺序保留错误上下文，并把连接局部设置集中在一个入口：

```rust
use std::path::Path;
use std::time::Duration;
use rusqlite::Connection;

fn open_database(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA temp_store=MEMORY;",
    )?;
    // run_versioned_migrations(&mut connection)?;
    Ok(connection)
}
```

生产实现还应读取 `journal_mode`/`foreign_keys` 验证结果，并在 migration 需要 transaction 时接收 `&mut Connection`。不要用 `unwrap` 打开主数据文件；路径、权限、disk full、corruption 都必须上抛并阻止服务带着半初始化 store 继续运行。

## V2 上线检查清单

- [ ] `cargo tree -i libsqlite3-sys` 只有一个版本/链接来源。
- [ ] 所有成员 crate 使用 `rusqlite = { workspace = true }`。
- [ ] bundled/system 由最终部署选择，CI 记录 `sqlite_version()`。
- [ ] 每条连接启用 foreign keys 与有限 busy timeout。
- [ ] 事务内没有网络、模型推理或跨 await 锁持有。
- [ ] WAL 文件大小、checkpoint 和 BUSY 次数有监控。
- [ ] migration 有版本、事务、失败回滚、未来版本拒绝和旧库 fixture 测试。
- [ ] 向量模型升级使用 generation，不原地破坏旧 BLOB。
- [ ] `foreign_key_check`、`integrity_check` 与关键 query plan 通过。
- [ ] VACUUM 仅按指标在维护窗口执行。
- [ ] 备份包含活跃 WAL 的一致视图，并已真实恢复演练。

这些默认值的“ceiling”是单机、本地、一个 writer 的 SQLite。出现跨主机写、持续高写并发、100K+ 高维 ANN 或严格 HA 时，升级路径是替换 `VectorStore`/storage backend，而不是继续堆连接池和 PRAGMA。
