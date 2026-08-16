# 自检-AR1: workspace 架构与异常文件自检（只读）

**总体结论：⚠️**（workspace 结构完整、依赖图可解析，无孤儿 manifest；存在少量卫生问题，无致命缺陷）

## ① Workspace 盘点（cargo metadata）
- ✅ 成员 crate：**82 个**，resolver=2；`crates/` 下所有非 `_archived`/`_frozen` 的 Cargo.toml 均已是成员（14 个 archived、11 个 frozen 按 16-crate-merge-policy 有意排除）。
- ✅ 完整依赖解析（cargo metadata 全量 + --offline）EXIT=0，无解析失败。
- ✅ 无真正的构建级循环依赖：verify↔supervisor、sovereignty↔verify、tool-runtime↔tool-approval 三处"环"经 kind 核验**均为 dev-dependencies 回环**（cargo 合法），不构成构建阻塞。
- ⚠️ 架构耦合提示：verify/supervisor/sovereignty 三角、tool-runtime↔tool-approval 互相 dev-dep，边界有腐化迹象，建议后续抽公共接口。
- ⚠️ `apeireth-tool-fetch` 存在自引用 dev-dep（`apeireth-tool-fetch = { path = "." }`），合法但疑似笔误。
- ⚠️ 零内部消费者的纯 lib 成员（疑似孤儿，需确认是否经运行时动态注册）：`apeireth-provider`、`apeireth-cron`、`apeireth-experience`、`apeireth-environment`、`apeireth-config`、`apeireth-state`、`apeireth-naming-v05`、`apeireth-livekit`、`apeireth-blueprint-impl`、`apeireth-library-governance`、`apeireth-voice`、`apeireth-context-fold`（cli/tui/web/bench/e2e 等终点 crate 不在此列）。

## ② 可疑文件判定
- ❌→建议清理 `ersXXXApeireth-rust`（根目录，11KB，**已被 git 跟踪**，commit abf12243 "R125 主仓挪到 Apeireth-rust"）。内容=带 ANSI 色的 git log 转储。判定：R125 迁仓时路径 `Apeireth-rust` 被误重定向压平成的文件名（丢了 `C:/Us`），纯误产物。**建议** `git rm ersXXXApeireth-rust`（如需保留内容可先移入 `_research_mem/`）；本次只建议不删除。
- ⚠️ `crates/apeireth-memory.db{,-shm,-wal}`：SQLite 运行时产物泄漏进 `crates/`（未跟踪，WAL 486KB），命名形似 crate 易误导。**建议**：删除 + `.gitignore` 增加 `crates/*.db*`。

## ③ 自检结论
团队工作区本体健康：成员关系、锁文件、依赖图一致，可正常构建解析。需处理项均为卫生级：1 个误提交的日志转储文件（建议 git rm）、3 个泄漏的 db 文件、约 12 个疑似零消费 crate 待各负责人确认去留。
