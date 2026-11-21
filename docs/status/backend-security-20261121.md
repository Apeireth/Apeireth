# 后端安全与隐私升级进度

[Document-Meta]
Document: docs/status/backend-security-20261121.md
Class: STATUS
Version: 0.1
Status: 活跃
Last-Modified: 2026-11-21
Author: Codex

## 基线与批次

以 `2459a3ca0` 为基线，在 `fix/backend-security-20261121` 开工。批次及验收见[设计稿](../design/backend-security-20261121.md)。

| 批次 | 当前状态 | 验证入口 |
|---|---|---|
| A | 工具域名解析须全部通过公网判定。MCP HTTP/SSE 禁止代理和自动跳转，SSE 消息端点保持同源，stdio 清理环境。源码提交 `3431a5ff1`，移除孤儿辅助入口 `7c2b6121e` 后复跑通过。 | `cargo test -p apeireth-tools-canonical --lib --test mcp_bridge_e2e --locked -j 2` |
| B | HTTP 与 CLI 共用删户预检和结果确认。目标主体、域或五账结果无法确认时保留账号。HTTP 暂拒独立用户域遗忘，CLI 仅用可确定的目标域数据库。源码提交 `9574f0145`。 | `cargo test -p apeireth-gateway --test user_admin_endpoints --locked -j 2`；`cargo test -p apeireth-cli --test user_admin --test user_admin_dual_entry --locked -j 2` |
| C | 普通网关按端点来源和供应商类型核对已有绑定，串行化管理更新。凭据写入端不支持绑定或持久化失败时保留旧的运行时状态。Windows 受管入口现有令牌与端点合同保持。源码提交 `af877a2f7`。 | `cargo test -p apeireth-gateway --lib --test admin_binding_warning --test admin_config --locked -j 2`；`cargo test -p apeireth-cli --lib keyring_bootstrap --locked`；`cargo test -p apeireth-provider --test endpoint_binding --locked` |
| E（模型出境） | 已接入当前生产 ProviderRouter 路径。启用时拒绝无法检查的图像，脱敏历史工具参数的字符串值，并以单次请求登记簿还原最终回复。默认关闭。源码提交 `146aec08a`。 | `cargo test -p apeireth-runtime -p apeireth-runtime-assembly --lib --locked -j 2` |
| D（根锁） | 主干已包含 `34d793ab9` 和 `60d30bfcf`，分别处理此前发现的传输和系统库问题。2026-11-21 在线刷新后按仓库现有告警政策审计退出 0。 | `cargo audit --deny warnings --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2026-0543 --file Cargo.lock` |
| D（仓库凭据） | 移除当前树中的凭据值及其豁免。历史文本加入扫描，当前合成夹具按内容与位置校验。本地提交 `ce8952632`，当前树和合成历史用例通过。 | `pwsh scripts/secret-scan.ps1 -Mode scan-all`；`python -m unittest discover -s scripts -p test_secret_scan.py` |
| D（发布） | 发行锁审计、签名和产物完整性进入阻断条件。签名输入核对来源运行、tag 与主干谱系。工作流输入先按数据校验，公钥更新通过审查产物交付。源码提交 `7990101e8`。 | `python scripts/check_yaml_duplicate_keys.py`；`python -m unittest scripts.test_cosign_release scripts.test_verify_release_assets` |

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
```

## 还差什么

- 供应商凭据吊销和远端历史处置需要单独确认。
- 已定点核实旧提交仍包含本批移除的值。全历史扫描未跑完。PR 仍保留当前树与全历史门禁，旧提交需要单独处置。
- 存量会话、记忆和桌面数据的默认加密、保留期限及可恢复迁移待验证。
- 账号删除当前以逻辑遗忘结果为准。独立域 HTTP 接线、存储介质清除、备份回收、与在途写入隔离的持久删除任务待完成。
- 普通网关的凭据绑定元数据仍只存在进程内。管理并发已串行化，模型请求读取凭据和绑定仍需统一快照；凭据保存成功而供应商热更新失败依旧可能产生部分生效回执。
- 外部工具 stdio 仍以启动账号权限运行，环境收窄不提供 OS 隔离。
- 独立旧桌面壳锁仍命中 `RUSTSEC-2026-0459` 和 `RUSTSEC-2026-0538`。私有桌面两个宿主锁在相同公告库未命中 vulnerability，严格模式仍有警告。桌面宿主、客户端存储及打包由前端施工线负责。
- 真实发行签名需要有效 Authenticode 及签名链公钥配置。当前已验证本地检查和 dry-run，实际 Actions、真实产物签名及 SLSA/attestation 尚待验收。
- 本机全仓槽位守卫发现既有未登记前端构建产物目录 `C:\Apeireth\cargo-target-ui-manual-20261118`。当前安全批的 Cargo 测试在已登记 base 槽位运行。
- 全仓 crate 体量守卫仍报 CLI、gateway、tools、provider、runtime 和 runtime-assembly 超基线。维持现有基线，下一批按实际复杂度收敛。
- 融合通道检查缺语料跳过。总门禁未放行，完整深门禁和多平台 CI 未验证。
- 生产云端、远程及其他平台仍需相应的发布验收。
