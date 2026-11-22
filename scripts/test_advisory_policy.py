"""Prevent active RustSec advisory ignore lists from drifting apart."""

from pathlib import Path
import re
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[1]
APPROVED_IGNORES = {"RUSTSEC-2024-0436", "RUSTSEC-2026-0543"}
RUSTSEC_ID = re.compile(r"RUSTSEC-\d{4}-\d{4}")


def toml_ignores(path: Path) -> set[str]:
    with path.open("rb") as source:
        return set(tomllib.load(source)["advisories"]["ignore"])


def command_ignores(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return set(re.findall(r"--ignore\s+(RUSTSEC-\d{4}-\d{4})", text))


class AdvisoryPolicyTests(unittest.TestCase):
    def test_all_active_ignore_lists_match_the_reviewed_set(self):
        sources = {
            "audit.toml": toml_ignores(ROOT / "audit.toml"),
            "deny.toml": toml_ignores(ROOT / "deny.toml"),
            "cargo-audit workflow": command_ignores(
                ROOT / ".github" / "workflows" / "cargo-audit.yml"
            ),
            "vet script": command_ignores(ROOT / "scripts" / "vet.sh"),
        }
        for name, found in sources.items():
            with self.subTest(source=name):
                self.assertEqual(found, APPROVED_IGNORES)

    def test_root_audit_config_has_no_unreviewed_advisory_id(self):
        text = (ROOT / "audit.toml").read_text(encoding="utf-8")
        self.assertEqual(set(RUSTSEC_ID.findall(text)), APPROVED_IGNORES)


if __name__ == "__main__":
    unittest.main()
