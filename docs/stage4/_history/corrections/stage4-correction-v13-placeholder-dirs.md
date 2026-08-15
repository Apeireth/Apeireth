# 阶段 4 修正 v13 — 3 个占位目录创建（Fix-13）

```
[Document-Meta]
Document: docs/stage4/stage4-correction-v13-placeholder-dirs.md
Version: Fix-13 + Design-4.0
R-Cycle: R14
Commit: <latest-commit-hash>
Last-Modified: 2026-07-31
Status: 🟢 活跃
```

> **性质**: Fix-13 修正 v13 = 3 个占位目录创建（apeireth-legacy/deploy/tests）。
>
> **触发**: 主人 2026-07-31 让外部 AI 精确验证路径，发现 3 个目录不真存在。
>
> **产物**：
> - `apeireth-legacy/README.md`（1,215 bytes）
> - `deploy/README.md`（1,264 bytes）
> - `tests/README.md`（1,391 bytes）
>
> **本文件仅为修正链命名一致性索引**，**详细内容看顶层 3 个 README.md**。

---

## §1. 完整内容指向

| 路径 | README |
|---|---|
| `apeireth-legacy/README.md` | 阶段 7+ 真正施工时再归档 R11 1305 文件 |
| `deploy/README.md` | 阶段 7+ 真正施工时再创建 Dockerfile + compose + k8s |
| `tests/README.md` | workspace 顶层 tests/ 暂未创建，集成测试在 crates/<name>/tests/ |

## §2. v13 关键决策

| # | 决策 | 状态 |
|---|---|---|
| 1 | 创建 3 个空目录 | ✅ |
| 2 | 每个目录加占位 README | ✅ |
| 3 | README 明确"何时填内容" | ✅ |
| 4 | Cargo workspace 不受影响（9 member）| ✅ |
| 5 | 不破坏 LOCKED | ✅ |

## §3. v13 后续（Fix-14）

主人 2026-07-31 让"最后全量检查"——又发现 4 个小问题：

1. ✅ 顶层 `examples/hello_world.rs` 与 `crates/apeireth-core/examples/hello_world.rs` 重复 → 删除顶层
2. ✅ 顶层 `examples/` 加 README.md
3. ✅ `_STRUCTURE.md` 加过时标记（指向 APEIRETH-CONVENTIONS.md + FINAL-CHECK）
4. ✅ v12 + v13 加 stage4-correction 修正链索引

## §4. 主哲学 6 锚穿透

```
S-1 主 22:33 北极星导向
S-2 主 17:43 实事求是 — 创建占位而非撒谎
O-5 主 17:58 不假装 — 描述与现实一致
O-2 主 19:33 走在前人经验上
O-3 主 23:44 干到底
O-4 主 00:56 任何人都能接手 — 占位 README 明确未来填内容时机
```

---

_本修正由 leader 亲自产出（按主人 2026-07-31 让外部 AI 验证路径）._
_指向顶层 3 个 README.md._
_主哲学 6 锚穿透._