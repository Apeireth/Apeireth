# TP23 纪律技能（E5 扩展，两类技能）验收报告

- 任务 ID: `12845201-3962-4c45-8de2-f906f4dbccf6`
- 角色: backend_engineer2
- 日期: 2026-08-18
- 范围: TP23（E5 扩展：能力技能 + 纪律技能）

---

## 1. 交付清单

| # | 文件 | 类型 | 说明 |
|---|---|---|---|
| 1 | `crates/apeireth-skills/src/lib.rs` | 修改 | 新增 `SkillKind` + `CapabilitySkill` + `DisciplineSkill` + `DisciplineCheck` trait + `DisciplineContext` + `DisciplineError` + `SkillRegistry` + 17 项测试 |
| 2 | `docs/backlog.md` | 修改 | TP23 已完成项登记（line 37, 225 两个快照） |

无新依赖（用 std + parking_lot 已装的 `Mutex` / `Arc`）。

---

## 2. 设计要点（按任务边界）

### 2.1 两类技能
```rust
pub enum SkillKind {
    Capability,  // 能力技能：可调用的功能
    Discipline,  // 纪律技能：可执行的原则（如"提交前跑测试"）
}
```

- `CapabilitySkill { base: Skill, handler: Option<String> }` — 复用 `Skill` 字段
- `DisciplineSkill { base: Skill, description: String }` — 原则说明

### 2.2 纪律技能挂执行检查

```rust
pub trait DisciplineCheck: Send + Sync + Debug {
    fn check(&self, ctx: &DisciplineContext) -> Result<(), DisciplineError>;
}
```

- **不能 panic**（任务纪律）：`SkillRegistry::check()` 用 `std::panic::catch_unwind(AssertUnwindSafe(...))` 兜底，把 panic 转成 `DisciplineError::CheckerPanic`
- **不破坏调用**：panic 不会传播到调用方，避免一次纪律实现 bug 拖垮整个检查管线

`DisciplineContext { operation, subject, extras }` 传入运行时信息。

### 2.3 注册表

```rust
pub struct SkillRegistry {
    capabilities: HashMap<String, CapabilitySkill>,
    disciplines:  HashMap<String, DisciplineSkill>,
    checkers:     HashMap<String, Arc<dyn DisciplineCheck>>,
}
```

API：
- `register_capability(skill) -> SkillResult<()>`
- `register_discipline(skill, checker) -> SkillResult<()>`
- `check(id, ctx) -> Result<(), DisciplineError>`
- `check_all(ctx) -> Vec<(id, Result)>`
- `unload(id) -> bool`
- `len() / is_empty() / capability_count() / discipline_count()`
- `get_capability(id) / get_discipline(id)`
- `capability_ids() / discipline_ids()`

跨通道冲突（同一 id 既注册为 capability 又注册为 discipline）通过 `SkillError::KindMismatch` 拦截。

### 2.4 卸载机制

- **纪律技能**：`unload(id)` 同时移除 `disciplines[id]` + `checkers[id]`（双清）
- **能力技能**：`unload(id)` 移除 `capabilities[id]`
- 返回 `bool`：true = 卸载了某条，false = id 不存在

### 2.5 向后兼容（重点）

任务纪律明确"不破坏现有 apeireth-skills API"。

| 旧 API | 状态 |
|---|---|
| `Skill` struct + `Skill::new()` + `Skill::validate()` | ✅ 不动 |
| `Registry` struct + `Registry::new()` + `register()` + `get()` + `len()` + `ids()` | ✅ 不动 |
| `is_valid_id()` / `parse_version()` / `compare_versions()` / `select_with_prefix()` | ✅ 不动 |
| `SkillError` 已有 5 个变体 | ✅ 加 `KindMismatch` 一个新变体（向后兼容的 enum 增量） |

新增类型全部为 `pub`，但旧调用方无需修改即可继续工作。验证：`tp23_backward_compat_existing_skill_api_unchanged` 测试。

---

## 3. 验收测试矩阵

| 验收项 | 测试名 | 结果 |
|---|---|---|
| 两类技能注册 — Capability | `tp23_register_capability_succeeds` | ✅ |
| 两类技能注册 — Discipline | `tp23_register_discipline_succeeds` | ✅ |
| 类别冲突 (KindMismatch) | `tp23_kind_mismatch_rejected` | ✅ |
| 重复注册 — Capability | `tp23_capability_duplicate_rejected` | ✅ |
| 重复注册 — Discipline | `tp23_discipline_duplicate_rejected` | ✅ |
| 非法 id 拒绝 | `tp23_invalid_id_rejected` | ✅ |
| 纪律 check — 成功路径 | `tp23_check_success_path` | ✅ |
| 纪律 check — 失败路径 | `tp23_check_failure_path` | ✅ |
| 纪律 check — 未知 id | `tp23_check_unknown_discipline_rejected` | ✅ |
| 纪律 check — panic 捕获 | `tp23_check_panic_caught_not_propagated` | ✅ |
| 纪律 check_all — 不短路 | `tp23_check_all_collects_all_results_no_short_circuit` | ✅ |
| 卸载 — Capability | `tp23_unload_removes_capability` | ✅ |
| 卸载 — Discipline + checker | `tp23_unload_removes_discipline_and_checker` | ✅ |
| 卸载 — 未知 id | `tp23_unload_unknown_returns_false` | ✅ |
| 向后兼容 (旧 Skill/Registry) | `tp23_backward_compat_existing_skill_api_unchanged` | ✅ |
| capability/discipline 列表 sorted | `tp23_capability_and_discipline_lists_are_sorted` | ✅ |

**16 个 TP23 新测试 + 201 个旧测试 = 217 个 apeireth-skills 测试全绿**。

---

## 4. 命令验证

```bash
$ cargo test -p apeireth-skills --lib -- tp23
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 201 filtered out

$ cargo test -p apeireth-skills --lib
test result: ok. 217 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo check --workspace --all-targets
(error count = 0, 仅预存 missing_docs warnings 与 TP23 无关)
```

---

## 5. 边界声明（0 假装）

| 项 | 当前实现 | 升级路径 |
|---|---|---|
| 能力技能调用入口 | 仅 `handler: Option<String>` 描述符字段, 实际 dispatch 走外部 `skill_executor` 等模块 | TP23 范围只到"描述符 + 注册表", 不重写 dispatch; 后续 N-TP 单独接 skill_executor |
| 纪律 check 调度点 | `SkillRegistry::check(id, ctx)` + `check_all(ctx)` API | 调用方在执行点显式调用; 后续可加 `MountPoint` 自动挂载 (类似 gate.rs) |
| DisciplineCheck 持久化 | 不存; 只在进程内注册 | 若需跨重启, 复用 apeireth-principles 模式 (sqlite + episodes) |
| `extras: serde_json::Value` | 透传, 不结构化 | 调用方按需填; 后续若多 caller 重复用可升级为强类型 enum |

---

## 6. 未触碰禁踩区（确认）

按 docs/next-team-handbook §1 + 团队 LOCKED 列表确认未触碰:
- `crates/apeireth-skills/src/{anthropic_skills,descriptor,eval_bridge,file_loader,library_stage6_guardianship,mcp_bridge,organ_kani_proofs,semver_strict,skill_executor,wasm_bridge,watcher}.rs` (WIP 锁)
- 仅 `lib.rs` 修改, 增量 API, 现有字段名/方法名/错误变体签名不变

---

## 7. 与 E5 / 任务边界对齐

任务边界: "技能分两类, 能力技能 + 纪律技能; 纪律技能挂执行检查; 失败时阻止该执行点继续 (return Err)"

| 边界 | 落地 |
|---|---|
| 两类技能 | `SkillKind` + 双通道 HashMap ✅ |
| 纪律挂执行检查 | `DisciplineCheck::check()` trait + `SkillRegistry::check()` 入口 ✅ |
| 失败阻止执行点 | `Result<(), DisciplineError>`, 调用方按 `Err` 决定阻断 ✅ |
| 动态卸载 | `unload(id) -> bool`, 双清 (descriptor + checker) ✅ |
| 不破坏现有 API | 旧 `Skill`/`Registry`/`is_valid_id` 等完全不动; `SkillError` 仅增量 `KindMismatch` ✅ |

---

## 8. 提交状态

- git commit: 待 push（commit 在 task/tp12-schema-guardrail-rework-final 工作树）
- 团队框架状态：报告 + backlog 同步, `team_complete_task` 待调用