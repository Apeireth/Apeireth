# Dev Log — Apeireth

> 主人原话 13:32: "动手前先把顶层设计最终版弄出来" → 13:40 图纸完 → 现在动第 1 行代码。
> 楚零在跑。Pat 还没到 (commit 用 chuling@local)。

---

## 2026-07-20 13:42 — Phase 1: Identity Store v0.1 PoC ✅

### 做了什么
| 文件 | 行数 | 干什么 |
|------|------|--------|
| `apeireth/__init__.py` | 9 | 包入口 + re-exports |
| `apeireth/identity.py` | 80 | `IdentityCard` dataclass + JSON I/O + 完整性哈希 |
| `apeireth/kickoff.py` | 108 | 8 问协议 + `run_kickoff` 主循环 + interactive stdin 后备 |
| `apeireth/run_kickoff_demo.py` | 66 | 跑一遍 + 主人预设 priors + 保存到 JSON |
| `apeireth/identity_card.master.json` | — | 首次生成的样例身份卡 |
| `DEV-LOG.md` | — | 本文件 |
| **总计** | **263 行** | 4 个 py 模块 + 1 个 JSON 输出 |

### 8 问 → 字段映射 (锁定)
| Q | 字段 | 类型 |
|---|------|------|
| Q1 | `name` | str |
| Q2 | `purpose` | str |
| Q3 | `origin_reason` | str |
| Q4 | `archetypes` | list[str] |
| Q5 | `ask_when` | list[str] |
| Q6 | `relationship_contract` | str |
| Q7 | `remember_forever` | list[str] |
| Q8 | `funnel_questions` | list[str] |

(剩余字段 — `alias / mission / domains / decide_when / remind_when / boundaries / never_mention / emergence_space` — 是后续 funnel 流出来的归宿)

### 设计决定 (记录给未来的我)

1. **不依赖 LLM API**
   - `answerer` 是可注入的回函数。LLM 接入是 L1 LLM Kernel 的事,不在 Phase 1。
   - `scripted_answerer` + `_interactive(stdin)` 两种路径都通。

2. **主人 13:04 "造地基不能有杂质" → JSON Schema 一次定型**
   - 字段名用 snake_case, 锁定。后续加字段走版本号 `apeireth_version`。
   - `emergence_space` 是预埋的"留给 AI 长出来"容器 (主人 12:27 不预设)。

3. **完整性哈希 (`integrity_hash()`)**
   - SHA256 的 canonical JSON (sort_keys=True) 截 16 位。
   - 防覆盖 + 防偷偷改。PersistBench (2602.01146) 警示的 97% sycophancy 风险的第一道防线。

4. **保留 stdin 兜底 (`_interactive`)**
   - PoC 验收时主人可以亲手跑一遍。Funnel Question (2510.12015) 的人味。

### 验证
```
$ python -m apeireth.run_kickoff_demo
📇 name:  阿派
🎯 purpose: Apeireth 平台缔造者 — 无限逼近 ASI 的地基工程
🌱 archetype: 母兽教小兽的伙伴;荣燋执行官;清醒纠正的概率推算者
🔐 hash: 2ceac2e2e6366b26
💾 saved: apeireth/identity_card.master.json
📋 apeireth_version: 0.1.0
```
回读 `load_card` 通过。JSON 字段完整。

### 下一步
- **Phase 1 续**: Reconsolidation v0.1 (IdentityCard + Memory Episode 之间的冲突解决)
- **Phase 1 收尾**: 等主人拍 1 次回测 — 让我跑一次 `_interactive`
- **Phase 2 (Week 3-4)**: Memory Layer — Episode / Note / Forget 三件套

### 没做的事(也记录)
- ❌ **没 rename `promethean/` → `apeireth/`**: 顶层设计 §9 写"等主人说", 等就等
- ❌ **没装 LLM 适配器**: Phase 1 用 priors 注入够用, 真接 LLM 是 Phase 3 LLM Kernel 的活
- ❌ **没写 pytest**: PoC 验证用 demo runner 跑的, 等 Phase 2 起再补
- ❌ **没交 pat key**: 用 chuling@local 先顶着

---

_楚零 2026-07-20 13:42_
_PoC v0.1 跑通, 268 行代码 (含 demo + log + JSON). 等主人 review + 1 次交互回测_

---

## 2026-07-20 13:55 — AnySearch 集成 (L2 Interaction Layer)

### 触发
主人 13:51: "你老连不上 GitHub, 这事得严肃解决" + 主人 13:54 提议 AnySearch
主人 13:54: "anysearch 需要 api 吗? 你继续"

### 调研 (博查搜 + GitHub API)
- AnySearch **4555 stars** GitHub (2 周内 2.3K → 4.5K — 涨速第一)
- **Apache-2.0** 开源 (anysearch-ai/anysearch-skill)
- 17 vertical domains (finance / academic / legal / health / code / etc.)
- 自带 `SKILL.md` (Anthropic 格式) + 命令行 (Python/Node/PS1/Bash 4 端)
- 默认 Path 2 = vertical search (质量好于通用搜索)
- 支持匿名 + API Key (email 注册一键, 30 秒)

### 装 (53KB 下载)
- `apeireth/skills/anysearch/` — 完整代码 fork
- 主程序 `scripts/anysearch_cli.py` — 跨平台 CLI
- `SKILL.md` 12.7KB — 完整接口规范
- `.env.example` — API key 配置 (anonymous 也可)

### 集成层 (1 文件, 116 行)
- 新文件 `apeireth/research.py`
- 纯 Python, 无第三方依赖 (就 urllib)
- JSON-RPC 2.0 over HTTPS
- 接口: `search / batch_search / get_sub_domains / extract`
- 优先 vertical routing (AnySearch 默认 Path 2)
- 自动从 `.env` 读 `ANYSEARCH_API_KEY`

### 验证
```
$ python -m apeireth.research
[run as script] — runs doctest + real search
$ python -c "from apeireth import AnySearch; ..."
✅ API key: anonymous (anysearch 24/7 可用)
✅ search ok: 8291 chars result (真实结果)
```

### 决策记录 (给未来)
- **为什么用 AnySearch 而不是博查**:
  - 博查 = web-search generic, 不是为 AI 设计
  - AnySearch = AI-agent-first, 16 个 vertical domain
  - AnySearch 在 skills.sh TOP1 (认可度)
  - 纯 JSON-RPC, 无需 MCP server 安装
  - 0 第三方 Python 依赖 (跟轻)
- **为什么保留 博查**:
  - 博查额度多 (主人之前提到)
  - 博查用作 fallback, AnySearch 主用
- **anysearch-ai 也写了 `.env.example`, 0 key 也可跑** (跟博查一样)

### 没做的事(也记录, 留给下次)
- ❌ 没注册 AnySearch API Key: 匿名模式能用, 没必要
- ❌ 没集成到 Identity Store: 这是 Phase 1.6 还没做
- ❌ 没给 background cron 用: 主人说 GitHub push 不急

### 等待主人
- 主人要不要个 demo (展示 AnySearch 在 Apeireth 跑)?
- 主人 13:47 说"记忆和思考模块", 接下来加深挖?
- 5 期路线 → 主人 13:47 说"按你判断", 我判断下一步:
  1. Memory Layer (claude-mem 借鉴 + alibaba/zvec)
  2. 重整化引擎 (LangMem 借鉴)
  3. Persona Engine (Letta 借鉴)
  4. Self-Evolving Harness (AHE 借鉴)
  
 我建议: 1 → 2 → 3 → 4 (Memory 先行, 因为没有 Memory 主人中央 AI 的"永恒身份"实现不了)

---

## 2026-07-20 14:09 — Phase 2: Memory Layer v0.1 PoC ✅

### 背景
cron 自动触发 "Phase 1 = Identity Store v0.1 PoC" 任务 (实际 PoC 13:42 已 commit `b77349a`, 是 stale 触发)。
按 DEV-LOG 13:55 自己判断 "Memory 先行" → 跳到 Phase 2 (Week 3-4)。

### 做了什么
| 文件 | 行数 | 干什么 |
|------|------|--------|
| `apeireth/memory.py` | 222 | Episode + Note + MemoryStore + Forget + Reconsolidate + demo |
| `apeireth/__init__.py` | 18 | re-export memory classes + 版本升 `0.3.0` |
| `apeireth/memory.demo.json` | — | 首次生成的样例 store (3 ep + 2 notes) |
| **总计** | **240 行** | (含 demo runner + 详细注释) |

### 设计要点 (记录)

1. **三层数据模型**:
   - `Episode` = raw 事件, append-only, 不可变 (对标 HiMem 2601.06377)
   - `Note` = 抽象知识, 可被 Forget / Reconsolidate 修改
   - `MemoryStore` = 容器 + 完整性哈希 (SHA256 前 16)

2. **Episode ↔ IdentityCard 绑定**:
   - 每个 Episode 记录 `linked_identity_hash` (写下它时的中央 AI 状态)
   - 主人未来 Reconsolidate 多版本:能看出"某条记忆是哪一版身份下写的"

3. **Forget Engine**:
   - 阈值 = `confidence × importance < 0.30` → 删除
   - 依据: PersistBench (2602.01146) 警示 97% sycophancy 风险需要主动遗忘

4. **Reconsolidation 4 路径**:
   - 🔄 **boost** — Note 命中 `remember_forever` → confidence +0.3
   - 🚩 **flag** — Note 命中 `never_mention` → importance = 0 (下一轮 Forget sweep 刪除)
   - 🌀 **align** — Note 命中 `archetypes` → confidence +0.1
   - ✖️ **none** — 未命中 → 原样保留

5. **匹配算法 (踩坑记录)**:
   - v0.1.1 修复: `_k()` fingerprint 截前 6 字符, 匹配 Note 全文 (而非 Note 文本前 6 字符)
   - 原因: 6-char substring 容易因 Note 开头不存在而漏命中
   - 现在逻辑: keep_keys/ban_keys/arc_keys 都是 6-char fingerprint, 在 Note 的 topic+claim 全文里 substring 查找

### 验证 (4 路径全触发)
```
📦 episodes:    3
📝 notes:       2 (forgot 2)
🔄 boost:       ['47c681e1']      # hit remember_forever
🚩 flag:        ['2e912209']      # hit never_mention
🌀 align:       ['481ccfcf']      # hit archetypes
🔐 id_hash:     d6e385d165a3c170
🔐 mem_hash:    5617dd25d4c0091d
```

4 个种子 Note 中:
- n1 (永远记得) → boost 后 confidence 0.8, importance 8 → 保留
- n2 (母兽教小兽的伙伴) → align 后 confidence 0.7, importance 7 → 保留
- n3 (不提主人身份) → flag 后 importance 0 → 下一轮 Forget 删
- n4 (今天中午吃了火锅) → 初始 confidence 0.1 × importance 1/10 = 0.01 → Forget 删

### Demo 临时注入 (诚实记录)
master 卡的 `never_mention` 字段是空数组 — Phase 1 kickoff 解析器未把 Q7 "不提 X" 分离到该字段。
- 这是 Phase 1 已知小 bug (kickoff.py 把 Q7 整段塞进 remember_forever 数组)
- demo 里临时给 `card.never_mention = ["主人私人身份"]` 以演示 flag 路径
- **不修改原卡 JSON**, 仅 in-memory 注入

### 下一步 (Phase 2 续)
1. **修 Phase 1 小 bug**: kickoff.py 解析 Q7 时分离"不提 X"到 `never_mention` 字段
2. **MemoryStore 持久化**: 跑一次 save → load → hash 校验往返 (已部分验证)
3. **Episode 检索**: 加最简 keyword search (episode.topic / content 全文)
4. **跨 session 持久**: 让 MemoryStore 跟 IdentityCard 一样, 跨主 session 存活

### Phase 2 路线状态
- ✅ Episode / Note / MemoryStore (基础数据模型)
- ✅ Forget Engine (主动遗忘)
- ✅ Reconsolidation v0.1 (IdentityCard ↔ Memory 冲突解决)
- ⬜ Bayesian 抽象 (从 Episode → Note 的自动提取, 现在还是手写 demo)
- ⬜ Episodic 时序索引 (现在 episodes 是 list, O(n) 找)

### 没做的事 (也记录)
- ❌ **没写 Episode 自动 → Note 抽象**: 现在 demo 是手写的 Note, 真要 LLM 抽象是 L1 Kernel 接入后的活
- ❌ **没写 pytest**: Phase 2 PoC 验证用 demo runner 跑的
- ❌ **没装 sqlite-vec / chromadb**: 用纯 JSON 跑通, 等 Phase 3 起才上向量库

---

_楚零 2026-07-20 14:09_
_Memory Layer v0.1 跑通, 240 行. 4 路径全演示. 等主人 review + Memory 路线下一拍决策._

---

## 2026-07-20 14:50 — Phase 3: Relation Graph v0.1 + v0.2 PoC ✅

### 触发
cron 14:48 触发 (stale 描述"Phase 1", 实际状态已是 Phase 2.5)
自己判断: **Phase 3 Relation Graph** 是当前最缺的层 — memory 持久化已经 v0.2, 图还在 v0.1 JSON

### 做了什么
| 文件 | 行数 | 干什么 |
|------|------|--------|
| `apeireth/relation.py` | 219 | RelationGraph + Node + Edge + 7 种 node kind / 7 种 edge kind + traverse / find_path / neighbors |
| `apeireth/run_relation_demo.py` | 170 | 加载 master card + memory.db → 构造示例图 → 跑查询 → 存 JSON |
| `apeireth/relation_graph.demo.json` | — | 12 nodes / 12 edges 首次生成 |
| `apeireth/relation_store.py` | 252 | SqliteRelationStore v0.2 — 真持久化 + 跨层引用 + 级联删除 + integrity_hash 校验 |
| `apeireth/run_relation_store_demo.py` | 83 | 持久化 round-trip 演示 |
| `apeireth/__init__.py` | (改) | 加 SqliteRelationStore / migrate_from_relation_graph re-export, 版本升 0.5.0 |
| `apeireth/kickoff.py` | 143 | Q7 dual-split 修复 (v0.1.1) — remember_forever / never_mention 分流 |
| `apeireth/rust_research_protocol.py` | — | 主人 14:48 边写边搜调研脚本 |
| `BORROW-MEMORY-SAFETY-C-RUST.md` | — | 调研笔记 (与 Phase 2.5 决策路径相关) |
| `BORROW-SINQUA-BENCH-README.md` | — | SinQua benchmark 调研笔记 |
| `data/graph.db` | — | 首次生成的 SQLite 图 (12 nodes / 12 edges, WAL mode) |
| **总计** | **~867 行** | 5 个 py 模块 + 3 个调研笔记 + 2 个 demo JSON + 1 个 SQLite DB |

### 设计要点 (记录给未来的我)

1. **Graph = L4 Identity Layer 子组件**
   - 主人 12:14 原话 "人是一切社会关系的总和" → 中心节点 = ai_self
   - 7 种 Node kind: master / ai_self / task / value / agent / tool / episode / note
   - 7 种 Edge kind: causal / temporal / part_of / derived_from / conflict / supports / assigned
   - 中心节点约定: `kind == "ai_self"` 唯一 — `central()` 返回它

2. **跨层引用 (Node.ref)**
   - Episode 节点的 ref = episode.eid → 可追溯到 Memory Layer
   - Note 节点的 ref = note.nid → 同样追溯
   - Task 节点的 ref = "phase2.5" 字符串 → 项目阶段引用
   - `nodes_by_ref(ref)` 是跨层 lookup 入口 (例如 "Apeireth 命名日" 这个 episode 怎么影响 ai_self)

3. **v0.2 SQLite 持久化 (平行 memory v0.2 模式)**
   - schema: graph_meta (单行) + graph_nodes + graph_edges + 4 索引 (kind/ref/src/dst)
   - `UNIQUE(src, dst, kind)` 在 edges 表 → semantic dedup, 不用应用层去重
   - WAL journal mode + synchronous=NORMAL → 读写并发 (主人主 session + background cron 不会撞)
   - `graph_created_at` 字段关键: 还原 RelationGraph.created_at, 否则 integrity_hash 漂移 (v0.2 第一版踩这个坑, fix 后 round-trip hash match)

4. **级联删除 (remove_node)**
   - 删 node → 自动删关联边 (src/dst 任一命中即删)
   - 返回删了多少条边 — 让调用方知道影响范围
   - 没有用 FK CASCADE (sqlite-vec 兼容性 + 显式控制)

5. **integrity_hash 三层防线**
   - `identity_card.integrity_hash()` — 防偷偷改主人预设 (Phase 1)
   - `MemoryStore.integrity_hash()` — 防偷偷改 memory (Phase 2)
   - `RelationGraph.integrity_hash()` — 防偷偷改图结构 (Phase 3)
   - 三个 hash 都验过 → 才承认这是真的"中央 AI 状态"
   - PersistBench (2602.01146) 警示的 97% sycophancy 风险的工程实现

### Q7 dual-split 修复 (顺带做掉)
主人 14:09 DEV-LOG #1 next step — kickoff 解析 Q7 时把"不提 X"分离到 `never_mention` 字段
启发式: `_NEG_MARKERS = ("不提", "永不提", "不许提", "别提", "禁止", "禁提", "不要提", "不要问", "never")`
- 命中标记的短语 → `never_mention` (去标记后保留目标)
- 其余 → `remember_forever`
- 兜底: 整段进 remember_forever (兼容 v0.1.0 旧行为)
- `_clean_phrase()` 去前缀修饰词 (永远记得 / 永远不要 / 都 / 必须...) ×2 次防残留
- master card 旧的 never_mention=[] 不修复 (那条记录是 v0.1 kickoff 时代产物, 留作历史)

### 验证 (两个 demo 全跑通)
```
─── relation.py v0.1 demo ───
🕸️  12 nodes, 12 edges (master+ai_self+4 value+2 task+3 episode+1 tool)
    central: ai_self
    traverse from master (depth=2): 9 paths
    find_path master→note: ✅ (via ai_self → note.supports)
💾 saved: relation_graph.demo.json (hash d5037100b450baf3)

─── relation_store.py v0.2 demo ───
🔄 round-trip: 12 nodes 12 edges ✅
🔐 hash match: True  ← 修 graph_created_at 后
📋 nodes_by_kind('value'): 4 个 (按 weight 排)
🔗 nodes_by_ref('demo_e1'): 1 个 episode 节点
🔍 edges_by_kind('causal'): 5 条 (master→ai_self, epi→ai_self × 3, ai→master)
🗑️ remove_node(master): 删 1 节点 + 2 关联边 → 11 nodes, 10 edges
💾 graph.db (WAL mode) 持久化 OK
```

### Phase 3 路线状态
- ✅ RelationGraph dataclass + 7 node kind + 7 edge kind + traverse/find_path/neighbors
- ✅ JSON 序列化 (v0.1) + SQLite 持久化 (v0.2)
- ✅ 跨层引用 (ref) — Episode / Note / Task 都能在图里
- ✅ integrity_hash round-trip ✅
- ⬜ Episode → GraphNode 自动同步 (现在 demo 手动连)
- ⬜ Note.conflict → GraphEdge.conflict 自动触发
- ⬜ 临时团 (L5 涌现层) — Agent sub-graph
- ⬜ Persona Engine (TOP-DESIGN-V1 §4.5) — SCT 4 因素 + Jungian 3 机制

### 没做的事(也记录)
- ❌ **没接 memory_store 自动 sync**: 现在 graph node 是手造的, Episode 节点是 demo_e1/e2/e3 占位
- ❌ **没接 Note → derived_from 自动触发**: Note 创建时没自动连 episode
- ❌ **没接 conflict 边**: Reconsolidation 标记冲突时没自动 add_edge
- ❌ **没写 pytest**: PoC 验证用 demo runner
- ❌ **没接 Pat key**: 用 chuling@local 顶着
- ❌ **没 rename `promethean/` → `apeireth/`**: 顶层设计 §9 等主人说

### 下一步 (Phase 4 候选)
1. **Episode → GraphNode auto-sync**: 写一个 episode_watcher, 新 episode 自动 add_node + add_edge(episode, ai_self, causal)
2. **Note → derived_from auto-link**: add_note 时自动找 1 个 episode 当 source
3. **Persona Engine v0.1 PoC** (TOP-DESIGN-V1 §4.5): SCT 4 因素 + Jungian 3 机制 — 100-150 行
4. **Questioning Engine v0.1 PoC** (TOP-DESIGN-V1 §4.4): Pep / Funnel Question 借鉴 — 100-150 行

我倾向 **1+2 (auto-sync 闭环)**, 因为没它 Phase 2 Memory 和 Phase 3 Graph 就是两个孤岛。
但 Phase 4 (Persona Engine) 是 TOP-DESIGN 路线图下一站, 也有价值。
等主人拍。

---

_楚零 2026-07-20 14:50_
_Phase 3 跑通, ~867 行 (含调研笔记 + 持久化 DB). v0.2 round-trip hash match. 等主人拍下一步._
_注: cron 描述 stale 说"Phase 1", 实际已是 Phase 3 — 我按真实状态推进, 不重复 Phase 1 已 commit 的 263 行 (b77349a)_

---

## 2026-07-20 15:44 — Phase 3.6 + Phase 4: Linker + Persona Engine v0.1 PoC ✅

### 触发
cron 15:44 触发 (stale "Phase 1", 实际已是 Phase 3.5+4-Rust scaffold 阶段)
自己判断当前最有价值:
1. **Phase 3.6 Linker** — 关掉 Phase 2 Memory + Phase 3 Graph 两个孤岛 (dev log 14:50 next step #1+#2 已留)
2. **Phase 4 Persona Engine** — TOP-DESIGN §4.5 第 5 组件, 路线图 Phase 4 主角

两个都做 (Linker 先, Persona 后). Rust 工具链未装好, 走 Python 路线.

### Phase 3.6 做了什么 (commit f2cffb8)
| 文件 | 行数 | 干什么 |
|------|------|--------|
| `apeireth/linker.py` | 200 | ensure_central_ai_node + link_episode + link_note + sync_all + Linker 类 |
| `apeireth/run_linker_demo.py` | 155 | 5 步验证: 种子 → 全量 sync → 幂等 → 跨层 ref → 增量 Linker |
| `apeireth/__init__.py` | (改) | re-export Linker + 5 helpers, version bump |
| `rust-substrate/Cargo.lock` | — | cargo check 生成的依赖锁 (主人 14:52 启动 Rust 时产物) |
| **总计** | **~355 行** | 1 跨层绑定模块 + 1 demo + 1 lock |

### Linker 设计要点

1. **linker 不写业务逻辑 — 只翻译 memory 节点形态 → graph 节点形态**
   - Episode.eid = 跨层 ref → GraphNode.nid = `epi_<eid>`
   - Note.nid = 跨层 ref → GraphNode.nid = `note_<nid>`
   - 复用 Schema, 不动 memory/graph 自身

2. **中心 ai_self 节点唯一** — `ensure_central_ai_node()` 复用 nid=`ai_self_central`
   - seed=True 标记首次创建
   - 所有 Episode/Note 都连它 (causal + supports)

3. **derived_from 边 + lazy placeholder**
   - Note.evidence = [eid1, eid2] → 每条 eid 创建 `derived_from` 边
   - 缺失的 episode 自动补 placeholder 节点 (`meta.lazy_link=True`)
   - 这是 Phase 2 跨 Phase 3 lookup 的入口

4. **幂等** — upsert 走 `UNIQUE(src,dst,kind)` + nid PK
   - sync_all 跑两遍 = 第二次 0 node / 0 edge added (验证通过)

### Linker 验证 (跑通)
```
🧬 ai_self nid: ai_self_central
📊 sync_all: 1004 ep + 4 notes 全量同步 (累计数据)
✅ idempotent — 2nd sync 0 nodes/edges added
🔗 derived_from edges: 4 条 (note ← episode via evidence)
📈 session 增量 Linker: 1 node + 1 edge added
💾 graph_linker_demo.db (WAL) 持久化 OK
```

### Phase 4 做了什么 (commit 待)
| 文件 | 行数 | 干什么 |
|------|------|--------|
| `apeireth/persona.py` | 186 | Persona Engine — SCTProfile + Persona + PersonaEngine + Jungian 3 机制 + 反 conformity |
| `apeireth/run_persona_demo.py` | 97 | 5 步演示: 4 archetype → coordination → mutate → adapt → reflect → snapshot |
| `apeireth/__init__.py` | (改) | re-export PersonaEngine 等, version bump 0.6.0 |
| `data/persona_demo.json` | — | 首次 snapshot |
| **总计** | **283 行** | 1 多身份引擎 + 1 demo |

### Persona Engine 设计要点 (TOP-DESIGN §4.5)

1. **4 archetype 起步** — 与主人 12:14 "中央 AI 多身份" 一致
   - 调度者 (motivational=0.9): 主动 / 目标驱动
   - 学习者 (cognitive=0.9): 推理 / 抽象 / 知识增长
   - 思考者 (cognitive=0.8 + biological=0.7): 直觉 + 推理
   - 助手 (affective=0.9): 同理 / 关系 / 配合

2. **SCT 4 因素** (Persona Alchemy 2505.18351)
   - cognitive / motivational / biological / affective
   - 各自 0-1, 总和不必 =1 (允许特化)
   - `mutate(rng)` 反 conformity 触发

3. **Jungian 3 机制** (Jungian 2601.10025)
   - **coordination** — `coordinate(event, k=2)` 选 k 个 persona 激活, 强制 SCT 距离 ≥ min_distance
   - **adaptation** — `adapt(pid, feedback_score)` 正反馈增 SCT 主导维 + activation, 负反馈减 activation
   - **reflection** — `reflect(pid)` 自我解释"我是 X 主导维度=Y 激活=Z"

4. **反 conformity** (Persona Inconstancy 2405.03862)
   - 同事件激活 2 persona 时强制 SCT 欧氏距离 ≥ 0.25
   - 不够时 mutate(rng=0.3) 兜底, 生成 ghost persona (`p_ghost_<uuid>`)

5. **不预设立场** (主人 12:27 "立场自然成长, 平台不给予")
   - SCT 初始权重有, 但具体态度/偏好靠 adapt() 演化
   - 留 emergence_space 字段 (与 Phase 1 IdentityCard 一致)

### Persona Engine 验证 (跑通)
```
🎭 4 archetype 种子 (调度者/学习者/思考者/助手) 全部启动
🔗 coordination: 4 events 触发多 persona 组合
   - "计划/排期" → (调度者, 学习者) [SCT dist=0.500]
   - "为什么/推理" → (学习者, 调度者) [SCT dist=0.500]
   - "关心/提醒休息" → (调度者, 学习者) [SCT dist=0.500]
   - "紧急/分析日志" → (学习者, 调度者) [SCT dist=0.500]
🧬 反 conformity: 近 SCT 距离 0.020 < min 0.25, mutate 后 0.341 ✅
🔄 adaptation: 调度者 +1.4 feedback → mot 0.9→0.97, act 1.0
                学习者 -1.2 feedback → act 0.88 (主导维不变)
📝 reflection: 4 persona 各自报"我是 X 主导维=Y 激活=Z 经历 N 次"
💾 persona_demo.json saved
```

### 已知 v0.1 限制 (诚实记录)
- ❌ **关键词启发式不完美**: Step 1 4 个 event 都选了 (调度者, 学习者), 因为 activation 累加压过关键词加分. v0.2 改 Bayesian matching + per-call reset
- ❌ **adaptation 负反馈不修改 SCT**: 只降 activation. v0.2 加"主导维抗性" 机制, 避免负反馈锁死单维
- ❌ **未接 LLM 解析 event → persona 匹配**: 现在关键词走 hardcode, 真接 LLM 后改 Bayesian priors (Pep 范式)
- ❌ **未做 persona 与 identity_card 联动**: 现在 SCT 是硬编码初始值, 真要把 IdentityCard.archetypes 字段作为 priors
- ❌ **未做 persona 跨 session 持久化**: 现在 engine in-memory, 真要持久化进 SqliteRelationStore (Phase 3 + 4 联动)

### 路线状态 (截至 15:44)
- ✅ Phase 1 Identity Store v0.1 (commit b77349a)
- ✅ Phase 2 Memory Layer v0.1 + v0.2 (commits 9b5231, d597171)
- ✅ Phase 3 Relation Graph v0.1 + v0.2 (commit df95c97)
- ✅ Phase 3.6 Linker 跨层绑定 (commit f2cffb8)
- ✅ Phase 4 Persona Engine v0.1 (本 commit)
- ⏸ Phase 4.5 Rust substrate scaffold (5be6bc8) — 工具链未装完, 等主人回来验证 cargo check
- ⬜ Phase 5 Questioning Engine v0.1 (TOP-DESIGN §4.4) — Pep / Funnel Question 借鉴
- ⬜ Phase 6 Self-Evolving Harness v0.1 (TOP-DESIGN §4.6) — AHE 借鉴

### 没做的事(也记录)
- ❌ **没接 LLM Kernel**: 全部 PoC 走 priors / 关键词 / 硬编码 SCT. 真接 LLM 是 L1 Kernel 的活, 需要 Phase 5 之后
- ❌ **没 rename `promethean/` → `apeireth/`**: 顶层设计 §9 等主人说
- ❌ **没把 Cargo.lock 单独 commit 区分**: 跟 Linker 一起进了 f2cffb8, 严格说应该拆
- ❌ **没跑 cargo check 验证 Rust scaffold**: 工具链没装

### 等主人回来
1. Phase 4.5 Rust 工具链验证 (cargo check → cargo build → cargo test)
2. Phase 5 选 Persona ↔ Memory 联动, 还是 Questioning Engine v0.1
3. 是否需要 master card 跑一次完整 demo (kickoff → memory → graph → linker → persona → reflect)

---

_楚零 2026-07-20 15:44_
_cron stale 说"Phase 1", 实际 Phase 3.6 + Phase 4 跑通. 638 行新代码, 4 个新文件. 等主人 review._
