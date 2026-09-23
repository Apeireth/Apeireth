# 交接文档 — 18-crate 文档对账 + CI 安全守门修复 (2026-09-23)

```yaml
[Document-Meta]
Document:        docs/04-internal/handoff-18crate-ci-fix-2026-09-23.md
Version:         Handoff-Rev-1.0
Last-Modified:   2026-09-23
Status:          🟢 活跃 (待接手 AI 审核)
```

> **给谁看**：接手审核本次会话工作、并继续推进后续项的 AI。
> **口径**：本文只写**实测过**的事实；推断一律标"推断"，没验的一律标"未验"。所有结论都附可复现命令。
> **本会话范围**：① 把 18-crate 口径的文档全部对齐；② 修 CI 上红掉的两个安全守门 job。**没有**改任何产品源码逻辑。

> **⚠️ 与并行交接包的关系（先读这条）**：本仓库另有一份**并行的**交接包
> `docs/04-internal/engineering-review-handoff-2026-10-06.md`（他人撰写，commits `909d67e9` / `2a43f9fc`），
> 覆盖**工程独立审核 + 8 个工作包（W1–W8）**。两份**互补、不重复**，审核方请都读：
>
> | | 那份（`engineering-review-handoff-2026-10-06.md`）| 本份（本文）|
> |---|---|---|
> | 主题 | 能力差距审计、v1×v2 对账、后续工作包 W1–W8 | **本会话改了什么、怎么证伪、安全/deps 侧还剩什么** |
> | 含 | 复核矩阵 A–F、实测基线（`3406 passed / 0 failed / 19 ignored` @ `e4f2f451`）、生产接线计数、真缺口、待主人拍板 5 项 | 两个提交的逐项证据、CI run id、Dependabot 实取清单、`src-tauri`/npm/legacy 未动项、审计清单 |
> | 不含 | **依赖与 CI 安全守门的工作包**（W1–W8 里没有）| 能力差距与产品排期 |
>
> **→ 本文 §4 的四项（§4.1–§4.4）应视为对那份交接包的增补**（安全/依赖侧的工作包）。
> 两份若出现冲突，以**命令实测**为准，并把结果回写两份。

---

## 0. 硬边界（先读，别踩）

1. **`frontend/companion-desktop/**`（含独立 workspace `src-tauri/`）正在施工**——原主人明确要求不得干扰。本会话对它**零改动**（包括它自己的 `Cargo.lock`，其中 `rustls` 仍是 **0.23.43**，见 §4.1）。要改必须先问主人。
2. **`research/**` 有非本会话的未提交改动**（`research/.gitignore`、`research/llm_judge/src/main.rs`，后者 +467 行）。不要提交、不要回滚、不要当自己的改动写进报告。
3. **`legacy/**`、`docs/archive/**`、`reports/**`、`**/*.backup-*.md` 是历史证据**：不动正文，必要时按仓内约定加"对账注"（见 §7）。
4. **LOCKED 项 0 触碰**：9 哲学锚 / 13 键判别词汇表 / 3 项不可变脊柱（Self-Disable、L0 HA、13 键 verdict cache 语义）/ `workspace.version` / R11 baseline 3 值。
5. **本仓库多 AI 并发**：本会话期间观察到其他协作者在我前后各推了提交（`5d9e62ca`、`e4f2f451` 等）。**push 前必须 fetch + 检查分叉**；你的 commit 可能被别人的提交插在前面（我的 `03e5087c` 的父提交就是别人的 `5d9e62ca`，不是我上一个提交）。
6. 提交前有 **pre-commit 密钥扫描钩子**（`.git/hooks/pre-commit` → `scripts/secret-scan.ps1 -Mode scan-staged`），扫 staged 文件，只返回 0/1。

---

## 1. 我做了什么（两个提交，均已在 `origin/main`）

### 提交 A — `5b0a3062` `docs: 18-crate 口径对账——全量文档同步新增 apeireth-guard`

**27 文件，+85/−66。** 起因：根 `Cargo.toml` 实测 18 个 members（foundation 6 / engine 8 / capabilities 1 / adapters 3），而文档分别停在 16 / 17 / 13，且安全组件 `apeireth-guard` 在权威索引里完全没有条目。

改动分类：

| 类别 | 文件 |
|---|---|
| 结构与索引 | `ARCHITECTURE.md`（`seventeen`→`eighteen`、Engine 组补 guard、ownership 表新增 guard 行）、`docs/01-architecture/architecture.md`、`docs/03-reference/crates.md`（"16 members"→18，补 `apeireth-runtime-assembly` 与 `apeireth-guard` 两行，并注明 guard 与 v1 `legacy/donor` 的 PII guard **同名不同物**）|
| 公开面 | `README.md`、`README.zh-CN.md`（徽章 / 冷启动基准行 / 章节标题 / crate 树 / 速查表；中文版原写"16 个核心 Crate"）、`INSTALL.md` |
| 交接与维护 | `docs/04-internal/HANDOFF-NOTES.md`（§1 简介 + §4 标题/拓扑树 engine 7→8 + guard 行 + 2026-10-06 注补充 17→18）、`docs/04-internal/maintenance-guide.md`（模块地图 Engine 8 + guard 行 + 依赖 DAG 补 `guard -> core, governance, protocol`（取自 `crates/engine/guard/Cargo.toml`）+ §4.3/§9/页脚）、`docs/03-reference/team-handover-reference.md`（标题 + TOC 锚点同步，防断链）、`docs/04-internal/TO-NEW-TEAM.md`、`docs/04-internal/ENGINEER-MANIFESTO.md`、`docs/04-internal/v2-reference-handbook-2026-08-28.md`、`docs/04-internal/design-intent.md`、`docs/04-internal/plugin-authoring-guide.md` |
| 政策与入口 | `SECURITY.md`（18-crate 范围 + 新增 `apeireth-guard` 安全边界条目 + Last updated 标注"测试口径未重跑"）、`CONTRIBUTING.md`、`ROADMAP.md`（新增"2026-10-06 对账追加" + §3 表 Workspace 行）、`Makefile`（help 文案）、`deploy/README.md`、`docs/development/5-min-quickstart.md`、`docs/02-guides/v2-frontend-quickstart.md`、`docs/02-guides/v2-gateway-frontend-integration-spec.md`、`docs/01-architecture/vision.md`、`docs/01-architecture/engineering-report.md`（只改"当前基线"指针）、`docs/README.md`（当前基线 13→18，旧 2026-08-27 快照下移为历史行）|
| 诚实标注 | `docs/02-guides/user-manual.md`：**只加**一条 2026-10-06 对账横幅（正文写于 `v2.0.0-alpha.1` 13-crate 时代，"记忆端到端管线不在工作区"等能力判断已被后续批次取代 → 指向 `live-verification-ledger.md` #26），**没有**把过期正文假装成现状 |

**有意未改**（保留历史属性，符合 `docs/README.md` 的既有约定）：`CHANGELOG.md`、`docs/archive/**`、`reports/**`、`legacy/**`、`commit-msg-*.md`、`README.backup-v2.md`、`README.zh-CN.backup-v2.md`、`docs/04-internal/round-*.md`、`r7-*/r11-*/r12-*/r14-*`、`FINAL-HANDOFF-V2.0.0-RC.1.md`、`v2.0.0-release-path*.md`、`v2-master-lineage-*.md`、`v2-line-by-line-verification-*.md`、`docs/03-reference/vision-alignment-whitepaper.md`（报告代号 `…20260830`，属日期性报告）。

### 提交 B — `03e5087c` `fix(ci): 修好 cargo-audit 的 SARIF 守门 + 升 rustls 解 RUSTSEC-2026-0285`

**2 文件，+13/−8。**

- `.github/workflows/cargo-audit.yml`：SARIF 转换脚本 `for v in (vulns.get("found") or []):` → `for v in (vulns.get("list") or []):`；`advisory.get("severity", 1)` → `(advisory.get("severity") or 1)`；订正原注释里"`or []` 兜底"的误诊。
- `Cargo.lock`：`cargo update -p rustls` → `rustls 0.23.43 → 0.23.45`、`rustls-webpki 0.103.13 → 0.103.15`（仅 2 包，语义兼容，0 源码改动）。

---

## 2. 证据与复现（附可证伪点）

### 2.1 18-crate 口径

```powershell
# members 实测（预期 18）
(Select-String -Path Cargo.toml -Pattern 'crates/(foundation|engine|capabilities|adapters)/' | Measure-Object).Count
# 提交内容
git show 5b0a3062 --stat
# 残留旧口径扫描（排除历史目录；预期只剩"带对账注/历史属性"的行）
$t = @(Get-ChildItem -File -Filter *.md) + @(Get-ChildItem docs/02-guides,docs/03-reference,docs/04-internal,docs/01-architecture,docs/development,docs/integration -File -Filter *.md)
Select-String -Path $t.FullName -Pattern '1[0-9][- ]?Crate|1[0-9]\s*个\s*crate|16 members'
```

**可证伪点**：若你发现某个**活跃**文档仍写旧数而我没改，即本文档失守。判断"活跃 vs 历史"的标准是：文件头是否有 `现状 (2026-08-27)` / `⚠️ 2026-09-05 对账批标注` 之类的历史横幅，或是否位于 `archive/legacy/reports/` 或带日期的批次文件名。

### 2.2 rustls 漏洞（真实存在，已修）

- **advisory**：`RUSTSEC-2026-0285`（别名 `GHSA-2mjx-qc3c-rqvc`），2026-09-14 发布，medium 5.3，`CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N`，修于 `>= 0.23.45`。
- **内容**：TLS 1.3 握手消息被跨加密级别边界接受（明文 `EncryptedExtensions` 可与 `ServerHello` 同 record 通过）。advisory 原文明确：握手 transcript 仍被认证，**网络位置攻击者无法借此改写或完成握手**，实际影响是"本该加密的握手消息可以明文发送而不被拒"。与 Go 的 `GO-2026-4340` / `CVE-2025-61730` 同源。
- **在本仓的路径**：`reqwest 0.12.28 → hyper-rustls 0.27.9 → rustls`，使用方含 `apeireth-provider`、`apeireth-perception`（真 HTTPS 调 LLM API）与 `apeireth-sdk`（`Cargo.toml` 声明 `reqwest` + `rustls-tls`）。

```powershell
cargo tree -i rustls --locked          # 预期 rustls v0.23.45
cargo audit --no-fetch                 # 预期 0 漏洞；仅剩 chacha20 0.10.1 yanked 警告
cargo deny check advisories            # 预期 advisories ok
cargo metadata --locked                # 预期 exit 0
cargo check -p apeireth-provider --locked   # 预期 exit 0（本会话实测 13.12s）
```

> 注：`cargo audit` 需要 advisory DB。本会话把 `~/.cargo/advisory-db` 从 `42ead52 (2026-08-20)` 更新到 `f7dc4b2 (2026-09-22)`——这是**仓库外**唯一被我改动的状态。离线复核用 `cargo audit --no-fetch`。

### 2.3 SARIF 脚本（CI 红的真正原因）

旧脚本在**有漏洞**时必炸：`vulnerabilities.found` 是布尔，`true or []` 得到 `true`，迭代 bool → `TypeError`。0 漏洞时 `false or []` → `[]` 正常，所以这条守门**只在"全绿"状态下工作过**。本会话用真实报告双向实测：

| 脚本 | 输入 | 结果 |
|---|---|---|
| 旧（原样） | 含 rustls 漏洞的 `--json` 报告 | `TypeError: 'bool' object is not iterable`，exit 1（与 CI 日志逐字一致）|
| 修复后 | 同上 | exit 0，findings=1 / rules=1 / executionSuccessful=false |
| 修复后 | 0 漏洞报告 | exit 0，findings=0 / executionSuccessful=true |

```powershell
# 复现（CI 同款命令 + ignore 清单见 workflow）
cargo audit --json --deny warnings --ignore RUSTSEC-2024-0411 ... > $env:TEMP\audit.json
(Get-Content $env:TEMP\audit.json -Raw | ConvertFrom-Json).vulnerabilities | Format-List   # found=Boolean, list=Object[]
```

**可证伪点**：若你怀疑 `found` 是数组而非布尔，上面最后一行会直接打印类型；CI 侧证据是 run `35835024572` 步骤 7 的 traceback。

### 2.4 CI 结论（run id，可用 `gh` 或 API 复核）

| 提交 | Cargo audit | Cargo deny |
|---|---|---|
| `f1a1082c` | failure `35824477673` | failure `35824477856` |
| `b1dd6f6f` | failure `35824521182` | failure `35824521204`（日志实锤 `RUSTSEC-2026-0285`）|
| `5d9e62ca` | failure `35835024572`（SARIF 脚本 TypeError）| failure `35835024436` |
| **`e4f2f451`（含我的修复）** | **success `35835417167`** | **success `35835417260`** |

**重要诚实点**：`03e5087c` **自身**的 run 没出现在列表里（很快被后续提交的 run 取代）。"修复生效"的证据来自**包含它作为祖先**的 `e4f2f451`，不是它自己。若审核要求严格归因，可重跑 `03e5087c` 的 workflow（`workflow_dispatch`）。

---

## 3. 当前状态（写本文时）

- `main` 本地与 `origin/main` 同步（`git rev-list --left-right --count origin/main...HEAD` → `0 0`）。**`main` 推进极快**：我推送 `03e5087c` 后，`origin/main` 在我写本文的这段时间里又经过了 `e4f2f451` → `2a43f9fc`（均为他人提交）。**不要信本文里的任何 HEAD 值**，用 `git log -1 --format='%h %s' origin/main` 现取。
- 工作区未提交改动**只有** `research/.gitignore`、`research/llm_judge/src/main.rs`（非本会话）。
- **Dependabot 开放警报 5 条**（用 API 实取，`state=open`）：

| 严重度 | 包 | 清单 | 说明 |
|---|---|---|---|
| medium | `jsonwebtoken` | `legacy/donor/apeireth-livekit/Cargo.toml` | GHSA-h395-gr6q-cpjc 类型混淆→潜在授权绕过；清单 pin `"9.3"`，vuln `< 10.3.0` |
| medium | `glib` | `frontend/companion-desktop/src-tauri/Cargo.lock` | GHSA-wrw7-89jp-8q8g（unsound）；仅 Linux/GTK 构建路径 |
| medium | `devalue` | `frontend/companion-desktop/pnpm-lock.yaml` | GHSA-9rgm-9g3h-6x36 DoS |
| low | `git2` | `legacy/archived/apeireth-integration-r20-stage4/Cargo.lock` | GHSA-j39j-6gw9-jw6h |
| low | `lru` | 同上 | GHSA-rhfx-m35p-ff5j |

> **`rustls`(RUSTSEC-2026-0285) 不在这 5 条里**——Dependabot 的 Rust 数据尚未同步该 advisory。**我最初把它推断为其中 2 条，是错的，已用 API 纠正。** 见 §6。

### 环境事实（会变，用前先验）

- **代理**：主人本机代理端口当时为 **7900**（此前 7897）。github.com 直连超时；仓内 `http.proxy` 被设为**空字符串**，所以 push/fetch 需要一次性参数：
  `git -c http.proxy=http://127.0.0.1:7900 -c https.proxy=http://127.0.0.1:7900 push origin main`
- **token**：**不在环境变量里**（只有 `CLAUDE_CODE_GIT_BASH_PATH`、`GIT_PAGER`）。它由 `credential.helper = .git/gh-credential.ps1` 提供，可用
  `("protocol=https`nhost=github.com`n`n" | git credential fill)` 静默取用（`gho_` 前缀，40 字符）。**不要把 token 写进任何文件或输出。**
- 远端：`origin` = `https://github.com/Apeireth/Apeireth.git`（本会话只推它）。另一个远端 `jimmy` = `https://github.com/Jimmyxiao2009/Apeireth-rust.git` —— 本会话未推；并行交接包称它是**死指针**（`Repository not found`），我**未复核**，删除与否见那份的"待主人拍板 #4"。

---

## 4. 待办（按价值排序）

### 4.1 `src-tauri` 的 rustls（产品面，需主人同意）
`frontend/companion-desktop/src-tauri/Cargo.lock` 仍是 `rustls 0.23.43`（同一 advisory，530 依赖的独立 workspace，随桌面安装包出货）。
命令：在该目录 `cargo update -p rustls`（dry-run 实测只动 1 个包）。**属施工区，先问主人。**
（同锁文件另有 `glib 0.18.5` unsound + 6 项 unmaintained，仅 Linux/GTK 路径。）

### 4.2 前端 npm `devalue`
`frontend/companion-desktop/pnpm-lock.yaml`（锁文件 2026-08-19，`package.json` 2026-09-01，可能已滞后）。修法：`pnpm update devalue`（其传递来源是 svelte）。实际风险低（DoS，解析自家构建产物）。**同样在施工区。**

### 4.3 `legacy/` 的 3 条警报处理（建议，未做）
`legacy/**` 已被 workspace exclude、不参与构建，但**仍被 git 跟踪**，所以永久挂在警报页。三选一：① 在 GitHub 的 dependency graph excluded paths 里排除 `legacy/**`；② 不再跟踪这些 legacy 锁文件；③ dismiss（理由 not affected）。`dependabot.yml` 只管版本更新 PR，管不了这个。

### 4.4 `deny.toml` 的过期 ignore + 三处清单漂移（低风险清理）
- `deny.toml` 的 `[advisories] ignore` 有 10 条 gtk 系，`cargo deny` 现在对**全部 10 条**报 `advisory-not-detected: no crate matched`（当前锁文件里没有 gtk crate，它只存在于 legacy）。
- 三处清单不一致：`audit.toml` 20 条 / CI 命令行 21 条（多 `RUSTSEC-2026-0217`）/ `deny.toml` 10 条。建议收敛为单一来源（例如 CI 读 `audit.toml`）。

### 4.5 文档里两个未经我验证的旧数字（需重跑才能改）
- **测试数 `3120`**：仍出现在 `README.md:388`、`README.zh-CN.md`（badge、:73、:387）、`INSTALL.md:177`、`ROADMAP.md:71`、`ENGINEER-MANIFESTO.md`、`maintenance-guide.md:148`、`TO-NEW-TEAM.md` 等处（其余命中属历史批次文档或对账注，按约定不改）。本会话**没有**把它改小或改大——因为它需要先跑全量 `cargo test --workspace` 才有真数，而我**未跑**。`SECURITY.md:75` 那处是我刻意加的"维持 2026-09-05 口径、未重跑"说明。
  - **候选真数（他人测量，我未复核）**：并行交接包 `engineering-review-handoff-2026-10-06.md` §3.1 记录在 `e4f2f451` 上实测为 **`129 suites / 3406 passed / 0 failed / 19 ignored`**。改文档前请自己跑一遍确认，并按仓内约定回写 `live-verification-ledger.md`。
- **`Makefile` help 横幅**里的 `main @ d6910cf7, tag v2.0.0-alpha.1`（`Makefile:34`）——同样是过期快照，未改。
- 另：`chacha20@0.10.1` 的 yanked 警告会让 CI 里 `cargo audit --deny warnings` 退出码为 1，但被 workflow 中的 `|| echo` 吞掉、不决定 job 成败（**保持原行为，我未改**）。

---

## 5. 审计清单（claim → 如何证伪）

| # | 我的 claim | 证伪方法 |
|---|---|---|
| 1 | workspace 是 18 个 members | `Cargo.toml` grep 计数（§2.1）|
| 2 | 18-crate 对账覆盖了所有活跃文档 | 跑 §2.1 的扫描命令，逐个判断命中是否为历史文件 |
| 3 | `5b0a3062` 只改文档 + `Cargo.toml` description，无 Rust 源码 | `git show 5b0a3062 --stat`，看是否含 `.rs` |
| 4 | `03e5087c` 只改 workflow + `Cargo.lock` | `git show 03e5087c` |
| 5 | rustls 漏洞已消失 | `cargo audit --no-fetch` / `cargo deny check advisories` |
| 6 | 新 rustls 能编译 | `cargo check -p apeireth-provider --locked` |
| 7 | 旧 SARIF 脚本确有 `bool` bug | 用 `found` 版本对含漏洞报告跑一次（§2.3）|
| 8 | CI 两个 job 已绿 | 查 run `35835417167` / `35835417260`，或对任意新提交看 CI |
| 9 | Dependabot 5 条清单 | `GET /repos/Apeireth/Apeireth/dependabot/alerts?state=open` |
| 10 | 我没碰前端与 research | `git status --short`、`git log --oneline -- frontend/`（我无提交）|
| 11 | 前端锁文件与 src-tauri 未被本会话改动 | `git log -1 -- frontend/companion-desktop/src-tauri/Cargo.lock` 应指向非我的提交 |

---

## 6. 诚实边界（我没做的 / 没验的 / 做错并已纠正的）

**未做 / 未验**
1. **未跑全量 `cargo test --workspace`**（也未跑 clippy/fmt）。因此文档里的 `3120 passed` 仍是 2026-09-05 的旧值，我既没改也没验；`03e5087c` 的"不破坏功能"只由 `cargo check -p apeireth-provider --locked` + `cargo metadata --locked` 支撑。
2. 未验证 `cargo update -p rustls` 对**运行时行为**的任何影响（仅验证解析、编译、审计三项）。
3. 未触碰 `src-tauri`、前端 npm、`legacy/`（§4.1–4.3 全部保持原样）。
4. 未验证 macOS / Linux 上的任何东西（本机是 Windows）。
5. 未逐条核对 Dependabot 页面显示（需登录）；5 条清单来自 API，不是页面。

**我出错并已纠正的**
1. **推断错误（已纠正）**：最初把 Dependabot 的"3 moderate + 2 low"推断为 `rustls×2 + devalue + h2 + glib`。用 API 实取后真相是 `jsonwebtoken / glib / devalue`（medium）+ `git2 / lru`（low）——**`rustls` 根本不在其中**。当时已标注是推断，但仍应视为一次"叙事先于实测"，接手方以此为准。
2. 本会话早期的另一处判断也被自己的实测推翻：曾以为 rustls 只出现在 SDK 的 stub 路径，实际在 `provider`/`perception` 的**生产 HTTPS 路径**上（§2.2）。

**我在仓库外改动的状态（唯一一处）**
- `~/.cargo/advisory-db`：`42ead52 (2026-08-20)` → `f7dc4b2 (2026-09-22)`。不影响仓库，但会影响后续本地 audit 结果的可比性。

---

## 7. 仓内约定速查（避免踩雷）

- **0 装 PASS（O-5 锚）**：未实现必须显式标注（`trait 口已备未接` / 显式 stub），绝不静默；不写"完成"于未实测项。
- **文档对账约定**：带日期的批次文档**正文不改**，只在顶部加对账注/历史横幅；`docs/README.md` 的 `## 当前基线` 才写"现况"。
- **真值源**：`docs/04-internal/live-verification-ledger.md`（"什么测过 / 什么没测"的唯一权威，写"已验证"前先查它）、`docs/04-internal/HANDOFF-NOTES.md`（接手入口，顶部按日期叠加对账注）、`docs/03-reference/crates.md`（crate 索引）、根 `ARCHITECTURE.md`（结构契约）。
- **提交信息**：中文三段（为什么 / 做了什么 / 测试结果）；scope 形如 `docs:` / `fix(ci):` / `crate:<name>`；`git status` 核对只含自己的文件再提交。
- **共享文件**（`Cargo.toml` / `ARCHITECTURE.md` / `ROADMAP.md` / `lib.rs`）改动前先确认没有并发改动者。

---

_本文由本会话主代理撰写，事实均有命令支撑；不确定处已显式标注。审核发现问题请直接在本文追加"审核注"（不要改我的原文），以便形成可追溯的修正链。_
