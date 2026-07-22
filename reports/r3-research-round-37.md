# R3 · Round 37 调研报告
主题 C3 记忆子工程 (Letta/mem0/memoryos)。why now: V1072 永恒身份真测 0.8441=4 项最低 + code-deep-study 现成 3 项目却 24 轮 0 专深读 + AgentMemory L1-L4 已落。R1 推荐 C3 > C1 > C2。

## 12 Query (每 q 5 merged sources, 总 60)
1 Letta L1-L4 · 2 mem0 scalable · 3 memoryos hot/cold · 4 Memento · 5 hippocampal replay · 6 ACT-R/SOAR · 7 Vector DB RAG · 8 letta-ai/letta GH · 9 mem0ai/mem0 GH · 10 memoryos-rust GH · 11 scratchpad 失效 · 12 hot/cold aging。Top hits: Letta Docs, mem0ai/mem0, MemTensor/MemOS, Memento FT-no-LLM, ACT-R/Soar compare, LASCon condensation, Five-Tier Cascade, Oblivion decay。

## 3 跨域亮点
- Q3+10: MemTensor/MemOS 把 memory 升 OS 级 substrate (tiered hot/cold + Rust), 与 AgentMemory L1-L4 同构
- Q7: Pinecone/Qdrant/Weaviate 已 production RAG 标准, V1076 真 LLM 路由直接吃
- Q4+5+11: Memento (fine-tune 不动 LLM) + 海马回放 + LASCon scratchpad 压缩 = 三路攻 scratchpad 失效

## 2 Gap 借鉴点
- Gap A scratchpad 失效: LASCon loop-aware condensation + Memento retrieval without fine-tune
- Gap B hot/cold aging: Oblivion self-adaptive decay + Five-Tier Memory Cascade = age-decay policy 直落地 V1072

## 3 GitHub 真读项目
code-deep-study/ 本地 clone: letta-ai/letta (memGPT 后继 L1-L4 三层) · mem0ai/mem0 (universal layer, top stars) · MemTensor/MemOS 含 memoryos-rust (OS-style hot/cold + Rust)

## R38 主题建议
C1 因果推断 + Pearl do-calculus (R1 备选, 29 轮全无; V1076 路由无因果信号; dowhy/ananke/causal-learn 待深读)。

## cron 同步跳 record
cron-research-runs.jsonl 末 2 行: 20:54 `skipped` (r36 才 4min 前, <30min 阈值) → 21:08 `done` (R3-RES-02 手动触发, 12/12 AnySearch, no Bocha web/AI)。文件: research-v7-round-37.json 52403B + round-37-runner.py 5547B。边界遵守: 不动 R8-R36 JSON/runner, 仅生成 R37 + append cron。

— 调研专家 · R3-RES-02