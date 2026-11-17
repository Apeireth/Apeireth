# 时间库迁移 · 进度页

[Document-Meta]
Document: docs/status/chrono-migration-20261117.md
Class: STATUS
Status: 🟡 已立项 · 未开工
Last-Modified: 2026-11-17
Author: lead

> **用途**：本工作线的「做到哪了」。断言只引文件（设计稿 / 公告 / 守卫输出），不引对话摘要。
> 逐条回答设计稿[批次与验收](../design/chrono-migration-20261117.md#批次与验收)的验收要求。

## 一、批次状态（2026-11-17）

| 批次 | 状态 | 证据（文件 / 命令 / 实测输出） |
|---|---|---|
| M1 盘点与选型 | ❌ 未开工 | 本工作线 2026-11-17 立项时尚无盘点产出；`chrono` 的在役引用面尚未逐条列出 |
| M2 替换 | ❌ 未开工 | 6 个在役 crate 的 `chrono` 调用面一行未改（`Cargo.toml` 与 `Cargo.lock` 仍指向 `chrono 0.4.45`） |
| M3 撤豁免 | ❌ 未开工 | 三处豁免仍在：`deny.toml` / `audit.toml` / `.github/workflows/cargo-audit.yml` 的 `--ignore` 各有一条 `RUSTSEC-2026-0543` |
| M4 收口 | ❌ 未开工 | 无提交、无文档更新 |

## 二、还差什么

- ❌ **还差**：M1 的调用面盘点（6 个在役 crate × 纪元/带时区/本地时间/时长/格式化/serde 六个维度，按文件:行号）。
- ❌ **还差**：两个候选替代实现的选型结论（理由须覆盖时区、纪元、本地时间、格式化、serde、体积六个维度）。
- ❌ **还差**：M2 的逐 crate 替换与每个 crate 的 clippy + test 双绿证据。
- ❌ **还差**：M3 的三处豁免同批删净，以及删净后 `cargo deny check` 四类全 ok 的实测输出。
- ❌ **待补**：纪元/时区相关的**等价断言**（禁止靠放宽既有断言让测试变绿）。
- ❌ **待补**：存量持久化数据里时间戳表示形式的排查结论（决定序列化兼容策略）。

## 三、当前生效的临时处置（不是迁移）

2026-11-17 起，公告 `RUSTSEC-2026-0543`（时间库被宣布不再维护，所有版本都受影响，公告自带「无安全升级路径」）按仓内既有政策**显式豁免**解阻塞：

| 面 | 处置 | 复审条件 |
|---|---|---|
| 依赖审计配置 | 三处同步加一条豁免（`deny.toml` / `audit.toml` / `.github/workflows/cargo-audit.yml` 的 `--ignore`） | 迁移完成即删；上游恢复维护（条目变 advisory-not-detected）即删 |
| 公告类型 | unmaintained（不再维护），**不是漏洞**：无 exploit、无 unsafe 缺陷、无数据泄漏面 | — |
| 为什么先豁免不先迁移 | 迁移跨 6 个在役 crate 且触及时间语义，属独立工作线；豁免带复审条件，不是永久静默 | — |

**诚实边界**：豁免只让依赖审计门禁恢复绿，**不改变**「时间库已被宣布不再维护」这一事实；仓内时间库面在 M2 完成前仍带该风险。

## 四、守卫（机器判对错）

```powershell
# 依赖审计四类（advisories / bans / licenses / sources）—— 本工作线的终局守卫
cargo deny check

# 迁移期间的等价性守卫（逐 crate；不得放宽既有断言）
cargo clippy -p apeireth-core --all-targets --locked -- -D warnings
cargo test -p apeireth-core --locked -j 2

# 文档真源（改完进度页/设计稿后必跑）
python3 scripts/docs_truth.py --verify --block-stale
```
