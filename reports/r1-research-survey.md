# R1 Research Survey — 33 轮方向候选 (next=41)
round_auto_naming --json: next=41, conflict=false, round-41 free

## 1. 33 轮主题 (R08-R40)
- R08-R12 ASI基线: breakthrough / long-horizon / self-improving / Apeireth arch / recursive
- R13-R14 真生产: MCP spec+Skills hot-reload / agentic hives+Ashby
- R15-R19 哲学群: Prigogine / Schrödinger+Friston+Yoneda / Penrose+Dennett / Piaget+Hofstadter
- R20-R24 哲学群: Canguilhem+Simondon / Tarde+Latour / Sapir-Whorf+Beer VSM / Connell IDH+Taleb / Mandelbrot+Watts
- R25-R31 哲学群: Walker+Landauer / Church+Adamatzky / Prigogine+Maturana / Rosen+Friston / Whitehead+Cajal / Peirce+Husserl / Fuller+Bateson
- R32-R36 哲学续: Schrödinger+Popper / Polanyi+Foucault / Stiegler+Marx / Arendt+Latour / Rosen+Kauffman
- R37 记忆子工程: Letta/mem0/memoryos/Memento/hippocampal/ACT-R/Vector-DB + 3 GH 真读
- R38 因果+Pearl: do-calc/CBN/SCM/Hall-Winston/DoWhy/ananke/EconML (Q1+Q11 双空)
- R39 自我进化: NAS/continual/meta/ShinkaEvolve/ASI-Arch/langgraph
- R40 感知/具身: active-inference/4E/affordance/GWT/world-model/CLIP/Whisper/Perceiver-io

## 2. 反复 Gap
- G1 哲学多工程少: R08-R36 共 29 轮 ~348 query 全哲学叙事,工程名反复列无落地
- G2 真读≠真复刻: R37-R40 已到源码级,缺映射 V1090+ 模块的可执行表
- G3 因果空白: R38 Q1 do-calc+Q11 反事实幻觉 双空,可抢首发
- G4 RL/Sutton 零覆盖: MuZero/TD/世界模型 33 轮 0 关键词,V1083 policy 缺背书
- G5 四支柱未贯穿: self-improving+causal+memory+world-model 各点 1 次

## 3. Round-41 三候选
| # | 候选 | why now | 假设产出 |
|---|------|---------|----------|
| C1 | 因果+Pearl 落地 | Q1+Q11 双空;V1082 audit 缺因果图 | DoWhy 真读 → V1091 SCM+反事实 replan |
| C2 | RL+MuZero+world-model | 33轮 0 次;R40 已铺;V1083 缺理论 | MuZero+DreamerV3 → V1092 policy world-model |
| C3 | ShinkaEvolve/ASI-Arch 落地 | R39 GH 真读后无落地映射;V1090+ 急用 | 三连真读 → V1090 self-mod 借鉴表 |
