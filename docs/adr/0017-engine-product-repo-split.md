# 0017. 引擎仓 / 产品仓分离 —— 双仓边界、护城河落在私有插件包、前端私有

- **Status**: **Accepted**（2026-10-31，owner 口述逐条裁决，见 §9 裁决记录）
- **Date**: 2026-10-31
- **Owners**: owner 逐条裁决（2026-10-31：「分仓」「私有插件包」「前端也私有」「图像一起发」「引擎也公开」「legacy 先不动但以后要处理」「包名占」「链接按建议」）；实施线 = 仓库治理线（总工：lead）
- **Supersedes**: —（与 [0015](0015-pure-rust-ui-engine.md) 并行不悖：0015 裁定 `crates-ui/` 独立 workspace，本 ADR 裁定它归**引擎仓**；与 [0016](0016-frontend-visual-rebuild.md) 并行：0016 裁定前端视觉重建路线，本 ADR 裁定前端**归产品仓**）
- **契约影响**: `docs/architecture/contract.toml` §[workspace]（新增双仓登记段）、§[layers]（**0 改动**）、§[deps]（**0 改动**——分仓不改主依赖图）、§[extension_points]（**0 改动**——私有插件包复用现有 4 个扩展点，不新增）
- **例外关联**: —
- **预算净额**: 依赖边 **±0**（主 workspace 111 条生产边 0 增 0 减）/ 开关 **±0** / 主 workspace crate **±0**（30 members 不变）/ 跨仓依赖边 **+23**（产品 → 引擎，改由 crates.io 版本依赖承载，非主图）/ 文件尺寸 **±0**（本批 0 行生产代码改动）

---

## 1. 背景与问题

### 1.1 现状实测（2026-10-31）

**仓库体量**（`git ls-files` + 逐文件字节求和）：

| 项 | 实测值 |
|---|---|
| 追踪文件 | **5,728 个 / 160.3 MB** |
| `.git` 体积 | **280 MB**（pack 181 MB）/ 3,389 commits |
| `legacy/` | **1,929 文件 / 21.3 MB**（34% 的文件，纯历史冻结归档） |
| `docs/` | 1,188 文件 / 13.9 MB（其中 `docs/archive` 684 文件） |
| `previews/` + `artifacts/` | **166 文件 / 61.1 MB**（38% 的字节，图片资产） |
| `crates/` + `crates-ui/` | 1,507 文件 / 26.5 MB |

**Rust 代码量分布**（`find src -name '*.rs' | xargs cat | wc -l`）：

| 层 | crate 数 | 行数 | 占比 |
|---|---:|---:|---:|
| `foundation/` | 6 | 83,142 | 16.8% |
| `engine/`（不含 image） | 7 | 166,031 | 33.6% |
| `engine/image/` | 11 | 118,734 | 24.1% |
| `capabilities/tools` | 1 | 81,527 | 16.5% |
| **引擎侧小计** | **25** | **449,434** | **91.1%** |
| `adapters/`（gateway/cli/sdk/tui） | 4 | 44,118 | 8.9% |

**前端**（产品面）：`frontend/companion-desktop` = `.svelte`+`.ts` **51,742 行** + 测试 **16,222 行** + `src-tauri` Rust **13,476 行**；`crates-ui/`（Rust 渲染引擎）**32,120 行**。

### 1.2 依赖图实测：切线已经是现成的

`cargo metadata --no-deps --format-version 1`（2026-10-31），主 workspace **111 条生产边**：

| 边方向 | 条数 | 判定 |
|---|---:|---|
| 引擎侧 → `adapters` | **0** | ✅ 无反向依赖，切线干净 |
| `adapters` → 引擎侧 | 23 | 单向（产品消费引擎） |
| `adapters` → `adapters` | 2 | `gateway→sdk`、`cli→gateway`，同仓内部，留在产品仓 |
| `crates-ui` → 主 workspace | **0** | ✅ 已是独立 workspace（ADR-0015） |

⇒ **"引擎层 / 产品层"这条切线不需要先做解耦工程**：依赖图上本来就没有反向边。

### 1.3 三个真问题

1. **产品策略可变，产品代码却和「已交付即冻结」的引擎家族同仓**。`image_family`（118,734 行）已冻结，`adapters`（44,118 行）+ 前端（81,440 行）却在高频改写——两种迭代节奏混在一个仓里。
2. **公开与私有的授权边界无处安放**。产品策略、前端形态、发行配置属私有资产；哲学锚 / 键集 / 3 不可变脊柱 / 架构契约属公开资产。同一仓无法表达这层边界。
3. **引擎尚未成为"可被消费的产物"**。crates.io 实测 `q=apeireth` **total = 0**（2026-10-31）⇒ 引擎 crate 一个都没发出去 ⇒ 产品即便分仓，也只能靠 path 依赖活着，**拿不到独立版本轴**。

### 1.4 同行做法实测（2026-10-31 查证）

| 案例 | 做法 | 关键事实 |
|---|---|---|
| **Kubernetes** | **发布单元分离，git 源仓合一** | `staging/README.md` 原文：staging 目录是 "packages that have been split to their own repository" 的暂存区，但 "The code in the staging/ directory is authoritative, i.e. the only copy of the code"；发布仓 "must have an initial empty commit"、PR **不直接受理**。35 个模块各有独立仓与版本轴，真源仍在一个仓。 |
| **oxc** | 引擎独立仓 + 双轨分发 | `oxc-project/oxc`，crates.io 上 `oxc` 212 版本（现 0.156.0）+ `oxc_parser_napi` 走 npm；产品（Biome）在另一仓，消费已发布产物。 |
| **Deno（deno_graph）** | 引擎 crate 独立仓 | `denoland/deno_graph`，crates.io 231 版本（现 0.111.1），GitHub OIDC trustpub 发布，**独立版本轴 0.x**。 |
| **Deno（deno_core）** | ⚠️ **拆出去后又搬回来** | crates.io 实测 `repository` 字段：0.386.0（2026-02-23）= `denoland/deno_core`（独立仓）→ **0.390.0（2026-03-05）= `denoland/deno`（搬回 monorepo）**，此后 0.413.0 仍指向 monorepo。 |
| **Moby / Android / HashiCorp** | 真拆仓 / 换协议 | 拆仓动机是**治理、授权、商业化**，**没有一家为"体积"分仓**；HashiCorp 则选择不拆仓、改用 BUSL 换协议。 |

**从中提炼的三条规律**：
1. **没有一家为体积分仓**——体积问题一律用归档仓 / LFS / Release 资产解决。
2. **分仓的正当理由只有三个**：授权控制、治理归属、发布节奏。
3. **拆出去容易，搬回来贵**（deno_core 实证）⇒ 边界要一次定义对，且**先让引擎具备独立版本轴，再动 git 边界**，否则会卡在"两个仓都不能独立发版"的中间态。

---

## 2. 决策

**拆两个仓：引擎仓（公开，crates.io 可消费）+ 产品仓（私有）。切线落在 `adapters` 层。护城河不放在 crate 边界上，放在**扩展点**上——产品专有能力写成私有仓里的插件包，通过引擎既有的 4 个扩展点注册进引擎，引擎 0 改动。前端应用私有，`crates-ui` 渲染引擎公开。**

### 2.1 双仓边界（硬约束，写入契约）

**引擎仓（公开）**

| 类别 | 内容 |
|---|---|
| crates | `crates/foundation/*`(6)、`crates/engine/*`(7)、`crates/engine/image/*`(11)、`crates/capabilities/tools`(1)、`crates-ui/*`(7)、`benches`(1) |
| 文档 | `docs/01-architecture`、`docs/02-guides`、`docs/03-reference`、`docs/04-internal`、`docs/adr`、`docs/architecture`、`docs/development`、`docs/security`、`docs/packaging` |
| 发行 | 根 `Cargo.toml`、`LICENSE`/`LICENSE-MIT`/`NOTICE`/`OSS_NOTICE.md`/`THIRD-PARTY-NOTICES.md`、`scripts/`、`.github/workflows/`（引擎侧） |
| LOCKED | **LOCKED 5 项全部随引擎仓**：9 哲学锚本体 / 14 键 / 3 不可变脊柱 / `workspace.version` / R11 baseline |

**产品仓（私有）**

| 类别 | 内容 |
|---|---|
| crates | `crates/adapters/{gateway,cli,tui,sdk}`(4)、**新增私有插件包**（§2.2） |
| 前端 | `frontend/companion-desktop`（应用 + Tauri 宿主）、`frontend/design-preview`（设计资产）、`frontend/perf-probe`（形象渲染性能探针） |
| 发行 | `deploy/`、`packaging/`、`examples/`、`library/` |
| 文档 | `docs/design`、`docs/vision`、`docs/beta`、`docs/integration`、`reports/` |
| 产品资产 | 产品策略、提示词、发行配置、路线图、定价（**当前 0 文件**，随产品生长） |

### 2.2 护城河形态：私有插件包（owner 2026-10-31 拍板）

**产品专有能力不 fork 引擎、不复制工具实现，而是写成产品仓里的一个普通 Rust crate，实现引擎既有的 4 个扩展点：**

| 扩展点 | 登记位置（契约 §[extension_points]） | 私有插件包放什么 | 实测公开性 |
|---|---|---|---|
| `Plugin` trait | `crates/foundation/plugin/src/plugin.rs` | 产品专有 provider + 工具（`ToolCapability`） | ✅ `pub trait Plugin` / `pub mod plugin; pub mod tool;` 已 re-export |
| `CapabilityRegistry::register` | `crates/engine/runtime/src/canonical/capability.rs` | 把产品工具注册进运行时能力表 | ✅ `pub fn register(...)` |
| `BehaviorModule` / `BehaviorRegistry` | `crates/engine/runtime/src/canonical/module.rs` | 产品级行为（提示词覆盖、hook、子循环） | ✅ `pub fn register(&mut self, module: Arc<dyn BehaviorModule>)` |
| `UiComponent` / `ComponentRegistry` | `crates-ui/apeireth-ui-core/src/component.rs`、`crates-ui/apeireth-ui-kit/src/registry.rs` | 产品私有前端组件 | ✅ 已登记（ADR-0015） |

**为什么这样最优**：
- **引擎 0 改动**——不新增扩展点、不新增 trait、不动契约棘轮；
- **插件是进程内静态插件**（`Plugin` trait 文档明载 dynamic library / WASM / hot reload / marketplace 均在范围外）⇒ 私有插件包就是产品仓里一个被编译进产品二进制的 crate，**没有动态加载带来的 ABI / 签名 / 安全面**；
- **能力面留在公开仓**（`capabilities/tools` 81,527 行通用工具库公开），**差异化留在私有仓**（产品专有工具 + 行为 + 组件）。

### 2.3 前端私有（owner 2026-10-31 拍板）

✅ **owner 2026-10-31 裁决确认**：`crates-ui`（Rust 渲染引擎）**归引擎仓（公开）**，前端应用与设计资产归产品仓（私有）。

| 资产 | 归属 | 理由 |
|---|---|---|
| `frontend/companion-desktop`（51,742 + 16,222 + 13,476 行） | **产品仓（私有）** | 产品形态、视觉方案、发行形态属产品资产；ADR-0016 的视觉重建路线也属产品线 |
| `frontend/design-preview` / `frontend/perf-probe` | **产品仓（私有）** | 设计概念资产与产品侧性能探针 |
| `crates-ui/*`（32,120 行，Rust 渲染引擎） | **引擎仓（公开）** | ① ADR-0015 已裁定"渲染引擎永远自己维护"，且已登记 `ui_component` 扩展点——**私有插件包要往里注册产品组件**，引擎必须公开可消费；② 公开它让公开仓的"全栈 Rust"叙事成立；③ 它与主 workspace **0 依赖边**，分仓零成本 |

### 2.4 版本轴（两轴并行）

| 轴 | 归属 | 规则 |
|---|---|---|
| 引擎 | 引擎仓根 `Cargo.toml` `[workspace.package].version` | **LOCKED 单轴**，现值 `2.0.0-rc.4`；仅 release authority 可改。crates.io 发布沿用此值 |
| 产品 | 产品仓根 `Cargo.toml` | **独立轴**，建议自 `0.1.0` 起，与引擎**不同步**（同行：oxc 0.x / deno_graph 0.x 独立轴） |
| 消费约束 | 产品仓 → 引擎 | **RC 期精确锁** `=2.0.0-rc.4`（半兼容版本不可用 `^`）；稳定后转 `^2.0` |

### 2.5 消费机制（分仓的"最后一公里"）

| 通道 | 用途 | 备注 |
|---|---|---|
| **crates.io**（主） | 引擎 crate 对外分发，产品仓按版本消费 | crates.io 收源码不收产物；`crates-ui` 的 wasm 产物另走 **npm**（同行：oxc 的 `oxc_parser_napi` 双轨） |
| **git tag 依赖**（过渡） | RC 期 crates.io 尚未铺开时的消费方式 | `git = "ssh://…", tag = "v2.0.0-rc.4"` |
| **git submodule**（可选） | 产品仓想同步看到引擎源码时 | 成本：submodule 指针更新要人工维护，不作默认方案 |

**首发顺序（依赖拓扑序，被依赖者先发）**：`apeireth-core` → `apeireth-protocol` → `apeireth-orchestration` → `apeireth-plugin` → `apeireth-governance` → `apeireth-credentials` → `apeireth-runtime` → `apeireth-storage` → `apeireth-provider` → `apeireth-memory` → `apeireth-organ` → `apeireth-guard` → `apeireth-tools-canonical` → `apeireth-gateway` / `cli` / `sdk` / `tui`（**产品 crate 不发 crates.io**，除非 owner 另裁）。`image_family` 维持 `publish = false` 不变。

### 2.6 门禁分离

| 仓 | 门禁 |
|---|---|
| 引擎仓 | 现有 31 个 workflow（`rust.yml` / `arch-guard.yml` / `image-family.yml` / `ui-engine-ci.yml` / `kani.yml` / `docs-truth.yml` …）**原样保留**；产品侧 workflow 迁出 |
| 产品仓 | 自带四关门禁（check / clippy -D warnings / fmt / test）+ `companion-desktop-ci.yml` + **新增一条：引擎依赖白名单守卫**（只允许依赖白名单内的引擎 crate，照抄 `scripts/arch_guard.py` 的做法） |

### 2.7 明确不改的（0 触碰）

- **LOCKED 5 项**：9 哲学锚本体 / 14 键 / 3 不可变脊柱 / `workspace.version` / R11 baseline——分仓**不改其内容**，只改其**所在仓**。
- **主 workspace 拓扑**：30 members / 29 product crates + 1 bench，**0 增 0 删**。
- **主依赖图**：111 条生产边 **0 增 0 减**。
- **扩展点清单**：**不新增**（私有插件包复用现有 4 个）。
- **契约棘轮**：`contract.toml` 只做"双仓登记"这一条**增量登记**，不动任何 accepted 条款。

---

## 3. O-6 三阶审查

### 3.1 总体最优

**问**：在"仓库太大 + 产品策略可能变"这条当前诉求里，分仓 + 先 publish 的顺序是不是最优切入点？

**答**：是——但**顺序必须倒过来：先让引擎可被消费，再拆 git 边界**。

**拒的 alternative A：直接 `git filter-repo` 拆仓（先拆再说）**
- 拒理由：拆完之后两仓**都还不能独立发版**（引擎没上 crates.io，产品仓只能靠 path 依赖指向一个不存在的引擎仓）⇒ 中间态比现状更糟：跨仓改动的原子性已经丢了，发版能力却还没拿到。**先 publish 再拆，中间每一态都是可用的**（Step 1-2 完成后，产品即便不拆仓也能独立发版）。

**拒的 alternative B：不分仓，只做 monorepo 内的发布/门禁分离（ADR-0015 的 crates-ui 模式）**
- 拒理由：crates-ui 模式解决了"迭代节奏"和"门禁口径"，**但解决不了授权边界**——产品策略、发行形态、定价、前端资产需要私有，而 monorepo 只有一个可见性开关。**owner 明确要"产品层私有" ⇒ 这条不成立**。
- 补充：这是本 ADR 唯一"必须拆"的硬约束；其余诉求（体积、节奏、门禁）都可用更便宜的手段解决。

**拒的 alternative C：为"体积"分仓**
- 拒理由：**没有一家同行这么做**（§1.4）。体积是 48% 的历史归档 + 38% 的图片字节（§1.1），用归档仓 + Release 资产解决，成本 1 天、风险 0，而分仓的成本是跨仓改动的原子性损失。**体积问题不该绑在商业边界决策上。**

**为什么本方案最优**：它把**两个不同性质的问题**（商业授权边界 / 仓库体积）拆开处理——分仓只解决前者（硬需求），后者用归档解决；并且用"先 publish"把跨仓改动的原子性损失控制在**最小窗口**（Step 1-2 完成前不动 git 边界）。

### 3.2 系统最优

**问**：在 `foundation → engine → capabilities → adapters` 的依赖图里，切线放哪一层最合适？

**答**：**切在 `adapters` 层**——实测该处已有 **0 条反向依赖**（§1.2），是全图唯一一个"切下去不用解耦"的位置。

**拒的 alternative A：切在 `capabilities` 层（tools 进产品仓）**
- 拒理由：① `capabilities` → `image_family` 有 1 条边（`tools→image-core`，契约已登记为 flagged edge）⇒ 切在 capabilities 会把 11 个 image crate（118,734 行）拖进产品仓，等于把**已交付即冻结的静态内核**私有化——既无商业价值，又损失公开仓的完整性；② 通用工具库（doc_ops / image_ops / filesystem / shell，81,527 行）公开反而是**生态资产**（同行：oxc 把通用能力全公开，让产品在其上竞争）。
- 补充：这正是 §2.2"通用能力公开、差异化留插件层"的技术依据。

**拒的 alternative B：切在 `foundation` 层**
- 拒理由：`foundation/core` 承载 **9 哲学锚 / 14 键 / 3 不可变脊柱**（LOCKED 4 项所在）⇒ 切在这里等于把 LOCKED 真源放进私有仓，公开仓连"为什么这么设计"都无法自证。**LOCKED 必须留在公开面。**

**拒的 alternative C：切在 `engine` 层内部（如 runtime 与 runtime-assembly 分家）**
- 拒理由：纯技术切分，**不解决任何真实诉求**（授权、节奏、体积），只增加跨仓边。`runtime-assembly` ↔ `runtime` ↔ `memory` ↔ `organ` 是运行时内核的紧密耦合体，拆开的收益为负。

**系统位置结论**：切线与现有层方向**完全一致**——adapters 本来就是"产品交付面"（gateway = HTTP/SSE 服务面，cli/tui = 人机界面，sdk = 嵌入面）。分仓不是新造边界，是**把既有的层边界提升为仓边界**。

### 3.3 架构最优

**问**：在单向依赖 + 扩展点约束下，公开接口形状与 crate 边界是否最优？

**答**：**护城河落在扩展点层，不落在 crate 边界层**——私有产品是一个"插件套件"，不是一个"引擎分叉"。

**拒的 alternative A：把产品专有工具直接写进产品仓的 `adapters` crate**
- 拒理由：产品仓的 adapter 是**交付面**（协议、进程、界面），把业务能力塞进去会让 adapter 同时承担"交付"与"能力"两种职责；且能力实现一旦分散在多个 adapter crate，就无法被 CLI / TUI / SDK **共享**⇒ 会出现三份实现漂移。能力必须住在**一个可被三个 adapter 共同依赖的 crate**——这就是私有插件包的形状。

**拒的 alternative B：把 `capabilities/tools` 整体私有**
- 拒理由：通用能力公开是生态资产（同 A）；且**差异化不会因此被保护**——真正属于产品的不是"有哪些工具"，而是"工具怎么组合、什么策略、什么界面"。护城河在**组合与策略层**，不在**工具清单层**。

**拒的 alternative C：私有插件包用动态库 / WASM 加载**
- 拒理由：`Plugin` trait 文档已明载本阶段 **仅支持静态进程内插件**，dynamic library / WASM / hot reload / marketplace **明确在范围外**。用动态加载会立刻撞上：ABI 版本协商、跨语言边界、签名与密钥边界、沙箱隔离——**四道新问题**，而静态插件包一道都没有。
- 因此私有插件包是**产品仓内一个被编译进产品二进制的普通 crate**，依赖引擎的 crates.io 版本依赖即可，**无 ABI 面**。

**公开接口形状**：`Plugin` / `ToolCapability` / `BehaviorModule` / `UiComponent` 四个 trait **全已 `pub` 且已登记**（§2.2 表）⇒ 私有仓实现它们**不需要引擎开任何新口子**。契约的"新增扩展点 = 结构变更，必须 ADR"纪律**0 触发**。

---

## 4. 迁移计划（分步、每步可独立验证、可独立回滚）

> **顺序纪律：Step 1-2 完成前，**不动 git 边界**（不建新仓、不删文件）。**

| 步 | 动作 | 验证 | 回滚点 |
|---|---|---|---|
| **S1** | 契约登记：在 `contract.toml` 新增双仓归属段（engine / product crate 清单 + 归属仓标记）；**0 改动其他条款** | `python scripts/arch_guard.py --strict` 0 违规 + 基线净额 ≤ 0 | 单 commit revert |
| **S2** | 引擎可发布化：crates.io 账号 + 占名（`apeireth-*` **现 total = 0**）+ 元数据补齐 + 按 §2.5 拓扑序首次 publish（先 `--dry-run`） | `cargo publish --dry-run` 全绿 + crates.io API 可见 + `cargo search apeireth` 命中 | yank 版本 / 不删 crate |
| **S3** | 私有插件包骨架（**产品仓尚未存在 ⇒ 此步只在引擎仓预留契约登记，不写实现**） | 契约登记项可被 arch_guard 读出 | 单 commit revert |
| **S4** | 建产品仓（`git filter-repo --path ...`，**保留历史**）+ 迁产品侧路径 | 产品仓 `cargo metadata` 成功 + 其 CI 四关绿 | 删仓，主仓 0 改动 |
| **S5** | 跨仓依赖改写：`src-tauri → apeireth-core`（path → crates.io 版本依赖）、`bench-harness → apeireth-gateway`（bench 归产品仓或拆用例）、`crates-ui` 走 npm/wasm 分发 | 两仓 CI 同时绿 + 产品仓白名单守卫绿 | 逐条 revert |
| **S6** | 治理分家：两仓各自 `AGENTS.md` / `claims.yaml` / `test-baseline.md`；引擎发版 → 自动开产品仓 bump PR | 两仓 `docs_truth.py --verify` 各自绿 | 单 commit revert |
| **S7** | 减重（**owner 2026-10-31 裁决：先不动，但列为必办债**）：`legacy/` + `docs/archive` → 归档仓；`previews/`+`artifacts` → Release 资产 | 追踪文件数 −49%、字节 −60% | `git revert` + 归档仓回填 |

⚠️ **S7 的 LOCKED 冲突**：`contract.toml` §[locked] `r11_baseline = "legacy/"` 是 LOCKED 4 项的引用点。**迁出 `legacy/` 须 owner 拍板并同步改该指向**（只引用、不复制历史；历史卷宗不追溯改写）。

⚠️ **S7 延期债登记（owner 2026-10-31 裁决「先不动，但以后也要处理」）**：
- **本批不动 `legacy/`**：`legacy/` 留在引擎仓，**LOCKED 4 项 0 触碰**，`contract.toml` §[locked] `r11_baseline` 指向**不变**。
- **必办债内容**：`legacy/` + `docs/archive`（2,613 文件 / 30.2 MB，占追踪文件 45.6%）迁独立归档仓；迁出时只改 §[locked] 的**指向**（引到归档仓），**不复制、不改写历史卷宗**。
- **触发条件（二者任一即启动）**：① 引擎仓首次正式 release（1.0）前；② 引擎仓追踪文件数 > 4,000 时（当前 5,728）。
- **归属**：仓库治理线；因触及 LOCKED 引用点，执行前需 owner 再签一次。

---

## 5. 回滚

| 步 | 回滚动作 | 回滚后状态 |
|---|---|---|
| S1 | `git revert` | 契约回到无双仓登记 |
| S2 | `cargo yank` 已发版本（**crates.io 版本不可删除，只能 yank**）；仓内 0 改动 | 引擎仓完全回到原状 |
| S3 | `git revert` | 0 残留 |
| S4 | 删产品仓（远端 + 本地副本） | 主仓完全回到原状（`filter-repo` 在副本上跑，主仓 0 改动） |
| S5 | 逐条 revert 依赖改写 commit | 两仓各自回到上一态 |
| S6 | revert 治理分家 commit | 同上 |
| S7 | `git revert` + 归档仓内容回填 | 追踪体积回到原状 |

**关键性质**：S1-S3 **不动 git 边界**；S4 起用 `git filter-repo` 在**副本**上执行并先做本地验证 ⇒ 主仓在 S4 之前始终可完整回滚。**没有一步是不可逆的**（唯一不可逆的是 crates.io 已发布版本，只能 yank 不能删——这是 S2 单独排期、且排在一切 git 操作之前的原因）。

---

## 6. 验收

- [ ] `python scripts/arch_guard.py --strict` 通过且**基线净额 ≤ 0**
- [ ] `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` 0 failed；`cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告；`cargo fmt --check` 0 diff
- [ ] 契约条款更新：§[workspace] 双仓登记（target → accepted），§[layers] / §[deps] / §[extension_points] **0 改动**
- [ ] 文档随行：`docs/03-reference/test-baseline.md`（测试数字单一真源，**不抄写**）、`docs/status/claims.yaml`（分仓后能力声明同步）
- [ ] LOCKED 5 项 **0 触碰**（9 哲学锚本体 / 14 键 / 3 不可变脊柱 / workspace.version / R11 baseline）
- [ ] **0 引新外部 dep**（分仓本身 0 新依赖）
- [ ] 私有插件包验证：4 个扩展点各 1 例真注册测试（`Plugin` / `CapabilityRegistry` / `BehaviorRegistry` / `ComponentRegistry`）
- [ ] 两仓 CI 各自四关门禁绿 + 产品仓引擎依赖白名单守卫绿

---

## 7. 后果（Consequences）

### 7.1 正面

- **商业边界落在代码结构上，不落在口头约定上**：私有资产 = 产品策略 + 前端 + 发行 + **私有插件包**；公开资产 = 运行时 + 图像内核 + 通用工具库 + **渲染引擎**。
- **护城河形态最轻**：私有插件包不需要 fork 引擎、不需要动态加载 ABI、不需要引擎开新扩展点 ⇒ 引擎 0 改动。
- **引擎获得独立版本轴与独立发布节奏**（同行 oxc / deno_graph 的形态），产品不必等引擎发版。
- **仓库体积问题被单独解决**（S7：文件 −49% / 字节 −60%），不与商业边界决策混在一起。
- **切线零解耦成本**：0 条反向依赖（实测）。

### 7.2 负面 / 接受的代价

| 代价 | 状态 |
|---|---|
| **跨仓改动失去原子性** | 接受。一次"引擎 + adapter"改动变成两个 PR + 一次引擎发版。**缓解**：S2 先 publish（产品从第一天就能独立发版）；RC 期用 git tag 依赖避免"等发版"的空窗；引擎发版自动开产品仓 bump PR |
| **两套治理体系** | 接受（同行 k8s 35 个仓各自带门禁）。`AGENTS.md` / `claims.yaml` / `test-baseline.md` 各一份，**测试数字真源按仓分离**，禁止跨仓抄写 |
| **crates.io 已发版本不可删** | 接受（只能 yank）。**缓解**：S2 排在一切 git 操作之前，且先 `--dry-run` |
| **产品仓需自建 CI 成本** | 接受。引擎仓 31 个 workflow 中的产品侧部分迁出，产品仓自带四关门禁 |
| **`crates-ui` 公开会暴露渲染实现** | 接受（owner 2026-10-31 裁决确认公开）。ADR-0015 已裁定永久自维护，且它无产品知识（业务语义明确在引擎之外） |
| **`legacy/` 迁出触碰 LOCKED 引用点** | **owner 2026-10-31 裁决：先不动，但列为必办债**（§4 S7 延期债登记：触发条件 = 1.0 release 前 或 追踪文件 > 4,000）。当前 `legacy/` 留引擎仓、LOCKED 指向不变 |

### 7.3 后续 stage

- **S2 之后**：引擎仓成为可被外部消费的公开产物 ⇒ 可评估是否对外开放 issue / PR 通道。
- **S5 之后**：私有插件包可承载第一批产品专有能力（具体能力清单另出 ADR）。
- **稳定版之后**：产品仓依赖从精确锁 `=2.0.0-rc.4` 转 `^2.0`；此时才引入"引擎 minor 升级需人工评审"的版本偏差策略。

---

## 8. 参考与出处

| 事实 | 出处 / 命令 |
|---|---|
| 仓库体量 | `git ls-files` + 逐文件字节求和（2026-10-31）：5,728 文件 / 160.3 MB；`.git` 280 MB；3,389 commits |
| legacy/docs/图片占比 | 同上，按顶层目录聚合 |
| Rust 行数分布 | `find <crate>/src -name '*.rs' \| xargs cat \| wc -l`（2026-10-31） |
| 前端体量 | `find frontend/companion-desktop/src -name '*.svelte' -o -name '*.ts'` + `src-tauri/src`（2026-10-31） |
| 依赖图 111 边 / 0 反向边 | `cargo metadata --no-deps --format-version 1`（2026-10-31），按 `contract.toml` §[layers] 分层聚合 |
| crates.io `apeireth*` 全空 | `https://crates.io/api/v1/crates?q=apeireth` → `total: 0`（2026-10-31） |
| deno_core 搬回 monorepo | `https://crates.io/api/v1/crates/deno_core/0.386.0`（repository=denoland/deno_core，2026-02-23）→ `/0.390.0`（repository=denoland/deno，2026-03-05）→ `/0.413.0`（同 monorepo） |
| deno_graph 独立仓 | `https://crates.io/api/v1/crates/deno_graph` → repository=denoland/deno_graph，231 版本，现 0.111.1 |
| oxc 引擎独立仓 + 双轨 | `https://crates.io/api/v1/crates?q=oxc` → `oxc` 212 版本 0.156.0 / `oxc_parser_napi` 137 版本，repository=oxc-project/oxc |
| Kubernetes staging 模式 | `https://raw.githubusercontent.com/kubernetes/kubernetes/master/staging/README.md`（官方 README 原文引用，2026-10-31 取） |
| 4 个扩展点公开性 | `crates/foundation/plugin/src/plugin.rs`（`pub trait Plugin`）、`crates/engine/runtime/src/canonical/capability.rs`（`pub fn register`）、`crates/engine/runtime/src/canonical/module.rs`（`pub fn register`）、`crates-ui/apeireth-ui-core/src/component.rs` + `crates-ui/apeireth-ui-kit/src/registry.rs`；契约登记见 `contract.toml` §[extension_points] |
| 插件形态边界（静态进程内） | `crates/foundation/plugin/src/plugin.rs`（`Plugin` trait 文档段："static, in-process plugins only … Dynamic library loading, WASM, hot reload, remote plugins, and a marketplace are explicitly out of scope"） |
| 契约与棘轮 | `docs/architecture/contract.toml`（§[locked] / §[layers] / §[deps] / §[extension_points]）、`docs/adr/README.md`（"只增不改"与 ADR 触发条件） |

---

## 附录 A：S1 契约登记草案（**尚未应用**，待本 ADR 转 Accepted 后落）

在 `docs/architecture/contract.toml` §[workspace] 之后新增以下段落，**其余条款 0 改动**：

```toml
# === 双仓归属（ADR-0017）===
# 引擎仓 = 公开仓（crates.io 可消费）；产品仓 = 私有仓（消费引擎的已发布版本）。
# 本段只做**归属登记**，不新增层、不新增依赖边、不新增扩展点（契约棘轮：允许集合只减不增）。
[[workspace.repos]]
id = "engine"
visibility = "public"
crates = [
  "apeireth-core", "apeireth-protocol", "apeireth-plugin", "apeireth-governance",
  "apeireth-credentials", "apeireth-orchestration",          # foundation
  "apeireth-runtime", "apeireth-runtime-assembly", "apeireth-provider",
  "apeireth-storage", "apeireth-memory", "apeireth-organ", "apeireth-guard",  # engine
  "apeireth-image-core", "apeireth-image-algo", "apeireth-image-cms",
  "apeireth-image-codecs", "apeireth-image-color", "apeireth-image-compose",
  "apeireth-image-doc", "apeireth-image-geom", "apeireth-image-psd",
  "apeireth-image-raster", "apeireth-image-vector",           # image_family
  "apeireth-tools-canonical",                                # capabilities
  "apeireth-ui", "apeireth-ui-core", "apeireth-ui-dom", "apeireth-ui-macro",
  "apeireth-ui-gpu", "apeireth-ui-kit",                        # ui_engine
  "apeireth-bench-harness",                                   # bench
]
publish = "crates.io"          # image_family 维持 publish = false（既有事实，不改）
note = "LOCKED 5 项全在此仓：9 哲学锚本体 / 14 键 / 3 不可变脊柱 / workspace.version / R11 baseline"

[[workspace.repos]]
id = "product"
visibility = "private"
crates = ["apeireth-gateway", "apeireth-cli", "apeireth-tui", "apeireth-sdk"]
paths = ["crates/adapters", "frontend", "deploy", "packaging", "examples", "library",
         "docs/design", "docs/vision", "docs/beta", "docs/integration", "reports"]
publish = "none"              # 产品 crate 不发 crates.io（除非 owner 另裁）
consumes = "engine"           # 通过 crates.io 版本依赖消费引擎（RC 期精确锁 =）
note = "私有护城河 = 本仓内的私有插件包（实现引擎既有 4 个扩展点，不 fork 引擎）"
```

**落地前须确认**：① `scripts/arch_guard.py` 是否需要读 `[[workspace.repos]]`（建议：只读不判，避免引入新的守卫维度）；② `[workspace] members = 30` 与 `product_crates = 29` 口径**不变**（分仓不改主 workspace 拓扑）。

---

## 附录 B：本批实测到的两处既有缺陷（**与本 ADR 无关，另案处理**）

| # | 缺陷 | 实测 | 建议 |
|---|---|---|---|
| B1 | **断链**：`docs/design/02-PARADIGM.md` 指向 `../adr/0018-frontend-visual-rebuild.md`（不存在；实际文件是 `0016-frontend-visual-rebuild.md`） | `python scripts/docs_links.py`（2026-10-31）：扫 1450 个 md，**断链 1**，已知缺失 44（政策保留） | ① 若确认前端视觉重建 ADR 就是 0016 ⇒ 改 PARADIGM 的两处引用（`:13` / `:227`）；② 若确有"重编号到 0018"的计划 ⇒ 先澄清编号意图再改。**本 ADR 取 0017 是按 `docs/adr/README.md`「四位递增编号」取下一个空号**；PARADIGM 期望 0018 说明**编号意图存在歧义**，请 owner 顺带裁定 |
| B1-bis | **⚠️ B1 不是一次性的：并行施工线会把它改回去** | 2026-10-31 实测：lead 改完 2 处（断链 → 0）后，**前端视觉重建线的并行改动**（该文件当轮 +179/−71）又把 `0018` 写回 2 处 ⇒ 断链复现 | 本 ADR 已再修 1 次（当前断链 0）。**根因是编号意图未澄清**：若前端视觉重建 ADR 计划重编号为 **0018**，则 **0017 大概率被预留给另一份 ADR**（如"新范式"决策）⇒ **本 ADR 的 0017 号存在撞车风险**，需 owner 裁定后统一改号（改号只动文件名 + README 索引 + 引用点，不动内容） |
| B2 | **开发依赖跨边**：`apeireth-gateway`（产品）dev-dep `apeireth-provider`（引擎） | `cargo metadata`（2026-10-31）：dev/build 跨边**仅 1 条** | 预期内（产品 dev 依赖引擎）；分仓后为跨仓 dev-dep，需在 S5 一并登记 |

---

## 附录 C：crates.io 发布就绪度实测（2026-10-31，静态审计 25 crate）

> 依据：`/tmp/apeireth-publish-audit.md`（705 行，静态审计；**未执行** `cargo build` / `cargo test` / `cargo publish`）。

### C.1 候选与占名

| 集合 | crate 数 | 说明 |
|---|---:|---|
| 根 workspace 候选 | **18** | 30 members − 11 个 `apeireth-image-*`（显式 `publish = false`）− 1 个 bench harness（publish = false） |
| 其中**产品 crate**（本 ADR 归产品仓，默认**不发布**） | 4 | `apeireth-gateway` / `cli` / `sdk` / `tui` |
| ⇒ **引擎仓发布集**（不含 image） | **14** | foundation 6 + engine 7 + tools-canonical 1 |
| crates-ui（独立 workspace） | 7 | 全候选；建议 `apeireth-ui-demo` 设 `publish = false` |
| image 家族（现 `publish = false`） | 11 | 仅在 C.3 选 b 时进入发布集 |

**占名实测**：上述 25 个名字在 crates.io **全部 HTTP 404 = 未被占用**（2026-10-31）⇒ 占名窗口开着。

### C.2 阻塞项（4 类硬 + 3 类次级）

| # | 阻塞 | 实测事实 | 修法 | 量级 |
|---|---|---|---|---|
| **B1** | **path 依赖缺 `version`** | `cargo package -p apeireth-protocol --no-verify` → exit 101 `all dependencies must have a version requirement specified when packaging`；**21/25 crate 直接失败** | 根 `[workspace.dependencies]` 写 `{ path = ..., version = "2.0.0-rc.4" }`，各 crate 改 `{ workspace = true }`（**修法已实测有效**） | 97 行 |
| **B2** | **tools-canonical → `publish = false` 的 image-core** | 实测 **optional 依赖也会被 cargo 解析向 registry ⇒ 改 optional 救不了**；连锁废掉 `runtime-assembly` / `gateway` / `cli` | **需 owner 拍板**：见 C.3 三选项 | 结构级 |
| **B3** | **发布顺序死锁环 `{memory, orchestration, plugin, provider}`** | 实测 **dev-dependency 也是发布顺序约束**（最小复现：仅 dev-dep 指向未上架 crate 时 `cargo package` 失败）。根因：`crates/foundation/orchestration/Cargo.toml:26-27` 的 2 条 dev-dep（`apeireth-plugin` + `apeireth-provider`）与 `plugin → orchestration` 成环 | 移除这 2 条 dev-dep（live E2E 测试迁产品仓或 CI 门控）⇒ 实测可解出完整 18 crate 顺序 | 2 行 + 测试迁移 |
| **B4** | **元数据缺口** | `license-file` **0/25**（各 crate 目录**无 LICENSE 正文**）；`repository` 缺 11/18；`homepage` **0/25**；`keywords` 缺 14/18；`categories` 缺 14/18；`readme` 缺 8/18 + 7/7 | 根 `[workspace.package]` 统一补 + 各 crate 继承；crate 目录放 LICENSE 副本 | 元数据批 |
| **B5** | **潜伏（未爆）** | 根 workspace `categories = ["ai", ...]`——**"ai" 不在 crates.io 实际 58 项允许列表**（应为 `artificial-intelligence`）；`keywords` **7 个 > 上限 5** | 改枚举值 + 砍到 ≤5 | 2 行 |
| **B6** | **registry 替换** | 本机 `~/.cargo/config.toml` 用 rsproxy 替换 crates-io ⇒ `cargo publish` 必带 `--registry crates-io`，否则报"像网络错"的错 | 命令行固定带 flag | 0 行 |
| **B7** | **crates-ui demo 包体** | `apeireth-ui-demo` 打包含 332 文件的 vite harness（node_modules 未被 git 跟踪故未爆） | 加 `exclude = ["harness/"]` 或直接 `publish = false` | 1 行 |
| **B8** | **大文件** | 9 个单文件 > 3000 行（最大 `crates/capabilities/tools/src/doc_ops.rs` **6150 行**） | **不阻塞发布**，只影响 review 体验 | 不修 |

⚠️ **一个容易误判的坑**：`cargo package --list` **25/25 exit 0**，但它**拦不住 B1** ⇒ **不能拿 `--list` 全绿当发布就绪证据**。

### C.3 B2 三选项（**需 owner 拍板**）

**实测事实链**：`tools-canonical` →（非 optional 生产依赖）→ `apeireth-image-core`（我们的门面，381 行）→ **10 个 vendored crate**（全 `publish = false`）。⇒ 发 tools-canonical 必须连整个 image 家族一起发。

**关键许可事实**：vendored 家族许可 = **MIT OR Apache-2.0**（上游双许可任选，与本仓 `Apache-2.0 OR MIT` 同集合），**版权头原样保留**（实测各 crate `src/lib.rs` 头部：`Copyright (c) 2026 ArtCraft Team and the PhotoCraft contributors` + `SPDX-License-Identifier: MIT OR Apache-2.0` + "Original license terms unchanged"），许可关 **PASS**（[docs/04-internal/license-port/gate1-license.md](docs/04-internal/license-port/gate1-license.md)），NOTICE 已落档（`crates/engine/image/NOTICE.md` + `OSS_NOTICE.md` §12）。⇒ **法律上允许在 crates.io 再分发**（宽松协议；义务 = 保留版权头 + NOTICE + 声明改动，**均已做**）。

| 选项 | 做法 | 公开引擎得到什么 | 代价 |
|---|---|---|---|
| **b（推荐）** | **发布 image 家族 11 crate** | ✅ **带完整创作能力面**（编解码/滤镜/几何/PSD）——第三方最想要的部分 | 11 crate 元数据补齐 + license-port 余下三关复检 + cargo-about 重生成；等于把 R4a-2「image 独立 workspace 抽取」落地 |
| **a（保守）** | 拆边：`tools-canonical` 拆成「通用工具（可发布）」+「创作工具（不可发布）」 | ⚠️ 公开引擎**缺创作能力**；contract 的 `[[flagged_edges]]`（`tools → image-core`）需正式裁决 | 结构改动 + 公开引擎能力面缩水 |
| **c（不可取）** | `tools-canonical` 也 `publish = false` | ❌ 公开引擎**无工具层** | 与"引擎可被消费"目标直接冲突 |

**本 ADR 起草口径：选 b**（理由：宽松协议已合规 + 公开引擎需要内容 + 契约早已把 image 家族的独立化列为既定目标）。

✅ **owner 2026-10-31 裁决确认：选 b**——image 家族 11 crate **随引擎一起发 crates.io**。⇒ 引擎仓发布集 = **14（根）+ 11（image）+ 7（crates-ui）= 32 crate**（`apeireth-ui-demo` 建议 `publish = false`）。

### C.4 首发拓扑序 + 三段门禁

**顺序**（B3 修完后实测拓扑序；同层任意）：`core → governance → orchestration → protocol → guard → plugin → credentials → sdk → storage → memory → provider → organ → runtime → **tools-canonical（B2 未解则此处起全卡）** → gateway → runtime-assembly → cli → tui`；crates-ui：`ui-core → ui-dom / ui-gpu → ui-macro → ui-kit → ui → ui-demo`。

**三段门禁**（**不可跳段**）：

| 段 | 命令 | 拦什么 | 网络 |
|---|---|---|---|
| G1 | `cargo package -p <c> --list` | 仅打包内容清单 | 离线 |
| **G2** | `cargo package -p <c> --no-verify` | **B1 + B2 + B3** | 离线 |
| **G3** | `cargo publish -p <c> --dry-run --registry crates-io` | 含 build verify | **需网络** |

⚠️ **不能一次性把 25 个 dry-run 全跑完再统一发**：G2/G3 会向 crates.io 解析内部依赖 ⇒ **必须按拓扑序逐个「发一个 → 下一个」**。（实测 `publish --dry-run -p apeireth-core -p apeireth-storage --registry crates-io` **exit 0**。）

### C.5 crates-ui（wasm）双轨

| 轨 | 内容 | 要点 |
|---|---|---|
| **A 轨**（crates.io 源码） | 6 个库 crate：`ui-core` / `ui-macro` / `ui-dom` / `ui-gpu` / `ui-kit` / `ui` | 补 17 行 path version + 继承 `repository`/`homepage`/`keywords.workspace`（4 个 ≤5 ✅）/`categories.workspace`（`gui` ✅）+ 建 README + 放 LICENSE |
| **B 轨**（npm 产物） | `@apeireth/ui`（wasm32 产物） | 现成起点 = `apeireth-ui-demo/harness/` 的 vite 工程；**`wasm-bindgen-cli` 必须与 manifest 硬钉的 `=0.2.129` 同版本**（否则产物 ABI 不匹配） |

**实测**：wasm32 依赖是 target-specific ⇒ native 编译不受影响、crates.io 不会因 wasm32 失败被拒；但**须自建 wasm32 CI 门禁**（**未实测**）。

### C.6 认证方式（诚实标注）

- crates.io 生产**已确认存在** trusted publishing（summary API 有 `trustpub_only` 字段，多个主流 crate 为 true）。
- **但 cargo 1.97.1 内建 provider 只有 `cargo:token` 与 `cargo:paseto`，无 `cargo:oidc`**（strings 实测）。
- **是否 GA / 确切配置文本未查证**（crates.io 官方文档页为 JS 渲染，`web_fetch` 只取回标题）⇒ **不臆测**。
- **建议**：首发用 **token**（流程最短）；1.0 后 CI 转 trustpub，**开工前先核对官方文档**。

---

## 9. 裁决记录（owner 2026-10-31，逐条）

| # | 议题 | 裁决 | 落点 |
|---|---|---|---|
| 1 | **image 家族是否随引擎一起发 crates.io** | ✅ **一起发**（选项 b） | §2.2 / 附录 C.3；引擎仓发布集 = **32 crate**（14 根 + 11 image + 7 ui） |
| 2 | **`crates-ui`（Rust 渲染引擎）公开 or 私有** | ✅ **公开**（归引擎仓）；前端应用与设计资产私有 | §2.3 |
| 3 | **`legacy/` 迁不迁** | ⏸️ **先不动**，但**列为必办债**（触发条件：1.0 release 前 / 追踪文件 > 4,000） | §4 S7 延期债登记 |
| 4 | **crates.io 包名占不占** | ✅ **占**（按拓扑序边发边占） | §2.5 / 附录 C.1 |
| 5 | **PARADIGM 断链 + ADR 编号歧义** | ✅ **按建议**：链接指到 `0016-frontend-visual-rebuild.md`；0017 归本 ADR | 附录 B（已单独修） |

**同时确认的两个设计选择**（owner 2026-10-31 口述）：**护城河 = 私有插件包**（复用引擎既有 4 个扩展点，不 fork 引擎）；**产品层私有**。

### 9.1 「发到 crates.io」的语义（owner 2026-10-31 问，已答）

- **crates.io = Rust 生态的官方包仓库**（对标 npm / PyPI），托管**源码包**。发上去之后，别人在自己的 `Cargo.toml` 写 `apeireth-core = "2.0.0-rc.4"`，`cargo build` 即自动下载编译，**不需要本机有你的文件夹**。
- **它是分仓的前置**：产品仓若仍用 `path` 依赖，就必须把引擎源码整个塞进产品仓（等于没分仓）；只有走 crates.io 版本依赖（或 git tag），产品仓才能真正"只拿版本号、不拿源码"。
- **附带价值**：占名（`apeireth-*` 25 个名字实测全空）+ 公开引擎可被任何人发现与使用。
- **两条硬事实**：① **单向门**——已发布版本**不可删除**，只能 yank（故必须按拓扑序逐个发、每个先 dry-run）；② **需 crates.io 账号 + token**，发包凭 token 操作（1.0 后可转 trustpub，见附录 C.6）。
- crates.io **只收源码、不收编译产物**（wasm 另走 npm，见附录 C.5）；新包有人工审核。

### 9.2 开工前置（owner 侧 1 件事 + 实施线 4 步）

**owner 侧（1 件）**：注册 crates.io 账号（组织 `Apeireth`）→ 创建 API token → 通过安全渠道把 token 交给施工线（或自行按 §9.3 清单执行发布）。

**实施线（4 步，顺序不可乱）**：
1. **S1** 契约登记（§4 S1）⇒ 验证 `arch_guard --strict` 0 违规 + 基线净额 ≤ 0。
2. **S2a** 修 B1（97 行 path→version）+ B3（2 条 dev-dep）+ B4/B5/B7 元数据 ⇒ 验证 G2 `cargo package --no-verify` 全绿。
3. **S2b** 按附录 C.4 拓扑序**逐个** publish（每步 G3 dry-run → 真发 → crates.io API 可见）。
4. **S3** 私有插件包契约登记预留。

### 9.3 逐个发布的操作形态（占名与发布同一动作）

```bash
# 执行器已备好（拓扑序现算 / 三段门禁 / dirty tree 默认拒绝 / --registry crates-io 固定带）
./scripts/publish-engine-crates.sh --print-order    # 只看拓扑序（离线）
./scripts/publish-engine-crates.sh --check           # 离线彩排自检（不联网, 不发布）
./scripts/publish-engine-crates.sh                   # 真发（需 CARGO_REGISTRY_TOKEN）
```

---

## 10. 实施记录（S2a，2026-10-31）

### 10.1 已完成

| 项 | 结果 | 验证 |
|---|---|---|
| **S1 契约登记** | `contract.toml` 新增 `[[workspace.repos]]` 双仓段（engine 33 crate / product 4 crate） | `arch_guard --strict` **0 违规**；111 边 / 30 members / `product_crates 29` / `bench 1` 口径**未变** ⇒ 基线净额 ≤ 0 |
| **B1 path 依赖缺 version** | 根 `[workspace.dependencies]` 声明 **29 条** `{ path, version = "2.0.0-rc.4" }`，各 crate 改 `{ workspace = true }` | 原报错 `all dependencies must have a version requirement specified when packaging` **100% 消失**（25/25 crate） |
| **B2 image 家族随引擎发布** | 11 个 `apeireth-image-*` 由 `publish = false` 改 **`publish = true`** | 编排顺序：image 家族全部排在 `tools-canonical` **之前** |
| **B3 发布顺序死锁环** | `orchestration` 的 2 条 dev-dep（plugin/provider）拆除；`council_live.rs`（`#[ignore]` 手动 live E2E）迁至 `crates/adapters/cli/tests/`（产品侧、不发布 ⇒ 无顺序约束） | 改前有环 `plugin→orchestration→plugin`；**改后无环**，25 crate 合法拓扑序可排；`cargo test -p apeireth-orchestration` **503 passed / 0 failed**（改动前同为 503 passed / 0 failed ⇒ **通过数 0 退**；那 1 个 ignored 随文件迁到 cli 包） |
| **B4/B5/B7 元数据** | 各 crate 补 `repository`/`homepage`/`keywords`/`categories`/`readme`；根 `keywords` 砍到 **5 个**（原 7 个超限）、`categories` 的 `"ai"` 改为合法枚举 `"artificial-intelligence"`；`apeireth-ui-demo` 加 `exclude = ["harness/"]` 且 `publish = false` | 见 §10.2 彩排 |
| **S2a 离线彩排** | 执行器 `--check` 模式：**31/31 crate 全绿**（25 根 workspace + 6 crates-ui 库 crate） | 彩排用**命令行临时 patch**（cargo `--config patch.crates-io.<name>.path=…`），**不落盘、不进版本库** |
| **M1 跨仓断链** | `crates-ui/apeireth-ui-demo/src/settings_wizard.rs` 原用 `include_str!` 编译期嵌入**产品前端源文件**（`FirstRunWizard.svelte` / `onboarding.ts`）⇒ 已逐字复制到 `crates-ui/apeireth-ui-demo/fixtures/` 并改指仓内副本 | `diff` 双向**空** + sha256 一致；`crates-ui/**` 内**已无** `../../../frontend/…` 编译期引用 |
| **demo 源码包自足** | `exclude = ["harness/"]` 曾导致包内缺 `harness/settings/fixtures/{theme,wizard}.json` ⇒ 已把两个 fixture **逐字复制**到 `fixtures/` 并改 `include_str!` 指向 | `cargo package --list` 显示 4 个 fixture 都在包内；带 verify 的打包**不再报缺文件**（只余"依赖未上架"这堵预期墙） |
| **文档债（A 线登记）** | 9 个既有 crate README：① `protocol/README.md:3` 的「字段级**借鉴**开源 …真代码」违反 AGENTS.md §2 中性表述 ⇒ 改为「字段级**自研实现**，对齐业界通行 wire format 语义」；② 9 处「Apeireth **1.0**」旧版本 ⇒ 「Apeireth v2」；③ 4 处抄写测试数字违反 AGENTS.md §3 单一真源 ⇒ 改为指向 `docs/03-reference/test-baseline.md`；④ `core/README.md` 引用的 `ALL_EIGHT_ANCHORS` **在本仓不存在** ⇒ 更正为真实符号 `ALL_NINE_ANCHORS: [PhilosophicalAnchor8; 9]`（@ `eight_anchors.rs:179`），并更正「8 哲学锚」为 **9 锚**（enum 名为历史沿用） | 复扫：借鉴/移植 **0 命中**、`Apeireth 1.0` **0 命中**、不存在的符号 **0 命中**、抄写测试数字 **0 命中**（仅剩指向真源的指针） |

### 10.2 两条实施裁决（lead 决，owner 2026-10-31 授权范围内）

| # | 议题 | 裁决 | 理由 |
|---|---|---|---|
| **R1** | 施工线提出在 `crates-ui/.cargo/config.toml` 落 `[patch.crates-io]`（发布前彩排用） | ❌ **不入仓**（该文件已删除） | ① 它让发布门禁**假通过**——真实条件"依赖已在注册表"根本没被验证，与 0 装纪律冲突；② 仓内配置只在 cwd∈`crates-ui/` 时生效 ⇒ 行为不一致（隐性坑）；③ 生命周期是"发完就删"的临时脚手架。**替代**：彩排能力改由执行器的 `--check` 模式以**命令行临时 patch** 提供（不落盘） |
| **R2** | `exclude = ["harness/"]` 与 `include_str!` 夹击导致 demo 源码包不自足 | ✅ **复制 2 个 fixture 到 `fixtures/` + 改 2 处 `include_str!`**（harness 原件不动） | 保住"包体小"（harness 743 文件 / 19 MB 不进包）与"包自足"两件事；代价是 fixture 成**快照**（harness 侧改动需手动同步）——已在代码注释中如实登记 |

### 10.3 已知临时回归（诚实登记）

| # | 回归 | 影响 | 承接方 |
|---|---|---|---|
| G1 | **「生产一改 ⇒ 断言立刻红」的生产漂移检测暂失效**（M1 的直接副作用：`FirstRunWizard.svelte` / `onboarding.ts` 改为仓内快照副本） | crates-ui 的向导断言不再自动跟随产品前端改动 | **产品仓建立后由产品仓承接该断言**（S5） |
| G2 | `fixtures/{theme,wizard}.json` 成快照（harness 侧改动需手动同步） | demo 的参考侧一致性需人工同步 | 同上 |

### 10.4 S2b 发布完成记录（2026-10-31 实测）

**26 / 26 全部上架，版本统一 `2.0.0-rc.4`**（crates.io API 逐个核验 200）：

- 引擎 24 个：`apeireth-core` / `-protocol` / `-plugin` / `-governance` / `-credentials` / `-orchestration` / `-runtime` / `-runtime-assembly` / `-provider` / `-storage` / `-memory` / `-organ` / `-guard` / `-tools-canonical` + image 家族 11 个（`-image-core/algo/cms/codecs/color/compose/doc/geom/psd/raster/vector`）
- **门面 1 个**：`apeireth`（裸名，ADR-0019）——**旗舰名归本项目**
- **下游消费实测通过**：空工程 `cargo add apeireth` → `cargo build`（25.35s，exit 0）→ `cargo run` 打印 `facade version = 2.0.0-rc.4`，四个默认模块解析成功

**执行中实测到的 crates.io 行为（如实登记）**：
1. **新 crate 速率限制远严于预期**：实际约 **1 个 / 9 分钟**（不是"10 分钟 5 个"），全程撞 429 多次；报错会给出精确解除时刻（`try again after <UTC 时间>`），按该时刻退避才能过。
2. **上传端点会瞬时返回 408**（3.6MiB 的 `tools-canonical` 撞过一次）——**这不等于限速**，需与 429 区分重试策略。
3. **`/api/v1/me` 只能网页访问**（返回 "this action can only be performed on the crates.io website"），故 **token 无法用 API 预验**；真正的验证就是首个 `cargo publish`（失败无副作用）。
4. `cargo add <crate>` 在"只有预发布版本"时**能自动选中**并写 `=2.0.0-rc.4` 等价约束（实测）。

**crates-ui 6 个库 crate（2026-11-01 已发布）**：`ui` / `ui-core` / `ui-dom` / `ui-macro` / `ui-gpu` / `ui-kit` —— **全部 `2.0.0-rc.5` 上架**（crates.io API 逐个核验 200）。

⚠️ **发布内容 = HEAD 已提交版本，非工作树在途版本**（关键处置）：
- 工作区有前端线 **38 个在途 `src/*.rs` 改动**（未提交、未评审）⇒ 直接发布会把在途代码永久上架；
- 处置：用 **`git worktree add --detach <tmp> HEAD`** 建干净副本，**只搬 manifest + README，src 0 改动** ⇒ 发布内容 = 已评审的 HEAD 代码；
- 核实：HEAD 的 6 个库 crate **无跨仓 `frontend/` 引用**（仅 `harness/**`，而 `apeireth-ui-demo` 是 `publish = false`）⇒ M1 修复**不需移植**（M1 是前端线新建 `settings_theme.rs` / `settings_wizard.rs` 时引入的，HEAD 里不存在）；
- 下游消费实测：空工程 `cargo add apeireth-ui` → `cargo build` 23.40s **exit 0**（114 包）；`cargo add apeireth-ui-core` 单独可用（2 包）；
- 发布后 worktree 已 `git worktree remove` 清理。

⇒ **crates.io 现有 32 个 `apeireth-*` crate，全部 `2.0.0-rc.5`**（26 主仓 + 6 crates-ui）。

**⭐ 限速实测补充（2026-11-01）**：crates-ui 6 个是**新 crate 首发**，但**限速命中 0 次**（2026-10-31 首发那批触发的是「短时间内新建 crate 数」窗口，跨日已释放）。⇒ 限速是**滚动窗口**而非每日配额。

### 10.4.1 发布可见性守卫（2026-11-02 落地）

**起因**：owner 追问「哪些发哪些不发」时实测发现——4 个产品 adapter（`gateway` / `cli` / `sdk` / `tui`）的 `publish` **未设** ⇒ cargo 默认 true ⇒ 它们一直是「可发布」状态。契约 §[[workspace.repos]] 早已写 `product` 段 `publish = "none"`，**但代码没落实** ⇒ 任意一次 `cargo publish --workspace` 都会把私有产品交付面**永久公开上架**（crates.io 版本不可删，只能 yank）。

**两层处置**：
1. **补齐代码**：4 个 adapter 补 `publish = false`（commit `02200c93`）；实测 `cargo publish -p apeireth-gateway --dry-run` 被 cargo 拒绝（exit 101）。
2. **补齐守卫**（本条）：`scripts/arch_guard.py` 新增第 10 项检查——**任何 `publish != false` 的 crate，必须登记在契约的可发布清单里**（范围含独立 workspace `crates-ui`；反向亦查：清单里登记了却不存在的 crate 也报）。⇒ 以后新增 crate 漏设 `publish = false`，门禁直接红，**不靠人记得**。

**守卫有效性已实测**（注入反例法）：把 `apeireth-tui` 的 `publish = false` 注释掉 ⇒ `arch_guard --strict` 报 `VIOLATIONS: 1`（`[W10] publish: publish apeireth-tui — crate 可发布（publish != false）但未登记在契约可发布清单`）+ exit 1；还原 ⇒ `VIOLATIONS: 0` + exit 0。

**当前口径**：crates.io 实际可发 **32 个**（全部 `2.0.0-rc.5`）；明确不发 **6 个**（4 adapter 私有产品面 + `ui-demo` 试点应用 + `bench-harness` bench-only；另有 2 个 fuzz crate 在独立 workspace，`publish = false`）。

### 10.5 下一步

1. **owner**：注册 crates.io 账号（组织 `Apeireth`）→ 创建 API token → 交给施工线（手册 [docs/04-internal/engine-publish-runbook.md](../04-internal/engine-publish-runbook.md) §2）。
2. **实施线**：真发（按 §附录 C.4 拓扑序逐个）⇒ crates.io API 可见 ⇒ **下游消费实测**（空目录 `cargo add apeireth-core@2.0.0-rc.4` 真下载编译）⇒ **revoke 一次性 token** ⇒ 补 crates.io 链接进公开仓 README（owner 面版）。

---

_本文档由仓库治理线（总工：lead）撰写于 2026-10-31，起草于 owner 口述拍板（「私有插件包」「前端也私有」「分仓」），并于同日经 owner **逐条裁决**（§9）转 **Accepted**。裁决前不动 git 边界（§4 顺序纪律）；裁决后按 §9.2 的 4 步执行，S1/S2a 已完成（§10）。_
