# R1 · 调研盘点
next=37。共 29 轮(R08–R36),cron ~2h,344 queries。

## 1 · R08–R31 主题
R08 ASI/FM · R09 自主 agent · R10 自改进+1M ctx · R11 Darwin Gödel · R12 ASI 真生产 · R13 MCP+Skills · R14 agentic hives · R15 Prigogine+Kauffman · R16 Schrödinger+Merleau · R17 Friston+范畴学 · R18 Penrose+Dennett · R19 Piaget+Hofstadter · R20 Canguilhem+Simondon · R21 Tarde+Latour · R22 哲学+控制论综合 · R23 生态+意识综合 · R24 分形+认知综合 · R25 Walker+Landauer · R26 Church+BZ · R27 Prigogine再+Maturana · R28 Rosen(M,R)+Friston再 · R29 Whitehead+Cajal · R30 Peirce+Husserl · R31 Fuller+Bateson

## 2 · 反复 Gap
显式重访:Prigogine R15/27 · Friston R17/28 · Schrödinger R16/32 · Latour R21/35 · Rosen R28/36 · Kauffman R15/36 · AlphaEvolve R11/22 · 1M ctx R10/12。主题簇:意识/具身≥6,控制论≥4,自组织≥4,认知生物≥5,社会理论≥4。
显著缺席(借鉴高价值):因果推断(Pearl do)0 · RL 基础(MuZero)0 · 形式化验证(Lean/TLA+)0 · 机制设计(VCG)0 · 计算最优律(Chinchilla)仅 R10/12 间接 · 记忆子工程(Letta/mem0/memoryos 在 code-deep-study 但 0 专轮) · 自改安全性(Rice/halting)0 · 编译器 IR(MLIR)0。

## 3 · R37 候选
C1 因果+Pearl do-calculus。why now:29 轮全无;与 Friston 互补;直喂 V1076 真 LLM 路由。产出:12×7(causal/cog/ML/哲学)+3 深读(dowhy/ananke/causal-learn)+2 Gap。
C2 RL+MuZero/Silver。why now:V1083 balanced policy 无理论骨架;R11 Gödel 互补;cookbook 缺 MuZero。产出:12×7(RL/cog/game-theory/neuro)+3 深读(muzero-general/cleanrl/SB3)+2 Gap。
C3 记忆子工程 Letta/mem0/memoryos。why now:V1072 永恒身份 0.8441=最低;AgentMemory L1–L4 已落;code-deep-study 现成 3 项目却 0 专轮。产出:12×7(memory/cog/DB/KV)+3 深读(letta/mem0/memoryos-rust)+2 Gap。**推荐:C3 优先**(对齐 V1072 最低,资料最丰);C1 备选(最稀缺);C2→R38。C3→C1 连推撬动"永恒身份"+"决策路由"两低分项。

— 调研专家