# ADR 0012: SpectrAI 0.9.21 1:1 翻译 — Apeireth Rust 重写的"对标基础"

> **状态**: 🟢 Accepted (R14 阶段主人 2026-07-30 拍板, 1.0 release 续补)
> **commit 锚**: `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行) + 17 核心 + 8 增量 + 9 估缺 = 34 crate
> **最后更新**: 2026-08-05 22:13
> **原版 ADR**: [`archive/r20-pre-renumber/0023-spectrAI-reverse-engineering.md`](archive/r20-pre-renumber/0023-spectrAI-reverse-engineering.md) (v0; v1 本 ADR 引用新编号 0001/0002/0003/0006/0009)

---

## 1. 背景 (Context)

Apeireth Rust 重写 (本仓库) 是 **SpectrAI 0.9.21** (2024-2025, Apache-2.0) 的 1:1 翻译 + 增量扩展。

**问题**:
- SpectrAI 0.9.21 是 TypeScript 实现, 17 个核心模块 + 若干扩展
- 本仓库是 Rust 重写, 1.0 release (v1.0.0) 目标: "行为 / API 跟 SpectrAI 0.9.21 完全一致, 性能更好, 部署更简"
- 哪些模块 1:1 翻译? 哪些是 R14+ 增量? 哪些是 R20 估缺?

**约束**:
- 1:1 翻译 = 行为一致, 但不抄 TS 代码 (翻译 ≠ 移植)
- Rust idiom 优先 (e.g. 不用 `Rc<RefCell>` 模拟 TS 的可变引用)
- 增量扩展 = 立体架构 v2 + 生命架构 v4/v4.1 + 17 crate 本源推导 (per `docs/architecture-v4-1-living-intelligence-update.md`)

---

## 2. 决策 (Decision)

**17 核心模块 1:1 翻译 + R14+ 立体架构 v2 增量 + R20 估缺 9 skeleton 续补**

### 2.1 17 核心模块 1:1 翻译

| # | SpectrAI 0.9.21 模块 | 本仓库 crate | 状态 | 备注 |
|---|---|---|---|---|
| 1 | `core` | `apeireth-core` | ✅ 1:1 | 17 trait 互锁 + 22 枚举 (per `archive/r14/0003-trait-interlock-22-enum.md`) |
| 2 | `cli` | `apeireth-cli` | ✅ 1:1 | CLI 入口 (per `archive/r14/0002-cli-session-api-binding.md`) |
| 3 | `tools` | `apeireth-tools` | ✅ 1:1 | 6 工具基类 (calendar/message/contact/task/search/drive, per [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md)) |
| 4 | `memory` | `apeireth-memory` | ✅ 1:1 | SQLite 持久化 + 5 表 (per [0009-d-07-sqlite-to-postgres.md](0009-d-07-sqlite-to-postgres.md)) |
| 5 | `asi` | `apeireth-asi` | ✅ 1:1 | ASI engine + V0.5 24 维 |
| 6 | `cognition` | `apeireth-cognition` | ✅ 1:1 | 认知层 |
| 7 | `action` | `apeireth-action` | ✅ 1:1 | 动作层 |
| 8 | `life-force` | `apeireth-life-force` | ✅ 1:1 | 生命力 (生长 / 衰老 / 死亡, per 主人 "9 阶段我们不需要衰老病死" 拍板) |
| 9 | `constraint` | `apeireth-constraint` | ✅ 1:1 | 约束层 (含 token bucket, per D-04) |
| 10 | `central` | `apeireth-central` | ✅ 1:1 | 中心调度 |
| 11 | `value` | `apeireth-value` | ✅ 1:1 | 价值观 (4 维 V0.5) |
| 12 | `consciousness` | `apeireth-consciousness` | ✅ 1:1 | 意识层 (4 类: PC/RC/HG/GP) |
| 13 | `relation` | `apeireth-relation` | ✅ 1:1 | 关系层 |
| 14 | `motivation` | `apeireth-motivation` | ✅ 1:1 | 动机层 |
| 15 | `perception` | `apeireth-perception` | ✅ 1:1 | 感知层 |
| 16 | `upgrade` | `apeireth-upgrade` | ✅ 1:1 | 自升级 (R-Measure verify) |
| 17 | `onion` | `apeireth-onion` | ✅ 1:1 | 双洋葱 (Principle + Permission, per `archive/r14/0001-double-onion-unity.md`) |

> 17 模块 100% 1:1 翻译, 0 行为偏差 (per `docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md`)。

### 2.2 R14+ 立体架构 v2 增量 (本仓库独有, 不在 SpectrAI 0.9.21)

| 增量 | 模块 | 状态 | 备注 |
|---|---|---|---|
| **Council** | `apeireth-council` | ✅ 24 trait 互锁 | 7 advisor 协同 (per R10 round10-07) |
| **Sovereignty** | `apeireth-sovereignty` | ✅ | 4 重守门 (per APEIRETH-CONVENTIONS §10) |
| **Supervisor** | `apeireth-supervisor` | ✅ | team-lead supervisor prompt 翻译 (per `archive/r14/0011-apeireth-team-lead-supervisor-prompt-translation.md`) |
| **PyBridge** | `apeireth-pybridge` | ✅ (feature flag) | Python 互操作 (PyO3) |
| **Verify** | `apeireth-verify` | ✅ | 形式化验证 (per [0010-6-philosophy-anchors.md](0010-6-philosophy-anchors.md)) |
| **Extension** | `apeireth-extension` | ✅ | 扩展点 (3 个: Tool / Provider / Council) |
| **Evolution** | `apeireth-evolution` | ✅ | 自演化 (R-Measure) |
| **Bus** | `apeireth-bus` | ✅ | 进程内消息总线 |

### 2.3 R20 估缺 9 skeleton 续补 (R20 阶段 1 续, per [0003-integrate-3-strategy.md](0003-integrate-3-strategy.md))

| 估缺 | crate | 来源 (1:1 翻译) | 估时 | 状态 |
|---|---|---|---|---|
| **image-prompt** | `apeireth-image-prompt` | v0.9.21 imageTools.ts (1:1) | 1 owner × 3 天 | ✅ 落地 (817 行) |
| **rollback** | `apeireth-rollback` | v0.9.21 rollback.js | 1 owner × 2 天 | ✅ 落地 (1040 行, 71GB 4 重防御) |
| **plugin** | `apeireth-plugin` | v0.9.21 plugin 体系 | 1 owner × 1 周 | ✅ 落地 (816 行) |
| **repo-scan** | `apeireth-repo-scan` | v0.9.21 repoScan.ts | 1 owner × 3 天 | ✅ 落地 (617 行) |
| **repo-analyzer** | `apeireth-repo-analyzer` | v0.9.21 repoAnalyzer.ts | 1 owner × 3 天 | ✅ 落地 (867 行) |
| **keyring** | `apeireth-keyring` | v0.9.21 keyring (OS keychain) | 1 owner × 2 天 | ✅ 落地 (972 行, PBKDF2 600_000) |
| **machine-id** | `apeireth-machine-id` | v0.9.21 machineId (hardware ID) | 1 owner × 1 天 | ✅ 落地 (359 行 + 4 平台 424 行) |
| **lark** | `apeireth-lark` | v0.9.21 lark SDK stub | 1 owner × 1 周 | ✅ 落地 (577 行, STUB_MODE 守门) |
| **voice** | `apeireth-voice` | v0.9.21 voice (TTS/STT) | 1 owner × 1 周 | ✅ 落地 (740 行, STUB_MODE 守门) |

### 2.4 1:1 翻译的"翻译 ≠ 移植"原则

**不抄 TS 代码**, 而是按 Rust idiom 重写:

| SpectrAI 0.9.21 (TS) | 本仓库 (Rust) | 翻译原则 |
|---|---|---|
| `interface` | `trait` | 同语义, 不同语法 |
| `type X = A \| B` | `enum X { A, B }` | sum type 统一 enum |
| `async/await` | `async/await` (tokio) | 同语义, runtime 换 tokio |
| `Promise<T>` | `Future<Output=T>` | 编译期检查 |
| `try/catch` | `Result<T, E>` | 错误显式, 编译期强制 |
| `Rc<RefCell<T>>` | `Arc<Mutex<T>>` | 跨线程 + 内部可变 |
| `JSON.parse` | `serde_json::from_str` | 编译期 schema |
| `fs.readFile` | `tokio::fs::read` (P0-1 改 `fs_err`) | async + 错误增强 |
| `child_process.spawn` | `tokio::process::Command` | async 化 |

### 2.5 翻译验证

- **行为一致**: 17 模块 1:1 测试 (per `docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md` §3)
- **API 一致**: HTTP API schema 跟 SpectrAI 0.9.21 100% 兼容 (per [0006-d-01-tool-endpoint-real.md](0006-d-01-tool-endpoint-real.md) D-01)
- **数据兼容**: SQLite 5 表 schema 跟 SpectrAI 0.9.21 100% 兼容 (per [0009-d-07-sqlite-to-postgres.md](0009-d-07-sqlite-to-postgres.md) D-07)

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **行为一致**: 用户从 SpectrAI 0.9.21 切 Apeireth-rust 0 学习成本
- ✅ **API 一致**: HTTP client 复用, TUI / Tauri / 第三方客户端 0 改
- ✅ **数据兼容**: SQLite 5 表 100% 兼容, 升级 0 丢失 (per [0009](0009-d-07-sqlite-to-postgres.md))
- ✅ **17 + 8 + 9 = 34 crate 全 1:1**: 行业最大 TS → Rust 翻译项目之一
- ✅ **R14+ 增量不破坏 1:1**: Council / Sovereignty / Supervisor 是新增, 不动 17 核心

### 3.2 负面

- ⚠️ **翻译 ≠ 移植**: 1:1 行为, 不 1:1 代码; 团队需 1 周消化 Rust idiom
- ⚠️ **9 估缺续补估时紧**: 1 owner × 6 周, R20 阶段 1 续 6 commits 估补
- ⚠️ **17 核心 + 8 增量 + 9 估缺 = 34 crate**: workspace 复杂度高, 文档压力大
- ⚠️ **生命架构 v4/v4.1** 是 R14+ 增量, 不在 SpectrAI 0.9.21; 1.0 release 必须重新设计验证

### 3.3 风险

- SpectrAI 0.9.21 行为变更 (上游) — 本仓库独立分支, 不跟随, 1:1 锁 0.9.21
- 9 估缺续补时间紧张 (R20 阶段 1 续 2026-08-05 拍板 1 owner × 6 周); 延期 R21 估补
- 71GB 事故 (R20 阶段 1 真发生) 根因 = `apeireth-rollback` 旧版无 TTL 兜底, 估补后 1.0 release 阶段 1 闭环

---

## 4. 备选 (Alternatives Considered)

### A. 推倒重写, 不 1:1
- 优点: 自由设计
- 否决: 失去 SpectrAI 0.9.21 行为一致优势; 用户升级成本高; 主人 2026-07-30 拍板 "1:1 翻译 + 增量"

### B. 1:1 移植 (TS 代码逐行 Rust 化)
- 优点: 快
- 否决: 翻译 ≠ 移植; 强 TS 习惯 (Rc<RefCell>) 会污染 Rust idiom

### C. 1:1 翻译 (行为一致, 代码 Rust 化) (本决策)
- 优点: 行为一致 + Rust idiom
- 拍板: R14 阶段主人 2026-07-30 拍

### D. 仅翻译, 不增量 (Council / Sovereignty 砍)
- 优点: 快 + 简
- 否决: R14+ 立体架构 v2 是 Apeireth 跟 SpectrAI 0.9.21 最大区别, 砍了等于 "换皮 SpectrAI"

### E. 翻译 + 增量 + 大幅重写 (9 估缺超过 1:1 范围)
- 优点: 1 步到位
- 否决: 重写范围不可控, 1.0 release 风险高; 1:1 范围是已验证基础

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上 (北极星)**: TS → Rust 1:1 翻译业界常见 (e.g. Deno 抄 Node, Rome 抄 TS)
- ✅ **S-2 实事求是**: 17 核心 + 8 增量 + 9 估缺 = 34 crate 是已 commit 状态, 不凭想象
- ✅ **O-2 走在前人肩上 (用户看结果不看哲学)**: 用户只看"行为 / API / 数据一致", 不看翻译策略
- ✅ **O-3 干到底 (信息密度"高")**: §2.1 + §2.2 + §2.3 3 表说清 17 + 8 + 9; §2.4 9 行表说清 翻译 ≠ 移植
- ✅ **O-4 任何人都能接手 (干净状态)**: 拒绝"先做后改" / 拒绝"翻译 ≠ 移植"含糊
- ✅ **O-5 不假装 (6 哲学锚穿透)**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 17 核心 ✅ + 8 增量 ✅ + 9 估缺 (R20 阶段 1 续 落地 6 commits) 状态诚实标
- ✅ **编译期 hardcode**: 17 trait 互锁 + 22 枚举 编译期固定 (per `archive/r14/0003-trait-interlock-22-enum.md`)
- ✅ **不改 LOCKED**: 7 LOCKED 文档 + 24 LOCKED crate 0 触碰 (24 LOCKED = 17 核心 + 7 增量核心)
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 Rust 重写, 0 依赖 SpectrAI 0.9.21 上游
- ✅ **不重复造轮子**: 沿用 Rust 生态 (tokio / serde / sqlx / axum), 不自造 runtime
- ✅ **诚实标缺**: 9 估缺续补时间紧, 延期 R21 估补; 生命架构 v4/v4.1 1.0 release 重验证

---

## 7. 引用

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) (604 行, per [0002-rival-blueprint.md](0002-rival-blueprint.md))
- 翻译 1:1 验证: [`docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md`](../../docs/stage4/apeireth-architecture-readonly-review-2026-08-05.md) §3
- 17 核心 + 8 增量 + 9 估缺: `Cargo.toml` [workspace] members lines 3-148
- 立体架构 v2 增量: [`docs/architecture-v4-1-living-intelligence-update.md`](../../docs/architecture-v4-1-living-intelligence-update.md)
- 整合 #3 策略: [`0003-integrate-3-strategy.md`](0003-integrate-3-strategy.md)
- D-01 API 一致: [`0006-d-01-tool-endpoint-real.md`](0006-d-01-tool-endpoint-real.md)
- D-07 数据兼容: [`0009-d-07-sqlite-to-postgres.md`](0009-d-07-sqlite-to-postgres.md)
- 1.0 release 总览: [`0001-apeireth-rust-1.0.md`](0001-apeireth-rust-1.0.md)
- 6 哲学锚: [`0010-6-philosophy-anchors.md`](0010-6-philosophy-anchors.md)
- TUI 瘦客户端: [`0011-tui-as-thin-client.md`](0011-tui-as-thin-client.md)
- 原版 ADR v0: [`archive/r20-pre-renumber/0023-spectrAI-reverse-engineering.md`](archive/r20-pre-renumber/0023-spectrAI-reverse-engineering.md)
- 原 R14 历史 ADR: [`archive/r14/`](archive/r14/) (12 R14 历史 ADR 已归档)

---

## 8. 附录

### 8.1 17 核心 + 8 增量 + 9 估缺 = 34 crate 速查 (per §2.1-2.3)

**17 核心 (1:1 翻译, per `apeireth-core` 17 trait 互锁)**:
```
apeireth-core             apeireth-cli              apeireth-tools
apeireth-memory           apeireth-asi              apeireth-cognition
apeireth-action           apeireth-life-force       apeireth-constraint
apeireth-central          apeireth-value            apeireth-consciousness
apeireth-relation         apeireth-motivation       apeireth-perception
apeireth-upgrade          apeireth-onion
```

**8 增量 (R14+ 立体架构 v2, 本仓库独有)**:
```
apeireth-council          apeireth-sovereignty      apeireth-supervisor
apeireth-pybridge         apeireth-verify           apeireth-extension
apeireth-evolution        apeireth-bus
```

**9 估缺 (R20 阶段 1 续, 1:1 翻译 v0.9.21 商业版估缺)**:
```
apeireth-image-prompt     apeireth-rollback         apeireth-plugin
apeireth-repo-scan        apeireth-repo-analyzer    apeireth-keyring
apeireth-machine-id       apeireth-lark             apeireth-voice
```

### 8.2 翻译 ≠ 移植 9 行映射表 (per §2.4)

| SpectrAI 0.9.21 (TS) | 本仓库 (Rust) | 翻译原则 |
|---|---|---|
| `interface` | `trait` | 同语义, 不同语法 |
| `type X = A \| B` | `enum X { A, B }` | sum type 统一 enum |
| `async/await` | `async/await` (tokio) | 同语义, runtime 换 tokio |
| `Promise<T>` | `Future<Output=T>` | 编译期检查 |
| `try/catch` | `Result<T, E>` | 错误显式, 编译期强制 |
| `Rc<RefCell<T>>` | `Arc<Mutex<T>>` | 跨线程 + 内部可变 |
| `JSON.parse` | `serde_json::from_str` | 编译期 schema |
| `fs.readFile` | `tokio::fs::read` (P0-1 改 `fs_err`) | async + 错误增强 |
| `child_process.spawn` | `tokio::process::Command` | async 化 |

### 8.3 翻译验证 3 维度 (per §2.5)

| 维度 | 验证方法 | 通过条件 |
|---|---|---|
| 行为一致 | 17 模块 1:1 测试 (per `apeireth-architecture-readonly-review-2026-08-05.md` §3) | 17/17 行为一致 |
| API 一致 | HTTP API schema 跟 SpectrAI 0.9.21 比对 (per [0006-d-01](0006-d-01-tool-endpoint-real.md) D-01) | 6 工具 100% 兼容 |
| 数据兼容 | SQLite 5 表 schema 跟 SpectrAI 0.9.21 比对 (per [0009-d-07](0009-d-07-sqlite-to-postgres.md) D-07) | 5 表 100% 兼容 |
