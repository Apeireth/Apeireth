# LoCoMo 真实数据评测报告 —— 记忆保留策略效用-成本曲线

> 日期: 2026-09-05 · 运行器: `research/runners` (apeireth-research-runner v0.1, release)
> 数据: LoCoMo (ACL 2024) `locomo10.json` + LoCoMo-MC10 (Percena mirror) —— **CC BY-NC 4.0, 仅限非商业研究**
> 判分: 确定性 evidence 命中 (0 LLM) · 显著性: bootstrap 95% CI (1000 重采样)

## 1. 实验设计

**任务**: 超长对话记忆保留 —— 在 token 预算约束下从对话历史轮次中选出保留子集,
使 QA 的证据轮次 (ground-truth evidence) 落在保留集中。

| 项 | 值 |
|---|---|
| 文档集 | LoCoMo locomo10: 10 会话 / 32 对话 / **1033 唯一轮次** (id=dia_id, tokens≈chars/4) |
| 查询集 (locomo) | **1986 条 QA**, relevant = evidence dia_id (数据集自带真值) |
| 查询集 (mc10) | mc10 JSONL 多选 QA, question 文本精确匹配 locomo10 借 evidence 真值 (见 §4) |
| 策略 | FixedWindow / RandomRetain / StackPinLite / VaultLruLite (runner v0.1 固定实现) |
| 预算档 | 2k / 4k / 8k / 16k / 32k tokens (每档独立跑; 全语料 ≈ 31k tokens) |
| 种子 | 42 (RandomRetain/StackPin 确定性复现) |

**效用定义**: 单轮成功 = 保留集 ∩ evidence ≠ ∅; 效用 = 成功率。成本 = 保留集 tokens。

**数据预处理 (2026-09-05 修正)**: locomo10 的 session_N 快照重复携带历史轮次
(实测 D1:3 出现 61 次, 原始拼接 5882 条含 5.7× 重复) —— 同一 dia_id 内容相同,
按首次出现去重为 1033 条, 等价于"提问时刻的记忆"模型。本文所有数字均为去重后口径。

## 2. 结果 (locomo, 1986 QA, 去重后 1033 文档)

| 策略 | B=2k | B=4k | B=8k | B=16k | B=32k |
|---|---|---|---|---|---|
| FixedWindow | 0.017 | 0.031 | 0.104 | 0.392 | 0.995 |
| RandomRetain | 0.096 | 0.189 | 0.346 | 0.611 | 0.995 |
| **StackPinLite** | **0.995** | **0.995** | **0.995** | **0.995** | **0.995** |
| VaultLruLite | 0.637 | 0.777 | 0.916 | 0.985 | 0.995 |

bootstrap 95% CI (效用差 vs FixedWindow, 同预算档) —— 全部显著非零:
- StackPinLite 在 B=2k 领先 **+0.972 ~ +0.985**; B=32k 时全部策略收敛 (全语料 31k tokens 恰好装下)。
- VaultLruLite 领先 +0.605 ~ +0.648 (B=2k) → +0.570 ~ +0.614 (B=16k)。
- RandomRetain 全档小幅领先 (B=2k: +0.058 ~ +0.084)。

**读图 (讲人话)**: 全语料只有 ~31k tokens, 32k 预算 = 全装下 (所有人 99.5%);
预算越紧, 结构策略越值钱 —— 2k 预算 (6% 语料) 下固定窗口 1.7%, 栈式保留 99.5%,
差距近 100 个百分点且 CI 不含 0。VaultLru 在低预算显著强于 RandomRetain/固定窗口,
弱于 oracle touch 的 StackPin (无 oracle 的公平对比待 LLM 接入)。

## 3. 结果 (locomo-mc10, 多选口径)

mc10 的 question 文本与 locomo10 QA **1986/1986 全部精确匹配** —— 两版数据同源
(mc10 是 locomo10 的 10 选 1 重打包), 借到的 evidence 真值与 §2 完全相同,
故曲线逐格一致 (见 §2 表)。**方法学结论**: mc10 不构成独立第二数据集,
其价值在于提供多选判分接口 (免 LLM-as-judge 即可测"保留→答对"的端到端链路),
该链路在 LLM judge 接上后使用; 本报告判分口径保持"证据轮次保留命中"。

## 4. 口径与已知偏差 (0 装, 论文必读)

1. **数据集结构偏向 recency**: LoCoMo 的 QA 紧跟对应对话提出, 证据轮次天然是"最近"的轮次,
   因此 recency 类策略 (FixedWindow 到 StackPinLite) 天然占优。跨数据集 (LongMemEval) 消融
   是后续工作; 本文结论措辞须限定为"recency-dominant 语料上的保留效用"。
2. **StackPinLite touch = oracle 上界**: touch 模拟直接用 QA 的 evidence 作为"本轮相关命中",
   等价于完美相关性反馈; 非可部署配置的公平估计。论文须标注为"recency + oracle relevance"
   上界消融; 公平对比需要模型自身检索命中的 touch (LLM/embedding, 待接)。
3. **VaultLruLite 是静态近似**: 固定权重打分器; FTRL 学习版在 workspace
   (`research_vault_ftrl.rs`, 遗憾界 ≤ 50√T 已验证) 未接入 runner —— 标注为静态基线。
4. **mc10 借标签**: mc10 不带 evidence 字段, 用 question 文本精确匹配 locomo10 QA 借真值,
   命中率 1986/1986 (100%, 两版同源); 借标签完备性已由全量命中事实验证。
5. **判分口径**: "证据轮次被保留" ≠ "模型能答对" —— 这是保留效用的代理指标;
   开放问答判分 (LLM-as-judge) 是下一步 (依赖 API key)。

## 5. 复现

```powershell
# 数据 (不入库, 自行下载): research/datasets/locomo/src (git clone snap-research/locomo)
#                          research/datasets/locomo-mc10/locomo_mc10.json (HF Percena/locomo-mc10)
cargo run --release --manifest-path research/runners/Cargo.toml -- --source locomo --seed 42
cargo run --release --manifest-path research/runners/Cargo.toml -- --source locomo-mc10 --seed 42
```

日志 (JSONL, schema 见 `research/logs/README.md`):
- `research/logs/locomo-retention-<hash>.jsonl`
- `research/logs/locomo-mc10-retention-<hash>.jsonl`
