# Mesh RC2 PR 收口（2026-11-11）

[Document-Meta]
Document: docs/status/mesh-rc2-closeout-2026-11-11.md
Class: STATUS
Status: 🟢 软件边界修复完成；CI 与合并状态见 PR
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
| crate 体量预算 | CI 抓到新增代码超额后，统一会话 scope 路径并复用 Mesh 错误映射；新增 unit fixture 归测试目录、保持原 unit 模块身份，未移动既有生产逻辑。Runtime `src` 为 26206/26206 行，Mesh 为 9618 行并收紧基线至 9618；文件数不增，生命周期逻辑未改。`python3 scripts/check_crate_budget.py --check` 通过。 |
| 依赖审计 | `rustix` 锁定到 `1.1.6`；按既有 workflow 策略本机审计通过，未增加 ignore。失败摘要和 SARIF 均包含 warning，真实失败 JSON 的旧实现三项反证失败、修复后通过。 |
| 当前提交证据 | 定向红/绿与范围审查已完成；全工作区测试、clippy、doctest、fmt 和声明 verify_cmd 由本地波次与当前 head 的 CI 验证。合并须同时满足本地检查与所有适用 CI 通过。 |

## 行为证据与范围复核

- 预算重构前的定向证据：`transparent_tool_routing` 16 项通过，`transparent_provider_routing` 16 项通过，父子 scope 单测与 Mesh 广告可见性回归分别通过。重构后增加 legacy owner 目录兼容回归，最终新树的定向与全量结论以本地波次及当前 head 的 CI 为准。
- 反向验伪：普通目录重名、空 scope 目录、猜名 RPC 三测旧实现失败；父子 scope 一测旧实现失败；隐藏广告一测旧实现失败；control 重名两测旧实现失败。对应修复后通过，未忽略用例或放宽断言。
- `governance_chain_drill` 原样命令在本机 4 项通过；合并后的工作流仍在 nextest 前执行演练。
- 受影响的 9 条声明仅更新范围复核与来源树，等级和默认开关未改；完整 verify_cmd 的结论由当前 head 的 Claims verify 门禁提供。
- 本页记录代码边界、可复验命令与定向证据；[PR #117](https://github.com/Apeireth/Apeireth/pull/117) 的实时 CI 和合并状态为动态裁决来源，不在此缓存运行中的结果。

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

- 合并条件：最终工作区门禁及当前 head 的所有适用 CI 通过后普通合并；实时结论见 PR。
- 未完成真实 NAT、自动传输编排、生产网关发现以及远端审批恢复；这些仍是显式阶段边界，不作为本批已交付能力。
