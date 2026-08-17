# M2 图社区分层聚合 + 双级检索 — 自审报告

- 任务: 7633bd06-660a-4dfc-b1d5-197b4362e4e7（记忆调研批 ⭐, 台账 M2, 主人指示团队可干）
- 执行: fullstack_engineer2（N8 后接力, memory 域上下文热）
- 交付: `crates/apeireth-companion/src/community.rs`（新模块, 提交 26b00c29）+ lib.rs 注册 + `MemoryGraph::triage` 入口小改
- VCP/LightRAG/GraphRAG 精神对照: 图节点社区分层聚合 + 双级检索分诊（具体问题走实体链 local, 宽泛问题走社区摘要 global）

## 1. 实现要点（对照任务方向）

| 任务方向 | 实现 |
|---|---|
| ① 轻量社区检测（确定性, 不上 Leiden/外部图库） | ✅ s/p/o 值同事实共现 → 无向图; 连通分量 = 社区。纯 std（BTreeMap/BTreeSet）, 字典序遍历 → `comm-{i}` id 稳定（comm-0 恒含字典序最小节点）, 乱序输入产出相同划分 |
| ② 社区级滚动摘要 | ✅ 确定性版 = 社区内高频实体（subject/object）top-N（频次降序 → 字典序升序）+ 规模信息; 提炼调度口留 `Summarizer` trait 0 装（升级路径: LLM 提炼实现该 trait 即可替换） |
| ③ 双级检索分诊（确定性路由规则） | ✅ `triage`: 查询含实体（s/o 值, 长度≥2, 子串命中）→ Route::Entity（返回 matched_entities 字典序, 调用方续走实体链 CRAWL）; 无实体命中 → Route::Broad（社区摘要 brief, 事实数降序 → id 升序, 截 max_communities） |
| 边界: memory_graph 内新增模块优先 + 检索入口分诊小改 | ✅ 新模块 community.rs; memory_graph.rs 仅新增 `triage` 方法 6 行（调 active_facts + community::triage）; lib.rs 一行注册 |
| 不改 CRAWL 本体评分 | ✅ `crawl` 函数 0 改动（git diff 验证: memory_graph.rs 仅 +triage 方法） |

## 2. 验收结果

- **scratch 独立验证 6/6 全绿**（`rustc --test scratch_m2_verify.rs`: #[path] 原样挂载 community.rs 原文 + GraphFact 同形副本）:
  - empty_graph_paths（空图: 无社区; Broad 且 briefs 空不 panic; 空查询安全）
  - clustering_disjoint_and_bridged（两不相交簇 → 2 社区; 桥接事实 → 并 1 社区; 成员集合确定性比对）
  - deterministic_across_input_order（输入 reverse → 相同划分; 同输入二次运行逐字节一致）
  - deterministic_summary_topn_order（频次降序 → 字典序升序; trait 口与函数一致）
  - triage_entity_vs_broad（Entity 命中字典序; Broad briefs 按事实数降序; 复跑一致）
  - entity_match_rules（长度≥2 门槛防单字误命中; object 值可命中）
- **`cargo check -p apeireth-companion` 绿**（community.rs + triage 入口 + 注册行全部编译通过）
- **`cargo test -p apeireth-memory -j 4 --lib` 312/312 全绿**（验收字面命令; M2 代码在 companion crate, memory crate 无回归）
- 0 装 PASS: Summarizer trait 与 CRAWL 实接线均为口, 无真实 LLM/管线挂接

## 3. 诚实标注

- `cargo test -p apeireth-companion --lib`（正式全量）被团队并行 WIP 阻塞: 提交时 tool_bridge.rs:997 有他人 E0433（`apeireth_tool_search` 未链接, 36 行脏改动）, 随后轮转到 apeireth-credentials 7 个错误 — 均非 community.rs 报错（全程 0 错）。与 N3/N8 同模式: 待树收敛后 QA 复跑终验, 届时 scratch_m2_verify.rs 可删。
- 首次 scratch 跑出 1 个测试断言修正（comm-0 归属判断误设"小明", 实为字典序最小"位于"所在服务器簇）— 已改为集合序无关确定性断言, 修正后 6/6。该过程如实记录, 非一次性完美。

## 4. 后续升级路径（0 装已备）

- LLM 社区提炼: 实现 `Summarizer` trait 替换 `DeterministicSummarizer`（接口不变）
- 查询管线实接线: `MemoryGraph::triage` 已挂, assemble 注入侧按需调用（Entity → crawl 种子, Broad → briefs 注入）
