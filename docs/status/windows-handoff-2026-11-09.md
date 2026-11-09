# Windows 接手须知 —— 结果面凭据绊线工作线（2026-11-09 收尾交接）

> **用途**：后续开发**搬到 Windows** 时，本页是**唯一入口** ⇒ ⇒ 读完本页就能接上，
> 不必翻对话、不必翻 git 历史。
> **配套真源**（职责不重叠）：
> * **做到哪了 + 还要谁拍板** ⇒ [merge-integration-closeout-2026-11-07.md](merge-integration-closeout-2026-11-07.md)（§5 逐条 + §5.1 六条裁决）
> * **每族的覆盖与残余** ⇒ [credential-tripwire-coverage-2026-11-07.md](credential-tripwire-coverage-2026-11-07.md)（族表 26 族 + §2.6 落地事实 + §2.6.3.5/§2.6.3.6 残余）
> * **测试数字** ⇒ [../03-reference/test-baseline.md](../03-reference/test-baseline.md)（**唯一真源**；其余文档只引用不抄写）
> * **能力等级口径** ⇒ [claims.yaml](claims.yaml)（`docs_truth.py --verify` 判 stale）

---

## 1. 一句话现状

**凭据形状绊线（M1）覆盖 16 族 / 26 族**（默认开族已全部接入）⇒ ⇒
**类 B 通道已 PRODUCTION WIRED（默认关）** ⇒ ⇒ **形状层真源已收口到治理层** ⇒ ⇒
**六条 owner 裁决已全部执行** ⇒ ⇒ **下一步 = 收口门禁 + 逐族开类 B 声明（owner 逐族裁决）**。
> ⚠️ **本页数字的时效**：撰写时实测 `claims=58 fail=0`｜`env max_distinct_names=165`｜基线 **519 套件 / 10933 passed / 0 failed / 47 ignored**（2026-11-08 11:03 独占门禁）⇒ ⇒
> ⇒ **接手时这些会变**（他线仍在施工）⇒ ⇒ **以文件为准**（§8 自检清单就是为此准备的）。

## 2. 已交付（提交号可逐条对账）

| 批次 | 内容 | 提交 |
|---|---|---|
| 批次 1 | 默认开三族 `filesystem` / `search` / `repo` 接 M1 | `70e6542a` |
| 批次 2 | `fetch` / `git_write` / `worktree_apply` / `re.*`（9 工具） | `87466ff7` |
| 批次 2c+2d | `ext.*`（10 工具，含 stderr 回显面） | `22705173` |
| 泛化收口 | 证据信封四面遮形上收共享模块（**−113 行**） | `4a500758` |
| 批次 3a | 转录面兜底（治理层扫描器，**默认关**） | `a6a64347` |
| 批次 3b + 3b-2 | 记忆摄取面（J2 队列 + **J1 直写 episodic**） | `586f0092` |
| 批次 3c-1 / 3c-1b / 3c-2 | 占位域归属 → **按会话选域** → **类 B 通道**（默认关） | `55f9ac31` / `e5c2350c` / `35a3ac65` |
| 形状层收口 | **治理层单一真源**（tools 侧 −432 行） | `ae24f8a5` |
| 裁决 E | 四族 `repo_map_tool` / `doc_ops` / `sheet_ops` / `skill_distill` | `3f395b79` + `3ef86afa` |
| 裁决 B | MCP **http** 传输补 `Accept` 头（修「必 406」） | `86419253` |
| 守卫 | 接线守卫期望集扩为**十六族** | `713529e4` |

## 2.5 ⚠️ **收口门禁最后一次未通过（2026-11-09 17:09）—— 接手第一件事**

> **不是环境问题、不是磁盘问题、也不是本工作线的问题** ⇒ ⇒ **是并行线 W3 的一个真缺陷**。

| 项 | 实测 |
|---|---|
| 门禁 | `main.cargo-test` **exit=101** ⇒ `GATE FULL FAILED`（**其余阶段未跑**） |
| 脚本给的归因 | `FAILURE_CLASS=oom` ⇒ ⚠️ **第四次证明它不是真因**（真因在 `logs/gate-full.log`） |
| **真因** | `the_model_face_cannot_probe_someone_elses_visibility_matrix` **FAILED**（`crates/engine/runtime-assembly/tests/w2_unit_d_depth_and_routing_policy.rs:344`）⇒ ⇒ **稳定红**（单独复跑同一条 ⇒ 同红 ⇒ 非偶发） |
| 根因（已定位到行） | panic = `授权请求: UnknownObject("ep-1")` ⇒ ⇒ 失败在 **`request_grant` 这一步** ⇒ ⇒ `object_exists("ep-1")` 为假 ⇒ ⇒ `object_exists` 是**外部注入的 resolver trait**（`crates/engine/memory/src/object_grant.rs:526`）⇒ ⇒ **测试夹具 `grant_harness::harness()` 提供的 resolver 里没有 `ep-1` 这个对象** ⇒ ⇒ **夹具与 W3 新机制的接口不匹配**（不是授权流程的逻辑错） |
| 接手怎么修 | 二选一：① 在夹具的 resolver 里**注册 `ep-1`**；② 把测试里的对象 id 换成**夹具中已注册**的那个 ⇒ ⇒ **两处都在并行线的写域内** ⇒ ⇒ **本工作线未代改**（改它要理解 W3 新机制，且会动他的写域） |

⚠️ **由此产生的两个口径**：
* ⇒ ⇒ **基线**（519 套件 / 10933 passed / 0 failed / 47 ignored，2026-11-08 11:03）**保持不动**
  ⇒ ⇒ **本次未刷新** ⇒ ⇒ **不写未经实测的数字**（门禁没过就没有新数字）⇒ ⇒
  ⇒ ⇒ **上一记是最后一次通过的门禁** ⇒ ⇒ 接手修好后**再跑一次并刷新**。
* ⇒ ⇒ **README 徽章**（`ef2dd5a3`，10933 / 2026-11-08）**保持不动** ⇒ ⇒ 理由同上。

## 2.6 ⚠️ **代码怎么带过去**（最要紧的一节 —— 别重新 clone）

> **现状**：本地分支 `ui-engine-p0-2026-10-24` @ `53cdbd66` **从未 push**（当前网络被阻断：
> `github.com` 被 DNS 劫持到 `198.18.0.6` 保留测试段 + HTTPS TLS 被断）⇒ ⇒
> ⇒ ⇒ **分叉要按「基准」说，三个基准三个答案**（2026-11-09 实测）：
>
> | 基准 | 本地领先 | 远端独有 | 含义 |
> |---|---|---|---|
> | `origin/main`（`18dfadf0`，11-01） | **662** | **0** | 本地**完全包含** main ⇒ ⇒ **main 可安全快进**（但它是默认分支 ⇒ 走 PR 更稳） |
> | `origin/ui-engine`（`658cd192`，11-07 18:35） | **716** | **299** | **双向分叉** ⇒ ⇒ ⚠️ **前端私有仓按精确 SHA 依赖这个分支** ⇒ ⇒ **绝不 force** |
> | `origin/ui-engine-p0-2026-10-24`（`b72ac26e`，11-06 12:50） | **716** | **284** | **双向分叉** ⇒ ⇒ 本工作线所在的远端分支 |
>
> ⚠️ **另一份交接页**
> （[capability-line-windows-handoff-2026-11-09.md](capability-line-windows-handoff-2026-11-09.md)）
> 写的是「714 独有 / 299 独有」⇒ ⇒ **与上表不矛盾，是基准不同**（它按 `origin/ui-engine`）
> ⇒ ⇒ **两页都要看**（那一页有私有前端仓 / 前端依赖 SHA / 三条推送铁律的细节）。
> ⚠️ **一处必须纠正的事实**：那一页写「本线工作所在分支 = `w3-capability-line-2026-11-09`」
> ⇒ ⇒ **实测：该分支在本地与远端都不存在**（`git branch -a --list "*w3-capability*"` 为空；
> 它引用的 `59844fc7` 实际是**本分支 `ui-engine-p0-2026-10-24` 上的提交**）
> ⇒ ⇒ **真实分支 = `ui-engine-p0-2026-10-24`** ⇒ ⇒ 按那个名字会找不到分支。
>
> ⇒ ⇒ ⚠️ **所以绝对不要「在 Windows 上重新 clone 然后继续」** —— clone 拿到的是**远端那个版本**
> （含 284 个提交、**没有**这 715 个）⇒ ⇒ **本轮全部成果会丢**。
>
> **正确做法：用 git bundle 把本地历史整体带过去**
>
> ```powershell
> # ① 传输前/后校验（防截断；sha256 必须一致）
> Get-FileHash D:\apeireth-20261109-1817.bundle -Algorithm SHA256
> # 期望: bdc8b62eefcb32f37515e2211a58595ddf1d1e84151402acd3fb4198b9390105
>
> # ② 直接从 bundle 克隆（一次拿到全部 296 个引用 + 完整历史）
> git clone D:\apeireth-20261109-1817.bundle Apeireth-rust
> cd Apeireth-rust
> git bundle verify ..\apeireth-20261109-1817.bundle   # 应报「记录完整历史」
>
> # ③ 接回远端，把远端那 284 个提交拉下来
> git remote set-url origin git@github.com:Apeireth/Apeireth.git
> git fetch origin
>
> # ④ 切换到本工作线并**合并**（⚠️ **不要** rebase —— 那会改写 715 个提交的历史）
> git switch ui-engine-p0-2026-10-24
> git merge origin/ui-engine-p0-2026-10-24
> #    解冲突后提交 ⇒ ⇒ **此时才 push**（普通 push，**绝不 --force**：
> #    --force 会毁掉远端那 284 个提交）
> git push origin ui-engine-p0-2026-10-24
> ```
>
> **bundle 基本信息**（实测）：`191M`｜`296` 个引用｜`refs/heads/ui-engine-p0-2026-10-24`
> = `e642c8d5`｜`git bundle verify` 报「**记录完整历史**」｜sha256 见上。

## 3. **接到 Windows 后第一件事**：跑一次收口门禁

```powershell
# ⓪ **先修 §2.5 那个缺陷**（并行线 W3 的夹具/接口不匹配 ⇒ 否则门禁必然红在同一处）

# ① 前置检查（三条硬规则，缺一条就别跑）
Get-Process cargo -ErrorAction SilentlyContinue   # 必须为空 ⇒ 无并行编译
Get-PSDrive C | Select-Object Used,Free          # 空闲 < 30% ⇒ 先清 target
python scripts/arch_guard.py --strict            # 必须 VIOLATIONS: 0

# ② 独占全量门禁（唯一执行器）
pwsh scripts/gate_full.ps1 -FailIfBusy

# ③ 三个独立 workspace（根门禁 0 覆盖 ⇒ 必须单独跑）
cd crates-ui; cargo test --workspace --all-targets --locked --no-fail-fast
cd frontend/companion-desktop/src-tauri
  # ⚠️ 需 sidecar 占位（binaries/ 被 gitignore ⇒ 必须手工造）
  New-Item -ItemType Directory -Force binaries
  New-Item -ItemType File -Force binaries/apeireth-x86_64-unknown-linux-gnu
  cargo test --workspace --all-targets --locked --no-fail-fast
cd ../; pnpm test

# ④ 刷新基线（数字真源）：只改 docs/03-reference/test-baseline.md 一处
# ⑤ 同步 README 徽章（两个 README，第 12 行）⇒ 数字必须与基线同日同口径
```

⚠️ **Windows 专属陷阱**（仓内已记，见基线文档的「与上一记基线的差异」段）：
* **D26 环境问题**：Windows 保留端口区间 **50000-50059** ⇒ 回环测试可能撞 ⇒ 表现为**超时型红**。
* **端口冲突** ⇒ 优先用**临时端口**（`127.0.0.1:0`）而不是固定端口。
* ⚠️ **Windows 全量复跑是 owner 裁决「往后放」的那一项** ⇒ ⇒ **本次接手若跑 ⇒
  就是那条裁决的落地** ⇒ ⇒ 跑完**必须**把 Windows 口径登记进基线（**保留 Linux 旧值** +
  〔历史口径（日期）〕注记，**不追溯改写**）。

## 4. 收尾时**必须先知道**的四条纪律（都是踩过的坑）

| 纪律 | 为什么 | 怎么查 |
|---|---|---|
| **门禁必须独占** | 并行编译 ⇒ 磁盘峰值越过守卫阈值 ⇒ 守卫清 `debug/deps` ⇒ **整片 `never executed` 假红** | 跑前 `Get-Process cargo` 必须为空 |
| **磁盘守卫会删测试二进制** | ≥85% 时删 ⇒ 正在跑的构建产物被清 ⇒ 假红 | `/tmp/apeireth-disk-guard.log`（Linux 侧） |
| **env 预算已用满** | `max_distinct_names` 现为 **165**（实测自 `contract.toml:383`）⇒ **新增 `APEIRETH_*` 名会破预算** ⇒ 棘轮红 | `python scripts/arch_guard.py --strict` |
| **脚本给的 `FAILURE_CLASS=oom` 不是真因** | 四次门禁报红**没有一次**是 OOM ⇒ 真因在 `logs/gate-full.log` 里 | 去看真实 panic 文案 / 退出码 |

## 5. 接手后**能直接做的事**（按优先级）

| # | 事项 | 判据 | 谁拍板 |
|---|---|---|---|
| 1 | **收口门禁 + 刷新基线 + 同步 README 徽章** | `GATE FULL PASSED` + 基线数字与徽章同日同口径 | 无人（技术性） |
| 2 | **Windows 全量复跑** | 同上，但登记 Windows 口径 | owner 已裁决「往后放」⇒ **接手即落地** |
| 3 | **逐族开类 B 声明** | 声明清单非空 + 逐族评估「该族返回值会进模型面」的责任 | **owner 逐族裁决**（裁决 A = **暂不开**） |
| 4 | **http 面同族缺口** | `Mcp-Session-Id` / `MCP-Protocol-Version` / POST 事件流响应 | owner |
| 5 | **http 形态口径升级** | 升级判据的**第 0 条前置闸已成立** ⇒ 但**真第三方 http MCP 服务器未实测** ⇒ **不升级** | owner |
| 6 | **剩余 10 族（不适用 / 零明文 / 未装配）** | **多数不是「待接」** ⇒ 「所有工具输出已脱敏」**仍不成立** ⇒ 口径按**适用族**表述 | 口径纪律，无需裁决 |
| 7 | **运行时即时封禁/授权面** | 需「常驻守护通道 + 鉴权」⇒ **独立工作线** | owner（裁决 C = 本工作线不做） |
| 8 | **SQLite busy 3s 预算** | 观察期未出现池等待反复红 ⇒ **维持 3s** | 裁决 F = 维持 |

## 6. **六处已知残余**（不是缺陷，是口径 —— 别「顺手修掉」）

1. **路径面 / 参数回显面不遮**（`doc_ops` / `sheet_ops`）⇒ 嵌在路径/参数里的硬编码密钥**仍会明文进模型面**
   ⇒ 理由：那是**位置与用户自填词**，且敏感路径另有**拒读闸** ⇒ **扩面要逐族另开一批**。
2. **`skill_distill` 的卡名面不入扫描面** ⇒ 卡名是**身份键**且 `id` 由它派生 ⇒ 同形串落卡名上不被遮。
3. **`repo_map` 直调口 `generate()` 返回未遮的原始大纲** ⇒ 生产路径已遮 ⇒ 直调口**当前无生产调用方**
   ⇒ **下沉到库 API 会改公开 API 语义** ⇒ **属产品决策**。
4. **形状层 `sk-` 族 token 以空白为界**（**内核既有语义**）⇒ 凭据形状的**文件名**被遮时 ⇒ 该路径段其后到行尾一并被替换。
5. **M1 不认「键名 + 值」的关系**（那是字段层 M4）⇒ 四族**均只接形状层**（接线前**没有**字段层 ⇒ 接了会引入**新的可见变形**）。
6. **跨进程 / 跨重启续接不成立**（登记簿只在内存 + 密钥掺进程级秘密）⇒ **不假装支持**。

## 7. 三条**容易误判**的口径（别再问一遍）

* **「所有工具输出已脱敏」不成立** ⇒ 26 族里 **12 族**是「不适用 / 设计即零明文 / 未装配（0 装）」⇒
  ⇒ **按「适用族」表述**（当前 **16 族**）。
* **「B1 误伤已修」不成立** ⇒ 类 B 通道已备但**声明清单为空** ⇒ ⇒ **未声明族仍按单向销毁**
  ⇒ ⇒ **通道空转是「可接受的已知状态」**，不是缺陷。
* **「HARDWARE VALIDATED」不成立** ⇒ 真第三方 **http** MCP 服务器**未实测**；本仓的 http 证据来自
  **进程内严格桩** ⇒ ⇒ **准确表述 =「http 请求已合规；真服务器端到端未实测」**。

## 8. 接手时的**自检清单**（五分钟）

```powershell
git log --oneline -20                 # 看最近提交（本文 §2 有对照表）
git status --short                     # 应干净（或只有他人在途）
python scripts/arch_guard.py --strict # VIOLATIONS 0
python scripts/docs_truth.py --verify  # claims=58 fail=0（⚠️ stale 取决于在途状态，见下）
python scripts/docs_links.py --check   # 断链 0
pwsh scripts/check-neutral-terms.ps1   # OK
```
⚠️ **若 `stale` 非 0** ⇒ 那是「源码动过而 `verified_at` 未更新」⇒ ⇒ **先核实实质是否被触及**
（**不要盲盖**）⇒ ⇒ 再 bump 到新提交 SHA ⇒ ⇒ `.github/workflows/docs-truth.yml` 跑的是
`--block-stale` ⇒ ⇒ **不 bump 会卡 CI**。
⚠️ **撰写本页时实测 `stale=32`** ⇒ ⇒ 那是**他线 42 个在途文件**导致（他线自己的 claim 尚未重盖）
⇒ ⇒ **不是本工作线的问题** ⇒ ⇒ **接手时先看他线是否已提交** ⇒ ⇒ 若已提交 ⇒ ⇒ 那批 claim 由
**他线**重盖（**不要替他盖** ⇒ 那是「盲盖」）⇒ ⇒ 若他线已收工 ⇒ ⇒ **你**再逐条核实并 bump。
