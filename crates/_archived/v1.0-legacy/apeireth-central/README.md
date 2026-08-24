# apeireth-central

> Apeireth CentralAI aggregate root, lifecycle coordinator, and PID 1 supervisor entry

apeireth-central 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (12 src 文件 / 114 测试 + 2 Kani proof)

- `src/lib.rs` — 17 架构组件 aggregate root + 9 阶段生命周期 (LEGAL_TRANSITIONS 编译期 hardcode) + IdentityCard 跨载体迁移 + Maturity 17 链接闸门 + 5 子树调度 + PidOneSupervisor + ApeirethCentral + 33 测试
- `src/skill_trait.rs` — R125-15e Skill trait (14 Skill 1:1 映射 superpowers) + 7 测试
- `src/skill_registry.rs` — R125-15e SkillRegistry 中央注册 + 12 测试
- `src/skill_execution.rs` — R125-18 SkillExecutor + StepExecution + 5 测试
- `src/skill_prompt.rs` — R125-18 SkillPrompt + SkillPromptCache + 13 测试
- `src/skill_validation.rs` — R125-18 validate_skill + 5 项质量门 + 9 测试
- `src/skill_companion.rs` — R125-18 7 variant 协作资源 + 8 测试
- `src/skill_frontmatter.rs` — R125-18 parse_frontmatter + 12 测试
- `src/skill_recommender.rs` — R125-16 14 Skill 关键词自动推荐 + 8 测试
- `src/skill_outcome.rs` — SkillOutcome 结构 (无单测, 共享类型)
- `src/skill_runner.rs` — SkillRunner 执行入口 (无单测, 共享类型)
- `src/organ_kani_proofs.rs` — R177 central organ Kani proofs (5 测试 + 2 `#[kani::proof]`)
