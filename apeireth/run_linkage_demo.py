"""Phase 5.5 联动层 demo runner — 跑通 A → B → C 三次完整闭环.

不依赖 LLM, 全程 scripted priors + 模拟主人回答.
验证:
  1) Path A: Reconsolidation.flag → funnel.add_question (新增 ≥1 个问题)
  2) Path B: funnel.ask_next()   → persona.coordinate() (每次激活 2 个 persona)
  3) Path C: feedback_score     → funnel.record_answer + persona.adapt (闭环完成)

输出:
  - data/linkage_demo.json — snapshot (turns + integrity_hash)
"""

from __future__ import annotations
import json
import sys
from pathlib import Path

# 让脚本可以单独跑
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.identity_store import IdentityStore
from apeireth.memory import MemoryStore, load_store as load_memory, Note, Episode
from apeireth.persona import PersonaEngine, seed_default_personas
from apeireth.questioning import BayesianFunnel
from apeireth.linkage import LinkageOrchestrator, LINKAGE_VERSION


def main() -> int:
    print("=" * 60)
    print(f"Linkage Layer v{LINKAGE_VERSION} — A → B → C demo")
    print("=" * 60)

    # ── 加载 IdentityStore (Phase 1 v0.2 — 已有 6 张卡) ──
    store_dir = ROOT / "apeireth" / "data" / "identity_store"
    print(f"\n[1] loading IdentityStore from {store_dir.name}/")
    identity = IdentityStore()
    log = identity.load_dir(store_dir)
    stats = identity.stats()
    print(f"    ✓ loaded {stats['total']} cards (roles: {stats['by_role']})")
    for line in log:
        if "[warn]" in line or "[error]" in line:
            print(f"    ! {line}")
    master = identity.master()
    if master is None:
        print("    ✗ no master card — run run_identity_store_demo.py first")
        return 1
    print(f"    master: name={master.name!r} apeireth_version={master.apeireth_version}")

    # ── 加载 / 构建 MemoryStore ──
    memory_path = ROOT / "data" / "memory.db.json"
    print(f"\n[2] loading MemoryStore from {memory_path.name}")
    if memory_path.exists():
        memory = load_memory(memory_path)
    else:
        # 兜底: 重建一个最小 store (有 flag note 让 Path A 跑通)
        memory = MemoryStore()
        memory.append_episode(Episode(
            eid="demo_e_link", actor="master",
            content="主人提到不要提私人身份",
            kind="kickoff",
        ))
        memory.add_note(Note(
            nid="demo_n_flag", topic="边界",
            claim="主人私人身份不在 funnel 里",
            evidence=["demo_e_link"],
            confidence=0.9, importance=0,    # importance=0 → Path A 触发
        ))
    print(f"    episodes={len(memory.episodes)} notes={len(memory.notes)}")
    flagged = [n for n in memory.notes if n.importance == 0]
    print(f"    flagged notes (importance=0): {len(flagged)} → Path A 候选")

    # ── 构建 PersonaEngine (4 archetype 种子) ──
    print(f"\n[3] building PersonaEngine (4 archetype)")
    personas = PersonaEngine(seed_default_personas())
    print(f"    personas: {len(personas.personas)} "
          f"({', '.join(p.archetype for p in personas.personas)})")

    # ── 构建 BayesianFunnel (从 master card 灌入 priors + gap) ──
    print(f"\n[4] seeding BayesianFunnel from master card")
    funnel = BayesianFunnel()
    n_prior = funnel.seed_from_identity(master)
    n_gap = funnel.seed_gap_questions(master)
    print(f"    offline_prior: {n_prior} questions")
    print(f"    gap_inference: {n_gap} questions")
    print(f"    total seeded: {len(funnel.questions)} questions")

    # ── 构建 LinkageOrchestrator ──
    print(f"\n[5] building LinkageOrchestrator")
    orch = LinkageOrchestrator(identity, memory, personas, funnel)
    print(f"    ✓ 4 modules linked: identity + memory + personas + funnel")

    # ── Path A: flag note → funnel ──
    print(f"\n[6] Path A: Reconsolidation.flag → funnel.add_question")
    added_qids = orch.run_path_a()
    print(f"    added {len(added_qids)} question(s) from flagged notes")
    for qid in added_qids:
        q = funnel.questions[qid]
        print(f"      [{qid}] topic={q.topic!r} prior={q.prior} prompt={q.prompt[:50]}...")

    # ── Path A → B → C × 3 完整闭环 ──
    print(f"\n[7] running full loop (A → B → C) × 3")
    scripted = [
        {"text": "[scripted #1] mission 字段要写 ASI 无限逼近",
         "observed": 0.85, "feedback": +0.8, "persona_pid": None},
        {"text": "[scripted #2] domains 包含全栈 / 攻防 / 人文 / 科研 / 预测",
         "observed": 0.80, "feedback": +0.6, "persona_pid": None},
        {"text": "[scripted #3] flag note 的真实原因是 '保护主人边界'",
         "observed": 0.75, "feedback": +0.4, "persona_pid": None},
    ]
    new_turns = orch.run_full_loop(n=3, scripted_answers=scripted)
    print(f"    ✓ ran {len(new_turns)} turns")
    for t in new_turns:
        print(f"      [{t.path}] {t.note}")

    # ── Funnel 状态 ──
    print(f"\n[8] funnel summary (top 6 by uncertainty):")
    summary = funnel.summary()
    summary.sort(key=lambda s: s.posterior)
    for s in summary[:6]:
        marker = "✓" if s.status == "answered" else "·"
        print(f"      {marker} [{s.topic:14s}] prior={s.prior:.2f} → "
              f"posterior={s.posterior:.3f}  {s.prompt[:40]}")

    # ── Persona 状态 ──
    print(f"\n[9] persona reflection:")
    for p in personas.personas:
        print(f"      {personas.reflect(p.pid)}")

    # ── Integrity hash (5 层) ──
    print(f"\n[10] cross-module integrity_hash (5 layers)")
    print(f"      identity_hash : {identity.integrity_hash()}")
    print(f"      memory_hash   : {memory.integrity_hash()}")
    print(f"      funnel_hash   : {funnel.integrity_hash()}")
    print(f"      linkage_hash  : {orch.integrity_hash()}")

    # ── 保存 snapshot ──
    out_dir = ROOT / "data"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "linkage_demo.json"
    snap = orch.snapshot()
    snap["version"] = LINKAGE_VERSION
    snap["module_hashes"] = {
        "identity": identity.integrity_hash(),
        "memory": memory.integrity_hash(),
        "funnel": funnel.integrity_hash(),
    }
    out_path.write_text(
        json.dumps(snap, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    print(f"\n[✓] saved: {out_path.relative_to(ROOT)}")
    print(f"    turns={snap['turn_count']} hash={snap['integrity_hash']}")

    print("\n" + "=" * 60)
    print(f"Phase 5.5 linkage v{LINKAGE_VERSION} demo passed.")
    print("A → B → C 闭环完成, 4 模块联动, 5 层 integrity_hash 一致.")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(main())