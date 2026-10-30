# harness 审计（2026-10-30）—— **手写副本清单** ＋ **workflow「SKIPPED 是否等于 PASS」语义**

[Document-Meta]
Document: docs/status/harness-audit-2026-10-30.md
Class: AUDIT
Status: 🟢 活跃
Last-Modified: 2026-10-30
Author: audit-bot

> **审计对象**：本仓的**规则副本**（同一事实被手写多份的地方）＋ **42 个 CI workflow 的状态语义**。
> **审计口径**：分支 `debt-round8`，HEAD = `4a42d9da`；`.github/workflows/` = **42** 个 `.yml`（实测 `ls .github/workflows/*.yml | wc -l` = 42）。
> **方法**：只读 —— `grep` / `glob` / 逐文件读；**未运行** cargo build / nextest / 全量套件。
> **复核命令**见 §五。**未验证**项已逐条标注，不猜数字。

---

## 〇、三句话结论

1. **CI 侧已经去手抄，真源是 `docs/architecture/contract.toml`**（`[locked]` / `[workspace]` / `[ci_gates]` 三节）；`rust.yml:118-135` 明确写着「**禁止在本 workflow 手抄路径清单（D5：禁硬编码）**」——这一条是**已修**的。
2. **人读侧仍有真副本**，且**多份已陈旧**：`.github/PULL_REQUEST_TEMPLATE.md` 之外的 `Cargo.toml` 元数据段、`.github/workflows/o6-anchor.yml`、两篇 status 文档都还在手写已被契约取代的旧数字。
3. **workflow 侧（CI 层）没有聚合裁决层**：42 个 workflow **零**跨 workflow 结论汇总；CI 内唯一的聚合门是 `kani.yml` 的 `kani-required-gate`（**workflow 内** job 聚合）。**被 path 过滤跳过的 workflow 不产出任何机器可读结论** ⇒ 「SKIPPED 等于 PASS」在 15 个可被路径跳过的 workflow 上**事实上成立**。本地侧已有 `scripts/harness.py merge-decision`（**并发施工中、未接 CI**，见 §3.1）。

---

## 一、手写副本清单

> 「真源」＝ 单一权威来源；「真副本」＝ 同一值被手写 ≥2 处 ⇒ 会漂。
> 下表只列**在活跃面**（`docs/`、`scripts/`、`.github/`、根 `Cargo.toml`）的手写副本；`docs/archive/`、`legacy/`、`reports/` 属历史卷宗，不计入。

| 事实 | 值 | 手写副本位置（文件:行） | 漂移风险 | 建议（真源应该是哪个文件） |
|---|---|---|---|---|
| **受锁源码清单（LOCKED 文件）** | 3 个 `.rs`：`crates/foundation/core/src/eight_anchors.rs`、`onion.rs`、`philosophy.rs` | **真源**：`docs/architecture/contract.toml:37-45`（`[locked]`），CI 读取处 `.github/workflows/rust.yml:118-135`（tomllib + 「禁止在本 workflow 手抄路径清单」）· **真副本（已生成化）**：`.github/PULL_REQUEST_TEMPLATE.md:24-27`（由 `scripts/gen_gate_manifest.py` 生成，`:219-234` 校验文件存在）· **残留真副本（陈旧）**：`.github/PULL_REQUEST_TEMPLATE.md:109`「不改 LOCKED 24 crate」、`Cargo.toml:130`「见 `docs/conventions/10-locked.md` + `docs/omnibus/24-locked-crates.md`」（**两路径均已归档**，实为 `docs/archive/conventions/`）、`Cargo.toml:132` `locked_crates_count = 24` | 🔴 高：24 crate 名单 R148 已形式撤销，PR 模板仍要求「不改 24 crate」 | **已修**（CI 侧零手抄）· **未修**：PR 模板 L109 与 `Cargo.toml:130,132` 的 24-crate 字面量 ⇒ 建议改为契约生成，或按「历史口径（日期）」标注 |
| **键集数量** | **14 键**（PHL-01..08） | **真源**：`crates/foundation/core/src/philosophy.rs:107`（`ALL_THIRTEEN_KEYS: [PhilosophyKey; 14]`）、`crates/foundation/core/src/philosophy.rs:162`（编译期断言「…PHL-08 = 14」）、`docs/architecture/contract.toml:44-45`、规范 `AGENTS.md:77`、`ROADMAP.md:68` · **真副本（陈旧）**：`.github/workflows/o6-anchor.yml:99,100,106,109,142,168`「13 键 LOCKED」、`Cargo.toml:75`「13 键」、`Cargo.toml:127`「A3 12 键 + PHL-07 = 13 键」 | 🔴 高：`o6-anchor.yml` 是 **CI 守门**，它按 13 键判 ⇒ 与代码里的 14 键不是同一个集合 | **未修** ⇒ 建议：键数从 `philosophy.rs` 或契约 `[locked]` 读，CI 禁止手写数字（与 D5 同原则） |
| **哲学锚数量** | **9**（S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5/O-6） | **真源**：`Cargo.toml:136` `philosophy_anchors`（9 项）、`AGENTS.md:77`「9 哲学锚本体」· **真副本（陈旧）**：`.github/PULL_REQUEST_TEMPLATE.md:46,88,92,111,140,169`「6 哲学 anchor」×6 处 | 🔴 高：模板要求人只穿透 6 条锚，实际有 9 条 ⇒ 人按模板勾选 ⇒ 漏 3 条 | **未修** ⇒ 建议：模板锚编号由契约/生成器产出（同 `gen_gate_manifest.py` 模式） |
| **workspace 成员数** | 30 members = 29 crates + 1 bench harness | **真源**：`docs/architecture/contract.toml:31-33`（`members/product_crates/bench_crates`）· **真副本（现值一致，共 9 处）**：`AGENTS.md:53`、`ARCHITECTURE.md:15`、`README.md:20`、`SECURITY.md:30`、`INSTALL.md:5`、`CONTRIBUTING.md:12`、`CONTRIBUTING.md:26`、`docs/01-architecture/architecture.md:4`、`docs/04-internal/ENGINEER-MANIFESTO.md:55` · **陈旧真副本**：`Cargo.toml:75`「**18-crate** 单一工作区」 | 🟠 中：9 处一致但**无守卫**；1 处已陈旧 | **部分已修**（CI 侧从契约读）· **未修**：`Cargo.toml:75` 描述字段 ⇒ 建议生成或标历史口径 |
| **crate 体量预算** | **29 crate / 528,144 行** | **真源**：`scripts/crate-budget-baseline.json`（29 项，`loc` 合计 **528,144**、`files` 合计 1038）· **真副本（陈旧）**：`docs/status/anti-drift-audit-2026-10-28.md:123`「**29 个 crate · 合计 528,097 行**」 | 🟠 中：基线已 **+47 行**（commit `caf499b7`「crate 体量基线 +47 行」），文档仍写旧值 ⇒ 差 47 行 | **未修** ⇒ 建议：文档数字由 `scripts/check_crate_budget.py` 读基线后生成 |
| **孤儿 API 棘轮** | 142 条 `orphan` | **真源**：`scripts/unwired-baseline.txt`（`^orphan` **142** 行）· **真副本（现值一致）**：`docs/status/anti-drift-audit-2026-10-28.md:61`「孤儿 API（符号级）142 不变」 | 🟢 低：现值一致，但属手抄 | **未修**（无守卫）⇒ 建议由 `scripts/audit_unwired.py` 输出后生成 |
| **死代码豁免** | 44 处 | **真源**：`scripts/dead-code-baseline.txt`（44 条 + 头部自述「合计: 44 处 · 指纹: 640b9e0ef93dc869」）· **无第二处手写副本**（全仓 grep 未命中别的 44） | 🟢 低：单点 | **无需改**（基线自带指纹，漂移可检） |
| **文档分类棘轮** | 491 份待分类 | **真源**：`scripts/doc-class-baseline.txt`（491 行 + 头部「已登记（待分类）：491 份」）· **真副本（现值一致）**：`docs/status/anti-drift-audit-2026-10-28.md:110`、`docs/status/spec-batch-status.md:118` | 🟢 低：现值一致 | **未修**（无守卫）⇒ 建议 `check_doc_class.py --report` 生成 |
| **CI workflow 数** | **42** | **真源** = `.github/workflows/` 目录本身 · **真副本（现值一致）**：`docs/status/harness-convergence-decision-2026-10-30.md:31,40`、`docs/architecture/contract.toml:209` · **陈旧真副本**：`docs/status/anti-drift-audit-2026-10-28.md:21`「CI 工作流 **41** 个」、`docs/status/anti-drift-audit-2026-10-28.md:184`「41 CI」、`.github/PULL_REQUEST_TEMPLATE.md:142`「跟现有 **18 workflow** yml + 18 LOCKED crate 0 冲突」 | 🔴 高：PR 模板写 18（实际 42）⇒ 审阅者按 18 判断覆盖面 | **未修** ⇒ 建议：计数由生成器写进模板 |
| **守卫脚本数** | **未验证**（口径未定义） | `docs/status/anti-drift-audit-2026-10-28.md:22`「守卫脚本 **38** 个」、`:184`「38 脚本」· 实测：`scripts/*.py` = **35**、`scripts/check_*.py` = **18** | ⚪ 未验证：原文未定义「守卫脚本」口径，无法判定 38 是否为错 | **未验证** ⇒ 建议改用有定义的量（契约 `[ci_gates]` = 33 条，或 `scripts/*.py` = 35） |
| **门禁登记表** | **33 条 gate / 31 条 required** | **真源**：`docs/architecture/contract.toml:225-529`（`[[ci_gates.gate]]` × **33**）· **生成副本**：`.github/PULL_REQUEST_TEMPLATE.md:28-30` · **机读副本**：`scripts/gate-manifest.json` · **守卫**：`scripts/gen_gate_manifest.py:183-245`（`--check`，实测 exit 0） | 🟢 低：已机器化 | **已修**（真源 = 契约 `[ci_gates]`） |
| **分支保护 required 集合** | **5 门禁**（`cargo fmt --check` / `cargo deny check (...)` / `8 硬墙守门 (LOCKED ...)` / `Secret scan (...)` / `Guard 3.3 ...`） | 手写副本 `docs/development/community-ops.md:77` · **互相矛盾的三处**：`.github/workflows/docs-truth.yml:11-14` 自称「本 workflow 是 **required check**」（不在 5 之内）、`docs/04-internal/03-audits/absorption-ledger.md:85`「**kani** 已设分支保护 required check」、`docs/04-internal/00-governance/ops-network-playbook.md:148`「分支保护要求名 `\"kani\"` 而现行检查叫聚合器名」 | 🔴 高：required 集合在 **4 处**手写且互相矛盾；且**与契约 `[ci_gates]` 的 31 条 required 是两套东西**（后者是本地门禁，非 GitHub required check） | **未修** ⇒ 建议：远端真源在 GitHub 设置（**本地不可读**）；仓内一律改为指针 + 生成，并在契约增 `[required_checks]` |
| **hard-walls 守门条数** | job 名写「8 硬墙守门」，**实际 6 条**（守门 1–6） | `.github/workflows/rust.yml:95`（job 名）vs `rust.yml:118,178,221,241,274,293`（守门 1..6）· 一致的副本 `docs/02-guides/verification-architecture.md:54`「守门 1–6」· 其他副本 `Cargo.toml:127` `hard_walls = "8 (...)"`、`.github/workflows/companion-desktop-ci.yml:133`（同名「8 硬墙守门」但只有 3 条守门步） | 🟠 中：job 名 ≠ 实际条数 ⇒ required check 名与内容脱节 | **未修** ⇒ 建议 job 名改用实际条数或从真源取 |
| **Kani 矩阵件数** | **10 件**（matrix 实列 10 项） | **真源**：`.github/workflows/kani.yml:108-121`（`harness:` 列表，自 `:101` 起，实测 **10** 项）· **陈旧真副本**：`docs/02-guides/verification-architecture.md:56`「**22 harness 矩阵**」、`.github/workflows/kani.yml:102`（注释「**11 件**本机实证可证全集」，另见 `:25`、`:152`、`:159`、`:165`）、`.github/workflows/kani.yml:169` echo「**10 件** harness 全部证完」、`.github/workflows/kani.yml:23`「10 件全绿才放行」 | 🟠 中：**同一文件内 11 与 10 自相矛盾**；文档写 22 | **未修** ⇒ 建议：注释与 echo 数字由 matrix 长度生成 |
| **Rust 工具链版本** | `rust-toolchain.toml` channel = **1.97.1**；MSRV **1.97** | **真源**：`rust-toolchain.toml:9`、`Cargo.toml:66`（注释「与 rust-toolchain.toml 对齐 (1.97.1)」）· **陈旧真副本**：`rust-toolchain.toml:5`「CI 仍用 dtolnay/rust-toolchain@stable (per `.github/workflows/*.yml` **33 处**)」⇒ 实测 **40 处**（`@stable` **35** + `@nightly` **5**）· **另一处冲突值**：`frontend/companion-desktop/src-tauri/Cargo.toml:9` `rust-version = "1.77.2"`（未继承 workspace） | 🟠 中：① 注释数字陈旧；② CI **40 处全部用 `@stable`/`@nightly`** ⇒ 钉住的 1.97.1 **在 CI 从不生效**（`rust-toolchain.toml:5-6` 自陈此事）；③ 子工程 MSRV 与 workspace 不一致 | **未修** ⇒ 建议：① 注释数字改为「40 处」并加机器校验；② CI 改钉版本或改文档；③ 子工程改 `rust-version.workspace = true` |
| **Rust edition** | 2021 | **真源**：`Cargo.toml:65` · **全仓未发现第二处手抄**（`grep -rn "edition" Cargo.toml` 仅此一行） | 🟢 低：无副本 | **无需改** |
| **workspace 版本** | **2.0.0-rc.4** | **真源**：`Cargo.toml:64` + `docs/architecture/contract.toml:34` · **真副本（已生成化）**：`.github/PULL_REQUEST_TEMPLATE.md:22` · **陈旧真副本**：`Cargo.toml:75`（描述字段写 v1 历史口径「13 键」「18-crate」）、`.github/PULL_REQUEST_TEMPLATE.md:35`「见 `docs/versioning/`」（**该目录不存在**，实测 `ls docs/versioning` 失败） | 🟠 低–中：模板指向不存在的路径 | **部分已修**（数字已生成化）· **未修**：模板 L35 的死路径 ⇒ 建议改为指向 `docs/04-internal/00-governance/release-checklist.md` |
| **R11 baseline 3 值** | 0.8682 / 0.8532 / 0.9063 | **真源（代码常量）**：`legacy/donor/apeireth-asi/tests/integration_r_measure.rs`（编译期 const，**已归档**）· **手写副本 ×3 处**：`.github/PULL_REQUEST_TEMPLATE.md:10`、`.github/PULL_REQUEST_TEMPLATE.md:71`、`Cargo.toml:127`（A1）、`.github/workflows/rust.yml:221`、`.github/workflows/rust.yml:233` | 🟠 中：同一组数字手写 3 处，且**公开复现入口已归档**（`ROADMAP.md:68` 已自记「R11 baseline 3 值无公开复现入口」） | **未修** ⇒ 建议：抽成单一机器可读文件（契约字段或 `scripts/r11-baseline.json`），CI/模板/元数据都读它 |
| **状态声明条数** | **29** | **真源**：`docs/status/claims.yaml`（29 条 `- id:`）· **真副本（现值一致）**：`docs/status/workline-registry.md`（工作线表行「状态声明（29 条能力口径）」）、`docs/architecture/contract.toml:234` note「claims 29 条」 | 🟢 低：现值一致 | **无需改** |
| **1.0 release 12 项 checklist** | 12 项（9 P0 + 3 P1） | **真源**：`scripts/release-1.0-checklist.sh:2-3`（头注「12 项」+「9 P0 + 3 P1」）与 `check 1`..`check 12` · **真副本（逐条手抄）**：`.github/PULL_REQUEST_TEMPLATE.md:116-131` | 🟠 中：脚本改一项即模板陈旧 | **未修** ⇒ 建议：模板该段由脚本生成（`gen_gate_manifest.py` 同款模式） |

---

## 二、workflow 状态语义

> **列 2**：能否被 path 过滤跳过（`paths:` / `paths-ignore:`）。
> **列 3**：跳过时仓内是否产出**机器可读结论**（`skipped` 结论 / 汇总产物 / 状态文件 / `needs` 聚合）。
> **列 4**：产物名是否**绑 commit SHA**。
> ⚠️ **通用事实**：GitHub 会自动给未触发的 workflow run 一个 `skipped` conclusion，但**仓内没有任何脚本/产物消费它** —— 全仓 `grep -rn "conclusion" .github/` **仅 1 处命中**（`.github/workflows/cosign.yml:228`）。

| workflow | 有 paths 过滤? | 跳过时是否产出结论 | 产物是否绑 SHA | 备注 |
|---|---|---|---|---|
| `arch-guard.yml` | ✅ 有（`:7` PR；`:15` push **无**） | ❌ 无 | ❌（`:74` `arch-guard-log`） | `:25` `continue-on-error: true` 灰度 report-only 至 2026-11-03 |
| `bench-report.yml` | ❌ 无 | ❌ 无 | ❌（`:108` `bench-output`） | schedule + dispatch |
| `cargo-audit.yml` | ❌ 无 | ❌ 无 | ❌（`:210` `cargo-audit-report`） | 无 paths ⇒ 不会被路径跳过 |
| `cargo-deny.yml` | ❌ 无 | ❌ 无 | 无产物 | job 名 `cargo deny check (...)` = 分支保护 required 之一 |
| `claims-verify.yml` | ❌ 无 | ❌ 无 | 无产物 | `check_claims_verify.py --wired-gate` |
| `companion-desktop-ci.yml` | ✅ 有（`:10`,`:15`） | 🟡 **workflow 内**聚合（`:240-260` `ci-gate`，`needs: [tauri-shell, svelte-ui, hard-walls]`，`if: always()`，逐个判 `!= success` ⇒ 红） | 无产物 | **跨 workflow 无关**；`:245` 判 `!= success` ⇒ **skipped ≠ pass**（严格） |
| `cosign.yml` | ❌ 无 | ⚠️ **例外：`:428` / `:597` 把 `needs.sign.result == 'skipped'` 与 `'success'` **等同**（`||`）** | ❌（`:391` `cosign-signatures-${{ env.APEIRETH_VERSION }}`） | **仓内唯一「SKIPPED 当 PASS」的写法**；`:56-58` 有唯一一处 `workflow_run` 跨 workflow 触发（`:228` 要求 `conclusion == 'success'`） |
| `coverage.yml` | ❌ 无 | ❌ 无 | 无产物 | 无 paths |
| `dead-code-ratchet.yml` | ✅ 有（`:23`,`:30`） | ❌ 无 | 无产物 | 死代码棘轮 |
| `dependabot-upgrade.yml` | ❌ 无 | ❌ 无 | 无产物 | `:81-84` major 版自动跳过 auto-merge（**明写 notice**，非静默） |
| `doc-caliber.yml` | ✅ 有（`:14`,`:19`） | ❌ 无 | 无产物 | `:48-49` 用 `$GITHUB_SHA` 做 `--changed` 入参（**非产物绑定**） |
| `doc-paths.yml` | ✅ 有（`:17`,`:22`） | ❌ 无 | 无产物 | — |
| `docs-governance.yml` | ✅ 有（`:21`,`:28`） | ❌ 无 | 无产物 | — |
| `docs-truth.yml` | ❌ **刻意无** | ❌ 无 | 无产物 | `:9-18` 明写「本 workflow 是 required check … 带 paths filter 时 PR 会永远停在 Waiting for status」⇒ **本仓已认知该坑** |
| `fuzz.yml` | ✅ 有（`:24`，**仅 PR 侧**） | ❌ 无 | ❌（`:77`,`:114` 按 matrix.target） | schedule 不受 paths 影响 |
| `golden-baseline.yml` | ✅ 有（`:17`,`:25`） | ❌ 无 | 无产物 | — |
| `guard-v3.3.yml` | ❌ 无 | ❌ 无 | ❌（`:86` `guard-v3-3-evidence`） | job 名 = 分支保护 required 之一 |
| `image-family.yml` | ✅ 有（`:7`,`:14`） | ❌ 无 | ❌（`:43` `image-family-gate-log`） | — |
| `kani.yml` | ❌ 无 | ✅ **仓内唯一**：「未触碰已验证面」由 `:54,85,89` 输出 `verified=true/false` → `:153-172` `kani-required-gate`（`needs: [changes, verify]`，`if: always()`）产出**路径豁免裁决** | 无产物 | 聚合**仅限 workflow 内**；`:164-172` 判 `RESULT != success` ⇒ **skipped ≠ pass**（严格） |
| `m2b-xv-isolation.yml` | ❌ 无 | ❌ 无 | 无产物 | push[main] + dispatch |
| `m2c-xv-shell-validation.yml` | ❌ 无 | ❌ 无 | 无产物 | 同上 |
| `m3a-canonical-fetch.yml` | ❌ 无 | ❌ 无 | 无产物 | 同上 |
| `miri.yml` | ❌ 无 | ❌ 无 | 无产物 | `community-ops.md:79` 记为红检查 |
| `mod-targets.yml` | ✅ 有（`:24`,`:29`） | ❌ 无 | 无产物 | — |
| `nightly-full.yml` | ❌ 无 | ❌ 无 | 🟡 `:69` `gate-full-log-${{ github.run_id }}`（绑 **run_id**，非 SHA） | `:34` `nightly full gate (gate_full.ps1)`；**唯一 full 层执行器载体** |
| `o6-anchor.yml` | ❌ 无 | ❌ 无 | 无产物 | ⚠️ `:99-168` 按「**13 键**」判（实际 14 键，见 §一） |
| `pii-leak-detection.yml` | ❌ 无 | ❌ 无 | 无产物 | schedule + push + dispatch |
| `protocol-e2e.yml` | ❌ 无 | ❌ 无 | 无产物 | — |
| `publish-release.yml` | ❌ 无 | ❌ 无 | ❌（`:87` `desktop-nsis-windows-x64`） | tag 触发；`:94` `needs: build-windows-installer` |
| `quality-metrics.yml` | ✅ 有（`:20`,`:25`） | ❌ 无 | 无产物 | — |
| `release-1.0.0.yml` | ❌ 无 | 🟡 **workflow 内**聚合（`:389-418` `release-gate`，`needs: [build-packages, security, perf, release-checklist]`，`if: always()`，逐个判 `!= success`） | ❌（`:196`,`:335`,`:381`） | ⚠️ `:368` `release-1.0-checklist.sh --skip 4,5,6 **\|\| true**` ⇒ 该步骤**永不红**（skipped 段被 `\|\| true` 吞掉） |
| `release-prep.yml` | ❌ 无 | ❌ 无 | 无产物 | `:28` `release-prep dry-run (8 硬墙 + PII + 12 项)` |
| `release.yml` | ❌ 无 | 🟡 **workflow 内**聚合（`:417-472` `release-gate`，6 个 `needs`，`if: always()`，逐个判 `!= success`） | ❌（8 处，按 target/版本命名） | tag `v1.0.0*` 触发 |
| `rust-lint.yml` | ❌ 无 | ❌ 无 | 无产物 | `:22` clippy 3 档 · `:59` `cargo fmt --check (单独)` |
| `rust.yml` | ❌ 无 | 🟡 **workflow 内**（`:94-312` `hard-walls` 6 条守门；`:315-335` `secret-scan`） | ❌（`:88` `nextest-junit-${{ matrix.os }}.xml` —— **同名不绑 SHA**） | `:118-135` 从契约读受锁清单；job 名「8 硬墙守门」但实为 6 条 |
| `rustdoc-test.yml` | ❌ 无 | ❌ 无 | ✅ `:55` `doc-test-${{ github.sha }}-${{ matrix.os }}` | **仅 2 个产物绑 SHA 之一** |
| `rustdoc.yml` | ❌ 无 | ❌ 无 | ✅ `:49` `doc-${{ github.sha }}` | 同上 |
| `rustfmt.yml` | ❌ 无 | ❌ 无 | 无产物 | job 名 = 分支保护 required 之一 |
| `self-status-vocab.yml` | ✅ 有（`:19`,`:24`） | ❌ 无 | 无产物 | `:42` `continue-on-error: true`（report-only） |
| `stale-artifacts.yml` | ✅ 有（`:22`,`:26`） | ❌ 无 | 无产物 | `:39` `continue-on-error: true` |
| `unwired-ratchet.yml` | ✅ 有（`:24`,`:32`） | ❌ 无 | 无产物 | — |
| `wiring-ledger.yml` | ✅ 有（`:12`,`:18`） | ❌ 无 | ❌（`:61` `wiring-ledger-log`） | `:51` `continue-on-error: true` 灰度 |

**统计（实测）**：42 个 workflow = **15 个有 paths 过滤**（可被路径跳过）＋ **27 个无**；15 个**全部**「跳过时无机器可读结论」；
产物绑 SHA 的只有 **2 个**（`rustdoc.yml:49`、`rustdoc-test.yml:55`）；`upload-artifact` 共 15 个 workflow 在用，**13 个产物名不含 SHA**。

---

## 三、是否存在聚合裁决

### 3.1 结论

**GitHub Actions 层没有聚合裁决层。** 42 个 workflow 各自独立，**零**「把这些 workflow 的结论汇成一个裁决」的载体。

⚠️ **但仓内已有一个**本地聚合入口（并发施工中，**未接线**）**：`scripts/harness.py` 提供 `merge-decision` 子命令，
其状态语义**明写「SKIPPED 不等于 PASS」**（`scripts/harness.py:45-49`：「`required = true` 的门禁若被 `SKIPPED` ⇒ 裁决 `BLOCKED`」），
另有绑 SHA 的证据包 `attest`（`scripts/harness.py:68` → `artifacts/harness/<commit_sha>/`）。
⇒ ⇒ **它解决的是「本地跑过了」≠「合并没问题」，尚未解决「远端 42 个 workflow 的结论怎么汇」** ——
实测 `grep -rn "harness" .github/workflows/` **零命中**（命中的全是 `kani.yml` 里无关的 harness 矩阵字样）
⇒ **未接进任何 workflow**，远端仍无聚合裁决。

### 3.2 grep 证据

| 检索 | 结果 |
|---|---|
| `grep -rni "merge-gate\|merge_gate\|required_checks\|aggregate\|rollup\|verdict" .github/ scripts/` | **仅 1 条命中**，且与聚合无关：`.github/workflows/rust.yml:241`「守门 4: 14 键 守门 (verdict cache)」（是「verdict cache」这个业务名词，不是裁决聚合） |
| `grep -rn "conclusion" .github/` | **仅 1 处**：`.github/workflows/cosign.yml:228` `(github.event_name == 'workflow_run' && github.event.workflow_run.conclusion == 'success') \|\|` —— **唯一**跨 workflow 读取他 workflow 结论的地方；它只是**触发** `cosign.yml` 自己，**不产出**统一裁决 |
| `grep -rn "workflow_run" .github/` | **仅 1 处**：`.github/workflows/cosign.yml:56-58`（`workflows: ["1.0.0 release pipeline"]`，`types: [completed]`）⇒ 全仓**唯一**的跨 workflow 依赖，且是**单向触发**，不是汇总 |
| `grep -rn "^    needs:\|needs: \[" .github/workflows/` | 命中的 `needs:` **全部是 workflow 内 job 依赖**：`companion-desktop-ci.yml:243`、`cosign.yml:422`、`cosign.yml:623`、`kani.yml:95`、`kani.yml:155`、`publish-release.yml:94`、`release-1.0.0.yml:216,252,308,350,392`、`release.yml:327,420` ⇒ **没有任何 workflow 用 `needs:` 去依赖另一个 workflow** |
| `grep -rn "GITHUB_STEP_SUMMARY" .github/workflows/` | **0 命中** ⇒ 无「统一裁决汇总页」 |
| `grep -rn "status file\|status.json\|verdict.json" .github/` | **0 命中** ⇒ CI 层无绑 SHA 的裁决状态文件（`scripts/harness.py` 的 `merge-decision` / `attest` 是**本地**入口，未接 CI） |
| `grep -rn "harness" .github/workflows/` | **0 命中**（仅 `kani.yml` 里无关的 harness 矩阵字样）⇒ 本地聚合器 `scripts/harness.py` **未接进任何 workflow** |

### 3.3 仓内**确实存在**的三个聚合门（全部是 workflow **内**聚合）

| 聚合门 | 位置 | 聚合范围 | 是否区分 skipped / pass |
|---|---|---|---|
| `kani-required-gate` | `.github/workflows/kani.yml:153-172` | `changes` + `verify`（10 件矩阵） | ✅ **区分**：`:164` 判 `VERIFIED != true` ⇒ 绿；`:169-172` 判 `RESULT != success` ⇒ **红** ⇒ skipped **不**等于 pass |
| `companion-desktop CI gate (3/3)` | `.github/workflows/companion-desktop-ci.yml:240-260` | 3 个 job | ✅ **区分**：`:248-258` 逐个判 `!= success` |
| `1.0 release gate (5/5)` / `6-package release gate (6/6)` | `.github/workflows/release-1.0.0.yml:389-418` / `.github/workflows/release.yml:417-472` | 4~6 个 job | ✅ **区分**：逐个判 `!= success` |

### 3.4 唯一的「SKIPPED 当 PASS」写法

`.github/workflows/cosign.yml:428` 与 `.github/workflows/cosign.yml:597`：

```
(github.event_name == 'workflow_dispatch' && (needs.sign.result == 'success' || needs.sign.result == 'skipped'))
```

⇒ **`skipped` 被当作 `success`**。这是全仓**唯一**一处显式的「SKIPPED == PASS」语义（其余聚合门都是严格 `!= success` 即红）。

### 3.5 分支保护 required 集合：本地**不可读**

GitHub 的 branch protection required checks 是**远端设置**，仓内没有配置文件。仓内能找到的只有**手写描述**：

* `docs/development/community-ops.md:77` —— 必需检查 **5 门禁**：`cargo fmt --check` / `cargo deny check (...)` / `8 硬墙守门 (LOCKED ...)` / `Secret scan (...)` / `Guard 3.3 ...`（选取口径：「每个 PR 必跑（无 paths 过滤）且当前稳定绿」）。
* `docs/development/community-ops.md:79` —— 诚实记录：main 尚有一批红检查（nextest / miri / kani / doc-test 矩阵等），故现阶段只强制这 5 个。
* `docs/04-internal/03-audits/absorption-ledger.md:85` —— 「✅ **已设**分支保护 required check（`kani` …）」。
* `docs/04-internal/00-governance/ops-network-playbook.md:148` —— 「分支保护要求名 `\"kani\"` 而现行检查叫聚合器名 ⇒ 合并被卡」。

⇒ **远端实际配置 = 未验证**（本地无 `.github/` 配置、无 API 凭证可用）；上面 4 处**互相矛盾**，本身即漂移点。

---

## 四、结论

### 4.1 真实的漂移风险（**已修**的也要标出来，因为「已修」靠的是机器守卫，不是靠人记得同步）

| # | 漂移风险 | 证据（file:line） | 状态 |
|---|---|---|---|
| **R1** | **PR 模板按 6 锚写，实际 9 锚** ⇒ 人按模板勾选会漏 3 条锚 | 副本 `.github/PULL_REQUEST_TEMPLATE.md:46,88,92,111,140,169`「6 哲学 anchor」；真源 `Cargo.toml:136`（9 项）、`AGENTS.md:77` | 🔴 **未修** |
| **R2** | **CI 守门 `o6-anchor.yml` 按 13 键判，代码是 14 键** | `.github/workflows/o6-anchor.yml:99,100,106,109,142,168` vs `crates/foundation/core/src/philosophy.rs:107`（`[PhilosophyKey; 14]`）、`:162`（编译期断言 14）、`AGENTS.md:77`、`ROADMAP.md:68` | 🔴 **未修** |
| **R3** | **PR 模板仍要求「不改 LOCKED 24 crate」，而 24 crate 名单 R148 已形式撤销** | `.github/PULL_REQUEST_TEMPLATE.md:109`；撤销事实 `Cargo.toml:127`（「B1 24 LOCKED 已形式撤销 R148」）、现行受锁源 3 个 `docs/architecture/contract.toml:37-45` | 🔴 **未修** |
| **R4** | **PR 模板「跟现有 18 workflow」= 实际 42** | `.github/PULL_REQUEST_TEMPLATE.md:142` vs `ls .github/workflows/*.yml \| wc -l` = 42 | 🔴 **未修** |
| **R5** | **分支保护 required 集合 4 处手写且互相矛盾** | `docs/development/community-ops.md:77`（5 门禁，无 kani、无 docs-truth）vs `.github/workflows/docs-truth.yml:11-14`（自称 required check）vs `docs/04-internal/03-audits/absorption-ledger.md:85`（kani 已设）vs `docs/04-internal/00-governance/ops-network-playbook.md:148`（required 名 = `"kani"`，现行检查叫聚合器名 ⇒ 合并被卡） | 🔴 **未修**（远端真值本身**未验证**） |
| **R6** | **crate 体量文档数字已漂 47 行** | 副本 `docs/status/anti-drift-audit-2026-10-28.md:123`「528,097 行」vs 真源 `scripts/crate-budget-baseline.json`（合计 **528,144**） | 🟠 **未修** |
| **R7** | **workflow 计数在两份 status 文档里已漂** | `docs/status/anti-drift-audit-2026-10-28.md:21`（41）与 `:184`（41 CI）vs 实际 42；正确值见 `docs/status/harness-convergence-decision-2026-10-30.md:31` | 🟠 **未修** |
| **R8** | **Kani 矩阵件数三处不一致（22 / 11 / 10）** | `docs/02-guides/verification-architecture.md:56`（22）· `.github/workflows/kani.yml:25,102,152,159,165` 注释（11）· `.github/workflows/kani.yml:108-121` 实列（10）· `.github/workflows/kani.yml:169` echo（10） | 🟠 **未修** |
| **R9** | **CI 从不读钉住的工具链版本，且注释里的处数已陈旧** | `rust-toolchain.toml:5`「33 处」→ 实测 **40 处**（`@stable` 35 + `@nightly` 5）；`rust-toolchain.toml:9` 钉 `1.97.1` 但 40 处全用 `@stable`/`@nightly` | 🟠 **未修** |
| **R10** | **子工程 MSRV 与 workspace 不一致** | `frontend/companion-desktop/src-tauri/Cargo.toml:9` `rust-version = "1.77.2"` vs `Cargo.toml:66` `rust-version = "1.97"` | 🟠 **未修** |
| **R11** | **15 个可被路径跳过的 workflow 跳过时不产出任何机器可读结论** ⇒ 在 **CI 层**「SKIPPED 当 PASS」事实上成立 | 15 个含 `paths:` 的 workflow 逐个无产物/无结论（§二）；全仓 `grep -rn "GITHUB_STEP_SUMMARY" .github/workflows/` = 0 命中；`grep -rn "conclusion" .github/` 仅 `cosign.yml:228` 一处 | 🔴 **未修**（`docs/status/harness-convergence-decision-2026-10-30.md:76` 已登记为 P3「⏳ 待做」）。⚠️ **本地侧已部分覆盖**：`scripts/harness.py:45-49` 明写「`SKIPPED` ≠ `PASS`，required 门禁被 `SKIPPED` ⇒ 裁决 `BLOCKED`」，但**未接 CI**（§3.1） |
| **R12** | **`cosign.yml` 唯一一处把 `skipped` 当 `success`** | `.github/workflows/cosign.yml:428`、`.github/workflows/cosign.yml:597` | 🟠 **未修**（**有意为之**，但仓内无标注说明） |
| **R13** | **`release-1.0.0.yml` 的 checklist 步骤 `\|\| true` ⇒ 该段永不红** | `.github/workflows/release-1.0.0.yml:368` | 🟠 **未修** |
| **R14** | **`Cargo.toml` 元数据段整段是 v1.0 时代手写副本（13 键 / 18-crate / 9 重守门 / 24 LOCKED / 1.2.0）** | `Cargo.toml:74`、`:75`、`:127`、`:130`、`:132`、`:145`；现值真源 `docs/architecture/contract.toml:30-45` | 🟠 **未修**（**已修**的部分：PR 模板侧已生成化，见 §四 4.2） |
| **R15** | **PR 模板指向不存在的路径** | `.github/PULL_REQUEST_TEMPLATE.md:35`「见 `docs/versioning/`」（实测不存在）、`:103`「per `docs/conventions/10-locked.md`」（已归档为 `docs/archive/conventions/10-locked.md`）、`:88`「同步 `docs/conventions/` + `docs/glossary/`」（两目录均不存在，实为 `docs/03-reference/glossary.md`） | 🟡 **未修** |
| **R16** | **job 名与实际守门条数不符** | `.github/workflows/rust.yml:95`「8 硬墙守门」vs 实为 6 条（`:118,178,221,241,274,293`）；`.github/workflows/companion-desktop-ci.yml:133` 同名但实为 3 条 | 🟡 **未修** |
| **R17** | **R11 baseline 3 值手写 3 处，公开复现入口已归档** | `.github/PULL_REQUEST_TEMPLATE.md:10,71`、`Cargo.toml:127`、`.github/workflows/rust.yml:221,233`；真源已归档 `legacy/donor/apeireth-asi/tests/integration_r_measure.rs`；`ROADMAP.md:68` 自记「无公开复现入口」 | 🟡 **未修** |
| **R18** | **1.0 release 12 项 checklist 在 PR 模板里逐条手抄** | `.github/PULL_REQUEST_TEMPLATE.md:116-131` vs `scripts/release-1.0-checklist.sh:2-3` 及 `check 1..12` | 🟡 **未修** |

### 4.2 **已修**（机器守卫，不是靠人记得同步）

| # | 事实 | 证据 | 状态 |
|---|---|---|---|
| **F1** | **CI 侧受锁清单零手抄** | `.github/workflows/rust.yml:118-135`（tomllib 读 `docs/architecture/contract.toml` `[locked]`，注释明写「禁止在本 workflow 手抄路径清单（D5：禁硬编码）」） | ✅ **已修** |
| **F2** | **PR 模板的版本 / 成员数 / 受锁清单 / 门禁条数已由真源生成并机器校验** | `docs/architecture/contract.toml:225-529`（`[ci_gates]` 33 条）· `scripts/gen_gate_manifest.py:74-114`（生成）· `:183-245`（`--check` 校验：副本一致 + 每条门禁引用的脚本存在 + 受锁源存在）· 实测 `python3 scripts/gen_gate_manifest.py --check` → `OK: 门禁清单一致（33 条门禁 · PR 模板自动事实块与契约一致 · 受锁源码 3 个存在）` | ✅ **已修** |
| **F3** | **PR 模板原先的 `1.2.0` / 24-crate 名单已被生成块替换** | `.github/PULL_REQUEST_TEMPLATE.md:22`（`2.0.0-rc.4`）、`:24-27`（3 个受锁源）、`:28-30`（33 条门禁） | ✅ **已修**（但同文件 **R1/R3/R4/R15** 仍在） |
| **F4** | **`docs-truth.yml` 刻意不加 paths filter**（避免 required check 永不结论） | `.github/workflows/docs-truth.yml:9-18`（明写「required check 必须永远有结论」） | ✅ **已修**（该坑已认知） |
| **F5** | **3 个聚合门严格判 `!= success` ⇒ skipped ≠ pass** | `.github/workflows/kani.yml:164-172`、`.github/workflows/companion-desktop-ci.yml:248-258`、`.github/workflows/release-1.0.0.yml:397-412`、`.github/workflows/release.yml:425-461` | ✅ **已修**（但仅限 workflow 内；见 R11/R12） |

### 4.3 **不是问题**（避免编造问题）

* **`docs/03-reference/test-baseline.md` 作为测试数字单一真源** —— `AGENTS.md:52` 明写「其余文档只引用不抄写」，本次未发现活跃文档抄写测试通过数 ⇒ **不列**为漂移点。
* **crate 成员数 9 处手抄** —— `AGENTS.md:53`、`ARCHITECTURE.md:15`、`README.md:20`、`SECURITY.md:30`、`INSTALL.md:5`、`CONTRIBUTING.md:12,26`、`docs/01-architecture/architecture.md:4`、`docs/04-internal/ENGINEER-MANIFESTO.md:55` **现值全部一致（30 = 29 + 1）** ⇒ 是「副本」但**当前无漂移**，列为低风险而非风险（`Cargo.toml:75` 的 18-crate 才是真漂移，已计入 R14）。
* **孤儿 API 142 / 死代码 44 / 文档分类 491** —— 三处手抄值**与真源一致**（`docs/status/anti-drift-audit-2026-10-28.md:61,110`、`docs/status/spec-batch-status.md:118`）⇒ 无漂移，仅「缺守卫」。
* **`CONTRIBUTING.md`** —— 全文**无陈旧数字**：`CONTRIBUTING.md:3`「9 锚」、`:12`「30 members：29 crates + bench harness」、`:26`「当前 workspace 30 members」**均与真源一致**；其「合并门槛」清单（`:66`）虽未列全 31 条 `[ci_gates]`，但那是**概括**不是**数字副本** ⇒ **不列**为漂移点。
* **`edition = "2021"`** —— 全仓唯一一份（`Cargo.toml:65`）⇒ **无副本**，不是问题。
* **27 个无 paths 过滤的 workflow** —— 它们**不会被路径跳过** ⇒ 与「SKIPPED 语义」无关，**不列**为问题。
* **`docs/archive/`、`legacy/`、`reports/` 里的旧口径**（如 `reports/03-audit-and-verification.md` 的「13 键」「85 crates」）—— 按仓内纪律「历史不追溯改写」（`AGENTS.md:60`）⇒ **不列**为漂移点。
* **`guard-wiring` / `d4-ledger` / `verify-ledger` 等 gate 的 `note` 字段** —— 在契约里，**不是副本** ⇒ 不列。

---

## 五、复核命令

> 全部**只读**；未触发 cargo 编译 / 全量测试。

```sh
cd /home/bai/wt-apeireth-r3

# —— 一、手写副本 ——
grep -rn "13 键" .github/workflows/ Cargo.toml            # 键集真源 vs 副本
grep -rn "6 哲学" .github/PULL_REQUEST_TEMPLATE.md          # 锚数副本
grep -rn "24 LOCKED" .github/PULL_REQUEST_TEMPLATE.md Cargo.toml
grep -rn "18 workflow" .github/PULL_REQUEST_TEMPLATE.md
grep -rn "528,097\|528097" docs/                          # crate 体量旧值
grep -rn "41 个\|41 CI" docs/status/                       # workflow 数旧值
grep -rn "22 harness\|11 件" docs/ .github/workflows/kani.yml
grep -rn "dtolnay/rust-toolchain" .github/workflows/ | wc -l   # 期望 40

# 真源侧
python3 -c "import json;d=json.load(open('scripts/crate-budget-baseline.json'));print(len(d['crates']),sum(v['loc'] for v in d['crates'].values()))"   # 29 528144
grep -c "^orphan" scripts/unwired-baseline.txt            # 142
grep -vc "^#" scripts/dead-code-baseline.txt              # 45（含 1 空行）⇒ 44 条
grep -vc "^#" scripts/doc-class-baseline.txt              # 491
grep -c "^\[\[ci_gates.gate\]\]" docs/architecture/contract.toml   # 33
ls .github/workflows/*.yml | wc -l                        # 42
grep -rn "rust-version" frontend/companion-desktop/src-tauri/Cargo.toml   # 1.77.2

# —— 二、三、workflow 语义 ——
ls .github/workflows/ | wc -l
grep -rn "conclusion" .github/                            # 期望仅 cosign.yml:228
grep -rn "workflow_run" .github/                          # 期望仅 cosign.yml:56
grep -rn "GITHUB_STEP_SUMMARY" .github/workflows/         # 期望 0
grep -rn "needs\..*\.result" .github/workflows/           # 聚合门判据
grep -rn "upload-artifact" -A3 .github/workflows/ | grep "name:"   # 产物名（含 SHA 的只有 2 个）

# —— 守卫 ——
python3 scripts/gen_gate_manifest.py --check
python3 scripts/check_doc_class.py --check
python3 scripts/docs_links.py --check
```

---

## 六、**未验证**清单（如实登记，不猜）

| 待证事实 | 为什么未验证 |
|---|---|
| **GitHub 远端 branch protection 的实际 required checks** | 远端设置，仓内无配置文件；本地无 API 凭证 ⇒ `docs/development/community-ops.md:77` 等 4 处描述**互相矛盾**，**哪一份为真未验证** |
| **「守卫脚本 38 个」是否成立** | 原文未定义口径；实测 `scripts/*.py` = 35、`scripts/check_*.py` = 18，无 38 的对应口径 |
| **`docs/status/anti-drift-audit-2026-10-28.md:25`「280 个文件含确定性 / 逐字节断言」** | 未逐文件核实（本审计不查测试断言覆盖率） |
| **`.github/workflows/docs-governance.yml`、`arch-guard.yml` 等无 `name:` 的 job 的实际 check 名** | 这些 job 未设 `name:` ⇒ GitHub 显示名即 job id；未在远端跑过 ⇒ 未验证其是否被分支保护收录 |
| **`scripts/harness.py` / `scripts/check_gate_manifest.py` / `scripts/guard-inventory.json`** | `docs/status/harness-convergence-decision-2026-10-30.md:74,84-86` 声明「本批已做」；实测审计时点 **HEAD `4a42d9da` 三个文件均不在树内**（`scripts/gen_gate_manifest.py` 与 `docs/architecture/contract.toml` 的 `[ci_gates]` 确实存在）⇒ 该决策文档的「已落地」清单**与 HEAD 不符**；审计期间工作区有**并发未提交**的 `scripts/harness.py`（本地聚合器，SKIPPED≠PASS 语义见 §3.1）、`scripts/gen_guard_inventory.py`、`scripts/guard-inventory.json` 出现，`scripts/check_gate_manifest.py` / `scripts/check_guard_inventory.py` **仍不存在** ⇒ **交接时需重新核对**（本审计只记录事实，不追责） |
