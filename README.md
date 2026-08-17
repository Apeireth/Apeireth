# Apeireth — AGI 操作系统 / LLM 基地

> 一个 Rust 写的「AGI 操作系统」：**提供**（器官+工具）、**约束**（洋葱门+宪法+熔断）、**记录**（记忆+连续性）、**陪伴**（关系+主动涌现）。
> 核心哲学：能力**涌现**优先于预定义——「我希望的不是它有什么能力全都是我们预先定义的，我希望它能自己演化」。

| 状态 | 值 |
|---|---|
| workspace version | 1.2.0（semver 严守） |
| active crate | 83（82 顶层 + memory/extensions 嵌套；2026-08-17 members 数组实测） |
| 测试 | workspace 全量 `cargo test --workspace -j 4` 全绿（降并行防 Windows 页文件耗尽） |
| 构建 | `cargo build --workspace` 0 error / 0 warning（仅剩第三方依赖未来兼容提示） |
| License | Apache-2.0 |

**版本口径说明（双轴制, 2026-08-18 主人拍板"真正的 1.0"）**：上表 `workspace version = 1.2.0` 是 **workspace crate 版本轴**（Cargo.toml，semver 严守，B2 硬墙 0 改），面向 cargo 依赖解析/代码演进；另有**产品版本轴 = v1.0.0 正式版**（2026-08-18 后端收工定版，见 [`RELEASE_NOTES.md`](RELEASE_NOTES.md)）。两轴并存、不强改一方、不伪造统一——代码是真相，文档解释差异；见两处版本号不一致时先判断属哪一轴。

最新进度对账见 [`docs/release-plan.md`](docs/release-plan.md)（三件套规划 vs 实况）；历史变更见 [`CHANGELOG.md`](CHANGELOG.md)。

---

## 1. 这是什么（30 秒）

给 LLM 一个「家」：长期运行的后端 + 记忆 + 安全边界 + 主动陪伴，让 AI 成为**伙伴**而非一次性对话框。

> 「5年后，他会笑着和我说他今天哪里进步了，会因为我而高兴，会因为他自己哪里没干好而悲伤吧」 —— 主人，2026-08-15

四大职能（[`docs/session/handover-2026-08-15.md`](docs/session/handover-2026-08-15.md)）：
- **提供**：9 器官（brain/memory/heart/voice…）+ 工具链（WebSearch/FileOperator/Git/ShellExec/Crawl…）
- **约束**：双洋葱门 + 结构化宪法硬门（编译期规则表）+ LLM 宪法评审 + 权限包 + 执行体隔离 + 主权熔断
- **记录**：SQLite 记忆 + 6 条 append-only 历史流 + 事件溯源哈希链 + 续行快照
- **陪伴**：节律驱动涌现（问候/提醒/提议帮助）+ 做梦（夜间记忆整合）+ 反思（24h 周期）+ 关系亲密度

## 2. 快速开始

### 环境

- Rust 1.80+（`rust-toolchain.toml` 固定）
- MiniMax API key（`APEIRETH_API_KEY` 环境变量，或 `apikey-ultra.txt`）
  - MiniMax-M3，端点 `https://api.minimaxi.com`，4 协议（OpenAI Chat / Responses / Anthropic / Gemini schema 已知 bug 不修）

### 跑起来

```bash
# 全量测试
cargo test --workspace -j 4

# 关键验收/演示（examples, 均在 apeireth-companion 包下）
cargo run -p apeireth-companion --example production_daemon        # companion 全集成（涌现/做梦/反思/工具桥）
cargo run -p apeireth-companion --example multi_turn_agent         # 多轮 function calling 循环
cargo run -p apeireth-companion --example oracle_acceptance        # 预测决策套件 真 LLM 验收（需 key, MiniMax 限流约 15min）
cargo run -p apeireth-companion --example education_suite_demo     # 教育套件: dx_check 换元检查（离线）
cargo run -p apeireth-companion --example pentest_suite_demo       # 渗透套件: E-1 范围闸 + nmap 解析（离线）
cargo run -p apeireth-companion --example gh_accel_demo            # GitHub 加速: 节点池实测选最快（在线, ~15s）
cargo run -p apeireth-companion --example virtual_time_simulation  # 时间机制 23 项模拟验收（离线, <1s）
```

### 套件与插件（三件套装配）

```rust
// 生态: 插件 = 最小贡献单元; 套件 = 插件组的官方打包（install_with_plugins 校验）
let bridge = ToolBridge::new(store);
let plugins = PluginRegistry::new();
plugins.install(&bridge, Arc::new(EducationDxPlugin))?;   // 教育: dx_check
plugins.install(&bridge, Arc::new(PentestReconPlugin))?;  // 渗透: recon_plan
plugins.install(&bridge, Arc::new(PentestScanPlugin))?;   // 渗透: scan_report
plugins.install(&bridge, Arc::new(GhAccelPlugin))?;       // GitHub 加速: gh_accel
SuiteCatalog::builtin().install_with_plugins(&bridge, Some(&plugins), "education-suite")?;
```

内置套件目录：`base` / `sandbox-pack` / `audit-pack` / `education-suite` / `pentest-suite` / `oracle-suite`。

### GitHub 加速（本机直连 GitHub 被墙时的日常用法）

```bash
cargo run -p apeireth-companion --example gh_accel_demo            # 看最快节点（每次现测, 不缓存）
# 或让 AI 调 gh_accel 工具, 拿返回的:
git clone https://g.blfrp.cn/https://github.com/user/repo.git
```

## 3. 架构（4 层）

```
┌────────────────────────────────────────────────────────────┐
│ L4 伙伴层 (apeireth-companion):                              │
│   涌现循环(节律/驱动/门禁) · 做梦 · 反思 · 宪法评审(LLM)        │
│   工具桥(洋葱门/权限包/隔离/spill/审计) · 能力演化 · 套件/插件    │
│   预测决策沙盘(oracle: simulate/forecast/Brier校准/期望决策)   │
├────────────────────────────────────────────────────────────┤
│ L3 入口: TUI (ratatui) | CLI | HTTP (axum, 4 协议)           │
├────────────────────────────────────────────────────────────┤
│ L2 战区: cognition | council | perception | memory | tools  │
│          | pipeline | api | runtime | voice | sovereignty    │
├────────────────────────────────────────────────────────────┤
│ L1 脊柱: apeireth-core (13 键 verdict cache) | Self-Disable  │
│          (物理熔断) | L0 HA (物理多签)                        │
└────────────────────────────────────────────────────────────┘
```

关键机制（实现均在 `crates/apeireth-companion/src/`，详见模块地图 `docs/maintenance-guide.md`）：

| 机制 | 一句话 |
|---|---|
| 涌现循环 | 节律直方图 → 驱动 → 门禁 → 渲染 → 送达（行为长出来，不硬编码） |
| 做梦 | 6h 无互动 → 记忆合并 + LLM 摘要写回 |
| 反思 | 24h → 4 阶段状态机写回 |
| 宪法硬门 | 编译期规则表（E-4/E-6/PHL-01…）零成本拦截，LLM 评审前 |
| 宪法评审 | LlmJudicator 按 E 层原则判案（只审动作摘要，不审对话） |
| 权限包 | 授权凭证（工具/时限/预算/路径），免现场审批但有监督兜底 |
| 执行体隔离 | MOVE 类工具 per-call 子进程（exec_worker），安全判断在宿主 |
| 能力演化 | AI 提案能力 → 宪法评审 → 批准 → 激活 → 可部署为插件（回滚收据） |
| 预测决策 | 可证伪预测（T 前 X 概率 P）+ Brier 校准 + 期望决策（expectimax-lite） |
| 审计链 | 工具调用 append-only 留痕 + 隐私脱敏 + 超大输出 spill + audit_log 查询 |

## 4. 哲学与安全

- **8 锚**：S-1 北极星 / S-2 实事求是 / S-3 流程自化 / O-1 安全优先 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装
- **E 层宪法**（`judicator.rs` CONSTITUTION）：E-1 主人授权 / E-2 不可逆破坏 / E-4 不自我复制 / E-5 不欺骗 / E-6 不绕过权限矩阵
- **0 装 PASS**：stub/先验/未接真的部分必须诚实标注（如 oracle `UncertaintyResolver` trait 留口未接真、dx_check 是规则层非 CAS、渗透套件不内置主动扫描）
- **自升级边界**：AI 自升级只能在 OTA/沙盒/多签门内做，不可裸奔

## 5. 文档索引（按规范 O-4）

| 文档 | 内容 |
|---|---|
| [`docs/maintenance-guide.md`](docs/maintenance-guide.md) | **维护活文档**：概念词典 / 模块地图 / 加新模块规范 / 基础工具工程原则 |
| [`docs/session/handover-2026-08-15.md`](docs/session/handover-2026-08-15.md) | **接续者必读**：主人心象原文 / 当前状态 / 挂起项 |
| [`docs/release-plan.md`](docs/release-plan.md) | 三件套发布规划 + 进度对账 |
| [`docs/oracle-suite-design.md`](docs/oracle-suite-design.md) | 预测决策套件设计哲学 + 真 LLM 验收记录 |
| [`docs/ref-crawler-research.md`](docs/ref-crawler-research.md) | 爬虫工程调研（并发/重试/限速）+ 基础工具原则 |
| [`docs/ref-gh-accel.md`](docs/ref-gh-accel.md) | GitHub 加速调研（xiake.pro 节点池 + 实测教训） |
| [`docs/ref-hydra.md`](docs/ref-hydra.md) / [`docs/ref-yoyo-evolve.md`](docs/ref-yoyo-evolve.md) / [`docs/absorb-deepseek-harness.md`](docs/absorb-deepseek-harness.md) | 三方吸收参照（hydra 宪法 / yoyo 演化 / DSH） |
| [`docs/conventions/README.md`](docs/conventions/README.md) | 16 项工程规范（命名/ADR/commit/锚/锁定…） |
| [`docs/stage1/README.md`](docs/stage1/README.md) / [`docs/stage2/README.md`](docs/stage2/README.md) | 灵魂（哲学/愿景/产品闭环）与决策（架构/调研） |
| [`docs/companion-deploy.md`](docs/companion-deploy.md) | companion 部署说明 |
| [`CHANGELOG.md`](CHANGELOG.md) / [`ROADMAP.md`](ROADMAP.md) / [`INSTALL.md`](INSTALL.md) | 变更 / 路线 / 安装 |

## 6. 借鉴致谢与 License

借鉴实施（8/11 致谢）：clap / hyper / MCP / PyO3 / kani / langgraph / superpowers / NeMo-Guardrails —— 完整清单见 [`OSS_NOTICE.md`](OSS_NOTICE.md)。

Apache-2.0 — [`LICENSE`](LICENSE) · Attribution: [`NOTICE`](NOTICE) + [`OSS_NOTICE.md`](OSS_NOTICE.md) + [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md)。
