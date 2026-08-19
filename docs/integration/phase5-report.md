# Phase 5 — Real Chat E2E + 最小产品闭环报告（2026-08-19）

## 1. Baseline

- 工作树基于 Fresh Integration HEAD `78c0c66a`（本地）同步远程 master
- 远程 master 有新提交 `91a6eed9`（Merge branch 'YintaTriss:master' into master，仅 CI 修复 `.github/workflows/release-1.0.0.yml` + `memory_effect_demo.rs` 格式化，**无 legacy graft**）——本地需 rebase/merge 合并
- 环境：无 MiniMax API key、无本地 LLM → **真模型 E2E 无法在本环境执行**

## 2. 实际修改文件（本轮 Phase 5）

| 文件 | 改动 |
|---|---|
| `frontend/companion-desktop/src/lib/runtime.ts` | +`HttpError`（带 status）；`streamChat`/`chatOnce`/`checkJson` 抛 HttpError；`toRuntimeError` 分类 HttpError/AbortError/TypeError |
| `frontend/companion-desktop/src/App.svelte` | 对话状态按 conversationId 原子更新（防 stale write）；`HealthState` 集成（connecting/ready/generating/error/offline）；10s 真实 health 轮询 |
| `frontend/companion-desktop/src/app.css` | `.conn-hint.offline` 红色样式 |
| `frontend/companion-desktop/src/lib/types.ts` | +`HealthState` 类型 |
| `docs/integration/runtime-bridge.md` | Phase 5 更新（错误语义/health/E2E 结果） |
| `docs/integration/native-readiness.md` | 新增 Phase 5F native audit |
| `_scripts/mock-openai-sse.mjs` | 新增 mock OpenAI SSE 上游（含故障注入 401/500/hang） |
| `_scripts/e2e-streamChat-test.mts` | 新增 E2E 测试（真实 streamChat 跑真实链路） |

## 3. Real Chat E2E 结果

**链路验证成功（真模型除外）**：`UI streamChat → apeireth-api :8080 stream_forward → mock 上游 SSE` 全链路真实工作。

- Streaming: `APEIRETH_E2E_OK` 完整到达，delta 正确累积，无重复
- Cancellation: mid-stream abort → `AbortError`，reader 关闭，无幽灵 delta
- Error-network: unreachable → `network` 分类
- Error-401: 上游 401 → `auth` 分类
- Error-500: 上游 500 → `http` 分类

**真模型 E2E：待 MiniMax key**。硬约束要求真 key 不 hardcode/commit，环境无 key，诚实标注。

## 4. Streaming / Cancel / Error / Recovery

| 场景 | 结果 |
|---|---|
| Streaming | ✅ delta 连续到达，UI 增量渲染（text-delta 事件） |
| Cancel | ✅ abort 生效，reader 关闭，无幽灵 generation |
| Error (401/500/network) | ✅ 转换为稳定 RuntimeError 语义（auth/http/network），UI 显示 error-banner |
| Recovery | ✅ error 后修正配置（saveSettings）重建 runtime，可继续聊天；health 轮询自动检测恢复 |
| 下一轮请求 | ✅ busy 正确退出，可再次发送 |

## 5. RuntimeEvent 真实支持矩阵

| 事件 | 状态 | 说明 |
|---|---|---|
| `run-start` | ✅ implemented | createAgentRuntime 发射 |
| `message-start` | ✅ implemented | 发射 |
| `text-delta` | ✅ implemented | SSE delta → UI 增量渲染 |
| `reasoning-delta` | 🔒 reserved | backend 无 reasoning 流 |
| `tool-call` | 🔒 reserved | backend 无 tool 流事件 |
| `tool-result` | 🔒 reserved | backend 无 tool 流事件 |
| `message-end` | ✅ implemented | 发射 |
| `run-error` | ✅ implemented | 错误 → RuntimeError 稳定语义 |
| `run-end` | ✅ implemented | 发射（含 aborted 标记） |

**未伪造 backend 能力**：reasoning/tool 事件类型已就绪但 backend 不暴露，UI 不渲染，符合任务要求。

## 6. Conversation lifecycle 测试

| 场景 | 状态 |
|---|---|
| new conversation | ✅ |
| multi-turn | ✅（历史传给 AgentRunRequest） |
| conversation switching | ✅ 修复：所有写操作按 conversationId 定位（原 `updateMessage` 用 activeConversation → stale write） |
| assistant streaming | ✅ |
| stop generation | ✅ |
| error retry | ✅ |
| scroll / markdown / empty / loading | ✅ 已有（MessageContent + blank-state + busy） |

**修复的缺陷**：
- `updateMessage` 用 `activeConversation`（全局 active）而非按 id → 切换会话时写错会话。已改为按 id 原子更新。
- text-delta 回调捕获 `conversations` 快照 → 切换会话时 `current` 指向新 active。已改为 `appendDelta(conversationId, ...)`。

## 7. Runtime health

- `HealthState`: `connecting/ready/generating/error/offline`
- 由真实 `checkHealth`(HTTP `/health`) + `agentRuntime.running` 驱动，非纯 timer
- 10s 后台轮询检测 backend 恢复，无需重启 app
- sidebar 显示健康状态（绿=ready/generating，红=offline）

## 8. Build/Test 摘要

- `pnpm check`: 0 errors 0 warnings
- `pnpm build`: ✓
- E2E 测试: 5/5 PASS
- `cargo check`(Tauri): ✓（Phase 5B 前验证）
- `tauri build --no-bundle`: ✓（Phase 5B 前验证）

## 9. Git Commits

| commit | 内容 |
|---|---|
| `567646fe` | Phase 5B verify real runtime chat E2E（HttpError + mock + E2E test） |

待提交：Phase 5D/5E/5F（conversation state + health + native audit 文档）。远程有并行 CI commit 需合并。

## 10. Remaining Limitations

1. **真模型 E2E 待 MiniMax key** — 环境无 key，无法验证真 provider 响应
2. **quick window 无 UI/快捷键** — Phase 6
3. **无单实例** — Phase 6
4. **无 OOBE** — Phase 6
5. **reasoning/tool 流事件 reserved** — backend 未暴露

## 11. Phase 6 推荐

1. OOBE（配置引导）
2. QuickWindow + Alt+Space 快捷键
3. single-instance
4. autostart 开关 UI
5. tray 增强（新建对话/快捷窗）
6. 通知接入

---

## 结论

**「现在 Companion Desktop 是否已经可以被称为『真实可用的 Apeireth 客户端』？」**

**部分可以，但有一个明确 blocker。**

已经成立：
- 前端→AgentRuntime→Apeireth→provider 的**完整链路真实工作**（mock 上游验证 SSE/cancel/error/recovery）
- 对话状态生命周期**正确**（修复了 stale write）
- runtime health **真实可用**
- 桌面 app **可实机启动**，托盘正常

明确 blocker：
- **真实模型响应未验证**（环境无 MiniMax key）。在拿到 key 之前，无法确认「真模型对话」这一端到端体验。当前是「链路已验证 + 上游被 mock」的状态。

所以：**不能诚实地称其为「真实可用的 Apeireth 客户端」**——它是「架构完整、链路正确、错误处理稳健、但真实模型响应未验证」的客户端。拿到 key 完成真模型 E2E 后，即可满足「真实可用」。
