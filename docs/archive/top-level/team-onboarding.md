# Apeireth 1.0 release 团队入职

> **目标读者**: 新加入 Apeireth 项目的工程师 / 设计师 / AI 协作者
> **最后更新**: 2026-08-05
> **必读**: [`APEIRETH-CONVENTIONS.md`](../APEIRETH-CONVENTIONS.md) (per §9 6 哲学锚 + §10 7 项不修改承诺) + [`CONTRIBUTING.md`](../CONTRIBUTING.md) (per 8 项不修改承诺)
> **不假装**: 本文档是 1.0 release 团队规范的**索引 + 节奏**, 不重写已有规范. 详细规范看 4 份源文档.

```
[Document-Meta]
Document: docs/team-onboarding.md
Version: Manual-Rev-A
R-Cycle: R20 阶段 6 (1.0 release 团队规范)
Status: ✅ 1 commit 落地
依据: docs/release/1.0.0-release-report-2026-08-05.md
依据: docs/stage4/r20-阶段-1-收官-2026-08-05.md
依据: APEIRETH-CONVENTIONS.md §9 §10
依据: CONTRIBUTING.md (8 项不修改承诺)
```

---

## 1. Apeireth 是什么

Apeireth 是 AGI 操作系统 Rust 重写, R14 阶段起:

- 立体架构 v2 + 生命架构 v4/v4.1
- 17 crate 本源推导 + 双洋葱统一体
- Self-Disable 防护 (apeireth-rollback 71GB 4 重防御 hardcode)

**前端路线** (per 主人 2026-08-04 拍板):
- **TUI** — 现在 (过渡, 主人自己干)
- **Tauri 2.0** — 终极 (等设计团队到位)

**核心资产**:
- 58 crate 在 `crates/` (5 P0 MCP + 14 new R20 阶段 1 + 9 skeleton + 24 LOCKED + 6 杂项)
- workspace v1.0.0 + rust-version 1.80 + Apache-2.0
- 193/193 测试全过 (per R20 阶段 1 收官)

---

## 2. 仓库结构

| 路径 | 内容 | 估行数 |
|---|---|---|
| `crates/` | 58 Rust crate (5 P0 MCP + 9 skeleton + 14 估补 + 24 LOCKED + 6 杂项) | — |
| `docs/` | 顶层文档 + 阶段 1-6 子目录 | 7 LOCKED + 14 估补 |
| `scripts/` | 升级 / 卸载 / cosign / 8 项承诺审计 / 迁移 | 8 估补 |
| `src-tauri/` | Tauri 2.0 desktop scaffold (R20 阶段 5 估补) | — |
| `.github/workflows/` | 3 workflow (release-1.0.0.yml + dependabot-upgrade.yml + benchmark-tracking.yml) | — |
| `.well-known/security.txt` | RFC 9116 安全漏洞报告入口 | 21 行 |
| `CODEOWNERS` | 24 LOCKED + 14 new + 5 P0 + 9 skeleton + 7 LOCKED doc 全部有 owner | — |
| `CONTRIBUTING.md` | 8 项不修改承诺 + PR 流程 (源规范 #1) | 92 行 |
| `APEIRETH-CONVENTIONS.md` | §9 6 哲学锚 + §10 7 项 LOCKED (源规范 #2) | 264 行 |
| `APEIRETH-VERSIONING.md` | semver 严守 (源规范 #3) | — |
| `APEIRETH-GLOSSARY.md` | 术语表 (源规范 #4) | — |

**入口顺序** (新成员第一周):
1. `CONTRIBUTING.md` — 8 项承诺 + PR 流程
2. `APEIRETH-CONVENTIONS.md` — 6 哲学锚穿透
3. `docs/release/1.0.0-release-report-2026-08-05.md` — 1.0 release 全局
4. `docs/stage4/r20-阶段-1-收官-2026-08-05.md` — R20 阶段 1 实查

---

## 3. 工具链

| 工具 | 版本 | 备注 |
|---|---|---|
| **Rust** | 1.80 (per `Cargo.toml [workspace.package] rust-version`) | workspace 锁 |
| **Cargo** | semver v1.0.0 严守 | 不改 workspace version 字段 |
| **License** | Apache-2.0 | per `Cargo.toml [workspace.package]` |
| **Mavis** | MiniMax-M3 (本助手) | 派 sub-agent 干实施 / 文档 / 治理 |
| **Git** | 主分支 `code_reviewer/t15-fix-rebase` (per 主人 21:18 拍板) | — |
| **CI** | GitHub Actions (3 workflow) | cargo audit + cargo deny + cargo bench |
| **签名** | cosign v2.2+ (sigstore 业界标准, 1-of-1 阈值) | 8 包 |
| **数据库** | SQLite (v2.0.0-alpha) → PostgreSQL (v1.0.0, D-07 一次性迁移) | per `f5c44769` |
| **包管理** | 8 形态 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) | per D-06 |
| **i18n** | 5 语言 (en / zh-CN / ja / fr / de) | — |
| **Editor** | 任意 (推荐 Rust-analyzer) | — |

**0 改实查命令** (提交 PR 前必跑, per `CONTRIBUTING.md` §0):

```bash
# 0 改 24 LOCKED crate (LOCKED list 在 CONTRIBUTING.md §0 实查命令里)
git diff main..HEAD -- crates/apeireth-{supervisor,agent,council,bus,protocol,mcp,tool-registry,tool-runtime,graph,pipeline,tool-approval,extension,evolution,api,core,memory,asi,tools,cli,bench,cognition,action,life-force,constraint}
# 必须 0 行

# 0 改 workspace version
git diff main..HEAD -- Cargo.toml | grep '^+.*version'
# 必须 0 行
```

---

## 4. 6 哲学锚 (per APEIRETH-CONVENTIONS §9 + CONTRIBUTING.md §6 哲学 anchor 必穿透)

每个 PR 必含 §1 §2 §3 §4 §5 §6 章节引用, 6 章节切分:

- **§1 S-1 北极星导向**: 1:1 翻译 v0.9.21 商业版? 0 重设计?
- **§2 S-2 实事求是**: 实查 5 决策点 (loc / files / deps / hardcode / 0 触碰)?
- **§3 O-2 走在前人肩上**: 复用 std / tokio / 业界标准?
- **§4 O-3 干到底**: 测试 N/N passed + cargo check 0 error?
- **§5 O-4 任何人都能接手**: doc 顶部 30+ 行 + 编译期 hardcode 守门?
- **§6 O-5 不假装**: skeleton 标 ⏳ + 不假装已实现?

**穿透原则**: 每 5 个成就 = 强制自检 6 锚. 任何一锚缺 = PR 不通过.

---

## 5. 8 项不修改承诺 (per CONTRIBUTING.md §8 项不修改承诺 必严守)

每个 PR 必严守 8 项:

- ✅ **不假装已实现** — skeleton 标 ⏳, 0 mock 假装上线
- ✅ **编译期 hardcode** — TTL / 容量上限 / 守门规则 全部 `const` 或 `#[compile_error]`
- ✅ **不改 LOCKED 24 crate** — mtime 实查 baseline 16:34 之前 (per R20 阶段 1 实查)
- ✅ **不改 workspace version** — semver v1.0.0 严守
- ✅ **6 哲学 anchor 穿透** — 见 §4
- ✅ **不依赖 NewAPI** — 不引入独立代理服务 (R17 砍)
- ✅ **不重复造轮子** — 复用 std / tokio / 业界标准, 引用 4 份 reports 完整 path
- ✅ **诚实标缺** — 缺 = 标 ⏳ + 估时 + owner, 不静默跳过

---

## 6. 4 决策 (主人 2026-08-05 20:53 拍板)

| ID | 决策 | 主人推翻 | 实施 |
|---|---|---|---|
| **D-01** | calendar + message 2 工具 1.0 release 阶段 2 真接 | 推翻 A 推荐 stub 501 (原话"加") | R20 阶段 2 续 (`bg_2be338d0`) |
| **D-02** | 6 工具各 1 URL 子路径 `/v1/tools/{name}/invoke` | 按 A 推荐 | R20 阶段 2 续 |
| **D-06** | 8 包齐发 (deb / rpm / brew / scoop / tarball / zip / MSI / Docker) | 按 A 推荐 + 主人补充"搞技术用户很多 Linux" (Linux 4 包重点: deb / rpm / tarball / Docker) | R20 阶段 3 + 阶段 6 |
| **D-07** | 一次性 SQLite → PostgreSQL 迁移 | 推翻 B 推荐双写 7 天 (原话"现在没用户用, 我都没怎么用过") | `f5c44769` |

**完整决策表** (4 + 3 默认 = 7/7): 见 `docs/release/1.0.0-release-report-2026-08-05.md` §3.2-§3.3.

**默认决策** (主人未反对): D-03 链接 token + D-04 鉴权 + D-05 加密 — 全部 1.0 release 阶段 2 实施.

---

## 7. Mavis sub-agent 派单规范

主 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力" → 20:30 "最大效率速度推进" → 21:18 "cpu 9955hx 内存 32G, 还能派的都给我派了".

**Mavis 派 sub-agent 干 3 类任务** (估 5-10min skeleton):

| 类别 | 例子 | 估时 |
|---|---|---|
| **实施类** | 代码 / 配置 / 脚本 | 5-10min skeleton |
| **文档类** | markdown / .well-known / .github | 5-10min |
| **治理类** | CODEOWNERS / security.txt / 8 项承诺审计 | 5-10min |

**派活规范**:
1. 写清任务 + 集成规范 + 不重复造轮子 (引用 4 份源报告 path)
2. 整合时**先看 sub-agent 产出**, 不要重写
3. 整合后 `git diff main..HEAD` 实查 0 触碰 24 LOCKED

**整合 #1+#2+#3 模式** (per R20 阶段 1 实操):
- 整合 #1 = 5 P0 MCP crate 一次性入 workspace (`128f9704` +5,731 lines)
- 整合 #2 = 9 skeleton crate 一次性入 workspace (`ae7bd2e5` +21,011 lines)
- 整合 #3 = 5 P0 + 9 skeleton + 14 估补 + 24 LOCKED 全 0 改 (git diff 空)

**Mavis 角色**: team lead (协调 + 整合 + 决策), 不是 worker. 主人 (chuling) 才是 dev 主.

---

## 8. 1.0 release 时间表

| 阶段 | 状态 | 计划日期 | 关键 commit |
|------|------|----------|------------|
| R20 阶段 1 收官 | ✅ DONE | 2026-08-05 | `8a643778` (蓝图) + `128f9704` (整合 #1) + `ae7bd2e5` (整合 #2) + `5f5b5fa3` (收官) + `3bc61686` (ROADMAP) + `6c518ee3` (CHANGELOG+README) |
| R20 阶段 2 公开 API | 🟡 IN PROGRESS | 2026-08-06 ~ 2026-08-15 | `6d6db9b0` (WS 8 帧 + 鉴权 5 组件, D-03) + 6 工具 endpoint (D-01 / D-02) 续 |
| R20 阶段 3 Docker + 8 包 | 🟡 IN PROGRESS | 2026-08-06 ~ 2026-08-15 | `f5c44769` (D-07 一次性迁移 + 卸载) + 8 包 Dockerfile (D-06) 续 |
| R20 阶段 4 16 估缺主体 | ⏸ PLANNED | 2026-08-16 ~ 2026-08-25 | 估 60h, 4 owner × 1.5 周 |
| R20 阶段 5 SDK + Tauri 2.0 | ⏸ PLANNED | 2026-08-26 ~ 2026-09-05 | 估 80h, 1 sub-agent Tauri scaffold 跑 (`src-tauri/`) |
| R20 阶段 6 1.0 release | 🟡 IN PROGRESS | 2026-09-06 ~ 2026-09-30 | `acfa963d` (CI 3 workflow) + `bbb26266` (cosign 8 包) + `702942fb` (workspace 治理) + 本文档 |
| **`v1.0.0` release tag** | ⏸ 计划 | **2026-09-30** | 12 项 checklist 全 PASS + 8 包全签名 + 升级 + 卸载 dry-run 全 0 错 |

**R20 阶段 6 估补中子任务** (per 主人 21:18 "内存大, 都派了"):

- observability (tracing + Prometheus metrics endpoint)
- i18n (zh-CN / en 0 missing)
- cargo audit / cargo deny / cargo bench baseline
- OSS NOTICE + 第三方 LICENSE 收集
- Tauri 2.0 desktop scaffold (per `src-tauri/`)
- 团队规范 7 文件 (PR / issue / CODEOWNERS / security.txt / CONTRIBUTING / CHANGELOG 模板)
- **team-onboarding.md (本文件)** ✅

---

## 9. 紧急救援 / 安全

### 9.1 71GB 事故根因 (per 2026-08-05 紧急救援)

- **修复**: `apeireth-rollback` 编译期 hardcode 6 重防御
- **守门**: TTL 7 天 + 单影子 100 MB + 总 2 GB + 3 清理钩子
- **位置**: `crates/apeireth-rollback/src/lib.rs` (1,040 行, 23/23 测试)

### 9.2 安全漏洞报告

按 **`.well-known/security.txt` RFC 9116**:

- mailto:security@apeireth.local
- https://github.com/apeireth/apeireth-rust/security/advisories/new
- 加密: https://github.com/apeireth/apeireth-rust/blob/main/docs/security/cosign.pub
- Expires: 2027-12-31T23:59:59Z

### 9.3 紧急联系

- **chuling@apeireth.local** (主人)
- **mavis@local** (Mavis 助手)

### 9.4 m3 hallucination 防御

per `docs/stage4/m3-hallucination-defense-2026-08-05.md`:

- 4 P0 crate 加 `TOOL_WHITELIST` 编译期 hardcode
- `validate_tool_call` 入口守门
- apeireth-team-lead supervisor_prompt 编译期嵌入 (303 行 7 段)

---

## 附录 A. 引用源 (4 份 reports, 0 重复造轮子)

| # | 路径 | 行数 | 用途 |
|---|---|---:|---|
| 1 | `docs/release/1.0.0-release-report-2026-08-05.md` | ~460 | 1.0 release 全局 + 时间表 + 决策 + 12 项 checklist |
| 2 | `docs/stage4/r20-阶段-1-收官-2026-08-05.md` | ~494 | R20 阶段 1 实查 (5 P0 + 9 skeleton + 71GB 修复) |
| 3 | `CONTRIBUTING.md` | 92 | 8 项不修改承诺 + PR 流程 |
| 4 | `APEIRETH-CONVENTIONS.md` | 264 | §9 6 哲学锚 + §10 7 项 LOCKED |

**本文件不重写上述 4 份内容, 只做索引 + 节奏登记**. 任何成员看一份 onboarding 即可追溯全规范.

---

_Generated by Mavis on 2026-08-05 21:27, per 主人 21:27 拍板"效率不慢下来, 验收了继续派"._
_1 commit 落地, 0 触碰 24 LOCKED, 0 改 workspace version, 0 引 NewAPI._
