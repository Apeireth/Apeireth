# 三线协作契约（架构线 × 效率线 × 生物线）

> `Status: ✅ 生效（2026-10-20；owner 转告三线）`
> 依据：[CONSTRUCTION-DISCIPLINE §二/§三](../04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（叠加不覆盖 / 战场不重叠 / 口径唯一）、[ADR-0001](../adr/0001-adopt-architecture-contract.md)、[contract.toml](contract.toml)。
> 一句话：三线共享一个工作树和一个 git index——**目标不冲突，冲突只在资源与文件接口**；本页把接口写死，机器能管的部分交给 `arch_guard`，机器管不了的（提交窗口/人读约定）写在这里。

## 1. 线分工

| 线 | 目标 | 主笔范围 | 不碰 |
|---|---|---|---|
| **架构线** | 契约/守卫/ADR/波次复盘/结构变更否决 | `docs/architecture/**`、`docs/adr/**`、`scripts/arch_guard.py`、`scripts/arch-baseline.txt`、`.github/workflows/arch-guard.yml` | 不改 `crates/**` 生产源码；不碰 LOCKED 5 项 |
| **效率线** | fast gate / 文档事实体系（DTS）/ claims / 验证架构 | `scripts/verify_changed.py`、`scripts/commit.ps1`、`scripts/docs_truth.py`、`docs/status/**`、`.github/workflows/docs-truth.yml` | 不改契约/守卫规则本身；不碰 LOCKED 5 项 |
| **生物线** | 机制实施 / R1 神经系统 / 全量门禁收口 | `crates/engine/organ/**`、`runtime-assembly/canonical` 生物接线、`docs/04-internal/backend-enhancement/**`（W 线卷宗）、[00-backlog](../04-internal/backend-enhancement/00-backlog.md) 相关行 | 不改契约/脚本；不碰 LOCKED 5 项 |
| 其它线（安全/总装等） | 各自交付 | 各自卷宗与源码域 | 同左纪律，尤其 §3/§5 |

## 2. 文件所有权矩阵（改前查表）

| 路径 | 拥有线 | 改动规则 |
|---|---|---|
| `docs/architecture/**`、`docs/adr/**` | 架构线 | 其它线只读；要改先提 ADR/评审，归架构线落笔 |
| `scripts/arch_guard.py`、`scripts/arch-baseline.txt`、`.github/workflows/arch-guard.yml` | 架构线 | 基线只许波收口缩表（ADR-0001 §2.2） |
| `scripts/verify_changed.py`、`scripts/commit.ps1` | **效率线** | 其中 `# arch-hook (architecture line)` 标记块归架构线；效率线重构可移动但**不得删除/绕过**，改钩子需两线互认 |
| `scripts/docs_truth.py`、`docs/status/**` | 效率线 | 生成物禁手改（`STATUS.md` 由 `--gen` 产出） |
| `.gitignore` | 共享 | 新增例外行须在 commit 正文注明用途；**任何线不得删除他人已加的例外行** |
| `crates/**` 生产源码 | 各实施线 | 架构线不改；跨线同文件走 hunk 级分账（宪章 §二） |
| `docs/03-reference/test-baseline.md` | **跑全量门禁的线**（当前生物线） | 只有全量实测者更新并广播；其它线只引用不抄写 |
| `docs/04-internal/backend-enhancement/**` | 生物线/安全线/总装线各卷宗 | 跨卷宗引用不断链；架构线只读 |

## 3. 提交纪律（共享 index 硬规则）

1. **禁止 `git add -A` / `git add .`**；只 `git add` 本线文件（显式路径）。
2. 提交前必跑 `git diff --cached --name-only` 自检；出现非本线文件 → abort 并 unstage。
3. **staged 不跨批过夜**；长时间 staged 前先 `git restore --staged`。
4. **同一时刻只允许一条线执行 commit**；拿不准就等一分钟重试（index 是共享单点）。
5. 跨线同文件改动 → **hunk 级分账**（`git apply --cached` 选择性暂存），commit 正文注明"与 X 线分账"。
6. 提交入口统一走 `commit.ps1`（secret/neutral/arch 三关；**arch 关默认 strict 阻断**，显式 `-Report` 才降级为仅告警；`-ArchStrict` 为历史兼容 no-op）。

## 4. 构建与门禁纪律

- **fast gate 随时跑**；**全量三件套只在 wave 收口/夜间跑**（实测 >18 分钟，且会与 fast gate 抢构建锁：`Blocking waiting for file lock` 已多次出现）。
- `arch_guard` 只读 metadata + 文本（3-5 秒、不碰构建锁），任何线可随时跑。
- CI 各 workflow 相互独立；`arch-guard.yml` 与 `docs-truth.yml` 并行不冲突。
- **谁跑全量，谁负责**：更新 `test-baseline.md` 单一真源并在波次群广播；不广播 = 数字漂移。

## 5. 真源拆分（禁止双重手写）

| 真源 | 拥有线 | 生成物/消费者 |
|---|---|---|
| `docs/architecture/contract.toml`（结构规则） | 架构线 | `arch_guard`；v0.2 起生成 `ARCHITECTURE.md` |
| `docs/status/claims.yaml`（能力状态） | 效率线 | `STATUS.md`（`--gen`）、CI |
| `docs/03-reference/test-baseline.md`（测试数字） | 全量门禁执行线 | 徽章/文档引用 |
| wiring ledger（R3，待建） | 效率线造生成器、架构线定 schema | 生成物；与 claims/contract 的 join |
| 规则 | 同一数字/状态只能有一个真源；其它文档**只引用不抄写**（0装纪律 + AGENTS §3） |

## 6. 波次与冻结日历

- **门禁等级（2026-11-05 实测）**：灰度期（report-only + `continue-on-error: true`）已于 **2026-11-04** 到期结束 ⇒ CI `arch-guard.yml`（`--strict`）、CI `wiring-ledger.yml`（R3c `--crosscheck --strict`）与本地 `commit.ps1`（默认 `--strict`）全部 **blocking**，均无 `continue-on-error`。**fast gate（`verify_changed.py`）的 arch 段仍 report-only**（ADR-0001 §2.3 / ADR-0011 的明确设计，非漏改）。〔历史口径（2026-10-20）：原定灰度至 2026-11-03 止〕
- **每波收口**：架构复盘（架构线主持，各线参加）→ 基线缩表 → `current_wave += 1` → 记 `wave_note`。
- **发版冻结**：`contract [exceptions].release_freeze = true` 由 **owner / release authority 宣布**；冻结期例外预算 0，各线停止新增例外、tag 前清零。架构线**不得单方面冻结**。
- **契约条款变更**：只在波收口做，不在他线施工中途改；紧急安全例外走 ADR + owner。

## 7. 冲突升级路径

1. 文件冲突：后到者 rebase / 最小改动，不覆盖他人在途（宪章 §二.1）。
2. 接口争议：先查本页；本页未写 → 架构线出 ADR 草案 + owner 裁决。
3. 共享资源争用（提交窗口 / 全量门禁窗口）：owner 调度。
4. 安全/发布相关：release authority 一票。
5. **任何线不得以"赶进度"绕过**：LOCKED 5 项 / 0装诚实 / 四级口径 / 本页纪律。违反 = 架构例外，按 `exceptions.toml` 登记，到期即红。

## 8. 生效与变更

- 本页 2026-10-20 生效；三线各存链接（owner 转告）。
- 变更：架构线主笔 + 受影响线互认 + owner 知会；变更记录进 ADR 索引。
- 违反本页的处置：登记例外 → 到期未清 → `arch_guard --strict` 拦截。

## 9. 重启登记（2026-10-21，Wave 4 收口后）

两条已收尾/停机的线由 owner 重启，开工入口为本页 + 各自重启简报（仓库内唯一入口，勿另口头传递）：

| 线 | 收尾/停机 | 重启简报 | 本轮任务 |
|---|---|---|---|
| 效率线（第三线） | 2026-10-20 收尾（`docs/status/CLOSEOUT-2026-10-20.md`） | [restart-brief-efficiency-2026-10-21.md](restart-brief-efficiency-2026-10-21.md) | E1 test-baseline 维护/全量执行权接管；E2 R8-D wiring 余项；E3 R6-D 夜间 full 首跑；E4 R6-C 校验器接门禁；E5 边界 |
| 安全线 | 2026-10-20 13:50 停机交接（`artifacts/security-line-handover-2026-10-20.md`） | [restart-brief-security-2026-10-21.md](restart-brief-security-2026-10-21.md) | S1 R8-B2 CLI 装配根切换；S2 R8-B3 持久 AuditSink + IM 审计对；S3 自有残项先报 owner；S4 边界 |

- 当前基线（2026-10-21）：`HEAD=5f6ee7da`、`wave=5`、全量 341 套件 / 8570 passed / 0 failed / 51 ignored
  （`artifacts/wave4-close-full-2026-10-21.log`）、guard VIOLATIONS 0 / exceptions=2 / STALE 0、
  `docs_truth` / `arch_doc` / `wiring_report` 均绿。
- 归属原则不变：契约/守卫/基线/ADR = 架构线；测试真源执行 = 效率线；R8 实施 = 安全线；生物机制接线 = 生物线。
  架构线在 Wave 4 期间按 ADR-0011 §9 / ADR-0010 §9 的「归属临时调整」代做的部分（R6-A/B、R8-A/B1/C、
  R1b-1a/1b/3/4a），自本页生效起回归各线，不再扩大架构线施工面。
- R6-C/D 的**四层签核只能 owner 本人签**（`verification-ledger.yaml` signoff），任何线不得代签。

## 10. 跨线分叉现状登记（2026-11-05 实测订正）

⚠️ **本页登记的是「两条线已经分叉多深」的事实**；**不预判怎么合**（那是 owner 决策）。
⚠️ **本节已于 2026-11-05 20:20 订正** —— 初版把此事登记成「若某条分支先合主线，cap 注释需对齐」，
**低估了分叉深度**。订正依据是实测（`git merge-base` / `git diff --shortstat` / `git ls-tree`）。

### 10.1 两条线的基本事实（实测）

| 项 | 本线 `ui-engine-p0-2026-10-24` | 并行线（tip `7ed1e3ed`，`debt-round8` 已并入） |
|---|---|---|
| HEAD | `18888d19` | `7ed1e3ed`（2026-11-05） |
| 分叉点 | `8ac0f9de`（**2026-10-24**）—— 同一分叉点 | 同左 |
| 分叉后独有提交 | **107** | **625** |
| workspace members | **31**（30 crates + bench harness） | **30**（29 crates + bench harness） |
| 生产依赖边实测 | **118** | **111** |
| `max_production_edges` | 118（= 实测） | 111（= 实测，2026-11-05 收紧） |
| 门面 crate `apeireth` | **有**（ADR-0019，2026-10-31 加入） | **无**（`git ls-tree` 实测无 `crates/apeireth`） |
| 最后一次并入远端 | 2026-10-24 | 2026-11-05（多次并入 `debt-round8`） |

⇒ **31 vs 30 的由来已查清**：门面 crate `apeireth` **只在本线**（2026-10-31 由 ADR-0019 加入）⇒
本线多一个 member、多 7 条生产边 ⇒ **118 与 111 各自都是自己树上的真实实测**（不是谁抄错）。

### 10.2 分叉深度（实测，不是估计）

```
$ git diff --shortstat HEAD 7ed1e3ed
1414 files changed, 216783 insertions(+), 67706 deletions(-)
```

⇒ ⚠️ **这不是「改一个 cap 值」的量级**：两条线自 2026-10-24 分叉后**各自演化成实质不同的代码线**。
⇒ 并行线上有本线**没有**的工作（逐条实测其提交）：「**还原被覆盖的上游安全修复**」+
「修端口夹具竞态」、「删 19 条死 API（基线 119 → 103）」、「**能力可达率闸门**（69/132 = 52%）、
「记忆引擎模块图谱 132 条」、「修 fuzz target 用了已删 API」。
⇒ 也有**文件级冲突已可见**的例子：并行线**删除**了 `crates/adapters/cli/tests/
crypto_flag_contract.rs`（342 行），而该文件在本线**已被本轮旗标注释归位批修改过**。

### 10.3 待决点（owner 决策，本页不代决）

1. **两条线要不要合、怎么合** —— 这是**主线级**决策，量级远大于「cap 值对齐」。⚠️ 本页**只登记
   事实**，不推荐方案（那需要 owner 先定「主线取哪条」）。
2. **门面 crate `apeireth` 是否保留** —— 它是 31 vs 30 的唯一来源 ⇒ **保留**则并行线的 111 要改成
   118；**不保留**则本线的 118 要改成 111。⚠️ 该字段属**结构化字段**（棘轮上限）⇒ **改动须 ADR**
   （契约注释头第 ③ 条：允许集合只减不增，新增依赖边/开关/结构必须 ADR + 本文件变更）。
3. **并行线上那批「还原被覆盖的上游安全修复」是否必须进主线** —— 若必须，则合线**不能拖**。
4. ⚠️ **本线的 `crates-ui/**` 68 项在途改动**（未提交，另一条施工线在工作树上）⇒ 合线前须先让
   那条线收工或明确其归属。

⇒ **本线不代改 `max_production_edges` 的值**，也不代提 ADR —— 等 owner 决策。

- **2026-10-21 效率线重启批完成（架构线验收）**：`d2621788..3d6d3d44` 共 10 提交——E1 test-baseline 维护/全量执行权接管、E2 治理面登记（claims 20→24 均 IMPLEMENTED）、E4 台账校验接入 docs-truth；E3 夜间 CI 按 owner 裁决「不推送、全本地」收口为 NOT MET（未冒称夜间 CI）；门禁全绿，crates 零改动。安全线自 `d2621788` 起 0 提交，S1/S2 待启动；R6-C/D 正式验收仍需 owner 签核。
