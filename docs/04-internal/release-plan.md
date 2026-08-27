# Apeireth 发布路线（v2 工程重构线, 2026-08-27）

> **现状 (2026-08-27)**：本文**重写**为 v2 发布路线（v2.0.0-alpha.1 @ `d6910cf7` 已发；v1.0.0 已结案 `993e9107`）。取代 v1 era 发布规划；当前基线见根 [ARCHITECTURE.md](../../ARCHITECTURE.md)；v2 下一步见根 [ROADMAP.md](../../ROADMAP.md) §4。

```
[Document-Meta]
Document:        docs/04-internal/release-plan.md
Version:         Manual-Rev-N (v2 重写)
Last-Modified:   2026-08-27
Status:          🟢 活跃 (v2 发布路线)
```

---

## 一、设计层真实原意（保留 v1 内核）

> 来源 [docs/04-internal/design-intent.md](design-intent.md) §2 + [docs/01-architecture/philosophy.md](../01-architecture/philosophy.md)。**v2 不改设计内核**，只改工程形态。

- **基地 = 给 LLM 的「操作系统」**：提供(工具+记忆) / 约束(governance+self-disable) / 记录(session+trace) / 接入(provider 插件)
- **涌现优先于预定义**：`Plugin::tools()` + `Plugin::providers()` 暴露**可声明**能力 + runtime 拼装；不靠硬编码 capability
- **「AI 发现你想要什么」和「AI 长出它自己想要什么」是同一个过程**——governance hook 是决策闸，plugin 是能力提供器，二者正交
- **安全 = 能力限制 + governance 闸 + 主人批准 + 熔断**（不堆关键词规则）——v2 由 `GovernancePipeline` 短路第一个非 allow 实现
- **记录/连续性**：trace + SessionId 跨 turn 连续（不假装灵魂同一）

## 二、工程现状 vs 设计原意（v2 对账）

| 设计原意 | v2 工程现状 | 偏差 |
|---|---|---|
| 基地 = OS（不绑模型） | `ProviderCapability` trait + 3 canonical provider（minimax/anthropic/openai-compatible）插件化 | ✅ 无偏差 |
| 涌现优先（AI 自己长能力） | `PluginManifest::declare()` + `PluginManager::register()` + runtime agent loop 用 typed view `active_tools()/active_providers()` | ✅ 无偏差（plugin 拼装 + capability registry 唯一事实源已建立） |
| 治理 = 闸 + 批准 + 熔断 | `crates/foundation/governance` 提供完整 hook 实现（`AllowAll`/`DenyCapabilities`/`MaxRounds`/`PermissionGovernanceHook`/`PromptInjectionHook`/`CredentialDisclosureHook`/`AuditHashChain`） | ✅ **P0 已接线 (upstream `873d2857`)**：CLI bootstrap 装 `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`；MaxRounds 结构性、AuditHashChain 按部署需要 |
| 记录/连续性（continuity 锚点） | `SessionId` + `TraceId` + `crates/engine/runtime/src/canonical/session.rs` 会话生命周期 | ✅ 锚点建立，approval 可恢复 |
| 工具默认安全 | `BuiltinToolsPlugin::new()` 默认注册 filesystem/search/repo（3 只读）；shell/fetch opt-in | ✅ 无偏差 |
| 0 装 PASS | SDK 7 个 `unimplemented!()` 守门；governance 默认空 pipeline → `AllowAll`（**可见默认非不可见 Option**） | ✅ 0 装 PASS 严守 |
| 文档同步自觉 | 本批对账批：13-crate + 4-段 docs + CODEOWNERS v2 重写 | ✅ 已落实 |

## 三、发布形态与版本轴

来源：[ROADMAP.md](../../ROADMAP.md) §0 + [Cargo.toml](../../Cargo.toml) `[workspace.package]` + 根 [README.md](../../README.md)。

### 1. 双轴版本号（产品轴 vs workspace 轴）

| 轴 | 当前值 | 来源 | 备注 |
| | | | |
| **产品轴（git tag）** | `v2.0.0-alpha.1` → `d6910cf7` | `git tag` | 公开 semver，crate.io publish 跟这个 |
| **workspace 轴（`Cargo.toml [workspace.package] version`）** | `1.2.0` | `Cargo.toml:41` | 内部 crate 版本（跟产品轴独立，per R148 双轨制）|

### 2. tag 历史

| tag | sha | 状态 |
|---|---|---|
| `v1.0.0` | `993e9107` | 历史（v1 时代结案，旧线归档 `archive/v1.0-master`）|
| `v1.5.0` | `ce4892a0`（peeled）| 历史 |
| `v2.0-preview` | `b57eef13` | **已删除**（带 v1 Release #375135391 一起删，2026-08-27）|
| `v2.0.0-alpha.1` | `d6910cf7` | ✅ 当前（reconstruct_v2 工程重构首个 alpha） |

### 3. v2.0 发布路线（per [ROADMAP.md](../../ROADMAP.md) §4）

| 优先级 | 任务 | 状态 | 预计 tag |
|---|---|---|---|
| **P0** | 生产 governance 接线（`build_canonical_runtime_from_env` 装 `GovernancePipeline`）| ✅ done (upstream `873d2857`) | — |
| **P0** | 13 键 verdict cache 角色**降级完成**（哲学标准 + 5 原则洋葱映射，`RUNTIME_ENFORCED = false`） | — | — |
| **P1** | core crate 脊椎去留（onion/gate/lifecycle/philosophy/memory 5 legacy 模块）+ `apeireth-credentials` 接线 | ⏳ | `v2.0.0-beta.1` |
| **P3** | M1B 记忆全量移植（ACT-R + 完整管线） | ⏳ | `v2.0.0-beta.2` |
| **P4** | MCP 动态能力注册 | ⏳ | `v2.0.0-rc.1` |
| **P5** | ProcessSupervisor + 进程树隔离 | ⏳ | 同上 |
| **P6** | companion 器官移植（W1/W2/W3/E4/F4/F1/F6/E7）| ⏳ | `v2.0.0-rc.2` |
| **P7** | 连续感知（voice/screen）| ⏳ | `v2.0.0-rc.3` |
| **P8** | 前端产品化（companion-desktop ↔ gateway）| ⏳ | `v2.0.0` |
| — | `v2.0.0` 正式版 | 🎯 | 当 P0-P8 全绿 + governance 生产验证通过 |

## 四、v2.0.0-alpha.1 发布 checklist（实测）

来源：[Makefile](../../Makefile) + `.github/workflows/*.yml`。

### 已通过（CI 实测 2026-08-27）

- [x] **构建**：`cargo build --workspace --tests --locked` ✅
- [x] **测试**：`cargo test --workspace --tests --bins --lib --locked` → **1338 passed / 0 failed**
- [x] **lint**：`rust-lint.yml`（clippy 3 档 + fmt）✅
- [x] **fmt**：`cargo fmt --all --check` ✅
- [x] **audit**：`cargo-audit.yml`（RUSTSEC database）✅
- [x] **deny**：`cargo-deny.yml`（bans + licenses + sources + advisories）✅
- [x] **miri**：`miri.yml`（unsafe code 检查）✅
- [x] **rustdoc**：`rustdoc.yml`（nightly `-Dwarnings`）✅
- [x] **coverage**：`coverage.yml`（cargo tarpaulin）✅
- [x] **13 键测试契约**：`rust.yml` hard-walls job（`crates/foundation/core/tests/verdict_keys.rs` 等）
- [x] **生产 governance 接线**（upstream `873d2857`）：`build_canonical_runtime_from_env` 装 `GovernancePipeline(PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook)`
- [x] **敏感 workspace 路径保护**（upstream `ac5cbf5a`）：`tool.filesystem` + `tool.search` 通过 `crates/capabilities/tools/src/sensitive_path.rs` 自动屏蔽 `.env`/`.ssh`/`.aws`/`.gnupg`/`.secret`
- [x] **M2B / M2C / M3A 三 OS 隔离验证**：m2b-xv-isolation.yml + m2c-xv-shell-validation.yml + m3a-canonical-fetch.yml ✅
- [x] **protocol 集成测试** ✅
- [x] **dock**：`crates/` + `legacy/` 双线管理（legacy reference-only + workspace exclude）
- [x] **CI 防御**：`pii-leak-detection` + `release-prep` 保持 master-only，不在 main 跑（per `601e5c21`）|

### 待完成（P1 接班）

- [ ] **场景 D 长程 AI 判断架构评估**（新增 P-arch 任务）：见 [`docs/04-internal/scene-d-v2-plan.md`](scene-d-v2-plan.md)——F1/F6/E7/W6 移植评估 + 多 agent 互审协议设计（架构层，2-3 周工作量）
- [ ] **core crate 脊椎去留 + `apeireth-credentials` 接线**（P1）
- [ ] **World model W1/W2**（P6）：v1 发布前置清单遗留（文本层 + 因果图推演）——根 [ROADMAP.md](../../ROADMAP.md) §4 P6
- [x] ✅ **13 键 verdict cache 降级**（P0 → 完成）：`philosophy.rs::RUNTIME_ENFORCED = false`，`onion.rs::VERDICT_KEYS_BY_PRINCIPLE` 映射到 E/S/A/M 5 原则洋葱

## 五、配套发布产物（v2.0.0-alpha.1 已落地）

| 产物 | 位置 | 状态 |
|---|---|---|
| tag | `v2.0.0-alpha.1` → `d6910cf7` | ✅ 已推（Clash proxy + IP rewrite + Host 头）|
| 远端 | `https://github.com/Apeireth/apeireth-rust`（默认分支 `main`）| ✅ |
| 文档集 | 根 README + ROADMAP + CHANGELOG + 4 段 docs + ARCHITECTURE + 根 SECURITY/CONTRIBUTING/INSTALL/CODEOWNERS | ✅ 文档对账批 2026-08-27 完成 |
| SBOM | `cargo cyclonedx sbom`（per [Makefile](../../Makefile) `make sbom`）| ⏳ 跑（v2 release 触发）|
| cosign 签名 | per `apeireth-cosign` 镜像（post-1.0.0 已配置）| ✅ v2 沿用 |
| Docker 多架构 | `linux/amd64` + `linux/arm64`（per `Dockerfile`）| ✅（实测未跑，本机无 docker） |

## 六、本指南与 ROADMAP/ARCHITECTURE 关系

- **顶层路线**：[ROADMAP.md](../../ROADMAP.md) §4 P0-P8 + §5 硬墙
- **架构契约**：[ARCHITECTURE.md](../../ARCHITECTURE.md) 顶层 + [architecture.md](../01-architecture/architecture.md) 详细归属
- **发布机制**：本文（版本轴 + tag + checklist）
- **已发布历史**：`docs/archive/roadmap/`（v1.0-released-r128-r178-2026-08-18.md 等）

任何"v2 路线 / 优先级 / 何时发什么版本"的问题，**唯一权威 = 根 ROADMAP §4 + §5**；本文不重复。

---

_本文 v2 重写 (2026-08-27)：取代 v1 era 发布规划（B2 三档 feature、套件、crack 装配层等术语在 v2 已弃用）；v2 路线 = ROADMAP §4 P0-P8 + 本批文档对账。**R**靠 v1 时代的"套件 / 装配矩阵 / world model 发布前置"——这些都是 v1 路径，v2 走 PluginManager + 13-crate 单一事实源。_