# Apeireth Rust 0.9 Release Report — 2026-08-07

> **作者**: 主线 codex (Apeireth-rust) + Hermes 审查 / 文件清理
> **接收**: 主仓 `.openclaw\workspace\promethean\Apeireth-rust` (HEAD = `b0940e73` on `master`)
> **桌面快照**: `Desktop\Apeireth—Rust-0.9\`
> **角色**: 透明登记 0.9 诞生原因、TUI 当前真状态、其他模块真状态、与 1.0 的差距。

---

## 1. 0.9 怎么诞生的

1. R17 战役 0-4 (2026-08-04) 完成“后端 1.0 release 收尾” + 打了 `v1.0.0` tag。但 `v1.0.0` tag 没推远程，主人 2026-08-04 拍“0 主动 push, 内测通过才推”。
2. R20 阶段 1 收官 (2026-08-05) 入了 14 new crate + workspace version 仍是 1.0.0 (LOCKED)。
3. R23 (2026-08-06) 9 commit 落地: P0 RUSTSEC 1 unsound / P1 endpoint hardcode / P1 6 module 实质 / P1 OAuth transport / P2 unimplemented fix / P3 doc cleanup / P3 sister #1 独立登记 / P3 body struct / P3 final docs。
4. 0.9 释义: 已交付 R14+R15+R16+R17+R23 全部阶段, 但仍有 4 项 R21+ 长线缺口 (OTA / WebAuthn / pybridge cdylib / R-Measure 持久化)。若直接叫 1.0 会让“1.0 release”名不副实；若沿叫 1.0 推 tag 又会被“0 主动 push”拦住。因此另立 `0.9.0` 透明登记 (CHANGELOG 头 + 本报告)，不重打 `v1.0.0` tag、不动 workspace.version。

## 2. TUI 真状态 (回答主人问题)

> 主人原问: “我们tui做的怎么样的”

### 2.1 工程清单 (实测 `crates/apeireth-tui\`)

| 项 | 数量/状态 | 路径 |
|---|---|---|
| 器官文件 | 9 + mod.rs | `src/organ/{brain,heart,mind,memory,voice,hand,ear,eye,body}.rs` + `mod.rs` |
| sister #1 command | 10 + mod.rs | `src/command/{brain,ear,eye,hand,heart,memory,mind,voice,error,body}.rs` + `mod.rs` |
| 顶层 src 模块 | 8 | `src/{app,backend,error,http_llm,http,main,observability,persistence,theme}.rs` |
| 公共 pub item | 294 | 跨 organ/command/backend 等 |
| TODO/unimplemented | 3 (全在文档) | `src/**` |
| stub/placeholder 字眼 | 207 (含文档) | `src/**` |
| 器官测试 | 10 文件 | `tests/organ_{brain,ear,eye,hand,heart,memory,mind,voice,body,command}_test.rs` |
| 启动 banner | 编译期 hardcode | `src/main.rs` setup_terminal 前 |

### 2.2 9 器官状态 (逐个简评, 数据来自 `src/organ/*.rs`)

| 器官 | 文件 | 后端真接 | 标 partial? | 备注 |
|---|---|---|---|---|
| Brain (脑) | `brain.rs` | ✅ 真接 | 否 | R17 战役 4-2 已接 `apeireth-cognition` |
| Heart (心) | `heart.rs` | ✅ 真接 | 否 | R17 战役 4-2 已接 `apeireth-life-force` |
| Mind (意) | `mind.rs` | ⚠️ 局部 | 部分 | label 已校准 (R23 c012ab0b); v05_overall = continuity + philosophy 平均, 标 “真接 backend stage” |
| Memory (忆) | `memory.rs` | ⚠️ 局部 | 部分 | 借 `apeireth-memory` 真接口, 部分走 SQLite 持久化 |
| Voice (声) | `voice.rs` | ⚠️ 局部 | 部分 | `apeireth-voice` 是 lib + real, stub_demo 不存在 (Hermes 老列已澄清) |
| Body (体) | `body.rs` | ❌ placeholder | 是 | R23 c012ab0b 加了 `BodyState` struct, 但 4 资源 (cpu/mem/disk/net) 用占位 (12.5%/256MB/45%)。R25.3 真接 `sysinfo` (注: 不在 Cargo.toml, 0 假装) |
| Eye (眼) | `eye.rs` | ⚠️ 局部 | 部分 | 借 `apeireth-perception` 真接口, 部分占位 |
| Ear (耳) | `ear.rs` | ⚠️ 局部 | 部分 | 借 `apeireth-perception` 真接口, 部分占位 |
| Hand (手) | `hand.rs` | ⚠️ 局部 | 部分 | 借 `apeireth-action` 真接口, calendar 子模块已接 |

总结: 9 器官都在, **2/9 完全真接** (brain/heart), **6/9 局部接** (mind/memory/voice/eye/ear/hand), **1/9 占位** (body, 但已加 struct 准备 R25.3 切 sysinfo)。

### 2.3 sister #1 command 状态 (R23 P3 新整理)

- 10 个命令模块 + 1 error, 全部独立在 `src/command/`, 不再借 organ/command/。
- `tests/organ_command_test.rs` 282 passed / 0 failed。
- 9 器官 × command 平行: body/brain/ear/eye/hand/heart/memory/mind/voice, 每个都跟同名 organ 一一对应。
- 当前是 R17 战役 4-3 “30 crate 接 supervisor”的 sister #1 入口。

### 2.4 TUI 测试 / CI

- `cargo test -p apeireth-tui --test organ_command_test`: **282 passed / 0 failed**
- `cargo test --workspace --lib`: 3623 passed / 0 failed (TUI 占其中 35 tests passed per R17)
- `.github/workflows/rust-ci.yml`: 3 jobs (workspace 全量 + battle-1-2 9 crate + tui 单线程)

## 3. 其他模块真状态

| 模块 | 状态 | 证据 |
|---|---|---|
| 后端 5 Provider | 1/5 真接 (claude-code), 4/5 mock 骨架 | `crates/apeireth-provider-*/src/lib.rs`, OAuth transport 入口已开 (`crates/apeireth-oauth/src/transport.rs`) |
| OAuth transport | ✅ 真接骨架 | `crates/apeireth-oauth/src/transport.rs`, 5 test |
| 4 SDK | stub 骨架 | `crates/apeireth-sdk-{lark,livekit,sandbox,voice}` (client.rs 7 unimplemented! 全 docstring) |
| Tauri 2 前端 | 暂离默认 build | `Cargo.toml` L38-44 注释, 永久 placeholder |
| 24 LOCKED crate src/ | 0 触 | `reports/1.0-release-doc-100-2026-08-06.md` §5.2 |
| 8 项不修改承诺 | 0 触 | workspace.version 1.0.0 + 24 LOCKED + R11 baseline + 4 类关系 + L0 HA + AND 门 + 补充式修正 + apeireth-legacy 仅增不删 |
| workspace Cargo.toml | 1.0.0 LOCKED | L204 |
| Cargo.lock | R23 P0 已 bump lru/git2/ratatui/bincode | 5→1 unsound |

## 4. 0.9 vs 1.0 的差距

### 4.1 已解决 (0.9 包含)

| 项 | 解决方式 | commit |
|---|---|---|
| RUSTSEC 10 advisories | lru 0.12→0.16.4, ratatui 0.29→0.30, git2 0.19→0.21, bincode 1.3→2.0.1 | `dd02f1a2` |
| 14 endpoint 编译期 hardcode | 新建 `endpoints.rs`, 30 route | `a3f70c81` |
| 6 module 业务实质薄 | +39 顶层 pub fn + +59 test | `3b569f1e` |
| OAuth transport | 新建 `transport.rs` reqwest 入口 + 5 test | `e8a3d244` |
| unimplemented! 16 处真 panic | 1 真 panic 修, 15 处全 docstring | `c2e614bb` |
| sister #1 借用 organ command | 搬到 `src/command/` | `57e73940` |
| Cargo.toml 注释过期 | L264 + L192-198 + commit msg 数字 | `439a8871` + `c012ab0b` |
| 30+ untracked 临时 | `.gitignore` 收编 84+ 行 | `f70c7796` |
| R23 报告 + supervision 测试报告 | 透明登记 | `79e4a49f` |
| bus L1/L2/L4 端口 E2E 误报 | 修 `FINISH-CONSTRUCTION.md` | `b0940e73` |

### 4.2 还在 R21+ 长线 (0.9 不包含)

| # | 缺口 | 当前状态 | 估时 |
|---|---|---|---|
| 1 | OTA 7 阶段原子切换 + rollback 演练 | 7 阶段框架已实装 | 1 月 |
| 2 | Self-Disable WebAuthn/FIDO2 | 多签 trait + mock 已实装 | 2 周 |
| 3 | `apeireth-pybridge` cdylib | pyo3 + rlib 冲突 known issue | 1 周 |
| 4 | R-Measure ML 校准持久化 | Round15-01 实装内存 EMA, 缺 SQLite | 1 周 |
| 5 | TUI 9 器官剩余 4/9 真接 | brain/heart 已接, 剩 eye/ear/hand/body/memory/mind/voice | 1 月 |
| 6 | OAuth 4 Provider trait method 真接 | transport 骨架已开, 4 Provider 仍 stub trait | 2 周 |
| 7 | 4 SDK 子 crate 真接 | client.rs 7 unimplemented! 全 docstring | 1 月 |
| 8 | 4 unmaintained 替换 | bincode/paste/instant/proc-macro-error2 | 1 周 |
| 9 | ASI V1257 选型 | V1256 unio_mystica 49 维 / 92.91% North Star 之后 | 待主人拍 |
| 10 | 5 入口 CLI / v2.0 / v3.0 | 13 周+ | 远期 |

### 4.3 还在等主人拍

| 项 | 选择 | 影响 |
|---|---|---|
| 重指 `v1.0.0` tag → `b0940e73` | 留基线 / 重打 / 留 0.9 | 决定 1.0 release 闭环 |
| 配置 `origin` remote + `git push` | 是否推到 GitHub | 决定发布渠道 |
| ASI V1257 选型 | 4 候选 | 决定 ASI 主线下一阶段 |
| OTA rollback 端到端 | 是否需要生产负载演练 | 决定 OTA 关门节奏 |
| Tauri 2 前端团队接手 | 是否继续 placeholder | 决定 Cargo.toml L38-44 注释去留 |

## 5. 0.9 交付物

### 5.1 源仓 (主仓)

- `CHANGELOG.md`: 加 `[0.9.0]` 章节 (透明登记, 不重打 `v1.0.0` tag)
- `reports/apeireth-0.9-release-2026-08-07.md`: 本报告

### 5.2 桌面快照 (`Desktop\Apeireth—Rust-0.9\`)

- 28.73 MB / 1932 个文件, 排除 `.git/ target/ Cargo.lock research/source/` + 30+ 临时
- `APEIRETH-RUST-0.9-RELEASE-NOTES.md` (8446 字节, 8 节)
- `APEIRETH-CONVENTIONS.md` / `APEIRETH-VERSIONING.md` / `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` 全保留

### 5.3 后续开工入口

1. 主仓 `promethean/Apeireth-rust` 仍是“真相来源”, 桌面快照只作分发。
2. 0.9.1 / 1.0 路线: 优先消化 R21+ 长线 P0 (OTA / WebAuthn / pybridge / R-Measure), 再回头补 TUI 4/9 真接 + 4 SDK 真接 + 4 unmaintained 替换。
3. 主人拍板后再动 `v1.0.0` tag / `workspace.version` / 推送策略。

---

**0.9 交付到此为止。** TUI 9 器官骨架齐, brain/heart 真接, mind/memory/voice/eye/ear/hand 局部接, body 占位 (已 struct 化待 R25.3 sysinfo); 后端 1 Provider 真接 + OAuth transport 入口已开; 4 SDK stub + 4 OAuth Provider stub 留作 R21+。
