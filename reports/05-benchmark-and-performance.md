# 册 05 — 基准与性能（7 篇合册）

> **一句话定位**：性能线的**实测卷宗**——基准基线表、基准复现线（9 个测量行 7 达标 2 未达标）、冷启动与沙箱 spawn 剖析（优化前后对比）、桌面形象渲染探针、形象层路线选型、磁盘与构建缓存治理建议书 + 交叉验证验收报告；共同特征是**每个数字都有命令 + 日期 + 工具链，未达标处如实标差距倍数**。
>
> **来源篇数**：7（分配表 §05，逐篇唯一归册）｜**原篇日期跨度**：2026-08-29 ~ 2026-10-16
> **合册生成日**：2026-10-23｜**复核口径（现行真源）**：[test-baseline](../docs/03-reference/test-baseline.md) §1 ／ [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) B 行 ／ [AGENTS.md](../AGENTS.md) §5 ／ [evidence-retention-policy](../docs/04-internal/evidence-retention-policy.md)

## 目录

| # | 原标题 | 原日期 |
|---|---|---|
| 1 | Apeireth Performance Benchmark & Latency Baseline | 2026-08-29 |
| 2 | 基准复现线实测报告 | 2026-09-27 |
| 3 | 冷启动与沙箱 spawn 剖析报告 | 2026-09-27 |
| 4 | 桌面常驻形象渲染性能探针实测报告 | 2026-10-11 |
| 5 | 桌面常驻伙伴 2D 形象层渲染路线选型报告 | 2026-10-11 |
| 6 | 磁盘与构建缓存治理建议书 | 2026-10-16 |
| 7 | 磁盘与构建缓存治理建议书 — 交叉验证验收报告 | 2026-10-16 |

---

### Apeireth Performance Benchmark & Latency Baseline (2026-08-29)

**来源**：`reports/benchmark-baseline.md`（约 60 行）｜平台：AMD Ryzen 9 / Intel Core i9，32GB RAM，NVMe SSD，Windows 11 / Ubuntu 24.04｜标注「Release Baseline for v2.0.0-preview」。
**结论/要点**
- **核心引擎性能矩阵 7 行（子系统 / 基准描述 / 目标指标 / 实测值 / 状态 全 PASS）**：混合记忆检索（`apeireth-memory`，BM25 `k_1=1.2, b=0.75` + 向量余弦 + RRF 融合，10,000 节点）目标 P99 < 10 ms → **1.82 ms**；Brier 意图校准（`intent_brier.rs`，30/100/300 轮多窗口 + 域诊断聚合）< 1 ms → **0.06 ms**；程序性习惯召回（`procedural.rs`，子串 + token 重叠 + Laplace 置信排序，1,000 habits）< 2 ms → **0.14 ms**；元思维链（`meta_thinking.rs`，5 阶段递归推演状态机，内存内）开销 < 1 ms → **0.03 ms**；实时语音打断（`barge_in.rs`，原子取消查找 + tokio `Notify` 广播）< 1 ms → **0.18 ms**；进程沙箱 spawn（Windows Win32 Job Object 创建 + 限额设置 + 子进程 spawn）开销 < 15 ms → **6.40 ms**；网关 SSE 序列化（`apeireth-gateway`，流式分块格式化 + 每 token SSE framing）< 0.1 ms → **0.02 ms**。
- **资源占用与足迹 4 行**：核心 workspace（**16 crates**）clean build < 45 s → **28.4 s**；增量重建 < 3 s → **1.38 s**；桌面伴侣 UI（Svelte 5 + Tauri 2）压缩分发包 < 30 MB → **约 14.8 MB**；冷启动到首次渲染 < 500 ms → **约 195 ms**。
- **可复现性**：给出 4 条命令——`cargo test --workspace -- --nocapture`；`cargo test -p apeireth-memory --lib hybrid_search::tests -- --nocapture`；`cargo test -p apeireth-gateway --lib barge_in::tests -- --nocapture`；`cd frontend/companion-desktop && pnpm build`。
**现状复核（2026-10-23）**：**已过时（口径与结论被后续两篇取代）**——① **本篇已被明确降级为「历史探索性测量，不是产品宣称」**：[benchmark-reproduction.md](benchmark-reproduction.md) 开篇即写「本文件是 README 性能目标表的**唯一实测出处**。`reports/benchmark-baseline.md` 等历史探索性测量**不是产品宣称**」；② **workspace 规模已变**：本篇「Core Workspace (16 Crates)」→ 现行 **30 members（29 crates + bench harness）**（[Cargo.toml](../Cargo.toml) / [AGENTS.md](../AGENTS.md) §3）；③ **关键数字已被推翻**：本篇称「Process Sandbox Spawn 6.40 ms ✅ PASS」（目标 < 15 ms），而复现线实测 P50 **59.58 ms**（差距 4.0 倍）、剖析报告优化后 P50 仍 **12.6 ms**（P99 15.2 ms 略超目标，如实标注）——**同一指标结论不一致，现行以复现线 + 剖析报告为准**；④ **冷启动**：本篇未列冷启动行，复现线记 P50 61.98 ms（目标 < 10 ms，差距 6.2 倍），剖析后 P50 22.0 ms（仍 ❌，差距 2.2×）。**仍成立（方法论）**：① **「目标指标 + 实测值 + PASS/FAIL」三列并置**的表结构，是现行 README 性能目标表的格式来源；② **「每条都给复现命令」**——现行纪律已升级为「**无脚本不写数字**」（[AGENTS.md](../AGENTS.md) §1），现行唯一执行器是 [scripts/run-benchmarks.ps1](../scripts/run-benchmarks.ps1) + `pwsh scripts/gate_full.ps1 -FailIfBusy`；③ **「Svelte 5 + Tauri 2」技术栈口径**仍在现行 [frontend/companion-desktop](../frontend/companion-desktop)。**未核实**：本篇各微秒级数字在现行代码上是否仍成立——**未核实**，不作断言。
**归属链接**：../scripts/run-benchmarks.ps1、../benches、../Cargo.toml、../AGENTS.md、../frontend/companion-desktop、../reports/benchmark-reproduction.md；同册相关 本册 §2、§3

### 基准复现线实测报告 (2026-09-27)

**来源**：`reports/benchmark-reproduction.md`（约 1,100 行）｜报告日期 2026-09-27｜对应脚本 [scripts/run-benchmarks.ps1](../scripts/run-benchmarks.ps1)｜对应 harness [benches/](../benches)（workspace 包 `apeireth-bench-harness`，`publish = false`）｜原始输出另存 [artifacts/bench-run-1.md](../artifacts/bench-run-1.md) / [artifacts/bench-run-2.md](../artifacts/bench-run-2.md)。
**结论/要点**
- **纪律（开篇）**：**无重跑脚本不发布**；每个数字都来自 `scripts/run-benchmarks.ps1` 实际输出（release 优化档），同一条命令可在本机复现；**未对任何数字取整美化，未达标行如实标注差距倍数**。本文件是 README 性能目标表的**唯一实测出处**。
- **结论速览**：README 性能目标表 **9 个测量行全部**落了可一键复跑的 harness 并实测取数 = **7 行达标 / 2 行未达标**。未达标行：**微内核冷启动** P50 61.98 ms vs 目标 < 10.0 ms（**差距 6.2 倍**；Run 1 = 63.39 ms，差距 6.3 倍）；**OS 物理沙箱 spawn** P50 59.58 ms vs 目标 < 15.0 ms（**差距 4.0 倍**；Run 1 = 96.46 ms，差距 6.4 倍），含子进程启动 + 等待全程；**受控边界增量 ≈ 6.9 ms**（对照无隔离同款执行），**两种口径均在 §7 列出，表按严格口径判定**。**判定口径统一 = P50 与目标比较**；P99 / min / max / stddev 原样呈现；两轮（同日同机）数值并列暴露轮间噪声。
- **测量环境**：主机 CHULING；Windows 11 专业工作站版；AMD Ryzen 9 9955HX 16-Core；RAM 31.2 GB；rustc 1.97.1（`rust-toolchain.toml` 钉死）；优化档 `cargo run --release --locked`（workspace release profile：`opt-level = 3`、`lto = "fat"`、`codegen-units = 1`、`strip = true`）；仓库版本 `3c1edb71` + 战役工作区改动；Run 1 = 2026-09-27 03:12:20 +08:00，Run 2 = 03:16:45 +08:00。
- **结构**：§3 harness 文件清单｜§4 统一测量口径｜§5 实测结果（5.1 Run 2 发布值 / 5.2 Run 1 第二轮 / 5.3 两轮一致性量级核对）｜§6 逐基准口径与噪声边界｜§7 诚实边界：未达标行与缺口｜§8 README 回填 diff 说明｜§9 校验记录｜§10 附原始输出（逐样本 RAW 与脚本输出逐字一致）。
- **harness 覆盖的基准键（逐项含样本数 n、统计与判定）**：混合记忆检索（BM25+向量+RRF，10K 节点）`hybrid-search-query` / 混合检索索引构建（10K 节点双路插入）`hybrid-search-build` / 认知配额调度-优先级队列派发 `quota-dispatch` / 认知配额调度-抢占+PIP 上下文切换 `quota-preempt-pip` / 因果世界模型 CoW `causal-cow` / SAGA 补偿回退 `saga-rollback` / 全双工打断响应 `barge-in-roundtrip` / Ember HUD 渲染帧 `ember-hud-tick` / OS 物理沙箱 spawn `os-sandbox-spawn` / 对照基线-无隔离进程 spawn `os-sandbox-plain-spawn` / OS 沙箱边界初始化 `os-sandbox-jobobject` / 微内核冷启动 `cold-start` / 待机内存 `idle-footprint` / 会话事件折叠 `fold-surface` / 溢出落盘取回-落盘截断 `spill-truncate` / 溢出落盘取回-全文取回 `spill-retrieve` / 原子写-完整性档 `atomic-write-integrity` / 原子写-持久档 `atomic-write-durable`。
- **示例原始统计（逐样本可核）**：`atomic-write-integrity` n=200，min=0.5381 / mean=0.8897 / stddev=0.3966 / **P50=0.8011** / P90=1.1377 / **P99=2.5271** / max=3.7201 ms，判定「—（无目标行，仅记录）」，备注「同一目标文件反复替换；断电语义 = 不承诺崩溃持久」；`atomic-write-durable` n=200，min=3.0872 / mean=3.7585 / stddev=0.6950 / P50=3.6481 / P90=4.0754 / P99=5.3370 / max=11.4917 ms，备注「Windows 平台差异：无父目录 fsync 等价语义，仅文件本体落盘；fsync 成本随磁盘/文件系统变化大，如实呈现」。
**现状复核（2026-10-23）**：**部分成立（脚本与 harness 仍在，但数字需重取）**——[scripts/run-benchmarks.ps1](../scripts/run-benchmarks.ps1) 与 [benches/](../benches)（`apeireth-bench-harness`，publish = false）**当前仍存在**，原始输出 [artifacts/bench-run-1.md](../artifacts/bench-run-1.md) / [artifacts/bench-run-2.md](../artifacts/bench-run-2.md) 仍在，故**复现入口未失效**。**已过时（数字口径）**——① 「9 个测量行 / 7 达标 2 未达标」是 2026-09-27 口径，且现行 workspace 已 30 members（29 crates + bench harness），基准面随 crate 增删变化；② 「仓库版本 `3c1edb71`」为历史 commit；③ 现行**测试**数字唯一真源是 [test-baseline.md](../docs/03-reference/test-baseline.md) §1（**341 套件 / 8570 passed / 0 failed / 51 ignored，2026-10-21**）——**性能数字与测试数字是两条独立真源，不可混谈**（[00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 末节明确「性能红行现状见 `reports/benchmark-reproduction.md`」，即本篇仍是性能面现行指针）。**仍成立（方法论，本册最重要）**：① **「无重跑脚本不发布 / 每个数字来自脚本实际输出 / 不取整美化 / 未达标标差距倍数」**——与现行 [AGENTS.md](../AGENTS.md) §1「每个数字必须可复核（命令 + 日期 + 工具链）；无脚本不写数字」完全一致；② **「P50 对目标、P99/min/max/stddev 原样、两轮并列暴露轮间噪声」**的统一口径；③ **「对照基线口径并列（受控 vs 无隔离）」**——现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) B 行「性能极致」仍为 **📋 未开工**，且注明「锁竞争仍列热点测量对象」，故**性能债未被认领关闭**；④ **「被测代码既有边界如实呈现（Windows 无父目录 fsync 等价语义）」**。**未核实**：本篇各基准在现行 30-member 代码上的现值——**未实测**，故**不写新数字**。
**归属链接**：../scripts/run-benchmarks.ps1、../benches、../artifacts/bench-run-1.md、../artifacts/bench-run-2.md、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md、../rust-toolchain.toml；同册相关 本册 §1、§3

### 冷启动与沙箱 spawn 剖析报告 (2026-09-27)

**来源**：`reports/perf-profile-coldstart-sandbox-2026-09-27.md`（约 240 行）｜机器：AMD Ryzen 9 9955HX / 32 GB / Windows 11 / rustc 1.97.1（与 [benchmark-reproduction.md](benchmark-reproduction.md) 同机同日）｜复跑命令 `pwsh -NoProfile -File scripts/run-benchmarks.ps1 -Bench <key>`；剖析子命令 `profile-cold-start` / `profile-sandbox` 由基准 harness 提供（**诊断用，不计入 README 目标行**）。
**结论/要点**
- **方法纪律**：**剖析先行**——先用分段计时找出大头，再动手；**全程行为语义零变化（既有测试零回归）**；所有数字可用附带脚本一键复跑；**未达标处如实标注差距倍数，不改口径粉饰**。
- **两条红线的优化前后对照**：**OS 物理沙箱 spawn**——优化前大头 = **受控创建路径的挂起子进程主线程恢复**：系统全线程快照 + 遍历 ≈ **27 ms/次**；其次监督循环 10 ms 轮询步长 ≈ **+5 ms**；子进程本身仅 ≈ **8 ms**。优化 = 创建句柄直接 `ResumeThread` + 事件化等待。结果 **P50 47.3 → 12.6 ms ✅ 达标**（**P99 15.2 ms 略超目标，如实标注**）。**微内核冷启动**——优化前大头 = **会话存储连接池 build 时急切建满 10 条文件连接**（省下 ≈ 4~11 ms，随负载）+ 两个全新库的文件创建 / schema DDL（合计 ≈ **18 ms**，其中文件系统侧 ≈ 8.5 ms/库）。优化 = 连接池启动期 1 条空闲连接 + 迁移执行编排合并为单事务提交。结果 **P50 22.0 ms，仍 ❌（差距 2.2×），诚实标注**。
- **关键反转（方法论要点）**：优化前报告曾推测沙箱 spawn 大头是「子进程冷启动不可消」——**分解实测推翻了该推测**：纯子进程地板（spawn + 等待退出 + 输出读取，无执行器）只要 **8.0 ms**，而无隔离 execute 全程 43.4 ms、沙箱 execute 全程 49.6 ms ⇒ **大头在执行器受控创建路径，不在子进程**。修复后无隔离 execute 全程 **7.5 ms ≈ 子进程地板（执行器附加 ≈ 0.2 ms）**，受控边界增量 ≈ **5.1 ms**。
- **结构**：§2 剖析方法｜§3 冷启动分段计时（优化前，n=30，每轮全新数据目录）｜§4 沙箱 spawn 分解（优化前，n=50）｜§5 根因探针：受控创建路径的 ~35 ms 去哪了｜§6 优化实现（行为语义零变化）｜§7 前后对比（同日同机 P50/P99；7.1 主对比同条件 / 7.2 README 复现口径 `-Bench all` 全量与历史参照 / 7.3 分段对照 / 7.4 沙箱分解对照）｜§8 双口径讲透（沙箱 spawn）｜§9 诚实边界与剩余空间｜§10 校验清单（全部实跑）。
- **§10 校验清单（全部实跑）**：`cargo test --workspace --all-targets --locked` **全绿**（exit 0, 0 失败；含 tools 真实子进程执行器 25 项、storage 130、memory 803+）；`cargo clippy --workspace --all-targets --locked -- -D warnings` **0 警告**；`cargo fmt --all --check` **0 diff**；`pwsh -NoProfile -File scripts/check-neutral-terms.ps1` **零命中**（50 changed files，added lines all neutral）；`run-benchmarks.ps1` 复测 —— os-sandbox 两轮 P50 **12.81 / 12.60**（量级一致），cold-start 空闲窗口两轮 P50 **22.57 / 22.04**（量级一致）；**并发负载窗口读数 33~48 ms 如实呈现（§7.2），不剔除、不取平均**。
**现状复核（2026-10-23）**：**部分成立（脚本与纪律仍在，数字需重取）**——[scripts/run-benchmarks.ps1](../scripts/run-benchmarks.ps1) 现存且含 `profile-cold-start` / `profile-sandbox` 剖析子命令入口；[scripts/check-neutral-terms.ps1](../scripts/check-neutral-terms.ps1) 现存（中性扫描仍是现行纪律，见 [AGENTS.md](../AGENTS.md) §2）。**已过时（数字口径）**——「47.3 → 12.6 ms」「22.0 ms」「43.4 / 49.6 / 7.5 ms」「12.81 / 12.60」「22.57 / 22.04」均为 2026-09-27 同机同日实测；现行 **测试**真源 341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21，[test-baseline.md](../docs/03-reference/test-baseline.md) §1），**性能数字须重跑本脚本取现值，本册不代写新数字**。**已过时（测试明细口径）**——§10 的「tools 25 项 / storage 130 / memory 803+」为彼时 crate 级计数，现行口径只认 workspace 单一真源。**仍成立（方法论，本篇最值钱处）**：① **「剖析先行、先定位大头再动手」**——现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) B 行「性能极致：找写得不好的开刀：冷启动红行、热点、并行」仍为 **📋 未开工**，其待办内容正是本篇方法；② **「用分解实测推翻原推测」**（子进程地板 8.0 ms vs 受控路径增量 5.1 ms）——这是「不假装、不迎合既有结论」的样本；③ **「并发负载窗口读数如实呈现，不剔除、不取平均」**；④ **「行为语义零变化 + 既有测试零回归」**作为性能优化的验收前提；⑤ **「诊断用剖析命令不计入 README 目标行」**的口径隔离。**未核实**：优化后的现值与「剩余空间」——**未实测**。
**归属链接**：../scripts/run-benchmarks.ps1、../scripts/check-neutral-terms.ps1、../benches、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../AGENTS.md、../rust-toolchain.toml；同册相关 本册 §2

### 桌面常驻形象渲染性能探针实测报告 (2026-10-11)

**来源**：`reports/avatar-perf-probe-2026-10-11.md`（约 165 行）｜**用词纪律**：正文只用中性类别名（2D 形象路线 / 3D 卡通渲染路线 / 光态基线），依赖名只出现在命令块与依赖表｜**口径纪律**：预算口径取自 `docs/design/00-PHILOSOPHY.md` §8（T0 预算目标 <150MB RAM、滚动 60fps，原文标注「数值待实测校准」）｜**证据纪律**：全部数字为本日实跑采集；凡属推断逐条标注「推断」，未测的标注「未测」。
**结论/要点**
- **① 一句话结论**：三条形象负载线在 **220×260** 舞台里**全部远低于性能红线**——3D 卡通渲染路线**每帧 JS 0.254ms**、形象负载本体内存约 **9MB 级**，「负载本体口径」下轻松压进 <150MB，稳态 FPS **164.6**（顶着 165Hz 刷新率上限）；2D 形象路线更轻（**每帧 0.133ms**）；光态基线近似零成本（**每帧 0.001ms**）。两条纪律（`document.hidden` 停渲染、WebGL 不可用降级）**三线全部通过**。但**「常驻窗整体内存」口径下浏览器进程树稳态 600–670MB，超过 150MB 预算**——三线无差别，属**浏览器壳层基线而非形象负载**，此预算口径**必须先定义清楚**（§4）。对照 [avatar-runtime-selection-2026-10-11.md](avatar-runtime-selection-2026-10-11.md) 的「2D 骨骼主线、3D 备选」结论：实测**印证并小幅上修**——3D 备选的内存疑虑解除，其未升主线的剩余理由是**资产/工具链与 WebGL 依赖面，不再是内存**。
- **② 实测环境与口径**：Windows；2560×1600 @ **165Hz**；AMD Radeon 610M（独显存在但未参与）；Edge 155 无头模式（`--headless=new`）+ CDP（UA `HeadlessChrome/155.0.0.0 Edg/155.0.0.0`，与 WebView2 同内核近似，**非 WebView2 实体**，见 §8）；WebGL2 可用，ANGLE (AMD Radeon 610M, Direct3D11) 真 GPU 上下文，另跑一轮 `--disable-3d-apis` 真实禁用；舞台 220×260 透明底；**FPS 口径**：rAF 采样，均值 = 窗口实际帧数/时长，1% low = 瞬时帧率最低 1% 帧的均值，**rAF 受 165Hz 封顶，跨机对比用每帧 JS 预算折算**；**内存口径 A/B/C**：`performance.memory.usedJSHeapSize`（1s 采样、稳态中位数、测点前 `gc()`）/ `measureUserAgentSpecificMemory()`（静态服务带 COOP/COEP）/ 浏览器进程树 Windows `WorkingSetSize` 合计（15s 采样，含进程类型拆分）；Node v24.16.0；`--js-flags=--expose-gc`；每次会话独立 `--user-data-dir`。
- **③ 三线数据表（每线 60s 稳态，主跑）**：FPS 均值 **164.64 / 164.67 / 164.65**；1% low **154.40 / 157.62 / 156.31**；瞬时最低 **73.39 / 84.53 / 81.37**；负载内容（3D = 程序化人形约 20 节关节、33 draw call、6692 三角形 + 卡通渐变着色 + 待机呼吸/口型开合/视线摆动/发丝摆动；2D = 程序化骨骼小人 35 节关节、34 骨骼、40+ 描画段 + 呼吸/口型/视线/眨眼；光态 = CSS·SVG 呼吸光点，光晕+亮核、4s 周期、合成器驱动）。
- **④ <150MB 预算逐项判定**、**⑤ 两条纪律逐项判定**（纪律一 `document.hidden` 停渲染 ✅ 三线全通过，帧计数证明；纪律二 WebGL 不可用优雅降级 ✅ 两路验证通过）、**⑥ 5 分钟浸泡（内存漂移）**、**⑦ 路线终选建议**、**⑧ 诚实边界（必读）**、**⑨ 实际跑过的命令清单**（依赖安装 / 语法检查 / 6s 冒烟 / 60s 主跑 + 300s 浸泡 + `--disable-3d-apis` 降级轮 / 25s 进程级内存拆分补测 / 约束核查「确认只新建文件、未动既有文件」）。
- **交付物**：[frontend/perf-probe/](../frontend/perf-probe)（package.json / index.html / main.js / README.md，自跑探针页，三线同页顺序测 + 结果面板 + `window.__AVATAR_PERF_PROBE__`）、`results/result.json`、`results/result.webgl-off.json`、`results/result.detail.json`、`results/shot-*.png`（运行留证截图）、[scripts/avatar-perf-probe.mjs](../scripts/avatar-perf-probe.mjs)（一键采集器：静态服务 + 无头浏览器 CDP + 存数 + 摘要）。末注：**三线人形/骨骼均为代码程序化生成，无任何下载美术资产**。
**现状复核（2026-10-23）**：**部分成立（探针与产物仍在，数字需重取）**——[frontend/perf-probe/](../frontend/perf-probe) 与 [scripts/avatar-perf-probe.mjs](../scripts/avatar-perf-probe.mjs) **当前均存在**；预算口径源 `docs/design/00-PHILOSOPHY.md` §8 现存。**已过时（数字口径）**——「0.254 / 0.133 / 0.001 ms 每帧」「164.64 FPS」「600–670MB 进程树」为 2026-10-11 本机实跑（Edge 155 无头、165Hz、AMD Radeon 610M）；跨机不可直接引用（本篇自陈「rAF 受 165Hz 封顶，跨机对比用每帧 JS 预算折算」）。**已过时（与选型报告的关系已重述）**——本篇 §⑦ 对 [avatar-runtime-selection-2026-10-11.md](avatar-runtime-selection-2026-10-11.md) 结论作「印证并小幅上修」（3D 备选内存疑虑解除，剩余理由改为资产/工具链与 WebGL 依赖面）；该上修是**同批两篇之间**的更新关系，现行树内**未见对应的后续选型决议登记面**（**未核实**）。**仍成立（方法论）**：① **「预算口径必须先定义清楚」**——「负载本体口径 vs 常驻窗整体内存口径」的分层，与现行 [test-baseline.md](../docs/03-reference/test-baseline.md) §2 把「运行时通过数」与「静态计数」分开是同一原则（**同名不同义必须分层定义**）；② **「用词纪律：正文只用中性类别名，依赖名只出现在命令块与依赖表」**——与现行 [AGENTS.md](../AGENTS.md) §2 中性表述纪律及 `scripts/check-neutral-terms.ps1` 的机器化扫描一致；③ **「推断标注推断、未测标注未测」**；④ **「约束核查：确认只新建文件、未动既有文件」**；⑤ **「无头浏览器 ≠ WebView2 实体」的诚实边界**。**未核实**：当前前端面是否已接入常驻形象层、`<150MB` 预算是否已在预算真源中改写——**未核实**，不作断言。
**归属链接**：../frontend/perf-probe、../scripts/avatar-perf-probe.mjs、../docs/design/00-PHILOSOPHY.md、../frontend/companion-desktop、../AGENTS.md、../docs/03-reference/test-baseline.md；同册相关 本册 §5、avatar-runtime-selection-2026-10-11.md

### 桌面常驻伙伴 2D 形象层渲染路线选型报告 (2026-10-11)

**来源**：`reports/avatar-runtime-selection-2026-10-11.md`（约 110 行）｜**用词纪律**：正文分析使用中性类别名；具体产品名只出现在对比表、候选清单、许可证据与素材生态证据中｜**证据纪律**：每条许可结论附本次实际抓取的 URL 与关键条款摘录；**抓取失败或未覆盖的一律写「未核实」**（web_search 当时 401 故障，未做社区口碑检索，性能口碑均为工程推断并已标注）｜**许可白名单（已拍板）**：T1 公有领域等价（CC0/Unlicense）；T2 允许商用署名类（MIT/Apache-2.0/BSD/CC-BY-4.0）；**拒收**：NC / ND / 禁二次分发 / 许可不明 / 「个人免费、商用需另行授权」的专有运行时；**运行时与资产分开审**。
**结论/要点**
- **① 一句话结论**：**主线走「开放许可的 2D 骨骼动画运行时（JSON+图集、MIT 类）+ 自制或委托的形象资产」**，开放 3D 人形格式 + 卡通渲染（MIT 运行时）作为**参数面更全的备选**，帧序列精灵图**兜底**；分层 2D 网格变形的专有形象生态**仅作对照评估、不进产品**——其运行时与素材许可均落在白名单外。（初始假设大体证实，修正点见 §⑤。）
- **② 四路线对比矩阵**（A 分层 2D 网格变形（专有格式/运行时）/ B 开放·半开放 2D 骨骼动画 / C 开放 3D 人形格式+卡通渲染 / D 帧序列·精灵图），每路线逐维给**运行时许可 / 资产生态许可分布 / WebView 性能 / 表情·口型·视线参数面 / 维护活跃度 / 结论**：A 专有双许可 + 年营收 ≥1000 万日元须签出版许可并付费 → **白名单外，拒收**；B 主流格式运行时**绑定编辑器授权** → 拒收；MIT 类 JSON+图集运行时 → **白名单内**；C 渲染库与加载库均 MIT → **白名单内**（格式规范文本许可未核实）；D 零第三方运行时 → 零许可风险。矩阵内 WebView 性能列含**工程推断/未核实**标注。
- **候选清单**（产品名仅列于此与证据节）：A 路线 1 项（专有许可 + 出版许可制度，拒收）；B 路线 3 项（自定义许可绑定编辑器授权 → 拒收；JSON+图集 MIT → 通过，编辑器许可另核；运行时 MIT 但格式为闭源二进制 → 有条件通过、格式不开放）；C 路线 1 项（MIT + MIT → 通过，规范文本未核实）。
- **③ 各候选许可证据（URL + 关键条款摘录）**、**④ 资产生态许可分布（附证据）**、**⑤ 推荐组合与风险**、**⑥ 未核实项清单（诚实边界）9 条**：① 3D 人形格式规范文本许可——规范仓库根目录 LICENSE/LICENSE.md/LICENSE.txt 在 master/main/dev 分支实抓均 404，README 无声明 → **规范文本许可未核实**（运行时许可 MIT×3 已核实）；② 某路线工具链编辑器许可（官网仅返回标题，条款未核实；旧编辑器免费但闭源的印象**未核实**）；③ 半开放方案的编辑器订阅条款与文件格式规范开放性（pricing 页仅返回标题，terms 页 404 → 未核实）；④ 开源运行时仓库的精确维护活跃度（GitHub API 限流 403、Atom feed 抓取失败 → 仅确认 README 版权年与工具链转向声明）；⑤ 平台市场内模型许可的具体分布比例（购买页正文截断、政策页连接重置、条款 403 → 仅确认「逐作品许可管理」机制存在，比例未核实）；⑥ 各路线在 WebView2/WKWebView 的实测内存/CPU/失焦口碑（web_search 401 → 矩阵中 WebView 性能列为工程推断，无引用来源）；⑦ 某路线素材市场与合作角色授权的分布（未抓取，未核实）；⑧ 免费素材许可协议全文（协议正文未抓取 → 摘录以样本条款页为准）；⑨ 某基础图形规范文本许可（未抓取，未核实）。
**现状复核（2026-10-23）**：**部分成立（选型结论与许可白名单仍可引用，证据链需重抓）**——**许可白名单（已拍板）与「运行时与资产分开审」两条是方法论，仍成立**；但全部 URL 为 2026-10-11 当日抓取或失败记录，**证据链已过期**。**已过时（候选生态状态）**——许可条款、订阅价格、维护活跃度、素材市场分布均为 2026-10 快照；**未核实**现行是否已有后续选型落地或路线决议登记面（本轮未见）。**仍成立（方法论，本篇最值得继承）**：① **「许可白名单先拍板 + 运行时与资产分开审 + 白名单外明确写拒收」**——与现行 [AGENTS.md](../AGENTS.md) §2 中性表述纪律、§4.5 取材通道（「先测活再取材、失败快退换节点」）与现行许可治理面（[deny.toml](../deny.toml) / [audit.toml](../audit.toml) / `cargo deny check` 的 bans / advisories / licenses / sources 四段，见 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D5）同构；② **「未核实项清单成节列出 + 逐条写清失败原因（404 / 403 / 限流 / 连接重置 / 401）」**——与现行 [evidence-retention-policy.md](../docs/04-internal/evidence-retention-policy.md) 的「抓取失败如实登记」一致；③ **「用词纪律：产品名只出现在对比表、候选清单、许可证据与素材生态证据中」**——这是仓内**研究引注豁免**的现成范式，与本合册「不写第三方产品名」的约束可直接对接（正文用中性类别名，证据节保留候选名）。④ **「初始假设大体证实，修正点见 §⑤」**（先出结论再标注修正，不掩盖）。
**归属链接**：../deny.toml、../audit.toml、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/04-internal/evidence-retention-policy.md、../AGENTS.md、../scripts/check-neutral-terms.ps1；同册相关 本册 §4、avatar-perf-probe-2026-10-11.md

### 磁盘与构建缓存治理建议书 (2026-10-16)

**来源**：`reports/disk-build-cache-advisory-2026-10-16.md`（约 160 行）｜提出方：DSH 会话（受主人委托做 C 盘磁盘压力排查）｜交付对象：项目主工程师｜状态：待主工程师处理｜验收方式：处理完成后由本会话做交叉验证检查验收（清单见 §5）｜先例参照 `reports/target-cleanup-2026-08-20.md`（上一轮 target 手术式清理）。
**结论/要点**
- **§0 TL;DR**：C 盘 **952.9GB** 曾仅剩 **71.8GB**；已做应急清理（约 **145GB**），现余 **147.5GB**。遗留 **5 个工程问题**待处理，其中两个 **P0**：① **构建调试信息策略缺失** → `target/` 膨胀到 **186.7GB**（§4 P0-1）；② **rustup 默认 nightly 工具链损坏** → 项目外任何 Rust 编译直接报错（§4 P0-2）。
- **§1 背景**：主人反馈「电脑没内存了」；排查结论 **RAM 32GB 仅用 53%，问题在 C 盘磁盘**（952.9GB 已用 881GB）；大头是 Rust `target/` 编译缓存与若干个人大文件（游戏/本地模型）；本建议书聚焦**工程侧可根治项**。
- **§2 现状证据（2026-10-16 实测）**：`Apeireth-rust\target` **186.7 GB**（全盘最大可治理项；8/20 清理后重新膨胀）；`apeireth-debt\target` 53.3 GB → **已删**（worktree 分支缓存）；`frontend\companion-desktop` **18.0 GB**（10/15 实测 3.11GB → 10/16 涨到 18GB）；C 盘剩余 71.8 GB → **147.5 GB**。
- **§3 已执行的应急清理（供对账，无需复核）**；**§4 待办建议 5 项**：**P0-1** 构建调试信息策略落地（target 治理核心）｜**P0-2** rustup 默认 nightly 工具链损坏｜**P1-1** target 手术式清理策略（代替整目录删除）｜**P1-2** worktree 生命周期管理｜**P1-3** `frontend/companion-desktop` 体积构成查明（18GB）。**§5 交叉验证验收清单（本会话执行）**；**§6 附录：证据复现命令**（盘面 / target 体积 / **调试信息实测对照实验**——临时项目 `Cargo.toml` 设 `[profile.dev] debug = 2`、`.cargo/config.toml` 设 `[profile.dev] debug = "line-tables-only"`，`cargo build -v` 观察 `-C debuginfo=` 值 → **实测 config 胜出** / 工具链 `rustup toolchain list; rustc -vV`（项目外执行）/ worktree `git worktree list` + `git rev-list --left-right --count origin/main...origin/fix/historical-debt`）。
**现状复核（2026-10-23）**：**已过时（结论已被同批验收报告闭环）**——本篇是**建议书**，其 §5 清单已于 2026-10-16 晚由同批验收报告（见本册 §7）逐项验收，**终态 7 项全通过**（初验 5✅/1⚠️/1❌，R1/R2 整改当日复验关闭）。**已过时（磁盘数字）**——186.7GB / 147.5GB / 18.0GB / 71.8GB 均为 2026-10-16 时点值，现行**无对应登记面**（**未核实**）。**仍成立（方法论）**：① **「区分『本会话已执行的应急清理』与『待办建议』，前者供对账无需复核」**——与现行 [evidence-retention-policy.md](../docs/04-internal/evidence-retention-policy.md) §1「过程证据不入 git、只把结论入库」的分层一致；② **「先做对照实验再下结论（config 胜出于 Cargo.toml profile）」**——与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) D6 的「生产/测试拆缝 + 纯判据面」方法同源（**用实验确定事实，不靠推断**）；③ **「工程侧可根治项」与个人大文件分开**；④ **「worktree 生命周期管理」**——现行债栏 D31 记有同族环境性问题（本机会话 token 权限收紧致全量门禁环境性失败），D28 记有「多线并发编译导致页文件耗尽 `os error 1455`」，二者共同说明**并发构建的环境债仍是活问题**。**未核实**：target 目录当前体积与磁盘现状——**未核实**，不作断言。
**归属链接**：../docs/04-internal/backend-enhancement/00-backlog.md、../docs/04-internal/evidence-retention-policy.md、../Cargo.toml、../.cargo/config.toml、../rust-toolchain.toml；同册相关 本册 §7；历史 `reports/target-cleanup-2026-08-20.md`（已迁出）

### 磁盘与构建缓存治理建议书 — 交叉验证验收报告 (2026-10-16)

**来源**：`reports/disk-build-cache-advisory-acceptance-2026-10-16.md`（约 145 行）｜验收方：DSH 会话（受主人委托，**独立交叉验证**）｜验收对象：建议书 §5 清单 + 主工程师整改工作集（**未提交态**）｜工具链 cargo 1.97.1 (c980f4866 2026-06-30) / rustc 1.97.1 (8bab26f4f 2026-07-14)，`-j 2` 口径｜**结论：通过（复验收口）—— 初验 5✅/1⚠️/1❌，R1/R2 整改当日复验关闭，终态 7 项全通过**。
**结论/要点**
- **§1 验收总表 7 项**：① 依赖调试信息关闭（P0-1）✅——工作区 9 重编单元全 `-C debuginfo=2`；依赖侧对照实验 `-C strip=debuginfo`；② 构建/测试不回退 ✅——BUILD/TEST 双 exit 0，**5392 passed / 0 failed**，fmt 0 diff，clippy `-D warnings` 0 警告；③ 默认工具链健康（P0-2）✅——项目外 `cargo new && cargo build` 成功，rustc 1.97.1 正常响应；④ 清理脚本落地（P1-1）✅（复验）——双脚本 + 5 步实测链全过，初验缺口 pdb 档经 R1 整改复验关闭；⑤ worktree 台账（P1-2）✅——台账建立、回收条件正确、**分支未合并故不 remove（正确）**；⑥ companion-desktop 构成表 + 清理（P1-3）✅（复验）——R2 整改：构成表入 maintenance-guide + 脚本纳管 + 构建可恢复；⑦ target 体积趋势 ✅——**186.7GB 基线 → 实测 147.1GB**；C 盘 free **176.9GB**；阈值机制就位。
- **§2 逐项证据（每数字可复核：命令 + 日期 + 工具链）**：#1 配置落点（方案 A）= `Cargo.toml:219-220` `[profile.dev.package."*"] debug = 0`；真实构建实测（`cargo build --workspace --all-targets --locked -j 2 -v`，2026-10-16 17:5x）本轮 9 个重编单元**全部为工作区 crate**（apeireth_tui / apeireth_bench_harness / 6 个 example+test bin），全部 `-C debuginfo=2`——**自有代码完整调试保住**；原文另记「曾按 `-L native=...\.cargo\registry\...` 误分类出 3 个『registry 单元』，**复核原始行确认为 windows_x86_64_msvc 原生库路径，非依赖编译**」；依赖侧语义 = 临时工程同款配置 + 1 个 registry 依赖，实测依赖 rustc 行**无 `-C debuginfo`** 且带 `-C strip=debuginfo`，本地 crate `-C debuginfo=2`；**体积实证 186.7GB → 154.5GB**。
- **§3 整改项与复验口径**；**§4 复验记录（R1/R2 收口，2026-10-16 晚）**：R1 = sweep v2 增补 pdb 档 ✅ 关闭；R2 = companion-desktop 纳管 ✅ 关闭（1 非阻塞备注）；终态 7 项全通过。
- **附录 A：证据复现命令**（调试信息真实构建 + 对照实验 / 测试 / 工具链 / `git worktree list` + `git rev-list --left-right --count` / pdb 与 companion-desktop 体积统计 / `Get-PSDrive C`）；**附录 B：本轮验收产生的记录**——测试日志 `%TEMP%\accept-test.log`、构建日志 `%TEMP%\accept-build-v.log`、clippy `%TEMP%\accept-clippy.log`（**一次性证据，未入库**）+ 本报告 + 被验收建议书。
**现状复核（2026-10-23）**：**已过时（测试数字）**——「**5392 passed / 0 failed**」为 2026-10-16 口径；现行唯一真源 = [test-baseline.md](../docs/03-reference/test-baseline.md) §1：**341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）**。**已过时（配置落点）**——`Cargo.toml:219-220` 的 `[profile.dev.package."*"] debug = 0` 行号与内容须以现行根 [Cargo.toml](../Cargo.toml) 为准（**未核实**该配置是否仍在现行文件、是否已随 2026-10-20 R4b 归档与图像家族入树而调整）。**仍成立（方法论，本篇是全册交叉验证范式最完整的一篇）**：① **「每数字可复核：命令 + 日期 + 工具链」**——与 [AGENTS.md](../AGENTS.md) §1 完全一致；② **「初验不通过 → 记整改项 → 当日复验 → 逐项关闭」**（5✅/1⚠️/1❌ → 7/7）的闭环写法，与现行 [00-backlog.md](../docs/04-internal/backend-enhancement/00-backlog.md) 债栏「状态注记 + 复核日 + 残留如实标注」同构；③ **「误判自查并如实更正」**（3 个 registry 单元的误分类经复核原始行更正）——与现行 D26 / D27 / D30 / D31 的环境债登记写法一致；④ **「分支未合并故不 remove（正确）」**——保守正确处置优先；⑤ **「一次性证据不入库」**（日志留在 `%TEMP%`）——正是现行 [evidence-retention-policy.md](../docs/04-internal/evidence-retention-policy.md) §3「过程证据不入仓库，只写结论」的早期实践；⑥ **「clippy `-D warnings` 0 警告 / fmt 0 diff / BUILD+TEST 双 exit 0」** 的三门禁联检——与现行 §5 校验命令一致。**未核实**：根 Cargo.toml 中调试信息配置的现状与 target 当前体积——**未核实**，不作断言。
**归属链接**：../Cargo.toml、../docs/03-reference/test-baseline.md、../docs/04-internal/backend-enhancement/00-backlog.md、../docs/04-internal/evidence-retention-policy.md、../AGENTS.md、../rust-toolchain.toml；同册相关 本册 §6

---

## 现状对照（2026-10-23）— 本册 7 篇横向口径

| 维度 | 原篇口径 | 现行口径（真源） | 判定 |
|---|---|---|---|
| 性能数字真源 | benchmark-baseline.md（2026-08-29）自述为基线 | **benchmark-reproduction.md 声明为 README 性能目标表唯一实测出处**；baseline.md 是「历史探索性测量，不是产品宣称」 | 已过时（base 被降级） |
| 沙箱 spawn | baseline 记 6.40 ms ✅ PASS | 复现线 P50 59.58 ms（差距 4.0×）→ 剖析优化后 P50 12.6 ms（P99 15.2 ms 略超） | 已过时（结论相反） |
| 冷启动 | 未列 | P50 61.98 ms（差距 6.2×）→ 优化后 22.0 ms（仍 ❌，2.2×） | 已过时（数字） |
| 测试数字 | 5392 passed（验收报告）/ 368 组（册 02） | **341 套件 / 8570 passed / 0 failed / 51 ignored（2026-10-21）** | 已过时 |
| 脚本与 harness 入口 | `scripts/run-benchmarks.ps1` / `benches/` | **两者仍存在**，复现入口未失效 | 部分成立 |
| 探针与产物 | `frontend/perf-probe/` / `scripts/avatar-perf-probe.mjs` | **两者仍存在** | 部分成立 |
| 「每数字可复核：命令+日期+工具链」 | 贯穿全册 | **仍成立**（AGENTS.md §1「无脚本不写数字」） | 仍成立 |
| 「未达标标差距倍数，不改口径粉饰」 | 复现线 / 剖析 / 探针 | **仍成立** | 仍成立 |
| 性能债状态 | B 行「冷启动红行」 | **📋 未开工**（锁竞争仍列热点测量对象） | 仍成立（债未清） |
| 许可白名单 + 运行时/资产分开审 | 选型报告 | **仍成立**（现行 deny/audit 三段式许可治理） | 仍成立 |
| 用词纪律（中性类别名 + 产品名只在证据节） | 选型 / 探针 | **仍成立**（AGENTS.md §2 + check-neutral-terms.ps1） | 仍成立 |
| 磁盘/构建缓存治理 | 建议书 + 验收 | **已闭环（7/7 通过）**；现行仍有环境债 D28/D31 | 部分成立 |

**本册复核统计**：7 篇中标「**已过时**」结论 **17 条**、「**仍成立**」**15 条**、「**部分成立**」**5 条**、「**未核实**」**6 条**。

*本册为 `reports/` 合册产物；来源 7 篇与分配表 `doc-audit/reports-assignment.txt` §05 一致（逐篇唯一归册）。历史内容按原文保留、不追溯改写；现行口径一律另起「现状对照（2026-10-23）」并指明真源。*
