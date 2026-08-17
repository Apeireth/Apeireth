# E-4 草稿 — 根 README "## 🤝 贡献" 节

```
[Document-Meta]
Document:       docs/1.0-release-prep/04-contribution.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 12 项 #1 doc E-4 续补
Last-Modified:  2026-08-06
Status:         🟢 草稿 (根 README.md LOCKED, 等 Mavis 整合 #3 拍板)
Author:         Mavis (Mavis@local)
Source:         续 reports/1.0-release-doc-30-2026-08-06.md §1.2 E-4
Target:         贡献入口显眼 (CONTRIBUTING.md 1 跳可达)
```

> **性质**: 根 README.md **没"贡献"明文入口** 草稿 (per 续补报告 §1.2 E-4: CONTRIBUTING.md 3095 字节已建, 但 8 套规范系统引 CONTRIBUTING.md 是间接入口, 接手者需 1 跳才能看到完整 PR 流程).
>
> **本节草稿目标**: 让贡献者 **1 跳** 看到 CONTRIBUTING.md 入口 + 4 必读规范 + 入门路径 4 表 + 安全漏洞入口.
>
> **不假装**: 草稿 4 表基于 `CONTRIBUTING.md` 实查 (0 触碰实查命令 + 6 哲学 anchor + 8 项承诺 + commit 规范), 0 编造.

---

## §0. 草稿内容 (建议合入根 README 引用节后)

> **合入位**: 根 README 引用节 (line 263 后新增) 后, **新增** 1 个 H2 节 "## 🤝 贡献".

```markdown
## 🤝 贡献 (Contribution)

**贡献入口** (必读): [`CONTRIBUTING.md`](./CONTRIBUTING.md) (6 哲学 anchor + 8 项不修改承诺 + PR 流程 + 1.0 release 12 项 checklist).

### 4 必读规范

| 文档 | 主题 | 行数 |
|------|------|-----:|
| [`APEIRETH-CONVENTIONS.md`](./APEIRETH-CONVENTIONS.md) | 工程哲学铁律 (6 哲学 anchor + 8 项不修改承诺) | ~600+ |
| [`APEIRETH-VERSIONING.md`](./APEIRETH-VERSIONING.md) | semver 严守 (workspace version 1.0.0) | ~150+ |
| [`APEIRETH-GLOSSARY.md`](./APEIRETH-GLOSSARY.md) | 17 项术语表 | ~100+ |
| [`ROADMAP.md`](./ROADMAP.md) | 1.0 release 路线图 (R20 阶段 1-6 + 9-30 tag 计划) | ~270+ |

### 入门路径 (4 阶段精读)

| 时间 | 读什么 | 关键 commit |
|------|--------|-------------|
| **5 分钟** | 本 README + [`docs/1.0-release/README.md`](./docs/1.0-release/README.md) | `02d5db6c` (1.0 release 报告) |
| **30 分钟** | [`APEIRETH-CONVENTIONS.md`](./APEIRETH-CONVENTIONS.md) + [`docs/adr/0010-6-philosophy-anchors.md`](./docs/adr/0010-6-philosophy-anchors.md) | `629995d3` (8 项承诺审计) |
| **1 小时** | [`docs/architecture-v4-1-living-intelligence-update.md`](./docs/architecture-v4-1-living-intelligence-update.md) | (LOCKED 阶段 1) |
| **4-6 小时** | [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](./docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) + [`docs/stage6/22-trait-interlock.md`](./docs/stage6/22-trait-interlock.md) | `8a643778` (蓝图 604 行) |

### PR 流程 8 步 (per CONTRIBUTING.md)

1. fork + branch (`feat/xxx` / `fix/xxx` / `chore/xxx` / `docs/xxx`)
2. 写代码 + 测试 (`cargo test --workspace` 全绿)
3. **0 触碰实查** (24 LOCKED crate + workspace version):
   \`\`\`bash
   git diff main..HEAD -- crates/apeireth-{supervisor,agent,council,bus,protocol,mcp,tool-registry,tool-runtime,graph,pipeline,tool-approval,extension,evolution,api,core,memory,asi,tools,cli,bench,cognition,action,life-force,constraint}
   # 必须 0 行
   \`\`\`
4. 6 哲学 anchor + 8 项承诺严守 (见 CONTRIBUTING.md §6 哲学 anchor 必穿透 + §8 项不修改承诺)
5. 提 PR (用 `.github/PULL_REQUEST_TEMPLATE.md` 模板)
6. CODEOWNERS 自动 review (@chuling)
7. CI green (cargo audit + cargo deny + cargo bench)
8. 1.0 release 12 项 checklist 标记

### 安全漏洞报告

按 [`.well-known/security.txt`](./.well-known/security.txt) RFC 9116 报告:
- `mailto:security@apeireth.local`
- https://github.com/apeireth/apeireth-rust/security/advisories/new
```

---

## §1. 草稿要点 (Mavis 整合 #3 拍板用)

| # | 要点 | 依据 |
|---:|------|------|
| 1 | **CONTRIBUTING.md 显眼入口**: 第 1 行加粗 | per CONTRIBUTING.md line 1-3 (6 哲学 anchor + 8 项承诺 必读) |
| 2 | **4 必读规范表**: CONVENTIONS / VERSIONING / GLOSSARY / ROADMAP | per CONTRIBUTING.md line 7-10 (必读段) |
| 3 | **入门路径 4 阶段**: 5min / 30min / 1h / 4-6h | per 根 README line 385-403 (精读顺序 §, 沿用不重造) |
| 4 | **PR 流程 8 步**: fork / branch / test / 0 触碰 / 哲学锚 / PR / CI / 1.0 标记 | per CONTRIBUTING.md line 47-54 (PR 流程) |
| 5 | **0 触碰实查命令**: 1 行 bash 列出 24 LOCKED crate 校验 | per CONTRIBUTING.md line 14-22 (0 触碰实查) |
| 6 | **安全漏洞入口**: security@apeireth.local + GitHub advisories | per CONTRIBUTING.md line 90-92 (RFC 9116) |

---

## §2. 守门表

| 守门 | 本草稿 | 验证 |
|------|--------|:----:|
| **0 触碰根 README.md** (LOCKED) | 草稿在本文件, 不动根 README | ✅ |
| **0 触碰根 CONTRIBUTING.md** (LOCKED, 3095 字节) | 草稿仅引用, 不复制粘贴 | ✅ |
| **0 触碰根 ROADMAP.md** (LOCKED) | 草稿仅引用 §R20 阶段 1-6 | ✅ |
| **0 改 workspace version** | 草稿不动 Cargo.toml | ✅ |
| **6 哲学锚穿透** (S-1/S-2/O-2/O-3/O-4/O-5) | O-4 任何人都能接手 (4 阶段精读表) + S-2 实事求是 (24 LOCKED crate 实查命令) | ✅ |
| **8 项不修改承诺** | 不假装已实现 + 编译期 hardcode (0 触碰实查命令) + 不重复造轮子 (沿用 CONTRIBUTING.md 8 步) | ✅ |
| **诚实标缺** | 入门路径 4 表基于根 README 精读顺序 (line 385-403) 实查, 0 编造 | ✅ |

---

## §3. R21 续合入动作

1. 主解除根 README.md LOCKED
2. R21 sub-agent 在根 README 引用节后**新增** 1 个 H2 "## 🤝 贡献" (per §0 草稿)
3. 估 commit: `docs: R21 续 — 根 README 加"贡献"节 (per #1 doc 续补 E-4, 4 必读 + 4 阶段 + 8 步 PR)`
4. 工时估: 0.5h (新增 H2 + 复刻 §0 草稿)

---

_本草稿路径: `docs/1.0-release-prep/04-contribution.md`_
_生成时间: 2026-08-06_
_续: `reports/1.0-release-doc-30-2026-08-06.md` §1.2 E-4 (根 README 没"贡献"明文入口, 估补 0.5h → 草稿 0.5h, 合入 0.5h)_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit_
