# 贡献指南 (Apeireth v2 工程线)

> Apeireth 是我们的伙伴型 AGI 操作系统。贡献前请先读哲学——[docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md)（8 锚 / 三洋葱 / 0 装 PASS）。

## 必读

- [docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md) — 哲学（8 锚 / 三洋葱 / 0 装 PASS）
- [docs/01-architecture/architecture.md](docs/01-architecture/architecture.md) — 架构总览（13 crates 分组）
- [docs/01-architecture/security.md](docs/01-architecture/security.md) — 安全模型
- [docs/03-reference/crates.md](docs/03-reference/crates.md) — crates 索引
- [docs/04-internal/design-intent.md](docs/04-internal/design-intent.md) — 设计意图与我们拍板历史
- [ROADMAP.md](ROADMAP.md) — 路线图（v2 下一步按优先级排列；旧台账已冻结见 docs/04-internal/backlog.md）

## 提交前必跑

```bash
cargo check --workspace --all-targets   # 编译全 target 干净（含 examples/bins/tests）
cargo test --workspace                  # 全量测试 0 失败（当前 13-crate 工作区 1338）
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

- 主分支 `main`（默认分支；旧 master 归档为 `archive/v1.0-master`）
- 开发在独立分支 → 全量验证 → PR 合入 main
- 禁止直接 force-push 共享分支（历史净化等特殊操作除外，需我们确认）

## 贡献声明 (等效于 DCO)

向本仓库提交任何代码、文档或素材,即表示你确认:

1. 该贡献的全部或部分由你本人创作,你有权按本项目双协议 (Apache-2.0 OR MIT) 提交;
2. 若贡献基于他人作品,你已获得符合本项目协议的许可,并在提交中注明来源与改动;
3. 你理解贡献将按本项目双协议公开分发,该授权不可撤回;
4. 若贡献来自雇主或委托方,你已取得其同意。

提交信息尾部建议附带签名行 (等效 DCO `Signed-off-by`):

    Signed-off-by: 你的姓名 <you@example.com>

未带签名行的提交仍可合并,合并行为即视为对上述声明作出确认。

## License

双协议: [Apache-2.0](LICENSE) 或 [MIT](LICENSE-MIT),任选其一。
