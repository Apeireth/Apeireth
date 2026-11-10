# AGENTS.md — AI 代理施工短指令（机器可读）

> 适用：本仓库内一切 AI 代理 / 自动化施工线。**先读两份宪章再动手**：
> 工程总纲 [docs/04-internal/ENGINEER-MANIFESTO.md](docs/04-internal/ENGINEER-MANIFESTO.md)（改 src / 改 doc / 派子代理前必读）；
> 施工纪律 [docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md](docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（多 AI 并发施工宪章）。
> 本文件是索引与硬短指令；冲突时以两份宪章为准。

## 0. **开工前必做：先认领「工作线」**（硬规则，2026-10-28 加）

⚠️ **背景（真实事故）**：一次上下文压缩之后，代理**忘记了「自己做过什么」**
⇒ ⇒ 误判「某模块不存在」（其实 557 行早已写好）· 误判「某能力未接线」（其实**生产已接、默认关**）
⇒ ⇒ 根因不是文档不细，而是 **拿对话摘要当事实** + **该工作线没有可查的进度页**。

### 0.1 三步（**动手之前**，不是之后）

1. **查这条工作线有没有设计稿**：
   * 先看 [docs/status/workline-registry.md](docs/status/workline-registry.md)（工作线 ↔ 设计稿 ↔ 进度页 ↔ 守卫 的登记表）；
   * 没登记 ⇒ 再搜 `docs/01-architecture/*-spec.md`、`docs/04-internal/**/*.md`、`docs/design/`。
2. **有设计稿 ⇒ 读「骨架 + 批次」，不要只读摘要**：
   * 至少读它的**章节目录**与「施工批次 / 验收标准」一节；
   * 再读它的**进度页**（登记表里那列）⇒ **那才是「做到哪了」的唯一权威**。
3. **没有设计稿 ⇒ 先写一句再动手**（放哪都行：设计稿 / 进度页 / 任务说明）
   ⇒ ⇒ **不许**在"没人知道目标形态"的情况下开工。

### 0.2 引用纪律（**这条最容易被违反**）

* **引用状态只引文件**（设计稿 / 进度页 / 台账 / 守卫输出），**不引对话记忆、不引上下文摘要**；
* 摘要可以帮你**想起去哪儿找**，**不能**当事实来源 ⇒ **凡数字与状态，务必回文件核一遍**；
* 若发现「文件写的」与「代码事实」不一致 ⇒ ⇒ **改文件**（并补守卫），**不要**两边都不动。

### 0.3 新工作线的登记义务

开工一条**新工作线**（有新设计稿或新批次计划）⇒ **必须**在
[docs/status/workline-registry.md](docs/status/workline-registry.md) 加一行：
工作线名 · 设计稿路径 · **进度页路径** · 守卫命令。
⇒ ⇒ **不登记 = 下一任（或压缩后的你）查不到进度** ⇒ 视同未交付。

## 1. 0 装 PASS（诚实纪律，硬规则）

- `0 装` = 不假装：未接线写「未接线」，桩写 `unimplemented!()`/`Err(NotImplemented)` 并显式断言；
  默认关闭的能力不得写成系统现状；live E2E 没跑过就写「未实测」。
- 双向性：不许把「已实现未接线」说成 0 装，也不许把 0 装说成已接线；凡写 `已实现`/`已接线`，按**四级口径**（IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED（口径阶梯）+ `DEFAULT ENABLED` **不是一级**，是横切属性（默认开/关，与阶梯正交），定义见 [docs/01-architecture/system-capabilities.md](docs/01-architecture/system-capabilities.md) 头部）标注落在哪一级。
- 每个数字必须可复核（命令 + 日期 + 工具链）；无脚本不写数字。

## 2. 中性表述

- 代码 / 注释 / commit 零第三方产品名、零「移植/借鉴」措辞（真实路径与研究引注走豁免表，见施工纪律 §三.4）。
- 对外文案只写已交付能力；未交付能力进路线图并标注阶段。

## 3. 口径纪律

- **测试数字**：单一真源 [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md)——其余文档只引用不抄写。
- **crate 口径**：`workspace = 30 members（29 crates + bench harness）`（2026-10-20 R4b 归档
  `apeireth-perception` 与 SDK 四孤岛后由 31→30；2026-10-18 许可搬运线第一批 +11：图像核心
  vendored 家族 10 crate + 接口层 1 crate，见 docs/04-internal/license-port/；历史口径：
  31 members（30 crates + bench harness，2026-10-18 至 2026-10-20）/ 20 members（19 crates +
  bench harness，2026-10-18 前））。
- **版本**：以根 `Cargo.toml` `[workspace.package] version` 为准（现 `2.0.0-rc.4`）；发版前按
  [docs/04-internal/release-checklist.md](docs/04-internal/release-checklist.md) 清版本残留。
- **历史不追溯改写**：带日期的历史记录保留原值，需要时加「历史口径（日期）」标注。
- **README.md / README.zh-CN.md = owner 面版（2026-10-18 已换回，草稿机制退役）**：owner 口令
  「readme换」后，`docs/design/README-draft-pending.md` / `README.zh-CN-draft-pending.md`
  （门面重排 + 小巧思 + 术语血统版）已换入本体并删除草稿；此后 README 类修正**直接改本体**、
  提交正文报 owner 复核，不再走「草稿-待换回」两段机制。**徽章/正文里的测试数字必须与
  [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md) 同日同口径，手改同步**。
  历史卷宗（`docs/design/readme-research-notes.md`、`docs/design/readme-term-lineage-audit-2026-10-18.md`）
  保留原样、不追溯改写。
- 默认开关以 `crates/engine/runtime-assembly/src/canonical/production.rs` 实况为准，
  现值表见 [docs/03-reference/capabilities-matrix.md](docs/03-reference/capabilities-matrix.md) §0.2。

## 4. 提交格式

- conventional commits：`<type>(<scope>): <一句话>`；type ∈ feat/fix/refactor/docs/test/chore/perf/ci/build/revert；
  scope = crate 名或文档路径。**1 commit = 1 任务**。
- 正文必带（模板见 ENGINEER-MANIFESTO §8.2）：0 装诚实真账（测试 / clippy / 0 触碰 LOCKED / 0 引新外部 dep）
  + **O-6 三阶审查**（总体最优 / 系统最优 / 架构最优，各带拒的 alternatives 与拒理由）+ 影响面 + 后续 stage。
- **LOCKED 5 项 0 触碰**：9 哲学锚本体 / **键集**（现行 **14 键** PHL-01..08；历史「13 键」为 2026-10-11 PHL-08 之前口径，见 ADR-0006）/ 3 不可变脊柱 / workspace.version / R11 baseline——
  workspace.version 仅 release authority 可改。

## 4.5 取材通道（调研/搬运/下载批必读）

- 取材按 [docs/04-internal/github-fetch-guide.md](docs/04-internal/github-fetch-guide.md) §〇 通道实测选路：**Rust crate 源码 → crates.io tarball**（不经 GitHub、许可原件齐）、**通用网页/文档 → `web_fetch`**（2026-10-18 实测 200）、**GitHub 内容 → 节点池**（`web_fetch` 在 GitHub 域定死 `fetch failed`，别试）。**先测活再取材**（池子延迟不稳，别信历史）、raw 优先于 API（API 匿名限速是盲等主因）、失败快退换节点、**绝不经第三方中转传凭据**。

## 5. 校验命令（交付前必跑，`-j 2` 口径）

```powershell
cargo test --workspace --all-targets --locked --no-fail-fast -j 2   # 期望 0 failed
cargo clippy --workspace --all-targets --locked -- -D warnings      # 期望 0 警告
cargo fmt --check                                                    # 期望 0 diff
```

- **Windows 侧 fmt 口径**：`cargo fmt --all -- --check` 在 Windows 上**不可用**（cargo-fmt 把每个目标文件当 argv 传给 rustfmt ⇒ 超 Windows 命令行长度上限 ⇒ 报 `文件名或扩展名太长。 (os error 206)` 并打印 rustfmt usage ⇒ 是**调用方式坏了**，不是格式坏了）⇒ Windows 改用 `pwsh -NoProfile -File scripts/fmt-check.ps1`（逐 crate `cargo fmt -p <crate> --check`，覆盖同一批文件）；CI（Linux）仍以 `cargo fmt --all -- --check` 为准。
- 数字对外引用时对照 test-baseline.md 现行口径；不一致如实报「本批实测值 + 日期」，不代改真源。
- 纯文档批同样跑校验；扫描类收尾（旧版本号 / 旧口径词）要求现值文档零命中、历史卷宗如实登记。

### 5.2 ⭐ **并行线与磁盘纪律**（2026-11-06 补 · 实测踩坑固化）

#### ① ⚠️ **`RUSTC_WRAPPER=` 必须置空**（否则你会撞 `Permission denied`）
`/usr/local/cargo/config.toml` 设了 `[build] rustc-wrapper = "sccache"`
⇒ ⇒ **sccache 守护进程以 root 运行** ⇒ ⇒ 它写的 `.d` / `.rmeta` / `.rlib` **root 拥有**
⇒ ⇒ ⇒ **以 `bai` 跑 cargo 会读不了自己的产物** ⇒ ⇒ 报
`E0786 invalid metadata files` / `could not parse/generate dep info … Permission denied`
⇒ ⇒ ⇒ ⭐ **这不是代码问题**（会误导成「代码坏了」⇒ ⇒ 修错方向）
⇒ ⇒ ⇒ **每条 cargo 命令都带 `RUSTC_WRAPPER=`**：

```bash
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= \
  bash -c 'cd /home/bai/wt-apeireth-r3 && cargo nextest run -p <crate> --offline'
```

#### ② ⚠️ **不要动不动跑全量**（磁盘 + 时间）
* `--workspace` 只在**收口 wave** 跑 ⇒ ⇒ 平时**只跑你碰到的 crate**；
* ⭐ **cargo 的磁盘占用 90% 来自测试二进制**（每个测试 = 一个可执行文件）
  ⇒ ⇒ 实测：一次全量 nextest ⇒ ⇒ **1129 个 executables = 256.3 G**（vs 17.0 G 的 `.rlib`）
  ⇒ ⇒ ⇒ 跑完立刻清（**保留 `*.so`** ⇒ ⇒ 那是 proc-macro ⇒ ⇒ 删了所有宏都要重编）：

```bash
find <target>/debug/deps -maxdepth 1 -type f -executable ! -name '*.so' -delete
```

* 磁盘守卫已入库：`python3 scripts/disk_guard.py`（磁盘 + 内存）
  ⇒ ⇒ `harness.py` 的**每条门禁之前**都会看 ⇒ ⇒ 空间不足 ⇒ ⇒ 门禁记 UNVERIFIED
  ⇒ ⇒ ⇒ **宁可「明确说没验」，也不「假装验过」**。

#### ③ ⚠️ **并行线纪律**（本仓**同时有多条线**在推 `debt-round8`）
* 开工前 `git fetch origin debt-round8` ⇒ ⇒ 看**远端最新提交**是谁的
  ⇒ ⇒ ⚠️ **`paths` 过滤器在长命分支上无效**（比 PR 的**累积 diff**）⇒ ⇒ 别指望「只跑相关路径」；
* ⚠️ **别用「逐文件 `git checkout`」落盘** ⇒ ⇒ 会**静默覆盖上游同文件的新代码**
  ⇒ ⇒ ⇒ 落盘用 **patch** ⇒ ⇒ 合并后**逐项复核上游的修复还在**；
* ⚠️ **不替别人的线做决定**：跨线在途的代码**不删**（⇒ ⇒ 走「登记 + 给理由」）
  ⇒ ⇒ 尤其**最近 3 天引入的 API** ⇒ ⇒ 一律保护；
* ⚠️ **别跑「零调用 ⇒ 可删」** ⇒ ⇒ 判据必须扫**全部消费者面**
  ⇒ ⇒ ⚠️ **`fuzz/` 是独立 cargo workspace 且 CI 真跑**（审计脚本曾因此虚报「零调用」）
  ⇒ ⇒ ⇒ 同族还有顶层 `benches/` · 顶层 `examples/`。

### 5.3 ⭐ **「反向验伪」纪律**（2026-11-07 补 · 实测踩坑固化）

#### ① 为什么：⭐ **恒假的断言比偶发红更坏**
2026-11-07 实测：`crates/engine/memory/tests/time_anchor_wiring.rs` 里有一条断言
「同一条记忆在『3年前』下的最终分 **>** 在『1年前』下的最终分」
⇒ ⇒ **它从来没绿过**（不是「偶尔红」）⇒ ⇒ 根因是
**最终 `score` 是「按路归一化」后的相对量**（`retrieval_pipeline.rs`：`normalized = raw / source_max`）
⇒ ⇒ ⇒ **每个问句的榜首恒等于 1.0** ⇒ ⇒ ⇒ `1.0 > 1.0` ⇒ ⇒ **结构上恒假**。
⇒ ⇒ ⇒ ⚠️ **闸门长期失效却没人知道** ⇒ ⇒ ⇒ 比「偶发红」更危险（偶发红会有人去查）。

#### ② 硬要求
* ⭐ **新写/改写的断言，必须有一次「反向验伪」**：
  ⇒ ⇒ **把被测接线摘掉（或把断言的输入改成必然不满足）⇒ ⇒ 该测试必须红**
  ⇒ ⇒ ⇒ ⇒ **否则「恒假 / 恒绿」会被当成「本来就绿」放过**；
* ⇒ ⇒ ⇒ 验伪要在**报告里贴出两条输出**（回退 ⇒ ⇒ **红** · 改回 ⇒ ⇒ **绿**）
  ⇒ ⇒ ⇒ ⇒ **没贴输出 ⇒ ⇒ 等于没验**；
* ⭐ **断言必须断言「可判真」的量** ⇒ ⇒ ⇒
  * ⚠️ **不许断言「按路归一化后的相对量」的跨组比较**
    （榜首恒 = 1.0 ⇒ ⇒ 跨问句比它**结构上恒假**）
    ⇒ ⇒ ⇒ ⇒ 改断言**跨问句可比**的量（如绝对量纲的分量 · 同组内部之差）；
  * ⇒ ⇒ ⇒ ⇒ 断言数**从少到多**是进步（每条都**严格**）⇒ ⇒ ⇒
    ⇒ ⇒ ⇒ ⇒ ⇒ **「放宽断言 / 加 `#[ignore]` / 跳过」⇒ ⇒ ⇒ 一律不算修好**。

#### ③ 顺带一条：⭐ **「看起来是偶发问题」先确认它是不是恒定**
* 看到「时间相关 / 竞态 / 偶发」⇒ ⇒ **先做平移/复现实验**
  ⇒ ⇒ ⇒ 若**平移后依然同一条、同样的失败** ⇒ ⇒ ⇒ 它**不是**漂移 ⇒ ⇒ ⇒ **是恒假/恒定**
  ⇒ ⇒ ⇒ ⇒ ⚠️ **「date-bomb」这个诊断很容易下错** ⇒ ⇒ ⇒ 本仓就下错过一次
  ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ ⇒ **⇒ ⇒ 诊断也要先验伪**。

### 5.1 AI 高频批的快反馈（2026-10-20 效率批；全量仍按 §5 在 wave 收口跑）

```powershell
python scripts/verify_changed.py                 # fast gate: 改动 crate + 反向依赖的 check/clippy/nextest
python scripts/docs_truth.py --verify --check    # 文档状态真源：claims 证据/staleness/生成物漂移
pwsh scripts/commit.ps1 -MessageFile <msg.txt>   # 显式 author + staged secret/neutral + 按次启用 .githooks
```

- fast gate 输出落 `logs/verify-changed.log`，**按落盘退出码判定**（不做管道直判）；不产生对外测试数字。
- 改文档或改被 `docs/status/claims.yaml` 覆盖的源码后必跑 `docs_truth.py --verify`；`STALE` =
  源码动过而 `verified_at` 未更新，交付 commit 前须复核并 bump。
- 提交走 `commit.ps1`（不全局设 `core.hooksPath`，避免拦住同工作树并行线）；详细口径见
  [docs/02-guides/fast-feedback-gate.md](docs/02-guides/fast-feedback-gate.md)。
