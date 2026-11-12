# 工作线登记表（工作线 ↔ 设计稿 ↔ **进度页** ↔ 守卫）

[Document-Meta]
Document: docs/status/workline-registry.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-11
Author: lead

> **用途**：`AGENTS.md` §0 要求「开工前先认领工作线」⇒ 本表就是那张**认领清单**。
> ⚠️ **卡点是哪一条**：一条工作线若**只登记了设计稿、没登记进度页**
> ⇒ ⇒ 下一个人（或压缩后的你）**查不到"做到哪了"** ⇒ 一定会重演 2026-10-28 的事故
> （误判已有模块不存在、误判已接生产的能力未接线）。
>
> **新工作线必须加一行**（`AGENTS.md` §0.3）。加行即视为"可接手"。

## 一、登记表

| 工作线 | 设计稿（目标形态） | **进度页（做到哪了）** | 守卫（机器判对错） |
|---|---|---|---|
| **Mesh RC2 PR 收口**（2026-11-11） | [mesh-architecture.md](../design/mesh-architecture.md) · [ADR-0024](../adr/0024-mesh-line-contract-registration.md) | [mesh-rc2-closeout-2026-11-11.md](mesh-rc2-closeout-2026-11-11.md)（缺口、验证与待办） | `cargo test -p apeireth-runtime --test transparent_tool_routing --locked -j 2` · `cargo test -p apeireth-mesh --test mesh_semantics --locked -j 2` · `python scripts/harness.py verify` |
| **合并裁决工具链收口**（2026-11-11，PR #121） | [merge-gate-toolchain-closeout-2026-11-11.md](merge-gate-toolchain-closeout-2026-11-11.md) §目标与范围 | 同文件 §进度与验收（全部适用 CI 通过，PR #121 已合并） | `python scripts/check_toolchain_pin.py` · `python scripts/test_provision_ci_toolchain.py` · `python scripts/check_yaml_duplicate_keys.py` · `python scripts/harness.py verify` |
| **官网与社区入口收口**（2026-11-11） | [community-ops.md](../development/community-ops.md) §9（目标与边界） | 同文件 §9（核验与待办） | `python scripts/verify_changed.py` · `python scripts/docs_links.py --check` · `python scripts/docs_index.py --check` · Issue 表单 YAML 结构检查 |
| **记忆与知识纵深**（D0–D5 / B0–B4） | [memory-knowledge-depth-system-spec.md](../01-architecture/memory-knowledge-depth-system-spec.md) | [spec-batch-status.md](spec-batch-status.md) | `python3 scripts/d4_ledger.py --check`（B0-c 候选池）· `python3 scripts/check_quality_metrics.py`（§6.1 十八项）· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| **护栏接线 / 越权拦截** | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md)（审计即设计依据 + 缺口清单） | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md) §4（缺口清单，逐条标「已修/未修」） | `python3 scripts/check_guard_wiring.py`（台账↔代码互校） |
| **记忆质量路线**（已证/已否的路） | [memory-quality-route-map-2026-10-27.md](memory-quality-route-map-2026-10-27.md) | 同文件（§〇 2026-10-28 更新块） | `python3 scripts/check_golden_baseline.py`（准确度不许降 + 送达率观察项） |
| **状态声明（29 条能力口径）** | [claims.yaml](claims.yaml) | `claims.yaml` 的 `verified_at` / `docs_truth --verify` 输出 | `python3 scripts/docs_truth.py --verify --block-stale` |
| **债务批次 9：Windows 零债 + 摄像头敏感读「放行只认宿主决定账」**（2026-11-10） | [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §摄像头敏感读（目标形态：**放行只认宿主决定账**；模型给的授权旗只是*请求*，声明面无授权人字段）<br>+ [security-audit-verification-low-2026-10-31.md](security-audit-verification-low-2026-10-31.md)（审计＝缺陷依据，历史件不改写） | [debt-round9-2026-11-10.md](debt-round9-2026-11-10.md)（逐条带实测数字 + **还差什么** + 未实测项） | `cargo test -p apeireth-tools-canonical --lib` · `cargo test -p apeireth-tools-canonical --test device_sense`<br>· `cargo clippy -p apeireth-tools-canonical --all-targets -- -D warnings`<br>· `python scripts/check_crate_budget.py --check`<br>· `pwsh -NoProfile -File scripts/fmt-check.ps1`（Windows 逐 crate） |
| ⭐ **后端 L4 零债收敛**（2026-11-06 起）<br>（L2 声明自洽 → L3 骨架完整 → L4 零债） | [l3-skeleton-claims-2026-11-06.md](../04-internal/01-handoffs/l3-skeleton-claims-2026-11-06.md)（7 条骨架声明审计）<br>+ [unwired-ledger.md](unwired-ledger.md) §一（待接线 W1–W34）· §五（分类）· §六（L3 骨架缺口） | 同上两处（**逐条带文件:行号实证**）<br>+ [claims.yaml](claims.yaml) 的 `level` 字段 | `python3 scripts/check_memory_reachability.py --check`（**未解释缺口** = 行动目标 · 上限只减不增）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（L2 声明自洽）<br>· `python3 scripts/audit_unwired.py --check`（L4 孤儿棘轮）<br>· `python3 scripts/harness.py verify`（总闸） |
| **Mesh 跨节点栈（RC2）** | [mesh-architecture.md](../design/mesh-architecture.md)（mesh 权威现状）<br>+ [mesh-nat-e2e-report.md](../design/mesh-nat-e2e-report.md)（NAT E2E 证据记录）<br>+ [mesh-phase-0.md](../design/mesh-phase-0.md)（Phase 0 设计稿）<br>+ [mesh-phase-2c1.md](../design/mesh-phase-2c1.md)（Phase 2C1 设计稿） | [mesh-rc2-status.md](../design/mesh-rc2-status.md)（RC2 冻结时点快照，2026-10-11）<br>**还差什么**：① 真实 NAT 下的穿透未实测（打洞只在 Windows loopback 验过；Linux namespace harness 没跑过）—— 需要两台机器；② 直连 → 打洞 → 中继的自动编排未实现（当前无自动回退）；③ 生产网关发现未接线（`connect_punched` 的首个调用方仍是测试夹具）；④ 远端准入不经本地 scope 校验、远端审批路径失败关闭（本阶段的有意取舍）<br>**2026-11-12 当前复核**：④中的远端 scope 缺口已在本次修复，远端审批恢复仍为后续阶段；①②③的当前边界见[收口页](mesh-rc2-closeout-2026-11-11.md)。 | `cargo test -p apeireth-mesh -p apeireth-mesh-protocol -p apeireth-mesh-quic -p apeireth-mesh-rendezvous -j 2`（mesh 栈回归）<br>· `cargo test -p apeireth-runtime --test transparent_tool_routing --test transparent_provider_routing -j 2`（远端工具 / provider 路由）<br>· `python3 scripts/check_dead_code.py`（豁免只减不增）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| ⭐ **自研 UI 引擎**（`crates-ui/`，L1 引擎 + 演示宿主 + 伴侣桌面）<br>（2026-11-10 起登记：此前该线只在交接页里，未登记 ⇒ 接手者查不到「做到哪了」） | [ui-engine-architecture.md](../design/ui-engine-architecture.md)（引擎分层与能力边界） | [ui-engine-windows-handoff-2026-11-09.md](ui-engine-windows-handoff-2026-11-09.md)（Windows 交接页 = 逐条实测口径）<br>+ `crates-ui/apeireth-ui-demo/harness/`（演示宿主与探针）<br>**还差什么**：① 合并 main 后 crates-ui 与主仓的依赖边界复核（`crates-ui/Cargo.toml` 独立 workspace）② 真实 Windows WebView2 宿主验收（Edge 证据 ≠ WebView2 证据）③ macOS / Linux WebKitGTK 腿未跑 | `cargo check --workspace --all-targets --locked -j 2`（crates-ui，wasm32 侧另跑 `cargo check --target wasm32-unknown-unknown`）<br>· `cargo clippy --workspace --all-targets --locked -- -D warnings`（crates-ui）<br>· `python3 scripts/arch_guard.py --strict`（尺寸与依赖边棘轮）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |

## 二、登记规则（四条）

1. **进度页必填**：只写设计稿不写进度页 ⇒ **不算登记**（接手者查不到"做到哪"）；
2. **进度页必须写「还差什么」**：只有"已完成/未完成"两态不够 ⇒ 要写**差的那一项**；
   验收标准里的每一条，进度页要能逐条回答 ⇒ 否则无法判定"算不算完"；
3. **每条可判定的断言都要有守卫**：凡进度页里写了「**PRODUCTION WIRED** / 已落地 / 数字 = N」
   ⇒ 必须有一条**退出码可判**的命令能验它 ⇒ 否则那句话等同散文（见下节）；
4. **守卫失败时改代码或改文档，不许改基线**（棘轮"变好"时的缩表除外）。

## 三、为什么这一列「守卫」是必须的（今天实测的证据）

同一仓库里，**有守卫的文档从没"悄悄漂"过**：

| 文档 | 守卫 | 结果 |
|---|---|---|
| `claims.yaml`（29 条） | `docs_truth --verify --block-stale` | 今天**两次** stale=7 ⇒ **当场被抓、当场修** |
| §6.1 十八项指标 | `check_quality_metrics.py` | 一直 18/18（台账与代码不一致即失败） |
| 架构 / 未接线 / 死代码基线 | `arch_guard` / `audit_unwired` / `check_dead_code` | 一直一致 |
| **设计稿 §8 批次计划**（原状态） | **无** | **漂了整整一天没人知道** |

⇒ ⇒ **结论**：差别不在"会不会漂"，而在"**漂了会不会有人喊**"。
⇒ ⇒ 没有守卫的文档 = 散文；**散文不能当进度用**。

## 四、守卫是什么（给新接手的人）

**文档守卫 = 一个在 CI / 门禁里跑的脚本，把「文档里写的断言」和「另一半真源」比对，
不一致就非 0 退出。**

三类形态（本仓都有实例）：

| 形态 | 判什么 | 本仓实例 |
|---|---|---|
| **一致性** | 文档说的 == 代码/台账里的 | `docs_truth.py --verify`（源码动过而声明没更新 ⇒ STALE）· `check_quality_metrics.py`（台账↔代码互校）· `check_guard_wiring.py` |
| **完整性** | 该有的都得有 | `docs_links.py`（断链）· `docs_index.py`（索引覆盖）· `check_mod_targets.py`（`mod` 声明必须有文件）· `check_doc_paths.py` |
| **棘轮** | 只能变好，不能变差 | `arch_guard.py --strict`（尺寸/依赖边）· `audit_unwired.py --check`（未接线 API 143→142 只许减）· `check_dead_code.py` |

**四条硬要求**：

1. **机器可判定**：断言能算成 true/false（"文档写得好"不可判定 ⇒ 不能当守卫）；
2. **失败要响**：非 0 退出 + 说**哪一条**不一致 + 给**修法**；
3. **有真源**：守卫要有可比对的另一半（代码 / git 历史 / 台账 JSON），不能自说自话；
4. **不许自我豁免**：发现不一致 ⇒ 改代码或改文档，**不是**改基线。

**关键认知**：进度页里写「❌ 没做」反而是**守卫友好**的形态——因为它**可判定**。
守卫真正要防的是：**悄悄变成"做了"（或悄悄退回"没做"）而页面没更新。**
