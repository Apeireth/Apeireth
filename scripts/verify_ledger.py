#!/usr/bin/env python3
"""四层验收台账（verification-ledger.yaml）只读校验器。

载体来源：ADR-0011 D4（`docs/status/verification-ledger.yaml` 的结构化真源）。
本脚本只读、不改文件、不接触效率线资产；用于把「台账漂移」变成可执行检查：

  1. schema：条目必备字段、layer/level 取值合法、id 唯一、同 claim 同层唯一；
  2. 零装下限：非 signoff 层 `signoff` 必须为 null；无 signoff 的 claim 不得标
     HARDWARE VALIDATED（ADR-0011 D4：无签核最高 PRODUCTION WIRED）；
  3. 证据落地：evidence 里的相对路径必须真实存在于仓库（目录也算存在）；
  4. summary 一致性：entries / claims_with_entries / four_layer_complete /
     signoff_entries 必须与条目实况一致；
  5. signoff 层必须有 owner/date/note 非空；
  6. claim 绑定：claim 若能在 docs/status/claims.yaml 找到则绑定通过，找不到只报
     info（schema 允许「纯验证条目」）。

用法：
  python scripts/verify_ledger.py            # 人类可读输出
  python scripts/verify_ledger.py --quiet    # 仅错误/汇总

退出码：0 = 通过（可能有 info）；1 = 有 error；2 = 台账/依赖不可读。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - 环境缺 PyYAML 时明确失败
    print("[verify_ledger] ERROR: PyYAML 不可用（pip install pyyaml）", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "status" / "verification-ledger.yaml"
CLAIMS = ROOT / "docs" / "status" / "claims.yaml"

LAYERS = ("formal", "bench", "scenario", "signoff")
LEVELS = (
    "IMPLEMENTED",
    "PRODUCTION WIRED",
    "DEFAULT ENABLED",
    "HARDWARE VALIDATED",
    "NOT IMPLEMENTED",
)
ENTRY_REQUIRED = ("id", "claim", "layer", "level", "evidence", "verify_cmd", "bounded", "verified_at")
SUMMARY_KEYS = ("entries", "claims_with_entries", "four_layer_complete", "signoff_entries")
HASH_RE = re.compile(r"^[0-9a-f]{7,40}$")


class Report:
    def __init__(self) -> None:
        self.errors: list[str] = []
        self.warnings: list[str] = []
        self.infos: list[str] = []

    def error(self, msg: str) -> None:
        self.errors.append(msg)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)

    def info(self, msg: str) -> None:
        self.infos.append(msg)


def load_claims_ids() -> set[str]:
    if not CLAIMS.exists():
        return set()
    try:
        data = yaml.safe_load(CLAIMS.read_text(encoding="utf-8")) or {}
    except Exception:
        return set()
    return {c.get("id") for c in data.get("claims", []) if isinstance(c, dict) and c.get("id")}


def check_evidence(entry_id: str, rel: str, rep: Report) -> None:
    if not isinstance(rel, str) or not rel.strip():
        rep.error(f"{entry_id}: evidence 内含空路径")
        return
    if rel.startswith(("http://", "https://")):
        return
    path = ROOT / rel
    if not path.exists():
        rep.error(f"{entry_id}: evidence 路径不存在: {rel}")


def main() -> int:
    ap = argparse.ArgumentParser(description="只读校验四层验收台账")
    ap.add_argument("--quiet", action="store_true", help="只输出错误与汇总")
    args = ap.parse_args()

    if not LEDGER.exists():
        print(f"[verify_ledger] ERROR: 台账不存在: {LEDGER}", file=sys.stderr)
        return 2
    try:
        data = yaml.safe_load(LEDGER.read_text(encoding="utf-8")) or {}
    except Exception as exc:  # noqa: BLE001 - 明确报错即可
        print(f"[verify_ledger] ERROR: YAML 解析失败: {exc}", file=sys.stderr)
        return 2

    rep = Report()
    claims_ids = load_claims_ids()

    if not isinstance(data, dict):
        print("[verify_ledger] ERROR: 台账顶层必须是 mapping", file=sys.stderr)
        return 2

    for key in ("version", "updated_at", "summary", "entries"):
        if key not in data:
            rep.error(f"顶层缺字段: {key}")
    if rep.errors:
        return finish(rep, args.quiet)

    entries = data.get("entries")
    if not isinstance(entries, list) or not entries:
        rep.error("entries 必须是非空列表")
        return finish(rep, args.quiet)

    seen_ids: set[str] = set()
    claim_layers: dict[str, set[str]] = {}
    signoff_entries = 0
    level_by_claim: dict[str, str] = {}

    for idx, entry in enumerate(entries):
        if not isinstance(entry, dict):
            rep.error(f"entries[{idx}] 不是 mapping")
            continue
        entry_id = entry.get("id")
        if not isinstance(entry_id, str) or not entry_id:
            rep.error(f"entries[{idx}] 缺 id")
            continue
        if entry_id in seen_ids:
            rep.error(f"{entry_id}: id 重复")
        seen_ids.add(entry_id)

        for field in ENTRY_REQUIRED:
            if field not in entry:
                rep.error(f"{entry_id}: 缺字段 {field}")

        layer = entry.get("layer")
        if layer not in LAYERS:
            rep.error(f"{entry_id}: layer 非法 {layer!r}（允许 {LAYERS}）")
            continue
        level = entry.get("level")
        if level not in LEVELS:
            rep.error(f"{entry_id}: level 非法 {level!r}")

        claim = entry.get("claim")
        if claim is not None and not isinstance(claim, str):
            rep.error(f"{entry_id}: claim 必须是字符串或 null")

        evidence = entry.get("evidence")
        if isinstance(evidence, list) and evidence:
            for rel in evidence:
                check_evidence(entry_id, rel, rep)
        else:
            rep.error(f"{entry_id}: evidence 必须是非空列表")

        if not isinstance(entry.get("verify_cmd"), str) or not entry.get("verify_cmd", "").strip():
            rep.error(f"{entry_id}: verify_cmd 必须是非空字符串")
        if "bounded" not in entry:
            rep.error(f"{entry_id}: 缺 bounded（非 formal 层写 '->（n/a）'）")
        verified_at = entry.get("verified_at")
        if not isinstance(verified_at, str) or not HASH_RE.match(verified_at):
            rep.error(f"{entry_id}: verified_at 必须是 commit hash")

        signoff = entry.get("signoff")
        if layer == "signoff":
            signoff_entries += 1
            if not isinstance(signoff, dict):
                rep.error(f"{entry_id}: signoff 层必须填 signoff{{owner,date,note}}")
            else:
                for k in ("owner", "date", "note"):
                    v = signoff.get(k)
                    if not isinstance(v, str) or not v.strip():
                        rep.error(f"{entry_id}: signoff.{k} 必须非空")
        elif signoff is not None:
            rep.error(f"{entry_id}: 非 signoff 层的 signoff 必须为 null（零装）")

        if claim:
            claim_layers.setdefault(claim, set()).add(layer)
            if level:
                level_by_claim[claim] = level

    # 同 claim 同层唯一
    for claim, layers in claim_layers.items():
        counted = [
            e.get("layer")
            for e in entries
            if isinstance(e, dict) and e.get("claim") == claim
        ]
        for layer in layers:
            if counted.count(layer) > 1:
                rep.error(f"claim={claim}: 同层重复登记 {layer}")

    # 无 signoff 的 claim 不得 HARDWARE VALIDATED
    for claim, level in level_by_claim.items():
        if level == "HARDWARE VALIDATED" and "signoff" not in claim_layers.get(claim, set()):
            rep.error(f"claim={claim}: 无 signoff 层却标 HARDWARE VALIDATED（违反 D4）")

    # claims.yaml 绑定（info，不阻断）
    for claim in claim_layers:
        if claim not in claims_ids:
            rep.info(f"claim={claim}: 不在 claims.yaml（纯验证条目，schema 允许）")

    # summary 一致性
    summary = data.get("summary")
    if not isinstance(summary, dict):
        rep.error("summary 必须是 mapping")
    else:
        expected_counts = {
            "entries": len(entries),
            "claims_with_entries": len(claim_layers),
            "four_layer_complete": sum(1 for s in claim_layers.values() if s == set(LAYERS)),
            "signoff_entries": signoff_entries,
        }
        for key in SUMMARY_KEYS:
            if key not in summary:
                rep.error(f"summary 缺字段 {key}")
                continue
            if summary[key] != expected_counts[key]:
                rep.error(
                    f"summary.{key}={summary[key]!r} 与实况 {expected_counts[key]!r} 不一致"
                )
        stage6 = summary.get("stage6_drill")
        if stage6 not in ("NOT RUN", "DONE"):
            rep.warn(f"summary.stage6_drill={stage6!r} 建议明确写 NOT RUN / DONE")

    return finish(rep, args.quiet)


def finish(rep: Report, quiet: bool) -> int:
    for msg in rep.infos:
        if not quiet:
            print(f"[verify_ledger] INFO  {msg}")
    for msg in rep.warnings:
        print(f"[verify_ledger] WARN  {msg}")
    for msg in rep.errors:
        print(f"[verify_ledger] ERROR {msg}")
    status = "FAILED" if rep.errors else "OK"
    print(
        f"[verify_ledger] {status}: errors={len(rep.errors)} "
        f"warnings={len(rep.warnings)} infos={len(rep.infos)}"
    )
    return 1 if rep.errors else 0


if __name__ == "__main__":
    sys.exit(main())
