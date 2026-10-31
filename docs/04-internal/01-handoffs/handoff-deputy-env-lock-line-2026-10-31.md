# 副总指挥线交接文档（handoff-deputy-env-lock-line-2026-10-31.md）

> **给谁看**：接续本工作线的人或 AI。**你不需要读过任何对话记录 —— 本文自足**。
> **本线是什么**：把「不敢信」变成「可测量」—— ① 挖真缺陷 ② 把每个坑变成跑不掉的机器守卫
> ③ 把**度量本身**修到可信（门禁、棘轮、真源对账、台账口径）。
> **上位文档**：协作看板 `/home/bai/Projects/apeireth-coordination/lead-commander-board.md`
> （**仓外、不进 git**；本线负责 **§三十四 – §一百三十七**）；总指挥 2026-10-30 裁定
> `docs/status/verification-signoff-ruling-2026-10-30.md`（**裁定是上位**，本文只做归档与复盘）。
> **状态（2026-10-31）**：PR #95 = **63 commits / 116 files / +40711 −1998**；
> **⚠️ `mergeable_state = dirty`（与 main 冲突）**；本地门禁 **25 项全绿**。

---

## 0. 三分钟接手

1. **现在做什么**：**§2 同步 main** —— 这是唯一的阻塞项（一次做完解掉「冲突」+「cargo-audit 红」两个问题）。
2. **动手前必读**：§5 方法论与纪律（8 条）—— 不读它你会重犯本线已经踩过的坑（§6 记了本线最贵的 4 个教训）。
3. **铁律**：§3 禁区（LOCKED 5 项 / `docs/status/**` 只允许机械 `--gen` + `verified_at` 重打 /
   债栏只允许**单行**登记 / 棘轮基线**只减不增**）—— 破一条就是**前功尽弃**。
4. **随手记**：每完成一步 → 门禁全绿 → 提交（body 必带 0 装真账 + O-6 三阶 + 影响面）→
   **攒批推**（§7 工具：一次 ref 更新 ⇒ 一次 CI）。
5. **不确定的**：先查看板 §一百三十四–一百三十七（总指挥裁定的分工与已给的两条请求），
   再查 `docs/status/verification-signoff-ruling-2026-10-30.md`。

---

## 1. 这条线已经交付了什么（可复核）

| 类别 | 读数 | 证据落点 |
|---|---|---|
| **真缺陷族** | **25 族** | 各自的 commit body（`0 装真账` 段列了复现命令与读数） |
| **fuzz target** | **57 个**（本地各 3000 runs / 0 crash） | `fuzz/fuzz_targets/` + `.github/workflows/fuzz.yml` 矩阵；`scripts/check_fuzz_bins.py` 守卫「name↔path / 无孤儿 / CI 矩阵覆盖」 |
| **自查脚本** | **8 个** | `scripts/{check_fuzz_bins,check_mod_files,check_ci_gate_coverage,check_env_race,check_self_status_vocab}.py` + 复用 `check_coverage_ratchet` / `audit_unwired` / `group_unwired_baseline` |
| **本地门禁** | **25 项**（`gates.sh` 一次跑完） | §4 |
| **债栏** | **D33**（新增一行，同族追加器「两次写 ⇒ 并发丢记录」，9 处已修 + core 残余点如实留账） | `docs/04-internal/backend-enhancement/00-backlog.md` |
| **债栏核对** | 状态不明 10 条 → **核完 7 条**（D4 / D8 / D19 / D20 / D21 / D23 / D24），每条带**可复核读数** | 同上（各行「状态注记（2026-10-2x 复核批）」） |
| **文档固化** | fuzz 规范**+68 行**（假阳性三型 / 私有契约两手法 / 4 条新硬坑）；矩阵**真源对账注记** | `docs/02-guides/fuzz-target-authoring.md` · `docs/03-reference/capabilities-matrix.md` |
| **阶段 6** | 三段闭环（清单 ✅ + 签核 ✅ + **反思归档 ✅**）⇒ ADR-0011 R6-C 达成 | `docs/architecture/handbook.md` **§10**（逐条引用台账 id） |
| **看板** | **5212 行**（本线 §三十四–§一百三十七） | 仓外 |

**最重的一条**：`crates/foundation/core/src/gate.rs:225` —— `HAMode::MultiHuman` **忘配 `multi_sign`**
⇒ `map_or(1, ..)` 把门槛降成 1 人 ⇒ **静默退化为单签**（不报错、不告警、审计看不出差别）；
而校验器侧是 **fail-closed** ⇒ **两处口径不一致，最弱的一条生效**。
**实测确证**：`MultiHuman` + `multi_sign: None` + 1 名真人 ⇒ **`Allow`**。
**为何长期没被发现**：全仓**所有** `MultiHuman` 测试**都配了** `multi_sign` ⇒ **那个默认分支从未被任何测试执行过**。
⇒ 已钉成 `#[ignore]` 显形测试（`crates/foundation/governance/tests/governance_chain_drill.rs`
`multi_human_without_policy_must_not_degrade_to_single_human`），**修法一行**写在测试的文档注释里
（**core 不是本线的区** ⇒ 本线只报不改）。

---

## 2. ⭐ 最优先：同步 main（唯一的阻塞项）

### 现状（2026-10-31 实测）
```
main                    : 0268e208a7b1  (Merge PR #107)
我的分支                 : dd53730a71d8
ahead / behind          : +63 / -108      ⇒ diverged
mergeable_state         : dirty          ⇒ 与 main 冲突
cargo-audit workflow    : ❌ 红            ← 不是新 bug！
```
**`cargo-audit` 红的真因**：我分支上带的是**修复前**的旧版 `.github/workflows/cargo-audit.yml`
（blob `57b0c70ee6`），而 main 已是修好的版本（blob `142315cf7d`；修复由 PR #64 提供，**已闭合并进 main**）。
⇒ ⇒ **它跟冲突是同一个问题：没同步** ⇒ ⇒ **同步完一起解**。

### ⚠️ 通道限制（**别踩**）
| 方式 | 结果 |
|---|---|
| `git fetch origin`（SSH） | ❌ `Permission denied (publickey)` —— 本机**无 SSH 密钥** |
| `git fetch https://…` | ❌ `GnuTLS recv error (-110): TLS 链接非正常地终止` |
| **GitHub REST API** | ✅ **可用**（本线全程走这条）⇒ `apipush.py` 就是这么推的 |

⇒ ⇒ **同步 main 只能走 API 通道**。建议做法（**按成本从低到高**）：

1. **先试最省事**：本地已有 `git` 对象库时，用 API 把 main 的**末端 commit 链**（约 108 个）逐个
   重建成本地对象（`git cat-file` 内容 ⇒ `git hash-object -w` ⇒ `git commit-tree`）——
   Git 的对象 SHA 是**内容寻址**的，**重建出的 SHA 必然等于远端 SHA** ⇒ ⇒ 之后 `git merge` 就是**真 3-way merge**。
   （本线的 `apipush.py` 反向用过同一套原理：它建 commit 后**校验远端 SHA == 本地 SHA**，逐环对上才推 ref。）
2. **次选**：只取「main 相对 merge base 改动的文件」与「我改动的文件」的**交集**做 3-way
   （`git merge-file` + API 取三方内容）⇒ 只解决真冲突面。
3. **兜底**：与总指挥商量**合并顺序**（他在合 PR #94/#107 ⇒ 或可由他先合，再让我 rebase）。

### 冲突面预估（**已实测，别再猜**）
⚠️ `GET /compare` 的 `files` 数组**最多返回 300 个** ⇒ PR #94 正好改了 300 个 ⇒ **列表被截断** ⇒
**必须逐文件 blob 比对**（`GET /contents/{path}?ref=` 比 sha）。实测结论：

| 文件 | main | #94(debt) | #95(本线) | 结论 |
|---|---|---|---|---|
| `docs/04-internal/live-verification-ledger.md` | 438a57fe | 438a57fe | 438a57fe | ✅ 三方一致 ⇒ 本线改它**零冲突** |
| `docs/architecture/handbook.md` | 6a6ed5ee | 6a6ed5ee | 6a6ed5ee | ✅ 三方一致 ⇒ 零冲突 |
| `docs/status/verification-ledger.yaml` | 07eda332 | **502fb166** | 07eda332 | ⚠️ **#94 的文件 ⇒ 本线不碰** |
| `.github/workflows/rust.yml` | 8065f139 | dc0a168a | 4617a1a0 | ⚠️ **三方都改过 ⇒ 真冲突面**（独立议题） |

⇒ ⇒ **本线的文档交付与 #94 零冲突**；**真正的冲突面是 `rust.yml`**（本线加了 2 个 job：
`ci-gate-coverage`〔**已按裁定撤掉**〕与 `env-race-inventory`；总指挥那边也改了它）⇒ **建议由他先定 rust.yml 的形状**。

---

## 3. 禁区（**破一条即前功尽弃**）

| # | 禁区 | 说明 |
|---|---|---|
| 1 | **LOCKED 5 项**（`docs/architecture/contract.toml` `[locked]` 真源） | `core/src/eight_anchors.rs` · `core/src/philosophy.rs` · `core/src/onion.rs` · `Cargo.toml`（version）· `legacy/`（只引用） |
| 2 | `crates-ui/**` · `frontend/companion-desktop/**` · `.github/workflows/ui-engine-ci.yml` | 前端面 |
| 3 | `docs/status/**` | **只允许**：机械 `--gen`（`wiring_report.py --gen` / `docs_truth.py --gen`）+ 重打 `verified_at` |
| 4 | `docs/04-internal/backend-enhancement/00-backlog.md` | **只允许单行登记**（一行一行加，不重排表格） |
| 5 | 棘轮基线族 | `scripts/{doc-caliber,coverage,unwired}-baseline.txt` · `scripts/arch-baseline.txt` · `docs/_known-missing.tsv` ⇒ **只减不增**，**永不碰基线** |
| 6 | 总指挥已列明文件 | `gateway/src/{principal_forget,panels,user_admin,lib,panel_memory_update}.rs` · `tools/src/{repo_map,repo_map_tool,lib}.rs` |
| 7 | 总指挥口头区 | `memory` / `provider` / `runtime` / `runtime-assembly` / `orchestration` / `sdk` / `cli` ⇒ **只读引用，不改** |
| 8 | **不引新外部依赖** | 本线 63 个 commit **零新增依赖**（保持） |
| 9 | 工作树一律**仓外** | 本线工作树：`/home/bai/Projects/apeireth-consolidate-wt`（分支 `feat/env-lock-crate-level`） |

### 本线**可以自由改**的区（其余是你的）
- `crates/capabilities/tools/**`（**36 个文件** —— 本线主战场）
- `crates/foundation/{protocol,plugin,governance,credentials}/**`
- `fuzz/**` · `scripts/check_*.py` · `.github/workflows/{fuzz.yml,mod-files.yml}`
- `docs/02-guides/fuzz-target-authoring.md` · `docs/03-reference/capabilities-matrix.md`
  · `docs/architecture/handbook.md` · `docs/04-internal/{live-verification-ledger.md,01-handoffs/}`

---

## 4. 门禁怎么跑（**25 项**，一次跑完）

```sh
# 仓外脚本（不在 git 里！克隆仓后要自己重建或找我要）
cd /root/Documents/deepseek-harness/default-workspace
bash gates.sh /home/bai/Projects/apeireth-consolidate-wt
```
> ⚠️ **25 项**（此前口头说「22 条」是**本线的数错**，以 `gates.sh` 实际输出为准）：

| # | 检查 | # | 检查 |
|---|---|---|---|
| 1 | `docs_index.py --check` | 14 | `cargo test --workspace` |
| 2 | `docs_links.py --check` | 15 | `cargo check`（fuzz 全部 target 编译） |
| 3 | `docs_truth.py --verify --check` | 16 | `cargo check --workspace --all-targets` |
| 4 | `docs_truth.py --verify --block-stale` | 17 | `check_self_status_vocab.py` |
| 5 | `check_doc_caliber.py` | 18 | `audit_unwired` + `group_unwired_baseline`（只读） |
| 6 | `wiring_report.py --verify` | 19 | `check_env_race.py`（候选清单） |
| 7 | `wiring_report.py --crosscheck` | 20 | `check_ci_gate_coverage.py`（CI 逐步覆盖） |
| 8 | `wiring_report.py --check-env-registry` | 21 | `check_fuzz_bins.py` |
| 9 | `verify_ledger.py --quiet` | 22 | `check_coverage_ratchet.py` |
| 10 | `arch_guard.py --strict` | 23 | `check_mod_files.py` |
| 11 | `arch_doc.py --check` | 24 | `cargo fmt --all --check` |
| 12 | `secret-scan.ps1 -Mode scan-all` | 25 | `cargo clippy --workspace --all-targets` |
| 13 | `secret-scan.ps1 -Mode scan-history` | | |

### 跑门禁的 4 个环境前提（**不设就会假红/假绿**）
```sh
export RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WRAPPER=""   # sccache 在本机是坏的 ⇒ 必须清空
export CARGO_TARGET_DIR=/home/bai/.cache/gate-target-<wt> # ① 绝不放 /tmp（tmpfs，RAM 支撑）
                                                     # ② 绝不用同一目录跑不同分支（并发互踩）
```
- **③** `pwsh` 需在 PATH（secret-scan 两步用它）；
- **④** `cargo +nightly fuzz` 需 nightly（**repo 根 `rust-toolchain.toml` 钉 1.97.1 stable**
  ⇒ ⇒ **必须显式 `+nightly`**，否则会被 stable 覆盖）。
⇒ **跑完看结论行**（`✅ 全绿` / `❌ 有红`）；**别只看前半段**——本线踩过「门禁报全绿但 cargo test 是红的」。

---

## 5. 方法论与纪律（本线的 8 条）

1. **每个 commit 的 body 必须带**：**0 装真账**（复现命令 + 实测读数 + 「未改什么」）+
   **O-6 三阶审查**（总体最优 / 系统最优 / 架构最优，**每阶都要写「拒的 alternatives + 为什么拒」**）+
   **影响面**。缺一条 = 不合格 commit。
2. **只宣称测过的**：「fuzz 报了 crash」**必须先在库层复现**才算数；
   **子代理的 crash 不是证据**；**子代理的补丁/oracle 要按合同复核** ⇒ **修 oracle，不修产品**。
3. **假阳性优先怀疑判据**：本线累计 **19 次**假阳性，**全部**来自「我的断言比实现更强/更偏」，
   **不来自实现有 bug** ⇒ ⇒ 假阳性率高时**先怀疑自己**。
4. **新写的检查脚本必须先证伪**（装饰性的检查比没有更坏）⇒ 本线的
   `check_ci_gate_coverage.py` **证伪了两次**才对（注释里的命令名会造**假绿** ⇒ 只比对**非注释的**真正命令）。
5. **门禁必须逐「步」对齐 CI**（不是按 job 名）⇒ 本线因此抓到 **5 个真缺口**
   （含 CI 有而 `cargo test --workspace` **不覆盖**的 `cargo check --workspace --all-targets`）。
6. **棘轮只减不增**；`--update-baseline` 会**掩盖漂移** ⇒ 只在 diff 确认后逐条人工改。
7. **编号只增不改**：台账/债栏的编号被别的文档按号回引 ⇒ 核销的行**保留编号**（标「已核销出账」），
   证据转入绿表 ⇒ **历史引用不断**。
8. **不擅自关信号**：例：Dependabot 告警（21 条）**不擅自 `ignore`**（关安全信号属重大决策 ⇒ 交 owner/总指挥）。

---

## 6. 本线最贵的 4 个教训（**比成果更值钱**）

| # | 教训 | 代价 |
|---|---|---|
| 1 | **「挂账表里有行」≠「那行是待办」** —— 我读台账只读 §2，没读 §1.4 的核销批注 ⇒ 向 owner 提了 4 项**没有一项非要 owner 不可**的问询 | 打断 owner 一次 + 差点与总指挥裁定冲突（我第一版把 #8/#9 写成「仍挂账」，而裁定说 **#9 早已核销**、**#8 本仓侧已闭环**）⇒ **幸好先读了裁定** |
| 2 | **CI 红时先定位「红在哪一步」** —— 我以为「secret-scan 红 = 扫到了什么」，花整轮查历史字面量；**真因在准备步**（`apt-get install powershell` 在 Ubuntu 无此包 ⇒ 而 runner 本就预装 pwsh ⇒ **两个真扫描步根本没跑到**） | 一整轮弯路 + 修了两个**自己造成**的 CI 红（一个恒红 job + 一个必失败准备步） |
| 3 | **恒红的 job 与恒绿的 job 一样没有信息量** —— 我把一个**本地**校验器做成了 CI job（它要读**仓外**的 `gates.sh` ⇒ CI 里没有 ⇒ 必然 `return 1`） | CI 长期挂红 ⇒ 污染信号 ⇒ 后来直接从 CI 撤掉、只留本地门禁 |
| 4 | **「同族」= 必须两处都钉** —— 同族缺陷只钉一处 ⇒ 另一处照旧能回退（例：`doc_ops` 与 `sheet_ops` 的禁填字段矩阵；追加器同形共 **9 处**） | 补钉了 9 处追加器 + 2 个同族字段矩阵 |

---

## 7. 仓外工具（**不在 git 里**，交接必须带走）

路径：`/root/Documents/deepseek-harness/default-workspace/`

| 文件 | 作用 | 关键用法 |
|---|---|---|
| `gates.sh` | **25 项门禁**（§4） | `bash gates.sh <worktree>` |
| `apipush.py` | **API 通道推送**（git 直连在本机被断） | `python3 apipush.py <branch> <worktree>`；**支持多 commit 链** ⇒ **逐个建 commit、最后只 PATCH 一次 ref ⇒ 只触发一次 CI**；四重校验（blob/tree/commit SHA 逐环相等才推） |
| `apialign.py` | 把本地与远端 commit **对齐**（SHA 逐环核） | 推送报「链条不对齐」时先跑它 |
| `check_env_race.py` 的清单 | 跨测试 env 竞态候选（**报告型**，恒绿） | 仓内副本 `scripts/check_env_race.py` |
| `/root/.apeireth-secrets/minimax.key` | owner 提供的临时 MiniMax key（**600**，目录 700，**在 git 之外**） | 用完即弃；**零 commit、零 print**；`secret-scan` 双守 |

⚠️ **推送纪律**：**攒批推**（一次 ref 更新 ⇒ 一次 CI）。本线一次推 1 个 commit 曾让 **85 个 job 重跑**；
改造成多 commit 链后，2 个 commit 一次推只触发**一轮**。

---

## 8. 未完的活（分四类，**别混**）

### 8.1 🔴 阻塞（等他）
| 事项 | 状态 |
|---|---|
| **同步 main**（§2） | 本线**唯一**的代码侧阻塞；解掉冲突 + cargo-audit 红 |

### 8.2 🟠 等 owner（**不阻塞任何核销**，可随时放掉）
| 事项 | 说明 |
|---|---|
| **Windows 真机**跑一次 `pwsh scripts/windows-live-check.ps1` | 本线写好并在 Linux 自测过（PASS 8 / FAIL 2），**从没在 Windows 上跑过**；必做项：③ shell 沙箱墙探针 · ② UI 最后一厘米（事件 + 审批卡）· ③ D32 note 在 UI 可读；交叉项：D19 试跑 SHA-256 · 回归 · fuzz 冒烟 · gateway HTTP 探针 |
| **#10**（台账挂账）图形卸载真删验证 / 斜杠菜单点击流 | 需 Windows 真机 |
| **#1 / #9 的厂商侧当期证据**（可选） | 走 **owner 侧只读复核命令**（密钥**不出 owner 的机器**，只把输出贴回）⇒ **不必把密钥交给我** |

### 8.3 🟡 等总指挥（**已在看板登记**）
| 事项 | 出处 |
|---|---|
| `docs/status/verification-ledger.yaml` 的 **`stage6_drill`** 字段（`NOT RUN → 达成`）在 **#94 的 10 条版本**里 ⇒ **本线不碰** | 看板 §一百三十七 |
| **两支 live 测试的端点/模型改为可配**（默认仍 DeepSeek，行为不变）⇒ 换厂商即可复跑 #9 | 看板 §一百三十七；实测：MiniMax key **复跑不了**（`openai_compatible_live` 硬编码 `DEEPSEEK_BASE_URL`；`organ_live_llm` 硬编码模型 `deepseek-v4-flash`） |
| **`.github/workflows/rust.yml` 的形状**（三方都改过 ⇒ 真冲突面） | §2 冲突面表 |
| **D23**：是否 `ignore` 归档/research 锁文件的 5 条 Dependabot 告警（**不擅自关信号**） | 看板 §一百二十六 |
| **D24**：那 10 条生物机制要不要进 `claims.yaml`（= 要不要把「有读数」升格为「对外承诺」） | 看板 §一百二十六 |
| **D19**：M 轴 3D 立项不立项（**技术可行但暂不立项** ⇒ 待 owner） | 看板 §一百二十八 |

### 8.4 🟢 本线可继续做（不依赖任何人）
| 事项 | 价值 |
|---|---|
| **扩 fuzz 面**（按「**判错代价最大**」选题：真设备 > 上下文注入 > 告警 > 教学 > 数值） | 本线已做 **9 个**（`education`/`skill_search`/`guard_watch`/`device_control`/`path_guard`/`doc_ops`/`content_feed`/`worktree_apply`/`sheet_ops`）⇒ 矩阵 48 → 57 |
| **`check_ci_gate_coverage.py` 的覆盖再验**（它会随 CI 变更漂移 ⇒ 每次改 workflow 后必跑） | 防「门禁漏一步」复发 |
| **台账/债栏的剩余状态不明项**（D1/D2/D3 属外部阻塞 ⇒ 不用碰） | — |

---

## 9. 给接手者的 3 个具体请求（对总指挥）

1. **先定 `.github/workflows/rust.yml` 的形状**（三方都改过）⇒ 否则同步 main 时必冲突，
   且**冲突面在 CI 口径文件**（最贵的冲突）。
2. **更新 `verification-ledger.yaml` 的 `stage6_drill`**（反思归档已在 `handbook.md` §10 就位 ⇒ 三段齐了）。
3. **两支 live 测试可配化**（端点 + 模型名，默认不变）⇒ 之后 #9 可用任意 openai-compatible 厂商复跑，
   **不再卡在「厂商 key 撤销」上**。

---

## 10. 相关文件索引

| 文件 | 作用 |
|---|---|
| `/home/bai/Projects/apeireth-coordination/lead-commander-board.md` | 协作看板（**仓外**；本线 §三十四–§一百三十七；§一百三十四–一百三十七 = 本轮交接上下文） |
| `docs/status/verification-signoff-ruling-2026-10-30.md` | **总指挥 2026-10-30 裁定（上位文档）** |
| `docs/architecture/handbook.md` §10 | 本线的**阶段 6 反思归档**（逐条引用台账 id） |
| `docs/04-internal/live-verification-ledger.md` | 实测台账（§1 绿表 / §2 挂账与已核销出账） |
| `docs/status/verification-ledger.yaml` | 四层验收**结构化真源**（#94 的 10 条版本） |
| `docs/02-guides/fuzz-target-authoring.md` | **fuzz 规范**（判据三型 / 私有契约两手法 / 4 条硬坑） |
| `docs/04-internal/backend-enhancement/00-backlog.md` | 债栏 D1–D33（本线新登记 D33；复核批注记 7 条） |
| `scripts/check_ci_gate_coverage.py` | **CI 逐步门禁覆盖**（本地专用；**不是** CI job） |
| `scripts/check_fuzz_bins.py` · `scripts/check_env_race.py` · `scripts/check_mod_files.py` | 本线新增的自查守卫 |
