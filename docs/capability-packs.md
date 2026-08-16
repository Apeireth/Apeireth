# Apeireth 能力包说明（capability-packs, 2026-08-16）

> **给谁看**: 想按需装配 Apeireth 的用户/部署者。回答「三档怎么装、装与不装差在哪」。
> **权威来源**: workspace 根 `suites.toml`（三件套装配清单，B2）+ `crates/apeireth-cli/Cargo.toml [features]` + 编译验证矩阵 `scripts/check-assembly-matrix.ps1`（日志 `logs/assembly-matrix.log`）。
> **0 假装**: 每档标注 `gated`（编译期真可裁）或 `declarative`（仅语义标记，未门控如实标注）——不假装所有包都能裁。

---

## 1. 三档装配模型（team-work-doc §1.3）

| 层 | 谁开发 | 改动方式 | 装配方式 |
|---|---|---|---|
| 模块（lib 核心） | 官方 | 整体大改，编译期强绑定 | 无条件编译 |
| 套件（suite） | 官方 | 拼积木：插件组 + 权限包 + 校验 | 编译期标记 + 运行时 `SuiteCatalog` 装配 |
| 插件（plugin） | 社区为主 | 最小单元热插拔 | 见 [plugin-authoring-guide.md](plugin-authoring-guide.md) |

**装配入口统一是 `apeireth-cli`**（suites.toml 头部）：

```powershell
cargo build -p apeireth-cli --no-default-features --features <组合>
```

`default = ["base"]`——默认装出来就是基地本体。

## 2. 档 1：base（基地本体，80% 必要能力）

- **feature**: `base`（语义标记；核心 crate 是无条件依赖，`--no-default-features` 时显式 opt-out）
- **crate 组**（suites.toml `[base]`）：core / memory / companion / api / protocol / asi / cognition / action / cli / tools / mcp / skills / council / config + security 系（sovereignty / constraint / guard）
- **不可裁项**（原文 note）：生命周期（涌现/做梦/反思/多轮/续传/宪法评审/权限包）+ 审计链（audit_log）属本体。
- **能力**：主动陪伴 + 记忆 + 基础工具 + 安全（release-plan §三.1）。

```powershell
cargo build -p apeireth-cli   # = --features base (default)
```

## 3. 档 2：capability packs（扩展能力包，80% → 完全体）

用户自选，默认不装（保持本体轻量安全）。**真实门控状态**（suites.toml `[packs.*]` + cli features 注释，逐项如实）：

| 包 | feature | 转发 | 状态 | 说明 |
|---|---|---|---|---|
| 本地智能包 | `local-intel` | `apeireth-memory/onnx` | ✅ **gated**（真 cfg 门控） | tract-onnx 本地嵌入；关闭 `semantic` default feature 后纯件（EmbedFn/HashEmbedder 等）仍可用，`--no-default-features` 可编译（矩阵 case 5/7 验证，suites.toml note） |
| GUI 包 | `gui` | `apeireth-api/tui-dashboard` | ✅ **gated**（真 cfg 门控） | ratatui TUI 仪表盘；apeireth-web/ssr（Web 面板）为独立 crate 默认开启，未纳入 cli 转发链 |
| 沙盒能力包 | `sandbox` | — | 🟡 **declarative**（未门控） | Layer 2 物理隔离（exec_worker per-call 子进程 + Job Object）**运行时已生效**；Sandboxie/landlock 参数口是 B3 范围 |
| 多通道包 | `channels` | — | 🟡 **declarative**（未门控） | lark/livekit/voice 是独立 workspace crate，装配 = 选择是否部署，非编译裁剪 |
| 审计能力包 | `audit` | — | 🟡 **declarative**（未门控） | audit_log 已在核心主线（memory action_stream append-only），属 base 不可裁 |

```powershell
cargo build -p apeireth-cli --features local-intel,gui   # 真裁剪示例
```

> **0 装 PASS 提醒**：勾了 `sandbox`/`channels`/`audit` feature 不会多编译任何东西——它们是语义标记；对应能力要么已在 base 生效（audit/沙盒运行时层），要么是部署选择（channels）。

## 4. 档 3：upgrade suites（升级套件，专业团队能力）

**编译期是标记，运行时才装配**：`SuiteCatalog::install_with_plugins`（`suites.rs:164-203`）校验组成插件已装 + 工具已注册 + 登记权限包。

| 套件 | feature | suite_id | 组成插件 | crate 组 | 真内容 |
|---|---|---|---|---|---|
| 教育升级套件 | `suite-education` | `education-suite` | `education-dx-check` | apeireth-skills | dx_check 规则层检查器（忘换 dx/混用/缺微分/残留 x/根号模式） |
| 渗透测试升级套件 | `suite-pentest` | `pentest-suite` | `pentest-recon` / `pentest-scan` | apeireth-tool-shell / apeireth-tool-fetch / apeireth-skills | recon_plan（E-1 范围闸）+ scan_report（nmap 解析）；宪法边界内 |
| 预测机核心升级套件 | `suite-oracle` | `oracle-suite` | （无插件） | apeireth-skills | simulate 情景推演 + forecast 可证伪预测 + Brier 校准 + 期望决策（`docs/oracle-suite-design.md`） |

**运行时装配序列**（真代码语义，来源：`examples/education_suite_demo.rs`）：

1. 装插件：`PluginRegistry.install(&bridge, Arc::new(<Plugin>))`（注册工具 + 授权）
2. 装套件：`SuiteCatalog::builtin().install_with_plugins(&bridge, Some(&reg), "education-suite")`（校验 + 登记权限包）
3. 桥执行工具：`bridge.execute_if_allowed(&call).await`

演示入口：`cargo run -p apeireth-companion --example education_suite_demo` / `pentest_suite_demo` / `oracle_acceptance`。

```powershell
cargo build -p apeireth-cli --features suite-education,suite-oracle   # 编译期标记
```

## 5. 装配矩阵验证

```powershell
pwsh scripts/check-assembly-matrix.ps1   # 逐组合编译验证 → logs/assembly-matrix.log
```

矩阵结论（suites.toml note + release-plan §四）：local-intel / gui 真门控验证通过；case 5 = 已知欠账如实标注；sandbox/channels/audit 为 declarative。

## 6. 相关文档

- 插件层开发（社区细件）：[plugin-authoring-guide.md](plugin-authoring-guide.md)
- 用户视角机制：[user-manual.md](user-manual.md) · 跑起来：[quick-start.md](quick-start.md)
- 发布规划与设计偏差审视：[release-plan.md](release-plan.md) §三/§四
