# 施工者主手册（进场 → 施工 → 收工的一站式工序卡）

```text
[Document-Meta]
Document:        00-construction-handbook.md
Version:         1.0（经验总装批：五矿 + 宪章 + 手册族 → 工序卡 + 环境事实卡 + 模板库导航）
Last-Modified:   2026-10-20
Status:          🟢 活跃（任何施工会话开工前花 5 分钟过一遍 §1 + §3）
Author:          经验总装批施工者
```

> **一事**：把分散在五矿、两份宪章、ops/kani 手册、各交接卷宗里的经验**按工序重组**——
> 你此刻处于哪一步，就照哪一节做；每个动作给「读什么 / 套什么模板 / 跑什么命令」三件套。
> **读法**：本册是**导航层与工序层**，不讲道理（为什么在五矿与宪章）；与五矿重叠处一律链接不重抄
> （[04-methodology.md](04-methodology.md) 读法注先例；产品洞察 5「重复 = 分层没说清」案的落法）。
> **上位文件**：[CONSTRUCTION-DISCIPLINE.md](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md)（施工宪章）、
> [ENGINEER-MANIFESTO.md](../ENGINEER-MANIFESTO.md)（工程总纲）；
> 对外协作者版见 [multi-ai-construction-guide.md](../../02-guides/multi-ai-construction-guide.md)。
> **机器可读短指令**：[AGENTS.md](../../../AGENTS.md)。

---

## 1. 一页工序卡（开工 → 收工）

```
①进场      读两份宪章 + 本册 §3 环境事实卡 + 声明 write_scope（动哪些文件）
            └ 模板: templates/preflight-checklist.md
②立项      查 00-backlog.md 认领想法 → 细节文件随批立 → 同文件区与他人排队
③骨架      先落盘骨架/清单再填内容（渐进落盘三律，宪章 §六.1）——猝死也留可续作产物
④施工      edit 前先读后改；占位必报（可被识别并喊破）；跨线接缝先报备再动手
⑤本批门禁  本批触碰面定向测试（只对本批负责，不当交付门）
⑥全量门禁  三件套 + 前端 + 中性扫描 + secret-scan；出红先定性再定罪（隔离复跑）
            └ 判定一律基于落盘 log，不在管道末端判（管道截断假绿，ops §5）
⑦收口      commit（O-6 三阶审查模板）→ CHANGELOG → 文档同步（数字只引不抄真源）
⑧遗嘱      交付文档带「接棒清单」四要素；中道停机必留「已完成/未完清单」
            └ 模板: templates/delivery-doc-skeleton.md / resumption-entry.md
⑨交接      跨会话/跨人接续写交接报告（结论复核矩阵 + 未验证边界 + 防复蹈教训）
            └ 模板: templates/handoff-report-skeleton.md
```

每步的「为什么」与反例：[01-collaboration-patterns.md](01-collaboration-patterns.md)（协同）、
[02-failure-anatomy.md](02-failure-anatomy.md)（死法）、[04-methodology.md](04-methodology.md) §三（门禁三级）。

---

## 2. 任务型导航（干什么 → 去哪读 / 套什么 / 跑什么）

| 你要干什么 | 先读 | 套模板 | 跑什么 |
|---|---|---|---|
| **第一次进场** | 两份宪章 + [multi-ai-construction-guide](../../02-guides/multi-ai-construction-guide.md) §一 | `preflight-checklist.md` | `git log --oneline -5` 对 HEAD 口径 |
| **改 src（任何）** | MANIFESTO §8/§10（LOCKED 5 项 + 工序） | `commit-message.md` | 三件套（见 §3 命令卡） |
| **派子代理** | MANIFESTO §6（workflow + 3 问 + 亲验义务） | `subagent-brief.md` | 派前自己先跑一遍基线 |
| **判某能力到底做没做** | [engineering-review-handoff-2026-10-06.md](../engineering-review-handoff-2026-10-06.md) §2.3（三关）+ [system-capabilities.md](../../01-architecture/system-capabilities.md) 四级口径 | — | 三关命令见本册 §4 |
| **写交付文档** | 任一份 *-delivery.md 卷宗抬头（接缝注先例） | `delivery-doc-skeleton.md` | `python scripts/check_doc_caliber.py <doc>` |
| **跨会话接续** | [handoff-w1-w5-closure](../handoff-w1-w5-closure-2026-10-10.md) §7（接手第一步） | `handoff-report-skeleton.md` | 交付方 §3 验证清单 |
| **写"已验证/已实测"** | [live-verification-ledger.md](../live-verification-ledger.md)（唯一权威，先查再写） | — | 台账行的复现命令 |
| **报测试数字** | [test-baseline.md](../../03-reference/test-baseline.md)（单一真源） | `retest-note.md`（复测注记） | 数字随行三件：命令+日期+工具链 |
| **网络/下载/git 排障** | [ops-network-playbook.md](../ops-network-playbook.md) §1/§2/§4 | — | 手册内探测命令 |
| **形式化验证（kani 面）** | [kani-verification-playbook.md](../kani-verification-playbook.md) §2（K1-K18） | §3 新增 harness 检查单 | WSL 本地回路（手册 §5） |
| **吸收外部项目** | [absorption-candidates-2026-10.md](../absorption-candidates-2026-10.md) §〇（第一律提问法） | 04-methodology 配方 3（一项目一小节） | 取材先净化（配方 4） |
| **发布/release** | [release-checklist.md](../release-checklist.md) + MANIFESTO §10（版本轴 LOCKED） | — | 发布资产校验脚本（B8 批） |
| **查在飞/待办** | [00-backlog.md](../backend-enhancement/00-backlog.md) + 债栏 D1-D27 | — | — |

---

## 3. 本机环境事实卡（会咬人的点，全部实测过）

> 来源：[handoff-w1-w5-closure](../handoff-w1-w5-closure-2026-10-10.md) §4/§6、
> [ops-network-playbook.md](../ops-network-playbook.md)、[00-backlog](../backend-enhancement/00-backlog.md) 债栏。
> 换机器即失效，以实测为准。

### 3.1 命令卡（交付前必跑，`-j 2` 口径）

```powershell
cargo test  --workspace --all-targets --locked --no-fail-fast -j 2   # 期望 0 failed（现行口径见 test-baseline，只引不抄）
cargo clippy --workspace --all-targets --locked -- -D warnings      # 期望 0 警告
cargo fmt --check                                                    # 期望 0 diff
cd frontend/companion-desktop; pnpm check; pnpm test                 # 前端两件
powershell scripts/check-neutral-terms.ps1                           # 期望 0 命中（豁免计数公开）
# 判定一律基于落盘 log：命令 > xx.log 2>&1，再 grep 落盘文件
```

### 3.2 git 与推送（本机实测纪律）

| 点 | 事实 | 出处 |
|---|---|---|
| 推送走代理 + HTTP/1.1 | `git -c http.proxy=http://127.0.0.1:7900 -c https.proxy=... -c http.version=HTTP/1.1 push origin main`；代理瞬断，重试循环（亲历 16 连败后恢复） | handoff-w1-w5 §4 |
| fetch 失败 ≠ push 失败 | 两通道独立；怀疑 push 状态时 retry push 看 error code，不凭 fetch 推断 | MANIFESTO §13 #10 |
| force push | 只许 `--force-with-lease=main:<expected-tip>`，不裸 `--force` | MANIFESTO §8.5 |
| `git add` 显式路径 | 新 `scripts/*.py` 被 `.gitignore` blanket 规则挡，需 `git add -f` | handoff-w1-w5 §4 |
| author env | commit 前设 `GIT_AUTHOR_NAME/COMMITTER_NAME`（防 fallback 到 git config 默认值）；commit-msg hook 会 BLOCK | MANIFESTO §13 #12 + §8.5 hooks |
| hooks | 一次性 `git config core.hooksPath .githooks`；commit message 必含三阶审查三段，否则 BLOCK | MANIFESTO §8.5 |
| amend 自验 | amend 后必验 tree（`git show HASH:path | grep <fix>`），不依赖 `git diff --stat`；PowerShell 里 `^{tree}` 要引号 | MANIFESTO §13 #9/#11 |
| `cargo fmt -- file` | 会格式化整个 workspace——用 `rustfmt <file>` 单文件 | MANIFESTO §8.6 |
| fmt 后 edit | fmt 重排后 Edit 会被"文件已变"挡 → 重读再贴（10+ 次规律） | handoff-w1-w5 §4 |
| 台账/ledger | 只追加不替换（append-only 红线） | handoff-w1-w5 §4 |

### 3.3 Windows 环境已知坑（债栏在账，勿误判为新 bug）

| 坑 | 现状与口径 | 出处 |
|---|---|---|
| 端口保留 WSAEACCES 10013 | `netsh` administered 保留区间 50000-50059；全量并行偶发 socket 红。**绿跑口径**：`-j 1` + `RUST_TEST_THREADS=4`；彻底修 = 环境侧删保留区间（不入仓） | 债栏 D26 |
| AppContainer 探针偶发 os error 2 | 并行负载下 1 次失败，单跑复绿；处置待 owner 口径，不掩盖 | 债栏 D25 |
| `write_atomic` tmp 名竞争 | 并行负载下 1 次失败，单跑复绿（199/0）；修法登记在账 | 债栏 D27 |
| `test-baseline.md` 含 1 个 NUL 字节 | 部分文本读取工具拒读；内容无损（UTF-8 容错解码已核），清理批顺手处理 | 本册总装批实测（2026-10-20） |
| `\\?\` 前缀 / canonical 键分叉 | 路径归一只认一把尺；悬案走示踪三选一 | ops §6.2 / 02 矿 4a |
| CRLF 吃替换 | 正则容 `\r?$`；替换后断言命中数；批量手术后配平校验 | 02 矿 4b |
| CI 绿 ≠ 本地绿 | cfg(unix) 块本地不编译；动跨平台代码后 WSL 侧过一遍 check | ops §6 |

### 3.4 跨线共树（多 AI 同树施工呼吸规则）

共享收敛点（`Cargo.lock`、装配根 `production.rs`、CLI 装配面）**不手改**，交工具收敛、`--locked` 复验；
撞见他线在途红 = 标红 + 归属线 + 报备，**零代改**（[01 矿模式 1/6/7](01-collaboration-patterns.md)）。
当前工作树如有他人脏文件（如 CHANGELOG / capabilities-matrix 的在途改动），视同他线战场，不碰不提交。

---

## 4. 判「未接线 / 未实现」三关（最容易写假账的一句话）

> 出处：[engineering-review-handoff-2026-10-06.md](../engineering-review-handoff-2026-10-06.md) §2.3
> （复核教训第三条 + C1/C3 命令形态）；[05-product-insights.md](05-product-insights.md) 洞察 2。

判「未接线」必须三关全过，缺一不得下结论：

1. **名字引用扫描**——目标名在 `runtime-assembly/src` + `cli/src` 引用计数为 0（注意「类型出现在同一文件」≠「被构造/注入」）；
2. **crate 内部互调**——「0 引用」可能是名字扫描陷阱：crate 内部互调也是生产路径（`hybrid_search` 三审推翻案）；判据是**调用链走到生产主路径**，不是名字计数；
3. **`Option<...>` 的 `Default` 值**——类型接进了装配、但 `Default = None` = 接而未通（`proactive_recall` 案），四级口径记 WIRED ✅ / DEFAULT ENABLED ❌。

配套铁律：**判决必须对源码，不对历史文档**（文档写 X、源码是 Y 时以源码为准；历史文档 🔴 清单会大面积过时）。
「未实现」判定考古要全量扫模块，不能凭印象（onering/v1 沙箱两次考古纠错案，handoff-w1-w5 §6.6）。

---

## 5. 模板库（`templates/`——复制即用的可复用资产）

| 文件 | 用途 | 口径真源（模板 = 可复制形态，口径以源为准） |
|---|---|---|
| [templates/preflight-checklist.md](templates/preflight-checklist.md) | 开工自检（环境 + 声明 + 基线） | 本册 §1/§3 |
| [templates/delivery-doc-skeleton.md](templates/delivery-doc-skeleton.md) | 批次交付文档骨架 | 各 *-delivery.md 卷宗实形 + MANIFESTO §7 |
| [templates/handoff-report-skeleton.md](templates/handoff-report-skeleton.md) | 交接报告（含结论复核矩阵） | engineering-review-handoff / handoff-w1-w5 实形 |
| [templates/commit-message.md](templates/commit-message.md) | commit message（O-6 三阶审查） | MANIFESTO §8.2（LOCKED 口径） |
| [templates/subagent-brief.md](templates/subagent-brief.md) | 派子代理 brief | MANIFESTO §6.2 |
| [templates/resumption-entry.md](templates/resumption-entry.md) | 接棒清单条目 + 复测注记 | 04-methodology §二/§四 |

新增模板纪律：一模板一文件；头部必带口径真源链接；与真源漂移即病（模板是「复制形态」，不是第二真源）。

---

## 6. 取材方法与诚实声明（0 装）

**本册取材范围（如实）**：

- **全文精读**：两份宪章、五矿（01-05）、L 轴矿脉图、ops/kani 两本手册、00-backlog、
  对外施工指南、test-baseline（容错解码）、handoff-w1-w5-closure、engineering-review-handoff 前半、
  security-hardening-delivery、governance-chain-drill、代码正史注释采样（`appcontainer.rs` 模块头）。
- **取样/转引**：FINAL-HANDOFF / HANDOFF-NOTES / A-block-o6-true-account 等 rc.1 时代卷宗、
  各 *-delivery.md 交付卷宗——经五矿转引核对，未逐字全文读。
- **未读**：docs/ 全量 842 份 md 的其余部分（spec/研究/白文/归档层）——本册不声称覆盖全部，
  只覆盖「施工经验」面；spec 面结论请走各自卷宗。

**方法**：本册是**二传手总装**——一手经验已在五矿与卷宗落档，本册只做工序重组与导航，
每条可指回出处；未一手验证的条目一律标注转引，不复述为亲历。
