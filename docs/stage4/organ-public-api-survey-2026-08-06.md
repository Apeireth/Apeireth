# 9 器官 4 crate 公开 API 表面调查（2026-08-06）

> 本文档登记 Hermes 审计中识别的“4 个器官 crate 没有 pub use 顶层导出”问题，给出当前 pub 表面清单与改进建议。
> **不动 src**：本文档只描述现状，不修改任何 crate 的 lib.rs，避免隐形升级 8 项不修改承诺。

## 1. 现状统计（per 当前 HEAD）

| crate | lib.rs 行数 | `pub use` 顶层 | 实际 pub fn/struct/enum/trait | 文件数 |
|---|---:|---:|---:|---:|
| `apeireth-motivation` | 705 | 0 | 23 | 1 |
| `apeireth-consciousness` | 173+ | 0 | 9（+ `pub mod transfer_monitor`） | 2 |
| `apeireth-relation` | 280+ | 0 | 9 | 1 |
| `apeireth-life-force` | 327+ | 0 | 14（+ `pub mod reflection_cycle` / `pub mod emergence`） | 3 |

注意：`apeireth-relations` 不存在，正确名字是 `apeireth-relation`。

## 2. 公开表面节选（per lib.rs 顶部 `pub` 项）

### apeireth-motivation
- 类型：`MotivationError`、`DriveKind`、`MotivationDrive`、`InternalDrive`、`ExternalDrive`、`Modality`、`MultimodalIntent`、`SGIStructured`、`SGIContent`、`EvidenceKind`、`Evidence`、`SGIEntry`、`SGI`、`WriteResult`、`AuditEvent`、`ReflectionAuditor`、`AutonomyConsistency`、`ValueStability`、`IntrinsicIntensity`、`MotivationScore`
- 函数：`check_csgi1_uniqueness`、`check_csgi5_content_kind`、`check_csgi6_max_chars`、`check_csgi7_three_required`、`evidence_check`、`write_flow`、`motivation_score`

### apeireth-consciousness
- 类型：`CognitiveDreamState`、`ConsciousnessError`、`TransitionRecord`、`TransitionReason`、`CognitiveDreamStateMachine`
- 函数：`legal_targets`、`can_transition`
- 新增 mod（路线 A2.2）：`transfer_monitor`

### apeireth-relation
- 类型：`RelationKind`、`Relation`、`RelationError`、`RelationDecision`、`RelationRegistry`
- 函数：`classify`、`classify_pair`

### apeireth-life-force
- 类型：`SelfGrowthIndicator`、`ReflectionPeriod`、`ReflectionPeriodState`、`StandardReflectionPeriod`、`ReflectionTrigger`、`LifeForce`、`LifeForceError`
- 函数：`reflection_trigger`、`exhaustion_check`、`recovery_start`、`validate_endurance`、`reflection_progress`
- 新增 mod（路线 A2.1/A2.3）：`reflection_cycle`、`emergence`

## 3. 改进建议（不动 src）

按 Hermes 提议“在 R23 加 4 crate lib.rs pub use 顶层导出 5+ 实体 each”，我建议如下（**待主人拍板，本会话不动 src**）：

### apeireth-motivation
```rust
pub use crate::{
    DriveKind, Evidence, EvidenceKind, ExternalDrive, InternalDrive, Modality, MultimodalIntent,
    MotivationDrive, MotivationError, MotivationScore, SGI, SGIContent, SGIStructured,
    WriteResult,
};
```

### apeireth-consciousness
```rust
pub use crate::{
    CognitiveDreamState, CognitiveDreamStateMachine, ConsciousnessError, TransitionReason,
    TransitionRecord, transfer_monitor::{CycleDetector, TransferRateLimiter},
};
```

### apeireth-relation
```rust
pub use crate::{
    Relation, RelationDecision, RelationError, RelationKind, RelationRegistry,
};
```

### apeireth-life-force
```rust
pub use crate::{
    LifeForce, LifeForceError, ReflectionPeriod, ReflectionPeriodState, ReflectionTrigger,
    SelfGrowthIndicator, emergence::EmergenceDetector, reflection_cycle::{ReflectionCycleScheduler, ReflectionCycleState},
};
```

## 4. 边界

- 4 crate 当前都没被 8 项 LOCKED 集合直接保护；本改进属“非 LOCKED 改进”，但与 LOCKED 工程层 24 名单里 `relation` 和 `life-force` 的 Mavis 范畴有关。
- 本会话不擅自动手；下一步等主人拍板后由 Mavis 在 R23 一并处理，估 0.5 天（含 build + test 回归）。
