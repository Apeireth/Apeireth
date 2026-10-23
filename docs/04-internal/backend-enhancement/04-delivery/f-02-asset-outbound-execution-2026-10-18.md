# F-02 大资产出库执行批交付报告（2026-10-18）

> `Status: ✅ #1 reports/ 批量日志 878 件已迁出 / ✅ #2 previews/ 34 PNG 走 git-lfs / ✅ #3 guard_ml 4 大 jsonl 已外置 / 🟡 #4 artifacts/ 仅登记（owner 拍板另批，D17 留账）`
> 执行依据：[audit-gap-caliber-hygiene-2026-10-16.md]../audit-gap-caliber-hygiene-2026-10-16.md) §4.3 F-02 方案表（**照方案执行，不重议**）。
> 实测口径：2026-10-18，工具链 rustc/cargo 1.97.1（MSVC）、git-lfs 3.7.1、bsdtar 3.8.9、pwsh 7；
> 体积 = `git ls-files` tracked 面 + `Get-Item` 字节数，sha256 = `Get-FileHash -Algorithm SHA256`。
> 迁移清单（逐件 916 行）：[f-02-migration-manifest-2026-10-18.tsv]../f-02-migration-manifest-2026-10-18.tsv)。

## 0. 方案表 → 执行对照（§4.3 表逐行）

| 方案表行 | 资产 | 方案建议 | 本批执行 | 结果 |
|---|---|---|---|---|
| 1 | `reports/` v1.0 时代批量日志 | 迁 release 资产（首选） | 打包 `release-assets/reports-archive-2026-07-08.tar` + `git rm`（工作树删除、git 历史不动） | ✅ 878 件 / 34,728,071 B 迁出 |
| 2 | `reports/` 近期证据/验收报告 | **留库** | 留库 195 件（2026-09~10 证据 + 基准/验收/评审件 + 7 个证据子目录） | ✅ 0 触碰 |
| 3 | `reports/` 非 md 资产 | 与 #1 同批同包 | 随日期规则归桶：带 2026-07/08 日期的 10 件（2 `.ps1` + 8 `.txt`）随 #1 同包迁出；其余非 md 资产落在「无日期/其他」桶 → 按任务口径（797+83）留库登记（§6.2） | ✅ 部分随包 / 🟡 残件登记 |
| 4 | `previews/` 34 PNG / 39MB | **git-lfs**（首选） | `git lfs track 'previews/*.png'` + `git add --renormalize previews/`（保路径零断链）；**git-lfs 3.7.1 已安装，降级面未触发** | ✅ 34 件转 LFS 指针 |
| 5 | `scripts/guard_ml/` 4 大 jsonl | **外置**（首选），`scenarios.jsonl`+`fixture_artifact.json` 等必留 | 4 件打包 `release-assets/guard-ml-exports-archive-2026-10-18.tar` + `git rm`；9 件留库（含 `include_str!` 硬依赖 2 件）；`scripts/guard_ml/README.md` 注再生入口；`guard-v3.3.yml` 校验步加时序说明注释 | ✅ 4 件外置，硬红线 0 触碰 |
| 6 | （旁注）`artifacts/` 361 件 157MB | 外置（超任务范围，仅登记） | **仅登记不处置**（§5），owner 拍板另批 | 🟡 留账 D17 |

## 1. 前后体积表（tracked 面口径，2026-10-18 实测）

| 资产组 | 前 | 后 | 差 | 手段 |
|---|---|---|---|---|
| `reports/` | 1073 件 / 39,938,586 B | 195 件 / 5,210,515 B | **−878 件 / −34,728,071 B** | 打包迁 release 资产 + `git rm` |
| `scripts/guard_ml/` | 13 件 / 22,087,951 B | 9 件 / 90,940 B | **−4 件 / −21,997,011 B** | 外置打包 + `git rm` |
| `previews/` | 34 件 / 40,937,344 B（普通 blob） | 34 件 / 40,937,344 B（工作树不变）；git 索引转 LFS 指针（每件 ~130 B） | 工作树 0；**git 对象跟踪面 −~40.9MB**（历史 blob 不减，方案表已注） | git-lfs（`.gitattributes` 新行） |
| `artifacts/` | tracked 131 件 / 20,172,935 B；盘上 395 件 / 172,061,213 B | 不变 | 0 | 仅登记 |
| **合计（工作树净减）** | | | **−56,725,082 B（54.1 MiB）** | 另 + git-lfs 面 −40.9MB |

> 口径注：`artifacts/` 盘上 395 件 / 172.1MB > 审计口径（2026-10-16，361 件 / 157.2MB），差额 = 其他施工线 2026-10-16~18 新增的未跟踪测试日志（`artifacts/*.log` 等，`.gitignore` 已排除面）。

## 2. 迁移清单 manifest

| 载体 | 内容 | 校验 |
|---|---|---|
| [f-02-migration-manifest-2026-10-18.tsv]../f-02-migration-manifest-2026-10-18.tsv) | 916 行逐件清单（reports 878 + guard_ml 4 + previews 34），列：group / disposition / bucket / repo_path / size_bytes / sha256 / destination | 与两包 manifest 逐行一致 |
| `release-assets/reports-archive-2026-07-08.tar` | 878 件 + 内置 manifest = 879 条目 / 35,641,344 B | sha256 `16c5af894711456246b4d889ddeb98c5ee273c81e413a933f775b70304f44f75` |
| `release-assets/reports-archive-2026-07-08.manifest.tsv` | 878 行（path / size / sha256） | 解包全量对账 **878/878 一致、0 缺失 0 失配** |
| `release-assets/guard-ml-exports-archive-2026-10-18.tar` | 4 件 + 内置 manifest = 5 条目 / 22,002,688 B | sha256 `601aac199cac0bb8bc70e07f2768c5152925c41718ef4e8552ac8b4fd460c73f`；解包对账 4/4 一致 |
| `release-assets/guard-ml-exports-archive-2026-10-18.manifest.tsv` | 4 行 | 同上 |

**迁移规则（可复核，与审计口径对账）**：reports/ tracked 面中，文件名含 `2026-07`/`2026-08` 的 v1.0 时代批量日志（795 件）+ 全长 UUID 命名工作日志（77 件）+ 短哈希前缀工作日志（6 件）= **878 件迁出**；
与审计口径 797 + 83 对账：797 = 795 tracked + 2 盘上未跟踪带日期件（`commit-msg-r20-changelog-readme-sync-2026-08-05.txt`、`cargo-audit-r23-final-2026-08-06.txt`，未跟踪不迁）；83 = 77 全长 UUID + 6 短哈希（`20131b80-*`/`2115d6c1-*`/`8b9d492b-*`/`a2557c25-*`/`a7c5b65b-*`/`d8437877-*`）。
`release-assets/` 已入 `.gitignore` 新行（打包件只随 Release 资产分发）；26 件含非 ASCII 文件名（`续/战役/严守/§/①` 等）解包核对完好。

## 3. 构建零伤证明（动文件前逐类验证 + 迁移后实测）

**前置引用审计（动文件前，逐类）**：

| 类 | 命令/口径 | 结论 |
|---|---|---|
| 编译期依赖 | `include_str!`/`include_bytes!` 全库 81 处 | 唯二数据面依赖 = `crates/engine/guard/tests/security_scenarios.rs:3-4` → `scripts/guard_ml/scenarios.jsonl` + `fixture_artifact.json`（**留库=硬红线，0 触碰**）；reports/、previews/ **0 处**编译期引用 |
| 代码运行时引用 | `git grep 'reports/\|previews/' -- '*.rs' '*.toml'` 68+7 处 | 全部为注释/引注（历史引用名），0 处文件读取；`Cargo.toml:162` metadata 引用为历史口径原值不动 |
| 脚本引用 | `scripts/*` 40 处 | 全部为历史引注或运行时**写入** reports/（目录留存不受影响）；`secret-scan.ps1`/`check_doc_caliber.py` 是目录排除口径（目录仍在） |
| CI 工作流 | `guard-v3.3.yml`（校验步在导出步**之后**，出库不缺件）；`release-1.0.0.yml:382` 上传 `reports/r20-v1.0.0-release-checklist-*.md`（`if-no-files-found: warn`）；`release.yml:410` 写入 `reports/tp20-s5-*.txt` | 0 处会失败；2 件 checklist 报告随批迁出后该历史工作流仅告警（§4 登记） |

**迁移后测试实测（`-j 2` 口径）**：

| 校验 | 命令 | 基线（动文件前） | 迁移后 |
|---|---|---|---|
| guard 全套（含 `include_str!` 硬依赖面） | `cargo test -p apeireth-guard --locked --all-targets -j 2` | ✅ exit 0（全 6 套件绿，`security_scenarios` 2 例过） | ✅ exit 0（**79 passed; 0 failed**，`security_scenarios` 2 例绿） |
| workspace 冒烟 | `cargo test --workspace --locked -j 2 --no-fail-fast` | —（数据面 0 引用已证无影响） | ✅ **exit 0（0 failed，全套件含 doc-tests 通过）** |
| 归档可回取性 | 解包 → 逐件 sha256 对账 manifest | — | ✅ reports 878/878、guard_ml 4/4，0 缺失 0 失配 |
| 中性表述扫描 | `pwsh -File scripts/check-neutral-terms.ps1` | — | ✅ `OK … added lines all neutral`（0 命中） |

> 报备：workspace 冒烟运行时工作树含**其他施工线在途改动**（`crates/adapters/*`、`crates/capabilities/tools/*`、`crates/engine/organ`、`frontend/companion-desktop/*`、`docs/02-guides/frontend-data-contract.md`、`docs/gateway-api-contract.md` 等，非本批战场），测试全绿含该在途面；本批 0 触碰他人在途文件。

## 4. 引用面处置（711 处 docs 引用不改）

- **711 处 docs 引用不改**：引用的是报告名 = 历史卷宗口径，`reports/README.md` 顶部已加迁出说明（「批量日志已迁 release 资产，历史引用名保留」+ 取回路径）；历史文档、代码注释、`Cargo.toml` metadata 原值全部保留。
- `previews/` 2 处活跃引用（`docs/design/frontend-handoff.md:178`、`docs/design/intro-animation.md:25`）**零断链**——git-lfs 保路径，实拍验收图仍在库可见。
- 登记不改：`release-1.0.0.yml:382` 通配引用的 `reports/r20-v1.0.0-release-checklist-2026-08-05.md` + `-2026-08-19.md` 随批迁出（历史 v1.0 发布工作流不再复跑，上传步 `if-no-files-found: warn` 只告警不失败）；`release.yml`/`release-1.0.sh` 等对 reports/ 的引注同为历史口径。
- 目录口径同步（方案最小动作②）：`docs/development/repository-layout.md:34-35` 两行注释同步（迁出 + LFS 口径）；`CONTRIBUTING.md` 检索 `reports/|previews/|guard_ml` **0 命中**，无需同步。
- `guard-v3.3.yml` 校验步加 2 行时序说明注释（方案最小动作④：断言对象 = 导出产物，与库内是否有件无关）；`scripts/guard_ml/README.md` 注「由 `export_guard_scenarios` 再生」。

## 5. `artifacts/` 登记（仅登记不处置，owner 拍板另批）

| 口径 | 件数 | 体积 |
|---|---|---|
| tracked（git 跟踪面） | 131 | 20,172,935 B |
| 盘上（含他线未跟踪日志） | 395 | 172,061,213 B |
| 审计口径（2026-10-16） | 361 | 157.2MB |

构成：审计转储（`audit-before/after/final` 文本）、`guard-*` 指标、batch 测试日志。方案表行 6 建议**外置**；体量超 F-02 三组之和，处置与逐件清单生成留 owner 拍板批（D17 继续留账）。

## 6. 边界与残件登记（登记不代拍板）

1. **口径差如实报**：审计口径（盘上含未跟踪）797 / 83；本批 tracked 实测迁出 878 = 795 + 83（对账见 §2）；`reports/` 盘上 8 件未跟踪工作件（`__tmp_report.py`、`_fmt_check*.py`、`commit-msg-*.txt` 等）非 repo 重量，**未动**。
2. **方案表行 3 残件**：非 md 资产中带日期的 10 件已随 #1 同包；其余非 md 工作件 **74 件**（21 `.ps1` + 13 `.log` + 13 `.png` + 9 `.html` + 9 `.txt` + 5 `.py` + 2 `.err` + 1 `.csv` + 1 `.yml`，落在「无日期/其他」桶）按任务口径 797+83 留库登记。若 owner 认可行 3「同批同包」更宽口径，可并入下一小批重打包。
3. **无日期桶中的批量日志形态残件**（195 留库件的构成，逐类实测）：`other md`（评审/设计/基准/证据）44、`rNN-*` 工作件 25、`agent/P/V` 验收件 23、`round*` 21、`achievement-*` 16、非 md 工作件 13、`dispatch-*` 4、`decision-*`（无日期）3、7 个证据子目录 46（`audit-2026-08-19` 11、`diagnostics-review` 11、`tp30` 10、`r18-snapshots` 7、`pet-real-smoke-2026-10-11` 3、`r20-stage-6-th` 3、`evidence-V10-qa2-A11-A20` 1）。其中 `round*`/`achievement-*`/`dispatch-*`/无日期 `decision-*` 等属批量日志形态但无日期命名，不在 797+83 口径内——**登记待拍板**（并入则同 tar 重打包，引用名同样保留）。
4. **previews 历史 blob**：LFS 只改现跟踪面，git 历史内 39MB 旧 blob 不减（方案表行 4 已注「历史 blob 不减」；如需历史瘦身属 `git lfs migrate` 重写历史批，破坏性、默认拒绝，须 owner 拍板）。

## 7. 纪律自检

- **0 装 PASS**：报告「已迁出」均有 manifest + 解包 sha256 对账证据；`artifacts/` 报「仅登记」不假称已瘦身；previews 报「LFS 已接」有 `git show :` 指针证据 + `git lfs ls-files` 34 件；
- **guard 依赖件留库 = 硬红线**：`scenarios.jsonl` + `fixture_artifact.json` 未动，迁移前后 `cargo test -p apeireth-guard` 双绿；
- **历史不追溯改写**：711 处 docs 引用、代码注释引注、`Cargo.toml` metadata、`reports/README.md` 内 R12 历史清单原值全部保留（仅顶部加迁出说明与「历史口径」标注）；
- **LOCKED 5 项 0 触碰**：哲学锚/13 键/脊柱/`workspace.version`/R11 baseline 均未动；`00-backlog.md` D17 仅追加状态注记；
- **不 commit / 不写 CHANGELOG**：全部改动留在工作树与索引（大批删除走 `git rm` 可追，复核用 `git diff`/`git diff --cached`）；
- **中性表述**：新增行 0 第三方产品名、0「移植/借鉴」措辞（`check-neutral-terms.ps1` 零命中）。

## 8. 后续（owner 项）

| # | 项 | 建议 |
|---|---|---|
| 1 | `release-assets/` 两包 + manifest 挂 Release 资产 | 挂 v1.0 归档 tag 的 Release（上传属发版权限，本批只备包）；包名 `reports-archive-2026-07-08.tar` / `guard-ml-exports-archive-2026-10-18.tar` |
| 2 | `artifacts/` ~172MB 处置 | 拍板后按方案表行 6「外置」另批执行（含逐件清单） |
| 3 | §6.2/§6.3 残件口径 | 拍板「并入重打包」或「留库」；并入则 26 件非 ASCII 文件名注意打包编码（本包已用 CP936 清单 + UTF-8 manifest 双备） |
| 4 | LFS 远端口径 | push 前确认远端 git-lfs 配额；fresh clone 需 `git lfs pull` 取实拍图（`previews/README.md` 已注） |

_本报告由「F-02 大资产出库执行批」产出（2026-10-18）。方案对照 §0；前后体积表 §1；迁移清单 §2；构建零伤证明 §3；引用面 §4；artifacts 登记 §5；边界登记 §6。_
