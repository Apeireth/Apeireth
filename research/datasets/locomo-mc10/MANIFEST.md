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
