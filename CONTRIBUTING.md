# 贡献指南 (Apeireth v2 工程线)

> Apeireth 是我们的伙伴型 AGI 操作系统。贡献前请先读哲学——[docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md)（9 锚 / 三洋葱 / 0 装 PASS）。

**欢迎！** 无论你是修一个错别字、报一个 Bug，还是推进一个大特性，都是这个项目的建设者。第一次贡献请走 [新人上手指南](docs/02-guides/first-contribution.md)（15 分钟读完，从 good-first-issue 到第一个 PR 合并的完整动线）。

参与本社区即表示你同意遵守 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)（行为准则）；角色、决策与争议解决见 [GOVERNANCE.md](GOVERNANCE.md)（治理章程）。

## 必读

- [docs/01-architecture/philosophy.md](docs/01-architecture/philosophy.md) — 哲学（9 锚 / 三洋葱 / 0 装 PASS）
- [docs/01-architecture/architecture.md](docs/01-architecture/architecture.md) — 架构总览（18 crates 分组）
- [docs/01-architecture/security.md](docs/01-architecture/security.md) — 安全模型
- [docs/03-reference/crates.md](docs/03-reference/crates.md) — crates 索引
- [docs/04-internal/design-intent.md](docs/04-internal/design-intent.md) — 设计意图与我们拍板历史
- [ROADMAP.md](ROADMAP.md) — 路线图（v2 下一步按优先级排列；旧台账已冻结见 docs/04-internal/backlog.md）

**动验证面 / 基建必读**（踩坑实录, 不读必踩）:
- [docs/04-internal/kani-verification-playbook.md](docs/04-internal/kani-verification-playbook.md) — Kani 形式化验证实战手册（14 坑 + 配方检查单 + 本地环境）
- [docs/04-internal/ops-network-playbook.md](docs/04-internal/ops-network-playbook.md) — 运维与网络排障（代理/下载/WSL/shell 陷阱）

## 提交前必跑

```bash
cargo check --workspace --all-targets   # 编译全 target 干净（含 examples/bins/tests）
cargo test --workspace                  # 全量测试 0 失败（当前 18-crate 工作区）
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

## Issue 分诊规则（我们对你的承诺）

| 类型 | 标签 | 首次响应（工作日） |
|---|---|---|
| Bug 报告 | `bug` | ≤ 3 天内分诊（确认/需复现/非问题） |
| 功能请求 | `enhancement` | ≤ 7 天内给出接受/暂缓/拒绝 + 理由 |
| 新人任务 | `good-first-issue` | 有人认领后 ≤ 3 天内给出上手指引 |
| 1.0 blocker | `1.0-blocker` | ≤ 1 天内分诊 |

- **分诊必有结论**：不接受"无人回应"超过上述时限；若我们暂时没空，也会先回一句"已看到，排队中"。
- 认领制：在 Issue 下留言"认领"并获确认后开干，避免撞车；`good-first-issue` 可直接认领。
- 标签体系与运营纪律详见 [docs/development/community-ops.md](docs/development/community-ops.md)。

## PR Review 规范（也是承诺）

- **首次响应** ≤ 3 个工作日（工作日口径见 [docs/development/community-ops.md](docs/development/community-ops.md)）；
- Review 对事不对人，意见必须给出"为什么"或替代方案（见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)）；
- 合并门槛：CI 全绿（`cargo check --workspace --all-targets` + `cargo test --workspace` + `cargo fmt --all --check`）+ 至少 1 名 Reviewer/Maintainer 批准；**重大变更**按 [GOVERNANCE.md](GOVERNANCE.md) 走 RFC；
- 超时升级：PR 首次响应超时 5 个工作日仍未获 review，可在 PR 里 @chuling 升级——**不允许无声黑洞**。

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
