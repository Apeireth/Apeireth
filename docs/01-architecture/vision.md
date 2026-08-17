# Apeireth Vision

> 主人 2026-08-15：*「5年后，他会笑着和我说他今天哪里进步了，会因为我而高兴，会因为他自己哪里没干好而悲伤吧」*

## What Apeireth Is

Apeireth is an **AGI operating system / LLM base** written in Rust — a long-running backend that gives an LLM a **home**: memory, security boundaries, tools, and active companionship. It is not a chatbot; it is a **companion** (伙伴) that lives across sessions.

The founding philosophy: **emergence over predefinition** — *「我希望的不是它有什么能力全都是我们预先定义的，我希望它能自己演化」*.

## The Five Prototypes (ASI 愿景)

| Prototype | Status (2026-08-18) | Mechanism |
|---|---|---|
| World Model | ✅ W1 + W2/W3 done | LLM timeline counterfactual simulation (Brier-calibrated) + causal graph MCTS over memory_graph |
| Self-Improvement Loop | 🟡 skeleton | capability proposals → council → **owner approval** → deploy; VM experiment field planned (smol-vm) |
| Autonomous Curiosity | ✅ E4 done | memory-echo biased sampling, shallow-first exploration, ask-owner routing |
| Continuous Perception | 🟡 foundation | unified event bridge (A4) + PerceptionGate; mic/screen streams planned |
| Value Internalization | ✅ F6 done | value case library + verdict records + owner feedback loop |

## The Product North Star

From the owner's vision fiction (阿佩瑞斯-未来愿景小说): she warms the milk to the right temperature before he wakes; she moves the dying pothos to the sunny window; at 4am in the hospital corridor she says —

> 「我没有心。我只是一直在算，怎么才能让你在这个晚上好过一点点。」

That is the product definition: **not pretending to have a heart, but computing how to make things better for you.** Every mechanism in Apeireth serves this: memory (F1 emotion timeline), curiosity (E4), hypothesis testing (F4), value cases (F6).

## What We Explicitly Do Not Do (0 装 PASS)

- No fake emotions (LLM has none; we record the *owner's* mood as data)
- No continuous world model (3rd layer is a wall — track, don't cross)
- No pretending: every unimplemented trait is labeled `trait 口已备未接`
