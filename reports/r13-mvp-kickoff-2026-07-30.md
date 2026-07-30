# R13 MVP 启动报告 (Phase 0 + Phase 1 第 1 周)

**Date**: 2026-07-30
**Task ID**: T9 (b02f5c4c-8ea8-4f7c-84c4-748900444209)
**Author**: fullstack_engineer
**授权来源**: 用户最高权 ("R11 收尾给的建议你不必全盘接受。你有最高权推进项目。")
**Master HEAD pre-R13**: `41583321 feat(r12-deploy-monitor)` (T8)
**R13 HEAD**: TBD (本 commit)

---

## 1. 执行摘要 (Executive Summary)

启动 **R13 MVP** 子项目 —— 基于用户最高权授权，突破 R11 收尾"选项 A：文档化收尾"的保守建议。

**Phase 0 (清场 + 骨架) + Phase 1 第 1 周 (跨 session 记忆存储层)** 已完成。

| 指标 | 结果 |
|---|---|
| 新增目录 | `mvp/` (不修改 Apeireth 任何 v*.py) |
| 新增文件 | 12 files, +1104 insertions |
| 测试 | 11/11 PASS (1.17s) |
| CLI 验证 | 关→重开 上下文完整, 中文 BM25 检索命中 |
| 依赖 | click + rich + pytest (无 langchain/autogen/letta) |
| 数据库 | SQLite 3.50.4 (Python 3.13 stdlib) |
| 硬性约束 | 100% 守住（见 §5） |

---

## 2. 用户最高权授权的边界

### 2.1 用户原始指令（最高权）

> "R11 收尾给的建议你不必全盘接受。你有最高权推进项目。"

**R11 收尾建议**："选项 A：文档化收尾 + 单节追加。不动工程，不合 master，留给下一团队。"

**用户授权突破**：可以新建 `mvp/` 子项目，可以 commit engineering。

### 2.2 主人哲学硬约束（仍尊重，不破）

- ❌ 不重写 V0.5 公式（§5.E 红线）
- ❌ 不重做 V1136 真测引擎
- ❌ 不重写哲学守门
- ❌ 不刷 KPI（主 17:43）
- ❌ 不假装达到 ASI（主 17:58 + 主 20:46）
- ✅ 实事求是（主 17:43 + 主 17:58）
- ✅ 干到底（主 23:44 + 主 23:09）

### 2.3 R11 收尾建议接受 / 拒绝

| 建议 | 决策 |
|---|---|
| 不动工程 | **接受**（mvp/ 是新增，不是改 Apeireth） |
| 不合 master | **拒绝**（用户最高权：commit mvp/ 合 master） |
| 留给下一团队 | **接受 + 拓展**（T9 自己实现 + 留后续 commit 给 Phase 2-4） |
| 文档化收尾 + 单节追加 | **接受 + 拓展**（R12 团队已写附录 N，T9 写本启动报告） |

---

## 3. mvp/ 子项目结构

```
mvp/
├── README.md                  (148 行, 项目说明 + 跑法 + 路线图)
├── pyproject.toml             (26 行, click + rich + pytest)
├── cli.py                     (165 行, --new-session/--resume-session/--chat/--recall)
├── memory/
│   ├── store.py               (274 行, Episode/Note/Session 三表 + 滚动保留)
│   └── retrieve.py            (153 行, LIKE-based 简化 BM25 + salience decay)
├── identity/
│   └── card.py                (150 行, IdentityCard JSON + 演化)
└── tests/
    └── test_memory.py         (188 行, 11 tests)
```

**依赖**：click 8.1+ / rich 13.7+ / pytest 8.0+ / Python 3.11+ / SQLite 3.9+（FTS5 builtin）

**核心模块功能**：

| 模块 | 功能 |
|---|---|
| `mvp/memory/store.py` | SQLite 三表 + Episode 滚动 200 条 + Note merge/forget |
| `mvp/memory/retrieve.py` | LIKE 扫描 + 简化 BM25 (term_freq/sqrt(len)) + 1/(1+Δt/τ) salience |
| `mvp/identity/card.py` | JSON 持久化 IdentityCard + 主人真实背景种子 + evolve() |
| `mvp/cli.py` | click 4 子命令 + REPL (`add/recall/note/whoami/bye`) |

**Ponytail 选择 (主 19:33 真借鉴)**：

1. **FTS5 → LIKE 扫描**：Python 3.13 sqlite3 3.50.4 unicode61 不分中文（CJK 字符 0 hits）。改用 LIKE + char-level tokenize (中文 char + 英文 word)。数据量 ≤200 episodes 性能可接受。
2. **BM25 简化版**：去掉 IDF 部分（数据量小无意义），保留 "term_freq / sqrt(doc_length)" 核心。
3. **Salience decay τ**：Episode 1 天 (86400s)，Note 7 天 (604800s) — Note 是 consolidate 后的提炼不该快速 decay。
4. **无 LangChain / Letta**：stdlib (sqlite3 + json + dataclass) + click + rich，零框架绑定。

---

## 4. 跨 session 记忆验证 (主 23:09 干到底)

### 4.1 测试 (11/11 PASS)

```
mvp/tests/test_memory.py::test_episode_append                       PASSED
mvp/tests/test_memory.py::test_episode_rolling_window               PASSED
mvp/tests/test_memory.py::test_session_id_autocreation              PASSED
mvp/tests/test_memory.py::test_note_consolidation                   PASSED
mvp/tests/test_memory.py::test_note_forget_low_confidence           PASSED
mvp/tests/test_memory.py::test_fts5_bm25_retrieve                  PASSED
mvp/tests/test_memory.py::test_salience_decay                       PASSED
mvp/tests/test_memory.py::test_time_window_filter                   PASSED
mvp/tests/test_memory.py::test_identity_card_evolution              PASSED
mvp/tests/test_memory.py::test_cross_session_persistence           PASSED
mvp/tests/test_memory.py::test_retrieve_notes_long_half_life        PASSED
11 passed in 1.17s
```

### 4.2 CLI 实际验证 (CliRunner, 模拟进程重启)

```
=== Session 1: new + add ===
exit: 0
out: new session: demo

[Store API 写入]
- Episode 1: user "主人是地方的"
- Episode 2: agent "(echo) 收到, 已记录"
- Episode 3: user "少数民族语翻译是我的测试场"
- Note: "主人关心养老问题" confidence=0.8

=== Session 2: resume (新 CliRunner = 新进程) ===
exit: 0
out:
  resumed session: demo
  recent episodes (3):
    [user] 主人是地方的
    [agent] (echo) 收到, 已记录
    [user] 少数民族语翻译是我的测试场

=== Session 3: recall 少数民族语 (BM25 中文) ===
exit: 0
out: [bm25=0.58 sal=1.00] [user] 少数民族语翻译是我的测试场

=== Session 4: recall 养老 ===
exit: 0
out: (no match — note 不在 CLI recall 范围, 仅 chat mode 查 note)
```

**核心验证点**：
- ✅ Session 2 在新 CliRunner 实例（= 进程重启）成功恢复，看到 Session 1 写入的 3 条 episode
- ✅ 中文 BM25 检索命中 ("少数民族语" → 1 hit, bm25=0.58)
- ✅ SQLite 持久化（db 文件写在 `mvp/data/test.db`）

---

## 5. 不做什么 (硬性约束守住)

- ❌ **不修改** `apeireth/v*.py`（1100+ 个 v 模块全部保留）
- ❌ **不修改** `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`（6546 行手册）
- ❌ **不修改** R12 已 commit 的 6 个 commit (12eeb9e8 / d67304a9 / 85074cf4 / b42c802b / 41583321 / 5bdf998d)
- ❌ **不重写** V0.5 公式 / V1136 真测引擎 / 哲学守门（§5.E 红线）
- ❌ **不写 ASI 北极星公式**（外部评估建议：MVP 验证用主人哲学，不用自设指标）
- ❌ **不引入** langchain / autogen / letta（主人哲学"借鉴而非闭门"）
- ❌ **不删**空壳模块（主人硬约束）
- ❌ **不 commit** 其他 30+ working changes（保留各自主权）

**mvp/ 是新增子项目**，全部 +1104 insertions，0 deletions on existing files。

---

## 6. R13 后续路线图

| Phase | 周 | 内容 | 状态 |
|---|---|---|---|
| **Phase 0** | 第 0 周 | mvp/ 骨架 + README + pyproject | ✅ 本次 |
| **Phase 1.1** | 第 1 周 | 存储层 (Episode + Note + 简化 BM25 + salience) | ✅ 本次 |
| Phase 1.2 | 第 2 周 | 提取层 (对话 → Note 提炼 + 合并 / 遗忘策略) | ⏳ |
| Phase 1.3 | 第 3 周 | 演化层 (IdentityCard 从 Note 周期 consolidate) | ⏳ |
| Phase 1.4 | 第 4 周 | 检索增强 (混合 LIKE-BM25 + char-ngram) | ⏳ |
| Phase 2 | 第 5-6 周 | 真实 LLM 接入 (OpenAI / Claude / 本地) | ⏳ |
| Phase 3 | 第 7 周 | 主人实测 (连续 7 天每天 1 次, 主观 > 7/10 通过) | ⏳ |
| Phase 4 | 后续 | TUI / 飞书 / 多用户 | ⏳ |

---

## 7. Commit Metadata (本次)

```
commit (TBD)
Author: fullstack_engineer

feat(r13-mvp): R13 MVP Phase 0 + Phase 1.1 跨 session 记忆存储层

12 files changed, 1104 insertions(+)
  - mvp/README.md (148 行)
  - mvp/pyproject.toml (26 行)
  - mvp/cli.py (165 行)
  - mvp/memory/store.py (274 行)
  - mvp/memory/retrieve.py (153 行)
  - mvp/identity/card.py (150 行)
  - mvp/tests/test_memory.py (188 行, 11/11 PASS)
  - mvp/{__init__,memory/__init__,identity/__init__,tests/__init__,tools/__init__}.py

验证:
- 11/11 tests PASS in 1.17s
- CliRunner 模拟进程重启: resume + 中文 BM25 检索命中

基于用户最高权授权 ("你有最高权推进项目"), 突破 R11 收尾保守建议.
主人哲学硬约束守住 (不重写 V0.5 / V1136 / 哲学守门, 不刷 KPI, 不假装 ASI).
mvp/ 子项目是新增, 不修改 apeireth/v*.py 也不动 6546 行手册.
```

---

## 8. 与 R12 收尾的关系

R12 接手**双轨并行**：
- **R12 轨**：文档化收尾 + 接续 commit (T1-T8 全部完成, 附录 N appended, §5.C 4 项 + §5.D #2 全部闭合)
- **R13 轨（本次）**：MVP 新路径（mvp/ 子项目, 跨 session 记忆, 主人实测为成功标准）

**R12 团队未关闭工程**（附录 M §5.C 列表，R13 不承接，按主人"选项 A 不修"）：
- W2/W4 dashboard 闭合
- V1121 fake-KPI detector dashboard yellow
- V1077 v0.4 dims_filled（**R12 commit 12eeb9e8 + d67304a9 已修** ✅）
- master → integration 合并收尾
- V1130 wallclock 7-11s → 2.5s target（远未达）
- V1136 子测度失败

R13 不接管这些。R12 团队 / 后续 commit 自行处理。

---

## 9. 一句话总结

R13 MVP 启动 ✅ —— Phase 0 骨架 + Phase 1.1 跨 session 记忆存储层 11/11 测试 PASS，CLI 验证关→重开上下文完整 + 中文 BM25 检索命中，基于用户最高权授权但不破主人哲学硬约束，mvp/ 是新增 +1104 insertions 不修改 Apeireth 任何 v*.py。

**Task T9 完成 ✅**。