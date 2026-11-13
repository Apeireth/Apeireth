# 后端语义覆盖、长程质量与系统工具验收

[Document-Meta]
Document: docs/design/backend-hardening-completion.md
Class: SPEC
Version: 0.1
Last-Modified: 2026-11-13
Status: 🟡 草稿
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
- 架构棘轮（`scripts/arch_guard.py --strict`）：本批改动让 `coordinator.rs`（2418→2424）与
  `recon_tool.rs`（1664→1667）超了尺寸棘轮 ⇒ **不抬基线盖章**（棘轮只减不增，例外簿额度已满）⇒
  改为把两文件压回基线以内（两条 refactor 提交，行为等价：候选查询构造下沉到 `scope.rs`、
  recon 调用改用本批建好的再导出面），并把 `memory_correction.rs` 的指纹按实测从 1535 缩到 1413
  （逻辑搬进新模块后该文件真实变小）。
  本批按用户要求**只本地提交**（共 10 条提交），不推送、不开 PR、不合并。

### 已确认的待复现或待实施项

- 自动纠错的英文文法、跨属性与跨主体一致性核验尚未覆盖；语义层目前是中文闭集文法。
- 情节候选挤掉长历史关键证据的缺陷已修复（同一预算内两条切片，三处后端共用一份可见性口径）；
  作用域逐行 JSON 解析的成本（1 万条约 18ms）未消除，作用域索引化留待下一轮，须保持
  作用域、治理状态和时间过滤不被放宽。
- netstat 的统一执行边界与 Recon 的整体期限、取消和异步执行已在 Windows 上实测；
  Linux/macOS 分支与 dispatcher 真实链路仍未实测。
- 长程评测的 CLI、正式装配、封存执行闭环和压力 runner 已实现并实跑（四级口径：IMPLEMENTED）；真实模型路径已实测一轮
  （4/6 通过，2 例模型侧质量失败），但题集仍是本仓自编回归集而非独立盲测，单轮结果不构成质量水平结论；
  独立保管（需另一持有人或另一台机器）与真实计价仍未接线。
- 厂商推理模式下的 `reasoning_content` 回传未在协议层实现：本批只提供厂商私有字段透传这一条显式通道，
  需要回传推理轮的厂商仍须走协议改造；本批实测端点以关闭推理模式的方式接入。
- 真实模型首跑曾出现一次传输层失败（厂商侧连接中断，非 400 业务拒绝），重跑通过；
  该类瞬时失败没有进入夹具的重试口径。
- 结构化系统观测、通用服务/计划任务和真实设备协议须继续设计及验收；已有正式 Shell、文件及隔离能力应复用。
- 系统能力规范中的全工作区纯 Safe/零 FFI 口径需依据实际平台边界复核并收窄，尚未修改。

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
