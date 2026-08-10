"""Show V1445 summary from the JSON report."""
import json
from pathlib import Path

d = json.load(open(Path(__file__).parent / ".v1445-asi-v2-position-closure-report.json", encoding="utf-8"))

print("schema:", d["schema"])
print("n_probes:", d["n_probes"])
print("closure_rate:", d["overall_closure_rate"])
print("per_kind:", d["per_kind_closure_rate"])
print()
print("per_position:")
for s in d["position_stats"]:
    pos = s["position"]
    cr = s["closure_rate"]
    bk = ",".join(s["broken_kinds"]) if s["broken_kinds"] else "-"
    print(f"  {pos:14s} {cr:.4f}  broken={bk}")
print()

# Cross-link matrix
matrix = {}
for cl in d["cross_links"]:
    matrix[(cl["source_position"], cl["target_position"])] = cl["linked"]

positions = ["scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"]
print("cross-link matrix (5x5):")
header = "              " + "  ".join(p[:8] for p in positions)
print(header)
for src in positions:
    row = []
    for tgt in positions:
        if src == tgt:
            row.append("  - ")
        else:
            row.append(f"  {matrix.get((src, tgt), 0)} ")
    print(f"  {src[:12]:12s} {''.join(row)}")