# 跨线合并收口进度页（2026-11-07）

> **用途**：AGENTS.md §0 要求的「做到哪了」的唯一权威页。本页只写**事实与证据**，
> 数字均带命令与日期；**还差什么**逐条列出（§5）。
> **状态**：🟡 主体完成 · 有明确遗留（§5 / §6）｜**2026-11-07 更新**：§5-1「frontend JS 侧未实测」**已消**（`a1048c90`）；§5-2「两个 UI 门禁并存」**已消**（门禁归并批）
> **Owner**：主代理（总工程师线）｜**执行记录**：`docs/adr/0024-merge-integration-contract-and-adr-numbering.md`

## 1. 这条工作线是什么

2026-10-24（分叉点 `8ac0f9de`）后两条线各自演进：总指挥线（本线，107 提交）与并行线
（**625 提交**）。实测分叉深度：`git diff --shortstat` = **1414 files / +216783 / −67706**。
⇒ **不是一次小对齐，是主线级整合** ⇒ owner 裁决「合呗」⇒ 本页记录整合与收口。

## 2. 合并基本盘（已定案并落库）

| 项 | 值 | 依据 |
|---|---|---|
| 合并提交 | **`35cf3710`**（双亲 `5bd643a7` + `7ed1e3ed`） | `git log --first-parent` |
| 收口批次 | 合并后 **14 笔**（`bf4d5d0e` … `23fb7a07`） | 同上 |
| 门面 crate | **保留** `apeireth` ⇒ `members = 31` | ADR-0019；契约 `[workspace].members` |
| 生产依赖边预算 | **118**（对方线的 111 是它自己 30 成员树的实测，**不是谁抄错**） | 契约 `[budgets]` |
| 开关预算 | **160**（守住，未抬） | 契约 `[env].max_distinct_names` |
| 独立 workspace 登记 | **1 条**（`ui_engine`，7 crate）⇒ 原 `ui_core_slice` 已按 ADR-0024 删除 | ADR-0024 §3 |
| MCP 能力 | **双方并集**（不是二选一） | 见 §3 |

## 3. 合并后各域实测（**逐条带命令与日期**）

### 3.1 门禁（2026-11-07，Linux x86_64，rustc/cargo 1.97.1）

| 门禁 | 结果 |
|---|---|
| `pwsh scripts/gate_full.ps1 -FailIfBusy` | **exit 0**，脚本结论 `GATE FULL PASSED`（5 项全 OK） |
| 全量测试 | **504 套件 / 10782 passed / 0 failed / 47 ignored**（main 427 / image 77） |
| `python3 scripts/arch_guard.py --strict` | **VIOLATIONS 0 / STALE 0** |
| `python3 scripts/docs_truth.py --verify --check` | **claims=51 fail=0 stale=0** |
| `python3 scripts/docs_links.py --check` | **断链 0** |
| `python3 scripts/docs_index.py --check` | **问题数 0** |
| `pwsh scripts/secret-scan.ps1 -Mode scan-all` | **[OK] No secrets found** |
| `pwsh scripts/check-neutral-terms.ps1` | **OK**（992 changed files 全中性） |
| 契约 TOML 解析 | **OK**（`members=31` / `max_crates=31` / `max_edges=118` / `env=160`） |

### 3.2 **根门禁覆盖不到的 3 个独立 workspace**（必须单独跑）

| workspace | 实测 | 覆盖它的门禁 |
|---|---|---|
| `crates-ui/` | **432 passed / 0 failed / 4 ignored**（2026-11-07 合并后实测；⚠️ 取自**他线在途改动落地前** ⇒ 数字随其施工变动；⚠️ **doc-test 未跑** ⇒ 见 §5-3） | `ui-engine-ci.yml`（**2026-11-07 已归并**：原 `ui-core-ci.yml` 删除 —— 它的 4 关重复关已在前者覆盖，独有的 doc-test 关**刻意未引入**（那一关红）） |
| `frontend/companion-desktop/src-tauri/` | **171 passed / 0 failed / 2 ignored**（Tauri Rust 侧）| `companion-desktop-ci.yml`（装 Tauri 系统依赖 + sidecar 占位 ⇒ **cargo check / cargo test**）⇒ **独立的 CI 已守** ⇒ ⇒ **不纳入根门禁**（纳入 = 与既有 CI 重复编译） |
| `frontend/companion-desktop/`（JS 侧） | ✅ **83 of 83 suites passed**（2026-11-07 实测，exit 0）—— 本机 Node 无内置 TS 支持 ⇒ 走**类型擦除降级路径**（`tests/ts-strip-hooks.mjs`，用仓内**既有** `typescript` devDependency）⇒ 修前本机 **66 of 83 suites failed**（`ERR_NO_TYPESCRIPT`，**模块加载阶段**就死）⇒ **已从「未实测」转为「全绿」** | `companion-desktop-ci.yml` 的 `pnpm test` 步骤（Node >=22 走原生擦除；**2026-11-07 起**无 TS 支持的 Node 也能跑完）⇒ **独立的 CI 已守** |

### 3.3 合并后逐项处置（**每条都有归因**，不是「重跑一次就绿了」）

| 合并后症状 | 判定 | 处置 | 提交 |
|---|---|---|---|
| **4 处编译级缺陷**（双层 Arc / 丢返回值 / 死代码） | **自动合并产物**（git 认为已合并 ⇒ 0 冲突标记） | 主代理逐处定位并修 | `bf4d5d0e` |
| **3 处测试红**（MCP 夹具 + 出网面期望） | **夹具/断言跟不上语义** | 夹具改走**真目录条目**（照仓内正确范式）⇒ **不放宽库** | `1a8ed2be` |
| **1 处真红**（策展表漏登 `tool.repo_map`） | **真缺口**（对方线接进生产、策展表来自本线） | **补登**（不放宽闸门） | `1a8ed2be` |
| **13 条 size-grow** | 11 条 =「导入对方已有裁决」；**2 条 = 真增长** | 2 条真增长**拆文件降回基线以下** ⇒ **不抬上限，棘轮净收紧**（size 84→79） | `53f0bbc4` + `80190015` |
| **4 条同源偶发红**（SQLITE_BUSY） | **既有缺陷**（三树同窗取证）+ 负载敏感 | **根因收口**（写线程同步收口 + 有界退避加固） | `60193565` |
| **1 条红**（council 聚合顺序） | **不可复现**（95 次全绿）+ **当前实现下断言不可达** | **未改断言、未删测试**，登记为已知项 | — |
| **全 workspace fmt 红** | 我只验了自己改的文件，**没跑全 workspace 口径** | `cargo fmt --all`（**纯格式**，净 −6 行） | `0c215b74` |
| **Tauri `--locked` 失效** | 合并把对方线的锁（rc.4）与本线版本（rc.5）配在一起 | **最小编辑 2 行**（未重生成锁、未升级任何依赖） | `0199c8b6` |

### 3.4 声明真源（45 条 claim 逐条**实跑 verify_cmd** 后重盖，**不盲盖**）

- 合并后 `claims.yaml` 从 29 条扩到 **51 条**（对方线 22 条生物/模块/安全族并入）。
- 复核并重盖：**29 条**（`4ee386e7`）→ **5 条**（`be964b27`，配置结构搬家后的 sources 补列）
  → **11 条**（`507653b4`，逐条实跑 verify_cmd：12/6/43/16/24/7/249/4/12/61/20 passed，0 failed）。
- **两处默认档订正**（ON→OFF，均有代码取证）：`sec.object_access`（判定器只挂 shell 面，
  而 shell 默认关）、`sec.output_injection_guard`（唯一装点在 fetch 分支内，而 fetch 默认关）。
- **6 条 MCP 安全不变量**：合并后**代码级逐条复核，全部成立**（人批权唯一归治理链 /
  桥内 deny-only / `Decision::Deny ⇒ PreVerdict::Deny` / 模型面名与能力 id 分离 /
  族判定段边界 / 能力身份唯一性）。

## 4. 合流拓扑（已完成）

- 合并分支 `merge/integrate-2026-11-06` 是 main HEAD 的**直接后代** ⇒ **fast-forward 合流**（零冲突）。
- **机器核验**「合并改动的 1119 个文件」∩「main 在途改动的 69 个文件」= **0 交集**
  ⇒ ⇒ **fast-forward 不覆盖任何在途改动** ⇒ 69 项 crates-ui 在途改动**完好无损**。
- main 现已 = `23fb7a07`；工作树仍留 69 项在途改动（**属另一条线，未提交、未触碰**）。

## 5. **还差什么**（逐条，每条有判据）

| # | 差什么 | 为什么没做 | 判据（做完怎么验） |
|---|---|---|---|
| 1 | ~~**frontend JS 侧未实测**~~ ⇒ ✅ **已消**（2026-11-07，`a1048c90`）：**83 of 83 suites passed**（exit 0）⇒ 缺口已由「换一台 Node」改为「**加一条降级路径**」填掉 |（原缺口来源：本机 Node 无内置 TS 支持；处置：类型擦除加载钩子，**0 新外部依赖**、**不改**任何套件断言） |**已闭环** |
| 2 | ~~**两个 UI CI 门禁并存**~~ ⇒ ✅ **已消**（2026-11-07）：**删** `ui-core-ci.yml`（它的 check/clippy/fmt/test 四关在 `ui-engine-ci.yml` 里**已有** ⇒ **零覆盖损失**、CI 少跑一轮重复编译）⇒ ⚠️ 它独有的 **doc-test 关刻意未引入**（那一关在 7-crate crates-ui 上**红** ⇒ 引入即让 CI 变红）⇒ 该缺口另列为 §5-3 | （已处置；doc-test 缺口见 §5-3） | **已闭环** |
| 3 | ~~**crates-ui 的 `cargo check/clippy --all-targets` 红**~~ ⇒ ✅ **已消**（2026-11-07）：**触发源是 crates-ui 侧的在途施工**（新建的 kit 模块）⇒ 其收工后实测 **check --all-targets 0 错误**、`cargo test --workspace --doc --locked` ⇒ **3 passed / 0 failed / 4 ignored**（连跑 3 轮同值）⇒ ⇒ **doc-test 关已纳入** `ui-engine-ci.yml`（补回合并收窄的那处覆盖：普通 `cargo test` **不跑** doctest）⇒ ⚠️ **根因仍在**：`apeireth-ui-macro` 的私有函数 `compile_error`（`src/lib.rs:122`）**产出 Ident 而非 `compile_error!`** ⇒ 当前**未触发**，但**同类错误路径仍可能复发** ⇒ 建议后续把该私有函数改名（不属本批写域） | **已闭环**（覆盖已补回；根因改名留待另批） |
| 4 | ~~**把 3 个独立 workspace 纳入 `gate_full.ps1`**~~ ⇒ ✅ **已消（2026-11-07 查实后判定为「不该做」）**：**三个独立 workspace 全都有独立门禁**（`crates-ui/` ⇒ `ui-engine-ci.yml`；`frontend/companion-desktop/src-tauri/` + JS 侧 ⇒ `companion-desktop-ci.yml`）⇒ ⇒ 纳入根门禁 = **与既有 CI 重复编译**（且 `src-tauri` 还需**先编 CLI 二进制做 sidecar**）⇒ ⇒ **不做** ⇒ ⇒ 改为在 **§3.2 逐行写明「由哪道门禁守」**（含具体 workflow 与 job）⇒ ⇒ 「根门禁绿 ≠ 全仓绿」这条限定**依然成立**，但它**有明确的责任人** | （已查实并落表）| **已闭环** |
| 5 | ~~**另 2 条红**~~ ⇒ ✅ **已消**（2026-11-07，`bd90ba0c`）：两处并发用例改**有界退避重试**（5 ms→250 ms 指数退避 + 20 s 总预算）⇒ **断言一条未动**（`assert_same_schema` / 版本簿账比对 / `list().len()==20` / `write_seq==20` 全部未改未放宽）⇒ ⚠️ **诚实边界**：**修前 540 轮、修后 1080 轮均 0 红**（自然复现不出）⇒ ⇒ 价值是「**把处置策略定下来**」，**不是**「证明修好了」⇒ 确定性证据靠**受控复现**（外部写者显式持锁）：旧写法 5006/5013 ms 报 `database is locked`、重试版 6143/6051 ms 跑通、持锁 30 s > 预算 ⇒ 20067/20058 ms **有界放弃**（不无限挂起） | **已闭环**（测试侧）｜⚠️ **根因仍在生产路径** ⇒ 见 §5-11 |
| 6 | **ADR 同号撞车**（0016–0021 各两份） | **裁决为不重编号**（ADR-0024 §2）⇒ 已用**并存索引 + 撞车说明**处置 ⇒ **不是未做**，是**已定案的遗留** | 新写 ADR 用下一个可用编号（0025+）；引用一律用**文件名** |
| 7 | **HARDWARE VALIDATED 不成立** | 真第三方 MCP 服务器端到端未跑（探针是**仓内自建**的） | 用真实第三方 MCP server 跑一次端到端 |
| 8 | **运行时即时封禁/授权面**未就位 | 需要常驻守护通道 + 鉴权设计 ⇒ 独立批次 | 配置改动**不重启即生效** |
| 9 | **其它工具族**结果面未接**凭据形状绊线** ⇒ ✅ **盘点已完成**（2026-11-07，26 族穷尽清单 + 3 批方案+ 类 B 清单，落 `docs/status/credential-tripwire-coverage-2026-11-07.md`）⇒ ⚠️ **实施待 owner 签字**（各批都会让含凭据的输出**变形** = 用户可见结果变形）⇒ 准确表述（经实测订正）：**出网清扫 `EgressSweeper` 只覆盖外部工具族**、**凭据形状绊线内核另覆盖 shell** ⇒ ⇒ 缺口 = 「其它族未接**凭据形状**绊线」⇒ **默认开却零绊线 = `filesystem` / `search` / `repo` 三族**（暴露面最大 ⇒ 性价比最高） | **批次 1**（默认开三族接绊线）落地 ⇒ 结果含凭据时无明文 + 干净夹具**逐字节全等** | **待 owner 签字**（实施前） |
| 10 | **Windows 全量复跑** | owner 裁决「往后放」 | 在 Windows 机器上跑 `gate_full.ps1` 并登记新口径 |
| 11 | **SQLite 并发等待策略未下沉到生产实现**（根因所在）：两处 `BEGIN IMMEDIATE` 仍可能因「对方写事务未提交」而直接失败（`migrations.rs:634` 装订整套 schema 持锁到提交；`sqlite_task_queue.rs:225` 整档读改写持锁到 `:233` commit）⇒ 输方只有 5 s `busy_timeout` ⇒ 生产路径（多进程同时首开同一 memory 库 / 同一任务库）仍会撞 | 上一批只改**测试**（测试侧重试是「替生产扛」）⇒ **生产侧加固另开一批** ⇒ 重试放在 **BEGIN 处** ⇒ 事务体一次都不重复执行 ⇒ ⇒ 副产物：task_queue 的 busy 判定可改**结构化** `ErrorCode::DatabaseBusy` ⇒ 文案匹配随之消失 ⇒ ⚠️ 生产侧预算须比测试侧**保守**（测试 20 s ⇒ 生产建议 ≲8 s，启动路径不能为一把锁等 20 s） | 已有生产调用方受益；副产物消除文案匹配；⚠️ 改的是**公开 API 启动路径** ⇒ 需单独批次与回归 |

## 6. 遗留（**已定案但明确不做**，防止后人重复讨论）

### 6.1 ADR 编号不重排
`0016`–`0021` 各两份（主题不同、互不替代）⇒ **并存索引 + 撞车说明**（`docs/adr/README.md`）
⇒ 代价：「四位递增编号」约定被打破 ⇒ **正文引用一律用文件名链接**。理由见 ADR-0024 §2。

### 6.2 `Signal::update` 的 breaking 不加兼容包装
`FnOnce(&mut T)` ⇒ **`FnOnce(T) -> T`**（合并采用并行线的 `signal.rs`）⇒ breaking
⇒ **属 rc.6 的变更项**（rc.5 已在 crates.io 发布并冻结，**不可能**再发同版本号）
⇒ **仓内 0 调用点** ⇒ 不加兼容包装（两个近义方法 = 长期债）。登记见 ADR-0024 §4 与
`crates-ui/apeireth-ui-core/README.md` 头部。

### 6.3 契约登记归并的代价
独立 workspace 登记 **2 → 1** ⇒ 「独立 workspace 有几个 crate」现在**有唯一答案**（7）
⇒ 切片归属写进 note ⇒ 可追溯到并行线 ADR-0021。

## 7. 怎么复查这条工作线

```bash
# 全量门禁（唯一执行器）
pwsh scripts/gate_full.ps1 -FailIfBusy          # 期望 exit 0 / GATE FULL PASSED
# 棘轮
python3 scripts/arch_guard.py --strict           # 期望 VIOLATIONS 0
# 声明真源
python3 scripts/docs_truth.py --verify --check    # 期望 claims=51 fail=0 stale=0
# 三个独立 workspace（根门禁覆盖不到 ⇒ 必须单独跑）
cd crates-ui && cargo test --workspace --locked                              # 432 passed
cd frontend/companion-desktop/src-tauri && cargo test --locked                # 171 passed
cd frontend/companion-desktop && pnpm test                                     # 83 of 83 suites passed
```
