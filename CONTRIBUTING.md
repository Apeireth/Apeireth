# 贡献指南 (Apeireth 1.0)

> Apeireth 是主人的伙伴型 AGI 操作系统。贡献前请先读哲学——[docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md)（6 锚 / 双洋葱 / 0 装 PASS）。

## 必读

- [docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md) — 哲学（6 锚 / 双洋葱 / 0 装 PASS）
- [docs/01-architecture/architecture.md](docs/01-architecture/architecture.md) — 架构总览（85 crates 分组）
- [docs/01-architecture/security.md](docs/01-architecture/security.md) — 安全模型
- [docs/03-reference/crates.md](docs/03-reference/crates.md) — crates 索引
- [docs/04-internal/design-intent.md](docs/04-internal/design-intent.md) — 设计意图与主人拍板历史
- [docs/04-internal/backlog.md](docs/04-internal/backlog.md) — 台账（完成项 ✅ / 待办 ⬜）

## 提交前必跑

```bash
cargo check --workspace --all-targets   # 编译全 target 干净（含 examples/bins/tests）
cargo test --workspace                  # 全量 368 组 0 失败
cargo fmt --all --check                 # 格式
```

## 0 装 PASS 纪律（最重要）

- 未实现 = 标注 `trait 口已备未接`，绝不静默
- 真网络测试 = 带限流退避，不因 API 限流自造失败
- 无环境实测 = 标"待实测"，不写"完成"
- 改公共结构（struct/enum/签名）→ `grep` 所有构造点 + all-targets 编译

## 文档同步自觉

- 改码必改对应 README/docs（规范 00）
- 新调研未落地 → 登记台账，不散落聊天记录
- 文档结构与实际对齐，历史文档进 `docs/archive/`

## 工作流

- 主分支 `master`（发布线）↔ `team/e8de47ae-.../integration`（集成线）
- 开发在独立分支 → 全量验证 → 合入 integration → 发布时同步 master
- 禁止直接 force-push 共享分支（历史净化等特殊操作除外，需主人确认）

## License

Apache-2.0 — 见 [LICENSE](LICENSE)。
