# 0011. R6 验证架构——三层门禁语义 / 单一执行器 / 四层验收载体

- **Status**: Accepted（owner 2026-10-20 授权 Wave 4 按架构线建议推进；本文为 ADR-0002 已 Accepted 的 R6 方向的实施形状裁决，效率线实施期如发现条款不可行可在 R6-A 开工前提出修订）
- **Date**: 2026-10-20
- **Owners**: 架构线出 spec / 验收标准 / 契约面；R6-A..D 实施 = 效率线（ADR-0002 主笔线口径）
- **Supersedes**: —
- **契约影响**: `docs/architecture/contract.toml` §meta.status_note（追加 ADR-0011 一行）与
  §meta.target_architecture.waves（追加 "ADR-0011 R6（Wave 4）"）；`[extension_points]` 不新增登记
  （验证门禁是仓库资产，不是扩展点）；依赖层边 ±0 / crate ±0 / env ±0
- **例外关联**: —（本 ADR 只定 spec 与真源规则，实施期如需改 CI/脚本按效率线流程）
- **预算净额**: 无代码变更；实施期（R6-A..D）新增脚本/CI 改动由效率线在其批次内登记，本 ADR 不预占预算
- **证据包**: [r6-inventory-2026-10-20.md](../architecture/r6-inventory-2026-10-20.md)（HEAD `8cecb85a` 实测）

## 1. 背景与问题

R6「验证架构」的问题**不是缺门禁，而是门禁语义不齐、口径出现第二真源、四层验收无载体**
（证据包 §1-§4）：

1. **阻断语义不一致（G1）**：fast（`verify_changed.py`）/ arch（`arch_guard.py` + `arch_doc.py` +
   `docs_truth.py` + `wiring_report.py`）/ full（`gate_full.ps1`）三层资产齐全，但 `arch-guard.yml`
   与 `wiring-ledger.yml` 仍是 `continue-on-error: true` 灰度到 2026-11-03，本地 fast gate 的 arch
   段默认 report-only → 同一契约在本地/CI 两条路径阻断性不同，交付者无法预期。
2. **full 不可重复、无成本分层（G2）**：本地无并发互斥，Wave 3 收口实测两条线并发全量 →
   `-p apeireth-image-compose --lib` OOM（`memory allocation of 1048576 bytes failed`，test_exit=101）
   + target 目录锁互等；CI 侧 `rust.yml` 每次 push/PR 跑 3 OS × 全 workspace nextest，而全仓没有
   定时全量 workflow（`schedule:` 仅 cargo-audit / image-family / pii-leak-detection）→
   「full = 夜间收口」NOT IMPLEMENTED。
3. **CI 口径第二真源（G3）**：`rust.yml` 的 hard-walls 仍写「24 LOCKED crate」「13 键」，
   与 ADR-0001/ADR-0006 现行 LOCKED 5 项 / 14 键不一致；CI 注释自述 24 LOCKED 已形式撤销但守门代码未同步。
4. **四层验收无载体（G4）**：`founding-design-v2.md:101` §7 已定义形式验证 / 基准复现 / 场景实测 /
   主人签核四层门槛，但 `claims.yaml`（20 条）只有 level / verified_at，`live-verification-ledger.md`
   只有绿表，无「层 → 证据 → 签核」字段；阶段 6 最小闭环无现行载体（`docs/archive/stage6/` 为历史）。
5. **形式化边界未登记（G5）**：`kani.yml`（22 harness）与 `miri.yml` 均已满足「证明阻塞合入」
   （无 continue-on-error），但覆盖模块/性质/有界边界/未覆盖面无单一登记处，易被读成「有 Kani = 系统已证明」。
6. **失败归因无口径（G6）**：环境故障（OOM/磁盘/target 锁）、flaky（nextest ci profile `retries 2`）、
   真实回归三者无分类输出；`logs/` 被 `.gitignore:328` 忽略 → 失败现场不可入库复现。

**为什么现在决策**：R6 是 ADR-0002 Wave 4 出口三条之一。当前验证资产已够用，缺的是「语义与真源」
而不是新工具；再拖下去，每个新线都会按自己的理解调用门禁（本地 report-only、CI 全量、日志不可追溯），
「已验收」口径继续分叉。

## 2. 决策

### D1 三层门禁语义（fast / arch / full）

| 层 | 何时跑 | 跑什么 | 阻断性 | 产出 |
|---|---|---|---|---|
| fast | 本地每次改动 / PR 受影响面 | 改动 crate + 反向依赖 check/clippy/nextest（`verify_changed.py`） | 阻断（含 arch 段：灰度期内 report-only，2026-11-03 后 `--arch-strict` 升阻断） | `logs/verify-changed.log`；失败须复述到 CI/PR 或 artifacts |
| arch | 每次提交 / PR / CI | 契约守卫、生成物漂移、文档真源、接线台账（`arch_guard` + `arch_doc` + `docs_truth` + `wiring_report`） | 灰度期内 report-only；2026-11-03 后阻断 | 各脚本 exit code + 报告行 |
| full | 夜间（R6-D 后）/ wave 收口 / 发版前 / 手动 | 全 workspace 全量测试 + fmt + clippy（唯一命令串 = `scripts/gate_full.ps1`） | 阻断（收口/发版）；夜间失败当日修或开 issue | `artifacts/` 日期戳日志 + 测试数字唯一来源 |

**规则**：fast 不发布对外数字；对外测试数字只来自 full；arch 灰度日期由 ADR-0001 决定，本 ADR 不改期。

### D2 full 层唯一执行器 + 串行互斥

- `scripts/gate_full.ps1` 是 full 层**唯一命令串真源**（含主 workspace + 图像家族两段），
  任何文档/CI/交接不得手抄第二份全量命令（与 ADR-0005 R4a-1 一致）。
- **并发互斥**：扩展 `gate_full.ps1` 增加进程级文件锁（`logs/gate-full.lock`，拿不到锁则等待或
  `-FailIfBusy` 立即失败）；同一时刻全仓只允许一个 full 实例运行。
- **失败分类**：脚本输出 `FAILURE_CLASS`（`oom` / `env` / `flaky` / `regression`）并落日志；
  分类只能由证据判定（OOM=分配失败行、env=锁/磁盘/网络、flaky=重跑通过且无代码改动、其余=regression）。
- 实施归效率线（R6-A），架构线验收。

### D3 测试数字单一真源

- 对外数字 = full 层 `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` 的
  **运行时通过数**（套件数 / passed / ignored），记录到 `docs/03-reference/test-baseline.md`，
  由**全量门禁执行线**维护（line-coordination.md §2）。
- 其他文档/README/ADR/commit 正文**只引用不抄写**数字；静态 `#[test]` 计数只可作覆盖参考，
  不得与运行时通过数混记（founding-design-v2 §7 口径）。
- 数字失效条件：归档/新增 crate、测试增删、门禁口径变更——任一发生，下一次 full 必须重取真源。
- 本 ADR 不发布任何测试数字（0 装）。

### D4 四层验收载体（founding-design-v2 §7 的可执行形态）

新增 `docs/status/verification-ledger.yaml`（由效率线实现脚本读写；架构线定 schema），每条登记：

```yaml
- id: V-<area>-<n>
  claim: <关联 claims.yaml id 或能力名>   # 可空：纯验证条目
  layer: formal | bench | scenario | signoff
  level: IMPLEMENTED | PRODUCTION WIRED | DEFAULT ENABLED | HARDWARE VALIDATED | NOT IMPLEMENTED
  evidence: [<commit / 测试文件 / 日志>]
  verify_cmd: <可复现命令>
  bounded: <形式验证的有界边界；非 formal 层填 ->（n/a）>
  signoff: { owner: <owner>, date: <YYYY-MM-DD>, note: <一句> }   # 仅 signoff 层必填
  verified_at: <commit hash>
```

- **四层缺一不可**：能力宣称「已交付/已验收」必须四层齐全；`level` 默认口径不变（四级 + NOT IMPLEMENTED）。
- **签核**：`signoff` 层必须是真实人类批准（owner 或授权人），签名只记录 owner 标识与日期，
  不入任何密钥；无签核的能力最高只能标 `PRODUCTION WIRED`，不得宣称「已验收」。
- **阶段 6 最小闭环**：清单当关卡（ledger 必须有该能力条目）+ 人工签核（signoff 字段）+
  反思归档（`docs/architecture/handbook.md` 每波复盘引用 ledger id）。
- **与现有台账关系**：`live-verification-ledger.md` 保留为叙述型绿表（历史连续），
  `verification-ledger.yaml` 是**结构化真源**；新条目写 yaml，叙述文档只引用。

### D5 CI 口径单一真源

- `rust.yml` 的 hard-walls 守门不得硬编码历史口径：
  - 「24 LOCKED crate」→ 引用 `contract.toml [locked]`（现行 LOCKED 5 项，ADR-0001/0006）；
  - 「13 键」→ 引用 14 键真源（`core/src/philosophy.rs`，ADR-0006）；
  - R11 baseline 三值（0.8682/0.8532/0.9063）与 workspace.version 规则保持（LOCKED 项，数值本身不变）。
- 实施方式：CI 侧读契约/脚本文本或调用小脚本取值，禁止在 workflow 内手抄清单（R6-B）。
- 校验：新增 `docs_truth.py` 或独立检查，断言 workflow 内无旧口径字面量。

### D6 形式化覆盖边界登记

- 登记为 `docs/status/formal-coverage.md`：Kani harness（22）→ 被测模块/性质 → 有界边界 →
  未覆盖面；Miri crate 列表 → unsafe 集中点。
- 已满足「证明阻塞合入」（`kani.yml` 汇总门 needs verify，无 continue-on-error），本 ADR 保持不降级。
- 新 unsafe 集中 crate 或新形式化目标须同时更新该清单（否则 `docs_truth` 漂移检查红）。

### D7 归属与批次

| 批次 | 内容 | 归属 | 验收 |
|---|---|---|---|
| R6-A | D1 语义表 + D2 唯一执行器/文件锁/失败分类 | 效率线 | 并发两条全量不再 OOM/互等（实测日志）；失败分类字段可见 |
| R6-B | D5 CI 口径单一真源 | 效率线 | workflow 内零旧口径命中；一致性检查通过 |
| R6-C | D4 四层验收载体 + 阶段 6 闭环 | 效率线 + owner 签核 | 至少 1 条 claim 走通四层；阶段 6 产出反思归档条目 |
| R6-D | 成本分层（PR fast / 夜间 full） | 效率线 | PR 平均时长与夜间结果可追溯；夜间失败进 ledger/issue |

### 不改什么

- 不改 2026-11-03 灰度日期（ADR-0001）；不新建第二套门禁脚本/平台；不替代 fast gate 作者归属；
  不在本 spec 内实施 CI 改造；不改 O-6 锚守门语义（`o6-anchor.yml` 保持 fail-fast 阻断）。

## 3. O-6 三阶审查（各含拒的 alternatives + 拒理由）

- **总体最优**：沿用三层现有资产并只补语义/真源/载体，不建新平台。
  拒：新建统一验证平台（重复造轮子、与效率线资产冲突）；拒：把所有门禁塞进单脚本（违反分层反馈速度）。
- **系统最优**：D1-D6 覆盖「何时跑/跑什么/阻断否/数字从哪来/验收怎么载/口径谁为准」六问，
  每问一条规则且都有归属批次；失败分类让 OOM/flaky 不再与真回归混记（Wave 3 收口实证的教训）。
  拒：只修 CI 旧口径（G3）不解决 full 不可重复（G2）——后者是反馈成本与误判主因。
- **架构最优**：spec 归架构线、实施归效率线（其持有脚本/CI 资产，line-coordination.md §2）；
  四层验收载体放 `docs/status/` 与 claims 同级，避免第二套状态体系。
  拒：架构线直接改 workflow（越界且与效率线在途改动冲突）；拒：把四层验收写进 claims.yaml
  单文件（状态声明与验收证据职责不同，单文件过载）。

## 4. 迁移计划（分步、每步可验证）

1. **R6-A**：`gate_full.ps1` 加锁 + 失败分类 + D1 语义表落入 `docs/02-guides/verification-architecture.md`
   → 实测两条 full 并发（第二条等待/失败而非 OOM）→ 提交。
2. **R6-B**：`rust.yml` hard-walls 改引用真源 + 一致性检查脚本 → CI 内 `grep '24 LOCKED|13 键'` 零命中。
3. **R6-C**：`verification-ledger.yaml` schema + 脚本 + 1 条样例（形式层=kani harness、场景层=live ledger
   条目、签核层=owner 签名）→ `docs_truth --verify` 纳入校验。
4. **R6-D**：PR 拆 fast / 夜间 full workflow（schedule）→ 两个 PR 周期后看时长与 flake 记录。
5. **收口**：Wave 4 收口复核 D1-D6 落地状态，未落地项如实标 NOT IMPLEMENTED 进 ledger。

## 5. 回滚

- 每批次独立提交，回滚 = `git revert <batch-commit>`。
- `gate_full.ps1` 加锁新增参数默认兼容（无 `-FailIfBusy` 时等待，不改变既有调用语义）。
- workflow 改动回滚后旧口径仍可运行（CI 只是变回硬编码，不引入不可恢复状态）。
- `verification-ledger.yaml` 为新增文件，删除即回滚，无迁移数据损失。

## 6. 验收

- [x] `docs/02-guides/verification-architecture.md` 存在且与 D1 表一致（R6-A） — 2026-10-21 复核：已满足（`5858fc1f`）。
- [x] `gate_full.ps1` 并发实测：两实例不同时跑，第二条等待或 `-FailIfBusy` 失败（R6-A） — 2026-10-21 复核：已满足（`5858fc1f`）。
- [x] 失败日志含 `FAILURE_CLASS`，OOM 场景实测识别为 `oom`（R6-A） — 2026-10-21 复核：已满足（`5858fc1f`）。
- [x] `rust.yml` 内 `grep -E '24 LOCKED|13 键'` 零命中且守门仍通过（R6-B） — 2026-10-21 复核：已满足（`5858fc1f`）。
- [ ] `verification-ledger.yaml` 至少 1 条四层齐全 + `signoff.owner/date` 非空（R6-C；
      载体 2026-10-21 已落，schema + 5 条单层条目；四层齐全 0 条、signoff 0 条，等 owner 签核）
- [ ] 阶段 6 闭环演练一次：清单关卡 + 签核 + 反思归档条目（R6-C；**NOT RUN**，须 owner 参与）
- [ ] 夜间 full workflow 存在且有最近一次结果可追溯（R6-D；载体 2026-10-21 已落
      `.github/workflows/nightly-full.yml`；**2026-10-21 E3 首跑尝试阻塞**：远端默认分支
      `main=fb68f5d3` 落后本地 194 提交、默认分支 workflow 列表无此文件，`workflow_dispatch`
      API 返回 404（`.git-credentials` 的 PAT 有效且 scope 含 repo/workflow，`/user`=200，
      但默认分支缺载体时 dispatch 不可用）；本地 YAML `safe_load` OK，首跑仍 0 次 →
      可追溯性 **NOT MET**，不勾选、不伪造 CI 结果；**2026-10-21 owner 裁决「不用推送，
      都本地就行」→ E3 按 NOT MET 收口**，本地全量实跑证据保留：`logs/gate-full.log` +
      E1 权威复跑 341/8570/0/51（见 test-baseline §1），但不计作夜间 CI 可追溯）
- [x] `docs/status/formal-coverage.md` 存在并登记 Kani 终局矩阵 11 件 / TLA / Miri /
      未覆盖边界（D6；2026-10-21 由架构线落，并订正 workflow「22 件」旧口径为 11 件）
- [x] 文档真源门禁覆盖 ledger 校验（R6-C 脚本化，归效率线；**2026-10-21 E4 落地**：
      `docs-truth.yml` 增 setup-python + `python scripts/verify_ledger.py --quiet` 步骤，
      本地/CI 口径 exit 0；`docs_truth.py` 本体不内嵌（保留其无 PyYAML 回退模式），
      门禁真源 = CI 步骤；本地实测 exit 0（远端 CI 首跑仍受 E3 同源阻塞：本地提交未推），
      负向测试：改坏 summary 计数 / evidence 路径 → exit 1 → 恢复 → exit 0）

## 7. 后果（Consequences）

- **正面**：门禁语义可预期；对外数字只有一条来源；「已验收」有结构化证据与人类签核；
  CI 不再制造旧口径第二真源；OOM/flaky 与环境故障可区分。
- **负面/代价**：R6-A..D 全部落在效率线，排期紧张时本 spec 只能部分落地——届时按 D7 表逐项
  如实标 NOT IMPLEMENTED，不得口头宣布完成。
- **风险**：夜间 full 与 wave 收口可能撞车（同一 target 目录），文件锁与 `-FailIfBusy` 用于防撞；
  ledger 条目的维护成本需在每波收口检查一次，防止「只建表不更新」。

## 8. 未决问题（效率线 R6-A 开工前裁决）

- **Q1** 夜间 full 的触发时区/频率（建议每日 1 次 main-only + workflow_dispatch，owner 拍）。
- **Q2** 四层验收台账载体：新建 `verification-ledger.yaml`（本文建议）vs 扩 `claims.yaml`（owner 拍）。
- **Q3** 阶段 6 反思归档介质：ledger 字段 + handbook 每波复盘双写（本文建议）。
- **Q4** 基准复现首批进 CI 的数字（建议冷启动 1 项；其余按需重跑并登记）。
- **Q5** CI 3 OS 全量是否保留在 PR（建议保留 build 3 OS、nextest 拆单 OS + 夜间 3 OS）。
- **Q6** nextest `retries 2` 是否保留（建议保留但 flake 必须开 issue 登记，不得静默）。

## 9. 变更记录

- **2026-10-21（R6-A/B 实施归属临时调整）**：ADR 生效后效率线无开工记录（2026-10-20 21:00 至
  2026-10-21 08:37 零提交、工作树无其改动），Wave 4 收口被 R6 实施阻塞。经 owner 先前授权
  "按你的建议来 / 一直做到底"，**R6-A（唯一执行器/文件锁/失败分类）与 R6-B（CI 口径单一真源）
  由架构线实施**，理由：①`gate_full.ps1` 本就是架构线 R4a-1 交付的资产、D2 只是扩展；
  ②`rust.yml` 无其他线在途改动；③不触碰效率线拥有的 `verify_changed.py` / `commit.ps1` /
  `docs_truth.py` / claims 流程。**R6-C/D 仍归效率线**（需要其脚本与 owner 签核流程）。
  恢复方式：效率线接管时直接以本批 commit 为起点继续，所有权在 handbook 裁决位更正。
  拒：让 R6 无限等待（Wave 4 无法收口）；拒：绕开 ADR 直接改 CI（本记录即口径）。

- **2026-10-21（R6-C/D 载体批，架构线落载体，验收仍归效率线 + owner）**：R6-A/B 提交后
  效率线仍无开工记录；为避免 Wave 4 收口长期悬空，架构线只落**载体**，不动 PR 门禁口径、
  不代替 owner 签核：
  ① `docs/status/verification-ledger.yaml`（D4 schema：id/claim/layer/level/evidence/
  verify_cmd/bounded/signoff/verified_at；5 条单层条目 = formal×2 + scenario×3；
  **四层齐全 0 条、signoff 0 条、阶段 6 演练 NOT RUN**——需要 owner 真实签核才能达标）；
  ② `docs/status/formal-coverage.md`（D6：Kani 终局矩阵 11 件逐件登记、matrix 外
  harness 一律按「未覆盖/不阻塞」口径、TLA 本机不入 CI、Miri 面与限制、明确未覆盖清单）；
  ③ `.github/workflows/nightly-full.yml`（R6-D 载体：cron 每日 UTC 18:00 + dispatch，
  windows-latest 跑 `gate_full.ps1 -FailIfBusy`，日志 artifact；**尚未有 CI 运行 = NOT RUN**）；
  ④ 顺带订正 `.github/workflows/kani.yml` 的「22 件」旧口径注释与汇总门文案为 11 件
  （2026-10-13 K18 终局矩阵实况；历史口径保留说明，避免第二真源）。
  **不做**：PR 层降级（rust.yml 每次 PR 全量 nextest → PR fast 子集 + 夜间 full）需要
  与全量门禁执行线排期，且会改变对外数字产生路径；`docs_truth.py` 对 ledger 的脚本化
  漂移校验、夜间结果回填、owner 签核流程均归效率线 R6-C/D 正式验收。
  拒：把「载体已落」写成「R6-C/D 已验收」（0 装）；拒：架构线代签 owner signoff。
- **2026-10-21（R6-C 载体补强：台账只读校验器）**：新增 `scripts/verify_ledger.py`
  （只读；校验 schema/证据路径存在性/非 signoff 层 signoff 必须 null/无 signoff 不得
  HARDWARE VALIDATED/summary 计数一致性/同 claim 同层唯一），实测 `exit 0`
  （发现并修正 `summary.claims_with_entries` 4->5 的真实漂移）；`.gitignore` 加
  `!scripts/verify_ledger.py`（`*.py` 忽略规则例外，与 arch_guard/arch_doc 同法）。
  仍归效率线：写入脚本、`docs_truth` 集成、夜间结果回填与 owner 签核。

- **2026-10-21（E3 R6-D 夜间首跑阻塞，效率线重启批）**：按 restart-brief E3 尝试触发
  `.github/workflows/nightly-full.yml`（`workflow_dispatch`，`main_only=true`）未成，阻塞证据：
  ①远端默认分支 `main=fb68f5d3`（2026-10-18），本地 `main` 领先 194 提交，夜间载体只在本地
  提交链；②`GET /repos/Apeireth/Apeireth/actions/workflows` 返回 26 个默认分支 workflow，
  无 `nightly-full`；③`POST .../actions/workflows/nightly-full.yml/dispatches` = HTTP 404；
  ④凭据侧不阻塞：`.git-credentials` 内 PAT 有效（`/user` 200，scope 含 `repo`/`workflow`），
  但默认分支缺载体时 dispatch API 不可用。**结论：首跑 0 次、无 run URL/artifact，不回填伪造
  数字**；四问如实——覆盖率未测（99% 未覆盖口径未验证）、1200s wall 未实测（本地 full 最近
  实跑约 14.8 分钟、非夜间 CI）、连续绿 0 次/绿分 0、失败兜底与通知未验证。`nightly-full.yml`
  本地 `yaml.safe_load` OK（on=schedule/workflow_dispatch，job=nightly-full-gate）。
  E4（ledger 门禁）不受阻，照做；不改 `rust.yml` PR 门禁口径。
  **2026-10-21 owner 裁决：不用推送，都本地就行** → E3 按 NOT MET 收口，不再尝试远端
  dispatch；本地全量实跑（`logs/gate-full.log`，E1 权威复跑 341/8570/0/51）保留为替代
  证据，但不计作「夜间 CI 可追溯」。后续 owner 改变决定时可原样重启 E3。

- **2026-10-21（E4 四层验收台账接门禁，效率线重启批）**：`docs-truth.yml` 增
  `setup-python@v5` + `python -m pip install --quiet pyyaml` +
  `python scripts/verify_ledger.py --quiet` 步骤，paths 增 `scripts/verify_ledger.py`；
  门禁真源 = CI 步骤（`docs_truth.py` 本体不内嵌该校验，保留其无 PyYAML 回退模式）。
  本地负向测试（2026-10-21 实跑）：基线 `[verify_ledger] OK: errors=0 warnings=0 infos=5`
  exit 0；改 `summary.entries 5→6` → `summary.entries=6 与实况 5 不一致` exit 1；恢复后
  evidence 路径改不存在 → `V-formal-002: evidence 路径不存在` exit 1；恢复 → exit 0、
  文件 SHA-256 与备份一致、`git diff` 仅头部注释行。**signoff 0 条不变、不代签**；
  owner 签核与阶段 6 演练仍 NOT RUN / NOT MET。
