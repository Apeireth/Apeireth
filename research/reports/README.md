# reports/

实验报告。每实验一份：`<experiment>-<date>.md`。

必含小节：
1. 问题定义与假设（可证伪陈述）
2. 设置（config hash、数据集、seed、环境、baseline 引用）
3. 结果（引用 logs/ JSONL 与指标）
4. 与 baseline 对比（四道门逐项）
5. 局限与失败判据核对
6. 学术账本（引用论文与 DOI；不确定标"待核实"）

## 索引

### Phase 交付报告（2026-09-04）

| 文件 | 内容 |
|---|---|
| `b2-phase1-delivery.md` | Phase 1 遗忘传播血缘（RA-1） |
| `b4-phase2-delivery.md` | Phase 2 BTFM 真双时态（RA-2） |
| `b3-phase3-delivery.md` | Phase 3 StackPin 上下文保留（RA-3） |
| `b5-phase4-delivery.md` | Phase 4 校准门控自治（RA-4） |
| `b6-phase5-delivery.md` | Phase 5 审批状态机形式化（RA-5） |
| `b7-b8-phase6-delivery.md` | Phase 6 CRDT + 非干扰（RA-6 部分） |

### 真实数据评测报告（2026-09-05）

| 文件 | 内容 |
|---|---|
| `eval-locomo-2026-09-05.md` | LoCoMo 1986 QA：效用-成本曲线 + bootstrap CI + 口径警示（recency 占优 / oracle 上界） |
| `eval-longmemeval-2026-09-05.md` | LongMemEval s/m 两版各 500 QA：非 recency 语料 + 每问独立 haystack + **haystack 规模敏感性消融**（FixedWindow 0.48→0.054, StackPin 恒 1.0） |
