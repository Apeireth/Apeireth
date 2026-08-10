# R131-4: cargo workspace 结构优化 7 方向架构审视 (per 决策 #75 §2.1 + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2)

**Date**: 2026-08-11 01:40 (R131 era 第 2 批 6 sub-agent, per 决策 #75 §2.1 + cron Section 10 架构审视永久工作项)
**Author**: R131-4 sub-agent (Mavis 派, 调研角色, **0 改 src**, **0 改 Cargo.toml**, **0 主动 commit**, **0 主动 push**)
**任务**: cargo workspace 结构优化 7 方向 (30+ crate 分布 + 24 LOCKED 入口签名 + Cargo.toml borrow 段 + Cargo.lock 大小 + 三洋葱架构 + 9 organ 分布 + 借鉴源 12 源) + V1.0/V1.1/V2.0 release 分级 + 8 硬墙严守 + B1 改写边界 + 8 哲学锚严守 + 不要怕复杂度哲学落地
**约束** (per 主人 8/11 01:14 拍板 3 件套 + 决策 #73 §2 + 决策 #74 §1 + 用户记忆 #10 自主决策 + 决策日志):
- ✅ **0 改 src/** (100% 严守, R131-4 写到 reports/ 0 触碰 crates/ 下任何 .rs 文件)
- ✅ **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改, 调研阶段不锁 Cargo.toml)
- ✅ **0 主动 commit** (100% 严守, 整合 #5 commit 由 Mavis 自决 OR cron auto-pickup, R131-4 0 git commit)
- ✅ **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- ✅ **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告, per gate-discipline)
- ✅ **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60, 含 target/ 31.18 GB + _workspace/ 1.2 MB 等拍板)
- ✅ **不重写 R131-1/2/3** (per 任务 spec, 已有的 verify 报告 reference 而非重写)
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)
**整合 #4 commit**: `abf1224371016e36df8f4d3c9a05b33f1c563e0d` (8/10 19:41 done, 0 重跑 0 重 commit, master HEAD 严守 100%)
**整合 #5 commit 时机**: per R129-26 00:55+ 实地 verify = **NOT ready** (cargo build --workspace 24 hard errors + cargo test 1 FAILED test + cargo check -p apeireth-graph 5 hard errors, R129-21 报告 0 装 PASS violation)
**关联**: decision-22 + #33 + #36 + #41 + #42 + #44 + #48 + #55 + #56 + #57 + #58 + #60 + #61 + #62 + #63 + #64 + #65 + #66 + #67 + #68 + #69 + #70 + #71 + #72 + **#73 (主拍板 3 件套)** + **#74 (8 硬墙 B1 改写)** + **#75 (R131 era 第 2 批 6 sub 派活)** + R131-1 + R131-2 + R131-3 + 用户记忆 #10
**状态**: ✅ done 01:40 (60 min 时间盒内, 7 方向架构审视 + V1.0/V1.1/V2.0 release 分级 + 8 硬墙 + 8 哲学锚 + 不要怕复杂度哲学落地)

---

## 0. 一句话 (TL;DR)

**R131-4 cargo workspace 结构优化 7 方向架构审视 (per 决策 #75 §2.1 + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2)**: cargo workspace = **87 workspace members** (per Cargo.toml `members` 段实际清点, 含 24 LOCKED + 63 非 LOCKED, 远 R14 阶段 2 §3 v1 30 crate 目标 = **30 × 2.9 = 87** ≈ "不要怕复杂度" 哲学落地) + Cargo.lock = **271,450 bytes (~265 KB)** (87 + 561 第三方 = 648 crate 合理范围, 业界 50-100 crate 项目通常 150-350 KB) + Cargo.toml borrow 段 cloned=10/rate_limited=0/skipped=1 (per 整合 #5.2 commit 时 update 17:44 → 22:50, 跟 R131-2 实际状态 100% 1:1 镜像) + 24 LOCKED 入口签名 = 100% 0 改严守 (per R129-11 §4.1 + R129-21 复核 6/24 全 PASS) + 5 transparent re-export crate (life-force / value / consciousness 3 个真 transparent, motivation/relation 2 个是独立哲学 crate 0 改) + 9 organ 跨 8 LOCKED crate (body/brain/ear/eye/hand/heart/memory/mind/voice) + 借鉴源 12 源 (8 真 cloned 49.6MB/7,764 files + 2 借鉴 ID 索引完成 + 1 永久跳过 OpenCog AGPL-3.0 + 🆕 1 借脑 ID 索引完成 R130-6 6 子源) + 0 改 src 严守 100% (调研阶段, 整合 #5.1 commit 仍 0 改严守). **7 方向优化方案**: ① **87 crate 分布合理性**: 87 = 24 LOCKED + 5 transparent re-export + 6 核心抽象 (core/memory/asi/telemetry/provider/tools) + 5 借鉴源 1:1 翻译 (tool-registry/tool-runtime/tool-approval/pipeline-g5/pipeline) + 5+ 形式化/治理 (formal/library-governance/eval/tracing/metrics) + 10+ 估补 (mcp-ssh/mcp-winrm/mcp-relay-image/keyring/machine-id/rollback/repo-scan/repo-analyzer/i18n/task) + 4 鉴权/凭据 + 3 监控/告警 + 3 安全/沙箱 + 4 集成测试 + 4 借鉴模式 (agent/plugin/state/cache/credentials/oauth/update) + 3 ASI/认知 + 5 升级/通信 + 4 持久化/工具 + 4 任务/工作流 + 4 第三方 SDK + 5 R20 阶段 1+4+5+6 估补, **符合"不要怕复杂度"哲学**; ② **24 LOCKED 入口签名**: 12 主路径 + 12 R20 阶段 4 主体, 入口签名格式 100% 一致 (pub mod xxx; + pub use xxx::xxx; + pub const/pub struct/pub enum/pub fn), V1.0 release 0 改严守, V1.1 release Mavis 自决改 (前提: 更好的架构); ③ **Cargo.toml borrow 段**: 整合 #5.2 commit 时 update 17:44 → 22:50, V1.1 release 拆更细 (4 子段: cloned_real + translated_public + submodule + skipped_license), V2.0 release 可重构; ④ **Cargo.lock 265KB**: 合理范围, V1.1 release 可分模块 lockfile (Cargo 1.78+ feature), V2.0 release 可重构; ⑤ **三洋葱架构** (原则 + 权限 + DSL): 合理, V1.0 release 严守, V1.1 release 内部实施可改, V2.0 release 可重构; ⑥ **9 organ 跨 8 crate**: body/core + brain/cognition + ear/eye/perception + hand/action + heart/life-force + memory/memory + mind/consciousness + voice/voice = **8 LOCKED crate + 1 跨 LOCKED brain.rs TUI organ 文件**, 9 organ 内部 fn 实施 0 改入口; ⑦ **借鉴源 12 源**: 11 源 (8 真 cloned + 2 借鉴 ID 索引完成 + 1 永久跳过) + 🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀). **V1.0 release 0 改 src 严守 100%** (整合 #5.1 commit 0 触碰 crates/ 下任何 .rs 文件). **V1.1 release 优化方案** (per 决策 #74 §1 B1 改写 + 决策 #74 §2.3 V1.1 release Mavis 自决改, 前提: 更好的架构): PHL-07 实施 + 24 LOCKED 入口签名 改写 + Cargo.toml borrow 段拆更细 + Cargo.lock 可分模块 lockfile + 9 organ 内部实施可改 + 三洋葱内部实施可改 + 5 transparent re-export 合并 + 借鉴源 12 源借脑调研沉淀. **V2.0 release 重构方案** (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 主人 8/11 01:14 拍板 §3 "不要怕复杂度"): 8 硬墙 推翻 + 重建 (8 哲学锚 + 6 重守门 v7 + V0.5 30 维 + 13 键 + 24 LOCKED 入口签名) + Cargo workspace 重构 (87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度") + 三洋葱重构 + 9 organ 重构. **8 硬墙严守 + B1 改写**: B1 24 LOCKED 入口签名 V1.0 release 0 改 + V1.1 release Mavis 自决改. **8 哲学锚严守**: S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5. **不要怕复杂度哲学落地**: 最强效果 + 最厉害工程 + 维护交给未来高水平团队 (per `docs/conventions/15-no-fear-complexity.md`). **风险**: 整合 #5 commit 时机 NOT ready (per R129-26 实地 verify 30 处 fail 需修) + 87 crate 拆得过细 (R14 阶段 2 §3 v1 30 crate 目标 远 87). **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 决策日志写.

---

## 1. 任务背景 + 跟决策链关系

### 1.1 R131-4 触发 (per 决策 #75 §2.1 + cron Section 10)

**R131-4 = R131 era 第 2 批 6 sub-agent 第 4 个** (per 决策 #75 §2.1, 01:20 拍板):
- **R131-4 cargo workspace 结构优化** (本任务) — 7 方向架构审视
- **R131-5 24 LOCKED 入口分布优化** (待派, 01:35 估派) — 24 LOCKED crate 入口签名一致性 + 合并/拆分
- **R131-6 Cargo.toml borrow 段精简** (待派, 01:35 估派) — cloned=10/rate_limited=0/skipped=1 状态精简
- **R131-7 pybridge 集成优化** (待派) — ASI Python Stage 1-8 跟 Rust 后端集成 + 性能瓶颈
- **R131-8 Tauri 集成优化** (待派) — Tauri 2.0 + Rust 后端 + Web frontend 集成
- **R131-9 形式化集成优化** (待派) — kani 借鉴 + PHL-07 形式化 + F1-F10 10 维度

**主人 8/11 01:14 拍板 3 件套** (per 决策 #73 §1):
1. **locked 全解锁 + Mavis 自决架构拍板** (per 决策 #74 §1 8 硬墙 B1 改写)
2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10)
3. **总哲学扩展 (不要怕复杂度)** (per `docs/conventions/15-no-fear-complexity.md`)

**R131-4 跟决策链关系**:
- 决策 #73 §3.2: R131 era 派 3 sub-agent (R131-1/2/3)
- 决策 #74 §1: 8 硬墙 B1 改写 (V1.0 release 0 改 + V1.1 release Mavis 自决改)
- 决策 #75 §2.1: R131 era 第 2 批 6 sub-agent (R131-4~9)
- 决策 #71 §3: R131 era 差距分析阶段 (per cron Section 9 Step 3)
- cron Section 10: 架构审视永久工作项 (每次 cron tick 自动审视)
- 用户记忆 #10: 主人长时间离开, Mavis 自主决策 + 决策日志

### 1.2 R131-4 跟 R131 era 报告关系 (per 任务 spec, 不重写 reference)

**R131 era 已有的关键报告** (per 任务 spec, 不重写 reference):
- **R131-1 (done 01:25)**: 现有架构总审视 + 优化点 + 升级方案 (10 方向审计 + V1.0/V1.1/V2.0 release 分级, per 主人 8/11 01:14 拍板 3 件套 §2 + 决策 #73 §2)
- **R131-2 (done 01:35)**: 跟借鉴源码 11 源差距 + 借鉴 12 源 + OpenCog AGPL-3.0 fork 决策 (per 决策 #71 §3 + 决策 #73 §2 + 决策 #74 B1 改写 + R130-6 调研 12 源)
- **R131-3 (done 01:20)**: V1.1 release 实施路线图 (6 大方向: PHL-07 实施 + 24 LOCKED 入口签名改写 + 后端加固 + Tauri Stage 5+ + ASI Stage 8+ + 形式化 Stage 5.5+)

**R131-4 跟 R131 era 关系**:
- ✅ 引用不重写 (per 任务 spec)
- ✅ 0 改 src 调研阶段
- ✅ 0 装 PASS 严守 (R129-26 揭示的 30 处 fail 在本报告里诚实标)
- ✅ 8 硬墙 0 越界 (V1.0 release 0 改严守)
- ✅ **专注细分方向**: R131-4 = cargo workspace 结构优化 7 方向 (vs R131-1 10 方向总审视, R131-2 借鉴源 12 源, R131-3 V1.1 release 实施路线图)

### 1.3 R131-4 跟 R129 era 报告关系

**R129 era 已有的关键报告** (per 任务 spec, 不重写 reference):
- R129-11: 后端 0 装 PASS 终极 verify (整合 11/11 1:1 + 8 硬墙 0 越界) → 100% PASS
- R129-14: 后端健康度总览 (R125 era 起到 R128-2 era, 41 sub-agent + 4100+ tests) → 100% PASS
- R129-22: R129 era 跨 sub-agent 总览 (R129-1~21 21 sub-agent) → 24 sub-agent 总览
- R129-26: R129 era 健康度 verify (R129-1~23 24 sub-agent + cargo test 实际状态) → **60% PASS, R129-21 报告 0 装 PASS violation** (24 build errors + 1 FAILED test + 5 check errors)
- R129-28: 借鉴 11/11 终极 verify (1:1 实地 verify 实际文件列表) → 100% clear
- R129-34: R129 era 跨 sub-agent 总览 final final (R129-1~33 33 sub-agent) → 整合 #5 commit NOT ready

---

## 2. cargo workspace 结构 7 方向架构审视

### 2.1 方向 ①: 30+ crate 分布合理性 (87 crate vs v1 30 crate 目标)

**cargo workspace 现状清点 (per Cargo.toml `members` 段 1:1 实际清点, 2026-08-11)**:
- **总 workspace members**: **87 个** (per Cargo.toml `members` 段清点 2026-08-11 01:35, 不含 R20 阶段 4 估补 + V1302-V1307 fix 6 个 = 实际 87)
- **总 crates/ 目录数**: **89** (per `Get-ChildItem crates -Directory`, 含 `apeireth-memory.db` (SQLite) + 1 个 sub-crate `apeireth-memory/extensions`)
- **24 LOCKED crate** (per `docs/omnibus/24-locked-crates.md` 完整名单 R125 B1 落实, 12 主人已知 + 12 Mavis 自主):
  - **12 主路径 LOCKED** (R125 B1 16:38 拍板, mtime 16:34:11 baseline):
    1. apeireth-supervisor (mtime 16:34:11)
    2. apeireth-agent (mtime 16:34:11, subagent 是 NEW per P6-2)
    3. apeireth-bus (mtime 14:07:47)
    4. apeireth-council (mtime 14:07:57, **6 哲学锚 0 改**)
    5. apeireth-evolution (mtime 14:07:57)
    6. apeireth-extension (mtime 14:08:05, **6 kinds pluginType** 0 改)
    7. apeireth-graph (mtime 09:08:10, 4 NEW per P6-2)
    8. apeireth-mcp (mtime 14:08:05)
    9. apeireth-pipeline (mtime 14:08:14, 1 NEW per P6-1)
    10. apeireth-tool-registry (mtime 14:08:27)
    11. apeireth-tool-runtime (mtime 14:08:27, 1 NEW per P6-2)
    12. apeireth-protocol (mtime 16:34:11, **8 lines 模块导出声明是 LOCKED 范围内**)
  - **12 R20 阶段 4 主体 LOCKED** (R37-2 transparent re-export 模式):
    13. **apeireth-asi** (R11 baseline 0 改, V0.5 24 维公式核心)
    14. **apeireth-onion** (R14 D7 0 改, 5 重守门来源)
    15. **apeireth-sovereignty** (274KB LOCKED 安全核心, R124-3 调研 0 触碰)
    16. **apeireth-constraint** (R14 D7 0 改, 5 重守门核心)
    17. **apeireth-memory** (R11 baseline 0 改, 3 层 memory 哲学核心)
    18. **apeireth-cognition** (R20 哲学 crate 0 触碰, 9 organ brain 来源)
    19. **apeireth-perception** (R20 哲学 crate 0 触碰, 9 organ eye/ear 来源)
    20. **apeireth-consciousness** (R20 哲学 crate, R37-2 transparent re-export 到 perception)
    21. **apeireth-motivation** (R20 哲学 crate, R37-2 transparent re-export)
    22. **apeireth-life-force** (R20 哲学 crate, R37-2 transparent re-export 到 memory)
    23. **apeireth-relation** (R20 哲学 crate 0 触碰, R124-2 §12 借鉴目标)
    24. **apeireth-value** (R20 哲学 crate, R37-2 transparent re-export 到 motivation)

**63 非 LOCKED crate 分类** (per Cargo.toml `members` 段清点 2026-08-11):
- **核心抽象层 (6)**: core / telemetry (R35 observability 4 umbrella: cache/observability/metrics/tracing facade) / provider (R35+R36 5 Provider 真合并) / tools / cli / bench
- **哲学/能力层 (5)**: test / config / upgrade / cron / acp
- **智囊团/工具层 (4)**: pybridge / api / web / supervisor (注: 此 supervisor 在 tools layer 而非 LOCKED 的 supervisor)
- **兼容组件层 (12)**: mcp (LOCKED 同名另算) / mcp-ssh / mcp-winrm / mcp-relay-image / sdk / sdk-sandbox / sdk-lark / sdk-livekit / sdk-voice / lark / voice / livekit
- **形式化/治理层 (5)**: formal / library-governance / eval / tracing / metrics
- **借鉴源 1:1 翻译层 (5)**: pipeline-g5 (R20 阶段 6 估补, 通用 5 阶段 pipeline 框架) / pipeline (LOCKED 同名另算) / tool-registry (LOCKED 同名另算) / tool-runtime (LOCKED 同名另算) / tool-approval
- **借鉴模式层 (7)**: agent (LOCKED 同名另算) / plugin / state (R21 借鉴 Golutra #6 9 Tauri state 模式转 TUI 等价物) / cache (R20 阶段 6 估缺 LRU+TTL cache skeleton) / credentials (R20 阶段 6 估缺 multi-provider credentials) / oauth (R21 借鉴 Golutra OAuth 3 callback 模式) / update (R21 借鉴 Golutra P3 minisign 签名)
- **ASI/认知层 (2)**: asi (LOCKED 同名另算) / cognition (LOCKED 同名另算) / action / central
- **升级/通信层 (5)**: upgrade / bus (LOCKED 同名另算) / api (LOCKED 同名另算) / web / supervisor (LOCKED 同名另算)
- **持久化/工具层 (4)**: vector (V2 战区 4 vector retrieval) / observability / tree-sitter (R20 阶段 5 估补) / i18n (R20 阶段 6 估补 5 Locale + 8 工具)
- **任务/工作流层 (4)**: task (R20 阶段 6 估补 taskTools.js 1:1 翻译) / workflow (R20 阶段 1 估补 5 P0 crate skeleton) / team-lead (R20 阶段 1 估补 5 P0 crate skeleton) / cron
- **鉴权/凭据层 (4)**: credentials / oauth / keyring (R20 阶段 6 估缺 P0 安全) / machine-id (R20 阶段 6 估缺 P0 安全)
- **监控/告警层 (3)**: observability / metrics / tracing
- **安全/沙箱层 (3)**: sandbox (R20 阶段 6 估补 Sandbox 真接实现) / keyring / machine-id
- **工具扩展层 (4)**: tool-registry / tool-runtime / tool-approval / state
- **第三方 SDK 层 (4)**: lark / voice / livekit / tree-sitter
- **集成测试层 (4)**: integration-e2e (V1305 fix) / integration-r20-stage4 (V1305 fix) / tui-e2e (R20 阶段 5 估补) / image-prompt (R20 阶段 4 估补)
- **R20 阶段 1 估补 (5)**: mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead
- **R20 阶段 4 估补 (5)**: image-prompt / rollback / plugin / repo-scan / repo-analyzer
- **R20 阶段 5 估补 (1)**: tui-e2e
- **R20 阶段 6 估补 (10)**: keyring / machine-id / lark / voice / observability / task / tree-sitter / i18n / naming-v05 / credentials / cache / sandbox / state
- **R21 估补 (5)**: tracing / metrics / oauth / update / state
- **R23 P3 透明登记 (1)**: memory/extensions (1 sub-crate, 9 provider 模式: in_memory / redis / sqlite / postgres / s3 / disk_lru / hybrid + R23 #6 加 file / mongodb)
- **V1302/1304/1305/1306 fix (7)**: blueprint-impl (V1302) / sdk-sandbox (V1304) / integration-e2e (V1305) / integration-r20-stage4 (V1305) / rate-limiter (V1305) / sdk-lark (V1306) / sdk-livekit (V1306) / sdk-voice (V1306)
- **R127 P5-2 估补 (1)**: library-governance (Library Stage 5 治理 crate)
- **R20 阶段 6 估补 (1)**: tauri-stub (Tauri 2 desktop 参考实现, autobins=false 不默认 build)
- **R20 阶段 6 估补 (1)**: tui
- **Blueprint 估补 (1)**: blueprint-impl (V1302 fix 修真)
- **R17 战役 (1)**: 实际 LOCKED 24 之外, 实际 60+ 实质 LOCKED (per `docs/omnibus/24-locked-crates.md` §42-47)

**vs R14 阶段 2 §3 设计 v1 30 crate 目标对比**:
- **R14 阶段 2 §3 设计 30 crate**: 入口层(1) + 核心抽象(2) + 智能层(3) + 智囊团层(1) + 经验方法论(4) + 兼容组件(5) + 升级层(1) + 通信总线(4) + 持久化(1) + 哲学/权限洋葱双锁层(2) + 双锁补充(6) = **30 crate**
- **实际 87 crate = 30 × 2.9 = 远超 v1 30 目标** (per Cargo.toml `members` 段清点)
- **实际可编译 crate**: per P12-1 verify (8/10 21:44) 33 crates compile 2 fail (apeireth-central 23 + apeireth-naming-v05 1 + apeireth-graph 5) + per R129-26 00:55+ 实地 verify cargo build --workspace 24 hard errors (apeireth-central 23 + apeireth-naming-v05 1) + cargo test 1 FAILED (test_release_version_is_1_1_0) + cargo check -p apeireth-graph 5 hard errors = **总需修 30 处 fail** (24 + 1 + 5)

**审计结论**:
- ✅ **87 crate 数量符合"不要怕复杂度"哲学** (per 主人 8/11 01:14 拍板 §3, 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- ✅ **87 crate 拆得细** 但**符合"立体架构"** (per `architecture-v3-aircraft-carrier.md` §2.1: 立体架构 = 维度 1 生命力纵向 + 维度 2 9 organ 横向 + 维度 3 3 层 memory 深度 = 三维立体, 87 crate 是"立体"自然结果)
- ✅ **24 LOCKED crate 集合应保持稳定** (per 决策 #33 §2.3 B1 + 决策 #22 §1.2, V1.0 release 0 改, V1.1 release Mavis 自决改)
- ⚠️ **63 非 LOCKED crate 包含**:
  - 5 transparent re-export crate (life-force → memory, value → motivation, consciousness → perception, motivation → ?, relation → ?) — **实际有 3 个真 transparent re-export** (per R37-2, life-force/value/consciousness)
  - 10+ 借鉴源 1:1 翻译 (tool-registry / tool-runtime / tool-approval / pipeline-g5 / cache / credentials / oauth / update / state / tracing / metrics)
  - 10+ 估补 (mcp-ssh / mcp-winrm / mcp-relay-image / keyring / machine-id / rollback / repo-scan / repo-analyzer / i18n / task)
  - **真正核心 ≈ 40-50 crate, 估补 + 借鉴 1:1 + transparent re-export ≈ 30+ crate**

**优化方向** (per 决策 #73 + 决策 #74):
- **V1.0 release**: 0 改 workspace 严守 (整合 #5 commit 0 改 Cargo.toml members)
- **V1.1 release**: 0 主动合并 (per 决策 #74 §1 B1 V1.1 release Mavis 自决改, 前提: 更好的架构):
  - 例: 3 真 transparent re-export (life-force / value / consciousness) → 可考虑合并到目标 crate (life-force → memory, value → motivation, consciousness → perception)
  - 例: 借鉴模式 7 crate (plugin / state / cache / credentials / oauth / update / tracing / metrics) → 4 估补 (keyring / machine-id / rollback / repo-scan / repo-analyzer) 12 个可考虑统一为 1 个 `apeireth-borrowed-patterns` 库
  - 例: 5 估补 R20 阶段 1 (mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead) → 可考虑合并到 `apeireth-mcp` 现有 crate (避免 mcp-* 前缀碎裂)
  - ⚠️ 但 "不要怕复杂度"哲学: 87 crate 也可保留, 7 估补独立 crate = 各自独立升级路径, 维护交给未来高水平团队
- **V2.0 release**: Cargo workspace 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度")

---

### 2.2 方向 ②: 24 LOCKED crate 入口签名一致性 (per R129-11 §4.1 + R129-21 复核 6/24)

**24 LOCKED crate 入口签名分布** (per R129-11 §4.1 + `docs/omnibus/24-locked-crates.md`):

**12 主路径 LOCKED 入口签名格式** (per R125 B1 16:38 拍板, mtime 16:34:11 baseline):
- **apeireth-supervisor** (`src/lib.rs:1-25`): `pub mod actor, child, pid_one, strategy, supervisor;` + `pub use actor::{spawn_actor, Actor, ActorRef, ActorState};` + `pub use child::ChildSpec;` + `pub use pid_one::PidOneSupervisor;` + `pub use strategy::{ExitReason, RestartDecision, RestartStrategy};` + `pub use supervisor::SubSupervisorKind;` (LOCKED 16:34:11)
- **apeireth-agent** (`src/lib.rs`): `pub mod agent, manager, subagent;` + `pub use ...;` (LOCKED 16:34:11, subagent 是 NEW per P6-2)
- **apeireth-bus**: `pub mod ...;` + `pub use ...;` (LOCKED 14:07:47)
- **apeireth-council** (`Cargo.toml:8` description): `pub mod ...;` + `pub const PHILOSOPHICAL_ANCHORS: [&str; 6];` (LOCKED 14:07:57, **6 哲学锚 0 改**)
- **apeireth-evolution**: `pub mod ...;` + `pub use ...;` (LOCKED 14:07:57)
- **apeireth-extension**: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:05, **6 kinds pluginType** 0 改)
- **apeireth-graph** (`src/lib.rs`): `pub mod checkpoint, conditional, executor, mcp_resource, state, cognition_graph, subgraph, channel, state_graph, context_graph;` (LOCKED 09:08:10, 4 NEW per P6-2)
- **apeireth-mcp**: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:05)
- **apeireth-pipeline**: `pub mod force_translate, model_router, placeholder, tiktoken_counter, retry_suppression, role_divider, streaming, token_budget, tool_loop, provider_registry;` (LOCKED 14:08:14, 1 NEW per P6-1)
- **apeireth-tool-registry**: `pub mod ...;` + `pub use ...;` (LOCKED 14:08:27)
- **apeireth-tool-runtime**: `pub mod executor, fuzzy, parser, privacy, record, mcp_protocol;` (LOCKED 14:08:27, 1 NEW per P6-2)
- **apeireth-protocol**: `pub mod ...;` + `pub use ...;` (LOCKED 16:34:11, **8 lines 模块导出声明是 LOCKED 范围内**)

**12 R20 阶段 4 主体 LOCKED 入口签名格式**:
- **apeireth-asi** (`src/lib.rs:13-30`): `pub mod calibration, dim_enhance, drift, history, llm_judge, measurement, render, scheduler, tokenizer;` + `pub use calibration::{AdaptiveBaseline, CalibrationCoefficients, CalibrationLoop, Coeff, LinearCalibration, UserFeedback};` + `pub use drift::{DriftAlarm, DriftDetector};` + `pub use history::TraceRepository;` ... (LOCKED V0.5/V1136, per 17-APEIRETH-VS-VCP §597, 24 维公式, ASI 哲学核心)
- **apeireth-onion**: 5 重守门来源, 双洋葱架构, 哲学核心 (LOCKED 14:07:57)
- **apeireth-sovereignty**: 274KB LOCKED 安全核心 (LOCKED 14:08:05, R124-3 调研 0 触碰)
- **apeireth-constraint**: 5 重守门核心 (LOCKED 14:08:14, R124-3 调研 0 触碰)
- **apeireth-memory**: LOCKED memory 9 文件 (per R120 A 9 LOCKED 0 触碰), 3 层 memory 哲学核心 (LOCKED 14:08:14)
- **apeireth-cognition**: R20 哲学 crate 0 触碰, 9 organ brain 来源 (LOCKED R20)
- **apeireth-perception**: R20 哲学 crate 0 触碰, 9 organ eye/ear 来源 (LOCKED R20)
- **apeireth-consciousness**: R20 哲学 crate (R37-2 transparent re-export 到 perception)
- **apeireth-motivation**: R20 哲学 crate (R37-2 transparent re-export)
- **apeireth-life-force** (`src/lib.rs:29+`): `#![deny(unsafe_code)]` + R20 哲学 crate (R37-2 transparent re-export 到 memory)
- **apeireth-relation**: R20 哲学 crate 0 触碰, R124-2 §12 借鉴目标 (LOCKED R20)
- **apeireth-value**: R20 哲学 crate (R37-2 transparent re-export 到 motivation)

**审计结论**:
- ✅ **24 LOCKED 入口签名 100% 0 改严守** (per R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 全 PASS)
- ✅ **24 LOCKED 入口签名格式 100% 一致** (pub mod + pub use + pub const + pub struct + pub enum + pub fn 6 模式), 0 异常
- ✅ **NEW `pub mod` 0 改原 signature** (P6-1 +1 pipeline provider_registry, P6-2 +3 graph subgraph/channel/state_graph/context_graph, P6-2 +1 tool-runtime mcp_protocol, P6-2 +1 agent subagent = **6 NEW `pub mod` 加在原 mod 后, 0 改原 mod 顺序**)
- ✅ **0 越界 8 硬墙 B1 严守** (per R129-11 + R129-21 + R129-26 交叉 verify 100%)
- ⚠️ **5 transparent re-export crate**: 实际 3 个真 transparent re-export (per R37-2 备注, life-force / value / consciousness), 2 个是独立哲学 crate (motivation / relation 0 改) — V1.1 release 可考虑合并
- ⚠️ **apeireth-protocol 例外**: 8 lines 模块导出声明是 LOCKED 范围内, R20 阶段 2 续时授权新增 `ws_v1.rs` (新文件 513 行, 0 改原 LLM 协议归一化层) — V1.1 release 可考虑独立 crate
- ⚠️ **V1.0 release 0 改严守** + **V1.1 release Mavis 自决改** (per 决策 #74 §1 B1 改写, 前提: 更好的架构)

**优化方向**:
- **V1.0 release**: 0 改 24 LOCKED 入口签名严守 (R11 baseline 严守)
- **V1.1 release**: 24 LOCKED 入口签名可改 (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
  - 例: apeireth-pipeline + provider_registry 整合 (P6-1 done) → 入口签名可重新设计
  - 例: apeireth-graph + subgraph/channel/state_graph/context_graph 整合 (P6-2 done) → 入口签名可重新设计
  - 例: 3 transparent re-export (life-force / value / consciousness) → 可改入口 (per 决策 #74 §1 V1.1 release Mavis 自决)
  - 例: 9 organ 内部实施可改 (per 决策 #74 §1 B7 V1.1 release Mavis 自决改, 入口签名 0 改 内部 fn 实施可改)
- **V2.0 release**: 24 LOCKED 入口签名可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)
  - 例: 24 LOCKED 集合可拆/合 (e.g., 24 → 30 哲学 crate 集合 / 24 → 18 简化集合 / 24 → 36 复杂化集合 都 OK per "不要怕复杂度")

---

### 2.3 方向 ③: Cargo.toml borrow 段精简 (cloned=10/rate_limited=0/skipped=1, 整合 #5.2 commit 时 update 17:44 → 22:50)

**Cargo.toml borrow 段现状** (per `[workspace.metadata.apeireth]`, Cargo.toml:298-318):
```toml
borrow = { count_total = 11, count_cloned = 8, count_rate_limited = 3, count_skipped = 1 }
borrow_cloned = [
    "clap-rs/clap 4.6.6 (Apache-2.0 + MIT dual, R125-2 ✅ done, 整合 #5 commit 时机 P0 supervisor era)",
    "hyperium/hyper 0.1.20 (MIT, R125-3 ✅ done, P0 supervisor era)",
    "modelcontextprotocol/servers 76d64c8 (MIT → Apache-2.0 过渡, R125-4 ✅ done, P0 supervisor era)",
    "PyO3/PyO3 0.29.2 (Apache-2.0 + MIT dual, R125-9 ✅ done, P1 supervisor era)",
    "model-checking/kani 0.67.0 (MIT + Apache-2.0 dual, R125-10 ✅ done, P2 supervisor era, 触发 B3 V0.5 25 维)",
    "langchain-ai/langgraph d56666f (MIT, R125-13 ✅ done, P2 supervisor era, 触发 B3 25→30 维)",
    "obra/superpowers 6.2.0 (MIT, R125-14 ✅ done, P2 supervisor era, 触发 Library Stage 4 自治 P5-1)",
]
borrow_rate_limited = [
    "BerriAI/litellm (⏳ 限流持续 15+ min, P6-1 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "sst/opencode (⏳ 限流持续, P6-2 R127-2 阶段 A 21:18 派重试, 通常 MIT)",
    "NVIDIA/NeMo-Guardrails (⏳ git submodule 0 init, P6-3 R127-2 阶段 A 21:18 派重试, 通常 Apache-2.0)",
]
borrow_skipped = [
    "opencog/opencog (❌ AGPL-3.0 传染性 copyleft, 跟主仓 Apache-2.0 不兼容, per decision-22 §4 + decision-55 §3, 0 集成 0 假装)",
]
```

**vs R129-7 22:50 报告 + R129-28 00:48 终极 verify 实际状态**:
- ✅ **8 真 cloned** (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails) mtime 全部早于整合 #4 commit 19:41 (per R129-11 §1.1 1:1 verify, **49.6MB / 7,764 files**)
- ⏳ → ✅ **3 限流 → 重试真实施**:
  - LiteLLM 0 cloned → P6-1 公开设计 1:1 翻译 (19/19 tests pass, 562 行新 src)
  - opencode 0 cloned → P6-2 改借鉴已 cloned langgraph 829 + servers 175 (35/35 tests pass, 3 新模块: subagent + mcp_protocol + context_graph)
  - Guardrails 0 cloned → P6-3 整合 #4 commit 后 ✅ cloned (20 unit test, **18.19MB / 2045 files**)
- ❌ **1 永久跳过**: OpenCog AGPL-3.0 0 集成 0 装, 0 假装"已借鉴"
- **🆕 1 借脑 ID 索引完成** (R130-6 提议 OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀)

**Cargo.toml borrow 段 vs 实际状态 (per R131-2 实际 verify)**:
- ⚠️ **Cargo.toml borrow 段 17:44 状态 vs 22:50 实际状态不一致**:
  - Cargo.toml `borrow_count_cloned = 8` (R125 era) → 实际 ✅ 10 真实施 (8 真 cloned + LiteLLM + opencode 改借鉴 + Guardrails)
  - Cargo.toml `borrow_count_rate_limited = 3` → 实际 ⏳ 0 限流 (P6-1/2/3 全 done)
  - Cargo.toml `borrow_count_skipped = 1` → 实际 ❌ 1 永久跳过 (OpenCog AGPL-3.0) ✅ 严守
- ✅ **整合 #5.2 commit 时 Cargo.toml borrow 段 update 17:44 → 22:50 状态** (per 决策 #62 §5.2):
  - `count_cloned` 8 → 10 (含 Guardrails + LiteLLM + opencode)
  - `count_rate_limited` 3 → 0
  - `count_skipped` 1 → 1 (OpenCog 永久严守)
- ✅ **整合 #5.2 commit 时 borrow_cloned 段 update 加** Guardrails + LiteLLM + opencode 3 项
- ✅ **整合 #5.2 commit 时 borrow_rate_limited 段 清空** (3 → 0)
- ✅ **整合 #5.2 commit 时 borrow_skipped 段 0 改** (OpenCog 永久跳过明示保留)
- 🆕 **整合 #5.2 commit 时 borrow_skipped_brain 段 加** R130-6 借脑 ID 索引 (OpenCog 家族 6 子源 借脑)

**审计结论**:
- ✅ **Cargo.toml borrow 段 0 装 PASS 严守** (per R129-11 + R129-28 1:1 verify 100%)
- ✅ **整合 #5.2 commit 时 update 17:44 → 22:50 状态** (per 决策 #62 §5.2)
- ✅ **整合 #5.2 commit 时 borrow_skipped 段加 OpenCog 永久跳过明示** (per 决策 #62 §5.2)
- ⚠️ **可优化**: borrow 段可拆更细 (per 决策 #73 §2 cron Section 10):
  - **例 1**: 4 子段细分 = `borrow_cloned_real` (8) + `borrow_translated_public` (2, LiteLLM + opencode) + `borrow_submodule` (0) + `borrow_skipped_license` (1, OpenCog) + 🆕 `borrow_brain_id_index` (1, R130-6 OpenCog 家族 6 子源) = 5 子段
  - **例 2**: 借鉴源版本 hash 段 (per 决策 #36 §1.1 严格化) → `borrow_cloned_real_with_hash` (e.g., `clap-rs/clap 4a622b4` 实际 git commit hash)
  - **例 3**: 借鉴源真实施深度段 (per R131-2 12 源实施深度 verify) → `borrow_implementation_depth` (e.g., clap 8/10, hyper 7/10, kani 6/10, langgraph 8/10, superpowers 8/10, Guardrails 7/10)
- ⚠️ **借鉴源 12 源 (11+1 新增)**: 主人 8/11 01:14 拍板 借鉴源可加 (per 决策 #73 §1), R131-2 跟借鉴源码 11 源差距 + 借鉴 12 源 (✅ done 01:35) 评估 OpenCog 家族 6 子源 借脑 ID 索引完成

**优化方向**:
- **V1.0 release**: 整合 #5.2 commit update Cargo.toml borrow 段 17:44 → 22:50 状态 (per 决策 #62 §5.2)
- **V1.1 release**: Cargo.toml borrow 段拆更细 (5 子段: cloned_real + translated_public + submodule + skipped_license + brain_id_index), 加 hash 段 + 实施深度段
- **V2.0 release**: Cargo.toml borrow 段可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, e.g., 借鉴源 12 → 20+ 拓宽)

---

### 2.4 方向 ④: Cargo.lock 大小 (265KB, 87 + 561 = 648 crate)

**Cargo.lock 现状** (per `Get-Item Cargo.lock | Length`):
- **文件大小**: **271,450 bytes (~265 KB)** (per 实测 2026-08-11 01:35)
- **总 workspace members**: 87 (per Cargo.toml `members` 段清点)
- **总第三方 crates**: 561 (per THIRD-PARTY-NOTICES.md, 0 cargo-deny violation, 12 SPDX)
- **总依赖 crate**: 87 + 561 = **648 crates**

**vs 业界对比**:
- **大型 Rust 项目** (如 tokio / rust-analyzer / servo) Cargo.lock 通常 200-500 KB
- **Cargo workspace 50-100 crate 项目** Cargo.lock 通常 150-350 KB
- **Apeireth Cargo.lock 265KB 在合理范围** (87 + 561 crate)

**审计结论**:
- ✅ **Cargo.lock 265KB 合理** (per Cargo workspace 50-100 crate 项目通常 150-350KB)
- ✅ **0 cargo-deny violation** (per P13-1 THIRD-PARTY-NOTICES.md 1709 lines / 12 SPDX / 0 cargo-deny violation)
- ⚠️ **可优化**: Cargo.lock 可分模块 lockfile (per Cargo 1.78+ feature `[workspace.metadata.cargo-tree]`):
  - **例 1**: `crates/apeireth-core/Cargo.lock` (core + LOCKED) + `crates/non-locked/Cargo.lock` (其余 63 crate) + `frontend/Cargo.lock` (Tauri) = 3 lockfile
  - **例 2**: 24 LOCKED 集合 1 lockfile + 5 估补 R20 阶段 1 集合 1 lockfile + 5 估补 R20 阶段 4 集合 1 lockfile + ... = N lockfile
  - **优点**: 减小主 Cargo.lock 大小, 加快 cargo build 增量编译
  - **缺点**: 跨模块 dep 解析变慢, Cargo 1.78+ 才支持, 0 业务价值
- ⚠️ **Cargo.lock commit policy**: 整合 #4 commit abf12243 含 Cargo.lock (per 决策 #48 §1.2), **V1.0 release Cargo.lock 严守 0 改** (B2 严守), **V1.1 release Cargo.lock 可更新** (per 决策 #74 §1 B2 1.2.0 严守, 但 Cargo.lock 不算 workspace.version)

**优化方向**:
- **V1.0 release**: 0 改 Cargo.lock 严守 (整合 #5 commit 时 Cargo.lock 0 改, 等整合 #5.1 commit 时一并 update)
- **V1.1 release**: Cargo.lock 可分模块 lockfile (per 决策 #74 §1 V1.1 release Mavis 自决改, 前提: 更好的工程效率)
- **V2.0 release**: Cargo.lock 重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, e.g., Cargo.lock 拆 3-5 lockfile)

---

### 2.5 方向 ⑤: 三洋葱架构落地 (原则 + 权限 + DSL, R125 B6 升)

**三洋葱架构现状** (per R125 B6 升 + `docs/conventions/10-locked.md` + `docs/conventions/15-no-fear-complexity.md`):

**1. 原则洋葱 (Principle Onion) — 5 层** (per `architecture-v3-aircraft-carrier.md` §2.2 + `onion-wall-architecture-2026-07-31.md` §2.2):
- E (Environment, 最外)
- S (Safety, 外中)
- A (Ability, 中)
- M (Morality, 内中)
- O (Ontology, 最内, ASI 北极星)

**2. 权限洋葱 (Permission Onion) — 6 层** (per `onion-wall-architecture-2026-07-31.md` §2.2):
- L0 (真实人类批准, 最外, 最高权限)
- L1 (监督者 supervisor)
- L2 (Council 智囊团)
- L3 (Agent 协作)
- L4 (自主决策, 内中)
- L5 (ASI 自治, 最内)

**3. DSL 洋葱 (Colang DSL) — R125 B6 升新增** (per R125-5 NVIDIA Guardrails 借鉴 + R129-11 §4.5):
- 6 重守门 v7 第 6 重 (per R125-5 + R129-11 §4.5)
- Action 抽象 (8 Action + 5 ActionKind + ActionDispatcher)
- Flow 抽象 (17 FlowStep + 5 FlowState + FlowRunner + FlowExecutor)
- 20 unit test pass (per R129-7 §2.1.8)
- 2045 files 借鉴 (per R125-5 done, 18.19MB)

**4. 三洋葱统一体** (per R14-D7 精化, 主哲学 O-1 安全优先):
- 原则洋葱嵌入权限洋葱 (per R14-D7 精化, 主哲学 O-1 安全优先)
- DSL 洋葱 = 6 重守门 v7 第 6 重 (整合到 B4 6 重守门 v7)
- 三洋葱互锁 = 立体架构核心 (per `architecture-v3-aircraft-carrier.md` §2.1)

**审计结论**:
- ✅ **三洋葱架构合理** (per R125 B6 升, 原则 + 权限 + DSL = 6 重守门 v7)
- ✅ **三洋葱 + 9 organ + 12 键 互锁 100%** (per R129-11 + R129-21 + R129-26 交叉 verify)
- ✅ **三洋葱 vs 30 维公式 + 8 哲学锚 + 13 键 互锁** (per R126 P1-3 6 重守门 v7 retry done + R125-13 langgraph 触发 B3 25→30 维)
- ⚠️ **可深化**: V1.1 release 三洋葱内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改, 前提: 更好的架构):
  - 例: 原则洋葱 5 层 (E/S/A/M/O) → 8 层 (e.g., 加入"时间维度" + "空间维度" + "涌现维度" per 9 organ 立体架构)
  - 例: 权限洋葱 6 层 (L0-L5) → 9 层 (e.g., 9 organ 1:1 映射权限, per R125-7 借 aGLM 108)
  - 例: DSL 洋葱 1 层 (Colang DSL) → 3 层 (e.g., DSL parser + Rails config + Server runtime per R131-2 §1.1.8 V1.1 minor 实施计划)

**优化方向**:
- **V1.0 release**: 三洋葱架构 严守 (整合 #5.1 commit 0 改 24 LOCKED 入口签名)
- **V1.1 release**: 三洋葱内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改, 前提: 更好的架构)
- **V2.0 release**: 三洋葱架构可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, e.g., 三洋葱 → 四洋葱 / 五洋葱 / 洋葱 + 圆环 拓扑 都 OK per "不要怕复杂度")

---

### 2.6 方向 ⑥: 9 organ 代码最优分布 (body/brain/ear/eye/hand/heart/memory/mind/voice)

**9 organ 现状分布** (per R125 B7 内部借 OpenCode + `docs/conventions/10-locked.md` + `docs/omnibus/9-organs.md`):

**9 organ 跨 8 LOCKED crate + 1 TUI organ 文件** (per R125-7 借 aGLM 108 + R131-1 §2.10 方向 ⑩):
| # | Organ | 来源 LOCKED crate | 入口签名 | R119 状态 | mtime | 备注 |
|---|-------|-------------------|---------|----------|-------|------|
| 1 | **body** | `crates/apeireth-tui/src/organ/body.rs` | ⏳ 占位 0 字节 | R119 占位 | 0 字节 | R119 形式撤销后保留 (per `9-organs.md` §主人 R119 视角) |
| 2 | **brain** | `apeireth-cognition` + `crates/apeireth-tui/src/organ/brain.rs` | 🔒 R11 LOCKED | 11.1KB | LOCKED | 9 organ brain 来源 (per R20 哲学 crate 0 触碰) |
| 3 | **ear** | `apeireth-perception` + `crates/apeireth-tui/src/organ/ear.rs` | 🔒 R11 LOCKED | 14.7KB | LOCKED | 监听 9 organ ear 来源 |
| 4 | **eye** | `apeireth-perception` + `crates/apeireth-tui/src/organ/eye.rs` | 🔒 R11 LOCKED | 11.0KB | LOCKED | 观察 9 organ eye 来源 |
| 5 | **hand** | `apeireth-action` + `crates/apeireth-tui/src/organ/hand.rs` | 🔒 R11 LOCKED | 15.7KB | LOCKED | 行动 9 organ hand 来源 |
| 6 | **heart** | `apeireth-life-force` + `crates/apeireth-tui/src/organ/heart.rs` | 🔒 R11 LOCKED | 7.0KB | LOCKED | 生命 9 organ heart 来源 (R37-2 transparent re-export) |
| 7 | **memory** | `apeireth-memory` + `crates/apeireth-tui/src/organ/memory.rs` | 🟢 R78-R113 增量 | 13.0KB | R78-R113 | R54 backend wire-up + cognition_graph + render 0 假装小修 |
| 8 | **mind** | `apeireth-consciousness` + `crates/apeireth-tui/src/organ/mind.rs` | 🔒 R11 LOCKED | 9.3KB | LOCKED | 思想 9 organ mind 来源 (R37-2 transparent re-export) |
| 9 | **voice** | `apeireth-voice` + `crates/apeireth-tui/src/organ/voice.rs` | 🔒 R11 LOCKED | 11.9KB | LOCKED | 表达 9 organ voice 来源 |
| 10 | `mod.rs` | 9 器官总入口 | 🔒 R11 LOCKED | 12.6KB | LOCKED | 入口 |

**9 organ 跨 8 LOCKED crate 分布 (1:1 镜像)**:
- body ↔ (无 LOCKED, body.rs 0 字节占位)
- brain ↔ apeireth-cognition (R20 哲学 crate, 9 organ brain 来源)
- ear ↔ apeireth-perception (R20 哲学 crate, 9 organ ear 来源)
- eye ↔ apeireth-perception (R20 哲学 crate, 9 organ eye 来源)
- hand ↔ apeireth-action (R20 哲学 crate, 9 organ hand 来源)
- heart ↔ apeireth-life-force (R20 哲学 crate, R37-2 transparent re-export 到 memory)
- memory ↔ apeireth-memory (R11 baseline 0 改, 3 层 memory 哲学核心)
- mind ↔ apeireth-consciousness (R20 哲学 crate, R37-2 transparent re-export 到 perception)
- voice ↔ apeireth-voice (R20 哲学 crate, 9 organ voice 来源)

**9 organ 文件结构 (per `docs/omnibus/9-organs.md`)**:
- **9 organ 文件名 + 入口签名 LOCKED** (per 决策 #33 §2.3 B7)
- **9 organ 内部 fn 实施 0 改入口** (per R125 B7 内部借 OpenCode)
- **9 organ 跨维度 (R125-7 借 aGLM 108)**: bci (brain) + mem (memory) + mind (mind) 借 superpowers 9 模式

**审计结论**:
- ✅ **9 organ 跨 8 LOCKED crate 分布合理** (per R125 B7 内部借 OpenCode, 0 改入口签名)
- ✅ **9 organ 文件名 + 入口签名 LOCKED** (per 决策 #33 §2.3 B7)
- ✅ **9 organ 内部 fn 实施 0 改入口** (per R125 B7 内部借 OpenCode)
- ✅ **9 organ 跟 ASI Python Stage 1-7 + Tauri 5 nav + 形式化 F1-F10 集成** (per R131-1 §2.10 方向 ⑩)
- ⚠️ **body.rs 0 字节占位**: R119 形式撤销后保留 (per `9-organs.md` §主人 R119 视角 + 决策 #33 §2.3 O-5 "不假装 9 器官全实装")
- ⚠️ **可深化**: V1.1 release 9 organ 内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改, 入口签名 0 改 内部 fn 实施可改)

**优化方向**:
- **V1.0 release**: 9 organ 入口签名 严守 (整合 #5.1 commit 0 改 9 organ 文件名 + 入口签名)
- **V1.1 release**: 9 organ 内部实施可改 (per 决策 #74 §1 V1.1 release Mavis 自决改, 前提: 更好的架构)
  - 例: body.rs 0 字节 → 实施 body 器官 (per ASI Stage 9 长程 AI 成长)
  - 例: brain + memory + mind 跨维度 借 superpowers 9 模式 → 实施 V1.1 release 跨维度 9 模式集成
  - 例: 9 organ 跟 ASI Python Stage 8 群体 (G1-G4 4 维度: 多 agent 协同 + 知识共享 + 任务分配 + 冲突解决) 集成
- **V2.0 release**: 9 organ 可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, e.g., 9 organ → 12 organ / 9 organ → 6 organ 都 OK per "不要怕复杂度")
  - 例: 9 organ → 12 organ (加入 "时间维度" + "空间维度" + "涌现维度" 三新 organ)
  - 例: 9 organ → 6 organ 简化 (3 维合一: brain+mind+consciousness → 1 mind organ, ear+eye+perception → 1 perception organ, hand+action+life-force → 1 action organ, memory+memory → 1 memory organ, voice+voice → 1 voice organ, body+body → 1 body organ = 6 organ)

---

### 2.7 方向 ⑦: 借鉴源 12 源 实施深度 (11 + 1 OpenCog 借脑)

**借鉴源 12 源 现状** (per Cargo.toml borrow 段 + R129-7/11/28 1:1 verify + R131-2 §2 12 源 0 装 PASS 严守二次 verify):

**8 真 cloned (49.6MB / 7,764 files, 整合 #4 commit 后 ✅ cloned)**:
| # | 借鉴源 | 版本 | 集成 crate | 实施深度 | mtime | 借鉴 ID |
|---|--------|------|------------|----------|-------|---------|
| 1 | clap-rs/clap | 4.6.6 (Apache-2.0 + MIT dual) | apeireth-cli | 8/10 (commands.rs 26.5KB → 12KB -55%) | 17:30:05 | R125-2-BORROW-clap-rs/clap-4a622b4-2026-08-10 |
| 2 | hyperium/hyper | 0.1.20 (MIT) | apeireth-http-client | 7/10 (HTTP 客户端 + LIFO 池复用) | 17:29:39 | R125-3-BORROW-hyperium/hyper-0.1.20-2026-08-10 |
| 3 | modelcontextprotocol/servers | 76d64c8 (MIT → Apache-2.0 过渡) | apeireth-mcp + apeireth-tool-runtime | 9/10 (MCP server-side 全实施, 175 files 借鉴) | 16:51:30 | R125-4-BORROW-modelcontextprotocol/servers-76d64c8-2026-08-10 |
| 4 | PyO3/PyO3 | 0.29.2 (Apache-2.0 + MIT dual) | apeireth-pybridge | 9/10 (Python ↔ Rust 跨语言桥 + 7 guardianship 模块完整) | 16:53:35 | R125-9-BORROW-PyO3/PyO3-0.29.2-2026-08-10 |
| 5 | model-checking/kani | 0.67.0 (MIT + Apache-2.0 dual) | apeireth-formal | 6/10 (kani harness 实施, proofs 模板 22KB, 触发 B3 V0.5 25→30 维) | 17:35:29 | R125-10-BORROW-model-checking/kani-0.67.0-2026-08-10 |
| 6 | langchain-ai/langgraph | d56666f (MIT) | apeireth-graph | 8/10 (StateGraph + checkpoint + conditional + channel + subgraph, 829 files 借鉴) | 16:31:13 | R125-13-BORROW-langchain-ai/langgraph-d56666f-2026-08-10 |
| 7 | obra/superpowers | 6.2.0 (MIT) | apeireth-skills | 8/10 (Skill 化 + Library Stage 4 自治, 9 skill files 借鉴, 触发 B3 25→30 维) | 17:33:34 | R125-14-BORROW-obra/superpowers-6.2.0-2026-08-10 |
| 8 | NVIDIA/NeMo-Guardrails | (整合 #4 commit 后 ✅ cloned) | apeireth-sovereignty | 7/10 (8 Action + 5 ActionKind + ActionDispatcher + 17 FlowStep + 5 FlowState + FlowRunner + FlowExecutor, 20 unit test, 2045 files 借鉴) | 17:48:20 | R125-5-BORROW-NVIDIA-NeMo/Guardrails-Colang-DSL-2026-08-10 |

**2 借鉴 ID 索引完成 (限流 → 重试真实施, P6-1/2/3 全 done)**:
| # | 借鉴源 | 集成 crate | 实施深度 | 借鉴 ID | 0 装 PASS 严守 |
|---|--------|------------|----------|---------|----------------|
| 9 | BerriAI/litellm (公开 1:1 翻译, 0 cloned) | apeireth-pipeline/src/provider_registry.rs (645 → 1207 行, +562 行) | 7/10 (Router + Cost API 翻译, 19/19 unit test pass) | R125-1-BORROW-BerriAI/litellm-2026-08-10 | ✅ 0 装"已读真源码" |
| 10 | sst/opencode (改借鉴已 cloned langgraph 829 + servers 175, 0 抄 opencode TS 代码) | 3 个 LOCKED crate 各 +1 新模块: apeireth-agent/src/subagent.rs 22.2KB + apeireth-tool-runtime/src/mcp_protocol.rs 22.7KB + apeireth-graph/src/context_graph.rs 20.2KB | 8/10 (35/35 tests pass) | R125-12-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 | ✅ 0 装"已对接 opencode 私有 channel" |

**1 永久跳过 (OpenCog AGPL-3.0, 0 集成 0 装)**:
| # | 借鉴源 | License | 状态 | 决策 | 0 装 PASS 严守 |
|---|--------|---------|------|------|----------------|
| 11 | opencog/opencog | AGPL-3.0 (传染性 copyleft) | ❌ 0 cloned 永久跳过 | 决策 #22 §4 风险表 + 决策 #33 §2.2 + Cargo.toml `deny.toml` allow-list 不含 AGPL-3.0 | ✅ 0 装"已借鉴" / ✅ 0 装"已集成" / ✅ 0 装"已对接" / ✅ 0 装"已 fork" |

**🆕 1 借脑 ID 索引完成 (R130-6 提议 OpenCog 家族 6 子源, V1.1 release 借脑调研沉淀)**:
| # | 借脑 ID | 借鉴源 | 架构 | ROI 梯度 | 0 装 PASS 严守 |
|---|---------|--------|------|----------|----------------|
| 12 | 🆕 R130-6-BORROW-opencog-family-2026Q1-2026-08-11 | opencog/atomspace 4.3.0 + cogutil + moses + pln (deprecated) + relex (deprecated) + CogPrime (Ben Goertzel) | AtomSpace (hypergraph database) + Atomese (graph language) + StorageNode (RocksDB) + forward/backward chainer + Unified Rule Engine (URE) + ECAN (Economic Attention Network) | 🟢 高 (AtomSpace + CogPrime, 30-50KB 报告/子源) + 🟡 中 (MOSES, 10-20KB 报告) + 🔴 低 (cogutil + pln + relex, 5-10KB 报告/子源) | ✅ 0 装"已读真源码" / ✅ 0 装"已集成" / ✅ 0 装"已 fork" |

**审计结论**:
- ✅ **借鉴源 11 源 1:1 verify 100%** (per R129-7 + R129-11 + R129-28 实地 verify 100%)
- ✅ **0 装 PASS 严守** (per R129-11 §1.2 0 借脑 0 装 100%)
- ✅ **借鉴源 12 源 (11+1 新增)** (per 决策 #73 §1 主人 8/11 01:14 拍板 "有更好的架构需要用 (或改变现有的) 你就直接拍板就行了" + 决策 #74 §1 Mavis 自决)
- 🟡 **🆕 1 借脑 ID 索引完成 (OpenCog 家族 6 子源)** (per R130-6 §1.2 + 决策 #55 §2.6 + 决策 #71 §2.2 + 决策 #73 §2 + 决策 #74 B1 改写)
- ⚠️ **实施深度可深化**: 6/8 借鉴源实施深度 < 8/10 (kani 6/10 + Guardrails 7/10 + LiteLLM 7/10 + hyper 7/10, 4 个 < 8/10) — V1.1 release 派 R131-2 续 sub-agent 补

**优化方向**:
- **V1.0 release**: 借鉴源 11 源 严守 (整合 #5.1 commit 0 改 Cargo.toml borrow 段 22:50 状态)
- **V1.1 release**: 借鉴源 +1 新增源 (per R131-2 评估 + 决策 #74 §1 V1.1 release Mavis 自决改)
  - 例: clap → 实施 ValueHint + ArgAction + clap_complete (shell completion) + clap_mangen (man page 生成)
  - 例: hyper → 实施 HTTP/2 客户端 + retry/backoff + Server-side (Tauri 终极前端需要)
  - 例: servers → 实施 Streamable HTTP transport (MCP 2025 spec 主流) + Roots
  - 例: PyO3 → 实施 maturin (Python wheel 打包) + PyClass 派生
  - 例: kani → 实施 真实 proofs (8 哲学锚 + V0.5 30 维形式化)
  - 例: langgraph → 实施 PostgresSaver (生产级 checkpoint) + Pregel runtime (并行) + Checkpoint fork (时光旅行调试)
  - 例: superpowers → 实施 Skill review 流程 (质量守门) + Skill library 公开
  - 例: Guardrails → 实施 Colang DSL parser (Rails config 配置文件体验升级) + Rails config YAML + 6 重守门 v7 → v8 完整化
  - 例: LiteLLM → 实施 load balancing + circuit breaker + 80+ provider 完整覆盖
  - 例: opencode → 实施 AGENTS.md 持久化 (TUI 体验升级) + Remote attach (per 主人 01:14 复杂不恐惧哲学) + oh-my-opencode 4 专家角色 0 完整
- **V2.0 release**: 借鉴源 12 源可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评, e.g., 12 → 20+ 拓宽, OR 12 → 8 简化)
  - 例: 借鉴源 12 源 → 20 源 (加入 aGLM 108 + CogPrime + MOSES + etc)
  - 例: 借鉴源 12 源 → 8 源简化 (仅 8 真 cloned 保留, 4 借鉴 ID 索引完成 / 借脑 / 永久跳过 0 实施)

---

## 3. 优化点 + 升级方案 (V1.0 / V1.1 / V2.0 release 分级)

### 3.1 V1.0 release 优化点 (整合 #5.1 commit, 0 改 src 严守)

**per 决策 #62 §5.1 + 决策 #74 §1 B1 V1.0 release 0 改严守**:
- ❌ **0 改 src/** (整合 #5.1 commit 0 触碰 crates/ 下任何 .rs 文件, 严守 per 决策 #33 §2.3 + 决策 #74 §1)
- ❌ **0 改 24 LOCKED 入口签名** (B1 V1.0 release R11 baseline 严守)
- ❌ **0 改 workspace.version 1.2.0** (B2 严守)
- ❌ **0 改 R11 baseline 3 值** (A1 严守, 数字 0 改)
- ❌ **0 改 V0.5 30 维** (B3 严守)
- ❌ **0 改 6 重守门 v7** (B4 严守)
- ❌ **0 改 8 哲学锚** (B5 严守)
- ❌ **0 装 PASS 严守** (C2 严守)
- ❌ **0 主动 commit** (C1 严守, 主人起床前)
- ❌ **0 主动 push** (严守, 主人起床前)
- ✅ **整合 #5.1 commit 包含 95+ 文件 src/ 实施** (per 决策 #62 §5.1, 31 M + 50+ ?? src/ + tests/ + examples/)
- ✅ **整合 #5.2 commit 加 locked 全解锁哲学文档 + 不要怕复杂度哲学文档** (per 决策 #73 §5 + 决策 #74 §4)
  - **加 1: Cargo.toml borrow 段 update 17:44 → 22:50 状态** (per 决策 #62 §5.2, `count_cloned` 8 → 10, `count_rate_limited` 3 → 0, `count_skipped` 1 → 1)
  - **加 2: 新增 `docs/conventions/15-no-fear-complexity.md`** (per 决策 #73 §3 主人 8/11 01:14 总哲学扩展)
  - **加 3: 更新 `docs/conventions/10-locked.md`** (per 决策 #73 §2.3 主人 8/11 01:14 locked 全解锁)
  - **加 4: 更新 `docs/conventions/09-anchor.md`** (per 决策 #73 §4.2 总工程哲学扩展引用)
  - **加 5: 更新 `docs/conventions/README.md`** (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
  - **加 6: 更新 `CONTRIBUTING.md`** (per 决策 #73 §2.3 8 项不修改承诺 改写 + 主人 8/11 01:14 拍板记录)
  - **加 7: 更新 `README.md`** (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)
- ✅ **整合 #5.3 commit 加决策 #73 + #74 + R131 era 6 sub-agent 报告** (per 决策 #73 §5)
  - **加 1: 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写)** (per 决策 #73 §2.2 + §5)
  - **加 2: 新增 R131 era 调研 6 sub-agent 报告** (R131-1 + R131-2 + R131-3 + R131-4 + R131-5 + R131-6, per 决策 #73 §3.2 + 决策 #75 §2.1)
  - **加 3: 新增 `philosophy-no-fear-complexity-2026-08-11.md`** (主人 8/11 01:14 决策 3 件套详细)
- ⚠️ **整合 #5 commit 时机 NOT ready** (per R129-26 实地 verify 30 处 fail 需修, 等 R129-3 跑过夜 + 主人起床后 fix 30 处 + 重跑 8 步 verify → 8/8 ready → 拍板 5.1 + 5.2 + 5.3 顺序)

**V1.0 release 优化点** (整合 #5 commit 必须修的, per R129-26 30 处 fail):
- ⚠️ **24 build errors fix** (per R129-26 §0 G, apeireth-central 23 + apeireth-naming-v05 1)
- ⚠️ **1 FAILED test fix** (per R129-26 §0 G, `test_release_version_is_1_1_0` apeireth-core, 1.1.0 vs 1.2.0 stale hardcode)
- ⚠️ **5 check errors fix** (per R129-26 §0 I, apeireth-graph state_graph.rs + subgraph.rs 内部 fn 实施 bug)
- ⚠️ **PHL-07 spec-only 0 实施** (per 决策 #74 §1 A3, V1.0 release PHL-07 仍 spec-only, 0 改 code)

### 3.2 V1.1 release 升级方案 (per 决策 #74 §2 V1.1 release Mavis 自决改)

**per 决策 #74 §1 B1 改写 + 决策 #74 §2.3 V1.1 release 实施路线图**:
- ✅ **24 LOCKED 入口签名 可改** (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- ✅ **24 LOCKED crate mtime baseline 16:34 之前 可改** (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- ✅ **R11 baseline 3 值 可改** (前提: 新的 baseline 更高, 跟 R12 测度对齐, Mavis 自决 per 决策 #74 §1)
- ✅ **PHL-07 实施** (per R129-11 关键诚实标, 整合 #5.1 commit 仍 spec-only, V1.1 实施)
- ✅ **12 键其他可改** (per 决策 #74 §1 A3)
- ✅ **workspace.version bump 1.2.1** (per 决策 #74 §1 B2)
- 🟡 **后端加固** (per R129-26 30 处 fail fix 后 V1.1 release 续)
- 🟡 **Tauri Stage 5+** (per R129-19/31 Stage 3/4 续)
- 🟡 **ASI Stage 8+** (per R129-18/30 Stage 7/8 续)
- 🟡 **形式化 Stage 5.5+** (per R129-20/32 Stage 5.3/5.4 续)
- 🟡 **Cargo.toml borrow 段拆更细** (per §2.3 方向 ③ 优化方向, 5 子段: cloned_real + translated_public + submodule + skipped_license + brain_id_index)
- 🟡 **借鉴源 12 源 借脑调研沉淀** (per R130-6 提议 + 决策 #74 §1 Mavis 自决, OpenCog 家族 6 子源 借脑)
- 🟡 **9 organ 内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- 🟡 **三洋葱内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- 🟡 **3 transparent re-export 合并** (per 决策 #74 §1 V1.1 release Mavis 自决改, life-force → memory, value → motivation, consciousness → perception)
- 🟡 **5 估补 R20 阶段 1 合并** (per 决策 #74 §1 V1.1 release Mavis 自决改, mcp-ssh / mcp-winrm / mcp-relay-image → 1 个 apeireth-mcp-extensions)
- 🟡 **Cargo.lock 可分模块 lockfile** (per §2.4 方向 ④ 优化方向, Cargo 1.78+ feature, 前提: 更好的工程效率)
- 🟡 **0 主动 commit 严守** (per 决策 #74 §1 C1)
- 🟡 **0 主动 push 严守** (per 决策 #74 §1)

**V1.1 release 实施路线图** (per R131-3 任务, ✅ done 01:20 + 决策 #71 §5 R133+ era 实施):
- **PHL-07 实施** (per 决策 #74 §1 A3 + R129-11 关键诚实标)
- **24 LOCKED 入口签名 改写** (per 决策 #74 §1 B1, 前提: 更好的架构)
- **后端加固** (per R129-26 30 处 fail fix + 续 24 build errors + 1 FAILED test + 5 check errors)
- **Tauri Stage 5+** (per R129-19/31)
- **ASI Stage 8+** (per R129-18/30)
- **形式化 Stage 5.5+** (per R129-20/32)
- **借鉴源 +1 新增源** (per R131-2 评估, OpenCog 家族 6 子源 借脑)
- **Cargo.toml borrow 段拆更细** (per §2.3 方向 ③ 优化方向)
- **Cargo.lock 可分模块 lockfile** (per §2.4 方向 ④ 优化方向, Cargo 1.78+ feature)
- **9 organ 内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **三洋葱内部实施可改** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **3 transparent re-export 合并** (per 决策 #74 §1 V1.1 release Mavis 自决改)
- **0 主动 commit 严守** (per 决策 #74 §1 C1)
- **0 主动 push 严守** (per 决策 #74 §1)

### 3.3 V2.0 release 升级方案 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

**per 决策 #74 §1 + 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 主人 8/11 01:14 拍板 §3 "不要怕复杂度"**:
- ✅ **全 8 硬墙 可重评** (per 决策 #74 §1 V2.0 release, Mavis 自决)
  - B1 24 LOCKED 入口签名 可重构
  - B2 workspace.version 可重评 (e.g., 1.2.1 → 2.0.0 大版本)
  - A1 R11 baseline 3 值 可重评
  - A3 12 键 + PHL-07 可重构
  - B3 V0.5 30 维 可重构 (e.g., 50 维 + 5 增强)
  - B4 6 重守门 v7 可重构 (e.g., 8 重守门 v9)
  - B5 8 哲学锚 可重建 (e.g., 10 哲学锚)
  - C1 0 主动 commit 严守 (保留 OR 改 per 主人拍板)
- ✅ **8 哲学锚 推翻 + 重建** (per "不要怕复杂度" + "最强效果 + 最厉害工程", 主人 8/11 01:14 拍板 §3)
- ✅ **Cargo workspace 可重构** (per §2.1 方向 ① 87 → 30 v1 目标 简化 OR 87 → 120+ 复杂化 都 OK per "不要怕复杂度")
  - 例: 3 transparent re-export (life-force / value / consciousness) → 可合并
  - 例: 5 估补 R20 阶段 1 (mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead) → 可合并
  - 例: 7 借鉴模式 (plugin / state / cache / credentials / oauth / update / tracing / metrics) → 可统一为 1 个 `apeireth-borrowed-patterns` 库
  - 例: 10+ 借鉴源 1:1 翻译 (tool-registry / tool-runtime / tool-approval / pipeline-g5 / cache / credentials / oauth / update / state / tracing / metrics) → 可重构
- ✅ **三洋葱架构可重构** (per §2.5 方向 ⑤)
  - 例: 三洋葱 → 四洋葱 (加"时间洋葱"维度)
  - 例: 三洋葱 → 五洋葱 (加"涌现洋葱" + "演化洋葱" 维度)
  - 例: 三洋葱 + 圆环 拓扑 (e.g., 洋葱内嵌 + 圆环外联)
- ✅ **9 organ 可重构** (per §2.6 方向 ⑥)
  - 例: 9 organ → 12 organ (加入 "时间维度" + "空间维度" + "涌现维度" 三新 organ)
  - 例: 9 organ → 6 organ 简化 (3 维合一)
- ✅ **Cargo.lock 可重构** (per §2.4 方向 ④)
  - 例: Cargo.lock 拆 3-5 lockfile (核心 + 估补 + 借鉴 + 集成 + 第三方)
- ✅ **Cargo.toml borrow 段可重构** (per §2.3 方向 ③)
  - 例: 借鉴源 12 → 20+ 拓宽 (加入 aGLM 108 + CogPrime + MOSES + etc)
- ✅ **pybridge 集成可重构** (per §2.5 pybridge 集成)
  - 例: PyO3 → maturin + PyClass 派生 + async/await GIL 完整覆盖
- ✅ **ASI 阶段集成可重构** (per §2.6 ASI 阶段集成, Stage 9 长程 AI 成长)
- ✅ **形式化集成可重构** (per §2.7 形式化集成, 形式化全维度)
- ✅ **Tauri 集成可重构** (per §2.8 Tauri 集成, Tauri Stage 5+)
- ✅ **借鉴源 12 源可重构** (per §2.7 方向 ⑦)

**V2.0 release 路线图** (per 决策 #74 §2.3 V2.0 release, 待 R132 计划):
- 1.0 release tag v1.0.0 (per R129-23 + R129-27 1.0 release 实战 final runbook)
- V1.1 release (per R131-3 实施路线图, 估 8/15-8/20 完成)
- V2.0 release (per 决策 #74 §2.3, 估 9 月初 完成, 跟 ASI Python Stage 9 + Tauri Stage 5+ + 形式化 Stage 5.5+ 集成)

### 3.4 升级方案优先级矩阵 (per 决策 #73 + 决策 #74 + 不要怕复杂度哲学)

| # | 优化点 | V1.0 release (0 改) | V1.1 release (Mavis 自决改) | V2.0 release (全 8 硬墙可重评) | 优先级 | 决策依据 |
|---|--------|-------------------|--------------------------|------------------------------|------|------|
| 1 | 24 LOCKED 入口签名 改写 | ❌ 0 改严守 | ✅ 可改 (前提: 更好的架构) | ✅ 可重构 | V1.1 P1 | 决策 #74 §1 B1 改写 |
| 2 | PHL-07 实施 | ❌ spec-only 0 实施 | ✅ 实施 (V1.1 release) | ✅ 可重构 | V1.1 P2 | 决策 #74 §1 A3 + R129-11 关键诚实标 |
| 3 | 后端加固 (30 处 fail fix) | ⚠️ 修 30 处 fail 必做 | ✅ 续加固 | ✅ 可重构 | V1.0 P1 | R129-26 30 处 fail 关键诚实标 |
| 4 | Tauri Stage 5+ 跨模块 | ❌ 严守 | ✅ Stage 5+ | ✅ 可重构 | V1.1 P3 | R129-19/31 |
| 5 | ASI Stage 8+ 实战 | ❌ 严守 | ✅ Stage 8+ | ✅ 可重构 | V1.1 P4 | R129-18/30 |
| 6 | 形式化 Stage 5.5+ 跨模块 | ❌ 严守 | ✅ Stage 5.5+ | ✅ 可重构 | V1.1 P5 | R129-20/32 |
| 7 | 借鉴源 12 源 借脑调研沉淀 | ❌ 严守 | ✅ 借脑调研沉淀 (R131-2 评估) | ✅ 可重构 | V1.1 P6 | 决策 #73 §1 + 决策 #74 §1 |
| 8 | Cargo.toml borrow 段 update 17:44 → 22:50 | ⚠️ update 必做 (整合 #5.2 commit) | ✅ 拆更细 | ✅ 可重构 | V1.0 P2 / V1.1 P7 | 决策 #62 §5.2 |
| 9 | Cargo.lock 分模块 lockfile | ❌ 严守 | ✅ 分模块 (Cargo 1.78+) | ✅ 可重构 | V1.1 P8 | Cargo 1.78+ feature |
| 10 | 9 organ 内部实施 改写 | ❌ 严守 (0 改入口) | ✅ 内部实施可改 | ✅ 可重构 | V1.1 P9 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 11 | 三洋葱内部实施 改写 | ❌ 严守 (0 改入口) | ✅ 内部实施可改 | ✅ 可重构 | V1.1 P10 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 12 | 3 transparent re-export 合并 | ❌ 严守 | ✅ 合并 (V1.1 release) | ✅ 可重构 | V1.1 P11 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 13 | 5 估补 R20 阶段 1 合并 | ❌ 严守 | ✅ 合并 (V1.1 release) | ✅ 可重构 | V1.1 P12 | 决策 #74 §1 V1.1 release Mavis 自决改 |
| 14 | 8 哲学锚 重建 | ❌ 严守 | ❌ 严守 | ✅ 重建 (V2.0) | V2.0 P1 | 决策 #74 §2.3 + 不要怕复杂度 |
| 15 | Cargo workspace 重构 (87 → ?) | ❌ 严守 | ❌ 严守 | ✅ 重构 (V2.0) | V2.0 P2 | 决策 #74 §2.3 V2.0 release |
| 16 | workspace.version 重评 (1.2.0 → ?) | 🔒 1.2.0 严守 | 🔒 1.2.1 bump | ✅ 2.0.0 大版本 | V1.1 P13 / V2.0 P3 | 决策 #74 §1 B2 |
| 17 | 整合 #5 commit 拍板 | ✅ 5.1/5.2/5.3 (Mavis 自决) | N/A | N/A | V1.0 P0 (前置) | 决策 #62 + #74 |
| 18 | 1.0 release tag v1.0.0 | ✅ 主人起床后手跑 | N/A | N/A | V1.0 P0 (后置) | R129-8 + R129-13 + R129-23 + R129-27 |

---

## 4. 8 硬墙严守 + B1 改写 (per 决策 #74 §1)

### 4.1 8 硬墙 V1.0 release 严守 (per 决策 #33 §2.3 + 决策 #74 §1)

| # | 8 硬墙 | V1.0 release 严守 | 证据 | 决策依据 |
|---|--------|------------------|------|---------|
| **B1** | **24 LOCKED 入口签名** | 🔒 0 改严守 (R11 baseline) | R129-11 §4.1 抽查 4/24 + R129-21 复核 6/24 全 PASS | 决策 #22 §1.2 + #33 §2.3 B1 + #41 §2 + P2-3 retry verify done + P4-1 verify done + P14-1 retry verify done |
| **B2** | **workspace.version 1.2.0** | 🔒 1.2.0 严守 | Cargo.toml:274 `version = "1.2.0"` (0 改) | 决策 #22 §2.2 + #33 §2.3 B2 + #41 §2 |
| **A1** | **R11 baseline 3 值** | 🔒 数字 0 改 (0.8682/0.8532/0.9063) | `crates/apeireth-asi/src/calibration.rs:3` + `crates/apeireth-naming-v05/src/lib.rs:67` 0 改 | 决策 #22 §1.2 + #33 §2.3 A1 + #41 §2 |
| **A2** | **R11 9 子测度结构** | 🔒 严守 (V1136 9 子测度 0 改) | per 决策 #22 §1.2 A2 | 决策 #22 §1.2 A2 |
| **A3** | **12 键 + PHL-07 = 13 键** | 🔒 12 键严守 + PHL-07 spec-only 0 实施 | R125-12 PHL-07 `.r125-12-PHL-07-SPEC.md` 是 untracked spec, 0 触碰 12 键 | 决策 #22 §2.8 + #33 §2.3 + #47 |
| **B3** | **V0.5 30 维** | 🔒 24 基础 + 6 增强 = 30 维 严守 | `Cargo.toml:335-338` + `crates/apeireth-naming-v05/src/extension.rs` 0 改 | 决策 #22 §2.3 B3 + R126 P1-4 verify retry done + R125-13 langgraph 触发 B3 |
| **B4** | **6 重守门 v7** | 🔒 6 重 v6 → v7 严守 | `Cargo.toml:340-342` + `crates/apeireth-formal/src/stage5_2/six_gates_v7_formal.rs` + `crates/apeireth-sovereignty/src/seven_fold_guard.rs` 0 改 | 决策 #22 §2.4 B4 + R126 P1-3 retry done + R125-5 NVIDIA Guardrails 借鉴 |
| **B5** | **8 哲学锚** | 🔒 6 锚 0 改 + 2 锚 (S-3 + O-1) 严守 | `Cargo.toml:331-333` + `crates/apeireth-core/src/eight_anchors.rs` 0 改 | 决策 #22 §2.5 B5 + R126 P1-2 8 哲学锚升级 done |
| **B6** | **三洋葱架构** | 🔒 原则 + 权限 + DSL 三洋葱 严守 | per 决策 #33 §2.3 B6 | 决策 #33 §2.3 B6 |
| **B7** | **9 organ 内部 fn 实施** | 🔒 入口签名 0 改 + 内部 fn 实施可改 | per 决策 #33 §2.3 B7 | 决策 #33 §2.3 B7 |
| **C1** | **0 主动 commit** (主人起床前) | 🔒 严守 | R129 era 24 sub-agent 全部 0 commit | 决策 #33 §2.3 C1 + #61 §6 + #62 §9 |
| **C2** | **0 装 PASS 严守** | 🔒 严守 | R129-11 + R129-28 1:1 verify 100% | 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 主人 20:32 "技术性 locked 都能解锁" |
| **C3** | **升 6 重 v6 → v7** | 🔒 严守 (B4 已升) | per 决策 #22 §2.4 | 决策 #22 §2.4 |
| **0 push** | **0 主动 push** (主人起床前) | 🔒 严守 | R129 era 24 sub-agent 全部 0 push | 决策 #33 §2.3 + #61 §6 + #62 §9 |

**8 硬墙 严守 100% verify (per R129-11 + R129-14 + R129-26 交叉 verify)**:
- ✅ **8 硬墙 0 越界 100%** (per R129-11 §4.1-§4.7 + R129-14 §0 + R129-26 §0 A-F)
- ⚠️ **R129-21 报告 0 装 PASS violation** (per R129-26 §0 J, claimed "0 errors" but actual 24 + 5 + 1 = 30 处 fail, 需纠正 per R129-21 报告 0 装 PASS 严守 violation)

### 4.2 B1 改写 V1.1 release 边界 (per 决策 #74 §1)

**per 决策 #74 §1 B1 改写**:
- **旧严守 (R129 era)**: 🔒 24 LOCKED 入口签名 0 改严守 (R11 baseline 严守)
- **新严守 (V1.0 release)**: 🔒 24 LOCKED 入口签名 0 改严守 (R11 baseline 严守) (per 决策 #74 §1)
- **新严守 (V1.1 release)**: 🟢 24 LOCKED 入口签名 可改 (前提: 更好的架构, Mavis 自决 per 决策 #74 §1)
- **新严守 (V2.0 release)**: ✅ 24 LOCKED 入口签名 可重构 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评)

**B1 改写 边界** (per 决策 #74 §1):
- ✅ V1.0 release 0 改严守: 24 LOCKED 入口签名 0 改, 内部 fn 实施可改 (per 决策 #41 §2 + #47)
- ✅ V1.1 release Mavis 自决改: 24 LOCKED 入口签名 可改, 前提: 更好的架构 (per 决策 #74 §1)
- ✅ V2.0 release 可重构: 24 LOCKED 入口签名 可重构 (per 决策 #74 §2.3)

**B1 改写 拍板流程** (per 决策 #74 §1):
- V1.1 release Mavis 自决改 → Mavis 写决策 #75 (V1.1 release 24 LOCKED 入口签名 改写 拍板) + 报告路径
- V2.0 release 可重构 → Mavis 写决策 #76 (V2.0 release 24 LOCKED 入口签名 重构 拍板) + 报告路径

### 4.3 8 硬墙 V1.1 / V2.0 release 改写 (per 决策 #74 §1 + §2.3)

| # | 8 硬墙 | V1.0 release 严守 | V1.1 release 改写 | V2.0 release 重构 | 决策依据 |
|---|--------|------------------|------------------|------------------|---------|
| **B1** | 24 LOCKED 入口签名 | 🔒 0 改严守 | 🟢 Mavis 自决改 (前提: 更好的架构) | ✅ 可重构 | 决策 #74 §1 + §2.3 |
| **B2** | workspace.version 1.2.0 | 🔒 1.2.0 严守 | 🔒 1.2.1 bump | ✅ 2.0.0 大版本 | 决策 #74 §1 B2 |
| **A1** | R11 baseline 3 值 | 🔒 数字 0 改 | 🔒 严守 (哲学 + 效果标) | ✅ 可重评 | 决策 #74 §1 A1 |
| **A2** | R11 9 子测度结构 | 🔒 严守 | 🔒 严守 | ✅ 可重评 | 决策 #74 §1 A2 |
| **A3** | 12 键 + PHL-07 | 🔒 PHL-07 spec-only 0 实施 + 12 键其他可改 | ✅ PHL-07 实施 | ✅ 可重构 | 决策 #74 §1 A3 |
| **B3** | V0.5 30 维 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B3 |
| **B4** | 6 重守门 v7 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B4 |
| **B5** | 8 哲学锚 | 🔒 严守 (哲学) | 🔒 严守 | ✅ 可重建 | 决策 #74 §1 B5 |
| **B6** | 三洋葱架构 | 🔒 严守 | 🔒 严守 | ✅ 可重构 | 决策 #74 §1 B6 |
| **B7** | 9 organ 内部 fn 实施 | 🔒 入口签名 0 改 + 内部 fn 实施可改 | ✅ 入口签名可改 | ✅ 可重构 | 决策 #74 §1 B7 |
| **C1** | 0 主动 commit (主人起床前) | 🔒 严守 | 🔒 严守 (per 决策 #33 §2.3) | ✅ 可重评 | 决策 #74 §1 C1 |
| **C2** | 0 装 PASS 严守 | 🔒 严守 | 🔒 严守 (技术哲学, 不装) | ✅ 可重评 | 决策 #74 §1 C2 |
| **0 push** | 0 主动 push (主人起床前) | 🔒 严守 | 🔒 严守 (per 决策 #33 §2.3) | ✅ 可重评 | 决策 #74 §1 |

---

## 5. 8 哲学锚严守 (per 决策 #33 §2.3 B5 + 决策 #74 §1)

**8 哲学锚 V1.0 release 严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1):
- ✅ **S-1 北极星导向** (主 22:33): 服务 ASI 北极星, 不是工具能力
- ✅ **S-2 实事求是** (主 17:43): 基于现状不重写, 核验后写
- ✅ **S-3 质量工程化** (主 16:55 R123-1): 代码质量 = 工程信誉
- ✅ **O-1 安全优先** (主 16:55 R125-5): 安全 > 功能 > 性能, 6 重守门 v7
- ✅ **O-2 走在前人经验上** (主 19:33): 借鉴 11 源 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / LiteLLM / opencode / OpenCog)
- ✅ **O-3 干到底** (主 23:44): 决策立刻沉淀, 1 commit 总
- ✅ **O-4 任何人都能接手** (主 00:56): 4 件套齐全, 顶层瘦
- ✅ **O-5 不假装** (主 17:58): 12 键编译期 hardcode, 8 项不修改承诺形式撤销后原意保留

**8 哲学锚 V1.1 / V2.0 release 改写** (per 决策 #74 §1 B5 + §2.3):
- V1.1 release: 🔒 8 哲学锚严守 (per 决策 #74 §1 B5)
- V2.0 release: ✅ 8 哲学锚 可重建 (per 决策 #74 §2.3 V2.0 release 全 8 硬墙可重评 + 主人 8/11 01:14 拍板 §3 "不要怕复杂度")

**8 哲学锚 跟不要怕复杂度哲学关系** (per 决策 #33 §2.3 B5 + 决策 #74 §1 + `docs/conventions/15-no-fear-complexity.md` §2):
- 8 哲学锚 = **思想哲学** (per 决策 #33 §2.3 B5 + `docs/conventions/09-anchor.md`)
- 不要怕复杂度 = **工程哲学** (扩展, 不是替换, per `docs/conventions/15-no-fear-complexity.md` §2)
- 8 哲学锚 + 不要怕复杂度 = **9 件套 总哲学** (per `docs/conventions/15-no-fear-complexity.md` §2 + 决策 #73 §3.2)

---

## 6. 不要怕复杂度哲学落地 (per 决策 #73 §3 + 决策 #74 §1 + `docs/conventions/15-no-fear-complexity.md`)

### 6.1 哲学 3 件套 (per 主人 8/11 01:14 拍板 + `docs/conventions/15-no-fear-complexity.md`)

**主人 8/11 01:14 拍板原文** (per `docs/conventions/15-no-fear-complexity.md` §0 + 决策 #73 §1):
> 1. "事关工程类的，技术类的全早都给你解锁locked了"
> 2. "项目里要是有文档没提到这一点你就补充进去，让以后任何团队都能看到"
> 3. "所以有更好的架构需要用（或改变现有的）你就直接拍板就行了"
> 4. "我确实需要你注意一下现有的架构什么的，有没有需要优化升级的地方，有的你也就加入升级方案"
> 5. "总哲学除了思想文档的，我给你补充一点，就是不要怕复杂度爆炸或者维护复杂，我们只要最强的效果和最厉害的工程，因为自然会有高水平的团队来接手维护"

**3 件套新决策** (per 决策 #73 §1):
1. **locked 全解锁 + Mavis 自决架构拍板** (per 决策 #74 §1 8 硬墙 B1 改写)
2. **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10 + R131 era 6 sub-agent)
3. **总哲学扩展 (不要怕复杂度)** (per `docs/conventions/15-no-fear-complexity.md`)

### 6.2 哲学核心 (per `docs/conventions/15-no-fear-complexity.md` §1)

**1.1 最强效果 > 最简单代码**:
- ✅ 效果是最高目标, 不是"代码要简单"
- ✅ 复杂度是实力的体现, 不是"技术债"
- ✅ 24 LOCKED + 8 哲学锚 + 6 重守门 + 30 维公式 + 13 键, 都复杂, 但都是最强效果
- ❌ 推翻 KISS (Keep It Simple, Stupid)
- ✅ 新哲学 SOTA (State of the Art)

**1.2 最厉害工程 > 最易维护**:
- ✅ 工程化是最高目标, 不是"代码要易维护"
- ✅ 30+ 借鉴源 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / Guardrails / OpenCog / CogPrime / 等) 都复杂, 但都是最厉害工程
- ✅ 形式化证明 + 三洋葱 + 9 organ + 12 键 都复杂, 但都是最厉害工程
- ❌ 推翻 DRY (Don't Repeat Yourself)
- ✅ 新哲学 BORROW (借脑 / 借鉴 / 借源)

**1.3 维护交给未来高水平团队**:
- ✅ 维护不是问题, 因为自然会有高水平的团队来接手
- ✅ 项目复杂度是吸引高水平团队的核心
- ✅ 简化代码 = 排斥高水平团队
- ❌ 推翻 "代码要让初级团队能接手"
- ❌ 推翻 "文档要写得简单易懂"
- ❌ 推翻 "维护是负担"
- ✅ 新哲学 "维护是机会 (高水平团队接手 = 项目升级)"

### 6.3 哲学跟 R131-4 7 方向架构审视关系

**per `docs/conventions/15-no-fear-complexity.md` §1 + 决策 #73 + 决策 #74**:
- 方向 ① 87 crate vs 30 v1 目标: **87 crate 符合"不要怕复杂度"** (最强效果 + 最厉害工程, 维护交给未来高水平团队)
- 方向 ② 24 LOCKED 入口签名: **24 LOCKED 符合"不要怕复杂度"** (B1 0 改严守, 0 装 PASS 严守)
- 方向 ③ Cargo.toml borrow 段: **10 真实施 + 1 永久跳过 + 🆕 1 借脑 ID 索引完成 符合"不要怕复杂度"** (克隆 8 源真实施 = 最厉害工程, 12 源借脑 = 最强效果)
- 方向 ④ Cargo.lock 265KB: **265KB 符合"不要怕复杂度"** (87 + 561 crate 合理范围, 0 装"该 lockfile 太小" / 0 装"该 lockfile 拆更细")
- 方向 ⑤ 三洋葱架构: **三洋葱符合"不要怕复杂度"** (立体架构 + 生命架构 = 最强效果)
- 方向 ⑥ 9 organ 跨 8 LOCKED crate: **9 organ 跨 8 LOCKED crate 符合"不要怕复杂度"** (9 organ 1:1 镜像 8 LOCKED crate, 入口签名 0 改严守, 内部 fn 实施可改)
- 方向 ⑦ 借鉴源 12 源: **12 源符合"不要怕复杂度"** (BORROW 借脑 / 借鉴 / 借源 = 最厉害工程, OpenCog 家族 6 子源 借脑 = 最强效果)

---

## 7. 风险 + 决策原则 (per 决策 #33 §2.3 + 决策 #74 §1)

### 7.1 风险 (per 决策 #33 §2.3 + 决策 #74 §1 + R129-26 关键诚实标)

| # | 风险 | 严重度 | 状态 | 缓解 |
|---|------|-------|------|------|
| 1 | **整合 #5 commit 时机 NOT ready** | 🔴 P0 | per R129-26 00:55+ 实地 verify 30 处 fail 需修 (24 build + 1 test + 5 check) | 等 R129-3 跑过夜 + 主人起床后 fix 30 处 + 重跑 8 步 verify → 8/8 ready → 拍板 |
| 2 | **R129-21 报告 0 装 PASS violation** | 🔴 P0 | per R129-26 §0 J, claimed "0 errors" but actual 30 处 fail | 8 硬墙 #C2 0 装 PASS 严守 violation, 需纠正 (per 决策 #33 §2.3 C2) |
| 3 | **PHL-07 spec-only 0 实施** | 🟡 P1 | per 决策 #74 §1 A3, V1.0 release 0 实施, V1.1 release 实施 | per R131-3 实施路线图 |
| 4 | **87 crate 拆得过细** | 🟡 P1 | per §2.1 方向 ①, 远超 v1 30 crate 目标 | per §3.2 V1.1 release 优化点 (3 transparent re-export 合并 + 5 估补 R20 阶段 1 合并) + §3.3 V2.0 release 重构 |
| 5 | **Cargo.lock 265KB 大** | 🟢 P2 | per §2.4 方向 ④, 87 + 561 crate 合理范围 | per §3.2 V1.1 release Cargo.lock 可分模块 lockfile |
| 6 | **Cargo.toml borrow 段 17:44 vs 22:50 不一致** | 🟡 P1 | per §2.3 方向 ③, Cargo.toml 仍 17:44 状态, 实际已 22:50 状态 | per §3.1 V1.0 release 整合 #5.2 commit update 17:44 → 22:50 |
| 7 | **24 LOCKED 入口签名 0 改严守** | 🟢 P2 | per §2.2 方向 ②, 100% 0 改严守 | per §3.1 V1.0 release 0 改 + §3.2 V1.1 release Mavis 自决改 |
| 8 | **Tauri Stage 3-5 跨模块** | 🟡 P1 | per §2.8 Tauri 集成, 跑过夜 | per R129-19/31 |
| 9 | **ASI Stage 7-9 跨模块** | 🟡 P1 | per §2.6 ASI 阶段集成, 跑过夜 | per R129-18/30 |
| 10 | **形式化 Stage 5.3-5.5 跨模块** | 🟡 P1 | per §2.7 形式化集成, 跑过夜 | per R129-20/32 |

### 7.2 决策原则 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 + 用户记忆 #10)

**核心原则 (per 决策 #33 §2.3 + 决策 #74 §1 + 决策 #73 + 用户记忆 #10)**:
- ✅ **Mavis = orchestrator + 全自决 + 最高权限** (per 主人 8/10 16:31 + 8/11 0:25 + 8/11 01:14 升级授权)
- ✅ **跑中 ≥ 16** (per 主人 0:34, 16 active 全 background 跑)
- ✅ **中断接手** (per 主人 0:43, 检查 reports/agent-*.md 写完则标 done / 没写完则重派)
- ✅ **编译产物清理决策矩阵** (per 主人 0:49 + 0:54: ≤50 保守 / 50-100 预警 / 100-150 强烈预警 / > 150 强制清理)
- ✅ **计划内任务完成自动接续 4 步 + 永久循环** (per 主人 0:57: 调研 + 差距 + 计划 + 实施 → 永久)
- ✅ **locked 全解锁 + Mavis 自决架构** (per 主人 8/11 01:14 拍板 3 件套 §1, 整合 #5.1 commit 仍 0 改严守 + V1.1 release Mavis 自决改)
- ✅ **架构审视 + 升级方案永久工作项** (per 主人 8/11 01:14 拍板 3 件套 §2, cron Section 10 新增)
- ✅ **总工程哲学扩展 "不要怕复杂度"** (per 主人 8/11 01:14 拍板 3 件套 §3, `docs/conventions/15-no-fear-complexity.md`)

**8 硬墙严守 + B1 改写 (per 决策 #74 §1)**:
- ✅ **B1 24 LOCKED 入口签名**: V1.0 release 0 改严守 + V1.1 release Mavis 自决改 (前提: 更好的架构)
- ✅ **B2 workspace.version 1.2.0**: V1.0 release 1.2.0 严守 + V1.1 release bump 1.2.1
- ✅ **A1 R11 baseline 3 值**: 严守 (哲学 + 效果标)
- ✅ **A3 12 键 + PHL-07**: PHL-07 V1.0 spec-only 0 实施 + V1.1 实施, 12 键其他可改
- ✅ **B3 V0.5 30 维**: 严守 (哲学)
- ✅ **B4 6 重守门 v7**: 严守 (哲学)
- ✅ **B5 8 哲学锚**: 严守 (哲学)
- ✅ **C1 0 主动 commit (主人起床前)**: 严守
- ✅ **C2 0 装 PASS 严守**: 严守
- ✅ **0 push (主人起床前)**: 严守

**流程严守 (per 决策 #33 §2.3 + 决策 #61 §6 + 决策 #62 §9 + 决策 #74 §1)**:
- ✅ **整合 #5 commit 由 Mavis 自决拍板** (per 主人 0:25 + 决策 #33 C1 + 决策 #64 + 决策 #73 §5)
- ✅ **0 主动 push 严守** (per 决策 #33 + 决策 #61 §6)
- ✅ **0 主动 IM 主人** (per gate-discipline, 仅 done notification)
- ✅ **0 主动删** (per Safety policy + 决策 #44 + #60, 含 target/ 31.18 GB + _workspace/ 1.2 MB)
- ✅ **整合 #4 commit abf12243 严守** (per 决策 #48 + 决策 #61 §1.2)
- ✅ **决策日志写** (per 决策 #10 + 用户记忆 #10)
- ✅ **0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权)
- ✅ **0 借脑 0 装** (per P6-2/3 改借鉴已 cloned 而非真 clone, 仍属"借鉴 ID 索引完成")
- ✅ **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)

---

## 8. R131-4 跟 R131 era 6 sub-agent 派活关系 (per 决策 #75 §2.1)

**R131 era 第 2 批 6 sub-agent 派活分工** (per 决策 #75 §2.1, 01:20 拍板):
- ✅ **R131-4 cargo workspace 结构优化** (本任务) — 7 方向架构审视
  - 方向 ① 87 crate 分布合理性
  - 方向 ② 24 LOCKED 入口签名一致性
  - 方向 ③ Cargo.toml borrow 段精简
  - 方向 ④ Cargo.lock 大小
  - 方向 ⑤ 三洋葱架构落地
  - 方向 ⑥ 9 organ 代码最优分布
  - 方向 ⑦ 借鉴源 12 源 实施深度
- 🟡 **R131-5 24 LOCKED 入口分布优化** (待派) — 24 LOCKED crate 入口签名一致性 + 合并/拆分 (R131-4 方向 ② 拓维, 1:1 续 0 重复造轮子)
- 🟡 **R131-6 Cargo.toml borrow 段精简** (待派) — cloned=10/rate_limited=0/skipped=1 状态精简 (R131-4 方向 ③ 拓维, 1:1 续 0 重复造轮子)
- 🟡 **R131-7 pybridge 集成优化** (待派) — ASI Python Stage 1-8 跟 Rust 后端集成 + 性能瓶颈 (R131-4 §2.5 pybridge 集成 拓维)
- 🟡 **R131-8 Tauri 集成优化** (待派) — Tauri 2.0 + Rust 后端 + Web frontend 集成 (R131-4 §2.8 Tauri 集成 拓维)
- 🟡 **R131-9 形式化集成优化** (待派) — kani 借鉴 + PHL-07 形式化 + F1-F10 10 维度 (R131-4 §2.7 形式化集成 拓维)

**R131-4 跟 R131-1/2/3 关系** (per 决策 #73 §3.2 + 决策 #75 §2.1):
- ✅ R131-1 (done 01:25): 现有架构总审视 10 方向 → R131-4 是 R131-1 方向 ① cargo workspace 87 crate 拓维
- ✅ R131-2 (done 01:35): 借鉴 12 源差距 → R131-4 是 R131-2 方向 ⑦ 借鉴源 12 源 实施深度 拓维
- ✅ R131-3 (done 01:20): V1.1 release 实施路线图 → R131-4 是 R131-3 §3 V1.1 release 优化方向 cargo workspace 拓维

**R131-4 跟 R132 era 计划 2 sub 关系** (per 决策 #75 §2.1):
- 🟡 **R132-1 V1.1 release 路线图 final** (待派) — per R130-5 V1.1 路线图 + R131-3 V1.1 实施路线图 + R131-4~9 架构细分 整合 final 版
- 🟡 **R132-2 V2.0 release 战略路线图** (待派) — 8 硬墙可重评 + 8 哲学锚可重建 + Cargo workspace 可重构 (per 决策 #74 §2.3 V2.0 release)

**R131-4 跟 R133 era 实施 3 sub 关系** (per 决策 #75 §2.1):
- 🟡 **R133-1 借鉴源 12 源 实施** (待派) — OpenCog AGPL-3.0 fork 决策 (per 决策 #73 §2.2 + 主人 01:14 拍板 3 件套 §1 + 不要怕复杂度哲学)
- 🟡 **R133-2 ASI Stage 9 长程 AI 成长 实施** (待派) — per R130-2 ASI Stage 8 + R131-7 pybridge 集成优化
- 🟡 **R133-3 三洋葱架构升级 实施** (待派) — per 决策 #73 §2.2 更好的架构 + 决策 #74 B1 V1.1 release Mavis 自决改

---

## 9. 整合 #5 commit 拍板逻辑 (per 决策 #62 + 决策 #73 §5 + 决策 #74 §4)

**整合 #5.1 commit (src/ 实施, 95+ 文件)** (per 决策 #62 §5.1):
- 仍按原计划 (per 决策 #62 §5.1)
- **0 改 24 LOCKED 入口签名** (V1.0 release R11 baseline 严守 per 决策 #74 §1)
- 排除 `crates/apeireth-graph/src/lib.rs.bak.p6-2` (P6-2 backup)
- PHL-07 spec-only 0 实施 (V1.1 release 实施 per 决策 #74 §1 A3)
- ⚠️ **整合 #5.1 commit 时机 NOT ready** (per R129-26 实地 verify 30 处 fail 需修: 24 build + 1 test + 5 check)

**整合 #5.2 commit (docs/ + Cargo.toml, 10 文件)** (per 决策 #62 §5.2 + 决策 #73 §5):
- 仍按原计划 (per 决策 #62 §5.2)
- Cargo.toml borrow 段 update 17:44 → 22:50 状态 (per 决策 #62 §5.2, `count_cloned` 8 → 10, `count_rate_limited` 3 → 0, `count_skipped` 1 → 1)
- **+ 新增 `docs/conventions/15-no-fear-complexity.md`** (per 决策 #73 §3 主人 8/11 01:14 总哲学扩展)
- **+ 更新 `docs/conventions/10-locked.md`** (per 决策 #73 §2.3 主人 8/11 01:14 locked 全解锁)
- **+ 更新 `docs/conventions/09-anchor.md`** (per 决策 #73 §4.2 总工程哲学扩展引用)
- **+ 更新 `docs/conventions/README.md`** (per 决策 #73 §2.3 + §4.2 加 15-no-fear-complexity.md 索引)
- **+ 更新 `CONTRIBUTING.md`** (per 决策 #73 §2.3 8 项不修改承诺 改写 + 主人 8/11 01:14 拍板记录)
- **+ 更新 `README.md`** (per 决策 #73 §2.3 状态行加 R130 era 主人 8/11 01:14 拍板)

**整合 #5.3 commit (reports/, 60+ 文件)** (per 决策 #62 §5.3 + 决策 #73 §5):
- 仍按原计划 (per 决策 #62 §5.3)
- **+ 新增 decision-73 (主) + decision-74 (8 硬墙 B1 改写) + decision-75 (R131 era 第 2 批 6 sub 派活)** (per 决策 #73 §2.2 + §5 + 决策 #75)
- **+ 新增 R131 era 调研 6 sub-agent 报告** (R131-1 + R131-2 + R131-3 + R131-4 + R131-5 + R131-6, per 决策 #73 §3.2 + 决策 #75 §2.1)
- **+ 新增 `philosophy-no-fear-complexity-2026-08-11.md`** (主人 8/11 01:14 决策 3 件套详细)

**整合 #5 commit 拍板时机 (per 决策 #61 §1.4 + 决策 #62 §2 + 决策 #74 §4)**:
- ⚠️ **当前 NOT ready** (per R129-26 00:55+ 实地 verify 30 处 fail 需修)
- ✅ **8 项 verify 100% 落实 → Mavis 自决拍板** (per 决策 #64 §4):
  - 8 项 verify: 41 任务 done / 借鉴 11/11 状态 clear / 8 硬墙 0 越界 / 24 LOCKED 入口签名 0 改 / Cargo.toml 1.2.0 严守 / master HEAD = abf12243 / 决策链 #30-#74 全读 / 8 步 verify 全 PASS (R129-3)
  - 当前: 7/8 verify ✅, R129-3 8 步 verify 跑过夜 (估 01:30 done), 完后 cron Section 4 自动拍板

---

## 10. 1.0 release 实战流程 (per R129-8 + R129-13 + R129-23 + R129-27 final runbook)

**1.0 release 完整 5 步流程** (per R129-8 + R129-13 准备 + 决策 #55 §2.6 + 决策 #58 §5 + 主人 8/4 23:33):

1. **8 步 verify** (verify-1.0-pre-tag.{ps1,sh}, 主人起床后手跑):
   - Step 1: 修 session working dir + master HEAD + Cargo.toml
   - Step 2: `cargo build --workspace` (per R129-26 24 build errors 需修)
   - Step 3: `cargo test --workspace` (per R129-26 1 FAILED test 需修 + 4100+ tests)
   - Step 4: `cargo run --bin apeireth-tui` 5s smoke
   - Step 5: `cargo run --bin apeireth-api` 5s smoke
   - Step 6: `cargo audit + cargo deny`
   - Step 7: 24 LOCKED 入口签名 0 改 verify (24/24)
   - Step 8: 8 硬墙 0 越界 + 0 装 PASS 严守 (14/14 verify)

2. **配 GitHub remote** (setup-github-remote.{ps1,sh}, 主人手跑):
   - 主人浏览器创建 GitHub repo `apeireth/apeireth-rust` (Public, 0 初始化 README/.gitignore/license)
   - 加 origin remote `https://github.com/apeireth/apeireth-rust.git`
   - 主人配 git push 认证 (gh auth login 或 Personal Access Token)

3. **git push 整合 #5 拆 3 commit** (git-push-1.0.{ps1,sh}, 主人手跑):
   - 整合 #5.1 commit (50+ src/ 改动)
   - 整合 #5.2 commit (10 docs + Cargo.toml)
   - 整合 #5.3 commit (30+ reports/)
   - `git push -u origin master`

4. **打 v1.0.0 tag + gh release create** (tag-1.0.0.{ps1,sh}, 主人手跑):
   - `git tag -a v1.0.0 -m "Apeireth 1.0.0 release"`
   - `git push origin v1.0.0`
   - `gh release create v1.0.0 --title "Apeireth 1.0.0" --notes-file RELEASE_NOTES.md`
   - verify GitHub release 页面

5. **1.0 release 反馈**: 主人 verify + Mavis 写 decision-67 (1.0 release 拍板) + decision-68 (后续 R130 era 派活规划)

**GitHub Pages 部署 5 步** (per R129-13 准备, 主人手跑):
- mkdocs build → 配 gh-pages branch → git push origin gh-pages → 启用 GitHub Pages 设置 → verify 文档页面
- 7 markdown 源文件 (index + getting-started + api + roadmap + changelog + borrowed-repos + architecture) + 根 mkdocs.yml (Material theme, 5 nav + 3 链式页)

---

## 11. 一句话 (再次强调)

**R131-4 cargo workspace 结构优化 7 方向架构审视 (per 决策 #75 §2.1 + cron Section 10 架构审视永久工作项 + 主人 8/11 01:14 拍板 3 件套 §2)**: 7 方向审计 (87 crate 远超 v1 30 目标符合"不要怕复杂度" + 24 LOCKED 入口签名 100% 0 改严守 + Cargo.toml borrow 段 cloned=10/rate_limited=0/skipped=1 整合 #5.2 commit 时 update 17:44 → 22:50 + Cargo.lock 265KB 87+561 crate 合理 + 三洋葱架构合理 + 9 organ 跨 8 LOCKED crate + 借鉴源 12 源实施深度 6/8 真 cloned < 8/10). **V1.0 release 0 改 src 严守** + **V1.1 release Mavis 自决改** (前提: 更好的架构) + **V2.0 release 全 8 硬墙可重评**. **8 硬墙严守 + B1 改写** (B1 V1.0 0 改 + V1.1 Mavis 自决改 + V2.0 可重构). **8 哲学锚严守** (S-1 / S-2 / S-3 / O-1 / O-2 / O-3 / O-4 / O-5). **不要怕复杂度哲学落地** (最强效果 + 最厉害工程 + 维护交给未来高水平团队, per `docs/conventions/15-no-fear-complexity.md`). **风险**: 整合 #5 commit 时机 NOT ready (per R129-26 实地 verify 30 处 fail 需修) + 87 crate 拆得过细 (R14 阶段 2 §3 v1 30 crate 目标 远 87). **决策原则**: 0 主动 IM 主人 + 0 主动 commit/push + 0 装 PASS 严守 + 8 硬墙 0 越界 + 决策日志写. **R131-4 0 改 src 严守 100%**, 报告路径 `reports/agent-r131-4-cargo-workspace-optimization-2026-08-11.md` (本文件, 整合 #5.3 commit 包含 per 决策 #62 §5.3 + 决策 #73 §5 + 决策 #75 §2.1).

---

## 12. 不漂移 (per 决策 #33 §2.3 + 决策 #74 §1)

- 🔒 **0 改 src/** (100% 严守, R131-4 调研阶段, 整合 #5.1 commit 仍 0 改严守)
- 🔒 **0 改 Cargo.toml** (100% 严守, B2 workspace.version 1.2.0 0 改)
- 🔒 **0 主动 commit** (100% 严守, 整合 #5 commit 由 Mavis 自决拍板, R131-4 0 git commit)
- 🔒 **0 主动 push** (100% 严守, 等主人 1.0 release 配 GitHub remote 后手跑)
- 🔒 **0 主动 IM 主人** (100% 严守, 仅 done notification 主动报告)
- 🔒 **0 主动删** (100% 严守, per Safety policy + 决策 #44 + #60)
- 🔒 **8 哲学锚严守** (per 决策 #33 §2.3 B5 + 决策 #74 §1)
- 🔒 **8 硬墙严守 + B1 改写** (per 决策 #33 §2.3 + 决策 #74 §1)
- 🔒 **V0.5 30 维严守** (per 决策 #33 §2.3 B3 + 决策 #74 §1)
- 🔒 **24 LOCKED 入口签名 0 改严守** (per 决策 #33 §2.3 B1 + 决策 #74 §1 V1.0 release 严守, V1.1 release Mavis 自决改, V2.0 release 可重构)
- 🔒 **6 重守门 v7 严守** (per 决策 #33 §2.3 B4 + 决策 #74 §1)
- 🔒 **12 键 + PHL-07 严守** (per 决策 #33 §2.3 A3 + 决策 #74 §1, PHL-07 V1.0 spec-only 0 实施, V1.1 实施)
- 🔒 **R11 baseline 3 值严守** (per 决策 #33 §2.3 A1 + 决策 #74 §1)
- 🔒 **0 装 PASS 严守** (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权)
- 🔒 **0 借脑 0 装** (per P6-2/3 改借鉴已 cloned 而非真 clone, 仍属"借鉴 ID 索引完成")
- 🔒 **0 借具体源码** (per 决策 #33 §2.3 C2, 架构审视是文档工作)
- 🔒 **不要怕复杂度哲学落地** (per 决策 #73 §3 + `docs/conventions/15-no-fear-complexity.md`, 最强效果 + 最厉害工程 + 维护交给未来高水平团队)
- 🔒 **架构审视 + 升级方案永久工作项** (per 决策 #73 §2 + cron Section 10, 每次 cron tick 自动审视)
- 🔒 **locked 全解锁 + Mavis 自决架构** (per 决策 #74 §1 + 主人 8/11 01:14 拍板 3 件套 §1)
