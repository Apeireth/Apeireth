# 实测验证台账 (Live Verification Ledger)

> **给谁看**：任何要写"X 已经验证过/还没验证"的人——写进文档、代码注释、
> commit message 之前先查这张表。**目的：别让后人反复测试已经测过的东西，
> 也别把没测过的当测过的。**
>
> **口径**：只有"当时真跑了、有可复现命令/提交记录/输出"的才上绿表。
> 挂账 = 明确未测或测不了，写明原因。本表随每次新实测更新（更新规则见 §4）。
> 更细的产品能力对账见 `docs/03-reference/absorption-2026-09.md`（RA-15 吸收批）。

## 1. 已实测（绿）——不要再重测，除非要复核

### 1.1 真模型 E2E（DeepSeek = openai-compatible，唯一实测 provider）

| # | 项 | 何时 | 证据 | 复现命令（需 key） |
|---|---|---|---|---|
| 1 | provider 能力级 + factory 级 smoke | 2026-09-08 | `crates/engine/provider/tests/openai_compatible_live.rs`（2 个 `#[ignore]` live） | `$env:OPENAI_API_KEY='…'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'; $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'; cargo test -p apeireth-provider --test openai_compatible_live -- --ignored` |
| 2 | CLI canonical 双轮对话（会话连续性 + trace/provider） | 2026-09-08 | 会话记录（commit `e2ab8213` 前后） | `apeireth chat "…" --model deepseek-v4-flash --session (uuid)`（同上 env） |
| 3 | shell 工具全闭环（提议→冻结→approve→执行→回灌→总结，真模型真调用） | 2026-09-08 | commit `edbd7694`；Trace 实证 `CapabilityDispatched/CapabilityCompleted(succeeded)` | `$env:APEIRETH_ENABLE_SHELL=1` + chat 触发工具 → `apeireth approve --session … --approval …` |
| 4 | organ W1 反事实推演 + W2 因果图（live LLM） | 2026-09-08 | `crates/engine/organ/tests/organ_live_llm.rs`（44s 全过） | `cargo test -p apeireth-organ --test organ_live_llm -- --ignored` |
| 5 | Council 7-advisor → Approved | 2026-09-08 | `crates/foundation/orchestration/tests/council_live.rs`（5.0s） | `cargo test -p apeireth-orchestration --test council_live -- --ignored` |

### 1.2 桌面 HTTP 主链路（gateway 协议 = 桌面前端实际调用面）

| # | 项 | 何时 | 证据 | 说明 |
|---|---|---|---|---|
| 6 | `/v1/chat/completions` 真模型多轮 + 会话连续性（turn2 正确回忆 turn1 事实） | 2026-09-10（新 key） | 本会话探针输出；commit `1a265600` | 桌面同款 payload：`{model, messages:[{role:'user',content}], session_id, stream:false}`；`served_by=provider.openai-compatible`；panel/sessions 持久化 |
| 7 | `stream:true` SSE 帧（init/content/final + apeireth 元数据） | 2026-09-10 | 同上 | 语义 = 整段完成后分帧（**不是 token 级**，见挂账 #5） |
| 8 | 错误路径 JSON body（`error` + `session_id`） | 2026-09-10 live + 库级测试更早 | `canonical_openai_compatible_entry.rs::the_gateway_reports_an_openai_compatible_missing_credential_as_unavailable` | 502/503 均带体；"裸 502"是早期探针读流姿势问题，不是产品缺陷 |
| 9 | `/v1/models` 按 id 去重（`minimax-m3` 2→1，`minimax-m3-thinking` 保留） | 2026-09-10 | live 实测 + `the_gateway_models_list_dedupes_ids_shared_across_providers` | 根因：anthropic 插件默认端点即 MiniMax Anthropic 兼容网关 |

### 1.3 装机 / 卸载 / 配置注入（Windows NSIS）

| # | 项 | 何时 | 证据 | 复现命令 |
|---|---|---|---|---|
| 10 | 装机 E2E 全链 11/11（安装→装机侧车真聊天→gateway /health→桌面冒烟→孤儿复现→卸载零残留） | 2026-09-10（新包 `CB318756…`） | `frontend/companion-desktop/scripts/install-e2e.ps1` | `$env:OPENAI_API_KEY='…'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'; $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'; pwsh frontend/companion-desktop/scripts/install-e2e.ps1` |
| 11 | NSIS 卸载器侧车进程检查（运行中侧车被卸载器杀掉，零残留） | 2026-09-08 | `installer.nsh` hook + install-e2e 孤儿场景回归 | 同 #10 |
| 12 | Settings provider 配置注入侧车环境（apply → 重启换端口 → `/v1/models` 出现注入模型 → 重复 apply 不重启） | 2026-09-10 | `frontend/companion-desktop/src-tauri/tests/supervisor_lifecycle.rs::provider_env_reaches_the_backend`（**真后端、无需 key**） | `cargo test -p companion-desktop --test supervisor_lifecycle`（src-tauri 内） |
| 13 | 高级能力旋钮注入侧车（P9：`apply_backend_config` 开 shell → 重启 → `/v1/tools/list` 出现 `"name":"shell"` 且 `permission:"granted"` → 重复 apply 不重启；合并 apply 单次重启契约） | 2026-09-10 | `supervisor_lifecycle.rs::capability_env_reaches_the_backend`（**真后端、无需 key**）+ `capability_env_pairs_match_canonical_knobs_and_fail_closed`（fail-closed 单元测试） | `cargo test -p companion-desktop --test supervisor_lifecycle` |
| 14 | **token 级真流式增量**（provider SSE → runtime sink → gateway 逐帧直通；审批/错误以显式终帧终止流） | 2026-09-10 | live 实测（DeepSeek，210 帧 ~23ms 逐帧到达）；`complete_streaming_forwards_incremental_deltas`（provider，mock SSE 顺序+合成同语义）；`the_gateway_streams_incremental_deltas_when_requested`（gateway，role<hel<lo<[DONE] + usage/元数据）；`openai_compatible_stream_ends_with_pending_approval_frame`（审批流式契约） | `curl -N -X POST …/v1/chat/completions -d '{"model":"deepseek-v4-flash","messages":[{"role":"user","content":"…"}],"stream":true}'`（需 key） |
| 15 | **侧车启动与 CWD 解耦**（System32 CWD 启动：侧车存活 + 存储落 `%LOCALAPPDATA%\Apeireth\data\` + /health 200） | 2026-09-28 | `supervisor_lifecycle.rs::sidecar_stores_land_in_app_data_regardless_of_cwd`（真后端、无需 key）+ install-e2e "hostile-CWD boot" 步骤（**用户真机点击流首发现的 P0 bug 的回归**） | `pwsh frontend/companion-desktop/scripts/install-e2e.ps1`（无 key 可跑，聊天探针自动 SKIP） |

### 1.4 DSH 参考采纳批（P0×5 / P1×6，2026-09-12）

> 全部为「自动化测试已证」口径；需要人工点击流验证的部分仍在 §2 挂账 #2。

| # | 项 | 证据 | 复现命令 |
|---|---|---|---|
| 16 | **网关统一错误帧契约**（`{"error":{"message","code","solution"}}`，8 码中文解决方案目录；SSE 错误帧同形状） | `crates/adapters/gateway` 单测（error_frame/error_codes）+ 4 处既有断言同步 + `tests/admin_config.rs`；**真机实证**：失效 key 的 vendor 401 经新契约上浮为可读错误（认证往返完成） | `cargo test -p apeireth-gateway` |
| 17 | **`/v1/admin/config` 无重启热配置**（model/base_url/api_key 热更新下一请求生效，非法 config 拒绝且旧值不变，GET 打码回显） | `gateway/tests/admin_config.rs` 2 个集成测试（mock server + 内存凭证 store） | `cargo test -p apeireth-gateway --test admin_config` |
| 18 | **会话级模型记忆 + 会话级权限预设三态**（read_only 拒写/执行、standard 沿用审批、full 免审批留日志；老库 JSON 自动迁移不丢数据；预设钩子已接生产 CLI 治理管线） | `runtime-assembly/tests/session_permission_preset.rs` + `sqlite_session.rs` 迁移测试 + CLI 接线 commit `e7da5809` | `cargo test -p apeireth-runtime-assembly -p apeireth-cli` |
| 19 | **审批载荷带命令文本**（`command_text` / `arguments_summary`，现有字段全保留） | `crates/engine/runtime/src/canonical/approval.rs` 序列化单测 | `cargo test -p apeireth-runtime` |
| 20 | **桌面钥匙串 IPC + 无重启应用（404/405 回退重启）+ 工作区目录**（keyring 只进系统钥匙串不落盘；绝对 env > 锚定 > 相对 env 视为无效的新 store 契约） | companion-desktop 30 单测 + `supervisor_lifecycle.rs` 8 集成测试（**真 sidecar**，含 `explicit_env_store_paths_take_priority`） | `cargo test`（src-tauri 内） |
| 21 | **前端数据层 + 6 个新组件 + SettingsView/App.svelte 集成**（审批卡显示命令文本、错误解决方案横幅、会话模型/预设选择器、工具生命周期卡、斜杠菜单、工作区选择器） | `npm run check` 0 errors（警告 5 条全为既有）+ `npm run test` 7/7 | `cd frontend/companion-desktop; npm run check; npm run test` |
| 22 | **本批新包装机回归 + 真聊天探针**（新 store 锚定契约的 hostile-CWD 启动 + 卸载钩子零残留 + 桌面冒烟 + 装机侧车真聊天） | install-e2e 17/17（2026-09-12，新 NSIS 包，新 key） | `$env:OPENAI_API_KEY='…'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'; $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'; pwsh frontend/companion-desktop/scripts/install-e2e.ps1` |
| 23 | **审批载荷旧 blob 迁移修复**（升级前持久化的审批记录缺 `command_text`/`arguments_summary` 导致整会话加载失败 → 两个字段加 `#[serde(default)]`；真机用户会话 `5f05690d` 实测修复后加载 200） | commit `f69cd68d` + 回归测试 `approval_view_serialized_before_command_text_still_deserializes` + **真实数据验证**（用户 `%LOCALAPPDATA%\Apeireth\data\sessions.sqlite3` 副本 → `GET /v1/sessions/5f05690d…/settings` 200） | `cargo test -p apeireth-runtime`；复核：gateway 指 DB 副本查 settings |

## 2. 挂账（未测 / 测不了）——不要声称已验

| # | 项 | 原因 | 若要做时的路径 |
|---|---|---|---|
| 1 | **MiniMax provider 真机** | 无 MiniMax key（用户侧无预算） | `#[ignore]` 测试齐备：`minimax_llm_factory::real_llm_call_smoke` 等，有 key 后 `cargo test -p apeireth-provider --test minimax_llm_factory -- --ignored` + env `MINIMAX_API_KEY` |
| 2 | **桌面 UI 点击流人工实测** | 🟡 核心链路已人工走通（2026-09-28 用户实测：装机→启动→设置填 key→保存→网关重启→真实流式对话出字，全程抓出 5 个真 bug：CWD 启动失败 / CORS 缺失 / 密钥弹窗不推侧车 / 保存按钮不可见 / 错误帧被吞）；**剩余人工步骤**：① 工具面板 shell 审批闭环（开 shell 旋钮→触发→批准，现审批卡已显示命令文本）② 图形卸载勾选"删除应用程序数据"验证数据目录真删 ③ 设置页钥匙串保存→重启后 key 仍在（P0-1 GUI 路径）④ 会话模型/预设选择器 + 斜杠菜单的点击流 | 清单：`frontend/companion-desktop/docs/first-run-click-through-checklist.md`；观察日志 `%LOCALAPPDATA%…/logs/apeireth-backend.log` |
| 3 | `/v1/apeireth/events` 订阅端到端（桌面 UI 里收事件） | 端点已确认是活流（探针连接保持），UI 消费未人工验证 | presence 订阅代码在 `presence.ts`；UI 验证并入 #2 |
| 4 | approvals 的 HTTP 完整闭环 | 完整闭环在 CLI 实测过（#3）；HTTP 路由只验了参数校验响应 | HTTP 闭环可并入 #2（工具触发 → 面板审批按钮） |
| 5 | macOS / Linux 打包与装机 | 仅 Windows NSIS 装机实测 | Tauri bundle 命令已有，缺真机验证环境 |
| 6 | MSI 卸载与 NSIS 对齐（侧车检查） | WiX 模板无此 hook，Windows 推荐 NSIS | 若 MSI 变主力分发，需 WiX CustomAction |
| 7 | RC-7 非文本感知（voice/screen） | 待硬件 | ROADMAP P7/P-arch-3 |

## 3. 环境口径（live 测试统一契约）

```powershell
# DeepSeek（openai-compatible，唯一实测路径）
$env:OPENAI_API_KEY='…'                 # 每次会话新给；不进 repo、不进 commit（secret-scan 守门）
$env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'
$env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'
```

- key 卫生：DeepSeek key 在聊天记录出现过后应轮换；测试脚本只从 env 读 key。
- 无需 key 的回归：装机 E2E 里除"真聊天"外全部步骤 + lifecycle 测试（#12）在 CI 无 key 环境可跑。

## 4. 更新规则

- 新实测过 → 移到 §1 并填"何时/证据/复现命令"；挂账解除 → 从 §2 删行。
- 只写"当时真跑了"的事实；探针脚本失败/输出存疑 = 不算实测，留在挂账。
- 涉及 commit 的写 commit 短 hash；涉及测试文件的写 `file::test_name`。
