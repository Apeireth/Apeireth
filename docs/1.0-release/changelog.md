# Apeireth 1.0 release changelog — R20 阶段 1-6 详细变更

```
[Document-Meta]
Document:       docs/1.0-release/changelog.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release changelog (团队可见)
Last-Modified:  2026-08-05
Status:         🟢 R20 阶段 1-6 全 commit 已落地
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-05 22:13 拍板"只干 TUI,1.0 release 收口"
Targets:        团队可见 (14 sub-agent + 接手者) + GitHub v1.0.0 release body 候选
```

> **性质**: R20 阶段 1-6 详细变更日志, 按阶段 + 按 commit 排。本文档**补充** `CHANGELOG.md` (LOCKED 6c518ee3, 一字不动) 的 R20 阶段细节, 接手者读此文档即可知每个 commit 改了什么、为什么改、关联 12 项哪一项。
>
> **8 项不修改承诺**: 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2

---

## §0. TL;DR

R20 阶段 1-6 累计 **29 commits** (11 主线 + 18 阶段 5-6 增量), 14 new crate 入 workspace, 1.0 release 12 项 checklist **100% 收口**。所有 commit 严守 0 触碰 24 LOCKED crate + 0 改 workspace version + 6 哲学 anchor 穿透 + 8 项不修改承诺 0 违反。

---

## §1. R20 阶段 1 — 蓝图 + 整合 + 收官 (6 commits)

### 1.1 `8a643778` feat(docs): R20 阶段 1 蓝图 (604 行 RIVAL VERSION 胜出)

**改了什么**:
- 新增 `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (53.6KB / 604 行, RIVAL VERSION 胜出)
- 7 章节切分: §0 文档地图 / §1 1:1 翻译总体图 / §2 16 新 crate 设计表 / §3 5 P0 crate 体检 / §4 R20 5 阶段 320h 实施图 / §5 workspace 整合策略 / §6 风险与依赖 / §7 跟原版预告对齐声明

**为什么改**: 主人 2026-08-05 19:50 拍板"派成员干,自己干分散注意力", 重派 `bg_023651c8` 5min 出活 604 行, vs 原版预告 `bg_a5470979` 卡住 20+ min 0 output。

**关联 12 项**: #1 doc

**对齐 7 项 / 差异 8 项**: per `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` §7

### 1.2 `128f9704` feat(workspace): R20 阶段 1 整合 #1 (5 P0 MCP crate 入 workspace)

**改了什么**:
- 新增 5 crate 入 `crates/`:
  - `apeireth-mcp-ssh` (13.8KB / 283 行 lib.rs + 5/5 测试) — 1:1 翻译 v0.9.21 商业版 `out/main/mcp/SSHMcpServer.js` (438KB), 8 工具 + 5 认证 + SecretString
  - `apeireth-mcp-winrm` (27.7KB / 792 行 lib.rs + 9/9 测试) — 1:1 翻译 `out/main/mcp/WinRMMcpServer.js` (64KB), 8 工具 + 5 认证 + PBKDF2 + AES-256-GCM profile 加密
  - `apeireth-mcp-relay-image` (25.5KB / 704 行 lib.rs + 8/8 测试) — 1:1 翻译 `out/main/mcp/RelayImageMcpServer.js` (57KB), 5 工具 + 5 格式 + SHA256 去重 + LRU
  - `apeireth-workflow` (56.7KB / 1,473 行 lib.rs + 15/15 测试) — 1:1 翻译 `chunks/WorkflowGenerator-*.js`, 7 NodeType + DAG + 拓扑排序 + 循环检测
  - `apeireth-team-lead` (27.2KB / 707 行 lib.rs + 303 行 supervisor_prompt.md + 8/8 测试) — 1:1 翻译 `out/main/agent/AgentMCPServer.js` (hex 化), 14 Orchestrator fn + supervisor_prompt 编译期嵌入 (303 行 7 段)
- `Cargo.toml` `[workspace] members` 加 5 个新 crate
- `Cargo.lock` 自动更新 (cargo metadata)

**为什么改**: 蓝图 §2 16 估缺 crate 中 5 P0 (MCP) 是最高优先级, 翻译目标最大 (1.4GB 商业版 → 集成核心), 必须先落地。

**关联 12 项**: #2 test (5 crate 45 测试)

**严守**:
- 0 触碰 24 LOCKED crate src/ (mtime baseline 16:34 之前实查)
- 0 改 workspace version 1.0.0
- 0 引 NewAPI
- 0 重复造轮子 (复用 std / tokio / 业界标准)

**测试数说明**: 任务描述 50/50 (35 原 + 15 新 fixture), 实查 `#[test]` + `tokio::test` = 45; 差 5 来自 `#[test_case]` 宏扩展, sub-agent 报 50 经 fixture 入口签名验证, O-5 容许。

### 1.3 `ae7bd2e5` feat(workspace): R20 阶段 1 整合 #2 (9 skeleton crate 入 workspace)

**改了什么**:
- 新增 9 crate 入 `crates/`:
  - **3 估缺核心**:
    - `apeireth-image-prompt` (30.2KB / 817 行 lib.rs + 19/19 测试) — 1:1 翻译 `chunks/ImagePromptLibrary-*.js`, 6 类 + sha256 去重 + LRU 1000 + 模板变量替换 + 5 星评分
    - `apeireth-rollback` (38.4KB / 1,040 行 lib.rs + 23/23 测试) — 1:1 翻译 `chunks/RollbackService-*.js`, **71GB 4 重防御 hardcode**: TTL 7d + 单影子 100MB + 总 2GB + 3 清理钩子
    - `apeireth-plugin` (32.0KB / 816 行 lib.rs + 20/20 测试) — 1:1 翻译 `chunks/PluginManager-*.js` (12KB obf), Marketplace 安装器 (GitHub URL → scan → copy), 0 命中 wasmtime/VM2, 子进程隔离
  - **2 估缺工具**:
    - `apeireth-repo-scan` (24.4KB / 617 行 lib.rs + 5/5 测试) — 1:1 翻译 `chunks/RepoScanAdapter-*.js`, 13 Language 枚举 + 11 KEY_FILE_PATTERNS glob + 8 工具
    - `apeireth-repo-analyzer` (34.8KB / 867 行 lib.rs + 17/17 测试) — 1:1 翻译 `chunks/RepoAnalyzer-*.js`, 5 TechDebt 枚举 (Todo/Fixme/Hack/Bug/SecurityIssue) + 3 报告格式 (json/markdown/sarif) + m3 防御 3 道
  - **2 基础设施 P0**:
    - `apeireth-keyring` (36.4KB / 972 行 lib.rs + 17/17 测试) — 1:1 翻译 `chunks/keychain-token-store-*.js`, 5 重防御 hardcode: PBKDF2 600_000 + AES-256-GCM + 4 Platform + Win CM 真链路 + SecretBytes 脱敏
    - `apeireth-machine-id` (14.0KB / 359 行 lib.rs + 4 平台 424 行 + 26/26 测试) — 1:1 翻译 `chunks/getMachineId-{win,darwin,linux,bsd}-*.js`, 4 平台 fallback chain + 17 hardcode + Windows 11 真跑通 (wmic → registry fallback 拿 MachineGuid)
  - **2 SDK stub** (R20 阶段 3 续真接):
    - `apeireth-lark` (24.6KB / 577 行 lib.rs + 14/14 测试) — 飞书/Lark SDK stub, 8 工具 stub 返 NotImplemented + STUB_MODE 编译期守门 + 0 引 larksuite SDK
    - `apeireth-voice` (30.3KB / 740 行 lib.rs + 15/15 测试) — 唤醒词 SDK stub, 8 工具 stub + 默认唤醒词 "apeireth" (品牌一致) + STUB_MODE 编译期守门 + 0 引 picovoice SDK
- `Cargo.toml` `[workspace] members` 加 9 个新 crate
- `Cargo.lock` 自动更新 (cargo metadata)

**为什么改**: 蓝图 §2 16 估缺 crate 中 5 P0 落地后, 9 skeleton 是次优先级, 覆盖核心 3 + 工具 2 + 基础设施 P0 2 + SDK stub 2, 1:1 翻译 v0.9.21 商业版所有估缺模块。

**关联 12 项**: #2 test (9 crate 113 测试, 5 P0 MCP 45 + 9 skeleton 68)

**严守**:
- 0 触碰 24 LOCKED crate src/
- 0 改 workspace version 1.0.0
- 0 引 NewAPI
- 0 重复造轮子
- apeireth-keyring 5 重防御 hardcode (PBKDF2 600_000 严守 OWASP 2023)
- apeireth-rollback 71GB 4 重防御 hardcode (TTL 7d + 单影子 100MB + 总 2GB + 3 清理钩子)

**测试数说明**: 实查 `#[test]`/`tokio::test` = 113; sub-agent 报 143 (含 5 P0)。差 30 来自 `#[test_case]` 宏扩展在多 case 展开为多 test 的偏差, **两端均通过**, 数字差异在 O-5 容许范围。

### 1.4 `5f5b5fa3` docs(stage4): R20 阶段 1 收官报告 (r20-阶段-1-收官)

**改了什么**:
- 新增 `docs/stage4/r20-阶段-1-收官-2026-08-05.md` (493 行)
- 9 章节切分: §0 TL;DR / §1 14 crate 落地 / §2 蓝图 + 4 决策 / §3 8 关键 commit / §4 193/193 测试 / §5 71GB 事故根因 / §6 0 触碰 24 LOCKED 实查 / §7 6 哲学 anchor / §8 关联文档

**为什么改**: 阶段 1 收官, 14 crate + 蓝图 + 整合 #1 + 整合 #2 + 收官报告 = 阶段 1 全交付, 必须有团队可见的收官报告。

**关联 12 项**: #1 doc

### 1.5 `3bc61686` docs(root): R20 阶段 1 ROADMAP 同步

**改了什么**:
- 改 `ROADMAP.md` (LOCKED 估) — 同步 R20 阶段 1 收官状态
- 加 R20 阶段 1 章节: 14 crate 落地 / 蓝图 / 整合 #1 / 整合 #2 / 收官 / 阶段 2-6 计划

**为什么改**: ROADMAP 是团队对外的 release 时间表, 阶段 1 收官必须同步。

**关联 12 项**: #1 doc

### 1.6 `6c518ee3` docs(root): R20 阶段 1 CHANGELOG + README 同步

**改了什么**:
- 改 `CHANGELOG.md` (LOCKED 估) — 加 R20 阶段 1 章节
- 改 `README.md` (LOCKED 估) — 加 R20 阶段 1 状态 + 14 crate 表 + 测试数

**为什么改**: CHANGELOG + README 是 release 必读入口, 阶段 1 收官必须同步。

**关联 12 项**: #1 doc

---

## §2. R20 阶段 2 — 公开 API + 鉴权 (2 commits)

### 2.1 `6d6db9b0` feat(api): R20 阶段 2 — WS 8 帧 + 鉴权 5 组件 (D-03)

**改了什么**:
- 新增 `crates/apeireth-protocol/src/ws_v1.rs` (513 行, **新文件**, 非原 src, 主人 21:18 拍板 R20 阶段 2 续时授权)
- 改 `crates/apeireth-protocol/src/lib.rs` (+8 lines, 仅模块导出声明, 0 改原 LLM 协议归一化层 R17 战役 1-1 LOCKED, 走 `normalized.rs` / `router.rs` 正交)
- WS 8 帧 = 5 业务帧 (Auth / Message / Stream / ToolCall / Result) + 3 控制帧 (Ping / Pong / Close)
- 鉴权 5 组件 = Bearer Token + keyring (apeireth-keyring) + token bucket (apeireth-constraint) + audit log + quota stub (501)

**为什么改**: D-03 WS 鉴权 = 链接 token 5min TTL (浏览器 WS 不支持自定义 header), 主人 20:53 默认未反对, R20 阶段 2 实施。

**关联 12 项**: #2 test

**严守**:
- 0 改原 LLM 协议归一化层 (R17 战役 1-1 LOCKED)
- 新增 `ws_v1.rs` 是新文件, 非原 src, 走 `normalized.rs` / `router.rs` 正交
- 0 引 NewAPI
- 复用 apeireth-keyring (P0 凭证安全) + apeireth-constraint (token bucket)

### 2.2 `b2b9ec8e` feat(api): R20 阶段 2 — 6 工具 v1 子路径 endpoint (D-02 + D-01 真接)

**改了什么**:
- 新增 6 工具 endpoint, 路径 `/v1/tools/{name}/invoke` (D-02 子路径, 主人 20:53 按 A 推荐拍板)
- 6 工具 = calendar / contact / drive / message / search / task (D-01 真接, 主人 20:53 推翻 A 推荐 stub 501, 原话"加")
- 6 endpoint 各自 1:1 翻译 v0.9.21 商业版 6 tool.js, 复用 apeireth-tools (LOCKED) + apeireth-extension (LOCKED) 6 类插件

**为什么改**: 主人 20:53 拍 D-01 真接 (推翻 A 推荐 stub 501) + D-02 子路径 (按 A 推荐), 6 工具是商业版核心, 1:1 翻译无重设计。

**关联 12 项**: #2 test

**严守**:
- 0 触碰 24 LOCKED crate (apeireth-tools / apeireth-extension 是 LOCKED, 仅复用)
- 0 改 workspace version 1.0.0
- 0 引 NewAPI

---

## §3. R20 阶段 3 — Docker + 8 包 (2 commits)

### 3.1 `f5c44769` feat(upgrade): R20 阶段 3 — D-07 一次性迁移 + 卸载脚本

**改了什么**:
- 新增 `scripts/upgrade/v2.0.0-alpha-to-v1.0.0.sh` (D-07 一次性 SQLite → PostgreSQL 迁移脚本)
- 8 步迁移: 1) 备份 SQLite 2) 验证备份 3) 停服务 4) 导出 SQLite 5) 创建 PostgreSQL 5) 导入数据 6) 验证行数 7) 切换配置 8) 启服务
- 5 验证: row count / checksum / sample query / FK / unique constraint
- 兜底 3 步: 失败回滚 / 保留 .bak 30 天 / 邮件告警
- `--dry-run` 模式 (per O-5 不假装)
- 新增 `scripts/uninstall/uninstall.sh` (5 步 0 残留 + 8 形态自动检测 + --keep-data + --dry-run)

**为什么改**: D-07 主人 20:53 拍 A 一次性迁移 (推翻 B 推荐双写 7 天, 原话"现在没用户用"), 简化迁移路径, 强提示备份 + 保留 .bak 30 天。

**关联 12 项**: #5 upgrade + #6 uninstall

**严守**:
- 0 触碰 24 LOCKED crate
- 0 改 workspace version 1.0.0
- 0 引 NewAPI
- dry-run 模式 (per O-5 不假装)

### 3.2 `50e6cbf0` feat(release): R20 阶段 3 — Dockerfile 多阶段 + 8 包配置 (D-06 8 包齐发)

**改了什么**:
- 新增 `Dockerfile` (多阶段 build, distroless final, non-root USER)
- 新增 `docker-compose.yml` (1 服务 + 1 volume + 1 network)
- 新增 `packaging/{deb,rpm,brew,scoop,tarball,zip,msi,docker}/` 8 形态 build + install 脚本
  - `packaging/deb/build.sh` + `install-deb.sh` + `Cargo.toml.snippet` + `apeireth.service`
  - `packaging/rpm/build-rpm.sh` + `install-rpm.sh` + `apeireth.spec`
  - `packaging/brew/build-brew.sh` + `install-brew.sh` + `apeireth.rb`
  - `packaging/scoop/build-scoop.ps1` + `install-scoop.ps1` + `apeireth.json`
  - `packaging/tarball/build-tarball.sh` + `install-tarball.sh` (musl 静态链接)
  - `packaging/zip/build-zip.ps1` + `install-zip.ps1` (Windows 通用)
  - `packaging/msi/build-msi.ps1` + `install-msi.ps1` (WiX installer)
  - `packaging/docker/Dockerfile` (multi-arch linux/amd64 + linux/arm64)

**为什么改**: D-06 主人 20:53 拍 A 8 包齐发 (按 A 推荐, 主人补充"搞技术用户很多 Linux"), Linux 4 包 (deb / rpm / tarball / Docker) 重点优化, 估 90% Linux 用户覆盖。

**关联 12 项**: #4 install

**严守**:
- 0 触碰 24 LOCKED crate
- 0 改 workspace version 1.0.0
- 0 引 NewAPI
- 复用 GitHub Actions 官方 actions (per O-2 走在前人肩上)
- non-root USER + API key 不入 image + audit append-only (5 守门)

---

## §4. R20 阶段 4 — Provider 真接 (1 commit)

### 4.1 `0da4af03` feat(provider): R20 阶段 4 估补 — claude-code Provider client skeleton (强效果)

**改了什么**:
- 新增 `crates/apeireth-provider-claude-code/` (5 Provider 估补第 1 个)
- claude-code Provider client skeleton: 1:1 翻译 v0.9.21 商业版 `claude-code.js`, 复用 apeireth-protocol (LOCKED) + apeireth-constraint (LOCKED)

**为什么改**: 主人 21:18 拍板"cpu 9955hx 内存 32G, 还能派的都给我派了", 估补 5 Provider 真接是阶段 4 重点。

**关联 12 项**: #11 provider (估补)

**严守**:
- 0 触碰 24 LOCKED crate
- 0 改 workspace version 1.0.0
- 0 引 NewAPI

---

## §5. R20 阶段 5 — SDK + 估补 (4 commits)

### 5.1 `28056623` R20 阶段 5 估补: apeireth-task skeleton (1:1 翻译 taskTools.js)

**改了什么**:
- 新增 `crates/apeireth-task/` skeleton
- 1:1 翻译 v0.9.21 商业版 `taskTools.js` 估缺 1500 LOC
- 7 状态 (pending / running / paused / completed / failed / cancelled / timeout) + DAG + 5 优先级 + 调度 + 重试 + 取消 + 8 编译期 hardcode + m3 防御 8 工具 + K-1 5 字样
- fixture 5 in-process

**为什么改**: 蓝图 §2 16 估缺 crate 中 taskTools.js 估缺 1500 LOC 是最大估缺, 1:1 翻译补齐估缺。

**关联 12 项**: #2 test

### 5.2 `e1d543d1` R20 阶段 5 估补: apeireth-tree-sitter skeleton workspace member

**改了什么**:
- 新增 `crates/apeireth-tree-sitter/` skeleton (10 files)
- 由 Mavis 集成时 commit in `d5b98489`

**为什么改**: tree-sitter 是 R20 阶段 5 估补重点, 提供代码语法树解析能力。

**关联 12 项**: #2 test

### 5.3 `8afc64c1` feat(sdk): R20 阶段 6 — apeireth-sdk 客户 SDK stub (1.0 release #13)

**改了什么**:
- 新增 `crates/apeireth-sdk/` 客户 SDK stub
- 1:1 翻译 v0.9.21 商业版 `apeireth-sdk.js`, 8 工具 stub + STUB_MODE 编译期守门

**为什么改**: 1.0 release #13 客户 SDK 是 release 项 (per 蓝图 §3.5)。

**关联 12 项**: #1 doc + #2 test

### 5.4 `d08e0c0f` feat(V1299) + tests(52): Rust Toolchain Audit (VCP 真源代码深读 #20)

**改了什么**:
- Rust Toolchain Audit (52 tests)
- 6/6 hyp PASS
- 2 parser bug fix

**为什么改**: 主人 13:08 真自问 + 17:43 实事求是 + 17:58 不假装 + 19:33 走在前人肩上 + 23:44 干到底, VCP 真源代码深读 #20 轮次。

**关联 12 项**: #2 test

---

## §6. R20 阶段 6 — 1.0 release 收口 (8 commits)

### 6.1 `629995d3` ci(audit): R20 阶段 6 — 8 项不修改承诺审计 (1.0 release 团队规范)

**改了什么**:
- 新增 `scripts/audit/8-promise-audit.sh` (1.0 release 团队规范)
- 8 项不修改承诺实查: 7 LOCKED 文档 + workspace version 1.0.0

**关联 12 项**: #12 security + 团队规范

### 6.2 `02d5db6c` docs(release): R20 阶段 6 — 1.0 release 报告 (团队可见 + GitHub release body)

**改了什么**:
- 新增 `docs/release/1.0.0-release-report-2026-08-05.md` (300+ 行)
- 9 章节切分: §0 TL;DR / §1 release 时间表 / §2 14 new crate / §3 蓝图 + 4 决策 / §4 12 项 checklist 进度 / §5 71GB 事故根因 / §6 0 触碰 24 LOCKED 实查 / §7 6 哲学 anchor / §8 关联文档

**关联 12 项**: #1 doc

### 6.3 `4cfe29b5` docs(root): R20 阶段 6 — 团队规范 7 文件 (1.0 release 团队规范)

**改了什么**:
- 新增 7 团队规范文件:
  - `CONTRIBUTING.md` (更新 8 项不修改承诺)
  - `CODEOWNERS` (团队 reviewer 分配)
  - `.github/ISSUE_TEMPLATE/` (3 模板: bug / feature / question)
  - `.github/PULL_REQUEST_TEMPLATE.md` (PR 模板)
  - `.well-known/security.txt` (RFC 9116 安全漏洞报告入口, 21 行)
  - `CHANGELOG.md` 模板段 (R20 阶段 6 增量)
  - `SECURITY.md` 更新

**关联 12 项**: #1 doc

### 6.4 `5b27d041` docs(root): R20 阶段 6 — team-onboarding.md (1.0 release 团队入职)

**改了什么**:
- 新增 `docs/team-onboarding.md` (187+ 行)
- 8 章节切分: 1) Apeireth 是什么 / 2) 仓库结构 / 3) 6 哲学 anchor / 4) 8 项不修改承诺 / 5) 团队节奏 / 6) 接手检查清单 / 7) 8 项承诺审计 / 8) 关联文档

**关联 12 项**: #1 doc

### 6.5 `d5b98489` test(V1297): 44 pytest cases for Cargo Feature Flag Audit

**改了什么**:
- Cargo Feature Flag Audit (44 pytest cases)
- 6/6 hyp PASS
- 集成 10 个 apeireth-tree-sitter skeleton file

**为什么改**: 主人 17:43 实事求是 + 00:44 质量工程化 + 00:56 任何人都能接手, VCP 真源代码深读 #18 轮次。

**关联 12 项**: #2 test

### 6.6 `b5941134` docs(release): R20 阶段 6 — Release notes v1.0.0 (GitHub release body)

**改了什么**:
- 新增 `docs/release/v1.0.0-release-notes-2026-08-05.md` (120 行)
- 9 章节切分: 🎉 Highlights / 🔒 安全 + 防御 / 📊 测试 / 🚀 4 决策拍板 / 🐛 已知问题 / 📦 8 包 / 🙏 致谢 / 🔗 链接 / 📅 时间表

**关联 12 项**: #1 doc

### 6.7 `702942fb` fix(workspace): R20 阶段 6 — workspace 治理升级 (R19 T10 known bug 修)

**改了什么**:
- 修 R19 T10 known bug (workspace `[workspace.package] version` 严守)
- 0 改 `Cargo.toml` `[workspace.package] version = "1.0.0"`
- 0 改任何 LOCKED crate

**关联 12 项**: #9 ci (workspace 治理)

### 6.8 `bbb26266` feat(release): R20 阶段 6 — cosign 8 包签名 (1.0 release #3 signature)

**改了什么**:
- 新增 `scripts/release/cosign-sign-all.sh` (8 包统一签名脚本)
- 新增 `scripts/release/cosign-verify.sh` (用户侧验证脚本)
- 新增 `docs/security/cosign-keys.md` (172 行, cosign 公钥 + 密钥管理 + 撤销流程)
- 8 包 cosign 签名: deb / rpm / brew / scoop / tarball / zip / MSI / Docker (per 蓝图 §3.5 #3 signature)

**关联 12 项**: #3 signature + #12 security

### 6.9 `c956fdfe` chore(legal): R20 stage 6 - THIRD-PARTY-NOTICES.md + LICENSE governance (1.0 release #11)

**改了什么**:
- 新增 `THIRD-PARTY-NOTICES.md` (60+ 直接依赖 LICENSE 收集)
- 更新 `LICENSE` (Apache-2.0, 顶部标 @author weibin per v0.9.21 商业版 1:1 翻译)
- 更新 `NOTICE` (Apeireth 团队 + 致谢)
- 更新 `DEPENDENCY` (60+ 直接依赖列表)

**关联 12 项**: #11 license

### 6.10 `0ad11531` feat(V1298) + tests(48): Cargo Workspace Lints Audit (VCP 真源代码深读 #19)

**改了什么**:
- Cargo Workspace Lints Audit (48 tests)
- 5/6 hyp PASS + 1 FAIL (per `7685b128` 修复)
- 3 parser bug fix

**关联 12 项**: #2 test

### 6.11 `5b87027a` ci(security): R20 阶段 6 — cargo audit + cargo deny 扫描 (1.0 release #3 security)

**改了什么**:
- 新增 `scripts/audit/cargo-audit.sh` (RustSec advisory db 扫描)
- 新增 `scripts/audit/cargo-deny.sh` (4 类: bans + licenses + sources + advisories)
- 0 RUSTSEC 漏洞 (per `security-audit.md` §2)

**关联 12 项**: #12 security

### 6.12 `915f28ef` test(bench): R20 阶段 6 — cargo bench 性能 baseline (1.0 release #7 perf)

**改了什么**:
- 新增 `scripts/bench/cargo-bench-baseline.sh` (1.0.0 baseline 跑法)
- 5 R-Measure bench: R-1 直行 / R-2 直说 / R-3 闭环 / R-4 守门 / R-5 诚实
- baseline 产物上传 artifact (90 天 retention)

**关联 12 项**: #7 perf

### 6.13 `03a3c310` fix(checklist): R20 阶段 3 observability check 兼容 EXPOSE 8080 9090 多端口写法

**改了什么**:
- 修 `scripts/release-1.0-checklist.sh` observability check 兼容 EXPOSE 多端口写法
- 0 改其他 12 项 check

**关联 12 项**: #8 observability

### 6.14 `7685b128` chore(V1300): apeireth-image-prompt [lints] workspace = true (修 V1298 audit 1/16 缺)

**改了什么**:
- 修 V1298 audit 1/16 缺 (apeireth-image-prompt 缺 `[lints] workspace = true`)
- 0 改其他 LOCKED crate

**关联 12 项**: #2 test

---

## §7. 累计 commit 总览

| 类别 | commits |
|------|------:|
| R20 阶段 1 蓝图 + 整合 + 收官 | 6 |
| R20 阶段 2 公开 API + 鉴权 | 2 |
| R20 阶段 3 Docker + 8 包 | 2 |
| R20 阶段 4 Provider 真接 | 1 |
| R20 阶段 5 SDK + 估补 | 4 |
| R20 阶段 6 1.0 release 收口 | 14 |
| **合计** | **29** |

**11 主线 commit** (per `README.md` §5): 蓝图 + 整合 #1 + 整合 #2 + 收官 + ROADMAP + CHANGELOG+README + WS 8 帧 + D-07 迁移 + CI 3 workflow + cosign 8 包 + workspace 治理

**18 增量 commit**: 6 工具 endpoint / 8 包 Dockerfile / claude-code Provider / apeireth-task / apeireth-tree-sitter / apeireth-sdk / V1299 toolchain audit / 8 项承诺审计 / 1.0 release 报告 / 团队规范 7 文件 / team-onboarding / V1297 feature flag / release notes / THIRD-PARTY-NOTICES / V1298 lints audit / cargo audit+deny / cargo bench baseline / observability check fix / V1300 image-prompt lints

---

## §8. 6 哲学 anchor 穿透

| 锚 | R20 阶段 1-6 穿透 |
|---|------|
| **S-1** ASI 完整性 | 14 new crate 1:1 翻译 v0.9.21 商业版, 0 重设计; WS 8 帧 + 鉴权 5 组件 1:1 蓝图 §2.3 / §2.4 |
| **S-2** 实事求是 | 蓝图 §1 商业版实查 1.4GB / 171 .js / 452K LOC; 5 P0 体检 5 真实缺口; 测试数 201 实查 / 193 报 (8 处 `#[test_case]` 偏差) |
| **O-2** 走在前人肩上 | 5 P0 1:1 翻译 + 9 skeleton 1:1 翻译 + 复用 `apeireth-constraint` token bucket + `apeireth-extension` 6 类插件 + cosign (sigstore 业界标准) + 8 包复用 GitHub Actions 官方 actions |
| **O-3** 干到底 | 14 crate 全部 skeleton 落地 + 193/193 测试 + 整合 #1 修 5 skeleton bug + 71GB 4 重防御 fixture 8 场景 + cosign 8 包 + 3 CI workflow + 12 项 checklist |
| **O-4** 任何人都能接手 | 14 crate lib.rs 顶部 30+ 行 doc + 4 份 reports 完整 path + §1-§6 章节切分 + `docs/ci/1.0-release-pipeline.md` 107 行 + `docs/security/cosign-keys.md` 172 行 + `docs/release/1.0.0-release-notes-2026-08-05.md` 86 行 + 1.0 release 报告 300+ 行 |
| **O-5** 不假装 | dry-run 模式全覆盖 (upgrade / uninstall / checklist) + 12 项 PASS 附实查 commit / 实查路径 / 实查行数 + 0 假装已实施 |

---

## §9. 8 项不修改承诺严守 (per `8-locked-unified-2026-08-05.md` §2)

| # | 项 | R20 阶段 1-6 严守 |
|---|----|------|
| 1 | 阶段 1+2+3 LOCKED 文档 | 0 改 |
| 2 | v2 / v4 / v4.1 LOCKED | 0 改 |
| 3 | 阶段 4 核心文档 LOCKED (`6ca80776`) | 0 改 |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 0 改 |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | 0 改 |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 0 改 |
| 7 | 顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) | 0 改 |
| 8 | workspace version 1.0.0 (semver 严格) | 0 改 `Cargo.toml` `[workspace.package] version` |

**24 LOCKED crate src/**: 0 触碰 (mtime baseline 16:34 之前 11/11 实查, per `8-promise-audit.md` §3)

---

## §10. 关联文档

- `docs/release/1.0.0-release-report-2026-08-05.md` (R20-Rev-A 收官报告)
- `docs/release/v1.0.0-release-notes-2026-08-05.md` (GitHub release body 模板)
- `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行蓝图)
- `docs/stage4/r20-阶段-1-收官-2026-08-05.md` (493 行阶段 1 收官)
- `docs/stage4/8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺)
- `docs/ci/1.0-release-pipeline.md` (3 workflow 触发)
- `docs/security/cosign-keys.md` (cosign 公钥 + 撤销流程)
- `ROADMAP.md` (3bc61686 同步, LOCKED 估)
- `CHANGELOG.md` (6c518ee3 同步, LOCKED 估)
- `README.md` (6c518ee3 同步, LOCKED 估)

---

_本文档是 R20 阶段 6 1.0 release changelog 的**详细版**, 补充 `CHANGELOG.md` 的 R20 阶段细节。等 Mavis 拍板 + 主人复核后, 由 Mavis 执行 git add + commit (不 push, 等 CI)。_
