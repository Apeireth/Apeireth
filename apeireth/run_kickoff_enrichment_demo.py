"""Demo: Kickoff v0.4 enrichment — 跑 enrichment + 写回 master + 报告完整度

PoC 流程:
  1) 加载 identity_card.master.json (raw kickoff 输出, recall_anchor/evidence_refs 都空)
  2) 跑 enrich() — 派生 3 项 + 注入回 card
  3) 存盘到 identity_card.master.json (覆写, 带时间戳)
  4) reload + round-trip verify (integrity hash 三方一致: original → enriched → reloaded)
  5) 打印 EnrichmentReport

依据: TOP-DESIGN-V1 §3.4 + DEV-LOG 21:09 "Phase 1 PoC enrichment 完成度 = v0.4"
"""

from __future__ import annotations
import sys
from pathlib import Path

if sys.platform == "win32":
    for s in (sys.stdout, sys.stderr):
        try:
            s.reconfigure(encoding="utf-8")
        except Exception:
            pass

from apeireth import IdentityCard, load_card, save_card
from apeireth.kickoff_enrichment import (
    enrich,
    derive_recall_anchor,
    suggest_evidence_refs,
    compute_completeness,
    check_version,
    EnrichmentReport,
)


def main() -> None:
    here = Path(__file__).parent
    master = here / "identity_card.master.json"

    print("=" * 64)
    print("🜂 Apeireth — Kickoff Enrichment v0.4 PoC")
    print("=" * 64)

    # 1) 加载 raw master
    raw_card = load_card(master)
    raw_hash = raw_card.integrity_hash()
    print(f"\n[1] Loaded raw master:")
    print(f"    name            = {raw_card.name}")
    print(f"    recall_anchor   = {raw_card.recall_anchor!r}  (空 = 未 enrichment)")
    print(f"    evidence_refs   = {raw_card.evidence_refs}  (空)")
    print(f"    completeness    = {compute_completeness(raw_card)}")
    print(f"    integrity_hash  = {raw_hash}")

    # 2) 跑 enrichment
    report = enrich(raw_card, write_back=True)

    print(f"\n[2] Enrichment results:")
    print(f"    ⚓ recall_anchor  = {report.recall_anchor}")
    print(f"    🔗 evidence_refs  = {len(report.evidence_refs)} refs")
    for r in report.evidence_refs:
        print(f"        - {r}")
    print(f"    📊 completeness   = {report.completeness_score}")
    print(f"    📦 version_status = "
          f"valid={report.version_status['schema_valid']}, "
          f"needs_migration={report.version_status['needs_migration']}")

    # 3) 存盘 (覆写 master)
    save_card(raw_card, master)
    enriched_hash = raw_card.integrity_hash()
    print(f"\n[3] Saved enriched master → {master}")
    print(f"    integrity_hash (after enrich) = {enriched_hash}")

    # 4) reload verify
    reloaded = load_card(master)
    reloaded_hash = reloaded.integrity_hash()
    print(f"\n[4] Round-trip verify:")
    print(f"    raw_hash     = {raw_hash}")
    print(f"    enriched_hash= {enriched_hash}  (changed because enriched_at + fields)")
    print(f"    reloaded_hash= {reloaded_hash}")
    print(f"    ✅ enriched == reloaded: {enriched_hash == reloaded_hash}")

    # 5) 验收
    print("\n[5] Acceptance:")
    print(f"    ✅ recall_anchor populated:  {bool(reloaded.recall_anchor)}")
    print(f"    ✅ evidence_refs populated:   {bool(reloaded.evidence_refs)}")
    print(f"    ✅ completeness ≥ 0.5:        {reloaded and compute_completeness(reloaded) >= 0.5}")
    print(f"    ✅ schema valid:              {check_version(reloaded)['schema_valid']}")
    print(f"    ✅ round-trip integrity:      {enriched_hash == reloaded_hash}")

    print("\n" + "=" * 64)
    print("Phase 1 v0.4 enrichment PoC ✅ — 富化产物已写入 master.")
    print("=" * 64)


if __name__ == "__main__":
    main()
