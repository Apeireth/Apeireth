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
| 本地与远端同名工具 | IMPLEMENTED；真实 Runtime 的全局及会话目录剔除有歧义的普通工具名，调用继续失败关闭；定向回归通过。 |
| 远端会话权限 | IMPLEMENTED；目录和调用边界都按 CapabilityId 检查同一 dispatcher 的会话权限；父会话的收窄不能被子会话或远端路由绕过；猜名拒绝且零 RPC 回归通过。 |
| Mesh 广告可见性 | IMPLEMENTED；广告复用工具与模块契约的 model_visible 判据，Internal 或显式隐藏工具不投影；Protected 的可见性仍由原契约决定；定向回归通过。 |
| resident control 保留名 | IMPLEMENTED；远端投影排除已注册 control 的保留名，原本地拦截语义不变；两项新增回归旧实现失败、修复后通过。 |
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

- 待补最终工作区门禁及当前 head 的远端检查；全部通过后普通合并。
- 未完成真实 NAT、自动传输编排、生产网关发现以及远端审批恢复；这些仍是显式阶段边界，不作为本批已交付能力。
