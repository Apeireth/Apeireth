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
