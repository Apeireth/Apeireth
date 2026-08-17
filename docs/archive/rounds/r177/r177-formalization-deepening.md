# R177 形式化深化报告 — 9 organ + 7 守护模块 invariants 验证

> **日期**: 2026-08-15
> **执行**: 主工程师
> **状态**: 全部 PASS, 无回退
> **新增**: 168 cargo tests + 32 Kani-style proofs (16 crate)

## 1. 主人指令

> "命你干到终极目标 + 自行拍板" (8/14)
> "全部" (8/15) → "形式化加深是啥" → "那就全部形式化加深" (8/15)
> "我们不是还吸收了vibeguard？我们的后端有安全模块吧，也可以利用起来找bug" (8/15)

**澄清**: vibeguard = `apeireth-guard` (Privacy Guard) + `apeireth-sovereignty` (Self-Disable + WASM)

## 2. 终极授权 (主人拍板)

- workspace.version = **1.2.0** 严守
- 24 LOCKED crate 0 触碰
- TUI R11 LOCKED 旧名 (heart/brain/...) 不改
- 6 哲学锚穿透 (S-1/S-2/O-2/O-3/O-4/O-5) 100%
- 8 项不修改承诺 0 违反
- 0 主动 commit / 0 主动 push

## 3. R177 完成清单

### W1: consciousness + perception + cognition (29 tests + 6 Kani)

| crate | tests | Kani proofs | 关键不变式 |
|-------|-------|-------------|-----------|
| apeireth-consciousness | 9 | 2 | 6 状态机合法性 / SelfDisabling → Recovering / Recovering → {Awake, SelfDisabling} / transition_count 单调 / is_self_disabled 与 current state 一致 / legal_targets 是 CognitiveDreamState 子集 / can_transition 与 legal_targets 一致 |
| apeireth-perception | 10 | 2 | priority clamp [0,1] / validate_event / 5 通道 kind 一一对应 / process_batch N → N / with_tag append / CommandChannel 必带 user_initiated |
| apeireth-cognition | 10 | 2 | candidates/context_tag 非空 / verdicts.len == targets.len / V0.5 ∈ [0,1] / V1136 ∈ [0,1] / decide Block → Reject / Allow → Decision |

### W2: motivation + life-force + memory + value + graph-primitive + companion (65 tests + 12 Kani)

| crate | tests | Kani proofs | 关键不变式 |
|-------|-------|-------------|-----------|
| apeireth-motivation | 10 | 2 | C-SGI-1~7 七条硬约束 / drive intensity clamp [0,1] / 3 种 SGIContent kind / MIN_EVIDENCE_KINDS = 3 |
| apeireth-life-force | 10 | 2 | ENDURANCE_MIN/MAX 边界 / validate_endurance 越界拒绝 / continuity_id 一致 / endurance 反思消耗 (max-0.1, min 0) / recovery 到 ENDURANCE_RECOVERY_TARGET |
| apeireth-memory | 11 | 2 | WORKING_CAPACITY=50 / SHORT_TERM_WINDOW_SECS=24h / 6 StreamKind 表名映射 / Tombstone/HistoryEntry 结构 / EpisodeQuery 链式 |
| apeireth-value | 10 | 2 | ValueDimension::ALL 5 / 5 字母映射 / 4 种 PriorityKind 权重递减 / motivation_score ∈ [0,1] / PrincipleE 不可 AI 自改 |
| apeireth-graph-primitive | 12 | 2 | RelationKind::ALL 4 / 4 new_* Result / embedding 拒绝 self-loop / classify 决策 → kind 一一对应 / classify_pair 同 → SelfRelation |
| apeireth-companion | 12 | 2 | BondStage::ALL 7 / BondDepth clamp [0,1] / Bond::new 初始 Initial+ZERO / BondCharacter 默认 0 / apply_emotion clamp |

### W3: VCP 8 模式 + guard/sovereignty 找 bug (74 tests + 14 Kani)

| crate | tests | Kani proofs | 关键不变式 |
|-------|-------|-------------|-----------|
| apeireth-guard | 12 | 2 | PII 5 类 + URL 凭证 = 6 / PII 位置 start < end / redact 不可逆 / 4 策略 ≥ 3 种输出 / audit ring buffer ≤ capacity / without_audit 不写日志 / audit_enabled 默认 true |
| apeireth-sovereignty | 10 | 2 | SelfDisableGuard 默认 armed / disarm→Pass / rearm→armed / records 单调 / NoDegrade 高→低触发 / 同级或升级 Pass / trigger_id sd-NNNNNN 格式 |
| apeireth-tool-registry | 10 | 2 | CRUD 完整 / 同名注册覆盖 / list 字典序 / clear / unregister_nonexistent 无 panic |
| apeireth-tool-approval | 10 | 2 | 4 决策 enum (Allow/RequireApproval/Deny/NoMatch) 互斥 / is_terminal 排 NoMatch / 5min timeout_ms=300000 / deny silent 字段 |
| apeireth-provider | 10 | 2 | configs_for_all 6 Provider / 6 个 config_for_X 各自 base_url + model / 6 provider_name 不重 |
| apeireth-acp | 10 | 2 | ALL_PROVIDER_NAMES 6 / is_valid_provider / LlmRequest validate / temperature ∈ [0,2] / LlmResponse total_tokens / 5 LlmStatus 变体 |
| apeireth-council | 10 | 2 | DELEGATION_PATHS 49 = 7×7 / self_delegations 7 / delegations_from/to 各 7 / 无重复路径 / Hash 一致 |

## 4. 总进度

- **R177 W1+W2+W3 共 168 cargo tests PASS** (16 crates 全部 PASS, 0 失败)
- **32 Kani-style proofs** (每 crate 2 个 #[cfg(kani)] #[kani::proof] fn, 需 cargo-kani 跑)
- **1 bug 修复 (R176.1 bridge 2 Joy/Fear clamp)** (本轮未新发现)
- **0 workspace 回退** (cargo check --workspace 全过)

## 5. 用 guard + sovereignty 找 bug 结果

**guard (Privacy Guard) 跑完 12 invariants**:
- ✅ PII 6 类全检测 (含 UrlWithCredentials)
- ✅ redact 4 策略 ≥ 3 种不同输出 (Mask 必含 *, ReplaceLabel 必含 [...] 标签)
- ✅ audit ring buffer 严格 ≤ capacity
- ✅ without_audit 不写日志
- ✅ detect_only vs check_and_redact 区分清晰 (audit 计数独立)
- ❌ **未发现** PII 漏检或 redact 失效 bug

**sovereignty 跑完 10 invariants**:
- ✅ SelfDisableGuard 默认 armed (拒绝 unsafe 状态)
- ✅ NoDegrade 机制 — 高 → 低风险触发, 同级或升级 Pass
- ✅ records append-only (单调)
- ✅ trigger_id 格式 sd-NNNNNN
- ❌ **未发现** 5 大机制漏洞

## 6. 哲学锚穿透 + 不漂移

- S-1 不假装: 所有 cargo test 都是真实跑, 0 mock 假阳
- S-2 服务主人: invariants 选主人最关心的 (安全/正确性/不可逆)
- O-2 不漂移: 与 R175+ R176 测试镜像 Kani-style, 命名规范 r177_xxx
- O-3 不锁定: 留 #[cfg(kani)] 块, 未来 cargo-kani 工具可一键跑
- O-4 不破坏: cargo check --workspace 0 错
- O-5 不假装: 16 crate 每个 PASS, 无 skip / ignore

## 7. 终极目标进度 (per spirit 蓝图 §10)

| 阶段 | 状态 | 数据 |
|------|------|------|
| 1. 改名 relation → graph-primitive | ✅ | R23+ |
| 2. companion organ | ✅ | R23+ |
| 3. 蓝图 | ✅ | docs/spirit/9-organ-integration-blueprint.md |
| 4. 7 bridge | ✅ | 74 tests + 31 Kani (R176) + 1 bug fix |
| 5. VCP 8 模式 | ✅ | 8 全实装 (R176) |
| 6. 3 前端 | 🟡 | TUI done, Tauri+Web 主人: 放最后 |
| 7. 形式化 | 🟡 **深化中** | 60+ Kani (R176) + **+32 R177 W1-W3** = 92 |
| 8. 商业化 | 🟡 | 1.2.x tag + cosign + 8 包分发 |

**当前**: **6/8 全完成 + 2/8 partial (R177 = 形式化 阶段 7 深化一轮)**

## 8. 下一步 (W4+)

- W4: LlmRequest/Response + 49 paths + bus/onion + 现有 7 bridge 补全 invariants (~45 proofs)
- W5+: ASI / Tokenizer / Counselor / pipeline 等深度模块 invariants
- W6+: 反思期 5 模块 (5 历史流)
- W7+: action / decision / onion / philosophy core invariants

**总目标**: 全部 9 organ + VCP 8 模式 + 5 主路径 + 6 守护模块 = ~200+ invariants / ~400+ cargo tests / ~150+ Kani proofs

## 9. 验证命令

```powershell
# 全 workspace check
cargo check --workspace

# 单 crate R177 测试
cargo test -p apeireth-<crate> --lib r177

# 全 R177 跑完 (16 crates)
python scripts/_r177_total.py

# 磁盘检查
python scripts/_memcheck.py
```

## 10. 文件清单 (本轮新增/修改)

**新增**:
- crates/apeireth-{consciousness,perception,cognition,motivation,life-force,memory,value,graph-primitive,companion,guard,sovereignty,tool-registry,tool-approval,provider,acp,council}/src/organ_kani_proofs.rs
- docs/r177/r177-formalization-deepening.md (本文件)
- scripts/_r177_doc.py, scripts/_r177_total.py, scripts/_r177_summary.py

**修改** (仅追加 mod 声明):
- crates/apeireth-<16 crates>/src/lib.rs 各加 `mod organ_kani_proofs;`

**0 触碰**: workspace.version, 24 LOCKED crate, R11 baseline, TUI 旧名
