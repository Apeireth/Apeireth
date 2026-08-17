# R19+ 集成文档维护 SOP (27 份文档 + Hermes 协同 + 隐形资产防治)

```
[Document-Meta]
Document: docs/stage4/docs-maintenance-sop-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 文档维护
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)
```

> **性质**: 纯文档交付。**不写代码、不改任何文件** (除本文件)。给后续团队 lead / 主人 / 任何接手者**维护 R19+ 集成期 27 份文档**用。
>
> **依据**:
> - `reports/r19-integration-wrap-up-2026-08-05.md` §1-§12 (总收口, 27 份文档地图 + 5 协同 + 5 衔接 + 10 待拍板 + 8 风险)
> - 14 份 docs/ (1 蓝图 + 3 ADR + 6 实施蓝图 + 1 R20 路线 + 3 资产/SOP/词条) + 13 份 reports/
> - APEIRETH-CONVENTIONS.md §0.1 (Document-Meta 格式) + §6 (commit 规范) + §9 (6 锚穿透) + §10 (不修改承诺) + §11 (R11 baseline 3 值)
> - Hermes (code_reviewer) R18/R19 5 commit (34992e9f 等) + 8 CI workflows + 122 集成测试
>
> **不修改承诺** (跟 ADR-0011 §不修改承诺一致): 阶段 1+2+3+4+5 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 全部保留 (见 §7)。
>
> **诚实登记** (S-2 17:43):
> 1. 27 份文档 = 14 docs/ + 13 reports/, 是 2026-08-05 14:40 总收口后的口径 (总收口报告 §5 内部列 25 行, 13 份 reports/ 比 §5 表 10 份多 3 份, 主人复核后口径可能有 ±2 浮动, 以 `r19-integration-wrap-up-2026-08-05.md` §5 实际清单为准)。
> 2. 本 SOP 是"维护"文档, 不创建/修改其他 26 份。
> 3. CI workflow 伪 YAML (§4) 是设计稿, 真正落 CI 需 Hermes 团队 lead 拍板 + 写入 `.github/workflows/`, 本 SOP 不动 CI YAML (per Hermes LOCKED)。

---

## §1 战略背景 (为什么需要这份 SOP)

### 1.1 27 份文档就位 (2026-08-05 14:40)

| 维度 | 数量 | 路径 | 用途 |
|---|---:|---|---|
| **14 份 docs/** | 14 | `.openclaw\workspace\promethean\Apeireth-rust\docs\` | 蓝图 (1) + ADR (3) + 实施蓝图 (6) + R20 路线 (1) + 资产/SOP/词条 (3) |
| **13 份 reports/** | 13 | `.minimax-agent-cn\spectrai\reports\` | 1 SpectrAI 架构 + 11 Apeireth 现状 + 1 总收口 |
| **总额** | **27** | 跨 2 个工作树 | 覆盖 R19+ 集成全维度 |

**完整地图见 `reports/r19-integration-wrap-up-2026-08-05.md` §5 (25 行表, 含 1 行 tauri-roadmap 注释)**。本 SOP §5 给出**摘要引用**, 不重复拷贝清单。

### 1.2 4 类一致性风险

| 风险类型 | 触发场景 | 后果 |
|---|---|---|
| **跨 sub-agent 一致性** | 4 份微调 + 14 份新文档由 7+ sub-agent 并行写, 引用口径可能不同 (e.g. "apeireth-sdk 11 文件" vs "T13 BLOCK" vs "14000 LOC") | 后续读者 grep 出 3 个版本, 失去权威源 |
| **跨 Hermes 协同** | Hermes R19 加 clippy `-D warnings` + 8 CI workflows + 122 集成测试, 文档不标"互补位置"会被误以为要重做 | rust-coder 接手时漏跑 Hermes 已就位的守门 |
| **跨 commit 同步** | 主人或 architect 改 src/, 忘改对应文档 (e.g. 改 `apeireth-team-lead` 命名, 忘改蓝图 §5.2) | 文档跟代码漂移, 接手者按错版本实施 |
| **隐形资产化** | 27 份文档没人维护, 3 个月后 grep "TBD" 涨到 30 项, 决策散在 5 个地方 | R21+ 接手者无法快速进入, 文档失信 |

### 1.3 没有"文档同步 SOP" = 隐形资产风险

> 主人 user memory #6: 派 sub-agent 干, 但要驾驭团队不重复造轮子。**反过来, sub-agent 派出去后也要有"回来收口"机制** — 不然 27 份就是 27 个孤儿。

本 SOP 就是**回来收口机制**:
- ✅ 5 步维护流程 (commit 自检 → 周会议 → CI 校验 → 季度审计 → 版本号)
- ✅ CI workflow 设计 (给 Hermes 拍板)
- ✅ 跨文档引用地图 (§5)
- ✅ 6 哲学 anchor 穿透 (§8)
- ✅ 跟 Hermes 协同明确写出 (§2.3)

---

## §2 5 类维护需求

### 2.1 文档内部一致 (Intra-Doc Consistency)

| 检查项 | 守门方式 | 失败时 |
|---|---|---|
| **Document-Meta 6 字段齐** | `head -10` grep 6 个关键字 (Document/Version/R-Cycle/Commit/Last-Modified/Status) | CI fail, 不许 commit |
| **Commit 字段已填** | `head -10` grep `Commit: <commit` 应为 0 匹配 | CI fail + 提示"回填 commit hash" |
| **Status 状态真实** | 拍板后必须 `🔍 草拟` → `✅ 已拍板 (YYYY-MM-DD)`, 不允许长期草拟 | §3 步骤 4 季度审计警告 |
| **章节内自洽** | §5 集成映射表 vs §6 Apeireth 现状 vs §8 决策清单 三者数字对得上 | 周会议人工 review |
| **8 项不修改承诺不破** | grep `LOCKED` 关键词 + 比对承诺清单 | CI fail, 强制主人拍板 |

### 2.2 跨文档引用 (Cross-Doc Reference)

| 引用类型 | 例子 | 检查方式 |
|---|---|---|
| **战略层 → ADR** | 蓝图 §5.2 引用 ADR-0010/0011/0012 | grep ADR-00XX 编号, 比对 `docs/adr/` 实际文件 |
| **战略层 → 实施** | 蓝图 §6 引用 team-lead-implementation-guide §3 | grep 相对路径, `[[ -f ref ]]` 存在性 |
| **ADR → ADR** | ADR-0011 §不依赖 supervisor 引用 ADR-0012 | grep 编号 |
| **蓝图 → reports** | 蓝图 §4 引用 reports/spectrai-architecture §3.2 | grep 相对路径 |
| **reports → 蓝图** | reports/apeireth-crate-api §6 引用 蓝图 §5.3 | grep 蓝图路径 |
| **实施 → 词条** | team-lead-implementation-guide §2 引用 glossary-spectrAI §team-lead 词条 | grep 词条名 |
| **总收口 → 全部** | r19-integration-wrap-up §5 引用全部 26 份 | 25 行表一一对应 |

**§3 步骤 3 CI 校验** 自动 grep 引用文件存在性, 跨工作树 (docs/ + reports/) 都覆盖。

### 2.3 跟 Hermes 同步 (Hermes Sync)

Hermes (code_reviewer) 在 2026-08-05 14:30 commit 5 个 R18/R19 工作 (**34992e9f** 等), **没动 docs/**。本 SOP 标"跟 Hermes 互补"位置, 防止后续文档飘走:

| Hermes 资产 | 文档用法 | 互补位置 |
|---|---|---|
| **R18 阶段 0: workspace.lints + deny.toml** | 6 份实施指南 §2 Cargo.toml 模板只写 `[lints] workspace = true` 继承 | 蓝图 §7.1 + ADR-0011 §3.2 + 实施指南 §2 |
| **R18 阶段 1: CI 配套 (cargo-deny + rust-lint workflow)** | 5 阶段路线 §守门 写 "fmt + clippy + deny + r-measure + test" 5 重守门 | r20-stage-1-2 §守门 + r-measure-verification-design §4 |
| **R18 阶段 2: 122 集成测试 for 12 product crates** | 6 份实施指南测试数 33/33 + 28/28 + 99/99 + 5/5 跟 Hermes 互补, 不重复 | team-lead-implementation-guide §5 + session-blueprint §5 + mcp-14-tool-analysis §4 |
| **R18 阶段 3: miri + coverage + rustdoc + SECURITY.md** | apeireth-formal-invariants §2 Kani 跟 miri 互补 (Kani 形式化, miri unsafe 检查) | formal-invariants §2 + r-measure-verification §4 |
| **R19 T10: clippy `-D warnings` 真正生效** | 5 阶段路线每子阶段必跑 `cargo clippy -- -D warnings` | r20-stage-1-2 §守门 + team-lead-implementation §6 |

**R19+ 集成期每份新文档**都应在"互补位置"标 `跟 Hermes R18 X 协同 (e84c9068 commit)` 类似标记, 让接手者秒查。

### 2.4 跟 R20 衔接 (R20 Handoff)

| R20 阶段 | 文档指南 | 衔接方式 |
|---|---|---|
| **阶段 1.1** TUI 9 命令深化 + 12 新命令 | r20-stage-1-2 §2.1 | TUI 改瘦 (R25) 后**只加 UI 层** + HTTP client, 不碰后端 |
| **阶段 1.2** apeireth-team-lead 公开 API | team-lead-implementation-guide §2-§3 + ADR-0011 | supervisorPrompt.ts 1:1 翻译守门, diff < 5% |
| **阶段 1.3** apeireth-mcp::team 14 工具 | mcp-14-tool-analysis + ADR-0010 | Tool trait (R17 已有) + McpServer::from_registry 一行打包 |
| **阶段 1.4** mid-task bug 3 处修法 | session-blueprint §4 + ADR-0010 | 3 处一起改, 改 1 留 2 = 撕裂状态复发 |
| **阶段 1.5** 集成 + R-Measure 守门 | r-measure-verification-design §3 | 17→24 维投影公式 (主人从 v1077 抽) + 编译期 hardcode |

R20 阶段 2-5 实施指南 (Tauri 团队 + 部署基础 + API 公开 + SDK 完善 + 文档营销) **待写**, 本 SOP §3 步骤 1 触发"新阶段文档必须建 §R20 衔接"硬规则。

### 2.5 主人待拍板项 (Pending Decisions)

来自总收口报告 §7, 累计 10 项, 本 SOP §3 步骤 2 周会议每周对照 1 次, 防止堆积:

| ID | 待拍板 | 来源 |
|---|---|---|
| D-01 | 17→24 维 R11 baseline 投影公式 | r-measure-verification-design §2.1 |
| D-02 | V1136 9→7 子测度 R11 baseline 投影权重 | r-measure-verification-design §2.3 |
| D-03 | 24 维具体分类名 (continuity / salience / identity / reflection / 4 other) | spectrAI §7.4 + asi 24 dim |
| D-04 | apeireth-sdk 升级方案 (一起 / 分阶段) | sdk-gap-analysis §3.1 |
| D-05 | SDK_VERSION 0.1.0 → 1.0.0 升级时机 | sdk-gap-analysis §2.2 |
| D-06 | apeireth-tauri-stub 命名 (留 / 移除) | global-architecture-map §2.4 ⛔ DEPRECATED |
| D-07 | R20 vs R21 边界 | r20-product-finalize §1.1 |
| D-08 | Tauri 团队同步节奏 (每 2 周?) | tauri-team-collab-sop §3 |
| D-09 | apeireth-session LOC 上下沿 (1500-2000) | session-blueprint §3.1 |
| D-10 | session 跟 storage 依赖方向 | session-blueprint §2.2 |

**SOP 规则**: 待拍板项 > 30 天 = §3 步骤 4 季度审计告警, 主人或 Mavis 必须 1 周内消化或拆解。

---

## §3 5 步维护 SOP (团队 lead 日常执行)

### 步骤 1: 每次 commit 前自检 (3 分钟) — 每个 contributor

**触发**: 任何 commit 涉及 `docs/stage4/` 或 `docs/adr/` 或 `docs/roadmap/` 路径。

**执行清单** (3 分钟, 不许跳过):

1. **Document-Meta 字段齐**
   ```bash
   for f in $(git diff --cached --name-only --diff-filter=AM | grep -E '\.md$'); do
     head -10 "$f" | grep -q "Document:" || { echo "❌ $f 缺 Document:"; exit 1; }
     head -10 "$f" | grep -q "Version:" || { echo "❌ $f 缺 Version:"; exit 1; }
     head -10 "$f" | grep -q "R-Cycle:" || { echo "❌ $f 缺 R-Cycle:"; exit 1; }
     head -10 "$f" | grep -q "Commit:" || { echo "❌ $f 缺 Commit:"; exit 1; }
     head -10 "$f" | grep -q "Last-Modified:" || { echo "❌ $f 缺 Last-Modified:"; exit 1; }
     head -10 "$f" | grep -q "Status:" || { echo "❌ $f 缺 Status:"; exit 1; }
   done
   ```

2. **Commit 字段已回填**
   ```bash
   git diff --cached | grep -E '^\+.*Commit: <commit' && \
     { echo "❌ 检出新增 Commit 字段还是 <commit 时回填> 占位"; exit 1; }
   ```
   **或** (用 commit hash 填):
   ```bash
   HASH=$(git rev-parse --short HEAD)
   # 手动把 "Commit: <commit 时回填>" 改成 "Commit: $HASH"
   ```

3. **跨文档引用存在**
   ```bash
   for f in $(git diff --cached --name-only --diff-filter=AM | grep -E '^docs/.*\.md$'); do
     grep -oE '(docs|reports)/[a-zA-Z0-9_./-]+\.md' "$f" | sort -u | while read ref; do
       [[ -f "$ref" ]] || { echo "❌ $f 引用 $ref 不存在"; exit 1; }
     done
   done
   ```

4. **决策清单跟拍板记录一致** (新增/修改 §决策章节时)
   ```bash
   # 拿当前文件的 §决策清单 vs §拍板记录时间线, 数字必须对得上
   # 人工 review 30 秒
   ```

5. **8 项不修改承诺不破** (改文档时)
   ```bash
   # grep 文档里有无 "LOCKED" + "不动" 关键词
   # 若有, 确认不在改 LOCKED 文件本身
   git diff --cached --name-only | grep -E '(CONVENTIONS|VERSIONING|GLOSSARY|0001|0002|0003|0004|0005|0006|0007|0008|0009|r20-product-finalize|stage3-blueprints)' && \
     { echo "⚠️ 改 LOCKED 区域, 必须主人拍板"; exit 1; }
   ```

**失败处理**:
- ❌ 任 1 项 fail = 不许 commit, 改完再试
- ⚠️ 警告项 = 主人周会议题

### 步骤 2: 每周一文档同步会议 (1 小时) — 团队 lead 主持

**触发**: 每周一上午 (北京/项目时区), 团队 lead (默认 Mavis) 主持。

**执行清单** (1 小时):

1. **5 分钟: 27 份文档状态速览**
   ```bash
   for f in $(find docs/stage4 docs/adr -name "*.md") $(find reports -name "*.md" 2>/dev/null); do
     status=$(grep "Status:" "$f" | head -1)
     commit=$(grep "Commit:" "$f" | head -1)
     echo "📄 $f | $status | $commit"
   done
   ```

2. **10 分钟: grep "🔍 草拟" 状态文档**
   ```bash
   grep -l "🔍 草拟" docs/stage4/*.md docs/adr/*.md 2>/dev/null
   # 列出哪些还没拍板, 谁来推
   ```

3. **10 分钟: grep "Commit: <commit 时回填>" 占位**
   ```bash
   grep -l "Commit: <commit 时回填>" docs/stage4/*.md 2>/dev/null
   # 列出哪些 commit hash 还没填, 谁来补
   ```

4. **10 分钟: grep "TBD" / "待定" / "主人" 待拍板项**
   ```bash
   grep -rE "TBD|待定|主人拍板|待 Mavis 拍板" docs/stage4/*.md 2>/dev/null | head -20
   # 列出待拍板项堆积
   ```

5. **15 分钟: 派 1 个 sub-agent 出"周报告"**
   - 任务: 跑 4 类 grep + 拍板状态矩阵 + 跨引用失效清单
   - 产出: `reports/docs-weekly-sync-YYYY-MM-DD.md` (Manual-Rev-A)
   - 派活模板 (per Mavis user memory #6):
     ```
     任务: 跑 27 份文档状态扫描
     集成规范: 输出到 reports/docs-weekly-sync-{date}.md, Document-Meta 严格按 APEIRETH-CONVENTIONS §0.1
     不重复造轮子: 复用本 SOP §3 步骤 2 的 4 个 grep 命令
     ```

6. **10 分钟: Mavis 看周报告 + 议题梳理**
   - 草拟 > 30 天 → 议题
   - commit 占位 > 5 份 → 议题
   - 待拍板 > 10 项 → 议题
   - 跨引用失效 > 3 处 → 议题

### 步骤 3: CI 校验 (Per Hermes R18 阶段 1 扩展) — Hermes 团队 lead 拍板

**触发**: 任何 commit 改 `docs/stage4/` 或 `docs/adr/` 或 `docs/roadmap/` 自动跑。

**核心守门** (Hermes R18 已就位 + 本 SOP 新增):

| Hermes 已就位 workflow | 本 SOP 复用 | 新增 docs-stage4-check workflow |
|---|---|---|
| `rust-lint.yml` (clippy + fmt) | ✅ 不重写, 引用 | ❌ |
| `cargo-deny.yml` | ✅ 不重写, 引用 | ❌ |
| `test.yml` (122 集成测试) | ✅ 不重写, 引用 | ❌ |
| `miri.yml` / `coverage.yml` / `rustdoc.yml` / `SECURITY.md` | ✅ 不重写, 引用 | ❌ |
| — | — | 🆕 `docs-stage4-check.yml` (本 SOP 设计, 详见 §4) |

**§4 给完整 workflow 伪 YAML**。

### 步骤 4: 季度文档审计 (4 小时) — 团队 lead + 主人

**触发**: 每季度 (3/6/9/12 月最后一周), 主人 + Mavis 双复核。

**执行清单** (4 小时, 拆 4 块各 1 小时):

1. **1 小时: 27 份文档 vs R-Measure baseline 3 值对照**
   ```bash
   # 所有 27 份文档必须引用 R11 baseline 3 值守门
   grep -l "0.8682" docs/stage4/*.md reports/*.md | wc -l  # 应 ≥ 20 份
   grep -l "0.8532" docs/stage4/*.md reports/*.md | wc -l
   grep -l "0.9063" docs/stage4/*.md reports/*.md | wc -l
   # 任一 baseline 引用 < 15 份 = 警告
   ```

2. **1 小时: 27 份文档 vs 8 项不修改承诺对照**
   ```bash
   # 27 份文档不应动 LOCKED 区域 (CONVENTIONS/VERSIONING/GLOSSARY/0001-0009/...)
   # 检查 git log: 过去 90 天 docs/ 改动不能碰 LOCKED
   git log --since="90 days ago" --name-only | grep -E '(CONVENTIONS|VERSIONING|GLOSSARY|0001|stage3-blueprints|r20-product-finalize)' | head
   # 出现 = 警告, 必须主人确认
   ```

3. **1 小时: 27 份文档 vs 6 哲学 anchor 穿透**
   ```bash
   # 6 anchor 必须穿透每份战略/实施/收口文档
   for anchor in "S-1" "S-2" "O-5" "O-2" "O-3" "O-4"; do
     echo "=== $anchor ==="
     grep -l "$anchor" docs/stage4/*.md | wc -l  # 应 ≥ 10 份
   done
   # 战略层 3 份 + 收口 1 份 + 6 实施 = 10 份穿透是底线
   ```

4. **1 小时: 27 份文档 vs 12 子规范对照 (APEIRETH-CONVENTIONS §1-§12)**
   ```bash
   # 12 子规范每条至少被 1 份文档引用
   for section in "Document-Meta" "commit" "6 锚" "不修改承诺" "R11 baseline" "3 层架构" "4 组件" "权限分配" "E 层" "mid-task" "7 advisor" "Kani"; do
     echo "=== §$section ==="
     grep -lr "$section" docs/stage4/*.md | wc -l
   done
   ```

**审计报告输出**: `reports/quarterly-docs-audit-YYYY-MM-DD.md` (Manual-Rev-A), 进 §9 关联文档。

### 步骤 5: 文档版本号管理 — 每个 contributor

**触发**: 任何实质修改。

**规则**:

1. **Manual-Rev-X 编号**
   - 草拟 = `Manual-Rev-A`
   - 实质性修改 (新章节 / 新决策 / 新守门) = `Manual-Rev-X+1`
   - 微调 (修 typo / 改链接) = 不升版本, 改 `Last-Modified`

2. **Status 状态切换**
   - 草拟期: `Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)`
   - 拍板后: `Status: ✅ 已拍板 (YYYY-MM-DD)` + 决策点引用
   - 实施中: `Status: 🚧 实施中 (rust-coder 接手, per {实施指南名})`
   - 锁定: `Status: 🔒 LOCKED (owner: {Mavis/主人/team-lead})`

3. **Commit 字段回填**
   - 草拟时: `Commit: <commit 时回填>`
   - commit 时: `Commit: {实际 7-12 位 hash}`
   - amend 时: 同步更新 Commit 字段

4. **R-Cycle 字段更新**
   - 新增时: `R-Cycle: R{N}+ {阶段描述}`
   - 跨周期: `R-Cycle: R{N}+ → R{N+1}+ {过渡说明}`

---

## §4 CI workflow 设计 (Custom, 给 Hermes 团队 lead 拍板)

> **性质**: 伪 YAML 设计稿, 给 Hermes 团队 lead 拍板后写入 `.github/workflows/docs-stage4-check.yml`。本 SOP 不动 CI YAML (LOCKED)。

```yaml
# .github/workflows/docs-stage4-check.yml (设计稿, 待 Hermes 拍板)
name: docs/stage4 check
on:
  push:
    paths:
      - 'docs/stage4/**'
      - 'docs/adr/**'
      - 'docs/roadmap/**'
  pull_request:
    paths:
      - 'docs/stage4/**'
      - 'docs/adr/**'
      - 'docs/roadmap/**'

jobs:
  check:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # 需 git log 算年龄

      - name: Check Document-Meta 6 字段
        run: |
          set -e
          changed=$(git diff --name-only HEAD~1 HEAD 2>/dev/null | grep -E '\.md$' || true)
          for f in $changed; do
            if [[ ! -f "$f" ]]; then continue; fi
            head -10 "$f" | grep -q "^Document:"    || { echo "❌ $f 缺 Document"; exit 1; }
            head -10 "$f" | grep -q "^Version:"     || { echo "❌ $f 缺 Version"; exit 1; }
            head -10 "$f" | grep -q "^R-Cycle:"     || { echo "❌ $f 缺 R-Cycle"; exit 1; }
            head -10 "$f" | grep -q "^Commit:"      || { echo "❌ $f 缺 Commit"; exit 1; }
            head -10 "$f" | grep -q "^Last-Modified:" || { echo "❌ $f 缺 Last-Modified"; exit 1; }
            head -10 "$f" | grep -q "^Status:"      || { echo "❌ $f 缺 Status"; exit 1; }
          done
          echo "✅ Document-Meta 6 字段全齐"

      - name: Check Commit 字段已回填
        run: |
          set -e
          changed=$(git diff --name-only HEAD~1 HEAD 2>/dev/null | grep -E '\.md$' || true)
          for f in $changed; do
            if [[ ! -f "$f" ]]; then continue; fi
            if head -10 "$f" | grep -q "Commit: <commit 时回填>"; then
              echo "❌ $f Commit 字段还是 <commit 时回填> 占位"
              exit 1
            fi
          done
          echo "✅ Commit 字段已回填"

      - name: Check 跨文档引用存在
        run: |
          set -e
          changed=$(git diff --name-only HEAD~1 HEAD 2>/dev/null | grep -E '^docs/.*\.md$' || true)
          for f in $changed; do
            if [[ ! -f "$f" ]]; then continue; fi
            # grep 形如 docs/xxx/yyy.md 或 reports/xxx.md 的引用
            refs=$(grep -oE '(docs|reports)/[a-zA-Z0-9_./-]+\.md' "$f" | sort -u || true)
            for ref in $refs; do
              if [[ ! -f "$ref" ]]; then
                echo "❌ $f 引用 $ref 不存在"
                exit 1
              fi
            done
          done
          echo "✅ 跨文档引用全存在"

      - name: Check 拍板状态年龄
        run: |
          for f in $(find docs/stage4 docs/adr -name "*.md" 2>/dev/null); do
            status=$(head -10 "$f" | grep "Status:" | head -1 || true)
            if echo "$status" | grep -q "🔍 草拟"; then
              # 取 Last-Modified 算天数
              last_mod=$(head -10 "$f" | grep "Last-Modified:" | head -1 | awk '{print $2}')
              # 简化: 用文件 mtime 算
              age_days=$(( ( $(date +%s) - $(stat -c %Y "$f") ) / 86400 ))
              if [[ $age_days -gt 30 ]]; then
                echo "⚠️ WARN: $f 草拟超 30 天 ($age_days days), 待 Mavis 拍板"
              fi
            fi
          done
          echo "✅ 草拟年龄检查完成"

      - name: Check 8 项不修改承诺不破
        run: |
          set -e
          # 改 LOCKED 区域 = 强制主人拍板
          locked_patterns=(
            "APEIRETH-CONVENTIONS\.md"
            "VERSIONING\.md"
            "GLOSSARY\.md"
            "docs/adr/000[1-9]-"
            "docs/stage3-blueprints/"
            "docs/roadmap/r20-product-finalize-2026-08-05\.md"
          )
          changed=$(git diff --name-only HEAD~1 HEAD 2>/dev/null || true)
          for pattern in "${locked_patterns[@]}"; do
            if echo "$changed" | grep -qE "$pattern"; then
              echo "❌ 改 LOCKED 区域匹配 $pattern, 必须主人拍板"
              exit 1
            fi
          done
          echo "✅ 8 项不修改承诺守住"

      - name: Check R11 baseline 3 值守门 (仅战略/收口文档)
        run: |
          set -e
          # 战略层 + 收口报告必须含 baseline 3 值
          required_docs=(
            "docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md"
            "docs/stage4/r20-stage-1-2-implementation-2026-08-05.md"
            "docs/stage4/r-measure-verification-design-2026-08-05.md"
          )
          for f in "${required_docs[@]}"; do
            [[ -f "$f" ]] || continue
            grep -q "0.8682" "$f" || { echo "❌ $f 缺 V1141=0.8682"; exit 1; }
            grep -q "0.8532" "$f" || { echo "❌ $f 缺 V1131=0.8532"; exit 1; }
            grep -q "0.9063" "$f" || { echo "❌ $f 缺 V1136=0.9063"; exit 1; }
          done
          echo "✅ R11 baseline 3 值守门守住"

      - name: 季度审计 hint (注释, 季度手动跑)
        if: false  # 季度触发, 不在每次 commit 跑
        run: |
          echo "📋 季度审计清单见 docs/stage4/docs-maintenance-sop-2026-08-05.md §3 步骤 4"
```

**CI 跑通条件** (跟 Hermes R18 8 workflows 互补):
- `rust-lint.yml` 通过 + `cargo-deny.yml` 通过 + `docs-stage4-check.yml` 通过
- 总时长 ~30-35 分钟 (5 重守门), docs-stage4-check 单独 ~5 分钟
- 不挡 PR 主流程 (r-measure-verify 也是单独 workflow, 同理)

---

## §5 文档地图 (27 份摘要, 完整见总收口报告 §5)

> 完整 25 行表见 `reports/r19-integration-wrap-up-2026-08-05.md` §5 (含 1 行 tauri-roadmap 注释)。本节只给分类摘要 + 引用规则, 防止双重维护。

### 5.1 14 docs/ 分类

| 类别 | 数量 | 文档 | SOP 引用规则 |
|---|---:|---|---|
| **战略层** | 3 | 蓝图 + R20 路线图 + 全局架构图 | 任何引用 = 根, 加 §编号 |
| **ADR 层** | 3 | ADR-0010/0011/0012 | 决策依据, 引用 ADR-00XX §编号 |
| **实施蓝图层** | 6 | team-lead-impl + session-blueprint + formal-invariants + sdk-gap + r20-stage-1-2 + r-measure-design | 接手 rust-coder 必读, 引用 §章节号 |
| **资产/SOP/词条** | 3 | tauri-assets + tauri-sop + glossary | 跨团队协同用, 引用 §资产编号 T-XXX / §词条名 |
| **路线图层** | 1 | r20-product-finalize (在 `docs/roadmap/`) | 跟 stage4 平行, 引用 §阶段号 |

### 5.2 13 reports/ 分类

| 类别 | 数量 | 文档 | SOP 引用规则 |
|---|---:|---|---|
| **SpectrAI 架构** | 1 | spectrai-architecture | 蓝图根源, 引用 §模块号 |
| **Apeireth 现状** | 11 | crate-api / platform-modules / council-7 / protocol-4 / mcp-14 / graph-pipeline / supervisor-tool-rules / session-vector-asi / asi-24dim / tauri-roadmap / ... | 接手时分析依据, 引用 §分析号 |
| **总收口** | 1 | r19-integration-wrap-up | 唯一索引, 引用 §编号 |

### 5.3 跨引用核心链路 (5 条主链)

```
1. 蓝图 (B) ──→ 3 ADR (A10/A11/A12)
2. 蓝图 (B) ──→ 6 实施指南 (G1-G6)
3. R20 路线图 (R) ──→ 5 衔接点 (R201-R205)
4. 全局架构图 (G) ──→ 10 reports (R1-R10)
5. 总收口 (W) ──→ 全部 26 份 (25 行表)
```

**§3 步骤 1 自检 + §3 步骤 3 CI** 守这 5 条主链不断。

---

## §6 风险清单

| # | 风险 | 严重度 | 缓解 | 触发 |
|---|---|---|---|---|
| **R-001** | 27 份文档没人维护, 3 个月后隐形资产化 | 🔴 高 | §3 步骤 2 周会议 + 步骤 4 季度审计 | 季度审计草拟 > 30 天 |
| **R-002** | 跨文档引用失效 (改了路径没改引用) | 🟡 中 | §3 步骤 3 CI 校验 | 任何 commit 改 docs/ |
| **R-003** | 决策清单跟拍板记录不一致 | 🔴 高 | §3 步骤 1 自检 + 步骤 2 周会议 | 改 §决策章节 |
| **R-004** | 主人待拍板项堆积 (> 10 项超 30 天) | 🔴 高 | §3 步骤 2 周会议议题 + §2.5 规则 | 季度审计待拍板计数 |
| **R-005** | CI workflow 加错, 阻塞 commit | 🟡 中 | §4 伪 YAML 留 `set -e` + 分步 check | CI 第一次 fail |
| **R-006** | Hermes 团队未拍板 docs-stage4-check, 5 步 SOP 缺 CI 守门 | 🟡 中 | §4 写完, 等 Hermes 拍板, 临时用 §3 步骤 1 手动守门 | 提交时 §3 步骤 1 必跑 |
| **R-007** | sub-agent 派 27 份, 口径漂移 (e.g. "11 文件" vs "14000 LOC" vs "T13 BLOCK") | 🟡 中 | §3 步骤 2 周会议 + §3 步骤 4 季度审计 baseline 对照 | 周会议 grep "TBD" |
| **R-008** | 文档跟代码漂移 (改 src/ 忘改文档) | 🔴 高 | APEIRETH-CONVENTIONS §6 commit 规范要求 `docs:` 前缀 commit, CI 检查 commit message 关联 | 改 src/ 必带 docs/ 同步 |
| **R-009** | 季度审计被跳过 (3 个月才 1 次, 主人忙就忘) | 🟡 中 | §3 步骤 4 写进团队 lead OKR | 季度第一周 |
| **R-010** | 5 步 SOP 本身成隐形资产 (写了不执行) | 🔴 高 | §3 步骤 2 周会议必跑 §3 步骤 1 + 2 步; §3 步骤 3 CI 自动跑 | 第一次周会议不出周报告 |

---

## §7 不修改承诺 (跟 ADR-0011 §不修改承诺一致)

| ❌ 不修改 | 原因 |
|---|---|
| 阶段 1+2+3+4+5 LOCKED 文档 | 主人明确沉淀 |
| v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
| 阶段 4 核心文档 LOCKED (`6ca80776`) | 蓝图 §10 已锁 |
| 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 实施时再引用 |
| v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | 主 AI 团队已 LOCKED |
| R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | 主人 2026-07-31 明确不动 |
| APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md (顶层 3 文件) | 不动 |
| START-CONSTRUCTION.md | 不动 |
| `apeireth-legacy/` (R17 finalize 后归档) | 不删 |
| workspace version 1.0.0 (Cargo.toml, semver 严格) | 不动 |
| 现有 ADR 0001~0009 | 不动 |
| **本 SOP §3 步骤 3 CI workflow Yaml** | Hermes 团队 LOCKED, 拍板后由 Hermes 写, 本 SOP 不动 |

> 8 项详见 docs/stage4/8-locked-unified-2026-08-05.md §2 (本指南统一版)

**本 SOP 也不修改**: ✅ 27 份文档全部在 M 标记文件外 (docs/ + reports/), 不碰任何 LOCKED + 不碰任何 Cargo.toml / 源码 / CI YAML。

---

## §8 6 哲学 anchor 穿透 (按 APEIRETH-CONVENTIONS §9)

| 锚 | 来源 | 本 SOP 落地 |
|---|---|---|
| **S-1** 主 22:33 | 6 anchor ASI 完整性 | 27 份文档 + 5 步 SOP + 1 个 CI workflow = ASI 完整性的工程化维护 (5 阶段 R19+ 路线 + 5 阶段 R20 路线服务 ASI 北极星) |
| **S-2** 主 17:43 | 6 anchor 实事求是 | 5 步 SOP 每步 grep 真实状态 (`🔍 草拟` / `Commit: <commit 时回填>` / `TBD` / `LOCKED`), 不假装文档已同步 |
| **O-5** 主 17:58 | 6 anchor 不假装 | 文档同步是 P0 急救 (隐形资产危害大, 5 步 SOP + 季度审计 = 守门), 5 重 CI 守门 (`fmt` + `clippy` + `deny` + `r-measure` + `test`) 不许绕过 |
| **O-2** 主 19:33 | 6 anchor 走在前人经验上 | 5 步 SOP 分类清晰 (commit 自检 / 周会议 / CI / 季度 / 版本号), 复用 Hermes R18 8 workflows 不重造 |
| **O-3** 主 23:44 | 6 anchor 决策清单 | 10 项主人待拍板清单 (§2.5) + 8 项不修改承诺 (§7) + 10 项风险清单 (§6) + 5 步 SOP (§3) = 4 类 33 项决策清单 |
| **O-4** 主 00:56 | 6 anchor 任何人都能接手 | 5 步 SOP + §4 CI 伪 YAML + §5 文档地图 + §6 风险清单 + §7 不修改承诺 + §9 关联文档 = 任何接手者查表即可执行 |

---

## §9 关联文档

**总收口 + 索引**:
- [R19+ 集成收口报告 (24 份文档地图)](file:///.minimax-agent-cn/spectrai/reports/r19-integration-wrap-up-2026-08-05.md) — 本 SOP 的总索引
- [SpectrAI 集成蓝图 (R19+ 根)](file:///.openclaw/workspace/promethean/Apeireth-rust/docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md)

**规范引用**:
- `APEIRETH-CONVENTIONS.md` §0.1 (Document-Meta 格式) + §6 (commit 规范) + §9 (6 锚穿透) + §10 (不修改承诺) + §11 (R11 baseline 3 值)
- `docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md` §不修改承诺 (11 项)

**Hermes R18 协同** (5 协同点):
- Hermes R18 阶段 0 工程基线 (`workspace.lints` + `deny.toml` + `rustfmt` + `clippy`)
- Hermes R18 阶段 1 CI 配套 (`cargo-deny` + `rust-lint` workflows, e84c9068 commit)
- Hermes R18 阶段 2 集成测试 (122 tests for 12 product crates)
- Hermes R18 阶段 3 miri + coverage + rustdoc + SECURITY
- Hermes R19 T10 clippy `-D warnings` 真正生效

**R20 衔接** (5 衔接点):
- R20 阶段 1.1 TUI 9 命令深化 (per r20-stage-1-2 §2.1)
- R20 阶段 1.2 apeireth-team-lead 公开 API (per team-lead-implementation-guide)
- R20 阶段 1.3 apeireth-mcp::team 14 工具 (per ADR-0010 + mcp-14-tool-analysis)
- R20 阶段 1.4 mid-task bug 3 处修法 (per session-blueprint §4)
- R20 阶段 1.5 集成 + R-Measure 守门 (per r-measure-verification-design §3)

**27 份 R19+ 集成文档** (按 §5 分类, 完整清单见总收口 §5):
- 14 docs/ (1 蓝图 + 3 ADR + 6 实施蓝图 + 1 R20 路线 + 3 资产/SOP/词条)
- 13 reports/ (1 SpectrAI 架构 + 11 Apeireth 现状 + 1 总收口)

**本 SOP 引用**:
- `docs/stage4/docs-maintenance-sop-2026-08-05.md` (本文件, Manual-Rev-A)

---

_文档维护 SOP 草拟 (Mavis / software-architect + technical_writer 角色) — 27 份文档 + 5 步维护 SOP + 1 个 docs-stage4-check CI workflow 设计稿 + 6 哲学 anchor 穿透 + 8 项不修改承诺 + 10 项风险清单 + 5 条主链引用._

_等 Mavis 拍板后由 architect2 在 R20 阶段 1 落地 (per `r20-stage-1-2-implementation-2026-08-05.md` §2.1), Hermes 团队 lead 拍板后写 `.github/workflows/docs-stage4-check.yml`, 团队 lead 接手周会议 + 季度审计._

_主人 2026-08-05 拍板后, 5 步 SOP 跟 Hermes R18 8 workflows 协同, R-Measure baseline 3 值守门, 27 份文档 + 1 个 SOP + 1 个 CI workflow = R19+ 集成完整维护机制._
