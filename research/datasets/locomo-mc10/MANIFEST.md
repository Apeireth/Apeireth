# MANIFEST — LoCoMo-MC10(多选题版)

| 项 | 值 |
|---|---|
| 来源 | [huggingface.co/datasets/Percena/locomo-mc10](https://huggingface.co/datasets/Percena/locomo-mc10)(社区派生;1986 道多选) |
| 本地路径 | `research/datasets/locomo-mc10/`(`locomo_mc10.json` 240 MB JSONL,已加入 .gitignore) |
| **License** | **CC BY-NC 4.0(Attribution-NonCommercial)——仅限非商业研究使用** |
| 结构 | 每行:question / choices / correct_choice_index / haystack_sessions(完整对话嵌入,故文件大)/ question_id / question_type |
| 判分 | 确定性(多选题正确与否),0 LLM 依赖——开放问答版的替代起步口径 |

## 使用口径(0 装)

- 无 evidence 字段;证据定位需内容匹配(确定性代理口径)或 LLM 判分。
- 当前评测批优先用原始版(`../locomo/MANIFEST.md`,自带 evidence 真值);mc10 作为后续 QA 正确率补充。

## 首跑结果(2026-09-05, runner v0.1, `--source locomo-mc10`)

- question 文本与 locomo10 QA **1986/1986 全部精确匹配** → 两版数据**同源**
  (mc10 = locomo10 的 10 选 1 重打包)。借 evidence 真值后曲线与原始版逐格一致,
  详见 `../../reports/eval-locomo-2026-09-05.md` §3。
- 方法学结论: mc10 不是独立第二数据集; 其价值 = 免 LLM 的多选判分接口
  (保留→答对 端到端链路的判分器), 等 LLM judge 接上后使用。
