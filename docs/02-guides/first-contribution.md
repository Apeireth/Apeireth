# 新人上手指南 (First Contribution)

> 这是你的第一个 PR 的完整动线：从"想参与"到"被合并"，预计 15 分钟读完。不需要读完全部文档才动手——**做中学，问中会**。

## 第 0 步：知道 Apeireth 是什么（5 分钟）

- 一句话：伙伴型 AGI 操作系统 & 认知微内核（Pure Safe Rust，`unsafe` 全禁）；
- 读 [README](../../README.md) 的 The Story 和 [philosophy.md](../01-architecture/philosophy.md) 的摘要——不了解"0 装 PASS"（未验证不装）哲学，你的 PR 会很难通过 review；
- 其余文档不用现在读，需要时再查。

## 第 1 步：搭好环境（10 分钟）

```bash
# 需要: Rust 工具链（版本以 rust-toolchain.toml 锁定为准）+ Git
git clone https://github.com/<你的用户名>/apeireth-rust.git
cd apeireth-rust

# 提交前必跑三连（我们的质量门禁，CI 也跑同样的）:
cargo check --workspace --all-targets   # 编译干净
cargo test --workspace                  # 全量测试 0 失败
cargo fmt --all --check                 # 格式
```

Windows 用户可参考 [INSTALL.md](../../INSTALL.md) 与 [development.md](development.md) 的详细说明。

## 第 2 步：找到你的第一个任务（5 分钟）

- 在 [Issues](https://github.com/Apeireth/Apeireth/issues) 里筛标签 **`good-first-issue`**——这些任务已被维护者标注"适合新人、范围清晰、有上手指引"；
- 没有合适的？三个替代入口：
  1. 修文档错漏（改文档同样是正经贡献，`docs/` 的 typo、过时命令都可以）;
  2. 报 Bug——按 [bug 模板](https://github.com/Apeireth/Apeireth/issues/new?template=bug_report.yml) 带复现步骤提交，报得清楚本身就是贡献；
  3. 在 [Discussions](https://github.com/Apeireth/Apeireth/discussions) 提出你的想法，先聊聊再动手。

**认领**：在想做的 Issue 下留言"认领"，获得确认后开工（`good-first-issue` 可直接开干），避免和别人撞车。

## 第 3 步：开发与提交（视任务而定）

```bash
git checkout -b feat/短描述        # 分支命名: feat/ fix/ docs/ chore/
# ... 开发 ...
# 提交前必跑"三连"（见第 1 步）
git commit -m "fix: 一句话说清改了什么" \
           -m "为什么这么改（简述）" \
           -m "Signed-off-by: 你的姓名 <you@example.com>"
```

- **提交信息**说清"改了什么 + 为什么"；`Signed-off-by` 是等效 DCO 的身份声明（[贡献声明](../../CONTRIBUTING.md#贡献声明-等效于-dco)，未带也可合并，但强烈建议带上）；
- **改公共结构**（struct/enum/签名）→ `grep` 所有构造点 + all-targets 编译（这是 review 必查项）；
- **没有实测过的结论**标"待实测"，没实现的接口标 `trait 口已备未接`——**绝不静默**。装了会被 review 识破，如实标注反而加分。

## 第 4 步：开 PR

- 用 [PR 模板](https://github.com/Apeireth/Apeireth/blob/main/.github/PULL_REQUEST_TEMPLATE.md)，说清动机、改动、验证方式；
- 一个 PR 只做一件事；改动越大，越需要先开 Issue 或 [RFC](../development/rfc-process.md) 对齐方案；
- PR 描述里链接对应 Issue（如 `Closes #123`）。

## 第 5 步：之后会发生什么（我们的承诺）

| 阶段 | 时限 | 详见 |
|---|---|---|
| 首次响应 | ≤ 3 个工作日 | [CONTRIBUTING.md](../../CONTRIBUTING.md) PR Review 规范 |
| Review | 对事不对人，必给理由 | [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) |
| 超时升级 | 5 个工作日无响应可 @chuling | 不允许无声黑洞 |
| 合并后 | 你会自动成为 **Contributor** | [GOVERNANCE.md](../../GOVERNANCE.md) 角色阶梯 |

被要求修改很正常——大神的 PR 也常被要求改几轮。看 review 意见里的"为什么"，那是免费的技术课。

## FAQ

**Q: 我的 Rust 水平一般，能参与吗？**
能。`good-first-issue` 就是为你准备的；文档、测试、示例代码都是入口。不懂就问，提问不丢人。

**Q: 可以用中文交流吗？**
可以。仓库文档中英双语（[README](../../README.md) / [README.zh-CN](../../README.zh-CN.md)），Issue/PR 用中文或英文都行。

**Q: 我想做的功能比较大，怎么办？**
先走 [RFC 流程](../development/rfc-process.md) 对齐设计再动手，避免做完才发现方向不合。小改动直接 PR 即可。

**Q: 会不会我干了半天白干？**
认领制 + 首次响应承诺就是为了避免这一点。只要按流程认领了，维护者就有义务跟进到底。
