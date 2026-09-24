# v2.0.0-rc.1 后端代码安全审计报告（crates/ 18 万行）

- **日期**: 2026-09-24
- **审计人**: DeepSeek Harness 会话（主审 + 6 个并行只读审计代理）
- **范围**: `crates/**` 全部 17 个 crate 的 src（约 182k 行 Rust）。**未触碰** `frontend/`（同事施工中）、`research/`、`legacy/`、`target/`。
- **方法**: 高危模式全仓扫描（unsafe/Command/SQL format!/unwrap/panic/密钥字面量）→ 安全关键文件逐行精读 → 并行子代理分组深审 → 每条 Critical/High 指控回读原始代码取证（含对本地 cargo registry 中 reqwest 0.12.28 源码的取证）。
- **约束**: 全程只读。未运行 cargo 构建/测试（避免干扰同事构建产物），未做任何源码修改。
- **严重度定义**（沿用 SECURITY.md）: Critical = RCE / L0 HA 绕过; High = 权限提升 / Self-Disable 绕过 / 密钥泄露 / 静默数据破坏; Medium = 信息泄露 / DoS; Low = 最佳实践违反。

---

## 0. 执行摘要

**未发现 Critical 级远程代码执行。** 工程质量在同类 Rust 项目中偏高：进程沙箱（Job Object/进程组）、egress 防护（DNS 钉扎 + 逐跳重验）、agent loop 终止性、能力歧义 fail-closed、凭据脱敏等关键不变量真实成立且有测试锚点；生产代码 unwrap/panic 密度实际很低（330 处，多为 Mutex-poison expect；表面统计的 2827 个 unwrap 中 3328 个位于测试模块，是统计假象）。

核心风险集中在四类：

1. **"假安全"形式的诚信型缺陷**（直击项目自身 O-5「0 装 PASS」红线）: p2p_mesh 宣称 Noise_XX/洋葱加密实为 hex 明文（C1）；V2/V3 权限门恒真（H2）；intent 缺失时对齐层 fail-open（H3）；WS auth token 从不校验（H6）；元问题禁令两套实现、公开 trait 是弱版（H8）。
2. **本地 gateway 零认证 + permissive CORS**（H1）: 任意网页可驱动 agent、**代替人类批准挂起工具调用**、热改 provider base_url 致密钥外泄、读取全部会话与记忆。
3. **凭据全链路泄露窗口**: 错误消息回显预期 webhook token（H7）、`unwrap_or_default` 静默覆盖全部凭据（H5）、审计日志存 API key 明文前缀、秘密容器 derive(Debug/Serialize)、master.key 与密文同目录、anthropic `x-api-key` 跨主机重定向转发。
4. **未接线 API 缺少纵深防御**（接线之日即成高危）: apply_patch 无路径包含校验（H4）、SDK sandbox 白名单三缺陷、guardrail 前置守门零调用、memory 加密后端无写锁。

---

## 1. Critical（1 项）

### C1. `p2p_mesh.rs`：宣称 Noise_XX / 洋葱加密，实际是 hex 明文 + 长度假校验

**位置**: `crates/foundation/protocol/src/p2p_mesh.rs:85-108`；经 `crates/foundation/protocol/src/lib.rs:86-88` 公开导出；`crates/adapters/cli/src/portable_bundle.rs:97` 有 `allow_p2p_mesh` 配置标志。

```rust
// 模块 doc 宣称 (p2p_mesh.rs:1-11):
// - **Noise Protocol Framework (Noise_XX)**: Mutual authentication and forward-secret session key exchange;
// - **Onion-Routed Multi-Hop Envelopes**: Layered ephemeral packaging preventing intermediate node snooping;

pub fn wrap_onion_packet(&mut self, target_node_id: &str, payload_bytes: &[u8], max_hops: u8) -> OnionMeshPacket {
    let payload_hex = payload_bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join("");
    let checksum = format!("chk_{:08x}", payload_bytes.len());              // 字段名 checksum_sha256
    OnionMeshPacket {
        packet_id: format!("pkt_{}_{}", self.local_node_id, payload_bytes.len()), // 同长度即碰撞
        hop_count: 0,                                                      // 永远是 0，无洋葱层
        ephemeral_pubkey_hex: self.local_pubkey_hex.clone(),                // 实为长期公钥
        encrypted_payload_hex: payload_hex,                                // 未加密，纯 hex
        checksum_sha256: checksum,                                         // 只是负载长度
    }
}
```

**问题**: ① `encrypted_payload_hex` 与加密无关，hex 解码即明文；② `checksum_sha256` 只含负载长度，两个不同内容的同长度负载 checksum 完全相同，「完整性校验」为零；③ `packet_id` 可预测、可枚举、同长度即碰撞；④ `process_roaming_delta`（:111-114）的「Merkle root 校验」只是 `len() >= 8`。模块**没有任何"这是占位"的诚实声明**，与同仓 `acp.rs:14-19`（诚实声明 SipHash 非安全边界）形成刺眼对照——正是项目 O-5 原则要禁止的最重形态。

**现状**: 无生产调用方（仅公开导出 + 测试）；属「接线即泄密」的潜伏 Critical。

**建议**: (a) 接真加密；或 (b) 立刻改名 `payload_hex` / `payload_len_hex`、删除模块 doc 中的 Noise_XX/洋葱路由宣称、加显式 `// TODO(v2.1): 真加密未实现` 占位声明。二选一，不允许维持现状。

---

## 2. High（13 项）

### H1. gateway 零认证 + `CorsLayer::permissive()`：任意网页可驱动 agent、代替人类批准、窃取数据

**位置**: `crates/adapters/gateway/src/canonical_entry.rs:489-523`（主 router）；`crates/adapters/gateway/src/panels.rs:641-677`（panel router，被 `.merge()` 进主 router）

全部端点无任何认证层：`/v1/chat`、`/v1/chat/completions`（含 SSE）、**`/v1/approvals/resolve`**（POST 即可代替主人**批准**挂起的工具调用——shell 工具的人工审批门可被远端批准）、**`/v1/admin/config`**（GET/POST 热改 provider base_url 与 api_key）、`/v1/panel/sessions|episodes|traces|audit|memory/graph`（读全部会话与记忆内容）、`/v1/sessions/:id/settings`（PATCH 改会话设置）、`/v1/memory/append`。叠加 `.layer(tower_http::cors::CorsLayer::permissive())`（canonical_entry.rs:521，注释自承"部署暴露时须替换"但无机制）：用户浏览器中任意网页可跨源调用上述全部 API（preflight 全放行）。

**触发**: gateway 运行期间用户浏览恶意网页 / 本机其他进程 / LAN 暴露时（`gateway serve --bind` 允许任意地址且无警告，cli/src/main.rs:711-717）。

**影响样例**: 恶意页面 `fetch('http://127.0.0.1:8080/v1/approvals/resolve', {method:'POST', body: ...})` 批准一次待决 shell 调用；或 POST `/v1/admin/config` 把 base_url 改为攻击者服务器，之后所有用户对话连同 API key 发往攻击者。

**建议**: 加 loopback token 或 Origin 白名单（tauri://localhost / http://tauri.localhost）；`approvals/resolve` 与 `admin/config` 至少要求本地口令；`--bind 0.0.0.0` 时强制要求认证或拒绝启动。

### H2. `gate.rs`：V2/V3 权限门是恒真空操作，「三者 AND」退化为只有 V1

**位置**: `crates/foundation/core/src/gate.rs:155-179`；固化测试 `crates/foundation/core/tests/integration_v1v2v3.rs:211`

```rust
fn check_permission(action: &Action, permission: &PermissionOnion) -> bool {
    match action.risk_level {
        RiskLevel::Critical => { if action.target == ActionTarget::ModifyL0HA && !permission.l0.requires_ha { return false; } action.target != ActionTarget::ModifyL0HA }
        RiskLevel::High   => permission.l4.requires_ha || true,  // ← 恒为 true
        RiskLevel::Medium => permission.l3.requires_ha || true,  // ← 恒为 true
        RiskLevel::Low => true, RiskLevel::Info => true,
    }
}
fn check_ha(action: &Action, ha: &HumanAuthority) -> bool {
    match ha.mode {
        HAMode::Offline => matches!(action.risk_level, RiskLevel::Low | RiskLevel::Info),
        _ => true,  // SingleHuman/MultiHuman 全部直接放行 —— "实际需要真实人类验证"从未发生
    }
}
```

`x || true` 是常量 true；`permission.l4/l3.requires_ha` 被读取后立即丢弃。叠加后 `ActionGuard::check_action` 实际只执行 V1（仅拦 11 个 hardcode `ActionTarget`，一切 `NormalAction(_)` 直接 Allow）。集成测试把「Critical + NormalAction + SingleHuman → Allow」当作「V1✓ V2✓ V3✓」正确行为固化——测试套在替空操作背书。

**建议**: 删两处 `|| true`；`check_ha` 在线分支改 fail-closed（`BlockByHumanAuthority("V3 未实现")`）或接真实 multi-sig；修正固化测试。

### H3. guard：intent 缺失时意图对齐层整体 fail-open（score=0.45 benign），高危效果被静默放行

**位置**: `crates/engine/guard/src/intent.rs:618-624`（`let Some(intent)=intent else { return AlignmentAssessment { class: Unknown, score: 0.45, reasons: vec!["turn_intent_unavailable"] } }`）；接入 `hook.rs:463`；放行路径 `fast_guard.rs:173-177` + `chain_guard.rs:114-134`；依赖 `features_v2.rs:140-196` + `classifier.rs:149`（默认 NoClassifier 令 unrequested_* 特征对决策无效，`fusion.rs:87-89`）。

intent 为 None 时所有 `unrequested_*` 检查（都需要 `intent.allows_*()`）都不执行，0.45 < 0.65/0.85 阈值 → `apply_alignment` 是 no-op。一次「干净 workspace 路径」的 `fs.delete`（无 command_head、sink=WorkspaceFile）：FastGuard → `allow()`(risk 0.0)；ChainGuard 单步 delete 不满足 `has_destructive_chain`（需 delete+external 并发，chain_guard.rs:457-474）→ `Decision::Allow`。若绑定只读 intent，同一动作命中 `unrequested_delete` → HighRiskMismatch(0.95) → Deny。即：**「未绑 intent」把高危效果从 Deny 降级为 Allow**（write/delete）；egress（http.send）降为 RequireApproval 而非 Allow。

**触发面**: 无 `TurnSecurityContext` 的派发（后台任务、系统动作、cron、未来的 subloop 路径）。正常 CLI/gateway turn 有 intent（cli/src/lib.rs:967、gateway/src/canonical_entry.rs:246 经 RuleIntentInterpreter 注入；hook.rs:423 的 `turn_intents` 历史映射可跨审批恢复兜住）。

**组合放大**: runtime 审批恢复路径传 `None` 作为 security_context（`crates/engine/runtime/src/canonical/execute.rs:705-721`（Reject/Cancel 恢复）与 `:776-792`（Approve 恢复）两处 `advance(..., None, ...)`；根因 `approval.rs:110-136` 的 `FrozenTurnContinuation` 不携带 context）——恢复轮次的其他 context 消费者看到空。

**建议**: intent 缺失对 Write/Modify/Delete/Publish/NetworkSend/Execute 一律 RequireApproval（fail-closed）；`FrozenTurnContinuation` 持久化 security context 并在恢复时传回。

### H4. `apply_patch` 事务应用器无任何根目录包含校验——绝对路径/`..` 任意文件写与删

**位置**: `crates/capabilities/tools/src/apply_patch.rs:249, 260, 273`（三处 `root_dir.join(path)`，`path` 原样来自补丁文本 `PathBuf::from(stripped.trim())`，parse 于 :121/133/137）

全模块无 `canonicalize`、无 `starts_with(root)`、无 symlink 检查；`atomic_write_file`（:302-303）还 `create_dir_all(parent)`。Windows 上 `root.join("C:\\...")` 遇绝对路径直接替换 base（unix 同理 `/etc/cron.d/x`）。模型（或被 prompt 注入诱导）产出 `*** Add File: ..\..\..\Users\...\Startup\evil.bat` 即任意写；`*** Delete File: ../../<任意>` 即任意删。写入以进程身份执行（shell 场景即用户全权）。

**现状**: workspace grep 确认 crate 外零生产调用方（仅测试 + lib.rs 再导出）——**接线为 model 工具当天即成 Critical**。

**建议**: apply 入口逐 action 校验（规范根 + 目标 canonicalize + `starts_with(root)`，拒绝对绝对路径/`..`/symlink 越界）；tmp 名改 `create_new`（防同路径 symlink 预置抢占）。

### H5. keyring `EncryptedFileBackend`：`set`/`delete` 的 `load_all().unwrap_or_default()` 在数据损坏/被篡改时**静默覆盖全部凭据**

**位置**: `crates/foundation/credentials/src/keyring.rs:937`（set）、`:955`（delete）

```rust
let mut all = self.load_all().unwrap_or_default();   // 解密失败/文件损坏 → 空表
all.insert(service.to_string(), secret.expose().to_vec());
match self.save_all(&all) { ... }                    // 用 1 条凭据覆盖整个文件
```

AEAD 的篡改检测（`load_all` 解密失败返回 `Crypto` 错误）被 `unwrap_or_default` 吞掉：数据文件稍损（或攻击者故意翻转一字节），下一次 set/delete 即把用户**全部凭据**静默替换为 1 条，且不留任何 Corruption 错误——数据静默丢失 + 完整性告警被消除。

**建议**: 区分「文件不存在」（空表）与「存在但打不开/解不开」（必须报错拒绝写）；加进程内/文件级锁防并发读-改-写交叉。

### H6. WebSocket 协议层 Auth 帧的 token 从不校验——任何 token 均可开会话

**位置**: `crates/foundation/protocol/src/ws_session.rs:126-137`（`admit_auth` 只比 `ws_version`）；`crates/foundation/protocol/src/ws_v1.rs:137-279`（帧字段零校验）

```rust
fn admit_auth(&mut self, auth: &AuthFrame) -> WsFrameDecision {
    if auth.ws_version != WS_PROTOCOL_VERSION { return WsFrameDecision::Close(...); }
    self.state = WsSessionState::Open;               // token 从未与任何期望值比较
    WsFrameDecision::Deliver
}
```

`AuthFrame { token, ws_version }` 的 `token` 在协议状态机里从未被验证（测试用 `"ws-tok-abc"` 纯装饰）。duplex 传输若接线，认证门是纸糊的。另 ws_v1 帧全部字段无长度/字符集校验、无 `deny_unknown_fields`（plugin/experience.rs:203-277 有惯例，protocol 未跟随）——单帧可携 GB 级 token/chunk/args。

**建议**: `admit_auth` 增加期望 token 注入点（恒定时间比较）；帧字段加上限与格式校验。

### H7. lark webhook：校验失败错误消息泄露预期 token；非常量时间比较；4 步校验只实现 1 步

**位置**: `crates/adapters/sdk/src/lark/webhook.rs:149-159`（错误含秘）、`crates/adapters/sdk/src/lark/auth.rs:379-381`（`self.token == incoming_token`）、`crates/adapters/sdk/src/lark/webhook.rs:140-174`（4 步只做 1 步）

```rust
if !webhook_token.verify(&event.token) {
    return Err(LarkError::Other(format!(
        "webhook token mismatch: expected '{}' got '{}'", webhook_token.token, event.token)));  // 长期共享秘密进错误串
}
```

攻击者发错 token 即可触发，错误一旦被日志/回显即泄秘（正是「凭据日志泄露」类问题）；`String == String` 先比长度再逐字节早退（时序侧信道，可逐字节恢复）；文档承诺的 4 步（token/app_id/timestamp 防重放/URL 校验）实际只做 token 一步，且 `EventCallback`/`Unknown` 事件直接 `Ok(Accepted)`——未解密事件体即被放行，比显式 stub 更危险。

**建议**: 错误只含 "mismatch" 不含期望值；恒定时间比较（先对长度做常数时间填充）；stub 期对非 URL 校验事件显式 `NotImplemented`；接线时补时间窗 ±5min、app_id 比对、AES 解密。

### H8. 元问题禁令两套强弱不一致的实现，公开 trait 默认方法是弱版（可绕过）

**位置**: `crates/foundation/core/src/philosophy.rs:192-199`（trait 默认：3 条字面量子串匹配）vs `crates/foundation/core/src/lib.rs:1188-1271`（const fn：零宽/全角/同形字/emoji + 多张字面清单）

`DefaultPhilosophyGuard`（lifecycle.rs:80-87）未覆写默认方法 → `guard.is_forbidden_meta_question("如何降低安全等级")` 返回 `false`，而同一查询过 const fn 返回 `true`。公开 trait 方法是对外 API，任何消费者用它守门即被绕过（同义改写、全角、同形字、Base64 走私全部漏过）；`SelfDisableAudit::record_reflection_query`（lib.rs:1600）用的 const fn 是安全的，但 trait 默认方法没有。100+ 负向测试只覆盖 const fn。

**建议**: 删默认实现或让默认实现直接委托 `is_forbidden_meta_question_const(query)`，保持单一 source of truth；给 trait 方法补与 const fn 等价的负向测试。

### H9. `embeddings.rs`：API key 以明文 `String` 存于长生命周期结构体且 `derive(Debug)`

**位置**: `crates/engine/provider/src/embeddings.rs:40-46`（`api_key: Option<String>` + `#[derive(Debug, Clone)]`）、`:82`（env 直读）、`:98-101`（bearer_auth）

违反同 crate `credentials.rs:11-14` 自己写下的不变量（"no secret sits on a long-lived struct, no Debug print can leak it"，且有测试 `the_resolver_does_not_carry_or_print_secrets` 守护）；三个 canonical provider 都手写了脱敏 Debug（各自有 `debug_does_not_leak_secrets` 测试），唯独这个模块漏掉。CLI 组装根构造后以 `Arc<dyn EmbeddingProvider>` 长期驻留（cli/src/lib.rs:162），任一上层 `{:?}`/`debug!` 即把 `APEIRETH_EMBEDDING_KEY` 落进日志/错误面板，且无测试守这条路径。

**建议**: 改持 `Option<Secret>`（`apeireth_plugin::Secret` 的 Debug 为 `Secret(<redacted>)`）或手写脱敏 Debug；补与 credentials.rs 对称的防泄露测试。

### H10. fetch 的 HTML 无障碍树提取：畸形页面触发 `.expect` panic + `to_snapshot` 无界递归栈溢出（进程级 DoS）

**位置**: `crates/capabilities/tools/src/fetch/accessibility.rs:299-302`（panic）、`:127-149`（递归）；生产入口 `crates/capabilities/tools/src/fetch.rs:455-458`（`content_type: text/html` 即调用 `extract_tree`）

```rust
let parent_idx = stack.last().copied().expect("synthetic document root always present"); // stack 可能为空
```

闭标签多于开标签时合成根被弹出、stack 为空，下一个开标签即 panic（触发样例 `<html><body></body></html></p></p><div>x</div>`，body ≤ 默认 1 MiB）；`render_node` 是普通递归，深度 = HTML 嵌套深度，1 MiB body 内约 5 字节/层的 `<div>` 重复 ≈ 20 万层 → 撑爆 worker 栈，**栈溢出 abort 整个进程，不可 catch**。

**触发**: 模型调用 `tool.fetch` 拉取攻击者控制的公开 HTML（工具默认 public_internet_only，公网可达）。

**建议**: `stack.last()` 改条件处理（空则挂到根）；`render_node` 迭代化或加 depth 上限（如 256，超出截断并标记）；HTML 后处理整体放 `spawn_blocking`。

### H11. provider SSE 流式解析缓冲无大小上限、无空闲超时——端点驱动的内存耗尽

**位置**: `crates/engine/provider/src/canonical_openai_compatible.rs:331-351`

```rust
let mut buffer: Vec<u8> = Vec::new();
loop {
    let chunk = response.chunk().await.map_err(...)?;
    let Some(chunk) = chunk else { break };
    buffer.extend_from_slice(&chunk);                    // 无上限追加
    while let Some(end) = find_sse_frame_end(&buffer) { ... }  // 只有完整帧才 drain
}
```

`buffer` 只在遇到 `\n\n`/`\r\n\r\n` 帧边界时才被消费。恶意/被攻陷/行为异常的端点（模块文档明确支持自托管 Ollama/vLLM/网关）可持续发送**永远不含空行**的字节流：唯一边界是整请求 60s 超时，窗口内足以分配数十 GiB → OOM；慢滴漏（每 59s 发 1 字节）可挂住一个 turn。同函数对非法 JSON 的处理是正确的（`let Ok(value) = serde_json::from_str(...) else { continue }`，:362-364，不 panic——明确结案）。

**建议**: `buffer` 超阈值（如 1 MiB）即返回 `ProviderError::BadResponse`；可选加 per-read idle timeout。

### H12. `EncryptedFileBackend` 无写锁、无 fsync——并发写即损坏、崩溃即静默丢写

**位置**: `crates/engine/memory/src/backend/file_encrypted.rs:291-315`（`write_record`）

```rust
let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
f.write_all(&len_bytes)?;   // 4 字节长度前缀
f.write_all(&sealed)?;      // 记录体 —— 两次独立 write
Ok(())                      // 无 sync_all()
```

- 结构体只有 `cipher/key/dir/service`（:55-64），**没有任何锁**（同目录 FileBackend 反而有 `episode_write_lock`/`stream_write_lock` 两个 Mutex，file.rs:47-53）。trait 方法都是 `&self`，调用方无需同步即可并发调 `put_episode`，而 `encrypted_file_backend_is_send_sync` 测试（:722-724）还宣称可跨线程共享。
- framing 写是**两次独立 `write_all`**：两个线程/进程交错时长度前缀与记录体错位，切帧即失败（测试 `framing_length_tamper_fails` :673 证明 framing 敏感）。
- `next_record_index`（:318-349）是全文件扫描计数，两个并发写者会算出**同一个 index** 写入 AAD，而 `read_records` 按物理位置顺序赋 expected index → AEAD 校验失败 → **整个 .enc 文件不可读**（测试 `record_swap_fails` :646 证明顺序敏感）。
- 无 `sync_all()`：崩溃/断电时已 write 的记录可能未落盘，append-only 语义被破坏（明文 FileBackend 每次写都 `f.sync_all()`，file.rs:121）。

**触发**: 启用 EncryptedFileBackend 后多线程调用 `put_episode/append_stream`（如 coordinator 并发写），或进程崩溃。单线程顺序写不受影响。

**建议**: 结构体加 `Mutex<()>`（或按 record_type 分锁）包住 `next_record_index` + 开文件 + 两次写；写完 `f.sync_all()`（至少 `sync_data`）；多进程支持加 `fs2` 文件锁。

### H13. 两条流写入路径对同一批表语义分裂：**跨 session 数据泄漏** + tombstone 失效

**位置**: `crates/engine/memory/src/backend/sqlite.rs:244-289`（`append_stream`）、`:301-308`（`list_stream`）；`crates/engine/memory/src/append_only.rs:105-132`（`insert_entry`）；`crates/engine/memory/src/lib.rs:628-651`

同一批物理表（`thought_stream` 等 6 张）有两条写入路径，字段布局不同：
- HistoryStream trait 路径（`insert_entry`）：`session_id` 写**列**，`payload` 列 = `entry.payload` 原样 JSON。
- SqliteBackend 路径：`session_id` 列留 NULL，`payload` 列 = 复合 JSON（含 session_id/tags/tombstoned_at）。

三个后果，全部有代码证据：
1. **跨 session 数据泄漏**：trait 路径写入的行 payload 里没有 `$.session_id` 键 → `list_stream` 的 `json_extract(payload,'$.session_id') IS NULL` 兜底为真 → `SqliteBackend::list_stream(kind, 任意session, n)` 把**其它 session 的条目**也返回。
2. **tombstone 失效**：trait 路径的 tombstone 在 `tombstoned_at` 列；SqliteBackend::list_stream 只读 `payload.get("tombstoned_at")`（:323）取不到 → 已软删除的条目当活条目返回。
3. **反向不可见**：SqliteBackend 写的行 session_id 列为 NULL，`append_only::list_for_session`（append_only.rs:242-245）按列过滤一条都查不到。

**触发**: 同一 SQLite 文件同时被 `SqliteBackend`（MemoryBackend trait 新路径）和 `HistoryStream` trait（24 个子模块旧路径）访问——两者共用 migrations 建的同名表，无命名空间隔离；new/rc 混合部署期概率高。

**建议**: 统一行布局（选定 payload 顶层 schema 并同时写 session_id 列）；或给两条路径的表加前缀；`list_stream` 的 `IS NULL` 兜底改为「仅当调用方显式要求无会话归属时」。

---

## 3. Medium（28 项）

### 凭据与密钥管理

- **M1. `FileCredentialsStore` 三缺陷** — `credentials/store.rs:103-127`: ① 先 `std::fs::write` 后 `set_permissions(0o600)`，明文凭据以默认 umask 权限（通常 0644）短暂暴露的窗口；② set/delete 是无锁 load-modify-save，并发丢失更新；③ 写非原子，崩溃丢全表。建议：临时文件 0600 + fsync + rename；加文件锁。
- **M2. master.key 管理** — `credentials/keyring.rs:762-766, 1017-1028`: master.key 与密文数据同目录（一起泄漏即全裸，如备份/同步/容器镜像）；同样先写后 chmod 窗口；崩溃半写 → 密钥损坏 → 凭据全不可读。模块头对此有诚实边界声明（"靠 OS 权限"），但窗口与原子性是真缺陷。建议：独立目录/OS keyring 封存；原子写。
- **M3. file_encrypted 读放大与重复 id 语义** — `file_encrypted.rs:352-392, 404-408`: 每次 `get_episode` 全文件读取并**逐条解密全部记录**（单次 get 是 O(全库) AES 解密，大文件直接 OOM）；`put_episode` 不查重（对比 FileBackend 显式拒绝重复 id，file.rs:96-106），同 id 再写只是追加，`get_episode` 返回**文件中第一条匹配（旧值）**——更新语义静默失效。另：任一记录截断/framing 错 → 整个 `.enc` 文件永久不可读（fail-closed 但无修复路径）；尾部记录被整段截除不可检测（位置顺序编号下前面记录 AAD 均值不变，如需防投弃需 head commitment）。
- **M4. 审计日志存 API key 明文前缀** — `sdk/src/client.rs:754-761, 981-987`: 字段文档自称 "api_key 哈希…防审计日志暴露原文"，实现却只是 `chars().take(16)`——**纯明文字节前缀，零哈希**。当前 in-memory，但模块文档明确规划落盘 `~/.apeireth/audit.log`（client.rs:753），届时 16 字符密钥前缀持久化到磁盘；`preflight()` 路径同样经 `short_hash` 写入。建议：真做 `Sha256(api_key)` 截 hex 或只存非秘密标识符。
- **M5. 秘密容器系统性 derive(Debug/Serialize)** — `sdk/src/livekit/auth.rs:33,103,176`、`sdk/src/voice/auth.rs:69,143`、`sdk/src/lark/auth.rs:148,220,358`、`sdk/src/client.rs:467`（`AuthPipeline { pub api_key: String }`）: 一次 `dbg!`/`serde_json::to_string` 即把密钥/token 明文写入日志/文件，违反各模块头自述的"P0 安全铁律：0 明文存盘"。建议：Debug 手写 `[redacted]`；Serialise 在接 keyring 前移除。
- **M6. anthropic `x-api-key` 跨主机重定向转发** — `provider/src/canonical_anthropic.rs:377` 自定义头 + reqwest 0.12.28 默认重定向策略（已对本地 registry 源码取证，`reqwest-0.12.28/src/redirect.rs:239-251`：`remove_sensitive_headers` 只删固定集合 Authorization/Cookie/Proxy-Authorization/WWW-Authenticate 且**仅跨 host/port 时**）。三个 vendor client 均未设 `.redirect(Policy::none())`。vendor Messages/Chat API 从不 30x，禁重定向同时消除该面（且 base_url 可被无认证 admin 端点热改，见 H1）。建议：三个 vendor client 显式 `.redirect(reqwest::redirect::Policy::none())`。

### 存储与持久化

- **M7. `machine_id` 占位 UUID 被当真 + 探测面** — `storage/src/machine_id.rs:262-272`: `looks_like_uuid` 只查「36 字符 + 4 连字符」，`FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF`（VM/容器未填 SMBIOS 的标准占位）通过检查并被采纳——① 重启/迁移 VM 后"身份"不变但与其它未填充 VM **完全相同**（machine-id 是 credentials 的哈希盐来源），跨机碰撞；② 厂商后续补上真 UUID 时身份又变，密钥失效。Linux 路径有占位过滤（:305），Windows 没有。另：子进程无 timeout（挂起的 `wmic` 挂死调用方）、经 PATH 解析、`parse_registry_machine_guid` 依赖英文字面量（非英文 Windows 直接失败）、macOS 单探测源无 fallback、无缓存。命令与参数均为 `&'static str` 常量——**无注入面**（明确结案）。
- **M8. 连接池写线程 panic 即永久报废** — `storage/src/pool.rs:332-345`（同 memory 代理独立发现）: 写线程 `while let Some(task) = write_rx.blocking_recv() { task(&mut writer); }` 不 `catch_unwind`——任一写闭包 panic → 线程 unwind 死亡 → `write_tx` 接收端消失 → 之后**所有** write/write_sync 永远返回 `WriteQueue("writer channel is closed")`，不自动重启，且错误信息指向 channel closed 而非真正根因。`pool.write` 是公开 API，调用方闭包不可控。建议：writer 循环内 `catch_unwind(AssertUnwindSafe(...))`，panic 时向该调用者回错误后继续循环。
- **M9. `SessionStore::list()` 无 principal 过滤 + last-writer-wins** — `runtime-assembly/src/sqlite_session.rs:88-110`（`SELECT data FROM sessions` 返回**每个** Session 含完整 messages/settings，无属主过滤；存储层无 principal 概念，隔离全靠调用方）、`:71-86`（save 整对象 `ON CONFLICT DO UPDATE`，两个并发 turn 写同一 session 互相覆盖——丢失更新）。gateway `/v1/panel/sessions` 无认证暴露（叠加 H1）。建议：list 加属主维度；save 走版本乐观并发或按会话互斥。
- **M21. memory 迁移跨进程并发硬失败** — `memory/src/migrations.rs:771-782`: `migration_applied` 的 SELECT 在事务外，随后 `conn.transaction()` 是 BEGIN **DEFERRED**，执行 INSERT 时升级为写锁。两个进程同时启动：A 提交后 B 的延迟事务尝试升级 → WAL 下 `SQLITE_BUSY_SNAPSHOT`，**busy_timeout 不重试该错误** → B 启动直接报错。`storage/migrations.rs:109` 用 `BEGIN IMMEDIATE` 正确规避了此模式，memory 版没有。所幸 SQL 全部幂等（IF NOT EXISTS），最坏是启动失败而非损坏。建议：memory 迁移同样 `BEGIN IMMEDIATE` 或事务内重读版本。
- **M22. dedup LRU 回滚路径产生「幽灵条目」** — `memory/src/dedup.rs:246-250`: `inner.lru.insert(key, prev_ts)` 直接插 lru 不进 `order` 队列 → 该 key 永远不被驱逐，`lru_cap` 形同虚设。长期运行中每个「LRU 未命中但 sqlite 窗口命中」的重复 key 泄漏一个条目——高流量重复请求场景下 `lru` **无限增长**。建议：回滚时若 key 不在 lru 则走 `touch_lru`，保证 `lru`/`order` 不变式。
- **M23. `accept_persisted` 跨进程 read-check-write 竞态** — `memory/src/dedup.rs:236-258`: 同进程内 `store.conn()` 的 MutexGuard 串行化保护存在；但两个进程同时打开同一 DB 时双方都读到无 prev → 都写 → 窗口内重复条目被双双接受。生产若多进程共享 memory.db 即破坏去重语义。建议：SELECT + INSERT 包进同一 `BEGIN IMMEDIATE` 事务。
- **M24. `deterministic_event_id` O(n) 探测占用唯一写线程** — `memory/src/access_history.rs:404-423`: 相同 identity 的第 n 次记录要做 n 次 SELECT 找空闲 ordinal，而 `pool.write` 是全局单写线程——该循环**阻塞所有其他写入**（episodes、streams、commitments 全排队）。重试风暴或高频相同查询可把写吞吐压到 O(n²)。建议：计数器表或随机 UUIDv4 + `INSERT OR IGNORE` 冲突重试一次。
- **M25. `PersistentVectorIndex::search` 全量物化 + `.ok()` 静默丢行** — `memory/src/persistent_vector.rs:202-229`: 全部语料 blob 一次性读出并打分后才 `truncate(top_k)`——100 万 × 768 维 ≈ 3GB 峰值内存；`filter_map(|r| r.ok())` 把行解析失败静默跳过，搜索结果**静默缺项**。该索引默认关闭（模块文档 :13 明说 default-off），故列 Medium。建议：分页/分批；行错误向上传播。
- **M26. `SqliteMemoryStore` 未设 busy_timeout** — `memory/src/lib.rs:484-490`: rusqlite 默认 busy_timeout=0，另一进程持写锁时立即 `SQLITE_BUSY` 失败无重试；同仓 `SqliteConfig::default()` 设了 5s（pool.rs:99）——两条链路行为不一致。`persistent_vector.rs:53-78` 的裸 Connection 同样缺省。建议：`PRAGMA busy_timeout = 5000`。
- **M27. 内存/storage 两套 schema 所有权碰撞** — `memory/src/migrations.rs:653`（`episodes(id, continuity_id NOT NULL, session_id, timestamp, role, content)`）vs `storage/src/migrations.rs:37`（`episodes(id, data)`）；`agent_traces` 同样两套定义；版本账本也不一致（storage 用 `PRAGMA user_version`，memory 用 `schema_migrations` 表）。若同一 .db 文件先后被两套迁移处理，后跑的 `CREATE TABLE IF NOT EXISTS` 静默 no-op，随后 INSERT 因缺列报难懂的错。目前两链路默认文件名不同故未爆雷。另：`dedup_fingerprints`、`episode_memory_metadata` 运行时懒建表未进 migrations 列表——schema 快照与迁移列表可能漂移。建议：明确表所有权；迁移前校验 `sqlite_master` 列集（`commitments.rs:311-333` 的 `validate_schema_columns` 是好样板）。

### 网络与工具边界

- **M12. egress `ExplicitAllowList` 模式不校验解析后 IP + IPv6 内嵌盲区** — `tools/src/egress.rs:245`（`validate_resolved` 对 allowlist 直接 `Ok(())`）: allowlisted 主机名在表内但 DNS 指向 169.254.169.254（云元数据）/内网/127.0.0.1 时逐跳全过（重定向链上每跳只重查主机名不查 IP）。另 `egress.rs:108-122` 未展开 IPv4-compatible（`::7f00:1`）、IPv4-translated、NAT64（`64:ff9b::/96`，如 `64:ff9b::7f00:1` 在公网分类下可达 127.0.0.1，需网络侧 NAT64 支持）、废弃站点本地 `fec0::/10`。`PublicInternetOnly` 主路径设计正确（resolve → validate → `resolve_to_addrs` 钉扎 + 每跳重验 + https→http 降级拒绝 + userinfo 拒绝 + `no_proxy` + 响应体上限 + 总超时），未发现问题。建议：allowlist 模式也至少拒 Loopback/LinkLocal/Private/Unspecified。
- **M13. guardrail 前置守门未接线** — `tools/src/guardrail.rs:62`（`verify_path_access`）、`:116`（`verify_shell_command`）: workspace grep 两个函数仅有定义、单测、再导出——**零生产调用方**。shell 执行前无高危命令内容过滤（`rm -rf /`、`netsh advfirewall set allprofiles off` 等仅靠 AppContainer 沙箱 + 审批卡；`APEIRETH_SHELL_SANDBOX=0` 显式裸跑时无任何命令过滤）。即便接线，`verify_shell_command` 是纯子串黑名单（`rm -r -f /`、`$RM`、双空格可绕）——纵深不是沙箱的替代，但不应作为缺席的理由。建议：`build_frozen` 冻结前调用它（fail-closed）；filesystem/search 在 `resolve_contained` 前调用路径校验。
- **M14. `ResponseCache` 无条目数/总字节上限 + TTL 仅惰性回收** — `tools/src/fetch/response_cache.rs:81-88, 63-78`: `put` 无条件 insert，无 LRU；`get` 只对被访问键 eviction。模型经 fetch 循环拉取大量不同 URL → 缓存随唯一 URL 数单调增长 → 长跑进程 OOM（`with_response_cache` 是可选装配，非默认）。建议：`max_entries` + 总字节预算，超限 LRU 驱逐。
- **M15. Windows 受限 token / AppContainer spawn 泄漏 `stdin_read` 句柄** — `tools/src/process/windows.rs:656-663, 694`（`spawn_restricted_child`）、`:786-791, 825`（`spawn_appcontainered_child`）: spawn 后父进程侧 `stdin_read` 副本永不 `CloseHandle`（成功与错误路径均漏）。每次沙箱 spawn 泄漏 1 个内核句柄，高频 shell 调用下累积 → 句柄耗尽。std 路径用 `Stdio::null()` 无此问题。建议：spawn 后（含失败分支）`CloseHandle(stdin_read)`。
- **M16. SDK sandbox 白名单三缺陷（未接线，R21 接 bollard/firecracker 前必修）** — `sdk/src/sandbox/policy.rs`: ① 卷挂载白名单是纯字符串前缀（:93-115）——`/tmpevil/x`、`/tmp/../etc/passwd`、`/database/x` 均通过校验（`SandboxSdk::new()` 今天即返回 Ok）；② 特权端口常量 `PRIVILEGED_PORT_RANGE` 声明但全 crate 无使用点（:54-55 vs :146-157）——`host_port: 22` 原样通过，K-1 #5 宣称的"禁特权端口"未实施；③ "禁 root" 只匹配字面量（:37-38, :293-305）——`user = "0"`/`"0:0"`/`"Root"` 均通过（runtime 解析即 UID 0）。另 `isolation.rs:43-45` 的 capabilities "白名单"注释但 `validate()` 从不校验。建议：`Path::starts_with` 段级比较 + 归一化；补 `allow_privileged` opt-in 字段；按 UID 解析拒绝。

### HTTP/管理与配置

- **M17. `valid_base_url` 允许 `http://` + `mask_api_key` 非 ASCII panic** — `gateway/src/admin.rs:291-301`: base_url 校验接受明文 http（叠加 H1 无认证 → 攻击者可致 API key 明文外发）；`:304-308` `mask_api_key` 用字节切片 `&api_key[..3]`/`&api_key[len-3..]`，多字节 UTF-8 key（如 `"ab你你d"`）在非字符边界切片 → panic（admin config GET 的 `view()` 路径可达，axum 下该请求连接被断）。
- **M18. `SelfDisableAudit` 无截断、查询无上限** — `core/src/lib.rs:1558-1610`: 文档声称「Vec 自动截断到 1000」实际零截断（`reflection_queries`/`ota_log`/`evolution_traits` 均无界）；`query` 无长度上限，而 `is_forbidden_meta_question_const` 是 O(查询长度 × 350 模式 × 模式长度) 朴素匹配——单条超大 query 同时造成内存驻留与 CPU 放大；`register_evolution_trait`（:1613-1620）把被禁 trait 名也 push 进注册表（只计数不拒绝）。
- **M19. rate_limit 按能力而非按会话 + poison panic** — `governance/src/rate_limit.rs:174-189`: 窗口 key 是 capability（:180 `lock.entry(cap_name.to_string())`）而非文档宣称的"单会话"——一个会话吃光配额饿死其他会话（反向也可被单会话 DoS 其他会话）；`:179` `self.windows.lock().unwrap()` poison 即 panic。
- **M20. runtime 若干工程债** — `runtime/src/canonical/execute.rs:1139-1142`: 单轮 `tool_calls` 无数量上限（模型单轮塞 N 个调用即 N 倍成本与副作用面；轮次限制约束的是轮数）；`runtime/src/canonical/subloop.rs:350-358`: 派发缺 `.with_action_id(&call.id)`（依赖 action_id 的审批去重/审计关联在 subloop 上失效），`:319-329` 工具查找按 name first-match（canonical 路径同场景 fail-closed 返 None）；`provider/src/openai_chat.rs:240-244`: 模型输出畸形 arguments JSON 被静默吞为 `Value::Null`（排障困难 + 可能触发工具侧 null 缺陷）；`runtime/src/runtime.rs:158-166`: `SessionLocks` map 只增不减（长运行随会话数线性膨胀）；`runtime-assembly/src/canonical/permission_preset.rs:81-86, 145-154`: `approval_remember` 按 (session,capability) 记忆——批准一次 `tool.shell` 后同会话后续含 `rm -rf` 都跳过审批（默认 false opt-in，inner 仍可 Deny）。

---

## 4. Low（分组概要，约 30 项）

**协议与 kernel**: ws_v1 帧无字段上限/无 `deny_unknown_fields`（ws_v1.rs:137-279）；`StreamBridge` 缓冲无界（protocol/src/bridge_ext.rs:42-64）；P2P peer 表无界无老化（p2p_mesh.rs:61）；`verify_sovereign_token` 硬编码口令 `"master"` 且非常定时间（core/src/lib.rs:1393-1409，doc 自述生产应换 FIDO2/SHA-256——但当前即是可用的全局密码）；`VerdictCache` 无淘汰（philosophy.rs:211-229）；statechart 可把 current 设为不存在的状态并静默卡死（statechart.rs:161-171, 224-241）；全角门把正常中文标点（，！？：，U+FF00-FFEF 区间）判为 forbidden（lib.rs:982-999，误报白名单查询）；`verify_multisig` 单人分支不校验签名者身份（onion.rs:94-149，`"attacker:x"` 即 Accepted）；`organ_kani_proofs.rs`（core + protocol 两个 crate）是 `String::len()==4` 级填充却以 Kani proofs 命名。

**tools**: filesystem read 检查后使用竞态（metadata 后 `read_to_string` 不遵守 size 上限，filesystem.rs:165-177，search.rs:294-300 同）；`ProcessLimits::unrestricted()` + `spawn_reader` 的 `max_bytes as u64 + 1` 在 debug 构建 panic / release 回绕为 0（mod.rs:988, 394-404，当前无调用方）；`StdSubSupervisor` kill 后不 wait（Unix 僵尸）+ 重启计数无时间窗衰减（std_sub_supervisor.rs:73-87, 152-168，宣称受 ProcessExecutor 隔离实则无）；AppContainer ACE 永驻不回收 + 确定性 SID 可复用（appcontainer.rs:126-128, 296-362）；apply_patch tmp 名可预测 + 回滚 best-effort（apply_patch.rs I7）；shell 冻结 cwd 的 freeze→execute TOCTOU 窗口（分钟级审批等待期，shell.rs:198-207 vs :376-424）；sensitive_path deny-list 固有缺口（.npmrc authToken/.kube/config/.docker/config.json 不在列，由输出侧 tripwire 兜底）；`spawn_restricted_child` 静默丢弃 raw_arg（windows.rs:645-651）。

**guard/runtime-assembly**: FastGuard 只读判定用子串 `contains("read_only")` 而 canonical 只读 scope 是 `"workspace_read"` → 规则永死不触发（fast_guard.rs:137, features.rs:104）；`cognitive.rs` 15+ 处 / `permission_preset.rs:64-66` 的 `.lock().expect` poison 级联 panic（telemetry 在模块热路径）；onion 末层纵深默认关（onion_layer.rs:6）+ 未知能力归 L2；`dream_llm.rs:103` 同步 `think()` 内 `block_on` LLM 调用阻塞 Tokio worker 秒级；`guard_observer.rs:57` 用 tool_call_id 顶替 action_id（数据集标签错配）+ approvals map 无界；`evidence.rs`（governance）EvidenceGuard 任何自述非 Inference 证据即 Pass——**证据可自证/可伪造**（不在治理管线，勿作"已执行证据"可信来源）。

**SDK**: `c.rs:152-171, 177-203` version()/compile_info() 每次调用泄漏一个 CString 且头文件暗示免 free（同一 API 两套所有权契约，tests 又用 free_string）；`count_tokens_heuristic` u32 加法 >2^32 tokens（≈4GiB 文本）debug panic 跨 extern "C"（理论备忘）；`client.rs` 6 处 Mutex poison expect、每次 invoke 新建 reqwest Client（无连接池）、AuditLogger Vec 无界、`base_url` 不校验 scheme（http:// 明文传 Bearer 无告警）；`wire.rs:9-22` `WireKind::Other(String)` serde 外部标签（`{"other":"x"}`）破坏 lib.rs:210-211 明示的跨语言 "kind: string" 契约——遵循文档的 Python/Node/Go 客户端发未知 kind 时 Rust 反序列化失败；`resource.rs:83-88` `cpu_cores: f32` NaN 通过全部范围校验；`sandbox_stub!` 宏展开未导出的 `$crate::tracing`（一用即编译失败）；livekit `AccessToken::with_ttl` 无上界校验（MAX_TOKEN_TTL 形同虚设）；429 硬编码 60s 忽略服务端 Retry-After。

**provider/storage**: embeddings client 无超时 + `response.text()` 无大小上限（embeddings.rs:63-65, 93-113）；`pool.read()` 同步 `pool.get()` 在 async 上下文阻塞 worker 至 30s（pool.rs:255-261，r2d2 默认无获取超时）；runtime 10 处 RwLock poison expect（capability.rs 6 处、runtime.rs 4 处）+ `unreachable!("module stop cannot complete a turn")`（execute.rs:688，脆弱但当前成立）；`storage/src/rate_limit/mod.rs:179, 421-423` `KeyedLimiter` 每键 map 无界累积；`InMemoryBackend::list_stream` None-session 语义与其它后端分裂（in_memory.rs:130）。

---

## 5. 明确未发现问题的方面（结案清单）

1. **进程执行边界**（tools/process）: CREATE_SUSPENDED → AssignProcessToJobObject → ResumeThread 时序正确；KILL_ON_JOB_CLOSE 常驻、孙进程杀死有实测（process_executor.rs:328-350）；无 breakaway 路径（未设 CREATE_BREAKAWAY_FROM_JOB）；`quote_windows_arg`（windows.rs:931-958）与 std CommandLineToArgvW 规则一致、逐元素加引号可正确还原；能力报告（capabilities()/IsolationProfile fail-closed）诚实，沙箱不可用时拒绝裸跑（windows.rs:751-757）；Windows `raw_arg`（cmd /D /S /C）是有意设计且引号层无绕过；architecture_invariants.rs:79-89 有源码级"防 shell backdoor"守门测试。
2. **egress 主路径**（PublicInternetOnly 默认策略）: DNS 解析后校验 + `resolve_to_addrs` 钉扎（防 rebinding）+ 每跳重验 + https→http 降级拒绝 + userinfo 拒绝 + `no_proxy` + 流式响应体上限 + 总超时——设计正确，未发现绕过；`0.0.0.0`/`127.x`/`[::1]`/`::ffff:127.0.0.1`/十进制 IP 均已覆盖。
3. **filesystem 包含检查**: 先 canonicalize 后 `starts_with(root)`，顺序正确；绝对路径与 `..` 被拦。
4. **guard 决策融合**: 只升级不降级（Deny 只能来自 base）；hook 内部错误 panic（fail-closed）而非放行；permission_preset 对 session 读取失败显式 deny（permission_preset.rs:96-107）；生产装配缺后端返回 misconfigured（production.rs:584-593，boot fail-closed）；guard 无 unsafe（lib.rs:6）；guard/runtime-assembly 不用 regex crate（无 ReDoS）。
5. **凭据链路主干**: `EnvCredentialResolver` 的 Secret 脱敏、空值视为缺失、无 catch-all 映射、Debug 不泄密（有测试）；三个 canonical provider 的 Debug 均脱敏且各有测试；`FileAuditSink` 只记 name_hash（SHA-256 前 16 hex，可加盐）不记明文，IO 失败降级不枪毙。
6. **agent loop 终止性**: 每条回环路径都递增 `continuation.round`（execute.rs:878/1023/1101/1463/1475）；审批恢复不重置预算（FrozenTurnContinuation 冻结 round/module_invocations）；`max_rounds==0` build 期拒绝；模块 side-call 预算 CAS 原子 + DEFAULT_MAX_INVOCATION_DEPTH 限嵌套——模型无法绕过。capability TOCTOU 窗口微秒级且同会话 per-session 锁串行化。
7. **能力注册**: 重复注册拒绝（registry.rs:54-57, 145-153）；`find_by_name` 歧义 fail-closed；ID 在构造与 serde 双路径校验（kernel/ids.rs:127-163, 214-222）。
8. **SQL**: 全部参数化；所有 `format!` 拼 SQL 点逐一核验均为编译期常量（表名 `{table}` 类型为 `&'static str`、唯一来源 `StreamKind::table_name_ext()` 的 enum match 与 `APPEND_ONLY_TRIGGERS` 常量表；`LIMIT {limit}` 的 limit 是 `usize` 纯数字不可能含注入 payload；`PRAGMA table_info({table})` 调用方传字面量）——**无注入**。
9. **迁移事务**: storage 版 BEGIN IMMEDIATE + 失败 ROLLBACK + 版本后置提交正确（memory 版事务正确但是 DEFERRED，见 M21）；IF NOT EXISTS 幂等有测试锚定。
10. **append-only 完整性**: 6 张流表 + episodes + 5 张 V13 审计表均有 BEFORE UPDATE/DELETE trigger ABORT；软删除仅允许 tombstoned_at NULL→非 NULL 一次性转换。trigger 不防"伪造 INSERT 新行"是 append-only 日志固有取舍；尾部整段截除不可检测（需 head commitment，备注）。
11. **TLS/网络**: 全仓无 `danger_accept_invalid_certs`、无 native-tls 降级（workspace reqwest 锁 rustls-tls）；egress 不走环境代理；reqwest 0.12.28 跨主机重定向删 Authorization（自定义头缺口见 M6）。
12. **ReDoS**: governance 正则均为线性模式且 LazyLock 编译一次；guard/runtime-assembly 零 regex。
13. **FFI/PyO3/napi**: 5 个 C-ABI 函数全部先 `is_null()` 检查、非 UTF-8 fail-soft、`CString::into_raw`/`from_raw` 配对正确、无全局状态无重入危害；python.rs 无 GIL 死锁、无 panic 跨边界；node.rs 签名转换安全。
14. **生产 unwrap 密度**: 全 src 330 处 unwrap/expect/panic，逐类核对后绝大多数是 Mutex-poison expect、静态字符串 `CapabilityId::new(...).unwrap()`、构造不可达路径；各"高 unwrap 文件"（session_note 63、reflexion 52、spill 47、keyring 43、manager 58、cron 56、replay 43、whisper 31 expect、cognitive 74）经 `#[cfg(test)]` 边界核对几乎全在测试模块。
15. **provider 路由**: 永久失败不级联、瞬态失败按序 fallback、latency EMA 对时钟回退用 `unsigned_abs` 兜底（provider.rs:317-320）；`research_approval_sm.rs` 无生产路径引用（默认关闭研究模块）。
16. **nightwatch 隐私**: 落盘 finding 仅含 episode.id/session、固定核词表命中词、聚合统计，**不落 episode 正文**（nightwatch.rs:160-162, 244, 285-291）。

---

## 6. 覆盖范围与未竟事项

| 区域 | 状态 |
|---|---|
| foundation/core、foundation/protocol | ✅ 全覆盖（子代理，21 文件） |
| foundation/credentials（store/keyring/gate） | ✅ 全覆盖（主审） |
| foundation/governance（input_security/rate_limit/approval_policy/audit/colang/evidence 要点） | ✅ 主审 + 子代理补审 |
| foundation/plugin（capability/registry/manager/tool/credentials 要点） | ✅ 主审（manager 58 unwrap 核实全在测试） |
| engine/memory 存储后端层（backend/canonical/migrations/append_only/history_streams/dedup/access_history/persistent_vector/lib） | ✅ 全覆盖（子代理） |
| engine/memory 领域 store（episode/identity/persona/session/reflexion/principles/hallways/community 等约 30 文件） | ⚠️ 部分：unwrap 分布与 SQL 拼接已核验；**逐文件深审未完成** |
| engine/storage | ✅ 全覆盖（子代理） |
| engine/provider、engine/runtime | ✅ 全覆盖（子代理，38 文件） |
| engine/guard、engine/runtime-assembly | ✅ 全覆盖（子代理，20+ 文件） |
| capabilities/tools（30 文件） | ✅ 全覆盖（子代理） |
| adapters/gateway、adapters/cli | ✅ 主审深读（router/admin/panels/CLI git runner/bootstrap；子代理因并发限制失败后由主审补上） |
| adapters/sdk（37 文件） | ✅ 全覆盖（子代理） |
| engine/organ、engine/perception、foundation/orchestration（cron/durable/continuation/llm/ambient_context/quota） | ⚠️ 部分：高 unwrap 文件的生产/测试分布已核验（cron 56、replay 43、whisper 31 expect 基本全在测试）；**逐文件深审未完成**（子代理两次因 API key invalid 中途失败） |

**建议后续动作**: 对两个 ⚠️ 区域补一轮逐文件深审（重点是 organ goal 状态机、cron 时区/DST/错过执行补偿、durable replay 幂等性与崩溃一致性、quota scheduler 多 session 旁路、persona/identity 的 principal 隔离）。

---

## 7. 修复优先级路线图

**P0（功能接线前必须修，否则接线即灾难）——合计约 1.5 人日**
1. C1 p2p_mesh 去假加密或接真加密（改字段名 + doc，半天）
2. H4 apply_patch 根目录包含校验（半天）
3. H6 WS token 校验（接线 duplex 前）
4. M16 SDK sandbox 白名单三缺陷（接 bollard/firecracker 前）

**P1（一周内，消除现实攻击面）**
5. H1 gateway 认证/Origin 白名单 + admin/approvals 端点保护 + bind 0.0.0.0 告警
6. H5 keyring `unwrap_or_default` 改区分损坏/不存在 + 文件锁
7. H7 lark 错误消息去秘 + 恒定时间比较
8. H3 intent 缺失 fail-closed + H2 删两处 `|| true`
9. H9 embeddings Secret 化；H10 fetch HTML panic/深度上限；H11 SSE buffer 上限（均一小时级）
10. H12 file_encrypted 写锁 + fsync；H13 流表双路径统一（任选其一并写迁移）

**P2（一个迭代内）**
11. M1/M2 凭据文件原子写与 chmod 顺序、master.key 独立封存
12. M6 vendor client 禁重定向（一行）
13. M7 machine_id 占位过滤 + 缓存；M8 写线程 catch_unwind；M10 guard 锁范围 + LRU 驱逐；M11 usage 饱和加；M12 allowlist IP 校验；M17 base_url https-only + mask 防 panic；M21 BEGIN IMMEDIATE；M26 busy_timeout；M4 审计日志真哈希；M5 秘密容器 Debug 脱敏
14. M13 guardrail 接线或明确职责归属文档

**P3（排期）**: 其余 Medium/Low；`evidence.rs`/`organ_kani_proofs` 类"命名强于实现"的项按 O-5 原则统一正名；两个 ⚠️ 覆盖区域的补审。

---

## 8. 复核命令（供团队复验）

```bash
# unwrap/panic 生产 vs 测试分布复核（关键：多数"高危密度"是测试代码）
rg -n --glob 'crates/**/src/**/*.rs' '\.unwrap\(\)|\.expect\(|panic!\(' crates | wc -l

# SQL 拼接点（应全部为 &'static str 表名或 usize LIMIT）
rg -n 'format!\("(SELECT|INSERT|UPDATE|DELETE)' crates

# 假安全命名核查（修复后应 0 命中 onion/checksum_sha256）
rg -n 'wrap_onion_packet|checksum_sha256|api_key_hash' crates

# 恒真门（修复后应 0 命中）
rg -n '\|\| true' crates/foundation/core/src/gate.rs

# CORS 白名单 + 令牌门存在性
rg -n 'is_allowed_local_origin|require_gateway_token' crates/adapters/gateway/src
```

---

## 9. 修复状态附录（2026-09-24 修复轮）

审计交付后即启动全量修复轮：按 crate 分区并行修复（主审 + 7 个修复代理），全部改动**未触碰 frontend/、未 git commit/push**。

### 终态（全部完成）

| 区域 | 条目 | 验证 |
|---|---|---|
| foundation/core + foundation/protocol | C1/H2/H8/M11/data-URL panic/ws 帧上限/StreamBridge 上界/sovereign CT/statechart 校验/verify_multisig 单人身份/VerdictCache 上限/SelfDisableAudit 截断/全角门收缩/organ_kani 诚实化 | core 各测试二进制全绿（新增 N7/N8）；protocol 215 全绿 |
| foundation/credentials + foundation/governance | H5/M1/M2/M19 | credentials 66+29、governance 119 全绿 |
| adapters/sdk | H7/M4/M5/M16/M-7/L 组 | `--all-features` 439 lib + 42 integration 全绿 |
| engine/memory + engine/storage | H12/H13/M21–M27/M7/M8/L 组 | memory 785+62、storage 115+15 全绿 |
| adapters/gateway + cli | H1（CORS 白名单 + `APEIRETH_GATEWAY_TOKEN` 令牌门）/M17/poison + H3 连带测试适配 | gateway 47 全绿（含新回归）；cli 旋钮/治理测试适配后全绿 |
| engine/provider + runtime + runtime-assembly + guard | H3/H9/H11/M6/M10/M2/M20/M27/L 组 | 四 crate 合计 556 全绿 |
| engine/organ + perception + orchestration | H1/H2/M1–M7/L1–L6（15 组） | orchestration 240+12、organ 179+34、perception 129+11 全绿 |

**终验**：`cargo test --workspace --no-fail-fast` → exit 0，0 failed；`cargo check --workspace --all-targets` → Finished；`cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0。
新增回归测试约 90 个（并发写防护、路径逃逸、CORS 敌意源、超长响应截断、字符边界、审批恢复上下文保真、配额回收、SSML 转义等）。
变更规模：114 个 crate 源/测试文件，+7928/-1041 行；未 git commit/push。

**已知测试环境备注（非代码回归）**：`apeireth-tools-canonical` 的 AppContainer/shell 集成测试（`shell_execution` / `sandbox_acquire_is_idempotent` 等）在满负载并行运行、或测试进程被中途 kill 后，可能间歇性报 `CreateAppContainerProfile failed: HRESULT=0x8000ffff` —— 根因是确定性 SID profile（`Apeireth.Shell.Sandbox`）的跨运行状态污染（即审计 L4 "确定性 SID 可复用" 的测试侧表现），隔离运行 3 连跑均 10/10 通过。若 CI 遇到偶发失败，重跑该测试二进制即可；长期方案是按 run 随机 profile 名（审计 L4 建议项，需连带 ACE 回收设计，未在本次最小修复内实施）。

### 追加轮（poison 类清零，2026-09-24）

应团队要求补做：`runtime-assembly` 3 文件 6 处漏网 poison expect（tool_modules mcp 锁 ×3、preference_learning telemetry ×2、organ_module observations ×1），随后做全仓生产代码扫描，将 memory（file_encrypted/coordinator/dedup/sleep_cycle/principles/partner 等 13 文件）、provider（3 个 vendor capability 的 resolver 槽锁 ×12）、tools（std_sub_supervisor ×3）、cli（gateway_panels ×1）、core（clock ×3）共 **62 处**生产段 `.lock().expect(...)` 统一为 `unwrap_or_else(|poisoned| poisoned.into_inner())`。测试模块内的 fake-mutex expect（cognitive.rs 5 处、principles.rs 1 处）按既定约定保留。
**复验**：`cargo test --workspace --no-fail-fast` → exit 0（130 个测试二进制全绿）；`cargo clippy --workspace --all-targets --all-features -- -D warnings` → exit 0。

### 关键修复的设计决策（备查）

- **H2（AND 门）**：V2 改为真实解析"风险→洋葱层"并把层 `requires_ha` 语义**交给 V3 执行**；V3 对触门动作要求已登记真实人类（SingleHuman ≥1 / MultiHuman ≥ multi_sign.required），Offline 仅 Low/Info；`integration_v1v2v3.rs` 原夹具（空 real_humans）按审计结论修正为登记人类，并增补 N7/N8 fail-closed 回归。
- **H1（gateway）**：permissive CORS 收敛为本地来源白名单谓词（`tauri://localhost` / `tauri.localhost` / `localhost` / `127.0.0.1` / `::1` 任意端口）；`/v1/approvals/resolve` 与 `/v1/admin/config` 增加可选令牌门（env `APEIRETH_GATEWAY_TOKEN`，恒定时间比较，未设置时放过——浏览器向量由 CORS 白名单收敛）；`--bind` 非回环告警 CLI 已存在。
- **C1（p2p_mesh）**：`OnionMeshPacket`→`MeshPacket`、`encrypted_payload_hex`→`payload_hex`、`checksum_sha256`→`payload_len_hex`、`ephemeral_pubkey_hex`→`sender_public_key_hex`，删除 Noise_XX/洋葱/Merkle 宣称，加 `TODO(v2.1)`；`process_roaming_delta` 改为真 hex 健全性检查；peer 表加上限 256。
- **H12/H13（memory）**：加密后端加 `Mutex<()>` 写锁 + 单次 write_all + `sync_data()` + 0600；双流路径统一行布局（写 session_id/tombstoned_at 列，读列优先 payload 兜底），删除跨 session 泄漏的 `IS NULL` 全匹配。
- **H3（guard）**：intent 缺失时对写/删/执行/网络发送/凭据类操作升为 mismatch(0.75)→RequireApproval，纯读维持 0.45。

### 复验方式

```bash
cargo check --workspace --all-targets   # 全工作区编译 + 测试目标
cargo test -p apeireth-core -p apeireth-protocol -p apeireth-credentials \
  -p apeireth-governance -p apeireth-sdk -p apeireth-memory -p apeireth-storage \
  -p apeireth-gateway -p apeireth-tools-canonical -p apeireth-provider \
  -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-guard \
  -p apeireth-organ -p apeireth-perception -p apeireth-orchestration -p apeireth-cli
```

---

*审计部分由只读过程产生，未修改任何源码；修复部分按上表状态推进。全部过程未触碰同事正在施工的 frontend/ 目录，未做 git add/commit/push。*
