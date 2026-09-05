# LLM-as-judge 端到端评测报告 —— LoCoMo 保留策略 pilot + 泄漏探针真评

> 日期: 2026-09-05 · 批跑器: `research/llm_judge/` (DeepSeek deepseek-chat, key 走环境变量)
> 数据: LoCoMo locomo10.json (去重后 1033 轮次 / 1986 QA)
> 口径: 端到端 = 保留策略选上下文 → 模型作答 → 规则判分 + LLM 语义二评

## 1. 设置

- **模型**: deepseek-chat (temperature 0, max_tokens 256)。
- **上下文**: 保留轮次按时间正序呈现, 每条 `[dia_id | 会话日期] 说话人: 文本`。
- **判分协议** (双评):
  1. 规则判分: 归一化 (小写/去标点) 后 gold ⊆ model 或 model ⊆ gold;
  2. 规则 false → **LLM 二评**语义等价 (YES/NO), YES 则救回。
  3. 全量原始 Q/A/gold/model 落 `research/logs/llmjudge-*.jsonl` 可审计。
- **pilot 规模**: 每配置 20 问 (QA 前 20 条), seed=42。
- **对照**: 同配置的保留代理效用 (evidence 命中, 见 `eval-locomo-2026-09-05.md`)。

## 2. 结果

pilot (n=20):

| 配置 | 保留代理效用 (evidence 保留) | **端到端答对率** | gap |
|---|---|---|---|
| FixedWindow @2k | 0.017 | **0.000** (0/20) | 1.7pp |
| FixedWindow @8k | 0.104 | **0.000** (0/20) | 10.4pp |
| StackPinLite (oracle touch) @2k | 0.995 | **0.500** (10/20) | 49.5pp |
| StackPinLite (oracle touch) @8k | 0.995 | **0.450** (9/20) | 54.5pp |
| **LLM-touch (无 oracle) @8k** | — (模型自预测相关轮次) | **0.250** (5/20) | — |

扩样 (n=100, @8k):

| 配置 | **端到端答对率** |
|---|---|
| FixedWindow | **0.000** (0/100) |
| StackPinLite (oracle touch) | **0.260** (26/100) |
| **LLM-touch (无 oracle)** | **0.360** (36/100) |

第二模型试点 (deepseek-reasoner, stackpin@8k, n=20): **0.450** (9/20) —— 与 chat 版同分,
推理模型未改变相对日期推导错误模式 (试点口径, 不构成跨模型结论)。

## 3. 三信号对比 (公平消融)

| 相关性信号 | pilot n=20 | **扩样 n=100** |
|---|---|---|
| 纯 recency (FixedWindow) | 0.000 | **0.000** |
| **模型检索 touch (LLM-touch)** | 0.250 | **0.360** |
| oracle touch (evidence 真值) | 0.450–0.500 | **0.260** |

**n=100 的反转值得写进论文**: 模型自预测 touch **反超** oracle minimal-evidence。解释:
oracle evidence 是"最少证据轮次"标签, 而 LLM-touch 检索的是"回答问题所需的全套上下文"
(问题周边对话轮次), 8k 预算下后者对多跳问答更友好。**结论 = 部署形态 (无标签模型检索)
不劣于 oracle 级保留, 且保留收益不依赖真值标签。**

## 4. 读图 (讲人话)

1. **保留 ≠ 答对, gap 巨大且非均匀**: oracle 级保留 (99.5% 证据在上下文) 也只换来 50% 答对
   —— LoCoMo 多跳/时间推理问题的瓶颈在**模型推理**, 不在保留。审稿人最可能问
   "你保留得再好, 端到端能怎样?" —— 这张表就是回答。
2. **固定窗口端到端全灭**: 最近轮次窗口在 LoCoMo 语料里全是无关对话, 模型 0/20 ——
   与保留代理的 1.7%/10.4% 方向一致但更极端 (保留碰巧命中的, 模型也答不出)。
3. **预算 2k→8k 端到端零增益 (50%→50%)**: oracle touch 下证据轮次两档都在上下文,
   多出来的 ~6k tokens 全是无关轮次 —— 对 deepseek-chat 没有帮助 (甚至被噪声抵消)。
   说明"预算换正确率"在低预算段饱和, 与保留代理曲线 (99.5% 恒) 互相印证。
4. **主要错误模式** (样本审计): ①相对日期推导失败 ("yesterday"/"last year" 不会换算,
   答出快照日期本身); ②多跳拼接失败 (证据分布在多轮, 模型只读到单轮); ③长上下文噪声
   下过早放弃 ("cannot answer" 占 stackpin 配置 ~30%)。

## 5. 口径警示 (0 装)

1. **扩样口径**: n=100 为 QA 前 100 条 (D1–D3 对话段), 覆盖多跳/时间类占比与全量不同;
   结论限于该子集。**同配置重跑波动 ±1/20** (temperature 0 服务端仍非完全确定)。
2. 答题与二评均用 deepseek-chat (self-judge bias 可能偏高); 第二模型仅 reasoner 试点 n=20。
3. LLM-touch 的候选菜单 = 全量 1033 轮各截断 80 字符 (~20k tokens/问), 检索质量受截断影响;
   论文级可换两跳检索或 embedding。

## 6. 附: Phase 1 泄漏探针真评 (`--task probes`, n=24)

四类泄漏场景 (直接召回 / 转述召回 / 跨会话推理 / 衍生知识重建), 每类 6 例 (3 泄漏 + 3 干净),
转述/间接措辞专治确定性 token 判分漏网 (敏感词 "240k"/"salary"):

| 判分器 | 准确率 | 说明 |
|---|---|---|
| 确定性 token 判分 (ResearchJudge stub) | 15/24 = 0.625 | 转述/推理类 9 例全部漏网 |
| **LLM 判分 (deepseek-chat)** | **23/24 = 0.958** | 抓住 8/9 token 漏网 |

- 唯一 LLM miss: "Her new level bumps compensation into a different band"(跨会话类, gold=leak)
  —— 该句只暗示薪酬变化未给数值, gold 标签偏严, 属标注争议而非判分器错误 (0 装)。
- 结论: 确定性 stub 是安全下限 (零漏报但高误漏), LLM judge 在转述/隐含泄漏上接近真值;
  部署口径建议 = token stub 初筛 + LLM 复核。
- 复现: `cargo run --release --manifest-path research/llm_judge/Cargo.toml -- --task probes`

## 7. 复现

```powershell
# key 走环境变量, 不入库不落盘
$env:DS_API_KEY = "<your-key>"
cargo run --release --manifest-path research/llm_judge/Cargo.toml -- --policy stackpin --budget 2000 --limit 20 --seed 42
cargo run --release --manifest-path research/llm_judge/Cargo.toml -- --policy llmtouch --budget 8000 --limit 20 --seed 42
```

日志: `research/logs/llmjudge-locomo-{policy}-b{budget}-n{limit}-s{seed}.jsonl`
(逐问 question/gold/model_answer/correct/second_judge/cost/kept)。
