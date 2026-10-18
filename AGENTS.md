# AGENTS.md — AI 代理施工短指令（机器可读）

> 适用：本仓库内一切 AI 代理 / 自动化施工线。**先读两份宪章再动手**：
> 工程总纲 [docs/04-internal/ENGINEER-MANIFESTO.md](docs/04-internal/ENGINEER-MANIFESTO.md)（改 src / 改 doc / 派子代理前必读）；
> 施工纪律 [docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md](docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（多 AI 并发施工宪章）。
> 本文件是索引与硬短指令；冲突时以两份宪章为准。

## 1. 0 装 PASS（诚实纪律，硬规则）

- `0 装` = 不假装：未接线写「未接线」，桩写 `unimplemented!()`/`Err(NotImplemented)` 并显式断言；
  默认关闭的能力不得写成系统现状；live E2E 没跑过就写「未实测」。
- 双向性：不许把「已实现未接线」说成 0 装，也不许把 0 装说成已接线。
- 每个数字必须可复核（命令 + 日期 + 工具链）；无脚本不写数字。

## 2. 中性表述

- 代码 / 注释 / commit 零第三方产品名、零「移植/借鉴」措辞（真实路径与研究引注走豁免表，见施工纪律 §三.4）。
- 对外文案只写已交付能力；未交付能力进路线图并标注阶段。

## 3. 口径纪律

- **测试数字**：单一真源 [docs/03-reference/test-baseline.md](docs/03-reference/test-baseline.md)——其余文档只引用不抄写。
- **crate 口径**：`workspace = 31 members（30 crates + bench harness）`（2026-10-18 许可搬运线
  第一批 +11：图像核心 vendored 家族 10 crate + 接口层 1 crate，见 docs/04-internal/license-port/；
  历史口径（2026-10-18 前）：20 members（19 crates + bench harness））。
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
- **LOCKED 5 项 0 触碰**：9 哲学锚本体 / 13 键 / 3 不可变脊柱 / workspace.version / R11 baseline——
  workspace.version 仅 release authority 可改。

## 4.5 取材通道（调研/搬运/下载批必读）

- 取材按 [docs/04-internal/github-fetch-guide.md](docs/04-internal/github-fetch-guide.md) §〇 通道实测选路：**Rust crate 源码 → crates.io tarball**（不经 GitHub、许可原件齐）、**通用网页/文档 → `web_fetch`**（2026-10-18 实测 200）、**GitHub 内容 → 节点池**（`web_fetch` 在 GitHub 域定死 `fetch failed`，别试）。**先测活再取材**（池子延迟不稳，别信历史）、raw 优先于 API（API 匿名限速是盲等主因）、失败快退换节点、**绝不经第三方中转传凭据**。

## 5. 校验命令（交付前必跑，`-j 2` 口径）

```powershell
cargo test --workspace --all-targets --locked --no-fail-fast -j 2   # 期望 0 failed
cargo clippy --workspace --all-targets --locked -- -D warnings      # 期望 0 警告
cargo fmt --check                                                    # 期望 0 diff
```

- 数字对外引用时对照 test-baseline.md 现行口径；不一致如实报「本批实测值 + 日期」，不代改真源。
- 纯文档批同样跑校验；扫描类收尾（旧版本号 / 旧口径词）要求现值文档零命中、历史卷宗如实登记。
