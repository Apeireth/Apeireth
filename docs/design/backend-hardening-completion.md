# 后端语义覆盖、长程质量与系统工具验收

[Document-Meta]
Document: docs/design/backend-hardening-completion.md
Class: SPEC
Version: 0.2
Last-Modified: 2026-11-14
Status: 🟢 活跃
Author: lead

## 目标与范围

在既有正式运行链路上完成更广的记忆语义覆盖、独立长程评测、规模与持续运行压力验证，
并核实系统工具从注册、授权、平台执行到审计结果的完整能力。依据当前源码和进度页确认缺口，
实现后执行对应验收及完整合并门禁。起始主分支为 `ca2b7b70f220b96988f64423b44479c307bd03df`。

本批沿用[记忆质量与持久一致性](memory-quality-hardening.md)的正式证据和恢复合同，
以及[系统能力](../01-architecture/system-capabilities.md)的实现、接线和实机验证分级。
默认关闭、条件接线、缺少外部条件和确实未实现分别记录。

## 批次与验收

| 批次 | 交付目标 | 验收要求 | 当前进度 |
|---|---|---|---|
| B1 | 扩大自然语言事实更正的表达覆盖 | 实际 Runtime 写入与召回；跨主体、跨属性、否定、转述及时间歧义的对抗验证；关键接线必须反向验伪 | 已接线（四级口径：IMPLEMENTED）：语义层 `canonical/memory_correction/semantics.rs` 进入产品判定链（属性分键、极性、作用域句界、起期识别、整源一致性）。`memory_automatic_correction` 26 项全绿（既有 21 + 本批 5），既有断言零改动；反向验伪 6 组摘线必红。未覆盖：英文文法未接线（英文用例只作守门）、跨属性与主体一致性未核 |
| B2 | 独立长历史、多会话的回答质量评测 | 实现冻结后使用未参与调试的自有题集；Oracle 不进入模型输入；固定模型和预算，保留逐例回答、来源、失败和用量 | 评测契约已实现（四级口径：IMPLEMENTED）：`examples/memory_longterm_eval/`（13 模块）+ 唯一 `[[example]]` 入口，单测 14 项全绿、反向验伪 2 组。四类前提逐例留痕，引用可追溯到证据条目，24 次负控全部被拒（真实模型跑为 36 次）。题集是本仓自编回归集，产物显式标注「不是独立盲测」。**真实模型已实测**（deepseek-flash，2026-11-13）：6 例中 4 例通过、2 例失败（模型把「歧义主体」答成「已知」、把「跨作用域」答成「已知」），36 次负控全被拒、4 类前提成立、6 条探针全绿、seal 一次性揭封与仅凭文件复核通过；产物零密钥泄漏已核。独立保管仍如实标未实现 |
| B3 | 规模、并发、恢复与持续运行压力验证 | 明确实际记录数、并发度、真实运行时长与故障点；校验完整性、隔离、重放、资源和全链路分层延迟 | 压力 runner 六阶段已实跑：存储规模 200/200 写读、并发度 4、全链路并发 8 轮 0 失败、规模下召回 p50 56.8ms/p95 60.8ms、故障注入与恢复 pending 1→0、真实墙钟持续 5.001s/11818 次操作 0 错误；虚拟 30 天单列并写明「只拨了时钟，不是持续运行」。首次跑因每 worker 各自迁移导致 `database is locked` 而红，改为先迁移一次再并发服务后转绿 |
| B4 | 系统工具生产能力与真实场景验收 | 逐项沿注册→治理→平台执行→结果审计核对；真实探针只作用于自建临时对象；未验证平台及设备不得记通过 | Windows 已实测（2026-11-13）：真实 netstat 采集跑通并解析出 71 条监听项，退出码/5 秒时限/输出上限/地址列均被断言钉死；可信路径闸门、取消与期限与租约、Scope 与端口预算负例、脱离 reactor 的兼容入口、`elapsed_ms` connect-only 口径均有回归测试。真跑暴露并修复了生产缺陷「清空环境 ⇒ netstat 地址列为空 ⇒ 监听面被误报成已测零发现」。Linux/macOS 未实测，dispatcher 真实链路未端到端 |

## 不变量

- 直接用户证据、Host 授权、当前作用域和修订 CAS 保持为修改事实的必要条件；模型提案不授予权限。
- 原始来源、时间语义、遗忘和恢复回执不能因扩大表达范围而放宽。不确定信息应显式弃权并计入质量结果。
- 既有开发题集保留为回归集，不标作独立盲测。新题一旦用于修复，后续报告应说明它已成为开发证据。
- 规模和持续运行的结果分别报告；虚拟时间推进不表述为相同长度的真实持续运行。
- 系统状态修改经正式治理和批准链；验收不修改用户既有文件、服务、计划任务或设备配置。
- 所有真实模型调用均在软件入口验证后使用本次临时凭据，凭据不进参数、仓库或报告。
- 不导入外部项目实现、提示词或评测数据；保留既有合法依赖及其许可材料。
- LOCKED 项、检索基线及质量门槛保持。失败和未执行不能由平均分、环境跳过或委托状态替代。

## 进度与验收记录

2026-11-13（移交批次，上一个团队留下的记录）：从主分支建立独立工作树，完成语义、评测和系统工具的源码核对，并在三个独立工作树留下草稿。
随后按用户要求停止新增开发，收拢补丁和原始文件移交其他团队。**该批次**没有新增提交、推送或合并，
没有运行 Cargo、真实模型、规模压力或新增 CI。仅运行了起始源码的记忆可达性静态守卫；
它不构成该批草稿的编译、正确性或质量通过证据。

主分支已合并的上一批为 PR #125；本页及该批草稿均不属于该 PR 的已验证交付。
接手方须先审查未完实现并取得实际 RED/GREEN，再执行完整回归及合并门禁。

2026-11-13（接手批次）：从 `ca2b7b70f` 新建整合工作树，先只挂测试补丁取得真实 RED，
再逐条审查接线实现，把三组未编译草稿分别接线成可构建、可运行、可证伪的交付。
本批实际取得的证据：

- B1：语义层进入产品判定链，`memory_automatic_correction` 26 项全绿；
  反向验伪 6 组（作用域、整源一致性、极性、起期识别、属性分键、取值层、修正域）摘线必红、改回必绿。
  被删的两处父模块逻辑已逐行核对为同语义搬家加增强，没有静默删能力。
- B2：`memory_longterm_eval` 单测 14 项全绿；离线跑 6/6 例通过、24/24 负控被拒、6 条差分探针全绿；
  seal 闭环跨进程成立（首次揭封成功、二次 `run --custody` 被拒退出码 1、仅凭文件独立复核通过）。
  反向验伪 2 组（作用域前提恒真 ⇒ 红；引用校验短路 ⇒ 红）。
  **真实模型实测（2026-11-13，deepseek-flash，厂商 OpenAI 兼容端点）**：6 例 4 通过 2 失败、
  36 次负控全被拒、4 类前提逐例成立、6 条探针全绿、8 次提供方调用、提问阶段 p50 1.17s / p95 1.57s；
  两例失败都是模型侧质量失败（歧义主体答成「已知」、跨作用域答成「已知」），不是夹具或门禁失败。
  首跑撞出厂商协议缺口：端点默认开启推理模式，历史里带工具调用的助手轮缺 `reasoning_content`
  会被厂商拒绝（400）；本仓规范化协议没有该字段，本批只加**显式可选、默认不变**的厂商私有字段透传
  （`thinking` / `reasoning_effort`，并拒绝覆盖协议保留键），不改协议本体。凭据只经环境变量注入，
  产物已逐文件核对不含密钥形态串。该轮实测跑在收口前的最后一版代码上；此后只有两处行为等价的重构
  （候选查询构造下沉、recon 调用写法），未再重跑真实模型。
- B3：压力六阶段实跑数据见上表；首次红与修复过程一并记入。
- B4：Windows 真实 netstat 采集跑通并被行级真值逐条对账；真跑暴露生产缺陷「清空环境 ⇒
  地址列整列为空 ⇒ 监听面被误报成已测零发现」并修复（只放行 `SystemRoot`，其余环境清空）。
  反向验伪 16 组，其中 F16「关掉监听态判定」第一轮竟绿，靠补行级真值判据后二次回退才转红。
- 附带修复：长历史候选发现阶段用近期无关记录挤掉早期关键事实。修复走同一预算内的两条切片
  （新鲜度 + 相关度），三处后端共用 `backend/candidate.rs` 一份可见性口径；
  反向验伪 8 组（含极性翻转）摘线必红。实测 1 万条 episode 候选阶段 24.5ms → 42.7ms，
  增量几乎全在作用域逐行 JSON 解析（≈17.9ms），绝对值偏乐观（内存库、未优化 profile），
  作用域索引化列为下一轮候选。
- 收口门禁（2026-11-13，本批最终树）：`cargo test --workspace --all-targets --locked --no-fail-fast -j 2`
  退出码 0；`cargo test --workspace --doc --locked -j 2` 退出码 0；
  `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 退出码 0（0 warning）；
  `pwsh -NoProfile -File scripts/fmt-check.ps1` 35 个 crate 全部 0 diff；
  `python scripts/arch_guard.py --strict` 违规 0；
  `python scripts/check_golden_baseline.py` 通过：内部卷检索基线未劣化（redchamber recall@10=0.5305、
  watershed recall@10=0.7319，证据送达率 Δ +0.0000）⇒ 本批的候选切片改动没有把检索分数压下去。
  该门禁首次跑红的根因是**语料定位口径**（Rust 侧按 `$APEIRETH_DATA_DIR/novels` 找语料，本机 HOME 为空 ⇒
  一份语料都没跑到 ⇒ 判「不能出分数」）；本批按 `APEIRETH_DATA_DIR` 指向本机语料后实跑，
  不改门槛、不改「残缺语料⇒判红」的判定逻辑。
  `python scripts/docs_truth.py --verify --check` claims=60 fail=0 stale=0 review_due=0
  （suspected_missing=1：Linux 声明与同目录 Windows 文件的同目录启发式提示，已复核不硬塞）；
  `python scripts/audit_unwired.py --check` 孤儿基线一致（90 条，无新增、无过期）；
  `python scripts/wiring_report.py --check-env-registry` 覆盖 165/165（本批新增的
  `APEIRETH_MEM_BENCH_ROWS` 是候选阶段基准夹具的测试旋钮，已登记为 test-only，不是产品开关）；
  `python scripts/wiring_report.py --verify` 生成物与源一致。
  `python scripts/wiring_report.py --crosscheck` 另有 2 条报告级提示（`sec.governance_chain_drill`、
  `sec.terminal_permission_surfaces` 标 PRODUCTION WIRED 但能力矩阵无生产引用）：这两条声明与能力矩阵
  本批均未改动，属上游既有提示，本批不处理。
  体量基线按官方修法①用 `python scripts/check_crate_budget.py --gen` 重写（四个 crate 的增长来自本批
  新增文件，不是口径放宽，理由写在该条提交信息里）。
- 仓库统一验证入口（`python scripts/harness.py verify --base ca2b7b70f`，按整批 diff 判定）：
  fast 阶段 30 条门禁全部 PASS；deep 阶段 claims-verify / arch-strict / unwired-groups / guard-wiring /
  golden-baseline / real-query-baseline / d4-ledger 共 7 条 PASS，ci-rust / ci-nextest / ci-clippy /
  ci-fmt / ci-doc-test / ci-cargo-deny / ci-promise-audit 共 7 条记 **DELEGATED（不是本地通过）**，
  由 GitHub Actions 负责 ⇒ 合并前须补 Actions 结果。
  首次按 HEAD~1 口径跑时 fast 阶段有两条红：`check_doc_caliber.py` 报 4 处「段落声称已实现/已接线
  但同段没有四级口径标注」，`check_batch_status.py --check` 报进度页事实块写 59 条声明而代码事实 60 条
  ⇒ 已按各自官方修法修掉（补同段口径标注 + `--gen` 重写事实块并还原 LF 行尾），复跑全绿；
  未放宽任何门槛，也未手改生成物。
- 本地补跑 CI 专属门禁的等价命令（2026-11-14，Windows 单机）：本机装有 `cargo-nextest 0.9.151` 与
  `cargo-deny 0.20.2`，按 CI 侧同一条命令本地实跑——`cargo nextest run --workspace --profile ci --locked`：
  **11069 个用例全部通过**（494 个测试二进制、49 个跳过，耗时 607.2s，退出码 0）；
  `cargo deny --all-features --offline check`：advisories / bans / licenses / sources 四类全 ok（退出码 0）；
  `python scripts/test_nextest_reporting.py` 退出码 0（重试证据隔离与 2/4 worker 资源占用探针均 PASS）。
  两处如实标注：① `cargo-deny` 的 advisories 用的是本机缓存的 advisory-db 快照，**不是当日刷新**——
  当日刷新被本机全局 git 代理配置（`http.https://github.com/.proxy` 指向当前未运行的本地代理）挡住，
  本批不擅自改用户全局 git 配置；② 本地 nextest 没有落出 JUnit 文件，CI 侧 JUnit 产物与上传属
  Actions 上报链，本地未复现该产物（不影响 pass/fail 结论）。
  这一步只补「本机能自证的等价证据」，**替代不了 Actions**：OS matrix（Linux/macOS）、
  真实 CI runner 环境与 Actions 产物上报仍只有 CI 能给。
- 架构棘轮（`scripts/arch_guard.py --strict`）：本批改动让 `coordinator.rs`（2418→2424）与
  `recon_tool.rs`（1664→1667）超了尺寸棘轮 ⇒ **不抬基线盖章**（棘轮只减不增，例外簿额度已满）⇒
  改为把两文件压回基线以内（两条 refactor 提交，行为等价：候选查询构造下沉到 `scope.rs`、
  recon 调用改用本批建好的再导出面），并把 `memory_correction.rs` 的指纹按实测从 1535 缩到 1413
  （逻辑搬进新模块后该文件真实变小）。
  本批按用户要求**只本地提交**（共 12 条提交），不推送、不开 PR、不合并（该阶段口径；
  其后按 2026-11-14「一切以上线为目标」的新指示改为推送并开 PR #126，见文末最终裁决）。
- 上线收口批次（2026-11-14，按「一切以上线为目标」推进）：
  - 生产工厂接线：新增 `crates/engine/provider/src/vendor_body.rs` 作为厂商私有请求字段的**命名、解析与校验单一真源**；
    生产 `OpenAiCompatibleLlmFactory::from_env_with_resolver` 在**构造期**读取 `APEIRETH_OPENAI_THINKING` 并挂到 capability。
    不设 ⇒ 不挂 extra body ⇒ 请求体与接线前**逐字节一致**；取值非法 ⇒ 构造期 `FactoryError::VendorBody` 报错
    （不等第一次真实请求才发现）；不认得该字段的端点会被厂商直接 400 ⇒ 该字段是**显式可选**的（四级口径：IMPLEMENTED）。
  - 反向验伪（主代理亲做）：摘掉工厂接线 ⇒ `vendor_body_factory_wiring` 两项红
    （`the_reasoning_switch_reaches_the_wire` 与 `the_factory_sends_only_the_field_it_owns`，
    原文 `left: None  right: Some(Object {"thinking": Object {"type": String("disabled")}})`），
    改回 ⇒ `ok. 5 passed; 0 failed`。
  - env 名额：架构契约按 src 口径只数能力开关，预算 154。本批最初把三个新名字放进生产 `src/`
    ⇒ arch-strict 实测 156 ⇒ **超预算** ⇒ 按契约「不许顺手加」把生产只留**一个**（推理模式开关），
    另外两个（评测侧推理强度、额外体 JSON）退回评测工具侧（`examples/` 不占 src 口径预算）⇒
    实测 154 = 预算 154，**预算值未改**；契约里补记了本次用掉最后一个名额的逐条核对
    （已登记 / 默认关且关时零变化 / 为何不能走配置项 / 不许顺手加）。
  - 评测侧改为复用 provider 的同一份映射（`vendor_body_from_parts` 与 `parse_thinking`）⇒
    生产与评测不再各维护一份真源；评测自己的两个开关名留在 `examples/` 侧。
  - 如实标注的缺口：① CLI 有两处装配点用 `if let Ok(factory)` ⇒ **构造错误被吞**
    （`crates/adapters/cli/src/lib.rs:2492` 的 `build_council_from_env` 与
    `crates/adapters/cli/src/llm_factory_env.rs:37`），只有同文件 `:2588` 的
    `build_subagent_orchestrator_from_env` 把错误透出 ⇒ 运维写错开关值时，这两条路径会静默回退默认工厂；
    ② 真实模型那一轮（4/6）跑在工厂接线**之前** ⇒ 上线前须按 R2 口径复跑一轮真实端点。
  - 本批门禁：provider 单测、评测 example 单测（14 项）、两 crate clippy（`-D warnings`，0 warning）、
    fmt-check（35 crate 0 diff）、arch-strict（0 违规）、env registry（168/168 覆盖）。
  - 提交形态：本分支截至本批共 **14 条提交**（自 `ca2b7b70f` 起），仍**只本地提交**，不推送、不开 PR、不合并；
    用户要求「一切以上线为目标」，合并与推送的时机由用户决定。
- 上线收口裁决（2026-11-14，按 R1「CI 裁决交给 Actions」执行）：
  - 推送与 PR：分支 `feat/backend-hardening-completion-20261113` 已推到 fork（`Xephylos/Apeireth`），
    并向上游 `Apeireth/Apeireth` 的 `main` 开 **PR #126**（https://github.com/Apeireth/Apeireth/pull/126 ）。
    PR 的全部检查 **43 项通过 / 0 失败 / 1 项 skipping**（kani 的非默认 harness 矩阵项），
    其中 **`合并裁决（唯一）`（merge-gate）通过** ⇒ CI 侧裁决门禁已由 Actions 出结论，
    不再依赖本地对 cargo 门禁的代跑（本地代跑见上一批记录，只作辅助证据）。
  - CI 首轮红 ①（供应链公告）：`cargo deny check` 报 `RUSTSEC-2026-0538`
    （`quinn-udp 0.5.15`，受影响链 `quinn-udp ← quinn ← apeireth-mesh-quic ← apeireth-cli`）。
    该公告**不是本批引入**（`git diff --stat ca2b7b70f..HEAD -- Cargo.lock` 为空，是新发布公告），
    修法按公告原文做补丁级升级 `quinn-udp 0.5.15 → 0.5.16`（提交 `34d793ab9`），
    **未加任何 advisory 豁免**；拒绝的替代方案＝`[advisories] ignore` 豁免（放宽门槛）、
    跨到 `0.6.3`（牵动 quinn-proto 全链）、全量 `cargo update`（大版本漂移）。
    复跑后 `cargo deny check`（advisories/bans/licenses/sources 四类）**pass**。
  - CI 首轮红 ②（macOS 专属用例偶发）：`cargo nextest (macos-latest)` 有一条
    `macos_tests::timeout_terminates_the_whole_process_group_tree` 以
    `Io("Operation not permitted (os error 1)")` 失败。**不在本 PR 的改动面内**：
    失败文件 `crates/capabilities/tools/tests/process_executor.rs` 在本 PR 的 diff 里为空，
    本 PR 在 `src/process/` 下只改了 `windows.rs` 的一个 `pub(crate)` 可见性；
    错误源是 `crates/capabilities/tools/src/process/macos.rs` 的 `setpgid(0, 0)`
    （进程已是组长时返回 EPERM）。**重跑同一 job（attempt 2）即绿**，同一批 43 项复跑后全绿 ⇒
    记为 macOS runner 相关偶发，未在本 PR 内改动该平台实现（改它会牵动进程树治理，不属本批范围）。
  - 本地统一验证入口复跑（最终树）：`python scripts/harness.py verify --base ca2b7b70f`
    ⇒ fast **30 项全 PASS**、deep PASS=7（claims-verify/arch-strict/unwired-groups/guard-wiring/
    golden-baseline/real-query-baseline/d4-ledger）+ DELEGATED=7（ci-rust/ci-nextest/ci-clippy/ci-fmt/
    ci-doc-test/ci-cargo-deny/ci-promise-audit）⇒ **VERDICT: PASS**；
    `python scripts/harness.py attest` 生成绑 `34d793ab9…` 的证据包
    （`artifacts/harness/` 在 .gitignore 内，不入仓、不脏工作树）。
  - R2 取证现状（如实）：生产工厂 → 真实端点的链路已通到「请求发出并拿到厂商应答」这一步
    （探针打印 `extra_body_keys = ["thinking"]`、`extra_body_present = true`，
    随后的 401 是厂商应答，不是本地拼装失败）；但 2026-11-13 那枚临时 key 已失效
    （厂商回 `401 … Your api key: ****e5de is invalid`）⇒ **本轮未取到 200 证据**，
    需另取一枚本次临时 key 复跑一轮（凭据只经环境变量注入，不进参数/源码/日志/产物）。
  - 本分支提交形态：截至本条共 **18 条提交**（自 `ca2b7b70f` 起），**已推送并开 PR，未合并**
    （合并须由上游维护者在 PR #126 上按仓库流程执行）。
- 上线前必做 R2/R3 收尾（2026-11-14，用本次临时 key 实测）：
  - R2 生产工厂 → 真实端点：新增 `crates/engine/provider/tests/openai_compatible_vendor_body_live.rs`
    （`#[ignore]`，按仓内同族 live 用例的纪律：默认不跑、手动跑、0 commit key、0 print key、CI 无 key 不跑）。
    手动跑一次（`APEIRETH_OPENAI_THINKING=disabled` + `APEIRETH_OPENAI_URL` + `APEIRETH_OPENAI_MODEL` +
    `OPENAI_API_KEY`）⇒ `LIVE endpoint = https://api.deepseek.com/v1 model = deepseek-flash
    vendor_field_keys = ["thinking"]`、`LIVE ok model = deepseek-flash content_len = 2
    vendor_field_keys = ["thinking"] usage = 18/1 tokens`，`test result: ok. 1 passed; 0 failed`。
    ⇒ **生产装配路径到厂商 200 的链路打通**（此前只有 401：旧 key 失效）。
    「没设开关 ⇒ 不发厂商私有字段」由**不联网**的 `vendor_body_factory_wiring.rs` 用 mock 服务器钉住。
  - 评测 live 路径复跑（同一枚 key）：`seal --live` ⇒ EXIT=0；`run --live` ⇒ EXIT=1（**2 例模型侧质量失败**，
    不是夹具或门禁失败）：6 例中 4 例通过、36/36 负控全拒、6 条探针全绿、`seal_chain.status = "verified"`
    且 `seal_body_digest_recomputed == seal_sha256`；`verify --live`（同一份环境）⇒ `verified: true`、
    `reveal_state: "verified"`、`reveal_verified_from_files: true`、EXIT=0；
    **同一 custody 复跑被拒**（`one-time use: the shard … was already revealed at 2026-11-14T07:23:25Z`）；
    配置不一致时 `verify` **fail-closed** 并给出原因（`the freeze does not bind this configuration`）
    ⇒ 封存链确实绑定了「本次配置 + 本次 EXE + 题集」。
  - 真实计价：产物里 `cost.status = "wired"`，按本轮配置的单价折算出金额（如首例 0.00033538 USD），
    `price_source` 写明官方价格页与所用档位（deepseek-flash 高峰档：cache miss 输入 0.3、输出 1.2、
    cache hit 0.006 USD/1M，`price_date = 2026-11-14`）⇒ 上一轮「真实计价未接线」在本轮**已接线并有真实用量**
    （四级口径：IMPLEMENTED，计价能力属评测契约 `mem.longterm_eval_contract`，默认 OFF、需显式给单价才折算）。
  - 密钥零泄漏：产物目录与封存目录共 15 个文件，检索 key 值与 `sk-[A-Za-z0-9]{8,}` ⇒ **0 命中**；
    凭据只经环境变量注入，未写入任何文件、参数、日志或报告。
  - R3 暴露面体检开关：**用户已拍板首发保持 default OFF**（2026-11-14）⇒ 部署侧不设
    `APEIRETH_ENABLE_EXPOSURE_AUDIT`，对外声明与默认行为一致（`sec.exposure_audit` = PRODUCTION WIRED + default OFF）。
  - 本批新增/变更：1 个 live 用例文件（`tests/openai_compatible_vendor_body_live.rs`，默认不跑）+ 本节与
    §上线就绪口径的文档更新；**不改**生产代码、不改契约、不改基线、不新增 env 名。
- 最终上线裁决（2026-11-14，PR #126 的最终代码提交 `c24bad42e…`）：
  - Actions 结论：**43 项 success / 0 项 failure / 2 项 skipped**（跳过的两项＝`auto-merge patch/minor
    dependabot PR`（仅对 dependabot PR 生效）与 `kani · ${{ matrix.harness }}`（该矩阵项本仓不适用））。
    关键门禁逐项 success：`harness / merge-gate`（唯一合并裁决）、`Cargo deny`（公告已消）、
    `Golden baseline`（内部卷检索基线）、`arch-guard`、`Unwired API ratchet`、`Dead code ratchet`、
    `Wiring ledger`、`Memory reachability ratchet`、`Docs truth`、`Doc caliber`、`Doc paths`、
    `Cargo audit`、`Kani`、`Miri`、`Protocol integration tests`、`rustdoc (nightly)`、
    `Rust tests (OS matrix + cargo-nextest)` 的 **ubuntu-latest / windows-latest / macos-latest 三个平台**。
  - 本地统一验证入口在同一提交上复跑：`python scripts/harness.py verify --base ca2b7b70f`
    ⇒ fast **30 项全 PASS**、deep PASS=7 + DELEGATED=7 ⇒ **VERDICT: PASS**（HARNESS_EXIT=0）；
    `python scripts/harness.py attest` 生成绑 `c24bad42e…` 的证据包
    （`artifacts/harness/<commit_sha>/`，在 .gitignore 内，不入仓、不脏工作树）。
  - 提交形态：截至本条共 **20 条提交**（自 `ca2b7b70f` 起），**已推送到 fork 并开 PR #126，未合并**
    （合并由上游维护者按仓库流程执行）。
  - 本条为纯文档记录，不含代码改动；此后的任何提交以 `git log` 为准。

### 已确认的待复现或待实施项

- 【长尾 L2】自动纠错的英文文法、跨属性与跨主体一致性核验尚未覆盖；语义层目前是中文闭集文法。
- 【长尾 L1】情节候选挤掉长历史关键证据的缺陷已修复（同一预算内两条切片，三处后端共用一份可见性口径）；
  作用域逐行 JSON 解析的成本（1 万条约 18ms）未消除，作用域索引化留待下一轮，须保持
  作用域、治理状态和时间过滤不被放宽。
- 【长尾 L4】netstat 的统一执行边界与 Recon 的整体期限、取消和异步执行已在 Windows 上实测；
  Linux/macOS 分支与 dispatcher 真实链路仍未实测。
- 【长尾 L3】长程评测的 CLI、正式装配、封存执行闭环和压力 runner 已实现并实跑（四级口径：IMPLEMENTED）；真实模型路径已实测一轮
  （4/6 通过，2 例模型侧质量失败），但题集仍是本仓自编回归集而非独立盲测，单轮结果不构成质量水平结论；
  独立保管（需另一持有人或另一台机器）与真实计价仍未接线。
- 【长尾 L5】厂商推理模式下的 `reasoning_content` 回传未在协议层实现：本批只提供厂商私有字段透传这一条显式通道，
  需要回传推理轮的厂商仍须走协议改造；本批实测端点以关闭推理模式的方式接入。
- 【上线前必须 R2 · 真实端点已实测】生产侧推理模式的口径由部署环境变量决定（见 §上线就绪口径）：默认值是**不发**任何厂商私有字段，
  显式设 `APEIRETH_OPENAI_THINKING=disabled` 才关闭该端点的推理模式；取值非法时在工厂构造期报错。
  2026-11-14 用本次临时 key 实测：生产工厂 → 真实端点拿到 200 应答；评测 live 路径封存链 verified、真实计价已折算
  （详见 §进度与验收记录的「上线前必做 R2/R3 收尾」）。**剩余缺口只剩一处**：CLI 的两条装配路径会吞掉构造错误
  （静默回退默认工厂）⇒ 运维写错开关值时不会报错。
- 【上线前必须 R2】架构契约的 env 名额已用尽（实测 154 = 预算 154，预算值未改）：下一轮任何新增能力开关名
  必须先删一个或改走配置项，不得再「顺手加」。
- 真实模型首跑曾出现一次传输层失败（厂商侧连接中断，非 400 业务拒绝），重跑通过；
  该类瞬时失败没有进入夹具的重试口径。
- 【上线前必须 R1 · 已完成】CI 专属门禁已由 Actions 出结论：PR #126 的 43 项检查全绿
  （含 `cargo nextest` 三平台、`cargo deny check`、`clippy 3 档 lint`、`cargo fmt --check`、
  `cargo test --doc` 三平台、`cargo miri`、`cargo kani`、`cargo tarpaulin`、`cargo audit`、
  死代码/孤儿 API/记忆可达率棘轮、wiring ledger、docs truth、doc caliber、golden baseline、
  以及唯一的 **`合并裁决（唯一）` merge-gate**）。本地此前跑的等价命令（nextest 11069 全通过、
  cargo-deny 四类 ok）**只作辅助证据**，其中 cargo-deny 的 advisories 用的是本机缓存快照而非当日刷新
  （当日刷新被本机全局 git 代理配置挡住，本批不改用户全局配置）⇒ 供应链与 OS matrix 的最终裁决以 CI 为准。
- 结构化系统观测、通用服务/计划任务和真实设备协议须继续设计及验收；已有正式 Shell、文件及隔离能力应复用。
- 系统能力规范中的全工作区纯 Safe/零 FFI 口径需依据实际平台边界复核并收窄，尚未修改。

## 上线就绪口径与长尾归类

本节是本批的上线口径与后续排期。判定规则：**会破坏用户数据、让对外声明超出事实、或让整条产品路径不可用**
⇒ 上线前必须做完；只影响性能、评测严谨度或额外语言覆盖 ⇒ 归入长尾，**降口径也算闭环**（不得写成已覆盖）。

### 上线前必须（3 项）

| 编号 | 事项 | 状态 | 验收口径 |
|---|---|---|---|
| R1 | 合并裁决交给 GitHub Actions | **已完成**：PR #126 的 43 项检查全绿，`合并裁决（唯一）`（merge-gate）通过；首轮两条红（供应链公告 `RUSTSEC-2026-0538`、macOS 专属用例 runner 偶发）已分别按公告补丁升级与重跑复核处理 | 已推送到 fork 并开 PR（fork → upstream `main`），Actions 对本批提交给出结论；`merge-gate` 是唯一合并裁决，它对含 Rust 变更的 PR 只给 BLOCKED（本地不能认证 cargo 门禁）⇒ 以 Actions 结论为准。**合并动作本身仍待上游维护者执行** |
| R2 | 生产模型推理模式口径 | **已实测（2026-11-14，用本次临时 key）**：生产 LlmFactory → 真实端点拿到 200 应答（`vendor_field_keys = ["thinking"]`，content 非空，usage 18/1 tokens）；评测 live 路径复跑 4/6 通过、封存链 verified、真实计价已折算 | 部署侧显式设 `APEIRETH_OPENAI_THINKING=disabled`，或不设（默认零变化 ⇒ 不发厂商私有字段）；需要回传推理轮的厂商走协议改造（长尾 L5）。⚠️ 剩余缺口：CLI 的两条装配路径会吞掉构造错误并静默回退默认工厂（见 §进度与验收记录） |
| R3 | 暴露面体检开关口径 | **已拍板（2026-11-14）：首发保持 default OFF** ⇒ 部署侧不设 `APEIRETH_ENABLE_EXPOSURE_AUDIT`，声明与默认一致 | 首发平台是 Windows 桌面：上线前须明确该开关首发是否打开（开＝每次启动跑一次真实 netstat 采集；不开＝声明保持 default OFF），两处口径必须一致 |

### 长尾（以后有空做）

| 编号 | 缺口 | 为什么归长尾 | 升级为「上线前必须」的触发条件 |
|---|---|---|---|
| L1 | 记忆候选阶段的作用域逐行 JSON 解析成本（1 万条约 18ms） | 只影响性能；B3 实测规模下召回 p50 56.8ms / p95 60.8ms 可接受 | 召回延迟进入首发体验指标时 |
| L2 | 英文文法、跨属性与跨主体一致性 | `mem.automatic_correction` 的 notes 写明「不声明一般自然语言语义完备」；对外材料不得写成通用自然语言语义 | 对外宣称支持英文或一般自然语言时 |
| L3 | 评测的独立保管、真实计价、真人盲测 | `mem.longterm_eval_contract` = IMPLEMENTED 且 default OFF；真实模型单轮 4/6 是模型侧质量失败，不是产品缺陷 | 把该评测当作上线质量裁决依据时 |
| L4 | Linux/macOS 产品行为实测、dispatcher 端到端 | 首发平台是 Windows 桌面，unix 分支 fail-closed | 首发支持 Linux/macOS，或 dispatcher 进入首发链路时 |
| L5 | 协议级 `reasoning_content` 回传 | 本批给了显式可选的厂商私有字段透传通道，可关闭推理模式规避 | 目标厂商强制要求回传推理内容时 |
| L6 | 架构棘轮剩 1 条可缩表项（`size crates/engine/memory/src/coordinator.rs 2418`） | 差额来自注释搬家而非能力收缩，且该文件他线在改 | 下一次架构波收口评审 |

未列入上两表的开放项（结构化系统观测、通用服务与计划任务、真实设备协议、系统能力规范的全工作区
纯 Safe/零 FFI 口径）沿用既有设计稿路线，不因本批上线而改变范围。

## 验收入口

本批专项运行命令随实现落地登记；现有回归与合并出口为：

```sh
cargo test -p apeireth-runtime-assembly --test memory_automatic_correction --test memory_writeback_recovery --test memory_autonomy_production --example memory_answer_eval --example memory_longterm_eval --locked -j 2
cargo test -p apeireth-memory --locked -j 2
cargo test -p apeireth-tools-canonical --locked -j 2
cargo test -p apeireth-provider --test canonical_openai_compatible --locked -j 2
cargo run -p apeireth-runtime-assembly --example memory_longterm_eval -- run --output <目录>
cargo run -p apeireth-runtime-assembly --example memory_longterm_eval -- pressure --output <目录>
python scripts/check_memory_reachability.py --check
python scripts/docs_truth.py --verify --check
python scripts/harness.py verify
```

真实模型那一跑的入口与口径：凭据只经 `OPENAI_API_KEY` 注入，端点与模型经
`APEIRETH_OPENAI_URL` / `APEIRETH_OPENAI_MODEL` 指定，厂商私有请求字段经
`APEIRETH_OPENAI_THINKING`（`enabled` / `disabled`）与 `APEIRETH_OPENAI_REASONING_EFFORT` 指定，
单价经 `APEIRETH_PRICE_*` 指定并写明单价来源；`run --live` 才调用真实模型，默认仍是离线确定性路径。
产物默认写 `--output` 目录，不写进仓库。

评测产物与压力产物默认写 `--output` 指定目录，不写进仓库；真实模型运行需在软件入口通过后
用本次临时凭据注入环境变量，凭据不进参数、仓库、产物或报告。

真实模型运行、持续压力运行和系统探针各自记录完整命令、环境、范围及结果；
没有实际执行的入口不记作通过。最终合并以对应提交的全部适用 CI 和原始证据裁决。
