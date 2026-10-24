# 清债轮验收摘要（2026-10-25 凌晨）

> 给 owner 的一页纸：**做了什么、还差什么、为什么差、需要谁拍板**。
> 本轮全部在本地提交（网络不稳，按协作看板 §七「本地推进为主，不推不等待」）。

---

## 一、本轮交付（本地已提交，未推）

分支 `debt-round8`（worktree `/tmp/apeireth-r3`）。

### 已推部分（PR #94，`state=open`，6 项 required check 中 5 绿）

| 设备 | 内容 |
|---|---|
| 限流守门 | `RateLimitGovernanceHook` 挂进生产治理链（授权之后、内容风险之前） |
| partner 持久化 | `InMemoryPartnerStore` → `SqlitePartnerStore`，顺带修 `ensure_schema` 从不被调用 |
| 代码地图 | `tool.repo_map` 工具面 + 生产注册；修掉两处平方级热点（152 文件 9.5s→0.26s、全仓 1319 文件 10.8s→2.1s） |
| 查询扩展 | 接进检索主路径，做成**召回增强**而非替换查询 |
| 被遗忘权 | 已在 #90 合入 main |

### 本地未推（8 个 commit）

| 内容 | 价值 |
|---|---|
| 推理字段归一化 | 12 别名 → 前置 `<think>` 块接进 provider；关键顺序「先归一化后判空」，否则推理模型烧光预算会被误判成空壳响应 |
| `apeireth bundle` 便携包 | 从「README 自认没发货」到**真机跑通**：`./run_apeireth.sh` 起网关 → `/health` ok → 数据落在包内 `./data/`。接线时修三处硬伤（启动脚本调不存在的命令、设无人读的变量、配置默认开**明文 P2P 桩**） |
| 错误闭合词表 | `ClassifyError` 此前**零实现**；现覆盖 SDK 三族 + 跨语言 `ErrorFrame` |
| 关系里程碑 | 新增 `SqliteMilestoneStore`（关系史跨重启）+ 幂等记录面 + 接进羁绊模块（初见 / 阶段跃迁） |
| **守卫判定落盘** | 消掉 `nightwatch` 自报缺口：守卫判定此前只在内存、重启即散。真机验证：`0 audit` + 缺口 → `2 audit` + 报出 `[medium] NoDegrade: critical → info` |
| file_fetcher 磁盘缓存 | `cache_dir` 原是**死字段**（文档宣称的磁盘缓存不存在）⇒ 真做出来（两级内容寻址 + 每次命中校验 + 原子写） |
| 五维记忆 Recent 维度 | 宣称 5 维实际只产出 4 维（`Recent` 从不产出、后备字段从不写入）⇒ 补齐 24h 滑动窗口 |
| async_context 更正 | 从 A 类改为 B 类（重复覆盖）：原判据「`ContextProjector` 是 `NoContextProjector`」前提不成立——生产无条件注入 `MemoryContextProjector` |

---

## 二、A 类缺口总账（原 16 条）

| 状态 | 数量 | 明细 |
|---|---|---|
| ✅ 已修并合入 main | 1 | 被遗忘权（#90） |
| ✅ 已修（本地，待推） | 4 | 限流、partner 持久化、代码地图、查询扩展（在 #94 未合） |
| ✅ 已修（本地新增） | 4 | 推理适配、便携包、错误词表、关系里程碑 |
| 🔶 半修（实现补全，消费点待定） | 2 | file_fetcher（消费点在副总指挥 crate）、五维记忆（面板未用） |
| 🔁 更正为重复实现 | 1 | async_context |
| ⛔ 受上游能力阻塞 | 4 | `cross_diary`（日记不在检索路径）、`three_tier_vault`（缺 markdown 语料面）、`care_potential_field`（全仓**无** distress/fatigue 信号源）、`speech_arbiter`（全双工未装配） |
| ⏸ 主动延后 | 1 | `continuation`（会话与审批历史**都已持久化**，剩余缺口要动 `execute.rs` 4987 行关键路径，风险/收益比差） |

**外加**：本轮自行发现并修掉的**非清单**缺陷 1 条（守卫判定无落盘面 → 已修）。

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

## 三、需要拍板的事（owner 不在，我按纪律没自作主张）

| # | 事项 | 为什么我不做 | 建议 |
|---|---|---|---|
| 1 | **`dead_code` 全局 allow** | `Cargo.toml` 里 `dead_code='allow'`，注释写明原因是「当时只有 1 处警告，在别人正在改的文件里」⇒ 整个 workspace 的这道闸被关掉。本轮临时打开拿到 60 条线索，查到 **1 条真缺陷**（五维记忆 Recent 恒空，已修）+ 1 条冗余（已删）。**收益大**：这类「字段存了不用」正是「能力齐全但没接电」最常见的形态 | 改 `dead_code='warn'` + 基线棘轮（只减不增）。但会让 CI 立刻红 60 处，且 CI workflow 属副总指挥范围 ⇒ 需拍板 |
| 2 | `council ballot` 未持久化 | council 已在生产装配，但 `rubric::Ballot` 需要 `confidence`/`weight`，而 council 只产出 `verdict`；`Ballot::new` **全仓从未被构造**。硬接必须**编造**信号 ⇒ 按「不编造信号」停下 | 需定「一个顾问评价如何折算成一张票」（stance 从 verdict 怎么来、confidence/weight 谁给），或改 nightwatch 输入类型 |
| 3 | `care_potential_field` 的 distress/fatigue 信号源 | 全仓 `grep` 无任何 fatigue/distress 生产者。造信号 = 发明产品语义（什么算疲劳？深夜？连续失败？） | 要么定信号定义（可作为下一批独立任务），要么承认该能力需要前置研究 |
| 4 | `three_tier_vault` / `cross_diary` | 前者缺 markdown 知识库语料面，后者需要日记进入检索路径 —— 都是产品形态决策 | 若日记/知识库要成为产品能力，先定语料来源与召回语义 |
| 5 | `speech_arbiter` 的发言权仲裁 | 全双工语音未装配（`voice.duplex = not_assembled`）⇒ 无输入可仲裁 | 等语音面立项 |

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
