# MANIFEST — LoCoMo(原始版)

| 项 | 值 |
|---|---|
| 来源 | [github.com/snap-research/locomo](https://github.com/snap-research/locomo)(ACL 2024, *Evaluating Very Long-Term Conversational Memory of LLM Agents*) |
| 本地路径 | `research/datasets/locomo/src/`(git clone --depth 1;`src/` 已加入 .gitignore) |
| 数据文件 | `data/locomo10.json`(10 会话 / 32 对话 / **1986 条 QA**,每条带 `evidence` 真值标签 = 对话轮次 `dia_id` 列表) |
| **License** | **CC BY-NC 4.0(Attribution-NonCommercial)——仅限非商业研究使用**;论文学术用途 OK,商用需另行授权 |
| 规模 | locomo10.json 2.74 MB;对话轮次 {dia_id, speaker, text} |
| 分割 | 无官方 train/test 分割;评测批按会话留出(session-level split)自行划分 |
| 污染检查 | 未做(数据集自带标签,评测为"从完整对话中按预算保留证据轮次",无训练成分) |

## 使用口径(0 装)

- 本仓库评测运行器的 `--source locomo` 将每条 QA 映射为:预算内保留的证据轮次是否命中 → 记忆保留效用(确定性判分,0 LLM)。
- 开放问答判分(需 LLM)与 mc10 多选版见 `../locomo-mc10/MANIFEST.md`。

## 首跑结果(2026-09-05, runner v0.1, seed=42, 预算档 2k/4k/8k/16k/32k tokens)

输入规模: **1033 唯一轮次**(去重后; 原始快照拼接 5882 条含 5.7× 重复, 见下)/ 1986 QA turns。

| 策略 | B=2k | B=4k | B=8k | B=16k | B=32k |
|---|---|---|---|---|---|
| FixedWindow | 0.017 | 0.031 | 0.104 | 0.392 | 0.995 |
| RandomRetain | 0.096 | 0.189 | 0.346 | 0.611 | 0.995 |
| StackPinLite | **0.995** | **0.995** | **0.995** | **0.995** | **0.995** |
| VaultLruLite | 0.637 | 0.777 | 0.916 | 0.985 | 0.995 |

bootstrap 95% CI(效用差 vs FixedWindow)全部显著非零; StackPinLite 在 2k 预算下领先 +0.97~+0.99。
全语料唯一轮次 ≈ 31k tokens, 32k 档 = 全装下, 策略收敛。

> ⚠️ **数据修正 (2026-09-05)**: locomo10 的 session_N 快照重复携带历史轮次
> (D1:3 出现 61 次), 原始拼接 5882 条 → 按 dia_id 去重 **1033 条** (同一 dia_id 内容相同,
> 去重 = "提问时刻的记忆"模型)。首版报告曾用 5882 口径, 已全部更正为去重后口径。

> ⚠️ **口径警示**: StackPinLite 的 touch 模拟使用了 QA 自带 evidence 作为"本轮相关命中", 等价于
> **oracle relevance feedback 上界**(理想检索), 不是可部署配置的无 oracle 估计。论文里应标注为
> "recency + 完美相关性反馈"消融, 同时补一组 touch=模型自身检索命中(需 LLM/embedding)的公平对比。
> 同样, VaultLruLite 的固定权重打分器是 FTRL 学习版的静态近似; 学习版结果见 FTRL 模块测试。
> LoCoMo 的 QA 紧跟在对应对话后提出, 证据轮次天然是最近轮次, 因此 recency 类策略本身占优 —— 这是数据集结构特性, 报告时须声明。
