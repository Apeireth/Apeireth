# 工程独立审核 + 推进交接包 (Engineering Review & Handoff Package)

> **交给人**: 独立审核方 (可以是另一个 AI 或新接手工程师)。
> **交给人做什么**: **先复核我的结论**(第 2 节, 每条带可复现命令与预期输出),
> **再按第 5 节的工作包推进**(每个包带验收门)。第 4 节明确列出我**没有**验证的东西。
> **日期**: 2026-10-06。**HEAD**: `909d67e9` (main, 已推 origin; 其父 `e4f2f451` = 台账 #39 那批)。
> **口径**: 本文件遵守仓库四级诚实口径 (`docs/03-reference/system-capabilities.md:8-10`) ——
> **IMPLEMENTED ≠ PRODUCTION WIRED ≠ DEFAULT ENABLED ≠ HARDWARE VALIDATED**。
> 本文件里每一条"已完成/已实现"都必须能落到四条里的一条, 不许含混。

---

## 0. 三分钟上手 (给审核方)

```powershell
# 仓库: https://github.com/Apeireth/Apeireth  (main)
# 本地: C:\Users\31683\Apeireth-rust   workspace.version = 2.0.0-rc.1
cargo test --workspace                 # 基线: 全绿 (见 §3.1 复核步骤)
cargo clippy --workspace --all-targets -- -D warnings
```

**必读四份** (按顺序, 约 40 分钟):

| # | 文件 | 为什么读它 |
|---|---|---|
| 1 | `docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md` | 本次能力差距审计主文 (§7 = 两级化结论, §8 = v2 18 crate 源码清单) |
| 2 | `docs/04-internal/live-verification-ledger.md` | **唯一**权威: 什么真机验过 / 什么挂账。写任何"已验证"之前必须查它 |
| 3 | `docs/03-reference/system-capabilities.md` | 四级口径定义; 它是所有能力声明的格式规范 |
| 4 | `docs/04-internal/HANDOFF-NOTES.md` | 接手人手册 (含 2026-10-06 晚审计批次段) |

**最重要的背景事实**: 本仓库有 105 个 crate 的 v1 在 `legacy/` (donor 77 / archived 15 /
frozen 13), v2 是 18 个 crate 的重构形态。**"v1 有真实现、v2 有没有"是本项目最大的信息噪音源**,
历史文档 (尤其 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`) 的 🔴 清单已大面积过时。

---

## 1. 本轮交付物

| 交付物 | 提交 | 内容 |
|---|---|---|
| 差距审计主文 (含 §7 更正 + §8 附录 A) | `f1a1082c` / `5d9e62ca` / `e4f2f451` | v1×v2 逐域对账; 库级已实现 vs 真缺口两级化; v2 18 crate 源码清单 |
| v1 全量域附录 (30+ 未编目子系统) | `b1dd6f6f` | 差距文档漏掉的 v1 子系统逐个列 v2 状态 |
| 台账 #39 | `e4f2f451` | 本批复核口径 + 复现命令 |
| `plugin/src/organ.rs` 陈旧表更正 | `e4f2f451` | 源码内"8 organ 0 装"假账 → 更正为 engine 权威现状 |
| 本审核交接包 | 本文件 | 复核矩阵 + 未验证边界 + 工作包 |

---

## 2. 结论复核矩阵 (审核方请逐条打勾)

> **怎么用**: 每条给了命令与**预期输出**。请自己跑, 不要信我的文字。
> 判定栏由审核方填写: ✅ 证实 / ⚠️ 部分正确 (附修正) / ❌ 推翻 (附反证)。

### 2.1 规模与形态

| # | 我的结论 | 复核命令 | 预期输出 | 判定 |
|---|---|---|---|---|
| A1 | v2 工作区 = **18 crate**, workspace.version = `2.0.0-rc.1` | `(Select-String -Path Cargo.toml -Pattern '^\s*"crates/').Count` + `Select-String -Path Cargo.toml -Pattern '^version'` | `18` + `version = "2.0.0-rc.1"` | |
| A2 | v1 在 `legacy/` = **106 个 Cargo.toml** (105 crate + workspace 根) | `(Get-ChildItem legacy -Recurse -Filter Cargo.toml).Count` | `106`; `legacy/Cargo.toml` members = `["donor/*","archived/*","frozen/*"]` | |

**若 A1/A2 不符**: 先确认 HEAD 与分支 (`git log --oneline -1` 应为 `909d67e9` 或其子提交)。

### 2.2 诚实纪律的"物理形态" (最重要的一组)

| # | 我的结论 | 复核命令 | 预期输出 | 判定 |
|---|---|---|---|---|
| B1 | **全 workspace 真宏调用 `unimplemented!(` / `todo!(` 只有 7 处, 全在 `adapters/sdk/src/client.rs`** (行 30/37/457/528/602/693/713) | `Select-String -Path (Get-ChildItem crates -Recurse -Filter *.rs).FullName -Pattern 'unimplemented!\(\|todo!\('` | 恰好 **8 处文本命中**, 其中 `foundation/plugin/src/perception.rs:23` 是**文档注释里提到这个宏名**(不是调用) —— 真调用 7 处, 全在 SDK | |
| B1b | perception 的非文本 modality 用**枚举变体**返回, 不是宏: `PerceptionError::NotImplemented { modality, when }` / `BackendNotWired { modality, backend, when }` | `Get-Content crates\foundation\plugin\src\perception.rs \| Select-Object -Skip 323 -First 12` | `VoiceInput::next_event` 分两支: 无 backend → `NotImplemented`; 有 backend 未接线 → `BackendNotWired` (带 `when` 字段写明 v2.1) | |
| B2 | 其余"未实现"走**枚举变体 + 注释**: `NotImplemented` 出现 **366** 处; `0 装` 字样出现 **847** 处 (**口径警告**: 这两个数是**原始文本命中数**, 含测试代码、文档注释、SDK 错误类型名; 它证明的是"这套写法在本仓库是主流形态", **不是**"有 366 个未实现功能") | `Select-String -Path (Get-ChildItem crates -Recurse -Filter *.rs).FullName -Pattern 'NotImplemented' \| Measure-Object` / `-Pattern '0 装'` | `366` / `847` | |
| B3 | **唯一全量 stub crate = `apeireth-sdk`**; `STUB_MODE = true` 是编译期硬编码, 且有 `const_assert` 守门 | `Select-String -Path crates\adapters\sdk\src\client.rs -Pattern 'STUB_MODE'` | `client.rs:112: pub const STUB_MODE: bool = true;` + `:129/:131` 编译期守门注释与断言 | |

> **B3 的解读请审核方重点判**: `STUB_MODE` 是**编译期常量**而非运行时开关 ——
> 意味着"接真 HTTP/WS"不是配 env 就能开, 必须**改代码**(见 §5 工作包 W5)。
> 这是设计选择 (阶段 6 明确未开始), 不是 bug; 但任何"SDK 可用"的说法都必须是错的。

### 2.3 生产接线状态 (本轮最容易被误读的一条)

| # | 我的结论 | 复核命令 | 预期输出 | 判定 |
|---|---|---|---|---|
| C1 | 差距文档列的库级模块在**生产装配路径**中引用数为 0: `partner`/`principles`/`consolidation`/`dreaming`/`memory_injection`/`meta_thinking`/`intent_brier`/`reflexion`/`topic_predictor`/`morphology`/`education`/`worktree_sandbox`/`hybrid_search` + 吸收批 4 个 | 见 §3.2 脚本 (对 `runtime-assembly/src` + `cli/src` 逐名计数) | 全部 `0` | |
| C2 | **两个例外**: `context_rot` = **3**, `proactive_recall` = **13** | 同上 | `context_rot 3`, `proactive_recall 13` | |
| C3 | C2 的 `proactive_recall` 是**库级真接线但默认关**: `ProductionModulesConfig.proactive_recall` 类型是 `Option<...>`, `Default` 给 `None` | `Select-String -Path crates\engine\runtime-assembly\src\canonical\production.rs -Pattern 'proactive_recall'` | `:113 pub proactive_recall: Option<ProactiveRecallPolicy>,` / `:151 proactive_recall: None,` / `:347-348 if let Some(policy) = ...` | |
| C4 | 全仓库 `APEIRETH_*` 环境旋钮里**没有** proactive recall 的开关 (即无对外开启手段) | `Select-String -Path (Get-ChildItem crates -Recurse -Filter *.rs).FullName -Pattern '"(APEIRETH_[A-Z0-9_]+)"' -AllMatches` 后去重 | 能力类旋钮只有: `ENABLE_SHELL/FETCH/ORGANS/PREFERENCE_LEARNING`、`DISABLE_LOCAL_READ_TOOLS`、`ENABLE_LOCAL_READ_TOOLS`、`COGNITIVE_JUDGE/COUNCIL`、`GUARD_*`、`MORPHOLOGY_TEMPERATURE`(是温度不是开关) | |

> **⚠️ C1/C2/C3 请审核方重点复核**: 这是"IMPLEMENTED 但未接线"这句话的**唯一硬证据**。
> 我把它记成四级口径的 **IMPLEMENTED ✅ / PRODUCTION WIRED ❌ (proactive_recall: WIRED ✅ /
> DEFAULT ENABLED ❌)**。如果审核方发现别的模块也有非零引用, 必须逐个查明它是
> "类型出现在同一文件里" 还是"真的被构造/注入" —— 后者才算 WIRED。

### 2.4 真缺口 (源码确认不存在)

| # | 我的结论 | 复核命令 (期望 0 命中) | 判定 |
|---|---|---|---|
| D1 | `community` (社群识别与分诊) / `experiment_field` (隔离实验场) / `HybridCognitiveRouter` / `ToolSynthesizer` / `OneRingLedger` 在 `crates/` 无实现 | `Select-String -Path (Get-ChildItem crates -Recurse -Filter *.rs).FullName -Pattern 'community\|experiment_field\|HybridCognitiveRouter\|ToolSynthesizer\|OneRing'` | |
| D2 | `thought_cluster` 按此名 0 命中 (**有** `cluster_store.rs` —— 疑似改名或部分实现, **我没查清**) | 同上 + `Select-String ... -Pattern 'cluster_store'` | |
| D3 | **真文件/网络沙箱未实现**: Linux 侧显式标 `Unsupported` | `Get-Content crates\capabilities\tools\src\process\linux.rs \| Select-Object -Skip 54 -First 18` | `FilesystemIsolation → Unsupported` + `NetworkIsolation → Unsupported`; `PrivilegeReduction → Partial`; 另有 `Enforced` 的两项 = 进程树遏制相关 | |
| D4 | 既有隔离 = **进程树遏制** (Windows Job Object / CREATE_SUSPENDED), **不含**文件与网络 | `Select-String -Path crates\capabilities\tools\src\process\mod.rs -Pattern 'JobObject\|CREATE_SUSPENDED\|IsolationCapability'` | 命中 `process/mod.rs` 与 `windows.rs` | |

### 2.5 被推翻的旧结论 (我先自我更正的部分)

| # | 旧结论 | 我的更正 | 复核命令 | 判定 |
|---|---|---|---|---|
| E1 | "v2 缺 BM25 混合检索" | **错**: `memory/src/hybrid_search.rs` 有 Okapi BM25 + 向量余弦 RRF 真实现 | `Select-String -Path crates\engine\memory\src\hybrid_search.rs -Pattern 'Bm25\|rrf\|k1\|b:'` | |
| E2 | `plugin/src/organ.rs` 写"仅 E4 真实现, 其余 8 organ 0 装" | **陈旧遗留**: engine 侧 9/9 全实装; 我已更正该表并留对账批注 | `Select-String -Path crates\engine\organ\src\lib.rs -Pattern '全实装'` (应命中 `:31`) + `git log -1 --format=%h -- crates/foundation/plugin/src/organ.rs` 之前为 `9ce172a9` | |
| E3 | "perception 5 modality 仅 Text 真" | **基本成立但措辞要收**: 5 modality 的类型与后端抽象都在, 真后端只有 Text; 其余走 Noop fail-closed | `Select-String -Path crates\foundation\plugin\src\perception.rs -Pattern 'NotImplemented'` | |

### 2.6 测试与"什么算验过"

| # | 我的结论 | 复核命令 | 预期输出 | 判定 |
|---|---|---|---|---|
| F1 | `crates/*/*/tests/*.rs` = **92** 个文件 | `(Get-ChildItem crates -Recurse -Filter *.rs \| Where-Object { $_.FullName -match '\\tests\\' }).Count` | `92` | |
| F2 | `#[ignore]` = **47** 处, 全部集中在需真 key/真硬件的 E2E: `engine/organ` 35 / `engine/provider` 7 / `foundation/orchestration` 3 / `engine/memory` 1 / `engine/perception` 1 | `Select-String -Path (Get-ChildItem crates -Recurse -Filter *.rs).FullName -Pattern '#\[ignore'` 后按 `crates/<层>/<crate>` 分组 | 上述五组, 合计 47 | |
| F3 | **推论**: `cargo test` 默认不跑任何真模型用例 | F2 成立即成立 | — | |

> **F1-F3 的实践含义 (审核方请确认这条推论)**: 本仓库的"测试全绿"**不等于**"功能可用"。
> 一切"能跑"的声明, 只能引用 `live-verification-ledger.md` 的绿表; 台账没写的 = 没验过。

### 2.7 复核结论（2026-10-06 复核批填写；按 §2 约定给出判定，为免改动原表汇总于此）

| # | 判定 | 复核批注 |
|---|---|---|
| A1 | ✅ 证实 | 18 members + `version = "2.0.0-rc.1"` |
| A2 | ✅ 证实 | `legacy/` 106 个 Cargo.toml |
| B1 | ⚠️ 部分正确（子结论 ❌ 推翻） | 命中 8 处对，但**全部是文档注释**（`//!`/`///`）；过滤非注释行后**真宏调用 = 0 处**。"真调用 7 处"不成立——正确表述：**0 真调用 / 8 文档提及**（7 在 `sdk/client.rs`，1 在 `plugin/perception.rs:23`）。主结论（SDK 唯一 stub crate、0 装物理形态）**强于**原表述 |
| B1b | ✅ 证实 | `perception.rs:185/:191` 枚举变体 + `:326/:330` 两分支 Err |
| B2 | ✅ 证实 | 366 / 847 逐字吻合 |
| B3 | ✅ 证实 | `client.rs:112` const true + `:131` 编译期断言；另 lark/livekit/sandbox/voice 四个子 SDK 同款镜像守门（多于原文） |
| C1 | ✅ 证实 | 19 模块名在 runtime-assembly/src + cli/src 引用全 0（除 C2 两项） |
| C2 | ✅ 证实 | context_rot=3、proactive_recall=13 逐字吻合 |
| C3 | ✅ 证实 | `production.rs:113` Option / `:151` None / `:347-348` if-let |
| C4 | ⚠️ 部分正确 | 核心结论"无 proactive recall 开关"证实。但"能力类旋钮只有…"漏 `APEIRETH_REASONING_ENABLED` / `APEIRETH_REASONING_MODEL_FILTERS` / `APEIRETH_REASONING_TAG`（reasoning_adapter 真开关）；另 KEYRING_*/DATA_DIR/SESSION_DB/COGNITIVE_DB/MODEL/CONTINUITY_ID/M2B_TEST_ENV 配置类亦未列 |
| D1 | ⚠️ 部分正确 | 实质证实（无实现）；"0 命中"差 1 条——`foundation/protocol:498` 注释提及 VCP `__oneRingMeta`（非实现） |
| D2 | ✅ 证实 | thought_cluster=0、cluster_store=4；关系仍未查清（原文已诚实标注） |
| D3 | ✅ 证实 | Filesystem/NetworkIsolation → Unsupported、PrivilegeReduction → Partial、FileSizeLimit + FailClosedPreExecutionContainment → Enforced |
| D4 | ✅ 证实 | `JobObject|CREATE_SUSPENDED` 32 命中 |
| E1 | ✅ 证实 | Okapi BM25（k1 参数化）+ RRF 真实现 |
| E2 | ✅ 证实 | `organ/lib.rs:31` "9 organ 全实装"；`plugin/organ.rs` 最后改动 `e4f2f451` |
| E3 | ✅ 证实 | 枚举变体 + Noop fail-closed 两分支 |
| F1 | ✅ 证实 | tests/ 92 个 .rs |
| F2 | ✅ 证实 | `#[ignore]` 47：organ 35 / provider 7 / orchestration 3 / memory 1 / perception 1 |
| F3 | ✅ 推论成立 | 基线亲跑复现 **129 suites / 3406 passed / 0 failed / 19 ignored**（与 §3.1 逐字一致） |

> 复核明细、两个安全提交的证据链与 §4 工作包推进记录：`handoff-18crate-ci-fix-2026-09-23.md` §8。

---

## 3. 复核用的可复制命令

### 3.1 基线三连 (先跑, 拿到自己的基线数字)

```powershell
cd C:\Users\31683\Apeireth-rust
cargo test --workspace 2>&1 | Select-String 'test result:'      # 记录 passed/failed 总数
cargo clippy --workspace --all-targets -- -D warnings            # 期望: 0 警告
cd frontend\companion-desktop; npm run check; npm run test        # 期望: 0 errors / 15 passed
```

**我在 `e4f2f451` 上实测的基线 (审核方对照用)**:

| 项 | 数值 |
|---|---|
| 工具链 | `cargo 1.97.1 (c980f4866 2026-06-30)` / `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| `cargo test --workspace` | **129 个 test suite, 3406 passed / 0 failed / 19 ignored** (exit 0) |
| 原始输出留存 | `artifacts/baseline-cargo-test.txt` (**未入库**, 见 §4 说明) |
| `#[ignore]` 总数 | **47 处** (其中 19 处属于会被 `cargo test` 汇总的 suite; 其余在 `src` 内联测试里) |
| 本批改动性质 | **只改文档 + 一处 doc-comment (`plugin/src/organ.rs`)**, 未动任何逻辑 |

> 我这一批未动逻辑, 因此基线应当与 `docs/04-internal/engineering-log-2026-10-06.md`
> 记录的一致。若你的基线数字与我记录的不同, **以你的为准**并回写台账。

### 3.2 生产接线计数脚本 (C1/C2 的完整版)

```powershell
cd C:\Users\31683\Apeireth-rust
$targets = Get-ChildItem crates\engine\runtime-assembly\src, crates\adapters\cli\src -Recurse -Filter *.rs
$mods = @('partner','principles','consolidation','dreaming','memory_injection','meta_thinking',
          'intent_brier','reflexion','topic_predictor','proactive_recall','morphology','education',
          'worktree_sandbox','hybrid_search','betti_hole_detector','residual_pyramid',
          'river_topology','kuramoto_resonance','context_rot')
foreach ($m in $mods) {
  $n = (Select-String -Path $targets.FullName -Pattern $m -SimpleMatch | Measure-Object).Count
  "{0,-22} {1}" -f $m, $n
}
```

### 3.3 真机验证 (需应用已装 + 一个有效 provider key)

```powershell
# 桌面 app 在 C:\Program Files\Apeireth Companion; key 存在 vault 里 (GUI 设置页唯一配置源)
# 无需 key 的回归:
pwsh frontend\companion-desktop\scripts\install-e2e.ps1     # 除"真聊天"外全部步骤可跑
```

---

## 4. 我**没有**验证的东西 (审核方请勿假设我已验)

1. **没有跑真机点击流**: 本批全部结论来自源码与库级测试; 桌面 UI 的人工点击流验收在台账 §2 挂账 #2。
2. **没有真机跑 `--ignored` 的 E2E**: organ live LLM (35 处 ignore)、provider live (7 处) 本批一次都没跑。
   台账绿表 #1-#39 里那些是**历史**跑过的, 不代表在你机器上现在还能跑。
3. **`legacy/` 只做了名称级扫描**: §6 附录的"v1 有 30+ 未编目子系统"是按目录/文件名得出的,
   **未逐个读实现**; 若要按它排期, 必须先读实现确认"v1 真的做成了"。
4. **主账 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` 我没有修订**: §4 只给了建议与裁决,
   原文仍是旧的 🔴 清单 + "🟢 活跃" 标记。**这是已知的文档债, 见工作包 W8。**
5. **`thought_cluster` / `cluster_store.rs` 的关系没查清** (D2)。
6. **`proactive_recall` 之外是否还有"WIRED 但 DEFAULT OFF"的模块没系统排查**:
   我只按差距文档点名的模块扫了一遍。建议审核方做一次**全量反查**:
   把 `runtime-assembly` 里所有 `Option<...>` 配置项与 `Default` 值列出来, 逐个确认默认开还是关。
7. **安全结论的边界**: "shell 非沙箱"是**用户真机实测**得出的 (工程日志 2026-10-06), 我复述;
   shell 在"完全放行"预设下的实际可达范围 (能读哪些路径) 我**没有**实测重放。
8. **`jimmy` remote 是死指针**: `https://github.com/Jimmyxiao2009/Apeireth-rust.git` 返回
   Repository not found。我未删除它 (不确定是否是你的私有/改名仓库)。

---

## 5. 工作包 (审核方推进用, 每个都有验收门)

> **排序原则**: 先做"用户/主人在真机上感知得到"的, 再做"内部质量"的。
> 每个包的验收门都用四级口径写, 不达标不许写"完成"。

### W1 — Shell 沙箱轻量档 (P0, 设计已就绪)

- **现状**: 设计文档 `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md` 已完成三档总览 +
  轻量档设计 (Windows AppContainer + 仅工作区目录授权 + 无网络能力 + 输出凭据触发器 + UI 开关) + 三阶段计划。
- **动工前置**: **需要主人拍板** (设计文档状态 = 待拍板)。审核方**不要**自行开工。
- **验收门**:
  1. 沙箱开 → shell 尝试读 `%USERPROFILE%\.git-credentials` **必须失败** (有测试 + 真机重放);
  2. 沙箱开 → shell 尝试 `curl` 外网 **必须失败**; 模型需联网须显式改走 fetch 工具;
  3. AppContainer 创建失败 → **fail-closed 拒绝执行**, 绝不裸跑;
  4. 沙箱关 → 行为与现状**逐字节一致** (回归测试锁住);
  5. Linux/macOS 在能力报告里 `Unsupported` 如实标注, 不许假装同步支持。

### W2 — 库级模块生产接线批 (P0, 投入产出比最高)

- **现状**: §2.3 C1 列的模块全是 `IMPLEMENTED ✅ / PRODUCTION WIRED ❌`。
- **做法 (每个模块独立成一小步, 不许一次性大爆炸)**:
  1. 先读该模块的 `lib.rs` 头注释 —— 仓库惯例是头注释里写明"真实现的边界在哪、哪些是 0 装";
  2. 找到它在 runtime 侧的落点 (memory recall/writeback、after-turn、context assembly 三类之一);
  3. 加 config 开关 (**默认关**, 除非有明确理由), 加 env 旋钮, 加进 `ProductionModulesConfig`;
  4. **写一条"接线可见性"集成测试**: 在生产装配路径上构造 runtime, 断言该模块的效果**出现在
     行为里** (不是断言"字段 == Some")。
- **验收门**: 每个模块必须有 (a) 默认关闭时的行为不变测试 (b) 打开后的效果测试
  (c) 能力文档四级口径的一行更新 (d) 台账新增一条。
- **建议批内顺序**: `hybrid_search` (检索质量直接可感) → `memory_injection`/`reflexion`/`consolidation`/`dreaming`
  (记忆闭环) → `partner`/`principles` (物种/关系愿景) → `morphology` (已有温度旋钮, 接线成本最低) → 吸收批 4 个。

### W3 — 真缺口批 (P1, 需主人在九项里排序)

- **清单**: `community` / `experiment_field` / `HybridCognitiveRouter` / `ToolSynthesizer` /
  `thought_cluster` (先查清 D2) / `onering` 账本 / 真文件网络沙箱 (与 W1 部分重叠) /
  SDK 真 HTTP·WS (独立成 W5) / 三洋葱 L3-L5。
- **动工前置**: **逐个读 `legacy/donor/` 里的 v1 实现**再排期 —— 本批只做了名称级扫描 (§4.3)。
- **裁决**: **禁止一次性全做**; 一次一个, 一个做完再做下一个。

### W4 — 治理与凭据收口 (P1)

- `credentials/gate.rs` 高危凭据审批门 (现为 trait 口 0 装) 接成真 hook;
- 密钥审计 sink 从 `NoopAudit` 换成真落档 + 去掉 "上层加盐占位" (`credentials/keyring.rs:275`);
- 确认 `colang` / `approval_policy` / `eval` / `evidence` / `rubric` / `risk` 这批
  **default-off helper** 是有意为之 (源码注释如此), 还是漏装 —— **要一个明确裁决并写进文档**。

### W5 — SDK 阶段 6 (P1)

- 把 `STUB_MODE` 从编译期常量改成可配置 + 真接 HTTP/WS; `QuotaStub` 换真 501 语义之外的实现;
  lark/livekit/sandbox/voice 四子 SDK 按优先级逐个接。
- **验收门**: 至少一条**跨进程真调用** E2E (起真 gateway, SDK 客户端打通一轮), 写进台账。

### W6 — 全双工与语音 (P2)

- 8 帧 + barge-in 的类型/控制器/分句**已就位但无真实 WS 传输接线** (`adapters/gateway/duplex_gateway.rs`);
- `panels.rs:1168-1234` 的 `voice.duplex` / `subagents.orchestration` 现返 `not_assembled`。

### W7 — 多签与三洋葱 (P2)

- `core/onion.rs:80-83,156` 的 M-of-N 多签是 **hex 占位签名**, 真 crypto (Ed25519) 标 v2.1;
- 三洋葱 L3-L5 只有数据模型 struct, runtime 走 governance hooks 而非洋葱分层门。

### W8 — 文档治理 (P1, 成本低但影响所有人)

1. 主账 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md`: 加历史批注 + 按四级口径重标 (现在标题还写 "🟢 活跃");
2. 6 份 R11 子文档已带"历史记录"横幅, 逐一核对是否还有漏的;
3. **加一条 CI 或脚本守门**: 文档里出现"已实现/已接线"字样时, 必须同段出现四级口径之一 ——
   这是把"0 装纪律"变成机器可检查信号的唯一办法。

---

## 6. 给审核方的方法论要求 (项目纪律, 不是建议)

1. **双重否定都要防**: 0 装要求"不假装完成", **同样要求"不假装未完成"**。
   本轮的 `plugin/src/organ.rs` 假账就是后者 —— 审核时两个方向都要查。
2. **判决必须对源码**, 不对历史文档。文档写 X、源码是 Y 时, **源码是权威**, 文档按 Y 改。
3. **评审类机制只降级, 不枪毙主任务** (2026-10-06 元层原则): judge/council 预算耗尽、空响应、
   坏 JSON → 降级继续; 显式否决必须用诚实错误码 (`review_rejected` / `turn_not_converged`)。
   新加任何"评审/守门"环节, 必须同时想清楚它的降级路径。
4. **"测试全绿"不是"功能可用"**: 见 §2.6 F3。
5. **不确定就写进"我没有验证"** (§4), 不要用叙述填补空白。

---

## 7. 待主人拍板事项 (审核方请勿代拍)

| # | 事项 | 现状 | 影响 |
|---|---|---|---|
| 1 | **Shell 沙箱轻量档开工** | 设计完成, 待拍板 | 决定 W1 能否启动; 关系到"完全放行"下的安全边界 |
| 2 | 认知深度档 council 顾问数 7 → 3 | 现为 7 (真跑, 延迟较高) | 决定"深度"档延迟; 用户体验取舍 |
| 3 | 九项真缺口 (W3) 的优先序 | 清单已列 | 决定 W3 从哪一项开始 |
| 4 | `jimmy` remote 是否删除 | 死指针 (仓库 not found) | 仓库卫生 |
| 5 | W4 里那批 default-off helper 的裁决 | 源码注释称"有意不装进 pipeline" | 决定它们是"设计"还是"漏装" |
