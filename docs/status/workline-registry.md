# 工作线登记表（工作线 ↔ 设计稿 ↔ **进度页** ↔ 守卫）

[Document-Meta]
Document: docs/status/workline-registry.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-19
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
| **受管桌面能力配置**（2026-11-21） | [网关合同](../gateway-api-contract.md#受管桌面能力配置) | [能力配置进度](backend-contracts-20261117.md#受管桌面能力配置) | `cargo test -p apeireth-gateway --test desktop_capabilities_face --locked` · `cargo test -p apeireth-cli --lib desktop_capabilities --locked` |
| **后端接线合同：工作区 / 联系人·人设·会话绑定 / C1 真流式**（2026-11-17 接前端交接草案） | [backend-contracts-20261117.md](../design/backend-contracts-20261117.md) §目标与范围、§批次与验收、§裁决 | [backend-contracts-20261117.md](backend-contracts-20261117.md) §一、批次状态与 §三、还差什么（**还差什么**：A1 已完成但只到领域层（提交 91888e27c，IMPLEMENTED）；A2 已完成（提交 3854c01a8，IMPLEMENTED：HTTP 面 + 409/403 分类码 + 审计 + 能力入册 + 组合根真实注入）——**A2 本身未持久化（A3 已补持久化）**、多根/临时根/预设根未装配、失效联动（workspace→session/task/审批/流）未做、多用户归属未建模；A3 已完成（提交 e5cb9bfb6，IMPLEMENTED：本地文件系统 + `StoredDoc` 信封持久化、跨重启可恢复、坏记录 fail-closed、已吊销不可复活、落盘失败内存零变化）——多根解析、自动过期回收、失效联动仍未做；B1 已完成但只到领域层（提交 5bfb959d7，IMPLEMENTED：ContactId/ContactAgentId/PersonaText/Contact + 墓碑归档 + 文件持久化）——B1 本身无 HTTP 面；HTTP 面与 CLI 落盘由 B3a 落地；B2 已完成（提交 49213dc78，IMPLEMENTED 领域层：`canonical::agent_binding` + `Session.agent_binding` + 合同版 memory_scope 映射（none 在类型层不可表达为 Global）+ 归档 fail-closed 闸 + 21 条契约测试）——（当时**未 PRODUCTION WIRED**：`admit_turn` 无调用方；**回合入口接线已由 B3b-1 落地**，见下）、canonical MemoryScope 翻译（B3b-2）已由 c137f5a9d 接线（见下）；B3a 已完成（提交 43291e4ea，IMPLEMENTED HTTP 面：`GET/POST /v1/agents/contacts` + `GET/PATCH /v1/agents/contacts/{id}` + `POST /v1/agents/contacts/{id}/archive` + 13 键 DTO + expected_revision CAS + 归档终态 403 `contact_archived` + workspace_binding 服务端校验 + 4 个新错误码 + 能力入册 + CLI 真落盘）——（当时回合入口未接线；**已由 B3b-1 落地**）；**B3b-1 已完成**（提交 b7bcbacc0，四级口径 **PRODUCTION WIRED**：会话绑定面 `GET/POST /v1/sessions/{id}/agent-binding`（expected_revision CAS + 显式 null 解绑 + 字段缺失 400）+ 回合入口只读闸「先 admit 后 append，拒绝零写入」+ 2 个新错误码 `agent_binding_rejected`/`session_binding_conflict` + 审计事件 `gateway.session_binding_action` + registry 进程级单例）——**未做 live E2E** ⇒ 不到 HARDWARE VALIDATED；绑定变更不写 SessionEvent；多进程部署下 registry 仍会分叉；**B3b-2 已完成**（提交 c137f5a9d，PR #138 已合主干 95efc4325，四级口径 PRODUCTION WIRED；装配层翻译 + 绑定会话写回已接生产装配；恢复补写未翻译）；**B3b-3 已完成**（feat 7d18e4c32 + docs f60642cf7，四级口径 PRODUCTION WIRED；仅空会话种子槽；未做 live E2E）；C1 已完成（领域层 e397cc687 + 生产装配 C2-3 ccb62e678，PR #135/#139，四级口径 PRODUCTION WIRED；权威缓冲进程内，重启即失）；C2 已完成（C2-0 80edf9832 / C2-1 da81ac1be / C2-2 3bb73032f / C2-3 ccb62e678，PR #139 已合主干 `27e3872f0`；operation 级幂等、stop 幂等端点、审批/停止终帧与 C1 生产装配均已落地；幂等表与停止摘要进程内，重启即失）；尺寸棘轮已压回基线以内（提交 ea4acf270：4 个受约束文件做行为等价搬迁，`arch_guard --strict` ⇒ VIOLATIONS 0）；15 条受源码范围影响的声明已逐条真跑 verify_cmd 并抬 verified_at（提交 6b81ffd8b，claims=60 fail=0 stale=0）；已合并进 upstream/main（PR #132，合并提交 c526472bb）；D1 未决裁决（2026-11-19）见 §一·补：已裁决 6 条；成果版本对象已由 PR #142 建模；仍待前端/产品确认 2 条＝workspace_binding 语义、联系人 capabilities→Permission 映射粒度；`/bridge/*` 经核实属私有前端仓宿主本地面 ⇒ 后端不实现该前缀，改为后端原生流面由宿主映射） | `cargo test -p apeireth-runtime --locked -j 2`<br>· `cargo test -p apeireth-gateway --locked -j 2`<br>· `cargo test -p apeireth-protocol --locked -j 2`<br>· `python3 scripts/arch_guard.py --strict`<br>· `python3 scripts/docs_truth.py --verify --check` |
| **构建产物槽位纪律（磁盘漂移根治）**（2026-11-17） | [build-artifact-discipline-2026-11-17.md](../design/build-artifact-discipline-2026-11-17.md) §目标与范围、批次与验收、四层落点 | [build-artifact-discipline-2026-11-17.md](build-artifact-discipline-2026-11-17.md)（逐层状态 + 现场实测 + **还差什么** 七条） | `python scripts/target_slot.py --audit`（游离目录 ⇒ 非 0；本机无槽位根 ⇒ 跳过并退出 0）<br>· `python scripts/target_dir.py`（生效产物目录唯一真源解析）<br>· `python scripts/disk_guard.py --ensure 20000`（绝对可用量硬线，默认 10 GB）<br>· `pwsh -NoProfile -File scripts/disk-watch.ps1`（每日哨兵） |
| **后端语义、长程质量与系统工具补强**（2026-11-13） | [backend-hardening-completion.md](../design/backend-hardening-completion.md) §目标与范围、批次与验收 | 同文件 §进度与验收记录（四级口径：IMPLEMENTED；B1 语义已接线 26 项全绿；B2 评测契约与 B3 压力六阶段已实跑，真实模型已实测一轮 4/6 通过、2 例模型侧质量失败、36 次负控全拒；B4 Windows 真实 netstat 实测并修掉「清空环境致地址列为空」生产缺陷；附带修掉长历史候选挤掉关键事实的缺陷。未实测项：英文文法、Linux/macOS、dispatcher 端到端、厂商推理内容回传；未接线：独立保管与真实计价。本地已补跑 CI 等价命令：nextest 11069 个用例全通过、cargo-deny 四类 check 全 ok（advisories 用本机缓存快照，非当日刷新）、nextest 报告探针通过；Actions 的 OS matrix 与产物上报仍只有 CI 能给。上线口径（2026-11-14）：上线前必须 3 项＝CI 裁决交给 Actions、部署侧模型推理模式口径、暴露面体检开关口径；其余缺口归入长尾 L1–L6，见设计稿 §上线就绪口径与长尾归类。生产厂商私有字段开关已在 OpenAI 兼容工厂**构造期**接线（四级口径：IMPLEMENTED，默认不发 ⇒ 请求体逐字节不变，非法取值构造期报错）；架构契约 env 名额用尽（154/154，未改预算值），评测侧专属开关不占生产名额；未完成：CLI 两条装配路径吞构造错误、真实端点复跑一轮待做）。上线裁决（2026-11-14）：已推到 fork 并向上游 main 开 **PR #126**，Actions **43 项检查全绿**、`合并裁决（唯一）`（merge-gate）通过；CI 首轮两条红已处置＝供应链公告 `RUSTSEC-2026-0538` 按公告补丁升级 quinn-udp 0.5.15→0.5.16（不加豁免）、macOS 专属用例 `setpgid` EPERM 判为 runner 偶发（该文件不在本 PR diff 内，重跑即绿）。本地 `harness.py verify` VERDICT: PASS（fast 30 全 PASS、deep PASS 7 + DELEGATED 7）并已生成绑 34d793ab9 的 attest 证据包。未合并（合并由上游维护者按 PR 流程执行）。上线前必做 R2/R3 收尾（2026-11-14，本次临时 key 实测）：生产 LlmFactory → 真实端点拿到 200（vendor_field_keys = ["thinking"]、content 非空、usage 18/1 tokens，新增 `#[ignore]` live 用例默认不跑）；评测 live 路径封存链 verified、36/36 负控全拒、6 条探针全绿、4/6 通过（2 例模型侧质量失败）、同 custody 复跑被一次性使用规则拒绝、配置不一致时 verify fail-closed；真实计价已接通（cost.status = wired，price_source 写明官方页与档位）；产物 15 个文件扫描无密钥泄漏。R3 已拍板：暴露面体检首发保持 default OFF。剩余缺口只剩 CLI 两条装配路径吞构造错误。最终裁决（2026-11-14）：PR #126 的最终代码提交 c24bad42e 上 Actions 43 项 success / 0 failure / 2 skipped（合并裁决 merge-gate、Cargo deny、黄金基线、arch-guard、Rust OS 矩阵三平台全 success）；本地 harness verify VERDICT PASS + attest 绑 c24bad42e；20 条提交已推送、未合并。文档收口（2026-11-14）：自 ca2b7b70f 起共 21 条提交、全部已推送，Actions 在 4520069e0（最终代码提交之上的纯文档提交）上同样 43 项 success / 0 failure / 2 skipped，本地 harness verify VERDICT PASS 且 attest 绑 4520069e0；设计稿、claims.yaml（60 条）、STATUS.md、wiring.yaml、env-registry.toml（168 条）、contract.toml（env 名额注释）、三份基线脚本与 CHANGELOG 顶部 Unreleased 段已全部同步到最终状态；本分支 diff 的前端路径计数为 0（新前端的在途改动不在本 PR 内，也未被推送）；长尾 L1–L6 的口径与升级条件见设计稿 §上线就绪口径与长尾归类 | 设计 §验收入口；专项命令随实现登记，最终运行 `python scripts/harness.py verify` 及全部适用专业 CI |
| **生产装配回归稳定性**（2026-11-12） | [production-test-reliability.md](../design/production-test-reliability.md) §批次与验收 | 同文件 §进度与验收（可追溯重试及隔离边界） | `cargo test -p apeireth-cli --test parallel_dispatch_production --locked -j 2` · `cargo test -p apeireth-runtime-assembly --test module_message_pipeline --locked -j 2` · 三平台实际 JUnit |
| **CI 测试报告与重试证据**（2026-11-12） | [ci-test-evidence.md](../design/ci-test-evidence.md) §目标与批次 | 同文件 §进度与验收（历史取证缺口与实际报告） | `python scripts/test_nextest_reporting.py` · `python scripts/check_yaml_duplicate_keys.py` · 三平台 nextest JUnit artifact 检查 |
| **记忆自动纠错、持久恢复与回答评测**（2026-11-12 接续） | [memory-quality-hardening.md](../design/memory-quality-hardening.md) §接续批次 | [spec-batch-status.md](spec-batch-status.md) §本轮接线范围与待验边界；质量设计 §第五轮固定合成语料与当前源码本地验收：LIVE05固定12类×3次为36/36，独立审计/root复算通过，旧01–04失败保留；G源码本地全仓/doc tests及严格检查通过；本页记提交前验收，最终提交按[PR #125 的检查](https://github.com/Apeireth/Apeireth/pull/125/checks)及绑定SHA完整门禁证据裁决，无外部同条件比较 | `cargo test -p apeireth-runtime --test committed_evidence --test context_budget --test minimal_kernel --locked -j 2` · `cargo test -p apeireth-memory --lib context_compiler --locked -j 2` · `cargo test -p apeireth-memory --test memory_source_evidence --test memory_automatic_correction --test writeback_recovery --locked -j 2` · `cargo test -p apeireth-runtime-assembly --test memory_automatic_correction --test memory_writeback_recovery --test memory_autonomy_production --example memory_answer_eval --locked -j 2`；真实调用另需显式模型、临时凭据与预算 |
| **记忆质量与持久一致性**（2026-11-12） | [memory-quality-hardening.md](../design/memory-quality-hardening.md) §目标与批次 | 同文件 §进度与验收（含未测边界） | `cargo test -p apeireth-memory --tests --locked -j 2` · `cargo test -p apeireth-runtime-assembly --test memory_provider_e2e --locked -j 2` · `python scripts/check_quality_metrics.py` · `python scripts/check_golden_baseline.py` |
| **文本成果与不可变版本 C2**（2026-11-13） | [text-artifact-versions.md](../design/text-artifact-versions.md) | [text-artifact-versions-2026-11-13.md](text-artifact-versions-2026-11-13.md)（契约/内核/工具/HTTP；成果线 2026-11-18 已合主干 `e95bb5e34`（PR #142）；只读候选回合 PR #143 已合主干 `ad9d69ad0`；未发布；仍缺项见页） | `cargo test -p apeireth-runtime-assembly --locked --all-targets -j 2` · `cargo test -p apeireth-gateway --locked --all-targets -j 2` · `cargo test -p apeireth-gateway --test artifact_candidates_http --locked -j 2` · `python scripts/docs_truth.py --verify --check` |
| **Mesh RC2 PR 收口**（2026-11-11） | [mesh-architecture.md](../design/mesh-architecture.md) · [ADR-0024](../adr/0024-mesh-line-contract-registration.md) | [mesh-rc2-closeout-2026-11-11.md](mesh-rc2-closeout-2026-11-11.md)（缺口、验证与待办） | `cargo test -p apeireth-runtime --test transparent_tool_routing --locked -j 2` · `cargo test -p apeireth-mesh --test mesh_semantics --locked -j 2` · `python scripts/harness.py verify` |
| **合并裁决工具链收口**（2026-11-11，PR #121） | [merge-gate-toolchain-closeout-2026-11-11.md](merge-gate-toolchain-closeout-2026-11-11.md) §目标与范围 | 同文件 §进度与验收（全部适用 CI 通过，PR #121 已合并） | `python scripts/check_toolchain_pin.py` · `python scripts/test_provision_ci_toolchain.py` · `python scripts/check_yaml_duplicate_keys.py` · `python scripts/harness.py verify` |
| **官网与社区入口收口**（2026-11-11） | [community-ops.md](../development/community-ops.md) §9（目标与边界） | 同文件 §9（核验与待办） | `python scripts/verify_changed.py` · `python scripts/docs_links.py --check` · `python scripts/docs_index.py --check` · Issue 表单 YAML 结构检查 |
| **记忆与知识纵深**（D0–D5 / B0–B4） | [memory-knowledge-depth-system-spec.md](../01-architecture/memory-knowledge-depth-system-spec.md) | [spec-batch-status.md](spec-batch-status.md) | `python3 scripts/d4_ledger.py --check`（B0-c 候选池）· `python3 scripts/check_quality_metrics.py`（§6.1 十八项）· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| **护栏接线 / 越权拦截** | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md)（审计即设计依据 + 缺口清单） | [guard-wiring-audit-2026-10-27.md](guard-wiring-audit-2026-10-27.md) §4（缺口清单，逐条标「已修/未修」） | `python3 scripts/check_guard_wiring.py`（台账↔代码互校） |
| **记忆质量路线**（已证/已否的路） | [memory-quality-route-map-2026-10-27.md](memory-quality-route-map-2026-10-27.md) | 同文件（2026-11-12 接续范围与 §〇 历史更新块） | `python3 scripts/check_golden_baseline.py`（准确度不许降 + 送达率观察项）；本轮生命周期/回答验收见上方接续工作线 |
| **受管桌面模型连接、只读候选与会话只读恢复同包**（2026-11-19） | [desktop-managed-model-connection.md](../design/desktop-managed-model-connection.md) §目标与批次、§契约、§持久与受管装配、§四级口径 | [desktop-managed-model-connection-2026-11-19.md](desktop-managed-model-connection-2026-11-19.md)（代码检查点、门禁、隔离验收；**还差什么**：PR #148 已合入 main（merge `e1d85ee31`）；2026-11-19 补充批恢复两条会话只读 GET（`/turns`、`/turn-result`），模块 32 passed；PR #150 已合入 main（merge `81f87b253`），同包 EXE 隔离验收 109 PASS / 0 FAIL，PRODUCTION WIRED；未做真实提供方与桌面 C41 实机联调；main 合并后 CI 以仓库 run 为准） | `cargo test -p apeireth-cli --lib desktop_model_connection --locked -j 2` · `cargo test -p apeireth-cli --test desktop_model_connection --locked -j 2` · `cargo test -p apeireth-gateway --test artifact_candidates_http --locked -j 2` · `cargo test -p apeireth-gateway --lib turn_results --locked -j 2` · `python scripts/docs_truth.py --verify --block-stale` |
| **状态声明（62 条能力口径）** | [claims.yaml](claims.yaml) | `claims.yaml` 的 `verified_at` / `docs_truth --verify` 输出 | `python3 scripts/docs_truth.py --verify --block-stale` |
| **债务批次 9：Windows 零债 + 摄像头敏感读「放行只认宿主决定账」**（2026-11-10） | [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §摄像头敏感读（目标形态：**放行只认宿主决定账**；模型给的授权旗只是*请求*，声明面无授权人字段）<br>+ [security-audit-verification-low-2026-10-31.md](security-audit-verification-low-2026-10-31.md)（审计＝缺陷依据，历史件不改写） | [debt-round9-2026-11-10.md](debt-round9-2026-11-10.md)（逐条带实测数字 + **还差什么** + 未实测项） | `cargo test -p apeireth-tools-canonical --lib` · `cargo test -p apeireth-tools-canonical --test device_sense`<br>· `cargo clippy -p apeireth-tools-canonical --all-targets -- -D warnings`<br>· `python scripts/check_crate_budget.py --check`<br>· `pwsh -NoProfile -File scripts/fmt-check.ps1`（Windows 逐 crate） |
| ⭐ **后端 L4 零债收敛**（2026-11-06 起）<br>（L2 声明自洽 → L3 骨架完整 → L4 零债） | [l3-skeleton-claims-2026-11-06.md](../04-internal/01-handoffs/l3-skeleton-claims-2026-11-06.md)（7 条骨架声明审计）<br>+ [unwired-ledger.md](unwired-ledger.md) §一（待接线 W1–W34）· §五（分类）· §六（L3 骨架缺口） | 同上两处（**逐条带文件:行号实证**）<br>+ [claims.yaml](claims.yaml) 的 `level` 字段 | `python3 scripts/check_memory_reachability.py --check`（**未解释缺口** = 行动目标 · 上限只减不增）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（L2 声明自洽）<br>· `python3 scripts/audit_unwired.py --check`（L4 孤儿棘轮）<br>· `python3 scripts/harness.py verify`（总闸） |
| **Mesh 跨节点栈（RC2）** | [mesh-architecture.md](../design/mesh-architecture.md)（mesh 权威现状）<br>+ [mesh-nat-e2e-report.md](../design/mesh-nat-e2e-report.md)（NAT E2E 证据记录）<br>+ [mesh-phase-0.md](../design/mesh-phase-0.md)（Phase 0 设计稿）<br>+ [mesh-phase-2c1.md](../design/mesh-phase-2c1.md)（Phase 2C1 设计稿） | [mesh-rc2-status.md](../design/mesh-rc2-status.md)（RC2 冻结时点快照，2026-10-11）<br>**还差什么**：① 真实 NAT 下的穿透未实测（打洞只在 Windows loopback 验过；Linux namespace harness 没跑过）—— 需要两台机器；② 直连 → 打洞 → 中继的自动编排未实现（当前无自动回退）；③ 生产网关发现未接线（`connect_punched` 的首个调用方仍是测试夹具）；④ 远端准入不经本地 scope 校验、远端审批路径失败关闭（本阶段的有意取舍）<br>**2026-11-12 当前复核**：④中的远端 scope 缺口已在本次修复，远端审批恢复仍为后续阶段；①②③的当前边界见[收口页](mesh-rc2-closeout-2026-11-11.md)。 | `cargo test -p apeireth-mesh -p apeireth-mesh-protocol -p apeireth-mesh-quic -p apeireth-mesh-rendezvous -j 2`（mesh 栈回归）<br>· `cargo test -p apeireth-runtime --test transparent_tool_routing --test transparent_provider_routing -j 2`（远端工具 / provider 路由）<br>· `python3 scripts/check_dead_code.py`（豁免只减不增）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| ⭐ **自研 UI 引擎**（`crates-ui/`，L1 引擎 + 演示宿主 + 伴侣桌面）<br>（2026-11-10 起登记：此前该线只在交接页里，未登记 ⇒ 接手者查不到「做到哪了」） | [ui-engine-architecture.md](../design/ui-engine-architecture.md)（引擎分层与能力边界） | [ui-engine-windows-handoff-2026-11-09.md](ui-engine-windows-handoff-2026-11-09.md)（Windows 交接页 = 逐条实测口径）<br>+ `crates-ui/apeireth-ui-demo/harness/`（演示宿主与探针）<br>**还差什么**：① 合并 main 后 crates-ui 与主仓的依赖边界复核（`crates-ui/Cargo.toml` 独立 workspace）② 真实 Windows WebView2 宿主验收（Edge 证据 ≠ WebView2 证据）③ macOS / Linux WebKitGTK 腿未跑 | `cargo check --workspace --all-targets --locked -j 2`（crates-ui，wasm32 侧另跑 `cargo check --target wasm32-unknown-unknown`）<br>· `cargo clippy --workspace --all-targets --locked -- -D warnings`（crates-ui）<br>· `python3 scripts/arch_guard.py --strict`（尺寸与依赖边棘轮）<br>· `python3 scripts/docs_truth.py --verify --block-stale`（状态声明） |
| **时间库迁移（chrono → 公告建议的替代实现）**（2026-11-17 立项）<br>（起因：公告 RUSTSEC-2026-0543 宣布时间库「不再维护」，所有版本都受影响、无安全升级路径 ⇒ main 的依赖审计门禁当日转红 ⇒ 先按仓内 unmaintained 政策显式豁免解阻塞，迁移单独立项） | [chrono-migration-20261117.md](../design/chrono-migration-20261117.md) §目标与范围、§批次与验收、§不做 | [chrono-migration-20261117.md](chrono-migration-20261117.md) §一、批次状态与 §二、还差什么<br>**还差什么**：M1 盘点与选型（6 个在役 crate × 六个语义维度的调用面清单，尚未逐条列出）· M2 逐 crate 替换（尚未动一行）· M3 撤三处豁免（三处各一条仍在）· M4 收口入账（无提交、无文档更新） | cargo deny check（依赖审计四类；本工作线终局守卫）<br>· cargo clippy -p apeireth-core --all-targets --locked -- -D warnings<br>· cargo test -p apeireth-core --locked -j 2<br>· python3 scripts/docs_truth.py --verify --block-stale |

### 一·补 D1 未决裁决（2026-11-19）

本表「后端接线合同」行原列的 5 条未决项，连同 08 的 D1 合同边界项，经 2026-11-19 裁决后收敛为：6 条已裁决、1 条已由其他批次关闭、2 条仍待前端/产品确认。裁决只固定后端边界，不改代码。

**已裁决（6 条）**

1. **流式新会话 session_id**：客户端在首个流式 `/v1/chat/completions` 请求体显式给 `session_id`（新会话可用新 UUID）；后端沿用并回显，不在首帧/响应头补发。客户端因此在发起请求前就知道 session_id；首帧 SSE chunk `id` 是运行时 request id，拿到后即可拼出 C1 订阅 URL。契约见 `docs/gateway-api-contract.md` §3。
2. **/bridge/* 归属**：维持私有前端宿主本地面；后端不实现该前缀，只提供原生 `/v1` 流面，由宿主映射。
3. **context_key / incarnation**：两者均为宿主所有的 opaque 回显位；后端不生成、不解析、不归一化。重绑必须 byte-identical（`None` 与 `Some` 也算不一致），否则 409 `session_binding_conflict`；清空只能显式解绑后重绑。
4. **非空会话 persona**：persona 仅在空会话的 system 种子槽注入；非空会话不追溯、不追加第二条 system；中途换绑定不改变已注入 persona，需新会话。
5. **审批恢复续流**：流式回合以 `approval_required` 终帧结束，C1/SSE 不续流；`POST /v1/approvals/resolve` 返回恢复结果（可能再次待批），需要流式就发新回合。
6. **多进程一致性**：首发限单网关进程/单活跃实例；C1 缓冲、stop 摘要、幂等表、绑定 registry 均进程内，重启即失；多进程共享与持久化属长尾，不做水平扩展承诺。

**已关闭**

- **成果版本对象**：已由 PR #142 建模（受管 Artifact / 不可变 Revision / operation 回执），不再列为未决；见 `docs/design/text-artifact-versions.md`。

**仍待前端/产品确认（2 条，后端维持最保守实现）**

- `workspace_binding` 语义：是否等同于文件系统工作区（`capabilities/tools/src/worktree_apply.rs` 一族）；现按「工作区即受管根」实现。
- 联系人 `capabilities{read,write,approval_required}` 到 `foundation/governance/src/permission.rs` 的映射粒度；现只做展示投影，不作为授权依据。

证据：`docs/design/backend-contracts-20261117.md` §未决与 D1 裁决；`docs/status/backend-contracts-20261117.md` §三第 10 条。

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
