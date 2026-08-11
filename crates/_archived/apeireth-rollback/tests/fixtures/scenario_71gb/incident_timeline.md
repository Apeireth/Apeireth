# 71GB 事故时间线 (2026-08-05 紧急救援)

## 00:00 - 主人发现磁盘异常

主人早上发现 `.minimax-agent-cn\` 磁盘空间异常,
扫描发现 91 个 `agent-xxxxxx` 影子目录, 总占 **71 GB**.

## 08:30 - 主人定位根因

主人在 `v09021-commercial-extract-2026-08-05.md` 找到源头:
- v0.9.21 商业版 `RollbackService-DN4d2R0Q.js` (22KB obfuscated webpack bundle)
- 影子备份功能从不清理
- 最早影子 90 天前
- 单影子平均 780 MB
- 91 × 780 MB ≈ **71 GB**

## 13:34 - 主人拍板 A 方案

主人 13:34 拍板 A 方案:
- **派 Mavis 成员写 `apeireth-rollback` crate 根治**
- 4 重防御 hardcode: TTL / 单大小 / 总大小 / 3 钩子
- K-1 强校验 5 字样 (apeireth / rollback / snapshot / restore / must-do)
- 修改需经 6 哲学锚 + 主人审

## 19:50 - 主人拍板"派成员干"

主人 19:50 拍板"派成员干, 自己干分散注意力".
5 P0 crate skeleton 启动 (mcp-ssh / mcp-winrm / mcp-relay-image / workflow / team-lead).
RIVAL 蓝图 v09021-rust-translation-blueprint-2026-08-05.md 604 行提交.

## 20:00 - RIVAL 蓝图 §2.2.4 apeireth-rollback 6 字段设计

| 字段 | 值 |
|------|---|
| 路径 | `crates/apeireth-rollback/` |
| 源 | v0.9.21 `out/main/chunks/RollbackService-DN4d2R0Q.js` (~22KB) |
| 估 LOC | **1,000** (1:1 翻译) |
| 估工时 | **3h** (skeleton 估 5-10min) |
| Cargo.toml deps | tokio + serde + serde_json + anyhow + thiserror + git2=0.19 + apeireth-team-lead + apeireth-agent + fs_err |
| 关键 API | `RollbackService::create_snapshot` / `restore` / `list_snapshots` / `delete` |
| 1:1 翻译点 | 6 策略 (full/file/diff/git/session/auto) + 4 git 操作 (status/diff/stash/checkout) |

## 20:18 - 5 P0 crate 入 workspace 1 commit (bg_eee92caa 整合 #1)

`Apeireth-rust/Cargo.toml` 加 5 P0 crate 到 members:
- `crates/apeireth-mcp-ssh`
- `crates/apeireth-mcp-winrm`
- `crates/apeireth-mcp-relay-image`
- `crates/apeireth-workflow`
- `crates/apeireth-team-lead`

## 20:21 - apeireth-rollback skeleton (本任务)

3 估缺核心 crate 之一 (plugin / image-prompt / rollback):
- `Cargo.toml` 1566 bytes
- `src/lib.rs` 934 行 (估 500-600 行, 实际多含注释)
- `examples/rollback_demo.rs` 154 行 (71GB 事故预防演示)
- `tests/test_rollback_in_process.rs` 300 行 (Fixture 5 + t71_gb_incident_defense)
- `tests/fixtures/scenario_71gb/` 5 文件 (mock 91 个影子 + LRU plan + 时间线 + 守门脚本)

## 22:00 (估) - 整合 #2 1 commit (后续 sub-agent)

整合 #2 sub-agent:
- 加 `apeireth-rollback` 到 `Apeireth-rust/Cargo.toml:3-57` members
- 同步 Cargo.lock (新增 git2 / sha2 / libgit2-sys 等 transitive)
- 0 触碰 24 LOCKED crate
- 0 改 workspace 元数据 (version / edition 严守)

## 关键 K-1 字样 (per supervisor-prompt-818 §5.3 模式)

- `"apeireth"` (平台名, K-1 必含)
- `"rollback"` (模块名, 1:1 翻译标志)
- `"snapshot"` (核心 API, K-1 必含)
- `"restore"` (核心 API, K-1 必含)
- `"must-do"` (翻译 invariant, K-1 必含)
- `"71GB"` (incident 防止再发生, 编译期 hardcode)
