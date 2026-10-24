# 清债轮验收摘要（2026-10-25）

> 给 owner 的一页纸：**做了什么、还差什么、为什么差、需要谁拍板**。
>
> **当前状态（最新）**：全部改动**已推送到 PR #94**（分支 `debt-round8`），
> 本地验证全绿；**CI 停在平台侧**（见 §四）。
> 需要你做的事只要两件：①处理 GitHub Actions 额度；②对 §五 的 6 项拍板。

---

## 一、本轮交付（全部已在 PR #94）

### A 类缺口接线（9 条修完）

| 缺口 | 内容 |
|---|---|
| **被遗忘权**（`universal_forget`） | 删用户 ⇒ 连记忆一起删（一个事务清 5 账）。**生产入口此前不可达**（只有 pool 版，而生产用 `Mutex<Connection>`） |
| **限流守门**（`rate_limit`） | 挂进生产治理链（授权之后、内容风险之前）。**默认挂载**：配额内完全放行、超限才拒 |
| **partner 持久化** | `InMemoryPartnerStore`（重启即散）→ `SqlitePartnerStore`；**顺带修 `ensure_schema()` 从不被调用**（生产会直接报 `no such table`） |
| **代码地图**（`repo_map`） | `tool.repo_map` 工具面 + 生产注册；**顺带修两处平方级热点**（152 文件 9.5s → 0.26s、全仓 1319 文件 10.8s → 2.1s） |
| **查询扩展** | 接进检索主路径，做成**召回增强**而非替换查询（打分是各项之和，替换会让同义词多的文档压过精确匹配） |
| **推理字段归一化** | 12 别名 → 前置 `<think>` 块接进 provider；**关键顺序「先归一化后判空」**，否则推理模型烧光预算会被误判成空壳响应 |
| **便携包 CLI** | 从「README 自认没发货」到**真机跑通**：`./run_apeireth.sh` 起网关 → `/health` ok → 数据落包内 `./data/`。接线时修三处硬伤（启动脚本调不存在的命令、设无人读的变量、配置默认开**明文 P2P 桩**） |
| **错误闭合词表** | `ClassifyError` 此前**零实现**；现覆盖 SDK 三族 + 跨语言 `ErrorFrame` |
| **关系里程碑** | 新增 `SqliteMilestoneStore`（关系史跨重启）+ 幂等记录面 + 接进羁绊模块（初见 / 阶段跃迁） |

### 自行发现并修掉的缺陷（不在原清单内，全部真机/集成证据）

| 缺陷 | 关键证据 |
|---|---|
| **治理审计哈希链在多写者下分叉 ⇒ 审计永久降级内存** | 现场：记录 180/181/182 的 `previous_hash` 全是同一个值 ⇒ 下次启动校验失败、审计不再持久。修法三层：强制单写者锁 + 进程内按路径单例 + 坏链隔离恢复（保留证据）。**真机验证**：坏链被隔离后审计恢复持久；并发场景第二进程如实说「已被另一个进程持有」而不污染链 |
| **守卫判定没有落盘面**（只在内存，重启即散） | `apeireth nightwatch` 自报缺口；修后真机：无账 → `0 audit`+缺口，有账 → `2 audit` + `[medium] NoDegrade: critical → info` |
| **五维记忆宣称 5 维实际只产出 4 维** | `MemoryDimension` 5 变体 vs `export_browser_entries` 4 个；已补 24h 滑动窗口 + 「维度必须可导出」守卫测试 |
| **并行工具派发生产永不发生** | 运行时支持完整但生产从不构造 `CallScheduler` ⇒ 窗口恒 1；已从**守卫能力安全注册表派生**只读白名单 + 装配级 E2E 断言 |
| **问责闭环两处断裂** | ①`egress impact` 从不读 consequence 账 ⇒ 回填后报告仍「后果未知」（闭环断在读取侧）；②`egress notify` 的补救建议基于**过期后果**（已知 `failed` 还建议「先确认结果」）。两处改为叠加同一 `ConsequenceTracker`（只升级不降级）+ 新增「超期未回填」显式缺口（24h 阈值 + 逐条 ⏰ 标记） |
| **两处错误被报成 500 + 误导建议** | 非法会话 id（应 400 `invalid_session_id`）、乐观并发冲突（应 409 `revision_conflict` + 「重读最新 revision」，不是「重启 Companion」） |
| **两条真死 API** | `StreamHandle` / `query_recent`（全仓零引用）已删；台账 §三 对账 |
| **认证/权限错误建议指向错的东西** | `auth_missing_key` 让人去配**模型密钥**（实际缺的是网关令牌）；`permission_denied` 只讲角色不足（实际可能是令牌未绑定用户）。已覆盖两种原因 |
| **主入口 `chat` 缺凭据时不给下一步** | 只回 `missing API key for provider.minimax.api_key`；现补「跑 `apeireth onboard` 或设哪些 env」。**过程中我自己第一版还写错了两个不存在的 env 名**，被 arch_guard 的 env 计数预算抓出（连注释都算） |
| **`council` 缺 LLM 时只回一行英文** | 与 `subagent` 对齐为可行动中文提示，并加守卫测试 |
| **`--help` 漏 8 个可用命令** | 帮助集中成单一真源 + **防漂移机检测试**（分发块里每个命令都必须在帮助里） |
| **备份包缺失误报「清单损坏」/ crypto 错误无路径** | 加 `NotABundle` 变体区分；crypto 错误带路径 + 下一步动作 |
| **审计锁文件退出后残留**（我自己上一批修法的副作用） | `main` 退出时显式释放（并抓到自己「两个函数各自 static」的 bug） |

### 机检守卫（目标②）

- 孤儿 API 棘轮 **双向**：新增孤儿 exit 1 + **基线条目过期也 exit 1**（此前只打印提示）；
- 尺寸基线收紧 **14 条、回收 660 行虚假余量**（最大一条 `guard/hook.rs` 1518→1093）；
- 帮助防漂移、hint 里的 env 名核对注册表、复杂度回归守卫（repo_map）、env 计数预算（本轮零新增）。

---

## 二、A 类缺口总账（原 16 条）

| 状态 | 数量 | 明细 |
|---|---|---|
| ✅ 已修（#90 已合 main） | 1 | 被遗忘权 |
| ✅ 已修（在 #94，待 CI） | 8 | 限流、partner 持久化、代码地图、查询扩展、推理适配、便携包、错误词表、关系里程碑 |
| 🔶 半修（实现补全，消费点待定） | 1 | file_fetcher（消费点在副总指挥 crate；磁盘缓存已真实现） |
| 🟡 部分接线（安全子集已上，剩余是决策项） | 1 | `continuation`：**回合级中断检测已接进 `apeireth chat`**（真机验证：正常回合零残留 / 伪崩溃快照被报出并清除；不重放、不存正文）；剩余「自动续跑」= 产品语义决策 |
| 🔁 更正为重复实现 | 1 | async_context（原判据「`ContextProjector` 是 `NoContextProjector`」前提不成立） |
| ⛔ 受上游能力阻塞 | 4 | `cross_diary`（日记不在检索路径）、`three_tier_vault`（缺 markdown 语料面）、`care_potential_field`（全仓**无** distress/fatigue 信号源）、`speech_arbiter`（全双工未装配） |
| **合计** | **16** | 9 修完 + 1 半修 + 1 部分接线 + 1 更正 + 4 阻塞 |

**外加（不在原 16 条清单内、本轮自行发现并修掉的）**：

| 缺陷 | 证据 |
|---|---|
| 守卫判定**没有落盘面**（只在内存 `VecDeque`，重启即散） | `apeireth nightwatch` 自报「audit chain 未持久化」；真机验证：无账 → 有账后报出 `[medium] NoDegrade: critical → info` |
| 五维记忆**宣称 5 维实际只产出 4 维**（`Recent` 从不产出） | `MemoryDimension` 5 变体 vs `export_browser_entries` 4 个；已补 24h 滑动窗口 + 「维度必须可导出」守卫 |
| **并行工具派发生产永不发生**（从不构造 `CallScheduler`） | 运行时支持完整但窗口恒 1；已从守卫注册表派生白名单 + 装配级 E2E 断言 |
| 两条真死 API（`StreamHandle` / `query_recent`） | 全仓零引用；已删 + 台账 §三 对账 |
| 三处**文档低估实现**（SDK HTTP 早已真接 / 生物机制实为 12-14 格 / 便携包能力） | 逐条按实测断言改写 |
| **问责闭环两处断裂**（真机测试发现） | ①`egress impact` 从不读 consequence 账 ⇒ 回填后报告仍显示「后果未知」（回填在 CLI 上毫无效果）；②`egress notify` 的补救建议基于**过期后果** ⇒ 已知 `failed` 还建议「先确认结果」。两处都改为叠加同一 `ConsequenceTracker`（只升级不降级），并新增「超期未回填」显式缺口（24h 阈值 + 逐条 ⏰ 标记） |

---

## 二·五、本阶段又追加的四项（都在本地，未推）

| 项 | 结果 |
|---|---|
| **并行工具派发接上生产** | 运行时完整支持并行派发，但生产从不构造 `CallScheduler` ⇒ 窗口恒 1 ⇒ 形同不存在。现从**守卫能力安全注册表派生**「可证明只读」白名单（不手写清单），并补登记 5 个真实只读工具；装配级 E2E 断言生产真的装上了 |
| **SDK 文档对账** | 文档说「不实接 HTTP、走 `unimplemented!()`」，实际 W5 早已真接 reqwest ⇒ **文档低估实现**，已按事实改写 |
| **生物机制文档对账** | 模块注释说「只注册承载力一格」、矩阵说 9/14，实测装配级断言 **wired=12/14**（生产已填 10 槽），两处都改对 |
| **孤儿基线缩表** | 157 → 156 → 155 → **154**（接线后不再孤儿的符号逐条移除） |

> 这几项的共同点：**都是「文档或计数与实测不符」**，方向既有夸大也有低估。
> 我按「可复核命令 + 实测输出」为准逐条对账，不以注释或旧台账为准。

## 二·六、证据补强（0 装：接线必须有集成/端到端证据，不能只有单测）

本轮把两处「只有单测」的接线补成了端到端证明：

| 接线 | 之前的证据 | 补上的证据 |
|---|---|---|
| 限流守门 | hook 级单测 + 组成契约（`hook_names` 含它） | `production_chain_rate_limit_denies_after_the_minute_quota`：从**生产构造器**出发连续派发 31 次 —— 前 30 次全放行、**第 31 次被拒**，且「超限后不得回到放行」 |
| 推理字段归一化 | `parse_response_with_reasoning` 级单测 | `reasoning_adapter_is_wired_through_the_real_complete_path`：TCP mock 厂商 + **真实 `ProviderCapability::complete()`** —— 适配关=纯结论、适配开=`<think>` 前置 |

**近全仓回归**（推前保险）：`cargo nextest run --workspace --exclude apeireth-tools-canonical
--profile ci` → **7874 tests passed / 48 skipped / 0 failed**（293 binaries）。
（唯一排除的 `tools-canonical` 因本机既有 `CARGO_BIN_EXE_sandbox-test-child` 问题跑不了，
已在干净 main 上验证是本机环境差异、非本批引入；该 crate 的 lib 735 例已单独验绿。）

## 三、需要拍板的事（owner 不在，我按纪律没自作主张）

| # | 事项 | 为什么我不做 | 建议 |
|---|---|---|---|
| 1 | **`dead_code` 全局 allow** | `Cargo.toml` 里 `dead_code='allow'`，注释写明原因是「当时只有 1 处警告，在别人正在改的文件里」⇒ 整个 workspace 的这道闸被关掉。本轮临时打开拿到 60 条线索，查到 **1 条真缺陷**（五维记忆 Recent 恒空，已修）+ 1 条冗余（已删）。**收益大**：这类「字段存了不用」正是「能力齐全但没接电」最常见的形态 | 改 `dead_code='warn'` + 基线棘轮（只减不增）。但会让 CI 立刻红 60 处，且 CI workflow 属副总指挥范围 ⇒ 需拍板 |
| 2 | `council ballot` 未持久化 | council 已在生产装配，但 `rubric::Ballot` 需要 `confidence`/`weight`，而 council 只产出 `verdict`；`Ballot::new` **全仓从未被构造**。硬接必须**编造**信号 ⇒ 按「不编造信号」停下 | 需定「一个顾问评价如何折算成一张票」（stance 从 verdict 怎么来、confidence/weight 谁给），或改 nightwatch 输入类型 |
| 3 | `care_potential_field` 的 distress/fatigue 信号源 | 全仓 `grep` 无任何 fatigue/distress 生产者。造信号 = 发明产品语义（什么算疲劳？深夜？连续失败？） | 要么定信号定义（可作为下一批独立任务），要么承认该能力需要前置研究 |
| 4 | `three_tier_vault` / `cross_diary` | 前者缺 markdown 知识库语料面，后者需要日记进入检索路径 —— 都是产品形态决策 | 若日记/知识库要成为产品能力，先定语料来源与召回语义 |
| 5 | `speech_arbiter` 的发言权仲裁 | 全双工语音未装配（`voice.duplex = not_assembled`）⇒ 无输入可仲裁 | 等语音面立项 |
| 6 | **问责闭环「后果追踪」缺自动回填** | 现状只有手动 `apeireth egress outcome`。**不能硬接**：`confirmed` 需外部证据（本地成功 ≠ 影响已确认，自动填=替外部背书）；且影响声明的 `action_id` 与 `ToolCall.id` **id 空间不对齐**，没有既有映射 | 需定「一次工具调用如何归属到一个已声明的影响」（1:1？1:N？跨会话？），才能只自动填确定的 `failed` |

---

## 四、协作状态

- 协作看板：`/home/bai/Projects/apeireth-coordination/lead-commander-board.md`（**仓库外**，已实测 git 完全看不到它：该目录及所有父目录都不是 git 仓库）。
- 与副总指挥**握手完成**：他同意「`scripts/` 债务守卫棘轮归我」、接受「改按文件划界」；已把 `file_fetcher.rs` 让给我（附约束：不改两个 fuzz 依赖的 API 签名 —— 我改了实现，没动签名）。
- 我在 §三 登记了每一件在飞工作，他的区（`capabilities/tools` 除已完的 repo_map 三文件、`adapters/gateway` 除已完五文件、`foundation/{protocol,plugin,governance}`、`fuzz`、`benches`、`.github/workflows`）我一律不碰。

---

## 五、证据（可复核命令）

```bash
cd /tmp/apeireth-r3

# 全量门禁（本轮每批都跑）
cargo fmt --all -- --check
cargo clippy --workspace --lib --locked -- -D warnings
python3 scripts/arch_guard.py --strict          # 期望 VIOLATIONS: 0
python3 scripts/audit_unwired.py --check        # 期望「基线一致（155 条）」
python3 scripts/group_unwired_baseline.py --check
python3 scripts/docs_truth.py --verify --check
python3 scripts/check_doc_caliber.py
python3 scripts/docs_index.py --check

# 真机验证 1：便携包
cargo build -p apeireth-cli --bin apeireth
./target/debug/apeireth bundle --out /tmp/b && cd /tmp/b && ./run_apeireth.sh  # /health → ok

# 真机验证 2：守卫判定落盘 → 守夜人
APEIRETH_DATA_DIR=/tmp/nw ./target/debug/apeireth nightwatch
#   无账     → 「0 audit」+ 缺口「audit chain 未持久化」
#   有账     → 「2 audit」+ `[medium] NoDegrade: critical → info`（缺口消失）

# 本地提交（未推）
git log --oneline origin/main..HEAD
```

---

## 六、一句话总结

**「能力齐全但没接电」这条线已经推到「清单上所有能独立完成的都完成了」**：
原 16 条 A 类里 9 条修完、2 条半修（消费点在别人区）、1 条更正为重复、
4 条被上游能力阻塞、1 条主动延后。
剩下 5 件**都需要 owner 或跨团队拍板**（见 §三），我没有自作主张去编信号或改共享策略。

---

## 六、当前状态与验证命令（2026-10-25 03:0x 更新）

### 6.1 仓库与 PR

```
分支 debt-round8（worktree /tmp/apeireth-r3）
PR #94（state=open，head 每次提交自动更新）
本地 == 远端（全部推送完成；代理关时用直连 + 逐 commit 推送绕过 TLS 中断）
```

### 6.2 本地验证（可复跑）

```bash
# 近全仓回归（唯一排除的 tools-canonical 因本机既有 CARGO_BIN_EXE 问题跑不了，CI 上正常）
cargo nextest run --workspace --exclude apeireth-tools-canonical --profile ci
# → 7893 tests passed / 48 skipped / 0 failed（293 binaries）

# 全量门禁
cargo fmt --all -- --check                                   # clean
cargo clippy --workspace --lib --locked -- -D warnings        # clean
python3 scripts/arch_guard.py --strict                        # VIOLATIONS: 0（无 STALE）
python3 scripts/audit_unwired.py --check                      # 153 条，无新增、无过期
python3 scripts/group_unwired_baseline.py --check             # 一致
python3 scripts/docs_truth.py --verify --check                # OK
python3 scripts/docs_index.py --check                        # 0 问题
python3 scripts/arch_doc.py --check                          # OK
python3 scripts/wiring_report.py --verify                     # OK
python3 scripts/check_doc_caliber.py                          # 0 新增
```

### 6.3 真机/端到端证据（各一条命令即可复现）

| 证据 | 复现 |
|---|---|
| 限流真的在生产链上 | `production_chain_rate_limit_denies_after_the_minute_quota`：前 30 次放行、第 31 次 `Deny` |
| 推理适配真的在 provider 路径上 | `reasoning_adapter_is_wired_through_the_real_complete_path`：mock 厂商 + 真实 `complete()` |
| 并行派发真的装上了 | `parallel_dispatch_production`（装配级）+ `Runtime::call_scheduler()` |
| 守卫判定真的落盘 | 跑一次带治理的回合后 `apeireth nightwatch` 报出 `N audit` 与真实降级事件 |
| 审计链真的抗多写者 | 同数据根先起 `gateway serve` 再跑 `apeireth session`：第二进程如实降级、**不污染链**；坏链自动隔离后审计恢复持久 |
| 问责闭环真的闭上 | `egress outcome <ID> confirmed` 后 `egress impact` 从「超期未回填=1」变「=0」，明细显示 `后果=confirmed` |
| 便携包真的能用 | `apeireth bundle --out <DIR>` → `./run_apeireth.sh` → `/health` ok → 数据落包内 `./data/` |

### 6.3.1 CI 门禁的「本地镜像」清单（2026-10-25 补齐）

此前我只跑 `clippy --lib` 与部分脚本，**比 CI 少两档**（因此漏掉过 3 处会打红测试代码的
lint）。现已在本地镜像到 CI 的全部轻量档：

| CI check | 本地命令 | 最近结果 |
|---|---|---|
| Rust lint (clippy 3 档 + fmt) | `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check` | ✅ 退出码 0（唯一排除 `tools-canonical`：本机既有 `CARGO_BIN_EXE_*` 问题，CI 正常） |
| `cargo test --doc (stable)` | `cargo test --doc --workspace` | ✅ 29 个 crate 全过、0 失败 |
| `rustdoc` | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` | ✅ 干净（注：CI 用 nightly，本地为 stable） |
| Rust tests | `cargo nextest run --workspace --exclude apeireth-tools-canonical --profile ci` | ✅ 7901 passed / 48 skipped / 0 failed |
| Docs truth / 索引 / 链接 / 豁免 | `docs_truth.py --verify --check`、`docs_index.py --check`、`docs_links.py`、`check_doc_caliber.py` | ✅ 全绿（链接：1466 文件，断链 0） |
| arch-guard / 孤儿棘轮 | `arch_guard.py --strict`、`audit_unwired.py --check`、`group_unwired_baseline.py --check` | ✅ VIOLATIONS 0（无 STALE）· 153 条无新增无过期 |
| Wiring ledger | `wiring_report.py --verify` | ✅ OK |
| Cargo deny（legacy 面） | `check_no_legacy_deps.py` | ✅ 30 成员、0 violation、0 legacy 包 |
| Ledger 验证 | `verify_ledger.py` | ✅ errors=0 warnings=0 |
| 提交树完整性 | `check_commit_tree.py` | ✅ 5681 文件完整 |
| **Guard 3.3 evaluation integrity** | `cargo run -p apeireth-guard --example export_guard_scenarios`、`py_compile`、`guard_ml/train_eval.py`、`analyze_shadow.py --self-test` | ✅ 2835 场景导出、train_eval 退出码 0、self-test ok |
| **O-6 锚 5 重守门** | 直接跑 workflow 里的 grep 不变量（见下） | ✅ 全过 |
| Self-status vocabulary | `check_self_status_vocab.py` | ✅ 报告项（退出码 0；10 处体验性词在债栏，非阻塞） |

| **cargo deny check**（必需检查） | `cargo deny check` | ✅ `advisories ok, bans ok, licenses ok, sources ok` |
| **8 硬墙守门**（必需检查） | 直接跑 workflow 的不变量（见下） | ✅ 5 项全过 |
| Guard 3.3 evaluation integrity | `export_guard_scenarios` + `guard_ml` 三段 | ✅ 2835 场景、train_eval 0、self-test ok |

**8 硬墙守门（本地实测）**：
① LOCKED 源码 logic 0 触碰（contract.toml 3 个 `[locked]` 源码，word-diff 无逻辑变更）✓
② `workspace.version` 未变（2.0.0-rc.4，无需 release 证据）✓
③ R11 baseline 三值（0.8682/0.8532/0.9063）各 1 处 ✓
④ 14 键数组 `ALL_THIRTEEN_KEYS: [PhilosophyKey; 14]` ✓
⑤ O-6 锚 5 重（另见 §6.3.1 上部）✓

**只有 CI 能跑的（本地不镜像）**：`Kani`（形式化，需专用工具链）、`Miri`（nightly，慢）、
`Fuzz`（cargo-fuzz 长跑）、`Coverage (tarpaulin)`（需生成 cobertura.xml）、
`cargo audit`（需克隆 RustSec 库；**其 advisories 面已由 `cargo deny check` 覆盖并全绿**）。
这四项的失败只会在 CI 暴露 —— 但目前它们覆盖的代码我这批都没改到关键路径
（Miri 面：未新增 `unsafe`；Kani 面：未改证明对象；fuzz 面：未改解析器）。

**O-6 锚 5 重守门（本地实测）**：`RUNTIME_ENFORCED = false` ✓ / 9 个哲学锚 ✓ /
`workspace.version = 2.0.0-rc.4` ✓ / R11 baseline 三值各 1 处 ✓ / 9 锚表头 ✓
（即 LOCKED 数据 0 触碰）。

### 6.4 已做过的真机核对清单（未发现缺陷，供验收参考）

| 面 | 核对内容 | 结论 |
|---|---|---|
| 备份恢复语义 | 导出后**改成员 + 加非成员**再恢复：成员文件回到包内**字节态**（sha256 一致）、非成员**原样保留** | 与文档契约一致 |
| 备份排除项 | 数据根放 `.env`/`creds.json`/`id_ed25519`/`client-tokens.json` → 包内**零敏感串**（`excluded=0` 是正确语义：清单式备份只收七类数据面） | 凭据不随行 |
| 体检第 4/5 节 | `backup_exclusions` 核对包内文件名；`token_bindings` 对**故意写坏的 JSON** 正确报「畸形、风险高」（fail-closed） | 非空转 |
| 密钥创建安全 | `crypto wrap` 重复执行到同一 `--keys` **拒绝覆盖**（「密钥文件已存在, 不覆盖」），原口令仍可解 | 不会毁掉已有密钥 |
| 用户管理全链 | owner 令牌增/删/改/查可用；**member 令牌被拒**（owner only）；**末 owner 不可降级**；删除后列表即时更新 | 权限门正确 |
| 鉴权链 | `APEIRETH_GATEWAY_TOKENS` + `user add --clients` → 带令牌 `/v1/panel/users` = 200 + owner 视图；伪令牌 401；无令牌 401；逃生阀与令牌表**互斥拒启动** | 三态正确 |
| 非回环绑定 | `--bind 0.0.0.0` 无令牌 ⇒ **拒启动**（并给三条出路）；有令牌 ⇒ 启动并**告警暴露面** | fail-closed |
| `lessons` 证据约束 | 经网关真造 episode 后 `lessons add --episode <id>` 成功（候选态，如实说「还不是学到」）；**不存在的出处直接拒** | 硬约束生效 |
| `trait` 锁定 | `lock/unlock` 幂等、落盘 `trait-locks.json`、周报同步显示锁定态 | 一致 |
| 首次引导 | `--tier casual` 交互走完落盘 `onboarding.json`、`--show` 读数准确；`--tier pro` **有意不写档案**；`--reset` 正常；EOF 中断如实报且不写半份 | 三档语义正确 |
| 网关长连接 | SSE `/v1/apeireth/events` = 200 + `text/event-stream` + 首帧；presence / runtime snapshot 正常 | 正常 |
| 守护形态 | `gateway serve --pid-file` 的 slot 文件（key=value，端口可达性判活）**正常关闭即释放** | 不双开、不残留 |
| `dream` 日记 | 按日 JSON 落盘（source/body/timestamp），0 素材时如实写「0 项轨迹」 | 输出可用 |
| **管理面 HTTP** | `POST /v1/admin/users`→201 · `POST /v1/admin/domains`→201 · `PATCH /v1/admin/users/:id`→200（非法角色→400 冻结词表、不存在→404）· `DELETE`→200 且响应含 **`memory_forgotten:true`**（**被遗忘权级联经 HTTP 面生效**）· 重复创建→409 · 删末 owner→409 · `POST /v1/admin/shutdown`→200 且进程**优雅退出** · 无令牌→401 | 全链正确 |
| **TUI 端到端** | 用 `script` 分配伪终端（`stty rows 40 cols 120`）真跑 `apeireth-tui` 连真实网关：渲染出「会话/对话/记忆账本/上下文波形/用量」等面板，**「后端已连接: http://…」**，空数据处如实标「未探到 / 不猜数」；两次 Ctrl-C 干净退出（exit 0） | 渲染 + 连接均正常 |
| TUI 非 TTY | `stdin`/`stdout` 非终端时**前置拒绝**并给可行动提示（exit 2），不再是裸 `os error 6` | 已修（§5.5b） |
