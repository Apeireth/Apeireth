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
