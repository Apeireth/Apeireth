# Apeireth API Reference

> 对齐 canonical gateway 实际路由（2026-08-27，v2.0.0-alpha.1）。
> Base URL: `http://127.0.0.1:8080`（`apeireth gateway serve --port <port>` 可改）。

> **历史（v1 时代）**：旧 companion_serve（:8090）的 OpenAI 兼容端点、`/v1/apeireth/*` 审批队列、
> `/panel` 面板与 `<<<[TOOL_REQUEST]>>>` marker 协议均为 86-crate 时代产物，代码现位于
> `legacy/donor/apeireth-companion/examples/companion_serve.rs` 及相关 crate；其能力将按根 `ROADMAP.md` §4 恢复。

## Gateway 端点

### `GET /health`

健康检查。

### `POST /v1/chat`

伙伴主链路对话端点。认证：生产前为本地场景，见"认证与安全"。

```json
{
  "session": "optional-session-id",
  "input": "你好",
  "model": "MiniMax-M3"
}
```

### `POST /v1/chat/completions`

OpenAI Chat Completions 兼容格式。

```json
{
  "model": "MiniMax-M3",
  "messages": [{"role": "user", "content": "你好"}]
}
```

两个端点都委托 `apeireth-runtime::canonical::Runtime::execute`，不自行编排。
单轮行为：governance（completion）→ provider 路由/调用 → 工具调用则走
capability 查找 + 插件分发 → 工具结果回灌 transcript 继续，直到最终回复；
approval 以 outcome 形式返回（pending approval 可恢复），trace 随响应返回且不含原始 CoT。

## CLI

```text
apeireth session
apeireth chat "<prompt>" [--model <model>]
apeireth gateway serve --port 8080
```

## Provider 配置

| Provider | Key 环境变量 | 可选变量 |
|---|---|---|
| MiniMax | `APEIRETH_API_KEY` | `APEIRETH_API_URL` / `APEIRETH_API_MODELS` |
| Anthropic | `APEIRETH_ANTHROPIC_KEY` | `APEIRETH_ANTHROPIC_URL` / `APEIRETH_ANTHROPIC_MODELS` |
| OpenAI-compatible | `OPENAI_API_KEY` | `APEIRETH_OPENAI_URL` / `APEIRETH_OPENAI_MODELS` |

凭据每轮经 `CredentialResolver` 解析，runtime/provider 不长期持有 key。

## 工具（默认状态）

| 工具 | 默认 | 说明 |
|---|---|---|
| `tool.filesystem` / `tool.search` / `tool.repo` | ✅ 启用 | 只读 |
| `tool.shell` | ❌ 关闭 | 显式配置才启用（opt-in） |
| `tool.fetch` | ❌ 关闭 | GET-only + DNS 钉扎 + 逐跳重校验，显式配置才启用 |
| `tool.apply_patch` | ❌ 关闭 | 受控文件写入（补丁式创建/修改/删除显式声明，`APEIRETH_ENABLE_FILE_WRITE=1` 开启；每次写入默认要人工审批）。**文件写入总闸：apply_patch 与 shell 写命令同受此闸**（关 = shell 写意图拒绝即帧 `pipeline.pre_deny`，开 = shell 行为不变）。**git 写走 `tool.git_write` 分级词表**（L1 默认人批 / L2 永远人批 / L3 毁灭无工具面），仓库只读工具维持只读合同 |
| `tool.git_write` | ❌ 关闭 | git 写分级（`APEIRETH_ENABLE_GIT_WRITE=1` 开启；命令白名单词法分级，组合命令逐段取最严，执行走受控通道 + 工作区限域）。L1 可逆（add / commit / branch 建·切 / tag 本地）默认每次人批（`APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1` 可开「自动放行 L1」档）；L2 出网（push / fetch / remote）**永远人工审批**（任何会话预设不放行）；**L3 毁灭级永远无工具**（拒绝即帧：reset --hard / clean -f / 强推 / 改史 / 引用删除）|
| `tool.worktree_plan` | ❌ 关闭 | 多文件改单（只读计划拍，`APEIRETH_ENABLE_WORKTREE_PLAN=1` 开启，需 `APEIRETH_ENABLE_FILE_WRITE=1` 同开——子依赖主 + file_write，fail-closed）：任务目标 → 改单（文件列表 + 每文件动作 + 预期）+ plan_id，**零落盘零审批**；改单是 `tool.worktree_apply` 的输入合同 |
| `tool.worktree_apply` | ❌ 关闭 | 四拍受控工作树写（`APEIRETH_ENABLE_WORKTREE_APPLY=1` 开启，需计划拍旋钮 + `APEIRETH_ENABLE_FILE_WRITE=1` 同开，fail-closed）：计划→改→验证→落，改单与补丁交叉一致（漂移拒绝即帧），改/验证拍在暂存面（工作区零写入），落拍过读前观测门禁且**全有或全无**（任一失败整批回滚）。**RequireApproval 级：每次落盘都人批**（无自动放行档，审批卡载荷 = 改单摘要）；只动文件不含 git 写（提交走 `tool.git_write` 分级词表）|

## 认证与安全

- Gateway 传输层认证：本地伙伴场景；生产化令牌方案待排期（见 `SECURITY.md`）。
- 工具执行走唯一 `ProcessExecutor` 边界（Windows Job Object 等，见 `docs/01-architecture/architecture.md`）。
- 出站 HTTP 走 `crates/capabilities/tools/src/egress.rs` 策略（M2D）；fetch 走 M3A 受控实现。
- 生产 governance pipeline（PII/注入/权限策略）接线状态见根 `ROADMAP.md` §4 P0。
