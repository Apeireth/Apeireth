# LongMemEval 真实数据评测报告 —— 记忆保留策略效用-成本曲线

> 日期: 2026-09-05 · 运行器: `research/runners` (apeireth-research-runner v0.1, release)
> 数据: LongMemEval (ICLR 2025) `longmemeval_s_cleaned.json` —— **MIT License**
> 判分: 确定性证据会话命中 (0 LLM) · 显著性: bootstrap 95% CI (1000 重采样)
> 配套: `research/datasets/longmemeval/MANIFEST.md` · 对照: `eval-locomo-2026-09-05.md`

## 1. 与 LoCoMo 的关键差异 (为什么值得跑)

| 维度 | LoCoMo | LongMemEval |
|---|---|---|
| QA 与证据的时间关系 | QA 紧跟对应对话 → **recency 天然占优** | QA 跨数月提出 → **recency 不占优** |
| 文档宇宙 | 全局共享 (5882 轮) | **每条 QA 独立 haystack** (平均 48 会话/问) |
| 证据粒度 | 轮次 (dia_id) | 会话 (answer_session_ids) + 轮次 (has_answer, 备用) |
| License | CC BY-NC 4.0 | **MIT** |

LongMemEval 是**独立第二数据集**: 能验证"结构保留策略的优势是否只是 LoCoMo 的 recency 假象"。

## 2. 结果 (500 QA, seed=42)

| 策略 | B=2k | B=4k | B=8k | B=16k | B=32k |
|---|---|---|---|---|---|
| FixedWindow | 0.020 | 0.064 | 0.136 | 0.268 | 0.480 |
| RandomRetain | 0.020 | 0.062 | 0.126 | 0.204 | 0.442 |
| **StackPinLite** | 0.124 | **0.822** | **1.000** | **1.000** | **1.000** |
| VaultLruLite | 0.124 | **0.822** | **1.000** | **1.000** | **1.000** |

bootstrap 95% CI (效用差 vs FixedWindow, 全部显著非零, 除 RandomRetain 个别档):
- B=2k: StackPin/Vault 领先 **+0.078 ~ +0.130**
- B=4k: **+0.664 ~ +0.744**
- B=8k: **+0.806 ~ +0.872** ← 峰值
- B=16k: +0.662 ~ +0.744; B=32k: +0.452 ~ +0.540
- RandomRetain 与 FixedWindow 无显著差异 (随机保留 ≈ 固定窗口, 都不懂相关性)

## 3. 读图 (讲人话)

1. **非 recency 语料上, 固定窗口彻底失效**: 32k 预算也只有 48% —— 相关会话不在"最近"里,
   只按新近度保留永远 miss。随机保留同样不行 (44%)。
2. **相关性驱动策略在 8k 预算达成 100%**: StackPinLite/VaultLruLite 的 oracle 相关性信号
   在 LongMemEval 上比 LoCoMo 上的区分度**更强** (LoCoMo 里 recency 已经躺赢, 区分不开)。
3. **2k 预算的 12.4% 是真实下限信号**: 预算只够 1–2 个会话时, 无模型相关性检索就无解 ——
   这是"最小可保留预算"的实证下界, 论文可引。
4. **StackPinLite 与 VaultLruLite 全档同分**: 两策略的 oracle 相关性项都来自 evidence,
   排序等价; 说明本口径下区分度在"相关性 vs 无相关性", 不在两种结构化策略之间。
   学习版 (FTRL, 无 oracle) 的对比是下一步 (需模型检索信号)。

## 4. 口径警示 (0 装, 与 LoCoMo 报告同款)

1. **touch/rel_hit = oracle 上界** (直接用 evidence 真值当相关性信号), 公平对比需模型自身
   检索命中 (LLM/embedding) —— 待 LLM judge 接入。
2. 会话级真值 (answer_session_ids) 比轮次级 (has_answer) 粗; has_answer 轮次级实验备用。
3. 判分 = "证据会话被保留", 非"模型答对"; 端到端判分待接。
4. 只跑了 s_cleaned (single-session); m_cleaned (multi-session) 未接。

## 5. 复现

```powershell
# 数据: HF xiaowu0162/longmemeval-cleaned → longmemeval_s_cleaned.json (277MB, 不入库)
cargo run --release --manifest-path research/runners/Cargo.toml -- --source longmemeval --seed 42
```

日志: `research/logs/longmemeval-retention-9bd6a144.jsonl` (schema 见 `research/logs/README.md`)。
