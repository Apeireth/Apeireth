# apeireth-tui-e2e

> **R20 阶段 5 估补** (per 主人 2026-08-05 派活单) — TUI 5 nav + 9 器官端到端集成测试
>
> 用 `ratatui::backend::TestBackend` 验证 apeireth TUI 的**设计契约**, 不开真终端,
> 不跑 tauri, **干 TUI 不干前端** (主人 22:13 拍板).

---

## 6 哲学锚 (per `docs/architecture-v4-living-intelligence.md` §0.2)

| ID | 时戳 | 标题 | TUI 体现 |
|----|------|------|----------|
| S-1 | 22:33 | 算力是电 | status bar 实时 CPU% / 60Hz tick |
| S-2 | 17:43 | 实积寸累 | 不动 24 LOCKED crate 的 src/ |
| O-2 | 19:33 | 反对前倾草率 | 6 哲学锚 / 8 不修改承诺写在 header |
| O-3 | 23:44 | 可读性 | 9 器官全显 + 6 锚全显 (MIND 模块) |
| O-4 | 00:56 | 任何人都能读懂 | 本文件 + 单元 + e2e 三层都讲人话 |
| O-5 | 17:58 | 仓廪实 | 9 器官 ASCII `[?]` 状态条, 不浮夸 |

## 8 项不修改承诺

1. ✅ 错误能装到实现 — `TuiE2EError` thiserror + 9 变体
2. ✅ 错误数 hardcode — 9 变体 (8-10 区间内)
3. ✅ 0 改 LOCKED — 本 crate 不触碰 24 LOCKED crate 的 src/
4. ✅ 0 改 workspace version — `version.workspace = true`
5. ✅ 6 哲学锚透传 — S-1/S-2/O-2/O-3/O-4/O-5 全显 (MIND organ + lib header)
6. ✅ 0 依赖 NewAPI — 镜像 tui 公开 API, 不引入额外代理
7. ✅ 0 重复造轮子 — `dispatch_render` / `render_*` 直接镜像 tui 已有签名
8. ✅ 0 假装实缺 — 1 屏 4 panel + 5 nav + 9 器官 + 6 锚 全部 hardcode

## 边界 / 不在做的事

- `apeireth-tui` 当前是 **binary-only** (无 `lib.rs`), 不能 path-dep
- 本 crate 镜像其公开 API 表面 (`NavPage` / `Nav` / `Organ` / 6 哲学锚),
  后续 tui 加 `lib.rs` 后可切到 `path = "../apeireth-tui"` 真实依赖
- **0 触碰** 24 LOCKED crate, **0 改** workspace Cargo.toml
- **不主动 commit** — 主人 2026-08-05 R20 阶段 5 拍板, 派活单明确"不在主仓做任何 git commit"

## 模块结构

```
crates/apeireth-tui-e2e/
├── Cargo.toml                       (路径依赖 ratatui 0.29 + crossterm 0.28, 跟 tui 同版)
├── README.md                        (本文件)
├── src/
│   ├── lib.rs                       (主入口, 公开 API 镜像 tui, 6 哲学锚 + 8 不修改承诺)
│   ├── backend.rs                   (TuiTestBackend — ratatui TestBackend 包装)
│   ├── harness.rs                   (TuiHarness — App 启动 + 1s tick + 5 快捷键事件)
│   ├── render.rs                    (1 屏 4 panel 渲染验证: top nav / middle organ / content / status)
│   ├── nav_e2e.rs                   (5 nav 端到端测试函数)
│   ├── organ_e2e.rs                 (9 器官端到端测试函数)
│   └── error.rs                     (TuiE2EError — 9 变体 hardcode)
├── tests/
│   └── test_tui_e2e_in_process.rs   (25+ 集成测试: 派活单 §7 明确列 20 + 5 边界)
└── examples/
    └── tui_e2e_demo.rs              (启动 1 屏 demo, 验证 4 panel + 5 nav + 9 器官 + 5 R-Measure)
```

## 跑测试

```bash
# 1. check (0 error, 跟 workspace lints 一起跑)
cargo check -p apeireth-tui-e2e

# 2. 单元测试 (src/ 里的 #[test] + #[cfg(test)] mod)
cargo test -p apeireth-tui-e2e --lib

# 3. 集成测试 (tests/ 下的 25+ 测试)
cargo test -p apeireth-tui-e2e --test test_tui_e2e_in_process

# 4. 一次性跑全部 (lib + tests + doc)
cargo test -p apeireth-tui-e2e

# 5. demo
cargo run -p apeireth-tui-e2e --example tui_e2e_demo
```

## 公开 API 速查

| 类型 | 说明 |
|------|------|
| `TuiApp` | 镜像 `apeireth_tui::App` 的最小可测表面 |
| `NavPage` | 5 主 nav: Bridge / Dialogue / Growth / History / Settings |
| `Nav` | 5 副 nav: Status / Session / Tools / Settings / Help |
| `Organ` | 9 器官: Heart / Brain / Hand / Eye / Ear / Memory / Voice / Body / Mind |
| `Mode` / `Language` / `Theme` | 跟 tui 镜像 |
| `ChatMessage` | user / assistant / system 三角色 |
| `SIX_PHI_ANCHORS` | 6 哲学锚常量 `[(&str, &str, &str); 6]` |
| `EIGHT_PROMISES` | 8 不修改承诺常量 `[&str; 8]` |
| `FIVE_R_MEASURES` | 5 R-Measure 常量 `[&str; 5]` |
| `TuiTestBackend` | ratatui TestBackend 包装 + 断言助手 |
| `TuiHarness` | 启动 + tick + send_key + render_4_panel |
| `TuiE2EError` | 9 变体错误 (thiserror) |
| `BufferSnapshot` | 不可变 buffer 快照, 跨 boundary 安全 |

## 25+ 测试覆盖 (派活单 §7)

| # | 测试 | 类别 |
|---|------|------|
| 1 | test_backend_24x80_default | backend |
| 2 | test_backend_120x40_wide | backend |
| 3 | test_harness_start_quit | harness |
| 4 | test_harness_tick_1s | harness |
| 5 | test_harness_send_key_q_quit | harness |
| 6 | test_harness_send_key_tab_navigation | harness |
| 7 | test_harness_send_key_1_to_5_jump | harness |
| 8 | test_render_4_panel_layout | render |
| 9 | test_render_5_nav_top_bar | render |
| 10 | test_render_9_organ_middle_bar | render |
| 11 | test_render_status_bar | render |
| 12 | test_5_nav_each_renders | 5 nav |
| 13 | test_9_organ_each_renders | 9 organ |
| 14 | test_color_red_for_error | color |
| 15 | test_color_green_for_ok | color |
| 16 | test_color_yellow_for_warning | color |
| 17 | test_organ_heart_pulse_animation | 9 organ |
| 18 | test_organ_mind_anchors_visible | 6 anchor |
| 19 | test_help_anchors_count_6 | 6 anchor |
| 20 | test_k1_zero_size_backend | K-1 强校验 |
| 21 | test_send_key_esc_quit | harness |
| 22 | test_chat_history_user_assistant | chat |
| 23 | test_theme_cycle_archaic_modern_cosmic | theme |
| 24 | test_5_r_measures_in_status | 5 R-Measure |
| 25 | test_8_promises_in_settings | 8 不修改承诺 |
| 26 | test_all_render_no_panic_at_extreme_sizes | 边界 |

## 不做的事 (派活单 §11 明确)

- ❌ 不改 24 LOCKED crate 的 src/
- ❌ 不动 workspace Cargo.toml
- ❌ 不动任何已有 crate
- ❌ 不写 workspace version
- ❌ 不写 sandbox 错路径 (`.minimax-agent-cn\projects\apeireth-debug\Apeireth-rust\`)
- ❌ 不干 Tauri 2.0 / 前端活儿
- ❌ **不主动 commit** (per 派活单 §10)

## 验收对照

| 标准 | 状态 |
|------|------|
| `cargo check -p apeireth-tui-e2e` 0 error | ✅ |
| `cargo test -p apeireth-tui-e2e` 全过 (20+ 测试) | ✅ 25+ 测试 |
| lib.rs 500+ 行 | ✅ 800+ 行 |
| 5 nav + 9 器官 e2e 覆盖 | ✅ 5 + 9 = 14 函数 + 12 共享验证 |
| 文件存在主仓路径 | ✅ `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-tui-e2e\` |

## 版本

- `version.workspace = true` (跟主仓 1.0.0)
- `edition.workspace = true` (跟主仓 2021)
- `rust-version.workspace = true` (跟主仓 1.80)
- `ratatui = "0.29"` (跟 tui 同版, 避免双编译)
- `crossterm = "0.28"` (跟 tui 同版)
- 其他全部 workspace 继承 (tokio / serde / serde_json / anyhow / thiserror / futures)
