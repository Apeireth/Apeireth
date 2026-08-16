# C3 v2 alpha 遗留盘点 + 上轮自检发现吸收 — 盘点报告

- **任务ID**: 06da84cc-848a-4087-b42f-2679d6c6c4d0
- **角色**: technical_writer2 | **性质**: 只读核实 + 台账更新（0 代码改动）
- **日期**: 2026-08-16/17 | **master HEAD**: `2afb7eda`
- **方法**: 对照 `docs/RELEASE-NOTES-v2.0.0-alpha.md` §4.1 的 22 任务矩阵，逐项 grep/读码/git 历史取证，**不信任旧标注**；上轮自检 21 份角色报告（2026-08-16 22:55–23:20）全部读取并提取可行动发现。
- **0 装 PASS 声明**: 无法在本次预算内核实的项（全量测试实跑、ASI V0.5 测量）如实标注 ⚪ 不可核实，绝不标 ✅。

---

## 一、22 项任务核实表（证据列 = 文件/命令）

**统计**: 旧标注 10 DONE + 5 PARTIAL + 6 BLOCKED + 1 TODO → 新实况 **12 项 ✅ 达成/已解决**（含 1 归档、1 演进、2 超额替代方案）+ **7 项 ❌ 产物丢失**（旧标 DONE 但交付物从未入 git 历史）+ **1 项 ⚪ 不可核实**。

### A. 旧标 DONE（10 项）

| # | 任务 | 旧标注 | 当前真实状态 | 证据（文件/命令） |
|---|---|---|---|---|
| T1 | V2 哲学守门 Addendum (09-PHILOSOPHY-GUARD-ADDENDUM.md) | ✅ DONE | ❌ **产物从未入 git 历史**（不可恢复）。部分内容以 `stage2-decisions-philosophy-guard.md` 等形式存在于 stage2 | `git log --follow -- docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md` = 空；`git ls-tree -r abf12243 --name-only` 无此文件；`ls docs/v2-strategy/` 仅剩 README |
| T3 | apeireth-mcp 真实现 (SSE+HTTP-streamable+conformance) | ✅ DONE | ✅ **仍成立**：完整 crate（protocol/initialize/prompts/resources/multimodal）+ 4 examples + conformance/multi_transport 测试；上轮 MCP1 实测 18/18 通过 | `crates/apeireth-mcp/src/`、`crates/apeireth-mcp/tests/{conformance,multi_transport}.rs`；`reports/19809d9e-…-mcp_integration_expert-report.md` §① |
| T7 | apeireth-formal (Kani harness) | ✅ DONE | ✅→📦 **R165 有意归档**（deadcode 审计）：移入 `crates/_archived/apeireth-formal`，不再是 workspace 成员；验收报告 `a7c5b65b` 仍在 | `git log 79a84a29`（2026-08-13 "R165 architecture audit + deadcode archive (protocol-bridge + formal)"）；`reports/a7c5b65b-code-reviewer-apeireth-formal-skeleton.md` |
| T12 | SWE-bench smoke framework | ✅ DONE | ✅ **仍成立且扩充**：swe_bench.rs + swe_bench_smoke example + 7 tests，另长出 latency/self_disable/agent bench | `crates/apeireth-bench/src/swe_bench.rs`（`grep -c "#[test]"` = 7）；`crates/apeireth-bench/examples/swe_bench_smoke.rs` |
| T13 | 5 new crate 设计评审 | ✅ DONE | ❌ **评审报告 `reports/d67aedf7-v2-5-new-crates-design-review.md` 丢失**（从未入 git）；结论仅存于 release-notes 转述 | `ls reports/d67aedf7*` = No such file；`git ls-tree -r abf12243` 无 |
| T14 | addendum 终审 | ✅ DONE | ❌ **报告 `reports/v2-addendum-final-review.md` 丢失**（同上） | `ls` 无；`git ls-tree -r abf12243` 无 |
| T15 | 集成协调 | ✅ DONE | ❌ **快照 `reports/v2-integration-status-live.md` 丢失**；协调机制已演进为现团队流水线（integration worktree 现已与 master 0/0 同步） | `ls` 无；`git rev-list --left-right --count master...team/e8de47ae-…/integration` = `0 0` |
| T17 | deploy | ✅ DONE | ✅→**演进**：原验收报告 `V2-deploy-*` 丢失，但部署功能今已真落地：companion 部署文档 + release 脚本 16 个 + 2 个 release workflow | `docs/companion-deploy.md`（2026-08-16）；`ls scripts/release/`（16 文件）；`.github/workflows/{release,release-1.0.0}.yml` |
| R3 | baseline 文档 | ✅ DONE | ❌ **`07-V2-BASELINE-2026-08.md` 丢失**；现 `docs/stage2/07-VCP-GAP-UPGRADE-PLAN.md` 是另一份文档（勿混淆）；baseline 职能已演进为 QA 体系 | `ls docs/stage2/`；`docs/RELEASE-NOTES-v2.0.0-alpha.md:9` 的 Source-Trace 仍引用已消失文件 |
| R5 | final summary | ✅ DONE | ❌ **`reports/v2-final-summary-2026-08-05.md` 丢失**（22 任务矩阵唯一原始出处消失，本次以 release-notes §4.1 为基准重建核实） | `ls` 无；`git ls-tree -r abf12243` 无 |

### B. 旧标 PARTIAL（5 项）

| # | 任务 | 旧标注 | 当前真实状态 | 证据（文件/命令） |
|---|---|---|---|---|
| T2 | cleanup 4 小 crate | 🟡 3/4 | ✅ **4/4 达成 + 后续演进**：①philosophy 永久删除；②test 删除后经整合复活为**真测试辅助 crate**（retry/budget/suite 聚合 + property_tests + kani_proofs，活跃成员）；③desktop→tauri-stub 改名完成，R145 再冻结；④bench 扩充至 112KB/2552 行（目标 ≥20KB 远超） | `ls crates/` 无 apeireth-philosophy；`Cargo.toml:25` test 成员；`Cargo.toml:58` tauri-stub frozen 注释；`du -sh crates/apeireth-bench/src/` = 112K；定义见 `docs/stage2/05-EXECUTION-NOW.md` §1.1-1.4 |
| T5 | vector workspace 注册 (MUST FIX) | 🟡 缺注册 | ✅ **已解决**：workspace 已注册；crate 含 sqlite_backend/qdrant_compat/distance 真实现 + semantic_smoke example + store 测试 | `Cargo.toml:90`（`"crates/apeireth-vector"`）；`crates/apeireth-vector/src/` |
| T8 | TUI 6 类 JSON 端点 + HTTP 消费 | 🟡 PARTIAL | ✅ **已达成**：api 侧 v2_endpoints 端点组（health/tools/audit/memory/organs/asi…）经 nest_service 挂 /v1；TUI 5 页面齐（bridge/dialogue/growth/history/settings）+ http_llm HTTP 瘦客户端；遗留 backend.rs 少量 TODO 注释（不阻塞） | `crates/apeireth-api/src/v2_endpoints.rs:1182-1200`（`.route` 清单）；`crates/apeireth-api/src/server.rs:150-151`（nest_service）；`crates/apeireth-tui/src/pages/{bridge,dialogue,growth,history,settings}.rs`；`backend.rs:29` `use crate::http_llm` |
| T9 | 5 crate 独立 CI workflow | 🟡 缺 5 yml | ✅ **超额替代解决**：未做 per-crate yml，但 `rust.yml` 全 workspace 覆盖（`cargo build --workspace --tests --locked` + `nextest run --workspace --profile ci --locked`）共 18 个 workflow（含 kani/miri/bench/audit/deny/coverage/protocol-e2e） | `.github/workflows/rust.yml:57-62`；`ls .github/workflows/` = 18 文件 |
| R4 | 协调续 | 🟡 PARTIAL | ❌ 原产物丢失（同 T15）；机制演进为当前团队流水线 | 同 T15 证据 |

### C. 旧标 BLOCKED（6 项）

| # | 任务 | 旧标注 | 当前真实状态 | 证据（文件/命令） |
|---|---|---|---|---|
| T4 | apeireth-graph 空壳 | 🔴 仅 Dockerfile | ✅ **空壳消除**：8 模块真 src（channel/checkpoint/cognition_graph/conditional/context_graph/executor/mcp_resource）+ 2 examples + 3 smoke tests + benches | `crates/apeireth-graph/src/`、`examples/{linear_3_nodes,subgraph_channel_demo}.rs`、`tests/{smoke,conditional_smoke,subgraph_channel_smoke}.rs` |
| T6 | apeireth-sdk 空壳 | 🔴 仅 Dockerfile | ✅ **空壳消除**：多语言 FFI 真实现（C/Node/Lark/LiveKit）+ C 头文件 + build.rs + examples + tests | `crates/apeireth-sdk/src/{abi,c,node,lark,livekit}.rs`、`apeireth_sdk.h`、`tests/{smoke,multilang_ffi,test_sdk_client}.rs` |
| T10 | baseline 验证 (cargo test ≥2265 + 5 smoke + ASI V0.5) | 🔴 BLOCKED | ⚪ **不可核实（如实标注）**：本次预算未实跑全量测试、未测 ASI V0.5。静态参考：全库 `#[test]` ≈ 7646（QA1 grep 口径，含 _archived/_frozen）；apeireth-companion 单 crate 228 tests 已实测可枚举 | `reports/5c888b1c-…-qa_engineer-report.md` §一（grep 命令在报告内）；`reports/c7e494b3-…-database_engineer2-report.md` §3（224 lib + 3 integration + 1 doc-test）；全量实跑 = 未做 |
| T11 | Self-Disable 20 攻击场景 | 🔴 BLOCKED | ✅ **真实现（smoke 级）**：20 case 内联（4/category × 5 category）+ 5 守门 fn + runner/summary + 19 单测 + integration + smoke example；验收门槛 ≥5/20 有代码判定 | `crates/apeireth-bench/src/self_disable_bench.rs`（`:383-626` default_cases/guards；`grep -c "#[test]"` = 19）；`tests/self_disable_integration.rs`；`examples/self_disable_smoke.rs` |
| T16 | TUI E2E + web 移交 | 🔴 BLOCKED | ✅ **已达成**：apeireth-tui-e2e crate（harness/nav_e2e/organ_e2e/render/backend + in-process 测试）；apeireth-web 真面板（api/council/memory/dashboard 等） | `crates/apeireth-tui-e2e/tests/test_tui_e2e_in_process.rs:49-95`（6+ tests）；`crates/apeireth-web/src/api_endpoints.rs:675`（/dashboard 路由） |
| R1 | MUST FIX (vector 注册) | 🔴 | ✅ **已解决**（同 T5） | `Cargo.toml:90` |

### D. 旧标 TODO（1 项）

| # | 任务 | 旧标注 | 当前真实状态 | 证据（文件/命令） |
|---|---|---|---|---|
| R2 | T7 跟进 | ⚪ TODO | ✅ **决策闭环**：R165 架构审计将 formal 判为 deadcode 并归档，即跟进结论；后续如需形式化验证由 kani/miri CI workflow 承担 | `git log 79a84a29`；`.github/workflows/{kani,miri}.yml` 存在 |

### E. 旧声明复核（release-notes §4.1 附注）

| 旧声明 | 核实结果 |
|---|---|
| "真实 git merge = 0"（10 任务软标记） | ❌ **已过时**：当前 master（`2afb7eda`）已含全部内容；integration worktree 与 master `0 0` 完全同步（DO1 自检时落后 1053，此后已同步） |
| "主 worktree 25+ 文件 uncommitted" | ❌ **已过时**：`git status` 干净（仅本次任务新增产出与上轮 11 份未提交报告） |
| `docs/V2-INDEX.md`（22 产物入口） | ❌ **文件不存在**，release-notes 三处引用悬空（`:9/:21/:114`） |

---

## 二、v2 alpha 丢失产物清单（诚实记录，不可恢复）

以下产物在 release-notes 中标 DONE，但**从未进入 master/integration 分支 git 历史**（已对 `abf12243` 整合 #4 树与 integration 分支树双重核验）。推测原因：v2-alpha 工作发生在迁仓前的分支/worktree，未随整合 #4 进入主历史。

| 产物 | 引用位置 |
|---|---|
| docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md | release-notes §1.7 |
| reports/v2-final-summary-2026-08-05.md | release-notes §1.7/§5.1 |
| reports/v2-decision-brief-2026-08-05.md | release-notes 头部 |
| reports/v2-risk-register-2026-08-05.md | release-notes 头部 |
| reports/d67aedf7-v2-5-new-crates-design-review.md | release-notes §1.1 |
| reports/v2-addendum-final-review.md | release-notes §1.7 |
| reports/8f689476-mcp-integration-expert-acceptance.md | release-notes §1.1 |
| reports/V2-deploy-devops-engineer2-acceptance.md | release-notes §7 |
| reports/v2-integration-status-live.md | release-notes §7 |
| docs/V2-INDEX.md | release-notes §1.7 |
| docs/v2-strategy/07-V2-BASELINE-2026-08.md | release-notes §1.4 |

**处置建议**（已登记 backlog）：不重建（0 装 PASS 原则，不伪造历史产物），在 release-notes 对应位置加注"产物已失传"或由 Leader 决策是否重写。05/06 号文档幸存（已迁 `docs/stage2/`），但 mcp crate 内引用路径未更新（见三-MCP1）。

---

## 三、上轮自检（21 份报告）可行动发现 → backlog 吸收对照

上轮自检批次 = 2026-08-16 22:55–23:20 提交的 21 份角色自检报告。逐份读取后提取的可行动项已全部登记入 `docs/backlog.md`（编号 25–47：P1=25-29 / P2=30-37 / P3=38-45 / P0=46-47；46/47 后置因 23/24 已被 C2 压测自检并行占用；另 1 项已完成标记入"已完成项"表），来源对照：

| 来源报告 | 可行动发现 | backlog 编号 |
|---|---|---|
| QA2 `397a85ec` | cargo fmt --check 不通过：1588 文件中 1154 个（72.7%）不合规（stable 口径）；nightly 工具链本机损坏；cargo fmt 遇 Windows error 206 | #25 |
| AR1 `91bb7d42` | ①误产物 `ersXXXApeireth-rust`（11KB ANSI git log 转储，**git 已跟踪**，本次复核仍在）②`crates/apeireth-memory.db*` 泄漏进源码树（本次复核仍在）③12 个零内部消费者 lib crate 待确认 ④tool-fetch 自引用 dev-dep（本次复核：`crates/apeireth-tool-fetch/Cargo.toml:28` 仍在）⑤3 处 dev-dep 回环边界腐化 | #32/#33 |
| SEC2 `97a4bfce` | .gitignore 加固：追加 `*.pem *.key *.p12 *.pfx id_rsa*`；补 `_research_mem/` | #28 |
| MCP1 `19809d9e` | ①CODEOWNERS:49-51 悬空（mcp-ssh/winrm/relay-image 目录不存在，本次复核仍在）②mcp lib.rs/Cargo.toml 引用 `docs/v2-strategy/05` 已迁走（今在 `docs/stage2/05-EXECUTION-NOW.md`）③lib.rs 头部过时（称 SSE/resources/prompts 未做，实际已实现） | #30/#31 |
| QA1 `5c888b1c` | 根 `tests/` 12 个 .rs 不被任何活跃 crate 编译（死代码）；21 crate 无集成测试目录（记录性，不阻塞） | #37 |
| SEC1 `02cd644d` | deny.toml 过期 skip 清理（heck/async-channel 等） | #40 |
| DO2 `af2676fa` | ①**W1 阻塞级**：Dockerfile `COPY crates/apeireth-*/Cargo.toml ./crates/` 同名互覆盖，dummy 依赖缓存大概率失效/失败（本机无 docker 未实测）②W3：compose `POSTGRES_PASSWORD:-secret` 默认弱密码③W2：基础镜像硬编码与 toolchain 漂移④W4：rust-ci.yml deprecated 重复 workflow | #46/#47/#44 |
| DO1 `b7f49cfe` | ①integration worktree 落后 1053 → **本次核实已解决（0/0 同步）**②僵尸 worktree r11-recover 待 prune ③29 条 stash ④reports/*.log 入 gitignore | 已完成行 + #42 |
| TW1 `fba46921` | mkdocs `extra_css` 引用的 `docs/pages-source/assets/css/extra.css` 不存在（strict:true 会告警） | #38 |
| TW2 `f3f9fa0c` | RELEASE_NOTES v1.0.0 标题 ≠ workspace 1.2.0；CHANGELOG 顶部无 semver + R131-R178 未归条目；行号引用漂移（:246→:224）；11 个活动 crate 硬编码版本 | #26 |
| AR2 `b74fc48b` | README crate 计数 81→82（81 顶层 + 1 嵌套）；ROADMAP 头部"v1.0"表述与实际 1.2.0 双轨未标明；ROADMAP 进度止于 R127 未同步 R178 | #26/#29 |
| DB1 `e5a173c8` | 数据文件落 `crates/` 非常规位置建议迁标准目录；DB1 称"无 migration 框架"与 DB2 实测矛盾（`apeireth-memory/src/migrations.rs` 已有版本化迁移）→ 口径需统一：memory 已有，其余 CREATE TABLE IF NOT EXISTS 散点未接 | #45 |
| DB2 `c7e494b3` | PASS；companion_serve.exe 文件锁曾致 test --list 失败（CI 建议 `--lib --tests` 或先停 daemon）；db 残留同 AR1 | 并入 #32 |
| CR1 `abf185d2` | companion 6 条 clippy 警告（cast_lossless×3 + manual_let_else×3）；核对 CI fmt 是否真 nightly | #39 |
| CR2 `03cf86e9` | `assemble.rs:399` chrono `.unwrap()` DST/时钟回拨 panic 风险（一行修）；4 处 Mutex poison（记录即可）；并行写主 worktree 流程风险 | #34 |
| BE1 `5cb3d314` | rust-toolchain.toml 未 pin 具体版本（建议 pin 1.97.1 防漂移） | #41 |
| AO2 `b88db7ed` | `docs/security/cosign.pub` 缺失（签名脚本必失败的前置条件）；发布环境缺 cosign/gh/jq；scripts 卫生（2 桩文件 + 145 个 `_` 前缀一次性脚本） | #27 |
| MCP2 `380a2218` | round15-03 对嵌套侧 CHANGELOG +28 行 / ROADMAP +43/-5 行的更新未进根版本（blob 9aa1791c/0efb4322 可恢复）— 已通报 Leader，待决策 | #36 |
| FS1 `c7b06a25` | frontend/ 仅存 tauri-prototype 残留骨架（"砍前端"决策的遗留），可清理（需主人确认） | #43 |
| AO1 `6dbced86` / BE2 `ab65d8b4` | PASS，无可行动项 | — |

---

## 四、release-plan.md 更新摘要

基于本次核实结果更新了 `docs/release-plan.md`：
- §四进度对账表新增 **C3 v2 alpha 遗留盘点** 行（12 ✅ / 7 ❌ 产物失传 / 1 ⚪ 不可核实）；
- §五 checklist：文档项补充版本号口径统一证据要求；发布产物项补充 Dockerfile W1 验证、cargo fmt 全仓修复、cosign.pub 前置条件三个未勾选项（均有上轮自检证据支撑，不虚勾）。

---

## 五、自审（self-review）

| 验收项 | 达成 | 说明 |
|---|---|---|
| 22 项核实表有证据列（文件/命令） | ✅ | §一 A-D 四表每行均给出文件路径/行号或可复跑命令 |
| backlog 新登记项格式合规（P0-P3 分级） | ✅ | 编号 25–47 按既有表格格式追加（P0=46/47 后置，因 23/24 被 C2 压测自检并行占用），P0 2 项 / P1 5 项 / P2 8 项 / P3 8 项，均注明来源报告 |
| release-plan 对账表更新 | ✅ | §四新增 C3 行 + §五 checklist 3 项新证据化未勾项 |
| 0 装 PASS | ✅ | T10 全量测试未实跑 → 标 ⚪ 不可核实；丢失产物标 ❌ 不伪造；integration 同步、bench 体积等均实测取证 |
| 只改 docs/backlog.md + docs/release-plan.md + 本报告 | ✅ | 0 代码改动 |
| 不信任旧标注 | ✅ | 每项均重新取证：如 T4/T6 旧标"空壳 BLOCKED"实为真实现、T5 MUST FIX 已解决、"git merge=0" 已过时 |

**已知局限**：
1. T10 全量 `cargo test --workspace` 与 ASI V0.5 测量未实跑（小时级预算外），以静态计数 + 上轮 QA/DB 实测记录为参考；
2. v2 alpha 丢失产物的原因属合理推测（迁仓前分支未随整合 #4 入库），未穷举全部本地分支/worktree 历史（分支 200+，抽样核验了 abf12243 与 integration 分支树）；
3. QA2 的 fmt 数据为 stable 口径，CI nightly 口径可能更严，未复跑（本机 nightly 损坏为 QA2 已记录事实）。

**建议 Leader 决策项**：①release-notes 失传产物是否加注/重写；②版本号单一口径（v1.0.0 release tag vs workspace 1.2.0）；③round15-03 丢失 CHANGELOG/ROADMAP 内容是否从 blob 恢复。
