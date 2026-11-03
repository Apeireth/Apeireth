[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-engine-status-2026-11-03.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-03
Author: deputy

# 记忆引擎状态报告（2026-11-03 收口）

> **这份是「状态快照」**：数字与结论**全部有实测出处**（见每项后面的括注）。
> 逐条推理过程在 [`memory-engine-closure-ledger-2026-11-01.md`](./memory-engine-closure-ledger-2026-11-01.md) 第 10 节。

## 一句话结论

**记忆引擎的「深化四期」全部完成，其中 P2（夜间整编）与在线质量漂移监控**已从库级推到生产接线**；
`DailyNote` 族**库侧已做真**（此前是桩），剩下「注册进工具面」需要产品契约层面的拍板。

## 一、规模（实测）

| 项 | 数字 | 出处 |
|---|---|---|
| 记忆 crate 模块数 | **133** 个（另有 7 个子目录模块） | `ls crates/engine/memory/src/*.rs` |
| `src` 内测试函数 | **919** 个 | `grep -c '#\[test\]'` |
| `lib` 测试数 | **918 passed / 0 failed**（1 ignored） | `cargo test -p apeireth-memory --lib` |
| 孤儿 API 基线 | **135 条**（本轮未增未减） | `python3 scripts/audit_unwired.py --check` |
| 本地门禁 | **39 步**（与 CI 的 63 个本地步 1:1 对齐） | `gates.sh` + `check_ci_gate_coverage.py` |

## 二、深化四期

| 期 | 状态 | 关键证据 |
|---|---|---|
| **P1 效果评估集** | ✅ 完成 | `scripts/golden-baseline.json` 固化 2 个语料；`case_fingerprint` 防偷换语料 |
| **P2 夜间整编** | ✅ **生产已接线** | `ConsolidationTrigger` → `MemoryCoordinator::run_consolidation_at` → `runtime-assembly/canonical/production.rs` 装配点；水位文件 `<data_root>/memory/consolidation-watermark.json` |
| **P3 教训回路** | ✅ 已接线 | `reflexion` 库外 16 处引用 |
| **P4 图谱/向量** | ✅ 完成 | `persistent_vector` 自述 default-off ⇒ **不是缺口**（模块自述即结论） |

## 三、生产接线点（都在远端 `debt-round8`）

| 接线 | 位置 | 语义 |
|---|---|---|
| **夜间整编驱动** | `production.rs` 装配点 | 窗口水位 + **只整新料** + 同窗口只整一次；水位**跨进程持久化** |
| **统一入口** | `cognitive.rs` 调 `run_consolidation_at(&session, now)` | 挂了驱动走增量，**没挂走一字未改的旧路径** |
| **检索质量漂移** | `retrieval_tuning_wire.rs` | 信号源 = 生产已有的**命中回执**（「上轮注入的记忆这轮是否再命中」）⇒ 告警累计计数 + 出口 |

**三条防假告警纪律**（写在代码注释里）：
1. **无证据 ⇒ 不采样**（「没数据」≠「质量差」）；
2. **证据不足 3 条 ⇒ 不采样**（一条证据的命中率只能是 0/1 ⇒ 噪声极大）；
3. **空轮不是机会轮 ⇒ 不裁决证据**（绝不拿「没有回执」当负信号）。

## 四、11-03 修掉的真问题（都是自查发现，不是别人报的）

| 问题 | 后果 | 修法 |
|---|---|---|
| **水位是单个全局时间戳** | 一个会话整编会推进另一个会话的水位 ⇒ 后者**永远等不到新料 ⇒ 静默停止整编**（无报错） | 水位改为**按会话隔离**（内存 `HashMap` / 文件 `{"sessions":{...}}`） |
| **遗忘的情节会被重新提炼成洞察** | **违反遗忘权**（忘了还能被读出来） | 产物照旧收全部素材并记账「丢了什么」，**报告只用治理可见的子集** |
| **`DailyNote` 的 MCP 只回显参数** | 4 个工具**从不真正执行** | 真执行（create/get/search/export） |
| **`DailyNote` 不传 tag 的搜索返回空** | 最常见的用法**永远搜不到东西** | 补 `store.list_all()` ⇒ 无 tag 也能搜 |
| **MCP 与本体各开一个 store** | 经 MCP 建的笔记在本体 `search` 里查不到 ⇒ **数据不通** | 共用一份库（`Arc`；`DailyNoteStore` 持 `Connection` ⇒ 不可 clone） |

## 五、还差什么（**未完成清单**）

| # | 事项 | 归属 | 状态 |
|---|---|---|---|
| 1 | `main` 合进 `debt-round8`（PR #94） | 总指挥 | **待做**（冲突文件多在对方改过的那些里） |
| 2 | `DailyNote` 的 4 个 MCP 工具注册进工具面 | 总指挥 | **待做**（工具清单 = 产品契约 ⇒ 跨区） |
| 3 | 检索排序 / 注入契约的 claim 登记 | 总指挥 | **待做**（`claims.yaml` = 真源） |
| 4 | `DailyNote` 台账条目的**分类**由「A 类真缺口」改成什么 | 总指挥 | **待裁决**（库侧已做真，理由已变） |

> ⇒ ⇒ **上面四项都不是我能单方面定的** ⇒ ⇒ 其余部分（第一至四节）**已完成且可复核**。

## 六、还需要别人拍板的三件事（都不是我能单方面定的）

| # | 事项 | 为什么需要别人 | 归属 |
|---|---|---|---|
| 1 | **把 `main` 合进 `debt-round8`**（PR #94） | 冲突文件多在对方改过的那些文件里 ⇒ 只能由 owner 合 | 总指挥 |
| 2 | **`DailyNote` 的 4 个 MCP 工具是否进工具面** | 工具清单 = **产品契约** ⇒ 跨区（`crates/capabilities/tools`） | 总指挥 |
| 3 | **检索排序 / 注入契约的 claim 登记** | `claims.yaml` = **真源** | 总指挥 |

## 七、纪律（8 条，全部是踩出来的）

1. **共享文件不许整文件从旧快照恢复**（11-02：静默抹掉总指挥的 `pub mod principal_sessions;`）
   ⇒ 推前跑 `clobber_check.py`（逐条列出被删行）；**验证要带 `--all-targets`**（`--lib` 绿不代表集成测试能编译）。
2. **同步远端时别把自己的改动弄丢**（11-03：snapshot + `reset --hard` ⇒ 工作树被覆盖）
   ⇒ 先比 blob sha 确认对方没动我要改的文件 ⇒ 才整文件套回 ⇒ 套回后**必须重跑测试**。
3. **推完必须核实远端真的收到文件**（`GET /contents/{path}?ref=tip`）—— 推送成功 ≠ 内容上去了。
4. **`/git/commits/{sha}` 要全 40 位 sha**（短 sha 一律 404）。
5. **同名遮蔽会伪造「债减少」** ⇒ 新公开 API 起名前先 `grep` 是否已是孤儿符号。
6. **名字出现 ≠ 真调用** ⇒ 判「已接线」要看 **import + 构造/调用**。
7. **零引用有多种假阳性形态**（同名遮蔽 / 真死被遮蔽 / 语言边界 / 间接消费链 / 模块自述 default-off）⇒ 判据要用**限定名**。
8. **只有 Windows 红的 CI 故障可以本地复现** ⇒ `cargo check --workspace --tests --target x86_64-pc-windows-gnu --keep-going`
   ⇒ **GNU 目标**（不要 MSVC，只要 rust-std）+ **`check` 不链接**（绕开 C 依赖）+ **`--keep-going`**（一次看全）。

## 八、工具（仓外，不进 git）

| 工具 | 作用 |
|---|---|
| `apipush2.py` | API 推送（**显式两基点**；校验 tree 逐字相同） |
| `pushon.py` | 重试式推送（⚠️ **会丢已提交内容** ⇒ 已提交的批次别用它） |
| `clobber_check.py` | **推前守卫**：列出「相对远端基点被删除的行」⇒ 别覆盖别人的东西 |
| `win_check.py` | 本地交叉编译复现「只有 Windows 红」的故障 |
| `gates.sh` | 本地 39 步门禁 |

## 九、相关文件

- 逐条推理过程：[`memory-engine-closure-ledger-2026-11-01.md`](./memory-engine-closure-ledger-2026-11-01.md)
- 未接线台账：[`docs/status/unwired-ledger.md`](../../status/unwired-ledger.md)
- 声明口径：[`docs/status/claims.yaml`](../../status/claims.yaml)
