# V4-b 记忆保鲜回路 · 交付实录（散列增量门 + 可恢复摄取队列 + 增量折入）

> `Status: ✅ 已交付（2026-10-17，V4-b 批）` ｜ 战场：`crates/engine/memory`（新件 `freshness/`）+ runtime-assembly 整编面 + `tool.self_status` 记忆节
> 出处：哲学研究记忆族机制卡（`docs/04-internal/philosophy-study/02-memory-family.md` 卡 6-3 保鲜三件套 + 卡 5-3 增量折入）假说改造；补「持续保鲜回路」空位。
> 来源纪律：中性表述、扫描零命中、不写 CHANGELOG、不 commit、校验 `-j 2`。
> 只读禁区 0 触碰：`persona_layer.rs` / `reflexion.rs` / `river_topology.rs`（基线散列存证见 §9）。

## 0. 一句话

记忆库的运行成本从「随摄入次数增长」压到「随变更量增长」：**散列增量门**（同源同散列跳过重复摄取/整编，省算力防重复记忆）+ **可恢复摄取队列**（待摄取清单落库、重启续摄、失败留队列带尝试计数）+ **增量折入**（同主题追加不重写，机器钉死 append-only）+ `tool.self_status` 记忆节三真值（待摄取数 / 最近折入 / 去重计数，缺源显式 null）。

## 1. 落点清单（设计 4 项 → 文件/表）

| 设计项 | 落点 | 语义 |
|---|---|---|
| 1 散列增量门 + 散列账 | `crates/engine/memory/src/freshness/hash_gate.rs`（FNV-1a 64 自含散列 + `IngestGate` 查门/两笔记账）+ 迁移 `V17__memory_freshness_loop` 的 `source_hash` 表 | 同源同散列 = 跳过重复摄取/整编（省算力防重复记忆）；变内容重摄；散列账 `(source, content_hash)` 主键 + `ingest_count`/`skip_count` 计数，账行禁删（触发器） |
| 2 可恢复摄取队列 | `crates/engine/memory/src/freshness/ingest_queue.rs` + V17 `ingest_queue` 表（`(source, content_hash)` 唯一约束） | 待摄取清单落库 = 进程重启续摄；失败留队列（`pending` 不是终态）+ 尝试计数 `attempts` + `last_error`；串行泵一次一条；折入/队列/散列记账**同一事务**原子落库 |
| 3 增量折入 | `crates/engine/memory/src/freshness/fold.rs` + V17 `memory_fold_segments` 表 | 同主题**追加**新段（`seq` 递增），既有段字节级不改写（UPDATE/DELETE 触发器 ABORT =「追加不重写」结构上不可能，不是调用方自律）；主题视图按 `seq` 拼接 |
| 4 self_status 记忆节扩 | `crates/capabilities/tools/src/self_status.rs`（`MemoryLedgerStats` + 3 字段）+ `crates/engine/memory/src/freshness/mod.rs`（`FreshnessStatus` 真值）+ CLI 探测口同库同账 | 待摄取数 / 最近折入（epoch 毫秒）/ 去重计数真值；缺源显式 `null` + 原因，尚未折入 = `null` 不编时间；**只报数字不回内容** |

整编面接线（runtime-assembly）：`MemoryWritebackModule::with_ingest_gate` 挂在**整编触发点**（`with_consolidation` 同一条管线）—— 整编素材内容散列查门，同源同散列跳过整轮整编；`ProductionBackends.ingest_gate` 落库面 + `ProductionModulesConfig.consolidation_hash_gate` 开关（**默认关**）+ 名册行 `consolidation_hash_gate`（按实际注册条件取值）。

## 2. 三件套管线图

```text
 摄取请求 (source, topic, content)
     │
     ▼
 ① 散列增量门 (hash_gate): FNV-1a 64 内容散列, 查 source_hash 散列账
     ├─ 同源同散列已在账上 ──▶ 跳过重复摄取 (省算力防重复记忆)
     │                        └─ 去重计数 skip_count +1 (真值)
     └─ 同源新散列 (新内容 / 变内容) ──▶ 放行
     │
     ▼
 ② 可恢复摄取队列 (ingest_queue): 待摄取清单落库 (ingest_queue 表)
     ├─ 串行处理 (一次泵一条, 并发不放大下游)
     ├─ 失败留队列 + 尝试计数 (attempts / last_error; 不静默丢, 不伪造完成)
     └─ 进程重启续摄 (清单落库 = 重启即续; pending 真值即「待摄取数」)
     │
     ▼
 ③ 增量折入 (fold): 同主题追加新段 (memory_fold_segments 表)
     ├─ 追加不重写 (旧行字节原样; 触发器钉死 append-only)
     └─ 折入后散列记账 (mark_ingested), 与队列收场同一事务
     │
     ▼
 自述记忆节真值: 待摄取数 / 最近折入 / 去重计数 (缺源显式 null)
```

整编面（另一条消费路径，同一个门）：

```text
 AfterTurn 整编触发点 (with_consolidation, 默认关)
     │
     ▼
 整编素材 = 治理可见近期情节的原始证据角色 (user/assistant/tool),
            派生记忆不入料 (W2 同口径); 去重 + 字典序规范拼接后散列
     │
     ├─ 同源同散列已在账上 ──▶ 跳过整轮整编 (省算力防重复记忆, 去重计数 +1)
     ├─ 变内容 / 新内容 ──▶ 整编 (run_consolidation + 人格提取步)
     │                       └─ 全部成功后才记摄入账 (失败不污染账, 可重摄)
     └─ 门自身故障 ──▶ fail-open 照常整编 (真因进 warning 计数, 不静默降级)
```

## 3. 记账语义（错误即帧，失败不污染账）

| 动作 | 时机 | 账效 |
|---|---|---|
| `check` | 摄入前 | **只读判定**，零记账（未记账不得跳过 —— 一次失败不把素材永久拉黑） |
| `mark_skipped` | 命中同源同散列 | `skip_count` +1（去重计数真值） |
| `mark_ingested` | 摄取/整编**成功后** | `(source, content_hash)` 入账 + `ingest_count` +1；折入与记账同一事务，无「段折了账没记」裂缝 |
| 摄取失败 | 泵失败 / 整编步硬失败 | 零记账 + 任务留队列 `attempts` +1 记 `last_error`（下次照常放行重摄） |

## 4. self_status 记忆节扩（`memory_ledger` 节字段级演进）

| 字段 | 真值来源 | 缺源/初始态 |
|---|---|---|
| `pending_ingest` | `ingest_queue` 表 `state='pending'` 行数 | `null` + 原因（探测失败）；如实 `0`（队列空） |
| `last_fold_at_ms` | `memory_fold_segments` 最新 `folded_at_ms`（epoch 毫秒） | 尚未折入 = `null`（**不编时间**） |
| `dedup_skipped` | `source_hash` 表 `skip_count` 总和 | `null` + 原因（探测失败）；如实 `0`（无去重） |

形状冻结演进注记：节合同 [`SELF_STATUS_SECTIONS`] **未动**（仍 14 节），本批是 `memory_ledger` 节内**字段级**加性演进（`MemoryLedgerStats` 全 `Option` 字段，字段级 null 语义既有）；渲染面 / 探测口 / fixture 三侧同账。

## 5. 接缝裁定（择简说明：独立轻量面，不复用 V6-a 任务队列）

`crates/foundation/orchestration/src/task_queue.rs`（V6-a）读码后裁定**独立轻量面**（`freshness/ingest_queue.rs`），理由：

1. **失败语义冲突**：V6-a 明文「失败绝不自动重试，`Failed` 只能人判」；本设计要求「失败留队列 + 尝试计数 + 重启续摄」的自动重试语义。复用必须改 V6-a 的状态机语义（禁区「task」不可改），或在其上叠一层绕过语义的壳（两套语义同名不同魂 = 隐性债）。
2. **事务接缝**：摄取队列要求「折入 + 队列收场 + 散列记账」同一事务原子落库（防「段折了账没记」裂缝）；V6-a 的 `TaskStore` 端口是整档原子读改写，接不进同事务。
3. **形状最小**：摄取清单只需两态（`pending`/`done`，失败留在 `pending`）+ 尝试计数；独立面 ~300 行（含测试），零跨 crate 语义牵连。

V6-a 任务队列 0 改动（`task_queue.rs` / `sqlite_task_queue.rs` / `task_board_tool.rs` 均未触碰）。

## 6. 测试证据（新增 27 个测试函数 + 既有断言扩展；全部行为钉死）

**三件套核心（`crates/engine/memory`，22 个）**——`src/freshness/hash_gate.rs` 8 个（FNV-1a 标准向量锚、散列稳定/随内容变、同内容跳过、变内容重摄、异源不去重、去重计数累计、只查门不污染账、空参拒绝即帧）；`src/freshness/fold.rs` 4 个（**追加不重写**断言旧段字节原样、UPDATE/DELETE 触发器拒绝、按主题独立编号、空参拒绝）；`src/freshness/ingest_queue.rs` 5 个（入队幂等单行、已摄取跳过+去重计数、变内容新行、**失败留队列+尝试计数**、空队 Idle）；`tests/freshness_loop.rs` 5 个（管线端到端、**队列重启续摄**（文件库重开续摄）、**失败留队列跨重启**（尝试计数/原因落库）、**增量折入跨重启追加不重写**、真值口径（未折入 = 显式 null 不编时间））。

**整编面（`crates/engine/runtime-assembly`，4 个）**——`cognitive.rs`：同源同散列跳过整轮整编（脚本门）、门故障 fail-open 照常整编、成功后恰一笔摄入记账、真散列门端到端（同素材第二轮跳过 + 去重计数 +1 + 不产重复记忆；变素材重摄 + 去重计数不涨）。

**自述节（1 个新 + 4 处既有断言扩展）**——`tools/self_status.rs` 新增保鲜三真值渲染测试（真值 / 尚未折入显式 null）；既有 fixture/缺源测试扩展三字段断言；`runtime-assembly` 自述探测透传断言；`self_status_runtime_e2e` 真库探测真值断言（含缺源 null 清单扩展）。

**既有套件零回归**：见 §7 校验（`--no-fail-fast` 全绿）；只读三件 + `tests/persona_layer.rs` 全绿且文件 0 改动（§9 散列存证）。

**既有断言演进一处（注明理由）**：`migrations.rs::migrations_reopen_preserves_data_and_idempotent` 的迁移计数断言 12 → 13（沿该断言既有的「迁移列表 append-only 增长面，数字随迁移清单走」演进注记先例；钉的是「无重复」语义，不是字面清单）。这是 V17 迁移追加的唯一连带断言改动（其余既有断言 0 改动）。

**口径注记（如实）**：「214 套件」为接棒口径中的**测试二进制目标计数**（真源 `docs/03-reference/test-baseline.md` 最近复测注记 = 208 套件 / 5619 passed / 2026-10-17，随批增长）；本批新增 1 个测试目标（`crates/engine/memory/tests/freshness_loop.rs`），终卷实测套件/通过数见 §7。另 `apeireth-memory` crate 级实测（2026-10-17）：批前 905 passed（lib 822 + 集成 83）→ 批后 **927 passed / 0 failed / 1 ignored**（lib 839 + 集成 88；新增 22）。按口径纪律报本批实测值，不代改真源。

## 7. 校验（2026-10-17，`-j 2` 口径）

| 项 | 命令 | 结果 |
|---|---|---|
| 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ **215 套件 / 5774 passed / 0 failed / 20 ignored** |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告 |
| 格式 | `cargo fmt --check` | ✅ 0 diff |
| 中性扫描 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（86 变更文件新增行全中性） |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无发现 |
| 文档口径 | `scripts/check_doc_caliber.py`（本实录） | ✅ 新增违规 0 处 |

**套件口径**：批前接棒口径「214 套件」+ 本批新增 1 个测试目标（`crates/engine/memory/tests/freshness_loop.rs`）= **215 套件**，对上（「套件」= 测试二进制目标计数，与真源注记同口径）。通过数 5774 为当前工作树全量实测（树上含前批未提交批次，按真源纪律「逐批交付只报自己批的增量」）：本批增量 = 新增 27 个测试函数 + `apeireth-memory` crate 905 → 927。

**负载 flake 定性注记（如实）**：全量串行复跑第 3 轮曾出现 1 例 `apeireth-tools-canonical` lib 负载 flake（416 passed / 1 failed，失败名未捕获），**隔离复跑 2 次全绿**（417 passed / 0 failed ×2），第 4 轮全量 5774 全绿 —— 按纪律「负载 flake 隔离复跑定性」记录，非确定性失败，未改动任何相关测试。

## 8. 接缝诚实声明（0 装双向性）

- **整编散列门开关 `consolidation_hash_gate` 默认关**：能力已在装配面接线（`ProductionBackends.ingest_gate` + 名册行按实际注册条件取值），CLI **未提供 env 旋钮**（与 V4-a `persona_consolidation` 同口径：开关在装配面可开，运行时开启归口 CLI 接线面）。不把默认关写成系统现状。
- **摄取队列的生产摄取触发源未接线**：本批交付的是队列机制面（持久清单/重启续摄/失败留队列/幂等）与整编面的散列门；把外部素材（内容流拉取等）投进摄取队列的触发源归口各自接入面（队列入口语义已备：`SqliteMemoryStore::enqueue_ingest`）。
- **战场外最小改动报备**（顺手修三判第二档：验收必经 + 最小改动 + 理由）：`crates/adapters/cli/src/lib.rs` 的 `self_status_ledger_probe`（记忆节三真值同库现探测）与 `CognitiveBackends` 构造（`ingest_gate` 落库面接线），共两处，均为 V4-b 验收必经的自述真值/整编门落库面，未动其他逻辑。
- 记忆节三字段只报数字与时刻，**永不回内容原文**（与记忆账本节同约束）。

## 9. 只读三件零改动存证（SHA-256，2026-10-17 取）

| 文件 | 基线 = 终局 |
|---|---|
| `crates/engine/memory/src/persona_layer.rs` | `EFE2FAF350F85B939912AE4034F6542C04A1E4A76F5D64CF550319C20DDE38E2` |
| `crates/engine/memory/src/reflexion.rs` | `422CF292556B520599EB340FE183D3352A2566197B129E0F086EEFFF4D062F21` |
| `crates/engine/memory/src/river_topology.rs` | `4B4F57E7468F2EF25AC02DE3A281FD1DF0962EDED984EB461EA61D5D9D3FDFC0` |

## 10. 后续 stage

- 外部素材接入面（内容流拉取 / 会话提交）把新素材投 `enqueue_ingest` —— 保鲜回路的摄取触发源归口各自接入面。
- CLI env 旋钮（`consolidation_hash_gate` 开启档）与摄取泵的后台节拍（cron/夜间档）归口装配线。
- 散列账/折入段账的长期归档与轮转（现只增不删）留后续阶段裁定。
