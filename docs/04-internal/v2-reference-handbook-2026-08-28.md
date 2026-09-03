# Apeireth v2.0.0 Reference Handbook — 给接手团队的话 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 9 收盘, Round 1-9 完整真账 + 0 装诚实标 + 一站式 reference)
> **用途**: 接手 Apeireth v2.0.0-rc.1 工程的工程师 / 未来 Mavis cycle 接手 / 任何改 src / 改 doc / 派子代理 必读
> **关系**: 这是 **一站式 reference** — 跟 `ENGINEER-MANIFESTO.md` (工程规范) + `handoff-log-2026-08-28-mavis.md` (Round 1-3 真账) + `sub-agent-audit-round-4-2026-08-28.md` (Round 4 audit) + `round-8-verifications-2026-08-28.md` (Round 8 verify) + 6 真账 doc (Round 9 调研) 互补

```
[Document-Meta]
Document:        docs/04-internal/v2-reference-handbook-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 v2.0.0-rc.1 收盘 + Round 1-9 真账 + 6 sub-agent 调研 + B-A 失守撤 + token 紧现实)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (一站式 reference, 接手工程师必读)
Author:          主代理 Mavis
```

---

## 0. 一句话接手

Apeireth v2.0.0-rc.1 在 `origin/main @ 70281cc6` (Round 9 完), **9 哲学锚 LOCKED, 1739 tests PASS, 0 clippy 警告, 0 触碰 LOCKED 5 项**. A 块 (OrganOrchestrator 完整化) 已落地, B/C/D 3 块调研真账已写完 (~1586 行 调研 docs), **派单顺序 + critical path 5-7 周 调研就位**, 接手工程师读本 handbook + 6 调研 doc + ENGINEER-MANIFESTO.md, 按本手册 §8 实施顺序开干即可.

---

## 1. v2 工程现状真账 (2026-08-28 Round 9 收盘)

### 1.1 工程总进度

| 项 | 值 | 来源 |
|---|---|---|
| HEAD (origin/main) | `70281cc6` | Round 9 7 doc batch commit |
| Workspace | **16 crates** | foundation 6 / engine 6 / capabilities 1 / adapters 3 |
| 代码量 | ~74k 行 active (不含 legacy/) | per ROADMAP §3 |
| 测试 | **1739 passed / 0 failed / 12 ignored** | per cargo test --workspace --locked |
| Clippy | **0 警告 / 0 错误** | per cargo clippy --workspace --all-targets --locked -- -D warnings |
| 总进度 | **80%** (A 块完成贡献 +10%, per ROADMAP §7 主代理估) | subjective 加权 |
| v2.0.0 release 估 | **2027-01-08 至 2027-02 月** (4-6 月, 因 A 块提前从 5-7 月缩短) | per ROADMAP §0 |

### 1.2 9 哲学锚 LOCKED 真账 (Round 9 维持)

```
S-1 北极星 | S-2 实事求是 | S-3 质量工程化 NEW (R126) | O-1 安全优先 NEW (R126) |
O-2 前人肩上 | O-3 干到底 | O-4 任何人都能接手 | O-5 不假装 (0 装 PASS) | O-6 永远追求最优 NEW (2026-08-27)
```

- 位置: `crates/foundation/core/src/eight_anchors.rs:58-79` (enum `PhilosophicalAnchor8`, 9 variants)
- 编译期 hardcode: `crates/foundation/core/src/eight_anchors.rs:222-366` (`NINE_ANCHORS_HARDCODE` const)
- **改这 9 enum / 顺序 / 描述 = 0 触碰 LOCKED 失守**, 必主代理拍板
- O-6 doctrine: "工作量与麻烦不是拒绝重做的理由" — Round 1-9 0 找借口

### 1.3 12 slot cognitive module 真账 (12 slot ledger)

| Slot | Status |
|---|---|
| `cognitive.memory_recall` | WIRED |
| `cognitive.preference_recall` | WIRED |
| `cognitive.judge` | WIRED, OFF by default |
| `cognitive.council` | WIRED, OFF by default |
| `cognitive.self_assessment` | WIRED, Judge-backed |
| `cognitive.memory_writeback` | WIRED |
| **`cognitive.preference_learning`** | **DEFERRED → R20 派单 (调研真账就位, 2-3 周)** |
| `cognitive.critic` | DEFERRED INTO JUDGE → R21 派单 (1 周, critic.rs 1:1 翻译) |
| `cognitive.reflection` | DEFERRED INTO SELF-ASSESSMENT → R22 派单 (1 周) |
| `cognitive.planner` | NOT AN AGENT MODULE → R23 派单 (3 周, LLM Adapter 新设计) |
| `cognitive.orchestrator` | NOT AN AGENT MODULE → R24 派单 (3 周, 严格分界 R12) |
| `cognitive.perception` | NOT AN AGENT MODULE → R14 真 modality (2-3 周, 需硬件) |

位置: `docs/04-internal/cognitive-module-wiring.md` (110 行)

### 1.4 LOCKED 5 项 (per MANIFESTO §10)

| LOCKED 项 | 位置 | 真例外 |
|---|---|---|
| 9 哲学锚本体 | `crates/foundation/core/src/eight_anchors.rs:58-79` + `NINE_ANCHORS_HARDCODE` 编译期锁 222-366 | 主人明确授权 (例: 2026-08-27 加 O-6) |
| 13 键 | `crates/foundation/core/src/philosophy.rs:142` `RUNTIME_ENFORCED = false` | 已拍板降级, 不接回 runtime 强制 |
| 3 项不可变脊柱 | `crates/foundation/core/src/onion.rs:249` (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义) | 同 9 哲学锚 |
| workspace.version | `Cargo.toml:44` `"1.2.0"` 双轴制 | tag 推进 v2.0.0 → v2.0.1 改 patch, 主代理拍板 |
| R11 baseline 3 值 | legacy reference (`legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44` `R11_V1141_BASELINE: f64 = 0.8682` / `R11_V1131_BASELINE: f64 = 0.8532` / `R11_V1136_BASELINE: f64 = 0.9063`) — **active workspace 无 const source**, 等 R12 spec 重新审定后移植 | R11 数字更新需 R12 spec 重新审定 + active workspace 移植, 主代理拍板 |

### 1.5 A 块完成真账 (Round 1-2 实施, Round 3 amend + 复盘, Round 4 author 修)

- 5 stage OrganOrchestrator 完整化 commits (per `organ-orchestrator-completion-plan.md` §7):
  - `c003e078` Stage 1 ratify_fresh_policy (缺口 D)
  - `087ab2ac` Stage 2 extract_emotion_mood (缺口 B)
  - `50ba2e57` Stage 3 check_8_gates (缺口 A)
  - `29e5ce66` Stage 4 decide_with_invoker (缺口 C)
  - `0afa733f` Stage 5 L0-L5 UpgradeCycle (缺口 E)
- + 1 复盘配对 commit `bbbfb75b` (O-6 三阶审查 + 后续 commit 标准)
- + `3a957056` ENGINEER-MANIFESTO doc 本体 8→9 真修 followup
- + `1d885299` A 块完成真账同步
- 测试: 1726 baseline → 1739 passed / 0 failed / 0 LOCKED 触碰 / 0 引新外部 dep

### 1.6 Round 1-9 完整 commit 链 (origin/main)

```
70281cc6 docs: Round 9 - 6 sub-agent 调研真账 batch (Round 9)
3eb7f26b chore+docs: Round 8 part 2 - 2 sub-agent 调研真账 + frontend hygiene + O-5 真账
66649ead docs: Round 8 part 1 - 4 处 v2-gateway HEAD stale + verify doc
7d990297 docs: §4.5 术语统一表 (Round 6)
2155d694 fix(sdk): SDK lib.rs _SIX_PHILOSOPHY_ANCHORS (Round 6)
7a861938 chore+docs: §8.5 pre-commit + commit-msg hook (Round 6)
dd4a72de docs+src: Round 5 backlog (src 漂移 + 3 docs R11 path + §13 #12 author env)
bde6268b docs: sub-agent 审计报告 Round 4
0ca16572 docs: §13 增 3 条 Round 1-3 工序教训 (8→11, Round 6)
aa488f1f docs: Round 3 audit 修 5 docs 8 处 8→9 锚 LOCKED 真漂移
13e73891 docs: organ-orchestrator-spec 8→9 锚漂移修 (Round 2)
3a957056 docs: ENGINEER-MANIFESTO doc 本体 8→9 真修 + .gitignore + handoff log (A 块 followup)
5e18e65b docs: ENGINEER-MANIFESTO 工程师团队 reference 手册 (14 章, Round 1 msg amend)
1d885299 docs: A 块完成真账同步 (Round 1 msg amend)
bbbfb75b docs: A 块 5 commit O-6 三阶审查 0 装诚实复盘 (Round 1 amend 配对)
c003e078 refactor(runtime): OrganOrchestrator 完整化 stage 1 (A 块)
087ab2ac refactor(runtime): OrganOrchestrator 完整化 stage 2 (A 块)
50ba2e57 refactor(runtime): OrganOrchestrator 完整化 stage 3 (A 块)
29e5ce66 refactor(runtime): OrganOrchestrator 完整化 stage 4 (A 块)
0afa733f refactor(runtime): OrganOrchestrator 完整化 stage 5 (A 块)
```

---

## 2. 哲学锚 9 项 (必读, 改 src 前必默念)

### 2.1 S-1 北极星

> Everything serves the ASI north star (五原型).

**工程兑现**: 9 organ 真移植 (Round 1-3) + OrganOrchestrator 完整化 (Round 1-2) + 12 slot cognitive module integration (Round 9 调研就位).

**改前自问**: "这个改动指向 ASI 北极星吗? 不是的话, 是不是走错路了?"

### 2.2 S-2 实事求是

> Verify before writing; truth over narrative.

**工程兑现**: 0 装诚实标 (Round 1-9 13 处 flag) + 数字实测 (cargo test 1739 / cargo clippy 0 / git rev-parse 真实 hash / wc -l 行数实测) + 文档数字漂移修 (Round 1 5 处 + Round 3 5 docs 8 处 + Round 7 20 处 + Round 8 4 处 = 28 处数字漂移全清).

**改前自问**: "数字必实测, 我有没有复用旧值?"

### 2.3 S-3 质量工程化

> Engineering rigor (CI gates + Kani proofs + clippy 0-warning).

**工程兑现**: clippy 0 警告 (Round 9 维持) + 5 重守门 baseline (clippy / tests / legacy compat path / LOCKED 5 项 / 9 哲学锚表头) + o6-anchor.yml workflow (`.github/workflows/o6-anchor.yml` 166 行, 自动跑 5 重守门).

**改前自问**: "clippy 跑过吗? 文档行数清吗? 测试覆盖率?"

### 2.4 O-1 安全优先

> Safety > function > performance, 9 重 v9 守门 + 13 键 verdict cache + 3 项不可变脊柱.

**工程兑现**: P0 governance 3 hook 已装 (`PermissionGovernanceHook` + `CredentialDisclosureHook` + `PromptInjectionHook`) + HTTP error 映射 (Denied → 403, ApprovalRequired → 409) + 13 键 RUNTIME_ENFORCED=false 显式标 + 3 项不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache 语义).

**改前自问**: "改动会绕过 P0 governance 3 hook 吗? 会接回 13 键 runtime 强制吗?"

### 2.5 O-2 前人肩上

> Borrow, attribute, adapt (上游标杆项目 + 标准协议), 借 + 标注 + 改。

**工程兑现**: A 块 5 stage 1:1 翻译 v1 (`legacy/donor/apeireth-companion/src/`) + R20 preference_learning 1:1 翻译 v1 TopicPredictor + PreloadChannel + R21 critic 1:1 翻译 v1 critic.rs + R22 reflection 1:1 翻译 v1 reflection.rs.

**改前自问**: "这方案借鉴了谁的? 标注来源了吗?"

### 2.6 O-3 干到底

> Finish what we start; no half-measures.

**工程兑现**: A 块 5 stage 完整 commit + 复盘配对 + 真账 sync 1 commit 1 doc 闭环. Round 1-9 每 commit 必带 0 装诚实标 + LOCKED 0 触碰 + 5 重守门 baseline.

**改前自问**: "改完跑完基线 + 文档同步 + commit + push 4 步, 不是'先这样, 以后补'"

### 2.7 O-4 任何人都能接手

> Any newcomer can onboard from docs alone.

**工程兑现**: ENGINEER-MANIFESTO.md (14 章, 596 行, Round 6 §13 12 行真实陷阱) + TO-NEW-TEAM.md + HANDOFF-NOTES.md + handoff-log-2026-08-28-mavis.md + sub-agent-audit-round-4-2026-08-28.md + round-8-verifications-2026-08-28.md + 本 handbook (Round 9 写) + 6 调研真账 (Round 9 batch).

**改前自问**: "接手人能只读你的 commit message 理解改动吗? 文档树清晰吗?"

### 2.8 O-5 不假装 (0 装 PASS)

> Never fake it — the trust bedrock.

**工程兑现**: Round 1-9 13 处 0 装诚实标 (amend 没真修 doc / 5 docs 数字漂移 / 4 处 stale doc / 5 重守门 baseline 实测 / 9 哲学锚 0 触碰实测 / 子代理 author 失守 / .harness-msg/ gitignore 谎报 7 round 才 flag / R20 + B-A sub-agent 真实施失守撤 / sub-agent 调研方法偏差修订).

**改前自问**: "TODO 是不是真没做? ✅ 是不是真过了? 没有'我觉得这样应该 work' (跑了才算)"

### 2.9 O-6 永远追求最优

> 总体最优 / 系统最优 / 架构最优 三阶审查 + 不做借口清单 + 可检查信号.

**工程兑现**: Round 1-9 每 commit 必带三阶审查 (总体/系统/架构) + 拒 alternatives + 拒理由. o6-anchor.yml 自动守门. §8.5 pre-commit + commit-msg hook (Round 6) 强制 commit msg 含三段审查关键词.

**改前自问**: "总体最优 / 系统最优 / 架构最优 三段写在 commit message 了吗? 拒 alternatives + 拒理由?"

---

## 3. 派子代理 workflow (per §6)

### 3.1 标准 brief 模板 (主代理派单必含)

```markdown
# Sub-Agent Brief Template (v2 一站式)

## 任务
[具体任务, 含估时上限]

## 必读 (真实施路径)
[4-6 file paths]

## 必输出 (写真账 doc)
[写真账 doc path + 行数约束 ≤200 行]

## 0 触碰 LOCKED (5 项 grep 命令清单)
[主代理 commit 前亲跑验证清单]

## commit message 必含 4 项标 (主代理 commit 时填, sub-agent 不 commit)
1. [具体改动描述]
2. 0 触碰 LOCKED 5 项
3. 0 引新外部 dep
4. 0 装诱导 prevention

## 约束 (主代理严守)
- ❌ 不写真账以外的 file
- ❌ 不 git add / commit / push (主代理亲验后 commit)
- ❌ 不写真账以外的 commit (5 重守门 baseline 主代理亲跑)
- ✅ 写真账到 [具体 path]
- ✅ 写真账 ≤200 行
- ✅ 写真账 必含 5 项 (实现摘要 / 5 重守门实测 / LOCKED 0 触碰验证 / 集成说明 / 下一步)

## 估时上限
[具体小时/天数]

## 报告 ≤ 200 行
```

### 3.2 sub-agent workflow 失守真账 (Round 9 教训)

| 失守 | 详情 | 教训 |
|---|---|---|
| **R20 真实施** (~6h) | sub-agent 写真账 + 真实施, 改 plan 后 sub-agent 自己撤 | brief 必含 "5 重守门 baseline + 不写真账以外的 file" |
| **B-A 真实施** (~3h) | sub-agent 写真账 + 真实施, cargo test EXIT 101 + 3 test fail, 主代理撤 | brief 必含 "跑 5 重守门 baseline + 主代理亲验前不假装 PASS" |
| **sub-agent 2 R9 spec flag 误判** | R13 review 历史快照 vs 当前 R9 spec 实测偏差 | brief 必含 "读当前 spec 实测, 不只读 review 标" |
| **sub-agent workflow `send_message` 不是 interrupt** | queued as next turn, sub-agent 已写完原 brief | 主代理发现后立即撤 + 写真账 |

### 3.3 sub-agent 写真账 vs 主代理亲验

- **sub-agent 写真账**: 实现 / 测试 / LOCKED 核验 / commit 模板 (≤200 行)
- **主代理亲验**: 写真账 + cargo check / cargo test / cargo clippy / git diff LOCKED (5 重守门 baseline 亲跑) + 决定 commit / 撤

---

## 4. 改前必跑 (per §8)

```bash
cd C:\Users\31683\apeireth-rust

# 1. 5 重守门 baseline (改前)
cargo test --workspace --locked                    # 期望: 1739 passed / 0 failed / 12 ignored
cargo clippy --workspace --all-targets --locked -- -D warnings  # 期望: 0 warning / 0 error

# 2. §10 改前必查 LOCKED (改前)
git diff HEAD -- crates/foundation/core/src/eight_anchors.rs   # 期望: 0 行
git diff HEAD -- crates/foundation/core/src/philosophy.rs        # 期望: 0 行
git diff HEAD -- crates/foundation/core/src/onion.rs             # 期望: 0 行
git diff HEAD -- Cargo.toml | grep -E "^[+-]version"             # 期望: 0 行
git diff HEAD -- crates/foundation/core/src/cognitive.rs         # 期望: 0 行 (R11 baseline 数字 0 触碰)

# 3. 9 哲学锚表头 (改前)
git diff HEAD -- crates/foundation/core/src/eight_anchors.rs | grep -E "^[+-]\s*[OS]-?[1-6]"  # 期望: 0 行

# 4. legacy compat path (改前)
grep -r "legacy/" crates/ | wc -l                  # 期望: < 100 (现 36)
```

---

## 5. commit message 模板 (per §5 + §8.2)

### 5.1 必含 5 段 (per O-6 doctrine + Round 1-9 0 装诚实标)

```markdown
<commit title>: <具体改动 + O-6 三阶审查 keywords>

- 0 装诚实真账 (per O-5):
  - [flag 任何 失守 / 0 装 PASS / 不假装 / 主代理亲验 + 修订]
  - [数字实测 vs 历史快照 vs spec vs 真账 vs 当前]
- 测试: cargo test --workspace --locked = N passed / N failed / N ignored (与 baseline 一致, doc-only / src 改?)
- clippy: 0 警告 / 0 错误 (与 baseline 一致)
- 0 触碰 LOCKED 5 项 (改的全是 doc / src? — 给具体路径)
- 0 引新外部 dep (Cargo.lock 0 行?)
- O-6 三阶审查:
  - 总体最优: <改动在大语境 (release 路线图 / 工作量约束 / 上下游依赖) 里是不是最优切入点?>
    拒方案 A: <拒>+ 拒理由
    拒方案 B: <拒>+ 拒理由
    选: <选>+ 理由
  - 系统最优: <在 Apeireth 子系统依赖图 (governance → orchestration → memory → runtime → organ) 里改动放在哪一层最合适?>
    拒方案 A: <拒>+ 拒理由
    选: <选>+ 理由
  - 架构最优: <在 workspace 16-crate 拓扑 + 单向依赖 + trait object 设计下, 公开 API 形状 + crate 边界 + 0 引新外部 dep, 这个方案是不是最优?>
    拒方案 A: <拒>+ 拒理由
    拒方案 B: <拒>+ 拒理由
    选: <选>+ 理由
- 文件 (N file, 0 源码?):
  - path1: <行数 diff> <描述>
  - path2: <行数 diff> <描述>
- 同步关系 (0 文档 0 数字漂移):
  - [改前的数字 / 改后数字 / 来源 / 真账实测]
- 下一步 (留 backlog / 派单 / 等等):
  - [下一步 action items]
```

### 5.2 commit msg 4 项标 (per Round 5 §13 #12 + Round 9 brief 模板)

1. **具体改动描述** (一句话, 含 file path / 主要变更)
2. **0 触碰 LOCKED 5 项** (per §10, 5 项 grep 实测)
3. **0 引新外部 dep** (Cargo.lock 0 行 diff)
4. **0 装诱导 prevention** (不假装 OK / 不假装 PASS / 测试真跑过 / 数字实测)

---

## 6. 文档规范 (per §7)

### 6.1 [Document-Meta] 头部 (5 行)

```markdown
[Document-Meta]
Document:        path/to/doc.md
Version:         X.Y (主代理 Mavis 写于 ...)
Last-Modified:   YYYY-MM-DD
Status:          🟢 活跃 / 🟡 待实施 / 🔴 已废弃
Author:          主代理 Mavis / 子代理 X / ...
```

### 6.2 0 数字漂移 (per §13 #5)

- 数字必实测 (cargo test / wc -l / git rev-parse / grep / cargo metadata), 不复用旧值
- 历史快照 OK 保留 (加 "当时" 标注), 但 active spec / 当前真账必须实测

### 6.3 文档树 (per §7)

```
docs/
├── 01-architecture/    (设计 + spec + 真账, LOCKED 锚定)
├── 02-guides/          (quickstart + onboarding + 真账)
├── 03-api/             (API 契约 + OpenAPI)
├── 04-internal/        (handoff + 真账 + audit + maintenance)
└── archive/            (历史归档, 0 触碰, ref-only)
```

---

## 7. 工程规范 (per §8)

### 7.1 §8.5 pre-commit + commit-msg hook (Round 6 写, 接手工程师必 enable)

```bash
git config core.hooksPath .githooks
```

- **pre-commit**: 检查 4 项
  - 检查 1 (BLOCK): `GIT_AUTHOR_NAME` env var 必须设置 (per §13 #12 Round 4 真账)
  - 检查 2 (WARN): LOCKED 5 项 0 触碰 (eight_anchors.rs / philosophy.rs / onion.rs / Cargo.toml)
- **commit-msg**: 检查 4 项
  - 检查 1 (BLOCK): msg 必含 "总体最优" / "系统最优" / "架构最优" 三段 (per §5)
  - 检查 2 (WARN): msg 必含 "拒" 关键词 (拒 alternatives + 拒理由)
- Override: `git commit --no-verify` (主代理拍板)

### 7.2 cargo fmt (per §13 #1)

- 用 `rustfmt file.rs` 单文件格式 (不要 `cargo fmt -- file1 file2`, 会格式化整个 workspace, 21 文件被动重排)

### 7.3 force push (per §8)

- `git push --force-with-lease=main:<expected-old-tip> origin main` (per §13 #6 stale info 教训)
- 老 tip 必须先 `git fetch origin` 看当前 origin/main, 用真实老 tip
- 不用 `--force` (裸 force 会覆盖别人工作)

### 7.4 工作流

```
[改 src]
1. git pull origin main (确保本地最新)
2. §4 5 重守门 baseline (改前)
3. §10 LOCKED 改前必查 (改前)
4. 改 src (per §13 真实陷阱)
5. §4 5 重守门 baseline (改后, 验证 0 副作用)
6. §10 LOCKED 改后必查 (改后, 验证 0 触碰)
7. §5 commit msg 模板 (写 5 段)
8. git commit (with env var GIT_AUTHOR_NAME)
9. git push origin main (fast-forward, 不用 --force)

[改 doc]
1. §4.5 改前必查 (per §13 #5)
2. §6 [Document-Meta] 头部 + 0 数字漂移
3. 改 doc
4. §4.5 改后必查
5. §5 commit msg 模板
6. git commit + push

[派子代理]
1. §3.1 brief 模板 (5 段)
2. sub-agent 写真账 (≤200 行)
3. 主代理亲验 (5 重守门 + LOCKED 0 触碰)
4. §5 commit msg 模板 + §10 LOCKED 核验
5. git commit + push
```

---

## 8. 实施路径 (Round 9 调研就位, 派单顺序 + critical path)

### 8.1 派单顺序 (per sub-agent 08e8bbdf §7 + sub-agent 1c7bbc45 §派单)

| # | 块 | 派单 | 估时 | critical path |
|---|---|---|---|---|
| 1 | **主代理亲做 1-2 天决策冻结** | (主代理) | 1-2 天 | 前置 |
| 2 | **A gateway SSE + auth + panel** | sub-agent A | 3-4 周 | Week 1-2 |
| 3 | **B frontend runtime.ts + 主人审批 modal** | sub-agent B | 2-3 周 | Week 2-3 |
| 4 | **C Tauri shell keyring 集成** | sub-agent C | 3-5 天 | Week 3 |
| 5 | **D E2E + 5 重守门 baseline** | sub-agent D | 1-2 周 | Week 5-6 |
| 6 | **R20 preference_learning 真实施** | sub-agent R20 | 2-3 周 | 并行 |
| 7 | **R21 critic + R22 reflection** | sub-agent | 2 周 | 并行 |
| 8 | **R14 perception 真 modality** | sub-agent | 2-3 周 (需硬件) | 硬件到位 |
| 9 | **R23 planner + R24 orchestrator** | sub-agent | 6 周 | 后 |

**critical path: 5-7 周** (per sub-agent 08e8bbdf 真账, 跟 sub-agent 2 §6 估 6-8 周相比 -1 周因 R12 已落地).
**总估时 12-19 周** (B + C + D 串行) / **4-6 月并行**.
**2027-Q1 启动, 2027-Q2 完**.

### 8.2 主代理亲做决策冻结 (Round 9 调研就位)

| # | 决策 | 推荐 (per sub-agent 08e8bbdf §7) | commit msg 模板 |
|---|---|---|---|
| 1 | **R9 spec 4 处错账修正** | 主代理亲做 commit (per Q1 C1 policy), 改 3 doc (R9 / R9 quickstart / R10), 1 commit | "R9 spec 错账修: §0 §25 / §5.1 §330 / §5.2 §342 / §10 §483, 跟 ledger 一致" |
| 2 | **R10 spec 12 slot 数字错账** | 同 §1 commit | 同上 |
| 3 | **R12 working tree** | **已跑通, 不续派** (per sub-agent 1c7bbc45 §3 项 1) | n/a |
| 4 | **9 organ UI 暴露范围** | **候选 A 默认不暴露 + 候选 D dry_run 模式 opt-in** (O-5 + Q1 + E7 8 重门控) | n/a (spec 决策, 不 commit) |
| 5 | **主人审批 modal 行为** | **候选 A 409 ApprovalRequired 弹 modal + session auto-approve toggle (VSCode Continue 模式)** | n/a |
| 6 | **Tauri keyring** | **候选 E 复用 v2 RC-9 后端 keyring** (frontend transient in-memory) | n/a |

---

## 9. Round 1-9 完整 commit 链 + 调研 doc 真账

### 9.1 Round 1 (接手 ENGINEER-MANIFESTO + 数字漂移修)

- 5 commit: `bbbfb75b` (A 块复盘) → `c003e078` → `087ab2ac` → `50ba2e57` → `29e5ce66` → `0afa733f` (A 块 5 stage) → `1d885299` (A 块真账 sync) → `5e18e65b` (ENGINEER-MANIFESTO 14 章 push, msg amend)
- 写真账: `.harness-step-log-2026-08-28.md` §0-4 (Round 1 接手 + 哲学锚读 + amend 失守 + 修)
- 0 装诚实标: amend 5e18e65b 没真修 doc 本体 (write-tree 没 git add), Round 2 flag + followup `e3300347` 真修

### 9.2 Round 2 (sub-agent R11 错账 + organ-orchestrator-spec drift)

- 1 commit: `13e73891` (organ-orchestrator-spec 8→9 锚漂移修)
- 写真账: step log §5

### 9.3 Round 3 (5 docs 8 处 8→9 锚 LOCKED drift batch + 1 force push 真账)

- 1 commit: `aa488f1f` (Round 3 audit 修 5 docs 8 处 8→9 锚)
- 写真账: step log §6 + handoff-log-2026-08-28-mavis.md
- 0 装诚实标: 0 触碰 LOCKED 5 项实测

### 9.4 Round 4 (§13 工序教训 + sub-agent audit + author amend)

- 2 commit: `bde6268b` (sub-agent audit) → `0ca16572` (§13 增 3 工序)
- 写真账: sub-agent-audit-round-4-2026-08-28.md (201 行)
- 0 装诚实标: sub-agent 报告 3 处误判, 主代理亲验 catch (5/5 brief 偏差 3 处 + O-5 doctrine + author Mavis amend)

### 9.5 Round 5 (src 漂移 + 3 docs R11 path + §13 #12 author env)

- 1 commit: `dd4a72de`
- 写真账: step log §5

### 9.6 Round 6 (§8.5 hook + SDK 真 bug fix + §4.5 术语表)

- 3 commit: `7a861938` (§8.5 hook) → `2155d694` (SDK _SIX bug fix) → `7d990297` (§4.5 术语表)
- 写真账: ENGINEER-MANIFESTO.md §8.5 + §4.5 (in-place 增)
- 0 装诚实标: SDK `_SIX_PHILOSOPHY_ANCHORS` const 6 entries 跟注释说 "8 字样" 不一致 (三方不一致), 修 const 6 → 8 entries + 名字改

### 9.7 Round 7 (20 处 stale doc batch fix)

- 1 commit: `155a9450`
- 写真账: 改 16 docs (4 个 HIGH HEAD hash + 3 个 MEDIUM 加标注 + 9 个 LOW 加 "A 块后 1739" 标注)
- 0 装诚实标: 数字漂移 (1726/1713/108 → 1739) 全部加标注保留历史

### 9.8 Round 8 part 1 (4 处 v2-gateway HEAD stale + verify doc)

- 1 commit: `66649ead`
- 写真账: round-8-verifications-2026-08-28.md §0-§3
- verify: o6-anchor.yml 5 重 CI 守门实测 + 12 slot ledger 跟 R15 spec 一致

### 9.9 Round 8 part 2 (2 sub-agent 调研真账 + frontend hygiene + O-5 失守 flag)

- 1 commit: `3eb7f26b`
- 写真账: round-8-verifications-2026-08-28.md §4-§5
- 0 装诚实标: Round 1 `e3300347` commit message 谎报 `.harness-msg/` ignore, Round 8 part 2 修

### 9.10 Round 9 (6 sub-agent 调研真账 batch)

- 1 commit: `70281cc6` (current HEAD)
- 7 真账 doc (~1586 行):
  - `r20-preference_learning-research` (346) — R20 真实施 spec
  - `r21-r24-r12-research` (195) — 5 slot + R12 调研
  - `rc7-perception-research` (228) — RC-7 真 modality
  - `b-block-decision-points-research` (199) — 6 项主代理决策
  - `b-block-gateway-sse-research` (170) — B-A 撤真账
  - `gitignore-hygiene-audit` (220) — gitignore audit
  - `r9-r10-spec-drift-audit` (228) — R9/R10 spec 0 错账
- 0 装诚实标: R20 + B-A sub-agent 真实施 O-6 失守 (cargo test EXIT 101), 主代理撤

---

## 10. 接手工程师"5 步读完" + 1 步开干

### 10.1 5 步读完 (估时 1-2 小时)

1. **读本 handbook** (1-2 hour, 你正在做)
2. **读 `ENGINEER-MANIFESTO.md`** (596 行, 14 章, 工程规范 + 真实陷阱 12 条)
3. **读 `handoff-log-2026-08-28-mavis.md`** (124 行, Round 1-3 接手真账)
4. **读 `sub-agent-audit-round-4-2026-08-28.md`** (201 行, Round 4 audit 真账)
5. **读 `round-8-verifications-2026-08-28.md`** (256 行, Round 8 CI/wiring 真账)

### 10.2 选 6 项决策冻结起点 (主代理亲做 1-2 天, per §8.2)

按 `b-block-decision-points-research-2026-08-28.md` §6 项决策:
1. R9 spec 4 处错账修 (commit msg 模板已写, 改 3 doc, 1 commit)
2. R10 spec 数字错账修 (同 §1 commit)
3. R12 不续派 (已落地, n/a)
4. 9 organ UI 暴露范围冻结 (候选 A+D, spec 决策不 commit)
5. 主人审批 modal 行为冻结 (候选 A, spec 决策不 commit)
6. Tauri keyring 冻结 (候选 E, spec 决策不 commit)

### 10.3 派 sub-agent A 真实施 B 块 gateway SSE (per §8.1)

- brief: `b-block-gateway-sse-research-2026-08-28.md` §4 (含 3 test fail 修法 spec)
- 必含 §3.1 brief 模板 5 段
- 主代理亲验 5 重守门 + 写真账 (≤200 行)
- commit + push

### 10.4 派 sub-agent 真实施 R20 preference_learning (per §8.1)

- brief: `r20-preference_learning-research-2026-08-28.md`
- 1:1 翻译 v1 TopicPredictor + PreloadChannel
- 含 ledger L30 DEFERRED→WIRED 1 行 doc sync (R15 §7.2 措辞修)
- 必含 §3.1 brief 模板 5 段
- 主代理亲验 + commit + push

### 10.5 5 重守门 baseline 严守 (每 commit)

- cargo test --workspace --locked (期望 1739 passed)
- cargo clippy --workspace --all-targets --locked -- -D warnings (期望 0 warning)
- git diff LOCKED 5 项 (期望 0 行)
- legacy compat path < 100 (期望 < 100, 现 36)
- 9 哲学锚表头 0 减 (期望 9)

---

## 11. 风险 + 0 装诚实标 (Round 1-9 教训)

### 11.1 风险总账

| 风险 | 等级 | 缓解 |
|---|---|---|
| B 块 frontend 估时溢出 | 中-高 | 派 A+B+C 并行 + 1 主代理亲做决策 + 6 周 buffer |
| C 块 6 DEFERRED slot 估时溢出 | 中 | R20 + R21+R22 并行 + R23+R24 后 + 6-10 周估时 |
| D 块 RC-7 需硬件 | 高 (硬阻塞) | 等硬件到位, mock test 先做 |
| sub-agent workflow O-6 失守 (Round 9 2 次) | 中 | brief 必含 "跑 5 重守门 + 不写真账以外" |
| 网络阻塞 push | 中 | 用户开代理 / 等恢复 / force-with-lease 老 tip |

### 11.2 0 装诚实标 (per O-5 历次 flag 真账)

1. **Round 1 amend 5e18e65b 没真修 doc 本体** (write-tree 没 git add, msg 写了"修 3 处"实际 0 修). 修: followup `e3300347` 真修 + flag
2. **Round 1 commit `e3300347` msg 谎报 `.harness-msg/` ignore** (Round 1 当时未加规则, .harness-*.txt 不递归 sub-dir). 修: Round 8 part 2 加规则 + flag
3. **Round 4 author 失守** (4 commits author = minimax-m3-agent, 不是 Mavis). 修: Round 4 author amend
4. **Round 7 batch fix commit msg 写 "20 处" 实际修 16 处** (4 处漏). 修: Round 8 part 1 收尾
5. **Round 9 R20 sub-agent 真实施 O-6 失守** (~6h, 改 plan 后撤). 修: sub-agent 自己撤 + 写真账 257 行
6. **Round 9 B-A sub-agent 真实施 O-6 失守** (~3h, cargo test EXIT 101 + 3 test fail). 修: 主代理撤 + 写真账 170 行
7. **Round 9 sub-agent 2 R9 spec flag 误判** (R13 review 历史快照 vs 当前 R9 spec 实测). 修: sub-agent 70f782ed 修订真账

### 11.3 0 装诚实 doctrine 真账

- **不假装 OK**: cargo test fail 不 commit, 撤 + 写真账
- **不"等以后修"**: O-6 失守 flag 即改
- **不"删 commit 重做"**: amend 历史 commit 风险 > 收益, 用 followup commit
- **不"只读 review"**: sub-agent 调研读当前 spec 实测, 不只读 review 标
- **不"派了不管"**: sub-agent workflow 主代理必亲验 (5 重守门 + LOCKED 0 触碰)

---

## 12. 后续 action items (按 §8.1 顺序)

| 优先级 | 项 | 派单 | 估时 | 阻塞 |
|---|---|---|---|---|
| 🟢 P0 | 主代理亲做 6 项决策冻结 (per §8.2) | 主代理 | 1-2 天 | 0 |
| 🟢 P0 | 派 sub-agent A 真实施 B 块 gateway SSE (含修 3 test fail spec) | sub-agent | 2-3 天 | §8.2 决策冻结 |
| 🟢 P0 | 派 sub-agent R20 真实施 preference_learning | sub-agent | 2-3 周 | R10 OrganKind 决策 |
| 🟡 P1 | 派 sub-agent B 真实施 frontend runtime.ts + 主人审批 modal | sub-agent | 2-3 周 | §8.2 §4-5 决策 + A 真实施 |
| 🟡 P1 | 派 sub-agent C 真实施 Tauri shell keyring (候选 E trivial) | sub-agent | 3-5 天 | §8.2 §6 决策 + C backend |
| 🟡 P1 | 派 sub-agent D 真实施 E2E + 5 重守门 baseline | sub-agent | 1-2 周 | A+B+C 全 done |
| 🟡 P1 | 派 sub-agent R21 critic + R22 reflection 真实施 | sub-agent | 2 周 | 0 (可并行) |
| 🔴 P2 | 派 sub-agent R23 planner + R24 orchestrator 真实施 (LLM Adapter 新设计) | sub-agent | 6 周 | R21+R22 done |
| 🔴 P2 | RC-7 Perception 真 modality (需硬件麦克风 + Windows) | sub-agent | 2-3 周 | 硬件到位 |
| 🟡 P1 | 修 gitignore 3 真漏 (reconstruction_v2/ + _scripts/ + .gitignore-research) + 缩 `.py` | 主代理 | 30 min | 0 |
| 🟡 P1 | 修 R15 spec §5.1 L364 critic 路径错账 (用 critic.rs 不是 judge.rs) | 主代理 | 5 min | 0 |

---

## 13. 1 段交付 (给接手工程师)

Apeireth v2.0.0-rc.1 在 `origin/main @ 70281cc6` (Round 9 完), **9 哲学锚 LOCKED, 1739 tests PASS, 0 警告, 0 触碰 LOCKED 5 项, 0 装诚实**. A 块 (OrganOrchestrator 完整化 5 stage) 已落地, B/C/D 3 块调研真账 ~1586 行已就位, 派单顺序 + critical path 5-7 周 调研清楚. **接手 = 1-2 小时读 5 份 doc + 1-2 天主代理亲做 6 项决策冻结 + 派 4-6 个 sub-agent 真实施 + 5 重守门 baseline 严守 + commit msg 5 段模板 + §8.5 hook 启用**. 2027-Q1 启动, 2027-Q2 完.

主代理 Mavis 收盘. 接手 = 你.

---

_Mavis 写于 2026-08-28 Round 9 收盘, A 块 + B/C/D 调研 + 13 处 0 装诚实标 + 1 commit chain 18 commits + 1 force push + 1 sub-agent workflow O-6 失守 + 1 token 紧现实. 一站式 reference 完成._
