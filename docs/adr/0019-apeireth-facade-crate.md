# 0019. `apeireth` 门面 crate —— 品牌旗舰名占位 + 单依赖入口

- **Status**: **Accepted**（2026-10-31 owner 裁决：口述选定方案 A + 「发就行」；本 ADR 与实现同批落地）
- **Date**: 2026-10-31
- **Owners**: owner（2026-10-31 裁决「A」：**建正经门面 crate 占名，不发占位垃圾**）；实施线 = 仓库治理线（总工：lead）
- **Supersedes**: —
- **契约影响**:
  - `contract.toml` §[workspace]：`members` **30 → 31**、`product_crates` **29 → 30**（`bench_crates` 1 不变）
  - `contract.toml` §[layers]：`engine` 数组新增 `"apeireth"`（**0 新增允许边**——`engine->foundation` / `engine->engine` / `engine->capabilities` 三条边**已在允许表内**）
  - `contract.toml` §[budgets]：`max_workspace_crates` **30 → 31**（棘轮需 ADR，本文即依据）；`max_production_edges` **118 不变**
  - `contract.toml` §[deps]：生产边 **111 → 118**（+7，**恰等于** `max_production_edges = 118` 上限；**未抬预算**。实测：optional 依赖也计入生产边）
  - `contract.toml` §[[workspace.repos]]（ADR-0017 段）：engine 仓 crate 清单 +1
- **例外关联**: —
- **预算净额**: crate **+1** / 生产边 **+7**（111 → **118**，恰等于 `max_production_edges` 上限，**未抬预算**）/ 允许层边 **±0** / 开关 **±0** / 文件尺寸 **+1 个新 crate（lib.rs ~130 行 + README + Cargo.toml）**
  > 落地实测修正：初稿估 +4 边有误——**optional 依赖同样计入生产边**，故 4 默认 + 5 可选 = 9 条会超预算（120 > 118）。已按纪律**砍掉 `organ` / `image` 两个可选模块**（移出门面，用户直接依赖对应 crate）⇒ 4 默认 + 3 可选 = **7 条**，总数落在 118 上限内。

---

## 1. 背景与问题

### 1.1 占名实测（2026-10-31，crates.io API 逐个 404/200 判定）

| 名字 | 状态 | 名字 | 状态 |
|---|---|---|---|
| **`apeireth`（裸名）** | ⬜ **空闲** | `apeireth-engine` | ⬜ 空闲 |
| `apeireth-sdk` / `apeireth-cli` / `apeireth-tui` / `apeireth-gateway` | ⬜ 全空闲 | `apeireth-companion` | ⬜ 空闲 |
| `apeireth-macros` / `apeireth-mcp` / `apeireth-skills` | ⬜ 全空闲 | `apeireth-image` / `apeireth-agent` | ⬜ 空闲 |

已上架（2026-10-31，ADR-0017 §10 S2b 执行中）：`apeireth-core` / `apeireth-protocol` / `apeireth-plugin` / `apeireth-governance` / `apeireth-credentials` / `apeireth-guard` / `apeireth-storage` / `apeireth-runtime` 等 —— **`apeireth-*` 前缀族已在占，唯独裸名未占**。

### 1.2 裸名为什么必须占

1. **用户最自然的写法是 `cargo add apeireth`**。若裸名在陌生账号手里，我们 31 个 crate 全部只能以 `apeireth-*` 出现，旗舰入口永久缺席。
2. **最坏情况不是"没人用"，而是"被人拿去做混淆包/蹭品牌"**——一个叫 `apeireth` 的第三方 crate 会天然获得品牌联想，我们无法下架、无法改名（crates.io 名字先到先得、发布永久）。
3. **占名成本此刻最低**：名字空着，且已有可用的真实内容（见 §1.4），不需要任何"占位"话术。

### 1.3 为什么不能发占位包（0 装纪律）

- crates.io 上**已发布版本不可删除**，只能 `yank`（`yank` 也不删代码）⇒ 发一个空壳 = 在官方仓库留一条**永久假账**。
- 本仓 0 装纪律（`AGENTS.md` §1）明确"不许把 0 装包装成已达四级口径"；同理，**不许把空壳说成产品**。
- ⇒ 占名必须靠**真实、可用、不吹牛**的 crate 完成。

### 1.4 现状缺陷：没有统一入口

用户要用整套引擎，今天必须手写多行依赖：

```toml
apeireth-core = "2.0.0-rc.4"
apeireth-protocol = "2.0.0-rc.4"
apeireth-runtime = "2.0.0-rc.4"
apeireth-tools-canonical = "2.0.0-rc.4"
# …… 还要自己判断版本互通
```

Rust 生态的通行做法是提供**门面 crate**（`tokio` / `serde` / `bevy` 均有）：用户依赖一个名字，拿到成套入口。**本 ADR 补的正是这一格。**

---

## 2. 决策

**新建 `apeireth` 门面 crate：纯 re-export、零逻辑、无新能力声明。用它占住裸名，同时给用户一个单依赖入口。**

### 2.1 公开面（最小、诚实）

| 项 | 内容 |
|---|---|
| crate 名 | `apeireth`（版本随 workspace 单轴 `2.0.0-rc.4`） |
| 内容 | **只有 `pub use` 与文档**。零业务逻辑、零新 trait、零新注册表 |
| 默认导出 | `apeireth_core as core`、`apeireth_protocol as protocol`、`apeireth_runtime as runtime`、`apeireth_tools_canonical as tools` |
| feature 门控 | `memory` / `orchestration` / `guard` —— 默认关，按需开。**`organ` / `image` 不进门面**（直接依赖对应 crate）：前者用途窄，后者含 10 个 vendored crate 且会挤占生产边预算 |
| 文档 | README 写明：**这是门面，不是新能力**；能力四级口径见 `docs/03-reference/capabilities-matrix.md`；测试数字不抄写（指向 `test-baseline.md`） |

### 2.2 归属层次：`engine` 层（**不动允许边**）

把 `apeireth` 登记进 `contract.toml` §[layers].`engine`：

- 它需要 `engine->foundation`（core/protocol）、`engine->engine`（runtime）、`engine->capabilities`（tools）三条边 —— **三条全已在 `allowed_layer_edges` 内** ⇒ **0 新增允许边**，对契约棘轮（只减不增）零冲击。
- 拒"新开一个 `facade` 层"：那要新增 3 条允许边（facade->foundation/engine/capabilities），**净增允许集合**，与棘轮正面冲突，收益为零。

### 2.3 发布位置：拓扑序**最后**

门面依赖全部被 re-export 的 crate ⇒ 必须排在它们之后（`--print-order` 自动算出，不手抄）。它是第 32 个发布的 crate（ADR-0017 发布集 31 → 32）。

### 2.4 品牌名清单处置（12 个候选）

| 处置 | 名字 | 理由 |
|---|---|---|
| **本文占** | `apeireth`（裸名） | 旗舰入口，真实内容可用 |
| 随将来真实 crate 占 | `apeireth-image`（image 家族统一门面）、`apeireth-engine`（可选别名）、`apeireth-macros`、`apeireth-mcp`、`apeireth-skills`、`apeireth-agent` | **有真实内容才发**；无内容就留着，不发空壳 |
| 随产品仓发布时占 | `apeireth-sdk` / `apeireth-cli` / `apeireth-tui` / `apeireth-gateway` / `apeireth-companion` | ADR-0017 裁定 adapters 归产品仓、`publish = none`；届时若要公开再占（需另裁） |

**不再新开"占名批次"**：本文只解决裸名这一格，其余按"有内容才发"的原则随真实 crate 走。

---

## 3. O-6 三阶审查

### 3.1 总体最优

**问**：在"裸名要占"这个诉求上，建门面 crate 是不是最优切入点？

**答**：是——因为它是**唯一同时满足"占名 + 0 装纪律 + 真实价值"**的路径。

**拒的 alternative A：发 `0.0.0` 空壳占名 + yank**
- 拒理由：① crates.io 上**永久留一条空壳版本**（yank 不删代码）——这是官方仓库里的假账；② 空壳包必然被下游误依赖（"apeireth 0.0.0 什么也没有"）；③ 违反 0 装纪律。**占名不该以造假为代价。**

**拒的 alternative B：不占，等将来有需要再说**
- 拒理由：名字**先到先得且永久**，空窗期完全不可控；`apeireth` 是品牌旗舰名，被占后的补救手段只有"改名/打官司"，而 crate 名对外是 API（改名波及所有下游）。**这是少数"现在做成本低、以后做成本无穷"的事。**

**拒的 alternative C：把门面做成"完整 SDK"（带 bootstrap / CLI / 配置加载）**
- 拒理由：那会让门面变成**第二个 `apeireth-runtime-assembly`**，制造"两个装配入口"——正是 ADR-0008（单一 ABI 家族 / 单注册表 / 单命名）与 R2 收口要消灭的结构债。门面**只允许 re-export**。

**为什么本方案最优**：占名（零风险）与补入口（真实价值）是同一件事；且实现成本 < 200 行、零新能力、零新依赖（全用既有 workspace 依赖）。

### 3.2 系统最优

**问**：在 `foundation → engine → capabilities → adapters` 的依赖图里，它该放哪一层？

**答**：**`engine` 层**，理由见 §2.2——它是全图唯一"放进去 0 新增允许边"的位置。

**拒的 alternative A：放 `foundation` 层**
- 拒理由：foundation 是**被所有人依赖的底座**（core/protocol/plugin/governance/credentials/orchestration）。让门面进 foundation ⇒ 任何 crate 依赖门面都不会成环，但**语义倒置**：门面依赖 runtime（engine 层），而 foundation 不允许 `-> engine` 边 ⇒ 直接违规。**不可行。**

**拒的 alternative B：放 `adapters` 层**
- 拒理由：adapters 是**产品交付面**（gateway/cli/tui/sdk），按 ADR-0017 归**私有产品仓**。门面是公开引擎的入口，放进核心里去会与分仓边界冲突。

**拒的 alternative C：新开 `facade` 层**
- 拒理由：见 §2.2——净增 3 条允许边，违反契约棘轮。

**系统位置结论**：`engine` 层 + 0 新增允许边 + 生产边 111→115（预算 118 内）。**这是"改动最小、语义最正"的位置。**

### 3.3 架构最优

**问**：在单向依赖 + 扩展点约束下，门面的公开面形状是否最优？

**答**：**纯 `pub use` + feature 门控**，公开面 = **4 个默认模块 + 3 个可选模块**（落地实测从 5 个可选砍到 3 个以守住生产边预算 118）。

**拒的 alternative A：门面自带 `prelude`（大量 `use` 重导出）**
- 拒理由：`prelude` 会把**命名空间所有权**从各 crate 收拢到门面 ⇒ 将来各 crate 增删符号会变成门面的**破坏性变更**（semver 上算 breaking）。**不做 prelude**，只做模块级 re-export。

**拒的 alternative B：门面 re-export 全部 31 个 crate（无 feature 门控）**
- 拒理由：会把 image 家族（11 crate）+ wgpu 树 + memory 的 sqlite 一起拉进每个用户的依赖图，**默认依赖树爆掉**；且与 `deny.toml` 的多版本策略冲突。**默认只给 4 个核心模块，其余 feature 门控。**

**公开接口形状**：`apeireth::{core, protocol, runtime, tools}` + `apeireth::memory` / `organ` / `image` / `guard` / `orchestration`（feature 开）。

---

## 4. 迁移计划（分步、每步可验证）

| 步 | 动作 | 验证 | 回滚 |
|---|---|---|---|
| **S1** | 新建 `crates/apeireth/`（`Cargo.toml` + `src/lib.rs` + `README.md`），登记进根 `Cargo.toml` members（30→31）与 `[workspace.dependencies]` | `cargo check -p apeireth --offline` 0 错 | 删目录 + revert |
| **S2** | 契约登记：§[workspace] members/product_crates、§[layers].engine、§[budgets].max_workspace_crates、§[[workspace.repos]] engine crate 清单 | `python scripts/arch_guard.py --strict` **0 违规** + 基线净额 ≤ 0 | 单 commit revert |
| **S3** | 门禁：`cargo fmt --check` / `cargo clippy -p apeireth -- -D warnings` / `cargo test -p apeireth` | 三条全 0 | revert |
| **S4** | 发布占名：`cargo publish -p apeireth --registry crates-io`（拓扑序最后） | crates.io API 返回 200 + 版本可见 | yank（名字保留） |
| **S5** | 文档随行：README 门面说明 + `docs/03-reference/` 加一行入口指引 | 文档门禁（caliber / links / truth）全 0 | revert |

⚠️ **发布节奏**：crates.io 对新 crate 有限速（实测 429，窗口约 10 分钟）。`apeireth` 应排在其他 crate 发完后单独发，**不要与批次挤窗口**。

---

## 5. 回滚

| 步 | 回滚动作 | 回滚后状态 |
|---|---|---|
| S1-S3 | `git revert` + 删 `crates/apeireth/` | 回到 30 members，契约未动 |
| S4 | `cargo yank -p apeireth --version <v>` | **名字仍归我们**（yank 不释放名字），仅隐藏该版本 |
| S5 | `git revert` | 文档回原状 |

**关键性质**：S1-S3 全程主依赖图 0 影响（新 crate 无人依赖）；S4 一旦成功，**裸名永久归本项目**——这是本 ADR 唯一"不可逆但想要"的结果。

---

## 6. 验收

- [x] `python scripts/arch_guard.py --strict` 通过且**基线净额 ≤ 0**（实测：`crates=31 edges=118`、**VIOLATIONS 0**；基线 111→118 条 edge 已按本文授权更新）
- [ ] `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` 0 failed
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告；`cargo fmt --check` 0 diff
- [ ] 契约条款更新（§[workspace] / §[layers].engine / §[budgets] / §[[workspace.repos]]）
- [ ] `apeireth` 在 crates.io 可见（API 200 + 版本 `2.0.0-rc.4`）
- [ ] 下游消费实测：空目录 `cargo add apeireth@2.0.0-rc.4` 真下载编译通过
- [ ] 文档随行：门面 README 明确"**这是门面，不是新能力**"，测试数字不抄写
- [ ] LOCKED 5 项 0 触碰 / 0 引新外部 dep（门面只用既有 workspace 依赖）

---

## 7. 后果（Consequences）

### 7.1 正面

- **裸名永久归本项目**（唯一"现在便宜、以后昂贵"的占名机会被抓住）。
- **用户入口从 4-14 行依赖降为 1 行** `apeireth = "2.0.0-rc.4"`。
- **零新能力、零新依赖、零允许边** ⇒ 对既有架构零冲击。
- 顺带把"品牌名清单"一次性盘清（§2.4），避免以后零散决策。

### 7.2 负面 / 接受的代价

| 代价 | 状态 |
|---|---|
| **crate 数 +1**（30→31） | 接受。`max_workspace_crates` 由 30 抬到 31（棘轮需 ADR，本文即依据）；**门面是唯一允许增加的 crate 类型**，后续仍应只减不增 |
| **多一个长期维护点** | 接受。门面只做 re-export，维护成本 ≈ 各 crate 增删公开模块时同步一行 |
| **门面 semver 受下游 crate 影响** | 接受。规则：门面 re-export 的模块集合**只增不减**；删模块即门面 breaking |
| **默认依赖树实测 252 包（重要）** | ⚠️ **实测**：空工程 `cargo add apeireth` 后 `Cargo.lock` **252 个包**——因为默认依赖里的 `apeireth-tools-canonical` 传递依赖 `apeireth-image-core`（B2 链）⇒ 带进 image 家族 10 个 vendored crate 及其编解码依赖。**feature 门控只对可选模块生效，对默认模块的传递依赖无效。** 见 §7.3 后续 stage 的收窄建议 |

### 7.3 后续 stage

- **默认面收窄（建议下一批做）**：把 `tools` 也改成 feature（如 `tools` 默认关，或提供 `default = ["minimal"]`），让 `apeireth` 默认只 = `core` + `protocol` + `runtime`（≈ 几十包），把 252 包的重量留给我明确要 `tools` 的人。**需先实测三者各自的依赖包数再定**（本条不改本 ADR 已 Accepted 的导出表，另出一小 ADR 或作为 patch 版本 2.0.0-rc.5 的变更）。
- `apeireth-image` 家族门面：等 image 家族有真实统一 API 需要时再出 ADR。
- 下游消费实测（2026-10-31 已做，记录在案）：空工程 `cargo add apeireth` → `cargo build`（25.35s，exit 0）→ `cargo run` 打印 `facade version = 2.0.0-rc.4` 且四个默认模块解析成功。
- 产品仓发布时再评估 `apeireth-companion` / `apeireth-cli` 等名字（届时可能仍不公开）。

---

## 8. 参考与出处

| 事实 | 出处 / 命令 |
|---|---|
| 裸名与 12 个候选名全部空闲 | `curl -o /dev/null -w '%{http_code}' https://crates.io/api/v1/crates/<name>`（2026-10-31，全 404） |
| 已发布 crate 现状 | crates.io API（2026-10-31）：18/25 已上架，见 ADR-0017 §10 |
| 占名 = 发布、发布永久、yank 不删 | [The Cargo Book — Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html) |
| 新增 crate 必须 ADR | `docs/adr/README.md`「什么时候必须写 ADR」第 1 行 |
| 契约棘轮与预算 | `docs/architecture/contract.toml` §[budgets]（`max_workspace_crates` / `max_production_edges` 均"只减不增，新增需 ADR"） |
| 已有允许边（engine 层三条） | `docs/architecture/contract.toml` §[deps].`allowed_layer_edges`：`engine->foundation` / `engine->engine` / `engine->capabilities` |
| 禁止双装配入口 | ADR-0008（R2 扩展 ABI 统一）与 `contract.toml` §[extension_points] |
| 0 装纪律 | `AGENTS.md` §1 / `docs/04-internal/ENGINEER-MANIFESTO.md` |

---

_本文由仓库治理线（总工：lead）撰写于 2026-10-31，依据 owner 同日裁决「A：建正经门面 crate 占名，不发占位垃圾」。**Status = Accepted**（owner 2026-10-31 裁决「发就行」），实现与本 ADR 同批落地（紧急类允许 Accepted 与实现同批）。_
