# Apeireth 安全与隐私报告

[Document-Meta]
Document: docs/status/backend-security-20261121.md
Class: STATUS
Version: 0.2
Status: 活跃
Last-Modified: 2026-11-21
Author: Codex

## 基线与批次

以 `2459a3ca0` 为基线，在 `fix/backend-security-20261121` 开工。批次及验收见[设计稿](../design/backend-security-20261121.md)。

## 现有安全边界

| 边界 | 设计与实际接线 | 阶段性结论 |
|---|---|---|
| 网关、身份和域 | 链路按网关令牌、客户端或用户归属及面板角色检查。受管桌面运行时使用本机连接和独立凭据。 | 非回环监听必须有令牌及 TLS。独立域删除和在途写入需要独立验收。 |
| Mesh 配对与传输 | LAN 发现只提供设备线索。节点经双方配对登记后，以固定的 Ed25519 身份指纹完成 QUIC 双向 TLS；连接握手再核对声明的节点身份。 | 发现结果不会直接进入工具和模型路由。多机部署与撤销传播需真实环境验收。 |
| 工具和外部执行 | 工作区对象经过范围判定，工具经过风险分级和审批链。外部工具与模型出网经过各自的边界。 | MCP stdio 收窄环境，保持启动账户权限。Windows、macOS、Linux 的实际隔离强度分别核实。 |
| 模型供应商 | 供应商凭据与请求端点的来源可分别配置。生产 CLI 运行时将对话和工具历史交给出网闸。 | 脱敏覆盖规则识别到的字段。显式启用的评估专用 HTTP 入口会为答卷保留请求和响应内容，凭据保留在认证头。 |
| 数据、日志和备份 | 数据根保留会话、记忆、设备与配置，桌面工作区有本地存储。主体遗忘基于可归属记录和各存储回执。 | 保护范围取决于实际启用的库、密钥和环境。较老副本、备份、外部供应商数据和桌面存储需要分别验证。 |
| 仓库与发布 | 现有私密报告渠道、仓库门禁、秘密扫描、依赖审计、发布构件校验和签名。 | 真实吊销、远端分支或镜像处置、签名发布和跨平台 CI 需相应服务的授权与回执。 |

## 本地工作记录

| 批次 | 当前状态 | 验证入口 |
|---|---|---|
| A | 工具域名解析须全部通过公网判定。MCP HTTP/SSE 禁止代理和自动跳转，SSE 消息端点保持同源，stdio 清理环境。单个 stdio 帧、HTTP 响应体和 SSE 待处理缓冲区限制为 4 MiB。 | `cargo test -p apeireth-tools-canonical --lib --test mcp_bridge_e2e --locked -j 2` |
| B | HTTP 与 CLI 共用删户预检和结果确认。目标主体、域或五账结果无法确认时保留账号。HTTP 暂拒独立用户域遗忘，CLI 仅用可确定的目标域数据库。源码提交 `9574f0145`。 | `cargo test -p apeireth-gateway --test user_admin_endpoints --locked -j 2`；`cargo test -p apeireth-cli --test user_admin --test user_admin_dual_entry --locked -j 2` |
| C | 普通网关按端点来源和供应商类型核对已有绑定，串行化管理更新。凭据写入端不支持绑定或持久化失败时保留旧的运行时状态。Windows 受管入口现有令牌与端点合同保持。源码提交 `af877a2f7`。 | `cargo test -p apeireth-gateway --lib --test admin_binding_warning --test admin_config --locked -j 2`；`cargo test -p apeireth-cli --lib keyring_bootstrap --locked`；`cargo test -p apeireth-provider --test endpoint_binding --locked` |
| C（凭据一致性） | `f31be8fbd` 使凭据、origin 绑定和请求 URL 使用同一请求快照。密钥环的伴随记录一起持久化值与绑定，新值在持久写成功后才热生效。 | plugin、credentials、provider 单元测试，provider `endpoint_binding` 及 CLI `keyring_bootstrap` 通过。持久测试使用合成密钥环。 |
| C（远程网关） | 实际绑定到非回环地址时强制令牌和 TLS。CLI 在创建数据目录等副作用之前预检。源码提交 `c679c8393`。 | `cargo test -p apeireth-gateway --test tls_serve -p apeireth-cli --test remote_access_e2e --locked -j 2` |
| E（模型出境） | 生产 CLI 的 ProviderRouter 默认脱敏消息文本和历史工具参数，单请求登记簿仅还原该请求的最终回复。默认不写审计账本，无法检查的图像请求被拒绝。`0` 或 `false` 全部关闭该出网闸。 | `cargo test -p apeireth-runtime -p apeireth-runtime-assembly --lib --locked -j 2` |
| E（会话封缄） | `ade8a8f73` 在显式启用时事务升级会话与已提交操作证据。密钥探针和 SQLite 写约束阻止无钥降级，行身份参与密文认证。 | `sqlite_session`、`session_seal`、`session_operation_evidence`、`crypto_keys` 专项通过。使用合成数据库与密钥。 |
| D（根锁） | 主干已包含 `34d793ab9` 和 `60d30bfcf` 的依赖修复。严格审计按两条维护状态豁免通过。提交 `bb2499dea` 将四处豁免清单对齐，并在 CI 检查漂移。 | `cargo audit --deny warnings --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2026-0543 --file Cargo.lock` |
| D（历史公开桌面锁） | `1a42be2c2` 更新历史公开 companion 独立锁的 SQLite 和 QUIC UDP 依赖。两项已知漏洞消除，正式锁扫描漏洞计数为零。 | 隔离副本 `--offline --locked --all-targets` 构建检查、171 项测试通过，3 项既有忽略。Windows Clippy 否决代码告警，按次排除 MSVC 链接信息。 |
| D（仓库凭据） | 当前树移除已知凭据值及相关豁免，扫描通过。流式扫描 `1a42be2c2` 的 28,176 个可达 blob，无超限跳过，历史扫描留下 31 条待核候选，记录仅保留脱敏定位及规则。 | `pwsh scripts/secret-scan.ps1 -Mode scan-all`；`pwsh scripts/secret-scan.ps1 -Mode scan-history -HistoryRef HEAD`；`python -m unittest scripts.test_secret_scan` |
| D（发布） | Windows 发行锁审计和 Authenticode 验签进入阻断条件。`9bc9b7c5f` 固定签名工具并验证 Sigstore bundle，Scoop 清单绑定同一作业 ZIP。`3e3157009` 打通独立公钥审查。`2869c46c6` 将五个发行及审计流程的 80 处外部 Actions 引用固定为官方提交 SHA。 | 工作流语法、签名与发布夹具、工具链映射正反例、67 项守卫登记通过。真实 Actions 待远端验证。 |
| 桌面宿主 | 私有独立工作树提交 `ee43dddb`。本机桥接校验当前宿主实例的请求凭据，入口内联脚本按响应绑定 CSP nonce。Windows 真实 WebView2 正常启动，外部本机客户端缺凭据时拒绝。 | 宿主单元测试和私有仓 `evidence/native_bridge_security_check.py`。使用一次性档案及合成网关。 |

本机严格 Clippy 已覆盖 tools、runtime、runtime-assembly、gateway、CLI、provider 的 all-targets。Windows 全仓逐包格式检查通过。治理链和受管模型连接专项测试通过。
反向验伪用例在本轮改动前后的输出摘录：

```text
test egress::tests::allowlist_rejects_mixed_public_and_special_addresses ... FAILED
test egress::tests::allowlist_rejects_mixed_public_and_special_addresses ... ok
test desktop::egress_gate::tests::unscannable_image_content_is_denied_without_recording_the_url ... FAILED
test desktop::egress_gate::tests::unscannable_image_content_is_denied_without_recording_the_url ... ok
test desktop::egress_gate::tests::nested_tool_arguments_are_redacted_without_changing_their_shape ... FAILED
test desktop::egress_gate::tests::nested_tool_arguments_are_redacted_without_changing_their_shape ... ok
test partial_forget_result_keeps_the_account_and_retry_succeeds ... FAILED
test partial_forget_result_keeps_the_account_and_retry_succeeds ... ok
test desktop::sqlite_session::tests::preexisting_plain_handle_cannot_downgrade_after_protection_commits ... FAILED
test desktop::sqlite_session::tests::preexisting_plain_handle_cannot_downgrade_after_protection_commits ... ok
test desktop::canonical::session_seal::tests::row_bound_ciphertext_cannot_move_between_rows ... FAILED
test desktop::canonical::session_seal::tests::row_bound_ciphertext_cannot_move_between_rows ... ok
test crypto_keys::tests::key_replacement_requires_the_observed_bytes ... FAILED
test crypto_keys::tests::key_replacement_requires_the_observed_bytes ... ok
```

后三组先替换合成测试输入以触发预期断言失败，恢复输入后通过，防护接线保持启用。

## Windows 全仓验证

2026-11-21 在 `528e58b09` 的最终 Rust 源码运行 `cargo test --workspace --all-targets --locked --no-fail-fast -j 2`，602 个测试目标汇总 11,500 项通过、0 项失败、50 项既有忽略。全仓首轮发现遗留评估 API 引用，运行轮发现 CLI 空账本提示与网关显式默认端口的合同差异，分别在 `8d33105f4` 和 `372a6f838` 修复。最终复跑退出 0。

`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 在 `528e58b09` 的 Rust 源码上退出 0。Windows 逐包格式检查覆盖 35 个成员。

秘密扫描、签名、发布资产、公告策略和工具链映射共 40 项脚本测试通过。工作流 YAML、67 项守卫登记和工具链版本校验通过。

历史公开桌面与私有桌面宿主各自使用独立工作区和锁文件，以上根工作区统计不含这两组测试。两组验证记录见上表。

## 还差什么

- 本机可达历史扫描已执行，旧提交含已移除的值。无效凭据、测试值和仍有效的值需逐条核验。真实供应商吊销、Git 远端历史与镜像处置需对应权限及授权。
- 会话封缄仍需显式配置，Windows 解锁密钥材料依赖目标 ACL，自动平台密钥托管尚未接线。迁移保护 SQLite 逻辑行，旧页、WAL、备份和桌面缓存仍可能留有明文。旧二进制回退需要迁移前备份。记忆与桌面存储的默认加密、保留期限和真实数据迁移仍待验证。
- `apeireth crypto` 从 stdin 读取口令，交互终端仍会回显。该入口暂缺无回显口令输入。
- 账号删除当前以逻辑遗忘结果为准。独立域 HTTP 接线、存储介质清除、备份回收、与在途写入隔离的持久删除任务待完成。
- 未绑定的旧 OpenAI 兼容凭据保持兼容模式。第三方可变 resolver 需自行实现快照。密钥环伴随记录保证同一条记录里的值和绑定一致，跨进程管理写缺少 CAS。持久化回归使用合成后端，实际平台密钥环和响应中的部分生效口径仍需分别验收。
- 外部工具 stdio 仍以启动账号权限运行，环境收窄不提供 OS 隔离。
- 历史公开桌面独立锁的严格审计仍有 14 条告警，含 10 条维护状态、3 条 unsound 与 1 条撤回版本。主仓未添加豁免，正式打包继续阻断。隔离的私有桌面宿主补丁基于已提交前端版本，活跃前端的集成、客户端存储和发行产物仍需验收。
- Windows 正式发布需要 Authenticode 证书、签名及时间戳步骤。历史包的 Sigstore 工作流需要已复核公钥和对应的外部签名材料。Scoop 新清单已离线验证，正式 2.x 发布目前只提供 Windows 安装包，ZIP 下载入口没有上线。实际 Actions、真实产物签名和可验证构建证明尚待验收。
- 历史 UI 引擎验证构建目录按 Cargo dep-info 归属登记到旧产物收缩规则。槽位守卫复跑通过，未移动或清理该目录。安全批构建使用已登记槽位。
- 全仓 crate 体量守卫阻断。基线与 `2459a3ca0` 的八个受影响 crate 实际源文件及行数一致，超出来自本批。CLI `+548`、gateway `+434`、tools `+175`、provider `+49`、runtime `+13`、runtime-assembly `+901`、credentials `+415`（另增一个源文件）、plugin `+53`。重复测试夹具和部分兼容代码可精简，已核实的空间仍不足以完成收缩，基线保持。
- 本轮找到融合与 MIRACL 所需语料并复制到隔离验证目录，供统一门禁实跑。多平台 CI 仍依赖远端。
- 生产云端、远程及其他平台仍需相应的发布验收。
