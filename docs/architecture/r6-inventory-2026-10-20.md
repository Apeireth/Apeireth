# R6 验证架构现状取证包（ADR-0011 前置）

> 日期：2026-10-20（Asia/Shanghai）
> HEAD：`8cecb85a`（ADR-0010 R8 Accepted 之后）
> 契约状态：`current_wave = 4`，guard = wave 4 / crates 30 / edges 111 / env 154 / VIOLATIONS 0
> 口径：本文只记录**实测现状与缺口**，不发布测试通过数字；测试数字真源 = `docs/03-reference/test-baseline.md`（归全量门禁执行线）。

---

## §1 结论摘要

R6（验证架构）的问题**不是缺门禁，而是门禁语义不齐、口径出现第二真源、四层验收无载体**：

1. **三层门禁资产齐全，阻断语义不齐**：fast（`verify_changed.py`）/ arch（`arch_guard.py` + `arch_doc.py` + `docs_truth.py` + `wiring_report.py`）/ full（`gate_full.ps1`）都有载体；但 arch 段的 CI 两层（`arch-guard.yml`、`wiring-ledger.yml`）仍是 `continue-on-error: true` 灰度到 2026-11-03，本地 fast gate 的 arch 段默认 report-only（`--arch-strict` 才阻断）→ **当前交付路径上架构契约不阻断**。
2. **full 层不可重复、无成本分层**：本地无串行/互斥机制，Wave 3 收口实测两条线并发跑全量 → `-p apeireth-image-compose --lib` OOM（`memory allocation of 1048576 bytes failed`，test_exit=101）+ target 目录锁互等；CI 侧 `rust.yml` 在每次 push/PR 上跑 3 OS × 全 workspace nextest，而全仓**没有定时全量 workflow**（`schedule:` 仅 cargo-audit / image-family / pii-leak-detection）→「full = 夜间收口」NOT IMPLEMENTED。
3. **CI 守门自身是旧口径的第二真源**：`rust.yml` 的 hard-walls 仍写「24 LOCKED crate」「13 键」，与 ADR-0001/ADR-0006 现行 **LOCKED 5 项 / 14 键**不一致（CI 注释里自述「R148 (2026-08-13) 24 LOCKED 已形式撤销」，但守门代码未同步）。
4. **四层验收有定义、无执行载体**：`founding-design-v2.md:101` §7 定义了形式验证 / 基准复现 / 场景实测 / 主人签核四层门槛；现状 `claims.yaml`（20 条）只有 level / verified_at / verify_cmd，`live-verification-ledger.md` 只有绿表与挂账，**没有「层 → 证据 → 签核」字段**；阶段 6 最小闭环（清单当关卡 + 人工签核 + 反思归档）无现行文档/脚本载体（`docs/archive/stage6/` 为历史）。
5. **形式化面已阻挡但覆盖边界未登记**：`kani.yml`（22 harness 矩阵 + 汇总门 needs 聚合）与 `miri.yml` 均无 `continue-on-error`，即已满足 §7 ①「证明阻塞合入」；但「哪些模块/性质被证明、边界有界、哪些未覆盖」没有单一登记处，容易被读成「有 Kani = 系统已证明」。
6. **失败归因无口径**：环境故障（OOM/磁盘/target 锁）、flaky（nextest ci profile `retries 2`）、真实回归三者无分类输出；本地 fast/full 日志落 `logs/` 且 `/logs/` 被 `.gitignore:328` 忽略 → 失败现场不可入库复现。

**建议**：ADR-0011 定「三层语义 + 全量唯一执行器 + 数字单一真源 + 四层验收载体 + CI 口径单一真源 + 形式化边界登记」六条契约；实施归效率线（门禁脚本/CI 是其已持有资产），架构线只定 spec 与真源规则。

---

## §2 验证资产清单（实测）

| # | 层/面 | 载体 | 现状 |
|---|---|---|---|
| 1 | fast | `scripts/verify_changed.py` | 改动 crate + 反向依赖 check/clippy/nextest；默认 arch report-only；日志 `logs/verify-changed.log`（gitignored） |
| 2 | fast 收尾 | `pwsh scripts/commit.ps1 -MessageFile` | secret-scan / neutral / 按次启用 `.githooks`（不全局设 hooksPath） |
| 3 | arch | `scripts/arch_guard.py` | 层/边/循环/env/尺寸/概念/扩展点/例外；`--report` 默认、`--strict`、`--update-baseline` |
| 4 | arch | `scripts/arch_doc.py --gen/--check` | `ARCHITECTURE.generated.md` 由契约 + metadata 生成，CI 校验漂移 |
| 5 | arch | `scripts/docs_truth.py --verify/--check` | `claims.yaml` 单一真源；STATUS.md / README 徽章一致性 |
| 6 | arch | `scripts/wiring_report.py --verify` | `docs/status/wiring.yaml` 接线台账，drift ≠ 0 红 |
| 7 | full | `scripts/gate_full.ps1` | 主 workspace（排除 vendored 图像家族）+ 图像家族独立门禁；`-MainOnly/-ImageOnly/-DryRun/-Log` |
| 8 | CI | `.github/workflows/*.yml` 共 **29** 个 | 见 §3.3；`arch-guard` / `wiring-ledger` 灰度 report-only |
| 9 | 形式化 | `kani.yml` / `miri.yml` | Kani 22 harness 矩阵 + 汇总门；Miri unsafe crate matrix；均无 continue-on-error |
| 10 | 验收定义 | `docs/01-architecture/founding-design-v2.md:101` §7 | 四层验收制正式形态（定义有、载体无） |
| 11 | 台账 | `docs/04-internal/live-verification-ledger.md` | 实测绿表 + 挂账（无「层/签核」字段） |
| 12 | 数字真源 | `docs/03-reference/test-baseline.md` | 单一真源（现行 §1 为 pre-R4b 归档口径，待全量门禁执行线广播更新） |

---

## §3 现状台账

### §3.1 fast 层

- 范围：改动 crate + 反向依赖（`cargo check/clippy/nextest`），避免全量成本。
- 阻断：arch 段默认 report-only（设计于 R4a-1/ADR-0005，配合 2026-11-03 灰度）；`--arch-strict` 可手动升阻断。
- 证据可复现性缺口：`logs/` 被 `.gitignore:328` 整目录忽略 → fast gate 日志只在本机，不进仓库；对外引用时只能重跑。

### §3.2 arch 层

- 本地：`commit.ps1` 做 secret-scan + 中性词扫描（阻断）；`arch_guard.py` 经 commit.ps1 默认 report-only。
- CI：`arch-guard.yml`（`continue-on-error: true`，注释写明灰度至 2026-11-03 后改 `--strict`）；`wiring-ledger.yml`（同灰度）；`doc-caliber.yml` / `docs-truth.yml` / `rustdoc*.yml` 等为阻断 job。

### §3.3 full 层与 CI 成本

- 本地：`gate_full.ps1` 任一阶段失败退出码 1，日志落 `logs/gate-full.log`（gitignored）。
- CI：`rust.yml` = 3 OS matrix（ubuntu/windows/macos × stable）× `cargo build --workspace --tests --locked` + `cargo nextest run --workspace --profile ci --locked`，每次 push/PR 触发；另有 `hard-walls`（6 守门：24 LOCKED / version / R11 baseline / 13 键 / V1136 / companion 污染）与 `secret-scan`（scan-all + scan-history）。
- 定时：`schedule:` 仅 3 个 workflow（cargo-audit / image-family / pii-leak-detection），**无定时全量**。
- 并发：本地与 CI 均无门禁排队/互斥；Wave 3 收口实测两条线并发全量 → OOM 崩溃 + target 锁互等（见 §4 G2）。

### §3.4 数字与口径

- 测试数字真源：`docs/03-reference/test-baseline.md`；口径定义在 founding-design-v2 §7：运行时通过数 = `cargo test --workspace --all-targets --locked --no-fail-fast` 实际绿数，静态 `#[test]` 计数只作覆盖参考。
- 现行 test-baseline §1 = 336 套件 / 9076 passed / 52 ignored（2026-10-20 pre-R4b 归档口径；R4b/R7/R2 后需全量门禁执行线复跑广播）。
- 本文件不发布数字，仅记录「数字从哪来、谁维护」。

### §3.5 四层验收现状

| 层 | 定义门槛（§7） | 现有载体 | 缺口 |
|---|---|---|---|
| ① 形式验证 | Kani/TLA 证明阻塞合入；有界边界如实标注 | `kani.yml`（22 harness + 汇总门）、`miri.yml`、`research/verification/{kani,tla}` | 覆盖清单/边界无单一登记；「阻塞」已满足 |
| ② 基准复现 | 性能数字无重跑脚本不发布 | `benches/` 基准 harness、artifacts 历史冷启动报告 | 无 CI 基准门禁；无「数字 ↔ 脚本」登记 |
| ③ 场景实测 | E2E + 人工点击流，真机结论入台账 | `live-verification-ledger.md`（绿表/挂账）、live E2E 测试（`#[ignore]`） | 无「场景 → 证据 commit」结构化台账 |
| ④ 主人签核 | 真实人类批准，签名入验证台账（与 ledger 同源） | 无 | **无流程/字段/脚本** |

### §3.6 阶段 6 最小闭环

- 定义：清单当关卡 + 人工签核 + 反思归档（founding-design-v2 §7 正式形态）。
- 现状：`docs/archive/stage6/` 为历史卷宗；`scripts/r20-stage-6-th/` 只剩 `dedup_licenses.py`；**无现行载体**。
- 影响：能力宣称「已交付/已验收」缺最后一步程序保障，只能靠 owner 口头授权（0 装风险）。

### §3.7 O-6 锚守门

`o6-anchor.yml`：4 重守门（clippy 0 警告 / tests 0 失败 / 旧 path 不回流 / LOCKED 0 触碰），`fail-fast: true`，阻断。该 job 是当前最接近「O-6 可执行化」的既有资产，R6 不改其语义。

---

## §4 缺口（G1–G6）

- **G1 阻断语义不一致**：arch 层灰度 report-only（2026-11-03 到期）；fast 的 arch 段默认 report-only → 同一契约在本地/CI 两条路径上阻断性不同，交付者无法预期。
- **G2 full 不可重复、无成本分层**：本地无并发互斥（实测 OOM `test_exit=101` + target 锁互等）；CI 每次 PR 全量 ×3 OS，无夜间全量。全量成本/时延无度量口径。
- **G3 CI 口径第二真源**：`rust.yml` hard-walls 的「24 LOCKED crate」「13 键」与 ADR-0001/0006 现行口径不一致；CI 注释自述 24 LOCKED 已形式撤销但守门未同步。
- **G4 四层验收无载体**：claims.yaml（20 条：PRODUCTION WIRED 10 / IMPLEMENTED 9 / NOT IMPLEMENTED 1）缺 layer/签核字段；live-verification-ledger 缺结构化「层→证据」；阶段 6 闭环无脚本。
- **G5 形式化边界未登记**：22 harness + Miri 覆盖哪些模块、边界、未覆盖面，无单一清单；「有 Kani」易被误读为「已证明」。
- **G6 失败归因无口径**：OOM/环境/flaky/真实回归无分类；nextest ci profile `retries 2` 可能掩盖 flake；本地日志 gitignored 不可追溯。

---

## §5 边界（R6 做什么 / 不做什么）

- **做**：定三层门禁语义与唯一的 full 执行器口径；定测试数字单一真源与引用规则；定四层验收载体（字段/流程/脚本）spec；定 CI 口径单一真源规则（禁硬编码旧口径）；登记形式化覆盖边界；定失败分类与证据落盘规则。
- **不做**：不新建第二套门禁脚本（沿用/扩展效率线现有资产）；不改 2026-11-03 灰度日期（ADR-0001 决定）；不代全量门禁执行线发布测试数字；不在本 spec 里做 CI 改造实现（实施归效率线）。
- 与 R8/R1b 的关系：R8 管治理执行面、R1b 管神经系统，二者验收通过 R6 门禁体系体现，互不改语义。

---

## §6 约束（C1–C6）

- **C1** 单一执行器：full 层只允许 `scripts/gate_full.ps1`（或其在 R6-A 中扩展后的同一文件）作为命令串真源，任何文档/CI 不得手抄第二份全量命令。
- **C2** 数字单一真源：对外测试数字只引用 `test-baseline.md`；本契约不产生新数字口径。
- **C3** 口径派生：CI 守门中的 LOCKED / 键集 / R11 baseline 等值必须从 `contract.toml` / AGENTS.md / ADR 派生或直接引用，禁止硬编码历史口径。
- **C4** 灰度不改期：2026-11-03 前 arch 层保持 report-only；到期转 blocking 由架构线复核后执行（R3c/R6 共用该日历）。
- **C5** 实施归属：门禁脚本、CI workflow、日志规则的实施归效率线；架构线只定 spec 与真源规则，验收时复核。
- **C6** 平台口径：本机 Windows（pwsh）与 CI Linux 行为等价；工具链版本钉在 CI 文件，本地不另立口径。

---

## §7 批次与验收（R6-A..D）

| 批次 | 内容 | 归属 | 验收 |
|---|---|---|---|
| R6-A | 三层语义文档化 + full 唯一执行器 + 并发互斥（文件锁/排队）+ 失败分类输出 | 效率线（架构线定 spec） | 三层各自 exit code 语义表；并发两条全量不再 OOM/互等（实测日志）；失败分类字段落日志 |
| R6-B | CI 口径单一真源：hard-walls 的 24 LOCKED / 13 键改为引用现行真源 | 效率线 | CI 内零旧口径命中；守门值与 contract/ADR 一致性脚本校验通过 |
| R6-C | 四层验收载体：扩展 `claims.yaml`（layer/evidence/signoff）或新 `verification-ledger`，阶段 6 闭环脚本 | 效率线 + owner 签核 | 至少 1 条 claim 走通四层（形式/基准/场景/签核）；阶段 6 脚本产出反思归档条目 |
| R6-D | 成本分层：PR 只跑 fast/受影响面，full 转夜间（schedule）+ 收口/发版触发 | 效率线 | PR 平均时长与 full 夜间结果可追溯；夜间失败进台账 |

---

## §8 未决问题（Q1–Q6）

- **Q1** 夜间 full 的触发时区与频率（每日 once？main-only？）——建议每日一次 main + release/wave 收口手动触发，owner 确认。
- **Q2** 四层验收的签核载体：扩 `claims.yaml` 还是新 `verification-ledger.yaml`（建议新台账，claims 只留 level/证据，避免单文件过载）；owner 确认。
- **Q3** 阶段 6 反思归档的归档介质（handbook 附录 / 独立 artifacts / ledger 字段）——建议 ledger 字段 + 每波 handbook 复盘双写。
- **Q4** 基准复现的门槛：哪些性能数字必须进 CI（冷启动/主要 E2E 时延），哪些只按需重跑——建议先冷启动 1 项。
- **Q5** CI 3 OS 全量是否保留在 PR：建议保留 build 的 3 OS、nextest 拆为单 OS + 夜间 3 OS，owner 拍。
- **Q6** 失败重试策略：nextest `retries 2` 是否保留（掩盖 flake 风险 vs 环境抖动）——建议保留但 flake 必须开 issue 登记，不得静默。

---

## §9 证据命令

```powershell
# 三层门禁资产与 CI 面
git ls-files scripts/ | Select-String 'verify_changed|arch_guard|arch_doc|docs_truth|wiring_report|gate_full'
Get-ChildItem .github/workflows -Filter *.yml | Measure-Object   # 29
Select-String -Path .github/workflows/*.yml -Pattern 'continue-on-error'   # arch-guard / wiring-ledger
Select-String -Path .github/workflows/*.yml -Pattern '^\s*schedule:'       # cargo-audit / image-family / pii-leak-detection
Select-String -Path .github/workflows/rust.yml -Pattern '24 LOCKED|13 键'

# 四层验收与台账
Select-String -Path ../01-architecture/founding-design-v2.md -Pattern '四层验收|阻塞合入|主人签核'
python -c "import yaml;d=yaml.safe_load(open('docs/status/claims.yaml',encoding='utf-8'));print(len(d['claims']))"  # 20
git ls-files ../04-internal/live-verification-ledger.md docs/status/claims.yaml

# full 层并发实证（Wave 3 收口）
Select-String -Path artifacts/wave3-close-full-test2.log -Pattern 'memory allocation|Blocking waiting|test failed'
```
