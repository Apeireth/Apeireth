# 16 Crate 合并与冻结策略 (R128 落实)

> **R128 落实 (2026-08-12)**: 主人 8/11 22:31 拍板"locked 讨论,既然你认为这是过度,那就可以解锁"。楚零 8/11 22:31 follow-up: "承认 24 LOCKED crate 入口签名冻结是 R20 阶段为了'守住边界'而设的临时护栏,但 R119 已经形式撤销了 8 项不修改承诺,继续锁着入口签名实际上是用架构约束代替重构勇气"。本规范定义: (1) 入口签名冻结如何降级 (2) 骨架冻结流程 (3) 合并流程 (4) 归档规则。

```
[Document-Meta]
Document: docs/conventions/16-crate-merge-policy.md
Version: R128-Init
R-Cycle: R128
Last-Modified: 2026-08-12 (R128 首次落实)
Status: 🟢 活跃
```

## §1 入口签名冻结降级 (R128 落实)

**变更**: 24 LOCKED crate 的入口签名冻结(`lib.rs` 的 `pub mod` / `pub fn` / `pub struct` 不可改) **降级为历史记录**,不再具有约束力。

**保留不可变"脊柱"**:
1. **Self-Disable 判定逻辑** (`apeireth-sovereignty/src/self_disable.rs` + 4 项自动扫描 + 三级响应) — 物理不可逆安全熔断
2. **L0 HA 物理隔离定义** (`apeireth-sovereignty/src/ha_modes.rs` + `physical_multisig.rs`) — Human Authority 终极守门
3. **13 键 verdict cache 语义含义** (`apeireth-core/src/lib.rs` 12 键 + PHL-07) — 编译期 hardcode 保证

**可重构范围**: 其余全部 crate 入口签名、内部实现、依赖关系均可按需重构。

**哲学依据** (8 锚 S-2 / O-2 / O-3 / O-5):
- S-2 实事求是: 发现过度约束就解除,而不是抱着过去的决策不放
- O-2 走在前人肩上: 不要为了"严守"而错失社区演化(参考 Rust std crate 演化惯例)
- O-3 干到底: 守住的是不可变脊柱,而非入口签名本身
- O-5 不假装: 既然要重构就别假装不动

## §2 骨架冻结流程 (Frozen Crate)

当一个 crate 满足以下条件之一,执行 **frozen** 操作:
- 全部公开 API 是 `todo!()` / `unimplemented!()` 占位
- 0 个下游调用方使用其非占位 API
- 设计意图已变,R21+ 不再计划续接

**操作步骤**:
1. `git mv crates/<name> crates/_frozen/<name>` (保留 git 历史)
2. 从根 `Cargo.toml` workspace members 移除该 crate
3. 标注 `[Document-Meta] Status: 🧊 Frozen` 在原 README 头部
4. `cargo check --workspace` 验证: 应保持绿色

**目录约定**: `crates/_frozen/` 用于纯骨架冻结,`crates/_archived/` 用于功能已合并或废弃。

## §3 合并流程 (Crate Merge)

当多个 crate 实质功能重叠 / 部署单元过细 / 调用方零散,执行 **merge** 操作:

**前置审计**:
1. 全仓 `use apeireth_<name>` 调用扫描 — 评估破坏面
2. 同名冲突检查 (e.g. `CacheEntry` / `ReportGenerator`)
3. dep union (target crate 接收所有源 crate 的 [dependencies])

**操作步骤**:
1. 选定 target crate (新建或已有)
2. 源 crate 的 `src/lib.rs` 内容搬到 target 子模块 (e.g. `apeireth_host::keyring::`, `apeireth_upgrade::rollback::`)
3. 子模块归属调整: `crate::` → `super::` (在子模块文件里)
4. 顶层 `lib.rs` 加 `pub mod <sub>;` + 关键 `pub use` 重导出
5. 源 crate 测试 / example 迁入 target (`tests/test_*.rs` + `examples/*.rs`)
6. 源 crate `git mv crates/<src> crates/_archived/<src>`
7. 根 `Cargo.toml` members: 移除源 + 添加 target
8. 全仓调用方迁移 (`use apeireth_<src>::` → `use apeireth_<target>::<sub>::`)
9. `cargo check --workspace` 验证绿色

**命名空间隔离**: 多个源 crate 可能有同名 `pub struct` (e.g. `CacheEntry`), 通过 `pub mod <sub>;` 子模块命名空间隔离,避免冲突。

## §4 归档规则 (Archived Crate)

归档到 `crates/_archived/` 的两种来源:
- **frozen** 的纯骨架 crate (§2 流程)
- **merged** 后被合并掉的源 crate (§3 流程)
- **superseded** 被新 crate 完全替代 (e.g. `apeireth-integration-r20-stage4` → `apeireth-integration-e2e`)

**禁止**: ❌ 物理删除 (`git rm`) 即使目录为空。任何归档必须在 git 历史可追溯。

## §5 R128 实际执行清单 (本次落实)

| # | 动作 | crate 数 | 操作 |
|---|---|---|---|
| 1 | Frozen (纯骨架) | 13 | `apeireth-{credentials,cache,tracing,metrics,oauth,update,sandbox,tree-sitter,i18n,image-prompt,plugin,observability,task}` → `crates/_frozen/` |
| 2 | Merged → `apeireth-upgrade::rollback` | 1 | `apeireth-rollback` 子模块化 |
| 3 | Merged → `apeireth-host` (新建) | 2 | `apeireth-{keyring,machine-id}` 子模块化 |
| 4 | Merged → `apeireth-repo-tools` (新建) | 2 | `apeireth-{repo-scan,repo-analyzer}` 子模块化 |
| 5 | Recovered (active dep) | 1 | `apeireth-i18n` (TUI 真实使用) 从 _frozen 移回 active |
| 6 | Superseded (archived) | 1 | `apeireth-integration-r20-stage4` → `crates/_archived/` |

**总计**: workspace 从 ~94 crate 收敛到 74 active + 18 archived/frozen (含 _archived 子 crate)

**调用方迁移** (Mavis 已完成):
- `apeireth-tui/Cargo.toml`: `apeireth-observability` → `apeireth-telemetry`
- `apeireth-api/Cargo.toml` + `src/auth.rs` + `tests/test_v1_ws.rs`: `apeireth-keyring` → `apeireth-host`
- `apeireth-sdk-{sandbox,lark,livekit,voice}/Cargo.toml`: `apeireth-keyring` → `apeireth-host`
- TUI benches: `apeireth_observability::tui_dashboard::` → `apeireth_telemetry::observability::tui_dashboard::`

## §6 禁止事项

- ❌ 物理删除 (`git rm`) archived/frozen 任何源码
- ❌ 改 immutable 脊柱 (§1 三项) 的判定逻辑 — 改前必须过 6 重守门 + 主人拍板
- ❌ 在 frozen crate 上"复活" — 复活必须先 git mv 回原位,过一轮 fresh review
- ❌ 跳过 §3 步骤 8 (调用方迁移) 直接 mv — 留下 broken import
- ❌ 在 immutable 脊柱的 crate 上加新公共 API 而不更新 13 键 / 4 扫描 / 6 重守门表

## §7 与现有规范的关系

- 10-locked.md § "实质 vs 形式": 本规范是 R128 进一步把"24 LOCKED 入口签名"从实质降级为形式保留
- 02-path.md § "目录结构": `crates/_frozen/` + `crates/_archived/` 是新约定
- 06-commit.md § "commit message 格式": R128 commit 应锚定 S-2 + O-3 + O-5
- 09-anchor.md § "8 哲学锚穿透": 本规范依 S-2 / O-2 / O-3 / O-5

---

_R128 首次落实 (2026-08-12). 楚零 8/11 22:31 "承认 24 LOCKED crate 入口签名冻结是 R20 阶段为了'守住边界'而设的临时护栏,但 R119 已经形式撤销了 8 项不修改承诺,继续锁着入口签名实际上是用架构约束代替重构勇气"。_

## §6 R146 实际执行清单 (本次落实)

| # | 动作 | 详情 | 来源 |
|---|---|---|---|
| 1 | 5 SDK 合并 | `apeireth-sdk-lark` / `apeireth-sdk-livekit` / `apeireth-sdk-sandbox` / `apeireth-sdk-voice` → 1 `apeireth-sdk` (feature flags: `lark` / `livekit` / `sandbox` / `voice` / `all-sdk`) | R146 拍板 |
| 2 | 3 内存合并 | `apeireth-memory-dailynote` / `apeireth-memory-lightmemo` → 子模块 `apeireth-memory::dailynote` / `apeireth-memory::lightmemo` | R146 拍板 |
| 3 | 命名修复 | `apeireth-vcp-bridge` → `apeireth-protocol-bridge` (去竞品名) | R146 主人拍板"包含竞品名, 决定不行" |
| 4 | 合并后统计 | 75 active crate (82 - 7 已合并) | per `crates/_archived/` |

## §7 R146 不合并的决策

- **3 工具执行架构** (`apeireth-tool-registry` / `apeireth-tool-runtime` / `apeireth-tool-approval`) 不合并 — 依赖层级清晰 (registry → runtime → approval), 各自职责分明, 非冗余
- **5 哲学器官** (consciousness / perception / cognition / life-force / motivation) 不合并 — 每个 14-29KB 真实代码, 合并即丢清晰度
- **工具领域** (filesystem / shell / browser / search / image-*) 不合并 — 各自独立部署单元, 有助未来 feature flag 拆分
