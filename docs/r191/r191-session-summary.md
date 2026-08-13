# R191 session summary — R177-R190 GitHub per-module 调研 + 终极目标实施路线

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R191 (summary + R192+ 实施路线)
> **日期**: 2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕 + 干到底

---

## 0. R177-R190 14 commit 总览 (R176+1 起算)

| Commit | R | 主题 | 文档大小 |
|---|---|---|---|
| 39b8095b | R177 | voice 模块调研 (whisper-rs / openWakeWord / ECAPA-TDNN) | 5.2KB |
| 91993f97 | R178 | sovereignty 调研 (microsandbox / Firecracker / Kani) | 6.0KB |
| 7c2373e9 | R179 | tool-browser 调研 (chromiumoxide / browser-use) | 6.4KB |
| e091f1a | R180 | council 调研 (LangGraph / AutoGen / swarms-rs) | 7.6KB |
| 9cf51978 | R181 | tool-codesearch 调研 (ast-grep / tree-sitter) | 5.5KB |
| 71941d1 | R182 | relation 调研 (SurrealDB / Kùzu / Cozo) | 6.9KB |
| 88966a17 | R183 | tui 调研 (ratatui / Bubble Tea / Elm 架构) | 6.4KB |
| 96b05285 | R184 | pipeline 调研 (DSPy / failsafe-rs / GPTCache) | 7.3KB |
| da438b3 | R185 | VCP 官网分批调研 (6 子系统 + 5 类插件) | 8.5KB |
| cd1e46ae | R186 | memory 调研 (Letta / Mem0 / Graphiti) | 7.9KB |
| 47039b6f | R187 | cognition layer 调研 (LATS / Perceiver / ImageBind) | 6.4KB |
| 4b522d0 | R188 | core/central/bus 调研 (NATS / Bevy ECS / Skills) | 5.4KB |
| 9dcc714 | R189 | tool 栈调研 (MCP / Gorilla / microsandbox) | 5.6KB |
| 5217cc7c | R190 | evolution/life-force 调研 (Voyager / Darwin Gödel) | 6.4KB |

**净影响**:
- 14 份调研文档 (R177-R190)
- 1 份 VCP 官网深度分析 (R185)
- 总新增文档: ~95KB Markdown
- 14 commits (1 commit / 调研, 清洁可追溯)
- 0 行代码改动 (调研阶段)
- cargo check --workspace: 0 errors (持续保持)

---

## 1. 调研覆盖矩阵 (R177-R190 14 模块)

| R | 模块 | 现状大小 | 推荐主升级 | 推荐 ROI |
|---|---|---|---|---|
| 177 | voice | 3 文件 deferred | whisper-rs (STT) / openWakeWord (wake) | 长期 |
| 178 | sovereignty | 32 文件 274KB | **microsandbox** (R192 真接) | **极高** |
| 179 | tool-browser | 255KB | chromiumoxide (R193 集成) | 高 |
| 180 | council | 31 文件 274KB | LangGraph Checkpoint/Time Travel | 中-高 |
| 181 | tool-codesearch | Tier 1.4 | **ast-grep CLI 包装** (R194) | **高** |
| 182 | relation | 4 文件 65KB | SurrealDB 真接 (R195) | 极高 |
| 183 | tui | 53 文件 255KB | Elm 架构 + tui-* (R220+ 推迟) | 长期 |
| 184 | pipeline | 12 文件 226KB + 10 文件 76KB | failsafe-rs + DSPy signatures | 中-高 |
| 185 | (VCP 官网) | 1.8MB JS | 三套通知 / OneRing / 5 类插件 | 高 |
| 186 | memory | 35 文件 ~300KB | LanceDB 评估 / Self-edit (Letta) | 中-高 |
| 187 | cognition layer | 11 文件 107KB | LATS 树搜索 / Plutchik 8 情绪 | 中 |
| 188 | core/central/bus | 25 文件 389KB | NATS subject / typed-builder | 中 |
| 189 | tool 栈 | 12 文件 396KB | MCP 协议对齐 / fuzzy upgrade | 中 |
| 190 | evolution/life-force | 12 文件 310KB | Voyager skill API / CRITIC 反思 | 中-高 |

**未单独调研但已覆盖**:
- protocol-bridge (R146 真接, 文档 v4 涵盖)
- api (R155 runtime_bridge 桥建)
- formal (Kani proofs, R188 提)
- constraint / onion (双洋葱, 哲学文档涵盖)
- sovereign (sovereignty 涵盖)

**主动不调研** (主人明确指示):
- 占卜 / 酒馆 / 论坛 (冻结)

---

## 2. 终极目标实施路线 (R192+ 排序)

### 2.1 🥇 极高 ROI (优先做)

| 优先级 | R | 主题 | 工作量 | 来源调研 |
|---|---|---|---|---|
| 1 | R192 | **microsandbox 真接** (替代 sandbox stub) | 3 days | R178 |
| 2 | R193 | **ast-grep CLI 包装** 进 tool-codesearch | 1 day | R181 |
| 3 | R194 | **chromiumoxide 集成** 进 tool-browser | 1-2 days | R179 |
| 4 | R195 | **SurrealDB embedded 真接** 进 relation | 5-7 days | R182 + R171 |

### 2.2 🥈 高 ROI

| 优先级 | R | 主题 | 工作量 | 来源调研 |
|---|---|---|---|---|
| 5 | R196 | **LanceDB 评估** 进 memory L2 向量 | 2-3 days | R186 |
| 6 | R197 | **Kani 3 关键 proof 补完** (Self-Disable / L0 HA / verdict cache) | 2-3 days | R178 + R188 |
| 7 | R198 | **failsafe-rs 集成** 进 pipeline reliability | 1-2 days | R184 |
| 8 | R199 | **bus 三套通知物理隔离** (AI/UI/Both) | 1-2 days | R185 + R188 |

### 2.3 🥉 中 ROI

| 优先级 | R | 主题 | 工作量 | 来源调研 |
|---|---|---|---|---|
| 9 | R200 | **MCP 协议完整对齐** tool-runtime | 2-3 days | R189 |
| 10 | R201 | **nucleo / fuzzy-matcher 升级** tool-registry | 1 day | R189 |
| 11 | R202 | **LATS 树搜索** 进 cognition decision | 2-3 days | R187 |
| 12 | R203 | **Plutchik 8 情绪** 进 consciousness emotion | 1 day | R187 |
| 13 | R204 | **typed-builder 集成** 进 core onion | 1 day | R188 |
| 14 | R205 | **LangGraph Checkpoint/Time Travel** 进 council | 3-5 days | R180 |
| 15 | R206 | **OneRing 风格时间线** 进 memory | 3-5 days | R185 |
| 16 | R207 | **Self-Edit 机制** (Letta 风格) 进 memory | 2-3 days | R186 |
| 17 | R208 | **Skill Voyager API 强化** 进 evolution | 2-3 days | R190 |
| 18 | R209 | **5 类插件分类** 进 tool-registry | 1-2 days | R185 |

### 2.4 🔵 长期 (R220+)

| R | 主题 |
|---|---|
| R220 | TUI 接入真后端 (R183 推迟) |
| R230 | Elm 架构重构 TUI |
| R240 | 协议桥 (OpenAI/Anthropic/Gemini) 全兼容 |
| R250 | VCP 全部插件最终验收 |
| R260 | 终极目标验收 |

### 2.5 最后阶段

- STT / 唤醒词 / 声纹 / 生图 / 图处理 真接 (主人 R173 指示, 放最后)

---

## 3. 0 触碰声明 (R177-R190 全程)

- 3 不可变脊柱: 0 触碰 (Self-Disable 判定 / L0 HA 物理多签 / 13 键 verdict cache 语义)
- workspace.version 1.2.0: 0 改
- 24 LOCKED crate 入口签名: 0 改 (R148 已形式撤销, 实际可改)
- V0.5 30 维 / V1136 / 9键 原始: 0 改
- STUB_MODE compile-time hardcode: 0 改
- 5 战区骨架: 0 改

---

## 4. 当前实施优先级 (今晚要做)

按 ROI 排序 + 依赖关系, 推荐接下来 3 个 R:

### R192 microsandbox 真接
- **理由**: R178 强推荐, 极高 ROI, 替代当前 stub sandbox
- **步骤**:
  1. 评估 microsandbox Rust SDK / 决定用 CLI 还是 lib
  2. 创建 apeireth-sovereignty/src/tool_isolation/microsandbox.rs
  3. 实现 ToolIsolation trait
  4. 加进 sandbox 选择策略 (per tool policy)
  5. 8 集成测试 + 1 端到端 demo
- **风险**: Linux-only (KVM), Windows 上需要 hyper-V fallback
- **0 触碰**: 用 trait 抽象, 不动 3 脊柱

### R193 ast-grep CLI 包装
- **理由**: R181 短期路径 (CLI subprocess, 0 编译增加), 极高 ROI
- **步骤**:
  1. 验证 ast-grep 在 Windows 路径
  2. 创建 apeireth-tool-codesearch/src/ast_grep.rs
  3. CLI subprocess 包装 (clap-based)
  4. JSON 输出标准化
  5. 5 集成测试
- **风险**: subprocess spawn 性能, 但毫秒级可接受
- **0 触碰**: 新增子模块, 不动现有 API

### R194 chromiumoxide 集成
- **理由**: R179 推荐, 高 ROI, 升级 tool-browser 底层
- **步骤**:
  1. 评估 chromiumoxide 编译产物
  2. 创建 apeireth-tool-browser/src/cdp.rs
  3. 替换当前手撸 HTTP
  4. 加 screenshot / element selector API
  5. 5 集成测试
- **风险**: 编译时间长 (~5min), 但仅编译一次
- **0 触碰**: 现有 tool-browser 公开 API 不变

---

## 5. R192+ 实施前置条件

每个实施开始前, 我会:
1. 写 docs/r19X/r19X-XXX-design.md 设计稿
2. 列出 0 触碰声明
3. 列出测试计划
4. 列出 0 触碰风险
5. 然后才动手写代码

主人的 "全部自主决定" 授权让我可以跳过 approval, 直接开干.

---

## 6. 0 触碰声明 (R191 summary)

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 24 LOCKED: 已 R148 撤销
- 本档 0 行代码改动 (纯文档)
- 14 调研文档 0 引入新依赖

---

## 7. R192+ 待办 (摘要)

1. R192 microsandbox 真接 (3 days, 极高 ROI)
2. R193 ast-grep CLI 包装 (1 day, 极高 ROI)
3. R194 chromiumoxide 集成 (1-2 days, 高 ROI)
4. R195 SurrealDB embedded (5-7 days, 极高 ROI)
5. R196 LanceDB 评估 (2-3 days, 中-高)
6. R197 Kani 3 proof 补完 (2-3 days, 高)
7. R198 failsafe-rs 集成 (1-2 days, 中-高)
8. R199 bus 三套通知 (1-2 days, 高)

总计 ~25 工作日 / 5 周 (5 commits / 周)
按 1 commit / 工作日 = 25 commits 至 R220 (TUI 接入)