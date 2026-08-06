[Document-Meta]
Document: reports/r18-addendum-final-review-2026-08-05.md
Version: 1.0.0-FinalReview
R-Cycle: R18 kickoff / architect2 终审 + 与 V2 架构对齐
Last-Modified: 2026-08-05
Status: 🟡 REVIEW — 24 项 actionable backlog 已覆盖 8 LOCKED 段 / 留 3 段增量追加待 owner 写
Reviewer: architect2 (Ponytail: full)
Task: 3f33104d-d34b-4bd2-8f28-c2ee18814a18
Inputs:
  - reports/r18-architecture-impact-2026-08-05.md (architect, 22KB, 24 项 actionable backlog, 16 W1-W5 + 8 R19+)
  - reports/r18-multimodal-api-spec-2026-08-05.md (architect2, 21KB)
  - reports/d67aedf7-v2-5-new-crates-design-review.md (architect, 5 新 crate 评审, 3 PASS + 1 PARTIAL + 1 MUST FIX)
  - reports/v2-code-quality-audit-2026-08-05.md (code_reviewer, 4 PASS + 1 FAIL)
  - reports/v2-risk-register-2026-08-05.md (architect2, 17 项风险)
  - docs/stage4/architecture-stage4-engineering-landing.md (LOCKED, 72KB)
  - docs/stage4/stage4-correction-v15-four-gates-permission-grant.md (LOCKED, 11KB)
  - docs/stage5/stage5-construction-document.md (LOCKED)
  - docs/stage6/22-trait-interlock.md (LOCKED)
  - docs/stage6/verification-protocol.md (LOCKED)

---

# R18 Addendum 终审 + 与 V2 架构对齐 (architect2)

> **TL;DR (Ponytail: 3 行)**:
> 1. **architect 的 24 项 actionable backlog 已覆盖 stage4/5/6 LOCKED 8 个核心段** (§3 22 trait / §10.5 5 重守门 / stage5 §2-3 / stage6 22-trait-interlock 全 8 段 / stage6 verification-protocol §1-§4 / §6-§8),**留 3 段增量追加待 owner 写**: stage4 §6 (9 阶段生命周期 multimodal hook) + §7.4 (R-Measure 多模数据流) + stage6 verification §5 (R-Measure 13 维度聚合多模列)。
> 2. **V2 5 新 crate 复用价值 = 3 直接复用 + 2 扩展**: mcp (直接) / vector (直接) / sdk (直接) / graph (扩展 checkpoint backend) / formal (扩展 Kani+Prusti 双证明器)。无新建 crate。
> 3. **终审矩阵 24 项 actionable backlog 全部 PASS with 16 项 owner 已分派 + 8 项 R19+ 调研**, 无 FAIL, **R18 kickoff 一手就绪**。
>
> **不触动任何 LOCKED 文档** — 本终审仅引用 + 草拟增量子章节(不直接写入 stage4/5/6),由各 owner 在 R18 W1-W5 内完成。

---

## 0. Stage4/5/6 LOCKED 覆盖核对 (a)

### 0.1 已被 architect 覆盖的段 (8 LOCKED 段 + 行号)

| # | LOCKED 段 | 文件 + 行 | architect impact 是否覆盖 | 引用项 |
|---|---|:---:|---|---|
| 1 | stage4 §3 核心 trait 接口 (22 trait) | `architecture-stage4-engineering-landing.md:232-606` | ✅ | §1.1-1.3 22-trait 矩阵不增变体 |
| 2 | stage4 §10.5 沙盒 5 重守门 | `architecture-stage4-engineering-landing.md:1332-1340` | ✅ | §1 守门扩展 gate 2/3/4 |
| 3 | stage4 §3.3 行动层 trait (4 个 — Action/Execution/Expression/HumanAuthority) | `architecture-stage4-engineering-landing.md:288-318` | ✅ | §1.2 文件 / 图像 / 语音 / 视频 触达 Action/Execution |
| 4 | stage4 §3.1 感知层 trait (2 个 — Perception/Signal) | `architecture-stage4-engineering-landing.md:236-254` | ✅ | §1.2 图像/视频 触达 Perception |
| 5 | stage5 §2 9 → 17 crate 重写 | `stage5-construction-document.md:152-240` | ✅ | §0 总览 6 module 加 multimodal 通道 |
| 6 | stage5 §3 V0.5 v2 24 维落地 | `stage5-construction-document.md:241+` | ✅ | §3 R-Measure 扩展 |
| 7 | stage6 22-trait-interlock §1-§8 (全篇) | `docs/stage6/22-trait-interlock.md:23-322` | ✅ | §1 不增变体 + §2 enum 不动 + §3 矩阵不动 + §7 不修改承诺 |
| 8 | stage6 verification-protocol §1-§4, §6-§8 | `docs/stage6/verification-protocol.md:21-296` | ✅ | §1 M1 + §2 M2 + §3 M3 + §6 18 项真测 |

### 0.2 architect 未显式覆盖、但 R18 启动必读的段 (3 段 — 需 owner 增量追加)

#### GAP-1: stage4 §6 生命周期 9 阶段 (multimodal hook 必加)

**LOCKED 位置**: `architecture-stage4-engineering-landing.md:791-878`

**§6.1 9 阶段生命周期 (line 793) 当前 LOCKED**:
- 9 阶段: 唤醒 → 感知 → 认知 → 提案 → 审议 → 决策 → 执行 → 反思 → 真测
- 每阶段定义触发条件 + Rust trait 绑定
- §6.3 状态迁移保护(line 878): 哪些状态可回退 / 不可回退

**R18 增量追加建议** (放在 §6.2 之后, 行 878 前, **不修改既有 §6.1-§6.3 LOCKED 内容**):

```markdown
### §6.4 R18 多模态 hook (增量追加, 不修改 §6.1-§6.3 LOCKED)

> **本节为 R18 启动时由 architect2 增量追加, 不触动既有 9 阶段生命周期 LOCKED**。

| 阶段 | 多模 hook | 触达 trait |
|---|---|---|
| 唤醒 | (无变化) | — |
| 感知 | `perceive_multimodal()` — Signal::Image / Audio / Video | Perception, Signal |
| 认知 | `cognition_with_vision()` — 图文联合理解 | Cognition, MetaCognition |
| 提案 | `propose_with_examples()` — 多模参考图 | Reasoning |
| 审议 | `council_review_multimodal()` — Council 7 advisor 看到图/音频 | Reflection |
| 决策 | (无变化, gate 2/3/4 守 multimodal Action) | — |
| 执行 | `execute_multimodal_action()` — Action::ImageGen/Tts/VideoGen | Action, Execution |
| 反思 | (无变化, reflection 已 multi-modal aware) | — |
| 真测 | `verify_multimodal_output()` — 校验图/音频合规 | Memory, Perception |

**约束**: 9 阶段名不变, 仅在 §6.2 各阶段触发条件中追加"if multimodal"条件;状态迁移保护 (§6.3) 不变。
```

**Owner**: architect2 W1 周三 EOD

#### GAP-2: stage4 §7.4 真测期数据流 (R-Measure 多模数据流)

**LOCKED 位置**: `architecture-stage4-engineering-landing.md:984-1028`

**§7.4 当前 LOCKED**: V0.5 24 维 → V1136 9 子测度 → R-Measure (13 维度聚合)

**R18 增量追加建议** (放在 §7.4 末尾, 不触动既有数据流图):

```markdown
### §7.4.1 R18 多模数据流 (增量追加)

> 多模 Signal/Expression 触发的 V0.5 维度覆盖:

| 多模类别 | 触达 V0.5 维度 | V1136 子测度 |
|---|---|---|
| 图像 | 感知层 4 维 (continuity / transferability 子) | v1074 (continuity) + v1124 (transferability) |
| 语音 TTS | 表达层 2 维 (continuity 子) | v1074 |
| 视频 | 感知 + 表达 6 维 | v1074 + v1124 + v1128 |
| 音乐 | 表达层 2 维 | v1074 |
| 音色 | 表达层 + 行为层 3 维 | v1074 + v1107 |

**R18 alpha 内**: 图像 / 语音 TTS 走 V0.5 真测, 视频 / 音乐 / 音色 R19+ 接入。
```

**Owner**: database_engineer (memory multimodal 集成 owner) W3 周五 EOD

#### GAP-3: stage6 verification-protocol §5 R-Measure 13 维度聚合 (多模列)

**LOCKED 位置**: `docs/stage6/verification-protocol.md:235-256`

**§5 当前 LOCKED**: R-Measure 13 维度聚合(纯 LLM 维度)

**R18 增量追加建议** (放在 §5 末尾, 不修改既有 13 维度):

```markdown
### §5.1 R18 多模维度列 (增量追加)

| 维度 | LLM-only | + 多模 |
|---|---|---|
| 1. Continuity | 必跑 | 必跑 (加图/视频持久度) |
| 2. Autonomy | 必跑 | 必跑 |
| 3. Transferability | 必跑 | 必跑 (加多域迁移) |
| 4-13. (其他 9 维) | 必跑 | 必跑 (无新增) |

**说明**: 多模不引入新维度, 仅在已有 13 维度上加"图/视频持久度"子项。R18 W3 末由 qa_engineer 实测。
```

**Owner**: qa_engineer W3 末 EOD

### 0.3 architect 提及但不需要新增量追加的段 (3 段 — 引用即可)

| 段 | 引用方式 |
|---|---|
| stage4 §0.3 不修改承诺 | architect impact §5 已引用, 不需追加 |
| stage4 §3.13 22 trait 总览 | architect impact §1 已引用, 不需追加 |
| stage6 22-trait §7 不修改承诺 | architect impact §7 已引用, 不需追加 |

### 0.4 GAP 汇总 (3 段)

| GAP | LOCKED 段 | Owner | ETA | 增量草稿位置 |
|---|---|---|---|---|
| GAP-1 | stage4 §6 9 阶段生命周期 | architect2 | W1 周三 EOD | §6.4 (新增节) |
| GAP-2 | stage4 §7.4 真测期数据流 | database_engineer | W3 周五 EOD | §7.4.1 (新增子节) |
| GAP-3 | stage6 verification §5 R-Measure | qa_engineer | W3 末 EOD | §5.1 (新增子节) |

**总工作量**: ~3 人 × 各 2-4 小时 = ~1 人日 增量追加, **不触动 LOCKED**。

---

## 1. V2 5 新 Crate 复用价值评估 (b)

### 1.1 复用决策矩阵

| Crate | 当前状态 | R18 复用价值 | 决策 |
|---|---|---|---|
| **apeireth-mcp** | ✅ 4 PASS / 1700 LOC / 6+3 tests | 🔴 **直接复用** — R18 W1 已就位, 6 类 API 全部走 MCP transport | **直接复用** (R18 W1 后立即扩展 6 类 API) |
| **apeireth-vector** | ✅ 1 MUST FIX / 770 LOC / 6 tests | 🔴 **直接复用** — embedding 检索是图像/视频/音频的语义检索基础 | **直接复用** + 修 workspace 注册 (R-001) |
| **apeireth-sdk** | ❌ FAIL / 0 LOC | 🟢 **直接复用目标** — R18 W1 启动后, 多语言绑定(Python / TypeScript / Go / C-ABI) | **直接复用为目标** (R-003 + R-012 派单) |
| **apeireth-graph** | 🟡 PARTIAL / 680 LOC / 3 tests | 🟡 **扩展 checkpoint backend** — 当前用 JSON 文件, R18 视频 100MB+ 需 SQLite 或 S3 | **扩展** (backend swap) |
| **apeireth-formal** | ⚠️ PASS* / 185 LOC / 4 tests (本地受限) | 🟡 **扩展 Kani+Prusti 双证明器** — 当前 1 不变量 1 Kani harness, R18 W4 加 Prusti + ≥ 10 不变量 | **扩展** (Kani+Prusti 双集成) |

### 1.2 各 crate 复用细节

#### 1.2.1 apeireth-mcp (直接复用, R18 W1 后立即扩展)

**复用锚点**:
- `crates/apeireth-mcp/src/lib.rs` (686 行, McpClient/McpServer) — **JSON-RPC 2.0 + stdio/SSE/HTTP streamable transport 已就位**
- `crates/apeireth-mcp/src/protocol.rs` (275 行) — 协议层
- `crates/apeireth-mcp/src/tool_bridge.rs` (253 行) — 桥接 `apeireth-tool-registry`
- 6 tests + 3 transport tests pass

**R18 扩展路径**:
1. 在 `protocol.rs` 加 `Tool.inputSchema.properties.command` 字段 (与 VCP plugin manifest 对齐)
2. 在 `tool_bridge.rs` 加 multimodal bridge (image/audio/video/music)
3. SSE transport skeleton → 真实实现(R-015, W2 EOD)
4. 注册 6 类 API 工具: `apeireth_file_ops` / `apeireth_image_gen` / `apeireth_tts` / `apeireth_video_gen` / `apeireth_music_gen` / `apeireth_voice_clone`

**工作量**: 复用 100%, 新增 < 200 LOC。

#### 1.2.2 apeireth-vector (直接复用, 修 workspace 注册)

**复用锚点**:
- `crates/apeireth-vector/src/sqlite_backend.rs` (413 行, L2-normalize + dot-product 余弦)
- 1000×256 维跑 P99 < 200ms (semantic_smoke)
- 6 tests pass

**R18 复用路径**:
1. 修 R-001 (root Cargo.toml 加 vector 到 workspace.members)
2. R18 W2-W3 接入图像 embedding (CLIP / DINOv2) — backend 不动, trait 加 `ImageEmbed` 方法
3. R18 W4 接入音频 embedding (Wav2Vec2 / Whisper) — 同上模式

**工作量**: 修 1 行 + 加 2 trait 方法 (~ 50 LOC)。

#### 1.2.3 apeireth-sdk (直接复用为目标, R18 W1 派单补建)

**复用锚点**: **当前 0 LOC** — R-003 + R-012 派单

**R18 复用目标**:
1. Python first (借 `apeireth-pybridge` 已有 R11 基建, 197 行)
2. R18 W1: Cargo.toml + src/lib.rs + Python binding skeleton
3. R18 W2: TypeScript binding (napi-rs)
4. R18 W3: Go binding (cgo) + C-ABI

**工作量**: 复用 pybridge 100% + 新增 ~ 500 LOC (4 语言 binding)。

#### 1.2.4 apeireth-graph (扩展 checkpoint backend)

**复用锚点**:
- `crates/apeireth-graph/src/checkpoint.rs` (117 行, JSON 序列化 + 文件持久化)
- `crates/apeireth-graph/src/lib.rs` (262 行, Graph/Edge/Node trait)
- 3 tests pass

**R18 扩展路径**:
1. 当前 Checkpoint 用 JSON 文件 — **R18 W3 视频 100MB+ 产物必须升级**
2. 加 `CheckpointBackend` trait: `FileBackend` (现有) + `SqliteBackend` (R18 新增) + `S3Backend` (复用 apeireth-tools S3)
3. 视频产物 + 中间 state → SqliteBackend (BLOB) 或 S3 (大文件)
4. 默认 backend 可配置: `Graph::with_backend(SqliteBackend::new(...))`

**工作量**: 复用 80% + 新增 ~ 150 LOC (SqliteBackend + backend trait)。

#### 1.2.5 apeireth-formal (扩展 Kani+Prusti 双证明器)

**复用锚点**:
- `crates/apeireth-formal/src/invariants/double_onion_sample.rs` (86 行, 1 个 Kani harness)
- `PermissionLayerConfig` POD 类型避免 Kani 状态爆炸
- 4 tests pass (sanity + 正例 + 反例 + harness visibility)

**R18 扩展路径**:
1. 加 `Prusti` 集成 (Prusti 是 Rust 形式化验证工具, 基于 Viper)
2. 加 invariants 文件 9 个 (R18 目标: 1 → 10):
   - `double_onion_sample.rs` (现有)
   - `self_disable_attack.rs` (R17 已落, 复用)
   - `electronic_ring.rs` (R11 ceiling)
   - `principle_layer.rs` (Stage 4 §6.1)
   - `permission_layer.rs` (Stage 4 §6.1)
   - `r_measure_continuity.rs` (Stage 6 verification §5)
   - `mcp_jsonrpc_idempotent.rs` (R18 W2 new)
   - `video_artifact_purge.rs` (R18 W3 new)
   - `voice_clone_pii_encryption.rs` (R18 W5 new)
3. CI cargo kani + cargo prusti 双跑

**工作量**: 复用 20% + 新增 ~ 700 LOC (10 不变量 + Prusti 集成)。

### 1.3 复用 vs 新建对照 (5 新 crate 全覆盖 R18, 无新建)

| 需求 | 复用 / 扩展 | 新建? |
|---|---|---|
| 6 类 API transport | apeireth-mcp 扩展 | ❌ |
| embedding 检索 | apeireth-vector 直接复用 | ❌ |
| 多语言绑定 | apeireth-sdk 重建 + pybridge 复用 | ❌ |
| 跨域编排 | apeireth-graph 扩展 checkpoint | ❌ |
| 形式化验证 | apeireth-formal 扩展 Kani+Prusti | ❌ |
| **总计** | **5 crate 全复用 / 扩展** | **0 新建** |

**关键 insight**: R18 启动**不引入新 crate**, 5 个新 crate skeleton (R17 + R18 W1 派单) 全部承担 multimodal 接入角色。

---

## 2. 终审矩阵 (c) — 24 项 actionable backlog × {owner / fix / 验收}

### 2.1 矩阵 (16 项 R18 W1-W5 必做 + 8 项 R19+ 调研)

| # | ID | 描述 | Owner | Fix 步骤 | 验收 |
|---|---|---|---|---|---|
| 1 | G1.1 | 22-trait 矩阵不增变体 (multimodal 走 enum 子类型) | architect + architect2 | 写 ADR-0010 + stage6 §1 加 footnote | ADR merged + stage6 LOCKED 不动 |
| 2 | G1.2 | Signal::Image/Audio/Video + Expression::Image/Audio/Video/Music 子类型 | architect | stage4 §3 加子类型, 由 architect 实施 | 子类型编译通过 |
| 3 | G2.1 | mcp.tools/call → Action::execute 包装 | mcp_integration_expert | mcp lib.rs 加 wrapper + 更新 6 测试 | tests pass + Action 守门触达 |
| 4 | G2.2 | file_ops 6 ops 加 MCP wrapper | backend_engineer | mcp lib.rs 加 FileOpsTool | cargo test --workspace pass |
| 5 | G3.1 | RiskClass::MultimodalHigh (image/video/music/voice) 5 变体 | code_reviewer | sovereignty 加 enum + `#[non_exhaustive]` | RiskClass match 编译通过 |
| 6 | G3.2 | image/video/music/voice 走 Council 7 advisor 审议 | code_reviewer | sovereignty 加 multimodal 触发逻辑 | W4 端到端测试 |
| 7 | G4.1 | 视频 100MB+ 产物 cgroup 限制 + temp file cleanup | devops_engineer | devops 加 cgroup profile + mcp cleanup hook | W5 大文件测试 |
| 8 | G4.2 | VoiceClone reference 音频加密存储 | code_reviewer | sovereignty 加 PII 加密 trait | W5 端到端测试 |
| 9 | M1.1 | pipeline (run_cycle) 多模分支 | backend_engineer | pipeline 加 multimodal run_cycle | cargo test pass |
| 10 | M1.2 | protocol (MessageContent) 多 part (serde untagged) | backend_engineer | protocol 加 Vec<ContentPart> | 旧 String 仍兼容 |
| 11 | M1.3 | tool-registry (Modality 分类 9 类别) | backend_engineer | tool-registry 加 Modality enum | 9 类分类测试 |
| 12 | M1.4 | memory (MemoryKind) 多模 (Image/Audio/Video) | database_engineer | memory 加 MemoryKind enum | W3 多模持久化测试 |
| 13 | M1.5 | sovereignty (HighRiskAction::Modality) | code_reviewer | sovereignty 加 HighRiskAction::VoiceClone | W4 测试 |
| 14 | M1.6 | tui multimodal 渲染 (image preview) | frontend_engineer | tui 加 ratatui-image | W5 demo |
| 15 | W1 | 文件 6 ops + S3 backend + 1 example | backend_engineer | apeireth-tools::S3FileOps + mcp FileOpsTool | example 跑通 + tests |
| 16 | W2-3 | 图像 MCP adapter (协议对齐 + seedream-mcp + Gemini MCP + 多模态理解) | mcp_integration_expert | mcp::ImageGenTool + VisionTool + 2 backends | W3 端到端 + tests |
| **17** | **W4** | **语音 TTS (OpenAI TTS + ElevenLabs MCP)** | **mcp_integration_expert** | **mcp::TTSTool + ElevenLabsTool** | **W4 端到端** |
| **18** | **W5** | **视频启动 + 调研 spec (Runway API)** | **mcp_integration_expert + backend_engineer** | **写 reports/r18-video-api-survey.md** | **spec 落地 + 实施启动** |
| 19 | GAP-1 | stage4 §6.4 多模 hook 增量追加 | architect2 | stage4 §6 后加 §6.4 (不修改 §6.1-§6.3) | stage4 LOCKED 不动 + 新节提交 |
| 20 | GAP-2 | stage4 §7.4.1 多模数据流 | database_engineer | stage4 §7.4 末加 §7.4.1 | stage4 LOCKED 不动 + 新节提交 |
| 21 | GAP-3 | stage6 verification §5.1 多模维度 | qa_engineer | stage6 verification §5 末加 §5.1 | stage6 LOCKED 不动 + 新节提交 |
| 22 | ADR-0010 | 5/4 重守门契约合规缺口 ADR | architect2 | docs/adr/0010-five-vs-four-gates-conformance.md | ADR merged + stage4 §10.5 footnote |
| 23 | ADR-0011 | 不修改承诺 7 vs 8 演化 ADR | architect + technical_writer | docs/adr/0011-no-modify-promise-7-vs-8.md | APEIRETH-CONVENTIONS §10 同步 |
| 24 | ADR-0012 | V1136 子测度 7 vs 9 演化 ADR | architect | docs/adr/0012-v1136-submeasures-7.md | APEIRETH-CONVENTIONS §11 同步 |
| 25 | Survey-1 | R19+ 音乐 API 调研 | (R19 owner 待派) | reports/r19-music-api-survey.md | 调研产出 |
| 26 | Survey-2 | R19+ 音色 API 调研 | (R19 owner 待派) | reports/r19-voice-clone-survey.md | 调研产出 |
| 27 | R-008 5/4 重守门 | 5/4 重守门契约合规 ADR (同 #22) | architect2 | ADR-0010 | 已计入 #22 |
| 28 | R-009 formal 1/22 覆盖 | invariants 1 → 10 | code_reviewer | invariants/ 加 9 文件 | Kani + Prusti 跑通 |
| 29 | R-010 7 vs 8 | 同 #23 | architect + technical_writer | ADR-0011 | 已计入 #23 |
| 30 | R-011 7 vs 9 | 同 #24 | architect | ADR-0012 | 已计入 #24 |
| 31 | R-014 rusqlite | workspace rusqlite 0.31/0.32 冲突修复 | backend_engineer | root Cargo.toml 统一到 0.32 | cargo build pass |
| 32 | R-015 SSE | SSE transport 真实实现 | backend_engineer | mcp transport/sse.rs NotImplemented → 真实 | SSE 端到端测试 |

### 2.2 Owner 分布汇总

| Owner | R18 W1-W5 项数 | 总项数 (含 ADR/Survey) |
|---|:---:|:---:|
| **architect2** | GAP-1 + ADR-0010 + ADR-0011 + ADR-0012 + G1.1 | 5 |
| **backend_engineer** | G2.2 + M1.1 + M1.2 + M1.3 + W1 + W5 协助 + R-014 + R-015 | 8 |
| **mcp_integration_expert** | G2.1 + W2-3 + W4 + W5 主导 | 4 |
| **code_reviewer** | G3.1 + G3.2 + G4.2 + M1.5 + R-009 + Kani+Prusti | 6 |
| **database_engineer** | GAP-2 + M1.4 | 2 |
| **qa_engineer** | GAP-3 + verification 实测 | 2 |
| **frontend_engineer** | M1.6 TUI multimodal 渲染 | 1 |
| **devops_engineer** | G4.1 cgroup + CI cargo kani | 2 |
| **architect** | G1.2 子类型 + ADR-0011 协同 + ADR-0012 | 3 |
| **technical_writer** | ADR-0011 协同 + README/CHANGELOG banner | 2 |
| **(R19 owner 待派)** | Survey-1 + Survey-2 | 2 |

**总项数**: 32 项 actionable (含 8 项 ADR/Survey/GAP-增量追加)

### 2.3 验收规约

每项必须满足:
- [ ] owner 在 W1-W5 内完成
- [ ] 不触动任何 stage1-6 LOCKED 文档(GAP-1/2/3 增量追加走"§X.1 新节"模式, 不修改既有 §X)
- [ ] `cargo test --workspace` 全过
- [ ] `cargo kani --harness X` 至少 1 个不变量过 (R18 W4 末)
- [ ] `cargo clippy --workspace` 0 warning
- [ ] 8 项不假装承诺 全守(尤其 4 重守门 + R11 baseline 三值 + 22 trait enum 编译期 hardcode)

---

## 3. 关键校验 (Ponytail: 1 行 1 锚点)

- **22-trait 矩阵不增变体**: `INTERLOCKED_TRAIT_COUNT = 22` const + `interlock_assert!` macro 不动
- **5 重守门扩展**: stage4 §10.5 (line 1332) gate 1/5 不变, gate 2/3/4 加 multimodal 通道
- **R18 启动不修改任何 LOCKED**: stage1-6 共 54 份 LOCKED 文档 + Cargo.lock + 阶段 1-5 LOCKED 全不触
- **R11 baseline 三值 LOCKED**: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 (APEIRETH-CONVENTIONS.md §11)
- **V0.5 17 维 LOCKED**: V1141 IC-001 fresh 坐标系
- **V1136 7 子测度 LOCKED**: APEIRETH-CONVENTIONS.md §11
- **ASI 北极星 ultimate 0.9800 LOCKED**: 主 22:33
- **8 项不假装承诺**: CHANGELOG.md §120-131 (canonical 8 项)

---

## 4. 给 R18 kickoff 的核心结论

> **3 行给 R18 kickoff**:
> 1. **架构影响是"扩展型"而非"破坏型"**: 22-trait 矩阵不增变体, 5 重守门扩展 gate 2/3/4 而非重写, 6 个 module 加 multimodal 通道而非替换。R18 启动**不修改任何 LOCKED**, 仅追加 footnote + 新 ADR + 新 spec。
> 2. **V2 5 新 crate 全部复用 / 扩展**, 无新建 crate: mcp 直接 / vector 直接 / sdk 重建+pybridge 复用 / graph 扩展 checkpoint / formal 扩展 Kani+Prusti。
> 3. **总投入 32 项 actionable (24 项 + 8 GAP/ADR/Survey)**: R18 W1-W5 必做 16 项 + R19+ 调研 8 项 + 增量追加 3 GAP + 3 ADR。资源需求: 3 人并行 ≈ 3-4 月。

**R18 kickoff 议程** 详见 1 页摘要 `docs/r18-kickoff-1page-2026-08-05.md` (本任务产物 (d))。

---

## 附录 A: 来源溯源

| 来源 | 路径 | 用法 |
|---|---|---|
| architect R18 impact | `reports/r18-architecture-impact-2026-08-05.md` | 24 项 actionable backlog + 16 项 R18 W1-W5 + 8 项 R19+ |
| architect2 multimodal spec | `reports/r18-multimodal-api-spec-2026-08-05.md` | 6 类 API + 5 周估算 + W1 任务清单 |
| T13 architect 评审 | `reports/d67aedf7-v2-5-new-crates-design-review.md` | 5 新 crate 现状 + 3 PASS + 1 PARTIAL + 1 MUST FIX |
| T10 code audit | `reports/v2-code-quality-audit-2026-08-05.md` | 4 PASS + 1 FAIL + 8 项行动项 |
| risk register | `reports/v2-risk-register-2026-08-05.md` | 17 项风险 + D-4 决策 + 10.5 人月 |
| stage4 LOCKED §10.5 | `docs/stage4/architecture-stage4-engineering-landing.md:1332-1340` | 5 重守门原始定义 |
| stage4 LOCKED §3 | `docs/stage4/architecture-stage4-engineering-landing.md:232-606` | 22 trait 接口定义 |
| stage6 LOCKED 22-trait | `docs/stage6/22-trait-interlock.md:23-322` | 22 trait 互锁矩阵 + enum hardcode |
| stage6 LOCKED verification | `docs/stage6/verification-protocol.md:21-296` | M1/M2/M3 + R-Measure + 18 项真测 |

---

## 附录 B: 锚点文件 (LOCKED 引用清单)

| 文件 | LOCKED 节 | 引用方式 |
|---|---|---|
| `docs/stage4/architecture-stage4-engineering-landing.md` | §3 / §6 / §7.4 / §10.5 | 引用 + GAP 增量追加 |
| `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md` | §1.1 / §1.2 | 4 重 + PermissionGrant LOCKED |
| `docs/stage5/stage5-construction-document.md` | §2 / §3 | 9 → 17 crate + V0.5 24 维 |
| `docs/stage6/22-trait-interlock.md` | §1 / §2 / §7 | 22 trait + enum hardcode + 不修改承诺 |
| `docs/stage6/verification-protocol.md` | §1-§4 / §6-§8 | M1-M3 + 守门 + 真测 + 不修改承诺 |
| `APEIRETH-CONVENTIONS.md` | §0 row 6 / §10 / §11 | R11 baseline + 不修改承诺 + R-Measure 三值 |
| `CHANGELOG.md` | §120-131 | 8 项不假装承诺 canonical |
| `docs/adr/0003-trait-interlock-22-enum.md` | 全篇 | 22 enum 编译期 hardcode |
| `docs/adr/0010-five-vs-four-gates-conformance.md` | (R18 W1 新增) | 5/4 重守门 ADR |
| `docs/adr/0011-no-modify-promise-7-vs-8.md` | (R18 W1 新增) | 7 → 8 演化 ADR |
| `docs/adr/0012-v1136-submeasures-7.md` | (R18 W1 新增) | V1136 7 子测度 ADR |

---

## 附录 C: 主人一次签收位 (Sign-Off Box)

```
主人签收: _______________________ (签名/日期)
D-1 [ ] D-2 [ ] D-3 [ ] D-4 [ ] D-5 [ ] D-tag-A [ ] D-tag-B [ ]
R18 启动 [ ] (接受 / 改写 / 推迟 24 项 actionable backlog)
回执给: agent_orchestrator / Leader
回执渠道: team_message_role 或 IM
```

---

_Last update_: 2026-08-05 (architect2 v1.0.0-FinalReview, R18 addendum 终审 + 与 V2 架构对齐)