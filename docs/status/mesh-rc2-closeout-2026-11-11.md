# Mesh RC2 PR 收口（2026-11-11）

[Document-Meta]
Document: docs/status/mesh-rc2-closeout-2026-11-11.md
Class: STATUS
Status: 🟡 验证中，尚未合并
Last-Modified: 2026-11-11
Author: lead

## 目标与边界

收口 [PR #117](https://github.com/Apeireth/Apeireth/pull/117)：保留 Mesh RC2 与当前主干两侧实现，修复目录可见性和会话权限收窄缺口，通过当前提交的专业 CI 与唯一合并裁决后普通合并。
目标架构与验收依据为 [Mesh 架构](../design/mesh-architecture.md) 及 [ADR-0024](../adr/0024-mesh-line-contract-registration.md)。

本批验证软件行为，不把回环网络测试写成真实 NAT 或硬件验收。真实 NAT、生产网关的自动穿透编排与中继回退仍按架构文档登记。

## 缺口与处理

| 项目 | 状态与验收条件 |
|---|---|
| 主干合并冲突 | 已保留主干 #119 的必需工作流触发修复与 Mesh 的 macOS mDNS 步骤；声明证据保留两侧复核记录，生成副本重新生成。 |
| 本地与远端同名工具 | 修复中；真实 Runtime 的全局及会话目录都不得发布有歧义的工具名，调用继续失败关闭。 |
| 远端会话权限 | 修复中；目录和调用边界都按 CapabilityId 检查同一 dispatcher 的会话权限；父会话的收窄不得被子会话或远端路由绕过。 |
| Mesh 广告可见性 | 修复中；广告复用工具与模块契约的 model_visible 判据，Internal 或显式隐藏工具不得投影。Protected 的可见性仍由原契约决定。 |
| 当前提交证据 | 待补定向红/绿、全工作区测试、clippy、fmt、文档守卫及远端 CI 结论。 |

## 可复验命令

Cargo 执行时清空 `RUSTC_WRAPPER`，并限制 `-j 2`。

```sh
cargo test -p apeireth-runtime --test transparent_tool_routing --locked -j 2
cargo test -p apeireth-mesh --test mesh_semantics --locked -j 2
cargo test --workspace --all-targets --locked --no-fail-fast -j 2
cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings
cargo test --workspace --doc --locked -j 2
cargo fmt --all -- --check
python scripts/check_yaml_duplicate_keys.py
python scripts/docs_truth.py --verify --block-stale
python scripts/harness.py verify
```

## 还差什么

完成表内行为修复、记录新断言的反向验伪与最终验证结果；推送后核对 PR 最新 head 的必需检查与所有适用检查，随后普通合并。当前页面不预支远端结论。
