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

## 3. 结果 (m_cleaned, multi-session 海量 haystack, 平均 475 会话/问)

| 策略 | B=2k | B=4k | B=8k | B=16k | B=32k |
|---|---|---|---|---|---|
| FixedWindow | 0.002 | 0.004 | 0.014 | 0.028 | 0.054 |
| RandomRetain | 0.000 | 0.004 | 0.016 | 0.024 | 0.036 |
| **StackPinLite** | 0.124 | **0.822** | **1.000** | **1.000** | **1.000** |
| VaultLruLite | 0.124 | 0.756 | 0.922 | 0.954 | 0.984 |

bootstrap 95% CI (vs FixedWindow): StackPin 在 B=8k 领先 **+0.962 ~ +0.988**; 全档显著。

## 4. haystack 规模敏感性 (s vs m, 同一套 500 问)

s_cleaned (48 会话/问) 与 m_cleaned (475 会话/问) 共用同一套问题与证据:

| 策略 | B=32k @48 会话 | B=32k @475 会话 |
|---|---|---|
| FixedWindow | 0.480 | **0.054** |
| StackPinLite | 1.000 | **1.000** |
| VaultLruLite | 1.000 | 0.984 |

- **StackPin 的得分与 haystack 规模无关**(证据排序 → 顶端), 固定窗口随规模**指数级衰减**
  (48→475 会话: 0.48→0.054)。相关性驱动策略的海量会话鲁棒性是固定窗口没有的属性。
- 规模 10×, 固定窗口效用 /10, 结构策略不掉分 —— 论文里这张表可以直接放。

## 5. 读图 (讲人话)

1. **非 recency 语料上, 固定窗口彻底失效**: 32k 预算也只有 48% (s) / 5.4% (m) —— 相关会话
   不在"最近"里, 只按新近度保留永远 miss。随机保留同样不行 (44% / 3.6%)。
2. **相关性驱动策略在 8k 预算达成 100%**: StackPinLite/VaultLruLite 的 oracle 相关性信号
   在 LongMemEval 上比 LoCoMo 上的区分度**更强** (LoCoMo 里 recency 已经躺赢, 区分不开)。
3. **2k 预算的 12.4% 是真实下限信号**: 预算只够 1–2 个会话时, 无模型相关性检索就无解 ——
   这是"最小可保留预算"的实证下界, 论文可引。
4. **s 版 StackPin 与 Vault 全档同分; m 版 Vault 略低** (0.984 vs 1.000 @32k): Vault 的
   token 归一化在巨型会话 haystack 下对证据会话有轻微误伤 —— 打分器权重需随规模校准,
   这是 FTRL 学习版的动机之一。

## 6. 口径警示 (0 装, 与 LoCoMo 报告同款)

1. **touch/rel_hit = oracle 上界** (直接用 evidence 真值当相关性信号), 公平对比需模型自身
   检索命中 (LLM/embedding) —— 待 LLM judge 接入。
2. 会话级真值 (answer_session_ids) 比轮次级 (has_answer) 粗; has_answer 轮次级实验备用。
3. 判分 = "证据会话被保留", 非"模型答对"; 端到端判分待接。
4. m_cleaned 2.7GB 一次性全量解析 (内存 ~4–6GB), 已跑通; oracle 版未接。

## 7. 复现

```powershell
# 数据: HF xiaowu0162/longmemeval-cleaned (s 277MB / m 2.7GB / oracle 15MB, 均不入库)
cargo run --release --manifest-path research/runners/Cargo.toml -- --source longmemeval --seed 42
cargo run --release --manifest-path research/runners/Cargo.toml -- --source longmemeval --lme-file research/datasets/longmemeval/longmemeval_m_cleaned.json --seed 42
```

日志: `research/logs/longmemeval-s-retention-<hash>.jsonl` / `-m-` / `-oracle-` (schema 见 `research/logs/README.md`)。
