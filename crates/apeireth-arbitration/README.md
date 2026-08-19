# apeireth-arbitration

> Apeireth R145: HASH-SQL 仲裁 + 唯一事实时间线. 跨前端/群聊/邮箱/Agent 通讯 1 套 SQL, content_hash 决定 canonical order. 上升为 Rust 编译期保证, 字段级复刻 HASH-SQL 仲裁 (开源项目 origin).

Apeireth 1.0 工作区 crate。**2 src 文件** (lib.rs / organ_kani_proofs.rs) — **13 单测 + 2 Kani proof**。

## 模块

- `src/lib.rs` — HASH-SQL ArbitrationEngine (rusqlite + sha2) + 8 测试
- `src/organ_kani_proofs.rs` — R177 arbitration organ Kani proofs (5 测试 + 2 `#[kani::proof]`)

## 文档

- 架构: [docs/01-architecture/architecture.md](../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../docs/03-reference/crates.md)
