# R125-14 Sub-Agent Dispatch Prompt (obra/superpowers Skill 化工作流)

**Date**: 2026-08-10 17:35
**Author**: R125 P2 supervisor (general agent, mvs_a7af0f1f15cd4a79901442e14878333d, dispatched 17:23)
**Receiving agent**: R125-14 sub-agent (Mavis 派)

---

## 任务 (per 主人 17:22 升级授权 + decision-33 + B1 实施可改)

**主题**: obra/superpowers Skill 化工作流借鉴到 `apeireth-central`. 20+ 预定义 Skill 文件为 AI 注入行为准则, TDD 强制化, Skill trait + 注册表.

**借鉴 ID**: `R124-2-BORROW-obra/superpowers-2026-05-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\superpowers\`

**目标文件**:
- `Apeireth-rust/crates/apeireth-central/src/skill_trait.rs` (NEW trait, ~200-300 行)
- `Apeireth-rust/crates/apeireth-central/src/skill_registry.rs` (NEW 注册表, ~150 行)
- `Apeireth-rust/crates/apeireth-central/src/lib.rs` (M: add `pub mod skill_trait;` + `pub mod skill_registry;`)
- `Apeireth-rust/crates/apeireth-central/examples/skill_demo.rs` (NEW example)
- `Apeireth-rust/crates/apeireth-central/tests/skill_test.rs` (8 unit tests, NEW)
- `Apeireth-rust/crates/apeireth-central/skills/` (NEW dir, 20+ Skill .md 文件)

**整合依赖**: apeireth-central 不在 24 LOCKED, 实施可改. 借鉴 superpowers 模式 (Skill = Markdown + 行为准则 + TDD 强制).

**估时**: 1-2 天 (skill_trait + skill_registry + 20+ Skill .md + 8 test).

**截止**: 8/12 17:30 (跑过夜 8/11-8/12).

---

## 0 装解除 (主人 17:22) — 重要

**借鉴源码状态** (verify 实施前):
```bash
Test-Path '.openclaw\workspace\borrowed-repos\superpowers\.git'  # 必须 True
```

**3 种状态对应动作**:
1. ✅ **cloned** (`.git` 存在) = 真实施, 报告里写 "借鉴源码 ✅ cloned, 已实施"
2. ⏳ **限流中** (`.git` 0 存在) = 等 30 min 再 verify, 仍 0 实施, 报告里写 "借鉴源码 ⏳ 限流中, 0 实施, 借鉴 ID 索引完成"
3. ❌ **永久失败** (24h 后仍 0 cloned) = 报 supervisor + 取消任务, 0 假装"已借鉴"

**0 装 PASS 严守**: ❌ 0 假装"已借鉴", ❌ 0 写 src 假装 import 借鉴代码, ❌ 0 写 Skill .md 假装"已应用" superpowers 模式.

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

| # | 硬墙 | 你 (R125-14) 必守 |
|---|------|-----------------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 你 0 再升) | ✅ 0 触碰 `Cargo.toml` `version` 字段 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | ✅ 0 触碰 `integration_r_measure.rs` |
| 3 | **B1** 24 LOCKED crate mtime (apeireth-central **不在 24 LOCKED**, 实施可改) | ✅ 0 触碰 24 LOCKED crate mtime |
| 4 | **B5** 6→8 哲学锚 (R125 末升) | ✅ 0 改 6 哲学锚原 6 实质, 8 锚是扩展 |
| 5 | **B3** V0.5 25 维 (R125 末升) | ✅ 0 改 V0.5 公式, 25 维是扩展 |
| 6 | **B4** 6 重守门 v6 (R125-5 实施) | ✅ 0 改 5 重原 5 重, 6 重是扩展 |
| 7 | **A3** 12→13 键 (R125-12 后 PHL-07) | ✅ 0 改 12 键原 12, 13 键是扩展 |
| 8 | **C1** 0 主动 commit (你 sub-agent 0 commit) + **C2** 0 装 解除 (主人 17:22) + **C3** 0 装 5 项 升 6 重 v6 + 0 主动 push 严守 | ✅ 0 commit, 0 push, 借鉴源码 ✅ cloned 才真实施 |

**新增 mod 0 触碰 workspace.version**: apeireth-central 自身 Cargo.toml 是 `version.workspace = true`, 你 0 触碰 workspace root.

**apeireth-central 不在 24 LOCKED** (per `docs/omnibus/24-locked-crates.md`):
- ✅ 0 触碰 24 LOCKED crate mtime
- 🟢 apeireth-central/src/lib.rs 0 改原 lib.rs 实质, 仅加 2 行 pub mod 声明
- 🟢 skill_trait.rs / skill_registry.rs / skills/ 全是新文件

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴源码 study (30 min)
```bash
# verify cloned
Test-Path '.openclaw\workspace\borrowed-repos\superpowers\.git'
# 读 superpowers 核心: skills/ + docs/ + .claude/commands/ + README.md
Get-ChildItem '.openclaw\workspace\borrowed-repos\superpowers\skills' -ErrorAction SilentlyContinue | Select-Object Name
```
提取 3 个核心 pattern:
1. **Skill = Markdown 行为准则**: 每个 Skill 是一个 SKILL.md, 定义"什么时候用 + 怎么做"
2. **TDD 强制**: Skill 包含 test-first 步骤, AI 必须先写 test 再写实现
3. **Skill 注册表**: 中央注册 + 加载机制, AI 启动时加载相关 Skill

### 阶段 2: Rust 实施 (3-4 hours, skill_trait + registry + 20+ Skill .md)
**skill_trait.rs** (NEW trait):
```rust
//! Skill trait — 借鉴 obra/superpowers Skill 化工作流 (R125-14)
//!
//! 20+ 预定义 Skill 为 AI 注入行为准则, TDD 强制化.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SkillId { TddFirst, CodeReview, DocLookup, ErrorHandling, /* ... 20+ */ }

pub trait Skill: Send + Sync {
    fn id(&self) -> SkillId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn when_to_use(&self) -> &'static str;
    fn steps(&self) -> Vec<String>;
    fn tdd_required(&self) -> bool { true }  // 默认 TDD 强制
    fn markdown_path(&self) -> &'static str;
    fn load_markdown(&self) -> String { /* 读 skills/{id}.md */ }
}

pub struct TddFirstSkill;
pub struct CodeReviewSkill;
pub struct DocLookupSkill;
pub struct ErrorHandlingSkill;
// ... 20+ Skill struct

impl Skill for TddFirstSkill { /* ... */ }
// ... 20+ impl
```

**skill_registry.rs** (NEW 注册表):
```rust
//! Skill 注册表 — 借鉴 obra/superpowers 中央注册模式 (R125-14)

use std::collections::HashMap;
use std::sync::Arc;

pub struct SkillRegistry { skills: HashMap<SkillId, Arc<dyn Skill>> }
impl SkillRegistry {
    pub fn new() -> Self {
        let mut r = Self { skills: HashMap::new() };
        r.register(Arc::new(TddFirstSkill));
        r.register(Arc::new(CodeReviewSkill));
        // ... 20+
        r
    }
    pub fn register(&mut self, skill: Arc<dyn Skill>);
    pub fn get(&self, id: SkillId) -> Option<Arc<dyn Skill>>;
    pub fn all(&self) -> Vec<Arc<dyn Skill>>;
    pub fn tdd_required(&self, id: SkillId) -> bool;
}
```

**lib.rs 修改** (2 行新 mod 声明):
```rust
// apeireth-central/src/lib.rs 末尾加:
pub mod skill_trait;
pub mod skill_registry;
pub use skill_trait::{Skill, SkillId};
pub use skill_registry::SkillRegistry;
```

**skills/** (NEW dir, 20+ Skill .md):
```
crates/apeireth-central/skills/
├── tdd-first.md       — 什么时候: 写新功能前. 步骤: 1) 写失败 test 2) 跑 test 失败 3) 写最小实现 4) 跑 test pass
├── code-review.md     — 什么时候: PR / 改动后. 步骤: 1) 读 diff 2) 检查 8 硬墙 3) 检查 0 装 4) 反馈
├── doc-lookup.md      — 什么时候: 遇到未知概念. 步骤: 1) 查 docs/conventions/ 2) 查 omnibus 3) 查 GitHub 4) 反馈
├── error-handling.md  — 什么时候: 任何 fn 可能 Err. 步骤: 1) Result<T, E> + thiserror 2) 0 panic 3) 错误信息含 context
├── refactor-scan.md   — 什么时候: 每周 / R-阶段. 步骤: 1) cargo clippy 2) cargo doc 3) cargo udeps 4) 报告
├── test-driven.md     — 什么时候: 任何 lib 改动. 步骤: 1) 写 test 2) 跑 3) 改 src 4) 跑
├── tdd-rgr.md         — 什么时候: 修复 bug. 步骤: 1) 写重现 test 2) 看 fail 3) 修 4) 看 pass 5) refactor
├── api-design.md      — 什么时候: 设计新 API. 步骤: 1) 公共 API ≤ 11 件套 2) 0 改 LOCKED 入口 3) doc 必填
├── locked-audit.md    — 什么时候: 改 LOCKED 附近. 步骤: 1) 查 24-locked-crates 2) 查 8-locked-unified 3) 0 改 mtime
├── gate-verify.md     — 什么时候: 任何 src 改动. 步骤: 1) cargo test 2) cargo clippy 3) cargo doc 4) verify
├── version-bump.md    — 什么时候: 改 workspace.version. 步骤: 1) 查 decision 2) semver 严守 3) 0 改 0 假装
├── tla-invariants.md  — 什么时候: 改状态机 / 并发. 步骤: 1) 写 TLA+ 不变量 2) 跑 TLC 3) verify
├── kani-harness.md    — 什么时候: 加新 LOCKED. 步骤: 1) 写 POD 模型 2) 写 harness 3) cargo kani 4) verify
├── multi-eval.md      — 什么时候: AI 改输出. 步骤: 1) 7 评估维度 2) 4 集成场景 3) 0 漂移 4) 报告
├── rollback-plan.md   — 什么时候: 任何 src 改动. 步骤: 1) git diff 2) 写 revert script 3) 测 4) commit 后保留
├── decision-log.md    — 什么时候: 关键决策. 步骤: 1) 写 decision-N 2) 引用 spec 3) 标 0 装 4) commit 后保留
├── changelog.md       — 什么时候: 任何 R-阶段末. 步骤: 1) 写 CHANGELOG 2) 引用 commit 3) 0 漂移
├── ci-pipeline.md     — 什么时候: 改 .github/. 步骤: 1) 查 R120 D 2) 矩阵化 3) 测本地 act 4) PR 后保留
├── 0-fake.md          — 什么时候: 任何报告. 步骤: 1) 真实标数 2) 0 假装"已借鉴" 3) 0 假装"已修" 4) commit 前核
└── supervisor-rule.md — 什么时候: supervisor 派活. 步骤: 1) 5 min tick 2) 卡 30 min kill 3) 派替代 4) 报告
```

### 阶段 3: 8 smoke test (30 min)
- `test_skill_trait_define` — Skill trait 完整, 20+ impl
- `test_skill_registry_new` — SkillRegistry::new() 注册 20+ skills
- `test_skill_registry_get` — get by id
- `test_skill_registry_tdd_required` — TDD 强制化
- `test_skill_load_markdown` — 读 skills/{id}.md 0 失败
- `test_skill_tdd_first_steps` — TddFirst skill 5 步骤
- `test_skill_code_review_steps` — CodeReview skill 4 步骤
- `test_skill_registry_all` — all() 列 20+ skills

### 阶段 4: example + final 报告 (30 min)
- `examples/skill_demo.rs` — 加载 3 skill + 展示 TDD 流程
- final 报告: `Apeireth-rust/reports/agent-r125-14-final-2026-08-10.md`

---

## 0 主动 commit (C1 严守)

❌ **你 (R125-14 sub-agent) 0 commit, 0 push**. 实施完成 = 写 src/skills/test/ + 写 final 报告. Mavis 整合 #3 拍板 17:30 (0 含 R125 实施, R125 续 mavis 整合 commit 链 8/15-9/10).

---

## final 报告 必含 6 段

```markdown
# R125-14 Final Report — obra/superpowers Skill 化
**Date**: 2026-08-10
**Author**: R125-14 sub-agent
**借鉴 ID**: R124-2-BORROW-obra/superpowers-2026-05-2026-08-10
**实施路径**: crates/apeireth-central/src/{skill_trait,skill_registry}.rs (NEW) + crates/apeireth-central/skills/ (NEW dir, 20+ .md)

## 1. 借鉴源码状态 (0 装解除 verify)
- ✅ cloned / ⏳ 限流中 / ❌ 永久失败 (3 选 1)

## 2. 实施步骤
- 阶段 1 借鉴 study: (3 提取 pattern: Skill=Markdown / TDD 强制 / Skill 注册表)
- 阶段 2 Rust 实施: (skill_trait.rs 20+ struct impl + skill_registry.rs 中央注册 + skills/ 20+ .md)
- 阶段 3 smoke test: (8 test pass/fail)
- 阶段 4 example + 报告: (skill_demo.rs + final)

## 3. 8 硬墙 verify (B1-B7 + A1-A3 + C1-C3)
- B2 ✅ 0 触碰 workspace.version
- A1 ✅ 0 触碰 R11 baseline 3 值
- B1 ✅ 0 触碰 24 LOCKED crate mtime
- B5 ✅ 0 改 6 哲学锚实质
- B3 ✅ 0 改 V0.5 公式
- B4 ✅ 0 改 5 重守门实质
- A3 ✅ 0 改 12 键原 12
- C1-C3 ✅ 0 commit, 0 装 PASS, 0 push

## 4. 0 装解除 verify
- 借鉴源码状态: (✅/⏳/❌)
- 0 假装"已借鉴": (true/false)
- 真实实施 vs 索引完成: (真实施/索引完成)

## 5. 整合 verify
- 20+ Skill struct impl 完整: (是/否 + 列表)
- SkillRegistry 中央注册: (是/否 + count)
- skills/ 20+ .md 文件: (是/否 + 列表)
- apeireth-central lib.rs 加 2 行 pub mod: (是/否 + diff)

## 6. 下一步 + 风险
- 1 个风险 / 1 个待 R125-N 续协调
```

---

## 你的工具 (你 sub-agent 必知)

你有: read, write, edit, grep, glob, bash. 你 0 commit, 0 push. 你 0 假装.

---

**派活完成 17:35. 截止 8/12 17:30 (跑过夜 8/11-8/12). 卡 30 min → 诊断 + kill + 派替代 (supervisor 监督).**
