# J 生活接入轴（life integration）——立轴

> `Status: 🚧 施工中（2026-10-13 J1「内容流接入」落地, 立轴同批; 2026-10-17 J2「拉取→摄取接缝 + 订阅生命周期」落地）`
> **范围口径**：J 轴 = 让 Apeireth 接入现代生活与网络（owner 定向：「Apeireth 可以做自己刷视频的模块，对接入现代生活，网络」）。
> **关系**：想法总纲见 [J-tradewind.md](J-tradewind.md)（生活信息流「信风」：多源订阅 + 摘要 + 陪伴共看）；本文件是 J 轴的**工程立轴**（现状 / 想法 / 缺口 / 分期），随批滚动。承载面（星港）见 [I-starport-builtins.md](I-starport-builtins.md)，治理口径遵 [CONSTRUCTION-DISCIPLINE.md](../CONSTRUCTION-DISCIPLINE.md)。

## 0. 立轴——J 轴是什么

让它**看见你的世界**：**内容流感知 → 记忆流**。外部世界的内容（订阅源 / 网页 / 后续视频源）经受控通道进来，结构化成摘要，作为**未经整理的原料**进记忆流；模型经 **L2 封套披露**（检索召回）看见它们——**跨源内容永不直进提示词**。

三条轴纪律（与想法总纲同文，随批验收）：

1. **不做对抗性抓取、不绕平台风控**；来源条款不可接受的源不上架；
2. **注意力纪律**：刷流时长 / 烧钱闸与 A 轴配额真接口联动（J2 起），防沉迷是产品底线；
3. **隐私**：观看画像只进本地记忆，不外发（星群共享只走匿名化经验）。

## 1. 现状（J1「内容流接入」已交付, 2026-10-13）

### 1.1 能力面

| 面 | 落点 | 语义 |
|---|---|---|
| 订阅源管理 | `tool.content_feed` 动作 `feed_add` / `feed_remove` / `feed_list` | 数据目录 `feeds.json`（StoredDoc 信封, **拒开语义**：畸形 / 串档 / 未来版本 / 坏条目 = 拒绝打开, 不静默迁移、不回退默认）；**改订阅 = 写文件**（write_intent 同族, 每次写停人工审批） |
| 受控拉取 | 动作 `pull` | 复用受控出口通道（与 `tool.fetch` 同一 `ControlledEgress` 口径：仅 http(s)、GET、纯文本、响应大小上限、超时归 `timeout.*` 族）；冻结载荷 = **订阅快照**（人批的就是这批 URL, 恢复审批不换目标） |
| 结构化摘要 | `content_feed/summary.rs` | **标题 / 正文截断 / 来源 / 时间**：正文 2000 字符截断（`body_truncated` 如实标注）；来源 = 订阅 id + 订阅 URL + 条目链接；时间 = 来源自带时间戳**原文**（不换算不编造）+ 拉取时刻实测；RSS `<item>` / Atom `<entry>` 逐条切块, 网页 / 纯文本单条 |
| 入记忆流 | `EpisodeWriter` 接缝 | **EpisodeSink 同源**：与感知链同一 append-only episodic 写入面（同一 `EpisodeStore::put_episode` 委托目标）、同一条 episode 流；原料 = **未保护记忆**（无保护标记, 可正常整理 / 遗忘——保护是人工显式动作）；内容裹 [`UntrustedEnvelope`]（**L2 封套披露语义**：固定警示头 + 显式边界 + 边界伪造转义, 每源预算截断）, **来源标注** `content_feed:<订阅 id>` |
| 治理 | `ContentFeedApprovalHook`（治理链） | **RequireApproval 级**：每次调用停人工审批, 无自动放行档（二选一已钉死, 见 §3）；敏感面是**拒**不是批 |
| 自述 | `tool.self_status` `content_feed` 节（九节合同演进, 附理由） | 能力名册一行 `content_feed`（真实注册条件）+ **订阅数 / 最近拉取**（`feeds.json` 真值；尚无拉取 = 显式 null + 如实注「尚无拉取记录」, 不编时间） |

### 1.2 接线图

```text
                      ┌──────────────── 治理层（审批四态 / 本地审批面板 · IM 审批卡同链）───────────────┐
                      │ ContentFeedApprovalHook: tool.content_feed 每次调用 → RequireApproval         │
                      │ （拉取与入库都经审批档；改订阅=写文件同族同档；无自动放行档；理由按写面标注）    │
                      └──────────────────────────────▲────────────────────────────────────────────┘
                                                     │
模型 ──调用──▶ tool.content_feed（五段流水线: 超时 timeout.* + 输出归一合同）
   │
   ├─ feed_add / feed_remove ──▶ FeedStore（数据目录 feeds.json, StoredDoc 拒开）──改订阅=写文件──▶ 人工审批
   │        └─ URL 校验（敏感面拒, 非审批）: 凭据 userinfo / 非 http(s) / 内网 IP 字面量
   │           （IP 分类复用受控通道 classify_ip/is_public_ip; 主机名归解析后逐跳重验）
   ├─ feed_list ──▶ 读订阅表 + 拉取台账（纯读面, 随能力同档）
   └─ pull ──▶ [冻结订阅快照] ──▶ ControlledEgress（仅 http(s)/GET/文本/大小上限/超时, 与 tool.fetch 同通道）
                  │
                  ▼
            结构化摘要（标题 / 正文截断 / 来源 / 时间）
                  │
                  ▼
            UntrustedEnvelope（L2 封套披露: 警示头+边界+伪造转义, 来源标注 content_feed:<id>）
                  │
                  ▼
            EpisodeWriter ──▶ episodic 记忆流（EpisodeSink 同源, 未保护记忆）◀──感知链同一写入面
                  │
                  └─▶ 拉取台账（feeds.json last_pull, 自述「最近拉取」真值来源）

工具结果 ──▶ 模型只看到元数据（订阅 id / 来源 URL / 条数 / episode id / 成败真因）——内容不直进提示词

日后召回: 检索 ──▶ L2 检索披露（同一封套披露语义, memory_injection/retrieval_pipeline 既有收口）──▶ 提示词
```

### 1.3 接线落点（变更面）

- 新增 [`crates/capabilities/tools/src/content_feed/`](../../../../crates/capabilities/tools/src/content_feed)：`mod.rs`（治理档钩子 + 能力组装）、`store.rs`（订阅表 StoredDoc + 敏感面拒）、`summary.rs`（结构化摘要）、`tool.rs`（`tool.content_feed` 四动作 + 入记忆流）；
- [`self_status.rs`](../../../../crates/capabilities/tools/src/self_status.rs)：`content_feed` 节（订阅数 / 最近拉取）入九节合同；
- [`crates/engine/guard/src/semantics.rs`](../../../../crates/engine/guard/src/semantics.rs)：`tool.content_feed` 规范描述符（读网络 + 写记忆流双面如实标注）；
- [`tool_modules.rs`](../../../../crates/engine/runtime-assembly/src/canonical/tool_modules.rs) / [`production.rs`](../../../../crates/engine/runtime-assembly/src/canonical/production.rs) / [`self_status_source.rs`](../../../../crates/engine/runtime-assembly/src/canonical/self_status_source.rs)：装配注册 + 名册行 + 自述探测口；
- [`crates/adapters/cli/src/lib.rs`](../../../../crates/adapters/cli/src/lib.rs)：`APEIRETH_ENABLE_CONTENT_FEED=1` 旋钮（注册 + grant + 审批档钩子同源）、内容流自述探测口、记忆流写入面（与感知链同一 episodic 写入）；
- 测试：`tools/tests/content_feed.rs`（集成: mock 端点 / 大小·超时·协议面拒 / 敏感面 / 自述行）、`runtime-assembly/tests/content_feed_registration.rs`（装配级: 注册 / 名册 / 自述真值 / 真存储入流未保护级）、模块内单测（订阅表拒开 / 摘要结构化 / 封套隔离 / 治理档语义）。

## 1.4 J2 拉取→摄取接缝 + 订阅生命周期（2026-10-17 落地）

> 战场 `crates/capabilities/tools/src/content_feed/`（J1 件叠加）+ 记忆保鲜回路摄取口**只读消费**（`enqueue_ingest` 端口, 不改 freshness 内部件）。J1「拉取即入记忆流」升级为「**走保鲜回路**（去重/队列/折入全自动）」。

**接缝管线图**：

```text
 pull ──▶ [冻结订阅快照] ──▶ ControlledEgress 拉取文本
              │
              ▼
        结构化摘要（标题/正文截断/来源/时间）
              │
   ┌──────────▼─────────── 每条摘要 ────────────┐
   │ ① 条目账去重（条目级幂等, lifecycle）        │
   │    同条目重复拉取 ──▶ 跳过 + 去重数 +1       │──▶ 零新记忆（不写 episode, 不投摄取）
   │ ② 注意力纪律（单次拉取上限 / 每日上限）      │
   │    超限 ──▶ 截断 + 如实标注（不静默丢）       │
   │ ③ 未超限未去重 ──▶ 下面两路并行              │
   └──────────────┬───────────────────────────┘
                  │
      ┌───────────┴────────────┐
      ▼                        ▼
  EpisodeWriter（J1 原料面）   IngestPort = enqueue_ingest（J2 保鲜回路, 接上时）
  ──▶ episodic 记忆流          ① 散列增量门（同源同内容去重, 内容级）
  （L2 召回原料, 未保护）          ② 摄取队列（重启续摄/失败留队列）
                                 ③ 增量折入（同主题追加不重写）
      │                        │
      └───────────┬────────────┘
                  ▼
   订阅生命周期账（feed_lifecycle.json）: 条目去重数 / 投摄取数 / 截断数 /
   逐订阅健康（连续失败→退避放大→degraded）/ 日计数
                  │
                  ▼
   self_status `content_feed` 节扩: dedup_skipped / ingest_queued /
   attention_truncated / degraded_feeds / backing_off_feeds（只报数字不回内容）
```

**落点**：`ingest.rs`（`IngestPort`/`IngestOutcome` 逐态映射 `EnqueueOutcome` + 注意力上限 + 条目身份 `source=feed id+条目链接`）、`lifecycle.rs`（条目去重账有界 FIFO + 退避 + 健康度 + 日计数, StoredDoc 拒开）、`tool.rs`（`execute_pull` 接缝）、`store.rs`/`self_status.rs`（自述节字段级加性演进）、CLI 摄取口适配（战场外最小改动报备）。两道去重合防刷屏：**条目账**（条目级, 防重复 episode）+ **散列门**（内容级, 防重复折入）= 同条目二次拉取**零新记忆**。

**测试证据（新增 26 例, 全部行为钉死; J1 既有 14 例零改动全绿）**：

| 验收项 | 测试 | 例数 |
|---|---|---|
| 拉取→摄取接缝（source=feed+条目链接, 折入主题=feed） | `content_feed_lifecycle.rs::pull_feeds_item_summaries_into_the_ingest_port` | 1 |
| 去重断言同条目二次拉取零新记忆 | `content_feed_lifecycle.rs::repeated_pull_of_the_same_items_yields_zero_new_memories`、`content_feed_freshness.rs::repeated_pull_of_the_same_items_yields_zero_new_memories`（真 enqueue） | 2 |
| 拉取→摄取全链（真散列门/队列/折入） | `content_feed_freshness.rs::pulled_content_lands_in_the_freshness_loop` | 1 |
| 散列门内容级去重（条目账丢失仍兜底） | `content_feed_freshness.rs::same_content_is_deduped_by_the_hash_gate_when_the_item_ledger_is_reset` | 1 |
| 幂等账（条目账, 跨进程重开） | `content_feed_lifecycle.rs::item_ledger_is_idempotent_and_survives_a_reopen` + `lifecycle.rs::item_ledger_is_idempotent_per_source` | 2 |
| 注意力上限截断如实（单次/每日） | `content_feed_lifecycle.rs::attention_per_pull_cap_truncates_and_labels_honestly`、`attention_daily_cap_truncates_and_labels_honestly` | 2 |
| 拉取失败退避（下次间隔放大, 不锤失败源） | `content_feed_lifecycle.rs::pull_failure_backs_off_the_next_pull` + `lifecycle.rs::backoff_grows_on_consecutive_failures_and_degrades` | 2 |
| degraded 标注（连续失败 N 次） | `content_feed_lifecycle.rs::consecutive_failures_mark_the_feed_degraded` + `lifecycle.rs::success_resets_backoff_and_degraded` | 2 |
| 自述节（去重数/投摄取/截断/订阅健康） | `content_feed_lifecycle.rs::self_status_content_feed_section_reports_lifecycle_stats` + `lifecycle.rs::stats_counts_degraded_and_backing_off_honestly` | 2 |
| 摄取端口故障即帧 | `content_feed_lifecycle.rs::ingest_port_failure_is_a_hard_frame_not_a_silent_drop` | 1 |
| 其余单测（条目身份/稳定内容/日计数/有界账/存储往返/坏账拒开/上限/结果态） | `ingest.rs` 6 例 + `lifecycle.rs` 其余 | 10 |

**校验（2026-10-17, `-j 2`）**：`cargo test --workspace --all-targets --locked --no-fail-fast -j 2` ✅ 220 结果行 / 5826 passed / 0 failed / 20 ignored（含前批未提交批次, 本批增量=新增 26 测试函数 + 2 测试目标 `content_feed_lifecycle`/`content_feed_freshness`）；`clippy -D warnings` ✅ 0 警告；`fmt --check` ✅ 0 diff；`check-neutral-terms.ps1` ✅ 零命中；`secret-scan.ps1` ✅ 无发现。

**接缝诚实**：摄取端口 `ingest_port: Option<IngestPort>`——未接（`None`, J1 口径）仍走 J1 原料面；生产 CLI 已接真 `enqueue_ingest`（内容流拉取→保鲜回路全自动）。`ingest_queued` 报「投摄取入队累计」（现存待摄取真值见 `memory_ledger.pending_ingest`, V4-b）, 不跨面编造待摄取深度。



| # | 想法 | 一句话 |
|---|---|---|
| a | 多源条目流统一抽象 | 信息源插件化（资讯 / 动态 / 后续视频源）, 统一「条目流」入记忆流的口径（想法总纲要点 1 的工程化） |
| b | 摘要与陪看 | 智能体预筛 / 摘要 / 陪看点评, 省时间而非抢时间（依赖模型面 H 与星港 I 承载） |
| c | 注意力纪律联动 | 刷流时长 / 烧钱闸数值化接 A 轴配额真接口, 防沉迷是产品底线 |
| d | 去重与已读台账 | 同源条目跨次拉取不重复入流, 「看过」状态本地可查 |
| e | 定时拉取与刷新 | 订阅按主人节律自动刷新（调度面, 拉取仍受同一治理档） |
| f | 观看画像隐私 | 画像只进本地记忆不外发; 星群共享只走匿名化经验（星群纪律） |
| g | 来源条款审读面 | 上架前对来源条款做可接受性审读（铁律的可执行化） |

## 3. 治理钉死记录（二选一, 本批钉死）

| 备选 | 处置 | 理由 |
|---|---|---|
| A「拉取只读档、入库审批档」 | **否决** | 拉取只读档意味着跨源内容在人审之前就以摘要形态回到调用侧——**正是本面要闸的注入面**; 「入库才审批」闸得住记忆, 闸不住提示词 |
| B「**拉取与入库都经审批档**」 | **选定（钉死）** | 人先看见「拉哪些源」并拍板, 内容才进世界; 一次调用 = 拉取 + 入库一个动作, 无免审中间态 |

- **档位承载**：`ContentFeedApprovalHook`（治理链钩子, 与本地审批面板 / IM 审批卡同一条审批链）——`tool.content_feed` **每次调用**停 [`Decision::RequireApproval`]，**无自动放行档**（任何预设 / 设置都不得越人）；审批理由按动作如实标注写面（pull = 读网络 + 写记忆流双面 / feed_add·feed_remove = 改订阅 = 写文件 write_intent 同族 / feed_list = 只读列随面同档）。只读自述走 `tool.self_status`（零审批）看订阅数 / 最近拉取, 不因此缺可见性。
- **敏感面（拒, 不是审批可放）**：带凭据（userinfo）的 URL、非 http(s) 协议、内网 / 回环 / 链路本地 IP 字面量一律**拒绝**——不进订阅表、不出网、拒绝帧不回显凭据本体。IP 分类复用受控通道的分类函数（复用不重复）; 主机名的内网判定归受控出口通道解析后逐跳重验（按口径不在工具内做 DNS 决策）。对抗用例：绕过 `feed_add` 直接写盘塞内网订阅 = 装载拒开, 拉取不到网络（`a_crafted_internal_address_table_refuses_to_open_and_never_fetches`）。

## 4. 测试证据

> 数字口径：`cargo test --workspace --locked -j 2 --no-fail-fast` 实跑输出（2026-10-13）；「套件」= 结果行（`test result:` 行数, 测试目标 180 + doc-tests 19）, 与 W3-2 / quick-capture 同口径。未实跑不发布数字。

**本批新增 35 例 / 2 套件**（`tools/tests/content_feed.rs` 集成 11 例、`runtime-assembly/tests/content_feed_registration.rs` 3 例, 其余 21 例散入既有 lib 套件）+ 2 处既有用例扩面（`self_status` 两节断言）:

| 验收项 | 测试（文件） | 例数 |
|---|---|---|
| 订阅源管理（增删列 / 重复·缺席拒 / 落盘信封） | `tools/tests/content_feed.rs::feed_management_add_list_remove_round_trip`、`content_feed/store.rs`：`add_list_remove_round_trip_persists_the_stored_doc`、`duplicate_ids_and_oversized_tables_are_refused` | 3 |
| 坏配置拒开（畸形 / 串档 / 未来版本 / 坏条目） | `content_feed.rs::bad_feeds_config_refuses_to_open_for_every_action`、`store.rs::missing_file_is_an_empty_state_but_broken_files_refuse_to_open`、`broken_entries_inside_an_envelope_refuse_to_open` | 3 |
| 拉取（mock 端点）→ 摘要结构化 → 入记忆流 | `content_feed.rs::pull_summarizes_and_ingests_without_putting_content_in_the_result`、`content_feed/summary.rs`：`rss_items_produce_structured_summaries`、`atom_entries_are_parsed_with_link_and_published_time`、`page_and_plain_text_summaries_truncate_and_label_honestly`、`hostile_boundary_forgery_in_content_is_stripped_to_plain_text`、`entity_unescaping_covers_the_closed_set` | 6 |
| 大小 / 超时 / 协议面拒 | `content_feed.rs::oversized_responses_are_refused_and_nothing_is_ingested`、`timeouts_are_reported_truthfully_and_nothing_is_ingested`、`protocol_surface_refusals_happen_before_any_network_contact` | 3 |
| 敏感面拒（凭据 / 内网 / 对抗写盘） | `content_feed.rs::sensitive_surfaces_are_refused_without_echoing_secrets`、`a_crafted_internal_address_table_refuses_to_open_and_never_fetches`、`store.rs::sensitive_and_invalid_urls_are_refused_not_approved` | 3 |
| 入记忆流（来源标注 / 封套披露 / 边界伪造隔离 / 台账如实） | `content_feed.rs::hostile_content_stays_quarantined_and_ledger_stays_honest`、`content_feed/tool.rs`：`episode_carries_source_attribution_and_the_untrusted_envelope`、`boundary_forgery_in_summary_stays_quarantined_inside_the_envelope`、`store.rs::pull_ledger_records_honestly_and_status_reports_it` | 4 |
| 未保护级 + EpisodeSink 同源同流（真存储） | `content_feed_registration.rs::pulled_content_lands_in_the_shared_episode_stream_unprotected` | 1 |
| 治理档语义（每次人批 / 写面标注 / 不误伤他能力 / fail-closed / 写面归类钉死） | `content_feed/mod.rs`：`every_content_feed_action_stops_at_require_approval`、`approval_reason_names_the_write_face_per_action`、`other_capabilities_are_untouched`、`store.rs::write_face_classification_is_pinned_per_action_family`、`tool.rs::write_face_classification_is_pinned_per_action` | 5 |
| 自述行（名册一行 + 订阅数 / 最近拉取如实） | `content_feed.rs::self_status_content_feed_section_reports_subscription_count_and_last_pull`、`content_feed_registration.rs::self_status_content_feed_section_reports_real_subscription_counts`、`content_feed_registers_only_when_configured_and_the_roster_says_so`、`self_status.rs::content_feed_section_reports_subscription_count_and_last_pull_honestly` | 4 |
| 描述符双面如实（guard） | `guard/src/semantics.rs::content_feed_descriptor_is_honest_about_its_dual_face` | 1 |
| 合同面（声明诚实 / 能力标识钉死） | `tool.rs::declaration_is_honest_about_governance_and_content_isolation`、`content_feed.rs::capability_identity_is_pinned` | 2 |

**全量实跑**（`cargo test --workspace --locked -j 2 --no-fail-fast`, 2026-10-13 终跑）：结果行 **199 套件 / 5359 例通过 / 1 失败 / 22 忽略**（测试目标 180 + doc-tests 19）。**J1 战场零失败**（tools + runtime-assembly + guard 全套件绿, 套件数含本批 +2）; **既有面零回归**。取证过程中出现过的红全部如实记账并隔离复跑定性（错误即帧）:

1. `apeireth-runtime --test canonical_architecture_invariants::execution_core_does_not_special_case_standard_modules`——取证时他线在途 `approval.rs`（+103 行）新测试夹具含字面量 `tool.shell`, 撞既有架构冻结护栏全文扫描; **隔离复跑已转绿**（其线已修夹具, 终跑绿）, 非本批引入亦非本批遗留;
2. `apeireth-sdk`（lib）`im::card::tests::approval_card_carries_the_family_annotation`——审批卡 family 标注线的在途自用例（`crates/adapters/sdk/src/im/card.rs:370`）, 隔离复跑仍红、终跑仍红, 归其线收口; J1 diff 零触碰 `engine/runtime` 与 `adapters/sdk`, 与本批无关。

**门禁其余四门**：`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` ✅ 全绿零警告（exit 0, 7m49s 实跑）、`cargo fmt --all -- --check` ✅ 干净、`scripts/check-neutral-terms.ps1` ✅ 零命中（115 变更文件）、`scripts/secret-scan.ps1` ✅ 无泄漏。

## 5. 缺口（如实, 不假装）

1. **摘要解析是启发式**：标签扫描不引外部解析器, 复杂嵌套 / 特殊转义形态可能取不全——兜底口径是「缺失如实标注」（无标题 =「无标题」, 来源没给时间 = `published_raw: null`）, 不编造;
2. **无定时拉取 / 后台刷新**：`pull` 是显式动作（每次人批）; 去重与订阅生命周期（去重账 / 失败退避 / 健康度 / 注意力上限）已随 **J2** 落地（§1.4）, 定时刷新归调度面（后续）;
3. ~~**无去重**~~ **J2 已补**：同条目重复 `pull` 走条目账 + 散列门两道去重, 零新记忆（§1.4）;
4. **视频源不在 J1**：owner 的「刷视频」场景需要媒体面 / 播放面 + 平台条款审读 + 防沉迷时长闸, 归 J3（铁律: 不做对抗性抓取、不绕平台风控）;
5. **内网主机名（非字面量）** 的静态拦截不在工具内（口径: 不做 DNS 决策）, 归受控通道解析后逐跳重验——J1 的 add 面只静态拦 IP 字面量;
6. **设置面板 / 热更未接**：旋钮目前是 env 面（`APEIRETH_ENABLE_CONTENT_FEED=1`）, 面板化随星港（I 轴）承载面批。

## 6. 分期

| 期 | 内容 | 依赖 |
|---|---|---|
| **J1（本批, ✅）内容流接入** | 订阅管理 + 受控拉取 + 结构化摘要 + 入记忆流 + 治理钉死 + 自述行（本文档 §1–§4） | 无 |
| **J2 订阅生命周期** | 去重与已读台账 / 失败退避 / 订阅健康度 / **注意力纪律上限**（单次·每日, 超限截断）✅ 已落地（§1.4）; 定时拉取·刷新 / 摘要正文清洗 / 拉取台账逐源历史 / 烧钱闸接 A 轴配额真接口归后续 | A 轴配额接口、调度面 |
| **J3 陪伴共看** | 智能预筛 / 陪看点评（模型面）、**视频源接入**（媒体面 + 来源条款审读 + 时长闸）、星港 UI 承载 | I 轴星港、模型面 H |
| **J4 群体面（可选）** | 匿名化订阅经验共享（星群纪律: 只共享经验, 不含记忆原文） | F 轴星群、共享口径 |

## 7. 铁律（随批验收）

不做对抗性抓取、不绕平台风控；来源条款不可接受的源不上架。命名即承诺：**J 生活接入轴**（内容流感知 → 记忆流, 跨源内容不直进提示词）。
