# v2.0.0-rc.1 后端安全审计与修复 — 收工报告（Closeout）

- **日期**: 2026-09-24
- **范围**: `crates/**`（17 crate / 约 18.2 万行 Rust 后端）；`frontend/`（同事施工中）全程只读未碰
- **性质**: 只读安全审计 → 全量修复 → 三级验证（单 crate 测试 / 全工作区测试 / CI 等价 clippy 门禁）
- **纪律**: 全程未 `git add/commit/push`（遵守项目"0 主动 commit/push"硬墙）；未运行任何会干扰同事的构建外操作；未安装新依赖

---

## 1. TL;DR

| 项 | 结果 |
|---|---|
| 审计发现 | Critical×1、High×13、Medium×28、Low×约30（另 organ/perception/orchestration 补充审计 H×2、M×7、L×6） |
| 修复完成度 | **100%**（含应团队要求追加的 poison 类清零轮） |
| 代码变更 | **126 个 crates 文件 + 2 份 reports 文档，+8014/−1117 行** |
| 新增回归测试 | 约 90 个（并发写、路径逃逸、CORS 敌意源、超长响应、字符边界、审批上下文保真、配额回收、SSML 转义等） |
| 终验 | `cargo test --workspace --no-fail-fast` → **exit 0**（130 个测试二进制全绿）；`cargo clippy --workspace --all-targets --all-features -- -D warnings` → **exit 0** |
| 遗留事项 | 3 项（均附触发条件/决策归属，见 §6；无阻断项） |

**交付文件**：
1. `reports/v2-rc1-backend-security-audit-2026-09-24.md` —— 审计报告本体（发现、证据、修复建议、修复状态附录）
2. `reports/v2-rc1-backend-security-closeout-2026-09-24.md` —— 本收工报告

---

## 2. 战役过程

```
Phase 1  只读审计（并行 9 审计代理 + 主审精读）
   高危模式全仓扫描 → 安全关键文件逐行取证 → 分组深审 → Critical/High 指控逐条回读源码复核
   （含对本地 cargo registry 中 reqwest 0.12.28 redirect.rs 的依赖级取证）
        │
Phase 2  全量修复（主审 + 7 修复代理，按 crate 分区零冲突）
   A core+protocol（主审接手）│B credentials+governance │C tools │E sdk
   F memory+storage │G provider+runtime+guard+assembly │H organ+perception+orchestration
   主审自担 gateway+cli（最高产品敏感度的 H1）
        │
Phase 3  追加轮（应团队要求）
   runtime-assembly 6 处漏网 poison + 全仓扫描后 62 处生产段 poison expect 统一收敛
        │
Phase 4  三级验证 + 清尾
   修复期间发现并适配 3 处"H3 语义变更的既有测试固化"；clippy 清零 10 个修复引入的 warning；
   甄别并恢复 AppContainer 测试瞬态（确定性 SID profile 污染，非回归）
```

---

## 3. 发现与修复对照表（按审计编号）

### Critical（1/1 修复）

| ID | 发现 | 修复 |
|---|---|---|
| C1 | p2p_mesh 宣称 Noise_XX/洋葱加密，实为 hex 明文 + 长度假校验 + 无占位声明 | 字段/类型全链诚实化（`OnionMeshPacket`→`MeshPacket`、`checksum_sha256`→`payload_len_hex`、`ephemeral_pubkey_hex`→`sender_public_key_hex`），删假宣称加 `TODO(v2.1)`，peer 表加上限 256 |

### High（13/13 修复）

| ID | 发现 | 修复 |
|---|---|---|
| H1 | gateway 零认证 + permissive CORS（任意网页可驱动/批准/窃取） | 本地来源白名单谓词（tauri/localhost/127.0.0.1/::1 任意端口）+ `/v1/approvals/resolve`、`/v1/admin/config` 令牌门（`APEIRETH_GATEWAY_TOKEN`，恒定时间比较）+ 新回归测试 |
| H2 | gate.rs V2/V3 恒真门（`\|\| true`、`_ => true`） | V2 真实解析风险→洋葱层并把 `requires_ha` 交 V3 执行；V3 触门动作要求已登记人类；固化测试修正 + N7/N8 fail-closed 回归 |
| H3 | guard intent 缺失 fail-open（0.45 benign） | intent 缺失对写/删/执行/网络发送/凭据类 mismatch(0.75)→RequireApproval，纯读维持低分；3 处 CLI 固化测试按语义分层适配 |
| H4 | apply_patch 无路径包含校验（任意文件写/删） | 词法层（拒绝对绝对路径/盘符/`..`）+ 现实层（canonicalize 最近存在祖先 + `starts_with(root)`）双层校验；tmp `create_new` 独占 |
| H5 | keyring set/delete `unwrap_or_default()` 静默覆盖全部凭据 | 区分"文件不存在"与"存在但损坏/被篡改"（后者拒绝写）；`Mutex` 串行化 read-modify-write |
| H6 | WS Auth 帧 token 从不校验 | `WsFrame::validate()` + 帧字段 1 MiB 上限（M8 同批） |
| H7 | lark webhook 错误泄露预期 token / 非常量时间 / 4 步只做 1 步 | 错误只留 mismatch；恒定时间比较；非 URL 校验事件显式 NotImplemented |
| H8 | 元问题禁令两套实现，公开 trait 是弱版 | 默认实现委托 const fn，单一 source of truth |
| H9 | embeddings API key 明文 + derive(Debug) | `Option<Secret>`（Debug 自动脱敏）+ 防泄露测试 |
| H10 | fetch HTML 提取 panic + 无界递归栈溢出 | 空栈挂合成根；解析期+渲染期双 256 深度上限 |
| H11 | provider SSE 缓冲无上限（端点驱动 OOM） | 1 MiB 硬限 → BadResponse |
| H12 | 加密 memory 后端无写锁无 fsync（并发即毁文件） | `Mutex` 写锁 + 单次 write_all + `sync_data()` + 0600 + 8 线程并发回归 |
| H13 | 双流路径语义分裂（跨 session 泄漏 + tombstone 失效） | 统一行布局（写 session_id/tombstoned_at 列，读列优先 payload 兜底），删除 `IS NULL` 全匹配；7 个双向可见性/防泄漏测试 |

### Medium（28/28 修复）

凭据与密钥（M1 原子写+600窗口+竞态；M2 master.key 原子写+同目录边界诚实声明；M3 读放大修复+重复 id 拒绝+尾部截断诚实声明；M4 审计日志真 SHA-256；M5 9 个秘密容器手写脱敏 Debug+api_key 私有化；M6 vendor client 全禁重定向）；存储（M7 machine_id 拒占位 UUID+OnceLock 缓存；M8 写线程 catch_unwind+ReplyGuard；M9 session list/并发语义——H1 认证叠加缓解，存储层语义随 H13 加固；M21 迁移 BEGIN IMMEDIATE+事务内重读版本；M22 dedup LRU 幽灵条目；M23 accept_persisted Immediate 事务；M24 event id O(n)探测改随机起点+重试；M25 vector 检索 top-k 堆+错误传播；M26 busy_timeout 5000×2）；网络与工具（M12 egress allowlist IP 校验；M13 guardrail 前置守门接线；M14 ResponseCache 双维上限+LRU；M15 Windows stdin 句柄泄漏；M16 sandbox 白名单三缺陷）；管理与配置（M17 base_url https 强制（环回例外）+mask 字符边界安全；M18 SelfDisableAudit 真截断+查询上限+forbidden 不落库；M19 限流按 (capability,session)+poison 安全；M20 单轮 tool_calls 上限 16+合成结果/subloop action_id+歧义 fail-closed/畸形参数 fail-loud/SessionLocks 有界）；guard（M10 锁范围收缩+插入序 LRU；M2-preset read_only 名单补齐 15 能力；M27 审批恢复持久化并回传 security context）。

### organ/perception/orchestration 补充审计（15/15 组修复）

H1 council 超长 LLM 响应 panic（入口 chars().take(2000) 截断）；H2 replay value_preview 字节切片（char 边界截断）；M1 配额调度器（重复 task_id 拒绝/终态回收/表有界/删未实施字段/抢占 doc 诚实化）；M2 W3 挖掘窗口 ms/s 单位 bug（补 /1000，功能级修复）；M3 10 文件 mutex-poison 全家桶；M4 ambient 窗口标题截断+URL/邮箱脱敏+误判否定表；M5 value_cases 裁决依据改 MasterDecision+冲突废弃+1000 上限；M6 worktree spec.id 净化+dispatch fail-closed；M7 无界增长组（hypothesis/speech/async_context/observe/emergence 全部加上限）；L1 cron dom/dow AND 语义显式声明+移位防护；L2 whisper 构造 Result 化+Retry-After 封顶；L3 SSML 五实体转义；L4 council 全 Abstain→DeferToHuman（消除 fail-open）；L5 lineage mock 签名 doc 降级；L6 零散 5 处（ftrl 索引/universe==0/uuid 碰撞注释/goal clear-then-take+id 哈希后缀/cron mo==0 防御）。

### Low 组（全部修复）

protocol：ws_v1 帧上限、StreamBridge 有界、工具参数畸形 JSON fail-loud（raw_metadata 记录）、organ_kani 诚实化；core：sovereign token 恒定时间、VerdictCache 上限、statechart 构造/转移校验、全角门收缩（中文标点不再误判）、verify_multisig 单人身份校验；tools：filesystem/search 有界读、unrestricted 回绕、std_sub 收尸+重启窗衰减、shell 冻结 cwd TOCTOU 复核、sensitive_path 补 .npmrc/.kube/.docker、raw_arg 显式错误、rollback 边界注释；guard/assembly：fast_guard 只读判定修正、poison 收敛（ cognitive/permission_preset/runtime/capability）、审批记忆加参数哈希、dream_llm 契约注释、guard_observer 有界化、unreachable! 改 typed error；sdk：c.rs OnceLock 统一所有权契约、WireKind serde untagged、NaN 拒绝、base_url scheme、sandbox_stub! 宏修复、TTL 封顶、429 Retry-After；provider/storage：embeddings 超时（保留请求级 60s 边界说明）、KeyedLimiter 有界、InMemoryBackend None-session 语义对齐、schema 所有权迁移前列集校验（V15 收编懒建表）。

---

## 4. 验证矩阵

| 阶段 | 命令 | 结果 |
|---|---|---|
| 分区修复验证 | 每 crate `cargo check -p X` + `--all-targets` | 全过（0 warning） |
| 分区测试 | 每 crate `cargo test -p X` | 全过（core 241 / credentials 95 / governance 119 / sdk 439+42(--all-features) / memory 785+62 / storage 115+15 / tools 205+25+20+10 / provider 152 / runtime 152 / assembly 177 / guard 75 / organ 179+34 / perception 129+11 / orchestration 240+12 / gateway 47 / cli 全部） |
| H3 语义变更适配 | cli 两测试文件按"授权维度 vs intent 维度"分层重断言 + ENV_LOCK poison 安全 | 4/4、5/5 |
| 清尾 | clippy 清零 10 个修复引入 warning（const fn/let-else/cast_lossless/手工前缀） | workspace clippy 0 warning |
| **终验 1** | `cargo test --workspace --no-fail-fast` | **exit 0（130 测试二进制，0 failed，0 panic）** |
| **终验 2** | `cargo clippy --workspace --all-targets --all-features -- -D warnings`（CI 等价） | **exit 0** |
| 追加轮复验 | 同上两条在 poison 批量转换后重跑 | 均 exit 0 |

**已知测试环境备注**：tools 的 AppContainer/shell 集成测试在满负载并行或测试进程被 kill 后可能间歇性报 `CreateAppContainerProfile failed: HRESULT=0x8000ffff`（确定性 SID profile 跨运行污染，即审计 L4 的测试侧表现；隔离 3 连跑 10/10）。CI 偶遇重跑该二进制即可。

---

## 5. 交付 manifest

**文档（2 份）**
- `reports/v2-rc1-backend-security-audit-2026-09-24.md` — 审计报告（发现+证据+建议+修复状态附录+追加轮记录）
- `reports/v2-rc1-backend-security-closeout-2026-09-24.md` — 本收工报告

**代码变更（126 文件，按 crate）**

| Crate | 文件数 | Crate | 文件数 |
|---|---|---|---|
| adapters/sdk | 17 | engine/memory | 16 |
| capabilities/tools | 12 | engine/organ | 9 |
| foundation/orchestration | 13 | foundation/protocol | 11 |
| foundation/core | 8 | engine/runtime | 7 |
| engine/runtime-assembly | 7 | engine/provider | 5 |
| adapters/cli | 3 | adapters/gateway | 3 |
| engine/guard | 4 | engine/perception | 4 |
| engine/storage | 4 | foundation/credentials | 2 |
| foundation/governance | 1 | | |

完整逐文件清单见 §8；`git status --short -- crates/` 可复现（未 staged、未 commit）。

---

## 6. 遗留事项（均不阻断，附触发条件与归属）

| # | 事项 | 性质 | 触发条件 / 建议 |
|---|---|---|---|
| 1 | AppContainer 按 run 随机 profile 名（审计 L4 长期方案） | 可选加固 | 代价=每次 run 重新授 ACE（累积）；当前文档化 + 测试重跑即可 |
| 2 | `RuntimeEvent::ApprovalRequired` 增加 action_id（guard_observer 标签对齐） | 数据质量（数据集标签），非安全旁路 | 根因（subloop 不绑 action_id）已修；需跨 crate 事件 schema 变更，建议单独立项 |
| 3 | intent 缺失时 Unknown+external_effect 能力是否升级审批 | 产品语义决策 | 做了会误伤后台 benign 未知工具派发；建议 Mavis/主人拍板 |
| 4 | memory/keyring 多进程文件锁（fs2） | 条件性必须 | 仅当出现多进程共享 memory.db/keyring 目录的部署；单进程已被进程内 Mutex 覆盖 |
| 5 | 加密记录尾段截除检测（head commitment） | 条件性加固 | 仅当威胁模型包含"能物理改文件的攻击者"；当前截断即 fail-closed |

建议将 4/5 记入 ROADMAP（附触发条件），1–3 走决策流。

---

## 7. 给团队的复核与合入指南

```bash
# 1) 全量验证（约 3-5 分钟，本机实测通过）
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 2) 重点区域单测（修复回归最密处）
cargo test -p apeireth-tools-canonical --test shell_execution   # 若偶发 AppContainer 失败见 §4 备注
cargo test -p apeireth-gateway                                    # 含 CORS 敌意源回归
cargo test -p apeireth-core --test integration_v1v2v3             # 含 N7/N8 AND 门回归

# 3) 合入前检查（未 commit，交接团队review）
git status --short -- crates/     # 126 文件
git diff --stat                   # +8014/-1117
```

**行为变更提示（影响调用方/部署方）**：
- gateway：permissive CORS → 本地来源白名单；`/v1/admin/config` 与 `/v1/approvals/resolve` 在设置 `APEIRETH_GATEWAY_TOKEN` 后需 Bearer 令牌；admin `base_url` 远端强制 https（环回除外）。
- 治理语义：无 turn intent 的写/删/执行/网络派发现在 RequireApproval（不再是静默 Allow）；council 全 Abstain → DeferToHuman。
- 凭据：重复 episode id 明确拒绝；损坏的 keyring 数据文件拒绝写入（不再静默覆盖）。
- API：`WhisperHttpBackend::new/openai/minimax` 返回 `Result`；`CognitiveQuota::new` 3 参；`submit_task` 返回 `Result`——已确认无生产调用方破坏（workspace check 通过）。

---

## 8. 完整变更文件清单

<details>
<summary>展开 126 个 crates 文件 + 2 份文档（点击展开）</summary>

**adapters/cli (3)**: `src/gateway_panels.rs`、`tests/local_read_tools_knob.rs`、`tests/production_governance.rs`
**adapters/gateway (3)**: `src/admin.rs`、`src/canonical_entry.rs`、`tests/canonical_openai_compatible_entry.rs`
**adapters/sdk (17)**: `apeireth_sdk.h`、`Cargo.toml`、`src/c.rs`、`src/client.rs`、`src/lark/auth.rs`、`src/lark/webhook.rs`、`src/livekit/auth.rs`、`src/livekit/error.rs`、`src/sandbox/isolation.rs`、`src/sandbox/mod.rs`、`src/sandbox/policy.rs`、`src/sandbox/resource.rs`、`src/voice/auth.rs`、`src/wire.rs`、`tests/multilang_ffi.rs`、`tests/smoke.rs`、`tests/test_sdk_client.rs`
**capabilities/tools (12)**: `src/apply_patch.rs`、`src/egress.rs`、`src/fetch/accessibility.rs`、`src/fetch/response_cache.rs`、`src/filesystem.rs`、`src/process/mod.rs`、`src/process/windows.rs`、`src/search.rs`、`src/sensitive_path.rs`、`src/shell.rs`、`src/std_sub_supervisor.rs`、`tests/process_executor.rs`
**engine/guard (4)**: `src/fast_guard.rs`、`src/hook.rs`、`src/intent.rs`、`tests/guard_tests.rs`
**engine/memory (16)**: `src/access_history.rs`、`src/append_only.rs`、`src/backend/file_encrypted.rs`、`src/backend/file.rs`、`src/backend/in_memory.rs`、`src/backend/sqlite.rs`、`src/coordinator.rs`、`src/dedup.rs`、`src/layered_memo/dream.rs`、`src/layered_memo/sleep_cycle.rs`、`src/lib.rs`、`src/migrations.rs`、`src/milestone.rs`、`src/partner.rs`、`src/persistent_vector.rs`、`src/principles.rs`
**engine/organ (9)**: `src/causal_world_model_edges.rs`、`src/causal_world_model.rs`、`src/curiosity.rs`、`src/emergence.rs`、`src/emotion_memory.rs`、`src/goal.rs`、`src/hypothesis.rs`、`src/memory.rs`、`src/value_cases.rs`
**engine/perception (4)**: `src/observe.rs`、`src/voice/emotion_voice.rs`、`src/voice/whisper_http.rs`、`tests/perception_integration.rs`
**engine/provider (5)**: `src/canonical_anthropic.rs`、`src/canonical_minimax.rs`、`src/canonical_openai_compatible.rs`、`src/embeddings.rs`、`src/openai_chat.rs`
**engine/runtime (7)**: `src/canonical/approval.rs`、`src/canonical/capability.rs`、`src/canonical/execute.rs`、`src/canonical/runtime.rs`、`src/canonical/subloop.rs`、`src/canonical/trace.rs`、`tests/canonical_approval_lifecycle.rs`
**engine/runtime-assembly (7)**: `src/canonical/cognitive.rs`、`src/canonical/dream_llm.rs`、`src/canonical/guard_observer.rs`、`src/canonical/organ_module.rs`、`src/canonical/permission_preset.rs`、`src/canonical/preference_learning.rs`、`src/canonical/tool_modules.rs`
**engine/storage (4)**: `src/machine_id.rs`、`src/pool.rs`、`src/rate_limit/mod.rs`、`tests/storage_foundation.rs`
**foundation/core (8)**: `src/clock.rs`、`src/gate.rs`、`src/lib.rs`、`src/onion.rs`、`src/organ_kani_proofs.rs`、`src/philosophy.rs`、`src/statechart.rs`、`tests/integration_v1v2v3.rs`
**foundation/credentials (2)**: `src/keyring.rs`、`src/store.rs`
**foundation/governance (1)**: `src/rate_limit.rs`
**foundation/orchestration (13)**: `src/ambient_context.rs`、`src/async_context.rs`、`src/cognitive_quota_scheduler.rs`、`src/continuation.rs`、`src/council/advisors_llm.rs`、`src/cron.rs`、`src/durable/replay.rs`、`src/lib.rs`、`src/lineage_spawning.rs`、`src/research_context_policy.rs`、`src/research_vault_ftrl.rs`、`src/speech_arbiter.rs`、`src/worktree_sandbox.rs`
**foundation/protocol (11)**: `src/adapters/anthropic_messages.rs`、`src/adapters/gemini.rs`、`src/adapters/openai_chat.rs`、`src/adapters/openai_responses.rs`、`src/bridge_ext.rs`、`src/lib.rs`、`src/normalized.rs`、`src/organ_kani_proofs.rs`、`src/p2p_mesh.rs`、`src/ws_v1.rs`、`tests/wire_format_ext.rs`
**reports (2)**: `reports/v2-rc1-backend-security-audit-2026-09-24.md`、`reports/v2-rc1-backend-security-closeout-2026-09-24.md`（本文件）

</details>

---

*收工。审计与修复全程只读起步、最小改动、可复现验证；未 commit/push，等团队 review 后按项目整合流程合入。*
