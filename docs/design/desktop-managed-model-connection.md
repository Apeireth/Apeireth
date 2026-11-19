# C3 受管桌面模型连接（2026-11-19 主干移植）

[Document-Meta]
Document: docs/design/desktop-managed-model-connection.md
Class: SPEC
Version: 1
Last-Modified: 2026-11-19
Status: 施工契约；受管模型连接已接主干装配，隔离验收通过；四级口径 PRODUCTION WIRED，非 HARDWARE VALIDATED
Author: desktop-collaboration-line

## 目标与批次

2026-11-19 接续交接单：把 2026-11-14 在集成线落地的受管桌面模型连接能力移植到当前主干，与主干的受管文本成果/版本和只读候选回合同包交付，使桌面 C41 在保留旧数据根、旧成果库和旧模型设置的前提下启用。

本批只改主干必要面：gateway 的模型连接路由与装配字段、CLI 的受管档案/凭据适配与组合根接线、成果服务的受管单操作者放行。不新增 protocol 服务端口，不改 /bridge/*，不碰私有前端，不增加新 crate 或外部依赖。

## 契约

`GET /v1/admin/model-connection` 返回 `schema_version`、`provider`、`base_url`、`model`、`credential_state`、`storage`、`endpoint_bound`、`restart_required`。`credential_state` 只表示凭据状态：`missing`/`available`/`unavailable`，不等于提供方或网络连通。所有响应不回显密钥或掩码。

`POST /v1/admin/model-connection` 只接受 strict JSON `{base_url, model, api_key?}`；`api_key` 可省略。保存的是下次启动配置，成功回 `{schema_version:1, ok:true, restart_required:bool}`；客户端必须再 GET 核对端点、模型与凭据状态，不能把任意 200 当作成功。保存成功不热换当前 runtime，不重放任务。

校验口径：`base_url` 可解析，只接受 https 或回环地址的 http（`127.0.0.1`/`localhost`/`[::1]`/`::1`），无 query/fragment/userinfo，长度不超过 2048；`model` 去空白后不超过 128 字节并满足 canonical 模型标识；`api_key` 若出现必须非空、不超过 4096 字节且不含控制字符。

错误映射：非法请求 400 `invalid_request`；受管单操作者门不满足 403 `single_operator_required`；档案或凭据存储不可用 503 `profile_unavailable` / `credential_unavailable`；已绑定端点后不带新凭据改端点 409 `endpoint_change_needs_key`。敏感路径响应带 `Cache-Control: no-store` 与 `X-Content-Type-Options: nosniff`。

## 持久与受管装配

非机密档案在数据根 `desktop-model-connection.json`，schema 1，含端点、模型和可选的 Windows 凭据服务引用。档案与凭据引用必须同为存在或同时缺省；凭据服务名形如 `desktop-model-<uuid>`，端点必须与档案一致。写入使用原子持久化与文件锁；新凭据先写入并读回逐字节确认，档案提交成功后才退役旧服务，提交失败则删新服务。

凭据只进 Windows Credential Manager，namespace 由规范化数据根派生，不同数据根不共享。读取/保存失败显式报告，不降级明文文件或假称持久化。启动时把已存凭据恢复到进程内 hot store，provider 从该 store 取用。

`APEIRETH_DESKTOP_MANAGED=1` 且在 Windows 才启用受管形态；必须使用独立的强制本机令牌（至少 32 字符）且未设置 `APEIRETH_GATEWAY_TOKENS`；数据根不得存在 `users.json` 或 `client-tokens.json`；用户表必须是唯一 `default/owner/env/default域` 的内存同形表。受管模式只装配 openai-compatible 协议族 provider，不装配其他厂商 provider；不注入 user/domain 管理服务；`/v1/admin/config` 热替换返回 409 `invalid_request`。

成果与候选服务对受管单操作者放行：只有模型连接服务存在、强制令牌长度达标、无额外客户端令牌、用户表与受管默认表严格同形时才放行；其他激活用户表、额外客户端、非默认域继续 503 `scope_unavailable`。

## 与只读候选回合的兼容

`POST /v1/artifacts/:artifact_id/candidates` 的契约、基准校验、逐轮禁工具、零成果写入和内部分会话边界不变。能力探测对候选路径 GET 仍返回 405 且 `Allow: POST`，不触发模型调用。候选生成前后 head 与版本数不变；过期基准和摘要不符在调用 provider 前拒绝。

## 旧库与失败边界

旧日常数据根的模型档案和 Windows 凭据可被新构建读取；旧成果库的 store_id、artifact、head revision、版本历史和正文摘要保持不变。凭据服务不存在时如实返回 `credential_state=missing`，不伪造可用；数据根迁移导致 keyring namespace 变化时同样如实缺失，不静默换库。

成果库打开失败必须显式报错并退出，不得生成空库顶替。已用损坏 SQLite 文件验证：进程退出码 1，stderr 带 `managed artifact service failed to open` 与底层 `file is not a database`，数据根未新增空库文件。

## 四级口径与边界

- 受管模型连接的档案、凭据、端点校验、重启加载、管理门和单协议族装配：PRODUCTION WIRED（代码在主干、Windows 隔离验收通过）。
- 只读候选回合：沿用既有 PRODUCTION WIRED；本批回归通过，未新增语义。
- 未做真实模型调用的桌面 E2E：隔离验收只用本地 OpenAI-compatible stub；旧日常档案只读核对，不做真实发送。
- 未做私有前端 C41 实机联调；未做多进程/跨平台凭据后端；C1 缓冲、stop 摘要、幂等表和绑定 registry 仍是进程内，重启即失。
- 候选回合仍写内部分 Session/消息/事件用于审计，不承诺零会话写；成果写入为零。

## 验收入口

代码与证据见 [进度页](../status/desktop-managed-model-connection-2026-11-19.md)。关键命令：

- `cargo test -p apeireth-cli --lib desktop_model_connection --locked -j 2`
- `cargo test -p apeireth-cli --test desktop_model_connection --locked -j 2`
- `cargo test -p apeireth-gateway --lib model_connection --locked -j 2`
- `cargo test -p apeireth-gateway --test artifact_candidates_http --locked -j 2`
- `python scripts/docs_truth.py --verify --block-stale`
