# 自检-DB2: 测试数据与迁移就绪度

任务ID: c7e494b3-63cd-4fbf-9736-8ae7d244e6fa · 角色: database_engineer2 · 日期: 2026-08-16

## 总结论: ✅ PASS（附 1 项观察，不阻塞）

## 1. 迁移就绪度 ✅
- 核心迁移模块: `crates/apeireth-memory/src/migrations.rs`（版本化、append-only、直接 SQL 无 ORM）
- 追踪机制: `schema_migrations` 表记录 version/name/applied_at，逐条幂等跳过已应用
- 当前版本: V1__init_six_history_streams, V2__hallways_for_wings（命名规范 `V<n>__desc`）
- 内建单测: 迁移幂等性 + append-only trigger（拒 UPDATE/硬删、允许软删 tombstone）
- 其他 CREATE TABLE 内嵌点: continuity_link.rs / dailynote/store.rs / lightmemo/l1_file.rs（均为 IF NOT EXISTS 防御式）
- 无独立 migrations/ 目录与 .sql 文件——迁移全部 Rust 内嵌，属有意设计（见模块头注释）

## 2. 测试数据/夹具就绪度 ✅
- workspace 测试文件: tests/ 下 11 个集成测试 + crates 内 177 个 tests/*.rs + 964 个文件含 #[test]
- apeireth-companion: 约 169 个 #[test]（tests/exec_worker_isolation.rs + src 内单测）
- apeireth-test 辅助 crate 存在（lib.rs / property_tests.rs / organ_kani_proofs.rs）
- 数据隔离良好: 测试用 `Connection::open_in_memory()` + tempfile/TempDir，无共享状态文件
- git 未跟踪任何活跃 .db/.sqlite/.sql 夹具；仅 _archived/_frozen 下有历史 JSON 夹具（不参与构建）
- .gitignore 覆盖 *.db / *.db-shm / *.db-wal

## 3. 测试可枚举性（动态验证） ✅
- 全量命令 `cargo test -p apeireth-companion -- --list` 因 examples 链接失败: LNK1104 无法写入 `companion_serve.exe`——运行中的 daemon 进程锁定了该 exe（环境问题，非测试/数据缺陷）
- 改用 `cargo test -p apeireth-companion --lib --tests -- --list`（跳过 examples/benches）: **成功枚举 224 lib tests + 3 integration tests（exec_worker_isolation），0 benchmarks**
- 编译在 5 分钟预算内完成（大部分依赖命中增量缓存）

## 4. 观察项（⚠️ 非阻塞，共 2 项）
1. ⚠️ 运行时 DB 残留: `crates/apeireth-memory.db*`（含 486KB WAL），已被 gitignore；建议本地清理避免误判为夹具。
2. ⚠️ `companion_serve.exe` 被占用导致全量 `cargo test --list` 失败；CI/自检脚本建议固定加 `--lib --tests` 或先停 daemon。

## 结论
✅ **PASS** — 迁移体系（append-only 版本化 + 幂等单测）与测试数据隔离（内存 DB/tempdir，无 git 跟踪夹具）均就绪；companion 测试可枚举（227 个）。DB 层无阻塞问题。
