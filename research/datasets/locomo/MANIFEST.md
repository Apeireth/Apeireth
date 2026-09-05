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

## 首跑结果(2026-09-04, runner v0.1, seed=42, 预算档 2k/4k/8k/16k/32k tokens)

输入规模: **5882 docs(全部对话轮次)/ 1986 QA turns**。

| 策略 | B=2k | B=4k | B=8k | B=16k | B=32k |
|---|---|---|---|---|---|
| FixedWindow | 0.050 | 0.125 | 0.292 | 0.673 | 0.917 |
| RandomRetain | 0.133 | 0.224 | 0.400 | 0.591 | 0.803 |
| StackPinLite | **0.995** | **0.995** | **0.995** | **0.995** | **0.995** |
| VaultLruLite | 0.900 | 0.909 | 0.923 | 0.933 | 0.962 |

bootstrap 95% CI(效用差 vs FixedWindow)全部显著非零; StackPinLite 在 2k 预算下领先 +0.93~+0.95。

> ⚠️ **口径警示**: StackPinLite 的 touch 模拟使用了 QA 自带 evidence 作为"本轮相关命中", 等价于
> **oracle relevance feedback 上界**(理想检索), 不是可部署配置的无 oracle 估计。论文里应标注为
> "recency + 完美相关性反馈"消融, 同时补一组 touch=模型自身检索命中(需 LLM/embedding)的公平对比。
> 同样, VaultLruLite 的固定权重打分器是 FTRL 学习版的静态近似; 学习版结果见 FTRL 模块测试。
> LoCoMo 的 QA 紧跟在对应对话后提出, 证据轮次天然是最近轮次, 因此 recency 类策略本身占优 —— 这是数据集结构特性, 报告时须声明。
