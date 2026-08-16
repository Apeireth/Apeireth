# apeireth-tool-approval

> Apeireth R17 鎴樺焦 2-3: 宸ュ叿瀹℃壒 (5 瑙勫垯 + 5 鍒嗛挓绐楀彛 + fuzzy matching 闆嗘垚, VCP 鍊熼壌 toolApprovalManager.js)

## Status

Part of the Apeireth workspace (74 active crate after R128 94鈫?5 merge).

**No-fake**: every public type or trait documented in this crate is real.
**Run-no-fear**: cargo check --workspace passes (0 errors).

## Where to start

- Cargo.toml: see [dependencies](Cargo.toml) for upstream crate.
- src/lib.rs: see top-level doc comment for module-level overview.

## See also

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth roadmap](../../docs/pages-source/roadmap.md)

---

_Auto-generated README per R128 batch (2026-08-12). Last-modified tracked in git log._
## R166 public API deep cleanup

`BORROWED_VCP_FIELDS` -> `BORROWED_LEGACY_FIELDS`. 62 tests pass.

## P1 增强: toolApprovalManager 新版吸收 (2026-08-17)

- **命令级粒度**: `ApprovalListRule` 支持 `Tool:command` 审批键 (specificity 2 > 1,
  同级静默优先); 命令从 args `command` / `command1..N` 提取 (`extract_commands`)
- **静默拒绝**: `::SilentReject` 后缀条目被拒 → `silent = true` (不打扰 AI),
  审计台账 `silent_rejection_audit()` 留痕可查
- **结构化拒绝**: `wait_for_approval_outcome()` → `ApprovalOutcome` /
  `Rejection { rejected_by_user, error_type, silent, reason, matched_rule, matched_command }`;
  错误码 `RejectErrorType`: rejected_by_user / approval_timeout / policy_deny / channel_unavailable
- **向后兼容**: `check` / `wait_for_approval` (bool) 签名不变, 内部委托结构化流程;
  `ApprovalDecision` 变体与字段零改动
- **洋葱安全红线**: 高危仍 `RequireApproval` → 主人批准通道; 无通道 fail-safe 拒绝,
  AI 不接触 master token
