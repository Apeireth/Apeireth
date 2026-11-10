[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-engine-e2e-smoke-2026-11-08.md
Class: EVIDENCE
Status: 🟢 已核验（本地门禁全绿；反证三次**实测红**）
Last-Modified: 2026-11-08
Author: impact-analyzer

# 记忆引擎端到端冒烟（2026-11-08）

> **口径**：本页只报**实测过**的东西（跑过的命令 + 真实输出）。
> 没跑过 / 没证明的部分在 §五**逐条列出来**，不混进结论。
>
> **四级口径**：`PRODUCTION WIRED` —— 本页的链路结论是**在生产装配路径上实测**的
> （装配根 → 记忆装配 → 写入面 → 召回面 → 注入渲染 → 提示词）。
> **不**宣称 `HARDWARE VALIDATED`（无真机、无远端 CI）。

**分支** `integration/consolidate-2026-10-31`（工作树 `/home/bai/wt-apeireth-r3`，**未提交**）。
**新增测试** `crates/engine/runtime-assembly/tests/memory_engine_e2e.rs`（9 条用例，全绿）。
**新增可观测面** `crates/engine/runtime-assembly/src/canonical/memory_contract_wire.rs`
（只读计数器 `contract_install_count()`，**行中性**：128 行 → 128 行）。

---

## 一、一句话结论

**「写入记忆 → 召回 → 注入提示词 → 模型看得见证据」这条链，在生产装配路径上整条跑通了**，
并且断言的是**注入文本里真的出现了那条记忆的内容**（不是「函数返回了 `Ok`」）。

配套三件事也一起钉住了：

1. **预算纪律**：注入长度**不超**硬上限（`DEFAULT_MAX_CONTEXT_CHARS = 4000`）；
2. **契约面确实被经过**：生产装配那一行**真的**过了契约门（只读计数会涨）；
3. **默认态零副作用**：影子/换算法**未配置** ⇒ 不构造任何东西、数据根**零新增文件**、
   产物与「未接线」的裸协调器**逐字相同**。

---

## 二、真实链路图（每步 file:line）

> 全部落点取自**当前工作树**（`integration/consolidate-2026-10-31`），不是从设计稿抄的。
> ⚠️ **行号会漂**：记忆 crate 里有三条并行线在改 ⇒ 引用以**符号名**为准，行号是本批快照。

| 步骤 | 落点 | 说明 |
|---|---|---|
| 装配根 | `crates/engine/runtime-assembly/src/canonical/production.rs:773` | `ProductionModules::build` |
| 记忆装配 | `canonical/production.rs:1433` → `canonical/memory_assembly.rs:104` | `memory_assembly::assemble_memory` |
| 契约口径自检 | `canonical/memory_assembly.rs:109` → `canonical/memory_contract_wire.rs:62`（`install`）→ `crates/engine/memory/src/canonical/memory_contracts.rs:260`（`assert_scope_alignment`） | `install()` → `assert_scope_alignment()`（5 × 5 = **25** 组范围组合） |
| 排序旋钮配置面 | `canonical/memory_assembly.rs:111` → `:126` | `apply_retrieval_knobs_at`（缺文件 ⇒ 不施加；坏文件 ⇒ **拒绝装配**） |
| 影子双跑 / 换算法 | `canonical/memory_assembly.rs:116` → `canonical/shadow_wire.rs:574` | `install_shadow`（缺配置 ⇒ **不构造任何东西**） |
| 召回模块注册 | `canonical/production.rs:1492-1495` | `MemoryRecallModule::new(...).with_coordinator(...)` |
| 写回模块注册 | `canonical/production.rs:1691-1697` | `MemoryWritebackModule::new(...).with_coordinator(...)` |
| **写入面** | `canonical/cognitive.rs:2129` → `crates/engine/memory/src/coordinator/owned.rs:4` → `crates/engine/memory/src/coordinator.rs:1121` | `writeback_scoped_episode` → `persist_episode` → `put_episode` + `put_episode_metadata` |
| **召回面** | `canonical/cognitive.rs:1550` → `crates/engine/memory/src/coordinator.rs:1164` → `:742` → `:383` → `:903` | `compile_prompt_overlay_with_selected_access` → `recall` → `collect_candidates` → `execute_hybrid_retrieval` |
| **读契约** | `crates/engine/memory/src/context_compiler.rs:167`（`contract_view`）→ `crates/engine/memory/src/canonical/memory_contracts.rs:76`（`SelectedMemoryView::build`） | 读契约 + 纯数据标注；两处调用点 `context_compiler.rs:237` / `:445` |
| **注入渲染** | `crates/engine/memory/src/coordinator.rs:1174`（`compile_selected`）→ `crates/engine/memory/src/context_compiler.rs:346` → `:404` → `:436`（`compile_injection_with_renderer`）→ `:576`（`render_through_contract`） | 注入契约渲染器（`NumberedEvidenceRenderer`，与 `memory_injection.rs:46` 的编号清单**逐字同形**） |
| **进提示词** | `canonical/cognitive.rs:1584` | `PromptOverlay::system_cross_source("memory_recall", overlay)` ⇒ 宿主把跨源内容经披露封套放进**工具结果**通道（模型看得见） |

---

## 三、测试清单（9 条，全绿）

> 命令：`cargo test -p apeireth-runtime-assembly --test memory_engine_e2e --offline`
> 结果：`9 passed; 0 failed`

| 用例 | 断言什么 |
|---|---|
| `the_production_chain_writes_recalls_and_injects_the_evidence` | ⭐ 生产装配 → 生产写回模块落库 → 生产召回模块召回 → **注入里真的出现记忆正文** + 闭世界块形态 + **≤ 4000 字** + **契约门计数上涨** |
| `the_injection_format_also_carries_the_evidence_through_the_contract_renderer` | ⭐ 注入形态（`compile_injection_with_renderer` 那一支）同样带证据 + 编号清单 + 反幻觉规则 + 预算纪律 |
| `the_model_actually_receives_the_recalled_evidence_in_its_request` | ⭐ **模型请求里**真的有那条记忆（跑两轮真实回合），且**不许**出现在系统通道 |
| `cross_session_and_multi_subject_scopes_are_respected` | ⭐ 跨会话（全局范围）+ 不同主体：可见的召回、**不可见的召不到**（范围纪律） |
| `the_default_state_has_no_shadow_side_effects_and_is_byte_identical` | ⭐ 影子面未配置 ⇒ `config_from_data_root` 为 `None` + 进程槽为空 + 数据根**零新增文件** + 产物与裸协调器**逐字相同** |
| `a_broken_configuration_refuses_to_assemble` | ⭐ **失败闭合**：坏 `retrieval.json` / 坏 `shadow.json` ⇒ 装配**必须失败**（不许静默继续） |
| `the_evidence_assertion_bites_when_the_injection_renderer_returns_empty` | **反证 ①**：渲染器返回空 ⇒ 核心谓词为假 |
| `the_evidence_assertion_bites_when_the_contract_view_is_not_granted` | **反证 ②**：契约视图未授权（fail-closed ⇒ 空）⇒ 条目到不了编译器 |
| `the_evidence_assertion_bites_when_the_memory_was_never_written` | **反证 ③**：根本没写 ⇒ 核心谓词为假 |

**核心判定谓词**（主用例与三条反证共用同一条 ⇒ 反证里它为假，就等于主断言会红）：

```rust
fn evidence_present(overlay: &str, canary: &str) -> bool { overlay.contains(canary) }
```

---

## 四、反证：**实测红**的三次突变

> 纪律：不做 `git checkout` / `restore` / `reset` / `clean` / `stash`；
> 每次突变都**先备份到 `/tmp`**、跑完**按字节还原**（`diff` 校验一致）。

### M1 —— 把**契约门**短路（改 `memory_contract_wire.rs`，允许动的文件）

突变：`install()` 恒返回 `Err("反证突变：契约可见性口径与 canonical 分叉")`。

```
---- the_production_chain_writes_recalls_and_injects_the_evidence stdout ----
panicked at crates/engine/runtime-assembly/tests/memory_engine_e2e.rs:143:6:
生产装配必须成功（契约口径分叉 / 配置坏 ⇒ 拒绝装配）: Misconfigured("反证突变：契约可见性口径与 canonical 分叉")
test result: FAILED. 0 passed; 1 failed
```

⇒ **证明了**：契约面**真的**在生产装配路径上（不是摆设），且它**失败闭合**（分叉 ⇒ 进程起不来）。

### M3 —— 把**写入面**短路（改本测试文件）

突变：写进库的内容不再含探针串（问句仍含探针串）。

```
panicked at crates/engine/runtime-assembly/tests/memory_engine_e2e.rs:282:5:
生产写入面必须真的落库；实际写入 = ["好的, 已记下", "交付窗口是周三下午, 别改时间"]
test result: FAILED. 0 passed; 1 failed
```

⇒ **证明了**：注入里的证据**来自库里那条记忆**，不是把问句抄回去。

### M3b —— 把**召回面**短路（改本测试文件）：⭐ 主断言自己红

突变：召回换成**另一个会话**去问（范围纪律挡住 ⇒ 召不到）。

```
panicked at crates/engine/runtime-assembly/tests/memory_engine_e2e.rs:305:5:
注入里必须真的出现记忆正文；注入 =
test result: FAILED. 0 passed; 1 failed
```

⇒ **证明了**：核心断言**咬住的是整条链**（写入 → 召回 → 注入）；断一环 ⇒ 断言立刻红。

**三次突变全部按字节还原**（`diff` 与备份一致），还原后 `9 passed`。

---

## 五、**边界**：**没证明 / 没测**的部分（逐条，如实）

1. **「读契约在生产读路径上被经过」只有代码级证据 + 行为级反证，没有运行时计数**。
   原因：`ClosedWorldContextCompiler::contract_view`（`context_compiler.rs:150`）在**记忆 crate** 内，
   协调器不暴露编译器 ⇒ 外面读不到它的计数；
   本次只加了一个**装配层**的只读计数（`contract_install_count()`）⇒ 它证明的是
   **装配真的过了契约门**，**不是**「读路径建过视图」。
   读契约在链路上这件事目前靠：`context_compiler.rs` 的 `contract_view` 两处调用点
   （`compile_with_selected_access` 与 `compile_injection_with_renderer`，代码事实）
   ＋ §四 反证 ②（契约视图 fail-closed ⇒ 条目到不了编译器）。
2. **契约口径「分叉 ⇒ 拒绝装配」是突变模拟的，不是真的分叉过**。
   真分叉需要改 `crates/engine/memory/src/**`（本批**明令不许动**，三条并行线在写）
   ⇒ 本页用 M1 突变证明「这道门会咬人」，**没有**在真实分叉状态下跑过一次。
3. **跨会话召回靠 `scoped_memory` 后端**（`SqliteMemoryStore` 的 `ScopedMemoryBackend` 实现）：
   默认生产装配若**不**接 `scoped_memory`，召回面只读**当前会话分区**
   ⇒ 本页的跨会话断言在「**接了** scoped 后端」这一前提下成立（生产装配的 `backends.scoped_memory` 槽）。
4. **影子/换算法的「配置后」行为没测**（本批只测**未配置**的默认态）：
   「影子侧绝不改生产结果」「切换要过治理面」这些**库级/装配级**性质由既有单测覆盖，本页不重复声明。
5. **端到端只跑了两个回合**（写一轮 + 问一轮）⇒ 多轮交错、去重门、整编节奏**未**被这条冒烟覆盖。
6. **注入形态的「锚定选段」只被间接覆盖**（探针串在正文开头）⇒ 主体在正文**中后段**时的选段行为，本页**未**证明。
7. **没有跑远端 CI**：本页所有结论只到「本地门禁全绿」这一层。

---

## 六、门禁自查（本地实测）

| 门禁 | 结果 |
|---|---|
| `cargo fmt -p apeireth-runtime-assembly -- --check` | ✅ 通过（先 `cargo fmt` 落盘，再 `--check` 干净） |
| `cargo clippy -p apeireth-runtime-assembly --all-targets --offline -- -D warnings` | ✅ exit 0 |
| `cargo test -p apeireth-runtime-assembly --test memory_engine_e2e` | ✅ **9 passed / 0 failed** |
| `cargo test -p apeireth-runtime-assembly --lib` | ✅ **429 passed / 0 failed** |
| `python3 scripts/arch_guard.py --strict` | ✅ **本批测量时 `VIOLATIONS: 0`**（edges=111 · env=150 · max_src_loc=6255 · 例外 2 条为既有）<br>⚠️ 复测时另有并行线（CLI 线）把 `crates/adapters/cli/src/main.rs` 撑大 4 行 ⇒ 现在报 **1 条 `[W1] size-grow`**，**与本批无关**（本批未碰 `crates/adapters/cli/**`，且 `memory_contract_wire.rs` 仍是 **128 行**、不进尺寸棘轮） |
| `python3 scripts/audit_unwired.py --check` | ✅ 「孤儿 API 基线一致（86 条，无新增、无过期）」 |
| `python3 scripts/check_doc_class.py --check` | ✅ 「文档分类守卫通过（活跃 575 份 · 已分类 41 · 基线待分类 534）」 |

> ⚠️ `docs/INDEX.csv` 由主指挥统一重生成 ⇒ 本批**没有**跑 `docs_index --gen`。

---

## 七、复现命令

```sh
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= \
  bash -c 'cd /home/bai/wt-apeireth-r3 && cargo test -p apeireth-runtime-assembly --test memory_engine_e2e --offline'
```

---

## 八、相关文件

| 文件 | 作用 |
|---|---|
| `crates/engine/runtime-assembly/tests/memory_engine_e2e.rs` | 本批新增：端到端冒烟（9 条用例，含 3 条反证） |
| `crates/engine/runtime-assembly/src/canonical/memory_contract_wire.rs` | 只读计数器 `contract_install_count()`（行中性） |
| `crates/engine/memory/src/context_compiler.rs` | 读契约接线点（`contract_view`） |
| `crates/engine/memory/src/canonical/memory_contracts.rs` | 三契约与口径对齐自检 |
| `crates/engine/runtime-assembly/src/canonical/memory_assembly.rs` | 生产记忆装配（契约门 + 旋钮 + 影子） |
