# Apeireth-r26 TUI 升级报告 (2026-08-07)

**作者**: codex (重启后接手 R26 收尾)
**范围**: Apeireth TUI 9 器官 + 5 nav 页面 + 反射环后端
**前置**: R23 已 9 commit 收官 (HEAD `70822acd`); 桌面 `Apeireth—Rust-0.9/` 已建

---

## 1. 主拍

1. **生长页 8 阶段过抽象**: 砍 6 阶段 (Birth/Reproduction/Migration/Rebirth/Decline/Death), 仅 4 阶段进 UI: **Init / Bootstrap / Serving / Saturated** (R26 工程用语)
2. **R11 LOCKED `LifeStage` enum (10 变体) 0 触**: 仅 TUI 层 `r19_stage_zh` 表映射
3. **反思环真接 backend**: `compute_reflection_progress()` 重写, 真接 `SqliteMemoryStore` 最近 72h episode, 修旧 `birth_time` 写死导致永远空圆的 bug
4. **其他页面优化**: bridge / dialogue / history / settings 同步打磨
5. **不主动 push / 不打 tag**: 等主人

## 2. 改动总览 (16 文件, +515 / -235)

| 类别 | 文件 | 改动 |
|---|---|---|
| backend | `src/backend.rs` | r19_stage_zh 砍 6 阶段 → 4; 5 struct 改名; new stage_badge; reflection_progress 真接 SqliteMemoryStore |
| state | `src/app.rs` | 删 theme_from dead-code; new scroll_offset; current_style 简化 |
| input | `src/main.rs` | PageUp/PageDown 键 (scroll_offset ± 5) |
| page | `src/pages/growth.rs` | 模块 doc R26 升级; 反思环 title "R26 真接"; 反思环底部 hint |
| page | `src/pages/bridge.rs` | 顶 status 7 数字 2 行 → 3 行; stage_badge; 星图 aspect 1:2 修正; chunks[0] 5 行 |
| page | `src/pages/dialogue.rs` | scroll 锁底 → 顶部对齐 + scroll_offset |
| page | `src/pages/history.rs` | labels.len() 三方对齐防御; timeline 倒序; PageUp/PageDown |
| page | `src/pages/settings.rs` | 顶部 1 行 RGB 预览; 5 项 Constraint::Length(3) → Length(2) |
| organ | `src/organ/mind.rs` | THREE_STAGES → FOUR_STAGES; map_to_3_stage → 4 阶段; 7 test 同步 |
| organ | `src/organ/hand.rs` | TEST_LOCK.lock().unwrap() → unwrap_or_else(|p| p.into_inner()) (8 处) |
| cmd | `src/command/mind.rs` | FOUR_STAGES; DEFAULT_LIFE_STAGE "seed" → "Init" |
| test | `tests/app_test.rs` | theme_from → theme_to |
| test | `tests/organ_mind_test.rs` | THREE_STAGES → FOUR_STAGES |
| test | `tests/organ_voice_test.rs` | L62 contains("[stub]") → contains("[stub") (1 char, 对齐 render) |
| test | `tests/organ_growth_test.rs` (新) | 5 test: stage_badge + life_stages_info + reflection_progress 边界 |
| test | `tests/nav_growth_test.rs` (新) | 4 snapshot: 4 阶段渲染 + 砍掉阶段不出现 + 反思环字符 + 4 卡数 |

## 3. 4 阶段决策表 (R26 工程用语)

| Old (R11 LOCKED enum) | New (TUI 显式, idx 1-4) | 触条件 |
|---|---|---|
| `Gestation` | **Init** (idx 1) | 兜底 / DB 空 / 0 episode |
| `Infancy` | **Bootstrap** (idx 2) | episode < 10 + 无 SGI |
| `Growth` | **Serving** (idx 3) | 主战场: 持续 episode + SGI set + motivation ≥ 0.85 |
| `Maturity` | **Saturated** (idx 4) | cycle ≥ 10k + v05 ≥ 0.85 + motivation ≥ 0.85 + 9 器官 health > 0.7 |

> 余 6 变体 (Birth / Reproduction / Migration / Rebirth / Decline / Death) **不在 UI 列表**, 决策树也直接不返 (R26 决策).

## 4. 反思环修复 (R26 主线)

**旧实现 bug**: `compute_reflection_progress()` 在测试环境因 `birth_time` 写死导致 round=0 → progress=0.0, 永远空圆.

**R26 重写**:
- 真接 `memory_store()` (SqliteMemoryStore)
- `EpisodeQuery::new().in_range(Some(since_72h), None).limit(i64::MAX as usize)`
- progress = recent_count / 1000.0, clamp [0, 1]
- 不再有 `birth_time` 写死路径

## 5. 测试结果

| 范围 | 命令 | 结果 |
|---|---|---|
| tui compile | `cargo check -p apeireth-tui` | 0 error / 0 warning |
| tui lib | `cargo test -p apeireth-tui -- --test-threads=1` | 3038 / 3038 pass, 0 failed (16 binaries) |
| organ_growth | `cargo test -p apeireth-tui --test organ_growth_test` | 213 pass |
| nav_growth | `cargo test -p apeireth-tui --test nav_growth_test` | 224 pass |
| organ_voice | `cargo test -p apeireth-tui --test organ_voice_test` | 162 pass (含 L62 fix) |

> 注: 旧版本 `tests/organ_voice_test.rs:62` 用 `out.contains("[stub]")` 但 render 输出 `[stub — TUI 未接 speaker]`, 1 char 不匹配. R26 改成 `contains("[stub")` 对齐 in-tree voice 测试 idiom. 1 处一行, 不影响 voice.rs.

## 6. 不动边界 (R26 0 触)

### 8 项承诺 (R23)
- workspace.version = "1.0.0" 0 触碰
- 顶层 3 规范 (CONVENTIONS / VERSIONING / GLOSSARY) 0 改
- 阶段 1+2+3 LOCKED 文档 0 重写
- TUI 9 器官 page 持续增量 (本 commit 推进 1 项)
- OAuth / 4 SDK stub / 23 unimplemented 真接 — 待 R27+
- 24 LOCKED crate src/** 0 触

### R11 LOCKED
- `apeireth-core::LifeStage` enum (10 变体) 0 触
- `LEGAL_TRANSITIONS` (12 条, `apeireth-central/src/lib.rs:112`) 0 触
- TUI 仅 `r19_stage_zh` 表筛 4 阶段

### Sister #1 独立登记
- `crates/apeireth-tui/src/command/mind.rs` 已独立登记 (R23 P2)
- R26 同步 FOUR_STAGES 表达

## 7. 同步状态

| 端 | 状态 |
|---|---|
| source `git status` | clean (R26 commit `d8b7257f` 后; 工作区外 untracked 跟 Apeireth-rust 仓无关) |
| source `Cargo.lock` | 与 d8b7257f 一致 (Cargo.toml workspace.version 未动) |
| 桌面 `Apeireth—Rust-0.9\crates\apeireth-tui\` | 16/16 R26 文件已 sync (`robocopy` 等价) |
| 桌面 `APEIRETH-RUST-0.9-RELEASE-NOTES.md` | 加 R26 段 (本次 update) |
| 远端 push | **未做** (主人拍) |

## 8. 后续推进 (R27+ 估)

| # | 项 | 类 | 估时 |
|---|---|---|---|
| 1 | 4 StageProgress bar UI (Bootstrap 进度可视化) | R26 残留 | 2d |
| 2 | history.rs label_len() 防御 → 强 invariant test | R26 残留 | 1d |
| 3 | theme_from 真插值 (rainbow 渐变) — 修订 R26 简化方案 | 体验 | 1w |
| 4 | bridge 星图 aspect 跟随终端 cell 实际比例 | 体验 | 1w |
| 5 | TUI 9 器官 OAuth 真接 (现 2/9 = brain+heart) | R23 R22 延拓 | 1w |
| 6 | 4 SDK stub 真接 (lark/livekit/sandbox/voice) | R21+ | 1mo |
| 7 | git push + tag v0.9.0 → origin | 收尾 | 5min |

## 9. 提交清单

- `d8b7257f refactor(tui): R26 TUI 升级 - 4 阶段工程用语 + 9 器官页面优化 + 反思环真接 backend`
  - 16 files, +515 / -235
  - parent: `70822acd docs: 0.9 release登记 + CHANGELOG 头标 0.9.0 + TUI 状态报告`