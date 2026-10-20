# 0004. R3 配置 schema + env 登记 + wiring ledger（spec）

- **Status**: Accepted（owner 授权代裁 2026-10-20「按你的建议来吧」；实施归效率线，schema 由架构线定）
- **Date**: 2026-10-20
- **Owners**: 架构线（schema/守卫规则）· 效率线（登记表/生成器/CI）
- **Supersedes**: —
- **契约影响**: `contract.toml [env]` 增加 `registry_file`；`arch_guard` v0.2 增加"env 名必须在登记表"检查（report-only 先行）
- **例外关联**: 迁移期新增 env 走"登记 + 同批基线更新"特批（ADR-0001 §2.2 env 豁免净零）
- **预算净额**: 新增 2 个数据文件 + 1 个生成器/校验脚本；0 新 crate / 0 新外部依赖

## 1. 背景

154 个 env 名散在 148 个文件、`env::var` 211 次；没有类型化配置、没有登记表、没有"实现→接线→启用→验证"四列账。`AGENTS.md §3` 曾写"默认开关以 production.rs 为准"，实测 production.rs 直接 env 读取 = 0——说明"口头真源"已经失真。R3 分三步把配置与接线变成可查账的数据。

## 2. 决策（三步，先账本后代码）

### R3a（Wave 2）登记表 + 接线账骨架

1. **`docs/architecture/env-registry.toml`**：每个开关一条记录
   `name / default / layer / owner / purpose / level / status(current|migrated|deprecated)`。
   首次以实测 154 条自动播种，`owner = "TODO-OWNER"` 起步；**TODO owner 数量每波必须下降**（记分牌项）。
2. **`docs/status/wiring.yaml` + `scripts/wiring_report.py`**：生成式接线账，每项 =
   `id / capability / implemented(sources) / wired(production refs) / default_enabled / validated(tests+claims_id) / gaps`；
   生成器扫 `claims.yaml` + 能力矩阵 + 源码引用；`--verify` 检查漂移/缺失（与 `docs_truth.py` 同模式：生成物禁手改）。
3. **`contract.toml [env]`** 增加 `registry_file`；`arch_guard` 增加规则：新 env 名不在登记表 → W3 违规。

### R3b（Wave 3）登记填实 + 高价值模块迁移

- 逐条补 owner/purpose；把 CLI/生物机制/写族等高价值开关迁入 typed config（`Config` struct + `from_env()` 单点读取）；
- 迁一个模块 = 登记表 `status: migrated`，`arch_guard` 对 migrated 模块做"只准 config 模块读 env"的位置检查（`allowed_read_locations` 逐步收紧）。

### R3c（Wave 4）守卫收紧

- `arch_guard` 对 `migrated` 标记的开关，禁止在 config 模块之外 `env::var`；
- wiring ledger 与 claims 交叉校验：`PRODUCTION WIRED` 的能力必须有生产引用，`IMPLEMENTED` 的必须有 `gaps` 原因。

## 3. 明确不做（本 ADR 范围外）

- 不一次迁移 154 个开关（风险高、收益低）；先登记+账本，逐模块迁；
- 不引入配置 crate/外部依赖；配置结构落在现有 `foundation`/`adapters` 内；
- 不改任何开关默认值与语义（迁移只改读取位置，不改行为）。

## 4. 验收

- R3a：登记表覆盖实测 154/154；wiring ledger 覆盖 `claims.yaml` 全部条目（当前 16，后续随 claims 增长）；CI `--verify` drift=0；`arch_guard --strict` 0 违规。
- R3b：每波 `TODO-OWNER` 数量下降；migrated 模块 env 读取位置符合 `allowed_read_locations`。
- R3c：`PRODUCTION WIRED` 能力 100% 有生产引用；`IMPLEMENTED` 100% 有 gaps 原因。

## 5. O-6 三阶审查

- **总体最优**：先"账本可查"再"代码迁移"——154 个开关一次改代码，回归面不可控；账本先行能让每步迁移有基线。拒的替代：①直接 typed config 大迁移（无账本，漏项不可见）②只写文档不接守卫（重演"口头真源"）。
- **系统最优**：登记表在 `docs/architecture`（结构真源旁），接线账在 `docs/status`（状态真源旁），生成器在 `scripts/`（与 docs_truth 同族）。拒的替代：①塞进 claims.yaml（结构规则与能力状态混源）②塞进 Cargo metadata（不可读、不可审）。
- **架构最优**：全部是数据文件 + 只读脚本，0 新 crate、0 新 dep；生成物禁手改、漂移可检测。拒的替代：①自研配置 DSL（先证明现有 TOML 不够）②引入第三方配置库（0 新外部 dep 纪律）。

## 6. 后续

R3a 交付后，R1a 的 `nervous_system` 与 R1b 的开关收敛直接复用 env-registry 的 owner/level 字段；R6 的分层门禁复用 wiring ledger 的 validated 列。

## 7. 口径补充（2026-10-20 效率线施工同步 → 架构线裁定）

### 7.1 `UNCLASSIFIED` 的正式定义（回应效率线提问）

- `level` 枚举 = `PRODUCTION WIRED | IMPLEMENTED | NOT IMPLEMENTED | UNCLASSIFIED`；
- **`UNCLASSIFIED` 是"登记先行占位"，不是四级口径之一**：只表示"该开关已登记但尚未分级"，
  **不得用于对外宣称**，也不等于任何交付级别；
- 机器口径：`arch_guard` **不校验** level 值（它只查 env 名是否登记）；四级口径仍由
  `claims.yaml` + wiring ledger 承担——**不把占位符做成第二套能力状态**；
- 计数纪律：`UNCLASSIFIED` 与 `TODO-OWNER` 同列记分牌，**每波只减不增**；R3b 收口时
  允许为 `IMPLEMENTED`/`NOT IMPLEMENTED` 或待验证的 `PRODUCTION WIRED`（须有证据）。
- 未来若要把 level 升级为 strict 机器校验（例如禁止 UNCLASSIFIED 上线），**先改本 ADR 定口径再改守卫**，
  不允许守卫先行倒逼登记表造假。

### 7.2 登记先行（registration-first）

新开关可先登记（`owner = "TODO-OWNER"`、`level = "UNCLASSIFIED"`、`status = "current"`），
即解除 `arch_guard` 的 W3「未登记」违规；但**收口必须补默认关 + 关时零变化测试**，
否则 capability ledger 不认。EXC-0001 即按此转正退休（见 exceptions.toml 索引注）。

### 7.3 R3b 排序（架构线批准效率线建议）
1. **先填 claims 覆盖的开关 owner**（当前 16 条 claims 对应开关）——理由：claims 是能力真源，
   owner 最明确、收益最高；
2. **再按 layer 分批清 TODO**：`foundation → engine → capabilities → adapters`（依赖方向，先稳后动）；
3. 每批 commit 正文报 `entries / TODO-OWNER / UNCLASSIFIED` 三个计数（`--stats` 可复核），
   三个数只许向"TODO/UNCLASSIFIED 下降"走；
4. 数据质量：种子抽取的 `purpose` 含 HTML 注释碎片（如 `! ! # ...`）时以源码为准清洗，不算能力声明。

### 7.4 R3c 升门禁

`wiring_report.py --verify` 从 report-only 升为 **CI 交叉校验门禁**（与 `arch_guard` 分工：
arch_guard 管结构/依赖/env 名，wiring ledger 管"实现→接线→启用→验证"四列证据）；
升格前保留两周 report-only；口径：`PRODUCTION WIRED` 必须有生产引用，`IMPLEMENTED` 必须有
`gaps` 原因，缺证据即 FAIL。R3c 具体批次由效率线排期，架构线验收。

### 7.5 R3b foundation 批验收标准（2026-10-20 架构线裁定）

**允许的动作范围**：只改 `docs/architecture/env-registry.toml`（entries 的 owner/purpose/level/status 字段）与必要文档；不改源码、不改契约、不新增开关。
**证据标准**：
- `owner` = 读取该 env 的 crate/组件名（如 `apeireth-runtime`），**不得填团队名、不得留 TODO-OWNER**；
- 新增**可选字段 `evidence`**（string，`路径:行号`，多个用 `;` 分隔）记录"读取点"证据；
  foundation 批中每个 `owner != TODO-OWNER` 的条目 `evidence` 必须非空；
- `level` 保守取值：claims 覆盖的按 claim 的级别；否则留 `UNCLASSIFIED`；`IMPLEMENTED` 必须能指向
  "实现存在但无生产引用"的 gaps；**禁止无生产引用却标 `PRODUCTION WIRED`**；
- `purpose` 必须来自源码文档注释并清洗：**不得以 `!` 开头、不得含代码注释碎片（`#`/行尾 `//`）**；
- `default` 必须与源码默认值一致（关时默认 off，除非显式白名单）。

**每批 commit 正文必须报四数**（`--stats` 可复核）：
`entries / TODO-OWNER / UNCLASSIFIED / purpose_fragments`；要求 foundation 子集 TODO-OWNER 归零、
purpose_fragments = 0，且全表 TODO-OWNER/UNCLASSIFIED 不增加。

**架构线验收**：
1. `tomllib` 解析通过；2. `arch_guard --strict` 无新增违规；3. 三数单调；4. `purpose` 碎片 = 0；
5. **抽查 ≥10%（至少 5 条）**：`evidence` 路径:行确实读取该 env；6. 只动允许文件。

**R3c workflow review 点**（效率线提交 report-only 版本后）：
①触发路径（wiring.yaml / claims.yaml / 脚本变更）②命令为 `wiring_report.py --verify`
③`continue-on-error: true` 灰度 ④日志/artifact 落盘 ⑤失败输出含缺失项清单。
架构线 review 通过后两周转 blocking；改动 workflow 需重新 review。

### 7.6 验收记录（2026-10-20，架构线）

**R3b foundation 批（commit `66dac8cd`）：通过。**

- 提交版四数（`--stats` 可复核）：`entries 155 / TODO-OWNER 132（↓13，foundation 子集 15→0）/ UNCLASSIFIED 154（↓1）/ purpose_fragments 26（↓6，foundation 子集 = 0）`；
- foundation 15 条 `owner != TODO-OWNER` 且 **evidence 全部非空**（15/15）；架构线抽查 5/5：`路径:行` 真实读取该 env（`APEIRETH_ANTHROPIC_KEY` 等命中 `std::env::var` 或 env 常量）；
- 范围合规：该提交只动 `docs/architecture/env-registry.toml`；工具升级另 commit `8530c5bf`（`scripts/wiring_report.py` / wiring.yaml / workflow / gitignore）；
- **条件**：ADR §7.5 的「purpose_fragments = 0」按**本批处理子集**执行——全表 26 处碎片随后续 layer 批次清零，**R3b 收口时全表必须 = 0**；TODO-OWNER 132 继续按 §7.3 分批降。

**R3c workflow（`.github/workflows/wiring-ledger.yml`，commit `8530c5bf`）：review 通过（report-only）。**

- 五点时：触发路径覆盖 `docs/status/**`、`env-registry.toml`、`wiring_report.py`、`crates/**` ✅；R3a registry 覆盖与 wiring drift 为 blocking（生成物禁手改，与 docs_truth 同模式）✅；R3c 四列交叉校验用 `--crosscheck` + `continue-on-error: true`（灰度至 2026-11-03）✅；日志落盘 + artifact ✅；失败输出含缺失项清单（由 `wiring_report.py` 输出，后续以一次样例核对）⚪待核。
- **转 blocking 前需再次 review**：删除 `continue-on-error`、确认缺失项清单样例、与架构线同步 `current_wave`。

### 7.7 四批验收 + 尾批口径裁决（2026-10-20，架构线）

**R3b engine / capabilities / adapters / image_family 四批：全部通过。**

- 提交：`846468bb`（capabilities）、`e8b98e85`（adapters）、`ed6e2d16`（image_family，终批）、engine 批同模式；**每批只动 `env-registry.toml`**（架构线逐 commit 核 stat）。
- 终态四数：`entries 155 / TODO-OWNER 0 / UNCLASSIFIED 147 / purpose_fragments 0`——**R3b 主体完成**；
- evidence：owner 填实者 0 缺失；架构线跨 5 层抽查 5/5 `路径:行` 真实读取（provider/cli/tools/image/compose/core）。
- **尾批裁决（一句）**：测试/基准夹具 env 与新字段 `kind = "test-only"` 归一类——`APEIRETH_IMAGE_UNSHARP_*`（7 条，仅 benches/tests 引用）与 `APEIRETH_UNIQ_SECRET_MARKER_12345`（secret-scan 夹具）**允许 `purpose = "TODO-PURPOSE"` 但必须同时 `kind = "test-only"` 并注明"测试/基准夹具，非产品开关"**；产品开关（`kind` 缺省 = `product`）**禁止** TODO-PURPOSE，purpose 必须来自源码文档注释。test-only 条目不参与 PRODUCTION WIRED 证据要求，仍占 entries 计数、不计入四级口径。
- 后续：R3b 收尾仅剩该裁决落地（效率线一批）；R3c report-only 至 2026-11-03 后按 §7.4/§7.5 再 review 转 blocking。

### 7.8 注释/动态名条目的 evidence 口径裁决（2026-10-20，架构线）

**证据双形态**：

- `direct:`（推荐）——显式 `pub const ... = "NAME"`、`std::env::var("NAME")` 或 env 读取封装处；**产品开关只认 direct**；
- `scan-only:`（受限）——名称仅出现在注释、`format!` 动态拼接、测试/基准夹具中，无显式独立读取点；
  仅 `kind = "test-only" | "derived"` 可用，且 `level` 必须 `UNCLASSIFIED`，不参与 PRODUCTION WIRED 证据与四级口径。

**`kind` 枚举**：`product`（缺省）/ `test-only` / `derived`。
`derived` = 动态前缀或拼接名（如 `APEIRETH_TUNE_`、`APEIRETH_REG_*`）——不是独立开关，purpose 必须注明"动态拼接/非独立开关"。

**棘轮**：`derived` 条数只减；清理二选一——①源码补显式 const 升级为 direct；②确认非开关后删除条目。
例外：`APEIRETH_TUNE_` 纯前缀**保留为 derived**（`arch_guard` 扫描会产生该名，删条目会让守卫重新报 W3）。

**本批 7 条处理**：`APEIRETH_TUNE_` + 4 条 `APEIRETH_TUNE_*` → `kind="derived"`、evidence 前缀 `scan-only:`；
`APEIRETH_REG_APEIRETH_CORE_A/B` → `kind="test-only"`、evidence `scan-only:`（测试夹具）。

**`arch_guard` 扫描口径不改**（不为数据形状改规则）；归类由登记表承担，守卫只看"名是否登记"。

### 7.9 架构线接管裁决（2026-10-20，效率线收尾后 4 项）

1. **`APEIRETH_MASTER_TOKEN` evidence**：本仓无 direct 读取点（仅 `gate.rs:53` 文档 + workspace 外 companion 消费）。禁止补假 direct/假 const。**新增受限证据形态 `external: <仓库外/workspace 外消费点>`**（仅限产品开关 + 外部消费者场景）：须附 companion 侧 `路径:行`；当前维持 `product + scan-only + PENDING` 标签。**Wave 3 收口前**必须落 `external:` 证据，否则删除登记（同批处理 `arch_guard` 注释扫描口径，不得永久 PENDING）。
2. **derived 确认**：登记表实有 **10 条** derived（`ANTHROPIC_`/`API_`/`DISABLE_`/`ENABLE_`/`QUOTA_MAX_` 5 条前缀 + `TUNE_` 组 5 条）；**全部确认有效**（CLOSEOUT 所指 5 条 = TUNE 组）。按 §7.8 只减：补显式 const 升 direct 或确认非开关后删除。
3. **R3c 转 blocking**：按原批准计划，**2026-11-03 到期后**由架构线 review（删 `continue-on-error` + 缺失项清单样例 + `current_wave` 同步）再转；不提前。
4. **matrix MCP advisory**：`APEIRETH_ENABLE_MCP` 已有 env-registry 登记，但 matrix 缺行属**生成物未覆盖**——**不手工补 matrix §0.1/§0.2**（会漂移）；纳入 DTS P1b 生成清单（生成时自动从 env-registry 带出）；紧急手改须标注并由生成器覆盖验证。
5. **效率线交付报告**：`artifacts/third-line-delivery-report-2026-10-20.md` 按本仓 artifacts 惯例**保持 untracked**，不额外 commit；长期交接真源 = 已入库的 `docs/status/CLOSEOUT-2026-10-20.md`（§5 移交项本裁决即接管记录）。
