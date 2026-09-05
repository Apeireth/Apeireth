# MANIFEST — LongMemEval

| 项 | 值 |
|---|---|
| 来源 | [xiaowu0162/LongMemEval](https://github.com/xiaowu0162/LongMemEval)(ICLR 2025, *LongMemEval: Benchmarking Chat Assistants on Long-Term Interactive Memory*); 数据镜像 [HF xiaowu0162/longmemeval-cleaned](https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned) |
| 本地路径 | `research/datasets/longmemeval/`(`longmemeval_s_cleaned.json` 277MB / `longmemeval_m_cleaned.json` 2.7GB / `longmemeval_oracle.json` 15MB; 均已 gitignore) |
| **License** | **MIT**(原作者 Di Wu) —— 商用友好 |
| 结构 | 500 条 QA × 独立 haystack:`question` / `answer_session_ids`(会话级证据真值) / `haystack_session_ids` / `haystack_sessions`(逐轮 {role, content, has_answer?} —— **has_answer 为轮次级真值, 备用**) / `question_type` |
| 判分 | 确定性证据会话命中, 0 LLM |
| 与 LoCoMo 的对比价值 | **QA 不紧跟对应对话提出**(haystack 跨数月), recency 不再占优 —— 独立第二数据集, 见 `../../reports/eval-longmemeval-2026-09-05.md` |

## 使用口径

- 运行器 `--source longmemeval`(默认读 s_cleaned; `--lme-file` 可换文件)。
- 每条 QA 自带文档宇宙(haystack 会话), 与真实"每用户独立记忆"语义一致;
  会话 id 取自 `haystack_session_ids`。
- m_cleaned(multi-session, 2.7GB)已下载并已跑; oracle 版已下载未接 adapter。
