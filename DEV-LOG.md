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
