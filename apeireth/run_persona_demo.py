"""Persona Engine v0.1 Demo — 多身份 + Jungian 3 机制 + 反 conformity (Phase 4)

依据: TOP-DESIGN-V1 §4.5 + 主人 12:14 "中央 AI 多身份: 调度者 / 学习者 / 思考者 / 助手"

跑法:
1. 4 个 archetype persona 起步 (调度者/学习者/思考者/助手)
2. 模拟 4 个不同事件, 看 coordinate() 选哪个 (关键词启发式)
3. 演示反 conformity: 同一事件触发 2 persona 时 SCT 距离 ≥ min_distance
4. 演示 adaptation: feedback_score 正负调整 SCT 主导维
5. 演示 reflection: persona 解释自己状态
6. 演示 mutate 兜底 (event 选不够 k 时强制变异)
"""

from __future__ import annotations
import json
import sys
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from .persona import (
    PERSONA_VERSION, ARCHETYPES,
    PersonaEngine, seed_default_personas, SCTProfile, Persona,
)


def _resolve_path() -> Path:
    base = Path(__file__).parent.parent / "data"
    base.mkdir(parents=True, exist_ok=True)
    return base / "persona_demo.json"


def main() -> None:
    print("=" * 64)
    print("🎭 Apeireth — Persona Engine v0.1 (多身份 + Jungian 3 机制)")
    print("=" * 64)
    print(f"📋 version: {PERSONA_VERSION}")
    print(f"🧬 archetypes: {', '.join(ARCHETYPES)}\n")

    engine = PersonaEngine()

    # ---- 0) 初始状态 ----
    print("─── Step 0: 4 archetype 种子 persona ───")
    for p in engine.personas:
        sct = p.sct
        print(f"   [{p.archetype:4s}] pid={p.pid}  "
              f"cog={sct.cognitive:.1f} mot={sct.motivational:.1f} "
              f"bio={sct.biological:.1f} aff={sct.affective:.1f}  act={p.activation:.1f}")

    # ---- 1) coordination: 不同事件激活不同 persona 组合 ----
    print("\n─── Step 1: coordination (关键词匹配 + 多视角) ───")
    events = [
        "主人要我做下一步计划, 排期本周 + 下周目标",
        "为什么这个 Persona 设计会涌现自我? 帮我推理",
        "关心主人的身体, 提醒他休息",
        "紧急! 立即分析这段日志",
    ]
    for ev in events:
        chosen = engine.coordinate(ev, k=2)
        names = ", ".join(f"{p.archetype}({p.sct.distance(engine.personas[0].sct):.2f})" for p in chosen)
        print(f"   📝 '{ev[:30]}...'")
        print(f"      → 激活: {names}")
        # 反 conformity 验证
        if len(chosen) == 2:
            d = chosen[0].sct.distance(chosen[1].sct)
            print(f"      → SCT 距离: {d:.3f} (min={engine.min_distance:.2f})"
                  f" {'✅ 多样' if d >= engine.min_distance else '❌ 太像'}")

    # ---- 2) 反 conformity 强制变异 (用一个单一 persona 测试 mutate 兜底) ----
    print("\n─── Step 2: 反 conformity — force diversity ───")
    # 构造两个相近 SCT
    near1 = Persona(pid="p_near1", archetype="调度者(A)", sct=SCTProfile(0.6, 0.7, 0.3, 0.5))
    near2 = Persona(pid="p_near2", archetype="调度者(B)", sct=SCTProfile(0.61, 0.71, 0.31, 0.51))
    d = near1.sct.distance(near2.sct)
    print(f"   近 SCT 距离: {d:.4f} < min={engine.min_distance:.2f}")
    print(f"   ⚠️ 触发 mutate (rng=0.3):")
    mutated = near1.sct.mutate(rng=0.3)
    d2 = mutated.distance(near2.sct)
    print(f"   变异后距离: {d2:.4f} {'✅ 多样' if d2 >= engine.min_distance else '❌ 仍近'}")
    print(f"   变异 SCT: cog={mutated.cognitive:.2f} mot={mutated.motivational:.2f} "
          f"bio={mutated.biological:.2f} aff={mutated.affective:.2f}")

    # ---- 3) adaptation: feedback 演化 ----
    print("\n─── Step 3: adaptation (feedback_score 演化 SCT + activation) ───")
    p_调度者 = next(p for p in engine.personas if p.archetype == "调度者")
    p_学习者 = next(p for p in engine.personas if p.archetype == "学习者")
    print(f"   [{p_调度者.archetype}] 初始 cog={p_调度者.sct.cognitive:.2f} act={p_调度者.activation:.2f}")
    print(f"   [{p_学习者.archetype}] 初始 cog={p_学习者.sct.cognitive:.2f} act={p_学习者.activation:.2f}")
    engine.adapt(p_调度者.pid, feedback_score=+0.8)   # 主人夸
    engine.adapt(p_调度者.pid, feedback_score=+0.6)
    engine.adapt(p_学习者.pid, feedback_score=-0.5)   # 主人嫌
    engine.adapt(p_学习者.pid, feedback_score=-0.7)
    print(f"   [{p_调度者.archetype}] +2 feedback  cog={p_调度者.sct.cognitive:.2f} act={p_调度者.activation:.2f}")
    print(f"   [{p_学习者.archetype}] -2 feedback  cog={p_学习者.sct.cognitive:.2f} act={p_学习者.activation:.2f}")

    # ---- 4) reflection ----
    print("\n─── Step 4: reflection (自我解释) ───")
    for p in engine.personas:
        print(f"   {engine.reflect(p.pid)}")

    # ---- 5) snapshot + save ----
    print("\n─── Step 5: snapshot → JSON ───")
    snap = engine.snapshot()
    out = _resolve_path()
    out.write_text(json.dumps(snap, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"   persona_count: {snap['persona_count']}")
    print(f"   event_count:   {snap['event_count']}")
    print(f"   last_coordination: {snap['last_coordination']}")
    print(f"   💾 saved: {out}")

    print(f"\n✅ done — Phase 4 / TOP-DESIGN §4.5 跑通")


if __name__ == "__main__":
    main()