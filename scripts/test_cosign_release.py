"""Fail-closed contracts for the release signing shell scripts."""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SIGN = ROOT / "scripts" / "release" / "cosign-sign-all.sh"
VERIFY = ROOT / "scripts" / "release" / "cosign-verify.sh"
VERSION = "1.0.0"
EXPECTED = [
    f"apeireth_{VERSION}_amd64.deb",
    f"apeireth-{VERSION}-1.x86_64.rpm",
    "apeireth.rb",
    "apeireth.json",
    f"apeireth-{VERSION}-linux-amd64.tar.gz",
    f"apeireth-{VERSION}-darwin-universal.tar.gz",
    f"apeireth-{VERSION}-windows-amd64.zip",
    f"apeireth-{VERSION}.msi",
]


@unittest.skipUnless(shutil.which("bash"), "bash is required")
class CosignReleaseContracts(unittest.TestCase):
    def run_sign(self, dist: Path):
        env = {**os.environ, "COSIGN_BIN": "true", "DIST_DIR": str(dist)}
        return subprocess.run(
            ["bash", str(SIGN), "--dry-run"],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )

    def test_missing_expected_artifact_fails_even_in_dry_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.run_sign(Path(tmp))
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_complete_expected_manifest_passes_dry_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            dist = Path(tmp)
            for name in EXPECTED:
                (dist / name).write_bytes(b"synthetic release fixture\n")
            result = self.run_sign(dist)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_blob_verification_requires_bundle_and_certificate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            package = root / "package.zip"
            package.write_bytes(b"package")
            (root / "package.zip.sig").write_text("signature")
            public_key = root / "cosign.pub"
            public_key.write_text("synthetic public key")
            env = {
                **os.environ,
                "COSIGN_BIN": "true",
                "COSIGN_PUB": str(public_key),
            }
            result = subprocess.run(
                ["bash", str(VERIFY), str(package)],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=30,
            )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bundle", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
