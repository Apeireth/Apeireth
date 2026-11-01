# 0020. 门面默认面瘦身 —— `tools` 由默认改可选，默认依赖树 252 → 92 包

- **Status**: **Accepted**（2026-10-31 owner 裁决「B」：做门面瘦身；本 ADR 与实现同批落地）
- **Date**: 2026-10-31
- **Owners**: owner 裁决（2026-10-31 口述「B」）；实施线 = 仓库治理线（总工：lead）
- **Supersedes**: [0019](0019-apeireth-facade-crate.md)（**仅 §2.1 的默认导出集合**：4 个默认模块 → 3 个默认模块 + 4 个可选模块；其余条款全部保留）
- **契约影响**:
  - `contract.toml` §[workspace]：**0 改动**（crate 数、members、layers、budgets 全不变）
  - `contract.toml` §[deps]：生产边 **±0**（`tools-canonical` 由默认依赖改可选依赖，**边仍计入** ⇒ 仍为 118，与 `max_production_edges` 上限齐平，未超）
  - `contract.toml` §[[workspace.repos]]：**0 改动**
- **例外关联**: —
- **预算净额**: crate **±0** / 生产边 **±0** / 允许层边 **±0** / 开关 **±0** / 文件尺寸 **±0**（门面 lib.rs 约 ±10 行）

---

## 1. 背景与问题

### 1.1 问题来源

[ADR-0019](0019-apeireth-facade-crate.md) §7.3 登记的待办：门面默认依赖 252 个包过重，需收窄。本 ADR 落地该待办。

### 1.2 实测数据（2026-10-31，空工程 `cargo add` + `cargo generate-lockfile`）

| 依赖组合 | `Cargo.lock` 包数 | 增量 |
|---|---:|---|
| `apeireth-core` | **67** | — |
| `apeireth-core` + `apeireth-protocol` | **69** | +2 |
| `apeireth-core` + `apeireth-protocol` + `apeireth-runtime` | **91** | +22 |
| ↑ + `apeireth-tools-canonical` | **251** | **+160** ⬅️ |
| 门面 rc.4 实测（默认 4 模块） | **252** | — |

**结论（根因定位）**：`tools-canonical` 的传递依赖带进 `apeireth-image-core` → image 家族 **11 个 crate** 及其编解码 / 色彩 / 几何 / 排版依赖链（ADR-0017 §2.2 B2 裁决「image 随引擎一起发布」引入的这条链）。**这 160 个包与"工具能力"本身无关，是图像内核的重量。**

⚠️ **feature 门控对默认模块的传递依赖无效**——这正是 ADR-0019 §7.3 那条建议的实测依据。

---

## 2. 决策

**把 `tools` 由默认导出改为 feature 可选；默认面 = `core` + `protocol` + `runtime`（实测 **92 包**，252 → 92，减 63.5%）。**

### 2.1 导出集合（取代 ADR-0019 §2.1）

| 类别 | 模块 | 数量 |
|---|---|---:|
| **默认**（`default = []`） | `core` / `protocol` / `runtime` | 3 |
| **可选**（feature 门控） | `memory` / `orchestration` / `guard` / **`tools`** | 4 |
| 不进门面 | `organ` / `image-core` | — |

**依赖边总数不变（7 条 ⇒ 生产边仍 118）**：`tools-canonical` 由"默认依赖"变"可选依赖"，**边本身不消失**（实测 optional 依赖同样计入 `cargo metadata` 生产边）⇒ **契约预算零改动、未超上限**。

### 2.2 用户视角

```toml
# 默认：92 个包 —— 核心运行时（91 依赖 + 1 门面本体）
apeireth = "2.0.0-rc.5"

# 需要工具层：+160 个包（图像内核依赖链）
apeireth = { version = "2.0.0-rc.5", features = ["tools"] }
```

### 2.3 ⚠️ 版本连锁与 LOCKED 冲突（**需 owner 就发布方案拍板**）

**实测事实链**：

1. 门面 **rc.4 已发布**（crates.io 可见）⇒ 内容变更**必须发新版本**（crates.io 版本不可覆盖）。
2. 门面 `[package] version.workspace = true` ⇒ 门面版本**跟随根 `workspace.version`** ⇒ 要发 rc.5 **必须 bump 根 `Cargo.toml` 的 `[workspace.package].version`**。
3. **根 `workspace.version` 是 LOCKED 5 项之一**（`AGENTS.md` §4：「workspace.version 仅 release authority 可改」；契约 §[locked] `workspace_version_source = "Cargo.toml"`）。
4. bump 到 rc.5 后，**门面的 7 个依赖全部变 rc.5**；而 `cargo package` 会把 path 依赖改写成 registry 依赖去解析 ⇒ **这 7 个都必须已存在 registry**（实测 optional 依赖同样被解析）⇒ **rc.5 最小发布集 = 8 个**（门面 + 7 依赖）。
5. crates.io 新 crate 限速实测约 **1 个 / 9 分钟** ⇒ 8 个约需 **70 分钟**。

**两个方案**：

| | **方案 A：bump workspace.version（保持单轴制）** | **方案 B：门面独立版本轴（不碰 LOCKED）** |
|---|---|---|
| 做法 | 根 `version` → `2.0.0-rc.5`；门面与 7 依赖同步 rc.5 | 门面 `[package] version = "2.0.0-rc.5"`（**自声明覆盖**），7 依赖显式锁 `version = "2.0.0-rc.4"` |
| LOCKED | ⚠️ **碰 `workspace.version`**（需 release authority 授权） | ✅ 0 触碰 |
| 单轴制 | ✅ 保持（contract §[meta].hard_walls B2） | ❌ 门面破例（需登记） |
| 要发几个 crate | **8 个**（门面 + 7 依赖） | **1 个**（仅门面） |
| 预计耗时 | ~70 分钟（受 crates.io 限速） | ~9 分钟 |
| 依赖版本 | 全 rc.5（一致） | 锁死 rc.4（门面以后升级需改依赖声明） |
| 线上版本一致性 | 本地 rc.5 / crates.io 上其余 18 个仍 rc.4（**分叉**） | 门面 rc.5 / 依赖 rc.4（**天然分叉**） |

**本 ADR 起草口径：方案 A**——理由：① **单轴制是本仓既有硬纪律**（contract §[meta].hard_walls B2 明载「workspace.version 单轴制」），破例需在契约里留永久例外；② 8 个 crate 一起推到 rc.5，**线上/本地版本分叉反而比方案 B 少**（方案 B 门面立刻与依赖分叉）；③ LOCKED 项由 **owner（release authority）** 授权修改，是既有机制内的正常动作，不是破例。

⚠️ **诚实边界**：方案 A 需要 owner 明确授权修改 LOCKED 的 `workspace.version`。**未获授权前，本 ADR 只落地代码改动（§2.1），不发版。**

---

## 3. O-6 三阶审查

### 3.1 总体最优

**问**：把 `tools` 移出默认面是不是最优？

**答**：是——**它用零契约成本换掉 64% 的默认依赖重量**。

**拒的 alternative A：保持 252 包默认，靠文档劝用户用 feature**
- 拒理由：多数用户**不会读 feature 列表**；默认依赖树是**事实上的产品面**，文档劝告不改变 `cargo add` 的实际行为。而且实测已证明"文档写清楚了"救不了体积——问题在依赖树本身。

**拒的 alternative B：把 `apeireth-tools-canonical` 的 image 依赖做成可选（改 tools 内部结构）**
- 拒理由：那是**动引擎 crate 的依赖结构**（tools 的 `image_ops` 工具需要 image 内核）⇒ 触及 ADR-0017 B2 裁决与 image 家族拓扑，远重于"门面挪一个 feature"；且 image 家族是**已交付冻结资产**。

**拒的 alternative C：把门面拆成两个 crate（`apeireth` 轻量 + `apeireth-full` 全量）**
- 拒理由：多一个 crate 名要占、多一套版本轴，且用户仍要**自己选**——没有减少"默认面过重"这个问题，只是把决策挪给用户。**单一门面 + feature 门控**是 Rust 生态通行形态。

**为什么本方案最优**：改动面 = 门面自己的 `Cargo.toml` 三行 + 文档；契约 **0 改动**；用户默认体积 252 → 92。

### 3.2 系统最优

**问**：这个改动该放在哪一层？

**答**：**门面层（`apeireth` crate 自身）**——不触碰任何引擎 crate 的结构、依赖、导出。

**拒的 alternative A：改 `apeireth-tools-canonical` 内部（拆 feature）**
- 拒理由：见 §3.1 B。tools 是引擎侧已交付 crate，动它会牵出 image 家族的发布拓扑与 ADR-0017 的 B2 裁决。

**拒的 alternative B：在 `apeireth-runtime` 层做减法**
- 拒理由：runtime 是引擎内核，它的依赖是**功能必需**（91 包里有 22 包是 runtime 带的），减它等于削引擎能力。

**系统位置结论**：依赖树是**门面（产品面）**的问题，不是引擎的问题 ⇒ 在门面解决，引擎 0 改动。

### 3.3 架构最优

**问**：公开面形状是否最优？

**答**：**3 默认 + 4 可选，且每个 feature 一一对应一个模块、不级联**。

**拒的 alternative A：`default = ["minimal"]` 之类的分组 feature**
- 拒理由：分组 feature 会产生"用户选了 `full` 却不知道装了什么"的组合语义；**一一对应**更可预测（`features = ["tools"]` ⇒ 恰好多一个模块）。

**拒的 alternative B：feature 门控 + `all-features` 一键全装**
- 拒理由：`all-features` 会让 CI 矩阵爆炸（7 个 feature 的子集）；且**一个 feature 一个模块**时，CI 只需测 `default` + `--all-features` 两个点。

**公开面形状**：`apeireth::{core, protocol, runtime}` 默认；`memory` / `orchestration` / `guard` / `tools` 各自一个 feature。

---

## 4. 迁移计划（分步、每步可验证）

| 步 | 动作 | 验证 | 状态 |
|---|---|---|---|
| **S1** | 门面瘦身：`tools-canonical` 改 optional + 新增 `tools` feature；lib.rs 门控；README/文档同步 | `cargo check -p apeireth`（默认 + `--all-features`）exit 0 | ✅ **已落地** |
| **S2** | 门禁 | `cargo clippy -p apeireth --all-targets [--all-features] -- -D warnings` 0；`cargo fmt --check` 0；`arch_guard --strict` **VIOLATIONS 0**（edges 仍 118） | ✅ **已落地** |
| **S3** | **owner 授权 bump `workspace.version` → `2.0.0-rc.5`**（LOCKED，release authority） | owner 明确授权（2026-10-31「那就发，授权」） | ✅ **已授权并执行** |
| **S4** | 按拓扑序发布 rc.5（8 个：7 依赖 + 门面） | crates.io API 逐个 200 + 版本 `2.0.0-rc.5` | ⏸️ 待 S3 |
| **S5** | 下游消费实测：空工程 `cargo add apeireth` ⇒ 断言 `Cargo.lock` 包数 **92**（瘦身生效的硬证据） | 实测包数 = **92** | ✅ **已实测** |

⚠️ **S4 限速**：crates.io 新 crate 限速实测约 1 个 / 9 分钟（rc.5 是**新版本**不是新 crate，预期更快；实测为准）。

---

## 5. 回滚

| 步 | 回滚动作 | 回滚后状态 |
|---|---|---|
| S1-S2 | `git revert` 门面 3 个文件 | 回到默认 4 模块 / 252 包 |
| S3 | 根 `Cargo.toml` version 改回 `2.0.0-rc.4`（**LOCKED，需 release authority**） | 回到 rc.4 轴 |
| S4 | `cargo yank -p <crate> --version 2.0.0-rc.5`（**yank 不删代码**，版本仍在 registry，只是隐藏） | rc.5 不可被新依赖解析 |
| S5 | 无（实测，无副作用） | — |

**关键性质**：S1-S2 **不碰 LOCKED、不碰 crates.io** ⇒ 可独立回滚。**唯一不可逆的是 S4 已发出的 rc.5**（但 yank 可隐藏，且 rc.4 永远可用）。

---

## 6. 验收

- [x] `cargo check -p apeireth --offline`（默认）exit 0
- [x] `cargo check -p apeireth --all-features --offline` exit 0
- [x] `cargo clippy -p apeireth --all-targets --offline -- -D warnings` exit 0；`--all-features` 同 0
- [x] `cargo fmt --check` exit 0
- [x] `python scripts/arch_guard.py --strict` **VIOLATIONS 0**（`crates=31 edges=118`，未超 `max_production_edges`）
- [x] 契约 **0 改动**（members/layers/budgets/repos 全未动）
- [ ] owner 授权 bump `workspace.version`（LOCKED）
- [ ] rc.5 已发布且 crates.io 可查
- [x] 下游消费实测：默认依赖树 **92 包**（实测 2026-10-31：`cargo add apeireth@2.0.0-rc.5` → `Cargo.lock` 92 包；带 `features = ["tools"]` → 252 包；`cargo build` 12.31s exit 0；`cargo run` 输出 `facade version = 2.0.0-rc.5`）

---

## 7. 后果（Consequences）

### 7.1 正面

- **默认依赖树 252 → 92 包（减 63.5%）**，用户首次 `cargo add apeireth` 的成本大幅下降。
- **契约零改动**：边数不变（optional 依赖仍计入生产边）⇒ 预算、棘轮、层规则全不受影响。
- **引擎 0 改动**：只动门面自己的三处。
- **不丢能力**：`tools` 一行 feature 即可取回。

### 7.2 负面 / 接受的代价

| 代价 | 状态 |
|---|---|
| **想用工具层的用户要多写一行 feature** | 接受（Rust 生态通行形态） |
| **rc.5 需 bump LOCKED `workspace.version`** | **待 owner 授权**（§2.3 方案 A）。若 owner 选方案 B（门面独立版本轴），则 0 触碰 LOCKED 但破单轴制 |
| **rc.5 需发 8 个 crate + 约 70 分钟（限速）** | 接受 |
| **线上版本分叉**：rc.5 发布后，crates.io 上其余 18 个仍是 rc.4 | 接受（RC 阶段正常；后续版本统一推进时自然收敛） |

### 7.2.1 实施实测（2026-10-31，rc.5 发布）

**版本清扫**（owner 授权后执行）：`2.0.0-rc.4` → `2.0.0-rc.5` 共 **166 处 / 107 个文件**，分三类：
- **A 类 权威源 + 门禁 4 件**（45 处）：根 `Cargo.toml`（`[workspace.package].version` + 29 条内部依赖 version）/ `frontend/companion-desktop/package.json` / `src-tauri/Cargo.toml` / `tauri.conf.json`
- **B 类** `crates-ui/Cargo.toml`（10 处，独立版本轴但口径对齐）
- **C 类** 现值行（121 处 / 61 文件）：契约 `contract.toml`、各 crate README 安装示例、发行脚本（`packaging/**` / `scripts/install|uninstall/**` / `docker-compose.yml`）、用户文档（`docs/01-architecture` / `docs/02-guides` / `docs/development`）
- **D 类 不改**：带日期的历史记录（`CHANGELOG.md` / `reports/**` / `docs/adr/**` 正文 / `docs/archive/**` / `legacy/**`）—— 符合「历史不追溯改写」

**门禁**：`pwsh scripts/check-release-version.ps1 -ExpectedVersion 2.0.0-rc.5 -Strict` → **RESULT: SUCCESS**（全 crate + desktop 三件套一致）；`arch_guard --strict` **VIOLATIONS 0**（`crates=31 edges=118`）；`clippy --workspace --all-targets --locked -D warnings` 0；`fmt --check` 0；文档三门禁 0。

**⚠️ rc.5 发布集 = 23 个（远超 §2.3 预估的 8 个）**：实测 `cargo metadata` 依赖闭包（含 **optional** 依赖 —— 实测 `cargo package` 会解析 optional 依赖）：

| 层级 | 数量 | 说明 |
|---|---:|---|
| 门面依赖闭包 | **22** | 门面的 7 个直接依赖 + 其传递依赖（`tools-canonical → image-core` 链带入 image 家族 11 个；`memory/guard/orchestration` 带入 `governance/plugin/provider/storage`） |
| 门面自身 | 1 | `apeireth` |
| **可留 rc.4** | 3 + 4 | `credentials` / `organ` / `runtime-assembly`（无人依赖）+ 4 个 adapters（产品仓，**不发布**） |

⇒ §2.3 的「8 个」是**低估**（当时只数了门面的直接依赖，未算 optional 与传递依赖）；实际 23 个。**结论未变**（单轴制保持），但**工时估算需修正**。

**⭐ 新 crate 限速与新版本限速是两回事（关键实测）**：
- **首次发布新 crate**：受限速约束（实测约 **1 个 / 9 分钟**，报 429「published too many new crates in a short period」）；
- **已发布 crate 的新版本**（rc.5）：**不受该限速约束** —— 23 个 rc.5 推送过程中 **429 命中 0 次**，单个 crate 约 8–15 秒完成。
⇒ **推论：后续版本迭代的发布成本极低**（rc.5 全部发布约 6 分钟），限速只在"开新 crate"时才是瓶颈。

**传输层瞬时错误（新发现，已扩展重试正则）**：`HTTP/2 stream reset by server (error 0x5 STREAM_CLOSED)` —— 出现在 3.6MiB 的 `tools-canonical` 上传时。**这不是限速**（429），是 HTTP/2 流被服务端重置。已扩展重试匹配（`stream error|HTTP/2 framing|STREAM_CLOSED|broken pipe|early eof`）。

**幂等续跑已验证**：重跑时已上传的版本报 `already exists on crates.io index` ⇒ 脚本识别并跳过，**不中断**（否则每次网络抖动都要从头跑）。

**⚠️ 索引传播延迟（实测）**：上传成功后 crates.io API 的 `max_version` **不会立刻更新** —— 实测 `runtime` / `provider` 上传后 API 仍显示 rc.4，数分钟后才变 rc.5。⇒ **核验发布结果必须留出传播窗口，不能以"刚上传完"为依据下结论**。

**⚠️ 断言口径修正（实测自洽核验）**：初稿断言"91 包"系**不含门面本体**的三个直接依赖实测值；下游 `cargo add apeireth` 实测为 **92 包** —— 差值恰为**门面本体自身 1 个包**（交叉核实：92 − 1 = 91 ✅）。对外口径统一取 **92**（用户真实看到的数字）。带 `features = ["tools"]` 实测 **252 包**。

**✅ rc.5 发布完成（2026-10-31 实测）**：**26 / 26 全部 rc.5**（crates.io API 逐个核验 200 + `max_version = 2.0.0-rc.5`）。门面板最终状态：`num_versions = 2`（rc.4 + rc.5）、license `Apache-2.0 OR MIT`、keywords 5 个、categories 3 个合法枚举、**包本体 22.7 KiB**（门面本体极小，符合"纯 re-export"设计）。

**⚠️ 线上与本地的一处已知差异（如实登记）**：rc.5 发布**之后**又把门面 `src/lib.rs` 与 `README.md` 里的包数表述从 "91" 更正为 "92"（+ 门面本体 1 个，见上文口径修正）⇒ **crates.io 上的 rc.5 源码里写的是 91**。**这是注释级差异，功能零变化**；crates.io 不可覆盖同版本号 ⇒ 该更正随**下一次发版（rc.6）**自然带上。

### 7.3 后续 stage

- rc.5 发完后：crates-ui 6 个库 crate 可与 rc.5 一并排期（届时版本轴独立于主仓）。
- 门面「默认面」的进一步收窄（只留 `core`）**暂不做**：实测 67 vs 92 只差 25 包，而 `protocol`/`runtime` 是绝大多数用户真正要的 ⇒ 收益小、feature 语义变碎。

---

## 8. 参考与出处

| 事实 | 出处 / 命令 |
|---|---|
| 各组合包数 67 / 69 / 91 / 251 | 空工程 `cargo add <组合>` + `cargo generate-lockfile` + `grep -c "^name = " Cargo.lock`（2026-10-31） |
| 门面 rc.4 实测 252 包 | 同法（`cargo add apeireth`） |
| `tools-canonical` 传递依赖 image 家族 | `cargo metadata`（2026-10-31）：`tools-canonical → apeireth-image-core → 10 个 vendored crate` |
| optional 依赖也计入生产边 | `cargo metadata --no-deps` 实测 + `arch_guard` 实测（118 条含 7 条门面边） |
| crates.io 版本不可覆盖 | [The Cargo Book — Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html) |
| `workspace.version` 属 LOCKED | `AGENTS.md` §4；`docs/architecture/contract.toml` §[locked].`workspace_version_source` |
| 单轴制（hard wall B2） | `docs/architecture/contract.toml` §[workspace.metadata.apeireth].`hard_walls` |
| 上游待办来源 | [ADR-0019](0019-apeireth-facade-crate.md) §7.3 |

---

_本文由仓库治理线（总工：lead）撰写于 2026-10-31，依据 owner 同日裁决「B：做门面瘦身」。**Status = Accepted**；S1-S2 已落地并通过全部门禁；**S3（bump LOCKED `workspace.version`）待 owner 授权后方可发版**。_
