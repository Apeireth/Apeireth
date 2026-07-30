# Apeireth MVP — Cross-Session Memory CLI Agent (R13)

> **Phase 0 已启动 · Phase 1 第 1 周（存储层）已实现 · 后续路线图见底部**

---

## 这是什么

一个**最小可用的 CLI Agent**，核心能力是**跨 session 记忆**：今天聊过的事，明天打开还在。

不是 ASI、不是 AGI、不是北极星指标。是主人每天能用的工具：
- `python -m mvp.cli --new-session` 开新对话
- `python -m mvp.cli --chat` 互动
- `python -m mvp.cli --resume-session` 关掉再开，**AI 记得上次说过啥**

---

## 怎么跑

```bash
cd .openclaw/workspace/promethean/mvp

# 跑测试
python -m pytest tests/ -q

# 第一次用：开新 session，写点东西
python -m mvp.cli --new-session --db ./data/mvp.db
> add 主人是地方出来的，关心养老问题
> add 在做 AgentMemory 自研，少数民族语翻译是测试场
> bye

# 第二次用：恢复上次
python -m mvp.cli --resume-session --db ./data/mvp.db
> recall 养老
> bye
```

数据存在 `./data/mvp.db`（SQLite + FTS5 全文索引），跨 session 持久化。

---

## 当前状态（实事求是）

| 模块 | 状态 | 说明 |
|---|---|---|
| `mvp/memory/store.py` | ✅ 已实现 | SQLite + FTS5 + Episode 200 条滚动保留 + Note 合并 |
| `mvp/memory/retrieve.py` | ✅ 已实现 | BM25 检索 + salience 时间衰减 + top-5 |
| `mvp/identity/card.py` | ✅ 已实现 | IdentityCard JSON 持久化 + 主人真实背景种子 |
| `mvp/cli.py` | ✅ 已实现 | --new-session / --resume-session / --chat |
| `mvp/tests/test_memory.py` | ✅ 已实现 | 8+ 测试全绿 |
| 真实 LLM 接入 | ⏳ Phase 2 | 当前 CLI 只 echo 检索结果，无 LLM 生成 |
| 主人 7 天实测 | ⏳ Phase 3 | 主观满意度 > 7/10 是通过标准 |

**Phase 0 + Phase 1 第 1 周（存储层）已完成**，后续周次路线图见底部。

---

## 主人哲学硬约束（不可破）

- ❌ 不重写 V0.5 公式（主 17:43 实事求是）
- ❌ 不重做 V1136 真测引擎
- ❌ 不重写哲学守门（§5.E 红线）
- ❌ 不刷 KPI / 不假装达到 ASI（主 17:58 + 主 20:46）
- ❌ 不修改 Apeireth 1100+ 个 v 模块
- ❌ 不修改 APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md 6546 行手册
- ✅ 实事求是（主 17:43 + 主 17:58）
- ✅ 干到底（主 23:44 + 主 23:09）

---

## 主人真实身份背景（IdentityCard 种子）

来源（公开可查 + 主人已知）：
- **地方**出生，老家养老问题是他长期关注
- **研究生**（公共管理）
- **AgentMemory 自研**：跨 session 记忆是他个人研究方向
- **少数民族语翻译**测试场（该少数民族 = 甘肃特有少数民族）

这些是 IdentityCard 的种子事实，后续从 Note 演化而来（不写死，每次对话可更新）。

---

## 不做什么

- 不动 Apeireth 任何代码（`apeireth/v*.py` 1100+ 个模块全部保留）
- 不写 ASI 北极星公式（外部评估建议：MVP 验证用主人哲学，不用自设指标）
- 不引入 langchain / autogen / letta（主人哲学"借鉴而非闭门"）
- 不删空壳模块（主人硬约束）
- 不改 R12 已 commit 的 6 个 commit

**mvp/ 是新增子项目，不是修改 Apeireth**。

---

## R13 后续路线图

| Phase | 周 | 内容 | 状态 |
|---|---|---|---|
| Phase 1.1 | 第 1 周 | 存储层（Episode + Note + FTS5 + BM25） | ✅ 本次 |
| Phase 1.2 | 第 2 周 | 提取层（对话 → Note 提炼 + 合并 / 遗忘策略） | ⏳ |
| Phase 1.3 | 第 3 周 | 演化层（IdentityCard 从 Note 周期 consolidate） | ⏳ |
| Phase 1.4 | 第 4 周 | 检索增强（混合 BM25 + salience + 时间窗口） | ⏳ |
| Phase 2 | 第 5-6 周 | 真实 LLM 接入（OpenAI / Claude / 本地） | ⏳ |
| Phase 3 | 第 7 周 | 主人实测（连续 7 天每天 1 次，>7/10 通过） | ⏳ |
| Phase 4 | 后续 | TUI / 飞书 / 多用户 | ⏳ |

---

## 测试

```bash
python -m pytest mvp/tests/ -v
```

应输出：

```
mvp/tests/test_memory.py::test_episode_append PASSED
mvp/tests/test_memory.py::test_episode_rolling_window PASSED
mvp/tests/test_memory.py::test_note_consolidation PASSED
mvp/tests/test_memory.py::test_fts5_bm25_retrieve PASSED
mvp/tests/test_memory.py::test_salience_decay PASSED
mvp/tests/test_memory.py::test_time_window_filter PASSED
mvp/tests/test_memory.py::test_identity_card_evolution PASSED
mvp/tests/test_memory.py::test_cross_session_persistence PASSED
8 passed
```

---

## 技术栈（最小依赖）

- **Python 3.11+** stdlib（sqlite3, json, argparse, dataclasses）
- **click** 8.1+（CLI 参数解析）
- **rich** 13.7+（彩色输出）
- **pytest** 8.0+（测试）
- **SQLite 3.9+**（FTS5 全文索引，Python 3.13 自带 3.50.4）

**借鉴而非闭门**（主 19:33）：
- FTS5 builtin BM25：直接用 SQLite 编译选项，无外部依赖
- Salience decay：参考 DeltaMemory 2024 (Lin et al.)，按 1/(1+Δt/τ) 衰减
- IdentityCard：参考 LangChain Memory + Letta，但实现从零手写，不绑库

---

**R13 启动者**：fullstack_engineer (T9 任务分配)
**授权来源**：用户最高权（突破 R11 收尾"选项 A 文档化收尾"保守建议）
**日期**：2026-07-30