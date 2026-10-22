# PromptCacheStabilizer 孤岛归档（R7-4a，ADR-0007 §2.3）

- 原路径: `crates/foundation/orchestration/src/prompt_stabilizer.rs`
- 移入: `legacy/archived/prompt-stabilizer/prompt_stabilizer.rs`（legacy 不参与构建；`exclude` 已含 legacy）
- 删除原因（owner 2026-10-20「按建议来」）: 无生产消费点（外部引用仅 lib.rs re-export + organ 注释），静态度量 5 个公开项（StabilizedRole/StabilizedMessage/EphemeralContextSnapshot/PromptCacheStabilizer/assemble_tiered）全为 export-only。
- 恢复步骤（如需重新接线）:
  1. `git mv legacy/archived/prompt-stabilizer/prompt_stabilizer.rs crates/foundation/orchestration/src/`
  2. 在 `crates/foundation/orchestration/src/lib.rs` 恢复 `pub mod prompt_stabilizer;` 与对应 `pub use` 块
  3. 开 ADR（R7 范围外的新接线须走 ADR 流程），并补 cache-prefix 稳定性测试 + 生产消费点
- 关联: ADR-0007（Accepted）；R7-2 上下文单一化（organ 的 assemble_tiered 为保留面）。
