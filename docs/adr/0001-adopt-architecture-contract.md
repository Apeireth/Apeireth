# 0001. 采纳架构契约体系与例外预算

- **Status**: Accepted（规则自 2026-10-20 起生效；架构改动按 owner 指示暂缓）
- **Date**: 2026-10-20
- **Owners**: owner（授权代裁：2026-10-20 chat「你了解项目全貌做出判断就行」）→ 执行：架构契约线
- **Supersedes**: —
- **契约影响**: 新建 `docs/architecture/contract.toml`（含 `[exceptions]` 全部裁决值）；新建守卫/基线/ADR/准入清单
- **例外关联**: —（本 ADR 建立例外制度本身）
- **预算净额**: 依赖边 ±0 / 开关 ±0 / crate ±0 / 源码 ±0（纯新增文件 + `.gitignore` +2 行）

## 1. 背景与问题

架构体检（2026-10-20）实测：118 条生产依赖边 vs `ARCHITECTURE.md` 声明 32 条；`foundation→engine` 0 但存在 `capabilities→engine`、`adapter→adapter` 方向异常；`orchestration`（3 万行、9 个依赖者）无 ownership 行；158/154 个 `APEIRETH_*` 开关散在 148 文件；同名概念 `ContextAssembler`×2 双活、`CapabilityRegistry`×2；`production.rs` 直接 env 读取 = 0 却被称为"默认开关真源"（`AGENTS.md` §3）。

问题不在"某次改动"，而在**没有机制阻止下一次改动**：架构文档不是可验证真源，结构新增没有准入，例外没有到期，收敛没有棘轮。继续按"批次"推进，架构只会随批次持续稀释——这违背 O-6（永远追求最优）。

## 2. 决策

采纳"**架构契约 + 架构守卫 + ADR 例外账 + 棘轮基线**"四件套，并定稿例外预算：

### 2.1 四件套

1. `docs/architecture/contract.toml` = 唯一可执行真源（层/边/扩展点/概念/预算/例外参数）；
2. `scripts/arch_guard.py` = fitness functions（层方向/新边/循环/env/尺寸/同名概念/扩展点/例外）；
3. `docs/adr/` = 结构变更唯一入口（O-6 三阶 + 契约影响 + 迁移/回滚 + 预算净额）；
4. `scripts/arch-baseline.txt` = 棘轮基线（356 条，只减不增）。

### 2.2 例外预算（六旋钮 + 四边界，全部按"棘轮只往紧走"裁决）

| 旋钮 | 裁决 | 拒的替代（理由） |
|---|---|---|
| 计量 | 权重分 | ❌ 条数：小额噪音（文档/尺寸）会吃掉配额，真结构问题反而被稀释 |
| 每波预算 | 3 分 | ❌ 0：迁移期合法中间态无处安放，逼出"拆两步/瞒报"；❌ 6：够堆 2 条依赖边，棘轮失效 |
| 活跃上限 | 2 条 | ❌ 1：并行线（安全线/W 线/docs 线）同时迁移会互相卡死；❌ 3：等于每次波内允许一次小绕行 |
| 存活期 | 2 波 | ❌ 按天：AI 施工线以波推进，按天会逼出最后一天突击清理；❌ 3 波：一个例外可横跨三个交付批，债务可视化失效 |
| 结构类预算 | 0 | ❌ 允许 W10 例外：契约可被例外蛀空，"方向违规"会常态化 |
| 特批 | ≤1 条、P0 安全/修复、不占每波权重、占活跃上限、ADR 记名+到期 | ❌ 无特批：P0 安全修复被流程卡死；❌ 特批不限期：等于第二预算池 |

边界裁定：
- **净零范围** = 依赖边 + 重复概念；env 新增是合法能力扩展，不要求净零，但必须登记 + 默认关 + 零变化测试；
- **不允许自动续期**：延期 = 新开一条 + 删旧条，占新波预算；
- **违规形态变化** = 新例外重新登记（防"重构洗白"）；
- **基线更新**仅波收口由 lead 执行，CI 校验 baseline diff 只减不增（防"更新基线"后门）；
- **批准分工**：结构类 owner、债务类 lead、W1 守卫自动；AI 施工线不得自批；
- **冻结窗口**：`release_freeze=true` 时预算 0，已有例外须在 tag 前清零。

### 2.3 灰度

CI / `commit.ps1` / `verify_changed.py` 先 **report-only**，至 **2026-11-03**（两周）后转 blocking；`verify_changed.py` 永远保持 report-only（快反馈），blocking 在 commit 与 CI 两处。

## 3. O-6 三阶审查

- **总体最优**：拒绝"继续靠人肉纪律/文档提醒"。项目已有 `docs_truth.py`（状态真源）与 `verify_changed.py`（快反馈）两条被验证过的机器闸模式；把同一模式复制到架构层，是当前路线图（v2.0.0-rc.4 内测 → 后续 R1-R7 重构）下成本最低、杠杆最高的切入点。拒的替代：①"先重构再立规矩"——重构期正是例外最多的时候，没有棘轮会把临时结构永久化；②"一次性大重构后冻结架构"——与 O-6 冲突，且 31 members 的演进不会停止。
- **系统最优**：契约放在 `docs/architecture/`（文档层），守卫放在 `scripts/`（工具层），基线放在 `scripts/`（同 `doc-caliber-baseline.txt` 族），ADR 放在 `docs/adr/`（决策层）——不新增 crate、不碰 runtime、不改任何生产代码路径，0 LOCKED 触碰。拒的替代：①把规则写进 `build.rs`/CI 硬编码——不可审计、不可增量；②新建 `apeireth-arch` crate——为检查器引入编译依赖，违反"0 引新外部 dep"与最小内核。
- **架构最优**：守卫只做只读扫描（cargo metadata + 文件正则），无状态、无网络、可在 fast gate/CI/本地三处复用；契约是纯数据（TOML），规则变更 = 数据变更 + ADR，不需要改代码。拒的替代：①自研 AST 分析器——维护成本远高于收益；②引入第三方架构 lint 工具——中性表述与依赖纪律不允许，且无法表达本项目特有约束（四层/扩展点/接线账）。

## 4. 迁移计划

1. **本批（Wave 1）**：契约草案 + 守卫 + 基线 + ADR 模板/准入清单 + 三处接入（report-only）。验证：`python scripts/arch_guard.py --strict` exit 0；反证：删基线一条边 → exit 1；到期例外 → exit 1。
2. **2026-11-03 前（灰度）**：每批跑 `--report`；不阻断；收集误报并修规则；基线不扩。
3. **2026-11-03 后（blocking）**：CI `--strict`、`commit.ps1 --strict`；`verify_changed.py` 保持 report-only。
4. **每波收口**：架构复盘 → 缩基线 → `current_wave += 1` → 写 wave 记录。

## 5. 回滚

- 代码回滚：删除本 ADR 引入的 9 个新增文件 + 还原 `.gitignore` 两行 + 停止 CI workflow（无需回滚任何生产代码）。
- 契约回滚：`contract.toml` 恢复上一版本；基线恢复上一版本（`git revert`）。
- 因本批 0 源码改动、0 LOCKED 触碰，回滚风险 = 0。

## 6. 验收

- [x] `python scripts/arch_guard.py --report` → `crates=31 edges=118 env=154 max_src_loc=6150`，基线 356 条
- [x] `python scripts/arch_guard.py --strict` → VIOLATIONS 0，exit 0
- [x] 反证①删基线一条边 → `[W3] edge` exit 1；反证②到期例外 → `[W10] exception-expired` exit 1
- [x] 0 源码改动 / 0 LOCKED 触碰 / 0 新外部 dep
- [x] 本 ADR + 模板 + 契约 + 准入清单落盘
- [x] 三处接入（verify_changed.py / commit.ps1 / CI）——灰度 report-only 至 2026-11-03（见迁移计划）

## 7. 后果

- **正面**：结构新增有唯一入口、有到期、有净零；架构收敛从"人肉纪律"变成"机器棘轮"；每波有可对比记分牌。
- **代价（接受）**：新增依赖边/扩展点要走 ADR，初期会慢半天到一天；灰度期误报需要维护规则。
- **后续 stage**：`contract.toml` 中 target 条款（R1 神经系统 / R2 扩展 ABI / R3 config / R7 上下文单一化）逐项出 ADR 裁决；`ARCHITECTURE.md` 改生成物（v0.2）；接线账与 claims 合并（v0.2）。
