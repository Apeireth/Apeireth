# 时间库迁移（chrono → 公告建议的替代实现）

[Document-Meta]
Document: docs/design/chrono-migration-20261117.md
Class: SPEC
Version: 0.1
Last-Modified: 2026-11-17
Status: 🟡 已立项 · 未开工
Author: lead

## 目标与范围

把仓内**在役直接依赖**的时间库从 `chrono` 迁到公告建议的替代实现，使依赖面上**不再有「不再维护」的组件**，并因此删除 `RUSTSEC-2026-0543` 的显式豁免。

**事实基线（2026-11-17 实测 + 公告原文）**：

| 项 | 值 | 出处 |
|---|---|---|
| 公告编号 | `RUSTSEC-2026-0543` | 公告页 `https://rustsec.org/advisories/RUSTSEC-2026-0543.html`（reported 2026-09-14，issued 2026-11-17） |
| 公告类型 | **unmaintained（不再维护）** —— 不是漏洞：无 exploit、无 unsafe 缺陷、无数据泄漏面 | 公告原文「The chrono crate is unmaintained; all versions are affected.」 |
| 是否可升级解掉 | **否** | 公告原文「Solution: No safe upgrade is available!」⇒ 所有版本都受影响，升级版本解不掉 |
| 建议替代 | 公告列出两个候选替代实现 | 公告原文「Recommended alternatives」 |
| 当前锁定版本 | `chrono 0.4.45` | `Cargo.lock` |
| 依赖形态 | **在役直接依赖**（根 `Cargo.toml` 的 workspace 依赖表） | 本批实测 `cargo deny check advisories` 输出与依赖树 |
| 在役引用 crate | 6 个（adapters/cli、foundation/core、engine/runtime、engine/runtime-assembly、engine/guard、engine/memory） | 本批实测 |

**范围**：只换时间库与其直接调用面。不改业务语义、不改时间语义（时区/本地时间/纪元）、不顺手升级其它依赖、不顺手重构调用方。

**背景**：2026-11-17 公告发布当日，`main` 上的 `Cargo deny` 门禁由绿转红（advisories 段），导致同期所有 PR 无法合并。本仓的处理是：先按仓内既有政策（`deny.toml` 里「本清单只收 unmaintained 类（非漏洞）」+ 显式豁免必须带复审条件）**显式豁免**解阻塞，同时把**迁移**立为本工作线单独推进。

## 批次与验收

| 批次 | 交付目标 | 验收要求 | 当前进度 |
|---|---|---|---|
| M1 盘点与选型 | 列出 6 个在役 crate 里 `chrono` 的**全部调用面**（类型、函数、trait 实现、serde 形态），在两个候选替代实现里选定一个并写明理由 | 调用面清单按文件:行号逐条列出；选型理由覆盖「时区/纪元/本地时间/格式化/serde/大小数」六个维度；本批**不改代码** | ❌ 未开工 |
| M2 替换 | 按 crate 逐个换掉 `chrono` 的调用面，编译与测试全绿 | 每个被换的 crate：`cargo clippy -p <crate> --all-targets --locked -- -D warnings` 0 警告、`cargo test -p <crate> --locked -j 2` 0 failed；纪元/时区相关的**既有断言不得放宽**（放宽断言不算换完） | ❌ 未开工 |
| M3 撤豁免 | 删除 `RUSTSEC-2026-0543` 的三处豁免并让门禁真绿 | `deny.toml` / `audit.toml` / `.github/workflows/cargo-audit.yml --ignore` **三处同步删净**；`cargo deny check` 四类（advisories / bans / licenses / sources）全 ok；`cargo audit --json --deny warnings` 0 未忽略 advisory | ❌ 未开工 |
| M4 收口 | 迁移事实入账，工作线关闭 | 本设计稿与进度页的批次表逐行更新；`docs/INDEX.csv` 重生成；提交正文带 0 装真账 + O-6 三阶审查 | ❌ 未开工 |

## 不做（明确不做的事）

1. **不**在解阻塞批里顺手迁移 —— 跨 6 个在役 crate 的大改与一个 CI 红灯塞进同一批，风险高于分批；
2. **不**改时间语义（本地时区、纪元起点、夏令时、单调时钟）—— 迁移是**等价替换**，不是行为变更；
3. **不**放宽任何与时间相关的既有断言来让测试变绿 —— 那会让「换完」变成「测不出来」（宪章 §5.3 反向验伪）；
4. **不**升级与时间库无关的其它依赖 —— 保持本批可复核；
5. **不**删除 `deny.toml` 里其它条目 —— 本批只动本公告对应的那一条。

## 边界与风险

| 风险 | 说明 | 缓解 |
|---|---|---|
| API 形状差异 | 现役调用面用到纪元时间、带时区时间、本地时间、时长、格式化与 serde 序列化；两个候选替代实现的 API 形状不同 | M1 逐条盘点调用面；M2 按 crate 推进，每个 crate 换完即跑 clippy + test |
| 隐式时区依赖 | 格式化输出若依赖本地时区，替换后可能与原输出不一致 | M2 保留/新增**固定时区**的等价断言，禁止依赖机器本地时区 |
| 序列化兼容 | 持久化/跨进程传输若存了时间戳文本，替换后表示可能变化 | M2 先查存量数据的表示形式；必要时保留旧表示并加版本字段 |
| 豁免被遗忘 | 豁免长期挂着 ⇒ 「不再维护」变成无人复核的事实 | 豁免条目自带复审条件（迁移完成即删；上游恢复维护即删）；本工作线在登记表内可查 |

## 与其他工作线的关系

- **后端接线合同**（工作区 / 联系人·人设·会话绑定 / C1 真流式）：本工作线**不阻塞**它；两者只共享 `Cargo.lock` 与 `Cargo.toml` 的依赖声明面，合并顺序上本工作线独立推进。
- **构建产物槽位纪律**：本工作线的 `cargo` 调用按 `scripts/cargo.ps1` 的单入口口径跑（槽位 + 卫生环境变量），避免产物漂移。
