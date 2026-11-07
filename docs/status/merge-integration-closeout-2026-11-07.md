# 跨线合并收口进度页（2026-11-07）

> **用途**：AGENTS.md §0 要求的「做到哪了」的唯一权威页。本页只写**事实与证据**，
> 数字均带命令与日期；**还差什么**逐条列出（§5）。
> **状态**：🟡 主体完成 · 有明确遗留（§5 / §6）
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
| `crates-ui/` | **432 passed / 0 failed / 4 ignored** | `ui-engine-ci.yml` + `ui-core-ci.yml`（**两个并存**，见 §6.1） |
| `frontend/companion-desktop/src-tauri/` | **171 passed / 0 failed / 2 ignored** | `claims-verify.yml` |
| `frontend/companion-desktop/`（JS 侧） | ⚠️ **未实测** —— 本机 Node 构建**无 TypeScript 支持**（`process.features.typescript = false` ⇒ `ERR_NO_TYPESCRIPT`）⇒ **环境限制**（`node_modules` 未跟踪 ⇒ 非仓库缺陷） | `pnpm test`（83 套件） |

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
| 1 | **frontend JS 侧未实测**（`pnpm test`，83 套件） | 本机 Node **无 TypeScript 支持** ⇒ **环境限制**，非仓库缺陷 | 在有 TS 支持的 Node 上 `pnpm install && pnpm test` ⇒ 0 failed |
| 2 | **两个 UI CI 门禁并存**（`ui-engine-ci.yml` + `ui-core-ci.yml`，`-j` 口径不同） | 归并属 **CI 口径变更**（需装 Tauri 系统库 + sidecar + Node TS 支持）⇒ 属独立批次 | 两个 workflow 归一为一个 ⇒ CI 跑一次通过 |
| 3 | **另 2 条红**（`migrations::concurrent_first_openers…`、`task_queue_store::concurrent_enqueues…`） | 用**裸 `rusqlite::Connection`**、**不走池** ⇒ 池收口**修不到**；且属**既有**缺陷、当前**不可复现**（700+ 次全绿） | 候选修法：输方在 `SQLITE_BUSY` 上做**有界退避**而非 `unwrap` 直接 panic |
| 4 | **ADR 同号撞车**（0016–0021 各两份） | **裁决为不重编号**（ADR-0024 §2）⇒ 已用**并存索引 + 撞车说明**处置 ⇒ **不是未做**，是**已定案的遗留** | 新写 ADR 用下一个可用编号（0025+）；引用一律用**文件名** |
| 5 | **HARDWARE VALIDATED 不成立** | 真第三方 MCP 服务器端到端未跑（探针是**仓内自建**的） | 用真实第三方 MCP server 跑一次端到端 |
| 6 | **运行时即时封禁/授权面**未就位 | 需要常驻守护通道 + 鉴权设计 ⇒ 独立批次 | 配置改动**不重启即生效** |
| 7 | **其它工具族**结果面未接凭据绊线 | 会改所有工具族的行为 ⇒ 独立批次 | 任一工具族的结果面也走凭据绊线 |
| 8 | **Windows 全量复跑** | owner 裁决「往后放」 | 在 Windows 机器上跑 `gate_full.ps1` 并登记新口径 |

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
cd frontend/companion-desktop && pnpm test                                     # ⚠️ 需有 TS 支持的 Node
```
