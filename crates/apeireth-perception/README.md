# apeireth-perception — 感知器官（A9）

```
[Document-Meta]
Document: crates/apeireth-perception/README.md
Crate: apeireth-perception
Version: 0.14.0 (workspace 继承)
R-Cycle: R14
Phase: 4 (A9 实质落地完成)
Status: 🟢 完整器官 (A9 落地, 2026-08-01)
```

## 🎯 一句话

**Apeireth 感知器官** = 外部输入接入层（CLI / TTY / HTTP / Python 桥 → 5 种 `PerceptionInput` → 5 种 `PerceptionChannel` → 统一 `PerceptionEvent` → 注意力过滤 → 交给 cognition）。

## 📦 当前状态

A9 完整落地（code_reviewer, 2026-08-01）：

| 组件 | 状态 |
|---|---|
| `Cargo.toml` | ✅ 完整（含 `[[example]]` 声明）|
| `src/lib.rs` | ✅ 模块聚合 + 6 pub fn + 错误类型 + 8 测试 |
| `src/input.rs` | ✅ `PerceptionInput` trait + 5 种实现 (Text/Voice/Vision/Tactile/Command) |
| `src/attention.rs` | ✅ `Attention` trait + 2 种策略 (TopK/Threshold) + 7 测试 |
| `src/channel.rs` | ✅ `PerceptionChannel` trait + 5 种通道 + 8 测试 |
| `examples/perception_demo.rs` | ✅ 4 场景演示 (单模态/TopK/多模态/批量) |
| `tests/pipeline_e2e.rs` | ✅ 2 集成测试 (端到端 + Command 链路) |
| workspace members | ✅ 已加入 |
| 实质代码 | ✅ A9 落地完成 |
| **总计** | **31 tests passing (29 unit + 2 integration)** |

## 🔌 依赖关系

```
apeireth-perception
    └── apeireth-core (Episode/Note/Session/IdentityCard)
```

## 📐 类型一览

| Trait / Type | 职责 |
|---|---|
| `PerceptionInput` | 一条外部信号 (timestamp/source/priority/id) |
| `TextInput` / `VoiceInput` / `VisionInput` / `TactileInput` / `CommandInput` | 5 种模态实现 |
| `Attention` | 注意力 trait (`score` + `filter`) |
| `TopKAttention` / `ThresholdAttention` | 2 种内置策略 |
| `PerceptionChannel` | 通道 trait (`kind` + `process` + `process_batch`) |
| `TextChannel` / `VoiceChannel` / `VisionChannel` / `TactileChannel` / `CommandChannel` | 5 种通道 |
| `PerceptionEvent` | cognition 的统一输入 |
| `ChannelKind` / `SignalSource` | 枚举标签 |

## 🚀 验证命令

```bash
cargo check  -p apeireth-perception    # 0 errors (18 doc warnings)
cargo test   -p apeireth-perception    # 31 passed
cargo run    -p apeireth-perception --example perception_demo
```

## 📚 必读

- `START-CONSTRUCTION.md` §A9
- `docs/stage4/architecture-stage4-engineering-landing.md` §2.2 (9 器官) + §3.1 (感知层 trait)
- `docs/stage3-blueprints/01-overall-architecture.md` (整体架构)

## 🚧 升级路径 (ponytail 标记)

阶段 5 可深化（不影响 A9 当前 DoD）:
- 显著性模型替换启发式 priority (像素/loudness)
- 多模态融合 (cross-modal event aggregation)
- 与 apeireth-cognition 的 `run_cycle` 自动对接

— code_reviewer（A9 落地，2026-08-01）