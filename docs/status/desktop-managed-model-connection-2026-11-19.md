# 受管桌面模型连接与只读候选同包：2026-11-19 进度

[Document-Meta]
Document: docs/status/desktop-managed-model-connection-2026-11-19.md
Class: STATUS
Version: 1
Last-Modified: 2026-11-19
Status: 代码与隔离验收完成；待提交、最终 release 重建与 PR 合入；PRODUCTION WIRED，非 HARDWARE VALIDATED
Author: desktop-collaboration-line

## 目标与检查点

[完整契约与批次](../design/desktop-managed-model-connection.md)。基线 `d319345dba43980a79c7a135fb14e5d282e1749d`（2026-11-19 的 upstream/main）。本批把受管模型连接移植到当前主干，与既有受管文本成果/版本、只读候选回合同包。

代码检查点为本工作树的 feat 提交（SHA 由后续 docs(claims) 提交绑定）。工作树 `C:\Apeireth\wt\desktop-managed-20261119`，分支 `feat/desktop-managed-20261119`。

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
- `PYTHONUTF8=1 python scripts/check_crate_budget.py --gen`：34 crate · 606682 行；`--check` exit 0。
- `python scripts/audit_unwired.py --check`：OK，孤儿基线 89 条，无新增。

`docs_truth.py --verify --check` 在代码提交前为 exit 0，但报告 16 条受本批源码范围影响的 STALE 声明；必须在 feat 提交后逐条复核并 bump 到该提交 sha，再由 docs(claims) 提交收口。

## 隔离验收（Windows x86_64，2026-11-19）

环境：数据根 `C:\Apeireth\tmp-desktop-accept\data`；本地 OpenAI-compatible stub 在 127.0.0.1:18771；gateway 在 127.0.0.1:18770；`APEIRETH_DESKTOP_MANAGED=1`，独立强制令牌 32+ 字符，无 `APEIRETH_GATEWAY_TOKENS`。所有密钥为合成测试值；无真实提供方调用。

- 受管启动成功；`GET /health` 返回 `execution_owner=apeireth-runtime::canonical`；`/v1/runtime/snapshot` 的 `max_rounds=8`。
- `GET /v1/artifacts/capabilities` 200：store 为旧库 `898c8cd6-efb4-46f8-a863-c178af40aeb7`，scope `single_operator`，正文上限 262144。
- 候选能力探测 `GET /v1/artifacts/:id/candidates`：405 且 `Allow: POST`，无模型调用。
- `POST /v1/admin/config` 受管模式：409 `invalid_request`，文案明确拒绝热替换。
- 模型连接保存：200 `ok=true`、`restart_required=true`；档案写入 `desktop-model-159ce333-c0a2-4ad2-988d-f4797adf739c`，同进程 GET 为 `available`、`endpoint_bound=true`、`restart_required=true`。
- 重启后 GET：`available`、`endpoint_bound=true`、`restart_required=false`，证明 Windows 凭据管理器持久化与重开加载。
- 旧 C4 成果库：head `a6b380ba-4a2f-4f9b-b78f-b9627346ca2f`、sequence 3、sha `38727a9d…`、3 个 revision；读取与旧库一致。
- 候选成功：200 `kind=artifact_candidate`，base/head 均 a6b380ba…，`base_is_current=true`，text 42 B、sha `b34ff786…`，request `8e3598e6-e33c-45c0-9b28-cd1d63afc3fe`，trace `6dffb097…`，rounds 1，tool_calls 0，artifact_writes 0。生成后 head、sequence、revision 数均不变。
- 过期基准：409 `stale_base_revision`，带当前 head；摘要不符：409 `base_digest_mismatch`，带 actual_sha256；两者均未调用 provider。
- 模型要求工具：422 `candidate_tool_call_rejected`，`requested_tool_calls=1`；stub 日志显示该次请求未声明工具，且无工具派发/审批/成果写入。
- 普通对话：200，served_by `provider.openai-compatible`，rounds 1，证明受管模型连接可用于既有 canonical 对话面。

### 旧日常数据根升级核对

旧日常根 `C:\Users\Dream\AppData\Local\app.apeireth.desktop\runtime`（已先备份）：

- `GET /v1/admin/model-connection` 返回旧端点、`model=deepseek-flash`、`credential_state=available`、`storage=windows-credential-manager`、`endpoint_bound=true`、`restart_required=false`。
- 停止并以同命令重开：仍为 `available`、`endpoint_bound=true`、`restart_required=false`。
- 旧成果：store `543c50f3-2f5a-4c40-8752-5fc0a74a64d9`，artifact `a05643c5-da3b-4432-a17d-e5accac284d9`（work-local-4，标题「文稿 2」），head `dbdd5f51-7eb5-4dc2-a44d-0302dfc2f8ed`，sequence 3，sha `ba85c0d7bc3756c7c73499e4755f0499383fe29f6e54ec3d89ac49a2881a352d`，6712 B，3 个 revision；逐项与旧库一致。

旧 C4 c3final 根与 c17 根也做了只读核对：前者档案引用的凭据服务不在 Windows 凭据中，返回 `missing`；后者因数据根路径变化导致 keyring namespace 不匹配，同样返回 `missing`。两者都是如实状态，不伪造可用、不静默换库。三个原始 runtime 均已从备份整目录恢复并核对字节数。

### 坏库启动

在 `C:\Apeireth\tmp-desktop-accept\bad-data\artifacts.sqlite3` 写入非 SQLite 文本后启动 `gateway serve --port 18777`：退出码 1，stderr 为 `gateway serve failed: managed artifact service failed to open: artifact storage failure: failed to open sqlite database ...: file is not a database`。数据根未生成空库或额外文件，满足「校验失败明确报错、不静默换空库」。

## 二进制与构建

隔离验收使用的 exe：`C:\Apeireth\cargo-target\desktop-managed\release\apeireth.exe`，PE32+ x86_64，45,635,072 B，SHA-256 `AF494A9C9A79708F0E9734626F9959A5B36E36FA479F1086453636B0E37B632C`。该二进制构建于 2026-11-19 11:05，之后仅有 `canonical_entry.rs` 的模块别名与注释配平，不改变行为。

最终提交后必须用同一工作树重新构建 release，并把最终 exe 的路径、字节数、SHA-256 和构建命令回填本页；未回填前不得把上述 SHA 当作最终交付物。

## 还差什么

- feat 提交（代码/测试/设计/进度/契约/INDEX/env/预算）与 docs(claims) 提交；16 条 STALE 声明复核并 bump，新增 `desktop.model_connection` 声明，claims 61→62，重生成 STATUS.md/wiring.yaml。
- 最终 release 重建与 SHA-256 回填；push、PR、必需 CI 全绿、合 main。
- 未做真实提供方调用、桌面 C41 实机联调、多进程/跨平台凭据后端；这些不进本批 PRODUCTION WIRED 口径。
