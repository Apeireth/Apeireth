# 如何贡献 Apeireth (Contribution Guide)

> **性质**: 贡献代码 / 文档 / 测试 / issue 的完整流程
> **依据**: `CONTRIBUTING.md` (根目录) + Apache-2.0 §5 + `CODEOWNERS`
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-5)

---

## 0. TL;DR

| 步骤 | 干什么 | 工具 |
|------|--------|------|
| 1. **issue 先行** | 提 issue 描述 bug / feature / 文档改进 | GitHub issue tracker |
| 2. **fork 仓库** | 派生自己的 GitHub 仓库 | GitHub UI |
| 3. **clone + 创分支** | `git clone ...` + `git checkout -b fix/xxx` | git |
| 4. **改 + 测** | 写代码 / 文档 + 跑 `cargo test --workspace` | cargo |
| 5. **commit + push** | `git commit -m "..."` + `git push origin` | git |
| 6. **Pull Request** | 在 GitHub 提 PR, 引用 issue | GitHub UI |
| 7. **CI 跑过** | CI 12 workflow 全过, 5 reviewer 审 | GitHub Actions |
| 8. **Squash merge** | maintainer squash merge 到 main | GitHub UI |

---

## 1. 贡献者许可协议 (CLA)

### 1.1 是否需要签 CLA?

**❌ 不需要签 CLA** (per 主人 2026-08-04 拍板 "不引入 CLA 流程, 保持 Apache-2.0 简单")

- Apache-2.0 §5 **已自动授予** 项目使用你贡献的代码的权利
- 你保留 **版权** (你的代码 = 你的版权)
- 你授予项目 **Apache-2.0 许可权** (可商用 / 修改 / 再分发)

### 1.2 你的贡献代表什么?

通过提 PR, 你**默认声明**:
1. 你**有权利**贡献这段代码 (你自己写的, 或者雇主允许的)
2. 你**同意**按 Apache-2.0 许可这段代码
3. 你**理解**项目**不**为你的贡献提供任何 **额外保证**

---

## 2. 代码风格 (8 项不修改承诺)

| # | 规则 | 工具 | 严守 |
|---|------|------|:----:|
| 1 | 0 触碰 **24 LOCKED crate src** | `apeireth-{core,memory,asi,...verify}` 24 个 | ✅ |
| 2 | 0 改 **6 哲学锚** (S-1/S-2/O-2/O-3/O-4/O-5) | per `docs/adr/0010-6-philosophy-anchors.md` | ✅ |
| 3 | 0 改 **workspace version 1.0.0** (semver 严守) | `Cargo.toml:188` 编译期 hardcode | ✅ |
| 4 | 0 重复造轮子 (借业界标准) | serde/tokio/axum/criterion/SQLx | ✅ |
| 5 | 0 假装已实现 (stub 必标 R21) | "不假装边界" per APEIRETH-CONVENTIONS §10 | ✅ |
| 6 | 0 改 7 LOCKED 文档 | `docs/adr/*.md` (除 archive) | ✅ |
| 7 | 0 触碰 sandbox 错路径 | 严守 `.openclaw\workspace\promethean\Apeireth-rust\` | ✅ |
| 8 | 0 主动 commit (除非主人授权) | 主人 2026-08-06 01:14 + 21:35 拍板 | ✅ |

---

## 3. 提 PR 前的本地守门

```bash
# 1. 编译
cargo build --workspace

# 2. 测试 (必须全过, 除非在 PR 描述中标 TODO R21)
cargo test --workspace --no-fail-fast

# 3. Clippy (无 warning)
cargo clippy --workspace --all-targets -- -D warnings

# 4. Format (rustfmt 默认)
cargo fmt --all -- --check

# 5. License check
cargo deny check licenses
# 期望: 0 errors, 0 warnings

# 6. Audit
cargo audit
# 期望: 0 vulnerabilities, 0 warnings

# 7. 文档生成测试
cargo doc --workspace --no-deps
# 期望: 0 warnings
```

**任何一个失败, 都不能提 PR。** (除非在 PR 描述里**显式标 TODO R21** + 主人授权)

---

## 4. commit message 格式

**严格按 Conventional Commits 1.0.0** (per `docs/adr/0004-8-promise-audit.md` §3.4):

```
<type>(<scope>): <subject>

<body>

<footer>
```

| type | 用途 | 例 |
|------|------|-----|
| `feat` | 新功能 | `feat(tui): add 9 organ command bar` |
| `fix` | 修 bug | `fix(api): rate-limit token bucket overflow` |
| `docs` | 文档 | `docs(api): add v1-tools-search example` |
| `style` | 格式 (无逻辑改) | `style(tui): reformat organ::heart mod` |
| `refactor` | 重构 (无功能改) | `refactor(sdk): split client into auth/api/streaming` |
| `test` | 加测试 | `test(voice): add 5 wake-word detection test` |
| `chore` | 工具 / 依赖 / CI | `chore(deps): bump tokio 1.40 -> 1.41` |
| `perf` | 性能 | `perf(vector): tantivy index batch 100 -> 1000` |

**scope** (可选): `tui` / `api` / `sdk` / `provider-claude-code` / `lark` / `voice` / `sandbox` / etc

**subject**:
- 50 字符以内
- 动词开头 (add / fix / bump / remove / etc)
- 小写, 末尾无句号

**body**:
- 72 字符换行
- 说明 **动机** + **改动** + **影响**, 不只列 diff

**footer**:
- 引用 issue: `Closes #123` / `Refs #456`
- breaking change: `BREAKING CHANGE: <desc>`

---

## 5. PR 模板 (5 段)

```markdown
## 1. 改了什么 (What)

1-3 句说明, e.g. "Add lark SDK send_message 真接实现 + 19 tests"

## 2. 为什么改 (Why)

引用 issue / discussion, e.g. "Closes #123 (lark 真接是 R20 阶段 6 估补需求)"

## 3. 怎么测的 (How tested)

- `cargo test --workspace`: 273/282 ok (8/9 failed group 修, 1 R21 续)
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 warning
- `cargo deny check licenses`: 0 error / 0 warning
- 新增 19 tests, 1 wiremock 端到端

## 4. 8 项不修改承诺检查 (per docs/stage4/8-locked-unified-2026-08-05.md)

- [x] 0 触碰 24 LOCKED src (本 PR 只动 crates/apeireth-lark/src/ + tests/)
- [x] 0 改 workspace version 1.0.0
- [x] 0 重复造轮子 (借 reqwest + serde, 业界主流)
- [x] 0 假装 (5 端点 + 19 tests 全过, 0 stub)
- [x] 0 改 7 LOCKED 文档
- [x] 6 哲学锚穿透 (S-1 借业界 + S-2 真接 + O-2 用户看结果 + O-3 信息密度 + O-4 任何人都能接手 + O-5 不假装)
- [x] 0 主动 commit (本 PR squash merge)
- [x] 0 sandbox 错路径

## 5. 6 哲学锚穿透自检 (per docs/adr/0010-6-philosophy-anchors.md §2.5)

| 锚 | 在本 PR 的体现 |
|----|---------------|
| S-1 走在前人肩上 | 借 reqwest 0.12 + serde 1.0 (业界主流) |
| S-2 实事求是 | 5 端点 + 19 tests 全过, 0 stub |
| O-2 用户看结果 | SDK 调用返 Result, 不暴露内部 retry 逻辑 |
| O-3 干到底 | 1 屏看 5 端点 + 19 tests 表格 |
| O-4 任何人都能接手 | 19 tests + 1 e2e 例子, 0 legacy 兼容 |
| O-5 不假装 | R21 真接 RUSTSEC fix 在本 PR 完成, 0 标缺 |

## 6. Checklist

- [x] 1. CI 12 workflow 全过
- [x] 2. 5 reviewer approve
- [x] 3. 8 项承诺全守
- [x] 4. 6 哲学锚穿透
- [x] 5. CHANGELOG.md 已更新
```

---

## 6. 5 reviewer 守门

| Reviewer | 关注 | 工具 |
|----------|------|------|
| **architect** | 架构决策 + 6 哲学锚 | `CODEOWNERS` + `docs/adr/` |
| **code-reviewer** | 代码风格 + 8 项承诺 + clippy | `cargo clippy --workspace -- -D warnings` |
| **qa-engineer** | 测试覆盖 + 集成测试 + e2e | `cargo test --workspace` |
| **security-reviewer** | 4 RUSTSEC + cosign + 0 NewAPI | `cargo audit` + `cargo deny` |
| **technical-writer** | 文档 / CHANGELOG / 6 哲学锚穿透 | `docs/` + `CHANGELOG.md` |

**squash merge 需 5/5 approve** 才能合入 main.

---

## 7. release 节奏 (R-Cycle)

| 阶段 | 时长 | 内容 |
|------|------|------|
| **Rxx 计划** | 1-2 天 | 主人拍板 Rxx 阶段目标 + 派工 |
| **Rxx 估补** | 1-2 周 | sub-agent 并行估补 (4 满硬限) |
| **Rxx 整合** | 1-2 天 | 整合 #N 拍板 (7 commit 模板) |
| **Rxx 收尾** | 0.5-1 天 | 12 项 checklist 100% 验证 + 报告 |
| **Rxx tag** | 0 | 打 v1.x.x tag (per APEIRETH-VERSIONING.md §1) |

**R21 估补 14h / 2 工作日** (整合 #3 续补估 1-2h / 项 × 5 项)

---

## 8. 不接受什么 PR

| 类型 | 理由 |
|------|------|
| ❌ 改 24 LOCKED src | per 8 项承诺 #1 |
| ❌ 改 6 哲学锚 | per 8 项承诺 #2 |
| ❌ 改 workspace version 1.0.0 | per 8 项承诺 #3 (semver 严守) |
| ❌ 引入 NewAPI 商业版 | per 8 项承诺 #6 + 主人 2026-08-04 拍 |
| ❌ 假装已实现 (stub 不标 TODO R21) | per 8 项承诺 #5 |
| ❌ 改 sandbox 错路径 | per 8 项承诺 #7 |
| ❌ 重复造轮子 (不用 serde/tokio/criterion) | per 8 项承诺 #4 |
| ❌ 不写测试 | 0 coverage = 不接受 |
| ❌ 不更新 CHANGELOG.md | per Keep a Changelog 1.1.0 |
| ❌ 不填 PR 模板 5 段 | 5 reviewer 没法审 |

---

## 9. 第一次贡献? (新手友好)

| 难度 | 类型 | 例 |
|------|------|-----|
| 🟢 简单 | 文档 typo / 链接修 / 例子补充 | `docs/license/04-faq.md` 加 1 问 |
| 🟢 简单 | 测试覆盖 | `crates/apeireth-voice/tests/wake_word_test.rs` 加 1 test |
| 🟡 中等 | 加新 SDK 端点 | `crates/apeireth-lark/src/` 加 1 方法 |
| 🟡 中等 | 加新 Provider 工具 | `crates/apeireth-provider-claude-code/src/tools/` 加 1 工具 |
| 🔴 难 | 跨 crate 改动 | 需主人授权, 不建议新手 |
| 🔴 难 | 改 LOCKED crate | **禁止** |

**建议**: 第一次贡献选 🟢 简单类型, 熟悉流程后再上 🟡 中等.

---

## 10. 相关

- 根 `CONTRIBUTING.md` (简要版)
- 根 `CODEOWNERS` (5 reviewer 分工)
- 根 `SECURITY.md` (安全漏洞报告流程)
- [docs/adr/0004-8-promise-audit.md](../adr/0004-8-promise-audit.md) (8 项不修改承诺详)
- [docs/adr/0010-6-philosophy-anchors.md](../adr/0010-6-philosophy-anchors.md) (6 哲学锚)
- [docs/adr/0005-1.0-release-checklist.md](../adr/0005-1.0-release-checklist.md) (R-Cycle)

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-5)
