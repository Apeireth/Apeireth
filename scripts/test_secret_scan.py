"""Behavior contracts for working, staged and historical credential scans."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess
import tempfile
import unittest

SCANNER = Path(__file__).resolve().with_name("secret-scan.ps1")


class SecretScanTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.scanner = SCANNER
        self.git("init", "-q")
        (self.repo / ".gitleaks.toml").write_text("[allowlist]\npaths = []\nregexes = []\n")

    def git(self, *args):
        return subprocess.run(
            ["git", "-c", "user.name=Codex", "-c", "user.email=codex@apeireth.local", *args],
            cwd=self.repo, check=True, capture_output=True, text=True,
        )

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "test: record scanner fixture")

    def scan(self, mode):
        return subprocess.run(
            ["pwsh", "-NoProfile", "-File", str(self.scanner), "-Mode", mode,
             "-RepoRoot", str(self.repo)],
            cwd=self.repo, capture_output=True, text=True, timeout=60,
        )

    def test_line_boundaries_do_not_create_a_credential(self):
        (self.repo / "fixture.rs").write_text(
            'let secret =\n    "a_long_symbol_name_without_credentials";\n'
        )
        self.commit()
        with (self.repo / "fixture.rs").open("a") as out:
            out.write("// staged clean fixture\n")
        self.git("add", ".")
        for mode in ["scan-all", "scan-staged", "scan-history"]:
            with self.subTest(mode=mode):
                result = self.scan(mode)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_each_mode_rejects_credentials_without_printing_them(self):
        credential = "sk" + "-" + "Q7Z9" * 8
        second = "gh" + "o_" + "Z9Q7" * 10
        (self.repo / "fixture.rs").write_text(
            '// header\n// second line\nlet value = "' + credential + ' ' + second + '";\n'
        )
        self.commit()
        # Stage another change so the staged mode also inspects this file.
        with (self.repo / "fixture.rs").open("a") as out:
            out.write("// staged\n")
        self.git("add", ".")
        for mode in ["scan-all", "scan-staged", "scan-history"]:
            with self.subTest(mode=mode):
                result = self.scan(mode)
                output = result.stdout + result.stderr
                self.assertEqual(result.returncode, 1)
                self.assertIn("fixture.rs:3:", output)
                self.assertIn("[REDACTED]", output)
                self.assertNotIn(credential, output)
                self.assertNotIn(second, output)

    def test_history_still_rejects_a_deleted_credential(self):
        credential = "sk" + "-" + "Z8Q6" * 8
        path = self.repo / "removed.rs"
        path.write_text('let value = "' + credential + '";\n')
        self.commit()
        path.unlink()
        self.commit()
        self.assertEqual(self.scan("scan-all").returncode, 0)
        result = self.scan("scan-history")
        self.assertEqual(result.returncode, 1)
        self.assertIn("[history] removed.rs:1:", result.stdout)
        self.assertNotIn(credential, result.stdout + result.stderr)

    def test_reviewed_history_exception_requires_every_immutable_identity_field(self):
        credential = "sk" + "-" + "Z8Q6" * 8
        path = self.repo / "fixture.rs"
        path.write_text('let value = "' + credential + '";\n')
        self.commit()
        blob = self.git("rev-parse", "HEAD:fixture.rs").stdout.strip()
        fixture = dict(Blob=blob, Path="fixture.rs", Line=1,
                       MatchSha256=hashlib.sha256(credential.encode()).hexdigest())
        with tempfile.TemporaryDirectory() as tools:
            self.scanner = Path(tools) / SCANNER.name
            shutil.copyfile(SCANNER, self.scanner)
            registry = Path(tools) / "secret-scan-history-fixtures.json"
            for field, wrong in [("Blob", "0" * 40), ("Path", "other.rs"),
                                 ("Line", 2), ("MatchSha256", "0" * 64)]:
                with self.subTest(field=field):
                    registry.write_text(json.dumps([{**fixture, field: wrong}]))
                    self.assertEqual(self.scan("scan-history").returncode, 1)
            registry.write_text(json.dumps([fixture]))
            result = self.scan("scan-history")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("immutable historical fixtures: 1", result.stdout)
            self.assertEqual(self.scan("scan-all").returncode, 1)
            with path.open("a") as out:
                out.write("// changed blob with the same credential\n")
            self.commit()
            self.assertEqual(self.scan("scan-history").returncode, 1)


if __name__ == "__main__":
    if not shutil.which("pwsh"):
        raise SystemExit("pwsh is required to verify the scanner")
    unittest.main()
