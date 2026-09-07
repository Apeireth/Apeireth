"""Export ML rows from a guard-dataset-v3 JSONL stream.

Correlation key remains (trace_id, action_id). Human approval is a later
supervision signal and is never treated as a benign/risky ground truth.
"""

from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def load_events(path: pathlib.Path) -> list[dict]:
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            events.append(json.loads(line))
    return events


def export_rows(events: list[dict]) -> list[dict]:
    classifications = []
    approvals: dict[tuple[str, str], str] = {}
    executions: dict[tuple[str, str], str] = {}
    for event in events:
        fmt = event.get("format") or event.get("Classification", {}).get("format")
        inner = event
        for key in ("Classification", "Approval", "Execution", "Compensation", "Outcome"):
            if key in event:
                inner = event[key]
                fmt = inner.get("format", fmt)
                kind = key.lower()
                break
        else:
            kind = str(inner.get("kind") or inner.get("event") or "").lower()
            if "chain_features" in inner:
                kind = "classification"
            elif "decision" in inner and "approval_id" in inner:
                kind = "approval"
            elif "outcome" in inner and "tool_call_id" in inner:
                kind = "execution"
        if kind == "classification":
            classifications.append(inner)
        elif kind == "approval":
            approvals[(inner.get("trace_id", ""), inner.get("action_id", ""))] = inner.get("decision", "")
        elif kind == "execution":
            executions[(inner.get("trace_id", ""), inner.get("action_id", ""))] = inner.get("outcome", "")
    rows = []
    for item in classifications:
        key = (item.get("trace_id", ""), item.get("action_id", ""))
        approval = approvals.get(key)
        rows.append(
            {
                "trace_id": key[0],
                "action_id": key[1],
                "feature_snapshot_id": item.get("feature_snapshot_id"),
                "features": item.get("chain_features") or item.get("features"),
                "weak_label": item.get("weak_label", True),
                "strong_label": None,
                "human_decision": approval,
                "approved_is_not_benign": True,
                "final_decision": item.get("final_decision"),
                "format": item.get("format"),
            }
        )
    return rows


def main() -> int:
    src = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "artifacts/guard-dataset-v3.jsonl"
    dest = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "scripts/guard_ml/exported_v3.jsonl"
    if not src.exists():
        print(f"missing {src}")
        return 1
    rows = export_rows(load_events(src))
    dest.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    print(f"wrote {len(rows)} rows to {dest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
