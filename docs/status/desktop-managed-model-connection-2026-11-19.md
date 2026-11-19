# 受管桌面模型连接与只读候选同包：2026-11-19 进度

[Document-Meta]
Document: docs/status/desktop-managed-model-connection-2026-11-19.md
Class: STATUS
Version: 1
Last-Modified: 2026-11-19
Status: 代码、声明与最终二进制隔离复核完成；PR #148 已推送，待必需 CI 与合入；PRODUCTION WIRED，非 HARDWARE VALIDATED
Author: desktop-collaboration-line

## 目标与检查点

[完整契约与批次](../design/desktop-managed-model-connection.md)。基线 `d319345dba43980a79c7a135fb14e5d282e1749d`（2026-11-19 的 upstream/main）。本批把受管模型连接移植到当前主干，与既有受管文本成果/版本、只读候选回合同包。

代码检查点为 feat 提交 `7f91de399aad8b461b0840d8a77f97492e5a33ea`（20 files +1348/−30，父 `d319345db`）；其后 docs(claims) 提交 `f36708ba9` 与批次看板提交 `4ac3cd763` 只更新声明与生成物。合并提交 `a5cb783eba54a880f1b144c94b8d666f74ad1d0f` 带入 PR #146（嵌套 PEM 脱敏）与 PR #147（git_write 根原因）两个 fuzz 修复，最终二进制由该合并点构建。工作树 `C:\Apeireth\wt\desktop-managed-20261119`，分支 `feat/desktop-managed-20261119`。

主要文件：

- `crates/adapters/gateway/src/model_connection.rs`（276 行）：视图、strict patch、校验、错误映射、受管单操作者判定、路由与敏感响应层。
- `crates/adapters/cli/src/desktop_model_connection.rs`（296 行）：数据根档案、Windows 凭据 namespace、保存/重开/回滚、启动装配。
- `crates/adapters/cli/src/desktop_model_connection_tests.rs`（189 行）与 `crates/adapters/cli/tests/desktop_model_connection.rs`（340 行）。
- 接线：`gateway/src/lib.rs`、`gateway/src/panels.rs`、`gateway/src/canonical_entry.rs`、`gateway/src/admin.rs`、`gateway/src/artifacts.rs`、`cli/src/lib.rs`、`cli/src/keyring_bootstrap.rs`、`cli/src/gateway_panels.rs`。

`canonical_entry.rs` 保持 2291 行，等于 arch baseline 2291；`cli/lib.rs` 3740 行、`gateway_panels.rs` 1470 行，均低于各自基线；无新 crate、无新外部依赖。

## 本地门禁

- `cargo test -p apeireth-gateway --lib --locked -j 2`：156 passed / 0 failed。
- `cargo test -p apeireth-gateway --test artifact_candidates_http --locked -j 2`：9 passed。
- `cargo test -p apeireth-cli --lib desktop_model_connection --locked -j 2`：8 passed（137 filtered）。
- `cargo test -p apeireth-cli --test desktop_model_connection --locked -j 2`：4 passed。
- `cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2`：7 passed。
- `cargo clippy -p apeireth-gateway -p apeireth-cli --all-targets --locked -- -D warnings`：exit 0。
- `cargo fmt -p apeireth-gateway -p apeireth-cli -- --check`：exit 0。
- `python scripts/arch_guard.py --strict`：exit 0，VIOLATIONS 0，env 155/155，max_src_loc 6255，exceptions 2；STALE 7 条为既有尺寸提示。
- `PYTHONUTF8=1 python scripts/check_crate_budget.py --gen`：34 crate · 606732 行；`--check` exit 0。
- `python scripts/audit_unwired.py --check`：OK，孤儿基线 89 条，无新增。

`docs_truth.py --verify --check` 与 `--block-stale` 在 claims 更新后均 exit 0：claims=62、fail=0、stale=0、review_due=0、suspected_missing=1（仅 `sec.linux_containment` 的既有文件面提示）。`wiring_report.py --verify` OK；`docs/status/STATUS.md` 由 `--gen` 重生成。新增声明 `desktop.model_connection`，16 条受本批源码范围影响的声明已复核并 bump 到 feat 提交。

## 隔离验收（Windows x86_64，2026-11-19）

环境：数据根 `C:\Apeireth\tmp-desktop-accept\data`；本地 OpenAI-compatible stub 在 127.0.0.1:18771；gateway 在 127.0.0.1:18778；`APEIRETH_DESKTOP_MANAGED=1`，独立强制令牌 32+ 字符，无 `APEIRETH_GATEWAY_TOKENS`。主验收数据根使用合成测试凭据与本地 stub，无真实提供方调用；旧日常根只读核对已存在凭据，未发起模型请求。以下为最终 exe 的复核记录。

- 受管启动成功；`GET /health` 返回 `execution_owner=apeireth-runtime::canonical`；`/v1/runtime/snapshot` 的 `max_rounds=8`。
- `GET /v1/artifacts/capabilities` 200：store 为旧库 `898c8cd6-efb4-46f8-a863-c178af40aeb7`，scope `single_operator`，正文上限 262144。
- 候选能力探测 `GET /v1/artifacts/:id/candidates`：405 且 `Allow: POST`，无模型调用。
- `POST /v1/admin/config` 受管模式：409 `invalid_request`，文案明确拒绝热替换。
- 模型连接保存：200 `ok=true`、`restart_required=true`；档案写入 `desktop-model-159ce333-c0a2-4ad2-988d-f4797adf739c`，同进程 GET 为 `available`、`endpoint_bound=true`、`restart_required=true`。
- 重启后 GET：`available`、`endpoint_bound=true`、`restart_required=false`，证明 Windows 凭据管理器持久化与重开加载。
- 旧 C4 成果库：head `a6b380ba-4a2f-4f9b-b78f-b9627346ca2f`、sequence 3、sha `38727a9d…`、3 个 revision；读取与旧库一致。
- 候选成功：200 `kind=artifact_candidate`，base/head 均 a6b380ba…，`base_is_current=true`，text 42 B、sha `b34ff786…`，request `c2583711-3bbb-41b6-a9c5-b4f5e81cb4e2`，trace `cd3ae7c9-c3d9-4ca0-9e14-d705951c75eb`，rounds 1，tool_calls 0，artifact_writes 0。生成后 head、sequence、revision 数均不变。
- 过期基准：409 `stale_base_revision`，带当前 head；摘要不符：409 `base_digest_mismatch`，带 actual_sha256；两者均未调用 provider。
- 模型要求工具：422 `candidate_tool_call_rejected`，`requested_tool_calls=1`；stub 日志显示该次请求未声明工具，且无工具派发/审批/成果写入。
- 普通对话：200，`session_id=1a327066-a6ab-49e7-aa5f-0ccb959bda70`，trace `7dc57605-7c4a-41e3-8261-bfa3b8c53fa6`，served_by `provider.openai-compatible`，rounds 1，证明受管模型连接可用于既有 canonical 对话面。

### 旧日常数据根升级核对

旧日常根 `C:\Users\Dream\AppData\Local\app.apeireth.desktop\runtime`（已先备份）：

- `GET /v1/admin/model-connection` 返回旧端点、`model=deepseek-flash`、`credential_state=available`、`storage=windows-credential-manager`、`endpoint_bound=true`、`restart_required=false`。
- 停止并以同命令重开：仍为 `available`、`endpoint_bound=true`、`restart_required=false`。
- 旧成果：store `543c50f3-2f5a-4c40-8752-5fc0a74a64d9`，artifact `a05643c5-da3b-4432-a17d-e5accac284d9`（work-local-4，标题「文稿 2」），head `dbdd5f51-7eb5-4dc2-a44d-0302dfc2f8ed`，sequence 3，sha `ba85c0d7bc3756c7c73499e4755f0499383fe29f6e54ec3d89ac49a2881a352d`，6712 B，3 个 revision；逐项与旧库一致。

旧 C4 c3final 根与 c17 根也做了只读核对：前者档案引用的凭据服务不在 Windows 凭据中，返回 `missing`；后者因数据根路径变化导致 keyring namespace 不匹配，同样返回 `missing`。两者都是如实状态，不伪造可用、不静默换库。三个原始 runtime 均已从备份整目录恢复并核对字节数。

### 坏库启动

在 `C:\Apeireth\tmp-desktop-accept\bad-data\artifacts.sqlite3` 写入非 SQLite 文本后启动 `gateway serve --port 18777`：退出码 1，stderr 为 `gateway serve failed: managed artifact service failed to open: artifact storage failure: failed to open sqlite database ...: file is not a database`。数据根未生成空库或额外文件，满足「校验失败明确报错、不静默换空库」。

## 二进制与构建

最终隔离复核使用的 exe：`C:\Apeireth\cargo-target\desktop-managed\release\apeireth.exe`，PE32+ x86_64，45,633,536 B，SHA-256 `56D158DF894BEEC8AFD665C4CB3F8702AE16DFE30437DFD1F38F65390E4A466F`，版本 `2.0.0-rc.5`；LastWrite `2026-11-19T13:04:29.7335445+08:00`。构建命令：`pwsh scripts/cargo.ps1 -Slot base -Line desktop-managed -CargoArgs "build --release -p apeireth-cli --locked"`；日志 `C:\Apeireth\tmp-build-discipline\desktop-release-merge.log`。该二进制由合并点 `a5cb783eba54a880f1b144c94b8d666f74ad1d0f` 的源码构建，包含 #142/#143 成果与候选、#146/#147 fuzz 修复及本批受管模型连接；docs 提交不改代码。早先 `7950452A5428C45A1D5C47894B334A4491B72775E0F775D87CC0A005EA4F29B4` 与 `AF494A9C9A79708F0E9734626F9959A5B36E36FA479F1086453636B0E37B632C` 均已被本次重建取代，不是交付物。

最终 exe 已在最终代码合并点后重建，并完成模型连接读取/重开、旧成果读取、候选成功/过期/摘要不符/工具拒绝、普通对话与管理面拒绝的复核；另以只读方式确认旧日常根的模型连接与旧成果（head `dbdd5f51-7eb5-4dc2-a44d-0302dfc2f8ed`、sequence 3、3 个 revision）可读，进程与临时验收端口均已关闭。

## 还差什么

- PR #148 已推送，待必需 CI（含 fuzz matrix 中 `inputs (git_write)` 与 `inputs (guard_supervisor)`）全绿后合 main；PR body 记录最终 exe SHA 与隔离证据。
- 未做真实提供方调用、桌面 C41 实机联调、多进程/跨平台凭据后端；这些不进本批 PRODUCTION WIRED 口径。
