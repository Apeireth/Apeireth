# Windows 接手须知 —— 能力创造线（插件 / 组件 / 能力开发）· 2026-11-09 交接

[Document-Meta]
Document: docs/status/capability-line-windows-handoff-2026-11-09.md
Class: GUIDE
Status: 🟢 活跃
Last-Modified: 2026-11-09
Author: lead

> **用途**：本线（**能力创造线**）后续开发**搬到 Windows** 时的**唯一入口**。
> 读完本页 ⇒ 知道「远程现在是什么状态 / 做完了什么 / 数字怎么验 / Windows 怎么起 / 下一批做什么 /
> 哪些坑踩过」⇒ 不必翻对话、不必翻 git 历史。
>
> **配套真源（职责不重叠，别重复读）**：
> * **本线目标形态**（八面地图 + 23 族规格 + W0–W6 批次）⇒ [capability-creation-line-spec.md](../01-architecture/capability-creation-line-spec.md)
> * **本线做到哪了**（族级看板 + 波次记录）⇒ [capability-creation-line-progress.md](capability-creation-line-progress.md)（§3.5 W1 / §3.6 W2 / **§3.7 W3** / §3.8 Windows 清单）
> * **能力等级口径（58 条）** ⇒ [claims.yaml](claims.yaml)（`docs_truth.py --verify` 判 stale）
> * **另一条工作线（结果面凭据绊线）的 Windows 交接 + 收口门禁** ⇒ [windows-handoff-2026-11-09.md](windows-handoff-2026-11-09.md)
> * **测试数字真源** ⇒ [../03-reference/test-baseline.md](../03-reference/test-baseline.md)（本页只引用不抄写总量）

---

## 1. 一句话现状

本线 **W1 / W2 / W3 三个波次已全部落地并提交**（14 族新面里 **9 族**已到 **PRODUCTION WIRED**，其余为 IMPLEMENTED 未接线）；
**全仓 `HARDWARE VALIDATED` 仍为 0 条**；代码**已推到公开仓的新分支**（见 §2）；下一步是**在 Windows 上做真机验证 + 补前端入口**。

## 2. 远程仓库现状（⚠️ 接手第一件事：先看这里，别急着 push）

| 项 | 值（2026-11-09 实测） |
|---|---|
| 公开仓 | `https://github.com/Apeireth/Apeireth.git` |
| **本线工作所在分支** | **`w3-capability-line-2026-11-09`** @ `59844fc7`（2026-11-09 17:21，**714 笔**） |
| 前端依赖的引擎分支 | `origin/ui-engine` @ `658cd192`（2026-11-07 18:35）—— **私有前端仓依赖它上面的 SHA `3174666b`** |
| 默认分支 | `origin/main` @ `18dfadf0`（2026-11-01）⇒ **落后 660 笔**，不是最新 |
| 私有前端仓 | `Apeireth/ui-desktop`（**PRIVATE**，2026-11-04 建，`main` @ `bab4af7`，44 文件） |

**分叉事实（已实测，据此决定怎么推）**：`w3-capability-line-2026-11-09` 与 `origin/ui-engine`
**双向分叉** —— 我这边独有 **714 笔**、`origin/ui-engine` 独有 **299 笔**。
⇒ **不要 force-push 任何既有远程分支**（会改写别人正在用的引擎分支，前端按精确 SHA 依赖它）。
⇒ 要合进 `ui-engine` / `main` ⇒ 走 **PR 或显式 merge**，并重新核对前端仓的 `apeireth-ui-engine-rev`。

**三条推送铁律**
1. **先 `git fetch origin` 再判断**（本地远程引用会过期；本线就踩过「以为 SHA 没上远程、实际只是没 fetch」）；
2. **公开仓只推新分支名**，不改写既有分支；
3. 推完**回报分支名 + 提交号**，并说明是快进还是新分支。

## 3. 已交付（提交号可逐条对账）

| 波次 | 提交 | 内容（口径） |
|---|---|---|
| **W1** | `8d43c6e7` + 重盖 `850bda73` | 4 族 **PRODUCTION WIRED**（默认关）+ 3 族 IMPLEMENTED 未接线；额度 crate/edge/env 全 +0 |
| **W2** | `d24dad53`（+ `76e13d9c` 台账） | C-H1 / C-H2 **接线**；C-D1 **账本加固 + 读路径并联**（库级，仍未注册生产） |
| **W3** | `c0cf1ec6`（+ `ee5f856b` 台账/汇总表） | ① **C-H1 隐私闸转 live**（真载体 + `--privacy` + 回合点现读）② **C-D1 升 PRODUCTION WIRED**（生产注册 + 宿主激活口 approve/deny + 两层审计）③ **C-B1 Windows 真机布线**（新 CI + 9 条断言 + 后端 4 个真 bug 修复）④ **C-H2 三处加固** ⑤ **C-B4 审计裁定**（改名 `tool.web_flow`，收窄版） |

**W3 对抗复核挖出的真洞（已修，别重复踩）**
1. C-D1 账本**撤销不是终局**（更高 version 的 active 行可复活授权）⇒ 已补终局闸；
2. C-D1 **信任边界**：账本文件 = 信任锚，其可写性 = 授权能力本身（HMAC 未做，见 §6）；
3. C-B1 **权限不足被报成「已退出」**、枚举失败静默空表、公共契约扫描漏 `snapshot.rs` ⇒ 已修；
4. 治理：`full` 预设会把 `RequireApproval` 改写成放行 ⇒ 高风险面已入**人工审批地板**（存量零影响）。

## 4. 数字怎么验（**每条都可复制执行**，2026-11-09 实测值）

```bash
# ① 架构棘轮（额度 / 分层 / 尺寸）
python3 scripts/arch_guard.py --strict
#   期望：VIOLATIONS: 0，crates=31 edges=118 env=165 max_src_loc=6250 exceptions=0

# ② 本线族级双判据守卫（探针 ↔ 状态词互校）
python3 scripts/check_creation_line.py --check        # 期望：✅ PASS

# ③ 58 条能力口径真源
python3 scripts/docs_truth.py --verify --check        # 期望：claims=58 fail=0 stale=0

# ④ 能力汇总表与真源一致
python3 scripts/gen_capability_summary.py --check     # 期望：✅ PASS

# ⑤ 文档治理三件
python3 scripts/docs_links.py --check                 # 断链 0
python3 scripts/docs_index.py --check                 # 问题数 0
python3 scripts/check_doc_class.py --check
```

**W3 新面定向测试（**别跑全量 workspace**）**

```bash
cargo test -p apeireth-gateway --test object_grant_activation_face --locked -j 2   # 8
cargo test -p apeireth-runtime-assembly --test object_grant --test object_grant_production --locked -j 2  # 13 + 6
cargo test -p apeireth-core --test routing_policy --test routing_policy_signal --locked -j 2              # 12 + 13
cargo test -p apeireth-memory --test object_grant_visibility --locked -j 2                                # 21
cargo test -p apeireth-tools-canonical --test process_tree_snapshot --test process_tree_tool \
  --test process_tree_windows --test architecture_invariants --locked -j 2                                # 13 + 12 + 1(非Windows占位) + 6
cargo test -p apeireth-runtime --test quota_depth --test quota_depth_wiring --locked -j 2                 # 13 + 14
```

> ⚠️ **测试数字对外引用**一律以 [test-baseline.md](../03-reference/test-baseline.md) 为准（本页数字是定向实测，不是总量）。

## 5. Windows 上怎么起（本线相关的部分）

**工具链（仓内已固定，别改）**：`rust-toolchain.toml` ⇒ `channel = "1.97.1"`（可重现构建；CI 全部 `dtolnay/rust-toolchain@1.97.1`）。

**必需前置**
1. **Rust 1.97.1** + **VS Build Tools（MSVC）**（`windows-sys` / MSVC 链接需要）；
2. **WebView2 Runtime**（Tauri/WebView2 壳需要）；
3. **Node + pnpm**（仅前端仓 `ui-desktop` 需要；根 workspace 不需要）；
4. `git config --global core.autocrlf false`（本仓跨平台，**别让 CRLF 污染** —— 大量 `.rs`/`.md` 会被整体改写）。

**拉代码（两份，别混）**

```bash
# ① 引擎（公开仓）—— 本线的工作面
git clone https://github.com/Apeireth/Apeireth.git
cd Apeireth && git checkout w3-capability-line-2026-11-09

# ② 前端（私有仓，需要 token）—— 消费引擎
git clone https://github.com/Apeireth/ui-desktop.git
#   它用 Cargo `git = ".../Apeireth.git", rev = "3174666b..."` 依赖引擎
#   ⇒ 该 SHA 在公开仓 origin/ui-engine 上（已核实可达）；改了引擎就要同步它的 apeireth-ui-engine-rev
```

**Windows 专属验证（本线已布好线，第一次绿灯就能升级口径）**

```powershell
# 进程树真机（W3 新增 CI：windows-latest + 9 条断言，无 --no-run / continue-on-error / || true）
cargo test -p apeireth-tools-canonical --test process_tree_windows --locked -j 2
```

> 首次绿后：把 `claim` 里 `tool.process_tree` 按**维度分列**升格（**Windows 后端**可升；**macOS 仍显式未实现**
> ⇒ 不得整族一句 `HARDWARE VALIDATED`），并回填**运行链接 + 日期**。

**交叉编译（在 Linux/macOS 上也能先验类型，W3 已用过）**

```bash
cargo check -p apeireth-tools-canonical --target x86_64-pc-windows-gnu --all-targets --locked -j 2
```

## 6. 诚实边界（**没做到的，逐条** —— 接手不要当成已完成）

| # | 项 | 现状 |
|---|---|---|
| 1 | 全仓 `HARDWARE VALIDATED` | **0 条**（这是口径阶梯第四级，不是"接近"） |
| 2 | C-B1 Windows 后端 | **已布线、未实测**；Windows 无进程启动时刻 ⇒ 取消台账**只能按 pid 记** ⇒ 命中即回放（漏杀上限靠 **300s 过期**兜底） |
| 3 | C-D1 审批可用性 | **无 pending 读面** ⇒ 审批人目前**凭 `grant_id` 盲批**（看不到对象分区 + 目标主体）⇒ 下一批补读端口 |
| 4 | C-D1 账本防伪 | **未做行级签名（HMAC）** ⇒ 能写账本文件者成对伪造可生效（已在模块头逐字写明） |
| 5 | C-H1 隐私闸 | 闸**已 live**，但**声明入口只有** CLI `--privacy` 与本地 PATCH（API）⇒ **没有面板 UI** ⇒ 生产里实际几乎无人声明 |
| 6 | C-H2 深度维 | 可达深度 **结构上 ≤ 1** ⇒ **不是有效防护**（要在 Windows 侧改存量运行时调用树读数，需 owner 拍板） |
| 7 | C-B4 网页会话 | 收窄版改名 **`tool.web_flow`**；**站点侧会话不连续**（受控出网不暴露响应头 ⇒ 无 cookie jar）；引外部浏览器引擎**待 owner 重拍** |
| 8 | 依赖漏洞 | 公开仓默认分支 **8 个 Dependabot 告警（1 critical）** ⇒ 与本线"上线前配套"冲突，建议单开一批 |

## 7. 下一批任务队列（W4 · 按优先级）

1. **C-B1 真机绿灯 → 按维度升格**（唯一能把这仓从 "0 条 HARDWARE VALIDATED" 往上抬的项；**Windows 上做最省往返**）；
2. **C-B1 残余根治**：用 **进程创建时间**替代 pid 判等（`windows-sys` 有该 API）⇒ 消掉"最多漏杀 5 分钟"；
3. **C-D1 审批前置读面**（pending 读端口：对象分区 + 目标主体）⇒ 与面板 UI 同批；
4. **C-H1 面板声明入口**（下拉选 privacy level）⇒ 让闸真被用起来；
5. **C-B4' `tool.web_flow` 构建**（收窄版：`crates/capabilities/tools/src/web_flow/`，**不新 crate**，env **+2**，额度已在 ADR-0025 内）；
6. **C-A1 `tool.external_gui`**（需**新 crate** `capabilities/automation/` ⇒ 先补 ADR 额度）；
7. **C-D2 `session_observability`**（会话期可观测性面，纯增量）。

## 8. 五个坑（都是本线踩过并记录的，别再踩）

1. **探针别名假阴性**：子代理用转义管道（`rg 'A\|B'`）⇒ 报"0 命中"，实际大量命中 ⇒ **一律用真正则**；
2. **子串假阳性**：`calendar` 命中 25 条实际是 `canonical` 的子串 ⇒ 结论前**看命中上下文**；
3. **生成物在脏树里跑**（汇总表/索引）⇒ 会把**并行线未提交的行号**烘进去 ⇒ 一律在**干净 worktree** 生成；
4. **`--no-verify` 禁手**：本线曾误用一次 ⇒ 已 amend 补全并在正文披露 ⇒ **禁止再犯**；
5. **他线文件不碰**：`crates-ui/**`、`repo_map_*`、`doc_ops/sheet_ops`、`mcp_bridge/transport.rs`、`Cargo.toml/lock`
   ⇒ 本线只改自己的写域；**提交也不代他线提交**（曾发生"我方暂存被他线提交一并扫入"⇒ 已如实登记在提交正文）。

## 9. 五分钟自检清单（Windows 上接手时照着敲）

```powershell
git -C <repo> status --porcelain            # 工作树干净？（他线在飞的话会有 crates-ui 的未跟踪文件，属正常）
git -C <repo> log -1 --oneline              # 当前在哪一笔
git -C <repo> fetch origin; git -C <repo> log -1 --oneline origin/ui-engine
python3 scripts/arch_guard.py --strict      # VIOLATIONS: 0
python3 scripts/check_creation_line.py --check
python3 scripts/docs_truth.py --verify --check
cargo test -p apeireth-tools-canonical --test process_tree_windows --locked -j 2
```

全绿 ⇒ 环境就绪，可从 §7 第 1 项开工。

## 10. 交接时的仓状态（2026-11-09 18:10 实测）

| 项 | 值 |
|---|---|
| 本线分支 | `w3-capability-line-2026-11-09` @ `59844fc7`（**已推公开仓**，新分支，非 force） |
| 工作树 | 干净（仅 `crates-ui/apeireth-ui-demo/harness/**` 有他线未跟踪文件，**不是本线**） |
| 棘轮 | `VIOLATIONS: 0` · `crates=31 edges=118 env=165 max_src_loc=6250 exceptions=0` |
| 台账 | `claims=58 fail=0 stale=0` |
| 新增外部依赖 | **0**（Cargo.toml / Cargo.lock 本波零变更） |
| 待 owner 裁决 | ① `web_search`/`calendar` 是否入人工审批地板 ② C-D1 账本 HMAC ③ C-B4 引外部引擎 ④ C-H2 是否改存量运行时语义 ⑤ 是否把本线并入 `ui-engine`/`main` |
