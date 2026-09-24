# Post-Integration Architecture Debts & Audits（2026-09-24）

> Mesh Phase 1A 善后批：只记录、不重写。每项标注 `changed / documented only / left untouched` 与原因。
> 范围守卫（§25）：comments / docs / 小一致性；本轮不做 Nightwatch 架构迁移、能力中心 schema 重写、memory 重设计。

---

## 1. Date / provenance audit（§19）

**结论：系统性前向日期戳族，全部 left untouched + 本补正记录。**

| 项 | 事实 |
| --- | --- |
| A. git commit timestamps | 全部实际提交时间均在 2026-09-24（`4774d8b8`…`b4115f6b` 同日） |
| B. source comments 含未来日期 | `2026-09-25`×2 · `09-26`×2 · `09-28`×6 · `09-30`×60 · `10-01`×2 · `10-02`×1 · `10-05`×1 · `10-06`×140 · `10-07`×1 · `10-10`×148（.rs 约 92 行含 10-10，含注释行内多日期） |
| C. docs 文件名含未来日期 | 7 个（`*-2026-10-10.md` / `*-2026-10-06.md` 等） |
| D. ledger 验证条目含未来日期 | `live-verification-ledger.md` 23 处 |
| E. 性质判定 | **work-log 时间戳族的系统性前向漂移**（或按计划跨度落款）——不是单点笔误 |

E 的判定依据：同一作者群在 2026-09-24 的安全修复轮里用的是正确日期（如 `normalized.rs` 的 "M11 修复 (2026-09-24 审计)"），而 W 批注释系统性地落在 09-25…10-10；两种落款并存，无法在不误伤"计划里程碑/文档版本名"读法的前提下全局改写。

**处置：**
- 源码/文档/文件名中的未来日期：**left untouched**（`明确 accidental` 判定不成立；§19 明令 不直接全局 replace）。
- `live-verification-ledger.md`：**append-only 纪律下不改写任何历史行**；仅在文末追加补正声明（见该文件 §5）。
- 未来若要收敛：应先约定统一落款规范（真实提交日 vs 计划里程碑分栏），再逐批更正；不要在 cleanup 轮里做。

## 2. Nightwatch lifecycle debt（§20）

```text
Nightwatch long-running lifecycle should eventually be represented as a
system Module/service rather than a CLI-owned daemon.
```

- 现状：`apeireth nightwatch --watch` 由 CLI 自持长循环（`adapters/cli/src/main.rs::run_nightwatch_watch`）+ `NightwatchIdleScheduler` 钩子。
- 三个审计问题的回答：
  1. **Does CLI own long-running service lifecycle?** 是——守护循环活在 CLI 进程里。
  2. **Could it already be represented as a Module?** 可以（hook 观察 + 生命周期完全在 Module ABI 表达力内），但需要一个宿主侧的服务生命周期容器。
  3. **Does moving it now materially simplify runtime?** 否——会新造 SchedulerManager/ServiceRegistry/DaemonModule 一类抽象（§25 明令禁止）。
- **处置：documented only**（本节 + `run_nightwatch_watch` 处一条 TODO 注释）。本轮默认不迁移，理由如上。

## 3. Nightwatch activity semantic debt（§23）

```text
device idle != session idle != continuity/global idle
```

- 现状：空闲判定以 episode timestamp 作为用户活跃代理（单机语义合理）。
- Mesh 引入后的真实风险：**误把某节点 idle 当整个用户 idle**（Laptop 静默 ≠ 用户离开，可能只是活在 Workstation 上）。
- **处置：documented only**（本节即 debt/TODO）。求解属于未来 Mesh 活性语义批，本轮不解决。

## 4. Capability Center metadata debt（§21）

- 现状：后端真相 = `ModuleManifest` / `CapabilityDescriptor` / `ModuleOrigin` / metadata；前端 `CapDef` / `BETA_DEFS` / `CapabilityToggles` / `SettingsView` 是**手工重复的产品目录**——双描述并存。
- 代价：每新增一个 capability 要走手工五连：`Rust field → TS field → parser → defaults → CapDef/SettingsView`。
- 未来方向（记录，不在本轮实施）：

```text
Module/Capability metadata
        ↓
runtime capability schema/snapshot
        ↓
frontend renderer
```

- **处置：documented only**。manifest-driven UI 大迁移明确不做；如需低风险准备，可先给 `CapabilityDescriptor::metadata` 约定 schema 键——但本轮连这也暂缓（cleanup 让路）。

## 5. community_triage consistency（§22）

一致性审计五项**全部正确**，按令零改动：

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| default-off | ✓ | `ProductionModulesConfig::default` + 测试 `config_defaults_to_community_triage_off` |
| no graph handle = no wiring | ✓ | `production.rs`：`if let Some(graph) = &backends.graph` 才 `with_community_triage` |
| all_facts default implementation safe | ✓ | trait 默认返回空 `Vec`；sqlite impl 真查询 + roundtrip 测试 |
| hook declaration matches TurnStart | ✓ | `MemoryRecallModule` declares `TurnStart`(+`AfterTurn`)，消费在 TurnStart 检索前置 |
| Module ownership unaffected | ✓ | 消费经 `KnowledgeGraphStore` 端口，capability 索引零变化 |

## 6. Mesh lock-discipline consistency（本轮新增的一条小债）

`engine/mesh` 的 `Mutex`/`RwLock` 访问点仍用 `.expect("…poisoned")`，与 2026-09-24 安全轮确立的 poison 容错标准（`unwrap_or_else(into_inner)`，L/G 组）不一致。属 §25 允许的 small API consistency，但为不让 cleanup 触碰 Mesh 运行时 diff（§30 拆分守卫），**本轮 left untouched**，挂账于此。
