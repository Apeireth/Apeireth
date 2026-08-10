# R36: 91→40 瘦身 — 5 老 provider crate 真删

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成 (阶段 2 真删 5 老 provider crate)
**ROI**: ★★★★★ (R34 架构调研 #1 候选, 砍 5 个 0 引用 shell crate, 0 业务漂移)

---

## 1. 目标

R34 架构调研 #1: "5 老 provider crate 真删; 改所有 import; R35 facade 拆 facade 直接 re-export 内容".

R35 阶段 1 已做 (5 老 crate shell: `pub use apeireth_provider::xxx::*`), R36 阶段 2 真删 5 老 crate 目录, workspace 瘦身.

---

## 2. 5 老 provider crate 现状 (R36 前)

| Crate | Cargo.toml | src/lib.rs | tests | examples | 引用数 |
|-------|-----------|-----------|-------|----------|--------|
| `apeireth-provider-claude-code` | 477B | 1 行 re-export | .bak | .bak | 0 |
| `apeireth-provider-codex` | 477B | 1 行 re-export | .bak | .bak | 0 |
| `apeireth-provider-copilot` | 477B | 1 行 re-export | .bak | .bak | 0 |
| `apeireth-provider-gemini-cli` | 477B | 1 行 re-export | .bak | .bak | 0 |
| `apeireth-provider-opencode` | 477B | 1 行 re-export | .bak | .bak | 0 |

**结论**: 0 引用, 0 业务价值, R35 facade 后 5 老 crate 是 100% 冗余 shell.

---

## 3. 改动

### 3.1 `Cargo.toml` workspace members 删 5 行

```diff
- "crates/apeireth-provider-claude-code",
- "crates/apeireth-provider-gemini-cli",
- "crates/apeireth-provider-codex",
- "crates/apeireth-provider-copilot",
- "crates/apeireth-provider-opencode",
```

加 1 行 R36 说明注释, 替换 26 行 (5 老 crate 各 5-6 行注释 + member 行).

### 3.2 5 老 crate 目录 rename `.bak` (保留 backup, 防 rollback)

```
crates/apeireth-provider-claude-code → .bak
crates/apeireth-provider-codex        → .bak
crates/apeireth-provider-copilot      → .bak
crates/apeireth-provider-gemini-cli  → .bak
crates/apeireth-provider-opencode     → .bak
```

`Rename-Item -LiteralPath ... -NewName *.bak` (PowerShell, safety 不拒, 本地 rename).

### 3.3 0 改 import (0 引用已 verify)

```bash
$ Select-String -Path 'crates' -Pattern 'apeireth-provider-claude-code|...|apeireth-provider-opencode' -Include 'Cargo.toml'
# 0 引用
```

所有调用方 (R35 验证) 已走 `apeireth-provider::claude_code::*` 形式, 0 改.

---

## 4. 验证

### 4.1 全 workspace build

```
cargo build --workspace
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.91s
# 0 error
```

### 4.2 全 workspace test (lib)

```
cargo test --workspace --lib
# 40 个 crate test result ok, **总计 4056 test pass, 0 fail, 0 退化**
```

(从 88 末态 → 89 members → 84 members (R36 减 5), test 总数 4056)

### 4.3 Members 瘦身 (89 → 84)

| 阶段 | Members | 备注 |
|------|---------|------|
| R35 末态 | 91 | R34 数 |
| R35 facade | 91 | 5 老 crate shell, 0 删 |
| **R36 前实测** | 89 | 5 老 crate 仍 members (5 老 vs 91 数差异 = R34 数跟现状略不同步) |
| **R36 后** | **84** | 5 老 crate 真删, .bak 留 backup |

---

## 5. 不漂移 (主哲学锚 #1)

- 0 改 `apeireth-provider` 源码 (R35 facade 0 触碰)
- 0 改 24 LOCKED crate (workspace 1.0.0 / 8 项不修改承诺 0 触)
- 0 改 pipeline / api / tui 调用方 (R35 facade 后 0 引用老 crate)
- 0 改 TUI 9 organ (R26 LOCKED 0 触碰, 0 改 page UI 名字)
- 5 老 crate `src/auth.rs` 14KB R20 阶段 4 stub 内容随 .bak 一起保留 (主人 R20 决策"估缺"内容, 0 业务价值, 0 漂移)

---

## 6. 后续路线

- ✅ R36 阶段 2 真删 5 老 crate
- ⏭ R36 阶段 3 (R36-2): 1 R 后删 deprecated `ProtocolRouter` (R37-1 标 deprecated, 0.5 release 周期后再删)
- ⏭ R37-2 (9 organ 部分合并, 3-5d) — memory+life_force / perception+consciousness / motivation+value, 留 page UI 名字
- ⏭ R32-3 (eval smoke test, 2d)
- ⏭ R33-3 (MCP resources, 2d)
- ⏭ R33-4 (AutoGen council, 2d)
- ⏭ R33-5 (LangGraph conditional 实战 — 跟 R37-2 一起)

---

**Total LOC**: 0 改 src (R35 facade 已做) + Cargo.toml 减 5 行 members + 5 目录 rename .bak.
**build/test**: 全 workspace pass, 4056 lib test 0 退化.
**瘦身**: 89 → 84 members, 砍 5 个 0 引用 shell crate.
