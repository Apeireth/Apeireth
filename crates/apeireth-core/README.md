# apeireth-core

> **Apeireth 主路径核心类型** — Episode / Note / Session / IdentityCard + bus + 13-key verdict cache.
> **当前状态**: R128 实际实施, 0 改 13 键 hardcode, PHL-07 V1.0 spec-only.
> **对应 crate**: `apeireth-sovereignty` (Self-Disable 4 项自动扫描), `apeireth-formal` (Kani proofs).

---

## 公共 API (R128 实际)

- `Episode` — 主路径 episode (id / timestamp / role / content / session_id)
- `Note` — session 内的笔记
- `Session` — 多 episode 容器
- `IdentityCard` — 主体身份卡 (per continuity_id)
- `bus` — 内部通信总线 trait
- `verdict` — 13 键 verdict cache 类型 (12 键 + PHL-07 = 13 键)
- `ApeirethError` — 顶层错误层级

## 13 键 verdict cache (编译期 hardcode)

| 键 | 语义 |
|---|---|
| V0 | 0 主动 commit |
| V1 | 0 主动 push |
| V2 | workspace.version 严守 |
| V3 | 24 LOCKED 入口签名冻结 (R128 降级, 历史保留) |
| V4 | R11 baseline 3 值严守 |
| V5 | V0.5 30 维守门 |
| V6 | 6 重守门 v7 |
| V7 | 8 哲学锚穿透 |
| V8 | 双洋葱架构 (PrincipleOnion + PermissionOnion) |
| V9 | 9 organ 内部 fn |
| V10 | 0 装 PASS 严守 |
| V11 | 0 重复造轮子严守 |
| V12 / PHL-07 | 借鉴标注开源 license 严守 |

## 依赖方向

```
所有 crate → apeireth-core (顶层依赖)
apeireth-core → std + serde + chrono + uuid + thiserror (5 核心库)
```

## 验证

- `cargo check -p apeireth-core` — 0 errors
- `cargo test -p apeireth-core` — 13 键 + Self-Disable 测试
- 12 + 1 = 13 键 编译期 hardcode 保证

## See also

- [Self-Disable](../apeireth-sovereignty/src/self_disable.rs)
- [13 键 verdict cache 规范](../../docs/conventions/10-locked.md)
- [13 键 + PHL-07 decision-130](../../reports/decision-130-12-15-tick-owner-3-q-a-6-b-phl-07-b-integrate-5-1-commit-execute-2026-08-11.md)