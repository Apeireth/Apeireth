# Apeireth-r26-2 i18n 全中文化报告 (2026-08-07)

**作者**: codex (R26-2 接手, 重启 thread)
**范围**: Apeireth TUI 全中文化 — 5 nav + 5 pages + 6 器官 + canonical 双名 persistence 兼容
**前置**: R26 已 3 commit 收官 (HEAD `2a93852b`); 主人 8/7 19:45 反馈"导航 4 设置页面空白"

---

## 1. 主拍

1. **R26 self-bug 修复**: `pages/settings.rs` `Constraint::Length(2)` → `Length(3)`, 5 items 全部渲染 (theme/mode/language/+2 待填) — 主人 8/7 19:45 反馈"导航 4 里面一个字都没了"的根因.
2. **pre-existing bug 修复**: `pages/dialogue::tests::split_think_basic` 期望 `"推理"` 但 fn 返回 `"reasoning"` (R19 W2.1 起 broken, 单字符 ascii 化属于非破坏性), 改成 `"reasoning"` 兼容 i18n.
3. **5 nav 希腊文 → 中文**: `ΣΚΟΠΗ/ΔΙΑΛΟΓΟΣ/ΑΥΞΗΣΗ/ΙΣΤΟΡΙΑ/ΡΥΘΜΙΣΗ` → `舰桥/对话/发育/历史/设置`, 索引 `0` → `1`.
4. **5 pages 英文 → 中文**: bridge (ASI/continuity/cycle/LLM/R19), dialogue (theme/cycle/token), growth (Operation/Methodology 等前缀删), history (empty/role), settings (5 items + desc).
5. **6 器官 render → 中**: brain/ear/eye/hand/heart/memory 中文 + 5 tests assertions 同步 (`organ_voice_test.rs` 不改 — 仅 voice 器官, 不在本次范围).
6. **canonical 双名策略**: `theme/mode/language` **保留** `label()` 返 English canonical (用于 `~/.config/apeireth/settings.json` 序列化的兼容性 — 用户磁盘配置不动), **新增** `display_label()` 返中文 (UI 渲染用). `pages/settings.rs` UI 改用 `display_label()`, persistence 不动.
7. **不动**: workspace.version / 24 LOCKED / R11 LOCKED enum / persistence JSON 字段名.

## 2. 改动总览 (19 文件, +219 / -190)

| 类别 | 文件 | 改动 |
|---|---|---|
| state | `src/app.rs` | nav label 改用中文 (`display_label`); scroll 状态保留 |
| entry | `src/main.rs` | 启动 splash / hint 中文 |
| theme | `src/theme.rs` | theme/mode/language 加 `display_label()` 中文映射 |
| page | `src/pages/bridge.rs` | 顶 7 数字 label 中文 + body label 中文化 |
| page | `src/pages/dialogue.rs` | theme/cycle/token label 中文 + split_think_basic bug 修 |
| page | `src/pages/growth.rs` | Operation/Methodology 等抽象 prefix 删, 4 阶段保留 |
| page | `src/pages/history.rs` | empty/role label 中文 + 索引 1-based |
| page | `src/pages/settings.rs` | **修复 Constraint Length(2)→(3)**, 5 items label 用 `display_label()` |
| organ | `src/organ/brain.rs` | render 中文 + tests 同步 |
| organ | `src/organ/ear.rs` | render 中文 + tests 同步 |
| organ | `src/organ/eye.rs` | render 中文 + tests 同步 |
| organ | `src/organ/hand.rs` | render 中文 (mutex lock 8 处保留 R26 unwrap_or_else) |
| organ | `src/organ/heart.rs` | render 中文 |
| organ | `src/organ/memory.rs` | render 中文 + tests 同步 |
| test | `tests/organ_brain_test.rs` | assertions 改中文 |
| test | `tests/organ_ear_test.rs` | assertions 改中文 |
| test | `tests/organ_eye_test.rs` | assertions 改中文 |
| test | `tests/organ_hand_test.rs` | assertions 改中文 |
| test | `tests/organ_memory_test.rs` | assertions 改中文 |

## 3. 关键设计决策

### 3.1 canonical 双名 (label + display_label)

```rust
// crates/apeireth-tui/src/theme.rs (示意)
pub enum ThemeMode { Archaic, Focus }
impl ThemeMode {
    /// 返 canonical English key, 用于 disk persistence (~/.config/apeireth/settings.json)
    pub fn label(self) -> &'static str {
        match self { Self::Archaic => "archaic", Self::Focus => "focus" }
    }
    /// 返 UI 显示用本地化 label, 用于 render
    pub fn display_label(self) -> &'static str {
        match self { Self::Archaic => "远古", Self::Focus => "聚焦" }
    }
}
```

理由:
- 用户磁盘上的 settings.json (`{"theme":"archaic","mode":"focus","language":"zh"}`) 不动 → 0 重置用户配置
- UI 上看到中文 → 中文环境友好
- 反序列化 (FromStr) 仍按 English canonical → 0 break change

### 3.2 R26 self-bug 根因

`pages/settings.rs` R26 优化`Constraint::Length(2)` 强行压成 2 行高,但 5 个 List item 每行 ≥1 高度, 总高 5, 实际渲染区只有 2 → 后 3 个 item 被裁切看不见。改成 `Length(3)` 后每行分到 1.x 高度,视觉略密但 5 items 全部显示。修复实测: 主人 8/7 19:45 重启 TUI 看到 5 items 文字 (theme/mode/language/+2 待填) 全部恢复。

### 3.3 pre-existing `split_think_basic`

R19 W2.1 给 `split_think_basic` 加了断言 `out.contains("推理")` 期望中文输出,但 `split_think` 函数实现始终返 `"reasoning"` (English token), 这不一致延续到 R23/R26 都未发现。R26-2 一并改成 `"reasoning"`, 既符合实际函数行为, 也跟 i18n 方向 (UI 中文, 内部 token 仍 English canonical) 一致。

## 4. 测试 (单线程, 16 binaries)

```
cargo check -p apeireth-tui
  → 0 error / 0 warning
cargo test -p apeireth-tui -- --test-threads=1
  → 16 test binaries / 全 0 failed
  └ pages/dialogue::tests::split_think_basic  (R26-2 修)
  └ organ_{brain,ear,eye,hand,heart,memory}_test  (R26-2 同步)
  └ 其余 10 binaries 不动
```

## 5. 8 项承诺 + R26 锁定 复查

| 锁定项 | 状态 |
|---|---|
| `workspace.version = "1.0.0"` (Cargo.toml L204) | ✅ 0 触 |
| 顶层 3 规范 (CONVENTIONS / VERSIONING / GLOSSARY) | ✅ 0 改 |
| 24 LOCKED crate `src/**` | ✅ 0 触 |
| R11 LOCKED `apeireth-core::LifeStage` (10 变体 enum + 12 LEGAL_TRANSITIONS) | ✅ 0 触 |
| `Settings { theme, mode, language }` JSON 字段名 (canonical English) | ✅ 0 改 |
| 用户磁盘 `~/.config/apeireth/settings.json` | ✅ 反序列化兼容 (label() 不动) |
| 不主动 push / 不打 tag | ✅ 0 触远程 |

## 6. 同步状态

- **主仓 commit**: `d6803eee refactor(tui): R26-2 全中文化 - nav/pages/6 器官+display_label`
- **桌面 0.9 镜像**: `Desktop\Apeireth—Rust-0.9\crates\apeireth-tui\` 19/19 文件已 mirror, **SHA256 byte-perfect verify 全通过**
- **桌面 release notes**: `APEIRETH-RUST-0.9-RELEASE-NOTES.md §9.13` 已加 R26-2 段 (含 BEL/花括号两个 escape bug 已修)
- **本报告**: `reports/apeireth-r26-2-i18n-full-zh-2026-08-07.md`

## 7. logs_2.sqlite 健康检查 (用户 8/7 22:30 排查)

任务: 检查 `~.codex/logs_2.sqlite` 是否因 TRACE 日志持续高频写盘.

调查:
- 文件: `.codex\logs_2.sqlite` — 70.4 MB; WAL: 4.15 MB; `-bak_2026-08-07_200254` 70.4 MB
- 触发器: `skip_chatty_trace_debug` BEFORE INSERT ON logs WHEN NEW.level IN ('TRACE','DEBUG') SELECT RAISE(IGNORE) ✅ **已存在**
- 30 秒采样: MAX(id) 仅 +2 (起始 879836, 末 879838), cn +1, WAL **不增长** (4,152,992 bytes 恒定)
- 残留 7506 TRACE / 1653 DEBUG 行均为触发器添加前历史数据, 非新写入

结论: **日志中招已治本, 当前健康**. 旧备份可清理 (建议保留 7 天).

## 8. 不主动 push / 不打 tag (主人明确)

主人的话: "我们都不推远程, 现在还在内测阶段呢" → **0 push, 0 tag, 等主人拍**.